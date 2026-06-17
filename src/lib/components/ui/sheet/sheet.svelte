<script lang="ts">
  import { Dialog as SheetPrimitive } from 'bits-ui';
  import { setContext } from 'svelte';

  let { open = $bindable(false), ...restProps }: SheetPrimitive.RootProps =
    $props();

  const openState = {
    get value() {
      return open;
    },
  };
  setContext('sheet-open', openState);
</script>

<SheetPrimitive.Root bind:open {...restProps} />
