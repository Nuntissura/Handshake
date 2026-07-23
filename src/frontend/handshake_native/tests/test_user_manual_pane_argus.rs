//! Headless AccessKit/Argus proof for the real Rust-native UserManual pane.

use std::{path::PathBuf, sync::Arc};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::backend_client::{HttpMethod, UserManualClient};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneRecord, PaneRenderContext, PaneType,
};
use handshake_native::user_manual_pane::{
    page_author_id, search_hit_author_id, UserManualNavigation, UserManualPageContent,
    UserManualPageSummary, UserManualPaneController, UserManualPaneFactory, UserManualResultCell,
    UserManualSearchHit, UserManualSearchResponse, UserManualTransport, ERROR_AUTHOR_ID,
    LOADING_AUTHOR_ID, NAVIGATION_AUTHOR_ID, PAGE_AUTHOR_ID, READ_RECEIPT_AUTHOR_ID,
    RETRY_AUTHOR_ID, SEARCH_ACTION_AUTHOR_ID, SEARCH_INPUT_AUTHOR_ID, SEARCH_RESULTS_AUTHOR_ID,
    SEARCH_STATUS_AUTHOR_ID, SURFACE_AUTHOR_ID, UNAVAILABLE_AUTHOR_ID,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const USER_MANUAL_CONTRACT_FILE: &str = "mt017-user-manual-native-contract.json";
const USER_MANUAL_SCREENSHOT_FILE: &str = "mt017-user-manual-pane.png";
const USER_MANUAL_CONTRACT_SCHEMA_ID: &str = "hsk.user_manual_native_contract_fixture@1";
const USER_MANUAL_PROVENANCE_SCHEMA_ID: &str = "hsk.mt017_user_manual_contract_provenance@1";
const MAX_PRODUCER_AGE_MS: u64 = 30 * 60 * 1_000;

#[derive(Clone, Deserialize)]
struct BackendUserManualContract {
    schema_id: String,
    navigation: UserManualNavigation,
    page: UserManualPageContent,
    search: UserManualSearchResponse,
}

#[derive(Clone, Deserialize)]
struct BackendUserManualProvenance {
    schema_id: String,
    proof_nonce: String,
    contract_schema_id: String,
    artifact_sha256: String,
    producer_test_id: String,
    producer_status: String,
    producer_completed_at_unix_ms: u64,
}

struct BackendContractTransport {
    contract: BackendUserManualContract,
}

impl UserManualTransport for BackendContractTransport {
    fn fetch_navigation(&self, cell: UserManualResultCell<UserManualNavigation>) {
        *cell.lock().expect("navigation cell") = Some(Ok(self.contract.navigation.clone()));
    }

    fn fetch_page(&self, _slug: &str, cell: UserManualResultCell<UserManualPageContent>) {
        *cell.lock().expect("page cell") = Some(Ok(self.contract.page.clone()));
    }

    fn fetch_search(&self, _query: &str, cell: UserManualResultCell<UserManualSearchResponse>) {
        *cell.lock().expect("search cell") = Some(Ok(self.contract.search.clone()));
    }
}

fn canonical_user_manual_contract_path() -> Result<PathBuf, String> {
    let configured = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
        .map_err(|_| "HANDSHAKE_ARTIFACTS_DIR must name Handshake_Artifacts")?;
    let configured = std::fs::canonicalize(configured)
        .map_err(|error| format!("canonicalize HANDSHAKE_ARTIFACTS_DIR: {error}"))?;
    let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .map_err(|error| format!("canonicalize native manifest: {error}"))?;
    let worktree_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .ok_or_else(|| "native crate must live below worktree src".to_owned())?;
    let expected = std::fs::canonicalize(
        worktree_root
            .parent()
            .ok_or_else(|| "worktree must have a parent".to_owned())?
            .join("Handshake_Artifacts"),
    )
    .map_err(|error| format!("canonicalize sibling Handshake_Artifacts: {error}"))?;
    if configured != expected {
        return Err(format!(
            "HANDSHAKE_ARTIFACTS_DIR must be the canonical sibling root: expected {}, got {}",
            expected.display(),
            configured.display()
        ));
    }
    Ok(configured
        .join("handshake-test")
        .join("wp1-final-audit")
        .join(USER_MANUAL_CONTRACT_FILE))
}

fn validate_user_manual_provenance(
    provenance: &BackendUserManualProvenance,
    bytes: &[u8],
    expected_nonce: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    if provenance.schema_id != USER_MANUAL_PROVENANCE_SCHEMA_ID {
        return Err("unsupported UserManual provenance schema".to_owned());
    }
    if provenance.proof_nonce != expected_nonce {
        return Err("UserManual provenance nonce mismatch".to_owned());
    }
    if provenance.contract_schema_id != USER_MANUAL_CONTRACT_SCHEMA_ID {
        return Err("UserManual contract schema mismatch".to_owned());
    }
    if provenance.producer_test_id != "mt201_search_finds_pages_and_tools"
        || provenance.producer_status != "passed_all_backend_assertions"
    {
        return Err("UserManual producer did not pass the canonical backend test".to_owned());
    }
    if provenance.producer_completed_at_unix_ms > now_unix_ms.saturating_add(30_000) {
        return Err("UserManual provenance timestamp is in the future".to_owned());
    }
    if now_unix_ms.saturating_sub(provenance.producer_completed_at_unix_ms) > MAX_PRODUCER_AGE_MS {
        return Err("UserManual provenance is stale".to_owned());
    }
    if provenance.artifact_sha256 != format!("{:x}", Sha256::digest(bytes)) {
        return Err("UserManual artifact hash mismatch".to_owned());
    }
    Ok(())
}

fn backend_user_manual_contract() -> BackendUserManualContract {
    let path = canonical_user_manual_contract_path().expect("canonical UserManual contract path");
    let configured = PathBuf::from(
        std::env::var("HANDSHAKE_MT017_USER_MANUAL_CONTRACT_ARTIFACT")
            .expect("HANDSHAKE_MT017_USER_MANUAL_CONTRACT_ARTIFACT is required"),
    );
    let configured = std::fs::canonicalize(
        configured
            .parent()
            .expect("configured UserManual artifact has a parent"),
    )
    .expect("configured UserManual artifact parent resolves")
    .join(
        configured
            .file_name()
            .expect("configured UserManual artifact has a file name"),
    );
    assert_eq!(
        configured, path,
        "configured UserManual artifact is canonical"
    );
    let bytes = std::fs::read(&path).expect("read backend-produced UserManual contract");
    let contract: BackendUserManualContract =
        serde_json::from_slice(&bytes).expect("typed UserManual contract JSON");
    assert_eq!(contract.schema_id, USER_MANUAL_CONTRACT_SCHEMA_ID);
    let provenance: BackendUserManualProvenance = serde_json::from_slice(
        &std::fs::read(path.with_extension("provenance.json")).expect("read UserManual provenance"),
    )
    .expect("typed UserManual provenance JSON");
    let nonce = std::env::var("HANDSHAKE_MT017_USER_MANUAL_PROOF_NONCE")
        .expect("HANDSHAKE_MT017_USER_MANUAL_PROOF_NONCE is required");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_millis() as u64;
    validate_user_manual_provenance(&provenance, &bytes, &nonce, now)
        .expect("fresh backend-produced UserManual contract provenance");
    contract
}

struct FixedManualTransport {
    fail: bool,
}

struct PendingManualTransport;

impl UserManualTransport for PendingManualTransport {
    fn fetch_navigation(&self, _cell: UserManualResultCell<UserManualNavigation>) {}

    fn fetch_page(&self, _slug: &str, _cell: UserManualResultCell<UserManualPageContent>) {}

    fn fetch_search(&self, _query: &str, _cell: UserManualResultCell<UserManualSearchResponse>) {}
}

impl UserManualTransport for FixedManualTransport {
    fn fetch_navigation(&self, cell: UserManualResultCell<UserManualNavigation>) {
        let result = if self.fail {
            Err("backend unavailable".to_string())
        } else {
            Ok(UserManualNavigation {
                manual_version: "2.0.11".to_string(),
                route_namespace: "/usermanual".to_string(),
                pages: vec![UserManualPageSummary {
                    slug: "model-runtime-registry-and-loom-degrade".to_string(),
                    title: "Model Runtime Registry and Loom Semantic Degrade".to_string(),
                    page_kind: "workflow".to_string(),
                    audience: "operator,model".to_string(),
                    manual_version: "2.0.11".to_string(),
                    content_hash: "hash-1".to_string(),
                    status: "current".to_string(),
                }],
            })
        };
        *cell.lock().expect("navigation cell") = Some(result);
    }

    fn fetch_page(&self, _slug: &str, cell: UserManualResultCell<UserManualPageContent>) {
        *cell.lock().expect("page cell") = Some(Ok(UserManualPageContent {
            page: serde_json::json!({
                "slug": "model-runtime-registry-and-loom-degrade",
                "title": "Model Runtime Registry and Loom Semantic Degrade",
                "manual_version": "2.0.11"
            }),
            sections: vec![serde_json::json!({
                "heading": "Loom dimension mismatch and recovery",
                "body_markdown": "Loom degrades to keyword/trigram and reports DimMismatch{expected, actual}."
            })],
            anchors: Vec::new(),
            bootstrap_receipt_event_id: "event-user-manual-open-1".to_string(),
            bootstrap_identity_used: true,
        }));
    }

    fn fetch_search(&self, query: &str, cell: UserManualResultCell<UserManualSearchResponse>) {
        let result = if self.fail {
            Err("backend unavailable".to_string())
        } else {
            Ok(UserManualSearchResponse {
                query: query.to_string(),
                count: 1,
                results: vec![UserManualSearchHit {
                    result_kind: "section".to_string(),
                    result_ref: "model-runtime-registry-and-loom-degrade:recovery".to_string(),
                    page_slug: Some("model-runtime-registry-and-loom-degrade".to_string()),
                    title: "Loom dimension mismatch and recovery".to_string(),
                    excerpt: "Loom degrades to keyword/trigram".to_string(),
                }],
            })
        };
        *cell.lock().expect("search cell") = Some(result);
    }
}

struct ManualHarnessState {
    factory: UserManualPaneFactory,
    record: PaneRecord,
}

fn render(ctx: &egui::Context, state: &mut ManualHarnessState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        state.factory.render(
            ui,
            &PaneRenderContext {
                record: &state.record,
                egui_id: egui::Id::new("user-manual-test-pane"),
            },
        );
    });
}

