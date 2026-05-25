import { describe, it, expect } from 'vitest';
import { calcExpandDirection, calcCardPosition } from './popoverBubble';

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
    const arrowSize = 8;

    it('positions card below-right of click for bottom-right', () => {
      const pos = calcCardPosition(400, 200, 'bottom-right', arrowSize);
      expect(pos.top).toBeGreaterThan(200);
      expect(pos.left).toBeGreaterThanOrEqual(400);
    });

    it('positions card below-left of click for bottom-left', () => {
      const pos = calcCardPosition(1000, 200, 'bottom-left', arrowSize);
      expect(pos.top).toBeGreaterThan(200);
      expect(pos.left).toBeLessThan(1000);
    });

    it('positions card above-right of click for top-right', () => {
      const pos = calcCardPosition(400, 600, 'top-right', arrowSize);
      expect(pos.top).toBeLessThan(600);
      expect(pos.left).toBeGreaterThanOrEqual(400);
    });

    it('positions card above-left of click for top-left', () => {
      const pos = calcCardPosition(1000, 600, 'top-left', arrowSize);
      expect(pos.top).toBeLessThan(600);
      expect(pos.left).toBeLessThan(1000);
    });
  });
});
