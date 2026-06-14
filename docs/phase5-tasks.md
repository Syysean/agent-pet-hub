# 第五阶段：实现规划（任务拆分）

> 日期：2026-06-08
> 基于：phase4-mvp.md（MVP 设计）+ phase2-architecture.md（架构设计）+ phase3-protocol.md（协议设计）
> 状态：已确认

---

## 任务概览

| 分组 | 任务数 | 总工时 | 串行/并行 |
|------|--------|--------|-----------|
| A. 项目初始化 | 4 | 8h | 可并行 |
| B. 核心协议与类型 | 3 | 6h | 可并行 |
| C. Rust 后端核心 | 6 | 16h | 部分并行 |
| D. Pi 适配器 | 3 | 8h | 串行 |
| E. 前端核心组件 | 5 | 12h | 可并行 |
| F. IPC 与集成 | 4 | 10h | 部分并行 |
| G. 基础功能 | 4 | 10h | 可并行 |
| H. 集成与测试 | 5 | 11h | 串行 |
| **合计** | **34** | **81h** | |

---

## A 组：项目初始化（4 个任务）

### T001: 初始化 Tauri 2.x 项目

| 属性 | 描述 |
|------|------|
| **目标** | 创建 Tauri 2.x 项目骨架，包含 Rust 后端 + React/TypeScript 前端基础 |
| **输入** | Tauri 2.x 官方文档、项目目录结构定义 |
| **输出** | `src-tauri/` 可编译的 Tauri 项目、`src/` React 项目可启动 |
| **依赖** | 无 |
| **验收标准** | `cargo build` 成功、`pnpm dev` 可启动空窗口、CI 能通过基础编译检查 |
| **预计工时** | 2h |
| **优先级** | P0 |
| **可并行** | ✅ |

**详细步骤**：
1. `cargo tauri init --tauri-with-webview2 --tauri-with-wkwebview`（初始化 Tauri）
2. 配置 `src-tauri/tauri.conf.json`（窗口设置、权限、bundle）
3. 初始化前端：`pnpm create vite src --template react-ts`
4. 配置 pnpm workspace monorepo
5. 配置 TypeScript tsconfig（路径别名、strict 模式）
6. 配置 ESLint + Prettier
7. 验证 `cargo tauri dev` 能启动窗口

---

### T002: 创建协议包 `packages/protocol`

| 属性 | 描述 |
|------|------|
| **目标** | 创建共享协议包，包含 TypeScript 类型定义和 Zod schemas |
| **输入** | phase3-protocol.md 中的协议定义 |
| **输出** | `packages/protocol/src/` 包含 events.ts、schemas.ts、index.ts |
| **依赖** | 无（与 T001 可并行） |
| **验收标准** | `pnpm build` 成功、TypeScript 类型可被其他包 import、Zod schemas 可通过测试 |
| **预计工时** | 3h |
| **优先级** | P0 |
| **可并行** | ✅ |

**详细步骤**：
1. `packages/protocol/package.json`（tsconfig、build 脚本）
2. `packages/protocol/src/events.ts`（所有类型定义：AgentSource、EventCategory、EventType、PetState、ErrorCode、UnifiedAgentEvent 等）
3. `packages/protocol/src/schemas.ts`（Zod schemas）
4. `packages/protocol/src/index.ts`（统一导出）
5. `packages/protocol/tests/events.test.ts`（类型导出测试）
6. `packages/protocol/tests/schemas.test.ts`（Zod 验证测试）

---

### T003: 创建 Pi 事件采集扩展 `skills/pi/pet-event-logger.ts`

| 属性 | 描述 |
|------|------|
| **目标** | 创建 Pi Agent 的 TypeScript Extension，用于采集事件并输出到 JSONL 文件 |
| **输入** | Pi Agent Extension API 文档、phase3-protocol.md |
| **输出** | `skills/pi/pet-event-logger.ts` 可被 `pi install` 安装 |
| **依赖** | T002（需要事件类型定义） |
| **验收标准** | 安装到 `~/.pi/agent/extensions/` 后能正确捕获 tool_call、tool_result、turn_end 等事件，并输出到 `~/.pi/logs/pet-events.jsonl` |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | ✅（与 C 组部分任务） |

**详细步骤**：
1. 创建 Pi Extension 模板（遵循 Pi 扩展规范）
2. 注册 event listener：`ctx.on('tool_call', ...)`、`ctx.on('tool_result', ...)`、`ctx.on('turn_end', ...)`、`ctx.on('compaction', ...)`
3. 将事件转换为统一格式
4. 追加写入 JSONL 文件
5. 错误处理：JSONL 写入失败时降级为 console.log
6. 测试：在 Pi Agent 中安装并触发工具调用，验证 JSONL 输出

---

### T004: 创建默认皮肤资源 `src-tauri/resources/skins/default/`

| 属性 | 描述 |
|------|------|
| **目标** | 创建 MVP 默认皮肤，包含 SVG 宠物和 CSS 动画 |
| **输入** | phase4-mvp.md 中的动画定义 |
| **输出** | `src-tauri/resources/skins/default/` 包含 skin.json、pet.svg、css/ 目录 |
| **依赖** | 无（与 T001 可并行） |
| **验收标准** | skin.json 格式正确、SVG 可渲染、6 种状态 CSS 动画可用 |
| **预计工时** | 2h |
| **优先级** | P0 |
| **可并行** | ✅ |

