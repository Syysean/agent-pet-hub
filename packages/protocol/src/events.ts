/**
 * Unified Agent Event Protocol — Type Definitions
 *
 * All types derived from phase3-protocol.md (v1.0)
 */

// ─── Agent Source ───────────────────────────────────────────────────────────

/** Agent 来源标识 */
export type AgentSource = "pi" | "hermes" | "openclaw";

/** Agent 身份详情（注册时使用） */
export interface AgentIdentity {
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

// ─── Event Category ─────────────────────────────────────────────────────────

/**
 * 事件类别：高层分类
 * 用于快速分类和过滤
 */
export type EventCategory =
  | "session"     // 会话生命周期
  | "thinking"    // Agent 思考/推理
  | "tool"        // 工具调用
  | "message"     // Agent 回复消息
  | "permission"  // 权限请求
  | "error"       // 错误
  | "system"      // 系统事件（心跳、连接等）
  | "user"        // 用户交互
  | "subagent";   // 子 Agent 事件

// ─── Event Type ─────────────────────────────────────────────────────────────

/**
 * 事件类型：与 PetState 和 EventCategory 的映射
 *
 * 设计原则：
 * - 每种事件类型唯一映射到一个 PetState
 * - 事件类型命名使用 camelCase
 * - 类型语义来自 Agent 无关的视角
 */
export type EventType =
  // ─── 会话生命周期 ───
  | "session_start"        // 会话开始 → Thinking
  | "session_end"          // 会话结束 → Idle
  | "session_compaction"   // 上下文压缩 → Thinking

  // ─── 用户交互 ───
  | "user_prompt"          // 用户提交提示词 → Thinking
  | "user_cancel"          // 用户取消 → Idle

  // ─── Agent 思考 ───
  | "thinking_start"       // Agent 开始思考 → Thinking
  | "thinking_tick"        // Agent 持续思考 → Thinking
  | "thinking_end"         // Agent 思考完成 → Working/Waiting

  // ─── 工具调用 ───
  | "tool_call_start"      // 工具开始执行 → Working
  | "tool_call_end"        // 工具执行完成 → Thinking
  | "tool_call_error"      // 工具执行失败 → Thinking/Error
  | "tool_batch"           // 批量工具调用 → Working

  // ─── 权限请求 ───
  | "permission_request"   // 等待用户审批 → Waiting
  | "permission_granted"   // 审批通过 → Thinking
  | "permission_denied"    // 审批拒绝 → Thinking

  // ─── Agent 消息 ───
  | "agent_message"        // Agent 回复消息 → Thinking
  | "agent_reply"          // Agent 直接回复（无工具调用）→ Thinking

  // ─── 子 Agent ───
  | "subagent_start"       // 子 Agent 启动 → Working
  | "subagent_end"         // 子 Agent 结束 → Thinking

  // ─── 系统事件 ───
  | "heartbeat"            // 心跳事件 → Idle
  | "adapter_connected"    // 适配器连接 → Idle
  | "adapter_disconnected"; // 适配器断开 → Idle

// ─── Pet State ──────────────────────────────────────────────────────────────

/**
 * 宠物状态
 * 每个状态对应一套动画
 */
export type PetState =
  | "idle"           // 空闲：呼吸、发呆、随机小动作
  | "thinking"       // 思考中：歪头、眨眼、托腮
  | "working"        // 工作中：敲代码、忙碌
  | "waiting"        // 等待中：看手表、等待
  | "success"        // 成功：庆祝、开心
  | "error"          // 错误：摇头、难过
  | "speaking"       // 语音播报中：说话动画
  | "connecting";    // 连接中：加载动画

// ─── Error Code ─────────────────────────────────────────────────────────────

/**
 * 错误码：标准化错误分类
 */
export type ErrorCode =
  // ─── 网络/连接错误 ───
  | "connection_refused"   // 连接被拒绝
  | "connection_timeout"   // 连接超时
  | "connection_lost"      // 连接丢失
  | "handshake_failed"     // 握手失败

  // ─── 认证错误 ───
  | "auth_failed"          // 认证失败
  | "token_expired"        // Token 过期
  | "permission_denied"    // 权限不足

  // ─── 工具执行错误 ───
  | "tool_not_found"       // 工具不存在
  | "tool_timeout"         // 工具超时
  | "tool_memory_limit"    // 内存限制
  | "tool_rate_limit"      // 速率限制

