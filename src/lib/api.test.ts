import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
const openMock = vi.hoisted(() => vi.fn());
const idbStore = vi.hoisted(() => new Map<string, unknown>());

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: openMock,
}));

vi.mock('idb-keyval', () => ({
  get: vi.fn((key: IDBValidKey) => Promise.resolve(idbStore.get(String(key)))),
  set: vi.fn((key: IDBValidKey, value: unknown) => {
    idbStore.set(String(key), value);
    return Promise.resolve();
  }),
  del: vi.fn((key: IDBValidKey) => {
    idbStore.delete(String(key));
    return Promise.resolve();
  }),
  keys: vi.fn(() => Promise.resolve([...idbStore.keys()])),
  entries: vi.fn(() => Promise.resolve([...idbStore.entries()])),
}));

const coverUrl = 'https://web.hycdn.cn/cover-a.png';
const themeUrl = 'https://web.hycdn.cn/cover-b.png';

async function flushPromises(count = 8) {
  for (let index = 0; index < count; index += 1) {
    await Promise.resolve();
  }
}

async function waitForInvokeCount(count: number) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    await flushPromises();
    if (invokeMock.mock.calls.length === count) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  expect(invokeMock).toHaveBeenCalledTimes(count);
}

describe('image resource API scheduling', () => {
  beforeEach(() => {
    vi.resetModules();
    invokeMock.mockReset();
    openMock.mockReset();
    idbStore.clear();
  });

  it('coalesces concurrent data URL requests for the same image', async () => {
    const { getImageDataUrl } = await import('./api');
    let resolveInvoke!: (value: string) => void;
    invokeMock.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveInvoke = resolve;
        })
    );

    const first = getImageDataUrl(coverUrl);
    const second = getImageDataUrl(coverUrl);

    await waitForInvokeCount(1);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('get_image_data_url', {
      imageUrl: coverUrl,
    });

    resolveInvoke('data:image/png;base64,cover');
    await expect(Promise.all([first, second])).resolves.toEqual([
      'data:image/png;base64,cover',
      'data:image/png;base64,cover',
    ]);
  });

  it('serializes album visual resource requests', async () => {
    const { extractImageTheme, getImageDataUrl } = await import('./api');
    const commands: string[] = [];
    let resolveTheme!: (value: unknown) => void;

    invokeMock.mockImplementation((command: string) => {
      commands.push(command);
      if (command === 'extract_image_theme') {
        return new Promise((resolve) => {
          resolveTheme = resolve;
        });
      }

      return Promise.resolve('data:image/png;base64,cover');
    });

    const theme = extractImageTheme(themeUrl);
    await waitForInvokeCount(1);
    expect(commands).toEqual(['extract_image_theme']);

    const cover = getImageDataUrl(coverUrl);
    await flushPromises();
    expect(commands).toEqual(['extract_image_theme']);

    resolveTheme({
      accentHex: '#111111',
      accentHoverHex: '#222222',
      accentRgb: [17, 17, 17],
      accentHoverRgb: [34, 34, 34],
      waveColors: [[17, 17, 17]],
    });
    await expect(theme).resolves.toMatchObject({
      accentHex: '#111111',
    });
    await waitForInvokeCount(2);

    expect(commands).toEqual(['extract_image_theme', 'get_image_data_url']);
    await expect(cover).resolves.toBe('data:image/png;base64,cover');
  });

  it('coalesces concurrent theme requests for the same image', async () => {
    const { extractImageTheme } = await import('./api');
    let resolveInvoke!: (value: unknown) => void;
    invokeMock.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveInvoke = resolve;
        })
    );

    const first = extractImageTheme(themeUrl);
    const second = extractImageTheme(themeUrl);

    await waitForInvokeCount(1);
    expect(invokeMock).toHaveBeenCalledWith('extract_image_theme', {
      imageUrl: themeUrl,
    });

    resolveInvoke({
      accentHex: '#111111',
      accentHoverHex: '#222222',
      accentRgb: [17, 17, 17],
      accentHoverRgb: [34, 34, 34],
      waveColors: [[17, 17, 17]],
    });

    await expect(Promise.all([first, second])).resolves.toEqual([
      expect.objectContaining({ accentHex: '#111111' }),
      expect.objectContaining({ accentHex: '#111111' }),
    ]);
  });
});
