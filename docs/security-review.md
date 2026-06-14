# Agent-Pet-Hub 全面安全审查报告

> 审查依据：`docs/initialization-analysis.md` + 全代码审计  
> 审查日期：2026-06-10  
> 审查范围：Rust 后端 (src-tauri/src/) + 前端 (src/) + 协议包 (packages/protocol/)

---

## 一、初始化分析文档问题的解决状态

### ✅ 已解决的问题

| # | 问题 | 解决状态 | 证据 |
|---|------|---------|------|
| 1 | `init_globals()` 的 `unwrap()` 可能 panic | ✅ 已修复 | `commands.rs` 使用 `.unwrap_or_else(\|e\| e.into_inner())` 中毒恢复 |
| 2 | STATE_MACHINE 多实例混乱 | ✅ 已修复 | `commands.rs` 中 `STATE_MACHINE` 为 `Arc<TokioMutex<>>`，lib.rs 中通过 `.clone()` 复用同一 Arc |
| 3 | 命令端同步/异步锁混乱 | ✅ 已修复 | `get_pet_state()`, `send_event()` 等改为 `#[command] pub async fn` + `tokio::sync::Mutex::lock().await` |

### ⚠️ 部分解决 / 待改进

| # | 问题 | 当前状态 | 剩余风险 |
|---|------|---------|---------|
| 4 | 窗口/托盘创建失败后无降级 | ⚠️ 记录 error 但继续 | 前端渲染窗口不存在时无任何反馈 |
| 5 | 缺少启动完成标志 | ⚠️ 仅有 `info!` 日志 | 前端无法通过 API 判断后端是否就绪 |
| 6 | 无统一错误上报机制 | ❌ 未解决 | 错误只在日志中，前端不可见 |
| 7 | TTS 传入 None | ❌ 未解决 | `lib.rs` 中 `tts_engine` 创建后传给 PiAdapter，但 `PiAdapter` 的 `start_listening` 调用时可能已丢失 |

### ❌ 未解决的问题

| # | 问题 | 位置 | 影响 |
|---|------|------|------|
| 8 | EventBus 创建无异常处理 | `lib.rs` line 40 | 内存分配失败 → crash（但 Rust OOM 本身有 backtrace） |
| 9 | 依赖链验证缺失 | `lib.rs` line 103-145 | EventBus 内部状态不一致时服务失败但前端无感知 |
| 10 | 配置热重载 | `config/settings.rs` | 修改配置后需重启应用 |

---

## 二、新增安全审查问题（代码审计发现）

### 🔴 CRITICAL — 安全漏洞

#### C1. TTS 命令注入
**位置**: `src-tauri/src/tts/engine.rs` line 92-115

```rust
// Linux: espeak -s 150 -v zh-cn "text"
let _ = Command::new("espeak")
    .args(["-s", &speed.to_string(), "-v", "zh-cn", text])
    .spawn();
```

**问题**:
- `text` 参数直接拼接进命令行，没有 sanitization
- Pi Agent 可以输出含引号、分号、反引号等特殊字符的文本
- 例如 `text = 'hello; rm -rf /'` 可能被 shell 解析
- macOS `say` 命令同样存在此问题

**严重程度**: 高  
**利用条件**: Pi Agent 输出特殊字符 → 触发 TTS 播报

**修复方案**:
```rust
// 使用 shell_escape 或手动转义
use shell_escape::escape;
let escaped = escape(Cow::Borrowed(text));
Command::new("espeak")
    .args(["-s", &speed.to_string(), "-v", "zh-cn"])
    .arg(text)  // espeak 本身会处理参数，不通过 shell
    .spawn();
```

---

#### C2. 配置文件中 WS Token 明文存储
**位置**: `src-tauri/src/config/settings.rs` line 96, `config.json`

```rust
fn default_ws_token() -> String {
    "agent-pet-hub".to_owned()
}
// 配置文件路径: ~/.config/agent-pet-hub/config.json
```

