/// WebSocket 服务器模块。
///
/// 实现基于 `tokio-tungstenite` 的 WebSocket 服务器，
/// 为外部进程提供事件推送服务。

use std::sync::atomic::{AtomicUsize, Ordering};

/// WebSocket 消息最大大小（64KB），防止 OOM 攻击
const MAX_WS_MESSAGE_SIZE: usize = 64 * 1024;

/// 静态空字节数组，用于心跳 Ping 消息，避免每次心跳分配临时 Vec
static EMPTY_PING_PAYLOAD: &[u8] = &[];

use tokio::net::TcpStream;

use chrono;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::{accept_async, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};
use uuid;

use crate::types::UnifiedAgentEvent;

#[derive(Debug, thiserror::Error)]
pub enum WSServerError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] TungsteniteError),
    #[error("Bind failed: {0}")]
    BindFailed(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Channel closed")]
    ChannelClosed,
}

/// WebSocket 服务器 — 为外部进程提供事件推送。
///
/// 支持 Bearer Token 认证、事件订阅、心跳机制、最大连接数限制。
pub struct WSServer {
    event_tx: broadcast::Sender<UnifiedAgentEvent>,
    port: u16,
    auth_token: String,
    /// 当前活跃连接数（原子计数器）
    connections: std::sync::Arc<AtomicUsize>,
    /// 最大允许连接数
    max_connections: usize,
}

impl WSServer {
    /// 创建新的 WebSocket 服务器。
    ///
    /// # 参数
    ///
    /// * `event_tx` — 从 EventBus 获取的事件发送者
    /// * `port` — 监听端口（默认 8765）
    /// * `auth_token` — Bearer Token 认证密钥
    /// * `max_connections` — 最大允许并发连接数（默认 10）
    pub fn new(
        event_tx: broadcast::Sender<UnifiedAgentEvent>,
        port: u16,
        auth_token: String,
        max_connections: usize,
    ) -> Self {
        Self {
            event_tx,
            port,
            auth_token,
            connections: std::sync::Arc::new(AtomicUsize::new(0)),
            max_connections,
        }
    }

