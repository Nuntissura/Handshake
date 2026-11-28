//! The shell's Palmistry session without a watcher process, a window or a device (MT-214 S7 P4a, AC214-11): the ids, the
//! lease, the version 2 ring and the typed degrade. Rings are named by a `RingFile` that removes them; nothing here starts
//! a watcher.
#![cfg(windows)]

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

use std::io;
use std::time::Duration;

use handshake_diag_ring::ring::{DiagRingReader, DiagRingReaderV2};
use handshake_diag_ring::{
    DiagEventCode, DiagPhase, DiagSeverity, DEFAULT_CAPACITY, HEADER_SIZE, RING_MAGIC,
};
use handshake_shell::palmistry::{prepare, Degrade, Seams};
use palmistry_launch::PalmistryLaunchSession;

use limits::{RingFile, TimeLimit};

const LIMIT: Duration = Duration::from_secs(30);
const SCOPE: &str = "hsk-shell-session-tests";

fn scope_set() -> io::Result<String> {
    Ok(SCOPE.to_owned())
}

fn scope_unset() -> io::Result<String> {
    Err(io::Error::new(io::ErrorKind::NotFound, "no host scope"))
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
        },
    );
    let second = prepare(
        Some(second_file.path()),
        &Seams {
            host_scope: &scope_set,
        },
    );
    assert_ne!(
        first.inputs.session.session_id(),
        second.inputs.session.session_id()
    );
    assert_ne!(first.inputs.launch_nonce, second.inputs.launch_nonce);
}
