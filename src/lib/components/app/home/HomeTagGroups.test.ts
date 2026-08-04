// @vitest-environment jsdom

import { cleanup, fireEvent, render } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import HomeTagGroups from './HomeTagGroups.svelte';

afterEach(cleanup);

describe('HomeTagGroups filter semantics', () => {
  it('uses a pressed button group instead of incomplete tab semantics', async () => {
    const onSelectDimension = vi.fn();
    const view = render(HomeTagGroups, {
      props: {
        dimensions: [
          { key: 'genre', label: '流派' },
          { key: 'mood', label: '情绪' },
        ],
        groups: [],
        selectedDimensionKey: 'genre',
        onSelectDimension,
        onSelectAlbum: vi.fn(),
      },
    });

    expect(view.queryByRole('tablist')).not.toBeInTheDocument();
    expect(view.getByRole('button', { name: '流派' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await fireEvent.click(view.getByRole('button', { name: '情绪' }));
    expect(onSelectDimension).toHaveBeenCalledWith('mood');
  });
});
