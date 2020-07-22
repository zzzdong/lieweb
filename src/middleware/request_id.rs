use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{
    middleware::{Middleware, Next},
    Request, Response,
};

use futures::future::BoxFuture;

#[derive(Debug, Clone)]
pub struct RequestId {
    count: Arc<AtomicU64>,
    prefix: String,
}

impl RequestId {
    pub fn new() -> Self {
        let count = Arc::new(AtomicU64::new(0));
        let bs = rand::random::<[u8; 4]>();
        let ss: Vec<String> = bs.iter().map(|b| format!("{:2X}", b)).collect();
        let prefix = ss.join("");

        RequestId { count, prefix }
    }

    pub fn with_prefix(prefix: Option<impl AsRef<str>>) -> Self {
        let count = Arc::new(AtomicU64::new(0));
        let prefix = match prefix {
            Some(s) => s.as_ref().to_string(),
            None => {
                let bs = rand::random::<[u8; 8]>();
                let ss: Vec<String> = bs.iter().map(|b| format!("{:2X}", b)).collect();
                ss.join("")
            }
        };

        RequestId { count, prefix }
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

            let value = format!("{}_{}", self.prefix, id);
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
    pub fn get_request_id(&self) -> String {
        let val = self.get_extension::<RequestIdValue>();
        val.map(|v| v.value.clone()).unwrap_or_default()
    }
}
