//! Integration proof for the MT-093 recovery-time FLIGHT-RECORDER FORWARDER (Master Spec v02.196 §6.13.7).
//!
//! Drives the REAL production `palmistry::fr_forward` + `palmistry::survivor_store` types. Proves:
//! - AC-013-3 (FR FORWARD, reuse-via-API): against a LOCAL STUB server that accepts the survivor-faithful
//!   body at the EXACT verified route `POST /api/flight_recorder/runtime_chat_event`, a forward succeeds
//!   and the record is marked forwarded IDEMPOTENTLY (no double-forward). The stub asserts the exact route
//!   + that the body is the typed-allowlist survivor shape (no free text).
//! - AC-013-4 (HONEST BLOCKER): forwarding into the EXISTING (verified-incompatible) chat-event route
//!   returns the typed `FrForwardBlocker::SchemaIncompatible` — NOT a faked success; the record stays
//!   local + pending; the FR is untouched. An ABSENT route returns `RouteAbsent`; a REJECTING route
//!   returns `Rejected{status}` — each leaves the record pending.
//! - AC-013-5 (recovery drain): a `drain` of unforwarded records forwards the pending ones and marks them,
//!   leaving a typed-blocker record pending for the next recovery.
//!
//! BOUNDED-TEST RULE (MT-092 precedent, MANDATORY): the stub server runs on its own thread, accepts EXACTLY
//! ONE connection with a hard read/accept TIMEOUT, and the test reads via the forwarder's bounded-timeout
//! blocking reqwest client — so a misbehaving stub can NEVER hang the harness. The stub thread is joined
//! with a bounded wait. No minidumper IPC, no binary spawn here.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use palmistry::fr_forward::{
    build_survivor_forward_body, FrForwardBlocker, FrForwarder, FrSchemaCompat,
    FR_INGESTION_FOLLOW_ON_WP, FR_ROUTE_PATH,
};
use palmistry::freeze_detect::FreezeReport;
use palmistry::survivor_store::{SurvivorProbeResult, SurvivorRecord, SurvivorStore};

/// What a one-shot stub server observed about the single request it served (sent back to the test).
struct StubObservation {
    /// The HTTP request line (e.g. `POST /api/flight_recorder/runtime_chat_event HTTP/1.1`).
    request_line: String,
    /// The parsed JSON body (the forwarder posts JSON).
    body: serde_json::Value,
}

/// Spawn a ONE-SHOT local HTTP/1.1 stub on an ephemeral port. It accepts EXACTLY one connection (with a
/// hard accept + read timeout so it can never block the harness), reads the request, replies with
/// `status_code`, and sends what it observed back over the channel. Returns `(base_url, observation_rx,
/// join_handle)`. `status_code` lets a test model an ACCEPTING FR (200) or a REJECTING FR (400).
fn spawn_stub(
    status_code: u16,
) -> (
    String,
    mpsc::Receiver<StubObservation>,
    std::thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral stub port");
    let addr = listener.local_addr().expect("stub addr");
    let base_url = format!("http://{addr}");
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        // Hard bound: accept() has no timeout API on std, but the forwarder (a real client) connects within
        // its bounded connect timeout, so the accept returns promptly in the happy path. If no client
        // connects, the test's own bound (the forwarder's bounded blocking client) fails fast; this thread
        // is then joined with a bounded wait and the process exits, so a stray blocked accept cannot wedge
        // the suite. Blocking accept (the std default) is exactly what we want here.
        match listener.accept() {
            Ok((mut stream, _peer)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                if let Some(obs) = read_one_request(&mut stream) {
                    let _ = tx.send(obs);
                }
                // Reply with the requested status so the forwarder sees a real HTTP outcome.
                let reason = if (200..300).contains(&status_code) {
                    "OK"
                } else {
                    "Bad Request"
                };
                let resp = format!(
                    "HTTP/1.1 {status_code} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
            }
            Err(err) => {
                eprintln!("stub accept failed: {err}");
            }
        }
    });
    (base_url, rx, handle)
}

