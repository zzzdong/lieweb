use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::Path;
use std::sync::Arc;

use hyper::http;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use lazy_static::lazy_static;
use tokio::net::TcpListener;

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
        let App { router } = self;
        let router = Arc::new(router);

        let server = Http::new();

        let listener = TcpListener::bind(addr).await.unwrap();
        while let Ok((socket, remote_addr)) = listener.accept().await {
            let server = server.clone();
            let router = router.clone();

            tokio::spawn(async move {
                let router = router.clone();

                let ret = server.serve_connection(
                    socket,
                    service_fn(|req| {
                        let router = router.clone();
                        let req = Request::new(req, remote_addr);

                        async move {
                            let endpoint = RouterEndpoint::new(router);
                            let resp = endpoint.call(req).await;
                            Ok::<_, Error>(resp.into())
                        }
                    }),
                );

                if let Err(e) = ret.await {
                    tracing::error!("serve_connection error: {:?}", e);
                }
            });
        }

        Ok(())
    }

    #[cfg(feature = "tls")]
    pub async fn run_with_tls(
        self,
        addr: &SocketAddr,
        cert: impl AsRef<Path>,
        key: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let App { router } = self;
        let router = Arc::new(router);

        let server = Http::new();

        let tls_acceptor = crate::tls::new_tls_acceptor(cert, key)?;

        let listener = TcpListener::bind(addr).await.unwrap();
        while let Ok((socket, remote_addr)) = listener.accept().await {
            let tls_acceptor = tls_acceptor.clone();
            let server = server.clone();
            let router = router.clone();

            tokio::spawn(async move {
                let tls_acceptor = tls_acceptor.clone();
                let router = router.clone();

                match tls_acceptor.accept(socket).await {
                    Ok(stream) => {
                        let ret = server.serve_connection(
                            stream,
                            service_fn(|req| {
                                let router = router.clone();
                                let req = Request::new(req, remote_addr);

                                async move {
                                    let endpoint = RouterEndpoint::new(router);
                                    let resp = endpoint.call(req).await;
                                    Ok::<_, Error>(resp.into())
                                }
                            }),
                        );

                        if let Err(e) = ret.await {
                            tracing::error!("serve_connection error: {:?}", e);
                        }
                    }
                    Err(err) => {
                        tracing::error!("tls accept failed, {:?}", err);
                    }
                }
            });
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
