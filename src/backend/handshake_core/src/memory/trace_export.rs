//! MT-165: Retrieval trace bundle exporter.
//!
//! Folded from WP-1-Retrieval-Trace-Bundle-Export. Takes a retrieval
//! `trace_id` and emits a redacted-by-default `TraceBundle` containing
//! the full RAG slice (QueryPlan + RetrievalTrace + budgets + cache
//! markers + selected spans + referenced artifacts + retrieval mode +
//! degradation report) for support / debugging without leaking PII.
//!
//! Redaction is a pre-step: every text field is scanned through the
//! same PII+credential scanner; redactions are recorded in
//! `redactions_applied` so reviewers can see what was hidden.

use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::progressive_retrieval::DegradationReport;
use super::retrieval_mode::RetrievalMode;
use crate::distillation::pii_patterns::{self, PiiKind};

pub const TRACE_EXPORT_VERSION: &str = "memory_trace_bundle_v1";

/// Hard bundle-size ceiling enforced post-serialization. 1 MiB matches
/// the contract red-team minimum control "oversize bundle bounded".
pub const MAX_BUNDLE_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPlan {
    pub plan_id: Uuid,
    pub query: String,
    pub task_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalTrace {
    pub trace_id: Uuid,
    pub query_plan_id: Uuid,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryPackBudgets {
    pub max_items: u32,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheMarkers {
    pub cache_hit_count: u32,
    pub cache_miss_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpanRef {
    pub span_id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub kind: String,
    pub bytes_estimate: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactionRecord {
    pub field: String,
    /// pii_email, pii_phone, credential, full_text — non-exhaustive.
    pub category: String,
    pub redacted_value_len: usize,
}

/// Optional route decision (from MT-164 RetrievalModeRouter) included
/// in the bundle so reviewers see why the retriever chose `retrieval_mode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub matched_rule_id: String,
    pub rationale: String,
}

/// Scoring inputs (from MT-161 InjectionScoringFormula) snapshotted at
/// retrieval time so the trace shows why each item was ranked where.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringInputSnapshot {
    pub item_id: String,
    pub importance: f64,
    pub recency: f64,
    pub trust: f64,
    pub outcome_tuned_weight: f64,
    pub embedding_similarity: f64,
    pub formula_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceBundle {
    pub trace_id: Uuid,
    pub query_plan: QueryPlan,
    pub retrieval_trace: RetrievalTrace,
    pub budgets: MemoryPackBudgets,
    pub cache_markers: CacheMarkers,
    pub retrieval_mode: RetrievalMode,
    pub route_decision: RouteDecision,
    pub degradation_report: DegradationReport,
    pub selected_spans: Vec<SpanRef>,
    pub referenced_artifacts: Vec<ArtifactRef>,
    pub scoring_inputs: Vec<ScoringInputSnapshot>,
    pub redactions_applied: Vec<RedactionRecord>,
    pub exported_at_utc: DateTime<Utc>,
    pub exporter_version: String,
}

impl TraceBundle {
    /// Deterministic content hash. Excludes `exported_at_utc` so the
    /// same trace produces the same hash across two exports.
    pub fn deterministic_hash(&self) -> String {
        let mut canonical = self.clone();
        canonical.exported_at_utc = DateTime::<Utc>::MIN_UTC;
        let bytes = serde_json::to_vec(&canonical).expect("canonical serialize");
        let mut h = Sha256::new();
        h.update(&bytes);
        format!("{:x}", h.finalize())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionPolicy {
    pub redact_pii: bool,
    pub redact_credentials: bool,
    pub redact_full_item_text: bool,
    pub allowlist_artifact_kinds: Vec<String>,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self {
            redact_pii: true,
            redact_credentials: true,
            redact_full_item_text: false,
            allowlist_artifact_kinds: vec!["packet.json".to_string()],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    JsonPretty,
    Tarball,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedBundle {
    pub format: ExportFormat,
    pub bytes: Vec<u8>,
    /// Deterministic content hash of the in-memory bundle (not of
    /// `bytes`). Lets recipients verify the trace shape independently
    /// of the serialization format.
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ExportError {
    #[error("unknown trace id: {trace_id}")]
    UnknownTrace { trace_id: Uuid },
    #[error("export serialization failed: {message}")]
    Serialization { message: String },
    #[error("redaction failed: {message}")]
    Redaction { message: String },
    #[error("bundle too large: {actual_bytes} bytes exceeds limit {limit_bytes}")]
    TooLarge {
        actual_bytes: usize,
        limit_bytes: usize,
    },
    #[error("bundle deserialization failed: {message}")]
    Deserialization { message: String },
    #[error("artifact load failed for {artifact_id}: {message}")]
    ArtifactLoad {
        artifact_id: String,
        message: String,
    },
}

pub trait TraceSource {
    fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError>;

    /// Default implementation returns an empty payload — Tarball export
    /// will still include the artifact entry, just with zero bytes. A
    /// real source overrides this to fetch the actual artifact bytes
    /// so the tarball is self-contained per the contract minimum control.
    fn load_artifact(&self, artifact_id: &str) -> Result<Vec<u8>, ExportError> {
        let _ = artifact_id;
        Ok(Vec::new())
    }
}

/// Round-trip a bundle from bytes back into a `TraceBundle`. The
/// adversarial "corrupt-bundle deserialization rejected typed" test
/// hits this entry point.
pub fn deserialize_bundle(bytes: &[u8]) -> Result<TraceBundle, ExportError> {
    serde_json::from_slice(bytes).map_err(|e| ExportError::Deserialization {
        message: e.to_string(),
    })
}

pub struct RetrievalTraceExporter<'a> {
    pub source: &'a dyn TraceSource,
    pub max_bytes: usize,
}

impl<'a> RetrievalTraceExporter<'a> {
    pub fn new(source: &'a dyn TraceSource) -> Self {
        Self {
            source,
            max_bytes: MAX_BUNDLE_BYTES,
        }
    }

    /// Override the size limit. Useful for tests that synthesize an
    /// oversize bundle and want to verify the typed `TooLarge` error.
    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    pub fn export(
        &self,
        trace_id: Uuid,
        policy: &RedactionPolicy,
        format: ExportFormat,
    ) -> Result<ExportedBundle, ExportError> {
        let mut bundle = self.source.load_trace(trace_id)?;
        let redactions = apply_redactions(&mut bundle, policy);
        bundle.redactions_applied.extend(redactions);
        bundle.exported_at_utc = Utc::now();
        bundle.exporter_version = TRACE_EXPORT_VERSION.to_string();
        let content_hash = bundle.deterministic_hash();
        let bytes = match format {
            ExportFormat::Json => {
                serde_json::to_vec(&bundle).map_err(|e| ExportError::Serialization {
                    message: e.to_string(),
                })?
            }
            ExportFormat::JsonPretty => {
                serde_json::to_vec_pretty(&bundle).map_err(|e| ExportError::Serialization {
                    message: e.to_string(),
                })?
            }
            ExportFormat::Tarball => {
                // Per contract: "Tarball includes referenced artifacts as
                // separate files". MT-165 finding #1 (FAIL): artifact bytes
                // MUST pass through the same redactor as inline text before
                // embedding — packet.json is in the DEFAULT allowlist, so a
                // credential-bearing packet would otherwise be exported
                // verbatim. Redact each artifact first, record what was
                // redacted, then serialize the manifest so the embedded
                // bundle reflects the artifact-level redactions too. (Json /
                // JsonPretty never embed artifact bytes, so this leak is
                // tarball-specific.)
                let artifacts = bundle.referenced_artifacts.clone();
                let mut artifact_entries: Vec<(String, Vec<u8>)> = Vec::new();
                let mut artifact_records: Vec<RedactionRecord> = Vec::new();
                for art in &artifacts {
                    let raw = self.source.load_artifact(&art.artifact_id)?;
                    let (redacted, recs) = redact_artifact_bytes(&art.artifact_id, raw, policy)?;
                    artifact_records.extend(recs);
                    let name = format!(
                        "artifacts/{}-{}",
                        sanitize_filename(&art.artifact_id),
                        sanitize_filename(&art.kind)
                    );
                    artifact_entries.push((name, redacted));
                }
                bundle.redactions_applied.extend(artifact_records);
                let payload =
                    serde_json::to_vec(&bundle).map_err(|e| ExportError::Serialization {
                        message: e.to_string(),
                    })?;
                let mut entries: Vec<(String, Vec<u8>)> =
                    vec![("trace_bundle.json".to_string(), payload)];
                entries.extend(artifact_entries);
                build_multi_tarball(&entries)
            }
        };
        if bytes.len() > self.max_bytes {
            return Err(ExportError::TooLarge {
                actual_bytes: bytes.len(),
                limit_bytes: self.max_bytes,
            });
        }
        Ok(ExportedBundle {
            format,
            bytes,
            content_hash,
        })
    }
}

fn apply_redactions(bundle: &mut TraceBundle, policy: &RedactionPolicy) -> Vec<RedactionRecord> {
    let mut records = Vec::new();
    if policy.redact_pii {
        for span in &mut bundle.selected_spans {
            let (redacted, count) = redact_pii(&span.text);
            if count > 0 {
                records.push(RedactionRecord {
                    field: format!("selected_spans.{}", span.span_id),
                    category: "pii".to_string(),
                    redacted_value_len: count,
                });
                span.text = redacted;
            }
        }
        // PII can also live in QueryPlan.query — operators sometimes
        // paste credentials into the prompt itself.
        let (redacted, count) = redact_pii(&bundle.query_plan.query);
        if count > 0 {
            records.push(RedactionRecord {
                field: "query_plan.query".to_string(),
                category: "pii".to_string(),
                redacted_value_len: count,
            });
            bundle.query_plan.query = redacted;
        }
    }
    if policy.redact_credentials {
        for span in &mut bundle.selected_spans {
            let (redacted, count) = redact_credentials(&span.text);
            if count > 0 {
                records.push(RedactionRecord {
                    field: format!("selected_spans.{}", span.span_id),
                    category: "credentials".to_string(),
                    redacted_value_len: count,
                });
                span.text = redacted;
            }
        }
        let (redacted, count) = redact_credentials(&bundle.query_plan.query);
        if count > 0 {
            records.push(RedactionRecord {
                field: "query_plan.query".to_string(),
                category: "credentials".to_string(),
                redacted_value_len: count,
            });
            bundle.query_plan.query = redacted;
        }
    }
    if policy.redact_full_item_text {
        for span in &mut bundle.selected_spans {
            if !span.text.is_empty() {
                records.push(RedactionRecord {
                    field: format!("selected_spans.{}", span.span_id),
                    category: "full_text".to_string(),
                    redacted_value_len: span.text.len(),
                });
                span.text = "[REDACTED]".to_string();
            }
        }
    }
    if !policy.allowlist_artifact_kinds.is_empty() {
        let before = bundle.referenced_artifacts.len();
        bundle.referenced_artifacts.retain(|artifact| {
            policy
                .allowlist_artifact_kinds
                .iter()
                .any(|kind| kind == &artifact.kind)
        });
        let filtered = before - bundle.referenced_artifacts.len();
        if filtered > 0 {
            records.push(RedactionRecord {
                field: "referenced_artifacts".to_string(),
                category: "allowlist_filter".to_string(),
                redacted_value_len: filtered,
            });
        }
    }
    records
}

// Credential keyword-assignment pattern (e.g. `password=hunter2`,
// `token: abc`). `pii_patterns` detects api-key TOKENS by known prefix
// (sk-, AKIA, ghp_, ...); this complements it by catching generic
// `keyword=value` secret assignments the token detector cannot see.
fn secret_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)(api[-_]?key|secret|token|password)[=:][^\s,]+").unwrap())
}

/// Placeholder token for a redacted PII detection, keyed by kind.
fn pii_placeholder(kind: PiiKind) -> &'static str {
    match kind {
        PiiKind::Email => "[REDACTED-EMAIL]",
        PiiKind::Phone => "[REDACTED-PHONE]",
        PiiKind::CreditCard => "[REDACTED-CREDIT-CARD]",
        PiiKind::ApiKey => "[REDACTED-API-KEY]",
        PiiKind::WindowsUserPath => "[REDACTED-WINDOWS-PATH]",
        PiiKind::MacAddress => "[REDACTED-MAC]",
        PiiKind::Ipv4 => "[REDACTED-IPV4]",
    }
}

/// Replace the byte ranges of `detections` (a subset of
/// `pii_patterns::scan` output) with kind-specific placeholders.
/// Overlapping detections are collapsed (first-wins) and replacement runs
/// back-to-front so earlier byte offsets stay valid. Returns the redacted
/// text and the number of replacements applied.
fn replace_detections(text: &str, mut detections: Vec<pii_patterns::PiiDetection>) -> (String, usize) {
    detections.sort_by_key(|d| (d.start, d.end));
    let mut kept: Vec<pii_patterns::PiiDetection> = Vec::new();
    let mut last_end = 0usize;
    for d in detections {
        if d.start >= last_end {
            last_end = d.end;
            kept.push(d);
        }
    }
    if kept.is_empty() {
        return (text.to_string(), 0);
    }
    let count = kept.len();
    let mut out = text.to_string();
    for d in kept.iter().rev() {
        out.replace_range(d.start..d.end, pii_placeholder(d.kind));
    }
    (out, count)
}

/// PII redaction via the canonical MT-120 `pii_patterns` scanner — the
/// same detector the distillation content-review pipeline uses (email,
/// phone, Luhn-checked credit card, Windows user path, MAC, IPv4). This
/// supersedes the prior ad-hoc email/phone regexes (MT-165 finding #2 +
/// #3). API-key tokens are handled in `redact_credentials`. Full NER
/// remains deferred to a future WP per `pii_patterns` / MT-120.
fn redact_pii(text: &str) -> (String, usize) {
    let detections: Vec<_> = pii_patterns::scan(text)
        .into_iter()
        .filter(|d| d.kind != PiiKind::ApiKey)
        .collect();
    replace_detections(text, detections)
}

/// Credential redaction: api-key TOKENS detected by `pii_patterns`
/// (sk-, AKIA, ghp_, ...) plus generic `keyword=value` secret
/// assignments the token detector cannot see.
fn redact_credentials(text: &str) -> (String, usize) {
    let token_detections: Vec<_> = pii_patterns::scan(text)
        .into_iter()
        .filter(|d| d.kind == PiiKind::ApiKey)
        .collect();
    let (after_tokens, token_hits) = replace_detections(text, token_detections);
    let secret_hits = secret_re().find_iter(&after_tokens).count();
    let after = secret_re()
        .replace_all(&after_tokens, "[REDACTED-SECRET]")
        .into_owned();
    (after, token_hits + secret_hits)
}

/// Redact a referenced artifact's bytes before embedding it in a tarball.
/// MT-165 finding #1 (FAIL): artifact payloads must pass through the same
/// redactor as inline text — otherwise a credential-bearing artifact that
/// survives the allowlist (packet.json is allowed by default) would be
/// exported verbatim. UTF-8 text artifacts are scanned and redacted;
/// non-UTF-8 binary artifacts cannot be scanned, so they are replaced with
/// a placeholder when full-text redaction is enabled and otherwise
/// rejected (fail-closed) so an unscannable secret cannot leak.
fn redact_artifact_bytes(
    artifact_id: &str,
    raw: Vec<u8>,
    policy: &RedactionPolicy,
) -> Result<(Vec<u8>, Vec<RedactionRecord>), ExportError> {
    if !policy.redact_pii && !policy.redact_credentials {
        return Ok((raw, Vec::new()));
    }
    match std::str::from_utf8(&raw) {
        Ok(text) => {
            let mut records = Vec::new();
            let mut current = text.to_string();
            if policy.redact_pii {
                let (red, count) = redact_pii(&current);
                if count > 0 {
                    records.push(RedactionRecord {
                        field: format!("artifacts.{artifact_id}"),
                        category: "pii".to_string(),
                        redacted_value_len: count,
                    });
                }
                current = red;
            }
            if policy.redact_credentials {
                let (red, count) = redact_credentials(&current);
                if count > 0 {
                    records.push(RedactionRecord {
                        field: format!("artifacts.{artifact_id}"),
                        category: "credentials".to_string(),
                        redacted_value_len: count,
                    });
                }
                current = red;
            }
            Ok((current.into_bytes(), records))
        }
        Err(_) => {
            if policy.redact_full_item_text {
                Ok((
                    b"[BINARY ARTIFACT REDACTED]".to_vec(),
                    vec![RedactionRecord {
                        field: format!("artifacts.{artifact_id}"),
                        category: "full_text".to_string(),
                        redacted_value_len: raw.len(),
                    }],
                ))
            } else {
                Err(ExportError::Redaction {
                    message: format!(
                        "artifact {artifact_id} is binary (non-UTF-8) and cannot be \
                         redaction-scanned; enable redact_full_item_text to export it as a \
                         placeholder or disable redaction"
                    ),
                })
            }
        }
    }
}

fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn build_multi_tarball(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut tar: Vec<u8> = Vec::new();
    for (name, payload) in entries {
        tar.extend_from_slice(&build_tar_entry(name, payload));
    }
    // Two zero blocks signaling end of archive.
    tar.extend(std::iter::repeat(0u8).take(1024));
    tar
}

fn build_tar_entry(filename: &str, payload: &[u8]) -> Vec<u8> {
    // POSIX ustar single-file header (512) + payload padded to 512.
    let mut header = [0u8; 512];
    let name_bytes = filename.as_bytes();
    let name_len = name_bytes.len().min(99);
    header[..name_len].copy_from_slice(&name_bytes[..name_len]);
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    let size_octal = format!("{:011o}\0", payload.len());
    header[124..136].copy_from_slice(size_octal.as_bytes());
    header[136..148].copy_from_slice(b"00000000000\0");
    for b in &mut header[148..156] {
        *b = b' ';
    }
    header[156] = b'0';
    header[257..262].copy_from_slice(b"ustar");
    header[263..265].copy_from_slice(b"00");
    let mut chksum: u32 = 0;
    for b in &header {
        chksum += *b as u32;
    }
    let chksum_str = format!("{:06o}\0 ", chksum);
    let chk_bytes = chksum_str.as_bytes();
    header[148..148 + chk_bytes.len()].copy_from_slice(chk_bytes);

    let mut entry: Vec<u8> = Vec::with_capacity(512 + payload.len() + 512);
    entry.extend_from_slice(&header);
    entry.extend_from_slice(payload);
    let pad = (512 - payload.len() % 512) % 512;
    entry.extend(std::iter::repeat(0u8).take(pad));
    entry
}

#[cfg(test)]
mod tests {
    use super::super::progressive_retrieval::{TIER_FULL_TEXT, TIER_VECTOR};
    use super::*;

    struct StubSource;
    impl TraceSource for StubSource {
        fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError> {
            Ok(TraceBundle {
                trace_id,
                query_plan: QueryPlan {
                    plan_id: Uuid::nil(),
                    query: "user@example.com please call +1 555-0100".to_string(),
                    task_type: "general".to_string(),
                },
                retrieval_trace: RetrievalTrace {
                    trace_id,
                    query_plan_id: Uuid::nil(),
                    item_ids: vec!["item-1".to_string()],
                },
                budgets: MemoryPackBudgets {
                    max_items: 10,
                    max_bytes: 65_536,
                },
                cache_markers: CacheMarkers {
                    cache_hit_count: 1,
                    cache_miss_count: 2,
                },
                retrieval_mode: RetrievalMode::Hybrid,
                route_decision: RouteDecision {
                    matched_rule_id: "general_freeform_query".to_string(),
                    rationale: "freeform queries fall back to hybrid".to_string(),
                },
                degradation_report: DegradationReport {
                    tiers_completed: vec![TIER_FULL_TEXT.to_string(), TIER_VECTOR.to_string()],
                    tiers_skipped: Vec::new(),
                    load_signal_at_start: 0.0,
                    started_at_utc: DateTime::<Utc>::MIN_UTC,
                    total_duration_ms: 0,
                    tier_chosen: super::super::capsule::DegradationTier::Tiered,
                },
                selected_spans: vec![SpanRef {
                    span_id: "span-1".to_string(),
                    text: "contact me at user@example.com or api_key=secret123".to_string(),
                }],
                referenced_artifacts: vec![
                    ArtifactRef {
                        artifact_id: "art-1".to_string(),
                        kind: "packet.json".to_string(),
                        bytes_estimate: 1024,
                    },
                    ArtifactRef {
                        artifact_id: "art-2".to_string(),
                        kind: "binary_blob".to_string(),
                        bytes_estimate: 1024,
                    },
                ],
                scoring_inputs: vec![ScoringInputSnapshot {
                    item_id: "item-1".to_string(),
                    importance: 0.7,
                    recency: 0.9,
                    trust: 0.8,
                    outcome_tuned_weight: 1.0,
                    embedding_similarity: 0.85,
                    formula_version: "v0".to_string(),
                }],
                redactions_applied: Vec::new(),
                exported_at_utc: DateTime::<Utc>::MIN_UTC,
                exporter_version: TRACE_EXPORT_VERSION.to_string(),
            })
        }
    }

    #[test]
    fn export_round_trips_via_json() {
        let src = StubSource;
        let exporter = RetrievalTraceExporter::new(&src);
        let trace_id = Uuid::now_v7();
        let bundle = exporter
            .export(trace_id, &RedactionPolicy::default(), ExportFormat::Json)
            .unwrap();
        let decoded: TraceBundle = serde_json::from_slice(&bundle.bytes).unwrap();
        assert_eq!(decoded.trace_id, trace_id);
    }

    #[test]
    fn pii_redaction_default_on() {
        let src = StubSource;
        let exporter = RetrievalTraceExporter::new(&src);
        let bundle = exporter
            .export(
                Uuid::now_v7(),
                &RedactionPolicy::default(),
                ExportFormat::Json,
            )
            .unwrap();
        let decoded: TraceBundle = serde_json::from_slice(&bundle.bytes).unwrap();
        let txt = &decoded.selected_spans[0].text;
        assert!(txt.contains("[REDACTED-EMAIL]"));
        assert!(txt.contains("[REDACTED-SECRET]"));
        assert!(!decoded.redactions_applied.is_empty());
    }

    #[test]
    fn allowlist_filters_artifacts() {
        let src = StubSource;
        let exporter = RetrievalTraceExporter::new(&src);
        let bundle = exporter
            .export(
                Uuid::now_v7(),
                &RedactionPolicy::default(),
                ExportFormat::Json,
            )
            .unwrap();
        let decoded: TraceBundle = serde_json::from_slice(&bundle.bytes).unwrap();
        assert_eq!(decoded.referenced_artifacts.len(), 1);
        assert_eq!(decoded.referenced_artifacts[0].kind, "packet.json");
    }

    #[test]
    fn tarball_export_has_ustar_header() {
        let src = StubSource;
        let exporter = RetrievalTraceExporter::new(&src);
        let bundle = exporter
            .export(
                Uuid::now_v7(),
                &RedactionPolicy::default(),
                ExportFormat::Tarball,
            )
            .unwrap();
        assert!(bundle.bytes.len() >= 1024);
        let magic = &bundle.bytes[257..262];
        assert_eq!(magic, b"ustar");
    }

    #[test]
    fn opt_out_of_pii_keeps_text() {
        let src = StubSource;
        let exporter = RetrievalTraceExporter::new(&src);
        let policy = RedactionPolicy {
            redact_pii: false,
            redact_credentials: false,
            redact_full_item_text: false,
            allowlist_artifact_kinds: Vec::new(),
        };
        let bundle = exporter
            .export(Uuid::now_v7(), &policy, ExportFormat::Json)
            .unwrap();
        let decoded: TraceBundle = serde_json::from_slice(&bundle.bytes).unwrap();
        assert!(decoded.selected_spans[0].text.contains("user@example.com"));
    }

    /// Source whose `load_artifact` returns a credential- + email-bearing
    /// text payload, used to prove MT-165 finding #1: artifact bytes must be
    /// redacted before they are embedded in the tarball.
    struct SecretArtifactSource;
    impl TraceSource for SecretArtifactSource {
        fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError> {
            StubSource.load_trace(trace_id)
        }
        fn load_artifact(&self, _artifact_id: &str) -> Result<Vec<u8>, ExportError> {
            Ok(b"artifact secret: password=topsecretvalue contact ops@corp.example.com".to_vec())
        }
    }

    #[test]
    fn tarball_redacts_artifact_bytes_before_embedding() {
        // MT-165 finding #1 (FAIL) regression: a credential-bearing artifact
        // that survives the default allowlist (packet.json) must be redacted
        // before it lands in the tarball.
        let src = SecretArtifactSource;
        let exporter = RetrievalTraceExporter::new(&src);
        let bundle = exporter
            .export(
                Uuid::now_v7(),
                &RedactionPolicy::default(),
                ExportFormat::Tarball,
            )
            .unwrap();
        let haystack = String::from_utf8_lossy(&bundle.bytes);
        assert!(
            !haystack.contains("topsecretvalue"),
            "raw credential leaked verbatim into the tarball artifact entry"
        );
        assert!(
            !haystack.contains("ops@corp.example.com"),
            "raw email leaked verbatim into the tarball artifact entry"
        );
        assert!(
            haystack.contains("[REDACTED-SECRET]"),
            "expected a credential redaction placeholder in the artifact entry"
        );
        assert!(
            haystack.contains("[REDACTED-EMAIL]"),
            "expected an email redaction placeholder in the artifact entry"
        );
    }

    #[test]
    fn tarball_binary_artifact_fails_closed_without_full_text_redaction() {
        // A non-UTF-8 artifact cannot be scanned. With redaction on and
        // redact_full_item_text off, the export must fail closed rather than
        // embed unscannable bytes that could carry a secret.
        struct BinarySource;
        impl TraceSource for BinarySource {
            fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError> {
                StubSource.load_trace(trace_id)
            }
            fn load_artifact(&self, _artifact_id: &str) -> Result<Vec<u8>, ExportError> {
                Ok(vec![0xff, 0xfe, 0x00, 0x01]) // invalid UTF-8
            }
        }
        let src = BinarySource;
        let exporter = RetrievalTraceExporter::new(&src);
        let err = exporter
            .export(
                Uuid::now_v7(),
                &RedactionPolicy::default(),
                ExportFormat::Tarball,
            )
            .unwrap_err();
        assert!(matches!(err, ExportError::Redaction { .. }));
    }

    #[test]
    fn tarball_binary_artifact_placeholder_when_full_text_redaction_on() {
        struct BinarySource;
        impl TraceSource for BinarySource {
            fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError> {
                StubSource.load_trace(trace_id)
            }
            fn load_artifact(&self, _artifact_id: &str) -> Result<Vec<u8>, ExportError> {
                Ok(vec![0xff, 0xfe, 0x00, 0x01])
            }
        }
        let src = BinarySource;
        let exporter = RetrievalTraceExporter::new(&src);
        let policy = RedactionPolicy {
            redact_full_item_text: true,
            ..RedactionPolicy::default()
        };
        let bundle = exporter
            .export(Uuid::now_v7(), &policy, ExportFormat::Tarball)
            .unwrap();
        let haystack = String::from_utf8_lossy(&bundle.bytes);
        assert!(haystack.contains("[BINARY ARTIFACT REDACTED]"));
    }

    #[test]
    fn redact_pii_detects_shared_scanner_kinds() {
        // MT-165 finding #2: redaction routes through the canonical
        // pii_patterns scanner, so credit cards (Luhn), IPv4, and email —
        // which the prior ad-hoc email/phone regexes missed — are caught.
        // The security property is that the raw value is removed; the exact
        // placeholder label may vary when detections overlap (e.g. a card
        // digit-run can also match the phone pattern and be relabelled).
        let (red, count) =
            redact_pii("card 4242 4242 4242 4242 here, host 192.168.1.1, mail a@b.co");
        assert!(count >= 3, "expected >=3 detections; got {count}: {red}");
        assert!(!red.contains("4242 4242 4242 4242"), "card not redacted: {red}");
        assert!(!red.contains("192.168.1.1"), "ipv4 not redacted: {red}");
        assert!(!red.contains("a@b.co"), "email not redacted: {red}");
    }
}
