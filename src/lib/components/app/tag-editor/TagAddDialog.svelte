<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { gsap, getMotionDuration, MOTION } from '$lib/design/gsap';
  import type { TagEditorLocalizedValue } from '$lib/types';
  import { type TagLibrary } from '$lib/features/tagEditor/tagLibrary';
  import {
    type ExpandDirection,
    calcExpandDirection,
    calcCardPosition,
    measureBubbleTargetSize,
  } from './popoverBubble';
  import { TAG_LOCALES, tagIdentity, displayValue } from './tagAddUtils';
  import TagSearchTab from './TagSearchTab.svelte';
  import TagCreateTab from './TagCreateTab.svelte';

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
    open = false,
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
  let cardEl: HTMLElement | undefined = $state();
  let tabContentEl: HTMLElement | undefined = $state();
  let sliderEl: HTMLElement | undefined = $state();
  let editingTag = $state<TagEditorLocalizedValue | null>(null);
  let cardTop = $state(0);
  let cardLeft = $state(0);

  let allDimensionTags = $derived(tagLibrary[dimensionKey] ?? []);
  let searchResults = $derived(
    searchQuery.trim()
      ? allDimensionTags.filter((v) => {
          const q = searchQuery.trim().toLowerCase();
          return Object.values(v).some(
            (text) => text && text.toLowerCase().includes(q)
          );
        })
      : allDimensionTags
  );

  function isAlreadyAdded(val: TagEditorLocalizedValue): boolean {
    const id = tagIdentity(val);
    return values.some((v) => tagIdentity(v) === id);
  }

  async function handleSelectCandidate(val: TagEditorLocalizedValue) {
    if (isAlreadyAdded(val)) return;
    await onSetTag(dimensionKey, [...values, val]);
  }

  async function handleEditSave(
    originalTag: TagEditorLocalizedValue,
    editValues: Record<string, string>
  ) {
    const updated: TagEditorLocalizedValue = {};
    for (const loc of TAG_LOCALES) {
      const v = (editValues[loc.key] ?? '').trim();
      if (v) updated[loc.key] = v;
    }
    const originalId = tagIdentity(originalTag);
    const idx = values.findIndex((v) => tagIdentity(v) === originalId);
    if (idx >= 0) {
      const newValues = values.map((v, i) => (i === idx ? updated : v));
      await onSetTag(dimensionKey, newValues);
    }
  }

  async function handleEditAndAdd(editValues: Record<string, string>) {
    const updated: TagEditorLocalizedValue = {};
    for (const loc of TAG_LOCALES) {
      const v = (editValues[loc.key] ?? '').trim();
      if (v) updated[loc.key] = v;
    }
    if (!isAlreadyAdded(updated)) {
      await onSetTag(dimensionKey, [...values, updated]);
    }
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
    activeTab = 'search';
    editingTag = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (editingTag !== null) {
        editingTag = null;
      } else {
        closeCard();
      }
    }
    const target = e.target as HTMLElement;
    const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA';
    if (
      e.key === 'Enter' &&
      isInput &&
      activeTab === 'search' &&
      searchQuery.trim()
    ) {
      const first = searchResults.find((v) => !isAlreadyAdded(v));
      if (first) void handleSelectCandidate(first);
    }
    if (e.key === 'Enter' && isInput && activeTab === 'create') {
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
  let expandDir = $state<ExpandDirection>('bottom-right');

  function handleTriggerClick(e: MouseEvent) {
    e.stopPropagation();
    clickX = e.clientX;
    clickY = e.clientY;
    const viewport = { width: window.innerWidth, height: window.innerHeight };
    expandDir = calcExpandDirection(
      e.clientX,
      e.clientY,
      viewport,
      targetWidth
    );
    const pos = calcCardPosition(e.clientX, e.clientY, expandDir, targetWidth);
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
      const rafId = requestAnimationFrame(() => {
        document.addEventListener('pointerdown', handleOutsideClick);
      });
      return () => {
        cancelAnimationFrame(rafId);
        document.removeEventListener('pointerdown', handleOutsideClick);
      };
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
      {
        opacity: 1,
        x: 0,
        duration: getMotionDuration(MOTION.OVERLAY_IN),
        ease: 'ios-out',
      }
    );
  });

  $effect(() => {
    if (!sliderEl) return;
    const isRight = activeTab === 'create';
    gsap.to(sliderEl, {
      x: isRight ? '100%' : '0%',
      duration: hasMounted ? getMotionDuration(MOTION.BASE) : 0,
      ease: 'ios-spring',
    });
  });

  const targetWidth = 220;

  function bubbleEnter(el: HTMLElement, params: { x: number; y: number }) {
    const contentEl = el.querySelector('.bubble-content') as HTMLElement | null;
    const startTop = params.y - 12;
    const startLeft = params.x - 12;
    const targetSize = measureBubbleTargetSize(el, targetWidth);
    const viewport = { width: window.innerWidth, height: window.innerHeight };
    expandDir = calcExpandDirection(
      clickX,
      clickY,
      viewport,
      targetWidth,
      targetSize.height
    );
    const finalPos = calcCardPosition(
      clickX,
      clickY,
      expandDir,
      targetWidth,
      undefined,
      targetSize.height
    );
    cardTop = finalPos.top;
    cardLeft = finalPos.left;
    const tweens: gsap.core.Tween[] = [];
    tweens.push(
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
          top: finalPos.top,
          left: finalPos.left,
          width: targetSize.width,
          height: targetSize.height,
          borderRadius: 10,
          duration: getMotionDuration(MOTION.SLOW),
          ease: 'ios-out',
          onComplete: () => {
            gsap.set(el, { height: 'auto' });
          },
        }
      )
    );
    if (contentEl) {
      tweens.push(
        gsap.fromTo(
          contentEl,
          { opacity: 0 },
          {
            opacity: 1,
            duration: getMotionDuration(MOTION.OVERLAY_IN),
            // 内容相对气泡展开的错位起步延迟（非元素时长），不并入时长令牌。
            delay: getMotionDuration(120),
            ease: 'ios-out',
          }
        )
      );
    }
    return {
      destroy() {
        tweens.forEach((t) => t.kill());
      },
    };
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

      <div class="tab-content" bind:this={tabContentEl}>
        {#if activeTab === 'search'}
          <TagSearchTab
            bind:searchQuery
            bind:editingTag
            {searchResults}
            {values}
            tagLocales={TAG_LOCALES}
            onSelect={handleSelectCandidate}
            onEditSave={handleEditSave}
            onEditAndAdd={handleEditAndAdd}
          />
        {:else}
          <TagCreateTab
            bind:createValues
            bind:createI18n
            tagLocales={TAG_LOCALES}
            onCreate={handleCreate}
          />
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
    color: var(--text-primary);
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
    background: transparent;
    border: 1px solid var(--color-primary, #6366f1);
    border-radius: 9999px;
    color: var(--color-primary, #6366f1);
    font-family: var(--font-body);
  }

  .chip-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.875rem;
    line-height: 1;
    color: var(--color-primary, #6366f1);
    padding: 0;
    opacity: 0.6;
  }

  .chip-remove:hover {
    opacity: 1;
    color: var(--color-destructive, #ef4444);
  }
</style>
