<script lang="ts">
  /**
   * VolumeCapsule · Router 薄壳（Phase 3 Step 3.2）。
   *
   * 根据 `getVisualContract().family` 分发到 glass / material view。
   * 业务逻辑（state 机、collapse timer）通过共享 helper（volume-capsule-state /
   * volume-capsule-timer）在两个 view 里保持一致；animator 由各 view 自选
   * `createGlassCapsuleAnimator` / `createMaterialCapsuleAnimator`。
   *
   * 保留原路径让 Phase 3 重构对外看不见（"无缝"承诺）。
   */
  import {
    getVisualContract,
    usesStructuredControls,
  } from '$lib/features/shell/visualContract.svelte';
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

{#if usesStructuredControls(contract.family)}
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
