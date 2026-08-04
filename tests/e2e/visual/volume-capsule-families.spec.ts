import { expect, test, type Browser, type Page } from '@playwright/test';

const FIXTURE_URL =
  'http://127.0.0.1:1421/tests/e2e/visual/fixtures/volume-capsule-family-fixture.html';
const COLLAPSE_GRACE_MS = 799;

const FAMILIES = [
  {
    family: 'ark',
    scheme: 'dark',
    depth: 'moderate',
    formatMarker: { width: 2, height: 10, border: 0, clipped: false },
    formatPill: { border: 1, rounded: false },
    formatPopover: {
      border: 1,
      radiusTopLeft: 0,
      radiusTopRight: 0,
      backdrop: false,
      headerMarker: { width: 2, height: 14, clipped: false },
    },
  },
  {
    family: 'endfield',
    scheme: 'dark',
    depth: 'complex',
    formatMarker: { width: 8, height: 12, border: 0, clipped: true },
    formatPill: { border: 1, rounded: false },
    formatPopover: {
      border: 1,
      radiusTopLeft: 2,
      radiusTopRight: 2,
      backdrop: false,
      headerMarker: { width: 9, height: 14, clipped: true },
    },
  },
  {
    family: 'exa',
    scheme: 'dark',
    depth: 'moderate',
    formatMarker: { width: 10, height: 10, border: 2, clipped: false },
    formatPill: { border: 1, rounded: true },
    formatPopover: {
      border: 1,
      radiusTopLeft: 0,
      radiusTopRight: 14,
      backdrop: true,
      headerMarker: { width: 10, height: 10, clipped: false },
    },
  },
  {
    family: 'popucom',
    scheme: 'light',
    depth: 'moderate',
    formatMarker: { width: 9, height: 9, border: 2, clipped: false },
    formatPill: { border: 2, rounded: true },
    formatPopover: {
      border: 2,
      radiusTopLeft: 16,
      radiusTopRight: 16,
      backdrop: false,
      headerMarker: { width: 10, height: 10, clipped: false },
    },
  },
  {
    family: 'corporate',
    scheme: 'dark',
    depth: 'moderate',
    formatMarker: { width: 10, height: 2, border: 0, clipped: false },
    formatPill: { border: 1, rounded: false },
    formatPopover: {
      border: 1,
      radiusTopLeft: 0,
      radiusTopRight: 0,
      backdrop: false,
      headerMarker: { width: 24, height: 2, clipped: false },
    },
  },
] as const;

type Family = (typeof FAMILIES)[number]['family'];
type LegacyFamily = 'glass' | 'material' | 'terminal';
type FixtureFamily = Family | LegacyFamily;
type Scheme = (typeof FAMILIES)[number]['scheme'];
type FixtureWindow = typeof window & {
  __SET_VOLUME_CAPSULE_FAMILY__?: (family: Family) => void;
  __SET_VOLUME_CAPSULE_POSITION__?: (position: number) => void;
  __SET_VOLUME_CAPSULE_FORMAT_AVAILABLE__?: (available: boolean) => void;
  __SET_VOLUME_CAPSULE_FORMAT_PROCESSING__?: (processing: boolean) => void;
};

async function openFixture(
  page: Page,
  family: FixtureFamily,
  scheme: Scheme,
  options: { processing?: boolean } = {}
): Promise<void> {
  const query = new URLSearchParams({ family, scheme });
  if (options.processing) query.set('processing', 'true');
  await page.goto(`${FIXTURE_URL}?${query.toString()}`);
  await page.waitForFunction(
    () =>
      (
        window as typeof window & {
          __VOLUME_CAPSULE_FAMILY_FIXTURE_READY__?: boolean;
        }
      ).__VOLUME_CAPSULE_FAMILY_FIXTURE_READY__ === true
  );
}

async function readFormatTokenOwnership(page: Page) {
  return page.evaluate(() => {
    const root = document.documentElement;
    const player = document.querySelector<HTMLElement>('.am-player')!;
    const anchor = document.querySelector<HTMLElement>(
      '.format-readout-anchor'
    )!;
    const shell = document.querySelector<HTMLElement>('.format-readout-shell')!;
    const pill = document.querySelector<HTMLElement>('.format-pill-source')!;
    const text = pill.querySelector<HTMLElement>('.format-text')!;
    const playerStyle = getComputedStyle(player);
    const anchorStyle = getComputedStyle(anchor);
    const pillStyle = getComputedStyle(pill);
    const shellRect = shell.getBoundingClientRect();
    const pillRect = pill.getBoundingClientRect();

    const resolveColor = (value: string): string => {
      const probe = document.createElement('span');
      probe.style.color = value;
      player.appendChild(probe);
      const resolved = getComputedStyle(probe).color;
      probe.remove();
      return resolved;
    };

    return {
      label: text.textContent.trim(),
      textColor: pillStyle.color,
      playerControlColor: resolveColor(
        playerStyle.getPropertyValue('--icon-default').trim()
      ),
      familySignal: resolveColor(
        anchorStyle.getPropertyValue('--format-family-signal').trim()
      ),
      appSignal: resolveColor(
        getComputedStyle(root).getPropertyValue('--theme-accent').trim()
      ),
      contextSignal: resolveColor(
        getComputedStyle(root).getPropertyValue('--album-accent').trim()
      ),
      insideShell:
        pillRect.left >= shellRect.left - 0.5 &&
        pillRect.right <= shellRect.right + 0.5 &&
        pillRect.top >= shellRect.top - 0.5 &&
        pillRect.bottom <= shellRect.bottom + 0.5,
    };
  });
}

async function openFormatPopover(page: Page): Promise<void> {
  const button = page.locator('.format-readout');
  await button.click();
  await expect(button).toHaveAttribute('aria-expanded', 'true');
  await expect(page.locator('#player-format-details')).toBeVisible();
  await expect
    .poll(() =>
      page.locator('#player-format-details').evaluate((panel) => ({
        clipPath: getComputedStyle(panel).clipPath,
        opacity: getComputedStyle(panel).opacity,
      }))
    )
    .toEqual({ clipPath: 'none', opacity: '1' });
}

