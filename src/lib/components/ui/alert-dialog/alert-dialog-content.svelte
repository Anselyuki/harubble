<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
  import AlertDialogPortal from './alert-dialog-portal.svelte';
  import AlertDialogOverlay from './alert-dialog-overlay.svelte';
  import {
    cn,
    type WithoutChild,
    type WithoutChildrenOrChild,
  } from '$lib/utils.js';
  import type { ComponentProps } from 'svelte';
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
    ...restProps
  }: WithoutChild<AlertDialogPrimitive.ContentProps> & {
    size?: 'default' | 'sm';
    portalProps?: WithoutChildrenOrChild<
      ComponentProps<typeof AlertDialogPortal>
    >;
  } = $props();

  const openCtx = getContext<{ value: boolean } | undefined>(
    'alert-dialog-open'
  );
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
      data-slot="alert-dialog-content"
      data-size={size}
      class={cn(
        'bg-popover text-popover-foreground ring-foreground/10 gap-4 rounded-xl p-4 ring-1 data-[size=default]:max-w-xs data-[size=sm]:max-w-xs data-[size=default]:sm:max-w-sm group/alert-dialog-content fixed top-1/2 left-1/2 z-50 grid w-full -translate-x-1/2 -translate-y-1/2 outline-none',
        className
      )}
      {...restProps}
    />
  </AlertDialogPortal>
{/if}
