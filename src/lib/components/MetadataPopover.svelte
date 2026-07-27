<script lang="ts">
  import { Popover } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { getAudioMetadata } from '$lib/api';
  import type { AlbumDetail, AudioFileMetadata, SongEntry } from '$lib/types';
  import { animateIn } from '$lib/design/gsap';

  interface AlbumMetadata {
    kind: 'album';
    album: AlbumDetail;
  }

  interface SongMetadata {
    kind: 'song';
    song: SongEntry;
    albumCid: string;
    albumName: string;
  }

  export type MetadataTarget = AlbumMetadata | SongMetadata;

  interface Props {
    target: MetadataTarget;
  }

  let { target }: Props = $props();

  let audioMeta: AudioFileMetadata | null = $state(null);
  let audioMetaLoaded = $state(false);

  const isDownloaded = $derived(
    target.kind === 'song' && target.song.download.isDownloaded
  );

  $effect(() => {
    if (target.kind === 'song' && isDownloaded) {
      audioMetaLoaded = false;
      audioMeta = null;
      getAudioMetadata(target.albumName, target.song.name)
        .then((result) => {
          audioMeta = result;
          audioMetaLoaded = true;
        })
        .catch((_err: unknown) => {
          audioMetaLoaded = true;
        });
    }
  });

  const title = $derived.by(() => {
    return target.kind === 'album' ? target.album.name : target.song.name;
  });

  interface MetadataRow {
    label: string;
    value: string;
  }

  const triggerAriaLabel = $derived.by(() => {
    void localeState.current;
    return m.metadata_trigger_aria();
  });

  function formatDuration(secs: number): string {
    const minutes = Math.floor(secs / 60);
    const seconds = Math.floor(secs % 60);
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatSampleRate(hz: number): string {
    return `${(hz / 1000).toFixed(1)} kHz`;
  }

  interface TagRow {
    dimension: string;
    values: string;
  }

  const infoRows = $derived.by((): MetadataRow[] => {
    void localeState.current;
    if (target.kind === 'album') {
      const a = target.album;
      const result: MetadataRow[] = [
        { label: m.metadata_label_cid(), value: a.cid },
        { label: m.metadata_label_album_name(), value: a.name },
      ];
      if (a.artists && a.artists.length > 0) {
        result.push({
          label: m.metadata_label_artists(),
          value: a.artists.join(', '),
        });
      }
      if (a.belong) {
        result.push({ label: m.metadata_label_belong(), value: a.belong });
      }
      if (a.intro) {
        result.push({ label: m.metadata_label_intro(), value: a.intro });
      }
      if (a.coverUrl) {
        result.push({ label: m.metadata_label_cover_url(), value: a.coverUrl });
      }
      if (a.coverDeUrl) {
        result.push({
          label: m.metadata_label_cover_de_url(),
          value: a.coverDeUrl,
        });
      }
      result.push({
        label: m.metadata_label_song_count(),
        value: String(a.songs.length),
      });
      return result;
    }
    const s = target.song;
    return [
      { label: m.metadata_label_cid(), value: s.cid },
      { label: m.metadata_label_song_name(), value: s.name },
      { label: m.metadata_label_album_cid(), value: target.albumCid },
      { label: m.metadata_label_artists(), value: s.artists.join(', ') },
    ];
  });

  const tagRows = $derived.by((): TagRow[] => {
    void localeState.current;
    const tags = target.kind === 'album' ? target.album.tags : target.song.tags;
    return tags.map((t) => ({
      dimension: t.dimension,
      values: t.values.join(', '),
    }));
  });

  const audioRows = $derived.by((): MetadataRow[] => {
    void localeState.current;
    if (!audioMetaLoaded || !audioMeta) return [];
    const result: MetadataRow[] = [
      { label: m.metadata_label_format(), value: audioMeta.format },
      {
        label: m.metadata_label_sample_rate(),
        value: formatSampleRate(audioMeta.sampleRate),
      },
      { label: m.metadata_label_channels(), value: String(audioMeta.channels) },
    ];
    if (audioMeta.bitsPerSample != null) {
      result.push({
        label: m.metadata_label_bits_per_sample(),
        value: `${audioMeta.bitsPerSample} bit`,
      });
    }
    result.push({
      label: m.metadata_label_duration(),
      value: formatDuration(audioMeta.durationSecs),
    });
    if (audioMeta.bitrateKbps != null) {
      result.push({
        label: m.metadata_label_bitrate(),
        value: `${audioMeta.bitrateKbps} kbps`,
      });
    }
    result.push({
      label: m.metadata_label_file_size(),
      value: formatFileSize(audioMeta.fileSize),
    });
    return result;
  });

  let popoverInnerEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!popoverInnerEl) return;
    animateIn(
      popoverInnerEl,
      { opacity: 0, y: -4, scale: 0.97 },
      { opacity: 1, y: 0, scale: 1 },
      150,
      'ios-out'
    );
  });
