<script lang="ts">
  import AudioPlayer from '$lib/components/AudioPlayer.svelte';
  import type { ComponentProps } from 'svelte';

  let props: ComponentProps<typeof AudioPlayer> = $props();
</script>

<div
  class="player-dock-wrap pointer-events-none relative z-[var(--z-player-dock)] w-full self-stretch px-0"
>
  <AudioPlayer {...props} />
</div>

<style>
  .player-dock-wrap :global(.am-player) {
    pointer-events: auto;
    position: relative;
    z-index: 1;
    width: 100%;
    border-radius: 0;
    border: 0;
    background: var(--surface-dock);
  }

  .player-dock-wrap :global(.am-player)::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: inherit;
    backdrop-filter: blur(24px) saturate(1.2);
    -webkit-backdrop-filter: blur(24px) saturate(1.2);
    pointer-events: none;
  }
</style>
