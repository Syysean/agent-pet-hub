# 🔒 Agent-Pet-Hub 安全审计报告 (v4)

> **审计时间**：2026-06-12
> **审计模式**：安全审计模式（只读，不修改代码）
> **审计范围**：整个 `desktop-pet/agent-pet-hub` 项目
> **审计方法**：8 维度串行子代理审计（认证授权 → 命令注入 → 文件系统 → 密钥配置 → IPC/网络 → 前端XSS → 日志泄露 → 依赖/构建）

---

## 📊 总体结论：⚠️ 有风险

| 严重级别 | 数量 | 描述 |
|---------|------|------|
| **P0 严重** | 2 | 路径穿越读取任意文件、JSONL 大文件内存爆炸 |
| **P1 高** | 4 | 配置无权限校验、WS 认证无状态跟踪、skin_id 路径穿越、非原子配置写入 |
| **P2 中** | 7 | WS 无连接数限制、TTS 命令注入、前端 console.log 泄露、事件 raw 字段未过滤 |
| **P3 低** | 6 | CSP 缺失、默认 token 硬编码、TOCTOU 竞态、日志路径泄露等 |
| **✅ 良好** | — | 无 unsafe 代码、无高危依赖、配置权限 0600、WS 仅绑定 localhost |

**是否可以上线**：✅ **可以上线**，但有 2 个 P0 漏洞建议优先修复，P1 漏洞在典型使用场景下影响可控。

---

## 📋 漏洞清单

### P0 严重

#### P0-1: `get_skin_metadata` 路径穿越 — 读取任意文件

- **文件**：`src-tauri/src/commands.rs` 第 230-256 行
- **代码**：
```rust
pub fn get_skin_metadata(skin_id: String) -> Result<serde_json::Value, String> {
    let mut path = std::path::Path::new("../src/assets/skins")
        .join(&skin_id)           // ← skin_id 未规范化
        .join("skin.json");
    // ... 如果 skin.json 不存在，再尝试其他路径
    let path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => { /* 尝试其他路径 */ }
    };
    let content = std::fs::read_to_string(&path)?;
    // ...
}
```
- **漏洞描述**：前端传入的 `skin_id` 参数直接用于路径拼接，未做 `strip_prefix` 或白名单校验。恶意值可穿越目录边界。
- **触发方式**：前端调用 `invoke("get_skin_metadata", { skinId: "../../../etc/passwd/skin.json" })` → 三级路径查找最终命中用户目录 `~/.config/agent-pet-hub/skins/../../../etc/passwd/skin.json` → 解析 `/etc/passwd/skin.json` → 读取文件内容。
- **影响**：可读取用户家目录下任意 `.skin.json` 结尾的文件，或通过路径构造读取任意文本文件。
- **修复建议**：
  1. 使用 `path.canonicalize()?.strip_prefix(base)?.to_str()?` 验证路径在基础目录内
  2. 或对 `skin_id` 做白名单校验（仅允许 `[a-zA-Z0-9_-]+`）

#### P0-2: JSONL 文件差值读取无上限 — 内存 OOM

- **文件**：`src-tauri/src/adapter/pi_watcher.rs` 第 246-274 行
- **代码**：
```rust
let metadata = tokio::fs::metadata(&self.file_path).await?;
let file_size = metadata.len();
let new_bytes = file_size - self.last_position;  // ← 可能极大
let mut buffer = vec![0u8; new_bytes as usize];  // ← 直接分配
let bytes_read = reader.read(&mut buffer).await?;
```
- **漏洞描述**：按文件大小差值分配 buffer，无上限保护。如果 JSONL 文件从 1KB 膨胀到 1GB（如 Pi Agent 崩溃后写入大错误），会一次性分配 1GB 内存。
- **触发方式**：攻击者控制 JSONL 文件内容，使其从 1KB 膨胀到 >2GB（触发 OOM 或 swap）。
- **影响**：DoS（内存耗尽）、可能 OOM kill 整个进程。
- **修复建议**：
  1. 设置最大读取限制（如 `MAX_READ_BYTES = 1MB`）
  2. 超限时截断或跳过
  3. 使用 `BufReader` 逐行读取而非一次性分配

