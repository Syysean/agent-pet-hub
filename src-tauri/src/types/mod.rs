/// 类型定义模块。
///
/// 提供 agent-pet-hub 的核心类型系统，分为两个子模块：
///
/// - `events` — 事件类型：代理事件、事件分类、错误码、会话状态等
/// - `pet` — 宠物类型：情绪、动画、皮肤、显示设置等
///
/// 所有类型均派生 `Debug, Clone, Serialize, Deserialize, schemars::JsonSchema`，
/// 确保可在 Tauri 前端（TypeScript）和 Rust 后端之间安全传输。
///
/// # 使用示例
///
/// ```no_run
/// use agent_pet_hub_lib::types::{events, pet};
///
/// // 生成事件 ID 和时间戳
/// let id = events::generate_event_id();
/// let timestamp = events::generate_timestamp();
///
/// // 映射宠物状态到动画
/// let animation = pet::map_pet_state_to_animation(&pet::PetState::Thinking);
/// ```

pub mod events;
pub mod pet;

// ─── 重新导出常用类型 ─────────────────────────────────────────────────────

// 事件相关
pub use events::{
    AgentHealthStatus, AgentIdentity, AgentSource, ErrorCode, EventCategory,
    EventType, PetState, Session, SessionStatus, TTSSpeechRules, UnifiedAgentEvent,
    generate_event_id, generate_timestamp, truncate,
};

// 宠物相关
pub use pet::{
    EdgeMode, PetAction, PetAnimationState, PetConfig, PetDisplaySettings,
    PetInteractionEntry, PetMood, PetPosition, PetSkin, PetStateSnapshot,
    WindowSize, calculate_mood, map_pet_state_to_animation,
};

// ─── 类型别名 ───────────────────────────────────────────────────────────────

/// 宠物尺寸（快捷别名）。
pub type PetSize = WindowSize;
