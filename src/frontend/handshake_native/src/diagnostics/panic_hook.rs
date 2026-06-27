//! The panic hook — Tier 2 internal_diagnostics DURABLE LOCAL CRASH RECORD
//! (Master Spec v02.196 §5.8.2 "durable crash record" + §5.8.3 typed-allowlist).
//!
//! # What this is
//!
//! [`install_panic_hook`] installs a process-global [`std::panic::set_hook`] that, on ANY Rust panic,
//! BEFORE the default abort/unwind runs:
//!
//! 1. writes a DURABLE LOCAL crash record (thread name/id, panic source location `file:line:col`, a
//!    captured backtrace, a monotonic + wall timestamp) to local disk via an ATOMIC temp+rename write;
//!    and
//! 2. signals the MT-081 ring with a typed [`DiagEventCode::PanicCaught`] [`crate::diagnostics::record`]
//!    so the external Palmistry watcher (Tier 3, MT-092) can distinguish a panic-exit from a clean
//!    [`DiagEventCode::Shutdown`] when it observes the process die; and then
//! 3. CHAINS to the previous hook (eframe/tracing/default) so the panic is NOT swallowed.
//!
//! This is the IN-PROCESS half of crash capture. The crashing process cannot reliably dump itself
//! (that is Palmistry's out-of-process minidump, MT-092), but it CAN write a backtrace + a ring marker
//! on the way down — the cheap, always-available first-party record.
//!
//! # Tier placement
//!
//! - **Tier 1 (Flight Recorder)** — kept as-is. Forwarding the typed crash fields into the FR is MT-093,
//!   NOT here. This module writes the LOCAL record + the ring marker only.
//! - **Tier 2 (internal_diagnostics)** — THIS module is the §5.8.2 "durable crash record" component.
//! - **Tier 3 (Palmistry)** — reads the `PanicCaught` ring marker (the live link this hook produces).
//!
//! # The typed-allowlist invariant — where the panic MESSAGE goes (§5.8.3)
//!
//! The crash record is MECHANICAL only: thread id, panic source LOCATION (a `file:line:col` of the
//! panic SITE — code position, not project data), backtrace frames (symbol names / addresses — code
//! structure, not user data), counters, and timestamps. The panic MESSAGE string is the ONE risk
//! surface: a panic payload can carry interpolated runtime values.
//!
//! Policy (DEFAULT = OMIT the payload from anything that leaves the local file):
//! - The RING write is structurally content-free — [`crate::diagnostics::record_with`] accepts only
//!   typed integers; there is NO string field on `DiagEvent`, so the message physically cannot enter
//!   the ring. (RISK-003-1 is closed by the type system, not a runtime filter.)
//! - The DURABLE on-disk record (`crash-<session>-<ts>.json`) carries the typed fields + the backtrace.
//!   The free-form payload message is written ONLY into a SEPARATE, clearly-marked LOCAL-ONLY sidecar
//!   (`crash-<session>-<ts>.message.txt`) that MT-093 reads through the typed-allowlist filter — it is
//!   NEVER forwarded raw. The primary JSON record does NOT contain the message, so the artifact that
//!   any forwarder ingests is content-free by construction.
//!
//! # Panic-safety (RISK-003-2)
//!
//! A panic INSIDE a panic hook aborts the process immediately with NO record. So the hook is written to
//! never panic: no `unwrap`/`expect`/`?`/indexing on fallible paths, best-effort IO whose errors are
//! ignored, and the minimum work needed. A failed write must not re-panic — it returns cleanly and the
//! chained previous hook still runs (RISK-003-3).

