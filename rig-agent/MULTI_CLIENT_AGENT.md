# 多客户端 Agent 系统

本文档介绍了重新设计的 rig-agent 多客户端 Agent 系统，该系统基于 rig-core 的 `DynClientBuilder` 实现，支持多个 AI 提供商客户端的统一管理。

## 设计理念

参考 [rig-core 的 dyn_client.rs 示例](https://github.com/0xPlaygrounds/rig/blob/main/rig-core/examples/dyn_client.rs)，新的 Agent 系统具有以下特点：

1. **统一客户端管理**: 使用单个 `DynClientBuilder` 实例管理多个客户端
2. **动态客户端创建**: 支持运行时动态创建不同提供商的 Agent
3. **客户端注册表**: 维护可用客户端的注册表
4. **简化的 Agent 包装**: 移除基于枚举的方法，直接使用动态客户端
5. **更好的错误处理**: 改进客户端创建和管理的错误处理

## 核心组件

### ClientRegistry

客户端注册表，负责管理多个 AI 提供商客户端：

```rust
pub struct ClientRegistry {
    builder: DynClientBuilder,
    clients: HashMap<String, ClientConfig>,
}
```

### ClientConfig

客户端配置结构：

```rust
pub struct ClientConfig {
    pub provider: String,        // 提供商名称
    pub default_model: String,   // 默认模型
    pub api_key: Option<String>, // API 密钥
    pub base_url: Option<String>, // 基础 URL
}
```

### AgentManager

Agent 管理器，整合了客户端注册表和 Agent 管理功能：

```rust
pub struct AgentManager {
    agents: RwLock<HashMap<String, Agent>>,
    default_config: AgentConfig,
    tool_manager: ToolManager,
    client_registry: ClientRegistry,
}
```

## 使用方法

### 1. 基本使用

```rust
use rig_agent::core::agent::{AgentManager, ClientConfig};
use rig_agent::core::types::AgentConfig;

// 创建 Agent 管理器
let default_config = AgentConfig::default();
let mut manager = AgentManager::new(default_config);

// 查看已注册的客户端
let clients = manager.get_registered_clients();
println!("已注册的客户端: {:?}", clients);
```

### 2. 创建不同提供商的 Agent

```rust
// OpenAI Agent
let openai_config = AgentConfig {
    model: "gpt-3.5-turbo".to_string(),
    provider: Some("openai".to_string()),
    preamble: Some("你是一个有用的AI助手。".to_string()),
    temperature: Some(0.7),
    max_tokens: Some(500),
    enable_tools: false,
    history_limit: Some(10),
    extra_params: HashMap::new(),
};

manager.create_agent("openai_agent".to_string(), Some(openai_config)).await?;

// Anthropic Agent
let anthropic_config = AgentConfig {
    model: "claude-3-sonnet-20240229".to_string(),
    provider: Some("anthropic".to_string()),
    preamble: Some("你是一个专业的AI助手。".to_string()),
    temperature: Some(0.5),
    max_tokens: Some(800),
    enable_tools: false,
    history_limit: Some(15),
    extra_params: HashMap::new(),
};

manager.create_agent("anthropic_agent".to_string(), Some(anthropic_config)).await?;
```

### 3. 发送消息

```rust
// 聊天消息（保存历史）
let response = manager.chat("openai_agent", "你好，请介绍一下你自己。").await?;
println!("响应: {}", response.content);

// 简单 prompt（不保存历史）
let response = manager.prompt("openai_agent", "请用一句话总结今天的天气。").await?;
println!("响应: {}", response);
```

### 4. 动态注册客户端

```rust
// 注册自定义客户端
let custom_config = ClientConfig {
    provider: "custom_provider".to_string(),
    default_model: "custom-model".to_string(),
    api_key: Some("your-api-key".to_string()),
    base_url: Some("https://api.custom-provider.com".to_string()),
};

manager.register_client("custom_provider".to_string(), custom_config)?;
```

### 5. 多轮对话

```rust
// 创建聊天 Agent
let chat_config = AgentConfig {
    model: "gpt-3.5-turbo".to_string(),
    provider: Some("openai".to_string()),
    preamble: Some("你是一个友好的聊天机器人。".to_string()),
    temperature: Some(0.8),
    max_tokens: Some(300),
    enable_tools: false,
    history_limit: Some(20),
    extra_params: HashMap::new(),
};

manager.create_agent("chat_agent".to_string(), Some(chat_config)).await?;

// 多轮对话
let messages = vec!["你好！", "今天天气怎么样？", "谢谢！"];
for message in messages {
    let response = manager.chat("chat_agent", message).await?;
    println!("AI: {}", response.content);
}
```

### 6. 获取对话历史

```rust
let history = manager.get_conversation_history("chat_agent").await?;
println!("总消息数: {}", history.total_messages);
println!("总令牌数: {:?}", history.total_tokens);
```

### 7. Agent 管理

```rust
// 列出所有 Agent
let agents = manager.list_agents().await;
println!("Agent 列表: {:?}", agents);

// 获取 Agent 统计信息
for agent_id in &agents {
    let stats = manager.get_agent_stats(agent_id).await?;
    println!("Agent {}: {} 条消息", agent_id, stats.total_messages);
}

// 删除 Agent
let removed = manager.remove_agent("chat_agent").await;
println!("删除结果: {}", removed);
```

## 环境变量配置

系统会自动检测以下环境变量来注册默认客户端：

- `OPENAI_API_KEY`: OpenAI API 密钥
- `ANTHROPIC_API_KEY`: Anthropic API 密钥

## 运行示例

```bash
# 设置环境变量
export OPENAI_API_KEY="your-openai-api-key"
export ANTHROPIC_API_KEY="your-anthropic-api-key"

# 运行多客户端示例
cargo run --example multi_client_agent
```

## 主要改进

### 1. 统一客户端管理

- 使用单个 `DynClientBuilder` 实例
- 统一的客户端注册和管理机制
- 自动检测和注册可用的客户端

### 2. 简化的 Agent 创建

- 移除复杂的枚举包装器
- 直接使用 `BoxCompletionModel` 类型
- 更简洁的 Agent 创建流程

### 3. 更好的错误处理

- 改进的客户端创建错误处理
- 更详细的错误信息
- 更好的错误恢复机制

### 4. 动态配置支持

- 运行时动态注册新客户端
- 支持自定义 API 端点和配置
- 灵活的客户端配置管理

### 5. 增强的管理功能

- Agent 统计信息
- 对话历史管理
- 客户端状态监控

## 与 rig-core 的集成

新的 Agent 系统完全基于 rig-core 的 `DynClientBuilder`，提供了：

- 与 rig-core 的完全兼容性
- 支持所有 rig-core 支持的 AI 提供商
- 利用 rig-core 的最新功能和改进

## 扩展性

系统设计为高度可扩展：

- 轻松添加新的 AI 提供商
- 支持自定义客户端配置
- 可插拔的工具系统
- 灵活的 Agent 配置选项

## 性能优化

- 使用 `RwLock` 进行并发安全的 Agent 管理
- 智能的客户端缓存和重用
- 高效的对话历史管理
- 优化的内存使用

这个重新设计的 Agent 系统提供了更强大、更灵活的多客户端支持，同时保持了与 rig-core 的紧密集成和良好的性能表现。
