import { expect, test, type Page } from '@playwright/test';

const FIXTURE_URL =
  'http://127.0.0.1:1421/tests/e2e/visual/fixtures/theme-settings-collapse-fixture.html';

async function setPackageStateWithoutMovingFocus(
  page: Page,
  testId: 'qa-activate-package' | 'qa-clear-package'
): Promise<void> {
  await page.getByTestId(testId).evaluate((element: HTMLButtonElement) => {
    element.click();
  });
}

test.describe('theme package color editor presence', () => {
  test('collapses package-owned colors and preserves independent controls', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 480, height: 760 });
    await page.goto(FIXTURE_URL);

    const colorPanel = page.getByTestId('theme-color-settings-panel');
    await expect(colorPanel).toBeVisible();
    await expect(page.getByRole('button', { name: 'Auto' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'On' })).toBeVisible();

    await page.getByTestId('qa-activate-package').click();
    await expect(colorPanel).toHaveAttribute('aria-hidden', 'true');
    await expect(colorPanel).toBeHidden();
    await expect(page.getByRole('button', { name: 'Auto' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'On' })).toBeVisible();

    await page.getByTestId('qa-clear-package').click();
    await expect(colorPanel).toBeVisible();

    const overflow = await page.evaluate(() => ({
      documentWidth: document.documentElement.scrollWidth,
      viewportWidth: window.innerWidth,
    }));
    expect(overflow.documentWidth).toBeLessThanOrEqual(overflow.viewportWidth);
  });

  test('closes the preset portal and preserves focus when package colors lock', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 480, height: 760 });
    await page.goto(FIXTURE_URL);

    const colorPanel = page.getByTestId('theme-color-settings-panel');
    const presetTrigger = colorPanel.locator('[data-slot="select-trigger"]');
    const presetContent = page.locator('[data-slot="select-content"]');

    await presetTrigger.click();
    await expect(presetContent).toBeVisible();
    await setPackageStateWithoutMovingFocus(page, 'qa-activate-package');
    await expect(presetContent).toBeHidden();
    await expect(page.getByRole('button', { name: 'Auto' })).toBeFocused();

    await setPackageStateWithoutMovingFocus(page, 'qa-clear-package');
    await expect(colorPanel).toBeVisible();
    await colorPanel.locator('#theme-color-accent').focus();
    await setPackageStateWithoutMovingFocus(page, 'qa-activate-package');
    await expect(page.getByRole('button', { name: 'Auto' })).toBeFocused();
  });

  test('reverses a running collapse without jumping back to zero height', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 480, height: 760 });
    await page.goto(FIXTURE_URL);

    const colorPanel = page.getByTestId('theme-color-settings-panel');
    const expandedBox = await colorPanel.boundingBox();
    expect(expandedBox).not.toBeNull();

    await setPackageStateWithoutMovingFocus(page, 'qa-activate-package');
    await page.waitForTimeout(70);
    const collapseFrame = await colorPanel.boundingBox();
    expect(collapseFrame).not.toBeNull();
    expect(collapseFrame!.height).toBeLessThan(expandedBox!.height);
    expect(collapseFrame!.height).toBeGreaterThan(0);

    await setPackageStateWithoutMovingFocus(page, 'qa-clear-package');
    await page.evaluate(() => new Promise(requestAnimationFrame));
    const reverseFrame = await colorPanel.boundingBox();
    expect(reverseFrame).not.toBeNull();
    expect(reverseFrame!.height).toBeGreaterThan(collapseFrame!.height * 0.75);
    await page.waitForTimeout(50);
    const expandingFrame = await colorPanel.boundingBox();
    expect(expandingFrame).not.toBeNull();
    expect(expandingFrame!.height).toBeGreaterThan(reverseFrame!.height);
    await expect(colorPanel).toBeVisible();
  });
});
