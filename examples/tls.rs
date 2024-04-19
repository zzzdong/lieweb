// generate tls cert
// cd examples && openssl req -new -x509 -nodes -newkey rsa:4096 -keyout server.key -out server.crt -days 1095

use std::sync::Arc;

use lieweb::{http, middleware, request::RequestParts, App, AppState, LieResponse, RemoteAddr};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const DEFAULT_ADDR: &str = "127.0.0.1:5000";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HelloMessage {
    message: String,
}

type State = Arc<Mutex<u64>>;

async fn request_handler(addr: RemoteAddr, req: AppState<State>) -> LieResponse {
    let value;

    let state: &State = req.value();

    {
        let mut counter = state.lock().await;
        value = *counter;
        *counter += 1;
    }

    LieResponse::with_html(format!("got request#{} from {:?}", value, addr.value()))
}

async fn not_found(req: RequestParts) -> LieResponse {
    println!("handler not found for {}", req.uri().path());
    http::StatusCode::NOT_FOUND.into()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let mut addr = DEFAULT_ADDR.to_string();

    let mut args = std::env::args();
    if args.len() > 2 {
        addr = args.nth(2).unwrap();
    }

    let state: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

    let mut app = App::with_state(state);

    let mut default_headers = middleware::DefaultHeaders::new();
    default_headers.header(http::header::SERVER, lieweb::server_id());

    app.middleware(middleware::AccessLog);
    app.middleware(default_headers);

    app.register(http::Method::GET, "/", request_handler);

    app.get("/hello", || async move { "hello, world!" });

    app.get("/json", || async move {
        let msg = HelloMessage {
            message: "hello, world!".to_owned(),
        };
        LieResponse::with_json(msg)
    });

    app.handle_not_found(not_found);

    app.run_with_tls(&addr, "examples/server.crt", "examples/abc.key")
        .await
        .unwrap();
}
