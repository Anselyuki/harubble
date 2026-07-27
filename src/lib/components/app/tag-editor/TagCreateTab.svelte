<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { gsap } from '$lib/design/gsap';
  import type { TagLocale } from './tagAddUtils';

  interface Props {
    createValues: Record<string, string>;
    createI18n: boolean;
    tagLocales: TagLocale[];
    onCreate: () => void;
  }

  let {
    createValues = $bindable(),
    createI18n = $bindable(),
    tagLocales,
    onCreate,
  }: Props = $props();

  let localeCardsEl: HTMLElement | undefined = $state();

  let prevI18n: boolean | null = null;
  let localeCardsTween: gsap.core.Tween | null = null;

  $effect(() => {
    const show = createI18n;
    if (!localeCardsEl) {
      prevI18n = show;
      return;
    }
    if (prevI18n === null) {
      prevI18n = show;
      localeCardsEl.style.gridTemplateRows = show ? '1fr' : '0fr';
      localeCardsEl.style.opacity = show ? '1' : '0';
      localeCardsEl.style.marginTop = show ? '0' : '-8px';
      return;
    }
    if (show === prevI18n) return;
    prevI18n = show;
    if (localeCardsTween) localeCardsTween.kill();
    const el = localeCardsEl;
    const proxy = { v: show ? 0 : 1 };
    localeCardsTween = gsap.to(proxy, {
      v: show ? 1 : 0,
      duration: show ? 0.25 : 0.2,
      ease: show ? 'ios-out' : 'ios-in',
      onUpdate: () => {
        el.style.gridTemplateRows = `${proxy.v}fr`;
        el.style.opacity = `${proxy.v}`;
        el.style.marginTop = `${-8 * (1 - proxy.v)}px`;
      },
    });
    return () => {
      if (localeCardsTween) {
        localeCardsTween.kill();
        localeCardsTween = null;
      }
    };
  });
</script>

<div class="create-form">
  <div class="create-input-wrapper">
    <input
      value={createValues[getLocale()]}
      oninput={(e) => {
        createValues[getLocale()] = (e.target as HTMLInputElement).value;
      }}
      class="form-input create-main-input"
      placeholder={m.tag_editor_create_name_placeholder()}
      disabled={createI18n}
    />
    <button
      type="button"
      class="i18n-toggle-inset"
      class:on={createI18n}
      onclick={() => (createI18n = !createI18n)}
      aria-pressed={createI18n}
      aria-label={m.tag_editor_create_i18n_toggle()}
      title={m.tag_editor_create_i18n_toggle()}
    >
      <span class="i18n-toggle-label"
        >{createI18n
          ? m.tag_editor_i18n_mode_multi()
          : m.tag_editor_i18n_mode_single()}</span
      >
    </button>
  </div>

  <div class="locale-cards" bind:this={localeCardsEl}>
    <div class="locale-cards-inner">
      {#each tagLocales as loc, idx (loc.key)}
        <div class="locale-card">
          <span class="locale-card-bar" style="opacity: {1 - idx * 0.2}"
            >{loc.label}</span
          >
          <input
            value={createValues[loc.key] ?? ''}
            oninput={(e) => {
              createValues[loc.key] = (e.target as HTMLInputElement).value;
            }}
            class="locale-card-input"
          />
        </div>
      {/each}
    </div>
  </div>

  <Button
    size="xs"
    variant="secondary"
    disabled={!tagLocales.some((loc) => (createValues[loc.key] ?? '').trim())}
    onclick={onCreate}>{m.tag_editor_create_button()}</Button
  >
</div>

<style>
  .form-input {
    width: 100%;
    padding: 0.375rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: var(--shape-sm);
    background: var(--bg-input, transparent);
    color: var(--text-primary);
    outline: none;
    font-family: var(--font-body);
  }

  .form-input:focus {
    border-color: var(--color-primary, #6366f1);
  }

  .create-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .create-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .create-main-input {
    flex: 1;
    padding: 8px 36px 8px 10px;
    border-radius: var(--shape-md);
  }

  .create-main-input:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .i18n-toggle-inset {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border-radius: var(--shape-pill);
    border: none;
    background: none;
    cursor: pointer;
    padding: 2px 6px 2px 4px;
    flex-shrink: 0;
  }

  .i18n-toggle-inset::before {
    content: '';
    display: block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-border, #d1d5db);
    flex-shrink: 0;
  }

  .i18n-toggle-inset.on::before {
    background: var(--color-primary, #6366f1);
    box-shadow: 0 0 6px 1px
      color-mix(in srgb, var(--color-primary, #6366f1) 50%, transparent);
  }

  .i18n-toggle-label {
    font-size: 0.6rem;
    font-weight: 500;
    color: var(--text-primary);
    font-family: var(--font-mono);
    line-height: 1;
  }

  .i18n-toggle-inset.on .i18n-toggle-label {
    color: var(--color-primary, #6366f1);
  }

  .locale-cards {
    display: grid;
    grid-template-rows: 0fr;
    margin: -8px -6px 0;
    opacity: 0;
  }

  .locale-cards-inner {
    overflow: hidden;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0 6px;
  }

  .locale-card {
    display: flex;
    align-items: center;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--shape-pill);
    padding: 4px;
  }

  .locale-card-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    min-width: 24px;
    height: 22px;
    font-size: 0.6rem;
    font-weight: 600;
    color: white;
    background: var(--color-primary, #6366f1);
    border-radius: var(--shape-pill);
    font-family: var(--font-body);
  }

  .locale-card-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    padding: 4px 8px;
    font-size: 0.75rem;
    font-family: var(--font-body);
    color: var(--text-primary);
  }

  .locale-card:focus-within {
    border-color: var(--color-primary, #6366f1);
    box-shadow: 0 0 6px 0
      color-mix(in srgb, var(--color-primary, #6366f1) 25%, transparent);
  }
</style>
