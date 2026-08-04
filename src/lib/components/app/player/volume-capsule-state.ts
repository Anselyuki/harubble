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
      if (event === 'EXPANDED') return CapsuleState.Open;
      return event === 'CLOSE' ? CapsuleState.Collapsing : state;
    case CapsuleState.Open:
      return event === 'CLOSE' ? CapsuleState.Collapsing : state;
    case CapsuleState.Collapsing:
      if (event === 'COLLAPSED') return CapsuleState.Closed;
      return event === 'OPEN' ? CapsuleState.Expanding : state;
  }
}
