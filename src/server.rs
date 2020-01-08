use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use lazy_static::lazy_static;

use crate::endpoint::Endpoint;
use crate::middleware::{Middleware, Next};
use crate::request::Request;
use crate::router::{Router, Selection};
use crate::Error;

lazy_static! {
    pub static ref SERVER_ID: String = format!("Lieweb {}", env!("CARGO_PKG_VERSION"));
}

pub struct App<State> {
    state: State,
    router: Router<State>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State: Send + Sync + 'static> App<State> {
    pub fn with_state(state: State) -> App<State> {
        App {
            state,
            router: Router::new(),
            middlewares: Vec::new(),
        }
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

    pub fn middleware(&mut self, m: impl Middleware<State>) -> &mut Self {
        self.middlewares.push(Arc::new(m));
        self
    }

    pub fn set_not_found(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.router.set_not_found(ep);
        self
    }

    pub async fn run(self, addr: &SocketAddr) -> Result<(), Error> {
        let App {
            state,
            router,
            middlewares,
        } = self;

        let state = Arc::new(state);
        let router = Arc::new(router);

        let make_service = make_service_fn(move |socket: &AddrStream| {
            let state = state.clone();
            let remote_addr = socket.remote_addr();
            let router = router.clone();
            let middlewares = middlewares.clone();

            async move {
                // This is the `Service` that will handle the connection.
                // `service_fn` is a helper to convert a function that
                // returns a Response into a `Service`.
                Ok::<_, Error>(service_fn(move |req| {
                    let path = req.uri().path().to_string();
                    let method = req.method().clone();
                    let router = router.clone();
                    let state = state.clone();
                    let middlewares = middlewares.clone();

                    async move {
                        let Selection { endpoint, params } = router.find(method, &path);
                        let request = Request::new(req, params, state, Some(remote_addr));
                        let next = Next {
                            endpoint,
                            next_middleware: &middlewares,
                        };

                        let resp = next.run(request).await;
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

    // pub async fn run2(self, addr: &SocketAddr) -> Result<(), Error> {
    //     let App { state, router } = self;

    //     let state = Arc::new(state);
    //     let router = Arc::new(router);

    //     let svc = Service {
    //         state,
    //         router,
    //         remote_addr: None,
    //     };

    //     let server = Server::bind(&addr).serve(MakeSvc { inner: svc });
    //     println!("Listening on http://{}", addr);
    //     server.await?;

    //     Ok(())
    // }
}
