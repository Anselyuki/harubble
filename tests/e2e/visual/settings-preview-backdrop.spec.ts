import { expect, test, type Locator, type Page } from '@playwright/test';

const FIXTURE_URL =
  'http://127.0.0.1:1421/tests/e2e/visual/fixtures/settings-preview-backdrop-fixture.html';

const VIEWPORTS = [
  { name: 'desktop', width: 1100, height: 760 },
  { name: 'tall', width: 1120, height: 1300 },
  { name: 'narrow', width: 390, height: 760 },
] as const;

function isExpandedClipPath(value: string): boolean {
  return (
    value === 'inset(0px)' ||
    value === 'inset(0px 0px)' ||
    value === 'inset(0px 0px 0px)' ||
    value === 'inset(0px 0px 0px 0px)'
  );
}

function isRetractedClipPath(value: string): boolean {
  return value === 'inset(0px 100% 0px 0px)';
}

async function openFixture(
  page: Page,
  options: { reducedMotion?: boolean; samePackage?: boolean } = {}
): Promise<void> {
  await page.emulateMedia({
    reducedMotion: options.reducedMotion ? 'reduce' : 'no-preference',
  });
  await page.goto(
    `${FIXTURE_URL}${options.samePackage ? '?same-package=1' : ''}`
  );
  await page.waitForFunction(
    () =>
      (
        window as typeof window & {
          __SETTINGS_PREVIEW_BACKDROP_FIXTURE_READY__?: boolean;
        }
      ).__SETTINGS_PREVIEW_BACKDROP_FIXTURE_READY__ === true
  );
  await expect(page.locator('[data-slot="sheet-content"]')).toBeVisible();
  await page.waitForTimeout(options.reducedMotion ? 50 : 650);
}

test('active package never exposes preview-only settings controls', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await openFixture(page, { samePackage: true });

  const fixture = page.getByTestId('settings-preview-backdrop-fixture');
  const activeItem = page.locator(
    '[data-testid="theme-package-item"][data-package-id="ark-ui-endfield"]'
  );

  await expect(fixture).toHaveAttribute(
    'data-active-package-id',
    'ark-ui-endfield'
  );
  await expect(fixture).toHaveAttribute(
    'data-previewing-id',
    'ark-ui-endfield'
  );
  await expect(page.getByTestId('theme-preview-backdrop-toggle')).toHaveCount(
    0
  );
  await expect(page.getByTestId('theme-package-dismiss-preview')).toHaveCount(
    0
  );
  await expect(
    activeItem.getByTestId('theme-package-preview-badge')
  ).toHaveCount(0);
  await expect(
    activeItem.getByRole('button', { name: '预览', exact: true })
  ).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByTestId('settings-preview-backdrop')).toHaveAttribute(
    'data-previewing',
    'false'
  );
});

async function readClipPath(backdrop: Locator): Promise<string> {
  return backdrop.evaluate((element) => getComputedStyle(element).clipPath);
}

async function expectExpanded(backdrop: Locator): Promise<void> {
  await expect
    .poll(async () => isExpandedClipPath(await readClipPath(backdrop)))
    .toBe(true);
}

async function expectRetracted(backdrop: Locator): Promise<void> {
  await expect
    .poll(async () => isRetractedClipPath(await readClipPath(backdrop)))
    .toBe(true);
}

async function expectFocusInsideDialog(page: Page): Promise<void> {
  await expect
    .poll(() =>
      page.evaluate(() => {
        const dialog = document.querySelector('[role="dialog"]');
        return Boolean(dialog?.contains(document.activeElement));
      })
    )
    .toBe(true);
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth
      )
    )
    .toBe(true);
}

