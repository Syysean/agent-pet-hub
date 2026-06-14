/// Tauri Commands - 前端 API 层。
///
/// 提供前端可调用的命令(通过 `window.__TAURI__.invoke`),
/// 封装事件总线、状态机和配置管理的核心操作。

use std::sync::Arc;
use tokio::runtime::Handle;

use tauri::command;
use tauri::Manager;
use tracing::info;

use crate::event_bus::EventBus;
use crate::skin;
use crate::state_machine::PetStateMachine;
use crate::config::SettingsManager;
use crate::types::{PetState, UnifiedAgentEvent};

// 全局单例引用
// EVENT_BUS 和 SETTINGS 使用 std::sync::Mutex（同步访问即可）
// STATE_MACHINE 使用 tokio::sync::Mutex（async 命令需要 .lock().await）
use lazy_static::lazy_static;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex as TokioMutex;

lazy_static! {
    pub static ref EVENT_BUS: StdMutex<Option<EventBus>> = StdMutex::new(None);
    pub static ref STATE_MACHINE: Arc<TokioMutex<PetStateMachine>> = Arc::new(TokioMutex::new(PetStateMachine::new()));
    pub static ref SETTINGS: StdMutex<Option<SettingsManager>> = StdMutex::new(None);
}

/// 初始化全局单例。
///
/// 在应用启动时由 `lib.rs` 调用,
/// 将事件总线、状态机、配置管理器注册为全局单例。
///
/// # 注意
///
/// 状态机已经通过 `lazy_static!` 预创建为 `Arc<TokioMutex<PetStateMachine>>`，
/// 此函数直接赋值 Arc 指针，`init_globals` 和 `PiAdapter` 共享同一个实例。
/// 全局 Mutex 使用 `.into_inner()` 中毒恢复，避免 panic。
pub fn init_globals(
    event_bus: EventBus,
    settings: SettingsManager,
) {
    // EVENT_BUS — 中毒安全
    {
        let mut bus = EVENT_BUS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *bus = Some(event_bus);
    }

    // STATE_MACHINE — 直接赋值 Arc，无需 lock


    // SETTINGS — 中毒安全
    {
        let mut guard = SETTINGS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = Some(settings);
    }

    info!("Global services initialized");
}

/// 获取宠物当前状态。
///
/// 前端可通过此命令查询当前宠物状态(Idle/Thinking/Working 等)。
#[command]
pub async fn get_pet_state() -> Result<PetState, String> {
    let machine = STATE_MACHINE.lock().await;
    Ok(machine.current_state().clone())
}

/// 获取上一个状态。
///
/// 可用于状态还原或历史记录展示。
#[command]
pub async fn get_previous_state() -> Result<PetState, String> {
    let machine = STATE_MACHINE.lock().await;
    Ok(machine.previous_state().clone())
}

/// 获取状态快照。
///
/// 返回包含情绪、动画、位置等完整信息的 JSON 值,
/// 适用于前端渲染宠物动画和表情。
#[command]
pub async fn get_state_snapshot() -> Result<serde_json::Value, String> {
    let machine = STATE_MACHINE.lock().await;
    let snapshot = machine.get_snapshot();
    serde_json::to_value(snapshot).map_err(|e| e.to_string())
}

/// 获取当前配置。
///
/// 返回完整的 AppSettings 序列化结果。
#[command]
pub fn get_settings() -> Result<serde_json::Value, String> {
    let settings = SETTINGS
        .lock()
        .map_err(|e| e.to_string())?;
    let manager = settings.as_ref().ok_or("Settings not initialized")?;
    let current = manager.get();
    serde_json::to_value(current).map_err(|e| e.to_string())
}

