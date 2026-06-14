# 第一阶段：信息收集调研报告

> 完成日期：2026-06-08
> 调研人：首席架构师

---

## 一、已发现项目

### 1. OpenPets (alvinunreal/openpets)
- **项目链接**：https://github.com/alvinunreal/openpets
- **Stars**：832
- **许可证**：MIT
- **最后更新**：2026-05（活跃）
- **主要功能**：
  - 桌面宠物应用，支持多 Agent（Claude Code, Codex, OpenClaw, Pi）
  - MCP 本地服务连接 Claude Code
  - 插件系统：宠物选择器、动画管理器、事件总线、插件扩展
  - 支持多种动画格式（Lottie, Live2D, Spine, Pixel Art, CSS 动画, WebGL）
  - 多桌面支持
  - 状态同步：思考中、写作中、等待中、完成、错误
  - CLI 控制 + 系统托盘
  - 跨平台：macOS, Windows, Linux
- **架构**：
  - pnpm monorepo
  - apps/ (桌面应用)
  - packages/ (核心库)
  - plugins/ (插件系统)
  - skills/ (技能)
- **优点**：
  - 架构最成熟、最完整
  - 插件系统设计良好（PluginInterface + PluginManager）
  - 事件总线已实现（EventBus）
  - 多格式动画支持
  - MCP 集成已有现成方案
  - 支持多宠物切换
  - TypeScript 优先
- **缺点**：
  - macOS 优先（但已支持 Windows/Linux）
  - 主要面向 Claude Code/Codex，OpenClaw 支持有限
  - 宠物动画偏静态（Lottie/SVG），Live2D 需要额外 SDK
  - 没有 Pi Agent 的原生深度集成（仅通过 MCP）
  - 代码量大，学习成本高
- **是否值得复用**：⭐⭐⭐⭐⭐ 核心架构可直接参考，事件总线+插件系统可复用

### 2. OpenPets (alterhq/openpets)
- **项目链接**：https://github.com/alterhq/openpets
- **Stars**：72
- **最后更新**：2026-05
- **主要功能**：
  - macOS 原生桌面宠物
  - MCP/CLI 控制
  - Codex Pets 支持
  - Swift 包（OpenPetsKit）
- **优点**：
  - 原生 macOS 体验优秀
  - Swift Package Manager 集成
- **缺点**：
  - 仅 macOS
  - 生态较小
  - 单一 Agent 支持
- **是否值得复用**：⭐⭐⭐ Swift 代码可参考，但不跨平台

### 3. DesktopClaw
- **项目链接**：https://github.com/divbasson/DesktopClaw
- **Stars**：13
- **最后更新**：2026-06（活跃）
- **主要功能**：
  - OpenClaw 专用的状态感知桌面宠物
  - 语音交互 + 文本交互
  - 实时会话可见性
  - 任务反馈
  - 快速动作触发
- **优点**：
  - OpenClaw 深度集成
  - 专注于单一 Agent 场景
  - 轻量级
- **缺点**：
  - 仅支持 OpenClaw
  - 社区规模小
  - 插件系统不完善
- **是否值得复用**：⭐⭐⭐⭐ 事件监听逻辑可参考

### 4. KKClaw
- **项目链接**：https://github.com/kk43994/kkclaw
- **主要功能**：
  - OpenClaw 桌面龙虾宠物
  - Edge TTS 语音
  - 情绪动画
- **优点**：语音集成方案可参考
- **缺点**：单一 Agent，社区小
- **是否值得复用**：⭐⭐ 语音方案可参考

### 5. Super Agent Party
- **项目链接**：https://github.com/heshengtao/super-agent-party
- **主要功能**：
  - VRM 桌面宠物机器人
  - 自定义头像
  - 自定义动画
  - 语音交互
  - 对话中断
- **优点**：
  - VRM 格式支持
  - 多 Agent 概念
  - 语音交互成熟
- **缺点**：VRM 格式较重，需 3D 渲染
- **是否值得复用**：⭐⭐⭐ VRM 和语音方案可参考

### 6. agent-hooks-playground
- **项目链接**：https://github.com/heruujoko/ai-weekend-lab
- **Stars**：~161/mo 下载（npm）
- **主要功能**：
  - 将 Claude Code、Codex、Pi 三个 Agent 的生命周期事件标准化为统一的 JSONL 日志
  - 提供 CLI 工具进行事件汇总
