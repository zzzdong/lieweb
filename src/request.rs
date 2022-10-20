use std::net::SocketAddr;

use bytes::{Buf, Bytes, BytesMut};
use headers::{Header, HeaderMapExt, HeaderName, HeaderValue};
use hyper::body::HttpBody;
use hyper::http;
use pathrouter::Params;
use serde::de::DeserializeOwned;

pub type Request = hyper::Request<hyper::Body>;

use crate::error::{invalid_header, invalid_param, missing_cookie, missing_header, missing_param};
use crate::response::IntoResponse;
use crate::Error;

#[crate::async_trait]
pub trait FromRequest: Sized {
    type Rejection: IntoResponse;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection>;
}

pub type RequestParts = hyper::Request<Option<hyper::Body>>;

#[crate::async_trait]
pub trait LieRequest {
    fn path(&self) -> &str;
    fn remote_addr(&self) -> Option<SocketAddr>;
    fn get_param<T>(&self, param: &str) -> Result<T, Error>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error;
    fn get_cookie(&self, name: &str) -> Result<String, Error>;
    fn get_header<K>(&self, header: K) -> Result<&HeaderValue, Error>
    where
        HeaderName: From<K>;
    fn get_typed_header<T: Header + Send + 'static>(&self) -> Result<T, Error>;

    async fn read_body(&mut self) -> Result<Bytes, Error>;
    async fn read_form<T: DeserializeOwned>(&mut self) -> Result<T, Error>;
    async fn read_json<T: DeserializeOwned>(&mut self) -> Result<T, Error>;
}

#[crate::async_trait]
impl LieRequest for Request {
    fn path(&self) -> &str {
        self.uri().path()
    }

    fn remote_addr(&self) -> Option<SocketAddr> {
        self.extensions()
            .get::<RequestCtx>()
            .and_then(|ctx| ctx.remote_addr)
    }

    fn get_param<T>(&self, param: &str) -> Result<T, Error>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error,
    {
        let ctx = self.extensions().get::<RequestCtx>();

        match ctx {
            Some(ctx) => match ctx.params.find(param) {
                Some(param) => param
                    .parse()
                    .map_err(|e| invalid_param(param, std::any::type_name::<T>(), e)),
                None => Err(missing_param(param)),
            },
            None => Err(missing_param(param)),
        }
    }

    fn get_header<K>(&self, header: K) -> Result<&HeaderValue, Error>
    where
        HeaderName: From<K>,
    {
        let key: HeaderName = header.into();
        let key_cloned = key.clone();
        let value = self
            .headers()
            .get(key)
            .ok_or_else(|| missing_header(key_cloned))?;

        Ok(value)
    }

    fn get_typed_header<T: Header + Send + 'static>(&self) -> Result<T, Error> {
        self.headers()
            .typed_get::<T>()
            .ok_or_else(|| invalid_header(T::name().as_str()))
    }

    fn get_cookie(&self, name: &str) -> Result<String, Error> {
        let cookie: headers::Cookie = self.get_typed_header()?;

        cookie
            .get(name)
            .ok_or_else(|| missing_cookie(name))
            .map(|s| s.to_string())
    }

    async fn read_body(&mut self) -> Result<Bytes, Error> {
        let mut bufs = BytesMut::new();

        while let Some(buf) = self.body_mut().data().await {
            let buf = buf?;
            if buf.has_remaining() {
                bufs.extend(buf);
            }
        }

        Ok(bufs.freeze())
    }

    async fn read_form<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body = self.read_body().await?;
        let form = serde_urlencoded::from_bytes(&body)?;

        Ok(form)
    }

    async fn read_json<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body = self.read_body().await?;
        let json = serde_json::from_slice(&body)?;

        Ok(json)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RequestCtx {
    params: Params,
    remote_addr: Option<SocketAddr>,
    route_path: Option<String>,
}

impl RequestCtx {
    pub(crate) fn init<B>(req: &mut http::Request<B>, remote_addr: Option<SocketAddr>) {
        let ctx = RequestCtx {
            params: Params::new(),
            remote_addr,
            route_path: None,
        };

        req.extensions_mut().insert(ctx);
    }

    pub(crate) fn extract_params<B>(req: &http::Request<B>) -> Option<&Params> {
        req.extensions().get::<Self>().map(|ctx| &ctx.params)
    }

    pub(crate) fn extract_remote_addr<B>(req: &http::Request<B>) -> Option<SocketAddr> {
        req.extensions()
            .get::<RequestCtx>()
            .and_then(|ctx| ctx.remote_addr)
    }

    pub(crate) fn route_path<B>(req: &http::Request<B>) -> &str {
        let ctx = req
            .extensions()
            .get::<Self>()
            .expect("can not extract RequestCtx from request");

        match ctx.route_path {
            Some(ref path) => path,
            None => req.uri().path(),
        }
    }

    pub(crate) fn set_route_path<B>(req: &mut http::Request<B>, path: &str) {
        let ctx = req
            .extensions_mut()
            .get_mut::<Self>()
            .expect("can not extract RequestCtx from request");
        ctx.route_path = Some(path.to_string());
    }

    pub(crate) fn merge_params<B>(req: &mut http::Request<B>, other: &Params) {
        let ctx = req
            .extensions_mut()
            .get_mut::<Self>()
            .expect("can not extract RequestCtx from request");

        for (k, v) in other {
            ctx.params.insert(k.to_string(), v.to_string());
        }
    }
}
