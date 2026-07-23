<script lang="ts">
  interface Props {
    slabEl?: HTMLElement | null;
  }

  let { slabEl = $bindable(null) }: Props = $props();
</script>

<span class="brand-slab" bind:this={slabEl}></span>

<style>
  .brand-slab {
    grid-area: 1 / 1;
    justify-self: start;
    align-self: stretch;
    width: var(--slab-width, 46px);
    margin-top: calc(20px + var(--safe-area-top));
    margin-bottom: 12px;
    border-radius: 0;
    background: var(--field-signal, var(--accent));
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - 12px),
      calc(100% - 12px) 100%,
      0 100%
    );
    pointer-events: none;
  }
</style>