**详细步骤**：
1. 设计 SVG 宠物（简单的卡通角色，椭圆身体 + 圆头 + 眼睛 + 嘴巴）
2. 编写 6 种状态的 CSS 动画：idle、thinking、working、waiting、success、error
3. 创建 skin.json（皮肤元数据：名称、版本、作者、预览图）
4. 添加 SVG 中的内联动画（眨眼、呼吸等）

---

## B 组：核心协议与类型（3 个任务）

### T005: Rust 端事件类型定义

| 属性 | 描述 |
|------|------|
| **目标** | 在 Rust 端定义与 TypeScript 对齐的事件类型 |
| **输入** | phase3-protocol.md、T002 的输出 |
| **输出** | `src-tauri/src/` 中的事件类型枚举和结构体 |
| **依赖** | T002 |
| **验收标准** | Rust 类型与 TS 类型对齐、`serde` 序列化/反序列化正确、`schemars` 可生成 JSON Schema |
| **预计工时** | 3h |
| **优先级** | P0 |
| **可并行** | ✅ |

**详细步骤**：
1. 定义 `AgentSource`、`EventCategory`、`EventType`、`PetState`、`ErrorCode` 枚举
2. 定义 `UnifiedAgentEvent` 结构体（`#[derive(Serialize, Deserialize, Debug, Clone)]`）
3. 使用 `schemars::JsonSchema` 派生
4. 编写单元测试：序列化/反序列化各事件类型
5. 生成 JSON Schema：`generate_schema()` 函数
6. 导出到 TypeScript（可选：用 `rust-type-to-ts` 自动生成）

---

### T006: Rust 端事件验证器

| 属性 | 描述 |
|------|------|
| **目标** | 创建事件验证函数，确保 JSON 输入符合 Schema |
| **输入** | T005 的类型定义 |
| **输出** | `src-tauri/src/validator.rs` 包含 `validate_event()` 函数 |
| **依赖** | T005 |
| **验收标准** | 合法事件通过验证、非法事件返回错误详情、边界条件（空字符串、超长字段）正确处理 |
| **预计工时** | 2h |
| **优先级** | P0 |
| **可并行** | ✅ |

**详细步骤**：
1. 使用 `serde_json::from_value::<UnifiedAgentEvent>()` 解析
2. 验证 `version` 字段是否为 `"1.0"`
3. 验证 `category` 和 `type` 的映射关系（如 `tool` 类别必须有 `toolName`）
4. 验证字符串长度限制（toolArgsPreview ≤ 200, errorMessage ≤ 500 等）
5. 验证 `id` 格式（ULID）
6. 编写测试用例

---

### T007: 事件映射表（Agent → Unified）

| 属性 | 描述 |
|------|------|
| **目标** | 创建各 Agent 原生事件到统一事件的映射规则 |
| **输入** | phase3-protocol.md 中的映射表 |
| **输出** | `src-tauri/src/mapping.rs` 包含映射函数 |
| **依赖** | T005 |
| **验收标准** | 所有 Phase 3 定义的映射关系已实现、映射测试覆盖 100% |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | ✅（与 D 组部分任务） |

**详细步骤**：
1. 实现 Pi 事件映射：`map_pi_event(&serde_json::Value) -> Result<UnifiedAgentEvent, Error>`
2. 实现 Hermes 事件映射（预留，MVP 不启用）
3. 实现 OpenClaw 事件映射（预留，MVP 不启用）
4. 提取辅助函数：`extract_tool_name()`、`extract_tool_args()`、`extract_prompt()`、`truncate()`
5. 编写映射测试（每个原生事件 → 统一事件的断言）

---

## C 组：Rust 后端核心（6 个任务）

### T008: 事件总线（Event Bus）

| 属性 | 描述 |
|------|------|
| **目标** | 实现基于 tokio::broadcast 的事件发布/订阅系统 |
| **输入** | phase2-architecture.md 中的 Event Bus 设计 |
| **输出** | `src-tauri/src/event_bus/bus.rs` 包含 EventBus 结构体和方法 |
| **依赖** | T005（需要 UnifiedAgentEvent 类型） |
| **验收标准** | publish/subscribe 正常工作、多订阅者能同时收到事件、背压处理正确 |
| **预计工时** | 3h |
| **优先级** | P0 |
| **可并行** | ✅ |

**详细步骤**：
1. 创建 `EventBus` 结构体，内部维护 `tokio::sync::broadcast::Sender<UnifiedAgentEvent>`
2. 实现 `publish()` 方法（发送事件）
3. 实现 `subscribe()` 方法（返回 Receiver）
4. 实现 `subscribe_to_frontend()` 方法（专门用于推送到 Tauri 前端）
5. 实现 `subscribe_to_ws()` 方法（专门用于推送给 WebSocket 客户端）
6. 测试：多订阅者场景、事件丢失场景（Lagged）
7. 编写全局单例访问（`lazy_static` 或 `once_cell`）

---

### T009: 状态机核心（State Machine）

