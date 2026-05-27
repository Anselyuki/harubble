<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { gsap, getMotionDuration } from '$lib/design/gsap';
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import type { Album } from '$lib/types';

  interface Props {
    albums: Album[];
    loading: boolean;
    reducedMotion?: boolean;
    onSelectAlbum: (album: Album) => void;
  }

  let {
    albums,
    loading,
    reducedMotion = false,
    onSelectAlbum,
  }: Props = $props();

  let gridEl: HTMLDivElement | undefined = $state();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.search_recently_played_title(),
    };
  });

  $effect(() => {
    if (!gridEl || albums.length === 0 || reducedMotion) return;
    const cards = Array.from(gridEl.children);
    if (!cards.length) return;
    gsap.fromTo(
      cards,
      { opacity: 0, y: 6 },
      {
        opacity: 1,
        y: 0,
        stagger: 0.03,
        duration: getMotionDuration(360),
        ease: 'ios-spring',
        delay: getMotionDuration(80),
      }
    );
  });
</script>

{#if loading}
  <section class="recently-played">
    <h2 class="section-title">{labels.title}</h2>
    <div class="loading-grid">
      {#each Array(8) as _, i (i)}
        <div class="skeleton-card"></div>
      {/each}
    </div>
  </section>
{:else if albums.length > 0}
  <section class="recently-played">
    <h2 class="section-title">{labels.title}</h2>
    <div class="album-grid" bind:this={gridEl}>
      {#each albums as album (album.cid)}
        <AlbumCard
          {album}
          layout="grid"
          selected={false}
          {reducedMotion}
          onclick={() => onSelectAlbum(album)}
        />
      {/each}
    </div>
  </section>
{/if}

<style>
  .recently-played {
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

  .album-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 18px;
  }

  .loading-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 18px;
  }

  .skeleton-card {
    aspect-ratio: 1;
    border-radius: 12px;
    background: linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.15) 25%,
      rgba(255, 255, 255, 0.3) 50%,
      rgba(255, 255, 255, 0.15) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-shimmer 1.6s ease-in-out infinite;
  }

  @keyframes skeleton-shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
