/**
 * SkinSelector — 皮肤选择器设置组件
 *
 * 展示所有可用皮肤列表，用户可点击切换皮肤。
 * 切换后通过 `update_settings` IPC 更新 `skinId` 配置。
 *
 * # 设计
 *
 * - 从 Rust IPC `list_skins` 获取皮肤列表
 * - 每个皮肤卡片显示名称 + 预览（如果皮肤有 idle 帧）
 * - 当前皮肤有高亮标记
 * - 自定义皮肤有徽章标识
 *
 * # 使用示例
 *
 * ```tsx
 * <SkinSelector
 *   currentSkinId={skinId}
 *   onSkinChange={(newSkinId) => setSkinId(newSkinId)}
 * />
 * ```
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SkinInfo } from "@/types/skin";

// ─── Props ─────────────────────────────────────────────────────────────────

interface SkinSelectorProps {
  /** 当前激活的皮肤 ID */
  currentSkinId: string;
  /** 皮肤切换回调 */
  onSkinChange: (skinId: string) => void;
}

// ─── 组件 ──────────────────────────────────────────────────────────────────

export function SkinSelector({
  currentSkinId,
  onSkinChange,
}: SkinSelectorProps) {
  const [skins, setSkins] = useState<SkinInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadSkins = async () => {
      try {
        const result = await invoke<SkinInfo[]>("list_skins");
        setSkins(result);
        setError(null);
      } catch (e) {
        const errMsg = e instanceof Error ? e.message : String(e);
        setError(`加载皮肤列表失败: ${errMsg}`);
      } finally {
        setLoading(false);
      }
    };
    loadSkins();
  }, []);

  const handleSkinSelect = async (skinId: string) => {
    if (skinId === currentSkinId) return;
    try {
      await invoke("update_settings", {
        updates: { pet: { skinId } },
      });
      onSkinChange(skinId);
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e);
      alert(`切换皮肤失败: ${errMsg}`);
    }
  };

  // ─── 加载状态 ────────────────────────────────────────────────────────

  if (loading) {
    return (
      <div className="skin-selector">
        <h3>皮肤选择</h3>
        <div className="skin-loading">
          <div className="skin-loading-spinner" />
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  // ─── 错误状态 ────────────────────────────────────────────────────────

  if (error) {
    return (
      <div className="skin-selector">
        <h3>皮肤选择</h3>
        <p className="skin-error">{error}</p>
      </div>
    );
  }

  // ─── 皮肤列表 ────────────────────────────────────────────────────────

  return (
    <div className="skin-selector">
      <h3>皮肤选择</h3>
      <div className="skin-grid">
        {skins.map((skin) => (
          <button
            key={skin.id}
            className={`skin-option${skin.id === currentSkinId ? " active" : ""}`}
            onClick={() => handleSkinSelect(skin.id)}
            title={skin.description || skin.name}
          >
            {/* 皮肤预览区域 */}
            <div className="skin-preview">
              <span className="skin-preview-icon">
                {skin.custom ? "🎨" : "🐾"}
              </span>
            </div>
            <span className="skin-name">{skin.name}</span>
            {/* 自定义皮肤徽章 */}
            {skin.custom && (
              <span className="skin-badge skin-badge-custom">自定义</span>
            )}
            {/* 当前皮肤标识 */}
            {skin.id === currentSkinId && (
              <span className="skin-badge skin-badge-active">当前</span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
