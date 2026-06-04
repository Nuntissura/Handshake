use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use chrono::{TimeZone, Utc};
use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};
use uuid::Uuid;

use handshake_core::hbr::vis_gap::{
    GapClass, Packet, VisGap, HBR_VIS_GAP_RECEIPT_KIND, HBR_VIS_GAP_SCHEMA_VERSION,
};

const EXPECTED_CANONICAL: &str = "{\"emitted_at_utc\":\"2026-05-18T00:00:00Z\",\"evidence_pointer\":\"artifact://visual/diagnostics-canvas.png\",\"gap_class\":\"opaque_canvas\",\"hbr_id\":\"HBR-VIS-005\",\"proposed_followup_wp\":\"WP-KERNEL-004-VIS-GAP-FOLLOWUP-v1\",\"receipt_kind\":\"HBR_VIS_GAP\",\"receipt_uuid\":\"018f6d3a-1f00-7a2b-8c3d-123456789abc\",\"schema_version\":1,\"surface_name\":\"Diagnostics canvas controls\",\"surface_path\":\"app://diagnostics/canvas-controls\",\"wp_id\":\"WP-KERNEL-004-TEST\"}\n";

fn repo_root() -> PathBuf {
    let mut current = std::env::current_dir().expect("current dir");
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}

fn hbr_vis_gap_schema() -> Value {
    let schema_path = repo_root().join(".GOV/roles_shared/schemas/hbr-vis-gap.schema.json");
    serde_json::from_str(&fs::read_to_string(schema_path).expect("read hbr vis gap schema"))
        .expect("parse hbr vis gap schema")
}

fn validate_against_schema(instance: &Value) {
    let schema = hbr_vis_gap_schema();
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema)
        .expect("compile hbr vis gap schema");
    if let Err(errors) = compiled.validate(instance) {
        let messages = errors.map(|error| error.to_string()).collect::<Vec<_>>();
        panic!("schema validation failed: {messages:?}");
    };
}

fn fixture_gap() -> VisGap {
    VisGap {
        receipt_kind: HBR_VIS_GAP_RECEIPT_KIND.to_string(),
        schema_version: HBR_VIS_GAP_SCHEMA_VERSION,
        receipt_uuid: Uuid::parse_str("018f6d3a-1f00-7a2b-8c3d-123456789abc")
            .expect("fixture uuid"),
        wp_id: "WP-KERNEL-004-TEST".to_string(),
        surface_name: "Diagnostics canvas controls".to_string(),
        surface_path: "app://diagnostics/canvas-controls".to_string(),
        gap_class: GapClass::OpaqueCanvas,
        proposed_followup_wp: Some("WP-KERNEL-004-VIS-GAP-FOLLOWUP-v1".to_string()),
        evidence_pointer: Some("artifact://visual/diagnostics-canvas.png".to_string()),
        emitted_at_utc: Utc
            .with_ymd_and_hms(2026, 5, 18, 0, 0, 0)
            .single()
            .expect("fixture timestamp"),
    }
}

#[test]
fn rust_vis_gap_outputs_canonical_jsonl_and_validates_schema() {
    let gap = fixture_gap();
    let canonical = gap.to_canonical_jsonl().expect("canonical vis gap jsonl");

    assert_eq!(canonical, EXPECTED_CANONICAL);
    assert_eq!(gap.receipt_uuid.get_version_num(), 7);
    validate_against_schema(&serde_json::from_str(canonical.trim()).expect("canonical json"));
}

#[test]
fn vis_gap_emit_mutates_packet_open_blockers() {
    let gap = fixture_gap();
    let mut packet: Packet = json!({
        "schema_id": "hsk.work_packet_contract@1",
        "wp_id": "WP-KERNEL-004-TEST",
        "acceptance_matrix": {
            "hbr": [],
            "hbr_not_applicable": []
        }
    });

    let receipt = VisGap::emit(&mut packet, gap).expect("emit vis gap");

    assert_eq!(
        receipt.to_canonical_jsonl().expect("canonical"),
        EXPECTED_CANONICAL
    );
    assert_eq!(
        packet["open_blockers"][0]["blocker_id"],
        "hbr-vis-gap-856b3fceea5a"
    );
    assert_eq!(packet["open_blockers"][0]["blocker_kind"], "HBR_VIS_GAP");
    assert_eq!(packet["open_blockers"][0]["status"], "OPEN");
    assert_eq!(
        packet["open_blockers"][0]["surface_name"],
        "Diagnostics canvas controls"
    );
    assert_eq!(
        packet["open_blockers"][0]["required_action"],
        "Open a follow-up WP for the missing automation hook before PASS closure."
    );
}

#[test]
fn node_normalizer_round_trips_rust_canonical_jsonl() {
    let script_path = repo_root().join(".GOV/roles_shared/scripts/hbr-vis-gap-emit.mjs");
    let canonical = fixture_gap()
        .to_canonical_jsonl()
        .expect("canonical vis gap jsonl");
    let mut child = Command::new("node")
        .arg(script_path)
        .arg("--normalize-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn node hbr vis gap normalizer");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(canonical.as_bytes())
        .expect("write canonical jsonl");

    let output = child.wait_with_output().expect("node output");
    assert!(
        output.status.success(),
        "node normalizer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        canonical
    );
}
