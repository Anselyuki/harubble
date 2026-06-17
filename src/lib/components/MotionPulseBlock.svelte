<script lang="ts">
  interface Props {
    className?: string;
    reducedMotion?: boolean;
    delay?: number;
    duration?: number;
    style?: string;
  }

  let {
    className = '',
    reducedMotion = false,
    delay = 0,
    duration = 1.8,
    style = '',
  }: Props = $props();
</script>

<!-- motion-pulse 是 app.css 中的全局 @keyframes：内联 style 无法命中
     Svelte 作用域化后的组件内 keyframes，必须依赖全局名 -->
<div
  class={className}
  {style}
  style:animation={reducedMotion
    ? 'none'
    : `motion-pulse ${duration}s var(--ease-ios, ease-in-out) ${delay}s infinite`}
  style:opacity={reducedMotion ? 1 : undefined}
></div>
