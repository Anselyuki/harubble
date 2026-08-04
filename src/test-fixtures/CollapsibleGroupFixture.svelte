<script lang="ts">
  import CollapsibleGroup from '$lib/components/ui/collapsible-group/CollapsibleGroup.svelte';
  import ListMusicIcon from '@lucide/svelte/icons/list-music';

  interface Props {
    empty?: boolean;
    onCreate?: () => void;
  }

  let { empty = true, onCreate = () => {} }: Props = $props();
</script>

<CollapsibleGroup title="Custom collections" icon={ListMusicIcon} {empty}>
  {#snippet actions()}
    <button type="button" onclick={onCreate}>Create collection</button>
  {/snippet}

  <div>Collection content</div>
</CollapsibleGroup>
