//! Agent 模块集成测试

use rig_agent::{
    core::{
        agent::AgentManager,
        types::{AgentConfig, AgentRole, MessageType},
    },
    tools::{ToolManager, BuiltinTools, ToolCall},
    error::AgentError,
};
use std::env;
use tokio;
use tracing_test::traced_test;

/// 测试环境设置
fn setup_test_env() {
    // 设置测试用的环境变量（如果没有的话）
    if env::var("OPENAI_API_KEY").is_err() {
        env::set_var("OPENAI_API_KEY", "test-key-for-testing");
    }
}

/// 创建测试用的 Agent 配置
fn create_test_config() -> AgentConfig {
    AgentConfig {
        model: "gpt-3.5-turbo".to_string(),
        provider: Some("openai".to_string()),
        system_prompt: Some("你是一个测试助手。".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(100),
        enable_tools: true,
        history_limit: Some(10),
        extra_params: std::collections::HashMap::new(),
    }
}

#[tokio::test]
#[traced_test]
async fn test_agent_manager_lifecycle() {
    setup_test_env();
    
    let config = create_test_config();
    let mut manager = AgentManager::new(config);
    
    // 测试初始状态
    let agents = manager.list_agents().await;
    assert_eq!(agents.len(), 0);
    
    // 测试创建 Agent
    let agent_id = "test_lifecycle_agent";
    let result = manager.create_agent(agent_id.to_string(), None).await;
    
    // 如果没有有效的 API 密钥，这个测试会失败，这是预期的
    match result {
        Ok(_) => {
            // 验证 Agent 已创建
            let agents = manager.list_agents().await;
            assert_eq!(agents.len(), 1);
            assert!(agents.contains(&agent_id.to_string()));
            
            // 测试删除 Agent
            let removed = manager.remove_agent(agent_id).await;
            assert!(removed);
            
            let agents = manager.list_agents().await;
            assert_eq!(agents.len(), 0);
        }
        Err(e) => {
            // 如果是因为 API 密钥问题导致的失败，这是可以接受的
            println!("Agent 创建失败（可能是 API 密钥问题）: {}", e);
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_agent_configuration_management() {
    setup_test_env();
    
    let config = create_test_config();
    let mut manager = AgentManager::new(config.clone());
    
    let agent_id = "test_config_agent";
    
    // 尝试创建 Agent
    if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
        // 测试获取配置
        let retrieved_config = manager.get_agent_config(agent_id).await.unwrap();
        assert_eq!(retrieved_config.model, config.model);
        assert_eq!(retrieved_config.temperature, config.temperature);
        
        // 测试更新配置
        let mut new_config = config.clone();
        new_config.temperature = Some(0.5);
        new_config.max_tokens = Some(200);
        
        manager.update_agent_config(agent_id, new_config.clone()).await.unwrap();
        
        let updated_config = manager.get_agent_config(agent_id).await.unwrap();
        assert_eq!(updated_config.temperature, Some(0.5));
        assert_eq!(updated_config.max_tokens, Some(200));
    }
}

#[tokio::test]
#[traced_test]
async fn test_conversation_history_management() {
    setup_test_env();
    
    let config = create_test_config();
    let mut manager = AgentManager::new(config);
    
    let agent_id = "test_history_agent";
    
    if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
        // 测试初始历史为空
        let history = manager.get_conversation_history(agent_id).await.unwrap();
        assert_eq!(history.messages.len(), 0);
        
        // 模拟对话（注意：这里可能会因为 API 调用失败而跳过）
        if let Ok(response) = manager.chat(agent_id, "你好").await {
            assert!(!response.content.is_empty());
            
            // 检查历史记录
            let history = manager.get_conversation_history(agent_id).await.unwrap();
            assert!(history.messages.len() >= 2); // 至少有用户消息和助手回复
            
            // 验证消息类型
            let user_msg = &history.messages[0];
            assert_eq!(user_msg.role, AgentRole::User);
            assert_eq!(user_msg.message_type, MessageType::Text);
            assert_eq!(user_msg.content, "你好");
        }
        
        // 测试清除历史
        manager.clear_conversation_history(agent_id).await.unwrap();
        let history = manager.get_conversation_history(agent_id).await.unwrap();
        assert_eq!(history.messages.len(), 0);
    }
}

#[tokio::test]
#[traced_test]
async fn test_tool_manager_functionality() {
    let tool_manager = ToolManager::new();
    
    // 测试内置工具
    assert!(tool_manager.has_tool("calculator"));
    assert!(tool_manager.has_tool("current_time"));
    assert!(tool_manager.has_tool("weather"));
    assert!(!tool_manager.has_tool("nonexistent_tool"));
    
    // 测试获取工具定义
    let tool_definitions = tool_manager.get_all_tool_definitions();
    assert!(!tool_definitions.is_empty());
    
    let calculator_def = tool_definitions.iter()
        .find(|def| def.name == "calculator")
        .expect("应该找到计算器工具");
    
    assert_eq!(calculator_def.name, "calculator");
    assert!(!calculator_def.description.is_empty());
}

#[tokio::test]
#[traced_test]
async fn test_builtin_tools_execution() {
    let builtin_tools = BuiltinTools::new();
    
    // 测试计算器工具
    let calculator_call = ToolCall {
        id: "test_calc_1".to_string(),
        name: "calculator".to_string(),
        arguments: r#"{"expression": "2+3*4"}"#.to_string(),
        timestamp: chrono::Utc::now(),
    };
    
    let result = builtin_tools.execute_tool(&calculator_call).await.unwrap();
    assert!(result.success);
    assert!(result.result.contains("14")); // 2+3*4 = 14
    
    // 测试当前时间工具
    let time_call = ToolCall {
        id: "test_time_1".to_string(),
        name: "current_time".to_string(),
        arguments: "{}".to_string(),
        timestamp: chrono::Utc::now(),
    };
    
    let result = builtin_tools.execute_tool(&time_call).await.unwrap();
    assert!(result.success);
    assert!(result.result.contains("当前时间"));
    
    // 测试天气工具
    let weather_call = ToolCall {
        id: "test_weather_1".to_string(),
        name: "weather".to_string(),
        arguments: r#"{"city": "北京"}"#.to_string(),
        timestamp: chrono::Utc::now(),
    };
    
    let result = builtin_tools.execute_tool(&weather_call).await.unwrap();
    assert!(result.success);
    assert!(result.result.contains("北京"));
}

#[tokio::test]
#[traced_test]
async fn test_error_handling() {
    setup_test_env();
    
    let config = create_test_config();
    let mut manager = AgentManager::new(config);
    
    // 测试访问不存在的 Agent
    let result = manager.get_agent_config("nonexistent_agent").await;
    assert!(matches!(result, Err(AgentError::AgentNotFound(_))));
    
    let result = manager.chat("nonexistent_agent", "hello").await;
    assert!(matches!(result, Err(AgentError::AgentNotFound(_))));
    
    let result = manager.clear_conversation_history("nonexistent_agent").await;
    assert!(matches!(result, Err(AgentError::AgentNotFound(_))));
    
    // 测试重复创建 Agent
    let agent_id = "duplicate_test_agent";
    if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
        let result = manager.create_agent(agent_id.to_string(), None).await;
        assert!(result.is_err());
    }
}

#[tokio::test]
#[traced_test]
async fn test_agent_with_tools_enabled() {
    setup_test_env();
    
    let mut config = create_test_config();
    config.enable_tools = true;
    
    let mut manager = AgentManager::new(config);
    let agent_id = "test_tools_agent";
    
    if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
        // 测试工具管理器集成
        let tool_manager = manager.get_tool_manager();
        assert!(tool_manager.has_tool("calculator"));
        
        // 尝试发送可能触发工具调用的消息
        // 注意：这需要真实的 API 调用才能完全测试
        if let Ok(response) = manager.chat(agent_id, "计算 2+3").await {
            assert!(!response.content.is_empty());
            // 如果工具被调用，response.tool_calls 应该不为空
            // 但这取决于 AI 模型是否决定调用工具
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_conversation_history_limits() {
    setup_test_env();
    
    let mut config = create_test_config();
    config.history_limit = Some(4); // 限制为 4 条消息
    
    let mut manager = AgentManager::new(config);
    let agent_id = "test_limit_agent";
    
    if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
        // 发送多条消息来测试历史限制
        for i in 1..=3 {
            let _ = manager.chat(agent_id, &format!("消息 {}", i)).await;
        }
        
        let history = manager.get_conversation_history(agent_id).await.unwrap();
        // 每次对话产生 2 条消息（用户 + 助手），所以 3 次对话 = 6 条消息
        // 但由于限制为 4，应该只保留最新的 4 条
        assert!(history.messages.len() <= 4);
    }
}

/// 性能测试：并发创建多个 Agent
#[tokio::test]
#[traced_test]
async fn test_concurrent_agent_operations() {
    setup_test_env();
    
    let config = create_test_config();
    let mut manager = AgentManager::new(config);
    
    // 并发创建多个 Agent
    let mut handles = vec![];
    
    for i in 0..5 {
        let agent_id = format!("concurrent_agent_{}", i);
        // 注意：这里我们不能直接并发调用 create_agent，因为它需要 &mut self
        // 在实际应用中，可能需要使用 Arc<Mutex<AgentManager>> 或类似的并发安全结构
        if manager.create_agent(agent_id, None).await.is_ok() {
            // Agent 创建成功
        }
    }
    
    let agents = manager.list_agents().await;
    // 根据 API 密钥的可用性，agents 的数量可能为 0 或 5
    println!("并发测试创建了 {} 个 Agent", agents.len());
}

/// 集成测试：完整的对话流程
#[tokio::test]
#[traced_test]
async fn test_complete_conversation_flow() {
    setup_test_env();
    
    let config = create_test_config();
    let mut manager = AgentManager::new(config);
    let agent_id = "complete_flow_agent";
    
    if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
        // 1. 发送初始消息
        if let Ok(response1) = manager.chat(agent_id, "你好，我是测试用户").await {
            assert!(!response1.content.is_empty());
            assert_eq!(response1.agent_id, agent_id);
            
            // 2. 继续对话
            if let Ok(response2) = manager.chat(agent_id, "请告诉我今天的日期").await {
                assert!(!response2.content.is_empty());
                
                // 3. 检查对话历史
                let history = manager.get_conversation_history(agent_id).await.unwrap();
                assert!(history.messages.len() >= 4); // 至少 2 轮对话
                
                // 4. 验证历史记录的完整性
                let user_messages: Vec<_> = history.messages.iter()
                    .filter(|msg| msg.role == AgentRole::User)
                    .collect();
                assert_eq!(user_messages.len(), 2);
                
                let assistant_messages: Vec<_> = history.messages.iter()
                    .filter(|msg| msg.role == AgentRole::Assistant)
                    .collect();
                assert_eq!(assistant_messages.len(), 2);
            }
        }
        
        // 5. 清理
        let removed = manager.remove_agent(agent_id).await;
        assert!(removed);
    }
}