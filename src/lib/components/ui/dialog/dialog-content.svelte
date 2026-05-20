<script lang="ts">
  import { Dialog as DialogPrimitive } from 'bits-ui';
  import DialogPortal from './dialog-portal.svelte';
  import type { Snippet } from 'svelte';
  import * as Dialog from './index.js';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';
  import type { ComponentProps } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import XIcon from '@lucide/svelte/icons/x';
  import { getContext } from 'svelte';
  import { gsap, getMotionDuration, killTweens } from '$lib/design/gsap';

  let {
    ref = $bindable(null),
    class: className,
    portalProps,
    children,
    showCloseButton = true,
    ...restProps
  }: WithoutChildrenOrChild<DialogPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof DialogPortal>>;
    children: Snippet;
    showCloseButton?: boolean;
  } = $props();

  const openCtx = getContext<{ value: boolean } | undefined>('dialog-open');
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
      { opacity: 0, scale: 0.95 },
      {
        opacity: 1,
        scale: 1,
        duration: getMotionDuration(200),
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
      duration: getMotionDuration(150),
      ease: 'ios-in',
      onComplete: () => {
        mounted = false;
      },
    });
  });
</script>

{#if mounted}
  <DialogPortal {...portalProps}>
    <Dialog.Overlay />
    <DialogPrimitive.Content
      bind:ref
      forceMount
      data-slot="dialog-content"
      class={cn(
        'dialog-content-fix bg-popover text-popover-foreground ring-foreground/10 grid max-w-[calc(100%-2rem)] gap-4 rounded-xl p-4 text-sm ring-1 sm:max-w-sm fixed top-1/2 left-1/2 z-50 w-full -translate-x-1/2 -translate-y-1/2 outline-none',
        className
      )}
      {...restProps}
    >
      {@render children?.()}
      {#if showCloseButton}
        <DialogPrimitive.Close data-slot="dialog-close">
          {#snippet child({ props })}
            <Button
              variant="ghost"
              class="absolute top-2 right-2"
              size="icon-sm"
              {...props}
            >
              <XIcon />
              <span class="sr-only">Close</span>
            </Button>
          {/snippet}
        </DialogPrimitive.Close>
      {/if}
    </DialogPrimitive.Content>
  </DialogPortal>
{/if}

<style>
  :global(.dialog-content-fix) {
    contain: style !important;
  }
</style>
