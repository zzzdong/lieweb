use std::net::SocketAddr;
use std::sync::Arc;

use http::header::{HeaderMap, HeaderValue};
use route_recognizer::Params;

pub(crate) type HyperRequest = hyper::Request<hyper::Body>;

#[derive(Debug)]
pub struct Request<State> {
    pub(crate) inner: HyperRequest,
    pub(crate) params: Params,
    state: Arc<State>,
    remote_addr: Option<SocketAddr>,
}

impl<State> Request<State> {
    pub fn new(
        request: HyperRequest,
        params: Params,
        state: Arc<State>,
        remote_addr: Option<SocketAddr>,
    ) -> Self {
        Request {
            inner: request,
            params,
            state,
            remote_addr,
        }
    }

    pub fn request(&self) -> &HyperRequest {
        &self.inner
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

    pub fn version(&self) -> http::Version {
        self.inner.version()
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }
}
