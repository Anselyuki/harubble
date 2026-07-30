// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from 'vitest';
import { applyMotionOverride } from '$lib/design/gsap';
import { transitionCssVariables } from '$lib/theme';
import {
  cancelThemePackageTransition,
  runThemePackageTransition,
} from './themePackageTransition';

describe('theme package transition', () => {
  afterEach(() => {
    cancelThemePackageTransition();
    applyMotionOverride(null);
    document
      .querySelectorAll('.theme-package-transition')
      .forEach((element) => element.remove());
    delete document.documentElement.dataset.themePackageTransition;
  });

  it('covers the current UI before committing and removes the mask after reveal', async () => {
    applyMotionOverride({ BASE: 20, BASE_OUT: 200 });
    const commit = vi.fn();

    const finished = runThemePackageTransition(commit, { reason: 'preview' });

    expect(commit).not.toHaveBeenCalled();
    expect(document.documentElement.dataset.themePackageTransition).toBe(
      'cover'
    );
    expect(
      document.querySelector<HTMLElement>('.theme-package-transition')?.dataset
        .reason
    ).toBe('preview');
    expect(
      document.querySelector('.theme-package-transition__veil')
    ).not.toBeNull();
    expect(
      document.querySelector('.theme-package-transition__signal')
    ).not.toBeNull();

    await vi.waitFor(() => expect(commit).toHaveBeenCalledTimes(1));
    expect(document.documentElement.dataset.themePackageTransition).toBe(
      'reveal'
    );

    await finished;
    expect(document.querySelector('.theme-package-transition')).toBeNull();
    expect(
      document.documentElement.dataset.themePackageTransition
    ).toBeUndefined();
  });

  it('cancels an older cover so rapid previews only commit the latest theme', async () => {
    applyMotionOverride({ BASE: 20, BASE_OUT: 20 });
    const firstCommit = vi.fn();
    const secondCommit = vi.fn();

    const firstFinished = runThemePackageTransition(firstCommit);
    const secondFinished = runThemePackageTransition(secondCommit);
    await Promise.all([firstFinished, secondFinished]);

    expect(firstCommit).not.toHaveBeenCalled();
    expect(secondCommit).toHaveBeenCalledTimes(1);
    expect(document.querySelector('.theme-package-transition')).toBeNull();
  });

  it('keeps only the latest intent across rapid preview, activate, and dismiss', async () => {
    applyMotionOverride({ BASE: 20, BASE_OUT: 20 });
    const previewCommit = vi.fn();
    const activateCommit = vi.fn();
    const dismissCommit = vi.fn();

    const previewFinished = runThemePackageTransition(previewCommit, {
      reason: 'preview',
    });
    const activateFinished = runThemePackageTransition(activateCommit, {
      reason: 'activate',
    });
    const dismissFinished = runThemePackageTransition(dismissCommit, {
      reason: 'dismiss-preview',
    });

    expect(
      document.querySelector<HTMLElement>('.theme-package-transition')?.dataset
        .reason
    ).toBe('dismiss-preview');
    await Promise.all([previewFinished, activateFinished, dismissFinished]);

    expect(previewCommit).not.toHaveBeenCalled();
    expect(activateCommit).not.toHaveBeenCalled();
    expect(dismissCommit).toHaveBeenCalledTimes(1);
    expect(document.querySelector('.theme-package-transition')).toBeNull();
  });

  it('commits synchronously without a mask when animation is disabled', async () => {
    const commit = vi.fn();

    await runThemePackageTransition(commit, { animate: false });

    expect(commit).toHaveBeenCalledTimes(1);
    expect(document.querySelector('.theme-package-transition')).toBeNull();
  });

  it('keeps a committed same-target reveal during an idempotent update', async () => {
    applyMotionOverride({ BASE: 20, BASE_OUT: 200 });
    const firstCommit = vi.fn();
    const replayCommit = vi.fn();
    const finished = runThemePackageTransition(firstCommit, {
      targetPackageId: 'active',
    });

    await vi.waitFor(() => expect(firstCommit).toHaveBeenCalledOnce());
    const revealingOverlay = document.querySelector<HTMLElement>(
      '.theme-package-transition'
    );
    expect(revealingOverlay?.dataset.phase).toBe('reveal');

    await runThemePackageTransition(replayCommit, {
      animate: false,
      targetPackageId: 'active',
    });

    expect(replayCommit).toHaveBeenCalledOnce();
    expect(document.querySelector('.theme-package-transition')).toBe(
      revealingOverlay
    );
    expect(document.documentElement.dataset.themePackageTransition).toBe(
      'reveal'
    );

    await finished;
    expect(document.querySelector('.theme-package-transition')).toBeNull();
  });

  it('absorbs pointer input over the painted mask without bubbling to the app', () => {
    applyMotionOverride({ BASE: 200, BASE_OUT: 200 });
    const appClick = vi.fn();
    document.body.addEventListener('click', appClick);
    void runThemePackageTransition(() => {});
    const overlay = document.querySelector<HTMLElement>(
      '.theme-package-transition'
    );
    const click = new MouseEvent('click', {
      bubbles: true,
      cancelable: true,
    });

    expect(overlay).not.toBeNull();
    overlay!.dispatchEvent(click);

    expect(click.defaultPrevented).toBe(true);
    expect(appClick).not.toHaveBeenCalled();
    document.body.removeEventListener('click', appClick);
  });

  it('rejects and removes the mask when the midpoint commit fails', async () => {
    applyMotionOverride({ BASE: 20, BASE_OUT: 20 });
    const failure = new Error('theme commit failed');

    const finished = runThemePackageTransition(() => {
      throw failure;
    });

    await expect(finished).rejects.toBe(failure);
    expect(document.querySelector('.theme-package-transition')).toBeNull();
    expect(
      document.documentElement.dataset.themePackageTransition
    ).toBeUndefined();
  });

  it('kills an old color tween and commits the midpoint token write atomically', async () => {
    const root = document.documentElement;
    root.style.setProperty('--theme-transition-test', '#000000');
    transitionCssVariables(
      { '--theme-transition-test': '#ffffff' },
      'theme-transition-test'
    );

    await runThemePackageTransition(
      () => {
        transitionCssVariables(
          { '--theme-transition-test': '#123456' },
          'theme-transition-test'
        );
      },
      { animate: false, reason: 'activate' }
    );

    expect(root.style.getPropertyValue('--theme-transition-test')).toBe(
      '#123456'
    );
    await new Promise((resolve) => setTimeout(resolve, 40));
    expect(root.style.getPropertyValue('--theme-transition-test')).toBe(
      '#123456'
    );
    root.style.removeProperty('--theme-transition-test');
  });
});
