//! Core search / tags / similarity (WP-KERNEL-005, MT-005 event coverage).
//!
//! Legacy source fold-in (translate behavior, NOT SQLite storage):
//!   * `app/backend/library.js` tag manager + TagRule CRUD + `_upsertDerivedTags`
//!     (deterministic rule ordering by `rule_id ASC`) + bulk/manual tagging.
//!   * `app/backend/dhash.js` `hammingDistanceHex64` / `isHex64` 64-bit perceptual
//!     hash distance used for near-duplicate / similarity search.
//!   * `app/backend/palette.js` dominant-palette projection persisted per asset.
//!
//! Storage authority is PostgreSQL (sqlx 0.8) ONLY. SQLite is forbidden in any
//! form. Every mutation emits an atelier event from the new families defined
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
use sqlx::Row;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

use crate::llm::{EmbeddingRequest, LlmClient, LlmError};

use super::{
    character_ref, collection_ref, event_ref_for_text, media_asset_ref, parse_character_ref,
    parse_collection_ref, parse_media_asset_ref, parse_sheet_version_ref, parse_tag_ref,
    reject_legacy_runtime_ref, sheet_version_ref, tag_ref, AtelierError, AtelierResult,
    AtelierStore, BulkTagRequest,
};

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
    /// A rich CKC tag note was created or updated.
    pub const TAG_NOTE_UPSERTED: &str = "atelier.tag.note_upserted";

    /// All search/tags/similarity event families (parity / coverage checks).
    pub const ALL: &[&str] = &[
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
        TAG_NOTE_UPSERTED,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CkcTagNote {
    pub tag_note_id: Uuid,
    pub tag_id: Uuid,
    pub tag_ref: String,
    pub tag_text: String,
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

fn similarity_rebuild_job_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<SimilarityRebuildJob> {
    let status: String = row.get("status");
    Ok(SimilarityRebuildJob {
        job_id: row.get("job_id"),
        asset_internal_id: row.get("asset_internal_id"),
        status: SimilarityRebuildJobStatus::parse(&status)?,
        requested_by: row.get("requested_by"),
        processed_count: row.get("processed_count"),
        failed_count: row.get("failed_count"),
        dhash_hex: row.get("dhash_hex"),
        palette_json: row.get("palette_json"),
        error_ref: row.get("error_ref"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
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

fn global_search_hit_from_row(
    row: &sqlx::postgres::PgRow,
    query: &str,
    view_mode: LensViewMode,
) -> AtelierResult<GlobalSearchHit> {
    let search_text: String = row.get("search_text");
    let extraction_tier_raw: String = row.get("extraction_tier");
    let content_tier_raw: Option<String> = row.get("content_tier");
    Ok(GlobalSearchHit {
        target_kind: row.get("target_kind"),
        target_id: row.get("target_id"),
        jump_target: row.get("jump_target"),
        title: row.get("title"),
        snippet: bounded_search_snippet(&search_text, query),
        rank: row.get("rank"),
        extraction_tier: LensExtractionTier::parse(&extraction_tier_raw)?,
        content_tier: content_tier_raw
            .as_deref()
            .map(LensContentTier::parse)
            .transpose()?,
        view_mode,
    })
}

fn saved_search_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<SavedSearch> {
    let include_tags_json: serde_json::Value = row.get("include_tags_json");
    let exclude_tags_json: serde_json::Value = row.get("exclude_tags_json");
    let scope_kind: String = row.get("scope_kind");
    let scope_id: Option<Uuid> = row.get("scope_id");
    let view_mode: String = row.get("view_mode");
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
        saved_search_id: row.get("saved_search_id"),
        name: row.get("name"),
        filters: SavedSearchFilters {
            include_tags: saved_search_tags_from_json(include_tags_json),
            exclude_tags: saved_search_tags_from_json(exclude_tags_json),
            min_rating: row.get("min_rating"),
            favorite: row.get("favorite"),
            color_hex: row.get("color_hex"),
            scope: SavedSearchScope::from_parts(&scope_kind, scope_id)?,
            view_mode,
        },
        created_by: row.get("created_by"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn saved_search_projection_hit_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<SavedSearchProjectionHit> {
    let tags_json: serde_json::Value = row.get("tags_json");
    let content_tier: Option<String> = row.get("content_tier");
    let view_mode: String = row.get("view_mode");
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
        saved_search_id: row.get("saved_search_id"),
        asset_id: row.get("asset_id"),
        content_hash: row.get("content_hash"),
        artifact_ref: row.get("artifact_ref"),
        jump_target: row.get("jump_target"),
        tags: saved_search_tags_from_json(tags_json),
        favorite: row.get("favorite"),
        rating: row.get("rating"),
        matched_color_hex: row.get("matched_color_hex"),
        content_tier: content_tier
            .as_deref()
            .map(LensContentTier::parse)
            .transpose()?,
        view_mode,
    })
}

fn ai_tag_suggestion_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<AiTagSuggestion> {
    let status: String = row.get("status");
    Ok(AiTagSuggestion {
        suggestion_id: row.get("suggestion_id"),
        character_internal_id: row.get("character_internal_id"),
        asset_id: row.get("asset_id"),
        tag_text: row.get("tag_text"),
        confidence: row.get("confidence"),
        model_receipt_ref: row.get("model_receipt_ref"),
        tool_receipt_ref: row.get("tool_receipt_ref"),
        suggested_by: row.get("suggested_by"),
        status: AiTagSuggestionStatus::parse(&status)?,
        decided_by: row.get("decided_by"),
        decision_reason: row.get("decision_reason"),
        applied_tag_id: row.get("applied_tag_id"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
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

fn tag_from_row(row: &sqlx::postgres::PgRow) -> Tag {
    Tag {
        tag_id: row.get("tag_id"),
        text: row.get("text"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn tag_note_from_row(row: &sqlx::postgres::PgRow) -> CkcTagNote {
    let tag_id: Uuid = row.get("tag_id");
    CkcTagNote {
        tag_note_id: row.get("tag_note_id"),
        tag_id,
        tag_ref: tag_ref(tag_id),
        tag_text: row.get("tag_text"),
        scope_ref: row.get("scope_ref"),
        note: row.get("note"),
        updated_by: row.get("updated_by"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn rule_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<TagRule> {
    let match_type_raw: String = row.get("match_type");
    Ok(TagRule {
        rule_id: row.get("rule_id"),
        source_field_id: row.get("source_field_id"),
        match_type: MatchType::parse(&match_type_raw)?,
        pattern: row.get("pattern"),
        emit_tag: row.get("emit_tag"),
        enabled: row.get("enabled"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

#[derive(Clone, Debug)]
struct CkcSearchCandidate {
    target_kind: String,
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

fn tags_from_json(value: serde_json::Value) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(values) = value.as_array() {
        for value in values {
            if let Some(tag) = value.as_str() {
                let normalized = normalize_tag(tag);
                if !normalized.is_empty() && !tags.iter().any(|existing| existing == &normalized) {
                    tags.push(normalized);
                }
            }
        }
    }
    tags.sort();
    tags
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

fn format_ckc_pgvector_literal(embedding: &[f32]) -> String {
    let mut out = String::with_capacity(embedding.len() * 8 + 2);
    out.push('[');
    for (idx, value) in embedding.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        if value.is_finite() {
            out.push_str(&format!("{value}"));
        } else {
            out.push('0');
        }
    }
    out.push(']');
    out
}

async fn embed_ckc_text(llm: &dyn LlmClient, text: &str) -> AtelierResult<Option<Vec<f32>>> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let model_id = llm.profile().model_id.clone();
    let req = EmbeddingRequest::new(Uuid::now_v7(), text.to_owned(), model_id);
    match llm.embedding(req).await {
        Ok(response) => {
            if response.vector.len() != crate::loom_search::LOOM_SEARCH_EMBEDDING_DIM {
                return Err(AtelierError::Validation(format!(
                    "CKC search embedding dimensionality mismatch: expected {}, got {}",
                    crate::loom_search::LOOM_SEARCH_EMBEDDING_DIM,
                    response.vector.len()
                )));
            }
            Ok(Some(response.vector))
        }
        Err(LlmError::EmbeddingUnsupported) | Err(_) => Ok(None),
    }
}

fn candidate_from_row(row: &sqlx::postgres::PgRow) -> CkcSearchCandidate {
    CkcSearchCandidate {
        target_kind: row.get("target_kind"),
        target_ref: row.get("target_ref"),
        title: row.get("title"),
        search_text: row.get("search_text"),
        fuzzy_score: row.get("fuzzy_score"),
        character_internal_id: row.get("character_internal_id"),
        sheet_version_id: row.get("sheet_version_id"),
        collection_id: row.get("collection_id"),
        asset_id: row.get("asset_id"),
        tag_id: row.get("tag_id"),
        tags: tags_from_json(row.get("tags_json")),
    }
}

fn candidate_scope_refs(candidate: &CkcSearchCandidate) -> Vec<String> {
    let mut refs = Vec::new();
    refs.push(candidate.target_ref.clone());
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

impl AtelierStore {
    async fn ensure_tag_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        text: &str,
    ) -> AtelierResult<Tag> {
        let norm = normalize_tag(text);
        if norm.is_empty() {
            return Err(AtelierError::Validation(
                "tag text must not be empty".into(),
            ));
        }
        let row = sqlx::query(
            r#"INSERT INTO atelier_tag (text)
               VALUES ($1)
               ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
               RETURNING tag_id, text, created_at_utc"#,
        )
        .bind(&norm)
        .fetch_one(&mut **tx)
        .await?;
        Ok(tag_from_row(&row))
    }

    async fn validate_ckc_tag_note_scope_ref(&self, scope_ref: &str) -> AtelierResult<()> {
        let exists = if let Some(id) = parse_character_ref(scope_ref) {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM atelier_character WHERE internal_id = $1)",
            )
            .bind(id)
            .fetch_one(self.pool())
            .await?
        } else if let Some((character_id, version_id)) = parse_sheet_version_ref(scope_ref) {
            sqlx::query_scalar::<_, bool>(
                r#"SELECT EXISTS(
                       SELECT 1 FROM atelier_sheet_version
                       WHERE character_internal_id = $1
                         AND version_id = $2
                   )"#,
            )
            .bind(character_id)
            .bind(version_id)
            .fetch_one(self.pool())
            .await?
        } else if let Some(id) = parse_collection_ref(scope_ref) {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM atelier_collection WHERE collection_id = $1)",
            )
            .bind(id)
            .fetch_one(self.pool())
            .await?
        } else if let Some(id) = parse_media_asset_ref(scope_ref) {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM atelier_media_asset WHERE asset_id = $1)",
            )
            .bind(id)
            .fetch_one(self.pool())
            .await?
        } else if let Some(id) = parse_tag_ref(scope_ref) {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM atelier_tag WHERE tag_id = $1)",
            )
            .bind(id)
            .fetch_one(self.pool())
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

        let mut tx = self.pool().begin().await?;
        let tag = self.ensure_tag_in_tx(&mut tx, &tag_text).await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_tag_note (tag_id, scope_ref, note, updated_by)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (tag_id, (COALESCE(scope_ref, '')))
               DO UPDATE SET note = EXCLUDED.note,
                             updated_by = EXCLUDED.updated_by,
                             updated_at_utc = NOW()
               RETURNING tag_note_id, tag_id,
                         (SELECT text FROM atelier_tag WHERE tag_id = atelier_tag_note.tag_id) AS tag_text,
                         scope_ref, note, updated_by, created_at_utc, updated_at_utc"#,
        )
        .bind(tag.tag_id)
        .bind(&scope_ref)
        .bind(note)
        .bind(updated_by)
        .fetch_one(&mut *tx)
        .await?;
        let tag_note = tag_note_from_row(&row);
        self.record_event_in_tx(
            &mut tx,
            search_event_family::TAG_NOTE_UPSERTED,
            "atelier_tag_note",
            &tag_note.tag_note_id.to_string(),
            serde_json::json!({
                "tag_note_id": tag_note.tag_note_id,
                "tag_id": tag_note.tag_id,
                "tag_ref": tag_note.tag_ref,
                "tag_text": tag_note.tag_text,
                "scope_ref": tag_note.scope_ref,
                "updated_by": updated_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(tag_note)
    }

    async fn refresh_ckc_semantic_projections(
        &self,
        candidates: &[CkcSearchCandidate],
        llm: &dyn LlmClient,
    ) -> AtelierResult<()> {
        let embedding_model = llm.profile().model_id.clone();
        for candidate in candidates {
            let search_text_hash = event_ref_for_text(&candidate.search_text);
            let current: Option<(String, Option<String>, bool)> = sqlx::query_as(
                r#"SELECT search_text_hash, embedding_model, embedding IS NOT NULL
                   FROM atelier_ckc_search_projection
                   WHERE target_ref = $1"#,
            )
            .bind(&candidate.target_ref)
            .fetch_optional(self.pool())
            .await?;
            let is_current = current
                .as_ref()
                .is_some_and(|(hash, model, has_embedding)| {
                    hash == &search_text_hash
                        && model.as_deref() == Some(embedding_model.as_str())
                        && *has_embedding
                });
            if is_current {
                continue;
            }
            let Some(embedding) = embed_ckc_text(llm, &candidate.search_text).await? else {
                continue;
            };
            let embedding_literal = format_ckc_pgvector_literal(&embedding);
            sqlx::query(
                r#"INSERT INTO atelier_ckc_search_projection
                     (target_ref, target_kind, search_text_hash, embedding, embedding_model)
                   VALUES ($1, $2, $3, $4::public.vector, $5)
                   ON CONFLICT (target_ref)
                   DO UPDATE SET target_kind = EXCLUDED.target_kind,
                                 search_text_hash = EXCLUDED.search_text_hash,
                                 embedding = EXCLUDED.embedding,
                                 embedding_model = EXCLUDED.embedding_model,
                                 indexed_at_utc = NOW()"#,
            )
            .bind(&candidate.target_ref)
            .bind(&candidate.target_kind)
            .bind(&search_text_hash)
            .bind(&embedding_literal)
            .bind(&embedding_model)
            .execute(self.pool())
            .await?;
        }
        Ok(())
    }

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
        let query_literal = format_ckc_pgvector_literal(query_embedding);
        let rows = sqlx::query(
            r#"SELECT target_ref,
                      GREATEST(0.0, 1.0 - (embedding OPERATOR(public.<=>) $1::public.vector))::float8
                          AS semantic_score
               FROM atelier_ckc_search_projection
               WHERE embedding IS NOT NULL
                 AND target_ref = ANY($2)"#,
        )
        .bind(&query_literal)
        .bind(&target_refs)
        .fetch_all(self.pool())
        .await?;
        let mut out = HashMap::new();
        for row in rows {
            out.insert(
                row.get::<String, _>("target_ref"),
                row.get("semantic_score"),
            );
        }
        Ok(out)
    }

    async fn ckc_search_candidates(
        &self,
        request: &CkcSearchRequest,
        modes: &[CkcSearchMode],
        prefetch_limit: i64,
    ) -> AtelierResult<Vec<CkcSearchCandidate>> {
        let query = request.query.trim();
        let allow_vector_scan = wants_vector(modes);
        let rows = sqlx::query(
            r#"WITH character_tags AS (
                   SELECT ct.character_internal_id,
                          jsonb_agg(DISTINCT t.text ORDER BY t.text) AS tags_json
                   FROM atelier_character_tag ct
                   JOIN atelier_tag t ON t.tag_id = ct.tag_id
                   GROUP BY ct.character_internal_id
               ),
               media_tags AS (
                   SELECT mat.asset_id,
                          jsonb_agg(DISTINCT t.text ORDER BY t.text) AS tags_json
                   FROM atelier_media_asset_tag mat
                   JOIN atelier_tag t ON t.tag_id = mat.tag_id
                   GROUP BY mat.asset_id
               ),
               latest_sheet AS (
                   SELECT DISTINCT ON (character_internal_id)
                          character_internal_id, version_id, raw_text, seq
                   FROM atelier_sheet_version
                   ORDER BY character_internal_id, seq DESC, created_at_utc DESC
               ),
               candidates AS (
                   SELECT 'character'::text AS target_kind,
                          concat('atelier://character/', c.internal_id::text) AS target_ref,
                          c.display_name AS title,
                          concat_ws(E'\n', c.display_name, c.public_id, ls.raw_text, ct.tags_json::text) AS search_text,
                          c.internal_id AS character_internal_id,
                          ls.version_id AS sheet_version_id,
                          NULL::uuid AS collection_id,
                          NULL::uuid AS asset_id,
                          NULL::uuid AS tag_id,
                          COALESCE(ct.tags_json, '[]'::jsonb) AS tags_json,
                          NULL::text AS dhash_hex
                   FROM atelier_character c
                   LEFT JOIN latest_sheet ls ON ls.character_internal_id = c.internal_id
                   LEFT JOIN character_tags ct ON ct.character_internal_id = c.internal_id

                   UNION ALL

                   SELECT 'sheet'::text AS target_kind,
                          concat('atelier://sheet/', sv.character_internal_id::text, '/', sv.version_id::text) AS target_ref,
                          concat(c.display_name, ' sheet v', sv.seq::text) AS title,
                          concat_ws(E'\n', c.display_name, c.public_id, sv.raw_text, ct.tags_json::text) AS search_text,
                          sv.character_internal_id,
                          sv.version_id AS sheet_version_id,
                          NULL::uuid AS collection_id,
                          NULL::uuid AS asset_id,
                          NULL::uuid AS tag_id,
                          COALESCE(ct.tags_json, '[]'::jsonb) AS tags_json,
                          NULL::text AS dhash_hex
                   FROM atelier_sheet_version sv
                   JOIN atelier_character c ON c.internal_id = sv.character_internal_id
                   LEFT JOIN character_tags ct ON ct.character_internal_id = c.internal_id

                   UNION ALL

                   SELECT 'album'::text AS target_kind,
                          concat('atelier://collection/', co.collection_id::text) AS target_ref,
                          co.name AS title,
                          concat_ws(E'\n', co.name, co.notes, co.tags_json::text, c.display_name, ls.raw_text, ct.tags_json::text) AS search_text,
                          co.character_internal_id,
                          co.sheet_version_id,
                          co.collection_id,
                          NULL::uuid AS asset_id,
                          NULL::uuid AS tag_id,
                          COALESCE(co.tags_json, '[]'::jsonb) || COALESCE(ct.tags_json, '[]'::jsonb) AS tags_json,
                          NULL::text AS dhash_hex
                   FROM atelier_collection co
                   LEFT JOIN atelier_character c ON c.internal_id = co.character_internal_id
                   LEFT JOIN latest_sheet ls ON ls.character_internal_id = co.character_internal_id
                   LEFT JOIN character_tags ct ON ct.character_internal_id = co.character_internal_id
                   WHERE co.character_internal_id IS NOT NULL

                   UNION ALL

                   SELECT 'media'::text AS target_kind,
                          concat('atelier://media/', ma.asset_id::text) AS target_ref,
                          concat(COALESCE(NULLIF(ma.source_provenance, ''), left(ma.content_hash, 16)), ' / ', co.name) AS title,
                          concat_ws(E'\n', ma.content_hash, ma.mime, ma.source_provenance, ma.artifact_ref,
                                    mrm.notes, mrm.review_status, mt.tags_json::text,
                                    co.name, co.notes, co.tags_json::text, c.display_name, ls.raw_text, ct.tags_json::text) AS search_text,
                          co.character_internal_id,
                          co.sheet_version_id,
                          co.collection_id,
                          ma.asset_id,
                          NULL::uuid AS tag_id,
                          COALESCE(mt.tags_json, '[]'::jsonb) || COALESCE(co.tags_json, '[]'::jsonb) || COALESCE(ct.tags_json, '[]'::jsonb) AS tags_json,
                          sp.dhash_hex
                   FROM atelier_collection_item ci
                   JOIN atelier_collection co ON co.collection_id = ci.collection_id
                   JOIN atelier_media_asset ma ON ma.asset_id = ci.asset_id
                   LEFT JOIN atelier_media_review_metadata mrm ON mrm.asset_id = ma.asset_id
                   LEFT JOIN atelier_similarity_projection sp ON sp.asset_internal_id = ma.asset_id
                   LEFT JOIN media_tags mt ON mt.asset_id = ma.asset_id
                   LEFT JOIN atelier_character c ON c.internal_id = co.character_internal_id
                   LEFT JOIN latest_sheet ls ON ls.character_internal_id = co.character_internal_id
                   LEFT JOIN character_tags ct ON ct.character_internal_id = co.character_internal_id
                   WHERE co.character_internal_id IS NOT NULL

                   UNION ALL

                   SELECT 'tag'::text AS target_kind,
                          concat('atelier://tag/', t.tag_id::text) AS target_ref,
                          t.text AS title,
                          concat_ws(E'\n', t.text, tn.note, tn.scope_ref) AS search_text,
                          NULL::uuid AS character_internal_id,
                          NULL::uuid AS sheet_version_id,
                          NULL::uuid AS collection_id,
                          NULL::uuid AS asset_id,
                          t.tag_id,
                          jsonb_build_array(t.text) AS tags_json,
                          NULL::text AS dhash_hex
                   FROM atelier_tag t
                   LEFT JOIN atelier_tag_note tn ON tn.tag_id = t.tag_id
               )
               SELECT target_kind, target_ref, title, search_text,
                      character_internal_id, sheet_version_id, collection_id,
                      asset_id, tag_id, tags_json, dhash_hex,
                      CASE WHEN $1::text = '' THEN 0.0::float8
                           ELSE GREATEST(
                               public.atelier_trgm_similarity(search_text, $1::text)::float8,
                               public.atelier_trgm_similarity(title, $1::text)::float8,
                               CASE WHEN position(lower($1::text) in lower(search_text)) > 0 THEN 1.0 ELSE 0.0 END
                               ,
                               CASE WHEN position(lower($1::text) in lower(title)) > 0 THEN 1.0 ELSE 0.0 END
                           )
                      END AS fuzzy_score
               FROM candidates
               WHERE ($2::uuid IS NULL OR character_internal_id = $2)
                 AND ($3::uuid IS NULL OR collection_id = $3)
                 AND ($4::uuid IS NULL OR asset_id = $4)
                 AND (
                    $1::text = ''
                    OR $5::bool
                    OR position(lower($1::text) in lower(search_text)) > 0
                    OR position(lower($1::text) in lower(title)) > 0
                    OR public.atelier_trgm_similarity(search_text, $1::text) > 0.08
                    OR public.atelier_trgm_similarity(title, $1::text) > 0.08
                 )
               ORDER BY fuzzy_score DESC, target_kind ASC, target_ref ASC
               LIMIT $6"#,
        )
        .bind(query)
        .bind(request.character_internal_id)
        .bind(request.collection_id)
        .bind(request.media_asset_id)
        .bind(allow_vector_scan)
        .bind(prefetch_limit)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(candidate_from_row).collect())
    }

    async fn ckc_tag_notes_for_candidate(
        &self,
        candidate: &CkcSearchCandidate,
    ) -> AtelierResult<Vec<CkcTagNoteHit>> {
        if candidate.tags.is_empty() {
            return Ok(Vec::new());
        }
        let scope_refs = candidate_scope_refs(candidate);
        let rows = sqlx::query(
            r#"SELECT tn.tag_note_id, tn.tag_id, t.text AS tag_text,
                      tn.scope_ref, tn.note, tn.updated_by,
                      tn.created_at_utc, tn.updated_at_utc
               FROM atelier_tag_note tn
               JOIN atelier_tag t ON t.tag_id = tn.tag_id
               WHERE t.text = ANY($1)
                 AND (
                    tn.scope_ref IS NULL
                    OR tn.scope_ref = ANY($2)
                 )
               ORDER BY t.text ASC, tn.scope_ref ASC NULLS FIRST"#,
        )
        .bind(&candidate.tags)
        .bind(&scope_refs)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .iter()
            .map(tag_note_from_row)
            .map(|row| CkcTagNoteHit {
                tag_ref: row.tag_ref,
                tag_text: row.tag_text,
                scope_ref: row.scope_ref,
                note: row.note,
            })
            .collect())
    }

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
        };
        let required_tags = normalized_search_tags(&request.tags);
        let query_terms = ckc_query_terms(&query);
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
            (true, true, true) => "llm_embedding+pgvector_projection+dhash_similarity",
            (true, true, false) => "llm_embedding+pgvector_projection",
            (true, false, true) => "semantic_unavailable_no_embedding_model+dhash_similarity",
            (true, false, false) => "semantic_unavailable_no_embedding_model",
            (false, _, true) => "dhash_similarity",
            (false, _, false) => "not_requested",
        }
        .to_owned();

        let mut scored = Vec::new();
        for candidate in candidates {
            let token_match = ckc_candidate_matches_query_terms(&candidate, &query_terms);
            let fuzzy_matched = query.is_empty() || token_match || candidate.fuzzy_score > 0.08;
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
            if !wants_vector(&modes) && !wants_fuzzy(&modes) && match_modes.is_empty() {
                match_modes.insert("fuzzy".to_owned());
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
            let tag_notes = self.ckc_tag_notes_for_candidate(&candidate).await?;
            let hit = CkcSearchHit {
                target_kind: candidate.target_kind.clone(),
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
                tag_notes,
                match_modes: match_modes.into_iter().collect(),
                fuzzy_score,
                vector_score,
                similarity_distance,
            };
            scored.push(hit);
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

    /// Search across sheet text, character documents, moodboard snapshots, and
    /// media rows with stable jump targets. This is PostgreSQL-backed pattern
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
        let rows = sqlx::query(
            r#"WITH candidates AS (
                   SELECT 'sheet'::text AS target_kind,
                          sv.version_id::text AS target_id,
                          concat('atelier://sheet/', sv.character_internal_id::text, '/', sv.version_id::text) AS jump_target,
                          concat('Sheet v', sv.seq::text, ' - ', c.display_name) AS title,
                          sv.raw_text AS search_text,
                          10::bigint AS rank,
                          sv.created_at_utc AS sort_at
                   FROM atelier_sheet_version sv
                   JOIN atelier_character c
                     ON c.internal_id = sv.character_internal_id

                   UNION ALL

                   SELECT CASE d.doc_type
                              WHEN 'note' THEN 'note'
                              WHEN 'story' THEN 'story_document'
                              WHEN 'moodboard' THEN 'moodboard_document'
                              ELSE 'document'
                          END AS target_kind,
                          d.document_id::text AS target_id,
                          concat('atelier://document/', d.document_id::text) AS jump_target,
                          v.title AS title,
                          concat_ws(E'\n', v.title, v.body_raw_text, d.tags_json::text) AS search_text,
                          20::bigint AS rank,
                          v.created_at_utc AS sort_at
                   FROM atelier_character_document d
                   JOIN atelier_character_document_version v
                     ON v.version_id = d.current_version_id

                   UNION ALL

                   SELECT 'moodboard_snapshot'::text AS target_kind,
                          m.snapshot_id::text AS target_id,
                          concat('atelier://moodboard/', m.snapshot_id::text) AS jump_target,
                          COALESCE(NULLIF(m.moodboard_json->>'name', ''), 'Moodboard') AS title,
                          m.raw_json_text AS search_text,
                          30::bigint AS rank,
                          m.created_at_utc AS sort_at
                   FROM atelier_moodboard m

                   UNION ALL

                   SELECT 'image'::text AS target_kind,
                          ma.asset_id::text AS target_id,
                          concat('atelier://image/', ma.asset_id::text) AS jump_target,
                          concat(ma.mime, ' ', left(ma.content_hash, 12)) AS title,
                          concat_ws(' ', ma.mime, ma.content_hash, ma.source_provenance, ma.artifact_ref) AS search_text,
                          40::bigint AS rank,
                          ma.created_at_utc AS sort_at
                   FROM atelier_media_asset ma
               ),
               annotated AS (
                   SELECT target_kind, target_id, jump_target, title, search_text,
                          lower(search_text) AS lower_search_text, rank, sort_at
                   FROM candidates
               ),
               projected AS (
                   SELECT target_kind, target_id, jump_target, title, search_text,
                          rank, sort_at,
                          CASE
                              WHEN lower_search_text ~ '(lens[_ -]?)?extraction[_ -]?tier[^a-z0-9]+tier3' THEN 'tier3'
                              WHEN lower_search_text ~ '(lens[_ -]?)?extraction[_ -]?tier[^a-z0-9]+tier2' THEN 'tier2'
                              ELSE 'tier1'
                          END AS extraction_tier,
                          CASE
                              WHEN lower_search_text ~ 'content[_ -]?tier[^a-z0-9]+adult[_ -]?explicit' THEN 'adult_explicit'
                              WHEN lower_search_text ~ 'content[_ -]?tier[^a-z0-9]+adult[_ -]?soft' THEN 'adult_soft'
                              WHEN lower_search_text ~ 'content[_ -]?tier[^a-z0-9]+sfw' THEN 'sfw'
                              ELSE NULL
                          END AS content_tier
                   FROM annotated
                   WHERE position(lower($1::text) in lower_search_text) > 0
               )
               SELECT target_kind, target_id, jump_target, title, search_text, rank,
                      extraction_tier, content_tier
               FROM projected
               WHERE CASE extraction_tier
                         WHEN 'tier1' THEN 1
                         WHEN 'tier2' THEN 2
                         ELSE 3
                     END <= $2
                 AND ($3::text <> 'SFW' OR content_tier = 'sfw')
                ORDER BY rank ASC, sort_at DESC, target_id ASC
                LIMIT $4"#,
        )
        .bind(trimmed)
        .bind(filters.extraction_tier.rank())
        .bind(filters.view_mode.as_str())
        .bind(limit)
        .fetch_all(self.pool())
        .await?;
        rows.iter()
            .map(|row| global_search_hit_from_row(row, trimmed, filters.view_mode))
            .collect()
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
            let exists: Option<Uuid> = sqlx::query_scalar(
                "SELECT collection_id FROM atelier_collection WHERE collection_id = $1",
            )
            .bind(collection_id)
            .fetch_optional(self.pool())
            .await?;
            if exists.is_none() {
                return Err(AtelierError::NotFound(format!(
                    "saved search collection scope not found: {collection_id}"
                )));
            }
        }
        let include_tags_json = serde_json::Value::from(filters.include_tags.clone());
        let exclude_tags_json = serde_json::Value::from(filters.exclude_tags.clone());
        let (scope_kind, scope_id) = filters.scope.into_parts();
        let row = sqlx::query(
            r#"INSERT INTO atelier_saved_search (
                   name, include_tags_json, exclude_tags_json, min_rating,
                   favorite, color_hex, scope_kind, scope_id, view_mode, created_by
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (name) DO UPDATE SET
                   include_tags_json = EXCLUDED.include_tags_json,
                   exclude_tags_json = EXCLUDED.exclude_tags_json,
                   min_rating = EXCLUDED.min_rating,
                   favorite = EXCLUDED.favorite,
                   color_hex = EXCLUDED.color_hex,
                   scope_kind = EXCLUDED.scope_kind,
                   scope_id = EXCLUDED.scope_id,
                   view_mode = EXCLUDED.view_mode,
                   created_by = EXCLUDED.created_by,
                   updated_at_utc = NOW()
               RETURNING saved_search_id, name, include_tags_json, exclude_tags_json,
                         min_rating, favorite, color_hex, scope_kind, scope_id,
                         view_mode, created_by, created_at_utc, updated_at_utc"#,
        )
        .bind(name)
        .bind(&include_tags_json)
        .bind(&exclude_tags_json)
        .bind(filters.min_rating)
        .bind(filters.favorite)
        .bind(&filters.color_hex)
        .bind(scope_kind)
        .bind(scope_id)
        .bind(filters.view_mode.as_str())
        .bind(created_by)
        .fetch_one(self.pool())
        .await?;
        let saved = saved_search_from_row(&row)?;
        self.record_event(
            search_event_family::SAVED_SEARCH_UPSERTED,
            "atelier_saved_search",
            &saved.saved_search_id.to_string(),
            serde_json::json!({
                "saved_search_id": saved.saved_search_id,
                "name": saved.name,
                "include_tags": saved.filters.include_tags,
                "exclude_tags": saved.filters.exclude_tags,
                "min_rating": saved.filters.min_rating,
                "favorite": saved.filters.favorite,
                "color_hex": saved.filters.color_hex,
                "scope": saved.filters.scope,
                "view_mode": saved.filters.view_mode,
                "created_by": created_by,
            }),
        )
        .await?;
        Ok(saved)
    }

    pub async fn get_saved_search(
        &self,
        saved_search_id: Uuid,
    ) -> AtelierResult<Option<SavedSearch>> {
        let row = sqlx::query(
            r#"SELECT saved_search_id, name, include_tags_json, exclude_tags_json,
                      min_rating, favorite, color_hex, scope_kind, scope_id,
                      view_mode, created_by, created_at_utc, updated_at_utc
               FROM atelier_saved_search
               WHERE saved_search_id = $1"#,
        )
        .bind(saved_search_id)
        .fetch_optional(self.pool())
        .await?;
        row.as_ref().map(saved_search_from_row).transpose()
    }

    pub async fn list_saved_searches(&self) -> AtelierResult<Vec<SavedSearch>> {
        let rows = sqlx::query(
            r#"SELECT saved_search_id, name, include_tags_json, exclude_tags_json,
                      min_rating, favorite, color_hex, scope_kind, scope_id,
                      view_mode, created_by, created_at_utc, updated_at_utc
               FROM atelier_saved_search
               ORDER BY updated_at_utc DESC, name ASC"#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(saved_search_from_row).collect()
    }

    pub async fn delete_saved_search(&self, saved_search_id: Uuid) -> AtelierResult<bool> {
        let removed = sqlx::query("DELETE FROM atelier_saved_search WHERE saved_search_id = $1")
            .bind(saved_search_id)
            .execute(self.pool())
            .await?;
        if removed.rows_affected() == 0 {
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
        let rows = sqlx::query(
            r#"SELECT saved_search_id, asset_id, content_hash, artifact_ref, jump_target,
                      tags_json, favorite, rating, matched_color_hex, content_tier, view_mode
               FROM atelier_saved_search_retrieval_projection
               WHERE saved_search_id = $1
               ORDER BY rating DESC, favorite DESC, created_at_utc DESC, asset_id ASC
               LIMIT $2"#,
        )
        .bind(saved_search_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;
        rows.iter()
            .map(saved_search_projection_hit_from_row)
            .collect()
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
        let row = sqlx::query(
            r#"INSERT INTO atelier_tag (text)
               VALUES ($1)
               ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
               RETURNING tag_id, text, created_at_utc"#,
        )
        .bind(&norm)
        .fetch_one(self.pool())
        .await?;
        Ok(tag_from_row(&row))
    }

    /// List every tag in the dictionary (ascending text). Mirrors the operator
    /// "all tags" picker in legacy source `listAllTags`.
    pub async fn list_all_tags(&self) -> AtelierResult<Vec<Tag>> {
        let rows = sqlx::query(
            r#"SELECT tag_id, text, created_at_utc FROM atelier_tag ORDER BY text ASC"#,
        )
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(tag_from_row).collect())
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
        sqlx::query(
            r#"INSERT INTO atelier_character_tag (character_internal_id, tag_id, tag_type)
               VALUES ($1, $2, $3)
               ON CONFLICT (character_internal_id, tag_id)
               DO UPDATE SET tag_type = EXCLUDED.tag_type"#,
        )
        .bind(character_internal_id)
        .bind(tag.tag_id)
        .bind(tag_type.as_str())
        .execute(self.pool())
        .await?;

        self.record_event(
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
        let removed = sqlx::query(
            r#"DELETE FROM atelier_character_tag ct
               USING atelier_tag t
               WHERE ct.tag_id = t.tag_id
                 AND ct.character_internal_id = $1
                 AND t.text = $2
                 AND ct.tag_type = 'manual'"#,
        )
        .bind(character_internal_id)
        .bind(&norm)
        .execute(self.pool())
        .await?;

        if removed.rows_affected() == 0 {
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
        let rows = sqlx::query(
            r#"SELECT ct.character_internal_id, ct.tag_id, t.text, ct.tag_type
               FROM atelier_character_tag ct
               JOIN atelier_tag t ON t.tag_id = ct.tag_id
               WHERE ct.character_internal_id = $1
               ORDER BY t.text ASC"#,
        )
        .bind(character_internal_id)
        .fetch_all(self.pool())
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let tag_type_raw: String = row.get("tag_type");
            let tag_type = match tag_type_raw.as_str() {
                "derived" => TagType::Derived,
                _ => TagType::Manual,
            };
            out.push(CharacterTag {
                character_internal_id: row.get("character_internal_id"),
                tag_id: row.get("tag_id"),
                text: row.get("text"),
                tag_type,
            });
        }
        Ok(out)
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

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_ai_tag_suggestion
                 (character_internal_id, asset_id, tag_text, confidence,
                  model_receipt_ref, tool_receipt_ref, suggested_by, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'proposed')
               RETURNING suggestion_id, character_internal_id, asset_id, tag_text,
                         confidence, model_receipt_ref, tool_receipt_ref,
                         suggested_by, status, decided_by, decision_reason,
                         applied_tag_id, created_at_utc, updated_at_utc"#,
        )
        .bind(new.character_internal_id)
        .bind(new.asset_id)
        .bind(&tag_text)
        .bind(confidence)
        .bind(&new.model_receipt_ref)
        .bind(&new.tool_receipt_ref)
        .bind(suggested_by)
        .fetch_one(&mut *tx)
        .await?;
        let suggestion = ai_tag_suggestion_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            search_event_family::AI_TAG_SUGGESTION_RECORDED,
            "atelier_ai_tag_suggestion",
            &suggestion.suggestion_id.to_string(),
            serde_json::json!({
                "suggestion_id": suggestion.suggestion_id,
                "character_internal_id": suggestion.character_internal_id,
                "asset_id": suggestion.asset_id,
                "tag_text": suggestion.tag_text,
                "confidence": suggestion.confidence,
                "model_receipt_ref": suggestion.model_receipt_ref,
                "tool_receipt_ref": suggestion.tool_receipt_ref,
                "suggested_by": suggestion.suggested_by,
                "status": suggestion.status.as_str(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(suggestion)
    }

    pub async fn list_ai_tag_suggestions_for_character(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<AiTagSuggestion>> {
        let rows = sqlx::query(
            r#"SELECT suggestion_id, character_internal_id, asset_id, tag_text,
                      confidence, model_receipt_ref, tool_receipt_ref,
                      suggested_by, status, decided_by, decision_reason,
                      applied_tag_id, created_at_utc, updated_at_utc
               FROM atelier_ai_tag_suggestion
               WHERE character_internal_id = $1
               ORDER BY created_at_utc ASC, suggestion_id ASC"#,
        )
        .bind(character_internal_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(ai_tag_suggestion_from_row).collect()
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
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_ai_tag_suggestion
               SET status = $2,
                   decided_by = $3,
                   decision_reason = $4,
                   updated_at_utc = NOW()
               WHERE suggestion_id = $1
                 AND status = 'proposed'
               RETURNING suggestion_id, character_internal_id, asset_id, tag_text,
                         confidence, model_receipt_ref, tool_receipt_ref,
                         suggested_by, status, decided_by, decision_reason,
                         applied_tag_id, created_at_utc, updated_at_utc"#,
        )
        .bind(decision.suggestion_id)
        .bind(next_status.as_str())
        .bind(decided_by)
        .bind(&reason)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            let current_status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM atelier_ai_tag_suggestion WHERE suggestion_id = $1",
            )
            .bind(decision.suggestion_id)
            .fetch_optional(&mut *tx)
            .await?;
            return match current_status {
                None => Err(AtelierError::NotFound(format!(
                    "ai tag suggestion_id={}",
                    decision.suggestion_id
                ))),
                Some(status) => Err(AtelierError::Validation(format!(
                    "AI tag suggestion {} is not proposed (status={status})",
                    decision.suggestion_id
                ))),
            };
        };
        let suggestion = ai_tag_suggestion_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family,
            "atelier_ai_tag_suggestion",
            &suggestion.suggestion_id.to_string(),
            serde_json::json!({
                "suggestion_id": suggestion.suggestion_id,
                "character_internal_id": suggestion.character_internal_id,
                "asset_id": suggestion.asset_id,
                "tag_text": suggestion.tag_text,
                "status": suggestion.status.as_str(),
                "decided_by": suggestion.decided_by,
                "decision_reason_ref": suggestion
                    .decision_reason
                    .as_ref()
                    .map(|reason| event_ref_for_text(reason)),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(suggestion)
    }

    pub async fn apply_ai_tag_suggestion(
        &self,
        suggestion_id: Uuid,
        applied_by: &str,
    ) -> AtelierResult<AiTagSuggestion> {
        let applied_by = require_ai_tag_actor("applied_by", applied_by)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"SELECT suggestion_id, character_internal_id, asset_id, tag_text,
                      confidence, model_receipt_ref, tool_receipt_ref,
                      suggested_by, status, decided_by, decision_reason,
                      applied_tag_id, created_at_utc, updated_at_utc
               FROM atelier_ai_tag_suggestion
               WHERE suggestion_id = $1
               FOR UPDATE"#,
        )
        .bind(suggestion_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("ai tag suggestion_id={suggestion_id}")))?;
        let current = ai_tag_suggestion_from_row(&row)?;
        match current.status {
            AiTagSuggestionStatus::Applied => {
                tx.commit().await?;
                return Ok(current);
            }
            AiTagSuggestionStatus::Accepted => {}
            status => {
                return Err(AtelierError::Validation(format!(
                    "AI tag suggestion {suggestion_id} must be accepted before apply (status={})",
                    status.as_str()
                )));
            }
        }

        let tag_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO atelier_tag (text)
               VALUES ($1)
               ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
               RETURNING tag_id"#,
        )
        .bind(&current.tag_text)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            r#"INSERT INTO atelier_character_tag
                 (character_internal_id, tag_id, tag_type)
               VALUES ($1, $2, 'manual')
               ON CONFLICT (character_internal_id, tag_id)
               DO UPDATE SET tag_type = 'manual'"#,
        )
        .bind(current.character_internal_id)
        .bind(tag_id)
        .execute(&mut *tx)
        .await?;
        let row = sqlx::query(
            r#"UPDATE atelier_ai_tag_suggestion
               SET status = 'applied',
                   decided_by = COALESCE(decided_by, $2),
                   applied_tag_id = $3,
                   updated_at_utc = NOW()
               WHERE suggestion_id = $1
               RETURNING suggestion_id, character_internal_id, asset_id, tag_text,
                         confidence, model_receipt_ref, tool_receipt_ref,
                         suggested_by, status, decided_by, decision_reason,
                         applied_tag_id, created_at_utc, updated_at_utc"#,
        )
        .bind(suggestion_id)
        .bind(applied_by)
        .bind(tag_id)
        .fetch_one(&mut *tx)
        .await?;
        let applied = ai_tag_suggestion_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            search_event_family::AI_TAG_SUGGESTION_APPLIED,
            "atelier_ai_tag_suggestion",
            &applied.suggestion_id.to_string(),
            serde_json::json!({
                "suggestion_id": applied.suggestion_id,
                "character_internal_id": applied.character_internal_id,
                "asset_id": applied.asset_id,
                "tag_id": tag_id,
                "tag_text": applied.tag_text,
                "status": applied.status.as_str(),
                "applied_by": applied_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(applied)
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
        let row = sqlx::query(
            r#"INSERT INTO atelier_tag_rule
                 (source_field_id, match_type, pattern, emit_tag, enabled)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING rule_id, source_field_id, match_type, pattern, emit_tag,
                         enabled, created_at_utc, updated_at_utc"#,
        )
        .bind(&new.source_field_id)
        .bind(new.match_type.as_str())
        .bind(&new.pattern)
        .bind(normalize_tag(&new.emit_tag))
        .bind(new.enabled)
        .fetch_one(self.pool())
        .await?;
        let rule = rule_from_row(&row)?;

        self.record_event(
            search_event_family::TAG_RULE_UPSERTED,
            "atelier_tag_rule",
            &rule.rule_id.to_string(),
            serde_json::json!({
                "rule_id": rule.rule_id,
                "source_field_id": rule.source_field_id,
                "match_type": rule.match_type.as_str(),
                "emit_tag": rule.emit_tag,
                "op": "create",
            }),
        )
        .await?;
        Ok(rule)
    }

    /// List saved tag rules in deterministic order (`rule_id ASC`), matching the
    /// legacy source `_upsertDerivedTags` ordering so derived tags are reproducible.
    pub async fn list_tag_rules(&self) -> AtelierResult<Vec<TagRule>> {
        let rows = sqlx::query(
            r#"SELECT rule_id, source_field_id, match_type, pattern, emit_tag,
                      enabled, created_at_utc, updated_at_utc
               FROM atelier_tag_rule
               ORDER BY rule_id ASC"#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(rule_from_row).collect()
    }

    /// Delete a saved tag rule. Emits `TAG_RULE_DELETED` when a row was removed.
    /// Mirrors legacy source `deleteTagRule`.
    pub async fn delete_tag_rule(&self, rule_id: Uuid) -> AtelierResult<bool> {
        let removed = sqlx::query("DELETE FROM atelier_tag_rule WHERE rule_id = $1")
            .bind(rule_id)
            .execute(self.pool())
            .await?;
        if removed.rows_affected() == 0 {
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

        let mut tx = self.pool().begin().await?;
        sqlx::query(
            r#"DELETE FROM atelier_character_tag
               WHERE character_internal_id = $1 AND tag_type = 'derived'"#,
        )
        .bind(character_internal_id)
        .execute(&mut *tx)
        .await?;

        for text in &emitted {
            let tag_id: Uuid = sqlx::query_scalar(
                r#"INSERT INTO atelier_tag (text)
                   VALUES ($1)
                   ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
                   RETURNING tag_id"#,
            )
            .bind(text)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                r#"INSERT INTO atelier_character_tag
                     (character_internal_id, tag_id, tag_type)
                   VALUES ($1, $2, 'derived')
                   ON CONFLICT (character_internal_id, tag_id)
                   DO UPDATE SET tag_type = 'derived'"#,
            )
            .bind(character_internal_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

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

        let row = sqlx::query(
            r#"INSERT INTO atelier_similarity_projection
                 (asset_internal_id, dhash_hex, palette_json)
               VALUES ($1, $2, $3)
               ON CONFLICT (asset_internal_id)
               DO UPDATE SET dhash_hex = EXCLUDED.dhash_hex,
                             palette_json = EXCLUDED.palette_json,
                             updated_at_utc = NOW()
               RETURNING asset_internal_id, dhash_hex, palette_json, updated_at_utc"#,
        )
        .bind(asset_internal_id)
        .bind(&normalized_hash)
        .bind(&palette)
        .fetch_one(self.pool())
        .await?;

        let projection = SimilarityProjection {
            asset_internal_id: row.get("asset_internal_id"),
            dhash_hex: row.get("dhash_hex"),
            palette_json: row.get("palette_json"),
            updated_at_utc: row.get("updated_at_utc"),
        };

        self.record_event(
            search_event_family::SIMILARITY_PROJECTED,
            "atelier_similarity_projection",
            &asset_internal_id.to_string(),
            serde_json::json!({
                "asset_internal_id": asset_internal_id,
                "has_dhash": projection.dhash_hex.is_some(),
            }),
        )
        .await?;
        Ok(projection)
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
        let row = sqlx::query(
            r#"INSERT INTO atelier_similarity_rebuild_job
                 (asset_internal_id, status, requested_by)
               VALUES ($1, 'running', $2)
               RETURNING job_id, asset_internal_id, status, requested_by,
                         processed_count, failed_count, dhash_hex, palette_json,
                         error_ref, created_at_utc, updated_at_utc"#,
        )
        .bind(asset_internal_id)
        .bind(requested_by)
        .fetch_one(self.pool())
        .await?;
        let running = similarity_rebuild_job_from_row(&row)?;

        let computed = compute_similarity_from_image_bytes(image_bytes);
        let (dhash_hex, palette_json) = match computed {
            Ok(value) => value,
            Err(err) => {
                let error_ref =
                    event_ref_for_text(&format!("similarity-rebuild:{}:{err}", running.job_id));
                let row = sqlx::query(
                    r#"UPDATE atelier_similarity_rebuild_job
                       SET status = 'failed',
                           processed_count = 0,
                           failed_count = 1,
                           error_ref = $2,
                           updated_at_utc = NOW()
                       WHERE job_id = $1
                       RETURNING job_id, asset_internal_id, status, requested_by,
                                 processed_count, failed_count, dhash_hex, palette_json,
                                 error_ref, created_at_utc, updated_at_utc"#,
                )
                .bind(running.job_id)
                .bind(&error_ref)
                .fetch_one(self.pool())
                .await?;
                let failed = similarity_rebuild_job_from_row(&row)?;
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
        let row = sqlx::query(
            r#"UPDATE atelier_similarity_rebuild_job
               SET status = 'completed',
                   processed_count = 1,
                   failed_count = 0,
                   dhash_hex = $2,
                   palette_json = $3,
                   error_ref = NULL,
                   updated_at_utc = NOW()
               WHERE job_id = $1
               RETURNING job_id, asset_internal_id, status, requested_by,
                         processed_count, failed_count, dhash_hex, palette_json,
                         error_ref, created_at_utc, updated_at_utc"#,
        )
        .bind(running.job_id)
        .bind(&dhash_hex)
        .bind(&projection.palette_json)
        .fetch_one(self.pool())
        .await?;
        let completed = similarity_rebuild_job_from_row(&row)?;
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
        let row = sqlx::query(
            r#"SELECT asset_internal_id, dhash_hex, palette_json, updated_at_utc
               FROM atelier_similarity_projection
               WHERE asset_internal_id = $1"#,
        )
        .bind(asset_internal_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(|row| SimilarityProjection {
            asset_internal_id: row.get("asset_internal_id"),
            dhash_hex: row.get("dhash_hex"),
            palette_json: row.get("palette_json"),
            updated_at_utc: row.get("updated_at_utc"),
        }))
    }

    /// Find media assets perceptually similar to `target_hash` within a Hamming
    /// `threshold` (0..=64), excluding the target asset, ordered nearest-first.
    /// Mirrors legacy source `image.similar.search`: candidate hashes are pulled from
    /// Postgres and scored with [`hamming_distance_hex64`] in-process (the dHash
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

        let rows = sqlx::query(
            r#"SELECT asset_internal_id, dhash_hex
               FROM atelier_similarity_projection
               WHERE dhash_hex IS NOT NULL
                 AND ($1::uuid IS NULL OR asset_internal_id <> $1)
               ORDER BY asset_internal_id ASC"#,
        )
        .bind(exclude_asset_internal_id)
        .fetch_all(self.pool())
        .await?;

        let mut hits: Vec<SimilarityHit> = Vec::new();
        for row in &rows {
            let asset_internal_id: Uuid = row.get("asset_internal_id");
            let dhash_hex: String = row.get("dhash_hex");
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

#[cfg(test)]
mod unit_tests {
    use super::*;

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
}
