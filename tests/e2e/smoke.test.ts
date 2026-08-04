/** Web 模式只验证前端交互；真实音频、下载和系统通知不在 mock 范围内。 */
import { expect, test } from '@playwright/test';

// 本机开发时手动设置 APP_URL，CI 中由启动脚本设定
const APP_URL = process.env['APP_URL'] ?? 'tauri://localhost';

test('设置导航与通知授权由用户显式触发', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('settings-trigger').click();

  const sheet = page.getByTestId('settings-sheet');
  await expect(sheet).toBeVisible();
  await expect(sheet.locator('.settings-section-nav button')).toHaveCount(6);
  await expect(sheet.getByText('尚未请求')).toBeVisible();
  await expect(sheet.getByRole('button', { name: '测试' })).toBeDisabled();
  await expect(
    sheet.locator('.settings-section-nav button').first()
  ).toHaveAttribute('aria-current', 'location');

  await sheet.getByRole('button', { name: '授权' }).click();
  await expect(sheet.getByText('已授权')).toBeVisible();
  await expect(sheet.getByRole('button', { name: '测试' })).toBeEnabled();

  await sheet.getByRole('button', { name: '日志与诊断' }).click();
  await expect(
    sheet.getByRole('button', { name: '日志与诊断' })
  ).toHaveAttribute('aria-current', 'location');
});

test('搜索范围同步语义状态并保持可用点击目标', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '搜索', exact: true }).click();

  await expect(page.locator('main')).toHaveCount(1);
  await expect(page.locator('main h1')).toHaveCount(1);
  await expect(page.getByRole('textbox')).toBeFocused();

  const scopeGroup = page.getByRole('group', { name: '搜索范围：全部' });
  await expect(scopeGroup).toBeVisible();

  const scopeButtons = scopeGroup.getByRole('button');
  await expect(scopeButtons).toHaveCount(3);
  for (const button of await scopeButtons.all()) {
    expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(40);
  }

  await page.getByRole('button', { name: '专辑', exact: true }).click();
  await expect(
    page.getByRole('group', { name: '搜索范围：专辑' })
  ).toBeVisible();
});

test('清空收听历史必须经过共享确认对话框', async ({ page }) => {
  await page.goto('/');

  await page.getByRole('button', { name: '清除历史' }).click();
  const dialog = page.getByRole('alertdialog');
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText('清空收听历史？')).toBeVisible();

  await dialog.getByRole('button', { name: '取消' }).click();
  await expect(page.getByText('E2E Test Song')).toBeVisible();

  await page.getByRole('button', { name: '清除历史' }).click();
  await page
    .getByRole('alertdialog')
    .getByRole('button', { name: '清空历史' })
    .click();
  await expect(page.getByText('E2E Test Song')).toHaveCount(0);
});

test('侧栏 separator 支持键盘调整并暴露当前宽度', async ({ page }) => {
  await page.goto('/');
  const separator = page.getByRole('separator', { name: '调整侧栏宽度' });

  await separator.focus();
  await page.keyboard.press('Home');
  await expect(separator).toHaveAttribute('aria-valuenow', '56');
  await page.keyboard.press('End');
  await expect(separator).toHaveAttribute('aria-valuenow', '248');
  expect((await separator.boundingBox())?.width).toBeGreaterThanOrEqual(40);
});

test('空合集折叠按钮与新建操作保持独立语义', async ({ page }) => {
  await page.goto('/');

  await expect(
    page.getByRole('button', { name: '自定义合集', exact: true })
  ).toBeDisabled();

  const create = page.getByRole('button', { name: '新建合集' });
  await expect(create).toBeEnabled();
  expect(
    await create.evaluate((element) =>
      Boolean(element.closest('[aria-disabled="true"]'))
    )
  ).toBe(false);
});

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
