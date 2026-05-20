<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';
  import { getContext } from 'svelte';
  import { gsap, getMotionDuration, killTweens } from '$lib/design/gsap';

  let {
    ref = $bindable(null),
    class: className,
    ...restProps
  }: AlertDialogPrimitive.OverlayProps = $props();

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
      { opacity: 0 },
      { opacity: 1, duration: getMotionDuration(200), ease: 'ios-out' }
    );
  });

  $effect(() => {
    if (open || !mounted || !ref) return;
    killTweens(ref);
    gsap.to(ref, {
      opacity: 0,
      duration: getMotionDuration(150),
      ease: 'ios-in',
      onComplete: () => {
        mounted = false;
      },
    });
  });
</script>

{#if mounted}
  <AlertDialogPrimitive.Overlay
    bind:ref
    forceMount
    data-slot="alert-dialog-overlay"
    class={cn(
      'bg-black/20 supports-[backdrop-filter]:backdrop-blur-sm fixed inset-0 z-50',
      className
    )}
    {...restProps}
  />
{/if}
