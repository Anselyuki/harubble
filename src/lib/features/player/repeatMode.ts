import type { RepeatMode } from '$lib/types';

export function getNextRepeatMode(mode: RepeatMode): RepeatMode {
  switch (mode) {
    case 'off':
      return 'all';
    case 'all':
      return 'one';
    case 'one':
      return 'off';
  }
}
