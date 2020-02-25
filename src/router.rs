use std::collections::HashMap;

use futures::future::BoxFuture;
use route_recognizer::{Match, Params, Router as MethodRouter};

use crate::endpoint::{DynEndpoint, Endpoint};
use crate::{IntoResponse, Request, Response};

pub struct Router<State> {
    method_map: HashMap<http::Method, MethodRouter<Box<DynEndpoint<State>>>>,
    handle_not_found: Option<Box<DynEndpoint<State>>>,
}

/// The result of routing a URL
pub(crate) struct Selection<'a, State> {
    pub(crate) endpoint: &'a DynEndpoint<State>,
    pub(crate) params: Params,
}

impl<State> Router<State>
where
    State: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Router {
            method_map: HashMap::new(),
            handle_not_found: Some(Box::new(&not_found_endpoint)),
        }
    }

    pub fn register(&mut self, method: http::Method, path: &str, ep: impl Endpoint<State>) {
        self.method_map
            .entry(method)
            .or_insert_with(MethodRouter::new)
            .add(path, Box::new(ep));
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
        } else if method == http::Method::HEAD {
            // If it is a HTTP HEAD request then check if there is a callback in the endpoints map
            // if not then fallback to the behavior of HTTP GET else proceed as usual

            self.find(http::Method::GET, path)
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
}

fn not_found_endpoint<State>(_cx: Request<State>) -> BoxFuture<'static, Response> {
    Box::pin(async move { http::StatusCode::NOT_FOUND.into_response() })
}
