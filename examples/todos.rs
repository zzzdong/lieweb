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
    env_logger::init();

    let mut addr = DEFAULT_ADDR.to_string();

    let mut args = std::env::args();
    if args.len() > 2 {
        addr = args.nth(2).unwrap();
    }

    let addr = addr.parse().unwrap();

    let state = Arc::new(Mutex::new(AppState::new()));

    let mut app = App::with_state(state);

    app.get("/todos", handlers::list_todos);
    app.post("/todos", handlers::create_todo);
    app.post("/todos/:id", handlers::update_todo);
    app.delete("/todos/:id", handlers::delete_todo);

    let mut default_headers = middleware::DefaultHeaders::new();
    default_headers.header(http::header::SERVER, lieweb::server_id());
    app.middleware(middleware::RequestLogger);
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
    use lieweb::{http::StatusCode, IntoResponse, Request, Response};

    pub async fn list_todos(req: Request<State>) -> Result<Response, lieweb::Error> {
        let opts: ListOptions = req.read_query()?;

        let state = req.state().lock().await;

        let todos: Vec<Todo> = state
            .db
            .clone()
            .into_iter()
            .skip(opts.offset.unwrap_or(0))
            .take(opts.limit.unwrap_or(std::usize::MAX))
            .collect();

        Ok(lieweb::json(&todos).into_response())
    }

    pub async fn create_todo(mut req: Request<State>) -> Result<StatusCode, lieweb::Error> {
        let create: Todo = req.read_json().await?;

        let mut state = req.state().lock().await;

        for todo in state.db.iter() {
            if todo.id == create.id {
                log::debug!("    -> id already exists: {}", create.id);
                // Todo with id already exists, return `400 BadRequest`.
                return Ok(StatusCode::BAD_REQUEST);
            }
        }

        state.db.push(create);

        Ok(StatusCode::CREATED)
    }

    pub async fn update_todo(mut req: Request<State>) -> Result<StatusCode, lieweb::Error> {
        let todo_id: u64 = req.read_param("id")?;

        let update: Todo = req.read_json().await?;

        let mut state = req.state().lock().await;

        for todo in state.db.iter_mut() {
            if todo.id == todo_id {
                *todo = update;
                return Ok(StatusCode::OK);
            }
        }

        log::debug!("    -> todo id not found!");

        Ok(StatusCode::NOT_FOUND)
    }

    pub async fn delete_todo(req: Request<State>) -> Result<StatusCode, lieweb::Error> {
        let todo_id: u64 = req.read_param("id")?;

        let mut state = req.state().lock().await;

        let len = state.db.len();
        state.db.retain(|todo| todo.id != todo_id);

        if len != state.db.len() {
            Ok(StatusCode::NO_CONTENT)
        } else {
            log::debug!("    -> todo id not found!");
            Ok(StatusCode::NOT_FOUND)
        }
    }
}
