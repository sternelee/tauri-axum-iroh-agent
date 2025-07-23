# Tauri-Axum-Iroh Agent

这是一个展示如何将iroh P2P文件传输和实时聊天功能集成到不同运行环境的项目。项目包含了一个通用的iroh模块，同时支持文件传输和实时聊天，以及在tauri桌面应用和axum web服务中的集成示例。

## 项目结构

```
├── iroh-node/          # 通用iroh P2P文件传输模块
├── tauri-app/          # Tauri桌面应用示例
├── axum-app/           # Axum Web服务示例
└── README.md           # 项目说明文档
```

## 重构成果

### 🎯 重构目标达成

本项目成功将原本强依赖tauri框架的iroh文件传输功能重构为通用的模块化实现，并新增了实时聊天功能：

1. ✅ **核心逻辑解耦** - 将P2P传输逻辑从tauri框架中完全分离
2. ✅ **标准化接口** - 设计了统一的API接口，支持多种运行环境
3. ✅ **适配器模式** - 创建了tauri、axum、独立运行等多种适配器
4. ✅ **错误处理** - 实现了完整的错误处理机制和类型安全
5. ✅ **进度回调** - 保留了完整的传输进度通知功能
6. ✅ **模块化设计** - 提供了清晰的模块导出和类型定义
7. ✅ **实时聊天** - 基于iroh gossip协议的P2P实时聊天功能
8. ✅ **文件分享** - 在聊天中直接分享和下载文件

### 🏗️ 架构设计

#### 核心模块 (`iroh-node`)

```
iroh-node/
├── src/
│   ├── core/                    # 核心功能模块
│   │   ├── client.rs           # iroh文件传输客户端
│   │   ├── chat.rs             # 聊天类型定义
│   │   ├── chat_client.rs      # iroh聊天客户端
│   │   ├── integrated_client.rs # 集成客户端（文件+聊天）
│   │   ├── types.rs            # 类型定义
│   │   ├── progress.rs         # 进度回调系统
│   │   ├── error.rs            # 错误处理
│   │   └── tests.rs            # 单元测试
│   ├── adapters/               # 适配器层
│   │   ├── standalone.rs       # 独立运行适配器
│   │   ├── tauri_adapter.rs    # Tauri适配器
│   │   └── axum_adapter.rs     # Axum适配器
│   └── lib.rs                  # 模块导出
├── examples/
│   ├── standalone_usage.rs     # 文件传输示例
│   └── chat_usage.rs           # 聊天功能示例
└── README.md                   # 模块文档
```

#### 关键特性

- **通用性** - 不依赖特定UI框架，可在多种环境使用
- **类型安全** - 完整的Rust类型系统支持
- **异步支持** - 基于tokio的异步实现
- **进度通知** - 支持实时传输进度回调
- **错误处理** - 标准化的错误类型和处理机制
- **实时聊天** - 基于iroh gossip协议的P2P聊天功能
- **文件分享** - 聊天中直接分享和下载文件

## 快速开始

### 1. 独立使用iroh模块

```bash
cd iroh-node
cargo run --example standalone_usage
```

### 2. 运行Tauri桌面应用

```bash
cd tauri-app
npm install
npm run tauri dev
```

### 3. 运行Axum Web服务

```bash
cd axum-app
cargo run
```

然后访问以下页面测试功能：
- `http://localhost:3000/static/iroh-test.html` - 文件传输功能测试
- `http://localhost:3000/static/chat-test.html` - 实时聊天功能测试

### 4. 运行聊天功能示例

```bash
cd iroh-node
cargo run --example chat_usage
```

## 使用示例

### 简单API使用

```rust
use iroh_node::simple_api;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 上传文件
    let file_path = Path::new("test.txt");
    let share_response = simple_api::upload_file(file_path, None).await?;
    println!("分享代码: {}", share_response.doc_ticket);

    // 下载文件
    let result = simple_api::download_file(
        &share_response.doc_ticket,
        Some(Path::new("/tmp/downloads")),
        None,
    ).await?;
    println!("下载完成: {}", result);

    Ok(())
}
```

### 带进度回调的使用

```rust
use iroh_node::{simple_api, TransferEvent};

let progress_callback = |event: TransferEvent| {
    match event {
        TransferEvent::UploadProgress { id, offset } => {
            println!("上传进度: {} - {}字节", id, offset);
        }
        TransferEvent::UploadDone { id } => {
            println!("上传完成: {}", id);
        }
        _ => {}
    }
};

let share_response = simple_api::upload_file_with_progress(
    file_path,
    None,
    progress_callback,
).await?;
```