| 属性 | 描述 |
|------|------|
| **目标** | 实现状态机核心，支持状态转换和防抖动 |
| **输入** | phase4-mvp.md 中的状态机设计 |
| **输出** | `src-tauri/src/state_machine/machine.rs` 包含 PetStateMachine 和 StateDebouncer |
| **依赖** | T005（需要 PetState 类型） |
| **验收标准** | 所有状态转换正确、防抖动正常工作、状态变更通知可触发 |
| **预计工时** | 4h |
| **优先级** | P0 |
| **可并行** | ✅ |

**详细步骤**：
1. 实现 `PetStateMachine` 结构体
2. 实现 `handle_event()` 方法（根据转换表返回新状态）
3. 实现 `get_current_state()` 方法
4. 实现 `get_previous_state()` 方法
5. 实现 `StateDebouncer`（500ms 最小状态保持时间）
6. 实现状态变更回调机制（`on_state_change` 闭包）
7. 编写测试：所有转换路径的断言
8. 边界测试：快速连续事件（验证防抖）

---

### T010: 状态转换表（Transitions）

| 属性 | 描述 |
|------|------|
| **目标** | 定义完整的状态转换规则表 |
| **输入** | phase4-mvp.md 中的转换表 |
| **输出** | `src-tauri/src/state_machine/transitions.rs` 包含 `build_transition_table()` |
| **依赖** | T009（状态机核心） |
| **验收标准** | 所有 Phase 4 定义的转换已实现、无遗漏、可序列化 |
| **预计工时** | 2h |
| **优先级** | P0 |
| **可并行** | ✅（与 T009 可并行） |

**详细步骤**：
1. 实现 `build_transition_table()` 函数（返回 HashMap<(PetState, EventType), PetState>）
2. 添加 MVP 核心转换：
   - Connecting → Idle（AdapterConnected）
   - Idle → Thinking（SessionStart / UserPrompt）
   - Thinking → Working（ToolCallStart）
   - Working → Thinking（ToolCallEnd）
   - Working/Thinking → Error（ToolCallError）
   - Thinking/Error → Idle（SessionEnd / UserCancel）
3. 添加防御性转换：Idle → Idle（SessionEnd）
4. 编写测试：每个转换的输入/输出断言

---

### T011: 窗口管理器（Window Manager）

| 属性 | 描述 |
|------|------|
| **目标** | 实现悬浮窗和托盘的创建/管理 |
| **输入** | phase2-architecture.md 中的窗口管理设计 |
| **输出** | `src-tauri/src/window/pet_window.rs` + `src-tauri/src/window/tray.rs` |
| **依赖** | T001（Tauri 项目） |
| **验收标准** | 悬浮窗可创建、置顶、拖拽、鼠标穿透、边界吸附、跨显示器支持 |
| **预计工时** | 4h |
| **优先级** | P1 |
| **可并行** | ✅（与 T008 可并行） |

**详细步骤**：
1. `pet_window.rs`：
   - 创建 Tauri 窗口（无边框、透明、可拖拽）
   - 实现鼠标穿透（`transparent: true`, `decorations: false`）
   - 实现拖拽支持（Tauri `drag` 事件）
   - 实现边界吸附（窗口位置记忆，`localStorage`）
   - 实现窗口置顶（`always_on_top: true`）
   - 实现多显示器支持（`monitor` 属性）
2. `tray.rs`：
   - 创建系统托盘图标
   - 添加菜单项：显示/隐藏、设置、退出
   - 托盘图标状态指示（颜色变化）
3. 配置 `tauri.conf.json` 中的窗口设置
4. 测试：各功能验证

---

### T012: Tauri Commands（前端 API）

| 属性 | 描述 |
|------|------|
| **目标** | 实现前端可调用的 Tauri Commands |
| **输入** | phase3-protocol.md 中的 Tauri Command 格式 |
| **输出** | `src-tauri/src/commands.rs` 包含所有 invoke 函数 |
| **依赖** | T008（Event Bus）、T009（状态机） |
| **验收标准** | 前端可 invoke 各命令、返回值正确、错误处理完整 |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | ✅（与 T011 可并行） |

**详细步骤**：
1. `get_pet_state` → 返回当前状态
2. `get_current_agent` → 返回当前活跃 Agent
3. `list_sessions` → 返回会话列表
4. `send_message` → 发送消息到 Agent
5. `get_settings` → 读取配置
6. `update_settings` → 写入配置
7. `toggle_pet_window` → 切换悬浮窗可见性
8. `toggle_tray_icon` → 切换托盘图标
9. 添加 `tauri::command` 宏
10. 测试：每个 command 的单元测试

---

### T013: 配置读写（Settings）

| 属性 | 描述 |
|------|------|
| **目标** | 实现配置文件的读写 |
| **输入** | 配置文件格式定义 |
| **输出** | `src-tauri/src/config/settings.rs` |
| **依赖** | 无 |
| **验收标准** | 配置可读写、格式验证正确、默认值可用 |
| **预计工时** | 2h |
| **优先级** | P1 |
| **可并行** | ✅ |

**详细步骤**：
1. 定义配置结构体（`AppSettings`）：
   - `pet_window`: { position, size, always_on_top }
   - `tray`: { enabled, icon_style }
   - `tts`: { enabled, engine, volume, rules }
   - `websocket`: { port, auth_token, enabled }
   - `plugins`: { dir, enabled }
   - `adapter`: { pi: { log_dir, event_file }, hermes, openclaw }
