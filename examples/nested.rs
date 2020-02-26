use std::sync::{Arc, Mutex};

use lieweb::{http, App, IntoResponse, Request, Router};

const SERVER_ID: &'static str = "lieweb";
const DEFAULT_ADDR: &'static str = "127.0.0.1:5000";

type State = Arc<Mutex<u64>>;

async fn request_handler(req: Request<State>) -> impl IntoResponse {
    let value;

    {
        let mut counter = req.state().lock().unwrap();
        value = *counter;
        *counter += 1;
    }

    let resp = lieweb::response::html(format!(
        "got request#{} from {:?}",
        value,
        req.remote_addr()
    ));
    lieweb::response::with_header(resp, http::header::SERVER, SERVER_ID)
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

    app.middleware(lieweb::middleware::RequestLogger);

    app.register(http::Method::GET, "/", request_handler);

    app.register(http::Method::GET, "/a", |_req| async move { "/a" });

    app.attach("/posts/:id/", posts_router()).unwrap();

    app.attach("/v2/posts/", posts_router()).unwrap();

    app.set_not_found(not_found);

    app.run(&addr).await.unwrap();
}

fn posts_router() -> Router<State> {
    let mut posts = Router::new();

    posts.register(
        http::Method::GET,
        "/new",
        |req: Request<State>| async move { format!("on /posts/new, {}", req.path()) },
    );

    posts.register(
        http::Method::GET,
        "/edit",
        |req: Request<State>| async move { format!("on /posts/edit, {}", req.path()) },
    );

    posts.register(
        http::Method::GET,
        "/delete",
        |req: Request<State>| async move { format!("on /posts/delete, {}", req.path()) },
    );

    posts.set_not_found(|_req| async move {
        let resp = lieweb::html("posts handler Not Found");
        lieweb::with_status(resp, http::StatusCode::NOT_FOUND)
    });

    posts
}
