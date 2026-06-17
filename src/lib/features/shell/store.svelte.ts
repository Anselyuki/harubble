import * as m from '$lib/paraglide/messages.js';
import type SettingsSheet from '$lib/components/app/shell/SettingsSheet.svelte';
import type DownloadTasksSheet from '$lib/components/app/shell/DownloadTasksSheet.svelte';

interface OpenSideSheetOptions {
  notifyError: (message: string) => void;
  beforeOpen?: () => void | Promise<void>;
}

type SettingsSheetComponent = typeof SettingsSheet;
type DownloadTasksSheetComponent = typeof DownloadTasksSheet;

export type AppView =
  | 'home'
  | 'search'
  | 'overview'
  | 'library'
  | 'tagEditor'
  | 'collection';

let settingsOpen = $state(false);
let downloadPanelOpen = $state(false);
let SettingsSheetView = $state<SettingsSheetComponent | null>(null);
let DownloadTasksSheetView = $state<DownloadTasksSheetComponent | null>(null);
let settingsSheetLoader = $state<Promise<SettingsSheetComponent> | null>(null);
let downloadTasksSheetLoader =
  $state<Promise<DownloadTasksSheetComponent> | null>(null);
let currentView = $state<AppView>('home');

const SIDEBAR_COLLAPSED_KEY = 'harubble:sidebar-collapsed';
const SIDEBAR_WIDTH_KEY = 'harubble:sidebar-width';
const DEFAULT_SIDEBAR_WIDTH = 248;

let sidebarCollapsed = $state(
  typeof window !== 'undefined' &&
    localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === 'true'
);

/**
 * 标记本次侧栏折叠/展开是否走「压缩同步」编排。
 *
 * 仅由 {@link setSidebarCollapsedTransient}（视图驱动的强制收缩/展开，如进入
 * Tag Editor）置 `true`，此时折叠动画压成单一时间线、与页面转场同时收尾，避免
 * logo FLIP 长尾抖动；用户手动点折叠按钮（{@link toggleSidebar}）或拖曳 resize
 * （`set sidebarCollapsed`）会置 `false`，保留完整保真特效。
 */
let sidebarTransientCompact = $state(false);

let sidebarWidth = $state(
  typeof window !== 'undefined'
    ? Number.parseFloat(
        localStorage.getItem(SIDEBAR_WIDTH_KEY) ?? String(DEFAULT_SIDEBAR_WIDTH)
      ) || DEFAULT_SIDEBAR_WIDTH
    : DEFAULT_SIDEBAR_WIDTH
);

let sideSheetRequestSeq = 0;
let initialized = false;

function init() {
  if (initialized) return;
  initialized = true;
}

function dispose() {
  settingsOpen = false;
  downloadPanelOpen = false;
  currentView = 'home';
  sidebarCollapsed = false;
  sidebarWidth = DEFAULT_SIDEBAR_WIDTH;
  sidebarTransientCompact = false;
  sideSheetRequestSeq = 0;
  initialized = false;
}

async function ensureSettingsSheetLoaded(
  notifyError: (message: string) => void
): Promise<boolean> {
  if (SettingsSheetView) {
    return true;
  }

  if (!settingsSheetLoader) {
    settingsSheetLoader =
      import('$lib/components/app/shell/SettingsSheet.svelte')
        .then((module) => {
          SettingsSheetView = module.default;
          return module.default;
        })
        .finally(() => {
          settingsSheetLoader = null;
        });
  }

  try {
    await settingsSheetLoader;
    return true;
  } catch (error: unknown) {
    notifyError(
      m.shell_error_open_settings_failed({
        error: error instanceof Error ? error.message : String(error),
      })
    );
    return false;
  }
}

async function ensureDownloadTasksSheetLoaded(
  notifyError: (message: string) => void
): Promise<boolean> {
  if (DownloadTasksSheetView) {
    return true;
  }

  if (!downloadTasksSheetLoader) {
    downloadTasksSheetLoader =
      import('$lib/components/app/shell/DownloadTasksSheet.svelte')
        .then((module) => {
          DownloadTasksSheetView = module.default;
          return module.default;
        })
        .finally(() => {
          downloadTasksSheetLoader = null;
        });
  }

  try {
    await downloadTasksSheetLoader;
    return true;
  } catch (error: unknown) {
    notifyError(
      m.shell_error_open_downloads_failed({
        error: error instanceof Error ? error.message : String(error),
      })
    );
    return false;
  }
}

