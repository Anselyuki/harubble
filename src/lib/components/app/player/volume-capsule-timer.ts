interface CollapseTimer {
  readonly pending: boolean;
  schedule(): void;
  cancel(): void;
  destroy(): void;
}

export function createCollapseTimer(
  delayMs: number,
  onFire: () => void
): CollapseTimer {
  let id: ReturnType<typeof setTimeout> | null = null;

  function clear() {
    if (id !== null) {
      clearTimeout(id);
      id = null;
    }
  }

  return {
    get pending() {
      return id !== null;
    },
    schedule() {
      clear();
      id = setTimeout(() => {
        id = null;
        onFire();
      }, delayMs);
    },
    cancel() {
      clear();
    },
    destroy() {
      clear();
    },
  };
}
