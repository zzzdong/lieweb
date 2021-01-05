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
    #[error("invalid request header {name:?}")]
    InvalidHeader {
        name: &'static str,
    },
    #[error("invalid param {name:?} as {expected:?}, {err:?}")]
    InvalidParam {
        name: String,
        expected: &'static str,
        err: String,
    },
    #[error("missing AppState {name:?}")]
    MissingAppState {
        name: &'static str,
    },
    #[error("missing url param {name:?}")]
    MissingParam {
        name: String,
    },
    #[error("missing cookie {name:?}")]
    MissingCookie {
        name: String,
    },
    #[error("missing header {name:?}")]
    MissingHeaader {
        name: String,
    },
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

pub fn invalid_header(name:  &'static str, ) -> Error {
    Error::InvalidHeader {
        name,
    }
}

pub fn invalid_param(name: impl ToString, expected: &'static str, err: impl std::error::Error) -> Error {
    Error::InvalidParam {
        name: name.to_string(),
        expected,
        err: err.to_string(),
    }
}

pub fn missing_appstate(name: &'static str) -> Error {
    Error::MissingAppState {
        name,
    }
}

pub fn missing_cookie(name: impl ToString) -> Error {
    Error::MissingCookie {
        name: name.to_string(),
    }
}

pub fn missing_header(name: impl ToString) -> Error {
    Error::MissingHeaader {
        name: name.to_string(),
    }
}

pub fn missing_param(name: impl ToString) -> Error {
    Error::MissingParam {
        name: name.to_string(),
    }
}