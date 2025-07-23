//! iroh P2P文件传输核心模块
//! 
//! 提供通用的P2P文件传输功能，不依赖特定的UI框架

pub mod types;
pub mod client;
pub mod progress;
pub mod error;

#[cfg(test)]
mod tests;

pub use client::IrohClient;
pub use types::*;
pub use progress::*;
pub use error::*;
