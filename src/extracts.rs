use std::{convert::Infallible, net::SocketAddr};

use hyper::StatusCode;
use serde::de::DeserializeOwned;

use crate::{
    error::{invalid_param, missing_param},
    middleware::WithState,
    request::{FromRequest, RequestCtx, RequestParts},
    response::IntoResponse,
    LieResponse, Response,
};

pub struct Params {
    inner: route_recognizer::Params,
}

impl Params {
    pub(crate) fn new(inner: route_recognizer::Params) -> Self {
        Params { inner }
    }

    pub fn get<T>(&self, param: &str) -> Result<T, crate::Error>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error,
    {
        match self.inner.find(param) {
            Some(param) => param
                .parse()
                .map_err(|e| invalid_param(param, std::any::type_name::<T>(), e)),
            None => Err(missing_param(param)),
        }
    }
}

#[crate::async_trait]
impl FromRequest for Params {
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection> {
        let params = RequestCtx::params(req).expect("params not found");

        Ok(Params::new(params.clone()))
    }
}

pub struct AppState<T> {
    inner: T,
}

impl<T> AppState<T> {
    pub(crate) fn new(inner: T) -> Self {
        AppState { inner }
    }

    pub fn value(&self) -> &T {
        &self.inner
    }
}

#[crate::async_trait]
impl<T> FromRequest for AppState<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Rejection = StateRejection;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection> {
        WithState::get_state(req)
            .ok_or(StateRejection)
            .map(|inner: T| AppState { inner })
    }
}

pub struct StateRejection;

impl IntoResponse for StateRejection {
    fn into_response(self) -> Response {
        LieResponse::with_str("can not extract AppState")
            .set_status(StatusCode::INTERNAL_SERVER_ERROR)
            .into()
    }
}

pub struct RemoteAddr {
    addr: Option<SocketAddr>,
}

impl RemoteAddr {
    pub fn value(&self) -> Option<SocketAddr> {
        self.addr
    }
}

#[crate::async_trait]
impl FromRequest for RemoteAddr {
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection> {
        let addr = RequestCtx::remote_addr(req);

        Ok(RemoteAddr { addr })
    }
}

#[crate::async_trait]
impl FromRequest for RequestParts {
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection> {
        Ok(RequestParts::from_other(req))
    }
}

#[derive(Default)]
pub struct Query<T: Default> {
    inner: T,
}

impl<T: Default> Query<T> {
    pub fn value(&self) -> &T {
        &self.inner
    }
}

#[crate::async_trait]
impl<T> FromRequest for Query<T>
where
    T: DeserializeOwned + Default,
{
    type Rejection = QueryRejection;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection> {
        match req.uri().query() {
            Some(query) => serde_urlencoded::from_str::<T>(query)
                .map(|inner| Query { inner })
                .map_err(QueryRejection::from),
            None => Ok(Default::default()),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum QueryRejection {
    #[error("decode query string error")]
    DecodeFailed(#[from] serde_urlencoded::de::Error),
}

impl IntoResponse for QueryRejection {
    fn into_response(self) -> Response {
        match self {
            Self::DecodeFailed(e) => LieResponse::with_status(StatusCode::BAD_REQUEST).into(),
        }
    }
}
