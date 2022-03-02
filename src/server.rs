#[cfg(feature = "tls")]
use std::path::Path;
use std::sync::Arc;

use hyper::http;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use lazy_static::lazy_static;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::endpoint::Handler;
use crate::error::Error;
use crate::middleware::{Middleware, WithState};
use crate::{register_method, HyperRequest, HyperResponse};
use crate::request::ReqCtx;
use crate::router::Router;
use crate::{
    endpoint::{Endpoint, RouterEndpoint},
    Response,
};

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

    pub fn merge(
        &mut self,
        prefix: impl AsRef<str>,
        router: Router,
    ) -> Result<(), crate::error::Error> {
        self.router.merge(prefix, router)
    }

    pub fn register<H, T>(&mut self, method: http::Method, path: impl AsRef<str>, handler: H)
    where
        H: Handler<T> + Send + Sync + 'static,
        T: 'static,
    {
        self.router.register(method, path, handler)
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

    pub fn middleware(&mut self, m: impl Middleware) -> &mut Self {
        self.router.middleware(m);
        self
    }

    pub fn handle_not_found<H, T>(&mut self, handler: H) -> &mut Self 
        where
            H: Handler<T> + Send + Sync + 'static,
            T: 'static,
    {
        self.router.set_not_found_handler(handler);
        self
    }

    pub async fn respond(self, req: HyperRequest) -> HyperResponse {
        let req = req.into();
        let App { router } = self;

        let router = Arc::new(router);

        let endpoint = RouterEndpoint::new(router);
        endpoint.call(req).await
    }

    pub async fn run(self, addr: impl ToSocketAddrs) -> Result<(), Error> {
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
                    service_fn(|mut req| {
                        let router = router.clone();
                        ReqCtx::init(&mut req, Some(remote_addr));

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
        addr: impl ToSocketAddrs,
        cert: impl AsRef<Path>,
        key: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let App { router } = self;

        let router = Arc::new(router.finalize());

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
                                let req = ReqCtx::new(req, Some(remote_addr));

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

#[cfg(test)]
mod test {
    use crate::{App, HyperRequest, Router};

    // fn app() -> App {
    //     let mut app = App::new();

    //     app.get("/", |_req| async move { "/" });
    //     app.post("/post", |_req| async move { "/post" });

    //     app
    // }

    fn request(method: &str, uri: &str) -> HyperRequest {
        hyper::Request::builder()
            .uri(uri)
            .method(method)
            .body(crate::hyper::Body::empty())
            .unwrap()
    }

    // #[tokio::test]
    // async fn basic() {
    //     let mut resp = app().respond(request("GET", "/")).await;
    //     assert_eq!(resp.body_bytes().await.unwrap(), b"/".to_vec())
    // }

    // #[tokio::test]
    // async fn basic_post() {
    //     let mut resp = app().respond(request("POST", "/post")).await;
    //     assert_eq!(resp.body_bytes().await.unwrap(), b"/post".to_vec())
    // }

    // #[tokio::test]
    // async fn tree() {
    //     let mut app = app();

    //     let mut router_c = Router::new();
    //     router_c.get("/c", |_| async move { "a-b-c" });

    //     let mut router_b = Router::new();
    //     router_b.merge("/b/", router_c).unwrap();

    //     app.merge("/a/", router_b).unwrap();

    //     let mut resp = app.respond(request("GET", "/a/b/c")).await;
    //     assert_eq!(resp.status(), 200);

    //     let body = resp.body_bytes().await.unwrap();

    //     assert_eq!(body, b"a-b-c".to_vec());
    // }
}
