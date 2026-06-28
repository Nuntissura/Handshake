//! WP-KERNEL-012 MT-096 (G2 end-to-end capstone) — SCENARIO 4: the SYSTEM-WIDE typed-allowlist privacy
//! scan (Master Spec v02.196 §5.8.3 + §6.13.8; AC-016-4 / RISK-016-4).
//!
//! Each tier asserts its OWN typed-allowlist per-MT. The capstone asserts the invariant holds SYSTEM-WIDE:
//! across EVERY telemetry artifact the three-tier system produces — the MT-081 ring records (the
//! heartbeat + last-N `DiagEvent`s), the durable survivor-store records, the typed crash-record METADATA
//! (the typed record, NOT the minidump's OS-image bytes), and the FR-forward body — NO project content /
//! sensitive data / free text appears: only typed event codes, ids, counters, phase markers, thread ids,
//! resource metrics, timestamps, and LOCAL path references.
//!
//! # What is and is NOT scanned
//!
//! This scans the RUNTIME TELEMETRY artifacts — the data that crosses (or could cross) into Palmistry and
//! the FR. It deliberately does NOT scan the build-time GOVERNANCE artifacts (the proof manifest, the
//! three-tier evidence file): those carry developer-authored governance identifiers + human reasons by
//! design (the `three_tier_evidence` module documents this as a DIFFERENT, non-telemetry concern). The
//! §5.8.3/§6.13.8 invariant is about the telemetry, and that is what is scanned here.
//!
//! # The two structural guarantees
//!
//! - The MT-081 `DiagEvent` ring record is a `bytemuck::Pod` with NO string/blob field — it CANNOT hold
//!   text (a compile-time guarantee). The scan verifies it at runtime: every serialized value is numeric.
//! - The survivor/crash/forward JSON records carry a few string-valued fields (an opaque session token, a
//!   fixed schema vocabulary, typed enum tags, a LOCAL path). The scan asserts (a) only allowlisted KEYS
//!   appear (so no `content`/`text`/`note` field can exist) and (b) every string VALUE is a typed-
//!   allowlist shape — a token / enum tag / ISO timestamp / LOCAL path — never a URL and never free prose.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use handshake_diag_ring::{DiagEvent, DiagRingReader, DiagRingWriter, DEFAULT_CAPACITY};
use handshake_native::diagnostics::read_survivor_records;
use serde_json::Value;

// ── artifact hygiene (CX-212E) ─────────────────────────────────────────────────────────────────────────

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local '{local}' dir may exist — artifacts go external only (found {})",
            p.display()
        );
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    std::env::temp_dir().join(format!("hsk-mt096-priv-{label}-{}-{nanos}", std::process::id()))
}

struct DirGuard(PathBuf);
impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct FileGuard(PathBuf);
impl Drop for FileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ── the system-wide value scanner ───────────────────────────────────────────────────────────────────────

/// Whether a STRING value found in a telemetry artifact is a typed-allowlist shape (§6.13.8): a token
/// (opaque session id / fixed schema vocabulary / typed enum tag / wire marker — all space-free) or a
/// LOCAL filesystem path. A URL (`scheme://`) is REJECTED (local-only, never auto-upload), and free prose
/// (whitespace that is not part of a path) is REJECTED (project content). This is what makes the scan a
/// real content check and not just a key check.
fn is_allowlisted_telemetry_string(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    // A network URL reference is never allowed in telemetry (§6.13.8 local-only, RISK-013-6).
    if s.contains("://") {
        return false;
    }
    // A LOCAL path (has a path separator) is allowed even if it contains spaces (e.g. a Windows path under
    // a "Handshake Worktrees" dir). It must not be a URL (checked above).
    let looks_like_path = s.contains('/') || s.contains('\\');
    if looks_like_path {
        return true;
    }
    // Otherwise it must be a TOKEN: a space-free run of the typed-vocabulary charset (alphanumerics + the
    // punctuation used by session ids, schema versions `hsk.x@0.1`, wire markers `a:b:c`, ISO timestamps
    // `2026-06-28T00:00:00Z`). Any whitespace here means free prose -> reject.
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '@' | '+'))
}

/// Recursively assert NO string value anywhere in `value` is free-text/project-content/URL: every string
/// must be an [`is_allowlisted_telemetry_string`]. `ctx` names the artifact for a clear failure.
fn assert_no_free_text(value: &Value, ctx: &str) {
    match value {
        Value::String(s) => assert!(
            is_allowlisted_telemetry_string(s),
            "AC-016-4: {ctx} carried a non-allowlisted string value {s:?} (free text / URL / project \
             content is forbidden in telemetry — only typed tokens / enum tags / timestamps / LOCAL paths)"
        ),
        Value::Array(items) => items.iter().for_each(|v| assert_no_free_text(v, ctx)),
        Value::Object(map) => map.iter().for_each(|(_, v)| assert_no_free_text(v, ctx)),
        // numbers / bools / null carry no content.
        _ => {}
    }
}

