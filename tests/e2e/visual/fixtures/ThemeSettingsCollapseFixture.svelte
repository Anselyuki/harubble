<script lang="ts">
  import ThemeSettingsSection from '$lib/components/app/shell/settings/ThemeSettingsSection.svelte';
  import {
    DEFAULT_THEME_PRESET_ID,
    HARUBBLE_CLASSIC_COLORS,
    THEME_PRESETS,
    isValidThemeHex,
    type ColorScheme,
    type ThemeColorSlot,
    type ThemeColorSlots,
  } from '$lib/themePresets';

  let packageColorsLocked = $state(false);
  let colorScheme = $state<ColorScheme>('auto');
  let dynamicAlbumAccent = $state(true);
  let themePresetId = $state(DEFAULT_THEME_PRESET_ID);
  let themeColors = $state<ThemeColorSlots>({ ...HARUBBLE_CLASSIC_COLORS });

  const presetOptions = THEME_PRESETS.map((preset) => ({
    ...preset,
    label: preset.id,
    description: `${preset.id} preset`,
  }));

  function updateColor(slot: ThemeColorSlot, value: string): void {
    if (isValidThemeHex(value)) {
      themeColors[slot] = value.toUpperCase();
    }
  }
</script>

<main class="qa-settings-shell app-side-sheet" data-side="right">
  <div class="qa-controls" aria-label="Theme package state">
    <button
      type="button"
      data-testid="qa-activate-package"
      aria-pressed={packageColorsLocked}
      onclick={() => (packageColorsLocked = true)}>Activate package</button
    >
    <button
      type="button"
      data-testid="qa-clear-package"
      aria-pressed={!packageColorsLocked}
      onclick={() => (packageColorsLocked = false)}>Clear package</button
    >
  </div>

  <ThemeSettingsSection
    bind:colorScheme
    bind:dynamicAlbumAccent
    {packageColorsLocked}
    {themePresetId}
    resolvedThemeColors={themeColors}
    themePresetOptions={presetOptions}
    currentThemePresetLabel={themePresetId}
    getThemeDraft={(slot) => themeColors[slot]}
    getSlotLabel={(slot) => slot}
    {isValidThemeHex}
    sectionTitle="Theme color"
    themePresetLabel="Preset"
    themeResetLabel="Reset"
    themeResetTitle="Reset theme colors"
    themeHexInvalidLabel="Use #RRGGBB"
    appearanceLabel="Appearance"
    appearanceAutoLabel="Auto"
    appearanceLightLabel="Light"
    appearanceDarkLabel="Dark"
    appearanceSegmentAria="Appearance mode"
    dynamicAlbumLabel="Dynamic album color"
    dynamicAlbumOnLabel="On"
    dynamicAlbumOffLabel="Off"
    onThemePresetChange={(id) => (themePresetId = id)}
    onThemeTextInput={updateColor}
    onThemeColorInput={updateColor}
    onResetThemeCustomColors={() =>
      (themeColors = { ...HARUBBLE_CLASSIC_COLORS })}
  />
</main>

<style>
  :global(body) {
    display: grid;
    place-items: start center;
    overflow: auto;
    min-height: 100vh;
    margin: 0;
    padding: 24px;
    background: var(--bg-primary);
  }

  .qa-settings-shell {
    display: grid;
    gap: 12px;
    width: min(100%, 420px);
  }

  .qa-controls {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .qa-controls button {
    min-height: 40px;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-md);
    background: var(--sheet-row-bg);
    color: var(--text-primary);
    font: inherit;
    cursor: pointer;
  }

  .qa-controls button[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-readable-foreground);
  }

  :global(.settings-section) {
    gap: 12px;
  }

  :global(.settings-section-heading) {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  :global(.settings-section-heading h3) {
    margin: 0;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0;
  }

  :global(.settings-field) {
    display: grid;
    gap: 6px;
    min-width: 0;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
  }

  :global(.settings-segment) {
    display: inline-grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-md);
    background: var(--sheet-row-bg);
    padding: 2px;
  }

  :global(.settings-segment button) {
    height: 26px;
    padding-inline: 10px;
    border: 0;
    border-radius: var(--shape-sm);
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  :global(.settings-segment button.active) {
    background: var(--accent);
    color: var(--accent-readable-foreground);
  }

  @media (max-width: 480px) {
    :global(body) {
      padding: 12px;
    }
  }
</style>
