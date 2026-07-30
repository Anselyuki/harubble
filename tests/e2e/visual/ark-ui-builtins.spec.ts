import { expect, test } from '@playwright/test';

const FAMILIES = ['ark', 'endfield', 'exa', 'popucom', 'corporate'] as const;
const SCHEMES = ['light', 'dark'] as const;

for (const family of FAMILIES) {
  test(`${family} built-in shell renders without overflow`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    page.on('pageerror', (error) => errors.push(error.message));

    await page.setViewportSize({ width: 1120, height: 800 });
    const scheme = family === 'endfield' ? 'light' : 'dark';
    await page.goto(
      `http://127.0.0.1:1421/tests/e2e/visual/fixtures/ark-ui-family-fixture.html?family=${family}&scheme=${scheme}`
    );
    await page.waitForFunction(
      () =>
        (window as typeof window & { __ARK_UI_FIXTURE_READY__?: boolean })
          .__ARK_UI_FIXTURE_READY__ === true
    );

    expect(errors).toEqual([]);
    await expect(page.locator('html')).toHaveAttribute(
      'data-ark-depth',
      family === 'endfield' ? 'complex' : 'moderate'
    );
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth
      )
    ).toBe(true);

    if (family === 'endfield') {
      const railContrast = await page.evaluate(() => {
        const left = getComputedStyle(document.querySelector('.sidebar')!);
        const main = document.querySelector('.main-region')!;
        const right = getComputedStyle(main, '::after');
        return {
          leftBackground: left.backgroundColor,
          leftColor: left.color,
          rightBackground: right.backgroundColor,
          rightWidth: right.width,
          rightBorder: right.borderLeftColor,
          rightShadow: right.boxShadow,
        };
      });
      expect(railContrast.leftBackground).not.toBe(
        railContrast.rightBackground
      );
      expect(Number.parseFloat(railContrast.rightWidth)).toBeGreaterThanOrEqual(
        56
      );
      expect(railContrast.rightBorder).not.toBe('rgba(0, 0, 0, 0)');
      expect(railContrast.rightShadow).toContain('7px');

      const toolbarGeometry = await page
        .locator('.top-toolbar')
        .evaluate((el) => {
          const rect = el.getBoundingClientRect();
          const button = el.querySelector('button[aria-label="More"]');
          if (!button) return null;
          const buttonRect = button.getBoundingClientRect();
          const hit = document.elementFromPoint(
            buttonRect.left + buttonRect.width / 2,
            buttonRect.top + buttonRect.height / 2
          );
          return {
            position: getComputedStyle(document.querySelector('.top-actions')!)
              .position,
            left: rect.left,
            right: rect.right,
            top: rect.top,
            bottom: rect.bottom,
            hitButton: hit === button || button.contains(hit),
          };
        });
      expect(toolbarGeometry).not.toBeNull();
      expect(toolbarGeometry?.position).toBe('absolute');
      expect(toolbarGeometry?.left).toBeGreaterThanOrEqual(0);
      expect(toolbarGeometry?.right).toBeLessThanOrEqual(1120);
      expect(toolbarGeometry?.top).toBeGreaterThanOrEqual(0);
      expect(toolbarGeometry?.bottom).toBeLessThanOrEqual(800);
      expect(toolbarGeometry?.hitButton).toBe(true);

      const moreButton = page.getByRole('button', { name: 'More' });
      await expect(moreButton).toHaveCount(1);
      await moreButton.click();
    }

    await expect(page.locator('.app-shell')).toHaveScreenshot(
      `ark-ui-${family}-desktop.png`,
      { animations: 'disabled' }
    );

    await page.setViewportSize({ width: 480, height: 760 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth
      )
    ).toBe(true);
    if (family === 'endfield') {
      const narrowToolbarGeometry = await page
        .locator('.top-toolbar')
        .evaluate((el) => {
          const rect = el.getBoundingClientRect();
          const button = el.querySelector('button[aria-label="More"]');
          if (!button) return null;
          const buttonRect = button.getBoundingClientRect();
          const hit = document.elementFromPoint(
            buttonRect.left + buttonRect.width / 2,
            buttonRect.top + buttonRect.height / 2
          );
          return {
            left: rect.left,
            right: rect.right,
            top: rect.top,
            bottom: rect.bottom,
            hitButton: hit === button || button.contains(hit),
          };
        });
      expect(narrowToolbarGeometry).not.toBeNull();
      expect(narrowToolbarGeometry?.left).toBeGreaterThanOrEqual(0);
      expect(narrowToolbarGeometry?.right).toBeLessThanOrEqual(480);
      expect(narrowToolbarGeometry?.top).toBeGreaterThanOrEqual(0);
      expect(narrowToolbarGeometry?.bottom).toBeLessThanOrEqual(760);
      expect(narrowToolbarGeometry?.hitButton).toBe(true);
      const narrowComposition = await page
        .locator('.qa-field-player')
        .evaluate((player) => {
          const playerRect = player.getBoundingClientRect();
          const visibleCards = Array.from(
            document.querySelectorAll<HTMLElement>('.qa-field-card')
          ).filter((card) => getComputedStyle(card).display !== 'none');
          const overlapsCard = visibleCards.some((card) => {
            const cardRect = card.getBoundingClientRect();
            return (
              cardRect.left < playerRect.right &&
              cardRect.right > playerRect.left &&
              cardRect.top < playerRect.bottom &&
              cardRect.bottom > playerRect.top
            );
          });
          return {
            playerBottom: playerRect.bottom,
            visibleCardCount: visibleCards.length,
            overlapsCard,
          };
        });
      expect(narrowComposition.visibleCardCount).toBe(4);
      expect(narrowComposition.overlapsCard).toBe(false);
      expect(narrowComposition.playerBottom).toBeLessThanOrEqual(760);
      const narrowMoreButton = page.getByRole('button', { name: 'More' });
      await expect(narrowMoreButton).toHaveCount(1);
      await narrowMoreButton.click();
    }
    await expect(page.locator('.app-shell')).toHaveScreenshot(
      `ark-ui-${family}-narrow.png`,
      { animations: 'disabled' }
    );

    if (family === 'endfield') {
      await page.setViewportSize({ width: 552, height: 460 });
      const compactToolbarGeometry = await page
        .locator('.top-toolbar')
        .evaluate((el) => {
          const rect = el.getBoundingClientRect();
          const button = el.querySelector('button[aria-label="More"]');
          if (!button) return null;
          const buttonRect = button.getBoundingClientRect();
          const hit = document.elementFromPoint(
            buttonRect.left + buttonRect.width / 2,
            buttonRect.top + buttonRect.height / 2
          );
          return {
            left: rect.left,
            right: rect.right,
            top: rect.top,
            bottom: rect.bottom,
            hitButton: hit === button || button.contains(hit),
          };
        });
      expect(compactToolbarGeometry).not.toBeNull();
      expect(compactToolbarGeometry?.left).toBeGreaterThanOrEqual(0);
      expect(compactToolbarGeometry?.right).toBeLessThanOrEqual(552);
      expect(compactToolbarGeometry?.top).toBeGreaterThanOrEqual(0);
      expect(compactToolbarGeometry?.bottom).toBeLessThanOrEqual(460);
      expect(compactToolbarGeometry?.hitButton).toBe(true);
      const compactMoreButton = page.getByRole('button', { name: 'More' });
      await expect(compactMoreButton).toHaveCount(1);
      await compactMoreButton.click();

      await page.setViewportSize({ width: 900, height: 600 });
      const shortWideComposition = await page
        .locator('.qa-field-player')
        .evaluate((player) => {
          const playerRect = player.getBoundingClientRect();
          const visibleCards = Array.from(
            document.querySelectorAll<HTMLElement>('.qa-field-card')
          ).filter((card) => getComputedStyle(card).display !== 'none');
          const overlapsCard = visibleCards.some((card) => {
            const cardRect = card.getBoundingClientRect();
            return (
              cardRect.left < playerRect.right &&
              cardRect.right > playerRect.left &&
              cardRect.top < playerRect.bottom &&
              cardRect.bottom > playerRect.top
            );
          });
          return {
            playerBottom: playerRect.bottom,
            visibleCardCount: visibleCards.length,
            overlapsCard,
          };
        });
      expect(shortWideComposition.visibleCardCount).toBe(3);
      expect(shortWideComposition.overlapsCard).toBe(false);
      expect(shortWideComposition.playerBottom).toBeLessThanOrEqual(600);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth
        )
      ).toBe(true);
      await expect(page.locator('.app-shell')).toHaveScreenshot(
        'ark-ui-endfield-short-wide.png',
        { animations: 'disabled' }
      );
    }
  });
}

