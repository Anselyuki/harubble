interface SidebarItemActivationOptions {
  collapsed: boolean;
  expandOnCollapsedClick: boolean;
  hasRequestExpand: boolean;
}

type SidebarItemActivation = 'activate' | 'expand';

export function getSidebarItemActivation({
  collapsed,
  expandOnCollapsedClick,
  hasRequestExpand,
}: SidebarItemActivationOptions): SidebarItemActivation {
  return collapsed && expandOnCollapsedClick && hasRequestExpand
    ? 'expand'
    : 'activate';
}
