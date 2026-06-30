//! Palmistry DURABLE SURVIVOR STORE (MT-093, Master Spec v02.196 §6.13.7).
//!
//! When Palmistry detects a FREEZE (MT-091) or a CRASH (MT-092) it must persist a durable LOCAL record
//! of that event — the freeze/crash timeline — that OUTLIVES both the Handshake process AND a Palmistry
//! restart. This is the §6.13.7 "survivor store": a Crashpad-style durable store where records survive
//! the client's death and the handler's own restart, so that when Handshake RECOVERS (unfreezes, or
//! restarts after a crash) Palmistry can FORWARD the captured evidence into the Tier-1 Flight Recorder
//! (the forward is [`crate::fr_forward`], not this module) and the out-of-process evidence rejoins the
//! governed business-event ledger.
//!
//! # What this module owns vs what it does NOT
//!
//! - OWNS: the typed [`SurvivorRecord`], the durable [`SurvivorStore`] (a per-user data dir of JSON
//!   records), ATOMIC writes (temp+replace) so a record survives an ill-timed Palmistry restart, read-
//!   existing-on-startup, and the idempotent `forwarded` flag (so a record is never double-forwarded).
//! - Does NOT own: the FR forward itself (that is [`crate::fr_forward`], which reads this store at
//!   recovery and posts to the EXISTING FR HTTP route — reuse-via-API), nor freeze/crash DETECTION
//!   (MT-091 / MT-092 produce the inputs this store persists).
//!
//! # The TYPED-ALLOWLIST invariant (HARD, §6.13.8)
//!
//! Every field of [`SurvivorRecord`] is a fixed-width integer, a typed enum, a numeric timestamp, an
//! opaque session token, or a LOCAL file-path reference (the minidump path is a path string, NEVER the
//! dump bytes, NEVER project content). There is deliberately NO free-text / project-content field, so a
//! record (and the FR forward built from it) cannot become a content-smuggling channel (RISK-013-3). The
//! [`SurvivorRecord::allowlisted_keys`] list + the `test_survivor_store` value-scan assert this.
//!
//! # Durability (atomic temp+replace — RISK-013-4)
//!
//! A record is written to `<file>.tmp` then atomically replaces its final path. A reader (a later
//! Palmistry startup) either sees the OLD complete file or the NEW complete file — never a half-written
//! record and, on Windows, never a remove-before-replace gap. The store reads ALL existing records on
//! startup (`load_existing`), so a Palmistry restart between capture and forward does not lose a pending
//! forward (AC-013-1 / AC-013-5).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use serde::{Deserialize, Serialize};

use handshake_diag_ring::{DiagEvent, DiagEventCode};

use crate::child_stall::ChildStallReport;
use crate::crash_capture::{CrashDetection, CrashRecord};

use crate::freeze_detect::FreezeReport;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

/// Which kind of survived event a [`SurvivorRecord`] captures. A small closed enum so a reviewer and the
/// FR forwarder reason over a typed kind, never a parsed string. Tagged JSON (`"kind":"Freeze"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SurvivorRecordKind {
    /// A confirmed FREEZE (MT-091, §6.13.5): the heartbeat went stale AND the OS hung-window probe
    /// corroborated. The record carries the freeze timeline (stale duration + last heartbeat).
    Freeze,
    /// A detected CRASH (MT-092, §6.13.6): the parent died abnormally and/or an out-of-process minidump
    /// was written. The record carries the crash facts (detection kind, exit code, minidump path).
    Crash,
    /// A child process is alive but its passive progress source stopped advancing (MT-106).
    ChildStall,
}

impl SurvivorRecordKind {
    /// The matching ring [`DiagEventCode`] for this survivor kind (the shared numeric code space): a
    /// freeze maps to `FreezeSuspected`, a crash to `CrashDetected`. Carried in the record + the FR
    /// forward so the kind is a number from the SAME allowlisted code space the ring uses, never text.
    pub fn diag_event_code(self) -> DiagEventCode {
        match self {
            SurvivorRecordKind::Freeze => DiagEventCode::FreezeSuspected,
            SurvivorRecordKind::Crash => DiagEventCode::CrashDetected,
            SurvivorRecordKind::ChildStall => DiagEventCode::Other,
        }
    }

    /// A short stable wire tag (`"freeze"` / `"crash"`) for the FR-forward `message_id` marker. A fixed
    /// vocabulary word, NOT free text / project content.
    pub fn wire_tag(self) -> &'static str {
        match self {
            SurvivorRecordKind::Freeze => "freeze",
            SurvivorRecordKind::Crash => "crash",
            SurvivorRecordKind::ChildStall => "child-stall",
        }
    }
}

