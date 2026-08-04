// @vitest-environment jsdom

import { cleanup, fireEvent, render } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SearchBar from './SearchBar.svelte';

afterEach(cleanup);

describe('SearchBar scope control', () => {
  it('updates the group name with scope and keeps 40px targets', async () => {
    const onScopeChange = vi.fn();
    const view = render(SearchBar, {
      props: {
        query: '',
        scope: 'all',
        onQueryChange: vi.fn(),
        onScopeChange,
        onSubmit: vi.fn(),
      },
    });

    expect(view.getByRole('group')).toHaveAccessibleName('搜索范围：全部');
    view.getByRole('button', { name: '专辑' }).click();
    expect(onScopeChange).toHaveBeenCalledWith('albums');

    await view.rerender({
      query: '',
      scope: 'albums',
      onQueryChange: vi.fn(),
      onScopeChange,
      onSubmit: vi.fn(),
    });
    expect(view.getByRole('group')).toHaveAccessibleName('搜索范围：专辑');
  });

  it('uses a search form with the query field first in tab order', async () => {
    const onSubmit = vi.fn();
    const view = render(SearchBar, {
      props: {
        query: '',
        scope: 'all',
        onQueryChange: vi.fn(),
        onScopeChange: vi.fn(),
        onSubmit,
      },
    });

    const form = view.getByRole('search');
    const focusable = form.querySelectorAll('input, button');
    expect(focusable[0]).toBe(view.getByRole('textbox'));

    await fireEvent.submit(form);
    expect(onSubmit).toHaveBeenCalledOnce();
  });
});
