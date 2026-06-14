/**
 * useSkinLoader — 皮肤加载 Hook
 *
 * 负责加载皮肤元数据（skin.json）并构建帧 URL 映射，
 * 支持内置皮肤（Vite import.meta.glob）和用户自定义皮肤（文件系统路径）。
 *
 * # 设计
 *
 * - 内置皮肤：通过 `import.meta.glob` 预加载所有 PNG 帧，返回 Vite hash URL
 * - 用户皮肤：通过 `image_path` 文件系统路径 + `skin.json` 解析构建 URL
 * - 状态切换时自动更新帧 URL
 *
 * # 使用示例
 *
 * ```tsx
 * const { skin, frames, loading, error } = useSkinLoader(skinId);
 * if (loading) return <Loading />;
 * if (error) return <Error />;
 * const frameUrl = frames?.[petState] ?? frames?.idle;
 * ```
 */

import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PetState } from "@agent-pet-hub/protocol";
import type {
  SkinMetadata,
  SkinInfo,
  SkinLoadStatus,
  ParsedSkinFrames,
} from "@/types/skin";

// ─── 内置皮肤帧的 Vite glob 模式─────────────────────────────────────────────

/**
 * Vite import.meta.glob 模式
 *
 * 匹配所有内置皮肤目录下的所有 PNG 文件。
 * key 为相对路径（如 "../../src/assets/skins/shark/idle.png"）,
 * value 为 Vite 解析后的模块对象（包含 default 属性的 hash URL）。
 *
 * `import.meta.glob` 需要在编译时静态解析，
 * 所以必须使用字符串字面量模式。
 *
 * ⚠️ 重要：Vite glob 对同名文件（如 shark/idle.png 和 custom/idle.png）
 * 会去重，key 仅保留最终文件名。因此需要保留原始路径来区分不同皮肤。
 */
const SKIN_PNG_GLOB = import.meta.glob(
  "../../src/assets/skins/**/*.png",
  { eager: true }
);

// ─── 调试日志 ──────────────────────────────────────────────────────────────
if (import.meta.env.DEV) {
  const globKeys = Object.keys(SKIN_PNG_GLOB);
  console.log("[skin] SKIN_PNG_GLOB keys count:", globKeys.length);
  console.log("[skin] SKIN_PNG_GLOB keys (first 3):", globKeys.slice(0, 3));
  console.log("[skin] SKIN_PNG_GLOB keys (last 3):", globKeys.slice(-3));
}

// ─── 类型 ──────────────────────────────────────────────────────────────────

/** 皮肤加载器返回结果 */
interface SkinLoaderResult {
  /** 皮肤元数据（加载成功时） */
  metadata: SkinMetadata | null;
  /** 解析后的帧 URL 映射（状态 → URL） */
  frames: ParsedSkinFrames;
  /** 加载状态 */
  status: SkinLoadStatus;
  /** 错误信息（加载失败时） */
  error: string | null;
}

// ─── 工具函数 ──────────────────────────────────────────────────────────────

/**
 * 从 Vite glob 结果中提取所有皮肤的帧 URL。
 *
 * # 返回结构
 *
 * `{ [skinId]: { [filename]: hashURL } }`
 *
 * 例如：
 * ```ts
 * {
 *   "shark": { "idle.png": "/dist/assets/xxx-hash.png" },
 *   "shark": { "idle.png": "/dist/assets/yyy-hash.png" }
 * }
 * ```
 *
 * # 原理
 *
 * Vite glob eager 模式下，Object.entries() 返回的 key 是
 * 原始模块路径（如 "../../src/assets/skins/shark/idle.png"），
 * 而非去重后的最终文件名。通过解析路径中的 skin 目录名，
 * 可以正确区分不同皮肤的同一文件名。
 */
