<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type { LibrarySearchScope } from '$lib/types';

  interface Props {
    query: string;
    scope: LibrarySearchScope;
  }

  // scope 为搜索上下文：预留给下一个 PR 的"按范围（全部 / 专辑 / 歌曲）
  // 展示搜索结果与空态"逻辑；本期占位视图尚未渲染，故以 _scope 显式标记暂未使用。
  let { query, scope: _scope }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.search_results_placeholder_title(),
      hint: m.search_results_placeholder_hint(),
    };
  });
</script>

<div class="results-placeholder">
  <div class="placeholder-icon" aria-hidden="true">🔍</div>
  <div class="placeholder-title">{labels.title}</div>
  <div class="placeholder-query">"{query}"</div>
  <div class="placeholder-hint">{labels.hint}</div>
</div>

<style>
  .results-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 80px 24px;
    color: var(--text-tertiary);
    text-align: center;
  }

  .placeholder-icon {
    font-size: 40px;
    opacity: 0.4;
    margin-bottom: 4px;
  }

  .placeholder-title {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .placeholder-query {
    font-family: var(--font-body);
    font-size: 13px;
    color: var(--accent);
    font-weight: 500;
  }

  .placeholder-hint {
    font-family: var(--font-body);
    font-size: 12px;
    color: var(--text-tertiary);
    margin-top: 2px;
  }
</style>
