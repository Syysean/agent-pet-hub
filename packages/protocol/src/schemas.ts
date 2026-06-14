/**
 * Unified Agent Event Protocol — Zod Schemas
 *
 * All schemas derived from phase3-protocol.md (v1.0)
 * Used for runtime validation and type inference
 */

import { z } from "zod";

import type {
  AgentSource,
  EventCategory,
  EventType,
  PetState,
  ErrorCode,
  UnifiedAgentEvent,
  WSMessage,
  TTSEvent,
} from "./events.js";

// ─── Primitive Schemas ──────────────────────────────────────────────────────

/** Agent 来源标识 schema */
const AgentSourceSchemaDef = z.enum([
  "pi",
  "hermes",
  "openclaw",
]) as z.ZodType<AgentSource>;

/** 事件类别 schema */
const EventCategorySchemaDef = z.enum([
  "session",
  "thinking",
  "tool",
  "message",
  "permission",
  "error",
  "system",
  "user",
  "subagent",
]) as z.ZodType<EventCategory>;

/** 事件类型 schema */
const EventTypeSchemaDef = z.enum([
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
  "adapter_disconnected",
]) as z.ZodType<EventType>;

/** 宠物状态 schema */
const PetStateSchemaDef = z.enum([
  "idle",
  "thinking",
  "working",
  "waiting",
  "success",
  "error",
  "speaking",
  "connecting",
]) as z.ZodType<PetState>;

/** 错误码 schema */
const ErrorCodeSchemaDef = z.enum([
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
  "unknown",
]) as z.ZodType<ErrorCode>;

// Re-export with original names for backward compatibility
export const AgentSourceSchema = AgentSourceSchemaDef;
export const EventCategorySchema = EventCategorySchemaDef;
export const EventTypeSchema = EventTypeSchemaDef;
export const PetStateSchema = PetStateSchemaDef;
export const ErrorCodeSchema = ErrorCodeSchemaDef;

// ─── ULID Pattern ───────────────────────────────────────────────────────────

const ULID_PATTERN = /^[0-9A-HJKMNP-TV-Z]{26}$/;

// ─── Unified Agent Event Schema ─────────────────────────────────────────────

/**
 * 统一 Agent 事件 schema
 * 包含 allOf 条件验证：
 * - tool 类别必须包含 toolName
 * - error 类别必须包含 errorCode 和 errorMessage
 * - tool_call_start 的 petState 必须为 "working"
 */
export const UnifiedAgentEventSchema: z.ZodType<UnifiedAgentEvent> =
  z.object({
    id: z.string().regex(ULID_PATTERN, "Invalid ULID format"),
    timestamp: z.string().datetime({ message: "Invalid ISO 8601 timestamp" }),
    version: z.literal("1.0"),
    source: AgentSourceSchema,
    category: EventCategorySchema,
    type: EventTypeSchema,
    petState: PetStateSchema,
    sessionId: z.string().optional(),
    subSessionId: z.string().optional(),
    toolName: z.string().max(200).optional(),
    toolArgsPreview: z.string().max(200).optional(),
    toolResultPreview: z.string().max(500).optional(),
    taskPreview: z.string().max(300).optional(),
    stepNumber: z.number().int().min(1).optional(),
    awaitingApproval: z.boolean().optional(),
    toolSuccess: z.boolean().optional(),
    errorCode: ErrorCodeSchema.optional(),
    errorMessage: z.string().max(500).optional(),
    agentReplyPreview: z.string().max(500).optional(),
    raw: z.record(z.unknown()),
    metadata: z.record(z.unknown()).optional(),
  })
  .refine(
    (data) => {
      if (data.category === "tool" && !data.toolName) {
        return false;
      }
      return true;
    },
    {
      message: "tool category requires toolName",
      path: ["toolName"],
    }
  )
  .refine(
    (data) => {
      if (data.category === "error" && (!data.errorCode || !data.errorMessage)) {
        return false;
      }
      return true;
    },
    {
      message: "error category requires errorCode and errorMessage",
      path: ["errorCode"],
    }
  )
  .refine(
    (data) => {
      if (data.type === "tool_call_start" && data.petState !== "working") {
        return false;
      }
      return true;
    },
    {
      message: "tool_call_start requires petState to be 'working'",
      path: ["petState"],
    }
  );

// ─── WebSocket Message Schema ───────────────────────────────────────────────

/** WSErrorCode schema */
const WSErrorCodeSchema = z.enum([
  "invalid_message",
  "unauthorized",
  "subscription_not_found",
  "rate_limited",
  "server_error",
  "protocol_error",
]);

/** WSMessageType schema */
const WSMessageTypeSchema = z.enum([
  "event",
  "subscribe",
  "unsubscribe",
  "subscribed",
  "error",
  "ping",
  "pong",
  "auth",
  "auth_ack",
  "agent_info",
  "agent_info_ack",
  "command",
  "command_result",
]) as z.ZodType<WSMessage["type"]>;

