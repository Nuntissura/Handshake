//! WP-KERNEL-009 MT-132 GraphTraversalPlanner.
//!
//! Spec 2.6.6.7.14.6 A0.3 ("graph_traversal SHOULD be preferred over hybrid_rag
//! when Loom or Knowledge Graph neighborhoods can satisfy the task within
//! budgets") + the folded WP-1-AIReady-RelationshipIds-GraphRetrieval-v1 intent
//! ("hybrid retrieval and graph traversal must produce non-empty graph
//! candidates for synthetic graph fixtures at minimum"; "retrieval traces can
//! cite relationship_id deterministically").
//!
//! This plans and executes a BOUNDED breadth-first graph traversal over the
//! committed KnowledgeEdge graph (`KnowledgeStore::list_knowledge_edges_for_entity`,
//! `entity_edge_degree`). Bounding controls, all actor-visible in the result:
//!   * `edge_type_allowlist` — only these edge types are followed,
//!   * `max_depth` — hop cap from the seed set,
//!   * `max_nodes` — total visited-node cap,
//!   * `hub_degree_suppression` — an entity whose degree exceeds this is a hub:
//!     it is recorded but NOT expanded (prevents a generic node exploding the
//!     neighborhood).
//!
//! Every followed edge contributes its stable `relationship_id` so a
//! RetrievalTrace can cite the exact graph path deterministically.

use std::collections::{BTreeSet, VecDeque};

use crate::storage::knowledge::{KnowledgeEdgeType, KnowledgeStore};
use crate::storage::knowledge_memory::entity_edge_degree;
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;
use sqlx::PgPool;

/// Bounding controls for a graph traversal. Defaults are conservative so a
/// traversal is always bounded even if a caller forgets to tune them.
///
/// The edge-type allowlist is held as the canonical edge-type strings
/// (`KnowledgeEdgeType::as_str`) rather than the foreign enum, so this struct
/// stays decoupled from the derive set on `KnowledgeEdgeType` (which is
/// read-only in `storage/knowledge.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphTraversalPolicy {
    /// Only these edge types are followed. Empty = follow any type.
    pub edge_type_allowlist: BTreeSet<String>,
    pub max_depth: u32,
    pub max_nodes: usize,
    /// An entity with degree strictly greater than this is treated as a hub and
    /// not expanded.
    pub hub_degree_suppression: i64,
}

impl Default for GraphTraversalPolicy {
    fn default() -> Self {
        Self {
            edge_type_allowlist: BTreeSet::new(),
            max_depth: 2,
            max_nodes: 64,
            hub_degree_suppression: 50,
        }
    }
}

impl GraphTraversalPolicy {
    /// Restrict to a set of edge types (builder).
    pub fn with_edge_types(mut self, types: impl IntoIterator<Item = KnowledgeEdgeType>) -> Self {
        self.edge_type_allowlist = types.into_iter().map(|t| t.as_str().to_string()).collect();
        self
    }

    fn allows(&self, edge_type: KnowledgeEdgeType) -> bool {
        self.edge_type_allowlist.is_empty() || self.edge_type_allowlist.contains(edge_type.as_str())
    }
}

/// One visited node in the traversal, with the hop distance and whether it was
/// suppressed as a hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisitedNode {
    pub entity_id: String,
    pub depth: u32,
    pub degree: i64,
    pub suppressed_as_hub: bool,
}

/// One followed edge, carrying the stable relationship_id for deterministic
/// citation (folded RelationshipIds intent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversedEdge {
    pub edge_id: String,
    pub relationship_id: String,
    pub edge_type: KnowledgeEdgeType,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub depth: u32,
}

/// The bounded result of a graph traversal, with actor-visible rationale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphTraversalResult {
    pub visited: Vec<VisitedNode>,
    pub edges: Vec<TraversedEdge>,
    /// Entities recorded but not expanded because they exceeded the hub degree.
    pub suppressed_hubs: Vec<String>,
    /// Human/actor-visible reason the traversal stopped (depth cap, node cap, or
    /// frontier exhausted) — the rationale a trace surfaces.
    pub stop_reason: String,
    pub seeds: Vec<String>,
}

impl GraphTraversalResult {
    /// Whether the traversal produced any graph candidate edges. The folded
    /// stub requires synthetic graph fixtures to yield non-empty candidates.
    pub fn has_candidates(&self) -> bool {
        !self.edges.is_empty()
    }

    /// The relationship_ids of every followed edge (deterministic citations).
    pub fn cited_relationship_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .edges
            .iter()
            .map(|e| e.relationship_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

/// The bounded graph-traversal planner/executor.
pub struct GraphTraversalPlanner<'a> {
    db: &'a PostgresDatabase,
    pool: &'a PgPool,
    policy: GraphTraversalPolicy,
}

impl<'a> GraphTraversalPlanner<'a> {
    pub fn new(db: &'a PostgresDatabase, pool: &'a PgPool, policy: GraphTraversalPolicy) -> Self {
        Self { db, pool, policy }
    }

