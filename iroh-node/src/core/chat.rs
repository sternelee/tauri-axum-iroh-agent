//! iroh P2P实时聊天功能

use chrono::{DateTime, Utc};
use iroh_gossip::{net::Gossip, proto::TopicId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 聊天消息类型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息ID
    pub id: String,
    /// 发送者ID
    pub sender_id: String,
    /// 发送者昵称
    pub sender_name: String,
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 发送时间
    pub timestamp: DateTime<Utc>,
    /// 聊天室ID
    pub room_id: String,
}

/// 消息类型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageType {
    /// 普通文本消息
    Text,
    /// 系统消息
    System,
    /// 用户加入
    UserJoined,
    /// 用户离开
    UserLeft,
    /// 文件分享
    FileShare { file_name: String, doc_ticket: String },
}

/// 聊天室信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRoom {
    /// 聊天室ID
    pub id: String,
    /// 聊天室名称
    pub name: String,
    /// 聊天室描述
    pub description: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 在线用户数
    pub online_users: u32,
    /// 主题ID（用于gossip）
    pub topic_id: TopicId,
}

/// 用户信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatUser {
    /// 用户ID
    pub id: String,
    /// 用户昵称
    pub name: String,
    /// 加入时间
    pub joined_at: DateTime<Utc>,
    /// 是否在线
    pub is_online: bool,
}

/// 聊天事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChatEvent {
    /// 收到新消息
    MessageReceived(ChatMessage),
    /// 用户加入聊天室
    UserJoined(ChatUser),
    /// 用户离开聊天室
    UserLeft { user_id: String, user_name: String },
    /// 聊天室创建
    RoomCreated(ChatRoom),
    /// 聊天室更新
    RoomUpdated(ChatRoom),
    /// 连接状态变化
    ConnectionChanged { connected: bool },
    /// 错误事件
    Error { message: String },
}

/// 聊天请求类型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// 聊天室ID
    pub room_id: String,
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: MessageType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    /// 聊天室ID
    pub room_id: String,
    /// 用户昵称
    pub user_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    /// 聊天室名称
    pub name: String,
    /// 聊天室描述
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaveRoomRequest {
    /// 聊天室ID
    pub room_id: String,
}

/// 聊天配置
#[derive(Clone, Debug)]
pub struct ChatConfig {
    /// 用户昵称
    pub user_name: String,
    /// 最大消息历史数量
    pub max_message_history: usize,
    /// 是否启用文件分享
    pub enable_file_sharing: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            user_name: format!("用户_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            max_message_history: 1000,
            enable_file_sharing: true,
        }
    }
}

impl ChatMessage {
    /// 创建新的文本消息
    pub fn new_text(sender_id: String, sender_name: String, content: String, room_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id,
            sender_name,
            content,
            message_type: MessageType::Text,
            timestamp: Utc::now(),
            room_id,
        }
    }

    /// 创建系统消息
    pub fn new_system(content: String, room_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id: "system".to_string(),
            sender_name: "系统".to_string(),
            content,
            message_type: MessageType::System,
            timestamp: Utc::now(),
            room_id,
        }
    }

    /// 创建文件分享消息
    pub fn new_file_share(
        sender_id: String,
        sender_name: String,
        file_name: String,
        doc_ticket: String,
        room_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender_id,
            sender_name,
            content: format!("分享了文件: {}", file_name),
            message_type: MessageType::FileShare { file_name, doc_ticket },
            timestamp: Utc::now(),
            room_id,
        }
    }
}

impl ChatRoom {
    /// 创建新的聊天室
    pub fn new(name: String, description: Option<String>) -> Self {
        let id = Uuid::new_v4().to_string();
        let topic_id = TopicId::from(id.as_bytes());
        
        Self {
            id,
            name,
            description,
            created_at: Utc::now(),
            online_users: 0,
            topic_id,
        }
    }
}

impl ChatUser {
    /// 创建新用户
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            joined_at: Utc::now(),
            is_online: true,
        }
    }
}