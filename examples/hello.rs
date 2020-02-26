use std::sync::Arc;

use lieweb::{http, middleware, App, IntoResponse, Request};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const DEFAULT_ADDR: &str = "127.0.0.1:5000";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HelloMessage {
    message: String,
}

type State = Arc<Mutex<u64>>;

async fn request_handler(req: Request<State>) -> impl IntoResponse {
    let value;

    {
        let mut counter = req.state().lock().await;
        value = *counter;
        *counter += 1;
    }

    lieweb::html(format!(
        "got request#{} from {:?}",
        value,
        req.remote_addr()
    ))
}

async fn not_found(req: Request<State>) -> impl IntoResponse {
    println!("handler not found for {}", req.uri().path());
    http::StatusCode::NOT_FOUND
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut addr = DEFAULT_ADDR.to_string();

    let mut args = std::env::args();
    if args.len() > 2 {
        addr = args.nth(2).unwrap();
    }

    let addr = addr.parse().unwrap();

    let state: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

    let mut app = App::with_state(state);

    let mut default_headers = middleware::DefaultHeaders::new();
    default_headers.header(http::header::SERVER, lieweb::server_id());

    app.middleware(middleware::RequestLogger);
    app.middleware(default_headers);

    app.register(http::Method::GET, "/", request_handler);

    app.register(http::Method::GET, "/hello", |_req| async move {
        "hello, world!"
    });

    app.register(http::Method::GET, "/json", |_req| async move {
        let msg = HelloMessage {
            message: "hello, world!".to_owned(),
        };
        lieweb::response::json(&msg)
    });

    app.register(
        http::Method::GET,
        "/posts/:id/edit",
        |req: Request<State>| async move {
            req.params()
                .find("id")
                .unwrap()
                .parse()
                .map(|id: i32| format!("you are editing post<{}>", id))
        },
    );

    app.set_not_found(not_found);

    app.run(&addr).await.unwrap();
}
