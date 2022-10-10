use std::collections::HashMap;

use crate::link::{Url, get_links};

pub enum ContentType {
    Html,
    Pdf,
}

pub struct Content {
    bytes: String,
    kind: ContentType,
}
impl Content {
    pub fn new(bytes: String, kind: ContentType) -> Self {
        Content {
            bytes,
            kind,
        }
    }

    pub fn get_links(&self) -> HashMap<Url, ()> {
        match self.kind {
            ContentType::Html => {
                get_links(&self.bytes)
            }
            ContentType::Pdf => todo!("get links from pdf"),
        }
    }
}
