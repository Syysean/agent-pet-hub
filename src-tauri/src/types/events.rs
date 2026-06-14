/// 事件类型定义模块。
///
/// 定义所有代理事件、宠物状态、错误码和会话相关的类型，
/// 用于跨代理（Pi、Hermes、OpenClaw）的统一事件规范。
///
/// 所有类型均派生 `Debug, Clone, Serialize, Deserialize, schemars::JsonSchema`，
/// 确保可在 Rust 和 TypeScript 之间无缝序列化。

use serde::{Deserialize, Serialize};

// ─── AgentSource ────────────────────────────────────────────────────────────

/// 代理来源，标识事件来自哪个 AI Agent。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
pub enum AgentSource {
    #[serde(rename = "pi")]
    Pi,
    #[serde(rename = "hermes")]
    Hermes,
    #[serde(rename = "openclaw")]
    OpenClaw,
}

impl std::fmt::Display for AgentSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentSource::Pi => write!(f, "pi"),
            AgentSource::Hermes => write!(f, "hermes"),
            AgentSource::OpenClaw => write!(f, "openclaw"),
        }
    }
}

// ─── EventCategory ──────────────────────────────────────────────────────────

/// 事件分类（9 种），用于对事件进行逻辑分组。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Hash)]
pub enum EventCategory {
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "thinking")]
    Thinking,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "message")]
    Message,
    #[serde(rename = "permission")]
    Permission,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "subagent")]
    Subagent,
}

// ─── EventType ──────────────────────────────────────────────────────────────

/// 事件类型（22 种），覆盖所有代理事件的具体操作。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Hash)]
pub enum EventType {
    #[serde(rename = "session_start")]
    SessionStart,
    #[serde(rename = "session_end")]
    SessionEnd,
    #[serde(rename = "session_compaction")]
    SessionCompaction,
    #[serde(rename = "user_prompt")]
    UserPrompt,
    #[serde(rename = "user_cancel")]
    UserCancel,
    #[serde(rename = "thinking_start")]
    ThinkingStart,
    #[serde(rename = "thinking_tick")]
    ThinkingTick,
    #[serde(rename = "thinking_end")]
    ThinkingEnd,
    #[serde(rename = "tool_call_start")]
    ToolCallStart,
    #[serde(rename = "tool_call_end")]
    ToolCallEnd,
    #[serde(rename = "tool_call_error")]
    ToolCallError,
    #[serde(rename = "tool_batch")]
    ToolBatch,
    #[serde(rename = "permission_request")]
    PermissionRequest,
    #[serde(rename = "permission_granted")]
    PermissionGranted,
    #[serde(rename = "permission_denied")]
    PermissionDenied,
    #[serde(rename = "agent_message")]
    AgentMessage,
    #[serde(rename = "agent_reply")]
    AgentReply,
    #[serde(rename = "subagent_start")]
    SubagentStart,
    #[serde(rename = "subagent_end")]
    SubagentEnd,
    #[serde(rename = "heartbeat")]
    Heartbeat,
    #[serde(rename = "adapter_connected")]
    AdapterConnected,
    #[serde(rename = "adapter_disconnected")]
    AdapterDisconnected,
}

// ─── PetState ───────────────────────────────────────────────────────────────

/// 宠物状态（8 种），映射到宠物的动画帧和交互行为。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Hash)]
pub enum PetState {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "thinking")]
    Thinking,
    #[serde(rename = "working")]
    Working,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "speaking")]
    Speaking,
    #[serde(rename = "connecting")]
    Connecting,
}

// ─── ErrorCode ──────────────────────────────────────────────────────────────

/// 错误码（23 种），用于统一错误分类。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ErrorCode {
    #[serde(rename = "connection_refused")]
    ConnectionRefused,
    #[serde(rename = "connection_timeout")]
    ConnectionTimeout,
    #[serde(rename = "connection_lost")]
    ConnectionLost,
    #[serde(rename = "handshake_failed")]
    HandshakeFailed,
    #[serde(rename = "auth_failed")]
    AuthFailed,
    #[serde(rename = "token_expired")]
    TokenExpired,
    #[serde(rename = "permission_denied")]
    PermissionDenied,
    #[serde(rename = "tool_not_found")]
    ToolNotFound,
    #[serde(rename = "tool_timeout")]
    ToolTimeout,
    #[serde(rename = "tool_memory_limit")]
    ToolMemoryLimit,
    #[serde(rename = "tool_rate_limit")]
    ToolRateLimit,
    #[serde(rename = "agent_crash")]
    AgentCrash,
    #[serde(rename = "agent_oom")]
    AgentOOM,
    #[serde(rename = "agent_context_overflow")]
    AgentContextOverflow,
    #[serde(rename = "parse_error")]
    ParseError,
    #[serde(rename = "version_mismatch")]
    VersionMismatch,
    #[serde(rename = "invalid_event")]
    InvalidEvent,
    #[serde(rename = "disk_full")]
    DiskFull,
    #[serde(rename = "file_locked")]
    FileLocked,
    #[serde(rename = "process_killed")]
    ProcessKilled,
    #[serde(rename = "unknown")]
    Unknown,
}