use std::backtrace::Backtrace;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::panic::{self, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use handshake_diag_ring::{DiagEventCode, DiagPhase, DiagSeverity};

/// Process-start diagnostic session id, generated ONCE in `main()` before the panic hook and the
/// MT-081 ring are created, so the crash file name and the ring share the SAME session id.
/// [`install_panic_hook`] writes it; [`process_session_id`] reads it (the ring install path in
/// `HandshakeApp` reuses it so a watcher correlates the crash file to the ring).
static PROCESS_SESSION_ID: OnceLock<String> = OnceLock::new();

/// Set (once) the process-start session id `main()` minted, and return the effective id. If it was
/// already set, the existing value is returned (the first writer wins — the hook and the ring agree).
///
/// Called by [`install_panic_hook`] (passing the id `main()` generated) and read by the ring-install
/// path so the crash record and the ring carry the same correlation id. Carries NO sensitive data — a
/// uuid only.
pub fn set_process_session_id(session_id: &str) -> &'static str {
    PROCESS_SESSION_ID.get_or_init(|| session_id.to_string())
}

/// The process-start session id, if [`set_process_session_id`] / [`install_panic_hook`] has run.
/// `None` in a headless/test shell that installed neither. Used by the ring-install path so the ring
/// reuses the SAME id the crash file is named with.
pub fn process_session_id() -> Option<&'static str> {
    PROCESS_SESSION_ID.get().map(String::as_str)
}

/// Resolve the per-user, portable crash directory: `<data_local>/handshake/crash/`
/// (e.g. `%LOCALAPPDATA%\handshake\crash` on Windows, `~/.local/share/handshake/crash` on Linux),
/// via the `dirs` crate — NEVER a hardcoded drive-letter / user-profile path (GLOBAL-PORTABILITY-004 /
/// AC-003-5). Returns `None` only if the platform has no local-data dir (extremely rare); the caller
/// then degrades to a still-portable temp-dir fallback so a crash is never lost to a missing dir.
pub fn default_crash_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("handshake").join("crash"))
}

/// Install the durable-local-crash-record panic hook.
///
/// `crash_dir` is the directory crash records are written to (resolve via [`default_crash_dir`] in
/// `main()`); `session_id` is the process-start id shared with the MT-081 ring so a watcher correlates
/// the crash file to the ring. Call this EARLY in `main()`, BEFORE `eframe::run_native` (ideally before
/// tracing init) so a startup-time panic is also captured.
///
/// The hook CAPTURES the previous hook ([`std::panic::take_hook`]) and CHAINS to it after recording, so
/// the default abort/unwind + any eframe/tracing hook still runs (the panic is never swallowed —
/// AC-003-4 / RISK-003-3).
pub fn install_panic_hook(crash_dir: PathBuf, session_id: &str) {
    let session_id = set_process_session_id(session_id).to_string();
    // Capture whatever hook is currently installed (default, or eframe/tracing's) so we CHAIN to it.
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        // Best-effort, panic-free: record first, then ALWAYS chain to the previous hook.
        write_crash_record(&crash_dir, &session_id, info);
        signal_ring();
        // Chain: the default abort/unwind (or eframe/tracing's hook) still runs — the panic is NOT
        // swallowed. This is the LAST statement so the record is durably flushed before the process
        // aborts (panic = "abort" in release-native runs the hook ONCE then aborts).
        previous(info);
    }));
}

/// The typed crash fields that go into BOTH the on-disk JSON record AND (the integer-only subset) the
/// ring. NO free-form message field — the payload string is handled separately (local-only sidecar).
struct CrashFields {
    /// Panic source file (`PanicHookInfo::location().file()`), a code path — NOT project data.
    location_file: String,
    /// Panic source line.
    location_line: u32,
    /// Panic source column.
    location_col: u32,
    /// Current thread name (`<unnamed>` if none) — a code-level thread label, not project data.
    thread_name: String,
    /// Opaque numeric thread id (the low bits of `ThreadId` hashed to a u64).
    thread_id: u64,
    /// Wall-clock timestamp (nanoseconds since the Unix epoch) for human correlation.
    wall_unix_nanos: u128,
    /// Monotonic timestamp (nanoseconds) so a reader can order crash vs ring/heartbeat events.
    monotonic_nanos: u64,
    /// The captured backtrace, rendered to text. Symbol names / addresses — code structure, NOT user
    /// data. Lives in the durable record (local-only), never forwarded raw.
    backtrace_text: String,
}

