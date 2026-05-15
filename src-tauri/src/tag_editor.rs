use crate::tag_registry::{
    albums_to_tag_map, songs_to_tag_map, tag_map_to_albums, tag_map_to_songs, LocalizedValue,
    TagDimension, TagRegistry, TagSet, CURRENT_SCHEMA_VERSION,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tempfile::NamedTempFile;

const REMOTE_FILE_NAME: &str = "tag_registry_remote.json";
const LOCAL_FILE_NAME: &str = "tag_registry_local.json";

/// 编辑器内部存储结构，使用 HashMap 便于 CRUD 操作。
#[derive(Debug, Clone, Default)]
struct EditorStore {
    schema_version: u32,
    updated_at: String,
    tag_dimensions: Vec<TagDimension>,
    type_definitions: HashMap<String, LocalizedValue>,
    albums: HashMap<String, TagSet>,
    songs: HashMap<String, TagSet>,
}

impl EditorStore {
    fn from_registry(registry: &TagRegistry) -> Self {
        Self {
            schema_version: registry.schema_version,
            updated_at: registry.updated_at.clone(),
            tag_dimensions: registry.tag_dimensions.clone(),
            type_definitions: registry.type_definitions.clone(),
            albums: albums_to_tag_map(&registry.albums, &registry.type_definitions),
            songs: songs_to_tag_map(&registry.songs),
        }
    }

    fn to_registry(&self) -> TagRegistry {
        TagRegistry {
            schema_version: self.schema_version,
            updated_at: self.updated_at.clone(),
            tag_dimensions: self.tag_dimensions.clone(),
            type_definitions: self.type_definitions.clone(),
            albums: tag_map_to_albums(&self.albums, &self.type_definitions),
            songs: tag_map_to_songs(&self.songs),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    Album,
    Song,
}

/// 三路合并中检测到的单个冲突条目。
///
/// 当 base、remote、local 三方对同一实体的同一维度均有不同修改时产生冲突，
/// 需要用户手动选择保留哪一方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeConflict {
    pub entity_type: EntityType,
    pub cid: String,
    pub dimension_key: String,
    pub base_values: Option<Vec<LocalizedValue>>,
    pub remote_values: Option<Vec<LocalizedValue>>,
    pub local_values: Option<Vec<LocalizedValue>>,
}

/// 三路合并的执行结果。
///
/// 包含自动合并的条目数量与需要用户手动解决的冲突列表。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub conflicts: Vec<MergeConflict>,
    pub auto_merged_count: u32,
}

/// 冲突解决策略。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictResolution {
    KeepLocal,
    KeepRemote,
}

#[derive(Clone)]
pub(crate) struct TagEditorService {
    remote: Arc<RwLock<EditorStore>>,
    local: Arc<RwLock<EditorStore>>,
    remote_path: PathBuf,
    local_path: PathBuf,
}

impl TagEditorService {
    pub(crate) fn new(app_data_dir: &Path) -> Self {
        let remote_path = app_data_dir.join(REMOTE_FILE_NAME);
        let local_path = app_data_dir.join(LOCAL_FILE_NAME);
        let remote = load_registry(&remote_path)
            .map(|r| EditorStore::from_registry(&r))
            .unwrap_or_else(empty_store);
        let local = load_registry(&local_path)
            .map(|r| EditorStore::from_registry(&r))
            .unwrap_or_else(empty_store);
        Self {
            remote: Arc::new(RwLock::new(remote)),
            local: Arc::new(RwLock::new(local)),
            remote_path,
            local_path,
        }
    }

    #[cfg(test)]
    pub(crate) fn remote_registry(&self) -> TagRegistry {
        self.remote
            .read()
            .expect("tag_editor remote RwLock poisoned")
            .to_registry()
    }

