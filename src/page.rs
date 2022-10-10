use std::{fmt::Debug, collections::HashMap};
use petgraph::Graph;
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

struct PagesGraph {
    graph: Graph<Url, i32>,
    pages: HashMap<Url, Page>,
}

impl PagesGraph {
    pub fn new() -> Self {
        PagesGraph {
            graph: Graph::new(),
            pages: HashMap::new(),
        }
    }

    pub fn add_url(&mut self, from: Url, to: Url) {
        let from_node = self.graph.add_node(from);
        let to_node = self.graph.add_node(to);
        self.graph.add_edge(from_node, to_node, 1);
    }

    pub fn add_page(&mut self, page: Page) {
        self.pages.insert(page.url.clone(), page);
    }

    pub fn remove_node(&mut self, url: Url) {
        let node = self.graph.node_indices().find(|n| self.graph[*n] == url).unwrap();
        self.graph.remove_node(node);
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
        graph.add_url(Url::parse(&"https://insagenda.fr/agenda").unwrap(), Url::parse(&"https://insagenda.fr/").unwrap());
        println!("{:?}", graph.graph);
    }
}