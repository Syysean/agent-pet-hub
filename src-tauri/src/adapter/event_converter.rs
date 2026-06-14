/// Pi 事件转换器模块。
///
/// 将 Pi Agent 的原生 JSON 事件转换为统一的 `UnifiedAgentEvent` 格式。
///
/// # 事件映射
///
/// Pi 原生事件类型到统一事件类型的映射规则：
///
/// | Pi 事件类型 | 统一事件类型 | 事件分类 | 宠物状态 |
/// |-------------|-------------|---------|---------|
/// | `session_start` / `user_prompt` | `SessionStart` | `Session` | `Thinking` |
/// | `tool_call` | `ToolCallStart` | `Tool` | `Working` |
/// | `tool_result` | `ToolCallEnd` | `Tool` | `Thinking` |
/// | `tool_error` | `ToolCallError` | `Error` | `Error` |
/// | `turn_end` | `SessionEnd` | `Session` | `Idle` |
/// | `compaction` | `SessionCompaction` | `Session` | `Thinking` |
/// | `text_delta` | `ThinkingTick` | `Thinking` | `Thinking` |
/// | 其他未知类型 | `ThinkingTick` | `Thinking` | `Thinking` |
///
/// # 设计原则
///
/// 1. **容错性** — 未知事件类型默认视为 `ThinkingTick`
/// 2. **完整性** — 原始 JSON 始终保留在 `raw` 字段中
/// 3. **截断** — 长文本被截断到合理长度，避免内存浪费

use serde_json::Value;
use tracing::debug;

use crate::types::{
    AgentSource, EventCategory, EventType, PetState, ErrorCode,
    UnifiedAgentEvent, generate_event_id, generate_timestamp,
};

use super::r#trait::AdapterError;

/// raw 字段最大字节数（8KB），超出后截断为占位信息
const MAX_RAW_SIZE_BYTES: usize = 8 * 1024;

/// 将 raw 值截断到最大字节数限制。
/// 超出时替换为 `{ truncated: true, original_size: N }`。
fn truncate_raw(raw: &Value) -> Value {
    let json_str = raw.to_string();
    if json_str.len() <= MAX_RAW_SIZE_BYTES {
        raw.clone()
    } else {
        serde_json::json!({
            "truncated": true,
            "original_size": json_str.len(),
            "preview": &json_str[..MAX_RAW_SIZE_BYTES.min(json_str.len())]
        })
    }
}

// ─── EventConverter ────────────────────────────────────────────────────────

/// Pi 事件转换器。
///
/// 将 Pi Agent 的原生 JSON 事件转换为统一事件格式。
///
/// # 转换流程
///
/// ```text
/// 原始 JSON
///     │
///     ▼
/// 提取 "type" 字段
///     │
///     ▼
/// 匹配事件类型
///     │
///     ▼
/// 提取辅助字段（tool_name, prompt 等）
///     │
///     ▼
/// 构建 UnifiedAgentEvent
///     │
///     ▼
/// 返回结果
/// ```
///
/// # 示例
///
/// ```
/// use agent_pet_hub_lib::adapter::event_converter::EventConverter;
/// use agent_pet_hub_lib::types::{AgentSource, EventType, PetState};
/// use serde_json::json;
///
/// let raw = json!({
///     "type": "session_start",
///     "prompt": "Hello world"
/// });
///
/// let event = EventConverter::convert(&raw).unwrap();
/// assert_eq!(event.event_type, EventType::SessionStart);
/// assert_eq!(event.pet_state, PetState::Thinking);
/// ```
pub struct EventConverter;

impl EventConverter {
    /// 将 Pi 原生事件转换为统一事件格式。
    ///
    /// # 参数
    ///
    /// * `raw` — 原始 JSON 值，应包含 `"type"` 字段。
    ///
    /// # 返回值
    ///
    /// - `Ok(UnifiedAgentEvent)` — 转换成功
    /// - `Err(AdapterError::ParseError)` — 缺少必要字段或类型未知
    ///
    /// # 错误
    ///
    /// 如果 JSON 缺少 `"type"` 字段，返回 `ParseError`。
    /// 其他解析错误会被记录为 debug 日志并映射为 `ThinkingTick`。
    ///
    /// # raw 字段截断
    ///
    /// `raw` 字段最大允许 8KB，超出则替换为截断占位信息，
    /// 防止内存 DoS。
    pub fn convert(raw: &Value) -> Result<UnifiedAgentEvent, AdapterError> {
        let event_type_str = raw
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::ParseError("Missing 'type' field".into()))?;

