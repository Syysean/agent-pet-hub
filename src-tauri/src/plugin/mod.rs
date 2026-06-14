/// 插件系统模块。
///
/// 提供桌面宠物的插件扩展能力，支持皮肤、动画等插件。
///
/// # 架构概述
///
/// - `Plugin` trait — 所有插件必须实现此接口
/// - `PluginManager` — 管理插件的生命周期（加载、卸载、重载）
/// - `PluginInfo` — 插件元数据，用于前端展示
///
/// # 插件类型
///
/// - `skin` — 皮肤插件，替换宠物外观
/// - `animation` — 动画插件，扩展动画行为
/// - `voice` — 语音插件，自定义语音效果
/// - `notification` — 通知插件，自定义通知样式
///
/// # 默认插件目录
///
/// `~/.local/share/agent-pet-hub/plugins/`

pub mod manager;

pub use manager::{
    DefaultSkinPlugin, Plugin, PluginError, PluginInfo, PluginManager,
};
