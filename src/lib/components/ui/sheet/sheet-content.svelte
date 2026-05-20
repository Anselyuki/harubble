<script lang="ts" module>
  export type Side = 'top' | 'right' | 'bottom' | 'left';
</script>

<script lang="ts">
  import { Dialog as SheetPrimitive } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import SheetPortal from './sheet-portal.svelte';
  import SheetOverlay from './sheet-overlay.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import XIcon from '@lucide/svelte/icons/x';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';
  import type { ComponentProps } from 'svelte';
  import { getContext } from 'svelte';
  import { gsap, getMotionDuration, killTweens } from '$lib/design/gsap';

  let {
    ref = $bindable(null),
    class: className,
    side = 'right',
    showCloseButton = true,
    portalProps,
    children,
    ...restProps
  }: WithoutChildrenOrChild<SheetPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof SheetPortal>>;
    side?: Side;
    showCloseButton?: boolean;
    children: Snippet;
  } = $props();

  const openCtx = getContext<{ value: boolean } | undefined>('sheet-open');
  const open = $derived(openCtx?.value ?? true);

  let mounted = $state(true);

  function getSlideFrom(s: Side): { x?: string; y?: string } {
    if (s === 'top') return { y: '-100%' };
    if (s === 'bottom') return { y: '100%' };
    if (s === 'left') return { x: '-100%' };
    return { x: '100%' };
  }

  $effect(() => {
    if (open) mounted = true;
  });

  $effect(() => {
    if (!ref || !open) return;
    const from = getSlideFrom(side);
    killTweens(ref);
    gsap.fromTo(
      ref,
      { opacity: 0, ...from },
      {
        opacity: 1,
        x: 0,
        y: 0,
        duration: getMotionDuration(250),
        ease: 'ios-spring',
      }
    );
  });

  $effect(() => {
    if (open || !mounted || !ref) return;
    const to = getSlideFrom(side);
    killTweens(ref);
    gsap.to(ref, {
      opacity: 0,
      ...to,
      duration: getMotionDuration(200),
      ease: 'ios-in',
      onComplete: () => {
        mounted = false;
      },
    });
  });
</script>

{#if mounted}
  <SheetPortal {...portalProps}>
    <SheetOverlay />
    <SheetPrimitive.Content
      bind:ref
      forceMount
      data-slot="sheet-content"
      data-side={side}
      class={cn(
        'bg-popover text-popover-foreground fixed z-[190] flex flex-col gap-4 text-sm shadow-lg data-[side=bottom]:inset-x-0 data-[side=bottom]:bottom-0 data-[side=bottom]:h-auto data-[side=bottom]:border-t data-[side=left]:inset-y-0 data-[side=left]:left-0 data-[side=left]:h-full data-[side=left]:w-3/4 data-[side=left]:border-r data-[side=right]:inset-y-0 data-[side=right]:right-0 data-[side=right]:h-full data-[side=right]:w-3/4 data-[side=right]:border-l data-[side=top]:inset-x-0 data-[side=top]:top-0 data-[side=top]:h-auto data-[side=top]:border-b data-[side=left]:sm:max-w-sm data-[side=right]:sm:max-w-sm',
        className
      )}
      {...restProps}
    >
      {@render children?.()}
      {#if showCloseButton}
        <SheetPrimitive.Close data-slot="sheet-close">
          {#snippet child({ props })}
            <Button
              variant="ghost"
              class="absolute top-3 right-3"
              size="icon-sm"
              {...props}
            >
              <XIcon />
              <span class="sr-only">Close</span>
            </Button>
          {/snippet}
        </SheetPrimitive.Close>
      {/if}
    </SheetPrimitive.Content>
  </SheetPortal>
{/if}