fn state(factory: UserManualPaneFactory) -> ManualHarnessState {
    ManualHarnessState {
        factory,
        record: PaneRecord::new(
            Arc::from("pane-a"),
            PaneType::UserManual,
            "default-project",
            Some("model-runtime-registry-and-loom-degrade".to_string()),
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ),
    }
}

fn author_ids(harness: &Harness<'_, ManualHarnessState>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn node_by_author<'a>(
    harness: &'a Harness<'_, ManualHarnessState>,
    author_id: &str,
) -> egui_kittest::Node<'a> {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("missing {author_id}: {:?}", author_ids(harness)))
}

#[test]
fn user_manual_route_content_is_reachable_and_argus_inspectable() {
    let factory =
        UserManualPaneFactory::with_transport(Arc::new(FixedManualTransport { fail: false }));
    let mut harness = Harness::builder().build_state(render, state(factory));
    harness.run();
    harness.run();
    harness.run();

    let ids = author_ids(&harness);
    for expected in [
        SURFACE_AUTHOR_ID.to_string(),
        NAVIGATION_AUTHOR_ID.to_string(),
        PAGE_AUTHOR_ID.to_string(),
        READ_RECEIPT_AUTHOR_ID.to_string(),
        page_author_id("model-runtime-registry-and-loom-degrade"),
    ] {
        assert!(ids.contains(&expected), "missing {expected}: {ids:?}");
    }
    harness.get_by_label(
        "Loom degrades to keyword/trigram and reports DimMismatch{expected, actual}.",
    );
    harness.get_by_label("Read receipt: event-user-manual-open-1");
}

