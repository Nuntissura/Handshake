//! Core search / tags / similarity (WP-KERNEL-005, MT-005 event coverage).
//!
//! CKC source fold-in (translate behavior, NOT SQLite storage):
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
//! Design notes mirrored from CKC:
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
use uuid::Uuid;

use super::{AtelierError, AtelierResult, AtelierStore};

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

    /// All search/tags/similarity event families (parity / coverage checks).
    pub const ALL: &[&str] = &[
        CHARACTER_TAGGED,
        CHARACTER_UNTAGGED,
        TAG_RULE_UPSERTED,
        TAG_RULE_DELETED,
        DERIVED_TAGS_RECOMPUTED,
        SIMILARITY_PROJECTED,
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

/// How a tag rule matches a source field value (CKC `match_type`).
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

/// A similarity projection (perceptual hash + dominant palette) for an asset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimilarityProjection {
    pub asset_internal_id: Uuid,
    pub dhash_hex: Option<String>,
    pub palette_json: serde_json::Value,
    pub updated_at_utc: DateTime<Utc>,
}

/// A nearest-neighbour similarity hit (CKC `image.similar.search`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimilarityHit {
    pub asset_internal_id: Uuid,
    pub dhash_hex: String,
    pub distance: i32,
}

/// dHash hex must be exactly 16 lowercase hex chars (64 bits). Mirrors CKC
/// `dhash.js::isHex64`.
fn is_hex64(s: &str) -> bool {
    let t = s.trim();
    t.len() == 16 && t.chars().all(|c| c.is_ascii_hexdigit())
}

/// Normalize tag text: trim + lowercase so the dictionary dedupes case- and
/// whitespace-insensitively, matching CKC tag handling intent.
fn normalize_tag(text: &str) -> String {
    text.trim().to_ascii_lowercase()
}

/// Hamming distance between two 16-char hex (64-bit) hashes. Mirrors CKC
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

fn tag_from_row(row: &sqlx::postgres::PgRow) -> Tag {
    Tag {
        tag_id: row.get("tag_id"),
        text: row.get("text"),
        created_at_utc: row.get("created_at_utc"),
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

impl AtelierStore {
    // ----- Tag dictionary -------------------------------------------------

    /// Ensure a tag exists in the dictionary (deduped by normalized text) and
    /// return it. Idempotent: re-ensuring identical text returns the same row.
    /// Mirrors CKC `_ensureTag`.
    pub async fn ensure_tag(&self, text: &str) -> AtelierResult<Tag> {
        let norm = normalize_tag(text);
        if norm.is_empty() {
            return Err(AtelierError::Validation("tag text must not be empty".into()));
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
    /// "all tags" picker in CKC `listAllTags`.
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
    /// the (character, tag) pair; emits `CHARACTER_TAGGED`. Mirrors CKC
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
            &character_internal_id.to_string(),
            serde_json::json!({
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
    /// single `CHARACTER_TAGGED` bulk event is recorded. Mirrors the CKC
    /// `batchUpdateCharacterTags` operator workflow.
    pub async fn bulk_tag_characters(
        &self,
        character_internal_ids: &[Uuid],
        texts: &[String],
    ) -> AtelierResult<i64> {
        if character_internal_ids.is_empty() || texts.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool().begin().await?;
        let mut tag_ids: Vec<Uuid> = Vec::with_capacity(texts.len());
        for text in texts {
            let norm = normalize_tag(text);
            if norm.is_empty() {
                continue;
            }
            let tag_id: Uuid = sqlx::query_scalar(
                r#"INSERT INTO atelier_tag (text)
                   VALUES ($1)
                   ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
                   RETURNING tag_id"#,
            )
            .bind(&norm)
            .fetch_one(&mut *tx)
            .await?;
            tag_ids.push(tag_id);
        }

        let mut written: i64 = 0;
        for character_internal_id in character_internal_ids {
            for tag_id in &tag_ids {
                let res = sqlx::query(
                    r#"INSERT INTO atelier_character_tag
                         (character_internal_id, tag_id, tag_type)
                       VALUES ($1, $2, 'manual')
                       ON CONFLICT (character_internal_id, tag_id)
                       DO UPDATE SET tag_type = EXCLUDED.tag_type"#,
                )
                .bind(character_internal_id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await?;
                written += res.rows_affected() as i64;
            }
        }
        tx.commit().await?;

        self.record_event(
            search_event_family::CHARACTER_TAGGED,
            "atelier_character_tag",
            "bulk",
            serde_json::json!({
                "character_count": character_internal_ids.len(),
                "tag_count": tag_ids.len(),
                "links_written": written,
                "mode": "bulk_manual",
            }),
        )
        .await?;

        Ok(written)
    }

    /// Detach a manual tag from a character. No-op if the tag/link does not
    /// exist. Emits `CHARACTER_UNTAGGED` when a link was removed. Mirrors CKC
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
            &character_internal_id.to_string(),
            serde_json::json!({ "text": norm, "tag_type": "manual" }),
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

    // ----- Saved tag rules -----------------------------------------------

    /// Create a saved tag rule. Emits `TAG_RULE_UPSERTED`. Mirrors CKC
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
    /// CKC `_upsertDerivedTags` ordering so derived tags are reproducible.
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
    /// Mirrors CKC `deleteTagRule`.
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
    /// rule output is re-inserted, exactly like CKC `_upsertDerivedTags`. The
    /// regex match type uses the `regex` crate; invalid patterns are ignored
    /// deterministically (mirroring the CKC try/catch). Returns the sorted list
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
            &character_internal_id.to_string(),
            serde_json::json!({
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
    /// 64-bit hex (CKC `isHex64`) when present. Emits `SIMILARITY_PROJECTED`.
    /// Mirrors CKC persistence of `dhash_hex` / `palette_json` on the asset.
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
    /// Mirrors CKC `image.similar.search`: candidate hashes are pulled from
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
    fn hamming_matches_ckc_semantics() {
        // Identical hashes => 0.
        assert_eq!(hamming_distance_hex64("0000000000000000", "0000000000000000"), 0);
        // Single bit set => 1.
        assert_eq!(hamming_distance_hex64("0000000000000000", "0000000000000001"), 1);
        // All bits differ => 64.
        assert_eq!(hamming_distance_hex64("0000000000000000", "ffffffffffffffff"), 64);
        // Invalid input => max distance 64.
        assert_eq!(hamming_distance_hex64("zzzz", "0000000000000000"), 64);
    }

    #[test]
    fn tag_normalization() {
        assert_eq!(normalize_tag("  BlondE "), "blonde");
        assert_eq!(normalize_tag("Red Hair"), "red hair");
    }
}
