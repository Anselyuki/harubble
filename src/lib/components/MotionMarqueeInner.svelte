<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /**
     * 是否激活滚动。仅在为 true 且 reducedMotion 为 false 时才启动动画；
     * 未激活时元素回落为普通 inline-block span，不占用额外末尾间距。
     */
    active: boolean;
    /** reduced-motion 开启时强制关闭动画；来自 shell/env 的 prefersReducedMotion。 */
    reducedMotion?: boolean;
    /** 一次循环耗时（秒），默认 8s，匹配全屏播放器原有节奏。 */
    duration?: number;
    /** 激活时末尾追加的空白，制造首尾循环之间的视觉间隙。 */
    gap?: string;
    /** 附加 class；容器本身是一个 inline span。 */
    className?: string;
    children: Snippet;
  }

  let {
    active,
    reducedMotion = false,
    duration = 8,
    gap = '3em',
    className = '',
    children,
  }: Props = $props();

  const shouldAnimate = $derived(active && !reducedMotion);
</script>

<!-- motion-marquee 是 app.css 中的全局 @keyframes：内联 style 无法命中
     Svelte 作用域化后的组件内 keyframes，必须依赖全局名 -->
<span
  class={className}
  style:display="inline-block"
  style:padding-right={active ? gap : '0'}
  style:animation={shouldAnimate
    ? `motion-marquee ${duration}s linear infinite`
    : 'none'}
>
  {@render children()}
</span>
