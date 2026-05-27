import { locales, getLocale } from '$lib/paraglide/runtime.js';
import type { TagEditorLocalizedValue } from '$lib/types';

export interface TagLocale {
  key: string;
  label: string;
}

const localeDisplayNames = new Intl.DisplayNames([getLocale()], {
  type: 'language',
  languageDisplay: 'standard',
  style: 'narrow',
});

export const TAG_LOCALES: TagLocale[] = locales.map((loc) => ({
  key: loc,
  label: localeDisplayNames.of(loc.split('-')[0]) ?? loc,
}));

export function displayValue(val: TagEditorLocalizedValue): string {
  return val['zh-CN'] || val['en-US'] || Object.values(val)[0] || '';
}

export function tagIdentity(val: TagEditorLocalizedValue): string {
  return Object.keys(val)
    .sort()
    .map((k) => `${k}=${val[k]}`)
    .join('\0');
}
