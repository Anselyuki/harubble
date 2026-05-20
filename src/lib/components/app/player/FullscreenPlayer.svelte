<script lang="ts">
  import { scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { getImageDataUrl } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type { LyricLine } from '$lib/features/player/lyrics';

  let dialogEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (dialogEl) {
      dialogEl.focus();
    }
  });

  type RepeatMode = 'all' | 'one';
  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Song {
    name: string;
    artists: string[];
    coverUrl: string | null;
  }

  interface Props {
    song: Song;
    isPlaying: boolean;
    isPaused: boolean;
    isLoading: boolean;
    hasPrevious: boolean;
    hasNext: boolean;
    progress: number;
    duration: number;
    isShuffled: boolean;
    repeatMode: RepeatMode;
    lyricsLoading: boolean;
    lyricsError: string;
    lyricsLines: LyricLine[];
    activeLyricIndex: number;
    reducedMotion: boolean;
    onPrevious: () => void | Promise<void>;
    onTogglePlay: () => void | Promise<void>;
    onSeek: (positionSecs: number) => void | Promise<void>;
    onNext: () => void | Promise<void>;
    onShuffleChange: (next: boolean) => void | Promise<void>;
    onRepeatModeChange: (next: RepeatMode) => void | Promise<void>;
    onDownload: () => void | Promise<void>;
    downloadState: SongDownloadState;
    downloadDisabled: boolean;
    volume?: number;
    onVolumeChange?: (gain: number) => void | Promise<void>;
    onToggleMute?: () => void;
    onClose: () => void;
  }

  let {
    song,
    isPlaying,
    isPaused,
    isLoading,
    hasPrevious,
    hasNext,
    progress,
    duration,
    isShuffled,
    repeatMode,
    lyricsLoading,
    lyricsError,
    lyricsLines,
    activeLyricIndex,
    reducedMotion,
    onPrevious,
    onTogglePlay,
    onSeek,
    onNext,
    onShuffleChange,
    onRepeatModeChange,
    onDownload,
    downloadState,
    downloadDisabled,
    volume = 1,
    onVolumeChange,
    onToggleMute,
    onClose,
  }: Props = $props();

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

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function formatTime(seconds: number): string {
    if (!isFinite(seconds) || isNaN(seconds) || seconds < 0) return '0:00';
    const minute = Math.floor(seconds / 60);
    const second = Math.floor(seconds % 60);
    return `${minute}:${second.toString().padStart(2, '0')}`;
  }

  function nextRepeatMode(mode: RepeatMode): RepeatMode {
    return mode === 'all' ? 'one' : 'all';
  }

  function readRangeValue(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  function dur(base: number): number {
    return reducedMotion ? 0 : base;
  }

  function dockTransition(
    _node: Element,
    { duration = 380 }: { duration?: number } = {}
  ) {
    return {
      duration,
      easing: cubicOut,
      css: (t: number) => {
        const y = (1 - t) * 60;
        const s = 0.92 + t * 0.08;
        return `opacity: ${t}; transform: translateY(${y}px) scale(${s})`;
      },
    };
  }

  const safeDuration = $derived(duration > 0 ? duration : 1);
  const shownProgress = $derived(seekPreview ?? progress);
  const progressRatio = $derived(clamp(shownProgress / safeDuration, 0, 1));
  const canSeek = $derived(duration > 0 && !isLoading);

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      close: m.player_fullscreen_close(),
      noLyrics: m.player_fullscreen_no_lyrics(),
      lyricsLoading: m.player_lyrics_loading(),
      unknownArtist: m.player_unknown_artist(),
      repeatOne: m.player_repeat_one(),
      repeatAll: m.player_repeat_all(),
    };
  });

  const artistText = $derived(
    song.artists.length ? song.artists.join(' · ') : labels.unknownArtist
  );
  const repeatLabel = $derived(
    repeatMode === 'one' ? labels.repeatOne : labels.repeatAll
  );

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
    !isLoading && downloadState === 'idle' && !downloadDisabled
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
        const dataUrl = await getImageDataUrl(coverUrl);
        if (seq !== coverRequestSeq) return;
        resolvedCoverUrl = dataUrl;
      } catch {
        if (seq !== coverRequestSeq) return;
        resolvedCoverUrl = null;
      }
    })();
  });

  $effect(() => {
    if (activeLyricIndex < 0 || !lyricsListRef) return;
    const el = lyricsListRef.children[activeLyricIndex] as
      | HTMLElement
      | undefined;
    el?.scrollIntoView({
      block: 'center',
      behavior: reducedMotion ? 'instant' : 'smooth',
    });
  });

  $effect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key === 'ArrowUp' && onVolumeChange) {
        event.preventDefault();
        void onVolumeChange(Math.min(1, volume + 0.05));
        return;
      }
      if (event.key === 'ArrowDown' && onVolumeChange) {
        event.preventDefault();
        void onVolumeChange(Math.max(0, volume - 0.05));
        return;
      }
      if ((event.key === 'm' || event.key === 'M') && onToggleMute) {
        event.preventDefault();
        onToggleMute();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  });

  $effect(() => {
    if (
      !draggingSeek &&
      seekPreview !== null &&
      Math.abs(seekPreview - progress) < 0.25
    ) {
      seekPreview = null;
    }
  });

  $effect(() => {
    const observer = new ResizeObserver(() => {
      if (titleRef) {
        titleOverflows = titleRef.scrollWidth > titleRef.clientWidth;
      }
      if (artistRef) {
        artistOverflows = artistRef.scrollWidth > artistRef.clientWidth;
      }
    });

    if (titleRef) observer.observe(titleRef);
    if (artistRef) observer.observe(artistRef);

    return () => observer.disconnect();
  });

  function handleSeekInput(event: Event) {
    if (!canSeek) return;
    draggingSeek = true;
    seekPreview = clamp(readRangeValue(event), 0, duration || 0);
  }

  async function handleSeekChange(event: Event) {
    draggingSeek = false;
    if (!canSeek) {
      seekPreview = null;
      return;
    }
    const target = clamp(readRangeValue(event), 0, duration);
    seekPreview = target;
    try {
      await onSeek(target);
    } catch {
      seekPreview = null;
    }
  }
