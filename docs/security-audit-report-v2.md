# 🔒 Agent-Pet-Hub 完整安全审计报告（v2）

**审计日期**: 2026-06-10  
**审计模式**: 安全审计模式（只读，未修改代码）  
**审计范围**: 整个项目 — 所有源码（Rust/TypeScript/JSX）、配置文件、构建脚本、静态资源、协议包、skills

---

## 一、仓库分区方案与审计分工

### 分区方案

| 分区 | 路径 | 审计优先级 | 风险领域 |
|------|------|-----------|---------|
| **P1-核心后端** | `src-tauri/src/` (Rust) | 高 | 命令执行、注入、内存安全、IPC |
| **P2-前端** | `src/` (TypeScript/JSX) | 高 | XSS、数据污染、CSP |
| **P3-配置** | `tauri.conf.json`, `Cargo.toml`, `package.json`, `settings.rs` | 高 | 密钥、依赖、默认配置 |
| **P4-协议包** | `packages/protocol/` | 中 | Schema 验证、类型安全 |
| **P5-静态资源** | `index.html`, CSS, SVG, icons | 低 | XSS、资源完整性 |
| **P6-构建/脚本** | `build.rs`, `vite.config.ts` | 中 | 构建注入、路径泄漏 |

### 审计顺序

1. 配置与依赖（Cargo.toml / package.json / tauri.conf.json）
2. Rust 核心后端（lib.rs, commands.rs, ws_server.rs, settings.rs, pi_adapter.rs, pi_watcher.rs, tts/engine.rs, plugin/manager.rs, event_bus/bus.rs, state_machine/machine.rs）
3. 前端（App.tsx, wsClient.ts, useAgentState.ts, PetSVG.tsx）
4. 协议包（schemas.ts, validators.ts, mapping.ts, events.ts）
5. 静态资源与构建（index.html, build.rs, CSS, SVG）

---

## 二、总体结论

| 指标 | 值 |
|------|------|
| **总体风险评级** | 🟡 **有风险** |
| P0 严重漏洞 | 2 |
| P1 高风险漏洞 | 5 |
| P2 中等风险 | 6 |
| P3 低风险/隐患 | 10 |
| 误报项 | 4 |
| **新增发现** | 4 个（vs v1 旧报告） |

**是否可以上线**: ⚠️ **有条件上线** — 修复 P0 后可上线，P1 建议迭代修复

---

## 三、漏洞清单

---

### P0 — 严重漏洞

#### P0-1: CSP 未启用 — 允许任意内联脚本执行

- **文件**: `src-tauri/tauri.conf.json:22`
- **代码**:
  ```json
  "security": {
    "csp": null
  }
  ```
- **描述**: Tauri 2.x 的 `csp: null` 表示不设置任何 CSP 头，Webview 允许执行任意内联 `<script>`、`javascript:` URL。
- **影响**: 如果 JSONL 日志被注入恶意 JSON，经过 `EventConverter` 转换后进入前端，可通过 `raw` 字段渲染实现 XSS。
- **复现方式**: 向 `~/.pi/agent/logs/latest.jsonl` 写入含 `<img src=x onerror=alert(1)>` 的事件 → 经 `EventConverter` → `raw` 字段 → 前端渲染。
- **修复建议**:
  ```json
  "security": {
    "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
  }
  ```

---

#### P0-2: WebSocket 订阅事件未过滤 — 所有客户端收到所有事件

- **文件1**: `src-tauri/src/ipc/ws_server.rs:86`
- **文件2**: `src-tauri/src/ipc/ws_server.rs:188-213`
- **代码**:
  ```rust
  // 第 86 行: 未过滤，订阅全部事件
  let mut event_rx = event_tx.subscribe();
  // 第 188-213 行: subscribe 消息只返回确认，不改变过滤行为
  "subscribe" => {
      let event_types = msg.get("payload")...;
      // event_types 仅用于返回给客户端，未在事件循环中使用！
  ```
