//! MT-124 BridgeEdgeGenerator (product orchestration).
//!
//! Generates bridging edges between fragmented subgraphs ONLY when (1) the two
//! endpoints co-occur in evidence (a shared detection span) and (2) they sit in
//! DIFFERENT connected components of the existing non-retired edge graph, and
//! (3) neither endpoint is a hub (degree below `hub_degree_threshold`). Every
//! considered pair — bridged or suppressed — is recorded in the bridge-decision
//! log so the "why did/didn't a bridge appear" is auditable.
//!
//! A produced bridge is a normal `knowledge_edges` row (`relates_to`, lifecycle
//! `proposed`, REQUIRED span evidence) created through
//! `KnowledgeStore::upsert_knowledge_edge` — it does NOT bypass the committed
//! edge substrate, so the deterministic relationship_id and the >=1-span trigger
//! hold. Bridges are born `proposed` (not `active`): a bridge is a SUGGESTION
//! that an operator/validator can accept, never an auto-accepted fact.
//!
//! Connectivity uses a union-find over the workspace's non-retired edges,
//! computed once per pass and bounded by the candidate `limit`. Hub suppression
//! prevents false shortcuts through high-degree entities (the translated-spec
//! rule "hub suppression is considered").

use std::collections::HashMap;

use sqlx::PgPool;

use crate::storage::knowledge::{
    KnowledgeEdgeLifecycle, KnowledgeEdgeType, KnowledgeStore, NewKnowledgeEdge,
};
use crate::storage::knowledge_memory::{
    entity_edge_degree, find_entity_cooccurrences, list_active_edge_endpoints,
    record_bridge_decision, BridgeDecision, BridgeDecisionRecord,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

pub use crate::storage::knowledge_memory::list_bridge_decisions;

/// A minimal union-find (disjoint-set) over entity ids, built from the
/// workspace's non-retired edges. Two entities are connected iff `find` returns
/// the same root.
struct UnionFind {
    parent: HashMap<String, String>,
}

impl UnionFind {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
        }
    }

    fn ensure(&mut self, id: &str) {
        if !self.parent.contains_key(id) {
            self.parent.insert(id.to_string(), id.to_string());
        }
    }

    fn find(&mut self, id: &str) -> String {
        self.ensure(id);
        // Iterative find with path compression.
        let mut root = id.to_string();
        while self.parent[&root] != root {
            root = self.parent[&root].clone();
        }
        let mut cur = id.to_string();
        while self.parent[&cur] != cur {
            let next = self.parent[&cur].clone();
            self.parent.insert(cur, root.clone());
            cur = next;
        }
        root
    }

    fn union(&mut self, a: &str, b: &str) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent.insert(ra, rb);
        }
    }

    fn connected(&mut self, a: &str, b: &str) -> bool {
        self.find(a) == self.find(b)
    }
}

/// Outcome of a bridge-generation pass: every decision recorded, and the edge
/// ids of the bridges that were actually created.
#[derive(Clone, Debug)]
pub struct BridgeGenerationResult {
    pub decisions: Vec<BridgeDecisionRecord>,
    pub bridged_edge_ids: Vec<String>,
}

/// Run a bridge-generation pass over a workspace.
///
/// For each co-occurring entity pair (bounded by `limit`):
/// * already in the same component  -> `suppressed_connected`
/// * either endpoint degree >= `hub_degree_threshold` -> `suppressed_hub`
/// * otherwise -> create a `proposed` `relates_to` bridge edge backed by the
///   shared span, union the two components, and record `bridged`.
///
/// `extractor_version` and `confidence` are stamped on the produced edges.
pub async fn generate_bridge_edges(
    db: &PostgresDatabase,
    pool: &PgPool,
    workspace_id: &str,
    hub_degree_threshold: i32,
    confidence: f64,
    extractor_version: &str,
    limit: i64,
) -> StorageResult<BridgeGenerationResult> {
    // Build the connectivity structure from existing non-retired edges.
    let mut uf = UnionFind::new();
    for (source, target) in list_active_edge_endpoints(pool, workspace_id).await? {
        uf.union(&source, &target);
    }

    let cooccurrences = find_entity_cooccurrences(pool, workspace_id, limit).await?;

    let mut decisions = Vec::new();
    let mut bridged_edge_ids = Vec::new();
    for pair in &cooccurrences {
        let degree_a = entity_edge_degree(pool, &pair.entity_id_a).await? as i32;
        let degree_b = entity_edge_degree(pool, &pair.entity_id_b).await? as i32;

        // Already connected: no bridge needed (not a fragmented subgraph).
        if uf.connected(&pair.entity_id_a, &pair.entity_id_b) {
            decisions.push(
                record_bridge_decision(
                    pool,
                    workspace_id,
                    &pair.entity_id_a,
                    &pair.entity_id_b,
                    BridgeDecision::SuppressedConnected,
                    degree_a,
                    degree_b,
                    hub_degree_threshold,
                    Some(&pair.shared_span_id),
                    None,
                )
                .await?,
            );
            continue;
        }

        // Hub suppression: do not bridge through a high-degree entity.
        if degree_a >= hub_degree_threshold || degree_b >= hub_degree_threshold {
            decisions.push(
                record_bridge_decision(
                    pool,
                    workspace_id,
                    &pair.entity_id_a,
                    &pair.entity_id_b,
                    BridgeDecision::SuppressedHub,
                    degree_a,
                    degree_b,
                    hub_degree_threshold,
                    Some(&pair.shared_span_id),
                    None,
                )
                .await?,
            );
            continue;
        }

        // Accepted: create a proposed bridge edge backed by the shared span.
        let edge = db
            .upsert_knowledge_edge(NewKnowledgeEdge {
                workspace_id: workspace_id.to_string(),
                edge_type: KnowledgeEdgeType::RelatesTo,
                source_entity_id: pair.entity_id_a.clone(),
                target_entity_id: pair.entity_id_b.clone(),
                extractor_version: extractor_version.to_string(),
                confidence,
                detected_in_run: None,
                evidence_span_ids: vec![pair.shared_span_id.clone()],
            })
            .await?;
        // A bridge is a SUGGESTION: mark it proposed (upsert defaults to active).
        let edge = db
            .set_knowledge_edge_lifecycle(&edge.edge_id, KnowledgeEdgeLifecycle::Proposed, None)
            .await?;

        uf.union(&pair.entity_id_a, &pair.entity_id_b);
        bridged_edge_ids.push(edge.edge_id.clone());
        decisions.push(
            record_bridge_decision(
                pool,
                workspace_id,
                &pair.entity_id_a,
                &pair.entity_id_b,
                BridgeDecision::Bridged,
                degree_a,
                degree_b,
                hub_degree_threshold,
                Some(&pair.shared_span_id),
                Some(&edge.edge_id),
            )
            .await?,
        );
    }

    Ok(BridgeGenerationResult {
        decisions,
        bridged_edge_ids,
    })
}
