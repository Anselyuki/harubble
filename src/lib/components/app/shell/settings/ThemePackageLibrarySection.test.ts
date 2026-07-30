// @vitest-environment jsdom

import {
  cleanup,
  fireEvent,
  render,
  waitFor,
  within,
} from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SvelteMap } from 'svelte/reactivity';
import type { ThemePackageSummary } from '$lib/types';

const managerState = new SvelteMap<string, string | null>([
  ['activePackageId', null],
  ['previewingId', null],
]);

const manager = {
  currentRevision: 1,
  get activePackageId(): string | null {
    return managerState.get('activePackageId') ?? null;
  },
  set activePackageId(value: string | null) {
    managerState.set('activePackageId', value);
  },
  get previewingId(): string | null {
    return managerState.get('previewingId') ?? null;
  },
  set previewingId(value: string | null) {
    managerState.set('previewingId', value);
  },
  installedPackages: [] as ThemePackageSummary[],
  latestError: null as string | null,
  hydrate: vi.fn(async () => {}),
  refreshList: vi.fn(async () => []),
  setActive: vi.fn(async (_id: string | null) => ({})),
  preview: vi.fn(async (_id: string) => ({})),
  dismissPreview: vi.fn(async () => {}),
  importFromFile: vi.fn(async () => ({})),
  importFromUrl: vi.fn(async () => ({})),
  inspect: vi.fn(async () => null),
  uninstall: vi.fn(async () => {}),
  exportPackage: vi.fn(async () => {}),
  startSubscription: vi.fn(async () => {}),
  stopSubscription: vi.fn(),
};

vi.mock('$lib/features/shell/themePackageManager.svelte', () => ({
  getThemePackageManager: () => manager,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(async () => null),
}));

import ThemePackageLibrarySection from './ThemePackageLibrarySection.svelte';

const BUILTIN: ThemePackageSummary = {
  id: 'ark-ui-ark',
  name: 'Industrial Cyan',
  version: '1.0.0',
  status: 'committed',
  builtin: true,
};

const INSTALLED: ThemePackageSummary = {
  id: 'custom-theme',
  name: 'Custom Theme',
  version: '1.0.0',
  status: 'committed',
  builtin: false,
};

function installLocalStorageMock(): Storage {
  const values = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return values.size;
    },
    clear: vi.fn(() => values.clear()),
    getItem: vi.fn((key: string) => values.get(key) ?? null),
    key: vi.fn((index: number) => [...values.keys()][index] ?? null),
    removeItem: vi.fn((key: string) => values.delete(key)),
    setItem: vi.fn((key: string, value: string) => {
      values.set(key, String(value));
    }),
  };
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: storage,
  });
  return storage;
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });
  return { promise, resolve };
}

