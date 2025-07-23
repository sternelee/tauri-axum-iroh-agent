//! 基于 rig-core 官方示例的实现

use rig_core::{
    completion::Prompt,
    providers::openai::{Client, GPT_3_5_TURBO},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从环境变量获取 API 密钥
    let openai_api_key = std::env::var("OPENAI_API_KEY")?;
    
    // 创建 OpenAI 客户端
    let openai_client = Client::new(&openai_api_key);
    
    // 创建完成模型
    let gpt3_5 = openai_client.completion_model(GPT_3_5_TURBO);
    
    // 发送提示并获取响应
    let response = gpt3_5
        .prompt("你好，请介绍一下你自己")
        .await?;
    
    println!("AI 回复: {}", response);
    
    Ok(())
}