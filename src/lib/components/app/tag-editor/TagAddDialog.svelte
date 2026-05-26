<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { locales, getLocale } from '$lib/paraglide/runtime.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { gsap } from '$lib/design/gsap';
  import type { TagEditorLocalizedValue } from '$lib/types';
  import { type TagLibrary } from '$lib/features/tagEditor/tagLibrary';
  import { calcExpandDirection, calcCardPosition } from './popoverBubble';

  interface Props {
    open: boolean;
    dimensionKey: string;
    dimensionLabel: string;
    values: TagEditorLocalizedValue[];
    tagLibrary: TagLibrary;
    onSetTag: (
      dimensionKey: string,
      values: TagEditorLocalizedValue[]
    ) => Promise<void>;
    onRemoveTag: (dimensionKey: string) => Promise<void>;
    onOpenChange: (open: boolean) => void;
  }

  let {
    open = $bindable(),
    dimensionKey,
    dimensionLabel,
    values,
    tagLibrary,
    onSetTag,
    onRemoveTag,
    onOpenChange,
  }: Props = $props();

  type TabMode = 'search' | 'create';
  let activeTab = $state<TabMode>('search');
  let searchQuery = $state('');
  let createValues = $state<Record<string, string>>({});
  let createI18n = $state(false);
  let editingTag = $state<TagEditorLocalizedValue | null>(null);
  let editValues = $state<Record<string, string>>({});
  let cardEl: HTMLElement | undefined = $state();
  let tabContentEl: HTMLElement | undefined = $state();
  let localeCardsEl: HTMLElement | undefined = $state();
  let sliderEl: HTMLElement | undefined = $state();
  let cardTop = $state(0);
  let cardLeft = $state(0);

  const localeDisplayNames = new Intl.DisplayNames([getLocale()], {
    type: 'language',
    languageDisplay: 'standard',
    style: 'narrow',
  });

  const TAG_LOCALES = locales.map((loc) => ({
    key: loc,
    label: localeDisplayNames.of(loc.split('-')[0]) ?? loc,
  }));

  let allDimensionTags = $derived(tagLibrary[dimensionKey] ?? []);
  let searchResults = $derived(
    searchQuery.trim()
      ? allDimensionTags.filter((v) => {
          const q = searchQuery.trim().toLowerCase();
          const zh = (v['zh-CN'] ?? '').toLowerCase();
          const en = (v['en-US'] ?? '').toLowerCase();
          return zh.includes(q) || en.includes(q);
        })
      : allDimensionTags
  );

  function displayValue(val: TagEditorLocalizedValue): string {
    return val['zh-CN'] || val['en-US'] || Object.values(val)[0] || '';
  }

  function tagIdentity(val: TagEditorLocalizedValue): string {
    return Object.keys(val)
      .sort()
      .map((k) => `${k}=${val[k]}`)
      .join('\0');
  }

  function isAlreadyAdded(val: TagEditorLocalizedValue): boolean {
    const id = tagIdentity(val);
    return values.some((v) => tagIdentity(v) === id);
  }

  async function handleSelectCandidate(val: TagEditorLocalizedValue) {
    if (isAlreadyAdded(val)) return;
    await onSetTag(dimensionKey, [...values, val]);
  }

  function startEdit(val: TagEditorLocalizedValue) {
    editingTag = val;
    editValues = {};
    for (const loc of TAG_LOCALES) {
      editValues[loc.key] = val[loc.key] ?? '';
    }
  }

  function cancelEdit() {
    editingTag = null;
    editValues = {};
  }

  async function handleEditSave() {
    if (!editingTag) return;
    const hasAnyValue = TAG_LOCALES.some((loc) => editValues[loc.key].trim());
    if (!hasAnyValue) return;
    const originalId = tagIdentity(editingTag);
    const updated: TagEditorLocalizedValue = {};
    for (const loc of TAG_LOCALES) {
      const v = editValues[loc.key].trim();
      if (v) updated[loc.key] = v;
    }

    const idx = values.findIndex((v) => tagIdentity(v) === originalId);
    if (idx >= 0) {
      const newValues = values.map((v, i) => (i === idx ? updated : v));
      await onSetTag(dimensionKey, newValues);
    }
    cancelEdit();
  }

  async function handleEditAndAdd() {
    if (!editingTag) return;
    const hasAnyValue = TAG_LOCALES.some((loc) => editValues[loc.key].trim());
    if (!hasAnyValue) return;
    const updated: TagEditorLocalizedValue = {};
    for (const loc of TAG_LOCALES) {
      const v = editValues[loc.key].trim();
      if (v) updated[loc.key] = v;
    }

    if (!isAlreadyAdded(updated)) {
      await onSetTag(dimensionKey, [...values, updated]);
    }
    cancelEdit();
  }

  async function handleCreate() {
    if (!TAG_LOCALES.some((loc) => (createValues[loc.key] ?? '').trim()))
      return;
    const newVal: TagEditorLocalizedValue = {};
    for (const loc of TAG_LOCALES) {
      const v = (createValues[loc.key] ?? '').trim();
      if (v) newVal[loc.key] = v;
    }
    await onSetTag(dimensionKey, [...values, newVal]);
    createValues = {};
  }

  async function handleRemoveValue(index: number) {
    const updated = values.filter((_, i) => i !== index);
    if (updated.length === 0) {
      await onRemoveTag(dimensionKey);
    } else {
      await onSetTag(dimensionKey, updated);
    }
  }

  function resetState() {
    searchQuery = '';
    createValues = {};
    createI18n = false;
    editingTag = null;
    editValues = {};
    activeTab = 'search';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (editingTag) {
        cancelEdit();
      } else {
        closeCard();
      }
    }
    if (e.key === 'Enter' && activeTab === 'search' && searchQuery.trim()) {
      const first = searchResults.find((v) => !isAlreadyAdded(v));
      if (first) void handleSelectCandidate(first);
    }
    if (e.key === 'Enter' && activeTab === 'create') {
      e.stopPropagation();
      void handleCreate();
    }
  }

  function closeCard() {
    resetState();
    onOpenChange(false);
  }

  let clickX = $state(0);
  let clickY = $state(0);

  function handleTriggerClick(e: MouseEvent) {
    e.stopPropagation();
    clickX = e.clientX;
    clickY = e.clientY;
    const viewport = { width: window.innerWidth, height: window.innerHeight };
    const direction = calcExpandDirection(
      e.clientX,
      e.clientY,
      viewport,
      targetWidth
    );
    const pos = calcCardPosition(e.clientX, e.clientY, direction, targetWidth);
    cardTop = pos.top;
    cardLeft = pos.left;
    onOpenChange(true);
  }

  let openedAt = 0;

  function handleOutsideClick(e: MouseEvent) {
    if (Date.now() - openedAt < 100) return;
    if (cardEl && !cardEl.contains(e.target as Node)) {
      closeCard();
    }
  }

  $effect(() => {
    if (open) {
      openedAt = Date.now();
      requestAnimationFrame(() => {
        document.addEventListener('pointerdown', handleOutsideClick);
      });
      return () =>
        document.removeEventListener('pointerdown', handleOutsideClick);
    }
  });

  let hasMounted = false;
  $effect(() => {
    const tab = activeTab;
    if (!tabContentEl || !hasMounted) {
      hasMounted = true;
      return;
    }
    const fromRight = tab === 'create';
    gsap.fromTo(
      tabContentEl,
      { opacity: 0, x: fromRight ? 12 : -12 },
      { opacity: 1, x: 0, duration: 0.22, ease: 'ios-out' }
    );
  });

  $effect(() => {
    if (!sliderEl) return;
    const isRight = activeTab === 'create';
    gsap.to(sliderEl, {
      x: isRight ? '100%' : '0%',
      duration: hasMounted ? 0.2 : 0,
      ease: 'ios-spring',
    });
  });

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
    const proxy = { v: show ? 0 : 1 };
    localeCardsTween = gsap.to(proxy, {
      v: show ? 1 : 0,
      duration: show ? 0.25 : 0.2,
      ease: show ? 'ios-out' : 'ios-in',
      onUpdate: () => {
        localeCardsEl!.style.gridTemplateRows = `${proxy.v}fr`;
        localeCardsEl!.style.opacity = `${proxy.v}`;
        localeCardsEl!.style.marginTop = `${-8 * (1 - proxy.v)}px`;
      },
    });
  });

  const targetWidth = 220;

  function bubbleEnter(el: HTMLElement, params: { x: number; y: number }) {
    const contentEl = el.querySelector('.bubble-content') as HTMLElement | null;
    const startTop = params.y - 12;
    const startLeft = params.x - 12;
    gsap.fromTo(
      el,
      {
        top: startTop,
        left: startLeft,
        width: 24,
        height: 24,
        borderRadius: 12,
      },
      {
        top: cardTop,
        left: cardLeft,
        width: targetWidth,
        height: 'auto',
        borderRadius: 10,
        duration: 0.32,
        ease: 'ios-out',
      }
    );
    if (contentEl) {
      gsap.fromTo(
        contentEl,
        { opacity: 0 },
        { opacity: 1, duration: 0.2, delay: 0.16, ease: 'ios-out' }
      );
    }
  }
