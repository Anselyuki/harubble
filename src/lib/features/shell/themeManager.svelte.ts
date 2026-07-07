import type { AlbumDetail, ThemePalette } from '$lib/types';
import { extractImageTheme, getImageDataUrl } from '$lib/api';
import { applyAlbumAccentPalette, applyThemeColors } from '$lib/theme';
import { resolveThemeColors } from '$lib/themePresets';
import { getPreferredAlbumArtworkUrl } from '$lib/features/library/selectors';
import { preloadImage } from '$lib/features/library/helpers';
import { shellStore } from '$lib/features/shell/store.svelte';

const ALBUM_VISUAL_REQUEST_DELAY_MS = 450;

const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, ms);
  });

interface ThemeManagerDeps {
  getSelectedAlbum: () => AlbumDetail | null;
  getFullscreenOpen: () => boolean;
  getSettingsTheme: () => {
    presetId: string;
    customColors: Record<string, string>;
    colorScheme: string;
  };
}

export function createThemeManager(deps: ThemeManagerDeps) {
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let activeThemeArtworkUrl: string | null = null;
  let cachedAlbumPalette = $state<ThemePalette | null>(null);
  let activeAlbumStageArtworkUrl: string | null = null;
  let selectedAlbumArtworkUrl = $state<string | null>(null);

  const resolvedThemeColors = $derived.by(() => {
    const theme = deps.getSettingsTheme();
    return resolveThemeColors({
      presetId: theme.presetId,
      customColors: theme.customColors,
    });
  });

  async function preloadAlbumArtwork(
    album: AlbumDetail
  ): Promise<number | null> {
    const sourceUrl = getPreferredAlbumArtworkUrl(album);
    if (!sourceUrl) return null;
    const resolvedUrl = await getImageDataUrl(sourceUrl).catch(() => sourceUrl);
    const meta = await preloadImage(resolvedUrl);
    return meta?.aspectRatio ?? null;
  }

  $effect(() => {
    const artworkUrl = getPreferredAlbumArtworkUrl(deps.getSelectedAlbum());
    if (artworkUrl === activeThemeArtworkUrl) return;

    activeThemeArtworkUrl = artworkUrl;
    const paletteRequestSeq = ++themeRequestSeq;

    if (!artworkUrl) {
      cachedAlbumPalette = null;
      return;
    }

    void (async () => {
      await delay(ALBUM_VISUAL_REQUEST_DELAY_MS);
      if (paletteRequestSeq !== themeRequestSeq) return;
      try {
        const palette = await extractImageTheme(artworkUrl);
        if (paletteRequestSeq !== themeRequestSeq) return;
        cachedAlbumPalette = palette;
      } catch {
        if (paletteRequestSeq !== themeRequestSeq) return;
        cachedAlbumPalette = null;
      }
    })();
  });

  $effect(() => {
    const themeColors = resolvedThemeColors;
    const shouldApplyAlbumTheme =
      shellStore.currentView === 'library' || deps.getFullscreenOpen();

    applyThemeColors(themeColors);
    applyAlbumAccentPalette(
      shouldApplyAlbumTheme ? cachedAlbumPalette : null,
      themeColors
    );
  });

  $effect(() => {
    const theme = deps.getSettingsTheme();
    const scheme = theme.colorScheme;
    const html = document.documentElement;

    function applyScheme(isDark: boolean) {
      html.classList.toggle('dark', isDark);
      html.classList.toggle('light', !isDark);
      html.style.colorScheme = isDark ? 'dark' : 'light';
    }

    if (scheme === 'light') {
      applyScheme(false);
      return;
    }
    if (scheme === 'dark') {
      applyScheme(true);
      return;
    }

    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    applyScheme(mq.matches);
    const handler = (e: MediaQueryListEvent) => applyScheme(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  });

  $effect(() => {
    const sourceUrl = getPreferredAlbumArtworkUrl(deps.getSelectedAlbum());
    if (sourceUrl === activeAlbumStageArtworkUrl) return;

    activeAlbumStageArtworkUrl = sourceUrl;
    const requestSeq = ++artworkRequestSeq;

    if (!sourceUrl) {
      selectedAlbumArtworkUrl = null;
      return;
    }

    void (async () => {
      await delay(ALBUM_VISUAL_REQUEST_DELAY_MS);
      if (requestSeq !== artworkRequestSeq) return;
      try {
        const dataUrl = await getImageDataUrl(sourceUrl);
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = dataUrl;
      } catch {
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = null;
      }
    })();
  });

  return {
    get resolvedThemeColors() {
      return resolvedThemeColors;
    },
    get selectedAlbumArtworkUrl() {
      return selectedAlbumArtworkUrl;
    },
    preloadAlbumArtwork,
  };
}

export type ThemeManager = ReturnType<typeof createThemeManager>;
