/// Pi JSONL 文件监听器模块。
///
/// 监控 Pi Agent 的 JSONL 日志文件，检测新增行，将其转换为统一事件格式并发布到事件总线。
///
/// # 设计
///
/// 监听器使用 `notify` crate 监控文件变化，在检测到变化时读取新增内容：
///
/// 1. 记录上次读取的文件位置
/// 2. 检测文件变化事件
/// 3. 从上次位置读取新增内容
/// 4. 逐行解析 JSON
/// 5. 通过 `EventConverter` 转换为统一事件
/// 6. 发布到 `EventBus`，由状态机处理

use std::path::PathBuf;
use std::sync::Arc;

use notify::Watcher;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::event_bus::EventBus;
use crate::state_machine::PetStateMachine;
use crate::tts::TTSEngine;

use super::event_converter::EventConverter;

// ─── PiJsonlWatcher ────────────────────────────────────────────────────────

/// Pi JSONL 文件监听器。
///
/// 监控 Pi Agent 的 JSONL 日志文件，将新增行转换为统一事件并发布到事件总线。
/// 监听器在后台运行，持续检测文件变化。
///
/// # 工作流程
///
/// ```text
/// 文件变化事件
///     │
///     ▼
/// 读取新增内容（从上次位置）
///     │
///     ▼
/// 逐行解析 JSON
///     │
///     ▼
/// EventConverter 转换
///     │
///     ▼
/// EventBus 发布
///     │
///     ▼
/// PetStateMachine 处理
///     │
///     ▼
/// TTSEngine 播报（可选）
/// ```
///
/// # 线程模型
///
/// 文件读取使用 `std::fs::File`（同步），在 `tokio::task::spawn_blocking` 中运行，
/// 避免阻塞异步运行时。文件位置跟踪在结构体内维护。
///
/// # 使用示例
///
/// ```no_run
/// use agent_pet_hub_lib::adapter::pi_watcher::PiJsonlWatcher;
/// use agent_pet_hub_lib::event_bus::EventBus;
/// use agent_pet_hub_lib::state_machine::PetStateMachine;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// let event_bus = EventBus::new(1024);
/// let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));
///
/// let mut watcher = PiJsonlWatcher::new(
///     std::path::PathBuf::from("/tmp/test.jsonl"),
///     event_bus,
///     state_machine,
///     None,
/// );
///
/// // 在 tokio 运行时中启动
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # rt.block_on(async {
/// watcher.start().await.ok();
/// # });
/// ```
pub struct PiJsonlWatcher {
    /// JSONL 日志文件路径。
    file_path: PathBuf,
    /// 事件总线，用于发布转换后的事件。
    event_bus: EventBus,
    /// 宠物状态机，用于处理事件驱动的状态变更。
    state_machine: Arc<Mutex<PetStateMachine>>,
    /// TTS 引擎（可选），用于状态变更播报。
    tts_engine: Option<Arc<Mutex<TTSEngine>>>,
    /// 上次读取的文件字节位置。
    last_position: u64,
    /// 监听器是否正在运行（线程安全原子标志）。
    running: std::sync::atomic::AtomicBool,
}

