use crate::{
    middleware::{Middleware, Next},
    HyperRequest, HyperResponse,
};

const RANDOM_STRING_LEN: usize = 6;

#[derive(Debug, Clone, Default)]
pub struct RequestId;

impl RequestId {
    pub fn get(req: &HyperRequest) -> Option<&str> {
        let val = req.extensions().get::<RequestIdValue>();
        val.map(|v| v.value.as_str())
    }
}

#[crate::async_trait]
impl Middleware for RequestId {
    async fn handle<'a>(&'a self, mut ctx: HyperRequest, next: Next<'a>) -> HyperResponse {
        let val = RequestIdValue::new(crate::utils::gen_random_string(RANDOM_STRING_LEN));
        ctx.extensions_mut().insert(val);

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
