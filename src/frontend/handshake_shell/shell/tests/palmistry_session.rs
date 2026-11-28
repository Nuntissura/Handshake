//! The shell's Palmistry session without a watcher process, a window or a device (MT-214 S7 P4a, AC214-11): the ids, the
//! lease, the version 2 ring and the typed degrade. Rings are named by a `RingFile` that removes them; nothing here starts
//! a watcher.
#![cfg(windows)]

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use handshake_diag_ring::ring::{DiagRingReader, DiagRingReaderV2};
use handshake_diag_ring::{
    DiagEventCode, DiagPhase, DiagSeverity, DEFAULT_CAPACITY, HEADER_SIZE, RING_MAGIC,
};
use handshake_shell::palmistry::{classify, prepare, start_with, Degrade, LaunchState, Seams};
use handshake_shell::{ShutdownWatcher, WatcherAck};
use palmistry_launch::{signed_watcher_argv_without_spawn, LaunchSession, PalmistryLaunchSession};

use limits::{RingFile, TimeLimit};

const LIMIT: Duration = Duration::from_secs(30);
const SCOPE: &str = "hsk-shell-session-tests";

fn scope_set() -> io::Result<String> {
    Ok(SCOPE.to_owned())
}

fn scope_unset() -> io::Result<String> {
    Err(io::Error::new(io::ErrorKind::NotFound, "no host scope"))
}

fn exe_missing() -> io::Result<PathBuf> {
    Err(io::Error::new(io::ErrorKind::NotFound, "no watcher exe"))
}

/// A file that is not there, in the test target's own folder: the real launcher is asked to start it and cannot.
fn exe_not_there() -> io::Result<PathBuf> {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("hsk-s7-no-such-watcher.exe");
    assert!(
        !path.exists(),
        "the watcher that is not there must not be there"
    );
    Ok(path)
}

fn scope_blank() -> io::Result<String> {
    Ok("   ".to_owned())
}

/// What the ring's last events say about the Palmistry handshake: `(phase, severity)` of each.
fn handshakes(path: &std::path::Path, nonce: [u8; 16]) -> Vec<(u8, u8)> {
    let reader =
        DiagRingReaderV2::open(path, nonce).expect("the version 2 reader opens with the nonce");
    reader
        .read_last_n(16)
        .iter()
        .filter(|event| event.event.event_code == DiagEventCode::PalmistryHandshake.as_u16())
        .map(|event| (event.event.phase_marker, event.event.severity))
        .collect()
}

fn degraded() -> (u8, u8) {
    (DiagPhase::Degraded as u8, DiagSeverity::Warn as u8)
}

#[test]
fn no_host_scope_degrades_with_a_marker_and_no_lease() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("no-host-scope");
    let prepared = prepare(
        Some(file.path()),
        &Seams {
            host_scope: &scope_unset,
            resolve_exe: &exe_missing,
        },
    );
    assert_eq!(prepared.degrade, Some(Degrade::NoHostScope));
    assert!(prepared.lease.is_none());
    assert!(
        prepared.ring.is_some(),
        "the ring is made even though no watcher can be"
    );
    assert_eq!(
        handshakes(file.path(), prepared.inputs.launch_nonce),
        vec![degraded()]
    );
}

#[test]
fn a_blank_host_scope_gives_no_lease_and_a_marker() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("no-lease");
    let prepared = prepare(
        Some(file.path()),
        &Seams {
            host_scope: &scope_blank,
            resolve_exe: &exe_missing,
        },
    );
    assert_eq!(prepared.degrade, Some(Degrade::NoLease));
    assert!(prepared.lease.is_none());
    assert_eq!(
        handshakes(file.path(), prepared.inputs.launch_nonce),
        vec![degraded()]
    );
}