function extractAllSkinPngUrls(
  globMap: Record<string, any>
): Record<string, Record<string, string>> {
  const result: Record<string, Record<string, string>> = {};

  if (import.meta.env.DEV) {
    const globKeys = Object.keys(globMap);
    console.log("[skin] extractAllSkinPngUrls: glob keys count =", globKeys.length);
    console.log("[skin] extractAllSkinPngUrls: glob keys =", globKeys);
  }

  for (const [originalPath, module] of Object.entries(globMap)) {
    // originalPath 格式: "../../src/assets/skins/{skinId}/{filename}.png"
    const segments = originalPath.split("/");
    const fileName = segments.pop() || "";
    const skinDir = segments.pop() || "";

    if (import.meta.env.DEV) {
      console.log(`[skin] glob entry: path="${originalPath}" → segments=`, segments.length, "skinDir=", skinDir, "fileName=", fileName);
    }

    if (!fileName || !skinDir) {
      if (import.meta.env.DEV) {
        console.warn(`[skin] Skipping entry: path="${originalPath}" fileName="${fileName}" skinDir="${skinDir}"`);
      }
      continue;
    }

    // module 可能是模块对象 { default: url } 或裸 URL 字符串
    let url: string;
    if (typeof module === "string") {
      url = module;
    } else if (typeof module === "object" && module !== null && "default" in module) {
      const mod = module as Record<string, unknown>;
      const def = mod.default;
      url = typeof def === "string" ? def : String(def);
    } else {
      if (import.meta.env.DEV) {
        console.warn(`[skin] Skipping entry (invalid module): path="${originalPath}" type=`, typeof module, "module=", module);
      }
      continue;
    }

    // 跳过空 URL
    if (!url) {
      if (import.meta.env.DEV) {
        console.warn(`[skin] Skipping entry (empty URL): path="${originalPath}" url="${url}"`);
      }
      continue;
    }

    if (import.meta.env.DEV) {
      console.log(`[skin] ✅ Extracted URL for ${skinDir}/${fileName}:`, url);
    }

    if (!result[skinDir]) {
      result[skinDir] = {};
    }
    result[skinDir][fileName] = url;
  }

  if (import.meta.env.DEV) {
    console.log("[skin] extractAllSkinPngUrls result:", JSON.stringify(result, null, 2));
  }

  return result;
}

/**
 * 从预构建的全局映射中提取指定皮肤的所有帧 URL。
 *
 * # 参数
 *
 * * `skinName` — 皮肤目录名称（如 "shark"）
 * * `allUrls` — extractAllSkinPngUrls 返回的全局映射
 *
 * # 返回
 *
 * `{ 文件名: hash URL }` 映射
 */
function extractSkinPngUrls(
  skinName: string,
  allUrls: Record<string, Record<string, string>>
): Record<string, string> {
  return allUrls[skinName] || {};
}

// ─── 预计算全局映射（模块级）─────────────────────────────────────────────────

/**
 * 预计算的皮肤帧 URL 映射。
 *
 * 在模块加载时即构建完成，避免每次调用时重复遍历 globMap。
 * 这也是为什么将 extractAllSkinPngUrls 调用放在模块顶层的原因。
 */
const ALL_SKIN_PNG_URLS = (() => {
  const result = extractAllSkinPngUrls(SKIN_PNG_GLOB);
  if (import.meta.env.DEV) {
    console.log("[skin] ALL_SKIN_PNG_URLS:", JSON.stringify(result, null, 2));
  }
  return result;
})();

/**
 * 将 skin.json 帧文件名映射解析为帧 URL 映射。
 *
 * # 参数
 *
 * * `frames` — skin.json 中的帧映射（文件名 → 文件名）
 * * `builtinUrls` — 内置皮肤 PNG URL 映射（文件名 → URL）
 * * `basePath` — 用户皮肤的基路径（文件系统路径）
 * * `isBuiltin` — 是否为内置皮肤
 *
 * # 返回
 *
 * 状态 → URL 的映射。如果某个状态没有对应的帧文件，
 * 则 fallback 到 idle 帧。
 */
function parseFrameUrls(
  frames: SkinMetadata["frames"],
  builtinUrls: Record<string, string>,
  basePath: string,
  isBuiltin: boolean
): ParsedSkinFrames {
  const result: ParsedSkinFrames = {};

  const allStates: PetState[] = [
    "idle",
    "thinking",
    "working",
    "waiting",
    "success",
    "error",
    "speaking",
    "connecting",
  ];

  for (const state of allStates) {
    const filename = frames?.[state] || frames?.idle;

    if (!filename) {
      continue;
    }

    let resolvedUrl: string | undefined;
    if (isBuiltin && builtinUrls[filename]) {
      // 内置皮肤：使用 Vite hash URL
      resolvedUrl = builtinUrls[filename];
    } else if (!isBuiltin && basePath) {
      // 用户皮肤：使用文件系统路径
      resolvedUrl = `${basePath}/${filename}`;
    } else {
      // fallback 到 idle
      const idleFile = frames?.idle;
      if (idleFile) {
        if (isBuiltin && builtinUrls[idleFile]) {
          resolvedUrl = builtinUrls[idleFile];
        } else if (!isBuiltin && basePath) {
          resolvedUrl = `${basePath}/${idleFile}`;
        }
      }
    }

    if (resolvedUrl) {
      result[state] = resolvedUrl;
      if (import.meta.env.DEV) {
        console.log(`[skin] parseFrameUrls: ${state} -> ${filename} =>`, resolvedUrl);
      }
    } else {
      if (import.meta.env.DEV) {
        console.warn(`[skin] parseFrameUrls: ${state} -> ${filename} NO URL`);
      }
    }
  }

  if (import.meta.env.DEV) {
    console.log(`[skin] parseFrameUrls complete (isBuiltin=${isBuiltin}):`, JSON.stringify(result, null, 2));
  }

  return result;
}

