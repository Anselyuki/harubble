import { expect, test, type Locator, type Page } from '@playwright/test';

const FIXTURE_URL =
  'http://127.0.0.1:1421/tests/e2e/visual/fixtures/endfield-album-actions-fixture.html';

const VIEWPORTS = [
  { name: 'desktop', width: 960, height: 640 },
  { name: 'portrait', width: 390, height: 700 },
] as const;

async function openFixture(
  page: Page,
  scheme: 'light' | 'dark'
): Promise<void> {
  await page.goto(`${FIXTURE_URL}?scheme=${scheme}`);
  await page.waitForFunction(
    () =>
      (
        window as typeof window & {
          __ENDFIELD_ALBUM_ACTIONS_FIXTURE_READY__?: boolean;
        }
      ).__ENDFIELD_ALBUM_ACTIONS_FIXTURE_READY__ === true
  );
  await page.waitForTimeout(600);
}

async function readInteractionStyle(locator: Locator) {
  return locator.evaluate((button: HTMLButtonElement) => {
    const style = getComputedStyle(button);
    const matrix = new DOMMatrixReadOnly(style.transform);
    return {
      boxShadow: style.boxShadow,
      opacity: style.opacity,
      translateX: matrix.m41,
      translateY: matrix.m42,
      scaleX: matrix.a,
      scaleY: matrix.d,
    };
  });
}

async function readHoverStyle(locator: Locator) {
  await locator.hover();
  await locator.page().waitForTimeout(320);
  return readInteractionStyle(locator);
}

async function readActiveStyle(page: Page, locator: Locator) {
  const box = await locator.boundingBox();
  if (!box) throw new Error('Album action button is not visible');

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  try {
    await expect
      .poll(async () => (await readInteractionStyle(locator)).boxShadow, {
        timeout: 2_000,
      })
      .toContain('1px 1px 0px');
    return await readInteractionStyle(locator);
  } finally {
    await page.mouse.move(1, 1);
    await page.mouse.up();
  }
}

async function readActionMetrics(page: Page) {
  return page.evaluate(() => {
    type Rgba = [number, number, number, number];

    const parseColor = (value: string): Rgba => {
      const channels = value.match(/[\d.]+/g)?.map(Number) ?? [];
      if (channels.length < 3) throw new Error(`Cannot parse color: ${value}`);
      return [channels[0]!, channels[1]!, channels[2]!, channels[3] ?? 1];
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
        0.2126 * channels[0]! + 0.7152 * channels[1]! + 0.0722 * channels[2]!
      );
    };
    const contrast = (foreground: Rgba, background: Rgba): number => {
      const paintedForeground = composite(foreground, background);
      const foregroundLuminance = luminance(paintedForeground);
      const backgroundLuminance = luminance(background);
      return (
        (Math.max(foregroundLuminance, backgroundLuminance) + 0.05) /
        (Math.min(foregroundLuminance, backgroundLuminance) + 0.05)
      );
    };
    const resolveColor = (value: string): string => {
      const probe = document.createElement('span');
      probe.style.color = value;
      document.body.append(probe);
      const resolved = getComputedStyle(probe).color;
      probe.remove();
      return resolved;
    };
    const inspectButton = (button: HTMLButtonElement) => {
      const style = getComputedStyle(button);
      const rect = button.getBoundingClientRect();
      const hit = document.elementFromPoint(
        rect.left + rect.width / 2,
        rect.top + rect.height / 2
      );
      const background = parseColor(style.backgroundColor);
      return {
        height: rect.height,
        borderWidths: [
          style.borderTopWidth,
          style.borderRightWidth,
          style.borderBottomWidth,
          style.borderLeftWidth,
        ],
        borderStyles: [
          style.borderTopStyle,
          style.borderRightStyle,
          style.borderBottomStyle,
          style.borderLeftStyle,
        ],
        borderColors: [
          style.borderTopColor,
          style.borderRightColor,
          style.borderBottomColor,
          style.borderLeftColor,
        ],
        borderRadius: [
          style.borderTopLeftRadius,
          style.borderTopRightRadius,
          style.borderBottomRightRadius,
          style.borderBottomLeftRadius,
        ],
        font: {
          family: style.fontFamily,
          size: style.fontSize,
          weight: style.fontWeight,
          lineHeight: style.lineHeight,
          letterSpacing: style.letterSpacing,
        },
        boxShadow: style.boxShadow,
        clipPath: style.clipPath,
        backgroundColor: style.backgroundColor,
        textContrast: contrast(parseColor(style.color), background),
        hitAtCenter: hit === button || button.contains(hit),
      };
    };

    const primary = document.querySelector<HTMLButtonElement>(
      '.album-detail-card .album-hero-actions .btn-primary'
    )!;
    const secondary = document.querySelector<HTMLButtonElement>(
      '.album-detail-card .album-hero-actions .btn:not(.btn-primary)'
    )!;
    const rootStyle = getComputedStyle(document.documentElement);

    return {
      primary: inspectButton(primary),
      secondary: inspectButton(secondary),
      signalColor: resolveColor(
        rootStyle.getPropertyValue('--ark-field-signal')
      ),
      paperColor: resolveColor(rootStyle.getPropertyValue('--ark-field-paper')),
      noHorizontalOverflow:
        document.documentElement.scrollWidth <= window.innerWidth,
    };
  });
}