2. 实现 `load()` / `save()` / `get()` / `set()` 方法
3. 配置路径：`~/.config/agent-pet-hub/config.json`
4. 实现默认配置生成
5. 测试：读写各配置项

---

## D 组：Pi 适配器（3 个任务）

### T014: Pi 适配器核心（PiAdapter）

| 属性 | 描述 |
|------|------|
| **目标** | 实现 Pi Adapter，作为 Event Bus 的事件源 |
| **输入** | T005 类型、T013 配置 |
| **输出** | `src-tauri/src/adapter/pi_adapter.rs` |
| **依赖** | T005, T008, T013 |
| **验收标准** | 可创建实例、可启动/停止、可连接/断开 |
| **预计工时** | 4h |
| **优先级** | P1 |
| **可并行** | ✅（与 C 组其他任务） |

**详细步骤**：
1. 实现 `PiAdapter` 结构体：
   ```rust
   pub struct PiAdapter {
       config: PiAdapterConfig,
       event_bus: EventBus,
       state_machine: Arc<Mutex<PetStateMachine>>,
       tts_engine: Option<Arc<TTSEngine>>,
   }
   ```
2. 实现 `new()` 构造函数
3. 实现 `connect()` / `disconnect()` 方法
4. 实现 `start_listening()` / `stop_listening()` 方法（调用 T016）
5. 实现 `send_message()` 方法（通过 Pi Extension）
6. 实现 `list_sessions()` 方法
7. 实现 `health_check()` 方法
8. 测试：生命周期各阶段

---

### T015: Pi 事件转换器（EventConverter）

| 属性 | 描述 |
|------|------|
| **目标** | 实现 Pi 原生事件到统一事件的转换 |
| **输入** | T005 类型、T007 映射规则 |
| **输出** | `src-tauri/src/adapter/event_converter.rs` |
| **依赖** | T005, T007 |
| **验收标准** | 所有 Pi 原生事件类型可正确转换、raw 字段保留原始数据 |
| **预计工时** | 4h |
| **优先级** | P1 |
| **可并行** | ✅（与 T014 可并行） |

**详细步骤**：
1. 实现 `EventConverter::convert()` 方法
2. 处理 Pi 事件类型：
   - `session_start` → SessionStart → Thinking
   - `user_prompt` → UserPrompt → Thinking
   - `tool_call` → ToolCallStart → Working
   - `tool_result` → ToolCallEnd → Thinking
   - `tool_error` → ToolCallError → Error
   - `turn_end` → SessionEnd → Idle
   - `compaction` → SessionCompaction → Thinking
3. 提取辅助函数：
   - `extract_tool_name()`
   - `extract_tool_args()` → trunc 200 chars
   - `extract_result_preview()` → trunc 500 chars
   - `extract_prompt()` → trunc 300 chars
   - `extract_error_message()` → trunc 500 chars
   - `extract_session_id()`
4. 生成 ULID 作为事件 ID
5. 测试：每种事件类型的转换断言

---

### T016: JSONL 文件监听器（PiWatcher）

| 属性 | 描述 |
|------|------|
| **目标** | 实现 JSONL 文件变更监听，将新事件推送到 Event Bus |
| **输入** | T005 类型、T015 转换器 |
| **输出** | `src-tauri/src/adapter/pi_watcher.rs` |
| **依赖** | T005, T008, T015 |
| **验收标准** | 文件创建后可检测到、新行可实时读取并转换、跨平台可用 |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | ✅（与 T014 可并行） |

**详细步骤**：
1. 跨平台文件监控：
   - Linux: `inotify`
   - macOS: `fsevents`
   - Windows: 轮询（1s 间隔，因 Windows 无原生事件）
2. 实现文件追踪（记录已读取的行数，只读新增行）
3. 每行解析为 JSON，调用 T015 转换器
4. 转换后的事件通过 Event Bus 发布
5. 错误处理：解析失败时跳过该行并记录日志
6. 测试：文件追加场景、格式错误场景、文件删除后重建场景

---

## E 组：前端核心组件（5 个任务）

### T017: 状态 Hook（useAgentState）

| 属性 | 描述 |
|------|------|
| **目标** | 创建 React Hook，管理桌宠状态 |
| **输入** | T002 类型定义 |
| **输出** | `src/hooks/useAgentState.ts` |
| **依赖** | T002 |
| **验收标准** | 可获取当前状态、状态变更可监听、Tauri 事件可接收 |
| **预计工时** | 3h |
| **优先级** | P0 |
| **可并行** | ✅（与 E 组其他任务） |

**详细步骤**：
1. 使用 `useState` 管理 `petState`、`previousState`
2. 使用 `useEffect` 监听 Tauri 事件 `pet:state_changed`
3. 使用 `invoke('get_pet_state')` 获取初始状态
4. 导出 `useAgentState()` Hook
5. 添加 WebSocket 连接逻辑（可选）
6. 测试：状态变更可被正确捕获

---

### T018: SVG 宠物渲染组件（PetSVG）

