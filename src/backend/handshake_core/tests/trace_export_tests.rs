//! MT-165 — RetrievalTraceExporter integration tests.
//!
//! Per the MT-165 contract proof_command:
//!   `cargo test -p handshake_core --test trace_export_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/memory/trace_export.rs
//! already covers the happy paths (Json round-trip, default PII
//! redaction, allowlist filtering, tarball ustar magic, opt-out keeps
//! text). This integration file satisfies the contract owned_files
//! entry and adds the adversarial scenarios the orchestrator handoff
//! envelope demanded:
//!
//!   - serde round-trip per ExportFormat (Json, JsonPretty, Tarball).
//!   - missing-field deserialization rejection is typed
//!     (`ExportError::Deserialization`).
//!   - corrupt-bundle bytes are rejected typed.
//!   - oversize bundle is bounded — exporter returns
//!     `ExportError::TooLarge { actual_bytes, limit_bytes }` rather
//!     than emitting a bundle larger than `MAX_BUNDLE_BYTES`.
//!   - trace records the MT-164 RetrievalMode + matched-rule
//!     RouteDecision so a reviewer can see which router rule fired.
//!   - trace records the MT-161 DegradationReport (tiers completed /
//!     tiers skipped / tier chosen) so the reviewer sees what the
//!     progressive retriever actually ran.
//!   - trace records the MT-161 scoring inputs per item so the
//!     reviewer can reproduce the ranking.
//!   - default policy MUST redact PII and credentials in every text
//!     field (selected_spans + query_plan.query).
//!   - the same trace produces the same `content_hash` across two
//!     exports (deterministic content hash; `exported_at_utc` does
//!     not enter the hash domain).
//!   - unknown trace id surfaces typed `ExportError::UnknownTrace`.
//!   - Tarball includes referenced artifacts as separate files —
//!     each surviving artifact appears as its own ustar entry.

use chrono::{DateTime, Utc};
use handshake_core::memory::capsule::DegradationTier;
use handshake_core::memory::progressive_retrieval::{
    DegradationReport, TIER_FULL_TEXT, TIER_GRAPH_EXPANSION, TIER_RERANK, TIER_VECTOR,
};
use handshake_core::memory::retrieval_mode::RetrievalMode;
use handshake_core::memory::trace_export::{
    deserialize_bundle, ArtifactRef, CacheMarkers, ExportError, ExportFormat, ExportedBundle,
    MemoryPackBudgets, QueryPlan, RedactionPolicy, RetrievalTrace, RetrievalTraceExporter,
    RouteDecision, ScoringInputSnapshot, SpanRef, TraceBundle, TraceSource, MAX_BUNDLE_BYTES,
    TRACE_EXPORT_VERSION,
};
use uuid::Uuid;

const FIXED_PLAN_ID: Uuid = Uuid::from_u128(0x01938b67_1234_7abc_89ef_0123456789ab);
const FIXED_TRACE_ID: Uuid = Uuid::from_u128(0x01938b67_2222_7abc_89ef_0123456789ac);

struct FixedSource {
    bundle: TraceBundle,
    artifact_bytes: std::collections::HashMap<String, Vec<u8>>,
}

impl FixedSource {
    fn new(bundle: TraceBundle) -> Self {
        Self {
            bundle,
            artifact_bytes: std::collections::HashMap::new(),
        }
    }

    fn with_artifact(mut self, id: &str, bytes: Vec<u8>) -> Self {
        self.artifact_bytes.insert(id.to_string(), bytes);
        self
    }
}

impl TraceSource for FixedSource {
    fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError> {
        if trace_id != self.bundle.trace_id {
            return Err(ExportError::UnknownTrace { trace_id });
        }
        Ok(self.bundle.clone())
    }
    fn load_artifact(&self, artifact_id: &str) -> Result<Vec<u8>, ExportError> {
        Ok(self
            .artifact_bytes
            .get(artifact_id)
            .cloned()
            .unwrap_or_default())
    }
}

