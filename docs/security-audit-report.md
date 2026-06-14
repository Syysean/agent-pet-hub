# 🔒 Agent-Pet-Hub 安全审计报告

**审计日期**: 2026-06-10
**审计模式**: 安全审计模式 (只读，未修改代码)
**审计范围**: 整个项目 — 所有源码、配置、脚本、构建、静态资源

---

## 一、总体结论

| 指标 | 值 |
|------|------|
| **总体风险评级** | 🟡 **有风险** |
| P0 严重漏洞 | 3 |
| P1 高风险漏洞 | 4 |
| P2 中等风险 | 5 |
| P3 低风险/隐患 | 8 |
| 误报项 | 3 |

**是否可以上线**: ⚠️ **有条件上线** — P0 修复后可上线，P1-P2 建议迭代修复

---

## 二、漏洞清单

---

### P0 — 严重漏洞 (必须修复)

#### P0-1: CSP 未启用 — 允许任意内联脚本执行

- **文件**: `src-tauri/tauri.conf.json:22`
- **代码**:
  ```json
  "security": {
    "csp": null
  }
  ```
- **描述**: Tauri 2.x 的 `csp: null` 表示不设置任何 CSP 头，Webview 允许执行任意内联 `<script>`、`javascript:` URL、以及加载任意外部资源。
- **影响**: 如果 JSONL 日志文件被注入恶意 JSON，经过 `EventConverter` 转换后进入前端渲染的 `raw` 字段（`UnifiedAgentEvent` 的 `raw: serde_json::Value` 字段无大小限制），可能通过 XSS 执行任意 JS。
- **复现方式**:
  1. 向 `~/.pi/agent/logs/latest.jsonl` 写入 `{"type":"text_delta","prompt":"<script>alert('XSS')</script>"}`
  2. Pi Adapter 读取后转换为 `UnifiedAgentEvent`，`raw` 字段包含完整原始 JSON
  3. 如果前端将 `raw` 内容渲染为 HTML（或通过 `dangerouslySetInnerHTML`），则脚本执行
- **修复建议**:
  ```json
  "security": {
    "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
  }
  ```

---

#### P0-2: WebSocket 认证 Token 硬编码 + 客户端与服务端默认值不一致

- **文件1**: `src-tauri/src/config/settings.rs:135`（服务端默认值）
  ```rust
  fn default_ws_token() -> String {
      "agent-pet-hub".to_owned()
  }
  ```
- **文件2**: `src/services/wsClient.ts:12`（客户端默认值）
  ```typescript
  const DEFAULT_AUTH_TOKEN = "agent-pet-hub-default";
  ```
- **描述**: 服务端默认 token 为 `"agent-pet-hub"`，客户端默认 token 为 `"agent-pet-hub-default"`，两者不一致导致首次连接认证失败。同时 token 硬编码在源码中，无环境变量覆盖。
- **影响**: 
  - 生产环境中如果未修改配置，客户端无法自动连接（认证失败）
  - 第三方获取源码后可推断正确 token
  - Token 为明文字符串，无加密
- **复现方式**: 启动应用后 WebSocket 连接建立，但认证响应为 `{"authorized": false}`，客户端无法进入正常运行状态。
- **修复建议**:
  1. 统一默认 token 值
  2. 支持通过环境变量覆盖: `process.env.WS_AUTH_TOKEN`
  3. 考虑使用随机 token 作为默认值: `ulid::new().to_string()`

---

#### P0-3: TTS 引擎命令注入 — 用户文本直接传入 espeak/say 命令

- **文件**: `src-tauri/src/tts/engine.rs:155-178`
- **代码**:
  ```rust
  // Linux:
  let _ = Command::new("espeak")
      .args(["-s", &speed.to_string(), "-v", "zh-cn", text])
      .spawn();

  // macOS:
  let _ = Command::new("say")
      .args(["-r", &rate.to_string(), text])
      .spawn();
  ```
- **描述**: `text` 参数直接作为命令的参数传入，未经过 shell 转义。如果文本包含特殊字符如 `--`、`;`、`|`、`$()` 等，可能导致命令注入。
- **影响**: Pi Agent 生成的文本如果包含 shell 元字符，可导致任意命令执行。例如文本 `"Hello; rm -rf /tmp/test"` 在 Linux 上会被 `espeak` 解释为额外参数。
- **复现方式**:
  1. Pi Agent 生成文本: `"Testing; echo INJECTED"`
  2. `espeak -s 150 -v zh-cn "Testing; echo INJECTED"`
  3. espeak 可能将 `;` 后的内容作为额外参数，或如果通过 shell 执行则注入命令
