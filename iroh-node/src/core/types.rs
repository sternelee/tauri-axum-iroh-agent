//! 核心类型定义

use iroh::{
    docs::{AuthorId, DocTicket},
    client::Doc,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 文件传输配置
#[derive(Debug, Clone)]
pub struct TransferConfig {
    /// 数据存储根目录
    pub data_root: PathBuf,
    /// 下载目录
    pub download_dir: Option<PathBuf>,
    /// 是否启用详细日志
    pub verbose_logging: bool,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            data_root: std::env::temp_dir().join("iroh_data"),
            download_dir: dirs_next::download_dir().map(|d| d.join("quick_send")),
            verbose_logging: false,
        }
    }
}

/// 文件下载请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadRequest {
    /// 文档票据字符串
    pub doc_ticket: String,
    /// 可选的自定义下载目录
    pub download_dir: Option<PathBuf>,
}

/// 文件上传请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadRequest {
    /// 要上传的文件路径
    pub file_path: PathBuf,
}

/// 文件删除请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoveRequest {
    /// 要删除的文件路径
    pub file_path: PathBuf,
}

/// 分享代码响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareResponse {
    /// 文档票据字符串
    pub doc_ticket: String,
}

/// 文件信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件ID
    pub id: String,
    /// 文件名
    pub name: String,
    /// 文件大小
    pub size: u64,
    /// 文件路径
    pub path: PathBuf,
}

/// iroh客户端状态
#[derive(Clone)]
pub struct IrohState {
    /// 当前作者ID
    pub author: AuthorId,
    /// 当前文档
    pub doc: Doc,
}

impl IrohState {
    pub fn new(author: AuthorId, doc: Doc) -> Self {
        Self { author, doc }
    }
}