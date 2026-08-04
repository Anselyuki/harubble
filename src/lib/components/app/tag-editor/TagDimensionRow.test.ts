// @vitest-environment jsdom

import { cleanup, fireEvent, render, waitFor } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import TagDimensionRow from './TagDimensionRow.svelte';

afterEach(cleanup);

describe('TagDimensionRow reordering', () => {
  it('offers touch and keyboard alternatives and announces the result', async () => {
    const onSetTag = vi.fn().mockResolvedValue(undefined);
    const view = render(TagDimensionRow, {
      props: {
        dimensionKey: 'genre',
        dimensionLabel: '流派',
        values: [{ 'zh-CN': '摇滚' }, { 'zh-CN': '爵士' }],
        isEditing: false,
        onSetTag,
        onRemoveTag: vi.fn(),
      },
    });

    await fireEvent.click(
      view.getByRole('button', { name: '将标签值「摇滚」下移' })
    );
    expect(onSetTag).toHaveBeenCalledWith('genre', [
      { 'zh-CN': '爵士' },
      { 'zh-CN': '摇滚' },
    ]);
    await waitFor(() =>
      expect(
        view.getByText(/已将标签值「摇滚」移动到第 2 位/)
      ).toBeInTheDocument()
    );

    await fireEvent.keyDown(
      view.getByRole('button', { name: /调整标签值「摇滚」顺序/ }),
      { key: 'ArrowUp' }
    );
    expect(onSetTag).toHaveBeenCalledTimes(1);
  });
});
