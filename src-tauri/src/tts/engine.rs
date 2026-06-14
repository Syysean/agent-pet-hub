/// TTS 语音引擎模块。
///
/// 跨平台文本转语音：
/// - macOS: 使用系统 `say` 命令
/// - Linux: 使用 `espeak` 命令
/// - Windows: 预留（未来使用 Edge-TTS HTTP API）
///
/// 支持播报队列、静音模式、最小间隔控制。

use std::collections::VecDeque;
use std::process::Command;
use std::time::Instant;

use tracing::{debug, info, warn};

use crate::types::{EventType, PetState, TTSSpeechRules};

/// TTS 语音引擎。
///
/// 跨平台文本转语音：
/// - macOS: 使用系统 `say` 命令
/// - Linux: 使用 `espeak` 命令
/// - Windows: 预留（未来使用 Edge-TTS HTTP API）
///
/// 支持播报队列、静音模式、最小间隔控制。
pub struct TTSEngine {
    enabled: bool,
    engine: String,
    volume: f32,
    language: String,
    queue: VecDeque<String>,
    rules: TTSSpeechRules,
    last_speak_time: Option<Instant>,
    is_speaking: bool,
}

impl TTSEngine {
    /// 创建新的 TTS 引擎。
    pub fn new(
        enabled: bool,
        engine: String,
        volume: f32,
        language: String,
        rules: TTSSpeechRules,
    ) -> Self {
        Self {
            enabled,
            engine,
            volume: volume.clamp(0.0, 1.0),
            language,
            queue: VecDeque::new(),
            rules,
            last_speak_time: None,
            is_speaking: false,
        }
    }

    /// 播报文本。
    ///
    /// # 返回值
    ///
    /// - `true` — 文本已加入队列或正在播报
    /// - `false` — TTS 被禁用或静音
    pub fn speak(&mut self, text: &str) -> bool {
        if !self.enabled || text.is_empty() {
            return false;
        }

        // 检查最小间隔
        if let Some(last) = self.last_speak_time {
            let elapsed = last.elapsed().as_millis() as u64;
            if elapsed < self.rules.min_interval_ms {
                debug!(
                    elapsed_ms = elapsed,
                    min_ms = self.rules.min_interval_ms,
                    "TTS throttled by min interval"
                );
                // 加入队列等待
                self.queue.push_back(text.to_string());
                return true;
            }
        }

        info!(text = %truncate_text(text, 100), engine = %self.engine, "Speaking");
        self.spawn_say_command(text);
        self.last_speak_time = Some(Instant::now());
        self.is_speaking = true;

        // 尝试播报队列中的下一项
        self.flush_queue();

        true
    }

    /// 根据状态变更生成播报文本并播报。
    ///
    /// 根据配置规则决定是否播报、播报什么内容。
    pub fn speak_state_change(
        &mut self,
        old_state: &PetState,
        new_state: &PetState,
        event_type: &EventType,
    ) -> bool {
        // 相同状态不播报
        if old_state == new_state {
            return false;
        }

        // 根据事件类型检查规则
        let should_speak = match event_type {
            EventType::SessionStart => self.rules.session_start,
            EventType::ToolCallStart | EventType::ToolBatch => self.rules.tool_call,
            EventType::ToolCallError => self.rules.tool_error,
            EventType::PermissionRequest => self.rules.permission_request,
            EventType::SessionEnd => self.rules.session_end,
            EventType::AgentMessage | EventType::AgentReply => self.rules.agent_message,
            _ => false,
        };

        if !should_speak {
            return false;
        }

        // 生成播报文本
        let text = self.generate_speech_text(new_state, event_type);
        self.speak(&text)
    }

