use lieweb::{App, LieResponse, Params, Request, Response};

#[tokio::main]
async fn main() {
    let mut app = App::new();

    // GET / => 200 OK with body "Hello, world!"
    app.get("/", || async move { "Hello, world!" });

    // GET /hello/lieweb => 200 OK with body "Hello, lieweb!"
    app.get("/hello/:name", |req: Params| async move {
        let name = req.get::<String>("name");

        format!("Hello, {:?}!", name)
    });

    app.run("127.0.0.1:5000").await.unwrap();
}
