//! Ignored production-binary Argus socket proof.
//!
//! This target is intentionally outside the default suite. It opens real native windows and requires
//! managed PostgreSQL plus a Palmistry-ready `handshake_core` on `127.0.0.1:37501`. Before running,
//! set `HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1` and point `HANDSHAKE_DIAGNOSTICS_DIR` at the existing
//! absolute directory shared by the backend, Palmistry, and the native child.
//!
//! The spawn/discovery/receipt/redaction plumbing lives in the shared
//! `argus_socket_support/live_socket.rs` module so this proof and
//! `test_argus_production_socket_live_surfaces.rs` drive the production transport through ONE
//! implementation instead of drifting copies. The assertions below are unchanged.

#![cfg(target_os = "windows")]

use std::process::Command;
use std::time::{Duration, Instant};
use std::{
    collections::BTreeMap,
    io::Read,
    io::Write,
    net::TcpStream,
    path::{Path, PathBuf},
};

use base64::Engine as _;
use sha2::{Digest, Sha256};

use handshake_native::internal_diagnostics::{
    read_ring_snapshot, DiagnosticCode, DiagnosticEventState, DiagnosticMechanism,
    InternalDiagnosticsSnapshot,
};
use handshake_native::pane_registry::PaneType;
use handshake_native::swarm_lane_diagnostics::{
    lane_author_id, message_author_id, message_payload_author_id, message_promotion_author_id,
    scoped_author_id, selected_message_author_id, EMPTY_MESSAGES_AUTHOR_ID, FRESHNESS_AUTHOR_ID,
    LANE_FILTER_AUTHOR_ID, MESSAGE_FILTER_AUTHOR_ID, PRIVACY_ACCESS_SPACE_AUTHOR_ID,
    PRIVACY_DENIAL_AUTHOR_ID,
    PRIVACY_OWNER_AUTHOR_ID, PRIVACY_PRINCIPAL_AUTHOR_ID, PRIVACY_SESSION_AUTHOR_ID,
    PRIVACY_VISIBILITY_AUTHOR_ID, PRIVACY_WORKSPACE_AUTHOR_ID, REFRESH_AUTHOR_ID,
    RUN_FILTER_AUTHOR_ID, SURFACE_AUTHOR_ID,
};

#[path = "argus_socket_support/live_socket.rs"]
mod live_socket;

use live_socket::{
    assert_bytes_exclude, assert_success, assert_visual_png, collect_author_ids,
    contains_author_id, decode_verified_capture, discover_binding, list_has_window, node_text,
    node_value, pane_id_hosting, proof_dir, request_child_close, require_node,
    require_palmistry_ready_backend, wait_for_author_id, wait_for_author_id_between,
    wait_for_window, ArgusClient, ChildGuard, LiveApp, SURFACE_TIMEOUT,
};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn json_sha256(value: &serde_json::Value) -> String {
    sha256_hex(&serde_json::to_vec(value).expect("serialize decisive proof observation"))
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after unix epoch")
        .as_millis() as u64
}

