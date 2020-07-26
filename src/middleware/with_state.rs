use std::sync::Arc;

use crate::{
    middleware::{Middleware, Next},
    Request, Response,
};

#[derive(Debug, Clone, Default)]
pub struct WithState<T: Send + Sync + 'static> {
    extension: Arc<T>,
}

impl<T: Send + Sync + 'static> WithState<T> {
    pub fn new(extension: T) -> Self {
        WithState {
            extension: Arc::new(extension),
        }
    }

    async fn append_extension<'a>(&'a self, mut ctx: Request, next: Next<'a>) -> Response {
        ctx.request.extensions_mut().insert(self.extension.clone());
        next.run(ctx).await
    }
}

#[async_trait::async_trait]
impl<T: Send + Sync + 'static + Clone> Middleware for WithState<T> {
    async fn handle<'a>(&'a self, ctx: Request, next: Next<'a>) -> Response {
        self.append_extension(ctx, next).await
    }
}
