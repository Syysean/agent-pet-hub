# 第四阶段：MVP 设计

> 日期：2026-06-08
> 状态：已确认

---

## 一、MVP 目标

构建一个**最小可运行闭环**：

```
Agent → Event → State Machine → Pet Animation
```

用户能看到的：
1. 桌宠悬浮窗在桌面上
2. 桌宠能响应 Agent 状态变化
3. 至少一种 Agent（Pi）能正确驱动桌宠动画

### MVP 成功标准（Definition of Done）

- [ ] Tauri 项目可编译运行
- [ ] 桌宠悬浮窗可显示、可拖拽、可关闭
- [ ] 系统托盘图标正常显示
- [ ] Pi Agent 通过 Extension 输出事件
- [ ] Rust 端能读取 Pi 事件并转换
- [ ] 状态机正确响应 Pi 事件
- [ ] 桌宠动画随状态正确切换
- [ ] WebSocket 服务器可被外部进程连接
- [ ] 基础 TTS 语音播报功能
- [ ] 基础插件系统可加载皮肤插件

---

## 二、MVP 功能范围

### ✅ MVP 包含

| 模块 | MVP 功能 |
|------|----------|
| **Tauri 后端** | Event Bus + State Machine + 基础 Adapter + WebSocket Server |
| **Pi 适配器** | JSONL 文件监听 + 事件转换 + 状态映射 |
| **桌宠渲染** | CSS/SVG 动画 + 基础状态动画（6 种） |
| **窗口管理** | 悬浮窗置顶 + 拖拽 + 鼠标穿透 + 边界吸附 |
| **系统托盘** | 托盘图标 + 菜单 + 状态指示 |
| **事件协议** | 完整事件 Schema + 验证 |
| **WebSocket** | 服务端 + 订阅 + 推送 |
| **TTS** | macOS `say` / Linux `espeak` 基础播报 |
| **插件** | 基础插件管理器 + 皮肤插件接口 |
| **配置** | 设置读写（JSON 文件） |

### ❌ MVP 不包含

| 功能 | 后续版本 |
|------|----------|
| Hermes 适配器 | Phase 4 |
| OpenClaw 适配器 | Phase 4 |
| Live2D 动画 | 后期扩展 |
| VRM 3D 宠物 | 后期扩展 |
| 对话交互（聊天面板） | 后期扩展 |
| 高级语音（Edge-TTS） | 后期扩展 |
| 多 Agent 同时运行 | 后期扩展 |
| 设置面板 GUI | 后期扩展 |

---

## 三、MVP 状态机设计

### 3.1 简化状态机

MVP 只需要处理最核心的状态转换：

```rust
// 状态机核心
pub struct PetStateMachine {
    current_state: PetState,
    transition_table: HashMap<(PetState, EventType), PetState>,
}

impl PetStateMachine {
    /// 处理事件，返回新的状态（可能不变）
    pub fn handle_event(&mut self, event: &UnifiedAgentEvent) -> Option<PetState> {
        let new_state = self.transition_table
            .get(&(self.current_state, event.event_type))
            .copied();
        
        if let Some(new_state) = new_state {
            if new_state != self.current_state {
                let old = self.current_state.clone();
                self.current_state = new_state;
                Some((old, new_state))
            } else {
                None
            }
        } else {
            None
        }
    }
}
```

### 3.2 状态转换表（MVP 精简版）

```rust
/// 状态转换表：MVP 只需要定义核心路径
/// 未定义转换保持当前状态不变
fn build_transition_table() -> HashMap<(PetState, EventType), PetState> {
    let mut table = HashMap::new();
    
    // ─── 初始化 ───
    table.insert((PetState::Connecting, EventType::AdapterConnected), PetState::Idle);
    
    // ─── 进入工作状态 ───
    table.insert((PetState::Idle, EventType::SessionStart), PetState::Thinking);
    table.insert((PetState::Idle, EventType::UserPrompt), PetState::Thinking);
    table.insert((PetState::Thinking, EventType::ToolCallStart), PetState::Working);
    
    // ─── 工作→思考 ───
    table.insert((PetState::Working, EventType::ToolCallEnd), PetState::Thinking);
    table.insert((PetState::Working, EventType::ThinkingEnd), PetState::Thinking);
    
    // ─── 等待→思考 ───
    table.insert((PetState::Waiting, EventType::PermissionGranted), PetState::Thinking);
    table.insert((PetState::Waiting, EventType::PermissionDenied), PetState::Thinking);
    
    // ─── 思考→空闲 ───
    table.insert((PetState::Thinking, EventType::SessionEnd), PetState::Idle);
    table.insert((PetState::Thinking, EventType::UserCancel), PetState::Idle);
    
    // ─── 错误 ───
    table.insert((PetState::Working, EventType::ToolCallError), PetState::Error);
    table.insert((PetState::Thinking, EventType::ToolCallError), PetState::Error);
    
    // ─── 错误恢复 ───
    table.insert((PetState::Error, EventType::SessionEnd), PetState::Idle);
    table.insert((PetState::Error, EventType::UserCancel), PetState::Idle);
    
    // ─── 系统事件 ───
    table.insert((PetState::Connecting, EventType::AdapterDisconnected), PetState::Idle);
    
    table
}
```

### 3.3 状态转换流程图