fn baseline_bundle() -> TraceBundle {
    TraceBundle {
        trace_id: FIXED_TRACE_ID,
        query_plan: QueryPlan {
            plan_id: FIXED_PLAN_ID,
            // Holds both PII (email + phone) and a credential — proves
            // the redaction pre-step scans query_plan.query, not only
            // selected_spans.
            query: "ping ops@example.com or +1 555-0100 with token=abc123XYZ".to_string(),
            task_type: "general".to_string(),
        },
        retrieval_trace: RetrievalTrace {
            trace_id: FIXED_TRACE_ID,
            query_plan_id: FIXED_PLAN_ID,
            item_ids: vec!["item-1".to_string(), "item-2".to_string()],
        },
        budgets: MemoryPackBudgets {
            max_items: 8,
            max_bytes: 32_768,
        },
        cache_markers: CacheMarkers {
            cache_hit_count: 3,
            cache_miss_count: 2,
        },
        retrieval_mode: RetrievalMode::Hybrid,
        route_decision: RouteDecision {
            matched_rule_id: "general_freeform_query".to_string(),
            rationale: "freeform queries fall back to hybrid".to_string(),
        },
        degradation_report: DegradationReport {
            tiers_completed: vec![TIER_FULL_TEXT.to_string(), TIER_VECTOR.to_string()],
            tiers_skipped: vec![
                TIER_GRAPH_EXPANSION.to_string(),
                TIER_RERANK.to_string(),
            ],
            load_signal_at_start: 0.85,
            started_at_utc: DateTime::<Utc>::MIN_UTC,
            total_duration_ms: 12,
            tier_chosen: DegradationTier::Tiered,
        },
        selected_spans: vec![
            SpanRef {
                span_id: "span-1".to_string(),
                text: "contact user@example.com or alt@example.org".to_string(),
            },
            SpanRef {
                span_id: "span-2".to_string(),
                text: "api_key=AKIASECRETHERE and another secret=xyz123".to_string(),
            },
        ],
        referenced_artifacts: vec![
            ArtifactRef {
                artifact_id: "art-1".to_string(),
                kind: "packet.json".to_string(),
                bytes_estimate: 1024,
            },
            ArtifactRef {
                artifact_id: "art-2".to_string(),
                kind: "binary_blob".to_string(),
                bytes_estimate: 2048,
            },
        ],
        scoring_inputs: vec![
            ScoringInputSnapshot {
                item_id: "item-1".to_string(),
                importance: 0.7,
                recency: 0.9,
                trust: 0.8,
                outcome_tuned_weight: 1.0,
                embedding_similarity: 0.85,
                formula_version: "v0".to_string(),
            },
            ScoringInputSnapshot {
                item_id: "item-2".to_string(),
                importance: 0.5,
                recency: 0.6,
                trust: 0.7,
                outcome_tuned_weight: 0.9,
                embedding_similarity: 0.72,
                formula_version: "v0".to_string(),
            },
        ],
        redactions_applied: Vec::new(),
        exported_at_utc: DateTime::<Utc>::MIN_UTC,
        exporter_version: TRACE_EXPORT_VERSION.to_string(),
    }
}

#[test]
fn mt165_round_trip_json() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export");
    let decoded: TraceBundle = deserialize_bundle(&exported.bytes).expect("decode");
    assert_eq!(decoded.trace_id, FIXED_TRACE_ID);
    assert_eq!(decoded.exporter_version, TRACE_EXPORT_VERSION);
}

#[test]
fn mt165_round_trip_json_pretty() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(
            FIXED_TRACE_ID,
            &RedactionPolicy::default(),
            ExportFormat::JsonPretty,
        )
        .expect("export");
    // Pretty must contain at least one newline so it's distinguishable
    // from the compact serializer.
    assert!(exported.bytes.contains(&b'\n'));
    let decoded: TraceBundle = deserialize_bundle(&exported.bytes).expect("decode");
    assert_eq!(decoded.trace_id, FIXED_TRACE_ID);
}

#[test]
fn mt165_round_trip_tarball_first_entry_is_bundle_json() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(
            FIXED_TRACE_ID,
            &RedactionPolicy::default(),
            ExportFormat::Tarball,
        )
        .expect("export");
    // First 100 bytes of a ustar header are the filename, NUL-padded.
    let name_end = exported.bytes[..100]
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(100);
    let name = std::str::from_utf8(&exported.bytes[..name_end]).expect("name utf8");
    assert_eq!(name, "trace_bundle.json");
    // ustar magic.
    assert_eq!(&exported.bytes[257..262], b"ustar");
    // Extract the JSON payload (size lives at bytes 124..136 as a
    // NUL-terminated octal string).
    let size_end = exported.bytes[124..136]
        .iter()
        .position(|b| *b == 0 || *b == b' ')
        .unwrap_or(11);
    let size_str = std::str::from_utf8(&exported.bytes[124..124 + size_end]).unwrap();
    let payload_len = usize::from_str_radix(size_str, 8).unwrap();
    let payload = &exported.bytes[512..512 + payload_len];
    let decoded: TraceBundle = deserialize_bundle(payload).expect("decode tar payload");
    assert_eq!(decoded.trace_id, FIXED_TRACE_ID);
}