async function readFormatPopover(page: Page) {
  return page.evaluate(() => {
    const root = document.documentElement;
    const player = document.querySelector<HTMLElement>('.am-player')!;
    const anchor = document.querySelector<HTMLElement>(
      '.format-readout-anchor'
    )!;
    const triggerShell = document.querySelector<HTMLElement>(
      '.format-readout-shell'
    )!;
    const trigger = document.querySelector<HTMLElement>('.format-readout')!;
    const triggerPill = trigger.querySelector<HTMLElement>(
      '.format-pill-source'
    )!;
    const panel = document.querySelector<HTMLElement>(
      '#player-format-details'
    )!;
    const header = panel.querySelector<HTMLElement>('.format-popover-header')!;
    const columns = panel.querySelector<HTMLElement>(
      '.format-popover-columns'
    )!;
    const columnList = Array.from(
      panel.querySelectorAll<HTMLElement>('.format-popover-column')
    );
    const labels = Array.from(
      panel.querySelectorAll<HTMLElement>('.format-popover-label')
    );
    const values = Array.from(
      panel.querySelectorAll<HTMLElement>('.format-popover-value')
    );
    const textNodes = Array.from(
      panel.querySelectorAll<HTMLElement>(
        '.format-popover-header, .format-popover-section-badge, .format-popover-section-badge-accent, .format-popover-label, .format-popover-value'
      )
    );
    const rootStyle = getComputedStyle(root);
    const anchorStyle = getComputedStyle(anchor);
    const panelStyle = getComputedStyle(panel);
    const markerStyle = getComputedStyle(header, '::before');
    const columnStyle = getComputedStyle(columns);
    const secondColumnStyle = getComputedStyle(columnList[1]!);
    const panelRect = panel.getBoundingClientRect();
    const triggerShellRect = triggerShell.getBoundingClientRect();
    const triggerRect = trigger.getBoundingClientRect();
    const triggerPillRect = triggerPill.getBoundingClientRect();

    const resolveColor = (value: string): string => {
      const probe = document.createElement('span');
      probe.style.color = value;
      player.appendChild(probe);
      const resolved = getComputedStyle(probe).color;
      probe.remove();
      return resolved;
    };
    const resolveBackground = (value: string): string => {
      const probe = document.createElement('span');
      probe.style.backgroundColor = value;
      player.appendChild(probe);
      const resolved = getComputedStyle(probe).backgroundColor;
      probe.remove();
      return resolved;
    };

    return {
      role: panel.getAttribute('role'),
      processingVisible:
        panel.querySelector('.format-popover-processing') !== null,
      surface: panelStyle.backgroundColor,
      themePanel: resolveBackground(
        rootStyle.getPropertyValue('--theme-custom-panel').trim()
      ),
      ink: resolveColor(
        anchorStyle.getPropertyValue('--format-popover-ink').trim()
      ),
      themeInk: resolveColor(
        rootStyle.getPropertyValue('--theme-text-primary').trim()
      ),
      muted: resolveColor(
        anchorStyle.getPropertyValue('--format-popover-muted').trim()
      ),
      themeMuted: resolveColor(
        rootStyle.getPropertyValue('--theme-text-secondary').trim()
      ),
      familySignal: resolveColor(
        anchorStyle.getPropertyValue('--format-family-signal').trim()
      ),
      appSignal: resolveColor(
        rootStyle.getPropertyValue('--theme-accent').trim()
      ),
      contextSignal: resolveColor(
        rootStyle.getPropertyValue('--album-accent').trim()
      ),
      border: Number.parseFloat(panelStyle.borderTopWidth),
      radiusTopLeft: Number.parseFloat(panelStyle.borderTopLeftRadius),
      radiusTopRight: Number.parseFloat(panelStyle.borderTopRightRadius),
      backdrop: panelStyle.backdropFilter,
      headerMarker: {
        width: Number.parseFloat(markerStyle.width),
        height: Number.parseFloat(markerStyle.height),
        clipped: markerStyle.clipPath !== 'none',
      },
      labelFontSize: Number.parseFloat(getComputedStyle(labels[0]!).fontSize),
      valueFontSize: Number.parseFloat(getComputedStyle(values[0]!).fontSize),
      triggerHeight: triggerRect.height,
      triggerContentVisible:
        triggerPillRect.left >= triggerShellRect.left - 0.5 &&
        triggerPillRect.right <= triggerShellRect.right + 0.5 &&
        triggerPillRect.top >= triggerShellRect.top - 0.5 &&
        triggerPillRect.bottom <= triggerShellRect.bottom + 0.5,
      columnCount: columnStyle.gridTemplateColumns.split(' ').length,
      secondColumnBorderLeft: Number.parseFloat(
        secondColumnStyle.borderLeftWidth
      ),
      secondColumnBorderTop: Number.parseFloat(
        secondColumnStyle.borderTopWidth
      ),
      panelInsideViewport:
        panelRect.left >= 0 &&
        panelRect.right <= window.innerWidth &&
        panelRect.top >= 0 &&
        panelRect.bottom <= window.innerHeight,
      noHorizontalOverflow:
        root.scrollWidth <= window.innerWidth &&
        panel.scrollWidth <= panel.clientWidth + 1,
      allTextInside: textNodes.every((node) => {
        const rect = node.getBoundingClientRect();
        return (
          rect.left >= panelRect.left - 0.5 &&
          rect.right <= panelRect.right + 0.5
        );
      }),
      textOverlapsTrigger: textNodes.some((node) => {
        const rect = node.getBoundingClientRect();
        return !(
          rect.right <= triggerRect.left ||
          rect.left >= triggerRect.right ||
          rect.bottom <= triggerRect.top ||
          rect.top >= triggerRect.bottom
        );
      }),
    };
  });
}

async function focusSlider(page: Page): Promise<void> {
  const button = page.locator('.capsule-icon-btn');
  const slider = page.locator('.capsule-slider');
  await button.focus();
  await expect(page.locator('.volume-wrapper')).toHaveAttribute(
    'data-state',
    'open'
  );
  await expect(button).toBeFocused();

  await page.keyboard.press('Tab');
  await expect(slider).toBeFocused();
  await expect
    .poll(async () => (await readGeometry(page)).revealWidth)
    .toBeGreaterThanOrEqual(199);
}

async function previewPosition(page: Page, position: number): Promise<void> {
  const slider = page.locator('.capsule-slider');
  await slider.dispatchEvent('pointerdown', {
    pointerId: 1,
    pointerType: 'mouse',
    isPrimary: true,
    buttons: 1,
  });
  await slider.evaluate((input: HTMLInputElement, nextPosition: number) => {
    input.value = String(nextPosition);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }, position);
  await expect(page.locator('.volume-hover-zone')).toHaveClass(/dragging/);
  await expectPosition(page, position);
}