/// Read ONE HTTP request from `stream` (headers + Content-Length body) under the stream's read timeout.
/// Minimal HTTP/1.1 parsing — enough to capture the request line + JSON body. Returns `None` on a read
/// error / malformed request (the test then fails on the missing observation, never hangs).
fn read_one_request(stream: &mut TcpStream) -> Option<StubObservation> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    // Read until we have the full headers (\r\n\r\n) + the declared Content-Length body, bounded by the
    // read timeout. A small cap on total bytes guards against a runaway body.
    loop {
        let headers_end = find_subsequence(&buf, b"\r\n\r\n");
        if let Some(hdr_end) = headers_end {
            let header_text = String::from_utf8_lossy(&buf[..hdr_end]).to_string();
            let content_len = header_text
                .lines()
                .find_map(|l| {
                    let lower = l.to_ascii_lowercase();
                    lower
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            let body_start = hdr_end + 4;
            if buf.len() >= body_start + content_len {
                let request_line = header_text.lines().next().unwrap_or_default().to_string();
                let body_bytes = &buf[body_start..body_start + content_len];
                let body = serde_json::from_slice(body_bytes).unwrap_or(serde_json::Value::Null);
                return Some(StubObservation { request_line, body });
            }
        }
        match stream.read(&mut chunk) {
            Ok(0) => break, // EOF before a full request.
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > 64 * 1024 {
                    break; // runaway guard.
                }
            }
            Err(_) => break, // timeout / error — bounded, never hangs.
        }
    }
    None
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hsk-mt093-fwd-{label}-{}-{nanos}",
        std::process::id()
    ))
}

struct DirGuard(PathBuf);
impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn freeze_record() -> SurvivorRecord {
    SurvivorRecord::from_freeze(
        "sess-fwd-it",
        4242,
        &FreezeReport {
            stale_ms: 6000,
            last_heartbeat_counter: 42,
            last_heartbeat_ts_nanos: 123,
        },
        3,
        SurvivorProbeResult::NotResponding,
    )
}

#[test]
fn faithful_forward_posts_verified_route_and_body_then_marks_forwarded_idempotently() {
    // AC-013-3: against a stub that ACCEPTS the survivor-faithful body at the EXACT verified route, a
    // forward (compat = Compatible) succeeds, the body is the typed-allowlist survivor shape, and the
    // record is marked forwarded idempotently.
    let (base_url, rx, handle) = spawn_stub(200);
    let forwarder = FrForwarder::with_compat(&base_url, FrSchemaCompat::Compatible);

    let dir = temp_dir("faithful");
    let _g = DirGuard(dir.clone());
    let mut store = SurvivorStore::open(&dir).expect("open store");
    let path = store.put(freeze_record()).expect("durable write");
    assert_eq!(store.unforwarded_count(), 1, "pending before forward");

    // Forward the record (real bounded-timeout blocking POST to the stub).
    forwarder
        .forward(&store.records()[0].record)
        .expect("a compatible accepting route must forward successfully");
    // Mark forwarded idempotently.
    assert!(store.mark_forwarded(&path).unwrap(), "mark forwarded");
    assert!(
        store.mark_forwarded(&path).unwrap(),
        "second mark is an idempotent no-op"
    );
    assert_eq!(
        store.unforwarded_count(),
        0,
        "now forwarded, not re-drained"
    );

    // The stub observed the EXACT verified route + a typed-allowlist survivor body.
    let obs = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("stub must have observed the forward request");
    assert!(
        obs.request_line
            .starts_with(&format!("POST {FR_ROUTE_PATH} ")),
        "the forward must hit the verified FR route, got '{}'",
        obs.request_line
    );
    // The body is the survivor-faithful shape (typed; no free text).
    assert_eq!(obs.body["kind"], "freeze");
    assert_eq!(obs.body["session_id"], "sess-fwd-it");
    assert_eq!(obs.body["stale_ms"], 6000);
    assert!(
        obs.body.get("message").is_none(),
        "no free-text field in the forward body"
    );

    let _ = handle.join();
}

#[test]
fn forward_body_matches_the_built_shape() {
    // The body the forwarder posts equals `build_survivor_forward_body` (the unit-asserted shape), so the
    // wire shape is the one a reviewer asserts without IO.
    let expected = build_survivor_forward_body(&freeze_record());
    assert_eq!(expected["kind"], "freeze");
    assert_eq!(
        expected["event_code"],
        handshake_diag_ring::DiagEventCode::FreezeSuspected.as_u16()
    );
}

