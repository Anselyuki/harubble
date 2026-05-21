<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { PartialOptions } from 'overlayscrollbars';
  import type { Album } from '$lib/types';

  interface Props {
    albums: Album[];
    selectedAlbumCid: string | null;
    searchQuery: string;
    overlayScrollbarOptions: PartialOptions;
    onSelectAlbum: (album: Album) => void;
    onSearchChange: (query: string) => void;
  }

  let {
    albums,
    selectedAlbumCid,
    searchQuery,
    overlayScrollbarOptions,
    onSelectAlbum,
    onSearchChange,
  }: Props = $props();
</script>

<aside class="tag-editor-album-list">
  <div class="album-list-search">
    <input
      type="text"
      class="album-search-input"
      placeholder={m.tag_editor_search_album_placeholder()}
      value={searchQuery}
      oninput={(e) => onSearchChange(e.currentTarget.value)}
    />
  </div>

  <OverlayScrollbarsComponent
    class="album-list-scroll"
    options={overlayScrollbarOptions}
    defer
  >
    <ul class="album-list-items" aria-label={m.tag_editor_album_list_label()}>
      {#each albums as album (album.cid)}
        <li>
          <button
            type="button"
            class="album-list-item"
            class:active={selectedAlbumCid === album.cid}
            aria-current={selectedAlbumCid === album.cid ? 'true' : undefined}
            onclick={() => onSelectAlbum(album)}
          >
            <span class="album-item-name">{album.name}</span>
            {#if album.artists.length > 0}
              <span class="album-item-artists">{album.artists.join(', ')}</span>
            {/if}
          </button>
        </li>
      {/each}

      {#if albums.length === 0}
        <li class="album-list-empty">
          {searchQuery.trim()
            ? m.tag_editor_album_search_empty()
            : m.tag_editor_album_list_empty()}
        </li>
      {/if}
    </ul>
  </OverlayScrollbarsComponent>
</aside>

<style>
  .tag-editor-album-list {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    background: var(--surface-sidebar, rgba(0, 0, 0, 0.2));
  }

  .album-list-search {
    padding: 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .album-search-input {
    width: 100%;
    padding: 6px 10px;
    font-size: 12px;
    font-family: var(--font-body);
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    outline: none;
  }

  .album-search-input:focus {
    border-color: rgba(var(--accent-rgb), 0.5);
  }

  .album-search-input::placeholder {
    color: var(--text-tertiary);
  }

  .tag-editor-album-list :global(.album-list-scroll) {
    flex: 1;
  }

  .album-list-items {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    margin: 0;
    list-style: none;
  }

  .album-list-items li {
    display: contents;
  }

  .album-list-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    text-align: left;
    cursor: pointer;
    font-family: var(--font-body);
  }

  .album-list-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .album-list-item.active {
    background: rgba(var(--accent-rgb), 0.12);
  }

  .album-item-name {
    font-size: 13px;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album-list-item.active .album-item-name {
    color: var(--accent);
  }

  .album-item-artists {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album-list-empty {
    padding: 24px 12px;
    text-align: center;
    font-size: 12px;
    color: var(--text-tertiary);
  }
</style>
