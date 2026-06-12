//! WP-KERNEL-009 MT-140 SemanticCatalogBridge.
//!
//! Folded WP-1-Semantic-Catalog-v2 intent: "Semantic Catalog remains a
//! deterministic backend routing contract for tools, data surfaces, retrieval,
//! and Spec Router planning"; "entries may not remain hidden in prompts, helper
//! labels, or UI-only descriptions"; "Project Knowledge Index can index, query,
//! and cite catalog entries as backend contracts." Spec 2.6.6.7.14.6 A: a
//! `QueryPlan.route[]` MUST be derived from the SemanticCatalog when present.
//!
//! This bridge is the product logic that turns a backend catalog entry
//! (`storage/knowledge_retrieval.rs` table 0260) into concrete plan
//! [`RouteStep`]s. The catalog is authoritative and queryable — the bridge reads
//! it, it does not invent routing from prompt text.

use sqlx::PgPool;

use crate::knowledge_retrieval::plan::{RetrievalStore, RouteStep};
use crate::storage::knowledge_retrieval::{resolve_semantic_catalog_entry, SemanticCatalogEntry};
use crate::storage::StorageResult;

/// Map a catalog route string (spec route vocabulary) to a plan
/// [`RetrievalStore`]. The catalog's `sql_query` / `bounded_read` routes map to
/// the bounded-read-only store (authoritative direct reads).
pub fn store_for_route(route: &str) -> Option<RetrievalStore> {
    match route {
        "knowledge_graph" => Some(RetrievalStore::KnowledgeGraph),
        "shadow_ws_lexical" => Some(RetrievalStore::ShadowWsLexical),
        "shadow_ws_vector" => Some(RetrievalStore::ShadowWsVector),
        "bounded_read" | "sql_query" => Some(RetrievalStore::BoundedReadOnly),
        _ => None,
    }
}

/// Build the route steps a catalog entry prescribes. Each declared route becomes
/// an ordered [`RouteStep`] whose purpose cites the catalog entry, so the plan
/// is explainable back to its routing contract. Unknown routes are skipped (the
/// catalog can name routes a future store implements).
pub fn route_steps_from_entry(entry: &SemanticCatalogEntry, max_candidates: u32) -> Vec<RouteStep> {
    entry
        .query_routes
        .iter()
        .filter_map(|route| {
            store_for_route(route).map(|store| {
                RouteStep::new(
                    store,
                    format!(
                        "semantic catalog '{}' (v{}) route '{}'",
                        entry.name, entry.version, route
                    ),
                    max_candidates,
                )
            })
        })
        .collect()
}

/// The result of a catalog lookup for planning.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogRouting {
    pub entry_name: String,
    pub route: Vec<RouteStep>,
    pub supported_selectors: Vec<String>,
}

/// Resolve a named catalog entry and produce its planned route. Returns `None`
/// when the catalog has no active contract for the name — the planner then falls
/// back to its default routing (spec 2.6.6.7.14.6 C default policy).
pub async fn routing_for(
    pool: &PgPool,
    workspace_id: &str,
    name: &str,
    max_candidates: u32,
) -> StorageResult<Option<CatalogRouting>> {
    let Some(entry) = resolve_semantic_catalog_entry(pool, workspace_id, name).await? else {
        return Ok(None);
    };
    let route = route_steps_from_entry(&entry, max_candidates);
    Ok(Some(CatalogRouting {
        entry_name: entry.name.clone(),
        route,
        supported_selectors: entry.supported_selectors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::knowledge_retrieval::SemanticCatalogKind;
    use serde_json::json;

    fn entry(routes: &[&str]) -> SemanticCatalogEntry {
        SemanticCatalogEntry {
            entry_id: "KSC-1".to_string(),
            workspace_id: "ws".to_string(),
            entry_kind: SemanticCatalogKind::Index,
            name: "code_symbols".to_string(),
            version: 1,
            description: "symbol index".to_string(),
            query_routes: routes.iter().map(ToString::to_string).collect(),
            supported_selectors: vec!["symbol".to_string()],
            default_budgets: None,
            examples: json!([]),
            lifecycle_state: "active".to_string(),
        }
    }

    #[test]
    fn maps_known_routes_to_stores() {
        assert_eq!(
            store_for_route("knowledge_graph"),
            Some(RetrievalStore::KnowledgeGraph)
        );
        assert_eq!(
            store_for_route("shadow_ws_vector"),
            Some(RetrievalStore::ShadowWsVector)
        );
        assert_eq!(
            store_for_route("sql_query"),
            Some(RetrievalStore::BoundedReadOnly)
        );
        assert_eq!(store_for_route("nonsense"), None);
    }

    #[test]
    fn route_steps_cite_the_catalog_entry() {
        let steps = route_steps_from_entry(&entry(&["knowledge_graph", "shadow_ws_lexical"]), 32);
        assert_eq!(steps.len(), 2);
        assert!(steps[0].purpose.contains("code_symbols"));
        assert_eq!(steps[0].store, RetrievalStore::KnowledgeGraph);
        assert_eq!(steps[1].store, RetrievalStore::ShadowWsLexical);
    }

    #[test]
    fn unknown_routes_are_skipped_not_errored() {
        let steps = route_steps_from_entry(&entry(&["knowledge_graph", "future_store"]), 16);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].store, RetrievalStore::KnowledgeGraph);
    }
}
