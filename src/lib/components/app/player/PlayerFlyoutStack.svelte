<script lang="ts">
  import { fade, fly } from 'svelte/transition';
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

  function dur(base: number): number {
    return shell.prefersReducedMotion ? 0 : base;
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
</script>

{#if player.currentSong}
  <div
    class="player-dock-stack-wrapper"
    in:fly={{ y: 18, duration: dur(220) }}
    out:fade={{ duration: dur(220) }}
  >
    <div
      class="player-dock-stack"
      data-panel={player.playlistOpen ? 'playlist' : 'none'}
    >
      {#if player.playlistOpen}
        <section
          class="player-flyout"
          data-panel="playlist"
          in:fly={{ y: 12, duration: dur(180) }}
          out:fly={{ y: 8, duration: dur(180) }}
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
