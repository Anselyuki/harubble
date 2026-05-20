import { del, entries, get, keys, set } from 'idb-keyval';
import type { AlbumDetail, SongDetail, ThemePalette } from './types';

const HOUR_MS = 60 * 60 * 1000;
const PERSISTENCE_KEY_PREFIX = 'phase9-cache:';
const PERSISTENCE_LATEST_ALBUMS_KEY = `${PERSISTENCE_KEY_PREFIX}warm:albums`;
const WARM_ALBUM_LIMIT = 10;

type CacheType = 'albums' | 'songs' | 'lyrics' | 'themes' | 'covers';

type CacheStatsSnapshot = Record<
  CacheType,
  {
    size: number;
    hits: number;
    misses: number;
    evictions: number;
  }
>;

interface CacheEntry<T> {
  data: T;
  timestamp: number;
  lastAccessedAt: number;
  tags: string[];
}

interface TieredCacheOptions {
  ttlMs: number;
  maxEntries: number;
  persistent: boolean;
}

type CacheLookup<T> =
  | {
      found: true;
      data: T;
    }
  | {
      found: false;
      data: null;
    };

interface LatestAlbumsSnapshot {
  keys: string[];
}

interface SerializedCacheEntry<T> extends CacheEntry<T> {
  type: CacheType;
  key: string;
}

const CACHE_OPTIONS: Record<CacheType, TieredCacheOptions> = {
  albums: {
    ttlMs: 6 * HOUR_MS,
    maxEntries: 50,
    persistent: true,
  },
  songs: {
    ttlMs: 6 * HOUR_MS,
    maxEntries: 200,
    persistent: true,
  },
  lyrics: {
    ttlMs: 6 * HOUR_MS,
    maxEntries: 200,
    persistent: true,
  },
  themes: {
    ttlMs: 24 * HOUR_MS,
    maxEntries: 200,
    persistent: false,
  },
  covers: {
    ttlMs: 6 * HOUR_MS,
    maxEntries: 100,
    persistent: false,
  },
};

function persistenceKey(type: CacheType, key: string): string {
  return `${PERSISTENCE_KEY_PREFIX}${type}:${key}`;
}

function isEntryValid(entry: CacheEntry<unknown>, ttlMs: number): boolean {
  return Date.now() - entry.timestamp < ttlMs;
}

function isCacheType(value: string): value is CacheType {
  return (
    value === 'albums' ||
    value === 'songs' ||
    value === 'lyrics' ||
    value === 'themes' ||
    value === 'covers'
  );
}

function isSerializedCacheEntry<T>(
  value: unknown
): value is SerializedCacheEntry<T> {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Partial<SerializedCacheEntry<T>>;
  return (
    typeof candidate.type === 'string' &&
    isCacheType(candidate.type) &&
    typeof candidate.key === 'string' &&
    typeof candidate.timestamp === 'number' &&
    typeof candidate.lastAccessedAt === 'number' &&
    Array.isArray(candidate.tags)
  );
}

function cloneEntry<T>(entry: CacheEntry<T>): CacheEntry<T> {
  return {
    data: entry.data,
    timestamp: entry.timestamp,
    lastAccessedAt: entry.lastAccessedAt,
    tags: [...entry.tags],
  };
}

function updateSetMap(
  map: Map<string, Set<string>>,
  mapKey: string,
  value: string
): void {
  const existing = map.get(mapKey) ?? new Set<string>();
  existing.add(value);
  map.set(mapKey, existing);
}

class TieredCache<T> {
  readonly type: CacheType;

  private readonly options: TieredCacheOptions;
  private readonly memory = new Map<string, CacheEntry<T>>();
  private readonly hitsState: Record<CacheType, number>;
  private readonly missesState: Record<CacheType, number>;
  private readonly evictionsState: Record<CacheType, number>;
  private readonly tagIndex: Map<string, Set<string>>;
  private readonly keyTags: Map<string, Set<string>>;

  constructor(
    type: CacheType,
    options: TieredCacheOptions,
    hitsState: Record<CacheType, number>,
    missesState: Record<CacheType, number>,
    evictionsState: Record<CacheType, number>,
    tagIndex: Map<string, Set<string>>,
    keyTags: Map<string, Set<string>>
  ) {
    this.type = type;
    this.options = options;
    this.hitsState = hitsState;
    this.missesState = missesState;
    this.evictionsState = evictionsState;
    this.tagIndex = tagIndex;
    this.keyTags = keyTags;
  }