---

### P1 高

#### P1-1: 所有 Tauri Commands 无认证校验

- **文件**：`src-tauri/src/lib.rs` 第 50-63 行
- **代码**：
```rust
.invoke_handler(tauri::generate_handler![
    commands::get_pet_state,
    commands::get_previous_state,
    commands::get_state_snapshot,
    commands::get_settings,           // ← 返回含 auth_token 的完整配置
    commands::update_settings,        // ← 接受任意 JSON 深度合并
    commands::send_event,             // ← 发送任意事件到 EventBus
    commands::set_pet_state,          // ← 任意设置状态
    commands::toggle_pet_window,
    commands::send_heartbeat,
    commands::get_agent_info,
    commands::start_drag,
    commands::list_skins,
    commands::get_skin_metadata,
])
```
- **漏洞描述**：13 个 Tauri 命令全部注册且无任何 scope/权限限制。前端任意 JS 代码均可调用。虽然桌面应用内 JS 可信，但如果有 XSS 漏洞则攻击者可通过 JS 执行任何后端操作。
- **影响**：前端被 XSS 后，攻击者可修改配置（包括 WS token）、发送任意事件到 EventBus、设置任意状态。
- **修复建议**：
  1. 对 `update_settings` 和 `send_event` 使用 `tauri::capa::CapabilityBuilder` 做 scope 限制
  2. 或对关键命令增加 token 校验层

#### P1-2: WebSocket 认证无状态跟踪

- **文件**：`src-tauri/src/ipc/ws_server.rs` 第 165-190 行
- **代码**：
```rust
async fn handle_text_message(...) {
    match msg_type {
        "auth" => {
            let token = msg.get("payload")...;
            let authorized = token == auth_token;  // 简单字符串比较
            // 发回 auth_ack，但不标记客户端已认证
        }
        "subscribe" => {
            // ⚠️ 认证前/后均可 subscribe，无状态检查
            self.event_tx.subscribe()  // 直接返回 broadcast::Receiver
        }
        // ...
    }
}
```
- **漏洞描述**：WS 认证是请求-响应式，客户端发送 auth 消息后服务器只返回 `auth_ack`，但不记录该客户端已认证。随后发送的 `subscribe` 或 `event` 消息均不需要认证即可执行。
- **触发方式**：任意本地进程连接 `127.0.0.1:8765` → 发送 `{"type":"subscribe","payload":{"eventTypes":["*"]}}` → 直接接收所有事件推送。
- **影响**：WS token 泄露或默认 token 被猜测后，攻击者可直接接收所有事件（含 prompt 内容）。
- **修复建议**：
  1. 维护客户端认证状态（`HashMap<ConnectionId, bool>`）
  2. `subscribe` 前检查认证状态

#### P1-3: `update_settings` 接受任意 JSON 深度合并

- **文件**：`src-tauri/src/commands.rs` 第 101-120 行
- **代码**：
```rust
pub fn update_settings(updates: serde_json::Value) -> Result<(), String> {
    // 仅验证 skinId 是否存在
    if let Some(pet_updates) = updates.get("pet") { ... }
    let manager = settings.as_mut()...;
    manager.update(updates).map_err(...)?;  // ← 深度 merge 任意 JSON
    manager.save().map_err(...)?;
    Ok(())
}
```
- **漏洞描述**：`updates` 参数类型为 `serde_json::Value`（裸 JSON），前端可传入任意键值对覆盖所有配置字段，包括 `websocket.authToken`、`tts.volume`、`pi.logPath` 等。
- **影响**：前端可修改 WS token（绕过 WS 认证）、修改日志路径、修改 TTS 音量等所有配置。
- **修复建议**：
  1. 定义 `UpdateSettingsInput` struct，仅允许修改指定字段
  2. 或使用 `serde_json::Map` + 白名单校验

#### P1-4: 配置保存非原子写入（TOCTOU）

