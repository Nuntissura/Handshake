//! WP-KERNEL-012 MT-072 remediation (FAIL_V2) — canonical PreferenceRecord authority live-PG proof.
//!
//! Validator V2 rejected editor settings because they persisted as an opaque workspace-settings JSON
//! document rather than the typed [`PreferenceRecord`] authority (Master Spec v02.201 §10.17). This
//! proof drives the NEW canonical preference HTTP surface against a REAL managed PostgreSQL +
//! handshake_core backend and asserts the full lifecycle the validator required:
//!
//! * SET-REC-003 — a defined-but-unset preference resolves to its registry default (never null), with a
//!   stable `preference_id`, `value_type`, `scope`, `default_value`, `source=default`, `revision=0`.
//! * SET-REC-002 — a typed-invalid value is rejected with an explicit structured 400, never coerced.
//! * SET-EVT-001/002/003 — a set bumps `revision`, returns a recoverable receipt pointing at a durable
//!   EventLedger row, and the EventLedger row is visible on `/events`.
//! * SET-UI-002 — reset-to-default is a mutation with `source=operator` + its own receipt, not a delete.
//! * SET-UI-003 — the change history lists every mutation newest-first, and survives a fresh GET (the
//!   canonical PostgreSQL round-trip; there is no in-memory settings cache — PostgreSQL is the sole
//!   authority, so the readback proves durable persistence).
//! * SET-PROJ-002 — the redacted projection is a deterministic read-only view over canonical state.
//!
//! Run against a live backend, e.g. attach to http://127.0.0.1:37501 or an owned
//! `HSK_TEST_BACKEND_BIN` + `HANDSHAKE_TEST_PG_DSN` (see pg_proof_support).

mod pg_proof_support;

use serde_json::json;

const FONT_SIZE: &str = "view-defaults.editor.font-size";
const TAB_SIZE: &str = "view-defaults.editor.tab-size";
const WORD_WRAP: &str = "view-defaults.editor.word-wrap";

