//! Agent 错误处理模块

use std::fmt;
use thiserror::Error;

/// Agent 错误类型
#[derive(Error, Debug)]
pub enum AgentError {
    /// 配置错误
    #[error("配置错误: {0}")]
    Configuration(String),

    /// 模型错误
    #[error("模型错误: {0}")]
    ModelError(String),

    /// 网络错误
    #[error("网络错误: {0}")]
    Network(String),

    /// Agent 不存在
    #[error("Agent 不存在: {0}")]
    AgentNotFound(String),

    /// 工具错误
    #[error("工具错误: {0}")]
    ToolError(String),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 数据库错误
    #[error("数据库错误: {0}")]
    Database(String),

    /// 权限错误
    #[error("权限错误: {0}")]
    Permission(String),

    /// 限流错误
    #[error("请求过于频繁，请稍后再试")]
    RateLimit,

    /// 令牌不足错误
    #[error("令牌不足")]
    InsufficientTokens,

    /// 其他错误
    #[error("其他错误: {0}")]
    Other(String),
}

impl AgentError {
    /// 创建配置错误
    pub fn config<T: fmt::Display>(msg: T) -> Self {
        Self::Configuration(msg.to_string())
    }

    /// 创建模型错误
    pub fn model<T: fmt::Display>(msg: T) -> Self {
        Self::ModelError(msg.to_string())
    }

    /// 创建网络错误
    pub fn network<T: fmt::Display>(msg: T) -> Self {
        Self::Network(msg.to_string())
    }

    /// 创建工具错误
    pub fn tool<T: fmt::Display>(msg: T) -> Self {
        Self::ToolError(msg.to_string())
    }

    /// 创建数据库错误
    pub fn database<T: fmt::Display>(msg: T) -> Self {
        Self::Database(msg.to_string())
    }

    /// 创建权限错误
    pub fn permission<T: fmt::Display>(msg: T) -> Self {
        Self::Permission(msg.to_string())
    }

    /// 创建其他错误
    pub fn other<T: fmt::Display>(msg: T) -> Self {
        Self::Other(msg.to_string())
    }

    /// 检查是否为可重试的错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AgentError::Network(_) | AgentError::RateLimit | AgentError::Other(_)
        )
    }

    /// 获取错误代码
    pub fn error_code(&self) -> &'static str {
        match self {
            AgentError::Configuration(_) => "CONFIG_ERROR",
            AgentError::ModelError(_) => "MODEL_ERROR",
            AgentError::Network(_) => "NETWORK_ERROR",
            AgentError::AgentNotFound(_) => "AGENT_NOT_FOUND",
            AgentError::ToolError(_) => "TOOL_ERROR",
            AgentError::Serialization(_) => "SERIALIZATION_ERROR",
            AgentError::Io(_) => "IO_ERROR",
            AgentError::Database(_) => "DATABASE_ERROR",
            AgentError::Permission(_) => "PERMISSION_ERROR",
            AgentError::RateLimit => "RATE_LIMIT",
            AgentError::InsufficientTokens => "INSUFFICIENT_TOKENS",
            AgentError::Other(_) => "OTHER_ERROR",
        }
    }
}

/// Agent 结果类型别名
pub type AgentResult<T> = Result<T, AgentError>;

/// 将 anyhow::Error 转换为 AgentError
impl From<anyhow::Error> for AgentError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}

/// 错误响应结构
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 详细信息
    pub details: Option<String>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorResponse {
    /// 从 AgentError 创建错误响应
    pub fn from_error(error: &AgentError) -> Self {
        Self {
            code: error.error_code().to_string(),
            message: error.to_string(),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// 创建带详细信息的错误响应
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = AgentError::config("测试配置错误");
        assert_eq!(error.error_code(), "CONFIG_ERROR");
        assert!(error.to_string().contains("配置错误"));
    }

    #[test]
    fn test_error_retryable() {
        let network_error = AgentError::network("网络连接失败");
        assert!(network_error.is_retryable());

        let config_error = AgentError::config("配置错误");
        assert!(!config_error.is_retryable());
    }

    #[test]
    fn test_error_response() {
        let error = AgentError::model("模型调用失败");
        let response = ErrorResponse::from_error(&error);

        assert_eq!(response.code, "MODEL_ERROR");
        assert!(response.message.contains("模型错误"));
    }
}

