// @vitest-environment jsdom

import { cleanup, fireEvent, render } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SongRow from './SongRow.svelte';

afterEach(cleanup);

const song = {
  cid: 'song-1',
  name: 'Test Song',
  artists: ['Artist'],
  download: {
    isDownloaded: false,
    downloadStatus: 'missing' as const,
    inventoryVersion: 'v1',
  },
  tags: [],
};

describe('SongRow interaction semantics', () => {
  it('keeps the row non-interactive while exposing explicit controls', () => {
    const view = render(SongRow, {
      props: {
        song,
        index: 0,
        albumCid: 'album-1',
        albumName: 'Album',
      },
    });

    const row = view.container.querySelector('[data-song-cid="song-1"]');
    expect(row).not.toHaveAttribute('role', 'button');
    expect(row).not.toHaveAttribute('tabindex');
    expect(
      view.getAllByRole('button', { name: /Test Song/ }).length
    ).toBeGreaterThan(1);
  });

  it('preserves double-click playback without stealing child button clicks', async () => {
    const onPlay = vi.fn();
    const onDownload = vi.fn();
    const view = render(SongRow, {
      props: {
        song,
        index: 0,
        albumCid: 'album-1',
        albumName: 'Album',
        onclick: onPlay,
        onDownload,
      },
    });
    const row = view.container.querySelector('[data-song-cid="song-1"]')!;

    await fireEvent.dblClick(row);
    expect(onPlay).toHaveBeenCalledOnce();

    const downloadButton = view.getByTitle(/下载|Download/);
    await fireEvent.click(downloadButton);
    expect(onDownload).toHaveBeenCalledOnce();
    expect(onPlay).toHaveBeenCalledOnce();
  });
});
