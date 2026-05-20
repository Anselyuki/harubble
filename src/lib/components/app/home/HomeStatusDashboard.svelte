<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { imageDataSrc } from '$lib/imageDataSrc';
  import type { HomepageStatus } from '$lib/types';

  interface Props {
    status: HomepageStatus | null;
    currentSong: {
      cid: string;
      name: string;
      artists: string[];
      coverUrl: string | null;
    } | null;
    isPlaying: boolean;
    activeDownloadCount: number;
  }

  let { status, currentSong, isPlaying, activeDownloadCount }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      overviewAria: m.home_overview_aria(),
      title: m.home_overview_title(),
      platformAlbums: m.home_stat_platform_albums(),
      platformTracks: m.home_stat_platform_tracks(),
      downloaded: m.home_stat_downloaded(),
      localStorage: m.home_stat_local_storage(),
      downloading: m.home_stat_downloading(),
      completed: m.home_stat_completed_downloads(),
      nowPlayingAria: m.home_now_playing_aria(),
      nowPlaying: m.home_now_playing(),
      paused: m.home_paused(),
    };
  });

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024)
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
</script>

<section class="status-dashboard" aria-label={labels.overviewAria}>
  <h2 class="section-title">{labels.title}</h2>

  <div class="stat-grid">
    <div class="stat-card">
      <span class="stat-label">{labels.platformAlbums}</span>
      <span class="stat-value">{status?.platformAlbumCount ?? '—'}</span>
    </div>

    <div class="stat-card">
      <span class="stat-label">{labels.platformTracks}</span>
      <span class="stat-value">{status?.platformSongCount ?? '—'}</span>
    </div>

    <div class="stat-card">
      <span class="stat-label">{labels.downloaded}</span>
      <span class="stat-value">{status?.localDownloadedCount ?? '—'}</span>
    </div>

    <div class="stat-card">
      <span class="stat-label">{labels.localStorage}</span>
      <span class="stat-value">
        {status ? formatBytes(status.localStorageBytes) : '—'}
      </span>
    </div>

    <div class="stat-card" class:active={activeDownloadCount > 0}>
      <span class="stat-label">{labels.downloading}</span>
      <span class="stat-value">{activeDownloadCount}</span>
    </div>

    <div class="stat-card">
      <span class="stat-label">{labels.completed}</span>
      <span class="stat-value">{status?.completedDownloadCount ?? '—'}</span>
    </div>
  </div>

  {#if currentSong}
    <div class="now-playing" aria-label={labels.nowPlayingAria}>
      {#if currentSong.coverUrl}
        <img
          use:imageDataSrc={currentSong.coverUrl}
          alt=""
          class="np-cover"
          class:playing={isPlaying}
        />
      {:else}
        <div class="np-cover-placeholder" class:playing={isPlaying}></div>
      {/if}
      <div class="np-info">
        <span class="np-label"
          >{isPlaying ? labels.nowPlaying : labels.paused}</span
        >
        <span class="np-song">{currentSong.name}</span>
        <span class="np-artist">{currentSong.artists.join(', ')}</span>
      </div>
    </div>
  {/if}
</section>

<style>
  .status-dashboard {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
    margin: 0;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.5rem;
  }

  .stat-card {
    background: var(--surface-secondary, rgba(255, 255, 255, 0.04));
    border-radius: 10px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    transition: background 0.15s ease;
  }

  .stat-card.active {
    background: var(--accent-surface, rgba(var(--accent-rgb), 0.12));
  }

  .stat-label {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
  }

  .stat-value {
    font-family: var(--font-mono);
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.04));
    border-radius: 10px;
    padding: 0.75rem;
  }

  .np-cover {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .np-cover.playing {
    animation: spin 8s linear infinite;
  }

  .np-cover-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    background: var(--surface-tertiary, rgba(255, 255, 255, 0.08));
    flex-shrink: 0;
  }

  .np-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .np-label {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    color: var(--accent-color, var(--text-secondary, rgba(255, 255, 255, 0.6)));
  }

  .np-song {
    font-family: var(--font-body);
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-primary, #fff);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .np-artist {
    font-family: var(--font-body);
    font-size: 0.75rem;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
