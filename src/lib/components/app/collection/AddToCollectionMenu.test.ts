// @vitest-environment jsdom

import { cleanup, fireEvent, render, waitFor } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AddToCollectionMenu from './AddToCollectionMenu.svelte';

afterEach(cleanup);

describe('AddToCollectionMenu', () => {
  it('exposes a regular action list and focuses its first action', async () => {
    const onAdd = vi.fn();
    const view = render(AddToCollectionMenu, {
      props: {
        collections: [
          {
            id: 'official',
            name: 'Official',
            description: '',
            cover: null,
            songCount: 1,
            isOfficial: true,
            createdAt: 1,
            updatedAt: 1,
          },
          {
            id: 'favorites',
            name: '收藏',
            description: '',
            cover: null,
            songCount: 2,
            isOfficial: false,
            createdAt: 1,
            updatedAt: 1,
          },
        ],
        onAdd,
      },
    });

    await fireEvent.click(view.getByRole('button', { name: '添加到合集' }));
    expect(view.queryByRole('menu')).not.toBeInTheDocument();
    expect(view.getByRole('list', { name: '添加到合集' })).toBeInTheDocument();
    const action = view.getByRole('button', { name: /收藏/ });
    await waitFor(() => expect(action).toHaveFocus());

    await fireEvent.click(action);
    expect(onAdd).toHaveBeenCalledWith('favorites');
  });
});
