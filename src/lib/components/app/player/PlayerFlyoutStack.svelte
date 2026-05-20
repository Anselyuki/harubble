<script lang="ts">
  import { animateIn, animateOut, killTweens } from '$lib/design/gsap';
  import PlayerDock from '$lib/components/app/player/PlayerDock.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import {
    getPlayerContext,
    getDownloadContext,
    getShellContext,
  } from '$lib/contexts';

  const player = getPlayerContext();
  const download = getDownloadContext();
  const shell = getShellContext();

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  const downloadState = $derived(
    (player.currentSong
      ? download.getSongDownloadState(player.currentSong.cid)
      : 'idle') as SongDownloadState
  );
  const downloadDisabled = $derived(
    player.currentSong
      ? download.isSongDownloadInteractionBlocked(player.currentSong.cid)
      : false
  );
  const onTogglePlay = $derived(
    player.isPlaying ? player.pause : player.resume
  );

  function handleDownload() {
    if (player.currentSong) {
      void download.handleSongDownload(player.currentSong.cid);
    }
  }

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      queueEyebrow: m.player_queue_eyebrow(),
      queueTitle: m.player_queue_title(),
      queueEmpty: m.player_queue_empty(),
    };
  });
  const queueCountLabel = $derived.by(() => {
    void localeState.current;
    return m.player_queue_count({ count: player.playbackOrder.length });
  });

  let wrapperEl = $state<HTMLElement | undefined>();
  let wrapperMounted = $state(false);

  let flyoutEl = $state<HTMLElement | undefined>();
  let flyoutMounted = $state(false);

  const hasSong = $derived(!!player.currentSong);
  const playlistOpen = $derived(player.playlistOpen);

  $effect(() => {
    if (hasSong) wrapperMounted = true;
  });

  $effect(() => {
    if (!wrapperEl || !hasSong) return;
    animateIn(
      wrapperEl,
      { opacity: 0, y: 18 },
      { opacity: 1, y: 0 },
      220,
      'ios-spring'
    );
    return () => killTweens(wrapperEl!);
  });

  $effect(() => {
    if (hasSong || !wrapperMounted || !wrapperEl) return;
    animateOut(wrapperEl, { opacity: 0 }, 220, {
      onComplete: () => {
        wrapperMounted = false;
      },
    });
  });

  $effect(() => {
    if (playlistOpen) flyoutMounted = true;
  });

  $effect(() => {
    if (!flyoutEl || !playlistOpen) return;
    animateIn(
      flyoutEl,
      { opacity: 0, y: 12 },
      { opacity: 1, y: 0 },
      180,
      'ios-spring'
    );
    return () => killTweens(flyoutEl!);
  });

  $effect(() => {
    if (playlistOpen || !flyoutMounted || !flyoutEl) return;
    animateOut(flyoutEl, { opacity: 0, y: 8 }, 180, {
      onComplete: () => {
        flyoutMounted = false;
      },
    });
  });
</script>

{#if wrapperMounted}
  <div class="player-dock-stack-wrapper" bind:this={wrapperEl}>
    <div
      class="player-dock-stack"
      data-panel={player.playlistOpen ? 'playlist' : 'none'}
    >
      {#if flyoutMounted}
        <section
          class="player-flyout"
          data-panel="playlist"
          bind:this={flyoutEl}
        >
          <div class="player-flyout-header">
            <div>
              <p class="player-flyout-eyebrow">{labels.queueEyebrow}</p>
              <h3 class="player-flyout-title">{labels.queueTitle}</h3>
            </div>
            <span class="player-flyout-count">{queueCountLabel}</span>
          </div>
          {#if player.playbackOrder.length > 0}
            <div class="player-playlist-list">
              {#each player.playbackOrder as entry, index (entry.cid)}
                <button
                  type="button"
                  class={`player-playlist-item${entry.cid === player.currentSong?.cid ? ' active' : ''}`}
                  aria-label={m.player_queue_item_aria({
                    index: index + 1,
                    name: entry.name,
                  })}
                  aria-current={entry.cid === player.currentSong?.cid
                    ? 'true'
                    : undefined}
                  onclick={() => {
                    void player.playQueueEntry(
                      entry,
                      player.playbackOrder,
                      index
                    );
                  }}
                >
                  <span class="player-playlist-index"
                    >{String(index + 1).padStart(2, '0')}</span
                  >
                  <span class="player-playlist-meta"
                    ><span class="player-playlist-name">{entry.name}</span><span
                      class="player-playlist-artists"
                      >{entry.artists.join(' · ')}</span
                    ></span
                  >
                </button>
              {/each}
            </div>
          {:else}
            <div class="player-flyout-empty">{labels.queueEmpty}</div>
          {/if}
        </section>
      {/if}
      <PlayerDock
        song={player.currentSong}
        isPlaying={player.isPlaying}
        isPaused={player.isPaused}
        hasPrevious={player.hasPrevious}
        hasNext={player.hasNext}
        progress={player.progress}
        duration={player.duration}
        isLoading={player.isLoading}
        isShuffled={player.shuffleEnabled}
        repeatMode={player.repeatMode}
        lyricsActive={player.lyricsOpen}
        lyricsUnavailable={player.lyricsUnavailable}
        lyricsLoading={player.lyricsLoading}
        lyricsError={player.lyricsError ?? ''}
        lyricsLines={player.lyricsLines}
        activeLyricIndex={player.activeLyricIndex}
        playlistActive={player.playlistOpen}
        {downloadState}
        {downloadDisabled}
        volume={player.volume}
        muted={player.muted}
        onVolumeChange={player.setVolume}
        onToggleMute={player.toggleMute}
        reducedMotion={shell.prefersReducedMotion}
        onPrevious={player.playPrevious}
        {onTogglePlay}
        onSeek={player.seek}
        onNext={player.playNext}
        onShuffleChange={player.toggleShuffle}
        onRepeatModeChange={player.toggleRepeat}
        onToggleLyrics={player.toggleLyrics}
        onTogglePlaylist={player.togglePlaylist}
        onToggleFullscreen={player.toggleFullscreen}
        onDownload={handleDownload}
      />
    </div>
  </div>
{/if}
