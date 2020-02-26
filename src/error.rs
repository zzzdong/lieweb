#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("http error")]
    HttpError(#[from] hyper::http::Error),
    #[error("json error")]
    JsonError(#[from] serde_json::Error),
    #[error("lieweb error")]
    Message(String),
}
