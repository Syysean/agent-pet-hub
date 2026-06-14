/**
 * Pet Event Logger - Pi Agent Extension
 *
 * 监听 Pi Agent 事件并将其写入 JSONL 文件，
 * 供 Agent-Pet-Hub 桌面宠物应用消费。
 *
 * 安装方式:
 *   pi install ./skills/pi/pet-event-logger.ts
 *
 * 输出路径: ~/.pi/logs/pet-events.jsonl
 */

import { Extension } from "pi";

// ─── 配置 ──────────────────────────────────────────────────────────────

const LOG_DIR = "~/.pi/logs";
const LOG_FILE = "pet-events.jsonl";

// ─── 事件转换 ─────────────────────────────────────────────────────────

/**
 * 将 Pi 原生事件转换为统一事件格式
 */
function toUnifiedEvent(
  type: string,
  raw: Record<string, unknown>,
  sessionId?: string
): Record<string, unknown> {
  const now = new Date().toISOString();
  const id = crypto.randomUUID();

  // 映射表
  const mapping: Record<string, { category: string; eventType: string; petState: string }> = {
    session_start: { category: "session", eventType: "session_start", petState: "thinking" },
    user_prompt: { category: "user", eventType: "user_prompt", petState: "thinking" },
    text_delta: { category: "thinking", eventType: "thinking_tick", petState: "thinking" },
    tool_call: { category: "tool", eventType: "tool_call_start", petState: "working" },
    tool_result: { category: "tool", eventType: "tool_call_end", petState: "thinking" },
    tool_error: { category: "error", eventType: "tool_call_error", petState: "error" },
    turn_end: { category: "session", eventType: "session_end", petState: "idle" },
    compaction: { category: "session", eventType: "session_compaction", petState: "thinking" },
    compaction_end: { category: "session", eventType: "session_end", petState: "idle" },
  };

  const match = mapping[type];
  if (!match) {
    // 未知事件类型，默认视为 thinking_tick
    return {
      id,
      timestamp: now,
      version: "1.0",
      source: "pi",
      category: "thinking",
      type: "thinking_tick",
      petState: "thinking",
      sessionId,
      raw,
    };
  }

  return {
    id,
    timestamp: now,
    version: "1.0",
    source: "pi",
    category: match.category,
    type: match.eventType,
    petState: match.petState,
    sessionId,
    raw,
  };
}

// ─── 扩展定义 ─────────────────────────────────────────────────────────

export default class PetEventLogger implements Extension {
  readonly name = "pet-event-logger";
  readonly version = "0.1.0";
  readonly description = "Logs Pi Agent events to JSONL for Agent-Pet-Hub desktop pet";

  private logPath: string;

  constructor() {
    // 解析 ~ 路径
    const home = process.env.HOME || process.env.USERPROFILE || ".";
    this.logPath = `${home}/${LOG_DIR.replace("~", "")}/${LOG_FILE}`;
  }

  async onSessionStart(ctx: any): Promise<void> {
    const event = toUnifiedEvent("session_start", { prompt: ctx.prompt });
    await this.appendLog(event);
  }

  async onUserPrompt(ctx: any): Promise<void> {
    const event = toUnifiedEvent("user_prompt", { text: ctx.text });
    await this.appendLog(event);
  }

  async onTextDelta(ctx: any): Promise<void> {
    const event = toUnifiedEvent("text_delta", { delta: ctx.delta });
    await this.appendLog(event);
  }

  async onToolCall(ctx: any): Promise<void> {
    const event = toUnifiedEvent("tool_call", {
      tool: ctx.tool,
      args: ctx.args,
    });
    await this.appendLog(event);
  }

  async onToolResult(ctx: any): Promise<void> {
    const event = toUnifiedEvent("tool_result", {
      tool: ctx.tool,
      result: ctx.result,
    });
    await this.appendLog(event);
  }

  async onToolError(ctx: any): Promise<void> {
    const event = toUnifiedEvent("tool_error", {
      tool: ctx.tool,
      error: ctx.error,
    });
    await this.appendLog(event);
  }

  async onTurnEnd(ctx: any): Promise<void> {
    const event = toUnifiedEvent("turn_end", {});
    await this.appendLog(event);
  }

  async onCompaction(ctx: any): Promise<void> {
    const event = toUnifiedEvent("compaction", {});
    await this.appendLog(event);
  }

  // ─── 私有方法 ──────────────────────────────────────────────────────

  private async appendLog(event: Record<string, unknown>): Promise<void> {
    try {
      const fs = await import("fs/promises");
      const path = await import("path");

      // 确保日志目录存在
      const logDir = path.dirname(this.logPath);
      await fs.mkdir(logDir, { recursive: true });

      // 追加到 JSONL 文件
      const line = JSON.stringify(event) + "\n";
      await fs.appendFile(this.logPath, line);
    } catch (error) {
      // 降级为 console.log
      console.error("[PetEventLogger] Failed to write event:", error);
    }
  }
}