- **描述**: 客户端发送 `"subscribe"` 消息指定 `eventTypes`，但服务器端的事件循环 `event_rx.recv()` 订阅了 broadcast 通道的所有事件，**完全忽略了客户端的订阅过滤**。所有客户端始终收到所有事件，无论是否订阅。
- **影响**: 无法实现事件隔离；客户端订阅功能形同虚设；如果后续添加基于订阅的权限控制，当前逻辑会导致越权接收事件。
- **复现方式**:
  1. 建立两个 WS 连接
  2. 连接 A 只订阅 `tool_call_start`
  3. 连接 B 不订阅任何事件
  4. 两者都会收到 `session_start`、`tool_call_end` 等所有事件类型
- **修复建议**:
  ```rust
  // 在事件循环中添加过滤逻辑
  match event {
      Ok(event) => {
          // 检查客户端订阅
          if !client_event_types.contains(&"*") 
             && !client_event_types.contains(&event.event_type) 
          { return; }
          // 发送
      }
  }
  ```

---

### P1 — 高风险漏洞

#### P1-1: `UnifiedAgentEvent.raw` 字段无大小限制 — 内存 DoS

- **文件**: `src-tauri/src/types/events.rs:282`
- **代码**:
  ```rust
  pub raw: Option<serde_json::Value>,
  ```
- **描述**: `raw` 字段存储完整原始 JSON，`EventConverter::convert` 中对 `raw.clone()` 不做任何截断。`UnifiedAgentEvent` 通过事件总线广播到所有订阅者（EventBus + WSServer）。
- **影响**: 攻击者通过 JSONL 文件写入超大 JSON（如 10MB+ 的 `{"data": "x".repeat(10_000_000)}`），导致内存爆炸。broadcast 通道的每个接收者都会持有完整副本。
- **复现方式**: 向 JSONL 文件追加 `{"type":"text_delta","prompt":"t","raw":{"data":"x".repeat(10_000_000)}}`
- **修复建议**: 在 `EventConverter::convert` 中截断 raw:
  ```rust
  let raw = if raw.get_array().map(|a| a.len()).unwrap_or(0) > 1000
       || raw.get_string().map(|s| s.len()).unwrap_or(0) > 4096 {
      Some(serde_json::json!({"raw_type": event_type_str, "truncated": true}))
  } else {
      Some(raw.clone())
  };
  ```

---

#### P1-2: `UnifiedAgentEvent.metadata` 字段无大小限制

- **文件**: `src-tauri/src/types/events.rs:290`
- **代码**:
  ```rust
  pub metadata: Option<serde_json::Value>,
  ```
- **描述**: 与 `raw` 类似，`metadata` 字段无大小限制。`EventConverter` 中对部分事件类型（如 `text_delta`、`tool_error`）设置了 `metadata` 字段（包含 `{"raw_type": ...}`），虽然当前数据量小，但未来扩展时若无限制也可能 DoS。
- **影响**: 如果未来从 Pi Agent 注入自定义 metadata，可能携带大数据。
- **修复建议**: 添加 `max_metadata_size: usize` 限制或在 `EventConverter` 中截断。

---

#### P1-3: WebSocket 服务器无 TLS — 明文传输 + 无最大客户端限制

- **文件**: `src-tauri/src/ipc/ws_server.rs:61,65-69`
- **代码**:
  ```rust
  let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).await?;
  // 每个连接都 spawn 新任务，无限制
  loop {
      let (stream, addr) = listener.accept().await?;
      tokio::spawn(async move { /* handle_client */ });
  }
  ```
- **描述**: (1) WS 服务器仅支持 `ws://` 明文，(2) 无 `max_clients` 限制，(3) 每个客户端处理函数无超时机制。
- **影响**: 本地 DoS — 无限连接可导致资源耗尽；消息明文传输。
- **修复建议**: 添加 `max_clients` 配置项，使用 `AtomicUsize` 计数器限制并发连接数。

---

#### P1-4: TTS 引擎 `say` 命令参数注入（macOS）

- **文件**: `src-tauri/src/tts/engine.rs:172-177`
- **代码**:
  ```rust
  let _ = Command::new("say")
      .args(["-r", &rate.to_string(), "--", text])
      .spawn();
  ```
