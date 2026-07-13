import { describe, expect, it } from 'vitest';
import { createEventSequence } from './eventSequence.svelte';

describe('createEventSequence', () => {
  it('next() returns monotonically increasing tokens per key', () => {
    const seq = createEventSequence();
    expect(seq.next('a')).toBe(1);
    expect(seq.next('a')).toBe(2);
    expect(seq.next('a')).toBe(3);
  });

  it('different keys have independent counters', () => {
    const seq = createEventSequence();
    expect(seq.next('a')).toBe(1);
    expect(seq.next('b')).toBe(1);
    expect(seq.next('a')).toBe(2);
    expect(seq.next('b')).toBe(2);
  });

  it('isCurrent returns true only for the most recent token', () => {
    const seq = createEventSequence();
    const t1 = seq.next('a');
    expect(seq.isCurrent('a', t1)).toBe(true);
    const t2 = seq.next('a');
    expect(seq.isCurrent('a', t1)).toBe(false);
    expect(seq.isCurrent('a', t2)).toBe(true);
  });

  it('isCurrent returns false for tokens on unknown keys', () => {
    const seq = createEventSequence();
    expect(seq.isCurrent('never-used', 1)).toBe(false);
  });

  it('invalidate() bumps the counter so previous tokens become stale', () => {
    const seq = createEventSequence();
    const t1 = seq.next('a');
    seq.invalidate('a');
    expect(seq.isCurrent('a', t1)).toBe(false);
  });

  it('token 0 is never returned by next()', () => {
    const seq = createEventSequence();
    expect(seq.next('fresh-key')).toBeGreaterThan(0);
  });
});
