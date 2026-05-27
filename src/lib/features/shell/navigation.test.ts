import { describe, it, expect, beforeEach } from 'vitest';
import {
  navigationStack,
  isSameEntry,
  type NavigationEntry,
} from './navigation.svelte';

describe('isSameEntry', () => {
  it('returns true for identical home entries', () => {
    const a: NavigationEntry = { view: 'home' };
    const b: NavigationEntry = { view: 'home' };
    expect(isSameEntry(a, b)).toBe(true);
  });

  it('returns false for different views', () => {
    const a: NavigationEntry = { view: 'home' };
    const b: NavigationEntry = { view: 'search' };
    expect(isSameEntry(a, b)).toBe(false);
  });

  it('returns true for same library entries with same albumCid', () => {
    const a: NavigationEntry = { view: 'library', albumCid: 'abc' };
    const b: NavigationEntry = { view: 'library', albumCid: 'abc' };
    expect(isSameEntry(a, b)).toBe(true);
  });

  it('returns false for library entries with different albumCid', () => {
    const a: NavigationEntry = { view: 'library', albumCid: 'abc' };
    const b: NavigationEntry = { view: 'library', albumCid: 'def' };
    expect(isSameEntry(a, b)).toBe(false);
  });

  it('returns true for same collection entries', () => {
    const a: NavigationEntry = { view: 'collection', collectionId: 'c1' };
    const b: NavigationEntry = { view: 'collection', collectionId: 'c1' };
    expect(isSameEntry(a, b)).toBe(true);
  });

  it('returns false for collection entries with different id', () => {
    const a: NavigationEntry = { view: 'collection', collectionId: 'c1' };
    const b: NavigationEntry = { view: 'collection', collectionId: 'c2' };
    expect(isSameEntry(a, b)).toBe(false);
  });

  it('returns true for same tagEditor entries', () => {
    const a: NavigationEntry = {
      view: 'tagEditor',
      albumCid: 'x',
      songCid: null,
    };
    const b: NavigationEntry = {
      view: 'tagEditor',
      albumCid: 'x',
      songCid: null,
    };
    expect(isSameEntry(a, b)).toBe(true);
  });

  it('returns false for tagEditor entries with different songCid', () => {
    const a: NavigationEntry = {
      view: 'tagEditor',
      albumCid: 'x',
      songCid: 's1',
    };
    const b: NavigationEntry = {
      view: 'tagEditor',
      albumCid: 'x',
      songCid: null,
    };
    expect(isSameEntry(a, b)).toBe(false);
  });
});

describe('navigationStack', () => {
  beforeEach(() => {
    navigationStack.clear();
  });

  it('starts empty with canGoBack false', () => {
    expect(navigationStack.canGoBack).toBe(false);
    expect(navigationStack.size).toBe(0);
  });

  it('push adds entry and enables canGoBack', () => {
    navigationStack.push({ view: 'home' });
    expect(navigationStack.canGoBack).toBe(true);
    expect(navigationStack.size).toBe(1);
  });

  it('pop returns last pushed entry', () => {
    navigationStack.push({ view: 'home' });
    navigationStack.push({ view: 'search' });
    const entry = navigationStack.pop();
    expect(entry).toEqual({ view: 'search' });
    expect(navigationStack.size).toBe(1);
  });

  it('pop returns undefined when empty', () => {
    expect(navigationStack.pop()).toBeUndefined();
  });

  it('peek returns top without removing', () => {
    navigationStack.push({ view: 'library', albumCid: 'abc' });
    expect(navigationStack.peek()).toEqual({
      view: 'library',
      albumCid: 'abc',
    });
    expect(navigationStack.size).toBe(1);
  });

  it('clear empties the stack', () => {
    navigationStack.push({ view: 'home' });
    navigationStack.push({ view: 'search' });
    navigationStack.clear();
    expect(navigationStack.canGoBack).toBe(false);
    expect(navigationStack.size).toBe(0);
  });

  it('deduplicates consecutive identical entries on push', () => {
    navigationStack.push({ view: 'home' });
    navigationStack.push({ view: 'home' });
    expect(navigationStack.size).toBe(1);
  });

  it('does not deduplicate different entries', () => {
    navigationStack.push({ view: 'home' });
    navigationStack.push({ view: 'library', albumCid: 'a' });
    navigationStack.push({ view: 'library', albumCid: 'b' });
    expect(navigationStack.size).toBe(3);
  });
});
