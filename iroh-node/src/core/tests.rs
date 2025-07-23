//! 核心模块单元测试

#[cfg(test)]
mod tests {
    use super::super::{
        error::IrohTransferError,
        progress::{DefaultProgressNotifier, TransferEvent},
        types::{TransferConfig, DownloadRequest, UploadRequest},
    };
    use std::path::PathBuf;

    #[test]
    fn test_transfer_config_default() {
        let config = TransferConfig::default();
        assert!(config.data_root.ends_with("iroh_data"));
        assert!(!config.verbose_logging);
    }

    #[test]
    fn test_download_request_creation() {
        let request = DownloadRequest {
            doc_ticket: "test_ticket".to_string(),
            download_dir: Some(PathBuf::from("/tmp/downloads")),
        };
        
        assert_eq!(request.doc_ticket, "test_ticket");
        assert!(request.download_dir.is_some());
    }

    #[test]
    fn test_upload_request_creation() {
        let request = UploadRequest {
            file_path: PathBuf::from("/tmp/test.txt"),
        };
        
        assert_eq!(request.file_path, PathBuf::from("/tmp/test.txt"));
    }

    #[test]
    fn test_error_creation() {
        let error = IrohTransferError::file_not_found("test.txt");
        assert!(error.to_string().contains("test.txt"));

        let error = IrohTransferError::duplicate_file_name("duplicate.txt");
        assert!(error.to_string().contains("duplicate.txt"));

        let error = IrohTransferError::config("配置错误");
        assert!(error.to_string().contains("配置错误"));
    }

    #[test]
    fn test_progress_notifier() {
        let notifier = DefaultProgressNotifier::new();
        let event = TransferEvent::DownloadQueueAppend {
            id: "test_id".to_string(),
            size: 1024,
            name: "test.txt".to_string(),
        };
        
        // 测试通知不会panic
        notifier.notify(event);
    }

    #[test]
    fn test_transfer_event_display() {
        let event = TransferEvent::DownloadProgress {
            id: "test_id".to_string(),
            offset: 512,
        };
        
        let display_str = format!("{}", event);
        assert!(display_str.contains("下载进度"));
        assert!(display_str.contains("test_id"));
        assert!(display_str.contains("512"));
    }
}