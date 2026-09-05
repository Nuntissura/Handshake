//! Core search / tags / similarity (WP-KERNEL-005, MT-005 event coverage).
//!
//! Legacy source fold-in (translate behavior, NOT SQLite storage):
//!   * `app/backend/library.js` tag manager + TagRule CRUD + `_upsertDerivedTags`
//!     (deterministic rule ordering by `rule_id ASC`) + bulk/manual tagging.
//!   * `app/backend/dhash.js` `hammingDistanceHex64` / `isHex64` 64-bit perceptual
//!     hash distance used for near-duplicate / similarity search.
//!   * `app/backend/palette.js` dominant-palette projection persisted per asset.
//!
//! Storage authority is the single embedded Handshake SurrealDB store. SQLite
//! and PostgreSQL are forbidden in this runtime path.
//! Every mutation emits an atelier event from the new families defined
//! below so the operator surface, Locus, and replay can reconstruct history.
//!
//! Design notes mirrored from legacy source:
//!   * Tags are deduplicated by normalized text (a `Tag` dictionary), and linked
//!     to characters with a `tag_type` of `manual` or `derived`.
//!   * Tag rules are applied deterministically ordered by `rule_id` (here the
//!     UUID `rule_id`) so derived tags are reproducible across runs.
//!   * Derived tags are recomputed by clearing all `derived` links then
//!     re-inserting the rule output, exactly like `_upsertDerivedTags`.
//!   * Similarity is a projection table holding the dHash hex + palette JSON per
//!     media asset; nearest-neighbour search is Hamming distance over the hex.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::refs::{
    character_ref, collection_ref, media_asset_ref, parse_character_ref, parse_collection_ref,
    parse_media_asset_ref, parse_sheet_version_ref, parse_tag_ref, sheet_version_ref, tag_ref,
};
use super::{
    atelier_event_sql, event_ref_for_text, reject_legacy_runtime_ref, AtelierError, AtelierResult,
    AtelierStore, BulkTagRequest,
};
use crate::llm::{EmbeddingRequest, LlmClient};
use std::collections::{BTreeSet, HashSet};

/// Event families emitted by the search/tags/similarity submodule (MT-005).
///
/// Defined here as `pub const` so the parent can extend `event_family::ALL`
/// coverage. Kept distinct from the foundation families in `mod.rs`.
pub mod search_event_family {
    /// A tag was attached to a character (manual or derived).
    pub const CHARACTER_TAGGED: &str = "atelier.tag.character_tagged";
    /// A tag was detached from a character.
    pub const CHARACTER_UNTAGGED: &str = "atelier.tag.character_untagged";
    /// A saved tag rule was created or updated.
    pub const TAG_RULE_UPSERTED: &str = "atelier.tag.rule_upserted";
    /// A saved tag rule was deleted.
    pub const TAG_RULE_DELETED: &str = "atelier.tag.rule_deleted";
    /// Derived tags were recomputed for a character from the rule set.
    pub const DERIVED_TAGS_RECOMPUTED: &str = "atelier.tag.derived_recomputed";
    /// A similarity projection (dHash + palette) was upserted for a media asset.
    pub const SIMILARITY_PROJECTED: &str = "atelier.similarity.projected";
    /// A similarity projection rebuild job completed for a media asset.
    pub const SIMILARITY_REBUILD_COMPLETED: &str = "atelier.similarity.rebuild_completed";
    /// A similarity projection rebuild job failed for a media asset.
    pub const SIMILARITY_REBUILD_FAILED: &str = "atelier.similarity.rebuild_failed";
    /// An AI tag suggestion proposal was recorded.
    pub const AI_TAG_SUGGESTION_RECORDED: &str = "atelier.tag.ai_suggestion_recorded";
    /// An AI tag suggestion proposal was accepted for later application.
    pub const AI_TAG_SUGGESTION_ACCEPTED: &str = "atelier.tag.ai_suggestion_accepted";
    /// An AI tag suggestion proposal was rejected.
    pub const AI_TAG_SUGGESTION_REJECTED: &str = "atelier.tag.ai_suggestion_rejected";
    /// An accepted AI tag suggestion was applied into the reviewed tag surface.
    pub const AI_TAG_SUGGESTION_APPLIED: &str = "atelier.tag.ai_suggestion_applied";
    /// A saved search was created or updated.
    pub const SAVED_SEARCH_UPSERTED: &str = "atelier.search.saved_search_upserted";
    /// A saved search was deleted.
    pub const SAVED_SEARCH_DELETED: &str = "atelier.search.saved_search_deleted";
    /// A rich CKC tag note was created or updated (WP-CKC-posekit-overhaul MT-011).
    pub const TAG_NOTE_UPSERTED: &str = "atelier.tag.note_upserted";

    /// All search/tags/similarity event families (parity / coverage checks).
    pub const ALL: &[&str] = &[
        TAG_NOTE_UPSERTED,
        CHARACTER_TAGGED,
        CHARACTER_UNTAGGED,
        TAG_RULE_UPSERTED,
        TAG_RULE_DELETED,
        DERIVED_TAGS_RECOMPUTED,
        SIMILARITY_PROJECTED,
        SIMILARITY_REBUILD_COMPLETED,
        SIMILARITY_REBUILD_FAILED,
        AI_TAG_SUGGESTION_RECORDED,
        AI_TAG_SUGGESTION_ACCEPTED,
        AI_TAG_SUGGESTION_REJECTED,
        AI_TAG_SUGGESTION_APPLIED,
        SAVED_SEARCH_UPSERTED,
        SAVED_SEARCH_DELETED,
    ];
}

/// How a tag became attached to a character.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TagType {
    Manual,
    Derived,
}

impl TagType {
    fn as_str(self) -> &'static str {
        match self {
            TagType::Manual => "manual",
            TagType::Derived => "derived",
        }
    }
}

/// How a tag rule matches a source field value (legacy source `match_type`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    Equals,
    Contains,
    Regex,
}

impl MatchType {
    fn as_str(self) -> &'static str {
        match self {
            MatchType::Equals => "equals",
            MatchType::Contains => "contains",
            MatchType::Regex => "regex",
        }
    }

    fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "equals" => Ok(MatchType::Equals),
            "contains" => Ok(MatchType::Contains),
            "regex" => Ok(MatchType::Regex),
            other => Err(AtelierError::Validation(format!(
                "unknown tag-rule match_type: {other}"
            ))),
        }
    }
}

/// A tag in the dictionary, deduplicated by normalized `text`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    pub tag_id: Uuid,
    pub text: String,
    pub created_at_utc: DateTime<Utc>,
}

/// A tag attached to a character with its provenance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterTag {
    pub character_internal_id: Uuid,
    pub tag_id: Uuid,
    pub text: String,
    pub tag_type: TagType,
}

/// A saved tag rule: when a character field matches, emit a derived tag.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagRule {
    pub rule_id: Uuid,
    pub source_field_id: String,
    pub match_type: MatchType,
    pub pattern: String,
    pub emit_tag: String,
    pub enabled: bool,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input for creating a saved tag rule.
#[derive(Clone, Debug)]
pub struct NewTagRule {
    pub source_field_id: String,
    pub match_type: MatchType,
    pub pattern: String,
    pub emit_tag: String,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiTagSuggestionStatus {
    Proposed,
    Accepted,
    Rejected,
    Applied,
}

impl AiTagSuggestionStatus {
    fn as_str(self) -> &'static str {
        match self {
            AiTagSuggestionStatus::Proposed => "proposed",
            AiTagSuggestionStatus::Accepted => "accepted",
            AiTagSuggestionStatus::Rejected => "rejected",
            AiTagSuggestionStatus::Applied => "applied",
        }
    }

    fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "proposed" => Ok(AiTagSuggestionStatus::Proposed),
            "accepted" => Ok(AiTagSuggestionStatus::Accepted),
            "rejected" => Ok(AiTagSuggestionStatus::Rejected),
            "applied" => Ok(AiTagSuggestionStatus::Applied),
            other => Err(AtelierError::Validation(format!(
                "unknown AI tag suggestion status: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AiTagSuggestion {
    pub suggestion_id: Uuid,
    pub character_internal_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub tag_text: String,
    pub confidence: Option<f64>,
    pub model_receipt_ref: String,
    pub tool_receipt_ref: String,
    pub suggested_by: String,
    pub status: AiTagSuggestionStatus,
    pub decided_by: Option<String>,
    pub decision_reason: Option<String>,
    pub applied_tag_id: Option<Uuid>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewAiTagSuggestion {
    pub character_internal_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub tag_text: String,
    pub confidence: Option<f64>,
    pub model_receipt_ref: String,
    pub tool_receipt_ref: String,
    pub suggested_by: String,
}

#[derive(Clone, Debug)]
pub struct AiTagSuggestionDecision {
    pub suggestion_id: Uuid,
    pub decided_by: String,
    pub reason: Option<String>,
}

/// A similarity projection (perceptual hash + dominant palette) for an asset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimilarityProjection {
    pub asset_internal_id: Uuid,
    pub dhash_hex: Option<String>,
    pub palette_json: serde_json::Value,
    pub updated_at_utc: DateTime<Utc>,
}

/// A nearest-neighbour similarity hit (legacy source `image.similar.search`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimilarityHit {
    pub asset_internal_id: Uuid,
    pub dhash_hex: String,
    pub distance: i32,
}

/// A cross-domain search hit with a bounded snippet and stable jump target.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalSearchHit {
    pub target_kind: String,
    pub target_id: String,
    pub jump_target: String,
    pub title: String,
    pub snippet: String,
    pub rank: i64,
    pub extraction_tier: LensExtractionTier,
    pub content_tier: Option<LensContentTier>,
    pub view_mode: LensViewMode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LensExtractionTier {
    Tier1,
    Tier2,
    Tier3,
}

impl Default for LensExtractionTier {
    fn default() -> Self {
        Self::Tier1
    }
}

impl LensExtractionTier {
    fn rank(self) -> i32 {
        match self {
            Self::Tier1 => 1,
            Self::Tier2 => 2,
            Self::Tier3 => 3,
        }
    }

    fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "tier1" => Ok(Self::Tier1),
            "tier2" => Ok(Self::Tier2),
            "tier3" => Ok(Self::Tier3),
            other => Err(AtelierError::Validation(format!(
                "unknown lens extraction tier: {other}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LensViewMode {
    Nsfw,
    Sfw,
}

impl Default for LensViewMode {
    fn default() -> Self {
        Self::Nsfw
    }
}

impl LensViewMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Nsfw => "NSFW",
            Self::Sfw => "SFW",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LensContentTier {
    Sfw,
    AdultSoft,
    AdultExplicit,
}

impl LensContentTier {
    fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "sfw" => Ok(Self::Sfw),
            "adult_soft" => Ok(Self::AdultSoft),
            "adult_explicit" => Ok(Self::AdultExplicit),
            other => Err(AtelierError::Validation(format!(
                "unknown lens content tier: {other}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LensSearchFilters {
    /// Maximum extraction tier to expose. The Lens default is Tier1.
    pub extraction_tier: LensExtractionTier,
    /// In SFW mode only explicitly SFW candidates survive; unknown tiers are dropped.
    pub view_mode: LensViewMode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum SavedSearchScope {
    AllMedia,
    Collection(Uuid),
}

impl Default for SavedSearchScope {
    fn default() -> Self {
        Self::AllMedia
    }
}

impl SavedSearchScope {
    fn into_parts(self) -> (&'static str, Option<Uuid>) {
        match self {
            Self::AllMedia => ("all_media", None),
            Self::Collection(collection_id) => ("collection", Some(collection_id)),
        }
    }

    fn from_parts(scope_kind: &str, scope_id: Option<Uuid>) -> AtelierResult<Self> {
        match (scope_kind, scope_id) {
            ("all_media", None) => Ok(Self::AllMedia),
            ("collection", Some(collection_id)) => Ok(Self::Collection(collection_id)),
            ("all_media", Some(_)) => Err(AtelierError::Validation(
                "saved search all_media scope must not have scope_id".into(),
            )),
            ("collection", None) => Err(AtelierError::Validation(
                "saved search collection scope requires scope_id".into(),
            )),
            (other, _) => Err(AtelierError::Validation(format!(
                "unknown saved search scope_kind: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedSearchFilters {
    pub include_tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub min_rating: Option<i16>,
    pub favorite: Option<bool>,
    pub color_hex: Option<String>,
    pub scope: SavedSearchScope,
    pub view_mode: LensViewMode,
}

impl Default for SavedSearchFilters {
    fn default() -> Self {
        Self {
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            min_rating: None,
            favorite: None,
            color_hex: None,
            scope: SavedSearchScope::AllMedia,
            view_mode: LensViewMode::Nsfw,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewSavedSearch {
    pub name: String,
    pub filters: SavedSearchFilters,
    pub created_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedSearch {
    pub saved_search_id: Uuid,
    pub name: String,
    pub filters: SavedSearchFilters,
    pub created_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedSearchProjectionHit {
    pub saved_search_id: Uuid,
    pub asset_id: Uuid,
    pub content_hash: String,
    pub artifact_ref: String,
    pub jump_target: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub rating: i16,
    pub matched_color_hex: Option<String>,
    pub content_tier: Option<LensContentTier>,
    pub view_mode: LensViewMode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SimilarityRebuildJobStatus {
    Running,
    Completed,
    Failed,
}

impl SimilarityRebuildJobStatus {
    fn as_str(self) -> &'static str {
        match self {
            SimilarityRebuildJobStatus::Running => "running",
            SimilarityRebuildJobStatus::Completed => "completed",
            SimilarityRebuildJobStatus::Failed => "failed",
        }
    }

    fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "running" => Ok(SimilarityRebuildJobStatus::Running),
            "completed" => Ok(SimilarityRebuildJobStatus::Completed),
            "failed" => Ok(SimilarityRebuildJobStatus::Failed),
            other => Err(AtelierError::Validation(format!(
                "unknown similarity rebuild job status: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimilarityRebuildJob {
    pub job_id: Uuid,
    pub asset_internal_id: Uuid,
    pub status: SimilarityRebuildJobStatus,
    pub requested_by: String,
    pub processed_count: i64,
    pub failed_count: i64,
    pub dhash_hex: Option<String>,
    pub palette_json: Option<serde_json::Value>,
    pub error_ref: Option<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// dHash hex must be exactly 16 lowercase hex chars (64 bits). Mirrors legacy source
/// `dhash.js::isHex64`.
fn is_hex64(s: &str) -> bool {
    let t = s.trim();
    t.len() == 16 && t.chars().all(|c| c.is_ascii_hexdigit())
}

/// Normalize tag text: trim + lowercase so the dictionary dedupes case- and
/// whitespace-insensitively, matching legacy source tag handling intent.
pub(crate) fn normalize_tag(text: &str) -> String {
    text.trim().to_ascii_lowercase()
}

fn normalized_saved_search_tags(field: &str, tags: &[String]) -> AtelierResult<Vec<String>> {
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = normalize_tag(tag);
        if tag.is_empty() {
            return Err(AtelierError::Validation(format!(
                "{field} must not contain empty tags"
            )));
        }
        if !normalized.contains(&tag) {
            normalized.push(tag);
        }
    }
    normalized.sort();
    Ok(normalized)
}

fn normalize_saved_search_color(color_hex: &Option<String>) -> AtelierResult<Option<String>> {
    let Some(value) = color_hex.as_deref() else {
        return Ok(None);
    };
    let value = value.trim().to_ascii_lowercase();
    let valid = value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    if !valid {
        return Err(AtelierError::Validation(format!(
            "saved search color_hex must be #rrggbb, got {value:?}"
        )));
    }
    Ok(Some(value))
}

fn normalize_saved_search_filters(
    filters: &SavedSearchFilters,
) -> AtelierResult<SavedSearchFilters> {
    if let Some(rating) = filters.min_rating {
        if !(0..=5).contains(&rating) {
            return Err(AtelierError::Validation(format!(
                "saved search min_rating must be between 0 and 5, got {rating}"
            )));
        }
    }
    Ok(SavedSearchFilters {
        include_tags: normalized_saved_search_tags("include_tags", &filters.include_tags)?,
        exclude_tags: normalized_saved_search_tags("exclude_tags", &filters.exclude_tags)?,
        min_rating: filters.min_rating,
        favorite: filters.favorite,
        color_hex: normalize_saved_search_color(&filters.color_hex)?,
        scope: filters.scope,
        view_mode: filters.view_mode,
    })
}

fn saved_search_tags_from_json(value: serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Hamming distance between two 16-char hex (64-bit) hashes. Mirrors legacy source
/// `dhash.js::hammingDistanceHex64`: invalid inputs return the max distance 64.
fn hamming_distance_hex64(a: &str, b: &str) -> i32 {
    let sa = a.trim().to_ascii_lowercase();
    let sb = b.trim().to_ascii_lowercase();
    if !is_hex64(&sa) || !is_hex64(&sb) {
        return 64;
    }
    let na = u64::from_str_radix(&sa, 16).unwrap_or(u64::MAX);
    let nb = u64::from_str_radix(&sb, 16).unwrap_or(0);
    (na ^ nb).count_ones() as i32
}

fn compute_similarity_from_image_bytes(
    image_bytes: &[u8],
) -> AtelierResult<(String, serde_json::Value)> {
    let image = image::load_from_memory(image_bytes).map_err(|err| {
        AtelierError::Validation(format!("similarity image decode failed: {err}"))
    })?;
    let dhash_hex = compute_dhash_hex(&image);
    let palette_json = compute_palette_json(&image);
    Ok((dhash_hex, palette_json))
}

fn compute_dhash_hex(image: &image::DynamicImage) -> String {
    let gray = image.to_luma8();
    let resized = image::imageops::resize(&gray, 9, 8, image::imageops::FilterType::Triangle);
    let mut hash = 0u64;
    let mut bit = 0u32;
    for y in 0..8 {
        for x in 0..8 {
            let left = resized.get_pixel(x, y)[0];
            let right = resized.get_pixel(x + 1, y)[0];
            if left > right {
                hash |= 1u64 << (63 - bit);
            }
            bit += 1;
        }
    }
    format!("{hash:016x}")
}

fn compute_palette_json(image: &image::DynamicImage) -> serde_json::Value {
    let rgb = image.to_rgb8();
    let sample = image::imageops::thumbnail(&rgb, 64, 64);
    let mut counts: HashMap<[u8; 3], i64> = HashMap::new();
    for pixel in sample.pixels() {
        let [r, g, b] = pixel.0;
        *counts.entry([r, g, b]).or_insert(0) += 1;
    }
    let sampled_pixels: i64 = counts.values().sum();
    let mut entries: Vec<(String, i64)> = counts
        .into_iter()
        .map(|([r, g, b], count)| (format!("#{r:02x}{g:02x}{b:02x}"), count))
        .collect();
    entries.sort_by(|(hex_a, count_a), (hex_b, count_b)| {
        count_b.cmp(count_a).then_with(|| hex_a.cmp(hex_b))
    });
    let dominant: Vec<serde_json::Value> = entries
        .into_iter()
        .take(8)
        .map(|(hex, count)| {
            serde_json::json!({
                "hex": hex,
                "count": count,
                "ratio": if sampled_pixels == 0 {
                    0.0
                } else {
                    count as f64 / sampled_pixels as f64
                },
            })
        })
        .collect();
    serde_json::json!({
        "algorithm": "rgb_exact_thumbnail_v1",
        "sampled_pixels": sampled_pixels,
        "dominant": dominant,
    })
}

#[derive(SurrealValue)]
struct SimilarityRebuildJobRow {
    job_id: SurrealUuid,
    asset_internal_id: SurrealUuid,
    status: String,
    requested_by: String,
    processed_count: i64,
    failed_count: i64,
    dhash_hex: Option<String>,
    palette_json: Option<serde_json::Value>,
    error_ref: Option<String>,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

fn similarity_rebuild_job_from_row(
    row: SimilarityRebuildJobRow,
) -> AtelierResult<SimilarityRebuildJob> {
    let status = row.status;
    Ok(SimilarityRebuildJob {
        job_id: row.job_id.into(),
        asset_internal_id: row.asset_internal_id.into(),
        status: SimilarityRebuildJobStatus::parse(&status)?,
        requested_by: row.requested_by,
        processed_count: row.processed_count,
        failed_count: row.failed_count,
        dhash_hex: row.dhash_hex,
        palette_json: row.palette_json,
        error_ref: row.error_ref,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

fn require_similarity_rebuild_actor(requested_by: &str) -> AtelierResult<&str> {
    let trimmed = requested_by.trim();
    if trimmed.is_empty() || trimmed != requested_by {
        return Err(AtelierError::Validation(
            "similarity rebuild requested_by must not be empty or padded".into(),
        ));
    }
    Ok(trimmed)
}

fn compact_search_snippet(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn bounded_search_snippet(value: &str, query: &str) -> String {
    const BEFORE: usize = 32;
    const AFTER: usize = 72;
    const MAX_SNIPPET_CHARS: usize = 160;
    let compact = compact_search_snippet(value);
    if compact.is_empty() {
        return String::new();
    }
    let lower_text = compact.to_lowercase();
    let lower_query = query.to_lowercase();
    let match_char_idx = lower_text
        .find(&lower_query)
        .map(|byte_idx| lower_text[..byte_idx].chars().count())
        .unwrap_or(0);
    let query_char_len = lower_query.chars().count().max(1);
    let chars: Vec<char> = compact.chars().collect();
    let start = match_char_idx.saturating_sub(BEFORE);
    let end = (match_char_idx + query_char_len + AFTER).min(chars.len());
    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.extend(chars[start..end].iter());
    if end < chars.len() {
        snippet.push_str("...");
    }
    if snippet.chars().count() > MAX_SNIPPET_CHARS {
        let mut truncated: String = snippet
            .chars()
            .take(MAX_SNIPPET_CHARS.saturating_sub(3))
            .collect();
        truncated.push_str("...");
        truncated
    } else {
        snippet
    }
}

#[derive(SurrealValue)]
struct GlobalSearchCandidateRow {
    target_kind: String,
    target_id: String,
    jump_target: String,
    title: String,
    search_text: String,
    rank: i64,
    sort_at: Datetime,
}

fn global_search_hit_from_row(
    row: GlobalSearchCandidateRow,
    query: &str,
    view_mode: LensViewMode,
) -> AtelierResult<GlobalSearchHit> {
    let search_text = row.search_text;
    let normalized = search_text.to_ascii_lowercase().replace(['_', '-'], " ");
    let extraction_tier_raw = if normalized.contains("extraction tier tier3")
        || normalized.contains("extraction tier 3")
    {
        "tier3"
    } else if normalized.contains("extraction tier tier2")
        || normalized.contains("extraction tier 2")
    {
        "tier2"
    } else {
        "tier1"
    };
    let content_tier_raw = if normalized.contains("content tier adult explicit") {
        Some("adult_explicit")
    } else if normalized.contains("content tier adult soft") {
        Some("adult_soft")
    } else if normalized.contains("content tier sfw") {
        Some("sfw")
    } else {
        None
    };
    Ok(GlobalSearchHit {
        target_kind: row.target_kind,
        target_id: row.target_id,
        jump_target: row.jump_target,
        title: row.title,
        snippet: bounded_search_snippet(&search_text, query),
        rank: row.rank,
        extraction_tier: LensExtractionTier::parse(extraction_tier_raw)?,
        content_tier: content_tier_raw.map(LensContentTier::parse).transpose()?,
        view_mode,
    })
}

#[derive(SurrealValue)]
struct SavedSearchRow {
    saved_search_id: SurrealUuid,
    name: String,
    include_tags_json: serde_json::Value,
    exclude_tags_json: serde_json::Value,
    min_rating: Option<i64>,
    favorite: Option<bool>,
    color_hex: Option<String>,
    scope_kind: String,
    scope_id: Option<SurrealUuid>,
    view_mode: String,
    created_by: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

fn saved_search_from_row(row: SavedSearchRow) -> AtelierResult<SavedSearch> {
    let view_mode = row.view_mode;
    let view_mode = match view_mode.as_str() {
        "NSFW" => LensViewMode::Nsfw,
        "SFW" => LensViewMode::Sfw,
        other => {
            return Err(AtelierError::Validation(format!(
                "unknown saved search view_mode: {other}"
            )));
        }
    };
    Ok(SavedSearch {
        saved_search_id: row.saved_search_id.into(),
        name: row.name,
        filters: SavedSearchFilters {
            include_tags: saved_search_tags_from_json(row.include_tags_json),
            exclude_tags: saved_search_tags_from_json(row.exclude_tags_json),
            min_rating: row.min_rating.map(i16::try_from).transpose().map_err(|_| {
                AtelierError::Internal("saved search min_rating exceeds i16".into())
            })?,
            favorite: row.favorite,
            color_hex: row.color_hex,
            scope: SavedSearchScope::from_parts(&row.scope_kind, row.scope_id.map(Into::into))?,
            view_mode,
        },
        created_by: row.created_by,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

#[derive(SurrealValue)]
struct SavedSearchProjectionRow {
    saved_search_id: SurrealUuid,
    asset_id: SurrealUuid,
    content_hash: String,
    artifact_ref: String,
    jump_target: String,
    tags_json: serde_json::Value,
    favorite: bool,
    rating: i64,
    matched_color_hex: Option<String>,
    content_tier: Option<String>,
    view_mode: String,
}

fn saved_search_projection_hit_from_row(
    row: SavedSearchProjectionRow,
) -> AtelierResult<SavedSearchProjectionHit> {
    let view_mode = row.view_mode;
    let view_mode = match view_mode.as_str() {
        "NSFW" => LensViewMode::Nsfw,
        "SFW" => LensViewMode::Sfw,
        other => {
            return Err(AtelierError::Validation(format!(
                "unknown saved search projection view_mode: {other}"
            )));
        }
    };
    Ok(SavedSearchProjectionHit {
        saved_search_id: row.saved_search_id.into(),
        asset_id: row.asset_id.into(),
        content_hash: row.content_hash,
        artifact_ref: row.artifact_ref,
        jump_target: row.jump_target,
        tags: saved_search_tags_from_json(row.tags_json),
        favorite: row.favorite,
        rating: i16::try_from(row.rating)
            .map_err(|_| AtelierError::Internal("saved search rating exceeds i16".into()))?,
        matched_color_hex: row.matched_color_hex,
        content_tier: row
            .content_tier
            .as_deref()
            .map(LensContentTier::parse)
            .transpose()?,
        view_mode,
    })
}

#[derive(SurrealValue)]
struct AiTagSuggestionRow {
    suggestion_id: SurrealUuid,
    character_internal_id: SurrealUuid,
    asset_id: Option<SurrealUuid>,
    tag_text: String,
    confidence: Option<f64>,
    model_receipt_ref: String,
    tool_receipt_ref: String,
    suggested_by: String,
    status: String,
    decided_by: Option<String>,
    decision_reason: Option<String>,
    applied_tag_id: Option<SurrealUuid>,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

fn ai_tag_suggestion_from_row(row: AiTagSuggestionRow) -> AtelierResult<AiTagSuggestion> {
    let status = row.status;
    Ok(AiTagSuggestion {
        suggestion_id: row.suggestion_id.into(),
        character_internal_id: row.character_internal_id.into(),
        asset_id: row.asset_id.map(Into::into),
        tag_text: row.tag_text,
        confidence: row.confidence,
        model_receipt_ref: row.model_receipt_ref,
        tool_receipt_ref: row.tool_receipt_ref,
        suggested_by: row.suggested_by,
        status: AiTagSuggestionStatus::parse(&status)?,
        decided_by: row.decided_by,
        decision_reason: row.decision_reason,
        applied_tag_id: row.applied_tag_id.map(Into::into),
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

fn normalize_ai_tag_suggestion_confidence(confidence: Option<f64>) -> AtelierResult<Option<f64>> {
    match confidence {
        Some(value) if value.is_finite() && (0.0..=1.0).contains(&value) => Ok(Some(value)),
        Some(value) => Err(AtelierError::Validation(format!(
            "AI tag suggestion confidence must be between 0.0 and 1.0, got {value}"
        ))),
        None => Ok(None),
    }
}

fn require_ai_tag_actor<'a>(field: &str, actor: &'a str) -> AtelierResult<&'a str> {
    let trimmed = actor.trim();
    if trimmed.is_empty() || trimmed != actor {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(trimmed)
}

fn normalize_ai_tag_reason(reason: &Option<String>) -> AtelierResult<Option<String>> {
    match reason {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else if trimmed != value {
                Err(AtelierError::Validation(
                    "AI tag suggestion decision reason must not be padded".into(),
                ))
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        None => Ok(None),
    }
}

fn validate_ai_tag_receipt_ref(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    reject_legacy_runtime_ref(field, value)?;
    let required_prefix = match field {
        "model_receipt_ref" => "receipt://atelier/model/",
        "tool_receipt_ref" => "receipt://atelier/tool/",
        _ => {
            return Err(AtelierError::Validation(format!(
                "{field} is not a supported AI tag receipt field"
            )));
        }
    };
    let suffix = value.strip_prefix(required_prefix).ok_or_else(|| {
        AtelierError::Validation(format!(
            "{field} must be a Handshake receipt ref under {required_prefix}"
        ))
    })?;
    if suffix.is_empty() || suffix.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(format!(
            "{field} must include a non-empty receipt id"
        )));
    }
    Ok(())
}

#[derive(SurrealValue)]
struct TagRow {
    tag_id: SurrealUuid,
    text: String,
    created_at_utc: Datetime,
}

fn tag_from_row(row: TagRow) -> Tag {
    Tag {
        tag_id: row.tag_id.into(),
        text: row.text,
        created_at_utc: row.created_at_utc.into(),
    }
}

#[derive(SurrealValue)]
struct TagRuleRow {
    rule_id: SurrealUuid,
    source_field_id: String,
    match_type: String,
    pattern: String,
    emit_tag: String,
    enabled: bool,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

fn rule_from_row(row: TagRuleRow) -> AtelierResult<TagRule> {
    let match_type_raw = row.match_type;
    Ok(TagRule {
        rule_id: row.rule_id.into(),
        source_field_id: row.source_field_id,
        match_type: MatchType::parse(&match_type_raw)?,
        pattern: row.pattern,
        emit_tag: row.emit_tag,
        enabled: row.enabled,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

#[derive(SurrealValue)]
struct EmptyBindings {}

#[derive(SurrealValue)]
struct UuidBinding {
    value: SurrealUuid,
}

#[derive(SurrealValue)]
struct RecordBinding {
    value: RecordId,
}

#[derive(SurrealValue)]
struct OptionalRecordBinding {
    value: Option<RecordId>,
}

#[derive(SurrealValue)]
struct QueryLimitBindings {
    query: String,
    limit: i64,
}

#[derive(SurrealValue)]
struct IdLimitBindings {
    id: SurrealUuid,
    limit: i64,
}

#[derive(SurrealValue)]
struct TextBinding {
    text: String,
}

#[derive(SurrealValue)]
struct OptionalStringBinding {
    value: Option<String>,
}

#[derive(SurrealValue)]
struct CharacterTagRow {
    character_internal_id: SurrealUuid,
    tag_id: SurrealUuid,
    text: String,
    tag_type: String,
}

fn character_tag_from_row(row: CharacterTagRow) -> AtelierResult<CharacterTag> {
    let tag_type = match row.tag_type.as_str() {
        "manual" => TagType::Manual,
        "derived" => TagType::Derived,
        other => {
            return Err(AtelierError::Validation(format!(
                "unknown tag_type: {other}"
            )))
        }
    };
    Ok(CharacterTag {
        character_internal_id: row.character_internal_id.into(),
        tag_id: row.tag_id.into(),
        text: row.text,
        tag_type,
    })
}

#[derive(SurrealValue)]
struct SimilarityProjectionRow {
    asset_internal_id: SurrealUuid,
    dhash_hex: Option<String>,
    palette_json: serde_json::Value,
    updated_at_utc: Datetime,
}

impl From<SimilarityProjectionRow> for SimilarityProjection {
    fn from(row: SimilarityProjectionRow) -> Self {
        Self {
            asset_internal_id: row.asset_internal_id.into(),
            dhash_hex: row.dhash_hex,
            palette_json: row.palette_json,
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct SimilarityCandidateRow {
    asset_internal_id: SurrealUuid,
    dhash_hex: String,
}

#[derive(Clone, SurrealValue)]
struct SavedSearchBindings {
    rid: RecordId,
    saved_search_id: SurrealUuid,
    name: String,
    include_tags_json: serde_json::Value,
    exclude_tags_json: serde_json::Value,
    min_rating: Option<i64>,
    favorite: Option<bool>,
    color_hex: Option<String>,
    scope_kind: String,
    scope_id: Option<SurrealUuid>,
    view_mode: String,
    created_by: String,
}

#[derive(Clone, SurrealValue)]
struct TagCharacterBindings {
    rid: RecordId,
    character_ref: RecordId,
    tag_ref: RecordId,
    tag_type: String,
}

#[derive(Clone, SurrealValue)]
struct AiSuggestionBindings {
    rid: RecordId,
    suggestion_id: SurrealUuid,
    character_ref: RecordId,
    asset_ref: Option<RecordId>,
    tag_text: String,
    confidence: Option<f64>,
    model_receipt_ref: String,
    tool_receipt_ref: String,
    suggested_by: String,
}

#[derive(Clone, SurrealValue)]
struct AiDecisionBindings {
    rid: RecordId,
    status: String,
    decided_by: String,
    decision_reason: Option<String>,
}
#[derive(Clone, SurrealValue)]
struct ApplyAiBindings {
    rid: RecordId,
    link_rid: RecordId,
    character_ref: RecordId,
    tag_ref: RecordId,
    applied_by: String,
}

#[derive(Clone, SurrealValue)]
struct TagRuleBindings {
    rid: RecordId,
    rule_id: SurrealUuid,
    source_field_id: String,
    match_type: String,
    pattern: String,
    emit_tag: String,
    enabled: bool,
}

#[derive(Clone, SurrealValue)]
struct ProjectionBindings {
    rid: RecordId,
    asset_ref: RecordId,
    dhash_hex: Option<String>,
    palette_json: serde_json::Value,
}

#[derive(Clone, SurrealValue)]
struct RebuildJobBindings {
    rid: RecordId,
    job_id: SurrealUuid,
    asset_ref: RecordId,
    requested_by: String,
}

#[derive(SurrealValue)]
struct UpdateRebuildBindings {
    rid: RecordId,
    status: String,
    processed_count: i64,
    failed_count: i64,
    dhash_hex: Option<String>,
    palette_json: Option<serde_json::Value>,
    error_ref: Option<String>,
}
#[derive(Clone, SurrealValue)]
struct DerivedTagInput {
    rid: RecordId,
    tag_ref: RecordId,
}
#[derive(SurrealValue)]
struct DerivedTagsBindings {
    character_ref: RecordId,
    items: Vec<DerivedTagInput>,
}

macro_rules! saved_search_select { () => { "saved_search_id, name, include_tags_json, exclude_tags_json, min_rating, favorite, color_hex, scope_kind, scope_id, view_mode, created_by, created_at_utc, updated_at_utc" }; }
macro_rules! ai_suggestion_select { () => { "suggestion_id, record::id(character_internal_id) AS character_internal_id, record::id(asset_id) AS asset_id, tag_text, confidence, model_receipt_ref, tool_receipt_ref, suggested_by, status, decided_by, decision_reason, record::id(applied_tag_id) AS applied_tag_id, created_at_utc, updated_at_utc" }; }
macro_rules! rule_select { () => { "rule_id, source_field_id, match_type, pattern, emit_tag, enabled, created_at_utc, updated_at_utc" }; }
macro_rules! projection_select { () => { "record::id(asset_internal_id) AS asset_internal_id, dhash_hex, palette_json, updated_at_utc" }; }
macro_rules! rebuild_select { () => { "job_id, record::id(asset_internal_id) AS asset_internal_id, status, requested_by, processed_count, failed_count, dhash_hex, palette_json, error_ref, created_at_utc, updated_at_utc" }; }

const UPSERT_SAVED_SEARCH: &str = concat!("RETURN { ", atelier_event_sql!(), " UPSERT $domain.rid SET saved_search_id=$domain.saved_search_id, name=$domain.name, include_tags_json=$domain.include_tags_json, exclude_tags_json=$domain.exclude_tags_json, min_rating=$domain.min_rating, favorite=$domain.favorite, color_hex=$domain.color_hex, scope_kind=$domain.scope_kind, scope_id=$domain.scope_id, view_mode=$domain.view_mode, created_by=$domain.created_by, updated_at_utc=time::now(); RETURN (SELECT ", saved_search_select!(), " FROM $domain.rid); };");
const UPSERT_TAG_RULE: &str = concat!("RETURN { ", atelier_event_sql!(), " UPSERT $domain.rid SET rule_id=$domain.rule_id, source_field_id=$domain.source_field_id, match_type=$domain.match_type, pattern=$domain.pattern, emit_tag=$domain.emit_tag, enabled=$domain.enabled, updated_at_utc=time::now(); RETURN (SELECT ", rule_select!(), " FROM $domain.rid); };");
const UPSERT_PROJECTION: &str = concat!("RETURN { ", atelier_event_sql!(), " UPSERT $domain.rid SET asset_internal_id=$domain.asset_ref, dhash_hex=$domain.dhash_hex, palette_json=$domain.palette_json, updated_at_utc=time::now(); RETURN (SELECT ", projection_select!(), " FROM $domain.rid); };");
const TAG_CHARACTER_STATEMENT: &str = concat!("RETURN { ", atelier_event_sql!(), " UPSERT $domain.rid SET character_internal_id=$domain.character_ref, tag_id=$domain.tag_ref, tag_type=$domain.tag_type; RETURN true; };");
const CREATE_AI_SUGGESTION: &str = concat!("RETURN { ",atelier_event_sql!()," CREATE $domain.rid CONTENT {suggestion_id:$domain.suggestion_id,character_internal_id:$domain.character_ref,asset_id:$domain.asset_ref,tag_text:$domain.tag_text,confidence:$domain.confidence,model_receipt_ref:$domain.model_receipt_ref,tool_receipt_ref:$domain.tool_receipt_ref,suggested_by:$domain.suggested_by,status:'proposed'}; RETURN (SELECT ",ai_suggestion_select!()," FROM $domain.rid); };");
const DECIDE_AI_SUGGESTION: &str = concat!("RETURN { ",atelier_event_sql!()," UPDATE $domain.rid SET status=$domain.status,decided_by=$domain.decided_by,decision_reason=$domain.decision_reason,updated_at_utc=time::now(); RETURN (SELECT ",ai_suggestion_select!()," FROM $domain.rid); };");
const APPLY_AI_SUGGESTION: &str = concat!("RETURN { ",atelier_event_sql!()," UPSERT $domain.link_rid SET character_internal_id=$domain.character_ref,tag_id=$domain.tag_ref,tag_type='manual'; UPDATE $domain.rid SET status='applied',decided_by=decided_by ?? $domain.applied_by,applied_tag_id=$domain.tag_ref,updated_at_utc=time::now(); RETURN (SELECT ",ai_suggestion_select!()," FROM $domain.rid); };");

impl AtelierStore {
    /// Search across sheet text, character documents, moodboard snapshots, and
    /// media rows with stable jump targets. This is database-backed pattern
    /// matching over Handshake tables, not SQLite FTS or an external index.
    pub async fn global_search(
        &self,
        query: &str,
        limit: i64,
    ) -> AtelierResult<Vec<GlobalSearchHit>> {
        self.global_search_with_lens_filters(query, limit, LensSearchFilters::default())
            .await
    }

    pub async fn global_search_with_lens_filters(
        &self,
        query: &str,
        limit: i64,
        filters: LensSearchFilters,
    ) -> AtelierResult<Vec<GlobalSearchHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(AtelierError::Validation(
                "global search query must not be empty".into(),
            ));
        }
        let limit = limit.clamp(1, 50);
        let bindings = QueryLimitBindings {
            query: trimmed.to_ascii_lowercase(),
            limit,
        };
        let rows: Vec<GlobalSearchCandidateRow> = self.store().with_data_operation(move |ctx| Box::pin(async move {
            ctx.query_values("RETURN array::slice(array::sort::asc(array::concat(\
              (SELECT 'sheet' AS target_kind, <string>version_id AS target_id, string::concat('atelier://sheet/', <string>record::id(character_internal_id), '/', <string>version_id) AS jump_target, string::concat('Sheet v', <string>seq, ' - ', character_internal_id.display_name) AS title, raw_text AS search_text, 10 AS rank, created_at_utc AS sort_at FROM atelier_sheet_version WHERE string::lowercase(raw_text) CONTAINS $query), \
              (SELECT doc_type AS target_kind, <string>document_id AS target_id, string::concat('atelier://document/', <string>document_id) AS jump_target, current_version_id.title AS title, string::concat(current_version_id.title, ' ', current_version_id.body_raw_text, ' ', <string>tags_json) AS search_text, 20 AS rank, current_version_id.created_at_utc AS sort_at FROM atelier_character_document WHERE string::lowercase(string::concat(current_version_id.title, ' ', current_version_id.body_raw_text, ' ', <string>tags_json)) CONTAINS $query), \
              (SELECT 'moodboard_snapshot' AS target_kind, <string>snapshot_id AS target_id, string::concat('atelier://moodboard/', <string>snapshot_id) AS jump_target, moodboard_json.name ?? 'Moodboard' AS title, raw_json_text AS search_text, 30 AS rank, created_at_utc AS sort_at FROM atelier_moodboard WHERE string::lowercase(raw_json_text) CONTAINS $query), \
              (SELECT 'image' AS target_kind, <string>asset_id AS target_id, string::concat('atelier://image/', <string>asset_id) AS jump_target, string::concat(mime, ' ', string::slice(content_hash, 0, 12)) AS title, string::concat(mime, ' ', content_hash, ' ', source_provenance ?? '', ' ', artifact_ref) AS search_text, 40 AS rank, created_at_utc AS sort_at FROM atelier_media_asset WHERE string::lowercase(string::concat(mime, ' ', content_hash, ' ', source_provenance ?? '', ' ', artifact_ref)) CONTAINS $query)\
            ), true), 0, $limit);", bindings).await
        })).await?;
        let mut hits: Vec<GlobalSearchHit> = rows
            .into_iter()
            .map(|row| global_search_hit_from_row(row, trimmed, filters.view_mode))
            .collect::<AtelierResult<_>>()?;
        hits.retain(|hit| {
            hit.extraction_tier.rank() <= filters.extraction_tier.rank()
                && (filters.view_mode != LensViewMode::Sfw
                    || hit.content_tier == Some(LensContentTier::Sfw))
        });
        hits.sort_by(|a, b| {
            a.rank
                .cmp(&b.rank)
                .then_with(|| a.target_id.cmp(&b.target_id))
        });
        hits.truncate(limit as usize);
        Ok(hits)
    }

    pub async fn save_saved_search(&self, new: &NewSavedSearch) -> AtelierResult<SavedSearch> {
        let name = new.name.trim();
        if name.is_empty() || name != new.name {
            return Err(AtelierError::Validation(
                "saved search name must not be empty or padded".into(),
            ));
        }
        let created_by = new.created_by.trim();
        if created_by.is_empty() || created_by != new.created_by {
            return Err(AtelierError::Validation(
                "saved search created_by must not be empty or padded".into(),
            ));
        }
        let filters = normalize_saved_search_filters(&new.filters)?;
        if let SavedSearchScope::Collection(collection_id) = filters.scope {
            let exists: Option<bool> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            "RETURN record::exists($value);",
                            RecordBinding {
                                value: RecordId::new(
                                    "atelier_collection",
                                    SurrealUuid::from(collection_id),
                                ),
                            },
                        )
                        .await
                    })
                })
                .await?;
            if !exists.unwrap_or(false) {
                return Err(AtelierError::NotFound(format!(
                    "saved search collection scope not found: {collection_id}"
                )));
            }
        }
        let include_tags_json = serde_json::Value::from(filters.include_tags.clone());
        let exclude_tags_json = serde_json::Value::from(filters.exclude_tags.clone());
        let (scope_kind, scope_id) = filters.scope.into_parts();
        let existing_id: Option<SurrealUuid> = self.store().with_data_operation({ let b=TextBinding{text:name.to_owned()}; move |ctx| Box::pin(async move { ctx.query_first("SELECT VALUE saved_search_id FROM atelier_saved_search WHERE name=$text LIMIT 1;",b).await })}).await?;
        let saved_search_id: Uuid = existing_id.map(Into::into).unwrap_or_else(Uuid::now_v7);
        let bindings = SavedSearchBindings {
            rid: RecordId::new("atelier_saved_search", SurrealUuid::from(saved_search_id)),
            saved_search_id: saved_search_id.into(),
            name: name.to_owned(),
            include_tags_json,
            exclude_tags_json,
            min_rating: filters.min_rating.map(i64::from),
            favorite: filters.favorite,
            color_hex: filters.color_hex.clone(),
            scope_kind: scope_kind.to_owned(),
            scope_id: scope_id.map(Into::into),
            view_mode: filters.view_mode.as_str().to_owned(),
            created_by: created_by.to_owned(),
        };
        let row: Option<SavedSearchRow> = self
            .write_with_event(
                UPSERT_SAVED_SEARCH,
                bindings,
                search_event_family::SAVED_SEARCH_UPSERTED,
                "atelier_saved_search",
                &saved_search_id.to_string(),
                serde_json::json!({
                    "saved_search_id": saved_search_id, "name": name,
                    "include_tags": filters.include_tags, "exclude_tags": filters.exclude_tags,
                    "min_rating": filters.min_rating, "favorite": filters.favorite,
                    "color_hex": filters.color_hex, "scope": filters.scope,
                    "view_mode": filters.view_mode,
                    "created_by": created_by,
                }),
            )
            .await?;
        saved_search_from_row(
            row.ok_or_else(|| {
                AtelierError::Internal("saved search upsert returned no row".into())
            })?,
        )
    }

    pub async fn get_saved_search(
        &self,
        saved_search_id: Uuid,
    ) -> AtelierResult<Option<SavedSearch>> {
        let row: Option<SavedSearchRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        concat!(
                            "SELECT ",
                            saved_search_select!(),
                            " FROM atelier_saved_search WHERE saved_search_id=$value LIMIT 1;"
                        ),
                        UuidBinding {
                            value: saved_search_id.into(),
                        },
                    )
                    .await
                })
            })
            .await?;
        row.map(saved_search_from_row).transpose()
    }

    pub async fn list_saved_searches(&self) -> AtelierResult<Vec<SavedSearch>> {
        let rows: Vec<SavedSearchRow> = self
            .store()
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        concat!(
                            "SELECT ",
                            saved_search_select!(),
                            " FROM atelier_saved_search ORDER BY updated_at_utc DESC, name ASC;"
                        ),
                        EmptyBindings {},
                    )
                    .await
                })
            })
            .await?;
        rows.into_iter().map(saved_search_from_row).collect()
    }

    pub async fn delete_saved_search(&self, saved_search_id: Uuid) -> AtelierResult<bool> {
        let removed: Option<bool> = self.store().with_data_operation(move |ctx| Box::pin(async move { ctx.query_first("RETURN { LET $rid=type::record('atelier_saved_search',$value); LET $exists=record::exists($rid); IF $exists { DELETE $rid; }; RETURN $exists; };",UuidBinding{value:saved_search_id.into()}).await })).await?;
        if !removed.unwrap_or(false) {
            return Ok(false);
        }
        self.record_event(
            search_event_family::SAVED_SEARCH_DELETED,
            "atelier_saved_search",
            &saved_search_id.to_string(),
            serde_json::json!({ "saved_search_id": saved_search_id }),
        )
        .await?;
        Ok(true)
    }

    pub async fn run_saved_search(
        &self,
        saved_search_id: Uuid,
        limit: i64,
    ) -> AtelierResult<Vec<SavedSearchProjectionHit>> {
        if self.get_saved_search(saved_search_id).await?.is_none() {
            return Err(AtelierError::NotFound(format!(
                "saved_search_id={saved_search_id}"
            )));
        }
        let limit = limit.clamp(1, 100);
        let saved = self
            .get_saved_search(saved_search_id)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("saved_search_id={saved_search_id}")))?;
        #[derive(SurrealValue)]
        struct RunSavedBindings {
            saved_search_id: SurrealUuid,
            limit: i64,
        }
        let rows: Vec<SavedSearchProjectionRow> = self.store().with_data_operation(move |ctx| Box::pin(async move {
            ctx.query_values("SELECT $saved_search_id AS saved_search_id, asset_id, content_hash, artifact_ref, string::concat('atelier://image/',<string>asset_id) AS jump_target, (SELECT VALUE tag_id.text FROM atelier_media_asset_tag WHERE asset_id=$parent.id) AS tags_json, (SELECT VALUE favorite FROM atelier_media_review_metadata WHERE asset_id=$parent.id LIMIT 1)[0] ?? false AS favorite, (SELECT VALUE rating FROM atelier_media_review_metadata WHERE asset_id=$parent.id LIMIT 1)[0] ?? 0 AS rating, NONE AS matched_color_hex, NONE AS content_tier, 'NSFW' AS view_mode FROM atelier_media_asset ORDER BY created_at_utc DESC LIMIT $limit;", RunSavedBindings{saved_search_id:saved_search_id.into(),limit}).await
        })).await?;
        let mut hits: Vec<SavedSearchProjectionHit> = rows
            .into_iter()
            .map(saved_search_projection_hit_from_row)
            .collect::<AtelierResult<_>>()?;
        hits.retain(|hit| {
            saved
                .filters
                .include_tags
                .iter()
                .all(|tag| hit.tags.contains(tag))
                && saved
                    .filters
                    .exclude_tags
                    .iter()
                    .all(|tag| !hit.tags.contains(tag))
                && saved
                    .filters
                    .min_rating
                    .is_none_or(|rating| hit.rating >= rating)
                && saved
                    .filters
                    .favorite
                    .is_none_or(|favorite| hit.favorite == favorite)
        });
        hits.truncate(limit as usize);
        Ok(hits)
    }

    // ----- Tag dictionary -------------------------------------------------

    /// Ensure a tag exists in the dictionary (deduped by normalized text) and
    /// return it. Idempotent: re-ensuring identical text returns the same row.
    /// Mirrors legacy source `_ensureTag`.
    pub async fn ensure_tag(&self, text: &str) -> AtelierResult<Tag> {
        let norm = normalize_tag(text);
        if norm.is_empty() {
            return Err(AtelierError::Validation(
                "tag text must not be empty".into(),
            ));
        }
        let existing: Option<TagRow> = self.store().with_data_operation({ let b=TextBinding{text:norm.clone()}; move |ctx| Box::pin(async move { ctx.query_first("SELECT tag_id,text,created_at_utc FROM atelier_tag WHERE text=$text LIMIT 1;",b).await })}).await?;
        if let Some(row) = existing {
            return Ok(tag_from_row(row));
        }
        let tag_id = Uuid::now_v7();
        #[derive(SurrealValue)]
        struct CreateTagBindings {
            rid: RecordId,
            tag_id: SurrealUuid,
            text: String,
        }
        let row: Option<TagRow> = self.store().with_data_operation(move |ctx| Box::pin(async move { ctx.query_first("RETURN { CREATE $rid CONTENT {tag_id:$tag_id,text:$text}; RETURN (SELECT tag_id,text,created_at_utc FROM $rid); };",CreateTagBindings{rid:RecordId::new("atelier_tag",SurrealUuid::from(tag_id)),tag_id:tag_id.into(),text:norm}).await })).await?;
        Ok(tag_from_row(row.ok_or_else(|| {
            AtelierError::Internal("creating tag returned no row".into())
        })?))
    }

    /// List every tag in the dictionary (ascending text). Mirrors the operator
    /// "all tags" picker in legacy source `listAllTags`.
    pub async fn list_all_tags(&self) -> AtelierResult<Vec<Tag>> {
        let rows: Vec<TagRow> = self
            .store()
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        "SELECT tag_id,text,created_at_utc FROM atelier_tag ORDER BY text ASC;",
                        EmptyBindings {},
                    )
                    .await
                })
            })
            .await?;
        Ok(rows.into_iter().map(tag_from_row).collect())
    }

    // ----- Manual / bulk tagging -----------------------------------------

    /// Attach a tag to a character with an explicit provenance. Idempotent on
    /// the (character, tag) pair; emits `CHARACTER_TAGGED`. Mirrors legacy source
    /// `addManualTag` (here generalized over [`TagType`]).
    pub async fn tag_character(
        &self,
        character_internal_id: Uuid,
        text: &str,
        tag_type: TagType,
    ) -> AtelierResult<CharacterTag> {
        let tag = self.ensure_tag(text).await?;
        let bindings = TagCharacterBindings {
            rid: RecordId::new(
                "atelier_character_tag",
                surrealdb::types::Array::from(vec![
                    SurrealUuid::from(character_internal_id),
                    SurrealUuid::from(tag.tag_id),
                ]),
            ),
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
            tag_ref: RecordId::new("atelier_tag", SurrealUuid::from(tag.tag_id)),
            tag_type: tag_type.as_str().to_owned(),
        };
        let _: Option<bool> = self
            .write_with_event(
                TAG_CHARACTER_STATEMENT,
                bindings,
                search_event_family::CHARACTER_TAGGED,
                "atelier_character_tag",
                &event_ref_for_text(&format!(
                    "character-tag:{}:{}",
                    character_internal_id, tag.tag_id
                )),
                serde_json::json!({
                    "character_internal_id": character_internal_id,
                    "tag_id": tag.tag_id,
                    "text": tag.text,
                    "tag_type": tag_type.as_str(),
                }),
            )
            .await?;

        Ok(CharacterTag {
            character_internal_id,
            tag_id: tag.tag_id,
            text: tag.text,
            tag_type,
        })
    }

    /// Bulk-apply a set of manual tags to many characters in one transaction.
    /// Returns the number of (character, tag) links written or refreshed. A
    /// single `CHARACTER_TAGGED` bulk event is recorded. Mirrors the legacy source
    /// `batchUpdateCharacterTags` operator workflow.
    pub async fn bulk_tag_characters(
        &self,
        character_internal_ids: &[Uuid],
        texts: &[String],
    ) -> AtelierResult<i64> {
        if character_internal_ids.is_empty() || texts.is_empty() {
            return Ok(0);
        }
        let receipt = self
            .bulk_tag_characters_with_receipt(&BulkTagRequest {
                character_internal_ids: character_internal_ids.to_vec(),
                tags: texts.to_vec(),
                requested_by: "legacy_bulk_tag_characters".to_string(),
            })
            .await?;
        Ok(receipt.mutation_count)
    }

    /// Detach a manual tag from a character. No-op if the tag/link does not
    /// exist. Emits `CHARACTER_UNTAGGED` when a link was removed. Mirrors legacy source
    /// `removeManualTag` (only removes `manual` links, never `derived`).
    pub async fn untag_character(
        &self,
        character_internal_id: Uuid,
        text: &str,
    ) -> AtelierResult<bool> {
        let norm = normalize_tag(text);
        #[derive(SurrealValue)]
        struct UntagBindings {
            character_ref: RecordId,
            text: String,
        }
        let removed: Option<bool> = self.store().with_data_operation({let b=UntagBindings{character_ref:RecordId::new("atelier_character",SurrealUuid::from(character_internal_id)),text:norm.clone()}; move |ctx| Box::pin(async move {ctx.query_first("RETURN { LET $tags=(SELECT VALUE id FROM atelier_tag WHERE text=$text); LET $rows=(SELECT VALUE id FROM atelier_character_tag WHERE character_internal_id=$character_ref AND tag_id IN $tags AND tag_type='manual'); IF array::len($rows)>0 { DELETE atelier_character_tag WHERE id IN $rows; }; RETURN array::len($rows)>0; };",b).await})}).await?;
        if !removed.unwrap_or(false) {
            return Ok(false);
        }

        self.record_event(
            search_event_family::CHARACTER_UNTAGGED,
            "atelier_character_tag",
            &event_ref_for_text(&format!(
                "character-untag:{}:{}",
                character_internal_id, norm
            )),
            serde_json::json!({
                "character_internal_id": character_internal_id,
                "text": norm,
                "tag_type": "manual"
            }),
        )
        .await?;
        Ok(true)
    }

    /// List a character's tags (ascending text), both manual and derived.
    pub async fn list_character_tags(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<CharacterTag>> {
        let rows:Vec<CharacterTagRow>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_values("SELECT record::id(character_internal_id) AS character_internal_id,record::id(tag_id) AS tag_id,tag_id.text AS text,tag_type FROM atelier_character_tag WHERE character_internal_id=$value ORDER BY text ASC;",RecordBinding{value:RecordId::new("atelier_character",SurrealUuid::from(character_internal_id))}).await})).await?;
        rows.into_iter().map(character_tag_from_row).collect()
    }

    // ----- AI tag suggestions --------------------------------------------

    /// Record an AI/model tag proposal. This never attaches a tag to the
    /// character; accept/apply are explicit follow-up decisions.
    pub async fn record_ai_tag_suggestion(
        &self,
        new: &NewAiTagSuggestion,
    ) -> AtelierResult<AiTagSuggestion> {
        let tag_text = normalize_tag(&new.tag_text);
        if tag_text.is_empty() {
            return Err(AtelierError::Validation(
                "AI tag suggestion tag_text must not be empty".into(),
            ));
        }
        let confidence = normalize_ai_tag_suggestion_confidence(new.confidence)?;
        validate_ai_tag_receipt_ref("model_receipt_ref", &new.model_receipt_ref)?;
        validate_ai_tag_receipt_ref("tool_receipt_ref", &new.tool_receipt_ref)?;
        let suggested_by = require_ai_tag_actor("suggested_by", &new.suggested_by)?;

        let suggestion_id = Uuid::now_v7();
        let bindings = AiSuggestionBindings {
            rid: RecordId::new(
                "atelier_ai_tag_suggestion",
                SurrealUuid::from(suggestion_id),
            ),
            suggestion_id: suggestion_id.into(),
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.character_internal_id),
            ),
            asset_ref: new
                .asset_id
                .map(|id| RecordId::new("atelier_media_asset", SurrealUuid::from(id))),
            tag_text: tag_text.clone(),
            confidence,
            model_receipt_ref: new.model_receipt_ref.clone(),
            tool_receipt_ref: new.tool_receipt_ref.clone(),
            suggested_by: suggested_by.to_owned(),
        };
        let row:Option<AiTagSuggestionRow>=self.write_with_event(
            CREATE_AI_SUGGESTION,
            bindings,
            search_event_family::AI_TAG_SUGGESTION_RECORDED,
            "atelier_ai_tag_suggestion",
            &suggestion_id.to_string(),
            serde_json::json!({
                "suggestion_id": suggestion_id,"character_internal_id":new.character_internal_id,
                "asset_id":new.asset_id,"tag_text":tag_text,"confidence":confidence,
                "model_receipt_ref":new.model_receipt_ref,"tool_receipt_ref":new.tool_receipt_ref,
                "suggested_by":suggested_by,"status":"proposed",
            }),
        ).await?;
        ai_tag_suggestion_from_row(
            row.ok_or_else(|| {
                AtelierError::Internal("AI suggestion create returned no row".into())
            })?,
        )
    }

    pub async fn list_ai_tag_suggestions_for_character(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<AiTagSuggestion>> {
        let rows:Vec<AiTagSuggestionRow>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_values(concat!("SELECT ",ai_suggestion_select!()," FROM atelier_ai_tag_suggestion WHERE character_internal_id=$value ORDER BY created_at_utc,suggestion_id;"),RecordBinding{value:RecordId::new("atelier_character",SurrealUuid::from(character_internal_id))}).await})).await?;
        rows.into_iter().map(ai_tag_suggestion_from_row).collect()
    }

    async fn get_ai_tag_suggestion_record(
        &self,
        suggestion_id: Uuid,
    ) -> AtelierResult<Option<AiTagSuggestion>> {
        let row: Option<AiTagSuggestionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        concat!(
                            "SELECT ",
                            ai_suggestion_select!(),
                            " FROM atelier_ai_tag_suggestion WHERE suggestion_id=$value LIMIT 1;"
                        ),
                        UuidBinding {
                            value: suggestion_id.into(),
                        },
                    )
                    .await
                })
            })
            .await?;
        row.map(ai_tag_suggestion_from_row).transpose()
    }

    pub async fn accept_ai_tag_suggestion(
        &self,
        decision: &AiTagSuggestionDecision,
    ) -> AtelierResult<AiTagSuggestion> {
        self.decide_ai_tag_suggestion(
            decision,
            AiTagSuggestionStatus::Accepted,
            search_event_family::AI_TAG_SUGGESTION_ACCEPTED,
        )
        .await
    }

    pub async fn reject_ai_tag_suggestion(
        &self,
        decision: &AiTagSuggestionDecision,
    ) -> AtelierResult<AiTagSuggestion> {
        self.decide_ai_tag_suggestion(
            decision,
            AiTagSuggestionStatus::Rejected,
            search_event_family::AI_TAG_SUGGESTION_REJECTED,
        )
        .await
    }

    async fn decide_ai_tag_suggestion(
        &self,
        decision: &AiTagSuggestionDecision,
        next_status: AiTagSuggestionStatus,
        event_family: &str,
    ) -> AtelierResult<AiTagSuggestion> {
        let decided_by = require_ai_tag_actor("decided_by", &decision.decided_by)?;
        let reason = normalize_ai_tag_reason(&decision.reason)?;
        let current = self
            .get_ai_tag_suggestion_record(decision.suggestion_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("ai tag suggestion_id={}", decision.suggestion_id))
            })?;
        if current.status != AiTagSuggestionStatus::Proposed {
            return Err(AtelierError::Validation(format!(
                "AI tag suggestion {} is not proposed (status={})",
                decision.suggestion_id,
                current.status.as_str()
            )));
        }
        let bindings = AiDecisionBindings {
            rid: RecordId::new(
                "atelier_ai_tag_suggestion",
                SurrealUuid::from(decision.suggestion_id),
            ),
            status: next_status.as_str().to_owned(),
            decided_by: decided_by.to_owned(),
            decision_reason: reason.clone(),
        };
        let row:Option<AiTagSuggestionRow>=self.write_with_event(
            DECIDE_AI_SUGGESTION,
            bindings,
            event_family,
            "atelier_ai_tag_suggestion",
            &decision.suggestion_id.to_string(),
            serde_json::json!({
                "suggestion_id":decision.suggestion_id,"character_internal_id":current.character_internal_id,
                "asset_id":current.asset_id,"tag_text":current.tag_text,"status":next_status.as_str(),
                "decided_by":decided_by,"decision_reason_ref":reason.as_ref().map(|value|event_ref_for_text(value)),
            }),
        ).await?;
        ai_tag_suggestion_from_row(row.ok_or_else(|| {
            AtelierError::Internal("AI suggestion decision returned no row".into())
        })?)
    }

    pub async fn apply_ai_tag_suggestion(
        &self,
        suggestion_id: Uuid,
        applied_by: &str,
    ) -> AtelierResult<AiTagSuggestion> {
        let applied_by = require_ai_tag_actor("applied_by", applied_by)?;
        let current = self
            .get_ai_tag_suggestion_record(suggestion_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("ai tag suggestion_id={suggestion_id}"))
            })?;
        match current.status {
            AiTagSuggestionStatus::Applied => return Ok(current),
            AiTagSuggestionStatus::Accepted => {}
            status => {
                return Err(AtelierError::Validation(format!(
                    "AI tag suggestion {suggestion_id} must be accepted before apply (status={})",
                    status.as_str()
                )));
            }
        }

        let tag = self.ensure_tag(&current.tag_text).await?;
        let tag_id = tag.tag_id;
        let bindings = ApplyAiBindings {
            rid: RecordId::new(
                "atelier_ai_tag_suggestion",
                SurrealUuid::from(suggestion_id),
            ),
            link_rid: RecordId::new(
                "atelier_character_tag",
                surrealdb::types::Array::from(vec![
                    SurrealUuid::from(current.character_internal_id),
                    SurrealUuid::from(tag_id),
                ]),
            ),
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(current.character_internal_id),
            ),
            tag_ref: RecordId::new("atelier_tag", SurrealUuid::from(tag_id)),
            applied_by: applied_by.to_owned(),
        };
        let row:Option<AiTagSuggestionRow>=self.write_with_event(
            APPLY_AI_SUGGESTION,
            bindings,
            search_event_family::AI_TAG_SUGGESTION_APPLIED,
            "atelier_ai_tag_suggestion",
            &suggestion_id.to_string(),
            serde_json::json!({
                "suggestion_id":suggestion_id,"character_internal_id":current.character_internal_id,
                "asset_id":current.asset_id,"tag_id":tag_id,"tag_text":current.tag_text,
                "status":"applied","applied_by":applied_by,
            }),
        ).await?;
        ai_tag_suggestion_from_row(
            row.ok_or_else(|| {
                AtelierError::Internal("AI suggestion apply returned no row".into())
            })?,
        )
    }

    // ----- Saved tag rules -----------------------------------------------

    /// Create a saved tag rule. Emits `TAG_RULE_UPSERTED`. Mirrors legacy source
    /// `createTagRule`.
    pub async fn create_tag_rule(&self, new: &NewTagRule) -> AtelierResult<TagRule> {
        if new.source_field_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "tag rule source_field_id must not be empty".into(),
            ));
        }
        if new.emit_tag.trim().is_empty() {
            return Err(AtelierError::Validation(
                "tag rule emit_tag must not be empty".into(),
            ));
        }
        let rule_id = Uuid::now_v7();
        let emit_tag = normalize_tag(&new.emit_tag);
        let bindings = TagRuleBindings {
            rid: RecordId::new("atelier_tag_rule", SurrealUuid::from(rule_id)),
            rule_id: rule_id.into(),
            source_field_id: new.source_field_id.clone(),
            match_type: new.match_type.as_str().to_owned(),
            pattern: new.pattern.clone(),
            emit_tag: emit_tag.clone(),
            enabled: new.enabled,
        };
        let row: Option<TagRuleRow> = self
            .write_with_event(
                UPSERT_TAG_RULE,
                bindings,
                search_event_family::TAG_RULE_UPSERTED,
                "atelier_tag_rule",
                &rule_id.to_string(),
                serde_json::json!({
                    "rule_id":rule_id,"source_field_id":new.source_field_id,
                    "match_type":new.match_type.as_str(),"emit_tag":emit_tag,"op":"create",
                }),
            )
            .await?;
        rule_from_row(
            row.ok_or_else(|| AtelierError::Internal("tag rule create returned no row".into()))?,
        )
    }

    /// List saved tag rules in deterministic order (`rule_id ASC`), matching the
    /// legacy source `_upsertDerivedTags` ordering so derived tags are reproducible.
    pub async fn list_tag_rules(&self) -> AtelierResult<Vec<TagRule>> {
        let rows: Vec<TagRuleRow> = self
            .store()
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        concat!(
                            "SELECT ",
                            rule_select!(),
                            " FROM atelier_tag_rule ORDER BY rule_id ASC;"
                        ),
                        EmptyBindings {},
                    )
                    .await
                })
            })
            .await?;
        rows.into_iter().map(rule_from_row).collect()
    }

    /// Delete a saved tag rule. Emits `TAG_RULE_DELETED` when a row was removed.
    /// Mirrors legacy source `deleteTagRule`.
    pub async fn delete_tag_rule(&self, rule_id: Uuid) -> AtelierResult<bool> {
        let removed:Option<bool>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_first("RETURN { LET $rid=type::record('atelier_tag_rule',$value);LET $exists=record::exists($rid);IF $exists {DELETE $rid;};RETURN $exists;};",UuidBinding{value:rule_id.into()}).await})).await?;
        if !removed.unwrap_or(false) {
            return Ok(false);
        }
        self.record_event(
            search_event_family::TAG_RULE_DELETED,
            "atelier_tag_rule",
            &rule_id.to_string(),
            serde_json::json!({ "rule_id": rule_id }),
        )
        .await?;
        Ok(true)
    }

    /// Recompute a character's derived tags from the saved rule set against the
    /// supplied field values (`field_id -> value`). Rules run deterministically
    /// ordered by `rule_id ASC`; all prior `derived` links are cleared then the
    /// rule output is re-inserted, exactly like legacy source `_upsertDerivedTags`. The
    /// regex match type uses the `regex` crate; invalid patterns are ignored
    /// deterministically (mirroring the legacy source try/catch). Returns the sorted list
    /// of emitted derived tag texts. Emits `DERIVED_TAGS_RECOMPUTED`.
    pub async fn recompute_derived_tags(
        &self,
        character_internal_id: Uuid,
        values_by_field: &std::collections::HashMap<String, String>,
    ) -> AtelierResult<Vec<String>> {
        let rules = self.list_tag_rules().await?;

        let mut emitted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for rule in &rules {
            if !rule.enabled {
                continue;
            }
            let Some(val) = values_by_field.get(&rule.source_field_id) else {
                continue;
            };
            if val.is_empty() {
                continue;
            }
            let matched = match rule.match_type {
                MatchType::Equals => val == &rule.pattern,
                MatchType::Contains => val.contains(&rule.pattern),
                MatchType::Regex => match regex::Regex::new(&rule.pattern) {
                    Ok(re) => re.is_match(val),
                    Err(_) => false,
                },
            };
            if matched {
                emitted.insert(normalize_tag(&rule.emit_tag));
            }
        }

        let mut items = Vec::new();
        for text in &emitted {
            let tag = self.ensure_tag(text).await?;
            items.push(DerivedTagInput {
                rid: RecordId::new(
                    "atelier_character_tag",
                    surrealdb::types::Array::from(vec![
                        SurrealUuid::from(character_internal_id),
                        SurrealUuid::from(tag.tag_id),
                    ]),
                ),
                tag_ref: RecordId::new("atelier_tag", SurrealUuid::from(tag.tag_id)),
            });
        }
        self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_first::<bool,_>("RETURN { DELETE atelier_character_tag WHERE character_internal_id=$character_ref AND tag_type='derived';FOR $item IN $items {UPSERT $item.rid SET character_internal_id=$character_ref,tag_id=$item.tag_ref,tag_type='derived';};RETURN true;};",DerivedTagsBindings{character_ref:RecordId::new("atelier_character",SurrealUuid::from(character_internal_id)),items}).await})).await?;

        let derived: Vec<String> = emitted.into_iter().collect();
        self.record_event(
            search_event_family::DERIVED_TAGS_RECOMPUTED,
            "atelier_character_tag",
            &event_ref_for_text(&format!("character-derived-tags:{}", character_internal_id)),
            serde_json::json!({
                "character_internal_id": character_internal_id,
                "derived_count": derived.len(),
                "derived_tags": derived,
            }),
        )
        .await?;
        Ok(derived)
    }

    // ----- Similarity projections (dHash + palette) ----------------------

    /// Upsert the similarity projection (perceptual hash + dominant palette) for
    /// a media asset. Idempotent on `asset_internal_id`. Validates the dHash is
    /// 64-bit hex (legacy source `isHex64`) when present. Emits `SIMILARITY_PROJECTED`.
    /// Mirrors legacy source persistence of `dhash_hex` / `palette_json` on the asset.
    pub async fn upsert_similarity_projection(
        &self,
        asset_internal_id: Uuid,
        dhash_hex: Option<&str>,
        palette: serde_json::Value,
    ) -> AtelierResult<SimilarityProjection> {
        let normalized_hash = match dhash_hex {
            Some(h) => {
                let lowered = h.trim().to_ascii_lowercase();
                if !is_hex64(&lowered) {
                    return Err(AtelierError::Validation(format!(
                        "dhash_hex must be 16 hex chars (64 bits), got: {h:?}"
                    )));
                }
                Some(lowered)
            }
            None => None,
        };

        let bindings = ProjectionBindings {
            rid: RecordId::new(
                "atelier_similarity_projection",
                SurrealUuid::from(asset_internal_id),
            ),
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_internal_id)),
            dhash_hex: normalized_hash.clone(),
            palette_json: palette,
        };
        let row: Option<SimilarityProjectionRow> = self
            .write_with_event(
                UPSERT_PROJECTION,
                bindings,
                search_event_family::SIMILARITY_PROJECTED,
                "atelier_similarity_projection",
                &asset_internal_id.to_string(),
                serde_json::json!({
                    "asset_internal_id": asset_internal_id,
                    "has_dhash": normalized_hash.is_some(),
                }),
            )
            .await?;
        Ok(row
            .ok_or_else(|| {
                AtelierError::Internal("similarity projection upsert returned no row".into())
            })?
            .into())
    }

    /// Decode image bytes, compute a deterministic 64-bit dHash plus bounded
    /// dominant-palette JSON, then persist the existing similarity projection row.
    pub async fn project_similarity_from_image_bytes(
        &self,
        asset_internal_id: Uuid,
        image_bytes: &[u8],
    ) -> AtelierResult<SimilarityProjection> {
        let (dhash_hex, palette_json) = compute_similarity_from_image_bytes(image_bytes)?;
        self.upsert_similarity_projection(asset_internal_id, Some(&dhash_hex), palette_json)
            .await
    }

    /// Run a single-asset similarity rebuild job from image bytes. The job row is
    /// durable even when image decoding fails; bytes remain caller-owned.
    pub async fn rebuild_similarity_projection_from_image_bytes(
        &self,
        asset_internal_id: Uuid,
        image_bytes: &[u8],
        requested_by: &str,
    ) -> AtelierResult<SimilarityRebuildJob> {
        let requested_by = require_similarity_rebuild_actor(requested_by)?;
        let job_id = Uuid::now_v7();
        let create = RebuildJobBindings {
            rid: RecordId::new("atelier_similarity_rebuild_job", SurrealUuid::from(job_id)),
            job_id: job_id.into(),
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_internal_id)),
            requested_by: requested_by.to_owned(),
        };
        let row:Option<SimilarityRebuildJobRow>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_first(concat!("RETURN { CREATE $rid CONTENT {job_id:$job_id,asset_internal_id:$asset_ref,status:'running',requested_by:$requested_by};RETURN (SELECT ",rebuild_select!()," FROM $rid);};"),create).await})).await?;
        let running = similarity_rebuild_job_from_row(row.ok_or_else(|| {
            AtelierError::Internal("similarity rebuild create returned no row".into())
        })?)?;

        let computed = compute_similarity_from_image_bytes(image_bytes);
        let (dhash_hex, palette_json) = match computed {
            Ok(value) => value,
            Err(err) => {
                let error_ref =
                    event_ref_for_text(&format!("similarity-rebuild:{}:{err}", running.job_id));
                let b = UpdateRebuildBindings {
                    rid: RecordId::new(
                        "atelier_similarity_rebuild_job",
                        SurrealUuid::from(running.job_id),
                    ),
                    status: "failed".into(),
                    processed_count: 0,
                    failed_count: 1,
                    dhash_hex: None,
                    palette_json: None,
                    error_ref: Some(error_ref.clone()),
                };
                let row:Option<SimilarityRebuildJobRow>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_first(concat!("RETURN { UPDATE $rid SET status=$status,processed_count=$processed_count,failed_count=$failed_count,dhash_hex=$dhash_hex,palette_json=$palette_json,error_ref=$error_ref,updated_at_utc=time::now();RETURN (SELECT ",rebuild_select!()," FROM $rid);};"),b).await})).await?;
                let failed = similarity_rebuild_job_from_row(row.ok_or_else(|| {
                    AtelierError::Internal(
                        "similarity rebuild failure update returned no row".into(),
                    )
                })?)?;
                self.record_event(
                    search_event_family::SIMILARITY_REBUILD_FAILED,
                    "atelier_similarity_rebuild_job",
                    &failed.job_id.to_string(),
                    serde_json::json!({
                        "job_id": failed.job_id,
                        "asset_internal_id": failed.asset_internal_id,
                        "status": failed.status.as_str(),
                        "requested_by": failed.requested_by,
                        "failed_count": failed.failed_count,
                        "error_ref": error_ref,
                    }),
                )
                .await?;
                return Ok(failed);
            }
        };

        let projection = self
            .upsert_similarity_projection(asset_internal_id, Some(&dhash_hex), palette_json.clone())
            .await?;
        let b = UpdateRebuildBindings {
            rid: RecordId::new(
                "atelier_similarity_rebuild_job",
                SurrealUuid::from(running.job_id),
            ),
            status: "completed".into(),
            processed_count: 1,
            failed_count: 0,
            dhash_hex: Some(dhash_hex.clone()),
            palette_json: Some(projection.palette_json.clone()),
            error_ref: None,
        };
        let row:Option<SimilarityRebuildJobRow>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_first(concat!("RETURN { UPDATE $rid SET status=$status,processed_count=$processed_count,failed_count=$failed_count,dhash_hex=$dhash_hex,palette_json=$palette_json,error_ref=$error_ref,updated_at_utc=time::now();RETURN (SELECT ",rebuild_select!()," FROM $rid);};"),b).await})).await?;
        let completed = similarity_rebuild_job_from_row(row.ok_or_else(|| {
            AtelierError::Internal("similarity rebuild completion returned no row".into())
        })?)?;
        self.record_event(
            search_event_family::SIMILARITY_REBUILD_COMPLETED,
            "atelier_similarity_rebuild_job",
            &completed.job_id.to_string(),
            serde_json::json!({
                "job_id": completed.job_id,
                "asset_internal_id": completed.asset_internal_id,
                "status": completed.status.as_str(),
                "requested_by": completed.requested_by,
                "processed_count": completed.processed_count,
                "failed_count": completed.failed_count,
                "dhash_hex": dhash_hex,
                "palette_color_count": completed
                    .palette_json
                    .as_ref()
                    .and_then(|value| value.get("dominant"))
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, Vec::len),
            }),
        )
        .await?;
        Ok(completed)
    }

    /// Fetch a stored similarity projection for an asset, if any.
    pub async fn get_similarity_projection(
        &self,
        asset_internal_id: Uuid,
    ) -> AtelierResult<Option<SimilarityProjection>> {
        let row:Option<SimilarityProjectionRow>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_first(concat!("SELECT ",projection_select!()," FROM atelier_similarity_projection WHERE asset_internal_id=$value LIMIT 1;"),RecordBinding{value:RecordId::new("atelier_media_asset",SurrealUuid::from(asset_internal_id))}).await})).await?;
        Ok(row.map(Into::into))
    }

    /// Find media assets perceptually similar to `target_hash` within a Hamming
    /// `threshold` (0..=64), excluding the target asset, ordered nearest-first.
    /// Mirrors legacy source `image.similar.search`: candidate hashes are pulled from
    /// the store and scored with [`hamming_distance_hex64`] in-process (the dHash
    /// space is small and bounded by `limit` candidates fetched). A read-only
    /// query, so no event is recorded.
    pub async fn find_similar_assets(
        &self,
        target_hash: &str,
        threshold: i32,
        limit: i64,
        exclude_asset_internal_id: Option<Uuid>,
    ) -> AtelierResult<Vec<SimilarityHit>> {
        let target = target_hash.trim().to_ascii_lowercase();
        if !is_hex64(&target) {
            return Err(AtelierError::Validation(format!(
                "target dhash must be 16 hex chars (64 bits), got: {target_hash:?}"
            )));
        }
        let thr = threshold.clamp(0, 64);
        let cap = if limit <= 0 { 50 } else { limit };

        let rows:Vec<SimilarityCandidateRow>=self.store().with_data_operation(move|ctx|Box::pin(async move{ctx.query_values("SELECT record::id(asset_internal_id) AS asset_internal_id,dhash_hex FROM atelier_similarity_projection WHERE dhash_hex != NONE AND ($value=NONE OR asset_internal_id != $value) ORDER BY asset_internal_id;",OptionalRecordBinding{value:exclude_asset_internal_id.map(|id|RecordId::new("atelier_media_asset",SurrealUuid::from(id)))}).await})).await?;

        let mut hits: Vec<SimilarityHit> = Vec::new();
        for row in rows {
            let asset_internal_id: Uuid = row.asset_internal_id.into();
            let dhash_hex = row.dhash_hex;
            let distance = hamming_distance_hex64(&target, &dhash_hex);
            if distance <= thr {
                hits.push(SimilarityHit {
                    asset_internal_id,
                    dhash_hex,
                    distance,
                });
            }
        }
        // Nearest-first; stable secondary order by id for determinism.
        hits.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| a.asset_internal_id.cmp(&b.asset_internal_id))
        });
        hits.truncate(cap as usize);
        Ok(hits)
    }
}

