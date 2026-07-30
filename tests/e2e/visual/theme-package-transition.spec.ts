import { expect, test, type Page } from '@playwright/test';

const FIXTURE_URL =
  'http://127.0.0.1:1421/tests/e2e/visual/fixtures/ark-ui-family-fixture.html?family=endfield&scheme=light';

type TransitionReason = 'activate' | 'preview' | 'dismiss-preview';

interface TransitionWindow extends Window {
  __THEME_PACKAGE_TRANSITION_FINISHED__?: Promise<void>;
  __THEME_PACKAGE_TRANSITION_COMMITS__?: string[];
}

async function openFixture(
  page: Page,
  reducedMotion: 'reduce' | 'no-preference' = 'no-preference'
): Promise<void> {
  await page.emulateMedia({ reducedMotion });
  await page.setViewportSize({ width: 1120, height: 800 });
  await page.goto(FIXTURE_URL);
  await page.waitForFunction(
    () =>
      (window as typeof window & { __ARK_UI_FIXTURE_READY__?: boolean })
        .__ARK_UI_FIXTURE_READY__ === true
  );
  await page.evaluate(() => {
    const transitionWindow = window as TransitionWindow;
    transitionWindow.__THEME_PACKAGE_TRANSITION_COMMITS__ = [];
    document.documentElement.dataset.qaThemePackage = 'endfield';
  });
}

async function startTransition(
  page: Page,
  reason: TransitionReason,
  nextPackage: string,
  options: { animate?: boolean; enterMs?: number; exitMs?: number } = {}
): Promise<void> {
  await page.evaluate(
    async ({ reason, nextPackage, animate, enterMs, exitMs }) => {
      const designModuleUrl = '/src/lib/design/gsap.ts';
      const transitionModuleUrl =
        '/src/lib/features/shell/themePackageTransition.ts';
      const { applyMotionOverride } = await import(
        /* @vite-ignore */ designModuleUrl
      );
      const { runThemePackageTransition } = await import(
        /* @vite-ignore */ transitionModuleUrl
      );
      applyMotionOverride({
        BASE: enterMs,
        BASE_OUT: exitMs,
      });

      const transitionWindow = window as TransitionWindow;
      transitionWindow.__THEME_PACKAGE_TRANSITION_FINISHED__ =
        runThemePackageTransition(
          () => {
            transitionWindow.__THEME_PACKAGE_TRANSITION_COMMITS__?.push(
              `${reason}:${nextPackage}`
            );
            document.documentElement.dataset.qaThemePackage = nextPackage;
          },
          { animate, reason }
        );
    },
    {
      reason,
      nextPackage,
      animate: options.animate,
      enterMs: options.enterMs ?? 520,
      exitMs: options.exitMs ?? 380,
    }
  );
}

async function waitForTransition(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await (window as TransitionWindow).__THEME_PACKAGE_TRANSITION_FINISHED__;
  });
}

test('theme package wipe covers right to left and commits only at full cover', async ({
  page,
}) => {
  await openFixture(page);
  await page.evaluate(async () => {
    const designModuleUrl = '/src/lib/design/gsap.ts';
    const { gsap } = await import(/* @vite-ignore */ designModuleUrl);
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
  });
  await startTransition(page, 'preview', 'ark');

  const root = page.locator('html');
  const overlay = page.getByTestId('theme-package-transition');
  await expect(overlay).toHaveAttribute('data-reason', 'preview');
  await expect(overlay).toHaveAttribute('data-phase', 'cover');
  await expect
    .poll(async () => {
      const box = await overlay.boundingBox();
      return Boolean(box && box.x > 80 && box.x < 1040);
    })
    .toBe(true);

  const firstX = (await overlay.boundingBox())!.x;
  await page.waitForTimeout(80);
  const secondX = (await overlay.boundingBox())!.x;
  expect(secondX).toBeLessThan(firstX);
  await expect(root).toHaveAttribute('data-qa-theme-package', 'endfield');

  await expect(root).toHaveAttribute('data-theme-package-transition', 'reveal');
  await expect(root).toHaveAttribute('data-qa-theme-package', 'ark');
  const veilAlpha = await overlay
    .locator('.theme-package-transition__veil')
    .evaluate((veil) => {
      const channels = getComputedStyle(veil)
        .backgroundColor.match(/[\d.]+/g)
        ?.map(Number);
      return channels?.[3] ?? 1;
    });
  expect(veilAlpha).toBe(1);

  await expect(page).toHaveScreenshot('theme-package-transition-covered.png', {
    animations: 'disabled',
  });
  await page.evaluate(async () => {
    const designModuleUrl = '/src/lib/design/gsap.ts';
    const { gsap } = await import(/* @vite-ignore */ designModuleUrl);
    gsap.globalTimeline.resume();
  });

  await waitForTransition(page);
  await expect(overlay).toHaveCount(0);
  await expect(root).not.toHaveAttribute('data-theme-package-transition');
});

test('rapid preview, activation, and dismissal only commit the newest intent', async ({
  page,
}) => {
  await openFixture(page);

  await startTransition(page, 'preview', 'ark', {
    enterMs: 300,
    exitMs: 220,
  });
  await startTransition(page, 'activate', 'exa', {
    enterMs: 300,
    exitMs: 220,
  });
  await startTransition(page, 'dismiss-preview', 'endfield', {
    enterMs: 300,
    exitMs: 220,
  });

  const overlay = page.getByTestId('theme-package-transition');
  await expect(overlay).toHaveAttribute('data-reason', 'dismiss-preview');
  await waitForTransition(page);

  await expect(page.locator('html')).toHaveAttribute(
    'data-qa-theme-package',
    'endfield'
  );
  expect(
    await page.evaluate(
      () => (window as TransitionWindow).__THEME_PACKAGE_TRANSITION_COMMITS__
    )
  ).toEqual(['dismiss-preview:endfield']);
  await expect(overlay).toHaveCount(0);
});

test('reduced motion and startup hydration commit without mounting a mask', async ({
  page,
}) => {
  await openFixture(page, 'reduce');

  await startTransition(page, 'preview', 'ark');
  await waitForTransition(page);
  await expect(page.locator('html')).toHaveAttribute(
    'data-qa-theme-package',
    'ark'
  );
  await expect(page.getByTestId('theme-package-transition')).toHaveCount(0);

  await startTransition(page, 'activate', 'endfield', { animate: false });
  await waitForTransition(page);
  await expect(page.locator('html')).toHaveAttribute(
    'data-qa-theme-package',
    'endfield'
  );
  await expect(page.getByTestId('theme-package-transition')).toHaveCount(0);
});
