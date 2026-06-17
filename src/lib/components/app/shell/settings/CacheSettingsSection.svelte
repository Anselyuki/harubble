<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';

  interface Props {
    isClearingAudioCache: boolean;
    sectionTitle: string;
    cacheDescription: string;
    cacheClearLabel: string;
    cacheClearingLabel: string;
    onClearAudioCache: () => void;
  }

  let {
    isClearingAudioCache,
    sectionTitle,
    cacheDescription,
    cacheClearLabel,
    cacheClearingLabel,
    onClearAudioCache,
  }: Props = $props();
</script>

<section class="sheet-section settings-section settings-action-section">
  <div class="settings-section-heading">
    <div>
      <h3>{sectionTitle}</h3>
      <p>{cacheDescription}</p>
    </div>
    <Button
      variant="secondary"
      disabled={isClearingAudioCache}
      onclick={onClearAudioCache}
      ><Trash2Icon data-icon="inline-start" />{isClearingAudioCache
        ? cacheClearingLabel
        : cacheClearLabel}</Button
    >
  </div>
</section>

<style>
  .settings-action-section {
    padding-block: 13px;
  }
</style>
