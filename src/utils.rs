#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;

pub(crate) type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;
pub(crate) type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
