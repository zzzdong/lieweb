use std::collections::HashMap;
use std::sync::Arc;

use hyper::http;
use route_recognizer::{Params, Router as PathRouter};

use crate::endpoint::{DynEndpoint, Endpoint, RouterEndpoint};
use crate::middleware::{Middleware, Next};
use crate::register_method;
use crate::{Request, Response};

const LIEWEB_NESTED_ROUTER: &str = "--lieweb-nested-router";

lazy_static::lazy_static! {
    pub static ref METHOD_ANY: http::Method = http::Method::from_bytes(b"__ANY__").unwrap();
}

/// The result of routing a URL
pub(crate) struct Selection<'a> {
    pub(crate) endpoint: &'a DynEndpoint,
    pub(crate) params: Params,
}

pub struct Router {
    middlewares: Vec<Arc<dyn Middleware>>,
    handle_not_found: Box<DynEndpoint>,
    path_router: PathRouter<HashMap<http::Method, Box<DynEndpoint>>>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            middlewares: Vec::new(),
            handle_not_found: Box::new(&not_found_endpoint),
            path_router: PathRouter::new(),
        }
    }

    pub fn register(&mut self, method: http::Method, path: impl AsRef<str>, ep: impl Endpoint) {
        self.path_router
            .at_or_default(path.as_ref())
            .insert(method, Box::new(ep));
    }

    register_method!(options, http::Method::OPTIONS);
    register_method!(get, http::Method::GET);
    register_method!(head, http::Method::HEAD);
    register_method!(post, http::Method::POST);
    register_method!(put, http::Method::PUT);
    register_method!(delete, http::Method::DELETE);
    register_method!(trace, http::Method::TRACE);
    register_method!(connect, http::Method::CONNECT);
    register_method!(patch, http::Method::PATCH);

    pub fn middleware(&mut self, m: impl Middleware) -> &mut Self {
        self.middlewares.push(Arc::new(m));
        self
    }

    pub fn set_not_found_handler(&mut self, ep: impl Endpoint) {
        self.handle_not_found = Box::new(ep)
    }

    pub fn merge(
        &mut self,
        prefix: impl AsRef<str>,
        sub: Router,
    ) -> Result<(), crate::error::Error> {
        let prefix = prefix.as_ref();
        if !prefix.starts_with('/') || !prefix.ends_with('/') {
            return Err(crate::error::Error::Message(
                "merge nested route, prefix must be a path, start with / and end with /"
                    .to_string(),
            ));
        }

        let path = prefix.to_string() + "*" + LIEWEB_NESTED_ROUTER;

        let mut sub_router: HashMap<http::Method, Box<DynEndpoint>> = HashMap::new();

        sub_router.insert(
            METHOD_ANY.clone(),
            Box::new(RouterEndpoint::new(Arc::new(sub))),
        );

        self.path_router.add(&path, sub_router);

        Ok(())
    }

    pub(crate) fn find(&self, path: &str, method: http::Method) -> Selection {
        match self.path_router.recognize(path) {
            Ok(m) => {
                if let Some(ep) = m.handler().get(&method) {
                    return Selection {
                        endpoint: &**ep,
                        params: m.params().clone(),
                    };
                }

                if let Some(sub) = m.handler().get(&METHOD_ANY) {
                    return Selection {
                        endpoint: &**sub,
                        params: m.params().clone(),
                    };
                }

                if m.handler().is_empty() {
                    Selection {
                        endpoint: &*self.handle_not_found,
                        params: Params::new(),
                    }
                } else {
                    Selection {
                        endpoint: &method_not_allowed,
                        params: Params::new(),
                    }
                }
            }
            Err(_e) => Selection {
                endpoint: &*self.handle_not_found,
                params: Params::new(),
            },
        }
    }

    pub(crate) async fn route(&self, req: Request) -> Response {
        let mut req = req;

        let method = req.method().clone();

        let path = req.route_path();
        let Selection { endpoint, params } = self.find(path, method);

        req.merge_params(&params);
        if let Some(rest) = params.find(LIEWEB_NESTED_ROUTER) {
            req.set_route_path(rest);
        }

        let next = Next {
            endpoint,
            next_middleware: &self.middlewares,
        };

        next.run(req).await
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Router{{ middlewares: (length: {}) }}",
            self.middlewares.len()
        )
    }
}

async fn not_found_endpoint(_ctx: Request) -> Response {
    http::StatusCode::NOT_FOUND.into()
}

async fn method_not_allowed(_ctx: Request) -> Response {
    http::StatusCode::METHOD_NOT_ALLOWED.into()
}