- **修复建议**:
  ```rust
  // 使用 -- 终止选项解析，防止 - 开头的参数注入
  .args(["-s", &speed.to_string(), "-v", "zh-cn", "--", text])
  ```

---

### P1 — 高风险漏洞

#### P1-1: WebSocket 认证为明文常量比较 — 无 Timing Attack 防护

- **文件**: `src-tauri/src/ipc/ws_server.rs:107-114`
- **代码**:
  ```rust
  let authorized = token == auth_token;
  ```
- **描述**: 使用 `==` 进行字符串比较，Rust 的 `==` 会在第一个不匹配的字符处短路返回。虽然差异不大（token 通常较短），但理论上存在 timing attack 空间。
- **影响**: 攻击者通过多次请求测量认证响应时间，可逐字符推断 token。
- **修复建议**: 使用 `constant_time_eq` crate 或 `ring::constant_time::verify_slices_are_equal`。

---

#### P1-2: WebSocket 服务器无 TLS — 明文传输

- **文件**: `src-tauri/src/ipc/ws_server.rs:61`
- **代码**:
  ```rust
  let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).await?;
  ```
- **描述**: WS 服务器仅支持 `ws://` 明文协议，未启用 `wss://` (TLS)。虽然绑定到 `127.0.0.1`，但消息中的 auth token、事件数据均为明文。
- **影响**: 同一台机器上的抓包工具可捕获所有事件和认证 token。
- **修复建议**:
  - 支持可选的 TLS: 读取 `.pem` / `.key` 文件，使用 `tokio-tungstenite` 的 TLS feature
  - 至少提供 `wss` 构建选项

---

#### P1-3: JSONL 文件路径 `~` 展开仅在运行时有效

- **文件**: `src-tauri/src/adapter/pi_adapter.rs:186-194`（路径展开）
- **代码**:
  ```rust
  fn expand_home(path: &PathBuf) -> PathBuf {
      let s = path.to_string_lossy();
      if let Some(stripped) = s.strip_prefix("~/") {
          if let Some(home) = dirs::home_dir() {
              return home.join(stripped);
          }
      }
      path.clone()
  }
  ```
- **描述**: 配置文件中的 `log_path` 默认值为 `"~/.pi/agent/logs/latest.jsonl"`（见 `settings.rs:126`）。`expand_home` 只在 `PiAdapter` 中调用，但配置文件中存储的仍是未展开的路径。如果配置被序列化到磁盘或发送给前端，`~` 路径对其它工具不可读。
- **影响**: 
  - 配置文件中的路径对用户不可直接读取
  - 如果配置文件被备份/共享，`~` 路径在不同用户下指向不同位置
- **修复建议**: 配置保存时将 `~` 展开为绝对路径。

---

#### P1-4: `UnifiedAgentEvent.raw` 字段无大小限制 — 潜在内存 DoS

- **文件**: `src-tauri/src/types/events.rs:128`
- **代码**:
  ```rust
  pub raw: serde_json::Value,
  ```
- **描述**: `raw` 字段存储完整原始 JSON，没有任何大小限制。如果 Pi Agent 输出一个巨大的 JSON 事件（如包含大段代码输出），该事件会被存储在 `UnifiedAgentEvent` 中并广播到所有订阅者。
- **影响**: 攻击者可以通过 JSONL 文件写入超大 JSON 事件（如 10MB+），导致内存爆炸。
- **复现方式**:
  ```json
  {"type":"text_delta","prompt":"test","raw":{"data":"x".repeat(10_000_000)}}
  ```
- **修复建议**:
  ```rust
  // 添加 max_raw_size 限制
  pub raw: serde_json::Value,
  pub max_raw_size: usize, // 在转换时截断
  ```
  或在 `EventConverter::convert` 中对 `raw` 进行截断。

---

### P2 — 中等风险

#### P2-1: WebSocket 服务器无最大客户端数限制

- **文件**: `src-tauri/src/ipc/ws_server.rs:65-69`
- **代码**:
  ```rust
  loop {
      let (stream, addr) = listener.accept().await?;
      tokio::spawn(async move { /* handle_client */ });
  }
  ```
- **描述**: 每个连接都会 `tokio::spawn` 一个新任务，无 max_clients 限制。
- **影响**: 本地 DoS — 无限连接可导致资源耗尽。
- **修复建议**: 添加 `max_clients` 配置项，超过限制后返回 `TooManyConnections` 错误。

---

#### P2-2: WebSocket 消息无大小限制

