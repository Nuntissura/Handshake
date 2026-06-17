//! WP-KERNEL-009 MT-072 CRDTAndConcurrencyCore-072-VectorClockOrEquivalentMetadata.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — CRDT
//! receipts MUST carry state-vector evidence. The kernel CRDT layer already
//! persists `state_vector_before` / `state_vector_after` TEXT columns on
//! every `kernel_crdt_updates` row (migration 0020); this module gives those
//! columns a typed, canonical, comparable representation WITHOUT forcing a
//! specific external CRDT service (MT-072 contract scope).
//!
//! Model: a per-site version vector (`site_id -> clock`). Site ids come from
//! the MT-065 actor/site derivation, so attribution and causality share one
//! vocabulary. The encoding is deterministic (`hsk-sv1:` prefix, sites in
//! lexicographic order), which makes equality, persistence round-trips, and
//! replay-order proofs textual AND structural at the same time.
//!
//! Causality verdicts (`Equal | Dominates | DominatedBy | Concurrent`) drive
//! MT-070 concurrent-save decisions and the MT-075 conflict UI payload. The
//! lamport view (`lamport_max`) is the "or equivalent" clock the MT contract
//! allows for consumers that only need a total-order hint.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::persistence::CrdtUpdateRecordV1;

pub const KNOWLEDGE_STATE_VECTOR_PREFIX_V1: &str = "hsk-sv1:";
pub const KNOWLEDGE_CAUSAL_CHAIN_PROOF_SCHEMA_ID: &str =
    "hsk.kernel.knowledge_causal_chain_proof@1";

/// Typed per-update causal metadata: a version vector keyed by stable CRDT
/// site ids. Serialized as the canonical `hsk-sv1:` string everywhere.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KnowledgeStateVectorV1 {
    clocks: BTreeMap<String, u64>,
}

/// Causality relation between two state vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStateVectorOrdering {
    Equal,
    /// `self` has seen everything `other` has, plus more.
    Dominates,
    /// `other` has seen everything `self` has, plus more.
    DominatedBy,
    /// Each side has updates the other has not seen.
    Concurrent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeStateVectorParseError {
    MissingPrefix { found: String },
    EmptyEntry,
    MissingClockSeparator { entry: String },
    EmptySite { entry: String },
    BadSiteChar { entry: String, found: char },
    BadClock { entry: String },
    ZeroClock { entry: String },
    DuplicateSite { site: String },
}

impl std::fmt::Display for KnowledgeStateVectorParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPrefix { found } => write!(
                f,
                "state vector '{found}' is missing the '{KNOWLEDGE_STATE_VECTOR_PREFIX_V1}' prefix"
            ),
            Self::EmptyEntry => write!(f, "state vector contains an empty 'site=clock' entry"),
            Self::MissingClockSeparator { entry } => {
                write!(f, "state vector entry '{entry}' is missing '='")
            }
            Self::EmptySite { entry } => {
                write!(f, "state vector entry '{entry}' has an empty site")
            }
            Self::BadSiteChar { entry, found } => write!(
                f,
                "state vector entry '{entry}' contains forbidden site character '{found}'"
            ),
            Self::BadClock { entry } => {
                write!(f, "state vector entry '{entry}' has a non-u64 clock")
            }
            Self::ZeroClock { entry } => write!(
                f,
                "state vector entry '{entry}' has clock 0 (clocks start at 1; absent sites are omitted)"
            ),
            Self::DuplicateSite { site } => {
                write!(f, "state vector repeats site '{site}'")
            }
        }
    }
}

impl std::error::Error for KnowledgeStateVectorParseError {}

