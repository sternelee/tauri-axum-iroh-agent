//! iroh P2P聊天功能使用示例

use iroh_node::{
    ChatConfig, ChatEvent, CreateRoomRequest, IntegratedClientBuilder, JoinRoomRequest,
    MessageType, SendMessageRequest, TransferConfig,
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("开始iroh P2P聊天功能示例");

    // 创建两个用户的配置
    let user1_transfer_config = TransferConfig {
        data_root: std::env::temp_dir().join("iroh_chat_user1"),
        download_dir: Some(std::env::temp_dir().join("downloads_user1")),
        verbose_logging: true,
    };

    let user1_chat_config = ChatConfig {
        user_name: "小明".to_string(),
        max_message_history: 100,
        enable_file_sharing: true,
    };

    let user2_transfer_config = TransferConfig {
        data_root: std::env::temp_dir().join("iroh_chat_user2"),
        download_dir: Some(std::env::temp_dir().join("downloads_user2")),
        verbose_logging: true,
    };

    let user2_chat_config = ChatConfig {
        user_name: "小红".to_string(),
        max_message_history: 100,
        enable_file_sharing: true,
    };

    // 创建两个集成客户端
    info!("创建用户1客户端");
    let mut client1 = IntegratedClientBuilder::new()
        .transfer_config(user1_transfer_config)
        .chat_config(user1_chat_config)
        .enable_chat(true)
        .build()
        .await?;

    info!("创建用户2客户端");
    let mut client2 = IntegratedClientBuilder::new()
        .transfer_config(user2_transfer_config)
        .chat_config(user2_chat_config)
        .enable_chat(true)
        .build()
        .await?;

    // 用户1创建聊天室
    info!("用户1创建聊天室");
    let room = client1
        .create_chat_room(CreateRoomRequest {
            name: "测试聊天室".to_string(),
            description: Some("这是一个测试聊天室".to_string()),
        })
        .await?;

    info!("聊天室创建成功: {} (ID: {})", room.name, room.id);

    // 用户2加入聊天室
    info!("用户2加入聊天室");
    client2
        .join_chat_room(JoinRoomRequest {
            room_id: room.id.clone(),
            user_name: "小红".to_string(),
        })
        .await?;

    // 订阅聊天事件
    let mut events1 = client1.subscribe_chat_events()?;
    let mut events2 = client2.subscribe_chat_events()?;

    // 启动事件监听任务
    let room_id_clone = room.id.clone();
    tokio::spawn(async move {
        info!("用户1开始监听聊天事件");
        while let Ok(event) = events1.recv().await {
            match event {
                ChatEvent::MessageReceived(message) => {
                    info!("用户1收到消息: [{}] {}: {}", 
                        message.timestamp.format("%H:%M:%S"),
                        message.sender_name, 
                        message.content
                    );
                }
                ChatEvent::UserJoined(user) => {
                    info!("用户1看到: {} 加入了聊天室", user.name);
                }
                ChatEvent::UserLeft { user_name, .. } => {
                    info!("用户1看到: {} 离开了聊天室", user_name);
                }
                _ => {}
            }
        }
    });

    let room_id_clone2 = room.id.clone();
    tokio::spawn(async move {
        info!("用户2开始监听聊天事件");
        while let Ok(event) = events2.recv().await {
            match event {
                ChatEvent::MessageReceived(message) => {
                    info!("用户2收到消息: [{}] {}: {}", 
                        message.timestamp.format("%H:%M:%S"),
                        message.sender_name, 
                        message.content
                    );
                }
                ChatEvent::UserJoined(user) => {
                    info!("用户2看到: {} 加入了聊天室", user.name);
                }
                ChatEvent::UserLeft { user_name, .. } => {
                    info!("用户2看到: {} 离开了聊天室", user_name);
                }
                _ => {}
            }
        }
    });

    // 等待一下让连接建立
    sleep(Duration::from_secs(2)).await;

    // 用户1发送消息
    info!("用户1发送消息");
    client1
        .send_chat_message(SendMessageRequest {
            room_id: room.id.clone(),
            content: "大家好！我是小明".to_string(),
            message_type: MessageType::Text,
        })
        .await?;

    sleep(Duration::from_secs(1)).await;

    // 用户2回复消息
    info!("用户2回复消息");
    client2
        .send_chat_message(SendMessageRequest {
            room_id: room.id.clone(),
            content: "你好小明！我是小红".to_string(),
            message_type: MessageType::Text,
        })
        .await?;

    sleep(Duration::from_secs(1)).await;

    // 演示文件分享功能
    info!("演示文件分享功能");
    
    // 创建测试文件
    let test_file = std::env::temp_dir().join("chat_test.txt");
    std::fs::write(&test_file, "这是一个通过聊天分享的测试文件")?;

    // 用户1上传并分享文件
    info!("用户1上传并分享文件");
    let upload_request = iroh_node::UploadRequest {
        file_path: test_file.clone(),
    };

    let notifier = std::sync::Arc::new(iroh_node::DefaultProgressNotifier::new());
    client1
        .upload_and_share_file(upload_request, room.id.clone(), notifier)
        .await?;

    sleep(Duration::from_secs(2)).await;

    // 获取消息历史
    info!("获取聊天室消息历史");
    let history = client2.get_message_history(&room.id)?;
    info!("聊天室共有 {} 条消息", history.len());

    for message in &history {
        info!("历史消息: [{}] {}: {}", 
            message.timestamp.format("%H:%M:%S"),
            message.sender_name, 
            message.content
        );
    }

    // 查找文件分享消息并下载
    for message in &history {
        if let MessageType::FileShare { file_name, .. } = &message.message_type {
            info!("用户2下载分享的文件: {}", file_name);
            let download_notifier = std::sync::Arc::new(iroh_node::DefaultProgressNotifier::new());
            let result = client2.download_file_from_chat(message, download_notifier).await?;
            info!("文件下载完成: {}", result);
            break;
        }
    }

    sleep(Duration::from_secs(1)).await;

    // 用户离开聊天室
    info!("用户离开聊天室");
    client1
        .leave_chat_room(iroh_node::LeaveRoomRequest {
            room_id: room.id.clone(),
        })
        .await?;

    client2
        .leave_chat_room(iroh_node::LeaveRoomRequest {
            room_id: room.id.clone(),
        })
        .await?;

    sleep(Duration::from_secs(1)).await;

    // 清理测试文件
    let _ = std::fs::remove_file(test_file);

    info!("聊天功能示例完成");
    Ok(())
}