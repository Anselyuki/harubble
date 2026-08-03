<script lang="ts">
  /**
   * VolumeCapsule router. The five built-in Ark UI families share one
   * semantic view and HCI motion model; static CSS keeps their visual geometry
   * distinct. Legacy families retain their existing glass/material views.
   */
  import {
    getVisualContract,
    isArkUiThemeFamily,
    usesStructuredControls,
  } from '$lib/features/shell/visualContract.svelte';
  import FamilyVolumeCapsuleView from './family/FamilyVolumeCapsuleView.svelte';
  import GlassVolumeCapsuleView from './glass/GlassVolumeCapsuleView.svelte';
  import MaterialVolumeCapsuleView from './material/MaterialVolumeCapsuleView.svelte';

  interface Props {
    volume: number;
    muted: boolean;
    open: boolean;
    onopen?: () => void;
    onclose?: () => void;
    onVolumeChange?: (gain: number) => void | Promise<void>;
    onToggleMute?: () => void;
  }

  let {
    volume,
    muted,
    open,
    onopen,
    onclose,
    onVolumeChange,
    onToggleMute,
  }: Props = $props();

  // getVisualContract 返回稳定的 $state 引用；响应性走属性读取，不需 $derived 包裹。
  const contract = getVisualContract();
</script>

{#if isArkUiThemeFamily(contract.family)}
  <FamilyVolumeCapsuleView
    family={contract.family}
    {volume}
    {muted}
    {open}
    {onopen}
    {onclose}
    {onVolumeChange}
    {onToggleMute}
  />
{:else if usesStructuredControls(contract.family)}
  <MaterialVolumeCapsuleView
    {volume}
    {muted}
    {open}
    {onopen}
    {onclose}
    {onVolumeChange}
    {onToggleMute}
  />
{:else}
  <GlassVolumeCapsuleView
    {volume}
    {muted}
    {open}
    {onopen}
    {onclose}
    {onVolumeChange}
    {onToggleMute}
  />
{/if}
