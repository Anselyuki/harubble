<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import SongRow from '$lib/components/SongRow.svelte';
  import MetadataPopover from '$lib/components/MetadataPopover.svelte';
  import {
    getDownloadBadgeLabel,
    shouldShowDownloadBadge,
  } from '$lib/downloadBadge';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type { AlbumDetail, CollectionSummary, SongEntry } from '$lib/types';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    album: AlbumDetail;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    isPlaybackPaused: boolean;
    downloadingAlbumCid: string | null;
    selectionModeEnabled: boolean;
    selectedSongCids: string[];
    reducedMotion: boolean;
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

  let props: Props = $props();

  function dur(base: number): number {
    return props.reducedMotion ? 0 : base;
  }

  const selectedSongCount = $derived.by(() => props.selectedSongCids.length);
  const selectedSongsLabel = $derived.by(() => {
    void localeState.current;
    if (selectedSongCount === 0) return m.library_selection_none();
    return m.library_selection_count({ count: selectedSongCount });
  });
  const songCountLabel = $derived.by(() => {
    void localeState.current;
    return m.library_song_count({ count: props.album.songs.length });
  });
  const isAlbumDownloadCreating = $derived.by(
    () => props.downloadingAlbumCid === props.album.cid
  );
  const hasAlbumDownloadJob = $derived.by(() =>
    props.hasAlbumDownloadJob(props.album.cid)
  );
  const isAlbumDownloadDisabled = $derived.by(
    () => isAlbumDownloadCreating || hasAlbumDownloadJob
  );
  const isAllSongsSelected = $derived.by(
    () => selectedSongCount === props.album.songs.length
  );
  const canInvertSelection = $derived.by(() => props.album.songs.length > 0);
  const isSelectionCreating = $derived.by(() =>
    props.isCurrentSelectionCreating(props.selectedSongCids)
  );
  const hasCurrentSelectionJob = $derived.by(() =>
    props.hasCurrentSelectionJob(props.selectedSongCids)
  );
  const isSelectionDownloadDisabled = $derived.by(() =>
    props.isSelectionDownloadDisabled(props.selectedSongCids)
  );

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      downloadAlbum: m.library_download_album(),
      downloadCreating: m.library_download_creating(),
      downloadQueued: m.library_download_queued(),
      selectionToggleOn: m.library_selection_toggle_on(),
      selectionToggleOff: m.library_selection_toggle_off(),
      selectAll: m.library_selection_select_all(),
      deselectAll: m.library_selection_deselect_all(),
      invert: m.library_selection_invert(),
      selectionDownload: m.library_selection_download(),
      selectionCreatingBatch: m.library_selection_creating_batch(),
    };
  });
</script>

<div
  class="album-detail-card"
  class:is-reduced-motion={props.reducedMotion}
  in:fade={{ duration: dur(220) }}
  out:fade={{ duration: dur(220) }}