#[test]
fn mt165_tarball_includes_referenced_artifacts_as_separate_files() {
    // Allow both artifact kinds through the policy so they survive
    // into the tarball; verify each surviving artifact appears as its
    // own ustar entry.
    let src = FixedSource::new(baseline_bundle())
        .with_artifact("art-1", b"packet contents".to_vec())
        .with_artifact("art-2", b"binary blob".to_vec());
    let policy = RedactionPolicy {
        redact_pii: true,
        redact_credentials: true,
        redact_full_item_text: false,
        allowlist_artifact_kinds: vec!["packet.json".to_string(), "binary_blob".to_string()],
    };
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(FIXED_TRACE_ID, &policy, ExportFormat::Tarball)
        .expect("export");
    // Walk tar entries and collect every entry filename. A real tar
    // has 512-byte aligned entries; the easiest correctness proof is
    // to look at every header position and decode the name field.
    let mut names: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    while cursor + 512 <= exported.bytes.len() {
        let header = &exported.bytes[cursor..cursor + 512];
        // End-of-archive: two zero blocks.
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let name_end = header[..100].iter().position(|b| *b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&header[..name_end])
            .unwrap_or("")
            .to_string();
        names.push(name);
        let size_end = header[124..136]
            .iter()
            .position(|b| *b == 0 || *b == b' ')
            .unwrap_or(11);
        let size_str = std::str::from_utf8(&header[124..124 + size_end]).unwrap_or("0");
        let payload_len = usize::from_str_radix(size_str, 8).unwrap_or(0);
        let padded = 512 + ((payload_len + 511) / 512) * 512;
        cursor += padded;
    }
    assert!(names.contains(&"trace_bundle.json".to_string()));
    assert!(
        names.iter().any(|n| n.starts_with("artifacts/art-1")),
        "art-1 must appear as separate tar entry; got names={:?}",
        names
    );
    assert!(
        names.iter().any(|n| n.starts_with("artifacts/art-2")),
        "art-2 must appear as separate tar entry; got names={:?}",
        names
    );
}

#[test]
fn mt165_unknown_trace_id_returns_typed_error() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let other = Uuid::from_u128(0x12345678_9abc_4def_8123_456789abcdef);
    let err = exporter
        .export(other, &RedactionPolicy::default(), ExportFormat::Json)
        .expect_err("unknown trace must error");
    match err {
        ExportError::UnknownTrace { trace_id } => assert_eq!(trace_id, other),
        other => panic!("expected UnknownTrace, got {other:?}"),
    }
}

#[test]
fn mt165_corrupt_bundle_deserialization_typed_error() {
    let garbage: &[u8] = b"this is not json at all { ;";
    let err = deserialize_bundle(garbage).expect_err("garbage bytes must error");
    assert!(matches!(err, ExportError::Deserialization { .. }));
}

#[test]
fn mt165_missing_required_field_rejected_typed() {
    // Valid JSON object, but missing all required TraceBundle fields.
    let partial = br#"{"trace_id":"01938b67-2222-7abc-89ef-0123456789ac"}"#;
    let err = deserialize_bundle(partial).expect_err("partial bundle must error");
    assert!(matches!(err, ExportError::Deserialization { .. }));
}

#[test]
fn mt165_oversize_bundle_typed_too_large_rejection() {
    let src = FixedSource::new(baseline_bundle());
    // Set the cap below the smallest possible serialized bundle so
    // we trip the TooLarge guard predictably.
    let exporter = RetrievalTraceExporter::new(&src).with_max_bytes(16);
    let err = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect_err("must reject oversize bundle");
    match err {
        ExportError::TooLarge {
            actual_bytes,
            limit_bytes,
        } => {
            assert!(actual_bytes > limit_bytes);
            assert_eq!(limit_bytes, 16);
        }
        other => panic!("expected TooLarge, got {other:?}"),
    }
}

#[test]
fn mt165_default_max_bytes_is_one_megabyte() {
    // Defensive: lock the contract minimum-controls ceiling at 1 MiB.
    assert_eq!(MAX_BUNDLE_BYTES, 1_048_576);
}

