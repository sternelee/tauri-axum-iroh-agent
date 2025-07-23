use axum_app::{create_axum_app, create_axum_app_with_iroh, create_full_featured_app};
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("启动axum服务器（含iroh P2P文件传输和聊天功能）");

    // 创建完整功能的应用
    let app = match create_full_featured_app().await {
        Ok(app) => {
            info!("完整功能axum应用初始化成功");
            app
        }
        Err(e) => {
            eprintln!("初始化完整功能失败: {}, 尝试仅iroh功能", e);
            match create_axum_app_with_iroh().await {
                Ok(app) => {
                    info!("iroh功能应用初始化成功");
                    app
                }
                Err(e2) => {
                    eprintln!("初始化iroh功能也失败: {}, 使用基础功能", e2);
                    create_axum_app()
                }
            }
        }
    };

    info!("服务器运行在 http://localhost:3000");
    info!("");
    info!("=== API端点列表 ===");
    info!("聊天功能:");
    info!("  POST /api/chat/rooms - 创建聊天室");
    info!("  GET  /api/chat/rooms - 获取聊天室列表");
    info!("  POST /api/chat/rooms/join - 加入聊天室");
    info!("  POST /api/chat/rooms/leave - 离开聊天室");
    info!("  POST /api/chat/messages - 发送消息");
    info!("  GET  /api/chat/messages/:room_id - 获取消息历史");
    info!("  POST /api/chat/session - 创建聊天事件会话");
    info!("  GET  /api/chat/events/:session_id - 聊天事件流 (SSE)");
    info!("");
    info!("文件传输功能:");
    info!("  GET  /api/iroh/share - 获取分享代码");
    info!("  POST /api/iroh/download - 下载文件");
    info!("  POST /api/iroh/upload - 上传文件");
    info!("  POST /api/iroh/remove - 删除文件");
    info!("  POST /api/iroh/session - 创建进度会话");
    info!("  GET  /api/iroh/progress/:session_id - 进度事件流 (SSE)");
    info!("");
    info!("基础功能:");
    info!("  GET  / - Todo列表");
    info!("  POST /todo - 创建Todo");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
