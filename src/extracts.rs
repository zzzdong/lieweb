use std::{
    convert::Infallible,
    net::SocketAddr,
    ops::{Deref, DerefMut},
};

use hyper::StatusCode;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{
    error::{invalid_param, missing_param},
    middleware::WithState,
    request::{FromRequest, RequestCtx, RequestParts},
    response::IntoResponse,
    LieResponse, Response,
};

pub struct ParamsRejection(params_de::Error);

impl IntoResponse for ParamsRejection {
    fn into_response(self) -> Response {
        LieResponse::new(StatusCode::BAD_REQUEST, format!("Path param parse error")).into()
    }
}

pub struct Params<T> {
    value: T,
}

impl<T> Params<T>
where
    T: DeserializeOwned,
{
    pub(crate) fn from_params(params: &route_recognizer::Params) -> Result<Self, ParamsRejection> {
        params_de::from_params::<T>(params)
            .map(|value| Params { value })
            .map_err(|e| ParamsRejection(e))
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn take(self) -> T {
        self.value
    }
}

#[crate::async_trait]
impl<T> FromRequest for Params<T>
where
    T: DeserializeOwned + Default,
{
    type Rejection = ParamsRejection;

    async fn from_request(req: &mut RequestParts) -> Result<Self, Self::Rejection> {
        let params = RequestCtx::params(req).expect("params not found");

        Params::from_params(params)
    }
}

pub struct AppState<T> {
    value: T,
}

impl<T> AppState<T> {
    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn take(self) -> T {
        self.value
    }
}

impl<T> Deref for AppState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for AppState<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
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
            .map(|value: T| AppState { value })
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
    value: T,
}

impl<T: Default> Query<T> {
    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn take(self) -> T {
        self.value
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
                .map(|value| Query { value })
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

mod params_de {
    use std::fmt::{self, Display};

    use super::Params;
    use serde::{
        de::{self, DeserializeOwned, IntoDeserializer, MapAccess},
        Deserializer,
    };

    #[derive(Clone, Debug, PartialEq)]
    pub enum Error {
        Message(String),
        Eof,
        Unsupported,
    }

    impl de::Error for Error {
        fn custom<T: Display>(msg: T) -> Self {
            Error::Message(msg.to_string())
        }
    }

    impl Display for Error {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Error::Message(msg) => formatter.write_str(msg),
                Error::Eof => formatter.write_str("unexpected end of input"),
                Error::Unsupported => formatter.write_str("unsupported type"),
            }
        }
    }

    impl std::error::Error for Error {}

    struct PathParamsDeserialzer<'de> {
        inner: &'de mut route_recognizer::Iter<'de>,
    }

    impl<'de> PathParamsDeserialzer<'de> {
        pub fn from_params(inner: &'de mut route_recognizer::Iter<'de>) -> Self {
            PathParamsDeserialzer { inner }
        }
    }

    pub fn from_params<'a, T>(params: &'a route_recognizer::Params) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut iter = params.into_iter();
        let mut deserializer = PathParamsDeserialzer::from_params(&mut iter);
        let t = T::deserialize(&mut deserializer)?;
        Ok(t)
    }

    impl<'de, 'a> Deserializer<'de> for &'a mut PathParamsDeserialzer<'de> {
        type Error = Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            Err(Error::Unsupported)
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct enum identifier ignored_any
        }

        fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            struct Access<'de, 'a> {
                iter: &'a mut route_recognizer::Iter<'de>,
                entry: Option<(&'de str, &'de str)>,
            }

            impl<'de, 'a> Access<'de, 'a> {
                fn new(de: &'a mut PathParamsDeserialzer<'de>) -> Self {
                    Access {
                        iter: de.inner,
                        entry: None,
                    }
                }
            }

            impl<'de, 'a> MapAccess<'de> for Access<'de, 'a> {
                type Error = Error;

                fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
                where
                    K: de::DeserializeSeed<'de>,
                {
                    match self.iter.next() {
                        Some(entry) => {
                            self.entry = Some(entry);
                            seed.deserialize(PartDeserialzer { inner: entry.0 })
                                .map(Some)
                        }
                        None => Ok(None),
                    }
                }

                fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
                where
                    V: de::DeserializeSeed<'de>,
                {
                    match self.entry {
                        Some(entry) => seed.deserialize(PartDeserialzer { inner: entry.1 }),
                        None => Err(Error::Eof),
                    }
                }
            }

            visitor.visit_map(Access::new(&mut self))
        }

        fn deserialize_struct<V>(
            self,
            name: &'static str,
            fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            self.deserialize_map(visitor)
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct PartDeserialzer<'de> {
        inner: &'de str,
    }

    impl<'de> PartDeserialzer<'de> {
        fn parse<F>(&self) -> Result<F, Error>
        where
            F: std::str::FromStr,
            <F as std::str::FromStr>::Err: std::fmt::Debug,
        {
            str::parse(self.inner).map_err(|e| Error::Message(format!("{e:?}")))
        }
    }

    macro_rules! deserialize_from_str {
        ($trait_fn:ident, $visit_fn: ident) => {
            fn $trait_fn<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                visitor.$visit_fn(self.parse()?)
            }
        };
    }

    impl<'de, 'a> Deserializer<'de> for PartDeserialzer<'de> {
        type Error = Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            Err(Error::Unsupported)
        }

        serde::forward_to_deserialize_any! {
            char bytes byte_buf unit unit_struct newtype_struct seq tuple
            tuple_struct map struct ignored_any
        }

        deserialize_from_str!(deserialize_bool, visit_bool);
        deserialize_from_str!(deserialize_i8, visit_i8);
        deserialize_from_str!(deserialize_i16, visit_i16);
        deserialize_from_str!(deserialize_i32, visit_i32);
        deserialize_from_str!(deserialize_i64, visit_i64);
        deserialize_from_str!(deserialize_u8, visit_u8);
        deserialize_from_str!(deserialize_u16, visit_u16);
        deserialize_from_str!(deserialize_u32, visit_u32);
        deserialize_from_str!(deserialize_u64, visit_u64);
        deserialize_from_str!(deserialize_f32, visit_f32);
        deserialize_from_str!(deserialize_f64, visit_f64);

        fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            visitor.visit_borrowed_str(self.inner)
        }

        fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            self.deserialize_str(visitor)
        }

        fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            visitor.visit_some(self)
        }

        fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            self.deserialize_str(visitor)
        }

        fn deserialize_enum<V>(
            self,
            name: &'static str,
            variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            visitor.visit_enum(self.inner.into_deserializer())
        }
    }

    #[cfg(test)]
    mod test {
        use super::from_params;
        use crate::Params;

        #[test]
        fn test() {
            let mut params = route_recognizer::Params::new();
            params.insert("version".into(), "v1".into());
            params.insert("id".into(), "123".into());
            params.insert("flag".into(), "false".into());

            #[derive(Debug, serde::Deserialize)]
            enum Version {
                v1,
                v2,
            }

            #[derive(Debug, serde::Deserialize)]
            struct PathParams {
                pub version: Version,
                pub id: Option<u32>,
                pub name: Option<String>,
                pub flag: bool,
            }

            let p: PathParams = from_params(&params).unwrap();

            println!("params: {:?}", &p);
        }
    }
}
