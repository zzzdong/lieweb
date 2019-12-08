use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{server::conn::AddrStream, Server};

#[derive(thiserror::Error, Debug)]
pub enum LieError {
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Request<State> {
    inner: hyper::Request<hyper::Body>,
    state: Arc<State>,
    remote_addr: SocketAddr,
}

pub trait IntoResponse<E>: Send + Sized
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Convert the value into a `Response`.
    fn into_response(self) -> Result<Response, E>;
}

pub type Response = hyper::Response<hyper::Body>;

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

use std::future::Future;
use std::pin::Pin;
pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub(crate) type DynEndpoint<State, E> =
    dyn (Fn(Request<State>) -> BoxFuture<'static, Result<Response, E>>) + 'static + Send + Sync;

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
    type Fut = BoxFuture<'static, Result<Response, E>>;
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

                        Ok::<_, LieError>(resp)
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
        hyper::Response::builder()
            .status(404)
            .body(hyper::Body::from("Not Found"))
            .unwrap()
    }

    fn handle_error(e: impl std::error::Error) -> Response {
        hyper::Response::builder()
            .status(500)
            .body(hyper::Body::from(format!("{:?}", e)))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    type State = Arc<Mutex<u64>>;

    async fn request_handler(req: Request<State>) -> Result<Response, std::io::Error> {
        let value;

        {
            let mut counter = req.state.lock().unwrap();
            value = *counter;
            *counter += 1;
        }

        let resp = hyper::Response::new(hyper::Body::from(format!(
            "got request#{} from {:?}",
            value, req.remote_addr
        )));

        Ok(resp)
    }

    #[tokio::test]
    async fn hello() {
        let addr = "127.0.0.1:8000".parse().unwrap();

        let state: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

        let mut app = App::with_state(state);

        app.register("/", request_handler);

        app.register("/hello", |_req| {
            async move { hyper::Response::new(hyper::Body::from("hello, world!")) }
        });

        app.run(&addr).await.unwrap();
    }
}
