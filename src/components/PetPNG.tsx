/**
 * PetPNG — PNG 皮肤渲染组件
 *
 * 根据宠物状态动态切换皮肤帧 PNG 图片，
 * 通过 CSS class 叠加动画效果（呼吸、思考、工作中等）。
 *
 * 状态指示器通过 CSS overlay 实现（Working/Error/Success），
 * 而非烘焙进 PNG 中。
 *
 * # 与 PetSVG 的差异
 *
 * - 使用 `<img>` 标签渲染 PNG 帧，而非内联 SVG
 * - 动画通过 CSS `@keyframes` 应用于 `<img>`
 * - 指示器使用 `<div>` overlay，通过 CSS 伪元素绘制
 * - 保留拖拽功能（`data-tauri-drag-region`）
 *
 * @see PetSVG 旧版 SVG 渲染组件（标记为废弃）
 */

import { useEffect, useState, useMemo } from "react";
import { PetState } from "@agent-pet-hub/protocol";
import { useSkinLoader } from "@/hooks/useSkinLoader";
import { PetSVG } from "./PetSVG";


// ─── Props ─────────────────────────────────────────────────────────────────

interface PetPNGProps {
  /** 当前宠物状态 */
  petState: PetState;
  /** 上一帧状态（用于过渡动画） */
  previousState?: PetState;
  /** 皮肤 ID（来自 config，默认 "shark"） */
  skinId?: string;
}

// ─── CSS Class 映射 ───────────────────────────────────────────────────────

/** PetState → CSS class 名映射 */
const STATE_CLASS_MAP: Record<PetState, string> = {
  idle: "pet-idle",
  thinking: "pet-thinking",
  working: "pet-working",
  waiting: "pet-waiting",
  success: "pet-success",
  error: "pet-error",
  speaking: "pet-speaking",
  connecting: "pet-connecting",
};

// ─── 组件 ──────────────────────────────────────────────────────────────────

export function PetPNG({
  petState,
  previousState,
  skinId = "shark",
}: PetPNGProps) {
  const { frames, status, error } = useSkinLoader(skinId);
  const [currentFrame, setCurrentFrame] = useState<string>("");
  const [isTransitioning, setIsTransitioning] = useState(false);

  // 状态变化时触发过渡动画
  useEffect(() => {
    if (previousState && previousState !== petState) {
      setIsTransitioning(true);
      const timer = setTimeout(() => setIsTransitioning(false), 200);
      return () => clearTimeout(timer);
    }
  }, [petState, previousState]);

  // 根据当前状态切换帧
  useEffect(() => {
    if (status === "loaded" && frames && frames[petState]) {
      setCurrentFrame(frames[petState]);
    } else if (status === "loaded" && frames && frames.idle) {
      // fallback 到 idle 帧
      setCurrentFrame(frames.idle);
    }
  }, [petState, status, frames]);

  // 构建 CSS class 字符串
  const cssClass = useMemo(() => {
    const base = `pet-png ${STATE_CLASS_MAP[petState] || STATE_CLASS_MAP.idle}`;
    return isTransitioning ? `${base} pet-transitioning` : base;
  }, [petState, isTransitioning]);

  // ─── 错误状态 — fallback 到 SVG 显示，确保桌宠不消失 ───────────

  if (status === "error") {
    if (import.meta.env.DEV) {
      console.warn("[PetPNG] Skin load error, falling back to SVG:", error);
    }
    return (
      <div className="pet-png-wrapper pet-svg-wrapper" data-tauri-drag-region>
        <div className="pet-error-fallback" style={{
          position: "absolute",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          fontSize: "10px",
          color: "#999",
          textAlign: "center",
        }}>
          <div>{error}</div>
        </div>
        <PetSVG petState={petState} previousState={previousState} />
      </div>
    );
  }

  // ─── 加载中状态 ──────────────────────────────────────────────────────

  if (status === "loading") {
    return (
      <div className="pet-png-wrapper" data-tauri-drag-region>
        <div className="pet-loading" />
      </div>
    );
  }

  // ─── 正常渲染 ────────────────────────────────────────────────────────

  return (
    <div className="pet-png-wrapper" data-tauri-drag-region>
      <img
        className={cssClass}
        src={currentFrame}
        alt={`Pet ${petState}`}
        draggable={false}
        onError={() => setCurrentFrame(frames?.idle || "")}
        style={{
          position: "absolute",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
        }}
      />

      {/* 状态指示器 — CSS overlay */}
      {petState === "working" && <div className="indicator indicator-working" />}
      {petState === "error" && <div className="indicator indicator-error" />}
      {petState === "success" && (
        <div className="indicator indicator-success">★</div>
      )}
    </div>
  );
}
