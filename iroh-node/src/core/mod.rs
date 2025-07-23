//! iroh P2P文件传输和聊天核心模块
//! 
//! 提供通用的P2P文件传输和实时聊天功能，不依赖特定的UI框架

pub mod types;
pub mod client;
pub mod progress;
pub mod error;
pub mod chat;
pub mod chat_client;
pub mod integrated_client;

#[cfg(test)]
mod tests;

pub use client::IrohClient;
pub use chat_client::IrohChatClient;
pub use integrated_client::{IrohIntegratedClient, IntegratedClientBuilder};
pub use types::*;
pub use progress::*;
pub use error::*;
pub use chat::*;
