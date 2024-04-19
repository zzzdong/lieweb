use std::{borrow::Cow, convert::Infallible};

use bytes::Bytes;

use futures_util::StreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Frame;
use hyper::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};

use crate::ty::{BytesBody, Form, Html, Json, StreamBody};
use crate::Error;

pub type Response = http::Response<BoxBody<Bytes, Error>>;

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
    pub fn new(status: StatusCode, body: impl Into<Bytes>) -> Self {
        LieResponse {
            inner: http::Response::builder()
                .status(status)
                .body(Full::new(body.into()).map_err(Into::into).boxed())
                .unwrap(),
        }
    }

    pub fn with_status(status: StatusCode) -> Self {
        let resp = Self::default();
        resp.set_status(status)
    }

    pub fn with_html(body: impl Into<Bytes>) -> Self {
        Html::new(body).into()
    }

    pub fn with_json<T>(val: T) -> Self
    where
        T: serde::Serialize,
    {
        Json::new(val).into()
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
        S: futures::Stream<Item = Result<B, E>> + Send + Sync + 'static,
        B: Into<Bytes> + 'static,
        E: Into<Error> + Send + Sync + 'static,
    {
        StreamBody::new(s, content_type).into()
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

    // pub async fn body_bytes(&mut self) -> Result<Vec<u8>, crate::Error> {
    //     use bytes::Buf;
    //     use bytes::BytesMut;
    //     use hyper::body::HttpBody;

    //     let mut bufs = BytesMut::new();

    //     while let Some(buf) = self.inner.body_mut().data().await {
    //         let buf = buf?;
    //         if buf.has_remaining() {
    //             bufs.extend(buf);
    //         }
    //     }

    //     Ok(bufs.freeze().to_vec())
    // }
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
            .body(Empty::new().map_err(Into::into).boxed())
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
            .body(
                Full::new(Bytes::from_static(val))
                    .map_err(Into::into)
                    .boxed(),
            )
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
            .body(
                Full::new(Bytes::from_static(self))
                    .map_err(Into::into)
                    .boxed(),
            )
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
            .body(Full::new(Bytes::from(val)).map_err(Into::into).boxed())
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
            .body(Full::new(Bytes::from(self)).map_err(Into::into).boxed())
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
            .body(
                Full::new(Bytes::from_static(val.as_bytes()))
                    .map_err(Into::into)
                    .boxed(),
            )
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
            .body(
                Full::new(Bytes::from_static(self.as_bytes()))
                    .map_err(Into::into)
                    .boxed(),
            )
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
            .body(Full::new(Bytes::from(val)).map_err(Into::into).boxed())
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
            .body(Full::new(Bytes::from(self)).map_err(Into::into).boxed())
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
            .body(Full::new(Bytes::from(self.1)).map_err(Into::into).boxed())
            .unwrap()
    }
}

impl IntoResponse for crate::Error {
    fn into_response(self) -> Response {
        tracing::error!("on IntoResponse for lieweb::Error, error: {:?}", self);

        http::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(
                Full::new(Bytes::from("Internal Server Error"))
                    .map_err(Into::into)
                    .boxed(),
            )
            .unwrap()
    }
}

impl From<crate::Error> for LieResponse {
    fn from(e: crate::Error) -> Self {
        tracing::error!("on From<lieweb::Error> for LieResponse, error: {:?}", e);

        http::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(
                Full::new(Bytes::from("Internal Server Error"))
                    .map_err(Into::into)
                    .boxed(),
            )
            .unwrap()
            .into()
    }
}

impl<E, R> From<Result<R, E>> for LieResponse
where
    R: Into<LieResponse>,
    E: Into<LieResponse>,
{
    fn from(val: Result<R, E>) -> Self {
        match val {
            Ok(r) => r.into(),
            Err(e) => e.into(),
        }
    }
}

impl<E, R> IntoResponse for Result<R, E>
where
    R: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(r) => r.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

impl<T> From<Form<T>> for LieResponse
where
    T: serde::Serialize,
{
    fn from(form: Form<T>) -> LieResponse {
        serde_urlencoded::to_string(&form.value)
            .map(|b| {
                LieResponse::from(
                    http::Response::builder()
                        .header(
                            hyper::header::CONTENT_TYPE,
                            mime::APPLICATION_WWW_FORM_URLENCODED.to_string(),
                        )
                        .body(Full::new(Bytes::from(b)).map_err(Into::into).boxed())
                        .unwrap(),
                )
            })
            .map_err(|e| {
                tracing::error!("urlencoded form serialize failed, {:?}", e);
                crate::Error::from(e)
            })
            .into()
    }
}

impl From<Html> for LieResponse {
    fn from(val: Html) -> LieResponse {
        http::Response::builder()
            .header(
                hyper::header::CONTENT_TYPE,
                mime::TEXT_HTML_UTF_8.to_string(),
            )
            .body(val.body.map_err(Into::into).boxed())
            .unwrap()
            .into()
    }
}

impl<T> From<Json<T>> for LieResponse
where
    T: serde::Serialize,
{
    fn from(json: Json<T>) -> LieResponse {
        serde_json::to_vec(&json.value)
            .map(|b| {
                LieResponse::from(
                    http::Response::builder()
                        .header(
                            hyper::header::CONTENT_TYPE,
                            mime::APPLICATION_JSON.to_string(),
                        )
                        .body(Full::new(Bytes::from(b)).map_err(Into::into).boxed())
                        .unwrap(),
                )
            })
            .map_err(|e| {
                tracing::error!("json serialize failed, {:?}", e);
                crate::Error::from(e)
            })
            .into()
    }
}

impl From<BytesBody> for LieResponse {
    fn from(body: BytesBody) -> Self {
        let BytesBody { body, content_type } = body;

        http::Response::builder()
            .header(hyper::header::CONTENT_TYPE, content_type.to_string())
            .body(Full::new(body).map_err(Into::into).boxed())
            .unwrap()
            .into()
    }
}

impl<S, B, E> From<StreamBody<S>> for LieResponse
where
    S: futures::Stream<Item = Result<B, E>> + Send + Sync + 'static,
    B: Into<Bytes> + 'static,
    E: Into<Error> + Send + Sync + 'static,
{
    fn from(body: StreamBody<S>) -> LieResponse {
        let StreamBody { s, content_type } = body;

        let body = s.map(|b| b.map(|b| Frame::data(b.into())).map_err(Into::into));

        let resp = http::Response::builder()
            .header(hyper::header::CONTENT_TYPE, content_type.to_string())
            .body(BodyExt::boxed(http_body_util::StreamBody::new(body)))
            .unwrap();

        resp.into()
    }
}