/** WSEventPayload schema */
const WSEventPayloadSchema = z.object({
  event: UnifiedAgentEventSchema,
  subscriptionId: z.string().optional(),
});

/** WSSubscribePayload schema */
const WSSubscribePayloadSchema = z.object({
  eventTypes: z.union([z.array(EventTypeSchema), z.literal("*")]),
  categories: z.array(EventCategorySchema).optional(),
  sources: z.array(AgentSourceSchema).optional(),
});

/** WSSubscribedPayload schema */
const WSSubscribedPayloadSchema = z.object({
  subscriptionId: z.string(),
  eventTypes: z.array(EventTypeSchema),
  categories: z.array(EventCategorySchema),
});

/** WSErrorPayload schema */
const WSErrorPayloadSchema = z.object({
  code: WSErrorCodeSchema,
  message: z.string(),
  details: z.record(z.unknown()).optional(),
});

/** WSAuthPayload schema */
const WSAuthPayloadSchema = z.object({
  token: z.string(),
});

/** WSAuthAckPayload schema */
const WSAuthAckPayloadSchema = z.object({
  authorized: z.boolean(),
  userId: z.string().optional(),
  permissions: z.array(z.string()).optional(),
});

/** WSCommandPayload schema */
const WSCommandPayloadSchema = z.object({
  agent: AgentSourceSchema,
  sessionId: z.string(),
  command: z.string(),
});

/** WSCommandResultPayload schema */
const WSCommandResultPayloadSchema = z.object({
  commandId: z.string(),
  agent: AgentSourceSchema,
  sessionId: z.string(),
  status: z.enum(["success", "error"]),
  result: z.string().optional(),
  error: z.string().optional(),
});

/** WSMessagePayload discriminated union */
const WSMessagePayloadSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("event"), payload: WSEventPayloadSchema }),
  z.object({ type: z.literal("subscribe"), payload: WSSubscribePayloadSchema }),
  z.object({
    type: z.literal("unsubscribe"),
    payload: z.object({ subscriptionId: z.string() }),
  }),
  z.object({ type: z.literal("subscribed"), payload: WSSubscribedPayloadSchema }),
  z.object({ type: z.literal("error"), payload: WSErrorPayloadSchema }),
  z.object({ type: z.literal("ping"), payload: z.object({}).strict() }),
  z.object({ type: z.literal("pong"), payload: z.object({}).strict() }),
  z.object({ type: z.literal("auth"), payload: WSAuthPayloadSchema }),
  z.object({
    type: z.literal("auth_ack"),
    payload: WSAuthAckPayloadSchema,
  }),
  z.object({ type: z.literal("agent_info"), payload: z.object({}).strict() }),
  z.object({
    type: z.literal("agent_info_ack"),
    payload: z.object({
      agents: z.array(
        z.object({
          source: AgentSourceSchema,
          displayName: z.string(),
          version: z.string().optional(),
          online: z.boolean(),
          activeSessionId: z.string().optional(),
        })
      ),
    }),
  }),
  z.object({ type: z.literal("command"), payload: WSCommandPayloadSchema }),
  z.object({
    type: z.literal("command_result"),
    payload: WSCommandResultPayloadSchema,
  }),
]);

/**
 * WebSocket 消息 schema
 * 使用 discriminated union 按 type 字段区分不同 payload 格式
 */
export const WSMessageSchema: z.ZodType<WSMessage> = z
  .object({
    type: WSMessageTypeSchema,
    id: z.string().optional(),
    timestamp: z.string().datetime({ message: "Invalid ISO 8601 timestamp" }),
    payload: z.any(),
  })
  .refine(
    (data) => {
      try {
        WSMessagePayloadSchema.parse(data.payload);
        return true;
      } catch {
        return false;
      }
    },
    {
      message: "Invalid payload for message type",
      path: ["payload"],
    }
  ) as unknown as z.ZodType<WSMessage>;

// ─── TTS Event Schema ───────────────────────────────────────────────────────

/** TTS 播报事件 schema */
export const TTSEventSchema: z.ZodType<TTSEvent> = z.object({
  id: z.string().regex(ULID_PATTERN, "Invalid ULID format"),
  timestamp: z.string().datetime({ message: "Invalid ISO 8601 timestamp" }),
  triggerEvent: EventTypeSchema,
  petState: PetStateSchema,
  shouldSpeak: z.boolean(),
  text: z.string(),
  priority: z.union([
    z.literal(0 as const),
    z.literal(1 as const),
    z.literal(2 as const),
  ]) as z.ZodType<0 | 1 | 2>,
  interruptible: z.boolean(),
});

// ─── Export All ─────────────────────────────────────────────────────────────

// 从 Zod schema 推断的类型（与 events.ts 中的类型一致）
export type {
  AgentSource,
  EventCategory,
  EventType,
  PetState,
  ErrorCode,
  UnifiedAgentEvent,
  WSMessage,
  TTSEvent,
};
