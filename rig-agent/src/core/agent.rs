//! 核心 Agent 实现 - 基于 rig-core

use crate::core::types::{AgentConfig, AgentMessage, AgentResponse, ConversationHistory};
use crate::error::{AgentError, AgentResult};
use crate::tools::ToolManager;
use rig::{
    agent::Agent as RigAgent,
    client::builder::DynClientBuilder,
    completion::{Chat, Prompt},
    message::Message,
    prelude::*,
    providers::{anthropic, openai},
};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

/// Agent 管理器，负责创建和管理 Agent 实例
pub struct AgentManager {
    agents: RwLock<HashMap<String, Agent>>,
    default_config: AgentConfig,
    tool_manager: ToolManager,
}

/// Agent 包装器，用于存储不同类型的 Agent
enum AgentWrapper {
    OpenAI(RigAgent<openai::responses_api::ResponsesCompletionModel>),
    Anthropic(RigAgent<anthropic::completion::CompletionModel>),
}

/// Agent 信息结构体
pub struct Agent {
    id: String,
    config: AgentConfig,
    wrapper: AgentWrapper,
    conversation_history: Vec<Message>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
}

impl AgentWrapper {
    /// 发送 prompt 消息
    async fn prompt(&self, message: &str) -> AgentResult<String> {
        let result = match self {
            AgentWrapper::OpenAI(agent) => agent.prompt(message).await,
            AgentWrapper::Anthropic(agent) => agent.prompt(message).await,
        };

        result.map_err(|e| AgentError::other(format!("AI 模型调用失败: {}", e)))
    }

    /// 发送聊天消息（带历史）
    async fn chat(&self, message: Message, history: Vec<Message>) -> AgentResult<String> {
        let result = match self {
            AgentWrapper::OpenAI(agent) => agent.chat(message, history).await,
            AgentWrapper::Anthropic(agent) => agent.chat(message, history).await,
        };

        result.map_err(|e| AgentError::other(format!("AI 模型调用失败: {}", e)))
    }
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

    /// 创建 Agent 实例
    #[instrument(skip(self), fields(provider = %config.provider.as_deref().unwrap_or("openai"), model = %config.model))]
    fn create_agent_wrapper(&self, config: &AgentConfig) -> AgentResult<AgentWrapper> {
        let provider = config.provider.as_deref().unwrap_or("openai");

        info!("正在创建 Agent 实例: {} - {}", provider, config.model);

        let result = match provider.to_lowercase().as_str() {
            "openai" => {
                debug!("创建 OpenAI Agent");
                let client = openai::Client::from_env();
                let mut agent_builder = client.agent(&config.model);

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
                Ok(AgentWrapper::OpenAI(agent))
            }
            "anthropic" => {
                debug!("创建 Anthropic Agent");
                let client = anthropic::Client::from_env();
                let mut agent_builder = client.agent(&config.model);

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
                Ok(AgentWrapper::Anthropic(agent))
            }
            _ => {
                error!("不支持的 AI 提供商: {}", provider);
                Err(AgentError::other(format!(
                    "不支持的 AI 提供商: {}",
                    provider
                )))
            }
        };

        match &result {
            Ok(_) => info!("Agent 实例创建成功: {} - {}", provider, config.model),
            Err(e) => error!("Agent 实例创建失败: {}", e),
        }

        result
    }

    /// 创建新的 Agent
    pub async fn create_agent(
        &mut self,
        agent_id: String,
        config: Option<AgentConfig>,
    ) -> AgentResult<()> {
        let mut agents = self.agents.write().await;

        if agents.contains_key(&agent_id) {
            return Err(AgentError::other(format!("Agent 已存在: {}", agent_id)));
        }

        let agent_config = config.unwrap_or_else(|| self.default_config.clone());
        let wrapper = self.create_agent_wrapper(&agent_config)?;

        agents.insert(
            agent_id.clone(),
            Agent {
                id: agent_id.clone(),
                config: agent_config,
                wrapper,
                conversation_history: Vec::new(),
                created_at: chrono::Utc::now(),
                last_activity: chrono::Utc::now(),
            },
        );

        info!("创建新 Agent: {}", agent_id);
        Ok(())
    }

