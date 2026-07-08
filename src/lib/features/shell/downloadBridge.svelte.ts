import { shellStore } from '$lib/features/shell/store.svelte';
import type { createDownloadController } from '$lib/features/download/controller.svelte';
import type { createPlayerController } from '$lib/features/player/controller.svelte';

interface DownloadBridgeDeps {
  downloadController: ReturnType<typeof createDownloadController>;
  playerController: ReturnType<typeof createPlayerController>;
  clearSongSelection: () => void;
  setSelectionModeEnabled: (value: boolean) => void;
  notifyError: (message: string) => void;
}

export function createDownloadBridge(deps: DownloadBridgeDeps) {
  function handleToggleDownloads() {
    void shellStore.toggleDownloads({ notifyError: deps.notifyError });
  }

  function handleToggleSettings() {
    void shellStore.toggleSettings({ notifyError: deps.notifyError });
  }

  function handleDownloadSelection(songCids: string[]) {
    void deps.downloadController.handleSelectionDownload(songCids, {
      afterCreated: () => {
        deps.clearSongSelection();
        deps.setSelectionModeEnabled(false);
      },
    });
  }

  function handleCurrentSongDownload() {
    const currentSong = deps.playerController.currentSong;
    if (currentSong) {
      void deps.downloadController.handleSongDownload(currentSong.cid);
    }
  }

  function hasAlbumDownloadJob(albumCid: string): boolean {
    return !!deps.downloadController.findAlbumDownloadJob(albumCid);
  }

  function hasCurrentSelectionJob(songCids: string[]): boolean {
    return !!deps.downloadController.getCurrentSelectionJob(songCids);
  }

  return {
    handleToggleDownloads,
    handleToggleSettings,
    handleDownloadSelection,
    handleCurrentSongDownload,
    hasAlbumDownloadJob,
    hasCurrentSelectionJob,
  };
}

export type DownloadBridge = ReturnType<typeof createDownloadBridge>;
