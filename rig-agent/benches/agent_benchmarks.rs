//! Agent 模块性能基准测试

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rig_agent::{
    core::{agent::AgentManager, types::AgentConfig},
    tools::{BuiltinTools, ToolCall},
};
use std::env;
use tokio::runtime::Runtime;

/// 设置测试环境
fn setup_bench_env() {
    if env::var("OPENAI_API_KEY").is_err() {
        env::set_var("OPENAI_API_KEY", "test-key-for-benchmarking");
    }
}

/// 创建基准测试用的配置
fn create_bench_config() -> AgentConfig {
    AgentConfig {
        model: "gpt-3.5-turbo".to_string(),
        provider: Some("openai".to_string()),
        system_prompt: Some("你是一个基准测试助手。".to_string()),
        temperature: Some(0.1),
        max_tokens: Some(50),
        enable_tools: false,
        history_limit: Some(20),
        extra_params: std::collections::HashMap::new(),
    }
}

/// 基准测试：Agent 创建性能
fn bench_agent_creation(c: &mut Criterion) {
    setup_bench_env();
    let rt = Runtime::new().unwrap();
    
    c.bench_function("agent_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = create_bench_config();
                let mut manager = AgentManager::new(config);
                
                let agent_id = format!("bench_agent_{}", uuid::Uuid::new_v4());
                let _ = manager.create_agent(black_box(agent_id), None).await;
            });
        });
    });
}

/// 基准测试：工具执行性能
fn bench_tool_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("calculator_tool_execution", |b| {
        b.iter(|| {
            rt.block_on(async {
                let tools = BuiltinTools::new();
                let tool_call = ToolCall {
                    id: "bench_calc".to_string(),
                    name: "calculator".to_string(),
                    arguments: black_box(r#"{"expression": "123+456*789"}"#.to_string()),
                    timestamp: chrono::Utc::now(),
                };
                
                let _ = tools.execute_tool(&tool_call).await;
            });
        });
    });
}

/// 基准测试：对话历史管理性能
fn bench_conversation_history(c: &mut Criterion) {
    setup_bench_env();
    let rt = Runtime::new().unwrap();
    
    c.bench_function("conversation_history_management", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = create_bench_config();
                let mut manager = AgentManager::new(config);
                let agent_id = "bench_history_agent";
                
                if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
                    // 添加多条消息到历史
                    for i in 0..10 {
                        let _ = manager.chat(agent_id, &format!("测试消息 {}", i)).await;
                    }
                    
                    // 获取历史
                    let _ = manager.get_conversation_history(black_box(agent_id)).await;
                    
                    // 清除历史
                    let _ = manager.clear_conversation_history(agent_id).await;
                }
            });
        });
    });
}

/// 基准测试：并发 Agent 操作
fn bench_concurrent_operations(c: &mut Criterion) {
    setup_bench_env();
    let rt = Runtime::new().unwrap();
    
    c.bench_function("concurrent_agent_list", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = create_bench_config();
                let manager = AgentManager::new(config);
                
                // 并发获取 Agent 列表
                let mut handles = vec![];
                for _ in 0..10 {
                    let agents = manager.list_agents().await;
                    black_box(agents);
                }
            });
        });
    });
}

/// 基准测试：配置管理性能
fn bench_config_management(c: &mut Criterion) {
    setup_bench_env();
    let rt = Runtime::new().unwrap();
    
    c.bench_function("config_management", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = create_bench_config();
                let mut manager = AgentManager::new(config.clone());
                let agent_id = "bench_config_agent";
                
                if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
                    // 获取配置
                    let _ = manager.get_agent_config(black_box(agent_id)).await;
                    
                    // 更新配置
                    let mut new_config = config;
                    new_config.temperature = Some(0.8);
                    let _ = manager.update_agent_config(agent_id, black_box(new_config)).await;
                }
            });
        });
    });
}

/// 基准测试：内存使用情况
fn bench_memory_usage(c: &mut Criterion) {
    setup_bench_env();
    let rt = Runtime::new().unwrap();
    
    c.bench_function("memory_usage_large_history", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut config = create_bench_config();
                config.history_limit = Some(1000); // 大历史限制
                
                let mut manager = AgentManager::new(config);
                let agent_id = "bench_memory_agent";
                
                if manager.create_agent(agent_id.to_string(), None).await.is_ok() {
                    // 创建大量历史记录
                    for i in 0..100 {
                        let message = format!("这是一条很长的测试消息，用于测试内存使用情况。消息编号：{}。这条消息包含了足够的文本来模拟真实的对话场景。", i);
                        let _ = manager.chat(agent_id, &message).await;
                    }
                    
                    let history = manager.get_conversation_history(black_box(agent_id)).await;
                    black_box(history);
                }
            });
        });
    });
}

criterion_group!(
    benches,
    bench_agent_creation,
    bench_tool_execution,
    bench_conversation_history,
    bench_concurrent_operations,
    bench_config_management,
    bench_memory_usage
);

criterion_main!(benches);