async function commitPosition(page: Page): Promise<void> {
  const slider = page.locator('.capsule-slider');
  await slider.dispatchEvent('pointerup', {
    pointerId: 1,
    pointerType: 'mouse',
    isPrimary: true,
    buttons: 0,
  });
  await slider.evaluate((input: HTMLInputElement) => {
    input.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await expect(page.locator('.volume-hover-zone')).not.toHaveClass(/dragging/);
}

async function readPosition(page: Page) {
  return page
    .locator('.capsule-slider')
    .evaluate((slider: HTMLInputElement) => {
      const readout =
        document.querySelector<HTMLOutputElement>('.volume-readout')!;
      const percent = Math.round(Number(slider.value) * 100);
      return {
        inputPercent: percent,
        cssPercent: Math.round(
          Number.parseFloat(slider.style.getPropertyValue('--volume-percent'))
        ),
        readout: readout.textContent.replace(/\s/g, ''),
        ariaValueText: slider.getAttribute('aria-valuetext'),
      };
    });
}

async function expectPosition(page: Page, position: number): Promise<void> {
  const percent = Math.round(position * 100);
  await expect
    .poll(() => readPosition(page))
    .toEqual({
      inputPercent: percent,
      cssPercent: percent,
      readout: `${percent}%`,
      ariaValueText: `${percent}%`,
    });
}

async function readGeometry(page: Page) {
  return page.evaluate(() => {
    const root = document.documentElement;
    const player = document.querySelector<HTMLElement>('.am-player')!;
    const zone = document.querySelector<HTMLElement>('.volume-hover-zone')!;
    const wrapper = document.querySelector<HTMLElement>('.volume-wrapper')!;
    const reveal = document.querySelector<HTMLElement>('.capsule-reveal')!;
    const track = document.querySelector<HTMLElement>('.capsule-track')!;
    const slider = document.querySelector<HTMLInputElement>('.capsule-slider')!;
    const button = document.querySelector<HTMLElement>('.capsule-icon-btn')!;
    const readout = document.querySelector<HTMLElement>('.volume-readout')!;
    const formatAnchor = document.querySelector<HTMLElement>(
      '.format-readout-anchor'
    )!;
    const formatShell = document.querySelector<HTMLElement>(
      '.format-readout-shell'
    )!;
    const formatPill = document.querySelector<HTMLElement>(
      '.format-pill-source'
    )!;
    const formatText = formatPill.querySelector<HTMLElement>('.format-text')!;
    const formatSwatch = document.querySelector<HTMLElement>('.format-swatch')!;
    const volumeGroup = document.querySelector<HTMLElement>('.volume-group')!;
    const previousControl = volumeGroup.previousElementSibling as HTMLElement;
    const transport =
      document.querySelector<HTMLElement>('.transport-cluster')!;
    const revealRect = reveal.getBoundingClientRect();
    const trackRect = track.getBoundingClientRect();
    const sliderRect = slider.getBoundingClientRect();
    const buttonRect = button.getBoundingClientRect();
    const readoutRect = readout.getBoundingClientRect();
    const previousControlRect = previousControl.getBoundingClientRect();
    const transportRect = transport.getBoundingClientRect();
    const playerRect = player.getBoundingClientRect();
    const volumeGroupRect = volumeGroup.getBoundingClientRect();
    const formatShellRect = formatShell.getBoundingClientRect();
    const formatPillRect = formatPill.getBoundingClientRect();
    const formatTextRect = formatText.getBoundingClientRect();
    const revealStyle = getComputedStyle(reveal);
    const trackStyle = getComputedStyle(track);
    const focusLayerStyle = getComputedStyle(track, '::after');
    const rootStyle = getComputedStyle(root);
    const zoneStyle = getComputedStyle(zone);
    const formatAnchorStyle = getComputedStyle(formatAnchor);
    const formatPillStyle = getComputedStyle(formatPill);
    const formatSwatchStyle = getComputedStyle(formatSwatch);

    const resolveColor = (value: string): string => {
      const probe = document.createElement('span');
      probe.style.color = value;
      player.appendChild(probe);
      const resolved = getComputedStyle(probe).color;
      probe.remove();
      return resolved;
    };
    const overlaps = (a: DOMRect, b: DOMRect): boolean =>
      !(
        a.right <= b.left ||
        a.left >= b.right ||
        a.bottom <= b.top ||
        a.top >= b.bottom
      );

    return {
      state: wrapper.dataset['state'],
      revealWidth: revealRect.width,
      revealOpacity: Number(revealStyle.opacity),
      revealVisibility: revealStyle.visibility,
      revealTransform: revealStyle.transform,
      trackWidth: trackRect.width,
      trackHeight: trackRect.height,
      trackCssHeight: Number.parseFloat(trackStyle.height),
      trackTransform: trackStyle.transform,
      volumeGroupWidth: volumeGroupRect.width,
      buttonTarget: { width: buttonRect.width, height: buttonRect.height },
      sliderTarget: { width: sliderRect.width, height: sliderRect.height },
      buttonOverlapsPreviousControl: overlaps(buttonRect, previousControlRect),
      revealOverlapsPreviousControl: overlaps(revealRect, previousControlRect),
      readoutOverlapsTransport: overlaps(readoutRect, transportRect),
      readoutInsideTrack:
        readoutRect.left >= trackRect.left - 0.5 &&
        readoutRect.right <= trackRect.right + 0.5 &&
        readoutRect.top >= trackRect.top - 0.5 &&
        readoutRect.bottom <= trackRect.bottom + 0.5,
      insideViewport:
        revealRect.left >= 0 && revealRect.right <= window.innerWidth,
      insidePlayer:
        revealRect.left >= playerRect.left &&
        revealRect.right <= playerRect.right,
      noHorizontalOverflow: root.scrollWidth <= window.innerWidth,
      tabIndex: slider.tabIndex,
      focusOutlineWidth: Number.parseFloat(focusLayerStyle.borderTopWidth),
      focusOutlineColor: resolveColor(focusLayerStyle.borderTopColor),
      focusLayerPointerEvents: focusLayerStyle.pointerEvents,
      formatMarker: {
        width: Number.parseFloat(formatSwatchStyle.width),
        height: Number.parseFloat(formatSwatchStyle.height),
        border: Number.parseFloat(formatSwatchStyle.borderTopWidth),
        clipped: formatSwatchStyle.clipPath !== 'none',
      },
      formatPill: {
        border: Number.parseFloat(formatPillStyle.borderTopWidth),
        radius: Number.parseFloat(formatPillStyle.borderTopLeftRadius),
        insideShell:
          formatPillRect.left >= formatShellRect.left - 0.5 &&
          formatPillRect.right <= formatShellRect.right + 0.5 &&
          formatPillRect.top >= formatShellRect.top - 0.5 &&
          formatPillRect.bottom <= formatShellRect.bottom + 0.5,
        textInside:
          formatTextRect.left >= formatPillRect.left - 0.5 &&
          formatTextRect.right <= formatPillRect.right + 0.5,
      },
      formatLabel: formatText.textContent.trim(),
      formatSignal: resolveColor(
        formatAnchorStyle.getPropertyValue('--format-family-signal').trim()
      ),
      appSignal: resolveColor(
        zoneStyle.getPropertyValue('--volume-family-signal').trim()
      ),
      contextSignal: resolveColor(
        zoneStyle.getPropertyValue('--volume-context-signal').trim()
      ),
      rootAppSignal: resolveColor(
        rootStyle.getPropertyValue('--theme-accent').trim()
      ),
      rootContextSignal: resolveColor(
        rootStyle.getPropertyValue('--album-accent').trim()
      ),
    };
  });
}

for (const config of FAMILIES) {
  test(`${config.family} has a distinct and usable volume terminal state`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text());
    });
    page.on('pageerror', (error) => errors.push(error.message));

    for (const viewport of [
      { name: 'desktop', width: 920, height: 320 },
      { name: 'narrow', width: 420, height: 520 },
    ] as const) {
      await page.setViewportSize(viewport);
      await openFixture(page, config.family, config.scheme);
      await expect(page.locator('html')).toHaveAttribute(
        'data-ark-theme',
        config.family
      );
      await expect(page.locator('html')).toHaveAttribute(
        'data-ark-depth',
        config.depth
      );

      await focusSlider(page);
      await previewPosition(page, 0.37);
      await commitPosition(page);

      const metrics = await readGeometry(page);
      expect(metrics.state).toBe('open');
      expect(metrics.revealWidth).toBeGreaterThanOrEqual(199);
      expect(metrics.revealWidth).toBeLessThanOrEqual(201);
      expect(metrics.trackWidth).toBeCloseTo(200, 0);
      expect(
        Math.abs(metrics.trackHeight - metrics.trackCssHeight)
      ).toBeLessThanOrEqual(0.5);
      expect(metrics.revealOpacity).toBeCloseTo(1, 2);
      expect(metrics.revealVisibility).toBe('visible');
      expect(metrics.revealTransform).toBe('none');
      expect(metrics.trackTransform).toBe('none');
      expect(metrics.buttonTarget.width).toBeGreaterThanOrEqual(40);
      expect(metrics.buttonTarget.height).toBeGreaterThanOrEqual(40);
      expect(metrics.sliderTarget.width).toBeGreaterThanOrEqual(40);
      expect(metrics.sliderTarget.height).toBeGreaterThanOrEqual(40);
      expect(metrics.buttonOverlapsPreviousControl).toBe(false);
      expect(metrics.revealOverlapsPreviousControl).toBe(false);
      expect(metrics.readoutOverlapsTransport).toBe(false);
      expect(metrics.readoutInsideTrack).toBe(true);
      expect(metrics.insideViewport).toBe(true);
      expect(metrics.insidePlayer).toBe(true);
      expect(metrics.noHorizontalOverflow).toBe(true);
      expect(metrics.tabIndex).toBe(0);
      expect(metrics.focusOutlineWidth).toBeGreaterThanOrEqual(2);
      expect(metrics.focusOutlineColor).toBe(metrics.appSignal);
      expect(metrics.focusLayerPointerEvents).toBe('none');
      expect(metrics.formatMarker).toEqual(config.formatMarker);
      expect(metrics.formatPill.border).toBe(config.formatPill.border);
      expect(metrics.formatPill.insideShell).toBe(true);
      expect(metrics.formatPill.textInside).toBe(true);
      expect(metrics.formatLabel).toBe('48k/24bit');
      if (config.formatPill.rounded) {
        expect(metrics.formatPill.radius).toBeGreaterThanOrEqual(10);
      } else {
        expect(metrics.formatPill.radius).toBeLessThanOrEqual(2);
      }
      expect(metrics.formatSignal).toBe(metrics.rootAppSignal);
      expect(metrics.formatSignal).not.toBe(metrics.rootContextSignal);
      expect(metrics.appSignal).toBe(metrics.rootAppSignal);
      expect(metrics.contextSignal).toBe(metrics.rootContextSignal);
      expect(metrics.appSignal).not.toBe(metrics.contextSignal);
      expect(errors).toEqual([]);

      await expect(page).toHaveScreenshot(
        `volume-capsule-${config.family}-${viewport.name}.png`,
        { animations: 'disabled', maxDiffPixelRatio: 0.001 }
      );
      await expect(page.locator('.format-readout-shell')).toHaveScreenshot(
        `format-readout-${config.family}-${viewport.name}.png`,
        { animations: 'disabled', maxDiffPixelRatio: 0 }
      );

      const formatButton = page.locator('.format-readout');
      const collapsedFormatWidth = await page
        .locator('.format-readout-shell')
        .evaluate((shell) => shell.getBoundingClientRect().width);
      await formatButton.focus();
      await page.keyboard.press('Tab');
      await page.keyboard.press('Shift+Tab');
      await expect(formatButton).toBeFocused();
      const expandedPath = () =>
        page.evaluate(() => {
          const shell = document.querySelector<HTMLElement>(
            '.format-readout-shell'
          )!;
          const outputPill = document.querySelector<HTMLElement>(
            '.format-pill-output'
          )!;
          const outputText =
            outputPill.querySelector<HTMLElement>('.format-text')!;
          const shellRect = shell.getBoundingClientRect();
          const outputPillRect = outputPill.getBoundingClientRect();
          const outputTextRect = outputText.getBoundingClientRect();
          return {
            shellWidth: shellRect.width,
            pillInside:
              outputPillRect.left >= shellRect.left - 0.5 &&
              outputPillRect.right <= shellRect.right + 0.5,
            textInside:
              outputTextRect.left >= outputPillRect.left - 0.5 &&
              outputTextRect.right <= outputPillRect.right + 0.5,
            textUnclipped: outputText.scrollWidth <= outputText.clientWidth + 1,
          };
        });
      await expect.poll(expandedPath).toMatchObject({
        pillInside: true,
        textInside: true,
        textUnclipped: true,
      });
      expect((await expandedPath()).shellWidth).toBeGreaterThan(
        collapsedFormatWidth
      );
      const formatFocus = await page.evaluate(() => {
        const root = document.documentElement;
        const shell = document.querySelector<HTMLElement>(
          '.format-readout-shell'
        )!;
        const anchor = document.querySelector<HTMLElement>(
          '.format-readout-anchor'
        )!;
        const focusStyle = getComputedStyle(shell, '::after');
        const probe = document.createElement('span');
        probe.style.color = getComputedStyle(anchor)
          .getPropertyValue('--format-family-signal')
          .trim();
        root.appendChild(probe);
        const appSignal = getComputedStyle(probe).color;
        probe.remove();
        return {
          width: Number.parseFloat(focusStyle.borderTopWidth),
          color: focusStyle.borderTopColor,
          appSignal,
          pointerEvents: focusStyle.pointerEvents,
        };
      });
      expect(formatFocus.width).toBeGreaterThanOrEqual(2);
      expect(formatFocus.color).toBe(formatFocus.appSignal);
      expect(formatFocus.pointerEvents).toBe('none');
    }
  });

  test(`${config.family} keeps playback information themed, pinned, and responsive`, async ({
    page,
  }) => {
    for (const viewport of [
      { name: 'desktop', width: 920, height: 420, columns: 2 },
      { name: 'narrow', width: 340, height: 520, columns: 1 },
    ] as const) {
      await page.setViewportSize(viewport);
      await openFixture(page, config.family, config.scheme, {
        processing: true,
      });
      const button = page.locator('.format-readout');
      const collapsedShellWidth = await page
        .locator('.format-readout-shell')
        .evaluate((shell) => shell.getBoundingClientRect().width);

      await openFormatPopover(page);
      await page.locator('.format-popover-content').hover();
      await expect(button).toHaveAttribute('aria-expanded', 'true');
      await expect
        .poll(() =>
          page
            .locator('.format-readout-shell')
            .evaluate((shell) => shell.getBoundingClientRect().width)
        )
        .toBeCloseTo(collapsedShellWidth, 0);

      const metrics = await readFormatPopover(page);
      expect(metrics.role).toBe('region');
      expect(metrics.processingVisible).toBe(true);
      expect(metrics.surface).toBe(metrics.themePanel);
      expect(metrics.ink).toBe(metrics.themeInk);
      expect(metrics.muted).toBe(metrics.themeMuted);
      expect(metrics.familySignal).toBe(metrics.appSignal);
      expect(metrics.familySignal).not.toBe(metrics.contextSignal);
      expect(metrics.border).toBe(config.formatPopover.border);
      expect(metrics.radiusTopLeft).toBe(config.formatPopover.radiusTopLeft);
      expect(metrics.radiusTopRight).toBe(config.formatPopover.radiusTopRight);
      expect(metrics.backdrop === 'none').toBe(!config.formatPopover.backdrop);
      expect(metrics.headerMarker).toEqual(config.formatPopover.headerMarker);
      expect(metrics.labelFontSize).toBeGreaterThanOrEqual(12);
      expect(metrics.valueFontSize).toBeGreaterThanOrEqual(12);
      expect(metrics.triggerHeight).toBeGreaterThanOrEqual(40);
      expect(metrics.triggerContentVisible).toBe(true);
      expect(metrics.columnCount).toBe(viewport.columns);
      expect(metrics.panelInsideViewport).toBe(true);
      expect(metrics.noHorizontalOverflow).toBe(true);
      expect(metrics.allTextInside).toBe(true);
      expect(metrics.textOverlapsTrigger).toBe(false);
      if (viewport.columns === 1) {
        expect(metrics.secondColumnBorderLeft).toBe(0);
        expect(metrics.secondColumnBorderTop).toBeGreaterThanOrEqual(1);
      } else {
        expect(metrics.secondColumnBorderLeft).toBeGreaterThanOrEqual(1);
        expect(metrics.secondColumnBorderTop).toBe(0);
      }

      await expect(page).toHaveScreenshot(
        `format-popover-${config.family}-${viewport.name}.png`,
        { animations: 'disabled', maxDiffPixelRatio: 0.001 }
      );

      await page.keyboard.press('Escape');
      await expect(button).toHaveAttribute('aria-expanded', 'false');
      await expect(page.locator('#player-format-details')).toBeHidden();
      await expect(button).toBeFocused();

      if (viewport.name === 'desktop') {
        await openFormatPopover(page);
        await page.mouse.click(2, 2);
        await expect(button).toHaveAttribute('aria-expanded', 'false');
        await expect(page.locator('#player-format-details')).toBeHidden();
      }
    }
  });

  test(`${config.family} keeps mute, focus, cancellation, and collapse predictable`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 760, height: 300 });
    await openFixture(page, config.family, config.scheme);

    const button = page.locator('.capsule-icon-btn');
    const stableLabel = await button.getAttribute('aria-label');
    await button.click();
    await expect(button).toHaveAttribute('aria-pressed', 'true');
    await expect(button).toHaveAttribute('aria-label', stableLabel!);
    await expect(button).toBeFocused();

    await openFixture(page, config.family, config.scheme);
    await button.hover();
    await page.waitForTimeout(400);
    await button.click();
    await expect(button).toHaveAttribute('aria-pressed', 'true');
    await expect(button).toHaveAttribute('aria-label', stableLabel!);

    await openFixture(page, config.family, config.scheme);
    await focusSlider(page);
    await page.waitForTimeout(250);
    await expect(page.locator('.capsule-slider')).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(page.locator('.capsule-slider')).toBeFocused();
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'open'
    );

    await previewPosition(page, 0.37);
    await page.evaluate(() => {
      (window as FixtureWindow).__SET_VOLUME_CAPSULE_POSITION__?.(0.64);
    });
    await expectPosition(page, 0.37);
    await page.locator('.capsule-slider').dispatchEvent('pointercancel', {
      pointerId: 1,
      pointerType: 'mouse',
      isPrimary: true,
      buttons: 0,
    });
    await expect(page.locator('.volume-hover-zone')).not.toHaveClass(
      /dragging/
    );
    await expectPosition(page, 0.64);

    await page.mouse.move(2, 2);
    await page
      .locator('.capsule-slider')
      .evaluate((slider: HTMLInputElement) => slider.blur());
    await page.waitForTimeout(COLLAPSE_GRACE_MS - 99);
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'open'
    );
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'closed',
      { timeout: 500 }
    );
  });

  test(`${config.family} preserves the same interaction grace with reduced motion`, async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.setViewportSize({ width: 760, height: 300 });
    await openFixture(page, config.family, config.scheme);

    const button = page.locator('.capsule-icon-btn');
    await button.focus();
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'open'
    );
    const metrics = await readGeometry(page);
    expect(metrics.revealWidth).toBeGreaterThanOrEqual(199);
    expect(metrics.revealOpacity).toBe(1);
    expect(metrics.revealVisibility).toBe('visible');
    await expect(button).toBeFocused();

    await page.mouse.move(2, 2);
    await button.evaluate((element: HTMLButtonElement) => element.blur());
    await page.waitForTimeout(COLLAPSE_GRACE_MS - 99);
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'open'
    );
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'closed',
      { timeout: 500 }
    );
  });

  test(`${config.family} delays playback information close and cancels it on return`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 920, height: 420 });
    await openFixture(page, config.family, config.scheme, {
      processing: true,
    });
    const button = page.locator('.format-readout');
    const popover = page.locator('#player-format-details');

    await button.focus();
    await page.keyboard.press('Enter');
    await page.mouse.move(2, 2);
    await button.evaluate((element: HTMLButtonElement) => element.blur());
    await page.waitForTimeout(COLLAPSE_GRACE_MS - 99);
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await expect(popover).toBeVisible();

    await button.hover();
    await page.waitForTimeout(200);
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await expect(popover).toBeVisible();

    await page.mouse.move(2, 2);
    await page.waitForTimeout(COLLAPSE_GRACE_MS - 99);
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await expect(popover).toBeVisible();
    await expect(button).toHaveAttribute('aria-expanded', 'false', {
      timeout: 500,
    });
    await expect(popover).toBeHidden();
  });
}

