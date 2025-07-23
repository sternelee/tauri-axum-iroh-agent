# iroh P2P文件传输通用模块

这是一个基于iroh库的通用P2P文件传输模块，支持跨平台使用，可以集成到tauri桌面应用、axum web服务等不同运行环境中。

## 特性

- 🚀 **通用模块化设计** - 不依赖特定UI框架，可在多种环境中使用
- 🔄 **P2P文件传输** - 基于iroh库的去中心化文件传输
- 📊 **进度回调支持** - 完整的传输进度通知机制
- 🛡️ **错误处理** - 标准化的错误处理和类型安全
- 🔌 **适配器模式** - 支持tauri、axum、独立运行等多种环境
- 🧪 **测试覆盖** - 包含单元测试和集成测试

## 快速开始

### 基本使用

```rust
use iroh_node::{simple_api, ConfigBuilder, TransferEvent};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 上传文件并获取分享代码
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
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let file_path = Path::new("test.txt");
    let share_response = simple_api::upload_file_with_progress(
        file_path,
        None,
        progress_callback,
    ).await?;

    println!("分享代码: {}", share_response.doc_ticket);
    Ok(())
}
```

## 架构设计

### 核心模块

- **`core::client`** - iroh客户端核心实现
- **`core::types`** - 类型定义和配置
- **`core::progress`** - 进度回调系统
- **`core::error`** - 错误处理

### 适配器层

- **`adapters::standalone`** - 独立运行适配器
- **`adapters::tauri_adapter`** - Tauri框架适配器
- **`adapters::axum_adapter`** - Axum Web框架适配器

## API文档

### 核心类型

#### `TransferConfig`

传输配置结构体：

```rust
pub struct TransferConfig {
    /// 数据存储根目录
    pub data_root: PathBuf,
    /// 下载目录
    pub download_dir: Option<PathBuf>,
    /// 是否启用详细日志
    pub verbose_logging: bool,
}
```

#### `TransferEvent`

进度事件枚举：

```rust
pub enum TransferEvent {
    DownloadQueueAppend { id: String, size: u64, name: String },
    DownloadProgress { id: String, offset: u64 },
    DownloadDone { id: String },
    UploadQueueAppend { id: String, size: u64, title: String },
    UploadProgress { id: String, offset: u64 },
    UploadDone { id: String },
    TransferError { id: String, error: String },
}
```

### 简单API

#### `simple_api::upload_file`

上传文件并返回分享代码：

```rust
pub async fn upload_file(
    file_path: &Path,
    data_root: Option<&Path>,
) -> TransferResult<ShareResponse>
```

#### `simple_api::download_file`

下载文件：

```rust
pub async fn download_file(
    doc_ticket: &str,
    download_dir: Option<&Path>,
    data_root: Option<&Path>,
) -> TransferResult<String>
```

#### `simple_api::upload_file_with_progress`

带进度回调的文件上传：

```rust
pub async fn upload_file_with_progress<F>(
    file_path: &Path,
    data_root: Option<&Path>,
    progress_callback: F,
) -> TransferResult<ShareResponse>
where
    F: Fn(TransferEvent) + Send + Sync + 'static
```

### 独立适配器

#### `StandaloneAdapter`

用于独立运行环境的适配器：

```rust
impl StandaloneAdapter {
    pub async fn new(config: TransferConfig) -> TransferResult<Self>
    pub async fn upload_file(&self, request: UploadRequest) -> TransferResult<()>
    pub async fn download_files(&self, request: DownloadRequest) -> TransferResult<String>
    pub async fn get_share_code(&self) -> TransferResult<ShareResponse>
    pub async fn remove_file(&self, request: RemoveRequest) -> TransferResult<()>
}
```

### 配置构建器

#### `ConfigBuilder`

用于构建传输配置：

```rust
impl ConfigBuilder {
    pub fn new() -> Self
    pub fn data_root<P: Into<PathBuf>>(self, path: P) -> Self
    pub fn download_dir<P: Into<PathBuf>>(self, path: Option<P>) -> Self
    pub fn verbose_logging(self, enabled: bool) -> Self
    pub fn build(self) -> TransferConfig
}
```

## 集成指南

### Tauri集成

1. 在`Cargo.toml`中添加依赖：

```toml
[dependencies]
iroh-node = { path = "../iroh-node", features = ["tauri-compat"] }
```

2. 实现事件发射器：

```rust
use iroh_node::adapters::tauri_adapter::{TauriAdapter, TauriEventEmitter};

struct AppEventEmitter {
    handle: tauri::AppHandle,
}

impl TauriEventEmitter for AppEventEmitter {
    fn emit_event(&self, event_name: &str, payload: serde_json::Value) {
        let _ = self.handle.emit_all(event_name, payload);
    }
}
```

3. 创建适配器并注册命令：

```rust
let config = ConfigBuilder::new()
    .data_root(app_data_dir.join("iroh_data"))
    .build();

let emitter = Arc::new(AppEventEmitter::new(handle));
let adapter = TauriAdapter::new(config, emitter).await?;

// 注册tauri命令
.invoke_handler(tauri::generate_handler![
    get_share_code,
    get_blob,
    append_file,
    remove_file
])
```

### Axum集成

1. 在`Cargo.toml`中添加依赖：

```toml
[dependencies]
iroh-node = { path = "../iroh-node" }
```

2. 创建适配器和路由：

```rust
use iroh_node::adapters::axum_adapter::AxumAdapter;

let config = ConfigBuilder::new()
    .data_root("/tmp/iroh_data")
    .build();

let adapter = AxumAdapter::new(config).await?;

// 创建路由
let app = Router::new()
    .route("/api/iroh/share", get(get_share_code))
    .route("/api/iroh/upload", post(upload_file))
    .route("/api/iroh/download", post(download_files))
    .with_state(adapter);
```

## 错误处理

模块使用`IrohTransferError`枚举来处理各种错误情况：

```rust
pub enum IrohTransferError {
    IrohClient(iroh::client::RpcError),
    DocError(iroh::docs::store::fs::Error),
    Io(std::io::Error),
    TicketParse(String),
    FileNotFound(String),
    DuplicateFileName(String),
    DownloadDirNotFound,
    Config(String),
    Network(String),
    Other(String),
}
```

## 测试

运行测试：

```bash
cd iroh-node
cargo test
```

运行示例：

```bash
cd iroh-node
cargo run --example standalone_usage
```

## 许可证

本项目采用MIT许可证。详见LICENSE文件。

## 贡献

欢迎提交Issue和Pull Request来改进这个项目。

## 更新日志

### v0.1.0

- 初始版本
- 支持P2P文件传输
- 提供tauri、axum、独立运行适配器
- 完整的错误处理和进度回调系统