import type {
  Collection,
  SongDetail,
  AlbumDetail,
  SongEntry,
  PlaybackQueueEntry,
} from '$lib/types';

/**
 * 单首歌曲的解析结果，聚合了歌曲条目与所属专辑的元信息。
 */
export interface ResolvedSong {
  entry: SongEntry;
  albumCid: string;
  albumName: string;
  coverUrl: string | null;
}

interface ResolvedSongsDeps {
  /** 通过歌曲 CID 获取歌曲详情；调用方负责处理 null（未找到或请求失败）。 */
  getSongDetail: (id: string) => Promise<SongDetail | null>;
  /** 通过专辑 CID 获取专辑详情。 */
  getAlbumDetail: (cid: string) => Promise<AlbumDetail>;
}

/**
 * 创建合集歌曲解析 store。
 *
 * 负责将合集中的 songId 列表批量解析为带封面、专辑信息的 ResolvedSong 列表，
 * 并通过 lastResolvedKey 防止重复请求（stale-request guard）。
 *
 * 用途：由上层 controller / 路由层持有并驱动，CollectionDetailPanel 只消费
 * resolvedSongs、isResolvingSongs 与 playbackQueue，不直接调用 API。
 *
 * @param deps - 注入的 API 函数，便于测试替换。
 *
 * 调用约束：
 * - 在 Svelte 5 响应式上下文（.svelte 或 .svelte.ts）内实例化。
 * - 每次合集变更时调用 resolve(collection)；传入 null 会清空状态并重置 key。
 * - 并发调用由 lastResolvedKey 幂等保护，同一 key 不会重复触发网络请求。
 */
export function createResolvedSongsStore(deps: ResolvedSongsDeps) {
  let resolvedSongs = $state<ResolvedSong[]>([]);
  let isResolvingSongs = $state(false);
  let lastResolvedKey = $state<string | null>(null);

  const playbackQueue = $derived.by((): PlaybackQueueEntry[] =>
    resolvedSongs.map((rs) => ({
      cid: rs.entry.cid,
      name: rs.entry.name,
      artists: rs.entry.artists,
      coverUrl: rs.coverUrl,
    }))
  );

  /**
   * 执行实际的批量解析请求。
   *
   * 前置条件：key 已写入 lastResolvedKey（用于并发检测）。
   * 副作用：更新 resolvedSongs 与 isResolvingSongs；若请求已过期（key 不匹配）则丢弃结果。
   */
  async function resolveSongs(songIds: string[], key: string): Promise<void> {
    lastResolvedKey = key;
    isResolvingSongs = true;
    resolvedSongs = [];

    try {
      const details = await Promise.all(
        songIds.map((id) =>
          deps.getSongDetail(id).catch((): SongDetail | null => null)
        )
      );

      const albumCidList: string[] = [];
      for (const d of details) {
        if (d && !albumCidList.includes(d.albumCid)) {
          albumCidList.push(d.albumCid);
        }
      }

      const albumMap: Partial<
        Record<string, { name: string; coverUrl: string | null }>
      > = {};
      const albumResults = await Promise.all(
        albumCidList.map((cid) => deps.getAlbumDetail(cid).catch(() => null))
      );
      for (const album of albumResults) {
        if (album) {
          albumMap[album.cid] = {
            name: album.name,
            coverUrl: album.coverUrl,
          };
        }
      }

      const resolved: ResolvedSong[] = [];
      for (const detail of details) {
        if (!detail) continue;
        const albumInfo = albumMap[detail.albumCid];
        resolved.push({
          entry: {
            cid: detail.cid,
            name: detail.name,
            artists: detail.artists,
            download: detail.download,
            tags: detail.tags,
          },
          albumCid: detail.albumCid,
          albumName: albumInfo?.name ?? '',
          coverUrl: albumInfo?.coverUrl ?? null,
        });
      }

      if (lastResolvedKey === key) {
        resolvedSongs = resolved;
      }
    } finally {
      if (lastResolvedKey === key) {
        isResolvingSongs = false;
      }
    }
  }

  /**
   * 触发（或跳过）合集歌曲解析。
   *
   * 传入 null 时清空已解析状态并重置 key；
   * 传入相同 key（collectionId + songIds）时幂等跳过，不重复请求。
   *
   * @param collection - 当前选中的合集，null 表示无选中。
   */
  function resolve(collection: Collection | null): void {
    if (!collection) {
      resolvedSongs = [];
      lastResolvedKey = null;
      return;
    }
    const ids = collection.sections.flatMap((s) => s.songIds);
    const key = `${collection.id}:${ids.join(',')}`;
    if (key === lastResolvedKey) return;
    void resolveSongs(ids, key);
  }

  return {
    get resolvedSongs() {
      return resolvedSongs;
    },
    get isResolvingSongs() {
      return isResolvingSongs;
    },
    get playbackQueue() {
      return playbackQueue;
    },
    resolve,
  };
}