/// 更新配置。
///
/// 接收一个 JSON 值作为部分更新,自动合并到当前配置并持久化到磁盘。
///
/// # 参数
///
/// * `updates` - 部分配置更新(如 `{ "tts": { "volume": 0.5 } }`)。
///
/// # 皮肤验证
///
/// 如果更新中包含 `pet.skinId`，会自动验证该皮肤 ID 是否存在于可用皮肤列表中。
/// 如果皮肤不存在，返回错误：`"皮肤 'xxx' 不存在"`。
#[command]
pub fn update_settings(updates: serde_json::Value) -> Result<(), String> {
    // 先验证 skin_id（如果存在）
    if let Some(pet_updates) = updates.get("pet") {
        if let Some(skin_id) = pet_updates.get("skinId").and_then(|v| v.as_str()) {
            // 获取用户皮肤目录
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("agent-pet-hub");
            let user_skins_dir = config_dir.join("skins");
            let available = skin::scan_all_skins(Some(user_skins_dir));

            if !skin::validate_skin_id(skin_id, &available) {
                return Err(format!("皮肤 '{}' 不存在", skin_id));
            }
        }
    }

    let mut settings = SETTINGS
        .lock()
        .map_err(|e| e.to_string())?;
    let manager = settings.as_mut().ok_or("Settings not initialized")?;
    manager.update(updates).map_err(|e| e.to_string())?;
    manager.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// 发送事件到事件总线。
///
/// 前端通过此命令向后端广播代理事件,
/// 触发状态机状态变更或状态通道通知。
///
/// # 参数
///
/// * `event` - 统一代理事件。
///
/// # 白名单校验
///
/// 仅允许 `source` 为 `pi` 的事件通过，防止前端伪造其他来源事件绕过适配器。
#[command]
pub async fn send_event(event: UnifiedAgentEvent) -> Result<usize, String> {
    // 仅允许 Pi 来源事件通过，防止前端伪造事件绕过适配器
    if event.source != crate::types::AgentSource::Pi {
        return Err(format!(
            "send_event only allows source 'pi', got '{}'",
            event.source
        ));
    }

    // 白名单校验：仅允许前端触发安全的控制型事件
    // 状态机转换事件通过 EventBus 由 Pi Adapter 发布，前端不直接发送
    let allowed_event_types = [
        crate::types::EventType::Heartbeat,
        crate::types::EventType::SessionEnd,
        crate::types::EventType::UserCancel,
        crate::types::EventType::PermissionGranted,
        crate::types::EventType::PermissionDenied,
    ];
    if !allowed_event_types.contains(&event.event_type) {
        return Err(format!(
            "send_event only allows control events, got {:?}",
            event.event_type
        ));
    }

    let bus = EVENT_BUS
        .lock()
        .map_err(|e| e.to_string())?;
    let event_bus = bus.as_ref().ok_or("Event bus not initialized")?;
    event_bus.publish_event(event).map_err(|e| e.to_string())
}

/// 手动触发状态变更。
///
/// 用于测试或外部控制(如从系统托盘切换状态),
/// 绕过事件总线直接设置状态机状态。
#[command]
pub async fn set_pet_state(state: PetState) -> Result<(), String> {
    let mut machine = STATE_MACHINE.lock().await;
    machine.set_state(state);
    Ok(())
}

/// 切换宠物窗口可见性。
///
/// 如果窗口可见则隐藏,否则显示并置顶。
#[command]
pub fn toggle_pet_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("pet-window") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_always_on_top(true);
        }
    }
    Ok(())
}

/// 发送心跳事件到事件总线。
///
/// 用于检测订阅者是否仍然活跃。
#[command]
pub async fn send_heartbeat() -> Result<(), String> {
    let bus = EVENT_BUS
        .lock()
        .map_err(|e| e.to_string())?;
    let event_bus = bus.as_ref().ok_or("Event bus not initialized")?;
    event_bus.publish_heartbeat().map_err(|e| e.to_string())?;
    Ok(())
}

/// 获取宠物当前状态（同步版本，供非 async 上下文使用）。
///
/// 当外部非 async 代码需要访问状态机时使用（如托盘回调）。
/// 注意：如果 async 锁正被持有，会短暂阻塞。
#[allow(dead_code)]
pub fn get_pet_state_sync() -> Result<PetState, String> {
    // tokio::sync::Mutex 不支持阻塞获取，通过当前 tokio handle 同步等待
    let rt = Handle::current();
    let machine = rt.block_on(async { STATE_MACHINE.lock().await });
    Ok(machine.current_state().clone())
}

/// 获取 Agent 在线状态。
///
/// 返回已配置的 Agent 列表及其在线状态。
/// 基于配置中的 enabled 字段判断 Agent 是否已启用。
#[command]
pub fn get_agent_info() -> Result<Vec<serde_json::Value>, String> {
    let settings = SETTINGS
        .lock()
        .map_err(|e| e.to_string())?;
    let manager = settings.as_ref().ok_or("Settings not initialized")?;
    let cfg = manager.get();

    let agents = vec![
        serde_json::json!({
            "source": "pi",
            "displayName": "Pi Agent",
            "online": cfg.adapter.pi.enabled,
            "version": "0.1.0"
        }),
        serde_json::json!({
            "source": "hermes",
            "displayName": "Hermes",
            "online": cfg.adapter.hermes.enabled,
            "version": null
        }),
        serde_json::json!({
            "source": "openclaw",
            "displayName": "OpenClaw",
            "online": cfg.adapter.openclaw.enabled,
            "version": null
        }),
    ];
    Ok(agents)
}

/// 启动原生窗口拖拽。
///
/// 前端通过此命令触发窗口拖拽操作，
/// 用于替代 data-tauri-drag-region（WSL2/WebKitGTK 环境下无效）
/// 和 Window.startDragging() JS API（某些平台不可靠）。
#[command]
pub fn start_drag(window: tauri::Window) -> Result<(), String> {
    window
        .start_dragging()
        .map_err(|e| e.to_string())
}

