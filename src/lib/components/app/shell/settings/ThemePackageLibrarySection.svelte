<script module lang="ts">
  let nextPreviewCleanupGeneration = 0;
  const latestPreviewCleanupGeneration = new WeakMap<object, number>();
</script>

<script lang="ts">
  /**
   * 主题包库 UI。
   *
   * 提供已安装主题包的列表、导入按钮和 preview/apply/dismiss/uninstall 交互。
   *
   * # 折叠状态 `theme_packages_v1`
   *
   * 沿用灰度阶段的 storage key 以兼容已有设置，但现在只控制库面板的展开/收起。
   * 收起不会清除激活主题包或预览状态，并始终保留可恢复的展开入口。
   *
   * 状态管理由 `themePackageManager` 完成，本组件仅负责视图和用户操作。
   */
  import { onDestroy, onMount, tick } from 'svelte';
  import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
  import { getThemePackageManager } from '$lib/features/shell/themePackageManager.svelte';
  import type { ThemePackageSummary } from '$lib/types';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';

  interface ThemePackageLibraryManager {
    readonly activePackageId: string | null;
    readonly previewingId: string | null;
    readonly installedPackages: ThemePackageSummary[];
    readonly latestError?: string | null;
    hydrate: () => Promise<unknown>;
    setActive: (id: string | null) => Promise<unknown>;
    preview: (id: string) => Promise<unknown>;
    dismissPreview: () => Promise<unknown>;
    importFromFile: (path: string) => Promise<unknown>;
    importFromUrl: (url: string) => Promise<unknown>;
    uninstall: (id: string) => Promise<unknown>;
  }

  interface Props {
    sectionTitle: string;
    sectionDescription: string;
    manager?: ThemePackageLibraryManager;
  }

  let {
    sectionTitle,
    sectionDescription,
    manager = getThemePackageManager(),
  }: Props = $props();

  // 集中收敛 paraglide 文案入口，显式声明 locale 依赖，locale 切换时全部 label 重求。
  // 参考 SidebarNav / SettingsSheet 的标准模式（void localeState.current 放在
  // $derived.by 内部才生效，放模块顶层等价 dead code）。
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      disable: m.settings_theme_packages_disable(),
      enable: m.settings_theme_packages_enable(),
      importFile: m.settings_theme_packages_import_file(),
      importUrl: m.settings_theme_packages_import_url(),
      urlPlaceholder: m.settings_theme_packages_url_placeholder(),
      importWarnings: m.settings_theme_packages_import_warnings(),
      preview: m.settings_theme_packages_preview(),
      apply: m.settings_theme_packages_apply(),
      dismissPreview: m.settings_theme_packages_dismiss_preview(),
      clearActive: m.settings_theme_packages_clear_active(),
      uninstall: m.settings_theme_packages_uninstall(),
      empty: m.settings_theme_packages_empty(),
      activeBadge: m.settings_theme_packages_active_badge(),
      previewBadge: m.settings_theme_packages_preview_badge(),
      builtinBadge: m.settings_theme_packages_builtin_badge(),
    };
  });

  let importError = $state<string | null>(null);
  let importWarnings = $state<string[]>([]);
  let actionError = $state<string | null>(null);
  let importing = $state(false);
  let importingUrl = $state(false);
  let urlInput = $state('');
  let busyId = $state<string | null>(null);
  let sectionEl = $state<HTMLElement | null>(null);
  let previewCleanupOwner: Promise<unknown> | null = null;
  nextPreviewCleanupGeneration += 1;
  const previewCleanupGeneration = nextPreviewCleanupGeneration;
  const actionBusy = $derived(busyId !== null || importing || importingUrl);
  const visiblePreviewingId = $derived(
    manager.previewingId !== manager.activePackageId
      ? manager.previewingId
      : null
  );

  function readInitialExpandedState(): boolean {
    if (typeof window === 'undefined') return true;
    try {
      return window.localStorage.getItem('theme_packages_v1') !== '0';
    } catch {
      // 沙盒 iframe / 严格隐私模式 / 禁用第三方存储时抛 SecurityError
      return true;
    }
  }

  let libraryExpanded = $state(readInitialExpandedState());

  function persistExpandedState(expanded: boolean): void {
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.setItem('theme_packages_v1', expanded ? '1' : '0');
      } catch {
        /* SecurityError：无 localStorage 访问权限，仍更新 state 以让本会话生效 */
      }
    }
    libraryExpanded = expanded;
  }

  function handleToggleLibrary(): void {
    if (libraryExpanded && actionBusy) {
      return;
    }
    const nextExpanded = !libraryExpanded;
    persistExpandedState(nextExpanded);
    if (nextExpanded) void manager.hydrate();
  }

  onMount(() => {
    latestPreviewCleanupGeneration.set(manager, previewCleanupGeneration);
    if (!libraryExpanded) return;
    void manager.hydrate();
  });

  onDestroy(() => {
    const owner = previewCleanupOwner;
    if (owner) {
      // Applying or removing a package already owns preview reconciliation.
      // Wait for it to settle so closing the sheet cannot briefly restore the
      // old package before a successful activation paints the persisted one.
      void owner
        .then(cleanupVisiblePreview, cleanupVisiblePreview)
        .catch(() => {});
      return;
    }
    // Only preview teardown may invalidate the manager intent. Calling this for
    // an unrelated in-flight activation cancels its package-switch animation.
    // `busyId` covers the gap before a preview response publishes previewingId.
    if (visiblePreviewingId === null && busyId !== '__preview__') return;
    if (!ownsPreviewCleanupLease()) return;
    void manager.dismissPreview().catch(() => {});
  });

  function ownsPreviewCleanupLease(): boolean {
    return (
      latestPreviewCleanupGeneration.get(manager) === previewCleanupGeneration
    );
  }

  async function cleanupVisiblePreview(): Promise<void> {
    // A reopened settings sheet owns any preview it exposes, even when it uses
    // the same manager and package id as the instance that just closed.
    if (!ownsPreviewCleanupLease()) return;
    if (
      manager.previewingId === null ||
      manager.previewingId === manager.activePackageId
    ) {
      return;
    }
    try {
      await manager.dismissPreview();
    } catch {
      // The settings sheet is already gone; cleanup remains best effort.
    }
  }

  function ownPreviewCleanup<T>(operation: Promise<T>): Promise<T> {
    const owned = operation.finally(() => {
      if (previewCleanupOwner === owned) previewCleanupOwner = null;
    });
    previewCleanupOwner = owned;
    return owned;
  }

  function readImportWarnings(value: unknown): string[] {
    if (!value || typeof value !== 'object' || !('warnings' in value)) {
      return [];
    }
    const warnings = value.warnings;
    return Array.isArray(warnings)
      ? warnings.filter(
          (warning): warning is string => typeof warning === 'string'
        )
      : [];
  }

  async function handleImport(): Promise<void> {
    importError = null;
    importWarnings = [];
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
      const summary = await ownPreviewCleanup(manager.importFromFile(selected));
      importWarnings = readImportWarnings(summary);
    } catch (err) {
      importError = err instanceof Error ? err.message : String(err);
    } finally {
      importing = false;
    }
  }

  async function handleImportFromUrl(): Promise<void> {
    importError = null;
    importWarnings = [];
    const trimmed = urlInput.trim();
    if (trimmed === '') return;
    importingUrl = true;
    try {
      const summary = await ownPreviewCleanup(manager.importFromUrl(trimmed));
      importWarnings = readImportWarnings(summary);
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
      await ownPreviewCleanup(manager.setActive(pkg.id));
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
      await ownPreviewCleanup(manager.setActive(null));
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    } finally {
      busyId = null;
    }
  }

  async function handlePreview(pkg: ThemePackageSummary): Promise<void> {
    if (actionBusy) return;
    const isActivePackage = manager.activePackageId === pkg.id;
    if (isActivePackage && visiblePreviewingId === null) return;
    actionError = null;
    busyId = isActivePackage ? '__dismiss-preview__' : '__preview__';
    try {
      if (isActivePackage) {
        // Previewing the persisted package is not a distinct state. If another
        // package is being previewed, this action simply returns to the active
        // package; otherwise the button remains an idle "Preview" affordance.
        await ownPreviewCleanup(manager.dismissPreview());
      } else {
        await manager.preview(pkg.id);
      }
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    } finally {
      busyId = null;
    }
  }

  async function handleDismissPreview(): Promise<void> {
    if (actionBusy) return;
    actionError = null;
    const dismissedId = visiblePreviewingId;
    busyId = '__dismiss-preview__';
    let dismissed = false;
    try {
      await ownPreviewCleanup(manager.dismissPreview());
      dismissed = true;
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    } finally {
      busyId = null;
    }
    if (!dismissed) return;
    // Release the operation lock before restoring focus: disabled buttons do
    // not accept programmatic focus, even after the preview state has cleared.
    await tick();
    const dismissedItem = Array.from(
      sectionEl?.querySelectorAll<HTMLElement>('[data-package-id]') ?? []
    ).find((item) => item.dataset.packageId === dismissedId);
    dismissedItem
      ?.querySelector<HTMLButtonElement>(
        '[data-theme-package-action="preview"]'
      )
      ?.focus();
  }

  async function handleUninstall(pkg: ThemePackageSummary): Promise<void> {
    actionError = null;
    busyId = pkg.id;
    try {
      await ownPreviewCleanup(manager.uninstall(pkg.id));
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    } finally {
      busyId = null;
    }
  }
