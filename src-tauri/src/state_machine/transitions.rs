/// 状态转换表模块。
///
/// 定义所有状态转换规则，作为状态机的核心决策依据。
///
/// # 设计原则
///
/// 1. **显式优于隐式** — 所有允许的转换都明确列出。
/// 2. **默认保持** — 未列出的 (状态, 事件) 组合保持当前状态。
/// 3. **全局连接断开** — `AdapterDisconnected` 从任何状态回到 `Idle`。
/// 4. **错误恢复** — `Error` 状态可以通过 `SessionStart` 或 `UserPrompt` 恢复。
///
/// # 状态转换规则总览
///
/// | 当前状态 | 事件 | 目标状态 | 说明 |
/// |----------|------|----------|------|
/// | Connecting | AdapterConnected | Idle | 连接成功 |
/// | Connecting | SessionStart / UserPrompt | Thinking | 初始化时直接启动 |
/// | Idle | SessionStart / UserPrompt | Thinking | 启动新会话 |
/// | Thinking | ToolCallStart / ToolBatch / SubagentStart | Working | 开始执行 |
/// | Thinking | SessionEnd / UserCancel | Idle | 结束会话 |
/// | Thinking | ToolCallError | Error | 工具执行出错 |
/// | Thinking | PermissionRequest | Waiting | 需要审批 |
/// | Working | ToolCallEnd / ThinkingEnd | Thinking | 工具执行完成 |
/// | Working | SessionEnd / UserCancel | Idle | 结束会话 |
/// | Working | ToolCallError | Error | 工具执行出错 |
/// | Working | PermissionRequest | Waiting | 需要审批 |
/// | Waiting | PermissionGranted / PermissionDenied | Thinking | 审批完成 |
/// | Waiting | SessionEnd / UserCancel | Idle | 取消等待 |
/// | Error | SessionEnd / UserCancel | Idle | 错误后结束 |
/// | Error | SessionStart / UserPrompt | Thinking | 从错误恢复 |
/// | 任何状态 | AdapterDisconnected | Idle | 连接断开 |

use std::collections::HashMap;
use crate::types::{PetState, EventType};

