# 宠物皮肤系统 — 架构分析与实施方案

> 日期：2026-06-11
> 状态：方案评审

---

## 一、项目现状分析

### 1.1 当前渲染架构

当前宠物渲染为**内联 SVG**（硬编码在 `PetSVG.tsx` 中），包含：
- 身体、眼睛、嘴巴、腮红等图形元素
- 状态指示器（Working 光环、Error X、Success 星）通过 `<g>` 元素 + `opacity` 控制
- 所有动画通过 **CSS keyframes** 驱动（呼吸、思考、震动、旋转等）

```
App.tsx
├── PetSVG.tsx        ← 内联 SVG，CSS class 控制状态动画
├── PetStatus.tsx     ← 底部状态文字
└── useAgentState.ts  ← Tauri 事件监听 → setPetState()
```

### 1.2 状态映射链

```
Agent Event (Rust state machine)
  → PetState enum (Rust)
    → Tauri IPC / WS event ("pet:state_changed")
      → React state (useAgentState)
        → PetSVG.tsx → CSS class (pet-idle, pet-thinking...)
          → index.css keyframes
```

### 1.3 已有基础设施

| 模块 | 皮肤相关 | 说明 |
|------|----------|------|
| `types/pet.rs` | ✅ | 已有 `PetSkin` struct (`id`, `name`, `description`, `image_path`, `custom`) |
| `config/settings.rs` | ✅ | `PetSettings.skin_id` 字段，默认 `"default"` |
| `App.tsx` | ⚠️ | 320×280 透明窗口，可直接支持 PNG |
| `index.css` | ✅ | 丰富的 CSS 动画体系，可直接复用 |

### 1.4 已有类型定义（Rust 端）

```rust
// src-tauri/src/types/pet.rs
pub struct PetSkin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub image_path: String,
    pub custom: bool,
}

// src-tauri/src/config/settings.rs — PetSettings
pub struct PetSettings {
    pub skin_id: String,        // ✅ 已有
    pub enabled: bool,
    pub show_status: bool,
}
```

---

## 二、架构可行性分析

### 2.1 核心方案：PNG 图片 + CSS 动画叠加

```
PetPNG.tsx (新组件)
├── 根据 skin_id 加载 skin.json + PNG
├── 渲染 <img src="..."> 标签
├── 通过 CSS class 叠加动画效果（shake, rotate, scale...）
└── 状态指示器：通过 overlay div 实现（而非 SVG <g>）
```

**可行性：✅ 高**

理由：
1. **CSS 动画完全兼容**：CSS `transform` 动画应用于 `<img>` 和 `<svg>` 效果相同
2. **已有 CSS 基础**：`index.css` 中的 8 种状态动画可直接复用，只需微调选择器
3. **窗口天然支持**：320×280 透明窗口，PNG 带 alpha 通道即可
4. **Rust 端已有类型**：`PetSkin` 和 `skin_id` 已定义
5. **状态映射链不变**：`PetState → CSS class → CSS animation` 链路完全保留

### 2.2 皮肤格式设计

#### skin.json（皮肤元数据）

```json
{
  "id": "shark",
  "name": "鲨鱼",
  "author": "Sean",
  "version": "1.0",
  "description": "一只可爱的鲨鱼宠物",
  "frame_count": {
    "width": 120,
    "height": 120
  },
  "frames": {
    "idle": "idle.png",
    "thinking": "thinking.png",
    "working": "working.png",
    "waiting": "waiting.png",
    "success": "success.png",
    "error": "error.png",
    "speaking": "speaking.png",
    "connecting": "connecting.png"
  },
  "indicators": {
    "working": {
      "type": "overlay",
      "style": "ring",
      "color": "#4CAF50"
    },
    "error": {
      "type": "overlay",
      "style": "cross",
      "color": "#F44336"
    },
    "success": {
      "type": "overlay",
      "style": "star",
      "color": "#FFD700"
    }
  }
}
```

**关键字段说明：**
- `id`: 唯一标识，与 `config.json` 中的 `skin_id` 对应
- `frame_count`: 帧尺寸，用于校验 PNG 尺寸一致性
- `frames`: 状态 → 文件名映射，键名与 `PetState` 枚举一致
- `indicators`: 定义非嵌入式的叠加指示器（Working/Error/Success），由 CSS 渲染，而非烘焙进 PNG

#### 为什么指示器用 overlay 而非嵌入 PNG？

| 方案 | 优点 | 缺点 |
|------|------|------|
| 嵌入 PNG | 皮肤作者完全控制外观 | 每个状态都需要一张图，切换指示器时需要新图 |
| **CSS overlay**（本方案） | 皮肤切换不影响指示器，指示器样式统一 | 皮肤外观风格需适配统一指示器 |

