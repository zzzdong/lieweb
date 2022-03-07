use lieweb::{
    request::{LieRequest, RequestParts},
    App,
};

#[tokio::main]
async fn main() {
    let mut app = App::new();

    // GET / => 200 OK with body "Hello, world!"
    app.get("/", || async move { "Hello, world!" });

    // GET /hello/lieweb => 200 OK with body "Hello, lieweb!"
    app.get("/hello/:name", |req: RequestParts| async move {
        let name = req.get_param::<String>("name");

        format!("Hello, {:?}!", name)
    });

    app.run("127.0.0.1:5000").await.unwrap();
}
