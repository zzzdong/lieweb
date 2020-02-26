use std::convert::TryFrom;

use crate::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use crate::{
    middleware::{Middleware, Next},
    Request, Response,
};

use futures::future::BoxFuture;

#[derive(Debug, Clone, Default)]
pub struct DefaultHeaders {
    headers: HeaderMap,
}

impl DefaultHeaders {
    pub fn new() -> DefaultHeaders {
        DefaultHeaders {
            headers: HeaderMap::new(),
        }
    }

    pub fn header<K, V>(&mut self, name: K, value: V)
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        match <HeaderName as TryFrom<K>>::try_from(name) {
            Ok(name) => match <HeaderValue as TryFrom<V>>::try_from(value) {
                Ok(value) => {
                    self.headers.insert(name, value);
                }
                Err(err) => {
                    log::error!("DefaultHeaders.header(), value error: {}", err.into());
                }
            },
            Err(err) => {
                log::error!("DefaultHeaders.header(),  name error: {}", err.into());
            }
        };
    }

    async fn append_header<'a, State: Send + Sync + 'static>(
        &'a self,
        ctx: Request<State>,
        next: Next<'a, State>,
    ) -> Response {
        let mut resp: Response = next.run(ctx).await;

        let headers = resp.headers_mut();
        for (k, v) in &self.headers {
            headers.append(k, v.clone());
        }

        resp
    }
}

impl<State: Send + Sync + 'static> Middleware<State> for DefaultHeaders {
    fn handle<'a>(&'a self, ctx: Request<State>, next: Next<'a, State>) -> BoxFuture<'a, Response> {
        Box::pin(async move { self.append_header(ctx, next).await })
    }
}