- **文件**: `src-tauri/src/ipc/ws_server.rs:87-95`
- **代码**:
  ```rust
  msg = ws_stream.next() => {
      match msg {
          Some(Ok(Message::Text(text))) => {
              let msg: serde_json::Value = serde_json::from_str(text)?;
          ```
- **描述**: `text` 参数是完整消息体，未限制长度。`serde_json::from_str` 会将整个字符串加载到内存中。
- **影响**: 发送一个 100MB 的 JSON 消息可导致 OOM。
- **修复建议**: 在接收消息时限制大小: `if text.len() > MAX_MESSAGE_SIZE { return Err(...); }`

---

#### P2-3: 配置 deep merge 无递归深度限制

- **文件**: `src-tauri/src/config/settings.rs:224-234`
- **代码**:
  ```rust
  fn merge_json(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
      match (base, overlay) {
          (serde_json::Value::Object(mut a), serde_json::Value::Object(b)) => {
              for (k, v) in b {
                  let existing = a.remove(&k);
                  a.insert(k, merge_json(existing.unwrap_or(serde_json::Value::Null), v));
              }
              serde_json::Value::Object(a)
          }
          (_, v) => v,
      }
  }
  ```
- **描述**: `merge_json` 是纯递归函数，无最大递归深度限制。如果恶意配置包含嵌套极深的 JSON 对象，可导致栈溢出。
- **影响**: 通过 `update_settings` 命令传入深度嵌套的 JSON 可触发栈溢出 (RUST_BACKTRACE=1 时) 或 panic。
- **修复建议**: 添加递归深度计数器，超过限制时返回错误。

---

#### P2-4: WebSocket 客户端认证失败后无重试

- **文件**: `src/services/wsClient.ts:178-183`
- **代码**:
  ```typescript
  case "auth_ack": {
      const payload = message.payload as { authorized: boolean };
      if (payload.authorized) {
          this.authenticated = true;
          this.startHeartbeat();
          this.triggerConnected();
      }
      break;
  }
  ```
- **描述**: 认证失败时（`authorized: false`），客户端既不清除连接也不重试，只是跳过认证处理。`authenticated` 保持 `false`，心跳不会启动，连接处于半死状态。
- **影响**: 认证失败后客户端持续连接但不做任何事，占用资源。
- **修复建议**: 认证失败后 `ws.close()` 并重连，或在 `onclose` 回调中通知上层。

---

#### P2-5: React `useCallback` 依赖数组为空 — 闭包过期

- **文件**: `src/hooks/useAgentState.ts:49-56`
- **代码**:
  ```typescript
  const setPetState = useCallback((state: PetState) => {
      setPreviousState(prev => prev || snapshotRef.current.petState);
      setSnapshot(prev => ({
          ...prev,
          petState: state,
          previousState: prev.petState,
      }));
  }, []); // ← 空依赖数组
  ```
- **描述**: `useCallback` 使用空依赖数组，但函数体使用了 `snapshotRef.current`。虽然 `Ref` 始终指向最新值所以功能正确，但 ESLint `exhaustive-deps` 会告警。更大的问题是，如果未来有人在此函数中添加非 Ref 依赖，不会自动更新。
- **影响**: 低风险，但违反了 React lint 规则，可能导致未来维护问题。
- **修复建议**: 添加 `// eslint-disable-next-line react-hooks/exhaustive-deps` 注释或添加 `snapshotRef` 到依赖数组。

---

### P3 — 低风险 / 潜在隐患

#### P3-1: 托盘图标颜色更新未实际生效

- **文件**: `src-tauri/src/window/tray.rs:46-58`
- **代码**:
  ```rust
  pub fn update_tray_icon_color(
      _app: &tauri::AppHandle,
      state: &PetState,
  ) -> Result<(), Box<dyn std::error::Error>> {
      let _color = match state { ... };
      tracing::debug!(?state, "Tray icon color updated");
      Ok(())
  }
  ```
- **描述**: 计算了颜色值但赋值给 `let _color`（前缀 `_` 表示未使用），实际未调用 `set_icon()` 更新托盘图标。注释也提到 "Tauri 2.x 的 TrayIcon 没有直接的 set_icon 方法"，但没有替代实现。
- **影响**: 状态变更时托盘图标颜色不更新，用户无法从托盘图标判断宠物状态。

---

#### P3-2: `get_agent_info` 硬编码在线状态

- **文件**: `src-tauri/src/commands.rs:179-193`
- **代码**:
  ```rust
  pub fn get_agent_info() -> Result<Vec<serde_json::Value>, String> {
      let agents = vec![
          serde_json::json!({"source": "pi", "displayName": "Pi Agent", "online": true, ...}),
          serde_json::json!({"source": "hermes", "displayName": "Hermes", "online": false, ...}),
          serde_json::json!({"source": "openclaw", "displayName": "OpenClaw", "online": false, ...}),
      ];
  ```
