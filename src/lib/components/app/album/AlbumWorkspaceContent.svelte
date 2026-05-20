<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { animateIn, killTweens } from '$lib/design/gsap';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { EventListeners, PartialOptions } from 'overlayscrollbars';
  import type { AlbumDetail, CollectionSummary, SongEntry } from '$lib/types';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import AlbumStage from '$lib/components/app/album/AlbumStage.svelte';
  import AlbumDetailSkeleton from '$lib/components/app/album/AlbumDetailSkeleton.svelte';
  import AlbumDetailPanel from '$lib/components/app/album/AlbumDetailPanel.svelte';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    loadingDetail: boolean;
    showDetailSkeleton: boolean;
    albumRequestSeq: number;
    selectedAlbum: AlbumDetail | null;
    selectedAlbumArtworkUrl: string | null;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    isPlaybackPaused: boolean;
    downloadingAlbumCid: string | null;
    selectionModeEnabled: boolean;
    selectedSongCids: string[];
    reducedMotion: boolean;
    overlayScrollbarOptions: PartialOptions;
    contentScrollbarEvents: EventListeners;
    onContentWheel: (event: WheelEvent) => void;
    albumStageStyle: string;
    albumStageMediaHeight: string;
    albumStageScrimOpacity: number;
    albumStageImageOpacity: number;
    albumStageImageTransform: string;
    albumStageSolidifyOpacity: number;
    albumStageElement?: HTMLElement | null;
    onToggleSelectionMode: () => void;
    onSelectAllSongs: () => void;
    onDeselectAllSongs: () => void;
    onInvertSongSelection: () => void;
    onDownloadAlbum: (albumCid: string) => void | Promise<void>;
    onDownloadSelection: (songCids: string[]) => void | Promise<void>;
    onPlaySong: (song: SongEntry) => void | Promise<void>;
    onTogglePlay: () => void | Promise<void>;
    onDownloadSong: (songCid: string) => void | Promise<void>;
    onToggleSongSelection: (songCid: string) => void;
    isSongSelected: (songCid: string) => boolean;
    getSongDownloadState: (songCid: string) => SongDownloadState;
    isSongDownloadInteractionBlocked: (songCid: string) => boolean;
    hasAlbumDownloadJob: (albumCid: string) => boolean;
    isSelectionDownloadDisabled: (songCids: string[]) => boolean;
    isCurrentSelectionCreating: (songCids: string[]) => boolean;
    hasCurrentSelectionJob: (songCids: string[]) => boolean;
    collections?: CollectionSummary[];
    onAddToCollection?: (collectionId: string, songCid: string) => void;
  }

  let {
    loadingDetail,
    showDetailSkeleton,
    albumRequestSeq: _albumRequestSeq,
    selectedAlbum,
    selectedAlbumArtworkUrl,
    currentSongCid,
    isPlaybackActive,
    isPlaybackPaused,
    downloadingAlbumCid,
    selectionModeEnabled,
    selectedSongCids,
    reducedMotion,
    overlayScrollbarOptions,
    contentScrollbarEvents,
    onContentWheel,
    albumStageStyle,
    albumStageMediaHeight,
    albumStageScrimOpacity,
    albumStageImageOpacity,
    albumStageImageTransform,
    albumStageSolidifyOpacity,
    albumStageElement = $bindable<HTMLElement | null>(null),
    onToggleSelectionMode,
    onSelectAllSongs,
    onDeselectAllSongs,
    onInvertSongSelection,
    onDownloadAlbum,
    onDownloadSelection,
    onPlaySong,
    onTogglePlay,
    onDownloadSong,
    onToggleSongSelection,
    isSongSelected,
    getSongDownloadState,
    isSongDownloadInteractionBlocked,
    hasAlbumDownloadJob,
    isSelectionDownloadDisabled,
    isCurrentSelectionCreating,
    hasCurrentSelectionJob,
    collections,
    onAddToCollection,
  }: Props = $props();

  let skeletonEl = $state<HTMLElement | undefined>();
  let albumPanelEl = $state<HTMLElement | undefined>();
  let loadingMaskEl = $state<HTMLElement | undefined>();

  $effect(() => {
    if (!skeletonEl) return;
    animateIn(skeletonEl, { opacity: 0 }, { opacity: 1 }, 180, 'ios-out');
    return () => killTweens(skeletonEl!);
  });

  $effect(() => {
    if (!albumPanelEl) return;
    animateIn(albumPanelEl, { opacity: 0 }, { opacity: 1 }, 180, 'ios-out');
    return () => killTweens(albumPanelEl!);
  });

  $effect(() => {
    if (!loadingMaskEl) return;
    animateIn(loadingMaskEl, { opacity: 0 }, { opacity: 1 }, 140, 'ios-out');
    return () => killTweens(loadingMaskEl!);
  });

  const emptyLabels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.library_workspace_select_album(),
      hint: m.library_workspace_select_album_hint(),
    };
  });