```
Connecting ──AdapterConnected──→ Idle
                                      │
                                      ├─UserPrompt / SessionStart──→ Thinking
                                      │                              │
                                      │                              ├─ToolCallStart──→ Working
                                      │                              │                 │
                                      │                              │                 ├─ToolCallEnd──→ Thinking
                                      │                              │                 │
                                      │                              │                 ├─ToolCallError──→ Error
                                      │                              │                 │
                                      │                              │                 └─ToolCallStart──→ Working
                                      │                              │
                                      │                              ├─SessionEnd──→ Idle
                                      │                              ├─UserCancel──→ Idle
                                      │                              └─ToolCallError──→ Error
                                      │
Idle ──SessionEnd / UserCancel─────→ Idle (self-loop)

Thinking ──SessionEnd / UserCancel──→ Idle
Thinking ──ToolCallError───────────→ Error

Working ──ToolCallEnd──────────────→ Thinking
Working ──ToolCallError────────────→ Error

Waiting ──PermissionGranted────────→ Thinking
Waiting ──PermissionDenied─────────→ Thinking

Error ──SessionEnd / UserCancel────→ Idle

Idle ──UserCancel / SessionEnd────→ Idle (self-loop, 防御性)
```

### 3.4 防抖动设计

防止频繁状态切换导致动画闪烁：

```rust
/// 状态防抖动配置
pub struct StateDebouncer {
    /// 最小状态保持时间（毫秒）
    min_hold_duration: Duration,
    /// 最近状态变更时间
    last_state_change: Option<Instant>,
    /// 当前状态
    current_state: PetState,
}

impl StateDebouncer {
    pub fn should_change_state(&mut self, new_state: PetState) -> bool {
        if new_state == self.current_state {
            return false;
        }
        
        match self.last_state_change {
            Some(last_change) => {
                let elapsed = last_change.elapsed();
                // 如果距上次变更时间太短，跳过
                elapsed >= self.min_hold_duration
            }
            None => true, // 首次变更
        }
    }
    
    pub fn record_state_change(&mut self, state: PetState) {
        self.last_state_change = Some(Instant::now());
        self.current_state = state;
    }
}

/// MVP 默认防抖时间：500ms
/// 原因：工具调用可能非常频繁（如批量读取多个文件），
///       需要合并为一次"工作中"状态，避免动画闪烁
impl Default for StateDebouncer {
    fn default() -> Self {
        Self {
            min_hold_duration: Duration::from_millis(500),
            last_state_change: None,
            current_state: PetState::Connecting,
        }
    }
}
```

### 3.5 状态到动画的映射

```rust
/// 状态到动画资源的映射
/// MVP 只定义核心状态，使用 CSS/SVG 实现
pub fn get_animation_for_state(state: &PetState) -> AnimationDefinition {
    match state {
        PetState::Idle => AnimationDefinition {
            name: "idle",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 呼吸动画：轻微缩放
                Keyframe::new(0.0, Transform::scale(1.0)),
                Keyframe::new(0.5, Transform::scale(1.02)),
                Keyframe::new(1.0, Transform::scale(1.0)),
            ],
            duration: Duration::from_secs(3), // 3 秒循环
            loop_: true,
        },
        
        PetState::Thinking => AnimationDefinition {
            name: "thinking",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 歪头 + 眨眼
                Keyframe::new(0.0, Transform::rotate(0.0)),
                Keyframe::new(0.3, Transform::rotate(5.0)),
                Keyframe::new(0.6, Transform::rotate(0.0)),
                Keyframe::new(0.8, Transform::rotate(-5.0)),
                Keyframe::new(1.0, Transform::rotate(0.0)),
            ],
            duration: Duration::from_secs(2),
            loop_: true,
        },
        
        PetState::Working => AnimationDefinition {
            name: "working",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 快速震动（敲代码效果）
                Keyframe::new(0.0, Transform::translate(0.0, 0.0)),
                Keyframe::new(0.1, Transform::translate(-1.0, 0.0)),
                Keyframe::new(0.2, Transform::translate(1.0, 0.0)),
                Keyframe::new(0.3, Transform::translate(-1.0, 0.0)),
                Keyframe::new(0.4, Transform::translate(0.0, 0.0)),
            ],
            duration: Duration::from_millis(200), // 快速
            loop_: true,
        },
        
        PetState::Waiting => AnimationDefinition {
            name: "waiting",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 缓慢摆动
                Keyframe::new(0.0, Transform::rotate(-3.0)),
                Keyframe::new(0.5, Transform::rotate(0.0)),
                Keyframe::new(1.0, Transform::rotate(3.0)),
            ],
            duration: Duration::from_secs(4),
            loop_: true,
        },
        
        PetState::Success => AnimationDefinition {
            name: "success",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 庆祝：弹跳 + 缩放
                Keyframe::new(0.0, Transform::scale(1.0)),
                Keyframe::new(0.2, Transform::scale(1.2).translate(0.0, -10.0)),
                Keyframe::new(0.4, Transform::scale(1.0)),
                Keyframe::new(0.6, Transform::scale(1.2).translate(0.0, -10.0)),
                Keyframe::new(1.0, Transform::scale(1.0)),
            ],
            duration: Duration::from_secs(1),
            loop_: false, // 只播放一次
        },
        
        PetState::Error => AnimationDefinition {
            name: "error",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 摇头 + 变红
                Keyframe::new(0.0, Transform::rotate(-8.0)),
                Keyframe::new(0.25, Transform::rotate(8.0)),
                Keyframe::new(0.5, Transform::rotate(-8.0)),
                Keyframe::new(0.75, Transform::rotate(8.0)),
                Keyframe::new(1.0, Transform::rotate(0.0)),
            ],
            duration: Duration::from_millis(800),
            loop_: true,
        },
        
        PetState::Speaking => AnimationDefinition {
            name: "speaking",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 说话：嘴部张合（简化为上下移动）
                Keyframe::new(0.0, Transform::translate(0.0, 0.0)),
                Keyframe::new(0.3, Transform::translate(0.0, 2.0)),
                Keyframe::new(0.6, Transform::translate(0.0, 0.0)),
            ],
            duration: Duration::from_millis(150),
            loop_: true,
        },
        
        PetState::Connecting => AnimationDefinition {
            name: "connecting",
            type_: AnimationType::CSS,
            keyframes: vec![
                // 旋转加载
                Keyframe::new(0.0, Transform::rotate(0.0)),
                Keyframe::new(1.0, Transform::rotate(360.0)),
            ],
            duration: Duration::from_secs(1),
            loop_: true,
        },
    }
}
```

