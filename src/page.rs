use std::fmt::Debug;

use crate::{
    content::{Content, ContentType},
    link::Url,
};
use errors::PageError::*;

use self::errors::PageError;


pub struct Page {
    url: Url,
    referers: Vec<Url>,
    links: Vec<Url>,
    content: Content,
}

impl Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Page {{ url: {}, referers: {:?}, links: {:?} }}", self.url.to_string(), self.referers, self.links)
    }
    
}

impl Page {
    pub async fn new(url: Url) -> Result<Self, PageError> {
        let mut page = Page {
            url,
            referers: Vec::new(),
            links: Vec::new(),
            content: Content::new(String::new(), ContentType::Html),
        };
        page.fetch().await?;        
        Ok(page)
    }


    async fn fetch(&mut self) -> Result<(), PageError> {
        let client = reqwest::ClientBuilder::new()
            .gzip(true)
            .build()
            .map_err(ReqwestError)?;

        let res = client
            .get(&self.url.to_string())
            .send()
            .await
            .map_err(ReqwestError)?;

        // get links from the page
        let bytes = res.text().await.map_err(ReqwestError)?;
        self.content = Content::new(bytes, ContentType::Html);
        self.links = self.content.get_links();
        Ok(())
    }
}

mod errors {
    use super::*;

    #[derive(Debug)]
    pub enum PageError {
        ReqwestError(reqwest::Error),
    }
}
