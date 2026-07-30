# Visual Regression Tests

Playwright 视觉回归测试目录。用于 Phase 0 主题迁移的像素级零差异保障。

## 运行

```bash
# 1. 安装浏览器（首次）
bunx playwright install --with-deps chromium

# 2. 拉起测试用 Vite 服务
bun run dev:web  # 启动 http://localhost:1421

# 3. 生成基线快照（首次）
bun run test:e2e:visual -- --update-snapshots

# 4. 后续回归运行
bun run test:e2e:visual
```

## 覆盖范围

- `theme-tokens.spec.ts`：CSS 变量 → DOM 渲染的像素级基线，覆盖 `deriveGlobalTokensFromSlots` 派生输出。
  - 不涉及 Tauri IPC 交互
  - 不依赖真实业务数据
  - 独立 HTML 测试夹具（`fixtures/theme-fixture.html`）
- `ark-ui-builtins.spec.ts`：五套内置 family 的真实 `src/app.css` 壳层回归，覆盖桌面与窄屏、运行时错误和水平溢出。

## 三层承诺

- **L1（Phase 0 完成后）**：像素零差异 - 3 个内置 preset 与旧硬编码路径完全一致。
- **L2（Phase 2 完成后）**：pixel-diff < 3% - token 化后仍视觉一致。
- **L3（Phase 3.2）**：人眼可辨可接受 - Material family。

## 快照更新策略

- 仅当明确期望视觉变化（新 feature / 设计调整）时使用 `--update-snapshots`
- 视觉回归失败先修 token 映射或派生系数，不改回硬编码