    /// Run a bounded BFS from `seed_entity_ids`. Deterministic: the frontier is
    /// drained in sorted id order and edges are sorted by (relationship_id) so
    /// the same graph + seeds yields the same result.
    pub async fn traverse(
        &self,
        seed_entity_ids: &BTreeSet<String>,
    ) -> StorageResult<GraphTraversalResult> {
        let mut visited_ids: BTreeSet<String> = BTreeSet::new();
        let mut visited: Vec<VisitedNode> = Vec::new();
        let mut edges: Vec<TraversedEdge> = Vec::new();
        let mut suppressed_hubs: Vec<String> = Vec::new();
        let mut frontier: VecDeque<(String, u32)> = VecDeque::new();
        let mut stop_reason = "frontier exhausted within bounds".to_string();

        // Seed the frontier in sorted order for determinism.
        for seed in seed_entity_ids {
            frontier.push_back((seed.clone(), 0));
        }
        let seeds: Vec<String> = seed_entity_ids.iter().cloned().collect();

        while let Some((entity_id, depth)) = frontier.pop_front() {
            if visited_ids.contains(&entity_id) {
                continue;
            }
            if visited.len() >= self.policy.max_nodes {
                stop_reason = format!("max_nodes cap ({}) reached", self.policy.max_nodes);
                break;
            }

            let degree = entity_edge_degree(self.pool, &entity_id).await?;
            let is_hub = degree > self.policy.hub_degree_suppression;
            visited_ids.insert(entity_id.clone());
            visited.push(VisitedNode {
                entity_id: entity_id.clone(),
                depth,
                degree,
                suppressed_as_hub: is_hub,
            });

            if is_hub {
                suppressed_hubs.push(entity_id.clone());
                // Record the hub but do not expand it.
                continue;
            }
            if depth >= self.policy.max_depth {
                // Reached the depth cap for this branch; keep draining others.
                stop_reason = format!("max_depth cap ({}) reached", self.policy.max_depth);
                continue;
            }

            // Expand: follow allowlisted edges to new neighbors.
            let mut neighbor_edges = self.db.list_knowledge_edges_for_entity(&entity_id).await?;
            // Deterministic order.
            neighbor_edges.sort_by(|a, b| a.relationship_id.cmp(&b.relationship_id));
            for edge in neighbor_edges {
                if !self.policy.allows(edge.edge_type) {
                    continue;
                }
                let neighbor = if edge.source_entity_id == entity_id {
                    edge.target_entity_id.clone()
                } else {
                    edge.source_entity_id.clone()
                };
                edges.push(TraversedEdge {
                    edge_id: edge.edge_id.clone(),
                    relationship_id: edge.relationship_id.clone(),
                    edge_type: edge.edge_type,
                    source_entity_id: edge.source_entity_id.clone(),
                    target_entity_id: edge.target_entity_id.clone(),
                    depth: depth + 1,
                });
                if !visited_ids.contains(&neighbor) {
                    frontier.push_back((neighbor, depth + 1));
                }
            }
        }

        // Deterministic edge ordering for stable trace citation.
        edges.sort_by(|a, b| {
            a.relationship_id
                .cmp(&b.relationship_id)
                .then(a.edge_id.cmp(&b.edge_id))
        });
        edges.dedup_by(|a, b| a.edge_id == b.edge_id);

        Ok(GraphTraversalResult {
            visited,
            edges,
            suppressed_hubs,
            stop_reason,
            seeds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_allowlist_filters_edge_types() {
        let policy = GraphTraversalPolicy::default()
            .with_edge_types([KnowledgeEdgeType::References, KnowledgeEdgeType::DependsOn]);
        assert!(policy.allows(KnowledgeEdgeType::References));
        assert!(policy.allows(KnowledgeEdgeType::DependsOn));
        assert!(!policy.allows(KnowledgeEdgeType::Mentions));
    }

    #[test]
    fn empty_allowlist_allows_any_edge_type() {
        let policy = GraphTraversalPolicy::default();
        assert!(policy.allows(KnowledgeEdgeType::Mentions));
        assert!(policy.allows(KnowledgeEdgeType::LinksTo));
    }

    #[test]
    fn default_policy_is_bounded() {
        let policy = GraphTraversalPolicy::default();
        assert_eq!(policy.max_depth, 2);
        assert_eq!(policy.max_nodes, 64);
        assert!(policy.hub_degree_suppression > 0);
    }

    #[test]
    fn result_cites_unique_sorted_relationship_ids() {
        let result = GraphTraversalResult {
            visited: vec![],
            edges: vec![
                edge("e2", "REL-b"),
                edge("e1", "REL-a"),
                edge("e3", "REL-a"),
            ],
            suppressed_hubs: vec![],
            stop_reason: "x".to_string(),
            seeds: vec![],
        };
        assert_eq!(result.cited_relationship_ids(), vec!["REL-a", "REL-b"]);
        assert!(result.has_candidates());
    }

    fn edge(edge_id: &str, relationship_id: &str) -> TraversedEdge {
        TraversedEdge {
            edge_id: edge_id.to_string(),
            relationship_id: relationship_id.to_string(),
            edge_type: KnowledgeEdgeType::References,
            source_entity_id: "a".to_string(),
            target_entity_id: "b".to_string(),
            depth: 1,
        }
    }
}