    pub(crate) fn local_registry(&self) -> TagRegistry {
        self.local
            .read()
            .expect("tag_editor local RwLock poisoned")
            .to_registry()
    }
    pub(crate) fn set_entity_tag(
        &self,
        entity_type: EntityType,
        cid: &str,
        dimension_key: &str,
        values: Vec<LocalizedValue>,
    ) -> Result<()> {
        {
            let mut local = self
                .local
                .write()
                .expect("tag_editor local RwLock poisoned");
            let map = match entity_type {
                EntityType::Album => &mut local.albums,
                EntityType::Song => &mut local.songs,
            };
            let tag_set = map.entry(cid.to_string()).or_default();
            tag_set.tags.insert(dimension_key.to_string(), values);
        }
        self.persist_local()
    }

    pub(crate) fn remove_entity_tag(
        &self,
        entity_type: EntityType,
        cid: &str,
        dimension_key: &str,
    ) -> Result<()> {
        {
            let mut local = self
                .local
                .write()
                .expect("tag_editor local RwLock poisoned");
            let map = match entity_type {
                EntityType::Album => &mut local.albums,
                EntityType::Song => &mut local.songs,
            };
            if let Some(tag_set) = map.get_mut(cid) {
                tag_set.tags.remove(dimension_key);
                if tag_set.tags.is_empty() {
                    map.remove(cid);
                }
            }
        }
        self.persist_local()
    }

    pub(crate) fn add_local_dimension(
        &self,
        key: &str,
        label_zh: &str,
        label_en: &str,
    ) -> Result<()> {
        {
            let remote = self.remote.read().expect("poisoned");
            if remote.tag_dimensions.iter().any(|d| d.key == key) {
                anyhow::bail!("维度 key '{key}' 已存在于远端注册表中");
            }
            let mut local = self.local.write().expect("poisoned");
            if local.tag_dimensions.iter().any(|d| d.key == key) {
                anyhow::bail!("维度 key '{key}' 已存在于本地注册表中");
            }
            local.tag_dimensions.push(TagDimension {
                key: key.to_string(),
                label: HashMap::from([
                    ("zh-CN".to_string(), label_zh.to_string()),
                    ("en-US".to_string(), label_en.to_string()),
                ]),
                scope: None,
            });
        }
        self.persist_local()
    }
    pub(crate) fn remove_local_dimension(&self, key: &str) -> Result<()> {
        {
            let mut local = self.local.write().expect("poisoned");
            local.tag_dimensions.retain(|d| d.key != key);
            for tag_set in local.albums.values_mut() {
                tag_set.tags.remove(key);
            }
            for tag_set in local.songs.values_mut() {
                tag_set.tags.remove(key);
            }
            local.albums.retain(|_, ts| !ts.tags.is_empty());
            local.songs.retain(|_, ts| !ts.tags.is_empty());
        }
        self.persist_local()
    }

    pub(crate) fn compute_merged(&self) -> TagRegistry {
        let remote = self.remote.read().expect("poisoned");
        let local = self.local.read().expect("poisoned");

        let dimensions = merge_dimensions(&remote.tag_dimensions, &local.tag_dimensions);
        let type_definitions = remote.type_definitions.clone();
        let albums = merge_entity_maps(&remote.albums, &local.albums);
        let songs = merge_entity_maps(&remote.songs, &local.songs);

        TagRegistry {
            schema_version: remote.schema_version.max(local.schema_version).max(1),
            updated_at: remote.updated_at.clone(),
            tag_dimensions: dimensions,
            type_definitions,
            albums: tag_map_to_albums(&albums, &remote.type_definitions),
            songs: tag_map_to_songs(&songs),
        }
    }

