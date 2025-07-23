//! Tauri 适配器实现

use crate::{
    core::{AgentConfig, AgentResponse},
    error::{AgentError, AgentResult},
    AgentManager,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tauri 事件发射器特征
pub trait TauriEventEmitter: Send + Sync {
    /// 发射事件到前端
    fn emit_event(&self, event_name: &str, payload: serde_json::Value);
}

/// Tauri Agent 适配器
pub struct TauriAgentAdapter<E: TauriEventEmitter> {
    /// Agent 管理器
    manager: Arc<RwLock<AgentManager>>,
    /// 事件发射器
    event_emitter: Arc<E>,
}

impl<E: TauriEventEmitter> TauriAgentAdapter<E> {
    /// 创建新的 Tauri 适配器
    pub fn new(default_config: AgentConfig, event_emitter: Arc<E>) -> Self {
        let manager = Arc::new(RwLock::new(AgentManager::new(default_config)));
        
        Self {
            manager,
            event_emitter,
        }
    }

    /// 获取 Agent 管理器
    pub async fn get_manager(&self) -> tokio::sync::RwLockReadGuard<'_, AgentManager> {
        self.manager.read().await
    }

    /// 获取可变 Agent 管理器
    pub async fn get_manager_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, AgentManager> {
        self.manager.write().await
    }

    /// 发送聊天消息并发射事件
    pub async fn chat_with_events(&self, agent_id: &str, message: &str) -> AgentResult<AgentResponse> {
        // 发射开始聊天事件
        self.event_emitter.emit_event("agent-chat-start", serde_json::json!({
            "agent_id": agent_id,
            "message": message,
            "timestamp": chrono::Utc::now()
        }));

        let manager = self.manager.read().await;
        let result = manager.chat(agent_id, message).await;

        match &result {
            Ok(response) => {
                // 发射聊天成功事件
                self.event_emitter.emit_event("agent-chat-response", serde_json::json!({
                    "agent_id": agent_id,
                    "response": response,
                    "timestamp": chrono::Utc::now()
                }));
            }
            Err(error) => {
                // 发射聊天错误事件
                self.event_emitter.emit_event("agent-chat-error", serde_json::json!({
                    "agent_id": agent_id,
                    "error": error.to_string(),
                    "timestamp": chrono::Utc::now()
                }));
            }
        }

        result
    }

    /// 创建 Agent 并发射事件
    pub async fn create_agent_with_events(&self, agent_id: String, config: Option<AgentConfig>) -> AgentResult<()> {
        let mut manager = self.manager.write().await;
        let result = manager.create_agent(agent_id.clone(), config.clone()).await;

        match &result {
            Ok(_) => {
                // 发射 Agent 创建成功事件
                self.event_emitter.emit_event("agent-created", serde_json::json!({
                    "agent_id": agent_id,
                    "config": config,
                    "timestamp": chrono::Utc::now()
                }));
            }
            Err(error) => {
                // 发射 Agent 创建失败事件
                self.event_emitter.emit_event("agent-create-error", serde_json::json!({
                    "agent_id": agent_id,
                    "error": error.to_string(),
                    "timestamp": chrono::Utc::now()
                }));
            }
        }

        result
    }

    /// 删除 Agent 并发射事件
    pub async fn remove_agent_with_events(&self, agent_id: &str) -> AgentResult<bool> {
        let mut manager = self.manager.write().await;
        let result = manager.remove_agent(agent_id).await;

        if result {
            // 发射 Agent 删除事件
            self.event_emitter.emit_event("agent-removed", serde_json::json!({
                "agent_id": agent_id,
                "timestamp": chrono::Utc::now()
            }));
        }

        Ok(result)
    }

    /// 获取对话历史并发射事件
    pub async fn get_conversation_history_with_events(&self, agent_id: &str) -> AgentResult<crate::core::ConversationHistory> {
        let manager = self.manager.read().await;
        let result = manager.get_conversation_history(agent_id).await;

        match &result {
            Ok(history) => {
                // 发射历史获取成功事件
                self.event_emitter.emit_event("agent-history-loaded", serde_json::json!({
                    "agent_id": agent_id,
                    "message_count": history.total_messages,
                    "timestamp": chrono::Utc::now()
                }));
            }
            Err(error) => {
                // 发射历史获取失败事件
                self.event_emitter.emit_event("agent-history-error", serde_json::json!({
                    "agent_id": agent_id,
                    "error": error.to_string(),
                    "timestamp": chrono::Utc::now()
                }));
            }
        }

        result
    }

    /// 清除对话历史并发射事件
    pub async fn clear_conversation_history_with_events(&self, agent_id: &str) -> AgentResult<()> {
        let manager = self.manager.read().await;
        let result = manager.clear_conversation_history(agent_id).await;

        match &result {
            Ok(_) => {
                // 发射历史清除成功事件
                self.event_emitter.emit_event("agent-history-cleared", serde_json::json!({
                    "agent_id": agent_id,
                    "timestamp": chrono::Utc::now()
                }));
            }
            Err(error) => {
                // 发射历史清除失败事件
                self.event_emitter.emit_event("agent-history-clear-error", serde_json::json!({
                    "agent_id": agent_id,
                    "error": error.to_string(),
                    "timestamp": chrono::Utc::now()
                }));
            }
        }

        result
    }
}

