/// 插件管理器。
///
/// 负责插件的加载、卸载、生命周期管理和皮肤数据获取。
/// 所有插件必须实现 `Plugin` trait。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::types::PetState;

// ─── PluginError ────────────────────────────────────────────────────────────

/// 插件系统错误类型。
///
/// 所有插件操作错误统一使用此枚举，便于前端和上层调用者处理。
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// 插件未找到
    #[error("Plugin not found: {0}")]
    NotFound(String),
    /// 插件已加载
    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),
    /// 插件加载失败
    #[error("Plugin load failed: {0}")]
    LoadFailed(String),
    /// 插件初始化失败
    #[error("Plugin init failed: {0}")]
    InitFailed(String),
    /// 插件内部错误
    #[error("Plugin error: {0}")]
    Other(String),
}

// ─── Plugin trait ───────────────────────────────────────────────────────────

/// 插件 trait — 所有插件必须实现此接口。
///
/// 插件管理器通过此 trait 与所有插件交互，包括生命周期管理和数据获取。
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 插件唯一标识，用于加载/卸载/查询。
    fn id(&self) -> &str;
    /// 插件显示名称。
    fn name(&self) -> &str;
    /// 语义化版本号，如 "1.0.0"。
    fn version(&self) -> &str;
    /// 插件描述。
    fn description(&self) -> &str;
    /// 插件类型，可选值：`skin`、`animation`、`voice`、`notification`。
    fn plugin_type(&self) -> &str;
    /// 初始化插件。
    ///
    /// 在插件加载后、可用前调用，用于初始化资源、连接外部服务等。
    async fn init(&self) -> Result<(), PluginError>;
    /// 销毁插件。
    ///
    /// 在插件卸载时调用，用于清理资源、关闭连接等。
    async fn destroy(&self) -> Result<(), PluginError>;
    /// 获取皮肤数据（如果是 skin 类型插件）。
    ///
    /// 非 skin 类型插件可返回 `None`。
    fn get_skin_data(&self) -> Option<serde_json::Value>;
    /// 处理事件（可选实现，默认无操作）。
    ///
    /// 当插件管理器分发事件时调用。
    async fn on_event(
        &self,
        _event_type: &str,
        _data: serde_json::Value,
    ) {
        // 默认无操作
    }
    /// 宠物状态变更回调（可选实现，默认无操作）。
    ///
    /// 当宠物状态发生变化时调用。
    async fn on_state_change(
        &self,
        _old: &PetState,
        _new: &PetState,
    ) {
        // 默认无操作
    }
}

// ─── PluginInfo ─────────────────────────────────────────────────────────────

/// 插件元数据（用于前端展示）。
///
/// 由 `PluginManager::list_plugins()` 返回，不包含内部状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// 插件唯一标识。
    pub id: String,
    /// 插件显示名称。
    pub name: String,
    /// 语义化版本号。
    pub version: String,
    /// 插件描述。
    pub description: String,
    /// 插件类型。
    pub plugin_type: String,
    /// 是否已加载。
    pub enabled: bool,
}

// ─── DefaultSkinPlugin ──────────────────────────────────────────────────────

/// 内置默认皮肤插件。
///
/// 提供一套基础宠物外观，无需外部文件即可运行。
pub struct DefaultSkinPlugin;

#[async_trait]
impl Plugin for DefaultSkinPlugin {
    fn id(&self) -> &str {
        "default-skin"
    }

    fn name(&self) -> &str {
        "默认皮肤"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Agent Pet Hub 内置默认皮肤，提供基础宠物外观。"
    }

    fn plugin_type(&self) -> &str {
        "skin"
    }

    async fn init(&self) -> Result<(), PluginError> {
        info!(plugin_id = "default-skin", "Default skin plugin initialized");
        Ok(())
    }

    async fn destroy(&self) -> Result<(), PluginError> {
        info!(plugin_id = "default-skin", "Default skin plugin destroyed");
        Ok(())
    }

