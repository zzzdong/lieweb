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
use crate::{middleware::WithState, Error};

pub struct Request {
    pub(crate) inner: HyperRequest,
    params: Params,
    remote_addr: Option<SocketAddr>,
    route_path: Option<String>,
}

impl Request {
    pub fn new(request: HyperRequest, remote_addr: Option<SocketAddr>) -> Self {
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

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn get_cookie(&self, name: &str) -> Result<String, Error> {
        let cookie: headers::Cookie = self.get_typed_header()?;

        cookie
            .get(name)
            .ok_or_else(|| missing_cookie(name))
            .map(|s| s.to_string())
    }

    pub fn get_state<T>(&self) -> Result<&T, Error>
    where
        T: Send + Sync + 'static + Clone,
    {
        WithState::get_state(self).ok_or_else(|| missing_appstate(std::any::type_name::<T>()))
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
                .map_err(|e| invalid_param(param, std::any::type_name::<T>(), e)),
            None => Err(missing_param(param)),
        }
    }

    pub fn get_header<K>(&self, header: K) -> Result<&HeaderValue, Error>
    where
        HeaderName: From<K>,
    {
        let key: HeaderName = header.into();
        let key_cloned = key.clone();
        let value = self
            .inner
            .headers()
            .get(key)
            .ok_or_else(|| missing_header(key_cloned))?;

        Ok(value)
    }

    pub fn get_typed_header<T: Header + Send + 'static>(&self) -> Result<T, Error> {
        self.inner
            .headers()
            .typed_get::<T>()
            .ok_or_else(|| invalid_header(T::name().as_str()))
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

impl From<hyper::Request<hyper::Body>> for Request {
    fn from(req: hyper::Request<hyper::Body>) -> Self {
        Request::new(req, None)
    }
}
