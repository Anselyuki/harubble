<script lang="ts">
  import { animateIn, killTweens, gsapScrollIntoView } from '$lib/design/gsap';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type { LyricLine } from '$lib/features/player/lyrics';
  import { createLyricsAutoFollowController } from './lyrics-auto-follow';

  interface Props {
    loading: boolean;
    error: string;
    lines: LyricLine[];
    activeLyricIndex: number;
    isPlaying: boolean;
    canSeek: boolean;
    reducedMotion: boolean;
    onSeek: (positionSecs: number) => void | Promise<void>;
    onClose: () => void;
  }

  let {
    loading,
    error,
    lines,
    activeLyricIndex,
    isPlaying,
    canSeek,
    reducedMotion: _reducedMotion,
    onSeek,
    onClose,
  }: Props = $props();

  let bubbleRef = $state<HTMLElement | null>(null);
  let listRef = $state<HTMLElement | null>(null);

  function followActiveLyric() {
    if (activeLyricIndex < 0 || !listRef) return;
    const activeEl = listRef.children[activeLyricIndex] as
      | HTMLElement
      | undefined;
    if (activeEl) gsapScrollIntoView(listRef, activeEl, 'center');
  }

  const autoFollow = createLyricsAutoFollowController({
    isPlaying: () => isPlaying,
    followActiveLyric,
  });

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      ariaLabel: m.player_lyrics_eyebrow(),
      loading: m.player_lyrics_loading(),
      empty: m.player_lyrics_empty(),
    };
  });

  $effect(() => {
    void activeLyricIndex;
    if (!autoFollow.followSuspended) followActiveLyric();
  });

  $effect(() => {
    void isPlaying;
    autoFollow.handlePlaybackChange();
  });

  $effect(() => {
    return () => autoFollow.destroy();
  });

  function trackUserScrolling(node: HTMLElement) {
    const handleScrollIntent = () => {
      killTweens(node);
      autoFollow.handleUserScrollIntent();
    };
    const handleScrollKey = (event: KeyboardEvent) => {
      if (
        ['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End'].includes(
          event.key
        )
      ) {
        handleScrollIntent();
      }
    };
    const handleScroll = () => autoFollow.handleScroll();

    node.addEventListener('wheel', handleScrollIntent, { passive: true });
    node.addEventListener('touchmove', handleScrollIntent, { passive: true });
    node.addEventListener('keydown', handleScrollKey);
    node.addEventListener('scroll', handleScroll, { passive: true });

    return {
      destroy() {
        node.removeEventListener('wheel', handleScrollIntent);
        node.removeEventListener('touchmove', handleScrollIntent);
        node.removeEventListener('keydown', handleScrollKey);
        node.removeEventListener('scroll', handleScroll);
      },
    };
  }

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

  $effect(() => {
    if (!bubbleRef) return;
    animateIn(
      bubbleRef,
      { opacity: 0, y: 8 },
      { opacity: 1, y: 0 },
      180,
      'ios-spring'
    );
    return () => killTweens(bubbleRef!);
  });
</script>

<div
  class="lyrics-bubble"
  bind:this={bubbleRef}
  role="region"
  aria-label={labels.ariaLabel}
>
  {#if loading}
    <div class="lyrics-bubble-empty">{labels.loading}</div>
  {:else if error}
    <div class="lyrics-bubble-empty">{error}</div>
  {:else if lines.length > 0}
    <div
      class="lyrics-bubble-body"
      bind:this={listRef}
      role="group"
      use:trackUserScrolling
    >
      {#each lines as line, index (line.id)}
        {#if line.time !== null && canSeek}
          <button
            type="button"
            class="lyrics-bubble-line seekable"
            class:active={index === activeLyricIndex}
            onclick={() => onSeek(line.time!)}
          >
            {line.text}
          </button>
        {:else}
          <p
            class="lyrics-bubble-line"
            class:active={index === activeLyricIndex}
          >
            {line.text}
          </p>
        {/if}
      {/each}
    </div>
  {:else}
    <div class="lyrics-bubble-empty">{labels.empty}</div>
  {/if}
</div>