        let timestamp = generate_timestamp();
        let id = generate_event_id();

        // 提取 session_id（兼容 sessionId 和 session_id 两种格式）
        let session_id = raw
            .get("sessionId")
            .or_else(|| raw.get("session_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 根据事件类型映射到统一事件
        match event_type_str {
            "session_start" | "user_prompt" => {
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Session,
                    EventType::SessionStart,
                    PetState::Thinking,
                    session_id,
                    Self::extract_prompt(raw),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    None,
                ))
            }

            "tool_call" => {
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Tool,
                    EventType::ToolCallStart,
                    PetState::Working,
                    session_id.clone(),
                    None,
                    Self::extract_tool_name(raw),
                    Self::extract_tool_args(raw),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    None,
                ))
            }

            "tool_result" => {
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Tool,
                    EventType::ToolCallEnd,
                    PetState::Thinking,
                    session_id,
                    None,
                    Self::extract_tool_name(raw),
                    None,
                    Self::extract_result_preview(raw),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    None,
                ))
            }

            "tool_error" => {
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Error,
                    EventType::ToolCallError,
                    PetState::Error,
                    session_id,
                    None,
                    Self::extract_tool_name(raw),
                    None,
                    Self::extract_result_preview(raw),
                    Some(ErrorCode::ToolTimeout),
                    Self::extract_error_message(raw),
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    Some(serde_json::json!({"raw_type": event_type_str})),
                ))
            }

            "turn_end" => {
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Session,
                    EventType::SessionEnd,
                    PetState::Idle,
                    session_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    None,
                ))
            }

            "compaction" => {
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Session,
                    EventType::SessionCompaction,
                    PetState::Thinking,
                    session_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    None,
                ))
            }

            "text_delta" => {
                // 持续生成文本，保持在 Thinking 状态
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Thinking,
                    EventType::ThinkingTick,
                    PetState::Thinking,
                    session_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    Some(serde_json::json!({"raw_type": event_type_str})),
                ))
            }

            _ => {
                // 未知事件类型，默认为 thinking_tick
                debug!(
                    raw_type = %event_type_str,
                    "Unknown Pi event type, treating as thinking_tick"
                );
                Ok(Self::make_event(
                    id,
                    timestamp,
                    AgentSource::Pi,
                    EventCategory::Thinking,
                    EventType::ThinkingTick,
                    PetState::Thinking,
                    session_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    truncate_raw(raw),
                    Some(serde_json::json!({"raw_type": event_type_str})),
                ))
            }
        }
    }

    // ─── 辅助方法 ───────────────────────────────────────────────────────

    /// 构建统一事件。
    ///
    /// 将各字段组装为 `UnifiedAgentEvent`。
    /// 使用 crate 级别的 `truncate` 函数进行截断。
    #[allow(clippy::too_many_arguments)]
    fn make_event(
        id: String,
        timestamp: String,
        source: AgentSource,
        category: EventCategory,
        event_type: EventType,
        pet_state: PetState,
        session_id: Option<String>,
        task_preview: Option<String>,
        tool_name: Option<String>,
        tool_args_preview: Option<String>,
        tool_result_preview: Option<String>,
        error_code: Option<ErrorCode>,
        error_message: Option<String>,
        tool_success: Option<bool>,
        awaiting_approval: Option<bool>,
        step_number: Option<i32>,
        agent_reply_preview: Option<String>,
        raw: Value,
        metadata: Option<Value>,
    ) -> UnifiedAgentEvent {
        UnifiedAgentEvent {
            id,
            timestamp,
            version: "1.0".to_string(),
            source,
            category,
            event_type,
            pet_state,
            session_id,
            sub_session_id: None,
            tool_name,
            tool_args_preview,
            tool_result_preview,
            task_preview,
            step_number,
            awaiting_approval,
            tool_success,
            error_code,
            error_message,
            agent_reply_preview,
            raw: Some(raw),
            raw_size: None,
            metadata,
        }
    }

    /// 提取工具名称。
    ///
    /// 兼容 `"tool"` 和 `"method"` 两种字段名。
    fn extract_tool_name(raw: &Value) -> Option<String> {
        raw.get("tool")
            .or_else(|| raw.get("method"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// 提取工具参数。
    ///
    /// 兼容 `"args"` 和 `"params"` 两种字段名，截断到 200 字符。
    fn extract_tool_args(raw: &Value) -> Option<String> {
        raw.get("args")
            .or_else(|| raw.get("params"))
            .and_then(|v| v.as_str())
            .map(|s| crate::types::truncate(s, 200))
    }

    /// 提取工具结果预览。
    ///
    /// 兼容 `"result"` 和 `"output"` 两种字段名，截断到 500 字符。
    fn extract_result_preview(raw: &Value) -> Option<String> {
        raw.get("result")
            .or_else(|| raw.get("output"))
            .and_then(|v| v.as_str())
            .map(|s| crate::types::truncate(s, 500))
    }

    /// 提取提示文本。
    ///
    /// 按优先级查找 `"prompt"`、`"message"`、`"text"` 字段，截断到 300 字符。
    fn extract_prompt(raw: &Value) -> Option<String> {
        raw.get("prompt")
            .or_else(|| raw.get("message"))
            .or_else(|| raw.get("text"))
            .and_then(|v| v.as_str())
            .map(|s| crate::types::truncate(s, 300))
    }

    /// 提取错误消息。
    ///
    /// 查找 `"error"` 字段，截断到 500 字符。
    fn extract_error_message(raw: &Value) -> Option<String> {
        raw.get("error")
            .and_then(|v| v.as_str())
            .map(|s| crate::types::truncate(s, 500))
    }

    /// 截断字符串到指定最大长度，中间用 `...` 替换。
    ///
    /// 委托给 [`crate::types::truncate`]。
    #[allow(dead_code)]
    fn truncate(s: &str, max_len: usize) -> String {
        crate::types::truncate(s, max_len)
    }
}

// ─── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_session_start() {
        let raw = serde_json::json!({
            "type": "session_start",
            "prompt": "Hello world"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.source, AgentSource::Pi);
        assert_eq!(event.event_type, EventType::SessionStart);
        assert_eq!(event.pet_state, PetState::Thinking);
        assert_eq!(event.task_preview, Some("Hello world".to_string()));
    }

    #[test]
    fn test_convert_user_prompt() {
        let raw = serde_json::json!({
            "type": "user_prompt",
            "message": "What is the weather?"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::SessionStart);
        assert_eq!(event.pet_state, PetState::Thinking);
    }

    #[test]
    fn test_convert_tool_call() {
        let raw = serde_json::json!({
            "type": "tool_call",
            "tool": "Bash",
            "args": "ls -la"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::ToolCallStart);
        assert_eq!(event.pet_state, PetState::Working);
        assert_eq!(event.tool_name, Some("Bash".to_string()));
    }

    #[test]
    fn test_convert_tool_call_with_method_field() {
        let raw = serde_json::json!({
            "type": "tool_call",
            "method": "read_file",
            "params": "path: /tmp/test"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::ToolCallStart);
        assert_eq!(event.tool_name, Some("read_file".to_string()));
    }

    #[test]
    fn test_convert_tool_result() {
        let raw = serde_json::json!({
            "type": "tool_result",
            "tool": "Bash",
            "result": "file1 file2 file3"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::ToolCallEnd);
        assert_eq!(event.pet_state, PetState::Thinking);
        assert_eq!(event.tool_name, Some("Bash".to_string()));
    }

    #[test]
    fn test_convert_tool_result_with_output_field() {
        let raw = serde_json::json!({
            "type": "tool_result",
            "tool": "read_file",
            "output": "file content here"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::ToolCallEnd);
        assert_eq!(event.tool_result_preview, Some("file content here".to_string()));
    }

    #[test]
    fn test_convert_tool_error() {
        let raw = serde_json::json!({
            "type": "tool_error",
            "tool": "Bash",
            "error": "Command timed out after 30 seconds"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::ToolCallError);
        assert_eq!(event.pet_state, PetState::Error);
        assert_eq!(event.error_code, Some(ErrorCode::ToolTimeout));
        assert_eq!(
            event.error_message,
            Some("Command timed out after 30 seconds".to_string())
        );
    }

    #[test]
    fn test_convert_turn_end() {
        let raw = serde_json::json!({
            "type": "turn_end"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::SessionEnd);
        assert_eq!(event.pet_state, PetState::Idle);
    }

    #[test]
    fn test_convert_compaction() {
        let raw = serde_json::json!({
            "type": "compaction"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::SessionCompaction);
        assert_eq!(event.pet_state, PetState::Thinking);
    }

    #[test]
    fn test_convert_text_delta() {
        let raw = serde_json::json!({
            "type": "text_delta",
            "delta": "Hello"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::ThinkingTick);
        assert_eq!(event.pet_state, PetState::Thinking);
        assert!(event.metadata.is_some());
    }

    #[test]
    fn test_convert_unknown_event() {
        let raw = serde_json::json!({
            "type": "unknown_event_type"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.event_type, EventType::ThinkingTick);
        assert_eq!(event.pet_state, PetState::Thinking);
        assert!(event.metadata.is_some());
    }

    #[test]
    fn test_convert_missing_type() {
        let raw = serde_json::json!({
            "message": "no type field"
        });
        let result = EventConverter::convert(&raw);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("Missing 'type' field"));
    }

    #[test]
    fn test_convert_with_session_id() {
        let raw = serde_json::json!({
            "type": "session_start",
            "sessionId": "sess-123"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.session_id, Some("sess-123".to_string()));
    }

    #[test]
    fn test_convert_with_session_id_field() {
        let raw = serde_json::json!({
            "type": "tool_call",
            "session_id": "sess-456"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.session_id, Some("sess-456".to_string()));
    }

    #[test]
    fn test_convert_preserves_raw_event() {
        let raw = serde_json::json!({
            "type": "session_start",
            "prompt": "test",
            "extra": "data",
            "nested": {"key": "value"}
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.raw.as_ref().unwrap()["type"], serde_json::json!("session_start"));
        assert_eq!(event.raw.as_ref().unwrap()["extra"], serde_json::json!("data"));
        assert_eq!(event.raw.as_ref().unwrap()["nested"]["key"], serde_json::json!("value"));
    }

    #[test]
    fn test_convert_event_has_ulid_id() {
        let raw = serde_json::json!({
            "type": "session_start"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert!(!event.id.is_empty());
        // ULID 格式：26 字符，以 base32 字符开头
        assert_eq!(event.id.len(), 26);
    }

    #[test]
    fn test_convert_event_has_timestamp() {
        let raw = serde_json::json!({
            "type": "session_start"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert!(event.timestamp.contains("T"));
        assert!(!event.timestamp.is_empty());
    }

    #[test]
    fn test_convert_event_version_is_one() {
        let raw = serde_json::json!({
            "type": "session_start"
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(event.version, "1.0");
    }

    #[test]
    fn test_truncate_no_truncation() {
        assert_eq!(EventConverter::truncate("hello", 10), "hello");
        assert_eq!(EventConverter::truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_with_truncation() {
        // 字符串长度超过 max_len 时才会截断
        let result = EventConverter::truncate("hello world foo bar baz", 20);
        assert!(result.len() <= 20);
        assert!(result.contains("..."));
    }

    #[test]
    fn test_truncate_short_string() {
        // 字符串长度 <= max_len 时不截断
        let result = EventConverter::truncate("hi", 4);
        assert_eq!(result, "hi");

        // 字符串长度 > max_len 时截断
        let result2 = EventConverter::truncate("hello", 4);
        assert!(result2.contains("..."));
    }

    #[test]
    fn test_convert_event_types_coverage() {
        // 验证所有支持的 Pi 事件类型都能被正确转换
        let supported_types = [
            ("session_start", EventType::SessionStart),
            ("user_prompt", EventType::SessionStart),
            ("tool_call", EventType::ToolCallStart),
            ("tool_result", EventType::ToolCallEnd),
            ("tool_error", EventType::ToolCallError),
            ("turn_end", EventType::SessionEnd),
            ("compaction", EventType::SessionCompaction),
            ("text_delta", EventType::ThinkingTick),
        ];

        for (type_str, expected_event_type) in supported_types {
            let raw = serde_json::json!({"type": type_str});
            let event = EventConverter::convert(&raw).unwrap();
            assert_eq!(
                event.event_type, expected_event_type,
                "Event type '{}' should map to {:?}",
                type_str, expected_event_type
            );
        }
    }

    #[test]
    fn test_extract_tool_name_priority() {
        // "tool" 优先于 "method"
        let raw = serde_json::json!({
            "tool": "tool_name",
            "method": "method_name"
        });
        assert_eq!(EventConverter::extract_tool_name(&raw), Some("tool_name".to_string()));

        // 只有 "method"
        let raw = serde_json::json!({"method": "method_name"});
        assert_eq!(EventConverter::extract_tool_name(&raw), Some("method_name".to_string()));

        // 都没有
        let raw = serde_json::json!({});
        assert_eq!(EventConverter::extract_tool_name(&raw), None);
    }

    #[test]
    fn test_extract_tool_args_priority() {
        // "args" 优先于 "params"
        let raw = serde_json::json!({
            "args": "args_value",
            "params": "params_value"
        });
        assert_eq!(
            EventConverter::extract_tool_args(&raw),
            Some("args_value".to_string())
        );

        // 只有 "params"
        let raw = serde_json::json!({"params": "params_value"});
        assert_eq!(
            EventConverter::extract_tool_args(&raw),
            Some("params_value".to_string())
        );

        // 都没有
        let raw = serde_json::json!({});
        assert_eq!(EventConverter::extract_tool_args(&raw), None);
    }

    #[test]
    fn test_extract_result_preview_priority() {
        // "result" 优先于 "output"
        let raw = serde_json::json!({
            "result": "result_value",
            "output": "output_value"
        });
        assert_eq!(
            EventConverter::extract_result_preview(&raw),
            Some("result_value".to_string())
        );

        // 只有 "output"
        let raw = serde_json::json!({"output": "output_value"});
        assert_eq!(
            EventConverter::extract_result_preview(&raw),
            Some("output_value".to_string())
        );

        // 都没有
        let raw = serde_json::json!({});
        assert_eq!(EventConverter::extract_result_preview(&raw), None);
    }

    #[test]
    fn test_extract_prompt_priority() {
        // "prompt" 优先于 "message" 优先于 "text"
        let raw = serde_json::json!({
            "prompt": "prompt_value",
            "message": "message_value",
            "text": "text_value"
        });
        assert_eq!(
            EventConverter::extract_prompt(&raw),
            Some("prompt_value".to_string())
        );

        // 只有 "message"
        let raw = serde_json::json!({"message": "message_value"});
        assert_eq!(
            EventConverter::extract_prompt(&raw),
            Some("message_value".to_string())
        );

        // 只有 "text"
        let raw = serde_json::json!({"text": "text_value"});
        assert_eq!(
            EventConverter::extract_prompt(&raw),
            Some("text_value".to_string())
        );

        // 都没有
        let raw = serde_json::json!({});
        assert_eq!(EventConverter::extract_prompt(&raw), None);
    }

    #[test]
    fn test_long_prompt_truncated() {
        let long_text = "a".repeat(400);
        let raw = serde_json::json!({
            "type": "session_start",
            "prompt": long_text
        });
        let event = EventConverter::convert(&raw).unwrap();
        // 截断到 300 字符
        assert!(event.task_preview.as_ref().unwrap().len() <= 300);
        assert!(event.task_preview.as_ref().unwrap().contains("..."));
    }

    #[test]
    fn test_truncate_raw_small() {
        // Small raw should pass through unchanged
        let raw = serde_json::json!({"key": "value", "nested": {"a": 1}});
        let result = truncate_raw(&raw);
        assert_eq!(result, raw);
    }

    #[test]
    fn test_truncate_raw_large() {
        // Large raw (> 8KB) should be truncated
        let big_data = "x".repeat(20_000);
        let raw = serde_json::json!({"data": big_data});
        let result = truncate_raw(&raw);
        assert!(result.get("truncated").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(result.get("original_size").is_some());
        // Should have a preview field
        assert!(result.get("preview").is_some());
    }

    #[test]
    fn test_convert_preserves_raw_when_small() {
        // Verify that small raw events still preserve original data
        let raw = serde_json::json!({
            "type": "session_start",
            "prompt": "test",
            "extra": "data",
            "nested": {"key": "value"}
        });
        let event = EventConverter::convert(&raw).unwrap();
        assert_eq!(
            event.raw.as_ref().unwrap()["type"],
            serde_json::json!("session_start")
        );
        assert_eq!(
            event.raw.as_ref().unwrap()["extra"],
            serde_json::json!("data")
        );
    }
}
