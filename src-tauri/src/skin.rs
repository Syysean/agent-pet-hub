/// 皮肤扫描模块。
///
/// 负责扫描内置皮肤和用户自定义皮肤，返回 `SkinInfo` 列表。
///
/// # 皮肤目录
///
/// - **内置皮肤**: `src/assets/skins/`（编译时嵌入）+ `src-tauri/resources/skins/`
/// - **用户自定义皮肤**: `~/.config/agent-pet-hub/skins/`（运行时扫描）
///
/// # 皮肤包格式
///
/// 每个皮肤是一个子目录，必须包含 `skin.json` 元数据文件。
/// `skin.json` 结构：
/// ```json
/// {
///   "id": "default",
///   "name": "Default Pet",
///   "description": "A cute pink round pet",
///   "frames": {
///     "idle": "idle.png",
///     ...
///   }
/// }
/// ```

use crate::types::PetSkin;
use include_dir::Dir as IncludeDir;
use tracing::{debug, info, warn};

// ─── 内置皮肤目录（编译时嵌入）────────────────────────────────────────────

/// 编译时嵌入的内置皮肤资源目录（来自 `src/assets/skins/`）。
static BUILTIN_SKINS_DIR: IncludeDir = include_dir::include_dir!("../src/assets/skins");

/// 编译时嵌入的传统资源皮肤目录（来自 `src-tauri/resources/skins/`）。
static RESOURCES_SKINS_DIR: IncludeDir = include_dir::include_dir!("resources/skins");

/// 从内置皮肤目录获取所有皮肤子目录的名称。
fn builtin_dir_names(dir: &IncludeDir) -> Vec<String> {
    dir.dirs()
        .filter_map(|d| d.path().file_name().and_then(|n| n.to_str()))
        .map(String::from)
        .collect()
}

/// 检查内置皮肤目录中，指定皮肤目录是否包含 skin.json 文件。
///
/// # 注意
///
/// `include_dir` 的文件路径是绝对路径（相对于嵌入根目录），
/// 所以需要查找 `skin_name/skin.json` 而不是 `skin.json`。
fn builtin_has_skin_json(dir: &IncludeDir, skin_name: &str) -> bool {
    let search_path = format!("{}/skin.json", skin_name);
    dir.get_file(&search_path).is_some()
}

/// 获取内置皮肤目录中指定皮肤的 skin.json 内容。
fn builtin_skin_json_content(dir: &IncludeDir, skin_name: &str) -> Option<String> {
    let search_path = format!("{}/skin.json", skin_name);
    dir.get_file(&search_path)
        .and_then(|f| f.contents_utf8().map(String::from))
}

// ─── 皮肤扫描核心逻辑───────────────────────────────────────────────────────

/// 扫描所有可用皮肤（内置 + 用户自定义）。
///
/// # 返回
///
/// `Vec<PetSkin>` — 所有发现的皮肤信息列表。
/// 列表顺序：先内置皮肤，后用户自定义皮肤。
pub fn scan_all_skins(user_skins_dir: Option<std::path::PathBuf>) -> Vec<PetSkin> {
    let mut skins = Vec::new();

    // 1. 扫描内置 PNG 皮肤 (src/assets/skins/)
    scan_builtin_png_skins(&mut skins);

    // 2. 扫描传统资源皮肤 (src-tauri/resources/skins/) — 向后兼容
    scan_builtin_resource_skins(&mut skins);

    // 3. 扫描用户自定义皮肤 (~/.config/agent-pet-hub/skins/)
    if let Some(ref dir) = user_skins_dir {
        scan_directory_skins(dir, true, &mut skins);
    }

    info!(skin_count = skins.len(), "Skin scanning completed");
    skins
}

/// 扫描内置 PNG 皮肤目录 (src/assets/skins/)。
fn scan_builtin_png_skins(skins: &mut Vec<PetSkin>) {
    let dir_names = builtin_dir_names(&BUILTIN_SKINS_DIR);

    for dir_name in dir_names {
        // 跳过隐藏目录
        if dir_name.starts_with('.') {
            continue;
        }

        // 跳过已存在的皮肤（避免与 resource skins 重复）
        if skins.iter().any(|s| s.id == dir_name) {
            continue;
        }

        // 查找 skin.json
        if !builtin_has_skin_json(&BUILTIN_SKINS_DIR, &dir_name) {
            debug!(dir = dir_name, "Skipping builtin skin (no skin.json)");
            continue;
        }

        let content = match builtin_skin_json_content(&BUILTIN_SKINS_DIR, &dir_name) {
            Some(c) => c,
            None => continue,
        };

        let metadata: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                warn!(dir = dir_name, error = %e, "Failed to parse skin.json");
                continue;
            }
        };

        let skin = PetSkin {
            id: metadata.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: metadata.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            description: metadata.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            image_path: format!("assets/skins/{}", dir_name),
            custom: false,
        };

        debug!(skin_id = skin.id, skin_name = skin.name, "Registered builtin PNG skin");
        skins.push(skin);
    }
}

