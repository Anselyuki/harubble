<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    currentSong: unknown;
    loadingDetail: boolean;
    selectedAlbum: unknown;
    children: Snippet;
  }

  let { currentSong, loadingDetail, selectedAlbum, children }: Props = $props();

  const contentClass = $derived.by(
    () =>
      `content album-workspace-shell${currentSong ? ' content-with-player' : ''}${loadingDetail && selectedAlbum ? ' content-pending' : ''}`
  );
</script>

<main class={contentClass}>
  {@render children()}
</main>
