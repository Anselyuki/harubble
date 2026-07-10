<script lang="ts">
  import { getImageDataUrl } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import {
    formatTime,
    formatSampleRate,
    formatBitDepth,
    formatBitrate,
    formatChannels,
    formatPlaybackCore,
    formatPlaybackEndpoint,
    normalizeSampleFormat,
  } from '$lib/features/player/formatUtils';
  import LyricsBubble from '$lib/components/app/player/LyricsBubble.svelte';
  import VolumeCapsule from '$lib/components/app/player/VolumeCapsule.svelte';
  import type { LyricLine } from '$lib/features/player/lyrics';
  import type { PlaybackFormatState } from '$lib/types';
  import {
    gsap,
    getMotionDuration,
    killTweens,
    MOTION,
  } from '$lib/design/gsap';
  type RepeatMode = 'all' | 'one';
  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';
  interface Song {
    cid: string;
    name: string;
    artists: string[];
    coverUrl: string | null;
  }
  interface Props {
    song: Song | null;
    isPlaying: boolean;
    isPaused: boolean;
    hasPrevious: boolean;
    hasNext: boolean;
    progress: number;
    duration: number;
    isLoading?: boolean;
    isPlayTogglePending?: boolean;
    reducedMotion?: boolean;
    isShuffled?: boolean;
    repeatMode?: RepeatMode;
    lyricsActive?: boolean;
    lyricsUnavailable?: boolean;
    lyricsLoading?: boolean;
    lyricsError?: string;
    lyricsLines?: LyricLine[];
    activeLyricIndex?: number;
    playlistActive?: boolean;
    downloadState?: SongDownloadState;
    downloadDisabled?: boolean;
    volume?: number;
    muted?: boolean;
    playbackFormat?: PlaybackFormatState | null;
    onVolumeChange?: (gain: number) => void | Promise<void>;
    onToggleMute?: () => void;
    onPrevious?: () => void;
    onTogglePlay?: () => void;
    onSeek?: (positionSecs: number) => void | Promise<void>;
    onNext?: () => void;
    onShuffleChange?: (next: boolean) => void | Promise<void>;
    onRepeatModeChange?: (next: RepeatMode) => void | Promise<void>;
    onToggleLyrics?: () => void;
    onTogglePlaylist?: () => void;
    onToggleFullscreen?: () => void;
    onDownload?: () => void | Promise<void>;
  }
  let {
    song,
    isPlaying,
    isPaused,
    hasPrevious,
    hasNext,
    progress,
    duration,
    isLoading = false,
    isPlayTogglePending = false,
    reducedMotion = false,
    isShuffled = false,
    repeatMode = 'all',
    lyricsActive = false,
    lyricsUnavailable = false,
    lyricsLoading = false,
    lyricsError = '',
    lyricsLines = [],
    activeLyricIndex = -1,
    playlistActive = false,
    downloadState = 'idle',
    downloadDisabled = false,
    volume = 1,
    muted = false,
    playbackFormat = null,
    onVolumeChange,
    onToggleMute,
    onPrevious,
    onTogglePlay,
    onSeek,
    onNext,
    onShuffleChange,
    onRepeatModeChange,
    onToggleLyrics,
    onTogglePlaylist,
    onToggleFullscreen,
    onDownload,
  }: Props = $props();
  let seekPreview = $state<number | null>(null);
  let draggingSeek = $state(false);
  let activeCid: string | null = null;
  let activeCoverUrl: string | null = null;
  let resolvedCoverUrl = $state<string | null>(null);
  let coverRequestSeq = 0;
  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }
  function nextRepeatMode(mode: RepeatMode): RepeatMode {
    return mode === 'all' ? 'one' : 'all';
  }
  function readRangeValue(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }
  const canSeek = $derived.by(
    () => !!song && duration > 0 && !isLoading && !!onSeek
  );
  const canShuffle = $derived.by(
    () => !!song && !isLoading && !!onShuffleChange
  );
  const canRepeat = $derived.by(
    () => !!song && !isLoading && !!onRepeatModeChange
  );
  const shownProgress = $derived.by(() =>
    seekPreview === null ? progress : seekPreview
  );
  const safeDuration = $derived.by(() => (duration > 0 ? duration : 1));
  const remainingProgress = $derived.by(() =>
    Math.max(duration - shownProgress, 0)
  );
  const progressRatio = $derived.by(() =>
    clamp(shownProgress / safeDuration, 0, 1)
  );
  const compactFormatLabel = $derived.by(() => {
    if (!playbackFormat) return '';
    return formatPlaybackCore(
      playbackFormat.sourceSampleRate,
      playbackFormat.sourceBitsPerSample
    );
  });
  const sourceFormatLabel = $derived.by(() => {
    if (!playbackFormat) return '';
    return formatPlaybackEndpoint(
      playbackFormat.sourceSampleRate,
      playbackFormat.sourceBitsPerSample,
      playbackFormat.sourceChannels,
      playbackFormat.sourceBitrateKbps
    );
  });
  const outputFormatLabel = $derived.by(() => {
    if (!playbackFormat) return '';
    return `${formatPlaybackEndpoint(
      playbackFormat.outputSampleRate,
      playbackFormat.outputBitsPerSample,
      playbackFormat.outputChannels
    )} ${normalizeSampleFormat(playbackFormat.outputSampleFormat)}`;
  });
  const playbackFormatTitle = $derived.by(() => {
    if (!playbackFormat) return '';
    const operations = [
      playbackFormat.resampling ? 'resample' : null,
      playbackFormat.channelRemix ? 'remix' : null,
    ].filter(Boolean);
    const suffix = operations.length ? ` (${operations.join(', ')})` : '';
    return `Input ${sourceFormatLabel} -> output ${outputFormatLabel}${suffix}`;
  });
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      unknownArtist: m.player_unknown_artist(),
      statusLoading: m.player_status_loading(),
      statusPaused: m.player_status_paused(),
      repeatOne: m.player_repeat_one(),
      repeatAll: m.player_repeat_all(),
      lyricsClose: m.player_lyrics_close(),
      lyricsOpen: m.player_lyrics_open(),
      playlistClose: m.player_playlist_close(),
      playlistOpen: m.player_playlist_open(),
      downloadIdle: m.player_download_idle(),
      ariaControls: m.player_aria_controls(),
      ariaTimeline: m.player_aria_timeline(),
      ariaSeek: m.player_aria_seek(),
      ariaTransport: m.player_aria_transport(),
      ariaShuffle: m.player_aria_shuffle(),
      ariaPrevious: m.player_aria_previous(),
      ariaNext: m.player_aria_next(),
      ariaPause: m.player_aria_pause(),
      ariaResume: m.player_aria_resume(),
      ariaPlay: m.player_aria_play(),
      ariaExtras: m.player_aria_extras(),
    };
  });
  const artistText = $derived.by(() =>
    song?.artists.length ? song.artists.join(' · ') : labels.unknownArtist
  );
  const subtitle = $derived.by(() =>
    isLoading
      ? `${artistText} · ${labels.statusLoading}`
      : isPaused
        ? `${artistText} · ${labels.statusPaused}`
        : artistText
  );
  const repeatLabel = $derived.by(() =>
    repeatMode === 'one' ? labels.repeatOne : labels.repeatAll
  );
  const playerState = $derived.by(() =>
    isLoading || isPlayTogglePending
      ? 'loading'
      : isPlaying
        ? 'playing'
        : isPaused
          ? 'paused'
          : 'idle'
  );
  const playButtonLoading = $derived(isLoading || isPlayTogglePending);
  const detailPanel = $derived.by(() =>
    lyricsActive ? 'lyrics' : playlistActive ? 'playlist' : 'none'
  );
  const lyricsButtonLabel = $derived.by(() => {
    if (lyricsUnavailable) return m.player_lyrics_unavailable();
    return lyricsActive ? labels.lyricsClose : labels.lyricsOpen;
  });
  const playlistButtonLabel = $derived.by(() =>
    playlistActive ? labels.playlistClose : labels.playlistOpen
  );
  const lyricsToggleDisabled = $derived(
    !song || isLoading || !onToggleLyrics || lyricsUnavailable
  );
  const playlistToggleDisabled = $derived(
    !song || isLoading || !onTogglePlaylist
  );
  const downloadButtonLabel = $derived.by(() => {
    if (!song) return labels.downloadIdle;
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
  const canDownload = $derived.by(
    () =>
      !!song &&
      !isLoading &&
      !!onDownload &&
      downloadState === 'idle' &&
      !downloadDisabled
  );
  let capsuleOpen = $state(false);
  let rightControlsRef = $state<HTMLElement | null>(null);
  let formatPopoverOpen = $state(false);
  let formatPopoverVisible = $state(false);
  let formatPopoverClosing = $state(false);
  let formatPopoverEl: HTMLDivElement | undefined = $state();
  let formatReadoutShellRef = $state<HTMLElement | null>(null);
  const remainingLabel = $derived.by(() =>
    duration > 0 ? `-${formatTime(remainingProgress)}` : '0:00'
  );
  const playerStyle = $derived.by(
    () =>
      `--motion-duration:${reducedMotion ? '0ms' : 'var(--motion-base)'};--player-progress-percent:${progressRatio * 100}%`
  );
  $effect(() => {
    const currentCid = song?.cid ?? null;
    if (currentCid !== activeCid) {
      activeCid = currentCid;
      seekPreview = null;
      draggingSeek = false;
    }
  });
  $effect(() => {
    const coverUrl = song?.coverUrl ?? null;
    if (coverUrl === activeCoverUrl) return;
    activeCoverUrl = coverUrl;
    const requestSeq = ++coverRequestSeq;
    if (!coverUrl) {
      resolvedCoverUrl = null;
      return;
    }
    void (async () => {
      try {
        const dataUrl = await getImageDataUrl(coverUrl);
        if (requestSeq !== coverRequestSeq) return;
        resolvedCoverUrl = dataUrl;
      } catch {
        if (requestSeq !== coverRequestSeq) return;
        resolvedCoverUrl = null;
      }
    })();
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
  async function commitSeek(nextValue: number) {
    draggingSeek = false;
    if (!canSeek) {
      seekPreview = null;
      return;
    }
    const target = clamp(nextValue, 0, duration);
    seekPreview = target;
    if (Math.abs(target - progress) < 0.05) {
      seekPreview = null;
      return;
    }
    try {
      await onSeek?.(target);
    } catch {
      seekPreview = null;
    }
  }
  function handleSeekInput(event: Event) {
    if (!canSeek) return;
    draggingSeek = true;
    seekPreview = clamp(readRangeValue(event), 0, duration || 0);
  }
  function handleSeekChange(event: Event) {
    void commitSeek(readRangeValue(event));
  }
  async function handleShuffleToggle() {
    if (!canShuffle) return;
    try {
      await onShuffleChange?.(!isShuffled);
    } catch {
      return;
    }
  }
  async function handleRepeatToggle() {
    if (!canRepeat) return;
    const next = nextRepeatMode(repeatMode);
    try {
      await onRepeatModeChange?.(next);
    } catch {
      return;
    }
  }

  let playIconPlayRef = $state<SVGElement | null>(null);
  let playIconPauseRef = $state<SVGElement | null>(null);
  let coverExpandHintRef = $state<HTMLElement | null>(null);
  let coverExpandTriggerRef = $state<HTMLElement | null>(null);

  function gsapStatefulIcon(node: SVGElement) {
    const badge = node.querySelector<SVGElement>('.toggle-badge');
    const mark = node.querySelector<SVGElement>('.toggle-mark');
    if (!badge || !mark) return {};

    const button = node.closest('button');
    if (!button) return {};

    const applyState = (pressed: boolean, animate: boolean) => {
      const dur = animate ? getMotionDuration(MOTION.BASE) : 0;
      if (pressed) {
        killTweens(badge);
        killTweens(mark);
        gsap.to(badge, { scale: 1, opacity: 1, duration: dur, ease: 'ios' });
        gsap.to(mark, { scale: 1, opacity: 1, duration: dur, ease: 'ios' });
      } else {
        killTweens(badge);
        killTweens(mark);
        gsap.to(badge, { scale: 0.72, opacity: 0, duration: dur, ease: 'ios' });
        gsap.to(mark, { scale: 0.72, opacity: 0, duration: dur, ease: 'ios' });
      }
    };

    const isPressed = () => button.getAttribute('aria-pressed') === 'true';
    applyState(isPressed(), false);

    const observer = new MutationObserver(() => applyState(isPressed(), true));
    observer.observe(button, {
      attributes: true,
      attributeFilter: ['aria-pressed'],
    });

    return {
      destroy() {
        observer.disconnect();
        killTweens(badge);
        killTweens(mark);
      },
    };
  }

  $effect(() => {
    const playEl = playIconPlayRef;
    const pauseEl = playIconPauseRef;
    if (!playEl || !pauseEl) return;
    killTweens(playEl);
    killTweens(pauseEl);
    if (playButtonLoading) {
      gsap.to(playEl, {
        x: 0.5,
        scale: 0.82,
        opacity: 0,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios',
      });
      gsap.to(pauseEl, {
        scale: 0.82,
        opacity: 0,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios',
      });
    } else if (isPlaying) {
      gsap.to(playEl, {
        x: 0.5,
        scale: 0.82,
        opacity: 0,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios',
      });
      gsap.to(pauseEl, {
        scale: 1,
        opacity: 1,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios',
      });
    } else {
      gsap.to(playEl, {
        x: 0.5,
        scale: 1,
        opacity: 1,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios',
      });
      gsap.to(pauseEl, {
        scale: 0.82,
        opacity: 0,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios',
      });
    }
  });

  function gsapFormatReadout(node: HTMLElement) {
    const readout = node.querySelector<HTMLElement>('.format-readout');
    const compact = node.querySelector<HTMLElement>('.format-pill-compact');
    const details = node.querySelector<HTMLElement>('.format-details');
    if (!readout || !compact || !details) return {};

    const collapsedWidth = 88;
    const maxExpandedWidth = 286;
    let hoverOpen = false;
    let focusOpen = false;
    let expandedWidth = maxExpandedWidth;
    let frame = 0;

    const measureExpandedWidth = () => {
      const prevShellWidth = node.style.width;
      const prevCompact = compact.style.display;
      const prevDetails = details.style.display;
      node.style.width = 'auto';
      compact.style.display = 'none';
      details.style.display = '';
      const shellStyle = getComputedStyle(node);
      const shellBorder =
        Number.parseFloat(shellStyle.borderLeftWidth) +
        Number.parseFloat(shellStyle.borderRightWidth);
      expandedWidth = clamp(
        Math.ceil(readout.scrollWidth + shellBorder),
        collapsedWidth,
        maxExpandedWidth
      );
      node.style.width = prevShellWidth;
      compact.style.display = prevCompact;
      details.style.display = prevDetails;
    };

    const applyState = (open: boolean, animate: boolean) => {
      measureExpandedWidth();
      killTweens(node);
      killTweens(compact);
      killTweens(details);
      const duration = animate ? getMotionDuration(MOTION.BASE) : 0;

      if (open) {
        compact.style.display = 'none';
        details.style.display = '';
        gsap.fromTo(
          details,
          { opacity: 0 },
          { opacity: 1, duration, ease: 'ios-out' }
        );
      } else {
        details.style.display = 'none';
        compact.style.display = '';
        gsap.fromTo(
          compact,
          { opacity: 0 },
          { opacity: 1, duration, ease: 'ios-out' }
        );
      }

      gsap.to(node, {
        width: open ? expandedWidth : collapsedWidth,
        duration,
        ease: open ? 'ios-spring' : 'ios',
      });
    };

    const updateState = (animate = true) => {
      applyState(hoverOpen || focusOpen || formatPopoverOpen, animate);
    };
    const handleEnter = () => {
      hoverOpen = true;
      if (!formatPopoverOpen) updateState(true);
    };
    const handleLeave = () => {
      hoverOpen = false;
      if (!formatPopoverOpen) updateState(true);
    };
    const handleFocusIn = () => {
      focusOpen = true;
      if (!formatPopoverOpen) updateState(true);
    };
    const handleFocusOut = () => {
      focusOpen = false;
      if (!formatPopoverOpen) updateState(true);
    };

    const handlePopoverChange = () => {
      updateState(true);
    };

    gsap.set(node, { width: collapsedWidth });
    details.style.display = 'none';
    frame = requestAnimationFrame(() => updateState(false));

    node.addEventListener('pointerenter', handleEnter);
    node.addEventListener('pointerleave', handleLeave);
    node.addEventListener('focusin', handleFocusIn);
    node.addEventListener('focusout', handleFocusOut);
    node.addEventListener('format-popover-change', handlePopoverChange);

    return {
      destroy() {
        cancelAnimationFrame(frame);
        node.removeEventListener('pointerenter', handleEnter);
        node.removeEventListener('pointerleave', handleLeave);
        node.removeEventListener('focusin', handleFocusIn);
        node.removeEventListener('focusout', handleFocusOut);
        node.removeEventListener('format-popover-change', handlePopoverChange);
        killTweens(node);
        killTweens(compact);
        killTweens(details);
      },
    };
  }

  $effect(() => {
    void formatPopoverOpen;
    if (!formatReadoutShellRef) return;
    formatReadoutShellRef.dispatchEvent(new Event('format-popover-change'));
  });

  function handleFormatReadoutClick() {
    if (formatPopoverClosing) return;
    if (formatPopoverOpen) {
      closeFormatPopover();
    } else {
      formatPopoverOpen = true;
      formatPopoverVisible = true;
    }
  }

  function closeFormatPopover() {
    if (!formatPopoverEl || formatPopoverClosing) {
      formatPopoverOpen = false;
      formatPopoverVisible = false;
      return;
    }
    formatPopoverClosing = true;
    killTweens(formatPopoverEl);
    gsap.to(formatPopoverEl, {
      opacity: 0,
      y: 6,
      scale: 0.96,
      duration: getMotionDuration(MOTION.FAST),
      ease: 'ios-in',
      onComplete: () => {
        formatPopoverOpen = false;
        formatPopoverVisible = false;
        formatPopoverClosing = false;
      },
    });
  }

  $effect(() => {
    if (!formatPopoverEl) return;
    killTweens(formatPopoverEl);
    gsap.fromTo(
      formatPopoverEl,
      { opacity: 0, y: 6, scale: 0.96 },
      {
        opacity: 1,
        y: 0,
        scale: 1,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios-out',
      }
    );
  });

  $effect(() => {
    if (!formatPopoverOpen) return;
    function handleClickOutside(e: PointerEvent) {
      if (
        formatPopoverEl?.contains(e.target as Node) ||
        formatReadoutShellRef?.contains(e.target as Node)
      ) {
        return;
      }
      closeFormatPopover();
    }
    function handleEscape(e: KeyboardEvent) {
      if (e.key === 'Escape') closeFormatPopover();
    }
    document.addEventListener('pointerdown', handleClickOutside);
    document.addEventListener('keydown', handleEscape);
    return () => {
      document.removeEventListener('pointerdown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  });

  $effect(() => {
    const hint = coverExpandHintRef;
    const trigger = coverExpandTriggerRef;
    if (!hint || !trigger) return;
    const handleEnter = () => {
      killTweens(hint);
      gsap.to(hint, {
        opacity: 1,
        duration: getMotionDuration(MOTION.FAST),
        ease: 'ios',
      });
    };
    const handleLeave = () => {
      killTweens(hint);
      gsap.to(hint, {
        opacity: 0,
        duration: getMotionDuration(MOTION.FAST),
        ease: 'ios',
      });
    };
    trigger.addEventListener('mouseenter', handleEnter);
    trigger.addEventListener('mouseleave', handleLeave);
    return () => {
      trigger.removeEventListener('mouseenter', handleEnter);
      trigger.removeEventListener('mouseleave', handleLeave);
      killTweens(hint);
    };
  });
</script>

{#if song}
  <section
    class="am-player"
    aria-label={labels.ariaControls}
    style={playerStyle}
    data-loading={isLoading || isPlayTogglePending ? 'true' : 'false'}
    data-state={playerState}
    data-panel={detailPanel}
    data-dragging={draggingSeek ? 'true' : 'false'}
  >
    <div class="timeline" role="group" aria-label={labels.ariaTimeline}>
      <div class="progress-track">
        <div class="track-bg" aria-hidden="true"></div>
        <input
          class="seek-slider"
          type="range"
          min="0"
          max={safeDuration}
          value={shownProgress}
          step="0.1"
          aria-label={labels.ariaSeek}
          disabled={!canSeek}
          oninput={handleSeekInput}
          onchange={handleSeekChange}
        />
      </div>
    </div>

    <div class="left-controls" role="group" aria-label={labels.ariaTransport}>
      <button
        type="button"
        class="icon-button side-toggle"
        aria-label={labels.ariaShuffle}
        aria-pressed={isShuffled}
        disabled={!canShuffle}
        onclick={handleShuffleToggle}
      >
        <svg class="control-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 7h2.2c1.5 0 2.8.6 3.8 1.6L19 16.6"></path>
          <path d="m16.2 16.6 2.8.1-.1-2.8"></path>
          <path d="M5 17h2.2c1.5 0 2.8-.6 3.8-1.6l2-2"></path>
          <path d="m16.2 7.4 2.8-.1-.1 2.8"></path>
        </svg>
      </button>

      <div class="transport-cluster">
        <button
          type="button"
          class="icon-button transport-button"
          aria-label={labels.ariaPrevious}
          disabled={!hasPrevious || isLoading}
          onclick={() => onPrevious?.()}
        >
          <svg
            class="control-icon solid-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <rect x="4.75" y="6.15" width="1.95" height="11.7" rx="0.75"></rect>
            <path d="M18.6 6.9v10.2L11.75 12z"></path>
            <path d="M12.2 6.9v10.2L5.35 12z"></path>
          </svg>
        </button>

        <button
          type="button"
          class="icon-button play-button"
          class:playing={isPlaying}
          aria-label={playButtonLoading
            ? labels.statusLoading
            : isPlaying
              ? labels.ariaPause
              : isPaused
                ? labels.ariaResume
                : labels.ariaPlay}
          disabled={playButtonLoading || !onTogglePlay}
          onclick={() => onTogglePlay?.()}
        >
          <span class="play-glyph" aria-hidden="true">
            {#if playButtonLoading}
              <svg
                class="control-icon play-icon play-icon-loading spin-icon"
                viewBox="0 0 24 24"
              >
                <path
                  d="M12 5a7 7 0 1 1-6.3 4"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.2"
                  stroke-linecap="round"
                ></path>
              </svg>
            {/if}
            <svg
              class="control-icon play-icon play-icon-pause"
              viewBox="0 0 24 24"
              bind:this={playIconPauseRef}
            >
              <rect x="7.15" y="5.95" width="3.4" height="12.1" rx="1.25"
              ></rect>
              <rect x="13.45" y="5.95" width="3.4" height="12.1" rx="1.25"
              ></rect>
            </svg>
            <svg
              class="control-icon play-icon play-icon-play"
              viewBox="0 0 24 24"
              bind:this={playIconPlayRef}
            >
              <path d="M8.2 6.3v11.4L17.35 12z"></path>
            </svg>
          </span>
        </button>

        <button
          type="button"
          class="icon-button transport-button"
          aria-label={labels.ariaNext}
          disabled={!hasNext || isLoading}
          onclick={() => onNext?.()}
        >
          <svg
            class="control-icon solid-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <rect x="17.3" y="6.15" width="1.95" height="11.7" rx="0.75"></rect>
            <path d="M5.4 6.9v10.2L12.25 12z"></path>
            <path d="M11.8 6.9v10.2L18.65 12z"></path>
          </svg>
        </button>
      </div>

      <button
        type="button"
        class="icon-button side-toggle"
        aria-label={m.player_aria_repeat_toggle({ mode: repeatLabel })}
        aria-pressed={repeatMode === 'one'}
        disabled={!canRepeat}
        onclick={handleRepeatToggle}
      >
        <svg class="control-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 8h10.8"></path>
          <path d="m13.3 5.4 2.7 2.6-2.7 2.6"></path>
          <path d="M19 16H8.2"></path>
          <path d="m10.7 18.6-2.7-2.6 2.7-2.6"></path>
          {#if repeatMode === 'one'}
            <circle class="repeat-badge" cx="12" cy="12" r="3.15"></circle>
            <path d="M12 10.3v3.4"></path>
            <path d="m11.4 10.9.6-.6"></path>
          {/if}
        </svg>
      </button>
    </div>

    <div class="center-panel">
      <div class="playback-stage">
        <div class="track-info">
          <button
            type="button"
            class="cover-expand-trigger"
            aria-label={m.player_fullscreen_open()}
            disabled={!onToggleFullscreen}
            onclick={() => onToggleFullscreen?.()}
            bind:this={coverExpandTriggerRef}
          >
            {#if resolvedCoverUrl}
              <img
                src={resolvedCoverUrl}
                alt={m.player_cover_alt({ name: song.name })}
                class="cover"
              />
            {:else}
              <div class="cover fallback" aria-hidden="true">
                <svg viewBox="0 0 24 24"
                  ><path d="M12 3v10.5a4 4 0 1 0 2 3.5V7h4V3h-6z" /></svg
                >
              </div>
            {/if}
            <div
              class="cover-expand-hint"
              aria-hidden="true"
              bind:this={coverExpandHintRef}
            >
              <svg viewBox="0 0 24 24">
                <path d="M15 3h6v6"></path>
                <path d="M9 21H3v-6"></path>
                <path d="m21 3-7 7"></path>
                <path d="m3 21 7-7"></path>
              </svg>
            </div>
          </button>

          <div class="meta meta-stage">
            <p class="title">{song.name}</p>
            <p class="artist">{subtitle}</p>
          </div>
        </div>
      </div>
    </div>

    <div
      class="right-controls"
      role="group"
      aria-label={labels.ariaExtras}
      bind:this={rightControlsRef}
    >
      <div
        class="time-readout"
        aria-label={m.player_aria_progress({
          time: formatTime(shownProgress),
          remaining: remainingLabel,
        })}
      >
        <span class="time">{formatTime(shownProgress)}</span>
        <span class="time-separator" aria-hidden="true">/</span>
        <span class="time time-remaining">{remainingLabel}</span>
      </div>

      {#if compactFormatLabel && sourceFormatLabel && outputFormatLabel}
        <div class="format-readout-anchor">
          <div
            class="format-readout-shell"
            use:gsapFormatReadout
            bind:this={formatReadoutShellRef}
          >
            <button
              type="button"
              class="format-readout"
              aria-label={playbackFormatTitle}
              title={playbackFormatTitle}
              aria-expanded={formatPopoverOpen}
              onclick={handleFormatReadoutClick}
            >
              <span class="format-pill format-pill-compact">
                <span class="format-swatch" aria-hidden="true"></span>
                <span class="format-text">{compactFormatLabel}</span>
              </span>
              <span class="format-details" aria-hidden="true">
                <span class="format-pill">
                  <span class="format-swatch" aria-hidden="true"></span>
                  <span class="format-text">{sourceFormatLabel}</span>
                </span>
                <span class="format-arrow" aria-hidden="true">-&gt;</span>
                <span class="format-pill format-pill-output">
                  <span
                    class="format-swatch format-swatch-output"
                    aria-hidden="true"
                  ></span>
                  <span class="format-text">{outputFormatLabel}</span>
                </span>
              </span>
            </button>
          </div>
          {#if formatPopoverVisible}
            <div
              class="format-popover-content"
              bind:this={formatPopoverEl}
              role="dialog"
              aria-label={m.player_format_popover_title()}
            >
              <div class="format-popover-header">
                {m.player_format_popover_title()}
              </div>
              <div class="format-popover-section-divider">
                <span class="format-popover-section-badge"
                  >{m.player_format_section_source()}</span
                >
              </div>
              <div class="format-popover-rows">
                <div class="format-popover-row">
                  <span class="format-popover-label"
                    >{m.player_format_label_sample_rate()}</span
                  >
                  <span class="format-popover-value"
                    >{formatSampleRate(playbackFormat!.sourceSampleRate)}</span
                  >
                </div>
                <div class="format-popover-row">
                  <span class="format-popover-label"
                    >{m.player_format_label_bit_depth()}</span
                  >
                  <span class="format-popover-value"
                    >{formatBitDepth(playbackFormat!.sourceBitsPerSample)}</span
                  >
                </div>
                <div class="format-popover-row">
                  <span class="format-popover-label"
                    >{m.player_format_label_channels()}</span
                  >
                  <span class="format-popover-value"
                    >{formatChannels(playbackFormat!.sourceChannels)}</span
                  >
                </div>
                {#if playbackFormat!.sourceBitrateKbps}
                  <div class="format-popover-row">
                    <span class="format-popover-label"
                      >{m.player_format_label_bitrate()}</span
                    >
                    <span class="format-popover-value"
                      >{formatBitrate(playbackFormat!.sourceBitrateKbps)}</span
                    >
                  </div>
                {/if}
              </div>
              <div class="format-popover-section-divider">
                <span class="format-popover-section-badge"
                  >{m.player_format_section_output()}</span
                >
              </div>
              <div class="format-popover-rows">
                <div class="format-popover-row">
                  <span class="format-popover-label"
                    >{m.player_format_label_sample_rate()}</span
                  >
                  <span class="format-popover-value"
                    >{formatSampleRate(playbackFormat!.outputSampleRate)}</span
                  >
                </div>
                <div class="format-popover-row">
                  <span class="format-popover-label"
                    >{m.player_format_label_bit_depth()}</span
                  >
                  <span class="format-popover-value"
                    >{formatBitDepth(playbackFormat!.outputBitsPerSample)}</span
                  >
                </div>
                <div class="format-popover-row">
                  <span class="format-popover-label"
                    >{m.player_format_label_channels()}</span
                  >
                  <span class="format-popover-value"
                    >{formatChannels(playbackFormat!.outputChannels)}</span
                  >
                </div>
                <div class="format-popover-row">
                  <span class="format-popover-label"
                    >{m.player_format_label_sample_format()}</span
                  >
                  <span class="format-popover-value"
                    >{normalizeSampleFormat(
                      playbackFormat!.outputSampleFormat
                    )}</span
                  >
                </div>
              </div>
              {#if playbackFormat!.resampling || playbackFormat!.channelRemix}
                <div class="format-popover-section-divider">
                  <span class="format-popover-section-badge-accent"
                    >{m.player_format_section_processing()}</span
                  >
                </div>
                <div class="format-popover-rows">
                  {#if playbackFormat!.resampling}
                    <div class="format-popover-row">
                      <span class="format-popover-label"
                        >{m.player_format_label_resampling()}</span
                      >
                      <span class="format-popover-value format-popover-flag"
                        >{m.player_format_flag_yes()}</span
                      >
                    </div>
                  {/if}
                  {#if playbackFormat!.channelRemix}
                    <div class="format-popover-row">
                      <span class="format-popover-label"
                        >{m.player_format_label_channel_remix()}</span
                      >
                      <span class="format-popover-value format-popover-flag"
                        >{m.player_format_flag_yes()}</span
                      >
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <div class="lyrics-toggle-anchor">
        <button
          type="button"
          class="icon-button panel-toggle"
          class:panel-active={lyricsActive}
          class:lyrics-unavailable={lyricsUnavailable}
          aria-label={lyricsButtonLabel}
          aria-pressed={lyricsActive}
          title={lyricsUnavailable ? lyricsButtonLabel : undefined}
          disabled={lyricsToggleDisabled}
          onclick={() => onToggleLyrics?.()}
        >
          <svg
            class="control-icon stateful-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
            use:gsapStatefulIcon
          >
            <path d="M5.5 7.25h13"></path>
            <path d="M5.5 11h13"></path>
            <path d="M5.5 14.75h9.5"></path>
            <path d="M5.5 18.5h6.25"></path>
            {#if lyricsUnavailable}
              <line class="lyrics-slash" x1="4" y1="4" x2="20" y2="20"></line>
            {:else}
              <circle class="toggle-badge" cx="18" cy="6" r="3.1"></circle>
              <path class="toggle-mark" d="m16.55 5.25 1.45 1.45 1.45-1.45"
              ></path>
            {/if}
          </svg>
        </button>

        {#if lyricsActive && song}
          <LyricsBubble
            loading={lyricsLoading}
            error={lyricsError}
            lines={lyricsLines}
            {activeLyricIndex}
            songName={song.name}
            {reducedMotion}
            onClose={() => onToggleLyrics?.()}
          />
        {/if}
      </div>

      <button
        type="button"
        class="icon-button panel-toggle"
        class:panel-active={playlistActive}
        aria-label={playlistButtonLabel}
        aria-pressed={playlistActive}
        disabled={playlistToggleDisabled}
        onclick={() => onTogglePlaylist?.()}
      >
        <svg
          class="control-icon stateful-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
          use:gsapStatefulIcon
        >
          <path d="M5.25 7h9.5"></path>
          <path d="M5.25 11.5h9.5"></path>
          <path d="M5.25 16h6.75"></path>
          <path d="M16.6 10.25 20 12.25l-3.4 2z"></path>
          <circle class="toggle-badge" cx="18" cy="6" r="3.1"></circle>
          <path class="toggle-mark" d="m16.55 5.25 1.45 1.45 1.45-1.45"></path>
        </svg>
      </button>

      <button
        type="button"
        class="icon-button"
        class:download-active={downloadState !== 'idle'}
        aria-label={downloadButtonLabel}
        title={downloadButtonLabel}
        disabled={!canDownload}
        onclick={() => onDownload?.()}
      >
        {#if downloadState === 'creating'}
          <svg
            class="control-icon spin-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path d="M21 12a9 9 0 1 1-2.64-6.36"></path>
            <path d="M21 3v6h-6"></path>
          </svg>
        {:else}
          <svg class="control-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 5v9"></path>
            <path d="m8.5 10.5 3.5 3.5 3.5-3.5"></path>
            <path d="M5 18h14"></path>
            {#if downloadState === 'queued'}
              <path d="M8 4.5h8"></path>
            {/if}
          </svg>
        {/if}
      </button>

      <div
        class="volume-group"
        role="group"
        aria-label={m.player_aria_volume()}
      >
        <VolumeCapsule
          {volume}
          {muted}
          open={capsuleOpen}
          onopen={() => (capsuleOpen = true)}
          onclose={() => (capsuleOpen = false)}
          {onVolumeChange}
          {onToggleMute}
        />
      </div>
    </div>
  </section>
{/if}

<style>
  .am-player {
    --surface: var(--player-shell-bg);
    --surface-border: var(--player-shell-border);
    --surface-highlight: var(--player-shell-highlight);
    --text-main: var(--player-title);
    --text-subtle: var(--player-subtitle);
    --icon-default: var(--player-control-color);
    --icon-active: var(--album-accent);
    --track-bg: var(--player-track-bg);
    --track-fill-end: var(--player-track-fill-end);
    --thumb-border: var(--player-thumb-border);
    --thumb-bg: var(--player-thumb-bg);
    --thumb-shadow: var(--player-thumb-shadow);
    --time-color: var(--player-time);
    --play-text: var(--player-play-text);
    --play-shadow: var(--player-play-shadow);
    --play-shadow-hover: var(--player-play-shadow-hover);
    --group-bg: color-mix(in srgb, var(--surface) 76%, transparent);
    --group-border: color-mix(in srgb, var(--surface-border) 84%, transparent);
    --control-button-size: 34px;
    --control-icon-size: 19px;
    --play-icon-size: 21px;
    --control-icon-stroke: 1.85;
    --seek-track-size: 4px;
    --transport-width: 172px;
    width: min(700px, calc(100vw - 20px));
    min-width: 0;
    min-height: 76px;
    margin: 0 auto;
    border-radius: 0;
    border: 0;
    background: transparent;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    box-shadow: none;
    display: grid;
    grid-template-columns: var(--transport-width) minmax(0, 1fr) auto;
    gap: 2px;
    align-items: center;
    padding: 11px 10px 8px 8px;
  }

  .am-player[data-panel='lyrics'],
  .am-player[data-panel='playlist'] {
    box-shadow:
      0 18px 36px rgba(15, 23, 42, 0.14),
      0 8px 20px rgba(var(--album-accent-rgb), 0.1),
      inset 0 1px 0
        color-mix(in srgb, var(--surface-highlight) 90%, transparent);
  }

  .left-controls {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 0;
    width: var(--transport-width);
    min-width: 0;
    flex-shrink: 0;
  }

  .transport-cluster {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 0;
  }

  .right-controls {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0;
    padding: 0;
    flex-shrink: 0;
  }

  .center-panel {
    position: relative;
    z-index: 2;
    min-width: 0;
    display: flex;
    align-items: center;
    padding: 0;
  }

  .track-info {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .playback-stage {
    min-width: 0;
    width: 100%;
    justify-self: start;
    display: grid;
    gap: 0;
  }

  .cover-expand-trigger {
    position: relative;
    appearance: none;
    border: 0;
    padding: 0;
    background: transparent;
    cursor: pointer;
    flex-shrink: 0;
    border-radius: 11px;
    overflow: hidden;
  }

  .cover-expand-trigger:disabled {
    cursor: default;
  }

  .cover-expand-hint {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.4);
    opacity: 0;
    border-radius: inherit;
  }

  .cover-expand-hint svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: #fff;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .cover {
    width: 46px;
    height: 46px;
    flex-shrink: 0;
    border-radius: 11px;
    object-fit: cover;
    box-shadow:
      0 12px 24px rgba(16, 18, 28, 0.18),
      0 0 0 1px rgba(255, 255, 255, 0.18);
  }

  .am-player[data-state='playing'] .cover {
    box-shadow:
      0 14px 28px rgba(16, 18, 28, 0.22),
      0 0 0 1px rgba(var(--album-accent-rgb), 0.12);
  }

  .fallback {
    display: grid;
    place-items: center;
    background: linear-gradient(
      145deg,
      var(--player-cover-start),
      var(--player-cover-end)
    );
    color: var(--player-placeholder-color);
  }

  .fallback svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
  }

  .meta {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .meta-stage {
    position: relative;
    flex: 1 1 auto;
    min-width: 0;
    padding: 0 2px;
    min-height: 31px;
    align-content: start;
    isolation: isolate;
    overflow: hidden;
    border-radius: 10px;
  }

  .title,
  .artist {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: 14px;
    line-height: 1.18;
    color: var(--text-main);
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .artist {
    font-size: 11.5px;
    line-height: 1.2;
    color: var(--text-subtle);
    opacity: 1;
  }

  .timeline {
    --timeline-hit-size: 14px;
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 5;
    min-width: 0;
    height: var(--timeline-hit-size);
  }

  .time {
    min-width: 0;
    font-size: 10.5px;
    font-weight: 600;
    color: color-mix(in srgb, var(--text-main) 68%, var(--text-subtle));
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .time-remaining {
    text-align: right;
  }

  .time-readout {
    min-width: 86px;
    padding: 0 8px 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    color: var(--time-color);
    flex-shrink: 0;
  }

  .time-readout .time {
    color: currentColor;
  }

  .time-separator {
    font-size: 10px;
    font-weight: 600;
    color: color-mix(in srgb, currentColor 58%, transparent);
    line-height: 1;
  }

  .progress-track {
    position: relative;
    height: var(--timeline-hit-size);
    display: flex;
    align-items: flex-start;
    min-width: 0;
  }

  .track-bg {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: var(--seek-track-size);
    border-radius: 0;
    background: linear-gradient(
      90deg,
      var(--album-accent) 0,
      var(--album-accent-hover) var(--player-progress-percent),
      rgba(120, 120, 128, 0.28) var(--player-progress-percent),
      rgba(120, 120, 128, 0.28) 100%
    );
    overflow: hidden;
  }

  .seek-slider {
    appearance: none;
    -webkit-appearance: none;
    width: 100%;
    margin: 0;
    background: transparent;
    height: var(--timeline-hit-size);
    position: relative;
    z-index: 2;
    cursor: pointer;
  }

  .seek-slider::-webkit-slider-runnable-track {
    height: var(--seek-track-size);
    background: transparent;
    border-radius: 0;
  }

  .seek-slider:disabled {
    cursor: not-allowed;
  }

  .seek-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 0;
    height: 0;
    margin-top: 0;
    border-radius: 0;
    border: 0;
    background: transparent;
    box-shadow: none;
    opacity: 0;
  }

  .seek-slider::-moz-range-track {
    height: var(--seek-track-size);
    background: transparent;
    border: 0;
    border-radius: 0;
  }

  .seek-slider::-moz-range-progress {
    background: transparent;
    border: 0;
  }

  .seek-slider::-moz-range-thumb {
    width: 0;
    height: 0;
    border-radius: 0;
    border: 0;
    background: transparent;
    box-shadow: none;
    opacity: 0;
  }

  .icon-button {
    position: relative;
    width: var(--control-button-size);
    height: var(--control-button-size);
    border-radius: 50%;
    border: 1px solid transparent;
    display: inline-grid;
    place-items: center;
    cursor: pointer;
    color: var(--icon-default);
    background: transparent;
  }

  .icon-button::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.2), transparent);
    opacity: 0;
    pointer-events: none;
  }

  .control-icon {
    width: var(--control-icon-size);
    height: var(--control-icon-size);
    fill: none;
    stroke: currentColor;
    stroke-width: var(--control-icon-stroke);
    stroke-linecap: round;
    stroke-linejoin: round;
    flex-shrink: 0;
  }

  .control-icon.solid-icon {
    fill: currentColor;
    stroke: none;
  }

  .control-icon .repeat-badge {
    fill: color-mix(in srgb, currentColor 12%, transparent);
    stroke: currentColor;
  }

  .stateful-icon .toggle-badge,
  .stateful-icon .toggle-mark {
    transform-origin: 18px 6px;
  }

  .stateful-icon .toggle-badge {
    fill: rgba(var(--album-accent-rgb), 0.12);
    stroke: rgba(var(--album-accent-rgb), 0.24);
    opacity: 0;
    transform: scale(0.72);
  }

  .stateful-icon .toggle-mark {
    opacity: 0;
    transform: scale(0.72);
    stroke-width: 2.15;
  }

  .icon-button:hover:not(:disabled),
  .icon-button[aria-pressed='true'] {
    background: rgba(var(--album-accent-rgb), 0.08);
    color: var(--icon-active);
    border-color: rgba(var(--album-accent-rgb), 0.08);
    box-shadow: none;
  }

  .icon-button:hover:not(:disabled)::before,
  .icon-button[aria-pressed='true']::before {
    opacity: 1;
  }

  .panel-toggle.panel-active {
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.2),
      0 8px 18px rgba(var(--album-accent-rgb), 0.12);
  }

  .format-readout-anchor {
    position: relative;
    display: inline-flex;
    align-items: center;
    flex: 0 0 auto;
  }

  .format-readout-shell {
    width: 88px;
    height: 28px;
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    overflow: hidden;
  }

  .format-readout {
    appearance: none;
    min-width: 0;
    height: 28px;
    width: max-content;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 5px;
    border: 0;
    color: var(--text-secondary);
    font-size: 10px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    background: transparent;
    cursor: default;
  }

  .format-readout:focus-visible {
    outline: 2px solid rgba(128, 128, 128, 0.35);
    outline-offset: 2px;
  }

  .format-details {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 5px;
    pointer-events: none;
  }

  .format-pill {
    min-width: 0;
    height: 20px;
    padding: 0 7px 0 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border: 1px solid rgba(128, 128, 128, 0.15);
    border-radius: 999px;
    background: rgba(128, 128, 128, 0.08);
    overflow: hidden;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.16);
  }

  .format-pill-compact {
    opacity: 1;
  }

  .format-pill-output {
    background: rgba(128, 128, 128, 0.06);
  }

  .format-swatch {
    width: 7px;
    height: 7px;
    flex: 0 0 auto;
    border-radius: 999px;
    background: rgba(128, 128, 128, 0.7);
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.38),
      0 0 8px rgba(128, 128, 128, 0.3);
  }

  .format-swatch-output {
    background: rgba(128, 128, 128, 0.55);
  }

  .format-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .format-arrow {
    flex: 0 0 auto;
    color: rgba(128, 128, 128, 0.5);
    font-size: 9px;
    line-height: 1;
  }

  .format-popover-content {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    z-index: var(--z-popover, 200);
    width: 280px;
    max-width: calc(100vw - 32px);
    max-height: 60vh;
    overflow-y: auto;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--bg-primary);
    border: 1px solid rgba(128, 128, 128, 0.14);
    box-shadow:
      0 8px 32px rgba(15, 23, 42, 0.12),
      0 2px 8px rgba(15, 23, 42, 0.06);
    transform-origin: bottom right;
    opacity: 0;
  }

  .format-popover-header {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid rgba(128, 128, 128, 0.1);
  }

  .format-popover-rows {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .format-popover-section-divider {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0 6px;
  }

  .format-popover-section-divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: rgba(128, 128, 128, 0.1);
  }

  .format-popover-section-badge,
  .format-popover-section-badge-accent {
    font-size: 10px;
    font-weight: 600;
    line-height: 1;
    padding: 3px 8px;
    border-radius: 9999px;
    flex-shrink: 0;
  }

  .format-popover-section-badge {
    background: rgba(128, 128, 128, 0.12);
    color: var(--text-secondary);
  }

  .format-popover-section-badge-accent {
    background: rgba(128, 128, 128, 0.08);
    color: var(--text-secondary);
  }

  .format-popover-row {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }

  .format-popover-label {
    flex-shrink: 0;
    min-width: 56px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .format-popover-value {
    font-size: 11px;
    color: var(--text-primary);
    font-family: var(--font-mono);
    line-height: 1.4;
  }

  .format-popover-flag {
    color: rgba(128, 128, 128, 0.7);
    font-weight: 600;
  }

  .lyrics-toggle-anchor {
    position: relative;
  }

  .lyrics-unavailable {
    opacity: 0.5;
  }

  .lyrics-slash {
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
  }

  .icon-button.download-active {
    background: var(--player-control-hover-bg);
    color: var(--icon-active);
    border-color: rgba(var(--album-accent-rgb), 0.14);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.18);
  }

  .icon-button.download-active::before {
    opacity: 1;
  }

  .spin-icon {
    animation: motion-spin var(--motion-spinner) linear infinite;
  }

  .icon-button:active:not(:disabled) {
    transform: scale(0.96);
  }

  .play-button {
    color: var(--icon-default);
  }

  .play-button.playing {
    color: var(--icon-active);
  }

  .play-glyph {
    position: relative;
    width: var(--play-icon-size);
    height: var(--play-icon-size);
    display: grid;
    place-items: center;
  }

  .play-icon {
    position: absolute;
    inset: 0;
    width: var(--play-icon-size);
    height: var(--play-icon-size);
    fill: currentColor;
    stroke: none;
  }

  .play-icon-play {
    transform: translateX(0.5px) scale(1);
    opacity: 1;
  }

  .play-icon-pause {
    transform: scale(0.82);
    opacity: 0;
  }

  .play-icon-loading {
    fill: none;
    stroke: currentColor;
    opacity: 1;
  }

  .icon-button:focus-visible,
  .seek-slider:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--surface-highlight) 86%, white 14%),
      0 0 0 4px rgba(var(--album-accent-rgb), 0.28);
    border-radius: 999px;
  }

  .icon-button:disabled,
  .seek-slider:disabled {
    opacity: 0.42;
  }

  .icon-button:disabled {
    cursor: not-allowed;
    box-shadow: none;
  }

  @media (max-width: 900px) {
    .am-player {
      grid-template-columns: 1fr;
      gap: 8px;
      padding: 11px 10px 7px;
    }

    .left-controls,
    .right-controls {
      justify-content: center;
      flex-wrap: wrap;
    }

    .center-panel {
      order: -1;
      display: flex;
    }

    .playback-stage {
      width: 100%;
    }

    .left-controls {
      width: auto;
    }
  }

  @media (max-width: 640px) {
    .am-player {
      --control-button-size: 32px;
      --control-icon-size: 18px;
      --play-icon-size: 20px;
      width: calc(100vw - 12px);
      min-height: 72px;
      padding: 8px 10px;
      gap: 8px;
    }

    .left-controls {
      gap: 0;
    }

    .track-info {
      gap: 6px;
    }

    .transport-cluster {
      gap: 0;
    }

    .right-controls {
      gap: 0;
    }

    .time-readout {
      min-width: 76px;
      padding: 0 4px 0 0;
    }

    .cover {
      width: 40px;
      height: 40px;
      border-radius: 10px;
    }

    .title {
      font-size: 13px;
    }

    .artist,
    .time {
      font-size: 10.5px;
    }
  }

  @media (hover: none) {
    .playback-stage {
      width: 100%;
      gap: 1px;
    }
  }

  .volume-group {
    position: relative;
    display: flex;
    align-items: center;
    width: var(--control-button-size, 34px);
    height: var(--control-button-size, 34px);
    flex-shrink: 0;
    margin-right: 2px;
  }
</style>
