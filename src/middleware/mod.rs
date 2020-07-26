// import mod
mod default_headers;
mod logger;
mod request_id;
mod with_state;

pub use default_headers::DefaultHeaders;
pub use logger::RequestLogger;
pub use request_id::RequestId;
pub use with_state::WithState;

use std::future::Future;
use std::sync::Arc;

use crate::endpoint::DynEndpoint;
use crate::{Request, Response};

/// Middleware that wraps around remaining middleware chain.
#[async_trait::async_trait]
pub trait Middleware: 'static + Send + Sync {
    /// Asynchronously handle the request, and return a response.
    async fn handle<'a>(&'a self, cx: Request, next: Next<'a>) -> Response;

    /// Set the middleware's name. By default it uses the type signature.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

#[async_trait::async_trait]
impl<F, Fut> Middleware for F
where
    F: Fn(Request, Next<'_>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + Sync + 'static,
{
    async fn handle<'a>(&'a self, cx: Request, next: Next<'a>) -> Response {
        (self)(cx, next).await
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
    pub async fn run(mut self, req: Request) -> Response {
        if let Some((current, next)) = self.next_middleware.split_first() {
            self.next_middleware = next;
            current.handle(req, self).await
        } else {
            (self.endpoint).call(req).await
        }
    }
}
