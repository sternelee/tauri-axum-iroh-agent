//! 简单聊天示例

use rig_agent::{AgentConfig, StandaloneAgentAdapter, AgentAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 检查是否有 OpenAI API 密钥
    if std::env::var("OPENAI_API_KEY").is_err() {
        println!("请设置 OPENAI_API_KEY 环境变量");
        return Ok(());
    }

    // 创建默认配置
    let config = AgentConfig {
        model: "gpt-3.5-turbo".to_string(),
        provider: Some("openai".to_string()),
        preamble: Some("你是一个友好的AI助手，请用中文回答问题。".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(1000),
        enable_tools: false,
        history_limit: Some(10),
        extra_params: std::collections::HashMap::new(),
    };

    // 创建适配器
    let adapter = StandaloneAgentAdapter::new(config.clone());

    // 创建 Agent
    println!("正在创建 Agent...");
    adapter.create_agent("chat_bot".to_string(), Some(config)).await?;

    // 获取 Agent 列表
    let agents = adapter.list_agents().await?;
    println!("可用的 Agent: {:?}", agents);

    // 发送消息
    println!("正在发送消息...");
    let response = adapter.chat("chat_bot", "你好，请介绍一下你自己").await?;
    println!("AI 回复: {}", response.content);

    // 再发送一条消息
    let response = adapter.chat("chat_bot", "你能帮我做什么？").await?;
    println!("AI 回复: {}", response.content);

    println!("聊天示例完成！");
    Ok(())
}