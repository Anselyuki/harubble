import * as m from '$lib/paraglide/messages.js';
import type SettingsSheet from '$lib/components/app/SettingsSheet.svelte';
import type DownloadTasksSheet from '$lib/components/app/DownloadTasksSheet.svelte';

interface OpenSideSheetOptions {
  notifyError: (message: string) => void;
  beforeOpen?: () => void | Promise<void>;
}

type SettingsSheetComponent = typeof SettingsSheet;
type DownloadTasksSheetComponent = typeof DownloadTasksSheet;

export type AppView =
  | 'home'
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

let sidebarCollapsed = $state(
  typeof window !== 'undefined' &&
    localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === 'true'
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
    settingsSheetLoader = import('$lib/components/app/SettingsSheet.svelte')
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
      import('$lib/components/app/DownloadTasksSheet.svelte')
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
    if (typeof window !== 'undefined') {
      localStorage.setItem(SIDEBAR_COLLAPSED_KEY, String(value));
    }
  },
  toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    if (typeof window !== 'undefined') {
      localStorage.setItem(SIDEBAR_COLLAPSED_KEY, String(sidebarCollapsed));
    }
  },
  navigateToHome() {
    currentView = 'home';
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