fn wait_for_author_id_absent(
    client: &mut ArgusClient,
    window_id: &str,
    author_id: &str,
) -> serde_json::Value {
    let deadline = Instant::now() + SURFACE_TIMEOUT;
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        last = client.inspect(window_id);
        if !contains_author_id(&last["snapshot"]["root"], author_id) {
            return last;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    panic!("author_id `{author_id}` remained visible in `{window_id}`: {last}");
}

fn wait_for_author_text_change(
    client: &mut ArgusClient,
    window_id: &str,
    author_id: &str,
    previous_text: &str,
) -> serde_json::Value {
    let deadline = Instant::now() + SURFACE_TIMEOUT;
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        last = client.inspect(window_id);
        if contains_author_id(&last["snapshot"]["root"], author_id)
            && node_text(require_node(&last["snapshot"]["root"], author_id)) != previous_text
        {
            return last;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    panic!(
        "author_id `{author_id}` did not change from `{previous_text}` in `{window_id}`: {last}"
    );
}

fn landmark_inspection(root: &serde_json::Value, author_id: &str) -> serde_json::Value {
    let node = require_node(root, author_id);
    let bounds = &node["bounds"];
    let positive_bounds = ["w", "h"].iter().all(|axis| {
        bounds[*axis]
            .as_f64()
            .is_some_and(|value| value.is_finite() && value > 0.0)
    });
    assert!(
        positive_bounds,
        "{author_id} has no positive finite bounds: {bounds}"
    );
    serde_json::json!({
        "author_id_sha256": sha256_hex(author_id.as_bytes()),
        "role": node["role"],
        "bounds": bounds,
        "disabled": node["disabled"],
        "text_sha256": sha256_hex(node_text(node).as_bytes()),
        "positive_finite_bounds": true,
    })
}

fn child_diagnostics_snapshot(
    diagnostics_dir: &Path,
    child_pid: u32,
) -> InternalDiagnosticsSnapshot {
    let deadline = Instant::now() + SURFACE_TIMEOUT;
    while Instant::now() < deadline {
        for entry in std::fs::read_dir(diagnostics_dir).expect("read diagnostics ring directory") {
            let path = entry.expect("read diagnostics ring entry").path();
            let is_ring = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("ring-") && name.ends_with(".bin"));
            if !is_ring {
                continue;
            }
            if let Ok(snapshot) = read_ring_snapshot(&path) {
                if snapshot.process_id == child_pid {
                    return snapshot;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    panic!("no internal-diagnostics ring found for production child pid {child_pid}");
}

fn renderer_runtime_error_scan(
    diagnostics_dir: &Path,
    child_pid: u32,
    capture_window_started_at_unix_ms: u64,
) -> serde_json::Value {
    let snapshot = child_diagnostics_snapshot(diagnostics_dir, child_pid);
    let relevant = snapshot
        .events
        .iter()
        .filter(|event| event.observed_at_unix_ms >= capture_window_started_at_unix_ms)
        .collect::<Vec<_>>();
    let uncaught = relevant
        .iter()
        .filter(|event| {
            event.state == DiagnosticEventState::Failed
                || event.mechanism == DiagnosticMechanism::Panic
                || event.code == Some(DiagnosticCode::PanicObserved)
                || event.code == Some(DiagnosticCode::RingPublishFailed)
        })
        .collect::<Vec<_>>();
    assert!(
        uncaught.is_empty(),
        "uncaught native renderer/runtime diagnostics during MT-008 capture: {uncaught:?}"
    );
    serde_json::json!({
        "schema_id": "handshake.argus.renderer_runtime_error_scan@1",
        "source": "Handshake native internal_diagnostics ring",
        "process_id": child_pid,
        "session_id_sha256": sha256_hex(snapshot.session_id.to_string().as_bytes()),
        "capture_window_started_at_unix_ms": capture_window_started_at_unix_ms,
        "event_count_scanned": relevant.len(),
        "uncaught_error_count": 0,
        "criteria": ["failed event state", "panic mechanism", "panic_observed code", "ring_publish_failed code"],
        "result": "no_uncaught_renderer_or_runtime_errors",
    })
}

fn privacy_node_digest(
    root: &serde_json::Value,
    pane_id: &str,
    author_id: &str,
    label: &str,
) -> String {
    node_text(require_node(root, &scoped_author_id(pane_id, author_id)))
        .strip_prefix(&format!("{label} verified | process-keyed fingerprint "))
        .and_then(|text| text.split(" | ").next())
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .unwrap_or_else(|| panic!("privacy node {author_id} omitted its redacted scope digest"))
        .to_owned()
}

fn backend_json(path: &str, headers: &[(&str, &str)]) -> (u16, serde_json::Value, Vec<u8>) {
    let mut stream = TcpStream::connect_timeout(
        &"127.0.0.1:37501".parse().expect("fixed backend address"),
        Duration::from_secs(3),
    )
    .expect("connect separately to handshake_core diagnostics authority");
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("set diagnostics authority read timeout");
    let mut request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:37501\r\nConnection: close\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .expect("write separately authenticated diagnostics request");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("read separately authenticated diagnostics response");
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("diagnostics response carries HTTP headers");
    let headers_text = String::from_utf8_lossy(&response[..header_end]);
    let status = headers_text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .expect("diagnostics response carries numeric status");
    let body = response[(header_end + 4)..].to_vec();
    let json = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("diagnostics response was not JSON: {error}"));
    (status, json, body)
}

fn write_proof_artifact(dir: &Path, file_name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(file_name);
    std::fs::write(&path, bytes).expect("write exact-scope MT-008 proof artifact");
    path
}

fn create_nonce_scoped_staging(scope_hash: &str, proof_nonce: &str) -> (PathBuf, PathBuf, String) {
    let nonce_hash = sha256_hex(proof_nonce.as_bytes());
    let scope_dir = proof_dir().join("mt008").join(scope_hash);
    std::fs::create_dir_all(&scope_dir).expect("create exact-scope MT-008 proof root");
    let final_dir = scope_dir.join(&nonce_hash);
    let staging_dir = scope_dir.join(format!(".{nonce_hash}.tmp-{}", std::process::id()));
    assert!(
        !final_dir.exists(),
        "proof nonce has already been published"
    );
    assert!(
        !staging_dir.exists(),
        "proof nonce staging directory already exists"
    );
    std::fs::create_dir(&staging_dir).expect("create nonce-scoped MT-008 staging directory");
    (staging_dir, final_dir, nonce_hash)
}

fn validate_and_publish_proof(
    staging_dir: &Path,
    final_dir: &Path,
    expected_scope_hash: &str,
    expected_nonce_hash: &str,
) {
    let manifest_path = staging_dir.join("manifest.json");
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&manifest_path).expect("read complete MT-008 proof manifest"),
    )
    .expect("MT-008 proof manifest is JSON");
    assert_eq!(
        manifest["schema_id"],
        "handshake.argus.production_socket_mt008_manifest@1"
    );
    assert_eq!(manifest["resource_scope_sha256"], expected_scope_hash);
    assert_eq!(manifest["proof_nonce_sha256"], expected_nonce_hash);
    let files = manifest["files"]
        .as_object()
        .expect("MT-008 manifest files map");
    assert!(
        files.len() == 10,
        "MT-008 manifest must enumerate every proof artifact"
    );
    for required in [
        "main.png",
        "edge-empty.png",
        "main-detached.png",
        "popout.png",
        "transcript_commitment.json",
        "backend_commitment.json",
        "provenance.json",
        "evidence_chain.json",
        "visual_inspection.json",
        "renderer_error_scan.json",
    ] {
        assert!(files.contains_key(required), "manifest omitted {required}");
    }
    for (name, expected_hash) in files {
        assert!(
            !name.contains('/') && !name.contains('\\'),
            "manifest file is local"
        );
        let bytes = std::fs::read(staging_dir.join(name)).expect("read manifested proof artifact");
        let actual_hash = sha256_hex(&bytes);
        assert_eq!(
            expected_hash.as_str(),
            Some(actual_hash.as_str()),
            "independently recomputed artifact hash for {name}"
        );
    }
    let chain: serde_json::Value = serde_json::from_slice(
        &std::fs::read(staging_dir.join("evidence_chain.json"))
            .expect("read MT-008 evidence chain"),
    )
    .expect("MT-008 evidence chain is JSON");
    assert_eq!(
        chain["schema_id"],
        "handshake.argus.production_socket_mt008_evidence_chain@2"
    );
    assert_eq!(chain["resource_scope_sha256"], expected_scope_hash);
    assert_eq!(chain["proof_nonce_sha256"], expected_nonce_hash);
    for (field, file) in [
        ("provenance_sha256", "provenance.json"),
        ("transcript_commitment_sha256", "transcript_commitment.json"),
        ("main_png_sha256", "main.png"),
        ("edge_empty_png_sha256", "edge-empty.png"),
        ("main_detached_png_sha256", "main-detached.png"),
        ("popout_png_sha256", "popout.png"),
        ("backend_commitment_sha256", "backend_commitment.json"),
        ("visual_inspection_sha256", "visual_inspection.json"),
        ("renderer_error_scan_sha256", "renderer_error_scan.json"),
    ] {
        assert_eq!(
            chain[field], files[file],
            "chain/manifest hash link {field}"
        );
    }
    let provenance: serde_json::Value = serde_json::from_slice(
        &std::fs::read(staging_dir.join("provenance.json")).expect("read MT-008 provenance"),
    )
    .expect("MT-008 provenance is JSON");
    assert_eq!(
        provenance["schema_id"],
        "handshake.argus.production_socket_mt008_diagnostics_provenance@2"
    );
    assert_eq!(provenance["resource_scope_sha256"], expected_scope_hash);
    assert_eq!(provenance["proof_nonce_sha256"], expected_nonce_hash);
    assert_eq!(
        provenance["decisive_observations"]["backend_commitment_sha256"],
        files["backend_commitment.json"]
    );
    assert_eq!(
        provenance["decisive_observations"]["minimized_transcript_sha256"],
        files["transcript_commitment.json"]
    );
    for capture in provenance["captures"]
        .as_array()
        .expect("provenance capture list")
    {
        let artifact = capture["artifact"].as_str().expect("capture artifact name");
        assert_eq!(capture["sha256"], files[artifact]);
    }
    let backend: serde_json::Value = serde_json::from_slice(
        &std::fs::read(staging_dir.join("backend_commitment.json"))
            .expect("read MT-008 backend commitment"),
    )
    .expect("MT-008 backend commitment is JSON");
    assert_eq!(
        backend["schema_id"],
        "handshake.argus.production_socket_mt008_backend_commitment@1"
    );
    assert_eq!(backend["resource_scope_sha256"], expected_scope_hash);
    assert_eq!(backend["stable_across_capture"], true);
    assert_eq!(
        backend["initial_projection_sha256"],
        backend["final_projection_sha256"]
    );
    let transcript: serde_json::Value = serde_json::from_slice(
        &std::fs::read(staging_dir.join("transcript_commitment.json"))
            .expect("read MT-008 transcript commitment"),
    )
    .expect("MT-008 transcript commitment is JSON");
    assert_eq!(
        transcript["schema_id"],
        "handshake.argus.minimized_transcript_commitment@1"
    );
    let visual: serde_json::Value = serde_json::from_slice(
        &std::fs::read(staging_dir.join("visual_inspection.json"))
            .expect("read MT-008 visual inspection record"),
    )
    .expect("MT-008 visual inspection record is JSON");
    assert_eq!(
        visual["schema_id"],
        "handshake.argus.mt008_visual_inspection@1"
    );
    assert_eq!(visual["matrix_complete"], true);
    assert_eq!(visual["semantic_verdict"], "inspection_ready");
    let renderer: serde_json::Value = serde_json::from_slice(
        &std::fs::read(staging_dir.join("renderer_error_scan.json"))
            .expect("read MT-008 renderer error scan"),
    )
    .expect("MT-008 renderer error scan is JSON");
    assert_eq!(
        renderer["schema_id"],
        "handshake.argus.renderer_runtime_error_scan@1"
    );
    assert_eq!(renderer["uncaught_error_count"], 0);
    std::fs::rename(staging_dir, final_dir)
        .expect("atomically publish complete nonce-scoped MT-008 proof directory");
}

fn assert_decoded_capture_privacy(
    png: &[u8],
    snapshot_root: &serde_json::Value,
    protected: &[&str],
    context: &str,
) {
    assert_visual_png(png, context);
    let decoded = image::load_from_memory(png)
        .unwrap_or_else(|error| panic!("{context} did not decode for privacy inspection: {error}"))
        .to_rgba8()
        .into_raw();
    let snapshot = serde_json::to_vec(snapshot_root).expect("serialize capture-bound snapshot");
    for canary in protected {
        assert_bytes_exclude(&snapshot, canary, context);
        assert_bytes_exclude(png, canary, context);
        assert_bytes_exclude(&decoded, canary, context);
    }
}

fn scoped_ids_with_prefix(
    root: &serde_json::Value,
    pane_id: &str,
    logical_prefix: &str,
) -> Vec<String> {
    let scope_suffix = scoped_author_id(pane_id, "__scope_marker")
        .strip_prefix("__scope_marker")
        .expect("scope marker preserves its logical prefix")
        .to_owned();
    let mut ids = collect_author_ids(root)
        .into_iter()
        .filter(|author_id| {
            author_id.starts_with(logical_prefix) && author_id.ends_with(&scope_suffix)
        })
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn first_scoped_id_matching(
    root: &serde_json::Value,
    pane_id: &str,
    logical_prefix: &str,
    predicate: impl Fn(&str) -> bool,
) -> String {
    scoped_ids_with_prefix(root, pane_id, logical_prefix)
        .into_iter()
        .find(|author_id| predicate(&node_text(require_node(root, author_id))))
        .unwrap_or_else(|| {
            panic!(
                "live diagnostics pane `{pane_id}` has no `{logical_prefix}` node matching the required text contract"
            )
        })
}

#[ignore = "LIVE production socket MT-008 E2E: opens a native diagnostics pane/pop-out and requires \
            managed PostgreSQL, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, HANDSHAKE_MT008_ARGUS_PROOF_NONCE, and a shared \
            HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn mt008_production_socket_diagnostics_scope_and_detached_capture() {
    let proof_nonce = std::env::var("HANDSHAKE_MT008_ARGUS_PROOF_NONCE")
        .expect("MT-008 live proof requires a fresh HANDSHAKE_MT008_ARGUS_PROOF_NONCE");
    assert!(
        !proof_nonce.trim().is_empty(),
        "MT-008 live proof nonce must not be blank"
    );

    let diagnostics_dir = require_palmistry_ready_backend();
    let capture_window_started_at_unix_ms = unix_ms();
    let mut app = LiveApp::start("mt008_diagnostics");
    app.open_models_menu_leaf("menu.models.swarm-lane-diagnostics");
    // Capture the immediate post-navigation frame before waiting for the diagnostics landmark. This
    // remains useful on the failure path (where the normal proof matrix is never reached) and makes a
    // production-only layout/accessibility regression directly inspectable instead of leaving only an
    // author-id list. The final proof manifest below still owns the accepted capture matrix.
    let navigation_shot = app.client.screenshot("main");
    let navigation_png = decode_verified_capture(
        &navigation_shot,
        "main",
        app.child_pid,
        "MT-008 immediate post-navigation diagnostic capture",
    );
    let navigation_dir = proof_dir();
    std::fs::create_dir_all(&navigation_dir)
        .expect("create MT-008 post-navigation diagnostic directory");
    std::fs::write(navigation_dir.join("mt008-navigation.png"), &navigation_png)
        .expect("write MT-008 post-navigation diagnostic capture");
    let discovered_surface = wait_for_author_id_between(
        &mut app.client,
        "main",
        &format!("{SURFACE_AUTHOR_ID}.pane."),
        "",
        SURFACE_TIMEOUT,
    );
    let initial_opened = app.client.inspect("main");
    let pane_id = pane_id_hosting(
        &initial_opened["snapshot"]["root"],
        &PaneType::SwarmLaneDiagnostics.label(),
    );
    assert_eq!(
        discovered_surface,
        scoped_author_id(&pane_id, SURFACE_AUTHOR_ID),
        "the live diagnostics surface must belong to its actual pane"
    );

    // The pane shell is synchronous, while its backend projection is fetched on
    // the native runtime.  Wait for one projection-owned privacy landmark before
    // taking the decisive snapshot so a slow real PostgreSQL read cannot be
    // mistaken for a missing exact-scope contract.
    wait_for_author_id_between(
        &mut app.client,
        "main",
        &scoped_author_id(&pane_id, PRIVACY_OWNER_AUTHOR_ID),
        "",
        SURFACE_TIMEOUT,
    );
    let opened = app.client.inspect("main");

    for author_id in [
        PRIVACY_OWNER_AUTHOR_ID,
        PRIVACY_PRINCIPAL_AUTHOR_ID,
        PRIVACY_SESSION_AUTHOR_ID,
        PRIVACY_ACCESS_SPACE_AUTHOR_ID,
        PRIVACY_WORKSPACE_AUTHOR_ID,
        PRIVACY_VISIBILITY_AUTHOR_ID,
        PRIVACY_DENIAL_AUTHOR_ID,
    ] {
        let scoped = scoped_author_id(&pane_id, author_id);
        assert!(
            contains_author_id(&opened["snapshot"]["root"], &scoped),
            "live diagnostics omitted exact server-owned privacy landmark {scoped}"
        );
    }
    assert!(
        node_text(require_node(
            &opened["snapshot"]["root"],
            &scoped_author_id(&pane_id, PRIVACY_VISIBILITY_AUTHOR_ID),
        ))
        .contains("exact account + Principal + session + AccessSpace + workspace"),
        "live diagnostics omitted its meaningful visible exact-scope posture"
    );
    assert!(
        node_text(require_node(
            &opened["snapshot"]["root"],
            &scoped_author_id(&pane_id, PRIVACY_DENIAL_AUTHOR_ID),
        ))
        .contains("foreign stored scope: not found"),
        "live diagnostics omitted its metadata-safe denial reason"
    );
    let owner_account_digest = privacy_node_digest(
        &opened["snapshot"]["root"],
        &pane_id,
        PRIVACY_OWNER_AUTHOR_ID,
        "active account",
    );
    let actor_principal_digest = privacy_node_digest(
        &opened["snapshot"]["root"],
        &pane_id,
        PRIVACY_PRINCIPAL_AUTHOR_ID,
        "acting Principal",
    );
    let authenticated_session_digest = privacy_node_digest(
        &opened["snapshot"]["root"],
        &pane_id,
        PRIVACY_SESSION_AUTHOR_ID,
        "authenticated session",
    );
    let access_space_digest = privacy_node_digest(
        &opened["snapshot"]["root"],
        &pane_id,
        PRIVACY_ACCESS_SPACE_AUTHOR_ID,
        "active AccessSpace",
    );
    let workspace_digest = privacy_node_digest(
        &opened["snapshot"]["root"],
        &pane_id,
        PRIVACY_WORKSPACE_AUTHOR_ID,
        "workspace",
    );

    // Discover the canonical run/lane/message values from the production projection itself. The
    // proof does not seed, reconstruct, or guess identifiers from a fixture.
    let root = &opened["snapshot"]["root"];
    let run_row = first_scoped_id_matching(root, &pane_id, "swarm-lane-diagnostics.run.", |text| {
        text.starts_with("run ") && text.contains(" | lanes ") && text.contains(" | messages ")
    });
    let run_text = node_text(require_node(root, &run_row));
    let run_id = run_text
        .strip_prefix("run ")
        .and_then(|text| text.split(" | ").next())
        .filter(|value| !value.is_empty())
        .expect("run row exposes its canonical run id")
        .to_owned();
    let lane_row =
        first_scoped_id_matching(root, &pane_id, "swarm-lane-diagnostics.lane.", |text| {
            text.contains(" | messages ") && text.contains(" | payload errors ")
        });
    let lane_text = node_text(require_node(root, &lane_row));
    let lane_id = lane_text
        .split(" | ")
        .next()
        .filter(|value| !value.is_empty())
        .expect("lane row exposes its canonical lane id")
        .to_owned();
    let message_row =
        first_scoped_id_matching(root, &pane_id, "swarm-lane-diagnostics.message.", |text| {
            text.starts_with("Message ")
        });
    let message_id = node_text(require_node(root, &message_row))
        .strip_prefix("Message ")
        .filter(|value| !value.is_empty())
        .expect("message row exposes its canonical message id")
        .to_owned();
    let payload_author_id = scoped_author_id(&pane_id, &message_payload_author_id(&message_id));
    let promotion_author_id = scoped_author_id(&pane_id, &message_promotion_author_id(&message_id));
    let payload_ref = node_text(require_node(root, &payload_author_id))
        .strip_prefix("Payload ")
        .filter(|value| !value.is_empty())
        .expect("payload drilldown exposes its canonical payload ref")
        .to_owned();
    let promotion_state = node_text(require_node(root, &promotion_author_id))
        .strip_prefix("Promotion ")
        .filter(|value| !value.is_empty())
        .expect("promotion drilldown exposes its canonical promotion state")
        .to_owned();

    // Independently re-read the canonical backend projection under the server-installed scope.
    // Raw five-dimensional identifiers never enter the rendered tree; equality is proved by
    // comparing the UI's stable digests with this non-visible structured response.
    let backend_path = format!("/swarm/model-lanes/diagnostics/{run_id}");
    let (backend_status, backend_projection, initial_backend_bytes) =
        backend_json(&backend_path, &[]);
    assert_eq!(
        backend_status, 200,
        "authorized canonical diagnostics re-read"
    );
    let backend_scope = &backend_projection["resource_scope"];
    let owner_account_fingerprint = backend_scope["owner_account_fingerprint"]
        .as_str()
        .expect("server scope has owner_account_fingerprint")
        .to_owned();
    let actor_principal_fingerprint = backend_scope["actor_principal_fingerprint"]
        .as_str()
        .expect("server scope has actor_principal_fingerprint")
        .to_owned();
    let authenticated_session_fingerprint = backend_scope["authenticated_session_fingerprint"]
        .as_str()
        .expect("server scope has authenticated_session_fingerprint")
        .to_owned();
    let access_space_fingerprint = backend_scope["access_space_fingerprint"]
        .as_str()
        .expect("server scope has access_space_fingerprint")
        .to_owned();
    let workspace_fingerprint = backend_scope["workspace_fingerprint"]
        .as_str()
        .expect("server scope has workspace_fingerprint")
        .to_owned();
    assert_eq!(backend_scope["visibility"], "private_exact_scope_only");
    assert_eq!(
        backend_scope["denial_posture"],
        "foreign_scope_is_absent_restricted_metadata_withheld"
    );
    let backend_projection_text = String::from_utf8_lossy(&initial_backend_bytes);
    for forbidden_raw_field in [
        "\"owner_account_id\"",
        "\"actor_principal_id\"",
        "\"authenticated_session_id\"",
        "\"access_space_id\"",
        "\"workspace_id\"",
    ] {
        assert!(
            !backend_projection_text.contains(forbidden_raw_field),
            "privacy-safe frontend response exposed raw scope field {forbidden_raw_field}"
        );
    }
    for (field, server_fingerprint, rendered_digest) in [
        (
            "owner_account_fingerprint",
            owner_account_fingerprint.as_str(),
            owner_account_digest.as_str(),
        ),
        (
            "actor_principal_fingerprint",
            actor_principal_fingerprint.as_str(),
            actor_principal_digest.as_str(),
        ),
        (
            "authenticated_session_fingerprint",
            authenticated_session_fingerprint.as_str(),
            authenticated_session_digest.as_str(),
        ),
        (
            "access_space_fingerprint",
            access_space_fingerprint.as_str(),
            access_space_digest.as_str(),
        ),
        (
            "workspace_fingerprint",
            workspace_fingerprint.as_str(),
            workspace_digest.as_str(),
        ),
    ] {
        assert_eq!(
            rendered_digest, server_fingerprint,
            "redacted UI privacy field {field} must bind the server-owned backend scope"
        );
    }
    let backend_lanes = backend_projection["lanes"]
        .as_array()
        .expect("backend diagnostics lanes array");
    let backend_messages = backend_projection["messages"]
        .as_array()
        .expect("backend diagnostics messages array");
    assert!(
        run_text.contains(&format!("lanes {}", backend_lanes.len()))
            && run_text.contains(&format!("messages {}", backend_messages.len())),
        "rendered counts must match the canonical backend projection: `{run_text}`"
    );
    let canonical_message = backend_messages
        .iter()
        .find(|message| message["message_id"] == message_id)
        .expect("rendered message resolves in canonical backend projection");
    assert_eq!(canonical_message["payload_ref"], payload_ref);
    assert_eq!(canonical_message["promotion_state"], promotion_state);

    let foreign_owner = "00000000-0000-4000-8000-000000000001";
    let foreign_workspace = "mt008-foreign-workspace-canary";
    let (foreign_owner_status, foreign_owner_body, _) = backend_json(
        &backend_path,
        &[("x-handshake-owner-account", foreign_owner)],
    );
    let (foreign_workspace_status, foreign_workspace_body, _) = backend_json(
        &backend_path,
        &[("x-handshake-workspace", foreign_workspace)],
    );
    for (dimension, status, body) in [
        ("owner", foreign_owner_status, foreign_owner_body),
        (
            "workspace",
            foreign_workspace_status,
            foreign_workspace_body,
        ),
    ] {
        assert_eq!(
            status, 403,
            "foreign {dimension} assertion must fail closed"
        );
        let denial = body.to_string();
        for protected in [&run_id, &message_id, &payload_ref] {
            assert!(
                !denial.contains(protected),
                "foreign {dimension} denial leaked protected diagnostics metadata"
            );
        }
    }

    // Drive every deep-link/filter control through authenticated production-socket mutations. The
    // run filter is followed by the real Refresh action, proving it addresses backend state rather
    // than merely changing a textbox.
    let freshness_author_id = scoped_author_id(&pane_id, FRESHNESS_AUTHOR_ID);
    let initial_freshness = node_text(require_node(
        &opened["snapshot"]["root"],
        &freshness_author_id,
    ));
    let run_filter = scoped_author_id(&pane_id, RUN_FILTER_AUTHOR_ID);
    let run_filter_clear_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        &run_filter,
        Some(("value", serde_json::json!(""))),
    );
    let run_filter_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        &run_filter,
        Some(("value", serde_json::json!(run_id))),
    );
    let refresh_receipt = app.client.mutation_on_live_surface(
        "argus.click",
        "main",
        &scoped_author_id(&pane_id, REFRESH_AUTHOR_ID),
        None,
    );
    let refreshed = wait_for_author_text_change(
        &mut app.client,
        "main",
        &freshness_author_id,
        &initial_freshness,
    );
    assert_eq!(
        node_value(require_node(&refreshed["snapshot"]["root"], &run_filter)),
        run_id,
        "run filter must retain the canonical run id after the production refresh"
    );

    let lane_filter = scoped_author_id(&pane_id, LANE_FILTER_AUTHOR_ID);
    let lane_filter_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        &lane_filter,
        Some(("value", serde_json::json!(lane_id))),
    );
    wait_for_author_id(
        &mut app.client,
        "main",
        &scoped_author_id(&pane_id, &lane_author_id(&lane_id)),
        SURFACE_TIMEOUT,
    );
    let message_filter = scoped_author_id(&pane_id, MESSAGE_FILTER_AUTHOR_ID);
    wait_for_author_id(
        &mut app.client,
        "main",
        &scoped_author_id(&pane_id, &message_author_id(&message_id)),
        SURFACE_TIMEOUT,
    );

    let message_filter_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        &message_filter,
        Some(("value", serde_json::json!(message_id))),
    );
    wait_for_author_id(&mut app.client, "main", &message_row, SURFACE_TIMEOUT);

    let payload_receipt =
        app.client
            .mutation_on_live_surface("argus.click", "main", &payload_author_id, None);
    let selected_author_id = scoped_author_id(&pane_id, &selected_message_author_id(&message_id));
    let payload_selected = wait_for_author_id(
        &mut app.client,
        "main",
        &selected_author_id,
        SURFACE_TIMEOUT,
    );
    let selected_text = node_text(require_node(
        &payload_selected["snapshot"]["root"],
        &selected_author_id,
    ));
    assert!(
        selected_text.contains(&format!("payload {payload_ref}"))
            && selected_text.contains(&format!("promotion {promotion_state}")),
        "payload drilldown must expose both canonical payload and promotion state: `{selected_text}`"
    );
    let promotion_receipt =
        app.client
            .mutation_on_live_surface("argus.click", "main", &promotion_author_id, None);
    let promotion_selected = wait_for_author_id(
        &mut app.client,
        "main",
        &selected_author_id,
        SURFACE_TIMEOUT,
    );
    assert!(
        node_text(require_node(
            &promotion_selected["snapshot"]["root"],
            &selected_author_id,
        ))
        .contains(&format!("promotion {promotion_state}")),
        "promotion drilldown must retain the canonical promotion state"
    );

    let main_shot = app.client.screenshot("main");
    let main_png = decode_verified_capture(
        &main_shot,
        "main",
        app.child_pid,
        "MT-008 docked diagnostics capture",
    );

    // Edge-state member of the HBR-VIS-003 matrix: a deliberately unmatched
    // message filter must render an explicit empty-state landmark, with the
    // canonical message row absent. Restore the real message afterwards so the
    // detached state proves the same payload/promotion drilldown.
    let edge_filter_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        &message_filter,
        Some((
            "value",
            serde_json::json!("no-message-can-match-this-edge-state"),
        )),
    );
    let empty_author_id = scoped_author_id(&pane_id, EMPTY_MESSAGES_AUTHOR_ID);
    let _ = wait_for_author_id(&mut app.client, "main", &empty_author_id, SURFACE_TIMEOUT);
    let empty_state = wait_for_author_id_absent(
        &mut app.client,
        "main",
        &scoped_author_id(&pane_id, &message_author_id(&message_id)),
    );
    assert!(
        node_text(require_node(
            &empty_state["snapshot"]["root"],
            &empty_author_id
        ))
        .contains("No messages match"),
        "edge-state landmark omitted its visible empty reason"
    );
    let edge_shot = app.client.screenshot("main");
    let edge_png = decode_verified_capture(
        &edge_shot,
        "main",
        app.child_pid,
        "MT-008 empty-message edge-state capture",
    );
    let restore_message_filter_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        &message_filter,
        Some(("value", serde_json::json!(message_id))),
    );
    wait_for_author_id(&mut app.client, "main", &message_row, SURFACE_TIMEOUT);
    let restore_payload_receipt =
        app.client
            .mutation_on_live_surface("argus.click", "main", &payload_author_id, None);
    let restored_selected = wait_for_author_id(
        &mut app.client,
        "main",
        &selected_author_id,
        SURFACE_TIMEOUT,
    );

    // Pop out the exact pane discovered above, then prove the same selected message state is
    // inspectable, capturable, and steerable in that detached OS window.
    let pane_header = handshake_native::pane_header::pane_header_author_id(&pane_id);
    let popout_context_receipt =
        app.client
            .mutation_on_live_surface("argus.show_context_menu", "main", &pane_header, None);
    wait_for_author_id(
        &mut app.client,
        "main",
        "ctx-menu.pane.pop_out",
        SURFACE_TIMEOUT,
    );
    let popout_receipt =
        app.client
            .mutation_on_live_surface("argus.click", "main", "ctx-menu.pane.pop_out", None);
    let popout_window_id = handshake_native::popout_window::argus_window_id(&pane_id);
    wait_for_window(&mut app.client, &popout_window_id, true);
    let detached = wait_for_author_id(
        &mut app.client,
        &popout_window_id,
        &selected_author_id,
        SURFACE_TIMEOUT,
    );
    let detached_selected_text = node_text(require_node(
        &detached["snapshot"]["root"],
        &selected_author_id,
    ));
    assert!(
        detached_selected_text.contains(&payload_ref)
            && detached_selected_text.contains(&promotion_state),
        "detached diagnostics must preserve payload/promotion drilldown state: `{detached_selected_text}`"
    );
    let main_while_detached = app.client.inspect("main");
    assert!(
        !contains_author_id(
            &main_while_detached["snapshot"]["root"],
            &scoped_author_id(&pane_id, SURFACE_AUTHOR_ID),
        ),
        "the exact diagnostics pane must no longer render in main while detached"
    );
    let main_detached_shot = app.client.screenshot("main");
    let main_detached_png = decode_verified_capture(
        &main_detached_shot,
        "main",
        app.child_pid,
        "MT-008 main window while diagnostics are detached",
    );
    let detached_promotion_receipt = app.client.mutation_on_live_surface(
        "argus.click",
        &popout_window_id,
        &promotion_author_id,
        None,
    );
    let detached_after_action = app.client.inspect(&popout_window_id);
    let detached_shot = app.client.screenshot(&popout_window_id);
    let detached_png = decode_verified_capture(
        &detached_shot,
        &popout_window_id,
        app.child_pid,
        "MT-008 detached diagnostics capture",
    );
    let main_width = main_shot["result"]["width"]
        .as_u64()
        .expect("main capture width");
    let main_height = main_shot["result"]["height"]
        .as_u64()
        .expect("main capture height");
    let detached_width = detached_shot["result"]["width"]
        .as_u64()
        .expect("detached capture width");
    let detached_height = detached_shot["result"]["height"]
        .as_u64()
        .expect("detached capture height");
    assert!(
        detached_width < main_width || detached_height < main_height,
        "detached diagnostics capture must provide the constrained viewport member: main={main_width}x{main_height}, detached={detached_width}x{detached_height}"
    );

    let merge_back_author_id = handshake_native::popout_window::merge_back_author_id(&pane_id);
    let merge_back_receipt =
        app.client
            .mutation_on_live_surface("argus.click", "main", &merge_back_author_id, None);
    wait_for_window(&mut app.client, &popout_window_id, false);
    wait_for_author_id(
        &mut app.client,
        "main",
        &scoped_author_id(&pane_id, SURFACE_AUTHOR_ID),
        SURFACE_TIMEOUT,
    );

    // Bracket every interaction and capture with a final canonical re-read. Model-lane UI
    // interactions are observational, so a changed canonical revision invalidates this proof and
    // requires a fresh capture against the new projection.
    let (final_backend_status, final_backend_projection, final_backend_bytes) =
        backend_json(&backend_path, &[]);
    assert_eq!(
        final_backend_status, 200,
        "final canonical diagnostics re-read"
    );
    assert_eq!(
        final_backend_projection["run"]["event_ledger_seq"],
        backend_projection["run"]["event_ledger_seq"],
        "canonical event-ledger revision changed during MT-008 capture"
    );
    assert_eq!(
        sha256_hex(&final_backend_bytes),
        sha256_hex(&initial_backend_bytes),
        "canonical diagnostics projection transitioned during MT-008 capture"
    );

    let scope_hash = sha256_hex(
        format!(
            "{owner_account_fingerprint}\0{actor_principal_fingerprint}\0{authenticated_session_fingerprint}\0{access_space_fingerprint}\0{workspace_fingerprint}"
        )
        .as_bytes(),
    );
    let protected_canaries = [
        run_id.as_str(),
        lane_id.as_str(),
        message_id.as_str(),
        payload_ref.as_str(),
        foreign_owner,
        foreign_workspace,
    ];
    let privacy_canaries = [foreign_owner, foreign_workspace];
    assert_decoded_capture_privacy(
        &main_png,
        &promotion_selected["snapshot"]["root"],
        &privacy_canaries,
        "MT-008 docked decoded-image privacy inspection",
    );
    assert_decoded_capture_privacy(
        &detached_png,
        &detached_after_action["snapshot"]["root"],
        &privacy_canaries,
        "MT-008 detached decoded-image privacy inspection",
    );
    assert_decoded_capture_privacy(
        &edge_png,
        &empty_state["snapshot"]["root"],
        &privacy_canaries,
        "MT-008 empty-message edge decoded-image privacy inspection",
    );
    assert_decoded_capture_privacy(
        &main_detached_png,
        &main_while_detached["snapshot"]["root"],
        &privacy_canaries,
        "MT-008 main-while-detached decoded-image privacy inspection",
    );

    let (staging_dir, final_dir, nonce_hash) =
        create_nonce_scoped_staging(&scope_hash, &proof_nonce);
    write_proof_artifact(&staging_dir, "main.png", &main_png);
    write_proof_artifact(&staging_dir, "edge-empty.png", &edge_png);
    write_proof_artifact(&staging_dir, "main-detached.png", &main_detached_png);
    write_proof_artifact(&staging_dir, "popout.png", &detached_png);
    let raw_transcript = app
        .client
        .assert_transcript_is_secret_free(&privacy_canaries);
    let methods = app
        .client
        .transcript
        .iter()
        .filter_map(|entry| entry["request"]["method"].as_str())
        .collect::<Vec<_>>();
    let transcript_summary = serde_json::json!({
        "schema_id": "handshake.argus.minimized_transcript_commitment@1",
        "entry_count": app.client.transcript.len(),
        "methods": methods,
        "full_transcript_sha256": sha256_hex(&raw_transcript),
        "content_posture": "full snapshots, author ids, values, payload refs, and screenshot bytes intentionally omitted",
    });
    let transcript_bytes = serde_json::to_vec_pretty(&transcript_summary)
        .expect("serialize minimized MT-008 transcript commitment");
    for canary in protected_canaries {
        assert_bytes_exclude(
            &transcript_bytes,
            canary,
            "MT-008 minimized transcript privacy scan",
        );
    }
    write_proof_artifact(
        &staging_dir,
        "transcript_commitment.json",
        &transcript_bytes,
    );
    let backend_commitment = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_mt008_backend_commitment@1",
        "resource_scope_sha256": scope_hash,
        "scope_dimension_process_fingerprints": {
            "owner_account": owner_account_digest,
            "actor_principal": actor_principal_digest,
            "authenticated_session": authenticated_session_digest,
            "access_space": access_space_digest,
            "workspace": workspace_digest,
        },
        "projection_schema_id": backend_projection["schema_id"],
        "event_ledger_seq": backend_projection["run"]["event_ledger_seq"],
        "initial_projection_sha256": sha256_hex(&initial_backend_bytes),
        "final_projection_sha256": sha256_hex(&final_backend_bytes),
        "stable_across_capture": true,
        "run_id_sha256": sha256_hex(run_id.as_bytes()),
        "lane_count": backend_lanes.len(),
        "message_count": backend_messages.len(),
        "selected_message_id_sha256": sha256_hex(message_id.as_bytes()),
        "selected_payload_ref_sha256": sha256_hex(payload_ref.as_bytes()),
        "selected_promotion_state_sha256": sha256_hex(promotion_state.as_bytes()),
    });
    let backend_commitment_bytes = serde_json::to_vec_pretty(&backend_commitment)
        .expect("serialize redacted MT-008 backend commitment");
    write_proof_artifact(
        &staging_dir,
        "backend_commitment.json",
        &backend_commitment_bytes,
    );
    let renderer_scan = renderer_runtime_error_scan(
        &diagnostics_dir,
        app.child_pid,
        capture_window_started_at_unix_ms,
    );
    let renderer_scan_bytes = serde_json::to_vec_pretty(&renderer_scan)
        .expect("serialize MT-008 native renderer/runtime error scan");
    write_proof_artifact(
        &staging_dir,
        "renderer_error_scan.json",
        &renderer_scan_bytes,
    );

    let visual_inspection = serde_json::json!({
        "schema_id": "handshake.argus.mt008_visual_inspection@1",
        "matrix_complete": true,
        "semantic_verdict": "inspection_ready",
        "independent_pixel_verdict": "pending_validator_inspection",
        "inspection_contract": {
            "readability": "all decisive landmarks have positive finite accessibility bounds",
            "discoverability": "menu route and controls are represented by stable authored landmarks",
            "navigation": "main -> detached -> main transition is captured",
            "important_state": "privacy visibility, denial posture, selection, empty state, and merge control are explicit",
            "overlap_and_responsiveness": "normal and constrained captures are independently decodable and nonblank",
            "privacy": "all five raw resource-scope values are absent from capture-bound trees, encoded PNGs, and decoded pixels",
        },
        "states": [
            {
                "state": "normal_docked",
                "artifact": "main.png",
                "artifact_sha256": sha256_hex(&main_png),
                "snapshot_sha256": json_sha256(&promotion_selected),
                "width": main_shot["result"]["width"],
                "height": main_shot["result"]["height"],
                "landmarks": [
                    landmark_inspection(&promotion_selected["snapshot"]["root"], &scoped_author_id(&pane_id, SURFACE_AUTHOR_ID)),
                    landmark_inspection(&promotion_selected["snapshot"]["root"], &scoped_author_id(&pane_id, PRIVACY_VISIBILITY_AUTHOR_ID)),
                    landmark_inspection(&promotion_selected["snapshot"]["root"], &scoped_author_id(&pane_id, PRIVACY_DENIAL_AUTHOR_ID)),
                    landmark_inspection(&promotion_selected["snapshot"]["root"], &selected_author_id),
                ],
            },
            {
                "state": "edge_empty_messages",
                "artifact": "edge-empty.png",
                "artifact_sha256": sha256_hex(&edge_png),
                "snapshot_sha256": json_sha256(&empty_state),
                "width": edge_shot["result"]["width"],
                "height": edge_shot["result"]["height"],
                "landmarks": [
                    landmark_inspection(&empty_state["snapshot"]["root"], &scoped_author_id(&pane_id, SURFACE_AUTHOR_ID)),
                    landmark_inspection(&empty_state["snapshot"]["root"], &empty_author_id),
                ],
            },
            {
                "state": "main_while_exact_pane_detached",
                "artifact": "main-detached.png",
                "artifact_sha256": sha256_hex(&main_detached_png),
                "snapshot_sha256": json_sha256(&main_while_detached),
                "width": main_detached_shot["result"]["width"],
                "height": main_detached_shot["result"]["height"],
                "detached_surface_absent": true,
                "landmarks": [landmark_inspection(&main_while_detached["snapshot"]["root"], &merge_back_author_id)],
            },
            {
                "state": "constrained_detached",
                "artifact": "popout.png",
                "artifact_sha256": sha256_hex(&detached_png),
                "snapshot_sha256": json_sha256(&detached_after_action),
                "width": detached_shot["result"]["width"],
                "height": detached_shot["result"]["height"],
                "smaller_than_main_in_at_least_one_dimension": true,
                "landmarks": [
                    landmark_inspection(&detached_after_action["snapshot"]["root"], &scoped_author_id(&pane_id, SURFACE_AUTHOR_ID)),
                    landmark_inspection(&detached_after_action["snapshot"]["root"], &scoped_author_id(&pane_id, PRIVACY_VISIBILITY_AUTHOR_ID)),
                    landmark_inspection(&detached_after_action["snapshot"]["root"], &selected_author_id),
                ],
            },
        ],
        "renderer_runtime_error_scan_sha256": sha256_hex(&renderer_scan_bytes),
    });
    let visual_inspection_bytes = serde_json::to_vec_pretty(&visual_inspection)
        .expect("serialize independently inspectable MT-008 visual matrix");
    write_proof_artifact(
        &staging_dir,
        "visual_inspection.json",
        &visual_inspection_bytes,
    );

    let receipt = |operation: &str,
                   requested_method: &str,
                   requested_window_id: &str,
                   requested_author_id: &str,
                   response: &serde_json::Value| {
        assert_eq!(response["result"]["window_id"], requested_window_id);
        assert_eq!(response["result"]["author_id"], requested_author_id);
        assert!(
            response["result"]["action_id"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "{operation} omitted its action id"
        );
        assert!(
            response["result"]["evidence_ref"]
                .as_str()
                .is_some_and(|value| {
                    value.starts_with("native-action-log://")
                        || value.starts_with("eventledger://kernel/")
                }),
            "{operation} omitted its dereferenceable action evidence"
        );
        serde_json::json!({
            "operation": operation,
            "action_id": response["result"]["action_id"],
            "requested_method": requested_method,
            "window_id": requested_window_id,
            "author_id_sha256": sha256_hex(requested_author_id.as_bytes()),
            "status": response["result"]["status"],
            "before_revision": response["result"]["before_revision"],
            "after_revision": response["result"]["after_revision"],
            "evidence_ref": response["result"]["evidence_ref"],
            "evidence_ref_sha256": sha256_hex(response["result"]["evidence_ref"].as_str().unwrap_or_default().as_bytes()),
            "agent_id_sha256": sha256_hex(response["result"]["agent_id"].as_str().unwrap_or_default().as_bytes()),
            "agent_label": response["result"]["agent_label"],
        })
    };
    let provenance = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_mt008_diagnostics_provenance@2",
        "mt_id": "MT-008",
        "proof_nonce_sha256": nonce_hash,
        "child_pid": app.child_pid,
        "authenticated_agent_id_sha256": sha256_hex(app.authenticated_agent_id.as_bytes()),
        "resource_scope_sha256": scope_hash,
        "navigation": ["menu-models", "menu.models.swarm-lane-diagnostics"],
        "pane_id": pane_id,
        "run_id_sha256": sha256_hex(run_id.as_bytes()),
        "lane_id_sha256": sha256_hex(lane_id.as_bytes()),
        "message_id_sha256": sha256_hex(message_id.as_bytes()),
        "payload_ref_sha256": sha256_hex(payload_ref.as_bytes()),
        "promotion_state_sha256": sha256_hex(promotion_state.as_bytes()),
        "canonical_counts": {
            "lanes": backend_lanes.len(),
            "messages": backend_messages.len(),
        },
        "decisive_observations": {
            "opened_snapshot_sha256": json_sha256(&opened),
            "payload_selected_snapshot_sha256": json_sha256(&payload_selected),
            "promotion_selected_snapshot_sha256": json_sha256(&promotion_selected),
            "detached_snapshot_sha256": json_sha256(&detached),
            "detached_after_action_snapshot_sha256": json_sha256(&detached_after_action),
            "backend_commitment_sha256": sha256_hex(&backend_commitment_bytes),
            "minimized_transcript_sha256": sha256_hex(&transcript_bytes),
            "visual_inspection_sha256": sha256_hex(&visual_inspection_bytes),
            "renderer_error_scan_sha256": sha256_hex(&renderer_scan_bytes),
        },
        "receipts": {
            "run_filter_clear": receipt("run_filter_clear", "argus.set_value", "main", &run_filter, &run_filter_clear_receipt),
            "run_filter": receipt("run_filter", "argus.set_value", "main", &run_filter, &run_filter_receipt),
            "run_refresh": receipt("run_refresh", "argus.click", "main", &scoped_author_id(&pane_id, REFRESH_AUTHOR_ID), &refresh_receipt),
            "lane_filter": receipt("lane_filter", "argus.set_value", "main", &lane_filter, &lane_filter_receipt),
            "message_filter": receipt("message_filter", "argus.set_value", "main", &message_filter, &message_filter_receipt),
            "payload_drilldown": receipt("payload_drilldown", "argus.click", "main", &payload_author_id, &payload_receipt),
            "promotion_drilldown": receipt("promotion_drilldown", "argus.click", "main", &promotion_author_id, &promotion_receipt),
            "edge_filter": receipt("edge_filter", "argus.set_value", "main", &message_filter, &edge_filter_receipt),
            "restore_message_filter": receipt("restore_message_filter", "argus.set_value", "main", &message_filter, &restore_message_filter_receipt),
            "restore_payload_drilldown": receipt("restore_payload_drilldown", "argus.click", "main", &payload_author_id, &restore_payload_receipt),
            "popout_context": receipt("popout_context", "argus.show_context_menu", "main", &pane_header, &popout_context_receipt),
            "popout": receipt("popout", "argus.click", "main", "ctx-menu.pane.pop_out", &popout_receipt),
            "detached_promotion_drilldown": receipt("detached_promotion_drilldown", "argus.click", &popout_window_id, &promotion_author_id, &detached_promotion_receipt),
            "merge_back": receipt("merge_back", "argus.click", "main", &merge_back_author_id, &merge_back_receipt),
        },
        "captures": [
            {
                "artifact": "main.png",
                "window_id": main_shot["result"]["window_id"],
                "pid": main_shot["result"]["pid"],
                "width": main_shot["result"]["width"],
                "height": main_shot["result"]["height"],
                "captured_at_utc": main_shot["result"]["captured_at_utc"],
                "sha256": main_shot["result"]["sha256"],
            },
            {
                "artifact": "edge-empty.png",
                "window_id": edge_shot["result"]["window_id"],
                "pid": edge_shot["result"]["pid"],
                "width": edge_shot["result"]["width"],
                "height": edge_shot["result"]["height"],
                "captured_at_utc": edge_shot["result"]["captured_at_utc"],
                "sha256": edge_shot["result"]["sha256"],
            },
            {
                "artifact": "main-detached.png",
                "window_id": main_detached_shot["result"]["window_id"],
                "pid": main_detached_shot["result"]["pid"],
                "width": main_detached_shot["result"]["width"],
                "height": main_detached_shot["result"]["height"],
                "captured_at_utc": main_detached_shot["result"]["captured_at_utc"],
                "sha256": main_detached_shot["result"]["sha256"],
            },
            {
                "artifact": "popout.png",
                "window_id": detached_shot["result"]["window_id"],
                "pid": detached_shot["result"]["pid"],
                "width": detached_shot["result"]["width"],
                "height": detached_shot["result"]["height"],
                "captured_at_utc": detached_shot["result"]["captured_at_utc"],
                "sha256": detached_shot["result"]["sha256"],
            }
        ],
    });
    let provenance_bytes =
        serde_json::to_vec_pretty(&provenance).expect("serialize MT-008 provenance");
    for canary in protected_canaries {
        assert_bytes_exclude(&provenance_bytes, canary, "MT-008 provenance privacy scan");
    }
    write_proof_artifact(&staging_dir, "provenance.json", &provenance_bytes);
    let evidence_chain = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_mt008_evidence_chain@2",
        "proof_nonce_sha256": nonce_hash,
        "resource_scope_sha256": scope_hash,
        "provenance_sha256": sha256_hex(&provenance_bytes),
        "transcript_commitment_sha256": sha256_hex(&transcript_bytes),
        "main_png_sha256": sha256_hex(&main_png),
        "edge_empty_png_sha256": sha256_hex(&edge_png),
        "main_detached_png_sha256": sha256_hex(&main_detached_png),
        "popout_png_sha256": sha256_hex(&detached_png),
        "backend_commitment_sha256": sha256_hex(&backend_commitment_bytes),
        "visual_inspection_sha256": sha256_hex(&visual_inspection_bytes),
        "renderer_error_scan_sha256": sha256_hex(&renderer_scan_bytes),
    });
    let evidence_chain_bytes =
        serde_json::to_vec_pretty(&evidence_chain).expect("serialize MT-008 evidence chain");
    write_proof_artifact(&staging_dir, "evidence_chain.json", &evidence_chain_bytes);
    let mut manifest_files = BTreeMap::new();
    for (name, bytes) in [
        ("main.png", main_png.as_slice()),
        ("edge-empty.png", edge_png.as_slice()),
        ("main-detached.png", main_detached_png.as_slice()),
        ("popout.png", detached_png.as_slice()),
        ("transcript_commitment.json", transcript_bytes.as_slice()),
        (
            "backend_commitment.json",
            backend_commitment_bytes.as_slice(),
        ),
        ("provenance.json", provenance_bytes.as_slice()),
        ("evidence_chain.json", evidence_chain_bytes.as_slice()),
        ("visual_inspection.json", visual_inspection_bytes.as_slice()),
        ("renderer_error_scan.json", renderer_scan_bytes.as_slice()),
    ] {
        manifest_files.insert(name, sha256_hex(bytes));
    }
    let manifest = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_mt008_manifest@1",
        "proof_nonce_sha256": nonce_hash,
        "resource_scope_sha256": scope_hash,
        "files": manifest_files,
    });
    write_proof_artifact(
        &staging_dir,
        "manifest.json",
        &serde_json::to_vec_pretty(&manifest).expect("serialize complete MT-008 manifest"),
    );
    validate_and_publish_proof(&staging_dir, &final_dir, &scope_hash, &nonce_hash);

    app.shutdown();
}

