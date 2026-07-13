import { defineConfig } from '@playwright/test';

/**
 * Playwright E2E 配置。
 *
 * 本地运行前提：
 *   1. 安装浏览器：bunx playwright install --with-deps chromium
 *   2. 构建应用：bun run tauri:build
 *   3. 启动 Tauri WebDriver（需安装 tauri-driver）
 *   4. 设置 APP_URL 指向应用窗口
 *
 * CI 中通过 webServer 配置自动启动；本地开发可手动设置 APP_URL。
 *
 * 当前状态：三条主链路均为 test.skip，等待 data-testid 属性和
 * Tauri WebDriver 集成稳定后逐步启用。
 */
export default defineConfig({
  testDir: './tests/e2e',
  timeout: 180_000,
  retries: 0,
  reporter: 'list',
  use: {
    baseURL: process.env['APP_URL'] ?? 'tauri://localhost',
    trace: 'on-first-retry',
    video: 'on-first-retry',
  },
  // webServer 在有 CI 构建产物时启用，本地开发暂不自动启动
  // webServer: {
  //   command: 'tauri-driver',
  //   url: 'http://localhost:4444',
  //   reuseExistingServer: !process.env['CI'],
  // },
});
