# Harubble 应用图标制品规格

面向设计师的图标交付说明。本文档描述图标的设计约束、文件格式要求和交付流程。

---

## 设计概念

红色蝴蝶结主体，搭配青绿色纯色背景，保留参考图里软糯、贴纸感的头像轮廓。

macOS 26 起采用 Apple Liquid Glass 图标风格，图标由多个图层叠加组成，系统会自动施加玻璃质感、光照和阴影效果。

---

## 目录结构

```
src-tauri/icons/
├── AppIcon.icon/           ← Apple Icon Composer 工程包（macOS Liquid Glass 源文件）
│   ├── icon.json           ← 图层配置（材质、透明度、阴影等参数）
│   └── Assets/             ← 各图层的 SVG 源文件
│       ├── 01-background.svg
│       └── 02-bow.svg
├── app-icon.svg            ← 完整合并版 SVG（用于非 Liquid Glass 场景的参考稿）
├── previews/               ← ictool 渲染的各模式预览图
│   ├── AppIcon-macOS-default.png
│   ├── AppIcon-macOS-dark.png
│   ├── AppIcon-macOS-tinted-light.png
│   ├── AppIcon-macOS-tinted-dark.png
│   ├── AppIcon-macOS-clear-light.png
│   └── AppIcon-macOS-clear-dark.png
├── icon.png                ← 512×512 PNG（Linux 桌面 / 通用场景）
├── icon.icns               ← macOS 传统图标包
├── icon.ico                ← Windows 图标包
├── 128x128.png             ← 128×128 PNG
├── 128x128@2x.png          ← 256×256 PNG（Retina 2x）
└── 32x32.png               ← 32×32 PNG
```

---

## 图层规格（Liquid Glass）

macOS Liquid Glass 图标由 **2 个图层** 从底到顶叠加：

| 序号 | 图层名称   | 文件                | 说明                                                           |
| ---- | ---------- | ------------------- | -------------------------------------------------------------- |
| 01   | Background | `01-background.svg` | 背景层。青绿色纯色背景。系统会用 superellipse 蒙版裁切最终形状 |
| 02   | Bow        | `02-bow.svg`        | 主体层。红色蝴蝶结。启用了玻璃材质和投影                       |

### SVG 源文件要求

| 项目     | 要求                                                                      |
| -------- | ------------------------------------------------------------------------- |
| 画布尺寸 | **1024 × 1024 px**（必须）                                                |
| viewBox  | `0 0 1024 1024`                                                           |
| 格式     | SVG 1.1，不使用 SVG 2.0 特性                                              |
| 文本     | **禁止使用文本元素**，所有文字必须转曲（转为路径）                        |
| 字体     | 不得引用任何外部字体                                                      |
| 外部资源 | 不得引用外部图片或链接（所有内容内联）                                    |
| 颜色空间 | sRGB                                                                      |
| 透明度   | 允许使用 opacity，但注意系统会额外施加玻璃透明效果                        |
| 圆角     | **不要在 SVG 内画圆角矩形边框**，系统蒙版会统一处理                       |
| 安全区域 | 主体内容建议保持在中心 **800 × 800** 区域内（四周各留 112px），避免被裁切 |

### 设计注意事项

1. **背景层（01）应铺满整个 1024×1024 画布**，不要自己画圆角或留白边
2. **主体层（02）的主要图形应居中**，利用安全区域确保不被裁切
3. **颜色选择**：Liquid Glass 会让图层呈现半透明玻璃质感，过深的纯色可能显得沉闷，建议使用中等饱和度的色彩
4. **细节粗细**：图标在 Dock 中通常显示为 64–128px，过细的线条（< 2px @1024）在小尺寸下不可见
5. **图层间的视觉关系**：系统会自动为上层图层添加阴影和高光，设计时不需要手动画投影

---

## 传统格式制品规格（Windows / Linux）

除 Liquid Glass 源文件外，还需要导出以下传统格式用于 Windows 和 Linux：

| 文件名           | 尺寸      | 格式        | 用途                                |
| ---------------- | --------- | ----------- | ----------------------------------- |
| `icon.png`       | 512 × 512 | PNG-32 RGBA | Linux 桌面图标、通用场景            |
| `128x128.png`    | 128 × 128 | PNG-32 RGBA | Tauri 默认引用                      |
| `128x128@2x.png` | 256 × 256 | PNG-32 RGBA | Retina 2x 显示                      |
| `32x32.png`      | 32 × 32   | PNG-32 RGBA | 小尺寸场景                          |
| `icon.icns`      | 多尺寸    | Apple ICNS  | macOS 传统图标（Liquid Glass 回退） |
| `icon.ico`       | 多尺寸    | Windows ICO | Windows 任务栏、资源管理器          |

### PNG 导出要求

- 色深：32 位（8-bit RGBA）
- 背景：**透明**（不要加白色或纯色底）
- 抗锯齿：开启
- 压缩：无损 PNG，不要使用有损压缩
- 内容：将各图层合并为一张扁平图，不含玻璃效果（即"所见即所得"的传统图标样式）

### ICO 文件内含尺寸

`icon.ico` 需要包含以下尺寸（从大到小）：

- 256 × 256（PNG 压缩）
- 128 × 128
- 64 × 64
- 48 × 48
- 32 × 32
- 16 × 16

### ICNS 文件内含尺寸

`icon.icns` 需要包含以下尺寸：

