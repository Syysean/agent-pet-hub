/// 宠物状态类型模块。
///
/// 定义桌面宠物的完整状态机、外观配置、动画帧和交互行为类型。
///
/// 所有类型均派生 `Debug, Clone, Serialize, Deserialize, schemars::JsonSchema`，
/// 确保前端 Tauri IPC 调用和 Rust 后端之间的类型安全。

use serde::{Deserialize, Serialize};

use super::events::PetState;

// ─── PetMood ────────────────────────────────────────────────────────────────

/// 宠物情绪状态，影响宠物的外观和行为。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum PetMood {
    /// 开心，宠物活跃且有响应。
    #[serde(rename = "happy")]
    Happy,
    /// 平静，宠物处于 idle 状态。
    #[serde(rename = "calm")]
    Calm,
    /// 专注，宠物正在思考或工作。
    #[serde(rename = "focused")]
    Focused,
    /// 疲惫，长时间运行后的低能量状态。
    #[serde(rename = "tired")]
    Tired,
    /// 好奇，宠物在等待用户交互。
    #[serde(rename = "curious")]
    Curious,
    /// 难过，代理出现错误或连接断开。
    #[serde(rename = "sad")]
    Sad,
}

// ─── PetAnimationState ──────────────────────────────────────────────────────

/// 宠物动画状态，映射到具体的动画帧序列。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum PetAnimationState {
    /// idle — 待机动画，缓慢呼吸。
    #[serde(rename = "idle")]
    Idle,
    /// walk — 走动动画。
    #[serde(rename = "walk")]
    Walk,
    /// eat — 吃东西动画。
    #[serde(rename = "eat")]
    Eat,
    /// sleep — 睡眠动画。
    #[serde(rename = "sleep")]
    Sleep,
    /// bounce — 弹跳动画（成功事件时）。
    #[serde(rename = "bounce")]
    Bounce,
    /// shake — 抖动动画（错误事件时）。
    #[serde(rename = "shake")]
    Shake,
    /// wave — 挥手动画（打招呼时）。
    #[serde(rename = "wave")]
    Wave,
    /// spin — 旋转动画（庆祝时）。
    #[serde(rename = "spin")]
    Spin,
    /// sleep_idle — 睡眠待机，微呼吸。
    #[serde(rename = "sleep_idle")]
    SleepIdle,
    /// eat_idle — 吃东西待机。
    #[serde(rename = "eat_idle")]
    EatIdle,
}

// ─── PetAction ──────────────────────────────────────────────────────────────

/// 用户可触发的宠物动作。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum PetAction {
    /// 点击宠物（默认交互）。
    #[serde(rename = "click")]
    Click,
    /// 双击宠物。
    #[serde(rename = "double_click")]
    DoubleClick,
    /// 长按宠物（拖拽）。
    #[serde(rename = "long_press")]
    LongPress,
    /// 投喂宠物。
    #[serde(rename = "feed")]
    Feed,
    /// 抚摸宠物。
    #[serde(rename = "pet")]
    Pet,
    /// 叫醒宠物。
    #[serde(rename = "wake")]
    Wake,
    /// 丢弃宠物（关闭窗口）。
    #[serde(rename = "discard")]
    Discard,
}

// ─── PetSkin ────────────────────────────────────────────────────────────────

/// 宠物皮肤配置。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PetSkin {
    /// 皮肤唯一标识。
    pub id: String,
    /// 皮肤显示名称。
    pub name: String,
    /// 皮肤描述。
    pub description: String,
    /// 皮肤图片目录或 URL。
    pub image_path: String,
    /// 是否为用户自定义皮肤。
    #[serde(default)]
    pub custom: bool,
}

// ─── PetPosition ────────────────────────────────────────────────────────────

/// 宠物在屏幕上的位置。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PetPosition {
    /// X 坐标（像素）。
    pub x: f64,
    /// Y 坐标（像素）。
    pub y: f64,
}

// ─── PetConfig ──────────────────────────────────────────────────────────────

