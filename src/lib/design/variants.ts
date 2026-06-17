import { tv } from 'tailwind-variants';

export const toolbarIconButton = tv({
  base: 'inline-flex items-center justify-center rounded-full border border-transparent text-[var(--text-primary)] transition-colors',
  variants: {
    active: {
      true: 'bg-[var(--surface-state)] text-[var(--accent)]',
      false: 'bg-transparent hover:bg-[var(--surface-state)]',
    },
  },
});
