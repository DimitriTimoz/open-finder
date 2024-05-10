use futures::{self, lock::Mutex};
use progress_bar::*;
use reqwest::{Client, ClientBuilder};
use rpassword::read_password;
use std::{
    collections::{HashSet, VecDeque}, fmt::Debug, fs::{self, File, OpenOptions}, io::Write, rc::Rc, sync::{atomic::{AtomicU32, Ordering}, Arc}, time::Duration
};
use urlencoding;

// TODO: blacklist personal pages
use crate::{
    content::Content,
    link::{HackTraitVecUrlString, Url},
    protocols::UriScheme,
};
use errors::PageError::{self, *};
pub struct Page {
    url: Url,
    referers: HashSet<Url>,
    links: HashSet<Url>,
    content: Option<Content>,
    client: Arc<Mutex<Client>>,
    status: u16,
}

const CONCURRENT_REQUESTS: usize = 20;

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
    pub async fn new(url: Url, client: Arc<Mutex<Client>>) -> Result<Self, PageError> {
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
        let res = self
            .client.lock().await
            .get(&self.url.to_string())
            .send()
            .await
            .map_err(ReqwestError)?;

    
        if Url::parse(res.url().to_string())
            .map_err(|_| PageError::InvalidFinalUrl)?
            .is_cas()
        {
            let cas_res = self.login_cas().await;
            if cas_res.is_err() {
                print_progress_bar_info(
                    "CAS",
                    &format!("Failed cas login {:?}", cas_res.as_ref().err().unwrap()),
                    Color::Red,
                    Style::Bold,
                );
                cas_res?;
            }
        } else {
            // get links from the page
            self.status = res.status().as_u16();
            let bytes = res.bytes().await.map_err(ReqwestError)?;
            self.content = Some(Content::new(bytes.into_iter().collect(), self.url.get_file_name()));
        }

        self.links = if let Some(content) = &self.content {
            content.get_links(self.url.clone())
        } else {
            HashSet::<Url>::new()
        };

        if let Some(content) = self.content.as_ref() {
            content.publish(&[self.url.clone()]);
            content.save(self.url.clone()).await;
        }

        self.links.remove(&self.url);

        Ok(())
    }

    fn is_cas(&self) -> bool {
        self.url.is_cas()
    }

    pub async fn login_cas(&mut self) -> Result<(), PageError> {
        // Pull the current page and get the execution
        let res = self
            .client.lock().await
            .get(&self.url.to_string())
            .send()
            .await
            .map_err(ReqwestError)?;

        let execution = if let Ok(content) = res.text().await {
            if !content.contains("name=\"execution\" value=\"") {
                return Err(NotContainsExecution);
            }

            content
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
        let username = std::env::var("CAS_USERNAME").unwrap_or_else(|_| {
            print!("Username: ");
            std::io::stdout().flush().unwrap();
            read_password().unwrap()
        });

        let password = std::env::var("CAS_PASSWORD").unwrap_or_else(|_| {
            print!("Password: ");
            std::io::stdout().flush().unwrap();
            read_password().unwrap()
        });

        let req = self.client
                .lock().await
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

        let res = self.client.lock().await.execute(req).await.map_err(ReqwestError)?;

        if !res.status().is_success() {
            return Err(FailedToLogin);
        }
        self.content = Some(Content::new(
            res.bytes().await.into_iter().flatten().collect(),
            self.url.get_file_name(),
        ));
        print_progress_bar_final_info("CAS", "Login successful", Color::Green, Style::Bold);
        Ok(())
    }

    pub fn get_status(&self) -> u16 {
        self.status
    }

    pub fn get_url(&self) -> &Url {
        &self.url
    }

    pub fn get_content(&self) -> Option<&Content> {
        self.content.as_ref()
    }
}

pub struct UrlCollection {
    to_fetch: VecDeque<Url>,
    known_url_hash: HashSet<u64>,
    client: Arc<Mutex<Client>>,
    #[cfg(feature = "graph")]
    last_fetch: Vec<(Url, Url)>,
    i: usize,
    to_save: Vec<(Url, u16)>,
}

impl Default for UrlCollection {
    fn default() -> Self {
        UrlCollection {
            to_fetch: VecDeque::with_capacity(2 * 1024 * 1024),
            known_url_hash: HashSet::with_capacity(7 * 1024 * 1024),
            client: Arc::new(Mutex::new(ClientBuilder::new().cookie_store(true).timeout(Duration::from_secs(2)).build().unwrap())),
            i: 0,
            #[cfg(feature = "graph")]
            last_fetch: Vec::new(),
            to_save: Vec::new(),
        }
    }
}

