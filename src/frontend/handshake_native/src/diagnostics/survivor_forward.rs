//! Handshake-side SURVIVOR-FORWARD READER (WP-KERNEL-012 MT-093, Master Spec v02.196 §6.13.7 + §10.12.5).
//!
//! This is the HANDSHAKE-side seam that lets the in-app Diagnostics Panel (MT-087) Tier-3 section show the
//! freeze/crash records the external Palmistry watcher persisted to its DURABLE survivor store. It is the
//! READ side of the §6.13.7 recovery forwarding: Palmistry writes the durable survivor records
//! out-of-process (the `palmistry` crate's `survivor_store`); when Handshake recovers, it can READ those
//! records from the SAME portable per-user data dir and project them into the Tier-3 panel section — so
//! the §10.12.5 Tier-3 surface that MT-087 left as an honest empty-state becomes populated post-recovery
//! (AC-013-6).
//!
//! # Why a SEPARATE reader (not a dependency on the palmistry crate)
//!
//! handshake-native and `palmistry` are SIBLING crates with NO dependency edge between them (the only
//! shared crate is `handshake-diag-ring`, the ring substrate). The durable survivor records are a
//! cross-process FILE contract (JSON in `dirs::data_local_dir()/handshake/palmistry/survivors/`), so the
//! Handshake side reads them as files with a typed-allowlist deserializer that mirrors the survivor record
//! shape. This keeps the two binaries decoupled (the §6.13 lifecycle-inversion stance) while letting the
//! panel project the forwarded evidence.
//!
//! # TYPED ALLOWLIST (HARD, §6.13.8)
//!
//! The view this module exposes ([`PalmistrySurvivorView`]) carries ONLY typed integers / enum tags /
//! numeric timestamps / an opaque session token / a LOCAL minidump path reference — NO project content /
//! free text. A reader that found a record with an unexpected free-text field would be a typed-allowlist
//! breach; the deserializer only reads the known typed fields, so an unknown extra field is ignored (it is
//! never surfaced as content).

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The kind of survived event a forwarded record captures (mirrors the palmistry-side
/// `SurvivorRecordKind`). A small closed enum so the panel renders a typed kind, never a parsed string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PalmistrySurvivorKind {
    /// A confirmed FREEZE (§6.13.5).
    Freeze,
    /// A detected CRASH (§6.13.6).
    Crash,
    /// An unrecognized kind tag (forward-compat: a future palmistry kind this Handshake build does not
    /// know). Rendered as "other" rather than dropped, so nothing is silently lost.
    Other,
}

impl PalmistrySurvivorKind {
    /// A short stable display label for the panel.
    pub fn label(self) -> &'static str {
        match self {
            PalmistrySurvivorKind::Freeze => "Freeze",
            PalmistrySurvivorKind::Crash => "Crash",
            PalmistrySurvivorKind::Other => "Other",
        }
    }

    fn from_tag(tag: &str) -> Self {
        match tag {
            "Freeze" => PalmistrySurvivorKind::Freeze,
            "Crash" => PalmistrySurvivorKind::Crash,
            _ => PalmistrySurvivorKind::Other,
        }
    }
}

/// The typed-allowlist view of one forwarded survivor record the panel renders (§10.12.5 Tier-3). EVERY
/// field is a number, a typed kind, an opaque session token, a numeric timestamp, or a LOCAL path string
/// — NO project content / free text (§6.13.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalmistrySurvivorView {
    /// Freeze or Crash (typed kind).
    pub kind: PalmistrySurvivorKind,
    /// The opaque diagnostic session id (a correlation token — NOT content).
    pub session_id: String,
    /// The watched parent process id (a number).
    pub process_id: u32,
    /// The typed ring event code (FreezeSuspected/CrashDetected as a u16).
    pub event_code: u16,
    /// FREEZE only: how long the heartbeat was stale (ms); 0 for a crash.
    pub stale_ms: u64,
    /// The parent OS exit code, if a crash resolved one (`None` otherwise).
    pub exit_code: Option<u32>,
    /// The LOCAL minidump path string, if a crash wrote one (a local reference — never the bytes).
    pub minidump_path: Option<String>,
    /// Wall-clock millis since the UNIX epoch when the record was captured.
    pub captured_at_unix_ms: u128,
    /// Whether the record has been forwarded to the Flight Recorder (the idempotent flag).
    pub forwarded: bool,
}

