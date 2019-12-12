use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future;
use hyper::service::{make_service_fn, service_fn};
use hyper::{server::conn::AddrStream, Server};
use lazy_static::lazy_static;

use crate::endpoint::Endpoint;
use crate::request::HyperRequest;
use crate::router::Router;
use crate::utils::BoxFuture;
use crate::LieError;
use crate::{Request, Response};

lazy_static! {
    pub static ref SERVER_ID: String = format!("Lieweb {}", env!("CARGO_PKG_VERSION"));
}

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

    pub async fn run2(self, addr: &SocketAddr) -> Result<(), crate::LieError> {
        let App { state, router } = self;

        let state = Arc::new(state);
        let router = Arc::new(router);

        let svc = Service {
            state,
            router,
            remote_addr: addr.clone(),
        };

        let server = Server::bind(&addr).serve(MakeSvc { inner: svc });
        println!("Listening on http://{}", addr);
        server.await?;

        Ok(())
    }
}

pub struct Service<State, E> {
    state: Arc<State>,
    router: Arc<Router<State, E>>,
    remote_addr: SocketAddr,
}

impl<State, E> Service<State, E>
where
    State: Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    fn handle_error(e: impl std::error::Error) -> Response {
        Response {
            inner: hyper::Response::builder()
                .status(500)
                .body(hyper::Body::from(format!("{:?}", e)))
                .unwrap(),
        }
    }
}

impl<State, E> Clone for Service<State, E>
where
    State: Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Service {
            state: self.state.clone(),
            router: self.router.clone(),
            remote_addr: self.remote_addr,
        }
    }
}

impl<State, E> hyper::service::Service<crate::request::HyperRequest> for Service<State, E>
where
    State: Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    type Response = crate::response::HyperResponse;
    type Error = hyper::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: HyperRequest) -> Self::Future {
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        let request = Request::new(req, self.state.clone(), self.remote_addr);

        let router = self.router.clone();

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

pub trait RemoteAddr {
    fn remote_addr(&self) -> SocketAddr;
}

impl RemoteAddr for &AddrStream {
    fn remote_addr(&self) -> SocketAddr {
        (*self).remote_addr()
    }
}

pub struct MakeSvc<State, E> {
    inner: Service<State, E>,
}

impl<T, State, E> hyper::service::Service<T> for MakeSvc<State, E>
where
    State: Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
    T: std::fmt::Debug + RemoteAddr,
{
    type Response = Service<State, E>;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, t: T) -> Self::Future {
        let mut svc = self.inner.clone();
        svc.remote_addr = t.remote_addr();
        future::ok(svc)
    }
}
