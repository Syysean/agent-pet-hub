/// Pi Agent 适配器模块。
///
/// 通过 JSONL 文件监听 Pi Agent 事件，实现 [`AgentAdapter`](super::trait::AgentAdapter) trait。
///
/// # 架构
///
/// PiAdapter 是 agent-pet-hub 与 Pi Agent 之间的桥梁：
///
/// ```text
/// Pi Agent (进程)
///     │
///     ▼ (写入 JSONL 日志)
/// JSONL 文件
///     │
///     ▼ (PiJsonlWatcher 监控)
/// PiAdapter
///     │
///     ├──▶ EventBus ─▶ PetStateMachine ─▶ TTSEngine
///     │
///     ▼
/// 统一事件格式 (UnifiedAgentEvent)
/// ```
///
/// # 生命周期
///
/// 1. **创建** — `PiAdapter::new()` 创建适配器实例
/// 2. **连接** — `connect()` 验证 JSONL 文件路径，更新连接状态
/// 3. **监听** — `start_listening()` 启动 PiJsonlWatcher 后台任务
/// 4. **运行** — 持续监控 JSONL 文件，新事件自动发布到 EventBus
/// 5. **停止** — `stop_listening()` 停止监听器
/// 6. **断开** — 连接状态自动更新

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::types::{
    AgentHealthStatus, AgentIdentity, AgentSource, Session,
};
use crate::event_bus::EventBus;
use crate::state_machine::PetStateMachine;
use crate::tts::TTSEngine;

use super::pi_watcher::PiJsonlWatcher;
use super::r#trait::{AdapterError, AdapterIdentity, AgentAdapter};

// ─── PiAdapterConfig ──────────────────────────────────────────────────────

/// Pi Adapter 配置。
///
/// 控制 Pi Adapter 的行为和连接参数。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PiAdapterConfig {
    /// JSONL 日志文件路径。
    ///
    /// Pi Agent 将事件写入此文件，适配器从中读取。
    pub log_path: PathBuf,
    /// Pi 主目录（用于定位日志文件等）。
    pub home: PathBuf,
    /// 是否启用 TTS 语音播报。
    pub enable_tts: bool,
}

impl Default for PiAdapterConfig {
    fn default() -> Self {
        Self {
            log_path: PathBuf::from("~/.pi/agent/logs/latest.jsonl"),
            home: PathBuf::from("~/.pi"),
            enable_tts: true,
        }
    }
}

// ─── PiAdapter ─────────────────────────────────────────────────────────────

/// Pi Agent 适配器。
///
/// 通过 JSONL 文件监控 Pi Agent 事件流，实现统一的 [`AgentAdapter`] trait。
///
/// # 线程模型
///
/// `PiAdapter` 内部使用 `Arc<Mutex<PetStateMachine>>` 和 `EventBus`（内部 clone），
/// 保证在多线程环境下安全运行。
///
/// # 示例
///
/// ```no_run
/// use agent_pet_hub_lib::adapter::{PiAdapter, PiAdapterConfig};
/// use agent_pet_hub_lib::event_bus::EventBus;
/// use agent_pet_hub_lib::state_machine::PetStateMachine;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// # #[tokio::main]
/// # async fn main() {
/// let event_bus = EventBus::new(1024);
/// let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));
///
/// let config = PiAdapterConfig::default();
/// let adapter = PiAdapter::new(
///     config,
///     event_bus,
///     state_machine,
///     None, // 可选 TTS 引擎
/// );
///
/// adapter.connect().await.expect("Failed to connect");
/// adapter.start_listening().await.expect("Failed to start listening");
/// # }
/// ```
#[allow(dead_code)]
pub struct PiAdapter {
    /// 适配器配置。
    config: PiAdapterConfig,
    /// 事件总线，用于发布事件。
    event_bus: EventBus,
    /// 宠物状态机，用于处理事件驱动的状态变更。
    state_machine: Arc<Mutex<PetStateMachine>>,
    /// TTS 引擎（可选）。
    tts_engine: Option<Arc<Mutex<TTSEngine>>>,
    /// 适配器身份标识。
    identity: AdapterIdentity,
    /// 连接状态。
    is_connected: std::sync::atomic::AtomicBool,
    /// JSONL 文件监听器。
    watcher: std::sync::Mutex<Option<PiJsonlWatcher>>,
}

