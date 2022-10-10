use std::{fmt::Debug, collections::{HashMap, BTreeMap}, cell::RefCell, rc::Rc, hash::{Hash, Hasher}};
use petgraph::{ data::Build, prelude::{GraphMap, UnGraphMap}, Undirected, EdgeType, Graph, visit::{IntoNodeIdentifiers, IntoNodeReferences}};
use progress_bar::*;
use sha2::{Sha256, Digest};

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

struct PagesGraph {
    graph: UnGraphMap<[u8; 32], ()>,
    pages: HashMap<Url, Page>,
    urls: HashMap<[u8; 32], Url>,
}

impl PagesGraph {
    pub fn new() -> Self {
        PagesGraph {
            graph: UnGraphMap::new(),
            pages: HashMap::new(),
            urls: HashMap::new(),
        }
    }

    pub fn add_url(&mut self, from: Url, to: Url) {
        // Hash from
        let from_hash = from.hash_sha256();
        if !self.graph.contains_node(from_hash) {
            self.graph.add_node(from_hash);
        } 
        self.urls.insert(from_hash, from);
                
        // Hash to
        let to_hash = to.hash_sha256();
        if !self.graph.contains_node(to_hash) {
            self.graph.add_node(to_hash);
        }
        self.urls.insert(to_hash, to);
        
        self.graph.add_edge(from_hash, to_hash, ());
    }

    pub fn add_page(&mut self, page: Page) {
        self.pages.insert(page.url.clone(), page);
    }

    pub fn remove_node(&mut self, url: Url) {
        let hash = url.hash_sha256();
        self.graph.remove_node(hash);
        self.pages.remove(&url);
        self.urls.remove(&hash);
    }
   
}

mod errors {
    use super::*;

    #[derive(Debug)]
    pub enum PageError {
        ReqwestError(reqwest::Error),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_graph() {
        let mut graph = PagesGraph::new();
        graph.add_url(Url::parse(&"https://insagenda.fr").unwrap(), Url::parse(&"https://insagenda.fr/agenda").unwrap());
        graph.add_url(Url::parse(&"https://insagenda.fr/agenda").unwrap(), Url::parse(&"https://insagenda.fr").unwrap());

        assert!(graph.graph.node_count() == 2);
        
        graph.remove_node(Url::parse(&"https://insagenda.fr").unwrap());

        assert!(graph.graph.node_count() == 1);

        graph.add_url(Url::parse(&"https://insagenda.fr").unwrap(), Url::parse(&"https://insagenda.fr/agenda").unwrap());
        graph.add_url(Url::parse(&"https://insagenda.fr/agenda").unwrap(), Url::parse(&"https://insagenda.fr/login").unwrap());
        graph.add_url(Url::parse(&"https://insagenda.fr/login").unwrap(), Url::parse(&"https://insagenda.fr/register").unwrap());
        graph.add_url(Url::parse(&"https://insagenda.fr/register").unwrap(), Url::parse(&"https://insagenda.fr/login").unwrap());
    }
}