| 属性 | 描述 |
|------|------|
| **目标** | 创建 SVG 宠物渲染组件，支持状态切换 |
| **输入** | T004 的皮肤资源、T017 Hook |
| **输出** | `src/components/PetSVG.tsx` |
| **依赖** | T004, T017 |
| **验收标准** | SVG 正确渲染、状态切换时动画类名正确更新、无闪烁 |
| **预计工时** | 4h |
| **优先级** | P0 |
| **可并行** | ✅（与 T019 可并行） |

**详细步骤**：
1. 创建 SVG 宠物组件（身体、头部、眼睛、嘴巴）
2. 根据 `useAgentState()` 的 petState 添加 CSS 类名
3. 添加内联 SVG 动画（眨眼、呼吸等）
4. 添加状态指示器（working 的绿色脉冲、error 的红色 X）
5. 添加拖拽支持
6. 测试：各状态渲染正确

---

### T019: 悬浮窗容器（PetWindow）

| 属性 | 描述 |
|------|------|
| **目标** | 创建悬浮窗容器组件，管理窗口行为 |
| **输入** | T011 的窗口管理逻辑 |
| **输出** | `src/components/PetWindow.tsx` |
| **依赖** | T011（部分，核心窗口创建可先用 Tauri 默认配置） |
| **验收标准** | 窗口可显示/隐藏、可拖拽、可关闭 |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | ✅（与 T018 可并行） |

**详细步骤**：
1. 创建窗口容器组件
2. 添加拖拽钩子
3. 添加窗口控制按钮（最小化到托盘、关闭）
4. 添加 CSS：透明背景、无边框、pointer-events 控制
5. 测试：拖拽行为

---

### T020: 状态文字显示组件（PetStatus）

| 属性 | 描述 |
|------|------|
| **目标** | 创建状态文字显示组件，显示当前 Agent 状态 |
| **输入** | T017 Hook |
| **输出** | `src/components/PetStatus.tsx` |
| **依赖** | T017 |
| **验收标准** | 状态文字正确显示、状态变更时有过渡动画 |
| **预计工时** | 2h |
| **优先级** | P2 |
| **可并行** | ✅ |

**详细步骤**：
1. 创建小标签组件，显示当前状态文字
2. 中文翻译：Idle=空闲、Thinking=思考中、Working=工作中...
3. 添加状态变更过渡动画（淡入淡出）
4. 可配置开关（settings.tts.show_status）
5. 测试：状态文字正确显示

---

### T021: 前端 WebSocket 客户端（wsClient）

| 属性 | 描述 |
|------|------|
| **目标** | 创建 WebSocket 客户端，连接后端 WS 服务器 |
| **输入** | T002 协议定义 |
| **输出** | `src/services/wsClient.ts` |
| **依赖** | T002 |
| **验收标准** | 可连接、认证、订阅、接收事件 |
| **预计工时** | 3h |
| **优先级** | P2 |
| **可并行** | ✅ |

**详细步骤**：
1. 实现 WebSocket 连接管理
2. 实现认证逻辑（Bearer Token）
3. 实现订阅/取消订阅
4. 实现事件路由（按类型分发）
5. 实现心跳机制
6. 实现自动重连
7. 导出为 Hook：`useWebSocket()`

---

## F 组：IPC 与集成（4 个任务）

### T022: WebSocket 服务器（WSServer）

| 属性 | 描述 |
|------|------|
| **目标** | 实现 WebSocket 服务器，为外部进程提供事件推送 |
| **输入** | T008 Event Bus、T002 协议定义 |
| **输出** | `src-tauri/src/ipc/ws_server.rs` |
| **依赖** | T008 |
| **验收标准** | 可启动/停止、可认证客户端、可推送事件、心跳正常 |
| **预计工时** | 4h |
| **优先级** | P2 |
| **可并行** | ✅（与 G 组部分任务） |

**详细步骤**：
1. 使用 `tokio-tungstenite` 实现 WebSocket 服务器
2. 监听 `127.0.0.1:8765`（可配置）
3. 实现认证：Bearer Token
4. 实现事件推送：从 Event Bus 订阅，推送到客户端
5. 实现心跳：ping/pong 每 30 秒
6. 实现速率限制
7. 测试：多客户端连接、事件推送

---

### T023: Tauri 事件桥接（Frontend Bridge）

| 属性 | 描述 |
|------|------|
| **目标** | 将 Rust Event Bus 事件桥接到 Tauri 前端事件 |
| **输入** | T008 Event Bus、T009 状态机 |
| **输出** | `src-tauri/src/lib.rs` 中的事件桥接逻辑 |
| **依赖** | T008, T009 |
| **验收标准** | Rust 端事件变更可触发前端 Tauri 事件、状态变更可通知前端 |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | ✅（与 T022 可并行） |

**详细步骤**：
1. 在 `lib.rs` 中启动 Event Bus 订阅
2. 状态变更时通过 `tauri::emit("pet:state_changed", ...)` 通知前端
3. 事件到达时通过 `tauri::emit("pet:event", ...)` 通知前端
4. 实现事件去重（避免重复通知）
5. 测试：事件从 Rust 端到前端的完整链路

---

### T024: TTS 基础引擎

| 属性 | 描述 |
|------|------|
| **目标** | 实现跨平台 TTS 引擎 |
| **输入** | 各平台 TTS 工具信息 |
| **输出** | `src-tauri/src/tts/engine.rs` |
| **依赖** | 无 |
| **验收标准** | macOS `say`、Linux `espeak` 可播报、错误可降级 |
| **预计工时** | 3h |
| **优先级** | P2 |
| **可并行** | ✅ |

