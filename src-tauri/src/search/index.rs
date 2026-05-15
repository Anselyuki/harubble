use crate::preferences::Locale;
use crate::search::snapshot::{inventory_index_dir, LibrarySearchSnapshot};
use anyhow::{Context, Result};
use harubble_core::{
    LibrarySearchHitField, LibrarySearchScope, SearchLibraryRequest, SearchLibraryResultItem,
    SearchLibraryResultKind,
};
use std::cmp::Ordering;
use std::path::Path;
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, TextFieldIndexing, TextOptions, Value,
    STORED, STRING,
};
use tantivy::tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer};
use tantivy::{doc, Index, IndexReader, ReloadPolicy, Term};

const SEARCH_TOKENIZER_NAME: &str = "siren_ngram";

#[derive(Clone)]
pub(crate) struct LibrarySearchIndex {
    index: Index,
    reader: IndexReader,
    fields: LibrarySearchFields,
}

#[derive(Clone, Copy)]
struct LibrarySearchFields {
    kind: Field,
    album_cid: Field,
    song_cid: Field,
    album_title: Field,
    album_title_display: Field,
    song_title: Field,
    artist_line: Field,
    intro: Field,
    belong: Field,
    album_title_pinyin_full: Field,
    album_title_pinyin_initials: Field,
    song_title_pinyin_full: Field,
    song_title_pinyin_initials: Field,
    artist_line_pinyin_full: Field,
    artist_line_pinyin_initials: Field,
    belong_pinyin_full: Field,
    belong_pinyin_initials: Field,
    tag_values: Field,
    tag_values_pinyin_full: Field,
    tag_values_pinyin_initials: Field,
}

#[derive(Clone)]
pub(crate) struct SanitizedSearchRequest {
    pub query: String,
    pub scope: LibrarySearchScope,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
struct LibrarySearchDocument {
    kind: SearchLibraryResultKind,
    album_cid: String,
    song_cid: Option<String>,
    album_title: String,
    song_title: Option<String>,
    artist_line: Option<String>,
    intro: Option<String>,
    belong: Option<String>,
    album_title_pinyin_full: Option<String>,
    album_title_pinyin_initials: Option<String>,
    song_title_pinyin_full: Option<String>,
    song_title_pinyin_initials: Option<String>,
    artist_line_pinyin_full: Option<String>,
    artist_line_pinyin_initials: Option<String>,
    belong_pinyin_full: Option<String>,
    belong_pinyin_initials: Option<String>,
    tag_values: Option<String>,
    tag_values_pinyin_full: Option<String>,
    tag_values_pinyin_initials: Option<String>,
}

#[derive(Debug)]
struct ScoredSearchItem {
    rank_score: i64,
    item: SearchLibraryResultItem,
}

impl LibrarySearchIndex {
    pub(crate) fn build(base_dir: &Path, snapshot: &LibrarySearchSnapshot) -> Result<Self> {
        let index_dir = inventory_index_dir(base_dir, &snapshot.inventory_version);
        if index_dir.exists() {
            std::fs::remove_dir_all(&index_dir)
                .with_context(|| format!("failed to clean {}", index_dir.display()))?;
        }
        std::fs::create_dir_all(&index_dir)
            .with_context(|| format!("failed to create {}", index_dir.display()))?;

        let (schema, fields) = build_schema();
        let index = Index::create_in_dir(&index_dir, schema)?;
        register_tokenizers(&index)?;

        let mut writer = index.writer(50_000_000)?;

        for album in &snapshot.albums {
            let _ = writer.add_document(doc!(
                fields.kind => "album",
                fields.album_cid => album.album_cid.clone(),
                fields.song_cid => "",
                fields.album_title => album.album_title.clone(),
                fields.album_title_display => album.album_title.clone(),
                fields.song_title => "",
                fields.artist_line => album.artist_line.clone().unwrap_or_default(),
                fields.intro => album.intro.clone().unwrap_or_default(),
                fields.belong => album.belong.clone().unwrap_or_default(),
                fields.album_title_pinyin_full => album.album_title_pinyin_full.clone().unwrap_or_default(),
                fields.album_title_pinyin_initials => album.album_title_pinyin_initials.clone().unwrap_or_default(),
                fields.song_title_pinyin_full => "",
                fields.song_title_pinyin_initials => "",
                fields.artist_line_pinyin_full => album.artist_line_pinyin_full.clone().unwrap_or_default(),
                fields.artist_line_pinyin_initials => album.artist_line_pinyin_initials.clone().unwrap_or_default(),
                fields.belong_pinyin_full => album.belong_pinyin_full.clone().unwrap_or_default(),
                fields.belong_pinyin_initials => album.belong_pinyin_initials.clone().unwrap_or_default(),
                fields.tag_values => album.tag_values.clone().unwrap_or_default(),
                fields.tag_values_pinyin_full => album.tag_values_pinyin_full.clone().unwrap_or_default(),
                fields.tag_values_pinyin_initials => album.tag_values_pinyin_initials.clone().unwrap_or_default(),
            ));
        }

        for song in &snapshot.songs {
            let _ = writer.add_document(doc!(
                fields.kind => "song",
                fields.album_cid => song.album_cid.clone(),
                fields.song_cid => song.song_cid.clone(),
                fields.album_title => "",
                fields.album_title_display => song.album_title.clone(),
                fields.song_title => song.song_title.clone(),
                fields.artist_line => song.artist_line.clone().unwrap_or_default(),
                fields.intro => "",
                fields.belong => "",
                fields.album_title_pinyin_full => "",
                fields.album_title_pinyin_initials => "",
                fields.song_title_pinyin_full => song.song_title_pinyin_full.clone().unwrap_or_default(),
                fields.song_title_pinyin_initials => song.song_title_pinyin_initials.clone().unwrap_or_default(),
                fields.artist_line_pinyin_full => song.artist_line_pinyin_full.clone().unwrap_or_default(),
                fields.artist_line_pinyin_initials => song.artist_line_pinyin_initials.clone().unwrap_or_default(),
                fields.belong_pinyin_full => "",
                fields.belong_pinyin_initials => "",
                fields.tag_values => song.tag_values.clone().unwrap_or_default(),
                fields.tag_values_pinyin_full => song.tag_values_pinyin_full.clone().unwrap_or_default(),
                fields.tag_values_pinyin_initials => song.tag_values_pinyin_initials.clone().unwrap_or_default(),
            ));
        }

        writer.commit()?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        reader.reload()?;

        Ok(Self {
            index,
            reader,
            fields,
        })
    }