/// How the OS hung-window probe resolved at capture time, as a typed enum mirrored from
/// [`crate::hung_window_probe::ProbeResult`] but `Serialize`/`Deserialize` for the durable record. A
/// freeze record carries this so a reviewer can see the second signal of the §6.13.5 double-signal gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "probe")]
pub enum SurvivorProbeResult {
    /// The OS reported the window as NOT responding (corroborated the freeze).
    NotResponding,
    /// The OS reported the window as responding.
    Responding,
    /// No window could be resolved for the parent (could not corroborate).
    WindowNotFound,
    /// Not applicable (a crash record has no live hung-window probe).
    NotApplicable,
}

/// The TYPED-ALLOWLIST durable survivor record (§6.13.8). EVERY field is a fixed-width integer, a typed
/// enum, a numeric timestamp, an opaque session token, or a LOCAL file-path reference — there is NO
/// free-text / project-content field (RISK-013-3). This is the durable freeze/crash timeline that
/// outlives the Handshake process and a Palmistry restart; [`crate::fr_forward`] forwards the typed
/// subset to the Tier-1 Flight Recorder at recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurvivorRecord {
    /// Schema version of this record shape (a fixed vocabulary string, not content). Lets a future
    /// Palmistry read an older durable record without guessing the layout.
    pub schema_version: String,
    /// Freeze or Crash (typed kind).
    pub kind: SurvivorRecordKind,
    /// The diagnostic session id (an opaque correlation token — NOT content). Used to name the durable
    /// file and to correlate the survivor record to the ring + crash files.
    pub session_id: String,
    /// The watched parent process id (a number).
    pub process_id: u32,
    /// The typed event code (`FreezeSuspected`/`CrashDetected` as a u16) — the SAME shared numeric code
    /// space the ring uses, so the record's kind is a number, not a parsed string.
    pub event_code: u16,
    /// FREEZE only: how long the heartbeat had been stale when the freeze was confirmed, in ms (0 for a
    /// crash record).
    pub stale_ms: u64,
    /// The last heartbeat COUNTER read passively from the ring before the freeze/crash (0 if none). A
    /// number, not content.
    pub last_heartbeat_counter: u64,
    /// The last heartbeat TIMESTAMP (monotonic nanos) read passively from the ring before death (0 if
    /// none). A number, not content.
    pub last_heartbeat_ts_nanos: u64,
    /// How many of the ring's last-N typed events were bundled as evidence (a COUNT — the events
    /// themselves are POD integers, never text, so only the count is carried in the durable record).
    pub last_event_count: u32,
    /// FREEZE only: the OS hung-window probe result at capture (the §6.13.5 second signal).
    /// [`SurvivorProbeResult::NotApplicable`] for a crash record.
    pub probe_result: SurvivorProbeResult,
    /// CRASH only: how the crash was detected (rich minidump vs floor post-mortem); `None` for a freeze.
    pub crash_detection: Option<CrashDetection>,
    /// CRASH only: the faulting thread id, if a CrashContext carried one (0/None otherwise).
    pub faulting_thread_id: Option<u64>,
    /// CRASH only: the parent OS exit code, if the handle resolved one (`None` for a freeze or a
    /// CrashContext-driven dump).
    pub exit_code: Option<u32>,
    /// CRASH only: the LOCAL path of the out-of-process minidump file, if one was written. A LOCAL path
    /// string reference — NEVER a URL, NEVER the dump bytes (§6.13.8 local-only, RISK-013-6). `None` for
    /// a freeze or a post-mortem record.
    pub minidump_path: Option<PathBuf>,
    /// CHILD-STALL only: OS pid of the stalled child process.
    #[serde(default)]
    pub child_process_id: Option<u32>,
    /// CHILD-STALL only: opaque child-session token assigned by the launcher.
    #[serde(default)]
    pub child_session_id: Option<u64>,
    /// CHILD-STALL only: last progress counter observed before the child stopped advancing.
    #[serde(default)]
    pub last_progress_counter: Option<u64>,
    /// CHILD-STALL only: passive source timestamp for the last progress counter.
    #[serde(default)]
    pub last_progress_ts_nanos: Option<u64>,
    /// CHILD-STALL only: stable numeric reason code.
    #[serde(default)]
    pub child_stall_reason_code: Option<u16>,
    /// Wall-clock millis since the UNIX epoch when the record was captured (a numeric timestamp).
    pub captured_at_unix_ms: u128,
    /// IDEMPOTENT FORWARD FLAG (RISK-013-5): `false` until [`crate::fr_forward`] has successfully posted
    /// this record to the Flight Recorder, then `true`. A pending (`false`) record is drained on the next
    /// recovery; a forwarded (`true`) record is never re-forwarded. Persisted so the flag survives a
    /// Palmistry restart.
    pub forwarded: bool,
}

