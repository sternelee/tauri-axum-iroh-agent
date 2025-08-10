//! P2P节点实现
//!
//! 提供P2P节点功能，用于处理iroh-gossip通信

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
    sync::Arc,
};

use bytes::Bytes;
use futures_lite::StreamExt;
use iroh_net::{
    key::{PublicKey, SecretKey},
    relay::RelayMode,
    endpoint::Endpoint,
    NodeAddr,
    magicsock::Watcher,
};
use iroh_gossip::{
    api::{Event, GossipReceiver, GossipSender},
    net::{Gossip, GOSSIP_ALPN},
    proto::topic::TopicId,
};
use rig_agent::{AgentConfig, AgentManager, AgentResponse, ClientConfig};
use rig_agent::core::ClientRegistry;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::{
    config::NodeConfig, error::NodeResult, fmt_relay_mode, MessageType, NodeStatus, SignedMessage,
    Ticket,
};

/// P2P节点
pub struct P2PNode {
    /// 节点配置
    config: NodeConfig,
    /// 节点端点
    endpoint: Endpoint,
    /// 节点密钥
    secret_key: SecretKey,
    /// 节点ID
    node_id: String,
    /// 节点名称
    name: Option<String>,
    /// 节点状态
    status: Arc<RwLock<NodeStatus>>,
    /// 活跃话题
    topics: Arc<RwLock<HashMap<TopicId, (GossipSender, GossipReceiver)>>>,
    /// Agent管理器
    agent_manager: Arc<RwLock<AgentManager>>,
    /// 客户端注册表
    client_registry: ClientRegistry,
    /// 消息处理器
    message_handlers: Arc<RwLock<HashMap<TopicId, mpsc::Sender<(PublicKey, MessageType)>>>>,
    /// 节点是否正在运行
    running: Arc<RwLock<bool>>,
}