impl<E: TauriEventEmitter> super::AgentAdapter for TauriAgentAdapter<E> {
    async fn chat(&self, agent_id: &str, message: &str) -> AgentResult<AgentResponse> {
        self.chat_with_events(agent_id, message).await
    }

    async fn create_agent(&self, agent_id: String, config: Option<AgentConfig>) -> AgentResult<()> {
        self.create_agent_with_events(agent_id, config).await
    }

    async fn remove_agent(&self, agent_id: &str) -> AgentResult<bool> {
        self.remove_agent_with_events(agent_id).await
    }

    async fn list_agents(&self) -> AgentResult<Vec<String>> {
        let manager = self.manager.read().await;
        Ok(manager.list_agents().await)
    }
}

/// Tauri 命令请求类型
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub agent_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub agent_id: String,
    pub config: Option<AgentConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentIdRequest {
    pub agent_id: String,
}

/// Tauri 命令响应类型
#[derive(Debug, Serialize, Deserialize)]
pub struct TauriResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> TauriResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl<T> From<AgentResult<T>> for TauriResponse<T> {
    fn from(result: AgentResult<T>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(error) => Self::error(error.to_string()),
        }
    }
}

/// 便捷的 Tauri 命令函数
pub mod commands {
    use super::*;

    /// 聊天命令
    pub async fn agent_chat<E: TauriEventEmitter>(
        adapter: tauri::State<'_, TauriAgentAdapter<E>>,
        request: ChatRequest,
    ) -> Result<TauriResponse<AgentResponse>, String> {
        let result = adapter.chat_with_events(&request.agent_id, &request.message).await;
        Ok(TauriResponse::from(result))
    }

    /// 创建 Agent 命令
    pub async fn create_agent<E: TauriEventEmitter>(
        adapter: tauri::State<'_, TauriAgentAdapter<E>>,
        request: CreateAgentRequest,
    ) -> Result<TauriResponse<()>, String> {
        let result = adapter.create_agent_with_events(request.agent_id, request.config).await;
        Ok(TauriResponse::from(result))
    }

    /// 删除 Agent 命令
    pub async fn remove_agent<E: TauriEventEmitter>(
        adapter: tauri::State<'_, TauriAgentAdapter<E>>,
        request: AgentIdRequest,
    ) -> Result<TauriResponse<bool>, String> {
        let result = adapter.remove_agent_with_events(&request.agent_id).await;
        Ok(TauriResponse::from(result))
    }

    /// 获取 Agent 列表命令
    pub async fn list_agents<E: TauriEventEmitter>(
        adapter: tauri::State<'_, TauriAgentAdapter<E>>,
    ) -> Result<TauriResponse<Vec<String>>, String> {
        let result = adapter.list_agents().await;
        Ok(TauriResponse::from(result))
    }

    /// 获取对话历史命令
    pub async fn get_conversation_history<E: TauriEventEmitter>(
        adapter: tauri::State<'_, TauriAgentAdapter<E>>,
        request: AgentIdRequest,
    ) -> Result<TauriResponse<crate::core::ConversationHistory>, String> {
        let result = adapter.get_conversation_history_with_events(&request.agent_id).await;
        Ok(TauriResponse::from(result))
    }

    /// 清除对话历史命令
    pub async fn clear_conversation_history<E: TauriEventEmitter>(
        adapter: tauri::State<'_, TauriAgentAdapter<E>>,
        request: AgentIdRequest,
    ) -> Result<TauriResponse<()>, String> {
        let result = adapter.clear_conversation_history_with_events(&request.agent_id).await;
        Ok(TauriResponse::from(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockEventEmitter;

    impl TauriEventEmitter for MockEventEmitter {
        fn emit_event(&self, _event_name: &str, _payload: serde_json::Value) {
            // Mock implementation
        }
    }

    #[tokio::test]
    async fn test_tauri_adapter_creation() {
        let config = AgentConfig::default();
        let emitter = Arc::new(MockEventEmitter);
        let adapter = TauriAgentAdapter::new(config, emitter);
        
        let agents = adapter.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }
}