#[test]
fn mt165_trace_records_mt164_route_decision() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export");
    let decoded: TraceBundle = deserialize_bundle(&exported.bytes).expect("decode");
    assert_eq!(decoded.retrieval_mode, RetrievalMode::Hybrid);
    assert_eq!(decoded.route_decision.matched_rule_id, "general_freeform_query");
    assert!(!decoded.route_decision.rationale.is_empty());
}

#[test]
fn mt165_trace_records_mt161_degradation_report() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export");
    let decoded: TraceBundle = deserialize_bundle(&exported.bytes).expect("decode");
    assert!(decoded
        .degradation_report
        .tiers_completed
        .contains(&TIER_FULL_TEXT.to_string()));
    assert!(decoded
        .degradation_report
        .tiers_completed
        .contains(&TIER_VECTOR.to_string()));
    assert!(decoded
        .degradation_report
        .tiers_skipped
        .contains(&TIER_GRAPH_EXPANSION.to_string()));
    assert!(decoded
        .degradation_report
        .tiers_skipped
        .contains(&TIER_RERANK.to_string()));
    assert_eq!(decoded.degradation_report.tier_chosen, DegradationTier::Tiered);
}

#[test]
fn mt165_trace_records_mt161_scoring_inputs() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export");
    let decoded: TraceBundle = deserialize_bundle(&exported.bytes).expect("decode");
    assert_eq!(decoded.scoring_inputs.len(), 2);
    let s1 = &decoded.scoring_inputs[0];
    assert_eq!(s1.item_id, "item-1");
    assert!((s1.importance - 0.7).abs() < 1e-9);
    assert!((s1.recency - 0.9).abs() < 1e-9);
    assert!((s1.trust - 0.8).abs() < 1e-9);
    assert_eq!(s1.formula_version, "v0");
}

#[test]
fn mt165_default_policy_redacts_pii_and_credentials_in_every_field() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export");
    let decoded: TraceBundle = deserialize_bundle(&exported.bytes).expect("decode");
    // selected_spans
    let span1 = &decoded.selected_spans[0].text;
    assert!(span1.contains("[REDACTED-EMAIL]"));
    assert!(!span1.contains("user@example.com"));
    assert!(!span1.contains("alt@example.org"));
    let span2 = &decoded.selected_spans[1].text;
    assert!(span2.contains("[REDACTED-SECRET]"));
    assert!(!span2.contains("AKIASECRETHERE"));
    // query_plan.query also scanned
    let q = &decoded.query_plan.query;
    assert!(q.contains("[REDACTED-EMAIL]"), "got {q}");
    assert!(q.contains("[REDACTED-SECRET]"), "got {q}");
    assert!(!q.contains("ops@example.com"));
    assert!(!q.contains("token=abc123XYZ"));
    // redactions_applied lists at least one record per redacted field
    // (selected_spans.span-1 pii, selected_spans.span-2 credentials,
    // query_plan.query pii, query_plan.query credentials).
    let categories: Vec<_> = decoded
        .redactions_applied
        .iter()
        .map(|r| (r.field.clone(), r.category.clone()))
        .collect();
    assert!(categories.contains(&("selected_spans.span-1".to_string(), "pii".to_string())));
    assert!(categories.contains(&("selected_spans.span-2".to_string(), "credentials".to_string())));
    assert!(categories.contains(&("query_plan.query".to_string(), "pii".to_string())));
    assert!(categories.contains(&("query_plan.query".to_string(), "credentials".to_string())));
}