    /// 接收新的远端快照，执行三路合并。
    ///
    /// base = 当前 self.remote（上次同步时的远端状态）
    /// theirs = new_remote（本次拉取的新远端状态）
    /// ours = self.local（用户本地编辑）
    ///
    /// 合并完成后更新 self.remote 为 new_remote，自动合并的部分从 local 中移除。
    pub(crate) fn apply_remote_update(&self, new_remote: TagRegistry) -> Result<MergeResult> {
        let mut remote_guard = self.remote.write().expect("poisoned");
        let mut local_guard = self.local.write().expect("poisoned");

        let base = remote_guard.clone();
        let local = local_guard.clone();
        let new_remote_store = EditorStore::from_registry(&new_remote);

        let mut conflicts = Vec::new();
        let mut auto_merged_count: u32 = 0;
        let mut new_local = local.clone();

        for entity_type in [EntityType::Album, EntityType::Song] {
            let (base_map, remote_map, local_map, new_local_map) = match entity_type {
                EntityType::Album => (
                    &base.albums,
                    &new_remote_store.albums,
                    &local.albums,
                    &mut new_local.albums,
                ),
                EntityType::Song => (
                    &base.songs,
                    &new_remote_store.songs,
                    &local.songs,
                    &mut new_local.songs,
                ),
            };

            let all_cids: HashSet<&str> = local_map.keys().map(|s| s.as_str()).collect();

            for cid in all_cids {
                let local_set = match local_map.get(cid) {
                    Some(s) => s,
                    None => continue,
                };

                let all_dim_keys: HashSet<&str> =
                    local_set.tags.keys().map(|s| s.as_str()).collect();

                for dim_key in all_dim_keys {
                    let base_vals = base_map.get(cid).and_then(|ts| ts.tags.get(dim_key));
                    let remote_vals = remote_map.get(cid).and_then(|ts| ts.tags.get(dim_key));
                    let local_vals = local_set.tags.get(dim_key);

                    let base_eq_remote = base_vals == remote_vals;
                    let base_eq_local = base_vals == local_vals;

                    if base_eq_remote && base_eq_local {
                        if let Some(ts) = new_local_map.get_mut(cid) {
                            ts.tags.remove(dim_key);
                        }
                        auto_merged_count += 1;
                    } else if base_eq_remote {
                        // 远端未变，本地有修改 → 保留本地
                    } else if base_eq_local {
                        if let Some(ts) = new_local_map.get_mut(cid) {
                            ts.tags.remove(dim_key);
                        }
                        auto_merged_count += 1;
                    } else {
                        conflicts.push(MergeConflict {
                            entity_type,
                            cid: cid.to_string(),
                            dimension_key: dim_key.to_string(),
                            base_values: base_vals.cloned(),
                            remote_values: remote_vals.cloned(),
                            local_values: local_vals.cloned(),
                        });
                    }
                }

                if let Some(ts) = new_local_map.get(cid) {
                    if ts.tags.is_empty() {
                        new_local_map.remove(cid);
                    }
                }
            }
        }

        *remote_guard = new_remote_store;
        *local_guard = new_local;

        drop(remote_guard);
        drop(local_guard);

        self.persist_remote()?;
        self.persist_local()?;

        Ok(MergeResult {
            conflicts,
            auto_merged_count,
        })
    }

    /// 解决单个冲突：用户选择保留 local 或 remote。
    ///
    /// - KeepLocal：保留本地 overlay 中的值不变（冲突已由用户确认）。
    /// - KeepRemote：从本地 overlay 中移除该条目，使 compute_merged 回退到远端值。
    pub(crate) fn resolve_conflict(
        &self,
        entity_type: EntityType,
        cid: &str,
        dimension_key: &str,
        keep: ConflictResolution,
    ) -> Result<()> {
        match keep {
            ConflictResolution::KeepLocal => {
                // 本地值已在 overlay 中，无需修改
            }
            ConflictResolution::KeepRemote => {
                let mut local = self.local.write().expect("poisoned");
                let map = match entity_type {
                    EntityType::Album => &mut local.albums,
                    EntityType::Song => &mut local.songs,
                };
                if let Some(tag_set) = map.get_mut(cid) {
                    tag_set.tags.remove(dimension_key);
                    if tag_set.tags.is_empty() {
                        map.remove(cid);
                    }
                }
            }
        }
        self.persist_local()
    }

    fn persist_local(&self) -> Result<()> {
        let local = self.local.read().expect("tag_editor local RwLock poisoned");
        let registry = local.to_registry();
        persist_registry(&self.local_path, &registry)
    }

    fn persist_remote(&self) -> Result<()> {
        let remote = self
            .remote
            .read()
            .expect("tag_editor remote RwLock poisoned");
        let registry = remote.to_registry();
        persist_registry(&self.remote_path, &registry)
    }
}

fn empty_store() -> EditorStore {
    EditorStore {
        schema_version: CURRENT_SCHEMA_VERSION,
        ..Default::default()
    }
}

