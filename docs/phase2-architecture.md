# 第二阶段：架构设计

> 基于用户确认的决策：
> - 视觉风格：A（2D 卡通/SVG，后期扩展 Live2D/VRM）
> - 交互方式：C（可拖拽），对话直接返回到 AI Agent
> - 目标 OS：C（全平台 Linux + macOS + Windows）
> - 常驻模式：C（托盘图标 + 悬浮窗）
> - MVP Agent：C（三种全部，但先实现 Pi 连接）
> - 插件系统：A（MVP 包含基础插件）
> - 语音：B（需要，TTS 播报状态）
> - 桌面框架：A（Tauri 2.x）

---

## 一、系统架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Agent-Pet-Hub                                │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Tauri Desktop App (Frontend)               │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │   │
│  │  │  System Tray │  │ Pet Window  │  │    Settings Panel   │  │   │
│  │  │  (托盘图标)  │  │ (悬浮窗)    │  │                     │  │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │   │
│  │         │                │                      │            │   │
│  │         └────────────────┼──────────────────────┘            │   │
│  │                          │ Tauri Events / invoke             │   │
│  │  ┌───────────────────────▼──────────────────────────────┐   │   │
│  │  │              Pet Renderer & Animation Engine          │   │   │
│  │  │  ┌───────────┐ ┌──────────┐ ┌────────────────────┐  │   │   │
│  │  │  │ Lottie    │ │ CSS/SVG  │ │ Pet State Machine  │  │   │   │
│  │  │  │ Renderer  │ │ Animator │ │ (状态管理)          │  │   │   │
│  │  │  └───────────┘ └──────────┘ └────────────────────┘  │   │   │
│  │  └─────────────────────────────────────────────────────────────│   │
│  │  ┌────────────────────────────────────────────────────────────│   │
│  │  │              Plugin System (Sandboxed)                     │   │
│  │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  │   │
│  │  │  │ Skin     │ │ Motion   │ │ Voice    │ │ Notification│  │   │
│  │  │  │ Plugin   │ │ Plugin   │ │ Plugin   │ │ Plugin     │  │   │
│  │  │  └──────────┘ └──────────┘ └──────────┘ └────────────┘  │   │
│  │  └─────────────────────────────────────────────────────────────│   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│              Tauri Backend (Rust)                                   │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │   │
│  │  │ Event    │  │ State    │  │ WebSocket│  │ Window     │  │   │
│  │  │ Bus      │  │ Manager  │  │ Server   │  │ Manager    │  │   │
│  │  │(事件总线) │  │(状态管理) │  │(外部IPC)│  │(窗口管理)   │  │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └────────────┘  │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │   │
│  │  │ Agent    │  │ Adapter  │  │ TTS      │                 │   │
│  │  │ Registry │  │ Router   │  │ Engine   │                 │   │
│  │  │(注册中心) │  │(路由分发) │  │(语音引擎) │                 │   │
│  │  └──────────┘  └──────────┘  └──────────┘                 │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │
│  │  Pi Adapter  │  │ Hermes       │  │ OpenClaw     │             │
│  │              │  │ Adapter      │  │ Adapter      │             │
│  │ • Extension  │  │ • Shell Hook │  │ • Hook API   │             │
│  │ • RPC Mode   │  │ • Gateway    │  │ • App SDK    │             │
│  │ • JSONL Log  │  │ • Plugin     │  │ • HTTP RPC   │             │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘             │
│         │                  │                  │                     │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌──────▼───────┐             │
│  │  Pi Agent    │  │  Hermes Agent│  │  OpenClaw    │             │
│  │  (Terminal)  │  │  (CLI/GW)    │  │  (Gateway)   │             │
│  └──────────────┘  └──────────────┘  └──────────────┘             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 二、数据流图