    /// 删除 Agent
    pub async fn remove_agent(&mut self, agent_id: &str) -> bool {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id).is_some()
    }

    /// 获取 Agent 列表
    pub async fn list_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }

    /// 发送聊天消息
    #[instrument(skip(self, message), fields(agent_id = %agent_id, message_len = message.len()))]
    pub async fn chat(&self, agent_id: &str, message: &str) -> AgentResult<AgentResponse> {
        let start_time = std::time::Instant::now();
        info!(
            "开始处理聊天消息，Agent: {}, 消息长度: {}",
            agent_id,
            message.len()
        );

        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(agent_id).ok_or_else(|| {
            error!("Agent 不存在: {}", agent_id);
            AgentError::AgentNotFound(agent_id.to_string())
        })?;

        // 更新最后活动时间
        agent.last_activity = chrono::Utc::now();
        debug!("更新 Agent {} 最后活动时间", agent_id);

        // 创建用户消息
        let user_message = Message::user(message);
        agent.conversation_history.push(user_message.clone());
        debug!(
            "添加用户消息到对话历史，当前历史长度: {}",
            agent.conversation_history.len()
        );

        // 调用 rig-core AI 模型
        debug!("准备调用 AI 模型");
        let ai_start_time = std::time::Instant::now();

        // 使用对话历史进行聊天
        let response = agent
            .wrapper
            .chat(user_message, agent.conversation_history.clone())
            .await?;

        let ai_duration = ai_start_time.elapsed();
        info!(
            "AI 模型调用完成，Agent: {}, 耗时: {:?}",
            agent_id, ai_duration
        );

        debug!("AI 响应内容长度: {}", response.len());

        // 创建助手消息并添加到历史
        let assistant_message = Message::assistant(&response);
        agent.conversation_history.push(assistant_message);

        // 应用历史限制
        if let Some(limit) = agent.config.history_limit {
            if agent.conversation_history.len() > limit {
                let excess = agent.conversation_history.len() - limit;
                agent.conversation_history.drain(0..excess);
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
            model: agent.config.model.clone(),
            usage: None,      // TODO: 从 rig-core 获取使用统计
            tool_calls: None, // TODO: 处理工具调用
            finish_reason: Some("stop".to_string()),
        })
    }

    /// 简单的 prompt 方法（不保存历史）
    #[instrument(skip(self, message), fields(agent_id = %agent_id, message_len = message.len()))]
    pub async fn prompt(&self, agent_id: &str, message: &str) -> AgentResult<String> {
        let agents = self.agents.read().await;
        let agent = agents.get(agent_id).ok_or_else(|| {
            error!("Agent 不存在: {}", agent_id);
            AgentError::AgentNotFound(agent_id.to_string())
        })?;

        debug!("准备调用 AI 模型进行简单 prompt");
        let ai_start_time = std::time::Instant::now();

        // 使用 prompt 方法，不保存历史
        let response = agent.wrapper.prompt(message).await?;

        let ai_duration = ai_start_time.elapsed();
        info!(
            "简单 prompt 完成，Agent: {}, 耗时: {:?}",
            agent_id, ai_duration
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
            .map(|msg| {
                match msg {
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
                                rig::message::AssistantContent::Text(text) => {
                                    Some(text.text.clone())
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        AgentMessage::assistant(text)
                    }
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
        &mut self,
        agent_id: &str,
        config: AgentConfig,
    ) -> AgentResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        agent.config = config;
        agent.last_activity = chrono::Utc::now();
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
            total_messages,
            user_messages,
            assistant_messages,
            created_at: agent.created_at,
            last_activity: agent.last_activity,
            uptime: chrono::Utc::now().signed_duration_since(agent.created_at),
        })
    }
}

/// Agent 统计信息
#[derive(Debug, Clone)]
pub struct AgentStats {
    pub agent_id: String,
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
    async fn test_create_and_remove_agent() {
        let config = AgentConfig::default();
        let mut manager = AgentManager::new(config);

        // 注意：这个测试需要有效的 API 密钥才能通过
        // 在实际环境中运行时需要设置相应的环境变量
        if std::env::var("OPENAI_API_KEY").is_ok() {
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
        let config = AgentConfig::default();
        let mut manager = AgentManager::new(config);

        if std::env::var("OPENAI_API_KEY").is_ok() {
            // 创建 Agent
            manager
                .create_agent("prompt_test_agent".to_string(), None)
                .await
                .unwrap();

            // 测试简单 prompt
            let response = manager
                .prompt("prompt_test_agent", "Hello, how are you?")
                .await
                .unwrap();

            assert!(!response.is_empty());
        }
    }

    #[tokio::test]
    async fn test_conversation_history() {
        let config = AgentConfig::default();
        let mut manager = AgentManager::new(config);

        if std::env::var("OPENAI_API_KEY").is_ok() {
            // 创建 Agent
            manager
                .create_agent("history_test_agent".to_string(), None)
                .await
                .unwrap();

            // 发送消息
            manager.chat("history_test_agent", "Hello").await.unwrap();
            manager
                .chat("history_test_agent", "How are you?")
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
}
