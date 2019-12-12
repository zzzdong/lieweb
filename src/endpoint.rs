use std::future::Future;

use crate::utils::BoxFuture;
use crate::{IntoResponse, Request, Response};

pub(crate) type DynEndpoint<State, E> =
    dyn (Fn(Request<State>) -> BoxFuture<Response, E>) + 'static + Send + Sync;

pub trait Endpoint<State, E>: Send + Sync + 'static
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// The async result of `call`.
    type Fut: Future<Output = Result<Response, E>> + Send + 'static;

    /// Invoke the endpoint within the given context
    fn call(&self, cx: Request<State>) -> Self::Fut;
}

impl<State, E: Send + Sync + 'static, F: Send + Sync + 'static, Fut> Endpoint<State, E> for F
where
    E: std::error::Error,
    F: Fn(Request<State>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse<E>,
{
    type Fut = BoxFuture<Response, E>;
    fn call(&self, cx: Request<State>) -> Self::Fut {
        let fut = (self)(cx);
        Box::pin(async move { fut.await.into_response() })
    }
}
