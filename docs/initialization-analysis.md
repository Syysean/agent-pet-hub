# 初始化流程与错误处理分析

## 现状总结

### 初始化顺序
```
1. SettingsManager 创建 & 加载配置
   ├─ 依赖: 文件系统
   └─ 错误处理: ✓ warn 级别，使用默认值继续

2. EventBus 创建
   ├─ 依赖: 无
   └─ 错误处理: ✗ 无任何处理（critical）

3. PetStateMachine 创建
   ├─ 依赖: 无
   └─ 错误处理: ✗ 无任何处理（critical）

4. init_globals() 注册全局单例
   ├─ 依赖: EVENT_BUS, STATE_MACHINE, SETTINGS 的 Mutex
   └─ 错误处理: ✗ 可能 panic（若 mutex 被中毒）

5. create_pet_window()
   ├─ 依赖: Tauri AppHandle
   └─ 错误处理: ✓ error 级别，但继续启动

6. create_tray()
   ├─ 依赖: Tauri AppHandle, 可能需要宠物状态
   └─ 错误处理: ✓ error 级别，但继续启动

7. PiAdapter::connect() & start_listening()
   ├─ 依赖: EventBus, StateM, TTS（可选）
   └─ 错误处理: ✓ warn 级别，后台任务失败不影响启动

8. WSServer::run()
   ├─ 依赖: EventBus
   └─ 错误处理: ✓ error 级别，后台任务失败不影响启动

9. 后续操作
   └─ 事件循环处理
```

---

## 问题分析

### 🔴 CRITICAL - 无降级处理

#### 1. **全局单例初始化缺乏错误处理**
**位置**: `lib.rs` line 46-47, `commands.rs` line 35-43

```rust
// 当前代码
pub fn init_globals(
    event_bus: EventBus,
    state_machine: PetStateMachine,
    settings: SettingsManager,
) {
    let mut bus = EVENT_BUS.lock().unwrap();  // ❌ 可能 panic
    *bus = Some(event_bus);
    // ...
}
```

**问题**：
- 如果 Mutex 被另一个线程 panic 中毒，调用 `.unwrap()` 会 panic
- Tauri 应用主线程崩溃导致应用无法启动

**后果**：
- 无法优雅降级，直接应用 crash

---

#### 2. **EventBus 和 StateM 创建无异常处理**
**位置**: `lib.rs` line 39-40

```rust
let event_bus = EventBus::new(1024);       // ❌ 可能分配失败
let state_machine = PetStateMachine::new(); // ❌ 可能初始化失败
```

**问题**：
- 如果内存分配失败，Rust 会 panic
- 如果 StateM 初始化有复杂逻辑（未来扩展），可能失败

**后果**：
- 无法捕获和处理初始化失败
- 无法降级到最小可行状态

---

### 🟡 HIGH - 优雅降级不完整

#### 3. **窗口创建失败后，应用无法恢复**
**位置**: `lib.rs` line 57-59

```rust
if let Err(e) = window::create_pet_window(app.handle()) {
    tracing::error!("Failed to create pet window: {}", e);
}
```

**问题**：
- 只记录 error，不检查是否关键
- 托盘可能也会失败（共享资源）
- 前端无法判断窗口是否成功创建

**后果**：
- 前端无法渲染（窗口不存在）
- 用户无法与应用交互
- 应该至少提供一个备选窗口或警告

---

#### 4. **缺少依赖链验证**
**位置**: `lib.rs` line 65-87

```rust
// Pi 适配器和 WebSocket 都依赖 EventBus，但未验证
let adapter = adapter::PiAdapter::new(
    adapter_config,
    event_bus_clone,  // ❌ 未检查是否成功克隆/初始化
    // ...
);

let ws_server = WSServer::new(
    event_bus.event_tx().clone(),  // ❌ 未检查 channel 是否可用
    // ...
);
```

**问题**：
- 如果 EventBus 内部状态不正确，后续服务会失败
- 没有依赖链的完整性检查
- 后台任务失败了，前端无法感知

**实际验证**：
- `PiAdapter` 已经使用 `Arc<Mutex<PetStateMachine>>`
- `lib.rs` 当前依然为 `PiAdapter` 新建了一个独立 `PetStateMachine` 实例
- `commands.rs` 还保存着另一套 `Mutex<Option<PetStateMachine>>`
- 这说明问题不是“是否共享”，而是“共享模型本身不统一”

---

#### 5. **缺少启动完成标志**
**位置**: `lib.rs` 全局无状态

```rust
// ❌ 无法知道应用是否完全初始化
// 前端可能在初始化完成前调用命令
```

**问题**：
- 前端无法判断后端是否准备好
- 竞态条件：前端可能在后台任务启动前查询状态
- 没有健康检查接口

---

### 🟠 MEDIUM - 设计缺陷

#### 6. **Lazy Static Mutex 被中毒后无恢复机制**
**位置**: `commands.rs` line 18-22

```rust
lazy_static! {
    static ref EVENT_BUS: Mutex<Option<EventBus>> = Mutex::new(None);
    // ...
}
```

**问题**：
- 如果任何地方 panic，这些 Mutex 会被中毒（poisoned）
- 所有后续访问都会 panic
- 无法从 panic 恢复

**建议**：
- 使用 `RwLock` 或 `tokio::sync::Mutex` 处理中毒
- 实现 panic recovery

---

#### 7. **缺少初始化模式验证**
**位置**: `lib.rs` line 43-50

```rust
// 当前模式：所有依赖都在 setup 中初始化
// 问题：setup 错误后无法重试
```