impl PiJsonlWatcher {
    /// 创建新的 JSONL 文件监听器。
    ///
    /// # 参数
    ///
    /// * `file_path` — JSONL 日志文件的绝对路径
    /// * `event_bus` — 事件总线，用于发布转换后的事件
    /// * `state_machine` — 宠物状态机，用于处理事件
    /// * `tts_engine` — TTS 引擎（可选），用于状态变更语音播报
    ///
    /// # 示例
    ///
    /// ```
    /// use agent_pet_hub_lib::adapter::pi_watcher::PiJsonlWatcher;
    /// use agent_pet_hub_lib::event_bus::EventBus;
    /// use agent_pet_hub_lib::state_machine::PetStateMachine;
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    ///
    /// let event_bus = EventBus::new(1024);
    /// let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));
    ///
    /// let watcher = PiJsonlWatcher::new(
    ///     std::path::PathBuf::from("/tmp/test.jsonl"),
    ///     event_bus,
    ///     state_machine,
    ///     None,
    /// );
    /// ```
    pub fn new(
        file_path: PathBuf,
        event_bus: EventBus,
        state_machine: Arc<Mutex<PetStateMachine>>,
        tts_engine: Option<Arc<Mutex<TTSEngine>>>,
    ) -> Self {
        Self {
            file_path,
            event_bus,
            state_machine,
            tts_engine,
            last_position: 0,
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 启动文件监听。
    ///
    /// 首先等待文件创建（最多 30 秒），然后启动后台监听循环。
    /// 监听在后台 tokio 任务中运行，不会阻塞调用者。
    ///
    /// # 参数
    ///
    /// * `on_event` — 事件回调函数，每个新事件都会调用此函数。
    ///   返回 `true` 表示接受事件，`false` 表示忽略。
    ///
    /// # 返回值
    ///
    /// 返回 `JoinHandle`，可用于后续停止监听。
    /// 如果文件在超时后仍未创建，返回 `Ok(None)`。
    ///
    /// # 错误
    ///
    /// 监听循环内部错误会被记录到日志，不向上传播。
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        // 等待文件创建（最多 30 秒）
        let timeout = std::time::Duration::from_secs(30);
        let start = std::time::Instant::now();
        while !self.file_path.exists() && start.elapsed() < timeout {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        if !self.file_path.exists() {
            warn!(
                path = ?self.file_path,
                timeout_secs = timeout.as_secs(),
                "JSONL file not found after timeout"
            );
            // 不视为错误，文件可能稍后创建
            return Ok(());
        }

        // 初始化文件位置
        if let Ok(metadata) = tokio::fs::metadata(&self.file_path).await {
            self.last_position = metadata.len();
        }

        info!(
            path = ?self.file_path,
            initial_position = self.last_position,
            "Starting JSONL file watcher"
        );

        // 在后台任务中运行监听循环
        let mut watcher_clone = self.clone_for_task();
        tokio::spawn(async move {
            watcher_clone.watch_loop().await;
        });

        Ok(())
    }

    /// 停止文件监听。
    ///
    /// 将 `running` 标志设为 `false`，监听循环在下次检查时退出。
    /// 此方法是幂等的，可安全重复调用。
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        info!(path = ?self.file_path, "JSONL file watcher stopped");
    }

    /// 检查监听器是否正在运行。
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 克隆用于后台任务的状态。
    fn clone_for_task(&self) -> Self {
        Self {
            file_path: self.file_path.clone(),
            event_bus: self.event_bus.clone(),
            state_machine: self.state_machine.clone(),
            tts_engine: self.tts_engine.clone(),
            last_position: self.last_position,
            running: std::sync::atomic::AtomicBool::new(
                self.running.load(std::sync::atomic::Ordering::SeqCst),
            ),
        }
    }

    /// 后台监听循环。
    ///
    /// 持续检测文件变化，读取新增内容并处理。
    async fn watch_loop(&mut self) {
        // 使用 notify 创建文件变化监控器
        let file_path = self.file_path.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            match res {
                Ok(event) => {
                    debug!(?event, "File watch event received");
                }
                Err(e) => {
                    warn!(?e, "File watch error");
                }
            }
        })
        .expect("Failed to create file watcher");

        if let Err(e) = watcher.watch(&file_path, notify::RecursiveMode::NonRecursive) {
            warn!(path = ?file_path, error = %e, "Failed to watch file, falling back to polling");
        }

