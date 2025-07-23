//! 适配器层，支持不同运行环境的集成

pub mod tauri_adapter;
pub mod axum_adapter;
pub mod standalone;

pub use tauri_adapter::TauriAdapter;
pub use axum_adapter::AxumAdapter;
pub use standalone::StandaloneAdapter;