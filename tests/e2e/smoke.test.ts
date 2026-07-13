/**
 * 最小冒烟测试 — 三条主链路存根
 *
 * 这些测试在启动真实 Tauri 应用进程后运行。
 * 当前为 STUB：跳过（skip）以允许 CI 在未配置 WebDriver 环境时通过；
 * 补全执行条件后移除 test.skip。
 *
 * 运行前置：bun tauri:build && bun run test:e2e
 */
import { expect, test } from '@playwright/test';

// 本机开发时手动设置 APP_URL，CI 中由启动脚本设定
const APP_URL = process.env['APP_URL'] ?? 'tauri://localhost';

test.skip('链路一：搜索 → 选择结果 → 启动播放', async ({ page }) => {
  await page.goto(APP_URL);
  // 等待库加载完成
  await page.waitForSelector('[data-testid="library-ready"]', {
    timeout: 30_000,
  });
  // 在搜索框输入查询
  await page.fill('[data-testid="search-input"]', '孤独的序曲');
  // 等待搜索结果
  await page.waitForSelector('[data-testid="search-result-item"]', {
    timeout: 10_000,
  });
  // 点击第一个结果
  await page.click('[data-testid="search-result-item"]:first-child');
  // 验证播放器启动
  await expect(
    page.locator('[data-testid="player-current-song"]')
  ).not.toBeEmpty();
});

test.skip('链路二：下载 → 进度事件 → 本地库更新', async ({ page }) => {
  await page.goto(APP_URL);
  await page.waitForSelector('[data-testid="library-ready"]', {
    timeout: 30_000,
  });
  // 打开某个专辑
  await page.click('[data-testid="album-card"]:first-child');
  // 点击下载按钮
  await page.click('[data-testid="download-album-button"]');
  // 验证下载任务出现在任务面板
  await page.click('[data-testid="downloads-panel-trigger"]');
  await expect(page.locator('[data-testid="download-job-item"]')).toBeVisible();
  // 等待任务完成（最多 120s）
  await expect(
    page.locator('[data-testid="download-job-status-completed"]')
  ).toBeVisible({ timeout: 120_000 });
  // 验证本地库存徽标更新
  await page.click('[data-testid="album-card"]:first-child');
  await expect(
    page.locator('[data-testid="download-badge-verified"]')
  ).toBeVisible();
});

test.skip('链路三：Tag 编辑 → 保存 → 搜索索引更新', async ({ page }) => {
  await page.goto(APP_URL);
  await page.waitForSelector('[data-testid="library-ready"]', {
    timeout: 30_000,
  });
  // 进入 Tag 编辑器
  await page.click('[data-testid="tag-editor-nav"]');
  await page.waitForSelector('[data-testid="tag-editor-ready"]', {
    timeout: 10_000,
  });
  // 选择一个专辑并编辑 tag
  await page.click('[data-testid="tag-editor-album-item"]:first-child');
  await page.click('[data-testid="tag-add-value-button"]');
  await page.fill('[data-testid="tag-value-input"]', 'e2e-test-tag');
  await page.keyboard.press('Enter');
  // 验证搜索能召回
  await page.click('[data-testid="nav-library"]');
  await page.fill('[data-testid="search-input"]', 'e2e-test-tag');
  await expect(page.locator('[data-testid="search-result-item"]')).toBeVisible({
    timeout: 15_000,
  });
});
