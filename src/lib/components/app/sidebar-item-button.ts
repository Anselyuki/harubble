export interface SidebarItemActivationOptions {
  collapsed: boolean;
  expandOnCollapsedClick: boolean;
  hasRequestExpand: boolean;
}

export type SidebarItemActivation = 'activate' | 'expand';

export function getSidebarItemActivation({
  collapsed,
  expandOnCollapsedClick,
  hasRequestExpand,
}: SidebarItemActivationOptions): SidebarItemActivation {
  return collapsed && expandOnCollapsedClick && hasRequestExpand
    ? 'expand'
    : 'activate';
}
