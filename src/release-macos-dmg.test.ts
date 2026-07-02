import { describe, expect, it } from 'vitest';

describe('macOS DMG postprocess assets', () => {
  it('documents the quarantine removal command users can copy', async () => {
    // @ts-expect-error Vitest runs in Node and reads release support files.
    const { readFileSync } = await import('node:fs');
    const readme = readFileSync('scripts/macos-dmg-readme.txt', 'utf8');

    expect(readme).toContain(
      'xattr -dr com.apple.quarantine /Applications/Harubble.app'
    );
    expect(readme).toContain('not code signed or notarized');
    expect(readme).toContain('Harubble macOS 安装说明');
    expect(readme).toContain('Harubble macOS Installation Guide');
    expect(readme).toContain('请先阅读本文件');
    expect(readme).toContain('Please read this file first');
  });

  it('guards the guided DMG rewrite without mutating the app bundle', async () => {
    // @ts-expect-error Vitest runs in Node and reads release support files.
    const { readFileSync } = await import('node:fs');
    const script = readFileSync('scripts/postprocess-macos-dmg.sh', 'utf8');

    expect(script).toContain('hdiutil attach');
    expect(script).toContain('Expected exactly one .app bundle');
    expect(script).toContain('macos-dmg-background.swift');
    expect(script).toContain('ln -s /Applications');
    expect(script).toContain('README-macOS.txt');
    expect(script).toContain('background picture');
    expect(script).toContain('osascript');
    expect(script).toContain('hdiutil convert');
    expect(script).not.toContain('codesign ');
    expect(script).not.toContain('xattr -cr');
  });

  it('provides a bilingual DMG background prompt', async () => {
    // @ts-expect-error Vitest runs in Node and reads release support files.
    const { existsSync, readFileSync } = await import('node:fs');
    const backgroundPath = 'scripts/macos-dmg-background.swift';
    const backgroundSource = existsSync(backgroundPath)
      ? readFileSync(backgroundPath, 'utf8')
      : '';

    expect(backgroundSource).toContain('请先阅读 README-macOS.txt');
    expect(backgroundSource).toContain('Please read README-macOS.txt first');
    expect(backgroundSource).toContain('拖到 Applications');
    expect(backgroundSource).toContain('Drag to Applications');
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

  it('allows rerunning a partial release when the tag already exists', async () => {
    // @ts-expect-error Vitest runs in Node and reads release workflow files.
    const { readFileSync } = await import('node:fs');
    const workflow = readFileSync('.github/workflows/distribute.yml', 'utf8');

    expect(workflow).toContain(
      'Release $release_tag already exists; continuing'
    );
    expect(workflow).toContain('gh release upload');
    expect(workflow).toContain('--clobber');
    expect(workflow).not.toContain('Release $release_tag already exists." >&2');
  });
});
