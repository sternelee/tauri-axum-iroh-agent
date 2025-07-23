//! 独立运行适配器

use crate::core::{
    client::IrohClient,
    error::TransferResult,
    progress::{DefaultProgressNotifier, ProgressCallback, ProgressNotifier, TransferEvent},
    types::{DownloadRequest, RemoveRequest, ShareResponse, TransferConfig, UploadRequest},
};
use std::sync::Arc;

/// 独立适配器
pub struct StandaloneAdapter {
    client: Arc<IrohClient>,
}

impl StandaloneAdapter {
    /// 创建新的独立适配器
    pub async fn new(config: TransferConfig) -> TransferResult<Self> {
        let client = Arc::new(IrohClient::new(config).await?);
        Ok(Self { client })
    }

    /// 获取分享代码
    pub async fn get_share_code(&self) -> TransferResult<ShareResponse> {
        self.client.get_share_code().await
    }

    /// 下载文件（带回调）
    pub async fn download_files_with_callback(
        &self,
        request: DownloadRequest,
        callback: ProgressCallback,
    ) -> TransferResult<String> {
        let notifier = Arc::new(DefaultProgressNotifier::with_callback(callback));
        self.client.download_files(request, notifier).await
    }

    /// 下载文件（无回调）
    pub async fn download_files(&self, request: DownloadRequest) -> TransferResult<String> {
        let notifier = Arc::new(DefaultProgressNotifier::new());
        self.client.download_files(request, notifier).await
    }

    /// 上传文件（带回调）
    pub async fn upload_file_with_callback(
        &self,
        request: UploadRequest,
        callback: ProgressCallback,
    ) -> TransferResult<()> {
        let notifier = Arc::new(DefaultProgressNotifier::with_callback(callback));
        self.client.upload_file(request, notifier).await
    }

    /// 上传文件（无回调）
    pub async fn upload_file(&self, request: UploadRequest) -> TransferResult<()> {
        let notifier = Arc::new(DefaultProgressNotifier::new());
        self.client.upload_file(request, notifier).await
    }

    /// 删除文件
    pub async fn remove_file(&self, request: RemoveRequest) -> TransferResult<()> {
        self.client.remove_file(request).await
    }

    /// 获取底层客户端引用（高级用法）
    pub fn client(&self) -> &IrohClient {
        &self.client
    }
}

/// 简化的API函数，用于快速集成
pub mod simple_api {
    use super::*;
    use std::path::Path;

    /// 简单下载文件
    pub async fn download_file(
        doc_ticket: &str,
        download_dir: Option<&Path>,
        data_root: Option<&Path>,
    ) -> TransferResult<String> {
        let config = TransferConfig {
            data_root: data_root
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::temp_dir().join("iroh_data")),
            download_dir: download_dir.map(|p| p.to_path_buf()),
            verbose_logging: false,
        };

        let adapter = StandaloneAdapter::new(config).await?;
        let request = DownloadRequest {
            doc_ticket: doc_ticket.to_string(),
            download_dir: download_dir.map(|p| p.to_path_buf()),
        };

        adapter.download_files(request).await
    }

    /// 简单上传文件
    pub async fn upload_file(
        file_path: &Path,
        data_root: Option<&Path>,
    ) -> TransferResult<ShareResponse> {
        let config = TransferConfig {
            data_root: data_root
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::temp_dir().join("iroh_data")),
            download_dir: None,
            verbose_logging: false,
        };

        let adapter = StandaloneAdapter::new(config).await?;
        let request = UploadRequest {
            file_path: file_path.to_path_buf(),
        };

        adapter.upload_file(request).await?;
        adapter.get_share_code().await
    }

    /// 带进度回调的下载
    pub async fn download_file_with_progress<F>(
        doc_ticket: &str,
        download_dir: Option<&Path>,
        data_root: Option<&Path>,
        progress_callback: F,
    ) -> TransferResult<String>
    where
        F: Fn(TransferEvent) + Send + Sync + 'static,
    {
        let config = TransferConfig {
            data_root: data_root
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::temp_dir().join("iroh_data")),
            download_dir: download_dir.map(|p| p.to_path_buf()),
            verbose_logging: false,
        };

        let adapter = StandaloneAdapter::new(config).await?;
        let request = DownloadRequest {
            doc_ticket: doc_ticket.to_string(),
            download_dir: download_dir.map(|p| p.to_path_buf()),
        };

        let callback = Box::new(progress_callback);
        adapter
            .download_files_with_callback(request, callback)
            .await
    }

    /// 带进度回调的上传
    pub async fn upload_file_with_progress<F>(
        file_path: &Path,
        data_root: Option<&Path>,
        progress_callback: F,
    ) -> TransferResult<ShareResponse>
    where
        F: Fn(TransferEvent) + Send + Sync + 'static,
    {
        let config = TransferConfig {
            data_root: data_root
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::temp_dir().join("iroh_data")),
            download_dir: None,
            verbose_logging: false,
        };

        let adapter = StandaloneAdapter::new(config).await?;
        let request = UploadRequest {
            file_path: file_path.to_path_buf(),
        };

        let callback = Box::new(progress_callback);
        adapter.upload_file_with_callback(request, callback).await?;
        adapter.get_share_code().await
    }
}
