use std::net::SocketAddr;

use bytes::{Buf, Bytes, BytesMut};
use hyper::body::HttpBody;
use hyper::http::header::{HeaderMap, HeaderValue};
use hyper::http::request::Parts;
use hyper::http::Extensions;
use hyper::{Method, Uri, Version};
use route_recognizer::Params;
use serde::de::DeserializeOwned;

pub type Request = hyper::Request<hyper::Body>;

use crate::response::IntoResponse;
use crate::Error;

#[crate::async_trait]
pub trait FromRequest: Sized {
    type Rejection: IntoResponse;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection>;
}

pub struct RequestParts {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap<HeaderValue>,
    pub extensions: Extensions,
    pub body: Option<hyper::Body>,
}

impl RequestParts {
    pub fn new(req: Request) -> Self {
        let (parts, body) = req.into_parts();
        let Parts {
            method,
            uri,
            version,
            headers,
            extensions,
            ..
        } = parts;

        RequestParts {
            method,
            uri,
            version,
            headers,
            extensions,
            body: Some(body),
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        self.uri().path()
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.headers
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub async fn read_body(&mut self) -> Result<Bytes, Error> {
        let mut bufs = BytesMut::new();

        match &mut self.body {
            Some(body) => {
                while let Some(buf) = body.data().await {
                    let buf = buf?;
                    if buf.has_remaining() {
                        bufs.extend(buf);
                    }
                }

                Ok(bufs.freeze())
            }
            None => Ok(bufs.freeze()),
        }
    }

    pub async fn read_form<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body = self.read_body().await?;
        let form = serde_urlencoded::from_bytes(&body)?;

        Ok(form)
    }

    pub async fn read_json<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body = self.read_body().await?;
        let json = serde_json::from_slice(&body)?;

        Ok(json)
    }

    pub(crate) fn from_other(other: &mut Self) -> Self {
        let body = None;
        let extensions = Extensions::new();

        RequestParts {
            method: other.method.clone(),
            uri: other.uri.clone(),
            version: other.version,
            headers: other.headers.clone(),
            extensions: std::mem::replace(&mut other.extensions, extensions),
            body: std::mem::replace(&mut other.body, body),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RequestCtx {
    params: Params,
    remote_addr: Option<SocketAddr>,
    route_path: Option<String>,
}

impl RequestCtx {
    pub(crate) fn init(request: &mut Request, remote_addr: Option<SocketAddr>) {
        let ctx = RequestCtx {
            params: Params::new(),
            remote_addr,
            route_path: None,
        };

        request.extensions_mut().insert(ctx);
    }

    pub(crate) fn params(req: &RequestParts) -> Option<&Params> {
        req.extensions()
            .get::<Self>()
            .and_then(|ctx| Some(&ctx.params))
    }

    pub(crate) fn remote_addr(req: &RequestParts) -> Option<SocketAddr> {
        req.extensions()
            .get::<RequestCtx>()
            .and_then(|ctx| ctx.remote_addr)
    }

    pub(crate) fn get_remote_addr(req: &Request) -> Option<SocketAddr> {
        req.extensions()
            .get::<RequestCtx>()
            .and_then(|ctx| ctx.remote_addr)
    }

    pub(crate) fn route_path(req: &mut Request) -> &str {
        let ctx = req
            .extensions()
            .get::<Self>()
            .expect("can not get RequestCtx from req.extensions()");

        match ctx.route_path {
            Some(ref path) => path,
            None => req.uri().path(),
        }
    }

    pub(crate) fn set_route_path(req: &mut Request, path: &str) {
        let ctx = req
            .extensions_mut()
            .get_mut::<Self>()
            .expect("can not get RequestCtx from req.extensions()");
        ctx.route_path = Some(path.to_string());
    }

    pub(crate) fn merge_params(req: &mut Request, other: &Params) {
        let ctx = req
            .extensions_mut()
            .get_mut::<Self>()
            .expect("can not get RequestCtx from req.extensions()");

        for (k, v) in other {
            ctx.params.insert(k.to_string(), v.to_string());
        }
    }
}
