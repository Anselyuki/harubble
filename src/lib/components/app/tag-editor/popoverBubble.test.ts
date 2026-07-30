import { describe, it, expect } from 'vitest';
import {
  calcExpandDirection,
  calcCardPosition,
  clampCardPosition,
  measureBubbleTargetSize,
} from './popoverBubble';

describe('popoverBubble', () => {
  const viewport = { width: 1280, height: 800 };
  const CARD_WIDTH = 280;
  const CARD_MAX_HEIGHT = 320;
  const SAFE_MARGIN = 16;

  describe('calcExpandDirection', () => {
    it('returns bottom-right when click is in center area', () => {
      expect(calcExpandDirection(400, 200, viewport)).toBe('bottom-right');
    });

    it('returns bottom-left when click is near right edge', () => {
      const nearRight = viewport.width - CARD_WIDTH - SAFE_MARGIN + 1;
      expect(calcExpandDirection(nearRight, 200, viewport)).toBe('bottom-left');
    });

    it('returns top-right when click is near bottom edge', () => {
      const nearBottom = viewport.height - CARD_MAX_HEIGHT - SAFE_MARGIN + 1;
      expect(calcExpandDirection(400, nearBottom, viewport)).toBe('top-right');
    });

    it('returns top-left when click is near bottom-right corner', () => {
      const nearRight = viewport.width - CARD_WIDTH - SAFE_MARGIN + 1;
      const nearBottom = viewport.height - CARD_MAX_HEIGHT - SAFE_MARGIN + 1;
      expect(calcExpandDirection(nearRight, nearBottom, viewport)).toBe(
        'top-left'
      );
    });

    it('uses default card dimensions when not specified', () => {
      expect(calcExpandDirection(400, 200, viewport)).toBe('bottom-right');
    });

    it('respects custom card dimensions', () => {
      const result = calcExpandDirection(800, 100, viewport, 500, 600, 20);
      expect(result).toBe('bottom-left');
    });
  });

  describe('calcCardPosition', () => {
    const cardWidth = 220;
    const arrowSize = 8;
    const cardHeight = 320;

    it('positions card below-right of click for bottom-right', () => {
      const pos = calcCardPosition(
        400,
        200,
        'bottom-right',
        cardWidth,
        arrowSize
      );
      expect(pos.top).toBe(200 + arrowSize);
      expect(pos.left).toBe(400);
    });

    it('positions card below-left of click for bottom-left', () => {
      const pos = calcCardPosition(
        1000,
        200,
        'bottom-left',
        cardWidth,
        arrowSize
      );
      expect(pos.top).toBe(200 + arrowSize);
      expect(pos.left).toBe(1000 - cardWidth);
    });

    it('positions card above-right of click for top-right', () => {
      const pos = calcCardPosition(
        400,
        600,
        'top-right',
        cardWidth,
        arrowSize,
        cardHeight
      );
      expect(pos.top).toBe(600 - arrowSize - cardHeight);
      expect(pos.left).toBe(400);
    });

    it('positions card above-left of click for top-left', () => {
      const pos = calcCardPosition(
        1000,
        600,
        'top-left',
        cardWidth,
        arrowSize,
        cardHeight
      );
      expect(pos.top).toBe(600 - arrowSize - cardHeight);
      expect(pos.left).toBe(1000 - cardWidth);
    });

    it('uses default cardHeight when not specified', () => {
      const pos = calcCardPosition(400, 600, 'top-right', cardWidth, arrowSize);
      expect(pos.top).toBe(600 - arrowSize - 320);
    });

    it('card top is always above click point for top directions', () => {
      const pos = calcCardPosition(
        400,
        700,
        'top-right',
        cardWidth,
        arrowSize,
        cardHeight
      );
      expect(pos.top).toBeLessThan(700 - arrowSize);
    });
  });

  describe('measureBubbleTargetSize', () => {
    it('measures scrollHeight at target width and restores original styles', () => {
      const el = {
        style: { width: '24px', height: '24px' },
        scrollHeight: 180,
      } as unknown as HTMLElement;

      const result = measureBubbleTargetSize(el, 220);

      expect(result).toEqual({ width: 220, height: 180 });
      expect(el.style.width).toBe('24px');
      expect(el.style.height).toBe('24px');
    });

    it('returns correct height when element has no prior inline styles', () => {
      const el = {
        style: { width: '', height: '' },
        scrollHeight: 260,
      } as unknown as HTMLElement;

      const result = measureBubbleTargetSize(el, 300);

      expect(result).toEqual({ width: 300, height: 260 });
      expect(el.style.width).toBe('');
      expect(el.style.height).toBe('');
    });
  });

  describe('clampCardPosition', () => {
    it('keeps all card edges inside the viewport safe margin', () => {
      expect(
        clampCardPosition(
          { top: -20, left: 1200 },
          { width: 220, height: 320 },
          viewport
        )
      ).toEqual({ top: 16, left: 1044 });
    });

    it('uses the safe margin when the card is larger than the viewport', () => {
      expect(
        clampCardPosition(
          { top: 100, left: 100 },
          { width: 400, height: 400 },
          { width: 300, height: 300 }
        )
      ).toEqual({ top: 16, left: 16 });
    });
  });
});
