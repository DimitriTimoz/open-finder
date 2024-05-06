use std::{collections::HashSet, path};

use futures::{executor::block_on, future::select_all, task::UnsafeFutureObj};
use meilisearch_sdk::Client;
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
    fn from(file_name: String, content: &str) -> Self {
        let file_name = file_name.to_lowercase();
        match file_name.split('.').last() {
            Some("html") | Some("htm") => ContentType::Html,
            Some("pdf") => ContentType::Pdf,
            Some("png") => ContentType::Image,
            Some("jpg") => ContentType::Image,
            Some("jpeg") => ContentType::Image,
            Some("gif") => ContentType::Image,
            Some("svg") => ContentType::Image,
            Some("ico") => ContentType::Image,
            Some("webp") => ContentType::Image,
            Some("bmp") => ContentType::Image,
            Some("tiff") => ContentType::Image,
            Some("tif") => ContentType::Image,
            Some("psd") => ContentType::Image,
            Some("raw") => ContentType::Image,
            Some("css") => ContentType::Css,
            Some("js") => ContentType::Js,
            Some("json") => ContentType::Json,
            Some("xml") => ContentType::Xml,
            _ => {
                if content.trim_start().starts_with("<!DOCTYPE html>") {
                    ContentType::Html
                } else {
                    ContentType::Other
                }
            }
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
    bytes: String,
    kind: ContentType,
}

impl Content {
    pub fn new(bytes: String, name: String) -> Self {
        Content {
            kind: ContentType::from(name.clone(), &bytes),
            bytes,
        }
    }

    async fn to_document(&self, url: Url) -> Document {
        Document {
            url,
            content: self.to_text().await.unwrap_or_default(),
            kind: self.kind.clone(),
            hash: format!("{:x}", md5::compute(&self.bytes)),
        }
    }

    pub fn publish(&self, urls: &[Url]) {
        block_on(async move {
            let documents = urls
                .iter()
                .map(|url| self.to_document(url.clone()));
            let documents = futures::future::join_all(documents).await;

            let client = Client::new("http://localhost:7700", Some("key"));
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
            _ => get_links(&self.bytes, url),
        }
    }

    pub fn get_bytes(&self) -> &str {
        &self.bytes
    }

    pub async fn to_text(&self) -> Option<String> {
        match self.kind {
            ContentType::Html => {
                let mut text = String::with_capacity(100);
                txt_extractor::extract_text(&self.bytes, &mut text).await;
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
        let path = format!("data/{}.txt", url.to_string().replace("/", "_"));
        let mut file = File::create(path).unwrap();
        file.write_all(self.to_text().await.into_iter().next().unwrap().as_bytes())
            .unwrap(); 
    }
}
