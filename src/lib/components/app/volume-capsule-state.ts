export enum CapsuleState {
  Closed = 'closed',
  Expanding = 'expanding',
  Open = 'open',
  Collapsing = 'collapsing',
}

export type CapsuleEvent = 'OPEN' | 'EXPANDED' | 'CLOSE' | 'COLLAPSED';

export function transition(
  state: CapsuleState,
  event: CapsuleEvent
): CapsuleState {
  switch (state) {
    case CapsuleState.Closed:
      return event === 'OPEN' ? CapsuleState.Expanding : state;
    case CapsuleState.Expanding:
      return event === 'EXPANDED' ? CapsuleState.Open : state;
    case CapsuleState.Open:
      return event === 'CLOSE' ? CapsuleState.Collapsing : state;
    case CapsuleState.Collapsing:
      return event === 'COLLAPSED' ? CapsuleState.Closed : state;
  }
}