for (const viewport of VIEWPORTS) {
  test(`settings preview backdrop retracts and resets at ${viewport.name} width`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await openFixture(page);

    const fixture = page.getByTestId('settings-preview-backdrop-fixture');
    const dialog = page.locator('[data-slot="sheet-content"]');
    const toggle = page.getByTestId('theme-preview-backdrop-toggle');
    const backdrop = page.getByTestId('settings-preview-backdrop');

    await expect(fixture).toHaveAttribute(
      'data-active-package-id',
      'ark-ui-endfield'
    );
    await expect(fixture).toHaveAttribute('data-previewing-id', 'ark-ui-ark');
    await expect(
      page.locator(
        '[data-testid="theme-package-item"][data-package-id="ark-ui-endfield"]'
      )
    ).toHaveAttribute('data-active', 'true');
    await expect(
      page.locator(
        '[data-testid="theme-package-item"][data-package-id="ark-ui-ark"]'
      )
    ).toHaveAttribute('data-previewing', 'true');

    await expect(toggle).toBeVisible();
    await expect(toggle).toHaveAttribute('aria-pressed', 'false');
    await expect(backdrop).toBeVisible();
    await expectExpanded(backdrop);
    await expectFocusInsideDialog(page);
    await expectNoHorizontalOverflow(page);
    await expect(page).toHaveScreenshot(
      `settings-preview-backdrop-${viewport.name}-expanded.png`,
      { animations: 'disabled' }
    );

    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-pressed', 'true');
    await expectRetracted(backdrop);
    await expect(toggle).toBeFocused();
    await expectFocusInsideDialog(page);
    await expectNoHorizontalOverflow(page);
    await expect(page).toHaveScreenshot(
      `settings-preview-backdrop-${viewport.name}-retracted.png`,
      { animations: 'disabled' }
    );

    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-pressed', 'false');
    await expectExpanded(backdrop);
    await expect(toggle).toBeFocused();

    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-pressed', 'true');
    await dialog.locator('[data-slot="sheet-close"]').click();
    await expect(dialog).toBeHidden();

    await page.getByTestId('qa-open-settings').click();
    await expect(dialog).toBeVisible();
    await expect(toggle).toBeVisible();
    await expect(toggle).toHaveAttribute('aria-pressed', 'false');
    await expectExpanded(backdrop);
    await expectFocusInsideDialog(page);
    await expectNoHorizontalOverflow(page);
  });
}

test('settings preview backdrop settles immediately with reduced motion', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 760 });
  await openFixture(page, { reducedMotion: true });

  const toggle = page.getByTestId('theme-preview-backdrop-toggle');
  const backdrop = page.getByTestId('settings-preview-backdrop');

  await expectExpanded(backdrop);
  await toggle.click();
  await page.evaluate(() => new Promise(requestAnimationFrame));
  expect(isRetractedClipPath(await readClipPath(backdrop))).toBe(true);
  expect(
    await backdrop.evaluate(
      (element) =>
        element
          .getAnimations()
          .filter((animation) => animation.playState === 'running').length
    )
  ).toBe(0);
  await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  await expect(toggle).toBeFocused();
  await expectFocusInsideDialog(page);
  await expectNoHorizontalOverflow(page);
});

test('settings preview backdrop restores when preview exits', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await openFixture(page);

  const fixture = page.getByTestId('settings-preview-backdrop-fixture');
  const toggle = page.getByTestId('theme-preview-backdrop-toggle');
  const backdrop = page.getByTestId('settings-preview-backdrop');

  await toggle.click();
  await expectRetracted(backdrop);
  await page.getByTestId('theme-package-dismiss-preview').click();

  await expect(fixture).toHaveAttribute('data-previewing-id', '');
  await expect(toggle).toHaveCount(0);
  await expect(backdrop).toHaveAttribute('data-retracted', 'false');
  await expectExpanded(backdrop);
  await expect(
    page.locator(
      '[data-package-id="ark-ui-ark"] [data-theme-package-action="preview"]'
    )
  ).toBeFocused();
  await expectFocusInsideDialog(page);
});

