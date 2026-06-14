/**
 * 宠物类型定义
 *
 * 包含位置、皮肤、状态快照等前端相关类型
 */

import { PetState } from "./events";

// ─── 位置 ──────────────────────────────────────────────────────────────────

/** 屏幕坐标位置（以 Tauri Window 左上角为原点） */
export interface PetPosition {
  x: number;
  y: number;
}

// ─── 皮肤 ─────────────────────────────────────────────────────────────────

/** 宠物皮肤配置 */
export interface PetSkin {
  /** 皮肤唯一标识 */
  id: string;
  /** 皮肤显示名称 */
  name: string;
  /** 皮肤描述 */
  description: string;
  /** 皮肤资源路径（相对 resources/skins/ 目录） */
  image_path: string;
  /** 是否为用户自定义皮肤 */
  custom?: boolean;
}

// ─── 状态快照 ──────────────────────────────────────────────────────────────

/**
 * 宠物完整状态快照
 * 用于前端状态管理和持久化
 */
export interface PetStateSnapshot {
  /** 当前宠物状态 */
  petState: PetState;
  /** 上一个宠物状态（用于过渡动画） */
  previousState?: PetState;
  /** 当前位置 */
  position: PetPosition;
  /** 当前激活的 Agent 标识（可选） */
  activeAgent?: string | null;
  /** 当前会话 ID（可选） */
  sessionId?: string;
  /** 连续错误计数 */
  errorCount: number;
}

// ─── 状态标签映射 ─────────────────────────────────────────────────────────

/**
 * 宠物状态 → 中文显示标签
 * 用于 UI 状态指示器展示
 */
export const STATE_LABELS: Record<PetState, string> = {
  idle: "空闲",
  thinking: "思考中",
  working: "工作中",
  waiting: "等待中",
  success: "成功",
  error: "错误",
  speaking: "播报中",
  connecting: "连接中",
};