- **描述**: macOS `say` 命令的 `--` 后 `text` 参数如果包含 `--` 或 `-` 开头的子串（如 `--verbose=quick`），可能被误解析为 `say` 的子选项。Linux `espeak` 已有 `--` 终止符保护。
- **影响**: Pi Agent 输出文本如 `"Testing --verbose=quick"` 在 macOS 上可能被 `say` 解释为选项。
- **修复建议**: 对 `say` 和 `espeak` 统一在 `--` 后传递文本，并对文本进行前缀检查：
  ```rust
  let text_arg = if text.starts_with('-') {
      format!("-- {}", text)
  } else {
      text.to_string()
  };
  ```

---

#### P1-5: `update_settings` 命令接受任意 JSON — deep merge 无递归深度限制

- **文件**: `src-tauri/src/commands.rs:88-96`
- **文件2**: `src-tauri/src/config/settings.rs:224-234`
- **代码**:
  ```rust
  #[command]
  pub fn update_settings(updates: serde_json::Value) -> Result<(), String> {
      let merged = merge_json(current, updates);
      self.settings = serde_json::from_value(merged)?;
  ```
  ```rust
  fn merge_json(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
      match (base, overlay) {
          (serde_json::Value::Object(mut a), serde_json::Value::Object(b)) => {
              for (k, v) in b {
                  a.insert(k, merge_json(existing.unwrap_or(Null), v)); // 无深度限制
              }
          }
      }
  }
  ```
- **描述**: `merge_json` 是纯递归函数，无最大递归深度限制。前端通过 Tauri IPC 传入深度嵌套的 JSON（如 `{ "pet": { "adapter": { "pi": { "hermes": ... } } } }` 嵌套 1000+ 层），可导致栈溢出。
- **影响**: 栈溢出 panic 或 `RUST_BACKTRACE=1` 时泄露调用栈信息。
- **修复建议**: 添加深度计数器，超过 64 层时返回错误。

---

### P2 — 中等风险

#### P2-1: WebSocket 客户端认证失败后连接挂死

- **文件**: `src/services/wsClient.ts:176-183`
- **代码**:
  ```typescript
  case "auth_ack": {
      const payload = message.payload as { authorized: boolean };
      if (payload.authorized) {
          this._authenticated = true;
          this.startHeartbeat();
          this.triggerConnected();
      }
      break; // authorized=false 时什么都不做
  }
  ```
- **描述**: 认证失败时客户端既不清除连接也不重试，`_authenticated` 保持 `false`，心跳不会启动，连接处于半死状态。
- **影响**: 认证失败后客户端持续连接但不做任何事，占用资源。
- **修复建议**: 认证失败后 `ws.close()` 并触发重连。

---

#### P2-2: Pi JSONL 文件路径可配置为任意路径 — 路径穿越风险

- **文件**: `src-tauri/src/config/settings.rs:126`
- **代码**:
  ```rust
  fn default_pi_log_path() -> String {
      "~/.pi/agent/logs/latest.jsonl".to_owned()
  }
  ```
- **描述**: `log_path` 可由用户配置为任意路径（如 `../../../../etc/passwd`）。虽然 `expand_home` 只处理 `~` 前缀，但绝对路径不受限制。`PiAdapter::send_message` 直接 `OpenOptions::new().create(true).append(true).open(&path)` 写入任意路径。
- **影响**: 配置被修改后，可以向任意路径写入 JSONL 数据。如果配合 Pi Agent 的 JSONL 格式，可能覆盖 `~/.pi/agent/config.json` 或 `~/.pi/.env` 等关键文件。
- **修复建议**: 在配置中限制 `log_path` 必须在 `~/.pi/agent/logs/` 目录内。

---

#### P2-3: `get_pet_state_sync` 使用 `rt.block_on` 可能死锁