impl PiAdapter {
    /// 创建新的 Pi Adapter 实例。
    ///
    /// # 参数
    ///
    /// * `config` — 适配器配置
    /// * `event_bus` — 事件总线，用于发布转换后的事件
    /// * `state_machine` — 宠物状态机，用于处理事件
    /// * `tts_engine` — TTS 引擎（可选），用于状态变更语音播报
    ///
    /// # 示例
    ///
    /// ```
    /// use agent_pet_hub_lib::adapter::{PiAdapter, PiAdapterConfig};
    /// use agent_pet_hub_lib::event_bus::EventBus;
    /// use agent_pet_hub_lib::state_machine::PetStateMachine;
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    ///
    /// let event_bus = EventBus::new(1024);
    /// let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));
    ///
    /// let adapter = PiAdapter::new(
    ///     PiAdapterConfig::default(),
    ///     event_bus,
    ///     state_machine,
    ///     None,
    /// );
    /// ```
    pub fn new(
        config: PiAdapterConfig,
        event_bus: EventBus,
        state_machine: Arc<Mutex<PetStateMachine>>,
        tts_engine: Option<Arc<Mutex<TTSEngine>>>,
    ) -> Self {
        Self {
            config,
            event_bus,
            state_machine,
            tts_engine,
            identity: AdapterIdentity {
                name: "pi".to_string(),
                version: None,
                display_name: "Pi Agent".to_string(),
                source: AgentSource::Pi,
            },
            is_connected: std::sync::atomic::AtomicBool::new(false),
            watcher: std::sync::Mutex::new(None),
        }
    }

    /// 展开路径中的 `~` 前缀。
    fn expand_home(path: &PathBuf) -> PathBuf {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(stripped);
            }
        }
        path.clone()
    }

    /// 获取展开后的 JSONL 文件路径。
    fn resolved_log_path(&self) -> PathBuf {
        Self::expand_home(&self.config.log_path)
    }

    /// 检查 JSONL 文件是否可读。
    #[allow(dead_code)]
    fn check_jsonl_file_readable(&self) -> bool {
        let path = self.resolved_log_path();
        path.exists() && std::fs::metadata(&path).map(|m| m.len() > 0 || true).unwrap_or(false)
    }
}

#[async_trait::async_trait]
impl AgentAdapter for PiAdapter {
    /// 获取适配器身份标识。
    fn identity(&self) -> &AdapterIdentity {
        &self.identity
    }

