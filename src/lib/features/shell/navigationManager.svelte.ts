import { tick } from 'svelte';
import type {
  Album,
  AlbumDetail,
  SongEntry,
  PlaybackQueueEntry,
  SearchLibraryResultItem,
} from '$lib/types';
import {
  navigationStack,
  isSameEntry,
  type NavigationEntry,
} from './navigation.svelte';
import { shellStore, type AppView } from '$lib/features/shell/store.svelte';
import { tagEditorStore } from '$lib/features/tagEditor/store.svelte';
import {
  buildAlbumPlaybackEntries,
  getSelectedAlbumCoverUrl,
} from '$lib/features/library/selectors';
import * as m from '$lib/paraglide/messages.js';
import type { createLibraryController } from '$lib/features/library/controller.svelte';
import type { createPlayerController } from '$lib/features/player/controller.svelte';
import type { createCollectionController } from '$lib/features/collection/controller.svelte';
import type { createTagEditorController } from '$lib/features/tagEditor/controller.svelte';
import type { createAlbumStageMotionController } from '$lib/features/shell/albumStageMotion.svelte';

interface NavigationManagerDeps {
  libraryController: ReturnType<typeof createLibraryController>;
  playerController: ReturnType<typeof createPlayerController>;
  collectionController: ReturnType<typeof createCollectionController>;
  tagEditorController: ReturnType<typeof createTagEditorController>;
  albumStageMotionController: ReturnType<
    typeof createAlbumStageMotionController
  >;
  clearSongSelection: () => void;
  setSelectionModeEnabled: (value: boolean) => void;
  notifyError: (message: string) => void;
  getAlbums: () => Album[];
  getSelectedAlbum: () => AlbumDetail | null;
  getShuffleEnabled: () => boolean;
  getPlaybackOrder: () => PlaybackQueueEntry[];
}

export function createNavigationManager(deps: NavigationManagerDeps) {
  let navigationSeq = 0;
  let isNavigating = $state(false);
  let isViewTransitioning = $state(false);
  let navigationDirection = $state<'forward' | 'back'>('forward');

  function resetContentScroll() {
    deps.albumStageMotionController.resetContentScroll();
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
          albumCid: deps.libraryController.selectedAlbumCid,
        };
      case 'collection':
        return {
          view: 'collection',
          collectionId: deps.collectionController.selectedCollectionId ?? '',
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
      deps.libraryController.deselectAlbum();
    }
    deps.clearSongSelection();
    deps.setSelectionModeEnabled(false);
    if (targetView !== 'collection') {
      deps.collectionController.clearSelection();
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

    navigationDirection = 'forward';
    navigationSeq++;
    navigationStack.push(current);
    shellStore.currentView = view;
    clearNonTargetState(view);
  }

  async function openAlbum(
    album: Album,
    options?: { pendingSongCid?: string | null }
  ): Promise<void> {
    const current = captureCurrentEntry();
    const target: NavigationEntry = { view: 'library', albumCid: album.cid };
    if (isSameEntry(current, target)) return;

    const seq = ++navigationSeq;
    navigationDirection = 'forward';
    isNavigating = true;
    navigationStack.push(current);
    shellStore.currentView = 'library';
    clearNonTargetState('library');

    const shouldDispose = () => seq !== navigationSeq;

    if (options?.pendingSongCid !== undefined) {
      deps.libraryController.setPendingScrollToSong(options.pendingSongCid);
    }

    try {
      await deps.libraryController.selectAlbum(album, {
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
    navigationDirection = 'forward';
    isNavigating = true;
    navigationStack.push(current);
    shellStore.currentView = 'collection';
    clearNonTargetState('collection');

    const shouldDispose = () => seq !== navigationSeq;

    try {
      await deps.collectionController.loadAndSelect(collectionId);
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
    navigationDirection = 'forward';
    isNavigating = true;
    navigationStack.push(current);
    shellStore.currentView = 'tagEditor';
    clearNonTargetState('tagEditor');

    const shouldDispose = () => seq !== navigationSeq;

    try {
      await deps.tagEditorController.selectAlbumForEditAsync(
        album,
        shouldDispose
      );
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
    navigationDirection = 'back';
    isNavigating = true;
    shellStore.currentView = entry.view;
    clearNonTargetState(entry.view);

    const shouldDispose = () => seq !== navigationSeq;

    try {
      switch (entry.view) {
        case 'library': {
          if (entry.albumCid) {
            const album = deps
              .getAlbums()
              .find((a) => a.cid === entry.albumCid);
            if (album) {
              await deps.libraryController.selectAlbum(album, {
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
            await deps.collectionController.restoreSelection(
              entry.collectionId
            );
          }
          break;
        }
        case 'tagEditor': {
          await deps.tagEditorController.restoreEditingState(
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
    const album = deps
      .getAlbums()
      .find((candidate) => candidate.cid === item.albumCid);
    if (!album) {
      deps.notifyError(m.app_error_album_not_found());
      return;
    }
    await openAlbum(album, {
      pendingSongCid: item.kind === 'song' ? item.songCid : null,
    });
  }

  async function handlePlay(song: SongEntry) {
    const selectedAlbum = deps.getSelectedAlbum();
    const sourceEntries = buildAlbumPlaybackEntries(selectedAlbum);
    const fallbackEntry: PlaybackQueueEntry = {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: getSelectedAlbumCoverUrl(selectedAlbum),
    };
    const entries = sourceEntries.length ? sourceEntries : [fallbackEntry];
    deps.playerController.applyPlaybackQueue(entries, song.cid);
    const shuffleEnabled = deps.getShuffleEnabled();
    const playbackOrder = deps.getPlaybackOrder();
    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;
    await deps.playerController.playQueueEntry(
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
    deps.playerController.applyPlaybackQueue(entries, song.cid);
    const shuffleEnabled = deps.getShuffleEnabled();
    const playbackOrder = deps.getPlaybackOrder();
    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;
    await deps.playerController.playQueueEntry(
      nextOrder[nextIndex],
      nextOrder,
      nextIndex
    );
  }

  return {
    get canGoBack() {
      return navigationStack.canGoBack;
    },
    get isNavigating() {
      return isNavigating;
    },
    get isViewTransitioning() {
      return isViewTransitioning;
    },
    get navigationDirection() {
      return navigationDirection;
    },
    handleTransitionStart() {
      isViewTransitioning = true;
    },
    handleTransitionEnd() {
      isViewTransitioning = false;
    },
    resetContentScroll,
    navigateToTop,
    openAlbum,
    openCollection,
    openTagEditor,
    goBack,
    handleSelectAlbum,
    handleSelectSearchResult,
    handlePlay,
    handlePlayCollectionSong,
  };
}

export type NavigationManager = ReturnType<typeof createNavigationManager>;