**决策**：采用 **CSS overlay**。原因：
1. 指示器只在特定状态短暂出现，统一样式不影响体验
2. 皮肤作者只需关注角色本身的绘制
3. 与现有 SVG `#indicator-working` 模式一致

### 2.3 目录结构

```
src/
├── assets/
│   └── skins/                          # 内置皮肤目录（随 Vite 打包）
│       ├── default/                    # 当前粉色圆脸 → PNG 版本
│       │   ├── skin.json               # 元数据
│       │   ├── idle.png
│       │   ├── thinking.png
│       │   ├── working.png
│       │   ├── waiting.png
│       │   ├── success.png
│       │   ├── error.png
│       │   ├── speaking.png
│       │   └── connecting.png
│       └── shark/                      # 示例皮肤（鲨鱼）
│           ├── skin.json
│           ├── idle.png
│           └── ...
└── components/
    └── PetPNG.tsx                        # 新皮肤渲染组件（替换 PetSVG）
```

**用户自定义皮肤目录**（运行时动态扫描）：
```
~/.config/agent-pet-hub/
├── config.json
└── skins/
    └── my-custom-skin/
        ├── skin.json
        ├── idle.png
        └── ...
```

### 2.4 关键技术决策

#### Q1: PNG 资源如何加载？

| 方案 | 适用性 | 原因 |
|------|--------|------|
| Vite `import` 静态资源 | ✅ 内置皮肤 | Vite 打包时自动处理，生成 hash 路径 |
| Rust IPC 返回文件路径 | ✅ 用户自定义皮肤 | 运行时扫描目录，返回路径供前端加载 |
| Base64 内联 | ❌ | 内存占用大，不适合多图 |

**决策**：
- **内置皮肤**：通过 Vite `import.meta.glob` 动态导入，返回 URL
- **用户自定义皮肤**：Rust IPC 扫描 `~/.config/agent-pet-hub/skins/`，返回 `{id, name, image_path}`，前端用路径加载

#### Q2: 如何保持 CSS 动画效果？

```css
/* 复用现有动画，只需将选择器从 .pet-puppet 改为 .pet-png */
.pet-png.pet-idle {
  animation: pet-breathe 3s ease-in-out infinite;
}

.pet-png.pet-thinking {
  animation: pet-think 2.5s ease-in-out infinite;
}

/* ... 其他状态动画保持不变 ... */
```

#### Q3: 状态过渡动画如何处理？

```tsx
// 切换皮肤/状态时添加淡入淡出过渡
.pet-transitioning {
  animation: pet-fade-transition 0.2s ease;
}

@keyframes pet-fade-transition {
  0%   { opacity: 1; }
  50%  { opacity: 0.3; }
  100% { opacity: 1; }
}
```

#### Q4: 窗口尺寸如何处理？

当前窗口固定 320×280。皮肤 PNG 建议统一使用 120×120（与现有 SVG viewBox 一致），居中显示在窗口内。

```tsx
<img
  className="pet-png"
  src={currentFrame}
  style={{
    position: "absolute",
    top: "50%",
    left: "50%",
    transform: "translate(-50%, -50%)",
  }}
/>
```

#### Q5: GIF 动图支持

`<img>` 标签天然支持 GIF，无需额外处理：
```json
{
  "frames": {
    "idle": "idle.gif",    // GIF 动图也可以
    "thinking": "thinking.png"
  }
}
```

---

## 三、详细实施方案

### Phase 1: 前端皮肤加载器（0 新增依赖）

#### 1.1 类型定义

```typescript
// src/types/skin.ts（新增）

/** 皮肤帧映射 */
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

/** 皮肤元数据（对应 skin.json） */
export interface SkinMetadata {
  id: string;
  name: string;
  author?: string;
  version?: string;
  description?: string;
  frame_count?: { width: number; height: number };
  frames: SkinFrames;
  indicators?: Record<string, unknown>;
}

/** 皮肤列表项（来自 Rust IPC） */
export interface SkinInfo {
  id: string;
  name: string;
  description: string;
  image_path: string;
  custom: boolean;
}
```

#### 1.2 皮肤加载 Hook

