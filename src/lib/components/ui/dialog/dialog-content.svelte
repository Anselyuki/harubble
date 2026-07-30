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
  import {
    gsap,
    getMotionDuration,
    killTweens,
    MOTION,
  } from '$lib/design/gsap';
  import * as m from '$lib/paraglide/messages.js';

  let {
    ref = $bindable(null),
    class: className,
    portalProps,
    children,
    showCloseButton = true,
    restoreScrollDelay = 0,
    ...restProps
  }: WithoutChildrenOrChild<DialogPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof DialogPortal>>;
    children: Snippet;
    showCloseButton?: boolean;
  } = $props();

  const openCtx = getContext<{ value: boolean } | undefined>('dialog-open');
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
      duration: getMotionDuration(MOTION.BASE_OUT),
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
      {restoreScrollDelay}
      data-slot="dialog-content"
      class={cn(
        'app-dialog dialog-content-fix fixed top-1/2 left-1/2 z-[var(--z-dialog)] -translate-x-1/2 -translate-y-1/2 text-sm outline-none',
        className,
        !open && '!pointer-events-none'
      )}
      {...restProps}
      inert={!open}
      aria-hidden={!open}
    >
      {#snippet child({ props })}
        <div {...props}>
          {@render children?.()}
          {#if showCloseButton}
            <DialogPrimitive.Close data-slot="dialog-close">
              {#snippet child({ props: closeProps })}
                <Button
                  variant="ghost"
                  class="absolute top-3 right-3 size-10"
                  size="icon-sm"
                  {...closeProps}
                >
                  <XIcon />
                  <span class="sr-only">{m.ui_close()}</span>
                </Button>
              {/snippet}
            </DialogPrimitive.Close>
          {/if}
        </div>
      {/snippet}
    </DialogPrimitive.Content>
  </DialogPortal>
{/if}

<style>
  :global(.dialog-content-fix) {
    contain: style !important;
  }
</style>
