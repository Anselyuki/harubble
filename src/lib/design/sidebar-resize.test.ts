// @vitest-environment jsdom

import { describe, expect, it, vi } from 'vitest';
import { createSidebarResize } from './sidebar-resize';

describe('sidebar resize keyboard interaction', () => {
  it('expands from collapsed with ArrowRight and supports Home and End', () => {
    const shellEl = document.createElement('div');
    const handleEl = document.createElement('div');
    shellEl.style.setProperty('--sidebar-width', '56px');
    let collapsed = true;
    const onWidthChange = vi.fn((width: number) => {
      shellEl.style.setProperty('--sidebar-width', `${width}px`);
    });
    const onDragEnd = vi.fn((_width: number, shouldCollapse: boolean) => {
      collapsed = shouldCollapse;
    });
    const resize = createSidebarResize({
      shellEl,
      handleEl,
      collapsedWidth: 56,
      expandedWidth: 248,
      threshold: 120,
      getCollapsed: () => collapsed,
      onWidthChange,
      onCrossThreshold: vi.fn(),
      onDragEnd,
    });

    handleEl.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', cancelable: true })
    );
    expect(onDragEnd).toHaveBeenLastCalledWith(120, false);

    handleEl.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'End', cancelable: true })
    );
    expect(onDragEnd).toHaveBeenLastCalledWith(248, false);

    handleEl.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Home', cancelable: true })
    );
    expect(onDragEnd).toHaveBeenLastCalledWith(56, true);
    resize.dispose();
  });
});
