export function shouldConsumeVerticalWheel(
  deltaX: number,
  deltaY: number,
  scrollLeft: number,
  maxScroll: number
): boolean {
  if (Math.abs(deltaX) > Math.abs(deltaY) || maxScroll <= 0 || deltaY === 0) {
    return false;
  }
  if (deltaY < 0 && scrollLeft <= 0) return false;
  if (deltaY > 0 && scrollLeft >= maxScroll - 1) return false;
  return true;
}
