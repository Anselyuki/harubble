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
  <div
    class="flex items-center gap-1 rounded-full border border-white/50 bg-white/[0.62] p-1 shadow-[0_16px_36px_rgba(15,23,42,0.12)] backdrop-blur-xl"
  >
    <Button
      size="icon"
      variant="ghost"
      class={`text-base ${toolbarIconButton({ active: false })}`}
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
      class={`relative text-base ${toolbarIconButton({ active: downloadPanelOpen })}`}
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
      class={`text-base ${toolbarIconButton({ active: settingsOpen })}`}
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

  .toolbar-badge {
    position: absolute;
    top: 2px;
    right: 2px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 999px;
    background: var(--accent, #5090ff);
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
    pointer-events: none;
  }
</style>
