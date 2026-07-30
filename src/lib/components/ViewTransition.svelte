<script lang="ts">
  import {
    freezeViewSnapshot,
    runViewTransition,
  } from '$lib/design/view-transition';
  import { gsap } from '$lib/design/gsap';
  import type { Snippet } from 'svelte';
  import { tick } from 'svelte';

  interface Props {
    viewKey: string;
    direction: 'forward' | 'back';
    reducedMotion: boolean;
    onTransitionStart?: () => void;
    onTransitionEnd?: () => void;
    children: Snippet;
  }

  let {
    viewKey,
    direction,
    reducedMotion,
    onTransitionStart,
    onTransitionEnd,
    children,
  }: Props = $props();

  let containerEl = $state<HTMLElement | null>(null);
  let currentTimeline: gsap.core.Timeline | null = null;
  let frozenNode: HTMLElement | null = null;
  let previousKey: string | null = null;
  let pendingSnapshot: HTMLElement | null = null;
  let snapshotDirection: 'forward' | 'back' = 'forward';

  let transitionActive = false;

  function notifyTransitionEnd() {
    if (!transitionActive) return;
    transitionActive = false;
    onTransitionEnd?.();
  }

  /** 移除并清理当前挂载的出场快照节点。 */
  function disposeFrozenNode() {
    if (frozenNode) {
      frozenNode.remove();
      frozenNode = null;
    }
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
      // 在新视图渲染替换 slot 内容之前，冻结当前视图的真实 DOM 节点。
      pendingSnapshot = freezeViewSnapshot(slot);
      snapshotDirection = direction;
    }
  });

  $effect(() => {
    void viewKey;
    const container: HTMLElement | null = containerEl;
    if (!pendingSnapshot || !container) return;

    const snapshot = pendingSnapshot;
    const dir = snapshotDirection;
    pendingSnapshot = null;

    void tick().then(() => {
      if (currentTimeline) {
        currentTimeline.kill();
        currentTimeline = null;
      }
      disposeFrozenNode();
      notifyTransitionEnd();

      const incomingSlot = container.querySelector(
        '[data-view-slot="current"]'
      ) as HTMLElement | null;
      if (!incomingSlot) return;

      // 把冻结快照作为绝对定位的出场层挂到容器中。
      snapshot.classList.add('view-slot', 'view-slot--outgoing');
      // 快照只用于视觉出场，不能把旧页面的控件重新带回 Tab 顺序。
      snapshot.setAttribute('inert', '');
      snapshot.setAttribute('aria-hidden', 'true');
      container.appendChild(snapshot);
      frozenNode = snapshot;

      currentTimeline = runViewTransition({
        incoming: incomingSlot,
        outgoing: snapshot,
        direction: dir,
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
          disposeFrozenNode();
          currentTimeline = null;
          notifyTransitionEnd();
        },
      });
    });
  });

  $effect(() => {
    return () => {
      if (currentTimeline) {
        currentTimeline.kill();
        currentTimeline = null;
      }
      disposeFrozenNode();
      notifyTransitionEnd();
    };
  });
</script>

<div class="view-transition-container" bind:this={containerEl}>
  <div data-view-slot="current" class="view-slot">
    {@render children()}
  </div>
</div>

<style>
  .view-transition-container {
    position: relative;
    width: 100%;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .view-slot {
    width: 100%;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  :global(.view-transition-container > .view-slot--outgoing) {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
</style>