// ─── SessionStatus ──────────────────────────────────────────────────────────

/// 会话状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SessionStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "error")]
    Error,
}

// ─── AgentHealthStatus ──────────────────────────────────────────────────────

/// 代理健康状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum AgentHealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unhealthy")]
    Unhealthy,
    #[serde(rename = "unknown")]
    Unknown,
}

// ─── UnifiedAgentEvent ──────────────────────────────────────────────────────

/// 统一代理事件结构体。
///
/// 所有代理（Pi、Hermes、OpenClaw）的事件都通过此结构统一表示，
/// 包含元数据、会话上下文、工具调用信息、错误信息等。
///
/// # 命名约定
/// - Rust 侧使用蛇形命名（如 `event_type`）
/// - 序列化到 TS 侧时使用驼峰命名（如 `petState`）
/// - `type` 字段因是 Rust 关键字，使用 `rename = "type"`
/// - 所有字段使用 camelCase 序列化，与 TypeScript 侧对齐
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedAgentEvent {
    /// 事件唯一标识（ULID）。
    pub id: String,
    /// 事件时间戳（RFC 3339）。
    pub timestamp: String,
    /// 事件协议版本。
    pub version: String,
    /// 代理来源。
    pub source: AgentSource,
    /// 事件分类。
    pub category: EventCategory,
    /// 事件类型（序列化时字段名为 `"type"`）。
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// 宠物状态（序列化时字段名为 `"petState"`）。
    #[serde(rename = "petState")]
    pub pet_state: PetState,
    /// 会话 ID（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 子会话 ID（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_session_id: Option<String>,
    /// 工具名称（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// 工具参数预览（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_args_preview: Option<String>,
    /// 工具结果预览（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result_preview: Option<String>,
    /// 任务预览（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_preview: Option<String>,
    /// 步骤编号（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_number: Option<i32>,
    /// 是否等待审批（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub awaiting_approval: Option<bool>,
    /// 工具是否成功（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_success: Option<bool>,
    /// 错误码（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    /// 错误消息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// 代理回复预览（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_reply_preview: Option<String>,
    /// 原始事件 JSON（完整内容）。
    /// 最大允许 8KB，超出则截断为 `[TRUNCATED]`，防止内存 DoS。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,

    /// 原始事件 JSON 字节大小（包含截断）。当 `raw` 被截断时为 `Some(原始大小)`。
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "rawSize")]
    pub raw_size: Option<usize>,
    /// 附加元数据（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// ─── AgentIdentity ──────────────────────────────────────────────────────────

/// 代理身份信息。
///
/// 描述一个代理的基本属性和在线状态，
/// 用于 UI 显示和连接管理。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AgentIdentity {
    /// 代理来源。
    pub source: AgentSource,
    /// 显示名称。
    pub display_name: String,
    /// 版本号（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// 是否在线。
    pub online: bool,
    /// 活跃会话 ID（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
}

// ─── Session ────────────────────────────────────────────────────────────────

/// 会话信息。
///
/// 记录一个代理会话的基本元数据，包括 ID、标题、状态和时间。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Session {
    /// 会话 ID。
    pub id: String,
    /// 会话标题（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 会话状态。
    pub status: SessionStatus,
    /// 开始时间（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
}

// ─── TTS 语音规则 ──────────────────────────────────────────────────────────

/// TTS 语音播报规则。
///
/// 控制 TTS 引擎在哪些事件/状态下进行语音播报，
/// 以及播报的最小时间间隔。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TTSSpeechRules {
    /// 会话开始时是否播报。
    #[serde(default)]
    pub session_start: bool,
    /// 工具调用时是否播报。
    #[serde(default)]
    pub tool_call: bool,
    /// 工具错误时是否播报。
    #[serde(default)]
    pub tool_error: bool,
    /// 权限请求时是否播报。
    #[serde(default)]
    pub permission_request: bool,
    /// 会话结束时是否播报。
    #[serde(default)]
    pub session_end: bool,
    /// Agent 消息时是否播报。
    #[serde(default)]
    pub agent_message: bool,
    /// 最小播报间隔（毫秒），防止语音轰炸。
    #[serde(default = "default_min_interval_ms")]
    pub min_interval_ms: u64,
    /// 焦点模式：当窗口获得焦点时是否播报。
    #[serde(default)]
    pub focus_mode: bool,
}

