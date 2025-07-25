//! 适配器层，支持不同运行环境的集成
pub mod standalone;
pub mod tauri_adapter;

pub use standalone::StandaloneAdapter;
pub use tauri_adapter::TauriAdapter;
