/**
 * PetSVG（旧版）— 宠物 SVG 渲染组件
 *
 * @deprecated 已替换为 PetPNG 组件。保留用于向后兼容。
 *
 * 根据宠物状态动态切换 CSS class，
 * 触发对应的 SVG 动画（呼吸、思考、工作中等）
 *
 * 动画定义参考 src-tauri/resources/skins/default/css/ 目录
 */

import { useMemo, useEffect } from "react";
import { PetState } from "@agent-pet-hub/protocol";

// ─── Props ─────────────────────────────────────────────────────────────────

interface PetSVGProps {
  /** 当前宠物状态 */
  petState: PetState;
  /** 上一帧状态（用于过渡动画） */
  previousState?: PetState;
}

// ─── SVG 类名映射 ─────────────────────────────────────────────────────────

/**
 * PetState → SVG class 名映射
 * 与 CSS 动画类名对应（如 .pet-idle, .pet-working）
 */
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

export function PetSVG({ petState, previousState }: PetSVGProps) {
  // 调试：追踪 petState prop 变化
  useEffect(() => {
    console.log("[PetSVG] petState changed:", petState);
    console.log("[PetSVG] previousState:", previousState);
    const currentClass = STATE_CLASS_MAP[petState] || STATE_CLASS_MAP.idle;
    console.log("[PetSVG] CSS class applied:", currentClass);
  }, [petState, previousState]);

  // 根据当前状态选择 SVG class
  const svgClass = useMemo(() => {
    return STATE_CLASS_MAP[petState] || STATE_CLASS_MAP.idle;
  }, [petState]);

  // 如果状态正在切换，添加过渡动画 class
  const hasTransition = previousState && previousState !== petState;
  const className = `pet-puppet ${svgClass}${hasTransition ? " pet-transitioning" : ""}`;

  // SVG viewBox 与资源文件一致：120x120
  return (
    <div className="pet-svg-wrapper" data-tauri-drag-region>
      <svg data-tauri-drag-region
        className={className}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 120 120"
        width="120"
        height="120"
        style={{
          // 居中显示在窗口内
          position: "absolute",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
        }}
      >
        <defs>
          {/* 身体渐变 - 粉色主题 */}
          <radialGradient id="bodyGrad" cx="50%" cy="40%" r="50%">
            <stop offset="0%" stopColor="#FFB6C1" />
            <stop offset="100%" stopColor="#FF69B4" />
          </radialGradient>

          {/* 阴影滤镜 */}
          <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
            <feDropShadow
              dx="0"
              dy="2"
              stdDeviation="3"
              floodColor="#00000020"
            />
          </filter>

          {/* 腮红渐变 */}
          <radialGradient id="blush" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="#FF69B4" stopOpacity="0.4" />
            <stop offset="100%" stopColor="#FF69B4" stopOpacity="0" />
          </radialGradient>
        </defs>

        {/* 阴影 */}
        <ellipse cx="60" cy="105" rx="30" ry="8" fill="#00000015" />

        {/* 宠物主体 */}
        <g filter="url(#shadow)">
          {/* 身体主体 */}
          <circle
            cx="60"
            cy="60"
            r="42"
            fill="url(#bodyGrad)"
            stroke="#E7548A"
            strokeWidth="1.5"
          />

          {/* 腮红 - 空闲时保持淡色 */}
          <circle
            className="pet-blush"
            cx="35"
            cy="68"
            r="10"
            fill="url(#blush)"
          />
          <circle
            className="pet-blush"
            cx="85"
            cy="68"
            r="10"
            fill="url(#blush)"
          />

          {/* 左眼 */}
          <g transform="translate(42, 50)">
            <ellipse cx="0" cy="0" rx="8" ry="9" fill="white" />
            <ellipse cx="0" cy="0" rx="5" ry="6" fill="#2C1810" />
            <circle cx="2" cy="-2" r="2.5" fill="white" opacity="0.9" />
            <circle cx="-1" cy="1" r="1" fill="white" opacity="0.5" />
            {/* 眨眼动画 */}
            <ellipse
              className="blink-animation"
              cx="0"
              cy="0"
              rx="8"
              ry="9"
              fill="#FF69B4"
              opacity="0"
            />
          </g>

          {/* 右眼 */}
          <g transform="translate(78, 50)">
            <ellipse cx="0" cy="0" rx="8" ry="9" fill="white" />
            <ellipse cx="0" cy="0" rx="5" ry="6" fill="#2C1810" />
            <circle cx="2" cy="-2" r="2.5" fill="white" opacity="0.9" />
            <circle cx="-1" cy="1" r="1" fill="white" opacity="0.5" />
            {/* 眨眼动画 */}
            <ellipse
              className="blink-animation"
              cx="0"
              cy="0"
              rx="8"
              ry="9"
              fill="#FF69B4"
              opacity="0"
            />
          </g>

          {/* 嘴巴 - 微笑 */}
          <path
            className="pet-mouth"
            d="M52 72 Q60 80 68 72"
            fill="none"
            stroke="#2C1810"
            strokeWidth="2.5"
            strokeLinecap="round"
          />

          {/* 触手装饰 */}
          <path
            d="M35 25 Q40 12 50 20"
            fill="none"
            stroke="#FF69B4"
            strokeWidth="3"
            strokeLinecap="round"
          />
          <path
            d="M85 25 Q80 12 70 20"
            fill="none"
            stroke="#FF69B4"
            strokeWidth="3"
            strokeLinecap="round"
          />
        </g>

        {/* ============================================
            状态指示器 - 通过 CSS class 控制显示/隐藏
            ============================================ */}

        {/* Working 状态光环 */}
        <g
          className="indicator"
          id="indicator-working"
          opacity="0"
        >
          <circle
            cx="60"
            cy="60"
            r="44"
            fill="none"
            stroke="#4CAF50"
            strokeWidth="2"
            strokeDasharray="8 4"
          />
        </g>

        {/* Error 状态 X 标记 */}
        <g
          className="indicator"
          id="indicator-error"
          opacity="0"
        >
          <line
            x1="52" y1="64" x2="68" y2="80"
            stroke="#F44336"
            strokeWidth="2.5"
            strokeLinecap="round"
          />
          <line
            x1="68" y1="64" x2="52" y2="80"
            stroke="#F44336"
            strokeWidth="2.5"
            strokeLinecap="round"
          />
        </g>

        {/* Success 状态星星 */}
        <g
          className="indicator"
          id="indicator-success"
          opacity="0"
        >
          <text
            x="60" y="12"
            textAnchor="middle"
            fontSize="12"
            fill="#FFD700"
          >
            ★
          </text>
        </g>
      </svg>
    </div>
  );
}