---

## 四、MVP 文件结构（精简版）

```
agent-pet-hub/
├── src-tauri/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                 # Tauri 入口
│   │   │
│   │   ├── state_machine/          # 状态机
│   │   │   ├── mod.rs
│   │   │   ├── machine.rs          # 状态机核心 + 防抖
│   │   │   └── transitions.rs      # 转换表
│   │   │
│   │   ├── event_bus/              # 事件总线
│   │   │   ├── mod.rs
│   │   │   └── bus.rs              # tokio::broadcast
│   │   │
│   │   ├── adapter/                # 适配器层（MVP 仅 Pi）
│   │   │   ├── mod.rs
│   │   │   ├── trait.rs            # AgentAdapter trait
│   │   │   ├── pi_adapter.rs       # Pi 适配器（核心）
│   │   │   ├── pi_watcher.rs       # JSONL 文件 watcher
│   │   │   └── event_converter.rs  # 事件转换
│   │   │
│   │   ├── ipc/                    # IPC 层
│   │   │   ├── mod.rs
│   │   │   └── ws_server.rs        # WebSocket 服务器
│   │   │
│   │   ├── window/                 # 窗口管理
│   │   │   ├── mod.rs
│   │   │   ├── pet_window.rs       # 悬浮窗
│   │   │   └── tray.rs             # 托盘
│   │   │
│   │   ├── tts/                    # TTS（基础）
│   │   │   ├── mod.rs
│   │   │   └── engine.rs           # 跨平台 TTS
│   │   │
│   │   ├── plugin/                 # 插件系统（基础）
│   │   │   ├── mod.rs
│   │   │   └── manager.rs          # 插件管理器
│   │   │
│   │   ├── config/                 # 配置
│   │   │   ├── mod.rs
│   │   │   └── settings.rs         # JSON 配置读写
│   │   │
│   │   └── lib.rs                  # 库入口
│   │
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   └── resources/
│       └── skins/
│           └── default/            # 默认皮肤
│               ├── index.json      # 皮肤元数据
│               ├── pet.svg         # 宠物 SVG
│               └── css/
│                   ├── idle.css
│                   ├── thinking.css
│                   ├── working.css
│                   ├── waiting.css
│                   ├── success.css
│                   └── error.css
│
├── src/                            # Web 前端
│   ├── main.tsx                    # React 入口
│   ├── App.tsx                     # 根组件
│   │
│   ├── components/
│   │   ├── PetWindow.tsx           # 桌宠悬浮窗（核心）
│   │   ├── PetSVG.tsx              # SVG 渲染组件
│   │   ├── PetAnimation.tsx        # 动画控制组件
│   │   └── PetStatus.tsx           # 状态文字显示
│   │
│   ├── hooks/
│   │   ├── useAgentState.ts        # Agent 状态 Hook（核心）
│   │   ├── usePetAnimation.ts      # 动画 Hook
│   │   └── useTauri.ts             # Tauri invoke Hook
│   │
│   ├── store/
│   │   ├── petStore.ts             # 宠物状态管理
│   │   └── settingsStore.ts        # 设置管理
│   │
│   ├── services/
│   │   └── wsClient.ts             # WebSocket 客户端
│   │
│   ├── types/
│   │   ├── events.ts               # 事件类型
│   │   └── pet.ts                  # 宠物类型
│   │
│   └── assets/
│       ├── skins/
│       └── animations/
│
├── packages/
│   └── protocol/                   # 协议包（复用 Phase 3）
│       ├── src/
│       │   ├── events.ts           # 类型定义
│       │   ├── schemas.ts          # Zod schemas
│       │   └── index.ts
│       └── tests/
│
├── skills/
│   └── pi/
│       └── pet-event-logger.ts     # Pi 事件采集扩展
│
└── docs/
    └── phase4-mvp.md
```

---

## 五、核心模块 MVP 实现

### 5.1 Pi 适配器（核心中的核心）

