import { expect, test } from '@playwright/test';

const FIXTURE_URL =
  'http://127.0.0.1:1421/tests/e2e/visual/fixtures/theme-package-library-surface-fixture.html';
const LONG_VALID_PACKAGE_ID = 'a'.repeat(64);

for (const scheme of ['light', 'dark'] as const) {
  test(`endfield ${scheme} keeps theme package settings readable`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 640, height: 900 });
    await page.goto(`${FIXTURE_URL}?scheme=${scheme}`);

    const fixture = page.getByTestId('endfield-settings-fixture');
    await expect(fixture).toHaveAttribute('data-scheme', scheme);
    await expect(page.getByText('Field Signal', { exact: true })).toBeVisible();
    await expect(page.getByTestId('endfield-button-contract')).toBeVisible();

    const sharedButtonMetrics = await page.evaluate(() => {
      const selectors = [
        '[data-testid="qa-button-primary"]',
        '[data-testid="qa-button-secondary"]',
        '[data-testid="qa-button-outline"]',
        '[data-testid="qa-button-danger"]',
        '.qa-button-contract .settings-segment button:first-child',
        '.qa-button-contract .settings-segment button:last-child',
      ];
      return selectors.map((selector) => {
        const element = document.querySelector<HTMLButtonElement>(selector);
        if (!element) throw new Error(`Missing shared button: ${selector}`);
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
          selector,
          height: rect.height,
          radius: style.borderRadius,
          borderWidth: style.borderWidth,
          borderStyle: style.borderStyle,
          clipPath: style.clipPath,
          fontWeight: style.fontWeight,
        };
      });
    });
    expect(
      sharedButtonMetrics.slice(0, 4).map((button) => button.height)
    ).toEqual([40, 40, 40, 40]);
    expect(
      sharedButtonMetrics.slice(0, 4).every((button) => button.radius === '0px')
    ).toBe(true);
    expect(
      sharedButtonMetrics
        .slice(0, 4)
        .every(
          (button) =>
            button.borderWidth === '1px' && button.borderStyle === 'solid'
        )
    ).toBe(true);
    expect(
      sharedButtonMetrics
        .slice(0, 4)
        .every((button) => button.clipPath === 'none')
    ).toBe(true);
    expect(
      sharedButtonMetrics
        .slice(4)
        .every((button) => button.height === 36 && button.radius === '0px')
    ).toBe(true);

    const metrics = await page.evaluate(() => {
      type Rgba = [number, number, number, number];
      const parseColor = (value: string): Rgba => {
        const channels = value.match(/[\d.]+/g)?.map(Number) ?? [];
        if (channels.length < 3) throw new Error(`Cannot parse ${value}`);
        return [channels[0]!, channels[1]!, channels[2]!, channels[3] ?? 1];
      };
      const composite = (foreground: Rgba, background: Rgba): Rgba => {
        const alpha = foreground[3] + background[3] * (1 - foreground[3]);
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
        const values = color.slice(0, 3).map((channel) => {
          const value = channel / 255;
          return value <= 0.03928
            ? value / 12.92
            : Math.pow((value + 0.055) / 1.055, 2.4);
        });
        return 0.2126 * values[0]! + 0.7152 * values[1]! + 0.0722 * values[2]!;
      };
      const contrast = (foreground: Rgba, background: Rgba): number => {
        const paintedForeground = composite(foreground, background);
        const first = luminance(paintedForeground);
        const second = luminance(background);
        return (
          (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05)
        );
      };

      const item = document.querySelector<HTMLElement>('.package-item')!;
      const itemStyle = getComputedStyle(item);
      const itemBackground = parseColor(itemStyle.backgroundColor);
      const textContrast = (selector: string): number =>
        contrast(
          parseColor(
            getComputedStyle(item.querySelector<HTMLElement>(selector)!).color
          ),
          itemBackground
        );
      const input = document.querySelector<HTMLInputElement>('.url-input')!;
      const inputStyle = getComputedStyle(input);
      const inputBackground = parseColor(inputStyle.backgroundColor);
      const preview = item.querySelector<HTMLButtonElement>('.btn-small')!;
      const previewStyle = getComputedStyle(preview);
      const previewBackground = parseColor(previewStyle.backgroundColor);

      return {
        nameContrast: textContrast('.package-name'),
        versionContrast: textContrast('.package-version'),
        idContrast: textContrast('.package-id'),
        inputContrast: contrast(parseColor(inputStyle.color), inputBackground),
        placeholderContrast: contrast(
          parseColor(getComputedStyle(input, '::placeholder').color),
          inputBackground
        ),
        previewContrast: contrast(
          parseColor(previewStyle.color),
          previewBackground
        ),
        itemLuminance: luminance(itemBackground),
        inputLuminance: luminance(inputBackground),
        noHorizontalOverflow:
          document.documentElement.scrollWidth <= window.innerWidth,
      };
    });

    expect(metrics.nameContrast).toBeGreaterThanOrEqual(4.5);
    expect(metrics.versionContrast).toBeGreaterThanOrEqual(4.5);
    expect(metrics.idContrast).toBeGreaterThanOrEqual(4.5);
    expect(metrics.inputContrast).toBeGreaterThanOrEqual(4.5);
    expect(metrics.placeholderContrast).toBeGreaterThanOrEqual(4.5);
    expect(metrics.previewContrast).toBeGreaterThanOrEqual(4.5);
    expect(metrics.itemLuminance).toBeLessThan(0.1);
    expect(metrics.inputLuminance).toBeLessThan(0.1);
    expect(metrics.noHorizontalOverflow).toBe(true);

    const importButton = page.getByTestId('theme-package-import');
    await importButton.focus();
    await expect(importButton).toBeFocused();
    expect(
      await importButton.evaluate(
        (button) => getComputedStyle(button).outlineOffset
      )
    ).toBe('2px');

    const actionGeometry = await page.evaluate(() => {
      const selectors = [
        '[data-testid="theme-package-import"]',
        '.toolbar .btn-secondary',
        '.package-actions .btn-small:not(.btn-primary)',
        '.package-actions .btn-small.btn-primary',
        '.package-actions .btn-danger',
      ];
      return selectors.map((selector) => {
        const element = document.querySelector<HTMLButtonElement>(selector);
        if (!element) throw new Error(`Missing button: ${selector}`);
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
          selector,
          height: rect.height,
          radius: style.borderRadius,
          borderWidth: style.borderWidth,
          borderStyle: style.borderStyle,
          clipPath: style.clipPath,
          fontWeight: style.fontWeight,
        };
      });
    });
    const regularButtons = actionGeometry.slice(0, 2);
    expect(regularButtons.map((button) => button.height)).toEqual([40, 40]);
    expect(regularButtons.map((button) => button.radius)).toEqual([
      '0px',
      '0px',
    ]);
    expect(regularButtons.map((button) => button.borderWidth)).toEqual([
      '1px',
      '1px',
    ]);
    expect(regularButtons.map((button) => button.borderStyle)).toEqual([
      'solid',
      'solid',
    ]);
    expect(
      actionGeometry.slice(2).every((button) => button.height === 36)
    ).toBe(true);
    expect(
      actionGeometry.slice(2).every((button) => button.radius === '0px')
    ).toBe(true);
    expect(
      actionGeometry.slice(2).every((button) => button.borderWidth === '1px')
    ).toBe(true);
    expect(
      actionGeometry.slice(2).every((button) => button.clipPath === 'none')
    ).toBe(true);

    await page
      .getByTestId('theme-package-item')
      .first()
      .getByRole('button', { name: '预览' })
      .click();
    await expect(
      page.getByTestId('theme-package-dismiss-preview')
    ).toBeVisible();
    await expect(fixture).toHaveAttribute(
      'data-active-package-id',
      'ark-ui-endfield'
    );
    await expect(fixture).toHaveAttribute('data-previewing-id', 'ark-ui-ark');

    await page.getByTestId('theme-package-library-disable').click();

    await expect(page.getByTestId('theme-package-library-section')).toHaveCount(
      0
    );
    await expect(
      page.getByTestId('theme-package-library-collapsed')
    ).toBeVisible();
    await expect(
      page.getByTestId('theme-package-library-enable')
    ).toBeFocused();
    await expect(fixture).toHaveAttribute(
      'data-active-package-id',
      'ark-ui-endfield'
    );
    await expect(fixture).toHaveAttribute('data-previewing-id', 'ark-ui-ark');
    expect(
      await page.evaluate(() => localStorage.getItem('theme_packages_v1'))
    ).toBe('0');

    await page.getByTestId('theme-package-library-enable').click();

    await expect(
      page.getByTestId('theme-package-library-section')
    ).toBeVisible();
    await expect(
      page.getByTestId('theme-package-library-disable')
    ).toBeFocused();
    await expect(
      page
        .getByTestId('theme-package-item')
        .filter({ has: page.getByText('Field Signal', { exact: true }) })
    ).toHaveClass(/package-item--active/);
    await expect(
      page.getByTestId('theme-package-dismiss-preview')
    ).toBeVisible();
    await expect(fixture).toHaveAttribute(
      'data-active-package-id',
      'ark-ui-endfield'
    );
    await expect(fixture).toHaveAttribute('data-previewing-id', 'ark-ui-ark');
    expect(
      await page.evaluate(() => localStorage.getItem('theme_packages_v1'))
    ).toBe('1');
  });
}