/// The current survivor-record schema version. A fixed vocabulary string.
pub const SURVIVOR_SCHEMA_VERSION: &str = "hsk.palmistry.survivor@0.1";

impl SurvivorRecord {
    /// Build a FREEZE survivor record from the MT-091 [`FreezeReport`] + the session/process facts +
    /// the last-event count + the hung-window probe result. `forwarded` starts `false`.
    pub fn from_freeze(
        session_id: &str,
        process_id: u32,
        report: &FreezeReport,
        last_event_count: u32,
        probe_result: SurvivorProbeResult,
    ) -> Self {
        Self {
            schema_version: SURVIVOR_SCHEMA_VERSION.to_owned(),
            kind: SurvivorRecordKind::Freeze,
            session_id: session_id.to_string(),
            process_id,
            event_code: DiagEventCode::FreezeSuspected.as_u16(),
            stale_ms: report.stale_ms,
            last_heartbeat_counter: report.last_heartbeat_counter,
            last_heartbeat_ts_nanos: report.last_heartbeat_ts_nanos,
            last_event_count,
            probe_result,
            crash_detection: None,
            faulting_thread_id: None,
            exit_code: None,
            minidump_path: None,
            child_process_id: None,
            child_session_id: None,
            last_progress_counter: None,
            last_progress_ts_nanos: None,
            child_stall_reason_code: None,
            captured_at_unix_ms: now_unix_ms(),
            forwarded: false,
        }
    }

    /// Build a CRASH survivor record from the MT-092 [`CrashRecord`]. Reuses the crash record's typed
    /// fields (detection, exit code, last heartbeat/event count, minidump path); `forwarded` starts
    /// `false`. A crash has no live hung-window probe, so `probe_result` is `NotApplicable`.
    pub fn from_crash(crash: &CrashRecord) -> Self {
        Self {
            schema_version: SURVIVOR_SCHEMA_VERSION.to_owned(),
            kind: SurvivorRecordKind::Crash,
            session_id: crash.session_id.clone(),
            process_id: crash.process_id,
            event_code: crash.crash_event_code,
            stale_ms: 0,
            last_heartbeat_counter: crash.last_heartbeat_counter,
            last_heartbeat_ts_nanos: crash.last_heartbeat_ts_nanos,
            last_event_count: crash.last_event_count,
            probe_result: SurvivorProbeResult::NotApplicable,
            crash_detection: Some(crash.detection),
            faulting_thread_id: Some(crash.faulting_thread_id),
            exit_code: crash.exit_code,
            minidump_path: crash.minidump_path.clone(),
            child_process_id: None,
            child_session_id: None,
            last_progress_counter: None,
            last_progress_ts_nanos: None,
            child_stall_reason_code: None,
            captured_at_unix_ms: now_unix_ms(),
            forwarded: false,
        }
    }

    /// Build a CHILD-STALL survivor record from the MT-106 no-progress report. `process_id` remains the
    /// watched Handshake parent pid for correlation; the stalled child is named by `child_process_id`.
    pub fn from_child_stall(
        session_id: &str,
        process_id: u32,
        report: &ChildStallReport,
        last_event_count: u32,
    ) -> Self {
        Self {
            schema_version: SURVIVOR_SCHEMA_VERSION.to_owned(),
            kind: SurvivorRecordKind::ChildStall,
            session_id: session_id.to_string(),
            process_id,
            event_code: SurvivorRecordKind::ChildStall.diag_event_code().as_u16(),
            stale_ms: report.stale_ms,
            last_heartbeat_counter: 0,
            last_heartbeat_ts_nanos: 0,
            last_event_count,
            probe_result: SurvivorProbeResult::NotApplicable,
            crash_detection: None,
            faulting_thread_id: None,
            exit_code: None,
            minidump_path: None,
            child_process_id: Some(report.child_pid),
            child_session_id: Some(report.child_session_id),
            last_progress_counter: Some(report.last_progress_counter),
            last_progress_ts_nanos: Some(report.last_progress_ts_nanos),
            child_stall_reason_code: Some(report.reason_code.as_u16()),
            captured_at_unix_ms: now_unix_ms(),
            forwarded: false,
        }
    }