impl KnowledgeStateVectorV1 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Canonical text form: `hsk-sv1:siteA=3;siteB=1` with sites in
    /// lexicographic order (BTreeMap iteration order). The empty vector is
    /// exactly `hsk-sv1:`.
    pub fn encode(&self) -> String {
        let body = self
            .clocks
            .iter()
            .map(|(site, clock)| format!("{site}={clock}"))
            .collect::<Vec<_>>()
            .join(";");
        format!("{KNOWLEDGE_STATE_VECTOR_PREFIX_V1}{body}")
    }

    pub fn parse(value: &str) -> Result<Self, KnowledgeStateVectorParseError> {
        let body = value
            .strip_prefix(KNOWLEDGE_STATE_VECTOR_PREFIX_V1)
            .ok_or_else(|| KnowledgeStateVectorParseError::MissingPrefix {
                found: value.to_string(),
            })?;
        let mut clocks = BTreeMap::new();
        if body.is_empty() {
            return Ok(Self { clocks });
        }
        for entry in body.split(';') {
            if entry.is_empty() {
                return Err(KnowledgeStateVectorParseError::EmptyEntry);
            }
            let (site, clock_token) = entry.split_once('=').ok_or_else(|| {
                KnowledgeStateVectorParseError::MissingClockSeparator {
                    entry: entry.to_string(),
                }
            })?;
            if site.is_empty() {
                return Err(KnowledgeStateVectorParseError::EmptySite {
                    entry: entry.to_string(),
                });
            }
            if let Some(found) = site
                .chars()
                .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
            {
                return Err(KnowledgeStateVectorParseError::BadSiteChar {
                    entry: entry.to_string(),
                    found,
                });
            }
            let clock: u64 =
                clock_token
                    .parse()
                    .map_err(|_| KnowledgeStateVectorParseError::BadClock {
                        entry: entry.to_string(),
                    })?;
            if clock == 0 {
                return Err(KnowledgeStateVectorParseError::ZeroClock {
                    entry: entry.to_string(),
                });
            }
            if clocks.insert(site.to_string(), clock).is_some() {
                return Err(KnowledgeStateVectorParseError::DuplicateSite {
                    site: site.to_string(),
                });
            }
        }
        Ok(Self { clocks })
    }

    pub fn clock(&self, site_id: &str) -> u64 {
        self.clocks.get(site_id).copied().unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.clocks.is_empty()
    }

    pub fn sites(&self) -> impl Iterator<Item = (&str, u64)> {
        self.clocks
            .iter()
            .map(|(site, clock)| (site.as_str(), *clock))
    }

    /// Record that `site_id` has produced (or been observed at) `clock`,
    /// keeping the maximum.
    pub fn observe(&mut self, site_id: &str, clock: u64) {
        if clock == 0 {
            return;
        }
        let entry = self.clocks.entry(site_id.to_string()).or_insert(0);
        if clock > *entry {
            *entry = clock;
        }
    }

    /// Advance `site_id` by one local event; returns the new clock value.
    pub fn increment(&mut self, site_id: &str) -> u64 {
        let entry = self.clocks.entry(site_id.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Pointwise-maximum merge (the CRDT join of two version vectors).
    pub fn merge(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        for (site, clock) in &other.clocks {
            merged.observe(site, *clock);
        }
        merged
    }

    /// Causality comparison against `other`.
    pub fn compare(&self, other: &Self) -> KnowledgeStateVectorOrdering {
        let mut self_ahead = false;
        let mut other_ahead = false;
        for (site, clock) in &self.clocks {
            match other.clock(site).cmp(clock) {
                std::cmp::Ordering::Less => self_ahead = true,
                std::cmp::Ordering::Greater => other_ahead = true,
                std::cmp::Ordering::Equal => {}
            }
        }
        for (site, clock) in &other.clocks {
            if self.clock(site) < *clock {
                other_ahead = true;
            }
        }
        match (self_ahead, other_ahead) {
            (false, false) => KnowledgeStateVectorOrdering::Equal,
            (true, false) => KnowledgeStateVectorOrdering::Dominates,
            (false, true) => KnowledgeStateVectorOrdering::DominatedBy,
            (true, true) => KnowledgeStateVectorOrdering::Concurrent,
        }
    }

    pub fn dominates_or_equal(&self, other: &Self) -> bool {
        matches!(
            self.compare(other),
            KnowledgeStateVectorOrdering::Equal | KnowledgeStateVectorOrdering::Dominates
        )
    }

    /// The "or equivalent" lamport view: the maximum clock across sites.
    /// Strictly grows along any causal chain, so it is a safe total-order
    /// hint for consumers that do not need per-site attribution.
    pub fn lamport_max(&self) -> u64 {
        self.clocks.values().copied().max().unwrap_or(0)
    }
}

impl std::fmt::Display for KnowledgeStateVectorV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl TryFrom<String> for KnowledgeStateVectorV1 {
    type Error = KnowledgeStateVectorParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<KnowledgeStateVectorV1> for String {
    fn from(value: KnowledgeStateVectorV1) -> Self {
        value.encode()
    }
}

/// One verified causal step inside a [`KnowledgeCausalChainProofV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeCausalStepV1 {
    pub update_id: String,
    pub update_seq: u64,
    pub actor_id: String,
    pub advanced_sites: Vec<String>,
    pub state_vector_before: String,
    pub state_vector_after: String,
    pub lamport_after: u64,
}

/// Machine-checkable replay-ordering proof over persisted update records:
/// every update's `after` strictly dominates its `before`, and each update's
/// `before` equals the previous update's `after` (linear server-ordered log).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeCausalChainProofV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub steps: Vec<KnowledgeCausalStepV1>,
    pub final_state_vector: String,
    pub final_lamport: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeCausalChainError {
    EmptyChain,
    UnparseableVector {
        update_id: String,
        field: &'static str,
        error: KnowledgeStateVectorParseError,
    },
    AfterDoesNotDominateBefore {
        update_id: String,
        ordering: KnowledgeStateVectorOrdering,
    },
    BeforeBreaksChain {
        update_id: String,
        expected_before: String,
        found_before: String,
    },
    NonContiguousSequence {
        expected: u64,
        found: u64,
    },
}

impl std::fmt::Display for KnowledgeCausalChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyChain => write!(f, "causal chain proof requires at least one update"),
            Self::UnparseableVector {
                update_id,
                field,
                error,
            } => write!(
                f,
                "update '{update_id}' field {field} is not a typed state vector: {error}"
            ),
            Self::AfterDoesNotDominateBefore {
                update_id,
                ordering,
            } => write!(
                f,
                "update '{update_id}' state_vector_after does not strictly dominate state_vector_before (ordering {ordering:?})"
            ),
            Self::BeforeBreaksChain {
                update_id,
                expected_before,
                found_before,
            } => write!(
                f,
                "update '{update_id}' state_vector_before '{found_before}' does not continue the chain (expected '{expected_before}')"
            ),
            Self::NonContiguousSequence { expected, found } => write!(
                f,
                "update_seq gap while proving causal chain: expected {expected}, found {found}"
            ),
        }
    }
}

