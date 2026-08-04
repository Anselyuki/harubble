<script lang="ts">
  import { flushSync } from 'svelte';
  import { getImageSrc } from '$lib/api';
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
  import { getNextRepeatMode } from '$lib/features/player/repeatMode';
  import LyricsBubble from '$lib/components/app/player/LyricsBubble.svelte';
  import PlayToggleGlyph from '$lib/components/app/player/PlayToggleGlyph.svelte';
  import VolumeCapsule from '$lib/components/app/player/VolumeCapsule.svelte';
  import PlayerTimeline from '$lib/components/app/player/PlayerTimeline.svelte';
  import { createCollapseTimer } from '$lib/components/app/player/volume-capsule-timer';
  import type { LyricLine } from '$lib/features/player/lyrics';
  import type { PlaybackFormatState, RepeatMode } from '$lib/types';
  import { Repeat, Repeat1, Shuffle } from '@lucide/svelte';
  import {
    gsap,
    getMotionDuration,
    killTweens,
    MOTION,
    shouldSkipMotion,
  } from '$lib/design/gsap';
  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';
  const FORMAT_POPOVER_CLOSE_DELAY_MS = 799;
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
    repeatMode = 'off',
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
  const sourceFormatExtraLabel = $derived.by(() => {
    if (!playbackFormat) return '';
    return [
      formatChannels(playbackFormat.sourceChannels),
      formatBitrate(playbackFormat.sourceBitrateKbps),
    ]
      .filter(Boolean)
      .join('/');
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
      repeatOff: m.player_repeat_off(),
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
  const repeatLabel = $derived.by(() => {
    if (repeatMode === 'one') return labels.repeatOne;
    if (repeatMode === 'all') return labels.repeatAll;
    return labels.repeatOff;
  });
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
  const formatPopoverCloseTimer = createCollapseTimer(
    FORMAT_POPOVER_CLOSE_DELAY_MS,
    () => {
      const activeElement = document.activeElement;
      const interactionActive =
        document.hasFocus() &&
        (formatReadoutShellRef?.matches(':hover') ||
          formatPopoverEl?.matches(':hover') ||
          (activeElement &&
            (formatReadoutShellRef?.contains(activeElement) ||
              formatPopoverEl?.contains(activeElement))));
      if (!interactionActive) closeFormatPopover();
    }
  );
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
        const imageSrc = await getImageSrc(coverUrl);
        if (requestSeq !== coverRequestSeq) return;
        resolvedCoverUrl = imageSrc;
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
    const next = getNextRepeatMode(repeatMode);
    try {
      await onRepeatModeChange?.(next);
    } catch {
      return;
    }
  }

  let playToggleTransitionKey = $state(0);
  let coverExpandHintRef = $state<HTMLElement | null>(null);
  let coverExpandTriggerRef = $state<HTMLElement | null>(null);

  function handlePlayToggle() {
    if (playButtonLoading || !onTogglePlay) return;
    playToggleTransitionKey += 1;
    flushSync();
    onTogglePlay();
  }

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

  function gsapFormatReadout(node: HTMLElement) {
    const readout = node.querySelector<HTMLElement>('.format-readout');
    const details = node.querySelector<HTMLElement>('.format-details');
    const sourcePill = node.querySelector<HTMLElement>('.format-pill-source');
    const sourceCore = node.querySelector<HTMLElement>(
      '.format-pill-source-core'
    );
    const sourceExtraClip = node.querySelector<HTMLElement>(
      '.format-source-extra-clip'
    );
    if (
      !readout ||
      !details ||
      !sourcePill ||
      !sourceCore ||
      !sourceExtraClip
    ) {
      return {};
    }

    const minimumCollapsedWidth = 88;
    const maxExpandedWidth = 286;
    let hoverOpen = false;
    let focusOpen = false;
    let collapsedWidth = minimumCollapsedWidth;
    let expandedWidth = maxExpandedWidth;
    let sourceCollapsedWidth = collapsedWidth;
    let sourceExpandedWidth = collapsedWidth;
    let sourceExtraWidth = 0;
    let detailsWidth = 0;
    let frame = 0;
    let remeasureFrame = 0;
    const narrowDisclosureQuery = window.matchMedia('(max-width: 360px)');

    const measureExpandedWidth = () => {
      const previousDetailsWidth = details.style.width;
      const previousSourceWidth = sourcePill.style.width;
      const previousSourceExtraWidth = sourceExtraClip.style.width;
      const previousReadoutWidth = readout.style.width;
      sourcePill.style.width = 'max-content';
      sourceExtraClip.style.width = 'max-content';
      details.style.width = 'max-content';
      readout.style.width = 'max-content';
      const sourceStyle = getComputedStyle(sourcePill);
      const readoutStyle = getComputedStyle(readout);
      const sourceHorizontalChrome =
        Number.parseFloat(sourceStyle.paddingLeft) +
        Number.parseFloat(sourceStyle.paddingRight) +
        Number.parseFloat(sourceStyle.borderLeftWidth) +
        Number.parseFloat(sourceStyle.borderRightWidth);
      sourceCollapsedWidth = Math.ceil(
        sourceCore.getBoundingClientRect().width + sourceHorizontalChrome
      );
      collapsedWidth = Math.max(
        minimumCollapsedWidth,
        Math.ceil(
          sourceCollapsedWidth +
            Number.parseFloat(readoutStyle.paddingLeft) +
            Number.parseFloat(readoutStyle.paddingRight)
        )
      );
      const fullSourceExpandedWidth = Math.ceil(
        sourcePill.getBoundingClientRect().width
      );
      const fullSourceExtraWidth = Math.ceil(
        sourceExtraClip.getBoundingClientRect().width
      );
      detailsWidth = Math.ceil(details.getBoundingClientRect().width);
      const shellStyle = getComputedStyle(node);
      const shellBorder =
        Number.parseFloat(shellStyle.borderLeftWidth) +
        Number.parseFloat(shellStyle.borderRightWidth);
      let measuredExpandedWidth = Math.ceil(readout.scrollWidth + shellBorder);
      sourceExpandedWidth = fullSourceExpandedWidth;
      sourceExtraWidth = fullSourceExtraWidth;

      if (
        measuredExpandedWidth > maxExpandedWidth &&
        fullSourceExtraWidth > 0
      ) {
        // The output endpoint is the comparison target. When the inline path is
        // tight, leave source channel/bitrate detail to the click disclosure.
        sourcePill.style.width = `${sourceCollapsedWidth}px`;
        sourceExtraClip.style.width = '0px';
        sourceExpandedWidth = sourceCollapsedWidth;
        sourceExtraWidth = 0;
        detailsWidth = Math.ceil(details.getBoundingClientRect().width);
        measuredExpandedWidth = Math.ceil(readout.scrollWidth + shellBorder);
      }

      expandedWidth = clamp(
        measuredExpandedWidth,
        collapsedWidth,
        maxExpandedWidth
      );
      node.parentElement?.style.setProperty(
        '--format-readout-expanded-width',
        `${expandedWidth}px`
      );
      sourcePill.style.width = previousSourceWidth;
      sourceExtraClip.style.width = previousSourceExtraWidth;
      details.style.width = previousDetailsWidth;
      readout.style.width = previousReadoutWidth;
    };

    const applyState = (
      open: boolean,
      animate: boolean,
      shouldMeasure = true
    ) => {
      if (shouldMeasure) measureExpandedWidth();
      killTweens(node);
      killTweens(readout);
      killTweens(details);
      killTweens(sourcePill);
      killTweens(sourceExtraClip);
      const duration = animate ? getMotionDuration(MOTION.BASE * 2) : 0;
      const ease = open ? 'ios-spring' : 'ios';
      gsap.to(node, {
        width: open ? expandedWidth : collapsedWidth,
        duration,
        ease,
      });
      gsap.to(sourcePill, {
        width: open ? sourceExpandedWidth : sourceCollapsedWidth,
        duration,
        ease,
      });
      gsap.to(sourceExtraClip, {
        width: open ? sourceExtraWidth : 0,
        duration,
        ease,
      });
      gsap.to(details, { width: open ? detailsWidth : 0, duration, ease });
    };

    const shouldExpand = () =>
      (!formatPopoverOpen || !narrowDisclosureQuery.matches) &&
      (hoverOpen || focusOpen);
    const updateState = (animate = true) => {
      applyState(shouldExpand(), animate);
    };
    const scheduleRemeasure = () => {
      if (remeasureFrame) return;
      remeasureFrame = requestAnimationFrame(() => {
        remeasureFrame = 0;
        measureExpandedWidth();
        applyState(shouldExpand(), false, false);
      });
    };
    const handleEnter = () => {
      hoverOpen = true;
      formatPopoverCloseTimer.cancel();
      updateState(true);
    };
    const handleLeave = (event: PointerEvent) => {
      hoverOpen = false;
      focusOpen = readout.matches(':focus-visible');
      if (!isFormatPopoverInteractionTarget(event.relatedTarget)) {
        scheduleFormatPopoverClose();
      }
      updateState(true);
    };
    const handleFocusIn = () => {
      focusOpen = readout.matches(':focus-visible');
      formatPopoverCloseTimer.cancel();
      updateState(true);
    };
    const handleFocusOut = (event: FocusEvent) => {
      focusOpen = false;
      if (!isFormatPopoverInteractionTarget(event.relatedTarget)) {
        scheduleFormatPopoverClose();
      }
      updateState(true);
    };
    const handlePopoverClosed = () => {
      updateState(true);
    };
    const handlePopoverOpened = () => {
      updateState(true);
    };
    const handleDisclosureBreakpointChange = () => {
      updateState(false);
    };

    gsap.set(node, { width: collapsedWidth });
    gsap.set(details, { width: 0 });
    gsap.set(sourcePill, { width: collapsedWidth });
    gsap.set(sourceExtraClip, { width: 0 });
    gsap.set(readout, { clearProps: 'width,transform' });
    frame = requestAnimationFrame(() => updateState(false));
    node.addEventListener('pointerenter', handleEnter);
    node.addEventListener('pointerleave', handleLeave);
    node.addEventListener('focusin', handleFocusIn);
    node.addEventListener('focusout', handleFocusOut);
    node.addEventListener('format-popover-opened', handlePopoverOpened);
    node.addEventListener('format-popover-closed', handlePopoverClosed);
    narrowDisclosureQuery.addEventListener(
      'change',
      handleDisclosureBreakpointChange
    );
    const contentObserver = new MutationObserver(scheduleRemeasure);
    contentObserver.observe(readout, {
      childList: true,
      characterData: true,
      subtree: true,
    });
    const contentResizeObserver = new ResizeObserver(scheduleRemeasure);
    contentResizeObserver.observe(sourceCore);
    const themeObserver = new MutationObserver(scheduleRemeasure);
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class', 'data-ark-theme', 'data-ark-depth'],
    });

    return {
      destroy() {
        cancelAnimationFrame(frame);
        cancelAnimationFrame(remeasureFrame);
        contentObserver.disconnect();
        contentResizeObserver.disconnect();
        themeObserver.disconnect();
        node.removeEventListener('pointerenter', handleEnter);
        node.removeEventListener('pointerleave', handleLeave);
        node.removeEventListener('focusin', handleFocusIn);
        node.removeEventListener('focusout', handleFocusOut);
        node.removeEventListener('format-popover-opened', handlePopoverOpened);
        node.removeEventListener('format-popover-closed', handlePopoverClosed);
        narrowDisclosureQuery.removeEventListener(
          'change',
          handleDisclosureBreakpointChange
        );
        killTweens(node);
        killTweens(readout);
        killTweens(details);
        killTweens(sourcePill);
        killTweens(sourceExtraClip);
      },
    };
  }

  function handleFormatReadoutClick() {
    if (formatPopoverClosing) return;
    if (formatPopoverOpen) {
      closeFormatPopover();
    } else {
      formatPopoverOpen = true;
      formatPopoverVisible = true;
      formatReadoutShellRef?.dispatchEvent(new Event('format-popover-opened'));
    }
  }

  function isFormatPopoverInteractionTarget(target: EventTarget | null) {
    return (
      target instanceof Node &&
      (formatReadoutShellRef?.contains(target) ||
        formatPopoverEl?.contains(target))
    );
  }

  function scheduleFormatPopoverClose() {
    if (formatPopoverOpen && !formatPopoverClosing) {
      formatPopoverCloseTimer.schedule();
    }
  }

  function handleFormatPopoverPointerLeave(event: PointerEvent) {
    if (isFormatPopoverInteractionTarget(event.relatedTarget)) {
      formatPopoverCloseTimer.cancel();
      return;
    }
    scheduleFormatPopoverClose();
  }

  function finishFormatPopoverClose() {
    formatPopoverCloseTimer.cancel();
    formatPopoverOpen = false;
    formatPopoverVisible = false;
    formatPopoverClosing = false;
    formatReadoutShellRef?.dispatchEvent(new Event('format-popover-closed'));
  }

  function closeFormatPopover() {
    formatPopoverCloseTimer.cancel();
    if (formatPopoverClosing) return;
    if (!formatPopoverEl) {
      finishFormatPopoverClose();
      return;
    }
    if (shouldSkipMotion()) {
      finishFormatPopoverClose();
      return;
    }
    formatPopoverClosing = true;
    const body = formatPopoverEl.querySelector<HTMLElement>(
      '.format-popover-body'
    );
    killTweens(formatPopoverEl);
    if (body) killTweens(body);
    const timeline = gsap.timeline({
      onComplete: finishFormatPopoverClose,
    });
    gsap.set(formatPopoverEl, { clipPath: 'inset(0% 0 0 0)' });
    timeline.to(
      formatPopoverEl,
      {
        opacity: 0,
        y: 6,
        clipPath: 'inset(100% 0 0 0)',
        duration: getMotionDuration(MOTION.BASE_OUT),
        ease: 'ios-in',
      },
      0
    );
    if (body) {
      timeline.to(
        body,
        {
          opacity: 0,
          y: 4,
          duration: getMotionDuration(MOTION.MICRO),
          ease: 'ios-in',
        },
        0
      );
    }
  }

  $effect(() => {
    if (!formatPopoverEl) return;
    const popover = formatPopoverEl;
    const body = popover.querySelector<HTMLElement>('.format-popover-body');
    const positionPopover = () => {
      if (!formatReadoutShellRef) return;
      const anchorRect = formatReadoutShellRef.getBoundingClientRect();
      const viewportMargin = 12;
      const popoverWidth = popover.offsetWidth;
      const preferredRight = anchorRect.right + 8;
      const left = clamp(
        preferredRight - popoverWidth,
        viewportMargin,
        window.innerWidth - viewportMargin - popoverWidth
      );
      popover.style.right = `${anchorRect.right - left - popoverWidth}px`;
    };
    positionPopover();
    killTweens(popover);
    if (body) killTweens(body);

    if (shouldSkipMotion()) {
      gsap.set(popover, {
        clearProps: 'height,scaleX',
        clipPath: 'none',
        opacity: 1,
        x: 0,
        y: 0,
      });
      if (body) gsap.set(body, { opacity: 1, y: 0 });
      const anchorObserver = new ResizeObserver(positionPopover);
      if (formatReadoutShellRef) anchorObserver.observe(formatReadoutShellRef);
      window.addEventListener('resize', positionPopover);
      return () => {
        anchorObserver.disconnect();
        window.removeEventListener('resize', positionPopover);
      };
    }

    gsap.set(popover, {
      clearProps: 'height,scaleX',
      clipPath: 'inset(100% 0 0 0)',
      opacity: 0,
      x: 0,
      y: 6,
    });
    if (body) gsap.set(body, { opacity: 0, y: 4 });
    const timeline = gsap.timeline({ onUpdate: positionPopover });
    timeline.to(popover, {
      clipPath: 'inset(0% 0 0 0)',
      opacity: 1,
      y: 0,
      duration: getMotionDuration(MOTION.BASE),
      ease: 'ios-out',
      onComplete: () => {
        gsap.set(popover, { clearProps: 'clipPath' });
        positionPopover();
      },
    });
    if (body) {
      timeline.to(
        body,
        {
          opacity: 1,
          y: 0,
          duration: getMotionDuration(MOTION.FAST),
          ease: 'ios-out',
        },
        0.04
      );
    }
    const anchorObserver = new ResizeObserver(positionPopover);
    if (formatReadoutShellRef) anchorObserver.observe(formatReadoutShellRef);
    window.addEventListener('resize', positionPopover);
    return () => {
      anchorObserver.disconnect();
      window.removeEventListener('resize', positionPopover);
      timeline.kill();
    };
  });

  $effect(() => {
    return () => formatPopoverCloseTimer.destroy();
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
    if (compactFormatLabel && sourceFormatLabel && outputFormatLabel) return;
    if (!formatPopoverOpen && !formatPopoverVisible && !formatPopoverClosing) {
      return;
    }
    finishFormatPopoverClose();
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
    <PlayerTimeline
      value={shownProgress}
      max={safeDuration}
      disabled={!canSeek}
      groupLabel={labels.ariaTimeline}
      seekLabel={labels.ariaSeek}
      onInput={handleSeekInput}
      onChange={handleSeekChange}
    />

    <div class="left-controls" role="group" aria-label={labels.ariaTransport}>
      <button
        type="button"
        class="icon-button side-toggle"
        aria-label={labels.ariaShuffle}
        aria-pressed={isShuffled}
        disabled={!canShuffle}
        onclick={handleShuffleToggle}
      >
        <Shuffle class="mode-icon" aria-hidden="true" />
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
          aria-busy={playButtonLoading}
          onclick={handlePlayToggle}
        >
          <PlayToggleGlyph
            {isPlaying}
            {isLoading}
            isPending={isPlayTogglePending}
            transitionKey={playToggleTransitionKey}
            {reducedMotion}
            size="calc(var(--play-icon-size) + 12px)"
          />
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
            <path d="M5.4 6.9v10.2L12.25 12z"></path>
            <path d="M11.8 6.9v10.2L18.65 12z"></path>
          </svg>
        </button>
      </div>

      <button
        type="button"
        class="icon-button side-toggle repeat-toggle"
        aria-label={m.player_aria_repeat_toggle({ mode: repeatLabel })}
        aria-pressed={repeatMode !== 'off'}
        disabled={!canRepeat}
        onclick={handleRepeatToggle}
      >
        {#if repeatMode === 'one'}
          <Repeat1 class="mode-icon" aria-hidden="true" />
        {:else}
          <Repeat class="mode-icon" aria-hidden="true" />
        {/if}
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
            <p class="title" data-testid="player-current-song">{song.name}</p>
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
        <div
          class="format-readout-anchor"
          class:format-popover-open={formatPopoverVisible}
        >
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
              aria-controls="player-format-details"
              onclick={handleFormatReadoutClick}
            >
              <span class="format-pill format-pill-source">
                <span class="format-pill-source-core">
                  <span class="format-swatch" aria-hidden="true"></span>
                  <span class="format-text">{compactFormatLabel}</span>
                </span>
                <span class="format-source-extra-clip">
                  <span class="format-source-extra"
                    >/{sourceFormatExtraLabel}</span
                  >
                </span>
              </span>
              <span class="format-details" aria-hidden="true">
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
              id="player-format-details"
              class="format-popover-content"
              bind:this={formatPopoverEl}
              role="region"
              aria-label={m.player_format_popover_title()}
              onpointerenter={() => formatPopoverCloseTimer.cancel()}
              onpointerleave={handleFormatPopoverPointerLeave}
            >
              <div class="format-popover-body">
                <div class="format-popover-header">
                  {m.player_format_popover_title()}
                </div>
                <div class="format-popover-columns">
                  <section
                    class="format-popover-column"
                    aria-label={m.player_format_section_source()}
                  >
                    <div class="format-popover-section-heading">
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
                          >{formatSampleRate(
                            playbackFormat!.sourceSampleRate
                          )}</span
                        >
                      </div>
                      <div class="format-popover-row">
                        <span class="format-popover-label"
                          >{m.player_format_label_bit_depth()}</span
                        >
                        <span class="format-popover-value"
                          >{formatBitDepth(
                            playbackFormat!.sourceBitsPerSample
                          )}</span
                        >
                      </div>
                      <div class="format-popover-row">
                        <span class="format-popover-label"
                          >{m.player_format_label_channels()}</span
                        >
                        <span class="format-popover-value"
                          >{formatChannels(
                            playbackFormat!.sourceChannels
                          )}</span
                        >
                      </div>
                      {#if playbackFormat!.sourceBitrateKbps}
                        <div class="format-popover-row">
                          <span class="format-popover-label"
                            >{m.player_format_label_bitrate()}</span
                          >
                          <span class="format-popover-value"
                            >{formatBitrate(
                              playbackFormat!.sourceBitrateKbps
                            )}</span
                          >
                        </div>
                      {/if}
                    </div>
                  </section>
                  <section
                    class="format-popover-column"
                    aria-label={m.player_format_section_output()}
                  >
                    <div class="format-popover-section-heading">
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
                          >{formatSampleRate(
                            playbackFormat!.outputSampleRate
                          )}</span
                        >
                      </div>
                      <div class="format-popover-row">
                        <span class="format-popover-label"
                          >{m.player_format_label_bit_depth()}</span
                        >
                        <span class="format-popover-value"
                          >{formatBitDepth(
                            playbackFormat!.outputBitsPerSample
                          )}</span
                        >
                      </div>
                      <div class="format-popover-row">
                        <span class="format-popover-label"
                          >{m.player_format_label_channels()}</span
                        >
                        <span class="format-popover-value"
                          >{formatChannels(
                            playbackFormat!.outputChannels
                          )}</span
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
                  </section>
                </div>
                {#if playbackFormat!.resampling || playbackFormat!.channelRemix}
                  <div class="format-popover-processing">
                    <div class="format-popover-section-heading">
                      <span class="format-popover-section-badge-accent"
                        >{m.player_format_section_processing()}</span
                      >
                    </div>
                    <div class="format-popover-processing-rows">
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
                  </div>
                {/if}
              </div>
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
      </div>

      {#if lyricsActive && song}
        <LyricsBubble
          loading={lyricsLoading}
          error={lyricsError}
          lines={lyricsLines}
          {activeLyricIndex}
          {isPlaying}
          {canSeek}
          {reducedMotion}
          onSeek={commitSeek}
          onClose={() => onToggleLyrics?.()}
        />
      {/if}

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
        class:volume-expanded={capsuleOpen}
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

  .transport-button .control-icon {
    width: calc(var(--control-icon-size) + 5px);
    height: calc(var(--control-icon-size) + 5px);
  }

  .side-toggle[aria-pressed='false'] {
    color: color-mix(in srgb, var(--icon-default) 65%, transparent);
  }

  .side-toggle[aria-pressed='false']:hover:not(:disabled) {
    color: var(--icon-default);
    background: rgba(var(--album-accent-rgb), 0.06);
    border-color: rgba(var(--album-accent-rgb), 0.08);
  }

  .side-toggle[aria-pressed='false']:hover:not(:disabled)::before {
    opacity: 1;
  }

  .side-toggle :global(svg.mode-icon) {
    width: 14px;
    height: 14px;
    stroke-width: 2;
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

  .icon-button:hover:not(:disabled):not(.side-toggle):not(
      .transport-button
    ):not(.play-button),
  .icon-button[aria-pressed='true'] {
    background: rgba(var(--album-accent-rgb), 0.08);
    color: var(--icon-active);
    border-color: rgba(var(--album-accent-rgb), 0.08);
    box-shadow: none;
  }

  .icon-button:hover:not(:disabled):not(.side-toggle):not(
      .transport-button
    ):not(.play-button)::before,
  .icon-button[aria-pressed='true']::before {
    opacity: 1;
  }

  .icon-button.transport-button:hover:not(:disabled),
  .icon-button.play-button:hover:not(:disabled) {
    color: var(--icon-active);
    background: rgba(var(--album-accent-rgb), 0.06);
    border-color: rgba(var(--album-accent-rgb), 0.08);
  }

  .icon-button.transport-button:hover:not(:disabled)::before,
  .icon-button.play-button:hover:not(:disabled)::before {
    opacity: 1;
  }

  .icon-button.side-toggle[aria-pressed='true'],
  .icon-button.side-toggle[aria-pressed='true']:hover:not(:disabled) {
    color: var(--icon-active);
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .icon-button.side-toggle[aria-pressed='true']::before,
  .icon-button.side-toggle[aria-pressed='true']:hover:not(:disabled)::before {
    opacity: 0;
  }

  .icon-button.repeat-toggle[aria-pressed='false']:hover:not(:disabled) {
    color: color-mix(in srgb, var(--icon-default) 65%, transparent);
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .icon-button.repeat-toggle[aria-pressed='false']:hover:not(
      :disabled
    )::before {
    opacity: 0;
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

  .format-readout-anchor.format-popover-open {
    z-index: var(--z-popover, 200);
  }

  .format-readout-shell {
    position: relative;
    z-index: 2;
    width: 88px;
    height: 40px;
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    overflow: hidden;
    overflow: clip;
  }

  .format-readout {
    appearance: none;
    flex: 0 0 auto;
    min-width: 0;
    height: 40px;
    width: 100%;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: 5px;
    border: 0;
    color: var(--text-secondary);
    font-size: 10px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    background: transparent;
    cursor: default;
    transform: none;
  }

  .format-readout:focus-visible {
    outline: 2px solid rgba(128, 128, 128, 0.35);
    outline-offset: 2px;
  }

  .format-details {
    width: 0;
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: 5px;
    flex: 0 0 auto;
    overflow: hidden;
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
    border-radius: var(--shape-pill);
    background: rgba(128, 128, 128, 0.08);
    overflow: hidden;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.16);
  }

  .format-pill-source {
    flex: 0 0 auto;
    justify-content: flex-start;
  }

  .format-pill-source-core {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: 0 0 auto;
  }

  .format-source-extra-clip {
    width: 0;
    display: inline-flex;
    flex: 0 0 auto;
    overflow: hidden;
  }

  .format-source-extra {
    flex: 0 0 auto;
    color: inherit;
  }

  .format-pill-output {
    background: rgba(128, 128, 128, 0.06);
  }

  .format-swatch {
    width: 7px;
    height: 7px;
    flex: 0 0 auto;
    border-radius: var(--shape-pill);
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
    color: var(--format-pill-color, var(--text-secondary));
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
  }

  :global(:root[data-ark-theme]) .format-readout-anchor {
    --format-family-signal: var(--theme-accent);
    --format-family-signal-alt: var(
      --theme-custom-signal-alt,
      var(--format-family-signal)
    );
    --format-pill-background: color-mix(
      in srgb,
      var(--bg-primary) 92%,
      transparent
    );
    --format-pill-output-background: color-mix(
      in srgb,
      var(--bg-primary) 86%,
      transparent
    );
    --format-pill-border: 1px solid
      color-mix(in srgb, var(--icon-default) 24%, transparent);
    --format-pill-radius: var(--shape-sm);
    --format-pill-shadow: none;
    --format-pill-color: var(--icon-default, var(--text-secondary));
    --format-pill-font: var(--font-mono);
    --format-pill-weight: 700;
    --format-pill-gap: 4px;
    --format-pill-padding: 0 5px 0 4px;
    --format-readout-padding-inline: 4px;
    --format-marker-width: 7px;
    --format-marker-height: 7px;
    --format-marker-radius: var(--shape-circle);
    --format-marker-background: var(--format-family-signal);
    --format-marker-output-background: var(--format-family-signal-alt);
    --format-marker-border: 0;
    --format-marker-shadow: none;
    --format-marker-clip: none;
    --format-marker-transform: none;
    --format-popover-surface: var(
      --theme-custom-panel,
      color-mix(in srgb, var(--theme-surface) 94%, transparent)
    );
    --format-popover-ink: var(--theme-text-primary, var(--text-primary));
    --format-popover-muted: var(--theme-text-secondary, var(--text-secondary));
    --format-popover-divider: var(--theme-custom-rule, var(--border));
    --format-popover-border: 1px solid var(--format-popover-divider);
    --format-popover-radius: var(--shape-md, 8px);
    --format-popover-shadow: none;
    --format-popover-backdrop: none;
    --format-popover-header-font: var(--font-display);
    --format-popover-header-weight: 700;
    --format-popover-header-marker-width: 8px;
    --format-popover-header-marker-height: 12px;
    --format-popover-header-marker-top: 3px;
    --format-popover-header-marker-background: var(--format-family-signal);
    --format-popover-header-marker-border: 0;
    --format-popover-header-marker-radius: 0;
    --format-popover-header-marker-clip: none;
    --format-popover-header-indent: 14px;
    --format-popover-header-rule-height: 1px;
    --format-popover-header-rule: linear-gradient(
      90deg,
      var(--format-family-signal) 0 24px,
      var(--format-popover-divider) 24px
    );
    --format-popover-badge-background: transparent;
    --format-popover-badge-border: 1px solid var(--format-popover-divider);
    --format-popover-badge-radius: var(--shape-xs, 2px);
    --format-popover-badge-shadow: none;
    --format-popover-badge-color: var(--format-popover-ink);
    --format-popover-badge-font: var(--font-body);
    --format-popover-badge-weight: 700;
    --format-popover-badge-padding: 4px 7px;
    --format-popover-badge-gap: 6px;
    --format-popover-badge-marker-width: 2px;
    --format-popover-badge-marker-height: 8px;
    --format-popover-badge-marker-background: var(--format-family-signal);
    --format-popover-badge-marker-border: 0;
    --format-popover-badge-marker-radius: 0;
    --format-popover-badge-marker-clip: none;
    --format-popover-output-marker-background: var(--format-family-signal);
    --format-popover-processing-background: transparent;
    --format-popover-flag-color: var(--format-popover-ink);
  }

  :global(:root[data-ark-theme]) .format-readout {
    padding-inline: var(--format-readout-padding-inline);
    gap: var(--format-pill-gap);
  }

  :global(:root[data-ark-theme]) .format-pill {
    box-sizing: border-box;
    padding: var(--format-pill-padding);
    gap: var(--format-pill-gap);
    border: var(--format-pill-border);
    border-radius: var(--format-pill-radius);
    background: var(--format-pill-background);
    box-shadow: var(--format-pill-shadow);
    color: var(--format-pill-color);
    font-family: var(--format-pill-font);
    font-weight: var(--format-pill-weight);
    letter-spacing: 0;
  }

  :global(:root[data-ark-theme]) .format-pill-source-core {
    gap: var(--format-pill-gap);
  }

  :global(:root[data-ark-theme]) .format-pill-output {
    background: var(--format-pill-output-background);
  }

  :global(:root[data-ark-theme]) .format-swatch {
    position: relative;
    width: var(--format-marker-width);
    height: var(--format-marker-height);
    border: var(--format-marker-border);
    border-radius: var(--format-marker-radius);
    background: var(--format-marker-background);
    box-shadow: var(--format-marker-shadow);
    clip-path: var(--format-marker-clip);
    transform: var(--format-marker-transform);
  }

  :global(:root[data-ark-theme]) .format-swatch-output {
    background: var(--format-marker-output-background);
    opacity: 0.7;
  }

  :global(:root[data-ark-theme]) .format-readout:focus-visible {
    outline: none;
  }

  :global(:root[data-ark-theme])
    .format-readout-shell:has(.format-readout:focus-visible)::after {
    content: '';
    position: absolute;
    inset: 6px 1px;
    z-index: 3;
    box-sizing: border-box;
    border: 2px solid var(--format-family-signal);
    border-radius: var(--format-pill-radius);
    pointer-events: none;
  }

  :global(:root[data-ark-theme='ark']) .format-readout-anchor {
    --format-pill-background: color-mix(
      in srgb,
      var(--bg-primary) 90%,
      var(--format-family-signal) 10%
    );
    --format-pill-output-background: color-mix(
      in srgb,
      var(--bg-primary) 93%,
      var(--format-family-signal-alt) 7%
    );
    --format-pill-border: 1px solid
      color-mix(in srgb, var(--format-family-signal) 42%, var(--border));
    --format-pill-radius: 0;
    --format-pill-shadow: inset 2px 0 0 var(--format-family-signal);
    --format-marker-width: 2px;
    --format-marker-height: 10px;
    --format-marker-radius: 0;
    --format-marker-shadow: 4px 0 0
      color-mix(in srgb, var(--format-family-signal) 34%, transparent);
    --format-popover-border: 1px solid
      color-mix(
        in srgb,
        var(--format-family-signal) 58%,
        var(--format-popover-divider)
      );
    --format-popover-radius: 0;
    --format-popover-header-font: var(--font-mono);
    --format-popover-header-marker-width: 2px;
    --format-popover-header-marker-height: 14px;
    --format-popover-header-marker-top: 2px;
    --format-popover-header-indent: 10px;
    --format-popover-badge-radius: 0;
    --format-popover-badge-font: var(--font-mono);
  }

  :global(:root[data-ark-theme='endfield']) .format-readout-anchor {
    --format-pill-background: var(
      --ark-field-dock-raised,
      color-mix(in srgb, var(--bg-primary) 88%, transparent)
    );
    --format-pill-output-background: var(
      --ark-field-dock-raised,
      color-mix(in srgb, var(--surface) 88%, transparent)
    );
    --format-pill-border: 1px solid
      color-mix(in srgb, var(--icon-default) 34%, transparent);
    --format-pill-radius: var(--shape-sm, 2px);
    --format-marker-width: 8px;
    --format-marker-height: 12px;
    --format-marker-radius: 0;
    --format-marker-clip: polygon(0 0, 100% 0, 55% 100%, 0 100%);
    --format-popover-radius: var(--shape-sm, 2px);
    --format-popover-shadow: 4px 4px 0
      color-mix(in srgb, var(--format-family-signal) 14%, transparent);
    --format-popover-header-weight: 800;
    --format-popover-header-marker-width: 9px;
    --format-popover-header-marker-height: 14px;
    --format-popover-header-marker-top: 2px;
    --format-popover-header-marker-clip: polygon(0 0, 100% 0, 58% 100%, 0 100%);
    --format-popover-header-indent: 15px;
    --format-popover-header-rule: linear-gradient(
      90deg,
      var(--format-family-signal) 0 40px,
      var(--format-popover-divider) 40px
    );
    --format-popover-badge-background: color-mix(
      in srgb,
      var(--format-family-signal) 7%,
      transparent
    );
    --format-popover-badge-radius: 0;
    --format-popover-badge-marker-width: 7px;
    --format-popover-badge-marker-height: 10px;
    --format-popover-badge-marker-clip: polygon(0 0, 100% 0, 58% 100%, 0 100%);
    --format-popover-output-marker-background: var(
      --theme-custom-signal-alt,
      var(--format-family-signal)
    );
    --format-popover-processing-background: color-mix(
      in srgb,
      var(--format-family-signal) 4%,
      transparent
    );
  }

  :global(:root[data-ark-theme='exa']) .format-readout-anchor {
    --format-pill-background: color-mix(
      in srgb,
      var(--bg-primary) 91%,
      var(--format-family-signal) 9%
    );
    --format-pill-output-background: color-mix(
      in srgb,
      var(--bg-primary) 92%,
      var(--format-family-signal-alt) 8%
    );
    --format-pill-border: 1px solid
      color-mix(in srgb, var(--format-family-signal) 78%, var(--border));
    --format-pill-radius: var(--shape-pill);
    --format-pill-shadow: inset 0 0 0 1px
      color-mix(in srgb, var(--format-family-signal) 18%, transparent);
    --format-marker-width: 10px;
    --format-marker-height: 10px;
    --format-marker-radius: var(--shape-circle);
    --format-marker-background: transparent;
    --format-marker-output-background: transparent;
    --format-marker-border: 2px solid var(--format-family-signal);
    --format-marker-transform: rotate(-24deg);
    --format-popover-border: 1px solid
      color-mix(
        in srgb,
        var(--format-family-signal) 54%,
        var(--format-popover-divider)
      );
    --format-popover-radius: 0 14px 0 14px;
    --format-popover-shadow: 0 12px 32px rgba(8, 9, 20, 0.24);
    --format-popover-backdrop: blur(14px) saturate(1.08);
    --format-popover-header-font: ui-serif, Georgia, Cambria, serif;
    --format-popover-header-weight: 650;
    --format-popover-header-marker-width: 10px;
    --format-popover-header-marker-height: 10px;
    --format-popover-header-marker-top: 4px;
    --format-popover-header-marker-background: transparent;
    --format-popover-header-marker-border: 2px solid var(--format-family-signal);
    --format-popover-header-marker-radius: var(--shape-circle);
    --format-popover-header-indent: 18px;
    --format-popover-header-rule: linear-gradient(
      90deg,
      var(--format-popover-divider),
      color-mix(
        in srgb,
        var(--format-family-signal) 62%,
        var(--format-popover-divider)
      ),
      var(--format-popover-divider)
    );
    --format-popover-badge-border: 1px solid
      color-mix(
        in srgb,
        var(--format-family-signal) 48%,
        var(--format-popover-divider)
      );
    --format-popover-badge-radius: var(--shape-pill);
    --format-popover-badge-marker-width: 6px;
    --format-popover-badge-marker-height: 6px;
    --format-popover-badge-marker-background: transparent;
    --format-popover-badge-marker-border: 1px solid var(--format-family-signal);
    --format-popover-badge-marker-radius: var(--shape-circle);
  }

  :global(:root[data-ark-theme='exa']) .format-swatch::after {
    content: '';
    position: absolute;
    right: -3px;
    top: 0;
    width: 4px;
    height: 4px;
    border-radius: var(--shape-circle);
    background: var(--format-family-signal-alt);
    box-shadow: 0 0 0 1px var(--surface, var(--bg-primary));
  }

  :global(:root[data-ark-theme='popucom']) .format-readout-anchor {
    --format-pill-background: color-mix(
      in srgb,
      var(--bg-primary) 90%,
      var(--format-family-signal) 10%
    );
    --format-pill-output-background: color-mix(
      in srgb,
      var(--bg-primary) 90%,
      var(--format-family-signal-alt) 10%
    );
    --format-pill-border: 2px solid var(--icon-default);
    --format-pill-radius: var(--shape-pill);
    --format-pill-shadow: 2px 2px 0
      var(--theme-custom-action-alt, var(--format-family-signal-alt));
    --format-pill-weight: 750;
    --format-marker-width: 9px;
    --format-marker-height: 9px;
    --format-marker-radius: var(--shape-circle);
    --format-marker-background: var(
      --theme-custom-action-alt,
      var(--format-family-signal)
    );
    --format-marker-border: 2px solid var(--icon-default);
    --format-marker-shadow: 1px 1px 0 var(--icon-default);
    --format-popover-border: 2px solid var(--format-popover-ink);
    --format-popover-radius: 16px;
    --format-popover-shadow: 3px 3px 0
      var(--theme-custom-signal-alt, var(--format-family-signal));
    --format-popover-header-weight: 800;
    --format-popover-header-marker-width: 10px;
    --format-popover-header-marker-height: 10px;
    --format-popover-header-marker-top: 4px;
    --format-popover-header-marker-background: var(
      --theme-custom-action-alt,
      var(--format-family-signal)
    );
    --format-popover-header-marker-border: 2px solid var(--format-popover-ink);
    --format-popover-header-marker-radius: var(--shape-circle);
    --format-popover-header-indent: 17px;
    --format-popover-header-rule-height: 2px;
    --format-popover-badge-background: color-mix(
      in srgb,
      var(--format-family-signal) 18%,
      transparent
    );
    --format-popover-badge-border: 2px solid var(--format-popover-ink);
    --format-popover-badge-radius: var(--shape-pill);
    --format-popover-badge-shadow: 1.5px 1.5px 0
      var(--theme-custom-action-alt, var(--format-family-signal));
    --format-popover-badge-weight: 800;
    --format-popover-badge-marker-width: 6px;
    --format-popover-badge-marker-height: 6px;
    --format-popover-badge-marker-background: var(
      --theme-custom-signal-alt,
      var(--format-family-signal)
    );
    --format-popover-badge-marker-border: 1px solid var(--format-popover-ink);
    --format-popover-badge-marker-radius: var(--shape-circle);
    --format-popover-processing-background: color-mix(
      in srgb,
      var(--theme-custom-action-alt, var(--format-family-signal)) 7%,
      transparent
    );
  }

  :global(:root[data-ark-theme='corporate']) .format-readout-anchor {
    --format-pill-background: color-mix(
      in srgb,
      var(--bg-primary) 94%,
      transparent
    );
    --format-pill-output-background: color-mix(
      in srgb,
      var(--bg-primary) 88%,
      transparent
    );
    --format-pill-border: 1px solid
      color-mix(in srgb, var(--icon-default) 46%, transparent);
    --format-pill-radius: 0;
    --format-marker-width: 10px;
    --format-marker-height: 2px;
    --format-marker-radius: 0;
    --format-popover-radius: 0;
    --format-popover-header-weight: 750;
    --format-popover-header-marker-width: 24px;
    --format-popover-header-marker-height: 2px;
    --format-popover-header-marker-top: 8px;
    --format-popover-header-indent: 32px;
    --format-popover-badge-background: transparent;
    --format-popover-badge-border: 0;
    --format-popover-badge-radius: 0;
    --format-popover-badge-font: var(--font-display);
    --format-popover-badge-padding: 4px 0;
    --format-popover-badge-marker-width: 8px;
    --format-popover-badge-marker-height: 2px;
  }

  .format-popover-content {
    position: absolute;
    bottom: -4px;
    right: -8px;
    z-index: 1;
    box-sizing: border-box;
    width: min(320px, calc(100vw - 24px));
    min-width: 0;
    max-width: calc(100vw - 24px);
    max-height: min(80vh, calc(100vh - 24px));
    overflow-x: auto;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--format-popover-divider, var(--border)) transparent;
    padding: 0;
    color: var(--format-popover-ink, var(--text-primary));
    border: var(--format-popover-border, 1px solid rgba(128, 128, 128, 0.14));
    border-radius: var(--format-popover-radius, 18px);
    background: var(
      --format-popover-surface,
      color-mix(in srgb, var(--bg-primary) 94%, transparent)
    );
    box-shadow: var(--format-popover-shadow, 0 8px 32px rgba(15, 23, 42, 0.12));
    backdrop-filter: var(--format-popover-backdrop, blur(24px) saturate(1.15));
    -webkit-backdrop-filter: var(
      --format-popover-backdrop,
      blur(24px) saturate(1.15)
    );
    transform-origin: bottom right;
    opacity: 0;
    will-change: opacity, transform, clip-path;
  }

  .format-popover-content::-webkit-scrollbar {
    width: 4px;
    height: 4px;
  }

  .format-popover-content::-webkit-scrollbar-thumb {
    border-radius: var(--format-popover-radius, 2px);
    background: var(--format-popover-divider, var(--border));
  }

  .format-popover-body {
    min-height: 0;
    padding: 12px 13px 42px;
  }

  .format-popover-header {
    position: relative;
    display: flex;
    align-items: center;
    min-height: 18px;
    margin-bottom: 10px;
    padding: 0 0 8px var(--format-popover-header-indent, 1px);
    color: var(--format-popover-ink, var(--text-primary));
    font-family: var(--format-popover-header-font, var(--font-body));
    font-size: 13px;
    font-weight: var(--format-popover-header-weight, 650);
    line-height: 1.35;
    letter-spacing: 0;
  }

  .format-popover-header::before {
    content: '';
    position: absolute;
    top: var(--format-popover-header-marker-top, 0);
    left: 0;
    box-sizing: border-box;
    width: var(--format-popover-header-marker-width, 0);
    height: var(--format-popover-header-marker-height, 0);
    border: var(--format-popover-header-marker-border, 0);
    border-radius: var(--format-popover-header-marker-radius, 0);
    background: var(--format-popover-header-marker-background, transparent);
    clip-path: var(--format-popover-header-marker-clip, none);
  }

  .format-popover-header::after {
    content: '';
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: var(--format-popover-header-rule-height, 0);
    background: var(--format-popover-header-rule, transparent);
  }

  .format-popover-columns {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0 12px;
  }

  .format-popover-column {
    min-width: 0;
    padding: 0 10px 1px 0;
  }

  .format-popover-column + .format-popover-column {
    padding: 0 0 1px 11px;
    border-left: 1px solid
      var(--format-popover-divider, rgba(128, 128, 128, 0.12));
  }

  .format-popover-section-heading {
    display: flex;
    align-items: center;
    min-height: 24px;
    margin-bottom: 6px;
  }

  .format-popover-rows {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .format-popover-section-badge,
  .format-popover-section-badge-accent {
    display: inline-flex;
    align-items: center;
    gap: var(--format-popover-badge-gap, 0);
    box-sizing: border-box;
    flex-shrink: 0;
    padding: var(--format-popover-badge-padding, 3px 7px);
    border: var(--format-popover-badge-border, 0);
    border-radius: var(--format-popover-badge-radius, var(--shape-pill));
    background: var(
      --format-popover-badge-background,
      rgba(128, 128, 128, 0.12)
    );
    box-shadow: var(--format-popover-badge-shadow, none);
    color: var(--format-popover-badge-color, var(--text-secondary));
    font-family: var(--format-popover-badge-font, var(--font-body));
    font-size: 11px;
    font-weight: var(--format-popover-badge-weight, 600);
    line-height: 1.15;
    letter-spacing: 0;
  }

  .format-popover-section-badge::before,
  .format-popover-section-badge-accent::before {
    content: '';
    box-sizing: border-box;
    width: var(--format-popover-badge-marker-width, 0);
    height: var(--format-popover-badge-marker-height, 0);
    flex: 0 0 auto;
    border: var(--format-popover-badge-marker-border, 0);
    border-radius: var(--format-popover-badge-marker-radius, 0);
    background: var(--format-popover-badge-marker-background, transparent);
    clip-path: var(--format-popover-badge-marker-clip, none);
  }

  .format-popover-column
    + .format-popover-column
    .format-popover-section-badge::before {
    background: var(
      --format-popover-output-marker-background,
      var(--format-popover-badge-marker-background, transparent)
    );
  }

  .format-popover-processing {
    margin-top: 10px;
    padding: 9px 8px 8px;
    border-top: 1px solid
      var(--format-popover-divider, rgba(128, 128, 128, 0.1));
    background: var(--format-popover-processing-background, transparent);
  }

  .format-popover-processing .format-popover-section-heading {
    margin-bottom: 6px;
  }

  .format-popover-processing-rows {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px 16px;
  }

  .format-popover-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content;
    gap: 8px;
    align-items: baseline;
    min-width: 0;
    min-height: 24px;
  }

  .format-popover-label {
    min-width: 0;
    color: var(--format-popover-muted, var(--text-secondary));
    font-size: 12px;
    font-weight: 600;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .format-popover-value {
    min-width: 0;
    max-width: 100%;
    color: var(--format-popover-ink, var(--text-primary));
    font-size: 12px;
    font-family: var(--font-mono);
    line-height: 1.4;
    text-align: right;
    white-space: nowrap;
  }

  .format-popover-flag {
    color: var(--format-popover-flag-color, var(--text-primary));
    font-weight: 700;
  }

  @media (max-width: 360px) {
    .format-popover-content {
      bottom: calc(100% + 4px);
      max-height: calc(100vh - 24px);
    }

    .format-popover-body {
      padding-bottom: 12px;
    }

    .format-popover-columns,
    .format-popover-processing-rows {
      grid-template-columns: minmax(0, 1fr);
    }

    .format-popover-columns {
      row-gap: 10px;
    }

    .format-popover-column,
    .format-popover-column + .format-popover-column {
      padding: 0;
    }

    .format-popover-column + .format-popover-column {
      padding-top: 10px;
      border-top: 1px solid
        var(--format-popover-divider, rgba(128, 128, 128, 0.12));
      border-left: 0;
    }
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

  .icon-button:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--surface-highlight) 86%, white 14%),
      0 0 0 4px rgba(var(--album-accent-rgb), 0.28);
    border-radius: var(--shape-pill);
  }

  .icon-button:disabled {
    opacity: 0.42;
  }

  .icon-button:disabled {
    cursor: not-allowed;
    box-shadow: none;
  }

  .play-button[aria-busy='true']:disabled {
    opacity: 1;
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

  .volume-group:has(:global(.volume-hover-zone--ark-ui)) {
    transition: width 120ms cubic-bezier(0.42, 0, 1, 1);
  }

  .volume-group.volume-expanded:has(:global(.volume-hover-zone--ark-ui)) {
    width: 200px;
    height: 40px;
    transition-duration: 180ms;
    transition-timing-function: cubic-bezier(0, 0, 0.58, 1);
  }

  .volume-group:has(:global(.volume-hover-zone--ark-ui .capsule-slider:focus)) {
    width: 200px;
    height: 40px;
    transition: none;
  }

  @media (hover: none) and (pointer: coarse) {
    .volume-group:has(:global(.volume-hover-zone--ark-ui)) {
      width: 200px;
      height: 40px;
      transition: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .volume-group:has(:global(.volume-hover-zone--ark-ui)),
    .right-controls:has(:global(.volume-hover-zone--ark-ui)) {
      transition: none;
    }
  }

  @media (max-width: 500px) {
    .right-controls:has(:global(.volume-hover-zone--ark-ui)) {
      justify-content: flex-end;
    }
  }

  @media (min-width: 501px) and (max-width: 900px) {
    .right-controls:has(:global(.volume-hover-zone--ark-ui)) {
      transition: transform 120ms cubic-bezier(0.42, 0, 1, 1);
    }

    .right-controls:has(
      .volume-group.volume-expanded :global(.volume-hover-zone--ark-ui)
    ) {
      transform: translateX(
        calc((var(--control-button-size, 34px) - 200px) / 2)
      );
      transition-duration: 180ms;
      transition-timing-function: cubic-bezier(0, 0, 0.58, 1);
    }

    .right-controls:has(
      :global(.volume-hover-zone--ark-ui .capsule-slider:focus)
    ) {
      transform: translateX(
        calc((var(--control-button-size, 34px) - 200px) / 2)
      );
      transition: none;
    }
  }
</style>
