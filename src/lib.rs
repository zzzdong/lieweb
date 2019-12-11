use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{server::conn::AddrStream, Server};

pub mod media_types {
    pub const TEXT_HTML: &str = "text/html; charset=utf-8";
    pub const JSON: &str = "application/json; charset=utf-8";
}

#[derive(thiserror::Error, Debug)]
pub enum LieError {
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
}

pub type HyperRequest = hyper::Request<hyper::Body>;

#[derive(Debug)]
pub struct Request<State> {
    inner: HyperRequest,
    state: Arc<State>,
    remote_addr: SocketAddr,
}

impl<State> Request<State> {
    pub fn new(inner: HyperRequest, state: Arc<State>, remote_addr: SocketAddr) -> Self {
        Request {
            inner,
            state,
            remote_addr,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

pub type HyperResponse = hyper::Response<hyper::Body>;

pub trait IntoResponse<E>: Send + Sized
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Convert the value into a `Response`.
    fn into_response(self) -> Result<Response, E>;
}

pub struct Response {
    inner: HyperResponse,
}

impl Response {
    pub fn with_html(html: impl Into<String>) -> Response {
        Response {
            inner: hyper::Response::builder()
                .header(hyper::header::CONTENT_TYPE, media_types::TEXT_HTML)
                .body(hyper::Body::from(html.into()))
                .unwrap(),
        }
    }
}

impl From<Response> for HyperResponse {
    fn from(resp: Response) -> HyperResponse {
        let Response { inner, .. } = resp;
        inner
    }
}

impl From<HyperResponse> for Response {
    fn from(resp: HyperResponse) -> Response {
        Response { inner: resp }
    }
}

impl<E> IntoResponse<E> for Result<Response, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_response(self) -> Result<Response, E> {
        self
    }
}

impl IntoResponse<std::io::Error> for Response {
    fn into_response(self) -> Result<Response, std::io::Error> {
        Ok(self)
    }
}

pub(crate) type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;
pub(crate) type DynEndpoint<State, E> =
    dyn (Fn(Request<State>) -> BoxFuture<Response, E>) + 'static + Send + Sync;

pub trait Endpoint<State, E>: Send + Sync + 'static
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// The async result of `call`.
    type Fut: Future<Output = Result<Response, E>> + Send + 'static;

    /// Invoke the endpoint within the given context
    fn call(&self, cx: Request<State>) -> Self::Fut;
}

impl<State, E: Send + Sync + 'static, F: Send + Sync + 'static, Fut> Endpoint<State, E> for F
where
    E: std::error::Error,
    F: Fn(Request<State>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse<E>,
{
    type Fut = BoxFuture<Response, E>;
    fn call(&self, cx: Request<State>) -> Self::Fut {
        let fut = (self)(cx);
        Box::pin(async move { fut.await.into_response() })
    }
}

pub struct App<State, E> {
    state: State,
    handlers: HashMap<String, Box<DynEndpoint<State, E>>>,
}

impl<State: Send + Sync + 'static, E: Send + Sync + 'static> App<State, E>
where
    E: std::error::Error,
{
    pub fn with_state(state: State) -> App<State, E> {
        App {
            state,
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, path: impl ToString, ep: impl Endpoint<State, E>) {
        self.handlers
            .insert(path.to_string(), Box::new(move |cx| Box::pin(ep.call(cx))));
    }

    pub async fn run(self, addr: &SocketAddr) -> Result<(), crate::LieError> {
        let App { state, handlers } = self;

        let state = Arc::new(state);
        let handlers = Arc::new(handlers);

        let make_service = make_service_fn(move |socket: &AddrStream| {
            let state = state.clone();
            let remote_addr = socket.remote_addr();
            let handlers = handlers.clone();

            async move {
                // This is the `Service` that will handle the connection.
                // `service_fn` is a helper to convert a function that
                // returns a Response into a `Service`.
                Ok::<_, LieError>(service_fn(move |req| {
                    let path = req.uri().path().to_string();

                    let request = Request {
                        inner: req,
                        state: state.clone(),
                        remote_addr,
                    };

                    let handlers = handlers.clone();

                    async move {
                        let resp = match handlers.get(&path) {
                            Some(handler) => match handler(request).await {
                                Ok(ret) => ret,
                                Err(e) => Self::handle_error(e),
                            },
                            None => Self::handle_not_found(request),
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

    fn handle_not_found(_request: Request<State>) -> Response {
        Response {
            inner: hyper::Response::builder()
                .status(404)
                .body(hyper::Body::from("Not Found"))
                .unwrap(),
        }
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
