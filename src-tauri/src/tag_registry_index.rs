use crate::preferences::Locale;
use crate::tag_registry::{AlbumEntry, LocalizedValue, SongRegistryEntry, TagDimension, TagSet};
use harubble_core::api::TagEntry;
use std::collections::HashMap;

pub(crate) type AlbumTagIndex = HashMap<String, TagSet>;
pub(crate) type SongTagIndex = HashMap<String, TagSet>;

pub(super) fn build_album_index(
    albums: &[AlbumEntry],
    type_defs: &HashMap<String, LocalizedValue>,
) -> AlbumTagIndex {
    albums
        .iter()
        .filter_map(|entry| {
            let tag_set = album_entry_to_tag_set(entry, type_defs);
            if tag_set.tags.is_empty() {
                None
            } else {
                Some((entry.cid.clone(), tag_set))
            }
        })
        .collect()
}

pub(super) fn build_song_index(songs: &[SongRegistryEntry]) -> SongTagIndex {
    songs
        .iter()
        .filter_map(|entry| {
            let tag_set = song_entry_to_tag_set(entry);
            if tag_set.tags.is_empty() {
                None
            } else {
                Some((entry.cid.clone(), tag_set))
            }
        })
        .collect()
}

fn album_entry_to_tag_set(
    entry: &AlbumEntry,
    type_defs: &HashMap<String, LocalizedValue>,
) -> TagSet {
    let mut tags: HashMap<String, Vec<LocalizedValue>> = HashMap::new();

    if let Some(ref key) = entry.album_type {
        if let Some(lv) = type_defs.get(key) {
            tags.insert("type".to_string(), vec![lv.clone()]);
        } else {
            let fallback = LocalizedValue(HashMap::from([
                ("zh-CN".to_string(), key.clone()),
                ("en-US".to_string(), key.clone()),
            ]));
            tags.insert("type".to_string(), vec![fallback]);
        }
    }
    if let Some(ref v) = entry.faction {
        tags.insert("faction".to_string(), vec![v.clone()]);
    }
    if let Some(ref v) = entry.character {
        tags.insert("character".to_string(), vec![v.clone()]);
    }

    TagSet { tags }
}

fn song_entry_to_tag_set(entry: &SongRegistryEntry) -> TagSet {
    let mut tags: HashMap<String, Vec<LocalizedValue>> = HashMap::new();

    if let Some(ref v) = entry.faction {
        tags.insert("faction".to_string(), vec![v.clone()]);
    }
    if let Some(ref v) = entry.character {
        tags.insert("character".to_string(), vec![v.clone()]);
    }
    for (key, values) in &entry.extra {
        if !values.is_empty() {
            tags.insert(key.clone(), values.clone());
        }
    }

    TagSet { tags }
}

pub(crate) fn albums_to_tag_map(
    albums: &[AlbumEntry],
    type_defs: &HashMap<String, LocalizedValue>,
) -> HashMap<String, TagSet> {
    build_album_index(albums, type_defs)
}

pub(crate) fn tag_map_to_albums(
    map: &HashMap<String, TagSet>,
    type_defs: &HashMap<String, LocalizedValue>,
) -> Vec<AlbumEntry> {
    map.iter()
        .map(|(cid, tag_set)| tag_set_to_album_entry(cid, tag_set, type_defs))
        .collect()
}

pub(crate) fn songs_to_tag_map(songs: &[SongRegistryEntry]) -> HashMap<String, TagSet> {
    songs
        .iter()
        .map(|entry| (entry.cid.clone(), song_entry_to_tag_set(entry)))
        .collect()
}

pub(crate) fn tag_map_to_songs(map: &HashMap<String, TagSet>) -> Vec<SongRegistryEntry> {
    map.iter()
        .map(|(cid, tag_set)| tag_set_to_song_entry(cid, tag_set))
        .collect()
}

fn tag_set_to_song_entry(cid: &str, tag_set: &TagSet) -> SongRegistryEntry {
    let get_first_lv = |key: &str| -> Option<LocalizedValue> {
        tag_set.tags.get(key).and_then(|vals| vals.first().cloned())
    };

    let extra: HashMap<String, Vec<LocalizedValue>> = tag_set
        .tags
        .iter()
        .filter(|(k, _)| k.as_str() != "faction" && k.as_str() != "character")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    SongRegistryEntry {
        cid: cid.to_string(),
        faction: get_first_lv("faction"),
        character: get_first_lv("character"),
        extra,
    }
}