#[test]
fn mt014_page_uses_the_production_usermanual_transport_route() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let client = UserManualClient::new("http://127.0.0.1:37501/", runtime.handle().clone());
    let request = client.page_request("model-runtime-registry-and-loom-degrade");
    assert_eq!(request.method, HttpMethod::Get);
    assert_eq!(
        request.url,
        "http://127.0.0.1:37501/usermanual/pages/model-runtime-registry-and-loom-degrade"
    );
    assert!(request.body.is_none());
}

#[test]
fn user_manual_search_uses_encoded_backend_route_and_renders_navigable_results() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let client = UserManualClient::new("http://127.0.0.1:37501/", runtime.handle().clone());
    let request = client.search_request("loom recovery & retry");
    assert_eq!(request.method, HttpMethod::Get);
    assert_eq!(
        request.url,
        "http://127.0.0.1:37501/usermanual/search?q=loom+recovery+%26+retry&limit=50"
    );
    assert!(request.body.is_none());

    let controller = UserManualPaneController::default();
    let factory = UserManualPaneFactory::with_transport_and_controller(
        Arc::new(FixedManualTransport { fail: false }),
        controller.clone(),
    );
    let mut harness = Harness::builder().build_state(render, state(factory));
    harness.run();
    harness.run();
    harness.run();
    controller.request_search_focus();
    harness.run();
    node_by_author(&harness, SEARCH_INPUT_AUTHOR_ID).type_text("loom recovery");
    harness.run();
    node_by_author(&harness, SEARCH_ACTION_AUTHOR_ID).click();
    harness.run();
    harness.run();

    let ids = author_ids(&harness);
    for expected in [
        SEARCH_INPUT_AUTHOR_ID.to_string(),
        SEARCH_ACTION_AUTHOR_ID.to_string(),
        SEARCH_STATUS_AUTHOR_ID.to_string(),
        SEARCH_RESULTS_AUTHOR_ID.to_string(),
        search_hit_author_id(
            "section",
            "model-runtime-registry-and-loom-degrade:recovery",
        ),
    ] {
        assert!(ids.contains(&expected), "missing {expected}: {ids:?}");
    }
    harness.get_by_label("1 UserManual result(s) for 'loom recovery'");
    node_by_author(
        &harness,
        &search_hit_author_id(
            "section",
            "model-runtime-registry-and-loom-degrade:recovery",
        ),
    )
    .click();
    harness.run();
    harness.run();
    harness.get_by_label(
        "Loom degrades to keyword/trigram and reports DimMismatch{expected, actual}.",
    );
}

