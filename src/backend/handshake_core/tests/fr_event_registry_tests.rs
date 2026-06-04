//! WP-KERNEL-004 cluster X.4 MT-198 FR-EVT-* registry alignment tests.
//!
//! Verifies:
//!  - Every Rust [`FrEventId`] variant appears in the JSON manifest at
//!    `.GOV/roles_shared/records/FR_EVENT_REGISTRY.json` (CI-fail on drift).
//!  - The JSON manifest schema header matches the Rust constant.
//!  - Round-trip enum<->string is total over [`FrEventId::all`].
//!  - Unknown ids return a typed [`UnknownEventId`] error (no panic, no
//!    silent near-miss).
//!  - Case-mismatched and whitespace-padded ids are rejected.
//!  - JSON manifest entries are unique by id (no double-registration).
//!  - Manifest event kinds are confined to the documented vocabulary.
//!
//! Spec-Realism Gate compliance: pure-Rust tests, no `#[ignore]`-gated
//! external services; no `unimplemented!()` paths.

use handshake_core::flight_recorder::fr_event_registry::{
    FrEventId, FrEventRegistry, FR_EVENT_REGISTRY_SCHEMA, FR_EVENT_REGISTRY_VERSION,
};
use std::collections::HashSet;
use std::path::PathBuf;

/// Resolve the on-disk JSON manifest path relative to the crate root.
/// `CARGO_MANIFEST_DIR` is set by cargo to the directory containing
/// `Cargo.toml` for the package under test, which is
/// `src/backend/handshake_core`. The .GOV junction at repo root
/// surfaces the governance file.
fn manifest_path() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crate_dir = .../src/backend/handshake_core
    let repo_root = crate_dir
        .parent() // .../src/backend
        .and_then(|p| p.parent()) // .../src
        .and_then(|p| p.parent()) // .../
        .expect("repo root above src/backend/handshake_core");
    repo_root.join(".GOV/roles_shared/records/FR_EVENT_REGISTRY.json")
}

fn load_disk_manifest() -> FrEventRegistry {
    let path = manifest_path();
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read FR_EVENT_REGISTRY.json at {path:?}: {e}"));
    serde_json::from_str(&body).expect("FR_EVENT_REGISTRY.json must be valid JSON for the schema")
}

#[test]
fn mt_198_disk_manifest_schema_header_matches_rust_constants() {
    let manifest = load_disk_manifest();
    assert_eq!(
        manifest.schema, FR_EVENT_REGISTRY_SCHEMA,
        "schema header must match Rust constant; bump both together"
    );
    assert_eq!(
        manifest.version, FR_EVENT_REGISTRY_VERSION,
        "version header must match Rust constant; bump both together"
    );
}

#[test]
fn mt_198_disk_manifest_aligns_with_rust_enum() {
    let disk = load_disk_manifest();
    let rust = FrEventRegistry::from_rust_enum();

    let disk_ids: HashSet<String> = disk.events.iter().map(|e| e.id.clone()).collect();
    let rust_ids: HashSet<String> = rust.events.iter().map(|e| e.id.clone()).collect();

    let only_in_disk: Vec<_> = disk_ids.difference(&rust_ids).collect();
    let only_in_rust: Vec<_> = rust_ids.difference(&disk_ids).collect();

    assert!(
        only_in_disk.is_empty() && only_in_rust.is_empty(),
        "FR-EVT-* registry drift: only in disk manifest = {only_in_disk:?}; only in Rust enum = {only_in_rust:?}"
    );

    // Per-entry alignment for subsystem, kind, schema_fields.
    let disk_by_id: std::collections::HashMap<&str, &_> =
        disk.events.iter().map(|e| (e.id.as_str(), e)).collect();
    for r in &rust.events {
        let d = disk_by_id
            .get(r.id.as_str())
            .unwrap_or_else(|| panic!("disk manifest missing entry for {}", r.id));
        assert_eq!(d.kind, r.kind, "kind drift for {}", r.id);
        assert_eq!(d.subsystem, r.subsystem, "subsystem drift for {}", r.id);
        assert_eq!(
            d.added_in_wp, r.added_in_wp,
            "added_in_wp drift for {}",
            r.id
        );
        assert_eq!(
            d.schema_fields.len(),
            r.schema_fields.len(),
            "schema_fields count drift for {}",
            r.id
        );
        for (a, b) in d.schema_fields.iter().zip(r.schema_fields.iter()) {
            assert_eq!(a.name, b.name, "schema_field name drift for {}", r.id);
            assert_eq!(a.kind, b.kind, "schema_field kind drift for {}", r.id);
        }
    }
}

#[test]
fn mt_198_round_trip_every_variant() {
    for id in FrEventId::all() {
        let s = id.as_str();
        let back = FrEventId::from_str_id(s).expect("canonical id must round-trip");
        assert_eq!(*id, back, "round-trip failed for {s}");
    }
}

