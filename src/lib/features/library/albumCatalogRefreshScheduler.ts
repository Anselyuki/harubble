export const ALBUM_CATALOG_CHECK_INTERVAL_MS = 60 * 1000;

interface AlbumCatalogRefreshSchedulerDeps {
  ensureFresh: () => void | Promise<void>;
  isDocumentVisible: () => boolean;
  timers: {
    setInterval: (
      callback: () => void,
      delayMs: number
    ) => ReturnType<typeof setInterval>;
    clearInterval: (timer: ReturnType<typeof setInterval>) => void;
  };
}

export function createAlbumCatalogRefreshScheduler(
  deps: AlbumCatalogRefreshSchedulerDeps
) {
  let active = false;
  let timer: ReturnType<typeof setInterval> | null = null;

  function clearTimer() {
    if (timer === null) return;
    deps.timers.clearInterval(timer);
    timer = null;
  }

  function requestRefresh() {
    if (!active || !deps.isDocumentVisible()) return;
    void Promise.resolve(deps.ensureFresh()).catch(() => {});
  }

  function setActive(next: boolean) {
    if (active === next) return;
    active = next;
    clearTimer();
    if (!active) return;

    requestRefresh();
    timer = deps.timers.setInterval(
      requestRefresh,
      ALBUM_CATALOG_CHECK_INTERVAL_MS
    );
  }

  function handleVisibilityChange() {
    requestRefresh();
  }

  function handleFocus() {
    requestRefresh();
  }

  function handleOnline() {
    requestRefresh();
  }

  function dispose() {
    active = false;
    clearTimer();
  }

  return {
    setActive,
    handleVisibilityChange,
    handleFocus,
    handleOnline,
    dispose,
  };
}