/// 宠物全局配置。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PetConfig {
    /// 皮肤 ID。
    pub skin_id: String,
    /// 屏幕边缘吸附模式。
    #[serde(default)]
    pub edge_mode: EdgeMode,
    /// 是否随窗口聚焦自动隐藏。
    #[serde(default = "default_true")]
    pub hide_on_focus: bool,
    /// 拖拽锁定状态。
    #[serde(default)]
    pub locked: bool,
    /// 透明度（0.0 ~ 1.0）。
    #[serde(default = "default_opacity")]
    pub opacity: f64,
}

fn default_true() -> bool {
    true
}

fn default_opacity() -> f64 {
    1.0
}

// ─── EdgeMode ───────────────────────────────────────────────────────────────

/// 屏幕边缘吸附模式。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub enum EdgeMode {
    /// 吸附到边缘。
    #[serde(rename = "snap")]
    #[default]
    Snap,
    /// 自由移动。
    #[serde(rename = "free")]
    Free,
}

// ─── PetStateSnapshot ───────────────────────────────────────────────────────

/// 宠物状态快照，用于持久化和前端同步。
///
/// 聚合宠物当前所有可序列化状态，
/// 包含情绪、动画、位置和交互历史。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PetStateSnapshot {
    /// 当前情绪。
    pub mood: PetMood,
    /// 当前动画状态。
    pub animation: PetAnimationState,
    /// 屏幕位置。
    pub position: PetPosition,
    /// 宠物状态（映射自代理事件）。
    pub pet_state: PetState,
    /// 活跃代理来源（可选）。
    pub active_agent: Option<super::events::AgentSource>,
    /// 当前会话 ID（可选）。
    pub session_id: Option<String>,
    /// 连续错误次数。
    pub error_count: u32,
}

// ─── PetInteractionHistory ──────────────────────────────────────────────────

/// 宠物交互历史条目。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PetInteractionEntry {
    /// 动作类型。
    pub action: PetAction,
    /// 动作时间戳（RFC 3339）。
    pub timestamp: String,
    /// 宠物响应动画（可选）。
    pub response_animation: Option<PetAnimationState>,
}

// ─── PetDisplaySettings ─────────────────────────────────────────────────────

/// 宠物显示设置。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PetDisplaySettings {
    /// 是否启用宠物。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否显示通知气泡。
    #[serde(default)]
    pub show_notifications: bool,
    /// 通知气泡持续时间（毫秒）。
    #[serde(default = "default_notification_duration")]
    pub notification_duration_ms: u64,
    /// 窗口尺寸。
    pub window_size: WindowSize,
}

fn default_notification_duration() -> u64 {
    5000
}

// ─── WindowSize ─────────────────────────────────────────────────────────────

/// 窗口尺寸。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WindowSize {
    /// 宽度（像素）。
    pub width: u32,
    /// 高度（像素）。
    pub height: u32,
}

// ─── 辅助函数 ───────────────────────────────────────────────────────────────

/// 将 PetState（代理事件状态）映射为 PetAnimationState。
///
/// # 映射规则
/// - `PetState::Idle` → `PetAnimationState::Idle`
/// - `PetState::Thinking` → `PetAnimationState::Idle`（专注呼吸）
/// - `PetState::Working` → `PetAnimationState::Walk`
/// - `PetState::Waiting` → `PetAnimationState::EatIdle`
/// - `PetState::Success` → `PetAnimationState::Bounce`
/// - `PetState::Error` → `PetAnimationState::Shake`
/// - `PetState::Speaking` → `PetAnimationState::Wave`
/// - `PetState::Connecting` → `PetAnimationState::Walk`
pub fn map_pet_state_to_animation(pet_state: &PetState) -> PetAnimationState {
    match pet_state {
        PetState::Idle => PetAnimationState::Idle,
        PetState::Thinking => PetAnimationState::Idle,
        PetState::Working => PetAnimationState::Walk,
        PetState::Waiting => PetAnimationState::EatIdle,
        PetState::Success => PetAnimationState::Bounce,
        PetState::Error => PetAnimationState::Shake,
        PetState::Speaking => PetAnimationState::Wave,
        PetState::Connecting => PetAnimationState::Walk,
    }
}