#[test]
fn user_manual_contract_provenance_rejects_nonce_status_age_and_hash_drift() {
    let bytes = br#"{"schema_id":"hsk.user_manual_native_contract_fixture@1"}"#;
    let now = 2_000_000_000_u64;
    let mut provenance = BackendUserManualProvenance {
        schema_id: USER_MANUAL_PROVENANCE_SCHEMA_ID.to_string(),
        proof_nonce: "current".to_string(),
        contract_schema_id: USER_MANUAL_CONTRACT_SCHEMA_ID.to_string(),
        artifact_sha256: format!("{:x}", Sha256::digest(bytes)),
        producer_test_id: "mt201_search_finds_pages_and_tools".to_string(),
        producer_status: "passed_all_backend_assertions".to_string(),
        producer_completed_at_unix_ms: now - 1_000,
    };
    validate_user_manual_provenance(&provenance, bytes, "current", now)
        .expect("fresh backend proof accepted");

    provenance.proof_nonce = "old".to_string();
    assert!(
        validate_user_manual_provenance(&provenance, bytes, "current", now)
            .unwrap_err()
            .contains("nonce")
    );
    provenance.proof_nonce = "current".to_string();
    provenance.producer_status = "started".to_string();
    assert!(
        validate_user_manual_provenance(&provenance, bytes, "current", now)
            .unwrap_err()
            .contains("producer")
    );
    provenance.producer_status = "passed_all_backend_assertions".to_string();
    provenance.producer_completed_at_unix_ms = now - MAX_PRODUCER_AGE_MS - 1;
    assert!(
        validate_user_manual_provenance(&provenance, bytes, "current", now)
            .unwrap_err()
            .contains("stale")
    );
    provenance.producer_completed_at_unix_ms = now - 1_000;
    provenance.artifact_sha256 = "00".repeat(32);
    assert!(
        validate_user_manual_provenance(&provenance, bytes, "current", now)
            .unwrap_err()
            .contains("hash")
    );
}

