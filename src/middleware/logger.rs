use crate::{
    middleware::{Middleware, Next},
    Request, Response,
};
use futures::future::BoxFuture;

/// A simple requests logger
#[derive(Debug, Clone, Default)]
pub struct RequestLogger;

impl RequestLogger {
    pub fn new() -> Self {
        Self::default()
    }

    async fn log_basic<'a>(&'a self, ctx: Request, next: Next<'a>) -> Response {
        let path = ctx.uri().path().to_owned();
        let method = ctx.method().as_str().to_owned();
        let remote_addr = ctx.remote_addr();
        let request_id = ctx.get_request_id().to_owned();
        tracing::trace!(
            "IN =>{} {} {}, From {:?}",
            request_id,
            method,
            path,
            remote_addr
        );
        let start = std::time::Instant::now();
        let res = next.run(ctx).await;
        let status = res.status();
        tracing::info!(
            request_id=%request_id,
            method=%method,
            path=%path,
            status=%status.as_str(),
            cost=%start.elapsed().as_millis(),
        );
        res
    }
}

impl Middleware for RequestLogger {
    fn handle<'a>(&'a self, ctx: Request, next: Next<'a>) -> BoxFuture<'a, Response> {
        Box::pin(async move { self.log_basic(ctx, next).await })
    }
}