/// Gather the typed crash fields from the panic info + the runtime. Panic-free (no unwrap).
fn collect_fields(info: &PanicHookInfo<'_>) -> CrashFields {
    let (location_file, location_line, location_col) = match info.location() {
        Some(loc) => (loc.file().to_string(), loc.line(), loc.column()),
        None => ("<unknown>".to_string(), 0, 0),
    };

    let current = std::thread::current();
    let thread_name = current.name().unwrap_or("<unnamed>").to_string();
    let thread_id = thread_id_to_u64(&current);

    // Wall clock for human correlation; monotonic for ordering against the ring/heartbeat.
    let wall_unix_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let monotonic_nanos = monotonic_now_nanos();

    // force_capture() symbolizes regardless of RUST_BACKTRACE (always-on crash backtrace) — std only,
    // no `backtrace` crate.
    let backtrace_text = Backtrace::force_capture().to_string();

    CrashFields {
        location_file,
        location_line,
        location_col,
        thread_name,
        thread_id,
        wall_unix_nanos,
        monotonic_nanos,
        backtrace_text,
    }
}

/// Hash a [`std::thread::ThreadId`] to an opaque `u64`. `ThreadId` has no stable public integer
/// accessor on stable Rust, so its `Debug` form (`ThreadId(N)`) is hashed to a stable-per-run opaque
/// number. This is a CODE-level identifier, not project data.
fn thread_id_to_u64(thread: &std::thread::Thread) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    thread.id().hash(&mut hasher);
    hasher.finish()
}

/// A monotonic nanosecond timestamp from a process-start `Instant` baseline. Monotonic (never goes
/// backward) so a reader can order the crash relative to ring heartbeats.
fn monotonic_now_nanos() -> u64 {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    let start = START.get_or_init(std::time::Instant::now);
    start.elapsed().as_nanos() as u64
}

/// Write the durable crash record. ATOMIC (temp + flush + sync_all + rename) so a crash mid-write never
/// leaves a truncated final record (RISK-003-4 / AC-003-3). Best-effort + panic-free: every IO error is
/// ignored (a failed write must NOT re-panic — RISK-003-2 / AC-003-3); the hook then chains to the
/// previous hook regardless.
fn write_crash_record(crash_dir: &Path, session_id: &str, info: &PanicHookInfo<'_>) {
    let fields = collect_fields(info);

    // Ensure the crash dir exists (ignore the error — the writes below will simply no-op if it does
    // not, and an unwritable dir must NOT re-panic; AC-003-3 IO-failure path).
    let _ = fs::create_dir_all(crash_dir);

    // Stable, collision-resistant file stem: session id + wall timestamp + thread id.
    let stem = format!(
        "crash-{}-{}-{}",
        sanitize_token(session_id),
        fields.wall_unix_nanos,
        fields.thread_id
    );

    // 1) The DURABLE typed JSON record (NO free-form payload message — content-free by construction).
    let json = build_record_json(session_id, &fields);
    let final_path = crash_dir.join(format!("{stem}.json"));
    let tmp_path = crash_dir.join(format!("{stem}.json.tmp"));
    atomic_write(&tmp_path, &final_path, json.as_bytes());

    // 2) The LOCAL-ONLY message sidecar (the ONE free-text risk surface). Written into a clearly-marked
    //    separate file so the primary record stays content-free; MT-093 reads THIS through the typed
    //    allowlist filter, never raw, and the default is to never forward it. Only written when a string
    //    payload is actually present.
    if let Some(message) = payload_message(info) {
        let msg_final = crash_dir.join(format!("{stem}.message.txt"));
        let msg_tmp = crash_dir.join(format!("{stem}.message.txt.tmp"));
        // Prefix marks the file as local-only so a human or a forwarder cannot mistake it for the
        // content-free record.
        let body = format!("LOCAL-ONLY (not forwarded; typed-allowlist filtered by MT-093)\n{message}");
        atomic_write(&msg_tmp, &msg_final, body.as_bytes());
    }
}