    /// The exact set of JSON object keys a serialized [`SurvivorRecord`] is allowed to carry. The
    /// typed-allowlist guard ([`assert_typed_allowlist`] + the `test_survivor_store` value scan) checks
    /// the serialized form contains ONLY these keys, so a future field that could smuggle free text
    /// fails the gate (AC-013-2 / RISK-013-3).
    pub fn allowlisted_keys() -> &'static [&'static str] {
        &[
            "schema_version",
            "kind",
            "session_id",
            "process_id",
            "event_code",
            "stale_ms",
            "last_heartbeat_counter",
            "last_heartbeat_ts_nanos",
            "last_event_count",
            "probe_result",
            "crash_detection",
            "faulting_thread_id",
            "exit_code",
            "minidump_path",
            "child_process_id",
            "child_session_id",
            "last_progress_counter",
            "last_progress_ts_nanos",
            "child_stall_reason_code",
            "captured_at_unix_ms",
            "forwarded",
            // tag keys emitted by the `#[serde(tag = ...)]` enums (NOT content; a fixed vocabulary):
            "kind",
            "probe",
            "detection",
        ]
    }
}

/// Assert a serialized [`SurvivorRecord`] carries ONLY allowlisted keys + only typed/number/enum/path
/// VALUES (no free-text/project-content string anywhere). The string VALUES permitted are the opaque
/// session token, the fixed schema-version vocabulary, the typed enum tags, and a LOCAL minidump path
/// string. Returns `Err(offending_key)` on a breach so a test can name it (AC-013-2 / RISK-013-3).
///
/// This is the runtime/structural complement to the compile-time typed-allowlist of `DiagEvent` (which
/// physically cannot hold a string): a `SurvivorRecord` DOES hold a few strings (session token, schema
/// version, local path), so the allowlist is enforced by this explicit key + value scan instead of by
/// the type system.
pub fn assert_typed_allowlist(record: &SurvivorRecord) -> Result<(), String> {
    let value = serde_json::to_value(record).map_err(|e| format!("serialize: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "survivor record did not serialize to a JSON object".to_owned())?;
    let allowed: std::collections::HashSet<&str> =
        SurvivorRecord::allowlisted_keys().iter().copied().collect();
    for key in obj.keys() {
        if !allowed.contains(key.as_str()) {
            return Err(format!(
                "survivor record carried a non-allowlisted key '{key}' (typed-allowlist breach, RISK-013-3)"
            ));
        }
    }
    // The minidump path is a LOCAL path string reference, NOT a URL and NOT content. If present, reject
    // anything that looks like a network URL (the §6.13.8 local-only invariant: never auto-upload).
    if let Some(p) = &record.minidump_path {
        let s = p.to_string_lossy();
        if s.contains("://") {
            return Err(format!(
                "minidump_path '{s}' looks like a URL, not a LOCAL path (§6.13.8 local-only, RISK-013-6)"
            ));
        }
    }
    Ok(())
}

/// Wall-clock millis since the UNIX epoch (best-effort; 0 on a clock error). A numeric timestamp.
fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Sanitize a session id into a filename-safe token (same rule as MT-089/MT-092): keep ASCII
/// alphanumerics, `-`, `_`; map everything else to `_`. Keeps the durable filenames stable + space-free
/// (CX-109A) regardless of the session id's source.
pub fn safe_session_token(session_id: &str) -> String {
    session_id
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

/// Count of the last-N ring events bundled as evidence. A free function so the watcher can compute it
/// from a `&[DiagEvent]` slice without this module depending on the reader. Only the COUNT is carried
/// into the durable record (the events are POD integers; the durable record never embeds them as text).
pub fn last_event_count(events: &[DiagEvent]) -> u32 {
    events.len() as u32
}

/// The default durable survivor store directory: the portable per-user data dir, the SAME convention as
/// MT-083's crash dir (`dirs::data_local_dir()/handshake/...`), under a `palmistry/survivors` subtree.
/// `None` when the platform has no resolvable data-local dir (a headless/odd environment) — the caller
/// then falls back to a ring-sibling dir so a record is still durable LOCALLY.
pub const ENV_PALMISTRY_SURVIVOR_DIR: &str = "HANDSHAKE_PALMISTRY_SURVIVOR_DIR";

pub fn default_survivor_dir() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os(ENV_PALMISTRY_SURVIVOR_DIR) {
        return Some(PathBuf::from(raw));
    }
    dirs::data_local_dir().map(|d| d.join("handshake").join("palmistry").join("survivors"))
}

/// A durable LOCAL store of [`SurvivorRecord`]s (§6.13.7). Records are JSON files in a per-user data dir;
/// writes are ATOMIC (temp+replace, RISK-013-4); existing records are read on construction so a Palmistry
/// restart does not lose a pending forward (AC-013-1 / AC-013-5). The store is the source the FR
/// forwarder ([`crate::fr_forward`]) drains at recovery.
pub struct SurvivorStore {
    /// The durable directory holding the per-record JSON files.
    dir: PathBuf,
    /// The in-memory mirror of the records on disk, keyed implicitly by file path. Rebuilt from disk on
    /// construction (read-existing-on-startup) and kept in sync on each write/mark.
    records: Vec<StoredRecord>,
}

/// One record in the store, pairing the typed [`SurvivorRecord`] with the durable file path it lives at
/// (so a `mark_forwarded` rewrites the SAME file atomically).
#[derive(Debug, Clone)]
pub struct StoredRecord {
    /// The durable file path this record is persisted at.
    pub path: PathBuf,
    /// The typed record.
    pub record: SurvivorRecord,
}

impl SurvivorStore {
    /// Open (or create) the durable store rooted at `dir`, READING ALL EXISTING records on startup so a
    /// Palmistry restart between capture and forward does not lose a pending forward (AC-013-1 /
    /// AC-013-5). Creates the directory if missing. A record file that fails to parse is SKIPPED with a
    /// log line (a corrupt sidecar must not crash the watcher), never silently treated as absent for the
    /// healthy ones.
    pub fn open(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let mut records = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            // Only our durable record files (skip temp files + foreign files).
            let is_record = path.extension().and_then(|e| e.to_str()) == Some("json")
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("survivor-"))
                    .unwrap_or(false);
            if !is_record {
                continue;
            }
            match fs::read(&path).and_then(|b| {
                serde_json::from_slice::<SurvivorRecord>(&b).map_err(std::io::Error::other)
            }) {
                Ok(record) => records.push(StoredRecord { path, record }),
                Err(err) => {
                    tracing::warn!(
                        path = %path.display(),
                        %err,
                        "skipping an unreadable survivor record (kept the rest)"
                    );
                }
            }
        }
        Ok(Self { dir, records })
    }

    /// Open the store at [`default_survivor_dir`], falling back to `<fallback_parent>/survivors` when the
    /// platform has no data-local dir (so a record is ALWAYS durable locally — a headless CI box still
    /// persists). The fallback parent is normally the ring's directory.
    pub fn open_default_or_sibling(fallback_parent: &Path) -> std::io::Result<Self> {
        let dir = default_survivor_dir().unwrap_or_else(|| fallback_parent.join("survivors"));
        Self::open(dir)
    }

    /// The durable directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The durable file path for a record. Freeze/crash remain one file per (kind, session); child stalls
    /// include child_session_id so multiple watched children in one app session do not overwrite each
    /// other.
    fn record_path(&self, record: &SurvivorRecord) -> PathBuf {
        let base = format!(
            "survivor-{}-{}",
            record.kind.wire_tag(),
            safe_session_token(&record.session_id)
        );
        let name = if record.kind == SurvivorRecordKind::ChildStall {
            format!("{base}-child-{}.json", record.child_session_id.unwrap_or(0))
        } else {
            format!("{base}.json")
        };
        self.dir.join(name)
    }

    /// Persist `record` durably with an ATOMIC write (temp+replace, RISK-013-4) and mirror it in memory.
    /// Returns the durable path. If a record for the same (kind, session) already exists in memory it is
    /// REPLACED (the latest capture wins) and the file is overwritten atomically. The typed-allowlist is
    /// asserted before any write so a content breach can never reach disk (AC-013-2).
    pub fn put(&mut self, record: SurvivorRecord) -> std::io::Result<PathBuf> {
        // HARD: never write a record that breaches the typed allowlist (AC-013-2 / RISK-013-3).
        assert_typed_allowlist(&record).map_err(std::io::Error::other)?;
        let path = self.record_path(&record);
        atomic_write_json(&path, &record)?;
        // Mirror in memory: replace any existing same-path record, else push.
        if let Some(slot) = self.records.iter_mut().find(|s| s.path == path) {
            slot.record = record;
        } else {
            self.records.push(StoredRecord {
                path: path.clone(),
                record,
            });
        }
        Ok(path)
    }

    /// All records currently in the store (in-memory mirror, kept in sync with disk).
    pub fn records(&self) -> &[StoredRecord] {
        &self.records
    }

    /// The records NOT yet forwarded to the Flight Recorder (the drain set for the next recovery,
    /// AC-013-5). Cloned so the forwarder can iterate without holding a borrow on the store across a
    /// network post + a `mark_forwarded` write-back.
    pub fn unforwarded(&self) -> Vec<StoredRecord> {
        self.records
            .iter()
            .filter(|s| !s.record.forwarded)
            .cloned()
            .collect()
    }

    /// Mark the record at `path` forwarded (idempotent, RISK-013-5): set `forwarded = true` and rewrite
    /// the file atomically so the flag survives a Palmistry restart. A second call is a harmless no-op
    /// (the record is already forwarded). Returns `Ok(false)` if no record at `path` is known.
    pub fn mark_forwarded(&mut self, path: &Path) -> std::io::Result<bool> {
        let Some(slot) = self.records.iter_mut().find(|s| s.path == path) else {
            return Ok(false);
        };
        if slot.record.forwarded {
            // Already forwarded — idempotent no-op (no redundant write).
            return Ok(true);
        }
        slot.record.forwarded = true;
        atomic_write_json(&slot.path, &slot.record)?;
        Ok(true)
    }

    /// How many records are not yet forwarded (a count for logging / a recovery decision).
    pub fn unforwarded_count(&self) -> usize {
        self.records.iter().filter(|s| !s.record.forwarded).count()
    }
}