fn load_registry(path: &Path) -> Option<TagRegistry> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let registry: TagRegistry = serde_json::from_str(&content).ok()?;
    if registry.schema_version != CURRENT_SCHEMA_VERSION {
        return None;
    }
    Some(registry)
}

fn persist_registry(path: &Path, registry: &TagRegistry) -> Result<()> {
    let parent = path.parent().context("缓存文件路径无父目录")?;
    std::fs::create_dir_all(parent).context("创建缓存目录失败")?;
    let content = serde_json::to_vec_pretty(registry).context("序列化失败")?;
    let mut temp = NamedTempFile::new_in(parent).context("创建临时文件失败")?;
    temp.write_all(&content).context("写入临时文件失败")?;
    temp.as_file().sync_all().context("sync 失败")?;
    temp.persist(path)
        .map_err(|e| e.error)
        .context("原子重命名失败")?;
    Ok(())
}

fn merge_dimensions(remote: &[TagDimension], local: &[TagDimension]) -> Vec<TagDimension> {
    let mut result = remote.to_vec();
    let remote_keys: HashSet<&str> = remote.iter().map(|d| d.key.as_str()).collect();
    for dim in local {
        if !remote_keys.contains(dim.key.as_str()) {
            result.push(dim.clone());
        }
    }
    result
}

fn merge_entity_maps(
    remote: &HashMap<String, TagSet>,
    local: &HashMap<String, TagSet>,
) -> HashMap<String, TagSet> {
    let mut result: HashMap<String, TagSet> = HashMap::new();
    let all_cids: HashSet<&str> = remote
        .keys()
        .chain(local.keys())
        .map(|s| s.as_str())
        .collect();

    for cid in all_cids {
        let remote_set = remote.get(cid);
        let local_set = local.get(cid);
        let merged_set = merge_tag_sets(remote_set, local_set);
        if !merged_set.tags.is_empty() {
            result.insert(cid.to_string(), merged_set);
        }
    }
    result
}
fn merge_tag_sets(remote: Option<&TagSet>, local: Option<&TagSet>) -> TagSet {
    match (remote, local) {
        (None, None) => TagSet::default(),
        (Some(r), None) => r.clone(),
        (None, Some(l)) => l.clone(),
        (Some(r), Some(l)) => {
            let mut merged_tags: HashMap<String, Vec<LocalizedValue>> = HashMap::new();
            let all_keys: HashSet<&str> = r
                .tags
                .keys()
                .chain(l.tags.keys())
                .map(|s| s.as_str())
                .collect();
            for key in all_keys {
                let r_vals = r.tags.get(key);
                let l_vals = l.tags.get(key);
                let merged_vals = match (r_vals, l_vals) {
                    (None, None) => continue,
                    (Some(rv), None) => rv.clone(),
                    (None, Some(lv)) => lv.clone(),
                    (Some(rv), Some(lv)) => union_localized_values(rv, lv),
                };
                merged_tags.insert(key.to_string(), merged_vals);
            }
            TagSet { tags: merged_tags }
        }
    }
}

