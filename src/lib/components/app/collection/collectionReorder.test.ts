import { describe, expect, it } from 'vitest';
import { getKeyboardReorderTarget, reorderItems } from './collectionReorder';

describe('collection reorder accessibility helpers', () => {
  it('moves items without mutating the source order', () => {
    const source = ['a', 'b', 'c'];
    expect(reorderItems(source, 0, 2)).toEqual(['b', 'c', 'a']);
    expect(source).toEqual(['a', 'b', 'c']);
  });

  it('maps arrow and boundary keys to valid positions', () => {
    expect(getKeyboardReorderTarget('ArrowUp', 0, 3)).toBe(0);
    expect(getKeyboardReorderTarget('ArrowDown', 2, 3)).toBe(2);
    expect(getKeyboardReorderTarget('Home', 2, 3)).toBe(0);
    expect(getKeyboardReorderTarget('End', 0, 3)).toBe(2);
    expect(getKeyboardReorderTarget('Enter', 1, 3)).toBeNull();
  });
});
