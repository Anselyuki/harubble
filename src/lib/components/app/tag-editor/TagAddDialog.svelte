<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { gsap } from '$lib/design/gsap';
  import type { TagEditorLocalizedValue } from '$lib/types';
  import {
    filterCandidates,
    type TagLibrary,
  } from '$lib/features/tagEditor/tagLibrary';
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

  let query = $state('');
  let showEnInput = $state(false);
  let pendingZhValue = $state('');
  let enValue = $state('');
  let cardEl: HTMLElement | undefined = $state();
  let cardTop = $state(0);
  let cardLeft = $state(0);

  let candidates = $derived(
    filterCandidates(tagLibrary, dimensionKey, values, query)
  );
  let isNewValue = $derived(
    query.trim() !== '' &&
      !candidates.some(
        (c) => (c['zh-CN'] ?? '').toLowerCase() === query.trim().toLowerCase()
      )
  );

  function displayValue(val: TagEditorLocalizedValue): string {
    return val['zh-CN'] || val['en-US'] || Object.values(val)[0] || '';
  }

  async function handleSelectCandidate(val: TagEditorLocalizedValue) {
    await onSetTag(dimensionKey, [...values, val]);
    query = '';
  }

  async function handleCreateNew() {
    if (!query.trim()) return;
    pendingZhValue = query.trim();
    showEnInput = true;
  }

  async function handleConfirmCreate() {
    const newVal: TagEditorLocalizedValue = { 'zh-CN': pendingZhValue };
    if (enValue.trim()) {
      newVal['en-US'] = enValue.trim();
    }
    await onSetTag(dimensionKey, [...values, newVal]);
    resetState();
  }

  function handleSkipEn() {
    const newVal: TagEditorLocalizedValue = { 'zh-CN': pendingZhValue };
    void onSetTag(dimensionKey, [...values, newVal]);
    resetState();
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
    query = '';
    pendingZhValue = '';
    enValue = '';
    showEnInput = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && query.trim()) {
      if (isNewValue) {
        void handleCreateNew();
      } else if (candidates.length > 0) {
        void handleSelectCandidate(candidates[0]);
      }
    }
    if (e.key === 'Escape') {
      closeCard();
    }
  }

  function closeCard() {
    resetState();
    onOpenChange(false);
  }

  let clickX = 0;
  let clickY = 0;

  function handleTriggerClick(e: MouseEvent) {
    e.stopPropagation();
    clickX = e.clientX;
    clickY = e.clientY;
    const viewport = { width: window.innerWidth, height: window.innerHeight };
    const direction = calcExpandDirection(e.clientX, e.clientY, viewport);
    const pos = calcCardPosition(e.clientX, e.clientY, direction);
    cardTop = pos.top;
    cardLeft = pos.left;
    onOpenChange(true);
  }

  function handleOutsideClick(e: MouseEvent) {
    if (cardEl && !cardEl.contains(e.target as Node)) {
      closeCard();
    }
  }

  $effect(() => {
    if (open) {
      document.addEventListener('pointerdown', handleOutsideClick);
      return () =>
        document.removeEventListener('pointerdown', handleOutsideClick);
    }
  });

  function bubbleEnter(el: HTMLElement) {
    const targetWidth = 280;
    const contentEl = el.querySelector('.bubble-content') as HTMLElement | null;
    const startTop = clickY - 12;
    const startLeft = clickX - 12;
    requestAnimationFrame(() => {
      const targetHeight = el.scrollHeight;
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
          height: targetHeight,
          borderRadius: 10,
          duration: 0.32,
          ease: 'ios-spring',
          onComplete: () => {
            el.style.height = 'auto';
          },
        }
      );
      if (contentEl) {
        gsap.fromTo(
          contentEl,
          { opacity: 0 },
          { opacity: 1, duration: 0.2, delay: 0.16, ease: 'ios-out' }
        );
      }
    });
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
    use:bubbleEnter
    style="top: {cardTop}px; left: {cardLeft}px;"
    onkeydown={handleKeydown}
    role="dialog"
  >
    <div class="bubble-content">
      <div class="popover-header">
        <span class="popover-title">{dimensionLabel}</span>
      </div>

      <div class="selected-tags">
        {#each values as val, idx (`${idx}-${val['zh-CN'] ?? ''}`)}
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

      {#if showEnInput}
        <div class="en-input-row">
          <span class="en-label">{pendingZhValue}</span>
          <input
            bind:value={enValue}
            placeholder={m.tag_editor_en_optional()}
            class="form-input"
            onkeydown={(e) => {
              if (e.key === 'Enter') void handleConfirmCreate();
              if (e.key === 'Escape') handleSkipEn();
            }}
          />
          <Button size="xs" variant="secondary" onclick={handleConfirmCreate}
            >OK</Button
          >
          <Button size="xs" variant="ghost" onclick={handleSkipEn}
            >{m.tag_editor_cancel()}</Button
          >
        </div>
      {:else}
        <input
          bind:value={query}
          placeholder={m.tag_editor_add_tag_placeholder()}
          class="form-input search-input"
          onkeydown={handleKeydown}
        />

        <div class="candidates">
          {#if isNewValue}
            <button
              type="button"
              class="candidate-item create-new"
              onclick={handleCreateNew}
            >
              {m.tag_editor_create_new({ value: query.trim() })}
            </button>
          {/if}
          {#if candidates.length > 0}
            <div class="candidates-label">{m.tag_editor_existing_tags()}</div>
            <div class="candidates-list">
              {#each candidates as val (`${val['zh-CN'] ?? ''}-${val['en-US'] ?? ''}`)}
                <button
                  type="button"
                  class="candidate-chip"
                  onclick={() => handleSelectCandidate(val)}
                  >{displayValue(val)}</button
                >
              {/each}
            </div>
          {/if}
        </div>
      {/if}
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
    white-space: nowrap;
  }

  .popover-header {
    display: flex;
    align-items: center;
    padding-bottom: 0.25rem;
    border-bottom: 1px solid var(--color-border, #e5e7eb);
  }

  .popover-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
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
    font-size: 0.8125rem;
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

  .en-input-row {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .en-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .candidates {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .candidates-label {
    font-size: 0.6875rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }

  .candidates-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .candidate-chip {
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 9999px;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    font-family: var(--font-body);
  }

  .candidate-chip:hover {
    border-color: var(--color-primary, #6366f1);
    color: var(--color-primary, #6366f1);
  }

  .candidate-item.create-new {
    font-size: 0.75rem;
    color: var(--color-primary, #6366f1);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.25rem 0;
    text-align: left;
    font-family: var(--font-body);
  }
</style>