### Tauri集成

```rust
use iroh_node::adapters::tauri_adapter::TauriAdapter;

// 在tauri应用中使用
let adapter = TauriAdapter::new(config, emitter).await?;
let response = adapter.get_share_code().await?;
```

### Axum集成

```rust
use iroh_node::adapters::axum_adapter::AxumAdapter;

// 在axum web服务中使用
let adapter = AxumAdapter::new(config).await?;
let result = adapter.download_files(request).await?;
```

### 聊天功能使用

```rust
use iroh_node::{IntegratedClientBuilder, ChatConfig, TransferConfig, CreateRoomRequest, SendMessageRequest, MessageType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建集成客户端（支持文件传输和聊天）
    let client = IntegratedClientBuilder::new()
        .transfer_config(TransferConfig::default())
        .chat_config(ChatConfig {
            user_name: "小明".to_string(),
            max_message_history: 100,
            enable_file_sharing: true,
        })
        .enable_chat(true)
        .build()
        .await?;

    // 创建聊天室
    let room = client.create_chat_room(CreateRoomRequest {
        name: "测试聊天室".to_string(),
        description: Some("这是一个测试聊天室".to_string()),
    }).await?;

    // 发送消息
    client.send_chat_message(SendMessageRequest {
        room_id: room.id.clone(),
        content: "大家好！".to_string(),
        message_type: MessageType::Text,
    }).await?;

    // 监听聊天事件
    let mut events = client.subscribe_chat_events()?;
    while let Ok(event) = events.recv().await {
        println!("收到聊天事件: {:?}", event);
    }

    Ok(())
}
```

## API文档

### Web API端点 (Axum)

#### 文件传输API
- `GET /api/iroh/share` - 获取分享代码
- `POST /api/iroh/upload` - 上传文件
- `POST /api/iroh/download` - 下载文件
- `POST /api/iroh/remove` - 删除文件
- `POST /api/iroh/session` - 创建进度会话
- `GET /api/iroh/progress/:session_id` - 进度事件流 (SSE)

#### 聊天API
- `POST /api/chat/rooms` - 创建聊天室
- `GET /api/chat/rooms` - 获取聊天室列表
- `POST /api/chat/rooms/join` - 加入聊天室
- `POST /api/chat/rooms/leave` - 离开聊天室
- `POST /api/chat/messages` - 发送消息
- `GET /api/chat/messages/:room_id` - 获取消息历史
- `POST /api/chat/session` - 创建聊天事件会话
- `GET /api/chat/events/:session_id` - 聊天事件流 (SSE)

### Tauri命令

- `get_share_code()` - 获取分享代码
- `get_blob(request)` - 下载文件
- `append_file(request)` - 上传文件
- `remove_file(request)` - 删除文件

## 技术栈

- **Rust** - 核心语言
- **iroh** - P2P传输和gossip协议库
- **iroh-gossip** - P2P实时通信协议
- **tokio** - 异步运行时
- **axum** - Web框架
- **tauri** - 桌面应用框架
- **serde** - 序列化/反序列化
- **tracing** - 日志系统
- **uuid** - 唯一标识符生成
- **chrono** - 时间处理

## 测试

```bash
# 运行iroh-node模块测试
cd iroh-node
cargo test

# 运行示例
cargo run --example standalone_usage

# 测试axum集成
cd ../axum-app
cargo run
# 访问 http://localhost:3000/static/iroh-test.html

# 测试tauri集成
cd ../tauri-app
npm run tauri dev
```

## 项目亮点

### 🔧 模块化设计
- 核心逻辑与UI框架完全解耦
- 清晰的模块边界和接口定义
- 支持多种运行环境的适配器模式

### 🚀 性能优化
- 基于tokio的高性能异步实现
- 流式传输支持大文件处理
- 内存高效的进度通知机制

### 🛡️ 类型安全
- 完整的Rust类型系统支持
- 编译时错误检查
- 标准化的错误处理机制

### 📊 实时反馈
- 完整的传输进度回调系统
- 支持SSE实时事件流
- 多种进度通知方式

### 🔌 易于集成
- 简单的API设计
- 详细的文档和示例
- 多种集成方式支持

## 贡献

欢迎提交Issue和Pull Request来改进这个项目。

## 许可证

本项目采用MIT许可证。