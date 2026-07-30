//! Compile-time registry for theme packages shipped with Harubble.
//!
//! Built-in packages live in the frontend source tree so the same canonical JSON
//! documents can be exercised by Vitest and embedded into the Tauri binary. They
//! never enter the mutable `PackageStore`; callers distinguish them through
//! `ThemePackageSummary::builtin`.

use crate::theme_packages::sanitizer::sanitize_document;
use crate::theme_packages::types::ThemePackageDocument;
use std::collections::BTreeMap;

pub(crate) const BUILTIN_THEME_PACKAGE_IDS: &[&str] = &[
    "ark-ui-ark",
    "ark-ui-corporate",
    "ark-ui-endfield",
    "ark-ui-exa",
    "ark-ui-popucom",
];

const BUILTIN_THEME_PACKAGE_SOURCES: &[(&str, &str)] = &[
    (
        "ark-ui-ark",
        include_str!("../../../src/lib/theme-packages/builtins/ark-ui-ark.json"),
    ),
    (
        "ark-ui-corporate",
        include_str!("../../../src/lib/theme-packages/builtins/ark-ui-corporate.json"),
    ),
    (
        "ark-ui-endfield",
        include_str!("../../../src/lib/theme-packages/builtins/ark-ui-endfield.json"),
    ),
    (
        "ark-ui-exa",
        include_str!("../../../src/lib/theme-packages/builtins/ark-ui-exa.json"),
    ),
    (
        "ark-ui-popucom",
        include_str!("../../../src/lib/theme-packages/builtins/ark-ui-popucom.json"),
    ),
];

pub(crate) fn load_builtin_theme_packages() -> Result<BTreeMap<String, ThemePackageDocument>, String>
{
    let mut packages = BTreeMap::new();
    for (expected_id, source) in BUILTIN_THEME_PACKAGE_SOURCES {
        let mut document: ThemePackageDocument = serde_json::from_str(source)
            .map_err(|error| format!("invalid built-in theme package {expected_id}: {error}"))?;
        if document.manifest.id != *expected_id {
            return Err(format!(
                "built-in theme package id mismatch: registry={expected_id}, manifest={}",
                document.manifest.id
            ));
        }
        sanitize_document(&mut document)
            .map_err(|error| format!("invalid built-in theme package {expected_id}: {error}"))?;
        if !document.warnings.is_empty() {
            return Err(format!(
                "built-in theme package {expected_id} required sanitization: {}",
                document.warnings.join("; ")
            ));
        }
        if packages
            .insert((*expected_id).to_string(), document)
            .is_some()
        {
            return Err(format!(
                "duplicate built-in theme package id: {expected_id}"
            ));
        }
    }
    Ok(packages)
}

pub(crate) fn builtin_theme_package_source(id: &str) -> Option<&'static str> {
    BUILTIN_THEME_PACKAGE_SOURCES
        .iter()
        .find_map(|(candidate, source)| (*candidate == id).then_some(*source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads_all_canonical_packages_without_warnings() {
        let packages = load_builtin_theme_packages().expect("load built-in theme packages");
        let ids = packages.keys().map(String::as_str).collect::<Vec<_>>();

        assert_eq!(ids, BUILTIN_THEME_PACKAGE_IDS);
        assert!(packages.values().all(|package| package.warnings.is_empty()));
        assert!(packages.values().all(|package| package.slots.len() == 6));
    }

    #[test]
    fn registry_sources_match_manifest_ids() {
        for id in BUILTIN_THEME_PACKAGE_IDS {
            let source = builtin_theme_package_source(id).expect("registered source");
            let document: ThemePackageDocument =
                serde_json::from_str(source).expect("parse built-in source");
            assert_eq!(document.manifest.id, *id);
        }
    }
}