- **优点**：
  - **已经实现了三个 Agent 的事件标准化**
  - 统一事件 Schema
  - 轻量级，可直接复用
  - 已有 Pi 的 TypeScript Extension 集成
- **缺点**：
  - 仅输出日志，无 UI
  - 不支持 OpenClaw/Hermes
  - 事件粒度较粗（只有 session/tool_call/stop）
- **是否值得复用**：⭐⭐⭐⭐⭐ **核心事件标准化方案可直接参考**

---

## 二、官方文档与关键能力

### 1. Pi Agent
- **文档链接**：https://pi.dev/
- **GitHub**：https://github.com/earendil-works/pi (60.8k ⭐)
- **关键能力**：
  - **Hooks**：TypeScript extensions，支持 tool_call, compaction, turn_end 等事件
  - **Extensions API**：`ctx.on('tool_call', ...)` 拦截工具调用
  - **RPC 模式**：stdin/stdout JSON 协议，可被外部进程监听
  - **SDK**：Node.js SDK，支持事件订阅
  - **TUI 扩展**：可通过 TUI API 显示自定义组件
  - **四种模式**：interactive, print/JSON, RPC, SDK
  - **事件类型**：session_start, tool_call, text_delta, turn_end, compaction 等
  - **事件拦截**：可 block/modify tool calls, inject context

### 2. Hermes Agent
- **文档链接**：https://hermes-agent.nousresearch.com/docs/
- **GitHub**：https://github.com/NousResearch/hermes-agent (186.4k ⭐)
- **关键能力**：
  - **三种 Hook 系统**：
    1. Gateway hooks: `HOOK.yaml` + `handler.py`（仅 Gateway）
    2. Plugin hooks: `ctx.register_hook()`（CLI + Gateway）
    3. Shell hooks: `hooks:` block in `config.yaml`（CLI + Gateway）
  - **事件类型**：session:start, agent:start, agent:step, agent:end, command:*
  - **Webhooks**：动态 webhook 订阅
  - **CLI 扩展**：`HermesCLI` 暴露 protected extension hooks
  - **Kanban 多 Agent**：支持多 Agent 调度
  - **Tool Gateway**：可监听工具调用
- **注意**：Hermes 有 CLI 和 Desktop 两种形态

### 3. OpenClaw
- **文档链接**：https://docs.openclaw.ai/
- **GitHub**：https://github.com/openclaw/openclaw
- **关键能力**：
  - **Internal Hooks**：`HOOK.md` 文件触发，支持 `/new`, `/reset`, `/stop` 等命令
  - **Plugin Hooks**：`api.on(...)` 类型化钩子，有 priority/merge/block 语义
  - **Webhooks**：外部 HTTP 端点触发工作
  - **App SDK**：`@openclaw/sdk` 外部应用 API
  - **Admin HTTP RPC**：`openclaw gateway call` 可 RPC 调用
  - **事件类型**：gateway:startup, agent:message, command:new, command:reset, command:stop, tool_result_persist
  - **Tool 事件**：正在开发中（issue #10502）
  - **钩子发现**：`openclaw hooks list` 可发现所有钩子

### 4. Claude Code
- **文档链接**：https://code.claude.com/docs/en/agent-sdk/hooks
- **关键能力**：
  - **Hooks**：callback functions + shell command hooks
  - **事件类型**：PreToolUse, PostToolUse, PreSubagentCall, PostSubagentCall, PreSessionPromptInjection, SessionIdle, SessionEnd, ToolError
  - **Matchers**：`"Write|Edit"` 模式匹配工具名
  - **输入输出**：可修改 tool input, require human approval, auto-approve
  - **MCP**：MCP tools 以 `mcp__server__tool` 命名出现在 tool events 中
  - **Shell Hooks**：`settings.json` 配置
  - **JSONL 输出**：`--output-format jsonl`
  - **Session Hooks**：session 级别的持久化 hooks

### 5. OpenClaw App SDK
- **文档链接**：https://docs.openclaw.ai/concepts/openclaw-sdk
- **关键能力**：
  - 外部应用 API
  - 可用于 dashboard、IDE 扩展、CLI 工具
  - 支持订阅 agent 运行状态

---

## 三、已发现的现成解决方案

### ✅ 事件标准化
- **agent-hooks-playground** 已实现 Claude Code / Codex / Pi 的标准化事件
- 统一 Schema: `AgentEvent { id, ts, agent, event, sessionId, cwd, toolName, promptPreview, summary, source, raw }`
- **可直接复用或参考其设计**

