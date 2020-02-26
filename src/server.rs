use std::net::SocketAddr;
use std::sync::Arc;

use hyper::http;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use lazy_static::lazy_static;

use crate::endpoint::{Endpoint, RouterEndpoint};
use crate::middleware::Middleware;
use crate::request::Request;
use crate::router::Router;
use crate::Error;

lazy_static! {
    pub static ref SERVER_ID: String = format!("Lieweb {}", env!("CARGO_PKG_VERSION"));
}

pub struct App<State> {
    state: State,
    router: Router<State>,
}

impl<State: Send + Sync + 'static> App<State> {
    pub fn with_state(state: State) -> App<State> {
        App {
            state,
            router: Router::new(),
        }
    }

    pub fn router_mut(&mut self) -> &mut Router<State> {
        &mut self.router
    }

    pub fn register(
        &mut self,
        method: http::Method,
        path: &str,
        ep: impl Endpoint<State>,
    ) -> &mut Self {
        self.router.register(method, path, ep);
        self
    }

    pub fn attach(&mut self, prefix: &str, router: Router<State>) -> Result<(), Error> {
        self.router.attach(prefix, router)
    }

    pub fn middleware(&mut self, m: impl Middleware<State>) -> &mut Self {
        self.router.middleware(m);
        self
    }

    pub fn set_not_found(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.router.set_not_found(ep);
        self
    }

    pub async fn run(self, addr: &SocketAddr) -> Result<(), Error> {
        let App { state, router } = self;

        let state = Arc::new(state);
        let router = Arc::new(router);

        let make_service = make_service_fn(move |socket: &AddrStream| {
            let state = state.clone();
            let remote_addr = socket.remote_addr();
            let router = router.clone();

            async move {
                // This is the `Service` that will handle the connection.
                // `service_fn` is a helper to convert a function that
                // returns a Response into a `Service`.
                Ok::<_, Error>(service_fn(move |req| {
                    let router = router.clone();
                    let state = state.clone();

                    async move {
                        let request = Request::new(req, state, Some(remote_addr));

                        let endpoint = RouterEndpoint::new(router);
                        let resp = endpoint.call(request).await;

                        Ok::<_, Error>(resp)
                    }
                }))
            }
        });

        let server = hyper::Server::bind(&addr).serve(make_service);
        println!("Listening on http://{}", addr);
        server.await?;

        Ok(())
    }
}
