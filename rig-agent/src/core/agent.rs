//! 核心 Agent 实现 - 基于 rig-core

use crate::core::types::{
    AgentConfig, AgentMessage, AgentResponse, ClientConfig, ConversationHistory,
};
use crate::error::{AgentError, AgentResult};
use crate::tools::ToolManager;
use rig::{
    client::builder::DynClientBuilder,
    completion::{Chat, Prompt},
    message::Message,
};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

/// 客户端注册表，管理多个 AI 提供商客户端
pub struct ClientRegistry {
    builder: DynClientBuilder,
    /// 已注册的客户端配置
    clients: HashMap<String, ClientConfig>,
}

impl ClientRegistry {
    /// 创建新的客户端注册表
    pub fn new() -> Self {
        let mut registry = Self {
            builder: DynClientBuilder::new(),
            clients: HashMap::new(),
        };
        registry.register_default_clients();
        registry
    }

    /// 注册默认客户端
    fn register_default_clients(&mut self) {
        // 注册 OpenAI 客户端
        if std::env::var("OPENAI_API_KEY").is_ok() {
            let config = ClientConfig {
                provider: "openai".to_string(),
                default_model: "gpt-3.5-turbo".to_string(),
                api_key: None,
                base_url: None,
                extra_params: std::collections::HashMap::new(),
            };
            let _ = self.register_openai(config);
        }

        // 注册 Anthropic 客户端
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            let config = ClientConfig {
                provider: "anthropic".to_string(),
                default_model: "claude-3-sonnet-20240229".to_string(),
                api_key: None,
                base_url: None,
                extra_params: std::collections::HashMap::new(),
            };
            let _ = self.register_anthropic(config);
        }

