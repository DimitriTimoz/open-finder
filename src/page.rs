use petgraph::prelude::UnGraphMap;
use progress_bar::*;
use std::{collections::HashMap, fmt::Debug, time::Duration, thread};

use crate::{
    content::{Content, ContentType},
    link::{HackTraitVecUrlString, Url},
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
        write!(
            f,
            " - url: {} \n - referers: {:?} \nlinks:\n{}",
            self.url,
            self.referers,
            self.links.to_string()
        )
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
        self.content = Content::new(String::new(), ContentType::Html); // TODO: save the content
        self.links.remove(&self.url);

        Ok(())
    }
}

pub struct PagesGraph {
    graph: UnGraphMap<[u8; 32], ()>,
    pages: HashMap<Url, Option<Page>>,
    urls: HashMap<[u8; 32], Url>, // key: hash of url, value: url
}

impl PagesGraph {
    pub fn new() -> Self {
        PagesGraph {
            graph: UnGraphMap::new(),
            pages: HashMap::new(),
            urls: HashMap::new(),
        }
    }

    /// Add a not fetched url
    pub fn add_url(&mut self, from: Url, to: Url) {
        // Hash from
        let from_hash = from.hash_sha256();
        if !self.graph.contains_node(from_hash) {
            self.graph.add_node(from_hash);
        }
        self.urls.insert(from_hash, from.clone());

        // From Page
        self.pages.entry(from).or_insert(None);

        // Hash to
        let to_hash = to.hash_sha256();
        if !self.graph.contains_node(to_hash) {
            self.graph.add_node(to_hash);
        }
        self.urls.insert(to_hash, to.clone());

        // To Page
        self.pages.entry(to).or_insert(None);

        // Add edge
        self.graph.add_edge(from_hash, to_hash, ());
    }

    /// Add a page to the graph withouth referer
    pub fn add_page(&mut self, page: Page) {
        let from_url = page.url.clone();
        for link in page.links.keys() {
            let url = link.clone();

            self.add_url(from_url.clone(), url);

        }
        self.pages.insert(from_url, Some(page));
    }

    /// Add a page to the graph with a referer
    pub fn add_page_with_referer(&mut self, page: Page, referer: Url) {
        let url = page.url.clone();
        self.add_url(referer, url);
        self.add_page(page);
    }

    /// Remove an url from the graph
    pub fn remove_node(&mut self, url: Url) {
        let hash = url.hash_sha256();
        self.graph.remove_node(hash);
        self.pages.remove(&url);
        self.urls.remove(&hash);
    }

    /// Closest to fetch first
    pub fn get_closest_url_to_fetch(&self, start: Url) -> Vec<Url> {
        let mut visited = HashMap::new();
        self.get_closest_url_to_fetch_recursion(start, &mut visited)
    }

    // Recursion
    fn get_closest_url_to_fetch_recursion(
        &self,
        start: Url,
        visited: &mut HashMap<[u8; 32], ()>,
    ) -> Vec<Url> {
        let start = start.hash_sha256();
        let neighbors = self.graph.neighbors(start);
        let mut to_fetch = Vec::new();
        visited.insert(start, ());

        for neighbor in neighbors {
            if visited.contains_key(&neighbor) {
                continue;
            }
            if let Some(url) = self.urls.get(&neighbor) {
                if let Some(page) = self.pages.get(url) {
                    if page.is_none() {
                        to_fetch.push(url.clone());
                    }
                }
                to_fetch.extend(self.get_closest_url_to_fetch_recursion(url.clone(), visited));
            } else {
                println!("Error: neighbor not found");
            }
        }

        to_fetch
    }

    pub fn get_links_count(&self) -> u32 {
        self.urls.len() as u32
    }

    /// Fetch all pages
    pub async fn fetch_form(&mut self, start: Url, max_distance: u8) -> Result<(), PageError> {
        let page = Page::new(start.clone()).await?;
        init_progress_bar(1);
        set_progress_bar_action("Fetching", Color::Green, Style::Bold);

        self.add_page(page);

        for _ in 0..max_distance {
            let to_fetch = self.get_closest_url_to_fetch(start.clone());
            for url in to_fetch {
                set_progress_bar_max(self.get_links_count().try_into().unwrap());

                let page = Page::new(url.clone()).await?;
                self.add_page_with_referer(page, url.clone());
                print_progress_bar_info("Fetched", &url.to_string(), Color::Blue, Style::Bold);
                inc_progress_bar();
                thread::sleep(Duration::from_millis(100));
            }
            
        }
        set_progress_bar_action("Fetched", Color::Green, Style::Bold);
        finalize_progress_bar();
        Ok(())
    }

    /// Get all links in the graph
    pub fn get_links(&self) -> Vec<Url> {
        self.urls.values().cloned().collect()
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
        graph.add_url(
            Url::parse(&"https://example.com").unwrap(),
            Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
        );
        graph.add_url(
            Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            Url::parse(&"https://example.com").unwrap(),
        );

        assert!(graph.graph.node_count() == 2);

        graph.remove_node(Url::parse(&"https://example.com").unwrap());

        assert!(graph.graph.node_count() == 1);

        graph.add_url(
            Url::parse(&"https://example.com").unwrap(),
            Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
        );
        graph.add_url(
            Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            Url::parse(&"https://example.com/login").unwrap(),
        );
        graph.add_url(
            Url::parse(&"https://example.com/login").unwrap(),
            Url::parse(&"https://example.com/register").unwrap(),
        );
        graph.add_url(
            Url::parse(&"https://example.com/register").unwrap(),
            Url::parse(&"https://example.com/login").unwrap(),
        );
    }

    #[tokio::test]
    async fn test_to_fetch_list() {
        let mut graph = PagesGraph::new();
        graph.add_url(
            Url::parse(&"https://example.com").unwrap(),
            Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
        );
        graph.add_url(
            Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            Url::parse(&"https://example.com").unwrap(),
        );

        let list = graph.get_closest_url_to_fetch(Url::parse(&"https://example.com").unwrap());
        assert!(list.len() == 1);

        graph.add_page(
            Page::new(Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap())
                .await
                .unwrap(),
        );
        let list = graph.get_closest_url_to_fetch(Url::parse(&"https://example.com").unwrap());
        assert!(!list.is_empty());

        let mut graph = PagesGraph::new();
        graph.add_page(
            Page::new(Url::parse(&"https://example.com").unwrap())
                .await
                .unwrap(),
        );
        let list = graph.get_closest_url_to_fetch(Url::parse(&"https://example.com").unwrap());

        assert!(!list.is_empty());
    }
}
