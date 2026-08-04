import { describe, expect, it } from 'vitest';
import { getNextRepeatMode } from './repeatMode';

describe('repeat mode', () => {
  it('cycles through off, all, and one', () => {
    expect(getNextRepeatMode('off')).toBe('all');
    expect(getNextRepeatMode('all')).toBe('one');
    expect(getNextRepeatMode('one')).toBe('off');
  });
});
