<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
  import { setContext } from 'svelte';

  let {
    open = $bindable(false),
    ...restProps
  }: AlertDialogPrimitive.RootProps = $props();

  const openState = {
    get value() {
      return open;
    },
  };
  setContext('alert-dialog-open', openState);
</script>

<AlertDialogPrimitive.Root bind:open {...restProps} />
