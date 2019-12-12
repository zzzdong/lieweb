use std::future::Future;
use std::pin::Pin;

pub(crate) type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;
