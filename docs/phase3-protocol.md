# 第三阶段：协议设计

> 日期：2026-06-08
> 状态：已确认

---

## 设计原则

1. **统一抽象**：所有 Agent 事件最终统一为 `UnifiedAgentEvent`
2. **版本控制**：协议带版本号，向后兼容
3. **可扩展**：预留 `metadata` 字段供未来扩展
4. **完整追溯**：保留 `raw` 原始数据，方便调试和回溯
5. **类型安全**：TypeScript + Rust 双端类型同步生成

---

## 一、统一事件 Schema

### 1.1 统一事件格式（UnifiedAgentEvent）

这是整个系统的核心事件格式，所有 Agent Adapter 必须输出此格式。

```typescript
/**
 * 统一 Agent 事件
 * 所有 Agent Adapter 将原生事件转换为此格式
 */
interface UnifiedAgentEvent {
  /** 事件唯一标识 (ULID 格式) */
  id: string;
  
  /** ISO 8601 时间戳 */
  timestamp: string;
  
  /** 协议版本号，当前为 "1.0" */
  version: "1.0";
  
  /** 来源 Agent 标识 */
  source: AgentSource;
  
  /** 事件类别（高层分类） */
  category: EventCategory;
  
  /** 具体事件类型（细分类型） */
  type: EventType;
  
  /** 对应的宠物状态（状态机目标状态） */
  petState: PetState;
  
  /** 会话 ID（可选，某些事件无会话） */
  sessionId?: string;
  
  /** 子会话 ID（用于子 Agent 事件） */
  subSessionId?: string;
  
  /** 工具名称（仅 tool 事件有效） */
  toolName?: string;
  
  /** 工具参数预览（截断至 200 字符） */
  toolArgsPreview?: string;
  
  /** 工具执行结果预览（截断至 500 字符，仅 post 事件有效） */
  toolResultPreview?: string;
  
  /** 用户提示词/任务预览（截断至 300 字符） */
  taskPreview?: string;
  
  /** 当前迭代步数（仅 Hermes agent:step 使用） */
  stepNumber?: number;
  
  /** 是否正在等待用户审批 */
  awaitingApproval?: boolean;
  
  /** 工具是否执行成功（仅 tool 事件有效） */
  toolSuccess?: boolean;
  
  /** 错误码（仅 error 事件有效） */
  errorCode?: ErrorCode;
  
  /** 错误消息（仅 error 事件有效，截断至 500 字符） */
  errorMessage?: string;
  
  /** Agent 回复文本预览（仅 message 事件有效） */
  agentReplyPreview?: string;
  
  /** 原始数据（保留各 Agent 的完整原始事件） */
  raw: Record<string, unknown>;
  
  /** 扩展元数据（预留字段） */
  metadata?: Record<string, unknown>;
}
```

### 1.2 Agent 来源枚举

```typescript
/** Agent 来源标识 */
type AgentSource = "pi" | "hermes" | "openclaw";

/** Agent 身份详情（注册时使用） */
interface AgentIdentity {
  /** 来源标识 */
  source: AgentSource;
  /** Agent 显示名称 */
  displayName: string;
  /** Agent 版本（可选） */
  version?: string;
  /** Agent 是否在线 */
  online: boolean;
  /** 当前连接的会话 ID */
  activeSessionId?: string;
}
```

### 1.3 事件类别枚举

```typescript
/**
 * 事件类别：高层分类
 * 用于快速分类和过滤
 */
type EventCategory =
  | "session"     // 会话生命周期
  | "thinking"    // Agent 思考/推理
  | "tool"        // 工具调用
  | "message"     // Agent 回复消息
  | "permission"  // 权限请求
  | "error"       // 错误
  | "system"      // 系统事件（心跳、连接等）
  | "user"        // 用户交互
  | "subagent";   // 子 Agent 事件
```

### 1.4 事件类型枚举

