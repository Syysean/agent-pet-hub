/// 设置模块 — 类型化配置，支持持久化。
///
/// 所有配置结构体都派生 `Serialize` / `Deserialize`，可以
/// 通过 JSON 文件进行序列化/反序列化。`SettingsManager` 负责
/// 从磁盘加载、写回，并通过 `serde_json::Value` 应用部分更新。
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────
// Top-level settings
// ─────────────────────────────────────────────

/// 顶级应用配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// 宠物子系统配置。
    pub pet: PetSettings,

    /// 智能体适配器配置。
    pub adapter: AdapterSettings,

    /// WebSocket 服务端配置。
    pub websocket: WebSocketSettings,

    /// TTS 引擎配置。
    pub tts: TTSSettings,

    /// 窗口外观与行为配置。
    pub window: WindowSettings,
}

// ─────────────────────────────────────────────
// Sub-settings
// ─────────────────────────────────────────────

/// 宠物子系统配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PetSettings {
    /// 皮肤 ID。
    #[serde(default = "default_skin_id")]
    pub skin_id: String,

    /// 宠物功能是否启用。
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 是否显示状态信息。
    #[serde(default = "default_show_status")]
    pub show_status: bool,
}

/// 适配器配置容器。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdapterSettings {
    /// Pi Agent 适配器配置。
    pub pi: PiAdapterSettings,

    /// Hermes Agent 适配器配置。
    pub hermes: HermesAdapterSettings,

    /// OpenClaw Agent 适配器配置。
    pub openclaw: OpenClawAdapterSettings,
}

/// Pi Agent 适配器配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PiAdapterSettings {
    /// 是否启用 Pi Agent。
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// JSONL 日志文件路径。
    #[serde(default = "default_pi_log_path")]
    pub log_path: String,
}

/// Hermes Agent 适配器配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HermesAdapterSettings {
    /// 是否启用 Hermes。
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// 网关 URL。
    #[serde(default = "default_hermes_gateway")]
    pub gateway_url: String,
}

/// OpenClaw Agent 适配器配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawAdapterSettings {
    /// 是否启用 OpenClaw。
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// OpenClaw 网关 / API 端点 URL。
    #[serde(default = "default_openclaw_url")]
    pub url: String,
}

/// WebSocket 服务端配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketSettings {
    /// 是否在启动时启动 WS 服务端。
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 监听端口。
    #[serde(default = "default_ws_port")]
    pub port: u16,

    /// 客户端认证的 Bearer token。
    #[serde(default = "default_ws_token")]
    pub auth_token: String,
}

/// TTS 引擎配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TTSSettings {
    /// 是否启用 TTS。
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 音量 [0.0, 1.0]。
    #[serde(default = "default_tts_volume")]
    pub volume: f64,

    /// 各事件类型的语音播报规则。
    pub rules: crate::types::TTSSpeechRules,

    /// TTS 语言代码（如 "zh-cn"、"en"）。
    #[serde(default = "default_tts_language")]
    pub language: String,
}

/// 窗口外观与行为配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WindowSettings {
    /// 窗口宽度（像素）。
    #[serde(default = "default_window_width")]
    pub width: u32,

    /// 窗口高度（像素）。
    #[serde(default = "default_window_height")]
    pub height: u32,

    /// 窗口是否始终置顶。
    #[serde(default = "default_true")]
    pub always_on_top: bool,
}

// ─────────────────────────────────────────────
// Default helpers
// ─────────────────────────────────────────────

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_skin_id() -> String {
    "default".to_owned()
}

fn default_show_status() -> bool {
    true
}

fn default_pi_log_path() -> String {
    "~/.pi/agent/logs/latest.jsonl".to_owned()
}

fn default_hermes_gateway() -> String {
    "ws://localhost:9100".to_owned()
}

fn default_openclaw_url() -> String {
    "http://localhost:3100".to_owned()
}

fn default_ws_port() -> u16 {
    8765
}

