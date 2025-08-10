//! iroh-node库
//!
//! 提供P2P通信功能，用于在tauri和axum中集成，并与rig-agent服务交互

mod config;
mod error;
mod p2p;

pub mod adapters;

use std::{fmt, str::FromStr};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use ed25519_dalek::Signature;
use iroh_net::{
    key::{PublicKey, SecretKey}, 
    relay::RelayMode, 
    NodeAddr
};
use iroh_gossip::proto::topic::TopicId;
use serde::{Deserialize, Serialize};

pub use crate::{
    config::NodeConfig,
    error::{NodeError, NodeResult},
    p2p::P2PNode,
};

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    /// 节点ID
    pub node_id: String,
    /// 节点地址
    pub node_addr: Option<NodeAddr>,
    /// 连接的对等节点数量
    pub connected_peers: usize,
    /// 活跃话题数量
    pub active_topics: usize,
    /// 启动时间
    pub started_at: DateTime<Utc>,
    /// 最后活动时间
    pub last_activity: DateTime<Utc>,
    /// 中继模式
    pub relay_mode: String,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// 聊天消息
    Chat {
        /// 消息文本
        text: String,
    },
    /// 节点信息
    NodeInfo {
        /// 节点名称
        name: Option<String>,
    },
    /// Agent请求
    AgentRequest {
        /// 提示词
        prompt: String,
        /// Agent ID
        agent_id: String,
    },
    /// Agent响应
    AgentResponse {
        /// 响应内容
        content: String,
        /// Agent ID
        agent_id: String,
    },
    /// 错误消息
    Error {
        /// 错误信息
        message: String,
    },
    /// 系统消息
    System {
        /// 系统消息内容
        content: String,
    },
}

/// 签名消息
#[derive(Debug, Serialize, Deserialize)]
pub struct SignedMessage {
    /// 发送者公钥
    from: PublicKey,
    /// 消息数据
    data: Bytes,
    /// 签名
    signature: Signature,
}

impl SignedMessage {
    /// 验证并解码消息
    pub fn verify_and_decode(bytes: &[u8]) -> NodeResult<(PublicKey, MessageType)> {
        let signed_message: Self = postcard::from_bytes(bytes)
            .map_err(|e| NodeError::DecodeError(format!("解码签名消息失败: {}", e)))?;
        
        let key: PublicKey = signed_message.from;
        key.verify(&signed_message.data, &signed_message.signature)
            .map_err(|e| NodeError::VerifyError(format!("验证签名失败: {}", e)))?;
        
        let message: MessageType = postcard::from_bytes(&signed_message.data)
            .map_err(|e| NodeError::DecodeError(format!("解码消息内容失败: {}", e)))?;
        
        Ok((signed_message.from, message))
    }

    /// 签名并编码消息
    pub fn sign_and_encode(secret_key: &SecretKey, message: &MessageType) -> NodeResult<Bytes> {
        let data: Bytes = postcard::to_stdvec(message)
            .map_err(|e| NodeError::EncodeError(format!("编码消息失败: {}", e)))?
            .into();
        
        let signature = secret_key.sign(&data);
        let from: PublicKey = secret_key.public();
        
        let signed_message = Self {
            from,
            data,
            signature,
        };
        
        let encoded = postcard::to_stdvec(&signed_message)
            .map_err(|e| NodeError::EncodeError(format!("编码签名消息失败: {}", e)))?;
        
        Ok(encoded.into())
    }
}

/// 票据
#[derive(Debug, Serialize, Deserialize)]
pub struct Ticket {
    /// 话题ID
    pub topic: TopicId,
    /// 对等节点地址
    pub peers: Vec<NodeAddr>,
}

impl Ticket {
    /// 从字节反序列化
    fn from_bytes(bytes: &[u8]) -> NodeResult<Self> {
        postcard::from_bytes(bytes)
            .map_err(|e| NodeError::DecodeError(format!("解码票据失败: {}", e)))
    }
    
    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("postcard::to_stdvec is infallible")
    }
}

/// 序列化为base32
impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{text}")
    }
}

/// 从base32反序列化
impl FromStr for Ticket {
    type Err = NodeError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32_NOPAD
            .decode(s.to_ascii_uppercase().as_bytes())
            .map_err(|e| NodeError::DecodeError(format!("解码base32失败: {}", e)))?;
        
        Self::from_bytes(&bytes)
    }
}

/// 格式化中继模式
pub(crate) fn fmt_relay_mode(relay_mode: &RelayMode) -> String {
    match relay_mode {
        RelayMode::Disabled => "无".to_string(),
        RelayMode::Default => "默认中继(生产)服务器".to_string(),
        RelayMode::Staging => "默认中继(测试)服务器".to_string(),
        RelayMode::Custom(map) => map
            .urls()
            .map(|url| url.to_string())
            .collect::<Vec<_>>()
            .join(" "),
    }
}