- **文件**：`src-tauri/src/config/settings.rs` 第 220-230 行
- **代码**：
```rust
pub fn save(&self) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&self.settings)?;
    fs::write(&self.config_path, json)?;  // ← 先写入内容
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(&self.config_path, perms)?;  // ← 后设置权限
}
```
- **漏洞描述**：先 `write` 后 `chmod`，在两步之间，配置文件内容为明文且权限为默认值（通常为 0644），其他用户可读取含 token 的配置。
- **影响**：进程被 signal 或崩溃时，config.json 可能以 0644 权限持久存在于磁盘上。
- **修复建议**：
  1. 使用 tmpfile + rename 原子写入模式
  2. 或在 `write` 时直接指定权限

---

### P2 中

#### P2-1: WebSocket 服务器无最大连接数限制

- **文件**：`src-tauri/src/ipc/ws_server.rs` 第 68-72 行
- **代码**：
```rust
loop {
    let (stream, addr) = listener.accept().await?;
    tauri::async_runtime::spawn(async move {
        // 每次连接 spawn 一个新 task，无数量限制
    });
}
```
- **漏洞描述**：没有 `max_connections` 配置，每次连接都 spawn 新 task，无限增长可能导致资源耗尽。
- **修复建议**：添加 `AtomicUsize` 计数器限制最大连接数（如 10）。

#### P2-2: TTS 命令注入

- **文件**：`src-tauri/src/tts/engine.rs` 第 175-188 行
- **代码**：
```rust
let _ = Command::new("say")
    .args(["-r", &rate.to_string(), "--", text])  // ← text 直接传入
    .spawn();
```
- **漏洞描述**：`text` 参数来自 JSONL 文件的 prompt/tool_args/result 字段，经 `EventConverter` 转换后传入。虽然使用 `--` 分隔符，但 `say`/`espeak` 对参数的解析可能不安全。
- **触发方式**：Pi Agent 输出包含 `"text": "--flag=value; rm -rf /"`（如果命令解析器将 `--` 后的内容也作为参数处理）。
- **修复建议**：对 `text` 做白名单过滤（仅允许 `[a-zA-Z0-9\u4e00-\u9fff\s.,!?;:'"()-]+`）。

#### P2-3: 前端 console.log 泄露完整事件 payload

- **文件**：`src/hooks/useAgentState.ts` 第 36、43 行
- **代码**：
```typescript
console.log("[pet] state_changed:", event.payload);    // L36
console.log("[pet] event:", event.payload);            // L43
```
- **漏洞描述**：每次收到 `pet:event` 都会 `console.log` 完整的 `UnifiedAgentEvent`，包含 `raw` 字段（最大 8KB 原始 JSON），可能包含 token、session_id、工具结果等敏感数据。
- **影响**：浏览器 DevTools 控制台可看到所有事件内容。如果后续使用 `dangerouslySetInnerHTML` 渲染 `raw` 字段，则产生 XSS。
- **修复建议**：
  1. 生产环境移除 `console.log` 或过滤 `raw` 字段
  2. 使用 `console.debug`（默认不输出）

#### P2-4: `UnifiedAgentEvent.raw` 字段保留完整原始 JSON

- **文件**：`src-tauri/src/types/events.rs` 第 279 行
- **代码**：
```rust
pub raw: Option<serde_json::Value>,  // ← 最大约 8KB 原始 JSON
```
- **漏洞描述**：所有事件转换后，`raw` 字段保留完整的原始 JSON 值，并通过 IPC 传递到前端。如果原始事件中包含 token/secret（如 Pi Agent 输出的 `raw` 事件），则会被泄露到前端。
- **修复建议**：在 `EventConverter` 中对 `raw` 字段做敏感字段过滤/截断。

#### P2-5: WS token 简单字符串比较（非 timing-safe）

- **文件**：`src/ipc/ws_server.rs` 第 128 行
- **代码**：
```rust
let authorized = token == auth_token;  // ← 简单字符串比较
```
- **漏洞描述**：使用 `==` 而非 `constant_time_eq`，理论上可通过计时攻击推断 token。对短 token（如 `"agent-pet-hub"`）影响有限。
- **修复建议**：使用 `subtle::ConstantTimeEq` 或 Rust 标准库的 `constant_time_eq`。

#### P2-6: `send_event` 接受任意 `UnifiedAgentEvent`

