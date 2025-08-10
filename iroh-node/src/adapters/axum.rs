//! Axum适配器
//!
//! 提供Axum适配器，用于在Axum应用中集成P2P节点

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use iroh_gossip::proto::topic::TopicId;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::{MessageType, NodeConfig, NodeError, NodeResult, NodeStatus, P2PNode};

/// Axum适配器
pub struct AxumAdapter {
    /// P2P节点
    node: Arc<RwLock<Option<P2PNode>>>,
}

/// 节点状态响应
#[derive(Debug, Serialize)]
pub struct NodeStatusResponse {
    /// 节点ID
    pub node_id: String,
    /// 连接的对等节点数量
    pub connected_peers: usize,
    /// 活跃话题数量
    pub active_topics: usize,
    /// 启动时间
    pub started_at: String,
    /// 最后活动时间
    pub last_activity: String,
    /// 中继模式
    pub relay_mode: String,
}

/// 话题响应
#[derive(Debug, Serialize)]
pub struct TopicResponse {
    /// 话题ID
    pub topic_id: String,
    /// 票据
    pub ticket: String,
}

/// 初始化请求
#[derive(Debug, Deserialize)]
pub struct InitRequest {
    /// 密钥
    pub secret_key: Option<String>,
    /// 中继服务器URL
    pub relay: Option<String>,
    /// 禁用中继
    pub no_relay: Option<bool>,
    /// 节点名称
    pub name: Option<String>,
    /// 绑定端口
    pub bind_port: Option<u16>,
}

/// 创建话题请求
#[derive(Debug, Deserialize)]
pub struct CreateTopicRequest {
    /// 话题ID
    pub topic_id: Option<String>,
}

/// 加入话题请求
#[derive(Debug, Deserialize)]
pub struct JoinTopicRequest {
    /// 票据
    pub ticket: String,
}

/// 消息请求
#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    /// 消息内容
    pub message: String,
}

/// Agent请求
#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    /// Agent ID
    pub agent_id: String,
    /// 提示词
    pub prompt: String,
}

/// API错误
#[derive(Debug, Serialize)]
pub struct ApiError {
    /// 错误消息
    pub message: String,
}

impl AxumAdapter {
    /// 创建新的Axum适配器
    pub fn new() -> Self {
        Self {
            node: Arc::new(RwLock::new(None)),
        }
    }

    /// 创建Axum路由
    pub fn create_router(&self) -> Router {
        let node = self.node.clone();

        Router::new()
            .route("/api/node", post(init_node))
            .route("/api/node/status", get(get_node_status))
            .route("/api/topics", post(create_topic))
            .route("/api/topics/join", post(join_topic))
            .route("/api/topics/:topic_id/messages", post(send_message))
            .route("/api/topics/:topic_id/agent", post(send_agent_request))
            .route("/api/topics/:topic_id", get(get_topic_info))
            .route("/api/topics/:topic_id", delete(leave_topic))
            .route("/api/node", delete(stop_node))
            .with_state(node)
    }
}

impl Default for AxumAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// 将NodeError转换为API响应
impl IntoResponse for NodeError {
    fn into_response(self) -> Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let error = ApiError {
            message: self.to_string(),
        };

        (status, Json(error)).into_response()
    }
}

/// 初始化P2P节点
async fn init_node(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
    Json(request): Json<InitRequest>,
) -> Result<Json<String>, NodeError> {
    // 检查节点是否已经初始化
    {
        let node_read = node.read().await;
        if node_read.is_some() {
            return Err(NodeError::ConfigError("节点已经初始化".to_string()));
        }
    }

    // 解析中继URL
    let relay_url = match request.relay {
        Some(url) => Some(
            url.parse()
                .map_err(|e| NodeError::ConfigError(format!("解析中继URL失败: {}", e)))?,
        ),
        None => None,
    };

    // 创建节点配置
    let config = NodeConfig {
        secret_key: request.secret_key,
        relay: relay_url,
        no_relay: request.no_relay.unwrap_or(false),
        name: request.name.clone(),
        bind_port: request.bind_port.unwrap_or(0),
    };

    // 创建P2P节点
    let mut p2p_node = P2PNode::new(config).await?;

    // 设置节点名称
    if let Some(name) = request.name {
        p2p_node.set_name(name);
    }

    // 启动节点
    p2p_node.start().await?;

    let node_id = p2p_node.node_id().to_string();

    // 保存节点
    {
        let mut node_write = node.write().await;
        *node_write = Some(p2p_node);
    }

    info!("P2P节点已初始化: {}", node_id);
    Ok(Json(node_id))
}

/// 获取节点状态
async fn get_node_status(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
) -> Result<Json<NodeStatusResponse>, NodeError> {
    let node_read = node.read().await;
    let node = node_read
        .as_ref()
        .ok_or_else(|| NodeError::ConfigError("节点未初始化".to_string()))?;

    let status = node.get_status().await;

    Ok(Json(NodeStatusResponse {
        node_id: status.node_id,
        connected_peers: status.connected_peers,
        active_topics: status.active_topics,
        started_at: status.started_at.to_rfc3339(),
        last_activity: status.last_activity.to_rfc3339(),
        relay_mode: status.relay_mode,
    }))
}

