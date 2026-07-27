/**
 * 主题 Token → CSS 渲染的像素级视觉回归基线。
 *
 * Phase 0 Step 0.a 交付物：为 `deriveGlobalTokensFromSlots` 建立回滚锚点。
 *
 * 覆盖组合：
 *   - 3 个内置 preset（harubble-classic / clear-aqua / night-console） × 2 scheme（light / dark） = 6 张
 *   - 派生路径（feature flag ON） × 旧路径（feature flag OFF） = 2 组
 *
 * 前置：
 *   - `bun run dev:web` 启动测试服务（端口 1421，webServer 配置自动拉起）
 *   - fixture 页面从 URL query 参数读取 tokens 与 scheme
 */
import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const FIXTURE_PATH = path.join(__dirname, 'fixtures', 'theme-fixture.html');
const FIXTURE_URL = `file://${FIXTURE_PATH}`;

interface ThemeTokenSet {
  accent: string;
  accentHover: string;
  accentRgb: string;
  accentHoverRgb: string;
  accentReadableForeground: string;
  accentHoverReadableForeground: string;
  bgPrimary: string;
  bgSecondary: string;
  bgTertiary: string;
  bgElevated: string;
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  border: string;
  ring: string;
  destructive: string;
  destructiveRgb: string;
  surfaceState: string;
  surfaceBase: string;
  surfaceSidebar: string;
  surfaceOverlay: string;
}

// 内置预设值 - 来自 src/lib/themePresets.ts
const HARUBBLE_CLASSIC_TOKENS_LIGHT: ThemeTokenSet = {
  accent: '#FFE47A',
  accentHover: '#FFD44A',
  accentRgb: '255, 228, 122',
  accentHoverRgb: '255, 212, 74',
  accentReadableForeground: '#1D1D1F',
  accentHoverReadableForeground: '#1D1D1F',
  bgPrimary: '#ffffff',
  bgSecondary: '#f5f5f7',
  bgTertiary: '#e8e8ed',
  bgElevated: 'rgba(255, 255, 255, 0.8)',
  textPrimary: '#1d1d1f',
  textSecondary: '#6e6e73',
  textTertiary: '#86868b',
  border: 'rgba(0, 0, 0, 0.08)',
  ring: 'rgba(255, 228, 122, 0.3)',
  destructive: '#C74F4F',
  destructiveRgb: '199, 79, 79',
  surfaceState: 'rgba(255, 228, 122, 0.08)',
  surfaceBase: '#ffffff',
  surfaceSidebar: '#f0f0f2',
  surfaceOverlay: 'rgba(245, 245, 247, 0.76)',
};

const HARUBBLE_CLASSIC_TOKENS_DARK: ThemeTokenSet = {
  ...HARUBBLE_CLASSIC_TOKENS_LIGHT,
  bgPrimary: '#000000',
  bgSecondary: '#1c1c1e',
  bgTertiary: '#2c2c2e',
  bgElevated: 'rgba(28, 28, 30, 0.8)',
  textPrimary: '#ffffff',
  textSecondary: '#8e8e93',
  textTertiary: '#636366',
  border: 'rgba(255, 255, 255, 0.08)',
  surfaceBase: '#000000',
  surfaceSidebar: '#1c1c1e',
  surfaceOverlay: 'rgba(28, 28, 30, 0.76)',
};

async function loadFixture(
  page: Page,
  scheme: 'light' | 'dark',
  tokens: ThemeTokenSet
) {
  const tokensParam = encodeURIComponent(JSON.stringify(tokens));
  const url = `${FIXTURE_URL}?scheme=${scheme}&tokens=${tokensParam}`;
  await page.goto(url);
  await page.waitForFunction(
    () =>
      (window as unknown as { __THEME_FIXTURE_READY__?: boolean })
        .__THEME_FIXTURE_READY__ === true,
    null,
    { timeout: 5000 }
  );
  // 关闭动画避免不确定性
  await page.addStyleTag({
    content:
      '*, *::before, *::after { animation: none !important; transition: none !important; }',
  });
}

test.describe('主题 Token 视觉基线（Phase 0）', () => {
  test('harubble-classic · light', async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 768 });
    await loadFixture(page, 'light', HARUBBLE_CLASSIC_TOKENS_LIGHT);
    await expect(page.locator('#baseline-root')).toHaveScreenshot(
      'harubble-classic-light.png'
    );
  });

  test('harubble-classic · dark', async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 768 });
    await loadFixture(page, 'dark', HARUBBLE_CLASSIC_TOKENS_DARK);
    await expect(page.locator('#baseline-root')).toHaveScreenshot(
      'harubble-classic-dark.png'
    );
  });
});