fn default_ws_token() -> String {
    // 首次启动时生成随机 token（使用 ULID），保存到配置文件后后续读取配置文件值
    ulid::Ulid::new().to_string()
}

fn default_tts_volume() -> f64 {
    1.0
}

fn default_tts_language() -> String {
    "zh-cn".to_owned()
}

fn default_window_width() -> u32 {
    320
}

fn default_window_height() -> u32 {
    280
}

/// 生成带合理默认值的 `AppSettings`。
pub fn default_settings() -> AppSettings {
    AppSettings {
        pet: PetSettings {
            skin_id: default_skin_id(),
            enabled: true,
            show_status: true,
        },
        adapter: AdapterSettings {
            pi: PiAdapterSettings {
                enabled: true,
                log_path: default_pi_log_path(),
            },
            hermes: HermesAdapterSettings {
                enabled: false,
                gateway_url: default_hermes_gateway(),
            },
            openclaw: OpenClawAdapterSettings {
                enabled: false,
                url: default_openclaw_url(),
            },
        },
        websocket: WebSocketSettings {
            enabled: true,
            port: default_ws_port(),
            auth_token: default_ws_token(),
        },
        tts: TTSSettings {
            enabled: true,
            volume: default_tts_volume(),
            rules: crate::types::TTSSpeechRules::default(),
            language: default_tts_language(),
        },
        window: WindowSettings {
            width: default_window_width(),
            height: default_window_height(),
            always_on_top: true,
        },
    }
}

/// 生成平台相关的配置文件路径。
///
/// | 平台     | 路径                                                |
/// |----------|-----------------------------------------------------|
/// | Linux    | `$XDG_CONFIG_HOME/agent-pet-hub/config.json` (或 `~/.config/…`) |
/// | macOS    | `~/Library/Application Support/agent-pet-hub/config.json` |
/// | Windows  | `%APPDATA%\agent-pet-hub\config.json`               |
pub fn default_config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("agent-pet-hub").join("config.json")
}

// ─────────────────────────────────────────────
// SettingsManager
// ─────────────────────────────────────────────

/// 管理应用配置的读取、写入和持久化。
///
/// 管理器在启动时从配置文件读取（若文件不存在则用默认值创建），
/// 并将当前配置保存在内存中以供快速访问。`save()` 将当前状态
/// 写回磁盘。
pub struct SettingsManager {
    /// 配置文件路径。
    config_path: PathBuf,

    /// 内存中的配置。
    settings: AppSettings,
}