/// 扫描传统资源皮肤目录 (src-tauri/resources/skins/)。
fn scan_builtin_resource_skins(skins: &mut Vec<PetSkin>) {
    let dir_names = builtin_dir_names(&RESOURCES_SKINS_DIR);

    for dir_name in dir_names {
        if dir_name.starts_with('.') {
            continue;
        }

        // 跳过已存在的内置 PNG 皮肤，避免重复
        if skins.iter().any(|s| s.id == dir_name) {
            continue;
        }

        // 查找 index.json（旧版格式）或 skin.json
        // 路径格式：dir_name/filename
        let index_path = format!("{}/index.json", dir_name);
        let skin_path = format!("{}/skin.json", dir_name);

        let content = RESOURCES_SKINS_DIR
            .get_file(&index_path)
            .and_then(|f| std::fs::read_to_string(f.path()).ok())
            .or_else(|| {
                RESOURCES_SKINS_DIR
                    .get_file(&skin_path)
                    .and_then(|f| std::fs::read_to_string(f.path()).ok())
            });

        let Some(content) = content else {
            continue;
        };

        let metadata: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let skin = PetSkin {
            id: metadata.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: metadata.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            description: metadata.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            image_path: format!("resources/skins/{}", dir_name),
            custom: false,
        };

        debug!(skin_id = skin.id, "Registered builtin resource skin");
        skins.push(skin);
    }
}

/// 扫描用户自定义皮肤目录。
///
/// # 参数
///
/// * `dir` — 用户皮肤目录路径
/// * `custom` — 标记为用户皮肤（true）
/// * `skins` — 结果向量（追加）
pub fn scan_directory_skins(
    dir: &std::path::Path,
    custom: bool,
    skins: &mut Vec<PetSkin>,
) {
    if !dir.is_dir() {
        debug!(dir = ?dir, "User skins directory does not exist, skipping");
        return;
    }

    info!(dir = ?dir, "Scanning user skins directory");

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warn!(dir = ?dir, error = %e, "Failed to read user skins directory");
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "Failed to read directory entry");
                continue;
            }
        };

        let path = entry.path();

        // 只处理子目录
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        // 跳过隐藏目录
        if dir_name.starts_with('.') {
            continue;
        }

        // 查找 skin.json
        let skin_json_path = path.join("skin.json");
        if !skin_json_path.exists() {
            debug!(dir = dir_name, "Skipping (no skin.json)");
            continue;
        }

        let content = match std::fs::read_to_string(&skin_json_path) {
            Ok(c) => c,
            Err(e) => {
                warn!(dir = dir_name, error = %e, "Failed to read skin.json");
                continue;
            }
        };

        let metadata: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                warn!(dir = dir_name, error = %e, "Failed to parse skin.json");
                continue;
            }
        };

        // 尝试规范化路径（解析 ..、冗余分隔符等），失败则保留原始路径
        let image_path = path.canonicalize()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let skin = PetSkin {
            id: metadata.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: metadata.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            description: metadata.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            image_path,
            custom,
        };

        debug!(skin_id = skin.id, skin_name = skin.name, custom, "Registered user skin");
        skins.push(skin);
    }
}

// ─── 皮肤验证 ──────────────────────────────────────────────────────────────

/// 验证 skin_id 是否存在于可用皮肤列表中。
pub fn validate_skin_id(skin_id: &str, available: &[PetSkin]) -> bool {
    available.iter().any(|s| s.id == skin_id)
}

/// 获取指定 ID 的皮肤信息。
pub fn find_skin_by_id<'a>(skins: &'a [PetSkin], id: &str) -> Option<&'a PetSkin> {
    skins.iter().find(|s| s.id == id)
}

// ─── 单元测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_all_skins_includes_builtin() {
        let dir_names = builtin_dir_names(&BUILTIN_SKINS_DIR);
        eprintln!("Builtin dir names: {:?}", dir_names);
        for name in &dir_names {
            let has_json = builtin_has_skin_json(&BUILTIN_SKINS_DIR, name);
            eprintln!("  Checking skin '{}', has skin.json: {}", name, has_json);
        }
        let skins = scan_all_skins(None);
        eprintln!(
            "Scanned skins count: {}, ids: {:?}",
            skins.len(),
            skins.iter().map(|s| &s.id).collect::<Vec<_>>()
        );
        let has_shark = skins.iter().any(|s| s.id == "shark");
        assert!(
            has_shark,
            "Should include builtin shark skin, found: {:?}",
            skins.iter().map(|s| &s.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_validate_skin_id() {
        let skins = scan_all_skins(None);
        assert!(validate_skin_id("shark", &skins));
        assert!(!validate_skin_id("nonexistent", &skins));
    }

    #[test]
    fn test_find_skin_by_id() {
        let skins = scan_all_skins(None);
        let found = find_skin_by_id(&skins, "shark");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Shark");

        let not_found = find_skin_by_id(&skins, "unknown");
        assert!(not_found.is_none());
    }
}

