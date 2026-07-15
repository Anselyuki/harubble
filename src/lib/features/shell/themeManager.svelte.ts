import type { AlbumDetail, ThemePalette } from '$lib/types';
import { extractImageTheme, getImageSrc } from '$lib/api';
import {
  resolveAppThemeTokenSet,
  applyAppThemeTokenSet,
  applyContextThemePalette,
} from '$lib/themeTokens';
import { resolveThemeColors } from '$lib/themePresets';
import { getPreferredAlbumArtworkUrl } from '$lib/features/library/selectors';
import { preloadImage } from '$lib/features/library/helpers';

interface ThemeManagerDeps {
  getSelectedAlbum: () => AlbumDetail | null;
  getCurrentView: () => string;
  getFullscreenOpen: () => boolean;
  getSettingsTheme: () => {
    presetId: string;
    customColors: Record<string, string>;
    colorScheme: string;
    dynamicAlbumAccent: boolean;
  };
}

export function createThemeManager(deps: ThemeManagerDeps) {
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let activeThemeArtworkUrl: string | null = null;
  let cachedAlbumPalette = $state<ThemePalette | null>(null);
  let activeAlbumStageArtworkUrl: string | null = null;
  let selectedAlbumArtworkUrl = $state<string | null>(null);
  let warmingUrl: string | null = null;
  let warmingPromise: Promise<string> | null = null;

  const resolvedThemeColors = $derived.by(() => {
    const theme = deps.getSettingsTheme();
    return resolveThemeColors({
      presetId: theme.presetId,
      customColors: theme.customColors,
    });
  });

  let prefersColorSchemeDark = $state(
    typeof window !== 'undefined'
      ? window.matchMedia('(prefers-color-scheme: dark)').matches
      : false
  );

  const effectiveScheme = $derived.by((): 'light' | 'dark' => {
    const { colorScheme } = deps.getSettingsTheme();
    if (colorScheme === 'dark') return 'dark';
    if (colorScheme === 'light') return 'light';
    return prefersColorSchemeDark ? 'dark' : 'light';
  });

  function warmAlbumArtwork(coverUrl: string): void {
    if (warmingUrl === coverUrl && warmingPromise) return;
    warmingUrl = coverUrl;
    warmingPromise = getImageSrc(coverUrl).catch(() => coverUrl);
  }

  async function preloadAlbumArtwork(
    album: AlbumDetail
  ): Promise<number | null> {
    const sourceUrl = getPreferredAlbumArtworkUrl(album);
    if (!sourceUrl) return null;

    const imageSrcPromise =
      warmingUrl === sourceUrl && warmingPromise
        ? warmingPromise
        : getImageSrc(sourceUrl).catch(() => sourceUrl);
    warmingUrl = null;
    warmingPromise = null;

    const [resolvedUrl] = await Promise.all([
      imageSrcPromise,
      extractImageTheme(sourceUrl).catch(() => null),
    ]);
    const meta = await preloadImage(resolvedUrl);
    selectedAlbumArtworkUrl = resolvedUrl;
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
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => {
      prefersColorSchemeDark = e.matches;
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  });

  $effect(() => {
    const isDark = effectiveScheme === 'dark';
    const html = document.documentElement;
    html.classList.toggle('dark', isDark);
    html.classList.toggle('light', !isDark);
    html.style.colorScheme = isDark ? 'dark' : 'light';
  });

  $effect(() => {
    const { dynamicAlbumAccent } = deps.getSettingsTheme();
    const tokens = resolveAppThemeTokenSet(
      resolvedThemeColors,
      effectiveScheme
    );

    const useAlbumPalette =
      dynamicAlbumAccent &&
      cachedAlbumPalette &&
      (deps.getCurrentView() === 'library' || deps.getFullscreenOpen());

    applyAppThemeTokenSet(tokens);
    applyContextThemePalette(
      useAlbumPalette ? cachedAlbumPalette : null,
      tokens,
      effectiveScheme
    );
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
      if (requestSeq !== artworkRequestSeq) return;
      try {
        const imageSrc = await getImageSrc(sourceUrl);
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = imageSrc;
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
    warmAlbumArtwork,
  };
}

export type ThemeManager = ReturnType<typeof createThemeManager>;
