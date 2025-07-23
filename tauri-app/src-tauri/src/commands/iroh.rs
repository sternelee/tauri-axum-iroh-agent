//! iroh P2P文件传输命令

use iroh_node::{
    adapters::tauri_adapter::{
        AppendFileRequest, GetBlobRequest, GetShareCodeResponse, RemoveFileRequest,
        TauriAdapter, TauriEventEmitter,
    },
    ConfigBuilder, DownloadRequest, RemoveRequest, UploadRequest,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Manager, Runtime, State};
use tracing::{error, info};

/// Tauri应用状态
pub struct IrohAppState {
    adapter: Arc<TauriAdapter<AppEventEmitter>>,
}

impl IrohAppState {
    pub async fn new<R: Runtime>(handle: AppHandle<R>) -> Result<Self, String> {
        // 获取应用数据目录
        let data_root = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法获取应用数据目录: {}", e))?
            .join("iroh_data");

        let config = ConfigBuilder::new()
            .data_root(data_root)
            .download_dir(dirs_next::download_dir().map(|d| d.join("quick_send")))
            .verbose_logging(true)
            .build();

        let emitter = Arc::new(AppEventEmitter::new(handle));
        let adapter = Arc::new(
            TauriAdapter::new(config, emitter)
                .await
                .map_err(|e| format!("创建iroh适配器失败: {}", e))?,
        );

        info!("iroh适配器初始化成功");

        Ok(Self { adapter })
    }

    pub fn adapter(&self) -> &TauriAdapter<AppEventEmitter> {
        &self.adapter
    }
}

/// Tauri事件发射器实现
pub struct AppEventEmitter {
    handle: AppHandle,
}

impl AppEventEmitter {
    pub fn new<R: Runtime>(handle: AppHandle<R>) -> AppEventEmitter {
        AppEventEmitter {
            handle: handle.clone(),
        }
    }
}

impl TauriEventEmitter for AppEventEmitter {
    fn emit_event(&self, event_name: &str, payload: serde_json::Value) {
        if let Err(e) = self.handle.emit_all(event_name, payload) {
            error!("发送事件失败 {}: {}", event_name, e);
        }
    }
}

/// 获取分享代码
#[tauri::command]
pub async fn get_share_code(
    state: State<'_, IrohAppState>,
) -> Result<GetShareCodeResponse, String> {
    let response = state
        .adapter()
        .get_share_code()
        .await
        .map_err(|e| e.to_string())?;

    Ok(GetShareCodeResponse {
        doc_ticket: response.doc_ticket,
    })
}

/// 下载文件
#[tauri::command]
pub async fn get_blob(
    state: State<'_, IrohAppState>,
    request: GetBlobRequest,
) -> Result<String, String> {
    let download_request = DownloadRequest {
        doc_ticket: request.blob_ticket,
        download_dir: None,
    };

    state
        .adapter()
        .download_files(download_request)
        .await
        .map_err(|e| e.to_string())
}

/// 上传文件
#[tauri::command]
pub async fn append_file(
    state: State<'_, IrohAppState>,
    request: AppendFileRequest,
) -> Result<(), String> {
    let upload_request = UploadRequest {
        file_path: PathBuf::from(request.file_path),
    };

    state
        .adapter()
        .upload_file(upload_request)
        .await
        .map_err(|e| e.to_string())
}

/// 删除文件
#[tauri::command]
pub async fn remove_file(
    state: State<'_, IrohAppState>,
    request: RemoveFileRequest,
) -> Result<(), String> {
    let remove_request = RemoveRequest {
        file_path: PathBuf::from(request.file_path),
    };

    state
        .adapter()
        .remove_file(remove_request)
        .await
        .map_err(|e| e.to_string())
}

/// 初始化iroh状态的辅助函数
pub async fn setup_iroh_state<R: Runtime>(handle: AppHandle<R>) -> Result<IrohAppState, String> {
    IrohAppState::new(handle).await
}