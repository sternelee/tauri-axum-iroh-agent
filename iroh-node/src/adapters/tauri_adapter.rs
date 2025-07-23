//! Tauri框架适配器

use crate::core::{
    client::IrohClient,
    error::{IrohTransferError, TransferResult},
    progress::{ProgressNotifier, TransferEvent},
    types::{DownloadRequest, RemoveRequest, ShareResponse, TransferConfig, UploadRequest},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

/// Tauri事件发射器
pub trait TauriEventEmitter: Send + Sync {
    /// 发射事件到前端
    fn emit_event(&self, event_name: &str, payload: serde_json::Value);
}

/// Tauri进度通知器
pub struct TauriProgressNotifier<T: TauriEventEmitter> {
    emitter: Arc<T>,
}

impl<T: TauriEventEmitter> TauriProgressNotifier<T> {
    pub fn new(emitter: Arc<T>) -> Self {
        Self { emitter }
    }
}

impl<T: TauriEventEmitter> ProgressNotifier for TauriProgressNotifier<T> {
    fn notify(&self, event: TransferEvent) {
        match event {
            TransferEvent::DownloadQueueAppend { id, size, name } => {
                let payload = serde_json::json!({
                    "id": id,
                    "size": size,
                    "name": name
                });
                self.emitter.emit_event("download-queue-append", payload);
            }
            TransferEvent::DownloadProgress { id, offset } => {
                let payload = serde_json::json!({
                    "id": id,
                    "offset": offset
                });
                self.emitter.emit_event("download-queue-progress", payload);
            }
            TransferEvent::DownloadDone { id } => {
                self.emitter.emit_event("download-queue-done", serde_json::json!(id));
            }
            TransferEvent::UploadQueueAppend { id, size, title } => {
                let payload = serde_json::json!({
                    "id": id,
                    "size": size,
                    "title": title
                });
                self.emitter.emit_event("upload-queue-append", payload);
            }
            TransferEvent::UploadProgress { id, offset } => {
                let payload = serde_json::json!({
                    "id": id,
                    "offset": offset
                });
                self.emitter.emit_event("upload-queue-progress", payload);
            }
            TransferEvent::UploadDone { id } => {
                self.emitter.emit_event("upload-queue-alldone", serde_json::json!(id));
            }
            TransferEvent::TransferError { id, error } => {
                let payload = serde_json::json!({
                    "id": id,
                    "error": error
                });
                self.emitter.emit_event("transfer-error", payload);
            }
        }
    }
}

/// Tauri适配器
pub struct TauriAdapter<T: TauriEventEmitter> {
    client: Arc<IrohClient>,
    emitter: Arc<T>,
}

impl<T: TauriEventEmitter> TauriAdapter<T> {
    /// 创建新的Tauri适配器
    pub async fn new(config: TransferConfig, emitter: Arc<T>) -> TransferResult<Self> {
        let client = Arc::new(IrohClient::new(config).await?);
        Ok(Self { client, emitter })
    }

    /// 获取分享代码
    pub async fn get_share_code(&self) -> TransferResult<ShareResponse> {
        self.client.get_share_code().await
    }

    /// 下载文件
    pub async fn download_files(&self, request: DownloadRequest) -> TransferResult<String> {
        let notifier = Arc::new(TauriProgressNotifier::new(self.emitter.clone()));
        self.client.download_files(request, notifier).await
    }

    /// 上传文件
    pub async fn upload_file(&self, request: UploadRequest) -> TransferResult<()> {
        let notifier = Arc::new(TauriProgressNotifier::new(self.emitter.clone()));
        self.client.upload_file(request, notifier).await
    }

    /// 删除文件
    pub async fn remove_file(&self, request: RemoveRequest) -> TransferResult<()> {
        self.client.remove_file(request).await
    }
}

// Tauri命令请求/响应类型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetBlobRequest {
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

/// 将通用类型转换为Tauri特定类型的辅助函数
impl From<GetBlobRequest> for DownloadRequest {
    fn from(req: GetBlobRequest) -> Self {
        Self {
            doc_ticket: req.blob_ticket,
            download_dir: None,
        }
    }
}

impl From<AppendFileRequest> for UploadRequest {
    fn from(req: AppendFileRequest) -> Self {
        Self {
            file_path: PathBuf::from(req.file_path),
        }
    }
}

impl From<RemoveFileRequest> for RemoveRequest {
    fn from(req: RemoveFileRequest) -> Self {
        Self {
            file_path: PathBuf::from(req.file_path),
        }
    }
}

impl From<ShareResponse> for GetShareCodeResponse {
    fn from(resp: ShareResponse) -> Self {
        Self {
            doc_ticket: resp.doc_ticket,
        }
    }
}