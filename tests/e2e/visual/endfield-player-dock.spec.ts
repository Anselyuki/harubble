import { expect, test, type Page } from '@playwright/test';

const FIXTURE_URL =
  'http://127.0.0.1:1421/tests/e2e/visual/fixtures/endfield-player-dock-fixture.html';

const VIEWPORTS = [
  { name: 'desktop', width: 1180, height: 360 },
  { name: 'portrait', width: 480, height: 520 },
] as const;

async function openFixture(
  page: Page,
  scheme: 'light' | 'dark',
  format: 'complete' | 'partial' = 'complete'
): Promise<void> {
  const params = new URLSearchParams({ scheme, format });
  await page.goto(`${FIXTURE_URL}?${params.toString()}`);
  await page.waitForFunction(
    () =>
      (
        window as typeof window & {
          __ENDFIELD_PLAYER_DOCK_FIXTURE_READY__?: boolean;
        }
      ).__ENDFIELD_PLAYER_DOCK_FIXTURE_READY__ === true
  );
  await page.waitForTimeout(300);
}

async function readExpandedFormatMetrics(page: Page) {
  return page.evaluate(() => {
    const shell = document.querySelector<HTMLElement>('.format-readout-shell')!;
    const source = document.querySelector<HTMLElement>('.format-pill-source')!;
    const sourceCore = source.querySelector<HTMLElement>(
      '.format-pill-source-core'
    )!;
    const sourceExtra = source.querySelector<HTMLElement>(
      '.format-source-extra'
    )!;
    const output = document.querySelector<HTMLElement>('.format-pill-output')!;
    const outputText = output.querySelector<HTMLElement>('.format-text')!;
    const arrow = document.querySelector<HTMLElement>('.format-arrow')!;
    const shellRect = shell.getBoundingClientRect();
    const sourceRect = source.getBoundingClientRect();
    const outputRect = output.getBoundingClientRect();
    const sourceStyle = getComputedStyle(source);
    const sourceExtraStyle = getComputedStyle(sourceExtra);
    const outputStyle = getComputedStyle(output);
    const arrowStyle = getComputedStyle(arrow);

    const insideShell = (rect: DOMRect): boolean =>
      rect.left >= shellRect.left - 0.5 &&
      rect.right <= shellRect.right + 0.5 &&
      rect.top >= shellRect.top - 0.5 &&
      rect.bottom <= shellRect.bottom + 0.5;

    return {
      sourceCore: sourceCore.textContent.trim(),
      sourceExtra: sourceExtra.textContent.trim(),
      output: outputText.textContent.trim(),
      arrow: arrow.textContent.trim(),
      sourceColor: sourceStyle.color,
      sourceExtraColor: sourceExtraStyle.color,
      outputColor: outputStyle.color,
      arrowColor: arrowStyle.color,
      sourceBackground: sourceStyle.backgroundColor,
      outputBackground: outputStyle.backgroundColor,
      sourceInsideShell: insideShell(sourceRect),
      outputInsideShell: insideShell(outputRect),
    };
  });
}

async function readPlayerMetrics(page: Page) {
  return page.evaluate(() => {
    type Rgba = [number, number, number, number];

    const parseColor = (value: string): Rgba => {
      const channels = value.match(/[\d.]+/g)?.map(Number) ?? [];
      if (channels.length < 3) throw new Error(`Cannot parse color: ${value}`);
      const normalized = value.startsWith('color(srgb');
      return [
        normalized ? channels[0]! * 255 : channels[0]!,
        normalized ? channels[1]! * 255 : channels[1]!,
        normalized ? channels[2]! * 255 : channels[2]!,
        channels[3] ?? 1,
      ];
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
        const normalized = channel / 255;
        return normalized <= 0.03928
          ? normalized / 12.92
          : Math.pow((normalized + 0.055) / 1.055, 2.4);
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

    const player = document.querySelector<HTMLElement>('.am-player')!;
    const playerStyle = getComputedStyle(player);
    const background = parseColor(playerStyle.backgroundColor);
    const readContrast = (selector: string): number => {
      const element = player.querySelector<HTMLElement>(selector)!;
      return contrast(parseColor(getComputedStyle(element).color), background);
    };
    const rect = player.getBoundingClientRect();

    return {
      background: playerStyle.backgroundColor,
      title: readContrast('.title'),
      artist: readContrast('.artist'),
      time: readContrast('.time-readout'),
      format: readContrast('.format-readout'),
      inactiveControl: readContrast('.side-toggle[aria-pressed="false"]'),
      transportControl: readContrast('.transport-button:not(:disabled)'),
      volumeControl: readContrast('.capsule-icon-btn'),
      insideViewport:
        rect.left >= 0 &&
        rect.right <= window.innerWidth &&
        rect.top >= 0 &&
        rect.bottom <= window.innerHeight,
      noHorizontalOverflow:
        document.documentElement.scrollWidth <= window.innerWidth,
    };
  });
}

for (const scheme of ['light', 'dark'] as const) {
  test(`endfield ${scheme} player dock keeps playback information readable`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    page.on('pageerror', (error) => errors.push(error.message));

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

      const metrics = await readPlayerMetrics(page);
      expect(errors).toEqual([]);
      expect(metrics.background).toBe('rgb(25, 25, 25)');
      expect(metrics.title).toBeGreaterThanOrEqual(4.5);
      expect(metrics.artist).toBeGreaterThanOrEqual(4.5);
      expect(metrics.time).toBeGreaterThanOrEqual(4.5);
      expect(metrics.format).toBeGreaterThanOrEqual(4.5);
      expect(metrics.inactiveControl).toBeGreaterThanOrEqual(3);
      expect(metrics.transportControl).toBeGreaterThanOrEqual(3);
      expect(metrics.volumeControl).toBeGreaterThanOrEqual(3);
      expect(metrics.insideViewport).toBe(true);
      expect(metrics.noHorizontalOverflow).toBe(true);

      await expect(page.locator('.am-player')).toHaveScreenshot(
        `endfield-player-dock-${scheme}-${viewport.name}.png`,
        { animations: 'disabled' }
      );
    }
  });
}

test('endfield expanded format path keeps partial metadata on one readable surface', async ({
  page,
}) => {
  await page.setViewportSize({ width: 760, height: 420 });

  for (const scheme of ['light', 'dark'] as const) {
    await openFixture(page, scheme, 'partial');
    const button = page.locator('.format-readout');
    await button.click();
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await expect(page.locator('.format-popover-content')).toBeVisible();
    await expect
      .poll(async () => {
        const metrics = await readExpandedFormatMetrics(page);
        return metrics.sourceInsideShell && metrics.outputInsideShell;
      })
      .toBe(true);

    const metrics = await readExpandedFormatMetrics(page);
    expect(metrics.sourceCore).toBe('48k/--bit');
    expect(metrics.sourceExtra).toBe('/2ch');
    expect(metrics.output).toBe('48k/32bit/2ch f32');
    expect(metrics.arrow).toBe('->');
    expect(metrics.sourceExtraColor).toBe(metrics.sourceColor);
    expect(metrics.outputColor).toBe(metrics.sourceColor);
    expect(metrics.arrowColor).toBe(metrics.sourceColor);
    expect(metrics.outputBackground).toBe(metrics.sourceBackground);
    expect(metrics.sourceInsideShell).toBe(true);
    expect(metrics.outputInsideShell).toBe(true);

    await expect(page.locator('.format-readout-shell')).toHaveScreenshot(
      `endfield-format-path-${scheme}.png`,
      { animations: 'disabled', maxDiffPixelRatio: 0 }
    );
  }
});
