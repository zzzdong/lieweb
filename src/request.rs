use std::net::SocketAddr;

use bytes::{Buf, Bytes, BytesMut};
use hyper::body::HttpBody;
use hyper::http::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use route_recognizer::Params;
use serde::de::DeserializeOwned;

pub type HyperRequest = hyper::Request<hyper::Body>;

use crate::Error;
use crate::middleware::AppState;

pub struct Request {
    pub(crate) inner: HyperRequest,
    params: Params,
    remote_addr: SocketAddr,
    route_path: Option<String>,
}

impl Request {
    pub(crate) fn new(request: HyperRequest, remote_addr: SocketAddr) -> Self {
        Request {
            inner: request,
            params: Params::new(),
            remote_addr,
            route_path: None,
        }
    }

    pub fn inner(&self) -> &HyperRequest {
        &self.inner
    }

    pub fn innner_mut(&mut self) -> &mut HyperRequest {
        &mut self.inner
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.inner.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        self.inner.headers_mut()
    }

    pub fn method(&self) -> &http::Method {
        self.inner.method()
    }

    pub fn uri(&self) -> &http::Uri {
        self.inner.uri()
    }

    pub fn path(&self) -> &str {
        self.uri().path()
    }

    pub fn version(&self) -> http::Version {
        self.inner.version()
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn get_state<T>(&self) -> Result<&T, Error>
    where
        T: Send + Sync + 'static + Clone,
    {
        self.inner
            .extensions()
            .get::<AppState<T>>()
            .map(|o| o.inner.as_ref())
            .ok_or_else(|| crate::error_msg!("state{:?} not exist", std::any::type_name::<T>()))
    }

    pub fn get_extension<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static + Clone,
    {
        self.inner.extensions().get::<T>()
    }

    pub fn get_extension_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Send + Sync + 'static + Clone,
    {
        self.inner.extensions_mut().get_mut::<T>()
    }

    pub fn insert_extension<T>(&mut self, val: T)
    where
        T: Send + Sync + 'static + Clone,
    {
        self.inner.extensions_mut().insert(val);
    }

    pub fn get_param<T>(&self, param: &str) -> Result<T, Error>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error,
    {
        match self.params.find(param) {
            Some(param) => param
                .parse()
                .map_err(|e| crate::error_msg!("parse param error: {:?}", e)),
            None => Err(crate::error_msg!("param {} not found", param)),
        }
    }

    pub fn get_header<K>(&self, header: K) -> Result<&HeaderValue, Error>
    where
        HeaderName: From<K>,
    {
        let key: HeaderName = header.into();
        let value = self
            .inner
            .headers()
            .get(key)
            .ok_or_else(|| crate::error_msg!("Header not found"))?;

        Ok(value)
    }

    pub fn get_query<T: DeserializeOwned + Default>(&self) -> Result<T, Error> {
        match self.inner.uri().query() {
            Some(query) => serde_urlencoded::from_str(query).map_err(Error::from),
            None => Ok(Default::default()),
        }
    }

    pub async fn read_body(&mut self) -> Result<Bytes, Error> {
        let mut bufs = BytesMut::new();

        while let Some(buf) = self.inner.body_mut().data().await {
            let buf = buf?;
            if buf.has_remaining() {
                bufs.extend(buf);
            }
        }

        Ok(bufs.freeze())
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

    pub(crate) fn route_path(&self) -> &str {
        match self.route_path {
            Some(ref path) => path,
            None => self.inner.uri().path(),
        }
    }

    pub(crate) fn set_route_path(&mut self, path: &str) {
        self.route_path = Some(path.to_string());
    }

    pub(crate) fn merge_params(&mut self, other: &Params) -> &Params {
        for (k, v) in other {
            self.params.insert(k.to_string(), v.to_string());
        }

        &self.params
    }
}
