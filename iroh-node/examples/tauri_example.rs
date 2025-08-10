use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use bytes::Bytes;
use ed25519_dalek::Signature;
use futures_lite::StreamExt;
use iroh::{Endpoint, NodeAddr, PublicKey, RelayMode, SecretKey};
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
use tokio::sync::{mpsc, RwLock};
use tracing::info;

/// 用于Tauri应用的P2P消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PMessage {
    /// 文本消息
    Text { from: String, content: String },
    /// 代理请求
    AgentRequest { query: String },
    /// 代理响应
    AgentResponse { query: String, response: String },
    /// 系统消息
    System { content: String },
    /// 错误消息
    Error { content: String },
}

/// 签名消息
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

/// 内部消息类型
#[derive(Debug, Serialize, Deserialize)]
enum Message {
    AboutMe { name: String },
    Message { text: String },
    AgentRequest { query: String },
    AgentResponse { query: String, response: String },
}

/// P2P节点状态
#[derive(Debug)]
pub struct P2PState {
    /// 节点ID
    pub node_id: String,
    /// 主题ID
    pub topic_id: TopicId,
    /// 密钥
    secret_key: SecretKey,
    /// 发送器
    sender: Option<GossipSender>,
    /// 对等点名称映射
    names: HashMap<PublicKey, String>,
    /// 消息发送通道
    message_tx: mpsc::Sender<P2PMessage>,
    /// 代理管理器
    agent_manager: AgentManager,
    /// 客户端注册表
    registry: ClientRegistry,
}

impl P2PState {
    /// 创建新的P2P状态
    pub fn new(
        topic_id: Option<TopicId>,
        secret_key: Option<SecretKey>,
        message_tx: mpsc::Sender<P2PMessage>,
    ) -> Self {
        // 使用提供的密钥或生成新密钥
        let secret_key = secret_key.unwrap_or_else(|| SecretKey::generate(&mut rand::rngs::OsRng));
        let node_id = secret_key.public().fmt_short().to_string();
        
        // 使用提供的主题或生成新主题
        let topic_id = topic_id.unwrap_or_else(|| TopicId::from_bytes(rand::random()));
        
        // 初始化代理
        let config = AgentConfig::default();
        let agent_manager = AgentManager::new(config);
        let registry = ClientRegistry::new();
        
        Self {
            node_id,
            topic_id,
            secret_key,
            sender: None,
            names: HashMap::new(),
            message_tx,
            agent_manager,
            registry,
        }
    }
    