    fn get_skin_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "id": "default-skin",
            "name": "默认皮肤",
            "description": "Agent Pet Hub 内置默认皮肤",
            "image_path": "skins/shark",
            "custom": false,
            "frames": ["idle_1.png", "idle_2.png"],
            "colors": {
                "primary": "#4A90D9",
                "accent": "#FF6B6B"
            },
            "animations": {
                "idle": "breathing",
                "click": "bounce",
                "error": "shake"
            }
        }))
    }
}

// ─── PluginManager ──────────────────────────────────────────────────────────

/// 插件管理器。
///
/// 管理所有已注册插件的生命周期，提供线程安全的加载/卸载/查询接口。
///
/// # 线程安全
///
/// 内部使用 `RwLock` 保护插件集合，允许多读单写。
pub struct PluginManager {
    /// 已注册插件映射（ID → Arc<dyn Plugin>）。
    /// 使用 Arc 以便跨线程安全共享插件引用。
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
    /// 插件目录路径。
    plugin_dir: PathBuf,
}

impl PluginManager {
    /// 创建新的插件管理器。
    ///
    /// # 参数
    ///
    /// * `plugin_dir` — 插件目录路径。默认为 `~/.local/share/agent-pet-hub/plugins/`。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use agent_pet_hub_lib::plugin::PluginManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = PluginManager::new(PathBuf::from("/path/to/plugins"));
    /// ```
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            plugin_dir,
        }
    }

    /// 加载插件。
    ///
    /// 将插件添加到管理器中。如果已存在同 ID 的插件，则返回 `AlreadyLoaded` 错误。
    /// 加载成功后自动调用插件的 `init()` 方法。
    ///
    /// # 参数
    ///
    /// * `plugin` — 要加载的插件实例（Arc<dyn Plugin>）。
    ///
    /// # 返回值
    ///
    /// - `Ok(())` — 插件加载并初始化成功。
    /// - `Err(PluginError::AlreadyLoaded)` — 插件 ID 已存在。
    /// - `Err(PluginError::InitFailed)` — 插件初始化失败。
    pub async fn load_plugin(
        &self,
        plugin: Arc<dyn Plugin>,
    ) -> Result<(), PluginError> {
        let id = plugin.id().to_string();

        // 检查是否已加载同名插件
        {
            let plugins = self.plugins.read();
            if plugins.contains_key(&id) {
                warn!(plugin_id = %id, "Plugin already loaded");
                return Err(PluginError::AlreadyLoaded(id));
            }
        }

        // 调用 init()
        plugin.init().await?;

        // 注册插件
        {
            let mut plugins = self.plugins.write();
            plugins.insert(id.clone(), plugin);
            info!(plugin_id = %id, plugin_name = %self.plugins.read().get(&id).map(|p| p.name()).unwrap_or("?"), "Plugin loaded successfully");
        }

        Ok(())
    }

    /// 卸载插件。
    ///
    /// 调用插件的 `destroy()` 方法清理资源，然后从管理器中移除。
    ///
    /// # 参数
    ///
    /// * `id` — 要卸载的插件 ID。
    ///
    /// # 返回值
    ///
    /// - `Ok(())` — 插件卸载成功。
    /// - `Err(PluginError::NotFound)` — 插件不存在。
    pub async fn unload_plugin(&self, id: &str) -> Result<(), PluginError> {
        let plugin = {
            let mut plugins = self.plugins.write();
            plugins.remove(id).ok_or_else(|| PluginError::NotFound(id.to_string()))?
        };

        // 调用 destroy()
        plugin.destroy().await?;

        info!(plugin_id = %id, "Plugin unloaded successfully");
        Ok(())
    }

    /// 列出所有已注册插件的元数据。
    ///
    /// # 返回值
    ///
    /// 返回所有插件的 `PluginInfo` 列表。
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read();
        plugins
            .values()
            .map(|p| PluginInfo {
                id: p.id().to_string(),
                name: p.name().to_string(),
                version: p.version().to_string(),
                description: p.description().to_string(),
                plugin_type: p.plugin_type().to_string(),
                enabled: true,
            })
            .collect()
    }

    /// 获取指定插件。
    ///
    /// # 参数
    ///
    /// * `id` — 插件 ID。
    ///
    /// # 返回值
    ///
    /// 返回插件的 `Arc` 引用，如果不存在则返回 `None`。
    /// 返回 `Arc` 而非 `&dyn Plugin` 以避免 `RwLockReadGuard` 的临时值借用问题。
    pub fn get_plugin(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read().get(id).cloned()
    }

    /// 获取当前活跃皮肤数据。
    ///
    /// 遍历所有已加载插件，返回第一个 skin 类型插件的皮肤数据。
    /// 如果没有 skin 类型插件，返回 `None`。
    pub fn get_active_skin(&self) -> Option<serde_json::Value> {
        let plugins = self.plugins.read();
        for plugin in plugins.values() {
            if plugin.plugin_type() == "skin" {
                return plugin.get_skin_data();
            }
        }
        None
    }

    /// 扫描插件目录并加载所有插件。
    ///
    /// 扫描 `plugin_dir` 目录下的 `.json` 插件清单文件，
    /// 每个 JSON 文件应包含插件的 `id` 字段。
    /// 对于每个清单文件，尝试从 `plugins/{id}/` 目录下加载对应的插件模块。
    ///
    /// 内置的 `DefaultSkinPlugin` 会始终被加载。
    ///
    /// # 参数
    ///
    /// * `plugin_dir` — 插件目录路径。
    ///
    /// # 返回值
    ///
    /// 返回成功加载的插件数量。
    pub async fn reload_from_dir(
        &self,
    ) -> Result<usize, PluginError> {
        let dir = &self.plugin_dir;
        let mut count = 0usize;

        // 始终加载内置默认皮肤插件
        let default_skin = Arc::new(DefaultSkinPlugin);
        match self.load_plugin(default_skin).await {
            Ok(()) => {
                count += 1;
            }
            Err(e) => {
                debug!(error = %e, "Default skin plugin already loaded or failed");
            }
        }

        // 扫描插件目录下的 .json 清单文件
        if dir.exists() {
            let entries = std::fs::read_dir(dir).map_err(|e| {
                PluginError::Other(format!("Failed to read plugin dir: {e}"))
            })?;

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        warn!(error = %e, "Failed to read directory entry, skipping");
                        continue;
                    }
                };

                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let ext = path.extension().and_then(|e| e.to_str());
                if ext != Some("json") {
                    continue;
                }

                // 读取插件清单文件
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Failed to read plugin manifest, skipping");
                        continue;
                    }
                };

                let manifest: PluginManifest = match serde_json::from_str(&content) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to parse plugin manifest, skipping"
                        );
                        continue;
                    }
                };

                // 使用清单中的 ID 加载插件
                let plugin = Arc::new(LoadedPlugin::from_manifest(&manifest).await);
                match self.load_plugin(plugin).await {
                    Ok(()) => {
                        count += 1;
                        info!(plugin_id = %manifest.id, "Plugin loaded from manifest");
                    }
                    Err(e) => {
                        warn!(
                            plugin_id = %manifest.id,
                            error = %e,
                            "Failed to load plugin from manifest"
                        );
                    }
                }
            }
        } else {
            debug!(plugin_dir = %dir.display(), "Plugin directory does not exist, using defaults only");
        }

        info!(total_loaded = count, "Plugin reload completed");
        Ok(count)
    }

    /// 分发事件到所有已注册插件。
    ///
    /// 按注册顺序依次调用每个插件的 `on_event()` 方法。
    pub async fn dispatch_event(&self, event_type: &str, data: serde_json::Value) {
        let plugins = self.plugins.read();
        for plugin in plugins.values() {
            plugin.on_event(event_type, data.clone()).await;
        }
    }

    /// 分发状态变更到所有已注册插件。
    ///
    /// 按注册顺序依次调用每个插件的 `on_state_change()` 方法。
    pub async fn dispatch_state_change(
        &self,
        old: &PetState,
        new: &PetState,
    ) {
        let plugins = self.plugins.read();
        for plugin in plugins.values() {
            plugin.on_state_change(old, new).await;
        }
    }

    /// 销毁所有已注册插件。
    ///
    /// 按顺序调用每个插件的 `destroy()` 方法，记录失败信息但不中断流程，
    /// 最后清空插件列表。
    pub async fn destroy_all(&self) {
        let ids: Vec<String> = {
            let plugins = self.plugins.read();
            plugins.keys().cloned().collect()
        };

        for id in ids {
            if let Err(e) = self.unload_plugin(&id).await {
                warn!(plugin_id = %id, error = %e, "Failed to destroy plugin");
            }
        }

        info!("All plugins destroyed");
    }
}

