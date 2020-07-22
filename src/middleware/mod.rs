// import mod
mod default_headers;
mod logger;
mod request_id;
mod with_state;

pub use default_headers::DefaultHeaders;
pub use logger::RequestLogger;
pub use request_id::RequestId;
pub use with_state::WithState;

use std::sync::Arc;

use futures::future::BoxFuture;

use crate::endpoint::DynEndpoint;
use crate::{Request, Response};

/// Middleware that wraps around remaining middleware chain.
pub trait Middleware: 'static + Send + Sync {
    /// Asynchronously handle the request, and return a response.
    fn handle<'a>(&'a self, cx: Request, next: Next<'a>) -> BoxFuture<'a, Response>;

    /// Set the middleware's name. By default it uses the type signature.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

impl<F> Middleware for F
where
    F: Fn(Request, Next<'_>) -> BoxFuture<'_, Response> + Send + Sync + 'static,
{
    fn handle<'a>(&'a self, req: Request, next: Next<'a>) -> BoxFuture<'a, Response> {
        (self)(req, next)
    }
}

/// The remainder of a middleware chain, including the endpoint.
#[allow(missing_debug_implementations)]
pub struct Next<'a> {
    pub(crate) endpoint: &'a DynEndpoint,
    pub(crate) next_middleware: &'a [Arc<dyn Middleware>],
}

impl<'a> Next<'a> {
    /// Asynchronously execute the remaining middleware chain.
    pub fn run(mut self, req: Request) -> BoxFuture<'a, Response> {
        if let Some((current, next)) = self.next_middleware.split_first() {
            self.next_middleware = next;
            current.handle(req, self)
        } else {
            (self.endpoint).call(req)
        }
    }
}
