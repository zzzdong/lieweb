use std::sync::{Arc, Mutex};

use lieweb::{App, Request, Response};

const DEFAULT_ADDR: &'static str = "127.0.0.1:5000";

type State = Arc<Mutex<u64>>;

async fn request_handler(req: Request<State>) -> Result<Response, std::io::Error> {
    let value;

    {
        let mut counter = req.state().lock().unwrap();
        value = *counter;
        *counter += 1;
    }

    let resp = Response::with_html(format!(
        "got request#{} from {:?}",
        value,
        req.remote_addr()
    ));

    Ok(resp.into())
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

    app.register("/", request_handler);

    app.register("/hello", |_req| {
        async move { Response::with_html("hello, world!") }
    });

    app.run(&addr).await.unwrap();
}
