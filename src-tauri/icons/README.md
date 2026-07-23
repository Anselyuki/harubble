# Harubble 应用图标

本目录同时保存 macOS Icon Composer 源文件、跨平台扁平源稿和 Tauri 打包制品。目录中的实际源文件与 `src-tauri/tauri.conf.json` 是事实来源。

## 当前源文件

```text
src-tauri/icons/
├── AppIcon.icon/
│   ├── icon.json
│   └── Assets/02-bow.svg
├── app-icon.svg
├── previews/
├── icon.png
├── icon.icns
├── icon.ico
├── 128x128.png
├── 128x128@2x.png
├── 32x32.png
└── tray-32x32.png
```

- `AppIcon.icon` 是 macOS Liquid Glass 源工程。当前只有 `02-bow.svg` 一个视觉图层；背景由 Icon Composer 的 `fill: automatic` 处理，不存在独立的 `01-background.svg`。
- `app-icon.svg` 是传统制品的扁平源稿，包含青绿色背景与完整蝴蝶结。
- `tray-32x32.png` 是 macOS 菜单栏使用的透明蝴蝶结图层；它必须保持透明背景，才能配合 template 图标正确显示。
- `previews/` 保存 Icon Composer 的 Default、Dark、Tinted 和 Clear 模式预览，不参与打包。
- Tauri 实际打包输入以 `tauri.conf.json` 的 `bundle.icon` 数组为准。

## SVG 约束

| 项目     | 要求                                   |
| -------- | -------------------------------------- |
| 画布     | 1024 x 1024，`viewBox="0 0 1024 1024"` |
| 文本     | 不使用文本元素；需要文字时先转为路径   |
| 外部资源 | 不引用外部图片、字体或链接             |
| 颜色空间 | sRGB                                   |
| 安全区域 | 主体尽量保持在中心 800 x 800 区域      |

`02-bow.svg` 保持透明背景，只承载 Icon Composer 图层内容；`app-icon.svg` 才包含传统图标使用的背景和完整合成效果。

## 打包制品

| 文件             | 用途                       |
| ---------------- | -------------------------- |
| `32x32.png`      | 小尺寸桌面图标             |
| `128x128.png`    | Tauri 通用 PNG             |
| `128x128@2x.png` | Retina PNG                 |
| `icon.icns`      | macOS 传统图标包           |
| `icon.ico`       | Windows 多尺寸图标包       |
| `icon.png`       | 512 x 512 通用预览与生成源 |

PNG 使用 8-bit RGBA 和无损压缩；ICO 至少覆盖 16、32、48、64、128、256 像素。修改源稿后必须同步重新生成相关制品，避免各平台图标不一致。

## 验证

用 Icon Composer 打开 `AppIcon.icon`，检查 Default、Dark、Tinted Light/Dark 与 Clear Light/Dark 六种模式。命令行可生成单个预览：

```bash
/Applications/Icon\ Composer.app/Contents/Executables/ictool \
  src-tauri/icons/AppIcon.icon \
  --export-image \
  --output-file src-tauri/icons/previews/AppIcon-macOS-default.png \
  --platform macOS \
  --rendition Default \
  --width 1024 --height 1024 --scale 1
```

检查 PNG 尺寸与透明通道：

```bash
sips -g pixelWidth -g pixelHeight -g hasAlpha \
  src-tauri/icons/icon.png \
  src-tauri/icons/128x128.png \
  src-tauri/icons/32x32.png
```

最终运行 `bun run tauri:build` 验证当前平台；跨平台制品继续由 CI 的 macOS/Windows build-check 与发布矩阵验证。
