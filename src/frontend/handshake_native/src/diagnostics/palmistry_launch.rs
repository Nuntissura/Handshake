//! WP-KERNEL-012 MT-094 (§6.13.3) Palmistry launcher, as the egui host sees it.
//!
//! The launcher itself (resolve, spawn, v2 ring nonce and runtime-instance lease binding, signed ready handshake,
//! child liveness, shutdown with acknowledgement) moved unchanged to the toolkit-free crate
//! `src/frontend/handshake_host_ports/palmistry_launch` (MT-215 S8, AC215-11), so both desktop hosts start the one
//! watcher through the one code path. This module keeps every previous path alive:
//!
//! - `pub use ::palmistry_launch::*` re-exports the whole launcher API under the old module path;
//! - [`DiagSession`] implements the launcher's [`PalmistryLaunchSession`] port and records the launcher markers
//!   through this host's recorder;
//! - the preinstalled-session slot (`main()` to `HandshakeApp::new`) stays here, because it holds a [`DiagSession`];
//! - the adoption methods stay here as [`PalmistryAdoptionExt`] over [`PalmistryHandle`], because they use
//!   `crate::internal_diagnostics` and `crate::local_account`; their state lives in the handle's host slot.
//!
//! HBR-QUIET, launch order, wire, signing, timing, ring layout and watcher behaviour are the launcher crate's and
//! are covered by its tests plus the source scans in `tests/test_palmistry_launch.rs`.

pub use ::palmistry_launch::*;

use std::sync::Mutex;

use super::DiagSession;

impl PalmistryLaunchSession for DiagSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn ring_path(&self) -> &std::path::Path {
        &self.ring_path
    }

    fn marker_sink(&self) -> Option<LaunchMarkerSink> {
        Some(record_launch_marker)
    }
}

/// The egui host's marker sink: the nine-field typed event the launcher always recorded here.
fn record_launch_marker(marker: LaunchMarker) {
    crate::diagnostics::record_with(
        marker.code,
        marker.phase,
        marker.severity,
        marker.thread_id,
        marker.sequence_id,
        marker.counter_a,
        marker.counter_b,
        marker.metric_micros,
        marker.timestamp_nanos,
    );
}

/// Backend adoption of the owned signed watcher, as methods on [`PalmistryHandle`]. The adoption operation lives
/// in the handle's host slot ([`PalmistryHandle::host_slot_mut`]); the launcher never reads or clears it.
#[allow(async_fn_in_trait)]
pub trait PalmistryAdoptionExt {
    fn prepare_adoption(
        &mut self,
        provenance: &crate::internal_diagnostics::AdoptedDiagnosticProvenance,
        account: std::sync::Arc<crate::local_account::AuthenticatedContext>,
        backend_base_url: &str,
        request: serde_json::Value,
    ) -> Result<uuid::Uuid, String>;

    fn prepare_adoption_read(
        &mut self,
        account: std::sync::Arc<crate::local_account::AuthenticatedContext>,
    ) -> Result<(), String>;

    async fn advance_adoption(
        &mut self,
        current_account: &std::sync::Arc<crate::local_account::AuthenticatedContext>,
    ) -> Result<crate::internal_diagnostics::PalmistryAdoptionStatus, String>;
}

fn adoption_state(
    slot: &mut Option<Box<dyn std::any::Any + Send>>,
) -> Option<&mut crate::internal_diagnostics::OwnedPalmistryAdoption> {
    slot.as_mut()
        .and_then(|state| state.downcast_mut::<crate::internal_diagnostics::OwnedPalmistryAdoption>())
}

