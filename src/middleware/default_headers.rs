use std::convert::TryFrom;

use crate::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use crate::{
    middleware::{Middleware, Next},
    HyperRequest, HyperResponse,
};

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

    async fn append_header<'a>(&'a self, ctx: HyperRequest, next: Next<'a>) -> HyperResponse {
        let mut resp: HyperResponse = next.run(ctx).await;

        let headers = resp.headers_mut();
        for (k, v) in &self.headers {
            headers.append(k, v.clone());
        }

        resp
    }
}

#[crate::async_trait]
impl Middleware for DefaultHeaders {
    async fn handle<'a>(&'a self, ctx: HyperRequest, next: Next<'a>) -> HyperResponse {
        self.append_header(ctx, next).await
    }
}