```
  Agent 执行任务
       │
       ▼
  ┌────────────┐
  │  Agent     │  触发原生事件（hook/extension/SDK）
  │  Native    │
  └──────┬─────┘
         │
         ▼
  ┌─────────────────────────────────────────────────┐
  │              Adapter Layer (适配器层)              │
  │                                                   │
  │  Pi Adapter     ──→ TypeScript Extension + JSONL  │
  │  Hermes Adapter ──→ Gateway Events + Shell Hook   │
  │  OpenClaw Adpt  ──→ Hook API + App SDK            │
  │                                                   │
  │  统一转换 → UnifiedAgentEvent                     │
  └────────────────┬──────────────────────────────────┘
                   │
                   ▼
  ┌─────────────────────────────────────────────────┐
  │              Event Bus (事件总线)                  │
  │                                                   │
  │  UnifiedAgentEvent {                            │ │
  │    source: "pi",                                 │ │
  │    type: "tool_call" | "thinking" | "working"  │ │
  │    state: "thinking" | "working" | "idle"      │ │
  │    payload: { ... },                            │ │
  │    timestamp: ISO8601                           │ │
  │  }                                              │ │
  │                                                   │
  │  发布 → StateMachine + WebSocket + TTS           │
  └───────┬───────────────┬───────────────┬──────────┘
          │               │               │
          ▼               ▼               ▼
  ┌──────────────┐ ┌─────────────┐ ┌──────────────┐
  │ StateMachine │ │ WebSocket   │ │   TTS Engine │
  │              │ │ Server      │ │              │
  │ Idle →       │ │ 广播事件    │ │ 状态变化播报 │
  │ Thinking →   │ │ 外部订阅    │ │              │
  │ Working →    │ │             │ │              │
  │ Success      │ │             │ │              │
  │ Error        │ │             │ │              │
  └──────┬───────┘ └─────────────┘ └──────────────┘
         │
         ▼
  ┌─────────────────────────────────────────────────┐
  │           Pet Animation Engine                   │
  │                                                   │
  │  State → Animation Mapping                        │
  │  thinking → 眨眼 + 歪头                            │
  │  working  → 敲代码动画                            │
  │  idle     → 呼吸 + 发呆                           │
  │  success  → 庆祝动画                              │
  │  error    → 摇头 + 红色提示                       │
  └─────────────────────────────────────────────────┘
```

---

## 三、目录结构

