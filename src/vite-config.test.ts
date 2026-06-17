import { describe, expect, it } from 'vitest';

describe('Vite build configuration', () => {
  it('disables Rolldown plugin timing warnings for production builds', async () => {
    // @ts-expect-error Vitest runs in Node and reads the source config file.
    const { readFileSync } = await import('node:fs');
    const source = readFileSync('vite.config.ts', 'utf8');

    expect(source).toMatch(
      /build:\s*{[\s\S]*rolldownOptions:\s*{[\s\S]*checks:\s*{[\s\S]*pluginTimings:\s*false/
    );
  });
});
