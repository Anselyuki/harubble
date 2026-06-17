<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui';
  import SelectPortal from './select-portal.svelte';
  import SelectScrollUpButton from './select-scroll-up-button.svelte';
  import SelectScrollDownButton from './select-scroll-down-button.svelte';
  import { cn, type WithoutChild } from '$lib/utils.js';
  import type { ComponentProps } from 'svelte';
  import type { WithoutChildrenOrChild } from '$lib/utils.js';
  import { getContext } from 'svelte';
  import {
    gsap,
    getMotionDuration,
    killTweens,
    MOTION,
  } from '$lib/design/gsap';

  let {
    ref = $bindable(null),
    class: className,
    sideOffset = 4,
    portalProps,
    children,
    preventScroll = true,
    ...restProps
  }: WithoutChild<SelectPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof SelectPortal>>;
  } = $props();

  const openCtx = getContext<{ value: boolean } | undefined>('select-open');
  const open = $derived(openCtx?.value ?? true);

  let mounted = $state(true);

  $effect(() => {
    if (open) mounted = true;
  });

  $effect(() => {
    if (!ref || !open) return;
    killTweens(ref);
    gsap.fromTo(
      ref,
      { opacity: 0, scale: 0.95, y: -4 },
      {
        opacity: 1,
        scale: 1,
        y: 0,
        duration: getMotionDuration(MOTION.OVERLAY_IN),
        ease: 'ios-spring',
      }
    );
  });

  $effect(() => {
    if (open || !mounted || !ref) return;
    killTweens(ref);
    gsap.to(ref, {
      opacity: 0,
      scale: 0.95,
      y: -4,
      duration: getMotionDuration(MOTION.MICRO),
      ease: 'ios-in',
      onComplete: () => {
        mounted = false;
      },
    });
  });
</script>

{#if mounted}
  <SelectPortal {...portalProps}>
    <SelectPrimitive.Content
      bind:ref
      forceMount
      {sideOffset}
      {preventScroll}
      data-slot="select-content"
      class={cn(
        'bg-popover text-popover-foreground ring-foreground/10 min-w-36 rounded-lg shadow-md ring-1 relative isolate z-[200] overflow-x-hidden overflow-y-auto',
        className
      )}
      {...restProps}
    >
      <SelectScrollUpButton />
      <SelectPrimitive.Viewport
        class={cn(
          'h-(--bits-select-anchor-height) w-full min-w-(--bits-select-anchor-width) scroll-my-1'
        )}
      >
        {@render children?.()}
      </SelectPrimitive.Viewport>
      <SelectScrollDownButton />
    </SelectPrimitive.Content>
  </SelectPortal>
{/if}
