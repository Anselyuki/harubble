<script lang="ts">
  import { Tooltip as TooltipPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';
  import TooltipPortal from './tooltip-portal.svelte';
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
    sideOffset = 0,
    side = 'top',
    children,
    arrowClasses,
    portalProps,
    ...restProps
  }: TooltipPrimitive.ContentProps & {
    arrowClasses?: string;
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof TooltipPortal>>;
  } = $props();

  const openCtx = getContext<{ value: boolean } | undefined>('tooltip-open');
  const open = $derived(openCtx?.value ?? true);

  let mounted = $state(openCtx?.value ?? true);

  $effect(() => {
    if (open) mounted = true;
  });

  $effect(() => {
    if (!ref || !open) return;
    killTweens(ref);
    gsap.fromTo(
      ref,
      { opacity: 0, scale: 0.95 },
      {
        opacity: 1,
        scale: 1,
        duration: getMotionDuration(MOTION.OVERLAY_IN),
        ease: 'ios-out',
      }
    );
  });

  $effect(() => {
    if (open || !mounted || !ref) return;
    killTweens(ref);
    gsap.to(ref, {
      opacity: 0,
      scale: 0.95,
      duration: getMotionDuration(MOTION.MICRO),
      ease: 'ios-in',
      onComplete: () => {
        mounted = false;
      },
    });
  });
</script>

{#if mounted}
  <TooltipPortal {...portalProps}>
    <TooltipPrimitive.Content
      bind:ref
      forceMount
      data-slot="tooltip-content"
      {sideOffset}
      {side}
      class={cn(
        'pointer-events-none inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs has-data-[slot=kbd]:pr-1.5 **:data-[slot=kbd]:relative **:data-[slot=kbd]:isolate **:data-[slot=kbd]:z-[var(--z-tooltip)] **:data-[slot=kbd]:rounded-sm bg-foreground text-background z-[var(--z-tooltip)] w-fit max-w-xs origin-(--bits-tooltip-content-transform-origin)',
        className
      )}
      {...restProps}
    >
      {@render children?.()}
      <TooltipPrimitive.Arrow>
        {#snippet child({ props })}
          <div
            class={cn(
              'size-2.5 translate-y-[calc(-50%-2px)] rotate-45 rounded-[2px] bg-foreground fill-foreground z-[var(--z-tooltip)]',
              'data-[side=top]:translate-x-1/2 data-[side=top]:translate-y-[calc(-50%+2px)]',
              'data-[side=bottom]:-translate-x-1/2 data-[side=bottom]:-translate-y-[calc(-50%+1px)]',
              'data-[side=right]:translate-x-[calc(50%+2px)] data-[side=right]:translate-y-1/2',
              'data-[side=left]:-translate-y-[calc(50%-3px)]',
              arrowClasses
            )}
            {...props}
          ></div>
        {/snippet}
      </TooltipPrimitive.Arrow>
    </TooltipPrimitive.Content>
  </TooltipPortal>
{/if}
