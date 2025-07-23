//! 进度回调和事件系统

use serde::{Deserialize, Serialize};
use std::fmt;

/// 传输进度事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransferEvent {
    /// 下载队列添加文件
    DownloadQueueAppend {
        id: String,
        size: u64,
        name: String,
    },
    /// 下载进度更新
    DownloadProgress {
        id: String,
        offset: u64,
    },
    /// 下载完成
    DownloadDone {
        id: String,
    },
    /// 上传队列添加文件
    UploadQueueAppend {
        id: String,
        size: u64,
        title: String,
    },
    /// 上传进度更新
    UploadProgress {
        id: String,
        offset: u64,
    },
    /// 上传完成
    UploadDone {
        id: String,
    },
    /// 传输错误
    TransferError {
        id: String,
        error: String,
    },
}

impl fmt::Display for TransferEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransferEvent::DownloadQueueAppend { id, size, name } => {
                write!(f, "下载队列添加: {} ({}字节) - {}", name, size, id)
            }
            TransferEvent::DownloadProgress { id, offset } => {
                write!(f, "下载进度: {} - {}字节", id, offset)
            }
            TransferEvent::DownloadDone { id } => {
                write!(f, "下载完成: {}", id)
            }
            TransferEvent::UploadQueueAppend { id, size, title } => {
                write!(f, "上传队列添加: {} ({}字节) - {}", title, size, id)
            }
            TransferEvent::UploadProgress { id, offset } => {
                write!(f, "上传进度: {} - {}字节", id, offset)
            }
            TransferEvent::UploadDone { id } => {
                write!(f, "上传完成: {}", id)
            }
            TransferEvent::TransferError { id, error } => {
                write!(f, "传输错误: {} - {}", id, error)
            }
        }
    }
}

/// 进度回调函数类型
pub type ProgressCallback = Box<dyn Fn(TransferEvent) + Send + Sync>;

/// 进度通知器trait
pub trait ProgressNotifier: Send + Sync {
    /// 发送进度事件
    fn notify(&self, event: TransferEvent);
}

/// 默认的进度通知器实现
pub struct DefaultProgressNotifier {
    callback: Option<ProgressCallback>,
}

impl DefaultProgressNotifier {
    pub fn new() -> Self {
        Self { callback: None }
    }

    pub fn with_callback(callback: ProgressCallback) -> Self {
        Self {
            callback: Some(callback),
        }
    }
}

impl Default for DefaultProgressNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressNotifier for DefaultProgressNotifier {
    fn notify(&self, event: TransferEvent) {
        if let Some(ref callback) = self.callback {
            callback(event);
        }
    }
}