use std::sync::Arc;

use lieweb::{http, middleware, App, Request, Response, Router};
use tokio::sync::Mutex;

const DEFAULT_ADDR: &str = "127.0.0.1:5000";

type State = Arc<Mutex<u64>>;

async fn request_handler(req: Request) -> Response {
    let value;

    let state: &State = req.get_state().unwrap();

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

    // add some middleware
    let mut default_headers = middleware::DefaultHeaders::new();
    default_headers.header(http::header::SERVER, lieweb::server_id());

    app.middleware(default_headers);
    app.middleware(middleware::AccessLog);

    app.register(http::Method::GET, "/", request_handler);

    app.register(http::Method::GET, "/a", |_req| async move { "/a" });

    app.merge("/posts/:id/", posts_router()).unwrap();

    app.merge("/v2/posts/", posts_router()).unwrap();

    app.handle_not_found(not_found);

    app.run(&addr).await.unwrap();
}

fn posts_router() -> Router {
    let mut posts = Router::new();

    posts.register(http::Method::GET, "/new", |req: Request| async move {
        format!("on /posts/new, {}", req.path())
    });

    posts.register(http::Method::GET, "/edit", |req: Request| async move {
        format!("on /posts/edit, {}", req.path())
    });

    posts.register(http::Method::GET, "/delete", |req: Request| async move {
        format!("on /posts/delete, {}", req.path())
    });

    posts.set_not_found_handler(|_req| async move {
        let resp = Response::with_html("posts handler Not Found");
        resp.set_status(http::StatusCode::NOT_FOUND)
    });

    posts
}
