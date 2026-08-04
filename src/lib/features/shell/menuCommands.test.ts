import { describe, expect, it, vi } from 'vitest';
import { dispatchMenuCommand, type MenuCommandDeps } from './menuCommands';

describe('listening history menu command', () => {
  it('requests the shared confirmation without clearing immediately', async () => {
    const requestClearListeningHistory = vi.fn();
    const deps = {
      runtime: { requestClearListeningHistory },
    } as unknown as MenuCommandDeps;

    await dispatchMenuCommand('app.file.clear_listening_history', deps);

    expect(requestClearListeningHistory).toHaveBeenCalledOnce();
  });
});
