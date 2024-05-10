use serde::{Deserialize, Serialize};

use crate::{link::errors::UrlError, protocols::UriScheme};
use core::fmt::Debug;
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    fmt::Display,
    hash::Hash,
    hash::Hasher,
};

#[derive(Clone, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Url {
    url: String,
}

impl Hash for Url {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

impl Url {
    pub fn get_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    pub fn is_black_listed(&self) -> bool {
        self.url
            .starts_with("https://catalogue.insa-rouen.fr/cgi-bin/koha/opac-search.pl")
            || self.url.starts_with("https://qualite.insa-rouen.fr")
    }

    pub fn is_media(&self) -> bool {
        const MEDIAS_EXTENSIONS: [&str; 18] = [
            "png", "jpg", "jpeg", "gif", "svg", "ico", "webp", "bmp", "tiff", "tif", "psd",
            "raw", "css", "js", "zip", "tar", "jar", "webm"
        ];
        if let Some(extension) = self
            .to_string()
            .split('.')
            .last()
            .map(|ext| ext.to_lowercase())
        {
            // Vérifier si l'extension extraite est dans le tableau des extensions de médias
            MEDIAS_EXTENSIONS
                .iter()
                .any(|&media_ext| media_ext == extension)
        } else {
            false
        }
    }
    pub fn is_allowed(&self) -> bool {
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
            string.push_str(&format!(" {};", &url.to_owned()));
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
            if !is_url_permissive(c as u8) {
                url = url[..i].to_string();
                break;
            }
        }

        let url = url.trim_end_matches('/').trim_end_matches('#');
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

    // Get the root of the url
    #[inline]
    pub fn get_root(&self) -> String {
        // Example: https://www.google.com/path/to/file -> https://www.google.com
        let mut root = self.url.split("://").next().unwrap().to_string();
        root.push_str("://");
        root.push_str(self.get_host());
        root.push('/');
        root
    }

    /// Get the file name
    #[inline]
    pub fn get_file_name(&self) -> String {
        self.url.split('/').last().unwrap().to_string()
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

/*
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
}*/

#[inline]
fn is_url_permissive(c: u8) -> bool {
    c.is_ascii_alphanumeric()
        || c == b'.'
        || c == b'/'
        || c == b'%'
        || c == b'?'
        || c == b':'
        || c == b'='
        || c == b'-'
        || c == b'_'
        || c == b'&'
}

/// Parse all the links in the content
/// even if the link is not http or https
/// for example: ="/path/to/file" should be parsed to "https://www.example.com/path/to/file"
pub fn get_links(content: &str, url: Url) -> HashSet<Url> {
    let mut links = HashSet::new();
    // Matching patterns
    let mut start: usize = 0;
    let mut pattern_matching = false;
    let mut pattern_matching_pos = 0;
    let host = url.get_root();
    let path = url.to_string();
    let mut has_get_param = false;
    for (end, c) in content.as_bytes().iter().enumerate() {
        if is_url_permissive(*c) && !(c == &b'?' && has_get_param) {
            if c == &b'?' {
                has_get_param = true;
            }
            if !pattern_matching {
                if let Some(content) = content.get(end.saturating_sub(2)..=end) {
                    if content == "://" {
                        pattern_matching = true;
                        pattern_matching_pos = end.saturating_sub(2);
                    }
                }
            }
        } else {
            let end = if c == &b'?' {
                end.saturating_sub(1)
            } else {
                end
            };
            
            // If before start is "=\""
            if let Some(url) = content.get(start..=end) {
                let mut url = url.to_string();
                if let Some(content) =
                    content.get(start.saturating_sub(6)..=start.saturating_sub(1))
                {
                    if (content.ends_with("src=\"") || content.ends_with("href=\""))
                        && (pattern_matching_pos.saturating_sub(start) >= 6 || !pattern_matching)
                    {
                        if url.starts_with('/') {
                            url = url[1..].to_string();
                            url = format!("{}{}", host, url);
                        } else {
                            url = format!("{}{}", path, url);
                        }

                        pattern_matching = true;
                    }
                }

                if start <= end && pattern_matching {
                    if let Ok(link) = Url::parse(url) {
                        links.insert(link);
                    }
                }
            }
            has_get_param = false;
            pattern_matching = false;
            start = end + 1;
        }
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
            get_links(
                "https://sentry.io}",
                Url::parse("https://sentry.io").unwrap()
            )
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
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        assert_eq!(links.len(), 1);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));

        // Check mutliple links
        let content = r#"<a href="https://www.google.com">Google</a><a href="https://www.youtube.com">Youtube</a>"#;
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        assert_eq!(links.len(), 2);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));
        assert!(links.contains(&Url::parse("https://www.youtube.com").unwrap()));

        // No links
        let content = r#"<a href="/">Google</a>"#;
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        assert_eq!(links.len(), 1);

        // Multiple protocols http, https, ftp
        let content = r#"<a href="https://www.google.com">Google</a><a href="http://www.youtube.com">Youtube</a><a href="ftp://www.rust-lang.org">Rust</a>"#;
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        assert_eq!(links.len(), 3);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));
        assert!(links.contains(&Url::parse("http://www.youtube.com").unwrap()));
        assert!(links.contains(&Url::parse("ftp://www.rust-lang.org").unwrap()));

        // Multiple links with special characters
        let content =
            r#""https://www.google.com", "https://www.youtube.com", "ftp://www.rust-lang.org""#;
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        assert_eq!(links.len(), 3);
        assert!(links.contains(&Url::parse("https://www.google.com").unwrap()));
        assert!(links.contains(&Url::parse("https://www.youtube.com").unwrap()));
        assert!(links.contains(&Url::parse("ftp://www.rust-lang.org").unwrap()));

        // Test href with an url as get parameter
        let content = r#"<a href="?link=https://www.youtube.com">Google</a>"#;
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        println!("{:?}", links);
        assert_eq!(links.len(), 1);
        assert!(links
            .contains(&Url::parse("https://www.google.com?link=https://www.youtube.com").unwrap()));
    }

    #[test]
    fn test_link_as_absolute_path() {
        let content = r#"<a href="/path/to/file">Google</a>"#;
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        assert_eq!(links.len(), 1);
        println!("{:?}", links);
        assert!(links.contains(&Url::parse("https://www.google.com/path/to/file").unwrap()));


        let content = r#"<a href="/path/to/file?a?b">Google</a>"#;
        let links = get_links(content, Url::parse("https://www.google.com").unwrap());
        assert_eq!(links.len(), 1);
        println!("{:?}", links);
        assert!(links.contains(&Url::parse("https://www.google.com/path/to/file?a").unwrap()));

    }
}
