# Rig Agent

基于 [rig-core](https://github.com/0xPlaygrounds/rig) 的 AI Agent 库，提供了简单易用的 AI 对话接口。

## 特性

- 🤖 支持多种 AI 提供商（OpenAI、Anthropic）
- 💬 对话历史管理
- 🔧 工具系统集成
- 🎯 多种适配器支持（独立运行、Tauri 集成）
- 📊 Agent 统计和监控
- 🔄 异步处理

## 快速开始

### 基本使用

```rust
use rig_agent::{AgentConfig, StandaloneAgentAdapter, AgentAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建配置
    let config = AgentConfig {
        model: "gpt-3.5-turbo".to_string(),
        provider: Some("openai".to_string()),
        preamble: Some("你是一个友好的AI助手".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(1000),
        enable_tools: false,
        history_limit: Some(10),
        extra_params: std::collections::HashMap::new(),
    };

    // 创建适配器
    let adapter = StandaloneAgentAdapter::new(config.clone());

    // 创建 Agent
    adapter.create_agent("my_bot".to_string(), Some(config)).await?;

    // 发送消息
    let response = adapter.chat("my_bot", "你好！").await?;
    println!("AI 回复: {}", response.content);

    Ok(())
}
```

### 环境变量

设置相应的 API 密钥：

```bash
export OPENAI_API_KEY="your-openai-api-key"
export ANTHROPIC_API_KEY="your-anthropic-api-key"
```

## 架构设计

### 核心组件

1. **AgentManager**: 核心 Agent 管理器，负责创建和管理 Agent 实例
2. **AgentWrapper**: Agent 包装器，支持不同的 AI 提供商
3. **AgentAdapter**: 适配器特征，支持不同的运行环境
4. **ToolManager**: 工具管理器，处理 Agent 工具调用

### 支持的 AI 提供商

- **OpenAI**: GPT-3.5, GPT-4 等模型
- **Anthropic**: Claude 系列模型

### 适配器类型

- **StandaloneAgentAdapter**: 独立运行适配器
- **TauriAgentAdapter**: Tauri 应用集成适配器（需要 `tauri-support` 特性）

## API 文档

### AgentConfig

Agent 配置结构：

```rust
pub struct AgentConfig {
    pub model: String,                    // AI 模型名称
    pub provider: Option<String>,         // 提供商名称
    pub preamble: Option<String>,         // 系统提示
    pub temperature: Option<f32>,         // 温度参数
    pub max_tokens: Option<u32>,          // 最大令牌数
    pub enable_tools: bool,               // 是否启用工具
    pub history_limit: Option<usize>,     // 历史消息限制
    pub extra_params: HashMap<String, serde_json::Value>, // 额外参数
}
```

### AgentAdapter 特征

```rust
pub trait AgentAdapter {
    async fn chat(&self, agent_id: &str, message: &str) -> AgentResult<AgentResponse>;
    async fn create_agent(&self, agent_id: String, config: Option<AgentConfig>) -> AgentResult<()>;
    async fn remove_agent(&self, agent_id: &str) -> AgentResult<bool>;
    async fn list_agents(&self) -> AgentResult<Vec<String>>;
}
```

### AgentResponse

AI 响应结构：

```rust
pub struct AgentResponse {
    pub id: String,                       // 响应 ID
    pub agent_id: String,                 // Agent ID
    pub content: String,                  // 响应内容
    pub timestamp: DateTime<Utc>,         // 时间戳
    pub model: String,                    // 使用的模型
    pub usage: Option<TokenUsage>,        // 使用统计
    pub tool_calls: Option<Vec<ToolCall>>, // 工具调用
    pub finish_reason: Option<String>,    // 完成原因
}
```

## 示例

### 运行示例

```bash
# 设置 API 密钥
export OPENAI_API_KEY="your-api-key"

# 运行简单聊天示例
cargo run --example simple_chat
```

### 更多示例

查看 `examples/` 目录获取更多使用示例。

## 开发

### 构建

```bash
cargo build
```

### 测试

```bash
cargo test
```

### 检查

```bash
cargo check
```

## 特性标志

- `tauri-support`: 启用 Tauri 集成支持

```toml
[dependencies]
rig-agent = { version = "0.1.0", features = ["tauri-support"] }
```

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License

## 基于的项目

- [rig-core](https://github.com/0xPlaygrounds/rig) - 强大的 Rust AI 框架