import { describe, expect, test } from 'vitest';
import {
  chainTimelineComplete,
  getCenterLockTransform,
  getLogoFlipTargets,
  getLogoGlyphClearProps,
  getLogoSlotClearProps,
  getLogoCenterPinFrame,
  getLogoCenterPinX,
  getPinnedLogoWidthFrame,
  collectSidebarAnimatorLabelEls,
  collectSidebarItemLabelEls,
  resolveLogoGlyphEl,
} from './sidebar-animator';

describe('getLogoCenterPinX', () => {
  test('offsets the collapsed logo left by half the width growth during expand', () => {
    expect(getLogoCenterPinX(56, 56)).toBe(0);
    expect(getLogoCenterPinX(56, 152)).toBe(-48);
    expect(getLogoCenterPinX(56, 248)).toBe(-96);
  });

  test('keeps the collapsed logo center stable when width and x share progress', () => {
    const frames = [0, 0.25, 0.5, 0.75, 1].map((progress) =>
      getLogoCenterPinFrame(56, 248, progress)
    );

    expect(frames).toEqual([
      { width: 56, x: 0 },
      { width: 104, x: -24 },
      { width: 152, x: -48 },
      { width: 200, x: -72 },
      { width: 248, x: -96 },
    ]);

    frames.forEach((frame) => {
      expect(frame.width / 2 + frame.x).toBe(28);
    });
  });

  test('keeps the logo box fixed during sidebar width expansion', () => {
    const frames = [0, 0.25, 0.5, 0.75, 1].map((progress) =>
      getPinnedLogoWidthFrame(56, 248, progress)
    );

    expect(frames).toEqual([
      { sidebarWidth: 56, logoWidth: 56, alignSelf: 'flex-start' },
      { sidebarWidth: 104, logoWidth: 56, alignSelf: 'flex-start' },
      { sidebarWidth: 152, logoWidth: 56, alignSelf: 'flex-start' },
      { sidebarWidth: 200, logoWidth: 56, alignSelf: 'flex-start' },
      { sidebarWidth: 248, logoWidth: 56, alignSelf: 'flex-start' },
    ]);
  });

  test('pins the logo to its measured collapsed width', () => {
    expect(getPinnedLogoWidthFrame(56, 248, 0.5, 55)).toEqual({
      sidebarWidth: 152,
      logoWidth: 55,
      alignSelf: 'flex-start',
    });
  });
});

describe('getCenterLockTransform', () => {
  test('adds the center correction to the existing transform offset', () => {
    expect(
      getCenterLockTransform(
        { left: 10, top: 20, width: 20, height: 10 },
        { left: 8, top: 18, width: 28, height: 16 },
        { x: -1, y: 0.5 }
      )
    ).toEqual({ x: -3, y: -0.5 });
  });
});

describe('resolveLogoGlyphEl', () => {
  test('uses the nested glyph as the rotation target when present', () => {
    const glyph = {} as HTMLElement;
    const char = {
      querySelector: (selector: string) =>
        selector === '[data-logo-glyph]' ? glyph : null,
    } as unknown as HTMLSpanElement;

    expect(resolveLogoGlyphEl(char)).toBe(glyph);
  });

  test('falls back to the character element for the old markup', () => {
    const char = {
      querySelector: () => null,
    } as unknown as HTMLSpanElement;

    expect(resolveLogoGlyphEl(char)).toBe(char);
  });
});

describe('getLogoFlipTargets', () => {
  test('uses only stable character slots for FLIP', () => {
    const charA = {} as HTMLSpanElement;
    const charB = {} as HTMLSpanElement;
    const glyphA = {} as HTMLElement;
    const glyphB = {} as HTMLElement;

    expect(getLogoFlipTargets([charA, charB], [glyphA, glyphB])).toEqual([
      charA,
      charB,
    ]);
  });
});

describe('logo FLIP cleanup props', () => {
  test('clears absolute FLIP styles from character slots', () => {
    expect(getLogoSlotClearProps()).toBe('all');
  });

  test('preserves collapsed glyph rotation while expanded glyphs become clean', () => {
    expect(getLogoGlyphClearProps('collapsed')).toContain('position');
    expect(getLogoGlyphClearProps('collapsed')).not.toContain('transform');
    expect(getLogoGlyphClearProps('collapsed')).not.toContain('rotate');
    expect(getLogoGlyphClearProps('collapsed')).not.toContain('translate');
    expect(getLogoGlyphClearProps('expanded')).toContain('transform');
  });
});

describe('chainTimelineComplete', () => {
  test('preserves an existing onComplete callback before resolving', async () => {
    const calls: string[] = [];
    let onComplete: (() => void) | undefined = () => {
      calls.push('existing');
    };
    const timeline = {
      totalDuration: () => 1,
      progress: () => 0,
      eventCallback: (name: string, callback?: () => void) => {
        if (name !== 'onComplete') return undefined;
        if (callback === undefined) return onComplete;
        onComplete = callback;
        return undefined;
      },
    };

    const promise = chainTimelineComplete(timeline).then(() => {
      calls.push('resolved');
    });
    onComplete();
    await promise;

    expect(calls).toEqual(['existing', 'resolved']);
  });
});

describe('collectSidebarItemLabelEls', () => {
  test('collects region labels before extra labels and removes duplicates', () => {
    const navLabel = {} as HTMLSpanElement;
    const collectionLabel = {} as HTMLSpanElement;
    const bottomLabel = {} as HTMLSpanElement;
    const navRegion = {
      querySelectorAll: (selector: string) =>
        selector === '[data-sidebar-item-label]' ? [navLabel] : [],
    } as unknown as HTMLElement;
    const collectionsRegion = {
      querySelectorAll: (selector: string) =>
        selector === '[data-sidebar-item-label]'
          ? [collectionLabel, bottomLabel]
          : [],
    } as unknown as HTMLElement;

    expect(
      collectSidebarItemLabelEls([navRegion, collectionsRegion], [bottomLabel])
    ).toEqual([navLabel, collectionLabel, bottomLabel]);
  });
});

describe('collectSidebarAnimatorLabelEls', () => {
  test('collects nav, collection, and bottom tag labels for shared sidebar animation', () => {
    const navLabel = {} as HTMLSpanElement;
    const collectionLabel = {} as HTMLSpanElement;
    const bottomLabel = {} as HTMLSpanElement;
    const navRegionEl = {
      querySelectorAll: (selector: string) =>
        selector === '[data-sidebar-item-label]' ? [navLabel] : [],
    } as unknown as HTMLElement;
    const collectionsRegionEl = {
      querySelectorAll: (selector: string) =>
        selector === '[data-sidebar-item-label]' ? [collectionLabel] : [],
    } as unknown as HTMLElement;

    expect(
      collectSidebarAnimatorLabelEls({
        navRegionEl,
        collectionsRegionEl,
        bottomLabelEl: bottomLabel,
      })
    ).toEqual([navLabel, collectionLabel, bottomLabel]);
  });
});
