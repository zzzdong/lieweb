use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{server::conn::AddrStream, Server};

use crate::endpoint::Endpoint;
use crate::router::Router;
use crate::LieError;
use crate::{Request, Response};

pub struct App<State, E> {
    state: State,
    router: Router<State, E>,
}

impl<State: Send + Sync + 'static, E: Send + Sync + 'static> App<State, E>
where
    E: std::error::Error,
{
    pub fn with_state(state: State) -> App<State, E> {
        App {
            state,
            router: Router::new(),
        }
    }

    pub fn register(
        &mut self,
        method: http::Method,
        path: impl ToString,
        ep: impl Endpoint<State, E>,
    ) {
        self.router.register(method, path, ep)
    }

    pub async fn run(self, addr: &SocketAddr) -> Result<(), crate::LieError> {
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
                Ok::<_, LieError>(service_fn(move |req| {
                    let path = req.uri().path().to_string();
                    let method = req.method().clone();

                    let request = Request::new(req, state.clone(), remote_addr);

                    let router = router.clone();

                    async move {
                        let handler = router.find(&method, &path);
                        let resp = match handler(request).await {
                            Ok(ret) => ret,
                            Err(e) => Self::handle_error(e),
                        };

                        Ok::<_, LieError>(resp.into())
                    }
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_service);
        println!("Listening on http://{}", addr);
        server.await?;

        Ok(())
    }

    fn handle_error(e: impl std::error::Error) -> Response {
        Response {
            inner: hyper::Response::builder()
                .status(500)
                .body(hyper::Body::from(format!("{:?}", e)))
                .unwrap(),
        }
    }
}
