//! MT-202 UserManualContextBundleBridge: context bundles cite UserManual
//! pages with version and source anchors.
//!
//! Path (reuses the existing bundle vocabulary and ProjectKnowledgeIndex
//! entity table):
//! * a manual page is mirrored as a `knowledge_entities` row of kind
//!   `user_manual_page` under an exact `ResourceScope` (identity = exact
//!   scope + kind + slug; provenance carries the manual version + content
//!   hash), so bundles cite it as `ref_kind = entity`;
//! * the entity mutation and its scoped EventLedger receipt commit atomically;
//!   identical retries return the same durable evidence;
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
use crate::storage::knowledge::KnowledgeBundleItemRefKind;
use crate::storage::surreal::{SurrealUserManualKnowledgeStore, UserManualKnowledgeEntityMutation};
use crate::storage::StorageResult;
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;

/// The citation base for a manual page: `usermanual:<slug>@<version>#<anchor>`.
/// The snippet machinery appends the span range and the content-hash prefix.
pub fn manual_citation_base(slug: &str, manual_version: &str, anchor: &str) -> String {
    format!("usermanual:{slug}@{manual_version}#{anchor}")
}

/// Mirror a manual page into the exact-scoped workspace knowledge graph so
/// bundles can cite it as an entity. The provider is idempotent on the full
/// scope plus `(user_manual_page, slug)` and returns stable receipt evidence.
pub async fn ensure_manual_page_entity(
    store: &SurrealUserManualKnowledgeStore,
    scope: &ExactResourceScopeAttribution,
    page: &UserManualPage,
) -> StorageResult<UserManualKnowledgeEntityMutation> {
    store
        .upsert_user_manual_page_entity(
            scope,
            &page.slug,
            &page.title,
            manual_entity_provenance(page),
        )
        .await
}

#[cfg(feature = "test-utils")]
pub async fn insert_manual_page_orphan_receipt_fixture(
    store: &SurrealUserManualKnowledgeStore,
    scope: &ExactResourceScopeAttribution,
    page: &UserManualPage,
) -> StorageResult<String> {
    store
        .insert_orphan_receipt_fixture(
            scope,
            &page.slug,
            &page.title,
            manual_entity_provenance(page),
        )
        .await
}

fn manual_entity_provenance(page: &UserManualPage) -> serde_json::Value {
    json!({
        "detector": "user_manual::bundle_bridge",
        "manual_version": page.manual_version,
        "content_hash": page.content_hash,
        "page_kind": page.page_kind,
    })
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
