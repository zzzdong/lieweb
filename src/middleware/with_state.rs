use std::sync::Arc;

use crate::{
    middleware::{Middleware, Next},
    HyperRequest, HyperResponse,
};

#[derive(Debug, Clone)]
pub struct WithState<T: Send + Sync + 'static> {
    extension: AppState<T>,
}

impl<T: Send + Sync + 'static> WithState<T> {
    pub fn new(extension: T) -> Self {
        WithState {
            extension: AppState {
                inner: Arc::new(extension),
            },
        }
    }

    async fn append_extension<'a>(&'a self, mut ctx: HyperRequest, next: Next<'a>) -> HyperResponse {
        ctx.extensions_mut().insert(self.extension.clone());
        next.run(ctx).await
    }

    pub(crate) fn get_state(ctx: &HyperRequest) -> Option<&T> {
        ctx.extensions().get::<AppState<T>>().map(|o| o.inner.as_ref())
    }
}

#[crate::async_trait]
impl<T: Send + Sync + 'static + Clone> Middleware for WithState<T> {
    async fn handle<'a>(&'a self, ctx: HyperRequest, next: Next<'a>) -> HyperResponse {
        self.append_extension(ctx, next).await
    }
}

#[derive(Debug)]
pub(crate) struct AppState<T: Send + Sync + 'static> {
    pub(crate) inner: Arc<T>,
}

impl<T> Clone for AppState<T>
where
    T: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        AppState {
            inner: self.inner.clone(),
        }
    }
}