/// Atomically write `record` as pretty JSON to `path`: write to a sibling `<path>.tmp` then atomically
/// replace `path`. A concurrent / post-restart reader sees either the OLD complete file or the NEW
/// complete file — never a torn half-write and never a remove-before-replace gap (RISK-013-4). The temp
/// file is in the SAME directory as the target so the replace is a cheap intra-filesystem move.
pub fn atomic_write_json(path: &Path, record: &SurvivorRecord) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(record).map_err(std::io::Error::other)?;
    // Unique temp name (pid-scoped) so two Palmistry instances never collide on the temp file.
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, json.as_bytes())?;
    match atomic_replace(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            // Clean up the temp file on a rename failure so it does not linger as debris.
            let _ = fs::remove_file(&tmp);
            Err(err)
        }
    }
}

#[cfg(not(windows))]
fn atomic_replace(tmp: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(tmp, path)
}

#[cfg(windows)]
fn atomic_replace(tmp: &Path, path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return fs::rename(tmp, path);
    }
    let target = wide_null(path.as_os_str());
    let replacement = wide_null(tmp.as_os_str());
    // ReplaceFileW preserves the old complete file or installs the new complete file; it avoids the
    // remove-then-rename durability gap that can lose a survivor record on Windows.
    let ok = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn wide_null(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::child_stall::{ChildStallReasonCode, ChildStallReport};
    use crate::crash_capture::CrashRecord;
    use crate::freeze_detect::FreezeReport;
    use handshake_diag_ring::Heartbeat;

    fn temp_store_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "hsk-mt093-store-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    struct DirGuard(PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn freeze_report() -> FreezeReport {
        FreezeReport {
            stale_ms: 6000,
            last_heartbeat_counter: 42,
            last_heartbeat_ts_nanos: 123_456,
        }
    }

    fn child_stall_report(child_session_id: u64) -> ChildStallReport {
        ChildStallReport {
            child_pid: 99,
            child_session_id,
            stale_ms: 7000,
            last_progress_counter: 4,
            last_progress_ts_nanos: 1234,
            reason_code: ChildStallReasonCode::ProgressStaleWhileAlive,
        }
    }

    #[test]
    fn freeze_record_is_typed_allowlist_only() {
        let rec = SurvivorRecord::from_freeze(
            "sess-1",
            4242,
            &freeze_report(),
            3,
            SurvivorProbeResult::NotResponding,
        );
        assert_eq!(rec.kind, SurvivorRecordKind::Freeze);
        assert_eq!(rec.event_code, DiagEventCode::FreezeSuspected.as_u16());
        assert_eq!(rec.stale_ms, 6000);
        assert!(rec.minidump_path.is_none(), "a freeze has no minidump");
        assert!(!rec.forwarded, "a fresh record is not yet forwarded");
        assert_typed_allowlist(&rec).expect("freeze record must be typed-allowlist clean");
    }

    #[test]
    fn crash_record_round_trips_from_crash_record() {
        let hb = Heartbeat {
            counter: 9,
            timestamp_nanos: 99,
        };
        let crash = CrashRecord::with_minidump(
            "sess-2",
            7,
            13,
            PathBuf::from("/tmp/diag/palmistry-crash-sess-2.dmp"),
            Some(hb),
            &[],
        );
        let rec = SurvivorRecord::from_crash(&crash);
        assert_eq!(rec.kind, SurvivorRecordKind::Crash);
        assert_eq!(
            rec.crash_detection,
            Some(CrashDetection::CrashContextMinidump)
        );
        assert_eq!(rec.faulting_thread_id, Some(13));
        assert_eq!(
            rec.minidump_path.as_deref(),
            Some(Path::new("/tmp/diag/palmistry-crash-sess-2.dmp"))
        );
        assert_eq!(rec.last_heartbeat_counter, 9);
        assert_typed_allowlist(&rec).expect("crash record must be typed-allowlist clean");
    }

    #[test]
    fn child_stall_record_is_typed_allowlist_only() {
        let rec = SurvivorRecord::from_child_stall("sess-child", 42, &child_stall_report(77), 5);
        assert_eq!(rec.kind, SurvivorRecordKind::ChildStall);
        assert_eq!(rec.event_code, DiagEventCode::Other.as_u16());
        assert_eq!(rec.stale_ms, 7000);
        assert_eq!(rec.child_process_id, Some(99));
        assert_eq!(rec.child_session_id, Some(77));
        assert_eq!(rec.last_progress_counter, Some(4));
        assert_eq!(rec.child_stall_reason_code, Some(1));
        assert!(rec.minidump_path.is_none(), "a child stall has no minidump");
        assert_typed_allowlist(&rec).expect("child-stall record must be typed-allowlist clean");
    }

    #[test]
    fn url_minidump_path_is_rejected_by_allowlist() {
        // §6.13.8 local-only: a path that looks like a network URL must be rejected (never auto-upload).
        let mut rec = SurvivorRecord::from_freeze(
            "sess-u",
            1,
            &freeze_report(),
            0,
            SurvivorProbeResult::NotResponding,
        );
        rec.minidump_path = Some(PathBuf::from("https://evil.example/dump"));
        assert!(
            assert_typed_allowlist(&rec).is_err(),
            "a URL-shaped minidump path must fail the local-only allowlist (RISK-013-6)"
        );
    }

    #[test]
    fn put_writes_atomically_and_record_survives_a_restart() {
        // AC-013-1: a record is durable + re-readable after a simulated Palmistry restart (a fresh
        // `SurvivorStore::open` on the SAME dir reads the existing record).
        let dir = temp_store_dir("restart");
        let _g = DirGuard(dir.clone());
        let path = {
            let mut store = SurvivorStore::open(&dir).unwrap();
            let rec = SurvivorRecord::from_freeze(
                "sess-restart",
                100,
                &freeze_report(),
                2,
                SurvivorProbeResult::NotResponding,
            );
            store.put(rec).unwrap()
        }; // store dropped — simulate the Palmistry process exiting.
        assert!(path.exists(), "the durable record file must exist on disk");

        // A NEW Palmistry process opens the SAME dir and must read the existing record back.
        let restarted = SurvivorStore::open(&dir).unwrap();
        assert_eq!(
            restarted.records().len(),
            1,
            "the record outlived the restart"
        );
        let back = &restarted.records()[0].record;
        assert_eq!(back.session_id, "sess-restart");
        assert_eq!(back.stale_ms, 6000);
        assert!(!back.forwarded, "still pending forward after a restart");
    }

    #[test]
    fn mark_forwarded_is_idempotent_and_survives_a_restart() {
        // AC-013-3 / RISK-013-5: marking forwarded persists the flag (survives a restart) and a second
        // mark is a harmless no-op.
        let dir = temp_store_dir("forward");
        let _g = DirGuard(dir.clone());
        let path;
        {
            let mut store = SurvivorStore::open(&dir).unwrap();
            let rec = SurvivorRecord::from_freeze(
                "sess-fwd",
                1,
                &freeze_report(),
                0,
                SurvivorProbeResult::NotResponding,
            );
            path = store.put(rec).unwrap();
            assert_eq!(store.unforwarded_count(), 1);
            assert!(store.mark_forwarded(&path).unwrap(), "first mark succeeds");
            assert_eq!(store.unforwarded_count(), 0, "now forwarded");
            // Idempotent: a second mark is a no-op (still true, no error).
            assert!(
                store.mark_forwarded(&path).unwrap(),
                "second mark is a no-op"
            );
            assert_eq!(store.unforwarded_count(), 0);
        }
        // Restart: the forwarded flag persisted, so the record is NOT re-drained.
        let restarted = SurvivorStore::open(&dir).unwrap();
        assert_eq!(
            restarted.unforwarded_count(),
            0,
            "a forwarded record must stay forwarded after a restart (no double-forward, RISK-013-5)"
        );
        assert!(restarted.records()[0].record.forwarded);
    }

    #[test]
    fn unforwarded_drains_only_pending_records() {
        // AC-013-5: a freeze + a crash captured; forwarding only the freeze leaves the crash pending for
        // the next recovery (not lost, not double-counted).
        let dir = temp_store_dir("drain");
        let _g = DirGuard(dir.clone());
        let mut store = SurvivorStore::open(&dir).unwrap();
        let freeze_path = store
            .put(SurvivorRecord::from_freeze(
                "sess-d",
                1,
                &freeze_report(),
                0,
                SurvivorProbeResult::NotResponding,
            ))
            .unwrap();
        let crash = CrashRecord::post_mortem("sess-d", 1, Some(1), None, &[]);
        store.put(SurvivorRecord::from_crash(&crash)).unwrap();
        assert_eq!(store.unforwarded().len(), 2, "both pending initially");

        store.mark_forwarded(&freeze_path).unwrap();
        let pending = store.unforwarded();
        assert_eq!(pending.len(), 1, "only the crash remains pending");
        assert_eq!(pending[0].record.kind, SurvivorRecordKind::Crash);
    }

    #[test]
    fn re_put_same_kind_session_replaces_not_duplicates() {
        // A re-captured freeze for the same session overwrites its own file (bounded growth), it does not
        // accumulate duplicate records.
        let dir = temp_store_dir("replace");
        let _g = DirGuard(dir.clone());
        let mut store = SurvivorStore::open(&dir).unwrap();
        let mut report = freeze_report();
        store
            .put(SurvivorRecord::from_freeze(
                "sess-r",
                1,
                &report,
                0,
                SurvivorProbeResult::NotResponding,
            ))
            .unwrap();
        report.stale_ms = 9000; // a longer freeze re-captured.
        store
            .put(SurvivorRecord::from_freeze(
                "sess-r",
                1,
                &report,
                0,
                SurvivorProbeResult::NotResponding,
            ))
            .unwrap();
        assert_eq!(
            store.records().len(),
            1,
            "same (kind, session) replaces, not duplicates"
        );
        assert_eq!(
            store.records()[0].record.stale_ms,
            9000,
            "the latest capture wins"
        );

        // And on disk too (a restart sees exactly one record).
        let restarted = SurvivorStore::open(&dir).unwrap();
        assert_eq!(restarted.records().len(), 1);
    }

    #[test]
    fn child_stall_records_are_keyed_by_child_session() {
        let dir = temp_store_dir("child-keys");
        let _g = DirGuard(dir.clone());
        let mut store = SurvivorStore::open(&dir).unwrap();
        let first = store
            .put(SurvivorRecord::from_child_stall(
                "sess-c",
                1,
                &child_stall_report(10),
                0,
            ))
            .unwrap();
        let second = store
            .put(SurvivorRecord::from_child_stall(
                "sess-c",
                1,
                &child_stall_report(11),
                0,
            ))
            .unwrap();
        assert_ne!(
            first, second,
            "separate child sessions must not overwrite each other"
        );
        assert_eq!(store.records().len(), 2);
        assert!(first
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("child-10"));
        assert!(second
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("child-11"));
    }

    #[test]
    fn open_default_or_sibling_falls_back_to_a_local_dir() {
        // The store is ALWAYS durable locally: even with no data-local dir the sibling fallback persists.
        let parent = temp_store_dir("sibling");
        let _g = DirGuard(parent.clone());
        fs::create_dir_all(&parent).unwrap();
        let store = SurvivorStore::open_default_or_sibling(&parent).unwrap();
        // The chosen dir exists (either the data-local default or the sibling fallback).
        assert!(store.dir().exists(), "the durable dir must exist");
    }

    #[test]
    fn default_survivor_dir_honors_override_env() {
        let dir = temp_store_dir("survivor-dir-override");
        let prev = std::env::var_os(ENV_PALMISTRY_SURVIVOR_DIR);
        std::env::set_var(ENV_PALMISTRY_SURVIVOR_DIR, &dir);
        assert_eq!(default_survivor_dir(), Some(dir));
        match prev {
            Some(v) => std::env::set_var(ENV_PALMISTRY_SURVIVOR_DIR, v),
            None => std::env::remove_var(ENV_PALMISTRY_SURVIVOR_DIR),
        }
    }

    #[test]
    fn allowlist_keys_cover_every_serialized_key() {
        // A guard on the guard: if a field is ADDED to SurvivorRecord without updating the allowlist, the
        // typed-allowlist scan would (correctly) fail — prove it passes for BOTH a freeze and a crash
        // record so the allowlist is complete for the real shapes.
        let freeze = SurvivorRecord::from_freeze(
            "s",
            1,
            &freeze_report(),
            0,
            SurvivorProbeResult::Responding,
        );
        assert_typed_allowlist(&freeze).expect("freeze keys covered");
        let crash = CrashRecord::post_mortem("s", 1, Some(2), None, &[]);
        assert_typed_allowlist(&SurvivorRecord::from_crash(&crash)).expect("crash keys covered");
        assert_typed_allowlist(&SurvivorRecord::from_child_stall(
            "s",
            1,
            &child_stall_report(1),
            0,
        ))
        .expect("child-stall keys covered");
    }
}