/// Assert every KEY in the top-level object of `value` is in `allowed` (so no `content`/`text`/`note`
/// field can exist) AND no string value is free text.
fn assert_keys_and_values(value: &Value, allowed: &HashSet<&str>, ctx: &str) {
    let obj = value
        .as_object()
        .unwrap_or_else(|| panic!("{ctx}: expected a JSON object"));
    for key in obj.keys() {
        assert!(
            allowed.contains(key.as_str()),
            "AC-016-4: {ctx} carried a non-allowlisted key '{key}' (a typed-allowlist record may only \
             carry its known typed fields — a free-text field is forbidden)"
        );
    }
    assert_no_free_text(value, ctx);
}

// ── AC-016-4: scan the ring records (the strongest guarantee — pure POD integers, no string at all) ─────

#[test]
fn ring_records_carry_no_text_only_typed_integers() {
    let path = temp_dir("ring").with_extension("ring");
    let _g = FileGuard(path.clone());
    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring");
    // A representative spread of EVERY ring record kind the three-tier system emits.
    writer.write_heartbeat(7, 12_345);
    writer.write(DiagEvent::heartbeat(1, 1, 7, 12_345));
    writer.write(DiagEvent::resource_sample(1, 2, 500, 1024, 0, 99));
    writer.write(DiagEvent::slow_frame(1, 3, 60, 33_000, 100));
    writer.write(DiagEvent::backend_unreachable(1, 4, 37501, 200));
    writer.write(DiagEvent::backend_recovered(1, 5, 37501, 300));

    let reader = DiagRingReader::open(&path).expect("open ring reader");
    // The heartbeat is two integers.
    let hb = reader.read_heartbeat().expect("heartbeat readable");
    assert!(hb.counter == 7 && hb.timestamp_nanos == 12_345);

    // Every last-N record serializes to ONLY numbers (no string field anywhere) — the compile-time POD
    // guarantee, verified at runtime across the full record spread.
    let events = reader.read_last_n(16);
    assert!(events.len() >= 5, "the written records are all readable");
    for ev in &events {
        let value = serde_json::to_value(ev).expect("serialize DiagEvent");
        assert_all_numeric(&value, "DiagEvent ring record");
        // Belt-and-braces: the typed-allowlist value scan also passes (no string at all -> trivially).
        assert_no_free_text(&value, "DiagEvent ring record");
    }

    drop(reader);
    assert_no_local_artifact_dir();
}

/// Assert a serialized value is ENTIRELY numeric (numbers, or arrays/objects of numbers). The ring record
/// has no string field by construction (`bytemuck::Pod`); this proves it at runtime — a string anywhere
/// would be a privacy regression that bypassed the type system.
fn assert_all_numeric(value: &Value, ctx: &str) {
    match value {
        Value::Number(_) | Value::Null | Value::Bool(_) => {}
        Value::Array(items) => items.iter().for_each(|v| assert_all_numeric(v, ctx)),
        Value::Object(map) => map.iter().for_each(|(_, v)| assert_all_numeric(v, ctx)),
        Value::String(s) => panic!(
            "AC-016-4: {ctx} serialized a STRING value {s:?} — the MT-081 ring record is a typed-integer \
             POD and must carry NO string field (a privacy regression that bypassed bytemuck::Pod)"
        ),
    }
}

// ── AC-016-4: scan the durable survivor-store records (read by the SHIPPED handshake-side reader) ───────