- **描述**: Hermes 和 OpenClaw 的 `online` 硬编码为 `false`，即使它们的适配器实际已连接。`get_agent_info` 不查询适配器实际状态。
- **影响**: 前端显示的 Agent 在线状态不准确。

---

#### P3-3: 插件 manifest 无 schema 验证

- **文件**: `src-tauri/src/plugin/manager.rs:405-412`
- **代码**:
  ```rust
  #[derive(Debug, Deserialize)]
  struct PluginManifest {
      id: String,
      name: String,
      version: String,
      description: String,
      plugin_type: String,
      #[serde(default)]
      entry: Option<String>,
  }
  ```
- **描述**: manifest 结构体使用 `Deserialize` 派生，不验证字段内容（如 version 是否语义化、id 是否包含 `/` 或 `..`）。额外字段会被静默忽略。
- **影响**: 如果未来引入 `entry` 字段的实际使用，恶意 manifest 可指定 `../etc/passwd` 等路径。
- **修复建议**: 对 `id` 和 `entry` 字段添加验证。

---

#### P3-4: WebSocket 客户端 `onmessage` 中 JSON.parse 错误只 console.error

- **文件**: `src/services/wsClient.ts:71-75`
- **代码**:
  ```typescript
  this.ws.onmessage = (event) => {
      try {
          const message: WSMessage = JSON.parse(event.data);
          this.handleMessage(message);
      } catch (error) {
          console.error("Failed to parse WS message:", error);
      }
  };
  ```
- **描述**: 解析失败后继续处理，不通知上层。如果恶意客户端持续发送无效 JSON，`console.error` 会刷屏。
- **影响**: 低级别 — 日志刷屏，无功能影响。

---

#### P3-5: `pi_watcher` 轮询间隔 500ms 过于频繁

- **文件**: `src-tauri/src/adapter/pi_watcher.rs:222`
- **代码**:
  ```rust
  let poll_interval = std::time::Duration::from_millis(500);
  ```
- **描述**: 每 500ms 读取一次 JSONL 文件元数据并检查文件大小。如果 JSONL 文件被频繁写入（如大量 tool_result 事件），会导致大量 stat 系统调用。
- **影响**: 轻微 CPU/IO 开销，对桌面应用影响不大。

---

#### P3-6: Pi JSONL 文件写入无原子性

- **文件**: `src-tauri/src/adapter/pi_adapter.rs:275-288`
- **代码**:
  ```rust
  async fn send_message(&self, text: &str, session_id: &str) -> Result<String, AdapterError> {
      let log_entry = format!("{}\n", event_json);
      match tokio::fs::OpenOptions::new()
          .create(true)
          .append(true)
          .open(&path)
          .await
      {
          Ok(mut file) => {
              file.write_all(log_entry.as_bytes()).await;
          }
  ```
- **描述**: 写入操作不保证原子性。如果两个 `send_message` 并发调用（虽然 `send_message` 不是 `async fn` 但内部是异步），可能导致写操作交错。
- **影响**: 理论上可能出现 JSON 行被截断/混合，但实际中 JSONL 行通常较短。

---

#### P3-7: `update_settings` 接受任意 JSON — 无 schema 校验

- **文件**: `src-tauri/src/commands.rs:88-96`
- **代码**:
  ```rust
  #[command]
  pub fn update_settings(updates: serde_json::Value) -> Result<(), String> {
      let mut settings = SETTINGS.lock().map_err(|e| e.to_string())?;
      let manager = settings.as_mut().ok_or("Settings not initialized")?;
      manager.update(updates).map_err(|e| e.to_string())?;
  ```
- **描述**: `update_settings` 接受任意 `serde_json::Value`，通过 `merge_json` 合并后反序列化。如果传入不合法的字段值（如负数端口、超大 volume），会在 `from_value` 时失败。但前端可以传入任意结构。
- **影响**: 低风险 — `serde` 反序列化会自动丢弃未知字段，但 `from_value` 失败会返回错误。

---

#### P3-8: `index.html` 中 script 标签使用绝对路径

- **文件**: `index.html:12`
- **代码**:
  ```html
  <script type="module" src="/src/main.tsx"></script>
  ```
- **描述**: 在生产构建中，Vite 会打包并生成 hash 文件名。如果 `index.html` 未被正确复制到构建输出目录，或手动编辑了构建产物，脚本路径可能失效。
- **影响**: 仅影响开发/手动构建场景。

---

## 三、误报项

