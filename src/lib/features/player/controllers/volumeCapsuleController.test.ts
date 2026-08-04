// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from 'vitest';
import { CapsuleState } from '$lib/components/app/player/volume-capsule-state';
import type { CapsuleAnimator } from '$lib/components/app/player/volume-capsule-animator';
import { createVolumeCapsuleController } from './volumeCapsuleController.svelte';

function makeAnimator(): CapsuleAnimator & {
  events: string[];
  expandCompletions: (() => void)[];
  collapseCompletions: (() => void)[];
} {
  const events: string[] = [];
  const expandCompletions: (() => void)[] = [];
  const collapseCompletions: (() => void)[] = [];

  return {
    events,
    expandCompletions,
    collapseCompletions,
    expand(onComplete) {
      events.push('expand');
      expandCompletions.push(onComplete);
    },
    collapse(onComplete) {
      events.push('collapse');
      collapseCompletions.push(onComplete);
    },
    showBadge() {
      events.push('show-badge');
    },
    hideBadge() {
      events.push('hide-badge');
    },
    destroy() {
      events.push('destroy');
    },
  };
}

function makeController(animator = makeAnimator()) {
  let open = false;
  const onopen = vi.fn(() => {
    open = true;
  });
  const onclose = vi.fn(() => {
    open = false;
  });
  const focusSlider = vi.fn();
  const onVolumeChange = vi.fn();
  const wrapper = document.createElement('div');

  const controller = createVolumeCapsuleController({
    animator,
    getOpen: () => open,
    getWrapperEl: () => wrapper,
    focusSlider,
    onopen,
    onclose,
    onVolumeChange,
  });

  return {
    animator,
    controller,
    focusSlider,
    onclose,
    onopen,
    onVolumeChange,
    wrapper,
    setOpen(value: boolean) {
      open = value;
    },
  };
}

afterEach(() => {
  vi.useRealTimers();
});

describe('volume capsule controller · 可中断开合', () => {
  it('expanding 期间收到 close 会反向收起并忽略旧完成回调', () => {
    const subject = makeController();
    subject.setOpen(true);
    subject.controller.syncOpen();
    expect(subject.controller.state).toBe(CapsuleState.Expanding);

    subject.setOpen(false);
    subject.controller.syncOpen();
    expect(subject.controller.state).toBe(CapsuleState.Collapsing);
    expect(subject.animator.events).toEqual(['expand', 'collapse']);

    subject.animator.expandCompletions[0]?.();
    expect(subject.controller.state).toBe(CapsuleState.Collapsing);
    subject.animator.collapseCompletions[0]?.();
    expect(subject.controller.state).toBe(CapsuleState.Closed);
  });

  it('collapsing 期间收到 open 会反向展开并忽略旧完成回调', () => {
    const subject = makeController();
    subject.setOpen(true);
    subject.controller.syncOpen();
    subject.animator.expandCompletions[0]?.();
    expect(subject.controller.state).toBe(CapsuleState.Open);

    subject.setOpen(false);
    subject.controller.syncOpen();
    subject.setOpen(true);
    subject.controller.syncOpen();
    expect(subject.controller.state).toBe(CapsuleState.Expanding);

    subject.animator.collapseCompletions[0]?.();
    expect(subject.controller.state).toBe(CapsuleState.Expanding);
    subject.animator.expandCompletions[1]?.();
    expect(subject.controller.state).toBe(CapsuleState.Open);
  });

  it('点击打开后只在当前展开完成时把焦点交给 slider', () => {
    const subject = makeController();
    subject.controller.handleIconClick();
    subject.controller.syncOpen();
    expect(subject.focusSlider).not.toHaveBeenCalled();

    subject.animator.expandCompletions[0]?.();
    expect(subject.focusSlider).toHaveBeenCalledTimes(1);
  });

  it('hover 已触发展开时，随后 click 仍等待展开完成再聚焦', () => {
    const subject = makeController();
    subject.controller.handleMouseEnter();
    subject.controller.syncOpen();
    subject.controller.handleIconClick();
    expect(subject.focusSlider).not.toHaveBeenCalled();

    subject.animator.expandCompletions[0]?.();
    expect(subject.focusSlider).toHaveBeenCalledTimes(1);
  });

  it('focus intent reveals the control without stealing focus', () => {
    const subject = makeController();
    subject.controller.handleFocusIn();
    subject.controller.syncOpen();
    subject.animator.expandCompletions[0]?.();

    expect(subject.onopen).toHaveBeenCalledTimes(1);
    expect(subject.focusSlider).not.toHaveBeenCalled();
  });
});

describe('volume capsule controller · 拖动预览', () => {
  it('input 同步预览、更新音量并显示数值，commit 后清除预览', () => {
    const subject = makeController();
    subject.controller.handleSliderInput(0.42);

    expect(subject.controller.sliderPreview).toBe(0.42);
    expect(subject.onVolumeChange).toHaveBeenCalledWith(0.42);
    expect(subject.animator.events).toContain('show-badge');

    subject.controller.handleSliderCommit();
    expect(subject.controller.sliderPreview).toBeNull();
  });

  it('pointerup 会结束拖动并清除实时预览', () => {
    const subject = makeController();
    subject.controller.handleSliderInput(0.42);
    subject.controller.handleSliderDown();

    subject.controller.handleSliderUp();
    expect(subject.controller.isDragging).toBe(false);
    expect(subject.controller.sliderPreview).toBeNull();
  });

  it('全局 pointercancel 会结束拖动并在指针离开后安排收起', () => {
    vi.useFakeTimers();
    const subject = makeController();
    subject.setOpen(true);
    subject.controller.handleSliderInput(0.42);
    subject.controller.handleSliderDown();
    const uninstall = subject.controller.installGlobalPointerListeners();

    document.dispatchEvent(new Event('pointercancel'));
    expect(subject.controller.isDragging).toBe(false);
    expect(subject.controller.sliderPreview).toBeNull();
    vi.advanceTimersByTime(799);
    expect(subject.onclose).toHaveBeenCalledTimes(1);

    uninstall();
  });

  it('窗口失焦会结束拖动并清除未提交预览', () => {
    const subject = makeController();
    subject.controller.handleSliderInput(0.42);
    subject.controller.handleSliderDown();
    const uninstall = subject.controller.installGlobalPointerListeners();

    window.dispatchEvent(new Event('blur'));
    expect(subject.controller.isDragging).toBe(false);
    expect(subject.controller.sliderPreview).toBeNull();

    uninstall();
  });

  it('收起计时触发时仍有内部焦点则保持展开，焦点离开后再收起', () => {
    vi.useFakeTimers();
    const subject = makeController();
    const slider = document.createElement('input');
    subject.wrapper.appendChild(slider);
    document.body.appendChild(subject.wrapper);
    subject.setOpen(true);
    slider.focus();

    subject.controller.handleMouseLeave();
    vi.advanceTimersByTime(799);
    expect(subject.onclose).not.toHaveBeenCalled();

    slider.blur();
    subject.controller.handleFocusOut(
      new FocusEvent('focusout', { relatedTarget: null })
    );
    vi.advanceTimersByTime(799);
    expect(subject.onclose).toHaveBeenCalledTimes(1);

    subject.controller.destroy();
    subject.wrapper.remove();
  });
});
