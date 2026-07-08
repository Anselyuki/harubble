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
    --brand-logo-collapsed-slab-right: 10px;
    --brand-logo-collapsed-char-line-height: calc(0.88em - 2px);
    --brand-logo-glyph-size: 22px;
    --brand-logo-char-gap: 0px;
    --brand-logo-mark-offset-x: 0px;

    grid-area: 1 / 1;
    z-index: 1;
    display: flex;
    flex-direction: column;
    padding: 20px 8px 12px 10px;
    padding-top: calc(20px + var(--safe-area-top));
    line-height: 1;
    user-select: none;
    -webkit-user-select: none;
  }

  .brand-logo.collapsed {
    --brand-logo-mark-offset-x: calc(
      var(--brand-logo-collapsed-slab-right) * -0.5
    );

    /* 折叠态把 logo 固定在折叠宽度（56px）的左侧足迹内居中，使其位置
       不随 brand-region 宽度（= --sidebar-width）变化。否则拖曳展开时
       brand-region 变宽，居中参考随之右移，竖向 logo 会跟着侧栏漂移，
       而左锚定的 slab 不动，二者错位。固定足迹后未触发 FLIP 时 logo 不动。 */
    width: var(--brand-collapsed-width, 56px);
    justify-self: start;

    align-items: center;
    padding: 20px 0 12px;
    padding-top: calc(20px + var(--safe-area-top));
    flex-direction: column-reverse;
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
    color: var(--accent-readable-foreground);
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
    -webkit-text-stroke: 1.2px
      color-mix(in srgb, var(--accent-readable-foreground) 40%, transparent);
  }
</style>
