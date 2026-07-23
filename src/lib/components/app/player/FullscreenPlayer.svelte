<script lang="ts">
  import { flushSync } from 'svelte';
  import {
    gsap,
    animateIn,
    getMotionDuration,
    killTweens,
    gsapScrollIntoView,
    MOTION,
  } from '$lib/design/gsap';
  import { getImageSrc } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import {
    getPlayerContext,
    getDownloadContext,
    getShellContext,
  } from '$lib/contexts';
  import { formatTime } from '$lib/features/player/formatUtils';
  import { getNextRepeatMode } from '$lib/features/player/repeatMode';
  import type { RepeatMode } from '$lib/types';
  import MotionMarqueeInner from '$lib/components/MotionMarqueeInner.svelte';
  import PlayToggleGlyph from '$lib/components/app/player/PlayToggleGlyph.svelte';
  import { lyricActiveTween } from '$lib/design/actions';
  import { Repeat, Repeat1, Shuffle } from '@lucide/svelte';

  const player = getPlayerContext();
  const download = getDownloadContext();
  const shell = getShellContext();

  // song is guaranteed non-null by the {#if} guard in App.svelte
  const song = $derived(player.currentSong!);

  const downloadState = $derived(
    player.currentSong
      ? download.getSongDownloadState(player.currentSong.cid)
      : 'idle'
  );
  const downloadDisabled = $derived(
    player.currentSong
      ? download.isSongDownloadInteractionBlocked(player.currentSong.cid)
      : false
  );
  const onTogglePlay = $derived(
    player.isPlaying ? player.pause : player.resume
  );
  const playButtonLoading = $derived(
    player.isLoading || player.isPlayTogglePending
  );
  let playToggleTransitionKey = $state(0);

  function handlePlayToggle() {
    if (playButtonLoading) return;
    playToggleTransitionKey += 1;
    flushSync();
    void onTogglePlay();
  }

  function handleDownload() {
    if (player.currentSong) {
      void download.handleSongDownload(player.currentSong.cid);
    }
  }

  function handleClose(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    player.toggleFullscreen();
  }

  let dialogEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (dialogEl) {
      dialogEl.focus();
    }
  });

  let lyricsListRef = $state<HTMLElement | null>(null);
  let seekPreview = $state<number | null>(null);
  let draggingSeek = $state(false);
  let resolvedCoverUrl = $state<string | null>(null);
  let activeCoverUrl: string | null = null;
  let coverRequestSeq = 0;
  let titleRef = $state<HTMLElement | null>(null);
  let artistRef = $state<HTMLElement | null>(null);
  let titleOverflows = $state(false);
  let artistOverflows = $state(false);
  let metaHovered = $state(false);

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function readRangeValue(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  let coverWrapEl = $state<HTMLElement | undefined>();

  $effect(() => {
    if (!dialogEl) return;
    killTweens(dialogEl);
    gsap.fromTo(
      dialogEl,
      { opacity: 0, y: 60, scale: 0.92 },
      {
        opacity: 1,
        y: 0,
        scale: 1,
        duration: getMotionDuration(MOTION.PAGE),
        ease: 'ios-spring',
      }
    );
    return () => killTweens(dialogEl!);
  });

  $effect(() => {
    if (!coverWrapEl) return;
    animateIn(
      coverWrapEl,
      { opacity: 0, scale: 0.85 },
      { opacity: 1, scale: 1 },
      MOTION.SLOW,
      'ios-spring'
    );
    return () => killTweens(coverWrapEl!);
  });

  const safeDuration = $derived(player.duration > 0 ? player.duration : 1);
  const shownProgress = $derived(seekPreview ?? player.progress);
  const progressRatio = $derived(clamp(shownProgress / safeDuration, 0, 1));
  const canSeek = $derived(player.duration > 0 && !player.isLoading);

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      close: m.player_fullscreen_close(),
      noLyrics: m.player_fullscreen_no_lyrics(),
      lyricsLoading: m.player_lyrics_loading(),
      unknownArtist: m.player_unknown_artist(),
      repeatOff: m.player_repeat_off(),
      repeatOne: m.player_repeat_one(),
      repeatAll: m.player_repeat_all(),
    };
  });

  const artistText = $derived(
    song.artists.length ? song.artists.join(' · ') : labels.unknownArtist
  );
  const repeatLabel = $derived.by(() => {
    const mode: RepeatMode = player.repeatMode;
    if (mode === 'one') return labels.repeatOne;
    if (mode === 'all') return labels.repeatAll;
    return labels.repeatOff;
  });

  const downloadButtonLabel = $derived.by(() => {
    switch (downloadState) {
      case 'creating':
        return m.common_download_creating_aria({ name: song.name });
      case 'queued':
        return m.common_download_queued_aria({ name: song.name });
      case 'running':
        return m.common_download_running_aria({ name: song.name });
      default:
        return m.common_download_idle_aria({ name: song.name });
    }
  });
  const canDownload = $derived(
    !player.isLoading && downloadState === 'idle' && !downloadDisabled
  );

  $effect(() => {
    const coverUrl = song.coverUrl ?? null;
    if (coverUrl === activeCoverUrl) return;
    activeCoverUrl = coverUrl;
    const seq = ++coverRequestSeq;
    if (!coverUrl) {
      resolvedCoverUrl = null;
      return;
    }
    void (async () => {
      try {
        const imageSrc = await getImageSrc(coverUrl);
        if (seq !== coverRequestSeq) return;
        resolvedCoverUrl = imageSrc;
      } catch {
        if (seq !== coverRequestSeq) return;
        resolvedCoverUrl = null;
      }
    })();
  });

  $effect(() => {
    if (player.activeLyricIndex < 0 || !lyricsListRef) return;
    const el = lyricsListRef.children[player.activeLyricIndex] as
      | HTMLElement
      | undefined;
    if (el) gsapScrollIntoView(lyricsListRef, el, 'center');
  });

  $effect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        event.preventDefault();
        player.toggleFullscreen();
        return;
      }
      if (event.key === 'ArrowUp') {
        event.preventDefault();
        player.setVolume(Math.min(1, player.volume + 0.05));
        return;
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        player.setVolume(Math.max(0, player.volume - 0.05));
        return;
      }
      if (event.key === 'm' || event.key === 'M') {
        event.preventDefault();
        player.toggleMute();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  });

  $effect(() => {
    if (
      !draggingSeek &&
      seekPreview !== null &&
      Math.abs(seekPreview - player.progress) < 0.25
    ) {
      seekPreview = null;
    }
  });

  $effect(() => {
    let rafId = 0;
    const observer = new ResizeObserver(() => {
      // 延迟到下一帧再写回，避免回调内同步改动布局触发
      // "ResizeObserver loop completed with undelivered notifications" 警告
      if (rafId) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = 0;
        if (titleRef) {
          titleOverflows = titleRef.scrollWidth > titleRef.clientWidth;
        }
        if (artistRef) {
          artistOverflows = artistRef.scrollWidth > artistRef.clientWidth;
        }
      });
    });

    if (titleRef) observer.observe(titleRef);
    if (artistRef) observer.observe(artistRef);

    return () => {
      if (rafId) cancelAnimationFrame(rafId);
      observer.disconnect();
    };
  });

  function handleSeekInput(event: Event) {
    if (!canSeek) return;
    draggingSeek = true;
    seekPreview = clamp(readRangeValue(event), 0, player.duration || 0);
  }

  function handleSeekChange(event: Event) {
    draggingSeek = false;
    if (!canSeek) {
      seekPreview = null;
      return;
    }
    const target = clamp(readRangeValue(event), 0, player.duration);
    seekPreview = target;
    try {
      player.seek(target);
    } catch {
      seekPreview = null;
    }
  }

  function handleLyricSeek(time: number) {
    if (!canSeek) return;
    seekPreview = time;
    try {
      player.seek(time);
    } catch {
      seekPreview = null;
    }
  }