async function openSettings(
  options: Pick<OpenSideSheetOptions, 'notifyError'>
): Promise<boolean> {
  const requestSeq = ++sideSheetRequestSeq;
  const loaded = await ensureSettingsSheetLoaded(options.notifyError);
  if (!loaded || requestSeq !== sideSheetRequestSeq) {
    return false;
  }

  settingsOpen = true;
  downloadPanelOpen = false;
  return true;
}

async function openDownloads(options: OpenSideSheetOptions): Promise<boolean> {
  const requestSeq = ++sideSheetRequestSeq;
  await options.beforeOpen?.();

  if (requestSeq !== sideSheetRequestSeq) {
    return false;
  }

  const loaded = await ensureDownloadTasksSheetLoaded(options.notifyError);
  if (!loaded || requestSeq !== sideSheetRequestSeq) {
    return false;
  }

  downloadPanelOpen = true;
  settingsOpen = false;
  return true;
}

async function toggleSettings(
  options: Pick<OpenSideSheetOptions, 'notifyError'>
): Promise<boolean> {
  if (settingsOpen) {
    sideSheetRequestSeq += 1;
    settingsOpen = false;
    return true;
  }

  return openSettings(options);
}

async function toggleDownloads(
  options: OpenSideSheetOptions
): Promise<boolean> {
  if (downloadPanelOpen) {
    sideSheetRequestSeq += 1;
    downloadPanelOpen = false;
    return true;
  }

  return openDownloads(options);
}

export const shellStore = {
  get settingsOpen() {
    return settingsOpen;
  },
  set settingsOpen(value: boolean) {
    settingsOpen = value;
  },
  get downloadPanelOpen() {
    return downloadPanelOpen;
  },
  set downloadPanelOpen(value: boolean) {
    downloadPanelOpen = value;
  },
  get currentView() {
    return currentView;
  },
  set currentView(value: AppView) {
    currentView = value;
  },
  get sidebarCollapsed() {
    return sidebarCollapsed;
  },
  set sidebarCollapsed(value: boolean) {
    sidebarCollapsed = value;
    // 手动 / 拖曳路径：恢复完整保真编排
    sidebarTransientCompact = false;
    if (typeof window !== 'undefined') {
      localStorage.setItem(SIDEBAR_COLLAPSED_KEY, String(value));
    }
  },
  /**
   * 视图驱动的强制收缩/展开是否走压缩同步编排。
   *
   * App.svelte 的共享 `$effect` 据此决定侧栏宽度与 logo 动画是否压成单一时间线
   * 与页面转场同时收尾。仅 {@link setSidebarCollapsedTransient} 会置 `true`。
   */
  get sidebarTransientCompact() {
    return sidebarTransientCompact;
  },
  setSidebarCollapsedTransient(value: boolean) {
    sidebarCollapsed = value;
    // 视图驱动（如进入 Tag Editor）：走压缩同步编排，与页面转场齐步收尾
    sidebarTransientCompact = true;
  },
  toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    // 手动按钮：恢复完整保真编排
    sidebarTransientCompact = false;
    if (typeof window !== 'undefined') {
      localStorage.setItem(SIDEBAR_COLLAPSED_KEY, String(sidebarCollapsed));
    }
  },
  get sidebarWidth() {
    return sidebarWidth;
  },
  set sidebarWidth(value: number) {
    sidebarWidth = value;
    if (typeof window !== 'undefined') {
      localStorage.setItem(SIDEBAR_WIDTH_KEY, String(value));
    }
  },
  navigateToHome() {
    currentView = 'home';
  },
  navigateToSearch() {
    currentView = 'search';
  },
  navigateToOverview() {
    currentView = 'overview';
  },
  navigateToLibrary() {
    currentView = 'library';
  },
  navigateToTagEditor() {
    currentView = 'tagEditor';
  },
  navigateToCollection() {
    currentView = 'collection';
  },
  get SettingsSheetView() {
    return SettingsSheetView;
  },
  get DownloadTasksSheetView() {
    return DownloadTasksSheetView;
  },
  ensureSettingsSheetLoaded,
  ensureDownloadTasksSheetLoaded,
  openSettings,
  openDownloads,
  toggleSettings,
  toggleDownloads,
  init,
  dispose,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    dispose();
  });
}