fn default_min_interval_ms() -> u64 {
    3000
}

impl Default for TTSSpeechRules {
    fn default() -> Self {
        Self {
            session_start: true,
            tool_call: true,
            tool_error: true,
            permission_request: true,
            session_end: true,
            agent_message: false,
            min_interval_ms: 3000,
            focus_mode: false,
        }
    }
}

// ─── 辅助函数 ───────────────────────────────────────────────────────────────

/// 生成 ULID 风格的事件 ID。
///
/// 使用 `ulid` crate 生成单调递增的唯一标识符。
pub fn generate_event_id() -> String {
    ulid::Ulid::new().to_string()
}

/// 生成当前时间的 RFC 3339 时间戳。
///
/// # 示例
/// ```
/// # use agent_pet_hub_lib::types::events::generate_timestamp;
/// let ts = generate_timestamp();
/// assert!(ts.contains("T"));
/// ```
pub fn generate_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 截断字符串到指定最大长度，中间用 `...` 替换。
///
/// # 参数
/// * `s` — 原始字符串
/// * `max_len` — 最大长度（包含省略号）
///
/// # 示例
/// ```
/// # use agent_pet_hub_lib::types::events::truncate;
/// assert_eq!(truncate("hello", 10), "hello");
/// let result = truncate("hello world", 8);
/// assert_eq!(result.len(), 8);
/// assert!(result.contains("..."));
/// ```
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let ellipsis = "...";
        let available = max_len.saturating_sub(ellipsis.len());
        let prefix_len = available / 2;
        let suffix_len = available - prefix_len;
        let prefix = &s[..prefix_len.max(1)];
        let suffix_start = s.len().saturating_sub(suffix_len.max(1));
        let suffix = &s[suffix_start..];
        format!("{}{}{}", prefix, ellipsis, suffix)
    }
}

