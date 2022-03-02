use lieweb::{App, Request, LieResponse, Response};

#[tokio::main]
async fn main() {
    let mut app = App::new();

    // GET / => 200 OK with body "Hello, world!"
    app.get("/", hello);

    // GET /hello/lieweb => 200 OK with body "Hello, lieweb!"
    app.get("/hello/:name", |req: Request| async move {
        let name: String = req.get_param("name").unwrap_or_default();

        format!("Hello, {}!", name)
    });

    app.run("127.0.0.1:5000").await.unwrap();
}


fn hello(req: Request) -> Response {
    LieResponse::from("Hello, world!").into()
}