use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
};

use bytes::Bytes;
use clap::Parser;
use ed25519_dalek::Signature;
use futures_lite::StreamExt;
use iroh::{Endpoint, NodeAddr, PublicKey, RelayMode, RelayUrl, SecretKey};
use iroh_gossip::{
    api::{Event, GossipReceiver, GossipSender},
    net::{Gossip, GOSSIP_ALPN},
    proto::TopicId,
};
use postcard;
use rig_agent::{
    core::{
        agent::{AgentManager, ClientRegistry},
        types::{AgentConfig, AgentMessage},
    },
    error::AgentResult,
};
use serde::{Deserialize, Serialize};
use tracing::info;

/// 简化的iroh-gossip通信示例
///
/// 这个示例展示了如何使用iroh-gossip进行p2p通信，并与rig-agent集成
/// 
/// 默认使用n0提供的中继服务器。要使用本地中继服务器，请在另一个终端运行：
///     cargo run --bin iroh-relay --features iroh-relay -- --dev
/// 然后在此示例中设置 `-d http://localhost:3340` 标志。
#[derive(Parser, Debug)]
struct Args {
    /// 用于派生节点ID的密钥
    #[clap(long)]
    secret_key: Option<String>,
    /// 设置自定义中继服务器。默认使用n0托管的中继服务器。
    #[clap(short, long)]
    relay: Option<RelayUrl>,
    /// 完全禁用中继
    #[clap(long)]
    no_relay: bool,
    /// 设置您的昵称
    #[clap(short, long)]
    name: Option<String>,
    /// 设置套接字的绑定端口。默认使用随机端口。
    #[clap(short, long, default_value = "0")]
    bind_port: u16,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// 为主题打开聊天室并打印供他人加入的票据
    ///
    /// 如果未提供主题，将创建一个新主题
    Open {
        /// 可选设置主题ID（64字节的十六进制字符串）
        topic: Option<TopicId>,
    },
    /// 使用票据加入聊天室
    Join {
        /// 票据（base32字符串）
        ticket: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct SignedMessage {
    from: PublicKey,
    data: Bytes,
    signature: Signature,
}

impl SignedMessage {
    pub fn verify_and_decode(bytes: &[u8]) -> Result<(PublicKey, Message), anyhow::Error> {
        let signed_message: Self = postcard::from_bytes(bytes)?;
        let key: PublicKey = signed_message.from;
        key.verify(&signed_message.data, &signed_message.signature)?;
        let message: Message = postcard::from_bytes(&signed_message.data)?;
        Ok((signed_message.from, message))
    }

    pub fn sign_and_encode(secret_key: &SecretKey, message: &Message) -> Result<Bytes, anyhow::Error> {
        let data: Bytes = postcard::to_stdvec(message)?.into();
        let signature = secret_key.sign(&data);
        let from: PublicKey = secret_key.public();
        let signed_message = Self {
            from,
            data,
            signature,
        };
        let encoded = postcard::to_stdvec(&signed_message)?;
        Ok(encoded.into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    AboutMe { name: String },
    Message { text: String },
    AgentRequest { query: String },
    AgentResponse { query: String, response: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct Ticket {
    topic: TopicId,
    peers: Vec<NodeAddr>,
}

impl Ticket {
    /// 从字节反序列化
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        Ok(postcard::from_bytes(bytes)?)
    }
    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("postcard::to_stdvec is infallible")
    }
}

/// 序列化为base32
impl std::fmt::Display for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{text}")
    }
}

/// 从base32反序列化
impl FromStr for Ticket {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32_NOPAD
            .decode(s.to_ascii_uppercase().as_bytes())?;
        Self::from_bytes(&bytes)
    }
}

// 辅助函数
fn fmt_relay_mode(relay_mode: &RelayMode) -> String {
    match relay_mode {
        RelayMode::Disabled => "None".to_string(),
        RelayMode::Default => "Default Relay (production) servers".to_string(),
        RelayMode::Staging => "Default Relay (staging) servers".to_string(),
        RelayMode::Custom(map) => map
            .urls()
            .map(|url| url.to_string())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// 处理来自p2p网络的消息
async fn handle_message(
    from: PublicKey,
    message: Message,
    sender: &GossipSender,
    secret_key: &SecretKey,
    names: &mut HashMap<PublicKey, String>,
    agent_manager: &AgentManager,
    registry: &ClientRegistry,
) -> Result<(), anyhow::Error> {
    match message {
        Message::AboutMe { name } => {
            names.insert(from, name.clone());
            println!("> {} 现在被称为 {}", from.fmt_short(), name);
        }
        Message::Message { text } => {
            let name = names
                .get(&from)
                .map_or_else(|| from.fmt_short(), String::to_string);
            println!("{name}: {text}");
        }
        Message::AgentRequest { query } => {
            println!("> 收到来自 {} 的代理请求: {}", from.fmt_short(), query);
            
            // 确保agent存在
            let agent_id = "p2p_agent";
            if !agent_manager.list_agents().await.contains(&agent_id.to_string()) {
                agent_manager.create_agent(agent_id.to_string(), None).await?;
            }
            
            // 调用rig-agent处理请求
            let response = match agent_manager.chat(registry, agent_id, &query).await {
                Ok(resp) => resp.content,
                Err(e) => format!("处理请求时出错: {}", e),
            };
            
            // 发送响应
            let response_message = Message::AgentResponse {
                query: query.clone(),
                response,
            };
            let encoded_message = SignedMessage::sign_and_encode(secret_key, &response_message)?;
            sender.broadcast(encoded_message).await?;
            println!("> 已发送代理响应");
        }
        Message::AgentResponse { query, response } => {
            let name = names
                .get(&from)
                .map_or_else(|| from.fmt_short(), String::to_string);
            println!("代理响应 (来自 {name}):");
            println!("> 查询: {query}");
            println!("> 响应: {response}");
        }
    }
    Ok(())
}

/// 订阅并处理消息循环
async fn subscribe_loop(
    mut receiver: GossipReceiver,
    sender: GossipSender,
    secret_key: SecretKey,
    agent_manager: AgentManager,
    registry: ClientRegistry,
) -> Result<(), anyhow::Error> {
    // 初始化peerid -> name哈希表
    let mut names = HashMap::new();
    
    while let Some(event) = receiver.try_next().await? {
        if let Event::Received(msg) = event {
            match SignedMessage::verify_and_decode(&msg.content) {
                Ok((from, message)) => {
                    if let Err(e) = handle_message(
                        from,
                        message,
                        &sender,
                        &secret_key,
                        &mut names,
                        &agent_manager,
                        &registry,
                    ).await {
                        println!("处理消息时出错: {}", e);
                    }
                }
                Err(e) => {
                    println!("无法验证消息: {}", e);
                }
            }
        }
    }
    Ok(())
}

/// 输入循环，读取stdin
fn input_loop(
    line_tx: tokio::sync::mpsc::Sender<String>,
    agent_mode: bool,
) -> Result<(), anyhow::Error> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    
    println!("> 输入消息并按回车发送...");
    if agent_mode {
        println!("> 以 '/agent ' 开头的消息将作为代理请求发送");
    }
    
    loop {
        buffer.clear();
        stdin.read_line(&mut buffer)?;
        if !buffer.trim().is_empty() {
            line_tx.blocking_send(buffer.clone())?;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // 解析CLI命令
    let (topic, peers) = match &args.command {
        Command::Open { topic } => {
            let topic = topic.unwrap_or_else(|| TopicId::from_bytes(rand::random()));
            println!("> 为主题 {} 打开聊天室", topic);
            (topic, vec![])
        }
        Command::Join { ticket } => {
            let Ticket { topic, peers } = Ticket::from_str(ticket)?;
            println!("> 加入主题 {} 的聊天室", topic);
            (topic, peers)
        }
    };

    // 解析或生成密钥
    let secret_key = match args.secret_key {
        None => SecretKey::generate(&mut rand::rngs::OsRng),
        Some(key) => key.parse()?,
    };
    println!(
        "> 我们的密钥: {}",
        data_encoding::HEXLOWER.encode(&secret_key.to_bytes())
    );

    // 配置中继模式
    let relay_mode = match (args.no_relay, args.relay) {
        (false, None) => RelayMode::Default,
        (false, Some(url)) => RelayMode::Custom(url.into()),
        (true, None) => RelayMode::Disabled,
        (true, Some(_)) => {
            return Err(anyhow::anyhow!("不能同时设置 --no-relay 和 --relay"));
        }
    };
    println!("> 使用中继服务器: {}", fmt_relay_mode(&relay_mode));

    // 构建端点
    let endpoint = Endpoint::builder()
        .secret_key(secret_key.clone())
        .relay_mode(relay_mode)
        .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, args.bind_port))
        .bind()
        .await?;
    println!("> 我们的节点ID: {}", endpoint.node_id());

    // 创建gossip协议
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // 打印包含我们自己节点ID和端点地址的票据
    let ticket = {
        let me = endpoint.node_addr().initialized().await;
        let peers = peers.iter().cloned().chain([me]).collect();
        Ticket { topic, peers }
    };
    println!("> 加入我们的票据: {ticket}");

    // 设置路由器
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(GOSSIP_ALPN, gossip.clone())
        .spawn();

    // 通过连接到已知对等点（如果有）加入gossip主题
    let peer_ids = peers.iter().map(|p| p.node_id).collect();
    if peers.is_empty() {
        println!("> 等待对等点加入我们...");
    } else {
        println!("> 尝试连接到 {} 个对等点...", peers.len());
        // 将票据中的对等地址添加到我们端点的地址簿中，以便可以拨号
        for peer in peers.into_iter() {
            endpoint.add_node_addr(peer)?;
        }
    };
    let (sender, receiver) = gossip.subscribe_and_join(topic, peer_ids).await?.split();
    println!("> 已连接!");

    // 初始化rig-agent
    let config = AgentConfig::default();
    let agent_manager = AgentManager::new(config);
    let registry = ClientRegistry::new();
    
    // 创建默认agent
    agent_manager.create_agent("p2p_agent".to_string(), None).await?;

    // 广播我们的名字（如果设置）
    if let Some(name) = args.name {
        let message = Message::AboutMe { name };
        let encoded_message = SignedMessage::sign_and_encode(&secret_key, &message)?;
        sender.broadcast(encoded_message).await?;
    }

    // 订阅和打印循环
    tokio::spawn(subscribe_loop(
        receiver,
        sender.clone(),
        secret_key.clone(),
        agent_manager.clone(),
        registry.clone(),
    ));

    // 生成一个输入线程，读取stdin
    let (line_tx, mut line_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || input_loop(line_tx, true));

    // 广播我们输入的每一行
    while let Some(text) = line_rx.recv().await {
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }

        // 检查是否是代理请求
        if text.starts_with("/agent ") {
            let query = text[7..].trim().to_string();
            if !query.is_empty() {
                let message = Message::AgentRequest { query: query.clone() };
                let encoded_message = SignedMessage::sign_and_encode(&secret_key, &message)?;
                sender.broadcast(encoded_message).await?;
                println!("> 已发送代理请求: {}", query);
            }
        } else {
            // 普通消息
            let message = Message::Message { text: text.clone() };
            let encoded_message = SignedMessage::sign_and_encode(&secret_key, &message)?;
            sender.broadcast(encoded_message).await?;
            println!("> 已发送: {}", text);
        }
    }

    // 关闭
    router.shutdown().await?;

    Ok(())
}