//! Rig Agent - 基于 rig-core 的通用 Agent 模块
//!
//! 提供跨平台的 AI Agent 功能，支持 tauri, iroh 等不同运行环境

pub mod adapters;
pub mod core;
pub mod error;
pub mod tools;

// 重新导出核心类型和功能
pub use core::{
    AgentConfig, AgentManager, AgentMessage, AgentResponse, AgentRole, ClientConfig, 
    ConversationHistory, MessageType, ToolCall, ToolResult,
};

// 重新导出错误类型
pub use error::{AgentError, AgentResult, ErrorResponse};

// 重新导出工具
pub use tools::{BuiltinTools, CustomTool, ToolDefinition, ToolManager};

// 重新导出适配器
pub use adapters::{AgentAdapter, StandaloneAgentAdapter};

#[cfg(feature = "tauri-support")]
pub use adapters::TauriAgentAdapter;

/// 便捷的初始化函数
pub async fn init_agent_manager(config: AgentConfig) -> AgentResult<AgentManager> {
    Ok(AgentManager::new(config))
}

/// 配置构建器
pub struct ConfigBuilder {
    config: AgentConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
        }
    }

    pub fn model<S: Into<String>>(mut self, model: S) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn system_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.config.preamble = Some(prompt.into());
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.config.temperature = Some(temperature);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.config.max_tokens = Some(max_tokens);
        self
    }

    pub fn enable_tools(mut self, enable: bool) -> Self {
        self.config.enable_tools = enable;
        self
    }

    pub fn history_limit(mut self, limit: usize) -> Self {
        self.config.history_limit = Some(limit);
        self
    }

    pub fn build(self) -> AgentConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
