//! MT-215 S8 conformance, outside `src/` so the heartbeat-slot scan of the product code does not read it (MT-215
//! `heartbeat_scan_routing_20261127`): two host session types, one launch path, one signed watcher argv. Reaches the
//! public API only. The ring is made for its nonce; nothing here writes the heartbeat slot, which the reader never reads.

use palmistry_launch::{
    control_socket_name, signed_watcher_argv_without_spawn, LaunchMarkerSink, LaunchSession, PalmistryLaunchSession,
};
use std::path::{Path, PathBuf};

/// A second host session type, standing in for any host that is not `LaunchSession`.
struct OtherHostSession {
    id: String,
    ring: PathBuf,
}

impl PalmistryLaunchSession for OtherHostSession {
    fn session_id(&self) -> &str {
        &self.id
    }
    fn ring_path(&self) -> &Path {
        &self.ring
    }
    fn marker_sink(&self) -> Option<LaunchMarkerSink> {
        None
    }
}

fn pair_value<'a>(argv: &'a [std::ffi::OsString], flag: &str) -> &'a std::ffi::OsString {
    let at = argv.iter().position(|a| a == flag).unwrap_or_else(|| panic!("{flag} missing from {argv:?}"));
    &argv[at + 1]
}

#[test]
fn two_hosts_build_the_same_signed_watcher_argv() {
    let dir = std::env::temp_dir().join(format!(
        "hsk-mt215-s8-conformance-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let session_uuid = uuid::Uuid::now_v7();
    let process_instance = uuid::Uuid::now_v7();
    let nonce = [0x66u8; 16];
    let ring = dir.join("conformance.ring");
    let writer =
        handshake_diag_ring::ring::DiagRingWriterV2::create(&ring, handshake_diag_ring::DEFAULT_CAPACITY, nonce)
            .unwrap();
    let lease = handshake_runtime_instance::acquire_embedded_runtime_instance_lease(
        session_uuid,
        "mt215-s8-conformance",
    )
    .unwrap();
    let control_socket = control_socket_name(&session_uuid.to_string());
    let exe = Path::new("palmistry-conformance-never-launched.exe");

    let shell = LaunchSession {
        session_id: session_uuid.to_string(),
        ring_path: ring.clone(),
        marker_sink: None,
    };
    let other = OtherHostSession { id: session_uuid.to_string(), ring: ring.clone() };
    let build = |session: &dyn PalmistryLaunchSession, ring: &Path, nonce: [u8; 16]| {
        signed_watcher_argv_without_spawn(
            exe,
            session,
            ring,
            &control_socket,
            &lease,
            process_instance,
            nonce,
            zeroize::Zeroizing::new(vec![0x77; 32]),
            [0x88; 32],
        )
        .unwrap()
    };
    let shell_argv = build(&shell, &ring, nonce);
    let other_argv = build(&other, &ring, nonce);
    assert_eq!(shell_argv, other_argv, "both hosts must build the identical watcher argv");

    // The vector carries the v2 nonce, the process instance and the runtime-instance lease.
    let descriptor = lease.descriptor();
    assert_eq!(pair_value(&shell_argv, "--session-id"), &std::ffi::OsString::from(session_uuid.to_string()));
    assert_eq!(pair_value(&shell_argv, "--ring-version"), &std::ffi::OsString::from("2"));
    assert_eq!(pair_value(&shell_argv, "--ring-nonce"), &std::ffi::OsString::from("66".repeat(16)));
    assert_eq!(
        pair_value(&shell_argv, "--signing-process-instance"),
        &std::ffi::OsString::from(process_instance.to_string())
    );
    assert_eq!(
        pair_value(&shell_argv, "--runtime-instance-id"),
        &std::ffi::OsString::from(descriptor.instance_id.to_string())
    );
    for flag in [
        "--runtime-instance-schema-id",
        "--runtime-host-scope-id",
        "--runtime-lease-protocol",
        "--runtime-lease-address",
        "--runtime-lease-port",
        "--parent-os-creation-time-100ns",
        "--parent-pid",
        "--ring-path",
        "--control-socket",
    ] {
        let _ = pair_value(&shell_argv, flag);
    }

    // Positive control: the comparison can fail (a different nonce must change the vector).
    let ring2 = dir.join("conformance-2.ring");
    let writer2 =
        handshake_diag_ring::ring::DiagRingWriterV2::create(&ring2, handshake_diag_ring::DEFAULT_CAPACITY, [0x55; 16])
            .unwrap();
    let other_nonce = build(&shell, &ring2, [0x55; 16]);
    assert_ne!(shell_argv, other_nonce);

    drop((writer, writer2));
    let _ = std::fs::remove_dir_all(&dir);
}
