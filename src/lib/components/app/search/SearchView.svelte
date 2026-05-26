<script lang="ts">
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import { getShellContext } from '$lib/contexts';
  import { gsap, getMotionDuration } from '$lib/design/gsap';
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
      { opacity: 1, duration: getMotionDuration(200), ease: 'ios' }
    );
  });

  $effect(() => {
    if (!placeholderEl || runtime.prefersReducedMotion) return;
    gsap.fromTo(
      placeholderEl,
      { opacity: 0, y: 10 },
      { opacity: 1, y: 0, duration: getMotionDuration(240), ease: 'ios-spring' }
    );
  });

  $effect(() => {
    searchController.init();
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
          onSelect={searchController.rerunQuery}
        />
        <SearchRecentlyPlayed
          albums={searchController.recentPlayed}
          loading={searchController.loading}
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
    padding: 20px 24px 32px;
    min-height: 100%;
  }

  .search-bar-region {
    position: sticky;
    top: 0;
    z-index: 10;
    padding-bottom: 20px;
    background: transparent;
  }

  .content-region {
    display: flex;
    flex-direction: column;
    gap: 28px;
  }
</style>
