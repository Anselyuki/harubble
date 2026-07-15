<script lang="ts">
  interface Props {
    className?: string;
    reducedMotion?: boolean;
    style?: string;
  }

  let { className = '', reducedMotion = false, style = '' }: Props = $props();
</script>

<!-- motion-spin 是 app.css 中的全局 @keyframes：内联 style 无法命中
     Svelte 作用域化后的组件内 keyframes，必须依赖全局名 -->
<div
  class={`motion-spinner${className ? ` ${className}` : ''}`}
  {style}
  style:animation={reducedMotion
    ? 'none'
    : 'motion-spin var(--motion-spinner) linear infinite'}
></div>

<style>
  .motion-spinner {
    box-sizing: border-box;
    width: 16px;
    height: 16px;
    flex: none;
    border: 2px solid color-mix(in srgb, currentColor 28%, transparent);
    border-top-color: currentColor;
    border-radius: 50%;
  }
</style>