</script>

<button type="button" class="add-trigger" onclick={handleTriggerClick}>
  {#if values.length === 0}
    <span class="add-trigger-placeholder"
      >+ {m.tag_editor_add_tag_button()}</span
    >
  {:else}
    <span class="add-trigger-icon">+</span>
  {/if}
</button>

{#if open}
  <div
    class="bubble-card"
    bind:this={cardEl}
    use:bubbleEnter={{ x: clickX, y: clickY }}
    style="top: {cardTop}px; left: {cardLeft}px;"
    onkeydown={handleKeydown}
    role="dialog"
    tabindex="-1"
  >
    <div class="bubble-content">
      <div class="popover-header">
        <span class="popover-title">{dimensionLabel}</span>
        <div class="tab-toggle">
          <span class="toggle-slider" bind:this={sliderEl}></span>
          <button
            type="button"
            class="toggle-item"
            class:active={activeTab === 'search'}
            onclick={() => (activeTab = 'search')}
            >{m.tag_editor_tab_search()}</button
          >
          <button
            type="button"
            class="toggle-item"
            class:active={activeTab === 'create'}
            onclick={() => (activeTab = 'create')}
            >{m.tag_editor_tab_create()}</button
          >
        </div>
      </div>

      {#if values.length > 0}
        <div class="selected-tags">
          {#each values as val, idx (tagIdentity(val))}
            <span class="selected-chip">
              {displayValue(val)}
              <button
                type="button"
                class="chip-remove"
                onclick={() => handleRemoveValue(idx)}
                aria-label={m.tag_editor_remove_value_aria()}>×</button
              >
            </span>
          {/each}
        </div>
      {/if}

      <!-- Search Tab -->
      <div class="tab-content" bind:this={tabContentEl}>
        {#if activeTab === 'search'}
          <input
            bind:value={searchQuery}
            placeholder={m.tag_editor_search_placeholder()}
            class="form-input search-input"
          />

          <div class="candidates">
            {#if searchResults.length > 0}
              <div class="candidates-list">
                {#each searchResults as val (`${val['zh-CN'] ?? ''}-${val['en-US'] ?? ''}`)}
                  {#if editingTag && tagIdentity(editingTag) === tagIdentity(val)}
                    <div class="edit-row">
                      {#each TAG_LOCALES as loc (loc.key)}
                        <div class="locale-field">
                          <span class="locale-label">{loc.label}</span>
                          <input
                            value={editValues[loc.key] ?? ''}
                            oninput={(e) => {
                              editValues[loc.key] = (
                                e.target as HTMLInputElement
                              ).value;
                            }}
                            class="form-input edit-input"
                            onkeydown={(e) => {
                              if (e.key === 'Enter') {
                                e.stopPropagation();
                                if (isAlreadyAdded(val)) void handleEditSave();
                                else void handleEditAndAdd();
                              }
                            }}
                          />
                        </div>
                      {/each}
                      <div class="edit-actions">
                        {#if isAlreadyAdded(val)}
                          <Button
                            size="xs"
                            variant="secondary"
                            onclick={handleEditSave}
                            >{m.tag_editor_edit_save()}</Button
                          >
                        {:else}
                          <Button
                            size="xs"
                            variant="secondary"
                            onclick={handleEditAndAdd}
                            >{m.tag_editor_edit_add()}</Button
                          >
                        {/if}
                        <Button size="xs" variant="ghost" onclick={cancelEdit}
                          >{m.tag_editor_cancel()}</Button
                        >
                      </div>
                    </div>
                  {:else}
                    <div
                      class="candidate-chip"
                      class:already-added={isAlreadyAdded(val)}
                      role="button"
                      tabindex="0"
                      onclick={() => handleSelectCandidate(val)}
                      onkeydown={(e) => {
                        if (e.key === 'Enter') void handleSelectCandidate(val);
                      }}
                    >
                      <span class="chip-text">
                        {displayValue(val)}
                        {#if val['en-US'] && val['en-US'] !== val['zh-CN']}
                          <span class="chip-en">{val['en-US']}</span>
                        {/if}
                      </span>
                      <button
                        type="button"
                        class="edit-btn-inset"
                        onclick={(e) => {
                          e.stopPropagation();
                          startEdit(val);
                        }}
                        aria-label={m.tag_editor_edit_i18n()}
                        title={m.tag_editor_edit_i18n()}>✎</button
                      >
                    </div>
                  {/if}
                {/each}
              </div>
            {:else if searchQuery.trim()}
              <div class="no-results">{m.tag_editor_no_results()}</div>
            {/if}
          </div>

          <!-- Create Tab -->
        {:else}
          <div class="create-form">
            <div class="create-input-wrapper">
              <input
                value={createValues[getLocale()] ?? ''}
                oninput={(e) => {
                  createValues[getLocale()] = (
                    e.target as HTMLInputElement
                  ).value;
                }}
                class="form-input create-main-input"
                placeholder={m.tag_editor_create_name_placeholder()}
                onkeydown={handleKeydown}
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
                  >{createI18n ? '多语' : '单语'}</span
                >
              </button>
            </div>

            <div class="locale-cards" bind:this={localeCardsEl}>
              <div class="locale-cards-inner">
                {#each TAG_LOCALES as loc, idx (loc.key)}
                  <div class="locale-card">
                    <span
                      class="locale-card-bar"
                      style="opacity: {1 - idx * 0.2}">{loc.label}</span
                    >
                    <input
                      value={createValues[loc.key] ?? ''}
                      oninput={(e) => {
                        createValues[loc.key] = (
                          e.target as HTMLInputElement
                        ).value;
                      }}
                      class="locale-card-input"
                      onkeydown={handleKeydown}
                    />
                  </div>
                {/each}
              </div>
            </div>

            <Button
              size="xs"
              variant="secondary"
              disabled={!TAG_LOCALES.some((loc) =>
                (createValues[loc.key] ?? '').trim()
              )}
              onclick={handleCreate}>{m.tag_editor_create_button()}</Button
            >
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .add-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--color-border, #d1d5db);
    border-radius: 9999px;
    background: transparent;
    cursor: pointer;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-family: var(--font-body);
    transition: none;
  }

  .add-trigger:hover {
    border-color: var(--color-primary, #6366f1);
    color: var(--color-primary, #6366f1);
  }

  .add-trigger-placeholder {
    white-space: nowrap;
  }

  .add-trigger-icon {
    font-size: 0.875rem;
    line-height: 1;
  }

  .bubble-card {
    position: fixed;
    z-index: 50;
    overflow: visible;
    padding: 0.75rem;
    border-radius: 10px;
    background: var(--bg-popover, var(--popover));
    color: var(--text-primary);
    box-shadow:
      0 4px 24px rgba(0, 0, 0, 0.12),
      0 0 0 1px rgba(0, 0, 0, 0.05);
    width: 24px;
    height: 24px;
  }

  .bubble-content {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    opacity: 0;
  }

  .popover-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--color-border, #e5e7eb);
  }

  .popover-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-toggle {
    position: relative;
    display: flex;
    border-radius: 6px;
    background: var(--color-primary, #6366f1);
    padding: 2px;
    gap: 0;
  }

  .toggle-slider {
    position: absolute;
    top: 2px;
    left: 2px;
    width: calc(50% - 2px);
    height: calc(100% - 4px);
    border-radius: 4px;
    background: var(--bg-popover, white);
    pointer-events: none;
  }

  .toggle-item {
    position: relative;
    z-index: 1;
    padding: 0.125rem 0.375rem;
    font-size: 0.625rem;
    font-weight: 500;
    color: white;
    background: none;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-body);
    line-height: 1.4;
  }

  .toggle-item.active {
    color: var(--color-primary, #6366f1);
  }

  .tab-content {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow: hidden;
  }

  .selected-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .selected-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    background: var(--color-chip-bg, #f3f4f6);
    border-radius: 9999px;
    color: var(--text-primary);
    font-family: var(--font-body);
  }

  .chip-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.875rem;
    line-height: 1;
    color: var(--text-secondary);
    padding: 0;
  }

  .chip-remove:hover {
    color: var(--color-destructive, #ef4444);
  }

  .form-input {
    width: 100%;
    padding: 0.375rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 6px;
    background: var(--bg-input, transparent);
    color: var(--text-primary);
    outline: none;
    font-family: var(--font-body);
  }

  .form-input:focus {
    border-color: var(--color-primary, #6366f1);
  }

  .locale-field {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .locale-label {
    flex-shrink: 0;
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--text-secondary);
    min-width: 2rem;
    text-align: right;
  }

  .candidates {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    max-height: 180px;
    overflow-y: auto;
  }

  .candidates-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .candidate-chip {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.375rem 0.25rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 9999px;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    font-family: var(--font-body);
    text-align: left;
  }

  .candidate-chip:hover:not(.already-added) {
    border-color: var(--color-primary, #6366f1);
    color: var(--color-primary, #6366f1);
  }

  .candidate-chip.already-added {
    opacity: 0.5;
    cursor: default;
  }

  .chip-text {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
  }

  .chip-en {
    font-size: 0.6875rem;
    color: var(--text-secondary);
    opacity: 0.7;
  }

  .edit-btn-inset {
    flex-shrink: 0;
    width: 1rem;
    height: 1rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.625rem;
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 50%;
    opacity: 0;
  }

  .candidate-chip:hover .edit-btn-inset {
    opacity: 1;
  }

  .edit-btn-inset:hover {
    color: var(--color-primary, #6366f1);
    background: var(--color-chip-bg, #f3f4f6);
  }

  .edit-row {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.375rem;
    border: 1px solid var(--color-primary, #6366f1);
    border-radius: 6px;
    background: var(--color-chip-bg, #f3f4f6);
  }

  .edit-input {
    padding: 0.25rem 0.375rem;
  }

  .edit-actions {
    display: flex;
    gap: 0.25rem;
    justify-content: flex-end;
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
    border-radius: 8px;
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
    border-radius: 9999px;
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
    color: var(--text-secondary);
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
    border-radius: 9999px;
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
    border-radius: 9999px;
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
  }

  .locale-card:focus-within {
    border-color: var(--color-primary, #6366f1);
    box-shadow: 0 0 6px 0
      color-mix(in srgb, var(--color-primary, #6366f1) 25%, transparent);
  }

  .no-results {
    font-size: 0.75rem;
    color: var(--text-secondary);
    padding: 0.25rem 0;
  }
</style>
