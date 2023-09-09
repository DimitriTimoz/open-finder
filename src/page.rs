use progress_bar::*;
use reqwest::{Client, ClientBuilder};
use urlencoding;
use rpassword::read_password;
use std::{collections::{HashSet, HashMap}, fmt::Debug, fs::{File, OpenOptions}, io::Write, rc::Rc};

use crate::{
    content::Content,
    link::{HackTraitVecUrlString, Url},
};
use errors::PageError::{self, *};
pub struct Page {
    url: Url,
    referers: HashSet<Url>,
    links: HashSet<Url>,
    content: Option<Content>,
    client: Rc<Client>,
    status: u16,
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
    pub async fn new(url: Url, client: Rc<Client>) -> Result<Self, PageError> {
        let mut page = Page {
            url: url.clone(),
            referers: HashSet::new(),
            links: HashSet::new(),
            content: None,
            client,
            status: 0,
        };
        page.fetch().await?;
        Ok(page)
    }

    async fn fetch(&mut self) -> Result<(), PageError> {
        let res = self.client
                                .get(&self.url.to_string())
                                .send()
                                .await
                                .map_err(ReqwestError)?;
                        
        // get links from the page
        self.status = res.status().as_u16();
        let bytes = res.text().await.map_err(ReqwestError)?;
        self.content = Some(Content::new(bytes,  self.url.get_file_name()));
        self.links = if let Some(content) = &self.content {
            content.get_links()
        } else {
            HashSet::<Url>::new()
        };

        if self.is_cas() {
            self.login_cas().await?;
        }

        self.links.remove(&self.url);

        Ok(())
    }

    fn is_cas(&self) -> bool {
        self.url.is_cas()
    }

    async fn login_cas(&mut self) -> Result<(), PageError> {
        // Pull the current page and get the execution        
        let execution = if let Some(content) = &self.content {
            content
                .get_bytes()
                .split("name=\"execution\" value=\"")
                .nth(1)
                .unwrap()
                .split('\"')
                .next()
                .unwrap()
                .to_string()
        } else {
            return Err(NotContainsExecution);
        };
        let username = std::env::var("CAS_USERNAME").unwrap_or_else(|_|{
            print!("Password: ");
            std::io::stdout().flush().unwrap();        
            read_password().unwrap()
        });


        let password = std::env::var("CAS_PASSWORD").unwrap_or_else(|_|{
            print!("Password: ");
            std::io::stdout().flush().unwrap();        
            read_password().unwrap()
        });


        let req = self.client
                .post(self.url.to_string())
                .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
                .header("Accept-Language", "en-US,en;q=0.5")
                .header("Accept-Encoding", "gzip, deflate, br")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Origin", "https://cas.insa-rouen.fr")
                .header("Connection", "keep-alive")
                .header("Referer", &*urlencoding::encode(self.url.to_string().as_str()))
                .header("Cookie", "org.springframework.web.servlet.i18n.CookieLocaleResolver.LOCALE=en-US")
                .header("Upgrade-Insecure-Requests", "1")
                .header("Sec-Fetch-Dest", "document")
                .header("Sec-Fetch-Mode", "navigate")
                .header("Sec-Fetch-Site", "same-origin")
                .header("Sec-Fetch-User", "?1")
                .form(&[
                    ("username", username.as_str()),
                    ("password", password.as_str()),
                    ("execution", execution.as_str()),
                    ("_eventId", "submit"),
                    ("geolocation", ""),
                    ("submit", "Login"),
                ]).build().unwrap();

        let res = self.client.execute(req).await.map_err(ReqwestError)?;
                    
        if !res.status().is_success() {
            return Err(FailedToLogin);
        }
        
        Ok(())
    }

    pub fn get_status(&self) -> u16 {
        self.status
    }
}

pub struct UrlCollection {
    to_fetch: HashSet<Url>,
    fetched: HashSet<Url>,
    client: Rc<Client>,
    last_fetch: Vec<((Url, u16), Url)>,
    i: usize,
}

impl Default for UrlCollection {
    fn default() -> Self {
        UrlCollection {
            to_fetch: HashSet::new(),
            fetched: HashSet::new(),
            client: Rc::new(ClientBuilder::new().cookie_store(true).build().unwrap()),
            i: 0,
            last_fetch: Vec::new(),
        }
    }
    
}

impl UrlCollection {
    pub fn new() -> Self {
        UrlCollection::default()
    }

    /// Add a not fetched url with a referer
    pub fn add_url_to_fetch_with_referer(&mut self, from: Url, to: Url, status: u16) {
        if !self.to_fetch.contains(&to.clone()) && !self.fetched.contains(&to.clone()){
            self.to_fetch.insert(to.clone());
        }

        self.last_fetch.push(((from, status), to));
    }