```
agent-pet-hub/
├── src-tauri/                      # Rust 后端 (Tauri)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                 # 入口
│   │   ├── lib.rs                  # 库入口（Tauri lib 模式）
│   │   │
│   │   ├── event_bus/              # 事件总线
│   │   │   ├── mod.rs
│   │   │   └── bus.rs              # 核心事件发布/订阅
│   │   │
│   │   ├── state_machine/          # 状态机
│   │   │   ├── mod.rs
│   │   │   ├── machine.rs          # 状态机核心
│   │   │   └── transitions.rs      # 状态转换表
│   │   │
│   │   ├── adapter/                # 适配器层
│   │   │   ├── mod.rs
│   │   │   ├── trait.rs            # AgentAdapter trait
│   │   │   ├── pi_adapter.rs       # Pi Agent 适配器
│   │   │   ├── hermes_adapter.rs   # Hermes Agent 适配器
│   │   │   ├── openclaw_adapter.rs # OpenClaw 适配器
│   │   │   └── base/               # 公共适配工具
│   │   │       ├── jsonl_reader.rs # JSONL 日志读取
│   │   │       └── shell_hook.rs   # Shell Hook 通用逻辑
│   │   │
│   │   ├── registry/               # Agent 注册中心
│   │   │   ├── mod.rs
│   │   │   └── registry.rs         # 适配器注册/发现
│   │   │
│   │   ├── ipc/                    # IPC 层
│   │   │   ├── mod.rs
│   │   │   ├── ws_server.rs        # WebSocket 服务器
│   │   │   └── commands.rs         # Tauri invoke commands
│   │   │
│   │   ├── window/                 # 窗口管理
│   │   │   ├── mod.rs
│   │   │   ├── pet_window.rs       # 桌宠悬浮窗
│   │   │   └── tray.rs             # 系统托盘
│   │   │
│   │   ├── tts/                    # 语音引擎
│   │   │   ├── mod.rs
│   │   │   └── engine.rs           # TTS 核心
│   │   │
│   │   ├── plugin/                 # 插件系统
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs          # 插件管理器
│   │   │   └── api.rs              # 插件 API
│   │   │
│   │   └── config/                 # 配置管理
│   │       ├── mod.rs
│   │       └── settings.rs         # 设置读写
│   │
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   └── resources/                  # 默认皮肤资源
│       ├── skins/
│       │   ├── default/
│       │   └── pixel-cat/
│       └── audio/
│
├── src/                            # Web 前端 (TypeScript)
│   ├── main.tsx                    # 入口
│   ├── App.tsx
│   │
│   ├── components/                 # React 组件
│   │   ├── PetRenderer.tsx         # 桌宠渲染器
│   │   ├── PetAnimation.tsx        # 动画组件
│   │   ├── StatusIndicator.tsx     # 状态指示器
│   │   ├── ChatBubble.tsx          # 对话气泡
│   │   ├── SettingsPanel.tsx       # 设置面板
│   │   ├── TrayMenu.tsx            # 托盘菜单
│   │   └── plugins/                # 插件组件
│   │       ├── SkinSelector.tsx    # 皮肤选择器
│   │       └── PluginManager.tsx   # 插件管理
│   │
│   ├── hooks/                      # React Hooks
│   │   ├── useAgentState.ts        # Agent 状态 Hook
│   │   ├── usePetAnimation.ts      # 动画 Hook
│   │   ├── useWebSocket.ts         # WebSocket Hook
│   │   └── useTauri.ts             # Tauri 调用 Hook
│   │
│   ├── store/                      # 状态管理
│   │   ├── petStore.ts             # 桌宠状态
│   │   ├── agentStore.ts           # Agent 状态
│   │   └── settingsStore.ts        # 设置状态
│   │
│   ├── services/                   # 前端服务
│   │   ├── petRenderer.ts          # 渲染器逻辑
│   │   └── animationEngine.ts      # 动画引擎
│   │
│   ├── plugins/                    # 前端插件
│   │   └── api.ts                  # 前端插件 API
│   │
│   ├── types/                      # TypeScript 类型
│   │   ├── agent.ts                # Agent 相关类型
│   │   ├── event.ts                # 事件类型
│   │   ├── pet.ts                  # 桌宠类型
│   │   └── plugin.ts               # 插件类型
│   │
│   └── assets/                     # 静态资源
│       ├── skins/
│       ├── animations/
│       └── icons/
│
├── packages/                       # Monorepo 包
│   ├── protocol/                   # 协议定义
│   │   ├── package.json
│   │   ├── src/
│   │   │   ├── events.ts           # 事件 Schema
│   │   │   ├── state.ts            # 状态枚举
│   │   │   ├── tool.ts             # 工具调用 Schema
│   │   │   └── index.ts
│   │   └── tests/
│   │
│   └── sdk/                        # 前端 SDK
│       ├── package.json
│       ├── src/
│       │   ├── client.ts           # WebSocket 客户端
│       │   └── index.ts
│       └── tests/
│
├── skills/                         # Agent 技能包
│   ├── pi/
│   │   └── agent-hooks-logger.ts   # Pi 事件采集扩展
│   ├── hermes/
│   │   └── hooks.yaml              # Hermes 钩子配置
│   └── openclaw/
│       └── hooks/                  # OpenClaw 钩子目录
│
├── docs/                           # 文档
│   ├── phase1-research.md
│   ├── phase2-architecture.md
│   ├── phase3-protocol.md
│   ├── phase4-mvp.md
│   └── phase5-tasks.md
│
├── scripts/                        # 构建脚本
│   ├── setup-agent-hooks.sh        # 初始化 Agent Hook
│   └── build-plugins.sh
│
├── .github/workflows/              # CI/CD
│   ├── build.yml
│   └── release.yml
│
├── package.json                    # 根 package.json
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── tsconfig.json
└── README.md
```

---

## 四、模块说明

### 核心模块

#### 1. Event Bus（事件总线）
- **职责**：Agent 事件的发布/订阅中枢
- **设计**：
  - Rust 端实现 `EventBus<T>`，支持类型安全的事件分发
  - 使用 `tokio::broadcast` 实现高效的 pub/sub
  - 支持优先级队列：State > Tool > System
- **关键 API**：
  ```rust
  pub struct EventBus;
  impl EventBus {
      pub fn publish(event: UnifiedAgentEvent);
      pub fn subscribe<T: Into<EventKind>>(kind: T) -> Receiver<UnifiedAgentEvent>;
      pub fn publish_to_frontend(event: UnifiedAgentEvent);
      pub fn publish_to_ws(event: UnifiedAgentEvent);
  }
  ```

#### 2. State Machine（状态机）
- **职责**：定义桌宠的状态转换规则
- **状态枚举**：
  ```rust
  pub enum PetState {
      Idle,        // 空闲 - 等待 Agent 指令
      Thinking,    // 思考中 - Agent 正在处理
      Working,     // 工作中 - 正在执行工具/代码
      Waiting,     // 等待中 - 等待用户输入/审批
      Success,     // 成功 - 任务完成
      Error,       // 错误 - 任务失败
      Speaking,    // 语音播报中（TTS 触发）
  }
  ```
