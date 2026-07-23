<script lang="ts">
  import FullscreenPlayer from '$lib/components/app/player/FullscreenPlayer.svelte';
  import {
    setDownloadContext,
    setPlayerContext,
    setShellContext,
  } from '$lib/contexts';
  import type {
    DownloadContext,
    PlayerContext,
    ShellContext,
  } from '$lib/contexts';

  let open = $state(true);
  let isPlaying = $state(true);
  let progress = $state(82);
  let repeatMode = $state<'off' | 'one' | 'all'>('all');
  let shuffleEnabled = $state(false);

  const song = {
    cid: 'qa-field-audio-01',
    name: 'INDUSTRIAL FIELD RECORD 01',
    artists: ['HARUBBLE AUDIO SYSTEM'],
    coverUrl: null,
  };

  const lyricsLines = [
    { id: 'l1', time: 0, text: '信号穿过空旷的站台' },
    { id: 'l2', time: 26, text: '我们在静默里校准方向' },
    { id: 'l3', time: 64, text: '让所有回声回到此刻' },
    { id: 'l4', time: 98, text: '沿着尚未完成的线路前行' },
    { id: 'l5', time: 136, text: '直到新的坐标被点亮' },
  ];

  setPlayerContext({
    get currentSong() {
      return song;
    },
    get isPlaying() {
      return isPlaying;
    },
    get isPaused() {
      return !isPlaying;
    },
    isLoading: false,
    isPlayTogglePending: false,
    hasPrevious: true,
    hasNext: true,
    get progress() {
      return progress;
    },
    duration: 198,
    get shuffleEnabled() {
      return shuffleEnabled;
    },
    get repeatMode() {
      return repeatMode;
    },
    playbackOrder: [],
    lyricsOpen: false,
    playlistOpen: false,
    lyricsLoading: false,
    lyricsError: null,
    lyricsLines,
    lyricsUnavailable: false,
    activeLyricIndex: 2,
    get fullscreenOpen() {
      return open;
    },
    volume: 0.72,
    muted: false,
    playbackFormat: null,
    pause: () => (isPlaying = false),
    resume: () => (isPlaying = true),
    seek: (value) => (progress = value),
    playPrevious: () => {},
    playNext: () => {},
    toggleShuffle: (value) => (shuffleEnabled = value),
    toggleRepeat: (value) => (repeatMode = value),
    toggleLyrics: () => {},
    togglePlaylist: () => {},
    toggleFullscreen: () => (open = !open),
    setVolume: () => {},
    toggleMute: () => {},
    playQueueEntry: async () => {},
  } satisfies PlayerContext);

  setDownloadContext({
    activeDownloadCount: 0,
    filteredJobs: [],
    hasDownloadHistory: false,
    searchQuery: '',
    scopeFilter: 'all',
    statusFilter: 'all',
    kindFilter: 'all',
    canClearDownloadHistory: () => false,
    getJobProgress: () => 0,
    getJobProgressText: () => '',
    getJobStatusLabel: () => '',
    getJobKindLabel: () => '',
    getJobSummaryLabel: () => '',
    getJobDisplayTitle: () => '',
    getJobErrorSummary: () => null,
    isJobActive: () => false,
    canCancelTask: () => false,
    canRetryTask: () => false,
    getTaskErrorLabel: () => null,
    getTaskStatusLabel: () => '',
    handleClearDownloadHistory: () => {},
    handleCancelDownloadJob: () => {},
    handleRetryDownloadJob: () => {},
    handleCancelDownloadTask: () => {},
    handleRetryDownloadTask: () => {},
    handleSongDownload: () => {},
    getSongDownloadState: () => 'idle',
    isSongDownloadInteractionBlocked: () => false,
  } satisfies DownloadContext);

  setShellContext({
    currentView: 'home',
    isMacOS: false,
    prefersReducedMotion: true,
    settingsOpen: false,
    downloadPanelOpen: false,
    isRefreshing: false,
    overlayScrollbarOptions: {},
    sidebarCollapsed: false,
    toggleSidebar: () => {},
    handleRefresh: () => {},
    handleToggleSettings: () => {},
    handleToggleDownloads: () => {},
    notifyInfo: () => {},
    notifyError: () => {},
    navigate: () => {},
  } satisfies ShellContext);
</script>

{#if open}
  <FullscreenPlayer />
{:else}
  <main class="qa-closed" data-testid="fullscreen-closed">CLOSED</main>
{/if}

<style>
  .qa-closed {
    display: grid;
    width: 100vw;
    height: 100vh;
    place-items: center;
    background: #191919;
    color: #fffa00;
    font-family: var(--font-wide);
    font-size: 48px;
  }
</style>