     /// Add a not fetched url
     pub fn add_url_to_fetch(&mut self, url: Url) {
        if !self.to_fetch.contains(&url) && !self.fetched.contains(&url) {
            self.to_fetch.insert(url);
        }
    }

    pub fn get_links_count(&self) -> usize {
        self.to_fetch.len() + self.fetched.len()
    }

    fn get_url_to_fetch(&self) -> impl Iterator<Item = &Url> {
        self.to_fetch.iter()
    }

    /// Fetch all pages
    pub async fn fetch_from(&mut self, starts: Vec<Url>) -> Result<(), PageError> {
        init_progress_bar(starts.len());

        for url in starts {
            self.add_url_to_fetch(url);
        }

        set_progress_bar_action("Fetching", Color::Green, Style::Bold);

        while let Some(url) = &self.to_fetch.iter().next() {  
            let url = (*url).to_owned();
            set_progress_bar_max(self.get_links_count());
            self.i += 1;
            inc_progress_bar();

            self.to_fetch.remove(&url);
            self.fetched.insert(url.clone());

            if self.i % 200 == 0 {
                self.save_graph();
            }

            if url.is_media() || !url.is_insa() {
                print_progress_bar_info("Skip", &url.to_string(), Color::Yellow, Style::Bold);
                continue;
            }

            let mut page = Page::new(url.clone(), self.client.clone()).await;
            if page.is_err() && url.is_cas() {
                page = Page::new(url.clone(), self.client.clone()).await;
                if page.is_err() {
                    print_progress_bar_info("Impossible to fetch", &url.to_string(), Color::Red, Style::Bold);
                    continue;
                }
            }

            let page = if let Ok(page) = page {
                page
            } else {
                continue;
            };
            page.links.iter().for_each(|link| {
                self.add_url_to_fetch_with_referer(page.url.clone(), link.clone(), page.get_status());
            });
            print_progress_bar_info("Fetched", &url.to_string(), Color::Blue, Style::Bold);
                        
        }
        finalize_progress_bar();
        self.save_graph();
        Ok(())
    }


    /// Save the graph to a file
    pub fn save_graph(&self) {
        // Check if the file exists and contains the header
        let mut file_nodes = OpenOptions::new()
                                    .append(true)
                                    .open("nodes.csv").unwrap_or_else(|_| {
            let mut file = File::create("nodes.csv").unwrap();
            file.write_all(b"status;label\n").unwrap();
            file
        });

        let mut file_edges = OpenOptions::new()
                                .append(true)
                                .open("edges.csv").unwrap_or_else(|_| {
            let mut file = File::create("edges.csv").unwrap();
            file.write_all(b"source;target\n").unwrap();
            file
        });

        let mut nodes_csv: Vec<String> = vec![];
        let mut edges_csv: Vec<String> = vec![];
        
        let mut nodes: HashMap<Url, u16> = HashMap::new();

        for ((from, _), to) in self.last_fetch.iter() {
            if !nodes.contains_key(from) {
                nodes.insert(from.clone(), nodes.len() as u16);
            }
            edges_csv.push(format!("{};{}",  from, to));
        }    

        for (url, status) in nodes.iter() {
            nodes_csv.push(format!("{};{}", status, url));
        }    

        // Append the nodes to the file
        file_nodes.write_all(nodes_csv.join("\n").as_bytes()).unwrap();
        
        // Append the edges to the file
        file_edges.write_all(edges_csv.join("\n").as_bytes()).unwrap();

    }
}

mod errors {
    use super::*;

    #[derive(Debug)]
    pub enum PageError {
        ReqwestError(reqwest::Error),
        NotContainsExecution,
        FailedToLogin,
    }
}
#[cfg(test)]
mod tests {
    use reqwest::ClientBuilder;

    use super::*;
    
    #[tokio::test]
    async fn test_login_cas() {
        let client = Rc::new(ClientBuilder::new().cookie_store(true).build().unwrap());
    
        let mut page = Page::new(Url::parse(&"https://cas.insa-rouen.fr/cas/login?service=https%3A%2F%2Fmoodle.insa-rouen.fr%2Flogin%2Findex.php%3FauthCAS%3DCAS").unwrap(), client).await.unwrap();
        if page.is_cas() {
            page.login_cas().await.unwrap();
        }   
    }

    #[tokio::test]
    async fn test_is_requiring_vpn() {
        let client = Rc::new(ClientBuilder::new().cookie_store(true).build().unwrap());
    
        let mut page = Page::new(Url::parse(&"https://support.insa-rouen.fr/").unwrap(), client).await.unwrap();
        if page.is_cas() {
            page.login_cas().await.unwrap();
        }   
    }
}
