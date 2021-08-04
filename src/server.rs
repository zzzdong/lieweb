use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{future::Future, sync::Arc};

use hyper::http;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use lazy_static::lazy_static;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tower::Service;

use crate::error::Error;
use crate::middleware::{Middleware, WithState};
use crate::request::Request;
use crate::router::Router;
use crate::trace::TraceExecutor;
use crate::{
    endpoint::{Endpoint, RouterEndpoint},
    Response,
};
use crate::{HyperRequest, HyperResponse};
pub type ResponseFuture =
    Pin<Box<dyn Future<Output = Result<HyperResponse, crate::Error>> + Send + 'static>>;

lazy_static! {
    pub static ref SERVER_ID: String = format!("Lieweb {}", env!("CARGO_PKG_VERSION"));
}

type HttpServer = hyper::server::conn::Http<TraceExecutor>;

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

    pub fn register(&mut self, method: http::Method, path: impl AsRef<str>, ep: impl Endpoint) {
        self.router.register(method, path, ep)
    }

    pub fn options(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::OPTIONS, path, ep)
    }

    pub fn get(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::GET, path, ep)
    }

    pub fn head(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::HEAD, path, ep)
    }

    pub fn post(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::POST, path, ep)
    }

    pub fn put(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::PUT, path, ep)
    }

    pub fn delete(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::DELETE, path, ep)
    }

    pub fn trace(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::TRACE, path, ep)
    }

    pub fn connect(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::CONNECT, path, ep)
    }

    pub fn patch(&mut self, path: impl AsRef<str>, ep: impl Endpoint) {
        self.register(http::Method::PATCH, path, ep)
    }

    pub fn middleware(&mut self, m: impl Middleware) -> &mut Self {
        self.router.middleware(m);
        self
    }

    pub fn handle_not_found(&mut self, ep: impl Endpoint) -> &mut Self {
        self.router.set_not_found_handler(ep);
        self
    }

    pub async fn respond(self, req: impl Into<Request>) -> Response {
        let req = req.into();
        let App { router } = self;

        let router = Arc::new(router);

        let endpoint = RouterEndpoint::new(router);
        endpoint.call(req).await
    }

    pub fn into_service(
        self,
    ) -> impl Service<crate::HyperRequest, Response = crate::HyperResponse, Error = crate::Error> + Clone
    {
        let App { router } = self;

        let router = Arc::new(router.finalize());

        Server { router }
    }

    pub async fn run(self, addr: impl ToSocketAddrs) -> Result<(), Error> {
        let App { router } = self;

        let router = Arc::new(router);

        let server = Http::new();

        let listener = TcpListener::bind(addr).await.unwrap();

        loop {
            tokio::select! {
                conn = listener.accept() => {
                    match conn{
                        Ok((socket, remote_addr)) => {
                            let server = server.clone();
                            let router = router.clone();

                            tokio::spawn(async move {
                                let router = router.clone();

                                let ret = server.serve_connection(
                                    socket,
                                    service_fn(|req| {
                                        let router = router.clone();
                                        let req = Request::new(req, Some(remote_addr));

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
                        Err(e) => {
                            tracing::error!("tcp accept error: {:?}", e)
                        }
                    }
                },
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("signal received, starting shutdown");
                    drop(listener);
                    break;
                }
            }
        }

        Ok(())
    }

    // pub async fn run2(self, addr: impl ToSocketAddrs) -> Result<(), Error> {
    //     let http = Http::new().with_executor(TraceExecutor::new());

    //     let listener = TcpListener::bind(addr).await?;

    //     let http_svc = LieService{};

    //     let conn_svc = ServeHttp::new(http_svc, http, watch.clone());

    //     loop {
    //         tokio::select! {
    //             ret = listener.accept() => {
    //                 match ret {
    //                     Ok((stream, remote_addr)) => {
    //                         let mut conn_svc = conn_svc.clone();
    //                         let span = tracing::debug_span!("connection", %remote_addr);
    //                         let _enter = span.enter();
    //                         let fut = async move {
    //                             let ret = Service::call(&mut conn_svc, stream).await;
    //                             tracing::debug!(?ret, "handle connection done");
    //                         };
    //                         tokio::spawn(fut.in_current_span());
    //                     }
    //                     Err(e) => {
    //                         tracing::error!("accept failed, {:?}", e);
    //                     }
    //                 }
    //             }
    //             _shutdown = watch.clone().signaled() => {
    //                 tracing::info!("stoping accept");
    //                 break;
    //             }
    //         }
    //     }
    // }

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
                                let req = Request::new(req, Some(remote_addr));

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

#[derive(Debug, Clone)]
struct RemoteInfo {
    pub addr: Option<SocketAddr>,
}

impl RemoteInfo {
    pub fn new(addr: Option<SocketAddr>) -> Self {
        RemoteInfo { addr }
    }
}

#[derive(Clone)]
pub struct Server {
    router: Arc<Router>,
}

impl Service<crate::HyperRequest> for Server {
    type Response = crate::HyperResponse;
    type Error = crate::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, mut req: crate::HyperRequest) -> Self::Future {
        let info = req.extensions_mut().remove::<RemoteInfo>();

        let req = Request::new(req, info.and_then(|info| info.addr));

        let router = self.router.clone();

        let fut = async move {
            let endpoint = RouterEndpoint::new(router);
            let resp = endpoint.call(req).await;
            Ok::<_, Error>(resp.into())
        };

        Box::pin(fut)
    }
}

#[derive(Clone, Debug)]
pub struct ConnService<S> {
    inner: S,
    server: HttpServer,
    drain: drain::Watch,
}

impl<S> ConnService<S> {
    pub fn new(svc: S, server: HttpServer, drain: drain::Watch) -> Self {
        ConnService {
            inner: svc,
            server,
            drain,
        }
    }
}

impl<I, S> Service<I> for ConnService<S>
where
    I: AsyncRead + AsyncWrite + RemoteAddr + Send + Unpin + 'static,
    S: Service<crate::HyperRequest, Response = crate::HyperResponse, Error = crate::Error>
        + Clone
        + Unpin
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = ();
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, io: I) -> Self::Future {
        let Self {
            inner,
            server,
            drain,
        } = self.clone();

        Box::pin(async move {
            let remote_addr = io.remote_addr();
            let svc = AppendInfoService::new(inner, RemoteInfo::new(remote_addr));
            let mut conn = server.serve_connection(io, svc);
            tokio::select! {
                res = &mut conn => {
                    tracing::debug!(?res, "The client is shutting down the connection");
                    res?
                }
                shutdown = drain.signaled() => {
                    tracing::debug!("The process is shutting down the connection");
                    Pin::new(&mut conn).graceful_shutdown();
                    shutdown.release_after(conn).await?;
                }
            }
            Ok(())
        })
    }
}

pub trait RemoteAddr {
    fn remote_addr(&self) -> Option<SocketAddr>;
}

impl RemoteAddr for tokio::net::TcpStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        self.peer_addr().ok()
    }
}