</script>

<section
  bind:this={sectionEl}
  class="sheet-section settings-section theme-package-section"
  class:theme-package-section--collapsed={!libraryExpanded}
  data-testid={libraryExpanded
    ? 'theme-package-library-section'
    : 'theme-package-library-collapsed'}
>
  <div class="settings-section-heading">
    <h3>{sectionTitle}</h3>
    <button
      type="button"
      class="btn"
      class:btn-tertiary={libraryExpanded}
      class:section-disable={libraryExpanded}
      class:btn-primary={!libraryExpanded}
      class:section-enable={!libraryExpanded}
      onclick={handleToggleLibrary}
      disabled={libraryExpanded && actionBusy}
      aria-expanded={libraryExpanded}
      aria-controls="theme-package-library-content"
      data-testid={libraryExpanded
        ? 'theme-package-library-disable'
        : 'theme-package-library-enable'}
      title={libraryExpanded ? labels.disable : labels.enable}
    >
      {libraryExpanded ? labels.disable : labels.enable}
    </button>
  </div>
  <p class="settings-section-description">{sectionDescription}</p>

  <div
    id="theme-package-library-content"
    class="theme-package-content"
    hidden={!libraryExpanded}
  >
    <div class="toolbar">
      <button
        type="button"
        class="btn btn-primary"
        onclick={handleImport}
        disabled={actionBusy}
        data-testid="theme-package-import"
      >
        {labels.importFile}
      </button>
      {#if manager.activePackageId}
        <button
          type="button"
          class="btn btn-secondary"
          onclick={handleClearActive}
          disabled={actionBusy}
        >
          {labels.clearActive}
        </button>
      {/if}
      {#if visiblePreviewingId}
        <button
          type="button"
          class="btn btn-secondary"
          onclick={handleDismissPreview}
          disabled={actionBusy}
          data-testid="theme-package-dismiss-preview"
        >
          {labels.dismissPreview}（{visiblePreviewingId}）
        </button>
      {/if}
    </div>

    <div class="url-import" data-testid="theme-package-url-import">
      <input
        type="url"
        class="url-input"
        placeholder={labels.urlPlaceholder}
        bind:value={urlInput}
        disabled={actionBusy}
        data-testid="theme-package-url-input"
      />
      <button
        type="button"
        class="btn btn-secondary"
        onclick={handleImportFromUrl}
        disabled={actionBusy || urlInput.trim() === ''}
        data-testid="theme-package-url-submit"
      >
        {labels.importUrl}
      </button>
    </div>

    {#if importError}
      <p class="error-banner" role="alert">{importError}</p>
    {/if}
    {#if importWarnings.length > 0}
      <div class="warning-banner" role="status">
        <strong>{labels.importWarnings}</strong>
        <ul>
          {#each importWarnings as warning, index (`${index}:${warning}`)}
            <li>{warning}</li>
          {/each}
        </ul>
      </div>
    {/if}
    {#if actionError}
      <p class="error-banner" role="alert">{actionError}</p>
    {/if}
    {#if manager.latestError}
      <p class="error-banner" role="alert">{manager.latestError}</p>
    {/if}

    <ul class="package-list">
      {#each manager.installedPackages as pkg (pkg.id)}
        {@const isActive = manager.activePackageId === pkg.id}
        {@const isPreviewing = visiblePreviewingId === pkg.id}
        <li
          class="package-item"
          class:package-item--active={isActive}
          class:package-item--previewing={isPreviewing}
          data-testid="theme-package-item"
          data-package-id={pkg.id}
          data-active={isActive}
          data-previewing={isPreviewing}
        >
          <div class="package-meta">
            <span class="package-name">{pkg.name}</span>
            <span class="package-version">v{pkg.version}</span>
            {#if isActive || isPreviewing || pkg.builtin}
              <div class="package-badges">
                {#if isActive}
                  <span
                    class="package-status package-status--active"
                    data-testid="theme-package-active-badge"
                  >
                    {labels.activeBadge}
                  </span>
                {/if}
                {#if isPreviewing}
                  <span
                    class="package-status package-status--previewing"
                    data-testid="theme-package-preview-badge"
                  >
                    {labels.previewBadge}
                  </span>
                {/if}
                {#if pkg.builtin}
                  <span class="package-builtin">{labels.builtinBadge}</span>
                {/if}
              </div>
            {/if}
            <span class="package-id">{pkg.id}</span>
          </div>
          <div class="package-actions">
            <button
              type="button"
              class="btn btn-small"
              class:btn-previewing={isPreviewing}
              data-theme-package-action="preview"
              onclick={() => handlePreview(pkg)}
              disabled={actionBusy || isPreviewing}
              aria-pressed={isPreviewing}
            >
              {isPreviewing ? labels.previewBadge : labels.preview}
            </button>
            <button
              type="button"
              class="btn btn-small btn-primary"
              onclick={() => handleApply(pkg)}
              disabled={actionBusy || isActive}
              aria-pressed={isActive}
            >
              {isActive ? labels.activeBadge : labels.apply}
            </button>
            {#if !pkg.builtin}
              <button
                type="button"
                class="btn btn-small btn-danger"
                onclick={() => handleUninstall(pkg)}
                disabled={actionBusy}
              >
                {labels.uninstall}
              </button>
            {/if}
          </div>
        </li>
      {:else}
        <li class="package-empty">{labels.empty}</li>
      {/each}
    </ul>
  </div>
</section>

<style>
  .theme-package-section {
    /* 外框 (border / bg / radius / padding) 已由 .sheet-section 提供，
       这里只补充内部 flex 布局与略大的 gap（section 内元素较多） */
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .theme-package-section--collapsed {
    gap: 8px;
  }
  .theme-package-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .theme-package-content[hidden] {
    display: none;
  }
  .section-enable {
    flex-shrink: 0;
    white-space: nowrap;
  }
  /* section 描述文字，紧邻 heading 下方，与其他 section 视觉一致 */
  .settings-section-description {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin: -8px 0 0;
    line-height: 1.5;
  }
  .theme-package-section--collapsed .settings-section-description {
    margin-top: 0;
  }
  .section-disable {
    align-self: flex-start;
    font-size: 0.75rem;
    color: var(--text-tertiary);
    background: transparent;
    border: 1px solid var(--sheet-border, var(--border));
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
    background-color: var(--sheet-control-bg, var(--bg-primary));
    border: 1px solid var(--sheet-border, var(--border));
    border-radius: var(--shape-sm);
    color: var(--text-primary);
    font-size: 0.875rem;
    font-family: var(--font-mono);
  }
  .url-input::placeholder {
    color: var(--text-tertiary);
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
    min-width: 0;
  }
  .toolbar > .btn {
    min-width: 0;
    max-width: 100%;
    overflow-wrap: anywhere;
    white-space: normal;
  }
  .btn {
    padding: 8px 14px;
    border-radius: var(--shape-md);
    border: 1px solid var(--sheet-border, var(--border));
    background-color: var(--sheet-control-bg, var(--bg-primary));
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
    background-color: var(--sheet-row-hover-bg, var(--surface-state));
  }
  .btn.section-disable {
    background: transparent;
    color: var(--text-tertiary);
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
  .warning-banner {
    padding: 10px 14px;
    border: 1px solid
      color-mix(in srgb, var(--accent) 56%, var(--sheet-border, var(--border)));
    border-radius: var(--shape-md);
    background: color-mix(
      in srgb,
      var(--accent) 9%,
      var(--sheet-row-bg, var(--bg-primary))
    );
    color: var(--text-primary);
    font-size: 0.8125rem;
    overflow-wrap: anywhere;
  }
  .warning-banner ul {
    margin: 6px 0 0;
    padding-inline-start: 18px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 0.75rem;
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
    background-color: var(--sheet-row-bg, var(--bg-primary));
    border: 1px solid var(--sheet-border, var(--border));
    border-radius: var(--shape-md);
  }
  .package-item--active {
    border-color: var(--accent);
    box-shadow: inset 4px 0 0 var(--accent);
  }
  .package-item--previewing {
    border-color: color-mix(
      in srgb,
      var(--accent) 76%,
      var(--sheet-border, var(--border))
    );
    border-style: dashed;
    background: color-mix(
      in srgb,
      var(--accent) 8%,
      var(--sheet-row-bg, var(--bg-primary))
    );
    box-shadow: inset 0 0 0 1px
      color-mix(in srgb, var(--accent) 34%, transparent);
  }
  .package-item--active.package-item--previewing {
    box-shadow:
      inset 4px 0 0 var(--accent),
      inset 0 0 0 1px color-mix(in srgb, var(--accent) 34%, transparent);
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
    overflow-wrap: anywhere;
  }
  .package-badges {
    display: flex;
    min-height: 18px;
    flex-wrap: wrap;
    gap: 4px;
  }
  .package-status,
  .package-builtin {
    width: fit-content;
    border: 1px solid
      color-mix(in srgb, var(--accent) 42%, var(--sheet-border, var(--border)));
    border-radius: var(--shape-xs);
    background: color-mix(in srgb, var(--accent) 9%, transparent);
    color: var(--text-secondary);
    padding: 1px 5px;
    font-size: 0.625rem;
    font-weight: 650;
    line-height: 1.35;
  }
  .package-status--active {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-readable-foreground);
  }
  .package-status--previewing {
    border-style: dashed;
    border-color: color-mix(
      in srgb,
      var(--accent) 76%,
      var(--sheet-border, var(--border))
    );
    background: color-mix(
      in srgb,
      var(--accent) 10%,
      var(--sheet-control-bg, var(--bg-primary))
    );
    color: var(--text-primary);
  }
  .btn-previewing {
    border-style: dashed;
    border-color: color-mix(
      in srgb,
      var(--accent) 76%,
      var(--sheet-border, var(--border))
    );
    background: color-mix(
      in srgb,
      var(--accent) 10%,
      var(--sheet-control-bg, var(--bg-primary))
    );
  }
  .btn[aria-pressed='true']:disabled {
    cursor: default;
    opacity: 1;
  }
  .package-version {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }
  .package-id {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    color: var(--text-tertiary);
    overflow-wrap: anywhere;
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
    background-color: var(--sheet-row-bg, var(--bg-primary));
    border: 1px dashed var(--sheet-border, var(--border));
    border-radius: var(--shape-md);
    font-size: 0.875rem;
  }
</style>