**问题**:
- 配置文件权限默认 644，同用户可读取
- Token 明文存储在 JSON 文件中
- 无过期机制、无轮换

**严重程度**: 中  
**利用条件**: 读取同用户的 config.json → 获取 WS Token → 连接 WebSocket 服务

**修复方案**:
1. 配置文件权限设为 600
2. 支持从环境变量读取 token: `os.environ.get("PET_HUB_WS_TOKEN")`
3. 实现 token 轮换机制

---

#### C3. `send_event` 命令暴露完整 raw payload
**位置**: `src-tauri/src/commands.rs` line 109-117

```rust
#[command]
pub async fn send_event(event: UnifiedAgentEvent) -> Result<usize, String> {
    let bus = EVENT_BUS.lock().map_err(|e| e.to_string())?;
    let event_bus = bus.as_ref().ok_or("Event bus not initialized")?;
    event_bus.publish_event(event).map_err(|e| e.to_string())
}
```

**问题**:
- `UnifiedAgentEvent` 包含 `raw: serde_json::Value` 字段
- 前端可以发送任意大小的 raw JSON（无大小限制）
- 大量数据可能撑爆 EventBus 的 broadcast channel（默认 1024）

**严重程度**: 中  
**利用条件**: 前端调用 `invoke("send_event", event)` → 注入大 payload

**修复方案**:
```rust
// 在 publish 前校验 payload 大小
if event.raw.to_string().len() > 1024 * 1024 {  // 1MB 限制
    return Err("Event payload too large".into());
}
```

---

### 🟡 HIGH — 功能安全

#### H1. EventBus 广播通道溢出风险
**位置**: `src-tauri/src/event_bus/mod.rs` line 62-63

```rust
pub fn new(channel_size: usize) -> Self {
    let (event_tx, _) = broadcast::channel(channel_size);  // 1024
```

**问题**:
- `broadcast::channel(1024)` 容量有限
- 当 Pi Agent 高速产生事件（如快速循环工具调用）时，旧事件会被丢弃
- 丢失的事件包含状态变更，导致前端动画不一致
- 当前日志只 warn `lagged = n`，但没有通知前端

**严重程度**: 高  
**触发条件**: Pi Agent 事件速率 > 消费者处理速率

**修复方案**:
1. 增大 channel 容量到 4096+
2. 在 `EventBus` 中添加 `lagged` 回调机制
3. 前端监听 `pet:lagged` 事件后重新同步状态

---

#### H2. WebSocket 服务器无连接限制
**位置**: `src-tauri/src/ipc/ws_server.rs` line 54-68

```rust
loop {
    let (stream, addr) = listener.accept().await?;
    // 每次 accept 都 spawn 一个任务
    tokio::spawn(async move {
        Self::handle_client(stream, token, event_tx).await
    });
}
```

**问题**:
- 无最大连接数限制
- 无连接速率限制（burst protection）
- 每个连接持有 `broadcast::Receiver`，内存随连接数增长
- 连接数过多时可能耗尽内存

**严重程度**: 中  
**利用条件**: 外部进程快速创建大量 WS 连接 → 内存耗尽

**修复方案**:
```rust
use std::sync::atomic::{AtomicUsize, Ordering};
static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

// 在 accept 前检查
if ACTIVE_CONNECTIONS.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
    warn!("Max connections reached, rejecting");
    continue;
}
```

---

#### H3. 事件监听器与适配器竞争状态
**位置**: `src-tauri/src/lib.rs` line 130-148

```rust
// 事件监听器在适配器启动前就开始运行
let event_bus_for_listener = crate::commands::EVENT_BUS.lock().ok().and_then(|g| g.clone());
if let Some(eb) = event_bus_for_listener {
    let mut rx = eb.subscribe_event();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = rx.recv().await { ... }
    });
}
```

