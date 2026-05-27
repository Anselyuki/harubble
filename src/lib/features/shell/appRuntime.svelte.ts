import { tick } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import type { PartialOptions } from 'overlayscrollbars';
import { gsapScrollIntoView } from '$lib/design/gsap';
import {
  getAlbums,
  getAlbumDetail,
  getDefaultOutputDir,
  playSong,
  pausePlayback,
  resumePlayback,
  seekCurrentPlayback,
  getPlayerState,
  setPlaybackVolume,
  clearResponseCache,
  resetHttpClient,
  extractImageTheme,
  getImageDataUrl,
  getSongLyrics,
  createDownloadJob,
  listDownloadJobs,
  cancelDownloadJob,
  cancelDownloadTask,
  retryDownloadJob,
  retryDownloadTask,
  clearDownloadHistory,
  getPreferences,
  setPreferences,
  getLocalInventorySnapshot,
  searchLibrary,
  getLatestAlbums,
  getAlbumsBySeriesGroup,
  getRecentHistory,
  getHomepageStatus,
  clearListeningHistory,
  getTagDimensions,
  getAlbumsByTagDimension,
  getTagEditorMerged,
  getTagEditorLocalOverlay,
  setTagEditorEntityTag,
  removeTagEditorEntityTag,
  addTagEditorDimension,
  removeTagEditorDimension,
  applyTagEditorRemoteUpdate,
  resolveTagEditorConflict,
  exportTagEditorRegistry,
  importTagEditorRegistry,
} from '$lib/api';
import {
  clearCache,
  createInventoryCacheTag,
  invalidateByTag,
  warmCacheManager,
} from '$lib/cache';
import type {
  Album,
  AlbumDetail,
  OutputFormat,
  SongEntry,
  PlaybackQueueEntry,
  AppErrorEvent,
  LogLevel,
  SearchLibraryResultItem,
  ThemePalette,
} from '$lib/types';
import { applyThemePalette, DEFAULT_THEME_PALETTE } from '$lib/theme';
import { envStore } from '$lib/features/env/store.svelte';
import { shellStore, type AppView } from '$lib/features/shell/store.svelte';
import {
  navigationStack,
  isSameEntry,
  type NavigationEntry,
} from './navigation.svelte';
import { createSettingsController } from '$lib/features/shell/settings.svelte';
import { createAlbumStageMotionController } from '$lib/features/shell/albumStageMotion.svelte';
import { createLibraryController } from '$lib/features/library/controller.svelte';
import { createPlayerController } from '$lib/features/player/controller.svelte';
import { createDownloadController } from '$lib/features/download/controller.svelte';
import { createHomeController } from '$lib/features/home/controller.svelte';
import { createTagEditorController } from '$lib/features/tagEditor/controller.svelte';
import { tagEditorStore } from '$lib/features/tagEditor/store.svelte';
import { createCollectionController } from '$lib/features/collection/controller.svelte';
import { createSearchController } from '$lib/features/search/controller.svelte';
import {
  listCollections,
  getCollection,
  createCollection,
  updateCollection,
  deleteCollection,
  addSongsToCollection,
  removeSongsFromCollection,
  reorderCollectionSongs,
  exportCollection,
  importCollection,
} from '$lib/collectionApi';
import { preloadImage } from '$lib/features/library/helpers';
import {
  buildAlbumPlaybackEntries,
  getSelectedAlbumCoverUrl,
} from '$lib/features/library/selectors';
import {
  bootstrapApp,
  subscribeToTauriEvents,
} from '$lib/features/shell/appRuntimeBootstrap.svelte';
import { localeState, type Locale } from '$lib/i18n';
import * as m from '$lib/paraglide/messages.js';
import { toast } from 'svelte-sonner';

const MIN_DISPLAY_MS = 260;
const DETAIL_SKELETON_DELAY_MS = 140;

const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, ms);
  });