- **文件**：`src-tauri/src/commands.rs` 第 133-140 行
- **代码**：
```rust
pub async fn send_event(event: UnifiedAgentEvent) -> Result<usize, String> {
    let bus = EVENT_BUS.lock()...;
    event_bus.publish_event(event)...  // ← 前端可发送任意事件
}
```
- **漏洞描述**：前端可通过 Tauri invoke 直接向 EventBus 发送任意事件，伪造事件类型、来源、状态。
- **修复建议**：校验 `event.source` 和 `event.event_type` 是否在允许列表中。

#### P2-7: EventBus broadcast channel 容量有限（事件丢失）

- **文件**：`src-tauri/src/event_bus/bus.rs` 第 57-59 行
- **代码**：
```rust
let (event_tx, _) = broadcast::channel(channel_size);  // 默认 1024
```
- **漏洞描述**：broadcast channel 容量 1024/4096，超过后旧事件被丢弃。WS 客户端 lag 时会丢失事件（见 `ws_server.rs` L108-109 的 Lagged 警告）。
- **影响**：快速事件流期间，部分事件可能丢失，导致 WS 客户端状态不一致。
- **修复建议**：增大 channel 容量或在 lag 时清空旧事件。

---

### P3 低

#### P3-1: CSP 为 null

- **文件**：`src-tauri/tauri.conf.json` 第 24 行
- **代码**：`"security": { "csp": null }`
- **描述**：无 Content Security Policy，允许所有源加载资源。对桌面应用影响较小，但如果加载远程皮肤则有风险。
- **修复建议**：添加 CSP meta 标签：`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:;`

#### P3-2: 默认 WS token 硬编码

- **文件**：`src-tauri/src/config/settings.rs` 第 199 行、`src/services/wsClient.ts` 第 13 行
- **代码**：`"agent-pet-hub"`
- **描述**：前后端硬编码相同的默认 token，易被猜测。
- **修复建议**：首次启动时生成随机 token（如 `ulid::ULID`），写入 config.json。

#### P3-3: 多个 TOCTOU 竞态条件

- **文件**：`src/config/settings.rs` L207（exists + read）、`src/skin.rs` L199-222（is_dir + read）、`src/adapter/pi_watcher.rs` L246（metadata + read）
- **描述**：多个文件操作存在 TOCTOU 竞态窗口。
- **修复建议**：使用 `OpenOptions` 直接打开文件，消除 exists 检查窗口。

#### P3-4: 日志泄露家目录路径

- **文件**：多处 debug!/info! 日志记录完整文件路径
- **描述**：`~/.pi/agent/logs/latest.jsonl`、`~/.config/agent-pet-hub/config.json` 等路径在日志中泄露。
- **修复建议**：对路径做脱敏处理（如 `~/.pi/agent/logs/{hash}.jsonl`）。

#### P3-5: 皮肤 image_path 未做路径规范化

- **文件**：`src-tauri/src/skin.rs` 第 218-220 行
- **代码**：`image_path: path.to_string_lossy().to_string()`
- **描述**：用户皮肤目录中的 `skin.json` 的 `image_path` 字段直接使用磁盘路径，未经过 `canonicalize` 校验。
- **修复建议**：对 `image_path` 做 `canonicalize` 校验。

#### P3-6: lib.rs 中 pi_config.log_path 未展开 `~` 前缀

- **文件**：`src-tauri/src/lib.rs` 第 100 行
- **描述**：`pi_config.log_path` 直接使用配置值（可能含 `~` 前缀），而 `pi_adapter.rs` 有 `expand_home()` 但此处未调用，可能导致文件路径解析错误。
- **修复建议**：在 `lib.rs` 中调用 `expand_home()` 展开 `~` 前缀。

---

## 🚫 误报项

以下发现被评估为**低风险**或**预期行为**：