>
  <div class="album-hero">
    <div
      class="album-hero-info"
      in:fly={{ y: 14, duration: dur(220), delay: dur(30) }}
      out:fly={{ y: 8, duration: dur(220) }}
    >
      <div class="album-tags-row">
        {#if props.album.belong && props.album.belong.toLowerCase() !== 'arknights'}
          <span class="album-belong-tag"
            >{props.album.belong.toUpperCase()}</span
          >
        {/if}
        {#each props.album.tags as tag (tag.dimension)}
          {#each tag.values as value, i (value)}
            <span
              class="album-belong-tag"
              style:color={tag.colors?.[i] ?? undefined}
              style:background={tag.colors?.[i]
                ? `${tag.colors[i]}1a`
                : undefined}>{value}</span
            >
          {/each}
        {/each}
      </div>
      <div class="album-title-row">
        <h1 class="album-hero-title">{props.album.name}</h1>
        <MetadataPopover target={{ kind: 'album', album: props.album }} />
      </div>
      {#if props.album.artists && props.album.artists.length > 0}
        <p class="album-hero-artists">{props.album.artists.join(', ')}</p>
      {/if}
      {#if props.album.intro}
        <p class="album-hero-intro">{props.album.intro}</p>
      {/if}
      <div class="album-hero-meta">
        <span class="album-song-count">{songCountLabel}</span>
        {#if shouldShowDownloadBadge(props.album.download.downloadStatus)}
          <span class="album-download-status-badge">
            {getDownloadBadgeLabel(props.album.download.downloadStatus)}
          </span>
        {/if}
      </div>
      <div class="controls album-hero-actions">
        <button
          type="button"
          class="btn btn-primary"
          class:is-disabled={isAlbumDownloadDisabled}
          onclick={() => props.onDownloadAlbum(props.album.cid)}
          disabled={isAlbumDownloadDisabled}
        >
          {#if isAlbumDownloadCreating}
            {labels.downloadCreating}
          {:else if hasAlbumDownloadJob}
            {labels.downloadQueued}
          {:else}
            {labels.downloadAlbum}
          {/if}
        </button>
        <button type="button" class="btn" onclick={props.onToggleSelectionMode}>
          {props.selectionModeEnabled
            ? labels.selectionToggleOn
            : labels.selectionToggleOff}
        </button>
        {#if props.selectionModeEnabled}
          <button
            type="button"
            class="btn"
            class:is-disabled={isAllSongsSelected}
            onclick={props.onSelectAllSongs}
            disabled={isAllSongsSelected}
          >
            {labels.selectAll}
          </button>
          <button
            type="button"
            class="btn"
            class:is-disabled={selectedSongCount === 0}
            onclick={props.onDeselectAllSongs}
            disabled={selectedSongCount === 0}
          >
            {labels.deselectAll}
          </button>
          <button
            type="button"
            class="btn"
            class:is-disabled={!canInvertSelection}
            onclick={props.onInvertSongSelection}
            disabled={!canInvertSelection}
          >
            {labels.invert}
          </button>
          <button
            type="button"
            class="btn btn-primary"
            class:is-disabled={isSelectionDownloadDisabled}
            onclick={() => props.onDownloadSelection(props.selectedSongCids)}
            disabled={isSelectionDownloadDisabled}
          >
            {#if isSelectionCreating}
              {labels.selectionCreatingBatch}
            {:else if hasCurrentSelectionJob}
              {labels.downloadQueued}
            {:else}
              {labels.selectionDownload}
            {/if}
          </button>
          <span class="album-selection-summary">{selectedSongsLabel}</span>
        {/if}
      </div>
    </div>
  </div>
  <div
    class="song-list"
    in:fly={{ y: 10, duration: dur(200), delay: dur(70) }}
    out:fade={{ duration: dur(200) }}
  >
    {#each props.album.songs as song, index (song.cid)}
      <SongRow
        {song}
        {index}
        albumCid={props.album.cid}
        albumName={props.album.name}
        albumTags={props.album.tags}
        isPlaying={props.currentSongCid === song.cid && props.isPlaybackActive}
        isPaused={props.currentSongCid === song.cid && props.isPlaybackPaused}
        downloadState={props.getSongDownloadState(song.cid)}
        downloadDisabled={props.isSongDownloadInteractionBlocked(song.cid)}
        selectionMode={props.selectionModeEnabled}
        isSelected={props.isSongSelected(song.cid)}
        selectionDisabled={isSelectionCreating}
        reducedMotion={props.reducedMotion}
        collections={props.collections}
        onAddToCollection={props.onAddToCollection}
        onclick={() => props.onPlaySong(song)}
        onTogglePlay={() => props.onTogglePlay()}
        onDownload={() => props.onDownloadSong(song.cid)}
        onToggleSelection={() => props.onToggleSongSelection(song.cid)}
      />
    {/each}
  </div>
</div>

<style>
  .album-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn {
    transition:
      background-color 0.16s ease-out,
      color 0.16s ease-out,
      box-shadow 0.16s ease-out,
      opacity 0.16s ease-out;
  }

  .btn:hover:not(:disabled):not(.is-reduced-motion *) {
    transform: translateY(-1px);
  }

  .btn:active:not(:disabled):not(.is-reduced-motion *) {
    transform: translateY(0) scale(0.98);
    opacity: 0.94;
  }

  .btn:not(.btn-primary):hover:not(:disabled) {
    background: var(--hover-bg-elevated);
    box-shadow: 0 8px 20px rgba(15, 23, 42, 0.08);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
    box-shadow: 0 10px 24px rgba(var(--accent-rgb), 0.2);
  }

  .btn.is-disabled {
    opacity: 0.42;
  }

  .btn-primary.is-disabled {
    border-color: rgba(15, 23, 42, 0.12);
    background: rgba(15, 23, 42, 0.08);
    color: var(--text-secondary);
    box-shadow: none;
    opacity: 1;
  }

  .is-reduced-motion .btn {
    transition: none;
  }
</style>
