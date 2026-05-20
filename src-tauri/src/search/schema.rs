use anyhow::Result;
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, STORED, STRING,
};
use tantivy::tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer};
use tantivy::Index;

pub(crate) const SEARCH_TOKENIZER_NAME: &str = "siren_ngram";

#[derive(Clone, Copy)]
pub(super) struct LibrarySearchFields {
    pub kind: Field,
    pub album_cid: Field,
    pub song_cid: Field,
    pub album_title: Field,
    pub album_title_display: Field,
    pub song_title: Field,
    pub artist_line: Field,
    pub intro: Field,
    pub belong: Field,
    pub album_title_pinyin_full: Field,
    pub album_title_pinyin_initials: Field,
    pub song_title_pinyin_full: Field,
    pub song_title_pinyin_initials: Field,
    pub artist_line_pinyin_full: Field,
    pub artist_line_pinyin_initials: Field,
    pub belong_pinyin_full: Field,
    pub belong_pinyin_initials: Field,
    pub tag_values: Field,
    pub tag_values_pinyin_full: Field,
    pub tag_values_pinyin_initials: Field,
}

pub(super) fn build_schema() -> (Schema, LibrarySearchFields) {
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

pub(super) fn load_fields(schema: Schema) -> Result<LibrarySearchFields> {
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

pub(super) fn register_tokenizers(index: &Index) -> Result<()> {
    let tokenizer = TextAnalyzer::builder(NgramTokenizer::new(1, 3, false)?)
        .filter(LowerCaser)
        .build();
    index
        .tokenizers()
        .register(SEARCH_TOKENIZER_NAME, tokenizer);
    Ok(())
}
