import { defineConfig } from '@playwright/test';

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
});
