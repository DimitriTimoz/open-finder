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
        if let Some(host) = url.split("://").nth(1) {
            let host = host.split('/').next().unwrap().to_string();
            Ok(Url {
                url: url.to_string(),
                host 
            })
        } else {
            Err(NotValidUrl)
        }
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

        assert!(Url::from(&"www.google.com").is_err());
        assert!(Url::from(&"http:/www.google.com").is_err());
        assert!(Url::from(&"http://www.rust-lang.org/").is_ok());

    }
}