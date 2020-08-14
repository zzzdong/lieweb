use std::sync::Arc;

use lieweb::{http, middleware, App, Error, Request, Response};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const DEFAULT_ADDR: &str = "127.0.0.1:5000";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HelloMessage {
    message: String,
}

type State = Arc<Mutex<u64>>;

async fn request_handler(req: Request) -> Response {
    let value;

    let state = req.get_state::<State>().unwrap();

    {
        let mut counter = state.lock().await;
        value = *counter;
        *counter += 1;
    }

    Response::with_html(format!(
        "got request#{} from {:?}",
        value,
        req.remote_addr()
    ))
}

async fn not_found(req: Request) -> Response {
    println!("handler not found for {}", req.uri().path());
    Response::with_status(http::StatusCode::NOT_FOUND)
}

async fn handle_form_urlencoded(mut req: Request) -> Result<Response, Error> {
    let form: serde_json::Value = req.read_form().await?;

    println!("form=> {:?}", form);

    Ok(Response::with_json(&form))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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

    app.middleware(middleware::RequestId);
    app.middleware(middleware::AccessLog);
    app.middleware(default_headers);

    app.register(http::Method::GET, "/", request_handler);

    app.get("/hello", |_req| async move { "hello, world!" });

    app.get("/json", |_req| async move {
        let msg = HelloMessage {
            message: "hello, world!".to_owned(),
        };
        Response::with_json(&msg)
    });

    app.post("/form-urlencoded", handle_form_urlencoded);

    app.post("/posts/:id/edit", |req: Request| async move {
        let id: u32 = req.get_param("id").unwrap();
        format!("you are editing post<{}>", id)
    });

    app.handle_not_found(not_found);

    app.run(&addr).await.unwrap();
}
