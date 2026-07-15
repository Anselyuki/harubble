mod index;
mod schema;
mod scoring;
mod service;
mod snapshot;

use harubble_core::LibraryIndexState;
use tauri::{AppHandle, Emitter};

pub(crate) use service::LibrarySearchService;
pub(crate) use snapshot::build_snapshot_records_for_album;

pub(crate) const LIBRARY_SEARCH_INDEX_STATE_CHANGED: &str = "library-search-index-state-changed";

pub(crate) fn emit_library_search_index_state_changed(app: &AppHandle, state: LibraryIndexState) {
    let _ = app.emit(LIBRARY_SEARCH_INDEX_STATE_CHANGED, state);
}
