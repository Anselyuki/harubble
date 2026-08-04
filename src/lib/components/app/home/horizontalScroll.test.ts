import { describe, expect, it } from 'vitest';
import { shouldConsumeVerticalWheel } from './horizontalScroll';

describe('shouldConsumeVerticalWheel', () => {
  it('consumes vertical wheel input while the carousel can move', () => {
    expect(shouldConsumeVerticalWheel(0, 40, 20, 100)).toBe(true);
    expect(shouldConsumeVerticalWheel(0, -40, 20, 100)).toBe(true);
  });

  it('allows the page to keep scrolling at either boundary', () => {
    expect(shouldConsumeVerticalWheel(0, -40, 0, 100)).toBe(false);
    expect(shouldConsumeVerticalWheel(0, 40, 100, 100)).toBe(false);
  });

  it('does not capture native horizontal gestures or non-overflowing content', () => {
    expect(shouldConsumeVerticalWheel(50, 10, 20, 100)).toBe(false);
    expect(shouldConsumeVerticalWheel(0, 10, 0, 0)).toBe(false);
  });
});
