use tokio::sync::broadcast;
use tracing::{info, debug};

use crate::types::{
    AgentSource, EventCategory, EventType, PetState, UnifiedAgentEvent,
    generate_event_id, generate_timestamp,
};

/// 事件总线核心实现。
///
/// 基于 `tokio::sync::broadcast` 提供发布/订阅机制，
/// 将代理事件和状态变更广播给所有订阅者。
///
/// # 设计
///
/// 事件总线将事件流拆分为两条独立通道，以便消费者可以按需订阅：
///
/// - `event_tx` / `subscribe_event()` — 统一代理事件流
/// - `state_tx` / `subscribe_state()` — 状态变更流（旧状态 + 新状态）
///
/// `EventBus` 实现了 `Clone`（内部为 clone 通道发送者），
/// 可在多个异步任务间安全共享。
///
/// # 示例
///
/// ```no_run
/// use agent_pet_hub_lib::event_bus::EventBus;
///
/// let bus = EventBus::new(1024);
/// let mut rx = bus.subscribe_event();
///
/// // 在另一个任务中发送事件
/// let bus_clone = bus.clone();
/// tokio::spawn(async move {
///     bus_clone.publish_heartbeat().ok();
/// });
///
/// // 接收事件
/// while let Ok(event) = rx.recv().await {
///     println!("Received event: {:?}", event.event_type);
/// }
/// ```
pub struct EventBus {
    /// 统一事件广播通道发送者。
    event_tx: broadcast::Sender<UnifiedAgentEvent>,
    /// 状态变更广播通道发送者。
    state_tx: broadcast::Sender<(PetState, PetState)>,
}

impl EventBus {
    /// 创建新的事件总线实例。
    ///
    /// # 参数
    ///
    /// * `channel_size` — 通道缓冲区大小，即在事件丢失前的最大缓冲数。
    ///   当订阅者消费速度跟不上生产速度时，超过此容量的旧事件会被丢弃。
    ///   推荐值：4096（高吞吐场景）。
    ///
    /// # 示例
    ///
    /// ```
    /// use agent_pet_hub_lib::event_bus::EventBus;
    ///
    /// let bus = EventBus::new(4096);
    /// ```
    pub fn new(channel_size: usize) -> Self {
        let (event_tx, _) = broadcast::channel(channel_size);
        let (state_tx, _) = broadcast::channel(channel_size);
        debug!(channel_size, "EventBus created");
        Self { event_tx, state_tx }
    }

    /// 发布统一事件到事件总线。
    ///
    /// 将事件广播给所有事件订阅者。如果通道满，旧事件会被丢弃。
    ///
    /// # 参数
    ///
    /// * `event` — 要发布的统一代理事件。
    ///
    /// # 返回值
    ///
    /// 成功发送的事件数量（即当前订阅者数量）。
    ///
    /// # 错误
    ///
    /// 如果所有订阅者都已断开（无活跃接收者），返回 `SendError`。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use agent_pet_hub_lib::event_bus::EventBus;
    /// # let bus = EventBus::new(1024);
    /// let event = todo!(); // 构建 UnifiedAgentEvent
    /// match bus.publish_event(event) {
    ///     Ok(count) => println!("Broadcast to {} subscribers", count),
    ///     Err(e) => warn!("Failed to publish event: {}", e),
    /// }
    /// ```
    pub fn publish_event(
        &self,
        event: UnifiedAgentEvent,
    ) -> Result<usize, broadcast::error::SendError<UnifiedAgentEvent>> {
        debug!(
            source = ?event.source,
            event_type = ?event.event_type,
            pet_state = ?event.pet_state,
            "Publishing event"
        );
        self.event_tx.send(event)
    }

    /// 发布状态变更到状态总线。
    ///
    /// 将旧状态和新状态广播给所有状态订阅者。
    ///
    /// # 参数
    ///
    /// * `old_state` — 变更前的宠物状态。
    /// * `new_state` — 变更后的宠物状态。
    ///
    /// # 返回值
    ///
    /// 成功广播的数量。
    ///
    /// # 错误
    ///
    /// 如果所有订阅者都已断开，返回 `SendError`。
    pub fn publish_state_change(
        &self,
        old_state: PetState,
        new_state: PetState,
    ) -> Result<usize, broadcast::error::SendError<(PetState, PetState)>> {
        info!(
            old_state = ?old_state,
            new_state = ?new_state,
            "State changed"
        );
        self.state_tx.send((old_state, new_state))
    }

    /// 创建事件订阅者。
    ///
    /// 返回一个 `broadcast::Receiver<UnifiedAgentEvent>`，
    /// 通过 `recv().await` 接收事件。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use agent_pet_hub_lib::event_bus::EventBus;
    /// let bus = EventBus::new(1024);
    /// let mut rx = bus.subscribe_event();
    ///
    /// tokio::spawn(async move {
    ///     while let Ok(event) = rx.recv().await {
    ///         println!("Event: {:?}", event.event_type);
    ///     }
    /// });
    /// ```
    pub fn subscribe_event(&self) -> broadcast::Receiver<UnifiedAgentEvent> {
        self.event_tx.subscribe()
    }

    /// 创建状态变更订阅者。
    ///
    /// 返回一个 `broadcast::Receiver<(PetState, PetState)>`，
    /// 每个接收值是 `(old_state, new_state)` 元组。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # use agent_pet_hub_lib::event_bus::EventBus;
    /// let bus = EventBus::new(1024);
    /// let mut rx = bus.subscribe_state();
    ///
    /// tokio::spawn(async move {
    ///     while let Ok((old, new)) = rx.recv().await {
    ///         println!("State transition: {:?} → {:?}", old, new);
    ///     }
    /// });
    /// ```
    pub fn subscribe_state(&self) -> broadcast::Receiver<(PetState, PetState)> {
        self.state_tx.subscribe()
    }

