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
        match crate::utils::parse_header(name, value) {
            Ok((name, value)) => {
                self.headers.insert(name, value);
            }
            Err(e) => {
                tracing::error!("DefaultHeaders.header error: {}", e);
            }
        }
    }

    async fn append_header<'a>(&'a self, ctx: Request, next: Next<'a>) -> Response {
        let mut resp: Response = next.run(ctx).await;

        let headers = resp.inner_mut().headers_mut();
        for (k, v) in &self.headers {
            headers.append(k, v.clone());
        }

        resp
    }
}

impl Middleware for DefaultHeaders {
    fn handle<'a>(&'a self, ctx: Request, next: Next<'a>) -> BoxFuture<'a, Response> {
        Box::pin(async move { self.append_header(ctx, next).await })
    }
}
