use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Buf;
use http::header::{HeaderMap, HeaderValue};
use http::request::Parts;

use crate::Error;

pub(crate) type HyperBody = hyper::Body;
pub(crate) type HyperRequest = hyper::Request<hyper::Body>;

#[derive(Debug)]
pub struct Request<State> {
    pub(crate) parts: Parts,
    pub(crate) body: HyperBody,
    state: Arc<State>,
    remote_addr: Option<SocketAddr>,
}

impl<State> Request<State> {
    pub fn new(request: HyperRequest, state: Arc<State>, remote_addr: Option<SocketAddr>) -> Self {
        let (parts, body) = request.into_parts();
        Request {
            parts,
            body,
            state,
            remote_addr,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.parts.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.parts.headers
    }

    pub async fn take_body_bytes(&mut self) -> Result<impl Buf, Error> {
        let empty = HyperBody::empty();
        let body = std::mem::replace(&mut self.body, empty);
        hyper::body::aggregate(body).await.map_err(Error::from)
    }
}
