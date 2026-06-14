mod adapter;
mod commands;
pub mod config;
pub mod event_bus;
pub mod ipc;
pub mod plugin;
pub mod skin;
pub mod state_machine;
pub mod tts;
pub mod types;
pub mod window;

// 重新导出公共 API
pub use commands::{
    get_pet_state, get_previous_state, get_state_snapshot, get_settings, update_settings,
    send_event, set_pet_state, toggle_pet_window, send_heartbeat, get_agent_info, init_globals,
    list_skins, get_skin_metadata,
};
pub use event_bus::EventBus;
pub use state_machine::PetStateMachine;
pub use config::SettingsManager;
pub use window::{create_pet_window, create_tray, update_tray_icon_color};
pub use ipc::WSServer;
pub use plugin::PluginManager;

use tracing::info;
use tauri::Emitter;
use adapter::AgentAdapter;

/// 展开路径中的 `~` 前缀。
/// 用于确保从配置读取的路径（如 `~/.pi/agent/logs/latest.jsonl`）
/// 在传递给 PiAdapter 前已展开为绝对路径。
fn expand_home_path(path_str: &str) -> std::path::PathBuf {
    if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    std::path::PathBuf::from(path_str)
}

/// Tauri 2.x 应用入口。
///
/// 在应用启动时初始化所有核心组件：
/// 1. 创建配置管理器并加载配置
/// 2. 创建事件总线
/// 3. 初始化全局单例（STATE_MACHINE 已预创建为 Arc<TokioMutex>）
/// 4. 创建宠物悬浮窗
/// 5. 创建系统托盘
/// 6. 初始化 TTS 引擎（如果启用）
/// 7. 启动 Pi 适配器（后台任务）
/// 8. 启动 WebSocket 服务器（如果启用）
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_pet_state,
            commands::get_previous_state,
            commands::get_state_snapshot,
            commands::get_settings,
            commands::update_settings,
            commands::send_event,
            commands::set_pet_state,
            commands::toggle_pet_window,
            commands::send_heartbeat,
            commands::get_agent_info,
            commands::start_drag,
            commands::list_skins,
            commands::get_skin_metadata,
        ])
        .setup(|app| {
            // 1. 初始化配置管理器
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("agent-pet-hub");
            std::fs::create_dir_all(&config_dir).ok();

            let mut settings_manager = config::SettingsManager::new(config_dir);
            if let Err(e) = settings_manager.load() {
                tracing::warn!("Failed to load settings, using defaults: {}", e);
            }

            let pi_config = settings_manager.get().adapter.pi.clone();
            let tts_enabled = settings_manager.get().tts.enabled;
            let tts_volume = settings_manager.get().tts.volume;
            let tts_language = settings_manager.get().tts.language.clone();
            let tts_rules = settings_manager.get().tts.rules.clone();
            let ws_config = settings_manager.get().websocket.clone();

            // 2. 创建事件总线（4096 = DEFAULT_CHANNEL_SIZE）
            let event_bus = EventBus::new(4096);

            // 3. 初始化全局单例
            //    STATE_MACHINE 已在 commands.rs 的 lazy_static! 中预创建为 Arc<TokioMutex<PetStateMachine>>
            //    init_globals 只注册 EVENT_BUS 和 SETTINGS，并复用同一个 STATE_MACHINE Arc
            init_globals(
                event_bus,
                settings_manager,
            );

            // 4. 创建宠物悬浮窗（单次调用）
            let window_result = window::create_pet_window(app.handle());
            if let Err(e) = &window_result {
                tracing::error!("Failed to create pet window: {}", e);
            }

            // 5. 创建系统托盘（单次调用）
            let tray_result = window::create_tray(app.handle());
            if let Err(e) = &tray_result {
                tracing::error!("Failed to create system tray: {}", e);
            }

            // 6. 初始化 TTS 引擎（如果启用）
            //    TTS engine 共享同一个 Arc<TokioMutex<PetStateMachine>> 状态机
            let tts_engine: Option<std::sync::Arc<tokio::sync::Mutex<tts::TTSEngine>>> =
                if tts_enabled {
                    Some(std::sync::Arc::new(tokio::sync::Mutex::new(
                        tts::TTSEngine::new(
                            true,
                            "espeak".to_string(), // Linux 默认使用 espeak
                            tts_volume as f32,
                            tts_language,
                            tts_rules,
                        )
                    )))
                } else {
                    None
                };

            // 7. 启动 Pi 适配器（后台任务）
            //    复用 commands::STATE_MACHINE，与全局命令共享同一个状态机实例
            let pi_config = pi_config;
            if pi_config.enabled {
                // 展开路径中的 ~ 前缀（如 "~/.pi/agent/logs/latest.jsonl"）
                let log_path = expand_home_path(&pi_config.log_path);
                let adapter_config = adapter::PiAdapterConfig {
                    log_path,
                    home: dirs::home_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join(".pi"),
                    enable_tts: tts_enabled,
                };

                let event_bus_clone = crate::commands::EVENT_BUS
                    .lock()
                    .ok()
                    .and_then(|g| g.clone());

                if let Some(event_bus_for_adapter) = event_bus_clone {
                    let adapter = adapter::PiAdapter::new(
                        adapter_config,
                        event_bus_for_adapter,
                        crate::commands::STATE_MACHINE.clone(), // ← 复用同一个 Arc<TokioMutex>
                        tts_engine,
                    );

                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = AgentAdapter::connect(&adapter).await {
                            tracing::warn!("Pi adapter connect error: {}", e);
                        }
                        if let Err(e) = AgentAdapter::start_listening(&adapter).await {
                            tracing::warn!("Pi adapter start_listening error: {}", e);
                        }
                    });
                } else {
                    tracing::error!("EventBus not available for Pi adapter");
                }
            }

            // 8. 启动 WebSocket 服务器（后台任务）
            if ws_config.enabled {
                // 从全局 EVENT_BUS 获取 broadcast::Sender，传给 WSServer
                let event_tx = crate::commands::EVENT_BUS
                    .lock()
                    .ok()
                    .and_then(|g| g.as_ref().map(|eb| eb.event_tx().clone()));

                if let Some(tx) = event_tx {
                    let ws_server = WSServer::new(tx, ws_config.port, ws_config.auth_token, 10);
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = ws_server.run().await {
                            tracing::error!("WebSocket server error: {}", e);
                        }
                    });
                }
            }

            // 订阅 EventBus，更新状态机并通知前端
            let app_handle = app.handle().clone();
            let event_bus_for_listener = crate::commands::EVENT_BUS
                .lock()
                .ok()
                .and_then(|g| g.clone());

            if let Some(eb) = event_bus_for_listener {
                let mut rx = eb.subscribe_event();
                tauri::async_runtime::spawn(async move {
                    while let Ok(event) = rx.recv().await {
                        // 通知前端
                        info!(event_type = ?event.event_type, "Emitted pet:event to frontend");
                        let _ = app_handle.emit("pet:event", &event);

                        // 读取当前状态并通知前端
                        let current_state = {
                            let sm = crate::commands::STATE_MACHINE.lock().await;
                            sm.current_state().clone()
                        };
                        info!(current_state = ?current_state, "Emitted pet:state_changed to frontend");
                        let _ = app_handle.emit("pet:state_changed", &current_state);
                    }
                });
            }

            info!("Agent Pet Hub initialized successfully");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
