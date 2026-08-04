import { beforeEach, describe, expect, it, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  exportPreferences: vi.fn(),
  getNotificationPermissionState: vi.fn(),
  importPreferences: vi.fn(),
  rescanLocalInventory: vi.fn(),
  sendTestNotification: vi.fn(),
}));

vi.mock('$lib/api', () => apiMocks);

import { dispatchMenuCommand, type MenuCommandDeps } from './menuCommands';

beforeEach(() => vi.clearAllMocks());

describe('listening history menu command', () => {
  it('requests the shared confirmation without clearing immediately', async () => {
    const requestClearListeningHistory = vi.fn();
    const deps = {
      runtime: { requestClearListeningHistory },
    } as unknown as MenuCommandDeps;

    await dispatchMenuCommand('app.file.clear_listening_history', deps);

    expect(requestClearListeningHistory).toHaveBeenCalledOnce();
  });
});

describe('download history menu command', () => {
  it('requests the shared confirmation without clearing immediately', async () => {
    const requestClearDownloadHistory = vi.fn();
    const deps = {
      runtime: { requestClearDownloadHistory },
    } as unknown as MenuCommandDeps;

    await dispatchMenuCommand('app.file.clear_download_history', deps);

    expect(requestClearDownloadHistory).toHaveBeenCalledOnce();
  });
});

describe('test notification menu command', () => {
  it('opens notification settings without sending when permission is missing', async () => {
    apiMocks.getNotificationPermissionState.mockResolvedValue('denied');
    const openSettingsAt = vi.fn();
    const notifyInfo = vi.fn();
    const deps = {
      runtime: { openSettingsAt },
      notifyInfo,
      notifyError: vi.fn(),
    } as unknown as MenuCommandDeps;

    await dispatchMenuCommand('app.app.test_notification', deps);

    expect(openSettingsAt).toHaveBeenCalledWith('notifications');
    expect(apiMocks.sendTestNotification).not.toHaveBeenCalled();
    expect(notifyInfo).toHaveBeenCalledWith('请先在通知设置中授权系统通知。');
  });

  it('sends only after permission has been granted', async () => {
    apiMocks.getNotificationPermissionState.mockResolvedValue('granted');
    const notifyInfo = vi.fn();
    const deps = {
      runtime: { openSettingsAt: vi.fn() },
      notifyInfo,
      notifyError: vi.fn(),
    } as unknown as MenuCommandDeps;

    await dispatchMenuCommand('app.app.test_notification', deps);

    expect(apiMocks.sendTestNotification).toHaveBeenCalledOnce();
    expect(notifyInfo).toHaveBeenCalledWith('已发送测试通知');
  });
});
