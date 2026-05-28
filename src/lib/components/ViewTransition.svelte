<script lang="ts">
  import { gsap, getMotionDuration } from '$lib/design/gsap';
  import type { Snippet } from 'svelte';
  import { tick } from 'svelte';

  interface Props {
    viewKey: string;
    direction: 'forward' | 'back';
    duration?: number;
    reducedMotion: boolean;
    onTransitionStart?: () => void;
    onTransitionEnd?: () => void;
    children: Snippet;
  }

  let {
    viewKey,
    direction,
    duration = 350,
    reducedMotion,
    onTransitionStart,
    onTransitionEnd,
    children,
  }: Props = $props();

  let containerEl = $state<HTMLElement | null>(null);
  let outgoingHtml = $state<string | null>(null);
  let currentTimeline: gsap.core.Timeline | null = null;
  let previousKey: string | null = null;
  let pendingSnapshot: string | null = null;
  let snapshotDirection: 'forward' | 'back' = 'forward';

  let transitionActive = false;

  function notifyTransitionEnd() {
    if (!transitionActive) return;
    transitionActive = false;
    onTransitionEnd?.();
  }

  $effect.pre(() => {
    const key = viewKey;
    if (previousKey === null) {
      previousKey = key;
      return;
    }
    if (key === previousKey) return;
    previousKey = key;

    if (reducedMotion || !containerEl) return;

    const slot = containerEl.querySelector(
      '[data-view-slot="current"]'
    ) as HTMLElement | null;
    if (slot) {
      pendingSnapshot = slot.innerHTML;
      snapshotDirection = direction;
    }
  });

  $effect(() => {
    void viewKey;
    const container: HTMLElement | null = containerEl;
    if (!pendingSnapshot || !container) return;

    const html = pendingSnapshot;
    const dir = snapshotDirection;
    pendingSnapshot = null;
    outgoingHtml = html;

    void tick().then(() => {
      if (currentTimeline) {
        currentTimeline.kill();
        currentTimeline = null;
        notifyTransitionEnd();
      }

      const incomingSlot = container.querySelector(
        '[data-view-slot="current"]'
      ) as HTMLElement | null;
      const outgoingSlot = container.querySelector(
        '[data-view-slot="outgoing"]'
      ) as HTMLElement | null;
      if (!incomingSlot || !outgoingSlot) return;

      const dur = getMotionDuration(duration);
      const isForward = dir === 'forward';
      const inFromPercent = isForward ? 100 : -100;
      const outToPercent = isForward ? -30 : 30;

      const tl = gsap.timeline({
        onStart: () => {
          incomingSlot.style.position = 'absolute';
          incomingSlot.style.inset = '0';
          transitionActive = true;
          onTransitionStart?.();
        },
        onComplete: () => {
          gsap.set(incomingSlot, { clearProps: 'xPercent,opacity' });
          incomingSlot.style.position = '';
          incomingSlot.style.inset = '';
          notifyTransitionEnd();
          outgoingHtml = null;
        },
      });

      tl.fromTo(
        incomingSlot,
        { xPercent: inFromPercent, opacity: 1 },
        { xPercent: 0, opacity: 1, duration: dur, ease: 'ios-spring' },
        0
      );
      tl.fromTo(
        outgoingSlot,
        { xPercent: 0, opacity: 1 },
        { xPercent: outToPercent, opacity: 0.3, duration: dur, ease: 'ios' },
        0
      );

      currentTimeline = tl;
    });
  });

  $effect(() => {
    return () => {
      if (currentTimeline) {
        currentTimeline.kill();
        currentTimeline = null;
      }
      notifyTransitionEnd();
    };
  });
</script>

<div class="view-transition-container" bind:this={containerEl}>
  <div data-view-slot="current" class="view-slot">
    {@render children()}
  </div>
  {#if outgoingHtml}
    <div data-view-slot="outgoing" class="view-slot view-slot--outgoing">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html outgoingHtml}
    </div>
  {/if}
</div>

<style>
  .view-transition-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .view-slot {
    width: 100%;
    height: 100%;
  }

  .view-slot--outgoing {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
</style>
