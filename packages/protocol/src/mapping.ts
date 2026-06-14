/**
 * Unified Agent Event Protocol — Agent Event Mappings
 *
 * Maps native events from each Agent to UnifiedAgentEvent fields.
 * Each mapping defines: category, type, petState, and field extraction rules.
 */

import type {
  AgentSource,
  EventCategory,
  EventType,
  PetState,
} from "./events.js";

// ─── Mapping Entry ──────────────────────────────────────────────────────────

/**
 * 单个事件映射条目
 * 定义原生事件到统一事件的字段映射
 */
export interface EventMappingEntry {
  /** 原生事件名称/模式 */
  nativeEvent: string;
  /** 统一 category */
  category: EventCategory;
  /** 统一 type */
  type: EventType;
  /** 对应的宠物状态 */
  petState: PetState;
  /** 说明 */
  description?: string;
}

/**
 * 事件映射表
 * key: 原生事件名称（支持通配符 * 匹配所有）
 * value: 映射条目
 */
export type EventMapping = Record<string, EventMappingEntry>;

// ─── Pi Agent Mapping ───────────────────────────────────────────────────────

/**
 * Pi Agent 事件映射表
 *
 * 来源：phase3-protocol.md §2.1
 * 采集方式：
 *   - 方式 A（推荐）：Pi TypeScript Extension → JSONL 文件
 *   - 方式 B（备选）：RPC stdin/stdout 解析
 */
export const piEventMapping: EventMapping = {
  session_start: {
    nativeEvent: "session_start",
    category: "session",
    type: "session_start",
    petState: "thinking",
    description: "会话开始",
  },
  user_prompt: {
    nativeEvent: "user_prompt",
    category: "user",
    type: "user_prompt",
    petState: "thinking",
    description: "用户提交提示词",
  },
  text_delta: {
    nativeEvent: "text_delta",
    category: "thinking",
    type: "thinking_tick",
    petState: "thinking",
    description: "持续生成文本",
  },
  tool_call: {
    nativeEvent: "tool_call",
    category: "tool",
    type: "tool_call_start",
    petState: "working",
    description: "工具开始执行",
  },
  tool_result: {
    nativeEvent: "tool_result",
    category: "tool",
    type: "tool_call_end",
    petState: "thinking",
    description: "工具执行完成",
  },
  tool_error: {
    nativeEvent: "tool_error",
    category: "error",
    type: "tool_call_error",
    petState: "thinking",
    description: "工具执行失败",
  },
  turn_end: {
    nativeEvent: "turn_end",
    category: "session",
    type: "session_end",
    petState: "idle",
    description: "回合结束",
  },
  compaction: {
    nativeEvent: "compaction",
    category: "session",
    type: "session_compaction",
    petState: "thinking",
    description: "上下文压缩",
  },
  compaction_end: {
    nativeEvent: "compaction_end",
    category: "session",
    type: "session_end",
    petState: "idle",
    description: "压缩完成",
  },
};

// ─── Hermes Agent Mapping ───────────────────────────────────────────────────

/**
 * Hermes Agent 事件映射表（预留）
 *
 * 来源：phase3-protocol.md §2.2
 * 采集方式：Gateway Hooks → handler.py → JSONL 文件
 */
export const hermesEventMapping: EventMapping = {
  "session:start": {
    nativeEvent: "session:start",
    category: "session",
    type: "session_start",
    petState: "thinking",
    description: "会话开始",
  },
  "agent:start": {
    nativeEvent: "agent:start",
    category: "thinking",
    type: "thinking_start",
    petState: "thinking",
    description: "Agent 开始处理",
  },
  "agent:step": {
    nativeEvent: "agent:step",
    category: "tool",
    type: "tool_call_start",
    petState: "working",
    description: "Agent 迭代（含工具调用）",
  },
  "agent:end": {
    nativeEvent: "agent:end",
    category: "session",
    type: "session_end",
    petState: "idle",
    description: "Agent 处理结束",
  },
  "command:new": {
    nativeEvent: "command:new",
    category: "session",
    type: "session_start",
    petState: "thinking",
    description: "新会话",
  },
  "command:reset": {
    nativeEvent: "command:reset",
    category: "session",
    type: "session_start",
    petState: "thinking",
    description: "重置会话",
  },
  "command:stop": {
    nativeEvent: "command:stop",
    category: "user",
    type: "user_cancel",
    petState: "idle",
    description: "用户停止",
  },
  "command:*": {
    nativeEvent: "command:*",
    category: "user",
    type: "user_prompt",
    petState: "thinking",
    description: "命令执行",
  },
};

