<script lang="ts">
  import AudioPlayer from '$lib/components/AudioPlayer.svelte';
  import type { ComponentProps } from 'svelte';

  let props: ComponentProps<typeof AudioPlayer> = $props();
</script>

<div
  class="player-dock-wrap pointer-events-none relative z-[140] w-full self-stretch px-0"
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
    background: var(--field-dock, var(--surface-dock));
    color: var(--field-paper, #f2f2f0);
    border-top: 1px solid var(--field-signal, var(--album-accent));
    --player-title: #f2f2f0;
    --player-subtitle: rgba(242, 242, 240, 0.64);
    --player-control-color: rgba(242, 242, 240, 0.82);
    --player-time: rgba(242, 242, 240, 0.56);
    --player-shell-bg: #191919;
    --player-shell-border: rgba(242, 242, 240, 0.16);
    --player-shell-highlight: rgba(242, 242, 240, 0.08);
    --player-track-bg: rgba(242, 242, 240, 0.14);
  }

  .player-dock-wrap :global(.am-player)::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: inherit;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    pointer-events: none;
  }
</style>
