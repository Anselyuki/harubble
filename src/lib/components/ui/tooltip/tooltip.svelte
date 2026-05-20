<script lang="ts">
  import { Tooltip as TooltipPrimitive } from 'bits-ui';
  import { setContext } from 'svelte';

  let { open = $bindable(false), ...restProps }: TooltipPrimitive.RootProps =
    $props();

  const openState = {
    get value() {
      return open;
    },
  };
  setContext('tooltip-open', openState);
</script>

<TooltipPrimitive.Root bind:open {...restProps} />