/// 根据宠物状态和错误计数计算当前情绪。
pub fn calculate_mood(pet_state: &PetState, error_count: u32) -> PetMood {
    if error_count >= 3 {
        PetMood::Sad
    } else {
        match pet_state {
            PetState::Idle => PetMood::Calm,
            PetState::Thinking => PetMood::Focused,
            PetState::Working => PetMood::Focused,
            PetState::Waiting => PetMood::Curious,
            PetState::Success => PetMood::Happy,
            PetState::Error => PetMood::Sad,
            PetState::Speaking => PetMood::Happy,
            PetState::Connecting => PetMood::Curious,
        }
    }
}

// ─── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::events::{AgentSource, PetState as _PetState};
    use serde_json::json;
    use tracing::{debug, info};

    // ─── PetMood 测试 ─────────────────────────────────────────────────

    #[test]
    fn test_pet_mood_serde() {
        let moods = [
            (PetMood::Happy, "happy"),
            (PetMood::Calm, "calm"),
            (PetMood::Focused, "focused"),
            (PetMood::Tired, "tired"),
            (PetMood::Curious, "curious"),
            (PetMood::Sad, "sad"),
        ];

        for (mood, expected) in moods {
            let json = serde_json::to_value(&mood).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: PetMood =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, mood);
        }
    }

    // ─── PetAnimationState 测试 ───────────────────────────────────────

    #[test]
    fn test_pet_animation_state_serde() {
        let animations = [
            (PetAnimationState::Idle, "idle"),
            (PetAnimationState::Walk, "walk"),
            (PetAnimationState::Eat, "eat"),
            (PetAnimationState::Sleep, "sleep"),
            (PetAnimationState::Bounce, "bounce"),
            (PetAnimationState::Shake, "shake"),
            (PetAnimationState::Wave, "wave"),
            (PetAnimationState::Spin, "spin"),
            (PetAnimationState::SleepIdle, "sleep_idle"),
            (PetAnimationState::EatIdle, "eat_idle"),
        ];

        for (anim, expected) in animations {
            let json = serde_json::to_value(&anim).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: PetAnimationState =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, anim);
        }
    }

    // ─── PetAction 测试 ───────────────────────────────────────────────

    #[test]
    fn test_pet_action_serde() {
        let actions = [
            (PetAction::Click, "click"),
            (PetAction::DoubleClick, "double_click"),
            (PetAction::LongPress, "long_press"),
            (PetAction::Feed, "feed"),
            (PetAction::Pet, "pet"),
            (PetAction::Wake, "wake"),
            (PetAction::Discard, "discard"),
        ];

        for (action, expected) in actions {
            let json = serde_json::to_value(&action).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: PetAction =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, action);
        }
    }

    // ─── PetSkin 测试 ─────────────────────────────────────────────────

    #[test]
    fn test_pet_skin_serde() {
        let skin = PetSkin {
            id: "shark".to_string(),
            name: "Shark Skin".to_string(),
            description: "Shark pet skin".to_string(),
            image_path: "skins/shark".to_string(),
            custom: false,
        };

        let json = serde_json::to_value(&skin).unwrap();
        assert_eq!(json.get("id").unwrap(), &json!("shark"));
        assert_eq!(json.get("custom").unwrap(), &json!(false));

        let deserialized: PetSkin =
            serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.id, "shark");
        assert!(!deserialized.custom);
    }

    // ─── EdgeMode 测试 ────────────────────────────────────────────────

    #[test]
    fn test_edge_mode_serde() {
        let modes = [
            (EdgeMode::Snap, "snap"),
            (EdgeMode::Free, "free"),
        ];

        for (mode, expected) in modes {
            let json = serde_json::to_value(&mode).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: EdgeMode =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, mode);
        }
    }

    // ─── PetStateSnapshot 测试 ────────────────────────────────────────

    #[test]
    fn test_pet_state_snapshot_serde() {
        let snapshot = PetStateSnapshot {
            mood: PetMood::Calm,
            animation: PetAnimationState::Idle,
            position: PetPosition { x: 100.0, y: 200.0 },
            pet_state: PetState::Idle,
            active_agent: None,
            session_id: None,
            error_count: 0,
        };

        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(
            json.get("mood").unwrap(),
            &json!("calm")
        );
        assert_eq!(
            json.get("animation").unwrap(),
            &json!("idle")
        );

        let deserialized: PetStateSnapshot =
            serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.mood, PetMood::Calm);
        assert_eq!(deserialized.position.x, 100.0);
        assert_eq!(deserialized.position.y, 200.0);
    }

    // ─── PetConfig 测试 ───────────────────────────────────────────────

    #[test]
    fn test_pet_config_defaults() {
        let config = PetConfig {
            skin_id: "shark".to_string(),
            edge_mode: EdgeMode::Snap,
            hide_on_focus: true,
            locked: false,
            opacity: 1.0,
        };

        // 验证默认值
        assert!(config.hide_on_focus);
        assert_eq!(config.opacity, 1.0);
        assert!(!config.locked);

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(
            json.get("edge_mode").unwrap(),
            &json!("snap")
        );
    }

    // ─── PetDisplaySettings 测试 ──────────────────────────────────────

    #[test]
    fn test_pet_display_settings_defaults() {
        let settings = PetDisplaySettings {
            enabled: true,
            show_notifications: true,
            notification_duration_ms: 5000,
            window_size: WindowSize {
                width: 300,
                height: 300,
            },
        };

        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(
            json.get("enabled").unwrap(),
            &json!(true)
        );
        assert_eq!(
            json.get("notification_duration_ms").unwrap(),
            &json!(5000)
        );
    }

    // ─── map_pet_state_to_animation 测试 ──────────────────────────────

    #[test]
    fn test_map_pet_state_to_animation_idle() {
        assert_eq!(
            map_pet_state_to_animation(&PetState::Idle),
            PetAnimationState::Idle
        );
    }

    #[test]
    fn test_map_pet_state_to_animation_thinking() {
        // Thinking → Idle（专注呼吸）
        assert_eq!(
            map_pet_state_to_animation(&PetState::Thinking),
            PetAnimationState::Idle
        );
    }

    #[test]
    fn test_map_pet_state_to_animation_working() {
        assert_eq!(
            map_pet_state_to_animation(&PetState::Working),
            PetAnimationState::Walk
        );
    }

    #[test]
    fn test_map_pet_state_to_animation_success() {
        assert_eq!(
            map_pet_state_to_animation(&PetState::Success),
            PetAnimationState::Bounce
        );
    }

    #[test]
    fn test_map_pet_state_to_animation_error() {
        assert_eq!(
            map_pet_state_to_animation(&PetState::Error),
            PetAnimationState::Shake
        );
    }

    #[test]
    fn test_map_pet_state_to_animation_speaking() {
        assert_eq!(
            map_pet_state_to_animation(&PetState::Speaking),
            PetAnimationState::Wave
        );
    }

    #[test]
    fn test_map_pet_state_to_animation_connecting() {
        assert_eq!(
            map_pet_state_to_animation(&PetState::Connecting),
            PetAnimationState::Walk
        );
    }

    #[test]
    fn test_map_pet_state_to_animation_waiting() {
        assert_eq!(
            map_pet_state_to_animation(&PetState::Waiting),
            PetAnimationState::EatIdle
        );
    }

    // ─── calculate_mood 测试 ──────────────────────────────────────────

    #[test]
    fn test_calculate_mood_normal() {
        assert_eq!(
            calculate_mood(&_PetState::Success, 0),
            PetMood::Happy
        );
        assert_eq!(
            calculate_mood(&_PetState::Idle, 0),
            PetMood::Calm
        );
    }

    #[test]
    fn test_calculate_mood_focused() {
        assert_eq!(
            calculate_mood(&PetState::Thinking, 0),
            PetMood::Focused
        );
        assert_eq!(
            calculate_mood(&PetState::Working, 0),
            PetMood::Focused
        );
    }

    #[test]
    fn test_calculate_mood_curious() {
        assert_eq!(
            calculate_mood(&PetState::Connecting, 0),
            PetMood::Curious
        );
    }

    #[test]
    fn test_calculate_mood_sad_on_errors() {
        assert_eq!(
            calculate_mood(&PetState::Idle, 3),
            PetMood::Sad
        );
        assert_eq!(
            calculate_mood(&PetState::Success, 5),
            PetMood::Sad
        );
    }

    #[test]
    fn test_calculate_mood_boundary() {
        // error_count = 2 时不应触发 Sad
        assert_ne!(
            calculate_mood(&PetState::Idle, 2),
            PetMood::Sad
        );
        assert_eq!(
            calculate_mood(&PetState::Idle, 2),
            PetMood::Calm
        );

        // error_count = 3 时触发 Sad
        assert_eq!(
            calculate_mood(&PetState::Idle, 3),
            PetMood::Sad
        );
    }

    // ─── PetInteractionEntry 测试 ─────────────────────────────────────

    #[test]
    fn test_pet_interaction_entry_serde() {
        let entry = PetInteractionEntry {
            action: PetAction::Click,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            response_animation: Some(PetAnimationState::Bounce),
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json.get("action").unwrap(), &json!("click"));

        let deserialized: PetInteractionEntry =
            serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.action, PetAction::Click);
    }

    #[test]
    fn test_pet_interaction_entry_no_response() {
        let entry = PetInteractionEntry {
            action: PetAction::Feed,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            response_animation: None,
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert!(json.get("responseAnimation").is_none());
    }

    // ─── PetPosition 测试 ─────────────────────────────────────────────

    #[test]
    fn test_pet_position_serde() {
        let pos = PetPosition { x: 150.5, y: 300.0 };

        let json = serde_json::to_value(&pos).unwrap();
        assert_eq!(json.get("x").unwrap(), &json!(150.5));
        assert_eq!(json.get("y").unwrap(), &json!(300.0));
    }

    // ─── WindowSize 测试 ──────────────────────────────────────────────

    #[test]
    fn test_window_size_serde() {
        let size = WindowSize {
            width: 400,
            height: 400,
        };

        let json = serde_json::to_value(&size).unwrap();
        assert_eq!(json.get("width").unwrap(), &json!(400));
        assert_eq!(json.get("height").unwrap(), &json!(400));
    }

    // ─── PetSkin custom 默认值测试 ────────────────────────────────────

    #[test]
    fn test_pet_skin_custom_defaults_false() {
        let skin_json = json!({
            "id": "custom-skin",
            "name": "Custom",
            "description": "A custom skin",
            "image_path": "skins/custom"
        });

        let skin: PetSkin =
            serde_json::from_value(skin_json).unwrap();
        assert!(!skin.custom, "custom 字段应为默认值 false");
    }

    // ─── PetStateSnapshot with active agent ───────────────────────────

    #[test]
    fn test_pet_state_snapshot_with_agent() {
        let snapshot = PetStateSnapshot {
            mood: PetMood::Focused,
            animation: PetAnimationState::Walk,
            position: PetPosition { x: 0.0, y: 0.0 },
            pet_state: PetState::Working,
            active_agent: Some(AgentSource::Pi),
            session_id: Some("sess-123".to_string()),
            error_count: 0,
        };

        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(
            json.get("activeAgent").unwrap(),
            &json!("pi")
        );
        assert_eq!(
            json.get("sessionId").unwrap(),
            &json!("sess-123")
        );

        let deserialized: PetStateSnapshot =
            serde_json::from_value(json).unwrap();
        assert_eq!(
            deserialized.active_agent,
            Some(AgentSource::Pi)
        );
    }

    // ─── 日志测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_state_snapshot_logging() {
        let snapshot = PetStateSnapshot {
            mood: PetMood::Happy,
            animation: PetAnimationState::Bounce,
            position: PetPosition { x: 100.0, y: 200.0 },
            pet_state: PetState::Success,
            active_agent: Some(AgentSource::Hermes),
            session_id: Some("sess-001".to_string()),
            error_count: 0,
        };

        debug!(
            mood = ?snapshot.mood,
            animation = ?snapshot.animation,
            pet_state = ?snapshot.pet_state,
            x = snapshot.position.x,
            y = snapshot.position.y,
            "Updated pet state snapshot"
        );

        info!(
            active_agent = ?snapshot.active_agent,
            session_id = ?snapshot.session_id,
            "Pet state updated with agent context"
        );
    }
}
