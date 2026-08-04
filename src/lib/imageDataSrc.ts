import { getImageSrc } from './api';

type ImageSource = string | null | undefined;
type ImageLoadingMode = 'eager' | 'lazy';

type ImageDataSrcOptions = {
  src?: ImageSource;
  loading?: ImageLoadingMode;
  rootMargin?: string;
};

type ImageDataSrcInput = ImageSource | ImageDataSrcOptions;

function normalizeImageDataSrcInput(
  input: ImageDataSrcInput
): Required<ImageDataSrcOptions> {
  if (typeof input === 'string' || input == null) {
    return {
      src: input ?? null,
      loading: 'eager',
      rootMargin: '150px',
    };
  }

  return {
    src: input.src ?? null,
    loading: input.loading ?? 'eager',
    rootMargin: input.rootMargin ?? '150px',
  };
}

export function imageDataSrc(
  node: HTMLImageElement,
  source: ImageDataSrcInput
) {
  let requestSeq = 0;
  let activeSource: ImageSource = undefined;
  let activeLoadingMode: ImageLoadingMode = 'eager';
  let activeRootMargin = '150px';
  let pendingSource: ImageSource = null;
  let observer: IntersectionObserver | null = null;
  let initialized = false;

  function setState(state: 'empty' | 'loading' | 'loaded' | 'error') {
    node.dataset.imageState = state;
  }

  function stopObserver() {
    observer?.disconnect();
    observer = null;
  }

  async function load(nextSource: ImageSource) {
    const normalizedSource = nextSource || null;
    if (
      normalizedSource === activeSource &&
      pendingSource === null &&
      node.dataset.imageState === 'loaded'
    ) {
      return;
    }

    activeSource = normalizedSource;
    pendingSource = null;
    stopObserver();
    const seq = ++requestSeq;
    node.removeAttribute('src');
    setState(normalizedSource ? 'loading' : 'empty');

    if (!normalizedSource) return;

    try {
      const imageSrc = await getImageSrc(normalizedSource);
      if (seq !== requestSeq) return;

      node.src = imageSrc;
      setState('loaded');
    } catch {
      if (seq !== requestSeq) return;

      node.removeAttribute('src');
      setState('error');
    }
  }

  function observeWhenVisible(nextSource: string, rootMargin: string) {
    activeSource = nextSource;
    pendingSource = nextSource;
    stopObserver();
    node.removeAttribute('src');
    setState('empty');

    if (typeof IntersectionObserver === 'undefined') {
      void load(nextSource);
      return;
    }

    const observedSource = nextSource;
    observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) {
          return;
        }

        if (
          pendingSource !== observedSource ||
          activeLoadingMode !== 'lazy' ||
          activeRootMargin !== rootMargin
        ) {
          stopObserver();
          return;
        }

        void load(observedSource);
      },
      { rootMargin, threshold: 0 }
    );

    observer.observe(node);
  }

  function apply(nextSource: ImageDataSrcInput) {
    const next = normalizeImageDataSrcInput(nextSource);
    const sameConfig =
      initialized &&
      next.src === activeSource &&
      next.loading === activeLoadingMode &&
      next.rootMargin === activeRootMargin &&
      (pendingSource === null || pendingSource === next.src);

    if (sameConfig) {
      return;
    }

    initialized = true;
    activeLoadingMode = next.loading;
    activeRootMargin = next.rootMargin;
    requestSeq += 1;

    if (!next.src) {
      activeSource = null;
      pendingSource = null;
      stopObserver();
      node.removeAttribute('src');
      setState('empty');
      return;
    }

    if (next.loading === 'lazy') {
      observeWhenVisible(next.src, next.rootMargin);
      return;
    }

    pendingSource = null;
    void load(next.src);
  }

  function handleImageError() {
    if (node.dataset.imageState !== 'loaded') return;

    node.removeAttribute('src');
    setState('error');
  }

  node.decoding = 'async';
  node.addEventListener('error', handleImageError);
  apply(source);

  return {
    update(nextSource: ImageDataSrcInput) {
      apply(nextSource);
    },
    destroy() {
      requestSeq += 1;
      stopObserver();
      node.removeEventListener('error', handleImageError);
    },
  };
}
