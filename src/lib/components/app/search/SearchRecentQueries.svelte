<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { gsap, getMotionDuration } from '$lib/design/gsap';
  import type { RecentQuery } from '$lib/features/search/store.svelte';

  interface Props {
    queries: RecentQuery[];
    onSelect: (entry: RecentQuery) => void;
  }

  let { queries, onSelect }: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state();

  const scopeLabelMap: Record<string, string> = {
    all: 'ALL',
    albums: '专辑',
    songs: '歌曲',
  };

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.search_recent_queries_title(),
    };
  });

  function formatRelativeTime(timestamp: number): string {
    const diff = Date.now() - timestamp;
    const minutes = Math.floor(diff / 60_000);
    if (minutes < 1) return '刚刚';
    if (minutes < 60) return `${minutes}分钟前`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}小时前`;
    return `${Math.floor(hours / 24)}天前`;
  }

  $effect(() => {
    if (!containerEl) return;
    const cards = containerEl.querySelectorAll('.query-card');
    if (!cards.length) return;
    gsap.fromTo(
      cards,
      { opacity: 0, y: 8 },
      {
        opacity: 1,
        y: 0,
        stagger: 0.05,
        duration: getMotionDuration(320),
        ease: 'ios-spring',
      }
    );
  });
</script>

{#if queries.length > 0}
  <section class="recent-queries">
    <h2 class="section-title">{labels.title}</h2>
    <div class="queries-row" bind:this={containerEl}>
      {#each queries as entry (entry.query + entry.scope)}
        <button
          type="button"
          class="query-card"
          onclick={() => onSelect(entry)}
        >
          <span class="query-scope" data-scope={entry.scope}>
            {scopeLabelMap[entry.scope] ?? 'ALL'}
          </span>
          <span class="query-text">{entry.query}</span>
          <span class="query-time">{formatRelativeTime(entry.timestamp)}</span>
        </button>
      {/each}
    </div>
  </section>
{/if}

<style>
  .recent-queries {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.02em;
    margin: 0;
    padding: 0 2px;
  }

  .queries-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }

  .query-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 14px;
    border: 1px solid rgba(255, 255, 255, 0.55);
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.62);
    backdrop-filter: blur(12px) saturate(1.2);
    -webkit-backdrop-filter: blur(12px) saturate(1.2);
    box-shadow:
      0 2px 8px rgba(15, 23, 42, 0.05),
      inset 0 1px 0 rgba(255, 255, 255, 0.88);
    cursor: pointer;
    text-align: left;
    font-family: var(--font-body);
  }

  .query-card:hover {
    background: rgba(255, 255, 255, 0.82);
    border-color: rgba(var(--accent-rgb), 0.2);
  }

  .query-card:active {
    transform: scale(0.97);
  }

  .query-scope {
    font-family: var(--font-wide);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.1);
    border-radius: 4px;
    padding: 2px 6px;
    align-self: flex-start;
  }

  .query-scope[data-scope='albums'] {
    color: oklch(from var(--accent) l c calc(h + 22));
    background: oklch(from var(--accent) l c calc(h + 22) / 0.1);
  }

  .query-scope[data-scope='songs'] {
    color: oklch(from var(--accent) l c calc(h - 28));
    background: oklch(from var(--accent) l c calc(h - 28) / 0.1);
  }

  .query-text {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .query-time {
    font-size: 11px;
    color: var(--text-tertiary);
  }
</style>
