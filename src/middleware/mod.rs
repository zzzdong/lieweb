// import mod
mod logger;

pub use logger::RequestLogger;

use std::sync::Arc;

use futures::future::BoxFuture;

use crate::endpoint::DynEndpoint;
use crate::{Request, Response};

/// Middleware that wraps around remaining middleware chain.
pub trait Middleware<State>: 'static + Send + Sync {
    /// Asynchronously handle the request, and return a response.
    fn handle<'a>(&'a self, cx: Request<State>, next: Next<'a, State>) -> BoxFuture<'a, Response>;
}

impl<State, F> Middleware<State> for F
where
    F: Send
        + Sync
        + 'static
        + for<'a> Fn(Request<State>, Next<'a, State>) -> BoxFuture<'a, Response>,
{
    fn handle<'a>(&'a self, req: Request<State>, next: Next<'a, State>) -> BoxFuture<'a, Response> {
        (self)(req, next)
    }
}

/// The remainder of a middleware chain, including the endpoint.
#[allow(missing_debug_implementations)]
pub struct Next<'a, State> {
    pub(crate) endpoint: &'a DynEndpoint<State>,
    pub(crate) next_middleware: &'a [Arc<dyn Middleware<State>>],
}

impl<'a, State: 'static> Next<'a, State> {
    /// Asynchronously execute the remaining middleware chain.
    pub fn run(mut self, req: Request<State>) -> BoxFuture<'a, Response> {
        if let Some((current, next)) = self.next_middleware.split_first() {
            self.next_middleware = next;
            current.handle(req, self)
        } else {
            (self.endpoint).call(req)
        }
    }
}
