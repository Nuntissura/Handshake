//! Integration proof for the MT-093 DURABLE SURVIVOR STORE (Master Spec v02.196 §6.13.7).
//!
//! Drives the REAL production `palmistry::survivor_store` types (the crate exposes them via its `[lib]`
//! target) — NOT a test-only reimplementation. Proves:
//! - AC-013-1: a freeze/crash survivor record is written with an ATOMIC write and is re-readable after a
//!   simulated Palmistry restart (a fresh `SurvivorStore::open` on the SAME dir reads it back).
//! - AC-013-2: the survivor record carries ONLY typed integers/enums/timestamps + a LOCAL path reference
//!   (the typed-allowlist value scan, §6.13.8).
//! - AC-013-5: an unforwarded record is DRAINED on the next recovery (not lost), and a forwarded record
//!   stays forwarded across a restart (no double-forward — idempotent mark-forwarded, RISK-013-5).
//!
//! BOUNDED-TEST RULE (MT-092 precedent): these tests do NO process spawn, NO IPC, NO socket — they are
//! pure filesystem + in-memory, so they cannot deadlock under the headless harness. (The forward tests
//! that DO touch a local socket are in `test_fr_forward.rs`, hard-bounded there.)

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use palmistry::crash_capture::{CrashDetection, CrashRecord};
use palmistry::freeze_detect::FreezeReport;
use palmistry::survivor_store::{
    assert_typed_allowlist, SurvivorProbeResult, SurvivorRecord, SurvivorRecordKind, SurvivorStore,
};
use handshake_diag_ring::{DiagEventCode, Heartbeat};

/// A unique temp dir for a test (no collisions across parallel tests).
fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hsk-mt093-it-{label}-{}-{nanos}", std::process::id()))
}

struct DirGuard(PathBuf);
impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn freeze_report() -> FreezeReport {
    FreezeReport {
        stale_ms: 7200,
        last_heartbeat_counter: 314,
        last_heartbeat_ts_nanos: 271_828,
    }
}

#[test]
fn freeze_record_is_durable_and_survives_a_palmistry_restart() {
    // AC-013-1: write a FREEZE survivor record, drop the store (simulate Palmistry exiting), then open a
    // FRESH store on the SAME dir and read the record back — it OUTLIVED the process.
    let dir = temp_dir("freeze-durable");
    let _g = DirGuard(dir.clone());

    let written_path = {
        let mut store = SurvivorStore::open(&dir).expect("open store");
        let rec = SurvivorRecord::from_freeze(
            "sess-freeze-durable",
            12345,
            &freeze_report(),
            5,
            SurvivorProbeResult::NotResponding,
        );
        store.put(rec).expect("durable atomic write")
    };
    assert!(written_path.exists(), "the durable record file must exist on disk");

    // Simulated Palmistry RESTART: a brand-new store on the same dir reads the existing record.
    let restarted = SurvivorStore::open(&dir).expect("reopen store after restart");
    assert_eq!(restarted.records().len(), 1, "the freeze record outlived the restart");
    let back = &restarted.records()[0].record;
    assert_eq!(back.kind, SurvivorRecordKind::Freeze);
    assert_eq!(back.session_id, "sess-freeze-durable");
    assert_eq!(back.event_code, DiagEventCode::FreezeSuspected.as_u16());
    assert_eq!(back.stale_ms, 7200);
    assert_eq!(back.last_heartbeat_counter, 314);
    assert_eq!(back.last_event_count, 5);
    assert!(!back.forwarded, "still pending forward after the restart (AC-013-5 drain set)");
}

#[test]
fn crash_record_is_durable_with_local_minidump_path_only() {
    // AC-013-1 + §6.13.8: a CRASH survivor record (built from the MT-092 CrashRecord) is durable and
    // names the minidump as a LOCAL path reference only (never the bytes, never a URL).
    let dir = temp_dir("crash-durable");
    let _g = DirGuard(dir.clone());
    let hb = Heartbeat {
        counter: 88,
        timestamp_nanos: 1_000,
    };
    let dump = dir.join("palmistry-crash-sess-c.dmp");
    let crash = CrashRecord::with_minidump("sess-crash-durable", 999, 21, dump.clone(), Some(hb), &[]);

    {
        let mut store = SurvivorStore::open(&dir).expect("open store");
        store.put(SurvivorRecord::from_crash(&crash)).expect("durable write");
    }
    let restarted = SurvivorStore::open(&dir).expect("reopen store");
    assert_eq!(restarted.records().len(), 1);
    let back = &restarted.records()[0].record;
    assert_eq!(back.kind, SurvivorRecordKind::Crash);
    assert_eq!(back.crash_detection, Some(CrashDetection::CrashContextMinidump));
    assert_eq!(back.faulting_thread_id, Some(21));
    assert_eq!(back.minidump_path.as_deref(), Some(dump.as_path()));
    assert_eq!(back.event_code, DiagEventCode::CrashDetected.as_u16());
    // The minidump path is a LOCAL filesystem path, NOT a URL.
    let p = back.minidump_path.as_ref().unwrap().to_string_lossy();
    assert!(!p.contains("://"), "minidump path must be local, not a URL (§6.13.8)");
}