```rust
/// Pi 适配器：通过 JSONL 文件监听 Pi Agent 事件
pub struct PiAdapter {
    config: PiAdapterConfig,
    event_bus: EventBus,
    state_machine: Arc<Mutex<PetStateMachine>>,
    tts_engine: Option<Arc<TTSEngine>>,
    watcher_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiAdapterConfig {
    /// Pi 日志目录（通常 ~/.pi/logs/）
    pub log_dir: PathBuf,
    /// 事件文件名称
    pub event_file: String,
    /// 是否启用 TTS
    pub enable_tts: bool,
    /// 是否启用 WebSocket 推送
    pub enable_ws_push: bool,
}

impl PiAdapter {
    pub async fn new(config: PiAdapterConfig, event_bus: EventBus) -> Self {
        Self {
            config,
            event_bus,
            state_machine: Arc::new(Mutex::new(PetStateMachine::new())),
            tts_engine: if config.enable_tts {
                Some(Arc::new(TTSEngine::new()))
            } else {
                None
            },
            watcher_handle: None,
        }
    }
    
    /// 启动事件监听
    pub async fn start(&mut self) -> Result<(), AdapterError> {
        let log_dir = self.config.log_dir.clone();
        let event_file = self.config.event_file.clone();
        let event_bus = self.event_bus.clone();
        let state_machine = Arc::clone(&self.state_machine);
        let tts_engine = self.tts_engine.clone();
        
        self.watcher_handle = Some(tokio::spawn(async move {
            // 等待事件文件创建
            let file_path = log_dir.join(&event_file);
            loop {
                if file_path.exists() {
                    // 开始监听文件尾部
                    Self::watch_jsonl_file(&file_path, event_bus, state_machine, tts_engine).await;
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }));
        
        Ok(())
    }
    
    /// 监听 JSONL 文件
    async fn watch_jsonl_file(
        file_path: &Path,
        event_bus: EventBus,
        state_machine: Arc<Mutex<PetStateMachine>>,
        tts_engine: Option<Arc<TTSEngine>>,
    ) {
        // 使用 inotify/fsevents 监控文件变化
        let mut watcher = FileWatcher::new(file_path).await;
        
        while let Some(new_lines) = watcher.await_lines().await {
            for line in new_lines {
                // 解析 JSON
                let raw_event: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Pi event parse error: {}", e);
                        continue;
                    }
                };
                
                // 转换为统一事件
                let unified = match EventConverter::convert(&raw_event) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Pi event convert error: {}", e);
                        continue;
                    }
                };
                
                // 发布到事件总线
                event_bus.publish(unified.clone()).await;
                
                // 更新状态机
                let mut sm = state_machine.lock().await;
                if let Some((old_state, new_state)) = sm.handle_event(&unified) {
                    // 状态变更通知
                    event_bus.publish_state_change(old_state, new_state.clone()).await;
                    
                    // TTS 播报（如果需要）
                    if let Some(tts) = &tts_engine {
                        tts.speak_state_change(&new_state).await;
                    }
                }
            }
        }
    }
}
```

### 5.2 事件转换器

```rust
/// 将 Pi 原生事件转换为统一事件
pub struct EventConverter;

impl EventConverter {
    pub fn convert(raw: &serde_json::Value) -> Result<UnifiedAgentEvent, ConverterError> {
        let event_type = raw.get("type")
            .and_then(|v| v.as_str())
            .ok_or(ConverterError::MissingType)?;
        
        let timestamp = chrono::Utc::now().to_rfc3339();
        let id = ulid::Ulid::new().to_string();
        
        match event_type {
            "session_start" => Ok(UnifiedAgentEvent {
                id,
                timestamp,
                version: "1.0".to_string(),
                source: AgentSource::Pi,
                category: EventCategory::Session,
                event_type: EventType::SessionStart,
                pet_state: PetState::Thinking,
                session_id: Self::extract_session_id(raw),
                tool_name: None,
                tool_args_preview: None,
                tool_result_preview: None,
                task_preview: Self::extract_prompt(raw),
                raw: raw.clone(),
                metadata: None,
            }),
            
            "tool_call" => Ok(UnifiedAgentEvent {
                id,
                timestamp,
                version: "1.0".to_string(),
                source: AgentSource::Pi,
                category: EventCategory::Tool,
                event_type: EventType::ToolCallStart,
                pet_state: PetState::Working,
                session_id: Self::extract_session_id(raw),
                tool_name: Self::extract_tool_name(raw),
                tool_args_preview: Self::extract_tool_args(raw),
                raw: raw.clone(),
                metadata: None,
            }),
            
            "tool_result" => Ok(UnifiedAgentEvent {
                id,
                timestamp,
                version: "1.0".to_string(),
                source: AgentSource::Pi,
                category: EventCategory::Tool,
                event_type: EventType::ToolCallEnd,
                pet_state: PetState::Thinking,
                session_id: Self::extract_session_id(raw),
                tool_name: Self::extract_tool_name(raw),
                tool_result_preview: Self::extract_result_preview(raw),
                raw: raw.clone(),
                metadata: None,
            }),
            
            "tool_error" => Ok(UnifiedAgentEvent {
                id,
                timestamp,
                version: "1.0".to_string(),
                source: AgentSource::Pi,
                category: EventCategory::Error,
                event_type: EventType::ToolCallError,
                pet_state: PetState::Error,
                session_id: Self::extract_session_id(raw),
                tool_name: Self::extract_tool_name(raw),
                error_code: Some(ErrorCode::ToolTimeout), // 简化
                error_message: Self::extract_error_message(raw),
                raw: raw.clone(),
                metadata: None,
            }),
            
            "turn_end" => Ok(UnifiedAgentEvent {
                id,
                timestamp,
                version: "1.0".to_string(),
                source: AgentSource::Pi,
                category: EventCategory::Session,
                event_type: EventType::SessionEnd,
                pet_state: PetState::Idle,
                session_id: Self::extract_session_id(raw),
                raw: raw.clone(),
                metadata: None,
            }),
            
            "user_prompt" => Ok(UnifiedAgentEvent {
                id,
                timestamp,
                version: "1.0".to_string(),
                source: AgentSource::Pi,
                category: EventCategory::User,
                event_type: EventType::UserPrompt,
                pet_state: PetState::Thinking,
                session_id: Self::extract_session_id(raw),
                task_preview: Self::extract_prompt(raw),
                raw: raw.clone(),
                metadata: None,
            }),
            
            // 其他事件类型，默认映射到 thinking
            _ => Ok(UnifiedAgentEvent {
                id,
                timestamp,
                version: "1.0".to_string(),
                source: AgentSource::Pi,
                category: EventCategory::Thinking,
                event_type: EventType::ThinkingTick,
                pet_state: PetState::Thinking,
                session_id: Self::extract_session_id(raw),
                raw: raw.clone(),
                metadata: Some(json!({ "raw_type": event_type })),
            }),
        }
    }
    
    // 辅助提取函数
    fn extract_tool_name(raw: &serde_json::Value) -> Option<String> {
        raw.get("tool")
            .or_else(|| raw.get("method"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    
    fn extract_tool_args(raw: &serde_json::Value) -> Option<String> {
        raw.get("args")
            .or_else(|| raw.get("params"))
            .and_then(|v| v.as_str())
            .map(|s| Self::truncate(s, 200))
    }
    
    fn extract_result_preview(raw: &serde_json::Value) -> Option<String> {
        raw.get("result")
            .or_else(|| raw.get("output"))
            .and_then(|v| v.as_str())
            .map(|s| Self::truncate(s, 500))
    }
    
    fn extract_prompt(raw: &serde_json::Value) -> Option<String> {
        raw.get("prompt")
            .or_else(|| raw.get("message"))
            .or_else(|| raw.get("text"))
            .and_then(|v| v.as_str())
            .map(|s| Self::truncate(s, 300))
    }
    
    fn extract_error_message(raw: &serde_json::Value) -> Option<String> {
        raw.get("error")
            .and_then(|v| v.as_str())
            .map(|s| Self::truncate(s, 500))
    }
    
    fn extract_session_id(raw: &serde_json::Value) -> Option<String> {
        raw.get("sessionId")
            .or_else(|| raw.get("session_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
}
```