    pub(crate) fn open(base_dir: &Path, inventory_version: &str) -> Result<Self> {
        let index_dir = inventory_index_dir(base_dir, inventory_version);
        let index = Index::open_in_dir(&index_dir)
            .with_context(|| format!("failed to open {}", index_dir.display()))?;
        register_tokenizers(&index)?;
        let fields = load_fields(index.schema())?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        reader.reload()?;
        Ok(Self {
            index,
            reader,
            fields,
        })
    }

    pub(crate) fn search(
        &self,
        request: &SanitizedSearchRequest,
    ) -> Result<(Vec<SearchLibraryResultItem>, usize)> {
        let searcher = self.reader.searcher();
        let query = self.build_search_query(request)?;
        let total = searcher.search(query.as_ref(), &Count)?;
        let fetch_limit = total.max(request.offset.saturating_add(request.limit));
        let top_docs = if fetch_limit == 0 {
            Vec::new()
        } else {
            searcher.search(query.as_ref(), &TopDocs::with_limit(fetch_limit))?
        };

        let normalized_query = normalize_query_text(&request.query);
        let compact_query = compact_ascii_query(&request.query);
        let mut items = top_docs
            .into_iter()
            .map(|(_text_score, address)| {
                let document = searcher.doc::<TantivyDocument>(address)?;
                let search_document = self.document_to_search_document(&document);
                let item = SearchLibraryResultItem {
                    kind: search_document.kind,
                    album_cid: search_document.album_cid.clone(),
                    song_cid: search_document.song_cid.clone(),
                    album_title: search_document.album_title.clone(),
                    song_title: search_document.song_title.clone(),
                    artist_line: search_document.artist_line.clone(),
                    matched_fields: collect_matched_fields(
                        &search_document,
                        &normalized_query,
                        &compact_query,
                    ),
                };
                Ok(ScoredSearchItem {
                    rank_score: rank_search_document(
                        &search_document,
                        &normalized_query,
                        &compact_query,
                    ),
                    item,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        items.sort_by(compare_scored_items);

        let paged_items = items
            .into_iter()
            .skip(request.offset)
            .take(request.limit)
            .map(|item| item.item)
            .collect::<Vec<_>>();

        Ok((paged_items, total))
    }

    fn build_search_query(&self, request: &SanitizedSearchRequest) -> Result<Box<dyn Query>> {
        let mut recall_clauses: Vec<(Occur, Box<dyn Query>)> =
            vec![(Occur::Should, self.build_text_query(request)?)];

        let compact_query = compact_ascii_query(&request.query);
        if !compact_query.is_empty() {
            recall_clauses.push((Occur::Should, self.build_pinyin_query(&compact_query)?));
        }

        let recall_query: Box<dyn Query> = if recall_clauses.len() == 1 {
            recall_clauses.remove(0).1
        } else {
            Box::new(BooleanQuery::new(recall_clauses))
        };

        if let Some(kind_value) = scope_kind_value(request.scope) {
            return Ok(Box::new(BooleanQuery::new(vec![
                (Occur::Must, recall_query),
                (
                    Occur::Must,
                    Box::new(TermQuery::new(
                        Term::from_field_text(self.fields.kind, kind_value),
                        IndexRecordOption::Basic,
                    )),
                ),
            ])));
        }

        Ok(recall_query)
    }

    fn build_text_query(&self, request: &SanitizedSearchRequest) -> Result<Box<dyn Query>> {
        let mut text_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.album_title,
                self.fields.song_title,
                self.fields.artist_line,
                self.fields.intro,
                self.fields.belong,
                self.fields.tag_values,
            ],
        );
        text_parser.set_field_boost(self.fields.song_title, 6.0);
        text_parser.set_field_boost(self.fields.album_title, 5.0);
        text_parser.set_field_boost(self.fields.artist_line, 2.0);
        text_parser.set_field_boost(self.fields.belong, 1.1);
        text_parser.set_field_boost(self.fields.intro, 0.8);
        text_parser.set_field_boost(self.fields.tag_values, 1.0);
        Ok(text_parser.parse_query(&escape_query_text(&request.query))?)
    }

    fn build_pinyin_query(&self, compact_query: &str) -> Result<Box<dyn Query>> {
        let mut pinyin_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.album_title_pinyin_full,
                self.fields.album_title_pinyin_initials,
                self.fields.song_title_pinyin_full,
                self.fields.song_title_pinyin_initials,
                self.fields.artist_line_pinyin_full,
                self.fields.artist_line_pinyin_initials,
                self.fields.belong_pinyin_full,
                self.fields.belong_pinyin_initials,
                self.fields.tag_values_pinyin_full,
                self.fields.tag_values_pinyin_initials,
            ],
        );
        pinyin_parser.set_field_boost(self.fields.song_title_pinyin_full, 2.8);
        pinyin_parser.set_field_boost(self.fields.album_title_pinyin_full, 2.5);
        pinyin_parser.set_field_boost(self.fields.artist_line_pinyin_full, 1.5);
        pinyin_parser.set_field_boost(self.fields.belong_pinyin_full, 0.9);
        pinyin_parser.set_field_boost(self.fields.song_title_pinyin_initials, 1.8);
        pinyin_parser.set_field_boost(self.fields.album_title_pinyin_initials, 1.6);
        pinyin_parser.set_field_boost(self.fields.artist_line_pinyin_initials, 1.0);
        pinyin_parser.set_field_boost(self.fields.belong_pinyin_initials, 0.7);
        pinyin_parser.set_field_boost(self.fields.tag_values_pinyin_full, 0.9);
        pinyin_parser.set_field_boost(self.fields.tag_values_pinyin_initials, 0.7);
        Ok(pinyin_parser.parse_query(&escape_query_text(compact_query))?)
    }

