<script lang="ts">
  /**
   * 主题包库 UI（Phase 1 Step 1.g，灰度阶段变更）。
   *
   * 提供已安装主题包的列表、导入按钮和 preview/apply/dismiss/uninstall 交互。
   *
   * # Feature flag `theme_packages_v1` 生命周期
   *
   * - Phase 1-3.2 期间：opt-in 语义，`localStorage['theme_packages_v1'] === '1'` 才显示
   * - **Phase 3.3 完成后（当前）**：opt-out 语义，默认显示；用户可通过设置
   *   `localStorage['theme_packages_v1'] = '0'` 隐藏（用于遇到问题时临时回退）
   * - 灰度稳定 N 个 minor 版本后：移除 flag 检查，永远显示
   *
   * 状态管理由 `themePackageManager` 完成，本组件仅负责视图和用户操作。
   */
  import { onDestroy, onMount } from 'svelte';
  import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
  import { listen } from '@tauri-apps/api/event';
  import { createThemePackageManager } from '$lib/features/shell/themePackageManager.svelte';
  import type { ThemePackageSummary } from '$lib/types';

  const manager = createThemePackageManager({ listen });
  let importError = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let importing = $state(false);
  let importingUrl = $state(false);
  let urlInput = $state('');
  let busyId = $state<string | null>(null);

  // Phase 3.3+ 灰度：opt-out 语义。默认启用；用户可通过 localStorage 设 '0' 显式禁用。
  // 注意：localStorage 读不是 Svelte 反应依赖，用 $state 保存快照；toggle 时同步更新
  // state + storage 双写，避免依赖 reload 才能感知 flag 变化。
  function readInitialFlag(): boolean {
    if (typeof window === 'undefined') return true;
    try {
      return window.localStorage.getItem('theme_packages_v1') !== '0';
    } catch {
      // 沙盒 iframe / 严格隐私模式 / 禁用第三方存储时抛 SecurityError
      return true;
    }
  }

  let featureFlagEnabled = $state(readInitialFlag());

  function setFeatureFlag(enabled: boolean): void {
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.setItem('theme_packages_v1', enabled ? '1' : '0');
      } catch {
        /* SecurityError：无 localStorage 访问权限，仍更新 state 以让本会话生效 */
      }
    }
    if (enabled && !featureFlagEnabled) {
      void manager.hydrate();
      void manager.startSubscription();
    } else if (!enabled && featureFlagEnabled) {
      manager.stopSubscription();
    }
    featureFlagEnabled = enabled;
  }

  onMount(() => {
    if (!featureFlagEnabled) return;
    void manager.hydrate();
    void manager.startSubscription();
  });

  onDestroy(() => {
    manager.stopSubscription();
  });

  async function handleImport(): Promise<void> {
    importError = null;
    importing = true;
    try {
      const selected = await openFileDialog({
        multiple: false,
        filters: [{ name: 'Theme Package JSON', extensions: ['json'] }],
      });
      if (typeof selected !== 'string') {
        importing = false;
        return;
      }
      await manager.importFromFile(selected);
    } catch (err) {
      importError = err instanceof Error ? err.message : String(err);
    } finally {
      importing = false;
    }
  }

  async function handleImportFromUrl(): Promise<void> {
    importError = null;
    const trimmed = urlInput.trim();
    if (trimmed === '') return;
    importingUrl = true;
    try {
      await manager.importFromUrl(trimmed);
      urlInput = '';
    } catch (err) {
      importError = err instanceof Error ? err.message : String(err);
    } finally {
      importingUrl = false;
    }
  }

  async function handleApply(pkg: ThemePackageSummary): Promise<void> {
    actionError = null;
    busyId = pkg.id;
    try {
      await manager.setActive(pkg.id);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    } finally {
      busyId = null;
    }
  }

  async function handleClearActive(): Promise<void> {
    actionError = null;
    busyId = '__clear__';
    try {
      await manager.setActive(null);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    } finally {
      busyId = null;
    }
  }

  async function handlePreview(pkg: ThemePackageSummary): Promise<void> {
    actionError = null;
    try {
      await manager.preview(pkg.id);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleDismissPreview(): Promise<void> {
    actionError = null;
    try {
      await manager.dismissPreview();
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleUninstall(pkg: ThemePackageSummary): Promise<void> {
    actionError = null;
    busyId = pkg.id;
    try {
      await manager.uninstall(pkg.id);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    } finally {
      busyId = null;
    }
  }
</script>

{#if featureFlagEnabled}
  <section
    class="theme-package-section"
    data-testid="theme-package-library-section"
  >
    <header class="section-header">
      <h3 class="section-title">主题包库</h3>
      <p class="section-hint">
        导入 JSON 主题包切换配色 / motion / shape / density / elevation / blur
        与视觉族。
      </p>
      <button
        type="button"
        class="btn btn-tertiary section-disable"
        onclick={() => setFeatureFlag(false)}
        data-testid="theme-package-library-disable"
      >
        隐藏主题包库
      </button>
    </header>

    <div class="toolbar">
      <button
        type="button"
        class="btn btn-primary"
        onclick={handleImport}
        disabled={importing}
        data-testid="theme-package-import"
      >
        {importing ? '导入中…' : '导入主题包 (.json)'}
      </button>
      {#if manager.activePackageId}
        <button
          type="button"
          class="btn btn-secondary"
          onclick={handleClearActive}
          disabled={busyId === '__clear__'}
        >
          清空激活状态
        </button>
      {/if}
      {#if manager.previewingId}
        <button
          type="button"
          class="btn btn-secondary"
          onclick={handleDismissPreview}
          data-testid="theme-package-dismiss-preview"
        >
          退出预览（{manager.previewingId}）
        </button>
      {/if}
    </div>

    <div class="url-import" data-testid="theme-package-url-import">
      <input
        type="url"
        class="url-input"
        placeholder="https://example.com/theme.json"
        bind:value={urlInput}
        disabled={importingUrl}
        data-testid="theme-package-url-input"
      />
      <button
        type="button"
        class="btn btn-secondary"
        onclick={handleImportFromUrl}
        disabled={importingUrl || urlInput.trim() === ''}
        data-testid="theme-package-url-submit"
      >
        {importingUrl ? '下载中…' : '从 URL 导入'}
      </button>
    </div>

    {#if importError}
      <p class="error-banner" role="alert">导入失败：{importError}</p>
    {/if}
    {#if actionError}
      <p class="error-banner" role="alert">操作失败：{actionError}</p>
    {/if}

    <ul class="package-list">
      {#each manager.installedPackages as pkg (pkg.id)}
        <li
          class="package-item"
          class:package-item--active={manager.activePackageId === pkg.id}
          data-testid="theme-package-item"
          data-package-id={pkg.id}
        >
          <div class="package-meta">
            <span class="package-name">{pkg.name}</span>
            <span class="package-version">v{pkg.version}</span>
            <span class="package-id">{pkg.id}</span>
          </div>
          <div class="package-actions">
            <button
              type="button"
              class="btn btn-small"
              onclick={() => handlePreview(pkg)}
              disabled={busyId === pkg.id}
            >
              预览
            </button>
            <button
              type="button"
              class="btn btn-small btn-primary"
              onclick={() => handleApply(pkg)}
              disabled={busyId === pkg.id || manager.activePackageId === pkg.id}
            >
              {manager.activePackageId === pkg.id ? '当前激活' : '应用'}
            </button>
            <button
              type="button"
              class="btn btn-small btn-danger"
              onclick={() => handleUninstall(pkg)}
              disabled={busyId === pkg.id}
            >
              卸载
            </button>
          </div>
        </li>
      {:else}
        <li class="package-empty">尚未安装主题包，点击"导入主题包"添加。</li>
      {/each}
    </ul>

    <footer class="section-footer">
      <span class="revision-badge">
        theme.revision: <code>{manager.currentRevision}</code>
      </span>
    </footer>
  </section>
{/if}

<style>
  .theme-package-section {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--shape-lg);
  }
  .section-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
    position: relative;
  }
  .section-title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
    color: var(--text-primary);
  }
  .section-hint {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin: 0;
  }
  .section-disable {
    align-self: flex-start;
    margin-top: 4px;
    font-size: 0.75rem;
    color: var(--text-tertiary);
    background: transparent;
    border: 1px solid var(--border);
    padding: 4px 8px;
  }
  .section-disable:hover {
    color: var(--text-secondary);
  }
  .url-import {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .url-input {
    flex: 1;
    min-width: 0;
    padding: 8px 12px;
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--shape-sm);
    color: var(--text-primary);
    font-size: 0.875rem;
    font-family: var(--font-mono);
  }
  .url-input:focus {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .url-input:disabled {
    opacity: 0.6;
  }
  .toolbar {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .btn {
    padding: 8px 14px;
    border-radius: var(--shape-md);
    border: 1px solid var(--border);
    background-color: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: var(--motion-hover);
  }
  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .btn-primary {
    background-color: var(--accent);
    color: var(--accent-readable-foreground);
    border-color: transparent;
  }
  .btn-secondary {
    background-color: var(--surface-state);
  }
  .btn-danger {
    background-color: var(--destructive);
    color: white;
    border-color: transparent;
  }
  .btn-small {
    padding: 6px 10px;
    font-size: 0.8125rem;
  }
  .error-banner {
    padding: 10px 14px;
    background-color: rgba(var(--destructive-rgb), 0.12);
    color: var(--destructive);
    border-radius: var(--shape-md);
    font-size: 0.875rem;
    margin: 0;
  }
  .package-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .package-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--shape-md);
  }
  .package-item--active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .package-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .package-name {
    font-weight: 600;
    color: var(--text-primary);
  }
  .package-version {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }
  .package-id {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    color: var(--text-tertiary);
  }
  .package-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .package-empty {
    padding: 20px;
    text-align: center;
    color: var(--text-secondary);
    background-color: var(--bg-primary);
    border: 1px dashed var(--border);
    border-radius: var(--shape-md);
    font-size: 0.875rem;
  }
  .section-footer {
    display: flex;
    justify-content: flex-end;
  }
  .revision-badge {
    font-size: 0.75rem;
    color: var(--text-tertiary);
  }
  .revision-badge code {
    font-family: var(--font-mono);
    background-color: var(--bg-tertiary);
    padding: 2px 6px;
    border-radius: var(--shape-xs);
    margin-left: 4px;
  }
</style>