```typescript
/**
 * 事件类型：与 PetState 和 EventCategory 的映射
 * 
 * 设计原则：
 * - 每种事件类型唯一映射到一个 PetState
 * - 事件类型命名使用 camelCase
 * - 类型语义来自 Agent 无关的视角
 */
type EventType =
  // ─── 会话生命周期 ───
  | "session_start"      // 会话开始 → Thinking
  | "session_end"        // 会话结束 → Idle
  | "session_compaction" // 上下文压缩 → Thinking
  
  // ─── 用户交互 ───
  | "user_prompt"        // 用户提交提示词 → Thinking
  | "user_cancel"        // 用户取消 → Idle
  
  // ─── Agent 思考 ───
  | "thinking_start"     // Agent 开始思考 → Thinking
  | "thinking_tick"      // Agent 持续思考 → Thinking
  | "thinking_end"       // Agent 思考完成 → Working/Waiting
  
  // ─── 工具调用 ───
  | "tool_call_start"    // 工具开始执行 → Working
  | "tool_call_end"      // 工具执行完成 → Thinking
  | "tool_call_error"    // 工具执行失败 → Thinking/Error
  | "tool_batch"         // 批量工具调用 → Working
  
  // ─── 权限请求 ───
  | "permission_request" // 等待用户审批 → Waiting
  | "permission_granted" // 审批通过 → Thinking
  | "permission_denied"  // 审批拒绝 → Thinking
  
  // ─── Agent 消息 ───
  | "agent_message"      // Agent 回复消息 → Thinking
  | "agent_reply"        // Agent 直接回复（无工具调用）→ Thinking
  
  // ─── 子 Agent ───
  | "subagent_start"     // 子 Agent 启动 → Working
  | "subagent_end"       // 子 Agent 结束 → Thinking
  
  // ─── 系统事件 ───
  | "heartbeat"          // 心跳事件 → Idle
  | "adapter_connected"  // 适配器连接 → Idle
  | "adapter_disconnected" // 适配器断开 → Idle
```

### 1.5 宠物状态枚举

```typescript
/**
 * 宠物状态
 * 每个状态对应一套动画
 */
type PetState =
  | "idle"           // 空闲：呼吸、发呆、随机小动作
  | "thinking"       // 思考中：歪头、眨眼、托腮
  | "working"        // 工作中：敲代码、忙碌
  | "waiting"        // 等待中：看手表、等待
  | "success"        // 成功：庆祝、开心
  | "error"          // 错误：摇头、难过
  | "speaking"       // 语音播报中：说话动画
  | "connecting";    // 连接中：加载动画
```

### 1.6 错误码枚举

```typescript
/**
 * 错误码：标准化错误分类
 */
type ErrorCode =
  // ─── 网络/连接错误 ───
  | "connection_refused"    // 连接被拒绝
  | "connection_timeout"    // 连接超时
  | "connection_lost"       // 连接丢失
  | "handshake_failed"      // 握手失败
  
  // ─── 认证错误 ───
  | "auth_failed"           // 认证失败
  | "token_expired"         // Token 过期
  | "permission_denied"     // 权限不足
  
  // ─── 工具执行错误 ───
  | "tool_not_found"        // 工具不存在
  | "tool_timeout"          // 工具超时
  | "tool_memory_limit"     // 内存限制
  | "tool_rate_limit"       // 速率限制
  
  // ─── Agent 错误 ───
  | "agent_crash"           // Agent 崩溃
  | "agent_oom"             // Agent 内存溢出
  | "agent_context_overflow" // 上下文溢出
  
  // ─── 协议错误 ───
  | "parse_error"           // 解析错误
  | "version_mismatch"      // 版本不匹配
  | "invalid_event"         // 无效事件
  
  // ─── 系统错误 ───
  | "disk_full"             // 磁盘空间不足
  | "file_locked"           // 文件被锁定
  | "process_killed"        // 进程被终止
  
  // ─── 未知错误 ───
  | "unknown";              // 未知错误
```

---

## 二、各 Agent 事件映射表

### 2.1 Pi Agent 映射

| Pi 原生事件 | 统一 category | 统一 type | PetState | 说明 |
|-------------|---------------|-----------|----------|------|
| `session_start` | session | session_start | thinking | 会话开始 |
| `user_prompt` | user | user_prompt | thinking | 用户提交提示词 |
| `text_delta` | thinking | thinking_tick | thinking | 持续生成文本 |
| `tool_call` | tool | tool_call_start | working | 工具开始执行 |
| `tool_result` | tool | tool_call_end | thinking | 工具执行完成 |
| `tool_error` | error | tool_call_error | thinking/error | 工具执行失败 |
| `turn_end` | session | session_end | idle | 回合结束 |
| `compaction` | session | session_compaction | thinking | 上下文压缩 |
| `compaction_end` | session | session_end | idle | 压缩完成 |