/// Extract the panic payload as a `String` if it is a `&str` or `String` (the two standard payload
/// types). Returns `None` for a non-string payload (nothing to write). Never panics.
fn payload_message(info: &PanicHookInfo<'_>) -> Option<String> {
    let payload = info.payload();
    if let Some(s) = payload.downcast_ref::<&str>() {
        Some((*s).to_string())
    } else {
        payload.downcast_ref::<String>().cloned()
    }
}

/// Render the typed crash fields to a JSON record. Hand-serialized (no serde derive on a hot panic
/// path; the fields are simple) with all strings JSON-escaped. Contains NO free-form panic message —
/// only the mechanical typed fields + the backtrace (code structure). Self-describing so MT-093 / a
/// human can read it without this module.
fn build_record_json(session_id: &str, f: &CrashFields) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema\": \"hsk.diag.crash_record@1\",\n",
            "  \"kind\": \"panic\",\n",
            "  \"note\": \"typed-allowlist: mechanical fields only; no free-form panic message (see .message.txt local-only sidecar)\",\n",
            "  \"session_id\": \"{session}\",\n",
            "  \"event_code\": {event_code},\n",
            "  \"severity\": {severity},\n",
            "  \"location_file\": \"{file}\",\n",
            "  \"location_line\": {line},\n",
            "  \"location_col\": {col},\n",
            "  \"thread_name\": \"{thread_name}\",\n",
            "  \"thread_id\": {thread_id},\n",
            "  \"wall_unix_nanos\": {wall},\n",
            "  \"monotonic_nanos\": {mono},\n",
            "  \"backtrace\": \"{backtrace}\"\n",
            "}}\n",
        ),
        session = json_escape(session_id),
        event_code = DiagEventCode::PanicCaught.as_u16(),
        severity = DiagSeverity::Error.as_u8(),
        file = json_escape(&f.location_file),
        line = f.location_line,
        col = f.location_col,
        thread_name = json_escape(&f.thread_name),
        thread_id = f.thread_id,
        wall = f.wall_unix_nanos,
        mono = f.monotonic_nanos,
        backtrace = json_escape(&f.backtrace_text),
    )
}

