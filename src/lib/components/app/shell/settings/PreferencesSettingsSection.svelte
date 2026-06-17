<script lang="ts">
  import * as Select from '$lib/components/ui/select/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import type { Locale } from '$lib/i18n/types';
  import type { LogLevel, OutputFormat } from '$lib/types';

  interface SelectOption<T> {
    value: T;
    label: string;
  }

  interface Props {
    locale?: Locale;
    format?: OutputFormat;
    logLevel?: LogLevel;
    outputDir?: string;
    localeOptions: SelectOption<Locale>[];
    formatOptions: SelectOption<OutputFormat>[];
    logLevelOptions: SelectOption<LogLevel>[];
    currentLocaleLabel: string;
    currentFormatLabel: string;
    currentLogLevelLabel: string;
    sectionTitle: string;
    languageLabel: string;
    outputFormatLabel: string;
    logLevelLabel: string;
    outputDirLabel: string;
    outputDirSelectLabel: string;
    onSelectDirectory: () => void;
  }

  let {
    locale = $bindable<Locale>('zh-CN'),
    format = $bindable<OutputFormat>('flac'),
    logLevel = $bindable<LogLevel>('error'),
    outputDir = '',
    localeOptions,
    formatOptions,
    logLevelOptions,
    currentLocaleLabel,
    currentFormatLabel,
    currentLogLevelLabel,
    sectionTitle,
    languageLabel,
    outputFormatLabel,
    logLevelLabel,
    outputDirLabel,
    outputDirSelectLabel,
    onSelectDirectory,
  }: Props = $props();
</script>

<section class="sheet-section settings-section">
  <div class="settings-section-heading">
    <h3>{sectionTitle}</h3>
  </div>
  <div class="settings-field-grid">
    <label class="settings-field" for="locale-select">
      <span>{languageLabel}</span>
      <Select.Root type="single" bind:value={locale}
        ><Select.Trigger
          id="locale-select"
          class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
          >{currentLocaleLabel}</Select.Trigger
        ><Select.Content class="sheet-select-content"
          >{#each localeOptions as option (option.value)}<Select.Item
              value={option.value}
              label={option.label}
            />{/each}</Select.Content
        ></Select.Root
      >
    </label>
    <label class="settings-field" for="format-select">
      <span>{outputFormatLabel}</span>
      <Select.Root type="single" bind:value={format}
        ><Select.Trigger
          id="format-select"
          class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
          >{currentFormatLabel}</Select.Trigger
        ><Select.Content class="sheet-select-content"
          >{#each formatOptions as option (option.value)}<Select.Item
              value={option.value}
              label={option.label}
            />{/each}</Select.Content
        ></Select.Root
      >
    </label>
    <label class="settings-field" for="log-level-select">
      <span>{logLevelLabel}</span>
      <Select.Root type="single" bind:value={logLevel}
        ><Select.Trigger
          id="log-level-select"
          class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
          >{currentLogLevelLabel}</Select.Trigger
        ><Select.Content class="sheet-select-content"
          >{#each logLevelOptions as option (option.value)}<Select.Item
              value={option.value}
              label={option.label}
            />{/each}</Select.Content
        ></Select.Root
      >
    </label>
    <div class="settings-field">
      <label for="output-dir">{outputDirLabel}</label>
      <div class="settings-path-row">
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              {@const { type: _type, id: _triggerId, ...triggerProps } = props}
              <Input
                {...triggerProps}
                id="output-dir"
                class="settings-path-input h-9 border-[var(--sheet-border)] bg-[var(--sheet-control-bg)]"
                readonly
                value={outputDir}
                title={outputDir}
              />
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content
            class="settings-path-tooltip"
            side="top"
            sideOffset={8}
          >
            {outputDir}
          </Tooltip.Content>
        </Tooltip.Root>
        <Button
          variant="secondary"
          class="h-9 shrink-0"
          onclick={onSelectDirectory}
          ><FolderOpenIcon
            data-icon="inline-start"
          />{outputDirSelectLabel}</Button
        >
      </div>
    </div>
  </div>
</section>

<style>
  .settings-field-grid {
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  .settings-path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    min-width: 0;
  }
  :global(.settings-path-input) {
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  :global(.settings-path-tooltip) {
    z-index: 220;
    max-width: min(28rem, calc(100vw - 32px));
    overflow-wrap: anywhere;
    line-height: 1.45;
    text-align: left;
    white-space: normal;
  }
  @media (max-width: 420px) {
    .settings-path-row {
      grid-template-columns: 1fr;
    }
  }
</style>
