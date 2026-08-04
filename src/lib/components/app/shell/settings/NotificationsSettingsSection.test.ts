// @vitest-environment jsdom

import { cleanup, fireEvent, render } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import NotificationsSettingsSection from './NotificationsSettingsSection.svelte';

afterEach(cleanup);

function renderSection(permission: 'granted' | 'prompt') {
  const onSendTestNotification = vi.fn();
  const onRequestNotificationPermission = vi.fn();
  const view = render(NotificationsSettingsSection, {
    props: {
      isSendingTestNotification: false,
      isRequestingNotificationPermission: false,
      notificationPermissionState: permission,
      sectionTitle: 'Notifications',
      notificationTestLabel: 'Test',
      notificationTestSendingLabel: 'Sending',
      notificationPermissionLabel: 'System notification permission',
      notificationPermissionStatus:
        permission === 'granted' ? 'Allowed' : 'Not requested',
      notificationPermissionRequestLabel: 'Allow',
      notificationPermissionRequestingLabel: 'Requesting',
      lyricsTitle: 'Lyrics',
      lyricsDescription: 'Download lyrics',
      notifyDownloadTitle: 'Downloads',
      notifyDownloadDescription: 'Notify when downloads finish',
      notifyPlaybackTitle: 'Playback',
      notifyPlaybackDescription: 'Notify when playback changes',
      onSendTestNotification,
      onRequestNotificationPermission,
    },
  });

  return { view, onSendTestNotification, onRequestNotificationPermission };
}

describe('NotificationsSettingsSection permission boundary', () => {
  it('keeps testing disabled until permission is granted', async () => {
    const { view, onSendTestNotification, onRequestNotificationPermission } =
      renderSection('prompt');

    const testButton = view.getByRole('button', { name: 'Test' });
    expect(testButton).toBeDisabled();
    await fireEvent.click(testButton);
    expect(onSendTestNotification).not.toHaveBeenCalled();

    await fireEvent.click(view.getByRole('button', { name: 'Allow' }));
    expect(onRequestNotificationPermission).toHaveBeenCalledOnce();
    expect(onSendTestNotification).not.toHaveBeenCalled();
  });

  it('allows an explicit test after permission is granted', async () => {
    const { view, onSendTestNotification } = renderSection('granted');

    await fireEvent.click(view.getByRole('button', { name: 'Test' }));
    expect(onSendTestNotification).toHaveBeenCalledOnce();
    expect(
      view.queryByRole('button', { name: 'Allow' })
    ).not.toBeInTheDocument();
  });
});
