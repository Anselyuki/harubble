import { animateIn, animateOut } from '$lib/design/gsap';
import { getImageSrc } from './api';

export type LazyLoadOptions = {
  root?: Element | Document | null;
  rootMargin?: string;
  reducedMotion?: boolean;
};

export function lazyLoad(
  node: HTMLElement,
  {
    root = null,
    rootMargin = '150px',
    reducedMotion = false,
  }: LazyLoadOptions = {}
) {
  let imageAnimation: gsap.core.Tween | null = null;
  let placeholderAnimation: gsap.core.Tween | null = null;
  let observer: IntersectionObserver | null = null;
  let loadSeq = 0;
  let loadStarted = false;
  let destroyed = false;

  const stopAnimations = () => {
    imageAnimation?.kill();
    placeholderAnimation?.kill();
  };

  const stopObserving = () => {
    observer?.disconnect();
    observer = null;
  };

  const loadImage = () => {
    if (loadStarted || destroyed) return;

    const src = node.dataset.src;
    const img = node.querySelector<HTMLImageElement>('img');
    if (!src || !img) {
      stopObserving();
      return;
    }

    const placeholder = node.querySelector<HTMLElement>(
      '.album-cover-placeholder'
    );
    const seq = ++loadSeq;
    loadStarted = true;
    stopObserving();

    void (async () => {
      try {
        const resolvedSrc = await getImageSrc(src);
        if (seq !== loadSeq) return;

        img.onload = () => {
          if (seq !== loadSeq) return;
          stopAnimations();
          if (placeholder) {
            placeholderAnimation = animateOut(
              placeholder,
              { opacity: 0 },
              reducedMotion ? 0 : 180
            );
          }
          imageAnimation = animateIn(
            img,
            { opacity: 0, scale: reducedMotion ? 1 : 1.04 },
            { opacity: 1, scale: 1 },
            reducedMotion ? 0 : 200,
            'ios-out'
          );
        };
        img.onerror = () => {
          if (seq !== loadSeq) return;
          stopAnimations();
          if (placeholder) {
            placeholder.style.opacity = '1';
          }
        };
        img.src = resolvedSrc;
      } catch {
        if (seq !== loadSeq) return;
        stopAnimations();
        if (placeholder) {
          placeholder.style.opacity = '1';
        }
      } finally {
        if (seq === loadSeq) {
          node.removeAttribute('data-src');
        }
      }
    })();
  };

  const startObserving = () => {
    stopObserving();
    if (loadStarted || destroyed || !node.dataset.src) return;

    if (typeof IntersectionObserver === 'undefined') {
      loadImage();
      return;
    }

    observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          loadImage();
        }
      },
      { root, rootMargin, threshold: 0 }
    );
    observer.observe(node);
  };

  startObserving();

  return {
    update(next: LazyLoadOptions = {}) {
      const nextRoot = next.root ?? null;
      const nextRootMargin = next.rootMargin ?? '150px';
      const observerConfigChanged =
        nextRoot !== root || nextRootMargin !== rootMargin;

      root = nextRoot;
      rootMargin = nextRootMargin;
      reducedMotion = next.reducedMotion ?? false;

      if (observerConfigChanged) {
        startObserving();
      }
    },
    destroy() {
      destroyed = true;
      loadSeq += 1;
      stopAnimations();
      stopObserving();
      const img = node.querySelector<HTMLImageElement>('img');
      if (img) {
        img.onload = null;
        img.onerror = null;
      }
    },
  };
}