### 5.3 WebSocket 服务器（MVP）

```rust
use tokio_tungstenite::{WebSocketStream, accept_async};
use tokio_tungstenite::tungstenite::Message;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct WSServer {
    /// 所有已认证的客户端
    clients: Arc<Mutex<HashMap<String, WebSocketStream<tokio::net::TcpStream>>>>,
    /// 事件订阅者
    event_rx: tokio::sync::broadcast::Receiver<UnifiedAgentEvent>,
    /// 端口
    port: u16,
    /// 认证 token
    auth_token: String,
}

impl WSServer {
    pub async fn new(port: u16, auth_token: String) -> Result<Self, WSServerError> {
        let (event_tx, event_rx) = tokio::sync::broadcast::channel::<UnifiedAgentEvent>(1000);
        
        // 从全局事件总线注册
        EventBus::subscribe_to_events(event_tx);
        
        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            event_rx,
            port,
            auth_token,
        })
    }
    
    pub async fn run(&self) -> Result<(), WSServerError> {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", self.port))
            .await
            .map_err(|e| WSServerError::BindFailed(e))?;
        
        println!("WS Server listening on 127.0.0.1:{}", self.port);
        
        loop {
            let (stream, addr) = listener.accept().await?;
            let clients = Arc::clone(&self.clients);
            let token = self.auth_token.clone();
            let mut event_rx = self.event_rx.resubscribe();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(stream, clients, token, event_rx).await {
                    eprintln!("WS client error (addr {}): {}", addr, e);
                }
            });
        }
    }
    
    async fn handle_client(
        stream: tokio::net::TcpStream,
        clients: Arc<Mutex<HashMap<String, WebSocketStream<tokio::net::TcpStream>>>>,
        token: String,
        mut event_rx: tokio::sync::broadcast::Receiver<UnifiedAgentEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = accept_async(stream).await?;
        let (ws_tx, mut ws_rx) = ws_stream.split();
        
        // 客户端 ID
        let client_id = uuid::Uuid::new_v4().to_string();
        
        // 注册客户端
        {
            let mut clients = clients.lock().await;
            clients.insert(client_id.clone(), ws_tx.clone());
        }
        
        // 消息循环
        loop {
            tokio::select! {
                // 接收客户端消息
                msg = ws_rx.recv() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            let ws_msg: WSMessage = serde_json::from_str(&text)?;
                            match ws_msg.r#type {
                                WSMessageType::Auth => {
                                    let payload: WSAuthPayload = ws_msg.payload;
                                    if payload.token == token {
                                        Self::send_message(&mut ws_tx, WSMessage {
                                            r#type: WSMessageType::AuthAck,
                                            id: ws_msg.id,
                                            timestamp: chrono::Utc::now().to_rfc3339(),
                                            payload: serde_json::json!({ "authorized": true }).into(),
                                        }).await?;
                                    } else {
                                        Self::send_message(&mut ws_tx, WSMessage {
                                            r#type: WSMessageType::AuthAck,
                                            id: ws_msg.id,
                                            timestamp: chrono::Utc::now().to_rfc3339(),
                                            payload: serde_json::json!({ "authorized": false }).into(),
                                        }).await?;
                                    }
                                }
                                WSMessageType::Ping => {
                                    Self::send_message(&mut ws_tx, WSMessage {
                                        r#type: WSMessageType::Pong,
                                        id: ws_msg.id,
                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                        payload: serde_json::json!({}).into(),
                                    }).await?;
                                }
                                _ => {}
                            }
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
                
                // 推送事件到客户端
                event = event_rx.recv() => {
                    match event {
                        Ok(e) => {
                            let ws_msg = WSMessage {
                                r#type: WSMessageType::Event,
                                id: None,
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                payload: serde_json::json!({
                                    "event": e,
                                }).into(),
                            };
                            if let Err(_) = Self::send_message(&mut ws_tx, ws_msg).await {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("Client lagged behind by {} events", n);
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        
        // 清理客户端
        {
            let mut clients = clients.lock().await;
            clients.remove(&client_id);
        }
        
        Ok(())
    }
    
    async fn send_message(
        ws_tx: &mut tokio_tungstenite::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>,
        msg: WSMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let text = serde_json::to_string(&msg)?;
        ws_tx.send(Message::Text(text)).await?;
        Ok(())
    }
}
```

