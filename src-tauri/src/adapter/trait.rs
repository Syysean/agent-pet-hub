/// 适配器接口定义模块。
///
/// 定义统一的 Agent 适配器接口（`AgentAdapter` trait），所有 Agent 适配器必须实现此接口。
/// 提供适配器身份标识（`AdapterIdentity`）和适配器错误类型（`AdapterError`）。
///
/// # 设计原则
///
/// 1. **统一接口** — 所有 Agent（Pi、Hermes、OpenClaw）通过同一 trait 交互
/// 2. **错误隔离** — 各适配器使用统一的 `AdapterError`，内部错误被包裹
/// 3. **身份标识** — 每个适配器提供身份信息，用于 UI 显示和连接管理

use std::fmt;

use tracing::info;

use crate::types::{
    AgentHealthStatus, AgentIdentity, AgentSource, PetState, Session, UnifiedAgentEvent,
    generate_event_id, generate_timestamp,
};
use crate::event_bus::EventBus;

// ─── AdapterIdentity ────────────────────────────────────────────────────────

/// 适配器身份标识。
///
/// 描述一个适配器（Agent 连接）的基本属性，用于内部管理和日志记录。
/// 与 [`AgentIdentity`](crate::types::AgentIdentity) 不同，后者包含在线状态和活跃会话信息，
/// 用于前端显示；而 `AdapterIdentity` 仅描述适配器本身。
///
/// # 字段
///
/// * `name` — 适配器唯一标识（如 "pi"、"hermes"）
/// * `version` — 适配器版本（可选）
/// * `display_name` — 显示名称（用于 UI）
/// * `source` — 代理来源
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdapterIdentity {
    /// 适配器名称标识。
    pub name: String,
    /// 适配器版本（可选）。
    pub version: Option<String>,
    /// 显示名称（用于 UI 展示）。
    pub display_name: String,
    /// 代理来源。
    pub source: AgentSource,
}

impl fmt::Display for AdapterIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.display_name, self.source)
    }
}

// ─── AdapterError ───────────────────────────────────────────────────────────