#[test]
fn survivor_record_is_typed_allowlist_no_free_text() {
    // AC-013-2: a source + value scan of the serialized record asserts ONLY allowlisted typed keys.
    let freeze = SurvivorRecord::from_freeze(
        "sess-allow",
        1,
        &freeze_report(),
        2,
        SurvivorProbeResult::NotResponding,
    );
    assert_typed_allowlist(&freeze).expect("freeze record is typed-allowlist clean");

    // Serialize and confirm there is no free-text / project-content key beyond the allowlist.
    let value = serde_json::to_value(&freeze).unwrap();
    let obj = value.as_object().unwrap();
    let allowed: std::collections::HashSet<&str> =
        SurvivorRecord::allowlisted_keys().iter().copied().collect();
    for key in obj.keys() {
        assert!(
            allowed.contains(key.as_str()),
            "survivor record carried a non-allowlisted key '{key}' (RISK-013-3)"
        );
    }
    // The crash path too.
    let crash = CrashRecord::post_mortem("sess-allow", 1, Some(7), None, &[]);
    assert_typed_allowlist(&SurvivorRecord::from_crash(&crash)).expect("crash record clean");
}

#[test]
fn mark_forwarded_is_idempotent_and_drains_only_pending() {
    // AC-013-5 + RISK-013-5: a freeze + a crash are captured; forwarding only the freeze leaves the crash
    // pending for the next recovery; a forwarded record stays forwarded across a restart (no double-fwd).
    let dir = temp_dir("drain-idem");
    let _g = DirGuard(dir.clone());
    let freeze_path;
    {
        let mut store = SurvivorStore::open(&dir).expect("open store");
        freeze_path = store
            .put(SurvivorRecord::from_freeze(
                "sess-drain",
                1,
                &freeze_report(),
                0,
                SurvivorProbeResult::NotResponding,
            ))
            .unwrap();
        let crash = CrashRecord::post_mortem("sess-drain", 1, Some(1), None, &[]);
        store.put(SurvivorRecord::from_crash(&crash)).unwrap();
        assert_eq!(store.unforwarded().len(), 2, "both pending initially");

        // Forward (mark) only the freeze; the crash stays in the drain set.
        assert!(store.mark_forwarded(&freeze_path).unwrap());
        // Idempotent: a second mark is a harmless no-op.
        assert!(store.mark_forwarded(&freeze_path).unwrap());
        let pending = store.unforwarded();
        assert_eq!(pending.len(), 1, "only the crash remains pending (drain, not lost)");
        assert_eq!(pending[0].record.kind, SurvivorRecordKind::Crash);
    }

    // RESTART: the freeze's forwarded flag persisted (it is NOT re-drained), the crash is still pending.
    let restarted = SurvivorStore::open(&dir).expect("reopen store");
    assert_eq!(
        restarted.unforwarded_count(),
        1,
        "exactly the crash is re-drained after a restart; the forwarded freeze is NOT re-forwarded"
    );
    let pending = restarted.unforwarded();
    assert_eq!(pending[0].record.kind, SurvivorRecordKind::Crash);
}

#[test]
fn atomic_write_leaves_no_temp_debris() {
    // RISK-013-4: after writing, ONLY the final .json record files remain — no .tmp.* debris (the rename
    // moved the temp onto the final path). A leftover temp would mean a non-atomic write path.
    let dir = temp_dir("atomic-debris");
    let _g = DirGuard(dir.clone());
    {
        let mut store = SurvivorStore::open(&dir).expect("open store");
        store
            .put(SurvivorRecord::from_freeze(
                "sess-atomic",
                1,
                &freeze_report(),
                0,
                SurvivorProbeResult::NotResponding,
            ))
            .unwrap();
    }
    let mut json_count = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            !name.contains(".tmp."),
            "an atomic write must leave no temp debris, found '{name}'"
        );
        if name.ends_with(".json") {
            json_count += 1;
        }
    }
    assert_eq!(json_count, 1, "exactly one durable record file remains");
}
