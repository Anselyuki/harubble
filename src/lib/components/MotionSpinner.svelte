<script lang="ts">
  interface Props {
    className?: string;
    reducedMotion?: boolean;
    duration?: number;
    style?: string;
  }

  let {
    className = '',
    reducedMotion = false,
    duration = 0.9,
    style = '',
  }: Props = $props();
</script>

<!-- motion-spin 是 app.css 中的全局 @keyframes：内联 style 无法命中
     Svelte 作用域化后的组件内 keyframes，必须依赖全局名 -->
<div
  class={className}
  {style}
  style:animation={reducedMotion
    ? 'none'
    : `motion-spin ${duration}s linear infinite`}
></div>