test('active and preview package states stay mutually exclusive', async ({
  page,
}) => {
  await page.setViewportSize({ width: 640, height: 900 });
  await page.goto(`${FIXTURE_URL}?scheme=light`);

  const fixture = page.getByTestId('endfield-settings-fixture');
  const activeItem = page
    .getByTestId('theme-package-item')
    .filter({ has: page.getByText('Field Signal', { exact: true }) });
  const previewItem = page
    .getByTestId('theme-package-item')
    .filter({ has: page.getByText('Industrial Cyan', { exact: true }) });

  await activeItem.getByRole('button', { name: '预览', exact: true }).click();
  await expect(fixture).toHaveAttribute('data-previewing-id', '');
  await expect(activeItem).toHaveAttribute('data-active', 'true');
  await expect(activeItem).toHaveAttribute('data-previewing', 'false');
  await expect(
    activeItem.getByTestId('theme-package-preview-badge')
  ).toHaveCount(0);
  await expect(
    activeItem.getByRole('button', { name: '预览', exact: true })
  ).toHaveAttribute('aria-pressed', 'false');

  await previewItem.getByRole('button', { name: '预览', exact: true }).click();
  await expect(fixture).toHaveAttribute('data-previewing-id', 'ark-ui-ark');
  await activeItem.getByRole('button', { name: '预览', exact: true }).click();
  await expect(fixture).toHaveAttribute('data-previewing-id', '');
  await expect(
    activeItem.getByTestId('theme-package-preview-badge')
  ).toHaveCount(0);

  await previewItem.getByRole('button', { name: '预览', exact: true }).click();
  await previewItem.getByRole('button', { name: '应用', exact: true }).click();
  await expect(fixture).toHaveAttribute('data-active-package-id', 'ark-ui-ark');
  await expect(fixture).toHaveAttribute('data-previewing-id', '');
  await expect(previewItem).toHaveAttribute('data-active', 'true');
  await expect(previewItem).toHaveAttribute('data-previewing', 'false');
  await expect(
    previewItem.getByTestId('theme-package-preview-badge')
  ).toHaveCount(0);
});

