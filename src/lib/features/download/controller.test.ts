import { describe, expect, it, vi } from 'vitest';
import type {
  DownloadJobSnapshot,
  DownloadManagerSnapshot,
  DownloadTaskSnapshot,
} from '$lib/types';
import { createDownloadController } from './controller.svelte';

function makeTask(
  overrides: Partial<DownloadTaskSnapshot> = {}
): DownloadTaskSnapshot {
  return {
    id: 'task-1',
    jobId: 'job-1',
    songCid: 'song-1',
    songName: 'Song 1',
    artists: ['Artist'],
    albumCid: 'album-1',
    albumName: 'Album 1',
    status: 'queued',
    bytesDone: 0,
    bytesTotal: null,
    outputPath: null,
    error: null,
    attempt: 0,
    songIndex: 0,
    songCount: 1,
    ...overrides,
  };
}

function makeJob(
  overrides: Partial<DownloadJobSnapshot> = {}
): DownloadJobSnapshot {
  return {
    id: 'job-1',
    kind: 'song',
    status: 'running',
    createdAt: new Date().toISOString(),
    startedAt: new Date().toISOString(),
    finishedAt: null,
    options: {
      outputDir: '/tmp',
      format: 'flac',
      downloadLyrics: true,
    },
    title: 'Job 1',
    taskCount: 1,
    completedTaskCount: 0,
    failedTaskCount: 0,
    cancelledTaskCount: 0,
    tasks: [makeTask()],
    error: null,
    ...overrides,
  };
}

function makeManager(
  overrides: Partial<DownloadManagerSnapshot> = {}
): DownloadManagerSnapshot {
  return {
    jobs: [],
    activeJobId: null,
    queuedJobIds: [],
    ...overrides,
  };
}

function makeDeps(
  overrides: Partial<Parameters<typeof createDownloadController>[0]> = {}
) {
  return {
    createDownloadJob: vi.fn(),
    cancelDownloadJob: vi.fn(),
    cancelDownloadTask: vi.fn(),
    retryDownloadJob: vi.fn(),
    retryDownloadTask: vi.fn(),
    clearDownloadHistory: vi.fn(),
    openDownloadPanel: vi.fn().mockResolvedValue(undefined),
    getDownloadOptions: vi.fn().mockReturnValue({
      outputDir: '/tmp',
      format: 'flac',
      downloadLyrics: true,
    }),
    notifyInfo: vi.fn(),
    notifyError: vi.fn(),
    ...overrides,
  };
}

describe('createDownloadController', () => {
  describe('applyManagerSnapshot hydration sequence', () => {
    it('applies snapshot when hydrated from command before any event', () => {
      const ctrl = createDownloadController(makeDeps());
      const seq = ctrl.beginHydrationAttempt();
      const manager = makeManager({ jobs: [makeJob()] });

      ctrl.applyManagerSnapshot(manager, seq);

      expect(ctrl.manager?.jobs).toHaveLength(1);
    });

    it('rejects snapshot from a stale hydration sequence', () => {
      const ctrl = createDownloadController(makeDeps());
      ctrl.beginHydrationAttempt();
      const staleSeq = 0;
      const manager = makeManager({ jobs: [makeJob()] });

      ctrl.applyManagerSnapshot(manager, staleSeq);

      expect(ctrl.manager).toBeNull();
    });

    it('rejects snapshot when event hydration has already happened', () => {
      const ctrl = createDownloadController(makeDeps());
      const seq = ctrl.beginHydrationAttempt();

      ctrl.applyManagerEvent(makeManager({ activeJobId: 'event-job' }));
      ctrl.applyManagerSnapshot(
        makeManager({ activeJobId: 'snapshot-job' }),
        seq
      );

      expect(ctrl.manager?.activeJobId).toBe('event-job');
    });
  });

  describe('isSelectionDownloadActionDisabled', () => {
    it('returns true for an empty selection', () => {
      const ctrl = createDownloadController(makeDeps());
      expect(ctrl.isSelectionDownloadActionDisabled([])).toBe(true);
    });

    it('returns false when no active job matches the selection', () => {
      const ctrl = createDownloadController(makeDeps());
      ctrl.applyManagerEvent(makeManager());
      expect(ctrl.isSelectionDownloadActionDisabled(['song-1'])).toBe(false);
    });

    it('returns true when an active job already covers the selection', () => {
      const ctrl = createDownloadController(makeDeps());
      const job = makeJob({
        id: 'sel-job',
        kind: 'selection',
        status: 'running',
        tasks: [
          makeTask({ songCid: 'song-a' }),
          makeTask({ songCid: 'song-b', id: 'task-2', jobId: 'sel-job' }),
        ],
      });
      ctrl.applyManagerEvent(
        makeManager({ jobs: [job], activeJobId: 'sel-job' })
      );

      expect(ctrl.isSelectionDownloadActionDisabled(['song-a', 'song-b'])).toBe(
        true
      );
    });
  });

  describe('filteredJobs', () => {
    it('returns all jobs when search query is empty', () => {
      const ctrl = createDownloadController(makeDeps());
      const jobs = [
        makeJob({ id: 'j1' }),
        makeJob({ id: 'j2', title: 'Another Job' }),
      ];
      ctrl.applyManagerEvent(makeManager({ jobs }));

      expect(ctrl.filteredJobs).toHaveLength(2);
    });

    it('filters jobs by title match', () => {
      const ctrl = createDownloadController(makeDeps());
      const jobs = [
        makeJob({ id: 'j1', title: 'Morning Rain' }),
        makeJob({ id: 'j2', title: 'Evening Glow' }),
      ];
      ctrl.applyManagerEvent(makeManager({ jobs }));
      ctrl.searchQuery = 'morning';

      expect(ctrl.filteredJobs).toHaveLength(1);
      expect(ctrl.filteredJobs[0].id).toBe('j1');
    });
  });

  describe('task operation guards', () => {
    it('canCancelTask returns true for queued task', () => {
      const ctrl = createDownloadController(makeDeps());
      expect(ctrl.canCancelTask(makeTask({ status: 'queued' }))).toBe(true);
    });

    it('canCancelTask returns false for completed task', () => {
      const ctrl = createDownloadController(makeDeps());
      expect(ctrl.canCancelTask(makeTask({ status: 'completed' }))).toBe(false);
    });

    it('canRetryTask returns true for failed task', () => {
      const ctrl = createDownloadController(makeDeps());
      expect(ctrl.canRetryTask(makeTask({ status: 'failed' }))).toBe(true);
    });

    it('canRetryTask returns false for running task', () => {
      const ctrl = createDownloadController(makeDeps());
      expect(ctrl.canRetryTask(makeTask({ status: 'downloading' }))).toBe(
        false
      );
    });
  });
});
