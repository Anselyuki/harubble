<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import {
    animateIn,
    animateOut,
    getMotionDuration,
    gsap,
    killTweens,
    MOTION,
  } from '$lib/design/gsap';
  import { cn } from '$lib/utils.js';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
  import {
    THEME_COLOR_SLOTS,
    type ColorScheme,
    type ThemeColorSlot,
    type ThemeColorSlots,
  } from '$lib/themePresets';

  interface ThemePresetOption {
    id: string;
    label: string;
    description: string;
    colors: ThemeColorSlots;
  }

  interface Props {
    colorScheme?: ColorScheme;
    dynamicAlbumAccent?: boolean;
    packageColorsLocked?: boolean;
    themePresetId: string;
    resolvedThemeColors: ThemeColorSlots;
    themePresetOptions: ThemePresetOption[];
    currentThemePresetLabel: string;
    getThemeDraft: (slot: ThemeColorSlot) => string;
    getSlotLabel: (slot: ThemeColorSlot) => string;
    isValidThemeHex: (value: string) => boolean;
    sectionTitle: string;
    themePresetLabel: string;
    themeResetLabel: string;
    themeResetTitle: string;
    themeHexInvalidLabel: string;
    appearanceLabel: string;
    appearanceAutoLabel: string;
    appearanceLightLabel: string;
    appearanceDarkLabel: string;
    appearanceSegmentAria: string;
    dynamicAlbumLabel: string;
    dynamicAlbumOnLabel: string;
    dynamicAlbumOffLabel: string;
    onThemePresetChange: (nextPresetId: string) => void;
    onThemeTextInput: (slot: ThemeColorSlot, value: string) => void;
    onThemeColorInput: (slot: ThemeColorSlot, value: string) => void;
    onResetThemeCustomColors: () => void;
  }

  let {
    colorScheme = $bindable<ColorScheme>('auto'),
    dynamicAlbumAccent = $bindable<boolean>(true),
    packageColorsLocked = false,
    themePresetId,
    resolvedThemeColors,
    themePresetOptions,
    currentThemePresetLabel,
    getThemeDraft,
    getSlotLabel,
    isValidThemeHex,
    sectionTitle,
    themePresetLabel,
    themeResetLabel,
    themeResetTitle,
    themeHexInvalidLabel,
    appearanceLabel,
    appearanceAutoLabel,
    appearanceLightLabel,
    appearanceDarkLabel,
    appearanceSegmentAria,
    dynamicAlbumLabel,
    dynamicAlbumOnLabel,
    dynamicAlbumOffLabel,
    onThemePresetChange,
    onThemeTextInput,
    onThemeColorInput,
    onResetThemeCustomColors,
  }: Props = $props();

  let colorPanelEl = $state<HTMLDivElement | undefined>();
  let appearanceSegmentEl = $state<HTMLDivElement | undefined>();
  let presetSelectOpen = $state(false);
  // Initial package hydration should render the settled state. Only subsequent
  // changes, including preview/apply/clear inside Settings, animate.
  const initialPackageColorsLocked = untrack(() => packageColorsLocked);
  let colorPanelMounted = $state(!initialPackageColorsLocked);
  let previousPackageColorsLocked = initialPackageColorsLocked;
  let animationSequence = 0;

  function focusSelectedAppearanceMode(): void {
    appearanceSegmentEl
      ?.querySelector<HTMLButtonElement>('[aria-pressed="true"]')
      ?.focus({ preventScroll: true });
  }

  async function updateColorPanelVisibility(
    show: boolean,
    sequence: number,
    freshlyMounted: boolean,
    restoreAppearanceFocus: boolean
  ): Promise<void> {
    await tick();
    if (sequence !== animationSequence) return;

    if (restoreAppearanceFocus) {
      focusSelectedAppearanceMode();
    }

    if (show) {
      const panel = colorPanelEl;
      if (!panel) return;

      const target = {
        height: 'auto',
        marginTop: 12,
        opacity: 1,
        y: 0,
        clipPath: 'inset(0 0 0% 0)',
        overflow: 'hidden',
        onComplete: () => {
          if (sequence !== animationSequence) return;
          gsap.set(panel, {
            clearProps: 'height,marginTop,opacity,transform,clipPath,overflow',
          });
        },
      };

      if (freshlyMounted) {
        animateIn(
          panel,
          {
            height: 0,
            marginTop: 0,
            opacity: 0,
            y: -8,
            clipPath: 'inset(0 0 100% 0)',
            overflow: 'hidden',
          },
          target,
          MOTION.SLOW,
          'ios-out'
        );
      } else {
        // Continue from the current collapse frame during rapid reversals.
        killTweens(panel);
        gsap.to(panel, {
          ...target,
          duration: getMotionDuration(MOTION.SLOW),
          ease: 'ios-out',
        });
      }
      return;
    }

    const panel = colorPanelEl;
    if (!panel) {
      colorPanelMounted = false;
      return;
    }

    animateOut(
      panel,
      {
        height: 0,
        marginTop: 0,
        opacity: 0,
        y: -8,
        clipPath: 'inset(0 0 100% 0)',
        overflow: 'hidden',
      },
      MOTION.SLOW_OUT,
      {
        ease: 'ios-in',
        onComplete: () => {
          if (sequence === animationSequence && packageColorsLocked) {
            colorPanelMounted = false;
          }
        },
      }
    );
  }

  $effect.pre(() => {
    const locked = packageColorsLocked;
    if (locked === previousPackageColorsLocked) return;
    previousPackageColorsLocked = locked;

    const sequence = ++animationSequence;
    const freshlyMounted = !colorPanelMounted;
    let restoreAppearanceFocus = false;

    if (locked) {
      const activeElement = document.activeElement;
      restoreAppearanceFocus =
        presetSelectOpen ||
        (activeElement instanceof HTMLElement &&
          Boolean(colorPanelEl?.contains(activeElement)));

      // Select.Content is portalled outside this panel, so close it before
      // inert/disabled state is rendered.
      presetSelectOpen = false;
      if (restoreAppearanceFocus) {
        focusSelectedAppearanceMode();
      }
    } else {
      colorPanelMounted = true;
    }

    void updateColorPanelVisibility(
      !locked,
      sequence,
      freshlyMounted,
      restoreAppearanceFocus
    );
  });

  onDestroy(() => {
    animationSequence += 1;
    if (colorPanelEl) killTweens(colorPanelEl);
  });
