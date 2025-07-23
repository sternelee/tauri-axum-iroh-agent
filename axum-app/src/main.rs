use axum_app::{create_axum_app, create_axum_app_with_iroh};
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("启动axum服务器（含iroh P2P文件传输功能）");

    // 创建带iroh功能的应用
    let app = match create_axum_app_with_iroh().await {
        Ok(app) => {
            info!("axum应用初始化成功");
            app
        }
        Err(e) => {
            eprintln!("初始化iroh功能失败: {}, 使用基础功能", e);
            create_axum_app()
        }
    };

    info!("服务器运行在 http://localhost:3000");
    info!("iroh API端点:");
    info!("  GET  /api/iroh/share - 获取分享代码");
    info!("  POST /api/iroh/download - 下载文件");
    info!("  POST /api/iroh/upload - 上传文件");
    info!("  POST /api/iroh/remove - 删除文件");
    info!("  POST /api/iroh/session - 创建进度会话");
    info!("  GET  /api/iroh/progress/:session_id - 进度事件流");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
