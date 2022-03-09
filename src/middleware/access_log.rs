use crate::{
    middleware::{Middleware, Next},
    request::RequestCtx,
    Request, Response,
};

/// A simple requests logger
#[derive(Debug, Default)]
pub struct AccessLog;

impl AccessLog {
    pub fn new() -> Self {
        Self::default()
    }

    async fn log_basic<'a>(&'a self, ctx: Request, next: Next<'a>) -> Response {
        let path = ctx.uri().path().to_owned();
        let method = ctx.method().as_str().to_owned();
        let remote_addr = RequestCtx::extract_remote_addr(&ctx)
            .map(|a| a.to_string())
            .unwrap_or_default();

        let start = std::time::Instant::now();
        let res = next.run(ctx).await;
        let status = res.status().as_u16();
        let cost = start.elapsed().as_millis() as f64 / 1000.0;
        tracing::info!(
            %remote_addr,
            %method,
            %path,
            %status,
            %cost,
        );
        res
    }
}

#[crate::async_trait]
impl Middleware for AccessLog {
    async fn handle<'a>(&'a self, ctx: Request, next: Next<'a>) -> Response {
        self.log_basic(ctx, next).await
    }
}
