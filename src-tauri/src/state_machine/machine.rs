use std::time::{Duration, Instant};
use tracing::{info, debug};

use crate::types::{PetState, PetStateSnapshot, UnifiedAgentEvent};
use crate::state_machine::transitions;

/// 状态变更回调类型。
///
/// 当状态机检测到状态变更时，会调用所有已注册的回调。
/// 回调签名：`fn(old_state: PetState, new_state: PetState)`
pub type StateChangeCallback = Box<dyn Fn(PetState, PetState) + Send + Sync>;

/// 宠物状态机 — 管理宠物状态和状态转换。
///
/// 状态机根据事件类型和当前状态，通过转换表确定目标状态，
/// 并通过防抖机制避免频繁状态切换。
///
/// # 初始状态
///
/// 状态机初始化为 `PetState::Connecting` 状态，
/// 等待 `AdapterConnected` 事件后转为 `Idle`。
///
/// # 线程安全
///
/// `PetStateMachine` 不是 `Sync`，需要在单线程上下文中使用，
/// 或通过 `tokio::sync::Mutex` 包装后在异步上下文中共享。
///
/// # 示例
///
/// ```
/// use agent_pet_hub_lib::state_machine::PetStateMachine;
/// use agent_pet_hub_lib::types::PetState;
///
/// let machine = PetStateMachine::new();
/// assert_eq!(*machine.current_state(), PetState::Connecting);
/// ```
pub struct PetStateMachine {
    /// 当前宠物状态。
    current_state: PetState,
    /// 上一次状态（用于状态还原和快照）。
    previous_state: PetState,
    /// 状态转换表：(当前状态, 事件类型) → 新状态。
    transition_table: std::collections::HashMap<(PetState, crate::types::EventType), PetState>,
    /// 防抖动器：防止频繁状态切换。
    debouncer: StateDebouncer,
    /// 状态变更回调列表。
    callbacks: Vec<StateChangeCallback>,
}

impl PetStateMachine {
    /// 创建新的状态机实例。
    ///
    /// 初始状态为 `PetState::Connecting`，
    /// 防抖间隔为 500ms。
    ///
    /// # 示例
    ///
    /// ```
    /// use agent_pet_hub_lib::state_machine::PetStateMachine;
    ///
    /// let machine = PetStateMachine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            current_state: PetState::Connecting,
            previous_state: PetState::Connecting,
            transition_table: transitions::build_transition_table(),
            debouncer: StateDebouncer::default(),
            callbacks: Vec::new(),
        }
    }

    /// 注册状态变更回调。
    ///
    /// 每次状态变更时，所有已注册的回调都会被调用。
    /// 回调在状态已更新后触发，接收 (旧状态, 新状态) 参数。
    ///
    /// # 参数
    ///
    /// * `callback` — 回调函数，签名应为 `Fn(PetState, PetState) + Send + Sync + 'static`。
    ///
    /// # 示例
    ///
    /// ```
    /// use agent_pet_hub_lib::state_machine::PetStateMachine;
    ///
    /// let mut machine = PetStateMachine::new();
    /// machine.on_state_change(|old, new| {
    ///     println!("{:?} → {:?}", old, new);
    /// });
    /// ```
    pub fn on_state_change<F>(&mut self, callback: F)
    where
        F: Fn(PetState, PetState) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// 处理事件，返回状态变更（如果有）。
    ///
    /// 状态机根据当前状态和事件类型，查找转换表确定目标状态。
    /// 如果找到目标状态且防抖通过，则执行状态变更。
    ///
    /// # 参数
    ///
    /// * `event` — 要处理的统一代理事件。
    ///
    /// # 返回值
