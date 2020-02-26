use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{Buf, Bytes, BytesMut};
use hyper::body::HttpBody;
use hyper::http::{
    self,
    header::{HeaderMap, HeaderValue},
};
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

    pub fn body_bytes(&self) -> Option<&Bytes> {
        self.body.as_ref()
    }

    pub async fn read_body(&mut self) -> Result<&Bytes, Error> {
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
        let body = self.read_body().await?;
        let json = serde_json::from_slice(body)?;
        Ok(json)
    }

    pub(crate) fn route_path(&mut self) -> &str {
        &self.uri().path()[self.route_prefix.len()..]
    }

    pub(crate) fn append_route_prefix(&mut self, prefix: &str) {
        self.route_prefix += prefix;
    }
}
