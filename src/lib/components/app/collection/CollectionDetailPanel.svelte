<script lang="ts">
  import { SvelteMap } from 'svelte/reactivity';
  import { animateIn, animateOut, killTweens, MOTION } from '$lib/design/gsap';
  import { runLayeredIn } from '$lib/design/view-transition';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { PartialOptions } from 'overlayscrollbars';
  import SongRow from '$lib/components/SongRow.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type {
    Collection,
    CollectionSummary,
    SongEntry,
    PlaybackQueueEntry,
  } from '$lib/types';
  import type { ResolvedSong } from '$lib/features/collection/resolvedSongs.svelte';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    collection: Collection | null;
    isLoading: boolean;
    reducedMotion: boolean;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    isPlaybackPaused: boolean;
    resolvedSongs: ResolvedSong[];
    isResolvingSongs: boolean;
    playbackQueue: PlaybackQueueEntry[];
    onEdit: () => void;
    onDelete: (id: string) => void;
    onExport: (id: string) => void;
    onRemoveSongs: (collectionId: string, songIds: string[]) => void;
    onReorderSongs: (collectionId: string, songIds: string[]) => void;
    onPlaySong: (song: SongEntry, queue: PlaybackQueueEntry[]) => void;
    onTogglePlay: () => void;
    onDownloadSong: (songCid: string) => void | Promise<void>;
    getSongDownloadState: (songCid: string) => SongDownloadState;
    isSongDownloadInteractionBlocked: (songCid: string) => boolean;
    collections?: CollectionSummary[];
    onAddToCollection?: (collectionId: string, songCid: string) => void;
  }

  let props: Props = $props();

  const scrollbarOptions = $derived.by(
    (): PartialOptions => ({
      scrollbars: {
        theme: 'os-theme-app',
        autoHide: props.reducedMotion ? 'leave' : 'move',
        autoHideDelay: props.reducedMotion ? 160 : 720,
        autoHideSuspend: true,
        dragScroll: true,
        clickScroll: false,
      },
    })
  );

  let dragSourceIndex = $state<number | null>(null);

  const isEditable = $derived.by(() => !props.collection?.isOfficial);

  const allSongIds = $derived.by((): string[] => {
    const sections = props.collection?.sections;
    if (!sections) return [];
    return sections.flatMap((s) => s.songIds);
  });

  const songCountLabel = $derived.by(() => {
    void localeState.current;
    return m.collection_song_count({ count: allSongIds.length });
  });

  const sectionStartMap = $derived.by((): SvelteMap<string, string> => {
    const sections = props.collection?.sections;
    if (!sections || sections.length === 0) return new SvelteMap();
    const map = new SvelteMap<string, string>();
    for (const s of sections) {
      if (s.name && s.songIds.length > 0) {
        map.set(s.songIds[0], s.name);
      }
    }
    return map;
  });

  let loadingEl = $state<HTMLElement | undefined>();
  let cardEl = $state<HTMLElement | undefined>();
  let heroInfoEl = $state<HTMLElement | undefined>();
  let songListEl = $state<HTMLElement | undefined>();
  let cardMounted = $state(false);

  const hasCollection = $derived(!!props.collection && !props.isLoading);

  $effect(() => {
    if (hasCollection) cardMounted = true;
  });

  $effect(() => {
    if (!loadingEl) return;
    animateIn(
      loadingEl,
      { opacity: 0 },
      { opacity: 1 },
      MOTION.SLOW,
      'ios-out'
    );
    return () => killTweens(loadingEl!);
  });

  // 合集详情分层进入：封面卡片 → Hero 信息 → 歌曲列表，统一节奏与曲线。
  $effect(() => {
    if (!cardEl || !hasCollection) return;
    const tl = runLayeredIn([
      { target: cardEl },
      { target: heroInfoEl, fromY: 14 },
      { target: songListEl, fromY: 10 },
    ]);
    return () => {
      tl.kill();
      killTweens(cardEl!);
      if (heroInfoEl) killTweens(heroInfoEl);
      if (songListEl) killTweens(songListEl);
    };
  });

  $effect(() => {
    if (hasCollection || !cardMounted || !cardEl) return;
    animateOut(cardEl, { opacity: 0 }, MOTION.SLOW_OUT, {
      onComplete: () => {
        cardMounted = false;
      },
    });
  });

  function handleDragStart(event: DragEvent, index: number) {
    dragSourceIndex = index;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/plain', String(index));
    }
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
  }

  function handleDrop(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    if (
      dragSourceIndex === null ||
      dragSourceIndex === targetIndex ||
      !props.collection
    ) {
      dragSourceIndex = null;
      return;
    }

    const newOrder = [...allSongIds];
    const [moved] = newOrder.splice(dragSourceIndex, 1);
    newOrder.splice(targetIndex, 0, moved);

    dragSourceIndex = null;
    props.onReorderSongs(props.collection.id, newOrder);
  }