/// 适配器错误类型。
///
/// 所有适配器方法可能返回的错误都通过此枚举统一表示，
/// 包含连接、解析、超时、权限等各类错误。
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AdapterError {
    /// 连接失败（网络问题、Agent 未启动等）。
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// 事件解析错误（JSON 格式不正确等）。
    #[error("Event parse error: {0}")]
    ParseError(String),

    /// 超时（超过指定时间未完成操作）。
    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// 未找到资源（会话不存在、文件不存在等）。
    #[error("Not found: {0}")]
    NotFound(String),

    /// 权限不足。
    #[error("Permission denied")]
    PermissionDenied,

    /// IO 错误。
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 其他错误（包裹任意 Send + Sync 错误）。
    #[error("Other: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

// ─── AgentAdapter Trait ─────────────────────────────────────────────────────

/// Agent 适配器 trait — 所有 Agent 适配器必须实现此接口。
///
/// 定义了所有 Agent（Pi、Hermes、OpenClaw 等）适配器必须实现的统一方法集：
///
/// - `connect()` / `start_listening()` / `stop_listening()` — 生命周期管理
/// - `send_message()` — 发送消息给 Agent
/// - `list_sessions()` — 获取会话列表
/// - `health_check()` — 健康检查
/// - `identity()` / `get_identity_info()` — 身份标识
///
/// # 实现者须知
///
/// 1. 实现必须线程安全（`Send + Sync`）
/// 2. `connect()` 应该幂等：重复调用不产生副作用
/// 3. `start_listening()` 和 `stop_listening()` 应该可重复调用
/// 4. 错误类型必须使用 `AdapterError`，不要返回具体实现错误
///
/// # 示例
///
/// ```ignore
/// #[async_trait::async_trait]
/// impl AgentAdapter for PiAdapter {
///     fn identity(&self) -> &AdapterIdentity {
///         &self.identity
///     }
///
///     async fn connect(&self) -> Result<(), AdapterError> {
///         // ...
///         # Ok(())
///     }
///
///     // ... 其他方法
/// }
/// ```
#[async_trait::async_trait]
#[allow(dead_code)]
pub trait AgentAdapter: Send + Sync {
    /// 获取适配器身份标识。
    ///
    /// # 返回值
    ///
    /// 不可变引用到 [`AdapterIdentity`]。
    fn identity(&self) -> &AdapterIdentity;

    /// 初始化适配器，建立与 Agent 的连接。
    ///
    /// 此方法应该是幂等的：重复调用不应产生副作用。
    /// 如果已经连接，直接返回 `Ok(())`。
    ///
    /// # 错误
    ///
    /// 连接失败时返回 `AdapterError::ConnectionFailed`。
    async fn connect(&self) -> Result<(), AdapterError>;

    /// 启动事件监听。
    ///
    /// 开始监听 Agent 的事件流，新事件将通过 [`EventBus`](crate::event_bus::EventBus)
    /// 发布，并通过状态机处理。
    ///
    /// # 错误
    ///
    /// 监听启动失败时返回 `AdapterError`。
    async fn start_listening(&self) -> Result<(), AdapterError>;

    /// 停止事件监听。
    ///
    /// 优雅地停止事件监听，清理资源。
    /// 此方法应该是幂等的：重复调用不产生副作用。
    ///
    /// # 错误
    ///
    /// 停止失败时返回 `AdapterError`。
    async fn stop_listening(&self) -> Result<(), AdapterError>;

    /// 发送消息到 Agent。
    ///
    /// 将文本消息发送给 Agent，返回 Agent 的回复摘要。
    ///
    /// # 参数
    ///
    /// * `text` — 要发送的消息文本。
    /// * `session_id` — 会话 ID，用于确定消息归属。
    ///
    /// # 返回值
    ///
    /// Agent 回复的文本摘要（截断到合理长度）。
    ///
    /// # 错误
    ///
    /// 发送失败时返回 `AdapterError`。
    async fn send_message(&self, text: &str, session_id: &str) -> Result<String, AdapterError>;

    /// 获取会话列表。
    ///
    /// # 返回值
    ///
    /// 会话列表，每个会话包含 ID、标题、状态和时间。
    /// MVP 实现可以返回空列表。
    async fn list_sessions(&self) -> Result<Vec<Session>, AdapterError>;

    /// 健康检查。
    ///
    /// 检查适配器的连接状态和 Agent 的可访问性。
    ///
    /// # 返回值
    ///
    /// - `AgentHealthStatus::Healthy` — 适配器健康
    /// - `AgentHealthStatus::Degraded` — 适配器部分功能异常
    /// - `AgentHealthStatus::Unhealthy` — 适配器完全不可用
    /// - `AgentHealthStatus::Unknown` — 无法确定状态
    async fn health_check(&self) -> Result<AgentHealthStatus, AdapterError>;

    /// 获取 Agent 身份信息（用于前端显示）。
    ///
    /// 返回一个 [`AgentIdentity`]，包含来源、显示名称、版本和在线状态。
    /// 前端通过此信息在 UI 上显示 Agent 连接状态。
    fn get_identity_info(&self) -> AgentIdentity;
}

// ─── 工具函数 ───────────────────────────────────────────────────────────────

/// 生成适配器连接事件。
///
/// 当适配器成功连接时，生成一个统一事件并发布到事件总线。
pub fn make_connected_event(event_bus: &EventBus) -> Result<(), AdapterError> {
    let event = UnifiedAgentEvent {
        id: generate_event_id(),
        timestamp: generate_timestamp(),
        version: "1.0".to_string(),
        source: AgentSource::Pi, // 默认来源，调用者可修改
        category: crate::types::EventCategory::System,
        event_type: crate::types::EventType::AdapterConnected,
        pet_state: PetState::Connecting,
        session_id: None,
        sub_session_id: None,
        tool_name: None,
        tool_args_preview: None,
        tool_result_preview: None,
        task_preview: None,
        step_number: None,
        awaiting_approval: None,
        tool_success: None,
        error_code: None,
        error_message: None,
        agent_reply_preview: None,
        raw: Some(serde_json::json!({"adapter_connected": true})),
        raw_size: None,
        metadata: None,
    };
    event_bus.publish_event(event).map_err(|e| {
        AdapterError::Other(format!("Failed to publish event: {}", e).into())
    })?;
    info!("Adapter connected event published");
    Ok(())
}

// ─── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_identity_display() {
        let identity = AdapterIdentity {
            name: "pi".to_string(),
            version: Some("1.0.0".to_string()),
            display_name: "Pi Agent".to_string(),
            source: AgentSource::Pi,
        };
        let display = format!("{}", identity);
        assert!(display.contains("Pi Agent"));
        assert!(display.contains("pi"));
    }

    #[test]
    fn test_adapter_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let adapter_err: AdapterError = io_err.into();
        let err_str = format!("{}", adapter_err);
        assert!(err_str.contains("IO error"));
    }

    #[test]
    fn test_adapter_error_from_box() {
        let boxed: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "box error"));
        let adapter_err: AdapterError = boxed.into();
        let err_str = format!("{}", adapter_err);
        assert!(err_str.contains("Other"));
    }

    #[test]
    fn test_adapter_identity_equality() {
        let a = AdapterIdentity {
            name: "pi".to_string(),
            version: Some("1.0.0".to_string()),
            display_name: "Pi Agent".to_string(),
            source: AgentSource::Pi,
        };
        let b = AdapterIdentity {
            name: "pi".to_string(),
            version: Some("1.0.0".to_string()),
            display_name: "Pi Agent".to_string(),
            source: AgentSource::Pi,
        };
        assert_eq!(a, b);
        assert_eq!(a, b); // Hash-based equality
    }

    #[test]
    fn test_adapter_identity_hash() {
        use std::collections::HashSet;

        let a = AdapterIdentity {
            name: "pi".to_string(),
            version: Some("1.0.0".to_string()),
            display_name: "Pi Agent".to_string(),
            source: AgentSource::Pi,
        };
        let mut set = HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&a));
    }

    #[test]
    fn test_adapter_identity_source_variants() {
        let sources = [
            AgentSource::Pi,
            AgentSource::Hermes,
            AgentSource::OpenClaw,
        ];

        for source in sources {
            let identity = AdapterIdentity {
                name: "test".to_string(),
                version: None,
                display_name: format!("{} Adapter", source),
                source: source.clone(),
            };
            assert_eq!(identity.source, source);
        }
    }

    #[test]
    fn test_adapter_error_variants() {
        // ConnectionFailed
        let err = AdapterError::ConnectionFailed("server unreachable".to_string());
        assert!(format!("{}", err).contains("server unreachable"));

        // ParseError
        let err = AdapterError::ParseError("invalid JSON".to_string());
        assert!(format!("{}", err).contains("invalid JSON"));

        // Timeout
        let err = AdapterError::Timeout(std::time::Duration::from_secs(30));
        assert!(format!("{}", err).contains("30s"));

        // NotFound
        let err = AdapterError::NotFound("session not found".to_string());
        assert!(format!("{}", err).contains("session not found"));

        // PermissionDenied
        let err = AdapterError::PermissionDenied;
        assert_eq!(format!("{}", err), "Permission denied");
    }

    #[test]
    fn test_make_connected_event() {
        let bus = EventBus::new(64);
        let mut _rx = bus.subscribe_event();
        let result = make_connected_event(&bus);
        assert!(result.is_ok());
    }
}
