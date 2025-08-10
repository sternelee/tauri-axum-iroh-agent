//! 适配器模块
//!
//! 提供不同环境的适配器，如Tauri和Axum

pub mod axum;
pub mod tauri;
pub mod tauri_adapter;

pub use self::{
    axum::AxumAdapter, 
    tauri::TauriAdapter as TauriAdapterV1,
    tauri_adapter::TauriPlugin as TauriAdapterV2
};

// 根据Tauri版本导出适当的适配器
#[cfg(feature = "tauri-compat")]
pub use self::tauri_adapter::TauriPlugin;

#[cfg(feature = "tauri-plugin")]
pub use self::tauri::TauriAdapter as TauriPlugin;