///
    /// - `Some((old_state, new_state))` — 状态发生了变化。
    /// - `None` — 状态未变化（无转换规则或防抖触发）。
    ///
    /// # 处理流程
    ///
    /// 1. 查找转换表：`(当前状态, 事件类型)` → `新状态`
    /// 2. 检查防抖：如果距上次变更时间 < 500ms，返回 `None`
    /// 3. 更新状态并记录时间戳
    /// 4. 触发所有注册的回调
    /// 5. 返回状态变更结果
    ///
    /// # 示例
    ///
    /// ```
    /// use agent_pet_hub_lib::state_machine::PetStateMachine;
    /// use agent_pet_hub_lib::types::{PetState, UnifiedAgentEvent, AgentSource, EventCategory, EventType};
    ///
    /// let mut machine = PetStateMachine::new();
    ///
    /// let event = UnifiedAgentEvent {
    ///     id: "test".to_string(),
    ///     timestamp: chrono::Utc::now().to_rfc3339(),
    ///     version: "1.0".to_string(),
    ///     source: AgentSource::Pi,
    ///     category: EventCategory::System,
    ///     event_type: EventType::AdapterConnected,
    ///     pet_state: PetState::Idle,
    ///     session_id: None,
    ///     sub_session_id: None,
    ///     tool_name: None,
    ///     tool_args_preview: None,
    ///     tool_result_preview: None,
    ///     task_preview: None,
    ///     step_number: None,
    ///     awaiting_approval: None,
    ///     tool_success: None,
    ///     error_code: None,
    ///     error_message: None,
    ///     agent_reply_preview: None,
    ///     raw: serde_json::json!({}),
    ///     metadata: None,
    /// };
    ///
    /// let change = machine.handle_event(&event);
    /// assert!(change.is_some());
    /// let (old, new) = change.unwrap();
    /// assert_eq!(old, PetState::Connecting);
    /// assert_eq!(new, PetState::Idle);
    /// ```
    pub fn handle_event(&mut self, event: &UnifiedAgentEvent) -> Option<(PetState, PetState)> {
        // 查找转换表确定目标状态
        let target_state = self.transition_table.get(&(self.current_state.clone(), event.event_type.clone())).cloned();

        let new_state = match target_state {
            Some(state) => state,
            None => {
                debug!(
                    current = ?self.current_state,
                    event_type = ?event.event_type,
                    "No transition rule found, keeping current state"
                );
                return None;
            }
        };

        // 防抖动检查
        if !self.debouncer.should_change_state(&new_state, &self.current_state) {
            debug!(
                current = ?self.current_state,
                target = ?new_state,
                "State change debounced"
            );
            return None;
        }

        // 记录状态变更
        let old_state = self.current_state.clone();
        self.previous_state = old_state.clone();
        self.current_state = new_state.clone();
        self.debouncer.record_state_change(new_state.clone());

        info!(
            event_type = ?event.event_type,
            source = ?event.source,
            old_state = ?old_state,
            new_state = ?new_state,
            "State changed"
        );

        // 触发回调
        for cb in &self.callbacks {
            cb(old_state.clone(), new_state.clone());
        }

        Some((old_state, new_state))
    }

    /// 获取当前状态的引用。
    pub fn current_state(&self) -> &PetState {
        &self.current_state
    }

    /// 获取上一次状态的引用。
    pub fn previous_state(&self) -> &PetState {
        &self.previous_state
    }

    /// 强制设置状态（用于初始化等场景）。
    ///
    /// 绕过防抖机制，直接将状态设置为指定值。
    /// 同时更新上一次状态为原当前状态。
    ///
    /// 注意：此方法将防抖时间戳设置为足够早的值，
    /// 以确保后续通过 `handle_event` 的转换不会被防抖。
    ///
    /// # 参数
    ///
    /// * `state` — 要设置的目标状态。
    pub fn set_state(&mut self, state: PetState) {
        let old = self.current_state.clone();
        self.previous_state = old.clone();
        self.current_state = state.clone();
        // 将时间戳设为足够早的值，确保下次 handle_event 不防抖
        self.debouncer.last_state_change = Some(Instant::now() - self.debouncer.min_hold_duration - Duration::from_secs(1));
        debug!(old_state = ?old, new_state = ?state, "State force set");
    }

    /// 获取完整的宠物状态快照。
    ///
    /// 返回一个包含当前状态、上一次状态和时间戳的结构体。
    /// 可用于持久化或发送到前端。
    pub fn get_snapshot(&self) -> PetStateSnapshot {
        PetStateSnapshot {
            mood: crate::types::pet::calculate_mood(&self.current_state, 0),
            animation: crate::types::pet::map_pet_state_to_animation(&self.current_state),
            position: crate::types::pet::PetPosition { x: 0.0, y: 0.0 },
            pet_state: self.current_state.clone(),
            active_agent: None,
            session_id: None,
            error_count: 0,
        }
    }
}