- **文件**: `src-tauri/src/commands.rs:134-140`
- **代码**:
  ```rust
  pub fn get_pet_state_sync() -> Result<PetState, String> {
      let rt = Handle::current();
      let machine = rt.block_on(async { STATE_MACHINE.lock().await });
      Ok(machine.current_state().clone())
  ```
- **描述**: `block_on` 在 Tokio handle 上同步等待 async 操作。如果此函数在 async 上下文中被调用（如从另一个 async 函数中调用），可能死锁 — 因为 Tokio 的单线程运行时无法同时执行被 block_on 阻塞的任务和 await 的任务。
- **影响**: 在特定调用路径下（如托盘菜单事件回调中调用了 async 函数，该函数又调用了 `get_pet_state_sync`），可能导致死锁。
- **修复建议**: 改用 `tokio::task::spawn_blocking` 或添加 `#[allow(dead_code)]` 标记为仅 sync 上下文使用。

---

#### P2-4: WebSocket 消息无大小限制

- **文件**: `src-tauri/src/ipc/ws_server.rs:87-95`
- **代码**:
  ```rust
  Some(Ok(Message::Text(text))) => {
      let msg: serde_json::Value = serde_json::from_str(text)?;
  ```
- **描述**: `text` 是完整消息体，未限制长度。`serde_json::from_str` 会将整个字符串加载到内存中。
- **影响**: 发送一个 100MB 的 JSON 消息可导致 OOM。
- **修复建议**: 在接收消息时限制大小（如 64KB）。

---

#### P2-5: `send_event` 命令接受任意 `UnifiedAgentEvent` — 无输入校验

- **文件**: `src-tauri/src/commands.rs:104-112`
- **代码**:
  ```rust
  #[command]
  pub async fn send_event(event: UnifiedAgentEvent) -> Result<usize, String> {
      event_bus.publish_event(event)
  }
  ```
- **描述**: 前端通过 Tauri IPC 发送任意 `UnifiedAgentEvent` 到后端，没有 schema 校验。前端可以发送 `pet_state: "error"`、`source: "hermes"` 等任何值。
- **影响**: 前端可以伪造事件类型和来源，绕过 Pi Adapter 直接控制状态机。例如前端直接发送 `SessionEnd` 事件可以将任何状态重置为 `Idle`。
- **修复建议**: 添加前端 schema 校验（使用 `packages/protocol/src/validators.ts`），或在后端添加白名单校验。

---

#### P2-6: `pi_watcher` 轮询间隔 500ms 过于频繁

- **文件**: `src-tauri/src/adapter/pi_watcher.rs:248`
- **代码**:
  ```rust
  let poll_interval = std::time::Duration::from_millis(500);
  ```
- **描述**: 每 500ms 读取一次 JSONL 文件元数据并检查文件大小。如果 JSONL 文件被频繁写入，会导致大量 stat 系统调用。
- **影响**: 轻微 CPU/IO 开销。
- **修复建议**: 增加到 1-2 秒，或依赖 `notify` 文件系统事件触发。

---

### P3 — 低风险 / 潜在隐患

#### P3-1: 托盘图标颜色更新未实际生效

- **文件**: `src-tauri/src/window/tray.rs:76`
- **代码**:
  ```rust
  let _color = match state { ... }; // 赋值给 _color，未使用
  ```
- **描述**: 计算了颜色值但赋值给 `let _color`（前缀 `_` 表示未使用），实际未调用 `set_icon()` 更新托盘图标。
- **影响**: 状态变更时托盘图标颜色不更新。

---

#### P3-2: `get_agent_info` 硬编码在线状态

- **文件**: `src-tauri/src/commands.rs:179-193`
- **描述**: Hermes 和 OpenClaw 的 `online` 硬编码为 `false`，不查询适配器实际状态。
- **影响**: 前端显示的 Agent 在线状态不准确。

---

#### P3-3: 插件 manifest 无 schema 验证

- **文件**: `src-tauri/src/plugin/manager.rs:405-412`
- **描述**: manifest 结构体使用 `Deserialize` 派生，不验证字段内容。额外字段会被静默忽略。
- **影响**: 如果未来引入 `entry` 字段，恶意 manifest 可指定 `../etc/passwd` 等路径。