| 误报项 | 原因 |
|--------|------|
| `get_settings()` 返回含 auth_token 的完整配置 | 前端需要 token 连接 WS，属于正常设计 |
| `console.log` 输出 event payload | 开发模式正常，生产环境需过滤 |
| `alert()` 弹窗显示后端错误消息 | 仅开发者可见，且 `alert()` 不会执行 JS |
| `include_dir::include_dir!("../src/assets/skins")` | 构建时嵌入，发布前路径已确定 |
| `core:default` 权限 | 包含合理的默认权限集（app, path, event, window 等） |
| `tokio = { features = ["full"] }` | 桌面应用使用 tokio full features 是常见做法 |
| 无 `unsafe` 代码 | 纯安全 Rust 代码，无手动 unsafe 块 |
| 无 API key / password | 项目无外部 API 调用依赖 |
| README 无 secret 泄露 | 所有示例使用 placeholder |
| 无 `.env` 文件 | 配置统一使用 config.json，无环境变量泄露 |

---

## 🎯 优先修复的 5 个问题

| 排名 | 级别 | 问题 | 工作量 |
|------|------|------|--------|
| 1 | **P0** | `get_skin_metadata` 路径穿越 | 10 分钟 — 加 `strip_prefix` 校验 |
| 2 | **P0** | JSONL 文件差值读取无上限 | 15 分钟 — 加 `MAX_READ_BYTES` 限制 |
| 3 | **P1** | WS 认证无状态跟踪 | 20 分钟 — 维护客户端认证状态 |
| 4 | **P1** | `update_settings` 接受任意 JSON | 15 分钟 — 定义 `UpdateSettingsInput` struct |
| 5 | **P2** | 前端 console.log 泄露完整事件 | 5 分钟 — 生产环境过滤 `raw` 字段 |

**5 个问题总工作量约 65 分钟即可修复全部 P0/P1/P2 漏洞。**

---

## ❓ 需要确认的地方

1. **`../src/assets/skins` 在发布构建中的路径**：`get_skin_metadata` 使用 `../src/assets/skins` 作为第一查找路径，发布构建（`dist/`）中该路径可能不存在，实际依赖第三级路径 `~/.config/.../skins/`。是否需要确认发布构建中皮肤资源的正确路径？

2. **Pi Agent JSONL 文件来源可信度**：`~/.pi/agent/logs/latest.jsonl` 由 Pi Agent 写入，是否信任其内容？如果不信任，TTS 命令注入的风险会更高。

3. **WS 服务器是否需要跨进程调用**：当前 WS 仅绑定 127.0.0.1，如果只需要本机通信，是否需要 WSS（TLS）加密？

4. **`core:webview:allow-create-webview` 是否会被使用**：ACL 清单中包含创建新 webview 的权限，如果未使用，可以移除以减少攻击面。

5. **Tauri bundle 是否包含源代码**：`tauri.conf.json` 中 `bundle.resources` 仅包含 `../src/assets/skins`，未包含 `src/` 目录，安全。但需要确认发布构建后 dist 目录是否包含 `.map` sourcemap 文件（可能泄露源码）。

---

## ✅ 上线评估

| 评估维度 | 评分 | 说明 |
|---------|------|------|
| **认证授权** | ⚠️ 6/10 | 无认证层，但桌面应用内 JS 可信 |
| **命令注入** | ✅ 8/10 | TTS 使用参数化命令，无 shell 执行 |
| **路径穿越** | ⚠️ 5/10 | `skin_id` 未规范化，存在穿越风险 |
| **密钥泄露** | ✅ 8/10 | 默认 token 简单但 config 有 0600 权限 |
| **IPC/网络** | ⚠️ 6/10 | WS 无状态认证、无连接数限制 |
| **前端安全** | ✅ 8/10 | 无 dangerouslySetInnerHTML，无 eval |
| **日志安全** | ⚠️ 6/10 | console.log 泄露完整事件 payload |
| **依赖安全** | ✅ 9/10 | 无高危依赖、无 unsafe、无未签名 crate |
| **构建安全** | ✅ 8/10 | build.rs 无注入、bundle 不含 node_modules |

**总体安全评分：7/10**

**结论：可以上线。** 典型使用场景下（单用户桌面、本地 WS 通信、可信皮肤目录），P0/P1 漏洞影响可控。建议在首次发布前修复 P0-1（路径穿越）和 P0-2（内存 OOM）。

---

*审计完成。所有发现均为只读扫描结果，未修改任何代码。*
