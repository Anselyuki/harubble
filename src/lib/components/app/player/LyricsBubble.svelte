<script lang="ts">
  import { fly } from 'svelte/transition';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type { LyricLine } from '$lib/features/player/lyrics';

  interface Props {
    loading: boolean;
    error: string;
    lines: LyricLine[];
    activeLyricIndex: number;
    songName: string;
    reducedMotion: boolean;
    onClose: () => void;
  }

  let {
    loading,
    error,
    lines,
    activeLyricIndex,
    songName,
    reducedMotion,
    onClose,
  }: Props = $props();

  let bubbleRef = $state<HTMLElement | null>(null);
  let listRef = $state<HTMLElement | null>(null);

  function dur(base: number): number {
    return reducedMotion ? 0 : base;
  }

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      eyebrow: m.player_lyrics_eyebrow(),
      loading: m.player_lyrics_loading(),
      empty: m.player_lyrics_empty(),
    };
  });

  const countLabel = $derived.by(() => {
    void localeState.current;
    return lines.length > 0
      ? m.player_lyrics_line_count({ count: lines.length })
      : labels.eyebrow;
  });

  $effect(() => {
    if (activeLyricIndex < 0 || !listRef) return;
    const activeEl = listRef.children[activeLyricIndex] as
      | HTMLElement
      | undefined;
    activeEl?.scrollIntoView({
      block: 'center',
      behavior: reducedMotion ? 'instant' : 'smooth',
    });
  });

  $effect(() => {
    function handlePointerDown(event: PointerEvent) {
      if (bubbleRef && !bubbleRef.contains(event.target as Node)) {
        onClose();
      }
    }
    document.addEventListener('pointerdown', handlePointerDown, true);
    return () =>
      document.removeEventListener('pointerdown', handlePointerDown, true);
  });
</script>

<div
  class="lyrics-bubble"
  bind:this={bubbleRef}
  transition:fly={{ y: 8, duration: dur(180) }}
>
  <div class="lyrics-bubble-header">
    <div>
      <p class="lyrics-bubble-eyebrow">{labels.eyebrow}</p>
      <h3 class="lyrics-bubble-title">{songName}</h3>
    </div>
    <span class="lyrics-bubble-count">{countLabel}</span>
  </div>

  {#if loading}
    <div class="lyrics-bubble-empty">{labels.loading}</div>
  {:else if error}
    <div class="lyrics-bubble-empty">{error}</div>
  {:else if lines.length > 0}
    <div class="lyrics-bubble-body" bind:this={listRef}>
      {#each lines as line, index (line.id)}
        <p
          class={`lyrics-bubble-line${index === activeLyricIndex ? ' active' : ''}`}
        >
          {line.text}
        </p>
      {/each}
    </div>
  {:else}
    <div class="lyrics-bubble-empty">{labels.empty}</div>
  {/if}

  <div class="lyrics-bubble-arrow" aria-hidden="true"></div>
</div>