impl UrlCollection {
    pub fn new() -> Self {
        UrlCollection::default()
    }

    /// Add a not fetched url with a referer
    pub fn add_url_to_fetch_with_referer(&mut self, _from: Url, to: Url, _status: u16) {
        if !self.known_url_hash.contains(&to.get_hash()) {
            self.known_url_hash.insert(to.get_hash());
            self.to_fetch.push_back(to.clone());
        }
        #[cfg(feature = "graph")]
        self.last_fetch.push((from, to));
    }

    /// Add a not fetched url
    pub fn add_url_to_fetch(&mut self, url: Url) {
        if !self.known_url_hash.contains(&url.get_hash()) {
            self.known_url_hash.insert(url.get_hash());
            self.to_fetch.push_back(url.clone());
        }
    }

    /// Get the number of links
    pub fn get_links_count(&self) -> usize {
        self.known_url_hash.len()
    }

    /// Start the fetch
    pub async fn fetch(&mut self) -> Result<(), PageError> {

        init_progress_bar(self.get_links_count());
        set_progress_bar_action("Fetching", Color::Green, Style::Bold);
        let mut ongoing_requests = vec![];

        let package_i = AtomicU32::new(1);
        
        // Find the highest package number
        let mut package_string = format!("data/package-{}.7z", package_i.load(Ordering::Acquire));
        while std::path::Path::new(&package_string).exists() {
            package_i.fetch_add(1, Ordering::SeqCst);
            package_string = format!("data/package-{}.7z", package_i.load(Ordering::Acquire));
        }
            // Start a new thread for the compression
        let compression_thread = std::thread::spawn(move || {
            // Ensure the package directory exists
            let _ = fs::create_dir(format!("data/package-{}", package_i.load(Ordering::Acquire)));
            
            loop {
                // Count the number of files and their size
                let mut size = 0;
                let mut n_files = 0;
                package_string = format!("data/package-{}", package_i.load(Ordering::Acquire));
                if let Ok(entries) = fs::read_dir(&package_string) {
                    for entry in entries.flatten() {
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.is_file() {
                                size += metadata.len();
                                n_files += 1;
                            }
                        }
                    }
                }            
                
                // Compress the files
                if size > 512 * 1024 * 1024 || n_files > 1_000 {
                    package_i.fetch_add(1, Ordering::SeqCst);
                    fs::create_dir(&format!("data/package-{}", package_i.load(Ordering::Acquire))).unwrap();
                    let _ = sevenz_rust::compress_to_path(&package_string, &format!("data/package-{}.7z", package_i.load(Ordering::Acquire)));
                    fs::remove_dir_all(&package_string).unwrap();
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        });

        while !self.to_fetch.is_empty() || !ongoing_requests.is_empty() {
            if ongoing_requests.len() < CONCURRENT_REQUESTS {
                while let Some(url) = self.to_fetch.pop_front() {
                    let url = url.clone();
                    self.known_url_hash.insert(url.get_hash());
                    self.i += 1;
                    if (url.get_uri_scheme() == UriScheme::Http
                        || url.get_uri_scheme() == UriScheme::Https)
                        && url.is_media()
                        || !url.is_allowed()
                        || url.is_black_listed()
                        || url.to_string().contains("mailto")
                        || url.to_string().ends_with("logout")

                    {
                        print_progress_bar_info(
                            "Skip",
                            &url.to_string(),
                            Color::Yellow,
                            Style::Bold,
                        );
                        self.to_save.push((url.clone(), 0));
                        continue;
                    }
                    ongoing_requests.push(Box::pin(Page::new(url.clone(), Arc::clone(&self.client))));
                    if ongoing_requests.len() >= CONCURRENT_REQUESTS {
                        break;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(1));

            if ongoing_requests.is_empty() {
                print_progress_bar_info("Empty queue", "No request", Color::Cyan, Style::Normal);
                continue;
            }

            let (page, _, remaining_requests) = futures::future::select_all(ongoing_requests).await;
            ongoing_requests = remaining_requests;
            inc_progress_bar();

            let page = if let Ok(page) = page {
                page
            } else {
                print_progress_bar_info(
                    "Error",
                    &format!("{:?}", page.err().unwrap()),
                    Color::Red,
                    Style::Bold,
                );
                continue;
            };

            page.links.iter().for_each(|link| {
                self.add_url_to_fetch_with_referer(
                    page.url.clone(),
                    link.clone(),
                    page.get_status(),
                );
            });
            self.to_save.push((page.url.clone(), page.get_status()));
            if self.to_save.len() > 300 {
                self.save_graph();
            }

            set_progress_bar_max(self.get_links_count());
            print_progress_bar_info(
                "Fetched",
                &page.get_url().to_string(),
                Color::Blue,
                Style::Bold,
            );
        }
        finalize_progress_bar();
        self.save_graph();
        Ok(())
    }

    /// Fetch all pages
    pub async fn fetch_from(&mut self, starts: Vec<Url>) -> Result<(), PageError> {
        for url in starts {
            self.add_url_to_fetch(url);
        }

        self.fetch().await
    }

    /// Save the graph to a file
    pub fn save_graph(&mut self) {
        // Check if the file exists and contains the header
        let mut file_fetcheds = OpenOptions::new()
            .append(true)
            .open("fetcheds.csv")
            .unwrap_or_else(|_| {
                let mut file = File::create("fetcheds.csv").unwrap();
                file.write_all(b"status;label\n").unwrap();
                file
            });

        let mut file_to_fetch = OpenOptions::new()
            .append(false)
            .open("to_fetch.csv")
            .unwrap_or_else(|_| {
                let mut file = File::create("to_fetch.csv").unwrap();
                file.write_all(b"url\n").unwrap();
                file
            });

        #[cfg(feature = "graph")]
        let mut file_edges = OpenOptions::new()
            .append(true)
            .open("edges.csv")
            .unwrap_or_else(|_| {
                let mut file = File::create("edges.csv").unwrap();
                file.write_all(b"source;target\n").unwrap();
                file
            });

        let mut fetcheds_csv: Vec<String> = vec![];
        let mut to_fetch_csv: Vec<String> = vec![];
        #[cfg(feature = "graph")]
        let mut edges_csv: Vec<String> = vec![];

        #[cfg(feature = "graph")]
        for (from, to) in self.last_fetch.iter() {
            edges_csv.push(format!("{};{}", from, to));
        }
        #[cfg(feature = "graph")]
        self.last_fetch.clear();

        for url in self.to_fetch.iter() {
            to_fetch_csv.push(format!("{}", url));
        }

        for (url, status) in self.to_save.iter() {
            fetcheds_csv.push(format!("{};{}", status, url));
        }

        self.to_save.clear();
        // Append the fetcheds to the file
        file_fetcheds
            .write_all(fetcheds_csv.join("\n").as_bytes())
            .unwrap();
        file_fetcheds.write_all(b"\n").unwrap();

        // Append the to_fetch to the file
        file_to_fetch
            .write_all(to_fetch_csv.join("\n").as_bytes())
            .unwrap();
        file_to_fetch.write_all(b"\n").unwrap();

        // Append the edges to the file
        #[cfg(feature = "graph")]
        file_edges
            .write_all(edges_csv.join("\n").as_bytes())
            .unwrap();
        #[cfg(feature = "graph")]
        file_edges.write_all(b"\n").unwrap();
    }

    /// Load the graph from a file
    pub async fn load_graph(&mut self) {
        // Check if the files exists
        if File::open("to_fetch.csv").is_err() || File::open("fetcheds.csv").is_err() {
            return;
        }

        // Load the to_fetch
        let to_fetch = std::fs::read_to_string("to_fetch.csv").unwrap();
        for url in to_fetch.lines().skip(1) {
            if let Ok(url) = Url::parse(url) {
                self.add_url_to_fetch(url);
            }
        }

        // Load the fetcheds
        let fetcheds = std::fs::read_to_string("fetcheds.csv").unwrap();
        for line in fetcheds.lines().skip(1) {
            let mut parts = line.split(';');
            let url = parts.nth(1).unwrap();
            if let Ok(url) = Url::parse(url) {
                self.known_url_hash.insert(url.get_hash());
                self.i += 1;
            }
        }
    }
}

mod errors {
    use super::*;

    #[derive(Debug)]
    pub enum PageError {
        ReqwestError(reqwest::Error),
        NotContainsExecution,
        FailedToLogin,
        InvalidFinalUrl,
    }
}
#[cfg(test)]
mod tests {
    use reqwest::ClientBuilder;

    use super::*;

    #[tokio::test]
    async fn test_login_cas() {
        let client = Arc::new(Mutex::new(ClientBuilder::new().cookie_store(true).build().unwrap()));

        let mut page = Page::new(Url::parse("https://cas.insa-rouen.fr/cas/login?service=https%3A%2F%2Fmoodle.insa-rouen.fr%2Flogin%2Findex.php%3FauthCAS%3DCAS").unwrap(), client).await.unwrap();

    }
}