**详细步骤**：
1. 检测平台（`std::env::consts::OS`）
2. 实现各平台 TTS：
   - macOS: `say` 命令
   - Linux: `espeak` 命令
   - Windows: 预留（未来用 Edge-TTS）
3. 实现播报队列（避免打断）
4. 实现静音模式（focus mode）
5. 测试：各平台播报

---

### T025: 系统集成（main.rs 组装）

| 属性 | 描述 |
|------|------|
| **目标** | 将所有模块组装到 main.rs，确保完整启动流程 |
| **输入** | T008-T024 的所有模块 |
| **输出** | `src-tauri/src/main.rs` 和 `src-tauri/src/lib.rs` |
| **依赖** | T008, T009, T014, T016, T022, T023, T024 |
| **验收标准** | `cargo tauri dev` 可完整启动、各模块正确初始化、错误可恢复 |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | 最后执行 |

**详细步骤**：
1. 初始化配置（加载 settings）
2. 初始化 Event Bus（全局单例）
3. 初始化状态机
4. 初始化 Pi Adapter（如果配置启用）
5. 启动 JSONL watcher
6. 启动 WebSocket 服务器（如果配置启用）
7. 启动 TTS 引擎（如果配置启用）
8. 初始化窗口和托盘
9. 连接各模块（Adapter → Event Bus → State Machine → Frontend）
10. 优雅关闭处理（SIGINT/SIGTERM）
11. 测试：完整启动流程

---

## G 组：基础功能（4 个任务）

### T026: 基础插件管理器

| 属性 | 描述 |
|------|------|
| **目标** | 实现基础插件系统 |
| **输入** | phase2-architecture.md 中的插件系统设计 |
| **输出** | `src-tauri/src/plugin/manager.rs` |
| **依赖** | T013（配置） |
| **验收标准** | 可加载/卸载插件、插件可获取状态、插件可订阅事件 |
| **预计工时** | 3h |
| **优先级** | P2 |
| **可并行** | ✅ |

**详细步骤**：
1. 定义 `Plugin` trait（`on_event()`, `get_name()`, `get_version()`）
2. 实现 `PluginManager`（注册、卸载、事件分发）
3. 实现皮肤插件加载（从 `~/.local/share/agent-pet-hub/plugins/` 读取）
4. 实现插件沙箱（独立作用域）
5. 测试：插件加载/卸载/事件分发

---

### T027: 前端设置管理（SettingsStore）

| 属性 | 描述 |
|------|------|
| **目标** | 创建前端设置状态管理 |
| **输入** | T013 配置结构 |
| **输出** | `src/store/settingsStore.ts` |
| **依赖** | T013（类型定义） |
| **验收标准** | 设置可读写、状态变更可响应、持久化正确 |
| **预计工时** | 2h |
| **优先级** | P2 |
| **可并行** | ✅ |

**详细步骤**：
1. 使用 `useReducer` 或 `zustand` 管理设置状态
2. 实现 `loadSettings()` 调用 Tauri invoke
3. 实现 `updateSetting()` 调用 Tauri invoke
4. 本地缓存设置
5. 测试：设置读写

---

### T028: 前端宠物状态管理（PetStore）

| 属性 | 描述 |
|------|------|
| **目标** | 创建前端宠物状态状态管理 |
| **输入** | T002 类型定义、T017 Hook |
| **输出** | `src/store/petStore.ts` |
| **依赖** | T002 |
| **验收标准** | 状态可获取、变更可响应、动画类名可更新 |
| **预计工时** | 2h |
| **优先级** | P2 |
| **可并行** | ✅ |

**详细步骤**：
1. 使用 zustand 管理宠物状态
2. 实现 `setPetState()`、`getPetState()`
3. 实现状态变更监听
4. 集成 T017 Hook
5. 测试：状态管理正确

---

### T029: 前端 WebSocket Hook（useWebSocket）

| 属性 | 描述 |
|------|------|
| **目标** | 创建 WebSocket 连接的 React Hook |
| **输入** | T021 的 wsClient |
| **输出** | `src/hooks/useWebSocket.ts` |
| **依赖** | T021 |
| **验收标准** | 可连接、认证、订阅、接收事件 |
| **预计工时** | 2h |
| **优先级** | P2 |
| **可并行** | ✅ |

**详细步骤**：
1. 封装 T021 的 wsClient 为 React Hook
2. 自动连接/重连
3. 自动认证
4. 事件订阅管理
5. 连接状态可视化（connected/disconnecting/error）

---

## H 组：集成与测试（5 个任务）

### T030: 端到端集成测试 — Pi → Event → Pet

| 属性 | 描述 |
|------|------|
| **目标** | 验证完整链路：Pi Agent 事件 → Adapter → State Machine → Pet Animation |
| **输入** | T014-T025 的所有模块 |
| **输出** | 端到端测试报告、演示视频 |
| **依赖** | T001-T025 |
| **验收标准** | Pi 启动后，桌宠正确响应 Thinking/Working/Idle 状态切换 |
| **预计工时** | 4h |
| **优先级** | P0 |
| **可并行** | 最后执行 |

