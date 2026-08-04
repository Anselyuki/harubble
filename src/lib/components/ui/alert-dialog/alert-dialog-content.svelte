<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
  import AlertDialogPortal from './alert-dialog-portal.svelte';
  import AlertDialogOverlay from './alert-dialog-overlay.svelte';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';
  import type { ComponentProps } from 'svelte';
  import type { Snippet } from 'svelte';
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
    size = 'default',
    portalProps,
    children,
    restoreScrollDelay = 0,
    ...restProps
  }: WithoutChildrenOrChild<AlertDialogPrimitive.ContentProps> & {
    size?: 'default' | 'sm';
    children?: Snippet;
    portalProps?: WithoutChildrenOrChild<
      ComponentProps<typeof AlertDialogPortal>
    >;
  } = $props();

  const openCtx = getContext<{ value: boolean } | undefined>(
    'alert-dialog-open'
  );
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
  <AlertDialogPortal {...portalProps}>
    <AlertDialogOverlay />
    <AlertDialogPrimitive.Content
      bind:ref
      forceMount
      {restoreScrollDelay}
      data-slot="alert-dialog-content"
      data-size={size}
      class={cn(
        'bg-popover text-popover-foreground ring-foreground/10 gap-4 rounded-xl p-4 ring-1 data-[size=default]:max-w-xs data-[size=sm]:max-w-xs data-[size=default]:sm:max-w-sm group/alert-dialog-content fixed top-1/2 left-1/2 z-[var(--z-dialog)] grid w-full -translate-x-1/2 -translate-y-1/2 outline-none',
        className,
        !open && '!pointer-events-none'
      )}
      {...restProps}
      inert={!open}
      aria-hidden={!open}
    >
      {#snippet child({ props })}
        <div {...props}>{@render children?.()}</div>
      {/snippet}
    </AlertDialogPrimitive.Content>
  </AlertDialogPortal>
{/if}
