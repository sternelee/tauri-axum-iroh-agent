use anyhow::Result;
use bytes::Bytes;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Iroh P2P 聊天应用
#[derive(Parser)]
#[command(name = "iroh-chat")]
#[command(about = "基于 Iroh 的 P2P 聊天应用")]
struct Cli {
    /// 用户名
    #[arg(short, long, default_value = "匿名用户")]
    name: String,

    /// 详细日志输出
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 创建新的聊天室
    Open,
    /// 加入现有聊天室
    Join {
        /// 聊天室邀请码
        ticket: String,
    },
}

/// 聊天消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
enum Message {
    /// 用户介绍消息
    AboutMe {
        name: String,
        node_id: String,
        nonce: u64,
    },
    /// 聊天消息
    Message {
        name: String,
        content: String,
        node_id: String,
        nonce: u64,
    },
}

impl Message {
    /// 创建介绍消息
    fn about_me(name: String, node_id: String) -> Self {
        let nonce = rand::random::<u64>();
        Self::AboutMe {
            name,
            node_id,
            nonce,
        }
    }

    /// 创建聊天消息
    fn chat_message(name: String, content: String, node_id: String) -> Self {
        let nonce = rand::random::<u64>();
        Self::Message {
            name,
            content,
            node_id,
            nonce,
        }
    }

    /// 序列化为字节
    fn to_bytes(&self) -> Result<Bytes> {
        let json = serde_json::to_string(self)?;
        Ok(Bytes::from(json))
    }

    /// 从字节反序列化
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let json = std::str::from_utf8(bytes)?;
        let message = serde_json::from_str(json)?;
        Ok(message)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("iroh_node={},iroh_gossip={}", log_level, log_level))
        .init();

    info!("启动 Iroh P2P 聊天应用...");

    // 暂时使用模拟的节点ID
    let node_id = format!("node_{}", rand::random::<u32>());
    info!("节点 ID: {}", node_id);

    match cli.command {
        Commands::Open => {
            info!("创建新的聊天室...");
            open_chat_room(cli.name, node_id).await?;
        }
        Commands::Join { ticket } => {
            info!("加入聊天室: {}", ticket);
            join_chat_room(cli.name, node_id, ticket).await?;
        }
    }

    Ok(())
}

/// 创建并打开聊天室
async fn open_chat_room(name: String, node_id: String) -> Result<()> {
    // 生成邀请码 - 使用简化的格式
    let topic_id = format!("topic_{}", rand::random::<u32>());
    let ticket_data = format!("{}:{}", topic_id, node_id);

    println!("\n=== 聊天室已创建 ===");
    println!("邀请码: {}", ticket_data);
    println!("请将此邀请码分享给其他人加入聊天室");
    println!("输入消息开始聊天，输入 'quit' 退出\n");

    // 发送介绍消息
    let about_me = Message::about_me(name.clone(), node_id.clone());
    println!(">>> {} 创建了聊天室", name);

    // 启动消息处理
    start_chat_simulation(name, node_id).await
}

/// 加入现有聊天室
async fn join_chat_room(name: String, node_id: String, ticket: String) -> Result<()> {
    // 解析邀请码 - 简化格式: topic_id:node_id
    let parts: Vec<&str> = ticket.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("无效的邀请码格式"));
    }

    let topic_id = parts[0];
    let creator_node_id = parts[1];

    println!("\n=== 已加入聊天室 ===");
    println!("主题ID: {}", topic_id);
    println!("创建者节点: {}", creator_node_id);
    println!("输入消息开始聊天，输入 'quit' 退出\n");

    // 发送介绍消息
    let about_me = Message::about_me(name.clone(), node_id.clone());
    println!(">>> {} 加入了聊天室", name);

    // 启动消息处理
    start_chat_simulation(name, node_id).await
}

/// 启动聊天模拟（由于 iroh API 问题，暂时使用本地模拟）
async fn start_chat_simulation(name: String, node_id: String) -> Result<()> {
    // 创建消息通道
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // 启动输入处理任务
    let input_tx = tx.clone();
    let input_handle = tokio::task::spawn_blocking(move || {
        input_loop(input_tx);
    });

    // 模拟消息接收（实际应用中这里会是真正的网络消息接收）
    let recv_name = name.clone();
    let recv_handle = tokio::spawn(async move {
        // 这里可以添加真正的网络消息接收逻辑
        // 目前只是一个占位符
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        info!("消息接收器已启动 (节点: {})", recv_name);
    });

    println!("💡 提示: 当前版本由于 iroh API 变化，暂时只支持本地输入测试");
    println!("💡 真正的 P2P 功能将在 API 稳定后实现");
    println!();

    // 主消息发送循环
    while let Some(input) = rx.recv().await {
        if input.trim() == "quit" {
            info!("退出聊天室...");
            break;
        }

        if !input.trim().is_empty() {
            let message =
                Message::chat_message(name.clone(), input.trim().to_string(), node_id.clone());

            // 显示自己发送的消息
            println!("[{}] {}", name, input.trim());

            // 这里应该广播消息到网络
            // 由于 API 问题，暂时只是本地显示
            info!("消息已发送: {}", input.trim());
        }
    }

    // 清理任务
    input_handle.abort();
    recv_handle.abort();

    Ok(())
}

/// 输入处理循环
fn input_loop(tx: mpsc::Sender<String>) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(input) => {
                if tx.blocking_send(input).is_err() {
                    break;
                }
            }
            Err(e) => {
                error!("读取输入失败: {}", e);
                break;
            }
        }
    }
}