**详细步骤**：
1. 准备测试环境：安装 Pi Extension（T003）
2. 启动 Agent-Pet-Hub
3. 在 Pi Agent 中执行命令触发工具调用
4. 验证桌宠状态变化：
   - Pi 启动 → Thinking
   - Pi 调用工具 → Working
   - Pi 工具完成 → Thinking
   - Pi 会话结束 → Idle
5. 录制演示视频
6. 编写集成测试文档

---

### T031: Rust 单元测试完善

| 属性 | 描述 |
|------|------|
| **目标** | 完善 Rust 端单元测试，覆盖核心模块 |
| **输入** | T005-T025 的所有 Rust 模块 |
| **输出** | `cargo test` 覆盖率 > 60% |
| **依赖** | T005-T025 |
| **验收标准** | 所有公共 API 有测试、边界条件有测试 |
| **预计工时** | 4h |
| **优先级** | P1 |
| **可并行** | 可与 T030 并行 |

**详细步骤**：
1. 为 T005（类型定义）添加序列化/反序列化测试
2. 为 T006（验证器）添加合法/非法输入测试
3. 为 T007（映射）添加完整映射测试
4. 为 T009（状态机）添加所有转换路径测试
5. 为 T015（转换器）添加各事件类型测试
6. 为 T022（WS Server）添加消息格式测试
7. 为 T024（TTS）添加平台检测测试
8. 运行 `cargo test -- --test-threads=1`
9. 查看覆盖率：`cargo tarpaulin`

---

### T032: TypeScript 测试

| 属性 | 描述 |
|------|------|
| **目标** | 创建前端和协议包的单元测试 |
| **输入** | T002、T017-T021、T026-T029 |
| **输出** | `pnpm test` 通过、ESLint 零错误 |
| **依赖** | T002、T017-T021、T026-T029 |
| **验收标准** | ESLint 零错误、Zod schemas 有测试 |
| **预计工时** | 3h |
| **优先级** | P1 |
| **可并行** | 可与 T031 并行 |

**详细步骤**：
1. 配置 Vitest 测试框架
2. 为 `packages/protocol/src/schemas.ts` 添加测试
3. 为 `src/hooks/useAgentState.ts` 添加 Hook 测试（@testing-library/react）
4. 为 `src/store/petStore.ts` 添加 Store 测试
5. ESLint 配置 + 运行
6. Prettier 格式化
7. 修复所有 lint 错误

---

### T033: 跨平台构建验证

| 属性 | 描述 |
|------|------|
| **目标** | 在 Linux/macOS/Windows 上验证构建 |
| **输入** | 所有代码完成 |
| **输出** | 各平台构建产物、安装说明 |
| **依赖** | T030-T032 |
| **验收标准** | Linux/macOS/Windows 均可 `cargo tauri build` 成功 |
| **预计工时** | 4h |
| **优先级** | P2 |
| **可并行** | 最后执行 |

**详细步骤**：
1. Linux（当前环境）：`cargo tauri build`
2. macOS（如有）：`cargo tauri build`
3. Windows（如有）：`cargo tauri build`
4. 修复各平台特定问题
5. 生成安装文档
6. 创建 release artifact

---

### T034: 文档与 README

| 属性 | 描述 |
|------|------|
| **目标** | 创建项目文档、README、安装指南 |
| **输入** | 所有代码和文档 |
| **输出** | `README.md`、`docs/` 目录完整、安装指南 |
| **依赖** | T030-T033 |
| **验收标准** | README 可指导新用户完成安装和运行、所有 API 有文档 |
| **预计工时** | 3h |
| **优先级** | P2 |
| **可并行** | 最后执行 |

**详细步骤**：
1. 编写 `README.md`（项目介绍、安装、使用、架构）
2. 编写安装指南（各平台）
3. 编写 API 文档（Tauri Commands、WebSocket）
4. 编写贡献指南
5. 创建 `.github/` 模板（issue、PR）
6. 添加许可证（MIT）

---

## 任务依赖图（文字版）

```
T001 ─┐
T002 ─┼──→ T005 ─→ T006 ─┐
     │     T007 ─────────┤
     │                    ▼
     │              T008 ─→ T022 ─┐
     │              T009 ─→ T010  │
     │              T011 ─┐        ▼
     │              T012 ─┼──→ T023 ─→ T025 ─→ T030
     │              T013 ─┘        ▲
     │                    T014 ─┘  │
     │                    T015 ────┤
     │                    T016 ────┤
     │                    T024 ────┤
     │                    T026 ────┤
     │                    T027 ────┤
     │                    T028 ────┤
     │                    T029 ────┘
     │
T003 ────────────────────────────────┘
     （Pi Extension）
     
T004 ─→ T018 ─→ T030
     │
T017 ─→ T018 ─┘

T021 ─→ T029
```

---

## 并行任务组（可同时进行）

### 并行组 1（Day 1-2）
| 任务 | 负责人 | 说明 |
|------|--------|------|
| T001 | 主代理 | Tauri 初始化 |
| T002 | 子代理 | 协议包 |
| T004 | 子代理 | 皮肤资源 |
| T013 | 子代理 | 配置读写 |

