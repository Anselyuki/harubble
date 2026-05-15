<script lang="ts">
  import type { Album } from '$lib/types';
  import { lazyLoad } from '$lib/lazyLoad';
  import {
    getDownloadBadgeLabel,
    shouldShowAlbumListDownloadBadge,
  } from '$lib/downloadBadge';

  interface Props {
    album: Album;
    layout?: 'list' | 'grid';
    selected?: boolean;
    reducedMotion?: boolean;
    onclick?: () => void;
  }

  let {
    album,
    layout = 'list',
    selected = false,
    reducedMotion = false,
    onclick,
  }: Props = $props();

  const showDownloadBadge = $derived.by(() =>
    shouldShowAlbumListDownloadBadge(album.download.downloadStatus)
  );
  const downloadBadgeLabel = $derived.by(() =>
    getDownloadBadgeLabel(album.download.downloadStatus)
  );

  function handleActivate() {
    onclick?.();
  }
</script>

<button
  type="button"
  class={`album-card album-card--${layout}${selected ? ' selected' : ''}${reducedMotion ? ' is-reduced-motion' : ''}`}
  onclick={handleActivate}
>
  <div
    class="album-cover-wrapper"
    use:lazyLoad={{ rootMargin: '150px', reducedMotion }}
    data-src={album.coverUrl}
  >
    <div class="album-cover-placeholder">♪</div>
    <img class="album-cover-img" alt={album.name} />
  </div>
  <div class="album-info">
    <div class="album-name">{album.name}</div>
    <div class="album-artists">{album.artists.join(', ')}</div>
    {#if showDownloadBadge}
      <span class="album-download-badge">{downloadBadgeLabel}</span>
    {/if}
  </div>
</button>

<style>
  :global(.album-card) {
    appearance: none;
    background: transparent;
    border: none;
    border-radius: 12px;
    padding: 12px;
    margin-bottom: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 12px;
    outline: none;
    font: inherit;
    text-align: left;
    color: inherit;
    box-shadow: inset 0 0 0 1px transparent;
    transform: translateY(0) scale(1);
    transition: transform 0.16s ease;
  }

  :global(.album-card:not(.selected)) {
    transition:
      background-color 0.16s ease,
      box-shadow 0.16s ease,
      transform 0.16s ease;
  }

  :global(.album-card:hover:not(.selected)),
  :global(.album-card:focus-visible:not(.selected)) {
    background: var(--hover-bg-elevated);
    box-shadow: 0 2px 8px rgba(15, 23, 42, 0.05);
  }

  :global(.album-card:hover:not(.selected):not(.is-reduced-motion)),
  :global(.album-card:focus-visible:not(.selected):not(.is-reduced-motion)) {
    transform: translateY(-1px);
  }

  :global(.album-card:active:not(.selected):not(.is-reduced-motion)) {
    transform: translateY(0) scale(0.99);
  }

  :global(.album-card.selected) {
    background: rgba(var(--accent-rgb), 0.1);
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.12);
    transform: translateY(0) scale(1);
    transition: none;
  }

  @media (prefers-color-scheme: dark) {
    :global(.album-card.selected) {
      background: rgba(var(--accent-rgb), 0.18);
    }
  }

  :global(.album-card:focus-visible:not(.selected)) {
    box-shadow:
      inset 0 0 0 1px rgba(var(--accent-rgb), 0.18),
      0 0 0 4px rgba(var(--accent-rgb), 0.08);
  }

  :global(.album-card.selected:focus-visible) {
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.12);
  }

  :global(.album-card.selected) .album-name {
    color: var(--accent);
  }

  .album-cover-wrapper {
    width: 48px;
    height: 48px;
    border-radius: 8px;
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
    box-shadow: 0 0 0 rgba(var(--accent-rgb), 0);
    transition: box-shadow 0.16s ease;
  }

  :global(.album-card:hover:not(.selected)) .album-cover-wrapper,
  :global(.album-card:focus-visible:not(.selected)) .album-cover-wrapper {
    box-shadow: 0 8px 18px rgba(var(--accent-rgb), 0.16);
  }

  :global(.album-card.selected) .album-cover-wrapper {
    box-shadow: 0 0 0 rgba(var(--accent-rgb), 0);
    transition: none;
  }

  :global(.album-card.is-reduced-motion) .album-cover-wrapper {
    transition: none;
  }

  .album-cover-placeholder {
    color: var(--text-tertiary);
    font-size: 20px;
    opacity: 1;
  }

  .album-cover-img {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    border-radius: 8px;
    opacity: 0;
    transform: scale(1.04);
  }

  .album-info {
    flex: 1;
    min-width: 0;
  }

  .album-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 2px;
  }

  .album-artists {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album-download-badge {
    display: inline-flex;
    align-items: center;
    margin-top: 6px;
    padding: 4px 8px;
    border-radius: 999px;
    font-size: 11px;
    line-height: 1;
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.1);
    border: 1px solid rgba(var(--accent-rgb), 0.12);
  }

  /* ── Grid layout variant ── */

  :global(.album-card.album-card--grid) {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    padding: 8px;
    margin-bottom: 0;
  }

  :global(.album-card.album-card--grid) .album-cover-wrapper {
    width: 100%;
    height: 0;
    padding-bottom: 100%;
    border-radius: 12px;
  }

  :global(.album-card.album-card--grid) .album-cover-placeholder {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 28px;
  }

  :global(.album-card.album-card--grid) .album-info {
    padding: 0 4px 4px;
  }

  :global(.album-card.album-card--grid) .album-name {
    font-size: 14px;
    font-weight: 500;
  }

  :global(.album-card.album-card--grid) .album-artists {
    font-size: 12px;
    font-weight: 400;
  }

  :global(.album-card.album-card--grid) .album-download-badge {
    margin-top: 4px;
  }

  :global(
    .album-card.album-card--grid:hover:not(.selected):not(.is-reduced-motion)
  ) {
    transform: translateY(-2px);
    box-shadow: 0 6px 16px rgba(15, 23, 42, 0.1);
  }
</style>
