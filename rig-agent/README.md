# Rig Agent - 基于 rig-core 的 AI Agent 模块

这是一个基于 [rig-core](https://github.com/0xPlaygrounds/rig) 构建的强大 AI Agent 模块，支持多种 AI 提供商、工具调用、对话管理等功能。

## 功能特性

- 🤖 **多 AI 提供商支持**: OpenAI、Anthropic、Cohere、Gemini
- 🛠️ **工具调用系统**: 内置工具 + 自定义工具支持
- 💬 **对话管理**: 智能历史管理和上下文保持
- ⚙️ **灵活配置**: 可配置的 Agent 参数和行为
- 📊 **性能监控**: 详细的日志记录和性能指标
- 🔒 **错误处理**: 完善的错误处理和重试机制
- 🧪 **全面测试**: 单元测试、集成测试和基准测试

## 快速开始

### 1. 安装依赖

在您的 `Cargo.toml` 中添加：

```toml
[dependencies]
rig-agent = { path = "./rig-agent" }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 2. 设置环境变量

```bash
# OpenAI
export OPENAI_API_KEY="your_openai_api_key"

# Anthropic (可选)
export ANTHROPIC_API_KEY="your_anthropic_api_key"

# Cohere (可选)
export COHERE_API_KEY="your_cohere_api_key"

# Gemini (可选)
export GEMINI_API_KEY="your_gemini_api_key"
```

### 3. 基本使用

```rust
use rig_agent::{
    core::{agent::AgentManager, types::AgentConfig},
    error::AgentResult,
};

#[tokio::main]
async fn main() -> AgentResult<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建 Agent 配置
    let config = AgentConfig {
        model: "gpt-3.5-turbo".to_string(),
        provider: Some("openai".to_string()),
        system_prompt: Some("你是一个友好的AI助手。".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(500),
        enable_tools: true,
        history_limit: Some(20),
        extra_params: std::collections::HashMap::new(),
    };

    // 创建 Agent 管理器
    let mut manager = AgentManager::new(config);

    // 创建 Agent
    let agent_id = "my_agent";
    manager.create_agent(agent_id.to_string(), None).await?;

    // 开始对话
    let response = manager.chat(agent_id, "你好！").await?;
    println!("助手: {}", response.content);

    Ok(())
}
```

## 详细功能

### Agent 管理

```rust
// 创建 Agent
manager.create_agent("agent_1".to_string(), None).await?;

// 列出所有 Agent
let agents = manager.list_agents().await;

// 删除 Agent
manager.remove_agent("agent_1").await;
```

### 对话管理

```rust
// 发送消息
let response = manager.chat("agent_1", "你好").await?;

// 获取对话历史
let history = manager.get_conversation_history("agent_1").await?;

// 清除对话历史
manager.clear_conversation_history("agent_1").await?;
```

### 配置管理

```rust
// 获取配置
let config = manager.get_agent_config("agent_1").await?;

// 更新配置
let mut new_config = config.clone();
new_config.temperature = Some(0.5);
manager.update_agent_config("agent_1", new_config).await?;
```

### 工具系统

#### 使用内置工具

```rust
let config = AgentConfig {
    enable_tools: true,
    // ... 其他配置
};

// 内置工具包括：
// - calculator: 数学计算
// - current_time: 获取当前时间
// - weather: 天气查询（示例）
```

#### 创建自定义工具

```rust
use rig_agent::tools::CustomTool;
use async_trait::async_trait;

struct MyCustomTool;

#[async_trait]
impl CustomTool for MyCustomTool {
    fn name(&self) -> &str {
        "my_tool"
    }
    
    fn description(&self) -> &str {
        "我的自定义工具"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "输入参数"
                }
            },
            "required": ["input"]
        })
    }
    
    async fn execute(&self, arguments: &str) -> AgentResult<String> {
        // 工具逻辑
        Ok("工具执行结果".to_string())
    }
}

