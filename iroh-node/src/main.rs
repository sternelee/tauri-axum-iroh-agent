//! iroh-node命令行工具
//!
//! 提供命令行接口，用于管理P2P节点

use clap::{Parser, Subcommand};
use iroh_net::relay::RelayUrl;
use iroh_gossip::proto::topic::TopicId;
use iroh_node::{NodeConfig, NodeResult, P2PNode};
use tracing::{error, info};

/// iroh-node命令行工具
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// 密钥
    #[clap(long)]
    secret_key: Option<String>,
    
    /// 中继服务器URL
    #[clap(short, long)]
    relay: Option<RelayUrl>,
    
    /// 禁用中继
    #[clap(long)]
    no_relay: bool,
    
    /// 节点名称
    #[clap(short, long)]
    name: Option<String>,
    
    /// 绑定端口
    #[clap(short, long, default_value = "0")]
    bind_port: u16,
    
    /// 子命令
    #[clap(subcommand)]
    command: Option<Command>,
}

/// 子命令
#[derive(Subcommand, Debug)]
enum Command {
    /// 创建或加入话题
    Topic {
        /// 话题ID
        #[clap(long)]
        topic_id: Option<TopicId>,
        
        /// 票据
        #[clap(long)]
        ticket: Option<String>,
    },
    
    /// 发送消息
    Send {
        /// 话题ID
        #[clap(long)]
        topic_id: TopicId,
        
        /// 消息内容
        #[clap(long)]
        message: String,
    },
    
    /// 发送Agent请求
    Agent {
        /// 话题ID
        #[clap(long)]
        topic_id: TopicId,
        
        /// Agent ID
        #[clap(long)]
        agent_id: String,
        
        /// 提示词
        #[clap(long)]
        prompt: String,
    },
    
    /// 获取节点状态
    Status,
}

#[tokio::main]
async fn main() -> NodeResult<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 解析命令行参数
    let args = Args::parse();
    
    // 创建节点配置
    let config = NodeConfig {
        secret_key: args.secret_key,
        relay: args.relay,
        no_relay: args.no_relay,
        name: args.name.clone(),
        bind_port: args.bind_port,
    };
    
    // 创建P2P节点
    let mut node = P2PNode::new(config).await?;
    
    // 设置节点名称
    if let Some(name) = args.name {
        node.set_name(name);
    }
    
    // 启动节点
    node.start().await?;
    
    // 处理子命令
    match args.command {
        Some(Command::Topic { topic_id, ticket }) => {
            let (topic, ticket) = node.join_topic(topic_id, ticket.as_deref()).await?;
            info!("话题ID: {}", topic);
            info!("票据: {}", ticket);
        }
        Some(Command::Send { topic_id, message }) => {
            let message = iroh_node::MessageType::Chat { text: message };
            node.send_message(&topic_id, message).await?;
            info!("消息已发送");
        }
        Some(Command::Agent { topic_id, agent_id, prompt }) => {
            node.send_agent_request(&topic_id, &agent_id, &prompt).await?;
            info!("Agent请求已发送");
        }
        Some(Command::Status) => {
            let status = node.get_status().await;
            info!("节点状态: {:?}", status);
        }
        None => {
            // 如果没有子命令，则创建一个新话题
            let (topic, ticket) = node.join_topic(None, None).await?;
            info!("已创建新话题");
            info!("话题ID: {}", topic);
            info!("票据: {}", ticket);
            
            // 等待用户输入
            info!("输入消息发送到话题，输入'exit'退出");
            let mut input = String::new();
            while input.trim() != "exit" {
                input.clear();
                if std::io::stdin().read_line(&mut input).is_err() {
                    error!("读取输入失败");
                    continue;
                }
                
                if input.trim() != "exit" {
                    let message = iroh_node::MessageType::Chat { text: input.trim().to_string() };
                    if let Err(e) = node.send_message(&topic, message).await {
                        error!("发送消息失败: {}", e);
                    }
                }
            }
        }
    }
    
    // 停止节点
    node.stop().await?;
    
    Ok(())
}