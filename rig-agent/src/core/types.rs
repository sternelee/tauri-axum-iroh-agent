//! Agent 核心类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 提供商名称
    pub provider: String,
    /// 默认模型
    pub default_model: String,
    /// API 密钥（可选，从环境变量获取）
    pub api_key: Option<String>,
    /// 基础 URL（可选，用于自定义端点）
    pub base_url: Option<String>,
    /// 其他配置参数
    pub extra_params: std::collections::HashMap<String, serde_json::Value>,
}

impl ClientConfig {
    /// 创建新的客户端配置
    pub fn new<S: Into<String>>(provider: S, default_model: S) -> Self {
        Self {
            provider: provider.into(),
            default_model: default_model.into(),
            api_key: None,
            base_url: None,
            extra_params: std::collections::HashMap::new(),
        }
    }

    /// 设置 API 密钥
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// 设置基础 URL
    pub fn with_base_url<S: Into<String>>(mut self, base_url: S) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// 添加额外参数
    pub fn with_param<S: Into<String>, V: Into<serde_json::Value>>(mut self, key: S, value: V) -> Self {
        self.extra_params.insert(key.into(), value.into());
        self
    }
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// AI 模型名称
    pub model: String,
    /// Provider 名称 (openai, anthropic, cohere, gemini)
    pub provider: String,
    /// 系统提示/前言
    pub preamble: Option<String>,
    /// 温度参数 (0.0-2.0)
    pub temperature: Option<f32>,
    /// 最大令牌数
    pub max_tokens: Option<u32>,
    /// 是否启用工具
    pub enable_tools: bool,
    /// 历史消息限制
    pub history_limit: Option<usize>,
    /// 其他配置参数
    pub extra_params: std::collections::HashMap<String, serde_json::Value>,
}

impl AgentConfig {
    /// 创建新的 Agent 配置
    pub fn new<S: Into<String>>(provider: S, model: S) -> Self {
        Self {
            model: model.into(),
            provider: provider.into(),
            preamble: Some("你是一个有用的AI助手。".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(1000),
            enable_tools: false,
            history_limit: Some(50),
            extra_params: std::collections::HashMap::new(),
        }
    }

    /// 设置系统提示
    pub fn with_preamble<S: Into<String>>(mut self, preamble: S) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    /// 设置温度
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 设置最大令牌数
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// 启用工具
    pub fn with_tools(mut self, enable: bool) -> Self {
        self.enable_tools = enable;
        self
    }

    /// 设置历史限制
    pub fn with_history_limit(mut self, limit: usize) -> Self {
        self.history_limit = Some(limit);
        self
    }

    /// 添加额外参数
    pub fn with_param<S: Into<String>, V: Into<serde_json::Value>>(mut self, key: S, value: V) -> Self {
        self.extra_params.insert(key.into(), value.into());
        self
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::new("openai", "gpt-3.5-turbo")
    }
}

/// Agent 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// 响应 ID
    pub id: String,
    /// Agent ID
    pub agent_id: String,
    /// 响应内容
    pub content: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 使用的模型
    pub model: String,
    /// 使用统计
    pub usage: Option<TokenUsage>,
    /// 工具调用
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 完成原因
    pub finish_reason: Option<String>,
}

/// 令牌使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 提示令牌数
    pub prompt_tokens: u32,
    /// 完成令牌数
    pub completion_tokens: u32,
    /// 总令牌数
    pub total_tokens: u32,
}

/// 对话历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    /// Agent ID
    pub agent_id: String,
    /// 消息列表
    pub messages: Vec<AgentMessage>,
    /// 总消息数
    pub total_messages: usize,
    /// 总令牌数
    pub total_tokens: Option<u64>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活动时间
    pub last_activity: DateTime<Utc>,
}

/// Agent 消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    /// 系统角色
    System,
    /// 用户角色
    User,
    /// 助手角色
    Assistant,
    /// 工具角色
    Tool,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// 文本消息
    Text,
    /// 工具调用
    ToolCall,
    /// 工具结果
    ToolResult,
    /// 系统消息
    System,
    /// 错误消息
    Error,
}

/// 工具调用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON 格式）
    pub arguments: String,
    /// 调用时间
    pub timestamp: DateTime<Utc>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 对应的工具调用 ID
    pub call_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 执行结果
    pub result: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 执行时间
    pub timestamp: DateTime<Utc>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

