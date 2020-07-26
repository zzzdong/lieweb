use std::borrow::Cow;
use std::convert::TryFrom;

use hyper::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};

pub type HyperResponse = http::Response<hyper::Body>;

pub struct Response {
    pub(crate) inner: HyperResponse,
}

impl Response {
    pub fn new() -> Self {
        Self::with_status(StatusCode::OK)
    }

    pub fn with_status(status: StatusCode) -> Self {
        status.into()
    }

    pub fn with_html<T>(body: T) -> Self
    where
        hyper::Body: From<T>,
        T: Send,
    {
        html(body).into()
    }

    pub fn with_json<T>(val: &T) -> Self
    where
        T: serde::Serialize,
    {
        json(val).into()
    }

    pub fn with_bytes(val: &'static [u8]) -> Self {
        val.into()
    }

    pub fn with_bytes_vec(val: Vec<u8>) -> Self {
        val.into()
    }

    pub fn with_str(s: &'static str) -> Self {
        s.into()
    }

    pub fn with_string(s: String) -> Self {
        s.into()
    }

    pub fn inner(&self) -> &HyperResponse {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut HyperResponse {
        &mut self.inner
    }

    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.inner.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        self.inner.headers_mut()
    }

    pub fn into_hyper_response(self) -> HyperResponse {
        let Self { inner } = self;
        inner
    }

    pub fn set_status(mut self, status: StatusCode) -> Self {
        *self.inner.status_mut() = status;
        self
    }

    pub fn insert_header<K, V>(mut self, name: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        match crate::utils::parse_header(name, value) {
            Ok((name, value)) => {
                self.inner.headers_mut().insert(name, value);
            }
            Err(e) => {
                tracing::error!("with_header error: {}", e);
            }
        }

        self
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HyperResponse> for Response {
    fn from(response: HyperResponse) -> Self {
        Response { inner: response }
    }
}

impl Into<HyperResponse> for Response {
    fn into(self) -> HyperResponse {
        let Response { inner } = self;
        inner
    }
}

impl From<StatusCode> for Response {
    fn from(val: StatusCode) -> Self {
        Self::with_status(val)
    }
}

impl From<&'static [u8]> for Response {
    fn from(val: &'static [u8]) -> Self {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::APPLICATION_OCTET_STREAM.to_string(),
            )
            .body(hyper::Body::from(val))
            .unwrap()
            .into()
    }
}

impl From<Vec<u8>> for Response {
    fn from(val: Vec<u8>) -> Self {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::APPLICATION_OCTET_STREAM.to_string(),
            )
            .body(hyper::Body::from(val))
            .unwrap()
            .into()
    }
}

impl From<&'static str> for Response {
    fn from(val: &'static str) -> Self {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(val))
            .unwrap()
            .into()
    }
}

impl From<String> for Response {
    fn from(val: String) -> Self {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(val))
            .unwrap()
            .into()
    }
}

impl From<Cow<'static, str>> for Response {
    fn from(val: Cow<'static, str>) -> Self {
        match val {
            Cow::Borrowed(s) => s.into(),
            Cow::Owned(s) => s.into(),
        }
    }
}

impl<E, R> From<Result<R, E>> for Response
where
    R: Into<Response>,
    E: std::error::Error + 'static + Send + Sync,
{
    fn from(val: Result<R, E>) -> Self {
        match val {
            Ok(r) => r.into(),
            Err(e) => {
                tracing::error!("on Result<R, E>, error: {:?}", e);

                http::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(hyper::Body::from("Internal Server Error"))
                    .unwrap()
                    .into()
            }
        }
    }
}

pub struct Html<T> {
    body: T,
}

pub fn html<T>(body: T) -> Html<T>
where
    hyper::Body: From<T>,
    T: Send,
{
    Html { body }
}

impl<T> From<Html<T>> for Response
where
    hyper::Body: From<T>,
    T: Send,
{
    fn from(val: Html<T>) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_HTML_UTF_8.to_string(),
            )
            .body(hyper::Body::from(val.body))
            .unwrap()
            .into()
    }
}

pub struct Json {
    inner: Result<Vec<u8>, serde_json::Error>,
}

pub fn json<T>(val: &T) -> Json
where
    T: serde::Serialize,
{
    Json {
        inner: serde_json::to_vec(val),
    }
}

impl From<Json> for Response {
    fn from(val: Json) -> Response {
        let resp: Result<Response, _> = val
            .inner
            .map(|j| {
                http::Response::builder()
                    .header(
                        hyper::header::CONTENT_TYPE,
                        mime::APPLICATION_JSON.to_string(),
                    )
                    .body(hyper::Body::from(j))
                    .unwrap()
                    .into()
            })
            .map_err(|e| {
                tracing::error!("json serialize failed, {:?}", e);
                e
            });

        resp.into()
    }
}
