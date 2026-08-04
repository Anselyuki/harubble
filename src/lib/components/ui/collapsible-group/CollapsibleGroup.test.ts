// @vitest-environment jsdom

import { cleanup, render } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import CollapsibleGroupFixture from '../../../../test-fixtures/CollapsibleGroupFixture.svelte';

afterEach(cleanup);

describe('CollapsibleGroup actions', () => {
  it('keeps an empty group toggle disabled without disabling its actions', () => {
    const onCreate = vi.fn();
    const view = render(CollapsibleGroupFixture, {
      props: { empty: true, onCreate },
    });

    const toggle = view.getByRole('button', { name: 'Custom collections' });
    const create = view.getByRole('button', { name: 'Create collection' });

    expect(toggle).toBeDisabled();
    expect(create).toBeEnabled();
    expect(create.closest('[aria-disabled="true"]')).toBeNull();

    create.click();
    expect(onCreate).toHaveBeenCalledOnce();
  });
});