/// Minimal JSON string escaping (quotes, backslashes, control chars). Enough to keep the hand-built
/// record valid JSON for the backtrace + paths; never panics.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Sanitize a token (session id) to a filesystem-safe form (no spaces / separators) per CX-109A, so the
/// crash file name never contains a space and matches the ring backing-file naming convention.
fn sanitize_token(token: &str) -> String {
    token
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

/// Atomically write `bytes` to `final_path`: write to `tmp_path`, flush, `sync_all` to force the data
/// to disk, then rename onto the final path (rename is atomic on the same filesystem). A crash
/// mid-write therefore leaves only the `.tmp`, never a half-written final record (RISK-003-4). Every IO
/// error is IGNORED (best-effort, panic-free — AC-003-3); a failed write simply produces no record.
fn atomic_write(tmp_path: &Path, final_path: &Path, bytes: &[u8]) {
    // Write + flush + fsync the temp file, scoped so the File is closed before the rename.
    let wrote = (|| -> std::io::Result<()> {
        let mut file: File = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(tmp_path)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();
    // Only rename if the temp write fully succeeded — otherwise leave the (incomplete) temp and emit
    // no final record. Either way, NEVER panic.
    if wrote.is_ok() {
        let _ = fs::rename(tmp_path, final_path);
    }
}

/// Signal the MT-081 ring with a typed [`DiagEventCode::PanicCaught`] event so Palmistry sees a panic
/// marker even if the process then aborts. TYPED INTEGERS ONLY — there is no string field on
/// `DiagEvent`, so this carries NO message (the typed-allowlist invariant is structural). A silent
/// no-op when no ring writer is installed (headless/test); never panics.
fn signal_ring() {
    let thread_id = thread_id_to_u64(&std::thread::current());
    let monotonic_nanos = monotonic_now_nanos();
    crate::diagnostics::record_with(
        DiagEventCode::PanicCaught,
        DiagPhase::End,
        DiagSeverity::Error,
        thread_id,
        /* sequence_id  */ 0,
        /* counter_a    */ 0,
        /* counter_b    */ 0,
        /* metric_micros*/ 0,
        monotonic_nanos,
    );
}

#[cfg(test)]
mod tests {
    //! In-crate unit tests for the pure, ring-independent helpers (JSON escaping, atomic write,
    //! field collection, crash-dir portability). The end-to-end caught-panic + ring-marker proofs live
    //! in `tests/test_panic_hook.rs` (they install the real process-global hook + a real ring writer,
    //! which must not collide with the in-crate unit tests sharing the same process-global).

    use super::*;

    #[test]
    fn json_escape_handles_quotes_backslashes_and_controls() {
        assert_eq!(json_escape(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(json_escape("line1\nline2\ttab"), "line1\\nline2\\ttab");
        // A raw control char (0x01) becomes a \u escape so the record stays valid JSON.
        assert_eq!(json_escape("\u{0001}"), "\\u0001");
    }

    #[test]
    fn sanitize_token_strips_unsafe_chars() {
        assert_eq!(sanitize_token("ab-CD_12"), "ab-CD_12");
        assert_eq!(sanitize_token("a b/c\\d"), "a_b_c_d");
    }

    #[test]
    fn atomic_write_produces_complete_file_and_no_leftover_tmp() {
        let dir = std::env::temp_dir().join(format!(
            "handshake-panic-unit-{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::create_dir_all(&dir);
        let tmp = dir.join("rec.json.tmp");
        let fin = dir.join("rec.json");
        atomic_write(&tmp, &fin, b"{\"ok\":true}");
        let body = fs::read_to_string(&fin).expect("final record exists");
        assert_eq!(body, "{\"ok\":true}", "complete content, not truncated");
        assert!(!tmp.exists(), "temp file was renamed away (no leftover tmp)");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_to_unwritable_dir_does_not_panic() {
        // A path under a non-existent directory: the write fails, no record appears, and crucially the
        // function returns cleanly (best-effort) rather than panicking — this is the IO-failure path the
        // hook relies on to never turn a panic into a re-panic (RISK-003-2 / AC-003-3).
        let bogus = Path::new("this/dir/does/not/exist/anywhere/rec");
        atomic_write(
            &bogus.with_extension("json.tmp"),
            &bogus.with_extension("json"),
            b"x",
        );
        assert!(!bogus.with_extension("json").exists(), "no record on a bad path");
    }

    #[test]
    fn default_crash_dir_is_portable_not_hardcoded() {
        // The resolved crash dir ends with the portable handshake/crash suffix and is rooted under the
        // platform local-data dir (NOT a hardcoded drive-letter/user-profile literal). On any host with
        // a local-data dir this is Some; the suffix is what AC-003-5 asserts.
        if let Some(dir) = default_crash_dir() {
            assert!(dir.ends_with(Path::new("handshake").join("crash")));
        }
    }

    #[test]
    fn process_session_id_is_set_once_and_readable() {
        // First setter wins; a second set with a different id returns the FIRST (so the hook + ring agree
        // on one id). This uses the process-global OnceLock, so it is order-independent within the run.
        let first = set_process_session_id("unit-session-aaa").to_string();
        let second = set_process_session_id("unit-session-bbb").to_string();
        assert_eq!(first, second, "the first session id wins (hook + ring agree)");
        assert_eq!(process_session_id(), Some(first.as_str()));
    }
}
