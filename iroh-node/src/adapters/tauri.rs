//! Tauri适配器
//!
//! 提供Tauri插件，用于在Tauri应用中集成P2P节点

use std::sync::Arc;

use iroh_gossip::proto::topic::TopicId;
use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::{MessageType, NodeConfig, NodeResult, P2PNode};

/// P2P节点状态
struct P2PNodeState {
    /// P2P节点
    node: Arc<RwLock<Option<P2PNode>>>,
}

/// Tauri插件
pub struct TauriAdapter<R: Runtime> {
    /// 插件
    plugin: TauriPlugin<R>,
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

/// 消息请求
#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    /// 话题ID
    pub topic_id: String,
    /// 消息内容
    pub message: String,
}

/// Agent请求
#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    /// 话题ID
    pub topic_id: String,
    /// Agent ID
    pub agent_id: String,
    /// 提示词
    pub prompt: String,
}

impl<R: Runtime> TauriAdapter<R> {
    /// 创建新的Tauri适配器
    pub fn new() -> Self {
        let plugin = Builder::new("iroh-node")
            .invoke_handler(tauri::generate_handler![
                init_node,
                get_node_status,
                create_topic,
                join_topic,
                send_message,
                send_agent_request,
                leave_topic,
                stop_node
            ])
            .setup(|app| {
                // 初始化P2P节点状态
                app.manage(P2PNodeState {
                    node: Arc::new(RwLock::new(None)),
                });
                Ok(())
            })
            .build();

        Self { plugin }
    }

    /// 获取Tauri插件
    pub fn plugin(&self) -> TauriPlugin<R> {
        self.plugin.clone()
    }
}

/// 初始化P2P节点
#[tauri::command]
async fn init_node(
    app: AppHandle,
    state: State<'_, P2PNodeState>,
    secret_key: Option<String>,
    relay: Option<String>,
    no_relay: Option<bool>,
    name: Option<String>,
    bind_port: Option<u16>,
) -> Result<String, String> {
    // 检查节点是否已经初始化
    {
        let node = state.node.read().await;
        if node.is_some() {
            return Err("节点已经初始化".to_string());
        }
    }

    // 解析中继URL
    let relay_url = match relay {
        Some(url) => {
            Some(url.parse().map_err(|e| format!("解析中继URL失败: {}", e))?)
        }
        None => None,
    };

    // 创建节点配置
    let config = NodeConfig {
        secret_key,
        relay: relay_url,
        no_relay: no_relay.unwrap_or(false),
        name: name.clone(),
        bind_port: bind_port.unwrap_or(0),
    };

    // 创建P2P节点
    let mut node = P2PNode::new(config)
        .await
        .map_err(|e| format!("创建节点失败: {}", e))?;

    // 设置节点名称
    if let Some(name) = name {
        node.set_name(name);
    }

    // 启动节点
    node.start()
        .await
        .map_err(|e| format!("启动节点失败: {}", e))?;

    let node_id = node.node_id().to_string();

    // 保存节点
    {
        let mut node_state = state.node.write().await;
        *node_state = Some(node);
    }

    // 发送节点初始化事件
    app.emit_all("iroh-node://initialized", node_id.clone())
        .map_err(|e| format!("发送事件失败: {}", e))?;

    Ok(node_id)
}

/// 获取节点状态
#[tauri::command]
async fn get_node_status(state: State<'_, P2PNodeState>) -> Result<NodeStatusResponse, String> {
    let node = state.node.read().await;
    let node = node
        .as_ref()
        .ok_or_else(|| "节点未初始化".to_string())?;

    let status = node.get_status().await;

    Ok(NodeStatusResponse {
        node_id: status.node_id,
        connected_peers: status.connected_peers,
        active_topics: status.active_topics,
        started_at: status.started_at.to_rfc3339(),
        last_activity: status.last_activity.to_rfc3339(),
        relay_mode: status.relay_mode,
    })
}

/// 创建话题
#[tauri::command]
async fn create_topic(
    app: AppHandle,
    state: State<'_, P2PNodeState>,
    topic_id: Option<String>,
) -> Result<TopicResponse, String> {
    let node = state.node.read().await;
    let node = node
        .as_ref()
        .ok_or_else(|| "节点未初始化".to_string())?;

    // 解析话题ID
    let topic = match topic_id {
        Some(id) => {
            Some(id.parse().map_err(|e| format!("解析话题ID失败: {}", e))?)
        }
        None => None,
    };

    // 创建话题
    let (topic, ticket) = node
        .join_topic(topic, None)
        .await
        .map_err(|e| format!("创建话题失败: {}", e))?;

    // 发送话题创建事件
    app.emit_all("iroh-node://topic-created", topic.to_string())
        .map_err(|e| format!("发送事件失败: {}", e))?;

    Ok(TopicResponse {
        topic_id: topic.to_string(),
        ticket,
    })
}

