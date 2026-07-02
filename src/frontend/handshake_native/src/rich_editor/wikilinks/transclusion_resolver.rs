//! Cycle-aware transclusion-chain resolver (WP-KERNEL-012 MT-045, wave-2 remediation).
//!
//! This is the NATIVE cycle-detection contribution the React reference lacks: a pure walk over a
//! "fetch one hop" function that tracks visited block ids in a `HashSet<String>` and returns a typed
//! [`TransclusionResolveError::CycleDetected`] the moment an id repeats — it NEVER loops forever and
//! NEVER panics. `max_depth` bounds non-cyclic but pathologically long chains.
//!
//! ## Product placement (the wave-2 remediation)
//!
//! The resolver originally lived ONLY inside `tests/test_perf_large_rich.rs` (the LR-05 perf proof),
//! so no production render path was cycle-guarded. It now lives HERE, in product code:
//! - [`crate::rich_editor::wikilinks::runtime::WikilinkRuntime::detect_transclusion_cycle`] walks the
//!   live transclusion cache with it, and
//! - [`crate::rich_editor::wikilinks::transclusion_view::render_transclusion`] renders a VISIBLE
//!   cycle indicator (suppressing the read-through preview) when the chain starting at the rendered
//!   node is cyclic — instead of resolving/painting a cyclic chain unguarded.
//! The LR-05 perf tests (`tests/test_perf_large_rich.rs`) now import THIS product symbol, so the
//! perf proof and the product guard are one algorithm, not a test-only fork.

use std::collections::HashSet;

/// The bound a render-path chain walk uses (well above any sane transclusion nesting; the LIVE LR-05
/// perf proof drives 50 hops, so 64 leaves headroom without allowing a runaway walk).
pub const MAX_TRANSCLUSION_CHAIN_DEPTH: usize = 64;

/// A typed resolution failure. `CycleDetected` carries the block id at which a previously-visited id
/// repeated — so a caller (and a reviewer) can confirm the resolver flags a CYCLE specifically, not
/// any transclusion (the MT-045 RISK-4 guard). `DepthExceeded` bounds runaway chains even without a
/// cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransclusionResolveError {
    /// A previously-visited block id repeated at `at` — the chain is cyclic.
    CycleDetected { at: String },
    /// The chain exceeded `max_depth` hops without terminating (no repeat seen yet).
    DepthExceeded { max_depth: usize },
}

impl std::fmt::Display for TransclusionResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The token "cycle_detected" is what the LR-05 proof_target #4 greps for.
            TransclusionResolveError::CycleDetected { at } => {
                write!(f, "cycle_detected at block {at}")
            }
            TransclusionResolveError::DepthExceeded { max_depth } => {
                write!(f, "depth_exceeded (max_depth={max_depth})")
            }
        }
    }
}

/// Resolve a transclusion chain starting at `start`, following each block's single transclusion
/// target via `fetch_hop` (`block_id -> Some(next_block_id)` to continue, `None` to terminate
/// cleanly). Tracks visited ids in a `HashSet` (MT-045 impl note 5): the moment `fetch_hop` returns
/// an id already in the set the resolver returns `Err(CycleDetected { at })` — it NEVER loops forever
/// and NEVER panics. `max_depth` bounds non-cyclic but pathologically long chains. Returns the
/// ordered list of visited ids on success.
pub fn resolve_transclusion_chain(
    start: &str,
    max_depth: usize,
    mut fetch_hop: impl FnMut(&str) -> Option<String>,
) -> Result<Vec<String>, TransclusionResolveError> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut order: Vec<String> = Vec::new();
    let mut current = start.to_owned();
    loop {
        if !visited.insert(current.clone()) {
            // `current` was already visited — a cycle. Report the repeated id explicitly.
            return Err(TransclusionResolveError::CycleDetected { at: current });
        }
        order.push(current.clone());
        if order.len() > max_depth {
            return Err(TransclusionResolveError::DepthExceeded { max_depth });
        }
        match fetch_hop(&current) {
            Some(next) => current = next,
            None => return Ok(order), // clean chain end
        }
    }
}