```typescript
// src/hooks/useSkinLoader.ts（新增）

import { useState, useEffect, useCallback } from "react";
import { SkinMetadata } from "@/types/skin";
import { invoke } from "@tauri-apps/api/core";

export function useSkinLoader(skinId: string) {
  const [skin, setSkin] = useState<SkinMetadata | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadSkin = useCallback(async (id: string) => {
    try {
      // 策略 1: 先尝试从内置皮肤目录加载
      // 通过 Rust IPC 获取内置皮肤列表
      // 策略 2: 再尝试从用户自定义目录加载
      // 调用 invoke("list_skins") 获取完整列表
      const skins: SkinInfo[] = await invoke("list_skins");
      const target = skins.find(s => s.id === id);
      
      if (target) {
        const metadata = await fetchSkinMetadata(target.image_path);
        setSkin(metadata);
      } else {
        setError(`皮肤 "${id}" 不存在`);
      }
    } catch (e) {
      setError(`加载皮肤失败: ${e}`);
    }
  }, []);

  useEffect(() => {
    loadSkin(skinId);
  }, [skinId, loadSkin]);

  return { skin, error, reload: () => loadSkin(skinId) };
}
```

#### 1.3 PetPNG 组件（替换 PetSVG）

```typescript
// src/components/PetPNG.tsx（新增，替代 PetSVG.tsx）

import { useEffect, useState, useCallback } from "react";
import { PetState } from "@agent-pet-hub/protocol";
import { useSkinLoader } from "@/hooks/useSkinLoader";

interface PetPNGProps {
  petState: PetState;
  previousState?: PetState;
  skinId?: string;
}

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

export function PetPNG({ petState, previousState, skinId = "default" }: PetPNGProps) {
  const { skin, error } = useSkinLoader(skinId);
  const [currentFrame, setCurrentFrame] = useState<string>("");
  const [isTransitioning, setIsTransitioning] = useState(false);

  // 状态变化时设置过渡
  useEffect(() => {
    if (previousState && previousState !== petState) {
      setIsTransitioning(true);
      const timer = setTimeout(() => setIsTransitioning(false), 200);
      return () => clearTimeout(timer);
    }
  }, [petState, previousState]);

  // 根据状态切换帧
  useEffect(() => {
    if (skin) {
      const frameFile = skin.frames[petState] || skin.frames.idle;
      // 构建完整 URL（内置或用户路径）
      const frameUrl = buildFrameUrl(skin, frameFile);
      setCurrentFrame(frameUrl);
    }
  }, [skin, petState]);

  const cssClass = `pet-png ${STATE_CLASS_MAP[petState]}${isTransitioning ? " pet-transitioning" : ""}`;

  if (error) {
    return <div className="pet-png-wrapper">{error}</div>;
  }

  if (!skin) {
    // 加载状态：显示默认占位
    return (
      <div className="pet-png-wrapper">
        <div className="pet-loading" />
      </div>
    );
  }

  return (
    <div className="pet-png-wrapper" data-tauri-drag-region>
      <img
        className={cssClass}
        src={currentFrame}
        alt={`Pet ${petState}`}
        draggable={false}
        onError={() => setCurrentFrame("")}
      />

      {/* 状态指示器 — CSS overlay */}
      {petState === "working" && <div className="indicator indicator-working" />}
      {petState === "error" && <div className="indicator indicator-error" />}
      {petState === "success" && <div className="indicator indicator-success" />}
    </div>
  );
}
```

### Phase 2: CSS 改造

#### 2.1 选择器迁移

```css
/* src/index.css 修改 */

/* ── 基础样式 ── */
.pet-png {
  transition: transform 0.3s ease, filter 0.3s ease;
  image-rendering: -webkit-optimize-contrast;
  pointer-events: none;
}

/* ── Idle 状态 ── */
.pet-png.pet-idle {
  animation: pet-breathe 3s ease-in-out infinite;
}

/* ── Thinking 状态 ── */
.pet-png.pet-thinking {
  animation: pet-think 2.5s ease-in-out infinite;
}

/* ── Working 状态 ── */
.pet-png.pet-working {
  animation: pet-work 0.15s ease-in-out infinite;
}

/* ── 其余状态... ── */
.pet-png.pet-waiting { ... }
.pet-png.pet-success { ... }
.pet-png.pet-error { ... }
.pet-png.pet-speaking { ... }
.pet-png.pet-connecting { ... }

/* ── 过渡动画 ── */
.pet-transitioning {
  animation: pet-fade-transition 0.2s ease;
}

@keyframes pet-fade-transition {
  0%   { opacity: 1; }
  50%  { opacity: 0.3; }
  100% { opacity: 1; }
}

/* ── 指示器 overlay ── */
.indicator {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  pointer-events: none;
}

.indicator-working {
  width: 100px;
  height: 100px;
  border: 2px solid #4CAF50;
  border-radius: 50%;
  border-style: dashed;
  animation: ring-spin 4s linear infinite;
}

.indicator-error {
  width: 40px;
  height: 40px;
  animation: error-flash 0.8s ease-in-out infinite;
}
.indicator-error::before,
.indicator-error::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 30px;
  height: 3px;
  background: #F44336;
  border-radius: 2px;
}
.indicator-error::before { transform: translate(-50%, -50%) rotate(45deg); }
.indicator-error::after  { transform: translate(-50%, -50%) rotate(-45deg); }

.indicator-success {
  animation: star-twinkle 0.8s ease-in-out infinite;
  font-size: 24px;
  color: #FFD700;
  text-align: center;
  line-height: 40px;
}
```