impl SettingsManager {
    /// 使用默认配置路径创建新的管理器。
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_path: config_dir.join("config.json"),
            settings: default_settings(),
        }
    }

    /// 从配置文件加载配置。
    ///
    /// 如果文件不存在，则创建默认配置并写入磁盘。
    pub fn load(&mut self) -> Result<(), String> {
        if self.config_path.exists() {
            let raw = fs::read_to_string(&self.config_path)
                .map_err(|e| format!("读取配置文件失败: {e}"))?;
            self.settings =
                serde_json::from_str(&raw).map_err(|e| format!("解析配置文件失败: {e}"))?;
        } else {
            // 文件不存在时，写入默认配置。
            self.save()?;
        }
        Ok(())
    }

    /// 将当前配置持久化到磁盘。
    ///
    /// 使用原子写入模式：先写入临时文件，再 rename 到目标路径，
    /// 避免 TOCTOU 竞态（旧实现先 write 再 chmod，中间状态可被读取）。
    pub fn save(&self) -> Result<(), String> {
        let parent = self
            .config_path
            .parent()
            .ok_or_else(|| "config_path 没有父目录".to_string())?;
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;

        let json = serde_json::to_string_pretty(&self.settings)
            .map_err(|e| format!("序列化配置失败: {e}"))?;

        // 原子写入：先写入临时文件，再 rename 到目标路径
        let temp_path = self.config_path.with_extension("tmp");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // 设置临时文件权限为 0o600
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::write(&temp_path, &json).map_err(|e| format!("写入临时配置文件失败: {e}"))?;
            std::fs::set_permissions(&temp_path, perms)
                .map_err(|e| format!("设置临时配置文件权限失败: {e}"))?;
            // 原子 rename
            fs::rename(&temp_path, &self.config_path)
                .map_err(|e| format!("原子替换配置文件失败: {e}"))?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&temp_path, &json)
                .map_err(|e| format!("写入临时配置文件失败: {e}"))?;
            // Windows: 设置文件权限为读写（等效于 Unix 的 0o600 效果）
            std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("设置临时配置文件权限失败: {e}"))?;
            fs::rename(&temp_path, &self.config_path)
                .map_err(|e| format!("原子替换配置文件失败: {e}"))?;
        }
        Ok(())
    }

    /// 返回当前配置的引用。
    pub fn get(&self) -> &AppSettings {
        &self.settings
    }

    /// 应用部分更新（deep merge）。
    ///
    /// `updates` 值会递归合并到当前配置中：`updates` 中存在的字段替换
    /// 当前配置中的对应字段，不存在的字段保持不变。
    ///
    /// 最大递归深度：64 层，超出返回错误防止栈溢出。
    pub fn update(&mut self, updates: serde_json::Value) -> Result<(), String> {
        let current =
            serde_json::to_value(&self.settings).map_err(|e| format!("序列化当前配置失败: {e}"))?;

        let merged = merge_json_with_depth(current, updates, 0, 64)
            .map_err(|e| format!("合并配置失败: {e}"))?;

        self.settings =
            serde_json::from_value(merged).map_err(|e| format!("反序列化合并后的配置失败: {e}"))?;
        Ok(())
    }

    /// 返回配置文件路径。
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

/// 递归合并两个 JSON Value（右侧重），带深度限制。
///
/// 对于对象类型，递归合并每个键；对于其他类型，直接取右侧值。
/// 最大深度由 `max_depth` 参数控制，超过时返回错误。
fn merge_json_with_depth(
    base: serde_json::Value,
    overlay: serde_json::Value,
    current_depth: usize,
    max_depth: usize,
) -> Result<serde_json::Value, String> {
    // 检查递归深度（包括对象层和嵌套值层）
    if current_depth > max_depth {
        return Err(format!(
            "JSON merge depth exceeded limit of {} levels",
            max_depth
        ));
    }

    match (base, overlay) {
        (serde_json::Value::Object(mut a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                let existing = a.remove(&k);
                a.insert(
                    k,
                    merge_json_with_depth(
                        existing.unwrap_or(serde_json::Value::Null),
                        v,
                        current_depth + 1,
                        max_depth,
                    )?,
                );
            }
            Ok(serde_json::Value::Object(a))
        }
        (serde_json::Value::Null, ref v @ serde_json::Value::Object(_)) => {
            // 当 base 为 Null 但 overlay 是对象时，检查其深度
            // 防止 overlay 有深层嵌套时绕过深度限制
            let overlay_depth = count_object_depth(v);
            if overlay_depth > max_depth - current_depth {
                return Err(format!(
                    "JSON merge depth exceeded limit of {} levels",
                    max_depth
                ));
            }
            Ok(v.clone())
        }
        (_, v) => Ok(v),
    }
}

/// 计算 JSON 值的最大嵌套对象深度。
fn count_object_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return 1;
            }
            let max_child = map.values()
                .map(|v| count_object_depth(v))
                .max()
                .unwrap_or(0);
            1 + max_child
        }
        _ => 0,
    }
}

