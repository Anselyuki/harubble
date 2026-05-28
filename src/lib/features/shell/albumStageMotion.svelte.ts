import type { EventListeners, OverlayScrollbars } from 'overlayscrollbars';
import { clamp } from '$lib/features/library/helpers';
import { gsap, getMotionDuration } from '$lib/design/gsap';

interface AlbumStageMotionDeps {
  getReducedMotion: () => boolean;
  getViewportHeight: () => number;
  getLoadingDetail: () => boolean;
  getIsViewTransitioning: () => boolean;
}

const DEFAULT_ALBUM_STAGE_ASPECT_RATIO = 16 / 9;
const ALBUM_STAGE_BASE_VIEWPORT_RATIO = 1 / 3;
const ALBUM_STAGE_COLLAPSE_SCROLL_RANGE = 260;
const ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE = 220;

export function createAlbumStageMotionController(deps: AlbumStageMotionDeps) {
  let contentElement = $state<HTMLElement | null>(null);
  let albumStageElement = $state<HTMLElement | null>(null);
  let albumStageAspectRatio = $state(DEFAULT_ALBUM_STAGE_ASPECT_RATIO);
  let albumStageWidth = $state(0);
  let albumStageCollapseOffset = $state(0);
  let albumStageScrollTop = $state(0);
  let albumStageMotionFrame = 0;
  let pendingAlbumStageCollapseOffset = 0;
  let pendingAlbumStageScrollTop = 0;

  const reducedMotion = $derived.by(() => deps.getReducedMotion());
  const viewportHeight = $derived.by(() => deps.getViewportHeight());
  const isTransitioning = $derived.by(() => deps.getIsViewTransitioning());

  function setContentViewport(instance: OverlayScrollbars) {
    const viewport = instance.elements().viewport;
    if (contentElement !== viewport) {
      contentElement = viewport;
    }
  }

  function flushMotion() {
    albumStageMotionFrame = 0;

    if (albumStageCollapseOffset !== pendingAlbumStageCollapseOffset) {
      albumStageCollapseOffset = pendingAlbumStageCollapseOffset;
    }

    if (albumStageScrollTop !== pendingAlbumStageScrollTop) {
      albumStageScrollTop = pendingAlbumStageScrollTop;
    }
  }

  function scheduleMotion(
    next: {
      collapseOffset?: number;
      scrollTop?: number;
    },
    immediate = false
  ) {
    pendingAlbumStageCollapseOffset =
      next.collapseOffset ?? pendingAlbumStageCollapseOffset;
    pendingAlbumStageScrollTop = next.scrollTop ?? pendingAlbumStageScrollTop;

    if (immediate || reducedMotion || typeof window === 'undefined') {
      if (albumStageMotionFrame) {
        cancelAnimationFrame(albumStageMotionFrame);
        albumStageMotionFrame = 0;
      }
      flushMotion();
      return;
    }

    if (albumStageMotionFrame) {
      return;
    }

    albumStageMotionFrame = requestAnimationFrame(() => {
      flushMotion();
    });
  }

  function resetMotion() {
    if (albumStageMotionFrame) {
      cancelAnimationFrame(albumStageMotionFrame);
      albumStageMotionFrame = 0;
    }

    pendingAlbumStageCollapseOffset = 0;
    pendingAlbumStageScrollTop = 0;
    albumStageCollapseOffset = 0;
    albumStageScrollTop = 0;
  }

  function syncAlbumStageWidth() {
    albumStageWidth = albumStageElement?.clientWidth ?? 0;
  }

  function setAspectRatio(value: number | null | undefined) {
    if (value && Number.isFinite(value) && value > 0) {
      albumStageAspectRatio = value;
      return;
    }

    albumStageAspectRatio = DEFAULT_ALBUM_STAGE_ASPECT_RATIO;
  }

  function resetContentScroll() {
    resetMotion();
    if (!contentElement) return;
    gsap.to(contentElement, {
      scrollTo: { y: 0 },
      duration: getMotionDuration(400),
      ease: 'ios-out',
    });
  }

  function handleContentScroll() {
    if (deps.getIsViewTransitioning()) return;
    if (deps.getLoadingDetail()) {
      scheduleMotion({ scrollTop: 0 }, true);
      return;
    }

    const nextScrollTop = Math.max(0, contentElement?.scrollTop ?? 0);
    const nextCollapseOffset =
      nextScrollTop > 0 &&
      pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        ? ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        : undefined;

    scheduleMotion({
      scrollTop: nextScrollTop,
      collapseOffset: nextCollapseOffset,
    });
  }

  function handleContentWheel(event: WheelEvent) {
    if (deps.getIsViewTransitioning()) return;
    if (deps.getLoadingDetail() || !contentElement) {
      return;
    }

    const atTop = contentElement.scrollTop <= 0.5;
    if (!atTop) {
      return;
    }

    if (
      event.deltaY > 0 &&
      pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
    ) {
      event.preventDefault();
      scheduleMotion({
        collapseOffset: clamp(
          pendingAlbumStageCollapseOffset + event.deltaY,
          0,
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        ),
        scrollTop: 0,
      });
      return;
    }

    if (event.deltaY < 0 && pendingAlbumStageCollapseOffset > 0) {
      event.preventDefault();
      scheduleMotion({
        collapseOffset: clamp(
          pendingAlbumStageCollapseOffset + event.deltaY,
          0,
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        ),
        scrollTop: 0,
      });
    }
  }

  const contentScrollbarEvents = $derived.by(
    (): EventListeners => ({
      initialized(instance) {
        setContentViewport(instance);
        handleContentScroll();
      },
      updated(instance) {
        setContentViewport(instance);
      },
      destroyed() {
        contentElement = null;
      },
      scroll(instance) {
        setContentViewport(instance);
        handleContentScroll();
      },
    })
  );

  const albumStageFullHeight = $derived.by(() => {
    if (!albumStageWidth || !albumStageAspectRatio) {
      return 0;
    }

    return albumStageWidth / albumStageAspectRatio;
  });

  const albumStageBaseHeight = $derived.by(() => {
    if (!albumStageWidth) {
      return 0;
    }

    return Math.min(
      albumStageFullHeight,
      viewportHeight * ALBUM_STAGE_BASE_VIEWPORT_RATIO
    );
  });

  const albumStageCollapseProgress = $derived.by(() =>
    clamp(albumStageCollapseOffset / ALBUM_STAGE_COLLAPSE_SCROLL_RANGE, 0, 1)
  );

  const albumStageRevealProgress = $derived.by(
    () => 1 - albumStageCollapseProgress
  );

  const albumStageSolidifyProgress = $derived.by(() =>
    Math.max(
      albumStageCollapseProgress,
      clamp(albumStageScrollTop / ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE, 0, 1)
    )
  );

  const albumStageHeight = $derived.by(() => {
    if (!albumStageBaseHeight) {
      return 0;
    }

    return (
      albumStageBaseHeight +
      (albumStageFullHeight - albumStageBaseHeight) * albumStageRevealProgress
    );
  });

  const albumStageStyle = $derived.by(
    () => `--album-stage-aspect-ratio: ${albumStageAspectRatio}`
  );

  const albumStageMotionHeight = $derived.by(() =>
    albumStageHeight > 0
      ? albumStageHeight
      : Math.max(albumStageBaseHeight || 0, 280)
  );

  const albumStageMediaHeight = $derived.by(
    () => `${albumStageMotionHeight}px`
  );
  const albumStageScrimOpacity = $derived.by(() =>
    isTransitioning ? 1 : Math.max(0.58, 1 - albumStageSolidifyProgress * 0.34)
  );
  const albumStageImageOpacity = $derived.by(() =>
    isTransitioning ? 0 : 1 - albumStageSolidifyProgress * 0.54
  );
  const albumStageImageTransform = $derived.by(() =>
    reducedMotion
      ? 'translateZ(0) scale(1)'
      : `translateZ(0) scale(${1 + albumStageRevealProgress * 0.006 + albumStageSolidifyProgress * 0.012})`
  );
  const albumStageSolidifyOpacity = $derived.by(() =>
    isTransitioning ? 1 : albumStageSolidifyProgress
  );

  $effect(() => {
    if (!albumStageElement) return;

    syncAlbumStageWidth();

    if (typeof ResizeObserver === 'undefined') return;

    const observer = new ResizeObserver((entries) => {
      const newWidth = entries[0].contentRect.width;
      if (newWidth === albumStageWidth) return;
      albumStageWidth = newWidth;
    });

    observer.observe(albumStageElement);

    return () => observer.disconnect();
  });

  function dispose() {
    resetMotion();
    contentElement = null;
    albumStageElement = null;
    albumStageWidth = 0;
  }

  return {
    get contentElement() {
      return contentElement;
    },
    get albumStageElement() {
      return albumStageElement;
    },
    set albumStageElement(value: HTMLElement | null) {
      albumStageElement = value;
    },
    get contentScrollbarEvents() {
      return contentScrollbarEvents;
    },
    get albumStageStyle() {
      return albumStageStyle;
    },
    get albumStageMediaHeight() {
      return albumStageMediaHeight;
    },
    get albumStageScrimOpacity() {
      return albumStageScrimOpacity;
    },
    get albumStageImageOpacity() {
      return albumStageImageOpacity;
    },
    get albumStageImageTransform() {
      return albumStageImageTransform;
    },
    get albumStageSolidifyOpacity() {
      return albumStageSolidifyOpacity;
    },
    setAspectRatio,
    resetContentScroll,
    handleContentWheel,
    dispose,
  };
}
