<script lang="ts">
  interface Props {
    value: number;
    max: number;
    disabled: boolean;
    groupLabel: string;
    seekLabel: string;
    onInput: (event: Event) => void;
    onChange: (event: Event) => void;
  }

  let {
    value,
    max,
    disabled,
    groupLabel,
    seekLabel,
    onInput,
    onChange,
  }: Props = $props();
</script>

<div class="timeline" role="group" aria-label={groupLabel}>
  <div class="progress-track">
    <div class="track-bg" aria-hidden="true"></div>
    <input
      class="seek-slider"
      type="range"
      min="0"
      {max}
      {value}
      step="0.1"
      aria-label={seekLabel}
      {disabled}
      oninput={onInput}
      onchange={onChange}
    />
  </div>
</div>

<style>
  .timeline {
    --timeline-hit-size: 14px;
    position: absolute;
    inset: 0 0 auto;
    z-index: 5;
    min-width: 0;
    height: var(--timeline-hit-size);
  }

  .progress-track {
    position: relative;
    display: flex;
    align-items: flex-start;
    min-width: 0;
    height: var(--timeline-hit-size);
  }

  .track-bg {
    position: absolute;
    inset: 0 0 auto;
    height: var(--seek-track-size);
    overflow: hidden;
    border-radius: 0;
    background: linear-gradient(
      90deg,
      var(--album-accent) 0,
      var(--album-accent-hover) var(--player-progress-percent),
      rgba(120, 120, 128, 0.28) var(--player-progress-percent),
      rgba(120, 120, 128, 0.28) 100%
    );
  }

  .seek-slider {
    position: relative;
    z-index: 2;
    width: 100%;
    height: var(--timeline-hit-size);
    margin: 0;
    appearance: none;
    -webkit-appearance: none;
    background: transparent;
    cursor: pointer;
  }

  .seek-slider::-webkit-slider-runnable-track {
    height: var(--seek-track-size);
    border-radius: 0;
    background: transparent;
  }

  .seek-slider::-webkit-slider-thumb {
    width: 0;
    height: 0;
    margin-top: 0;
    appearance: none;
    -webkit-appearance: none;
    border: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    opacity: 0;
  }

  .seek-slider::-moz-range-track {
    height: var(--seek-track-size);
    border: 0;
    border-radius: 0;
    background: transparent;
  }

  .seek-slider::-moz-range-progress,
  .seek-slider::-moz-range-thumb {
    border: 0;
    background: transparent;
  }

  .seek-slider::-moz-range-thumb {
    width: 0;
    height: 0;
    border-radius: 0;
    box-shadow: none;
    opacity: 0;
  }

  .seek-slider:focus-visible {
    border-radius: var(--shape-pill);
    outline: none;
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--surface-highlight) 86%, white 14%),
      0 0 0 4px rgba(var(--album-accent-rgb), 0.28);
  }

  .seek-slider:disabled {
    cursor: not-allowed;
    opacity: 0.42;
  }
</style>