impl PalmistryAdoptionExt for PalmistryHandle {
    fn prepare_adoption(
        &mut self,
        provenance: &crate::internal_diagnostics::AdoptedDiagnosticProvenance,
        account: std::sync::Arc<crate::local_account::AuthenticatedContext>,
        backend_base_url: &str,
        mut request: serde_json::Value,
    ) -> Result<uuid::Uuid, String> {
        if self.host_slot_mut().is_some() {
            return Err("Palmistry adoption already retains its original operation".into());
        }
        let (ready, creation_time) = self.registration_ready()
            .map(|(ready, creation_time)| (ready.clone(), creation_time))
            .map_err(|error| error.to_string())?;
        let socket_name = self.socket_name().to_string();
        let operation_id = self.adoption_operation_id();
        let process = self.retained_watcher_process()
            .ok_or("Missing retained Palmistry child image")?;
        let wire = request.as_object_mut().ok_or("Palmistry adoption request is not an object")?;
        wire.insert("ring".into(), serde_json::to_value(process.ring_path)
            .map_err(|_| "Original Palmistry ring path cannot be encoded")?);
        let adopt = wire.get_mut("adopt_existing").and_then(serde_json::Value::as_object_mut)
            .ok_or("Palmistry adoption requires the existing watcher wire evidence")?;
        adopt.insert("os_creation_time_100ns".into(), creation_time.get().into());
        adopt.insert("watcher_executable".into(), serde_json::to_value(process.executable)
            .map_err(|_| "Owned Palmistry executable path cannot be encoded")?);
        adopt.insert("watcher_executable_sha256".into(), process.executable_sha256.into());
        adopt.insert("control_socket".into(), socket_name.into());
        let operation = crate::internal_diagnostics::OwnedPalmistryAdoption::new(
            operation_id, ready, provenance.clone(), account,
            backend_base_url, request,
        )?;
        *self.host_slot_mut() = Some(Box::new(operation));
        Ok(operation_id)
    }

    fn prepare_adoption_read(
        &mut self,
        account: std::sync::Arc<crate::local_account::AuthenticatedContext>,
    ) -> Result<(), String> {
        adoption_state(self.host_slot_mut())
            .ok_or("Palmistry adoption has no original operation")?
            .prepare_read(account)
    }

    async fn advance_adoption(
        &mut self,
        current_account: &std::sync::Arc<crate::local_account::AuthenticatedContext>,
    ) -> Result<crate::internal_diagnostics::PalmistryAdoptionStatus, String> {
        let (slot, mut child) = self.host_slot_and_child_probe();
        let operation = adoption_state(slot)
            .ok_or("Palmistry adoption has no original operation")?;
        if child.has_exited().map_err(|_| "Owned watcher status is unavailable")? {
            operation.suspend();
            return Err("Original Palmistry child exited before adoption".into());
        }
        let status = operation.advance(current_account).await?;
        if child.has_exited().map_err(|_| "Owned watcher status is unavailable")? {
            operation.suspend();
            return Err("Original Palmistry child exited before adoption activation".into());
        }
        if status != crate::internal_diagnostics::PalmistryAdoptionStatus::Uncertain {
            operation.accept_positive(current_account)?;
        }
        Ok(status)
    }
}

// ----------------------------------------------------------------------------------------------------
// Preinstalled-session handoff (main() -> HandshakeApp::new).
//
// MT-094 creates the MT-081 ring + launches Palmistry in `main()` BEFORE `eframe::run_native` (so the
// whole kittest suite, which builds `HandshakeApp` directly, never spawns a palmistry child — the
// anti-leak rule). `HandshakeApp::new` then REUSES that already-created ring via this process-global slot
// instead of creating a SECOND ring. The kittest path (no `main()`) leaves the slot empty and `new`
// creates its own ring exactly as before.
// ----------------------------------------------------------------------------------------------------

static PREINSTALLED_DIAG_SESSION: Mutex<Option<DiagSession>> = Mutex::new(None);

/// Record the [`DiagSession`] `main()` already created + installed, so [`HandshakeApp::new`] reuses it
/// instead of creating a second ring.
///
/// [`HandshakeApp::new`]: crate::app::HandshakeApp::new
pub fn set_preinstalled_diag_session(session: DiagSession) {
    let mut slot = match PREINSTALLED_DIAG_SESSION.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    *slot = Some(session);
}

