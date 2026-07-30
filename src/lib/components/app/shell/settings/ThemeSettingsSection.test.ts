// @vitest-environment jsdom

import { cleanup, render, waitFor } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ThemeColorSlots } from '$lib/themePresets';

const motion = vi.hoisted(() => {
  let pendingExit: (() => void) | undefined;
  return {
    animateIn: vi.fn(
      (
        _target: Element,
        _fromVars: Record<string, unknown>,
        toVars: { onComplete?: () => void }
      ) => toVars.onComplete?.()
    ),
    animateOut: vi.fn(
      (
        _target: Element,
        _toVars: Record<string, unknown>,
        _duration: number,
        options?: { onComplete?: () => void }
      ) => {
        pendingExit = options?.onComplete;
      }
    ),
    completeExit: () => {
      pendingExit?.();
      pendingExit = undefined;
    },
    set: vi.fn(),
    to: vi.fn((_target: Element, vars: { onComplete?: () => void }) =>
      vars.onComplete?.()
    ),
    killTweens: vi.fn(),
    getMotionDuration: vi.fn((durationMs: number) => durationMs / 1000),
  };
});

vi.mock('$lib/design/gsap', () => ({
  animateIn: motion.animateIn,
  animateOut: motion.animateOut,
  getMotionDuration: motion.getMotionDuration,
  gsap: { set: motion.set, to: motion.to },
  killTweens: motion.killTweens,
  MOTION: { SLOW: 260, SLOW_OUT: 200 },
}));

import ThemeSettingsSection from './ThemeSettingsSection.svelte';

const COLORS: ThemeColorSlots = {
  accent: '#7C3AED',
  surface: '#FFFFFF',
  textPrimary: '#111111',
  textSecondary: '#666666',
  tint: '#808080',
  danger: '#CC2233',
};

function props(packageColorsLocked: boolean) {
  return {
    packageColorsLocked,
    themePresetId: 'harubble-classic',
    resolvedThemeColors: COLORS,
    themePresetOptions: [
      {
        id: 'harubble-classic',
        label: 'Harubble Classic',
        description: 'Default',
        colors: COLORS,
      },
    ],
    currentThemePresetLabel: 'Harubble Classic',
    getThemeDraft: (slot: keyof ThemeColorSlots) => COLORS[slot],
    getSlotLabel: (slot: keyof ThemeColorSlots) => slot,
    isValidThemeHex: () => true,
    sectionTitle: 'Theme',
    themePresetLabel: 'Theme color',
    themeResetLabel: 'Reset',
    themeResetTitle: 'Reset theme colors',
    themeHexInvalidLabel: 'Invalid color',
    appearanceLabel: 'Appearance',
    appearanceAutoLabel: 'Auto',
    appearanceLightLabel: 'Light',
    appearanceDarkLabel: 'Dark',
    appearanceSegmentAria: 'Appearance mode',
    dynamicAlbumLabel: 'Dynamic album color',
    dynamicAlbumOnLabel: 'On',
    dynamicAlbumOffLabel: 'Off',
    onThemePresetChange: vi.fn(),
    onThemeTextInput: vi.fn(),
    onThemeColorInput: vi.fn(),
    onResetThemeCustomColors: vi.fn(),
  };
}

describe('ThemeSettingsSection package color ownership', () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('starts with package-owned color controls hidden while keeping scheme controls', () => {
    const view = render(ThemeSettingsSection, { props: props(true) });

    expect(
      view.queryByTestId('theme-color-settings-panel')
    ).not.toBeInTheDocument();
    expect(view.getByRole('button', { name: 'Auto' })).toBeInTheDocument();
    expect(view.getByRole('button', { name: 'On' })).toBeInTheDocument();
    expect(motion.animateOut).not.toHaveBeenCalled();
  });

  it('collapses on package selection and expands after clearing the package', async () => {
    const view = render(ThemeSettingsSection, { props: props(false) });
    const colorPanel = view.getByTestId('theme-color-settings-panel');
    const accentInput = view.container.querySelector<HTMLInputElement>(
      '#theme-color-accent'
    );
    expect(accentInput).not.toBeNull();
    accentInput?.focus();

    await view.rerender(props(true));
    expect(colorPanel).toHaveAttribute('aria-hidden', 'true');
    expect(colorPanel).toHaveProperty('inert', true);
    expect(view.getByRole('button', { name: 'Auto' })).toHaveFocus();
    motion.completeExit();
    await waitFor(() => {
      expect(
        view.queryByTestId('theme-color-settings-panel')
      ).not.toBeInTheDocument();
    });
    expect(motion.animateOut).toHaveBeenCalledWith(
      expect.any(HTMLElement),
      expect.objectContaining({ height: 0, opacity: 0, y: -8 }),
      200,
      expect.objectContaining({ ease: 'ios-in' })
    );

    await view.rerender(props(false));
    await waitFor(() => {
      expect(
        view.getByTestId('theme-color-settings-panel')
      ).toBeInTheDocument();
    });
    expect(motion.animateIn).toHaveBeenCalledWith(
      expect.any(HTMLElement),
      expect.objectContaining({ height: 0, marginTop: 0, opacity: 0, y: -8 }),
      expect.objectContaining({
        height: 'auto',
        marginTop: 12,
        opacity: 1,
        y: 0,
      }),
      260,
      'ios-out'
    );
  });

  it('reverses an in-flight collapse from its current frame', async () => {
    const view = render(ThemeSettingsSection, { props: props(false) });

    await view.rerender(props(true));
    expect(motion.animateOut).toHaveBeenCalledOnce();

    await view.rerender(props(false));
    await waitFor(() => expect(motion.to).toHaveBeenCalledOnce());
    expect(motion.animateIn).not.toHaveBeenCalled();
    expect(motion.to).toHaveBeenCalledWith(
      expect.any(HTMLElement),
      expect.objectContaining({
        height: 'auto',
        marginTop: 12,
        duration: 0.26,
      })
    );

    // A stale exit completion must not unmount the panel after reversal.
    motion.completeExit();
    expect(view.getByTestId('theme-color-settings-panel')).toBeInTheDocument();
  });
});