    /// 初始化P2P连接
    pub async fn initialize(&mut self, name: Option<String>, bind_port: u16) -> Result<String, anyhow::Error> {
        // 构建端点
        let endpoint = Endpoint::builder()
            .secret_key(self.secret_key.clone())
            .relay_mode(RelayMode::Default)
            .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, bind_port))
            .bind()
            .await?;
            
        // 创建gossip协议
        let gossip = Gossip::builder().spawn(endpoint.clone());
        
        // 生成票据
        let ticket = {
            let me = endpoint.node_addr().initialized().await;
            let peers = vec![me];
            let ticket_data = Ticket {
                topic: self.topic_id,
                peers,
            };
            ticket_data.to_string()
        };
        
        // 设置路由器
        let router = iroh::protocol::Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();
            
        // 加入gossip主题
        let (sender, receiver) = gossip.subscribe_and_join(self.topic_id, vec![]).await?.split();
        self.sender = Some(sender.clone());
        
        // 创建默认agent
        let _ = self.agent_manager.create_agent("p2p_agent".to_string(), None).await;
        
        // 广播我们的名字（如果设置）
        if let Some(name) = name {
            let message = Message::AboutMe { name };
            let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
            sender.broadcast(encoded_message).await?;
        }
        
        // 启动消息处理循环
        let state_clone = Arc::new(RwLock::new(self.clone_essential()));
        tokio::spawn(subscribe_loop(
            receiver,
            sender,
            self.secret_key.clone(),
            self.message_tx.clone(),
            state_clone,
            self.agent_manager.clone(),
            self.registry.clone(),
        ));
        
        // 返回票据
        Ok(ticket)
    }
    
    /// 加入现有P2P网络
    pub async fn join(&mut self, ticket: &str, name: Option<String>, bind_port: u16) -> Result<(), anyhow::Error> {
        // 解析票据
        let Ticket { topic, peers } = ticket.parse::<Ticket>()?;
        self.topic_id = topic;
        
        // 构建端点
        let endpoint = Endpoint::builder()
            .secret_key(self.secret_key.clone())
            .relay_mode(RelayMode::Default)
            .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, bind_port))
            .bind()
            .await?;
            
        // 创建gossip协议
        let gossip = Gossip::builder().spawn(endpoint.clone());
        
        // 设置路由器
        let router = iroh::protocol::Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();
            
        // 将票据中的对等地址添加到我们端点的地址簿中
        let peer_ids = peers.iter().map(|p| p.node_id).collect();
        for peer in peers.into_iter() {
            endpoint.add_node_addr(peer)?;
        }
        
        // 加入gossip主题
        let (sender, receiver) = gossip.subscribe_and_join(self.topic_id, peer_ids).await?.split();
        self.sender = Some(sender.clone());
        
        // 创建默认agent
        let _ = self.agent_manager.create_agent("p2p_agent".to_string(), None).await;
        
        // 广播我们的名字（如果设置）
        if let Some(name) = name {
            let message = Message::AboutMe { name };
            let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
            sender.broadcast(encoded_message).await?;
        }
        
        // 启动消息处理循环
        let state_clone = Arc::new(RwLock::new(self.clone_essential()));
        tokio::spawn(subscribe_loop(
            receiver,
            sender,
            self.secret_key.clone(),
            self.message_tx.clone(),
            state_clone,
            self.agent_manager.clone(),
            self.registry.clone(),
        ));
        
        Ok(())
    }
    
    /// 发送文本消息
    pub async fn send_text(&self, text: String) -> Result<(), anyhow::Error> {
        if let Some(sender) = &self.sender {
            let message = Message::Message { text: text.clone() };
            let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
            sender.broadcast(encoded_message).await?;
            
            // 发送本地确认
            let _ = self.message_tx.send(P2PMessage::System {
                content: format!("已发送: {}", text),
            }).await;
        } else {
            return Err(anyhow::anyhow!("未初始化P2P连接"));
        }
        Ok(())
    }
    
    /// 发送代理请求
    pub async fn send_agent_request(&self, query: String) -> Result<(), anyhow::Error> {
        if let Some(sender) = &self.sender {
            let message = Message::AgentRequest { query: query.clone() };
            let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
            sender.broadcast(encoded_message).await?;
            
            // 发送本地确认
            let _ = self.message_tx.send(P2PMessage::System {
                content: format!("已发送代理请求: {}", query),
            }).await;
        } else {
            return Err(anyhow::anyhow!("未初始化P2P连接"));
        }
        Ok(())
    }
    
    /// 克隆基本状态（用于消息处理循环）
    fn clone_essential(&self) -> EssentialState {
        EssentialState {
            names: self.names.clone(),
        }
    }
}

/// 基本状态（用于消息处理循环）
#[derive(Debug, Clone)]
struct EssentialState {
    names: HashMap<PublicKey, String>,
}

/// 票据
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
impl std::str::FromStr for Ticket {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32_NOPAD
            .decode(s.to_ascii_uppercase().as_bytes())?;
        Self::from_bytes(&bytes)
    }
}