test('closing sheet releases pointer input before its exit animation ends', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await openFixture(page);

  const fixture = page.getByTestId('settings-preview-backdrop-fixture');
  const dialog = page.locator('[data-slot="sheet-content"]');
  const overlay = page.locator('[data-slot="sheet-overlay"]');
  const openButton = page.getByTestId('qa-open-settings');

  await dialog.locator('[data-slot="sheet-close"]').click();
  expect(await fixture.getAttribute('data-settings-open')).toBe('false');
  expect(await dialog.count()).toBe(1);
  expect(await overlay.count()).toBe(1);
  expect(
    await dialog.evaluate((element) => getComputedStyle(element).pointerEvents)
  ).toBe('none');
  expect(
    await overlay.evaluate((element) => getComputedStyle(element).pointerEvents)
  ).toBe('none');

  const buttonBox = await openButton.boundingBox();
  expect(buttonBox).not.toBeNull();
  const hitTarget = await openButton.evaluate((button) => {
    const rect = button.getBoundingClientRect();
    const hit = document.elementFromPoint(
      rect.left + rect.width / 2,
      rect.top + rect.height / 2
    );
    return hit === button || button.contains(hit);
  });
  expect(hitTarget).toBe(true);

  await page.mouse.click(
    buttonBox!.x + buttonBox!.width / 2,
    buttonBox!.y + buttonBox!.height / 2
  );
  await expect(fixture).toHaveAttribute('data-settings-open', 'true');
  await expect(dialog).toBeVisible();
});

test('theme wipe absorbs covered pointer input without dismissing settings', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await openFixture(page);

  await page.evaluate(async () => {
    const designModuleUrl = '/src/lib/design/gsap.ts';
    const transitionModuleUrl =
      '/src/lib/features/shell/themePackageTransition.ts';
    const { applyMotionOverride, gsap } = await import(
      /* @vite-ignore */ designModuleUrl
    );
    const { runThemePackageTransition } = await import(
      /* @vite-ignore */ transitionModuleUrl
    );
    applyMotionOverride({ BASE: 320, BASE_OUT: 240 });
    const root = document.documentElement;
    const observer = new MutationObserver(() => {
      if (root.dataset.themePackageTransition !== 'reveal') return;
      gsap.globalTimeline.pause();
      const overlay = document.querySelector<HTMLElement>(
        '.theme-package-transition'
      );
      if (overlay) overlay.style.transform = 'translate(0px, 0px)';
      observer.disconnect();
    });
    observer.observe(root, {
      attributes: true,
      attributeFilter: ['data-theme-package-transition'],
    });
    (
      window as typeof window & {
        __SETTINGS_THEME_TRANSITION_FINISHED__?: Promise<void>;
      }
    ).__SETTINGS_THEME_TRANSITION_FINISHED__ = runThemePackageTransition(
      () => {},
      { reason: 'preview' }
    );
  });

  const fixture = page.getByTestId('settings-preview-backdrop-fixture');
  const dialog = page.locator('[data-slot="sheet-content"]');
  const closeButton = dialog.locator('[data-slot="sheet-close"]');
  const transition = page.getByTestId('theme-package-transition');
  await expect(transition).toHaveAttribute('data-phase', 'reveal');
  const closeBox = await closeButton.boundingBox();
  expect(closeBox).not.toBeNull();

  await page.mouse.click(
    closeBox!.x + closeBox!.width / 2,
    closeBox!.y + closeBox!.height / 2
  );
  await expect(fixture).toHaveAttribute('data-settings-open', 'true');
  await expect(dialog).toBeVisible();

  await page.evaluate(async () => {
    const designModuleUrl = '/src/lib/design/gsap.ts';
    const { applyMotionOverride, gsap } = await import(
      /* @vite-ignore */ designModuleUrl
    );
    gsap.globalTimeline.resume();
    await (
      window as typeof window & {
        __SETTINGS_THEME_TRANSITION_FINISHED__?: Promise<void>;
      }
    ).__SETTINGS_THEME_TRANSITION_FINISHED__;
    applyMotionOverride(null);
  });
  await expect(transition).toHaveCount(0);

  await closeButton.click();
  await expect(fixture).toHaveAttribute('data-settings-open', 'false');
});
