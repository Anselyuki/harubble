interface ImageMeta {
  aspectRatio: number;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function getImageMeta(image: HTMLImageElement): ImageMeta | null {
  const width = image.naturalWidth || image.width;
  const height = image.naturalHeight || image.height;

  if (!width || !height) {
    return null;
  }

  return {
    aspectRatio: width / height,
  };
}

export function preloadImage(
  src: string | null | undefined
): Promise<ImageMeta | null> {
  if (!src) return Promise.resolve(null);

  return new Promise((resolve) => {
    const image = new Image();
    let settled = false;

    const finish = (meta: ImageMeta | null) => {
      if (settled) return;
      settled = true;
      resolve(meta);
    };

    image.decoding = 'async';
    image.onload = () => finish(getImageMeta(image));
    image.onerror = () => finish(null);
    image.src = src;

    if (image.complete) {
      queueMicrotask(() => finish(getImageMeta(image)));
    }
  });
}
