#![allow(dead_code)]

use std::convert::TryFrom;
use std::future::Future;
use std::pin::Pin;

use crate::http;
use crate::http::header::{HeaderName, HeaderValue};

use rand::distributions::Alphanumeric;
use rand::Rng;

pub(crate) type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;
pub(crate) type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub(crate) fn parse_header<K, V>(
    name: K,
    value: V,
) -> Result<(HeaderName, HeaderValue), crate::Error>
where
    HeaderName: TryFrom<K>,
    <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
    HeaderValue: TryFrom<V>,
    <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
{
    let name = <HeaderName as TryFrom<K>>::try_from(name).map_err(|e| {
        let e = e.into();
        crate::error_msg!("parse header name failed, err: {}", e)
    })?;

    let value = <HeaderValue as TryFrom<V>>::try_from(value).map_err(|e| {
        let e = e.into();
        crate::error_msg!("parse header value failed, err: {}", e)
    })?;

    Ok((name, value))
}

pub(crate) fn gen_random_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(length)
        .collect::<String>()
}

#[macro_export]
macro_rules! register_method {
    ($func_name: ident, $method: expr) => {
        pub fn $func_name<H, T>(&mut self, path: impl AsRef<str>, handler: H)
        where
            H: Handler<T> + Send + Sync + 'static,
            T: 'static,
        {
            self.register($method, path, handler)
        }
    };
}
