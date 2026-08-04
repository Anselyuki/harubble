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

- `theme-tokens.spec.ts`：CSS 变量到 DOM 渲染的 Phase 0 像素基线。
- `ark-ui-builtins.spec.ts`：五套内置 family 的真实壳层、深浅模式、命中区和溢出回归。
- `endfield-*.spec.ts`：Endfield 专辑操作、播放器 Dock 和主题设置的对比度、响应式与状态回归。
- `theme-package-transition.spec.ts` / `theme-settings-collapse.spec.ts`：主题切换提交时机、竞态、reduced motion、设置折叠和焦点保持。
- `settings-preview-backdrop.spec.ts`：设置预览背景、关闭时序、滚动和输入屏蔽合同。
- `volume-capsule-families.spec.ts`：五个 Ark UI family 的独立视觉、共享交互、键盘/粗指针/reduced motion 与切换连续性。

视觉 fixture 不替代真实 Tauri 集成验证；音频、下载、系统菜单和持久化链路仍需 Rust 测试或桌面端验证。

## 三层承诺

- **L1（Phase 0 完成后）**：像素零差异 - 3 个内置 preset 与旧硬编码路径完全一致。
- **L2（Phase 2 完成后）**：pixel-diff < 3% - token 化后仍视觉一致。
- **L3（Phase 3+）**：不同 family 允许有意的视觉差异，但共享交互、可读性和命中区合同必须保持一致。

## 快照更新策略

- 仅当明确期望视觉变化（新 feature / 设计调整）时使用 `--update-snapshots`
- 视觉回归失败先修 token 映射或派生系数，不改回硬编码
