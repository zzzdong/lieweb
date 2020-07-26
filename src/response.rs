use std::borrow::Cow;
use std::convert::TryFrom;

use hyper::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};

pub type HyperResponse = http::Response<hyper::Body>;

pub trait IntoResponse: Send + Sized {
    /// Convert the value into a `Response`.
    fn into_response(self) -> Response;
}

pub struct Response {
    pub(crate) inner: HyperResponse,
}

impl Response {
    pub fn new() -> Self {
        Self::with_status(StatusCode::OK)
    }

    pub fn with_status(status: StatusCode) -> Self {
        status.into_response()
    }

    pub fn with_html<T>(body: T) -> Self
    where
        hyper::Body: From<T>,
        T: Send,
    {
        html(body).into_response()
    }

    pub fn with_json<T>(val: &T) -> Self
    where
        T: serde::Serialize,
    {
        json(val).into_response()
    }

    pub fn with_bytes(val: &'static [u8]) -> Self {
        val.into_response()
    }

    pub fn with_bytes_vec(val: Vec<u8>) -> Self {
        val.into_response()
    }

    pub fn with_str(s: &'static str) -> Self {
        s.into_response()
    }

    pub fn with_string(s: String) -> Self {
        s.into_response()
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

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
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

impl From<&'static [u8]> for Response {
    fn from(val: &'static [u8]) -> Self {
        val.into_response()
    }
}

impl From<Vec<u8>> for Response {
    fn from(val: Vec<u8>) -> Self {
        val.into_response()
    }
}

impl From<&'static str> for Response {
    fn from(val: &'static str) -> Self {
        val.into_response()
    }
}

impl From<String> for Response {
    fn from(val: String) -> Self {
        val.into_response()
    }
}

impl From<Cow<'static, str>> for Response {
    fn from(val: Cow<'static, str>) -> Self {
        match val {
            Cow::Borrowed(s) => s.into_response(),
            Cow::Owned(s) => s.into_response(),
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

impl<T> IntoResponse for Html<T>
where
    hyper::Body: From<T>,
    T: Send,
{
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_HTML_UTF_8.to_string(),
            )
            .body(hyper::Body::from(self.body))
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

impl IntoResponse for Json {
    fn into_response(self) -> Response {
        let resp: Result<Response, _> = self
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

        resp.into_response()
    }
}

pub struct WithStatus<T> {
    response: T,
    status: StatusCode,
}

pub fn with_status<T: IntoResponse>(response: T, status: StatusCode) -> WithStatus<T> {
    WithStatus { response, status }
}

impl<T> IntoResponse for WithStatus<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        let mut resp = self.response.into_response();
        *resp.inner.status_mut() = self.status;
        resp
    }
}

pub struct WithHeader<T> {
    header: Option<(HeaderName, HeaderValue)>,
    response: T,
}

pub fn with_header<T, K, V>(response: T, name: K, value: V) -> WithHeader<T>
where
    T: IntoResponse,
    HeaderName: TryFrom<K>,
    <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
    HeaderValue: TryFrom<V>,
    <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
{
    let header = match crate::utils::parse_header(name, value) {
        Ok(h) => Some(h),
        Err(e) => {
            tracing::error!("with_header error: {}", e);
            None
        }
    };

    WithHeader { header, response }
}

impl<T> IntoResponse for WithHeader<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        let mut resp = self.response.into_response();
        if let Some((name, value)) = self.header {
            resp.inner.headers_mut().insert(name, value);
        }
        resp
    }
}

impl<E, R> IntoResponse for Result<R, E>
where
    R: IntoResponse,
    E: std::error::Error + 'static + Send + Sync,
{
    fn into_response(self) -> Response {
        match self {
            Ok(r) => r.into_response(),
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

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(self)
            .body(hyper::Body::empty())
            .unwrap()
            .into()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
            .into()
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
            .into()
    }
}

impl IntoResponse for Cow<'static, str> {
    #[inline]
    fn into_response(self) -> Response {
        match self {
            Cow::Borrowed(s) => s.into_response(),
            Cow::Owned(s) => s.into_response(),
        }
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::APPLICATION_OCTET_STREAM.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
            .into()
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::APPLICATION_OCTET_STREAM.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
            .into()
    }
}