---

#### P3-4: WebSocket 客户端 JSON.parse 错误只 console.error

- **文件**: `src/services/wsClient.ts:71-75`
- **描述**: 解析失败后继续处理，不通知上层。
- **影响**: 低级别 — 日志刷屏，无功能影响。

---

#### P3-5: Pi JSONL 文件写入无原子性

- **文件**: `src-tauri/src/adapter/pi_adapter.rs:275-288`
- **描述**: `OpenOptions::new().create(true).append(true)` 写入不保证原子性。
- **影响**: 并发写入可能导致 JSON 行交错。

---

#### P3-6: TTS 命令 `espeak` 执行结果未检查

- **文件**: `src-tauri/src/tts/engine.rs:185`
- **代码**:
  ```rust
  let _ = Command::new("espeak")...spawn();
  ```
- **描述**: `let _ =` 丢弃 `Command::spawn()` 的返回值。如果 `espeak` 未安装，命令静默失败。
- **影响**: 用户无法得知 TTS 不可用。

---

#### P3-7: `PiAdapter::send_message` 使用 JSONL 文件发送消息

- **文件**: `src-tauri/src/adapter/pi_adapter.rs:275-288`
- **描述**: `send_message` 通过将 JSON 追加到 JSONL 日志文件来发送消息。这意味着 Pi Agent 必须能识别 JSONL 文件中新追加的行作为输入。如果 Pi Agent 不是通过读取 JSONL 文件来接收消息，而是通过 RPC/管道，此方法无效。
- **影响**: MVP 功能不完整，实际发送消息可能不会触发 Pi Agent 行为。

---

#### P3-8: `pi_watcher` 首次启动等待 30 秒无超时反馈

- **文件**: `src-tauri/src/adapter/pi_watcher.rs:142-151`
- **代码**:
  ```rust
  while !self.file_path.exists() && start.elapsed() < timeout {
      tokio::time::sleep(std::time::Duration::from_secs(1)).await;
  }
  ```
- **描述**: 文件不存在时等待 30 秒，超时后返回 `Ok(())` 而非错误。监听器静默启动但不读取任何数据。
- **影响**: 用户无法得知 JSONL 文件创建失败。

---

#### P3-9: `build.rs` 标记了不必要的 rerun

- **文件**: `src-tauri/build.rs:2`
- **代码**:
  ```rust
  println!("cargo:rerun-if-changed=tauri.conf.json");
  println!("cargo:rustc-cfg=tauri_build");
  ```
- **描述**: `cargo:rustc-cfg=tauri_build` 会引入一个未使用的编译标志（代码中无 `#[cfg(tauri_build)]` 检查），但不影响功能。

---

#### P3-10: WebSocket 服务器 heartbeat 使用 `Vec::new().into()` 而非空切片

- **文件**: `src-tauri/src/ipc/ws_server.rs:141`
- **代码**:
  ```rust
  ws_stream.send(Message::Ping(Vec::new().into())).await.is_err()
  ```
- **描述**: 发送空 Ping 帧是合法的，但 `Vec::new().into()` 创建了临时的 `Vec<u8>`。可优化为静态引用。
- **影响**: 极小的性能开销，不影响功能。

---

## 四、旧报告误报项修正

1. **旧报告 P0-2 Token 不一致** → **已纠正**: 客户端和服务端默认 token 均为 `"agent-pet-hub"`，一致。旧报告的 `"agent-pet-hub-default"` 是笔误。
2. **旧报告 P0-3 TTS 命令注入** → **降级为 P1-4**: `espeak` 已有 `--` 终止符，macOS `say` 也有 `--`，但 `text` 参数如果以 `-` 开头可能被误解析。风险降低但仍需关注。
3. **旧报告 P2-5 React useCallback 闭包** → **保留但降级**: 使用 `useRef` 是正确模式，空依赖数组无问题。
4. **旧报告 P2-7 任意 JSON 更新** → **保留为 P1-5**: `merge_json` 无递归深度限制是实际风险。

