<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    isMacOS?: boolean;
    layoutCollapsed: boolean;
    containerEl?: HTMLDivElement | null;
    onCharsReady?: (els: HTMLSpanElement[]) => void;
  }

  let {
    isMacOS = false,
    layoutCollapsed,
    containerEl = $bindable(null),
    onCharsReady,
  }: Props = $props();

  const BRAND_LETTERS = [
    { char: 'H', outline: false },
    { char: 'A', outline: false },
    { char: 'R', outline: false },
    { char: 'U', outline: false },
    { char: 'K', outline: true },
    { char: 'A', outline: true },
    { char: 'B', outline: true },
    { char: 'U', outline: true },
    { char: 'B', outline: false },
    { char: 'B', outline: false },
    { char: 'L', outline: false },
    { char: 'E', outline: false },
  ] as const;

  const ROW1 = BRAND_LETTERS.slice(0, 6);
  const ROW2 = BRAND_LETTERS.slice(6);

  const charEls: (HTMLSpanElement | null)[] = $state(Array(12).fill(null));

  onMount(() => {
    const ready = charEls.filter((el): el is HTMLSpanElement => el !== null);
    if (ready.length === 12) {
      onCharsReady?.(ready);
    }
  });
</script>

<div
  class="brand-logo"
  class:macos={isMacOS}
  class:collapsed={layoutCollapsed}
  aria-hidden="true"
  bind:this={containerEl}
>
  <span class="brand-logo-slab"></span>
  <span class="brand-logo-mark">
    <span class="brand-row">
      {#each ROW1 as letter, i (i)}
        <span
          class="brand-char"
          class:outline={letter.outline}
          bind:this={charEls[i]}
        >
          <span data-logo-glyph>{letter.char}</span>
        </span>
      {/each}
    </span>
    <span class="brand-row">
      {#each ROW2 as letter, i (i)}
        <span
          class="brand-char"
          class:outline={letter.outline}
          bind:this={charEls[i + 6]}
        >
          <span data-logo-glyph>{letter.char}</span>
        </span>
      {/each}
    </span>
  </span>
</div>

<style>
  .brand-logo {
    --brand-logo-slab-left: 0px;
    --brand-logo-slab-right: 8px;
    --brand-logo-collapsed-slab-right: 10px;
    --brand-logo-collapsed-char-line-height: calc(0.88em - 2px);
    --brand-logo-glyph-size: 22px;
    --brand-logo-char-gap: 0px;
    --brand-logo-mark-offset-x: 0px;
    --brand-logo-slab-top: calc(20px + var(--safe-area-top));
    --brand-logo-slab-bottom: 12px;

    position: relative;
    display: flex;
    flex-direction: column;
    padding: 20px 8px 12px 10px;
    padding-top: calc(20px + var(--safe-area-top));
    line-height: 1;
    user-select: none;
    -webkit-user-select: none;
  }

  .brand-logo.collapsed {
    --brand-logo-slab-left: 0px;
    --brand-logo-slab-right: var(--brand-logo-collapsed-slab-right);
    --brand-logo-mark-offset-x: calc(
      var(--brand-logo-collapsed-slab-right) * -0.5
    );

    align-items: center;
    padding: 20px 0 12px;
    padding-top: calc(20px + var(--safe-area-top));
    flex-direction: column-reverse;
  }

  .brand-logo-slab {
    position: absolute;
    z-index: 0;
    top: var(--brand-logo-slab-top);
    right: var(--brand-logo-slab-right);
    bottom: var(--brand-logo-slab-bottom);
    left: var(--brand-logo-slab-left);
    border-radius: 0 8px 8px 0;
    background: var(--accent);
    pointer-events: none;
  }

  .brand-logo-mark {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 8px 10px 7px;
    row-gap: var(--brand-logo-char-gap);
  }

  .brand-row {
    display: flex;
    flex-direction: row;
    font-family: var(--font-wide);
    gap: var(--brand-logo-char-gap);
    font-size: var(--brand-logo-glyph-size);
    font-weight: 700;
    line-height: 0.88;
    color: var(--theme-text-primary);
    white-space: nowrap;
  }

  .collapsed .brand-logo-mark {
    flex-direction: column-reverse;
    transform: translateX(var(--brand-logo-mark-offset-x));
  }

  .collapsed .brand-row {
    flex-direction: column-reverse;
    align-items: center;
    line-height: var(--brand-logo-collapsed-char-line-height);
  }

  .brand-char {
    display: inline-block;
  }

  .brand-char [data-logo-glyph] {
    display: inline-block;
  }

  .brand-char.outline {
    color: transparent;
    -webkit-text-stroke: 1.5px var(--theme-tint);
  }
</style>