test('long valid package ids stay clear of package actions', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto(`${FIXTURE_URL}?scheme=light`);

  const fixture = page.getByTestId('endfield-settings-fixture');
  const item = page.locator(
    `[data-testid="theme-package-item"][data-package-id="${LONG_VALID_PACKAGE_ID}"]`
  );
  await expect(item).toBeVisible();

  const itemMetrics = await item.evaluate((element) => {
    const meta = element.querySelector<HTMLElement>('.package-meta')!;
    const actions = element.querySelector<HTMLElement>('.package-actions')!;
    const id = element.querySelector<HTMLElement>('.package-id')!;
    const itemRect = element.getBoundingClientRect();
    const metaRect = meta.getBoundingClientRect();
    const actionsRect = actions.getBoundingClientRect();

    return {
      metaRight: metaRect.right,
      actionsLeft: actionsRect.left,
      actionsRight: actionsRect.right,
      itemRight: itemRect.right,
      idFits: id.scrollWidth <= id.clientWidth,
    };
  });

  expect(itemMetrics.metaRight).toBeLessThanOrEqual(itemMetrics.actionsLeft);
  expect(itemMetrics.actionsRight).toBeLessThanOrEqual(itemMetrics.itemRight);
  expect(itemMetrics.idFits).toBe(true);

  await item.getByRole('button', { name: '预览', exact: true }).click();
  await expect(fixture).toHaveAttribute(
    'data-previewing-id',
    LONG_VALID_PACKAGE_ID
  );

  const dismiss = page.getByTestId('theme-package-dismiss-preview');
  await expect(dismiss).toBeVisible();
  const dismissMetrics = await dismiss.evaluate((button) => {
    const buttonRect = button.getBoundingClientRect();
    const toolbarRect = button.parentElement!.getBoundingClientRect();
    return {
      buttonLeft: buttonRect.left,
      buttonRight: buttonRect.right,
      toolbarLeft: toolbarRect.left,
      toolbarRight: toolbarRect.right,
    };
  });
  expect(dismissMetrics.buttonLeft).toBeGreaterThanOrEqual(
    dismissMetrics.toolbarLeft
  );
  expect(dismissMetrics.buttonRight).toBeLessThanOrEqual(
    dismissMetrics.toolbarRight
  );

  await dismiss.click();
  await expect(fixture).toHaveAttribute('data-previewing-id', '');
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth
    )
  ).toBe(true);
});
