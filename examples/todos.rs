use std::sync::Arc;

use lieweb::{http, middleware, App};
use tokio::sync::Mutex;

const DEFAULT_ADDR: &str = "127.0.0.1:5000";

use models::Todo;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub db: Vec<Todo>,
}

impl AppState {
    pub fn new() -> Self {
        Default::default()
    }
}

pub type State = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let mut addr = DEFAULT_ADDR.to_string();

    let mut args = std::env::args();
    if args.len() > 2 {
        addr = args.nth(2).unwrap();
    }

    let state = Arc::new(Mutex::new(AppState::new()));

    let mut app = App::with_state(state);

    app.get("/todos", handlers::list_todos);
    app.post("/todos", handlers::create_todo);
    app.post("/todos/:id", handlers::update_todo);
    app.delete("/todos/:id", handlers::delete_todo);

    let mut default_headers = middleware::DefaultHeaders::new();
    default_headers.header(http::header::SERVER, lieweb::server_id());
    app.middleware(middleware::AccessLog);
    app.middleware(default_headers);

    app.run(&addr).await.unwrap();
}

mod models {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct Todo {
        pub id: u64,
        pub text: String,
        pub completed: bool,
    }

    // The query parameters for list_todos.
    #[derive(Debug, Deserialize, Default, Clone)]
    pub struct ListOptions {
        pub offset: Option<usize>,
        pub limit: Option<usize>,
    }
}

mod handlers {
    use super::models::*;
    use super::State;
    use lieweb::{http::StatusCode, RequestCtx, Response};

    pub async fn list_todos(req: RequestCtx) -> Result<Response, lieweb::Error> {
        let opts: ListOptions = req.get_query()?;

        let state: &State = req.get_state()?;
        let state = state.lock().await;

        let todos: Vec<Todo> = state
            .db
            .clone()
            .into_iter()
            .skip(opts.offset.unwrap_or(0))
            .take(opts.limit.unwrap_or(std::usize::MAX))
            .collect();

        Ok(Response::with_json(&todos))
    }

    pub async fn create_todo(mut req: RequestCtx) -> Result<Response, lieweb::Error> {
        let create: Todo = req.read_json().await?;

        let state: &State = req.get_state()?;
        let mut state = state.lock().await;

        for todo in state.db.iter() {
            if todo.id == create.id {
                tracing::debug!("    -> id already exists: {}", create.id);
                // Todo with id already exists, return `400 BadRequest`.
                return Ok(Response::with_status(StatusCode::BAD_REQUEST));
            }
        }

        state.db.push(create);

        Ok(Response::with_status(StatusCode::CREATED))
    }

    pub async fn update_todo(mut req: RequestCtx) -> Result<Response, lieweb::Error> {
        let todo_id: u64 = req.get_param("id")?;

        let update: Todo = req.read_json().await?;

        let state: &State = req.get_state()?;
        let mut state = state.lock().await;

        for todo in state.db.iter_mut() {
            if todo.id == todo_id {
                *todo = update;
                return Ok(Response::with_status(StatusCode::OK));
            }
        }

        tracing::debug!("-> todo id not found!");

        Ok(Response::with_status(StatusCode::NOT_FOUND))
    }

    pub async fn delete_todo(req: RequestCtx) -> Result<Response, lieweb::Error> {
        let todo_id: u64 = req.get_param("id")?;

        let state: &State = req.get_state()?;
        let mut state = state.lock().await;

        let len = state.db.len();
        state.db.retain(|todo| todo.id != todo_id);

        if len != state.db.len() {
            Ok(Response::with_status(StatusCode::NO_CONTENT))
        } else {
            tracing::debug!("    -> todo id not found!");
            Ok(Response::with_status(StatusCode::NOT_FOUND))
        }
    }
}