### 5.4 前端核心组件

```tsx
// src/components/PetSVG.tsx - 核心渲染组件
import React, { useEffect, useRef, useState } from 'react';
import { useAgentState } from '../hooks/useAgentState';
import type { PetState } from '@agent-pet-hub/protocol';

interface PetSVGProps {
  skinName: string;
}

export const PetSVG: React.FC<PetSVGProps> = ({ skinName }) => {
  const { petState, previousState } = useAgentState();
  const [animClass, setAnimClass] = useState('pet-idle');
  const prevRef = useRef<PetState>(petState);
  
  useEffect(() => {
    // 状态变更时更新动画类名
    if (prevRef.current !== petState) {
      prevRef.current = petState;
      
      // 添加入场动画
      const petEl = document.getElementById('pet-svg');
      if (petEl) {
        petEl.classList.add('pet-transitioning');
        setTimeout(() => {
          petEl.classList.remove('pet-transitioning');
        }, 300);
      }
      
      setAnimClass(`pet-${petState}`);
    }
  }, [petState]);
  
  return (
    <div 
      className={`pet-container ${animClass}`}
      style={{
        cursor: 'grab',
        userSelect: 'none',
      }}
    >
      <svg 
        id="pet-svg"
        className="pet-svg"
        viewBox="0 0 120 120"
        xmlns="http://www.w3.org/2000/svg"
      >
        {/* 皮肤定义 */}
        <g className={`pet-body pet-${skinName}`}>
          {/* 身体 */}
          <ellipse cx="60" cy="70" rx="35" ry="30" fill="#FFB6C1" />
          {/* 头 */}
          <circle cx="60" cy="40" r="25" fill="#FFB6C1" />
          {/* 眼睛 */}
          <g className="pet-eyes">
            <ellipse cx="50" cy="38" rx="3" ry="4" fill="#333">
              {petState === 'idle' && (
                <animate attributeName="ry" values="4;0;4" dur="3s" repeatCount="indefinite" />
              )}
              {petState === 'thinking' && (
                <ellipse cx="50" cy="38" rx="3" ry="4" fill="#333" />
              )}
            </ellipse>
            <ellipse cx="70" cy="38" rx="3" ry="4" fill="#333">
              {petState === 'idle' && (
                <animate attributeName="ry" values="4;0;4" dur="3s" repeatCount="indefinite" />
              )}
              {petState === 'thinking' && (
                <ellipse cx="70" cy="38" rx="3" ry="4" fill="#333" />
              )}
            </ellipse>
          </g>
          {/* 嘴巴 */}
          <path 
            className="pet-mouth"
            d={petState === 'success' 
              ? 'M 52 48 Q 60 55 68 48'  // 微笑
              : petState === 'error'
              ? 'M 52 52 Q 60 45 68 52'  // 难过
              : 'M 55 50 Q 60 52 65 50'  // 正常
            }
            stroke="#333" strokeWidth="2" fill="none"
          />
          {/* 腮红（thinking 状态增强） */}
          {petState === 'thinking' && (
            <g className="blush">
              <ellipse cx="42" cy="45" rx="5" ry="3" fill="#FF6B6B" opacity="0.4" />
              <ellipse cx="78" cy="45" rx="5" ry="3" fill="#FF6B6B" opacity="0.4" />
            </g>
          )}
        </g>
        
        {/* 状态指示器 */}
        {petState === 'working' && (
          <g className="working-indicator">
            <circle cx="60" cy="10" r="3" fill="#4CAF50">
              <animate attributeName="opacity" values="1;0;1" dur="0.5s" repeatCount="indefinite" />
            </circle>
          </g>
        )}
        
        {petState === 'error' && (
          <g className="error-indicator">
            <text x="60" y="10" textAnchor="middle" fill="#F44336" fontSize="12">✕</text>
          </g>
        )}
      </svg>
    </div>
  );
};
```

```tsx
// src/hooks/useAgentState.ts - 核心状态 Hook
import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PetState, UnifiedAgentEvent } from '@agent-pet-hub/protocol';

export function useAgentState() {
  const [petState, setPetState] = useState<PetState>('connecting');
  const [previousState, setPreviousState] = useState<PetState | null>(null);
  const [currentAgent, setCurrentAgent] = useState<string | null>(null);
  
  // 从 Tauri 后端获取初始状态
  useEffect(() => {
    const init = async () => {
      try {
        const state = await invoke<PetState>('get_pet_state');
        setPetState(state);
      } catch (err) {
        console.error('Failed to get pet state:', err);
      }
    };
    init();
  }, []);
  
  // 监听 Tauri 事件
  useEffect(() => {
    const unsubscribe = window.addEventListener('tauri://event', (event: any) => {
      if (event.payload.event === 'pet:state_changed') {
        const { newState, previousState } = event.payload.data;
        setPreviousState(previousState);
        setPetState(newState);
      }
      
      if (event.payload.event === 'pet:event') {
        const unifiedEvent: UnifiedAgentEvent = event.payload.data.event;
        console.log('Agent event:', unifiedEvent.type, unifiedEvent.petState);
      }
      
      if (event.payload.event === 'pet:agent_info') {
        const agents = event.payload.data.agents;
        const active = agents.find((a: any) => a.online && a.active);
        if (active) {
          setCurrentAgent(active.source);
        }
      }
    });
    
    return unsubscribe;
  }, []);
  
  // WebSocket 连接（可选，用于实时事件）
  useEffect(() => {
    // 可选：连接 WebSocket 获取实时事件
    // const ws = new WebSocket('ws://127.0.0.1:8765');
    // ws.onmessage = (e) => { ... };
  }, []);
  
  return {
    petState,
    previousState,
    currentAgent,
  };
}
```

