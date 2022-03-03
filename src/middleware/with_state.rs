use std::sync::Arc;

use crate::{
    middleware::{Middleware, Next},
    request::RequestParts,
    Request, Response,
};

#[derive(Debug, Clone)]
pub struct WithState<T: Clone + Send + Sync + 'static> {
    extension: AppState<T>,
}

impl<T: Clone + Send + Sync + 'static> WithState<T> {
    pub fn new(extension: T) -> Self {
        WithState {
            extension: AppState {
                inner: extension.clone(),
            },
        }
    }

    async fn append_extension<'a>(&'a self, mut ctx: Request, next: Next<'a>) -> Response {
        ctx.extensions_mut().insert(self.extension.clone());
        next.run(ctx).await
    }

    pub(crate) fn get_state(ctx: &RequestParts) -> Option<T> {
        ctx.extensions.get::<AppState<T>>().map(|o| o.inner.clone())
    }
}

#[crate::async_trait]
impl<T: Send + Sync + 'static + Clone> Middleware for WithState<T> {
    async fn handle<'a>(&'a self, ctx: Request, next: Next<'a>) -> Response {
        self.append_extension(ctx, next).await
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AppState<T: Clone + Send + Sync + 'static> {
    pub(crate) inner: T,
}
