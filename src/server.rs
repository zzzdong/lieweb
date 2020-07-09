use std::net::SocketAddr;
use std::sync::Arc;

use hyper::http;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use lazy_static::lazy_static;

use crate::endpoint::{Endpoint, RouterEndpoint};
use crate::error::Error;
use crate::middleware::{Middleware, WithState};
use crate::request::Request;
use crate::router::Router;

lazy_static! {
    pub static ref SERVER_ID: String = format!("Lieweb {}", env!("CARGO_PKG_VERSION"));
}

pub struct App {
    router: Router,
}

impl App {
    pub fn new() -> App {
        App {
            router: Router::new(),
        }
    }

    pub fn with_state<T>(state: T) -> App
    where
        T: Send + Sync + 'static + Clone,
    {
        let mut app = App::new();

        app.middleware(WithState::new(state));
        app
    }

    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }

    pub fn get(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.get(path, ep);
        self
    }
    pub fn head(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.head(path, ep);
        self
    }

    pub fn post(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.post(path, ep);
        self
    }

    pub fn put(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.put(path, ep);
        self
    }

    pub fn delete(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.delete(path, ep);
        self
    }

    pub fn connect(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.connect(path, ep);
        self
    }

    pub fn options(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.options(path, ep);
        self
    }

    pub fn trace(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.trace(path, ep);
        self
    }

    pub fn patch(&mut self, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.patch(path, ep);
        self
    }

    pub fn register(&mut self, method: http::Method, path: &str, ep: impl Endpoint) -> &mut Self {
        self.router.register(method, path, ep);
        self
    }

    pub fn attach(&mut self, prefix: &str, router: Router) -> Result<&mut Self, Error> {
        self.router.attach(prefix, router).map(|_| self)
    }

    pub fn middleware(&mut self, m: impl Middleware) -> &mut Self {
        self.router.middleware(m);
        self
    }

    pub fn handle_not_found(&mut self, ep: impl Endpoint) -> &mut Self {
        self.router.set_not_found_handler(ep);
        self
    }

    pub async fn run(self, addr: &SocketAddr) -> Result<(), Error> {
        let App { router } = self;
        let router = Arc::new(router);

        // And a MakeService to handle each connection...
        let make_svc = make_service_fn(|socket: &AddrStream| {
            let remote_addr = socket.remote_addr();
            let router = router.clone();

            async move {
                // This is the `Service` that will handle the connection.
                // `service_fn` is a helper to convert a function that
                // returns a Response into a `Service`.
                Ok::<_, Error>(service_fn(move |req| {
                    let router = router.clone();

                    async move {
                        let req = Request::new(req, remote_addr);

                        let endpoint = RouterEndpoint::new(router);
                        let resp = endpoint.call(req).await;

                        Ok::<_, Error>(resp)
                    }
                }))
            }
        });

        let server = hyper::Server::bind(&addr).serve(make_svc);
        println!("Listening on http://{}", addr);
        server.await?;

        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// struct Service<State, T> {
//     router: Arc<Router>,
//     state: Arc,
//     extension: Arc<T>,
//     remote_addr: SocketAddr,
// }

// impl<State, T> Clone for Service<State, T>
// where
//     State: Send + Sync + 'static,
//     T: Send + Sync + 'static,
// {
//     fn clone(&self) -> Self {
//         Service {
//             router: self.router.clone(),
//             state: self.state.clone(),
//             extension: self.extension.clone(),
//             remote_addr: self.remote_addr,
//         }
//     }
// }

// impl<State, T> HyperService<HyperRequest> for Service<State, T>
// where
//     State: Send + Sync + 'static,
//     T: Send + Sync + 'static,
// {
//     type Response = HyperResponse;
//     type Error = Error;
//     type Future = crate::utils::BoxFuture<Self::Response, Self::Error>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Ok(()).into()
//     }

//     fn call(&mut self, req: HyperRequest) -> Self::Future {
//         let router = self.router.clone();
//         let state = self.state.clone();
//         let extension = self.extension.clone();

//         let mut request = Request::new(req, state, Some(self.remote_addr));
//         request.inner_mut().extensions_mut().insert(extension);

//         let endpoint = RouterEndpoint::new(router);

//         let fut = async move {
//             let resp = Endpoint::call(&endpoint, request).await;
//             Ok::<HyperResponse, Error>(resp)
//         };

//         Box::pin(fut)
//     }
// }

// struct MakeSvc<State, T>(Service<State, T>);

// impl<State, T> HyperService<&AddrStream> for MakeSvc<State, T>
// where
//     State: Send + Sync + 'static,
//     T: Send + Sync + 'static,
// {
//     type Response = Service<State, T>;
//     type Error = std::io::Error;
//     type Future = future::Ready<Result<Self::Response, Self::Error>>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Ok(()).into()
//     }

//     fn call(&mut self, s: &AddrStream) -> Self::Future {
//         let mut svc = self.0.clone();
//         svc.remote_addr = s.remote_addr();
//         future::ok(svc)
//     }
// }

pub fn server_id() -> &'static str {
    &SERVER_ID
}