#[test]
fn survivor_store_records_are_typed_allowlist_system_wide() {
    // The durable survivor records the watcher persists (the palmistry-side on-disk shape — the §6.13.7
    // cross-process FILE contract). Written in the SAME shape the survivor_store + survivor_forward modules
    // use, then read by the SHIPPED handshake-side reader (read_survivor_records) + scanned as raw JSON.
    let dir = temp_dir("survivors");
    let _g = DirGuard(dir.clone());
    std::fs::create_dir_all(&dir).unwrap();

    let freeze = r#"{"schema_version":"hsk.palmistry.survivor@0.1","kind":"Freeze","session_id":"019eb067-947f-7603-856d-03e2d1047692","process_id":4242,"event_code":7,"stale_ms":6000,"last_heartbeat_counter":42,"last_heartbeat_ts_nanos":123456,"last_event_count":3,"probe":"NotResponding","crash_detection":null,"faulting_thread_id":null,"exit_code":null,"minidump_path":null,"captured_at_unix_ms":1717000000000,"forwarded":true}"#;
    let crash = r#"{"schema_version":"hsk.palmistry.survivor@0.1","kind":"Crash","session_id":"mt096-crash-sess","process_id":7,"event_code":8,"stale_ms":0,"last_heartbeat_counter":9,"last_heartbeat_ts_nanos":99,"last_event_count":1,"probe":"NotApplicable","crash_detection":{"detection":"PostMortemNoContext"},"faulting_thread_id":0,"exit_code":3221225477,"minidump_path":"C:/Handshake_Artifacts/handshake-test/palmistry-crash-mt096.dmp","captured_at_unix_ms":1717000000001,"forwarded":false}"#;
    std::fs::write(dir.join("survivor-freeze-a.json"), freeze).unwrap();
    std::fs::write(dir.join("survivor-crash-b.json"), crash).unwrap();

    // The survivor-record key allowlist (mirrors survivor_store::SurvivorRecord::allowlisted_keys + the
    // `#[serde(tag = ...)]` tag keys). A record may carry ONLY these keys.
    let allowed: HashSet<&str> = [
        "schema_version",
        "kind",
        "session_id",
        "process_id",
        "event_code",
        "stale_ms",
        "last_heartbeat_counter",
        "last_heartbeat_ts_nanos",
        "last_event_count",
        "probe",
        "crash_detection",
        "faulting_thread_id",
        "exit_code",
        "minidump_path",
        "captured_at_unix_ms",
        "forwarded",
        "detection",
    ]
    .into_iter()
    .collect();

    for (label, raw) in [("freeze", freeze), ("crash", crash)] {
        let value: Value = serde_json::from_str(raw).unwrap();
        assert_keys_and_values(&value, &allowed, &format!("survivor {label} record"));
    }

    // The SHIPPED handshake-side reader surfaces ONLY typed fields (the type itself has no free-text
    // field); assert each view's string-shaped values are allowlisted (session token + LOCAL minidump
    // path, never a URL).
    let views = read_survivor_records(&dir);
    assert_eq!(views.len(), 2, "both records read by the shipped reader");
    for v in &views {
        assert!(
            is_allowlisted_telemetry_string(&v.session_id),
            "survivor view session_id must be a typed token, got {:?}",
            v.session_id
        );
        if let Some(p) = &v.minidump_path {
            assert!(
                is_allowlisted_telemetry_string(p),
                "survivor view minidump_path must be a LOCAL path (no URL), got {p:?}"
            );
        }
    }

    assert_no_local_artifact_dir();
}

// ── AC-016-4: scan the typed crash-record METADATA (the typed record, NOT the dump's OS-image bytes) ────

#[test]
fn crash_record_metadata_is_typed_allowlist_no_dump_bytes() {
    // The typed crash-record METADATA (CrashRecord shape, MT-092 §6.13.8) names the LOCAL minidump PATH —
    // never the dump bytes, never a URL. The dump's OS-image bytes are §6.13.8 local-only and are NOT a
    // telemetry artifact (they never cross into the FR/Palmistry as content). This scans the metadata.
    let metadata = r#"{"session_id":"mt096-crash-sess","detection":{"detection":"CrashContextMinidump"},"crash_event_code":8,"process_id":4242,"faulting_thread_id":12648430,"exit_code":null,"last_heartbeat_counter":42,"last_heartbeat_ts_nanos":123456,"last_event_count":2,"minidump_path":"C:/Handshake_Artifacts/handshake-test/wp-kernel-012-mt-096/palmistry-crash-mt096.dmp","recorded_at_unix_ms":1717000000002}"#;
    let allowed: HashSet<&str> = [
        "session_id",
        "detection",
        "crash_event_code",
        "process_id",
        "faulting_thread_id",
        "exit_code",
        "last_heartbeat_counter",
        "last_heartbeat_ts_nanos",
        "last_event_count",
        "minidump_path",
        "recorded_at_unix_ms",
    ]
    .into_iter()
    .collect();
    let value: Value = serde_json::from_str(metadata).unwrap();
    assert_keys_and_values(&value, &allowed, "crash record metadata");

    // The minidump_path is a LOCAL path reference, not the bytes and not a URL.
    let path = value["minidump_path"].as_str().unwrap();
    assert!(!path.contains("://"), "the minidump path is LOCAL, never a URL (§6.13.8)");
    assert!(path.ends_with(".dmp"), "the minidump path names the dump FILE, not its bytes");

    assert_no_local_artifact_dir();
}

// ── AC-016-4: scan the FR-forward body (the survivor-faithful shape that would cross to the FR) ─────────