**问题**:
- 事件监听器在 `PiAdapter::start_listening()` 之前就已经启动
- 这意味着 `PiAdapter::connect()` 发出的 `AdapterConnected` 事件可能被监听器消费
- 但此时状态机还未处理过该事件（因为监听器消费事件后需要等下次 recv）
- 实际上事件会正常流转（因为监听器和适配器都消费同一个 EventBus），但**日志顺序可能混乱**

**严重程度**: 低（功能上正确，但日志可读性差）

---

#### H4. 前端 `useAgentState` 的 listen 没有错误处理
**位置**: `src/hooks/useAgentState.ts` line 39-51

```typescript
useEffect(() => {
    const setup = async () => {
        unlisten1 = await listen<PetState>("pet:state_changed", (event) => {
            console.log("pet:state_changed", event.payload);
            setPetState(event.payload);
        });
        unlisten2 = await listen("pet:event", (event) => {
            console.log("pet:event", event.payload);
        });
    };
    setup().catch(console.error);  // ❌ 如果 setup 失败，没有重试机制
    return () => { unlisten1?.(); unlisten2?.(); };
}, [setPetState]);
```

**问题**:
- 如果 Tauri 窗口还未完全初始化，`listen()` 可能失败
- 失败后不重试，Tauri 事件永远收不到
- React 重渲染时 `setPetState` 引用变化 → 重新建立 listen → 可能重复注册

**严重程度**: 高（与本次 bug 直接相关）  
**修复方案**: 添加 retry 逻辑 + 使用 `setPetState` 的 stable 版本

---

### 🟠 MEDIUM — 设计改进

#### M1. `EventConverter` 未限制 `raw` 字段大小
**位置**: `src-tauri/src/adapter/event_converter.rs`

```rust
fn make_event(...) -> UnifiedAgentEvent {
    UnifiedAgentEvent {
        // ...
        raw,  // 原始 JSON，无大小限制
        // ...
    }
}
```

**问题**: 每个事件携带完整原始 JSON，如果 Pi Agent 输出大文件内容（如 `cat /var/log/syslog`），每个事件都会携带几 MB 数据

**修复方案**: 截断 `raw` 字段到 8KB

---

#### M2. 托盘菜单事件 ID 使用字符串
**位置**: `src-tauri/src/window/tray.rs` line 25

```rust
let show_menu = MenuItem::new(app, "显示桌宠", true, Some("show"))?;
```

**问题**: `Some("show")` 作为 C-string，但 Tauri 2.x 的 `on_menu_event` 回调使用 `event.id().0.as_str()`，如果菜单项顺序改变，匹配可能出错

**修复方案**: 使用 `Id::new("show")` 而非裸字符串

---

#### M3. `toggle_pet_window` 不检查窗口是否存在
**位置**: `src-tauri/src/commands.rs` line 123-133

```rust
#[command]
pub fn toggle_pet_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("pet-window") {
        // 窗口不存在时返回 Ok(()) — 静默成功
        // 前端无法区分"窗口已隐藏"和"窗口不存在"
    }
    Ok(())
}
```

**修复方案**: 返回明确的错误码

---

#### M4. Pi 适配器 send_message 直接写入 JSONL 文件
**位置**: `src-tauri/src/adapter/pi_adapter.rs` line 160-182

```rust
let log_entry = format!("{}\n", event_json);
tokio::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)
    .await
```

**问题**: 没有文件锁保护，多进程同时写入可能交错  
**修复方案**: 使用 `filelock` crate 或 FIFO 管道

---

#### M5. WebSocket 服务器 `subscribe` 不验证事件类型
**位置**: `src-tauri/src/ipc/ws_server.rs` line 108-119

```rust
"subscribe" => {
    // 客户端订阅事件类型
    let resp = serde_json::json!({
        "type": "subscribed",
        "payload": {
            "subscriptionId": uuid::Uuid::new_v4().to_string(),
            "eventTypes": ["*"]  // ❌ 固定返回 "*"，忽略客户端实际订阅的请求
        }
    });
```