### 并行组 2（Day 2-3）
| 任务 | 负责人 | 说明 |
|------|--------|------|
| T005 | 子代理 | Rust 类型 |
| T006 | 子代理 | 验证器 |
| T007 | 子代理 | 映射表 |
| T008 | 主代理 | Event Bus |
| T009 | 主代理 | 状态机 |
| T010 | 子代理 | 转换表 |

### 并行组 3（Day 3-5）
| 任务 | 负责人 | 说明 |
|------|--------|------|
| T011 | 子代理 | 窗口管理 |
| T012 | 子代理 | Tauri Commands |
| T014 | 主代理 | Pi 适配器 |
| T015 | 主代理 | 事件转换器 |
| T016 | 主代理 | JSONL 监听 |
| T017 | 子代理 | 状态 Hook |
| T018 | 子代理 | SVG 渲染 |
| T019 | 子代理 | 悬浮窗容器 |

### 并行组 4（Day 5-7）
| 任务 | 负责人 | 说明 |
|------|--------|------|
| T020 | 子代理 | 状态文字组件 |
| T021 | 子代理 | WebSocket 客户端 |
| T022 | 主代理 | WebSocket 服务器 |
| T023 | 主代理 | 事件桥接 |
| T024 | 子代理 | TTS 引擎 |
| T026 | 子代理 | 插件管理器 |
| T027 | 子代理 | 设置 Store |
| T028 | 子代理 | 宠物 Store |
| T029 | 子代理 | WebSocket Hook |

### 串行任务（最后执行）
| 任务 | 依赖 |
|------|------|
| T025（系统集成） | T008, T009, T014, T016, T022, T023, T024 |
| T030（端到端测试） | T001-T025 |
| T031（Rust 测试） | T005-T025 |
| T032（TS 测试） | T002, T017-T021, T026-T029 |
| T033（跨平台构建） | T030-T032 |
| T034（文档） | T030-T033 |

---

## 时间线（14 天计划）

| 天 | 任务 | 累计工时 |
|----|------|----------|
| 1 | T001, T002, T004, T013 | 9h |
| 2 | T003, T005, T006, T007 | 10h |
| 3 | T008, T009, T010 | 9h |
| 4 | T011, T012, T014, T015, T016 | 17h |
| 5 | T017, T018, T019, T020, T021 | 14h |
| 6 | T022, T023, T024, T026, T027, T028, T029 | 17h |
| 7 | T025（系统集成） | 3h |
| 8-9 | T030（端到端测试）、T031（Rust 测试） | 8h |
| 10 | T032（TS 测试） | 3h |
| 11 | 修复 bug、性能优化 | 4h |
| 12 | T033（跨平台构建） | 4h |
| 13 | T034（文档） | 3h |
| 14 | 缓冲日（bug 修复、打磨） | 4h |

**总计**：81 小时，约 14 个工作日

---

## 任务拆分说明

### 为什么拆分到 34 个任务？

1. **粒度适中**：每个任务 2-4 小时，子代理可以独立完成
2. **依赖清晰**：任务之间有明确的先后关系
3. **并行度高**：同一分组内任务可并行执行
4. **可测试**：每个任务完成后可独立验证

### 哪些任务可以用子代理完成？

| 任务 | 子代理可行性 | 说明 |
|------|-------------|------|
| T002 | ✅ | 纯类型定义，无复杂逻辑 |
| T003 | ✅ | 模板化扩展代码 |
| T004 | ✅ | 静态资源创建 |
| T005-T007 | ✅ | 类型和映射，无运行时逻辑 |
| T010 | ✅ | 纯数据定义 |
| T011 | ✅ | Tauri 窗口配置 |
| T012 | ✅ | Tauri Commands |
| T013 | ✅ | 配置读写 |
| T017-T021 | ✅ | 前端组件 |
| T020, T027-T029 | ✅ | 前端辅助组件 |
| T024 | ✅ | TTS 跨平台检测 |
| T026 | ✅ | 插件管理器 |
| T032 | ✅ | TypeScript 测试 |

| 任务 | 主代理负责 | 说明 |
|------|-----------|------|
| T001 | ✅ | Tauri 项目初始化（需要决策） |
| T008 | ✅ | Event Bus（核心模块） |
| T009 | ✅ | 状态机（核心模块） |
| T014-T016 | ✅ | Pi 适配器链（需要协调） |
| T022 | ✅ | WebSocket 服务器（需要调试） |
| T023 | ✅ | 事件桥接（需要调试） |
| T025 | ✅ | 系统集成（最后组装） |
| T030 | ✅ | 端到端测试（需要验证） |
| T031 | ✅ | Rust 测试（需要理解代码） |
| T033 | ✅ | 跨平台构建（需要手动验证） |
| T034 | ✅ | 文档（需要理解全貌） |

---

## 下一步：第六阶段 — 编码

完成本阶段后，按以下顺序进入编码：

1. **确认本任务拆分** → 用户确认
2. **并行启动并行组 1** → T001, T002, T004, T013
3. **并行启动并行组 2** → 依赖满足后
4. **...** → 依次推进
5. **系统集成 T025** → 所有前置完成
6. **测试与修复** → T030-T032
7. **发布 MVP** → T033-T034

---

*本任务拆分为项目执行路线图，严格按依赖顺序执行。*
