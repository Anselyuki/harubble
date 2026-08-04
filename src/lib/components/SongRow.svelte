<script lang="ts">
  import type { SongEntry, TagEntry, CollectionSummary } from '$lib/types';
  import MetadataPopover from '$lib/components/MetadataPopover.svelte';
  import AddToCollectionMenu from '$lib/components/app/collection/AddToCollectionMenu.svelte';
  import { imageDataSrc } from '$lib/imageDataSrc';
  import {
    getDownloadBadgeLabel,
    shouldShowDownloadBadge,
  } from '$lib/downloadBadge';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';
  interface Props {
    song: SongEntry;
    index: number;
    albumCid: string;
    albumName: string;
    coverUrl?: string | null;
    isPlaying?: boolean;
    isPaused?: boolean;
    downloadState?: SongDownloadState;
    downloadDisabled?: boolean;
    selectionMode?: boolean;
    isSelected?: boolean;
    selectionDisabled?: boolean;
    reducedMotion?: boolean;
    albumTags?: TagEntry[];
    collections?: CollectionSummary[];
    onAddToCollection?: (collectionId: string, songCid: string) => void;
    onclick?: () => void;
    onTogglePlay?: () => void;
    onDownload?: () => void;
    onToggleSelection?: () => void;
  }
  let {
    song,
    index,
    albumCid,
    albumName,
    coverUrl = null,
    isPlaying = false,
    isPaused = false,
    downloadState = 'idle',
    downloadDisabled = false,
    selectionMode = false,
    isSelected = false,
    selectionDisabled = false,
    reducedMotion = false,
    albumTags = [],
    collections = [],
    onAddToCollection,
    onclick,
    onTogglePlay,
    onDownload,
    onToggleSelection,
  }: Props = $props();
  let isHovered = $state(false);
  let isFocused = $state(false);
  const showEmphasis = $derived.by(
    () => isPlaying || isHovered || isFocused || isSelected
  );
  const showPlayIndicator = $derived.by(
    () => isPlaying || isHovered || isFocused
  );
  const showDownloadedBadge = $derived.by(() =>
    shouldShowDownloadBadge(song.download.downloadStatus)
  );
  const downloadedBadgeLabel = $derived.by(() =>
    getDownloadBadgeLabel(song.download.downloadStatus)
  );
  const isBusy = $derived.by(() => downloadState !== 'idle');
  const isDownloadDisabled = $derived.by(
    () => isBusy || downloadDisabled || selectionMode
  );
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      downloadCreatingAria: m.common_download_creating_aria({
        name: song.name,
      }),
      downloadQueuedAria: m.common_download_queued_aria({ name: song.name }),
      downloadRunningAria: m.common_download_running_aria({ name: song.name }),
      downloadIdleAria: m.common_download_idle_aria({ name: song.name }),
      downloadCreatingTitle: m.common_download_creating_title(),
      downloadQueuedTitle: m.common_download_queued_title(),
      downloadRunningTitle: m.common_download_running_title(),
      downloadIdleTitle: m.common_download_idle_title(),
      deselectAria: m.common_selection_deselect_aria({ name: song.name }),
      selectAria: m.common_selection_select_aria({ name: song.name }),
      playAria: m.library_song_row_play_aria({ name: song.name }),
    };
  });
  const downloadButtonLabel = $derived.by(() => {
    switch (downloadState) {
      case 'creating':
        return labels.downloadCreatingAria;
      case 'queued':
        return labels.downloadQueuedAria;
      case 'running':
        return labels.downloadRunningAria;
      default:
        return labels.downloadIdleAria;
    }
  });
  const downloadButtonTitle = $derived.by(() => {
    switch (downloadState) {
      case 'creating':
        return labels.downloadCreatingTitle;
      case 'queued':
        return labels.downloadQueuedTitle;
      case 'running':
        return labels.downloadRunningTitle;
      default:
        return labels.downloadIdleTitle;
    }
  });
  function isNestedInteractiveTarget(target: EventTarget | null): boolean {
    return (
      target instanceof Element &&
      Boolean(
        target.closest(
          'button, a, input, select, textarea, [contenteditable="true"], [role="menuitem"]'
        )
      )
    );
  }

  function handleRowActivate(event: MouseEvent) {
    if (isNestedInteractiveTarget(event.target)) return;
    if (selectionMode) {
      if (!selectionDisabled) {
        onToggleSelection?.();
      }
    }
  }

  function handleRowPlay() {
    if (selectionMode) return;
    if (isPlaying) {
      onTogglePlay?.();
    } else {
      onclick?.();
    }
  }

  function handleRowDoubleClick(event: MouseEvent) {
    if (isNestedInteractiveTarget(event.target)) return;
    handleRowPlay();
  }

  function rowPointerInteractions(node: HTMLElement) {
    node.addEventListener('click', handleRowActivate);
    node.addEventListener('dblclick', handleRowDoubleClick);
    return {
      destroy() {
        node.removeEventListener('click', handleRowActivate);
        node.removeEventListener('dblclick', handleRowDoubleClick);
      },
    };
  }

  function computeExtraTags(
    songTags: TagEntry[],
    albumTags: TagEntry[]
  ): TagEntry[] {
    return songTags
      .map((st): TagEntry | null => {
        const albumValues =
          albumTags.find((at) => at.dimension === st.dimension)?.values ?? [];
        const albumSet = new Set(albumValues);
        const filtered = st.values
          .map((v, i) => ({
            value: v,
            color: st.colors?.[i] ?? null,
            keep: !albumSet.has(v),
          }))
          .filter((x) => x.keep);
        if (filtered.length === 0) return null;
        return {
          dimension: st.dimension,
          values: filtered.map((x) => x.value),
          colors: filtered.map((x) => x.color),
        };
      })
      .filter((entry): entry is TagEntry => entry !== null);
  }

  const extraTags = $derived.by(() => computeExtraTags(song.tags, albumTags));
