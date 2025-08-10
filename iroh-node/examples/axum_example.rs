use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
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
use tokio::sync::{broadcast, mpsc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

/// WebSocket消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsMessage {
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

/// API请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiRequest {
    /// 初始化P2P连接
    Initialize { name: Option<String> },
    /// 加入现有P2P网络
    Join { ticket: String, name: Option<String> },
    /// 发送文本消息
    SendText { content: String },
    /// 发送代理请求
    SendAgentRequest { query: String },
}

/// API响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    /// 是否成功
    pub success: bool,
    /// 消息
    pub message: String,
    /// 数据
    pub data: Option<serde_json::Value>,
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
    pub topic_id: Option<TopicId>,
    /// 密钥
    secret_key: SecretKey,
    /// 发送器
    sender: Option<GossipSender>,
    /// 对等点名称映射
    names: HashMap<PublicKey, String>,
    /// 代理管理器
    agent_manager: AgentManager,
    /// 客户端注册表
    registry: ClientRegistry,
    /// 是否已初始化
    initialized: bool,
}

impl P2PState {
    /// 创建新的P2P状态
    pub fn new() -> Self {
        // 生成新密钥
        let secret_key = SecretKey::generate(&mut rand::rngs::OsRng);
        let node_id = secret_key.public().fmt_short().to_string();
        
        // 初始化代理
        let config = AgentConfig::default();
        let agent_manager = AgentManager::new(config);
        let registry = ClientRegistry::new();
        
        Self {
            node_id,
            topic_id: None,
            secret_key,
            sender: None,
            names: HashMap::new(),
            agent_manager,
            registry,
            initialized: false,
        }
    }
    
    /// 初始化P2P连接
    pub async fn initialize(&mut self, name: Option<String>, tx: broadcast::Sender<WsMessage>) -> Result<String, anyhow::Error> {
        // 生成新主题
        let topic_id = TopicId::from_bytes(rand::random());
        self.topic_id = Some(topic_id);
        
        // 构建端点
        let endpoint = Endpoint::builder()
            .secret_key(self.secret_key.clone())
            .relay_mode(RelayMode::Default)
            .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .bind()
            .await?;
            
        // 创建gossip协议
        let gossip = Gossip::builder().spawn(endpoint.clone());
        
        // 生成票据
        let ticket = {
            let me = endpoint.node_addr().initialized().await;
            let peers = vec![me];
            let ticket_data = Ticket {
                topic: topic_id,
                peers,
            };
            ticket_data.to_string()
        };
        
        // 设置路由器
        let router = iroh::protocol::Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();
            
        // 加入gossip主题
        let (sender, receiver) = gossip.subscribe_and_join(topic_id, vec![]).await?.split();
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
            tx,
            state_clone,
            self.agent_manager.clone(),
            self.registry.clone(),
        ));
    
    Ok(())
}
        // 返回票据
        Ok(ticket)
    }
    
    /// 加入现有P2P网络
    pub async fn join(&mut self, ticket: &str, name: Option<String>, tx: broadcast::Sender<WsMessage>) -> Result<(), anyhow::Error> {
        // 解析票据
        let Ticket { topic, peers } = ticket.parse::<Ticket>()?;
        self.topic_id = Some(topic);
        
        // 构建端点
        let endpoint = Endpoint::builder()
            .secret_key(self.secret_key.clone())
            .relay_mode(RelayMode::Default)
            .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
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
        let (sender, receiver) = gossip.subscribe_and_join(topic, peer_ids).await?.split();
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
            tx,
            state_clone,
            self.agent_manager.clone(),
            self.registry.clone(),
        ));
        
        self.initialized = true;
        
        Ok(())
    }
    
    /// 发送文本消息
    pub async fn send_text(&self, text: String, tx: &broadcast::Sender<WsMessage>) -> Result<(), anyhow::Error> {
        if !self.initialized {
            return Err(anyhow::anyhow!("未初始化P2P连接"));
        }
        
        if let Some(sender) = &self.sender {
            let message = Message::Message { text: text.clone() };
            let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
            sender.broadcast(encoded_message).await?;
            
            // 发送本地确认
            let _ = tx.send(WsMessage::System {
                content: format!("已发送: {}", text),
            });
        } else {
            return Err(anyhow::anyhow!("未初始化P2P连接"));
        }
        Ok(())
    }
    
    /// 发送代理请求
    pub async fn send_agent_request(&self, query: String, tx: &broadcast::Sender<WsMessage>) -> Result<(), anyhow::Error> {
        if !self.initialized {
            return Err(anyhow::anyhow!("未初始化P2P连接"));
        }
        
        if let Some(sender) = &self.sender {
            let message = Message::AgentRequest { query: query.clone() };
            let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
            sender.broadcast(encoded_message).await?;
            
            // 发送本地确认
            let _ = tx.send(WsMessage::System {
                content: format!("已发送代理请求: {}", query),
            });
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
    tx: broadcast::Sender<WsMessage>,
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
                        &tx,
                        &state,
                        &agent_manager,
                        &registry,
                    ).await?;
                }
                Err(e) => {
                    let _ = tx.send(WsMessage::Error {
                        content: format!("无法验证消息: {}", e),
                    });
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
    tx: &broadcast::Sender<WsMessage>,
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
            let _ = tx.send(WsMessage::System {
                content: format!("{} 现在被称为 {}", from.fmt_short(), name),
            });
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
            let _ = tx.send(WsMessage::Text {
                from: name,
                content: text,
            });
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
            let _ = tx.send(WsMessage::System {
                content: format!("收到来自 {} 的代理请求: {}", name, query),
            });
            
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
            let _ = tx.send(WsMessage::System {
                content: "已发送代理响应".to_string(),
            });
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
            let _ = tx.send(WsMessage::AgentResponse {
                query,
                response,
            });
        }
    }
    Ok(())
}