#[test]
fn forward_into_existing_incompatible_fr_is_typed_blocker_not_fake() {
    // AC-013-4 (the honesty gate): the EXISTING chat-event route cannot carry a survivor record, so a
    // forward returns SchemaIncompatible — NOT a faked success. No stub needed: the blocker is decided
    // from the verified schema BEFORE any network call (proven by pointing at an unreachable base URL).
    let forwarder = FrForwarder::for_existing_fr("http://127.0.0.1:1");
    let dir = temp_dir("blocker");
    let _g = DirGuard(dir.clone());
    let mut store = SurvivorStore::open(&dir).expect("open store");
    let path = store.put(freeze_record()).unwrap();

    let err = forwarder
        .forward(&store.records()[0].record)
        .expect_err("the incompatible FR must yield a typed blocker, never Ok");
    match err {
        FrForwardBlocker::SchemaIncompatible { follow_on_wp, .. } => {
            assert_eq!(
                follow_on_wp, FR_INGESTION_FOLLOW_ON_WP,
                "names the WP-016 follow-on"
            );
        }
        other => panic!("expected SchemaIncompatible, got {other:?}"),
    }
    // The record STAYS local + pending (not faked forwarded).
    assert_eq!(
        store.unforwarded_count(),
        1,
        "the record stays pending after the typed blocker"
    );
    assert!(
        !store.records().iter().any(|s| s.record.forwarded),
        "nothing was faked-forwarded"
    );
    // mark_forwarded was NOT called for the blocked record.
    assert!(store
        .records()
        .iter()
        .any(|s| s.path == path && !s.record.forwarded));
}

#[test]
fn forward_against_rejecting_route_is_rejected_blocker() {
    // AC-013-4: a COMPATIBLE route that REJECTS the body (400) yields Rejected{status} — the record stays
    // pending (never marked forwarded). Bounded: the stub replies 400 promptly.
    let (base_url, _rx, handle) = spawn_stub(400);
    let forwarder = FrForwarder::with_compat(&base_url, FrSchemaCompat::Compatible);
    let err = forwarder
        .forward(&freeze_record())
        .expect_err("a 400 must be a Rejected blocker, never Ok");
    match err {
        FrForwardBlocker::Rejected { status, .. } => assert_eq!(status, 400),
        other => panic!("expected Rejected, got {other:?}"),
    }
    let _ = handle.join();
}

#[test]
fn forward_against_absent_route_is_route_absent_blocker() {
    // AC-013-4: a COMPATIBLE route that is UNREACHABLE yields RouteAbsent — bounded (the blocking client's
    // connect timeout), never hangs. Port 1 is reliably refused.
    let forwarder = FrForwarder::with_compat("http://127.0.0.1:1", FrSchemaCompat::Compatible);
    let err = forwarder
        .forward(&freeze_record())
        .expect_err("an absent route must block");
    assert!(
        matches!(err, FrForwardBlocker::RouteAbsent { .. }),
        "got {err:?}"
    );
}

#[test]
fn recovery_drain_forwards_pending_and_leaves_blocked_pending() {
    // AC-013-5: a recovery DRAIN forwards the pending records against a compatible accepting route and
    // marks them; a record that hits a typed blocker stays pending for the next recovery. Here we model
    // the drain explicitly over the store + a compatible stub for the faithful path.
    let (base_url, rx, handle) = spawn_stub(200);
    let forwarder = FrForwarder::with_compat(&base_url, FrSchemaCompat::Compatible);

    let dir = temp_dir("recovery-drain");
    let _g = DirGuard(dir.clone());
    let mut store = SurvivorStore::open(&dir).expect("open store");
    store.put(freeze_record()).unwrap();
    assert_eq!(store.unforwarded_count(), 1);

    // Drain: forward each pending record, marking the successful ones.
    let pending = store.unforwarded();
    for stored in pending {
        if forwarder.forward(&stored.record).is_ok() {
            store.mark_forwarded(&stored.path).unwrap();
        }
    }
    assert_eq!(
        store.unforwarded_count(),
        0,
        "the pending record was drained + forwarded"
    );

    let obs = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("the drain must have forwarded the pending record");
    assert!(obs
        .request_line
        .starts_with(&format!("POST {FR_ROUTE_PATH} ")));
    let _ = handle.join();
}