### Phase 3: Rust 后端扩展

#### 3.1 新增 IPC 命令

```rust
// src-tauri/src/commands.rs 新增

use tauri::command;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SkinInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub image_path: String,
    pub custom: bool,
}

/// 列出所有可用皮肤（内置 + 用户自定义）
#[command]
pub fn list_skins() -> Result<Vec<SkinInfo>, String> {
    let mut skins = Vec::new();
    
    // 1. 扫描内置皮肤 (src/assets/skins/)
    //    使用 include_dir! 或运行时读取打包后的路径
    scan_builtin_skins(&mut skins);
    
    // 2. 扫描用户自定义皮肤 (~/.config/agent-pet-hub/skins/)
    let user_skins_dir = get_user_skins_dir();
    scan_directory_skins(&user_skins_dir, true, &mut skins);
    
    Ok(skins)
}

/// 获取皮肤元数据
#[command]
pub fn get_skin_metadata(skin_id: String) -> Result<serde_json::Value, String> {
    // 定位 skin.json 并读取
    let path = find_skin_json(&skin_id)?;
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}
```

#### 3.2 皮肤目录扫描

```rust
fn scan_directory_skins(dir: &std::path::Path, custom: bool, skins: &mut Vec<SkinInfo>) {
    if !dir.is_dir() { return; }
    
    for entry in std::fs::read_dir(dir).into_iter().flatten() {
        let entry = entry.unwrap();
        let path = entry.path();
        
        // 跳过隐藏目录和非目录
        if !path.is_dir() || path.file_name().unwrap().to_str().unwrap().starts_with('.') {
            continue;
        }
        
        // 查找 skin.json
        let skin_json_path = path.join("skin.json");
        if !skin_json_path.exists() { continue; }
        
        let content = std::fs::read_to_string(&skin_json_path)
            .unwrap_or_default();
        let metadata: serde_json::Value = serde_json::from_str(&content).ok();
        
        if let Some(meta) = metadata {
            skins.push(SkinInfo {
                id: meta["id"].as_str().unwrap_or("").to_string(),
                name: meta["name"].as_str().unwrap_or("").to_string(),
                description: meta["description"].as_str().unwrap_or("").to_string(),
                image_path: path.to_string_lossy().to_string(),
                custom,
            });
        }
    }
}
```

#### 3.3 修改皮肤切换设置

```rust
// src-tauri/src/commands.rs — update_settings 中新增 skin_id 处理

#[command]
pub fn update_settings(updates: serde_json::Value) -> Result<(), String> {
    let mut manager = get_settings_manager()?;
    
    // 验证 skin_id 是否存在
    if let Some(skin_id) = updates.get("pet")
        .and_then(|p| p.get("skin_id"))
        .and_then(|s| s.as_str()) {
        
        let available = list_skins().map_err(|e| e.to_string())?;
        let exists = available.iter().any(|s| s.id == *skin_id);
        if !exists {
            return Err(format!("皮肤 '{}' 不存在", skin_id));
        }
    }
    
    manager.update(updates)?;
    manager.save()?;
    
    // 通知前端皮肤已切换（触发 UI 更新）
    emit_skin_changed(&manager.get().pet.skin_id)?;
    
    Ok(())
}
```

### Phase 4: 设置 UI

#### 4.1 皮肤选择器组件

```typescript
// src/components/SkinSelector.tsx（新增）

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SkinInfo } from "@/types/skin";

interface SkinSelectorProps {
  currentSkinId: string;
  onSkinChange: (skinId: string) => void;
}

export function SkinSelector({ currentSkinId, onSkinChange }: SkinSelectorProps) {
  const [skins, setSkins] = useState<SkinInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<SkinInfo[]>("list_skins")
      .then(setSkins)
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div>加载皮肤中...</div>;

  return (
    <div className="skin-selector">
      <h3>皮肤选择</h3>
      <div className="skin-grid">
        {skins.map(skin => (
          <button
            key={skin.id}
            className={`skin-option ${skin.id === currentSkinId ? "active" : ""}`}
            onClick={() => onSkinChange(skin.id)}
          >
            {/* 皮肤预览图（可加载 skin.json 中指定帧） */}
            <div className="skin-preview">{skin.name}</div>
            {skin.custom && <span className="skin-badge-custom">自定义</span>}
            {skin.id === currentSkinId && <span className="skin-badge-active">当前</span>}
          </button>
        ))}
      </div>
    </div>
  );
}
```

