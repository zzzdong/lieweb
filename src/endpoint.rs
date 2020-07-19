use std::future::Future;
use std::sync::Arc;

use futures::future::BoxFuture;

use crate::router::Router;
use crate::{IntoResponse, Request, Response};

pub(crate) type DynEndpoint = dyn Endpoint;

pub trait Endpoint: Send + Sync + 'static {
    /// Invoke the endpoint within the given context
    fn call(&self, req: Request) -> BoxFuture<'_, Response>;
}

impl<F: Send + Sync + 'static, Fut> Endpoint for F
where
    F: Fn(Request) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    fn call(&self, req: Request) -> BoxFuture<'_, Response> {
        let fut = (self)(req);
        Box::pin(async move { fut.await.into_response() })
    }
}

pub(crate) struct RouterEndpoint {
    router: Arc<Router>,
}

impl RouterEndpoint {
    pub(crate) fn new(router: Arc<Router>) -> RouterEndpoint {
        RouterEndpoint { router }
    }
}

impl Endpoint for RouterEndpoint {
    fn call(&self, req: Request) -> BoxFuture<'_, Response> {
        let fut = self.router.route(req);
        Box::pin(async move { fut.await.into_response() })
    }
}

impl std::fmt::Debug for RouterEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RouterEndpoint{{ router: {:?} }}", self.router)
    }
}
