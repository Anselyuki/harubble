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
    isMacOS?: boolean;
    onSelectAlbum: (album: Album) => void;
    onSearchChange: (query: string) => void;
    onImport: () => void;
    onExport: () => void;
  }

  let {
    albums,
    selectedAlbumCid,
    searchQuery,
    overlayScrollbarOptions,
    isMacOS = false,
    onSelectAlbum,
    onSearchChange,
    onImport,
    onExport,
  }: Props = $props();
</script>

<aside class="tag-editor-album-list" class:macos={isMacOS}>
  <div class="album-list-header">
    <div class="album-list-search">
      <input
        type="text"
        class="album-search-input"
        placeholder={m.tag_editor_search_album_placeholder()}
        value={searchQuery}
        oninput={(e) => onSearchChange(e.currentTarget.value)}
      />
    </div>

    <div class="album-list-actions">
      <button type="button" class="action-button" onclick={onImport}>
        {m.tag_editor_import()}
      </button>
      <button type="button" class="action-button" onclick={onExport}>
        {m.tag_editor_export()}
      </button>
    </div>
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
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    background: var(--surface-sidebar, rgba(0, 0, 0, 0.2));
    overflow: hidden;
  }

  .album-list-header {
    padding-top: var(--safe-area-top);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .album-list-search {
    padding: 12px;
  }

  .album-search-input {
    width: 100%;
    padding: 6px 10px;
    font-size: 12px;
    font-family: var(--font-body);
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: var(--shape-sm);
    outline: none;
  }

  .album-search-input:focus {
    border-color: rgba(var(--accent-rgb), 0.5);
  }

  .album-search-input::placeholder {
    color: var(--text-tertiary);
  }

  .album-list-actions {
    display: flex;
    gap: 8px;
    padding: 0 12px 12px 12px;
  }

  .action-button {
    flex: 1;
    padding: 6px 12px;
    font-size: 12px;
    font-family: var(--font-body);
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--shape-sm);
    cursor: pointer;
    transition: var(--motion-hover);
  }

  .action-button:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .action-button:active {
    background: rgba(255, 255, 255, 0.08);
  }

  .tag-editor-album-list :global(.album-list-scroll) {
    flex: 1;
    min-height: 0;
    overflow: hidden;
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
    border-radius: var(--shape-sm);
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
