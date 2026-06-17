<script lang="ts">
  import { Dialog as DialogPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';
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
    ...restProps
  }: DialogPrimitive.OverlayProps = $props();

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
      { opacity: 0 },
      {
        opacity: 1,
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
      duration: getMotionDuration(MOTION.BASE_OUT),
      ease: 'ios-in',
      onComplete: () => {
        mounted = false;
      },
    });
  });
</script>

{#if mounted}
  <DialogPrimitive.Overlay
    bind:ref
    forceMount
    data-slot="dialog-overlay"
    class={cn(
      'bg-black/20 supports-[backdrop-filter]:backdrop-blur-sm fixed inset-0 isolate z-50',
      className
    )}
    {...restProps}
  />
{/if}
