<script lang="ts">
  import { createAppRuntime } from '$lib/features/shell/appRuntime.svelte';
  import AppProviders from '$lib/components/app/shell/AppProviders.svelte';
  import TopToolbar from '$lib/components/app/shell/TopToolbar.svelte';
  import StatusToastHost from '$lib/components/app/shell/StatusToastHost.svelte';
  import AppSidebar from '$lib/components/app/sidebar/AppSidebar.svelte';
  import BrandLogo from '$lib/components/app/sidebar/BrandLogo.svelte';
  import BrandSlab from '$lib/components/app/sidebar/BrandSlab.svelte';
  import PlayerFlyoutStack from '$lib/components/app/player/PlayerFlyoutStack.svelte';
  import FullscreenPlayer from '$lib/components/app/player/FullscreenPlayer.svelte';
  import AppSideSheets from '$lib/components/app/shell/AppSideSheets.svelte';
  import CollectionFormDialog from '$lib/components/app/collection/CollectionFormDialog.svelte';
  import ViewRouter from '$lib/components/app/shell/ViewRouter.svelte';
  import {
    createSidebarAnimator,
    type SidebarAnimator,
  } from '$lib/design/sidebar-animator';
  import {
    createSidebarResize,
    animateSnapToWidth,
    type SidebarResizeHandle,
  } from '$lib/design/sidebar-resize';

  const runtime = createAppRuntime();

  let animator: SidebarAnimator | null = null;
  let resizeHandle: SidebarResizeHandle | null = null;
  let resizeHandleEl: HTMLElement | null = $state(null);
  let logoCharEls: HTMLSpanElement[] = $state([]);
  let shellEl: HTMLElement | null = $state(null);
  let sidebarEl: HTMLElement | null = $state(null);
  let navRegionEl: HTMLElement | null = $state(null);
  let collectionsRegionEl: HTMLElement | null = $state(null);
  let collectionsCollapsedEl: HTMLElement | null = $state(null);
  let bottomLabelEl: HTMLSpanElement | null = $state(null);
  let logoContainerEl: HTMLDivElement | null = $state(null);
  let logoSlabEl: HTMLElement | null = $state(null);
  let brandRegionEl: HTMLElement | null = $state(null);

  let contentCollapsed = $state(runtime.sidebarCollapsed);
  let contentInteractive = $state(!runtime.sidebarCollapsed);
  let layoutCollapsed = $state(runtime.sidebarCollapsed);
  let isDragging = $state(false);

  const COLLAPSED_WIDTH = 56;
  const MAX_SIDEBAR_WIDTH = 248;
  const COLLAPSE_THRESHOLD = 120;

  function handleCharsReady(els: HTMLSpanElement[]) {
    logoCharEls = els;
  }

  function onContentInteractive(interactive: boolean) {
    contentInteractive = interactive;
  }
  function onContentSwitch(collapsed: boolean) {
    contentCollapsed = collapsed;
  }
  function onLayoutSwitch(collapsed: boolean) {
    layoutCollapsed = collapsed;
  }

  /* eslint-disable @typescript-eslint/no-unnecessary-condition -- $state(null) refs are populated by bind:this at runtime */
  $effect(() => {
    if (
      shellEl &&
      sidebarEl &&
      logoContainerEl &&
      logoSlabEl &&
      bottomLabelEl &&
      navRegionEl &&
      collectionsRegionEl &&
      collectionsCollapsedEl &&
      logoCharEls.length === 12
    ) {
      if (animator) return;
      // 初始化 sidebar 宽度（animator 不再控制）
      const initWidth = runtime.sidebarCollapsed
        ? COLLAPSED_WIDTH
        : runtime.shellStore.sidebarWidth;
      shellEl.style.setProperty('--sidebar-width', `${initWidth}px`);

      animator = createSidebarAnimator({
        shellEl,
        sidebarEl,
        logoCharEls,
        logoContainerEl,
        logoSlabEl,
        navRegionEl,
        collectionsRegionEl,
        collectionsCollapsedEl,
        bottomLabelEl,
        initialCollapsed: runtime.sidebarCollapsed,
        onContentInteractive,
        onContentSwitch,
        onLayoutSwitch,
      });
    }
  });
  /* eslint-enable @typescript-eslint/no-unnecessary-condition */

  let prevCollapsed: boolean | null = null;
  $effect(() => {
    const curr = runtime.sidebarCollapsed;
    if (!animator) return;
    if (prevCollapsed === null) {
      prevCollapsed = curr;
      return;
    }
    if (curr === prevCollapsed) return;
    prevCollapsed = curr;

    // 同步 sidebar 宽度（animator 不再控制）
    const targetWidth = curr
      ? COLLAPSED_WIDTH
      : runtime.shellStore.sidebarWidth;
    animateSnapToWidth(shellEl!, targetWidth);

    if (curr) {
      animator.collapse();
    } else {
      animator.expand();
    }
  });

  $effect(() => {
    return () => {
      animator?.dispose();
      animator = null;
      resizeHandle?.dispose();
      resizeHandle = null;
    };
  });

  /* eslint-disable @typescript-eslint/no-unnecessary-condition -- $state(null) refs are populated by bind:this at runtime */
  $effect(() => {
    if (shellEl && resizeHandleEl) {
      if (resizeHandle) return;
      resizeHandle = createSidebarResize({
        shellEl,
        handleEl: resizeHandleEl,
        collapsedWidth: COLLAPSED_WIDTH,
        expandedWidth: MAX_SIDEBAR_WIDTH,
        threshold: COLLAPSE_THRESHOLD,
        getCollapsed: () => runtime.sidebarCollapsed,
        onWidthChange: (width) => {
          isDragging = true;
          // 拖曳期间实时更新 sidebar 宽度（brand-region 独立不受影响）
          shellEl!.style.setProperty('--sidebar-width', `${width}px`);
          // 展开稳定态下让 slab 右边界跟随侧栏宽度（logo 保持不动）；
          // 折叠态拖出时 animator 内部会自行跳过，slab 维持折叠宽度。
          animator?.updateSlabFollow(width);
        },
        onCrossThreshold: (collapsed) => {
          // 仅在「从折叠态向外拖出」时实时切换内容布局：
          // 此时 committed 的 sidebarCollapsed 仍为 true，直到松手才提交。
          // 让导航 / 收藏区随宽度跨越阈值即时在图标态 / 带标签态之间切换，
          // logo 与 slab 保持折叠态不动，松手后由 expandLogoOnly 补完 logo FLIP。
          if (runtime.sidebarCollapsed) {
            animator?.previewContentCollapsed(collapsed);
          }
        },
        onDragEnd: (finalWidth, shouldCollapse) => {
          isDragging = false;
          const wasCollapsed = runtime.sidebarCollapsed;

          if (shouldCollapse !== wasCollapsed) {
            // 跨阈值——切换折叠态
            if (!shouldCollapse) {
              // 折叠 → 展开：内容已在拖曳期间实时展开，这里只补 logo 的
              // 竖排 → 横排 FLIP，并以释放位置作为新的展开宽度持久化。
              // 抢先把 prevCollapsed 同步为目标值，抑制共享 $effect 再次跑
              // 完整 expand()（那会把已展开的内容重置回折叠态再重放）。
              runtime.shellStore.sidebarWidth = finalWidth;
              prevCollapsed = false;
              runtime.shellStore.sidebarCollapsed = false;
              animator?.expandLogoOnly();
            } else {
              // 展开 → 折叠：内容在拖曳期间保持展开，交由共享 $effect 跑完整 collapse()
              runtime.shellStore.sidebarCollapsed = true;
            }
          } else if (wasCollapsed) {
            // 折叠态未跨阈值——内容回到折叠布局并弹回折叠宽度
            animator?.previewContentCollapsed(true);
            animateSnapToWidth(shellEl!, COLLAPSED_WIDTH);
          } else {
            // 展开态未跨阈值——保留当前宽度并持久化
            runtime.shellStore.sidebarWidth = finalWidth;
          }
        },
      });
    }
  });
  /* eslint-enable @typescript-eslint/no-unnecessary-condition */

  /**
   * 同步品牌浮层（logo + slab）的实际高度到 --brand-region-height。
   *
   * .brand-region 是 position:absolute 浮层，不参与侧栏 flex 流，
   * 由 .sidebar-brand-spacer 用该变量预留等高安全区把导航推到浮层下方。
   * 折叠态下字母竖排会显著增高，必须实时跟随，否则 spacer 停在 80px
   * 兜底值，logo/slab 会直接覆盖在导航之上。
   */
  /* eslint-disable @typescript-eslint/no-unnecessary-condition -- $state(null) refs are populated by bind:this at runtime */
  $effect(() => {
    if (!shellEl || !brandRegionEl) return;
    const shell = shellEl;
    const region = brandRegionEl;

    const syncHeight = () => {
      shell.style.setProperty(
        '--brand-region-height',
        `${region.offsetHeight}px`
      );
    };

    syncHeight();

    if (typeof ResizeObserver === 'undefined') return;

    let rafId = 0;
    const observer = new ResizeObserver(() => {
      // 延迟到下一帧再写回，避免回调内同步改动布局触发
      // "ResizeObserver loop completed with undelivered notifications" 警告
      if (rafId) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = 0;
        syncHeight();
      });
    });
    observer.observe(region);

    return () => {
      if (rafId) cancelAnimationFrame(rafId);
      observer.disconnect();
    };
  });
  /* eslint-enable @typescript-eslint/no-unnecessary-condition */
