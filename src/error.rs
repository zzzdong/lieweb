#[derive(thiserror::Error, Debug)]
pub enum LieError {
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("http error")]
    HttpError(#[from] http::Error),
    #[error("json error")]
    JsonError(#[from] serde_json::Error),
}
