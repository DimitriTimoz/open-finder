use sha2::{Sha256, Digest};

use crate::link::errors::UrlError;
use core::fmt::Debug;
use std::{collections::HashMap, hash::Hasher, hash::Hash, fmt::Display};


#[derive(Clone, PartialOrd, Ord)]
pub struct Url {
    url: String,
    host: String,
}

impl Hash for Url {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

impl Url {
    pub fn hash_sha256(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.url.clone());
        let mut hash: [u8; 32] = Default::default();
        hash.copy_from_slice(hasher.finalize().as_slice());
        hash
    }
}

impl PartialEq for Url {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for Url {}

pub trait HackTraitVecUrlString {
    fn to_string(&self) -> String;
}

impl HackTraitVecUrlString for HashMap<Url, ()> {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for url in self.keys() {
            string.push_str(&format!(" {};", &url.to_string()));
        }
        string
    }
}

impl Url {
    pub fn parse(v: &dyn ToString) -> Result<Self, UrlError> {
        use UrlError::*;

        let url = v.to_string();
        let url = url.trim_end_matches('/');
        let mut url_split = url.split("://");
        match url_split.next() {
            Some(v) => {
                if v.is_empty() {
                    return Err(NoProtocol);
                }
            }
            None => return Err(NotValidUrl),
        };

        let host = match url_split.next() {
            Some(host) => host.split('/').next().unwrap().to_string(),
            None => return Err(NotValidUrl),
        };
        Ok(Url {
            url: url.to_string(),
            host,
        })
    }
    /// Get the protocol of the url before the `://`
    fn get_protocol(&self) -> &str {
        self.url.split("://").next().unwrap()
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl Debug for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url)
    }
}

mod errors {
    use super::*;

    #[derive(Debug)]
    pub enum UrlError {
        NotValidUrl,
        NoProtocol,
    }
}

pub fn get_links(content: &str) -> HashMap<Url, ()> {
    let mut links = HashMap::new();
    if let Some(i) = content.find("://") {
        let mut start = i;
        let content_p = content.as_bytes();
        let mut end = content.len();
        for j in (0..i).rev() {
            if content_p[j].is_ascii_alphabetic() {
                start = j;
            } else {
                break;
            }
        }
        for j in i..end {
            if content_p[j] == b' ' || content_p[j] == b'"' || content_p[j] == b'\'' {
                end = j;
                break;
            } else {
                end = j;
            }
        }
        let link = Url::parse(&content[start..end].to_string()).unwrap();
        links.insert(link, ());
        links.extend(get_links(&content[end..]));
    }
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url() {
        let url = Url::parse(&"https://www.google.com").unwrap();

        assert_eq!(url.url, "https://www.google.com");
        assert_eq!(url.host, "www.google.com");

        let url = Url::parse(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(url.host, "www.youtube.com");
        assert_eq!(url.get_protocol(), "https");

        assert!(Url::parse(&"www.google.com").is_err());
        assert!(Url::parse(&"http:/www.google.com").is_err());
        assert!(Url::parse(&"http://www.rust-lang.org/").is_ok());

        assert!(Url::parse(&"://www.rust-lang.org/").is_err());

        assert_eq!(Url::parse(&"https://www.google.com").unwrap(), Url::parse(&"https://www.google.com/").unwrap());
    }

    #[test]
    fn test_get_links() {
        let content = r#"<a href="https://www.google.com">Google</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 1);
        assert!(links.contains_key(&Url::parse(&"https://www.google.com").unwrap()));

        // Check mutliple links
        let content = r#"<a href="https://www.google.com">Google</a><a href="https://www.youtube.com">Youtube</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 2);
        assert!(links.contains_key(&Url::parse(&"https://www.google.com").unwrap()));
        assert!(links.contains_key(&Url::parse(&"https://www.youtube.com").unwrap()));

        // No links
        let content = r#"<a href="https:/www.google.com">Google</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 0);

        // Multiple protocols http, https, ftp
        let content = r#"<a href="https://www.google.com">Google</a><a href="http://www.youtube.com">Youtube</a><a href="ftp://www.rust-lang.org">Rust</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains_key(&Url::parse(&"https://www.google.com").unwrap()));
        assert!(links.contains_key(&Url::parse(&"http://www.youtube.com").unwrap()));
        assert!(links.contains_key(&Url::parse(&"ftp://www.rust-lang.org").unwrap()));
    }
}
