<script lang="ts">
  import { Dialog as DialogPrimitive } from 'bits-ui';
  import { setContext } from 'svelte';

  let { open = $bindable(false), ...restProps }: DialogPrimitive.RootProps =
    $props();

  const openState = {
    get value() {
      return open;
    },
  };
  setContext('dialog-open', openState);
</script>

<DialogPrimitive.Root bind:open {...restProps} />
