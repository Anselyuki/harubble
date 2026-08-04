// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

const getImageSrcMock = vi.hoisted(() => vi.fn());

vi.mock('./api', () => ({
  getImageSrc: getImageSrcMock,
}));

vi.mock('$lib/design/gsap', () => ({
  animateIn: vi.fn(),
  animateOut: vi.fn(),
}));

type ObserverRecord = {
  callback: IntersectionObserverCallback;
  instance: MockIntersectionObserver;
  options: IntersectionObserverInit;
};

const observerRecords: ObserverRecord[] = [];

class MockIntersectionObserver {
  readonly root: Element | Document | null;
  readonly rootMargin: string;
  readonly thresholds: readonly number[];
  observedTarget: Element | null = null;
  disconnect = vi.fn();
  unobserve = vi.fn();

  constructor(
    callback: IntersectionObserverCallback,
    options: IntersectionObserverInit = {}
  ) {
    this.root = options.root ?? null;
    this.rootMargin = options.rootMargin ?? '0px';
    this.thresholds = Array.isArray(options.threshold)
      ? options.threshold
      : [options.threshold ?? 0];
    observerRecords.push({ callback, instance: this, options });
  }

  observe(target: Element) {
    this.observedTarget = target;
  }

  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }

  trigger(isIntersecting = true) {
    if (!this.observedTarget) return;
    const record = observerRecords.find((item) => item.instance === this);
    record?.callback(
      [
        {
          isIntersecting,
          target: this.observedTarget,
        } as IntersectionObserverEntry,
      ],
      this as unknown as IntersectionObserver
    );
  }
}

function createCoverNode() {
  const node = document.createElement('div');
  node.dataset.src = 'https://example.com/cover.jpg';
  node.innerHTML =
    '<div class="album-cover-placeholder"></div><img alt="cover" />';
  return node;
}

describe('lazyLoad', () => {
  beforeEach(() => {
    observerRecords.length = 0;
    getImageSrcMock.mockReset();
    getImageSrcMock.mockResolvedValue('asset://localhost/cover.jpg');
    vi.stubGlobal('IntersectionObserver', MockIntersectionObserver);
  });

  it('observes against the configured scroll root and preload margin', async () => {
    const { lazyLoad } = await import('./lazyLoad');
    const root = document.createElement('main');
    const node = createCoverNode();

    lazyLoad(node, {
      root,
      rootMargin: '0px 0px 900px 0px',
    });

    expect(observerRecords).toHaveLength(1);
    expect(observerRecords[0]?.options).toMatchObject({
      root,
      rootMargin: '0px 0px 900px 0px',
      threshold: 0,
    });

    observerRecords[0]?.instance.trigger();
    observerRecords[0]?.instance.trigger();
    await vi.waitFor(() => {
      expect(getImageSrcMock).toHaveBeenCalledTimes(1);
    });
  });

  it('recreates the observer when the scroll root becomes available', async () => {
    const { lazyLoad } = await import('./lazyLoad');
    const root = document.createElement('main');
    const node = createCoverNode();
    const action = lazyLoad(node);

    action.update({
      root,
      rootMargin: '0px 0px 900px 0px',
    });

    expect(observerRecords).toHaveLength(2);
    expect(observerRecords[0]?.instance.disconnect).toHaveBeenCalledOnce();
    expect(observerRecords[1]?.options.root).toBe(root);
    expect(observerRecords[1]?.options.rootMargin).toBe('0px 0px 900px 0px');
  });
});
