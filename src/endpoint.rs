
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::request::{FromRequest, RequestParts};
use crate::response::IntoResponse;
use crate::router::Router;
use crate::{HyperRequest, HyperResponse, Response, Error};

pub(crate) type DynEndpoint = dyn Endpoint;

#[crate::async_trait]
pub trait Endpoint: Send + Sync + 'static {
    /// Invoke the endpoint within the given context
    async fn call(&self, req: HyperRequest) -> HyperResponse;
}

#[crate::async_trait]
impl<F: Send + Sync + 'static, Fut, Res> Endpoint for F
where
    F: Fn(HyperRequest) -> Fut,
    Fut: Future<Output = Res> + Send + 'static,
    Res: Into<Response> + 'static,
{
    async fn call(&self, req: HyperRequest) -> HyperResponse {
        let resp = self(req).await;
        resp.into().into()
    }
}

pub struct IntoEndpoint<H, T> {
    handler: H,
    _marker: PhantomData<fn() -> T>,
}

impl<H, T> IntoEndpoint<H, T> {
    pub fn new(handler: H) -> Self {
        IntoEndpoint {
            handler,
            _marker: PhantomData,
        }
    }
}

#[crate::async_trait]
impl<H, T> Endpoint for IntoEndpoint<H, T>
    where
        H: Handler<T> + Clone + Send + Sync + 'static,
        T: 'static
{
    async fn call(&self, req: HyperRequest) -> HyperResponse {
        let handler = self.handler.clone();

        let resp = Handler::call(handler, req).await;
        resp
    }
}




#[crate::async_trait]
pub trait Handler<T>: Clone + Send + Sized + 'static {
    async fn call(self, req: HyperRequest) -> HyperResponse;

    fn into_endpoint(self) -> IntoEndpoint<Self, T> {
        IntoEndpoint::new(self)
    }
}

#[crate::async_trait]
impl<F, Fut, Res> Handler<()> for F
where
    F: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse + Send + 'static,
{
    async fn call(self, req: HyperRequest) -> HyperResponse {
        self().await.into_response()
    }
}

macro_rules! impl_handler {
    ($($ty: ident),+) => {
        #[crate::async_trait]
        #[allow(non_snake_case)]
        impl<F, Fut, Res, $($ty,)*> Handler<($($ty,)*)> for F
        where
            F: FnOnce($($ty,)*) -> Fut + Clone + Send + 'static,
            Fut: Future<Output = Res> + Send,
            Res: IntoResponse,
            $( $ty: FromRequest + Send,)*
        {
            async fn call(self, req: HyperRequest) -> HyperResponse {
                let mut req = RequestParts::new(req);

                $(
                    let $ty = match $ty::from_request(&mut req).await {
                        Ok(value) => value,
                        Err(rejection) => return rejection.into_response(),
                    };
                )*

                let res = self($($ty,)*).await;

                res.into_response()
            }
        }
    };
}

impl_handler!(T1);
impl_handler!(T1, T2);
impl_handler!(T1, T2, T3);
impl_handler!(T1, T2, T3, T4);
impl_handler!(T1, T2, T3, T4, T5);
impl_handler!(T1, T2, T3, T4, T5, T6);
impl_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

pub(crate) struct RouterEndpoint {
    router: Arc<Router>,
}

impl RouterEndpoint {
    pub(crate) fn new(router: Arc<Router>) -> RouterEndpoint {
        RouterEndpoint { router }
    }
}

#[crate::async_trait]
impl Endpoint for RouterEndpoint {
    async fn call(&self, req: HyperRequest) -> HyperResponse {
        self.router.route(req).await
    }
}

impl std::fmt::Debug for RouterEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RouterEndpoint{{ router: {:?} }}", self.router)
    }
}
