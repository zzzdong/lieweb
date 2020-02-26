#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("http error")]
    HttpError(#[from] hyper::http::Error),
    #[error("serde_json error")]
    JsonError(#[from] serde_json::Error),
    #[error("decode query string error")]
    QueryError(#[from] serde_urlencoded::de::Error),
    #[error("lieweb error")]
    Message(String),
}

impl<'a> From<&'a str> for Error {
    fn from(s: &'a str) -> Self {
        Error::Message(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Message(s)
    }
}

#[macro_export]
macro_rules! error_msg {
    ($msg:literal) => {
        $crate::Error::Message($msg.to_string())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error::Message(format!($fmt, $($arg)*))
    };
}
