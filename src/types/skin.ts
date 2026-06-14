/**
 * 皮肤系统类型定义
 *
 * 包含皮肤元数据（skin.json 格式）、皮肤列表项（Rust IPC 返回格式）、
 * 帧映射等类型。
 *
 * 与 Rust 端 `src/pet/skin.rs` 中的 SkinMetadata / PetSkin struct 对齐。
 */

// ─── 帧映射 ────────────────────────────────────────────────────────────────

/**
 * 皮肤帧映射
 *
 * key 为 PetState 值，value 为皮肤帧 PNG 文件名。
 * 每个状态必须提供对应的帧文件，否则该状态无法显示。
 */
export interface SkinFrames {
  idle: string;
  thinking: string;
  working: string;
  waiting: string;
  success: string;
  error: string;
  speaking: string;
  connecting: string;
}

// ─── 皮肤元数据（对应 skin.json）────────────────────────────────────────────

/**
 * 皮肤元数据 — 对应 skin.json 文件内容
 *
 * 由 skin.json 文件解析得到，或用于验证皮肤包完整性。
 * 所有字段均来自皮肤包自身的元数据声明。
 */
export interface SkinMetadata {
  /** 皮肤唯一标识，与 config.json 中的 skin_id 对应 */
  id: string;

  /** 皮肤显示名称 */
  name: string;

  /** 皮肤作者 */
  author?: string;

  /** 皮肤版本号 */
  version?: string;

  /** 皮肤描述 */
  description?: string;

  /** 帧尺寸（可选，用于校验 PNG 尺寸一致性） */
  frame_count?: {
    width: number;
    height: number;
  };

  /** 状态 → 文件名映射 */
  frames: SkinFrames;

  /** 指示器配置（可选，定义非嵌入式的叠加指示器） */
  indicators?: Record<string, unknown>;
}

// ─── 皮肤列表项（来自 Rust IPC）─────────────────────────────────────────────

/**
 * 皮肤列表项 — 由 Rust IPC 命令返回
 *
 * 用于皮肤选择器列表展示，不包含完整的帧映射信息。
 * 与 Rust 端 `PetSkin` struct 字段对齐。
 */
export interface SkinInfo {
  /** 皮肤唯一标识 */
  id: string;

  /** 皮肤显示名称 */
  name: string;

  /** 皮肤描述 */
  description: string;

  /** 皮肤资源路径（内置皮肤为 Vite hash 路径，用户皮肤为文件系统路径） */
  image_path: string;

  /** 是否为用户自定义皮肤 */
  custom: boolean;
}

// ─── 皮肤加载状态 ──────────────────────────────────────────────────────────

/** 皮肤加载状态 */
export type SkinLoadStatus = "loading" | "loaded" | "error";

/**
 * 皮肤加载结果
 *
 * 使用 discriminated union 模式：status 为 "loaded" 时 skin 非空，
 * status 为 "error" 时 error 非空。
 */
export interface SkinLoadResult {
  /** 加载状态 */
  status: SkinLoadStatus;

  /** 皮肤元数据（加载成功时存在） */
  skin: SkinMetadata | null;

  /** 错误信息（加载失败时存在） */
  error: string | null;
}

// ─── 帧 URL 构建 ────────────────────────────────────────────────────────────

/**
 * 内置皮肤帧 URL 映射
 *
 * 由 import.meta.glob 动态导入生成，key 为文件名，value 为 Vite 解析后的 URL。
 */
export interface BuiltinSkinURLs {
  [filename: string]: string;
}

/**
 * 皮肤帧 URL 解析结果
 *
 * 将 skin.json 中的帧文件名解析为可加载的 URL。
 * 用于将 SkinFrames（文件名）映射到 ParsedSkinFrames（可加载 URL）。
 */
export interface ParsedSkinFrames {
  /** 状态 → 可加载 URL 的映射 */
  [state: string]: string;
}