</script>

<!-- Row-level pointer activation is a convenience; explicit child buttons own keyboard actions. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="song-row"
  class:is-selection-mode={selectionMode}
  class:is-selected={isSelected}
  class:is-playing={isPlaying}
  class:is-hovered={isHovered || isFocused}
  class:is-reduced-motion={reducedMotion}
  data-song-cid={song.cid}
  class:has-row-action={selectionMode}
  use:rowPointerInteractions
  onmouseenter={() => {
    isHovered = true;
  }}
  onmouseleave={() => {
    isHovered = false;
  }}
  onfocusin={() => {
    isFocused = true;
  }}
  onfocusout={() => {
    isFocused = false;
  }}
>
  {#if selectionMode}
    <button
      type="button"
      class="song-selection-toggle"
      class:is-selected={isSelected}
      disabled={selectionDisabled}
      aria-label={isSelected ? labels.deselectAria : labels.selectAria}
      aria-pressed={isSelected}
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        onToggleSelection?.();
      }}><span class="song-selection-dot"></span></button
    >
  {/if}
  <div class="song-number" class:is-emphasis={showEmphasis}>{index + 1}</div>
  {#if coverUrl}
    <img
      class="song-cover-thumb"
      use:imageDataSrc={coverUrl}
      alt=""
      aria-hidden="true"
    />
  {/if}
  <div class="song-info">
    <div class="song-name" class:is-emphasis={showEmphasis}>{song.name}</div>
    <div class="song-artists">{song.artists.join(' · ')}</div>
    {#if extraTags.length > 0}
      <div class="song-extra-tags">
        {#each extraTags as tag (tag.dimension)}
          {#each tag.values as value, i (`${tag.dimension}:${i}`)}
            <span
              class="song-tag-badge"
              style:color={tag.colors?.[i] ?? undefined}
              style:background={tag.colors?.[i]
                ? `${tag.colors[i]}1a`
                : undefined}>{value}</span
            >
          {/each}
        {/each}
      </div>
    {/if}
  </div>
  <button
    type="button"
    class="song-play-indicator"
    class:is-playing={isPlaying}
    class:is-visible={showPlayIndicator}
    aria-label={labels.playAria}
    onclick={(event: MouseEvent) => {
      event.stopPropagation();
      if (isPlaying) {
        onTogglePlay?.();
      } else {
        onclick?.();
      }
    }}
  >
    <svg class="play-indicator-icon" viewBox="0 0 24 24" aria-hidden="true">
      {#if isPlaying && !isPaused}<rect
          x="7.15"
          y="5.95"
          width="3.4"
          height="12.1"
          rx="1.25"
        ></rect><rect x="13.45" y="5.95" width="3.4" height="12.1" rx="1.25"
        ></rect>{:else}<path d="M8.2 6.3v11.4L17.35 12z"></path>{/if}
    </svg>
  </button>
  <div class="song-actions">
    {#if showDownloadedBadge}<span class="song-download-badge"
        >{downloadedBadgeLabel}</span
      >{/if}
    <span class="song-meta-wrapper">
      <MetadataPopover target={{ kind: 'song', song, albumCid, albumName }} />
    </span>
    {#if collections.length > 0 && onAddToCollection}
      <AddToCollectionMenu
        {collections}
        onAdd={(colId) => onAddToCollection(colId, song.cid)}
      />
    {/if}
    <button
      type="button"
      class="song-download-button"
      class:is-busy={isBusy}
      disabled={isDownloadDisabled}
      aria-label={downloadButtonLabel}
      title={downloadButtonTitle}
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        onDownload?.();
      }}
    >
      <svg class="download-icon" viewBox="0 0 24 24" aria-hidden="true">
        {#if downloadState === 'creating' || downloadState === 'running'}<circle
            class="download-spinner-ring"
            cx="12"
            cy="12"
            r="8"
          ></circle>{:else if downloadState === 'queued'}<circle
            cx="12"
            cy="12"
            r="2.5"
          ></circle><circle cx="12" cy="6" r="1.5" opacity="0.4"
          ></circle><circle cx="12" cy="18" r="1.5" opacity="0.4"
          ></circle>{:else}<path d="M12 4v12m0 0-4-4m4 4 4-4"></path><path
            d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2"
          ></path>{/if}
      </svg>
    </button>
  </div>
</div>

<style>
  .song-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 16px;
    border-radius: 14px;
    cursor: default;
    outline: none;
    background: rgba(15, 23, 42, 0);
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0);
  }
  .song-row.has-row-action {
    cursor: pointer;
  }
  .song-row:not(.is-reduced-motion):active {
    transform: scale(0.996);
  }
  .song-row.is-hovered:not(.is-playing):not(.is-selected) {
    background: rgba(15, 23, 42, 0.04);
  }
  .song-row.is-playing {
    background: rgba(var(--accent-rgb), 0.1);
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.08);
  }
  .song-row.is-selected {
    background: rgba(var(--accent-rgb), 0.12);
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.12);
  }
  .song-number {
    width: 28px;
    text-align: center;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }
  .song-number.is-emphasis {
    color: var(--accent);
    opacity: 0.86;
  }
  .song-cover-thumb {
    width: 36px;
    height: 36px;
    border-radius: var(--shape-sm);
    object-fit: cover;
    flex-shrink: 0;
  }
  .song-info {
    flex: 1;
    min-width: 0;
  }
  .song-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .song-name.is-emphasis {
    color: var(--accent);
  }
  .song-artists {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 2px;
  }
  .song-extra-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 3px;
  }
  .song-tag-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: var(--shape-xs);
    background: rgba(var(--accent-rgb), 0.08);
    color: var(--text-secondary);
    white-space: nowrap;
  }
  .song-play-indicator {
    appearance: none;
    border: none;
    padding: 0;
    cursor: pointer;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    opacity: 0;
    pointer-events: none;
    background: rgba(15, 23, 42, 0.05);
    color: var(--text-secondary);
    box-shadow: 0 0 0 rgba(var(--accent-rgb), 0);
  }
  .song-play-indicator.is-visible:not(.is-playing) {
    opacity: 1;
    pointer-events: auto;
    background: rgba(var(--accent-rgb), 0.1);
    color: var(--accent);
  }
  .song-play-indicator.is-playing {
    opacity: 1;
    pointer-events: auto;
    background: var(--accent);
    color: var(--accent-readable-foreground);
    box-shadow: 0 10px 20px rgba(var(--accent-rgb), 0.18);
  }
  .song-play-indicator:not(.is-visible):not(.is-playing) {
    transform: scale(0.92);
  }
  .play-indicator-icon {
    width: 16px;
    height: 16px;
    fill: currentColor;
    stroke: none;
  }
  .song-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .song-meta-wrapper {
    display: inline-flex;
    opacity: 0;
    pointer-events: none;
  }
  .song-row:hover .song-meta-wrapper,
  .song-row:focus-within .song-meta-wrapper {
    opacity: 1;
    pointer-events: auto;
  }
  .song-row.is-reduced-motion .song-meta-wrapper {
    opacity: 1;
    pointer-events: auto;
  }
  @media (hover: none), (pointer: coarse) {
    .song-play-indicator,
    .song-meta-wrapper {
      opacity: 1;
      pointer-events: auto;
    }
    .song-play-indicator:not(.is-visible):not(.is-playing) {
      transform: none;
    }
  }
  .song-download-badge {
    display: inline-flex;
    align-items: center;
    padding: 3px 8px;
    border-radius: var(--shape-pill);
    font-size: 11px;
    line-height: 1;
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.1);
    border: 1px solid rgba(var(--accent-rgb), 0.12);
  }
  .song-download-button {
    appearance: none;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .song-download-button:hover:not(:disabled) {
    background: rgba(var(--accent-rgb), 0.1);
    color: var(--accent);
  }
  .song-download-button:disabled {
    opacity: 0.42;
    cursor: default;
  }
  .song-download-button.is-busy {
    color: var(--accent);
    opacity: 1;
  }
  .download-icon {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .download-spinner-ring {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-dasharray: 20 30;
    animation: motion-spin var(--motion-spinner) linear infinite;
  }
  .song-selection-toggle {
    appearance: none;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid var(--text-tertiary);
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    flex-shrink: 0;
  }
  .song-selection-toggle.is-selected {
    border-color: var(--accent);
    background: var(--accent);
  }
  .song-selection-toggle:disabled {
    opacity: 0.42;
    cursor: default;
  }
  .song-selection-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: transparent;
  }
  .song-selection-toggle.is-selected .song-selection-dot {
    background: var(--accent-readable-foreground);
  }
</style>