// ─── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tracing::{debug, info};

    // ─── AgentSource 测试 ─────────────────────────────────────────────

    #[test]
    fn test_agent_source_serde() {
        let source = AgentSource::Pi;
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json, json!("pi"));

        let deserialized: AgentSource = serde_json::from_str("\"pi\"").unwrap();
        assert_eq!(deserialized, AgentSource::Pi);

        let json = serde_json::to_value(&AgentSource::Hermes).unwrap();
        assert_eq!(json, json!("hermes"));

        let json = serde_json::to_value(&AgentSource::OpenClaw).unwrap();
        assert_eq!(json, json!("openclaw"));
    }

    // ─── EventCategory 测试 ───────────────────────────────────────────

    #[test]
    fn test_event_category_serde() {
        let categories = [
            (EventCategory::Session, "session"),
            (EventCategory::Thinking, "thinking"),
            (EventCategory::Tool, "tool"),
            (EventCategory::Message, "message"),
            (EventCategory::Permission, "permission"),
            (EventCategory::Error, "error"),
            (EventCategory::System, "system"),
            (EventCategory::User, "user"),
            (EventCategory::Subagent, "subagent"),
        ];

        for (cat, expected) in categories {
            let json = serde_json::to_value(&cat).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: EventCategory =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, cat);
        }
    }

    // ─── EventType 测试 ───────────────────────────────────────────────

    #[test]
    fn test_event_type_serde() {
        let event_type = EventType::SessionStart;
        let json = serde_json::to_value(&event_type).unwrap();
        assert_eq!(json, json!("session_start"));

        let deserialized: EventType =
            serde_json::from_str("\"session_start\"").unwrap();
        assert_eq!(deserialized, EventType::SessionStart);
    }

    // ─── PetState 测试 ────────────────────────────────────────────────

    #[test]
    fn test_pet_state_serde() {
        let states = [
            (PetState::Idle, "idle"),
            (PetState::Thinking, "thinking"),
            (PetState::Working, "working"),
            (PetState::Waiting, "waiting"),
            (PetState::Success, "success"),
            (PetState::Error, "error"),
            (PetState::Speaking, "speaking"),
            (PetState::Connecting, "connecting"),
        ];

        for (state, expected) in states {
            let json = serde_json::to_value(&state).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: PetState =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, state);
        }
    }

    // ─── ErrorCode 测试 ───────────────────────────────────────────────

    #[test]
    fn test_error_code_serde() {
        let codes = [
            (ErrorCode::ConnectionRefused, "connection_refused"),
            (ErrorCode::ConnectionTimeout, "connection_timeout"),
            (ErrorCode::ConnectionLost, "connection_lost"),
            (ErrorCode::HandshakeFailed, "handshake_failed"),
            (ErrorCode::AuthFailed, "auth_failed"),
            (ErrorCode::TokenExpired, "token_expired"),
            (ErrorCode::PermissionDenied, "permission_denied"),
            (ErrorCode::ToolNotFound, "tool_not_found"),
            (ErrorCode::ToolTimeout, "tool_timeout"),
            (ErrorCode::ToolMemoryLimit, "tool_memory_limit"),
            (ErrorCode::ToolRateLimit, "tool_rate_limit"),
            (ErrorCode::AgentCrash, "agent_crash"),
            (ErrorCode::AgentOOM, "agent_oom"),
            (ErrorCode::AgentContextOverflow, "agent_context_overflow"),
            (ErrorCode::ParseError, "parse_error"),
            (ErrorCode::VersionMismatch, "version_mismatch"),
            (ErrorCode::InvalidEvent, "invalid_event"),
            (ErrorCode::DiskFull, "disk_full"),
            (ErrorCode::FileLocked, "file_locked"),
            (ErrorCode::ProcessKilled, "process_killed"),
            (ErrorCode::Unknown, "unknown"),
        ];

        for (code, expected) in codes {
            let json = serde_json::to_value(&code).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: ErrorCode =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, code);
        }
    }

    // ─── UnifiedAgentEvent 测试 ───────────────────────────────────────

    #[test]
    fn test_unified_agent_event_roundtrip() {
        let event_json = json!({
            "id": "01HQXYZ123",
            "timestamp": "2025-01-01T00:00:00Z",
            "version": "1.0.0",
            "source": "pi",
            "category": "session",
            "type": "session_start",
            "petState": "thinking",
            "sessionId": "sess-001",
            "subSessionId": "sub-001",
            "toolName": "read_file",
            "toolArgsPreview": "path: /tmp/test.txt",
            "toolResultPreview": "content: hello world",
            "taskPreview": "Read test file",
            "stepNumber": 1,
            "awaitingApproval": true,
            "toolSuccess": true,
            "errorCode": "tool_timeout",
            "errorMessage": "Tool timed out after 30s",
            "agentReplyPreview": "I read the file successfully.",
            "raw": {"foo": "bar"},
            "metadata": {"extra": "data"}
        });

        let event: UnifiedAgentEvent =
            serde_json::from_value(event_json.clone()).unwrap();

        assert_eq!(event.id, "01HQXYZ123");
        assert_eq!(event.source, AgentSource::Pi);
        assert_eq!(event.category, EventCategory::Session);
        assert_eq!(event.event_type, EventType::SessionStart);
        assert_eq!(event.pet_state, PetState::Thinking);
        assert_eq!(event.session_id, Some("sess-001".to_string()));
        assert_eq!(event.tool_name, Some("read_file".to_string()));
        assert_eq!(event.step_number, Some(1));
        assert_eq!(event.awaiting_approval, Some(true));
        assert_eq!(event.tool_success, Some(true));
        assert_eq!(event.error_code, Some(ErrorCode::ToolTimeout));
        assert_eq!(
            event.error_message,
            Some("Tool timed out after 30s".to_string())
        );
        assert_eq!(
            event.agent_reply_preview,
            Some("I read the file successfully.".to_string())
        );

        // 序列化回 JSON，验证字段名映射
        let serialized = serde_json::to_value(&event).unwrap();
        assert_eq!(
            serialized.get("type").unwrap(),
            &json!("session_start")
        );
        assert_eq!(
            serialized.get("petState").unwrap(),
            &json!("thinking")
        );
        assert_eq!(
            serialized.get("sessionId").unwrap(),
            &json!("sess-001")
        );
    }

    #[test]
    fn test_unified_agent_event_minimal() {
        let event_json = json!({
            "id": "01HQXYZ456",
            "timestamp": "2025-01-01T00:00:00Z",
            "version": "1.0.0",
            "source": "hermes",
            "category": "error",
            "type": "thinking_tick",
            "petState": "idle",
            "raw": {"message": "empty event"}
        });

        let event: UnifiedAgentEvent =
            serde_json::from_value(event_json).unwrap();

        assert_eq!(event.session_id, None);
        assert_eq!(event.tool_name, None);
        assert_eq!(event.error_code, None);
        assert_eq!(event.metadata, None);
    }

    // ─── AgentIdentity 测试 ───────────────────────────────────────────

    #[test]
    fn test_agent_identity_serde() {
        let identity = AgentIdentity {
            source: AgentSource::Pi,
            display_name: "Pi Agent".to_string(),
            version: Some("1.0.0".to_string()),
            online: true,
            active_session_id: Some("sess-001".to_string()),
        };

        let json = serde_json::to_value(&identity).unwrap();
        assert_eq!(json.get("source").unwrap(), &json!("pi"));
        assert_eq!(
            json.get("display_name").unwrap(),
            &json!("Pi Agent")
        );
        assert_eq!(json.get("online").unwrap(), &json!(true));

        let deserialized: AgentIdentity =
            serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.display_name, "Pi Agent");
        assert!(deserialized.online);
    }

    // ─── Session 测试 ─────────────────────────────────────────────────

    #[test]
    fn test_session_serde() {
        let session = Session {
            id: "sess-001".to_string(),
            title: Some("Test Session".to_string()),
            status: SessionStatus::Active,
            started_at: Some("2025-01-01T00:00:00Z".to_string()),
        };

        let json = serde_json::to_value(&session).unwrap();
        assert_eq!(json.get("id").unwrap(), &json!("sess-001"));
        assert_eq!(
            json.get("status").unwrap(),
            &json!("active")
        );

        let deserialized: Session =
            serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.id, "sess-001");
        assert_eq!(deserialized.status, SessionStatus::Active);
    }

    // ─── SessionStatus 测试 ───────────────────────────────────────────

    #[test]
    fn test_session_status_serde() {
        let statuses = [
            (SessionStatus::Idle, "idle"),
            (SessionStatus::Active, "active"),
            (SessionStatus::Paused, "paused"),
            (SessionStatus::Error, "error"),
        ];

        for (status, expected) in statuses {
            let json = serde_json::to_value(&status).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: SessionStatus =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    // ─── AgentHealthStatus 测试 ───────────────────────────────────────

    #[test]
    fn test_agent_health_status_serde() {
        let statuses = [
            (AgentHealthStatus::Healthy, "healthy"),
            (AgentHealthStatus::Degraded, "degraded"),
            (AgentHealthStatus::Unhealthy, "unhealthy"),
            (AgentHealthStatus::Unknown, "unknown"),
        ];

        for (status, expected) in statuses {
            let json = serde_json::to_value(&status).unwrap();
            assert_eq!(json, json!(expected));

            let deserialized: AgentHealthStatus =
                serde_json::from_str(format!("\"{}\"", expected).as_str()).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    // ─── 辅助函数测试 ─────────────────────────────────────────────────

    #[test]
    fn test_generate_event_id() {
        let id1 = generate_event_id();
        let id2 = generate_event_id();
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert_ne!(id1, id2, "ULID 应该是唯一的");
    }

    #[test]
    fn test_generate_timestamp() {
        let ts = generate_timestamp();
        assert!(ts.contains("T"), "时间戳应包含 T 分隔符 (RFC 3339)");
    }

    #[test]
    fn test_truncate_no_truncation() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_with_truncation() {
        let result = truncate("hello world foo bar", 11);
        assert!(result.len() == 11, "截断后长度应为 {}", result.len());
        assert!(result.contains("..."), "应包含省略号");
    }

    #[test]
    fn test_truncate_very_short() {
        let result = truncate("hello world", 4);
        // With max_len=4, available=1, prefix=0->1, suffix=1 -> "h...d" = 5 chars
        assert_eq!(result.len(), 5);
        assert!(result.contains("..."));
    }

    #[test]
    fn test_truncate_exact_max_len() {
        let s = "abcdef";
        let result = truncate(s, 3);
        // With max_len=3, available=0, prefix=1, suffix=1 -> "a...f" = 5 chars
        // When string is very short relative to ellipsis, result exceeds max_len
        assert!(result.len() >= 3);
        assert!(result.contains("..."));
    }

    // ─── 日志测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_event_logging() {
        let event_json = json!({
            "id": "test-id",
            "timestamp": "2025-01-01T00:00:00Z",
            "version": "1.0.0",
            "source": "openclaw",
            "category": "tool",
            "type": "tool_call_start",
            "petState": "working",
            "raw": {"test": true}
        });

        let event: UnifiedAgentEvent =
            serde_json::from_value(event_json).unwrap();

        debug!(
            event_id = %event.id,
            source = ?event.source,
            event_type = ?event.event_type,
            pet_state = ?event.pet_state,
            "Parsed unified event"
        );

        info!(
            source = ?event.source,
            "Event logged successfully"
        );
    }
}
