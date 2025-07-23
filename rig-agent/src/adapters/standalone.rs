//! 独立适配器实现

use crate::{
    core::{AgentConfig, AgentResponse, ConversationHistory},
    error::{AgentError, AgentResult},
    AgentManager,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 独立 Agent 适配器
pub struct StandaloneAgentAdapter {
    /// Agent 管理器
    manager: Arc<RwLock<AgentManager>>,
}

impl StandaloneAgentAdapter {
    /// 创建新的独立适配器
    pub fn new(default_config: AgentConfig) -> Self {
        let manager = Arc::new(RwLock::new(AgentManager::new(default_config)));
        
        Self { manager }
    }

    /// 获取 Agent 管理器
    pub async fn get_manager(&self) -> tokio::sync::RwLockReadGuard<'_, AgentManager> {
        self.manager.read().await
    }

    /// 获取可变 Agent 管理器
    pub async fn get_manager_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, AgentManager> {
        self.manager.write().await
    }

    /// 获取对话历史
    pub async fn get_conversation_history(&self, agent_id: &str) -> AgentResult<ConversationHistory> {
        let manager = self.manager.read().await;
        manager.get_conversation_history(agent_id).await
    }

    /// 清除对话历史
    pub async fn clear_conversation_history(&self, agent_id: &str) -> AgentResult<()> {
        let manager = self.manager.read().await;
        manager.clear_conversation_history(agent_id).await
    }

    /// 获取 Agent 配置
    pub async fn get_agent_config(&self, agent_id: &str) -> AgentResult<AgentConfig> {
        let manager = self.manager.read().await;
        manager.get_agent_config(agent_id).await
    }

    /// 更新 Agent 配置
    pub async fn update_agent_config(&self, agent_id: &str, config: AgentConfig) -> AgentResult<()> {
        let mut manager = self.manager.write().await;
        manager.update_agent_config(agent_id, config).await
    }

    /// 批量聊天
    pub async fn batch_chat(&self, requests: Vec<(String, String)>) -> Vec<(String, AgentResult<AgentResponse>)> {
        let manager = self.manager.read().await;
        let mut results = Vec::new();
        
        for (agent_id, message) in requests {
            let result = manager.chat(&agent_id, &message).await;
            results.push((agent_id, result));
        }
        
        results
    }

    /// 并发聊天
    pub async fn concurrent_chat(&self, requests: Vec<(String, String)>) -> Vec<(String, AgentResult<AgentResponse>)> {
        let manager = self.manager.clone();
        let tasks: Vec<_> = requests.into_iter().map(|(agent_id, message)| {
            let manager = manager.clone();
            let agent_id_clone = agent_id.clone();
            tokio::spawn(async move {
                let manager = manager.read().await;
                let result = manager.chat(&agent_id_clone, &message).await;
                (agent_id, result)
            })
        }).collect();

        let mut results = Vec::new();
        for task in tasks {
            if let Ok(result) = task.await {
                results.push(result);
            }
        }
        
        results
    }

    /// 获取统计信息
    pub async fn get_statistics(&self) -> AgentResult<AgentStatistics> {
        let manager = self.manager.read().await;
        let agents = manager.list_agents().await;
        let mut total_messages = 0;
        let mut total_tokens = 0;

        for agent_id in &agents {
            if let Ok(history) = manager.get_conversation_history(agent_id).await {
                total_messages += history.total_messages;
                total_tokens += history.total_tokens.unwrap_or(0);
            }
        }

        Ok(AgentStatistics {
            total_agents: agents.len(),
            total_messages,
            total_tokens,
            active_agents: agents.len(), // 简化实现，假设所有 Agent 都是活跃的
        })
    }
}

impl super::AgentAdapter for StandaloneAgentAdapter {
    async fn chat(&self, agent_id: &str, message: &str) -> AgentResult<AgentResponse> {
        let manager = self.manager.read().await;
        manager.chat(agent_id, message).await
    }

    async fn create_agent(&self, agent_id: String, config: Option<AgentConfig>) -> AgentResult<()> {
        let mut manager = self.manager.write().await;
        manager.create_agent(agent_id, config).await
    }

    async fn remove_agent(&self, agent_id: &str) -> AgentResult<bool> {
        let mut manager = self.manager.write().await;
        Ok(manager.remove_agent(agent_id).await)
    }