impl Default for PetStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// 宠物状态防抖动器。
///
/// 防止频繁状态切换导致动画闪烁。
/// 通过记录上次状态变更时间，检查距当前时间是否超过最小保持时间。
///
/// # 默认行为
///
/// - 最小状态保持时间：500ms
/// - 首次状态变更不防抖
/// - 相同状态之间不触发防抖（状态未变化直接返回 false）
pub struct StateDebouncer {
    /// 最小状态保持时间。
    pub(crate) min_hold_duration: Duration,
    /// 上次状态变更的时间点。
    pub(crate) last_state_change: Option<Instant>,
}

impl StateDebouncer {
    /// 创建新的防抖器。
    ///
    /// # 参数
    ///
    /// * `min_hold_duration` — 最小状态保持时间，低于此时间的状态变更将被防抖。
    ///
    /// # 示例
    ///
    /// ```
    /// use agent_pet_hub_lib::state_machine::StateDebouncer;
    /// use std::time::Duration;
    ///
    /// let debouncer = StateDebouncer::new(Duration::from_millis(1000));
    /// ```
    pub fn new(min_hold_duration: Duration) -> Self {
        Self {
            min_hold_duration,
            last_state_change: None,
        }
    }

    /// 检查是否应该改变状态。
    ///
    /// # 参数
    ///
    /// * `new_state` — 目标状态。
    /// * `current_state` — 当前状态。
    ///
    /// # 返回值
    ///
    /// - `true` — 可以改变状态（状态不同且距上次变更时间足够长）。
    /// - `false` — 不应改变状态（状态相同或防抖触发）。
    ///
    /// # 防抖规则
    ///
    /// 1. 如果 `new_state == current_state`，直接返回 `false`（状态未变化）。
    /// 2. 如果是首次变更（`last_state_change` 为 `None`），返回 `true`。
    /// 3. 如果距上次变更时间 `< min_hold_duration`，返回 `false`（防抖）。
    /// 4. 否则返回 `true`。
    pub fn should_change_state(&mut self, new_state: &PetState, current_state: &PetState) -> bool {
        if new_state == current_state {
            return false; // 状态没变
        }

        match self.last_state_change {
            Some(last_change) => {
                let elapsed = last_change.elapsed();
                let allowed = elapsed >= self.min_hold_duration;
                if !allowed {
                    debug!(
                        elapsed_ms = elapsed.as_millis(),
                        min_ms = self.min_hold_duration.as_millis(),
                        "Debouncing state change"
                    );
                }
                allowed
            }
            None => true, // 首次变更，不防抖
        }
    }

    /// 记录状态变更时间。
    ///
    /// # 参数
    ///
    /// * `state` — 变更到的新状态（目前仅用于调试日志）。
    pub fn record_state_change(&mut self, _state: PetState) {
        self.last_state_change = Some(Instant::now());
    }
}

impl Default for StateDebouncer {
    fn default() -> Self {
        Self {
            min_hold_duration: Duration::from_millis(500),
            last_state_change: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentSource, EventCategory, EventType};

    fn make_event(event_type: EventType) -> UnifiedAgentEvent {
        UnifiedAgentEvent {
            id: crate::types::generate_event_id(),
            timestamp: crate::types::generate_timestamp(),
            version: "1.0".to_string(),
            source: AgentSource::Pi,
            category: EventCategory::Session,
            event_type,
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
            raw: Some(serde_json::json!({})),
            raw_size: None,
            metadata: None,
        }
    }

    #[test]
    fn test_machine_initial_state() {
        let machine = PetStateMachine::new();
        assert_eq!(*machine.current_state(), PetState::Connecting);
        assert_eq!(*machine.previous_state(), PetState::Connecting);
    }

    #[test]
    fn test_machine_default() {
        let machine = PetStateMachine::default();
        assert_eq!(*machine.current_state(), PetState::Connecting);
    }

    #[test]
    fn test_connecting_to_idle_on_adapter_connected() {
        let mut machine = PetStateMachine::new();
        let event = make_event(EventType::AdapterConnected);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (old, new) = change.unwrap();
        assert_eq!(old, PetState::Connecting);
        assert_eq!(new, PetState::Idle);
    }

    #[test]
    fn test_connecting_to_thinking_on_session_start() {
        let mut machine = PetStateMachine::new();
        let event = make_event(EventType::SessionStart);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Thinking);
    }

