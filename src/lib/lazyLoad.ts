import { animateIn, animateOut } from '$lib/design/gsap';
import { getImageDataUrl } from './api';

export function lazyLoad(
  node: HTMLElement,
  {
    rootMargin = '150px',
    reducedMotion = false,
  }: { rootMargin?: string; reducedMotion?: boolean } = {}
) {
  let imageAnimation: gsap.core.Tween | null = null;
  let placeholderAnimation: gsap.core.Tween | null = null;
  let loadSeq = 0;

  const stopAnimations = () => {
    imageAnimation?.kill();
    placeholderAnimation?.kill();
  };

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) {
          return;
        }

        const src = node.dataset.src;
        if (!src) {
          observer.unobserve(node);
          return;
        }

        const img = node.querySelector('img');
        const placeholder = node.querySelector<HTMLElement>(
          '.album-cover-placeholder'
        );
        if (!img) {
          observer.unobserve(node);
          return;
        }

        const seq = ++loadSeq;

        void (async () => {
          try {
            const resolvedSrc = await getImageDataUrl(src);
            if (seq !== loadSeq) return;

            img.onload = () => {
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
              stopAnimations();
              if (placeholder) {
                placeholder.style.opacity = '1';
              }
            };
            img.src = resolvedSrc;
          } catch {
            stopAnimations();
            if (placeholder) {
              placeholder.style.opacity = '1';
            }
          } finally {
            if (seq === loadSeq) {
              node.removeAttribute('data-src');
              observer.unobserve(node);
            }
          }
        })();
      });
    },
    { rootMargin, threshold: 0 }
  );

  observer.observe(node);

  return {
    update(next: { rootMargin?: string; reducedMotion?: boolean } = {}) {
      reducedMotion = next.reducedMotion ?? false;
    },
    destroy() {
      stopAnimations();
      observer.disconnect();
    },
  };
}
