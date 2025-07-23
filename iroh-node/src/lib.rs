//! iroh P2P文件传输通用模块
//! 
//! 提供跨平台的P2P文件传输功能，支持tauri、axum等不同运行环境

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod core;
pub mod adapters;

// 重新导出核心类型和功能
pub use core::{
    client::IrohClient,
    error::{IrohTransferError, TransferResult},
    progress::{DefaultProgressNotifier, ProgressCallback, ProgressNotifier, TransferEvent},
    types::{
        DownloadRequest, FileInfo, IrohState, RemoveRequest, ShareResponse, TransferConfig,
        UploadRequest,
    },
};

// 重新导出适配器
pub use adapters::{
    axum_adapter::{AxumAdapter, WebApiResponse, WebProgressEvent},
    standalone::{simple_api, StandaloneAdapter},
    tauri_adapter::TauriAdapter,
};

// 为了向后兼容，保留原有的tauri特定实现
#[cfg(feature = "tauri-compat")]
pub mod legacy {
    //! 向后兼容的tauri实现
    
    use crate::core::{
        client::IrohClient,
        error::TransferResult,
        progress::{ProgressNotifier, TransferEvent},
        types::{TransferConfig, DownloadRequest, UploadRequest, RemoveRequest},
    };
    use anyhow::Result;
    use serde::{Deserialize, Serialize};
    use std::{path::PathBuf, sync::Arc};
    use tracing::{error, info};

    type IrohNode = iroh::node::Node<iroh::blobs::store::fs::Store>;

    /// 兼容的AppState结构
    pub struct AppState {
        client: Arc<IrohClient>,
    }

    impl AppState {
        pub async fn new(data_root: PathBuf) -> TransferResult<Self> {
            let config = TransferConfig {
                data_root,
                download_dir: dirs_next::download_dir().map(|d| d.join("quick_send")),
                verbose_logging: true,
            };
            
            let client = Arc::new(IrohClient::new(config).await?);
            Ok(Self { client })
        }

        pub fn client(&self) -> &IrohClient {
            &self.client
        }
    }

    /// Tauri事件发射器实现
    pub struct TauriEventEmitter<R: tauri::Runtime> {
        handle: tauri::AppHandle<R>,
    }

    impl<R: tauri::Runtime> TauriEventEmitter<R> {
        pub fn new(handle: tauri::AppHandle<R>) -> Self {
            Self { handle }
        }
    }

    impl<R: tauri::Runtime> crate::adapters::tauri_adapter::TauriEventEmitter for TauriEventEmitter<R> {
        fn emit_event(&self, event_name: &str, payload: serde_json::Value) {
            let _ = self.handle.emit_all(event_name, payload);
        }
    }

    /// 兼容的tauri命令类型
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GetBlob {
        pub blob_ticket: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GetShareCodeResponse {
        pub doc_ticket: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AppendFileRequest {
        pub file_path: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct RemoveFileRequest {
        pub file_path: String,
    }

    /// 兼容的tauri命令实现
    pub async fn get_blob<R: tauri::Runtime>(
        state: tauri::State<'_, AppState>,
        get_blob_request: GetBlob,
        handle: tauri::AppHandle<R>,
    ) -> Result<String, String> {
        let emitter = Arc::new(TauriEventEmitter::new(handle));
        let adapter = crate::adapters::TauriAdapter::new(
            TransferConfig::default(),
            emitter,
        ).await.map_err(|e| e.to_string())?;

        let request = DownloadRequest {
            doc_ticket: get_blob_request.blob_ticket,
            download_dir: None,
        };

        adapter.download_files(request).await.map_err(|e| e.to_string())
    }

    pub async fn get_share_code(
        state: tauri::State<'_, AppState>,
    ) -> Result<GetShareCodeResponse, String> {
        let response = state.client().get_share_code().await.map_err(|e| e.to_string())?;
        Ok(GetShareCodeResponse {
            doc_ticket: response.doc_ticket,
        })
    }

    pub async fn append_file<R: tauri::Runtime>(
        state: tauri::State<'_, AppState>,
        append_file_request: AppendFileRequest,
        handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        let emitter = Arc::new(TauriEventEmitter::new(handle));
        let adapter = crate::adapters::TauriAdapter::new(
            TransferConfig::default(),
            emitter,
        ).await.map_err(|e| e.to_string())?;

        let request = UploadRequest {
            file_path: PathBuf::from(append_file_request.file_path),
        };

        adapter.upload_file(request).await.map_err(|e| e.to_string())
    }

    pub async fn remove_file(
        state: tauri::State<'_, AppState>,
        remove_file_request: RemoveFileRequest,
    ) -> Result<(), String> {
        let request = RemoveRequest {
            file_path: PathBuf::from(remove_file_request.file_path),
        };

        state.client().remove_file(request).await.map_err(|e| e.to_string())
    }
}

/// 便捷的初始化函数
pub async fn init_iroh_client(config: TransferConfig) -> TransferResult<IrohClient> {
    IrohClient::new(config).await
}

/// 便捷的配置构建器
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