</script>

<OverlayScrollbarsComponent
  element="div"
  class="h-full"
  data-overlayscrollbars-initialize
  options={overlayScrollbarOptions}
  events={contentScrollbarEvents}
  defer
  onwheel={onContentWheel}
  aria-busy={loadingDetail}
>
  {#if loadingDetail && showDetailSkeleton}
    <section class="album-panel album-panel-loading" bind:this={skeletonEl}>
      <AlbumStage
        loading={true}
        {reducedMotion}
        stageStyle={albumStageStyle}
        mediaHeight={albumStageMediaHeight}
        scrimOpacity={albumStageScrimOpacity}
        bind:element={albumStageElement}
      />
      <AlbumDetailSkeleton {reducedMotion} />
    </section>
  {:else if selectedAlbum}
    {#key selectedAlbum.cid}
      <section class="album-panel" bind:this={albumPanelEl}>
        <AlbumStage
          albumName={selectedAlbum.name}
          artworkUrl={selectedAlbumArtworkUrl}
          {reducedMotion}
          stageStyle={albumStageStyle}
          mediaHeight={albumStageMediaHeight}
          scrimOpacity={albumStageScrimOpacity}
          imageOpacity={albumStageImageOpacity}
          imageTransform={albumStageImageTransform}
          solidifyOpacity={albumStageSolidifyOpacity}
          bind:element={albumStageElement}
        />
        <AlbumDetailPanel
          album={selectedAlbum}
          {currentSongCid}
          {isPlaybackActive}
          {isPlaybackPaused}
          {downloadingAlbumCid}
          {selectionModeEnabled}
          {selectedSongCids}
          {reducedMotion}
          {onToggleSelectionMode}
          {onSelectAllSongs}
          {onDeselectAllSongs}
          {onInvertSongSelection}
          {onDownloadAlbum}
          {onDownloadSelection}
          {onPlaySong}
          {onTogglePlay}
          {onDownloadSong}
          {onToggleSongSelection}
          {isSongSelected}
          {getSongDownloadState}
          {isSongDownloadInteractionBlocked}
          {hasAlbumDownloadJob}
          {isSelectionDownloadDisabled}
          {isCurrentSelectionCreating}
          {hasCurrentSelectionJob}
          {collections}
          {onAddToCollection}
        />
      </section>
    {/key}
  {/if}

  {#if !loadingDetail && !selectedAlbum}
    <h1 class="page-title">{emptyLabels.title}</h1>
    <p class="page-subtitle">{emptyLabels.hint}</p>
  {/if}

  {#if loadingDetail && selectedAlbum}
    <div
      class="content-loading-mask"
      aria-hidden="true"
      bind:this={loadingMaskEl}
    >
      <MotionSpinner className="content-loading-mask-spinner" {reducedMotion} />
    </div>
  {/if}
</OverlayScrollbarsComponent>