test('every family variant keeps format text surface-owned and its signal app-owned', async ({
  page,
}) => {
  await page.setViewportSize({ width: 920, height: 320 });

  for (const config of FAMILIES) {
    for (const scheme of ['light', 'dark'] as const) {
      await openFixture(page, config.family, scheme);
      const ownership = await readFormatTokenOwnership(page);
      expect(ownership.label).toBe('48k/24bit');
      expect(ownership.insideShell).toBe(true);
      expect(ownership.textColor).toBe(ownership.playerControlColor);
      expect(ownership.familySignal).toBe(ownership.appSignal);
      expect(ownership.familySignal).not.toBe(ownership.contextSignal);

      await openFormatPopover(page);
      const popover = await readFormatPopover(page);
      expect(popover.surface).toBe(popover.themePanel);
      expect(popover.ink).toBe(popover.themeInk);
      expect(popover.muted).toBe(popover.themeMuted);
      expect(popover.familySignal).toBe(popover.appSignal);
      expect(popover.familySignal).not.toBe(popover.contextSignal);
      expect(popover.panelInsideViewport).toBe(true);
      expect(popover.noHorizontalOverflow).toBe(true);
      await page.keyboard.press('Escape');
      await expect(page.locator('#player-format-details')).toBeHidden();
    }
  }
});

