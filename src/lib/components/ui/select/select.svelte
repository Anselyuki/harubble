<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui';
  import { setContext } from 'svelte';

  let {
    open = $bindable(false),
    value = $bindable(),
    ...restProps
  }: SelectPrimitive.RootProps = $props();

  const openState = {
    get value() {
      return open;
    },
  };
  setContext('select-open', openState);
</script>

<SelectPrimitive.Root bind:open bind:value={value as never} {...restProps} />
