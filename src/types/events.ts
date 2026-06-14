/**
 * 事件类型定义 — 与 packages/protocol 对齐
 *
 * 前端直接从协议包导入类型，此处提供本地副本
 * 确保类型一致且前端构建不依赖 dist 产物
 */

// ─── Agent 来源 ───────────────────────────────────────────────────────────

/** Agent 来源标识 */
export type AgentSource = "pi" | "hermes" | "openclaw";

// ─── 事件类别 ─────────────────────────────────────────────────────────────

/**
 * 事件类别：高层分类
 * 用于快速分类和过滤
 */
export type EventCategory =
  | "session"    // 会话生命周期
  | "thinking"   // Agent 思考/推理
  | "tool"       // 工具调用
  | "message"    // Agent 回复消息
  | "permission" // 权限请求
  | "error"      // 错误
  | "system"     // 系统事件（心跳、连接等）
  | "user"       // 用户交互
  | "subagent";  // 子 Agent 事件

// ─── 事件类型 ─────────────────────────────────────────────────────────────

/**
 * 事件类型：与 PetState 和 EventCategory 的映射
 * 每种事件类型唯一映射到一个 PetState
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
  | "agent_reply"          // Agent 直接回复 → Thinking

  // ─── 子 Agent ───
  | "subagent_start"       // 子 Agent 启动 → Working
  | "subagent_end"         // 子 Agent 结束 → Thinking

  // ─── 系统事件 ───
  | "heartbeat"            // 心跳事件 → Idle
  | "adapter_connected"    // 适配器连接 → Idle
  | "adapter_disconnected"; // 适配器断开 → Idle

// ─── 宠物状态 ─────────────────────────────────────────────────────────────

/**
 * 宠物状态机
 * 每个状态对应一套 SVG 动画
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

// ─── 统一 Agent 事件 ──────────────────────────────────────────────────────

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

  /** 协议版本号 */
  version: "1.0";

  /** 来源 Agent 标识 */
  source: AgentSource;

  /** 事件类别（高层分类） */
  category: EventCategory;

  /** 具体事件类型（细分类型） */
  type: EventType;

  /** 对应的宠物状态（状态机目标状态） */
  petState: PetState;

  /** 会话 ID（可选） */
  sessionId?: string;

  /** 子会话 ID（用于子 Agent 事件） */
  subSessionId?: string;

  /** 工具名称（仅 tool 事件有效） */
  toolName?: string;

  /** 工具参数预览（截断至 200 字符） */
  toolArgsPreview?: string;

  /** 工具执行结果预览（截断至 500 字符） */
  toolResultPreview?: string;

  /** 用户提示词/任务预览（截断至 300 字符） */
  taskPreview?: string;

  /** 当前迭代步数 */
  stepNumber?: number;

  /** 是否正在等待用户审批 */
  awaitingApproval?: boolean;

  /** 工具是否执行成功 */
  toolSuccess?: boolean;

  /** 错误码 */
  errorCode?: string;

  /** 错误消息 */
  errorMessage?: string;

  /** Agent 回复文本预览 */
  agentReplyPreview?: string;

  /** 原始数据（保留各 Agent 的完整原始事件） */
  raw: Record<string, unknown>;

  /** 扩展元数据（预留字段） */
  metadata?: Record<string, unknown>;
}