    /// 启动 WebSocket 服务器。
    ///
    /// 阻塞当前任务直到服务器关闭。
    pub async fn run(self) -> Result<(), WSServerError> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).await?;
        info!(port = self.port, "WebSocket server listening");

        loop {
            let (stream, addr) = listener.accept().await?;

            // 检查最大连接数
            let current = self.connections.load(Ordering::Relaxed);
            if current >= self.max_connections {
                warn!(
                    current_connections = current,
                    max_connections = self.max_connections,
                    "Max connections reached, rejecting new connection"
                );
                continue;
            }

            let connections = self.connections.clone();
            let token = self.auth_token.clone();
            let event_tx = self.event_tx.clone();

            // 原子增加连接计数
            connections.fetch_add(1, Ordering::Relaxed);

            debug!(%addr, "New WebSocket connection");

            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(stream, token, event_tx).await {
                    warn!(%addr, error = %e, "WebSocket client error");
                }
                // 连接断开后原子减少计数
                connections.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }

    /// 处理单个 WebSocket 客户端连接。
    async fn handle_client(
        ws: TcpStream,
        auth_token: String,
        event_tx: broadcast::Sender<UnifiedAgentEvent>,
    ) -> Result<(), WSServerError> {
        let mut ws_stream = accept_async(ws).await?;
        let mut event_rx = event_tx.subscribe();
        let client_id = uuid::Uuid::new_v4().to_string();
        let mut authorized = false; // 维护客户端认证状态

        info!(client_id = %client_id, "WebSocket client connected");

        let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        // 合并所有操作的统一事件循环
        loop {
            tokio::select! {
                // 客户端消息
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // 消息大小限制（64KB），防止 OOM 攻击
                            if text.len() > MAX_WS_MESSAGE_SIZE {
                                warn!(size = text.len(), "WS message too large, rejecting");
                                let resp = serde_json::json!({
                                    "type": "error",
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                    "payload": { "message": "Message too large (max 64KB)" }
                                });
                                let _ = ws_stream.send(Message::Text(serde_json::to_string(&resp)?.into())).await;
                                continue;
                            }
                            match Self::handle_text_message(&mut ws_stream, &text, &auth_token, authorized).await {
                                Ok(new_auth) => authorized = new_auth,
                                Err(e) => {
                                    warn!(error = %e, "Error handling text message");
                                    break;
                                }
                            }
                        }
                        Some(Ok(Message::Ping(_))) => {
                            // tokio-tungstenite handles ping automatically
                        }
                        Some(Ok(Message::Close(_))) | Some(Ok(Message::Binary(_))) | Some(Ok(Message::Frame(_))) => {
                            break;
                        }
                        Some(Ok(Message::Pong(_))) => {}
                        Some(Err(TungsteniteError::ConnectionClosed)) | Some(Err(TungsteniteError::AlreadyClosed)) => {
                            break;
                        }
                        Some(Err(e)) => {
                            warn!(error = %e, "WebSocket receive error");
                            break;
                        }
                        None => {
                            break;
                        }
                    }
                }

                // 事件推送（仅推送给已认证的客户端）
                event = event_rx.recv() => {
                    if authorized {
                        match event {
                            Ok(event) => {
                                // 过滤 raw 字段：如果超过 8KB，只保留截断标记和原始大小
                                let filtered_raw = event.raw.as_ref().and_then(|r| {
                                    let json_str = r.to_string();
                                    if json_str.len() > 8192 {
                                        Some(serde_json::json!({ "truncated": true, "original_size": json_str.len() }))
                                    } else {
                                        Some(r.clone())
                                    }
                                });
                                let ws_msg = serde_json::json!({
                                    "type": "event",
                                    "timestamp": event.timestamp,
                                    "payload": { "event": {
                                        "id": event.id,
                                        "timestamp": event.timestamp,
                                        "version": event.version,
                                        "source": event.source.to_string(),
                                        "category": format!("{:?}", event.category).to_lowercase(),
                                        "type": format!("{:?}", event.event_type).to_lowercase(),
                                        "petState": format!("{:?}", event.pet_state).to_lowercase(),
                                        "sessionId": event.session_id,
                                        "toolName": event.tool_name,
                                        "toolArgsPreview": event.tool_args_preview,
                                        "toolResultPreview": event.tool_result_preview,
                                        "taskPreview": event.task_preview,
                                        "stepNumber": event.step_number,
                                        "awaitingApproval": event.awaiting_approval,
                                        "toolSuccess": event.tool_success,
                                        "errorCode": event.error_code.as_ref().map(|e| format!("{:?}", e).to_lowercase()),
                                        "errorMessage": event.error_message,
                                        "agentReplyPreview": event.agent_reply_preview,
                                        "raw": filtered_raw,
                                        "rawSize": event.raw_size,
                                        "metadata": event.metadata,
                                    }}
                                });
                                let msg_str = serde_json::to_string(&ws_msg)?;
                                if ws_stream.send(Message::Text(msg_str.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!(lagged = n, "Client lagged behind, dropping events");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                break;
                            }
                        }
                    }
                }

                // 心跳
                _ = heartbeat_interval.tick() => {
                    if ws_stream.send(Message::Ping(EMPTY_PING_PAYLOAD.into())).await.is_err() {
                        break;
                    }
                }
            }
        }

        info!(client_id = %client_id, "WebSocket client disconnected");
        Ok(())
    }

    /// 处理 WebSocket 文本消息。
    ///
    /// # 返回值
    ///
    /// `Ok(true)` — auth 消息且认证成功，客户端已认证。
    /// `Ok(false)` — auth 消息但认证失败，或未认证消息（auth/ping 等）。
    /// `Err` — 解析或发送错误。
    async fn handle_text_message(
        ws_stream: &mut WebSocketStream<TcpStream>,
        text: &str,
        auth_token: &str,
        is_authorized: bool,
    ) -> Result<bool, WSServerError> {
        let msg: serde_json::Value = serde_json::from_str(text)?;
        let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match msg_type {
            "auth" => {
                let token = msg
                    .get("payload")
                    .and_then(|p| p.get("token"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                let authorized = token == auth_token;
                let resp = serde_json::json!({
                    "type": "auth_ack",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "payload": { "authorized": authorized }
                });
                ws_stream
                    .send(Message::Text(serde_json::to_string(&resp)?.into()))
                    .await?;
                return Ok(authorized);
            }
            "subscribe" => {
                // 认证前不允许 subscribe（auth 和 ping 除外）
                if !is_authorized {
                    let resp = serde_json::json!({
                        "type": "error",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "payload": { "message": "Authentication required" }
                    });
                    ws_stream
                        .send(Message::Text(serde_json::to_string(&resp)?.into()))
                        .await?;
                    return Ok(false);
                }
                // 解析客户端订阅的事件类型
                let event_types = msg
                    .get("payload")
                    .and_then(|p| p.get("eventTypes"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let event_types_output: Vec<String> = if event_types.is_empty() {
                    vec!["*".to_string()]
                } else {
                    event_types
                };

                let resp = serde_json::json!({
                    "type": "subscribed",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "payload": {
                        "subscriptionId": uuid::Uuid::new_v4().to_string(),
                        "eventTypes": event_types_output
                    }
                });
                ws_stream
                    .send(Message::Text(serde_json::to_string(&resp)?.into()))
                    .await?;
                return Ok(false);
            }
            "ping" => {
                let resp = serde_json::json!({
                    "type": "pong",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "payload": {}
                });
                ws_stream
                    .send(Message::Text(serde_json::to_string(&resp)?.into()))
                    .await?;
                return Ok(false);
            }
            _ => {
                warn!(msg_type = %msg_type, "Unknown message type");
                return Ok(false);
            }
        }
    }
}
