//! MT-204 UserManualFreshnessCheck: typed verdicts that the stored manual is
//! current with the compiled-in corpus and the declared surface registry.
//!
//! Three-way comparison:
//! * seed corpus (compiled into THIS binary — what the manual should say),
//! * `user_manual_*` PostgreSQL rows (what the manual DOES say),
//! * [`registry::wp009_surface_registry`] (what the product DOES serve;
//!   runtime-probed by the doc-vs-runtime tests).
//!
//! Verdict vocabulary (stable, machine-consumable):
//! * `current`            — page row matches the compiled-in content hash.
//! * `stale_content`      — page row exists but its hash differs from the
//!                          seed (changed seed without resync, or tampering).
//! * `missing_page`       — the seed expects a page the DB does not hold.
//! * `uncovered_surface`  — a registry surface has NO `http_route` anchor on
//!                          any page (spec 10.15.8 build-rule defect).
//! * `dangling_anchor`    — an `http_route` anchor documents a surface the
//!                          registry does not declare, or a `page_link`
//!                          anchor targets a missing page (stale docs).
//! * `unseeded_version`   — no `user_manual_versions` row for this binary's
//!                          corpus version/hash.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::registry::wp009_surface_registry;
use super::seed::{corpus_hash, seed_corpus};
use super::store::UserManualStore;
use super::USER_MANUAL_VERSION;
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessVerdictKind {
    Current,
    StaleContent,
    MissingPage,
    UncoveredSurface,
    DanglingAnchor,
    UnseededVersion,
}

impl FreshnessVerdictKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::StaleContent => "stale_content",
            Self::MissingPage => "missing_page",
            Self::UncoveredSurface => "uncovered_surface",
            Self::DanglingAnchor => "dangling_anchor",
            Self::UnseededVersion => "unseeded_version",
        }
    }

    pub fn is_problem(self) -> bool {
        !matches!(self, Self::Current)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreshnessVerdict {
    pub kind: FreshnessVerdictKind,
    /// What the verdict is about (slug, route, anchor value, version).
    pub subject: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreshnessReport {
    pub manual_version: String,
    pub seed_content_hash: String,
    pub fresh: bool,
    pub current_count: usize,
    pub problem_count: usize,
    pub verdicts: Vec<FreshnessVerdict>,
}

/// Run the full freshness check against the live database.
pub async fn check_freshness(db: &PostgresDatabase) -> StorageResult<FreshnessReport> {
    let store = UserManualStore::new(db);
    let corpus = seed_corpus();
    let seed_hash = corpus_hash(&corpus);
    let mut verdicts = Vec::new();

    // 1) Version metadata.
    match store.get_version(USER_MANUAL_VERSION).await? {
        Some(row) if row.seed_content_hash == seed_hash => {}
        Some(row) => verdicts.push(FreshnessVerdict {
            kind: FreshnessVerdictKind::UnseededVersion,
            subject: USER_MANUAL_VERSION.to_string(),
            detail: format!(
                "stored seed hash {} != compiled-in corpus hash {} — run POST /usermanual/resync",
                row.seed_content_hash, seed_hash
            ),
        }),
        None => verdicts.push(FreshnessVerdict {
            kind: FreshnessVerdictKind::UnseededVersion,
            subject: USER_MANUAL_VERSION.to_string(),
            detail: "no user_manual_versions row for this binary's corpus — run POST /usermanual/resync"
                .to_string(),
        }),
    }

    // 2) Page-level content freshness.
    let stored_pages = store.list_pages(None, None, super::store::LIST_CAP).await?;
    let stored_by_slug: BTreeMap<&str, &super::store::UserManualPage> =
        stored_pages.iter().map(|p| (p.slug.as_str(), p)).collect();
    let seed_slugs: BTreeSet<&str> = corpus.pages.iter().map(|p| p.slug.as_str()).collect();
    for page in &corpus.pages {
        match stored_by_slug.get(page.slug.as_str()) {
            None => verdicts.push(FreshnessVerdict {
                kind: FreshnessVerdictKind::MissingPage,
                subject: page.slug.clone(),
                detail: "seed expects this page; the database does not hold it".to_string(),
            }),
            Some(stored) if stored.content_hash != page.content_hash() => {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleContent,
                    subject: page.slug.clone(),
                    detail: format!(
                        "stored hash {} != seed hash {}",
                        stored.content_hash,
                        page.content_hash()
                    ),
                })
            }
            Some(_) => verdicts.push(FreshnessVerdict {
                kind: FreshnessVerdictKind::Current,
                subject: page.slug.clone(),
                detail: String::new(),
            }),
        }
    }

    // 3) Registry coverage (uncovered surfaces).
    let route_anchors = store.anchors_by_kind("http_route").await?;
    let covered: BTreeSet<(String, String)> = route_anchors
        .iter()
        .map(|a| (a.http_method.clone(), a.anchor_value.clone()))
        .collect();
    for surface in wp009_surface_registry() {
        if !covered.contains(&(surface.method.to_string(), surface.route.to_string())) {
            verdicts.push(FreshnessVerdict {
                kind: FreshnessVerdictKind::UncoveredSurface,
                subject: format!("{} {}", surface.method, surface.route),
                detail: format!(
                    "registry surface {} has no http_route anchor on any UserManual page",
                    surface.surface_id
                ),
            });
        }
    }

    // 4) Dangling http_route anchors (documented but not declared).
    let declared: BTreeSet<(String, String)> = wp009_surface_registry()
        .iter()
        .map(|s| (s.method.to_string(), s.route.to_string()))
        .collect();
    for anchor in &route_anchors {
        if !declared.contains(&(anchor.http_method.clone(), anchor.anchor_value.clone())) {
            verdicts.push(FreshnessVerdict {
                kind: FreshnessVerdictKind::DanglingAnchor,
                subject: format!("{} {}", anchor.http_method, anchor.anchor_value),
                detail: "http_route anchor documents a surface the WP-009 registry does not declare"
                    .to_string(),
            });
        }
    }

    // 5) Dangling page links (stored rows referencing missing pages).
    let stored_slugs: BTreeSet<&str> = stored_pages.iter().map(|p| p.slug.as_str()).collect();
    for anchor in store.anchors_by_kind("page_link").await? {
        let known = stored_slugs.contains(anchor.anchor_value.as_str())
            || seed_slugs.contains(anchor.anchor_value.as_str());
        if !known {
            verdicts.push(FreshnessVerdict {
                kind: FreshnessVerdictKind::DanglingAnchor,
                subject: anchor.anchor_value.clone(),
                detail: "page_link anchor targets a page that exists neither in the database nor in the seed"
                    .to_string(),
            });
        }
    }

    let current_count = verdicts
        .iter()
        .filter(|v| v.kind == FreshnessVerdictKind::Current)
        .count();
    let problem_count = verdicts.iter().filter(|v| v.kind.is_problem()).count();
    Ok(FreshnessReport {
        manual_version: USER_MANUAL_VERSION.to_string(),
        seed_content_hash: seed_hash,
        fresh: problem_count == 0,
        current_count,
        problem_count,
        verdicts,
    })
}
