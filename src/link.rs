use sha1::{Digest, Sha1};

use crate::{link::errors::UrlError, protocols::UriScheme};
use core::fmt::Debug;
use std::{collections::HashSet, fmt::Display, hash::Hash, hash::Hasher};

#[derive(Clone, PartialOrd, Ord)]
pub struct Url {
    url: String,
}

impl Hash for Url {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

impl Url {
    pub fn hash_sha128(&self) -> [u8; 20] {
        let mut hasher = Sha1::new();
        hasher.update(self.url.clone());
        let mut hash: [u8; 20] = Default::default();
        hash.copy_from_slice(hasher.finalize().as_slice());
        hash
    }

    pub fn is_black_listed(&self) -> bool {
        self.url.starts_with("https://catalogue.insa-rouen.fr/cgi-bin/koha/opac-search.pl")
    }

    pub fn is_media(&self) -> bool {
        let extension = if self.url.contains('?') {
            let url = self.url.split('?').next().unwrap();
            url
        } else {
            self.url.as_str()
        };
        let Some(extension) = extension.split('.').last() else { return false; };

        matches!(extension.to_lowercase().as_str(), "pdf" | "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" | "bmp" | "tiff" | "tif" | "psd" | "raw" | "css" | "js")
    }

    pub fn is_insa(&self) -> bool {
        self.url.contains("insa-rouen.fr")
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

impl HackTraitVecUrlString for HashSet<Url> {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for url in self.iter() {
            string.push_str(&format!(" {};", &url.to_string()));
        }
        string
    }
}

impl Url {
    pub fn parse(v: impl ToString) -> Result<Self, UrlError> {
        use UrlError::*;

        let mut url = v.to_string();
        if !url.contains("://") {
            return Err(NoProtocol);
        }
        
        for (i, c) in url.clone().chars().enumerate() {
            if !c.is_ascii()  {
                url = url[..i].to_string();
                break;
            }
        }

        // TODO: clear trim_end_matches
        let binding = url
            .replace('>', "");
        let url = binding
            .trim_end_matches('/')
            .trim_end_matches('\\')
            .trim_end_matches('"')
            .trim_matches('}');
        let mut url_split = url.split("://");
        match url_split.next() {
            Some(v) => {
                if v.is_empty() {
                    return Err(NoProtocol);
                }
            }
            None => return Err(NotValidUrl),
        };

        // Escape the url
        let url = url.replace(';', "%3B");

        Ok(Url {
            url: url.to_string(),
        })
    }
    /// Get the protocol of the url before the `://`
    #[inline]
    pub fn get_uri_scheme(&self) -> UriScheme {
        UriScheme::from(self.url.split("://").next().unwrap().to_string())
    }
    /// Get the host of the url
    #[inline]
    pub fn get_host(&self) -> &str {
        self.url
            .split("://")
            .nth(1)
            .unwrap()
            .split('/')
            .next()
            .unwrap()
    }

    /// Get the file name 
    #[inline]
    pub fn get_file_name(&self) -> String {
    self.url
            .split('/')
            .last()
            .unwrap().to_string()
    }

    #[inline]
    pub fn is_cas(&self) -> bool {
        self.url.contains("://cas.insa-rouen.fr")
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

pub fn get_links(content: &str) -> HashSet<Url> {
    let mut links = HashSet::new();
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
        for (j, c) in content_p.iter().enumerate().take(end).skip(i) {
            let escape = b"\n ,\"'()<>\r";
            end = j;
            if !escape.is_ascii() ||  escape.contains(c){
                break;
            }
        }
        if let Some(url) = content.get(start..end) {
            if let Ok(link) = Url::parse(url) {
                links.insert(link);
            }
        }
        if let Some(v) = content.get(end..) { links.extend(get_links(v)); }
    }
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url() {
        let url = Url::parse("https://www.google.com").unwrap();

        assert_eq!(url.url, "https://www.google.com");
        assert_eq!(url.get_host(), "www.google.com");

        let url = Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(url.get_host(), "www.youtube.com");

        assert!(Url::parse("www.google.com").is_err());
        assert!(Url::parse("http:/www.google.com").is_err());
        assert!(Url::parse("http://www.rust-lang.org/").is_ok());

        assert!(Url::parse("://www.rust-lang.org/").is_err());

        assert_eq!(
            Url::parse("https://www.google.com").unwrap(),
            Url::parse("https://www.google.com/").unwrap()
        );

        assert_eq!(
            get_links("https://sentry.io/}")
                .iter()
                .next()
                .unwrap()
                .to_string(),
            Url::parse("https://sentry.io").unwrap().to_string()
        );
    }

    #[test]
    fn test_get_uri_scheme() {
        assert_eq!(
            Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
                .unwrap()
                .get_uri_scheme(),
            UriScheme::Https
        );
        assert_eq!(
            Url::parse("ftp://example.com").unwrap().get_uri_scheme(),
            UriScheme::Ftp
        );
        assert_eq!(
            Url::parse("Sftp://example.com").unwrap().get_uri_scheme(),
            UriScheme::Sftp
        );
    }

    #[test]
    fn test_get_links() {
        let content = r#"<a href="https://www.google.com">Google</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 1);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));

        // Check mutliple links
        let content = r#"<a href="https://www.google.com">Google</a><a href="https://www.youtube.com">Youtube</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 2);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));
        assert!(links.contains(&Url::parse("https://www.youtube.com").unwrap()));

        // No links
        let content = r#"<a href="https:/www.google.com">Google</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 0);

        // Multiple protocols http, https, ftp
        let content = r#"<a href="https://www.google.com">Google</a><a href="http://www.youtube.com">Youtube</a><a href="ftp://www.rust-lang.org">Rust</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));
        assert!(links.contains(&Url::parse("http://www.youtube.com").unwrap()));
        assert!(links.contains(&Url::parse("ftp://www.rust-lang.org").unwrap()));

        // Multiple links with special characters
        let content = r#""https://www.google.com", "https://www.youtube.com", "ftp://www.rust-lang.org""#;
        let links = get_links(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));
        assert!(links.contains(&Url::parse("https://www.youtube.com").unwrap()));
        assert!(links.contains(&Url::parse("ftp://www.rust-lang.org").unwrap()));
    }
}
