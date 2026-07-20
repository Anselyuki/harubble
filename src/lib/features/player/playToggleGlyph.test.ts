import { describe, expect, it } from 'vitest';
import { selectGlyphAfterCollapse } from './playToggleGlyph';

describe('play toggle glyph selection', () => {
  it('shows play directly when pause settles during collapse', () => {
    expect(
      selectGlyphAfterCollapse('pause', {
        isPlaying: false,
        isLoading: false,
        isPending: false,
      })
    ).toBe('play');
  });

  it('shows pause directly when resume settles before pending cleanup', () => {
    expect(
      selectGlyphAfterCollapse('play', {
        isPlaying: true,
        isLoading: false,
        isPending: true,
      })
    ).toBe('pause');
  });

  it('shows loading when playback has not switched at the midpoint', () => {
    expect(
      selectGlyphAfterCollapse('pause', {
        isPlaying: true,
        isLoading: false,
        isPending: true,
      })
    ).toBe('loading');
  });

  it('restores the settled glyph when a command fails before the midpoint', () => {
    expect(
      selectGlyphAfterCollapse('play', {
        isPlaying: false,
        isLoading: false,
        isPending: false,
      })
    ).toBe('play');
  });

  it('collapses loading straight to the settled glyph once state clears', () => {
    expect(
      selectGlyphAfterCollapse('loading', {
        isPlaying: true,
        isLoading: false,
        isPending: false,
      })
    ).toBe('pause');
  });

  it('keeps loading visible while backend still reports loading', () => {
    expect(
      selectGlyphAfterCollapse('loading', {
        isPlaying: true,
        isLoading: true,
        isPending: false,
      })
    ).toBe('loading');
  });

  it('keeps loading visible while a pending cleanup is still in flight', () => {
    expect(
      selectGlyphAfterCollapse('loading', {
        isPlaying: true,
        isLoading: false,
        isPending: true,
      })
    ).toBe('loading');
  });

  it('promotes to loading when only isLoading is set from a non-loading outgoing', () => {
    expect(
      selectGlyphAfterCollapse('play', {
        isPlaying: false,
        isLoading: true,
        isPending: false,
      })
    ).toBe('loading');
  });
});
