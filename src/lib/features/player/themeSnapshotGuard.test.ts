import { describe, expect, it } from 'vitest';
import { shouldAcceptThemeSnapshot } from './themeSnapshotGuard';

describe('shouldAcceptThemeSnapshot', () => {
  it('accepts newer snapshots and idempotent replays', () => {
    expect(shouldAcceptThemeSnapshot(2, 'active', 3, 'next')).toBe(true);
    expect(shouldAcceptThemeSnapshot(2, 'active', 2, 'active')).toBe(true);
  });

  it('rejects older snapshots', () => {
    expect(shouldAcceptThemeSnapshot(3, 'active', 2, 'stale')).toBe(false);
  });

  it('rejects a conflicting package id at the same revision', () => {
    expect(shouldAcceptThemeSnapshot(2, 'active', 2, 'stale')).toBe(false);
    expect(shouldAcceptThemeSnapshot(2, null, 2, 'stale')).toBe(false);
  });
});