#[test]
fn mt165_deterministic_content_hash_across_two_exports() {
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let a = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export 1");
    // Sleep tiny so exported_at_utc differs between exports; if the
    // hash domain leaked exported_at_utc this test would fail.
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export 2");
    assert_eq!(a.content_hash, b.content_hash);
    // Hash is a 64-hex SHA-256 digest.
    assert_eq!(a.content_hash.len(), 64);
    assert!(a.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn mt165_content_hash_changes_when_input_changes() {
    let mut bundle_a = baseline_bundle();
    bundle_a.selected_spans[0].text = "alpha".to_string();
    let mut bundle_b = bundle_a.clone();
    bundle_b.selected_spans[0].text = "beta".to_string();
    assert_ne!(bundle_a.deterministic_hash(), bundle_b.deterministic_hash());
}

#[test]
fn mt165_exported_bundle_is_serde_clean() {
    // Make sure the public ExportedBundle struct itself round-trips
    // (operator UIs serialize this for download).
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let exported = exporter
        .export(FIXED_TRACE_ID, &RedactionPolicy::default(), ExportFormat::Json)
        .expect("export");
    let s = serde_json::to_string(&exported).expect("serialize ExportedBundle");
    let back: ExportedBundle = serde_json::from_str(&s).expect("deserialize ExportedBundle");
    assert_eq!(back.format, ExportFormat::Json);
    assert_eq!(back.content_hash, exported.content_hash);
}

#[test]
fn mt165_artifact_load_failure_propagates_typed() {
    // Custom source that fails on load_artifact to prove typed error
    // path; only fires for Tarball format because Json/JsonPretty do
    // not need artifact bytes.
    struct FailArtifact(TraceBundle);
    impl TraceSource for FailArtifact {
        fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError> {
            if trace_id == self.0.trace_id {
                Ok(self.0.clone())
            } else {
                Err(ExportError::UnknownTrace { trace_id })
            }
        }
        fn load_artifact(&self, artifact_id: &str) -> Result<Vec<u8>, ExportError> {
            Err(ExportError::ArtifactLoad {
                artifact_id: artifact_id.to_string(),
                message: "simulated artifact store failure".to_string(),
            })
        }
    }
    // Use a policy that keeps at least one artifact so load_artifact
    // is invoked.
    let mut bundle = baseline_bundle();
    bundle.referenced_artifacts = vec![ArtifactRef {
        artifact_id: "art-1".to_string(),
        kind: "packet.json".to_string(),
        bytes_estimate: 10,
    }];
    let src = FailArtifact(bundle);
    let exporter = RetrievalTraceExporter::new(&src);
    let err = exporter
        .export(
            FIXED_TRACE_ID,
            &RedactionPolicy::default(),
            ExportFormat::Tarball,
        )
        .expect_err("artifact load must error");
    match err {
        ExportError::ArtifactLoad { artifact_id, .. } => assert_eq!(artifact_id, "art-1"),
        other => panic!("expected ArtifactLoad, got {other:?}"),
    }
}

#[test]
fn mt165_redactions_applied_record_count_matches_categories() {
    // Stress: every category lands at least one record.
    let src = FixedSource::new(baseline_bundle());
    let exporter = RetrievalTraceExporter::new(&src);
    let policy = RedactionPolicy {
        redact_pii: true,
        redact_credentials: true,
        // turning on full_text also logs a record per non-empty span
        redact_full_item_text: true,
        allowlist_artifact_kinds: vec!["packet.json".to_string()],
    };
    let exported = exporter
        .export(FIXED_TRACE_ID, &policy, ExportFormat::Json)
        .expect("export");
    let decoded: TraceBundle = deserialize_bundle(&exported.bytes).expect("decode");
    let cats: std::collections::HashSet<_> = decoded
        .redactions_applied
        .iter()
        .map(|r| r.category.clone())
        .collect();
    assert!(cats.contains("pii"));
    assert!(cats.contains("credentials"));
    assert!(cats.contains("full_text"));
    assert!(cats.contains("allowlist_filter"));
}

#[test]
fn mt165_export_format_serde_round_trip_for_every_variant() {
    for v in [ExportFormat::Json, ExportFormat::JsonPretty, ExportFormat::Tarball] {
        let s = serde_json::to_string(&v).unwrap();
        let back: ExportFormat = serde_json::from_str(&s).unwrap();
        assert_eq!(v, back);
    }
}

#[test]
fn mt165_export_error_serde_round_trip_for_every_variant() {
    let cases: Vec<ExportError> = vec![
        ExportError::UnknownTrace {
            trace_id: FIXED_TRACE_ID,
        },
        ExportError::Serialization {
            message: "boom".to_string(),
        },
        ExportError::Redaction {
            message: "boom".to_string(),
        },
        ExportError::TooLarge {
            actual_bytes: 2_000_000,
            limit_bytes: MAX_BUNDLE_BYTES,
        },
        ExportError::Deserialization {
            message: "boom".to_string(),
        },
        ExportError::ArtifactLoad {
            artifact_id: "art-1".to_string(),
            message: "boom".to_string(),
        },
    ];
    for c in cases {
        let s = serde_json::to_string(&c).unwrap();
        let back: ExportError = serde_json::from_str(&s).unwrap();
        assert_eq!(c, back);
    }
}