**Pi 事件采集方式**：
- **方式 A（推荐）**：通过 Pi TypeScript Extension 订阅事件
  - 文件位置：`~/.pi/agent/extensions/pet-event-logger.ts`
  - 通过 `ctx.on('tool_call', ...)` 等 API 捕获
  - 输出到 JSONL 文件：`~/.pi/logs/pet-events.jsonl`
  
- **方式 B（备选）**：通过 RPC 模式监听 stdin/stdout
  - 启动 Pi Agent：`pi --rpc`
  - 解析 JSON 流
  - 转换统一格式

### 2.2 Hermes Agent 映射

| Hermes 原生事件 | 统一 category | 统一 type | PetState | 说明 |
|-----------------|---------------|-----------|----------|------|
| `session:start` | session | session_start | thinking | 会话开始 |
| `agent:start` | thinking | thinking_start | thinking | Agent 开始处理 |
| `agent:step` | tool | tool_call_start | working | Agent 迭代（含工具调用） |
| `agent:end` | session | session_end | idle | Agent 处理结束 |
| `command:new` | session | session_start | thinking | 新会话 |
| `command:reset` | session | session_start | thinking | 重置会话 |
| `command:stop` | user | user_cancel | idle | 用户停止 |
| `command:*` | user | user_prompt | thinking | 命令执行 |

**Hermes 事件采集方式**：
- 通过 Gateway Hooks（HOOK.yaml + handler.py）
- handler.py 将事件写入 JSONL 文件
- Rust 端读取 JSONL 并转换

### 2.3 OpenClaw 映射

| OpenClaw 原生事件 | 统一 category | 统一 type | PetState | 说明 |
|-------------------|---------------|-----------|----------|------|
| `agent:message` | message | agent_message | thinking | Agent 发送消息 |
| `tool_result_persist` | tool | tool_call_end | thinking | 工具结果持久化 |
| `command:new` | session | session_start | thinking | 新会话 |
| `command:reset` | session | session_start | thinking | 重置会话 |
| `command:stop` | user | user_cancel | idle | 用户停止 |
| `gateway:startup` | system | adapter_connected | connecting | Gateway 启动 |
| `command:*` | user | user_prompt | thinking | 命令执行 |

**OpenClaw 事件采集方式**：
- 通过 Internal Hooks（HOOK.md 文件）
- 通过 App SDK 订阅事件
- 通过 HTTP RPC 轮询

### 2.4 事件映射验证矩阵

```
PetState  ←  Event Category
            session  thinking  tool  message  permission  error  system  user
idle        ✓        -         -     -        -           -        ✓       ✓
thinking    ✓        ✓         -     ✓        -           -        -       ✓
working     -        -         ✓     -        -           -        -       -
waiting     -        -         -     -        ✓           -        -       -
success     -        -         -     -        -           -        ✓       -
error       -        -         ✓     -        -           ✓        -       -
connecting  ✓        -         -     -        -           -        ✓       -
```

---

## 三、JSON Schema 定义