        // 注册 Gemini 客户端
        if std::env::var("GEMINI_API_KEY").is_ok() {
            let config = ClientConfig {
                provider: "gemini".to_string(),
                default_model: "gemini-pro".to_string(),
                api_key: None,
                base_url: None,
                extra_params: std::collections::HashMap::new(),
            };
            let _ = self.register_gemini(config);
        }
    }

    /// 注册客户端
    pub fn register_client(&mut self, provider: &str, config: ClientConfig) -> AgentResult<()> {
        info!("注册 {} 客户端: {}", provider, config.default_model);
        self.clients.insert(provider.to_string(), config);
        Ok(())
    }

    /// 注册 OpenAI 客户端
    pub fn register_openai(&mut self, config: ClientConfig) -> AgentResult<()> {
        self.register_client("openai", config)
    }

    /// 注册 Anthropic 客户端
    pub fn register_anthropic(&mut self, config: ClientConfig) -> AgentResult<()> {
        self.register_client("anthropic", config)
    }

    /// 注册 Gemini 客户端
    pub fn register_gemini(&mut self, config: ClientConfig) -> AgentResult<()> {
        self.register_client("gemini", config)
    }

    /// 注册 Cohere 客户端
    pub fn register_cohere(&mut self, config: ClientConfig) -> AgentResult<()> {
        self.register_client("cohere", config)
    }

    /// 创建 Agent 实例
    pub fn create_agent<'a>(
        &'a self,
        config: &'a AgentConfig,
    ) -> AgentResult<rig::agent::Agent<rig::client::completion::CompletionModelHandle<'a>>> {
        let provider = &config.provider;

        info!("创建 Agent 实例: {} - {}", provider, config.model);

        // 检查客户端是否已注册
        if !self.clients.contains_key(provider) {
            return Err(AgentError::config(format!(
                "提供商 {} 未注册，请先注册客户端",
                provider
            )));
        }

        // 使用构建器
        let mut agent_builder = self
            .builder
            .agent(provider, &config.model)
            .map_err(|e| AgentError::config(format!("创建 {} 客户端失败: {}", provider, e)))?;

        // 应用配置参数
        if let Some(preamble) = &config.preamble {
            agent_builder = agent_builder.preamble(preamble);
        }

        if let Some(temperature) = config.temperature {
            agent_builder = agent_builder.temperature(temperature as f64);
        }

        if let Some(max_tokens) = config.max_tokens {
            agent_builder = agent_builder.max_tokens(max_tokens as u64);
        }

        let agent = agent_builder.build();
        info!("Agent 实例创建成功: {} - {}", provider, config.model);

        Ok(agent)
    }

    /// 获取已注册的客户端列表
    pub fn get_registered_clients(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    /// 检查客户端是否已注册
    pub fn has_client(&self, provider: &str) -> bool {
        self.clients.contains_key(provider)
    }

    /// 获取客户端配置
    pub fn get_client_config(&self, provider: &str) -> Option<&ClientConfig> {
        self.clients.get(provider)
    }
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent 信息结构体
pub struct Agent {
    id: String,
    config: AgentConfig,
    conversation_history: Vec<Message>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
}

/// Agent 管理器，负责创建和管理 Agent 实例
pub struct AgentManager {
    agents: RwLock<HashMap<String, Agent>>,
    default_config: AgentConfig,
    tool_manager: ToolManager,
}

impl AgentManager {
    /// 创建新的 Agent 管理器
    pub fn new(default_config: AgentConfig) -> Self {
        let tool_manager = ToolManager::new();

        Self {
            default_config,
            agents: RwLock::new(HashMap::new()),
            tool_manager,
        }
    }

    /// 创建新的 Agent
    pub async fn create_agent(
        &self,
        agent_id: String,
        config: Option<AgentConfig>,
    ) -> AgentResult<()> {
        let mut agents = self.agents.write().await;

        if agents.contains_key(&agent_id) {
            return Err(AgentError::other(format!("Agent 已存在: {}", agent_id)));
        }

        let agent_config = config.unwrap_or_else(|| self.default_config.clone());

        agents.insert(
            agent_id.clone(),
            Agent {
                id: agent_id.clone(),
                config: agent_config,
                conversation_history: Vec::new(),
                created_at: chrono::Utc::now(),
                last_activity: chrono::Utc::now(),
            },
        );

        info!("创建新 Agent: {}", agent_id);
        Ok(())
    }

    /// 删除 Agent
    pub async fn remove_agent(&self, agent_id: &str) -> bool {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id).is_some()
    }

    /// 获取 Agent 列表
    pub async fn list_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }

    /// 获取 Agent 列表及其提供商信息
    pub async fn list_agents_with_providers(&self) -> Vec<(String, String)> {
        let agents = self.agents.read().await;
        agents
            .iter()
            .map(|(id, agent)| (id.clone(), agent.config.provider.clone()))
            .collect()
    }

    /// 发送聊天消息
    #[instrument(skip(self, registry, message), fields(agent_id = %agent_id, message_len = message.len()))]
    pub async fn chat(
        &self,
        registry: &ClientRegistry,
        agent_id: &str,
        message: &str,
    ) -> AgentResult<AgentResponse> {
        let start_time = std::time::Instant::now();
        info!(
            "开始处理聊天消息，Agent: {}, 消息长度: {}",
            agent_id,
            message.len()
        );

        let mut agents = self.agents.write().await;
        let agent_data = agents.get_mut(agent_id).ok_or_else(|| {
            error!("Agent 不存在: {}", agent_id);
            AgentError::AgentNotFound(agent_id.to_string())
        })?;

        // 动态创建 agent
        let agent = registry.create_agent(&agent_data.config)?;

        // 更新最后活动时间
        agent_data.last_activity = chrono::Utc::now();
        debug!("更新 Agent {} 最后活动时间", agent_id);

        // 创建用户消息
        let user_message = Message::user(message);
        agent_data.conversation_history.push(user_message.clone());
        debug!(
            "添加用户消息到对话历史，当前历史长度: {}",
            agent_data.conversation_history.len()
        );

        // 调用 rig-core AI 模型
        debug!(
            "准备调用 AI 模型 ({}/{})",
            agent_data.config.provider, agent_data.config.model
        );
        let ai_start_time = std::time::Instant::now();

        // 使用对话历史进行聊天
        let response = agent
            .chat(user_message, agent_data.conversation_history.clone())
            .await
            .map_err(|e| AgentError::other(format!("AI 模型调用失败: {}", e)))?;

        let ai_duration = ai_start_time.elapsed();
        info!(
            "AI 模型调用完成，Agent: {}, 提供商: {}, 模型: {}, 耗时: {:?}",
            agent_id, agent_data.config.provider, agent_data.config.model, ai_duration
        );

        debug!("AI 响应内容长度: {}", response.len());

        // 创建助手消息并添加到历史
        let assistant_message = Message::assistant(&response);
        agent_data.conversation_history.push(assistant_message);

        // 应用历史限制
        if let Some(limit) = agent_data.config.history_limit {
            if agent_data.conversation_history.len() > limit {
                let excess = agent_data.conversation_history.len() - limit;
                agent_data.conversation_history.drain(0..excess);
            }
        }

        let total_duration = start_time.elapsed();
        let response_id = uuid::Uuid::new_v4().to_string();

        info!(
            "聊天消息处理完成，Agent: {}, 响应ID: {}, 总耗时: {:?}, 响应长度: {}",
            agent_id,
            response_id,
            total_duration,
            response.len()
        );

        Ok(AgentResponse {
            id: response_id,
            agent_id: agent_id.to_string(),
            content: response,
            timestamp: chrono::Utc::now(),
            model: agent_data.config.model.clone(),
            usage: None,      // TODO: 从 rig-core 获取使用统计
            tool_calls: None, // TODO: 处理工具调用
            finish_reason: Some("stop".to_string()),
        })
    }

    /// 简单的 prompt 方法（不保存历史）
    #[instrument(skip(self, registry, message), fields(agent_id = %agent_id, message_len = message.len()))]
    pub async fn prompt(
        &self,
        registry: &ClientRegistry,
        agent_id: &str,
        message: &str,
    ) -> AgentResult<String> {
        let agents = self.agents.read().await;
        let agent_data = agents.get(agent_id).ok_or_else(|| {
            error!("Agent 不存在: {}", agent_id);
            AgentError::AgentNotFound(agent_id.to_string())
        })?;

        // 动态创建 agent
        let agent = registry.create_agent(&agent_data.config)?;

        debug!("准备调用 AI 模型进行简单 prompt");
        let ai_start_time = std::time::Instant::now();

        // 使用 prompt 方法，不保存历史
        let response = agent
            .prompt(message)
            .await
            .map_err(|e| AgentError::other(format!("AI 模型调用失败: {}", e)))?;

        let ai_duration = ai_start_time.elapsed();
        info!(
            "简单 prompt 完成，Agent: {}, 提供商: {}, 模型: {}, 耗时: {:?}",
            agent_id, agent_data.config.provider, agent_data.config.model, ai_duration
        );

        Ok(response)
    }

    /// 使用指定提供商和模型创建临时 Agent 并执行 prompt
    pub async fn prompt_with(
        &self,
        registry: &ClientRegistry,
        provider: &str,
        model: &str,
        message: &str,
    ) -> AgentResult<String> {
        // 检查提供商是否已注册
        if !registry.has_client(provider) {
            return Err(AgentError::config(format!(
                "提供商 {} 未注册，请先注册客户端",
                provider
            )));
        }

        // 创建临时配置
        let config = AgentConfig::new(provider, model);

        // 创建临时 Agent
        let agent = registry.create_agent(&config)?;

        debug!("准备使用临时 Agent 调用 AI 模型进行 prompt");
        let ai_start_time = std::time::Instant::now();

        // 使用 prompt 方法
        let response = agent
            .prompt(message)
            .await
            .map_err(|e| AgentError::other(format!("AI 模型调用失败: {}", e)))?;

        let ai_duration = ai_start_time.elapsed();
        info!(
            "临时 Agent prompt 完成，提供商: {}, 模型: {}, 耗时: {:?}",
            provider, model, ai_duration
        );

        Ok(response)
    }

    /// 获取对话历史
    pub async fn get_conversation_history(
        &self,
        agent_id: &str,
    ) -> AgentResult<ConversationHistory> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        // 将 rig Message 转换为我们的 AgentMessage
        let messages: Vec<AgentMessage> = agent
            .conversation_history
            .iter()
            .map(|msg| match msg {
                Message::User { content, .. } => {
                    // 提取文本内容
                    let text = content
                        .iter()
                        .filter_map(|c| match c {
                            rig::message::UserContent::Text(text) => Some(text.text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    AgentMessage::user(text)
                }
                Message::Assistant { content, .. } => {
                    // 提取文本内容
                    let text = content
                        .iter()
                        .filter_map(|c| match c {
                            rig::message::AssistantContent::Text(text) => Some(text.text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    AgentMessage::assistant(text)
                }
            })
            .collect();

        let total_tokens = messages.iter().map(|msg| msg.content.len() as u64).sum();

        Ok(ConversationHistory {
            agent_id: agent_id.to_string(),
            messages,
            total_messages: agent.conversation_history.len(),
            total_tokens: Some(total_tokens),
            created_at: agent.created_at,
            last_activity: agent.last_activity,
        })
    }

    /// 获取 Agent 的提供商信息
    pub async fn get_agent_provider(&self, agent_id: &str) -> AgentResult<String> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        Ok(agent.config.provider.clone())
    }

    /// 清除对话历史
    pub async fn clear_conversation_history(&self, agent_id: &str) -> AgentResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        agent.conversation_history.clear();
        agent.last_activity = chrono::Utc::now();
        Ok(())
    }

    /// 获取 Agent 配置
    pub async fn get_agent_config(&self, agent_id: &str) -> AgentResult<AgentConfig> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        Ok(agent.config.clone())
    }

    /// 更新 Agent 配置
    pub async fn update_agent_config(
        &self,
        agent_id: &str,
        config: AgentConfig,
    ) -> AgentResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        // 只更新配置
        agent.config = config;
        agent.last_activity = chrono::Utc::now();
        Ok(())
    }

    /// 切换 Agent 的提供商和模型
    pub async fn switch_provider(
        &self,
        agent_id: &str,
        provider: &str,
        model: &str,
    ) -> AgentResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        // 创建新配置，保留原有的其他设置
        let mut new_config = agent.config.clone();
        new_config.provider = provider.to_string();
        new_config.model = model.to_string();

        // 只更新配置
        agent.config = new_config;
        agent.last_activity = chrono::Utc::now();

        info!("Agent {} 已切换到 {}/{}", agent_id, provider, model);
        Ok(())
    }

    /// 获取工具管理器
    pub fn get_tool_manager(&self) -> &ToolManager {
        &self.tool_manager
    }

    /// 获取可变工具管理器
    pub fn get_tool_manager_mut(&mut self) -> &mut ToolManager {
        &mut self.tool_manager
    }

    /// 获取 Agent 统计信息
    pub async fn get_agent_stats(&self, agent_id: &str) -> AgentResult<AgentStats> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        let total_messages = agent.conversation_history.len();
        let user_messages = agent
            .conversation_history
            .iter()
            .filter(|msg| matches!(msg, Message::User { .. }))
            .count();
        let assistant_messages = agent
            .conversation_history
            .iter()
            .filter(|msg| matches!(msg, Message::Assistant { .. }))
            .count();

        Ok(AgentStats {
            agent_id: agent_id.to_string(),
            provider: agent.config.provider.clone(),
            model: agent.config.model.clone(),
            total_messages,
            user_messages,
            assistant_messages,
            created_at: agent.created_at,
            last_activity: agent.last_activity,
            uptime: chrono::Utc::now().signed_duration_since(agent.created_at),
        })
    }

    /// 获取所有 Agent 的统计信息
    pub async fn get_all_agent_stats(&self) -> Vec<AgentStats> {
        let agents = self.agents.read().await;
        let mut stats = Vec::with_capacity(agents.len());

        for (agent_id, agent) in agents.iter() {
            let total_messages = agent.conversation_history.len();
            let user_messages = agent
                .conversation_history
                .iter()
                .filter(|msg| matches!(msg, Message::User { .. }))
                .count();
            let assistant_messages = agent
                .conversation_history
                .iter()
                .filter(|msg| matches!(msg, Message::Assistant { .. }))
                .count();

            stats.push(AgentStats {
                agent_id: agent_id.clone(),
                provider: agent.config.provider.clone(),
                model: agent.config.model.clone(),
                total_messages,
                user_messages,
                assistant_messages,
                created_at: agent.created_at,
                last_activity: agent.last_activity,
                uptime: chrono::Utc::now().signed_duration_since(agent.created_at),
            });
        }

        stats
    }
}