---

## 六、MVP 开发优先级与工作量估算

### 6.1 开发顺序（必须严格遵循）

| 优先级 | 任务 | 模块 | 预计工作量 | 依赖 |
|--------|------|------|-----------|------|
| **P0** | 1 | Tauri 项目初始化 | 2h | 无 |
| **P0** | 2 | 协议包（events.ts） | 3h | 无 |
| **P0** | 3 | 状态机核心（machine.rs） | 4h | P0-2 |
| **P0** | 4 | 事件总线（bus.rs） | 3h | P0-1 |
| **P1** | 5 | 状态转换表（transitions.rs） | 2h | P0-3 |
| **P1** | 6 | 事件转换器（event_converter.rs） | 4h | P0-2 |
| **P1** | 7 | Pi 适配器核心（pi_adapter.rs） | 4h | P0-4, P1-6 |
| **P1** | 8 | JSONL 文件监听（pi_watcher.rs） | 3h | P1-7 |
| **P2** | 9 | 悬浮窗组件（PetWindow.tsx） | 4h | P0-1 |
| **P2** | 10 | SVG 宠物渲染（PetSVG.tsx） | 4h | P2-9 |
| **P2** | 11 | 动画 CSS（6 种状态） | 4h | P2-9 |
| **P2** | 12 | 状态 Hook（useAgentState.ts） | 3h | P0-4, P2-9 |
| **P2** | 13 | 托盘图标与菜单（tray.rs） | 3h | P0-1 |
| **P3** | 14 | WebSocket 服务器（ws_server.rs） | 4h | P0-4 |
| **P3** | 15 | TTS 基础引擎（engine.rs） | 3h | 无 |
| **P3** | 16 | 配置读写（settings.rs） | 2h | 无 |
| **P3** | 17 | 基础插件管理器（manager.rs） | 3h | 无 |
| **P4** | 18 | Pi 事件采集扩展（pet-event-logger.ts） | 3h | P0-2 |
| **P4** | 19 | 集成测试与端到端验证 | 4h | 所有 |
| **P4** | 20 | 文档与 README | 2h | 所有 |

**总工作量估算**：约 **58 小时**（约 1.5-2 周，按每天 4 小时计）

### 6.2 里程碑

| 里程碑 | 目标 | 预计完成 |
|--------|------|----------|
| **M1** | 状态机可编译，有单元测试 | Day 1 |
| **M2** | Pi 事件 → 状态变更可触发 | Day 3 |
| **M3** | 前端可渲染 SVG 宠物 + 状态动画 | Day 5 |
| **M4** | 端到端：Pi 事件驱动桌宠动画 | Day 7 |
| **M5** | WebSocket + TTS + 托盘 | Day 10 |
| **M6** | MVP 发布候选 | Day 12 |

---

## 七、MVP 目录结构（最终版）

```
agent-pet-hub/
├── src-tauri/
│   ├── Cargo.toml                    # 依赖：tokio, tokio-tungstenite, serde, schemars, tauri, inotify (Linux) / fsevents (macOS)
│   ├── src/
│   │   ├── main.rs                   # Tauri 入口，启动各模块
│   │   ├── lib.rs                    # 库入口
│   │   │
│   │   ├── event_bus/
│   │   │   ├── mod.rs
│   │   │   └── bus.rs                # tokio::broadcast 实现
│   │   │
│   │   ├── state_machine/
│   │   │   ├── mod.rs
│   │   │   ├── machine.rs            # 状态机 + 防抖
│   │   │   └── transitions.rs        # 转换表
│   │   │
│   │   ├── adapter/
│   │   │   ├── mod.rs
│   │   │   ├── trait.rs              # AgentAdapter trait
│   │   │   ├── pi_adapter.rs         # Pi 适配器
│   │   │   ├── pi_watcher.rs         # JSONL watcher
│   │   │   └── event_converter.rs    # 事件转换
│   │   │
│   │   ├── ipc/
│   │   │   ├── mod.rs
│   │   │   └── ws_server.rs          # WebSocket 服务器
│   │   │
│   │   ├── window/
│   │   │   ├── mod.rs
│   │   │   ├── pet_window.rs         # 悬浮窗
│   │   │   └── tray.rs               # 托盘
│   │   │
│   │   ├── tts/
│   │   │   ├── mod.rs
│   │   │   └── engine.rs             # 跨平台 TTS
│   │   │
│   │   ├── plugin/
│   │   │   ├── mod.rs
│   │   │   └── manager.rs            # 插件管理器
│   │   │
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   └── settings.rs           # 配置读写
│   │   │
│   │   └── lib.rs                    # 汇总导出
│   │
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   └── resources/skins/default/
│       ├── index.json
│       ├── pet.svg
│       └── css/
│           ├── idle.css
│           ├── thinking.css
│           ├── working.css
│           ├── waiting.css
│           ├── success.css
│           └── error.css
│
├── src/                              # 前端
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── PetWindow.tsx             # 悬浮窗容器
│   │   ├── PetSVG.tsx                # SVG 渲染
│   │   └── PetStatus.tsx             # 状态文字
│   ├── hooks/
│   │   ├── useAgentState.ts          # 状态管理
│   │   └── useTauri.ts               # Tauri 调用
│   ├── store/
│   │   ├── petStore.ts
│   │   └── settingsStore.ts
│   ├── services/
│   │   └── wsClient.ts
│   ├── types/
│   │   ├── events.ts
│   │   └── pet.ts
│   └── assets/
│       └── skins/
│
├── packages/
│   └── protocol/
│       ├── src/
│       │   ├── events.ts             # 类型定义
│       │   ├── schemas.ts            # Zod schemas
│       │   └── index.ts
│       └── tests/
│
├── skills/pi/
│   └── pet-event-logger.ts           # Pi 扩展
│
├── docs/
│   └── phase4-mvp.md
│
├── package.json
├── pnpm-workspace.yaml
└── README.md
```

