//! 适配器模块，支持不同运行环境

pub mod tauri_adapter;
pub mod standalone;

pub use tauri_adapter::TauriAgentAdapter;
pub use standalone::StandaloneAgentAdapter;

/// 通用适配器特征
pub trait AgentAdapter {
    /// 发送聊天消息
    async fn chat(&self, agent_id: &str, message: &str) -> crate::error::AgentResult<crate::core::AgentResponse>;

    /// 创建新的 Agent
    async fn create_agent(&self, agent_id: String, config: Option<crate::core::AgentConfig>) -> crate::error::AgentResult<()>;

    /// 删除 Agent
    async fn remove_agent(&self, agent_id: &str) -> crate::error::AgentResult<bool>;

    /// 获取 Agent 列表
    async fn list_agents(&self) -> crate::error::AgentResult<Vec<String>>;
}
