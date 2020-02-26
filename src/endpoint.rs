use std::future::Future;
use std::sync::Arc;

use futures::future::BoxFuture;

use crate::router::Router;
use crate::{IntoResponse, Request, Response};

pub(crate) type DynEndpoint<State> = dyn Endpoint<State>;

pub trait Endpoint<State>: Send + Sync + 'static {
    /// Invoke the endpoint within the given context
    fn call<'a>(&'a self, req: Request<State>) -> BoxFuture<'a, Response>;
}

impl<State, F: Send + Sync + 'static, Fut> Endpoint<State> for F
where
    F: Fn(Request<State>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    fn call<'a>(&'a self, req: Request<State>) -> BoxFuture<'a, Response> {
        let fut = (self)(req);
        Box::pin(async move { fut.await.into_response() })
    }
}

pub(crate) struct RouterEndpoint<State> {
    router: Arc<Router<State>>,
}

impl<State> RouterEndpoint<State> {
    pub(crate) fn new(router: Arc<Router<State>>) -> RouterEndpoint<State> {
        RouterEndpoint { router }
    }
}

impl<State: Send + Sync + 'static> Endpoint<State> for RouterEndpoint<State> {
    fn call<'a>(&'a self, req: Request<State>) -> BoxFuture<'a, Response> {
        let fut = self.router.route(req);
        Box::pin(async move { fut.await.into_response() })
    }
}

impl<State: Send + Sync + 'static> std::fmt::Debug for RouterEndpoint<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RouterEndpoint{{ router: {:?} }}", self.router)
    }
}