### 3.1 完整 JSON Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-pet-hub.dev/schemas/unified-agent-event/v1.0.json",
  "title": "UnifiedAgentEvent",
  "description": "统一 Agent 事件格式，所有 Agent Adapter 必须输出此格式",
  "version": "1.0",
  "type": "object",
  "required": ["id", "timestamp", "version", "source", "category", "type", "petState"],
  "properties": {
    "id": {
      "type": "string",
      "description": "事件唯一标识，使用 ULID 格式",
      "pattern": "^[0-9A-HJKMNP-TV-Z]{26}$",
      "examples": ["01ARZ3NDEKTSV4RRFFQ69G5FAV"]
    },
    "timestamp": {
      "type": "string",
      "description": "ISO 8601 时间戳",
      "format": "date-time",
      "examples": ["2026-06-08T12:00:00.000Z"]
    },
    "version": {
      "type": "string",
      "const": "1.0",
      "description": "协议版本号"
    },
    "source": {
      "type": "string",
      "enum": ["pi", "hermes", "openclaw"],
      "description": "来源 Agent 标识"
    },
    "category": {
      "type": "string",
      "enum": [
        "session",
        "thinking",
        "tool",
        "message",
        "permission",
        "error",
        "system",
        "user",
        "subagent"
      ],
      "description": "事件类别"
    },
    "type": {
      "type": "string",
      "enum": [
        "session_start",
        "session_end",
        "session_compaction",
        "user_prompt",
        "user_cancel",
        "thinking_start",
        "thinking_tick",
        "thinking_end",
        "tool_call_start",
        "tool_call_end",
        "tool_call_error",
        "tool_batch",
        "permission_request",
        "permission_granted",
        "permission_denied",
        "agent_message",
        "agent_reply",
        "subagent_start",
        "subagent_end",
        "heartbeat",
        "adapter_connected",
        "adapter_disconnected"
      ],
      "description": "事件类型"
    },
    "petState": {
      "type": "string",
      "enum": [
        "idle",
        "thinking",
        "working",
        "waiting",
        "success",
        "error",
        "speaking",
        "connecting"
      ],
      "description": "对应的宠物状态"
    },
    "sessionId": {
      "type": "string",
      "description": "会话 ID"
    },
    "subSessionId": {
      "type": "string",
      "description": "子会话 ID"
    },
    "toolName": {
      "type": "string",
      "description": "工具名称（仅 tool 事件有效）",
      "maxLength": 200
    },
    "toolArgsPreview": {
      "type": "string",
      "description": "工具参数预览（截断至 200 字符）",
      "maxLength": 200
    },
    "toolResultPreview": {
      "type": "string",
      "description": "工具结果预览（截断至 500 字符）",
      "maxLength": 500
    },
    "taskPreview": {
      "type": "string",
      "description": "用户提示词/任务预览（截断至 300 字符）",
      "maxLength": 300
    },
    "stepNumber": {
      "type": "integer",
      "description": "当前迭代步数",
      "minimum": 1
    },
    "awaitingApproval": {
      "type": "boolean",
      "description": "是否正在等待用户审批"
    },
    "toolSuccess": {
      "type": "boolean",
      "description": "工具是否执行成功"
    },
    "errorCode": {
      "type": "string",
      "enum": [
        "connection_refused",
        "connection_timeout",
        "connection_lost",
        "handshake_failed",
        "auth_failed",
        "token_expired",
        "permission_denied",
        "tool_not_found",
        "tool_timeout",
        "tool_memory_limit",
        "tool_rate_limit",
        "agent_crash",
        "agent_oom",
        "agent_context_overflow",
        "parse_error",
        "version_mismatch",
        "invalid_event",
        "disk_full",
        "file_locked",
        "process_killed",
        "unknown"
      ],
      "description": "错误码"
    },
    "errorMessage": {
      "type": "string",
      "description": "错误消息",
      "maxLength": 500
    },
    "agentReplyPreview": {
      "type": "string",
      "description": "Agent 回复文本预览",
      "maxLength": 500
    },
    "raw": {
      "type": "object",
      "description": "原始数据（保留各 Agent 的完整原始事件）",
      "additionalProperties": true
    },
    "metadata": {
      "type": "object",
      "description": "扩展元数据（预留字段）",
      "additionalProperties": true
    }
  },
  "additionalProperties": false,
  "allOf": [
    {
      "if": {
        "properties": { "category": { "const": "tool" } }
      },
      "then": {
        "required": ["toolName"]
      }
    },
    {
      "if": {
        "properties": { "category": { "const": "error" } }
      },
      "then": {
        "required": ["errorCode", "errorMessage"]
      }
    },
    {
      "if": {
        "properties": { "type": { "const": "tool_call_start" } }
      },
      "then": {
        "properties": {
          "petState": { "const": "working" }
        }
      }
    }
  ]
}
```

---

## 四、WebSocket 通信协议

### 4.1 协议概述

- **地址**：`ws://127.0.0.1:8765`（可配置）
- **认证**：Bearer Token
- **心跳**：客户端/服务端每 30 秒发送 ping/pong
- **编码**：UTF-8 JSON

### 4.2 WebSocket 消息格式

```typescript
/** WebSocket 消息统一格式 */
interface WSMessage {
  /** 消息类型 */
  type: WSMessageType;
  /** 消息 ID（用于请求-响应匹配） */
  id?: string;
  /** 消息时间戳 */
  timestamp: string;
  /** 消息负载 */
  payload: WSMessagePayload;
}

type WSMessageType =
  | "event"            // 服务端推送事件
  | "subscribe"        // 客户端订阅
  | "unsubscribe"      // 客户端取消订阅
  | "subscribed"       // 订阅确认
  | "error"            // 错误消息
  | "ping"             // 心跳请求
  | "pong"             // 心跳响应
  | "auth"             // 认证消息
  | "auth_ack"         // 认证响应
  | "agent_info"       // Agent 信息请求
  | "agent_info_ack"   // Agent 信息响应
  | "command"          // 发送命令到 Agent
  | "command_result";  // 命令结果

type WSMessagePayload =
  | WSEventPayload
  | WSSubscribePayload
  | WSErrorPayload
  | WSAuthPayload
  | WSCommandPayload;
```

