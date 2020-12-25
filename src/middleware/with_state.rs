use std::sync::Arc;

use crate::{
    middleware::{Middleware, Next},
    Request, Response,
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

    async fn append_extension<'a>(&'a self, mut ctx: Request, next: Next<'a>) -> Response {
        ctx.insert_extension(self.extension.clone());
        next.run(ctx).await
    }

    pub(crate) fn get_state(ctx: &Request) -> Option<&T> {
        ctx.get_extension::<AppState<T>>().map(|o| o.inner.as_ref())
    }
}

#[crate::async_trait]
impl<T: Send + Sync + 'static + Clone> Middleware for WithState<T> {
    async fn handle<'a>(&'a self, ctx: Request, next: Next<'a>) -> Response {
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
