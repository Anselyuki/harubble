import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { createCollapseTimer } from './volume-capsule-timer';

describe('createCollapseTimer', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  test('fires callback after delay', () => {
    const cb = vi.fn();
    const timer = createCollapseTimer(500, cb);
    timer.schedule();
    vi.advanceTimersByTime(499);
    expect(cb).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(cb).toHaveBeenCalledOnce();
  });

  test('cancel prevents firing', () => {
    const cb = vi.fn();
    const timer = createCollapseTimer(500, cb);
    timer.schedule();
    timer.cancel();
    vi.advanceTimersByTime(600);
    expect(cb).not.toHaveBeenCalled();
  });

  test('schedule resets existing timer', () => {
    const cb = vi.fn();
    const timer = createCollapseTimer(500, cb);
    timer.schedule();
    vi.advanceTimersByTime(300);
    timer.schedule();
    vi.advanceTimersByTime(300);
    expect(cb).not.toHaveBeenCalled();
    vi.advanceTimersByTime(200);
    expect(cb).toHaveBeenCalledOnce();
  });

  test('destroy clears pending timer', () => {
    const cb = vi.fn();
    const timer = createCollapseTimer(500, cb);
    timer.schedule();
    timer.destroy();
    vi.advanceTimersByTime(600);
    expect(cb).not.toHaveBeenCalled();
  });

  test('pending returns true when scheduled', () => {
    const cb = vi.fn();
    const timer = createCollapseTimer(500, cb);
    expect(timer.pending).toBe(false);
    timer.schedule();
    expect(timer.pending).toBe(true);
    timer.cancel();
    expect(timer.pending).toBe(false);
  });
});