// 添加自定义工具
let custom_tool = Box::new(MyCustomTool);
manager.get_tool_manager_mut().add_custom_tool(custom_tool);
```

## 支持的 AI 提供商

### OpenAI

```rust
let config = AgentConfig {
    model: "gpt-3.5-turbo".to_string(), // 或 "gpt-4"
    provider: Some("openai".to_string()),
    // ...
};
```

### Anthropic

```rust
let config = AgentConfig {
    model: "claude-3-sonnet-20240229".to_string(),
    provider: Some("anthropic".to_string()),
    // ...
};
```

### Cohere

```rust
let config = AgentConfig {
    model: "command".to_string(),
    provider: Some("cohere".to_string()),
    // ...
};
```

### Gemini

```rust
let config = AgentConfig {
    model: "gemini-pro".to_string(),
    provider: Some("gemini".to_string()),
    // ...
};
```

## 配置选项

```rust
pub struct AgentConfig {
    /// AI 模型名称
    pub model: String,
    /// Provider 名称 (openai, anthropic, cohere, gemini)
    pub provider: Option<String>,
    /// 系统提示
    pub system_prompt: Option<String>,
    /// 温度参数 (0.0-2.0)
    pub temperature: Option<f32>,
    /// 最大令牌数
    pub max_tokens: Option<u32>,
    /// 是否启用工具
    pub enable_tools: bool,
    /// 历史消息限制
    pub history_limit: Option<usize>,
    /// 其他配置参数
    pub extra_params: HashMap<String, serde_json::Value>,
}
```

## 错误处理

```rust
use rig_agent::error::{AgentError, AgentResult};

match manager.chat("agent_1", "hello").await {
    Ok(response) => println!("成功: {}", response.content),
    Err(AgentError::AgentNotFound(id)) => println!("Agent {} 不存在", id),
    Err(AgentError::Network(msg)) => println!("网络错误: {}", msg),
    Err(AgentError::RateLimit) => println!("请求过于频繁"),
    Err(e) => println!("其他错误: {}", e),
}
```

## 日志记录

使用 `tracing` 进行日志记录：

```rust
use tracing::{info, warn, error, debug};
use tracing_subscriber;

// 初始化日志
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

// 日志会自动记录 Agent 的各种操作
```

## 测试

### 运行单元测试

```bash
cargo test
```

### 运行集成测试

```bash
cargo test --test integration_tests
```

### 运行基准测试

```bash
cargo bench
```

## 示例

查看 `examples/` 目录中的完整示例：

- `agent_usage_example.rs`: 基本使用示例
- `rig_core_example.rs`: rig-core 集成示例

运行示例：

```bash
cargo run --example agent_usage_example
```

## 架构设计

```
rig-agent/
├── src/
│   ├── core/           # 核心模块
│   │   ├── agent.rs    # Agent 管理器
│   │   ├── types.rs    # 类型定义
│   │   └── mod.rs
│   ├── tools/          # 工具系统
│   │   └── mod.rs
│   ├── adapters/       # 适配器
│   │   ├── axum_adapter.rs
│   │   ├── tauri_adapter.rs
│   │   └── mod.rs
│   ├── error.rs        # 错误处理
│   ├── lib.rs
│   └── main.rs
├── tests/              # 集成测试
├── benches/            # 基准测试
├── examples/           # 示例代码
└── Cargo.toml
```

## 性能优化

- 使用 `tokio` 异步运行时提高并发性能
- 智能的对话历史管理避免内存泄漏
- 工具调用缓存减少重复计算
- 详细的性能监控和日志记录

## 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 更新日志

### v0.1.0 (当前版本)

- ✅ 基于 rig-core 的完整重构
- ✅ 多 AI 提供商支持
- ✅ 工具调用系统
- ✅ 对话管理功能
- ✅ 错误处理和日志记录
- ✅ 完整的测试套件

## 支持

如果您遇到问题或有建议，请：

1. 查看 [文档](./docs/)
2. 搜索 [Issues](../../issues)
3. 创建新的 Issue

## 相关项目

- [rig-core](https://github.com/0xPlaygrounds/rig) - 底层 AI 框架
- [tokio](https://tokio.rs/) - 异步运行时
- [tracing](https://tracing.rs/) - 日志记录框架