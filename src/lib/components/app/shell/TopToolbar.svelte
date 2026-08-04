<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { Button } from '$lib/components/ui/button/index.js';
  import { toolbarIconButton } from '$lib/design/variants';
  import { RefreshCw, ArrowDown, Settings } from '@lucide/svelte';

  interface Props {
    activeDownloadCount: number;
    isRefreshing?: boolean;
    settingsOpen?: boolean;
    downloadPanelOpen?: boolean;
    onRefresh: () => void;
    onOpenDownloads: () => void;
    onOpenSettings: () => void;
  }

  let {
    activeDownloadCount,
    isRefreshing = false,
    settingsOpen = false,
    downloadPanelOpen = false,
    onRefresh,
    onOpenDownloads,
    onOpenSettings,
  }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      refresh: m.shell_toolbar_refresh(),
      downloads: m.shell_toolbar_downloads(),
      settings: m.shell_toolbar_settings(),
    };
  });
</script>

<div class="top-actions">
  <div class="top-actions-pill flex items-center gap-1 p-1">
    <Button
      size="icon"
      variant="ghost"
      class={`size-10 text-base ${toolbarIconButton({ active: false })}`}
      onclick={onRefresh}
      disabled={isRefreshing}
      aria-label={labels.refresh}
      title={labels.refresh}
    >
      <RefreshCw size={16} />
    </Button>

    <Button
      size="icon"
      variant="ghost"
      class={`relative size-10 text-base ${toolbarIconButton({ active: downloadPanelOpen })}`}
      data-testid="downloads-panel-trigger"
      onclick={onOpenDownloads}
      aria-label={labels.downloads}
      aria-pressed={downloadPanelOpen}
      title={labels.downloads}
    >
      <ArrowDown size={16} />
      {#if activeDownloadCount > 0}
        <span class="toolbar-badge">{activeDownloadCount}</span>
      {/if}
    </Button>

    <Button
      size="icon"
      variant="ghost"
      class={`size-10 text-base ${toolbarIconButton({ active: settingsOpen })}`}
      data-testid="settings-trigger"
      onclick={onOpenSettings}
      aria-label={labels.settings}
      aria-pressed={settingsOpen}
      title={labels.settings}
    >
      <Settings size={16} />
    </Button>
  </div>
</div>

<style>
  .top-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* 悬浮 chrome：走 toolbar / shape / blur token，跟随 :root.dark 与主题包切换。
     禁止在这里回写 bg-white/border-white/shadow-[rgba(…)] 一类硬编码，
     否则深色主题包会出现"白底白 icon = 图标消失"的失效外观。 */
  .top-actions-pill {
    border: 1px solid var(--toolbar-highlight);
    background: var(--toolbar-surface);
    border-radius: var(--shape-pill);
    box-shadow: var(--toolbar-shadow);
    backdrop-filter: blur(var(--blur-lg, 20px)) saturate(1.2);
    -webkit-backdrop-filter: blur(var(--blur-lg, 20px)) saturate(1.2);
  }

  .toolbar-badge {
    position: absolute;
    top: 2px;
    right: 2px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: var(--shape-pill);
    background: var(--accent, #5090ff);
    color: var(--accent-readable-foreground);
    font-size: 9px;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
    pointer-events: none;
  }
</style>