fn union_localized_values(a: &[LocalizedValue], b: &[LocalizedValue]) -> Vec<LocalizedValue> {
    let mut result = a.to_vec();
    for val in b {
        if let Some(existing) = result.iter_mut().find(|e| e.text_eq(val)) {
            *existing = LocalizedValue::merge_metadata(existing, val);
        } else {
            result.push(val.clone());
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag_registry::{songs_to_tag_map, AlbumEntry, SongRegistryEntry};

    fn make_lv(value: &str) -> LocalizedValue {
        LocalizedValue(HashMap::from([("zh-CN".to_string(), value.to_string())]))
    }

    fn lv_zh(lv: &Option<LocalizedValue>) -> Option<&str> {
        lv.as_ref()
            .and_then(|v| v.0.get("zh-CN").map(|s| s.as_str()))
    }

    fn make_album_entry(cid: &str, faction: Option<&str>) -> AlbumEntry {
        AlbumEntry {
            cid: cid.to_string(),
            faction: faction.map(make_lv),
            ..Default::default()
        }
    }

    fn make_registry_with_album(cid: &str, faction: &str, updated_at: &str) -> TagRegistry {
        TagRegistry {
            schema_version: CURRENT_SCHEMA_VERSION,
            updated_at: updated_at.to_string(),
            tag_dimensions: vec![],
            type_definitions: HashMap::new(),
            albums: vec![AlbumEntry {
                cid: cid.to_string(),
                faction: Some(make_lv(faction)),
                ..Default::default()
            }],
            songs: Vec::new(),
        }
    }

    fn find_album_in_registry<'a>(registry: &'a TagRegistry, cid: &str) -> Option<&'a AlbumEntry> {
        registry.albums.iter().find(|a| a.cid == cid)
    }

    #[test]
    fn new_service_with_empty_dir_has_empty_layers() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        assert!(svc.remote_registry().albums.is_empty());
        assert!(svc.local_registry().albums.is_empty());
    }

    #[test]
    fn new_service_loads_existing_remote_cache() {
        let dir = tempfile::tempdir().unwrap();
        let remote = TagRegistry {
            schema_version: CURRENT_SCHEMA_VERSION,
            updated_at: "2026-05-01T00:00:00Z".to_string(),
            tag_dimensions: vec![],
            type_definitions: HashMap::new(),
            albums: vec![make_album_entry("A1", Some("罗德岛"))],
            songs: Vec::new(),
        };
        let path = dir.path().join("tag_registry_remote.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&remote).unwrap()).unwrap();
        let svc = TagEditorService::new(dir.path());
        assert!(find_album_in_registry(&svc.remote_registry(), "A1").is_some());
    }

    #[test]
    fn new_service_loads_existing_local_overlay() {
        let dir = tempfile::tempdir().unwrap();
        let local = TagRegistry {
            schema_version: CURRENT_SCHEMA_VERSION,
            updated_at: "".to_string(),
            tag_dimensions: vec![],
            type_definitions: HashMap::new(),
            albums: vec![],
            songs: vec![SongRegistryEntry {
                cid: "S1".to_string(),
                ..Default::default()
            }],
        };
        let path = dir.path().join("tag_registry_local.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&local).unwrap()).unwrap();
        let svc = TagEditorService::new(dir.path());
        assert!(svc.local_registry().songs.iter().any(|s| s.cid == "S1"));
    }
    #[test]
    fn set_entity_tag_writes_to_local_overlay() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        let values = vec![LocalizedValue({
            let mut m = HashMap::new();
            m.insert("zh-CN".to_string(), "罗德岛".to_string());
            m
        })];
        svc.set_entity_tag(EntityType::Album, "A1", "faction", values)
            .unwrap();
        let local = svc.local_registry();
        let a1 = find_album_in_registry(&local, "A1").unwrap();
        assert_eq!(lv_zh(&a1.faction), Some("罗德岛"));
    }

    #[test]
    fn remove_entity_tag_deletes_from_local_overlay() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        let values = vec![LocalizedValue({
            let mut m = HashMap::new();
            m.insert("zh-CN".to_string(), "罗德岛".to_string());
            m
        })];
        svc.set_entity_tag(EntityType::Album, "A1", "faction", values)
            .unwrap();
        svc.remove_entity_tag(EntityType::Album, "A1", "faction")
            .unwrap();
        let local = svc.local_registry();
        assert!(find_album_in_registry(&local, "A1").is_none());
    }

    #[test]
    fn set_entity_tag_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        let values = vec![LocalizedValue({
            let mut m = HashMap::new();
            m.insert("zh-CN".to_string(), "测试".to_string());
            m
        })];
        svc.set_entity_tag(EntityType::Song, "S1", "genre", values)
            .unwrap();
        let svc2 = TagEditorService::new(dir.path());
        let local = svc2.local_registry();
        let s1_tags = songs_to_tag_map(&local.songs);
        assert!(s1_tags.get("S1").unwrap().tags.contains_key("genre"));
    }
    #[test]
    fn add_local_dimension_appends_to_local_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        svc.add_local_dimension("mood", "心情", "Mood").unwrap();
        let local = svc.local_registry();
        let dim = local
            .tag_dimensions
            .iter()
            .find(|d| d.key == "mood")
            .unwrap();
        assert_eq!(dim.label.get("zh-CN").unwrap(), "心情");
        assert_eq!(dim.label.get("en-US").unwrap(), "Mood");
    }

    #[test]
    fn add_local_dimension_rejects_duplicate_key() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        svc.add_local_dimension("mood", "心情", "Mood").unwrap();
        let result = svc.add_local_dimension("mood", "情绪", "Emotion");
        assert!(result.is_err());
    }

    #[test]
    fn remove_local_dimension_cleans_up_references() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        svc.add_local_dimension("mood", "心情", "Mood").unwrap();
        let values = vec![LocalizedValue({
            let mut m = HashMap::new();
            m.insert("zh-CN".to_string(), "欢快".to_string());
            m
        })];
        svc.set_entity_tag(EntityType::Song, "S1", "mood", values)
            .unwrap();
        svc.remove_local_dimension("mood").unwrap();
        let local = svc.local_registry();
        assert!(local.tag_dimensions.iter().all(|d| d.key != "mood"));
        let s1_tags = songs_to_tag_map(&local.songs);
        assert!(s1_tags
            .get("S1")
            .is_none_or(|ts| !ts.tags.contains_key("mood")));
    }
    #[test]
    fn compute_merged_combines_remote_and_local() {
        let dir = tempfile::tempdir().unwrap();
        let remote = TagRegistry {
            schema_version: CURRENT_SCHEMA_VERSION,
            updated_at: "2026-05-01T00:00:00Z".to_string(),
            tag_dimensions: vec![TagDimension {
                key: "faction".to_string(),
                label: HashMap::from([("zh-CN".into(), "阵营".into())]),
                scope: None,
            }],
            type_definitions: HashMap::new(),
            albums: vec![AlbumEntry {
                cid: "A1".to_string(),
                faction: Some(make_lv("罗德岛")),
                ..Default::default()
            }],
            songs: Vec::new(),
        };
        std::fs::write(
            dir.path().join("tag_registry_remote.json"),
            serde_json::to_vec_pretty(&remote).unwrap(),
        )
        .unwrap();

        let svc = TagEditorService::new(dir.path());
        svc.add_local_dimension("mood", "心情", "Mood").unwrap();
        svc.set_entity_tag(
            EntityType::Album,
            "A1",
            "mood",
            vec![LocalizedValue(HashMap::from([(
                "zh-CN".into(),
                "热血".into(),
            )]))],
        )
        .unwrap();

        let merged = svc.compute_merged();
        assert_eq!(merged.tag_dimensions.len(), 2);
        assert_eq!(merged.tag_dimensions[0].key, "faction");
        assert_eq!(merged.tag_dimensions[1].key, "mood");
        let a1 = find_album_in_registry(&merged, "A1").unwrap();
        assert_eq!(lv_zh(&a1.faction), Some("罗德岛"));
    }

    #[test]
    fn compute_merged_deduplicates_same_dimension_values() {
        let dir = tempfile::tempdir().unwrap();
        let shared_value = LocalizedValue(HashMap::from([("zh-CN".into(), "罗德岛".into())]));
        let remote = TagRegistry {
            schema_version: CURRENT_SCHEMA_VERSION,
            updated_at: "".to_string(),
            tag_dimensions: vec![TagDimension {
                key: "faction".to_string(),
                label: HashMap::from([("zh-CN".into(), "阵营".into())]),
                scope: None,
            }],
            type_definitions: HashMap::new(),
            albums: vec![AlbumEntry {
                cid: "A1".to_string(),
                faction: Some(make_lv("罗德岛")),
                ..Default::default()
            }],
            songs: Vec::new(),
        };
        std::fs::write(
            dir.path().join("tag_registry_remote.json"),
            serde_json::to_vec_pretty(&remote).unwrap(),
        )
        .unwrap();

        let svc = TagEditorService::new(dir.path());
        svc.set_entity_tag(EntityType::Album, "A1", "faction", vec![shared_value])
            .unwrap();

        let merged = svc.compute_merged();
        let a1 = find_album_in_registry(&merged, "A1").unwrap();
        assert!(a1.faction.is_some(), "faction should exist after merge");
    }

    #[test]
    fn apply_remote_update_no_conflict_when_only_remote_changed() {
        let dir = tempfile::tempdir().unwrap();
        let base_remote = make_registry_with_album("A1", "旧值", "v1");
        std::fs::write(
            dir.path().join(REMOTE_FILE_NAME),
            serde_json::to_vec_pretty(&base_remote).unwrap(),
        )
        .unwrap();

        let svc = TagEditorService::new(dir.path());

        let new_remote = make_registry_with_album("A1", "新值", "v2");
        let result = svc.apply_remote_update(new_remote).unwrap();
        assert!(result.conflicts.is_empty());
        assert_eq!(svc.remote_registry().updated_at, "v2");
    }

    #[test]
    fn apply_remote_update_no_conflict_when_only_local_changed() {
        let dir = tempfile::tempdir().unwrap();
        let base_remote = make_registry_with_album("A1", "原值", "v1");
        std::fs::write(
            dir.path().join(REMOTE_FILE_NAME),
            serde_json::to_vec_pretty(&base_remote).unwrap(),
        )
        .unwrap();

        let svc = TagEditorService::new(dir.path());
        svc.set_entity_tag(
            EntityType::Album,
            "A1",
            "faction",
            vec![LocalizedValue(HashMap::from([(
                "zh-CN".into(),
                "本地值".into(),
            )]))],
        )
        .unwrap();

        let new_remote = base_remote.clone();
        let result = svc.apply_remote_update(new_remote).unwrap();
        assert!(result.conflicts.is_empty());
        let local = svc.local_registry();
        let a1 = find_album_in_registry(&local, "A1").unwrap();
        assert_eq!(lv_zh(&a1.faction), Some("本地值"));
    }

    #[test]
    fn apply_remote_update_detects_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let base_remote = make_registry_with_album("A1", "基线", "v1");
        std::fs::write(
            dir.path().join(REMOTE_FILE_NAME),
            serde_json::to_vec_pretty(&base_remote).unwrap(),
        )
        .unwrap();

        let svc = TagEditorService::new(dir.path());
        svc.set_entity_tag(
            EntityType::Album,
            "A1",
            "faction",
            vec![LocalizedValue(HashMap::from([(
                "zh-CN".into(),
                "本地改".into(),
            )]))],
        )
        .unwrap();

        let new_remote = make_registry_with_album("A1", "远端改", "v2");
        let result = svc.apply_remote_update(new_remote).unwrap();
        assert_eq!(result.conflicts.len(), 1);
        let c = &result.conflicts[0];
        assert_eq!(c.cid, "A1");
        assert_eq!(c.dimension_key, "faction");
    }

    #[test]
    fn resolve_conflict_keep_local_preserves_local_value() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        svc.set_entity_tag(
            EntityType::Album,
            "A1",
            "faction",
            vec![LocalizedValue(HashMap::from([(
                "zh-CN".into(),
                "本地".into(),
            )]))],
        )
        .unwrap();

        svc.resolve_conflict(
            EntityType::Album,
            "A1",
            "faction",
            ConflictResolution::KeepLocal,
        )
        .unwrap();

        let local = svc.local_registry();
        let a1 = find_album_in_registry(&local, "A1").unwrap();
        assert_eq!(lv_zh(&a1.faction), Some("本地"));
    }

    #[test]
    fn resolve_conflict_keep_remote_removes_local_override() {
        let dir = tempfile::tempdir().unwrap();
        let svc = TagEditorService::new(dir.path());
        svc.set_entity_tag(
            EntityType::Album,
            "A1",
            "faction",
            vec![LocalizedValue(HashMap::from([(
                "zh-CN".into(),
                "本地".into(),
            )]))],
        )
        .unwrap();

        svc.resolve_conflict(
            EntityType::Album,
            "A1",
            "faction",
            ConflictResolution::KeepRemote,
        )
        .unwrap();

        let local = svc.local_registry();
        assert!(find_album_in_registry(&local, "A1").is_none());
    }
}