### ✅ 桌面宠物 UI
- **OpenPets** 提供了完整的桌面宠物框架（状态管理 + 动画 + 插件）
- 支持 Lottie / Live2D / Spine / Pixel Art / CSS / WebGL
- 插件系统设计良好

### ✅ MCP 通信
- MCP (Model Context Protocol) 已是 Anthropic 主导的标准协议
- OpenPets 已有 MCP 本地服务器实现
- 可用于 Agent <-> Pet 的状态同步

### ✅ Shell Hook 通用方案
- Claude Code / Hermes / OpenClaw 都支持 Shell Hook
- 意味着可以用 shell script 做最通用的事件采集
- 不需要每个 Agent 都写 SDK 集成

### ❌ 没有的多功能统一方案
- 没有一个项目同时覆盖：Pi + Hermes + OpenClaw + Claude Code + 未来 Agent
- 没有统一的跨平台桌面宠物框架支持所有这些 Agent
- **这正是我们的机会**

---

## 四、风险点分析

### 🔴 高风险
1. **各 Agent 生命周期模型不统一**
   - Pi: 基于 extension events (tool_call, text_delta, turn_end)
   - Claude Code: 基于 SDK hooks (PreToolUse, PostToolUse)
   - Hermes: 基于 gateway events (agent:start, agent:step, agent:end)
   - OpenClaw: 基于 hooks (agent:message, tool_result_persist)
   - **风险**：需要设计强大的抽象层来统一这些差异

2. **工具调用事件的粒度差异**
   - Pi 和 Claude Code 有 pre/post tool call
   - Hermes 只有 agent:step (包含工具名列表)
   - OpenClaw 只有 tool_result_persist
   - **风险**：状态机的 `working` 状态判定在不同 Agent 上有差异

3. **Windows 权限问题**
   - Tauri 在 Windows 上需要安装 WebView2
   - 悬浮窗在 Windows 上的 z-order 管理
   - 鼠标穿透（click-through）在 Windows 上的实现

### 🟡 中风险
4. **IPC 通讯兼容问题**
   - 不同 Agent 的 hook 触发时机不同
   - Shell hook 有延迟（进程启动开销）
   - WebSocket 需要处理断线重连
   - 需要设计优雅降级机制

5. **Live2D 性能问题**
   - Live2D Cubism SDK for Web 需要 WASM
   - 桌面应用中的渲染性能
   - 动画切换的流畅性
   - 多实例（多个桌宠）的内存占用

6. **Agent SDK 版本差异**
   - Claude Code SDK v1 vs v2
   - Pi Agent SDK 频繁更新
   - Hermes Agent 的钩子机制在演进中（issue #25204 显示 shell hooks 有 bug）

### 🟢 低风险
7. **桌面框架选择**
   - Tauri 2.x 已经稳定，生态成熟
   - Electron 更稳定但内存占用高
   - 对于桌宠场景，Tauri 足够

8. **动画资源**
   - Lottie 动画格式成熟、轻量
   - Live2D 模型有丰富社区资源
   - CSS 动画不需要额外 SDK

---

## 五、技术选型建议

### 桌面框架
| 方案 | 推荐度 | 理由 |
|------|--------|------|
| **Tauri 2.x** | ⭐⭐⭐⭐⭐ | 内存占用极低（~30-40MB vs Electron 200-300MB），bundle 小 96%，Rust 后端 + Web 前端，WebView2/WebKitGTK 原生渲染 |
| Electron | ⭐⭐⭐ | 生态最成熟但内存占用高，桌宠需要常驻运行，Electron 太重 |
| Neutralinojs | ⭐⭐ | 轻量但生态小，社区不活跃 |

**选择 Tauri 2.x**，原因：
1. 桌宠需要 7x24 运行，内存占用是关键
2. Tauri 2.x 已发布正式版，生态成熟
3. WebView2 (Windows) + WebKitGTK (Linux) + WKWebView (macOS)
4. Rust 后端可以做高效的 IPC 和事件处理
5. 社区活跃（50k+ ⭐）

### 动画方案
| 方案 | 推荐度 | 理由 |
|------|--------|------|
| **Lottie + CSS 动画** | ⭐⭐⭐⭐⭐ | 轻量、跨平台、设计师友好、SVG 缩放无损 |
| Live2D Cubism SDK for Web | ⭐⭐⭐⭐ | 适合角色类桌宠，但需要 WASM，模型制作门槛高 |
| Spine 2D | ⭐⭐⭐ | 游戏引擎出身，Web 支持不如 Live2D |
| 纯 CSS/SVG 动画 | ⭐⭐⭐⭐ | 最简单，适合像素风格 |

