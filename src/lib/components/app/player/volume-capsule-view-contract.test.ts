import { describe, expect, it } from 'vitest';

describe('volume capsule family interaction contract', () => {
  async function readSources(): Promise<string[]> {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    return [
      'src/lib/components/app/player/material/MaterialVolumeCapsuleView.svelte',
      'src/lib/components/app/player/glass/GlassVolumeCapsuleView.svelte',
      'src/lib/components/app/player/family/FamilyVolumeCapsuleView.svelte',
    ].map((path) => readFileSync(path, 'utf8'));
  }

  it('uses a single click for mute in every visual family', async () => {
    const sources = await readSources();
    for (const source of sources) {
      expect(source).not.toContain('ondblclick');
      expect(source).toMatch(
        /onclick=\{(?:handleMuteClick|\(\) => onToggleMute\?\.\(\))\}/
      );
    }
  });

  it('uses state-aware accessible names in every visual family', async () => {
    const sources = await readSources();
    for (const source of sources) {
      expect(source).toContain(
        'aria-label={muted ? m.player_aria_unmute() : m.player_aria_mute()}'
      );
    }
  });
});
