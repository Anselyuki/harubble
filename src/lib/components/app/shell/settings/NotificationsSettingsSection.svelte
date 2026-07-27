<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import BellIcon from '@lucide/svelte/icons/bell';

  interface Props {
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    isSendingTestNotification: boolean;
    sectionTitle: string;
    notificationTestLabel: string;
    notificationTestSendingLabel: string;
    lyricsTitle: string;
    lyricsDescription: string;
    notifyDownloadTitle: string;
    notifyDownloadDescription: string;
    notifyPlaybackTitle: string;
    notifyPlaybackDescription: string;
    onSendTestNotification: () => void;
  }

  let {
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    isSendingTestNotification,
    sectionTitle,
    notificationTestLabel,
    notificationTestSendingLabel,
    lyricsTitle,
    lyricsDescription,
    notifyDownloadTitle,
    notifyDownloadDescription,
    notifyPlaybackTitle,
    notifyPlaybackDescription,
    onSendTestNotification,
  }: Props = $props();
</script>

<section class="sheet-section settings-section">
  <div class="settings-section-heading">
    <h3>{sectionTitle}</h3>
    <Button
      variant="secondary"
      disabled={isSendingTestNotification}
      onclick={onSendTestNotification}
      ><BellIcon data-icon="inline-start" />{isSendingTestNotification
        ? notificationTestSendingLabel
        : notificationTestLabel}</Button
    >
  </div>
  <div class="settings-toggle-list">
    <label class="settings-toggle"
      ><span
        ><strong>{lyricsTitle}</strong><small>{lyricsDescription}</small></span
      ><Switch bind:checked={downloadLyrics} /></label
    >
    <label class="settings-toggle"
      ><span
        ><strong>{notifyDownloadTitle}</strong><small
          >{notifyDownloadDescription}</small
        ></span
      ><Switch bind:checked={notifyOnDownloadComplete} /></label
    >
    <label class="settings-toggle"
      ><span
        ><strong>{notifyPlaybackTitle}</strong><small
          >{notifyPlaybackDescription}</small
        ></span
      ><Switch bind:checked={notifyOnPlaybackChange} /></label
    >
  </div>
</section>

<style>
  .settings-toggle-list {
    display: grid;
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-md);
  }
  .settings-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 58px;
    padding: 10px 12px;
    background: var(--sheet-row-bg);
    cursor: pointer;
  }
  .settings-toggle + .settings-toggle {
    border-top: 1px solid var(--sheet-border);
  }
  .settings-toggle:hover {
    background: var(--sheet-row-hover-bg);
  }
  .settings-toggle span {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .settings-toggle strong {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
  }
  .settings-toggle small {
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
  }
</style>