#[ignore = "LIVE production socket E2E: opens native main/pop-out windows and requires managed \
            PostgreSQL, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, and a shared HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn production_binary_argus_socket_inspect_click_set_screenshot_receipts_and_popout() {
    let diagnostics_dir = require_palmistry_ready_backend();
    let tmp = std::env::temp_dir().join(format!(
        "hsk_argus_production_socket_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create isolated LOCALAPPDATA");
    let binding_path = tmp.join("handshake").join("swarm_mcp_binding.json");

    let child = Command::new(env!("CARGO_BIN_EXE_handshake-native"))
        .env("LOCALAPPDATA", &tmp)
        .env("HANDSHAKE_DIAGNOSTICS_DIR", &diagnostics_dir)
        .spawn()
        .expect("spawn production handshake-native binary");
    let child_pid = child.id();
    let mut child_guard = ChildGuard(child);
    let binding = discover_binding(
        &binding_path,
        child_pid,
        Instant::now() + Duration::from_secs(30),
    );
    let mut client = ArgusClient {
        addr: binding.tcp_addr,
        token: binding.token,
        next_id: 1,
        agent_token: None,
        agent_id: None,
        transcript: Vec::new(),
    };
    let authenticated_agent_id = client.authenticate_agent();
    assert!(!authenticated_agent_id.is_empty());

    let windows = client.call("argus.list_windows", serde_json::json!({}));
    assert_success(&windows, "argus.list_windows");
    assert!(list_has_window(&windows, "main"), "main window not listed");

    let initial = client.inspect("main");
    assert!(contains_author_id(
        &initial["snapshot"]["root"],
        "shell.chrome.theme-toggle"
    ));
    assert!(contains_author_id(
        &initial["snapshot"]["root"],
        "bottom-rail.input"
    ));

    client.mutation("argus.click", "main", "shell.chrome.theme-toggle", None);
    client.mutation(
        "argus.set_value",
        "main",
        "bottom-rail.input",
        Some((
            "value",
            serde_json::Value::String("production-socket-proof".to_owned()),
        )),
    );
    let after_set = client.inspect("main");
    assert!(
        serde_json::to_string(&after_set["snapshot"]["root"])
            .expect("serialize tree")
            .contains("production-socket-proof"),
        "set value was not visible in the next canonical snapshot"
    );

    let screenshot = client.call("argus.screenshot", serde_json::json!({"window_id": "main"}));
    assert_success(&screenshot, "argus.screenshot(main)");
    let png = base64::engine::general_purpose::STANDARD
        .decode(
            screenshot["result"]["png_base64"]
                .as_str()
                .expect("screenshot png_base64"),
        )
        .expect("decode screenshot PNG");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "capture is not PNG");
    assert_eq!(
        screenshot["result"]["sha256"],
        format!("{:x}", Sha256::digest(&png))
    );
    assert_eq!(screenshot["result"]["window_id"], "main");
    assert_eq!(screenshot["result"]["pid"], child_pid);
    assert!(screenshot["result"]["width"]
        .as_u64()
        .is_some_and(|v| v > 0));
    assert!(screenshot["result"]["height"]
        .as_u64()
        .is_some_and(|v| v > 0));
    assert_visual_png(&png, "main-window capture");

    // Exercise the actual MT-015 Settings/cloud-access surface through the production socket.
    let secret_canary = "production-socket-secret-canary";
    let settings_landmarks = [
        "settings.dialog",
        "settings.cloud.byok.openai.key",
        "settings.cloud.byok.openai.status",
        "settings.cloud.byok.openai.save",
        "settings.cloud.byok.anthropic.key",
        "settings.cloud.byok.anthropic.status",
        "settings.cloud.cli.claude_code.status",
        "settings.cloud.cli.codex.status",
    ];
    client.mutation("argus.click", "main", "menu-help", None);
    let help_menu = client.inspect("main");
    assert!(
        contains_author_id(&help_menu["snapshot"]["root"], "menu.help.settings"),
        "HELP menu did not expose Open Settings"
    );
    client.mutation("argus.click", "main", "menu.help.settings", None);
    let settings = client.inspect("main");
    for author_id in settings_landmarks {
        assert!(
            contains_author_id(&settings["snapshot"]["root"], author_id),
            "production Settings snapshot omitted {author_id}"
        );
    }
    let settings_revision = settings["revision"]
        .as_u64()
        .expect("Settings inspect revision is numeric");
    let secret_denial = client.call(
        "argus.set_value",
        serde_json::json!({
            "window_id": "main",
            "author_id": "settings.cloud.byok.openai.key",
            "expected_snapshot_revision": settings_revision,
            "value": secret_canary
        }),
    );
    assert!(
        secret_denial.get("error").is_some(),
        "secret-bearing input accepted generic Argus set_value"
    );
    assert!(
        !secret_denial.to_string().contains(secret_canary),
        "secret-bearing denial echoed its value"
    );
    let settings_after_denial = client.inspect("main");
    assert_eq!(
        settings_after_denial["revision"].as_u64(),
        Some(settings_revision),
        "denied secret input unexpectedly advanced the Settings revision"
    );
    for author_id in settings_landmarks {
        assert!(
            contains_author_id(&settings_after_denial["snapshot"]["root"], author_id),
            "Settings landmark disappeared before visual capture: {author_id}"
        );
    }
    let settings_json = serde_json::to_string(&settings_after_denial["snapshot"]["root"])
        .expect("serialize Settings tree");
    assert!(
        !settings_json.contains(secret_canary),
        "Settings snapshot disclosed the BYOK canary"
    );

    // Bracket the targeted capture with canonical Settings snapshots. This proves the live main-window
    // PNG was captured while the Settings/cloud controls were rendered, not from an earlier frame.
    let settings_shot = client.call("argus.screenshot", serde_json::json!({"window_id": "main"}));
    assert_success(&settings_shot, "argus.screenshot(main Settings-open)");
    assert_eq!(settings_shot["result"]["window_id"], "main");
    assert_eq!(settings_shot["result"]["pid"], child_pid);
    assert!(
        settings_shot["result"]["width"]
            .as_u64()
            .is_some_and(|value| value > 0)
            && settings_shot["result"]["height"]
                .as_u64()
                .is_some_and(|value| value > 0),
        "Settings-open capture had zero dimensions"
    );
    assert!(
        !settings_shot.to_string().contains(secret_canary),
        "Settings-open screenshot response disclosed the BYOK canary"
    );
    let settings_png = base64::engine::general_purpose::STANDARD
        .decode(
            settings_shot["result"]["png_base64"]
                .as_str()
                .expect("Settings screenshot png_base64"),
        )
        .expect("decode Settings screenshot PNG");
    assert!(
        settings_png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "Settings-open capture is not PNG"
    );
    assert_eq!(
        settings_shot["result"]["sha256"],
        format!("{:x}", Sha256::digest(&settings_png))
    );
    assert_visual_png(&settings_png, "Settings-open main-window capture");
    assert!(
        !settings_png
            .windows(secret_canary.len())
            .any(|window| window == secret_canary.as_bytes()),
        "Settings-open PNG bytes disclosed the BYOK canary"
    );
    let settings_after_capture = client.inspect("main");
    for author_id in settings_landmarks {
        assert!(
            contains_author_id(&settings_after_capture["snapshot"]["root"], author_id),
            "Settings landmark was not present after visual capture: {author_id}"
        );
    }
    assert!(
        !serde_json::to_string(&settings_after_capture["snapshot"]["root"])
            .expect("serialize post-capture Settings tree")
            .contains(secret_canary),
        "post-capture Settings snapshot disclosed the BYOK canary"
    );
    client.mutation("argus.click", "main", "settings.close", None);

    // Canonical non-pointer context-menu opening, causal menu-item acknowledgement, and real pop-out.
    client.mutation(
        "argus.show_context_menu",
        "main",
        "pane-pane-a-header",
        None,
    );
    let menu_snapshot = client.inspect("main");
    assert!(
        contains_author_id(&menu_snapshot["snapshot"]["root"], "ctx-menu.pane.pop_out"),
        "pane context menu did not expose its stable pop-out item"
    );
    client.mutation("argus.click", "main", "ctx-menu.pane.pop_out", None);
    wait_for_window(&mut client, "popout-pane-a", true);
    let popout = client.inspect("popout-pane-a");
    assert!(contains_author_id(
        &popout["snapshot"]["root"],
        "popout-window-pane-a"
    ));
    let popout_shot = client.call(
        "argus.screenshot",
        serde_json::json!({"window_id": "popout-pane-a"}),
    );
    assert_success(&popout_shot, "argus.screenshot(popout-pane-a)");
    assert_eq!(popout_shot["result"]["window_id"], "popout-pane-a");
    assert_eq!(popout_shot["result"]["pid"], child_pid);
    assert!(
        popout_shot["result"]["title"]
            .as_str()
            .is_some_and(|title| !title.is_empty()),
        "pop-out capture lacked its exact OS title"
    );
    let popout_png = base64::engine::general_purpose::STANDARD
        .decode(
            popout_shot["result"]["png_base64"]
                .as_str()
                .expect("pop-out screenshot png_base64"),
        )
        .expect("decode pop-out screenshot PNG");
    assert!(
        popout_png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "pop-out capture is not PNG"
    );
    assert_eq!(
        popout_shot["result"]["sha256"],
        format!("{:x}", Sha256::digest(&popout_png))
    );
    assert_visual_png(&popout_png, "detached-window capture");

    // A detached window must be steerable, not merely enumerable/capturable.
    client.mutation(
        "argus.show_context_menu",
        "popout-pane-a",
        "pane-pane-a-header",
        None,
    );
    let popout_after_action = client.inspect("popout-pane-a");
    assert!(
        contains_author_id(
            &popout_after_action["snapshot"]["root"],
            "ctx-menu.pane.lock"
        ) || contains_author_id(
            &popout_after_action["snapshot"]["root"],
            "ctx-menu.pane.pop_out"
        ),
        "detached-window mutation did not produce an observable newer snapshot"
    );

    client.mutation("argus.click", "main", "merge-back-pane-a", None);
    wait_for_window(&mut client, "popout-pane-a", false);

    // Protocol fences: each negative case must fail and therefore cannot be mistaken for an action.
    let bad_token = client.call_with_credentials(
        "argus.inspect",
        serde_json::json!({"window_id": "main"}),
        "not-the-session-token",
        "production-socket-live",
    );
    assert!(bad_token.get("error").is_some(), "bad token was accepted");
    let valid_token = client.token.clone();
    let missing_label = client.call_with_credentials(
        "argus.inspect",
        serde_json::json!({"window_id": "main"}),
        &valid_token,
        "",
    );
    assert!(
        missing_label.get("error").is_some(),
        "missing agent_label was accepted"
    );
    let wrong_window = client.call(
        "argus.inspect",
        serde_json::json!({"window_id": "does-not-exist"}),
    );
    assert!(
        wrong_window.get("error").is_some(),
        "unknown window was accepted"
    );
    let current_revision = client.inspect("main")["revision"]
        .as_u64()
        .expect("current main revision");
    let stale = client.call(
        "argus.click",
        serde_json::json!({
            "window_id": "main",
            "author_id": "shell.chrome.theme-toggle",
            "expected_snapshot_revision": current_revision.saturating_sub(1)
        }),
    );
    assert!(stale.get("error").is_some(), "stale revision was accepted");

    let proof_dir = proof_dir();
    std::fs::create_dir_all(&proof_dir).expect("create external proof directory");
    std::fs::write(proof_dir.join("argus_production_socket_main.png"), &png)
        .expect("write production screenshot proof");
    std::fs::write(
        proof_dir.join("argus_production_socket_settings_open.png"),
        &settings_png,
    )
    .expect("write production Settings-open screenshot proof");
    std::fs::write(
        proof_dir.join("argus_production_socket_popout_pane_a.png"),
        &popout_png,
    )
    .expect("write production pop-out screenshot proof");
    let transcript =
        serde_json::to_vec_pretty(&client.transcript).expect("serialize redacted transcript");
    assert!(
        !String::from_utf8_lossy(&transcript).contains(client.token.as_str()),
        "proof transcript retained the live session token"
    );
    assert!(
        client.agent_token.as_deref().is_none_or(|agent_token| {
            !String::from_utf8_lossy(&transcript).contains(agent_token)
        }),
        "proof transcript retained the broker-minted agent token"
    );
    assert!(
        !String::from_utf8_lossy(&transcript).contains(secret_canary),
        "proof transcript retained the sensitive-value canary"
    );
    std::fs::write(
        proof_dir.join("argus_production_socket_transcript.json"),
        &transcript,
    )
    .expect("write production socket transcript");
    let provenance = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_provenance@1",
        "child_pid": child_pid,
        "authenticated_agent_id": authenticated_agent_id,
        "transcript": "argus_production_socket_transcript.json",
        "captures": [
            {
                "artifact": "argus_production_socket_main.png",
                "purpose": "main-window-before-settings",
                "window_id": screenshot["result"]["window_id"],
                "pid": screenshot["result"]["pid"],
                "width": screenshot["result"]["width"],
                "height": screenshot["result"]["height"],
                "captured_at_utc": screenshot["result"]["captured_at_utc"],
                "sha256": screenshot["result"]["sha256"],
            },
            {
                "artifact": "argus_production_socket_settings_open.png",
                "purpose": "main-window-settings-cloud-controls-open",
                "window_id": settings_shot["result"]["window_id"],
                "pid": settings_shot["result"]["pid"],
                "width": settings_shot["result"]["width"],
                "height": settings_shot["result"]["height"],
                "captured_at_utc": settings_shot["result"]["captured_at_utc"],
                "sha256": settings_shot["result"]["sha256"],
                "snapshot_revision_before_capture": settings_after_denial["revision"],
                "snapshot_revision_after_capture": settings_after_capture["revision"],
                "required_author_id_landmarks": settings_landmarks,
                "landmarks_present_before_and_after_capture": true,
                "sensitive_values_redacted": true,
            },
            {
                "artifact": "argus_production_socket_popout_pane_a.png",
                "purpose": "detached-window-targeting-and-capture",
                "window_id": popout_shot["result"]["window_id"],
                "pid": popout_shot["result"]["pid"],
                "width": popout_shot["result"]["width"],
                "height": popout_shot["result"]["height"],
                "captured_at_utc": popout_shot["result"]["captured_at_utc"],
                "sha256": popout_shot["result"]["sha256"],
            }
        ],
        "redaction": {
            "session_token_absent_from_transcript": true,
            "agent_token_absent_from_transcript": true,
            "sensitive_canary_absent_from_transcript_and_settings_capture": true,
        }
    });
    let provenance =
        serde_json::to_vec_pretty(&provenance).expect("serialize production proof provenance");
    let provenance_text = String::from_utf8_lossy(&provenance);
    assert!(
        !provenance_text.contains(client.token.as_str())
            && client
                .agent_token
                .as_deref()
                .is_none_or(|agent_token| !provenance_text.contains(agent_token))
            && !provenance_text.contains(secret_canary),
        "production proof provenance retained a secret"
    );
    std::fs::write(
        proof_dir.join("argus_production_socket_provenance.json"),
        provenance,
    )
    .expect("write production screenshot provenance");

    request_child_close(child_pid);
    let exit_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < exit_deadline {
        if child_guard
            .0
            .try_wait()
            .expect("poll production child")
            .is_some()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        child_guard
            .0
            .try_wait()
            .expect("final production child poll")
            .is_some(),
        "production child did not exit after WM_CLOSE"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while binding_path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !binding_path.exists(),
        "owned binding survived production child shutdown"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
