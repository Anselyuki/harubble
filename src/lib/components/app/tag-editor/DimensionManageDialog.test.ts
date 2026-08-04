// @vitest-environment jsdom

import { cleanup, fireEvent, render, waitFor } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import DimensionManageDialog from './DimensionManageDialog.svelte';

afterEach(cleanup);

describe('DimensionManageDialog deletion', () => {
  it('requires an explicit alert-dialog confirmation and supports cancellation', async () => {
    const onRemoveDimension = vi.fn().mockResolvedValue(undefined);
    const view = render(DimensionManageDialog, {
      props: {
        open: true,
        dimensions: [{ key: 'genre', label: { 'zh-CN': '流派' } }],
        onAddDimension: vi.fn(),
        onRemoveDimension,
        onOpenChange: vi.fn(),
      },
    });

    await fireEvent.click(view.getByRole('button', { name: '删除维度 流派' }));
    expect(view.getByRole('alertdialog')).toBeInTheDocument();
    await fireEvent.click(view.getByRole('button', { name: '取消' }));
    expect(onRemoveDimension).not.toHaveBeenCalled();

    await fireEvent.click(view.getByRole('button', { name: '删除维度 流派' }));
    await fireEvent.click(view.getByRole('button', { name: '删除维度' }));
    await waitFor(() => expect(onRemoveDimension).toHaveBeenCalledOnce());
  });
});
