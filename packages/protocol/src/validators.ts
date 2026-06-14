/**
 * Unified Agent Event Protocol — Validators
 *
 * Runtime validation helpers using Zod schemas
 */

import type { UnifiedAgentEvent } from "./events.js";
import { UnifiedAgentEventSchema } from "./schemas.js";

// ─── Result Type ────────────────────────────────────────────────────────────

/**
 * Result 类型：用于安全返回验证结果
 * 避免在验证失败时抛出异常
 */
export type Result<T, E extends Error = Error> =
  | { ok: true; value: T }
  | { ok: false; error: E };

/** 创建一个成功结果 */
export function ok<T>(value: T): Result<T> {
  return { ok: true, value };
}

/** 创建一个失败结果 */
export function err<E extends Error = Error>(error: E): Result<never, E> {
  return { ok: false, error };
}

// ─── Parse Error Helper ─────────────────────────────────────────────────────

/**
 * 将 Zod 解析错误转换为普通 Error
 */
function toError(e: unknown): Error {
  if (e instanceof Error) {
    return e;
  }
  if (typeof e === "string") {
    return new Error(e);
  }
  return new Error(`Validation failed: ${String(e)}`);
}

// ─── Validation Functions ───────────────────────────────────────────────────

/**
 * 验证并解析事件数据
 * 验证失败时抛出 ZodError
 *
 * @param data - 待验证的未知数据
 * @returns 验证通过的 UnifiedAgentEvent
 * @throws {z.ZodError} 验证失败时抛出
 */
export function validateEvent(data: unknown): UnifiedAgentEvent {
  return UnifiedAgentEventSchema.parse(data);
}

/**
 * 尝试验证事件数据，返回 Result 类型
 * 验证失败时不抛出异常，而是返回错误结果
 *
 * @param data - 待验证的未知数据
 * @returns Result<UnifiedAgentEvent, Error>
 */
export function tryValidateEvent(
  data: unknown
): Result<UnifiedAgentEvent, Error> {
  const result = UnifiedAgentEventSchema.safeParse(data);
  if (result.success) {
    return ok(result.data as UnifiedAgentEvent);
  }
  return err(toError(result.error));
}
