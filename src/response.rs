use std::convert::TryFrom;
use std::{borrow::Cow, convert::Infallible};

use bytes::Bytes;
use futures::Stream;

use hyper::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};

pub type Response = http::Response<hyper::Body>;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Response {
        LieResponse::default().into()
    }
}

#[derive(Default)]
pub struct LieResponse {
    pub(crate) inner: Response,
}

impl LieResponse {
    pub fn new(status: StatusCode, body: impl Into<hyper::Body>) -> Self {
        LieResponse {
            inner: http::Response::builder()
                .status(status)
                .body(body.into())
                .unwrap(),
        }
    }

    pub fn with_status(status: StatusCode) -> Self {
        let resp = Self::default();
        resp.set_status(status)
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

    pub fn with_string(s: impl ToString) -> Self {
        s.to_string().into()
    }

    pub fn with_stream<S, B, E>(s: S, content_type: mime::Mime) -> Self
    where
        S: Stream<Item = Result<B, E>> + Send + 'static,
        B: Into<Bytes> + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        WithStream::new(s, content_type).into()
    }

    pub async fn send_file(path: impl AsRef<std::path::Path>) -> Result<Self, crate::Error> {
        match tokio::fs::File::open(path.as_ref()).await {
            Ok(file) => {
                let s =
                    tokio_util::codec::FramedRead::new(file, tokio_util::codec::BytesCodec::new());

                let resp = LieResponse::with_stream(
                    s,
                    mime_guess::from_path(path).first_or_octet_stream(),
                );

                Ok(resp)
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(LieResponse::with_status(StatusCode::NOT_FOUND))
                } else {
                    Err(err.into())
                }
            }
        }
    }

    pub fn inner(&self) -> &Response {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut Response {
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

    pub fn into_hyper_response(self) -> Response {
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

    pub fn append_header<K, V>(mut self, name: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        match crate::utils::parse_header(name, value) {
            Ok((name, value)) => {
                self.inner.headers_mut().append(name, value);
            }
            Err(e) => {
                tracing::error!("with_header error: {}", e);
            }
        }

        self
    }

    pub fn append_cookie(self, cookie: crate::Cookie) -> Self {
        self.append_header(http::header::SET_COOKIE, cookie.to_string())
    }

    pub async fn body_bytes(&mut self) -> Result<Vec<u8>, crate::Error> {
        use bytes::Buf;
        use bytes::BytesMut;
        use hyper::body::HttpBody;

        let mut bufs = BytesMut::new();

        while let Some(buf) = self.inner.body_mut().data().await {
            let buf = buf?;
            if buf.has_remaining() {
                bufs.extend(buf);
            }
        }

        Ok(bufs.freeze().to_vec())
    }
}

impl From<Response> for LieResponse {
    fn from(response: Response) -> Self {
        LieResponse { inner: response }
    }
}

impl From<LieResponse> for Response {
    fn from(resp: LieResponse) -> Self {
        resp.inner
    }
}

impl IntoResponse for LieResponse {
    fn into_response(self) -> Response {
        self.inner
    }
}

impl From<StatusCode> for LieResponse {
    fn from(val: StatusCode) -> Self {
        Self::with_status(val)
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(self)
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(format!("{}", self)))
            .unwrap()
    }
}

impl From<&'static [u8]> for LieResponse {
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

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::APPLICATION_OCTET_STREAM.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
    }
}

impl From<Vec<u8>> for LieResponse {
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

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::APPLICATION_OCTET_STREAM.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
    }
}

impl From<&'static str> for LieResponse {
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

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
    }
}

impl From<String> for LieResponse {
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

impl IntoResponse for String {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(self))
            .unwrap()
    }
}

impl From<Cow<'static, str>> for LieResponse {
    fn from(val: Cow<'static, str>) -> Self {
        match val {
            Cow::Borrowed(s) => s.into(),
            Cow::Owned(s) => s.into(),
        }
    }
}

impl IntoResponse for Cow<'static, str> {
    fn into_response(self) -> Response {
        match self {
            Cow::Borrowed(s) => s.into_response(),
            Cow::Owned(s) => s.into_response(),
        }
    }
}

impl IntoResponse for (StatusCode, &'static str) {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(self.0)
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_PLAIN_UTF_8.to_string(),
            )
            .body(hyper::Body::from(self.1))
            .unwrap()
    }
}

impl<E, R> From<Result<R, E>> for LieResponse
where
    R: Into<LieResponse>,
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

impl<E, R> IntoResponse for Result<R, E>
where
    R: Into<LieResponse>,
    E: std::error::Error + 'static + Send + Sync,
{
    fn into_response(self) -> Response {
        match self {
            Ok(r) => r.into().into_response(),
            Err(e) => {
                tracing::error!("on Result<R, E>, error: {:?}", e);

                http::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(hyper::Body::from("Internal Server Error"))
                    .unwrap()
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

impl<T> From<Html<T>> for LieResponse
where
    hyper::Body: From<T>,
    T: Send,
{
    fn from(val: Html<T>) -> LieResponse {
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

impl From<Json> for LieResponse {
    fn from(val: Json) -> LieResponse {
        let resp: Result<LieResponse, _> = val
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

pub struct WithStream<S> {
    s: S,
    content_type: mime::Mime,
}

impl<S, B, E> WithStream<S>
where
    S: Stream<Item = Result<B, E>> + Send + 'static,
    B: Into<Bytes> + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    pub fn new(s: S, content_type: mime::Mime) -> Self {
        WithStream { s, content_type }
    }
}

impl<S, B, E> From<WithStream<S>> for LieResponse
where
    S: Stream<Item = Result<B, E>> + Send + 'static,
    B: Into<Bytes> + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(val: WithStream<S>) -> LieResponse {
        let WithStream { s, content_type } = val;

        http::Response::builder()
            .header(hyper::header::CONTENT_TYPE, content_type.to_string())
            .body(hyper::Body::wrap_stream(s))
            .unwrap()
            .into()
    }
}
