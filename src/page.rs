use std::{fmt::Debug, collections::HashMap};
use progress_bar::*;

use crate::{
    content::{Content, ContentType},
    link::{Url, HackTraitVecUrlString},
};
use errors::PageError::*;

use self::errors::PageError;


pub struct Page {
    url: Url,
    referers: HashMap<Url, ()>,
    links: HashMap<Url, ()>,
    content: Content,
}

impl Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, " - url: {} \n - referers: {:?} \nlinks:\n{}", self.url.to_string(), self.referers, self.links.to_string())
    }
    
}

impl Page {
    pub async fn new(url: Url) -> Result<Self, PageError> {
        let mut page = Page {
            url,
            referers: HashMap::new(),
            links: HashMap::new(),
            content: Content::new(String::new(), ContentType::Html),
        };
        page.fetch().await?;        
        Ok(page)
    }


    async fn fetch(&mut self) -> Result<(), PageError> {
        init_progress_bar(2);
        set_progress_bar_action("Loading", Color::Green, Style::Bold);

        let client = reqwest::ClientBuilder::new()
            .gzip(true)
            .build()
            .map_err(ReqwestError)?;

        let res = client
            .get(&self.url.to_string())
            .send()
            .await
            .map_err(ReqwestError)?;
        inc_progress_bar();
        set_progress_bar_action("Parsing", Color::Green, Style::Bold);

        // get links from the page
        let bytes = res.text().await.map_err(ReqwestError)?;
        self.content = Content::new(bytes, ContentType::Html);
        self.links = self.content.get_links();
        self.links.remove(&self.url);
        print_progress_bar_info("Success", &self.url.to_string(), Color::Green, Style::Bold);
        
        inc_progress_bar();
        finalize_progress_bar();

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
