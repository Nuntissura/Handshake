//! MT-202 UserManualContextBundleBridge: context bundles cite UserManual
//! pages with version and source anchors.
//!
//! Path (reuses the existing bundle vocabulary, no schema change):
//! * a manual page is mirrored as a `knowledge_entities` row of kind
//!   `user_manual_page` (natural key = slug; provenance carries the manual
//!   version + content hash), so bundles cite it as `ref_kind = entity`;
//! * the bundle item's citation string is the manual citation
//!   `usermanual:<slug>@<manual_version>#<anchor>@0-0@<hash8>` — version,
//!   source anchor, AND a content-hash prefix so a consumer can detect drift
//!   against the cited page (same drift law as span citations);
//! * `BundleTargetKind::UserManualPage` (already in the compiler) marks
//!   bundles compiled ABOUT a manual page.

use serde_json::json;

use super::store::{UserManualPage, UserManualSection};
use crate::knowledge_retrieval::budget::PriorityTier;
use crate::knowledge_retrieval::compiler::BundleCandidate;
use crate::knowledge_retrieval::snippet::EvidenceSnippet;
use crate::storage::knowledge::{
    KnowledgeBundleItemRefKind, KnowledgeEntity, KnowledgeEntityKind, KnowledgeStore,
    NewKnowledgeEntity,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

/// The citation base for a manual page: `usermanual:<slug>@<version>#<anchor>`.
/// The snippet machinery appends the span range and the content-hash prefix.
pub fn manual_citation_base(slug: &str, manual_version: &str, anchor: &str) -> String {
    format!("usermanual:{slug}@{manual_version}#{anchor}")
}

/// Mirror a manual page into the workspace's knowledge graph (idempotent on
/// (workspace, user_manual_page, slug)) so bundles can cite it as an entity.
pub async fn ensure_manual_page_entity(
    db: &PostgresDatabase,
    workspace_id: &str,
    page: &UserManualPage,
) -> StorageResult<KnowledgeEntity> {
    db.upsert_knowledge_entity(NewKnowledgeEntity {
        workspace_id: workspace_id.to_string(),
        entity_kind: KnowledgeEntityKind::UserManualPage,
        entity_key: page.slug.clone(),
        display_name: page.title.clone(),
        detection_provenance: json!({
            "detector": "user_manual::bundle_bridge",
            "manual_version": page.manual_version,
            "content_hash": page.content_hash,
            "page_kind": page.page_kind,
        }),
        primary_source_id: None,
        detected_in_run: None,
        evidence_span_ids: Vec::new(),
    })
    .await
}

/// Build a ranked bundle candidate citing a manual page section. The snippet
/// carries the page's content hash (drift detection) and a bounded excerpt.
pub fn manual_bundle_candidate(
    page: &UserManualPage,
    section: &UserManualSection,
    entity_id: &str,
    tier: PriorityTier,
    token_count: u32,
    relevance_score: f64,
) -> BundleCandidate {
    let anchor = format!("{}-{}", section.section_kind, section.position);
    let citation_base = manual_citation_base(&page.slug, &page.manual_version, &anchor);
    let excerpt: String = section.body_md.chars().take(280).collect();
    BundleCandidate {
        ref_kind: KnowledgeBundleItemRefKind::Entity,
        ref_id: entity_id.to_string(),
        tier,
        token_count,
        relevance_score,
        source_id: format!("usermanual:{}", page.slug),
        snippet: Some(EvidenceSnippet {
            span_id: format!("UMSPAN-{}-{}", page.slug, section.position),
            source_id: format!("usermanual:{}", page.slug),
            source_path: Some(citation_base),
            range_start: 0,
            range_end: 0,
            line_start: None,
            line_end: None,
            content_sha256: page.content_hash.clone(),
            excerpt: Some(excerpt),
            extraction_receipt_event_id: page.ledger_event_id.clone(),
            supported: true,
            unsupported_reason: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_base_carries_slug_version_and_anchor() {
        let citation = manual_citation_base("manual-toc", "2.0.0", "navigation-0");
        assert_eq!(citation, "usermanual:manual-toc@2.0.0#navigation-0");
    }
}
