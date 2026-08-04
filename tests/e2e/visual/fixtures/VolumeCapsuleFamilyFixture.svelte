<script lang="ts">
  import { onMount } from 'svelte';
  import PlayerDock from '$lib/components/app/player/PlayerDock.svelte';
  import {
    applyAppThemeTokenSet,
    applyContextThemePalette,
    deriveGlobalTokensFromSlots,
  } from '$lib/themeTokens';
  import { applyThemePackageDocument } from '$lib/features/shell/themePackageManager.svelte';
  import { sliderToGain } from '$lib/features/player/volume';
  import arkPackageJson from '$lib/theme-packages/builtins/ark-ui-ark.json';
  import endfieldPackageJson from '$lib/theme-packages/builtins/ark-ui-endfield.json';
  import exaPackageJson from '$lib/theme-packages/builtins/ark-ui-exa.json';
  import popucomPackageJson from '$lib/theme-packages/builtins/ark-ui-popucom.json';
  import corporatePackageJson from '$lib/theme-packages/builtins/ark-ui-corporate.json';
  import type {
    PlaybackFormatState,
    ThemeColorSlots,
    ThemePalette,
    ThemePackageDocument,
  } from '$lib/types';
  import {
    applyVisualContract,
    isArkUiThemeFamily,
    SUPPORTED_THEME_FAMILIES,
    type ArkUiThemeFamily,
    type ThemeFamily,
  } from '$lib/features/shell/visualContract.svelte';

  const packages = {
    ark: arkPackageJson,
    endfield: endfieldPackageJson,
    exa: exaPackageJson,
    popucom: popucomPackageJson,
    corporate: corporatePackageJson,
  } as unknown as Record<ArkUiThemeFamily, ThemePackageDocument>;

  const params = new URLSearchParams(window.location.search);
  const requestedFamily = params.get('family');
  let family = $state<ThemeFamily>(
    requestedFamily &&
      (SUPPORTED_THEME_FAMILIES as readonly string[]).includes(requestedFamily)
      ? (requestedFamily as ThemeFamily)
      : 'ark'
  );
  const requestedScheme = params.get('scheme');
  const scheme: 'light' | 'dark' =
    requestedScheme === 'light' ? 'light' : 'dark';
  const showProcessing = params.get('processing') === 'true';

  const contextPalette: ThemePalette = {
    accentHex: '#F06A8B',
    accentHoverHex: '#F58BA5',
    accentRgb: [240, 106, 139],
    accentHoverRgb: [245, 139, 165],
    waveColors: [
      [240, 106, 139],
      [245, 139, 165],
    ],
    surfaceHex: '#161318',
    textPrimaryHex: '#F8F4F5',
    textSecondaryHex: '#C8BCC0',
    tintHex: '#8C7580',
    dangerHex: '#FF536E',
  };

  const song = {
    cid: 'qa-volume-family',
    name: 'Signal Path',
    artists: ['Harubble QA'],
    coverUrl: null,
  };
  const cleanPlaybackFormat: PlaybackFormatState = {
    sourceSampleRate: 48_000,
    sourceChannels: 2,
    sourceBitsPerSample: 24,
    sourceBitrateKbps: 2_304,
    outputSampleRate: 48_000,
    outputChannels: 2,
    outputBitsPerSample: 24,
    outputSampleFormat: 'f32',
    resampling: false,
    channelRemix: false,
  };
  const processingPlaybackFormat: PlaybackFormatState = {
    ...cleanPlaybackFormat,
    sourceSampleRate: 44_100,
    sourceBitrateKbps: 2_117,
    resampling: true,
  };

  let playbackFormat = $state<PlaybackFormatState | null>(
    showProcessing ? processingPlaybackFormat : cleanPlaybackFormat
  );
  let volume = $state(0.64);
  let muted = $state(false);
  let lastGain = $state(0.64);

  function handleVolumeChange(gain: number) {
    lastGain = gain;
    volume = gain;
  }

  function applyFamily(nextFamily: ThemeFamily) {
    family = nextFamily;
    if (!isArkUiThemeFamily(nextFamily)) {
      applyVisualContract({ family: nextFamily, depth: 'balanced' });
      return;
    }

    const themePackage = packages[nextFamily];
    applyThemePackageDocument(themePackage, scheme);
    const slots = {
      ...themePackage.slots,
      ...themePackage.variants?.[scheme],
    } as ThemeColorSlots;
    const tokens = deriveGlobalTokensFromSlots(slots, scheme);
    applyAppThemeTokenSet(tokens, { animate: false });
    applyContextThemePalette(contextPalette, tokens, scheme, {
      animate: false,
    });
  }

  onMount(() => {
    const root = document.documentElement;
    root.classList.remove('light', 'dark');
    root.classList.add(scheme);
    root.style.colorScheme = scheme;

    applyFamily(family);

    (
      window as typeof window & {
        __SET_VOLUME_CAPSULE_FAMILY__?: (nextFamily: ThemeFamily) => void;
        __SET_VOLUME_CAPSULE_POSITION__?: (position: number) => void;
        __SET_VOLUME_CAPSULE_FORMAT_AVAILABLE__?: (available: boolean) => void;
        __SET_VOLUME_CAPSULE_FORMAT_PROCESSING__?: (
          processing: boolean
        ) => void;
      }
    ).__SET_VOLUME_CAPSULE_FAMILY__ = (nextFamily) => {
      if (
        (SUPPORTED_THEME_FAMILIES as readonly ThemeFamily[]).includes(
          nextFamily
        )
      ) {
        applyFamily(nextFamily);
      }
    };
    (
      window as typeof window & {
        __SET_VOLUME_CAPSULE_POSITION__?: (position: number) => void;
      }
    ).__SET_VOLUME_CAPSULE_POSITION__ = (position) => {
      volume = sliderToGain(position);
    };
    (
      window as typeof window & {
        __SET_VOLUME_CAPSULE_FORMAT_AVAILABLE__?: (available: boolean) => void;
      }
    ).__SET_VOLUME_CAPSULE_FORMAT_AVAILABLE__ = (available) => {
      playbackFormat = available
        ? showProcessing
          ? processingPlaybackFormat
          : cleanPlaybackFormat
        : null;
    };
    (
      window as typeof window & {
        __SET_VOLUME_CAPSULE_FORMAT_PROCESSING__?: (
          processing: boolean
        ) => void;
      }
    ).__SET_VOLUME_CAPSULE_FORMAT_PROCESSING__ = (processing) => {
      playbackFormat = processing
        ? processingPlaybackFormat
        : cleanPlaybackFormat;
    };

    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        (
          window as typeof window & {
            __VOLUME_CAPSULE_FAMILY_FIXTURE_READY__?: boolean;
          }
        ).__VOLUME_CAPSULE_FAMILY_FIXTURE_READY__ = true;
      });
    });
  });
