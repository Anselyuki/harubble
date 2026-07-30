<script lang="ts">
  import { onMount } from 'svelte';
  import AlbumDetailPanel from '$lib/components/app/album/AlbumDetailPanel.svelte';
  import {
    applyAppThemeTokenSet,
    applyContextThemePalette,
    deriveGlobalTokensFromSlots,
  } from '$lib/themeTokens';
  import endfieldPackageJson from '$lib/theme-packages/builtins/ark-ui-endfield.json';
  import type {
    AlbumDetail,
    ThemeColorSlots,
    ThemePackageDocument,
  } from '$lib/types';

  const endfieldPackage =
    endfieldPackageJson as unknown as ThemePackageDocument;
  const requestedScheme = new URLSearchParams(window.location.search).get(
    'scheme'
  );
  const scheme: 'light' | 'dark' =
    requestedScheme === 'dark' ? 'dark' : 'light';
  const album: AlbumDetail = {
    cid: 'qa-operation-basepoint',
    name: 'Operation Basepoint',
    intro: null,
    belong: 'Endfield',
    coverUrl: '',
    coverDeUrl: null,
    artists: ['Field Signal Unit'],
    download: {
      isDownloaded: false,
      downloadStatus: 'missing',
      inventoryVersion: 'qa-v1',
    },
    tags: [
      {
        dimension: 'record',
        values: ['FIELD RECORD / 01'],
      },
    ],
    songs: [],
  };

  let primaryClicks = $state(0);
  let secondaryClicks = $state(0);

  onMount(() => {
    const root = document.documentElement;
    const slots = {
      ...endfieldPackage.slots,
      ...endfieldPackage.variants?.[scheme],
    } as ThemeColorSlots;
    const tokens = deriveGlobalTokensFromSlots(slots, scheme);
    const depth = endfieldPackage.visualContract?.depth ?? 'complex';

    root.classList.remove('light', 'dark');
    root.classList.add(scheme);
    root.style.colorScheme = scheme;
    root.dataset.arkTheme = 'endfield';
    root.dataset.arkDepth = depth;
    root.dataset.themeFamily = 'endfield';
    root.dataset.themeDepth = depth;

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
        __ENDFIELD_ALBUM_ACTIONS_FIXTURE_READY__?: boolean;
      }
    ).__ENDFIELD_ALBUM_ACTIONS_FIXTURE_READY__ = true;
  });
</script>

<main
  class="qa-album-workspace"
  data-testid="endfield-album-actions-fixture"
  data-primary-clicks={primaryClicks}
  data-secondary-clicks={secondaryClicks}
>
  <div class="qa-album-stage" aria-hidden="true"></div>
  <AlbumDetailPanel
    {album}
    currentSongCid={null}
    isPlaybackActive={false}
    isPlaybackPaused={false}
    downloadingAlbumCid={null}
    selectionModeEnabled={false}
    selectedSongCids={[]}
    reducedMotion={false}
    onToggleSelectionMode={() => (secondaryClicks += 1)}
    onSelectAllSongs={() => {}}
    onDeselectAllSongs={() => {}}
    onInvertSongSelection={() => {}}
    onDownloadAlbum={() => {
      primaryClicks += 1;
    }}
    onDownloadSelection={() => {}}
    onPlaySong={() => {}}
    onTogglePlay={() => {}}
    onDownloadSong={() => {}}
    onToggleSongSelection={() => {}}
    isSongSelected={() => false}
    getSongDownloadState={() => 'idle'}
    isSongDownloadInteractionBlocked={() => false}
    hasAlbumDownloadJob={() => false}
    isSelectionDownloadDisabled={() => false}
    isCurrentSelectionCreating={() => false}
    hasCurrentSelectionJob={() => false}
  />
</main>

<style>
  :global(html),
  :global(body) {
    min-width: 0;
    min-height: 100%;
  }

  :global(body) {
    display: grid;
    margin: 0;
    place-items: center;
    overflow-x: hidden;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .qa-album-workspace {
    width: min(760px, calc(100% - 48px));
    padding: 72px 0 48px;
  }

  .qa-album-stage {
    position: relative;
    height: 116px;
    border: 1px solid var(--ark-field-ink);
    background:
      linear-gradient(
        112deg,
        transparent 0 64%,
        rgba(242, 242, 240, 0.16) 64% calc(64% + 1px),
        transparent calc(64% + 1px)
      ),
      var(--ark-field-ink);
  }

  .qa-album-stage::after {
    position: absolute;
    right: 22px;
    bottom: 16px;
    width: 84px;
    height: 8px;
    background: var(--ark-field-signal);
    content: '';
  }

  @media (max-width: 520px), (orientation: portrait) {
    :global(body) {
      place-items: start center;
    }

    .qa-album-workspace {
      width: calc(100% - 32px);
      padding: 56px 0 36px;
    }

    .qa-album-stage {
      height: 94px;
    }
  }
</style>