- **转换规则**：
  ```
  Idle → Thinking (session_start / user_prompt)
  Thinking → Working (pre_tool_call)
  Working → Thinking (post_tool_call / tool_result)
  Working → Waiting (pre_tool_call with approval_required)
  Waiting → Thinking (user_approved / user_rejected)
  Thinking → Success (session_end / agent_stop with no errors)
  Thinking → Error (session_end / tool_error)
  Any → Idle (session_end / timeout)
  ```

#### 3. Adapter Layer（适配器层）
- **设计原则**：Interface Segregation + Open/Closed Principle
- **核心 Trait**：
  ```rust
  #[async_trait]
  pub trait AgentAdapter: Send + Sync {
      /// 适配器唯一标识
      fn identity(&self) -> AgentIdentity;
      
      /// 初始化适配器（连接 Agent）
      async fn connect(&self, config: &AgentConfig) -> Result<(), AdapterError>;
      
      /// 启动事件监听
      async fn start_listening(&self, event_handler: EventHandler) -> Result<(), AdapterError>;
      
      /// 停止事件监听
      async fn stop_listening(&self) -> Result<(), AdapterError>;
      
      /// 发送消息到 Agent（用于对话）
      async fn send_message(&self, text: &str, session_id: &str) -> Result<String, AdapterError>;
      
      /// 获取当前会话列表
      async fn list_sessions(&self) -> Result<Vec<Session>, AdapterError>;
      
      /// 适配器健康检查
      async fn health_check(&self) -> Result<HealthStatus, AdapterError>;
  }
  ```

#### 4. Registry（注册中心）
- **职责**：管理所有已注册的 Agent 适配器
- **设计**：
  ```rust
  pub struct AgentRegistry {
      adapters: RwLock<HashMap<String, Box<dyn AgentAdapter>>>,
  }
  
  impl AgentRegistry {
      pub fn register(&mut self, adapter: Box<dyn AgentAdapter>);
      pub fn get(&self, name: &str) -> Option<&dyn AgentAdapter>;
      pub fn list(&self) -> Vec<AgentIdentity>;
      pub fn get_active(&self) -> Option<&dyn AgentAdapter>;
      pub fn set_active(&self, name: &str) -> Result<(), RegistryError>;
  }
  ```

#### 5. WebSocket Server（外部 IPC）
- **职责**：为外部进程提供事件订阅
- **设计**：
  - 使用 `tokio-tungstenite` 实现 WebSocket
  - 默认端口：`8765`（可配置）
  - 支持认证：`Authorization: Bearer <token>`
  - 事件订阅：`{"action": "subscribe", "eventTypes": ["tool_call", "session_start"]}`
  - 订阅确认：`{"type": "subscribed", "eventTypes": [...]}`

#### 6. Window Manager（窗口管理）
- **职责**：管理桌宠悬浮窗和系统托盘
- **功能**：
  - 悬浮窗：置顶、透明、鼠标穿透、拖拽、边界吸附
  - 托盘图标：状态指示（颜色/表情）
  - 窗口位置记忆（跨重启）
  - 多显示器支持

#### 7. TTS Engine（语音引擎）
- **职责**：状态变化的语音播报
- **设计**：
  - 支持多种后端（macOS `say` / Linux `espeak` / Windows `Edge-TTS`）
  - 可配置播报时机
  - 消息队列，避免打断
  - 静音模式（专注模式）

#### 8. Plugin System（插件系统）
- **职责**：扩展桌宠功能
- **插件类型**：
  - **Skin Plugin**：替换桌宠外观（皮肤包）
  - **Motion Plugin**：自定义动画逻辑
  - **Voice Plugin**：自定义语音引擎
  - **Notification Plugin**：自定义通知方式
- **插件 API**：
  ```typescript
  interface PetPlugin {
    id: string;
    name: string;
    version: string;
    onEvent?(event: UnifiedAgentEvent): void;
    onStateChanged?(newState: PetState, prevState: PetState): void;
    getSkin?(): SkinDefinition;
    getAnimations?(): AnimationDefinition[];
  }
  ```

---

## 五、Adapter 层详细设计

### AgentAdapter Trait 完整定义