  get size(): number {
    return this.memory.size;
  }

  async get(key: string): Promise<CacheLookup<T>> {
    const memoryEntry = this.memory.get(key);
    if (memoryEntry) {
      if (isEntryValid(memoryEntry, this.options.ttlMs)) {
        this.hitsState[this.type] += 1;
        const refreshed = {
          ...cloneEntry(memoryEntry),
          lastAccessedAt: Date.now(),
        };
        this.memory.delete(key);
        this.memory.set(key, refreshed);
        if (this.options.persistent) {
          await this.persistEntry(key, refreshed);
        }
        return { found: true, data: refreshed.data };
      }
      await this.delete(key);
    }

    if (!this.options.persistent) {
      this.missesState[this.type] += 1;
      return { found: false, data: null };
    }

    const persisted = await get<SerializedCacheEntry<T>>(
      persistenceKey(this.type, key)
    );
    if (!persisted) {
      this.missesState[this.type] += 1;
      return { found: false, data: null };
    }

    const entry: CacheEntry<T> = {
      data: persisted.data,
      timestamp: persisted.timestamp,
      lastAccessedAt: persisted.lastAccessedAt,
      tags: persisted.tags,
    };

    if (!isEntryValid(entry, this.options.ttlMs)) {
      await this.delete(key);
      this.missesState[this.type] += 1;
      return { found: false, data: null };
    }

    const refreshed = {
      ...entry,
      lastAccessedAt: Date.now(),
    };
    this.hitsState[this.type] += 1;
    this.memory.set(key, refreshed);
    this.registerTags(key, refreshed.tags);
    await this.persistEntry(key, refreshed);
    await this.trimMemory();
    return { found: true, data: refreshed.data };
  }

  async set(key: string, data: T, tags: string[] = []): Promise<void> {
    const entry: CacheEntry<T> = {
      data,
      timestamp: Date.now(),
      lastAccessedAt: Date.now(),
      tags: [...new Set(tags)],
    };

    this.memory.delete(key);
    this.memory.set(key, entry);
    this.registerTags(key, entry.tags);
    if (this.options.persistent) {
      await this.persistEntry(key, entry);
    }
    await this.trimMemory();
  }

  async delete(key: string): Promise<void> {
    this.memory.delete(key);
    this.unregisterTags(key);
    if (this.options.persistent) {
      await del(persistenceKey(this.type, key));
      if (this.type === 'albums') {
        await cacheManager.syncLatestAlbumKeys();
      }
    }
  }

  async clear(): Promise<void> {
    const cacheKeys = [...this.memory.keys()];
    this.memory.clear();
    for (const key of cacheKeys) {
      this.unregisterTags(key);
    }

    if (!this.options.persistent) {
      return;
    }

    const persistentKeys = await keys();
    const scopedKeys = persistentKeys.filter(
      (value: IDBValidKey) =>
        typeof value === 'string' &&
        value.startsWith(`${PERSISTENCE_KEY_PREFIX}${this.type}:`)
    );
    await Promise.all(
      scopedKeys.map((value: IDBValidKey) => del(String(value)))
    );
    if (this.type === 'albums') {
      await set(PERSISTENCE_LATEST_ALBUMS_KEY, {
        keys: [],
      } satisfies LatestAlbumsSnapshot);
    }
  }

  async warmStart(keysToLoad: string[]): Promise<void> {
    if (!this.options.persistent) {
      return;
    }

    for (const key of keysToLoad.slice(0, WARM_ALBUM_LIMIT)) {
      const persisted = await get<SerializedCacheEntry<T>>(
        persistenceKey(this.type, key)
      );
      if (!persisted) {
        continue;
      }
      const entry: CacheEntry<T> = {
        data: persisted.data,
        timestamp: persisted.timestamp,
        lastAccessedAt: persisted.lastAccessedAt,
        tags: persisted.tags,
      };
      if (!isEntryValid(entry, this.options.ttlMs)) {
        await this.delete(key);
        continue;
      }
      this.memory.set(key, entry);
      this.registerTags(key, entry.tags);
    }

    await this.trimMemory();
  }

  async keysByTag(tag: string): Promise<string[]> {
    return [...(this.tagIndex.get(tag) ?? [])];
  }

  private async persistEntry(key: string, entry: CacheEntry<T>): Promise<void> {
    if (!this.options.persistent) {
      return;
    }

    await set(persistenceKey(this.type, key), {
      type: this.type,
      key,
      data: entry.data,
      timestamp: entry.timestamp,
      lastAccessedAt: entry.lastAccessedAt,
      tags: entry.tags,
    } satisfies SerializedCacheEntry<T>);

    await this.trimPersistent();
    if (this.type === 'albums') {
      await cacheManager.syncLatestAlbumKeys();
    }
  }

