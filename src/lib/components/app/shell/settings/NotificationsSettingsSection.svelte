<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import BellIcon from '@lucide/svelte/icons/bell';
  import type { NotificationPermissionState } from '$lib/api';

  interface Props {
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    isSendingTestNotification: boolean;
    isRequestingNotificationPermission: boolean;
    notificationPermissionState: NotificationPermissionState | null;
    sectionTitle: string;
    notificationTestLabel: string;
    notificationTestSendingLabel: string;
    notificationPermissionLabel: string;
    notificationPermissionStatus: string;
    notificationPermissionRequestLabel: string;
    notificationPermissionRequestingLabel: string;
    lyricsTitle: string;
    lyricsDescription: string;
    notifyDownloadTitle: string;
    notifyDownloadDescription: string;
    notifyPlaybackTitle: string;
    notifyPlaybackDescription: string;
    onSendTestNotification: () => void;
    onRequestNotificationPermission: () => void;
  }

  let {
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    isSendingTestNotification,
    isRequestingNotificationPermission,
    notificationPermissionState,
    sectionTitle,
    notificationTestLabel,
    notificationTestSendingLabel,
    notificationPermissionLabel,
    notificationPermissionStatus,
    notificationPermissionRequestLabel,
    notificationPermissionRequestingLabel,
    lyricsTitle,
    lyricsDescription,
    notifyDownloadTitle,
    notifyDownloadDescription,
    notifyPlaybackTitle,
    notifyPlaybackDescription,
    onSendTestNotification,
    onRequestNotificationPermission,
  }: Props = $props();
</script>

<section class="sheet-section settings-section">
  <div class="settings-section-heading">
    <h3>{sectionTitle}</h3>
    <Button
      variant="secondary"
      disabled={notificationPermissionState !== 'granted' ||
        isSendingTestNotification}
      onclick={() => {
        if (
          notificationPermissionState === 'granted' &&
          !isSendingTestNotification
        ) {
          onSendTestNotification();
        }
      }}
      ><BellIcon data-icon="inline-start" />{isSendingTestNotification
        ? notificationTestSendingLabel
        : notificationTestLabel}</Button
    >
  </div>
  <div class="notification-permission" aria-live="polite">
    <span>
      <strong>{notificationPermissionLabel}</strong>
      <small>{notificationPermissionStatus}</small>
    </span>
    {#if notificationPermissionState !== 'granted'}
      <Button
        variant="outline"
        disabled={isRequestingNotificationPermission}
        onclick={onRequestNotificationPermission}
      >
        {isRequestingNotificationPermission
          ? notificationPermissionRequestingLabel
          : notificationPermissionRequestLabel}
      </Button>
    {/if}
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
  .notification-permission {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 58px;
    padding: 10px 12px;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-md);
    background: var(--sheet-row-bg);
  }
  .notification-permission span {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .notification-permission strong {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
  }
  .notification-permission small {
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
  }
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
