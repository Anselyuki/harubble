<script lang="ts">
  import {
    gsap,
    animateIn,
    getMotionDuration,
    killTweens,
    gsapScrollIntoView,
    MOTION,
  } from '$lib/design/gsap';
  import { getImageDataUrl } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { getPlayerContext, getDownloadContext } from '$lib/contexts';
  import { formatTime } from '$lib/features/player/formatUtils';

  const player = getPlayerContext();
  const download = getDownloadContext();

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

  function handleDownload() {
    if (player.currentSong) {
      void download.handleSongDownload(player.currentSong.cid);
    }
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

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function nextRepeatMode(mode: 'all' | 'one'): 'all' | 'one' {
    return mode === 'all' ? 'one' : 'all';
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
      repeatOne: m.player_repeat_one(),
      repeatAll: m.player_repeat_all(),
    };
  });

  const artistText = $derived(
    song.artists.length ? song.artists.join(' · ') : labels.unknownArtist
  );
  const repeatLabel = $derived(
    player.repeatMode === 'one' ? labels.repeatOne : labels.repeatAll
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
  role="dialog"
  aria-modal="true"
  aria-label={song.name}
  tabindex="-1"
  bind:this={dialogEl}
  onkeydown={(e) => e.key === 'Escape' && player.toggleFullscreen()}
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
    onclick={player.toggleFullscreen}
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
        class="fs-btn"
        aria-label={m.player_aria_shuffle()}
        aria-pressed={player.shuffleEnabled}
        disabled={player.isLoading}
        onclick={() => player.toggleShuffle(!player.shuffleEnabled)}
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
        disabled={!player.hasPrevious || player.isLoading}
        onclick={() => player.playPrevious()}
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
        class:playing={player.isPlaying}
        aria-label={player.isPlaying
          ? m.player_aria_pause()
          : player.isPaused
            ? m.player_aria_resume()
            : m.player_aria_play()}
        disabled={playButtonLoading}
        onclick={() => onTogglePlay()}
      >
        {#if playButtonLoading}
          <svg class="fs-spin" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M12 5a7 7 0 1 1-6.3 4"
              fill="none"
              stroke="currentColor"
              stroke-width="2.2"
              stroke-linecap="round"
            ></path>
          </svg>
        {:else if player.isPlaying}
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
        disabled={!player.hasNext || player.isLoading}
        onclick={() => player.playNext()}
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
        aria-pressed={player.repeatMode === 'one'}
        disabled={player.isLoading}
        onclick={() => player.toggleRepeat(nextRepeatMode(player.repeatMode))}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 8h10.8"></path>
          <path d="m13.3 5.4 2.7 2.6-2.7 2.6"></path>
          <path d="M19 16H8.2"></path>
          <path d="m10.7 18.6-2.7-2.6 2.7-2.6"></path>
          {#if player.repeatMode === 'one'}
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
              onclick={() => handleLyricSeek(line.time!)}
            >
              {line.text}
            </button>
          {:else}
            <p
              class="fullscreen-lyric-line"
              class:active={index === player.activeLyricIndex}
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