```rust
/// 适配器身份标识
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentIdentity {
    pub name: String,           // "pi" | "hermes" | "openclaw"
    pub version: Option<String>,
    pub display_name: String,   // "Pi Agent" | "Hermes" | "OpenClaw"
}

/// 适配器错误
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Event parse error: {0}")]
    ParseError(String),
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Other: {0}")]
    Other(#[from] anyhow::Error),
}

/// 会话信息
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub status: SessionStatus,
    pub started_at: Option<DateTime<Utc>>,
}

/// 会话状态
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Idle,
    Active,
    Paused,
    Error,
}

/// 统一事件（所有适配器输出此格式）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnifiedAgentEvent {
    /// 事件唯一 ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 事件类型（统一枚举）
    pub event_type: AgentEventType,
    /// 对应的宠物状态
    pub pet_state: PetState,
    /// 来源 Agent
    pub source: AgentIdentity,
    /// 会话 ID
    pub session_id: Option<String>,
    /// 工具名称（仅 tool 事件）
    pub tool_name: Option<String>,
    /// 工具参数预览
    pub tool_args_preview: Option<String>,
    /// 任务/提示词预览
    pub task_preview: Option<String>,
    /// 原始数据（保留原始格式）
    pub raw: Option<serde_json::Value>,
}
```

### Pi 适配器实现要点

```rust
pub struct PiAdapter {
    identity: AgentIdentity,
    config: PiConfig,
    event_sender: tokio::sync::broadcast::Sender<UnifiedAgentEvent>,
    process: Option<tokio::process::Child>,
    watcher: Option<tokio::fs::File>,
}

impl AgentAdapter for PiAdapter {
    async fn connect(&self, config: &AgentConfig) -> Result<(), AdapterError> {
        // 1. 写入 Pi 扩展文件到 ~/.pi/agent/extensions/
        // 2. 启动 Pi Agent 的 RPC 模式（stdin/stdout JSON）
        // 3. 或通过 JSONL 文件监听
    }
    
    async fn start_listening(&self, event_handler: EventHandler) -> Result<(), AdapterError> {
        // 1. 监听 Pi Agent 输出的 JSONL 事件
        // 2. 转换为 UnifiedAgentEvent
        // 3. 发送到 Event Bus
    }
}
```

### Hermes 适配器实现要点

```rust
pub struct HermesAdapter {
    identity: AgentIdentity,
    config: HermesConfig,
    event_sender: tokio::sync::broadcast::Sender<UnifiedAgentEvent>,
}

impl AgentAdapter for HermesAdapter {
    async fn connect(&self, config: &AgentConfig) -> Result<(), AdapterError> {
        // 1. 写入 HOOK.yaml + handler.py 到 ~/.hermes/hooks/
        // 2. handler.py 将事件写入 JSONL 文件
        // 3. 或通过 Gateway Events API 订阅
    }
    
    async fn start_listening(&self, event_handler: EventHandler) -> Result<(), AdapterError> {
        // 1. 监控 JSONL 日志文件
        // 2. 或 WebSocket 订阅 Gateway Events
    }
}
```

### OpenClaw 适配器实现要点

```rust
pub struct OpenClawAdapter {
    identity: AgentIdentity,
    config: OpenClawConfig,
    event_sender: tokio::sync::broadcast::Sender<UnifiedAgentEvent>,
}

impl AgentAdapter for OpenClawAdapter {
    async fn connect(&self, config: &AgentConfig) -> Result<(), AdapterError> {
        // 1. 写入 HOOK.md 到 ~/.openclaw/hooks/
        // 2. 通过 @openclaw/sdk 订阅事件
        // 3. 或通过 HTTP RPC 轮询
    }
    
    async fn start_listening(&self, event_handler: EventHandler) -> Result<(), AdapterError> {
        // 1. 监控 OpenClaw 日志/事件
        // 2. 使用 App SDK 订阅
    }
}
```

---

## 六、IPC 通讯层设计

### 内部 IPC（Rust ↔ Web）
```
Rust Backend                      Web Frontend
     │                                 │
     │  tauri::emit("agent:state", ...)│
     │◄────────────────────────────────│
     │                                 │
     │  tauri::invoke("pet:get_state") │
     │────────────────────────────────►│
     │                                 │
     │  Response: { state: "working" } │
     │◄────────────────────────────────│
```

### 外部 IPC（WebSocket）
```
External Process                    WebSocket Server
     │                                   │
     │  CONNECT ws://127.0.0.1:8765      │
     │──────────────────────────────────►│
     │                                   │
     │  AUTH {"token": "xxx"}            │
     │──────────────────────────────────►│
     │                                   │
     │  SUBSCRIBE {"events": ["*"]}      │
     │──────────────────────────────────►│
     │                                   │
     │  EVENT {"type": "tool_call", ...} │
     │◄──────────────────────────────────│
```