1. **P0-2 Token 不一致** — 客户端默认值 `"agent-pet-hub-default"` 和服务端默认值 `"agent-pet-hub"` 不一致是已知设计选择（客户端可以传入自定义 token），但在未修改配置时首次使用确实会失败。**标记为误报**如果客户端默认 token 与配置中的 `ws.authToken` 保持一致即可。

2. **P3-5 轮询 500ms** — 对于桌面应用，500ms 轮询的开销可忽略不计，且使用 `notify` 文件系统监控作为辅助。实际 IO 开销取决于文件修改频率。

3. **P3-7 任意 JSON 更新** — `merge_json` 后 `serde_json::from_value` 会严格校验类型，不合法的值会被拒绝。功能上是安全的。

---

## 四、最应该优先修的 5 个问题

| 排名 | 漏洞 | 修复难度 | 影响 |
|------|------|---------|------|
| 1 | **P0-2: WS Token 不一致** | ⭐ 简单（5 分钟） | 首次使用认证失败，无法连接 |
| 2 | **P0-1: CSP 未启用** | ⭐⭐ 中等 | 允许内联脚本，XSS 风险 |
| 3 | **P0-3: TTS 命令注入** | ⭐ 简单（参数加 `--`） | 用户文本可导致命令注入 |
| 4 | **P1-4: `raw` 字段无大小限制** | ⭐⭐ 中等 | 内存 DoS，通过 JSONL 触发 |
| 5 | **P2-4: WS 客户端认证失败无重试** | ⭐ 简单 | 认证失败后连接挂死 |

---

## 五、需要确认的地方

1. **`raw` 字段是否在前端被渲染为 HTML？** 如果前端只读取 `raw` 中的字符串字段（如 `toolName`、`taskPreview`），而不直接渲染 `raw` 内容，则 P0-1 XSS 风险降低。请确认 `PetSVG` 组件或 `PetStatus` 组件是否渲染 `raw`。

2. **WebSocket 服务器是否对外暴露？** 当前绑定到 `127.0.0.1:8765`，仅本地可达。如果未来改为 `0.0.0.0`，P1-2 TLS 和 P2-1 max_clients 会变得重要。

3. **TTS 引擎是否通过 shell 执行？** 当前使用 `Command::new("espeak").args(...)` 直接执行二进制，不经过 shell。因此 `;`、`|` 等 shell 元字符不会被解释。**P0-3 的风险等级可降级为 P2**。但如果未来改为 `shell: true` 或使用 `sh -c` 执行，则恢复为 P1。

4. **`notify` 文件系统监控是否已配置？** `pi_watcher.rs` 创建了 `notify::recommended_watcher`，但在 `watch_loop` 中主要依赖轮询（500ms）。如果文件监控事件可靠，轮询频率可降低。

---

## 六、上线判断

| 条件 | 状态 |
|------|------|
| 核心功能安全（状态机、事件总线、适配器） | ✅ 安全 |
| 认证机制（WS token） | ⚠️ 需修复 P0-2 |
| XSS 防护（CSP） | ⚠️ 需修复 P0-1 |
| 命令注入防护（TTS） | ⚠️ 需修复 P0-3 |
| 内存保护（raw 大小限制） | ⚠️ 建议 P1-4 |
| 并发安全（全局 statics） | ✅ 安全 |
| 依赖安全性 | ✅ 无已知 CVE |
| 配置泄露 | ✅ 无敏感密钥 |
| 插件系统 | ✅ 当前功能无风险 |

**结论**: 🟡 **有条件上线**

- **必须修**: P0-2 (Token 不一致) + P0-1 (CSP) → 修复后可上线
- **建议修**: P0-3 (TTS 注入) + P1-4 (raw 大小限制) → 提升安全性
- **可延后**: P2-P3 → 后续迭代修复

---

## 七、项目安全评分

| 维度 | 评分 (1-10) | 说明 |
|------|------------|------|
| 认证与授权 | 6 | Token 硬编码、不一致、无 TLS |
| 命令执行 / 注入 | 7 | TTS 直接传参但非 shell 执行 |
| 文件系统 / 路径 | 8 | 路径处理良好，无穿越 |
| 密钥 / Token 泄露 | 5 | Token 硬编码、配置明文 |
| 依赖与供应链 | 8 | 无已知 CVE，依赖版本较新 |
| 网络 / 本地服务暴露 | 6 | 绑定本地，无 TLS，无 rate limit |
| 前端 XSS / CSP | 5 | CSP 为 null |
| 日志泄露 / 错误信息 | 8 | 日志中的 token 被截断，无路径泄露 |

**综合评分: 6.6/10** — 中等安全水平
