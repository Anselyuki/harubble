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
  import ClearListeningHistoryDialog from '$lib/components/app/home/ClearListeningHistoryDialog.svelte';
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
  import { MOTION } from '$lib/design/gsap';
  import { syncBrandHeight } from '$lib/design/actions';
  import * as m from '$lib/paraglide/messages.js';

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

  let contentCollapsed = $state(runtime.sidebarCollapsed);
  let contentInteractive = $state(!runtime.sidebarCollapsed);
  let layoutCollapsed = $state(runtime.sidebarCollapsed);
  let isDragging = $state(false);
  let currentSidebarWidth = $state(
    runtime.sidebarCollapsed ? 56 : runtime.shellStore.sidebarWidth
  );

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
      currentSidebarWidth = initWidth;

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

    // 视图驱动的强制收缩/展开（如进入 Tag Editor）走压缩同步编排：
    // 侧栏宽度与 logo 动画压成单一时间线，与页面转场（MOTION.PAGE）同时收尾。
    const compact = runtime.shellStore.sidebarTransientCompact;

    // 同步 sidebar 宽度（animator 不再控制）
    const targetWidth = curr
      ? COLLAPSED_WIDTH
      : runtime.shellStore.sidebarWidth;
    animateSnapToWidth(
      shellEl!,
      targetWidth,
      compact ? MOTION.PAGE : undefined
    );
    currentSidebarWidth = targetWidth;

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
          currentSidebarWidth = width;
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
              currentSidebarWidth = COLLAPSED_WIDTH;
            }
          } else if (wasCollapsed) {
            // 折叠态未跨阈值——内容回到折叠布局并弹回折叠宽度
            animator?.previewContentCollapsed(true);
            animateSnapToWidth(shellEl!, COLLAPSED_WIDTH);
            currentSidebarWidth = COLLAPSED_WIDTH;
          } else {
            // 展开态未跨阈值——保留当前宽度并持久化
            runtime.shellStore.sidebarWidth = finalWidth;
          }
        },
      });
    }
  });
  /* eslint-enable @typescript-eslint/no-unnecessary-condition */
</script>

<StatusToastHost />

<AppProviders {runtime}>
  <div
    class="app-shell"
    class:macos-overlay={runtime.isMacOS}
    bind:this={shellEl}
  >
    <div class="brand-region" aria-hidden="true" use:syncBrandHeight>
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
      onSelectCollection={(id) => runtime.openCollection(id)}
      onCreateCollection={runtime.collectionController.openCreateDialog}
      onRequestExpand={runtime.toggleSidebar}
      bind:sidebarEl
      bind:navRegionEl
      bind:collectionsRegionEl
      bind:collectionsCollapsedEl
      bind:bottomLabelEl
    />

    <!-- svelte-ignore a11y_no_noninteractive_tabindex (ARIA separator is keyboard-resizable) -->
    <div
      class="sidebar-resize-handle"
      class:dragging={isDragging}
      bind:this={resizeHandleEl}
      role="separator"
      aria-label={m.sidebar_resize_label()}
      aria-orientation="vertical"
      aria-valuemin={COLLAPSED_WIDTH}
      aria-valuemax={MAX_SIDEBAR_WIDTH}
      aria-valuenow={Math.round(currentSidebarWidth)}
      tabindex="0"
    ></div>

    <main class="main-region">
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
        bind:colorScheme={runtime.settingsState.colorScheme}
        bind:dynamicAlbumAccent={runtime.settingsState.dynamicAlbumAccent}
        settingsLogRefreshToken={runtime.settingsState.settingsLogRefreshToken}
        settingsInitialSection={runtime.shellStore.settingsInitialSection}
        notifyInfo={runtime.notifyInfo}
        notifyError={runtime.notifyError}
        onOutputDirChange={runtime.handleOutputDirChange}
      />
    </main>
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
  <ClearListeningHistoryDialog
    open={runtime.clearListeningHistoryDialogOpen}
    onOpenChange={(open) => (runtime.clearListeningHistoryDialogOpen = open)}
    onConfirm={runtime.confirmClearListeningHistory}
  />
</AppProviders>
