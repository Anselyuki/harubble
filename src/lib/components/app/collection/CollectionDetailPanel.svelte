<script lang="ts">
  import { SvelteMap } from 'svelte/reactivity';
  import { fade, fly } from 'svelte/transition';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { PartialOptions } from 'overlayscrollbars';
  import SongRow from '$lib/components/SongRow.svelte';
  import { getSongDetail, getAlbumDetail } from '$lib/api';
  import type {
    Collection,
    CollectionSummary,
    SongEntry,
    SongDetail,
    PlaybackQueueEntry,
  } from '$lib/types';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface ResolvedSong {
    entry: SongEntry;
    albumCid: string;
    albumName: string;
    coverUrl: string | null;
  }

  interface Props {
    collection: Collection | null;
    isLoading: boolean;
    reducedMotion: boolean;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    isPlaybackPaused: boolean;
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

  function dur(base: number): number {
    return props.reducedMotion ? 0 : base;
  }

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

  const isEditable = $derived.by(() => !props.collection?.isOfficial);

  const allSongIds = $derived.by((): string[] => {
    const sections = props.collection?.sections;
    if (!sections) return [];
    return sections.flatMap((s) => s.songIds);
  });

  const songCountLabel = $derived.by(() => `${allSongIds.length} 首歌曲`);

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

  let resolvedSongs = $state<ResolvedSong[]>([]);
  let isResolvingSongs = $state(false);
  let lastResolvedKey = $state<string | null>(null);

  const playbackQueue = $derived.by((): PlaybackQueueEntry[] =>
    resolvedSongs.map((rs) => ({
      cid: rs.entry.cid,
      name: rs.entry.name,
      artists: rs.entry.artists,
      coverUrl: rs.coverUrl,
    }))
  );

  $effect(() => {
    const collection = props.collection;
    if (!collection) {
      resolvedSongs = [];
      lastResolvedKey = null;
      return;
    }
    const ids = collection.sections.flatMap((s) => s.songIds);
    const key = `${collection.id}:${ids.join(',')}`;
    if (key === lastResolvedKey) return;
    void resolveSongs(ids, key);
  });

  async function resolveSongs(songIds: string[], key: string): Promise<void> {
    lastResolvedKey = key;
    isResolvingSongs = true;
    resolvedSongs = [];

    try {
      const details = await Promise.all(
        songIds.map((id) =>
          getSongDetail(id).catch((): SongDetail | null => null)
        )
      );

      const albumCidList: string[] = [];
      for (const d of details) {
        if (d && !albumCidList.includes(d.albumCid)) {
          albumCidList.push(d.albumCid);
        }
      }

      const albumMap: Partial<
        Record<string, { name: string; coverUrl: string | null }>
      > = {};
      const albumResults = await Promise.all(
        albumCidList.map((cid) => getAlbumDetail(cid).catch(() => null))
      );
      for (const album of albumResults) {
        if (album) {
          albumMap[album.cid] = {
            name: album.name,
            coverUrl: album.coverUrl,
          };
        }
      }

      const resolved: ResolvedSong[] = [];
      for (const detail of details) {
        if (!detail) continue;
        const albumInfo = albumMap[detail.albumCid];
        resolved.push({
          entry: {
            cid: detail.cid,
            name: detail.name,
            artists: detail.artists,
            download: detail.download,
            tags: detail.tags,
          },
          albumCid: detail.albumCid,
          albumName: albumInfo?.name ?? '',
          coverUrl: albumInfo?.coverUrl ?? null,
        });
      }

      if (lastResolvedKey === key) {
        resolvedSongs = resolved;
      }
    } finally {
      isResolvingSongs = false;
    }
  }

  let dragSourceIndex = $state<number | null>(null);

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
    <div class="collection-detail-loading" in:fade={{ duration: dur(200) }}>
      <span>加载中…</span>
    </div>
  {:else if !props.collection}
    <div class="collection-detail-loading" in:fade={{ duration: dur(200) }}>
      <span>请从侧边栏选择一个合集</span>
    </div>
  {:else}
    {@const collection = props.collection}
    <div
      class="collection-detail-card"
      class:is-reduced-motion={props.reducedMotion}
      in:fade={{ duration: dur(220) }}
      out:fade={{ duration: dur(220) }}
    >
      <div class="collection-hero">
        <div
          class="collection-hero-info"
          in:fly={{ y: 14, duration: dur(220), delay: dur(30) }}
          out:fly={{ y: 8, duration: dur(220) }}
        >
          {#if collection.isOfficial}
            <span class="collection-official-tag">★ 官方合集</span>
          {/if}
          <h1 class="collection-hero-title">{collection.name}</h1>
          {#if collection.description}
            <p class="collection-hero-description">
              {collection.description}
            </p>
          {/if}
          <div class="collection-hero-meta">
            <span class="collection-song-count">{songCountLabel}</span>
          </div>
          <div class="controls collection-hero-actions">
            {#if isEditable}
              <button type="button" class="btn" onclick={props.onEdit}>
                编辑
              </button>
              <button
                type="button"
                class="btn btn-danger"
                onclick={() => {
                  if (
                    confirm(
                      `确定要删除合集「${collection.name}」吗？此操作不可撤销。`
                    )
                  ) {
                    props.onDelete(collection.id);
                  }
                }}
              >
                删除
              </button>
            {/if}
            <span class="collection-actions-spacer"></span>
            <button
              type="button"
              class="btn"
              onclick={() => props.onExport(collection.id)}
            >
              导出
            </button>
          </div>
        </div>
      </div>

      <div class="collection-divider"></div>

      <div
        class="song-list"
        in:fly={{ y: 10, duration: dur(200), delay: dur(70) }}
        out:fade={{ duration: dur(200) }}
      >
        {#if isResolvingSongs && resolvedSongs.length === 0}
          <div class="song-list-loading">加载歌曲信息…</div>
        {:else if resolvedSongs.length > 0}
          {#each resolvedSongs as rs, index (rs.entry.cid)}
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
                  onclick={() => props.onPlaySong(rs.entry, playbackQueue)}
                  onTogglePlay={() => props.onTogglePlay()}
                  onDownload={() => props.onDownloadSong(rs.entry.cid)}
                />
              </div>
              {#if isEditable}
                <button
                  type="button"
                  class="remove-btn"
                  title="移除"
                  aria-label="从合集中移除"
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
            暂无歌曲，从专辑详情页将歌曲添加到此合集
          </div>
        {/if}
      </div>
    </div>
  {/if}
</OverlayScrollbarsComponent>

<style>
  :global(.collection-scroll-container) {
    flex: 1;
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
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    letter-spacing: 0.04em;
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

  .collection-actions-spacer {
    flex: 1;
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
    border-radius: 8px;
    cursor: pointer;
    transition:
      background-color 0.15s ease,
      border-color 0.15s ease;
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
    transition: background-color 0.12s ease;
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
    border: none;
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 6px;
    font-size: 12px;
    opacity: 0;
    flex-shrink: 0;
    transition:
      opacity 0.12s ease,
      color 0.12s ease,
      background-color 0.12s ease;
  }

  .collection-song-wrapper:hover .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
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