    /// 发布系统事件（心跳等）。
    ///
    /// 创建一个心跳事件并发布到事件通道。
    /// 心跳用于检测订阅者是否仍然活跃。
    ///
    /// # 返回值
    ///
    /// 成功广播的数量。
    pub fn publish_heartbeat(&self) -> Result<usize, broadcast::error::SendError<UnifiedAgentEvent>> {
        let event = UnifiedAgentEvent {
            id: generate_event_id(),
            timestamp: generate_timestamp(),
            version: "1.0".to_string(),
            source: AgentSource::Pi, // 系统事件默认来源
            category: EventCategory::System,
            event_type: EventType::Heartbeat,
            pet_state: PetState::Idle,
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
            raw: Some(serde_json::json!({"heartbeat": true})),
            raw_size: None,
            metadata: None,
        };
        debug!("Publishing heartbeat event");
        self.event_tx.send(event)
    }

    /// 获取事件发送者的不可变引用。
    ///
    /// 用于其他模块直接调用 `Sender::send()` 发送自定义事件。
    pub fn event_tx(&self) -> &broadcast::Sender<UnifiedAgentEvent> {
        &self.event_tx
    }

    /// 获取状态发送者的不可变引用。
    pub fn state_tx(&self) -> &broadcast::Sender<(PetState, PetState)> {
        &self.state_tx
    }
}

// EventBus 需要在多个线程间共享
impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            event_tx: self.event_tx.clone(),
            state_tx: self.state_tx.clone(),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(super::DEFAULT_CHANNEL_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentSource, EventCategory, EventType, PetState};
    use tokio::runtime::Runtime;

    fn make_test_event(event_type: EventType) -> UnifiedAgentEvent {
        UnifiedAgentEvent {
            id: generate_event_id(),
            timestamp: generate_timestamp(),
            version: "1.0".to_string(),
            source: AgentSource::Pi,
            category: EventCategory::Session,
            event_type,
            pet_state: PetState::Thinking,
            session_id: Some("test-session".to_string()),
            sub_session_id: None,
            tool_name: None,
            tool_args_preview: None,
            tool_result_preview: None,
            task_preview: Some("Test task".to_string()),
            step_number: None,
            awaiting_approval: None,
            tool_success: None,
            error_code: None,
            error_message: None,
            agent_reply_preview: None,
            raw: Some(serde_json::json!({"test": true})),
            raw_size: None,
            metadata: None,
        }
    }

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new(100);
        assert_eq!(bus.subscribe_event().len(), 0);
        assert_eq!(bus.subscribe_state().len(), 0);
    }

    #[test]
    fn test_event_bus_default() {
        let bus = EventBus::default();
        // 默认通道大小应为 1024
        assert_eq!(bus.subscribe_event().len(), 0);
    }

    #[test]
    fn test_event_bus_clone() {
        let bus = EventBus::new(64);
        let cloned = bus.clone();
        // clone 后原 bus 的通道仍存在
        assert!(bus.subscribe_event().try_recv().is_err());
        assert!(cloned.subscribe_event().try_recv().is_err());
    }

    #[test]
    fn test_publish_and_consume_event() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let bus = EventBus::new(1024);
            // 无订阅者时发送，返回 SendError
            let event = make_test_event(EventType::SessionStart);
            let sent = bus.publish_event(event);
            assert!(sent.is_err(), "无订阅者时发送应返回 SendError");

            // 创建订阅者后再发送
            let mut _rx = bus.subscribe_event();
            let event2 = make_test_event(EventType::ToolCallStart);
            let sent2 = bus.publish_event(event2).unwrap();
            assert!(sent2 > 0, "有订阅者时应返回正数");
        });
    }

    #[test]
    fn test_publish_and_consume_state_change() {
        let bus = EventBus::new(1024);

        // 创建订阅者
        let mut state_rx = bus.subscribe_state();

        // 发送状态变更
        let result = bus.publish_state_change(PetState::Idle, PetState::Thinking);
        assert!(result.is_ok(), "有订阅者时发送应成功");

        // 接收数据
        assert!(state_rx.try_recv().is_ok());
    }

    #[test]
    fn test_heartbeat_event() {
        let bus = EventBus::new(1024);
        let mut _rx = bus.subscribe_event();
        let result = bus.publish_heartbeat();
        assert!(result.is_ok(), "发送心跳事件应成功");
    }

    #[test]
    fn test_channel_backpressure() {
        let bus = EventBus::new(4);
        let mut _rx = bus.subscribe_event();

        // 发送 5 条事件（通道容量为 4）
        // 最后一条应该导致旧事件被丢弃
        for i in 0..5 {
            let event = make_test_event(EventType::ToolCallStart);
            let result = bus.publish_event(event);
            assert!(result.is_ok(), "Event {} should be sent", i);
        }
    }

    #[test]
    fn test_event_tx_method() {
        let bus = EventBus::new(1024);
        let _tx = bus.event_tx();
        // Sender 可被克隆，证明通道存在
        let cloned = _tx.clone();
        assert!(cloned.subscribe().try_recv().is_err());
    }

    #[test]
    fn test_state_tx_method() {
        let bus = EventBus::new(1024);
        let _tx = bus.state_tx();
        // Sender 可被克隆，证明通道存在
        let cloned = _tx.clone();
        assert!(cloned.subscribe().try_recv().is_err());
    }
}
