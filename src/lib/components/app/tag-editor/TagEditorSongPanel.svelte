<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import TagOverview from './TagOverview.svelte';
  import type {
    SongEntry,
    TagEditorLocalizedValue,
    TagEditorRegistry,
  } from '$lib/types';

  interface Props {
    song: SongEntry;
    merged: TagEditorRegistry | null;
    selectedEntityTags: Record<string, TagEditorLocalizedValue[]>;
    onSetTag: (
      dimensionKey: string,
      values: TagEditorLocalizedValue[]
    ) => Promise<void>;
    onRemoveTag: (dimensionKey: string) => Promise<void>;
    onBack: () => void;
  }

  let {
    song,
    merged,
    selectedEntityTags,
    onSetTag,
    onRemoveTag,
    onBack,
  }: Props = $props();
</script>

<div class="song-panel">
  <header class="song-panel-header">
    <button
      type="button"
      class="back-btn"
      onclick={onBack}
      aria-label={m.tag_editor_back_to_album()}
    >
      {m.tag_editor_back()}
    </button>
    <h2 class="song-title">{song.name}</h2>
  </header>

  {#if merged}
    <section class="tags-section">
      <h3 class="section-title">{m.tag_editor_song_tag()}</h3>
      <TagOverview {merged} {selectedEntityTags} {onSetTag} {onRemoveTag} />
    </section>
  {/if}
</div>

<style>
  .song-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: calc(var(--safe-area-top) + 32px) 1.5rem 1.5rem;
  }

  .song-panel-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .back-btn {
    font-size: 0.8125rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
  }

  .back-btn:hover {
    background: var(--hover-bg-elevated);
    color: var(--text-primary);
  }

  .song-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
</style>
