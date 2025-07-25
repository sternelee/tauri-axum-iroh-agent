//! 核心 Agent 实现 - 基于 rig-core

use crate::core::types::{
    AgentConfig, AgentMessage, AgentResponse, AgentRole, ConversationHistory, MessageType,
    TokenUsage, ToolCall, ToolResult,
};
use crate::error::{AgentError, AgentResult};
use crate::tools::ToolManager;
use rig::{
client::builder::DynClientBuilder, completion::Prompt
    completion::{CompletionModel, Message},
    providers::{anthropic, cohere, gemini, openai},
    tool::Tool,
};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

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
    rig_agent: rig_core::agent::Agent,
    conversation_history: Vec<AgentMessage>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
    dynamic_context: Option<rig_core::agent::DynamicContext>,
    dynamic_tools: Option<rig_core::agent::DynamicTools>,
}

impl AgentManager {
    /// 创建新的 Agent 管理器
    pub fn new(default_config: AgentConfig) -> Self {
        let mut tool_manager = ToolManager::new();
        tool_manager.register_default_tools();

        Self {
            default_config,
            agents: RwLock::new(HashMap::new()),
            tool_manager,
        }
    }

    /// 创建 AI 提供商实例
    #[instrument(skip(self), fields(provider = %config.provider.as_deref().unwrap_or("openai"), model = %config.model))]
    async fn create_provider<T: CompletionModel + Send + Sync>(
        &self,
        config: &AgentConfig,
    ) -> AgentResult<Box<dyn CompletionModel + Send + Sync>> {
        let provider = config.provider.as_deref().unwrap_or("openai");

        info!("正在创建 AI 提供商实例: {} - {}", provider, config.model);

        let result = match provider.to_lowercase().as_str() {
            "openai" => {
                debug!("初始化 OpenAI 客户端");
                let client = openai::Client::from_env();
                let model = client.model(&config.model);
                Ok(Box::new(model))
            }
            "anthropic" => {
                debug!("初始化 Anthropic 客户端");
                let client = anthropic::Client::from_env();
                let model = client.model(&config.model);
                Ok(Box::new(model))
            }
            "cohere" => {
                debug!("初始化 Cohere 客户端");
                let client = cohere::Client::from_env();
                let model = client.model(&config.model);
                Ok(Box::new(model))
            }
            "gemini" => {
                debug!("初始化 Gemini 客户端");
                let client = gemini::Client::from_env();
                let model = client.model(&config.model);
                Ok(Box::new(model))
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
            Ok(_) => info!("AI 提供商实例创建成功: {} - {}", provider, config.model),
            Err(e) => error!("AI 提供商实例创建失败: {}", e),
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
            return Err(AgentError::AgentAlreadyExists(agent_id));
        }

        let agent_config = config.unwrap_or_else(|| self.default_config.clone());
        let provider = self.create_provider(&agent_config).await?;
        let multi_client = DynClientBuilder::new();

        let mut agent_builder = multi_client.agent(provider, agent_config.model.clone())
                .with_preamble(agent_config.preamble.clone().unwrap_or_default())
                .with_temperature(agent_config.temperature.unwrap_or(0.7))
                .with_max_tokens(agent_config.max_tokens.unwrap_or(1024));

        // 添加静态工具
        let static_tools = self.tool_manager.get_static_tools();
        if !static_tools.is_empty() {
            agent_builder = agent_builder.with_static_tools(static_tools);
        }

        // 添加静态上下文（如果有）
        if let Some(static_context) = &agent_config.static_context {
            agent_builder = agent_builder.with_static_context(static_context.clone());
        }

        let rig_agent = agent_builder.build()?;

        agents.insert(
            agent_id.clone(),
            Agent {
                id: agent_id.clone(),
                config: agent_config,
                rig_agent,
                conversation_history: Vec::new(),
                created_at: chrono::Utc::now(),
                last_activity: chrono::Utc::now(),
                dynamic_context: None,
                dynamic_tools: None,
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

        // 构建对话上下文
        let mut messages = Vec::new();

        // 添加系统提示
        if let Some(system_prompt) = &agent.config.system_prompt {
            messages.push(Message::new_system_message(system_prompt.clone()));
        }

        // 添加历史消息
        for msg in &agent.conversation_history {
            match msg.role {
                AgentRole::User => {
                    messages.push(Message::new_user_message(msg.content.clone()));
                }
                AgentRole::Assistant => {
                    messages.push(Message::new_assistant_message(msg.content.clone()));
                }
                _ => {} // 暂时忽略其他角色
            }
        }

        // 调用 rig-core AI 模型
        debug!(
            "准备调用 AI 模型，消息数量: {}",
            prompt_request.history().len()
        );
        let ai_start_time = std::time::Instant::now();

        let response = agent.rig_agent.chat(prompt_request).await.map_err(|e| {
            error!("AI 模型调用失败，Agent: {}, 错误: {}", agent_id, e);
            AgentError::other(format!("AI 模型调用失败: {}", e))
        })?;

        // 处理工具调用
        let mut completion = match response.message_type() {
            rig_core::agent::MessageType::ToolCall => {
                self.handle_tool_calls(agent, response, &agent_id).await?
            }
            _ => response.content().to_string(),
        };

        let ai_duration = ai_start_time.elapsed();
        info!(
            "AI 模型调用完成，Agent: {}, 耗时: {:?}",
            agent_id, ai_duration
        );

        let mut response_content = completion.choice.message.content;
        let mut tool_calls_result = None;

        debug!("AI 响应内容长度: {}", response_content.len());

        // 处理工具调用
        if let Some(tool_calls) = &completion.choice.message.tool_calls {
            info!("检测到工具调用，数量: {}", tool_calls.len());
            let mut executed_tools = Vec::new();
            let mut tool_results = Vec::new();

            for (index, tool_call) in tool_calls.iter().enumerate() {
                debug!(
                    "执行工具调用 {}/{}: {}",
                    index + 1,
                    tool_calls.len(),
                    tool_call.function.name
                );

                let rig_tool_call = ToolCall {
                    id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    arguments: tool_call.function.arguments.clone(),
                    timestamp: chrono::Utc::now(),
                };

                let tool_start_time = std::time::Instant::now();
                let tool_result = self.tool_manager.execute_tool(&rig_tool_call).await;
                let tool_duration = tool_start_time.elapsed();

                let result = match tool_result {
                    Ok(result) => {
                        info!(
                            "工具执行成功: {}, 耗时: {:?}",
                            tool_call.function.name, tool_duration
                        );
                        debug!("工具执行结果: {}", result.result);

                        tool_results.push(ToolResult {
                            call_id: tool_call.id.clone(),
                            tool_name: tool_call.function.name.clone(),
                            result: result.result.clone(),
                            success: true,
                            error: None,
                            timestamp: chrono::Utc::now(),
                            duration_ms: tool_duration.as_millis() as u64,
                        });
                        result.result
                    }
                    Err(e) => {
                        error!(
                            "工具执行失败: {}, 错误: {}, 耗时: {:?}",
                            tool_call.function.name, e, tool_duration
                        );
                        let error_msg = format!("工具执行失败: {}", e);

                        tool_results.push(ToolResult {
                            call_id: tool_call.id.clone(),
                            tool_name: tool_call.function.name.clone(),
                            result: String::new(),
                            success: false,
                            error: Some(error_msg.clone()),
                            timestamp: chrono::Utc::now(),
                            duration_ms: tool_duration.as_millis() as u64,
                        });
                        error_msg
                    }
                };

                executed_tools.push(ToolCall {
                    id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    arguments: tool_call.function.arguments.clone(),
                    timestamp: chrono::Utc::now(),
                });
            }

            // 添加工具调用消息到历史
            let tool_call_message = AgentMessage::tool_call(executed_tools.clone());
            agent.conversation_history.push(tool_call_message);

            // 添加工具结果消息到历史
            let tool_result_message = AgentMessage::tool_result(tool_results.clone());
            agent.conversation_history.push(tool_result_message);

            tool_calls_result = Some(executed_tools);

            // 如果有工具调用，可能需要再次调用 AI 来生成最终响应
            if response_content.is_empty() {
                response_content = "已执行工具调用，请查看结果。".to_string();
            }
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
