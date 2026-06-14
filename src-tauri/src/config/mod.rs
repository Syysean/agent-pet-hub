/// 配置管理模块。
///
/// 提供应用配置的读写、合并、持久化功能。
/// 配置存储在 `~/.config/agent-pet-hub/config.json`。
pub mod settings;

pub use settings::SettingsManager;
