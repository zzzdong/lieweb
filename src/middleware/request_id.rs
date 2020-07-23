use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{
    middleware::{Middleware, Next},
    Request, Response,
};

use futures::future::BoxFuture;

const RANDOM_STRING_LEN: usize = 6;

#[derive(Debug, Clone)]
pub struct RequestId {
    count: Arc<AtomicU64>,
}

impl RequestId {
    pub fn new() -> Self {
        let count = Arc::new(AtomicU64::new(0));

        RequestId { count }
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for RequestId {
    fn handle<'a>(&'a self, mut ctx: Request, next: Next<'a>) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let id = self.count.fetch_add(1, Ordering::SeqCst);

            let value = format!("{}-{}", crate::utils::gen_random_string(RANDOM_STRING_LEN), id);
            let val = RequestIdValue::new(value);
            ctx.insert_extension(val);

            next.run(ctx).await
        })
    }
}

#[derive(Debug, Clone, Default)]
struct RequestIdValue {
    value: String,
}

impl RequestIdValue {
    fn new(value: String) -> Self {
        RequestIdValue { value }
    }
}

impl Request {
    pub fn get_request_id(&self) -> &str {
        let val = self.get_extension::<RequestIdValue>();
        val.map(|v| v.value.as_str()).unwrap_or_default()
    }
}
