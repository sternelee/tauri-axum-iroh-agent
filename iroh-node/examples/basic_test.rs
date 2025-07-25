//! 基本的 Iroh 节点测试
//! 
//! 这个示例用于测试 Iroh 节点的基本功能

use anyhow::Result;
use clap::{Parser, Subcommand};
use iroh::node::Node;
use tracing::info;

#[derive(Parser)]
#[command(name = "basic-test")]
#[command(about = "基本的 Iroh 节点测试")]
struct Cli {
    /// 详细日志输出
    #[arg(short, long)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动节点
    Start,
    /// 显示节点信息
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("basic_test={},iroh={}", log_level, log_level))
        .init();

    info!("启动基本 Iroh 节点测试...");

    // 创建 Iroh 节点
    let node = Node::memory().spawn().await?;
    let node_id = node.node_id();

    match cli.command {
        Commands::Start => {
            info!("节点已启动");
            info!("节点 ID: {}", node_id);
            info!("节点地址: {:?}", node.my_addr().await?);
            
            // 保持节点运行
            println!("节点正在运行，按 Ctrl+C 退出...");
            tokio::signal::ctrl_c().await?;
            println!("正在关闭节点...");
        }
        Commands::Info => {
            info!("节点信息:");
            info!("  节点 ID: {}", node_id);
            info!("  节点地址: {:?}", node.my_addr().await?);
        }
    }

    Ok(())
}