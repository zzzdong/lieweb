use std::future::Future;

use crate::utils::{BoxFuture, StdError};
use crate::{IntoResponse, Request, Response};

pub(crate) type DynEndpoint<State> =
    dyn (Fn(Request<State>) -> BoxFuture<Response, StdError>) + 'static + Send + Sync;

pub trait Endpoint<State>: Send + Sync + 'static {
    /// The async result of `call`.
    type Fut: Future<Output = Result<Response, StdError>> + Send + 'static;

    /// Invoke the endpoint within the given context
    fn call(&self, cx: Request<State>) -> Self::Fut;
}

impl<State, F: Send + Sync + 'static, Fut> Endpoint<State> for F
where
    F: Fn(Request<State>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    type Fut = BoxFuture<Response, StdError>;

    fn call(&self, cx: Request<State>) -> Self::Fut {
        let fut = (self)(cx);
        Box::pin(async move { fut.await.into_response() })
    }
}