/// 应用状态
#[derive(Clone)]
struct AppState {
    p2p: Arc<RwLock<P2PState>>,
    tx: broadcast::Sender<WsMessage>,
}

/// 处理WebSocket连接
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// 处理WebSocket连接
async fn handle_socket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();
    
    // 发送初始状态
    let p2p = state.p2p.read().await;
    let node_id = p2p.node_id.clone();
    let initialized = p2p.initialized;
    let topic_id = p2p.topic_id.map(|t| t.to_string());
    drop(p2p);
    
    // 发送初始状态消息
    let init_msg = WsMessage::System {
        content: format!(
            "已连接到P2P节点。节点ID: {}，状态: {}{}",
            node_id,
            if initialized { "已初始化" } else { "未初始化" },
            topic_id.map_or("".to_string(), |t| format!(", 主题: {}", t)),
        ),
    };
    
    // 转发消息到WebSocket
    let mut send_task = tokio::spawn(async move {
        if let Err(e) = sender.send(axum::extract::ws::Message::Text(
            serde_json::to_string(&init_msg).unwrap()
        )).await {
            warn!("发送初始状态消息失败: {}", e);
            return;
        }
        
        while let Ok(msg) = rx.recv().await {
            let ws_msg = match serde_json::to_string(&msg) {
                Ok(s) => axum::extract::ws::Message::Text(s),
                Err(e) => {
                    warn!("序列化WebSocket消息失败: {}", e);
                    continue;
                }
            };
            
            if let Err(e) = sender.send(ws_msg).await {
                warn!("发送WebSocket消息失败: {}", e);
                break;
            }
        }
    });
    
    // 接收WebSocket消息
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let axum::extract::ws::Message::Text(text) = msg {
                match serde_json::from_str::<ApiRequest>(&text) {
                    Ok(request) => {
                        handle_api_request(request, &state_clone).await;
                    }
                    Err(e) => {
                        warn!("解析WebSocket请求失败: {}", e);
                        let _ = state_clone.tx.send(WsMessage::Error {
                            content: format!("无效的请求格式: {}", e),
                        });
                    }
                }
            }
        }
    });
    
    // 等待任一任务完成
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