**问题**: 客户端发送的 `eventTypes` 数组被完全忽略，所有客户端收到所有事件

---

#### M6. 前端 `PetSVG` 的 `STATE_CLASS_MAP` 未覆盖 `connecting` 状态
**位置**: `src/components/PetSVG.tsx` line 31-38

```typescript
const STATE_CLASS_MAP: Record<PetState, string> = {
    idle: "pet-idle",
    thinking: "pet-thinking",
    working: "pet-working",
    waiting: "pet-waiting",
    success: "pet-success",
    error: "pet-error",
    speaking: "pet-speaking",
    connecting: "pet-connecting",  // ✅ 有
};
```

实际上 8 种状态都覆盖了 ✅，但 `index.css` 中 `.pet-connecting` 的动画是 `rotate(360deg)` 全旋转，视觉上可能被用户注意到窗口旋转。

---

### 🟢 LOW — 优化项

| # | 问题 | 影响 |
|---|------|------|
| L1 | `lib.rs` 中 `use tauri::Manager;` 未使用 | 编译警告 |
| L2 | `EventConverter` 中 `truncate` 方法重复定义 | 代码冗余 |
| L3 | Pi Adapter 的 `send_message` 返回模拟回复 | 前端无法区分真实回复 |
| L4 | `PiJsonlWatcher` 中 `stop()` 无跨线程同步 | `running` 是普通 bool，无原子/锁保护 |
| L5 | WebSocket 服务器 `ping` 没有 pong 超时检测 | 僵尸连接不释放 |

---

## 三、修复方案优先级

| 优先级 | 问题编号 | 问题 | 工作量 | 方案 |
|--------|---------|------|--------|------|
| P0 | H4 | `useAgentState` listen 无错误处理 | 小 | 添加 setup 重试 + 稳定引用 |
| P0 | C1 | TTS 命令注入 | 小 | 使用 `Command::args()` 而非 shell 拼接 |
| P1 | C2 | WS Token 明文存储 | 小 | config.json 设 600 权限 + 环境变量覆盖 |
| P1 | H1 | EventBus 广播通道溢出 | 中 | 增大 channel + 添加 lagged 通知 |
| P1 | H2 | WS 服务器无连接限制 | 中 | 添加 `MAX_CONNECTIONS` 限制 |
| P2 | C3 | send_event raw 大小限制 | 小 | 添加 payload 大小校验 |
| P2 | C3 (cont) | EventConverter raw 大小 | 小 | 截断到 8KB |
| P2 | M4 | JSONL 文件锁 | 中 | 添加 `filelock` |
| P3 | M5 | WS subscribe 忽略事件类型 | 小 | 解析并过滤客户端订阅 |
| P3 | M6 | connecting 动画过于明显 | 小 | 降低旋转速度 |

---

## 四、实施计划

### Phase 1 — 立即修复（1-2 小时）

1. **`useAgentState.ts`** — 添加 listen retry 和 error boundary
2. **`tts/engine.rs`** — 使用 `Command::args()` 避免 shell 注入
3. **`config/settings.rs`** — `save()` 时设置文件权限 0o600

### Phase 2 — 本周内（1-2 天）

4. **`event_bus/bus.rs`** — 增大 channel 到 4096，添加 lagged 通知
5. **`ipc/ws_server.rs`** — 添加 `MAX_CONNECTIONS` + `subscribe` 过滤
6. **`adapter/event_converter.rs`** — 截断 raw 到 8KB
7. **`commands.rs`** — `send_event` 添加 payload 大小校验

### Phase 3 — 后续迭代

8. **`adapter/pi_adapter.rs`** — JSONL 文件锁
9. **WebSocket pong 超时检测**
10. **启动状态 API** — 返回 Ready/Degraded/Failed 状态