// ─── PluginManifest (用于 .json 插件清单) ──────────────────────────────────

/// 插件清单文件结构。
///
/// 每个 `.json` 文件描述一个可加载插件的元数据。
#[derive(Debug, Deserialize)]
struct PluginManifest {
    /// 插件唯一标识。
    id: String,
    /// 插件显示名称。
    name: String,
    /// 语义化版本号。
    version: String,
    /// 插件描述。
    description: String,
    /// 插件类型。
    #[serde(default = "default_plugin_type")]
    plugin_type: String,
    /// 入口路径（相对于插件目录）。
    #[serde(default)]
    #[allow(dead_code)]
    entry: Option<String>,
}

fn default_plugin_type() -> String {
    "skin".to_string()
}

// ─── LoadedPlugin (从 manifest 加载的占位插件) ─────────────────────────────

/// 从 JSON manifest 动态加载的插件占位实现。
///
/// 在 `reload_from_dir` 中使用，从 manifest 文件提取元数据并注册为插件。
struct LoadedPlugin {
    id: String,
    name: String,
    version: String,
    description: String,
    plugin_type: String,
}

impl LoadedPlugin {
    /// 从 manifest 创建插件实例（init 时执行异步初始化）。
    async fn from_manifest(manifest: &PluginManifest) -> Self {
        debug!(
            plugin_id = %manifest.id,
            plugin_name = %manifest.name,
            "Creating plugin from manifest"
        );
        Self {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            plugin_type: manifest.plugin_type.clone(),
        }
    }
}