test('playback information skips reveal motion when reduced motion is requested', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 920, height: 420 });

  for (const config of FAMILIES) {
    await openFixture(page, config.family, config.scheme);
    await openFormatPopover(page);
    const state = await page
      .locator('#player-format-details')
      .evaluate((panel) => ({
        clipPath: getComputedStyle(panel).clipPath,
        opacity: getComputedStyle(panel).opacity,
        transform: getComputedStyle(panel).transform,
      }));
    expect(state).toEqual({
      clipPath: 'none',
      opacity: '1',
      transform: 'none',
    });
    await page.keyboard.press('Escape');
    await expect(page.locator('#player-format-details')).toBeHidden();
  }
});

test('playback information stays closed when format metadata is restored', async ({
  page,
}) => {
  await page.setViewportSize({ width: 920, height: 420 });
  await openFixture(page, 'endfield', 'dark');
  await openFormatPopover(page);

  await page.evaluate(() => {
    (window as FixtureWindow).__SET_VOLUME_CAPSULE_FORMAT_AVAILABLE__?.(false);
  });
  await expect(page.locator('.format-readout')).toHaveCount(0);
  await expect(page.locator('#player-format-details')).toHaveCount(0);

  await page.evaluate(() => {
    (window as FixtureWindow).__SET_VOLUME_CAPSULE_FORMAT_AVAILABLE__?.(true);
  });
  await expect(page.locator('.format-readout')).toBeVisible();
  await expect(page.locator('.format-readout')).toHaveAttribute(
    'aria-expanded',
    'false'
  );
  await expect(page.locator('#player-format-details')).toHaveCount(0);
});

