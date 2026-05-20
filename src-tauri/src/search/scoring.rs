use std::cmp::Ordering;

use harubble_core::{LibrarySearchHitField, SearchLibraryResultItem, SearchLibraryResultKind};

use super::index::LibrarySearchDocument;

fn normalize_query_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn compact_query_text(value: &str) -> String {
    normalize_query_text(value).replace(' ', "")
}

pub(super) fn compact_ascii_query(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

pub(super) fn normalize_query(value: &str) -> String {
    normalize_query_text(value)
}

pub(super) fn escape_query_text(value: &str) -> String {
    const RESERVED: [char; 16] = [
        '\\', '+', '-', '&', '|', '!', '(', ')', '{', '}', '[', ']', '^', '"', '~', ':',
    ];

    value
        .chars()
        .flat_map(|character| {
            if RESERVED.contains(&character) {
                vec!['\\', character]
            } else {
                vec![character]
            }
        })
        .collect()
}

fn exact_normalized_match(field_value: &str, normalized_query: &str) -> bool {
    !normalized_query.is_empty()
        && compact_query_text(field_value) == compact_query_text(normalized_query)
}

fn prefix_normalized_match(field_value: &str, normalized_query: &str) -> bool {
    !normalized_query.is_empty()
        && compact_query_text(field_value).starts_with(&compact_query_text(normalized_query))
}

fn contains_normalized_match(field_value: &str, normalized_query: &str) -> bool {
    !normalized_query.is_empty()
        && compact_query_text(field_value).contains(&compact_query_text(normalized_query))
}

fn exact_compact_match(field_value: Option<&str>, compact_query: &str) -> bool {
    matches!(field_value, Some(value) if !compact_query.is_empty() && compact_query_text(value) == compact_query)
}

fn prefix_compact_match(field_value: Option<&str>, compact_query: &str) -> bool {
    matches!(field_value, Some(value) if !compact_query.is_empty() && compact_query_text(value).starts_with(compact_query))
}

fn contains_compact_match(field_value: Option<&str>, compact_query: &str) -> bool {
    matches!(field_value, Some(value) if !compact_query.is_empty() && compact_query_text(value).contains(compact_query))
}

fn score_text_match(
    field_value: Option<&str>,
    normalized_query: &str,
    exact: i64,
    prefix: i64,
    contains: i64,
) -> i64 {
    match field_value {
        Some(value) if exact_normalized_match(value, normalized_query) => exact,
        Some(value) if prefix_normalized_match(value, normalized_query) => prefix,
        Some(value) if contains_normalized_match(value, normalized_query) => contains,
        _ => 0,
    }
}

#[allow(clippy::too_many_arguments)]
fn score_compact_match(
    full_value: Option<&str>,
    initials_value: Option<&str>,
    compact_query: &str,
    full_exact: i64,
    full_prefix: i64,
    full_contains: i64,
    initials_exact: i64,
    initials_prefix: i64,
    initials_contains: i64,
) -> i64 {
    if exact_compact_match(full_value, compact_query) {
        return full_exact;
    }
    if prefix_compact_match(full_value, compact_query) {
        return full_prefix;
    }
    if contains_compact_match(full_value, compact_query) {
        return full_contains;
    }
    if exact_compact_match(initials_value, compact_query) {
        return initials_exact;
    }
    if prefix_compact_match(initials_value, compact_query) {
        return initials_prefix;
    }
    if contains_compact_match(initials_value, compact_query) {
        return initials_contains;
    }
    0
}

/// PLACEHOLDER_REMAINING_SCORING

pub(super) fn rank_search_document(
    document: &LibrarySearchDocument,
    normalized_query: &str,
    compact_query: &str,
) -> i64 {
    let (title_text_score, title_pinyin_score) = match document.kind {
        SearchLibraryResultKind::Song => (
            score_text_match(
                document.song_title.as_deref(),
                normalized_query,
                4_200,
                3_800,
                3_400,
            ),
            score_compact_match(
                document.song_title_pinyin_full.as_deref(),
                document.song_title_pinyin_initials.as_deref(),
                compact_query,
                2_500,
                2_300,
                2_100,
                1_950,
                1_750,
                1_550,
            ),
        ),
        SearchLibraryResultKind::Album => (
            score_text_match(
                Some(&document.album_title),
                normalized_query,
                4_000,
                3_600,
                3_200,
            ),
            score_compact_match(
                document.album_title_pinyin_full.as_deref(),
                document.album_title_pinyin_initials.as_deref(),
                compact_query,
                2_400,
                2_200,
                2_000,
                1_900,
                1_700,
                1_500,
            ),
        ),
    };

    let artist_text_score = score_text_match(
        document.artist_line.as_deref(),
        normalized_query,
        1_600,
        1_450,
        1_300,
    );
    let artist_pinyin_score = score_compact_match(
        document.artist_line_pinyin_full.as_deref(),
        document.artist_line_pinyin_initials.as_deref(),
        compact_query,
        1_250,
        1_150,
        1_050,
        980,
        920,
        860,
    );
    let belong_text_score =
        score_text_match(document.belong.as_deref(), normalized_query, 820, 760, 700);
    let belong_pinyin_score = score_compact_match(
        document.belong_pinyin_full.as_deref(),
        document.belong_pinyin_initials.as_deref(),
        compact_query,
        720,
        660,
        620,
        560,
        520,
        480,
    );
    let intro_text_score =
        score_text_match(document.intro.as_deref(), normalized_query, 420, 360, 320);
    let tag_text_score = score_text_match(
        document.tag_values.as_deref(),
        normalized_query,
        620,
        560,
        500,
    );
    let tag_pinyin_score = score_compact_match(
        document.tag_values_pinyin_full.as_deref(),
        document.tag_values_pinyin_initials.as_deref(),
        compact_query,
        520,
        480,
        440,
        400,
        360,
        320,
    );
    let kind_bias = match document.kind {
        SearchLibraryResultKind::Song => 40,
        SearchLibraryResultKind::Album => 0,
    };

    title_text_score
        + title_pinyin_score
        + artist_text_score
        + artist_pinyin_score
        + belong_text_score
        + belong_pinyin_score
        + intro_text_score
        + tag_text_score
        + tag_pinyin_score
        + kind_bias
}

/// PLACEHOLDER_COLLECT_AND_COMPARE

pub(super) fn collect_matched_fields(
    document: &LibrarySearchDocument,
    normalized_query: &str,
    compact_query: &str,
) -> Vec<LibrarySearchHitField> {
    let mut matched_fields = Vec::new();

    let title_text_matched = match document.kind {
        SearchLibraryResultKind::Album => {
            contains_normalized_match(&document.album_title, normalized_query)
        }
        SearchLibraryResultKind::Song => document
            .song_title
            .as_deref()
            .is_some_and(|value| contains_normalized_match(value, normalized_query)),
    };
    let title_pinyin_matched = match document.kind {
        SearchLibraryResultKind::Album => {
            contains_compact_match(document.album_title_pinyin_full.as_deref(), compact_query)
                || contains_compact_match(
                    document.album_title_pinyin_initials.as_deref(),
                    compact_query,
                )
        }
        SearchLibraryResultKind::Song => {
            contains_compact_match(document.song_title_pinyin_full.as_deref(), compact_query)
                || contains_compact_match(
                    document.song_title_pinyin_initials.as_deref(),
                    compact_query,
                )
        }
    };
    if title_text_matched || title_pinyin_matched {
        matched_fields.push(LibrarySearchHitField::Title);
    }

    let artist_text_matched = document
        .artist_line
        .as_deref()
        .is_some_and(|value| contains_normalized_match(value, normalized_query));
    let artist_pinyin_matched =
        contains_compact_match(document.artist_line_pinyin_full.as_deref(), compact_query)
            || contains_compact_match(
                document.artist_line_pinyin_initials.as_deref(),
                compact_query,
            );
    if artist_text_matched || artist_pinyin_matched {
        matched_fields.push(LibrarySearchHitField::Artist);
    }

    if document
        .intro
        .as_deref()
        .is_some_and(|value| contains_normalized_match(value, normalized_query))
    {
        matched_fields.push(LibrarySearchHitField::Intro);
    }

    let belong_text_matched = document
        .belong
        .as_deref()
        .is_some_and(|value| contains_normalized_match(value, normalized_query));
    let belong_pinyin_matched =
        contains_compact_match(document.belong_pinyin_full.as_deref(), compact_query)
            || contains_compact_match(document.belong_pinyin_initials.as_deref(), compact_query);
    if belong_text_matched || belong_pinyin_matched {
        matched_fields.push(LibrarySearchHitField::Belong);
    }

    let tag_text_matched = document
        .tag_values
        .as_deref()
        .is_some_and(|value| contains_normalized_match(value, normalized_query));
    let tag_pinyin_matched =
        contains_compact_match(document.tag_values_pinyin_full.as_deref(), compact_query)
            || contains_compact_match(
                document.tag_values_pinyin_initials.as_deref(),
                compact_query,
            );
    if tag_text_matched || tag_pinyin_matched {
        matched_fields.push(LibrarySearchHitField::TagValues);
    }

    matched_fields
}

pub(super) fn compare_scored_items(left: &ScoredSearchItem, right: &ScoredSearchItem) -> Ordering {
    right
        .rank_score
        .cmp(&left.rank_score)
        .then_with(|| compare_result_items(&left.item, &right.item))
}

fn compare_result_items(
    left: &SearchLibraryResultItem,
    right: &SearchLibraryResultItem,
) -> Ordering {
    let left_title = left.song_title.as_ref().unwrap_or(&left.album_title);
    let right_title = right.song_title.as_ref().unwrap_or(&right.album_title);
    left_title
        .cmp(right_title)
        .then_with(|| left.album_title.cmp(&right.album_title))
        .then_with(|| left.album_cid.cmp(&right.album_cid))
        .then_with(|| left.song_cid.cmp(&right.song_cid))
}

#[derive(Debug)]
pub(super) struct ScoredSearchItem {
    pub rank_score: i64,
    pub item: SearchLibraryResultItem,
}
