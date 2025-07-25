//! 核心模块 - 临时禁用版本
//! 
//! 由于 iroh API 变化，暂时禁用复杂功能

// 暂时注释掉有问题的模块
// pub mod chat;
// pub mod chat_client;
// pub mod client;
// pub mod error;
// pub mod integrated_client;
// pub mod progress;
// pub mod types;

// 只保留基本的类型定义
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub room_name: String,
    pub user_name: String,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            room_name: "默认聊天室".to_string(),
            user_name: "匿名用户".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequest {
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub doc_ticket: String,
    pub download_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareResponse {
    pub doc_ticket: String,
}