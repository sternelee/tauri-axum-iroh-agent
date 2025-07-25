//! 核心 Agent 实现 - 基于 rig-core

use crate::core::types::{AgentConfig, AgentMessage, AgentResponse, ConversationHistory, ToolCall};
use crate::error::{AgentError, AgentResult};
use crate::tools::ToolManager;
use rig::{
    agent::Agent as RigAgent,
    completion::CompletionModel,
    providers::{anthropic, cohere, gemini, openai},
    tool::Tool,
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

/// Agent 结构体
pub struct Agent {
    id: String,
    config: AgentConfig,
    rig_agent: RigAgent<Box<dyn CompletionModel + Send + Sync>>,
    conversation_history: Vec<AgentMessage>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
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

    /// 创建完成模型
    #[instrument(skip(self), fields(provider = %config.provider.as_deref().unwrap_or("openai"), model = %config.model))]
    fn create_completion_model(
        &self,
        config: &AgentConfig,
    ) -> AgentResult<Box<dyn CompletionModel + Send + Sync>> {
        let provider = config.provider.as_deref().unwrap_or("openai");

        info!("正在创建完成模型: {} - {}", provider, config.model);

        let result = match provider.to_lowercase().as_str() {
            "openai" => {
                debug!("创建 OpenAI 完成模型");
                let client = openai::Client::from_env();
                let model = client.completion_model(&config.model);
                Ok(Box::new(model) as Box<dyn CompletionModel + Send + Sync>)
            }
            "anthropic" => {
                debug!("创建 Anthropic 完成模型");
                let client = anthropic::Client::from_env();
                let model = client.completion_model(&config.model);
                Ok(Box::new(model) as Box<dyn CompletionModel + Send + Sync>)
            }
            "cohere" => {
                debug!("创建 Cohere 完成模型");
                let client = cohere::Client::from_env();
                let model = client.completion_model(&config.model);
                Ok(Box::new(model) as Box<dyn CompletionModel + Send + Sync>)
            }
            "gemini" => {
                debug!("创建 Gemini 完成模型");
                let client = gemini::Client::from_env();
                let model = client.completion_model(&config.model);
                Ok(Box::new(model) as Box<dyn CompletionModel + Send + Sync>)
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
            Ok(_) => info!("完成模型创建成功: {} - {}", provider, config.model),
            Err(e) => error!("完成模型创建失败: {}", e),
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
        let completion_model = self.create_completion_model(&agent_config)?;

        // 创建 Agent 构建器
        let mut agent_builder = RigAgent::builder(completion_model)
            .preamble(agent_config.preamble.clone().unwrap_or_default())
            .temperature(agent_config.temperature.unwrap_or(0.7))
            .max_tokens(agent_config.max_tokens.unwrap_or(1024));

        // 添加工具（如果有）
        let available_tools = self.tool_manager.get_available_tools();
        if !available_tools.is_empty() {
            agent_builder = agent_builder.tools(available_tools);
        }

        let rig_agent = agent_builder.build();

        agents.insert(
            agent_id.clone(),
            Agent {
                id: agent_id.clone(),
                config: agent_config,
                rig_agent,
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

        // 添加用户消息到历史
        let user_message = AgentMessage::user(message.to_string());
        agent.conversation_history.push(user_message);
        debug!(
            "添加用户消息到对话历史，当前历史长度: {}",
            agent.conversation_history.len()
        );

        // 调用 rig-core AI 模型
        debug!("准备调用 AI 模型");
        let ai_start_time = std::time::Instant::now();

        let response = agent.rig_agent.chat(message).await.map_err(|e| {
            error!("AI 模型调用失败，Agent: {}, 错误: {}", agent_id, e);
            AgentError::other(format!("AI 模型调用失败: {}", e))
        })?;

        let ai_duration = ai_start_time.elapsed();
        info!(
            "AI 模型调用完成，Agent: {}, 耗时: {:?}",
            agent_id, ai_duration
        );

        let response_content = response.content().to_string();
        let mut tool_calls_result = None;

        debug!("AI 响应内容长度: {}", response_content.len());

        // 处理工具调用
        if let Some(tool_calls) = response.tool_calls() {
            info!("检测到工具调用，数量: {}", tool_calls.len());
            let mut executed_tools = Vec::new();

            for tool_call in tool_calls {
                debug!("执行工具调用: {}", tool_call.name());

                let rig_tool_call = ToolCall {
                    id: tool_call.id().to_string(),
                    name: tool_call.name().to_string(),
                    arguments: tool_call.arguments().to_string(),
                    timestamp: chrono::Utc::now(),
                };

                let tool_start_time = std::time::Instant::now();
                let tool_result = self.tool_manager.execute_tool(&rig_tool_call).await;
                let tool_duration = tool_start_time.elapsed();

                match tool_result {
                    Ok(result) => {
                        info!(
                            "工具执行成功: {}, 耗时: {:?}",
                            tool_call.name(),
                            tool_duration
                        );
                        debug!("工具执行结果: {}", result.result);
                    }
                    Err(e) => {
                        error!(
                            "工具执行失败: {}, 错误: {}, 耗时: {:?}",
                            tool_call.name(),
                            e,
                            tool_duration
                        );
                    }
                }

                executed_tools.push(rig_tool_call);
            }

            tool_calls_result = Some(executed_tools);
        }

        let assistant_message = AgentMessage::assistant(response_content.clone());
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
            response_content.len()
        );

        Ok(AgentResponse {
            id: response_id,
            agent_id: agent_id.to_string(),
            content: response_content,
            timestamp: chrono::Utc::now(),
            model: agent.config.model.clone(),
            usage: None, // TODO: 从 rig-core 获取使用统计
            tool_calls: tool_calls_result,
            finish_reason: Some("stop".to_string()),
        })
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

        let total_tokens = agent
            .conversation_history
            .iter()
            .map(|msg| msg.content.len() as u64)
            .sum();

        Ok(ConversationHistory {
            agent_id: agent_id.to_string(),
            messages: agent.conversation_history.clone(),
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
    async fn test_tool_manager_integration() {
        let config = AgentConfig::default();
        let manager = AgentManager::new(config);

        // 测试工具管理器
        let tool_manager = manager.get_tool_manager();
        assert!(tool_manager.has_tool("calculator"));
        assert!(tool_manager.has_tool("current_time"));
        assert!(!tool_manager.has_tool("nonexistent_tool"));
    }

    #[tokio::test]
    async fn test_agent_config_management() {
        let config = AgentConfig::default();
        let mut manager = AgentManager::new(config.clone());

        if std::env::var("OPENAI_API_KEY").is_ok() {
            // 创建 Agent
            manager
                .create_agent("config_test_agent".to_string(), None)
                .await
                .unwrap();

            // 获取配置
            let retrieved_config = manager.get_agent_config("config_test_agent").await.unwrap();
            assert_eq!(retrieved_config.model, config.model);

            // 更新配置
            let mut new_config = config.clone();
            new_config.temperature = Some(0.5);
            manager
                .update_agent_config("config_test_agent", new_config.clone())
                .await
                .unwrap();

            let updated_config = manager.get_agent_config("config_test_agent").await.unwrap();
            assert_eq!(updated_config.temperature, Some(0.5));
        }
    }
}
