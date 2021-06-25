use std::sync::Arc;

use lieweb::{http, middleware, App, Error, Request, Response};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const DEFAULT_ADDR: &str = "127.0.0.1:5000";

struct CounterMiddleware;

#[lieweb::async_trait]
impl middleware::Middleware for CounterMiddleware {
    async fn handle<'a>(&'a self, ctx: Request, next: middleware::Next<'a>) -> Response {
        let counter = ctx.get_state::<State>().unwrap().counter.clone();

        let resp = next.run(ctx).await;

        {
            let mut counter = counter.lock().await;
            *counter += 1;
        }

        resp
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HelloMessage {
    message: String,
}

#[derive(Clone)]
struct State {
    counter: Arc<Mutex<u64>>,
    meta_data: Arc<Option<String>>,
}

async fn request_handler(req: Request) -> Response {
    let state = req.get_state::<State>().unwrap();

    Response::with_html(format!(
        "got request#{} from {:?}",
        state.counter.lock().await,
        req.remote_addr()
    ))
}

async fn not_found(req: Request) -> Response {
    let state = req.get_state::<State>().unwrap();

    let counter = { state.counter.lock().await };

    Response::with_string(format!(
        "{:?} - got request[{}]({:?}) from {:?}",
        state.meta_data,
        counter,
        req.uri(),
        req.remote_addr()
    ))
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

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        addr = args.get(1).unwrap().clone();
    }

    let state = State {
        counter: Arc::new(Mutex::new(0)),
        meta_data: Arc::new(args.get(2).cloned()),
    };

    println!("meta_data => {:?}", state.meta_data);

    let mut app = App::with_state(state);

    let mut default_headers = middleware::DefaultHeaders::new();
    default_headers.header(http::header::SERVER, lieweb::server_id());

    app.middleware(middleware::RequestId);
    app.middleware(middleware::AccessLog);
    app.middleware(default_headers);
    app.middleware(CounterMiddleware);

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

    app.get("/readme", |_req: Request| async move {
        Response::send_file("READMEx.md").await
    });

    app.handle_not_found(not_found);

    app.run(&addr).await.unwrap();
}