// ─── OpenClaw Agent Mapping ─────────────────────────────────────────────────

/**
 * OpenClaw Agent 事件映射表（预留）
 *
 * 来源：phase3-protocol.md §2.3
 * 采集方式：Internal Hooks → App SDK → HTTP RPC
 */
export const openclawEventMapping: EventMapping = {
  "agent:message": {
    nativeEvent: "agent:message",
    category: "message",
    type: "agent_message",
    petState: "thinking",
    description: "Agent 发送消息",
  },
  "tool_result_persist": {
    nativeEvent: "tool_result_persist",
    category: "tool",
    type: "tool_call_end",
    petState: "thinking",
    description: "工具结果持久化",
  },
  "command:new": {
    nativeEvent: "command:new",
    category: "session",
    type: "session_start",
    petState: "thinking",
    description: "新会话",
  },
  "command:reset": {
    nativeEvent: "command:reset",
    category: "session",
    type: "session_start",
    petState: "thinking",
    description: "重置会话",
  },
  "command:stop": {
    nativeEvent: "command:stop",
    category: "user",
    type: "user_cancel",
    petState: "idle",
    description: "用户停止",
  },
  "gateway:startup": {
    nativeEvent: "gateway:startup",
    category: "system",
    type: "adapter_connected",
    petState: "connecting",
    description: "Gateway 启动",
  },
  "command:*": {
    nativeEvent: "command:*",
    category: "user",
    type: "user_prompt",
    petState: "thinking",
    description: "命令执行",
  },
};

// ─── All Mappings Registry ──────────────────────────────────────────────────

/**
 * 所有 Agent 事件映射表注册表
 *
 * 按 AgentSource 键索引，方便运行时查找
 */
export const allEventMappings: Record<AgentSource, EventMapping> = {
  pi: piEventMapping,
  hermes: hermesEventMapping,
  openclaw: openclawEventMapping,
};

/**
 * 根据 Agent 来源获取对应的事件映射表
 *
 * @param source - Agent 来源标识
 * @returns 事件映射表
 */
export function getEventMapping(
  source: AgentSource
): EventMapping {
  const mapping = allEventMappings[source];
  if (!mapping) {
    throw new Error(`No event mapping registered for source: ${source}`);
  }
  return mapping;
}

/**
 * 获取原生事件对应的统一映射条目
 * 支持通配符匹配（如 "command:*" 匹配 "command:new"）
 *
 * @param source - Agent 来源
 * @param nativeEvent - 原生事件名称
 * @returns 映射条目，未找到则返回 undefined
 */
export function findMappingEntry(
  source: AgentSource,
  nativeEvent: string
): EventMappingEntry | undefined {
  const mapping = getEventMapping(source);

  // 精确匹配优先
  if (mapping[nativeEvent]) {
    return mapping[nativeEvent];
  }

  // 通配符匹配（将 "command:*" 映射到 "command:new"）
  const wildcardKey = `${nativeEvent.split(":")[0]}:*`;
  if (mapping[wildcardKey]) {
    return mapping[wildcardKey];
  }

  return undefined;
}

/**
 * 获取所有已注册的映射条目（用于测试和文档生成）
 */
export function getAllMappingEntries(): EventMappingEntry[] {
  const entries: EventMappingEntry[] = [];
  for (const source of Object.keys(allEventMappings) as AgentSource[]) {
    const mapping = allEventMappings[source];
    for (const key of Object.keys(mapping)) {
      const entry = mapping[key];
      if (entry) {
        entries.push(entry);
      }
    }
  }
  return entries;
}