/// Agent 统计信息
#[derive(Debug, Clone)]
pub struct AgentStats {
    pub agent_id: String,
    pub provider: String,
    pub model: String,
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub uptime: chrono::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_manager_creation() {
        let config = AgentConfig::default();
        let manager = AgentManager::new(config);

        let agents = manager.list_agents().await;
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_client_registry() {
        let mut registry = ClientRegistry::new();

        // 注册客户端
        registry
            .register_openai(ClientConfig {
                provider: "openai".to_string(),
                default_model: "gpt-3.5-turbo".to_string(),
                api_key: None,
                base_url: None,
                extra_params: std::collections::HashMap::new(),
            })
            .unwrap();

        let clients = registry.get_registered_clients();
        assert!(clients.contains(&"openai".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_providers() {
        let mut registry = ClientRegistry::new();

        // 注册多个客户端
        registry
            .register_openai(ClientConfig {
                provider: "openai".to_string(),
                default_model: "gpt-3.5-turbo".to_string(),
                api_key: None,
                base_url: None,
                extra_params: std::collections::HashMap::new(),
            })
            .unwrap();

        registry
            .register_anthropic(ClientConfig {
                provider: "anthropic".to_string(),
                default_model: "claude-3-sonnet-20240229".to_string(),
                api_key: None,
                base_url: None,
                extra_params: std::collections::HashMap::new(),
            })
            .unwrap();

        let clients = registry.get_registered_clients();
        assert_eq!(clients.len(), 2);
        assert!(clients.contains(&"openai".to_string()));
        assert!(clients.contains(&"anthropic".to_string()));
    }

    #[tokio::test]
    async fn test_create_and_remove_agent() {
        let config = AgentConfig::new("openai", "gpt-3.5-turbo");
        let manager = AgentManager::new(config);
        let registry = ClientRegistry::new();

        // 注意：这个测试需要有效的 API 密钥才能通过
        // 在实际环境中运行时需要设置相应的环境变量
        if registry.has_client("openai") {
            // 创建 Agent
            manager
                .create_agent("test_agent".to_string(), None)
                .await
                .unwrap();
            let agents = manager.list_agents().await;
            assert_eq!(agents.len(), 1);
            assert!(agents.contains(&"test_agent".to_string()));

            // 删除 Agent
            let removed = manager.remove_agent("test_agent").await;
            assert!(removed);
            let agents = manager.list_agents().await;
            assert_eq!(agents.len(), 0);
        }
    }

    #[tokio::test]
    async fn test_simple_prompt() {
        let config = AgentConfig::new("openai", "gpt-3.5-turbo");
        let manager = AgentManager::new(config);
        let registry = ClientRegistry::new();

        if registry.has_client("openai") {
            // 创建 Agent
            manager
                .create_agent("prompt_test_agent".to_string(), None)
                .await
                .unwrap();

            // 测试简单 prompt
            let response = manager
                .prompt(&registry, "prompt_test_agent", "Hello, how are you?")
                .await
                .unwrap();

            assert!(!response.is_empty());
        }
    }

    #[tokio::test]
    async fn test_prompt_with() {
        let config = AgentConfig::new("openai", "gpt-3.5-turbo");
        let manager = AgentManager::new(config);
        let registry = ClientRegistry::new();

        if registry.has_client("openai") {
            // 测试临时 prompt
            let response = manager
                .prompt_with(&registry, "openai", "gpt-3.5-turbo", "Hello, how are you?")
                .await
                .unwrap();

            assert!(!response.is_empty());
        }
    }

    #[tokio::test]
    async fn test_conversation_history() {
        let config = AgentConfig::new("openai", "gpt-3.5-turbo");
        let manager = AgentManager::new(config);
        let registry = ClientRegistry::new();

        if registry.has_client("openai") {
            // 创建 Agent
            manager
                .create_agent("history_test_agent".to_string(), None)
                .await
                .unwrap();

            // 发送消息
            manager
                .chat(&registry, "history_test_agent", "Hello")
                .await
                .unwrap();
            manager
                .chat(&registry, "history_test_agent", "How are you?")
                .await
                .unwrap();

            // 获取历史
            let history = manager
                .get_conversation_history("history_test_agent")
                .await
                .unwrap();

            assert!(history.total_messages >= 4); // 2 user + 2 assistant
            assert!(!history.messages.is_empty());

            // 清除历史
            manager
                .clear_conversation_history("history_test_agent")
                .await
                .unwrap();

            let history_after = manager
                .get_conversation_history("history_test_agent")
                .await
                .unwrap();

            assert_eq!(history_after.total_messages, 0);
        }
    }

    #[tokio::test]
    async fn test_switch_provider() {
        let config = AgentConfig::new("openai", "gpt-3.5-turbo");
        let manager = AgentManager::new(config);
        let mut registry = ClientRegistry::new();

        if registry.has_client("openai") && std::env::var("ANTHROPIC_API_KEY").is_ok() {
            // 注册 Anthropic 客户端
            registry
                .register_anthropic(ClientConfig {
                    provider: "anthropic".to_string(),
                    default_model: "claude-3-sonnet-20240229".to_string(),
                    api_key: None,
                    base_url: None,
                    extra_params: std::collections::HashMap::new(),
                })
                .unwrap();

            // 创建 OpenAI Agent
            manager
                .create_agent("switch_test_agent".to_string(), None)
                .await
                .unwrap();

            // 获取初始配置
            let initial_config = manager
                .get_agent_config("switch_test_agent")
                .await
                .unwrap();
            assert_eq!(initial_config.provider, "openai");

            // 切换到 Anthropic
            manager
                .switch_provider("switch_test_agent", "anthropic", "claude-3-sonnet-20240229")
                .await
                .unwrap();

            // 验证切换成功
            let new_config = manager
                .get_agent_config("switch_test_agent")
                .await
                .unwrap();
            assert_eq!(new_config.provider, "anthropic");
            assert_eq!(new_config.model, "claude-3-sonnet-20240229");
        }
    }
}
