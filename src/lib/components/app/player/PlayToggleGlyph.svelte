<script lang="ts">
  /**
   * Play toggle glyph · Router 薄壳（Phase 3 Step 3.2）。
   *
   * 依据 `getVisualContract().family` 分发：
   * - `glass` → GlassPlayToggleView（iOS spring）
   * - `material` / `terminal` → MaterialPlayToggleView（terminal 通过 CSS
   *   `:root[data-theme-family='terminal']` 覆盖为直角 monochrome）
   *
   * 业务逻辑（LOADING_GRACE_MS / MIN_VISIBLE / 世代号）由 controller 收敛，
   * 三个 family 共享同一份 controller 实现。
   *
   * # 为什么保留旧文件路径
   *
   * 所有调用方通过 `import PlayToggleGlyph from '.../PlayToggleGlyph.svelte'`
   * 引用，保留原路径能让 Phase 3 重构对外看不见（"无缝"承诺）。
   */
  import { getVisualContract } from '$lib/features/shell/visualContract.svelte';
  import GlassPlayToggleView from './glass/GlassPlayToggleView.svelte';
  import MaterialPlayToggleView from './material/MaterialPlayToggleView.svelte';

  interface Props {
    isPlaying: boolean;
    isLoading?: boolean;
    isPending?: boolean;
    transitionKey?: number;
    reducedMotion?: boolean;
    size?: string;
  }

  let {
    isPlaying,
    isLoading = false,
    isPending = false,
    transitionKey = 0,
    reducedMotion = false,
    size = '24px',
  }: Props = $props();

  // getVisualContract 返回响应式 $state 引用，主题包切换时 $derived 自动重求值
  // getVisualContract 返回的是稳定的 $state 引用（每次调用返回同一对象）。
  // 响应性靠对 contract.family 的属性读取触发，不需要 $derived 包裹。
  const contract = getVisualContract();
</script>

{#if contract.family === 'material' || contract.family === 'terminal'}
  <MaterialPlayToggleView
    {isPlaying}
    {isLoading}
    {isPending}
    {transitionKey}
    {reducedMotion}
    {size}
  />
{:else}
  <GlassPlayToggleView
    {isPlaying}
    {isLoading}
    {isPending}
    {transitionKey}
    {reducedMotion}
    {size}
  />
{/if}