#[test]
fn mt_198_unknown_id_returns_typed_error_not_panic() {
    let r = FrEventId::from_str_id("FR-EVT-DOES-NOT-EXIST");
    assert!(r.is_err());
    let err = r.unwrap_err();
    assert_eq!(err.0, "FR-EVT-DOES-NOT-EXIST");
}

#[test]
fn mt_198_lowercase_id_is_rejected() {
    // Canonical case is UPPER-KEBAB-CASE; lowercase must NOT match a
    // variant (this is the "case-mismatch handling" red-team control).
    assert!(FrEventId::from_str_id("fr-evt-span-started").is_err());
    assert!(FrEventId::from_str_id("Fr-Evt-Span-Started").is_err());
}

#[test]
fn mt_198_whitespace_padding_is_rejected() {
    // Trailing/leading whitespace must NOT round-trip.
    assert!(FrEventId::from_str_id(" FR-EVT-SPAN-STARTED").is_err());
    assert!(FrEventId::from_str_id("FR-EVT-SPAN-STARTED ").is_err());
    assert!(FrEventId::from_str_id("FR-EVT-SPAN-STARTED\n").is_err());
}

#[test]
fn mt_198_typo_near_miss_is_rejected_not_silently_mapped() {
    // The folded WP-1 risk: typo silently maps to a near variant.
    // Asserts exact-match rejection.
    assert!(FrEventId::from_str_id("FR-EVT-MAILBOX-BACKPRESURE").is_err()); // missing S
    assert!(FrEventId::from_str_id("FR-EVT-SPAN-START").is_err()); // missing -ED
}

#[test]
fn mt_198_disk_manifest_no_duplicate_ids() {
    let disk = load_disk_manifest();
    let mut seen: HashSet<&str> = HashSet::new();
    for e in &disk.events {
        assert!(
            seen.insert(e.id.as_str()),
            "duplicate id in disk manifest: {}",
            e.id
        );
    }
}

#[test]
fn mt_198_disk_manifest_kinds_are_within_vocabulary() {
    let disk = load_disk_manifest();
    for e in &disk.events {
        assert!(
            matches!(e.kind.as_str(), "emission" | "span_lifecycle"),
            "unexpected kind for {}: {}",
            e.id,
            e.kind
        );
    }
}

#[test]
fn mt_198_disk_manifest_subsystems_are_within_vocabulary() {
    let disk = load_disk_manifest();
    for e in &disk.events {
        assert!(
            matches!(
                e.subsystem.as_str(),
                "role_mailbox"
                    | "mt_executor"
                    | "session_checkpoint"
                    | "flight_recorder"
                    | "model_runtime"
                    | "distillation"
            ),
            "unexpected subsystem for {}: {}",
            e.id,
            e.subsystem
        );
    }
}

#[test]
fn mt_198_disk_manifest_ids_match_canonical_shape() {
    // Mirror the JSON Schema pattern check in Rust so test failure
    // points to the exact id with a stack trace.
    let disk = load_disk_manifest();
    for e in &disk.events {
        assert!(e.id.starts_with("FR-EVT-"), "id missing prefix: {}", e.id);
        assert!(
            e.id.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'),
            "id contains non-canonical character: {}",
            e.id
        );
    }
}

#[test]
fn mt_198_rust_serialisation_is_canonical_json() {
    // `to_canonical_json` is the source-of-truth serialiser. The disk
    // manifest is hand-curated to match this shape so a future
    // regen-from-Rust pass is byte-stable.
    let r = FrEventRegistry::from_rust_enum();
    let s = r.to_canonical_json();
    assert!(s.ends_with('\n'), "canonical JSON must end with newline");
    // Round-trip self check.
    let parsed: FrEventRegistry = serde_json::from_str(s.trim()).unwrap();
    assert_eq!(parsed.schema, r.schema);
    assert_eq!(parsed.events.len(), r.events.len());
}

#[test]
fn mt_198_disk_manifest_count_matches_rust_count() {
    let disk = load_disk_manifest();
    assert_eq!(
        disk.events.len(),
        FrEventId::all().len(),
        "disk manifest count {} != Rust variant count {}",
        disk.events.len(),
        FrEventId::all().len()
    );
}

