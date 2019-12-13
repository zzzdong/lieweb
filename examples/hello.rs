use std::sync::{Arc, Mutex};

use lieweb::{App, LieError, Request, Response};

const DEFAULT_ADDR: &'static str = "127.0.0.1:5000";

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HelloMessage {
    message: String,
}

type State = Arc<Mutex<u64>>;

async fn request_handler(req: Request<State>) -> Result<Response, LieError> {
    let value;

    {
        let mut counter = req.state().lock().unwrap();
        value = *counter;
        *counter += 1;
    }

    Response::with_html(format!(
        "got request#{} from {:?}",
        value,
        req.remote_addr()
    ))
}

#[tokio::main]
async fn main() {
    let mut addr = DEFAULT_ADDR.to_string();

    let mut args = std::env::args();
    if args.len() > 2 {
        addr = args.nth(2).unwrap();
    }

    let addr = addr.parse().unwrap();

    let state: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

    let mut app = App::with_state(state);

    app.register(http::Method::GET, "/", request_handler);

    app.register(http::Method::GET, "/hello", |_req| {
        async move { Response::with_html("hello, world!") }
    });

    app.register(http::Method::GET, "/json", |_req| {
        async move {
            let msg = HelloMessage {
                message: "hello, world!".to_owned(),
            };
            Response::with_json(msg)
        }
    });

    app.run2(&addr).await.unwrap();
}
