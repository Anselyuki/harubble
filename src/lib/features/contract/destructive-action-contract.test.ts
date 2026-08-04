import { describe, expect, it } from 'vitest';

async function readSource(path: string): Promise<string> {
  // @ts-expect-error Vitest runs in Node and reads source files.
  const { readFileSync } = await import('node:fs');
  return readFileSync(path, 'utf8');
}

describe('destructive action confirmation contract', () => {
  it('does not use the WebView confirm API for collection deletion', async () => {
    const source = await readSource(
      'src/lib/components/app/collection/CollectionDetailPanel.svelte'
    );
    expect(source).not.toMatch(/\bconfirm\s*\(/);
    expect(source).toContain('<AlertDialog.Root');
  });

  it('routes both download-history entry points through the shared request', async () => {
    const menu = await readSource('src/lib/features/shell/menuCommands.ts');
    const sheets = await readSource(
      'src/lib/components/app/shell/AppSideSheets.svelte'
    );
    const runtime = await readSource(
      'src/lib/features/shell/appRuntime.svelte.ts'
    );
    expect(menu).toContain('runtime.requestClearDownloadHistory()');
    expect(sheets).toContain(
      'onClearDownloadHistory={onRequestClearDownloadHistory}'
    );
    expect(runtime).toContain('notifyInfo(m.download_notify_history_empty())');
  });
});