### 4.3 各消息类型详细定义

```typescript
// ─── 事件推送 ───
interface WSEventPayload {
  event: UnifiedAgentEvent;
  subscriptionId?: string;
}

// ─── 订阅 ───
interface WSSubscribePayload {
  /** 订阅的事件类型列表，使用 * 表示全部 */
  eventTypes: EventType[] | "*";
  /** 订阅的事件类别列表 */
  categories?: EventCategory[];
  /** 订阅的来源 Agent 列表 */
  sources?: AgentSource[];
}

interface WSSubscribedPayload {
  subscriptionId: string;
  eventTypes: EventType[];
  categories: EventCategory[];
}

// ─── 错误 ───
interface WSErrorPayload {
  code: WSErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

type WSErrorCode =
  | "invalid_message"
  | "unauthorized"
  | "subscription_not_found"
  | "rate_limited"
  | "server_error"
  | "protocol_error";

// ─── 认证 ───
interface WSAuthPayload {
  token: string;
}

interface WSAuthAckPayload {
  authorized: boolean;
  userId?: string;
  permissions?: string[];
}

// ─── 命令 ───
interface WSCommandPayload {
  /** 目标 Agent */
  agent: AgentSource;
  /** 会话 ID */
  sessionId: string;
  /** 命令内容 */
  command: string;
}

interface WSCommandResultPayload {
  commandId: string;
  agent: AgentSource;
  sessionId: string;
  status: "success" | "error";
  result?: string;
  error?: string;
}
```

### 4.4 WebSocket 交互流程

```
客户端                          服务端
  │                               │
  │  CONNECT ws://127.0.0.1:8765  │
  │──────────────────────────────►│
  │                               │
  │  AUTH {"token": "xxx"}        │
  │──────────────────────────────►│
  │                               │
  │  AUTH_ACK {authorized: true}  │
  │◄──────────────────────────────│
  │                               │
  │  SUBSCRIBE {                   │
  │    eventTypes: ["*"],         │
  │    sources: ["pi", "hermes"]  │
  │  }                            │
  │──────────────────────────────►│
  │                               │
  │  SUBSCRIBED {                  │
  │    subscriptionId: "sub_001", │
  │    eventTypes: ["*"]          │
  │  }                            │
  │◄──────────────────────────────│
  │                               │
  │  PING                         │
  │──────────────────────────────►│
  │                               │
  │  PONG                         │
  │◄──────────────────────────────│
  │                               │
  │◄── EVENT { unified event }───│ (服务端推送)
  │◄── EVENT { unified event }───│
  │                               │
```

### 4.5 速率限制

```typescript
interface RateLimitConfig {
  /** 每秒最大事件数（默认 100） */
  maxEventsPerSecond: number;
  /** 每秒最大订阅数（默认 10） */
  maxSubscriptionsPerUser: number;
  /** 事件队列最大长度（默认 1000） */
  maxQueueSize: number;
}
```

---

## 五、Tauri 内部通信协议

### 5.1 Tauri Event 格式

Tauri 前端通过 Tauri Events 从 Rust 后端接收事件：

```typescript
// Tauri emit 事件格式
interface TauriEvent {
  /** 事件名称 */
  event: string;
  /** 事件数据 */
  payload: Record<string, unknown>;
}

// 事件名称规范
// pet:state_changed  → { petState, previousState, timestamp }
// pet:event          → { event: UnifiedAgentEvent }
// pet:agent_info     → { agents: AgentIdentity[] }
// pet:settings       → { settings: Record<string, unknown> }
```

### 5.2 Tauri Command 格式

前端通过 `invoke` 调用 Rust 后端命令：

```typescript
// 获取宠物状态
interface GetPetStateCommand {
  command: "pet:get_state";
  response: {
    petState: PetState;
    currentAgent: AgentSource | null;
    sessionCount: number;
  };
}

// 切换 Agent
interface SwitchAgentCommand {
  command: "agent:switch";
  input: { agent: AgentSource };
  response: { success: boolean; message?: string };
}

// 发送消息到 Agent
interface SendMessageCommand {
  command: "agent:send_message";
  input: {
    agent: AgentSource;
    sessionId: string;
    message: string;
  };
  response: {
    success: boolean;
    reply?: string;
    sessionId: string;
  };
}

// 获取会话列表
interface ListSessionsCommand {
  command: "agent:list_sessions";
  input: { agent: AgentSource };
  response: {
    sessions: Array<{
      id: string;
      title?: string;
      status: SessionStatus;
      startedAt: string;
    }>;
  };
}
```