</script>

<OverlayScrollbarsComponent
  class="collection-scroll-container"
  options={scrollbarOptions}
  defer
>
  {#if props.isLoading}
    <div class="collection-detail-loading" bind:this={loadingEl}>
      <span>{m.collection_detail_loading()}</span>
    </div>
  {:else if !props.collection}
    <div class="collection-detail-loading" bind:this={loadingEl}>
      <span>{m.collection_detail_select_hint()}</span>
    </div>
  {:else}
    {@const collection = props.collection}
    <div
      class="collection-detail-card"
      class:is-reduced-motion={props.reducedMotion}
      bind:this={cardEl}
    >
      <div class="collection-hero">
        <div class="collection-hero-info" bind:this={heroInfoEl}>
          {#if collection.isOfficial}
            <span class="collection-official-tag"
              >{m.collection_official_badge()}</span
            >
          {/if}
          <h1 class="collection-hero-title">{collection.name}</h1>
          {#if collection.description}
            <p class="collection-hero-description">
              {collection.description}
            </p>
          {/if}
          <div class="collection-hero-meta">
            <span class="collection-song-count">{songCountLabel}</span>
            <button
              type="button"
              class="btn btn-meta"
              onclick={() => props.onExport(collection.id)}
            >
              {m.collection_action_export()}
            </button>
          </div>
          {#if isEditable}
            <div class="controls collection-hero-actions">
              <button type="button" class="btn" onclick={props.onEdit}>
                {m.collection_action_edit()}
              </button>
              <button
                type="button"
                class="btn btn-danger"
                onclick={() => {
                  if (
                    confirm(
                      m.collection_delete_confirm({ name: collection.name })
                    )
                  ) {
                    props.onDelete(collection.id);
                  }
                }}
              >
                {m.collection_action_delete()}
              </button>
            </div>
          {/if}
        </div>
      </div>

      <div class="collection-divider"></div>

      <div class="song-list" bind:this={songListEl}>
        {#if props.isResolvingSongs && props.resolvedSongs.length === 0}
          <div class="song-list-loading">{m.collection_songs_loading()}</div>
        {:else if props.resolvedSongs.length > 0}
          {#each props.resolvedSongs as rs, index (rs.entry.cid)}
            {#if sectionStartMap.get(rs.entry.cid)}
              <div
                class="section-header"
                class:section-header-first={index === 0}
              >
                <span class="section-header-title"
                  >{sectionStartMap.get(rs.entry.cid)}</span
                >
              </div>
            {/if}
            <div
              class="collection-song-wrapper"
              class:is-drag-over={dragSourceIndex !== null &&
                dragSourceIndex !== index}
              draggable={isEditable ? 'true' : undefined}
              role="listitem"
              ondragstart={(e) => {
                if (isEditable) handleDragStart(e, index);
              }}
              ondragover={(e) => handleDragOver(e)}
              ondrop={(e) => handleDrop(e, index)}
              ondragend={() => {
                dragSourceIndex = null;
              }}
            >
              {#if isEditable}
                <button
                  type="button"
                  class="drag-handle"
                  aria-hidden="true"
                  tabindex={-1}>⠿</button
                >
              {/if}
              <div class="collection-song-row-content">
                <SongRow
                  song={rs.entry}
                  {index}
                  albumCid={rs.albumCid}
                  albumName={rs.albumName}
                  coverUrl={rs.coverUrl}
                  isPlaying={props.currentSongCid === rs.entry.cid &&
                    props.isPlaybackActive}
                  isPaused={props.currentSongCid === rs.entry.cid &&
                    props.isPlaybackPaused}
                  downloadState={props.getSongDownloadState(rs.entry.cid)}
                  downloadDisabled={props.isSongDownloadInteractionBlocked(
                    rs.entry.cid
                  )}
                  reducedMotion={props.reducedMotion}
                  collections={props.collections}
                  onAddToCollection={props.onAddToCollection}
                  onclick={() =>
                    props.onPlaySong(rs.entry, props.playbackQueue)}
                  onTogglePlay={() => props.onTogglePlay()}
                  onDownload={() => props.onDownloadSong(rs.entry.cid)}
                />
              </div>
              {#if isEditable}
                <button
                  type="button"
                  class="remove-btn"
                  title={m.collection_song_remove_title()}
                  aria-label={m.collection_song_remove_aria()}
                  onclick={(e) => {
                    e.stopPropagation();
                    props.onRemoveSongs(collection.id, [rs.entry.cid]);
                  }}
                >
                  ✕
                </button>
              {/if}
            </div>
          {/each}
        {:else if allSongIds.length === 0}
          <div class="empty-song-list">
            {m.collection_songs_empty()}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</OverlayScrollbarsComponent>

<style>
  :global(.collection-scroll-container) {
    flex: 1;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .collection-detail-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    color: var(--text-tertiary);
    font-size: 14px;
  }

  .collection-detail-card {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 24px;
  }

  .collection-hero {
    display: flex;
    gap: 20px;
  }

  .collection-hero-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .collection-official-tag {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 20px;
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
    color: var(--accent-readable-foreground);
    background: var(--accent);
    padding: 1px 8px 0;
    border-radius: var(--shape-pill);
    letter-spacing: 0.04em;
    width: fit-content;
  }

  .collection-hero-title {
    font-size: 22px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
    line-height: 1.2;
  }

  .collection-hero-description {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .collection-hero-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
  }

  .collection-song-count {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .collection-hero-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }

  .btn {
    appearance: none;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 13px;
    font-weight: 500;
    padding: 6px 14px;
    border-radius: var(--shape-md);
    cursor: pointer;
  }

  .btn:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .btn-danger {
    color: #f87171;
    border-color: rgba(248, 113, 113, 0.3);
  }

  .btn-danger:hover {
    background: rgba(248, 113, 113, 0.1);
    border-color: rgba(248, 113, 113, 0.5);
  }

  .is-reduced-motion .btn {
    transition: none;
  }

  .collection-divider {
    height: 1px;
    background: var(--text-tertiary);
    opacity: 0.25;
    margin: 4px 0;
  }

  .song-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .song-list-loading {
    padding: 32px 16px;
    text-align: center;
    font-size: 13px;
    color: var(--text-tertiary);
  }

  .collection-song-wrapper {
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: 14px;
  }

  .collection-song-wrapper[draggable='true'] {
    cursor: grab;
  }

  .collection-song-wrapper[draggable='true']:active {
    cursor: grabbing;
  }

  .collection-song-row-content {
    flex: 1;
    min-width: 0;
  }

  .drag-handle {
    appearance: none;
    border: none;
    background: none;
    font-size: 14px;
    color: var(--text-tertiary);
    cursor: grab;
    user-select: none;
    flex-shrink: 0;
    padding: 4px;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .remove-btn {
    appearance: none;
    display: inline-flex;
    width: 40px;
    height: 40px;
    align-items: center;
    justify-content: center;
    border: none;
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 0;
    border-radius: var(--shape-sm);
    font-size: 12px;
    opacity: 0;
    pointer-events: none;
    flex-shrink: 0;
  }

  .collection-song-wrapper:hover .remove-btn,
  .collection-song-wrapper:focus-within .remove-btn {
    opacity: 1;
    pointer-events: auto;
  }

  .remove-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  @media (hover: none), (pointer: coarse) {
    .remove-btn {
      opacity: 1;
      pointer-events: auto;
    }
  }

  .section-header {
    display: flex;
    align-items: center;
    padding: 12px 8px 6px;
    margin-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .section-header-first {
    margin-top: 0;
    border-top: none;
  }

  .section-header-title {
    font-family: var(--font-display);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .empty-song-list {
    padding: 32px 16px;
    text-align: center;
    font-size: 13px;
    color: var(--text-tertiary);
  }
</style>
