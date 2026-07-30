import { describe, expect, it } from 'vitest';

describe('window chrome interaction contract', () => {
  it('keeps native drag regions inside the app shell', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const app = readFileSync('src/App.svelte', 'utf8');
    const sidebar = readFileSync(
      'src/lib/components/app/sidebar/AppSidebar.svelte',
      'utf8'
    );
    const css = readFileSync('src/app.css', 'utf8');

    expect(app).not.toContain('macos-window-drag-region');
    expect(app).toMatch(
      /class="main-drag-region"[\s\S]*data-tauri-drag-region/
    );
    expect(sidebar).toMatch(
      /class="sidebar-drag-region"[\s\S]*data-tauri-drag-region/
    );

    const mainDrag = [...css.matchAll(/\.main-drag-region\s*\{([^}]*)\}/g)]
      .map((match) => match[1])
      .find((block) => block.includes('height:'));
    const sidebarDrag = [
      ...css.matchAll(/\.sidebar-drag-region\s*\{([^}]*)\}/g),
    ]
      .map((match) => match[1])
      .find((block) => block.includes('height:'));
    expect(mainDrag).toBeDefined();
    expect(mainDrag).toMatch(/height:\s*var\(--window-drag-strip-height\)/);
    expect(mainDrag).toMatch(/z-index:\s*var\(--z-window-drag\)/);
    expect(mainDrag).not.toMatch(/pointer-events:\s*none/);
    expect(sidebarDrag).toBeDefined();
    expect(sidebarDrag).toMatch(/z-index:\s*var\(--z-window-drag\)/);
    expect(sidebarDrag).not.toMatch(/pointer-events:\s*none/);
  });

  it('reserves the fullscreen action zone for the close button', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const css = readFileSync('src/app.css', 'utf8');
    const fullscreen = readFileSync(
      'src/lib/components/app/player/FullscreenPlayer.svelte',
      'utf8'
    );

    const dragRegion = css.match(/\.fullscreen-drag-region\s*\{([^}]*)\}/)?.[1];
    expect(dragRegion).toBeDefined();
    expect(dragRegion).toMatch(/right:\s*var\(--fullscreen-action-reserve\)/);
    expect(fullscreen).toContain('function handleClose(event: MouseEvent)');
    expect(fullscreen).toContain('event.preventDefault()');
    expect(fullscreen).toContain('event.stopPropagation()');
    expect(fullscreen).not.toContain(
      "onkeydown={(e) => e.key === 'Escape' && player.toggleFullscreen()}"
    );
  });

  it('uses deep drag markers only for non-interactive mini-player content', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const miniPlayer = readFileSync(
      'src/lib/components/app/player/MiniPlayerWindow.svelte',
      'utf8'
    );

    expect(miniPlayer).toContain('class="cover-shell"');
    expect(miniPlayer).toContain('data-tauri-drag-region="deep"');
    expect(miniPlayer).toContain(
      'class="track-meta" data-tauri-drag-region="deep"'
    );
    expect(miniPlayer).not.toContain('data-tauri-drag-region>');
  });

  it('keeps transient floating layers above persistent controls', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const css = readFileSync('src/app.css', 'utf8');
    const addToCollection = readFileSync(
      'src/lib/components/app/collection/AddToCollectionMenu.svelte',
      'utf8'
    );
    const tagDialog = readFileSync(
      'src/lib/components/app/tag-editor/TagAddDialog.svelte',
      'utf8'
    );

    const layerNames = [
      'window-drag',
      'window-actions',
      'player-dock',
      'sheet-overlay',
      'sheet',
      'fullscreen',
      'dialog-overlay',
      'dialog',
      'popover',
      'tooltip',
    ];
    const layers = layerNames.map((name) => {
      const value = css.match(new RegExp(`--z-${name}:\\s*(\\d+);`))?.[1];
      return Number(value);
    });
    expect(layers.every(Number.isFinite)).toBe(true);
    expect(layers).toEqual([...layers].sort((a, b) => a - b));
    expect(addToCollection).toContain('<Popover.Portal>');
    expect(addToCollection).toContain('z-index: var(--z-popover)');
    expect(tagDialog).toContain('z-index: var(--z-popover)');
  });

  it('makes retained exit layers release pointer input immediately', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const tooltip = readFileSync(
      'src/lib/components/ui/tooltip/tooltip-content.svelte',
      'utf8'
    );
    const retainedLayers = [
      'src/lib/components/ui/select/select-content.svelte',
      'src/lib/components/ui/sheet/sheet-overlay.svelte',
      'src/lib/components/ui/sheet/sheet-content.svelte',
      'src/lib/components/ui/dialog/dialog-overlay.svelte',
      'src/lib/components/ui/dialog/dialog-content.svelte',
      'src/lib/components/ui/alert-dialog/alert-dialog-overlay.svelte',
      'src/lib/components/ui/alert-dialog/alert-dialog-content.svelte',
    ].map((path) => readFileSync(path, 'utf8'));

    expect(tooltip).toContain('pointer-events-none inline-flex');
    for (const source of retainedLayers) {
      expect(source).toContain("!open && '!pointer-events-none'");
    }
  });

  it('keeps primary floating icon targets at least 40px', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const toolbar = readFileSync(
      'src/lib/components/app/shell/TopToolbar.svelte',
      'utf8'
    );
    const css = readFileSync('src/app.css', 'utf8');
    const miniPlayer = readFileSync(
      'src/lib/components/app/player/MiniPlayerWindow.svelte',
      'utf8'
    );
    const metadata = readFileSync(
      'src/lib/components/MetadataPopover.svelte',
      'utf8'
    );
    const tagSearch = readFileSync(
      'src/lib/components/app/tag-editor/TagSearchTab.svelte',
      'utf8'
    );
    const collection = readFileSync(
      'src/lib/components/app/collection/CollectionDetailPanel.svelte',
      'utf8'
    );
    const sidebar = readFileSync(
      'src/lib/components/app/sidebar/AppSidebar.svelte',
      'utf8'
    );
    const songRow = readFileSync('src/lib/components/SongRow.svelte', 'utf8');

    expect(toolbar.match(/size-10/g)).toHaveLength(3);
    expect(css).toMatch(
      /\.album-workspace-content > \.back-button\s*\{[\s\S]*?width:\s*40px;[\s\S]*?height:\s*40px;/
    );
    expect(miniPlayer).toMatch(
      /\.icon-button,[\s\S]*?\.play-button\s*\{[\s\S]*?width:\s*40px;[\s\S]*?height:\s*40px;/
    );
    expect(metadata).toMatch(
      /:global\(\.meta-trigger\)\s*\{[\s\S]*?width:\s*40px;[\s\S]*?height:\s*40px;/
    );
    expect(tagSearch).toMatch(
      /\.edit-btn-inset\s*\{[\s\S]*?width:\s*40px;[\s\S]*?height:\s*40px;/
    );
    expect(collection).toMatch(
      /\.remove-btn\s*\{[\s\S]*?width:\s*40px;[\s\S]*?height:\s*40px;/
    );
    expect(sidebar).toMatch(
      /\.section-action-btn\s*\{[\s\S]*?width:\s*40px;[\s\S]*?height:\s*40px;/
    );
    expect(
      songRow.match(/width:\s*40px;\s*\n\s*height:\s*40px;/g)
    ).toHaveLength(2);
  });

  it('does not steal focus from an outside pointer target when a tag bubble closes', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const tagDialog = readFileSync(
      'src/lib/components/app/tag-editor/TagAddDialog.svelte',
      'utf8'
    );

    expect(tagDialog).toContain('closeCard(false)');
    expect(tagDialog).toContain('aria-haspopup="dialog"');
    expect(tagDialog).not.toContain('aria-modal="true"');
  });
});