/// 处理API请求
async fn handle_api_request(request: ApiRequest, state: &AppState) {
    match request {
        ApiRequest::Initialize { name } => {
            let mut p2p = state.p2p.write().await;
            match p2p.initialize(name, state.tx.clone()).await {
                Ok(ticket) => {
                    let _ = state.tx.send(WsMessage::System {
                        content: format!("P2P连接已初始化，票据: {}", ticket),
                    });
                }
                Err(e) => {
                    let _ = state.tx.send(WsMessage::Error {
                        content: format!("初始化P2P连接失败: {}", e),
                    });
                }
            }
        }
        ApiRequest::Join { ticket, name } => {
            let mut p2p = state.p2p.write().await;
            match p2p.join(&ticket, name, state.tx.clone()).await {
                Ok(_) => {
                    let _ = state.tx.send(WsMessage::System {
                        content: "已加入P2P网络".to_string(),
                    });
                }
                Err(e) => {
                    let _ = state.tx.send(WsMessage::Error {
                        content: format!("加入P2P网络失败: {}", e),
                    });
                }
            }
        }
        ApiRequest::SendText { content } => {
            let p2p = state.p2p.read().await;
            match p2p.send_text(content, &state.tx).await {
                Ok(_) => {}
                Err(e) => {
                    let _ = state.tx.send(WsMessage::Error {
                        content: format!("发送消息失败: {}", e),
                    });
                }
            }
        }
        ApiRequest::SendAgentRequest { query } => {
            let p2p = state.p2p.read().await;
            match p2p.send_agent_request(query, &state.tx).await {
                Ok(_) => {}
                Err(e) => {
                    let _ = state.tx.send(WsMessage::Error {
                        content: format!("发送代理请求失败: {}", e),
                    });
                }
            }
        }
    }
}

/// 初始化P2P连接
async fn initialize(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<ApiResponse> {
    let name = payload.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    let mut p2p = state.p2p.write().await;
    match p2p.initialize(name, state.tx.clone()).await {
        Ok(ticket) => Json(ApiResponse {
            success: true,
            message: "P2P连接已初始化".to_string(),
            data: Some(serde_json::json!({ "ticket": ticket })),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("初始化P2P连接失败: {}", e),
            data: None,
        }),
    }
}

/// 加入P2P网络
async fn join(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<ApiResponse> {
    let ticket = match payload.get("ticket").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return Json(ApiResponse {
            success: false,
            message: "缺少票据".to_string(),
            data: None,
        }),
    };
    
    let name = payload.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    let mut p2p = state.p2p.write().await;
    match p2p.join(ticket, name, state.tx.clone()).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: "已加入P2P网络".to_string(),
            data: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("加入P2P网络失败: {}", e),
            data: None,
        }),
    }
}

/// 发送文本消息
async fn send_text(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<ApiResponse> {
    let content = match payload.get("content").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return Json(ApiResponse {
            success: false,
            message: "缺少消息内容".to_string(),
            data: None,
        }),
    };
    
    let p2p = state.p2p.read().await;
    match p2p.send_text(content.to_string(), &state.tx).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: "消息已发送".to_string(),
            data: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("发送消息失败: {}", e),
            data: None,
        }),
    }
}

/// 发送代理请求
async fn send_agent_request(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<ApiResponse> {
    let query = match payload.get("query").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return Json(ApiResponse {
            success: false,
            message: "缺少查询内容".to_string(),
            data: None,
        }),
    };
    
    let p2p = state.p2p.read().await;
    match p2p.send_agent_request(query.to_string(), &state.tx).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: "代理请求已发送".to_string(),
            data: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("发送代理请求失败: {}", e),
            data: None,
        }),
    }
}

/// 获取状态
async fn get_status(
    State(state): State<AppState>,
) -> Json<ApiResponse> {
    let p2p = state.p2p.read().await;
    let node_id = p2p.node_id.clone();
    let initialized = p2p.initialized;
    let topic_id = p2p.topic_id.map(|t| t.to_string());
    
    Json(ApiResponse {
        success: true,
        message: "状态获取成功".to_string(),
        data: Some(serde_json::json!({
            "node_id": node_id,
            "initialized": initialized,
            "topic_id": topic_id,
        })),
    })
}

/// Axum示例主函数
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    
    // 创建广播通道
    let (tx, _) = broadcast::channel::<WsMessage>(100);
    
    // 创建P2P状态
    let p2p_state = P2PState::new();
    
    // 创建应用状态
    let app_state = AppState {
        p2p: Arc::new(RwLock::new(p2p_state)),
        tx: tx.clone(),
    };
    
    // 创建CORS层
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // 创建路由
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/initialize", post(initialize))
        .route("/api/join", post(join))
        .route("/api/send_text", post(send_text))
        .route("/api/send_agent_request", post(send_agent_request))
        .route("/api/status", get(get_status))
        .layer(cors)
        .with_state(app_state);
    
    // 启动服务器
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("启动服务器，监听地址: {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;