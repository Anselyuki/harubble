<script lang="ts">
  import { onMount } from 'svelte';
  import PlayerDock from '$lib/components/app/player/PlayerDock.svelte';
  import {
    applyAppThemeTokenSet,
    applyContextThemePalette,
    deriveGlobalTokensFromSlots,
  } from '$lib/themeTokens';
  import { applyVisualContract } from '$lib/features/shell/visualContract.svelte';
  import endfieldPackageJson from '$lib/theme-packages/builtins/ark-ui-endfield.json';
  import type {
    PlaybackFormatState,
    ThemeColorSlots,
    ThemePackageDocument,
  } from '$lib/types';

  const endfieldPackage =
    endfieldPackageJson as unknown as ThemePackageDocument;
  const params = new URLSearchParams(window.location.search);
  const requestedScheme = params.get('scheme');
  const scheme: 'light' | 'dark' =
    requestedScheme === 'dark' ? 'dark' : 'light';
  const partialFormat = params.get('format') === 'partial';
  const song = {
    cid: 'qa-operation-basepoint',
    name: 'Operation Basepoint',
    artists: ['Field Signal Unit', 'MSR'],
    coverUrl: null,
  };
  const playbackFormat: PlaybackFormatState = {
    sourceSampleRate: 48_000,
    sourceChannels: 2,
    sourceBitsPerSample: partialFormat ? null : 24,
    sourceBitrateKbps: partialFormat ? null : 2_304,
    outputSampleRate: 48_000,
    outputChannels: 2,
    outputBitsPerSample: partialFormat ? 32 : 24,
    outputSampleFormat: 'f32',
    resampling: false,
    channelRemix: false,
  };

  onMount(() => {
    const root = document.documentElement;
    const slots = {
      ...endfieldPackage.slots,
      ...endfieldPackage.variants?.[scheme],
    } as ThemeColorSlots;
    const tokens = deriveGlobalTokensFromSlots(slots, scheme);

    root.classList.remove('light', 'dark');
    root.classList.add(scheme);
    root.style.colorScheme = scheme;
    applyVisualContract(endfieldPackage.visualContract);

    for (const [key, value] of Object.entries(endfieldPackage.shape ?? {})) {
      root.style.setProperty(`--shape-${key}`, `${value}px`);
    }
    for (const [key, value] of Object.entries(endfieldPackage.density ?? {})) {
      root.style.setProperty(`--density-${key}`, `${value}px`);
    }
    for (const [key, value] of Object.entries(
      endfieldPackage.elevation ?? {}
    )) {
      root.style.setProperty(`--elevation-${key}`, value);
    }
    for (const [key, value] of Object.entries(endfieldPackage.blur ?? {})) {
      root.style.setProperty(`--blur-${key}`, `${value}px`);
    }

    const cssVariables = {
      ...(endfieldPackage.cssVariables ?? {}),
      ...(endfieldPackage.cssVariableVariants?.[scheme] ?? {}),
    };
    for (const [key, value] of Object.entries(cssVariables)) {
      root.style.setProperty(key, value);
    }

    const fontVariableMap = {
      body: '--font-body',
      display: '--font-display',
      mono: '--font-mono',
    } as const;
    for (const [key, value] of Object.entries(
      endfieldPackage.fontFamily ?? {}
    )) {
      root.style.setProperty(
        fontVariableMap[key as keyof typeof fontVariableMap],
        value
      );
    }

    applyAppThemeTokenSet(tokens, { animate: false });
    applyContextThemePalette(null, tokens, scheme, { animate: false });

    (
      window as typeof window & {
        __ENDFIELD_PLAYER_DOCK_FIXTURE_READY__?: boolean;
      }
    ).__ENDFIELD_PLAYER_DOCK_FIXTURE_READY__ = true;
  });
</script>

<main class="qa-player-workspace" data-testid="endfield-player-dock-fixture">
  <section class="qa-player-stage" aria-hidden="true">
    <span>FIELD SIGNAL / PLAYBACK</span>
  </section>
  <div class="qa-player-dock">
    <PlayerDock
      {song}
      isPlaying={true}
      isPaused={false}
      hasPrevious={true}
      hasNext={true}
      progress={61}
      duration={249}
      reducedMotion={true}
      isShuffled={false}
      repeatMode="off"
      lyricsActive={false}
      lyricsUnavailable={false}
      playlistActive={false}
      downloadState="idle"
      downloadDisabled={false}
      volume={0.72}
      muted={false}
      {playbackFormat}
      onVolumeChange={() => {}}
      onToggleMute={() => {}}
      onPrevious={() => {}}
      onTogglePlay={() => {}}
      onSeek={() => {}}
      onNext={() => {}}
      onShuffleChange={() => {}}
      onRepeatModeChange={() => {}}
      onToggleLyrics={() => {}}
      onTogglePlaylist={() => {}}
      onToggleFullscreen={() => {}}
      onDownload={() => {}}
    />
  </div>
</main>

<style>
  :global(html),
  :global(body) {
    min-width: 0;
    min-height: 100%;
  }

  :global(body) {
    margin: 0;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .qa-player-workspace {
    width: 100vw;
    min-height: 100vh;
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .qa-player-stage {
    position: relative;
    min-height: 0;
    overflow: hidden;
    border-inline: 1px solid var(--ark-field-rule-soft);
    background:
      linear-gradient(
        118deg,
        transparent 0 68%,
        color-mix(in srgb, var(--ark-field-signal) 20%, transparent) 68% 76%,
        transparent 76%
      ),
      linear-gradient(var(--ark-field-rule-soft) 1px, transparent 1px),
      var(--bg-primary);
    background-size:
      auto,
      100% 48px,
      auto;
  }

  .qa-player-stage span {
    position: absolute;
    right: 22px;
    bottom: 18px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
  }

  .qa-player-dock {
    width: 100%;
    min-width: 0;
  }
</style>
