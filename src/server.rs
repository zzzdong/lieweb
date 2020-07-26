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
use crate::register_method;
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

    pub fn register(&mut self, method: http::Method, path: impl AsRef<str>, ep: impl Endpoint) {
        self.router.register(method, path, ep);
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

                        Ok::<_, Error>(resp.into())
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

                        Ok::<_, Error>(resp.into())
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
