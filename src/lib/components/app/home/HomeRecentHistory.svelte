<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { imageDataSrc } from '$lib/imageDataSrc';
  import type { HistoryEntry } from '$lib/types';

  interface Props {
    entries: HistoryEntry[];
    onPlay: (entry: HistoryEntry) => void;
    onClear: () => void;
  }

  let { entries, onPlay, onClear }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.home_recent_history_title(),
      clear: m.home_clear_history(),
      empty: m.home_empty_history(),
    };
  });

  function formatRelativeTime(isoString: string): string {
    const date = new Date(isoString);
    const now = Date.now();
    const diffMs = now - date.getTime();
    const diffMin = Math.floor(diffMs / 60000);

    if (diffMin < 1) return m.home_time_just_now();
    if (diffMin < 60) return m.home_time_minutes_ago({ count: diffMin });
    const diffHour = Math.floor(diffMin / 60);
    if (diffHour < 24) return m.home_time_hours_ago({ count: diffHour });
    const diffDay = Math.floor(diffHour / 24);
    if (diffDay < 30) return m.home_time_days_ago({ count: diffDay });
    return date.toLocaleDateString();
  }
</script>

<section class="recent-history" aria-label={labels.title}>
  <div class="section-header">
    <h2 class="section-title">{labels.title}</h2>
    {#if entries.length > 0}
      <button class="clear-btn" onclick={onClear} type="button">
        {labels.clear}
      </button>
    {/if}
  </div>

  {#if entries.length === 0}
    <p class="empty-hint">{labels.empty}</p>
  {:else}
    <div class="history-list">
      {#each entries as entry (entry.id)}
        <button
          class="history-item"
          onclick={() => onPlay(entry)}
          type="button"
        >
          {#if entry.coverUrl}
            <img
              use:imageDataSrc={entry.coverUrl}
              alt=""
              class="history-cover"
              loading="lazy"
            />
          {:else}
            <div class="history-cover-placeholder"></div>
          {/if}
          <div class="history-info">
            <span class="history-song">{entry.songName}</span>
            <span class="history-artist">{entry.artists.join(', ')}</span>
          </div>
          <span class="history-time">{formatRelativeTime(entry.playedAt)}</span>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .recent-history {
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
    font-family: var(--font-display);
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
    margin: 0;
  }

  .clear-btn {
    font-family: var(--font-body);
    font-size: 0.75rem;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    transition: color 0.15s ease;
  }

  .clear-btn:hover {
    color: var(--text-primary, #fff);
  }

  .history-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .history-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem;
    border-radius: 8px;
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    text-align: left;
    transition: background 0.15s ease;
  }

  .history-item:hover {
    background: var(--surface-secondary, rgba(255, 255, 255, 0.06));
  }

  .history-cover {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .history-cover-placeholder {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.06));
    flex-shrink: 0;
  }

  .history-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .history-song {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary, #fff);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .history-artist {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .history-time {
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
    flex-shrink: 0;
  }

  .empty-hint {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
    margin: 0;
  }
</style>