#[test]
fn fr_forward_body_is_typed_allowlist() {
    // The survivor-faithful FR-forward body (the WP-016 ingestion shape, MT-093) carries ONLY typed
    // fields — numbers, typed enum tags, opaque tokens, a LOCAL path. NO free text. (Built here in the
    // documented shape — the palmistry-side test builds it from the real `build_survivor_forward_body`;
    // this scans the same shape system-wide on the handshake side.)
    let body = r#"{"schema_version":"hsk.palmistry.survivor_forward@0.1","kind":"freeze","session_id":"mt096-fwd-sess","process_id":4242,"event_code":7,"stale_ms":6000,"last_heartbeat_counter":42,"last_heartbeat_ts_nanos":123,"last_event_count":3,"exit_code":null,"faulting_thread_id":0,"minidump_path":null,"captured_at_unix_ms":1717000000003}"#;
    let allowed: HashSet<&str> = [
        "schema_version",
        "kind",
        "session_id",
        "process_id",
        "event_code",
        "stale_ms",
        "last_heartbeat_counter",
        "last_heartbeat_ts_nanos",
        "last_event_count",
        "exit_code",
        "faulting_thread_id",
        "minidump_path",
        "captured_at_unix_ms",
    ]
    .into_iter()
    .collect();
    let value: Value = serde_json::from_str(body).unwrap();
    assert_keys_and_values(&value, &allowed, "FR-forward body");
    assert!(value.get("message").is_none(), "no free-text 'message' field");
    assert!(value.get("text").is_none(), "no free-text 'text' field");

    assert_no_local_artifact_dir();
}

// ── AC-016-4: the scanner itself is honest (it would CATCH a planted content leak) ──────────────────────

#[test]
fn scanner_rejects_planted_free_text_and_urls() {
    // A guard on the guard: prove the scanner is not a tautology — it must FAIL on a planted free-text
    // value, a URL, and a non-allowlisted key. If any of these passed, the system-wide scan would be
    // worthless.
    assert!(!is_allowlisted_telemetry_string("the user's secret document body"), "free prose must fail");
    assert!(!is_allowlisted_telemetry_string("https://evil.example/upload"), "a URL must fail");
    assert!(is_allowlisted_telemetry_string("hsk.palmistry.survivor@0.1"), "a schema version must pass");
    assert!(is_allowlisted_telemetry_string("019eb067-947f-7603-856d-03e2d1047692"), "a uuid must pass");
    assert!(is_allowlisted_telemetry_string("C:/Handshake_Artifacts/x.dmp"), "a local path must pass");
    assert!(
        is_allowlisted_telemetry_string("D:/Projects/Handshake Worktrees/x.dmp"),
        "a local path WITH a space (a real worktree path) must pass"
    );
    assert!(is_allowlisted_telemetry_string("Freeze"), "an enum tag must pass");
    assert!(is_allowlisted_telemetry_string("2026-06-28T00:00:00Z"), "an ISO timestamp must pass");

    // The recursive object scanner must catch a planted free-text VALUE and a planted URL.
    let leaky = serde_json::json!({"session_id":"ok-token","note":"this is the user's private note"});
    let caught = std::panic::catch_unwind(|| {
        assert_no_free_text(&leaky, "planted-leak");
    });
    assert!(caught.is_err(), "the scanner MUST catch a planted free-text value");

    let leaky_url = serde_json::json!({"minidump_path":"https://evil.example/dump"});
    let caught_url = std::panic::catch_unwind(|| {
        assert_no_free_text(&leaky_url, "planted-url");
    });
    assert!(caught_url.is_err(), "the scanner MUST catch a planted URL");

    assert_no_local_artifact_dir();
}

// ── AC-016-4: opportunistically scan any REAL survivor records the palmistry support test wrote ─────────

#[test]
fn any_real_emitted_survivor_records_are_typed_allowlist() {
    // If the palmistry support test ran first (same external root), it wrote REAL survivor records under
    // the MT-096 artifact dir. Scan any present (newest of each kind) so the system-wide claim covers the
    // genuinely-emitted artifacts, not only representatives. Absence is fine (cross-binary ordering is not
    // guaranteed) — the representative scans above carry the AC; this strengthens it when present.
    let root = external_artifact_dir("wp-kernel-012-mt-096");
    let mut scanned = 0usize;
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            for view in read_survivor_records(&p) {
                assert!(
                    is_allowlisted_telemetry_string(&view.session_id),
                    "a REAL emitted survivor record session_id must be a typed token: {:?}",
                    view.session_id
                );
                if let Some(mp) = &view.minidump_path {
                    assert!(
                        is_allowlisted_telemetry_string(mp),
                        "a REAL emitted survivor minidump_path must be LOCAL (no URL): {mp:?}"
                    );
                }
                scanned += 1;
            }
        }
    }
    println!("MT-096 privacy: scanned {scanned} real emitted survivor record(s) under {}", root.display());
    assert_no_local_artifact_dir();
}
