//! Axum Web框架适配器

use crate::core::{
    client::IrohClient,
    error::{IrohTransferError, TransferResult},
    progress::{ProgressNotifier, TransferEvent},
    types::{DownloadRequest, RemoveRequest, ShareResponse, TransferConfig, UploadRequest},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

/// Web进度事件
#[derive(Clone, Debug, Serialize)]
pub struct WebProgressEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

/// Web进度通知器
pub struct WebProgressNotifier {
    sender: broadcast::Sender<WebProgressEvent>,
}

impl WebProgressNotifier {
    pub fn new() -> (Self, broadcast::Receiver<WebProgressEvent>) {
        let (sender, receiver) = broadcast::channel(1000);
        (Self { sender }, receiver)
    }
}

impl Default for WebProgressNotifier {
    fn default() -> Self {
        let (notifier, _) = Self::new();
        notifier
    }
}

impl ProgressNotifier for WebProgressNotifier {
    fn notify(&self, event: TransferEvent) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let web_event = match event {
            TransferEvent::DownloadQueueAppend { id, size, name } => WebProgressEvent {
                event_type: "download_queue_append".to_string(),
                data: serde_json::json!({
                    "id": id,
                    "size": size,
                    "name": name
                }),
                timestamp,
            },
            TransferEvent::DownloadProgress { id, offset } => WebProgressEvent {
                event_type: "download_progress".to_string(),
                data: serde_json::json!({
                    "id": id,
                    "offset": offset
                }),
                timestamp,
            },
            TransferEvent::DownloadDone { id } => WebProgressEvent {
                event_type: "download_done".to_string(),
                data: serde_json::json!({
                    "id": id
                }),
                timestamp,
            },
            TransferEvent::UploadQueueAppend { id, size, title } => WebProgressEvent {
                event_type: "upload_queue_append".to_string(),
                data: serde_json::json!({
                    "id": id,
                    "size": size,
                    "title": title
                }),
                timestamp,
            },
            TransferEvent::UploadProgress { id, offset } => WebProgressEvent {
                event_type: "upload_progress".to_string(),
                data: serde_json::json!({
                    "id": id,
                    "offset": offset
                }),
                timestamp,
            },
            TransferEvent::UploadDone { id } => WebProgressEvent {
                event_type: "upload_done".to_string(),
                data: serde_json::json!({
                    "id": id
                }),
                timestamp,
            },
            TransferEvent::TransferError { id, error } => WebProgressEvent {
                event_type: "transfer_error".to_string(),
                data: serde_json::json!({
                    "id": id,
                    "error": error
                }),
                timestamp,
            },
        };

        let _ = self.sender.send(web_event);
    }
}

/// Axum适配器
pub struct AxumAdapter {
    client: Arc<IrohClient>,
    progress_notifiers: Arc<Mutex<HashMap<String, Arc<WebProgressNotifier>>>>,
}

impl AxumAdapter {
    /// 创建新的Axum适配器
    pub async fn new(config: TransferConfig) -> TransferResult<Self> {
        let client = Arc::new(IrohClient::new(config).await?);
        let progress_notifiers = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            client,
            progress_notifiers,
        })
    }

    /// 获取分享代码
    pub async fn get_share_code(&self) -> TransferResult<ShareResponse> {
        self.client.get_share_code().await
    }

    /// 下载文件（带进度通知）
    pub async fn download_files_with_progress(
        &self,
        request: DownloadRequest,
        session_id: String,
    ) -> TransferResult<(String, broadcast::Receiver<WebProgressEvent>)> {
        let (notifier, receiver) = WebProgressNotifier::new();
        let notifier = Arc::new(notifier);

        // 存储进度通知器
        {
            let mut notifiers = self.progress_notifiers.lock().unwrap();
            notifiers.insert(session_id, notifier.clone());
        }

        let result = self.client.download_files(request, notifier).await?;
        Ok((result, receiver))
    }

    /// 下载文件（无进度通知）
    pub async fn download_files(&self, request: DownloadRequest) -> TransferResult<String> {
        let (notifier, _) = WebProgressNotifier::new();
        let notifier = Arc::new(notifier);
        self.client.download_files(request, notifier).await
    }

    /// 上传文件（带进度通知）
    pub async fn upload_file_with_progress(
        &self,
        request: UploadRequest,
        session_id: String,
    ) -> TransferResult<broadcast::Receiver<WebProgressEvent>> {
        let (notifier, receiver) = WebProgressNotifier::new();
        let notifier = Arc::new(notifier);

        // 存储进度通知器
        {
            let mut notifiers = self.progress_notifiers.lock().unwrap();
            notifiers.insert(session_id, notifier.clone());
        }

        self.client.upload_file(request, notifier).await?;
        Ok(receiver)
    }

    /// 上传文件（无进度通知）
    pub async fn upload_file(&self, request: UploadRequest) -> TransferResult<()> {
        let (notifier, _) = WebProgressNotifier::new();
        let notifier = Arc::new(notifier);
        self.client.upload_file(request, notifier).await
    }

    /// 删除文件
    pub async fn remove_file(&self, request: RemoveRequest) -> TransferResult<()> {
        self.client.remove_file(request).await
    }

    /// 清理会话的进度通知器
    pub fn cleanup_session(&self, session_id: &str) {
        let mut notifiers = self.progress_notifiers.lock().unwrap();
        notifiers.remove(session_id);
    }
}

// Web API请求/响应类型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebDownloadRequest {
    pub doc_ticket: String,
    pub download_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebUploadRequest {
    pub file_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebRemoveRequest {
    pub file_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebShareResponse {
    pub doc_ticket: String,
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> WebApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// 类型转换实现
impl From<WebDownloadRequest> for DownloadRequest {
    fn from(req: WebDownloadRequest) -> Self {
        Self {
            doc_ticket: req.doc_ticket,
            download_dir: req.download_dir.map(PathBuf::from),
        }
    }
}

impl From<WebUploadRequest> for UploadRequest {
    fn from(req: WebUploadRequest) -> Self {
        Self {
            file_path: PathBuf::from(req.file_path),
        }
    }
}

impl From<WebRemoveRequest> for RemoveRequest {
    fn from(req: WebRemoveRequest) -> Self {
        Self {
            file_path: PathBuf::from(req.file_path),
        }
    }
}

impl From<ShareResponse> for WebShareResponse {
    fn from(resp: ShareResponse) -> Self {
        Self {
            doc_ticket: resp.doc_ticket,
            success: true,
            message: "分享代码生成成功".to_string(),
        }
    }
}

impl From<IrohTransferError> for WebApiResponse<()> {
    fn from(err: IrohTransferError) -> Self {
        Self::error(err.to_string())
    }
}
