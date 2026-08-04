import { describe, expect, it, vi } from 'vitest';
import type { AppEventMap } from '$lib/appEvents';
import type { AlbumCatalogRefreshedEvent, AppPreferences } from '$lib/types';
import {
  subscribeToTauriEvents,
  type EventSubscriptionDeps,
} from './appRuntimeBootstrap.svelte';

function createSubscriptionDeps() {
  const handlers = new Map<
    keyof AppEventMap,
    (event: { payload: unknown }) => void | Promise<void>
  >();
  const handleRefreshedEvent = vi.fn();
  const unlisten = vi.fn();

  const listen = vi.fn(
    async <T>(
      eventName: string,
      handler: (event: { event: string; id: number; payload: T }) => void
    ) => {
      handlers.set(eventName as keyof AppEventMap, (event) =>
        handler({
          event: eventName,
          id: 0,
          payload: event.payload as T,
        })
      );
      return unlisten;
    }
  ) as unknown as EventSubscriptionDeps['listen'];

  const deps: EventSubscriptionDeps = {
    listen,
    playerController: {
      syncPlayerState: vi.fn(),
      syncPlayerProgress: vi.fn(),
      syncPlaybackEnded: vi.fn(),
    },
    downloadController: {
      applyManagerEvent: vi.fn(),
      applyJobUpdate: vi.fn(),
      applyTaskProgress: vi.fn(),
    },
    libraryController: {
      handleInventoryStateChanged: vi.fn(),
    },
    searchController: {
      handleIndexStateChanged: vi.fn(),
    },
    albumCatalogController: {
      handleRefreshedEvent,
    },
    homeController: {
      handleBelongReady: vi.fn(),
    },
    handleAppErrorEvent: vi.fn(),
    clearSongSelection: vi.fn(),
    setSelectionModeEnabled: vi.fn(),
    invalidateInventoryCaches: vi.fn(),
    setPlayerStateHydratedFromEvent: vi.fn(),
    handleMenuCommand: vi.fn(),
    handlePreferencesSnapshot: vi.fn(),
  };

  return { deps, handlers, handleRefreshedEvent, unlisten };
}

describe('subscribeToTauriEvents', () => {
  it('forwards album catalog refresh events to the shared controller', async () => {
    const { deps, handlers, handleRefreshedEvent } = createSubscriptionDeps();
    const payload: AlbumCatalogRefreshedEvent = {
      revision: 3,
      checkedAt: 1_721_440_000_000,
      changed: true,
      albumCount: 42,
    };

    const cleanup = await subscribeToTauriEvents(deps, () => false);
    const handler = handlers.get('album-catalog-refreshed');

    expect(handler).toBeTypeOf('function');
    await handler?.({ payload });
    expect(handleRefreshedEvent).toHaveBeenCalledOnce();
    expect(handleRefreshedEvent).toHaveBeenCalledWith(payload);

    cleanup();
  });

  it('forwards preference snapshots to the runtime controllers', async () => {
    const { deps, handlers } = createSubscriptionDeps();
    const snapshot = {
      theme: { revision: 4, activePackageId: 'ark-ui-ark' },
    } as AppPreferences;

    const cleanup = await subscribeToTauriEvents(deps, () => false);
    const handler = handlers.get('preferences_snapshot');

    expect(handler).toBeTypeOf('function');
    await handler?.({ payload: snapshot });
    expect(deps.handlePreferencesSnapshot).toHaveBeenCalledWith(snapshot);

    cleanup();
  });
});
