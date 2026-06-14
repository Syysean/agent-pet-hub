# Agent-Pet-Hub 默认皮肤

## 描述

这是 Agent-Pet-Hub 的 MVP 默认皮肤，使用简单的卡通角色设计，通过 SVG + CSS 实现状态动画。

## 动画状态

| 状态 | 类名 | 动画效果 | 周期 | 说明 |
|------|------|---------|------|------|
| 空闲 | `.pet-idle` | 轻微呼吸缩放 | 3s 循环 | 模拟生命感的微小起伏 |
| 思考 | `.pet-thinking` | 歪头 + 眨眼 | 2s 循环 | 思考时的小动作 |
| 工作中 | `.pet-working` | 快速震动 | 200ms 循环 | 模拟敲代码的动感 |
| 等待 | `.pet-waiting` | 缓慢摆动 | 4s 循环 | 耐心等待的样子 |
| 成功 | `.pet-success` | 庆祝弹跳 | 1s 单次 | 完成任务时的庆祝 |
| 错误 | `.pet-error` | 摇头 + 红色 | 800ms 循环 | 出错误时的反应 |
| 说话 | `.pet-speaking` | 嘴部张合 | 150ms 循环 | TTS 语音播报时 |
| 连接中 | `.pet-connecting` | 旋转加载环 | 1s 循环 | 初始化连接时 |

## 文件结构

```
default/
├── index.json      # 皮肤元数据
├── pet.svg         # 宠物 SVG 图形（含 SMIL 内联动画）
├── css/
│   ├── idle.css        # 空闲状态
│   ├── thinking.css    # 思考状态
│   ├── working.css     # 工作中状态
│   ├── waiting.css     # 等待状态
│   ├── success.css     # 成功状态
│   └── error.css       # 错误状态
└── README.md         # 本文件
```

## 如何自定义

### 1. 修改颜色

编辑 `pet.svg`，修改以下元素的颜色：

```svg
<!-- 身体颜色 -->
<circle cx="60" cy="42" r="26" fill="#FFB6C1" />

<!-- 腮红颜色 -->
<ellipse cx="38" cy="50" rx="6" ry="4" fill="url(#blush)" />
```

### 2. 修改动画

编辑对应状态的 CSS 文件，修改 `@keyframes` 中的数值。

例如修改 idle 呼吸速度：

```css
/* 将 3s 改为其他值 */
@keyframes pet-breathe {
  0%   { transform: scale(1); }
  50%  { transform: scale(1.02); }
  100% { transform: scale(1); }
}
```

### 3. 创建新皮肤

复制 `default/` 目录并重命名，修改 `index.json` 中的元数据，然后更新动画。

### 4. SMIL 动画

`pet.svg` 中的 `<animate>` 和 `<animateTransform>` 标签用于简单的内联动画（如眨眼、脉冲）。
这些动画可以通过 CSS 控制 `begin` 属性来触发。

## 技术说明

- **SVG**: viewBox `0 0 120 120`，可直接在浏览器中打开预览
- **CSS**: 使用标准 CSS 属性（`transform: translate`, `rotate`, `scale`），跨平台兼容
- **SMIL**: 用于 SVG 内联动画（眨眼、脉冲、旋转加载环）
- **编码**: 所有文件使用 UTF-8 编码
