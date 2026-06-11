//! WP-KERNEL-009 MT-107 CodeIndexStalenessDetector.
//!
//! Master Spec anchor: 2.3.13.11 KnowledgeSource staleness ("mark stale, never
//! serve stale silently"). Decides, for a code file, whether its current index
//! (symbols/spans/edges) is stale relative to the source.
//!
//! Pure logic; no DB. The engine ([`super::engine`]) reads the persisted
//! `knowledge_code_files` row (indexed content hash + parser version) and the
//! live source state (current content hash + the adapter's parser version), and
//! asks [`evaluate_staleness`] for the verdict. The nav API
//! (`api/knowledge_code_nav.rs`) attaches the resulting [`StalenessVerdict`] to
//! every served symbol so a stale result is FLAGGED, never silently returned as
//! fresh.

use serde::{Deserialize, Serialize};

/// Why a code file's index is stale (or that it is fresh).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StalenessVerdict {
    /// Index reflects the current source hash and parser version.
    Fresh,
    /// The source content hash changed since indexing.
    SourceChanged {
        indexed_hash: String,
        current_hash: String,
    },
    /// The parser version changed since indexing (grammar/extractor upgrade),
    /// so spans/edges may be incomplete even if content is identical.
    ParserChanged {
        indexed_parser_version: String,
        current_parser_version: String,
    },
    /// The file was explicitly marked stale (e.g. source deleted/moved by the
    /// ingestion lifecycle) and not yet re-indexed.
    MarkedStale,
}

impl StalenessVerdict {
    pub fn is_fresh(&self) -> bool {
        matches!(self, Self::Fresh)
    }

    /// Machine label for the served payload.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::SourceChanged { .. } => "source_changed",
            Self::ParserChanged { .. } => "parser_changed",
            Self::MarkedStale => "marked_stale",
        }
    }
}

/// The current live source state to compare against the persisted index state.
#[derive(Debug, Clone)]
pub struct LiveSourceState {
    pub current_content_hash: String,
    pub current_parser_version: String,
}

/// The persisted index state (from `knowledge_code_files`).
#[derive(Debug, Clone)]
pub struct IndexedState {
    pub indexed_content_hash: String,
    pub indexed_parser_version: String,
    /// The row's `stale` flag (set by the ingestion lifecycle or a prior
    /// detector pass).
    pub marked_stale: bool,
}

/// Evaluate staleness. Precedence: an explicit stale mark first, then a content
/// hash change, then a parser-version change. Content takes precedence over
/// parser because a content change implies the spans are definitely wrong,
/// whereas a parser change only means they MIGHT be incomplete.
pub fn evaluate_staleness(indexed: &IndexedState, live: &LiveSourceState) -> StalenessVerdict {
    if indexed.marked_stale {
        return StalenessVerdict::MarkedStale;
    }
    if indexed.indexed_content_hash != live.current_content_hash {
        return StalenessVerdict::SourceChanged {
            indexed_hash: indexed.indexed_content_hash.clone(),
            current_hash: live.current_content_hash.clone(),
        };
    }
    if indexed.indexed_parser_version != live.current_parser_version {
        return StalenessVerdict::ParserChanged {
            indexed_parser_version: indexed.indexed_parser_version.clone(),
            current_parser_version: live.current_parser_version.clone(),
        };
    }
    StalenessVerdict::Fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed(hash: &str, parser: &str, marked: bool) -> IndexedState {
        IndexedState {
            indexed_content_hash: hash.to_string(),
            indexed_parser_version: parser.to_string(),
            marked_stale: marked,
        }
    }

    fn live(hash: &str, parser: &str) -> LiveSourceState {
        LiveSourceState {
            current_content_hash: hash.to_string(),
            current_parser_version: parser.to_string(),
        }
    }

    #[test]
    fn fresh_when_identical() {
        let v = evaluate_staleness(&indexed("a", "p1", false), &live("a", "p1"));
        assert_eq!(v, StalenessVerdict::Fresh);
        assert!(v.is_fresh());
    }

    #[test]
    fn marked_stale_wins() {
        let v = evaluate_staleness(&indexed("a", "p1", true), &live("a", "p1"));
        assert_eq!(v, StalenessVerdict::MarkedStale);
    }

    #[test]
    fn source_change_detected_and_precedes_parser_change() {
        let v = evaluate_staleness(&indexed("a", "p1", false), &live("b", "p2"));
        assert_eq!(v.label(), "source_changed");
    }

    #[test]
    fn parser_change_detected_when_content_same() {
        let v = evaluate_staleness(&indexed("a", "p1", false), &live("a", "p2"));
        assert_eq!(v.label(), "parser_changed");
        assert!(!v.is_fresh());
    }
}
