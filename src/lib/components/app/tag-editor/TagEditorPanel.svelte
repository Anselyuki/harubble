<script lang="ts">
  import TagOverview from './TagOverview.svelte';
  import DimensionManageDialog from './DimensionManageDialog.svelte';
  import { lazyLoad } from '$lib/lazyLoad';
  import * as m from '$lib/paraglide/messages.js';
  import type {
    Album,
    SongEntry,
    TagEditorLocalizedValue,
    TagEditorRegistry,
  } from '$lib/types';

  interface Props {
    album: Album;
    songs: SongEntry[];
    loadingSongs: boolean;
    reducedMotion?: boolean;
    merged: TagEditorRegistry | null;
    selectedEntityTags: Record<string, TagEditorLocalizedValue[]>;
    onSetTag: (
      dimensionKey: string,
      values: TagEditorLocalizedValue[]
    ) => Promise<void>;
    onRemoveTag: (dimensionKey: string) => Promise<void>;
    onSelectSong: (song: SongEntry) => void;
    onAddDimension: (
      key: string,
      labelZh: string,
      labelEn: string
    ) => Promise<void>;
    onRemoveDimension: (key: string) => Promise<void>;
  }

  let {
    album,
    songs,
    loadingSongs,
    reducedMotion = false,
    merged,
    selectedEntityTags,
    onSetTag,
    onRemoveTag,
    onSelectSong,
    onAddDimension,
    onRemoveDimension,
  }: Props = $props();

  let songsExpanded = $state(false);
  let dimDialogOpen = $state(false);

  function getSongTagCount(song: SongEntry): number {
    if (!merged || !(song.cid in merged.songs)) return 0;
    return Object.keys(merged.songs[song.cid].tags).length;
  }

  function getTaggedSongCount(): number {
    if (!merged) return 0;
    return songs.filter(
      (s) =>
        s.cid in merged!.songs &&
        Object.keys(merged!.songs[s.cid].tags).length > 0
    ).length;
  }
</script>

<div class="tag-editor-panel">
  <header class="panel-header">
    {#key album.cid}
      <div
        class="album-cover"
        use:lazyLoad={{ rootMargin: '0px', reducedMotion }}
        data-src={album.coverUrl}
      >
        <div class="album-cover-placeholder">♪</div>
        <img class="album-cover-img" alt={album.name} />
      </div>
    {/key}
    <div class="album-meta">
      <h2 class="album-name">{album.name}</h2>
      {#if album.artists.length > 0}
        <p class="album-artists">{album.artists.join(', ')}</p>
      {/if}
    </div>
    {#if merged}
      <button
        type="button"
        class="settings-btn"
        onclick={() => {
          dimDialogOpen = true;
        }}
        aria-label={m.tag_editor_dimension_manage()}
        title={m.tag_editor_dimension_manage()}>⚙</button
      >
    {/if}
  </header>

  {#if merged}
    <section class="tags-section">
      <h3 class="section-title">{m.tag_editor_album_tag()}</h3>
      <TagOverview {merged} {selectedEntityTags} {onSetTag} {onRemoveTag} />
    </section>

    <section class="songs-section">
      <button
        type="button"
        class="songs-toggle"
        onclick={() => {
          songsExpanded = !songsExpanded;
        }}
      >
        <span class="songs-toggle-icon">{songsExpanded ? '▼' : '▶'}</span>
        <span>{m.tag_editor_songs_list()}</span>
        <span class="songs-count">({songs.length})</span>
        <span class="songs-tagged"
          >{m.tag_editor_songs_tagged_count({
            count: getTaggedSongCount(),
            total: songs.length,
          })}</span
        >
      </button>

      {#if songsExpanded}
        {#if loadingSongs}
          <p class="songs-loading">{m.tag_editor_songs_loading()}</p>
        {:else if songs.length === 0}
          <p class="songs-empty">{m.tag_editor_songs_empty()}</p>
        {:else}
          <ul class="songs-list">
            {#each songs as song (song.cid)}
              {@const tagCount = getSongTagCount(song)}
              <li class="song-item">
                <button
                  type="button"
                  class="song-btn"
                  onclick={() => onSelectSong(song)}
                >
                  <span class="song-name">{song.name}</span>
                  {#if tagCount > 0}
                    <span class="song-tag-badge"
                      >{m.tag_editor_song_tag_count({
                        count: tagCount,
                      })}</span
                    >
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </section>

    <DimensionManageDialog
      open={dimDialogOpen}
      dimensions={merged.tagDimensions}
      {onAddDimension}
      {onRemoveDimension}
      onOpenChange={(v) => {
        dimDialogOpen = v;
      }}
    />
  {/if}
</div>

<style>
  .tag-editor-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: calc(var(--safe-area-top) + 32px) 1.5rem 1.5rem;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .album-cover {
    width: 64px;
    height: 64px;
    border-radius: 10px;
    background: linear-gradient(
      135deg,
      var(--bg-tertiary) 0%,
      var(--bg-secondary) 100%
    );
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
  }

  .album-cover-placeholder {
    color: var(--text-tertiary);
    font-size: 24px;
  }

  .album-cover-img {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 10px;
    opacity: 0;
  }

  .album-meta {
    flex: 1;
    min-width: 0;
  }

  .album-name {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album-artists {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin: 0.25rem 0 0;
  }

  .settings-btn {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: var(--shape-sm);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.875rem;
  }

  .settings-btn:hover {
    background: var(--hover-bg-elevated);
    color: var(--text-primary);
  }

  .tags-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    font-size: 0.8125rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0;
  }

  .songs-section {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--color-border, #e5e7eb);
    padding-top: 1rem;
  }

  .songs-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
    padding: 0.25rem 0;
  }

  .songs-toggle-icon {
    font-size: 0.625rem;
    color: var(--text-secondary);
  }

  .songs-count {
    font-weight: 400;
    color: var(--text-secondary);
  }

  .songs-tagged {
    font-weight: 400;
    font-size: 0.6875rem;
    color: var(--text-tertiary);
    margin-left: auto;
  }

  .songs-loading,
  .songs-empty {
    font-size: 0.75rem;
    color: var(--text-secondary);
    padding: 0.5rem 0;
    margin: 0;
  }

  .songs-list {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0 0;
  }

  .song-item {
    border-bottom: 1px solid var(--color-border, #f3f4f6);
  }

  .song-item:last-child {
    border-bottom: none;
  }

  .song-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.5rem 0.5rem;
    background: none;
    border: none;
    border-radius: var(--shape-sm);
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-size: 0.8125rem;
  }

  .song-btn:hover {
    background: var(--hover-bg-elevated);
  }

  .song-name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .song-tag-badge {
    flex-shrink: 0;
    font-size: 0.6875rem;
    padding: 0.125rem 0.375rem;
    border-radius: var(--shape-pill);
    background: rgba(var(--accent-rgb), 0.1);
    color: var(--accent);
  }
</style>