    fn document_to_search_document(&self, document: &TantivyDocument) -> LibrarySearchDocument {
        let kind = field_text(document, self.fields.kind);
        LibrarySearchDocument {
            kind: if kind == "song" {
                SearchLibraryResultKind::Song
            } else {
                SearchLibraryResultKind::Album
            },
            album_cid: field_text(document, self.fields.album_cid),
            song_cid: empty_to_none(field_text(document, self.fields.song_cid)),
            album_title: field_text(document, self.fields.album_title_display),
            song_title: empty_to_none(field_text(document, self.fields.song_title)),
            artist_line: empty_to_none(field_text(document, self.fields.artist_line)),
            intro: empty_to_none(field_text(document, self.fields.intro)),
            belong: empty_to_none(field_text(document, self.fields.belong)),
            album_title_pinyin_full: empty_to_none(field_text(
                document,
                self.fields.album_title_pinyin_full,
            )),
            album_title_pinyin_initials: empty_to_none(field_text(
                document,
                self.fields.album_title_pinyin_initials,
            )),
            song_title_pinyin_full: empty_to_none(field_text(
                document,
                self.fields.song_title_pinyin_full,
            )),
            song_title_pinyin_initials: empty_to_none(field_text(
                document,
                self.fields.song_title_pinyin_initials,
            )),
            artist_line_pinyin_full: empty_to_none(field_text(
                document,
                self.fields.artist_line_pinyin_full,
            )),
            artist_line_pinyin_initials: empty_to_none(field_text(
                document,
                self.fields.artist_line_pinyin_initials,
            )),
            belong_pinyin_full: empty_to_none(field_text(document, self.fields.belong_pinyin_full)),
            belong_pinyin_initials: empty_to_none(field_text(
                document,
                self.fields.belong_pinyin_initials,
            )),
            tag_values: empty_to_none(field_text(document, self.fields.tag_values)),
            tag_values_pinyin_full: empty_to_none(field_text(
                document,
                self.fields.tag_values_pinyin_full,
            )),
            tag_values_pinyin_initials: empty_to_none(field_text(
                document,
                self.fields.tag_values_pinyin_initials,
            )),
        }
    }
}

pub(crate) fn sanitize_search_request(
    request: SearchLibraryRequest,
    max_limit: usize,
    max_offset: usize,
    locale: Locale,
) -> Result<SanitizedSearchRequest> {
    let query = request.query.trim().to_string();
    if query.is_empty() {
        anyhow::bail!(crate::i18n::tr(locale, "search-query-empty"));
    }
    if query.chars().count() > harubble_core::SEARCH_LIBRARY_QUERY_MAX_LENGTH {
        anyhow::bail!(crate::i18n::tr(locale, "search-query-too-long"));
    }

    Ok(SanitizedSearchRequest {
        query,
        scope: request.scope,
        limit: request
            .limit
            .unwrap_or(harubble_core::SEARCH_LIBRARY_DEFAULT_LIMIT)
            .min(max_limit),
        offset: request
            .offset
            .unwrap_or(harubble_core::SEARCH_LIBRARY_DEFAULT_OFFSET)
            .min(max_offset),
    })
}

fn build_schema() -> (Schema, LibrarySearchFields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(SEARCH_TOKENIZER_NAME)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    let fields = LibrarySearchFields {
        kind: builder.add_text_field("kind", STRING | STORED),
        album_cid: builder.add_text_field("album_cid", STRING | STORED),
        song_cid: builder.add_text_field("song_cid", STRING | STORED),
        album_title: builder.add_text_field("album_title", text_options.clone()),
        album_title_display: builder.add_text_field("album_title_display", STORED),
        song_title: builder.add_text_field("song_title", text_options.clone()),
        artist_line: builder.add_text_field("artist_line", text_options.clone()),
        intro: builder.add_text_field("intro", text_options.clone()),
        belong: builder.add_text_field("belong", text_options.clone()),
        album_title_pinyin_full: builder
            .add_text_field("album_title_pinyin_full", text_options.clone()),
        album_title_pinyin_initials: builder
            .add_text_field("album_title_pinyin_initials", text_options.clone()),
        song_title_pinyin_full: builder
            .add_text_field("song_title_pinyin_full", text_options.clone()),
        song_title_pinyin_initials: builder
            .add_text_field("song_title_pinyin_initials", text_options.clone()),
        artist_line_pinyin_full: builder
            .add_text_field("artist_line_pinyin_full", text_options.clone()),
        artist_line_pinyin_initials: builder
            .add_text_field("artist_line_pinyin_initials", text_options.clone()),
        belong_pinyin_full: builder.add_text_field("belong_pinyin_full", text_options.clone()),
        belong_pinyin_initials: builder
            .add_text_field("belong_pinyin_initials", text_options.clone()),
        tag_values: builder.add_text_field("tag_values", text_options.clone()),
        tag_values_pinyin_full: builder
            .add_text_field("tag_values_pinyin_full", text_options.clone()),
        tag_values_pinyin_initials: builder
            .add_text_field("tag_values_pinyin_initials", text_options),
    };

    (builder.build(), fields)
}

fn load_fields(schema: Schema) -> Result<LibrarySearchFields> {
    Ok(LibrarySearchFields {
        kind: schema
            .get_field("kind")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        album_cid: schema
            .get_field("album_cid")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        song_cid: schema
            .get_field("song_cid")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        album_title: schema
            .get_field("album_title")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        album_title_display: schema
            .get_field("album_title_display")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        song_title: schema
            .get_field("song_title")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        artist_line: schema
            .get_field("artist_line")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        intro: schema
            .get_field("intro")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        belong: schema
            .get_field("belong")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        album_title_pinyin_full: schema
            .get_field("album_title_pinyin_full")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        album_title_pinyin_initials: schema
            .get_field("album_title_pinyin_initials")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        song_title_pinyin_full: schema
            .get_field("song_title_pinyin_full")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        song_title_pinyin_initials: schema
            .get_field("song_title_pinyin_initials")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        artist_line_pinyin_full: schema
            .get_field("artist_line_pinyin_full")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        artist_line_pinyin_initials: schema
            .get_field("artist_line_pinyin_initials")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        belong_pinyin_full: schema
            .get_field("belong_pinyin_full")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        belong_pinyin_initials: schema
            .get_field("belong_pinyin_initials")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        tag_values: schema
            .get_field("tag_values")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        tag_values_pinyin_full: schema
            .get_field("tag_values_pinyin_full")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        tag_values_pinyin_initials: schema
            .get_field("tag_values_pinyin_initials")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
    })
}

fn register_tokenizers(index: &Index) -> Result<()> {
    let tokenizer = TextAnalyzer::builder(NgramTokenizer::new(1, 3, false)?)
        .filter(LowerCaser)
        .build();
    index
        .tokenizers()
        .register(SEARCH_TOKENIZER_NAME, tokenizer);
    Ok(())
}

fn field_text(document: &TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

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

fn compact_ascii_query(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn escape_query_text(value: &str) -> String {
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

fn rank_search_document(
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

fn collect_matched_fields(
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

fn compare_scored_items(left: &ScoredSearchItem, right: &ScoredSearchItem) -> Ordering {
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

fn scope_kind_value(scope: LibrarySearchScope) -> Option<&'static str> {
    match scope {
        LibrarySearchScope::All => None,
        LibrarySearchScope::Albums => Some("album"),
        LibrarySearchScope::Songs => Some("song"),
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize_search_request, LibrarySearchIndex};
    use crate::preferences::Locale;
    use crate::search::snapshot::LibrarySearchSnapshot;
    use crate::search::snapshot::{LibrarySearchAlbumRecord, LibrarySearchSongRecord};
    use tempfile::tempdir;

    fn build_snapshot() -> LibrarySearchSnapshot {
        LibrarySearchSnapshot {
            root_output_dir: "/tmp/music".to_string(),
            inventory_version: "inv-1".to_string(),
            built_at: "2026-01-01T00:00:00Z".to_string(),
            albums: vec![
                LibrarySearchAlbumRecord {
                    album_cid: "album-a".to_string(),
                    album_title: "Alpha".to_string(),
                    artist_line: Some("Artist One".to_string()),
                    intro: Some("The archive of alpha signals".to_string()),
                    belong: Some("Official Release".to_string()),
                    album_title_pinyin_full: None,
                    album_title_pinyin_initials: None,
                    artist_line_pinyin_full: None,
                    artist_line_pinyin_initials: None,
                    belong_pinyin_full: None,
                    belong_pinyin_initials: None,
                    tag_values: None,
                    tag_values_pinyin_full: None,
                    tag_values_pinyin_initials: None,
                },
                LibrarySearchAlbumRecord {
                    album_cid: "album-b".to_string(),
                    album_title: "白日梦".to_string(),
                    artist_line: Some("塞壬唱片".to_string()),
                    intro: Some("梦境记录".to_string()),
                    belong: Some("官方专辑".to_string()),
                    album_title_pinyin_full: Some("bairimeng".to_string()),
                    album_title_pinyin_initials: Some("brm".to_string()),
                    artist_line_pinyin_full: Some("sirenchangpian".to_string()),
                    artist_line_pinyin_initials: Some("srcp".to_string()),
                    belong_pinyin_full: Some("guanfangzhuanji".to_string()),
                    belong_pinyin_initials: Some("gfzj".to_string()),
                    tag_values: None,
                    tag_values_pinyin_full: None,
                    tag_values_pinyin_initials: None,
                },
                LibrarySearchAlbumRecord {
                    album_cid: "album-c".to_string(),
                    album_title: "Beacon Stories".to_string(),
                    artist_line: Some("Artist Three".to_string()),
                    intro: Some("Mentions alpha only in intro".to_string()),
                    belong: Some("Compilation".to_string()),
                    album_title_pinyin_full: None,
                    album_title_pinyin_initials: None,
                    artist_line_pinyin_full: None,
                    artist_line_pinyin_initials: None,
                    belong_pinyin_full: None,
                    belong_pinyin_initials: None,
                    tag_values: None,
                    tag_values_pinyin_full: None,
                    tag_values_pinyin_initials: None,
                },
            ],
            songs: vec![
                LibrarySearchSongRecord {
                    album_cid: "album-a".to_string(),
                    song_cid: "song-a1".to_string(),
                    album_title: "Alpha".to_string(),
                    song_title: "Beacon".to_string(),
                    artist_line: Some("Artist One".to_string()),
                    song_title_pinyin_full: None,
                    song_title_pinyin_initials: None,
                    artist_line_pinyin_full: None,
                    artist_line_pinyin_initials: None,
                    tag_values: None,
                    tag_values_pinyin_full: None,
                    tag_values_pinyin_initials: None,
                },
                LibrarySearchSongRecord {
                    album_cid: "album-b".to_string(),
                    song_cid: "song-b1".to_string(),
                    album_title: "白日梦".to_string(),
                    song_title: "源石".to_string(),
                    artist_line: Some("塞壬唱片".to_string()),
                    song_title_pinyin_full: Some("yuanshi".to_string()),
                    song_title_pinyin_initials: Some("ys".to_string()),
                    artist_line_pinyin_full: Some("sirenchangpian".to_string()),
                    artist_line_pinyin_initials: Some("srcp".to_string()),
                    tag_values: None,
                    tag_values_pinyin_full: None,
                    tag_values_pinyin_initials: None,
                },
                LibrarySearchSongRecord {
                    album_cid: "album-c".to_string(),
                    song_cid: "song-c1".to_string(),
                    album_title: "Beacon Stories".to_string(),
                    song_title: "Beyond".to_string(),
                    artist_line: Some("Artist Three".to_string()),
                    song_title_pinyin_full: None,
                    song_title_pinyin_initials: None,
                    artist_line_pinyin_full: None,
                    artist_line_pinyin_initials: None,
                    tag_values: None,
                    tag_values_pinyin_full: None,
                    tag_values_pinyin_initials: None,
                },
            ],
        }
    }

    #[test]
    fn rejects_empty_query() {
        let request = harubble_core::SearchLibraryRequest {
            query: "   ".to_string(),
            scope: harubble_core::LibrarySearchScope::All,
            limit: None,
            offset: None,
        };
        let error = match sanitize_search_request(request, 50, 500, Locale::EnUS) {
            Ok(_) => panic!("empty query should fail"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "Search query cannot be empty");
    }

    #[test]
    fn clamps_limit_and_offset() {
        let request = harubble_core::SearchLibraryRequest {
            query: "alpha".to_string(),
            scope: harubble_core::LibrarySearchScope::All,
            limit: Some(999),
            offset: Some(999),
        };
        let sanitized =
            sanitize_search_request(request, 50, 500, Locale::default()).expect("sanitized");
        assert_eq!(sanitized.limit, 50);
        assert_eq!(sanitized.offset, 500);
    }

    #[test]
    fn accepts_plain_text_with_query_parser_characters() {
        let request = harubble_core::SearchLibraryRequest {
            query: "artist:(alpha) \"beta\"".to_string(),
            scope: harubble_core::LibrarySearchScope::All,
            limit: None,
            offset: None,
        };
        let sanitized =
            sanitize_search_request(request, 50, 500, Locale::default()).expect("sanitized");
        assert_eq!(sanitized.query, "artist:(alpha) \"beta\"");
    }

    #[test]
    fn searches_by_scope_and_preserves_total_before_pagination() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "be".to_string(),
                scope: harubble_core::LibrarySearchScope::Songs,
                limit: Some(1),
                offset: Some(0),
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 2);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, harubble_core::SearchLibraryResultKind::Song);
        assert_eq!(items[0].song_title.as_deref(), Some("Beacon"));
    }

    #[test]
    fn recalls_auxiliary_album_fields() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "archive".to_string(),
                scope: harubble_core::LibrarySearchScope::All,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 1);
        assert_eq!(items[0].album_cid, "album-a");
        assert_eq!(
            items[0].matched_fields,
            vec![harubble_core::LibrarySearchHitField::Intro]
        );
    }

    #[test]
    fn supports_pinyin_recall_for_title_and_belong() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");

        let title_request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "brm".to_string(),
                scope: harubble_core::LibrarySearchScope::Albums,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");
        let (title_items, title_total) = index.search(&title_request).expect("search");
        assert_eq!(title_total, 1);
        assert_eq!(title_items[0].album_cid, "album-b");
        assert!(title_items[0]
            .matched_fields
            .contains(&harubble_core::LibrarySearchHitField::Title));

        let belong_request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "guanfang".to_string(),
                scope: harubble_core::LibrarySearchScope::Albums,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");
        let (belong_items, belong_total) = index.search(&belong_request).expect("search");
        assert_eq!(belong_total, 1);
        assert_eq!(belong_items[0].album_cid, "album-b");
        assert!(belong_items[0]
            .matched_fields
            .contains(&harubble_core::LibrarySearchHitField::Belong));
    }

    #[test]
    fn ranks_direct_title_matches_ahead_of_auxiliary_matches() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "alpha".to_string(),
                scope: harubble_core::LibrarySearchScope::All,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 2);
        assert_eq!(items[0].album_cid, "album-a");
        assert_eq!(items[0].kind, harubble_core::SearchLibraryResultKind::Album);
        assert_eq!(items[1].album_cid, "album-c");
        assert!(items[1]
            .matched_fields
            .contains(&harubble_core::LibrarySearchHitField::Intro));
    }

    #[test]
    fn ranks_song_title_exact_match_ahead_of_album_title_partial_match() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "beacon".to_string(),
                scope: harubble_core::LibrarySearchScope::All,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 2);
        assert_eq!(items[0].kind, harubble_core::SearchLibraryResultKind::Song);
        assert_eq!(items[0].song_title.as_deref(), Some("Beacon"));
        assert_eq!(items[1].kind, harubble_core::SearchLibraryResultKind::Album);
        assert_eq!(items[1].album_cid, "album-c");
    }

    #[test]
    fn returns_no_results_for_unmatched_query() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "zzzznothing".to_string(),
                scope: harubble_core::LibrarySearchScope::All,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 0);
        assert!(items.is_empty());
    }

    #[test]
    fn album_only_query_does_not_leak_into_song_scope() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "official".to_string(),
                scope: harubble_core::LibrarySearchScope::Songs,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 0);
        assert!(items.is_empty());
    }

    #[test]
    fn song_only_query_does_not_leak_into_album_scope() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            harubble_core::SearchLibraryRequest {
                query: "beyond".to_string(),
                scope: harubble_core::LibrarySearchScope::Albums,
                limit: None,
                offset: None,
            },
            50,
            500,
            Locale::default(),
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 0);
        assert!(items.is_empty());
    }
}