</script>

<section
  class="sheet-section settings-section settings-theme-section"
  data-package-colors-hidden={packageColorsLocked}
>
  <div class="settings-section-heading settings-theme-heading">
    <h3>{sectionTitle}</h3>
  </div>
  <div class="settings-field settings-appearance-field">
    <span>{appearanceLabel}</span>
    <div
      class="settings-segment"
      aria-label={appearanceSegmentAria}
      bind:this={appearanceSegmentEl}
    >
      <button
        type="button"
        data-settings-theme-focus-target={colorScheme === 'auto'
          ? ''
          : undefined}
        class:active={colorScheme === 'auto'}
        aria-pressed={colorScheme === 'auto'}
        onclick={() => (colorScheme = 'auto')}>{appearanceAutoLabel}</button
      >
      <button
        type="button"
        data-settings-theme-focus-target={colorScheme === 'light'
          ? ''
          : undefined}
        class:active={colorScheme === 'light'}
        aria-pressed={colorScheme === 'light'}
        onclick={() => (colorScheme = 'light')}>{appearanceLightLabel}</button
      >
      <button
        type="button"
        data-settings-theme-focus-target={colorScheme === 'dark'
          ? ''
          : undefined}
        class:active={colorScheme === 'dark'}
        aria-pressed={colorScheme === 'dark'}
        onclick={() => (colorScheme = 'dark')}>{appearanceDarkLabel}</button
      >
    </div>
  </div>

  {#if colorPanelMounted}
    <div
      class="settings-theme-color-panel"
      data-testid="theme-color-settings-panel"
      aria-hidden={packageColorsLocked}
      inert={packageColorsLocked}
      bind:this={colorPanelEl}
    >
      <div class="settings-theme-color-panel-heading">
        <label for="theme-preset-select">{themePresetLabel}</label>
        <Button
          variant="secondary"
          class="h-8"
          title={themeResetTitle}
          disabled={packageColorsLocked}
          onclick={onResetThemeCustomColors}
        >
          <RotateCcwIcon data-icon="inline-start" />{themeResetLabel}
        </Button>
      </div>

      <Select.Root
        type="single"
        bind:open={presetSelectOpen}
        value={themePresetId}
        onValueChange={onThemePresetChange}
        disabled={packageColorsLocked}
      >
        <Select.Trigger
          id="theme-preset-select"
          class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
        >
          {currentThemePresetLabel}
        </Select.Trigger>
        <Select.Content class="sheet-select-content">
          {#each themePresetOptions as preset (preset.id)}
            <Select.Item value={preset.id} label={preset.label}>
              <div class="settings-theme-preset-option">
                <div class="settings-theme-preset-copy">
                  <strong>{preset.label}</strong>
                  <small>{preset.description}</small>
                </div>
                <div class="settings-theme-swatch-strip" aria-hidden="true">
                  {#each THEME_COLOR_SLOTS as slot (slot)}
                    <span style={`--swatch-color: ${preset.colors[slot]}`}
                    ></span>
                  {/each}
                </div>
              </div>
            </Select.Item>
          {/each}
        </Select.Content>
      </Select.Root>

      <div class="settings-theme-color-list">
        {#each THEME_COLOR_SLOTS as slot (slot)}
          {@const draft = getThemeDraft(slot)}
          {@const invalid = draft.length > 0 && !isValidThemeHex(draft)}
          {@const invalidHelpId = `theme-color-${slot}-error`}
          <label class="settings-theme-color-row" for={`theme-color-${slot}`}>
            <span
              class="settings-theme-color-swatch"
              style={`--swatch-color: ${resolvedThemeColors[slot]}`}
            ></span>
            <span class="settings-theme-color-label">{getSlotLabel(slot)}</span>
            <input
              id={`theme-color-${slot}`}
              class={cn(
                'border-input focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 aria-invalid:border-destructive h-8 rounded-lg border bg-transparent px-2.5 py-1 text-base focus-visible:ring-3 aria-invalid:ring-3 md:text-sm w-full min-w-0 outline-none',
                'settings-theme-hex-input h-8 border-[var(--sheet-border)] bg-[var(--sheet-control-bg)]'
              )}
              value={draft}
              disabled={packageColorsLocked}
              aria-invalid={invalid}
              aria-describedby={invalid ? invalidHelpId : undefined}
              oninput={(event) =>
                onThemeTextInput(slot, event.currentTarget.value)}
            />
            <input
              class="settings-theme-native-color"
              type="color"
              value={resolvedThemeColors[slot]}
              disabled={packageColorsLocked}
              aria-label={getSlotLabel(slot)}
              oninput={(event) =>
                onThemeColorInput(slot, event.currentTarget.value)}
            />
            {#if invalid}
              <small id={invalidHelpId} class="settings-theme-invalid"
                >{themeHexInvalidLabel}</small
              >
            {/if}
          </label>
        {/each}
      </div>
    </div>
  {/if}

  <div class="settings-field settings-appearance-field">
    <span>{dynamicAlbumLabel}</span>
    <div class="settings-segment" aria-label={dynamicAlbumLabel}>
      <button
        type="button"
        class:active={dynamicAlbumAccent === true}
        aria-pressed={dynamicAlbumAccent === true}
        onclick={() => (dynamicAlbumAccent = true)}
        >{dynamicAlbumOnLabel}</button
      >
      <button
        type="button"
        class:active={dynamicAlbumAccent === false}
        aria-pressed={dynamicAlbumAccent === false}
        onclick={() => (dynamicAlbumAccent = false)}
        >{dynamicAlbumOffLabel}</button
      >
    </div>
  </div>
</section>

<style>
  .settings-theme-heading {
    align-items: center;
  }
  .settings-theme-section {
    gap: 0;
  }
  .settings-theme-section > .settings-appearance-field,
  .settings-theme-section > .settings-theme-color-panel {
    margin-top: 12px;
  }
  .settings-theme-color-panel {
    display: grid;
    gap: 12px;
    min-width: 0;
    transform-origin: 50% 0;
    will-change: height, opacity, transform, clip-path;
  }
  .settings-theme-color-panel[aria-hidden='true'] {
    pointer-events: none;
  }
  .settings-theme-color-panel-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-width: 0;
  }
  .settings-theme-color-panel-heading label {
    min-width: 0;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
  }
  .settings-theme-color-panel-heading :global([data-slot='button']) {
    flex-shrink: 0;
  }
  .settings-theme-preset-option {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .settings-theme-preset-copy {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .settings-theme-preset-copy strong {
    overflow: hidden;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 700;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .settings-theme-preset-copy small {
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .settings-theme-swatch-strip {
    display: grid;
    grid-template-columns: repeat(6, 10px);
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-pill);
  }
  .settings-theme-swatch-strip span,
  .settings-theme-color-swatch {
    background: var(--swatch-color);
  }
  .settings-theme-swatch-strip span {
    width: 10px;
    height: 18px;
  }
  .settings-theme-color-list {
    display: grid;
    gap: 7px;
  }
  .settings-theme-color-row {
    display: grid;
    grid-template-columns: 18px minmax(5.5rem, 1fr) minmax(6.5rem, 8rem) 34px;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .settings-theme-color-swatch {
    width: 18px;
    height: 18px;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-pill);
    box-shadow: inset 0 1px 0 color-mix(in srgb, white 36%, transparent);
  }
  .settings-theme-color-label {
    min-width: 0;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.3;
  }
  :global(.settings-theme-hex-input) {
    font-family: var(--font-mono);
    font-size: 12px;
    text-transform: uppercase;
  }
  :global(.settings-theme-hex-input[aria-invalid='true']) {
    border-color: color-mix(
      in srgb,
      var(--destructive) 55%,
      var(--sheet-border)
    );
    color: var(--destructive);
  }
  .settings-theme-native-color {
    width: 34px;
    height: 32px;
    border: 1px solid var(--sheet-border);
    border-radius: 7px;
    background: var(--sheet-control-bg);
    padding: 3px;
  }
  .settings-theme-invalid {
    grid-column: 3 / 5;
    margin-top: -3px;
    color: var(--destructive);
    font-size: 11px;
    font-weight: 500;
    line-height: 1.35;
  }
  @media (max-width: 420px) {
    .settings-theme-color-row {
      grid-template-columns: 18px minmax(0, 1fr) 34px;
    }
    :global(.settings-theme-hex-input) {
      grid-column: 2 / 4;
    }
    .settings-theme-invalid {
      grid-column: 2 / 4;
    }
  }
</style>
