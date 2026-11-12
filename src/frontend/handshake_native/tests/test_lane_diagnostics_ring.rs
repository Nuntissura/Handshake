use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use handshake_diag_ring::ring::{DiagRingReaderV2, DiagRingWriterV2};
use handshake_diag_ring::schema::DiagEventV2;
use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity, DEFAULT_CAPACITY};
use handshake_native::backend_client::SwarmLaneDiagnosticsClient;
use handshake_native::diagnostics::gui_events::{GuiDiagnosticOperation, GuiDiagnosticPhase};
use handshake_native::diagnostics::recorder::install_v2;
use handshake_native::swarm_lane_diagnostics::SwarmLaneDiagnosticsCell;

#[test]
#[ignore = "requires an admitted MT212 case output directory and a fresh test process"]
fn lane_diagnostics_pre_http_rejection_reaches_the_typed_ring_once() {
    let artifact_root = PathBuf::from(
        std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT").expect("bind the external artifact root"),
    )
    .canonicalize()
    .expect("resolve the external artifact root");
    let mt_root = artifact_root
        .join("WP-KERNEL-012")
        .join("MT-212")
        .canonicalize()
        .expect("resolve the MT212 artifact directory");
    let run_root = PathBuf::from(
        std::env::var_os("HANDSHAKE_CAPTURE_RUN_ROOT")
            .expect("bind the owned case output directory"),
    )
    .canonicalize()
    .expect("resolve the owned case output directory");
    assert!(mt_root.starts_with(&artifact_root));
    assert!(run_root != mt_root && run_root.starts_with(&mt_root) && run_root.is_dir());

    let scratch = run_root.join("lane-diagnostics-ring");
    fs::create_dir(&scratch).expect("exclusively reserve the case scratch directory");
    let path = scratch.join("lane-diagnostics.ring");
    let reservation = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("reserve a new ring backing file before the writer sizes it");
    let nonce = *uuid::Uuid::new_v4().as_bytes();
    let writer = DiagRingWriterV2::create(&path, DEFAULT_CAPACITY, nonce)
        .expect("create the Native recorder's shared V2 ring");
    drop(reservation);
    assert!(
        install_v2(writer),
        "the test binary must own a fresh recorder"
    );
    let reader = DiagRingReaderV2::open(&path, nonce).expect("map a separate V2 reader");
    assert!(reader.read_last_n(1).is_empty());

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("start the isolated client runtime");
    let client = SwarmLaneDiagnosticsClient::new("http://127.0.0.1:0", runtime.handle().clone());
    let delivery: SwarmLaneDiagnosticsCell = Arc::new(Mutex::new(None));
    runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(2),
            client.run_request_future("private-run-id-must-not-reach-the-ring", delivery.clone()),
        )
        .await
        .expect("deliver the pre-HTTP rejection");
    });
    assert_eq!(
        delivery
            .lock()
            .unwrap()
            .take()
            .expect("one client result")
            .expect_err("no account"),
        "http: Account login required",
    );

    let events = reader.read_last_n(DEFAULT_CAPACITY);
    assert_eq!(
        events.len(),
        1,
        "one actual client attempt must record one event"
    );
    let observed = events[0];
    assert_eq!(
        GuiDiagnosticOperation::SwarmLaneDiagnosticsFetch as u64,
        0x4755_4900_0000_0010
    );
    let expected = DiagEventV2::ordinary(DiagEvent::generic(
        DiagEventCode::Other,
        DiagPhase::Degraded,
        DiagSeverity::Warn,
        0,
        0,
        GuiDiagnosticOperation::SwarmLaneDiagnosticsFetch as u64,
        GuiDiagnosticPhase::Rejected as u64,
        0,
        observed.event.timestamp_nanos,
    ))
    .expect("ordinary mechanical event");
    assert!(
        observed == expected,
        "the rejected fetch must use only its typed fields and a zero V2 payload extension",
    );
    // The process-global writer unmaps on exit. The proof owner may remove only this scratch
    // directory after observing the test process exit and checking custody.
}
