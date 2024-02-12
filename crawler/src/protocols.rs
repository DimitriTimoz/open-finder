use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum UriScheme {
    Http,
    Https,
    Ftp,
    Sftp,
    Ssh,
    Chrome,
    File,
    Facetime,
    Git,
    Unknown(String),
}

impl From<String> for UriScheme {
    fn from(scheme: String) -> Self {
        match scheme.to_lowercase().as_str() {
            "http" => UriScheme::Http,
            "https" => UriScheme::Https,
            "ftp" => UriScheme::Ftp,
            "sftp" => UriScheme::Sftp,
            "ssh" => UriScheme::Ssh,
            "chrome" => UriScheme::Chrome,
            "file" => UriScheme::File,
            "facetime" => UriScheme::Facetime,
            "git" => UriScheme::Git,
            _ => UriScheme::Unknown(scheme),
        }
    }
}

impl Display for UriScheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UriScheme::Http => write!(f, "http"),
            UriScheme::Https => write!(f, "https"),
            UriScheme::Ftp => write!(f, "ftp"),
            UriScheme::Sftp => write!(f, "sftp"),
            UriScheme::Ssh => write!(f, "ssh"),
            UriScheme::Chrome => write!(f, "chrome"),
            UriScheme::File => write!(f, "file"),
            UriScheme::Facetime => write!(f, "facetime"),
            UriScheme::Git => write!(f, "git"),
            UriScheme::Unknown(scheme) => write!(f, "{}", scheme),
        }
    }
}