for (const family of FAMILIES) {
  for (const scheme of SCHEMES) {
    test(`${family} ${scheme} keeps theme chrome readable and clickable`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1120, height: 800 });
      await page.goto(
        `http://127.0.0.1:1421/tests/e2e/visual/fixtures/ark-ui-family-fixture.html?family=${family}&scheme=${scheme}`
      );
      await page.waitForFunction(
        () =>
          (window as typeof window & { __ARK_UI_FIXTURE_READY__?: boolean })
            .__ARK_UI_FIXTURE_READY__ === true
      );

      const moreButton = page.getByRole('button', { name: 'More' });
      await moreButton.focus();
      const metrics = await page.evaluate(() => {
        type Rgba = [number, number, number, number];
        const parseColor = (value: string): Rgba => {
          const match = value.match(/[\d.]+/g)?.map(Number) ?? [];
          if (match.length < 3) throw new Error(`Cannot parse color: ${value}`);
          return [match[0]!, match[1]!, match[2]!, match[3] ?? 1];
        };
        const composite = (foreground: Rgba, background: Rgba): Rgba => {
          const alpha = foreground[3] + background[3] * (1 - foreground[3]);
          if (alpha === 0) return [0, 0, 0, 0];
          return [
            (foreground[0] * foreground[3] +
              background[0] * background[3] * (1 - foreground[3])) /
              alpha,
            (foreground[1] * foreground[3] +
              background[1] * background[3] * (1 - foreground[3])) /
              alpha,
            (foreground[2] * foreground[3] +
              background[2] * background[3] * (1 - foreground[3])) /
              alpha,
            alpha,
          ];
        };
        const luminance = (color: Rgba): number => {
          const channels = color.slice(0, 3).map((channel) => {
            const value = channel / 255;
            return value <= 0.03928
              ? value / 12.92
              : Math.pow((value + 0.055) / 1.055, 2.4);
          });
          return (
            0.2126 * channels[0]! +
            0.7152 * channels[1]! +
            0.0722 * channels[2]!
          );
        };
        const contrast = (first: Rgba, second: Rgba): number => {
          const firstLuminance = luminance(first);
          const secondLuminance = luminance(second);
          return (
            (Math.max(firstLuminance, secondLuminance) + 0.05) /
            (Math.min(firstLuminance, secondLuminance) + 0.05)
          );
        };
        const resolveColor = (value: string): Rgba => {
          const probe = document.createElement('span');
          probe.style.color = value;
          document.body.append(probe);
          const resolved = parseColor(getComputedStyle(probe).color);
          probe.remove();
          return resolved;
        };

        const root = getComputedStyle(document.documentElement);
        const rootBackground = resolveColor(
          root.getPropertyValue('--bg-primary')
        );
        const toolbar = document.querySelector<HTMLElement>('.top-toolbar')!;
        const sidebar = document.querySelector<HTMLElement>('.sidebar')!;
        const activeSidebarItem = sidebar.querySelector<HTMLElement>(
          '.sidebar-item-button.active'
        )!;
        const selected = toolbar.querySelector<HTMLElement>(
          'button[aria-pressed="true"]'
        )!;
        const more = toolbar.querySelector<HTMLElement>(
          'button[aria-label="More"]'
        )!;
        const toolbarStyle = getComputedStyle(toolbar);
        const sidebarStyle = getComputedStyle(sidebar);
        const activeSidebarStyle = getComputedStyle(activeSidebarItem);
        const selectedStyle = getComputedStyle(selected);
        const moreStyle = getComputedStyle(more);
        const toolbarBackground = composite(
          parseColor(toolbarStyle.backgroundColor),
          rootBackground
        );
        const sidebarBackground = composite(
          parseColor(sidebarStyle.backgroundColor),
          rootBackground
        );
        const activeSidebarBackground = composite(
          parseColor(activeSidebarStyle.backgroundColor),
          sidebarBackground
        );
        const selectedBackground = composite(
          parseColor(selectedStyle.backgroundColor),
          toolbarBackground
        );
        const rule = resolveColor(root.getPropertyValue('--theme-custom-rule'));
        const ruleComposite = composite(rule, rootBackground);
        const moreRect = more.getBoundingClientRect();
        const hit = document.elementFromPoint(
          moreRect.left + moreRect.width / 2,
          moreRect.top + moreRect.height / 2
        );

        return {
          selectedContrast: contrast(
            parseColor(selectedStyle.color),
            selectedBackground
          ),
          sidebarContrast: contrast(
            parseColor(activeSidebarStyle.color),
            activeSidebarBackground
          ),
          focusContrast: contrast(
            parseColor(moreStyle.outlineColor),
            toolbarBackground
          ),
          ruleContrast: contrast(ruleComposite, rootBackground),
          hitButton: hit === more || more.contains(hit),
          panelVariable: root.getPropertyValue('--theme-custom-panel').trim(),
          ruleVariable: root.getPropertyValue('--theme-custom-rule').trim(),
        };
      });

      expect(metrics.panelVariable).not.toBe('');
      expect(metrics.ruleVariable).not.toBe('');
      expect(metrics.selectedContrast).toBeGreaterThanOrEqual(4.5);
      expect(metrics.sidebarContrast).toBeGreaterThanOrEqual(4.5);
      expect(metrics.focusContrast).toBeGreaterThanOrEqual(3);
      expect(metrics.ruleContrast).toBeGreaterThanOrEqual(1.1);
      expect(metrics.hitButton).toBe(true);
      await moreButton.click();
    });
  }
}

test('endfield dark variant keeps the utility dock in the viewport', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1120, height: 800 });
  await page.goto(
    'http://127.0.0.1:1421/tests/e2e/visual/fixtures/ark-ui-family-fixture.html?family=endfield&scheme=dark'
  );
  await page.waitForFunction(
    () =>
      (window as typeof window & { __ARK_UI_FIXTURE_READY__?: boolean })
        .__ARK_UI_FIXTURE_READY__ === true
  );

  const geometry = await page.locator('.top-toolbar').evaluate((el) => {
    const rect = el.getBoundingClientRect();
    return {
      right: rect.right,
      bottom: rect.bottom,
      background: getComputedStyle(el).backgroundColor,
    };
  });
  expect(geometry.right).toBeLessThanOrEqual(1120);
  expect(geometry.bottom).toBeLessThanOrEqual(800);
  expect(geometry.background).not.toBe('rgba(0, 0, 0, 0)');
  await expect(page.getByRole('button', { name: 'More' })).toHaveCount(1);
});