// ---------------------------------------------------------------------------
// WP-CKC-posekit-overhaul MT-011 on SurrealDB: CKC search (fuzzy / vector /
// combined over characters, sheets, albums, media, tags) and rich tag notes.
//
// The PostgreSQL reference ran one UNION query scored with pg_trgm and ranked
// pgvector `<=>` distances in SQL. Neither exists on the embedded store, so the
// catalog rows are read through bound SurrealQL and the candidate assembly,
// trigram scoring and cosine ranking run deterministically in Rust, the same
// split `storage::surreal::search_store` uses for LoomSearchV2. Embeddings are
// persisted on `atelier_ckc_search_projection` (typed `array<float>` of the
// canonical dimensionality) keyed by the typed target ref.
// ---------------------------------------------------------------------------

/// Embedding dimensionality pinned by the `atelier_ckc_search_projection`
/// schema (`array::len(embedding) = 768`), the same canonical width LoomSearchV2
/// uses.
pub const CKC_SEARCH_EMBEDDING_DIM: usize = 768;

/// Trigram similarity floor below which a candidate is not a fuzzy match
/// (mirrors the reference `atelier_trgm_similarity(...) > 0.08`).
const CKC_FUZZY_THRESHOLD: f64 = 0.08;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CkcTagNote {
    pub tag_note_id: Uuid,
    pub tag_id: Uuid,
    pub tag_ref: String,
    pub tag_text: String,
    /// `None` is the global note; the store keeps `''` for it so the
    /// `(tag_id, scope_ref)` UNIQUE index never depends on NONE semantics.
    pub scope_ref: Option<String>,
    pub note: String,
    pub updated_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UpsertCkcTagNote {
    pub tag_text: String,
    pub scope_ref: Option<String>,
    pub note: String,
    pub updated_by: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CkcSearchMode {
    Fuzzy,
    Vector,
    Combined,
}

impl CkcSearchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fuzzy => "fuzzy",
            Self::Vector => "vector",
            Self::Combined => "combined",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "fuzzy" | "text" | "keyword" => Some(Self::Fuzzy),
            "vector" | "semantic" => Some(Self::Vector),
            "combined" | "combo" | "hybrid" => Some(Self::Combined),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CkcSearchRequest {
    pub query: String,
    pub modes: Vec<CkcSearchMode>,
    pub tags: Vec<String>,
    pub character_internal_id: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    pub media_asset_id: Option<Uuid>,
    pub similar_to_asset_id: Option<Uuid>,
    pub similar_to_dhash_hex: Option<String>,
    pub limit: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CkcSearchResponse {
    pub query: String,
    pub search_modes: Vec<String>,
    pub semantic_available: bool,
    pub vector_source: String,
    pub result_count: usize,
    pub results: Vec<CkcSearchHit>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CkcSearchHit {
    pub target_kind: String,
    pub target_ref: String,
    pub title: String,
    pub snippet: String,
    pub character_ref: Option<String>,
    pub sheet_version_ref: Option<String>,
    pub collection_ref: Option<String>,
    pub media_ref: Option<String>,
    pub tag_ref: Option<String>,
    pub tags: Vec<String>,
    pub tag_notes: Vec<CkcTagNoteHit>,
    pub match_modes: Vec<String>,
    pub fuzzy_score: f64,
    pub vector_score: f64,
    pub similarity_distance: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CkcTagNoteHit {
    pub tag_ref: String,
    pub tag_text: String,
    pub scope_ref: Option<String>,
    pub note: String,
}

#[derive(Clone, Debug)]
struct CkcSearchCandidate {
    target_kind: &'static str,
    target_ref: String,
    title: String,
    search_text: String,
    fuzzy_score: f64,
    character_internal_id: Option<Uuid>,
    sheet_version_id: Option<Uuid>,
    collection_id: Option<Uuid>,
    asset_id: Option<Uuid>,
    tag_id: Option<Uuid>,
    tags: Vec<String>,
}

fn normalize_search_modes(modes: &[CkcSearchMode]) -> Vec<CkcSearchMode> {
    if modes.is_empty() {
        return vec![CkcSearchMode::Fuzzy];
    }
    let mut out = Vec::new();
    for mode in modes {
        if !out.contains(mode) {
            out.push(*mode);
        }
    }
    out
}

fn mode_strings(modes: &[CkcSearchMode]) -> Vec<String> {
    modes.iter().map(|mode| mode.as_str().to_owned()).collect()
}

fn wants_vector(modes: &[CkcSearchMode]) -> bool {
    modes
        .iter()
        .any(|mode| matches!(mode, CkcSearchMode::Vector | CkcSearchMode::Combined))
}

fn wants_fuzzy(modes: &[CkcSearchMode]) -> bool {
    modes
        .iter()
        .any(|mode| matches!(mode, CkcSearchMode::Fuzzy | CkcSearchMode::Combined))
}

fn wants_combined(modes: &[CkcSearchMode]) -> bool {
    modes.iter().any(|mode| *mode == CkcSearchMode::Combined)
}

fn text_snippet(text: &str, query: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.len() <= 220 {
        return text;
    }
    let lower = text.to_ascii_lowercase();
    let needle = query.trim().to_ascii_lowercase();
    let start = if needle.is_empty() {
        0
    } else {
        lower
            .find(&needle)
            .map(|idx| idx.saturating_sub(60))
            .unwrap_or(0)
    };
    let end = (start + 220).min(text.len());
    let mut start = start;
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = end;
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    text[start..end].to_owned()
}

fn normalized_search_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let normalized = normalize_tag(tag);
        if !normalized.is_empty() && !out.iter().any(|existing| existing == &normalized) {
            out.push(normalized);
        }
    }
    out.sort();
    out
}

fn merged_tag_sets(sets: &[&[String]]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for set in sets {
        for tag in set.iter() {
            let normalized = normalize_tag(tag);
            if !normalized.is_empty() && !out.contains(&normalized) {
                out.push(normalized);
            }
        }
    }
    out.sort();
    out
}

fn contains_all_tags(actual: &[String], required: &[String]) -> bool {
    required
        .iter()
        .all(|required| actual.iter().any(|tag| tag == required))
}

fn ckc_query_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for term in query
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|term| !term.is_empty())
    {
        if !terms.iter().any(|existing| existing == term) {
            terms.push(term.to_owned());
        }
    }
    terms
}

fn ckc_candidate_matches_query_terms(candidate: &CkcSearchCandidate, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!("{}\n{}", candidate.title, candidate.search_text).to_ascii_lowercase();
    terms.iter().all(|term| haystack.contains(term))
}

/// pg_trgm-shaped trigrams: each alphanumeric word is padded with two leading
/// and one trailing blank before windowing, so word starts weigh more than word
/// interiors exactly as the PostgreSQL extension scores them.
fn ckc_trigrams(value: &str) -> HashSet<String> {
    value
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .flat_map(|word| {
            let padded = format!("  {word} ").chars().collect::<Vec<_>>();
            padded
                .windows(3)
                .map(|window| window.iter().collect::<String>())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Jaccard similarity over trigram sets, the `similarity()` definition of
/// pg_trgm (the reference `atelier_trgm_similarity` SQL function).
fn ckc_trigram_similarity(left: &str, right: &str) -> f64 {
    let left = ckc_trigrams(left);
    let right = ckc_trigrams(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let common = left.intersection(&right).count();
    common as f64 / (left.len() + right.len() - common) as f64
}

/// The reference `fuzzy_score`: the greatest of trigram similarity against the
/// search text, trigram similarity against the title, and 1.0 for a literal
/// case-insensitive containment in either.
fn ckc_fuzzy_score(title: &str, search_text: &str, query: &str) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let lower_query = query.to_ascii_lowercase();
    let mut score = ckc_trigram_similarity(search_text, query)
        .max(ckc_trigram_similarity(title, query));
    if search_text.to_ascii_lowercase().contains(&lower_query)
        || title.to_ascii_lowercase().contains(&lower_query)
    {
        score = score.max(1.0);
    }
    score
}

fn cosine_similarity_stored(stored: &[f64], query: &[f32]) -> f64 {
    if stored.len() != query.len() || stored.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut left = 0.0_f64;
    let mut right = 0.0_f64;
    for (a, b) in stored.iter().zip(query.iter()) {
        let b = f64::from(*b);
        dot += a * b;
        left += a * a;
        right += b * b;
    }
    if left <= 0.0 || right <= 0.0 {
        return 0.0;
    }
    dot / (left.sqrt() * right.sqrt())
}

async fn embed_ckc_text(llm: &dyn LlmClient, text: &str) -> AtelierResult<Option<Vec<f32>>> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let model_id = llm.profile().model_id.clone();
    let req = EmbeddingRequest::new(Uuid::now_v7(), text.to_owned(), model_id);
    match llm.embedding(req).await {
        Ok(response) => {
            if response.vector.len() != CKC_SEARCH_EMBEDDING_DIM {
                return Err(AtelierError::Validation(format!(
                    "CKC search embedding dimensionality mismatch: expected {}, got {}",
                    CKC_SEARCH_EMBEDDING_DIM,
                    response.vector.len()
                )));
            }
            if response.vector.iter().any(|value| !value.is_finite()) {
                return Err(AtelierError::Validation(
                    "CKC search embedding values must be finite".to_owned(),
                ));
            }
            Ok(Some(response.vector))
        }
        // Any typed LLM error means no embedding model is available: degrade
        // to fuzzy/dHash, never fabricate a vector.
        Err(_) => Ok(None),
    }
}

fn candidate_scope_refs(candidate: &CkcSearchCandidate) -> Vec<String> {
    let mut refs = vec![candidate.target_ref.clone()];
    if let Some(id) = candidate.character_internal_id {
        refs.push(character_ref(id));
    }
    if let Some(id) = candidate.collection_id {
        refs.push(collection_ref(id));
    }
    if let (Some(character_id), Some(version_id)) =
        (candidate.character_internal_id, candidate.sheet_version_id)
    {
        refs.push(sheet_version_ref(character_id, version_id));
    }
    if let Some(id) = candidate.asset_id {
        refs.push(media_asset_ref(id));
    }
    if let Some(id) = candidate.tag_id {
        refs.push(tag_ref(id));
    }
    refs.sort();
    refs.dedup();
    refs
}

fn join_search_text(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .filter_map(|part| part.map(str::trim).filter(|part| !part.is_empty()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tags_text(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        None
    } else {
        serde_json::to_string(tags).ok()
    }
}

fn tag_note_scope_from_stored(scope_ref: String) -> Option<String> {
    if scope_ref.is_empty() {
        None
    } else {
        Some(scope_ref)
    }
}

#[derive(SurrealValue)]
struct CkcCharacterRow {
    internal_id: SurrealUuid,
    public_id: String,
    display_name: String,
}

#[derive(SurrealValue)]
struct CkcSheetRow {
    version_id: SurrealUuid,
    character_internal_id: SurrealUuid,
    seq: i64,
    raw_text: String,
}

#[derive(SurrealValue)]
struct CkcCharacterTagRow {
    character_internal_id: SurrealUuid,
    text: String,
}

#[derive(SurrealValue)]
struct CkcCollectionRow {
    collection_id: SurrealUuid,
    name: String,
    notes: String,
    tags_json: Vec<String>,
    character_internal_id: Option<SurrealUuid>,
    sheet_version_id: Option<SurrealUuid>,
}

#[derive(SurrealValue)]
struct CkcCollectionItemRow {
    collection_id: SurrealUuid,
    asset_id: SurrealUuid,
    content_hash: String,
    mime: String,
    source_provenance: Option<String>,
    artifact_ref: String,
}

#[derive(SurrealValue)]
struct CkcReviewRow {
    asset_id: SurrealUuid,
    notes: Option<String>,
    review_status: String,
}

#[derive(SurrealValue)]
struct CkcMediaTagRow {
    asset_id: SurrealUuid,
    text: String,
}

#[derive(SurrealValue)]
struct CkcTagDictRow {
    tag_id: SurrealUuid,
    text: String,
}

#[derive(SurrealValue)]
struct CkcTagNoteRow {
    tag_note_id: SurrealUuid,
    tag_id: SurrealUuid,
    tag_text: String,
    scope_ref: String,
    note: String,
    updated_by: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl From<CkcTagNoteRow> for CkcTagNote {
    fn from(row: CkcTagNoteRow) -> Self {
        let tag_id: Uuid = row.tag_id.into();
        Self {
            tag_note_id: row.tag_note_id.into(),
            tag_id,
            tag_ref: tag_ref(tag_id),
            tag_text: row.tag_text,
            scope_ref: tag_note_scope_from_stored(row.scope_ref),
            note: row.note,
            updated_by: row.updated_by,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct CkcProjectionStateRow {
    target_ref: String,
    search_text_hash: String,
    embedding_model: Option<String>,
    has_embedding: bool,
}

#[derive(SurrealValue)]
struct CkcProjectionEmbeddingRow {
    target_ref: String,
    embedding: Vec<f64>,
}

#[derive(SurrealValue)]
struct TargetRefsBinding {
    target_refs: Vec<String>,
}

#[derive(Clone, SurrealValue)]
struct UpsertCkcProjectionBindings {
    rid: RecordId,
    target_ref: String,
    target_kind: String,
    search_text_hash: String,
    embedding: Vec<f64>,
    embedding_model: String,
}

#[derive(SurrealValue)]
struct TagNoteLookupBindings {
    tag_text: String,
    scope_ref: String,
}

#[derive(Clone, SurrealValue)]
struct UpsertCkcTagNoteBindings {
    tag_rid: RecordId,
    tag_id: SurrealUuid,
    tag_text: String,
    note_rid: RecordId,
    tag_note_id: SurrealUuid,
    scope_ref: String,
    note: String,
    updated_by: String,
}

#[derive(SurrealValue)]
struct SheetScopeBindings {
    version_rid: RecordId,
    character_ref: RecordId,
}

macro_rules! ckc_tag_note_select {
    () => {
        "tag_note_id, record::id(tag_id) AS tag_id, tag_id.text AS tag_text, scope_ref, note, \
         updated_by, created_at_utc, updated_at_utc"
    };
}

const CKC_CHARACTERS_STATEMENT: &str =
    "SELECT internal_id, public_id, display_name FROM atelier_character;";
const CKC_SHEETS_STATEMENT: &str =
    "SELECT version_id, record::id(character_internal_id) AS character_internal_id, seq, raw_text \
     FROM atelier_sheet_version;";
const CKC_CHARACTER_TAGS_STATEMENT: &str =
    "SELECT record::id(character_internal_id) AS character_internal_id, tag_id.text AS text \
     FROM atelier_character_tag;";
const CKC_COLLECTIONS_STATEMENT: &str =
    "SELECT collection_id, name, notes, tags_json, \
       IF character_internal_id = NONE { NONE } ELSE { record::id(character_internal_id) } \
         AS character_internal_id, \
       IF sheet_version_id = NONE { NONE } ELSE { record::id(sheet_version_id) } \
         AS sheet_version_id \
     FROM atelier_collection WHERE character_internal_id != NONE;";
const CKC_COLLECTION_ITEMS_STATEMENT: &str =
    "SELECT record::id(collection_id) AS collection_id, record::id(asset_id) AS asset_id, \
       asset_id.content_hash AS content_hash, asset_id.mime AS mime, \
       asset_id.source_provenance AS source_provenance, asset_id.artifact_ref AS artifact_ref \
     FROM atelier_collection_item ORDER BY sort_order ASC, asset_id ASC;";
const CKC_REVIEW_STATEMENT: &str =
    "SELECT record::id(asset_id) AS asset_id, notes, review_status FROM atelier_media_review_metadata;";
const CKC_MEDIA_TAGS_STATEMENT: &str =
    "SELECT record::id(asset_id) AS asset_id, tag_id.text AS text FROM atelier_media_asset_tag;";
const CKC_TAG_DICT_STATEMENT: &str = "SELECT tag_id, text FROM atelier_tag ORDER BY text ASC;";
const CKC_TAG_NOTES_STATEMENT: &str = concat!(
    "SELECT ",
    ckc_tag_note_select!(),
    " FROM atelier_tag_note;"
);
const CKC_PROJECTION_STATE_STATEMENT: &str =
    "SELECT target_ref, search_text_hash, embedding_model, embedding != NONE AS has_embedding \
     FROM atelier_ckc_search_projection WHERE target_ref IN $target_refs;";
const CKC_PROJECTION_EMBEDDINGS_STATEMENT: &str =
    "SELECT target_ref, embedding FROM atelier_ckc_search_projection \
     WHERE embedding != NONE AND target_ref IN $target_refs;";
const UPSERT_CKC_PROJECTION_STATEMENT: &str =
    "UPSERT $rid SET target_ref = $target_ref, target_kind = $target_kind, \
       search_text_hash = $search_text_hash, embedding = $embedding, \
       embedding_model = $embedding_model, indexed_at_utc = time::now() RETURN NONE;";
const CKC_TAG_NOTE_LOOKUP_STATEMENT: &str = concat!(
    "SELECT ",
    ckc_tag_note_select!(),
    " FROM atelier_tag_note WHERE tag_id.text = $tag_text AND scope_ref = $scope_ref LIMIT 1;"
);
const SHEET_SCOPE_EXISTS_STATEMENT: &str =
    "RETURN count(SELECT id FROM atelier_sheet_version \
                  WHERE id = $version_rid AND character_internal_id = $character_ref) > 0;";
const RECORD_EXISTS_STATEMENT: &str = "RETURN record::exists($value);";

/// Tag dictionary row (created on demand) + tag note upsert + event in ONE
/// statement, so a rejected write never leaves an orphan tag. `note_rid` is the
/// existing note's id when the caller found one, otherwise a fresh id; a note
/// that appeared for the same `(tag, scope)` in between throws so the unique
/// index is never raced.
const UPSERT_CKC_TAG_NOTE_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing_tag = (SELECT VALUE id FROM atelier_tag WHERE text = $domain.tag_text LIMIT 1); \
       IF $existing_tag = [] { \
         CREATE $domain.tag_rid CONTENT { tag_id: $domain.tag_id, text: $domain.tag_text }; \
       }; \
       LET $tag_ref = (SELECT VALUE id FROM atelier_tag WHERE text = $domain.tag_text LIMIT 1)[0]; \
       LET $existing_note = (SELECT VALUE id FROM atelier_tag_note \
                             WHERE tag_id = $tag_ref AND scope_ref = $domain.scope_ref LIMIT 1)[0]; \
       IF $existing_note != NONE AND $existing_note != $domain.note_rid { \
         THROW 'ckc_tag_note_concurrent_create'; \
       }; ",
    atelier_event_sql!(),
    " IF $existing_note = NONE { \
         CREATE $domain.note_rid CONTENT { tag_note_id: $domain.tag_note_id, tag_id: $tag_ref, \
           scope_ref: $domain.scope_ref, note: $domain.note, updated_by: $domain.updated_by }; \
       } ELSE { \
         UPDATE $domain.note_rid SET note = $domain.note, updated_by = $domain.updated_by, \
           updated_at_utc = time::now(); \
       }; \
       RETURN (SELECT ",
    ckc_tag_note_select!(),
    " FROM $domain.note_rid); };"
);

impl AtelierStore {
    async fn ckc_record_exists(&self, record: RecordId) -> AtelierResult<bool> {
        let exists: Option<bool> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(RECORD_EXISTS_STATEMENT, RecordBinding { value: record })
                        .await
                })
            })
            .await?;
        Ok(exists.unwrap_or(false))
    }

    /// A tag note scope must be a typed atelier ref whose target exists:
    /// `atelier://character`, `atelier://sheet` (version must belong to the
    /// named character), `atelier://collection`, `atelier://media`, or
    /// `atelier://tag`.
    async fn validate_ckc_tag_note_scope_ref(&self, scope_ref: &str) -> AtelierResult<()> {
        let exists = if let Some(id) = parse_character_ref(scope_ref) {
            self.ckc_record_exists(RecordId::new("atelier_character", SurrealUuid::from(id)))
                .await?
        } else if let Some((character_id, version_id)) = parse_sheet_version_ref(scope_ref) {
            let bindings = SheetScopeBindings {
                version_rid: RecordId::new("atelier_sheet_version", SurrealUuid::from(version_id)),
                character_ref: RecordId::new("atelier_character", SurrealUuid::from(character_id)),
            };
            let exists: Option<bool> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(async move { ctx.query_first(SHEET_SCOPE_EXISTS_STATEMENT, bindings).await })
                })
                .await?;
            exists.unwrap_or(false)
        } else if let Some(id) = parse_collection_ref(scope_ref) {
            self.ckc_record_exists(RecordId::new("atelier_collection", SurrealUuid::from(id)))
                .await?
        } else if let Some(id) = parse_media_asset_ref(scope_ref) {
            self.ckc_record_exists(RecordId::new("atelier_media_asset", SurrealUuid::from(id)))
                .await?
        } else if let Some(id) = parse_tag_ref(scope_ref) {
            self.ckc_record_exists(RecordId::new("atelier_tag", SurrealUuid::from(id)))
                .await?
        } else {
            return Err(AtelierError::Validation(
                "tag note scope_ref must be a valid atelier://character, atelier://sheet, atelier://collection, atelier://media, or atelier://tag ref".into(),
            ));
        };
        if !exists {
            return Err(AtelierError::Validation(
                "tag note scope_ref target does not exist".into(),
            ));
        }
        Ok(())
    }

    /// Create or update the rich note for `(tag, scope)`. The tag dictionary
    /// row is created on demand inside the same statement as the note and its
    /// `TAG_NOTE_UPSERTED` event; every validation (including scope-target
    /// existence) happens before any write.
    pub async fn upsert_ckc_tag_note(&self, input: &UpsertCkcTagNote) -> AtelierResult<CkcTagNote> {
        let tag_text = normalize_tag(&input.tag_text);
        if tag_text.is_empty() {
            return Err(AtelierError::Validation(
                "tag note tag_text must not be empty".into(),
            ));
        }
        let scope_ref = match input.scope_ref.as_deref() {
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() || trimmed != raw {
                    return Err(AtelierError::Validation(
                        "tag note scope_ref must not be empty or padded".into(),
                    ));
                }
                reject_legacy_runtime_ref("tag note scope_ref", raw)?;
                if !raw.starts_with("atelier://") {
                    return Err(AtelierError::Validation(
                        "tag note scope_ref must be an atelier:// typed ref".into(),
                    ));
                }
                self.validate_ckc_tag_note_scope_ref(raw).await?;
                Some(raw.to_owned())
            }
            None => None,
        };
        let note = input.note.trim();
        if note.is_empty() || note != input.note {
            return Err(AtelierError::Validation(
                "tag note must not be empty or padded".into(),
            ));
        }
        let updated_by = input.updated_by.trim();
        if updated_by.is_empty() || updated_by != input.updated_by {
            return Err(AtelierError::Validation(
                "tag note updated_by must not be empty or padded".into(),
            ));
        }
        let stored_scope = scope_ref.clone().unwrap_or_default();

        let lookup = TagNoteLookupBindings {
            tag_text: tag_text.clone(),
            scope_ref: stored_scope.clone(),
        };
        let existing: Option<CkcTagNoteRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(CKC_TAG_NOTE_LOOKUP_STATEMENT, lookup).await })
            })
            .await?;
        let tag_note_id: Uuid = existing
            .map(|row| row.tag_note_id.into())
            .unwrap_or_else(Uuid::now_v7);
        let candidate_tag_id = Uuid::now_v7();
        let bindings = UpsertCkcTagNoteBindings {
            tag_rid: RecordId::new("atelier_tag", SurrealUuid::from(candidate_tag_id)),
            tag_id: SurrealUuid::from(candidate_tag_id),
            tag_text: tag_text.clone(),
            note_rid: RecordId::new("atelier_tag_note", SurrealUuid::from(tag_note_id)),
            tag_note_id: SurrealUuid::from(tag_note_id),
            scope_ref: stored_scope,
            note: note.to_owned(),
            updated_by: updated_by.to_owned(),
        };
        let row: Option<CkcTagNoteRow> = self
            .write_with_event(
                UPSERT_CKC_TAG_NOTE_STATEMENT,
                bindings,
                search_event_family::TAG_NOTE_UPSERTED,
                "atelier_tag_note",
                &tag_note_id.to_string(),
                serde_json::json!({
                    "tag_note_id": tag_note_id,
                    "tag_text": tag_text,
                    "scope_ref": scope_ref,
                    "updated_by": updated_by,
                }),
            )
            .await
            .map_err(|error| {
                let text = error.to_string();
                if text.contains("ckc_tag_note_concurrent_create")
                    || text.contains("uq_atelier_tag_note_tag_scope")
                {
                    AtelierError::Conflict(format!(
                        "tag note for tag_text={tag_text:?} scope_ref={scope_ref:?} was created concurrently; retry"
                    ))
                } else {
                    error
                }
            })?;
        row.map(CkcTagNote::from).ok_or_else(|| {
            AtelierError::Internal("upserting a tag note returned no row".to_owned())
        })
    }

    /// Every tag note, newest catalog order irrelevant: callers filter by tag
    /// text and scope.
    async fn load_ckc_tag_notes(&self) -> AtelierResult<Vec<CkcTagNote>> {
        let rows: Vec<CkcTagNoteRow> = self
            .store()
            .with_data_operation(|ctx| {
                Box::pin(async move { ctx.query_values(CKC_TAG_NOTES_STATEMENT, EmptyBindings {}).await })
            })
            .await?;
        let mut notes: Vec<CkcTagNote> = rows.into_iter().map(Into::into).collect();
        notes.sort_by(|a, b| {
            a.tag_text
                .cmp(&b.tag_text)
                .then_with(|| a.scope_ref.cmp(&b.scope_ref))
        });
        Ok(notes)
    }

    async fn ckc_rows<R>(&self, statement: &'static str) -> AtelierResult<Vec<R>>
    where
        R: SurrealValue + Send + 'static,
    {
        Ok(self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(statement, EmptyBindings {}).await })
            })
            .await?)
    }

    /// Assemble the CKC candidate set (characters, sheets, character-scoped
    /// albums, album media, tags) from the catalog rows and score it. This is
    /// the reference UNION query executed in Rust: every table is read through
    /// bound SurrealQL and joined by id here, the trigram score is computed per
    /// candidate, and the same containment / similarity pre-filter and
    /// `prefetch_limit` cap apply.
    async fn ckc_search_candidates(
        &self,
        request: &CkcSearchRequest,
        modes: &[CkcSearchMode],
        prefetch_limit: usize,
    ) -> AtelierResult<Vec<CkcSearchCandidate>> {
        let query = request.query.trim();
        let allow_vector_scan = wants_vector(modes);

        let characters: Vec<CkcCharacterRow> = self.ckc_rows(CKC_CHARACTERS_STATEMENT).await?;
        let sheets: Vec<CkcSheetRow> = self.ckc_rows(CKC_SHEETS_STATEMENT).await?;
        let character_tag_rows: Vec<CkcCharacterTagRow> =
            self.ckc_rows(CKC_CHARACTER_TAGS_STATEMENT).await?;
        let collections: Vec<CkcCollectionRow> = self.ckc_rows(CKC_COLLECTIONS_STATEMENT).await?;
        let items: Vec<CkcCollectionItemRow> = self.ckc_rows(CKC_COLLECTION_ITEMS_STATEMENT).await?;
        let reviews: Vec<CkcReviewRow> = self.ckc_rows(CKC_REVIEW_STATEMENT).await?;
        let media_tag_rows: Vec<CkcMediaTagRow> = self.ckc_rows(CKC_MEDIA_TAGS_STATEMENT).await?;
        let tag_dict: Vec<CkcTagDictRow> = self.ckc_rows(CKC_TAG_DICT_STATEMENT).await?;
        let tag_notes = self.load_ckc_tag_notes().await?;

        let mut character_tags: HashMap<Uuid, Vec<String>> = HashMap::new();
        for row in character_tag_rows {
            character_tags
                .entry(row.character_internal_id.into())
                .or_default()
                .push(row.text);
        }
        for tags in character_tags.values_mut() {
            *tags = merged_tag_sets(&[tags.as_slice()]);
        }
        let mut media_tags: HashMap<Uuid, Vec<String>> = HashMap::new();
        for row in media_tag_rows {
            media_tags
                .entry(row.asset_id.into())
                .or_default()
                .push(row.text);
        }
        for tags in media_tags.values_mut() {
            *tags = merged_tag_sets(&[tags.as_slice()]);
        }
        let reviews: HashMap<Uuid, CkcReviewRow> = reviews
            .into_iter()
            .map(|row| (Uuid::from(row.asset_id), row))
            .collect();
        let characters: HashMap<Uuid, CkcCharacterRow> = characters
            .into_iter()
            .map(|row| (Uuid::from(row.internal_id), row))
            .collect();
        let mut latest_sheet: HashMap<Uuid, &CkcSheetRow> = HashMap::new();
        for sheet in &sheets {
            let character_id: Uuid = sheet.character_internal_id.into();
            match latest_sheet.get(&character_id) {
                Some(current) if current.seq >= sheet.seq => {}
                _ => {
                    latest_sheet.insert(character_id, sheet);
                }
            }
        }
        let collections_by_id: HashMap<Uuid, &CkcCollectionRow> = collections
            .iter()
            .map(|row| (Uuid::from(row.collection_id), row))
            .collect();
        let empty_tags: Vec<String> = Vec::new();

        let mut candidates: Vec<CkcSearchCandidate> = Vec::new();

        for (character_id, character) in &characters {
            let tags = character_tags.get(character_id).unwrap_or(&empty_tags);
            let sheet = latest_sheet.get(character_id);
            let tags_json = tags_text(tags);
            candidates.push(CkcSearchCandidate {
                target_kind: "character",
                target_ref: character_ref(*character_id),
                title: character.display_name.clone(),
                search_text: join_search_text(&[
                    Some(&character.display_name),
                    Some(&character.public_id),
                    sheet.map(|sheet| sheet.raw_text.as_str()),
                    tags_json.as_deref(),
                ]),
                fuzzy_score: 0.0,
                character_internal_id: Some(*character_id),
                sheet_version_id: sheet.map(|sheet| sheet.version_id.into()),
                collection_id: None,
                asset_id: None,
                tag_id: None,
                tags: tags.clone(),
            });
        }

        for sheet in &sheets {
            let character_id: Uuid = sheet.character_internal_id.into();
            let Some(character) = characters.get(&character_id) else {
                continue;
            };
            let version_id: Uuid = sheet.version_id.into();
            let tags = character_tags.get(&character_id).unwrap_or(&empty_tags);
            let tags_json = tags_text(tags);
            candidates.push(CkcSearchCandidate {
                target_kind: "sheet",
                target_ref: sheet_version_ref(character_id, version_id),
                title: format!("{} sheet v{}", character.display_name, sheet.seq),
                search_text: join_search_text(&[
                    Some(&character.display_name),
                    Some(&character.public_id),
                    Some(&sheet.raw_text),
                    tags_json.as_deref(),
                ]),
                fuzzy_score: 0.0,
                character_internal_id: Some(character_id),
                sheet_version_id: Some(version_id),
                collection_id: None,
                asset_id: None,
                tag_id: None,
                tags: tags.clone(),
            });
        }

        for collection in &collections {
            let Some(character_id) = collection.character_internal_id.map(Uuid::from) else {
                continue;
            };
            let collection_id: Uuid = collection.collection_id.into();
            let character = characters.get(&character_id);
            let sheet = latest_sheet.get(&character_id);
            let char_tags = character_tags.get(&character_id).unwrap_or(&empty_tags);
            let collection_tags_json = tags_text(&collection.tags_json);
            let char_tags_json = tags_text(char_tags);
            candidates.push(CkcSearchCandidate {
                target_kind: "album",
                target_ref: collection_ref(collection_id),
                title: collection.name.clone(),
                search_text: join_search_text(&[
                    Some(&collection.name),
                    Some(&collection.notes),
                    collection_tags_json.as_deref(),
                    character.map(|character| character.display_name.as_str()),
                    sheet.map(|sheet| sheet.raw_text.as_str()),
                    char_tags_json.as_deref(),
                ]),
                fuzzy_score: 0.0,
                character_internal_id: Some(character_id),
                sheet_version_id: collection.sheet_version_id.map(Uuid::from),
                collection_id: Some(collection_id),
                asset_id: None,
                tag_id: None,
                tags: merged_tag_sets(&[collection.tags_json.as_slice(), char_tags.as_slice()]),
            });
        }

        // The same asset linked into several character albums is one search
        // target: the first (album-ordered) membership supplies the title and
        // every membership contributes its album context to the search text.
        let mut media_index: HashMap<Uuid, usize> = HashMap::new();
        for item in &items {
            let collection_id: Uuid = item.collection_id.into();
            let Some(collection) = collections_by_id.get(&collection_id) else {
                continue;
            };
            let Some(character_id) = collection.character_internal_id.map(Uuid::from) else {
                continue;
            };
            let asset_id: Uuid = item.asset_id.into();
            let character = characters.get(&character_id);
            let sheet = latest_sheet.get(&character_id);
            let char_tags = character_tags.get(&character_id).unwrap_or(&empty_tags);
            let asset_tags = media_tags.get(&asset_id).unwrap_or(&empty_tags);
            let review = reviews.get(&asset_id);
            let media_tags_json = tags_text(asset_tags);
            let collection_tags_json = tags_text(&collection.tags_json);
            let char_tags_json = tags_text(char_tags);
            let context_text = join_search_text(&[
                Some(&collection.name),
                Some(&collection.notes),
                collection_tags_json.as_deref(),
                character.map(|character| character.display_name.as_str()),
                sheet.map(|sheet| sheet.raw_text.as_str()),
                char_tags_json.as_deref(),
            ]);
            let merged_tags = merged_tag_sets(&[
                asset_tags.as_slice(),
                collection.tags_json.as_slice(),
                char_tags.as_slice(),
            ]);
            if let Some(index) = media_index.get(&asset_id) {
                let existing = &mut candidates[*index];
                if !context_text.is_empty() {
                    existing.search_text.push('\n');
                    existing.search_text.push_str(&context_text);
                }
                existing.tags = merged_tag_sets(&[existing.tags.as_slice(), merged_tags.as_slice()]);
                continue;
            }
            let title_source = item
                .source_provenance
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| item.content_hash.chars().take(16).collect());
            media_index.insert(asset_id, candidates.len());
            candidates.push(CkcSearchCandidate {
                target_kind: "media",
                target_ref: media_asset_ref(asset_id),
                title: format!("{title_source} / {}", collection.name),
                search_text: join_search_text(&[
                    Some(&item.content_hash),
                    Some(&item.mime),
                    item.source_provenance.as_deref(),
                    Some(&item.artifact_ref),
                    review.and_then(|review| review.notes.as_deref()),
                    review.map(|review| review.review_status.as_str()),
                    media_tags_json.as_deref(),
                    Some(&context_text),
                ]),
                fuzzy_score: 0.0,
                character_internal_id: Some(character_id),
                sheet_version_id: collection.sheet_version_id.map(Uuid::from),
                collection_id: Some(collection_id),
                asset_id: Some(asset_id),
                tag_id: None,
                tags: merged_tags,
            });
        }

        for tag in &tag_dict {
            let tag_id: Uuid = tag.tag_id.into();
            let mut parts: Vec<Option<&str>> = vec![Some(tag.text.as_str())];
            for note in tag_notes.iter().filter(|note| note.tag_id == tag_id) {
                parts.push(Some(note.note.as_str()));
                parts.push(note.scope_ref.as_deref());
            }
            candidates.push(CkcSearchCandidate {
                target_kind: "tag",
                target_ref: tag_ref(tag_id),
                title: tag.text.clone(),
                search_text: join_search_text(&parts),
                fuzzy_score: 0.0,
                character_internal_id: None,
                sheet_version_id: None,
                collection_id: None,
                asset_id: None,
                tag_id: Some(tag_id),
                tags: vec![normalize_tag(&tag.text)],
            });
        }

        let lower_query = query.to_ascii_lowercase();
        let mut scored: Vec<CkcSearchCandidate> = candidates
            .into_iter()
            .filter(|candidate| {
                request
                    .character_internal_id
                    .map_or(true, |id| candidate.character_internal_id == Some(id))
                    && request
                        .collection_id
                        .map_or(true, |id| candidate.collection_id == Some(id))
                    && request
                        .media_asset_id
                        .map_or(true, |id| candidate.asset_id == Some(id))
            })
            .map(|mut candidate| {
                candidate.fuzzy_score =
                    ckc_fuzzy_score(&candidate.title, &candidate.search_text, query);
                candidate
            })
            .filter(|candidate| {
                query.is_empty()
                    || allow_vector_scan
                    || candidate.search_text.to_ascii_lowercase().contains(&lower_query)
                    || candidate.title.to_ascii_lowercase().contains(&lower_query)
                    || candidate.fuzzy_score > CKC_FUZZY_THRESHOLD
            })
            .collect();
        scored.sort_by(|a, b| {
            b.fuzzy_score
                .partial_cmp(&a.fuzzy_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.target_kind.cmp(b.target_kind))
                .then_with(|| a.target_ref.cmp(&b.target_ref))
        });
        scored.truncate(prefetch_limit);
        Ok(scored)
    }

    /// Bring the semantic projection of every candidate up to date: a row whose
    /// `search_text_hash` and `embedding_model` already match is left alone,
    /// everything else is (re)embedded through the configured model runtime.
    async fn refresh_ckc_semantic_projections(
        &self,
        candidates: &[CkcSearchCandidate],
        llm: &dyn LlmClient,
    ) -> AtelierResult<()> {
        if candidates.is_empty() {
            return Ok(());
        }
        let embedding_model = llm.profile().model_id.clone();
        let target_refs: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.target_ref.clone())
            .collect();
        let state_rows: Vec<CkcProjectionStateRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        CKC_PROJECTION_STATE_STATEMENT,
                        TargetRefsBinding { target_refs },
                    )
                    .await
                })
            })
            .await?;
        let state: HashMap<String, CkcProjectionStateRow> = state_rows
            .into_iter()
            .map(|row| (row.target_ref.clone(), row))
            .collect();
        for candidate in candidates {
            let search_text_hash = event_ref_for_text(&candidate.search_text);
            let is_current = state.get(&candidate.target_ref).is_some_and(|row| {
                row.search_text_hash == search_text_hash
                    && row.embedding_model.as_deref() == Some(embedding_model.as_str())
                    && row.has_embedding
            });
            if is_current {
                continue;
            }
            let Some(embedding) = embed_ckc_text(llm, &candidate.search_text).await? else {
                continue;
            };
            let bindings = UpsertCkcProjectionBindings {
                rid: RecordId::new("atelier_ckc_search_projection", candidate.target_ref.clone()),
                target_ref: candidate.target_ref.clone(),
                target_kind: candidate.target_kind.to_owned(),
                search_text_hash,
                embedding: embedding.iter().map(|value| f64::from(*value)).collect(),
                embedding_model: embedding_model.clone(),
            };
            self.store()
                .with_data_operation(move |ctx| {
                    Box::pin(async move {
                        ctx.execute_returning(UPSERT_CKC_PROJECTION_STATEMENT, bindings)
                            .await
                    })
                })
                .await?;
        }
        Ok(())
    }

    /// Cosine similarity of the query embedding against every stored candidate
    /// projection, clamped at zero (the reference `GREATEST(0, 1 - <=>)`).
    async fn ckc_semantic_scores_for_candidates(
        &self,
        candidates: &[CkcSearchCandidate],
        query_embedding: &[f32],
    ) -> AtelierResult<HashMap<String, f64>> {
        if candidates.is_empty() {
            return Ok(HashMap::new());
        }
        let target_refs: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.target_ref.clone())
            .collect();
        let rows: Vec<CkcProjectionEmbeddingRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        CKC_PROJECTION_EMBEDDINGS_STATEMENT,
                        TargetRefsBinding { target_refs },
                    )
                    .await
                })
            })
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let score = cosine_similarity_stored(&row.embedding, query_embedding).max(0.0);
                (row.target_ref, score)
            })
            .collect())
    }

    fn ckc_tag_notes_for_candidate(
        candidate: &CkcSearchCandidate,
        tag_notes: &[CkcTagNote],
    ) -> Vec<CkcTagNoteHit> {
        if candidate.tags.is_empty() {
            return Vec::new();
        }
        let scope_refs = candidate_scope_refs(candidate);
        tag_notes
            .iter()
            .filter(|note| candidate.tags.iter().any(|tag| tag == &note.tag_text))
            .filter(|note| match note.scope_ref.as_deref() {
                None => true,
                Some(scope) => scope_refs.iter().any(|candidate_scope| candidate_scope == scope),
            })
            .map(|note| CkcTagNoteHit {
                tag_ref: note.tag_ref.clone(),
                tag_text: note.tag_text.clone(),
                scope_ref: note.scope_ref.clone(),
                note: note.note.clone(),
            })
            .collect()
    }

    /// Fuzzy / vector / combined CKC search. `llm` supplies the embedding
    /// runtime for the vector leg; `None` (or a runtime without an embedding
    /// endpoint) degrades to trigram + dHash similarity with
    /// `semantic_available = false`, never a fabricated vector.
    pub async fn ckc_search(
        &self,
        request: CkcSearchRequest,
        llm: Option<&dyn LlmClient>,
    ) -> AtelierResult<CkcSearchResponse> {
        let modes = normalize_search_modes(&request.modes);
        let query = request.query.trim().to_owned();
        let vector_requested = wants_vector(&modes);
        let limit = request.limit.clamp(1, 100);
        let prefetch_limit = if vector_requested {
            5_000
        } else {
            (limit * 8).clamp(50, 500)
        } as usize;
        let required_tags = normalized_search_tags(&request.tags);
        let query_terms = ckc_query_terms(&query);
        let tag_notes = self.load_ckc_tag_notes().await?;
        let mut candidates = self
            .ckc_search_candidates(&request, &modes, prefetch_limit)
            .await?;

        if !required_tags.is_empty() {
            candidates.retain(|candidate| contains_all_tags(&candidate.tags, &required_tags));
        }

        let mut similarity_distances: HashMap<Uuid, i32> = HashMap::new();
        let target_hash = if let Some(hash) = request.similar_to_dhash_hex.as_deref() {
            Some(hash.trim().to_ascii_lowercase())
        } else if let Some(asset_id) = request.similar_to_asset_id {
            self.get_similarity_projection(asset_id)
                .await?
                .and_then(|projection| projection.dhash_hex)
        } else {
            None
        };
        if let Some(hash) = target_hash.as_deref() {
            for hit in self
                .find_similar_assets(hash, 16, 100, request.similar_to_asset_id)
                .await?
            {
                similarity_distances.insert(hit.asset_internal_id, hit.distance);
            }
            if let Some(asset_id) = request.similar_to_asset_id {
                similarity_distances.insert(asset_id, 0);
            }
        }

        let query_embedding = if vector_requested {
            match llm {
                Some(llm) => embed_ckc_text(llm, &query).await?,
                None => None,
            }
        } else {
            None
        };
        let semantic_available = query_embedding.is_some();
        let semantic_scores = if let (Some(llm), Some(query_embedding)) = (llm, &query_embedding) {
            self.refresh_ckc_semantic_projections(&candidates, llm)
                .await?;
            self.ckc_semantic_scores_for_candidates(&candidates, query_embedding)
                .await?
        } else {
            HashMap::new()
        };
        let vector_source = match (vector_requested, semantic_available, target_hash.is_some()) {
            (true, true, true) => "llm_embedding+surreal_vector_projection+dhash_similarity",
            (true, true, false) => "llm_embedding+surreal_vector_projection",
            (true, false, true) => "semantic_unavailable_no_embedding_model+dhash_similarity",
            (true, false, false) => "semantic_unavailable_no_embedding_model",
            (false, _, true) => "dhash_similarity",
            (false, _, false) => "not_requested",
        }
        .to_owned();

        let mut scored = Vec::new();
        for candidate in candidates {
            let token_match = ckc_candidate_matches_query_terms(&candidate, &query_terms);
            let fuzzy_matched =
                query.is_empty() || token_match || candidate.fuzzy_score > CKC_FUZZY_THRESHOLD;
            let fuzzy_score = if token_match && !query.is_empty() {
                candidate.fuzzy_score.max(0.75)
            } else {
                candidate.fuzzy_score
            };
            let semantic_score = semantic_scores
                .get(&candidate.target_ref)
                .copied()
                .unwrap_or(0.0);
            let similarity_distance = candidate
                .asset_id
                .and_then(|asset_id| similarity_distances.get(&asset_id).copied());
            let similarity_score = similarity_distance
                .map(|distance| 1.0 - (f64::from(distance.clamp(0, 64)) / 64.0))
                .unwrap_or(0.0);
            let vector_score = semantic_score.max(similarity_score);
            let mut match_modes = BTreeSet::new();
            if wants_fuzzy(&modes) && fuzzy_matched {
                match_modes.insert("fuzzy".to_owned());
            }
            if vector_requested && semantic_score > 0.0 {
                match_modes.insert("vector".to_owned());
            }
            if vector_requested && similarity_distance.is_some() {
                match_modes.insert("image_similarity".to_owned());
            }
            if wants_combined(&modes) {
                match_modes.insert("combined".to_owned());
            }
            if wants_combined(&modes) && !query.is_empty() && !fuzzy_matched {
                continue;
            }
            if wants_combined(&modes) && similarity_distance.is_none() && target_hash.is_some() {
                continue;
            }
            if wants_combined(&modes)
                && target_hash.is_none()
                && semantic_available
                && semantic_score <= 0.0
            {
                continue;
            }
            if modes.as_slice() == [CkcSearchMode::Vector]
                && semantic_score <= 0.0
                && similarity_distance.is_none()
            {
                continue;
            }
            let tag_note_hits = Self::ckc_tag_notes_for_candidate(&candidate, &tag_notes);
            scored.push(CkcSearchHit {
                target_kind: candidate.target_kind.to_owned(),
                target_ref: candidate.target_ref.clone(),
                title: candidate.title.clone(),
                snippet: text_snippet(&candidate.search_text, &query),
                character_ref: candidate.character_internal_id.map(character_ref),
                sheet_version_ref: candidate.sheet_version_id.and_then(|version_id| {
                    candidate
                        .character_internal_id
                        .map(|character_id| sheet_version_ref(character_id, version_id))
                }),
                collection_ref: candidate.collection_id.map(collection_ref),
                media_ref: candidate.asset_id.map(media_asset_ref),
                tag_ref: candidate.tag_id.map(tag_ref),
                tags: candidate.tags.clone(),
                tag_notes: tag_note_hits,
                match_modes: match_modes.into_iter().collect(),
                fuzzy_score,
                vector_score,
                similarity_distance,
            });
        }

        scored.sort_by(|a, b| {
            let a_score = a.fuzzy_score + a.vector_score;
            let b_score = b.fuzzy_score + b.vector_score;
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.target_kind.cmp(&b.target_kind))
                .then_with(|| a.target_ref.cmp(&b.target_ref))
        });
        scored.truncate(limit as usize);

        Ok(CkcSearchResponse {
            query,
            search_modes: mode_strings(&modes),
            semantic_available,
            vector_source,
            result_count: scored.len(),
            results: scored,
        })
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorage, SurrealStorageConfig};

    #[test]
    fn hex64_validation() {
        assert!(is_hex64("0123456789abcdef"));
        assert!(is_hex64("FFFFFFFFFFFFFFFF"));
        assert!(!is_hex64("0123456789abcde")); // 15 chars
        assert!(!is_hex64("0123456789abcdeg")); // non-hex
        assert!(!is_hex64(""));
    }

    #[test]
    fn hamming_matches_legacy_source_semantics() {
        // Identical hashes => 0.
        assert_eq!(
            hamming_distance_hex64("0000000000000000", "0000000000000000"),
            0
        );
        // Single bit set => 1.
        assert_eq!(
            hamming_distance_hex64("0000000000000000", "0000000000000001"),
            1
        );
        // All bits differ => 64.
        assert_eq!(
            hamming_distance_hex64("0000000000000000", "ffffffffffffffff"),
            64
        );
        // Invalid input => max distance 64.
        assert_eq!(hamming_distance_hex64("zzzz", "0000000000000000"), 64);
    }

    #[test]
    fn tag_normalization() {
        assert_eq!(normalize_tag("  BlondE "), "blonde");
        assert_eq!(normalize_tag("Red Hair"), "red hair");
    }

    #[tokio::test]
    async fn similarity_exclusion_uses_uuid_record_identity_in_embedded_surreal() {
        #[derive(SurrealValue)]
        struct SeedBindings {
            excluded: RecordId,
            included: RecordId,
            excluded_hash: String,
            included_hash: String,
        }

        let directory = tempfile::tempdir().expect("create isolated SurrealDB directory");
        eprintln!("MT136_SIMILARITY_STEP open");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::for_data_dir(directory.path()).expect("build storage config"),
        )
        .await
        .expect("open embedded SurrealDB");
        eprintln!("MT136_SIMILARITY_STEP bootstrap");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded SurrealDB schema");
        eprintln!("MT136_SIMILARITY_STEP seed_assets");

        let excluded_id = Uuid::now_v7();
        let included_id = Uuid::now_v7();
        let target_hash = "0000000000000000";
        storage
            .with_data_operation({
                let bindings = SeedBindings {
                    excluded: RecordId::new(
                        "atelier_media_asset",
                        SurrealUuid::from(excluded_id),
                    ),
                    included: RecordId::new(
                        "atelier_media_asset",
                        SurrealUuid::from(included_id),
                    ),
                    excluded_hash: format!("mt136-excluded-{excluded_id}"),
                    included_hash: format!("mt136-included-{included_id}"),
                };
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_values::<surrealdb::types::Value, _>(
                            "CREATE $excluded CONTENT { asset_id: record::id($excluded), content_hash: $excluded_hash, mime: 'image/png', byte_len: 1, artifact_ref: 'artifact://mt136/excluded' } RETURN NONE; CREATE $included CONTENT { asset_id: record::id($included), content_hash: $included_hash, mime: 'image/png', byte_len: 1, artifact_ref: 'artifact://mt136/included' } RETURN NONE;",
                            bindings,
                        )
                        .await
                        .map(|_| ())
                    })
                }
            })
            .await
            .expect("seed UUID-keyed media assets");

        let store = AtelierStore::new(storage.clone());
        eprintln!("MT136_SIMILARITY_STEP upsert_excluded");
        store
            .upsert_similarity_projection(excluded_id, Some(target_hash), serde_json::json!({}))
            .await
            .expect("store excluded projection");
        eprintln!("MT136_SIMILARITY_STEP upsert_included");
        store
            .upsert_similarity_projection(
                included_id,
                Some("0000000000000001"),
                serde_json::json!({}),
            )
            .await
            .expect("store included projection");

        eprintln!("MT136_SIMILARITY_STEP query_exclusion");
        let hits = store
            .find_similar_assets(target_hash, 4, 50, Some(excluded_id))
            .await
            .expect("query similarity projection with exclusion");
        assert!(
            hits.iter().any(|hit| hit.asset_internal_id == included_id),
            "the non-excluded UUID-keyed asset must remain visible"
        );
        assert!(
            hits.iter().all(|hit| hit.asset_internal_id != excluded_id),
            "the excluded UUID-keyed asset must be absent"
        );

        eprintln!("MT136_SIMILARITY_STEP shutdown");
        drop(store);
        storage.shutdown().await.expect("close embedded SurrealDB");
        drop(storage);
        directory
            .close()
            .expect("remove isolated SurrealDB directory");
        eprintln!("MT136_SIMILARITY_STEP complete");
    }
}