        let poll_interval = std::time::Duration::from_millis(500);

        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            if let Err(e) = self.read_new_lines().await {
                warn!(error = %e, "Error reading JSONL file");
            }
            tokio::time::sleep(poll_interval).await;
        }

        info!(path = ?self.file_path, "Watch loop exited");
    }

    /// 读取文件新增行并处理。
    ///
    /// 从 `last_position` 开始读取新增内容，逐行解析并转换为统一事件。
    /// 每次最多读取 1MB，防止超大文件膨胀导致 OOM。
    async fn read_new_lines(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 检查文件是否存在
        let metadata = match tokio::fs::metadata(&self.file_path).await {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(path = ?self.file_path, "File disappeared, skipping");
                return Ok(());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read file metadata");
                return Err(e.into());
            }
        };

        let file_size = metadata.len();

        // 文件是否缩小（被截断/轮转）
        if file_size < self.last_position {
            info!(
                old_position = self.last_position,
                new_size = file_size,
                "File was truncated or rotated, resetting position"
            );
            self.last_position = 0;
            return Ok(());
        }

        // 没有新增内容
        if file_size <= self.last_position {
            return Ok(());
        }

        // 读取新增部分（限制每次最多 1MB，防止超大文件膨胀导致 OOM）
        let max_read_bytes: u64 = 1_024 * 1_024; // 1MB
        let new_bytes = (file_size - self.last_position).min(max_read_bytes);
        if new_bytes == 0 {
            warn!(
                remaining = file_size - self.last_position,
                "File has more than 1MB new data, truncating to 1MB"
            );
        }
        let mut reader = tokio::fs::File::open(&self.file_path).await?;
        use tokio::io::AsyncSeekExt;
        reader.seek(std::io::SeekFrom::Start(self.last_position)).await?;

        // 读取所有新增内容
        use tokio::io::AsyncReadExt;
        let mut buffer = vec![0u8; new_bytes as usize];
        let bytes_read = reader.read(&mut buffer).await?;
        if bytes_read == 0 {
            return Ok(());
        }

        // 更新位置
        self.last_position = file_size;

        // 将字节转换为字符串（逐行处理，支持部分行）
        let content = String::from_utf8_lossy(&buffer).into_owned();

        // 逐行处理
        let lines: Vec<&str> = content.lines().collect();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // 解析 JSON
            let raw_event: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        line = ?line.chars().take(100).collect::<String>(),
                        error = %e,
                        "Failed to parse JSONL line"
                    );
                    continue;
                }
            };

            // 转换为统一事件
            let unified = match EventConverter::convert(&raw_event) {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "Event conversion failed");
                    continue;
                }
            };

            // 发布到事件总线
            if let Err(e) = self.event_bus.publish_event(unified.clone()) {
                warn!(error = %e, "Failed to publish event");
            }

            // 处理状态机
            {
                let mut sm = self.state_machine.lock().await;
                if let Some((old_state, new_state)) = sm.handle_event(&unified) {
                    // 发布状态变更到事件总线
                    if let Err(e) =
                        self.event_bus.publish_state_change(old_state.clone(), new_state.clone())
                    {
                        warn!(error = %e, "Failed to publish state change");
                    }

                    // TTS 播报
                    if let Some(ref tts) = self.tts_engine {
                        let mut tts_engine = tts.lock().await;
                        let _ = tts_engine.speak_state_change(&old_state, &new_state, &unified.event_type);
                    }
                }
            }

            debug!(
                event_type = ?unified.event_type,
                source = ?unified.source,
                pet_state = ?unified.pet_state,
                "Processed JSONL event"
            );
        }

        Ok(())
    }
}

impl Clone for PiJsonlWatcher {
    fn clone(&self) -> Self {
        Self {
            file_path: self.file_path.clone(),
            event_bus: self.event_bus.clone(),
            state_machine: self.state_machine.clone(),
            tts_engine: self.tts_engine.clone(),
            last_position: self.last_position,
            running: std::sync::atomic::AtomicBool::new(
                self.running.load(std::sync::atomic::Ordering::SeqCst),
            ),
        }
    }
}