/// Agent 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// 消息角色
    pub role: AgentRole,
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 工具调用（如果有）
    pub tool_calls: Vec<ToolCall>,
    /// 工具结果（如果有）
    pub tool_results: Vec<ToolResult>,
}

impl AgentMessage {
    /// 创建用户消息
    pub fn user(content: String) -> Self {
        Self {
            role: AgentRole::User,
            content,
            message_type: MessageType::Text,
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    /// 创建助手消息
    pub fn assistant(content: String) -> Self {
        Self {
            role: AgentRole::Assistant,
            content,
            message_type: MessageType::Text,
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    /// 创建系统消息
    pub fn system(content: String) -> Self {
        Self {
            role: AgentRole::System,
            content,
            message_type: MessageType::System,
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    /// 创建工具调用消息
    pub fn tool_call(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: AgentRole::Assistant,
            content: "正在调用工具...".to_string(),
            message_type: MessageType::ToolCall,
            timestamp: Utc::now(),
            tool_calls,
            tool_results: Vec::new(),
        }
    }

    /// 创建工具结果消息
    pub fn tool_result(tool_results: Vec<ToolResult>) -> Self {
        let content = tool_results
            .iter()
            .map(|r| format!("工具 {} 执行结果: {}", r.tool_name, r.result))
            .collect::<Vec<_>>()
            .join("\n");

        Self {
            role: AgentRole::Tool,
            content,
            message_type: MessageType::ToolResult,
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
            tool_results,
        }
    }

    /// 创建错误消息
    pub fn error(error: String) -> Self {
        Self {
            role: AgentRole::System,
            content: format!("错误: {}", error),
            message_type: MessageType::Error,
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    /// 获取消息的令牌估算数量
    pub fn estimated_tokens(&self) -> u32 {
        // 简单的令牌估算：大约 4 个字符 = 1 个令牌
        (self.content.len() as u32 + 3) / 4
    }

    /// 检查消息是否包含工具调用
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// 检查消息是否包含工具结果
    pub fn has_tool_results(&self) -> bool {
        !self.tool_results.is_empty()
    }

    /// 获取消息的简短描述
    pub fn summary(&self) -> String {
        let role_str = match self.role {
            AgentRole::System => "系统",
            AgentRole::User => "用户",
            AgentRole::Assistant => "助手",
            AgentRole::Tool => "工具",
        };

        let content_preview = if self.content.len() > 50 {
            format!("{}...", &self.content[..50])
        } else {
            self.content.clone()
        };

        format!("[{}] {}", role_str, content_preview)
    }
}

/// 聊天会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// 会话 ID
    pub id: String,
    /// 会话标题
    pub title: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 消息数量
    pub message_count: usize,
    /// 使用的模型
    pub model: String,
    /// 会话标签
    pub tags: Vec<String>,
}

impl ChatSession {
    /// 创建新的聊天会话
    pub fn new(title: String, model: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            created_at: now,
            updated_at: now,
            message_count: 0,
            model,
            tags: Vec::new(),
        }
    }

    /// 更新会话
    pub fn update(&mut self, message_count: usize) {
        self.updated_at = Utc::now();
        self.message_count = message_count;
    }

    /// 添加标签
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// 移除标签
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_creation() {
        let user_msg = AgentMessage::user("你好".to_string());
        assert_eq!(user_msg.role, AgentRole::User);
        assert_eq!(user_msg.content, "你好");
        assert_eq!(user_msg.message_type, MessageType::Text);
    }

    #[test]
    fn test_message_token_estimation() {
        let msg = AgentMessage::user("这是一个测试消息".to_string());
        let tokens = msg.estimated_tokens();
        assert!(tokens > 0);
    }

    #[test]
    fn test_chat_session_creation() {
        let session = ChatSession::new("测试会话".to_string(), "gpt-3.5-turbo".to_string());
        assert_eq!(session.title, "测试会话");
        assert_eq!(session.model, "gpt-3.5-turbo");
        assert_eq!(session.message_count, 0);
    }

    #[test]
    fn test_message_summary() {
        let msg = AgentMessage::user(
            "这是一个很长的消息内容，用来测试摘要功能是否正常工作，应该会被截断显示".to_string(),
        );
        let summary = msg.summary();
        assert!(summary.contains("[用户]"));
        assert!(summary.contains("..."));
    }
}

