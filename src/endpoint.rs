use std::future::Future;

use futures::future::BoxFuture;

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
