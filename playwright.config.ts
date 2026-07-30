import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E 配置。
 *
 * 两种测试目录：
 *   - `tests/e2e/smoke.test.ts`：Tauri 全链路冒烟测试（当前 test.skip，需 WebDriver 集成）
 *   - `tests/e2e/visual/`：视觉回归测试（Phase 0 主题迁移基线保护）
 *
 * 视觉回归运行前提：
 *   1. 安装浏览器：bunx playwright install --with-deps chromium
 *   2. 直接运行：bun run test:e2e:visual（webServer 自动拉起）
 *   3. 更新基线：bun run test:e2e:visual -- --update-snapshots
 *
 * Tauri 冒烟测试运行前提：
 *   1. 构建应用：bun run tauri:build
 *   2. 启动 Tauri WebDriver（需安装 tauri-driver）
 *   3. 设置 APP_URL 指向应用窗口
 */
export default defineConfig({
  testDir: './tests/e2e',
  timeout: 180_000,
  retries: process.env['CI'] ? 2 : 0,
  reporter: 'list',
  use: {
    baseURL: process.env['APP_URL'] ?? 'tauri://localhost',
    trace: 'on-first-retry',
    video: 'on-first-retry',
  },
  projects: [
    {
      name: 'visual',
      testMatch: /visual\/.*\.spec\.ts$/,
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1024, height: 768 },
      },
    },
    {
      name: 'smoke',
      testMatch: /smoke\.test\.ts$/,
    },
  ],
  webServer: {
    command: 'bun run dev:web -- --host 127.0.0.1',
    url: 'http://127.0.0.1:1421',
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
