<script lang="ts">
  import { untrack } from 'svelte';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import { getShellContext } from '$lib/contexts';
  import { gsap, getMotionDuration, MOTION } from '$lib/design/gsap';
  import SearchBar from './SearchBar.svelte';
  import SearchRecentQueries from './SearchRecentQueries.svelte';
  import SearchRecentlyPlayed from './SearchRecentlyPlayed.svelte';
  import SearchResultsPlaceholder from './SearchResultsPlaceholder.svelte';
  import type { createSearchController } from '$lib/features/search/controller.svelte';
  import type { Album } from '$lib/types';

  interface Props {
    runtime: {
      searchController: ReturnType<typeof createSearchController>;
      handleSelectAlbum: (album: Album) => void | Promise<void>;
      prefersReducedMotion: boolean;
      loadingAlbumCid: string | null;
    };
  }

  let { runtime }: Props = $props();

  const shell = getShellContext();
  const searchController = $derived(runtime.searchController);

  const isSearchMode = $derived(searchController.query.trim().length > 0);

  let discoveryEl: HTMLDivElement | undefined = $state();
  let placeholderEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!discoveryEl || runtime.prefersReducedMotion) return;
    gsap.fromTo(
      discoveryEl,
      { opacity: 0 },
      { opacity: 1, duration: getMotionDuration(MOTION.BASE), ease: 'ios' }
    );
  });

  $effect(() => {
    if (!placeholderEl || runtime.prefersReducedMotion) return;
    gsap.fromTo(
      placeholderEl,
      { opacity: 0, y: 10 },
      {
        opacity: 1,
        y: 0,
        duration: getMotionDuration(MOTION.SLOW),
        ease: 'ios-spring',
      }
    );
  });

  $effect(() => {
    untrack(() => searchController.init());
  });
</script>

<OverlayScrollbarsComponent
  class="search-scroll-container"
  options={shell.overlayScrollbarOptions}
  defer
>
  <div class="search-view">
    <div class="search-bar-region">
      <SearchBar
        query={searchController.query}
        scope={searchController.scope}
        onQueryChange={searchController.setQuery}
        onScopeChange={searchController.setScope}
        onSubmit={searchController.submitSearch}
      />
    </div>

    {#if isSearchMode}
      <div class="content-region" bind:this={placeholderEl}>
        <SearchResultsPlaceholder
          query={searchController.query}
          scope={searchController.scope}
        />
      </div>
    {:else}
      <div class="content-region discovery" bind:this={discoveryEl}>
        <SearchRecentQueries
          queries={searchController.recentQueries}
          reducedMotion={runtime.prefersReducedMotion}
          onSelect={searchController.rerunQuery}
        />
        <SearchRecentlyPlayed
          albums={searchController.recentPlayed}
          loading={searchController.loading}
          loadingAlbumCid={runtime.loadingAlbumCid}
          reducedMotion={runtime.prefersReducedMotion}
          onSelectAlbum={(album) => {
            void runtime.handleSelectAlbum(album);
          }}
        />
      </div>
    {/if}
  </div>
</OverlayScrollbarsComponent>

<style>
  :global(.search-scroll-container) {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .search-view {
    display: flex;
    flex-direction: column;
    gap: 0;
    min-height: 100%;
  }

  /*
   * 顶部间距收进 sticky 区域自身的 padding-top，配合 top: 0，
   * 让搜索框在滚动时保持相对位置不变，下方内容滑入其下层。
   */
  .search-bar-region {
    position: sticky;
    top: 0;
    z-index: 10;
    padding: 65px 24px 20px;
  }

  /* 全宽磨砂条：内容滚动到搜索框下层时被遮罩并向底部渐隐，营造层次感。 */
  .search-bar-region::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    background: linear-gradient(
      180deg,
      var(--bg-primary) 0%,
      color-mix(in srgb, var(--bg-primary) 55%, transparent) 60%,
      transparent 100%
    );
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    -webkit-mask-image: linear-gradient(
      180deg,
      #000 0%,
      #000 52%,
      transparent 100%
    );
    mask-image: linear-gradient(180deg, #000 0%, #000 52%, transparent 100%);
  }

  /*
   * 不支持 backdrop-filter 的环境（部分 Windows/Linux WebView）降级：
   * 改用更高不透明度的纯色渐变兜底遮罩，确保下层内容不透出。
   * 注意：@supports 只能检测能力，无法检测性能；低端设备的卡顿需另行处理。
   */
  @supports not (
    (backdrop-filter: blur(16px)) or (-webkit-backdrop-filter: blur(16px))
  ) {
    .search-bar-region::before {
      background: linear-gradient(
        180deg,
        var(--bg-primary) 0%,
        color-mix(in srgb, var(--bg-primary) 88%, transparent) 60%,
        transparent 100%
      );
    }
  }

  .content-region {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding: 0 24px 32px;
  }
</style>
