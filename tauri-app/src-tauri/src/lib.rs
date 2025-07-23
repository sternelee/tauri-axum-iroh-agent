use std::sync::Arc;

use axum::Router;
use axum_app::create_axum_app;
use tauri::{async_runtime::Mutex, State};
use tauri_axum_htmx::{LocalRequest, LocalResponse};
use tracing::{error, info, Level};

mod commands;
use commands::{
    default::{read, write},
    iroh::{append_file, get_blob, get_share_code, remove_file, setup_iroh_state, IrohAppState},
};

struct AppState {
    router: Arc<Mutex<Router>>,
}

#[tauri::command]
async fn local_app_request(
    state: State<'_, AppState>,
    local_request: LocalRequest,
) -> Result<LocalResponse, ()> {
    let mut router = state.router.lock().await;

    let response = local_request.send_to_router(&mut router).await;

    Ok(response)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let router: Router = create_axum_app();

    let app_state = AppState {
        router: Arc::new(Mutex::new(router)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();
            
            // 异步初始化iroh状态
            tauri::async_runtime::spawn(async move {
                info!("开始初始化iroh后端...");
                match setup_iroh_state(handle.clone()).await {
                    Ok(iroh_state) => {
                        handle.manage(iroh_state);
                        info!("iroh后端初始化成功");
                    }
                    Err(err) => {
                        error!("iroh后端初始化失败: {}", err);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            local_app_request,
            read,
            write,
            get_share_code,
            get_blob,
            append_file,
            remove_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
