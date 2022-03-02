use std::net::SocketAddr;

use bytes::{Buf, Bytes, BytesMut};
use headers::{Header, HeaderMapExt};
use hyper::body::HttpBody;
use hyper::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use route_recognizer::Params;
use serde::de::DeserializeOwned;

pub type HyperRequest = hyper::Request<hyper::Body>;

use crate::error::{
    invalid_header, invalid_param, missing_appstate, missing_cookie, missing_header, missing_param,
};
use crate::response::IntoResponse;
use crate::{middleware::WithState, Error};

#[crate::async_trait]
pub trait FromRequest: Sized {
    type Rejection: IntoResponse;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection>;
}

pub struct RequestParts {
    parts: hyper::http::request::Parts,
    body: Option<hyper::Body>,
}

impl RequestParts {
    pub fn new(req: HyperRequest) -> Self {
        let (parts, body) = req.into_parts();
        RequestParts {
            parts,
            body: Some(body),
        }
    }
}

pub trait RequestCtx {
    fn route_path(&self) -> &str;
    fn remote_addr(&self) -> Option<SocketAddr>;
    fn set_route_path(&mut self, path: &str);
    fn merge_params(&mut self, other: &Params) -> &Params;
}

#[derive(Debug, Clone)]
pub(crate) struct ReqCtx {
    params: Params,
    remote_addr: Option<SocketAddr>,
    route_path: Option<String>,
}

impl ReqCtx {
    pub(crate) fn init(request: &mut HyperRequest, remote_addr: Option<SocketAddr>) {
        let ctx = ReqCtx {
            params: Params::new(),
            remote_addr,
            route_path: None,
        };

        request.extensions_mut().insert(ctx);
    }

    pub(crate) fn get(req: &HyperRequest) -> &Self {
        req.extensions()
            .get::<ReqCtx>()
            .expect("can not get ReqCtx from req.extensions()")
    }

    pub(crate) fn get_mut(req: &mut HyperRequest) -> &mut Self {
        req.extensions_mut()
            .get_mut::<ReqCtx>()
            .expect("can not get ReqCtx from req.extensions()")
    }
}

impl RequestCtx for HyperRequest {
    fn remote_addr(&self) -> Option<SocketAddr> {
        match self.extensions().get::<ReqCtx>() {
            Some(r) => r.remote_addr.clone(),
            None => None,
        }
    }

    fn route_path(&self) -> &str {
        let ctx = ReqCtx::get(self);
        match ctx.route_path {
            Some(ref path) => path,
            None => self.uri().path(),
        }
    }

    fn set_route_path(&mut self, path: &str) {
        let ctx = ReqCtx::get_mut(self);
        ctx.route_path = Some(path.to_string());
    }

    fn merge_params(&mut self, other: &Params) -> &Params {
        let ctx = ReqCtx::get_mut(self);

        for (k, v) in other {
            ctx.params.insert(k.to_string(), v.to_string());
        }

        &ctx.params
    }
}
