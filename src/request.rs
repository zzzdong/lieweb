use std::net::SocketAddr;
use std::sync::Arc;

pub(crate) type HyperRequest = hyper::Request<hyper::Body>;

#[derive(Debug)]
pub struct Request<State> {
    pub(crate) inner: HyperRequest,
    state: Arc<State>,
    remote_addr: Option<SocketAddr>,
}

impl<State> Request<State> {
    pub fn new(inner: HyperRequest, state: Arc<State>, remote_addr: Option<SocketAddr>) -> Self {
        Request {
            inner,
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
}