/// The typed-allowlist deserializer for a durable survivor record FILE (the palmistry-side JSON). Only the
/// known typed fields are read; the `#[serde(tag = "kind")]` value lands in `kind_tag`. An unknown extra
/// field on disk is IGNORED (never surfaced) — the read side cannot become a content channel.
#[derive(Debug, Deserialize)]
struct SurvivorRecordOnDisk {
    /// The serde tag for the kind enum (`"Freeze"` / `"Crash"`).
    #[serde(rename = "kind", default)]
    kind_tag: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    process_id: u32,
    #[serde(default)]
    event_code: u16,
    #[serde(default)]
    stale_ms: u64,
    #[serde(default)]
    exit_code: Option<u32>,
    #[serde(default)]
    minidump_path: Option<String>,
    #[serde(default)]
    captured_at_unix_ms: u128,
    #[serde(default)]
    forwarded: bool,
}

impl SurvivorRecordOnDisk {
    fn into_view(self) -> PalmistrySurvivorView {
        PalmistrySurvivorView {
            kind: PalmistrySurvivorKind::from_tag(&self.kind_tag),
            session_id: self.session_id,
            process_id: self.process_id,
            event_code: self.event_code,
            stale_ms: self.stale_ms,
            exit_code: self.exit_code,
            // Defensive §6.13.8: a path that looks like a URL is dropped (never surface a non-local ref).
            minidump_path: self.minidump_path.filter(|p| !p.contains("://")),
            captured_at_unix_ms: self.captured_at_unix_ms,
            forwarded: self.forwarded,
        }
    }
}

/// The default Palmistry survivor-store directory on the Handshake side — the SAME portable per-user data
/// dir the watcher writes to (`dirs::data_local_dir()/handshake/palmistry/survivors/`, the MT-083
/// convention). `None` when the platform has no data-local dir.
pub fn default_survivor_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("handshake").join("palmistry").join("survivors"))
}

/// Read all durable survivor records from `dir`, returning the typed-allowlist views NEWEST-FIRST (by
/// capture time). A record file that fails to parse is SKIPPED (a corrupt sidecar must not break the
/// panel). Returns an empty vec if the dir does not exist (the honest empty-state). This is the READ path
/// the Diagnostics Panel Tier-3 section projects (AC-013-6).
pub fn read_survivor_records(dir: &Path) -> Vec<PalmistrySurvivorView> {
    let mut views = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return views; // no dir yet => honest empty-state.
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_record = path.extension().and_then(|e| e.to_str()) == Some("json")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("survivor-"))
                .unwrap_or(false);
        if !is_record {
            continue;
        }
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(rec) = serde_json::from_slice::<SurvivorRecordOnDisk>(&bytes) {
                views.push(rec.into_view());
            }
        }
    }
    // Newest-first so the panel shows the most recent freeze/crash at the top.
    views.sort_by(|a, b| b.captured_at_unix_ms.cmp(&a.captured_at_unix_ms));
    views
}