/**
 * 从 skin.json 内容解析元数据。
 */
function parseSkinMetadata(content: string): SkinMetadata {
  const json = JSON.parse(content);
  return {
    id: json.id,
    name: json.name,
    author: json.author,
    version: json.version,
    description: json.description,
    frame_count: json.frame_count,
    frames: json.frames,
    indicators: json.indicators,
  };
}

// ─── Hook ──────────────────────────────────────────────────────────────────

/**
 * 皮肤加载 Hook
 *
 * 根据 skinId 加载皮肤元数据和帧 URL 映射。
 * 支持自动热更新：当 skinId 变化时自动重新加载。
 *
 * @param skinId - 皮肤唯一标识（来自 config.json 的 pet.skinId）
 * @returns 皮肤加载结果
 */
export function useSkinLoader(skinId: string): SkinLoaderResult {
  const [metadata, setMetadata] = useState<SkinMetadata | null>(null);
  const [frames, setFrames] = useState<ParsedSkinFrames>({});
  const [status, setStatus] = useState<SkinLoadStatus>("loading");
  const [error, setError] = useState<string | null>(null);

  // 使用 ref 跟踪上一次加载的皮肤 ID，避免重复加载
  const lastSkinIdRef = useRef<string | null>(null);

  const loadSkin = useCallback(
    async (id: string) => {
      if (import.meta.env.DEV) {
        console.log("[skin] loadSkin called with id:", id);
        console.log("[skin] ALL_SKIN_PNG_URLS (global):", JSON.stringify(ALL_SKIN_PNG_URLS, null, 2));
      }

      // 防止重复加载
      if (lastSkinIdRef.current === id) {
        if (import.meta.env.DEV) {
          console.log("[skin] Skipping duplicate load");
        }
        return;
      }
      lastSkinIdRef.current = id;

      setStatus("loading");
      setError(null);
      setMetadata(null);
      setFrames({});

      try {
        // 1. 获取所有可用皮肤列表
        const skins: SkinInfo[] = await invoke<SkinInfo[]>("list_skins");
        const target = skins.find((s) => s.id === id);

        if (!target) {
          setError(`皮肤 "${id}" 不存在`);
          setStatus("error");
          return;
        }

        // 2. 获取皮肤元数据
        let meta: SkinMetadata;
        const isBuiltin = !target.custom;

        try {
          const raw = await invoke<Record<string, unknown>>(
            "get_skin_metadata",
            { skinId: id }
          );
          meta = parseSkinMetadata(JSON.stringify(raw));
        } catch {
          // fallback: 如果没有 get_skin_metadata，尝试直接解析
          setError(`无法加载皮肤 "${id}" 的元数据`);
          setStatus("error");
          return;
        }

        // 3. 构建帧 URL 映射
        let builtinUrls: Record<string, string> = {};
        if (isBuiltin) {
          // 内置皮肤：从 Vite glob 中提取
          builtinUrls = extractSkinPngUrls(id, ALL_SKIN_PNG_URLS);
          if (import.meta.env.DEV) {
            console.log("[skin] extractSkinPngUrls(" + id + "):", builtinUrls);
            console.log("[skin] builtinUrls keys:", Object.keys(builtinUrls));
            console.log("[skin] ALL_SKIN_PNG_URLS keys:", Object.keys(ALL_SKIN_PNG_URLS));
          }

          if (Object.keys(builtinUrls).length === 0) {
            // Vite glob 中没找到帧文件，fallback 到文件系统路径
            const fallbackPath = target.image_path;
            if (import.meta.env.DEV) {
              console.log("[skin] builtinUrls empty, fallbackPath:", fallbackPath);
            }
            if (fallbackPath) {
              const frames = parseFrameUrls(
                meta.frames,
                {},
                fallbackPath,
                false
              );
              setMetadata(meta);
              setFrames(frames);
              setStatus("loaded");
              return;
            }
          }
        }

        const parsedFrames = parseFrameUrls(
          meta.frames,
          builtinUrls,
          target.image_path,
          isBuiltin
        );

        if (import.meta.env.DEV) {
          console.log("[skin] parsedFrames:", parsedFrames);
        }

        setMetadata(meta);
        setFrames(parsedFrames);
        setStatus("loaded");
      } catch (e) {
        const errMsg = e instanceof Error ? e.message : String(e);
        setError(`加载皮肤失败: ${errMsg}`);
        setStatus("error");
      }
    },
    []
  );

  useEffect(() => {
    if (import.meta.env.DEV) {
      console.log("[skin] useSkinLoader effect triggered with skinId:", skinId);
    }
    if (skinId) {
      loadSkin(skinId);
    }
  }, [skinId, loadSkin]);

  return { metadata, frames, status, error };
}
