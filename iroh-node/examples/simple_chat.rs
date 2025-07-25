//! 简单的 P2P 聊天示例
//! 
//! 这个示例展示了如何使用 iroh-node 创建一个基本的点对点聊天应用

use anyhow::Result;
use iroh_node::{IrohChatClient, ChatConfig, ChatEvent};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== Iroh P2P 聊天示例 ===");
    println!("1. 创建聊天室 (输入 'create')");
    println!("2. 加入聊天室 (输入 'join <邀请码>')");
    println!("请选择操作:");

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input = String::new();

    reader.read_line(&mut input).await?;
    let input = input.trim();

    let config = ChatConfig::default();
    let mut client = IrohChatClient::new(config).await?;

    if input == "create" {
        // 创建聊天室
        let room = client.create_room("默认聊天室".to_string()).await?;
        println!("聊天室已创建！");
        println!("邀请码: {}", room.invite_code);
        println!("请分享此邀请码给其他人");
        
        // 开始聊天循环
        start_chat_loop(&mut client, room.room_id).await?;
        
    } else if input.starts_with("join ") {
        // 加入聊天室
        let invite_code = input.strip_prefix("join ").unwrap_or("");
        if invite_code.is_empty() {
            println!("错误: 请提供邀请码");
            return Ok(());
        }
        
        let room = client.join_room(invite_code.to_string()).await?;
        println!("已加入聊天室: {}", room.name);
        
        // 开始聊天循环
        start_chat_loop(&mut client, room.room_id).await?;
        
    } else {
        println!("无效的输入，请重新运行程序");
    }

    Ok(())
}

async fn start_chat_loop(client: &mut IrohChatClient, room_id: String) -> Result<()> {
    println!("\n=== 聊天开始 ===");
    println!("输入消息并按回车发送，输入 'quit' 退出");
    
    // 启动事件监听任务
    let mut event_receiver = client.subscribe_events().await?;
    tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            match event {
                ChatEvent::MessageReceived { user, message, .. } => {
                    println!("[{}] {}", user.name, message.content);
                }
                ChatEvent::UserJoined { user } => {
                    println!(">>> {} 加入了聊天室", user.name);
                }
                ChatEvent::UserLeft { user } => {
                    println!(">>> {} 离开了聊天室", user.name);
                }
                _ => {}
            }
        }
    });

    // 输入循环
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input = String::new();

    loop {
        input.clear();
        if reader.read_line(&mut input).await.is_err() {
            break;
        }

        let message = input.trim();
        if message == "quit" {
            break;
        }

        if !message.is_empty() {
            if let Err(e) = client.send_message(room_id.clone(), message.to_string()).await {
                error!("发送消息失败: {}", e);
            }
        }
    }

    println!("退出聊天室...");
    client.leave_room(room_id).await?;
    Ok(())
}