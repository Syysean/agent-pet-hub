/// IPC 模块：WebSocket 服务器用于外部进程通信。
///
/// 提供基于 WebSocket 的实时事件推送服务，
/// 允许外部进程订阅 agent-pet-hub 内部的事件流。
///
/// # 功能
///
/// - Bearer Token 认证
/// - 事件订阅与推送
/// - 心跳机制（30s ping/pong）
/// - 多客户端连接支持
///
/// # 使用示例
///
/// ```no_run
/// use agent_pet_hub_lib::event_bus::EventBus;
/// use agent_pet_hub_lib::ipc::WSServer;
///
/// let bus = EventBus::new(1024);
/// let server = WSServer::new(
///     bus.event_tx(),
///     8765,
///     "my-secret-token".to_string(),
/// );
/// // tokio::spawn(server.run());
/// ```
pub mod ws_server;
pub use ws_server::WSServer;
