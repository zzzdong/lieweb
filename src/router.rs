use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use hyper::http;
use route_recognizer::{Match, Params, Router as MethodRouter};

use crate::endpoint::{DynEndpoint, Endpoint, RouterEndpoint};
use crate::middleware::{Middleware, Next};
use crate::{IntoResponse, Request, Response};

/// The result of routing a URL
pub(crate) struct Selection<'a, State> {
    pub(crate) endpoint: &'a DynEndpoint<State>,
    pub(crate) params: Params,
}

pub struct Router<State> {
    prefix: String,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
    method_map: HashMap<http::Method, MethodRouter<Box<DynEndpoint<State>>>>,
    handle_not_found: Option<Box<DynEndpoint<State>>>,
    all_method_router: MethodRouter<Box<DynEndpoint<State>>>,
}

impl<State> Router<State>
where
    State: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Router {
            prefix: String::new(),
            middlewares: Vec::new(),
            handle_not_found: Some(Box::new(&not_found_endpoint)),
            method_map: HashMap::new(),
            all_method_router: MethodRouter::new(),
        }
    }

    pub fn middleware(&mut self, m: impl Middleware<State>) -> &mut Self {
        self.middlewares.push(Arc::new(m));
        self
    }

    pub fn register(&mut self, method: http::Method, path: &str, ep: impl Endpoint<State>) {
        self.method_map
            .entry(method)
            .or_insert_with(MethodRouter::new)
            .add(path, Box::new(ep));
    }

    pub fn get(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::GET, path, ep)
    }
    pub fn head(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::HEAD, path, ep)
    }

    pub fn post(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::POST, path, ep)
    }

    pub fn put(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::PUT, path, ep)
    }

    pub fn delete(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::DELETE, path, ep)
    }

    pub fn connect(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::CONNECT, path, ep)
    }

    pub fn options(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::OPTIONS, path, ep)
    }

    pub fn trace(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::TRACE, path, ep)
    }

    pub fn patch(&mut self, path: &str, ep: impl Endpoint<State>) {
        self.register(http::Method::PATCH, path, ep)
    }

    pub fn attach(
        &mut self,
        prefix: &str,
        router: Router<State>,
    ) -> Result<(), crate::error::Error> {
        if !prefix.starts_with('/') || !prefix.ends_with('/') {
            return Err(crate::error::Error::Message(
                "attach nested route, prefix must be a path, start with / and end with /"
                    .to_string(),
            ));
        }

        let mut router = router;
        router.set_prefix(prefix);
        let router = Arc::new(router);

        let endpoint = RouterEndpoint::new(router);

        let path = prefix.to_string() + "*lieweb-nested-router";

        self.all_method_router.add(&path, Box::new(endpoint));

        Ok(())
    }

    pub(crate) fn find(&self, method: http::Method, path: &str) -> Selection<'_, State> {
        if let Some(Match { handler, params }) = self
            .method_map
            .get(&method)
            .and_then(|r| r.recognize(path).ok())
        {
            Selection {
                endpoint: &**handler,
                params,
            }
        } else if let Ok(Match { handler, params }) = self.all_method_router.recognize(path) {
            Selection {
                endpoint: &**handler,
                params,
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

    pub fn set_not_found(&mut self, ep: impl Endpoint<State>) {
        self.handle_not_found = Some(Box::new(ep))
    }

    pub(crate) async fn route(&self, req: Request<State>) -> Response {
        let mut req = req;
        req.append_route_prefix(&self.prefix);
        let method = req.method().clone();

        let path = req.route_path();
        let Selection { endpoint, params } = self.find(method, path);
        req.merge_params(&params);

        let next = Next {
            endpoint,
            next_middleware: &self.middlewares,
        };

        next.run(req).await
    }

    fn set_prefix(&mut self, prefix: &str) {
        self.prefix = prefix.to_string();
    }
}

impl<State: Send + Sync + 'static> Default for Router<State> {
    fn default() -> Self {
        Self::new()
    }
}

impl<State: Send + Sync + 'static> std::fmt::Debug for Router<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Router{{ prefix: {}, middlewares: (length: {}) }}",
            self.prefix,
            self.middlewares.len()
        )
    }
}

fn not_found_endpoint<State>(_cx: Request<State>) -> BoxFuture<'static, Response> {
    Box::pin(async move { http::StatusCode::NOT_FOUND.into_response() })
}

fn method_not_allowed<State>(_cx: Request<State>) -> BoxFuture<'static, Response> {
    Box::pin(async move { http::StatusCode::METHOD_NOT_ALLOWED.into_response() })
}
