import { describe, expect, it } from 'vitest';

describe('LyricsBubble seek contract', () => {
  it('makes timestamped lyrics seekable through the player seek callback', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const bubble = readFileSync(
      'src/lib/components/app/player/LyricsBubble.svelte',
      'utf8'
    );
    const player = readFileSync(
      'src/lib/components/AudioPlayer.svelte',
      'utf8'
    );

    expect(bubble).toContain('line.time !== null && canSeek');
    expect(bubble).toContain('class="lyrics-bubble-line seekable"');
    expect(bubble).toContain('onclick={() => onSeek(line.time!)}');
    expect(bubble).toContain('createLyricsAutoFollowController');
    expect(player).toContain('{isPlaying}');
    expect(player).toContain('{canSeek}');
    expect(player).toContain('onSeek={commitSeek}');
  });
});
