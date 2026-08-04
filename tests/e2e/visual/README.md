# Visual Regression Tests

Playwright 视觉回归测试目录。除 Phase 0 token 基线外，也覆盖内置视觉族、设置页、主题切换、播放器 Dock 与音量/音质交互合同。

## 运行

```bash
# 1. 安装浏览器（首次）
bunx playwright install --with-deps chromium

# 2. 生成基线快照（首次）
bun run test:e2e:visual -- --update-snapshots

# 3. 后续回归运行（Playwright 自动管理 127.0.0.1:1421）
bun run test:e2e:visual
```

## 覆盖范围

- `theme-tokens.spec.ts`：Harubble Classic light / dark 两张 CSS token 到 DOM 渲染基线。
- `ark-ui-builtins.spec.ts`：五套内置 family 的真实壳层、深浅模式、命中区和溢出回归。
- `endfield-*.spec.ts`：Endfield 专辑操作、播放器 Dock 和主题设置的对比度、响应式与状态回归。
- `theme-package-transition.spec.ts` / `theme-settings-collapse.spec.ts`：主题切换提交时机、竞态、reduced motion、设置折叠和焦点保持。
- `settings-preview-backdrop.spec.ts`：设置预览背景、关闭时序、滚动和输入屏蔽合同。
- `volume-capsule-families.spec.ts`：五个 Ark UI family 的独立视觉、共享交互、键盘/粗指针/reduced motion 与切换连续性。

视觉 fixture 不替代真实 Tauri 集成验证；音频、下载、系统菜单和持久化链路仍需 Rust 测试或桌面端验证。

## 断言层级

- `theme-tokens.spec.ts` 使用 Playwright 默认截图比较，锁定 Classic light / dark 当前渲染结果；它不代表 3 个旧 preset 或新旧两条运行时路径的全量等价证明。
- family、设置与 Endfield 套件按各自 fixture 锁定期望视觉；多数截图沿用默认比较，音量胶囊整页允许 `maxDiffPixelRatio: 0.001`，音质读数仍要求 `0`。
- 不同 family 允许有意的视觉差异，但共享交互、可读性、焦点和命中区合同必须保持一致；这些行为同时由 DOM/样式断言覆盖，不只依赖截图。

当前仓库中的基线文件均为 `darwin` 快照。其他平台运行截图断言前需要生成并评审对应平台基线，不能把缺少基线误判为产品视觉回归。

## 快照更新策略

- 仅当明确期望视觉变化（新 feature / 设计调整）时使用 `--update-snapshots`
- 视觉回归失败先修 token 映射或派生系数，不改回硬编码