---

## 八、MVP 验收标准（详细）

### 8.1 功能验收

| # | 验收项 | 验收方式 |
|---|--------|----------|
| 1 | `cargo build` 成功编译 | CI 自动执行 |
| 2 | `pnpm dev` 前端可启动 | 手动测试 |
| 3 | 桌宠悬浮窗可显示在桌面 | 手动测试 |
| 4 | 桌宠可被鼠标拖拽 | 手动测试 |
| 5 | 托盘图标显示并可用 | 手动测试 |
| 6 | Pi Agent 启动后，桌宠进入 Thinking 状态 | 手动测试 |
| 7 | Pi 工具调用时，桌宠变为 Working 状态 | 手动测试 |
| 8 | Pi 工具完成后，桌宠回到 Thinking | 手动测试 |
| 9 | Pi 会话结束，桌宠回到 Idle | 手动测试 |
| 10 | 工具错误时，桌宠显示 Error 动画 | 手动测试 |
| 11 | WebSocket 可连接并接收事件 | 手动测试 |
| 12 | TTS 在状态变更时播报 | 手动测试 |

### 8.2 质量验收

| # | 验收项 | 标准 |
|---|--------|------|
| 1 | Rust 单元测试覆盖率 | > 60% |
| 2 | 事件转换测试覆盖所有 Pi 事件类型 | 100% |
| 3 | 状态机测试覆盖所有转换路径 | 100% |
| 4 | TypeScript ESLint 无错误 | 零错误 |
| 5 | 无 console.error 在生产日志 | 零错误 |

### 8.3 性能验收

| # | 验收项 | 标准 |
|---|--------|------|
| 1 | 桌宠内存占用 | < 100MB（含 Tauri 运行时） |
| 2 | 状态变更响应延迟 | < 200ms |
| 3 | WebSocket 事件推送延迟 | < 100ms |
| 4 | TTS 播报启动时间 | < 2s |

---

## 九、MVP 技术决策说明

### 为什么 MVP 只做 Pi 适配器？

1. **Pi 的事件系统最完善**：TypeScript Extension API 成熟，支持 tool_call, text_delta, turn_end 等细粒度事件
2. **agent-hooks-playground 已验证**：Pi 的事件采集方案已有成熟实现参考
3. **Hermes 和 OpenClaw 的适配器可复用同一套代码结构**：先验证核心架构，再扩展其他 Agent
4. **降低 MVP 风险**：单一 Agent 简化端到端测试

### 为什么 MVP 使用 SVG + CSS 动画而非 Lottie？

| 方案 | MVP 适用性 | 理由 |
|------|-----------|------|
| **SVG + CSS** | ⭐⭐⭐⭐⭐ | 零依赖、轻量、可直接用 React 渲染、状态切换简单 |
| Lottie | ⭐⭐⭐ | 需要 lottie-web 依赖，增加 bundle 体积 |
| Live2D | ⭐ | WASM 加载慢，MVP 阶段太重 |

**决策**：MVP 使用 SVG + CSS，后续可无缝切换到 Lottie。

### 为什么 JSONL 文件而非直接 IPC？

| 方案 | 适用性 | 理由 |
|------|--------|------|
| **JSONL 文件** | ⭐⭐⭐⭐ | 所有 Agent 都支持 Shell Hook 输出 JSONL，解耦最彻底 |
| 直接 TCP/UDP | ⭐⭐ | 需要各 Agent 启动自己的 socket server |
| Unix Socket | ⭐⭐ | Linux/macOS 友好，Windows 需命名管道 |

**决策**：MVP 使用 JSONL 文件作为事件通道，后续可升级为 WebSocket/Unix Socket。

---

## 十、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Pi JSONL 文件格式变更 | 事件解析失败 | 版本检测 + fallback 到 raw 解析 |
| 文件 watcher 在 macOS 上的性能 | 高 CPU 占用 | 使用 fsevents 而非轮询 |
| SVG 动画在不同 OS 上渲染不一致 | UI 不一致 | 使用标准 CSS 属性，避免浏览器特定特性 |
| TTS 在 Linux 上不可用 | 语音功能缺失 | 优雅降级：仅显示状态，不崩溃 |
| WebSocket 客户端连接失败 | 外部集成失败 | 记录日志，不阻断主流程 |

---

*此 MVP 设计为项目核心路线图，后续 Phase 5（实现规划）将基于此拆分为具体开发任务。*
