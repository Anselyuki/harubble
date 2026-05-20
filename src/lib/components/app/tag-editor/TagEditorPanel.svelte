<script lang="ts">
  import TagEditorDimension from './TagEditorDimension.svelte';
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
    merged,
    selectedEntityTags,
    onSetTag,
    onRemoveTag,
    onSelectSong,
    onAddDimension,
    onRemoveDimension,
  }: Props = $props();

  let songsExpanded = $state(false);
  let dimFormOpen = $state(false);
  let newDimKey = $state('');
  let newDimZh = $state('');
  let newDimEn = $state('');

  function getSongTagCount(song: SongEntry): number {
    if (!merged) return 0;
    const entry = merged.songs[song.cid];
    return Object.keys(entry.tags).length;
  }

  async function handleAddDimension() {
    if (!newDimKey.trim() || !newDimZh.trim()) return;
    await onAddDimension(newDimKey.trim(), newDimZh.trim(), newDimEn.trim());
    newDimKey = '';
    newDimZh = '';
    newDimEn = '';
    dimFormOpen = false;
  }
</script>

<div class="tag-editor-panel">
  <header class="panel-header">
    <div
      class="album-cover"
      use:lazyLoad={{ rootMargin: '0px', reducedMotion: false }}
      data-src={album.coverUrl}
    >
      <div class="album-cover-placeholder">♪</div>
      <img class="album-cover-img" alt={album.name} />
    </div>
    <div class="album-meta">
      <h2 class="album-name">{album.name}</h2>
      {#if album.artists.length > 0}
        <p class="album-artists">{album.artists.join(', ')}</p>
      {/if}
    </div>
  </header>

  {#if merged}
    <section class="dimensions-section">
      <div class="section-header">
        <h3 class="section-title">{m.tag_editor_album_tag()}</h3>
        <button
          type="button"
          class="dim-manage-btn"
          onclick={() => {
            dimFormOpen = !dimFormOpen;
          }}
        >
          {dimFormOpen
            ? m.tag_editor_cancel()
            : m.tag_editor_manage_dimensions()}
        </button>
      </div>

      {#if dimFormOpen}
        <div class="dim-form">
          <div class="dim-form-row">
            <input
              bind:value={newDimKey}
              placeholder={m.tag_editor_placeholder_dim_key()}
              class="dim-input"
            />
            <input
              bind:value={newDimZh}
              placeholder={m.tag_editor_placeholder_dim_zh()}
              class="dim-input"
            />
            <input
              bind:value={newDimEn}
              placeholder={m.tag_editor_placeholder_dim_en()}
              class="dim-input"
            />
            <button
              type="button"
              class="dim-add-btn"
              onclick={handleAddDimension}>{m.tag_editor_add()}</button
            >
          </div>
          {#if merged.tagDimensions.length > 0}
            <ul class="dim-list">
              {#each merged.tagDimensions as dim (dim.key)}
                <li class="dim-list-item">
                  <span>{dim.label['zh-CN'] ?? dim.key}</span>
                  <button
                    type="button"
                    class="dim-remove-btn"
                    onclick={() => onRemoveDimension(dim.key)}
                    aria-label={m.tag_editor_remove_dimension_aria({
                      key: dim.key,
                    })}>×</button
                  >
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}

      <div class="dimension-rows">
        {#each merged.tagDimensions as dim (dim.key)}
          <TagEditorDimension
            dimensionKey={dim.key}
            dimensionLabel={dim.label['zh-CN'] ?? dim.key}
            values={selectedEntityTags[dim.key] ?? []}
            {onSetTag}
            {onRemoveTag}
          />
        {/each}
      </div>
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
                      >{m.tag_editor_song_tag_count({ count: tagCount })}</span
                    >
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </section>
  {/if}
</div>

<style>
  .tag-editor-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: 1.5rem;
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

  .dimensions-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .section-title {
    font-size: 0.8125rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0;
  }

  .dim-manage-btn {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .dim-manage-btn:hover {
    background: var(--hover-bg-elevated);
    color: var(--text-primary);
  }

  .dim-form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 8px;
    background: var(--bg-secondary, rgba(255, 255, 255, 0.02));
  }

  .dim-form-row {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .dim-input {
    flex: 1;
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 4px;
    background: transparent;
    color: var(--text-primary);
  }

  .dim-add-btn {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border: none;
    border-radius: 4px;
    background: var(--accent);
    color: white;
    cursor: pointer;
  }

  .dim-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .dim-list-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    color: var(--text-primary);
  }

  .dim-remove-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 1rem;
    line-height: 1;
    padding: 0 0.25rem;
  }

  .dim-remove-btn:hover {
    color: var(--color-danger, #ef4444);
  }

  .dimension-rows {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
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
    border-radius: 6px;
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
    border-radius: 999px;
    background: rgba(var(--accent-rgb), 0.1);
    color: var(--accent);
  }
</style>