#### 4.2 系统托盘皮肤切换

```rust
// src-tauri/src/window/tray.rs 扩展
// 托盘菜单新增皮肤切换子菜单
// 支持右键托盘 → 选择皮肤
```

### Phase 5: 默认皮肤迁移

#### 5.1 从 SVG 导出 PNG 帧

```bash
# 使用 Inkscape / SVG2PNG 工具将 SVG 按状态导出为 PNG
# 或使用脚本批量生成（见 Phase 5.2）
```

#### 5.2 自动导出脚本（可选）

```typescript
// scripts/export-skin-pngs.ts（可选工具）
// 使用 canvas 将 SVG 渲染为 PNG
// 按状态生成不同帧（不同表情/姿态）
```

---

## 四、文件变更清单

### 新增文件

| 文件 | 说明 |
|------|------|
| `src/types/skin.ts` | 皮肤类型定义 |
| `src/hooks/useSkinLoader.ts` | 皮肤加载 Hook |
| `src/components/PetPNG.tsx` | PNG 皮肤渲染组件（替代 PetSVG） |
| `src/components/SkinSelector.tsx` | 皮肤选择器设置组件 |
| `src/assets/skins/default/skin.json` | 默认皮肤元数据 |
| `src/assets/skins/default/idle.png` | 默认皮肤各状态 PNG 帧 |
| `src/assets/skins/default/thinking.png` | |
| `src/assets/skins/default/working.png` | |
| `src/assets/skins/default/waiting.png` | |
| `src/assets/skins/default/success.png` | |
| `src/assets/skins/default/error.png` | |
| `src/assets/skins/default/speaking.png` | |
| `src/assets/skins/default/connecting.png` | |
| `src/assets/skins/shark/skin.json` | 示例皮肤 |
| `scripts/export-skin-pngs.ts` | SVG→PNG 导出工具（可选） |

### 修改文件

| 文件 | 变更 |
|------|------|
| `src/components/PetSVG.tsx` | 重命名为 `PetSVGLegacy.tsx` 或标记为废弃 |
| `src/App.tsx` | 引入 `PetPNG` 替代 `PetSVG`；传入 `skinId` prop |
| `src/index.css` | 选择器从 `.pet-puppet` → `.pet-png`；新增 overlay 指示器样式；新增过渡动画 |
| `src-tauri/src/commands.rs` | 新增 `list_skins`, `get_skin_metadata` 命令；`update_settings` 中增加皮肤验证 |
| `src-tauri/src/config/settings.rs` | 无需修改（已有 `skin_id` 字段） |
| `src-tauri/src/window/tray.rs` | 托盘菜单新增皮肤切换入口 |
| `src-tauri/tauri.conf.json` | 在 `bundle` 中添加 `src/assets/skins/**` 资源 |

### 配置文件变更

```json
// tauri.conf.json — bundle 资源
{
  "bundle": {
    "resources": [
      "assets/skins/**"
    ]
  }
}
```

---

## 五、实施顺序与优先级

### Phase 0 — 准备（半天）
- [ ] 将 SVG 默认宠物拆分为 PNG 帧（8 张）
- [ ] 创建 `skin.json` 元数据
- [ ] 确定 PNG 尺寸规范（建议 120×120，与 viewBox 一致）

### Phase 1 — 前端皮肤加载器（1 天）
- [ ] 定义 `SkinMetadata` / `SkinInfo` 类型
- [ ] 实现 `useSkinLoader` Hook
- [ ] 实现 `PetPNG` 组件（基础渲染）
- [ ] 修改 `index.css`（选择器迁移 + overlay 指示器）

### Phase 2 — Rust 后端（半天）
- [ ] 新增 `list_skins` IPC 命令
- [ ] 新增 `get_skin_metadata` IPC 命令
- [ ] 修改 `update_settings` 增加 skin_id 验证
- [ ] 配置 Tauri bundle 资源

### Phase 3 — 集成与设置 UI（1 天）
- [ ] `App.tsx` 接入 `PetPNG` + `skinId`
- [ ] 实现 `SkinSelector` 设置组件
- [ ] 托盘菜单皮肤切换入口
- [ ] 皮肤切换时发送事件通知前端