#[test]
fn a_ring_that_cannot_be_made_degrades_without_a_ring() {
    let _limit = TimeLimit::new(LIMIT);
    // A folder is not a ring file.
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let prepared = prepare(
        Some(folder),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    assert_eq!(prepared.degrade, Some(Degrade::NoRing));
    assert!(prepared.ring.is_none());
    assert!(
        prepared.lease.is_some(),
        "the lease was taken before the ring was tried"
    );
}

#[test]
fn a_good_start_has_a_lease_a_ring_and_no_degrade() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("good");
    let prepared = prepare(
        Some(file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    assert_eq!(prepared.degrade, None);
    assert!(prepared.ring.is_some());
    assert!(
        handshakes(file.path(), prepared.inputs.launch_nonce).is_empty(),
        "nothing is marked before a launch"
    );
}

#[test]
fn the_ring_is_a_version_two_ring_with_the_launch_nonce() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("layout");
    let prepared = prepare(
        Some(file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    let nonce = prepared.inputs.launch_nonce;
    assert_ne!(nonce, [0; 16]);
    let bytes = std::fs::read(file.path()).expect("the ring file reads");
    assert_eq!(bytes.len(), HEADER_SIZE + DEFAULT_CAPACITY as usize * 128);
    assert_eq!(
        u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        RING_MAGIC
    );
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        2,
        "ring version"
    );
    assert_eq!(
        u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        120,
        "record size"
    );
    assert_eq!(bytes[48..64], nonce, "the nonce is in the header");
    DiagRingReaderV2::open(file.path(), nonce).expect("the version 2 reader opens it");
    assert!(
        DiagRingReader::open(file.path()).is_err(),
        "the version 1 reader refuses it"
    );
    let mut other = nonce;
    other[0] ^= 0xff;
    assert!(
        DiagRingReaderV2::open(file.path(), other).is_err(),
        "another nonce does not open it"
    );
}

#[test]
fn the_session_ids_are_the_leases_and_the_launcher_accepts_them() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("ids");
    let prepared = prepare(
        Some(file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    let inputs = &prepared.inputs;
    let session =
        uuid::Uuid::parse_str(inputs.session.session_id()).expect("the session id is a uuid");
    assert!(!session.is_nil());
    assert!(!inputs.process_instance_id.is_nil());
    assert_ne!(
        inputs.process_instance_id, session,
        "the signed launch refuses equal ids"
    );
    assert_ne!(inputs.transport_public_key, [0; 32]);
    assert_eq!(inputs.signing_seed().len(), 32);
    assert_eq!(
        inputs.session.ring_path(),
        file.path(),
        "the launcher is told the ring the shell made"
    );
    assert_eq!(
        inputs.control_socket,
        palmistry_launch::control_socket_name(inputs.session.session_id())
    );
    let lease = prepared.lease.as_ref().expect("the lease");
    let descriptor = lease.descriptor();
    assert_eq!(
        descriptor.instance_id, session,
        "the lease is named by the session"
    );
    assert_eq!(descriptor.host_scope_id, SCOPE);
}

#[test]
fn the_launchers_marker_sink_is_empty() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("sink");
    let prepared = prepare(
        Some(file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    assert!(
        prepared.inputs.session.marker_sink().is_none(),
        "the ring writer is the heartbeat's alone"
    );
}

#[test]
fn two_starts_have_two_sessions_and_two_nonces() {
    let _limit = TimeLimit::new(LIMIT);
    let first_file = RingFile::new("first");
    let second_file = RingFile::new("second");
    let first = prepare(
        Some(first_file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    let second = prepare(
        Some(second_file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    assert_ne!(
        first.inputs.session.session_id(),
        second.inputs.session.session_id()
    );
    assert_ne!(first.inputs.launch_nonce, second.inputs.launch_nonce);
}

fn value(argv: &[OsString], flag: &str) -> OsString {
    let at = argv
        .iter()
        .position(|argument| argument == flag)
        .unwrap_or_else(|| panic!("{flag} is not in {argv:?}"));
    argv[at + 1].clone()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn a_watcher_exe_that_cannot_be_resolved_degrades_with_a_marker() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("exe-missing");
    let started = start_with(
        Some(file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_missing,
        },
    );
    assert_eq!(started.state, LaunchState::Degraded(Degrade::ExeNotFound));
    assert_eq!(started.state.name(), "degraded");
    assert_eq!(started.state.reason(), "exe_not_found");
    assert_eq!(started.failure, Some(io::ErrorKind::NotFound));
    assert!(!started.watcher.holds_watcher());
    assert!(started.lease.is_some());
    assert_eq!(
        handshakes(file.path(), started.inputs.launch_nonce),
        vec![degraded()]
    );
    assert_eq!(
        started.watcher.acknowledge_shutdown(),
        WatcherAck::NotConnected
    );
}

#[test]
fn a_launch_that_cannot_start_the_exe_degrades_and_the_error_is_not_found() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("spawn-failed");
    let started = start_with(
        Some(file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_not_there,
        },
    );
    assert_eq!(started.state, LaunchState::Degraded(Degrade::SpawnFailed));
    assert_eq!(started.state.reason(), "spawn_failed");
    // Not any error: the exe is what is missing. A refused input (zero nonce, ids, ring) would be another kind.
    assert_eq!(started.failure, Some(io::ErrorKind::NotFound));
    assert!(!started.watcher.holds_watcher(), "no handle, no watcher");
    assert!(
        started.lease.is_some(),
        "the lease stays held until the caller lets it go"
    );
    assert_eq!(
        handshakes(file.path(), started.inputs.launch_nonce),
        vec![degraded()]
    );
}

#[test]
fn a_degraded_start_before_the_launch_never_resolves_the_exe() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("no-resolve");
    let started = start_with(
        Some(file.path()),
        &Seams {
            host_scope: &scope_unset,
            resolve_exe: &exe_not_there,
        },
    );
    assert_eq!(started.state, LaunchState::Degraded(Degrade::NoHostScope));
    assert_eq!(started.failure, None);
    assert_eq!(
        handshakes(file.path(), started.inputs.launch_nonce).len(),
        1,
        "one marker, not two"
    );
}

#[test]
fn an_unconfirmed_handshake_is_not_a_confirmed_launch() {
    assert_eq!(classify(true), LaunchState::Launched);
    assert_eq!(classify(false), LaunchState::HandshakeUnconfirmed);
    assert_eq!(
        LaunchState::HandshakeUnconfirmed.name(),
        "handshake_unconfirmed"
    );
}

/// The launch inputs of a real start, through the launcher's validation and command builder (which spawn nothing), against
/// the launcher's own neutral session built from the same values.
#[test]
fn the_launch_inputs_make_the_launchers_argv_with_the_ring_nonce_and_path() {
    let _limit = TimeLimit::new(LIMIT);
    let file = RingFile::new("argv");
    let started = start_with(
        Some(file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_not_there,
        },
    );
    let inputs = &started.inputs;
    let lease = started.lease.as_ref().expect("the lease");
    let exe = exe_not_there().unwrap();
    let build = |session: &dyn PalmistryLaunchSession, nonce: [u8; 16]| {
        signed_watcher_argv_without_spawn(
            &exe,
            session,
            session.ring_path(),
            &inputs.control_socket,
            lease,
            inputs.process_instance_id,
            nonce,
            inputs.signing_seed(),
            inputs.transport_public_key,
        )
    };
    // The real inputs pass the launcher's validation (a zero nonce, a zero key or equal ids would not).
    let shell = build(&inputs.session, inputs.launch_nonce)
        .expect("the launcher accepts the shell's inputs");
    // The launcher's neutral session, built from the values and not from the shell's record.
    let neutral = LaunchSession {
        session_id: inputs.session.session_id().to_owned(),
        ring_path: file.path().to_path_buf(),
        marker_sink: None,
    };
    assert_eq!(
        shell,
        build(&neutral, inputs.launch_nonce).unwrap(),
        "the same inputs, the same vector"
    );

    let bytes = std::fs::read(file.path()).unwrap();
    assert_eq!(
        value(&shell, "--ring-nonce"),
        OsString::from(hex(&bytes[48..64])),
        "the argv carries the ring's nonce"
    );
    assert_eq!(
        value(&shell, "--ring-path"),
        file.path().as_os_str(),
        "the argv carries the ring the shell made"
    );
    assert_eq!(value(&shell, "--ring-version"), OsString::from("2"));
    assert_eq!(
        value(&shell, "--session-id"),
        OsString::from(inputs.session.session_id())
    );
    assert_eq!(
        value(&shell, "--runtime-instance-id"),
        OsString::from(inputs.session.session_id())
    );
    assert_eq!(
        value(&shell, "--signing-process-instance"),
        OsString::from(inputs.process_instance_id.to_string())
    );
    assert_eq!(
        value(&shell, "--control-socket"),
        OsString::from(&inputs.control_socket)
    );
    for flag in [
        "--parent-pid",
        "--parent-os-creation-time-100ns",
        "--runtime-lease-address",
        "--runtime-lease-port",
    ] {
        let _ = value(&shell, flag);
    }

    // Positive control: the comparison can fail. A nonce the ring does not hold is refused, and a second start has
    // another vector.
    assert!(
        build(&inputs.session, [0x55; 16]).is_err(),
        "a nonce the ring does not carry is refused"
    );
    let other_file = RingFile::new("argv-other");
    let other = start_with(
        Some(other_file.path()),
        &Seams {
            host_scope: &scope_set,
            resolve_exe: &exe_not_there,
        },
    );
    let other_argv = signed_watcher_argv_without_spawn(
        &exe,
        &other.inputs.session,
        other.inputs.session.ring_path(),
        &other.inputs.control_socket,
        other.lease.as_ref().unwrap(),
        other.inputs.process_instance_id,
        other.inputs.launch_nonce,
        other.inputs.signing_seed(),
        other.inputs.transport_public_key,
    )
    .unwrap();
    assert_ne!(shell, other_argv);
}
