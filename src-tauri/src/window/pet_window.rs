/// 宠物悬浮窗创建模块。
///
/// 负责创建桌宠的主悬浮窗口，配置无边框、透明、置顶等属性。
/// 窗口始终保持在最顶层，并隐藏在任务栏中。

use tauri::Manager;
use tracing::info;

/// 创建宠物悬浮窗。
///
/// 窗口属性：
/// - 无装饰（无边框）
/// - 透明背景
/// - 始终置顶
/// - 隐藏于任务栏
/// - 固定尺寸 320×280 像素
///
/// # 参数
///
/// * `app` — Tauri 应用句柄，用于创建窗口。
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回 `Err`。
pub fn create_pet_window(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::WebviewWindowBuilder;

    // 窗口已存在则直接复用（避免热重载时 "already exists" 错误）
    if app.get_webview_window("pet-window").is_some() {
        info!("Pet window already exists, reusing");
        return Ok(());
    }

    let _window = WebviewWindowBuilder::new(
        app,
        "pet-window",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Agent Pet Hub")
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .inner_size(320.0, 280.0)
    .build()?;

    info!("Pet window created (320×280, transparent, always-on-top)");
    Ok(())
}