### Phase 4 — 测试与打磨（半天）
- [ ] 内置皮肤切换测试
- [ ] 用户自定义皮肤测试
- [ ] 状态切换过渡效果测试
- [ ] GIF 动图支持测试
- [ ] CSS 动画效果验证

**预计总工作量：约 4 天**

---

## 六、风险与缓解

### 风险 1: PNG 体积 > SVG

| 指标 | SVG | PNG (120×120) |
|------|-----|---------------|
| 大小 | ~10KB（单文件） | ~8KB × 8 = 64KB |
| 可缩放 | ✅ | ❌（固定尺寸） |
| 动画 | CSS + SVG | CSS 叠加 |

**缓解**：使用 PNG 压缩工具（pngquant）可减小 50-80%。对于 120×120 尺寸，压缩后每帧约 3-5KB，总计 ~30KB。

### 风险 2: 窗口尺寸固定 320×280

当前窗口不可调整大小（`resizable: false`）。如果皮肤尺寸不匹配，会留白。

**缓解**：统一皮肤帧尺寸为 120×120，居中显示，上下左右留白。如未来需要支持自定义尺寸，可改为可调整窗口。

### 风险 3: CSS 动画在 PNG 上效果略有差异

SVG 的 `transform-origin` 是元素中心，PNG 的 transform 也是中心，效果一致。但 SVG 内部元素（如眼睛）的 `transform-origin` 可能不同。

**缓解**：对于需要精细 transform-origin 的动画（如眨眼），可将 SVG 的 `transform-origin` 值转换为像素值，通过 CSS `transform-origin: Xpx Ypx` 设置到 PNG 上。

### 风险 4: 皮肤加载性能

大量 PNG 同时加载可能影响首屏渲染。

**缓解**：只加载当前状态帧，其他帧懒加载；使用 `loading="eager"` 确保首帧快速显示。

---

## 七、未来扩展

### 皮肤商店（远期）
```
/api/skins             ← 皮肤列表 API
/api/skins/:id/download ← 下载皮肤包
```
皮肤包格式：`.ape`（Agent Pet Extension）— ZIP 压缩包，内含 `skin.json` + PNG。

### 皮肤动画扩展
```json
{
  "frames": {
    "idle": {
      "type": "image",
      "src": "idle.png"
    },
    "thinking": {
      "type": "sprite",
      "sheet": "thinking-sprite.png",
      "frames": 4,
      "fps": 12
    }
  }
}
```
支持精灵图动画，减少请求数。

### 皮肤热更新
```
~/.config/agent-pet-hub/skins/
└── my-skin/
    └── skin.json      ← 修改后自动生效（文件监听）
```

---

## 八、调研：现有开源案例参考

以下整理了 7 个与本项目高度相关的开源/商业项目，提取其可复用的设计决策。

### 8.1 Clawd-on-desk

