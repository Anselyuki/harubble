<script lang="ts">
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import HomeLatestAlbums from './HomeLatestAlbums.svelte';
  import HomeSeriesGroups from './HomeSeriesGroups.svelte';
  import HomeTagGroups from './HomeTagGroups.svelte';
  import HomeRecentHistory from './HomeRecentHistory.svelte';
  import HomeStatusDashboard from './HomeStatusDashboard.svelte';
  import {
    getShellContext,
    getPlayerContext,
    getDownloadContext,
  } from '$lib/contexts';
  import type {
    Album,
    SeriesGroup,
    HistoryEntry,
    HomepageStatus,
    SongEntry,
    PlaybackQueueEntry,
    TagDimension,
    TagGroup,
  } from '$lib/types';

  interface Props {
    runtime: {
      homeController: {
        latestAlbums: Album[];
        seriesGroups: SeriesGroup[];
        recentHistory: HistoryEntry[];
        status: HomepageStatus | null;
        loading: boolean;
        belongReady: boolean;
        tagDimensions: TagDimension[];
        tagGroups: TagGroup[];
        selectedDimensionKey: string | null;
        selectDimension: (key: string) => void;
      };
      handleSelectAlbum: (album: Album) => void | Promise<void>;
      handlePlayCollectionSong: (
        song: SongEntry,
        queue: PlaybackQueueEntry[]
      ) => void | Promise<void>;
      prefersReducedMotion: boolean;
      loadingAlbumCid: string | null;
      requestClearListeningHistory: () => void;
    };
  }

  let { runtime }: Props = $props();

  const shell = getShellContext();
  const player = getPlayerContext();
  const download = getDownloadContext();
</script>

<OverlayScrollbarsComponent
  class="home-scroll-container"
  options={shell.overlayScrollbarOptions}
  defer
>
  <div class="home-view">
    <HomeLatestAlbums
      albums={runtime.homeController.latestAlbums}
      loading={runtime.homeController.loading}
      loadingAlbumCid={runtime.loadingAlbumCid}
      reducedMotion={runtime.prefersReducedMotion}
      onSelect={runtime.handleSelectAlbum}
    />

    <HomeSeriesGroups
      groups={runtime.homeController.seriesGroups}
      belongReady={runtime.homeController.belongReady}
      reducedMotion={runtime.prefersReducedMotion}
      onSelectAlbum={runtime.handleSelectAlbum}
    />

    <HomeTagGroups
      dimensions={runtime.homeController.tagDimensions}
      groups={runtime.homeController.tagGroups}
      selectedDimensionKey={runtime.homeController.selectedDimensionKey}
      onSelectDimension={runtime.homeController.selectDimension}
      onSelectAlbum={runtime.handleSelectAlbum}
    />

    <HomeRecentHistory
      entries={runtime.homeController.recentHistory}
      onPlay={(entry) => {
        const queueEntry = {
          cid: entry.songCid,
          name: entry.songName,
          artists: entry.artists,
          coverUrl: entry.coverUrl,
        };
        void runtime.handlePlayCollectionSong(
          {
            cid: entry.songCid,
            name: entry.songName,
            artists: entry.artists,
            download: {
              isDownloaded: false,
              downloadStatus: 'unknown',
              inventoryVersion: '',
            },
            tags: [],
          },
          [queueEntry]
        );
      }}
      onClear={runtime.requestClearListeningHistory}
    />

    <HomeStatusDashboard
      status={runtime.homeController.status}
      currentSong={player.currentSong}
      isPlaying={player.isPlaying}
      activeDownloadCount={download.activeDownloadCount}
    />
  </div>
</OverlayScrollbarsComponent>

<style>
  :global(.home-scroll-container) {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .home-view {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: 1.25rem 1.5rem 2rem;
  }
</style>
