use std::fmt::Debug;

use self::errors::UrlError;

pub struct Url {
    url: String,
    host: String,
}

impl Url {
    pub fn parse(v: &dyn ToString) -> Result<Self, UrlError> {
        use UrlError::*;

        let url = v.to_string();
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

impl ToString for Url {
    fn to_string(&self) -> String {
        self.url.to_string()
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

pub fn get_links(content: &str) -> Vec<Url> {
    let mut links = Vec::new();
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
        links.push(link);
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
    }

    #[test]
    fn test_get_links() {
        let content = r#"<a href="https://www.google.com">Google</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].to_string(), "https://www.google.com");

        // Check mutliple links
        let content = r#"<a href="https://www.google.com">Google</a><a href="https://www.youtube.com">Youtube</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].to_string(), "https://www.google.com");
        assert_eq!(links[1].to_string(), "https://www.youtube.com");

        // No links
        let content = r#"<a href="https:/www.google.com">Google</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 0);

        // Multiple protocols http, https, ftp
        let content = r#"<a href="https://www.google.com">Google</a><a href="http://www.youtube.com">Youtube</a><a href="ftp://www.rust-lang.org">Rust</a>"#;
        let links = get_links(content);
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].to_string(), "https://www.google.com");
        assert_eq!(links[1].to_string(), "http://www.youtube.com");
        assert_eq!(links[2].to_string(), "ftp://www.rust-lang.org");
    }
}
