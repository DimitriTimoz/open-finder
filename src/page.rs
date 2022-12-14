use petgraph::prelude::UnGraphMap;
use progress_bar::*;
use std::{collections::HashMap, fmt::Debug, fs::File, io::Write, thread, time::Duration};

use crate::{
    content::Content,
    link::{HackTraitVecUrlString, Url},
};
use errors::PageError::{self, *};

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
            url: url.clone(),
            referers: HashMap::new(),
            links: HashMap::new(),
            content: Content::new(String::new(), url.get_file_name()),
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
        self.content = Content::new(bytes,  self.url.get_file_name());
        self.links = self.content.get_links();
        self.content = Content::new(String::new(), self.url.get_file_name()); // TODO: save the content
        self.links.remove(&self.url);

        Ok(())
    }
}

#[derive(Default)]
pub struct PagesGraph {
    graph: UnGraphMap<[u8; 20], ()>,
    pages: HashMap<Url, Option<Page>>,
    urls: HashMap<[u8; 20], Url>, // key: hash of url, value: url
}

impl PagesGraph {
    pub fn new() -> Self {
        PagesGraph::default()
    }

    /// Add a not fetched url
    pub fn add_url(&mut self, from: Url, to: Url) {
        // Hash from
        let from_hash = from.hash_sha128();
        if !self.graph.contains_node(from_hash) {
            self.graph.add_node(from_hash);
        }
        self.urls.insert(from_hash, from.clone());

        // From Page
        self.pages.entry(from).or_insert(None);

        // Hash to
        let to_hash = to.hash_sha128();
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
        let hash = url.hash_sha128();
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
        visited: &mut HashMap<[u8; 20], ()>,
    ) -> Vec<Url> {
        let start = start.hash_sha128();
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
        let mut count = 0;
        self.add_page(page);

        for i in 0..max_distance {
            let to_fetch = self.get_closest_url_to_fetch(start.clone());
            print_progress_bar_info(
                &format!("{} - New links", i + 1),
                &format!("{}", to_fetch.len()),
                Color::Cyan,
                Style::Bold,
            );

            for url in to_fetch {
                if i != max_distance - 1 {
                    set_progress_bar_max(self.get_links_count().try_into().unwrap());
                }
                if count % 200 == 0 {
                    self.save_graph();
                }
                count += 1;


                let page = Page::new(url.clone()).await;
                if page.is_err() {
                    continue;
                }
                let page = page.unwrap();
                self.add_page_with_referer(page, url.clone());
                print_progress_bar_info("Fetched", &url.to_string(), Color::Blue, Style::Bold);
                inc_progress_bar();
                thread::sleep(Duration::from_millis(100));
            }
            self.save_graph();
        }
        set_progress_bar_action("Fetched", Color::Green, Style::Bold);
        finalize_progress_bar();
        Ok(())
    }

    /// Get all links in the graph
    pub fn get_links(&self) -> Vec<Url> {
        self.urls.values().cloned().collect()
    }

    /// Save the graph to a file
    pub fn save_graph(&self) {
        let mut nodes_csv: Vec<String> = vec![String::from("id;label")];
        let mut edges_csv: Vec<String> = vec![String::from("Source;Target")];

        let mut nodes: HashMap<[u8; 20], u32> = HashMap::new();
        for (i, node) in self.graph.nodes().enumerate() {
            if let Some(url) = self.urls.get(&node) {
                let url = url
                    .to_string()
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .replace('\\', "/")
                    .replace('\"', "\\\"")
                    .replace(';', "%3B");
                nodes_csv.push(format!("{};{}", i, url));
                nodes.insert(node, i as u32);
            }
        }

        for (from, to, _) in self.graph.all_edges() {
            let from = nodes.get(&from).unwrap();
            let to = nodes.get(&to).unwrap();
            edges_csv.push(format!("{};{}", from, to));
        }

        // Copy template
        let nodes_csv = nodes_csv.join("\n");
        let edges_csv = edges_csv.join("\n");
    

        // Write to file
        let mut file = File::create("nodes.csv").unwrap();
        file.write_all(nodes_csv.as_bytes()).unwrap();

        let mut file = File::create("edges.csv").unwrap();
        file.write_all(edges_csv.as_bytes()).unwrap();

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
