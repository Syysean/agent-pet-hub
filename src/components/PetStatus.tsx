/**
 * PetStatus — 宠物状态指示组件
 *
 * 在屏幕底部中央显示当前宠物状态文本，
 * 根据 PetState 映射到中文标签
 */

import { STATE_LABELS } from "@/types/pet";
import { PetStateSnapshot } from "@/types/pet";

// ─── Props ─────────────────────────────────────────────────────────────────

interface PetStatusProps {
  /** 宠物状态快照 */
  petState: PetStateSnapshot;
}

// ─── 组件 ──────────────────────────────────────────────────────────────────

export function PetStatus({ petState }: PetStatusProps) {
  /**
   * 获取状态标签文本
   * 如果状态不在映射表中，显示未知
   */
  const label = STATE_LABELS[petState.petState] ?? "未知";

  /**
   * 根据状态获取指示器样式
   */
  const getIndicatorClass = () => {
    switch (petState.petState) {
      case "working":
        return "working-indicator";
      case "error":
        return "error-indicator";
      default:
        return "";
    }
  };

  // 空闲或连接中时不显示状态文字（保持界面干净）
  const shouldShow = petState.petState !== "idle" && petState.petState !== "connecting";

  return (
    <div
      className={`status-indicator ${getIndicatorClass()}`}
      role="status"
      aria-live="polite"
      aria-label={`宠物状态：${label}`}
      style={{ opacity: shouldShow ? 1 : 0 }}
    >
      {label}
    </div>
  );
}