#[async_trait]
impl Plugin for LoadedPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn plugin_type(&self) -> &str {
        &self.plugin_type
    }

    async fn init(&self) -> Result<(), PluginError> {
        info!(plugin_id = %self.id, "LoadedPlugin initialized");
        Ok(())
    }

    async fn destroy(&self) -> Result<(), PluginError> {
        info!(plugin_id = %self.id, "LoadedPlugin destroyed");
        Ok(())
    }

    fn get_skin_data(&self) -> Option<serde_json::Value> {
        None
    }
}

// ─── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PetState;

    /// 测试插件：用于测试的模拟插件。
    struct TestPlugin {
        id: String,
        name: String,
        version: String,
        description: String,
        plugin_type: String,
        skin_data: Option<serde_json::Value>,
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn plugin_type(&self) -> &str {
            &self.plugin_type
        }

        async fn init(&self) -> Result<(), PluginError> {
            Ok(())
        }

        async fn destroy(&self) -> Result<(), PluginError> {
            Ok(())
        }

        fn get_skin_data(&self) -> Option<serde_json::Value> {
            self.skin_data.clone()
        }
    }

    impl TestPlugin {
        fn new(id: &str, name: &str, ptype: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: format!("Test plugin: {id}"),
                plugin_type: ptype.to_string(),
                skin_data: None,
            }
        }

        fn with_skin(id: &str, skin: serde_json::Value) -> Self {
            Self {
                id: id.to_string(),
                name: format!("Test Skin: {id}"),
                version: "1.0.0".to_string(),
                description: "Test skin plugin".to_string(),
                plugin_type: "skin".to_string(),
                skin_data: Some(skin),
            }
        }
    }

    // ─── PluginManager 基础测试 ─────────────────────────────────────

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        assert_eq!(manager.list_plugins().len(), 0);
    }

    #[tokio::test]
    async fn test_load_plugin_success() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        let plugin = Arc::new(TestPlugin::new("test-1", "Test Plugin 1", "skin"));
        manager.load_plugin(plugin).await.unwrap();

        let list = manager.list_plugins();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "test-1");
        assert_eq!(list[0].name, "Test Plugin 1");
        assert!(list[0].enabled);
    }

    #[tokio::test]
    async fn test_load_duplicate_plugin() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        let plugin = Arc::new(TestPlugin::new("test-2", "Test Plugin 2", "skin"));
        manager.load_plugin(plugin.clone()).await.unwrap();

        let result = manager.load_plugin(plugin).await;
        assert!(matches!(result, Err(PluginError::AlreadyLoaded(_))));
    }

    #[tokio::test]
    async fn test_unload_plugin() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        let plugin = Arc::new(TestPlugin::new("test-3", "Test Plugin 3", "skin"));
        manager.load_plugin(plugin).await.unwrap();

        assert_eq!(manager.list_plugins().len(), 1);

        manager.unload_plugin("test-3").await.unwrap();
        assert_eq!(manager.list_plugins().len(), 0);
    }

    #[tokio::test]
    async fn test_unload_nonexistent_plugin() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        let result = manager.unload_plugin("nonexistent").await;
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_plugin() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        let plugin = Arc::new(TestPlugin::new("test-4", "Test Plugin 4", "skin"));
        manager.load_plugin(plugin).await.unwrap();

        let retrieved = manager.get_plugin("test-4");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "Test Plugin 4");

        assert!(manager.get_plugin("missing").is_none());
    }

    // ─── get_active_skin 测试 ─────────────────────────────────────────

    #[tokio::test]
    async fn test_get_active_skin() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));

        // 无皮肤插件时返回 None
        assert!(manager.get_active_skin().is_none());

        // 加载 skin 插件后返回皮肤数据
        let skin_data = serde_json::json!({
            "id": "test-skin",
            "name": "Test Skin",
            "image_path": "skins/test"
        });
        let skin_plugin = Arc::new(TestPlugin::with_skin("test-skin", skin_data.clone()));
        manager.load_plugin(skin_plugin).await.unwrap();

        let active = manager.get_active_skin();
        assert!(active.is_some());
        let active = active.unwrap();
        assert_eq!(active["id"], "test-skin");
    }

    #[tokio::test]
    async fn test_get_active_skin_ignores_non_skin() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));

        let anim_plugin = Arc::new(TestPlugin::new("test-anim", "Animation", "animation"));
        manager.load_plugin(anim_plugin).await.unwrap();

        assert!(manager.get_active_skin().is_none());
    }

    // ─── DefaultSkinPlugin 测试 ──────────────────────────────────────

    #[tokio::test]
    async fn test_default_skin_plugin_traits() {
        let plugin = DefaultSkinPlugin;

        assert_eq!(plugin.id(), "default-skin");
        assert_eq!(plugin.name(), "默认皮肤");
        assert_eq!(plugin.plugin_type(), "skin");
        assert!(!plugin.version().is_empty());
        assert!(!plugin.description().is_empty());
    }

    #[tokio::test]
    async fn test_default_skin_plugin_init_destroy() {
        let plugin = DefaultSkinPlugin;
        assert!(plugin.init().await.is_ok());
        assert!(plugin.destroy().await.is_ok());
    }

    #[tokio::test]
    async fn test_default_skin_plugin_skin_data() {
        let plugin = DefaultSkinPlugin;
        let skin = plugin.get_skin_data();
        assert!(skin.is_some());

        let skin = skin.unwrap();
        assert_eq!(skin["id"], "default-skin");
        assert_eq!(skin["name"], "默认皮肤");
        assert_eq!(skin["custom"], false);
    }

    // ─── reload_from_dir 测试 ─────────────────────────────────────────

    #[tokio::test]
    async fn test_reload_from_dir_creates_default() {
        let tmp = std::env::temp_dir().join("agent-pet-hub-test-plugins");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let manager = PluginManager::new(tmp.clone());
        let count = manager.reload_from_dir().await.unwrap();

        assert!(count >= 1, "应至少加载 default-skin 插件");
        assert!(manager.get_plugin("default-skin").is_some());

        // 清理
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ─── PluginInfo 序列化测试 ───────────────────────────────────────

    #[test]
    fn test_plugin_info_serde() {
        let info = PluginInfo {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            plugin_type: "skin".to_string(),
            enabled: true,
        };

        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], "test-plugin");
        assert_eq!(json["enabled"], true);

        let deserialized: PluginInfo = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.id, "test-plugin");
        assert!(deserialized.enabled);
    }

    // ─── PluginError 测试 ────────────────────────────────────────────

    #[test]
    fn test_plugin_error_display() {
        let err = PluginError::NotFound("missing".to_string());
        assert_eq!(format!("{}", err), "Plugin not found: missing");

        let err = PluginError::AlreadyLoaded("dup".to_string());
        assert_eq!(format!("{}", err), "Plugin already loaded: dup");

        let err = PluginError::Other("boom".to_string());
        assert_eq!(format!("{}", err), "Plugin error: boom");
    }

    // ─── dispatch_event 测试 ─────────────────────────────────────────

    #[tokio::test]
    async fn test_dispatch_event() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        let plugin = Arc::new(TestPlugin::new("test-dispatch", "Dispatch Test", "skin"));
        manager.load_plugin(plugin).await.unwrap();

        let event_data = serde_json::json!({ "action": "click", "x": 100, "y": 200 });
        manager
            .dispatch_event("pet.click", event_data)
            .await;
    }

    // ─── dispatch_state_change 测试 ──────────────────────────────────

    #[tokio::test]
    async fn test_dispatch_state_change() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));
        let plugin = Arc::new(TestPlugin::new("test-state", "State Test", "skin"));
        manager.load_plugin(plugin).await.unwrap();

        manager
            .dispatch_state_change(&PetState::Idle, &PetState::Working)
            .await;
    }

    // ─── destroy_all 测试 ────────────────────────────────────────────

    #[tokio::test]
    async fn test_destroy_all() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));

        let p1 = Arc::new(TestPlugin::new("destroy-1", "Destroy 1", "skin"));
        let p2 = Arc::new(TestPlugin::new("destroy-2", "Destroy 2", "animation"));
        manager.load_plugin(p1).await.unwrap();
        manager.load_plugin(p2).await.unwrap();

        assert_eq!(manager.list_plugins().len(), 2);
        manager.destroy_all().await;
        assert_eq!(manager.list_plugins().len(), 0);
    }

    // ─── 并发安全测试 ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_concurrent_load_and_list() {
        let manager = PluginManager::new(PathBuf::from("/tmp/test-plugins"));

        let mut handles = vec![];
        for i in 0..10u32 {
            let mgr = manager.clone_arc();
            let id = format!("concurrent-{i}");
            handles.push(tokio::spawn(async move {
                let plugin = Arc::new(TestPlugin::new(&id, &id, "skin"));
                let _ = mgr.load_plugin(plugin).await;
            }));
        }

        // 等待所有加载完成
        for h in handles {
            h.await.unwrap();
        }

        // list_plugins 应在读锁保护下正常工作
        let list = manager.list_plugins();
        assert!(list.len() <= 10);
    }

    // 为并发测试提供 Arc 克隆
    impl PluginManager {
        fn clone_arc(&self) -> Arc<Self> {
            Arc::new(self.clone())
        }
    }

    impl Clone for PluginManager {
        fn clone(&self) -> Self {
            Self {
                plugins: RwLock::new(self.plugins.read().clone()),
                plugin_dir: self.plugin_dir.clone(),
            }
        }
    }
}