export function createAppRuntime() {
  function notifyInfo(message: string) {
    toast(message);
  }

  function notifyError(message: string) {
    toast.error(message);
  }

  async function preloadAlbumArtwork(
    album: AlbumDetail
  ): Promise<number | null> {
    const sourceUrl = album.coverDeUrl ?? album.coverUrl;
    const resolvedUrl = await getImageDataUrl(sourceUrl).catch(() => sourceUrl);
    const meta = await preloadImage(resolvedUrl);
    return meta?.aspectRatio ?? null;
  }

  function setAlbumStageAspectRatio(value: number | null | undefined) {
    albumStageMotionController.setAspectRatio(value);
  }

  const libraryController = createLibraryController({
    delay,
    detailSkeletonDelayMs: DETAIL_SKELETON_DELAY_MS,
    minDetailDisplayMs: MIN_DISPLAY_MS,
    getAlbums,
    getAlbumDetail,
    searchLibrary,
    preloadAlbumArtwork,
    setAlbumStageAspectRatio,
    notifyError,
  });

  const playerController = createPlayerController({
    playSong: async (songCid, coverUrl, context) => {
      await playSong(songCid, coverUrl ?? undefined, context ?? undefined);
    },
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback: async (positionSecs) => {
      await seekCurrentPlayback(positionSecs);
    },
    setPlaybackVolume,
    getSongLyrics,
    notifyError,
  });

  const downloadController = createDownloadController({
    createDownloadJob,
    cancelDownloadJob,
    cancelDownloadTask,
    retryDownloadJob,
    retryDownloadTask,
    clearDownloadHistory,
    openDownloadPanel: async (resetFilters = false) => {
      await shellStore.openDownloads({
        notifyError,
        beforeOpen: resetFilters
          ? () => {
              downloadController.resetFilters();
            }
          : undefined,
      });
    },
    getDownloadOptions: () => ({
      outputDir: settingsState.outputDir,
      format: settingsState.format,
      downloadLyrics: settingsState.downloadLyrics,
    }),
    notifyInfo,
    notifyError,
  });

  const settingsController = createSettingsController({
    getPreferences,
    setPreferences,
    notifyError,
    onLocaleChanged: (locale) => localeState.applyBackendLocale(locale),
  });

  const albumStageMotionController = createAlbumStageMotionController({
    getReducedMotion: () => envStore.prefersReducedMotion,
    getViewportHeight: () => envStore.viewportHeight,
    getLoadingDetail: () => libraryController.loadingDetail,
  });

  const homeController = createHomeController({
    getLatestAlbums,
    getAlbumsBySeriesGroup,
    getRecentHistory,
    getHomepageStatus,
    clearListeningHistory,
    getTagDimensions,
    getAlbumsByTagDimension,
    notifyError,
  });

  const tagEditorController = createTagEditorController({
    getTagEditorMerged,
    getTagEditorLocalOverlay,
    setTagEditorEntityTag,
    removeTagEditorEntityTag,
    addTagEditorDimension,
    removeTagEditorDimension,
    applyTagEditorRemoteUpdate,
    resolveTagEditorConflict,
    exportTagEditorRegistry,
    importTagEditorRegistry,
    getAlbumDetail: (albumCid: string) => getAlbumDetail(albumCid),
    getAlbums: () => libraryController.albums,
    notifyError,
  });

  const searchController = createSearchController({
    getRecentHistory,
    getAlbums: () => libraryController.albums,
    notifyError,
  });

  const collectionController = createCollectionController({
    listCollections,
    getCollection,
    createCollection,
    updateCollection,
    deleteCollection,
    addSongsToCollection,
    removeSongsFromCollection,
    reorderCollectionSongs,
    exportCollection,
    importCollection,
    navigateToCollection: shellStore.navigateToCollection,
    notifyInfo,
    notifyError,
  });

  let selectedSongCids = $state<string[]>([]);
  let selectionModeEnabled = $state(false);
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let playerStateInitSeq = 0;
  let playerStateHydratedFromEvent = false;
  let navigationSeq = 0;
  let isNavigating = $state(false);

  const settingsOpen = $derived(shellStore.settingsOpen);
  const downloadPanelOpen = $derived(shellStore.downloadPanelOpen);
  const SettingsSheetView = $derived(shellStore.SettingsSheetView);
  const DownloadTasksSheetView = $derived(shellStore.DownloadTasksSheetView);
  const contentScrollbarEvents = $derived(
    albumStageMotionController.contentScrollbarEvents
  );
  const albumStageStyle = $derived(albumStageMotionController.albumStageStyle);
  const albumStageMediaHeight = $derived(
    albumStageMotionController.albumStageMediaHeight
  );
  const albumStageScrimOpacity = $derived(
    albumStageMotionController.albumStageScrimOpacity
  );
  const albumStageImageOpacity = $derived(
    albumStageMotionController.albumStageImageOpacity
  );
  const albumStageImageTransform = $derived(
    albumStageMotionController.albumStageImageTransform
  );
  const albumStageSolidifyOpacity = $derived(
    albumStageMotionController.albumStageSolidifyOpacity
  );
  const prefersReducedMotion = $derived(envStore.prefersReducedMotion);
  const albums = $derived(libraryController.albums);
  const selectedAlbum = $derived(libraryController.selectedAlbum);
  const selectedAlbumCid = $derived(libraryController.selectedAlbumCid);
  const loadingAlbums = $derived(libraryController.loadingAlbums);
  const loadingDetail = $derived(libraryController.loadingDetail);
  const errorMsg = $derived(libraryController.errorMsg);
  const librarySearchQuery = $derived(libraryController.librarySearchQuery);
  const librarySearchScope = $derived(libraryController.librarySearchScope);
  const librarySearchLoading = $derived(libraryController.librarySearchLoading);
  const librarySearchResponse = $derived(
    libraryController.librarySearchResponse
  );
  const pendingScrollToSongCid = $derived(
    libraryController.pendingScrollToSongCid
  );
  const showDetailSkeleton = $derived(libraryController.showDetailSkeleton);
  const albumRequestSeq = $derived(libraryController.albumRequestSeq);
  const currentSong = $derived(playerController.currentSong);
  const isPlaying = $derived(playerController.isPlaying);
  const isPaused = $derived(playerController.isPaused);
  const isLoading = $derived(playerController.isLoading);
  const progress = $derived(playerController.progress);
  const duration = $derived(playerController.duration);
  const shuffleEnabled = $derived(playerController.shuffleEnabled);
  const repeatMode = $derived(playerController.repeatMode);
  const playbackOrder = $derived(playerController.playbackOrder);
  const lyricsOpen = $derived(playerController.lyricsOpen);
  const playlistOpen = $derived(playerController.playlistOpen);
  const lyricsLoading = $derived(playerController.lyricsLoading);
  const lyricsError = $derived(playerController.lyricsError);
  const lyricsLines = $derived(playerController.lyricsLines);
  const lyricsUnavailable = $derived(playerController.lyricsUnavailable);
  const fullscreenOpen = $derived(playerController.fullscreenOpen);
  const downloadingAlbumCid = $derived(downloadController.downloadingAlbumCid);
  const activeDownloadCount = $derived(downloadController.activeDownloadCount);
  const filteredDownloadJobs = $derived(downloadController.filteredJobs);
  const hasDownloadHistory = $derived(downloadController.hasDownloadHistory);
  const contentEl = $derived(albumStageMotionController.contentElement);
  const isMacOS = $derived(envStore.isMacOS);

  const settingsState = $state({
    format: 'flac' as OutputFormat,
    outputDir: '',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error' as LogLevel,
    locale: 'zh-CN' as Locale,
    volume: 1,
    settingsLogRefreshToken: 0,
    prefsReady: false,
    isSaving: false,
    persistedSnapshot: '',
    lastSaveFailedSnapshot: '',
    dirty: {
      format: false,
      outputDir: false,
      downloadLyrics: false,
      notifyOnDownloadComplete: false,
      notifyOnPlaybackChange: false,
      logLevel: false,
      locale: false,
    },
    suspendDirtyTracking: 0,
  });

  let selectedAlbumArtworkUrl = $state<string | null>(null);
  let albumStageElement = $state<HTMLElement | null>(null);
  let activeThemeArtworkUrl: string | null = null;
  let cachedAlbumPalette = $state<ThemePalette | null>(null);
  let activeAlbumStageArtworkUrl: string | null = null;
  const lastObservedSettings = {
    format: settingsState.format,
    outputDir: settingsState.outputDir,
    downloadLyrics: settingsState.downloadLyrics,
    notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
    notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
    logLevel: settingsState.logLevel,
    locale: settingsState.locale,
  };

  const playerHasPrevious = $derived(playerController.playerHasPrevious);
  const playerHasNext = $derived(playerController.playerHasNext);

  const activeLyricIndex = $derived.by(() => {
    if (!lyricsOpen && !fullscreenOpen) return -1;
    let activeIndex = -1;
    for (let index = 0; index < lyricsLines.length; index += 1) {
      const lineTime = lyricsLines[index].time;
      if (lineTime === null) continue;
      if (progress + 0.08 >= lineTime) {
        activeIndex = index;
      } else {
        break;
      }
    }
    return activeIndex;
  });

  const overlayScrollbarOptions = $derived.by(
    (): PartialOptions => ({
      scrollbars: {
        theme: 'os-theme-app',
        autoHide: prefersReducedMotion ? 'leave' : 'move',
        autoHideDelay: prefersReducedMotion ? 160 : 720,
        autoHideSuspend: true,
        dragScroll: true,
        clickScroll: false,
      },
    })
  );

  let isRefreshing = $state(false);

  // --- PLACEHOLDER_HELPERS ---

  function resetContentScroll() {
    albumStageMotionController.resetContentScroll();
  }

  function isSongSelected(songCid: string): boolean {
    return selectedSongCids.includes(songCid);
  }

  function toggleSongSelection(songCid: string) {
    if (selectedSongCids.includes(songCid)) {
      selectedSongCids = selectedSongCids.filter((cid) => cid !== songCid);
      return;
    }
    selectedSongCids = [...selectedSongCids, songCid];
  }

  function clearSongSelection() {
    selectedSongCids = [];
  }

  function selectAllSongs() {
    if (!selectedAlbum) return;
    selectedSongCids = selectedAlbum.songs.map((s: SongEntry) => s.cid);
  }

  function deselectAllSongs() {
    selectedSongCids = [];
  }

  function invertSongSelection() {
    if (!selectedAlbum) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- ephemeral, non-reactive lookup
    const allCids = new Set(selectedAlbum.songs.map((s: SongEntry) => s.cid));
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- ephemeral, non-reactive lookup
    const currentSelected = new Set(selectedSongCids);
    selectedSongCids = [...allCids].filter((cid) => !currentSelected.has(cid));
  }

  function toggleSelectionMode() {
    selectionModeEnabled = !selectionModeEnabled;
    if (!selectionModeEnabled) {
      clearSongSelection();
    }
  }

  function getSettingsSnapshot() {
    return JSON.stringify({
      format: settingsState.format,
      outputDir: settingsState.outputDir,
      downloadLyrics: settingsState.downloadLyrics,
      notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
      notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
      logLevel: settingsState.logLevel,
      locale: settingsState.locale,
    });
  }

  function handleContentWheel(event: WheelEvent) {
    albumStageMotionController.handleContentWheel(event);
  }

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    settingsController.handleAppError(settingsState, settingsOpen);
  }

  async function invalidateInventoryCaches(
    inventoryVersion: string | null | undefined
  ) {
    await invalidateByTag(createInventoryCacheTag(inventoryVersion));
  }

  function captureCurrentEntry(): NavigationEntry {
    const view = shellStore.currentView;
    switch (view) {
      case 'home':
        return { view: 'home' };
      case 'search':
        return { view: 'search' };
      case 'overview':
        return { view: 'overview' };
      case 'library':
        return {
          view: 'library',
          albumCid: libraryController.selectedAlbumCid,
        };
      case 'collection':
        return {
          view: 'collection',
          collectionId: collectionController.selectedCollectionId ?? '',
        };
      case 'tagEditor':
        return {
          view: 'tagEditor',
          albumCid: tagEditorStore.editingAlbum?.cid ?? null,
          songCid: tagEditorStore.editingSong?.cid ?? null,
        };
    }
  }

  function clearNonTargetState(targetView: AppView): void {
    if (targetView !== 'library') {
      libraryController.deselectAlbum();
    }
    clearSongSelection();
    selectionModeEnabled = false;
    if (targetView !== 'collection') {
      collectionController.clearSelection();
    }
    if (targetView !== 'tagEditor') {
      tagEditorStore.reset();
    }
  }

  function navigateToTop(view: AppView): void {
    const current = captureCurrentEntry();
    const target: NavigationEntry =
      view === 'library'
        ? { view: 'library', albumCid: null }
        : view === 'collection'
          ? { view: 'collection', collectionId: '' }
          : view === 'tagEditor'
            ? { view: 'tagEditor', albumCid: null, songCid: null }
            : { view };

    if (isSameEntry(current, target)) return;

    navigationSeq++;
    navigationStack.push(current);
    clearNonTargetState(view);
    shellStore.currentView = view;
  }

  async function openAlbum(
    album: Album,
    options?: { pendingSongCid?: string | null }
  ): Promise<void> {
    const current = captureCurrentEntry();
    const target: NavigationEntry = { view: 'library', albumCid: album.cid };
    if (isSameEntry(current, target)) return;

    const seq = ++navigationSeq;
    isNavigating = true;
    navigationStack.push(current);
    clearNonTargetState('library');
    shellStore.currentView = 'library';

    const shouldDispose = () => seq !== navigationSeq;

    if (options?.pendingSongCid !== undefined) {
      libraryController.setPendingScrollToSong(options.pendingSongCid);
    }

    try {
      await libraryController.selectAlbum(album, {
        afterSelect: async () => {
          if (shouldDispose()) return;
          await tick();
          resetContentScroll();
        },
        shouldDispose,
      });
    } finally {
      if (seq === navigationSeq) {
        isNavigating = false;
      }
    }
  }

  async function openCollection(collectionId: string): Promise<void> {
    const current = captureCurrentEntry();
    const target: NavigationEntry = { view: 'collection', collectionId };
    if (isSameEntry(current, target)) return;

    const seq = ++navigationSeq;
    isNavigating = true;
    navigationStack.push(current);
    clearNonTargetState('collection');
    shellStore.currentView = 'collection';

    const shouldDispose = () => seq !== navigationSeq;

    try {
      await collectionController.loadAndSelect(collectionId);
      if (shouldDispose()) return;
    } finally {
      if (seq === navigationSeq) {
        isNavigating = false;
      }
    }
  }

  async function openTagEditor(album: Album): Promise<void> {
    const current = captureCurrentEntry();
    const target: NavigationEntry = {
      view: 'tagEditor',
      albumCid: album.cid,
      songCid: null,
    };
    if (isSameEntry(current, target)) return;

    const seq = ++navigationSeq;
    isNavigating = true;
    navigationStack.push(current);
    clearNonTargetState('tagEditor');
    shellStore.currentView = 'tagEditor';

    const shouldDispose = () => seq !== navigationSeq;

    try {
      await tagEditorController.selectAlbumForEditAsync(album, shouldDispose);
      if (shouldDispose()) return;
    } finally {
      if (seq === navigationSeq) {
        isNavigating = false;
      }
    }
  }

  async function goBack(): Promise<void> {
    if (!navigationStack.canGoBack) return;

    const entry = navigationStack.pop()!;
    const seq = ++navigationSeq;
    isNavigating = true;
    clearNonTargetState(entry.view);
    shellStore.currentView = entry.view;

    const shouldDispose = () => seq !== navigationSeq;

    try {
      switch (entry.view) {
        case 'library': {
          if (entry.albumCid) {
            const album = albums.find((a) => a.cid === entry.albumCid);
            if (album) {
              await libraryController.selectAlbum(album, {
                afterSelect: async () => {
                  if (shouldDispose()) return;
                  await tick();
                  resetContentScroll();
                },
                shouldDispose,
              });
            }
          }
          break;
        }
        case 'collection': {
          if (entry.collectionId) {
            await collectionController.restoreSelection(entry.collectionId);
          }
          break;
        }
        case 'tagEditor': {
          await tagEditorController.restoreEditingState(
            entry.albumCid,
            entry.songCid,
            shouldDispose
          );
          break;
        }
      }
    } finally {
      if (seq === navigationSeq) {
        isNavigating = false;
      }
    }
  }

  async function handleSelectAlbum(album: Album) {
    await openAlbum(album);
  }

  async function handleSelectSearchResult(item: SearchLibraryResultItem) {
    const album = albums.find((candidate) => candidate.cid === item.albumCid);
    if (!album) {
      notifyError(m.app_error_album_not_found());
      return;
    }
    await openAlbum(album, {
      pendingSongCid: item.kind === 'song' ? item.songCid : null,
    });
  }

  async function handlePlay(song: SongEntry) {
    const sourceEntries = buildAlbumPlaybackEntries(selectedAlbum);
    const fallbackEntry: PlaybackQueueEntry = {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: getSelectedAlbumCoverUrl(selectedAlbum),
    };
    const entries = sourceEntries.length ? sourceEntries : [fallbackEntry];
    playerController.applyPlaybackQueue(entries, song.cid);
    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;
    await playerController.playQueueEntry(
      nextOrder[nextIndex],
      nextOrder,
      nextIndex
    );
  }

  async function handlePlayCollectionSong(
    song: SongEntry,
    queue: PlaybackQueueEntry[]
  ) {
    const entries = queue.length
      ? queue
      : [
          {
            cid: song.cid,
            name: song.name,
            artists: song.artists,
            coverUrl: null,
          },
        ];
    playerController.applyPlaybackQueue(entries, song.cid);
    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;
    await playerController.playQueueEntry(
      nextOrder[nextIndex],
      nextOrder,
      nextIndex
    );
  }

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;
    clearSongSelection();
    selectionModeEnabled = false;
    try {
      await clearCache();
      await clearResponseCache();
      await libraryController.reloadAlbumsAndRefreshCurrentSelection({
        afterSelect: async () => {
          await tick();
          resetContentScroll();
        },
      });
    } catch (e) {
      notifyError(
        m.app_error_refresh_failed({
          error: e instanceof Error ? e.message : String(e),
        })
      );
    } finally {
      await delay(400);
      isRefreshing = false;
    }
  }

  function handleToggleDownloads() {
    void shellStore.toggleDownloads({ notifyError });
  }

  function handleToggleSettings() {
    void shellStore.toggleSettings({ notifyError });
  }

  function handleOutputDirChange() {
    return settingsController.savePreferences(settingsState);
  }

  function handleDownloadSelection(songCids: string[]) {
    void downloadController.handleSelectionDownload(songCids, {
      afterCreated: () => {
        clearSongSelection();
        selectionModeEnabled = false;
      },
    });
  }

  function handleCurrentSongDownload() {
    if (currentSong) {
      void downloadController.handleSongDownload(currentSong.cid);
    }
  }

  function hasAlbumDownloadJob(albumCid: string): boolean {
    return !!downloadController.findAlbumDownloadJob(albumCid);
  }

  function hasCurrentSelectionJob(songCids: string[]): boolean {
    return !!downloadController.getCurrentSelectionJob(songCids);
  }

  $effect(() => {
    const artworkUrl =
      selectedAlbum?.coverUrl ?? selectedAlbum?.coverDeUrl ?? null;
    if (artworkUrl === activeThemeArtworkUrl) {
      return;
    }

    activeThemeArtworkUrl = artworkUrl;
    const paletteRequestSeq = ++themeRequestSeq;

    if (!artworkUrl) {
      cachedAlbumPalette = null;
      return;
    }

    void (async () => {
      try {
        const palette = await extractImageTheme(artworkUrl);
        if (paletteRequestSeq !== themeRequestSeq) return;
        cachedAlbumPalette = palette;
      } catch {
        if (paletteRequestSeq !== themeRequestSeq) return;
        cachedAlbumPalette = null;
      }
    })();
  });

  $effect(() => {
    const shouldApplyAlbumTheme =
      shellStore.currentView === 'library' || fullscreenOpen;
    if (shouldApplyAlbumTheme && cachedAlbumPalette) {
      applyThemePalette(cachedAlbumPalette);
    } else {
      applyThemePalette(DEFAULT_THEME_PALETTE);
    }
  });

  $effect(() => {
    const sourceUrl =
      selectedAlbum?.coverDeUrl ?? selectedAlbum?.coverUrl ?? null;
    if (sourceUrl === activeAlbumStageArtworkUrl) {
      return;
    }

    activeAlbumStageArtworkUrl = sourceUrl;
    const requestSeq = ++artworkRequestSeq;

    if (!sourceUrl) {
      selectedAlbumArtworkUrl = null;
      return;
    }

    void (async () => {
      try {
        const dataUrl = await getImageDataUrl(sourceUrl);
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = dataUrl;
      } catch {
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = null;
      }
    })();
  });

  $effect(() => {
    const value = settingsState.format;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.format = value;
      return;
    }
    if (value !== lastObservedSettings.format) {
      settingsState.dirty.format = true;
      lastObservedSettings.format = value;
    }
  });

  $effect(() => {
    const value = settingsState.outputDir;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.outputDir = value;
      return;
    }
    if (value !== lastObservedSettings.outputDir) {
      settingsState.dirty.outputDir = true;
      lastObservedSettings.outputDir = value;
    }
  });

  $effect(() => {
    const value = settingsState.downloadLyrics;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.downloadLyrics = value;
      return;
    }
    if (value !== lastObservedSettings.downloadLyrics) {
      settingsState.dirty.downloadLyrics = true;
      lastObservedSettings.downloadLyrics = value;
    }
  });

  $effect(() => {
    const value = settingsState.notifyOnDownloadComplete;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.notifyOnDownloadComplete = value;
      return;
    }
    if (value !== lastObservedSettings.notifyOnDownloadComplete) {
      settingsState.dirty.notifyOnDownloadComplete = true;
      lastObservedSettings.notifyOnDownloadComplete = value;
    }
  });

  $effect(() => {
    const value = settingsState.notifyOnPlaybackChange;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.notifyOnPlaybackChange = value;
      return;
    }
    if (value !== lastObservedSettings.notifyOnPlaybackChange) {
      settingsState.dirty.notifyOnPlaybackChange = true;
      lastObservedSettings.notifyOnPlaybackChange = value;
    }
  });

  $effect(() => {
    const value = settingsState.logLevel;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.logLevel = value;
      return;
    }
    if (value !== lastObservedSettings.logLevel) {
      settingsState.dirty.logLevel = true;
      lastObservedSettings.logLevel = value;
    }
  });

  $effect(() => {
    const value = settingsState.locale;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.locale = value;
      return;
    }
    if (value !== lastObservedSettings.locale) {
      settingsState.dirty.locale = true;
      lastObservedSettings.locale = value;
    }
  });

  $effect(() => {
    const { persistedSnapshot, isSaving, lastSaveFailedSnapshot, prefsReady } =
      settingsState;
    if (!prefsReady || isSaving) {
      return;
    }

    const currentSnapshot = getSettingsSnapshot();

    if (currentSnapshot === persistedSnapshot) {
      return;
    }

    if (currentSnapshot === lastSaveFailedSnapshot) {
      return;
    }

    void settingsController.savePreferences(settingsState);
  });

  $effect(() => {
    albumStageMotionController.albumStageElement = albumStageElement;
  });

  async function doBootstrapApp(shouldDispose: () => boolean) {
    await bootstrapApp(
      {
        warmCacheManager,
        settingsController,
        settingsState,
        libraryController,
        getDefaultOutputDir,
        getLocalInventorySnapshot,
        clearSongSelection,
        setSelectionModeEnabled: (value) => {
          selectionModeEnabled = value;
        },
        resetContentScroll,
        tick,
        downloadController,
        listDownloadJobs,
        playerController,
        getPlayerState,
        getPlayerStateInitSeq: () => playerStateInitSeq,
        incrementPlayerStateInitSeq: () => ++playerStateInitSeq,
        getPlayerStateHydratedFromEvent: () => playerStateHydratedFromEvent,
        homeController,
      },
      shouldDispose
    );
  }

  async function doSubscribeToTauriEvents(shouldDispose: () => boolean) {
    return subscribeToTauriEvents(
      {
        listen,
        playerController,
        downloadController,
        libraryController,
        homeController,
        handleAppErrorEvent,
        clearSongSelection,
        setSelectionModeEnabled: (value) => {
          selectionModeEnabled = value;
        },
        invalidateInventoryCaches,
        setPlayerStateHydratedFromEvent: (value) => {
          playerStateHydratedFromEvent = value;
        },
      },
      shouldDispose
    );
  }

  function teardownAppRuntime(unsubscribe: (() => void) | null) {
    shellStore.dispose();
    envStore.dispose();
    libraryController.dispose();
    playerController.dispose();
    downloadController.dispose();
    albumStageMotionController.dispose();
    homeController.dispose();
    tagEditorController.dispose();
    searchController.dispose();
    playerStateInitSeq += 1;
    playerStateHydratedFromEvent = false;
    unsubscribe?.();
  }

  $effect(() => {
    libraryController.init();
    playerController.init();
    downloadController.init();
    envStore.init();
    shellStore.init();
    homeController.init();
    void collectionController.loadCollections();

    let disposed = false;
    let unsubscribe: (() => void) | null = null;

    const handleOnline = () => {
      void resetHttpClient().catch(() => {});
    };
    window.addEventListener('online', handleOnline);

    void (async () => {
      try {
        const nextUnsubscribe = await doSubscribeToTauriEvents(() => disposed);
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- async race guard
        if (disposed) {
          nextUnsubscribe();
          return;
        }
        unsubscribe = nextUnsubscribe;

        await doBootstrapApp(() => disposed);
      } catch (error) {
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- async race guard
        if (disposed) {
          return;
        }
        notifyError(
          m.app_error_init_failed({
            error: error instanceof Error ? error.message : String(error),
          })
        );
      }
    })();

    return () => {
      disposed = true;
      window.removeEventListener('online', handleOnline);
      teardownAppRuntime(unsubscribe);
    };
  });

  $effect(() => {
    playerController.syncPlaybackLifecycle();
  });

  $effect(() => {
    if (!pendingScrollToSongCid || !selectedAlbum || loadingDetail) {
      return;
    }

    const expectedSongCid = pendingScrollToSongCid;
    void tick().then(() => {
      if (pendingScrollToSongCid !== expectedSongCid || !contentEl) {
        return;
      }

      const row = contentEl.querySelector<HTMLElement>(
        `[data-song-cid="${CSS.escape(expectedSongCid)}"]`
      );
      if (!row) {
        return;
      }

      gsapScrollIntoView(contentEl, row, 'center');
      libraryController.clearPendingScrollToSong(expectedSongCid);
    });
  });

  let sidebarStateBeforeTagEditor: boolean | null = null;

  $effect(() => {
    const view = shellStore.currentView;
    if (view === 'tagEditor') {
      if (sidebarStateBeforeTagEditor === null) {
        sidebarStateBeforeTagEditor = shellStore.sidebarCollapsed;
      }
      if (!shellStore.sidebarCollapsed) {
        shellStore.setSidebarCollapsedTransient(true);
      }
    } else if (sidebarStateBeforeTagEditor !== null) {
      shellStore.setSidebarCollapsedTransient(sidebarStateBeforeTagEditor);
      sidebarStateBeforeTagEditor = null;
    }
  });

  return {
    get isMacOS() {
      return isMacOS;
    },
    get currentView() {
      return shellStore.currentView;
    },
    get albums() {
      return albums;
    },
    get selectedAlbum() {
      return selectedAlbum;
    },
    get selectedAlbumCid() {
      return selectedAlbumCid;
    },
    get loadingAlbums() {
      return loadingAlbums;
    },
    get loadingDetail() {
      return loadingDetail;
    },
    get errorMsg() {
      return errorMsg;
    },
    get librarySearchQuery() {
      return librarySearchQuery;
    },
    get librarySearchScope() {
      return librarySearchScope;
    },
    get librarySearchLoading() {
      return librarySearchLoading;
    },
    get librarySearchResponse() {
      return librarySearchResponse;
    },
    get showDetailSkeleton() {
      return showDetailSkeleton;
    },
    get albumRequestSeq() {
      return albumRequestSeq;
    },
    get selectedAlbumArtworkUrl() {
      return selectedAlbumArtworkUrl;
    },
    get currentSong() {
      return currentSong;
    },
    get isPlaying() {
      return isPlaying;
    },
    get isPaused() {
      return isPaused;
    },
    get isLoading() {
      return isLoading;
    },
    get progress() {
      return progress;
    },
    get duration() {
      return duration;
    },
    get shuffleEnabled() {
      return shuffleEnabled;
    },
    get repeatMode() {
      return repeatMode;
    },
    get playbackOrder() {
      return playbackOrder;
    },
    get lyricsOpen() {
      return lyricsOpen;
    },
    get playlistOpen() {
      return playlistOpen;
    },
    get lyricsLoading() {
      return lyricsLoading;
    },
    get lyricsError() {
      return lyricsError;
    },
    get lyricsLines() {
      return lyricsLines;
    },
    get lyricsUnavailable() {
      return lyricsUnavailable;
    },
    get activeLyricIndex() {
      return activeLyricIndex;
    },
    get fullscreenOpen() {
      return fullscreenOpen;
    },
    get playerHasPrevious() {
      return playerHasPrevious;
    },
    get playerHasNext() {
      return playerHasNext;
    },
    get downloadingAlbumCid() {
      return downloadingAlbumCid;
    },
    get activeDownloadCount() {
      return activeDownloadCount;
    },
    get filteredDownloadJobs() {
      return filteredDownloadJobs;
    },
    get hasDownloadHistory() {
      return hasDownloadHistory;
    },
    get prefersReducedMotion() {
      return prefersReducedMotion;
    },
    get overlayScrollbarOptions() {
      return overlayScrollbarOptions;
    },
    get contentScrollbarEvents() {
      return contentScrollbarEvents;
    },
    get albumStageStyle() {
      return albumStageStyle;
    },
    get albumStageMediaHeight() {
      return albumStageMediaHeight;
    },
    get albumStageScrimOpacity() {
      return albumStageScrimOpacity;
    },
    get albumStageImageOpacity() {
      return albumStageImageOpacity;
    },
    get albumStageImageTransform() {
      return albumStageImageTransform;
    },
    get albumStageSolidifyOpacity() {
      return albumStageSolidifyOpacity;
    },
    get albumStageElement() {
      return albumStageElement;
    },
    set albumStageElement(value: HTMLElement | null) {
      albumStageElement = value;
    },
    get selectionModeEnabled() {
      return selectionModeEnabled;
    },
    get selectedSongCids() {
      return selectedSongCids;
    },
    get settingsOpen() {
      return settingsOpen;
    },
    get downloadPanelOpen() {
      return downloadPanelOpen;
    },
    get SettingsSheetView() {
      return SettingsSheetView;
    },
    get DownloadTasksSheetView() {
      return DownloadTasksSheetView;
    },
    get isRefreshing() {
      return isRefreshing;
    },
    get currentSongDownloadState() {
      return currentSong
        ? downloadController.getSongDownloadState(currentSong.cid)
        : ('idle' as const);
    },
    get currentSongDownloadDisabled() {
      return currentSong
        ? downloadController.isSongDownloadInteractionBlocked(currentSong.cid)
        : false;
    },
    get sidebarCollapsed() {
      return shellStore.sidebarCollapsed;
    },
    toggleSidebar() {
      shellStore.toggleSidebar();
    },
    settingsState,
    shellStore,
    libraryController,
    playerController,
    downloadController,
    homeController,
    tagEditorController,
    collectionController,
    searchController,
    notifyInfo,
    notifyError,
    handleSelectAlbum,
    handleDeselectAlbum: libraryController.deselectAlbum,
    handleSelectSearchResult,
    handlePlay,
    handlePlayCollectionSong,
    handleRefresh,
    handleContentWheel,
    handleToggleDownloads,
    handleToggleSettings,
    handleOutputDirChange,
    handleDownloadSelection,
    handleCurrentSongDownload,
    hasAlbumDownloadJob,
    hasCurrentSelectionJob,
    toggleSelectionMode,
    selectAllSongs,
    deselectAllSongs,
    invertSongSelection,
    toggleSongSelection,
    isSongSelected,
    get canGoBack() {
      return navigationStack.canGoBack;
    },
    get isNavigating() {
      return isNavigating;
    },
    navigateToTop,
    openAlbum,
    openCollection,
    openTagEditor,
    goBack,
  };
}

export type AppRuntime = ReturnType<typeof createAppRuntime>;