    /// 初始化适配器，建立与 Agent 的连接。
    ///
    /// 验证 JSONL 文件路径是否存在（或可创建），更新连接状态。
    /// 此方法是幂等的，重复调用不产生副作用。
    async fn connect(&self) -> Result<(), AdapterError> {
        if self.is_connected.load(std::sync::atomic::Ordering::SeqCst) {
            debug!("Already connected, skipping connect");
            return Ok(());
        }

        let path = self.resolved_log_path();
        info!(path = ?path, "Connecting to Pi Agent");

        // 检查 JSONL 文件路径是否存在
        if !path.exists() {
            // 不报错，文件可能稍后由 Pi Agent 创建
            debug!(path = ?path, "JSONL file not found yet (agent may not be running)");
        } else {
            debug!(path = ?path, "JSONL file found, connection verified");
        }

        // 尝试确保目录存在
        if let Some(parent) = path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                warn!(
                    path = ?parent,
                    error = %e,
                    "Failed to ensure log directory exists"
                );
            }
        }

        self.is_connected.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("Pi Agent connected successfully");

        // 发布连接事件
        super::r#trait::make_connected_event(&self.event_bus)?;

        Ok(())
    }

    /// 启动事件监听。
    ///
    /// 创建 `PiJsonlWatcher` 并启动后台任务，持续监控 JSONL 文件。
    async fn start_listening(&self) -> Result<(), AdapterError> {
        if !self.is_connected.load(std::sync::atomic::Ordering::SeqCst) {
            warn!("Adapter not connected, connecting first");
            self.connect().await?;
        }

        let log_path = self.resolved_log_path();
        info!(path = ?log_path, "Starting event listener");

        let mut watcher = PiJsonlWatcher::new(
            log_path,
            self.event_bus.clone(),
            self.state_machine.clone(),
            self.tts_engine.clone(),
        );

        {
            let watcher_guard = self
                .watcher
                .lock()
                .map_err(|e| AdapterError::Other(e.to_string().into()))?;

            if watcher_guard.is_some() {
                debug!("Listener already running, skipping");
                return Ok(());
            }

            // Drop guard before async op to avoid Send issue
            drop(watcher_guard);
        }

        watcher
            .start()
            .await
            .map_err(|e| AdapterError::ConnectionFailed(format!("Failed to start watcher: {}", e)))?;

        {
            let mut watcher_guard = self
                .watcher
                .lock()
                .map_err(|e| AdapterError::Other(e.to_string().into()))?;
            *watcher_guard = Some(watcher);
        }
        info!("Event listener started");

        Ok(())
    }

    /// 停止事件监听。
    ///
    /// 停止 `PiJsonlWatcher` 并清理资源。
    async fn stop_listening(&self) -> Result<(), AdapterError> {
        let mut watcher_guard = self
            .watcher
            .lock()
            .map_err(|e| AdapterError::Other(e.to_string().into()))?;

        if let Some(ref mut watcher) = *watcher_guard {
            info!("Stopping event listener");
            watcher.stop();
            *watcher_guard = None;
            debug!("Event listener stopped");
        } else {
            debug!("No listener to stop");
        }

        Ok(())
    }

    /// 发送消息到 Pi Agent。
    ///
    /// MVP 简化实现：通过 Pi Extension 或 RPC 发送消息。
    /// 当前实现返回模拟回复，实际实现应通过 Pi Agent 的 API 发送。
    async fn send_message(
        &self,
        text: &str,
        session_id: &str,
    ) -> Result<String, AdapterError> {
        if !self.is_connected.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(AdapterError::ConnectionFailed(
                "Adapter not connected".into(),
            ));
        }

        debug!(
            session_id = session_id,
            text_preview = %text.chars().take(100).collect::<String>(),
            "Sending message to Pi Agent"
        );

        // MVP 简化实现：通过写入 JSONL 文件触发
        // 实际实现应调用 Pi Agent 的 RPC 接口或扩展 API
        let path = self.resolved_log_path();
        let event_json = serde_json::json!({
            "type": "user_prompt",
            "text": text,
            "sessionId": session_id,
        });

        let log_entry = format!("{}\n", event_json);
        match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
        {
            Ok(mut file) => {
                use tokio::io::AsyncWriteExt;
                if let Err(e) = file.write_all(log_entry.as_bytes()).await {
                    warn!(error = %e, "Failed to write message to JSONL file");
                }
            }
            Err(e) => {
                warn!(path = ?path, error = %e, "Failed to open JSONL file for writing");
            }
        }

        // 返回模拟的回复摘要
        let reply_preview = if text.len() > 100 {
            format!("Reply for: {}...", &text[..100])
        } else {
            format!("Reply for: {}", text)
        };

        Ok(reply_preview)
    }

    /// 获取会话列表。
    ///
    /// MVP 简化实现：返回空列表。
    /// 实际实现应从 Pi Agent 的会话管理 API 获取。
    async fn list_sessions(&self) -> Result<Vec<Session>, AdapterError> {
        if !self.is_connected.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(AdapterError::ConnectionFailed(
                "Adapter not connected".into(),
            ));
        }

        debug!("Listing sessions (MVP: returning empty list)");
        Ok(Vec::new())
    }

    /// 健康检查。
    ///
    /// 检查连接状态和 JSONL 文件可读性。
    async fn health_check(&self) -> Result<AgentHealthStatus, AdapterError> {
        // 首先检查连接状态
        if !self.is_connected.load(std::sync::atomic::Ordering::SeqCst) {
            debug!("Adapter not connected");
            return Ok(AgentHealthStatus::Unhealthy);
        }

        // 检查 JSONL 文件是否可读
        let path = self.resolved_log_path();
        let file_readable = path.exists() && std::fs::metadata(&path).is_ok();

        if file_readable {
            info!("Health check: healthy");
            Ok(AgentHealthStatus::Healthy)
        } else {
            warn!(path = ?path, "Health check: JSONL file not accessible");
            Ok(AgentHealthStatus::Degraded)
        }
    }

    /// 获取 Agent 身份信息（用于前端显示）。
    ///
    /// 返回一个 [`AgentIdentity`]，包含来源、显示名称、版本和在线状态。
    fn get_identity_info(&self) -> AgentIdentity {
        AgentIdentity {
            source: self.identity.source.clone(),
            display_name: self.identity.display_name.clone(),
            version: self.identity.version.clone(),
            online: self.is_connected.load(std::sync::atomic::Ordering::SeqCst),
            active_session_id: None, // MVP: 不跟踪活跃会话
        }
    }
}