---

## 六、TTS 通知协议

### 6.1 TTS 事件格式

```typescript
/** TTS 播报事件 */
interface TTSEvent {
  /** 事件唯一标识 */
  id: string;
  /** 时间戳 */
  timestamp: string;
  /** 触发的事件类型 */
  triggerEvent: EventType;
  /** 宠物状态 */
  petState: PetState;
  /** 是否播报（根据配置） */
  shouldSpeak: boolean;
  /** 播报文本 */
  text: string;
  /** 播报优先级（0=低, 1=中, 2=高） */
  priority: 0 | 1 | 2;
  /** 是否打断当前播报 */
  interruptible: boolean;
}

/** TTS 状态 */
interface TTSState {
  /** 当前状态 */
  status: "idle" | "speaking" | "paused" | "queue_full" | "error";
  /** 队列长度 */
  queueLength: number;
  /** 是否开启播报 */
  enabled: boolean;
  /** 当前播报文本（如果有） */
  currentText?: string;
  /** 音量 */
  volume: number;
  /** 语音引擎 */
  engine: "macos" | "linux-espeak" | "linux-pipewire" | "windows-edge" | "edge-tts";
}
```

### 6.2 播报规则配置

```typescript
/** TTS 播报规则 */
interface TTSSpeechRules {
  /** 会话开始播报 */
  session_start: boolean;
  /** 工具调用播报 */
  tool_call: boolean;
  /** 工具错误播报 */
  tool_error: boolean;
  /** 权限请求播报 */
  permission_request: boolean;
  /** 会话结束播报 */
  session_end: boolean;
  /** Agent 回复播报 */
  agent_message: boolean;
  /** 最小间隔（毫秒，避免连续播报） */
  minIntervalMs: number;
  /** 专注模式（静音） */
  focusMode: boolean;
}
```

---

## 七、插件事件协议

### 7.1 插件事件格式

```typescript
/** 插件可订阅的事件 */
interface PluginEvent {
  /** 事件类型 */
  type: "event" | "state_change" | "settings_change" | "command";
  /** 事件数据 */
  data: Record<string, unknown>;
  /** 时间戳 */
  timestamp: string;
}

/** 插件 API */
interface PluginAPI {
  /** 订阅事件 */
  on(event: string, handler: (data: Record<string, unknown>) => void): () => void;
  /** 获取宠物状态 */
  getPetState(): Promise<PetState>;
  /** 发送消息到 Agent */
  sendMessage(agent: AgentSource, message: string, sessionId: string): Promise<string>;
  /** 获取设置 */
  getSettings(): Promise<Record<string, unknown>>;
  /** 更新设置 */
  updateSettings(settings: Record<string, unknown>): Promise<void>;
  /** 播放动画 */
  playAnimation(animationName: string): Promise<void>;
  /** 显示通知 */
  showNotification(title: string, body: string, priority?: number): Promise<void>;
}
```

---

## 八、Schema 验证工具链

### 8.1 验证工具选择

| 工具 | Rust | TypeScript | 速度 | 推荐度 |
|------|------|------------|------|--------|
| **schemars** | ✅ 原生 | ❌ | 快 | ⭐⭐⭐⭐⭐ |
| **serde_json** | ✅ 原生 | ❌ | 快 | ⭐⭐⭐⭐ |
| **ajv** | ❌ | ✅ | 中 | ⭐⭐⭐⭐ |
| **zod** | ❌ | ✅ | 慢 | ⭐⭐⭐⭐ |
| **typescript-json-schema** | ✅ (生成) | ✅ | 中 | ⭐⭐⭐ |

**推荐方案**：
- **Rust 端**：使用 `schemars` + `serde_json` 生成 JSON Schema
- **TypeScript 端**：使用 `zod` 进行运行时验证（开发快）+ `ajv`（生产验证）
- **双向同步**：从 TypeScript `zod` schema 生成 JSON Schema，Rust 侧使用 `schemars` 验证

### 8.2 TypeScript 验证示例

