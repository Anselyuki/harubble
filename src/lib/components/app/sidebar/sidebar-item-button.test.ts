import { describe, expect, test } from 'vitest';
import { getSidebarItemActivation } from './sidebar-item-button';

describe('getSidebarItemActivation', () => {
  test('expands instead of activating when collapsed expansion is enabled', () => {
    expect(
      getSidebarItemActivation({
        collapsed: true,
        expandOnCollapsedClick: true,
        hasRequestExpand: true,
      })
    ).toBe('expand');
  });

  test('activates directly when collapsed expansion is disabled', () => {
    expect(
      getSidebarItemActivation({
        collapsed: true,
        expandOnCollapsedClick: false,
        hasRequestExpand: true,
      })
    ).toBe('activate');
  });

  test('activates directly while expanded', () => {
    expect(
      getSidebarItemActivation({
        collapsed: false,
        expandOnCollapsedClick: true,
        hasRequestExpand: true,
      })
    ).toBe('activate');
  });
});