impl RemoteAddr for tokio::net::UnixStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        None
    }
}

#[cfg(feature = "tls")]
impl<T: RemoteAddr> RemoteAddr for tokio_rustls::server::TlsStream<T> {
    fn peer_addr(&self) -> Option<SocketAddr> {
        self.get_ref().0.peer_addr().ok()
    }
}

#[derive(Clone, Debug)]
struct AppendInfoService<S, T> {
    inner: S,
    info: T,
}

impl<S, T> AppendInfoService<S, T> {
    pub fn new(inner: S, info: T) -> Self {
        AppendInfoService { inner, info }
    }
}

impl<S, T> Service<HyperRequest> for AppendInfoService<S, T>
where
    S: Service<HyperRequest, Response = HyperResponse, Error = crate::Error>
        + Clone
        + Unpin
        + Send
        + 'static,
    S::Future: Send + 'static,
    T: Clone + Send + Sync + 'static,
{
    type Response = HyperResponse;
    type Error = crate::Error;
    type Future = ResponseFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: HyperRequest) -> Self::Future {
        let AppendInfoService { mut inner, info } = self.clone();

        req.extensions_mut().insert(info);

        Box::pin(Service::call(&mut inner, req))
    }
}

#[cfg(test)]
mod test {
    use crate::{App, HyperRequest, Router};

    fn app() -> App {
        let mut app = App::new();

        app.get("/", |_req| async move { "/" });
        app.post("/post", |_req| async move { "/post" });

        app
    }

    fn request(method: &str, uri: &str) -> HyperRequest {
        hyper::Request::builder()
            .uri(uri)
            .method(method)
            .body(crate::hyper::Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn basic() {
        let mut resp = app().respond(request("GET", "/")).await;
        assert_eq!(resp.body_bytes().await.unwrap(), b"/".to_vec())
    }

    #[tokio::test]
    async fn basic_post() {
        let mut resp = app().respond(request("POST", "/post")).await;
        assert_eq!(resp.body_bytes().await.unwrap(), b"/post".to_vec())
    }

    #[tokio::test]
    async fn tree() {
        let mut app = app();

        let mut router_c = Router::new();
        router_c.get("/c", |_| async move { "a-b-c" });

        let mut router_b = Router::new();
        router_b.merge("/b/", router_c).unwrap();

        app.merge("/a/", router_b).unwrap();

        let mut resp = app.respond(request("GET", "/a/b/c")).await;
        assert_eq!(resp.status(), 200);

        let body = resp.body_bytes().await.unwrap();

        assert_eq!(body, b"a-b-c".to_vec());
    }
}