```typescript
import { z } from "zod";

// 定义事件 schema
const UnifiedAgentEventSchema = z.object({
  id: z.string().regex(/^[0-9A-HJKMNP-TV-Z]{26}$/),
  timestamp: z.string().datetime(),
  version: z.literal("1.0"),
  source: z.enum(["pi", "hermes", "openclaw"]),
  category: z.enum([
    "session", "thinking", "tool", "message",
    "permission", "error", "system", "user", "subagent"
  ]),
  type: z.enum([
    "session_start", "session_end", "session_compaction",
    "user_prompt", "user_cancel",
    "thinking_start", "thinking_tick", "thinking_end",
    "tool_call_start", "tool_call_end", "tool_call_error", "tool_batch",
    "permission_request", "permission_granted", "permission_denied",
    "agent_message", "agent_reply",
    "subagent_start", "subagent_end",
    "heartbeat", "adapter_connected", "adapter_disconnected"
  ]),
  petState: z.enum([
    "idle", "thinking", "working", "waiting",
    "success", "error", "speaking", "connecting"
  ]),
  sessionId: z.string().optional(),
  toolName: z.string().max(200).optional(),
  toolArgsPreview: z.string().max(200).optional(),
  toolResultPreview: z.string().max(500).optional(),
  taskPreview: z.string().max(300).optional(),
  errorCode: z.enum([
    "connection_refused", "connection_timeout", "connection_lost",
    "auth_failed", "tool_not_found", "agent_crash", "unknown"
  ]).optional(),
  errorMessage: z.string().max(500).optional(),
  raw: z.record(z.unknown()),
  metadata: z.record(z.unknown()).optional(),
});

// 验证函数
function validateEvent(data: unknown): UnifiedAgentEvent {
  return UnifiedAgentEventSchema.parse(data);
}
```

### 8.3 Rust 验证示例

```rust
use schemars::schema_for;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedAgentEvent {
    pub id: String,
    pub timestamp: String,
    pub version: String,
    pub source: AgentSource,
    pub category: EventCategory,
    pub event_type: EventType,
    pub pet_state: PetState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_args_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_reply_preview: Option<String>,
    pub raw: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// 生成 JSON Schema
fn generate_schema() -> String {
    let schema = schema_for!(UnifiedAgentEvent);
    serde_json::to_string_pretty(&schema).unwrap()
}
```

---

## 九、协议版本管理

### 9.1 版本策略

```
主版本.次版本-修订号
  │    │    │
  │    │    └─ 向后兼容的修正（字段添加、默认值变更）
  │    └───── 新增功能（保留字段，不破坏现有客户端）
  └────────── 破坏性变更（需迁移工具）
```

### 9.2 当前版本

```
v1.0.0 (当前)
  - 初始版本
  - 包含核心事件格式
  - 包含 WebSocket 协议
  - 包含 TTS 协议
  - 包含插件事件协议
```

### 9.3 向后兼容规则

| 变更类型 | 是否兼容 | 示例 |
|----------|----------|------|
| 添加可选字段 | ✅ | 新增 `subSessionId` |
| 添加枚举值 | ✅ | 新增 `tool_batch` 事件类型 |
| 添加可选枚举值 | ✅ | `errorCode` 新增值 |
| 移除可选字段 | ✅ | 移除已弃用的字段 |
| 修改必填字段 | ⚠️ | `sessionId` 从可选改为必填 |
| 修改字段类型 | ❌ | `source` 从 string 改为 object |
| 修改枚举值含义 | ❌ | `thinking` 事件含义改变 |
| 修改协议版本号 | ❌ | `version` 从 `"1.0"` 改为 `"2.0"` |

---

## 十、协议使用示例

### 10.1 Pi Agent 事件转换示例

**Pi 原生事件**（JSON 格式）：
```json
{
  "type": "tool_call",
  "tool": "Bash",
  "args": "ls -la /home/user/project",
  "id": "tool_abc123"
}
```

**转换后**（UnifiedAgentEvent）：
```json
{
  "id": "01KXYZ1234ABCDEF5678GHIJKL",
  "timestamp": "2026-06-08T12:00:00.000Z",
  "version": "1.0",
  "source": "pi",
  "category": "tool",
  "type": "tool_call_start",
  "petState": "working",
  "sessionId": "session_xyz789",
  "toolName": "Bash",
  "toolArgsPreview": "ls -la /home/user/project",
  "raw": {
    "type": "tool_call",
    "tool": "Bash",
    "args": "ls -la /home/user/project",
    "id": "tool_abc123"
  },
  "metadata": {
    "pi_event_id": "tool_abc123"
  }
}
```