describe('ThemePackageLibrarySection built-in package actions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    installLocalStorageMock();
    manager.installedPackages = [BUILTIN, INSTALLED];
    manager.activePackageId = null;
    manager.previewingId = null;
    manager.hydrate.mockReset().mockImplementation(async () => {});
    manager.setActive
      .mockReset()
      .mockImplementation(async (id: string | null) => {
        manager.activePackageId = id;
        manager.previewingId = null;
        return {};
      });
    manager.preview.mockReset().mockImplementation(async (id: string) => {
      manager.previewingId = id;
      return {};
    });
    manager.dismissPreview.mockReset().mockImplementation(async () => {
      manager.previewingId = null;
    });
    manager.importFromFile.mockReset().mockResolvedValue({});
    manager.importFromUrl.mockReset().mockResolvedValue({});
  });

  afterEach(() => {
    cleanup();
    document.body.replaceChildren();
  });

  it('marks built-ins and only exposes uninstall for mutable packages', () => {
    const { getAllByTestId, getByText } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });

    const [builtinItem, installedItem] = getAllByTestId('theme-package-item');
    expect(builtinItem).toHaveAttribute('data-package-id', BUILTIN.id);
    expect(within(builtinItem!).getByText('内置')).toBeInTheDocument();
    expect(
      within(builtinItem!).queryByRole('button', { name: '卸载' })
    ).not.toBeInTheDocument();

    expect(installedItem).toHaveAttribute('data-package-id', INSTALLED.id);
    expect(
      within(installedItem!).getByRole('button', { name: '卸载' })
    ).toBeInTheDocument();
    expect(getByText('Industrial Cyan')).toBeInTheDocument();
  });

  it('keeps preview and activation available for built-in packages', async () => {
    const { getAllByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const [builtinItem] = getAllByTestId('theme-package-item');

    await fireEvent.click(
      within(builtinItem!).getByRole('button', { name: '预览' })
    );
    await fireEvent.click(
      within(builtinItem!).getByRole('button', { name: '应用' })
    );

    expect(manager.preview).toHaveBeenCalledWith(BUILTIN.id);
    expect(manager.setActive).toHaveBeenCalledWith(BUILTIN.id);
  });

  it('distinguishes the active package from the package being previewed', () => {
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    const { getAllByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const items = getAllByTestId('theme-package-item');
    const activeItem = items.find(
      (item) => item.getAttribute('data-package-id') === BUILTIN.id
    );
    const previewItem = items.find(
      (item) => item.getAttribute('data-package-id') === INSTALLED.id
    );

    expect(activeItem).toBeDefined();
    expect(activeItem).toHaveAttribute('data-active', 'true');
    expect(activeItem).toHaveAttribute('data-previewing', 'false');
    expect(activeItem).toHaveClass('package-item--active');
    expect(activeItem).not.toHaveClass('package-item--previewing');
    expect(
      within(activeItem!).getByTestId('theme-package-active-badge')
    ).toHaveTextContent('激活中');
    expect(
      within(activeItem!).queryByTestId('theme-package-preview-badge')
    ).not.toBeInTheDocument();
    expect(
      within(activeItem!).getByRole('button', { name: '预览' })
    ).toHaveAttribute('aria-pressed', 'false');
    expect(
      within(activeItem!).getByRole('button', { name: '预览' })
    ).toBeEnabled();
    expect(
      within(activeItem!).getByRole('button', { name: '激活中' })
    ).toHaveAttribute('aria-pressed', 'true');
    expect(
      within(activeItem!).getByRole('button', { name: '激活中' })
    ).toBeDisabled();

    expect(previewItem).toBeDefined();
    expect(previewItem).toHaveAttribute('data-active', 'false');
    expect(previewItem).toHaveAttribute('data-previewing', 'true');
    expect(previewItem).toHaveClass('package-item--previewing');
    expect(previewItem).not.toHaveClass('package-item--active');
    expect(
      within(previewItem!).getByTestId('theme-package-preview-badge')
    ).toHaveTextContent('预览中');
    expect(
      within(previewItem!).queryByTestId('theme-package-active-badge')
    ).not.toBeInTheDocument();
    expect(
      within(previewItem!).getByRole('button', { name: '预览中' })
    ).toHaveAttribute('aria-pressed', 'true');
    expect(
      within(previewItem!).getByRole('button', { name: '预览中' })
    ).toBeDisabled();
    expect(
      within(previewItem!).getByRole('button', { name: '应用' })
    ).toHaveAttribute('aria-pressed', 'false');
    expect(
      within(previewItem!).getByRole('button', { name: '应用' })
    ).toBeEnabled();
  });

  it('keeps active status exclusive when stale state matches the preview id', async () => {
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = BUILTIN.id;
    const { getAllByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const activeItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === BUILTIN.id
    );
    const previewButton = within(activeItem!).getByRole('button', {
      name: '预览',
    });

    expect(activeItem).toHaveAttribute('data-active', 'true');
    expect(activeItem).toHaveAttribute('data-previewing', 'false');
    expect(activeItem).toHaveClass('package-item--active');
    expect(activeItem).not.toHaveClass('package-item--previewing');
    expect(
      within(activeItem!).queryByTestId('theme-package-preview-badge')
    ).not.toBeInTheDocument();
    expect(previewButton).toHaveAttribute('aria-pressed', 'false');
    expect(previewButton).toBeEnabled();
    expect(
      within(activeItem!).queryByTestId('theme-package-preview-badge')
    ).not.toBeInTheDocument();

    await fireEvent.click(previewButton);

    expect(manager.dismissPreview).not.toHaveBeenCalled();
    expect(manager.preview).not.toHaveBeenCalled();
    expect(manager.previewingId).toBe(BUILTIN.id);
    expect(
      within(activeItem!).getByRole('button', { name: '预览' })
    ).toBeEnabled();
  });

  it('uses the active preview action to dismiss a different package preview', async () => {
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    const { getAllByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const activeItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === BUILTIN.id
    );

    await fireEvent.click(
      within(activeItem!).getByRole('button', { name: '预览' })
    );

    expect(manager.dismissPreview).toHaveBeenCalledOnce();
    expect(manager.preview).not.toHaveBeenCalled();
    expect(manager.previewingId).toBeNull();
  });

  it('leaves the active preview button idle when no preview exists', async () => {
    manager.activePackageId = BUILTIN.id;
    const { getAllByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const activeItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === BUILTIN.id
    );

    await fireEvent.click(
      within(activeItem!).getByRole('button', { name: '预览' })
    );

    expect(manager.preview).not.toHaveBeenCalled();
    expect(manager.dismissPreview).not.toHaveBeenCalled();
    expect(manager.previewingId).toBeNull();
  });

  it('collapses to a recovery control without clearing package-owned state', async () => {
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    const { getByTestId, queryByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const collapseButton = getByTestId('theme-package-library-disable');
    collapseButton.focus();

    await fireEvent.click(collapseButton);

    await waitFor(() => {
      expect(window.localStorage.getItem('theme_packages_v1')).toBe('0');
      expect(
        queryByTestId('theme-package-library-section')
      ).not.toBeInTheDocument();
      expect(
        getByTestId('theme-package-library-collapsed')
      ).toBeInTheDocument();
      expect(getByTestId('theme-package-library-enable')).toHaveFocus();
    });
    expect(manager.activePackageId).toBe(BUILTIN.id);
    expect(manager.previewingId).toBe(INSTALLED.id);
    expect(manager.dismissPreview).not.toHaveBeenCalled();
    expect(manager.setActive).not.toHaveBeenCalled();
  });

  it('keeps the library expanded while a package operation is pending', async () => {
    const applyGate = deferred<void>();
    manager.setActive.mockImplementation(async (id: string | null) => {
      await applyGate.promise;
      manager.activePackageId = id;
      return {};
    });
    const { getAllByTestId, getByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const customItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === INSTALLED.id
    );

    await fireEvent.click(
      within(customItem!).getByRole('button', { name: '应用' })
    );

    await waitFor(() => {
      expect(getByTestId('theme-package-library-disable')).toBeDisabled();
      expect(getByTestId('theme-package-library-section')).toBeInTheDocument();
      expect(window.localStorage.getItem('theme_packages_v1')).toBeNull();
    });

    applyGate.resolve(undefined);
    await waitFor(() => {
      expect(getByTestId('theme-package-library-disable')).toBeEnabled();
    });
  });

  it('does not dismiss preview state when unmounted during an unrelated activation', async () => {
    const applyGate = deferred<void>();
    manager.setActive.mockImplementation(async (id: string | null) => {
      await applyGate.promise;
      manager.activePackageId = id;
      return {};
    });
    const { getAllByTestId, unmount } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const customItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === INSTALLED.id
    );

    await fireEvent.click(
      within(customItem!).getByRole('button', { name: '应用' })
    );
    await waitFor(() => expect(manager.setActive).toHaveBeenCalledOnce());
    unmount();

    expect(manager.dismissPreview).not.toHaveBeenCalled();
    applyGate.resolve(undefined);
  });

  it('lets current-preview activation own cleanup when unmounted', async () => {
    const applyGate = deferred<void>();
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    manager.setActive.mockImplementation(async (id: string | null) => {
      await applyGate.promise;
      manager.activePackageId = id;
      manager.previewingId = null;
      return {};
    });
    const { getAllByTestId, unmount } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const previewItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === INSTALLED.id
    );

    await fireEvent.click(
      within(previewItem!).getByRole('button', { name: '应用' })
    );
    await waitFor(() => expect(manager.setActive).toHaveBeenCalledOnce());
    unmount();

    expect(manager.dismissPreview).not.toHaveBeenCalled();
    expect(manager.previewingId).toBe(INSTALLED.id);
    applyGate.resolve(undefined);
    await waitFor(() => expect(manager.previewingId).toBeNull());
    expect(manager.dismissPreview).not.toHaveBeenCalled();
  });

  it('cleans a remaining preview after unmounted activation fails', async () => {
    const applyGate = deferred<void>();
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    manager.setActive.mockImplementation(async () => {
      await applyGate.promise;
      throw new Error('activation failed');
    });
    const { getAllByTestId, unmount } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const previewItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === INSTALLED.id
    );

    await fireEvent.click(
      within(previewItem!).getByRole('button', { name: '应用' })
    );
    await waitFor(() => expect(manager.setActive).toHaveBeenCalledOnce());
    unmount();

    expect(manager.dismissPreview).not.toHaveBeenCalled();
    applyGate.resolve(undefined);
    await waitFor(() => expect(manager.dismissPreview).toHaveBeenCalledOnce());
    expect(manager.previewingId).toBeNull();
  });

  it('dismisses a preview request that is still pending when unmounted', async () => {
    const previewGate = deferred<void>();
    manager.preview.mockImplementation(async () => {
      await previewGate.promise;
      return {};
    });
    const { getAllByTestId, unmount } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const customItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === INSTALLED.id
    );

    await fireEvent.click(
      within(customItem!).getByRole('button', { name: '预览' })
    );
    await waitFor(() => expect(manager.preview).toHaveBeenCalledOnce());
    unmount();

    expect(manager.dismissPreview).toHaveBeenCalledOnce();
    previewGate.resolve(undefined);
  });

  it('does not dismiss twice when unmounted during active-package restoration', async () => {
    const dismissGate = deferred<void>();
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    manager.dismissPreview.mockImplementation(async () => {
      manager.previewingId = null;
      await dismissGate.promise;
    });
    const { getAllByTestId, unmount } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const activeItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === BUILTIN.id
    );

    await fireEvent.click(
      within(activeItem!).getByRole('button', { name: '预览' })
    );
    await waitFor(() => expect(manager.dismissPreview).toHaveBeenCalledOnce());
    unmount();

    expect(manager.dismissPreview).toHaveBeenCalledOnce();
    dismissGate.resolve(undefined);
  });

  it('restores the expanded library and keeps its active and preview state', async () => {
    window.localStorage.setItem('theme_packages_v1', '0');
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    const { getByTestId, getAllByTestId, queryByTestId } = render(
      ThemePackageLibrarySection,
      {
        props: {
          sectionTitle: 'Theme packages',
          sectionDescription: 'Manage themes',
        },
      }
    );

    expect(
      queryByTestId('theme-package-library-section')
    ).not.toBeInTheDocument();
    expect(getByTestId('theme-package-library-collapsed')).toBeInTheDocument();
    expect(manager.hydrate).not.toHaveBeenCalled();

    const expandButton = getByTestId('theme-package-library-enable');
    expandButton.focus();
    await fireEvent.click(expandButton);

    await waitFor(() => {
      expect(window.localStorage.getItem('theme_packages_v1')).toBe('1');
      expect(getByTestId('theme-package-library-section')).toBeInTheDocument();
      expect(
        queryByTestId('theme-package-library-collapsed')
      ).not.toBeInTheDocument();
      expect(getByTestId('theme-package-library-disable')).toHaveFocus();
    });
    const activeItem = getAllByTestId('theme-package-item').find(
      (item) => item.getAttribute('data-package-id') === BUILTIN.id
    );
    expect(activeItem).toHaveClass('package-item--active');
    expect(getByTestId('theme-package-dismiss-preview')).toHaveTextContent(
      INSTALLED.id
    );
    expect(manager.activePackageId).toBe(BUILTIN.id);
    expect(manager.previewingId).toBe(INSTALLED.id);
    expect(manager.hydrate).toHaveBeenCalledOnce();
    expect(manager.dismissPreview).not.toHaveBeenCalled();
    expect(manager.setActive).not.toHaveBeenCalled();
  });

  it('renders the recovery control when the persisted state starts collapsed', () => {
    window.localStorage.setItem('theme_packages_v1', '0');
    const { getByTestId, queryByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });

    expect(
      queryByTestId('theme-package-library-section')
    ).not.toBeInTheDocument();
    expect(getByTestId('theme-package-library-collapsed')).toBeInTheDocument();
    expect(
      within(getByTestId('theme-package-library-collapsed')).getByRole(
        'button',
        {
          name: '展开主题包库',
        }
      )
    ).toBeInTheDocument();
  });

  it('shows sanitizer adjustments returned after a URL import', async () => {
    manager.importFromUrl.mockResolvedValue({
      ...INSTALLED,
      warnings: ["slots: unknown slot 'legacyAccent' dropped"],
    });
    const { getByRole, getByTestId } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });

    await fireEvent.input(getByTestId('theme-package-url-input'), {
      target: { value: 'https://example.com/theme.json' },
    });
    await fireEvent.click(getByTestId('theme-package-url-submit'));

    await waitFor(() => {
      expect(manager.importFromUrl).toHaveBeenCalledWith(
        'https://example.com/theme.json'
      );
      expect(getByRole('status')).toHaveTextContent(
        '主题包已导入，以下内容已调整'
      );
      expect(getByRole('status')).toHaveTextContent('legacyAccent');
    });
  });

  it('waits for a pending URL import before cleaning preview on unmount', async () => {
    const importGate = deferred<Record<string, never>>();
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    manager.importFromUrl.mockImplementation(() => importGate.promise);
    const { getByTestId, unmount } = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });

    await fireEvent.input(getByTestId('theme-package-url-input'), {
      target: { value: 'https://example.com/active-theme.json' },
    });
    await fireEvent.click(getByTestId('theme-package-url-submit'));
    await waitFor(() => expect(manager.importFromUrl).toHaveBeenCalledOnce());
    unmount();

    expect(manager.dismissPreview).not.toHaveBeenCalled();
    expect(manager.previewingId).toBe(INSTALLED.id);
    importGate.resolve({});
    await waitFor(() => expect(manager.dismissPreview).toHaveBeenCalledOnce());
    expect(manager.previewingId).toBeNull();
  });

  it('does not let an old pending import dismiss the same preview in a reopened sheet', async () => {
    const importGate = deferred<Record<string, never>>();
    manager.activePackageId = BUILTIN.id;
    manager.previewingId = INSTALLED.id;
    manager.importFromUrl.mockImplementation(() => importGate.promise);
    const oldSheet = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });

    await fireEvent.input(oldSheet.getByTestId('theme-package-url-input'), {
      target: { value: 'https://example.com/active-theme.json' },
    });
    await fireEvent.click(oldSheet.getByTestId('theme-package-url-submit'));
    await waitFor(() => expect(manager.importFromUrl).toHaveBeenCalledOnce());
    oldSheet.unmount();

    manager.previewingId = null;
    const reopenedSheet = render(ThemePackageLibrarySection, {
      props: {
        sectionTitle: 'Theme packages',
        sectionDescription: 'Manage themes',
      },
    });
    const reopenedPreviewItem = reopenedSheet
      .getAllByTestId('theme-package-item')
      .find((item) => item.getAttribute('data-package-id') === INSTALLED.id);
    await fireEvent.click(
      within(reopenedPreviewItem!).getByRole('button', { name: '预览' })
    );
    await waitFor(() => {
      expect(manager.preview).toHaveBeenCalledWith(INSTALLED.id);
      expect(manager.previewingId).toBe(INSTALLED.id);
    });

    importGate.resolve({});
    await importGate.promise;
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(manager.dismissPreview).not.toHaveBeenCalled();
    expect(manager.previewingId).toBe(INSTALLED.id);
  });
});
