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
    border-radius: 0 8px 8px 0;
    background: var(--accent);
    pointer-events: none;
  }
</style>
