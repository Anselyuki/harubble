import { describe, expect, test } from 'vitest';
import { CapsuleState, transition } from './volume-capsule-state';

describe('CapsuleState transition', () => {
  test('closed → expanding on OPEN', () => {
    expect(transition(CapsuleState.Closed, 'OPEN')).toBe(
      CapsuleState.Expanding
    );
  });

  test('expanding → open on EXPANDED', () => {
    expect(transition(CapsuleState.Expanding, 'EXPANDED')).toBe(
      CapsuleState.Open
    );
  });

  test('open → collapsing on CLOSE', () => {
    expect(transition(CapsuleState.Open, 'CLOSE')).toBe(
      CapsuleState.Collapsing
    );
  });

  test('collapsing → closed on COLLAPSED', () => {
    expect(transition(CapsuleState.Collapsing, 'COLLAPSED')).toBe(
      CapsuleState.Closed
    );
  });

  test('expanding → collapsing when the target closes before expansion finishes', () => {
    expect(transition(CapsuleState.Expanding, 'CLOSE')).toBe(
      CapsuleState.Collapsing
    );
  });

  test('collapsing → expanding when the target reopens before collapse finishes', () => {
    expect(transition(CapsuleState.Collapsing, 'OPEN')).toBe(
      CapsuleState.Expanding
    );
  });

  test('ignores OPEN when already expanding', () => {
    expect(transition(CapsuleState.Expanding, 'OPEN')).toBe(
      CapsuleState.Expanding
    );
  });

  test('ignores CLOSE when already collapsing', () => {
    expect(transition(CapsuleState.Collapsing, 'CLOSE')).toBe(
      CapsuleState.Collapsing
    );
  });

  test('ignores CLOSE when closed', () => {
    expect(transition(CapsuleState.Closed, 'CLOSE')).toBe(CapsuleState.Closed);
  });

  test('ignores OPEN when open', () => {
    expect(transition(CapsuleState.Open, 'OPEN')).toBe(CapsuleState.Open);
  });
});