**建议**：默认使用 Lottie + CSS，可选集成 Live2D 作为高级选项。

### IPC 方案
| 方案 | 推荐度 | 理由 |
|------|--------|------|
| **Tauri Events** | ⭐⭐⭐⭐⭐ | Rust -> JS 双向通信，低延迟，原生支持 |
| **WebSocket** | ⭐⭐⭐⭐ | 适合外部进程监听，但需要管理连接 |
| **Named Pipes / FIFO** | ⭐⭐⭐ | Unix 系统友好，Windows 需命名管道 |
| **Unix Domain Socket** | ⭐⭐⭐ | Linux 友好，跨平台需适配 |

**建议**：内部用 Tauri Events，外部用 WebSocket（让其他进程可以连接监听）。

### 事件采集方案
| 方案 | 推荐度 | 理由 |
|------|--------|------|
| **Shell Hook + stdin/stdout** | ⭐⭐⭐⭐⭐ | 所有 Agent 都支持，最简单通用 |
| **Agent SDK Extension** | ⭐⭐⭐⭐ | 更精确的事件粒度，但需要各 Agent 的 SDK |
| **MCP 协议** | ⭐⭐⭐ | 新兴标准，但生态还在发展中 |
| **JSONL 文件** | ⭐⭐⭐⭐ | agent-hooks-playground 已验证可行 |

**建议**：
- 短期：Shell Hook + JSONL 文件（最通用）
- 中期：各 Agent SDK Extension（更精确）
- 长期：MCP 协议（行业标准）

---

## 六、需要用户确认的问题

以下是会影响架构设计的关键问题，请确认：

### Q1: 桌宠的视觉风格？
- [ ] A) 2D 卡通风格（像素艺术 / SVG 矢量）
- [ ] B) Live2D 风格（日本动漫角色）
- [ ] C) 3D VRM 风格
- [ ] D) 简洁 UI 风格（状态指示器 + 简单动画）
- [ ] E) 以上全部支持，让用户选择

### Q2: 桌宠的交互方式？
- [ ] A) 仅被动展示状态（不交互）
- [ ] B) 可点击交互（点击有反馈）
- [ ] C) 可拖拽移动
- [ ] D) 可对话（桌宠回复用户消息）
- [ ] E) 以上全部支持

### Q3: 桌面框架偏好？
- [ ] A) Tauri 2.x（推荐，轻量高效）
- [ ] B) Electron（生态成熟）
- [ ] C) 用户决定

### Q4: 目标操作系统？
- [ ] A) 仅 Linux
- [ ] B) Linux + macOS
- [ ] C) 全平台（Linux + macOS + Windows）
- [ ] D) 用户决定

### Q5: 桌宠常驻模式？
- [ ] A) 桌面悬浮窗（始终可见，可穿透点击）
- [ ] B) 系统托盘图标（点击展开面板）
- [ ] C) 两者都有（托盘图标 + 可选悬浮窗）
- [ ] D) 用户决定

### Q6: MVP Agent 范围？
- [ ] A) 先实现 Pi Agent + Claude Code（已有成熟方案）
- [ ] B) 先实现 OpenClaw（DesktopClaw 已有参考）
- [ ] C) 先实现所有三种（Pi, Hermes, OpenClaw）
- [ ] D) 用户决定

### Q7: 插件系统设计优先级？
- [ ] A) MVP 包含基础插件系统（仅动画皮肤）
- [ ] B) MVP 不包含插件系统，第二阶段加入
- [ ] C) 用户决定

### Q8: 是否需要语音功能？
- [ ] A) 不需要
- [ ] B) 需要（TTS 播报 Agent 状态）
- [ ] C) 用户决定

---

## 七、调研结论

### 核心发现
1. **OpenPets (alvinunreal)** 是目前最完整的桌宠项目，架构可直接参考
2. **agent-hooks-playground** 已实现 3 个 Agent 的事件标准化，可复用
3. 没有一个项目同时覆盖所有目标 Agent，这是市场机会
4. Tauri 2.x + Lottie 是最佳技术栈选择

### 下一步
等待用户确认以上 8 个问题后，进入 **第二阶段：架构设计**。

---

*本调研文档为项目机密，未经授权不得外传。*
