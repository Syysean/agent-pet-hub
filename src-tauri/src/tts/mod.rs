/// 文本转语音（TTS）引擎模块。
///
/// 支持跨平台 TTS：
/// - macOS: `say` 命令
/// - Linux: `espeak` 命令
/// - Windows: 预留 Edge-TTS
///
/// # 架构
///
/// TTS 引擎提供以下核心功能：
///
/// - 跨平台 TTS 命令执行（macOS `say` / Linux `espeak`）
/// - 播报队列管理，防止语音轰炸
/// - 最小间隔控制（默认 3 秒）
/// - 状态变更自动播报
///
/// # 使用示例
///
/// ```no_run
/// use agent_pet_hub_lib::types::TTSSpeechRules;
/// use agent_pet_hub_lib::tts::TTSEngine;
///
/// let rules = TTSSpeechRules::default();
/// let mut engine = TTSEngine::new(true, "say".to_string(), 1.0, rules);
/// engine.speak("Agent 正在思考");
/// ```
pub mod engine;
pub use engine::TTSEngine;