</script>

<div
  class="fullscreen-player"
  data-ark-screen="fullscreen-player"
  role="dialog"
  aria-modal="true"
  aria-label={song.name}
  tabindex="-1"
  bind:this={dialogEl}
>
  <div
    class="fullscreen-drag-region"
    data-tauri-drag-region
    aria-hidden="true"
  ></div>

  {#if resolvedCoverUrl}
    <div
      class="fullscreen-bg fullscreen-bg-base"
      style="background-image: url({resolvedCoverUrl})"
      aria-hidden="true"
    ></div>
    <div
      class="fullscreen-bg fullscreen-bg-detail"
      style="background-image: url({resolvedCoverUrl})"
      aria-hidden="true"
    ></div>
  {/if}

  <button
    type="button"
    class="fullscreen-close"
    data-testid="fullscreen-close"
    aria-label={labels.close}
    onpointerdown={(event) => event.stopPropagation()}
    onclick={handleClose}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M18 6 6 18"></path>
      <path d="m6 6 12 12"></path>
    </svg>
  </button>

  <div class="fullscreen-left">
    <div class="fullscreen-cover-wrap" bind:this={coverWrapEl}>
      {#if resolvedCoverUrl}
        <img
          src={resolvedCoverUrl}
          alt={m.player_cover_alt({ name: song.name })}
          class="fullscreen-cover"
        />
      {:else}
        <div
          class="fullscreen-cover fullscreen-cover-fallback"
          aria-hidden="true"
        >
          <svg viewBox="0 0 24 24"
            ><path d="M12 3v10.5a4 4 0 1 0 2 3.5V7h4V3h-6z" /></svg
          >
        </div>
      {/if}
    </div>

    <div class="fullscreen-meta">
      <div
        class="fullscreen-meta-text"
        role="presentation"
        onpointerenter={() => (metaHovered = true)}
        onpointerleave={() => (metaHovered = false)}
      >
        <h2
          class="fullscreen-title"
          class:overflowing={titleOverflows}
          bind:this={titleRef}
        >
          <MotionMarqueeInner
            active={metaHovered && titleOverflows}
            reducedMotion={shell.prefersReducedMotion}
          >
            {song.name}
          </MotionMarqueeInner>
        </h2>
        <p
          class="fullscreen-artist"
          class:overflowing={artistOverflows}
          bind:this={artistRef}
        >
          <MotionMarqueeInner
            active={metaHovered && artistOverflows}
            reducedMotion={shell.prefersReducedMotion}
          >
            {artistText}
          </MotionMarqueeInner>
        </p>
      </div>
      <button
        type="button"
        class="fs-btn fs-download"
        class:download-active={downloadState !== 'idle'}
        aria-label={downloadButtonLabel}
        title={downloadButtonLabel}
        disabled={!canDownload}
        onclick={() => handleDownload()}
      >
        {#if downloadState === 'creating'}
          <svg class="fs-spin" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M21 12a9 9 0 1 1-2.64-6.36"></path>
            <path d="M21 3v6h-6"></path>
          </svg>
        {:else}
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 5v9"></path>
            <path d="m8.5 10.5 3.5 3.5 3.5-3.5"></path>
            <path d="M5 18h14"></path>
            {#if downloadState === 'queued'}
              <path d="M8 4.5h8"></path>
            {/if}
          </svg>
        {/if}
      </button>
    </div>

    <div
      class="fullscreen-progress"
      style="--fs-progress:{progressRatio * 100}%"
    >
      <input
        class="fullscreen-seek"
        type="range"
        min="0"
        max={safeDuration}
        value={shownProgress}
        step="0.1"
        disabled={!canSeek}
        oninput={handleSeekInput}
        onchange={handleSeekChange}
      />
      <div class="fullscreen-times">
        <span>{formatTime(shownProgress)}</span>
        <span>{formatTime(player.duration)}</span>
      </div>
    </div>
    <div class="fullscreen-controls">
      <button
        type="button"
        class="fs-btn fs-mode-toggle"
        aria-label={m.player_aria_shuffle()}
        aria-pressed={player.shuffleEnabled}
        disabled={player.isLoading}
        onclick={() => player.toggleShuffle(!player.shuffleEnabled)}
      >
        <Shuffle class="fs-mode-icon" aria-hidden="true" />
      </button>

      <button
        type="button"
        class="fs-btn fs-transport-button"
        aria-label={m.player_aria_previous()}
        disabled={!player.hasPrevious || player.isLoading}
        onclick={() => player.playPrevious()}
      >
        <svg class="fs-solid" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M18.6 6.9v10.2L11.75 12z"></path>
          <path d="M12.2 6.9v10.2L5.35 12z"></path>
        </svg>
      </button>

      <button
        type="button"
        class="fs-btn fs-play"
        class:playing={player.isPlaying}
        aria-label={playButtonLoading
          ? m.player_status_loading()
          : player.isPlaying
            ? m.player_aria_pause()
            : player.isPaused
              ? m.player_aria_resume()
              : m.player_aria_play()}
        disabled={playButtonLoading}
        aria-busy={playButtonLoading}
        onclick={handlePlayToggle}
      >
        <PlayToggleGlyph
          isPlaying={player.isPlaying}
          isLoading={player.isLoading}
          isPending={player.isPlayTogglePending}
          transitionKey={playToggleTransitionKey}
          reducedMotion={shell.prefersReducedMotion}
          size="52px"
        />
      </button>

      <button
        type="button"
        class="fs-btn fs-transport-button"
        aria-label={m.player_aria_next()}
        disabled={!player.hasNext || player.isLoading}
        onclick={() => player.playNext()}
      >
        <svg class="fs-solid" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5.4 6.9v10.2L12.25 12z"></path>
          <path d="M11.8 6.9v10.2L18.65 12z"></path>
        </svg>
      </button>

      <button
        type="button"
        class="fs-btn fs-mode-toggle fs-repeat-toggle"
        aria-label={m.player_aria_repeat_toggle({ mode: repeatLabel })}
        aria-pressed={player.repeatMode !== 'off'}
        disabled={player.isLoading}
        onclick={() =>
          player.toggleRepeat(getNextRepeatMode(player.repeatMode))}
      >
        {#if player.repeatMode === 'one'}
          <Repeat1 class="fs-mode-icon" aria-hidden="true" />
        {:else}
          <Repeat class="fs-mode-icon" aria-hidden="true" />
        {/if}
      </button>
    </div>
  </div>

  <div class="fullscreen-right">
    {#if player.lyricsLoading}
      <div class="fullscreen-lyrics-empty">{labels.lyricsLoading}</div>
    {:else if player.lyricsError}
      <div class="fullscreen-lyrics-empty">{player.lyricsError}</div>
    {:else if player.lyricsLines.length > 0}
      <div class="fullscreen-lyrics" bind:this={lyricsListRef}>
        {#each player.lyricsLines as line, index (line.id)}
          {#if line.time !== null && canSeek}
            <button
              type="button"
              class="fullscreen-lyric-line seekable"
              class:active={index === player.activeLyricIndex}
              use:lyricActiveTween={index === player.activeLyricIndex}
              onclick={() => handleLyricSeek(line.time!)}
            >
              {line.text}
            </button>
          {:else}
            <p
              class="fullscreen-lyric-line"
              class:active={index === player.activeLyricIndex}
              use:lyricActiveTween={index === player.activeLyricIndex}
            >
              {line.text}
            </p>
          {/if}
        {/each}
      </div>
    {:else}
      <div class="fullscreen-lyrics-empty">{labels.noLyrics}</div>
    {/if}
  </div>
</div>