#[test]
fn mt_188_fr_evt_mt_015_schema_matches_candidate_payload_contract() {
    let rust_fields: Vec<(&str, &str)> = FrEventId::Mt015DistillationCandidate
        .schema_fields()
        .iter()
        .map(|field| (field.name, field.kind))
        .collect();
    assert_eq!(
        rust_fields,
        vec![
            ("wp_id", "string"),
            ("mt_id", "string"),
            ("candidate_ref", "object")
        ],
        "FR-EVT-MT-015 must describe the payload emitted by MtOutcomeRecorder"
    );

    let disk = load_disk_manifest();
    let entry = disk
        .events
        .iter()
        .find(|event| event.id == "FR-EVT-MT-015")
        .expect("disk manifest must include FR-EVT-MT-015");
    let disk_fields: Vec<(&str, &str)> = entry
        .schema_fields
        .iter()
        .map(|field| (field.name.as_str(), field.kind.as_str()))
        .collect();
    assert_eq!(
        disk_fields, rust_fields,
        "FR-EVT-MT-015 disk manifest must match the Rust registry"
    );
}

#[test]
fn mt_120_fr_evt_distill_pii_detect_schema_matches_content_review_payload_contract() {
    let id = FrEventId::DistillPiiDetect;
    assert_eq!(id.as_str(), "FR-EVT-DISTILL-PII-DETECT");
    assert_eq!(id.kind(), "emission");
    assert_eq!(id.subsystem(), "distillation");

    let rust_fields: Vec<(&str, &str)> = id
        .schema_fields()
        .iter()
        .map(|field| (field.name, field.kind))
        .collect();
    assert_eq!(
        rust_fields,
        vec![
            ("turn_id", "string"),
            ("pii_kinds", "string_array"),
            ("severity", "string")
        ],
        "FR-EVT-DISTILL-PII-DETECT must describe the privacy-preserving content review payload"
    );

    let disk = load_disk_manifest();
    let entry = disk
        .events
        .iter()
        .find(|event| event.id == "FR-EVT-DISTILL-PII-DETECT")
        .expect("disk manifest must include FR-EVT-DISTILL-PII-DETECT");
    let disk_fields: Vec<(&str, &str)> = entry
        .schema_fields
        .iter()
        .map(|field| (field.name.as_str(), field.kind.as_str()))
        .collect();
    assert_eq!(
        disk_fields, rust_fields,
        "FR-EVT-DISTILL-PII-DETECT disk manifest must match the Rust registry"
    );
}

#[test]
fn mt_198_span_lifecycle_variants_present() {
    // MT-198 contract explicitly requires SpanStarted/Ended/Failed.
    // Agent instructions add SpanLifecycleCheckpoint and
    // SpanLifecycleAttachActivity for MT-199 pre-allocation.
    let required: &[FrEventId] = &[
        FrEventId::SpanStarted,
        FrEventId::SpanEnded,
        FrEventId::SpanFailed,
        FrEventId::SpanLifecycleCheckpoint,
        FrEventId::SpanLifecycleAttachActivity,
    ];
    for id in required {
        assert_eq!(id.kind(), "span_lifecycle");
        assert_eq!(id.subsystem(), "flight_recorder");
        let back = FrEventId::from_str_id(id.as_str()).expect("must round-trip");
        assert_eq!(*id, back);
    }
}

#[test]
fn mt_198_mt_198_contract_required_taxonomy_present() {
    // Mirrors the explicit taxonomy listed in
    // .GOV/task_packets/.../MT-198.json `implementation_notes`.
    let mt198_required: &[&str] = &[
        "FR-EVT-MAILBOX-BACKPRESSURE",
        "FR-EVT-MAILBOX-ROUTING-DENIED",
        "FR-EVT-MAILBOX-LEASE-ACQUIRED",
        "FR-EVT-MAILBOX-LEASE-EXPIRED",
        "FR-EVT-MAILBOX-LEASE-TAKEOVER",
        "FR-EVT-MT-CANCEL-REQUESTED",
        "FR-EVT-MT-CANCEL-FORCED",
        "FR-EVT-MT-CANCEL-CLEANUP-FAILED",
        "FR-EVT-MT-STARVED",
        "FR-EVT-MT-EXEC-ERROR",
        "FR-EVT-MT-015",
        "FR-EVT-CHECKPOINT-OVERFLOW",
        "FR-EVT-CHECKPOINT-SHUTDOWN-FORCED",
        "FR-EVT-REPLAY-STARTED",
        "FR-EVT-REPLAY-PROGRESS",
        "FR-EVT-REPLAY-COMPLETED",
        "FR-EVT-REPLAY-FAILED",
        "FR-EVT-RESTART-RESUME-STARTED",
        "FR-EVT-RESTART-RESUME-SESSION-RESUMED",
        "FR-EVT-RESTART-RESUME-SESSION-RECOVERY-FAILED",
        "FR-EVT-RESTART-RESUME-DB-UNAVAILABLE",
        "FR-EVT-RESTART-RESUME-COMPLETED",
        "FR-EVT-SPAN-STARTED",
        "FR-EVT-SPAN-ENDED",
        "FR-EVT-SPAN-FAILED",
    ];
    for s in mt198_required {
        FrEventId::from_str_id(s)
            .unwrap_or_else(|_| panic!("MT-198 required id missing from registry: {s}"));
    }
}
