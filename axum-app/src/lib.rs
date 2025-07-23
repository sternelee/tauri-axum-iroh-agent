mod routes;
mod todo;
mod iroh_routes;

use axum::{
    routing::{get, post},
    Router,
};
use routes::{create_todo, delete_todo, list_todos, toggle_todo};
use iroh_routes::{create_iroh_routes, IrohAppState};
use std::sync::{Arc, Mutex};
use todo::Todo;
use tracing::{error, info};

#[derive(Debug, Clone, Default)]
pub struct AppState {
    todos: Arc<Mutex<Vec<Todo>>>,
}

pub fn create_axum_app() -> Router {
    let state = AppState {
        todos: Arc::new(Mutex::new(Vec::new())),
    };

    Router::new()
        .route("/", get(list_todos))
        .route("/todo", post(create_todo))
        .route("/todo/{:id}/delete", post(delete_todo))
        .route("/todo/{:id}/toggle", post(toggle_todo))
        .with_state(state)
}

/// 创建带iroh功能的axum应用
pub async fn create_axum_app_with_iroh() -> Result<Router, String> {
    // 初始化iroh状态
    let iroh_state = IrohAppState::new().await?;
    
    // 创建基础todo应用
    let todo_state = AppState {
        todos: Arc::new(Mutex::new(Vec::new())),
    };

    let todo_routes = Router::new()
        .route("/", get(list_todos))
        .route("/todo", post(create_todo))
        .route("/todo/{:id}/delete", post(delete_todo))
        .route("/todo/{:id}/toggle", post(toggle_todo))
        .with_state(todo_state);

    // 创建iroh路由
    let iroh_routes = create_iroh_routes().with_state(iroh_state);

    // 合并路由
    let app = Router::new()
        .merge(todo_routes)
        .merge(iroh_routes);

    info!("axum应用（含iroh功能）创建成功");
    Ok(app)
}
