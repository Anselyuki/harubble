import { getContext, setContext } from 'svelte';
import type { PartialOptions } from 'overlayscrollbars';
import type { AppView } from '$lib/features/shell/store.svelte';
import { SHELL_CONTEXT_KEY } from './keys';

export interface ShellContext {
  readonly currentView: AppView;
  readonly isMacOS: boolean;
  readonly prefersReducedMotion: boolean;
  readonly settingsOpen: boolean;
  readonly downloadPanelOpen: boolean;
  readonly isRefreshing: boolean;
  readonly overlayScrollbarOptions: PartialOptions;
  readonly sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  handleRefresh: () => void | Promise<void>;
  handleToggleSettings: () => void;
  handleToggleDownloads: () => void;
  notifyInfo: (message: string) => void;
  notifyError: (message: string) => void;
  navigate: (view: AppView) => void;
}

export function setShellContext(ctx: ShellContext): void {
  setContext(SHELL_CONTEXT_KEY, ctx);
}

export function getShellContext(): ShellContext {
  return getContext<ShellContext>(SHELL_CONTEXT_KEY);
}