/// 创建话题
async fn create_topic(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
    Json(request): Json<CreateTopicRequest>,
) -> Result<Json<TopicResponse>, NodeError> {
    let node_read = node.read().await;
    let node = node_read
        .as_ref()
        .ok_or_else(|| NodeError::ConfigError("节点未初始化".to_string()))?;

    // 解析话题ID
    let topic = match request.topic_id {
        Some(id) => Some(
            id.parse()
                .map_err(|e| NodeError::TopicError(format!("解析话题ID失败: {}", e)))?,
        ),
        None => None,
    };

    // 创建话题
    let (topic, ticket) = node.join_topic(topic, None).await?;

    info!("已创建话题: {}", topic);
    Ok(Json(TopicResponse {
        topic_id: topic.to_string(),
        ticket,
    }))
}

/// 加入话题
async fn join_topic(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
    Json(request): Json<JoinTopicRequest>,
) -> Result<Json<TopicResponse>, NodeError> {
    let node_read = node.read().await;
    let node = node_read
        .as_ref()
        .ok_or_else(|| NodeError::ConfigError("节点未初始化".to_string()))?;

    // 加入话题
    let (topic, ticket) = node.join_topic(None, Some(&request.ticket)).await?;

    info!("已加入话题: {}", topic);
    Ok(Json(TopicResponse {
        topic_id: topic.to_string(),
        ticket,
    }))
}

/// 发送消息
async fn send_message(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
    Path(topic_id): Path<String>,
    Json(request): Json<MessageRequest>,
) -> Result<(), NodeError> {
    let node_read = node.read().await;
    let node = node_read
        .as_ref()
        .ok_or_else(|| NodeError::ConfigError("节点未初始化".to_string()))?;

    // 解析话题ID
    let topic_id = topic_id
        .parse()
        .map_err(|e| NodeError::TopicError(format!("解析话题ID失败: {}", e)))?;

    // 创建消息
    let message = MessageType::Chat {
        text: request.message,
    };

    // 发送消息
    node.send_message(&topic_id, message).await?;

    info!("已发送消息到话题: {}", topic_id);
    Ok(())
}

/// 发送Agent请求
async fn send_agent_request(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
    Path(topic_id): Path<String>,
    Json(request): Json<AgentRequest>,
) -> Result<(), NodeError> {
    let node_read = node.read().await;
    let node = node_read
        .as_ref()
        .ok_or_else(|| NodeError::ConfigError("节点未初始化".to_string()))?;

    // 解析话题ID
    let topic_id = topic_id
        .parse()
        .map_err(|e| NodeError::TopicError(format!("解析话题ID失败: {}", e)))?;

    // 发送Agent请求
    node.send_agent_request(&topic_id, &request.agent_id, &request.prompt)
        .await?;

    info!(
        "已发送Agent请求到话题: {}, agent_id: {}",
        topic_id, request.agent_id
    );
    Ok(())
}

/// 获取话题信息
async fn get_topic_info(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
    Path(topic_id): Path<String>,
) -> Result<Json<TopicResponse>, NodeError> {
    let node_read = node.read().await;
    let node = node_read
        .as_ref()
        .ok_or_else(|| NodeError::ConfigError("节点未初始化".to_string()))?;

    // 解析话题ID
    let topic_id = topic_id
        .parse()
        .map_err(|e| NodeError::TopicError(format!("解析话题ID失败: {}", e)))?;

    // 检查话题是否存在
    let active_topics = node.get_active_topics().await;
    if !active_topics.contains(&topic_id) {
        return Err(NodeError::TopicError(format!("话题不存在: {}", topic_id)));
    }

    // 生成票据
    let ticket = node
        .generate_ticket(topic_id.clone())
        .await
        .map_err(|e| NodeError::TopicError(format!("生成票据失败: {}", e)))?;

    Ok(Json(TopicResponse {
        topic_id: topic_id.to_string(),
        ticket,
    }))
}

/// 离开话题
async fn leave_topic(
    State(node): State<Arc<RwLock<Option<P2PNode>>>>,
    Path(topic_id): Path<String>,
) -> Result<(), NodeError> {
    let node_read = node.read().await;
    let node = node_read
        .as_ref()
        .ok_or_else(|| NodeError::ConfigError("节点未初始化".to_string()))?;

    // 解析话题ID
    let topic_id = topic_id
        .parse()
        .map_err(|e| NodeError::TopicError(format!("解析话题ID失败: {}", e)))?;

    // 离开话题
    node.leave_topic(&topic_id).await?;

    info!("已离开话题: {}", topic_id);
    Ok(())
}

/// 停止节点
async fn stop_node(State(node): State<Arc<RwLock<Option<P2PNode>>>>) -> Result<(), NodeError> {
    let node_option = {
        let mut node_write = node.write().await;
        node_write.take()
    };

    if let Some(node) = node_option {
        // 停止节点
        node.stop().await?;

        info!("P2P节点已停止");
        Ok(())
    } else {
        Err(NodeError::ConfigError("节点未初始化".to_string()))
    }
}