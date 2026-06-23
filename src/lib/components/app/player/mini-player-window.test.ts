import { describe, expect, it } from 'vitest';

describe('mini player window entrypoint', () => {
  it('routes the secondary window without booting the full app runtime', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const entry = readFileSync('src/main.ts', 'utf8');
    const miniPlayer = readFileSync(
      'src/lib/components/app/player/MiniPlayerWindow.svelte',
      'utf8'
    );

    expect(entry).toContain('MiniPlayerWindow');
    expect(entry).toContain("get('window') === 'mini-player'");
    expect(miniPlayer).toMatch(
      /listen<PlayerState>\(\s*'player-state-changed'/
    );
    expect(miniPlayer).toMatch(/listen<PlayerState>\(\s*'player-progress'/);
    expect(miniPlayer).toContain('showMainWindow');
    expect(miniPlayer).not.toContain('createAppRuntime');
  });
});
