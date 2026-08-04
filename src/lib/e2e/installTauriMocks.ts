import {
  mockConvertFileSrc,
  mockIPC,
  mockWindows,
} from '@tauri-apps/api/mocks';

const preferences = {
  schemaVersion: 2,
  outputFormat: 'flac',
  outputDir: '/tmp/harubble-e2e',
  downloadLyrics: true,
  notifyOnDownloadComplete: true,
  notifyOnPlaybackChange: true,
  logLevel: 'error',
  locale: 'zh-CN',
  volume: 1,
  theme: {
    presetId: 'harubble-classic',
    customColors: {},
    colorScheme: 'light',
    dynamicAlbumAccent: true,
    activePackageId: null,
    revision: 0,
  },
};

export function installTauriMocks() {
  mockWindows('main');
  mockConvertFileSrc('macos');

  let notificationPermission = 'prompt';
  let listeningHistory = [
    {
      songCid: 'e2e-song-1',
      songName: 'E2E Test Song',
      artists: ['Harubble'],
      coverUrl: null,
      playedAt: new Date().toISOString(),
    },
  ];

  mockIPC(
    (command) => {
      switch (command) {
        case 'get_preferences':
        case 'set_preferences':
          return preferences;
        case 'get_default_output_dir':
          return preferences.outputDir;
        case 'get_album_catalog':
        case 'refresh_album_catalog':
          return { albums: [], revision: 1, checkedAt: Date.now() };
        case 'get_albums':
        case 'get_latest_albums':
        case 'get_albums_by_series':
        case 'get_tag_dimensions':
        case 'get_albums_by_tag_dimension':
        case 'list_collections':
        case 'list_theme_packages':
          return [];
        case 'get_recent_history':
          return listeningHistory;
        case 'clear_listening_history': {
          const removed = listeningHistory.length;
          listeningHistory = [];
          return removed;
        }
        case 'get_homepage_status':
          return {
            platformAlbumCount: 0,
            platformSongCount: 0,
            localDownloadedCount: 0,
            localStorageBytes: 0,
            activeDownloadCount: 0,
            completedDownloadCount: 0,
          };
        case 'get_local_inventory_snapshot':
          return {
            rootOutputDir: preferences.outputDir,
            status: 'completed',
            inventoryVersion: 'e2e-1',
            startedAt: null,
            finishedAt: null,
            scannedFileCount: 0,
            matchedTrackCount: 0,
            verifiedTrackCount: 0,
            lastError: null,
          };
        case 'list_download_jobs':
          return { jobs: [], activeJobId: null, queuedJobIds: [] };
        case 'get_player_state':
          return {
            sessionId: 0,
            songCid: null,
            songName: null,
            artists: [],
            coverUrl: null,
            isPlaying: false,
            isPaused: false,
            isLoading: false,
            hasPrevious: false,
            hasNext: false,
            progress: 0,
            duration: 0,
            volume: 1,
            playbackFormat: null,
          };
        case 'get_notification_permission_state':
          return notificationPermission;
        case 'request_notification_permission':
          notificationPermission = 'granted';
          return notificationPermission;
        case 'get_log_file_status':
          return { hasSessionLog: false, hasPersistentLog: false };
        case 'list_log_records':
          return { records: [], total: 0, kind: 'session' };
        default:
          return null;
      }
    },
    { shouldMockEvents: true }
  );
}