  private registerTags(key: string, tags: string[]): void {
    this.unregisterTags(key);
    const nextTags = new Set(tags);
    this.keyTags.set(`${this.type}:${key}`, nextTags);
    for (const tag of nextTags) {
      updateSetMap(this.tagIndex, tag, `${this.type}:${key}`);
    }
  }

  private unregisterTags(key: string): void {
    const scopedKey = `${this.type}:${key}`;
    const tags = this.keyTags.get(scopedKey);
    if (!tags) {
      return;
    }

    for (const tag of tags) {
      const scopedKeys = this.tagIndex.get(tag);
      if (!scopedKeys) {
        continue;
      }
      scopedKeys.delete(scopedKey);
      if (scopedKeys.size === 0) {
        this.tagIndex.delete(tag);
      }
    }

    this.keyTags.delete(scopedKey);
  }

  private async trimPersistent(): Promise<void> {
    if (!this.options.persistent) {
      return;
    }

    const persistentEntries = await entries();
    const scopedEntries = persistentEntries
      .flatMap(([key, value]: [IDBValidKey, unknown]) => {
        if (
          typeof key !== 'string' ||
          !key.startsWith(`${PERSISTENCE_KEY_PREFIX}${this.type}:`) ||
          !isSerializedCacheEntry<T>(value)
        ) {
          return [];
        }

        return [
          {
            storageKey: String(key),
            entry: value,
          },
        ];
      })
      .sort(
        (left, right) => right.entry.lastAccessedAt - left.entry.lastAccessedAt
      );

    const overflowEntries = scopedEntries.slice(this.options.maxEntries);
    await Promise.all(overflowEntries.map(({ storageKey }) => del(storageKey)));
    this.evictionsState[this.type] += overflowEntries.length;
  }

  private async trimMemory(): Promise<void> {
    while (this.memory.size > this.options.maxEntries) {
      const [oldestKey] = this.memory.keys();
      if (!oldestKey) {
        break;
      }
      this.memory.delete(oldestKey);
      this.unregisterTags(oldestKey);
      this.evictionsState[this.type] += 1;
    }
  }
}

class CacheManager {
  readonly hits: Record<CacheType, number> = {
    albums: 0,
    songs: 0,
    lyrics: 0,
    themes: 0,
    covers: 0,
  };

  readonly misses: Record<CacheType, number> = {
    albums: 0,
    songs: 0,
    lyrics: 0,
    themes: 0,
    covers: 0,
  };

  readonly evictions: Record<CacheType, number> = {
    albums: 0,
    songs: 0,
    lyrics: 0,
    themes: 0,
    covers: 0,
  };

  private readonly tagIndex = new Map<string, Set<string>>();
  private readonly keyTags = new Map<string, Set<string>>();

  readonly albums = new TieredCache<AlbumDetail>(
    'albums',
    CACHE_OPTIONS.albums,
    this.hits,
    this.misses,
    this.evictions,
    this.tagIndex,
    this.keyTags
  );
  readonly songs = new TieredCache<SongDetail>(
    'songs',
    CACHE_OPTIONS.songs,
    this.hits,
    this.misses,
    this.evictions,
    this.tagIndex,
    this.keyTags
  );
  readonly lyrics = new TieredCache<string | null>(
    'lyrics',
    CACHE_OPTIONS.lyrics,
    this.hits,
    this.misses,
    this.evictions,
    this.tagIndex,
    this.keyTags
  );
  readonly themes = new TieredCache<ThemePalette>(
    'themes',
    CACHE_OPTIONS.themes,
    this.hits,
    this.misses,
    this.evictions,
    this.tagIndex,
    this.keyTags
  );
  readonly covers = new TieredCache<string>(
    'covers',
    CACHE_OPTIONS.covers,
    this.hits,
    this.misses,
    this.evictions,
    this.tagIndex,
    this.keyTags
  );

  async invalidateKey(key: string): Promise<void> {
    const [type, ...rest] = key.split(':');
    const unscopedKey = rest.join(':');
    if (!type || !unscopedKey || !isCacheType(type)) {
      return;
    }

    switch (type) {
      case 'albums':
        await this.albums.delete(unscopedKey);
        break;
      case 'songs':
        await this.songs.delete(unscopedKey);
        break;
      case 'lyrics':
        await this.lyrics.delete(unscopedKey);
        break;
      case 'themes':
        await this.themes.delete(unscopedKey);
        break;
      case 'covers':
        await this.covers.delete(unscopedKey);
        break;
      default:
        break;
    }
  }

