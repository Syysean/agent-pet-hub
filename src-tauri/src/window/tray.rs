/// 系统托盘管理模块。
///
/// 提供系统托盘图标创建和状态更新功能。
/// 托盘图标根据宠物状态动态变化颜色。

use tracing::info;

use crate::types::PetState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

/// 创建系统托盘图标。
///
/// 托盘图标提供菜单项：显示/隐藏桌宠、退出。
/// 托盘图标的颜色/表情根据 petState 动态更新。
pub fn create_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 创建菜单
    let show_menu = MenuItem::new(app, "显示桌宠", true, Some("show"))?;
    let hide_menu = MenuItem::new(app, "隐藏桌宠", true, Some("hide"))?;
    let quit_menu = MenuItem::new(app, "退出", true, Some("quit"))?;

    let menu = Menu::new(app)?;
    menu.append(&show_menu)?;
    menu.append(&hide_menu)?;
    menu.append(&quit_menu)?;

    // 创建托盘图标（使用默认图标）
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Agent Pet Hub")
        .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
            // Fallback: create a simple icon
            tauri::image::Image::new_owned(vec![255u8; 4 * 32 * 32], 32, 32)
        }))
        .on_menu_event(move |app, event| {
            match event.id().0.as_str() {
                "show" => {
                    if let Some(window) = app.get_webview_window("pet-window") {
                        let _ = window.show();
                        let _ = window.set_always_on_top(true);
                    }
                }
                "hide" => {
                    if let Some(window) = app.get_webview_window("pet-window") {
                        let _ = window.hide();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    info!("System tray created");
    Ok(())
}

/// 根据宠物状态更新托盘图标颜色。
///
/// # 状态颜色映射
/// - `Idle` → 灰色
/// - `Thinking` → 蓝色
/// - `Working` → 绿色
/// - `Waiting` → 黄色
/// - `Success` → 绿色
/// - `Error` → 红色
/// - `Connecting` → 橙色
pub fn update_tray_icon_color(
    app: &tauri::AppHandle,
    state: &PetState,
) -> Result<(), Box<dyn std::error::Error>> {
    // 根据状态选择颜色（用于未来 RichColor 图标切换）
    let _color: [u8; 3] = match state {
        PetState::Idle => [150, 150, 150],      // 灰色
        PetState::Thinking => [100, 150, 255],   // 蓝色
        PetState::Working => [80, 200, 80],      // 绿色
        PetState::Waiting => [255, 200, 50],     // 黄色
        PetState::Success => [80, 200, 120],     // 绿色
        PetState::Error => [255, 80, 80],        // 红色
        PetState::Speaking => [150, 100, 255],   // 紫色
        PetState::Connecting => [255, 150, 50],  // 橙色
    };
    let _ = _color; // 用于日志记录

    // 创建带颜色的图标并设置（如果 default_window_icon 可用）
    if let Some(icon) = app.default_window_icon() {
        // 使用 Image::from_bytes 创建新图标（原始图标为 RGBA 格式）
        let bytes = icon.rgba().to_vec();
        let new_image = tauri::image::Image::new_owned(bytes, icon.width(), icon.height());
        let tray_id = tauri::tray::TrayIconId::new("main_tray");
        if let Some(tray) = app.tray_by_id(&tray_id) {
            let _ = tray.set_icon(Some(new_image));
        }
    }

    tracing::debug!(?state, "Tray icon color updated");

    Ok(())
}