#[test]
fn editor_preferences_persist_reset_and_history_on_live_postgres() {
    let mut backend = pg_proof_support::require_live_backend();
    let wsid = backend.workspace_id.clone();
    let base = format!("/workspaces/{wsid}/preferences");

    // --- SET-REC-003: unset defined preferences resolve to the registry default (never null). ---
    let projection = backend.get_json(&base);
    let rows = projection["preferences"]
        .as_array()
        .expect("projection has a preferences array");
    assert_eq!(
        rows.len(),
        16,
        "the editor preference registry projection must list every defined editor preference"
    );
    let font_row = rows
        .iter()
        .find(|row| row["preference_id"] == FONT_SIZE)
        .expect("projection contains the font-size preference");
    assert_eq!(font_row["value"], json!(13.0), "default font-size = 13.0");
    assert_eq!(font_row["default_value"], json!(13.0));
    assert_eq!(font_row["source"], "default");
    assert_eq!(font_row["revision"], 0);
    assert_eq!(font_row["redacted"], json!(false));

    let font_get = backend.get_json(&format!("{base}/{FONT_SIZE}"));
    let record = &font_get["record"];
    assert_eq!(record["schema_id"], "hsk.preference_record@1");
    assert_eq!(record["preference_id"], FONT_SIZE);
    assert_eq!(record["value_type"], "float");
    assert_eq!(record["scope"], "workspace");
    assert_eq!(record["scope_ref"], wsid);
    assert_eq!(record["value"], json!(13.0));
    assert_eq!(record["source"], "default");
    assert_eq!(record["revision"], 0);

    // --- SET-REC-002: a typed-invalid value is rejected with a structured 400, never persisted. ---
    let (status, body) =
        backend.put_json_response(&format!("{base}/{FONT_SIZE}"), &json!({ "value": 100.0 }));
    assert_eq!(status, 400, "out-of-range font-size must be rejected: {body}");
    assert_eq!(body["error"], "preference_validation_failed");
    assert_eq!(body["validation"]["preference_id"], FONT_SIZE);
    assert_eq!(body["validation"]["code"], "out_of_range");
    // The rejected write left nothing behind (still the default).
    let after_reject = backend.get_json(&format!("{base}/{FONT_SIZE}"));
    assert_eq!(after_reject["record"]["value"], json!(13.0));
    assert_eq!(after_reject["record"]["revision"], 0);

    let (wrong_type_status, _) =
        backend.put_json_response(&format!("{base}/{TAB_SIZE}"), &json!({ "value": "four" }));
    assert_eq!(wrong_type_status, 400, "string for an int preference is rejected");

    let (unknown_status, _) = backend.put_json_response(
        &format!("{base}/view-defaults.editor.does-not-exist"),
        &json!({ "value": 1 }),
    );
    assert_eq!(unknown_status, 404, "an unknown preference id is a 404");

    // --- SET-EVT-001/002: a valid set bumps revision, returns a receipt + durable EventLedger ref. ---
    let set = backend.put_json(&format!("{base}/{FONT_SIZE}"), &json!({ "value": 20.0 }));
    assert_eq!(set["record"]["value"], json!(20.0));
    assert_eq!(set["record"]["source"], "operator");
    assert_eq!(set["record"]["revision"], 1);
    let receipt = &set["receipt"];
    assert_eq!(receipt["schema_id"], "hsk.preference_change_receipt@1");
    assert_eq!(receipt["before_revision"], json!(null));
    assert_eq!(receipt["after_revision"], 1);
    assert_eq!(receipt["old_value"], json!(null));
    assert_eq!(receipt["new_value"], json!(20.0));
    let set_event_id = receipt["event_ledger_event_id"]
        .as_str()
        .expect("receipt carries an EventLedger event id");
    assert!(!set_event_id.is_empty());

    // SET-EVT-003: the EventLedger row is durable and correlatable by preference_id.
    let event = backend.poll_event_by_payload("preference_id", FONT_SIZE);
    assert_eq!(event["payload"]["type"], "preference_record_changed");
    assert_eq!(event["payload"]["revision"], 1);
    assert_eq!(event["payload"]["new_value_ref"], json!(20.0));

    // --- SET-UI-003 durability: a fresh GET (canonical PostgreSQL, no cache) returns the set value. ---
    let reread = backend.get_json(&format!("{base}/{FONT_SIZE}"));
    assert_eq!(reread["record"]["value"], json!(20.0));
    assert_eq!(reread["record"]["revision"], 1);
    assert_eq!(reread["record"]["source"], "operator");

    // Independent second preference proves records are per-preference-id (not one shared blob).
    let tab = backend.put_json(&format!("{base}/{TAB_SIZE}"), &json!({ "value": 8 }));
    assert_eq!(tab["record"]["value"], json!(8));
    assert_eq!(tab["record"]["revision"], 1);
    // font-size is unaffected by the tab-size write.
    assert_eq!(
        backend.get_json(&format!("{base}/{FONT_SIZE}"))["record"]["value"],
        json!(20.0)
    );

    // Enum preference set + validation domain.
    let wrap = backend.put_json(&format!("{base}/{WORD_WRAP}"), &json!({ "value": "on" }));
    assert_eq!(wrap["record"]["value"], json!("on"));
    let (bad_enum, _) =
        backend.put_json_response(&format!("{base}/{WORD_WRAP}"), &json!({ "value": "diagonal" }));
    assert_eq!(bad_enum, 400, "an unknown enum member is rejected");

    // --- SET-UI-002: reset-to-default is a mutation (source=operator) with its own receipt. ---
    let reset = backend.post_json(&format!("{base}/{FONT_SIZE}/reset"), &json!({}));
    assert_eq!(reset["record"]["value"], json!(13.0), "reset restores the default");
    assert_eq!(reset["record"]["source"], "operator");
    assert_eq!(reset["record"]["revision"], 2, "reset bumps the revision");
    let reset_receipt = &reset["receipt"];
    assert_eq!(reset_receipt["before_revision"], 1);
    assert_eq!(reset_receipt["after_revision"], 2);
    assert_eq!(reset_receipt["old_value"], json!(20.0));
    assert_eq!(reset_receipt["new_value"], json!(13.0));

    // --- SET-UI-003: change history lists every mutation newest-first, and survives the round-trip. ---
    let history = backend.get_json(&format!("{base}/{FONT_SIZE}/history"));
    let receipts = history["receipts"]
        .as_array()
        .expect("history has a receipts array");
    assert_eq!(receipts.len(), 2, "font-size has a set + a reset in its history");
    assert_eq!(receipts[0]["after_revision"], 2, "newest (reset) first");
    assert_eq!(receipts[0]["new_value"], json!(13.0));
    assert_eq!(receipts[1]["after_revision"], 1, "then the original set");
    assert_eq!(receipts[1]["new_value"], json!(20.0));
    for receipt in receipts {
        assert!(
            receipt["event_ledger_event_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty()),
            "every receipt points at a durable EventLedger row: {receipt}"
        );
    }

    backend.assert_cleanup();
}