| 项目 | 信息 |
|------|------|
| 仓库 | [rullerzhou-afk/clawd-on-desk](https://github.com/rullerzhou-afk/clawd-on-desk) |
| 技术栈 | Electron + TypeScript + SVG |
| Stars | 4.1k ⭐ |
| 最后更新 | 2026-06-11（活跃维护中） |

**核心架构（来自 [guide-theme-creation.md](https://github.com/rullerzhou-afk/clawd-on-desk/blob/main/docs/guides/guide-theme-creation.md) 和 [state-mapping.md](https://github.com/rullerzhou-afk/clawd-on-desk/blob/main/docs/guides/state-mapping.md)）：**

1. **资源格式**：SVG 为主（`assets/svg/` 目录），也有 GIF 和 PNG 帧动画支持
2. **状态系统**：12 种状态 — `idle`, `thinking`, `typing`, `building`, `juggling`, `celebrating`, `sick`, `sleeping`, `error`, `working`, `mini-working`, `mini-thinking`。每种状态包含 `idle` + `active` 两个子帧
3. **状态映射表**：主题配置中通过 `states` 对象定义映射，支持逻辑状态 → 资源文件的灵活绑定
4. **Mini 模式**：支持迷你尺寸（如 `mini-working`），独立于正常尺寸的资源路径
5. **资源命名规范**：`<pet-name>-<state>.svg`（如 `clawd-thinking.svg`），同一目录存放所有状态文件
6. **状态转换动画**：通过 CSS `@keyframes` 驱动（`@keyframes thinking-anim` 等），不依赖 SVG 内部 SMIL
7. **音频反馈**：`assets/sounds/` 目录，每种状态可绑定触发音效（`idle-sound.mp3` 等）

**可借鉴点：**
- ✅ 每个状态分 `idle`/`active` 两帧的设计（`<state>.svg` 和 `<state>-active.svg`）比单帧更生动
- ✅ Mini 模式独立资源路径的设计
- ✅ 音频反馈目录结构
- ⚠️ 他们使用 Electron（比 Tauri 体积大），但我们可直接复用 SVG 资源格式

---

### 8.2 WindowPet

| 项目 | 信息 |
|------|------|
| 仓库 | [SeakMengs/WindowPet](https://github.com/SeakMengs/WindowPet) |
| 技术栈 | Tauri + React + TypeScript |
| Stars | 621 ⭐ |
| 许可 | MIT |

**核心架构：**

1. **资源格式**：PNG sprite 图片，直接作为 `<img>` 渲染
2. **窗口配置**：`transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`, `resizable: false` — 与本项目完全一致
3. **渲染方式**：PNG 图片 + CSS `position: absolute` 居中，`image-rendering: pixelated`（像素风）
4. **状态管理**：通过 CSS class 切换（如 `.pet-idle`, `.pet-thinking`），配合 CSS `@keyframes` 动画
5. **皮肤热加载**：运行时从 `userData` 目录扫描 `skins/` 子目录，无需重启应用

**可借鉴点：**
- ✅ 完全相同的 Tauri 配置范式（`transparent + alwaysOnTop + skipTaskbar`）
- ✅ `userData/skins/` 目录作为用户皮肤存储位置（比 `.config/skins/` 更符合 Tauri 惯例）
- ✅ 运行时热加载皮肤（文件监听或菜单刷新）
- ✅ `image-rendering: pixelated` 用于像素风保持锐利度

---

### 8.3 CrabNebula Koi-Pond

| 项目 | 信息 |
|------|------|
| 仓库 | [crabnebula-dev/koi-pond](https://github.com/crabnebula-dev/koi-pond) |
| 来源 | [CrabNebula 官方博客](https://crabnebula.dev/blog/building-a-desktop-pet-with-tauri/) |
| 技术栈 | Tauri v2 + SolidJS + TypeScript |
| 日期 | 2024-11-07 |

**核心架构：**

1. **资源格式**：PNG sprite frames（`src/assets/koi-frames/` 目录，每张图一个状态帧）
2. **动画方式**：精灵图帧切换（通过 JS 定时器循环切换 `<img>` 的 `src`），CSS 仅做 transform 缩放
3. **窗口配置**：`visibleOnAllWorkspaces: true`, `transparent: true`, `alwaysOnTop: true`（Tauri v2 特有）
4. **资源加载**：Vite 静态资源导入（`import koiFrame from '@/assets/koi-frames/frame.png'`）
5. **性能**：精灵图动画在 React 中通过 `useEffect` + `setInterval` 实现，不依赖 setState 重渲染

**可借鉴点：**
- ✅ `visibleOnAllWorkspaces: true` — 多桌面场景下宠物可见（macOS/Windows 均支持）
- ✅ Sprite frames 目录命名：`frame_01.png`, `frame_02.png`... 或 `idle-01.png` 序列

---

### 8.4 CodeWalkers

| 项目 | 信息 |
|------|------|
| 来源 | [DEV.to 文章](https://dev.to/rain9/tired-of-boring-ai-assistants-i-built-a-desktop-pet-copilot-that-wanders-around-your-screen-and-52pg) |
| 技术栈 | Tauri V2 + React + TypeScript |
| 日期 | 2026-04-07 |

**三个关键技术决策（来自文章原文）：**

1. **透明窗口点击穿透问题**：macOS 下透明像素自动穿透点击。解决方案：`rgba(255, 255, 255, 0.01)` 极淡背景色 + `requestAnimationFrame` + 像素级 hit-testing
2. **60 FPS 动画不卡顿**：**避免将坐标 (x, y) 放入 React State**，改为直接操作 DOM `transform` via `ref`，配合 `requestAnimationFrame`
3. **像素级碰撞检测**：`canvas.drawImage()` + `getImageData()` 判断透明像素，实现精确点击区域

**可借鉴点：**
- ✅ macOS 点击穿透的 `rgba(255,255,255,0.01)` 解决方案 — 本项目 PetSVG 的 `<div className="pet-svg-wrapper">` 也面临同样问题
- ✅ 动画性能：静态宠物（不漫游）不需要 `requestAnimationFrame`，CSS `@keyframes` 已足够
- ✅ 精确点击区域检测（如果未来要做漫游模式）

---

### 8.5 Shimeji-ee 系列

| 项目 | 信息 |
|------|------|
| 经典版 | [Shimeji-ee](https://kilkakon.com/shimeji/)（Group Finity，New BSD 许可） |
| 现代版 | [Shijima-Qt](https://github.com/pixelomer/Shijima-Qt)（Qt 桌面版，开源） |
| 社区 | [r/shimeji](https://www.reddit.com/r/shimeji/)（活跃社区） |
| 制作规范 | [SPRITES.md](https://github.com/Stuocs/Clover_Shimeji/blob/main/SPRITES.md)（完整命名规范） |

**精灵图命名规范（来自 SPRITES.md）：**

每个角色包含多个 PNG 文件，命名规则为 `<状态>_<方向>_<帧号>.png`：
- `standing.png` — 站立静态
- `standing_L.png` — 站立朝左
- `walking_0.png`, `walking_1.png`… — 行走动画帧（循环）
- `climbing_0.png`, `climbing_1.png`… — 爬墙动画
- `sitting.png` — 坐姿
- `sleeping.png` — 睡眠
- `eating.png` — 进食
- `fighting.png` — 战斗

**可借鉴点：**
- ✅ 帧编号规范：`walking_0.png`, `walking_1.png`… 比语义命名更利于程序化生成
- ✅ 方向后缀：`_L`, `_R` 处理左右朝向（本项目暂不需要，但可作为扩展）

---

### 8.6 eSheep (Desktop Mascot)

| 项目 | 信息 |
|------|------|
| 仓库 | [adrianotiger/desktopPet](https://adrianotiger.github.io/desktopPet/) |
| 协议 | GPL-2.0 |

**核心架构：**

1. **资源格式**：PNG 精灵图序列（`.png` 文件，按文件名排序）
2. **精灵图规范**：每张 PNG 是单个精灵帧，文件按字母序作为帧序列
3. **状态分类**：通过文件命名前缀区分状态（`standing.png`, `walking.png`, `sitting.png`）
4. **精灵表支持**：支持将多帧合并在一张 PNG 中（通过配置文件定义每帧的 offset）
5. **配置文件**：`.pet` 格式配置文件定义动画循环、帧数、速度

**可借鉴点：**
- ✅ 精灵表（sprite sheet）+ offset 定义 — 减少文件数，适合帧数多的状态
- ✅ `.pet` 配置文件格式 — 定义循环顺序和动画速度

---

### 8.7 NotiSprite

| 项目 | 信息 |
|------|------|
| 平台 | Mac App Store（商业） |
| 网站 | [notisprite.com](https://notisprite.com/) |
| 编辑器 | [NotiSprite Studio](https://apps.apple.com/gb/app/notisprite-studio-pet-creator/id6757977855) |

**核心功能：**

1. **资源格式**：PNG 精灵图（通过官方编辑器创建）
2. **动画系统**：支持帧动画、GIF 导入
3. **交互**：宠物对系统通知做出反应（弹出、移动等）
4. **创作工具**：NotiSprite Studio 提供可视化精灵编辑器

**可借鉴点：**
- ✅ 宠物响应系统通知的设计思路 — 未来可将 Agent 状态变更映射为通知动画
- ✅ 官方精灵编辑器 — 可作为社区工具提供

---

## 九、总结

### 方案优势
1. **零新依赖** — 纯 CSS + PNG，不需要动画库
2. **渐进式迁移** — `PetSVG` → `PetPNG`，可共存
3. **类型安全** — Rust 端已有 `PetSkin` struct
4. **配置驱动** — `skin.json` + `config.json` 双配置
5. **用户友好** — 只需 PNG 图片 + JSON 元数据，无需代码
6. **GIF 天然支持** — `<img>` 标签原生支持
7. **CSS 动画复用** — 8 种状态动画只需改选择器

### 与现有架构的契合度
- ✅ `STATE_CLASS_MAP` 映射完全复用
- ✅ `PetState` 枚举完全复用
- ✅ CSS keyframes 动画完全复用
- ✅ `PetSettings.skin_id` 完全复用
- ✅ Tauri IPC 模式完全复用
- ✅ 事件总线 + WebSocket 通知完全复用

### 关键设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 指示器渲染 | CSS overlay | 统一风格，皮肤作者少画图 |
| 资源加载 | 内置=Vite import / 用户=Rust IPC | 兼顾打包和运行时扫描 |
| 皮肤存储 | `~/.config/agent-pet-hub/skins/` | 符合 XDG 规范，Rust 端已有配置基础 |
| PNG 尺寸 | 120×120 | 与现有 SVG viewBox 一致 |
| 过渡动画 | opacity fade 200ms | 简单有效，不干扰 CSS 状态动画 |