  // ─── Agent 错误 ───
  | "agent_crash"          // Agent 崩溃
  | "agent_oom"            // Agent 内存溢出
  | "agent_context_overflow" // 上下文溢出

  // ─── 协议错误 ───
  | "parse_error"          // 解析错误
  | "version_mismatch"     // 版本不匹配
  | "invalid_event"        // 无效事件

  // ─── 系统错误 ───
  | "disk_full"            // 磁盘空间不足
  | "file_locked"          // 文件被锁定
  | "process_killed"       // 进程被终止

  // ─── 未知错误 ───
  | "unknown";             // 未知错误

// ─── Unified Agent Event ────────────────────────────────────────────────────

/**
 * 统一 Agent 事件
 * 所有 Agent Adapter 将原生事件转换为此格式
 *
 * 版本：1.0
 */
export interface UnifiedAgentEvent {
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

// ─── WebSocket Message ──────────────────────────────────────────────────────

/** WebSocket 消息统一格式 */
export interface WSMessage {
  /** 消息类型 */
  type: WSMessageType;
  /** 消息 ID（用于请求-响应匹配） */
  id?: string;
  /** 消息时间戳 */
  timestamp: string;
  /** 消息负载 */
  payload: WSMessagePayload;
}

/** WebSocket 消息类型 */
export type WSMessageType =
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

/** WebSocket 消息负载联合类型 */
export type WSMessagePayload =
  | WSEventPayload
  | WSSubscribePayload
  | WSSubscribedPayload
  | WSErrorPayload
  | WSAuthPayload
  | WSAuthAckPayload
  | WSCommandPayload
  | WSCommandResultPayload;

// ─── WebSocket Payload Types ────────────────────────────────────────────────

// 事件推送
export interface WSEventPayload {
  event: UnifiedAgentEvent;
  subscriptionId?: string;
}

// 订阅
export interface WSSubscribePayload {
  /** 订阅的事件类型列表，使用 "*" 表示全部 */
  eventTypes: EventType[] | "*";
  /** 订阅的事件类别列表 */
  categories?: EventCategory[];
  /** 订阅的来源 Agent 列表 */
  sources?: AgentSource[];
}

// 订阅确认
export interface WSSubscribedPayload {
  subscriptionId: string;
  eventTypes: EventType[];
  categories: EventCategory[];
}

// 错误
export interface WSErrorPayload {
  code: WSErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

export type WSErrorCode =
  | "invalid_message"
  | "unauthorized"
  | "subscription_not_found"
  | "rate_limited"
  | "server_error"
  | "protocol_error";

// 认证
export interface WSAuthPayload {
  token: string;
}

export interface WSAuthAckPayload {
  authorized: boolean;
  userId?: string;
  permissions?: string[];
}

// 命令
export interface WSCommandPayload {
  /** 目标 Agent */
  agent: AgentSource;
  /** 会话 ID */
  sessionId: string;
  /** 命令内容 */
  command: string;
}

export interface WSCommandResultPayload {
  commandId: string;
  agent: AgentSource;
  sessionId: string;
  status: "success" | "error";
  result?: string;
  error?: string;
}

// ─── TTS Protocol ───────────────────────────────────────────────────────────

/** TTS 播报事件 */
export interface TTSEvent {
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
export interface TTSState {
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

/** TTS 播报规则配置 */
export interface TTSSpeechRules {
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

// ─── Plugin Protocol ────────────────────────────────────────────────────────

/** 插件可订阅的事件 */
export interface PluginEvent {
  /** 事件类型 */
  type: "event" | "state_change" | "settings_change" | "command";
  /** 事件数据 */
  data: Record<string, unknown>;
  /** 时间戳 */
  timestamp: string;
}

/** 插件 API */
export interface PluginAPI {
  /** 订阅事件 */
  on(
    event: string,
    handler: (data: Record<string, unknown>) => void
  ): () => void;

  /** 获取宠物状态 */
  getPetState(): Promise<PetState>;

  /** 发送消息到 Agent */
  sendMessage(
    agent: AgentSource,
    message: string,
    sessionId: string
  ): Promise<string>;

  /** 获取设置 */
  getSettings(): Promise<Record<string, unknown>>;

  /** 更新设置 */
  updateSettings(settings: Record<string, unknown>): Promise<void>;

  /** 播放动画 */
  playAnimation(animationName: string): Promise<void>;

  /** 显示通知 */
  showNotification(
    title: string,
    body: string,
    priority?: number
  ): Promise<void>;
}