fn tag_set_to_album_entry(
    cid: &str,
    tag_set: &TagSet,
    type_defs: &HashMap<String, LocalizedValue>,
) -> AlbumEntry {
    let get_first_lv = |key: &str| -> Option<LocalizedValue> {
        tag_set.tags.get(key).and_then(|vals| vals.first().cloned())
    };

    let get_first_str = |key: &str| -> Option<String> {
        tag_set.tags.get(key).and_then(|vals| {
            vals.first().map(|lv| {
                lv.0.get("zh-CN")
                    .or_else(|| lv.0.get("en-US"))
                    .or_else(|| lv.0.values().next())
                    .cloned()
                    .unwrap_or_default()
            })
        })
    };

    let album_type = tag_set.tags.get("type").and_then(|vals| {
        vals.first().and_then(|lv| {
            type_defs
                .iter()
                .find(|(_, def)| *def == lv)
                .map(|(k, _)| k.clone())
                .or_else(|| {
                    lv.0.get("en-US")
                        .or_else(|| lv.0.get("zh-CN"))
                        .or_else(|| lv.0.values().next())
                        .cloned()
                })
        })
    });

    AlbumEntry {
        cid: cid.to_string(),
        album_type,
        name: get_first_str("name"),
        release_date: get_first_str("releaseDate"),
        faction: get_first_lv("faction"),
        character: get_first_lv("character"),
    }
}

pub(super) fn resolve_tag_set(
    tag_set: Option<&TagSet>,
    dimensions: &[TagDimension],
    locale: Locale,
) -> Vec<TagEntry> {
    let Some(tag_set) = tag_set else {
        return Vec::new();
    };
    build_tag_entries(&tag_set.tags, dimensions, locale)
}

pub(super) fn resolve_merged_tag_set(
    album: Option<&TagSet>,
    song: Option<&TagSet>,
    dims: &[TagDimension],
    locale: Locale,
) -> Vec<TagEntry> {
    let mut merged: HashMap<String, Vec<LocalizedValue>> = HashMap::new();

    for set_opt in [album, song] {
        let Some(set) = set_opt else { continue };
        for (dim_key, values) in &set.tags {
            let entry = merged.entry(dim_key.clone()).or_default();
            for v in values {
                if let Some(existing) = entry.iter_mut().find(|e| e.text_eq(v)) {
                    *existing = LocalizedValue::merge_metadata(existing, v);
                } else {
                    entry.push(v.clone());
                }
            }
        }
    }

    build_tag_entries(&merged, dims, locale)
}

/// PLACEHOLDER_BUILD_TAG_ENTRIES

fn build_tag_entries(
    tags: &HashMap<String, Vec<LocalizedValue>>,
    dims: &[TagDimension],
    locale: Locale,
) -> Vec<TagEntry> {
    let mut result = Vec::new();
    for dim in dims {
        let Some(values) = tags.get(&dim.key) else {
            continue;
        };
        let resolved_values: Vec<String> = values
            .iter()
            .map(|lv| resolve_localized_value(&lv.0, locale))
            .collect();
        if resolved_values.is_empty() {
            continue;
        }
        let colors: Vec<Option<String>> =
            values.iter().map(|lv| lv.0.get("color").cloned()).collect();
        let colors = if colors.iter().any(|c| c.is_some()) {
            colors
        } else {
            Vec::new()
        };
        result.push(TagEntry {
            dimension: resolve_locale_str(&dim.label, locale),
            values: resolved_values,
            colors,
        });
    }
    result
}

pub(super) fn collect_all_locale_values(album: Option<&TagSet>, song: Option<&TagSet>) -> String {
    let mut all_values: Vec<String> = Vec::new();
    for set_opt in [album, song] {
        let Some(set) = set_opt else { continue };
        for values in set.tags.values() {
            for lv in values {
                for (k, v) in &lv.0 {
                    if is_locale_key(k) && !v.is_empty() {
                        all_values.push(v.clone());
                    }
                }
            }
        }
    }
    all_values.join(" ")
}

pub(super) fn locale_to_key(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCN => "zh-CN",
        Locale::EnUS => "en-US",
    }
}

pub(super) fn is_locale_key(key: &str) -> bool {
    matches!(key, "zh-CN" | "en-US" | "ja-JP" | "ko-KR")
}

pub(super) fn resolve_locale_str(map: &HashMap<String, String>, locale: Locale) -> String {
    let key = locale_to_key(locale);
    if let Some(v) = map.get(key) {
        return v.clone();
    }
    if let Some(v) = map.get("zh-CN") {
        return v.clone();
    }
    if let Some(v) = map.get("en-US") {
        return v.clone();
    }
    map.iter()
        .find(|(k, _)| is_locale_key(k))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

pub(super) fn resolve_localized_value(map: &HashMap<String, String>, locale: Locale) -> String {
    resolve_locale_str(map, locale)
}