    async fn list_agents(&self) -> AgentResult<Vec<String>> {
        let manager = self.manager.read().await;
        Ok(manager.list_agents().await)
    }
}

/// Agent 统计信息
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentStatistics {
    /// 总 Agent 数量
    pub total_agents: usize,
    /// 总消息数量
    pub total_messages: usize,
    /// 总令牌数量
    pub total_tokens: u64,
    /// 活跃 Agent 数量
    pub active_agents: usize,
}

/// 简单 API 模块
pub mod simple_api {
    use super::*;
    use crate::core::AgentConfig;

    /// 创建默认 Agent 并发送消息
    pub async fn quick_chat(message: &str) -> AgentResult<AgentResponse> {
        let adapter = StandaloneAgentAdapter::new(AgentConfig::default());
        adapter.create_agent("default".to_string(), None).await?;
        adapter.chat("default", message).await
    }

    /// 创建自定义 Agent 并发送消息
    pub async fn custom_chat(agent_id: &str, message: &str, config: AgentConfig) -> AgentResult<AgentResponse> {
        let adapter = StandaloneAgentAdapter::new(config.clone());
        adapter.create_agent(agent_id.to_string(), Some(config)).await?;
        adapter.chat(agent_id, message).await
    }

    /// 批量处理消息
    pub async fn batch_process(messages: Vec<&str>) -> AgentResult<Vec<AgentResponse>> {
        let adapter = StandaloneAgentAdapter::new(AgentConfig::default());
        adapter.create_agent("batch".to_string(), None).await?;
        
        let mut responses = Vec::new();
        for message in messages {
            let response = adapter.chat("batch", message).await?;
            responses.push(response);
        }
        
        Ok(responses)
    }

    /// 对话式聊天
    pub async fn conversation_chat(agent_id: &str, messages: Vec<&str>, config: Option<AgentConfig>) -> AgentResult<Vec<AgentResponse>> {
        let adapter = StandaloneAgentAdapter::new(config.unwrap_or_default());
        adapter.create_agent(agent_id.to_string(), config).await?;
        
        let mut responses = Vec::new();
        for message in messages {
            let response = adapter.chat(agent_id, message).await?;
            responses.push(response);
        }
        
        Ok(responses)
    }
}

/// 配置构建器
pub struct StandaloneConfigBuilder {
    config: AgentConfig,
}

impl StandaloneConfigBuilder {
    /// 创建新的配置构建器
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
        }
    }

    /// 设置模型名称
    pub fn model(mut self, model: String) -> Self {
        self.config.model = model;
        self
    }

    /// 设置系统提示
    pub fn system_prompt(mut self, prompt: String) -> Self {
        self.config.system_prompt = Some(prompt);
        self
    }

    /// 设置温度
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.config.temperature = Some(temperature);
        self
    }

    /// 设置最大令牌数
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.config.max_tokens = Some(max_tokens);
        self
    }

    /// 启用工具
    pub fn enable_tools(mut self, enable: bool) -> Self {
        self.config.enable_tools = enable;
        self
    }

    /// 设置历史消息限制
    pub fn history_limit(mut self, limit: usize) -> Self {
        self.config.history_limit = Some(limit);
        self
    }

    /// 构建配置
    pub fn build(self) -> AgentConfig {
        self.config
    }

    /// 构建适配器
    pub fn build_adapter(self) -> StandaloneAgentAdapter {
        StandaloneAgentAdapter::new(self.config)
    }
}

impl Default for StandaloneConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_standalone_adapter_creation() {
        let config = AgentConfig::default();
        let adapter = StandaloneAgentAdapter::new(config);
        
        let agents = adapter.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = StandaloneConfigBuilder::new()
            .model("gpt-4".to_string())
            .temperature(0.7)
            .max_tokens(1000)
            .enable_tools(true)
            .build();
        
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_tokens, Some(1000));
        assert!(config.enable_tools);
    }

    #[tokio::test]
    async fn test_simple_api() {
        // 注意：这个测试需要有效的 API 密钥才能运行
        // let response = simple_api::quick_chat("你好").await;
        // assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_statistics() {
        let adapter = StandaloneAgentAdapter::new(AgentConfig::default());
        let stats = adapter.get_statistics().await.unwrap();
        
        assert_eq!(stats.total_agents, 0);
        assert_eq!(stats.total_messages, 0);
    }
}