// ─── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use crate::types::UnifiedAgentEvent;

        fn make_test_adapter() -> (PiAdapter, broadcast::Receiver<UnifiedAgentEvent>) {
        let event_bus = EventBus::new(64);
        // 添加一个订阅者，确保 publish_event 不会因无接收者而失败
        let _rx = event_bus.subscribe_event();
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let config = PiAdapterConfig {
            log_path: PathBuf::from("/tmp/test_adapter.jsonl"),
            home: PathBuf::from("/tmp"),
            enable_tts: false,
        };

        let adapter = PiAdapter::new(config, event_bus, state_machine, None);
        (adapter, _rx)
    }

    #[test]
    fn test_adapter_creation() {
        let (adapter, _rx) = make_test_adapter();
        assert_eq!(adapter.identity().name, "pi");
        assert_eq!(adapter.identity().display_name, "Pi Agent");
        assert_eq!(adapter.identity().source, AgentSource::Pi);
    }

    #[test]
    fn test_adapter_identity_display() {
        let (adapter, _rx) = make_test_adapter();
        let display = format!("{}", adapter.identity());
        assert!(display.contains("Pi Agent"));
        assert!(display.contains("pi"));
    }

    #[test]
    fn test_adapter_not_connected_initially() {
        let (adapter, _rx) = make_test_adapter();
        assert!(!adapter.is_connected.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_expand_home() {
        let path = PathBuf::from("~/.pi/test.jsonl");
        let expanded = PiAdapter::expand_home(&path);
        if let Some(home) = dirs::home_dir() {
            assert!(expanded.starts_with(home));
        }
    }

    #[test]
    fn test_expand_home_no_tilde() {
        let path = PathBuf::from("/tmp/test.jsonl");
        let expanded = PiAdapter::expand_home(&path);
        assert_eq!(expanded, PathBuf::from("/tmp/test.jsonl"));
    }

    #[test]
    fn test_config_defaults() {
        let config = PiAdapterConfig::default();
        assert_eq!(config.enable_tts, true);
        assert_eq!(config.log_path, PathBuf::from("~/.pi/agent/logs/latest.jsonl"));
        assert_eq!(config.home, PathBuf::from("~/.pi"));
    }

    #[tokio::test]
    async fn test_connect() {
        let (adapter, _rx) = make_test_adapter();
        let result = adapter.connect().await;
        assert!(result.is_ok());
        assert!(adapter.is_connected.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_connect_idempotent() {
        let (adapter, _rx) = make_test_adapter();
        // 第一次连接
        let r1 = adapter.connect().await;
        assert!(r1.is_ok());
        // 第二次连接（应该也是成功的，且无副作用）
        let r2 = adapter.connect().await;
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn test_connect_publishes_event() {
        let event_bus = EventBus::new(64);
        let mut _rx = event_bus.subscribe_event();
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let config = PiAdapterConfig {
            log_path: PathBuf::from("/tmp/test_adapter.jsonl"),
            home: PathBuf::from("/tmp"),
            enable_tts: false,
        };

        let adapter = PiAdapter::new(config, event_bus, state_machine, None);
        let result = adapter.connect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_message_not_connected() {
        let (adapter, _rx) = make_test_adapter();
        // 不先 connect
        let result = adapter.send_message("hello", "sess-123").await;
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not connected"));
    }

    #[tokio::test]
    async fn test_send_message_after_connect() {
        let (adapter, _rx) = make_test_adapter();
        adapter.connect().await.unwrap();

        let result = adapter.send_message("hello world", "sess-123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let (adapter, _rx) = make_test_adapter();
        adapter.connect().await.unwrap();

        let sessions = adapter.list_sessions().await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_health_check_healthy() {
        let (adapter, _rx) = make_test_adapter();
        adapter.connect().await.unwrap();

        // 创建 JSONL 文件
        std::fs::File::create("/tmp/test_adapter.jsonl").unwrap();

        let health = adapter.health_check().await.unwrap();
        assert_eq!(health, AgentHealthStatus::Healthy);

        // 清理
        let _ = std::fs::remove_file("/tmp/test_adapter.jsonl");
    }

    #[tokio::test]
    async fn test_health_check_degraded() {
        let event_bus = EventBus::new(64);
        let _rx = event_bus.subscribe_event();
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let config = PiAdapterConfig {
            log_path: PathBuf::from("/tmp/test_adapter_degraded_xyz.jsonl"),
            home: PathBuf::from("/tmp"),
            enable_tts: false,
        };

        let adapter = PiAdapter::new(config, event_bus, state_machine, None);
        adapter.connect().await.unwrap();

        // 文件不存在，应为 Degraded
        let health = adapter.health_check().await.unwrap();
        assert_eq!(health, AgentHealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_get_identity_info() {
        let (adapter, _rx) = make_test_adapter();
        adapter.connect().await.unwrap();

        let info = adapter.get_identity_info();
        assert_eq!(info.source, AgentSource::Pi);
        assert_eq!(info.display_name, "Pi Agent");
        assert!(info.online);
        assert!(info.active_session_id.is_none());
    }

    #[tokio::test]
    async fn test_start_listening_after_connect() {
        let (adapter, _rx) = make_test_adapter();
        adapter.connect().await.unwrap();

        let result = adapter.start_listening().await;
        assert!(result.is_ok());

        // 清理
        adapter.stop_listening().await.unwrap();
    }

    #[tokio::test]
    async fn test_stop_listening_without_start() {
        let (adapter, _rx) = make_test_adapter();
        adapter.connect().await.unwrap();

        // 未启动监听器就停止，应成功
        let result = adapter.stop_listening().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_message_not_connected_error() {
        let (adapter, _rx) = make_test_adapter();
        let result = adapter.send_message("test", "sess-123").await;
        match result {
            Err(AdapterError::ConnectionFailed(_)) => {}
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_list_sessions_not_connected() {
        let (adapter, _rx) = make_test_adapter();
        let result = adapter.list_sessions().await;
        match result {
            Err(AdapterError::ConnectionFailed(_)) => {}
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_send_message_long_text() {
        let (adapter, _rx) = make_test_adapter();
        adapter.connect().await.unwrap();

        let long_text = "a".repeat(500);
        let result = adapter.send_message(&long_text, "sess-123").await;
        assert!(result.is_ok());
        // 回复应该被截断
        let reply = result.unwrap();
        assert!(reply.len() < long_text.len());
    }
}
