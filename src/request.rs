use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{Buf, Bytes, BytesMut};
use hyper::body::HttpBody;
use hyper::http::{
    self,
    header::{self, HeaderMap, HeaderName, HeaderValue},
};
use mime::Mime;
use multipart::server::Multipart;
use route_recognizer::Params;
use serde::de::DeserializeOwned;

use crate::error::Error;

pub(crate) type HyperRequest = hyper::Request<hyper::Body>;

#[derive(Debug)]
pub struct Request<State> {
    pub(crate) inner: HyperRequest,
    pub(crate) params: Params,
    state: Arc<State>,
    remote_addr: Option<SocketAddr>,
    body: Option<Bytes>,
    route_prefix: String,
}

impl<State> Request<State> {
    pub fn new(request: HyperRequest, state: Arc<State>, remote_addr: Option<SocketAddr>) -> Self {
        Request {
            inner: request,
            params: Params::new(),
            state,
            remote_addr,
            body: None,
            route_prefix: String::new(),
        }
    }

    pub fn inner(&self) -> &HyperRequest {
        &self.inner
    }

    pub fn innner_mut(&mut self) -> &mut HyperRequest {
        &mut self.inner
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.inner.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        self.inner.headers_mut()
    }

    pub fn method(&self) -> &http::Method {
        self.inner.method()
    }

    pub fn uri(&self) -> &http::Uri {
        self.inner.uri()
    }

    pub fn path(&self) -> &str {
        self.uri().path()
    }

    pub fn version(&self) -> http::Version {
        self.inner.version()
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub(crate) fn merge_params(&mut self, other: &Params) -> &Params {
        for (k, v) in other {
            self.params.insert(k.to_string(), v.to_string());
        }

        &self.params
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    pub async fn body_bytes(&mut self) -> Result<&Bytes, Error> {
        match self.body {
            Some(ref body) => Ok(body),
            None => {
                let mut bufs = BytesMut::new();
                while let Some(buf) = self.inner.body_mut().data().await {
                    let buf = buf?;
                    if buf.has_remaining() {
                        bufs.extend(buf);
                    }
                }

                self.body = Some(bufs.freeze());

                Ok(self.body.as_ref().unwrap())
            }
        }
    }

    pub async fn read_json<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body = self.body_bytes().await?;
        let json = serde_json::from_slice(body)?;

        Ok(json)
    }

    pub async fn read_form<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body = self.body_bytes().await?;
        let form = serde_urlencoded::from_bytes(body)?;

        Ok(form)
    }

    pub async fn read_multipartform(&mut self) -> Result<Multipart<Cursor<Bytes>>, Error> {
        let content_type = self.get_header(header::CONTENT_TYPE)?;
        let mime: Mime = content_type
            .to_str()
            .unwrap()
            .parse()
            .map_err(|e| crate::error_msg!("parse mime failed: {}", e))?;
        let boundary = mime
            .get_param("boundary")
            .map(|v| v.to_string())
            .ok_or_else(|| crate::error_msg!("read_form, boundary not found"))?;

        let m = Multipart::with_body(Cursor::new(self.body_bytes().await?.clone()), boundary);

        Ok(m)
    }

    pub fn get_header<K>(&mut self, header: K) -> Result<&HeaderValue, Error>
    where
        HeaderName: From<K>,
    {
        let key: HeaderName = header.into();
        let value = self
            .headers()
            .get(key)
            .ok_or_else(|| crate::error_msg!("Header not found"))?;

        Ok(value)
    }

    pub fn get_query<T: DeserializeOwned + Default>(&self) -> Result<T, Error> {
        match self.uri().query() {
            Some(query) => serde_urlencoded::from_str(query).map_err(Error::from),
            None => Ok(Default::default()),
        }
    }

    pub fn get_param<T>(&self, param: &str) -> Result<T, Error>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error,
    {
        match self.params.find(param) {
            Some(param) => param
                .parse()
                .map_err(|e| crate::error_msg!("parse param error: {:?}", e)),
            None => Err(crate::error_msg!("param {} not found", param)),
        }
    }

    pub(crate) fn route_path(&mut self) -> &str {
        &self.path()[self.route_prefix.len()..]
    }

    pub(crate) fn append_route_prefix(&mut self, prefix: &str) {
        self.route_prefix += prefix;
    }
}