</script>

{#if runtime.isMacOS}
  <div
    class="macos-window-drag-region"
    data-tauri-drag-region
    aria-hidden="true"
  ></div>
{/if}

<StatusToastHost />

<AppProviders {runtime}>
  <div
    class="app-shell"
    class:macos-overlay={runtime.isMacOS}
    bind:this={shellEl}
  >
    <div class="brand-region" aria-hidden="true" bind:this={brandRegionEl}>
      <BrandSlab bind:slabEl={logoSlabEl} />
      <BrandLogo
        isMacOS={runtime.isMacOS}
        {layoutCollapsed}
        bind:containerEl={logoContainerEl}
        onCharsReady={handleCharsReady}
      />
    </div>

    <AppSidebar
      isMacOS={runtime.isMacOS}
      currentView={runtime.currentView}
      {contentCollapsed}
      {contentInteractive}
      onNavigate={(view) => {
        runtime.navigateToTop(view);
      }}
      collections={runtime.collectionController.collections}
      selectedCollectionId={runtime.collectionController.selectedCollectionId}
      isCollectionsLoading={runtime.collectionController.isLoading}
      onSelectCollection={(id) => runtime.openCollection(id)}
      onCreateCollection={runtime.collectionController.openCreateDialog}
      onRequestExpand={runtime.toggleSidebar}
      bind:sidebarEl
      bind:navRegionEl
      bind:collectionsRegionEl
      bind:collectionsCollapsedEl
      bind:bottomLabelEl
    />

    <div
      class="sidebar-resize-handle"
      class:dragging={isDragging}
      bind:this={resizeHandleEl}
      aria-hidden="true"
    ></div>

    <section class="main-region">
      {#if runtime.isMacOS}
        <div
          class="main-drag-region"
          data-tauri-drag-region
          aria-hidden="true"
        ></div>
      {/if}

      <TopToolbar
        activeDownloadCount={runtime.activeDownloadCount}
        isRefreshing={runtime.isRefreshing}
        settingsOpen={runtime.settingsOpen}
        downloadPanelOpen={runtime.downloadPanelOpen}
        onRefresh={runtime.handleRefresh}
        onOpenDownloads={runtime.handleToggleDownloads}
        onOpenSettings={runtime.handleToggleSettings}
      />

      <ViewRouter {runtime} />

      <PlayerFlyoutStack />

      {#if runtime.fullscreenOpen && runtime.currentSong}
        <FullscreenPlayer />
      {/if}

      <AppSideSheets
        SettingsSheetView={runtime.SettingsSheetView}
        DownloadTasksSheetView={runtime.DownloadTasksSheetView}
        bind:settingsOpen={runtime.shellStore.settingsOpen}
        bind:downloadPanelOpen={runtime.shellStore.downloadPanelOpen}
        bind:format={runtime.settingsState.format}
        bind:outputDir={runtime.settingsState.outputDir}
        bind:downloadLyrics={runtime.settingsState.downloadLyrics}
        bind:notifyOnDownloadComplete={
          runtime.settingsState.notifyOnDownloadComplete
        }
        bind:notifyOnPlaybackChange={
          runtime.settingsState.notifyOnPlaybackChange
        }
        bind:logLevel={runtime.settingsState.logLevel}
        bind:locale={runtime.settingsState.locale}
        bind:themePresetId={runtime.settingsState.themePresetId}
        bind:themeCustomColors={runtime.settingsState.themeCustomColors}
        settingsLogRefreshToken={runtime.settingsState.settingsLogRefreshToken}
        notifyInfo={runtime.notifyInfo}
        notifyError={runtime.notifyError}
        onOutputDirChange={runtime.handleOutputDirChange}
      />
    </section>
  </div>

  <CollectionFormDialog
    bind:open={runtime.collectionController.formDialogOpen}
    mode={runtime.collectionController.formDialogMode}
    initialName={runtime.collectionController.selectedCollection?.name ?? ''}
    initialDescription={runtime.collectionController.selectedCollection
      ?.description ?? ''}
    onSubmit={(name, description) => {
      if (runtime.collectionController.formDialogMode === 'create') {
        return runtime.collectionController.handleCreate(name, description);
      }
      const id = runtime.collectionController.selectedCollectionId;
      if (id) {
        return runtime.collectionController.handleUpdate(id, name, description);
      }
    }}
    onClose={runtime.collectionController.closeFormDialog}
  />
</AppProviders>
