function createMemoryStorage(): Storage {
  const values = new Map<string, string>();

  return {
    get length() {
      return values.size;
    },
    clear() {
      values.clear();
    },
    getItem(key) {
      return values.get(String(key)) ?? null;
    },
    key(index) {
      return [...values.keys()][index] ?? null;
    },
    removeItem(key) {
      values.delete(String(key));
    },
    setItem(key, value) {
      values.set(String(key), String(value));
    },
  };
}

if (typeof window !== 'undefined') {
  const storage = createMemoryStorage();
  // Node 26 exposes an experimental localStorage getter that masks jsdom's
  // implementation unless --localstorage-file is provided.
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: storage,
  });
  if (globalThis !== window) {
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: storage,
    });
  }

  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}