// ─── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_creation() {
        let event_bus = EventBus::new(64);
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let watcher = PiJsonlWatcher::new(
            PathBuf::from("/tmp/test.jsonl"),
            event_bus,
            state_machine,
            None,
        );

        assert!(!watcher.is_running());
    }

    #[test]
    fn test_watcher_is_running_initially_false() {
        let event_bus = EventBus::new(64);
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let watcher = PiJsonlWatcher::new(
            PathBuf::from("/tmp/test.jsonl"),
            event_bus,
            state_machine,
            None,
        );

        assert!(!watcher.is_running());
    }

    #[test]
    fn test_watcher_stop() {
        let event_bus = EventBus::new(64);
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let mut watcher = PiJsonlWatcher::new(
            PathBuf::from("/tmp/test.jsonl"),
            event_bus,
            state_machine,
            None,
        );
        watcher.running.store(true, std::sync::atomic::Ordering::SeqCst);
        watcher.stop();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_watcher_clone() {
        let event_bus = EventBus::new(64);
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let watcher = PiJsonlWatcher::new(
            PathBuf::from("/tmp/test.jsonl"),
            event_bus,
            state_machine,
            None,
        );
        let cloned = watcher.clone();
        // 克隆后各自独立
        assert_eq!(cloned.file_path, watcher.file_path);
    }

    #[tokio::test]
    async fn test_watcher_read_nonexistent_file() {
        let event_bus = EventBus::new(64);
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let mut watcher = PiJsonlWatcher::new(
            PathBuf::from("/tmp/nonexistent_test_file_xyz.jsonl"),
            event_bus,
            state_machine,
            None,
        );
        watcher.last_position = 0;

        // 读取不存在的文件应返回 Ok（文件消失处理）
        let result = watcher.read_new_lines().await;
        assert!(result.is_ok(), "读取不存在的文件应返回 Ok");
    }

    #[tokio::test]
    async fn test_watcher_read_empty_file() {
        let event_bus = EventBus::new(64);
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let dir = std::env::temp_dir();
        let file_path = dir.join("test_watcher_empty.jsonl");

        // 创建空文件
        std::fs::File::create(&file_path).expect("Failed to create temp file");

        let cleanup_path = file_path.clone();
        let mut watcher = PiJsonlWatcher::new(
            file_path,
            event_bus,
            state_machine,
            None,
        );
        watcher.last_position = 0;

        let result = watcher.read_new_lines().await;
        assert!(result.is_ok());
        assert_eq!(watcher.last_position, 0);

        // 清理
        let _ = std::fs::remove_file(&cleanup_path);
    }

    #[tokio::test]
    async fn test_watcher_read_file_with_content() {
        let event_bus = EventBus::new(64);
        let _state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let dir = std::env::temp_dir();
        let file_path = dir.join("test_watcher_with_content.jsonl");

        // 创建带内容的文件
        let content = r#"{"type": "session_start", "prompt": "Hello"}
{"type": "turn_end"}
"#;
        std::fs::write(&file_path, content).expect("Failed to write temp file");

        let file_size = std::fs::metadata(&file_path).unwrap().len();

        let cleanup_path = file_path.clone();
        let mut watcher = PiJsonlWatcher::new(
            file_path,
            event_bus,
            _state_machine,
            None,
        );
        watcher.last_position = 0;

        let result = watcher.read_new_lines().await;
        assert!(result.is_ok());
        assert_eq!(watcher.last_position, file_size);

        // 清理
        let _ = std::fs::remove_file(&cleanup_path);
    }

    #[tokio::test]
    async fn test_watcher_detect_file_truncation() {
        let event_bus = EventBus::new(64);
        let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));

        let dir = std::env::temp_dir();
        let file_path = dir.join("test_watcher_truncate.jsonl");

        // 先写一个大内容
        let content = "x".repeat(1000);
        std::fs::write(&file_path, &content).expect("Failed to write temp file");

        let cleanup_path = file_path.clone();
        let mut watcher = PiJsonlWatcher::new(
            file_path,
            event_bus,
            state_machine,
            None,
        );
        watcher.last_position = 1000;

        // 缩小文件
        std::fs::write(&cleanup_path, "small").expect("Failed to truncate temp file");

        watcher.read_new_lines().await.unwrap();
        assert_eq!(watcher.last_position, 0, "文件缩小后应重置位置");

        // 清理
        let _ = std::fs::remove_file(&cleanup_path);
    }
}