**问题**：
- setup 是 one-shot 的，失败了无法再次运行
- 没有 "retry" 或 "reload" 机制
- 配置加载失败后无法在运行时重新加载

---

#### 8. **无统一错误上报机制**
**位置**: `lib.rs` 全文

```rust
// 错误处理分散，没有统一的收集机制
tracing::warn!("Failed to load settings...");  // 位置 1
tracing::error!("Failed to create pet window");  // 位置 2
tracing::warn!("Pi adapter connect error...");    // 位置 3
// ❌ 前端无法通过 API 查看这些错误
```

**问题**：
- 错误只在日志中，前端无法访问
- 无法向用户报告初始化问题
- 无法决策是否要求用户干预

---

## 初始化状态转移图

```
┌────────────────────┐
│    启动应用        │
└─────────┬──────────┘
          │
          ▼
┌────────────────────┐
│  加载配置文件       │  ◄─────── [OK] 或使用默认值
│  (SettingsManager) │
└─────────┬──────────┘
          │
          ▼
┌────────────────────┐
│  创建 EventBus      │  ◄─────── [FAIL] ❌ 无处理 → CRASH
└─────────┬──────────┘
          │
          ▼
┌────────────────────┐
│  创建 StateM       │  ◄─────── [FAIL] ❌ 无处理 → CRASH
└─────────┬──────────┘
          │
          ▼
┌────────────────────┐
│  init_globals()    │  ◄─────── [FAIL] ❌ panic → CRASH
└─────────┬──────────┘
          │
          ▼
┌────────────────────┐
│  创建窗口 & 托盘   │  ◄─────── [FAIL] ✓ 记录 error，继续
└─────────┬──────────┘
          │
          ▼
┌────────────────────┐
│  启动 Pi Adapter   │  ◄─────── [FAIL] ✓ warn，后台任务无影响
│  启动 WSServer     │
└─────────┬──────────┘
          │
          ▼
┌────────────────────┐
│  应用启动完成      │  ❌ 无状态标志
│  (无法判断是否成功) │
└────────────────────┘
```

---

## 最佳实践建议

### 1. **统一状态机共享模型**
- 现状：`PiAdapter` 使用 `Arc<Mutex<PetStateMachine>>`，但 `commands.rs` 仍保存 `Mutex<Option<PetStateMachine>>`
- 修正：全局只保留一个 `Arc<Mutex<PetStateMachine>>`，`commands`、`PiAdapter`、以及后续健康检查都引用同一份实例
- 目标：消除“看上去是同一个对象，实际上是两份”的错误共享模型

### 2. **修复 `init_globals` 的锁错误**
- 不再使用 `unwrap()`
- 可用写法：
```rust
let mut bus = EVENT_BUS.lock().unwrap_or_else(|p| p.into_inner());
*bus = Some(event_bus);
```
- 更好：用 `RwLock` 或 `tokio::sync::Mutex`，并把全局对象包装在一个 `GlobalServices` 结构里

### 3. **将命令端改成 async，使用 tokio 锁**
- 如果状态机全局改为 `Arc<tokio::sync::Mutex<PetStateMachine>>`，则命令应改为：
```rust
#[command]
pub async fn get_pet_state() -> Result<PetState, String> {
    let sm = STATE_MACHINE.lock().await;
    // ...
}
```
- 这样避免“同步命令 + 异步锁”的混乱

### 4. **窗口 / 托盘错误处理应只执行一次并记录状态**
- 正确模式：
```rust
let window_result = create_pet_window(app.handle());
match window_result {
    Ok(_) => {}
    Err(e) => {
        tracing::error!("...", e = %e);
        startup_status.note_failure("pet_window", e.to_string());
    }
}
```
- 避免 `create_pet_window(...).is_ok()` 这种重复调用

### 5. **避免不必要的“是否初始化”检查**
- `lib.rs` 中已有 `event_bus` 局部变量，后续传递给 `PiAdapter` / `WSServer` 即可
- 不要再去检查 `commands::EVENT_BUS` 是否初始化，这不是实际问题点

### 6. **TTS 配置要么接入、要么标记未完成**
- 现状：`tts_enabled` 已读取，但 `PiAdapter::new(..., None)` 仍传 `None`
- 修正：要么传入真实 `TTSEngine`，要么先把 TTS 开关视为“功能未实施”并暂不暴露

### 7. **显式设置托盘菜单 id**
- 现状：`MenuItem::new(app, "显示桌宠", true, None::<&str>)` 可能没有 `id`
- 应该显式给 `show/hide/quit` 设置 id，避免事件匹配失败

### 8. **优先收敛策略：三个轻量动作**
1. 统一状态机为单一 `Arc<Mutex<PetStateMachine>>`
2. 修掉 `init_globals` 的 `unwrap()` 并统一 `commands` 为 async 访问
3. 用轻量 `StartupStatus` 记录 Ready / Degraded / Failed 以及失败组件列表

---

## 优先级排序

| 优先级 | 问题 | 影响 | 工作量 |
|--------|------|------|--------|
| P0 | EventBus 创建无异常处理 | 应用 crash | 小 |
| P0 | 全局单例初始化无错误处理 | 应用 crash | 小 |
| P1 | 缺少启动完成标志 | 前端竞态条件 | 中 |
| P1 | 缺少健康检查接口 | 无法诊断 | 中 |
| P2 | Mutex 中毒恢复 | 后续请求 panic | 大 |
| P2 | 配置热重载 | 用户体验 | 大 |
| P3 | 统一错误上报 | 调试困难 | 中 |