### 10.2 Hermes Agent 事件转换示例

**Hermes 原生事件**（Gateway Hook 输出）：
```json
{
  "event_type": "agent:step",
  "context": {
    "platform": "cli",
    "user_id": "user_001",
    "session_id": "sess_hermes_001",
    "iteration": 3,
    "tool_names": ["Bash", "Read"]
  }
}
```

**转换后**（UnifiedAgentEvent）：
```json
{
  "id": "01KXYZ1234ABCDEF5678GHIJKM",
  "timestamp": "2026-06-08T12:00:01.000Z",
  "version": "1.0",
  "source": "hermes",
  "category": "tool",
  "type": "tool_call_start",
  "petState": "working",
  "sessionId": "sess_hermes_001",
  "toolName": "Bash",
  "toolArgsPreview": "[multi-tool iteration, tools: Bash, Read]",
  "stepNumber": 3,
  "raw": {
    "event_type": "agent:step",
    "context": {
      "platform": "cli",
      "user_id": "user_001",
      "session_id": "sess_hermes_001",
      "iteration": 3,
      "tool_names": ["Bash", "Read"]
    }
  }
}
```

### 10.3 WebSocket 客户端订阅示例

```javascript
// 客户端连接
const ws = new WebSocket("ws://127.0.0.1:8765");

ws.onopen = () => {
  // 认证
  ws.send(JSON.stringify({
    type: "auth",
    id: "auth_001",
    timestamp: new Date().toISOString(),
    payload: { token: "my-secret-token" }
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  
  switch (msg.type) {
    case "auth_ack":
      if (msg.payload.authorized) {
        // 订阅所有事件
        ws.send(JSON.stringify({
          type: "subscribe",
          id: "sub_001",
          timestamp: new Date().toISOString(),
          payload: {
            eventTypes: ["*"],
            categories: ["tool", "thinking", "session"],
            sources: ["pi", "hermes", "openclaw"]
          }
        }));
      }
      break;
      
    case "subscribed":
      console.log("Subscribed to:", msg.payload.eventTypes);
      break;
      
    case "event":
      console.log("New event:", msg.payload.event);
      // 更新 UI
      updatePetAnimation(msg.payload.event.petState);
      break;
  }
};
```

---

## 十一、协议文件清单

以下是协议设计完成后需要创建的文件：

```
packages/protocol/
├── package.json
├── src/
│   ├── events.ts              # TypeScript 类型定义
│   ├── schemas.ts             # Zod schema 定义
│   ├── mapping.ts             # Agent 事件映射表
│   ├── validators.ts          # 验证函数
│   └── index.ts               # 导出
├── schemas/
│   ├── unified-agent-event.schema.json  # JSON Schema
│   ├── ws-message.schema.json           # WebSocket 消息 Schema
│   └── tts-event.schema.json            # TTS 事件 Schema
└── tests/
    ├── events.test.ts         # 类型测试
    ├── mapping.test.ts        # 映射测试
    └── validators.test.ts     # 验证测试
```

---

## 十二、技术决策说明

### 为什么使用 ULID 作为事件 ID？

| 方案 | 排序性 | 唯一性 | 可读性 | 推荐度 |
|------|--------|--------|--------|--------|
| UUID v4 | ❌ 无序 | ✅ | ⚠️ | ⭐⭐⭐ |
| UUID v7 | ✅ 有序 | ✅ | ⚠️ | ⭐⭐⭐⭐ |
| **ULID** | ✅ 有序 | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| Timestamp | ❌ 可能重复 | ⚠️ | ✅ | ⭐⭐ |

**决策**：ULID
- 时间有序，便于日志排序
- 128 位熵，唯一性极高
- 字符串格式，可读性好
- 无前导零问题

### 为什么分层设计 category + type？

- **category**（事件类别）：用于快速过滤和路由
- **type**（事件类型）：用于精确的状态转换
- 两层设计支持灵活的事件过滤策略

### 为什么保留 raw 字段？

- 各 Agent 的原生事件格式可能变化
- 调试时需要查看原始数据
- 未来新增 Agent 时可以参考原始格式
- 支持事件回放和重放功能

---

*本协议设计为项目核心契约，后续所有 Adapter 和客户端必须遵循。*