  async invalidateByTag(tag: string): Promise<void> {
    const scopedKeys = new Set<string>(this.tagIndex.get(tag) ?? []);

    const persistentEntries = await entries();
    for (const [key, value] of persistentEntries) {
      if (typeof key !== 'string' || !key.startsWith(PERSISTENCE_KEY_PREFIX)) {
        continue;
      }
      if (key === PERSISTENCE_LATEST_ALBUMS_KEY) {
        continue;
      }
      if (!isSerializedCacheEntry<unknown>(value)) {
        continue;
      }

      if (!value.tags.includes(tag)) {
        continue;
      }
      scopedKeys.add(`${value.type}:${value.key}`);
    }

    await Promise.all([...scopedKeys].map((key) => this.invalidateKey(key)));
  }

  async clearAll(): Promise<void> {
    await Promise.all([
      this.albums.clear(),
      this.songs.clear(),
      this.lyrics.clear(),
      this.themes.clear(),
      this.covers.clear(),
    ]);
    this.tagIndex.clear();
    this.keyTags.clear();
  }

  getCacheStats(): CacheStatsSnapshot {
    return {
      albums: {
        size: this.albums.size,
        hits: this.hits.albums,
        misses: this.misses.albums,
        evictions: this.evictions.albums,
      },
      songs: {
        size: this.songs.size,
        hits: this.hits.songs,
        misses: this.misses.songs,
        evictions: this.evictions.songs,
      },
      lyrics: {
        size: this.lyrics.size,
        hits: this.hits.lyrics,
        misses: this.misses.lyrics,
        evictions: this.evictions.lyrics,
      },
      themes: {
        size: this.themes.size,
        hits: this.hits.themes,
        misses: this.misses.themes,
        evictions: this.evictions.themes,
      },
      covers: {
        size: this.covers.size,
        hits: this.hits.covers,
        misses: this.misses.covers,
        evictions: this.evictions.covers,
      },
    };
  }

  async warmStart(): Promise<void> {
    const latestAlbums = (await get<LatestAlbumsSnapshot>(
      PERSISTENCE_LATEST_ALBUMS_KEY
    )) ?? {
      keys: [],
    };
    await this.albums.warmStart(latestAlbums.keys);
  }

  async syncLatestAlbumKeys(): Promise<void> {
    const cachedEntries = await entries();
    const latestAlbumKeys = cachedEntries
      .flatMap(([key, value]: [IDBValidKey, unknown]) => {
        if (
          typeof key !== 'string' ||
          !key.startsWith(`${PERSISTENCE_KEY_PREFIX}albums:`) ||
          !isSerializedCacheEntry<unknown>(value)
        ) {
          return [];
        }

        return [
          {
            key: String(key).replace(`${PERSISTENCE_KEY_PREFIX}albums:`, ''),
            lastAccessedAt: value.lastAccessedAt,
          },
        ];
      })
      .sort(
        (
          left: { key: string; lastAccessedAt: number },
          right: { key: string; lastAccessedAt: number }
        ) => right.lastAccessedAt - left.lastAccessedAt
      )
      .slice(0, WARM_ALBUM_LIMIT)
      .map((entry: { key: string; lastAccessedAt: number }) => entry.key);

    await set(PERSISTENCE_LATEST_ALBUMS_KEY, {
      keys: latestAlbumKeys,
    } satisfies LatestAlbumsSnapshot);
  }
}

export const cacheManager = new CacheManager();

export function createAlbumCacheTag(albumCid: string): string {
  return `tag:album:${albumCid}`;
}

export function createSongCacheTag(songCid: string): string {
  return `tag:song:${songCid}`;
}

export function createInventoryCacheTag(
  inventoryVersion: string | null | undefined
): string {
  return `tag:inventory:${inventoryVersion ?? 'unversioned'}`;
}

export async function clearCache(): Promise<void> {
  await cacheManager.clearAll();
}

export async function invalidateKey(key: string): Promise<void> {
  await cacheManager.invalidateKey(key);
}

export async function invalidateByTag(tag: string): Promise<void> {
  await cacheManager.invalidateByTag(tag);
}

export async function warmCacheManager(): Promise<void> {
  await cacheManager.warmStart();
}