test('format changes remeasure the compact disclosure without interaction', async ({
  page,
}) => {
  await page.setViewportSize({ width: 340, height: 520 });
  await openFixture(page, 'ark', 'dark');
  const shell = page.locator('.format-readout-shell');
  const initialWidth = await shell.evaluate(
    (element) => element.getBoundingClientRect().width
  );

  await page.evaluate(() => {
    (window as FixtureWindow).__SET_VOLUME_CAPSULE_FORMAT_PROCESSING__?.(true);
  });
  await expect(page.locator('.format-text').first()).toHaveText('44.1k/24bit');
  await expect
    .poll(() =>
      shell.evaluate((element) => element.getBoundingClientRect().width)
    )
    .toBeGreaterThan(initialWidth);

  const readVisibility = () =>
    page.evaluate(() => {
      const shellRect = document
        .querySelector<HTMLElement>('.format-readout-shell')!
        .getBoundingClientRect();
      const pillRect = document
        .querySelector<HTMLElement>('.format-pill-source')!
        .getBoundingClientRect();
      const coreRect = document
        .querySelector<HTMLElement>('.format-pill-source-core')!
        .getBoundingClientRect();
      return {
        pillInside:
          pillRect.left >= shellRect.left - 0.5 &&
          pillRect.right <= shellRect.right + 0.5,
        coreInside:
          coreRect.left >= pillRect.left - 0.5 &&
          coreRect.right <= pillRect.right + 0.5,
      };
    });
  await expect.poll(readVisibility).toEqual({
    pillInside: true,
    coreInside: true,
  });

  await page.evaluate(() => {
    (window as FixtureWindow).__SET_VOLUME_CAPSULE_FAMILY__?.('endfield');
  });
  await expect(page.locator('html')).toHaveAttribute(
    'data-ark-theme',
    'endfield'
  );
  await expect.poll(readVisibility).toEqual({
    pillInside: true,
    coreInside: true,
  });
});