</script>

<div
  class="fullscreen-player"
  role="dialog"
  aria-modal="true"
  aria-label={song.name}
  tabindex="-1"
  bind:this={dialogEl}
  transition:dockTransition={{ duration: dur(380) }}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
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
    aria-label={labels.close}
    onclick={onClose}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M18 6 6 18"></path>
      <path d="m6 6 12 12"></path>
    </svg>
  </button>

  <div class="fullscreen-left">
    <div
      class="fullscreen-cover-wrap"
      transition:scale={{ start: 0.85, duration: dur(300) }}
    >
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
      <div class="fullscreen-meta-text">
        <h2
          class="fullscreen-title"
          class:overflowing={titleOverflows}
          bind:this={titleRef}
        >
          <span class="fullscreen-title-inner">{song.name}</span>
        </h2>
        <p
          class="fullscreen-artist"
          class:overflowing={artistOverflows}
          bind:this={artistRef}
        >
          <span class="fullscreen-artist-inner">{artistText}</span>
        </p>
      </div>
      <button
        type="button"
        class="fs-btn fs-download"
        class:download-active={downloadState !== 'idle'}
        aria-label={downloadButtonLabel}
        title={downloadButtonLabel}
        disabled={!canDownload}
        onclick={() => onDownload()}
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
        <span>{formatTime(duration)}</span>
      </div>
    </div>
    <div class="fullscreen-controls">
      <button
        type="button"
        class="fs-btn"
        aria-label={m.player_aria_shuffle()}
        aria-pressed={isShuffled}
        disabled={isLoading}
        onclick={() => onShuffleChange(!isShuffled)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 7h2.2c1.5 0 2.8.6 3.8 1.6L19 16.6"></path>
          <path d="m16.2 16.6 2.8.1-.1-2.8"></path>
          <path d="M5 17h2.2c1.5 0 2.8-.6 3.8-1.6l2-2"></path>
          <path d="m16.2 7.4 2.8-.1-.1 2.8"></path>
        </svg>
      </button>

      <button
        type="button"
        class="fs-btn"
        aria-label={m.player_aria_previous()}
        disabled={!hasPrevious || isLoading}
        onclick={() => onPrevious()}
      >
        <svg class="fs-solid" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="4.75" y="6.15" width="1.95" height="11.7" rx="0.75"></rect>
          <path d="M18.6 6.9v10.2L11.75 12z"></path>
          <path d="M12.2 6.9v10.2L5.35 12z"></path>
        </svg>
      </button>

      <button
        type="button"
        class="fs-btn fs-play"
        class:playing={isPlaying}
        aria-label={isPlaying
          ? m.player_aria_pause()
          : isPaused
            ? m.player_aria_resume()
            : m.player_aria_play()}
        disabled={isLoading}
        onclick={() => onTogglePlay()}
      >
        {#if isPlaying}
          <svg class="fs-solid" viewBox="0 0 24 24" aria-hidden="true">
            <rect x="7.15" y="5.95" width="3.4" height="12.1" rx="1.25"></rect>
            <rect x="13.45" y="5.95" width="3.4" height="12.1" rx="1.25"></rect>
          </svg>
        {:else}
          <svg class="fs-solid" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M8.2 6.3v11.4L17.35 12z"></path>
          </svg>
        {/if}
      </button>

      <button
        type="button"
        class="fs-btn"
        aria-label={m.player_aria_next()}
        disabled={!hasNext || isLoading}
        onclick={() => onNext()}
      >
        <svg class="fs-solid" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="17.3" y="6.15" width="1.95" height="11.7" rx="0.75"></rect>
          <path d="M5.4 6.9v10.2L12.25 12z"></path>
          <path d="M11.8 6.9v10.2L18.65 12z"></path>
        </svg>
      </button>

      <button
        type="button"
        class="fs-btn"
        aria-label={m.player_aria_repeat_toggle({ mode: repeatLabel })}
        aria-pressed={repeatMode === 'one'}
        disabled={isLoading}
        onclick={() => onRepeatModeChange(nextRepeatMode(repeatMode))}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 8h10.8"></path>
          <path d="m13.3 5.4 2.7 2.6-2.7 2.6"></path>
          <path d="M19 16H8.2"></path>
          <path d="m10.7 18.6-2.7-2.6 2.7-2.6"></path>
          {#if repeatMode === 'one'}
            <circle
              cx="12"
              cy="12"
              r="3.15"
              fill="rgba(255,255,255,0.12)"
              stroke="currentColor"
            ></circle>
            <path d="M12 10.3v3.4"></path>
            <path d="m11.4 10.9.6-.6"></path>
          {/if}
        </svg>
      </button>
    </div>
  </div>

  <div class="fullscreen-right">
    {#if lyricsLoading}
      <div class="fullscreen-lyrics-empty">{labels.lyricsLoading}</div>
    {:else if lyricsError}
      <div class="fullscreen-lyrics-empty">{lyricsError}</div>
    {:else if lyricsLines.length > 0}
      <div class="fullscreen-lyrics" bind:this={lyricsListRef}>
        {#each lyricsLines as line, index (line.id)}
          <p
            class={`fullscreen-lyric-line${index === activeLyricIndex ? ' active' : ''}`}
          >
            {line.text}
          </p>
        {/each}
      </div>
    {:else}
      <div class="fullscreen-lyrics-empty">{labels.noLyrics}</div>
    {/if}
  </div>
</div>
