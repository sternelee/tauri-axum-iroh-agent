//! Agent 使用示例
//! 
//! 这个示例展示了如何使用重构后的 rig-agent 模块来创建和管理 AI Agent

use rig_agent::{
    core::{
        agent::AgentManager,
        types::{AgentConfig, AgentRole},
    },
    tools::{ToolManager, CustomTool},
    error::AgentResult,
};
use std::env;
use tracing::{info, Level};
use tracing_subscriber;

/// 自定义工具示例：文本长度计算器
struct TextLengthTool;

#[async_trait::async_trait]
impl CustomTool for TextLengthTool {
    fn name(&self) -> &str {
        "text_length"
    }
    
    fn description(&self) -> &str {
        "计算文本的字符长度"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "要计算长度的文本"
                }
            },
            "required": ["text"]
        })
    }
    
    async fn execute(&self, arguments: &str) -> AgentResult<String> {
        let args: serde_json::Value = serde_json::from_str(arguments)?;
        let text = args["text"]
            .as_str()
            .ok_or_else(|| rig_agent::error::AgentError::tool("缺少 text 参数"))?;
        
        let length = text.chars().count();
        Ok(format!("文本「{}」的字符长度为：{}", text, length))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("开始 Agent 使用示例");

    // 检查环境变量
    if env::var("OPENAI_API_KEY").is_err() {
        println!("警告：未设置 OPENAI_API_KEY 环境变量");
        println!("请设置您的 OpenAI API 密钥：export OPENAI_API_KEY=your_api_key");
        println!("继续运行示例（可能会失败）...\n");
    }

    // 1. 创建 Agent 配置
    let config = AgentConfig {
        model: "gpt-3.5-turbo".to_string(),
        provider: Some("openai".to_string()),
        system_prompt: Some("你是一个友好的AI助手，擅长回答问题和使用工具。".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(500),
        enable_tools: true,
        history_limit: Some(20),
        extra_params: std::collections::HashMap::new(),
    };

    // 2. 创建 Agent 管理器
    let mut manager = AgentManager::new(config);

    // 3. 添加自定义工具
    let text_length_tool = Box::new(TextLengthTool);
    manager.get_tool_manager_mut().add_custom_tool(text_length_tool);

    // 4. 创建 Agent
    let agent_id = "demo_agent";
    match manager.create_agent(agent_id.to_string(), None).await {
        Ok(_) => {
            info!("Agent {} 创建成功", agent_id);
        }
        Err(e) => {
            eprintln!("Agent 创建失败: {}", e);
            return Ok(());
        }
    }

    // 5. 展示基本对话功能
    println!("\n=== 基本对话示例 ===");
    let messages = vec![
        "你好！",
        "请介绍一下你自己",
        "你能做什么？",
    ];

    for message in messages {
        println!("用户: {}", message);
        
        match manager.chat(agent_id, message).await {
            Ok(response) => {
                println!("助手: {}\n", response.content);
                
                // 显示响应详情
                if let Some(usage) = response.usage {
                    println!("令牌使用: 提示={}, 完成={}, 总计={}", 
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
                }
            }
            Err(e) => {
                eprintln!("对话失败: {}\n", e);
            }
        }
        
        // 短暂延迟
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // 6. 展示工具调用功能
    println!("\n=== 工具调用示例 ===");
    let tool_messages = vec![
        "请计算 15 + 27 * 3",
        "现在几点了？",
        "北京今天天气怎么样？",
    ];

    for message in tool_messages {
        println!("用户: {}", message);
        
        match manager.chat(agent_id, message).await {
            Ok(response) => {
                println!("助手: {}", response.content);
                
                if let Some(tool_calls) = response.tool_calls {
                    println!("工具调用: {} 个工具被调用", tool_calls.len());
                    for tool_call in tool_calls {
                        println!("  - {}: {}", tool_call.name, tool_call.arguments);
                    }
                }
                println!();
            }
            Err(e) => {
                eprintln!("工具调用失败: {}\n", e);
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // 7. 展示对话历史管理
    println!("\n=== 对话历史管理 ===");
    match manager.get_conversation_history(agent_id).await {
        Ok(history) => {
            println!("对话历史统计:");
            println!("  - 总消息数: {}", history.total_messages);
            println!("  - 创建时间: {}", history.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("  - 最后活动: {}", history.last_activity.format("%Y-%m-%d %H:%M:%S"));
            
            if let Some(total_tokens) = history.total_tokens {
                println!("  - 估计令牌数: {}", total_tokens);
            }

            println!("\n最近的对话:");
            let recent_messages = history.messages.iter().rev().take(6).collect::<Vec<_>>();
            for msg in recent_messages.iter().rev() {
                let role_str = match msg.role {
                    AgentRole::User => "用户",
                    AgentRole::Assistant => "助手",
                    AgentRole::System => "系统",
                    AgentRole::Tool => "工具",
                };
                
                let content_preview = if msg.content.len() > 100 {
                    format!("{}...", &msg.content[..100])
                } else {
                    msg.content.clone()
                };
                
                println!("  [{}] {}: {}", 
                    msg.timestamp.format("%H:%M:%S"), role_str, content_preview);
            }
        }
        Err(e) => {
            eprintln!("获取对话历史失败: {}", e);
        }
    }

    // 8. 展示配置管理
    println!("\n=== 配置管理示例 ===");
    match manager.get_agent_config(agent_id).await {
        Ok(config) => {
            println!("当前配置:");
            println!("  - 模型: {}", config.model);
            println!("  - 提供商: {:?}", config.provider);
            println!("  - 温度: {:?}", config.temperature);
            println!("  - 最大令牌: {:?}", config.max_tokens);
            println!("  - 启用工具: {}", config.enable_tools);
            println!("  - 历史限制: {:?}", config.history_limit);
        }
        Err(e) => {
            eprintln!("获取配置失败: {}", e);
        }
    }

    // 9. 展示工具管理
    println!("\n=== 工具管理示例 ===");
    let tool_manager = manager.get_tool_manager();
    let tool_definitions = tool_manager.get_all_tool_definitions();
    
    println!("可用工具 ({} 个):", tool_definitions.len());
    for tool_def in tool_definitions {
        println!("  - {}: {}", tool_def.name, tool_def.description);
    }

    // 10. 清理
    println!("\n=== 清理 ===");
    
    // 清除对话历史
    if let Err(e) = manager.clear_conversation_history(agent_id).await {
        eprintln!("清除对话历史失败: {}", e);
    } else {
        println!("对话历史已清除");
    }
    
    // 删除 Agent
    if manager.remove_agent(agent_id).await {
        println!("Agent {} 已删除", agent_id);
    } else {
        println!("Agent {} 删除失败", agent_id);
    }

    // 11. 展示多 Agent 管理
    println!("\n=== 多 Agent 管理示例 ===");
    
    // 创建多个不同配置的 Agent
    let agents_config = vec![
        ("creative_agent", AgentConfig {
            model: "gpt-3.5-turbo".to_string(),
            provider: Some("openai".to_string()),
            system_prompt: Some("你是一个富有创意的助手，善于创作和想象。".to_string()),
            temperature: Some(0.9),
            max_tokens: Some(300),
            enable_tools: false,
            history_limit: Some(10),
            extra_params: std::collections::HashMap::new(),
        }),
        ("analytical_agent", AgentConfig {
            model: "gpt-3.5-turbo".to_string(),
            provider: Some("openai".to_string()),
            system_prompt: Some("你是一个分析型助手，善于逻辑推理和数据分析。".to_string()),
            temperature: Some(0.2),
            max_tokens: Some(400),
            enable_tools: true,
            history_limit: Some(15),
            extra_params: std::collections::HashMap::new(),
        }),
    ];

    for (agent_name, agent_config) in agents_config {
        match manager.create_agent(agent_name.to_string(), Some(agent_config)).await {
            Ok(_) => println!("创建 Agent: {}", agent_name),
            Err(e) => eprintln!("创建 Agent {} 失败: {}", agent_name, e),
        }
    }

    let all_agents = manager.list_agents().await;
    println!("当前活跃的 Agent: {:?}", all_agents);

    info!("Agent 使用示例完成");
    Ok(())
}