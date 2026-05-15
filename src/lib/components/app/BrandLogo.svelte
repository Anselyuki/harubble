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
  <span class="brand-row">
    {#each ROW1 as letter, i (i)}
      <span
        class="brand-char"
        class:outline={letter.outline}
        bind:this={charEls[i]}><span data-logo-glyph>{letter.char}</span></span
      >
    {/each}
  </span>
  <span class="brand-row">
    {#each ROW2 as letter, i (i)}
      <span
        class="brand-char"
        class:outline={letter.outline}
        bind:this={charEls[i + 6]}
        ><span data-logo-glyph>{letter.char}</span></span
      >
    {/each}
  </span>
</div>

<style>
  .brand-logo {
    display: flex;
    flex-direction: column;
    padding: 20px 24px 12px;
    line-height: 1;
    user-select: none;
    -webkit-user-select: none;
  }

  .brand-logo.macos {
    padding-top: 48px;
  }

  .brand-logo.collapsed {
    align-items: center;
    padding: 20px 0 12px;
    flex-direction: column-reverse;
  }

  .brand-logo.collapsed.macos {
    padding-top: 48px;
  }

  .brand-row {
    display: flex;
    flex-direction: row;
    font-family: var(--font-wide);
    font-size: 22px;
    font-weight: 700;
    line-height: 0.88;
    letter-spacing: 0.04em;
    color: var(--accent);
    white-space: nowrap;
  }

  .collapsed .brand-row {
    flex-direction: column-reverse;
    align-items: center;
  }

  .brand-char {
    display: inline-block;
  }

  .brand-char [data-logo-glyph] {
    display: inline-block;
  }

  .brand-char.outline {
    color: transparent;
    -webkit-text-stroke: 1.5px var(--accent);
  }
</style>