### 事件协议（WebSocket）
```typescript
interface WSMessage {
  type: 'event' | 'error' | 'ack' | 'ping' | 'pong';
  payload: Record<string, unknown>;
}

interface WSEvent extends WSMessage {
  type: 'event';
  payload: {
    event: UnifiedAgentEvent;
    subscriptionId?: string;
  };
}
```

---

## 七、技术决策说明

### 为什么选择 Tauri 2.x？

| 维度 | Tauri 2.x | Electron | Neutralinojs |
|------|-----------|----------|--------------|
| 内存占用 | ~30-40MB | ~200-300MB | ~20-30MB |
| 安装包大小 | <10MB | ~150MB | <5MB |
| WebView | 系统原生 | 内置 Chromium | 系统原生 |
| 语言栈 | Rust + Web | JS/TS | Go + Web |
| GitHub Stars | ~90k ⭐ | ~109k ⭐ | ~22k ⭐ |
| 活跃度 | 极高（每周提交）| 极高 | 中等 |
| 最后更新 | 持续维护 | 持续维护 | 较慢 |
| 跨平台 | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux |
| Rust FFI | ✅ 原生 | ⚠️ 需 napi | ❌ 无 |

**决策**：Tauri 2.x
- 桌宠需要 7x24 运行，内存是关键
- Rust 后端适合事件处理和 IPC
- 系统 WebView 不捆绑 Chromium，轻量
- 90k+ ⭐ 社区规模足够大

### 为什么选择 Lottie + CSS 动画？

| 维度 | Lottie | Live2D | CSS/SVG |
|------|--------|--------|---------|
| 复杂度 | ⭐ | ⭐⭐⭐⭐ | ⭐ |
| 文件大小 | 小 | 中-大 | 极小 |
| 设计师友好 | ✅ | ⚠️ 需要 Cubism Editor | ✅ |
| 动画流畅度 | 好 | 极好 | 中等 |
| 跨平台 | ✅ | ✅ (WebSML) | ✅ |
| 学习曲线 | 低 | 高 | 低 |

**决策**：默认 Lottie + CSS，预留 Live2D 接口
- MVP 阶段用 Lottie 足够
- Live2D 作为后期扩展
- CSS 动画处理简单状态变化（呼吸、眨眼）

### 为什么选择 WebSocket 作为外部 IPC？

- 所有 Agent 的 Adapter 都在 Rust 进程中，外部进程需要通过 IPC 获取事件
- WebSocket 是标准协议，任何语言/平台都能连接
- 支持多客户端同时订阅
- 内置心跳机制，自动重连
- 比 Unix Socket 跨平台性好

---

## 八、开发优先级

### Phase 1 - Core（核心）
1. Tauri 项目初始化
2. Event Bus 实现
3. State Machine 实现
4. 基础前端框架（Pet Window + Tray）

### Phase 2 - Pi Adapter
5. Pi Adapter 实现
6. Pi 事件采集（JSONL/Extension）
7. Pi → UnifiedEvent 转换
8. 端到端测试（Pi → Event Bus → Pet Animation）

### Phase 3 - Animation & Interaction
9. Lottie 动画渲染器
10. 状态到动画的映射
11. 拖拽交互
12. 鼠标穿透 + 窗口管理

### Phase 4 - Multi-Agent
13. Hermes Adapter
14. OpenClaw Adapter
15. Agent 切换逻辑
16. 多 Agent 状态隔离

### Phase 5 - IPC & Plugin
17. WebSocket Server
18. 基础 Plugin 系统
19. Skin 插件
20. 前端 Plugin API

### Phase 6 - Polish
21. TTS 语音引擎
22. 设置面板
23. 跨平台构建
24. CI/CD + Release

---

## 九、关键设计原则

1. **Adapter 隔离**：新增 Agent 只需实现 AgentAdapter trait，不改核心代码
2. **事件驱动**：所有状态变化通过 Event Bus，天然支持多消费者
3. **插件沙箱**：插件运行在独立作用域，不污染核心
4. **配置驱动**：所有行为通过配置文件控制，无需改代码
5. **向后兼容**：协议版本管理，新版本不破坏旧客户端

---

*本设计文档为项目核心架构，后续阶段需严格遵循。*