// ─── 皮肤管理命令 ─────────────────────────────────────────────────────────

/// 皮肤信息（前端调用 `list_skins` 时返回）。
#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct SkinListEntry {
    /// 皮肤唯一标识
    pub id: String,
    /// 皮肤显示名称
    pub name: String,
    /// 皮肤描述
    pub description: String,
    /// 皮肤资源路径
    pub image_path: String,
    /// 是否为用户自定义皮肤
    pub custom: bool,
}

/// 列出所有可用皮肤（内置 + 用户自定义）。
///
/// 前端可通过此命令获取皮肤列表，用于皮肤选择器 UI。
///
/// # 皮肤来源
///
/// 1. **内置皮肤** — `src/assets/skins/`（编译时嵌入）+ `src-tauri/resources/skins/`
/// 2. **用户自定义皮肤** — `~/.config/agent-pet-hub/skins/`
///
/// 返回的皮肤列表按来源排序：先内置，后用户自定义。
#[command]
pub fn list_skins() -> Result<Vec<SkinListEntry>, String> {
    // 获取用户皮肤目录
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("agent-pet-hub");
    let user_skins_dir = config_dir.join("skins");

    // 扫描所有皮肤
    let skins = skin::scan_all_skins(Some(user_skins_dir));

    // 转换为前端格式
    Ok(skins
        .into_iter()
        .map(|s| SkinListEntry {
            id: s.id,
            name: s.name,
            description: s.description,
            image_path: s.image_path,
            custom: s.custom,
        })
        .collect())
}

/// 获取指定皮肤的元数据（skin.json 内容）。
///
/// # 参数
///
/// * `skin_id` — 皮肤唯一标识
///
/// # 返回
///
/// 皮肤的 `skin.json` 解析后的 JSON 值。
///
/// # 查找优先级
///
/// 1. `src/assets/skins/{skin_id}/skin.json` — 内置皮肤（开发环境）
/// 2. `assets/skins/{skin_id}/skin.json` — 内置皮肤（发布环境）
/// 3. `~/.config/agent-pet-hub/skins/{skin_id}/skin.json` — 用户自定义
///
/// # 安全性
///
/// `skin_id` 经过 `canonicalize` + `strip_prefix` 验证，防止路径穿越。
#[command]
pub fn get_skin_metadata(skin_id: String) -> Result<serde_json::Value, String> {
    // 验证 skin_id 安全：不允许包含 ".." 或 "/" 防止路径穿越
    if skin_id.contains("..") || skin_id.contains('/') {
        return Err(format!("Invalid skin_id: '{}'", skin_id));
    }

    // 优先级 1: ../src/assets/skins/（开发环境）
    let src_path = std::path::Path::new("../src/assets/skins")
        .join(&skin_id)
        .join("skin.json");
    if let Ok(canonical) = src_path.canonicalize() {
        if canonical.exists() {
            if let Ok(base) = std::path::Path::new("../src/assets/skins").canonicalize() {
                if canonical.starts_with(&base) {
                    if let Ok(content) = std::fs::read_to_string(&canonical) {
                        if let Ok(metadata) = serde_json::from_str(&content) {
                            return Ok(metadata);
                        }
                    }
                }
            }
        }
    }

    // 优先级 2: ../assets/skins/（发布环境，bundle 输出目录）
    let builtin_path = std::path::Path::new("../assets/skins")
        .join(&skin_id)
        .join("skin.json");
    if let Ok(canonical) = builtin_path.canonicalize() {
        if canonical.exists() {
            if let Ok(base) = std::path::Path::new("../assets/skins").canonicalize() {
                if canonical.starts_with(&base) {
                    if let Ok(content) = std::fs::read_to_string(&canonical) {
                        if let Ok(metadata) = serde_json::from_str(&content) {
                            return Ok(metadata);
                        }
                    }
                }
            }
        }
    }

    // 优先级 3: 用户自定义皮肤目录
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("agent-pet-hub");
    let user_skin_dir = config_dir.join("skins").join(&skin_id);
    let user_skin_json = user_skin_dir.join("skin.json");
    if user_skin_json.exists() {
        if let Ok(canonical) = user_skin_json.canonicalize() {
            if let Ok(base) = config_dir.canonicalize() {
                if canonical.starts_with(base.join("skins")) {
                    let content = std::fs::read_to_string(&canonical)
                        .map_err(|e| format!("Failed to read skin.json: {e}"))?;
                    let metadata: serde_json::Value =
                        serde_json::from_str(&content).map_err(|e| format!("Invalid skin.json: {e}"))?;
                    return Ok(metadata);
                }
            }
        }
    }

    Err(format!("Skin '{}' not found", skin_id))
}
