//! 错误类型定义

use std::fmt;

/// 节点结果类型
pub type NodeResult<T> = Result<T, NodeError>;

/// 节点错误
#[derive(Debug)]
pub enum NodeError {
    /// 配置错误
    ConfigError(String),
    /// Iroh错误
    IrohError(String),
    /// 话题错误
    TopicError(String),
    /// Agent错误
    AgentError(String),
    /// 编码错误
    EncodeError(String),
    /// 解码错误
    DecodeError(String),
    /// 验证错误
    VerifyError(String),
    /// IO错误
    IoError(String),
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigError(msg) => write!(f, "配置错误: {}", msg),
            Self::IrohError(msg) => write!(f, "Iroh错误: {}", msg),
            Self::TopicError(msg) => write!(f, "话题错误: {}", msg),
            Self::AgentError(msg) => write!(f, "Agent错误: {}", msg),
            Self::EncodeError(msg) => write!(f, "编码错误: {}", msg),
            Self::DecodeError(msg) => write!(f, "解码错误: {}", msg),
            Self::VerifyError(msg) => write!(f, "验证错误: {}", msg),
            Self::IoError(msg) => write!(f, "IO错误: {}", msg),
        }
    }
}

impl std::error::Error for NodeError {}

impl From<std::io::Error> for NodeError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<rig_agent::Error> for NodeError {
    fn from(err: rig_agent::Error) -> Self {
        Self::AgentError(err.to_string())
    }
}