- 512 × 512 @2x（即 1024 × 1024 实际像素）
- 512 × 512
- 256 × 256 @2x（即 512 × 512 实际像素）
- 256 × 256
- 128 × 128 @2x（即 256 × 256 实际像素）
- 128 × 128
- 32 × 32 @2x（即 64 × 64 实际像素）
- 32 × 32
- 16 × 16 @2x（即 32 × 32 实际像素）
- 16 × 16

---

## 交付清单

设计师完成图标修改后，请交付以下文件：

### 必须交付

- [ ] `01-background.svg` — 背景层 SVG（1024×1024）
- [ ] `02-bow.svg` — 主体层 SVG（1024×1024）
- [ ] `icon.png` — 512×512 合并版 PNG（透明底）
- [ ] `app-icon.svg` — 完整合并版 SVG（所有图层合并为一个文件）

### 按需交付

- [ ] `icon.icns` — macOS 传统图标包（也可由开发者从 PNG 生成）
- [ ] `icon.ico` — Windows 图标包（也可由开发者从 PNG 生成）
- [ ] `128x128.png`、`128x128@2x.png`、`32x32.png` — 也可由开发者从 512px PNG 缩放生成

> **简化交付**：如果你只交付 SVG 源文件 + 512×512 PNG，开发者可以用工具自动生成其余所有尺寸和格式。

---

## 色彩参考

当前图标使用的主要色彩：

| 用途       | 色值      | 说明             |
| ---------- | --------- | ---------------- |
| 背景       | `#78A4A5` | 青绿色纯色       |
| 蝴蝶结主体 | 红色系    | 与参考图主体呼应 |
| 蝴蝶结包边 | `#FFE39A` | 金色缎带描边     |
| 暗部内腔   | `#7C1124` | 深红色卷边内侧   |

---

## 预览与验证

### 在 Icon Composer 中预览（设计师可选）

1. 用 Xcode 16+ 附带的 **Icon Composer** 打开 `AppIcon.icon` 文件夹
2. 检查以下 6 种渲染模式的视觉效果：
   - **Default** — 标准浅色模式
   - **Dark** — 深色模式
   - **Tinted Light** — 浅色着色模式
   - **Tinted Dark** — 深色着色模式
   - **Clear Light** — 浅色透明模式
   - **Clear Dark** — 深色透明模式

### 命令行渲染预览（开发者操作）

使用 `ictool` 批量渲染预览图到 `previews/` 目录：

```bash
/Applications/Icon\ Composer.app/Contents/Executables/ictool \
  src-tauri/icons/AppIcon.icon \
  --export-image \
  --output-file src-tauri/icons/previews/AppIcon-macOS-default.png \
  --platform macOS \
  --rendition Default \
  --width 1024 --height 1024 --scale 1
```

全部模式渲染：

```bash
# Dark
ictool AppIcon.icon --export-image \
  --output-file previews/AppIcon-macOS-dark.png \
  --platform macOS --rendition Dark --width 1024 --height 1024 --scale 1

# Tinted Light
ictool AppIcon.icon --export-image \
  --output-file previews/AppIcon-macOS-tinted-light.png \
  --platform macOS --rendition TintedLight --width 1024 --height 1024 --scale 1 \
  --tint-color 0.58 --tint-strength 0.75

# Tinted Dark
ictool AppIcon.icon --export-image \
  --output-file previews/AppIcon-macOS-tinted-dark.png \
  --platform macOS --rendition TintedDark --width 1024 --height 1024 --scale 1 \
  --tint-color 0.58 --tint-strength 0.75

# Clear Light
ictool AppIcon.icon --export-image \
  --output-file previews/AppIcon-macOS-clear-light.png \
  --platform macOS --rendition ClearLight --width 1024 --height 1024 --scale 1

# Clear Dark
ictool AppIcon.icon --export-image \
  --output-file previews/AppIcon-macOS-clear-dark.png \
  --platform macOS --rendition ClearDark --width 1024 --height 1024 --scale 1
```

---

## 常见问题

**Q: 我只会用 Figma / Illustrator，不会用 Icon Composer 怎么办？**

只需要交付符合规格的 SVG 文件（1024×1024、无文本、无外部资源）。Icon Composer 的配置和渲染由开发者完成。

**Q: 我需要关心 Liquid Glass 的玻璃效果吗？**

不需要在 SVG 中模拟玻璃效果。系统会自动施加。你只需要确保图形在半透明状态下仍然清晰可辨。

**Q: 背景层需要画圆角吗？**

不需要。背景层应该铺满整个 1024×1024 画布，系统的 superellipse 蒙版会自动裁切出最终的圆角矩形形状。

**Q: 传统 PNG 图标需要带圆角吗？**

macOS 的 ICNS 不需要（系统自动加圆角）。Windows ICO 和 Linux PNG 通常也不加圆角，保持方形透明底即可。

**Q: 最小可辨识尺寸是多少？**

图标最小会以 16×16 显示（Windows 任务栏）。请确保主体轮廓在 32×32 下仍可辨认。过于复杂的细节在小尺寸下会糊成一团。

**Q: SVG 里可以用渐变和滤镜吗？**

渐变（linearGradient / radialGradient）可以使用。SVG 滤镜（如 feGaussianBlur）也可以使用，但要注意 Icon Composer 对复杂滤镜的兼容性——建议在交付前用 Icon Composer 预览确认效果。

**Q: 我修改了图标，怎么看最终效果？**

最快的方式：把修改后的 SVG 放入 `AppIcon.icon/Assets/` 对应位置，然后用 Icon Composer 打开 `AppIcon.icon` 即可实时预览所有模式下的效果。