/// 加入话题
#[tauri::command]
async fn join_topic(
    app: AppHandle,
    state: State<'_, P2PNodeState>,
    ticket: String,
) -> Result<TopicResponse, String> {
    let node = state.node.read().await;
    let node = node
        .as_ref()
        .ok_or_else(|| "节点未初始化".to_string())?;

    // 加入话题
    let (topic, ticket) = node
        .join_topic(None, Some(&ticket))
        .await
        .map_err(|e| format!("加入话题失败: {}", e))?;

    // 发送话题加入事件
    app.emit_all("iroh-node://topic-joined", topic.to_string())
        .map_err(|e| format!("发送事件失败: {}", e))?;

    Ok(TopicResponse {
        topic_id: topic.to_string(),
        ticket,
    })
}

/// 发送消息
#[tauri::command]
async fn send_message(
    app: AppHandle,
    state: State<'_, P2PNodeState>,
    request: MessageRequest,
) -> Result<(), String> {
    let node = state.node.read().await;
    let node = node
        .as_ref()
        .ok_or_else(|| "节点未初始化".to_string())?;

    // 解析话题ID
    let topic_id = request
        .topic_id
        .parse()
        .map_err(|e| format!("解析话题ID失败: {}", e))?;

    // 创建消息
    let message = MessageType::Chat {
        text: request.message,
    };

    // 发送消息
    node.send_message(&topic_id, message)
        .await
        .map_err(|e| format!("发送消息失败: {}", e))?;

    // 发送消息发送事件
    app.emit_all("iroh-node://message-sent", topic_id.to_string())
        .map_err(|e| format!("发送事件失败: {}", e))?;

    Ok(())
}

/// 发送Agent请求
#[tauri::command]
async fn send_agent_request(
    app: AppHandle,
    state: State<'_, P2PNodeState>,
    request: AgentRequest,
) -> Result<(), String> {
    let node = state.node.read().await;
    let node = node
        .as_ref()
        .ok_or_else(|| "节点未初始化".to_string())?;

    // 解析话题ID
    let topic_id = request
        .topic_id
        .parse()
        .map_err(|e| format!("解析话题ID失败: {}", e))?;

    // 发送Agent请求
    node.send_agent_request(&topic_id, &request.agent_id, &request.prompt)
        .await
        .map_err(|e| format!("发送Agent请求失败: {}", e))?;

    // 发送Agent请求发送事件
    app.emit_all(
        "iroh-node://agent-request-sent",
        format!("{}:{}", topic_id, request.agent_id),
    )
    .map_err(|e| format!("发送事件失败: {}", e))?;

    Ok(())
}

/// 离开话题
#[tauri::command]
async fn leave_topic(
    app: AppHandle,
    state: State<'_, P2PNodeState>,
    topic_id: String,
) -> Result<(), String> {
    let node = state.node.read().await;
    let node = node
        .as_ref()
        .ok_or_else(|| "节点未初始化".to_string())?;

    // 解析话题ID
    let topic_id = topic_id
        .parse()
        .map_err(|e| format!("解析话题ID失败: {}", e))?;

    // 离开话题
    node.leave_topic(&topic_id)
        .await
        .map_err(|e| format!("离开话题失败: {}", e))?;

    // 发送话题离开事件
    app.emit_all("iroh-node://topic-left", topic_id.to_string())
        .map_err(|e| format!("发送事件失败: {}", e))?;

    Ok(())
}

/// 停止节点
#[tauri::command]
async fn stop_node(app: AppHandle, state: State<'_, P2PNodeState>) -> Result<(), String> {
    let node_option = {
        let mut node_state = state.node.write().await;
        node_state.take()
    };

    if let Some(node) = node_option {
        // 停止节点
        node.stop()
            .await
            .map_err(|e| format!("停止节点失败: {}", e))?;

        // 发送节点停止事件
        app.emit_all("iroh-node://stopped", ())
            .map_err(|e| format!("发送事件失败: {}", e))?;

        Ok(())
    } else {
        Err("节点未初始化".to_string())
    }
}