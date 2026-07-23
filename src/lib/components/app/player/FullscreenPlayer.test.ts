import { describe, expect, it } from 'vitest';

describe('FullscreenPlayer close controls', () => {
  it('keeps close clicks outside the Tauri drag path', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const player = readFileSync(
      'src/lib/components/app/player/FullscreenPlayer.svelte',
      'utf8'
    );
    const endfield = readFileSync('src/endfield.css', 'utf8');

    expect(player).toContain('function handleClose(event: MouseEvent)');
    expect(player).toContain('event.preventDefault()');
    expect(player).toContain('event.stopPropagation()');
    expect(player).toContain('onclick={handleClose}');
    expect(player).toContain(
      'onpointerdown={(event) => event.stopPropagation()}'
    );
    expect(endfield).toMatch(
      /\.fullscreen-drag-region\s*\{[^}]*right:\s*84px/s
    );
    expect(endfield).toMatch(
      /\.fullscreen-close\s*\{[^}]*pointer-events:\s*auto/s
    );
  });

  it('has only the document-level Escape handler', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const player = readFileSync(
      'src/lib/components/app/player/FullscreenPlayer.svelte',
      'utf8'
    );

    expect(player).toContain("if (event.key === 'Escape')");
    expect(player).toContain(
      "document.addEventListener('keydown', handleKeyDown)"
    );
    expect(player).not.toContain('onkeydown=');
  });
});
