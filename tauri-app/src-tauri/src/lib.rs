use tauri::Manager;
use tracing::{error, info, Level};

// `commands/mod.rs` 现在负责管理所有命令模块
mod commands;
use commands::{
    // 引入 agent 相关的命令和状态
    agent::{initialize_agent, send_agent_message, AgentState},
    // 保留现有的 default 和 iroh 命令
    default::{read, write},
    iroh::{append_file, get_blob, get_share_code, remove_file, setup_iroh_state},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // 管理新的 AgentState
        .manage(AgentState::new())
        .setup(|app| {
            let handle = app.handle().clone();
            
            // 异步初始化 iroh 状态 (保留现有功能)
            tauri::async_runtime::spawn(async move {
                info!("开始初始化 iroh 后端...");
                match setup_iroh_state(handle.clone()).await {
                    Ok(iroh_state) => {
                        handle.manage(iroh_state);
                        info!("iroh 后端初始化成功");
                    }
                    Err(err) => {
                        error!("iroh 后端初始化失败: {}", err);
                    }
                }
            });

            Ok(())
        })
        // 将新的 agent 命令添加到 invoke_handler
        .invoke_handler(tauri::generate_handler![
            // Default commands
            read,
            write,
            // Iroh commands
            get_share_code,
            get_blob,
            append_file,
            remove_file,
            // Agent commands
            initialize_agent,
            send_agent_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}