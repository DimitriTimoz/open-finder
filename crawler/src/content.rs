use std::{collections::HashSet, fs, path};

use futures::executor::block_on;
use meilisearch_sdk::client::*;
use serde::{Deserialize, Serialize};

use crate::link::{get_links, Url};

#[derive(Clone, Serialize, Deserialize)]
pub enum ContentType {
    Html,
    Css,
    Js,
    Pdf,
    Image,
    Json,
    Xml,
    Other,
}

impl ContentType {
    fn from(file_name: String, content: &Vec<u8>) -> Self {
        let mut file_name = file_name.to_lowercase();
        if let Some(filen_name) = file_name.split('.').last() {
            file_name = filen_name.split('?').next().unwrap_or("").to_string();
            match file_name.as_str() {
                "html" | "htm" => ContentType::Html,
                "pdf" => ContentType::Pdf,
                "png" => ContentType::Image,
                "jpg" => ContentType::Image,
                "jpeg" => ContentType::Image,
                "gif" => ContentType::Image,
                "svg" => ContentType::Image,
                "ico" => ContentType::Image,
                "webp" => ContentType::Image,
                "bmp" => ContentType::Image,
                "tiff" => ContentType::Image,
                "tif" => ContentType::Image,
                "psd" => ContentType::Image,
                "raw" => ContentType::Image,
                "css" => ContentType::Css,
                "js" => ContentType::Js,
                "json" => ContentType::Json,
                "xml" => ContentType::Xml,
                _ => {
                    if content.starts_with(b"<!DOCTYPE html>") {
                        ContentType::Html
                    } else {
                        ContentType::Other
                    }        
                }
            }
    
        } else if content.starts_with(b"<!DOCTYPE html>") {
            ContentType::Html
        } else {
            ContentType::Other
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Document {
    url: Url,
    content: String,
    kind: ContentType,
    hash: String,
}

pub struct Content {
    bytes: Vec<u8>,
    kind: ContentType,
}

impl Content {
    pub fn new(bytes: Vec<u8>, name: String) -> Self {
        Content {
            kind: ContentType::from(name.clone(), &bytes),
            bytes,
        }
    }

    async fn to_document(&self, url: Url) -> Document {
        Document {
            url: url.clone(),
            content: self.to_text().await.unwrap_or_default(),
            kind: self.kind.clone(),
            hash: format!("{:x}", md5::compute(url.to_string().as_bytes())),
        }
    }

    pub fn publish(&self, urls: &[Url]) {
        block_on(async move {
            let documents = urls
                .iter()
                .map(|url| self.to_document(url.clone()));
            let documents = futures::future::join_all(documents).await;

            let client = Client::new("http://localhost:7700", Some("key")).unwrap();
            // adding documents
            let res = client
                .index("docs")
                .add_documents(documents.as_slice(), Some("hash"))
                .await;
            if res.is_err() {
                println!("{:?}", res);
            }
        });
    }

    pub fn get_links(&self, url: Url) -> HashSet<Url> {
        match self.kind {
            ContentType::Pdf => HashSet::new(),
            _ => get_links(&String::from_utf8(self.bytes.clone()).unwrap_or_default(), url),
        }
    }

    pub fn get_bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub async fn to_text(&self) -> Option<String> {
        match self.kind {
            ContentType::Html => {
                let mut text = String::new();
                txt_extractor::extract_text(&String::from_utf8(self.bytes.clone()).unwrap_or_default(), &mut text).await;
                Some(text)
            },
            ContentType::Pdf => {
                let mut text = String::new();
                txt_extractor::extract_text_from_pdf(self.bytes.as_slice(), &mut text).await;
                Some(text)
            }
            _ => None,
        }
    }

    pub async fn save(&self, url: Url) {
        use std::fs::File;
        use std::io::Write;
        // Mkdir
        let path = path::Path::new("data");
        if !path.exists() {
            std::fs::create_dir(path).unwrap();
        }

        if let Some(bytes) = self.to_text().await.into_iter().next() {
            if bytes.is_empty() {
                return;
            }
            // Find the unique folder
            let path = "data";  // Specify the directory path
            let mut unique_dirs = HashSet::new();
        
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            if let Ok(file_name) = entry.file_name().into_string() {
                                unique_dirs.insert(file_name);
                            }
                        }
                    }
                }
            }

            let folder = unique_dirs.iter().next().unwrap();

            let mut path = format!("data/{folder}/{}.txt", url.to_string().replace('/', "_"));
            path.truncate(255);
            let file = File::create(path);
            if let Ok(mut file) = file  {
                file.write_all(bytes.as_bytes())
                    .unwrap(); 
            }
    
        }
    }
}
