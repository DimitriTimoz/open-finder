use std::collections::HashMap;

use crate::link::{get_links, Url};

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

impl From<String> for ContentType {
    fn from(file_name: String) -> Self {
        let file_name = file_name.to_lowercase();
        match file_name.split('.').last() {
            Some("html") => ContentType::Html,
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
            _ => ContentType::Other,
        }
    }
}
pub struct Content {
    name: String,
    bytes: String,
    kind: ContentType,
}
impl Content {
    pub fn new(bytes: String, name: String) -> Self {
        Content { bytes, kind: ContentType::from(name.clone()), name }
    }
    
    pub fn get_links(&self) -> HashMap<Url, ()> {
        match self.kind {
            ContentType::Html => get_links(&self.bytes),
            ContentType::Pdf => todo!("get links from pdf"),
            _ => HashMap::new(),
        }
    }
}
