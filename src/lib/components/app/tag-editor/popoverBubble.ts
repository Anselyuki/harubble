export type ExpandDirection =
  | 'bottom-right'
  | 'bottom-left'
  | 'top-right'
  | 'top-left';

export interface Viewport {
  width: number;
  height: number;
}

export interface CardPosition {
  top: number;
  left: number;
}

const DEFAULT_CARD_WIDTH = 280;
const DEFAULT_CARD_MAX_HEIGHT = 320;
const DEFAULT_SAFE_MARGIN = 16;
const DEFAULT_ARROW_SIZE = 8;

export function calcExpandDirection(
  clickX: number,
  clickY: number,
  viewport: Viewport,
  cardWidth = DEFAULT_CARD_WIDTH,
  cardMaxHeight = DEFAULT_CARD_MAX_HEIGHT,
  safeMargin = DEFAULT_SAFE_MARGIN
): ExpandDirection {
  const expandRight = viewport.width - clickX >= cardWidth + safeMargin;
  const expandDown = viewport.height - clickY >= cardMaxHeight + safeMargin;

  if (expandRight && expandDown) return 'bottom-right';
  if (!expandRight && expandDown) return 'bottom-left';
  if (expandRight && !expandDown) return 'top-right';
  return 'top-left';
}

export function calcCardPosition(
  clickX: number,
  clickY: number,
  direction: ExpandDirection,
  arrowSize = DEFAULT_ARROW_SIZE
): CardPosition {
  const offset = arrowSize;

  switch (direction) {
    case 'bottom-right':
      return { top: clickY + offset, left: clickX };
    case 'bottom-left':
      return { top: clickY + offset, left: clickX - DEFAULT_CARD_WIDTH };
    case 'top-right':
      return { top: clickY - offset, left: clickX };
    case 'top-left':
      return { top: clickY - offset, left: clickX - DEFAULT_CARD_WIDTH };
  }
}
