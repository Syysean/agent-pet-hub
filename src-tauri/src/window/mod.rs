/// 窗口管理模块。
///
/// 负责创建和管理桌宠悬浮窗与系统托盘。
///
/// # 模块结构
///
/// - `pet_window` — 宠物悬浮窗创建
/// - `tray` — 系统托盘图标及菜单管理

pub mod pet_window;
pub mod tray;

pub use pet_window::create_pet_window;
pub use tray::{create_tray, update_tray_icon_color};