    /// 设置 TTS 启用状态。
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.queue.clear();
            self.is_speaking = false;
        }
    }

    /// 获取队列长度。
    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }

    /// 生成状态变更的播报文本。
    fn generate_speech_text(&self, state: &PetState, _event_type: &EventType) -> String {
        match state {
            PetState::Thinking => "Agent 正在思考".to_string(),
            PetState::Working => "Agent 正在工作".to_string(),
            PetState::Waiting => "Agent 等待用户审批".to_string(),
            PetState::Success => "任务完成".to_string(),
            PetState::Error => "发生错误".to_string(),
            PetState::Idle => "Agent 空闲".to_string(),
            PetState::Connecting => "正在连接".to_string(),
            PetState::Speaking => "正在播报".to_string(),
        }
    }

    /// 尝试清空队列中的待播报文本。
    fn flush_queue(&mut self) {
        if self.is_speaking || self.queue.is_empty() {
            return;
        }

        if let Some(text) = self.queue.pop_front() {
            self.speak(&text);
        }
    }

    /// 根据平台生成 TTS 命令。
    fn spawn_say_command(&self, text: &str) {
        let platform = self.detect_platform();

        // 对 text 做前缀检查：如果以 `-` 开头，可能在 macOS `say` 或 Linux `espeak`
        // 中被误解析为选项参数（如 `-r 180`、`--verbose=quick`）
        let safe_text = if text.starts_with("--") {
            // macOS `say` 将 "--" 后的内容解析为选项，用 "-\x00" 替代 "--"
            format!("-\x00{}", &text[2..])
        } else if text.starts_with('-') {
            format!("\x00{}", text) // 前置 null 字节，命令程序通常跳过
        } else {
            text.to_string()
        };

        match platform.as_str() {
            "macos" => {
                // macOS: use `say` command
                // say -v Siri -r 180 "text"
                let rate = (180.0 * self.volume) as i32;
                let _ = Command::new("say")
                    .args(["-r", &rate.to_string(), "--", &safe_text])
                    .spawn();
            }
            "linux" => {
                // Linux: use `espeak` command
                // espeak -s 150 -v zh-cn -- "text"
                // 白名单校验：仅允许合法的 language 代码（如 "zh-cn", "en", "ja"）
                let valid_language = if self.language.chars().all(|c| {
                    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
                }) && !self.language.contains("--")
                    && !self.language.contains('/')
                {
                    &self.language
                } else {
                    "en" // fallback to English
                };
                let speed = (150.0 * self.volume) as i32;
                let _ = Command::new("espeak")
                    .args(["-s", &speed.to_string(), "-v", valid_language, "--", &safe_text])
                    .spawn();
            }
            "windows" => {
                // Windows: placeholder for Edge-TTS
                debug!(platform = "windows", "TTS not yet implemented for Windows");
            }
            _ => {
                warn!(platform = %platform, "Unknown platform, TTS skipped");
            }
        }
    }

    /// 检测当前运行平台。
    fn detect_platform(&self) -> String {
        std::env::consts::OS.to_string()
    }
}

/// 截断文本用于日志。
fn truncate_text(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_rules() -> TTSSpeechRules {
        TTSSpeechRules {
            session_start: true,
            tool_call: true,
            tool_error: true,
            permission_request: true,
            session_end: true,
            agent_message: true,
            min_interval_ms: 0,
            focus_mode: false,
        }
    }

    #[test]
    fn test_tts_disabled() {
        let mut engine = TTSEngine::new(false, "say".to_string(), 1.0, "zh-cn".to_string(), default_rules());
        assert!(!engine.speak("hello"));
    }

    #[test]
    fn test_tts_enabled_but_empty() {
        let mut engine = TTSEngine::new(true, "say".to_string(), 1.0, "zh-cn".to_string(), default_rules());
        assert!(!engine.speak(""));
    }

    #[test]
    fn test_tts_volume_clamp() {
        let engine = TTSEngine::new(true, "say".to_string(), 2.0, "zh-cn".to_string(), default_rules());
        assert_eq!(engine.volume, 1.0);
    }

    #[test]
    fn test_tts_set_enabled() {
        let mut engine = TTSEngine::new(true, "say".to_string(), 1.0, "zh-cn".to_string(), default_rules());
        engine.set_enabled(false);
        assert!(!engine.enabled);
    }

    #[test]
    fn test_generate_speech_text() {
        let engine = TTSEngine::new(true, "say".to_string(), 1.0, "zh-cn".to_string(), default_rules());

        assert_eq!(
            engine.generate_speech_text(&PetState::Thinking, &EventType::SessionStart),
            "Agent 正在思考"
        );
        assert_eq!(
            engine.generate_speech_text(&PetState::Success, &EventType::SessionEnd),
            "任务完成"
        );
    }

    #[test]
    fn test_detect_platform() {
        let engine = TTSEngine::new(true, "say".to_string(), 1.0, "zh-cn".to_string(), default_rules());
        let platform = engine.detect_platform();
        assert!(!platform.is_empty());
        assert!(
            ["linux", "macos", "windows"].contains(&platform.as_str()),
            "Unexpected platform: {}",
            platform
        );
    }
}
