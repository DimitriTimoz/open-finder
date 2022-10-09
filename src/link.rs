use std::fmt::Debug;

use self::errors::UrlError;

pub(crate) struct Url {
    url: String,
    host: String,
}

impl Url {
    fn from (v: &dyn ToString) -> Result<Self, UrlError> {
        use UrlError::*;

        let url = v.to_string();
        let mut url_split = url.split("://"); 
        match url_split.next() {
            Some(v) => {
                if v.is_empty(){
                    return Err(NoProtocol);
                }
            },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url() {
        let url = Url::from(&"https://www.google.com").unwrap();
    
        assert_eq!(url.url, "https://www.google.com");
        assert_eq!(url.host, "www.google.com");

        let url = Url::from(&"https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(url.host, "www.youtube.com");
        assert_eq!(url.get_protocol(), "https");

        assert!(Url::from(&"www.google.com").is_err());
        assert!(Url::from(&"http:/www.google.com").is_err());
        assert!(Url::from(&"http://www.rust-lang.org/").is_ok());
    

        assert!(Url::from(&"://www.rust-lang.org/").is_err());

    }
}