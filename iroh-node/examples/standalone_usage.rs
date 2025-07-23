//! 独立使用iroh传输模块的示例

use iroh_node::{
    simple_api, ConfigBuilder, StandaloneAdapter, TransferEvent, UploadRequest, DownloadRequest,
};
use std::path::Path;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("开始iroh P2P文件传输示例");

    // 示例1: 使用简单API上传文件
    let file_path = Path::new("test.txt");
    
    // 创建测试文件
    std::fs::write(file_path, "这是一个测试文件内容")?;
    
    info!("上传文件: {:?}", file_path);
    let share_response = simple_api::upload_file(file_path, None).await?;
    info!("文件上传成功，分享代码: {}", share_response.doc_ticket);

    // 示例2: 使用带进度回调的上传
    let progress_callback = |event: TransferEvent| {
        match event {
            TransferEvent::UploadQueueAppend { id, size, title } => {
                println!("开始上传: {} ({}字节) - {}", title, size, id);
            }
            TransferEvent::UploadProgress { id, offset } => {
                println!("上传进度: {} - {}字节", id, offset);
            }
            TransferEvent::UploadDone { id } => {
                println!("上传完成: {}", id);
            }
            _ => {}
        }
    };

    info!("带进度回调的文件上传");
    let share_response2 = simple_api::upload_file_with_progress(
        file_path,
        None,
        progress_callback,
    ).await?;
    info!("带进度的文件上传成功，分享代码: {}", share_response2.doc_ticket);

    // 示例3: 使用StandaloneAdapter进行更复杂的操作
    let config = ConfigBuilder::new()
        .data_root("/tmp/iroh_example")
        .download_dir(Some("/tmp/downloads"))
        .verbose_logging(true)
        .build();

    let adapter = StandaloneAdapter::new(config).await?;

    // 上传文件
    let upload_request = UploadRequest {
        file_path: file_path.to_path_buf(),
    };
    
    adapter.upload_file(upload_request).await?;
    let share_code = adapter.get_share_code().await?;
    info!("使用适配器上传成功，分享代码: {}", share_code.doc_ticket);

    // 下载文件
    let download_request = DownloadRequest {
        doc_ticket: share_code.doc_ticket,
        download_dir: Some(Path::new("/tmp/downloads").to_path_buf()),
    };

    let download_result = adapter.download_files(download_request).await?;
    info!("文件下载完成: {}", download_result);

    // 清理测试文件
    std::fs::remove_file(file_path)?;
    
    info!("示例执行完成");
    Ok(())
}