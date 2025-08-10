//! 多客户端示例
//!
//! 这个示例展示了如何使用重构后的 rig-agent 库同时使用多个 AI 提供商
//! 确保设置了相应的环境变量：OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY

use rig_agent::{AgentConfig, AgentManager, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建默认配置
    let config = AgentConfig::new("openai", "gpt-3.5-turbo")
        .with_preamble("你是一个有用的AI助手。")
        .with_temperature(0.7);

    // 创建 AgentManager
    let mut manager = AgentManager::new(config);

    // 注册多个提供商
    if std::env::var("OPENAI_API_KEY").is_ok() {
        println!("注册 OpenAI 客户端");
        manager.register_openai(ClientConfig::new("openai", "gpt-4o"))?;
    }

    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        println!("注册 Anthropic 客户端");
        manager.register_anthropic(ClientConfig::new("anthropic", "claude-3-sonnet-20240229"))?;
    }

    if std::env::var("GEMINI_API_KEY").is_ok() {
        println!("注册 Gemini 客户端");
        manager.register_gemini(ClientConfig::new("gemini", "gemini-pro"))?;
    }

    // 获取已注册的客户端列表
    let clients = manager.get_registered_clients();
    println!("已注册的客户端: {:?}", clients);

    if clients.is_empty() {
        println!("没有可用的客户端，请设置相应的环境变量");
        return Ok(());
    }

    // 创建使用不同提供商的 Agent
    let prompt = "请用一句话介绍你自己";

    for provider in clients {
        let model = match provider.as_str() {
            "openai" => "gpt-4o",
            "anthropic" => "claude-3-sonnet-20240229",
            "gemini" => "gemini-pro",
            _ => continue,
        };

        let agent_id = format!("{}_agent", provider);
        println!("创建 {} Agent ({})", provider, model);

        // 创建 Agent
        manager
            .create_agent(agent_id.clone(), Some(AgentConfig::new(&provider, model)))
            .await?;

        // 发送相同的提示到不同的 Agent
        println!("向 {} 发送提示: {}", agent_id, prompt);
        let response = manager.chat(&agent_id, prompt).await?;
        println!("{} 的响应: {}\n", agent_id, response.content);
    }

    // 使用临时 Agent 进行快速提问
    if manager.get_client_registry().has_client("openai") {
        println!("使用临时 OpenAI Agent 进行快速提问");
        let response = manager
            .prompt_with("openai", "gpt-3.5-turbo", "用一句话描述人工智能的未来")
            .await?;
        println!("临时 Agent 响应: {}\n", response);
    }

    // 切换 Agent 的提供商
    if manager.get_client_registry().has_client("openai") && manager.get_client_registry().has_client("anthropic") {
        let agent_id = "openai_agent";
        println!("将 {} 从 OpenAI 切换到 Anthropic", agent_id);
        
        manager
            .switch_provider(agent_id, "anthropic", "claude-3-sonnet-20240229")
            .await?;
        
        let response = manager.chat(agent_id, "你现在是哪个模型?").await?;
        println!("切换后的响应: {}\n", response.content);
    }

    // 获取所有 Agent 的统计信息
    let stats = manager.get_all_agent_stats().await;
    println!("Agent 统计信息:");
    for stat in stats {
        println!(
            "- {}: 提供商={}, 模型={}, 消息数={}",
            stat.agent_id, stat.provider, stat.model, stat.total_messages
        );
    }

    Ok(())
}