#[test]
#[ignore = "requires a fresh real-PostgreSQL backend UserManual contract artifact"]
fn native_consumes_fresh_backend_user_manual_contract_artifact() {
    let contract = backend_user_manual_contract();
    assert!(!contract.navigation.pages.is_empty());
    assert_eq!(contract.search.query, "backlinks");
    assert_eq!(contract.search.count, contract.search.results.len());
    assert!(!contract.search.results.is_empty());
    assert!(contract
        .navigation
        .pages
        .iter()
        .any(|page| page.slug == "model-runtime-registry-and-loom-degrade"));

    let first_hit = contract.search.results[0].clone();
    let expected_search_status = format!(
        "{} UserManual result(s) for '{}'",
        contract.search.count, contract.search.query
    );
    let factory =
        UserManualPaneFactory::with_transport(Arc::new(BackendContractTransport { contract }));
    let mut harness = Harness::builder().build_state(render, state(factory));
    harness.run_steps(5);
    node_by_author(&harness, SEARCH_INPUT_AUTHOR_ID).focus();
    harness.step();
    node_by_author(&harness, SEARCH_INPUT_AUTHOR_ID).type_text("backlinks");
    harness.step();
    node_by_author(&harness, SEARCH_ACTION_AUTHOR_ID).click();
    harness.run_steps(4);

    harness.get_by_label(&expected_search_status);
    let result_id = search_hit_author_id(&first_hit.result_kind, &first_hit.result_ref);
    assert!(
        author_ids(&harness).contains(&result_id),
        "backend search result has a stable native target"
    );
    harness.get_by_label("Registry and catalog contract");

    use image::ImageEncoder;

    let image = harness
        .render()
        .expect("GPU host renders the fresh backend UserManual frame");
    let (width, height) = (image.width(), image.height());
    let pixels = image.as_raw();
    let opaque_pixels = pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[3] == u8::MAX)
        .count();
    let distinct_rgb = pixels
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<std::collections::BTreeSet<_>>();
    let duplicate_id_diagnostic_pixels = pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 200 && pixel[1] < 80 && pixel[2] < 80)
        .count();
    let pixel_count = usize::try_from(width * height).expect("frame pixel count fits usize");
    assert!(
        opaque_pixels * 100 >= pixel_count * 99,
        "the governed UserManual frame must be at least 99% opaque: \
         opaque={opaque_pixels}, total={pixel_count}"
    );
    assert!(
        distinct_rgb.len() > 32,
        "the governed UserManual frame must contain rendered detail, got only {} colors",
        distinct_rgb.len()
    );
    assert_eq!(
        duplicate_id_diagnostic_pixels, 0,
        "the governed UserManual frame must not contain egui duplicate-widget-ID diagnostics"
    );

    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .expect("encode UserManual PNG");
    assert!(
        png.len() > 1024,
        "rendered UserManual PNG must not be blank"
    );

    let contract_path = canonical_user_manual_contract_path()
        .expect("resolve canonical backend UserManual contract path");
    let screenshot_path = contract_path.with_file_name(USER_MANUAL_SCREENSHOT_FILE);
    let contract_bytes = std::fs::read(&contract_path).expect("read source UserManual contract");
    let proof_nonce = std::env::var("HANDSHAKE_MT017_USER_MANUAL_PROOF_NONCE")
        .expect("UserManual screenshot requires the producer nonce");
    let screenshot_sha256 = format!("{:x}", Sha256::digest(&png));
    std::fs::write(&screenshot_path, &png).expect("write UserManual screenshot");
    let producer_completed_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as u64;
    std::fs::write(
        screenshot_path.with_extension("provenance.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "hsk.mt017_user_manual_screenshot_provenance@1",
            "proof_nonce": proof_nonce,
            "source_contract_schema_id": USER_MANUAL_CONTRACT_SCHEMA_ID,
            "source_contract_sha256": format!("{:x}", Sha256::digest(&contract_bytes)),
            "screenshot_sha256": screenshot_sha256,
            "producer_test_id": "native_consumes_fresh_backend_user_manual_contract_artifact",
            "producer_status": "passed_all_native_and_pixel_assertions",
            "producer_completed_at_unix_ms": producer_completed_at_unix_ms,
        }))
        .expect("serialize UserManual screenshot provenance"),
    )
    .expect("write UserManual screenshot provenance");
    println!(
        "REAL_USER_MANUAL_SCREENSHOT={} ({width}x{height})",
        screenshot_path.display()
    );
}

#[test]
fn user_manual_loading_state_has_stable_argus_target() {
    let factory = UserManualPaneFactory::with_transport(Arc::new(PendingManualTransport));
    let mut harness = Harness::builder().build_state(render, state(factory));
    harness.run();

    let ids = author_ids(&harness);
    assert!(
        ids.contains(&LOADING_AUTHOR_ID.to_string()),
        "loading state missing: {ids:?}"
    );
    harness.get_by_label("Loading UserManual content...");
}

#[test]
fn user_manual_unavailable_and_error_retry_states_have_stable_targets() {
    let mut offline =
        Harness::builder().build_state(render, state(UserManualPaneFactory::offline()));
    offline.run();
    assert!(author_ids(&offline).contains(&UNAVAILABLE_AUTHOR_ID.to_string()));

    let factory =
        UserManualPaneFactory::with_transport(Arc::new(FixedManualTransport { fail: true }));
    let mut failed = Harness::builder().build_state(render, state(factory));
    failed.run();
    failed.run();
    let ids = author_ids(&failed);
    assert!(
        ids.contains(&ERROR_AUTHOR_ID.to_string()),
        "error state missing: {ids:?}"
    );
    assert!(
        ids.contains(&RETRY_AUTHOR_ID.to_string()),
        "retry state missing: {ids:?}"
    );
}
