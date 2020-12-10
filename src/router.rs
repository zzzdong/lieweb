use std::collections::HashMap;
use std::sync::Arc;

use hyper::http;
use route_recognizer::{Params, Router as MethodRouter};

use crate::endpoint::{DynEndpoint, Endpoint, RouterEndpoint};
use crate::middleware::{Middleware, Next};
use crate::register_method;
use crate::{Request, Response};

const LIEWEB_NESTED_ROUTER: &str = "--lieweb-nested-router";

/// The result of routing a URL
pub(crate) struct Selection<'a> {
    pub(crate) endpoint: &'a DynEndpoint,
    pub(crate) params: Params,
}

pub struct Router {
    middlewares: Vec<Arc<dyn Middleware>>,
    method_map: HashMap<http::Method, MethodRouter<Box<DynEndpoint>>>,
    handle_not_found: Option<Box<DynEndpoint>>,
    all_method_router: MethodRouter<Box<DynEndpoint>>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            middlewares: Vec::new(),
            handle_not_found: Some(Box::new(&not_found_endpoint)),
            method_map: HashMap::new(),
            all_method_router: MethodRouter::new(),
        }
    }

    pub fn middleware(&mut self, m: impl Middleware) -> &mut Self {
        self.middlewares.push(Arc::new(m));
        self
    }

    pub fn register(&mut self, method: http::Method, path: impl AsRef<str>, ep: impl Endpoint) {
        self.method_map
            .entry(method)
            .or_insert_with(MethodRouter::new)
            .add(path.as_ref(), Box::new(ep));
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

    pub fn attach(&mut self, prefix: &str, router: Router) -> Result<(), crate::error::Error> {
        if !prefix.starts_with('/') || !prefix.ends_with('/') {
            return Err(crate::error::Error::Message(
                "attach nested route, prefix must be a path, start with / and end with /"
                    .to_string(),
            ));
        }
        let router = Arc::new(router);

        let endpoint = RouterEndpoint::new(router);

        let path = prefix.to_string() + "*" + LIEWEB_NESTED_ROUTER;

        self.all_method_router.add(&path, Box::new(endpoint));

        Ok(())
    }

    pub(crate) fn find(&self, method: http::Method, path: &str) -> Selection {
        if let Some(m) = self
            .method_map
            .get(&method)
            .and_then(|r| r.recognize(path).ok())
        {
            Selection {
                endpoint: &***m.handler(),
                params: m.params().clone(),
            }
        } else if let Ok(m) = self.all_method_router.recognize(path) {
            Selection {
                endpoint: &***m.handler(),
                params: m.params().clone(),
            }
        } else if method == http::Method::HEAD {
            // If it is a HTTP HEAD request then check if there is a callback in the endpoints map
            // if not then fallback to the behavior of HTTP GET else proceed as usual
            self.find(http::Method::GET, path)
        } else if self
            .method_map
            .iter()
            .filter(|(k, _)| *k != method)
            .any(|(_, r)| r.recognize(path).is_ok())
        {
            // If this `path` can be handled by a callback registered with a different HTTP method
            // should return 405 Method Not Allowed
            Selection {
                endpoint: &method_not_allowed,
                params: Params::new(),
            }
        } else {
            match self.handle_not_found {
                Some(ref handler) => {
                    let endpoint = handler;

                    Selection {
                        endpoint: &**endpoint,
                        params: Params::new(),
                    }
                }
                None => Selection {
                    endpoint: &not_found_endpoint,
                    params: Params::new(),
                },
            }
        }
    }

    pub fn set_not_found_handler(&mut self, ep: impl Endpoint) {
        self.handle_not_found = Some(Box::new(ep))
    }

    pub(crate) async fn route(&self, req: Request) -> Response {
        let mut req = req;

        let method = req.method().clone();

        let path = req.route_path();
        let Selection { endpoint, params } = self.find(method, path);

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
