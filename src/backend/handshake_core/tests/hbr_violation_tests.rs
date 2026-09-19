use chrono::{TimeZone, Utc};
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use uuid::Uuid;

use handshake_core::hbr::violation::{
    EvaluationPoint, HbrViolation, HbrViolationRole, ViolationClass, ViolationSink,
};

const EXPECTED_CANONICAL: &str = "{\"emitted_at_utc\":\"2026-05-18T00:00:00Z\",\"evaluation_point\":\"build\",\"evidence_pointer\":\"test://hbr_violation_wire_contract\",\"hbr_id\":\"HBR-INT-001\",\"mt_id\":\"MT-006\",\"notes\":\"wire contract fixture\",\"receipt_kind\":\"HBR_VIOLATION\",\"receipt_uuid\":\"018f6d3a-1f00-7a2b-8c3d-123456789abc\",\"role\":\"KERNEL_BUILDER\",\"schema_version\":1,\"source_session\":\"KERNEL_BUILDER-20260518-012310\",\"violation_class\":\"MISSING_EVIDENCE\",\"wp_id\":\"WP-KERNEL-004-TEST\"}\n";

#[derive(Default)]
struct InMemoryViolationSink {
    rows: std::sync::Mutex<Vec<String>>,
}

impl ViolationSink for InMemoryViolationSink {
    fn write_violation(&self, canonical_jsonl: &str) -> Result<(), std::io::Error> {
        self.rows
            .lock()
            .expect("violation rows lock")
            .push(canonical_jsonl.to_string());
        Ok(())
    }
}

fn fixture_violation() -> HbrViolation {
    HbrViolation {
        receipt_kind: "HBR_VIOLATION".to_string(),
        schema_version: 1,
        receipt_uuid: Uuid::parse_str("018f6d3a-1f00-7a2b-8c3d-123456789abc")
            .expect("fixture uuid"),
        hbr_id: "HBR-INT-001".to_string(),
        wp_id: "WP-KERNEL-004-TEST".to_string(),
        mt_id: Some("MT-006".to_string()),
        role: HbrViolationRole::KernelBuilder,
        evaluation_point: EvaluationPoint::Build,
        evidence_pointer: Some("test://hbr_violation_wire_contract".to_string()),
        violation_class: ViolationClass::MissingEvidence,
        emitted_at_utc: Utc
            .with_ymd_and_hms(2026, 5, 18, 0, 0, 0)
            .single()
            .expect("fixture timestamp"),
        source_session: Some("KERNEL_BUILDER-20260518-012310".to_string()),
        notes: Some("wire contract fixture".to_string()),
    }
}

fn hbr_schema() -> Value {
    serde_json::from_str(include_str!("fixtures/hbr/hbr-violation.schema.json")).expect("product wire schema fixture")
}

fn validate_against_schema(instance: &Value) {
    let schema = hbr_schema();
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema)
        .expect("compile hbr violation schema");
    if let Err(errors) = compiled.validate(instance) {
        let messages = errors.map(|error| error.to_string()).collect::<Vec<_>>();
        panic!("schema validation failed: {messages:?}");
    };
}

#[test]
fn rust_emitter_outputs_canonical_jsonl_and_validates_schema() {
    let violation = fixture_violation();
    let canonical = violation
        .to_canonical_jsonl()
        .expect("canonical violation jsonl");

    assert_eq!(canonical, EXPECTED_CANONICAL);
    assert_eq!(violation.receipt_uuid.get_version_num(), 7);
    validate_against_schema(&serde_json::from_str(canonical.trim()).expect("canonical json"));
}

#[test]
fn violation_sink_receives_canonical_jsonl() {
    let sink = InMemoryViolationSink::default();
    fixture_violation().emit(&sink).expect("emit violation");

    assert_eq!(
        sink.rows.lock().expect("rows lock").as_slice(),
        &[EXPECTED_CANONICAL.to_string()]
    );
}

#[test]
fn all_violation_class_variants_validate_against_schema() {
    for violation_class in [
        ViolationClass::MissingEvidence,
        ViolationClass::EvidenceKindMismatch,
        ViolationClass::EvidenceProofFailed,
        ViolationClass::ApplicabilityMisconfig,
        ViolationClass::DowngradeAttempt,
        ViolationClass::MatrixSchemaViolation,
    ] {
        let mut violation = fixture_violation();
        violation.violation_class = violation_class;
        validate_against_schema(
            &serde_json::from_str(violation.to_canonical_jsonl().expect("jsonl").trim())
                .expect("json"),
        );
    }
}

#[test]
fn builder_mints_v7_receipt_uuid() {
    let violation = HbrViolation::new(
        "HBR-INT-008",
        "WP-KERNEL-004-TEST",
        Some("MT-006"),
        HbrViolationRole::KernelBuilder,
        EvaluationPoint::Build,
        Some("event://uuid-v7-proof"),
        ViolationClass::MissingEvidence,
        Some("KERNEL_BUILDER-20260518-012310"),
        None,
    );

    assert_eq!(violation.receipt_uuid.get_version_num(), 7);
    validate_against_schema(
        &serde_json::from_str(violation.to_canonical_jsonl().expect("jsonl").trim()).expect("json"),
    );
}