    #[test]
    fn test_idle_to_thinking_on_session_start() {
        let mut machine = PetStateMachine::new();
        // 先连接到 Idle
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        assert_eq!(*machine.current_state(), PetState::Idle);
        // 等待防抖间隔过后
        std::thread::sleep(std::time::Duration::from_millis(600));

        // 然后启动 session
        let event = make_event(EventType::SessionStart);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Thinking);
    }

    #[test]
    fn test_thinking_to_working() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();
        assert_eq!(*machine.current_state(), PetState::Thinking);

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::ToolCallStart);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Working);
    }

    #[test]
    fn test_working_to_thinking_on_tool_end() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::ToolCallStart)).unwrap();
        assert_eq!(*machine.current_state(), PetState::Working);

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::ToolCallEnd);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Thinking);
    }

    #[test]
    fn test_thinking_to_error_on_tool_call_error() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::ToolCallError);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Error);
    }

    #[test]
    fn test_thinking_to_waiting_on_permission_request() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::PermissionRequest);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Waiting);
    }

    #[test]
    fn test_waiting_to_thinking_on_permission_granted() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::PermissionRequest)).unwrap();
        assert_eq!(*machine.current_state(), PetState::Waiting);

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::PermissionGranted);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Thinking);
    }

    #[test]
    fn test_any_state_to_idle_on_session_end() {
        for initial_state in [
            PetState::Thinking,
            PetState::Working,
            PetState::Waiting,
            PetState::Error,
        ] {
            let mut machine = PetStateMachine::new();
            machine.set_state(initial_state.clone());

            let event = make_event(EventType::SessionEnd);
            let change = machine.handle_event(&event);
            assert!(change.is_some(), "SessionEnd from {:?} should transition", initial_state);
            let (_, new) = change.unwrap();
            assert_eq!(new, PetState::Idle, "SessionEnd from {:?} should go to Idle", initial_state);
        }
    }

    #[test]
    fn test_adapter_disconnect_from_any_state() {
        let event = make_event(EventType::AdapterDisconnected);

        for initial_state in [
            PetState::Connecting,
            PetState::Thinking,
            PetState::Working,
            PetState::Waiting,
            PetState::Error,
        ] {
            let mut machine = PetStateMachine::new();
            machine.set_state(initial_state.clone());

            let change = machine.handle_event(&event);
            assert!(change.is_some(), "AdapterDisconnected from {:?} should transition", initial_state);
            let (_, new) = change.unwrap();
            assert_eq!(new, PetState::Idle, "AdapterDisconnected from {:?} should go to Idle", initial_state);
        }
    }

    #[test]
    fn test_no_transition_returns_none() {
        let mut machine = PetStateMachine::new();
        // Heartbeat 在 Connecting 状态下没有转换规则
        let event = make_event(EventType::Heartbeat);
        let change = machine.handle_event(&event);
        assert!(change.is_none());
    }

    #[test]
    fn test_debouncing() {
        use std::time::Duration as StdDuration;

        let mut machine = PetStateMachine::new();

        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        assert_eq!(*machine.current_state(), PetState::Idle);

        // 快速触发 Thinking（500ms 内，应该被防抖）
        let event = make_event(EventType::SessionStart);
        let change = machine.handle_event(&event);
        assert!(change.is_none(), "快速连续的事件应被防抖");
        assert_eq!(*machine.current_state(), PetState::Idle);

        // 等待防抖间隔过后
        std::thread::sleep(StdDuration::from_millis(600));

        // 再次触发 Thinking，应该成功
        let event2 = make_event(EventType::SessionStart);
        let change2 = machine.handle_event(&event2);
        assert!(change2.is_some(), "防抖间隔后应允许状态变更");
        assert_eq!(*machine.current_state(), PetState::Thinking);

        // 再快速触发一次 Thinking → Thinking（相同状态，不防抖）
        let event3 = make_event(EventType::SessionStart);
        let change3 = machine.handle_event(&event3);
        assert!(change3.is_none());
        assert_eq!(*machine.current_state(), PetState::Thinking);
    }

    #[test]
    fn test_set_state() {
        let mut machine = PetStateMachine::new();
        assert_eq!(*machine.current_state(), PetState::Connecting);

        machine.set_state(PetState::Idle);
        assert_eq!(*machine.current_state(), PetState::Idle);
        assert_eq!(*machine.previous_state(), PetState::Connecting);
    }

    #[test]
    fn test_get_snapshot() {
        let machine = PetStateMachine::new();
        let snapshot = machine.get_snapshot();
        assert_eq!(snapshot.pet_state, PetState::Connecting);
    }

    #[test]
    fn test_state_change_callback() {
        use std::sync::{Arc, Mutex};

        let mut machine = PetStateMachine::new();
        let recorded = Arc::new(Mutex::new(Vec::new()));

        machine.on_state_change({
            let recorded = recorded.clone();
            move |old, new| {
                recorded.lock().unwrap().push((old, new));
            }
        });

        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();

        let changes = recorded.lock().unwrap();
        assert_eq!(changes.len(), 1);
        let (old, new) = &changes[0];
        assert_eq!(*old, PetState::Connecting);
        assert_eq!(*new, PetState::Idle);
    }

    #[test]
    fn test_debouncer_creation() {
        let mut debouncer = StateDebouncer::new(Duration::from_millis(100));
        let r1 = debouncer.should_change_state(&PetState::Thinking, &PetState::Idle);
        assert!(r1);

        let r2 = debouncer.should_change_state(&PetState::Thinking, &PetState::Thinking);
        assert!(!r2); // 相同状态

        let r3 = debouncer.should_change_state(&PetState::Working, &PetState::Idle);
        assert!(r3); // 首次不同状态变更
    }

    #[test]
    fn test_error_to_thinking_on_new_session() {
        let mut machine = PetStateMachine::new();
        machine.set_state(PetState::Error);

        let event = make_event(EventType::SessionStart);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Thinking);
    }

    #[test]
    fn test_working_to_error() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::ToolCallStart)).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::ToolCallError);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Error);
    }

    #[test]
    fn test_subagent_start_from_thinking() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::SubagentStart);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Working);
    }

    #[test]
    fn test_thinking_end_from_working() {
        let mut machine = PetStateMachine::new();
        machine.handle_event(&make_event(EventType::AdapterConnected)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::SessionStart)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        machine.handle_event(&make_event(EventType::ToolCallStart)).unwrap();
        assert_eq!(*machine.current_state(), PetState::Working);

        std::thread::sleep(std::time::Duration::from_millis(600));
        let event = make_event(EventType::ThinkingEnd);
        let change = machine.handle_event(&event);
        assert!(change.is_some());
        let (_, new) = change.unwrap();
        assert_eq!(new, PetState::Thinking);
    }

    #[test]
    fn test_user_cancel_from_any_active_state() {
        for initial in [PetState::Thinking, PetState::Working, PetState::Waiting] {
            let mut machine = PetStateMachine::new();
            machine.set_state(initial.clone());

            let event = make_event(EventType::UserCancel);
            let change = machine.handle_event(&event);
            assert!(change.is_some(), "UserCancel from {:?} should transition", initial);
            let (_, new) = change.unwrap();
            assert_eq!(new, PetState::Idle);
        }
    }

    #[test]
    fn test_transition_table_completeness() {
        // 验证核心状态（有 outgoing 转换的）都在转换表中被定义过
        // Success 和 Speaking 是结果状态，通常没有 outgoing 规则
        let table = transitions::build_transition_table();

        let states_with_rules = [
            PetState::Connecting,
            PetState::Idle,
            PetState::Thinking,
            PetState::Working,
            PetState::Waiting,
            PetState::Error,
        ];

        for state in &states_with_rules {
            let mut has_rules = false;
            for ((s, _), _) in &table {
                if s == state {
                    has_rules = true;
                    break;
                }
            }
            assert!(has_rules, "State {:?} should have at least one transition rule", state);
        }
    }
}
