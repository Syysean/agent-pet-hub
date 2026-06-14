/// 宠物状态机模块。
///
/// 管理宠物状态和状态转换，根据代理事件类型驱动状态变更。
///
/// # 架构
///
/// 状态机由三个子模块组成：
///
/// - `machine` — 状态机核心：管理当前状态、防抖动、回调注册
/// - `transitions` — 状态转换表：定义所有 (当前状态, 事件类型) → 新状态的映射规则
///
/// # 状态机设计
///
/// 宠物有以下状态：
///
/// - `Connecting` — 初始化连接中
/// - `Idle` — 空闲等待
/// - `Thinking` — 思考中
/// - `Working` — 工作中（执行工具调用）
/// - `Waiting` — 等待用户审批
/// - `Success` — 成功完成
/// - `Error` — 发生错误
/// - `Speaking` — 正在说话
///
/// # 防抖动
///
/// 为防止频繁状态切换导致动画闪烁，状态机内置防抖机制：
/// - 最小状态保持时间：500ms（可配置）
/// - 两次状态变更时间间隔不足时，新状态被忽略
///
/// # 使用示例
///
/// ```no_run
/// use agent_pet_hub_lib::state_machine::{PetStateMachine, transitions};
/// use agent_pet_hub_lib::types::{PetState, UnifiedAgentEvent, AgentSource, EventCategory, EventType};
///
/// let mut machine = PetStateMachine::new();
///
/// // 注册状态变更回调
/// machine.on_state_change(|old, new| {
///     println!("State changed: {:?} → {:?}", old, new);
/// });
///
/// // 模拟一个 session_start 事件
/// let event = UnifiedAgentEvent {
///     id: "test".to_string(),
///     timestamp: chrono::Utc::now().to_rfc3339(),
///     version: "1.0".to_string(),
///     source: AgentSource::Pi,
///     category: EventCategory::Session,
///     event_type: EventType::SessionStart,
///     pet_state: PetState::Thinking,
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
/// ```

mod machine;
pub mod transitions;

pub use machine::{PetStateMachine, StateChangeCallback, StateDebouncer};
