import type { TagEditorLocalizedValue, TagEditorRegistry } from '$lib/types';

export type TagLibrary = Record<string, TagEditorLocalizedValue[]>;

export function buildTagLibrary(merged: TagEditorRegistry): TagLibrary {
  const library: TagLibrary = {};

  for (const dim of merged.tagDimensions) {
    library[dim.key] = [];
  }

  for (const album of merged.albums) {
    if (album.type) {
      const typeDef = (
        merged.typeDefinitions as Record<
          string,
          TagEditorLocalizedValue | undefined
        >
      )[album.type];
      if (typeDef) {
        addUnique(library, 'type', typeDef);
      } else {
        addUnique(library, 'type', {
          'zh-CN': album.type,
          'en-US': album.type,
        });
      }
    }
    if (album.faction)
      addUnique(library, 'faction', album.faction as TagEditorLocalizedValue);
    if (album.character)
      addUnique(
        library,
        'character',
        album.character as TagEditorLocalizedValue
      );

    for (const [key, value] of Object.entries(album)) {
      if (
        key !== 'cid' &&
        key !== 'type' &&
        key !== 'name' &&
        key !== 'releaseDate' &&
        key !== 'faction' &&
        key !== 'character' &&
        Array.isArray(value)
      ) {
        for (const v of value as TagEditorLocalizedValue[]) {
          addUnique(library, key, v);
        }
      }
    }
  }

  for (const songTags of Object.values(merged.songs)) {
    for (const [key, values] of Object.entries(
      (songTags as { tags: Record<string, TagEditorLocalizedValue[]> }).tags
    )) {
      for (const v of values) {
        addUnique(library, key, v);
      }
    }
  }

  return library;
}

function addUnique(
  library: TagLibrary,
  key: string,
  value: TagEditorLocalizedValue
): void {
  const zhVal = value['zh-CN'] ?? '';
  const arr = (library[key] ??= []);
  const exists = arr.some((v) => (v['zh-CN'] ?? '') === zhVal);
  if (!exists) {
    arr.push(value);
  }
}

export function filterCandidates(
  library: TagLibrary,
  dimensionKey: string,
  currentValues: TagEditorLocalizedValue[],
  query: string
): TagEditorLocalizedValue[] {
  const all = library[dimensionKey] ?? [];
  const currentZhSet = new Set(currentValues.map((v) => v['zh-CN'] ?? ''));
  const candidates = all.filter((v) => !currentZhSet.has(v['zh-CN'] ?? ''));

  if (!query.trim()) return candidates;

  const q = query.trim().toLowerCase();
  return candidates.filter((v) => {
    const zh = (v['zh-CN'] ?? '').toLowerCase();
    const en = (v['en-US'] ?? '').toLowerCase();
    return zh.includes(q) || en.includes(q);
  });
}