</script>

<main
  class="fixture-workspace"
  data-testid="volume-capsule-family-fixture"
  data-family={family}
  data-scheme={scheme}
  data-last-gain={lastGain}
>
  <div class="fixture-stage" aria-hidden="true"></div>
  <div class="fixture-dock">
    <PlayerDock
      {song}
      isPlaying={true}
      isPaused={false}
      hasPrevious={true}
      hasNext={true}
      progress={73}
      duration={247}
      reducedMotion={false}
      isShuffled={false}
      repeatMode="off"
      lyricsActive={false}
      lyricsUnavailable={false}
      playlistActive={false}
      downloadState="idle"
      downloadDisabled={false}
      {volume}
      {muted}
      {playbackFormat}
      onVolumeChange={handleVolumeChange}
      onToggleMute={() => (muted = !muted)}
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

  .fixture-workspace {
    width: 100vw;
    min-height: 100vh;
    display: grid;
    grid-template-rows: minmax(92px, 1fr) auto;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .fixture-stage {
    min-height: 0;
    opacity: 0.38;
    background:
      linear-gradient(
        90deg,
        transparent 0 68%,
        color-mix(in srgb, var(--theme-accent) 12%, transparent) 68% 69%,
        transparent 69%
      ),
      linear-gradient(
        var(--theme-custom-rule, var(--border)) 1px,
        transparent 1px
      );
    background-size:
      auto,
      100% 48px;
  }

  .fixture-dock {
    width: 100%;
    min-width: 0;
  }

  @media (max-width: 640px) {
    .fixture-dock :global(.am-player) {
      min-height: 141px;
    }
  }
</style>
