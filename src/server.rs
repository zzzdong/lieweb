use std::future::Future;
use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::Path;
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
        self.run_with_shutdown::<futures::future::Ready<()>>(addr, None)
            .await
    }

    pub async fn run_with_shutdown<F>(
        self,
        addr: &SocketAddr,
        signal: Option<F>,
    ) -> Result<(), Error>
    where
        F: Future<Output = ()>,
    {
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

        if let Some(signal) = signal {
            hyper::Server::bind(&addr)
                .serve(make_svc)
                .with_graceful_shutdown(signal)
                .await?;
        } else {
            hyper::Server::bind(&addr).serve(make_svc).await?;
        }

        Ok(())
    }

    #[cfg(feature = "tls")]
    pub async fn run_with_tls<F>(
        self,
        addr: &SocketAddr,
        cert: impl AsRef<Path>,
        key: impl AsRef<Path>,
        signal: Option<F>,
    ) -> Result<(), Error>
    where
        F: Future<Output = ()>,
    {
        let App { router } = self;
        let router = Arc::new(router);

        // And a MakeService to handle each connection...
        let make_svc = make_service_fn(|socket: &crate::tls::TlsStream| {
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

        let incoming = crate::tls::TlsIncoming::new(addr, cert, key)?;

        if let Some(signal) = signal {
            hyper::Server::builder(incoming)
                .serve(make_svc)
                .with_graceful_shutdown(signal)
                .await?;
        } else {
            hyper::Server::builder(incoming).serve(make_svc).await?;
        }

        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub fn server_id() -> &'static str {
    &SERVER_ID
}