/// The FIRST `loomTransclusion` target (`attrs.refValue`) embedded anywhere inside a resolved
/// transclusion `content_json` (a Tiptap doc value), in document order — the "next hop" of a
/// render-path transclusion chain. `None` when the content embeds no transclusion (the chain ends
/// cleanly there). Pure + unit-testable; the shape is the verified backend `loomTransclusion` node
/// (`doc_json.rs` round-trip: `{type:"loomTransclusion", attrs:{refValue}}`).
pub fn next_transclusion_ref(content_json: &serde_json::Value) -> Option<String> {
    if content_json.get("type").and_then(|t| t.as_str()) == Some("loomTransclusion") {
        if let Some(ref_value) = content_json
            .get("attrs")
            .and_then(|a| a.get("refValue"))
            .and_then(|v| v.as_str())
        {
            return Some(ref_value.to_owned());
        }
    }
    if let Some(content) = content_json.get("content").and_then(|c| c.as_array()) {
        for child in content {
            if let Some(found) = next_transclusion_ref(child) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_chain_resolves_without_false_cycle() {
        // A deterministic LINEAR chain of 50: block-0 -> block-1 -> ... -> block-49 -> (end).
        let chain: std::collections::HashMap<String, String> = (0..49)
            .map(|i| (format!("block-{i}"), format!("block-{}", i + 1)))
            .collect();
        let order = resolve_transclusion_chain("block-0", 100, |id| chain.get(id).cloned())
            .expect("a linear 50-chain must resolve, not report a false cycle");
        assert_eq!(order.len(), 50);
        assert_eq!(order.first().map(String::as_str), Some("block-0"));
        assert_eq!(order.last().map(String::as_str), Some("block-49"));
    }

    #[test]
    fn cycle_is_detected_at_the_repeated_id() {
        // A 5-block CYCLE: block-0 -> ... -> block-4 -> block-0. The resolver MUST return
        // Err(CycleDetected) at the FIRST repeated id (block-0), NOT panic / loop forever, and NOT
        // report DepthExceeded (the 100 bound is far above the 5-cycle — it must catch the cycle
        // first).
        let cycle: std::collections::HashMap<String, String> = (0..5)
            .map(|i| (format!("block-{i}"), format!("block-{}", (i + 1) % 5)))
            .collect();
        match resolve_transclusion_chain("block-0", 100, |id| cycle.get(id).cloned()) {
            Err(TransclusionResolveError::CycleDetected { at }) => assert_eq!(at, "block-0"),
            other => panic!("a 5-cycle must be CycleDetected, got {other:?}"),
        }
    }

    #[test]
    fn depth_bound_stops_a_runaway_chain() {
        // An unbounded generator (every id yields a fresh next) trips the depth bound, never spins.
        let mut n = 0usize;
        let result = resolve_transclusion_chain("head", 10, |_| {
            n += 1;
            Some(format!("gen-{n}"))
        });
        assert_eq!(
            result,
            Err(TransclusionResolveError::DepthExceeded { max_depth: 10 })
        );
    }

    #[test]
    fn display_carries_the_cycle_detected_token() {
        let e = TransclusionResolveError::CycleDetected { at: "BLK-1".into() };
        assert!(e.to_string().contains("cycle_detected"));
        let d = TransclusionResolveError::DepthExceeded { max_depth: 3 };
        assert!(d.to_string().contains("depth_exceeded"));
    }

    #[test]
    fn next_transclusion_ref_finds_the_first_embedded_target() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                {"type":"paragraph","content":[{"type":"text","text":"before"}]},
                {"type":"paragraph","content":[
                    {"type":"loomTransclusion","attrs":{"refValue":"BLK-NEXT"}},
                    {"type":"loomTransclusion","attrs":{"refValue":"BLK-LATER"}}
                ]}
            ]
        });
        assert_eq!(next_transclusion_ref(&doc).as_deref(), Some("BLK-NEXT"));
    }

    #[test]
    fn next_transclusion_ref_none_for_plain_content() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{"type":"paragraph","content":[{"type":"text","text":"no embeds"}]}]
        });
        assert_eq!(next_transclusion_ref(&doc), None);
        // A malformed transclusion (no refValue) contributes no hop rather than panicking.
        let bad = serde_json::json!({
            "type": "doc",
            "content": [{"type":"loomTransclusion","attrs":{}}]
        });
        assert_eq!(next_transclusion_ref(&bad), None);
    }
}
