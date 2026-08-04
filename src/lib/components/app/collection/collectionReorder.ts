export function reorderItems<T>(
  items: readonly T[],
  sourceIndex: number,
  targetIndex: number
): T[] {
  if (
    sourceIndex === targetIndex ||
    sourceIndex < 0 ||
    sourceIndex >= items.length ||
    targetIndex < 0 ||
    targetIndex >= items.length
  ) {
    return [...items];
  }

  const reordered = [...items];
  const [moved] = reordered.splice(sourceIndex, 1);
  reordered.splice(targetIndex, 0, moved);
  return reordered;
}

export function getKeyboardReorderTarget(
  key: string,
  currentIndex: number,
  itemCount: number
): number | null {
  if (key === 'ArrowUp') return Math.max(0, currentIndex - 1);
  if (key === 'ArrowDown') return Math.min(itemCount - 1, currentIndex + 1);
  if (key === 'Home') return 0;
  if (key === 'End') return itemCount - 1;
  return null;
}