/// Take the preinstalled [`DiagSession`] (consuming it) if `main()` set one. Returns `None` in the
/// kittest path (no `main()`), so the caller creates its own ring.
pub fn take_preinstalled_diag_session() -> Option<DiagSession> {
    let mut slot = match PREINSTALLED_DIAG_SESSION.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    slot.take()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn preinstalled_session_round_trips_then_empties() {
        let session = DiagSession {
            session_id: "preinstall-test".to_string(),
            ring_path: PathBuf::from("/tmp/r.ring"),
        };
        set_preinstalled_diag_session(session.clone());
        assert_eq!(take_preinstalled_diag_session(), Some(session));
        assert_eq!(take_preinstalled_diag_session(), None, "taken once only");
    }

    /// The handle keeps the original handle's auto traits: `Send`, not `Sync`. The host adoption state it stores in
    /// the slot is `Send` and not `Sync` (its retained HTTP attempt is a `dyn Future + Send`).
    #[test]
    fn adoption_state_and_handle_auto_traits_are_send_not_sync() {
        trait AmbiguousIfSync<A> {
            fn some_item() {}
        }
        impl<T: ?Sized> AmbiguousIfSync<()> for T {}
        impl<T: ?Sized + Sync> AmbiguousIfSync<u8> for T {}
        fn is_send<T: Send + 'static>() {}
        is_send::<PalmistryHandle>();
        is_send::<crate::internal_diagnostics::OwnedPalmistryAdoption>();
        let _ = <PalmistryHandle as AmbiguousIfSync<_>>::some_item;
        let _ = <crate::internal_diagnostics::OwnedPalmistryAdoption as AmbiguousIfSync<_>>::some_item;
    }

    /// The marker sink this host hands the launcher records through the host recorder (the nine fields unchanged).
    #[test]
    fn diag_session_hands_the_launcher_its_recorder_sink() {
        let session = DiagSession {
            session_id: uuid::Uuid::from_u128(1).to_string(),
            ring_path: PathBuf::from("/tmp/r.ring"),
        };
        assert!(PalmistryLaunchSession::marker_sink(&session).is_some());
        assert_eq!(PalmistryLaunchSession::session_id(&session), session.session_id);
        assert_eq!(PalmistryLaunchSession::ring_path(&session), session.ring_path.as_path());
    }

    /// S8 conformance (AC215-11): the shell adapter (`LaunchSession`) and this host (`DiagSession`) reach the one
    /// `launch_palmistry_signed_at` validation and command builder, and build the identical watcher argv, including
    /// the v2 `--ring-nonce`, the `--runtime-instance-*` lease and `--signing-process-instance`. Nothing is launched.
    #[test]
    fn shell_adapter_and_egui_host_build_the_same_signed_watcher_argv() {
        let dir = std::env::temp_dir().join(format!(
            "hsk-mt215-s8-egui-conformance-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let session_uuid = uuid::Uuid::now_v7();
        let process_instance = uuid::Uuid::now_v7();
        let nonce = [0x66u8; 16];
        let ring = dir.join("conformance.ring");
        let writer = handshake_diag_ring::ring::DiagRingWriterV2::create(
            &ring,
            handshake_diag_ring::DEFAULT_CAPACITY,
            nonce,
        )
        .unwrap();
        writer.write_heartbeat(1, 2);
        let lease = handshake_runtime_instance::acquire_embedded_runtime_instance_lease(
            session_uuid,
            "mt215-s8-egui-conformance",
        )
        .unwrap();
        let control_socket = control_socket_name(&session_uuid.to_string());
        let exe = std::path::Path::new("palmistry-conformance-never-launched.exe");

        let shell = LaunchSession {
            session_id: session_uuid.to_string(),
            ring_path: ring.clone(),
            marker_sink: None,
        };
        let host = DiagSession {
            session_id: session_uuid.to_string(),
            ring_path: ring.clone(),
        };
        let build = |session: &dyn PalmistryLaunchSession| {
            signed_watcher_argv_without_spawn(
                exe,
                session,
                &ring,
                &control_socket,
                &lease,
                process_instance,
                nonce,
                zeroize::Zeroizing::new(vec![0x77; 32]),
                [0x88; 32],
            )
            .unwrap()
        };
        let shell_argv = build(&shell);
        let host_argv = build(&host);
        assert_eq!(shell_argv, host_argv, "the shell adapter and the egui host must build the identical argv");

        let value_of = |flag: &str| -> std::ffi::OsString {
            let at = host_argv
                .iter()
                .position(|arg| arg == flag)
                .unwrap_or_else(|| panic!("{flag} missing from {host_argv:?}"));
            host_argv[at + 1].clone()
        };
        assert_eq!(value_of("--ring-version"), std::ffi::OsString::from("2"));
        assert_eq!(value_of("--ring-nonce"), std::ffi::OsString::from("66".repeat(16)));
        assert_eq!(
            value_of("--signing-process-instance"),
            std::ffi::OsString::from(process_instance.to_string())
        );
        assert_eq!(
            value_of("--runtime-instance-id"),
            std::ffi::OsString::from(lease.descriptor().instance_id.to_string())
        );
        for flag in [
            "--runtime-instance-schema-id",
            "--runtime-host-scope-id",
            "--runtime-lease-protocol",
            "--runtime-lease-address",
            "--runtime-lease-port",
            "--parent-os-creation-time-100ns",
        ] {
            let _ = value_of(flag);
        }

        // The egui host's real launch goes through the one signed entry point (source scans of the call site and
        // of the launcher crate).
        let main_rs = include_str!("../main.rs");
        assert!(
            main_rs.contains("diagnostics::palmistry_launch::launch_palmistry_signed_at("),
            "the egui host's real launch must go through launch_palmistry_signed_at"
        );
        let launcher = include_str!("../../../handshake_host_ports/palmistry_launch/src/lib.rs");
        assert_eq!(
            launcher.matches("pub fn launch_palmistry_signed_at<").count(),
            1,
            "there is exactly one signed launch entry point"
        );

        drop(writer);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod signed_launch_evidence_tests {
    use ::palmistry_launch::{
        signed_watcher_challenge_bytes, verify_signed_watcher_reply, SignedWatcherIdentity,
    };
    use handshake_runtime_instance::{
        EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID, EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL,
    };

    #[test]
    fn signed_launch_verifier_rejects_forgery_replay_and_legacy_ack() {
        use ed25519_dalek::Signer;
        let identity = SignedWatcherIdentity {
            protocol_version: 1,
            session_id: uuid::Uuid::from_u128(1),
            process_instance_id: uuid::Uuid::from_u128(2),
            parent_pid: 3,
            watcher_pid: 4,
            ring_version: 2,
            record_size: 120,
            launch_nonce: [5; 16],
            runtime_instance_schema_id: EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID.to_string(),
            runtime_instance_id: uuid::Uuid::from_u128(1),
            runtime_host_scope_id: "signed-wire-unit-test".to_string(),
            runtime_lease_protocol: EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL.to_string(),
            runtime_lease_address: std::net::Ipv4Addr::LOCALHOST.into(),
            runtime_lease_port: 43210,
            parent_os_creation_time_100ns: 17,
        };
        let seed = zeroize::Zeroizing::new([7; 32]);
        let signer = ed25519_dalek::SigningKey::from_bytes(&seed);
        let verifier = signer.verifying_key().to_bytes();
        let challenge = [8; 32];
        let transport = [9; 32];
        let expected_identity = br#"{"protocol_version":1,"session_id":"00000000-0000-0000-0000-000000000001","process_instance_id":"00000000-0000-0000-0000-000000000002","parent_pid":3,"watcher_pid":4,"ring_version":2,"record_size":120,"launch_nonce":[5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5],"runtime_instance_schema_id":"hsk.embedded_runtime.instance@2","runtime_instance_id":"00000000-0000-0000-0000-000000000001","runtime_host_scope_id":"signed-wire-unit-test","runtime_lease_protocol":"hsk.embedded_runtime.loopback_udp_exclusive@1","runtime_lease_address":"127.0.0.1","runtime_lease_port":43210,"parent_os_creation_time_100ns":17}"#;
        let mut expected = b"handshake-current-watcher-provenance-v1\0".to_vec();
        expected.extend_from_slice(&(expected_identity.len() as u64).to_le_bytes());
        expected.extend_from_slice(expected_identity);
        expected.extend_from_slice(&[8; 32]);
        expected.extend_from_slice(&[9; 32]);
        assert_eq!(
            signed_watcher_challenge_bytes(&identity, challenge, transport).unwrap(),
            expected,
        );
        let signature = signer
            .sign(&signed_watcher_challenge_bytes(&identity, challenge, transport).unwrap())
            .to_bytes();
        let reply = serde_json::json!({"type":"ProvenanceReadyV1","identity":identity,"challenge":challenge,"transport_public_key":transport,"public_verifier":verifier,"signature":signature.to_vec()});
        let bytes = serde_json::to_vec(&reply).unwrap();
        assert_eq!(
            verify_signed_watcher_reply(&bytes, &identity, verifier, challenge, transport)
                .unwrap()
                .identity,
            identity
        );
        assert!(
            verify_signed_watcher_reply(&bytes, &identity, verifier, [10; 32], transport).is_err()
        );
        assert!(
            verify_signed_watcher_reply(&bytes, &identity, [11; 32], challenge, transport).is_err()
        );
        assert!(
            verify_signed_watcher_reply(&bytes, &identity, verifier, challenge, [12; 32]).is_err()
        );
        for field in ["parent_pid", "watcher_pid", "ring_version", "record_size"] {
            let mut forged = reply.clone();
            forged["identity"][field] = serde_json::json!(99);
            assert!(verify_signed_watcher_reply(
                &serde_json::to_vec(&forged).unwrap(),
                &identity,
                verifier,
                challenge,
                transport
            )
            .is_err());
        }
        for (field, changed) in [
            ("runtime_instance_schema_id", serde_json::json!("wrong-schema")),
            ("runtime_instance_id", serde_json::json!(uuid::Uuid::from_u128(99))),
            ("runtime_host_scope_id", serde_json::json!("wrong-host")),
            ("runtime_lease_protocol", serde_json::json!("wrong-protocol")),
            ("runtime_lease_address", serde_json::json!("127.0.0.2")),
            ("runtime_lease_port", serde_json::json!(43211)),
            ("parent_os_creation_time_100ns", serde_json::json!(18)),
        ] {
            let mut forged = reply.clone();
            forged["identity"][field] = changed;
            assert!(verify_signed_watcher_reply(
                &serde_json::to_vec(&forged).unwrap(),
                &identity,
                verifier,
                challenge,
                transport,
            )
            .is_err());
            let mut missing = reply.clone();
            missing["identity"].as_object_mut().unwrap().remove(field);
            assert!(verify_signed_watcher_reply(
                &serde_json::to_vec(&missing).unwrap(),
                &identity,
                verifier,
                challenge,
                transport,
            )
            .is_err());
        }
        let mut unknown = reply.clone();
        unknown["identity"]["unrecognized_runtime_identity"] = serde_json::json!(true);
        assert!(verify_signed_watcher_reply(
            &serde_json::to_vec(&unknown).unwrap(),
            &identity,
            verifier,
            challenge,
            transport,
        )
        .is_err());
        let mut forged = reply.clone();
        forged["signature"] = serde_json::json!(vec![0; 64]);
        assert!(verify_signed_watcher_reply(
            &serde_json::to_vec(&forged).unwrap(),
            &identity,
            verifier,
            challenge,
            transport
        )
        .is_err());
        forged["signature"] = serde_json::json!(vec![0; 63]);
        assert!(verify_signed_watcher_reply(
            &serde_json::to_vec(&forged).unwrap(),
            &identity,
            verifier,
            challenge,
            transport
        )
        .is_err());
        assert!(verify_signed_watcher_reply(
            br#"{"type":"Ack"}"#,
            &identity,
            verifier,
            challenge,
            transport
        )
        .is_err());
    }
}