/// 订阅并处理消息循环
async fn subscribe_loop(
    mut receiver: GossipReceiver,
    sender: GossipSender,
    secret_key: SecretKey,
    message_tx: mpsc::Sender<P2PMessage>,
    state: Arc<RwLock<EssentialState>>,
    agent_manager: AgentManager,
    registry: ClientRegistry,
) -> Result<(), anyhow::Error> {
    while let Some(event) = receiver.try_next().await? {
        if let Event::Received(msg) = event {
            match SignedMessage::verify_and_decode(&msg.content) {
                Ok((from, message)) => {
                    handle_message(
                        from,
                        message,
                        &sender,
                        &secret_key,
                        &message_tx,
                        &state,
                        &agent_manager,
                        &registry,
                    ).await?;
                }
                Err(e) => {
                    let _ = message_tx.send(P2PMessage::Error {
                        content: format!("无法验证消息: {}", e),
                    }).await;
                }
            }
        }
    }
    Ok(())
}

/// 处理来自p2p网络的消息
async fn handle_message(
    from: PublicKey,
    message: Message,
    sender: &GossipSender,
    secret_key: &SecretKey,
    message_tx: &mpsc::Sender<P2PMessage>,
    state: &Arc<RwLock<EssentialState>>,
    agent_manager: &AgentManager,
    registry: &ClientRegistry,
) -> Result<(), anyhow::Error> {
    match message {
        Message::AboutMe { name } => {
            // 更新名称映射
            {
                let mut state = state.write().await;
                state.names.insert(from, name.clone());
            }
            
            // 发送系统消息
            let _ = message_tx.send(P2PMessage::System {
                content: format!("{} 现在被称为 {}", from.fmt_short(), name),
            }).await;
        }
        Message::Message { text } => {
            // 获取发送者名称
            let name = {
                let state = state.read().await;
                state.names
                    .get(&from)
                    .map_or_else(|| from.fmt_short(), |n| n.clone())
            };
            
            // 发送文本消息
            let _ = message_tx.send(P2PMessage::Text {
                from: name,
                content: text,
            }).await;
        }
        Message::AgentRequest { query } => {
            // 获取发送者名称
            let name = {
                let state = state.read().await;
                state.names
                    .get(&from)
                    .map_or_else(|| from.fmt_short(), |n| n.clone())
            };
            
            // 发送系统消息
            let _ = message_tx.send(P2PMessage::System {
                content: format!("收到来自 {} 的代理请求: {}", name, query),
            }).await;
            
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
                response: response.clone(),
            };
            let encoded_message = SignedMessage::sign_and_encode(secret_key, &response_message)?;
            sender.broadcast(encoded_message).await?;
            
            // 发送系统消息
            let _ = message_tx.send(P2PMessage::System {
                content: "已发送代理响应".to_string(),
            }).await;
        }
        Message::AgentResponse { query, response } => {
            // 获取发送者名称
            let name = {
                let state = state.read().await;
                state.names
                    .get(&from)
                    .map_or_else(|| from.fmt_short(), |n| n.clone())
            };
            
            // 发送代理响应
            let _ = message_tx.send(P2PMessage::AgentResponse {
                query,
                response,
            }).await;
        }
    }
    Ok(())
}

/// Tauri示例主函数
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    
    // 创建消息通道
    let (tx, mut rx) = mpsc::channel::<P2PMessage>(100);
    
    // 创建P2P状态
    let mut p2p_state = P2PState::new(None, None, tx.clone());
    
    // 初始化P2P连接
    let ticket = p2p_state.initialize(Some("Tauri用户".to_string()), 0).await?;
    println!("已创建P2P连接，票据: {}", ticket);
    
    // 发送测试消息
    p2p_state.send_text("Hello from Tauri!".to_string()).await?;
    
    // 接收消息
    println!("等待消息...");
    while let Some(message) = rx.recv().await {
        match message {
            P2PMessage::Text { from, content } => {
                println!("{}: {}", from, content);
            }
            P2PMessage::AgentRequest { query } => {
                println!("代理请求: {}", query);
            }
            P2PMessage::AgentResponse { query, response } => {
                println!("代理响应:");
                println!("查询: {}", query);
                println!("响应: {}", response);
            }
            P2PMessage::System { content } => {
                println!("系统: {}", content);
            }
            P2PMessage::Error { content } => {
                println!("错误: {}", content);
            }
        }
    }
    
    Ok(())
}