//! iroh P2P文件传输和聊天核心模块
//!
//! 提供通用的P2P文件传输和实时聊天功能，不依赖特定的UI框架

pub mod chat;
pub mod chat_client;
pub mod client;
pub mod error;
pub mod integrated_client;
pub mod progress;
pub mod types;

#[cfg(test)]
mod tests;

pub use chat::*;
pub use chat_client::IrohChatClient;
pub use client::IrohClient;
pub use error::*;
pub use integrated_client::{IntegratedClientBuilder, IrohIntegratedClient};
pub use progress::*;
pub use types::*;
