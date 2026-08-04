// @vitest-environment jsdom

import { cleanup, fireEvent, render } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import PlayerTimeline from './PlayerTimeline.svelte';

afterEach(cleanup);

describe('PlayerTimeline', () => {
  it('exposes the seek contract and forwards preview and commit events', async () => {
    const onInput = vi.fn();
    const onChange = vi.fn();
    const view = render(PlayerTimeline, {
      props: {
        value: 12,
        max: 120,
        disabled: false,
        groupLabel: 'Timeline',
        seekLabel: 'Seek',
        onInput,
        onChange,
      },
    });
    const slider = view.getByRole('slider', { name: 'Seek' });

    expect(view.getByRole('group', { name: 'Timeline' })).toContainElement(
      slider
    );
    expect(slider).toHaveValue('12');
    await fireEvent.input(slider, { target: { value: '30' } });
    await fireEvent.change(slider, { target: { value: '30' } });
    expect(onInput).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledOnce();
  });

  it('disables seeking when the parent contract does not allow it', () => {
    const view = render(PlayerTimeline, {
      props: {
        value: 0,
        max: 1,
        disabled: true,
        groupLabel: 'Timeline',
        seekLabel: 'Seek',
        onInput: vi.fn(),
        onChange: vi.fn(),
      },
    });
    expect(view.getByRole('slider', { name: 'Seek' })).toBeDisabled();
  });
});