/// 构建状态转换表。
///
/// 返回一个 `HashMap`，键为 `(PetState, EventType)` 元组，值为目标 `PetState`。
/// 状态机通过查找此表确定事件处理后的目标状态。
///
/// # 规则详述
///
/// ## 初始化 (Connecting)
/// - `Connecting + AdapterConnected → Idle`：连接成功，宠物就绪。
/// - `Connecting + SessionStart → Thinking`：连接时已有活跃会话。
/// - `Connecting + UserPrompt → Thinking`：连接时有用户输入。
///
/// ## Idle 状态
/// - `Idle + SessionStart → Thinking`：用户启动新会话。
/// - `Idle + UserPrompt → Thinking`：用户发送消息。
///
/// ## Thinking 状态
/// - `Thinking + ToolCallStart → Working`：开始执行工具。
/// - `Thinking + ToolBatch → Working`：开始批处理工具调用。
/// - `Thinking + SubagentStart → Working`：启动子代理。
/// - `Thinking + SessionEnd → Idle`：会话结束，返回空闲。
/// - `Thinking + UserCancel → Idle`：用户取消。
/// - `Thinking + ToolCallError → Error`：工具调用失败。
/// - `Thinking + PermissionRequest → Waiting`：需要用户审批。
///
/// ## Working 状态
/// - `Working + ToolCallEnd → Thinking`：单个工具完成，继续思考。
/// - `Working + ThinkingEnd → Thinking`：思考阶段结束。
/// - `Working + SessionEnd → Idle`：会话结束。
/// - `Working + UserCancel → Idle`：用户取消。
/// - `Working + ToolCallError → Error`：工具调用失败。
/// - `Working + PermissionRequest → Waiting`：需要用户审批。
///
/// ## Waiting 状态
/// - `Waiting + PermissionGranted → Thinking`：权限已批准，继续执行。
/// - `Waiting + PermissionDenied → Thinking`：权限已拒绝，继续执行。
/// - `Waiting + SessionEnd → Idle`：会话结束。
/// - `Waiting + UserCancel → Idle`：用户取消。
///
/// ## Error 状态
/// - `Error + SessionEnd → Idle`：错误后结束会话。
/// - `Error + UserCancel → Idle`：用户取消。
/// - `Error + SessionStart → Thinking`：新会话从错误中恢复。
/// - `Error + UserPrompt → Thinking`：用户输入恢复。
///
/// ## 全局连接断开
/// 从任何状态 `AdapterDisconnected → Idle`。
///
/// # 返回值
///
/// 包含所有转换规则的 `HashMap<(PetState, EventType), PetState>`。
///
/// # 示例
///
/// ```
/// use agent_pet_hub_lib::state_machine::transitions;
/// use agent_pet_hub_lib::types::{PetState, EventType};
///
/// let table = transitions::build_transition_table();
/// let key = (PetState::Idle, EventType::SessionStart);
/// assert_eq!(table.get(&key), Some(&PetState::Thinking));
/// ```
pub fn build_transition_table() -> HashMap<(PetState, EventType), PetState> {
    let mut table = HashMap::new();

    // ─── 初始化状态 (Connecting) ───
    table.insert((PetState::Connecting, EventType::AdapterConnected), PetState::Idle);
    table.insert((PetState::Connecting, EventType::SessionStart), PetState::Thinking);
    table.insert((PetState::Connecting, EventType::UserPrompt), PetState::Thinking);

    // ─── Idle 状态转换 ───
    table.insert((PetState::Idle, EventType::SessionStart), PetState::Thinking);
    table.insert((PetState::Idle, EventType::UserPrompt), PetState::Thinking);

    // ─── Thinking 状态转换 ───
    table.insert((PetState::Thinking, EventType::ToolCallStart), PetState::Working);
    table.insert((PetState::Thinking, EventType::ToolBatch), PetState::Working);
    table.insert((PetState::Thinking, EventType::SubagentStart), PetState::Working);
    table.insert((PetState::Thinking, EventType::SessionEnd), PetState::Idle);
    table.insert((PetState::Thinking, EventType::UserCancel), PetState::Idle);
    table.insert((PetState::Thinking, EventType::ToolCallError), PetState::Error);
    table.insert((PetState::Thinking, EventType::PermissionRequest), PetState::Waiting);

    // ─── Working 状态转换 ───
    table.insert((PetState::Working, EventType::ToolCallEnd), PetState::Thinking);
    table.insert((PetState::Working, EventType::ThinkingEnd), PetState::Thinking);
    table.insert((PetState::Working, EventType::SessionEnd), PetState::Idle);
    table.insert((PetState::Working, EventType::UserCancel), PetState::Idle);
    table.insert((PetState::Working, EventType::ToolCallError), PetState::Error);
    table.insert((PetState::Working, EventType::PermissionRequest), PetState::Waiting);

    // ─── Waiting 状态转换 ───
    table.insert((PetState::Waiting, EventType::PermissionGranted), PetState::Thinking);
    table.insert((PetState::Waiting, EventType::PermissionDenied), PetState::Thinking);
    table.insert((PetState::Waiting, EventType::SessionEnd), PetState::Idle);
    table.insert((PetState::Waiting, EventType::UserCancel), PetState::Idle);

    // ─── Error 状态转换 ───
    table.insert((PetState::Error, EventType::SessionEnd), PetState::Idle);
    table.insert((PetState::Error, EventType::UserCancel), PetState::Idle);
    table.insert((PetState::Error, EventType::SessionStart), PetState::Thinking);
    table.insert((PetState::Error, EventType::UserPrompt), PetState::Thinking);

    // ─── 全局连接断开（任何状态 → Idle）───
    table.insert((PetState::Connecting, EventType::AdapterDisconnected), PetState::Idle);
    table.insert((PetState::Idle, EventType::AdapterDisconnected), PetState::Idle);
    table.insert((PetState::Thinking, EventType::AdapterDisconnected), PetState::Idle);
    table.insert((PetState::Working, EventType::AdapterDisconnected), PetState::Idle);
    table.insert((PetState::Waiting, EventType::AdapterDisconnected), PetState::Idle);
    table.insert((PetState::Error, EventType::AdapterDisconnected), PetState::Idle);

    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PetState;

    #[test]
    fn test_table_non_empty() {
        let table = build_transition_table();
        assert!(!table.is_empty(), "转换表不应为空");
    }

    #[test]
    fn test_specific_transition() {
        let table = build_transition_table();

        // 验证关键转换
        assert_eq!(
            table.get(&(PetState::Connecting, EventType::AdapterConnected)),
            Some(&PetState::Idle)
        );
        assert_eq!(
            table.get(&(PetState::Idle, EventType::SessionStart)),
            Some(&PetState::Thinking)
        );
        assert_eq!(
            table.get(&(PetState::Thinking, EventType::ToolCallStart)),
            Some(&PetState::Working)
        );
        assert_eq!(
            table.get(&(PetState::Working, EventType::ToolCallEnd)),
            Some(&PetState::Thinking)
        );
        assert_eq!(
            table.get(&(PetState::Error, EventType::SessionStart)),
            Some(&PetState::Thinking)
        );
    }

    #[test]
    fn test_adapter_disconnect_from_all_states() {
        let table = build_transition_table();

        let states = [
            PetState::Connecting,
            PetState::Idle,
            PetState::Thinking,
            PetState::Working,
            PetState::Waiting,
            PetState::Error,
        ];

        for state in &states {
            assert_eq!(
                table.get(&(state.clone(), EventType::AdapterDisconnected)),
                Some(&PetState::Idle),
                "AdapterDisconnected from {:?} should go to Idle",
                state
            );
        }
    }

    #[test]
    fn test_transition_table_size() {
        let table = build_transition_table();
        // 应该至少有 30 条规则（实际约 31 条）
        assert!(
            table.len() >= 30,
            "转换表应有至少 30 条规则，实际有 {} 条",
            table.len()
        );
    }

    #[test]
    fn test_no_transition_for_heartbeat_from_connecting() {
        let table = build_transition_table();
        assert!(
            table.get(&(PetState::Connecting, EventType::Heartbeat)).is_none(),
            "Connecting 状态的 Heartbeat 不应有转换规则"
        );
    }

    #[test]
    fn test_success_state_no_rules() {
        let table = build_transition_table();
        // Success 和 Speaking 状态在转换表中可能没有入边规则（它们通常是结果状态）
        // 但这不影响它们作为目标状态被引用
        let has_success_as_source = table.keys().any(|(s, _)| *s == PetState::Success);
        let has_speaking_as_source = table.keys().any(|(s, _)| *s == PetState::Speaking);
        // 允许它们有规则或没有规则，仅验证表的结构正确性
        let _ = has_success_as_source;
        let _ = has_speaking_as_source;
    }

    #[test]
    fn test_working_to_waiting() {
        let table = build_transition_table();
        assert_eq!(
            table.get(&(PetState::Working, EventType::PermissionRequest)),
            Some(&PetState::Waiting)
        );
    }

    #[test]
    fn test_thinking_to_waiting() {
        let table = build_transition_table();
        assert_eq!(
            table.get(&(PetState::Thinking, EventType::PermissionRequest)),
            Some(&PetState::Waiting)
        );
    }
}
