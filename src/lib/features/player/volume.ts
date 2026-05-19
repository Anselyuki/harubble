/**
 * 滑块位置 [0,1] → 线性增益 [0,1]，使用二次曲线近似感知响度。
 * position=1.0 时精确返回 1.0，保证 bit-perfect 直通。
 */
export function sliderToGain(position: number): number {
  if (position >= 1) return 1;
  if (position <= 0) return 0;
  return position * position;
}

/**
 * 线性增益 [0,1] → 滑块位置 [0,1]（sliderToGain 的逆函数）。
 */
export function gainToSlider(gain: number): number {
  if (gain >= 1) return 1;
  if (gain <= 0) return 0;
  return Math.sqrt(gain);
}
