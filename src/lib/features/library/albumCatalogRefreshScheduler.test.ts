import { describe, expect, it, vi } from 'vitest';
import {
  ALBUM_CATALOG_CHECK_INTERVAL_MS,
  createAlbumCatalogRefreshScheduler,
} from './albumCatalogRefreshScheduler';

describe('album catalog refresh scheduler', () => {
  it('checks immediately and periodically while an eligible view is active', () => {
    vi.useFakeTimers();
    const ensureFresh = vi.fn();
    const scheduler = createAlbumCatalogRefreshScheduler({
      ensureFresh,
      isDocumentVisible: () => true,
      timers: globalThis,
    });

    scheduler.setActive(true);
    expect(ensureFresh).toHaveBeenCalledOnce();

    vi.advanceTimersByTime(ALBUM_CATALOG_CHECK_INTERVAL_MS * 2);
    expect(ensureFresh).toHaveBeenCalledTimes(3);

    scheduler.setActive(false);
    vi.advanceTimersByTime(ALBUM_CATALOG_CHECK_INTERVAL_MS);
    expect(ensureFresh).toHaveBeenCalledTimes(3);
    vi.useRealTimers();
  });

  it('waits for visibility before checking and refreshes on resume', () => {
    let visible = false;
    const ensureFresh = vi.fn();
    const scheduler = createAlbumCatalogRefreshScheduler({
      ensureFresh,
      isDocumentVisible: () => visible,
      timers: globalThis,
    });

    scheduler.setActive(true);
    expect(ensureFresh).not.toHaveBeenCalled();

    visible = true;
    scheduler.handleVisibilityChange();
    scheduler.handleFocus();
    scheduler.handleOnline();
    expect(ensureFresh).toHaveBeenCalledTimes(3);
    scheduler.dispose();
  });

  it('coalesces scheduler errors into the catalog controller boundary', async () => {
    const scheduler = createAlbumCatalogRefreshScheduler({
      ensureFresh: () => Promise.reject(new Error('offline')),
      isDocumentVisible: () => true,
      timers: globalThis,
    });

    scheduler.setActive(true);
    await Promise.resolve();
    scheduler.dispose();
  });

  it('calls timer methods through their owning host', () => {
    const timer = 1 as ReturnType<typeof setInterval>;
    const timers = {
      setInterval(callback: () => void, delayMs: number) {
        expect(this).toBe(timers);
        expect(callback).toBeTypeOf('function');
        expect(delayMs).toBe(ALBUM_CATALOG_CHECK_INTERVAL_MS);
        return timer;
      },
      clearInterval(received: ReturnType<typeof setInterval>) {
        expect(this).toBe(timers);
        expect(received).toBe(timer);
      },
    };
    const scheduler = createAlbumCatalogRefreshScheduler({
      ensureFresh: vi.fn(),
      isDocumentVisible: () => true,
      timers,
    });

    scheduler.setActive(true);
    scheduler.dispose();
  });
});
