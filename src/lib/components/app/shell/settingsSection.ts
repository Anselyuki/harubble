/**
 * SettingsSheet 可锚定的分节标识。
 *
 * 供菜单命令等外部入口在打开设置抽屉时指定初始滚动位置；
 * 与 SettingsSheet.svelte 的各分节 `data-settings-section` 属性一一对应。
 */
export type SettingsSection =
  | 'preferences'
  | 'theme'
  | 'notifications'
  | 'cache'
  | 'logs';
