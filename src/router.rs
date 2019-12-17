use std::collections::HashMap;

use crate::endpoint::{DynEndpoint, Endpoint};
use crate::{Request, Response};

type HandlerMap<State> = HashMap<String, Box<DynEndpoint<State>>>;

pub struct Router<State> {
    handlers: HashMap<http::Method, HandlerMap<State>>,
    handle_not_found: Option<Box<DynEndpoint<State>>>,
}

impl<State> Router<State>
where
    State: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Router {
            handlers: HashMap::new(),
            handle_not_found: Some(Box::new(move |cx| {
                Box::pin(Endpoint::call(&Self::handle_not_found, cx))
            })),
        }
    }

    pub fn register(
        &mut self,
        method: http::Method,
        path: impl ToString,
        ep: impl Endpoint<State>,
    ) {
        self.handlers
            .entry(method)
            .or_insert_with(HashMap::new)
            .entry(path.to_string())
            .or_insert_with(|| Box::new(move |cx| Box::pin(ep.call(cx))));
    }

    pub(crate) fn find(&self, method: &http::Method, path: &str) -> &DynEndpoint<State> {
        let map = self.handlers.get(method);
        if map.is_none() {
            return self.handle_not_found.as_ref().unwrap();
        }

        let handler = map.unwrap().get(path);
        if handler.is_none() {
            return self.handle_not_found.as_ref().unwrap();
        }

        handler.unwrap()
    }

    pub fn set_not_found(&mut self, ep: impl Endpoint<State>) {
        self.handle_not_found = Some(Box::new(move |cx| Box::pin(ep.call(cx))))
    }

    pub(crate) async fn handle_not_found(
        _request: Request<State>,
    ) -> Result<Response, std::io::Error> {
        Ok(Response {
            inner: hyper::Response::builder()
                .status(404)
                .body(hyper::Body::from("Not Found"))
                .unwrap(),
        })
    }
}