/// 递归合并两个 JSON Value（右侧重）。
///
/// 对于对象类型，递归合并每个键；对于其他类型，直接取右侧值。
/// 保留旧版 `merge_json` 用于不需要深度限制的调用（如有）。
#[allow(dead_code)]
fn merge_json(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    match (base, overlay) {
        (serde_json::Value::Object(mut a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                let existing = a.remove(&k);
                a.insert(
                    k,
                    merge_json(existing.unwrap_or(serde_json::Value::Null), v),
                );
            }
            serde_json::Value::Object(a)
        }
        (_, v) => v,
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        // Use a counter to ensure unique directories per test call
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let tmp = std::env::temp_dir();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        tmp.join(format!("agent-pet-hub-test-config-{}-{}", std::process::id(), id))
    }

    #[test]
    fn test_default_settings_struct() {
        let settings = default_settings();

        assert_eq!(settings.pet.skin_id, "default");
        assert!(settings.pet.enabled);
        assert!(settings.pet.show_status);
        assert!(settings.adapter.pi.enabled);
        assert_eq!(
            settings.adapter.pi.log_path,
            "~/.pi/agent/logs/latest.jsonl"
        );
        assert!(!settings.adapter.hermes.enabled);
        assert!(!settings.adapter.openclaw.enabled);
        assert!(settings.websocket.enabled);
        assert_eq!(settings.websocket.port, 8765);
        // auth_token 为首次启动时生成的随机 ULID
        assert!(!settings.websocket.auth_token.is_empty());
        assert_eq!(settings.websocket.auth_token.len(), 26); // ULID 固定 26 字符
        assert!(settings.tts.enabled);
        assert_eq!(settings.tts.volume, 1.0);
        assert_eq!(settings.window.width, 320);
        assert_eq!(settings.window.height, 280);
        assert!(settings.window.always_on_top);
    }

    #[test]
    fn test_settings_serialization_roundtrip() {
        let settings = default_settings();
        let json = serde_json::to_string_pretty(&settings).expect("serialize");
        let restored: AppSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(settings.pet.skin_id, restored.pet.skin_id);
        assert_eq!(settings.tts.volume, restored.tts.volume);
        assert_eq!(settings.websocket.port, restored.websocket.port);
        assert_eq!(settings.window.width, restored.window.width);
    }

    #[test]
    fn test_settings_camel_case_rename() {
        // Verify `rename_all = "camelCase"` works correctly.
        let json = r#"{
            "pet": {"skinId": "cat-v2", "enabled": true, "showStatus": false},
            "adapter": {
                "pi": {"enabled": true, "logPath": "/tmp/log.jsonl"},
                "hermes": {"enabled": false, "gatewayUrl": "ws://example.com"},
                "openclaw": {"enabled": false, "url": "http://example.com"}
            },
            "websocket": {"enabled": true, "port": 9999, "authToken": "secret"},
            "tts": {"enabled": true, "volume": 0.5, "rules": {}},
            "window": {"width": 400, "height": 350, "alwaysOnTop": false}
        }"#;
        let settings: AppSettings = serde_json::from_str(json).expect("parse camelCase JSON");
        assert_eq!(settings.pet.skin_id, "cat-v2");
        assert!(!settings.pet.show_status);
        assert_eq!(settings.tts.volume, 0.5);
        assert_eq!(settings.window.width, 400);
        assert_eq!(settings.window.height, 350);
        assert!(!settings.window.always_on_top);
    }

    #[test]
    fn test_settings_manager_save_and_load() {
        let dir = temp_config_dir();
        let mut manager = SettingsManager::new(dir.clone());

        // Mutate values before saving.
        manager.settings.tts.volume = 0.7;
        manager.settings.window.width = 500;
        manager.settings.pet.skin_id = "custom-skin".to_owned();

        manager.save().expect("save");
        let config_path = manager.config_path();
        assert!(config_path.exists(), "配置文件应已创建");

        // Create a fresh manager and load.
        let mut loaded = SettingsManager::new(dir.clone());
        loaded.load().expect("load");
        assert_eq!(loaded.settings.tts.volume, 0.7);
        assert_eq!(loaded.settings.window.width, 500);
        assert_eq!(loaded.settings.pet.skin_id, "custom-skin");

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_manager_load_creates_defaults() {
        let dir = temp_config_dir();
        let mut manager = SettingsManager::new(dir.clone());

        // Config file does not exist yet.
        assert!(!manager.config_path.exists());

        manager.load().expect("load");
        assert!(
            manager.config_path.exists(),
            "load 应在文件不存在时创建默认配置"
        );

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_partial_update_top_level() {
        let dir = temp_config_dir();
        let mut manager = SettingsManager::new(dir.clone());

        let updates: serde_json::Value = serde_json::json!({
            "websocket": {
                "port": 1234
            }
        });

        // 记录原始 token（首次启动时为随机 ULID）
        let original_token = manager.settings.websocket.auth_token.clone();
        let original_port = manager.settings.websocket.port;
        manager.update(updates).expect("update");
        assert_eq!(manager.settings.websocket.port, 1234);
        // Other fields should be preserved.
        assert_eq!(manager.settings.websocket.auth_token, original_token);
        assert_eq!(manager.settings.pet.skin_id, "default");

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_partial_update_nested_fields() {
        let dir = temp_config_dir();
        let mut manager = SettingsManager::new(dir.clone());

        let updates: serde_json::Value = serde_json::json!({
            "tts": {
                "volume": 0.3,
                "rules": {
                    "sessionStart": false
                }
            }
        });

        manager.update(updates).expect("update");
        assert_eq!(manager.settings.tts.volume, 0.3);
        assert_eq!(manager.settings.tts.rules.session_start, false);
        // Non-updated TTS fields should remain at defaults.
        assert!(manager.settings.tts.enabled);

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_partial_update_preserves_unmentioned_fields() {
        let mut manager = SettingsManager::new(temp_config_dir());

        let original_port = manager.settings.websocket.port;

        let updates: serde_json::Value = serde_json::json!({
            "pet": {
                "enabled": false
            }
        });

        manager.update(updates).expect("update");
        assert!(!manager.settings.pet.enabled);
        // WebSocket settings should be unchanged.
        assert_eq!(manager.settings.websocket.port, original_port);
    }

    #[test]
    fn test_default_config_path() {
        let path = default_config_path();
        assert!(path.to_string_lossy().contains("agent-pet-hub"));
        assert_eq!(
            path.file_name().map(|s| s.to_str().unwrap()),
            Some("config.json")
        );
    }

    #[test]
    fn test_update_replaces_object_fully() {
        let mut manager = SettingsManager::new(temp_config_dir());

        // Replace the entire websocket block with new values.
        let updates: serde_json::Value = serde_json::json!({
            "websocket": {
                "enabled": false,
                "port": 7777,
                "authToken": "new-token"
            }
        });

        manager.update(updates).expect("update");
        assert!(!manager.settings.websocket.enabled);
        assert_eq!(manager.settings.websocket.port, 7777);
        assert_eq!(manager.settings.websocket.auth_token, "new-token");
    }

    #[test]
    fn test_merge_json_with_depth_normal() {
        let base = serde_json::json!({"a": {"b": {"c": 1}}});
        let overlay = serde_json::json!({"a": {"b": {"d": 2}, "e": 3}});
        let result = merge_json_with_depth(base, overlay, 0, 64).unwrap();
        assert_eq!(result["a"]["b"]["c"], 1);
        assert_eq!(result["a"]["b"]["d"], 2);
        assert_eq!(result["a"]["e"], 3);
    }

    #[test]
    fn test_merge_json_with_depth_exceed() {
        // Build deeply nested JSON (100 levels) with non-null base at deepest level
        // to ensure the depth check is triggered (both sides must be objects)
        let base_deep = serde_json::json!({"a": 1});
        let mut deep = base_deep;
        for _ in 0..100 {
            deep = serde_json::json!({"nested": deep});
        }
        let base = serde_json::json!({});
        let result = merge_json_with_depth(base, deep, 0, 64);
        assert!(result.is_err(), "Depth > 64 should fail, got: {:?}", result);
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("depth exceeded") || err_msg.contains("64"));
    }
}