impl std::error::Error for KnowledgeCausalChainError {}

/// Verify the persisted causal metadata of an ordered update log and emit a
/// replay-ordering proof. Records may be passed unordered; they are sorted
/// by `update_seq` (the Postgres replay order) first.
pub fn verify_causal_chain(
    records: &[CrdtUpdateRecordV1],
) -> Result<KnowledgeCausalChainProofV1, KnowledgeCausalChainError> {
    if records.is_empty() {
        return Err(KnowledgeCausalChainError::EmptyChain);
    }
    let mut ordered: Vec<&CrdtUpdateRecordV1> = records.iter().collect();
    ordered.sort_by_key(|record| record.update_seq);

    let mut steps = Vec::with_capacity(ordered.len());
    let mut expected_seq = ordered[0].update_seq;
    let mut previous_after: Option<KnowledgeStateVectorV1> = None;

    for record in ordered.iter() {
        if record.update_seq != expected_seq {
            return Err(KnowledgeCausalChainError::NonContiguousSequence {
                expected: expected_seq,
                found: record.update_seq,
            });
        }
        expected_seq += 1;

        let before =
            KnowledgeStateVectorV1::parse(&record.state_vector_before).map_err(|error| {
                KnowledgeCausalChainError::UnparseableVector {
                    update_id: record.update_id.clone(),
                    field: "state_vector_before",
                    error,
                }
            })?;
        let after = KnowledgeStateVectorV1::parse(&record.state_vector_after).map_err(|error| {
            KnowledgeCausalChainError::UnparseableVector {
                update_id: record.update_id.clone(),
                field: "state_vector_after",
                error,
            }
        })?;

        let ordering = after.compare(&before);
        if ordering != KnowledgeStateVectorOrdering::Dominates {
            return Err(KnowledgeCausalChainError::AfterDoesNotDominateBefore {
                update_id: record.update_id.clone(),
                ordering,
            });
        }

        if let Some(previous) = &previous_after {
            if previous != &before {
                return Err(KnowledgeCausalChainError::BeforeBreaksChain {
                    update_id: record.update_id.clone(),
                    expected_before: previous.encode(),
                    found_before: before.encode(),
                });
            }
        }

        let advanced_sites = after
            .sites()
            .filter(|(site, clock)| before.clock(site) < *clock)
            .map(|(site, _)| site.to_string())
            .collect();

        steps.push(KnowledgeCausalStepV1 {
            update_id: record.update_id.clone(),
            update_seq: record.update_seq,
            actor_id: record.actor_id.clone(),
            advanced_sites,
            state_vector_before: before.encode(),
            state_vector_after: after.encode(),
            lamport_after: after.lamport_max(),
        });
        previous_after = Some(after);
    }

    let final_vector = previous_after.expect("non-empty chain verified above");
    let first = ordered.first().expect("non-empty chain");
    Ok(KnowledgeCausalChainProofV1 {
        schema_id: KNOWLEDGE_CAUSAL_CHAIN_PROOF_SCHEMA_ID.to_string(),
        workspace_id: first.workspace_id.clone(),
        document_id: first.document_id.clone(),
        crdt_document_id: first.crdt_document_id.clone(),
        final_lamport: final_vector.lamport_max(),
        final_state_vector: final_vector.encode(),
        steps,
    })
}