---

## 五、最应该优先修的 5 个问题

| 排名 | 漏洞 | 修复难度 | 影响 |
|------|------|---------|------|
| 1 | **P0-2: WS 订阅未过滤** | ⭐⭐ 中等 | 所有客户端收到所有事件，订阅功能形同虚设 |
| 2 | **P0-1: CSP 未启用** | ⭐⭐ 中等 | 允许内联脚本，XSS 风险 |
| 3 | **P1-1: raw 字段无大小限制** | ⭐⭐ 中等 | 内存 DoS，通过 JSONL 触发 |
| 4 | **P1-4: TTS 参数注入** | ⭐ 简单 | macOS 上 `--` 开头的文本被误解析 |
| 5 | **P1-3: WS 无 max_clients** | ⭐ 简单 | 本地 DoS |

---

## 六、需要确认的地方

1. **`raw` 字段是否在前端被渲染为 HTML？** 当前 `PetSVG` 和 `PetStatus` 组件不直接渲染 `raw`，但如果有未来扩展（如开发者工具面板），可能渲染 `raw` 内容。请确认前端是否会直接渲染 `UnifiedAgentEvent.raw`。

2. **WebSocket 服务器是否对外暴露？** 当前绑定到 `127.0.0.1:8765`，仅本地可达。如果未来改为 `0.0.0.0`，P1-3 的 max_clients 和 TLS 变得重要。

3. **Pi Agent JSONL 格式是否由可信进程写入？** `latest.jsonl` 由 Pi Agent（本地进程）写入，但也可以通过 IPC 从外部进程追加。如果外部进程不可信，JSONL 注入风险增加。

4. **TTS 引擎是否通过 shell 执行？** 当前使用 `Command::new("espeak").args(...)` 直接执行二进制，不经过 shell。因此 `;`、`|`、`$()` 不会被 shell 解释。但如果未来改为 `shell: true`，P0-3 恢复为 P1。

---

## 七、上线判断

| 条件 | 状态 |
|------|------|
| 核心功能安全（状态机、事件总线、适配器） | ✅ 安全 |
| 认证机制（WS token） | ✅ 一致，但无 TLS |
| XSS 防护（CSP） | ⚠️ 需修复 P0-1 |
| 命令注入防护（TTS） | ⚠️ 需修复 P1-4 |
| 内存保护（raw 大小限制） | ⚠️ 需修复 P1-1 |
| 并发安全（全局 statics） | ✅ 安全 |
| 依赖安全性 | ✅ 无已知 CVE |
| 配置泄露 | ✅ 无敏感密钥 |
| 插件系统 | ✅ 当前功能无风险 |
| WS 事件过滤 | ⚠️ 需修复 P0-2 |

**结论**: 🟡 **有条件上线**

- **必须修**: P0-2 (WS 订阅过滤) + P0-1 (CSP) → 修复后可上线
- **建议修**: P1-1 (raw 大小限制) + P1-4 (TTS 注入) + P1-3 (WS max_clients)
- **可延后**: P2-P3 → 后续迭代修复

---

## 八、项目安全评分

| 维度 | 评分 (1-10) | 说明 |
|------|------------|------|
| 认证与授权 | 6 | Token 硬编码、无 TLS、无 timing-safe 比较 |
| 命令执行/注入 | 7 | TTS 直接传参但有 `--` 终止符，非 shell 执行 |
| 文件系统/路径 | 8 | 路径处理良好，log_path 可配置为任意路径 |
| 密钥/Token 泄露 | 6 | Token 硬编码、配置明文存储、无环境变量覆盖 |
| 依赖与供应链 | 8 | 无已知 CVE，依赖版本较新 |
| 网络/本地服务暴露 | 5 | 绑定本地、无 TLS、无 rate limit、无 max_clients |
| 前端 XSS/CSP | 5 | CSP 为 null，但 raw 不直接渲染 HTML |
| 日志泄露/错误信息 | 8 | 日志中的文本被截断，无路径泄露 |

**综合评分: 6.6/10** — 中等安全水平