test('narrow keyboard disclosure keeps playback information in the viewport', async ({
  page,
}) => {
  await page.setViewportSize({ width: 340, height: 520 });
  await openFixture(page, 'endfield', 'dark', { processing: true });
  const button = page.locator('.format-readout');
  const shell = page.locator('.format-readout-shell');
  const collapsedShellWidth = await shell.evaluate(
    (element) => element.getBoundingClientRect().width
  );

  await button.focus();
  await expect(button).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(button).toHaveAttribute('aria-expanded', 'true');
  await expect(page.locator('#player-format-details')).toBeVisible();
  await expect
    .poll(() =>
      shell.evaluate((element) => element.getBoundingClientRect().width)
    )
    .toBeCloseTo(collapsedShellWidth, 0);

  const metrics = await readFormatPopover(page);
  expect(metrics.panelInsideViewport).toBe(true);
  expect(metrics.triggerContentVisible).toBe(true);

  await page.keyboard.press('Escape');
  await expect(page.locator('#player-format-details')).toHaveCount(0);
  await expect(button).toBeFocused();
});

test('family switching preserves the open DOM, focus, and volume state', async ({
  page,
}) => {
  await page.setViewportSize({ width: 920, height: 320 });
  await openFixture(page, 'ark', 'dark');
  await focusSlider(page);
  await previewPosition(page, 0.37);
  await commitPosition(page);
  await page.locator('.capsule-slider').evaluate((slider) => {
    slider.setAttribute('data-preserved-node', 'true');
  });

  for (const config of FAMILIES.slice(1)) {
    await page.evaluate((family: Family) => {
      (window as FixtureWindow).__SET_VOLUME_CAPSULE_FAMILY__?.(family);
    }, config.family);
    await expect(page.locator('html')).toHaveAttribute(
      'data-ark-theme',
      config.family
    );
    await expect(page.locator('.volume-hover-zone')).toHaveAttribute(
      'data-volume-family',
      config.family
    );
    await expect(page.locator('.capsule-slider')).toHaveAttribute(
      'data-preserved-node',
      'true'
    );
    await expect(page.locator('.capsule-slider')).toBeFocused();
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'open'
    );
    await expectPosition(page, 0.37);
    expect((await readGeometry(page)).revealWidth).toBeGreaterThanOrEqual(199);
  }
});

test('family switching restyles the open playback information without remounting it', async ({
  page,
}) => {
  await page.setViewportSize({ width: 920, height: 420 });
  await openFixture(page, 'ark', 'dark');
  await openFormatPopover(page);
  await page.locator('#player-format-details').evaluate((panel) => {
    panel.setAttribute('data-preserved-popover', 'true');
  });

  for (const [index, config] of FAMILIES.entries()) {
    if (index > 0) {
      await page.evaluate((family: Family) => {
        (window as FixtureWindow).__SET_VOLUME_CAPSULE_FAMILY__?.(family);
      }, config.family);
    }
    await expect(page.locator('html')).toHaveAttribute(
      'data-ark-theme',
      config.family
    );
    await expect(page.locator('.format-readout')).toHaveAttribute(
      'aria-expanded',
      'true'
    );
    await expect(page.locator('#player-format-details')).toHaveAttribute(
      'data-preserved-popover',
      'true'
    );
    await expect(page.locator('.format-readout')).toBeFocused();

    const metrics = await readFormatPopover(page);
    expect(metrics.surface).toBe(metrics.themePanel);
    expect(metrics.familySignal).toBe(metrics.appSignal);
    expect(metrics.familySignal).not.toBe(metrics.contextSignal);
    expect(metrics.triggerContentVisible).toBe(true);
    expect(metrics.border).toBe(config.formatPopover.border);
    expect(metrics.radiusTopLeft).toBe(config.formatPopover.radiusTopLeft);
    expect(metrics.radiusTopRight).toBe(config.formatPopover.radiusTopRight);
  }
});