/// Read the forwarded/known survivor records from the DEFAULT dir (the portable per-user store). A
/// convenience for the shell to build the panel's Tier-3 view each frame; returns empty when the platform
/// has no data-local dir or the store is empty (the honest empty-state MT-087 already renders).
pub fn read_default_survivor_records() -> Vec<PalmistrySurvivorView> {
    match default_survivor_dir() {
        Some(dir) => read_survivor_records(&dir),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hsk-mt093-hsk-{label}-{}-{nanos}", std::process::id()))
    }

    /// Write a survivor record JSON in the palmistry-side shape (the cross-process file contract).
    fn write_record(dir: &Path, file: &str, json: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(file), json).unwrap();
    }

    #[test]
    fn reads_freeze_and_crash_records_typed() {
        let dir = temp_dir("read");
        write_record(
            &dir,
            "survivor-freeze-sess-a.json",
            r#"{"schema_version":"hsk.palmistry.survivor@0.1","kind":"Freeze","session_id":"sess-a",
                "process_id":4242,"event_code":7,"stale_ms":6000,"last_heartbeat_counter":42,
                "last_heartbeat_ts_nanos":123,"last_event_count":3,"probe":"NotResponding",
                "crash_detection":null,"faulting_thread_id":null,"exit_code":null,"minidump_path":null,
                "captured_at_unix_ms":1000,"forwarded":true}"#,
        );
        write_record(
            &dir,
            "survivor-crash-sess-b.json",
            r#"{"schema_version":"hsk.palmistry.survivor@0.1","kind":"Crash","session_id":"sess-b",
                "process_id":7,"event_code":8,"stale_ms":0,"last_heartbeat_counter":9,
                "last_heartbeat_ts_nanos":99,"last_event_count":1,"probe":"NotApplicable",
                "detection":"PostMortemNoContext","faulting_thread_id":0,"exit_code":3221225477,
                "minidump_path":"C:/data/palmistry-crash-sess-b.dmp","captured_at_unix_ms":2000,
                "forwarded":false}"#,
        );
        let _g = scopeguard(dir.clone());

        let views = read_survivor_records(&dir);
        assert_eq!(views.len(), 2, "both records read");
        // Newest-first: the crash (captured_at 2000) is first.
        assert_eq!(views[0].kind, PalmistrySurvivorKind::Crash);
        assert_eq!(views[0].session_id, "sess-b");
        assert_eq!(views[0].exit_code, Some(3221225477));
        assert_eq!(
            views[0].minidump_path.as_deref(),
            Some("C:/data/palmistry-crash-sess-b.dmp")
        );
        assert!(!views[0].forwarded);
        assert_eq!(views[1].kind, PalmistrySurvivorKind::Freeze);
        assert_eq!(views[1].stale_ms, 6000);
        assert!(views[1].forwarded);
    }

    #[test]
    fn missing_dir_is_empty_state_not_error() {
        let dir = temp_dir("missing");
        // Never created.
        assert!(read_survivor_records(&dir).is_empty(), "a missing dir is the honest empty-state");
    }

    #[test]
    fn url_minidump_path_is_dropped_local_only() {
        // §6.13.8: a non-local (URL) minidump path is never surfaced.
        let dir = temp_dir("urlpath");
        write_record(
            &dir,
            "survivor-crash-sess-u.json",
            r#"{"kind":"Crash","session_id":"sess-u","process_id":1,"event_code":8,"stale_ms":0,
                "minidump_path":"https://evil.example/dump","captured_at_unix_ms":5,"forwarded":false}"#,
        );
        let _g = scopeguard(dir.clone());
        let views = read_survivor_records(&dir);
        assert_eq!(views.len(), 1);
        assert!(views[0].minidump_path.is_none(), "a URL minidump path must be dropped (local-only)");
    }

    #[test]
    fn corrupt_record_is_skipped_not_fatal() {
        let dir = temp_dir("corrupt");
        write_record(&dir, "survivor-freeze-good.json", r#"{"kind":"Freeze","session_id":"g","captured_at_unix_ms":1}"#);
        write_record(&dir, "survivor-crash-bad.json", "not json at all {{{");
        let _g = scopeguard(dir.clone());
        let views = read_survivor_records(&dir);
        assert_eq!(views.len(), 1, "the good record is read; the corrupt one is skipped");
        assert_eq!(views[0].session_id, "g");
    }

    struct ScopeGuard(PathBuf);
    impl Drop for ScopeGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn scopeguard(dir: PathBuf) -> ScopeGuard {
        ScopeGuard(dir)
    }
}
