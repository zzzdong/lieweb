use std::future::Future;

use futures::future::BoxFuture;

use crate::{IntoResponse, Request, Response};

pub(crate) type DynEndpoint<State> =
    dyn (Fn(Request<State>) -> BoxFuture<'static, Response>) + 'static + Send + Sync;

pub trait Endpoint<State>: Send + Sync + 'static {
    /// The async result of `call`.
    type Fut: Future<Output = Response> + 'static + Send;

    /// Invoke the endpoint within the given context
    fn call(&self, req: Request<State>) -> Self::Fut;
}

impl<State, F: Send + Sync + 'static, Fut> Endpoint<State> for F
where
    F: Fn(Request<State>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    type Fut = BoxFuture<'static, Response>;

    fn call(&self, req: Request<State>) -> Self::Fut {
        let fut = (self)(req);
        Box::pin(async move { fut.await.into_response() })
    }
}