test('the shared reveal is monotonic and leaves family geometry static', async ({
  page,
}) => {
  await page.setViewportSize({ width: 760, height: 300 });
  await openFixture(page, 'exa', 'dark');
  await page.mouse.move(2, 2);
  await page.evaluate(() => {
    const samples: {
      width: number;
      groupWidth: number;
      opacity: number;
      trackWidth: number;
      transform: string;
    }[] = [];
    (
      window as typeof window & { __VOLUME_REVEAL_SAMPLES__?: typeof samples }
    ).__VOLUME_REVEAL_SAMPLES__ = samples;
    const reveal = document.querySelector<HTMLElement>('.capsule-reveal')!;
    const track = document.querySelector<HTMLElement>('.capsule-track')!;
    const group = document.querySelector<HTMLElement>('.volume-group')!;
    const start = performance.now();
    const sample = () => {
      const revealStyle = getComputedStyle(reveal);
      samples.push({
        width: reveal.getBoundingClientRect().width,
        groupWidth: group.getBoundingClientRect().width,
        opacity: Number(revealStyle.opacity),
        trackWidth: track.getBoundingClientRect().width,
        transform: revealStyle.transform,
      });
      if (performance.now() - start < 300) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });

  await page.locator('.capsule-icon-btn').hover();
  await page.waitForTimeout(340);
  const samples = await page.evaluate(
    () =>
      (
        window as typeof window & {
          __VOLUME_REVEAL_SAMPLES__?: {
            width: number;
            groupWidth: number;
            opacity: number;
            trackWidth: number;
            transform: string;
          }[];
        }
      ).__VOLUME_REVEAL_SAMPLES__ ?? []
  );

  expect(samples.length).toBeGreaterThan(5);
  expect(samples.at(-1)?.width).toBeGreaterThanOrEqual(199);
  for (const [index, sample] of samples.entries()) {
    expect(sample.width).toBeGreaterThanOrEqual(0);
    expect(sample.width).toBeLessThanOrEqual(201);
    expect(sample.width).toBeLessThanOrEqual(sample.groupWidth + 0.5);
    expect(sample.opacity).toBeGreaterThanOrEqual(0);
    expect(sample.opacity).toBeLessThanOrEqual(1);
    expect(sample.trackWidth).toBeCloseTo(200, 0);
    expect(sample.transform).toBe('none');
    if (index === 0) continue;
    expect(sample.width).toBeGreaterThanOrEqual(
      samples[index - 1]!.width - 0.5
    );
    expect(sample.opacity).toBeGreaterThanOrEqual(
      samples[index - 1]!.opacity - 0.01
    );
  }
});

test('focus-driven reveal keeps the mute target under a stationary pointer', async ({
  page,
}) => {
  for (const width of [760, 500, 480, 420]) {
    await page.mouse.move(2, 2);
    await page.setViewportSize({ width, height: 520 });
    await openFixture(page, 'ark', 'dark');

    const button = page.locator('.capsule-icon-btn');
    const box = await button.boundingBox();
    expect(box).not.toBeNull();
    const point = {
      x: box!.x + box!.width / 2,
      y: box!.y + box!.height / 2,
    };

    await page.evaluate(() => {
      const samples: {
        left: number;
        right: number;
        top: number;
        bottom: number;
      }[] = [];
      (
        window as typeof window & {
          __VOLUME_MUTE_TARGET_SAMPLES__?: typeof samples;
        }
      ).__VOLUME_MUTE_TARGET_SAMPLES__ = samples;
      const button = document.querySelector<HTMLElement>('.capsule-icon-btn')!;
      button.addEventListener(
        'mouseenter',
        () => {
          const start = performance.now();
          const sample = () => {
            const rect = button.getBoundingClientRect();
            samples.push({
              left: rect.left,
              right: rect.right,
              top: rect.top,
              bottom: rect.bottom,
            });
            if (performance.now() - start < 300) requestAnimationFrame(sample);
          };
          requestAnimationFrame(sample);
        },
        { once: true }
      );
    });

    await page.mouse.move(point.x, point.y);
    await page.mouse.down();
    await page.waitForTimeout(400);
    await page.mouse.up();

    await expect(button).toHaveAttribute('aria-pressed', 'true');
    const finalBox = await button.boundingBox();
    expect(finalBox).not.toBeNull();
    expect(point.x).toBeGreaterThanOrEqual(finalBox!.x);
    expect(point.x).toBeLessThanOrEqual(finalBox!.x + finalBox!.width);
    expect(point.y).toBeGreaterThanOrEqual(finalBox!.y);
    expect(point.y).toBeLessThanOrEqual(finalBox!.y + finalBox!.height);

    const targetSamples = await page.evaluate(
      () =>
        (
          window as typeof window & {
            __VOLUME_MUTE_TARGET_SAMPLES__?: {
              left: number;
              right: number;
              top: number;
              bottom: number;
            }[];
          }
        ).__VOLUME_MUTE_TARGET_SAMPLES__ ?? []
    );
    expect(targetSamples.length).toBeGreaterThan(5);
    for (const sample of targetSamples) {
      expect(point.x).toBeGreaterThanOrEqual(sample.left - 0.5);
      expect(point.x).toBeLessThanOrEqual(sample.right + 0.5);
      expect(point.y).toBeGreaterThanOrEqual(sample.top - 0.5);
      expect(point.y).toBeLessThanOrEqual(sample.bottom + 0.5);
    }
  }
});

test('coarse pointers get persistent space and direct slider access', async ({
  browser,
}) => {
  const context = await createTouchContext(browser);
  const page = await context.newPage();
  try {
    await openFixture(page, 'ark', 'dark');
    expect(
      await page.evaluate(
        () => window.matchMedia('(hover: none) and (pointer: coarse)').matches
      )
    ).toBe(true);
    await expect(page.locator('.volume-wrapper')).toHaveAttribute(
      'data-state',
      'open'
    );
    await expect(page.locator('.capsule-slider')).toHaveAttribute(
      'tabindex',
      '0'
    );

    const metrics = await readGeometry(page);
    expect(metrics.revealWidth).toBeGreaterThanOrEqual(199);
    expect(metrics.volumeGroupWidth).toBeCloseTo(200, 0);
    expect(metrics.buttonOverlapsPreviousControl).toBe(false);
    expect(metrics.revealOverlapsPreviousControl).toBe(false);
    expect(metrics.noHorizontalOverflow).toBe(true);

    const slider = page.locator('.capsule-slider');
    await slider.tap({ position: { x: 12, y: 20 } });
    await expect
      .poll(async () => (await readPosition(page)).inputPercent)
      .not.toBe(64);

    const button = page.locator('.capsule-icon-btn');
    const label = await button.getAttribute('aria-label');
    await button.tap();
    await expect(button).toHaveAttribute('aria-pressed', 'true');
    await expect(button).toHaveAttribute('aria-label', label!);
  } finally {
    await context.close();
  }
});

test('legacy families keep compact volume controls on coarse pointers', async ({
  browser,
}) => {
  for (const family of ['glass', 'material', 'terminal'] as const) {
    const context = await createTouchContext(browser);
    const page = await context.newPage();
    try {
      await openFixture(page, family, 'dark');
      expect(
        await page.evaluate(
          () => window.matchMedia('(hover: none) and (pointer: coarse)').matches
        )
      ).toBe(true);

      const metrics = await page.evaluate(() => {
        const group = document.querySelector<HTMLElement>('.volume-group')!;
        const button =
          document.querySelector<HTMLElement>('.capsule-icon-btn')!;
        const slider =
          document.querySelector<HTMLInputElement>('.capsule-slider')!;
        return {
          family: document.documentElement.dataset.arkTheme,
          groupWidth: group.getBoundingClientRect().width,
          buttonWidth: button.getBoundingClientRect().width,
          sliderTabIndex: slider.tabIndex,
          noHorizontalOverflow:
            document.documentElement.scrollWidth <= window.innerWidth,
        };
      });

      expect(metrics.family).toBe(family);
      expect(metrics.groupWidth).toBeLessThanOrEqual(metrics.buttonWidth + 1);
      expect(metrics.sliderTabIndex).toBe(-1);
      expect(metrics.noHorizontalOverflow).toBe(true);
    } finally {
      await context.close();
    }
  }
});

async function createTouchContext(browser: Browser) {
  return browser.newContext({
    viewport: { width: 420, height: 520 },
    hasTouch: true,
    isMobile: true,
  });
}
