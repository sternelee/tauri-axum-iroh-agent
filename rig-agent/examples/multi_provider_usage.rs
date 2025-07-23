//! 多 Provider 使用示例

use rig_agent::{ConfigBuilder, StandaloneAgentAdapter, AgentAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("=== Rig Agent 多 Provider 使用示例 ===\n");

    // 示例 1: OpenAI GPT-4
    println!("1. 使用 OpenAI GPT-4:");
    if std::env::var("OPENAI_API_KEY").is_ok() {
        let openai_config = ConfigBuilder::new()
            .model("gpt-4".to_string())
            .system_prompt("你是一个专业的AI助手，使用OpenAI GPT-4模型。".to_string())
            .temperature(0.7)
            .max_tokens(1000)
            .build();

        let adapter = StandaloneAgentAdapter::new(openai_config);
        
        match adapter.create_agent("openai_agent".to_string(), None).await {
            Ok(_) => {
                match adapter.chat("openai_agent", "请简单介绍一下你自己").await {
                    Ok(response) => println!("OpenAI 回复: {}\n", response.content),
                    Err(e) => println!("OpenAI 聊天错误: {}\n", e),
                }
            }
            Err(e) => println!("创建 OpenAI Agent 失败: {}\n", e),
        }
    } else {
        println!("跳过 OpenAI 示例 (缺少 OPENAI_API_KEY)\n");
    }

    // 示例 2: Anthropic Claude
    println!("2. 使用 Anthropic Claude:");
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        let anthropic_config = ConfigBuilder::new()
            .model("claude-3-sonnet".to_string())
            .system_prompt("你是Claude，一个由Anthropic开发的AI助手。".to_string())
            .temperature(0.8)
            .max_tokens(1500)
            .build();

        // 手动设置 provider
        let mut config = anthropic_config;
        config.provider = Some("anthropic".to_string());

        let adapter = StandaloneAgentAdapter::new(config);
        
        match adapter.create_agent("claude_agent".to_string(), None).await {
            Ok(_) => {
                match adapter.chat("claude_agent", "你和GPT有什么不同？").await {
                    Ok(response) => println!("Claude 回复: {}\n", response.content),
                    Err(e) => println!("Claude 聊天错误: {}\n", e),
                }
            }
            Err(e) => println!("创建 Claude Agent 失败: {}\n", e),
        }
    } else {
        println!("跳过 Anthropic 示例 (缺少 ANTHROPIC_API_KEY)\n");
    }

    // 示例 3: Cohere Command
    println!("3. 使用 Cohere Command:");
    if std::env::var("COHERE_API_KEY").is_ok() {
        let cohere_config = ConfigBuilder::new()
            .model("command-r".to_string())
            .system_prompt("你是Command，一个由Cohere开发的AI助手。".to_string())
            .temperature(0.6)
            .max_tokens(800)
            .build();

        // 手动设置 provider
        let mut config = cohere_config;
        config.provider = Some("cohere".to_string());

        let adapter = StandaloneAgentAdapter::new(config);
        
        match adapter.create_agent("cohere_agent".to_string(), None).await {
            Ok(_) => {
                match adapter.chat("cohere_agent", "请介绍一下Cohere公司").await {
                    Ok(response) => println!("Cohere 回复: {}\n", response.content),
                    Err(e) => println!("Cohere 聊天错误: {}\n", e),
                }
            }
            Err(e) => println!("创建 Cohere Agent 失败: {}\n", e),
        }
    } else {
        println!("跳过 Cohere 示例 (缺少 COHERE_API_KEY)\n");
    }

    // 示例 4: Google Gemini
    println!("4. 使用 Google Gemini:");
    if std::env::var("GEMINI_API_KEY").is_ok() {
        let gemini_config = ConfigBuilder::new()
            .model("gemini-pro".to_string())
            .system_prompt("你是Gemini，一个由Google开发的AI助手。".to_string())
            .temperature(0.9)
            .max_tokens(1200)
            .build();

        // 手动设置 provider
        let mut config = gemini_config;
        config.provider = Some("gemini".to_string());

        let adapter = StandaloneAgentAdapter::new(config);
        
        match adapter.create_agent("gemini_agent".to_string(), None).await {
            Ok(_) => {
                match adapter.chat("gemini_agent", "Google的AI发展历程是怎样的？").await {
                    Ok(response) => println!("Gemini 回复: {}\n", response.content),
                    Err(e) => println!("Gemini 聊天错误: {}\n", e),
                }
            }
            Err(e) => println!("创建 Gemini Agent 失败: {}\n", e),
        }
    } else {
        println!("跳过 Gemini 示例 (缺少 GEMINI_API_KEY)\n");
    }

    // 示例 5: 比较不同模型的回答
    println!("5. 比较不同模型的回答:");
    let question = "什么是人工智能的未来？";
    println!("问题: {}\n", question);

    let providers = vec![
        ("openai", "gpt-3.5-turbo", "OPENAI_API_KEY"),
        ("anthropic", "claude-3-haiku", "ANTHROPIC_API_KEY"),
        ("cohere", "command-r", "COHERE_API_KEY"),
        ("gemini", "gemini-pro", "GEMINI_API_KEY"),
    ];

    for (provider, model, env_key) in providers {
        if std::env::var(env_key).is_ok() {
            let config = ConfigBuilder::new()
                .model(model.to_string())
                .system_prompt(format!("你是一个使用{}模型的AI助手。", model))
                .temperature(0.7)
                .build();

            // 手动设置 provider
            let mut config = config;
            config.provider = Some(provider.to_string());

            let adapter = StandaloneAgentAdapter::new(config);
            let agent_id = format!("{}_comparison", provider);
            
            match adapter.create_agent(agent_id.clone(), None).await {
                Ok(_) => {
                    match adapter.chat(&agent_id, question).await {
                        Ok(response) => {
                            println!("【{}】回答:", model);
                            println!("{}\n", response.content);
                        }
                        Err(e) => println!("【{}】错误: {}\n", model, e),
                    }
                }
                Err(e) => println!("创建【{}】Agent 失败: {}\n", model, e),
            }
        } else {
            println!("跳过【{}】(缺少 {})\n", model, env_key);
        }
    }

    println!("=== 多 Provider 示例完成 ===");
    println!("\n使用说明:");
    println!("1. 设置相应的环境变量来启用不同的 Provider:");
    println!("   export OPENAI_API_KEY=your_openai_key");
    println!("   export ANTHROPIC_API_KEY=your_anthropic_key");
    println!("   export COHERE_API_KEY=your_cohere_key");
    println!("   export GEMINI_API_KEY=your_gemini_key");
    println!("2. 运行示例:");
    println!("   cargo run --example multi_provider_usage");

    Ok(())
}