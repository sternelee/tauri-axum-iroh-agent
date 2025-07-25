//! 高级 P2P 聊天示例
//! 
//! 支持文件传输、多房间管理等高级功能

use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh_node::{IrohIntegratedClient, TransferConfig, ConfigBuilder};
use std::path::PathBuf;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tracing::{info, error, warn};

#[derive(Parser)]
#[command(name = "advanced-chat")]
#[command(about = "高级 P2P 聊天和文件传输应用")]
struct Cli {
    /// 用户名
    #[arg(short, long, default_value = "用户")]
    name: String,
    
    /// 数据目录
    #[arg(short, long)]
    data_dir: Option<PathBuf>,
    
    /// 下载目录
    #[arg(long)]
    download_dir: Option<PathBuf>,
    
    /// 详细日志
    #[arg(short, long)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动交互式聊天
    Chat,
    /// 发送文件
    SendFile {
        /// 文件路径
        file: PathBuf,
        /// 接收者邀请码
        recipient: String,
    },
    /// 接收文件
    ReceiveFile {
        /// 发送者邀请码
        sender: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("advanced_chat={},iroh_node={}", log_level, log_level))
        .init();

    // 构建配置
    let mut config_builder = ConfigBuilder::new();
    
    if let Some(data_dir) = cli.data_dir {
        config_builder = config_builder.data_root(data_dir);
    }
    
    if let Some(download_dir) = cli.download_dir {
        config_builder = config_builder.download_dir(Some(download_dir));
    }
    
    let config = config_builder
        .verbose_logging(cli.verbose)
        .build();

    // 创建集成客户端
    let client = IrohIntegratedClient::new(config).await?;
    
    match cli.command {
        Commands::Chat => {
            start_interactive_chat(client, cli.name).await?;
        }
        Commands::SendFile { file, recipient } => {
            send_file(client, file, recipient).await?;
        }
        Commands::ReceiveFile { sender } => {
            receive_file(client, sender).await?;
        }
    }

    Ok(())
}

async fn start_interactive_chat(mut client: IrohIntegratedClient, username: String) -> Result<()> {
    println!("=== 高级 P2P 聊天 ===");
    println!("命令:");
    println!("  /create <房间名> - 创建聊天室");
    println!("  /join <邀请码> - 加入聊天室");
    println!("  /send <文件路径> - 发送文件");
    println!("  /rooms - 列出所有房间");
    println!("  /quit - 退出");
    println!();

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut current_room: Option<String> = None;

    loop {
        print!("> ");
        let mut input = String::new();
        if reader.read_line(&mut input).await.is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.starts_with('/') {
            // 处理命令
            let parts: Vec<&str> = input.splitn(2, ' ').collect();
            let command = parts[0];
            let args = parts.get(1).unwrap_or(&"");

            match command {
                "/create" => {
                    if args.is_empty() {
                        println!("用法: /create <房间名>");
                        continue;
                    }
                    
                    match client.chat_client().create_room(args.to_string()).await {
                        Ok(room) => {
                            println!("聊天室 '{}' 已创建", room.name);
                            println!("邀请码: {}", room.invite_code);
                            current_room = Some(room.room_id);
                        }
                        Err(e) => {
                            error!("创建聊天室失败: {}", e);
                        }
                    }
                }
                "/join" => {
                    if args.is_empty() {
                        println!("用法: /join <邀请码>");
                        continue;
                    }
                    
                    match client.chat_client().join_room(args.to_string()).await {
                        Ok(room) => {
                            println!("已加入聊天室: {}", room.name);
                            current_room = Some(room.room_id);
                        }
                        Err(e) => {
                            error!("加入聊天室失败: {}", e);
                        }
                    }
                }
                "/send" => {
                    if args.is_empty() {
                        println!("用法: /send <文件路径>");
                        continue;
                    }
                    
                    let file_path = PathBuf::from(args);
                    if !file_path.exists() {
                        println!("文件不存在: {}", args);
                        continue;
                    }
                    
                    match client.transfer_client().upload_file(file_path.into()).await {
                        Ok(response) => {
                            println!("文件上传成功!");
                            println!("分享码: {}", response.doc_ticket);
                        }
                        Err(e) => {
                            error!("文件上传失败: {}", e);
                        }
                    }
                }
                "/rooms" => {
                    // 这里可以实现房间列表功能
                    println!("当前房间: {:?}", current_room);
                }
                "/quit" => {
                    break;
                }
                _ => {
                    println!("未知命令: {}", command);
                }
            }
        } else {
            // 发送聊天消息
            if let Some(ref room_id) = current_room {
                if let Err(e) = client.chat_client().send_message(room_id.clone(), input.to_string()).await {
                    error!("发送消息失败: {}", e);
                } else {
                    println!("[{}] {}", username, input);
                }
            } else {
                println!("请先创建或加入一个聊天室");
            }
        }
    }

    println!("退出聊天...");
    Ok(())
}

async fn send_file(mut client: IrohIntegratedClient, file_path: PathBuf, _recipient: String) -> Result<()> {
    info!("发送文件: {:?}", file_path);
    
    if !file_path.exists() {
        return Err(anyhow::anyhow!("文件不存在: {:?}", file_path));
    }

    let response = client.transfer_client().upload_file(file_path.into()).await?;
    
    println!("文件发送成功!");
    println!("分享码: {}", response.doc_ticket);
    println!("请将此分享码发送给接收者");

    Ok(())
}

async fn receive_file(mut client: IrohIntegratedClient, sender_code: String) -> Result<()> {
    info!("接收文件，分享码: {}", sender_code);
    
    let request = iroh_node::DownloadRequest {
        doc_ticket: sender_code,
        download_dir: None,
    };

    match client.transfer_client().download_files(request).await {
        Ok(download_path) => {
            println!("文件接收成功!");
            println!("保存位置: {}", download_path);
        }
        Err(e) => {
            error!("文件接收失败: {}", e);
        }
    }

    Ok(())
}