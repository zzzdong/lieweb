use crate::{
    middleware::{Middleware, Next},
    Request, Response,
};

const RANDOM_STRING_LEN: usize = 6;

#[derive(Debug, Clone, Default)]
pub struct RequestId;

#[crate::async_trait]
impl Middleware for RequestId {
    async fn handle<'a>(&'a self, mut ctx: Request, next: Next<'a>) -> Response {
        let val = RequestIdValue::new(crate::utils::gen_random_string(RANDOM_STRING_LEN));
        ctx.insert_extension(val);

        next.run(ctx).await
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
    pub fn get_request_id(&self) -> Option<&str> {
        let val = self.get_extension::<RequestIdValue>();
        val.map(|v| v.value.as_str())
    }
}
