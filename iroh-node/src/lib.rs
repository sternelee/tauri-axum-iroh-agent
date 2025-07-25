//! iroh P2P 聊天和文件传输模块
//!
//! 简化版本，专注于核心功能

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// 暂时注释掉复杂的模块，避免编译错误
// pub mod adapters;
// pub mod core;

// 重新导出基本类型
pub use anyhow::{Error as IrohError, Result as IrohResult};

/// 传输配置
#[derive(Debug, Clone)]
pub struct TransferConfig {
    pub data_root: std::path::PathBuf,
    pub download_dir: Option<std::path::PathBuf>,
    pub verbose_logging: bool,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            data_root: dirs_next::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("iroh-node"),
            download_dir: dirs_next::download_dir().map(|d| d.join("iroh-downloads")),
            verbose_logging: false,
        }
    }
}

/// 配置构建器
pub struct ConfigBuilder {
    config: TransferConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: TransferConfig::default(),
        }
    }

    pub fn data_root<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.config.data_root = path.into();
        self
    }

    pub fn download_dir<P: Into<std::path::PathBuf>>(mut self, path: Option<P>) -> Self {
        self.config.download_dir = path.map(|p| p.into());
        self
    }

    pub fn verbose_logging(mut self, enabled: bool) -> Self {
        self.config.verbose_logging = enabled;
        self
    }

    pub fn build(self) -> TransferConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// 基本的类型定义
pub mod types {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RemoveRequest {
        pub file_path: PathBuf,
    }
}

pub use types::*;