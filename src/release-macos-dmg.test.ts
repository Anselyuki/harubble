import { describe, expect, it } from 'vitest';

describe('macOS DMG postprocess assets', () => {
  it('documents the quarantine removal command users can copy', async () => {
    // @ts-expect-error Vitest runs in Node and reads release support files.
    const { readFileSync } = await import('node:fs');
    const readme = readFileSync('scripts/macos-dmg-readme.txt', 'utf8');

    expect(readme).toContain(
      'xattr -dr com.apple.quarantine /Applications/Harubble.app'
    );
    expect(readme).toContain('Harubble for macOS is not notarized yet');
  });

  it('guards the DMG rewrite and ad-hoc signs exactly one app bundle', async () => {
    // @ts-expect-error Vitest runs in Node and reads release support files.
    const { readFileSync } = await import('node:fs');
    const script = readFileSync('scripts/postprocess-macos-dmg.sh', 'utf8');

    expect(script).toContain('hdiutil attach');
    expect(script).toContain('codesign --force --deep --sign -');
    expect(script).toContain('Expected exactly one .app bundle');
    expect(script).toContain('xattr -cr');
    expect(script).toContain('hdiutil convert');
  });

  it('runs only for macOS release assets before collection', async () => {
    // @ts-expect-error Vitest runs in Node and reads release workflow files.
    const { readFileSync } = await import('node:fs');
    const workflow = readFileSync('.github/workflows/distribute.yml', 'utf8');

    expect(workflow).toContain('Postprocess unsigned macOS DMG');
    expect(workflow).toContain("if: runner.os == 'macOS'");
    expect(workflow.indexOf('Postprocess unsigned macOS DMG')).toBeLessThan(
      workflow.indexOf('Collect release asset')
    );
  });
});
