//! 最小化的 P2P 聊天示例
//! 
//! 专注于核心的 gossip 聊天功能

use anyhow::Result;
use bytes::Bytes;
use clap::{Parser, Subcommand};
use futures_lite::StreamExt;
use iroh::node::Node;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead};
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "minimal-chat")]
#[command(about = "最小化的 P2P 聊天应用")]
struct Cli {
    /// 用户名
    #[arg(short, long, default_value = "用户")]
    name: String,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 创建聊天室
    Create,
    /// 加入聊天室
    Join {
        /// 主题ID (hex格式)
        topic: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    user: String,
    content: String,
    timestamp: u64,
}

impl ChatMessage {
    fn new(user: String, content: String) -> Self {
        Self {
            user,
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    fn to_bytes(&self) -> Result<Bytes> {
        let json = serde_json::to_string(self)?;
        Ok(Bytes::from(json))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let json = std::str::from_utf8(bytes)?;
        Ok(serde_json::from_str(json)?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("启动最小化聊天应用...");

    // 创建节点
    let node = Node::memory().spawn().await?;
    let node_id = node.node_id();
    info!("节点 ID: {}", node_id);

    match cli.command {
        Commands::Create => {
            create_chat_room(&node, cli.name).await?;
        }
        Commands::Join { topic } => {
            join_chat_room(&node, cli.name, topic).await?;
        }
    }

    Ok(())
}

async fn create_chat_room(node: &Node<iroh::blobs::store::mem::Store>, username: String) -> Result<()> {
    // 创建固定的主题ID用于测试
    let topic_id = TopicId::from([1u8; 32]);
    
    println!("=== 聊天室已创建 ===");
    println!("主题ID: {}", hex::encode(topic_id.as_bytes()));
    println!("其他人可以使用以下命令加入:");
    println!("cargo run --example minimal_chat -- join {}", hex::encode(topic_id.as_bytes()));
    println!();

    start_chat(node, topic_id, username).await
}

async fn join_chat_room(node: &Node<iroh::blobs::store::mem::Store>, username: String, topic_hex: String) -> Result<()> {
    let topic_bytes = hex::decode(topic_hex)?;
    if topic_bytes.len() != 32 {
        return Err(anyhow::anyhow!("无效的主题ID长度"));
    }

    let mut topic_array = [0u8; 32];
    topic_array.copy_from_slice(&topic_bytes);
    let topic_id = TopicId::from(topic_array);

    println!("=== 加入聊天室 ===");
    println!("主题ID: {}", hex::encode(topic_id.as_bytes()));
    println!();

    start_chat(node, topic_id, username).await
}

async fn start_chat(node: &Node<iroh::blobs::store::mem::Store>, topic_id: TopicId, username: String) -> Result<()> {
    let gossip = node.gossip();
    let (mut sink, mut stream) = gossip.subscribe(topic_id).await?;

    println!("聊天开始！输入消息并按回车发送，输入 'quit' 退出");

    // 发送加入消息
    let join_msg = ChatMessage::new(username.clone(), "加入了聊天室".to_string());
    sink.broadcast(join_msg.to_bytes()?).await?;

    // 创建输入通道
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // 启动输入任务
    let input_tx = tx.clone();
    let input_handle = tokio::task::spawn_blocking(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(input) => {
                    if input_tx.blocking_send(input).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 启动消息接收任务
    let recv_handle = tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            match event {
                Ok(iroh_gossip::net::Event::Received(msg)) => {
                    match ChatMessage::from_bytes(&msg.content) {
                        Ok(chat_msg) => {
                            println!("[{}] {}", chat_msg.user, chat_msg.content);
                        }
                        Err(e) => {
                            error!("解析消息失败: {}", e);
                        }
                    }
                }
                Ok(iroh_gossip::net::Event::Joined(peer)) => {
                    info!("节点加入: {}", peer);
                }
                Ok(iroh_gossip::net::Event::Left(peer)) => {
                    info!("节点离开: {}", peer);
                }
                Err(e) => {
                    error!("接收事件失败: {}", e);
                }
            }
        }
    });

    // 主循环
    while let Some(input) = rx.recv().await {
        let input = input.trim();
        if input == "quit" {
            break;
        }

        if !input.is_empty() {
            let msg = ChatMessage::new(username.clone(), input.to_string());
            if let Err(e) = sink.broadcast(msg.to_bytes()?).await {
                error!("发送消息失败: {}", e);
            }
        }
    }

    // 发送离开消息
    let leave_msg = ChatMessage::new(username, "离开了聊天室".to_string());
    let _ = sink.broadcast(leave_msg.to_bytes()?).await;

    // 清理任务
    input_handle.abort();
    recv_handle.abort();

    println!("聊天结束");
    Ok(())
}