for (const scheme of ['light', 'dark'] as const) {
  test(`endfield ${scheme} album actions share one button system`, async ({
    page,
  }) => {
    for (const viewport of VIEWPORTS) {
      await page.setViewportSize(viewport);
      await openFixture(page, scheme);

      await expect(page.locator('html')).toHaveAttribute(
        'data-ark-theme',
        'endfield'
      );
      await expect(page.locator('html')).toHaveAttribute(
        'data-ark-depth',
        'complex'
      );

      const metrics = await readActionMetrics(page);
      expect(metrics.primary.height).toBeGreaterThanOrEqual(40);
      expect(metrics.primary.height).toBeCloseTo(metrics.secondary.height, 2);
      expect(metrics.primary.borderWidths).toEqual(
        metrics.secondary.borderWidths
      );
      expect(metrics.primary.borderWidths).toEqual([
        '1px',
        '1px',
        '1px',
        '1px',
      ]);
      expect(metrics.primary.borderStyles).toEqual(
        metrics.secondary.borderStyles
      );
      expect(metrics.primary.borderStyles).toEqual([
        'solid',
        'solid',
        'solid',
        'solid',
      ]);
      expect(metrics.primary.borderColors).toEqual(
        metrics.secondary.borderColors
      );
      expect(metrics.primary.borderRadius).toEqual(
        metrics.secondary.borderRadius
      );
      expect(metrics.primary.borderRadius).toEqual([
        '0px',
        '0px',
        '0px',
        '0px',
      ]);
      expect(metrics.primary.font).toEqual(metrics.secondary.font);
      expect(Number(metrics.primary.font.weight)).toBeGreaterThanOrEqual(700);
      expect(metrics.primary.boxShadow).toBe(metrics.secondary.boxShadow);
      expect(metrics.primary.boxShadow).not.toBe('none');
      expect(metrics.primary.clipPath).toBe(metrics.secondary.clipPath);
      expect(metrics.primary.clipPath).toBe('none');

      expect(metrics.primary.backgroundColor).toBe(metrics.signalColor);
      expect(metrics.secondary.backgroundColor).toBe(metrics.paperColor);
      expect(metrics.primary.backgroundColor).not.toBe(
        metrics.secondary.backgroundColor
      );
      expect(metrics.primary.textContrast).toBeGreaterThanOrEqual(4.5);
      expect(metrics.secondary.textContrast).toBeGreaterThanOrEqual(4.5);
      expect(metrics.primary.hitAtCenter).toBe(true);
      expect(metrics.secondary.hitAtCenter).toBe(true);
      expect(metrics.noHorizontalOverflow).toBe(true);

      await expect(page.locator('.album-detail-card')).toHaveScreenshot(
        `endfield-album-actions-${scheme}-${viewport.name}.png`,
        { animations: 'disabled' }
      );

      const primaryAction = page
        .locator('.album-detail-card .album-hero-actions .btn-primary')
        .first();
      const secondaryAction = page
        .locator(
          '.album-detail-card .album-hero-actions .btn:not(.btn-primary)'
        )
        .first();
      const primaryHover = await readHoverStyle(primaryAction);
      const secondaryHover = await readHoverStyle(secondaryAction);
      expect(primaryHover.boxShadow).toBe(secondaryHover.boxShadow);
      expect(primaryHover.boxShadow).toContain('4px 4px 0px');
      expect(primaryHover.boxShadow).not.toContain('15, 23, 42');
      expect(primaryHover.boxShadow).not.toContain('20px');
      expect(primaryHover.boxShadow).not.toContain('24px');
      expect(primaryHover.translateX).toBeCloseTo(-1, 2);
      expect(primaryHover.translateY).toBeCloseTo(-1, 2);
      expect(primaryHover.translateX).toBeCloseTo(secondaryHover.translateX, 2);
      expect(primaryHover.translateY).toBeCloseTo(secondaryHover.translateY, 2);
      expect(primaryHover.scaleX).toBeCloseTo(1, 3);
      expect(primaryHover.scaleY).toBeCloseTo(1, 3);

      const primaryActive = await readActiveStyle(page, primaryAction);
      const secondaryActive = await readActiveStyle(page, secondaryAction);
      expect(primaryActive.boxShadow).toBe(secondaryActive.boxShadow);
      expect(primaryActive.boxShadow).toContain('1px 1px 0px');
      expect(primaryActive.boxShadow).not.toContain('15, 23, 42');
      expect(primaryActive.boxShadow).not.toContain('20px');
      expect(primaryActive.boxShadow).not.toContain('24px');
      expect(primaryActive.translateX).toBeCloseTo(2, 2);
      expect(primaryActive.translateY).toBeCloseTo(2, 2);
      expect(primaryActive.translateX).toBeCloseTo(
        secondaryActive.translateX,
        2
      );
      expect(primaryActive.translateY).toBeCloseTo(
        secondaryActive.translateY,
        2
      );
      expect(primaryActive.scaleX).toBeCloseTo(1, 3);
      expect(primaryActive.scaleY).toBeCloseTo(1, 3);
      expect(primaryActive.opacity).toBe('1');
      expect(secondaryActive.opacity).toBe('1');

      await page
        .locator('.album-detail-card')
        .evaluate((card) => card.classList.add('is-reduced-motion'));
      const reducedHover = await readHoverStyle(primaryAction);
      expect(reducedHover.translateX).toBeCloseTo(0, 3);
      expect(reducedHover.translateY).toBeCloseTo(0, 3);
      const reducedActive = await readActiveStyle(page, primaryAction);
      expect(reducedActive.translateX).toBeCloseTo(0, 3);
      expect(reducedActive.translateY).toBeCloseTo(0, 3);

      const fixture = page.getByTestId('endfield-album-actions-fixture');
      await primaryAction.click();
      await expect(fixture).toHaveAttribute('data-primary-clicks', '1');
      await secondaryAction.click();
      await expect(fixture).toHaveAttribute('data-secondary-clicks', '1');
    }
  });
}
