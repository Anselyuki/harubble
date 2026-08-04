import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E 配置。
 *
 * 两种测试目录：
 *   - `tests/e2e/smoke.test.ts`：使用 Tauri Web mock 的浏览器交互冒烟测试
 *   - `tests/e2e/visual/`：视觉回归测试（Phase 0 主题迁移基线保护）
 *
 * 视觉回归运行前提：
 *   1. 安装浏览器：bunx playwright install --with-deps chromium
 *   2. 直接运行：bun run test:e2e:visual（webServer 自动拉起）
 *   3. 更新基线：bun run test:e2e:visual -- --update-snapshots
 *
 * 真正的音频、下载和系统集成仍应通过 Tauri WebDriver 单独验证。
 */
export default defineConfig({
  testDir: './tests/e2e',
  timeout: 180_000,
  retries: process.env['CI'] ? 2 : 0,
  reporter: 'list',
  use: {
    baseURL: process.env['APP_URL'] ?? 'http://127.0.0.1:1421',
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