impl P2PNode {
    /// 创建新的P2P节点
    pub async fn new(config: NodeConfig) -> NodeResult<Self> {
        // 解析或生成密钥
        let secret_key = match &config.secret_key {
            Some(key) => key.parse().map_err(|e| crate::error::NodeError::ConfigError(format!("解析密钥失败: {}", e)))?,
            None => SecretKey::generate(&mut rand::rngs::OsRng),
        };

        // 配置中继模式
        let relay_mode = match (config.no_relay, &config.relay) {
            (false, None) => RelayMode::Default,
            (false, Some(url)) => RelayMode::Custom(url.clone().into()),
            (true, None) => RelayMode::Disabled,
            (true, Some(_)) => {
                return Err(crate::error::NodeError::ConfigError(
                    "不能同时设置--no-relay和--relay".to_string(),
                ))
            }
        };

        // 构建端点
        let endpoint = Endpoint::builder()
            .secret_key(secret_key.clone())
            .relay_mode(relay_mode.clone())
            .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.bind_port))
            .bind()
            .await
            .map_err(|e| crate::error::NodeError::IrohError(e.to_string()))?;

        let node_id = endpoint.node_id().to_string();
        info!("节点ID: {}", node_id);
        info!("使用中继服务器: {}", fmt_relay_mode(&relay_mode));

        // 创建Agent管理器
        let agent_config = AgentConfig::default();
        let agent_manager = AgentManager::new(agent_config);
        let client_registry = ClientRegistry::new();

        // 创建节点状态
        let status = NodeStatus {
            node_id: node_id.clone(),
            node_addr: None,
            connected_peers: 0,
            active_topics: 0,
            started_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            relay_mode: fmt_relay_mode(&relay_mode),
        };

        Ok(Self {
            config,
            endpoint,
            secret_key,
            node_id,
            name: None,
            status: Arc::new(RwLock::new(status)),
            topics: Arc::new(RwLock::new(HashMap::new())),
            agent_manager: Arc::new(RwLock::new(agent_manager)),
            client_registry,
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// 启动节点
    pub async fn start(&self) -> NodeResult<()> {
        // 检查节点是否已经在运行
        {
            let running = self.running.read().await;
            if *running {
                info!("节点已经在运行");
                return Ok(());
            }
        }

        info!("启动P2P节点: {}", self.node_id);

        // 创建gossip协议
        let gossip = Gossip::builder().spawn(self.endpoint.clone());

        // 设置路由器
        let router = iroh_net::protocol::Router::builder(self.endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();

        // 更新节点地址
        let node_addr = self.endpoint.node_addr().initialized().await;
        {
            let mut status = self.status.write().await;
            status.node_addr = Some(node_addr.clone());
        }

        // 标记节点为运行状态
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        // 如果设置了名称，广播节点信息
        if let Some(name) = &self.name {
            info!("广播节点名称: {}", name);
            // 为每个活跃话题广播名称
            for (topic_id, (sender, _)) in self.topics.read().await.iter() {
                let message = MessageType::NodeInfo {
                    name: Some(name.clone()),
                };
                let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
                sender.broadcast(encoded_message).await
                    .map_err(|e| crate::error::NodeError::IrohError(e.to_string()))?;
            }
        }

        info!("P2P节点启动成功");
        Ok(())
    }

    /// 设置节点名称
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// 获取节点状态
    pub async fn get_status(&self) -> NodeStatus {
        self.status.read().await.clone()
    }

    /// 创建或加入话题
    pub async fn join_topic(&self, topic: Option<TopicId>, ticket: Option<&str>) -> NodeResult<(TopicId, String)> {
        // 检查节点是否在运行
        {
            let running = self.running.read().await;
            if !*running {
                return Err(crate::error::NodeError::ConfigError(
                    "节点未启动".to_string(),
                ));
            }
        }

        let (topic_id, peers) = match (topic, ticket) {
            (Some(topic), None) => {
                info!("创建话题: {}", topic);
                (topic, vec![])
            }
            (None, None) => {
                let topic = TopicId::from_bytes(rand::random());
                info!("创建新话题: {}", topic);
                (topic, vec![])
            }
            (_, Some(ticket_str)) => {
                let ticket = crate::Ticket::from_str(ticket_str)?;
                info!("加入话题: {}", ticket.topic);
                (ticket.topic, ticket.peers)
            }
        };

        // 检查是否已经加入该话题
        if self.topics.read().await.contains_key(&topic_id) {
            info!("已经加入话题: {}", topic_id);
            // 生成票据
            let ticket = self.generate_ticket(topic_id).await?;
            return Ok((topic_id, ticket));
        }

        // 创建gossip协议
        let gossip = Gossip::builder().spawn(self.endpoint.clone());

        // 连接到已知的对等节点
        let peer_ids = peers.iter().map(|p| p.node_id).collect();
        if peers.is_empty() {
            info!("等待对等节点加入...");
        } else {
            info!("尝试连接到{}个对等节点...", peers.len());
            // 将票据中的对等节点地址添加到端点的地址簿
            for peer in peers.iter() {
                self.endpoint.add_node_addr(peer.clone())
                    .map_err(|e| crate::error::NodeError::IrohError(e.to_string()))?;
            }
        }

        // 订阅话题
        let (sender, receiver) = gossip.subscribe_and_join(topic_id.clone(), peer_ids).await
            .map_err(|e| crate::error::NodeError::IrohError(e.to_string()))?
            .split();
        info!("已连接到话题: {}", topic_id);

        // 保存话题
        {
            let mut topics = self.topics.write().await;
            topics.insert(topic_id.clone(), (sender, receiver));

            // 更新状态
            let mut status = self.status.write().await;
            status.active_topics = topics.len();
            status.last_activity = chrono::Utc::now();
        }

        // 启动消息处理循环
        self.start_message_handler(topic_id.clone()).await?;

        // 生成票据
        let ticket = self.generate_ticket(topic_id).await?;

        Ok((topic_id, ticket))
    }

    /// 生成票据
    async fn generate_ticket(&self, topic_id: TopicId) -> NodeResult<String> {
        let me = self.endpoint.node_addr().initialized().await;
        let peers = vec![me];
        let ticket = Ticket {
            topic: topic_id,
            peers,
        };
        Ok(ticket.to_string())
    }

    /// 启动消息处理循环
    async fn start_message_handler(&self, topic_id: TopicId) -> NodeResult<()> {
        let topics = self.topics.read().await;
        let (_, receiver) = topics.get(&topic_id).ok_or_else(|| {
            crate::error::NodeError::TopicError(format!("话题不存在: {}", topic_id))
        })?;

        // 克隆接收器
        let mut receiver = receiver.clone();
        
        // 创建消息通道
        let (tx, mut rx) = mpsc::channel::<(PublicKey, MessageType)>(100);
        
        // 保存消息处理器
        {
            let mut handlers = self.message_handlers.write().await;
            handlers.insert(topic_id.clone(), tx);
        }

        // 克隆必要的引用
        let secret_key = self.secret_key.clone();
        let agent_manager = self.agent_manager.clone();
        let client_registry = &self.client_registry;
        let topics_ref = self.topics.clone();
        let topic_id_clone = topic_id.clone();
        let running = self.running.clone();

        // 启动接收消息的任务
        tokio::spawn(async move {
            info!("启动话题 {} 的消息处理循环", topic_id);
            
            while let Some(event) = receiver.try_next().await
                .map_err(|e| {
                    error!("接收消息错误: {}", e);
                    None
                })
                .unwrap_or(None) {
                // 检查节点是否仍在运行
                if !*running.read().await {
                    info!("节点已停止，终止消息处理循环");
                    break;
                }
                
                if let Event::Received(msg) = event {
                    match SignedMessage::verify_and_decode(&msg.content) {
                        Ok((from, message)) => {
                            debug!("收到来自 {} 的消息: {:?}", from.fmt_short(), message);
                            
                            // 发送到处理通道
                            if let Err(e) = tx.send((from, message)).await {
                                error!("发送消息到处理通道失败: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("验证消息失败: {}", e);
                        }
                    }
                }
            }
            
            info!("话题 {} 的消息处理循环结束", topic_id);
        });

        // 启动处理消息的任务
        tokio::spawn(async move {
            info!("启动话题 {} 的消息处理器", topic_id_clone);
            
            while let Some((from, message)) = rx.recv().await {
                // 检查节点是否仍在运行
                if !*running.read().await {
                    info!("节点已停止，终止消息处理器");
                    break;
                }
                
                match message {
                    MessageType::Chat { text } => {
                        debug!("收到聊天消息: {}", text);
                        // 这里可以添加聊天消息的处理逻辑
                    }
                    MessageType::AgentRequest { prompt, agent_id } => {
                        debug!("收到Agent请求: {}, agent_id: {}", prompt, agent_id);
                        
                        // 使用tokio::spawn处理异步请求，避免阻塞消息处理循环
                        let agent_manager_clone = agent_manager.clone();
                        let client_registry_ref = client_registry;
                        let secret_key_clone = secret_key.clone();
                        let topics_ref_clone = topics_ref.clone();
                        let topic_id_clone2 = topic_id_clone.clone();
                        let agent_id_clone = agent_id.clone();
                        let prompt_clone = prompt.clone();
                        
                        tokio::spawn(async move {
                            // 处理Agent请求
                            let response = match process_agent_request(&agent_manager_clone, client_registry_ref, &agent_id_clone, &prompt_clone).await {
                                Ok(resp) => {
                                    debug!("Agent请求处理成功，响应长度: {}", resp.content.len());
                                    MessageType::AgentResponse {
                                        content: resp.content,
                                        agent_id: agent_id_clone,
                                    }
                                },
                                Err(e) => {
                                    error!("处理Agent请求失败: {}", e);
                                    MessageType::Error {
                                        message: format!("处理Agent请求失败: {}", e),
                                    }
                                },
                            };
                            
                            // 发送响应
                            if let Some((sender, _)) = topics_ref_clone.read().await.get(&topic_id_clone2) {
                                match SignedMessage::sign_and_encode(&secret_key_clone, &response) {
                                    Ok(encoded) => {
                                        match sender.broadcast(encoded).await {
                                            Ok(_) => debug!("成功发送Agent响应"),
                                            Err(e) => error!("广播响应失败: {}", e),
                                        }
                                    },
                                    Err(e) => error!("编码响应失败: {}", e),
                                }
                            } else {
                                error!("话题 {} 不存在，无法发送响应", topic_id_clone2);
                            }
                        });
                    }
                    MessageType::NodeInfo { name } => {
                        if let Some(name) = name {
                            debug!("节点 {} 的名称: {}", from.fmt_short(), name);
                            // 这里可以保存节点名称
                        }
                    }
                    MessageType::AgentResponse { content, agent_id } => {
                        debug!("收到Agent响应: agent_id={}, 内容长度={}", agent_id, content.len());
                        // 这里可以处理Agent响应
                    }
                    MessageType::Error { message } => {
                        error!("收到错误消息: {}", message);
                        // 这里可以处理错误消息
                    }
                    MessageType::System { content } => {
                        info!("收到系统消息: {}", content);
                        // 这里可以处理系统消息
                    }
                }
            }
            
            info!("话题 {} 的消息处理器结束", topic_id_clone);
        });

        Ok(())
    }

    /// 发送消息到话题
    pub async fn send_message(&self, topic_id: &TopicId, message: MessageType) -> NodeResult<()> {
        // 检查节点是否在运行
        {
            let running = self.running.read().await;
            if !*running {
                return Err(crate::error::NodeError::ConfigError(
                    "节点未启动".to_string(),
                ));
            }
        }

        let topics = self.topics.read().await;
        let (sender, _) = topics.get(topic_id).ok_or_else(|| {
            crate::error::NodeError::TopicError(format!("话题不存在: {}", topic_id))
        })?;

        let encoded_message = SignedMessage::sign_and_encode(&self.secret_key, &message)?;
        sender.broadcast(encoded_message).await
            .map_err(|e| crate::error::NodeError::IrohError(e.to_string()))?;

        // 更新状态
        {
            let mut status = self.status.write().await;
            status.last_activity = chrono::Utc::now();
        }

        Ok(())
    }

    /// 发送Agent请求
    pub async fn send_agent_request(&self, topic_id: &TopicId, agent_id: &str, prompt: &str) -> NodeResult<()> {
        let message = MessageType::AgentRequest {
            prompt: prompt.to_string(),
            agent_id: agent_id.to_string(),
        };
        
        self.send_message(topic_id, message).await
    }

    /// 离开话题
    pub async fn leave_topic(&self, topic_id: &TopicId) -> NodeResult<()> {
        let mut topics = self.topics.write().await;
        if topics.remove(topic_id).is_some() {
            info!("已离开话题: {}", topic_id);
            
            // 更新状态
            let mut status = self.status.write().await;
            status.active_topics = topics.len();
            status.last_activity = chrono::Utc::now();
            
            // 移除消息处理器
            let mut handlers = self.message_handlers.write().await;
            handlers.remove(topic_id);
            
            Ok(())
        } else {
            Err(crate::error::NodeError::TopicError(format!(
                "话题不存在: {}",
                topic_id
            )))
        }
    }

    /// 停止节点
    pub async fn stop(&self) -> NodeResult<()> {
        info!("停止P2P节点: {}", self.node_id);
        
        // 标记节点为非运行状态
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        
        // 离开所有话题
        let topics = {
            let topics_read = self.topics.read().await;
            topics_read.keys().cloned().collect::<Vec<_>>()
        };
        
        for topic_id in topics {
            self.leave_topic(&topic_id).await?;
        }
        
        info!("P2P节点已停止");
        Ok(())
    }

    /// 获取节点ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 获取密钥
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// 获取Agent管理器
    pub async fn get_agent_manager(&self) -> tokio::sync::RwLockReadGuard<'_, AgentManager> {
        self.agent_manager.read().await
    }

    /// 获取可变Agent管理器
    pub async fn get_agent_manager_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, AgentManager> {
        self.agent_manager.write().await
    }

    /// 获取客户端注册表
    pub fn get_client_registry(&self) -> &ClientRegistry {
        &self.client_registry
    }
    
    /// 获取活跃话题列表
    pub async fn get_active_topics(&self) -> Vec<TopicId> {
        let topics = self.topics.read().await;
        topics.keys().cloned().collect()
    }
    
    /// 检查节点是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// 处理Agent请求
async fn process_agent_request(
    agent_manager: &Arc<RwLock<AgentManager>>,
    client_registry: &ClientRegistry,
    agent_id: &str,
    prompt: &str,
) -> NodeResult<AgentResponse> {
    // 检查Agent是否存在，如果不存在则创建
    {
        let manager = agent_manager.read().await;
        let agents = manager.list_agents().await;
        
        if !agents.contains(&agent_id.to_string()) {
            drop(manager); // 释放读锁
            
            let mut manager = agent_manager.write().await;
            manager.create_agent(agent_id.to_string(), None).await?;
        }
    }
    
    // 重新获取读锁并处理请求
    let manager = agent_manager.read().await;
    let response = manager
        .chat(client_registry, agent_id, prompt)
        .await
        .map_err(|e| crate::error::NodeError::AgentError(format!("Agent请求失败: {}", e)))?;
    
    Ok(response)
}