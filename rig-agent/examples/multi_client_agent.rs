//! 多客户端 Agent 示例
//! 
//! 这个示例展示了如何使用重新设计的 Agent 系统来支持多个 AI 提供商客户端。
//! 参考了 rig-core 的 dyn_client.rs 示例模式。

use rig_agent::core::agent::{AgentManager, ClientConfig};
use rig_agent::core::types::AgentConfig;
use rig_agent::error::AgentResult;

#[tokio::main]
async fn main() -> AgentResult<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== 多客户端 Agent 示例 ===");
    println!("确保设置了相应的 API 密钥环境变量：");
    println!("- OPENAI_API_KEY");
    println!("- ANTHROPIC_API_KEY");
    println!();

    // 创建默认配置
    let default_config = AgentConfig::default();
    let mut manager = AgentManager::new(default_config);

    // 显示已注册的客户端
    let registered_clients = manager.get_registered_clients();
    println!("已注册的客户端: {:?}", registered_clients);

    // 示例 1: 使用 OpenAI 客户端
    if registered_clients.contains(&"openai".to_string()) {
        println!("\n--- 示例 1: OpenAI 客户端 ---");
        
        let openai_config = AgentConfig {
            model: "gpt-3.5-turbo".to_string(),
            provider: Some("openai".to_string()),
            preamble: Some("你是一个有用的AI助手，请用中文回答。".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(500),
            enable_tools: false,
            history_limit: Some(10),
            extra_params: std::collections::HashMap::new(),
        };

        // 创建 OpenAI Agent
        manager
            .create_agent("openai_agent".to_string(), Some(openai_config))
            .await?;

        // 发送消息
        let response = manager
            .chat("openai_agent", "你好，请介绍一下你自己。")
            .await?;

        println!("OpenAI 响应: {}", response.content);
    }

    // 示例 2: 使用 Anthropic 客户端
    if registered_clients.contains(&"anthropic".to_string()) {
        println!("\n--- 示例 2: Anthropic 客户端 ---");
        
        let anthropic_config = AgentConfig {
            model: "claude-3-sonnet-20240229".to_string(),
            provider: Some("anthropic".to_string()),
            preamble: Some("你是一个专业的AI助手，请用中文回答。".to_string()),
            temperature: Some(0.5),
            max_tokens: Some(800),
            enable_tools: false,
            history_limit: Some(15),
            extra_params: std::collections::HashMap::new(),
        };

        // 创建 Anthropic Agent
        manager
            .create_agent("anthropic_agent".to_string(), Some(anthropic_config))
            .await?;

        // 发送消息
        let response = manager
            .chat("anthropic_agent", "请解释一下什么是人工智能。")
            .await?;

        println!("Anthropic 响应: {}", response.content);
    }

    // 示例 3: 动态注册新客户端
    println!("\n--- 示例 3: 动态注册客户端 ---");
    
    // 注册一个自定义客户端配置
    let custom_config = ClientConfig {
        provider: "custom_provider".to_string(),
        default_model: "custom-model".to_string(),
        api_key: Some("your-api-key".to_string()),
        base_url: Some("https://api.custom-provider.com".to_string()),
    };

    manager.register_client("custom_provider".to_string(), custom_config)?;
    
    let updated_clients = manager.get_registered_clients();
    println!("更新后的客户端列表: {:?}", updated_clients);

    // 示例 4: 多轮对话
    if registered_clients.contains(&"openai".to_string()) {
        println!("\n--- 示例 4: 多轮对话 ---");
        
        let chat_config = AgentConfig {
            model: "gpt-3.5-turbo".to_string(),
            provider: Some("openai".to_string()),
            preamble: Some("你是一个友好的聊天机器人，请保持对话的连贯性。".to_string()),
            temperature: Some(0.8),
            max_tokens: Some(300),
            enable_tools: false,
            history_limit: Some(20),
            extra_params: std::collections::HashMap::new(),
        };

        // 创建聊天 Agent
        manager
            .create_agent("chat_agent".to_string(), Some(chat_config))
            .await?;

        // 多轮对话
        let messages = vec![
            "你好！",
            "今天天气怎么样？",
            "你能帮我写一首诗吗？",
            "谢谢你的帮助！",
        ];

        for (i, message) in messages.iter().enumerate() {
            println!("用户 (第{}轮): {}", i + 1, message);
            
            let response = manager.chat("chat_agent", message).await?;
            println!("AI (第{}轮): {}", i + 1, response.content);
            println!();
        }

        // 获取对话历史
        let history = manager.get_conversation_history("chat_agent").await?;
        println!("对话历史统计:");
        println!("- 总消息数: {}", history.total_messages);
        println!("- 总令牌数: {:?}", history.total_tokens);
        println!("- 创建时间: {}", history.created_at);
        println!("- 最后活动: {}", history.last_activity);
    }

    // 示例 5: 简单 prompt（不保存历史）
    if registered_clients.contains(&"openai".to_string()) {
        println!("\n--- 示例 5: 简单 Prompt ---");
        
        let response = manager
            .prompt("openai_agent", "请用一句话总结今天的天气。")
            .await?;

        println!("简单 Prompt 响应: {}", response);
    }

    // 示例 6: Agent 管理
    println!("\n--- 示例 6: Agent 管理 ---");
    
    let agents = manager.list_agents().await;
    println!("当前 Agent 列表: {:?}", agents);

    // 获取 Agent 统计信息
    for agent_id in &agents {
        if let Ok(stats) = manager.get_agent_stats(agent_id).await {
            println!("Agent {} 统计:", agent_id);
            println!("- 总消息数: {}", stats.total_messages);
            println!("- 用户消息数: {}", stats.user_messages);
            println!("- 助手消息数: {}", stats.assistant_messages);
            println!("- 运行时间: {:?}", stats.uptime);
        }
    }

    // 清理 Agent
    for agent_id in &agents {
        let removed = manager.remove_agent(agent_id).await;
        println!("删除 Agent {}: {}", agent_id, removed);
    }

    println!("\n=== 示例完成 ===");
    Ok(())
}
