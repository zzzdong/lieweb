use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future;
use hyper::service::{make_service_fn, service_fn};
use hyper::{server::conn::AddrStream, Server};
use lazy_static::lazy_static;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::endpoint::Endpoint;
use crate::request::{HyperRequest, Request};
use crate::response::{HyperResponse, Response};
use crate::router::Router;
use crate::utils::{BoxFuture, StdError};
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

    pub fn register(
        &mut self,
        method: http::Method,
        path: impl ToString,
        ep: impl Endpoint<State>,
    ) {
        self.router.register(method, path, ep)
    }

    pub fn set_not_found(&mut self, ep: impl Endpoint<State>) {
        self.router.set_not_found(ep)
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
                    let path = req.uri().path().to_string();
                    let method = req.method().clone();

                    let request = Request::new(req, state.clone(), Some(remote_addr));

                    let router = router.clone();

                    async move {
                        let handler = router.find(&method, &path);
                        let resp = match handler(request).await {
                            Ok(ret) => ret,
                            Err(e) => Self::handle_error(e).await,
                        };

                        Ok::<_, Error>(resp.into())
                    }
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_service);
        println!("Listening on http://{}", addr);
        server.await?;

        Ok(())
    }

    pub async fn run2(self, addr: &SocketAddr) -> Result<(), Error> {
        let App { state, router } = self;

        let state = Arc::new(state);
        let router = Arc::new(router);

        let svc = Service {
            state,
            router,
            remote_addr: None,
        };

        let server = Server::bind(&addr).serve(MakeSvc { inner: svc });
        println!("Listening on http://{}", addr);
        server.await?;

        Ok(())
    }

    async fn handle_error(e: StdError) -> Response {
        Response {
            inner: hyper::Response::builder()
                .status(500)
                .body(hyper::Body::from(format!("{:?}", e)))
                .unwrap(),
        }
    }
}

pub struct Service<State> {
    state: Arc<State>,
    router: Arc<Router<State>>,
    remote_addr: Option<SocketAddr>,
}

impl<State> Service<State>
where
    State: Send + Sync + 'static,
{
    fn handle_error(e: StdError) -> Response {
        Response {
            inner: hyper::Response::builder()
                .status(500)
                .body(hyper::Body::from(format!("{:?}", e)))
                .unwrap(),
        }
    }
}

impl<State> Clone for Service<State>
where
    State: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Service {
            state: self.state.clone(),
            router: self.router.clone(),
            remote_addr: self.remote_addr,
        }
    }
}

impl<State> hyper::service::Service<HyperRequest> for Service<State>
where
    State: Send + Sync + 'static,
{
    type Response = HyperResponse;
    type Error = hyper::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: HyperRequest) -> Self::Future {
        let path = req.uri().path().to_string();
        let method = req.method().clone();
        let router = self.router.clone();

        let request = Request::new(req, self.state.clone(), self.remote_addr);

        let fut = async move {
            let handler = router.find(&method, &path);
            let resp = match handler(request).await {
                Ok(ret) => ret,
                Err(e) => Self::handle_error(e),
            };

            Ok::<_, hyper::Error>(resp.into())
        };

        Box::pin(fut)
    }
}

pub trait Transport: AsyncRead + AsyncWrite {
    fn remote_addr(&self) -> Option<SocketAddr>;
}

impl Transport for AddrStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr())
    }
}

pub struct MakeSvc<State> {
    inner: Service<State>,
}

impl<T, State> hyper::service::Service<&T> for MakeSvc<State>
where
    State: Send + Sync + 'static,
    T: std::fmt::Debug + Transport,
{
    type Response = Service<State>;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, t: &T) -> Self::Future {
        let mut svc = self.inner.clone();
        svc.remote_addr = Transport::remote_addr(t);
        future::ok(svc)
    }
}