</script>

<Popover.Root>
  <Popover.Trigger class="meta-trigger" aria-label={triggerAriaLabel}>
    {#if target.kind === 'album'}
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="6" cy="12" r="2"></circle>
        <circle cx="12" cy="12" r="2"></circle>
        <circle cx="18" cy="12" r="2"></circle>
      </svg>
    {:else}
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="1.5"></circle>
        <circle cx="12" cy="6.5" r="1.5"></circle>
        <circle cx="12" cy="17.5" r="1.5"></circle>
      </svg>
    {/if}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      class="meta-popover-content"
      side="bottom"
      align="start"
      sideOffset={6}
    >
      <div class="meta-popover-inner" bind:this={popoverInnerEl}>
        <div class="meta-popover-header">{title}</div>
        <div class="meta-popover-rows">
          {#each infoRows as row (row.label)}
            <div class="meta-popover-row">
              <span class="meta-popover-label">{row.label}</span>
              <span class="meta-popover-value">{row.value}</span>
            </div>
          {/each}
        </div>
        {#if tagRows.length > 0}
          <div class="meta-popover-section-divider">
            <span class="meta-popover-section-badge"
              >{m.metadata_section_tags()}</span
            >
          </div>
          <div class="meta-popover-rows">
            {#each tagRows as tag (tag.dimension)}
              <div class="meta-popover-row">
                <span class="meta-popover-label">{tag.dimension}</span>
                <span class="meta-popover-value">{tag.values}</span>
              </div>
            {/each}
          </div>
        {/if}
        {#if audioRows.length > 0}
          <div class="meta-popover-section-divider">
            <span class="meta-popover-section-badge"
              >{m.metadata_section_audio()}</span
            >
          </div>
          <div class="meta-popover-rows">
            {#each audioRows as row (row.label)}
              <div class="meta-popover-row">
                <span class="meta-popover-label">{row.label}</span>
                <span class="meta-popover-value">{row.value}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.meta-trigger) {
    appearance: none;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    flex-shrink: 0;
  }
  :global(.meta-trigger:hover) {
    background: rgba(var(--accent-rgb), 0.1);
    color: var(--accent);
  }
  :global(.meta-trigger svg) {
    width: 18px;
    height: 18px;
    fill: currentColor;
  }
  :global(.meta-popover-content) {
    z-index: var(--z-popover, 200);
    width: 320px;
    max-width: calc(100vw - 32px);
    max-height: 60vh;
    overflow-y: auto;
    padding: 12px 14px;
    border-radius: var(--shape-lg);
    background: var(--bg-primary);
    border: 1px solid rgba(var(--accent-rgb), 0.1);
    box-shadow:
      0 8px 32px rgba(15, 23, 42, 0.12),
      0 2px 8px rgba(15, 23, 42, 0.06);
  }
  .meta-popover-inner {
    display: contents;
  }
  .meta-popover-header {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid rgba(var(--accent-rgb), 0.08);
  }
  .meta-popover-rows {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .meta-popover-section-divider {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 0 8px;
  }
  .meta-popover-section-divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: rgba(var(--accent-rgb), 0.08);
  }
  .meta-popover-section-badge {
    font-size: 10px;
    font-weight: 600;
    line-height: 1;
    padding: 3px 8px;
    border-radius: var(--shape-pill);
    background: rgba(var(--accent-rgb), 0.12);
    color: var(--accent);
    flex-shrink: 0;
  }
  .meta-popover-row {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .meta-popover-label {
    flex-shrink: 0;
    min-width: 64px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .meta-popover-value {
    font-size: 11px;
    color: var(--text-primary);
    word-break: break-all;
    font-family: var(--font-mono);
    line-height: 1.4;
  }
</style>
