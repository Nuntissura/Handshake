//! WP-1 MT-014: backend-independent projection provenance guards.

use handshake_native::model_runtime_panel::PROJECTION_SCHEMA_ID;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const PROJECTION_PROVENANCE_SCHEMA_ID: &str = "hsk.mt014_model_runtime_projection_provenance@1";
const MAX_PRODUCER_AGE_MS: u64 = 30 * 60 * 1_000;

#[derive(Clone, Debug, Deserialize)]
struct ModelRuntimeProjectionProvenance {
    schema_id: String,
    proof_nonce: String,
    projection_schema_id: String,
    artifact_sha256: String,
    producer_test_id: String,
    producer_status: String,
    producer_completed_at_unix_ms: u64,
}

fn validate_configured_artifact_path(
    configured: &std::path::Path,
    expected: &std::path::Path,
    variable: &str,
) -> Result<(), String> {
    let configured = match (configured.parent(), configured.file_name()) {
        (Some(parent), Some(file_name)) => std::fs::canonicalize(parent)
            .map(|parent| parent.join(file_name))
            .unwrap_or_else(|_| configured.to_path_buf()),
        _ => configured.to_path_buf(),
    };
    if configured != expected {
        return Err(format!(
            "{variable} must name canonical artifact {}; got {}",
            expected.display(),
            configured.display()
        ));
    }
    Ok(())
}

fn validate_projection_provenance(
    provenance: &ModelRuntimeProjectionProvenance,
    artifact_bytes: &[u8],
    expected_nonce: &str,
    projection_schema_id: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    if provenance.schema_id != PROJECTION_PROVENANCE_SCHEMA_ID {
        return Err(format!(
            "unsupported provenance schema `{}`",
            provenance.schema_id
        ));
    }
    if provenance.proof_nonce != expected_nonce {
        return Err("projection provenance nonce does not match this producer run".to_owned());
    }
    if provenance.projection_schema_id != projection_schema_id {
        return Err("projection schema does not match producer provenance".to_owned());
    }
    if provenance.producer_test_id
        != "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256"
    {
        return Err("projection provenance names an unexpected producer".to_owned());
    }
    if provenance.producer_status != "passed_all_backend_assertions" {
        return Err("projection producer did not complete every backend assertion".to_owned());
    }
    if provenance.producer_completed_at_unix_ms > now_unix_ms.saturating_add(30_000) {
        return Err("projection provenance timestamp is in the future".to_owned());
    }
    if now_unix_ms.saturating_sub(provenance.producer_completed_at_unix_ms) > MAX_PRODUCER_AGE_MS {
        return Err("projection provenance is stale".to_owned());
    }
    let actual_sha256 = format!("{:x}", Sha256::digest(artifact_bytes));
    if provenance.artifact_sha256 != actual_sha256 {
        return Err("projection bytes do not match producer provenance".to_owned());
    }
    Ok(())
}

#[test]
fn mt014_projection_provenance_rejects_stale_nonce_root_status_and_hash() {
    let bytes = br#"{"schema_id":"hsk.model_runtime_registry_projection@3"}"#;
    let now = 2_000_000_000_u64;
    let mut provenance = ModelRuntimeProjectionProvenance {
        schema_id: PROJECTION_PROVENANCE_SCHEMA_ID.to_owned(),
        proof_nonce: "current-run".to_owned(),
        projection_schema_id: PROJECTION_SCHEMA_ID.to_owned(),
        artifact_sha256: format!("{:x}", Sha256::digest(bytes)),
        producer_test_id:
            "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256".to_owned(),
        producer_status: "passed_all_backend_assertions".to_owned(),
        producer_completed_at_unix_ms: now - 1_000,
    };
    validate_projection_provenance(&provenance, bytes, "current-run", PROJECTION_SCHEMA_ID, now)
        .expect("fresh matching producer provenance is accepted");

    provenance.proof_nonce = "old-run".to_owned();
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("nonce"));
    provenance.proof_nonce = "current-run".to_owned();

    provenance.producer_completed_at_unix_ms = now - MAX_PRODUCER_AGE_MS - 1;
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("stale"));
    provenance.producer_completed_at_unix_ms = now - 1_000;

    provenance.producer_status = "failed_before_publish".to_owned();
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("did not complete"));
    provenance.producer_status = "passed_all_backend_assertions".to_owned();

    provenance.artifact_sha256 = "0".repeat(64);
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("bytes"));

    assert!(validate_configured_artifact_path(
        std::path::Path::new("D:/wrong-root/projection.json"),
        std::path::Path::new("D:/canonical-root/projection.json"),
        "HANDSHAKE_MT014_PROJECTION_ARTIFACT"
    )
    .unwrap_err()
    .contains("canonical artifact"));
}


// Component coverage of the actual renderer/transport seam; authenticated native
// HTTP, canonical event identity and fault-injection proof remain independent.
mod selection_intent {
    use std::sync::{Arc, Mutex};

    use egui_kittest::{kittest::{NodeT, Queryable}, Harness};
    use handshake_native::{
        model_runtime_panel::{
            self, ModelRuntimePaneFactory, ModelRuntimeRegistryCell,
            ModelRuntimeRegistryProjection, ModelRuntimeRegistryTransport,
            ModelRuntimeSelectionIntent,
        },
        pane_registry::{
            DirtyState, LockState, PaneAuthority, PaneFactory, PaneRecord,
            PaneRenderContext, PaneType,
        },
    };
    use serde_json::json;

    const PANE: &str = "model-selection-component";

    #[derive(Default)]
    struct Transport {
        fetches: Mutex<Vec<ModelRuntimeRegistryCell>>,
        selections: Mutex<Vec<(ModelRuntimeSelectionIntent, ModelRuntimeRegistryCell)>>,
    }

    impl ModelRuntimeRegistryTransport for Transport {
        fn fetch_registry(&self, cell: ModelRuntimeRegistryCell) {
            self.fetches.lock().unwrap().push(cell);
        }

        fn select_model(&self, intent: ModelRuntimeSelectionIntent, cell: ModelRuntimeRegistryCell) {
            self.selections.lock().unwrap().push((intent, cell));
        }
    }

    fn projection(selected: &str, revision: Option<u64>) -> ModelRuntimeRegistryProjection {
        let unavailable = json!({"status": "unavailable", "reason": "component observation unavailable"});
        let disabled = json!({"enabled": false, "reason": "component runtime control unavailable"});
        let rows: Vec<_> = [("current", "a"), ("target", "b")].into_iter().map(|(model, digit)| {
            let active = model == selected && revision.is_some();
            let artifact = digit.repeat(64);
            json!({
                "artifact_sha256": artifact,
                "artifact_locator": format!("sha256:{}", digit.repeat(64)),
                "display_label": model,
                "selected_adapter": "candle",
                "selection_revision": 11,
                "selection_audit_event_ref": "eventledger://kernel/component-row",
                "runtime_role": "completion",
                "default_selectable": true,
                "runtime_state": "live",
                "active_purposes": if active { vec!["application/default"] } else { Vec::<&str>::new() },
                "active_selection_revision": if active { revision } else { None },
                "selected": active,
                "live_model_id": model,
                "canonical_artifact_path": unavailable.clone(),
                "kv_cache": unavailable.clone(),
                "lora_stack": unavailable.clone(),
                "active_steering": unavailable.clone(),
                "process_ownership_ledger_link": unavailable.clone(),
                "tokens_per_second": unavailable.clone(),
                "vram_resident_bytes": unavailable.clone(),
                "last_call_at_utc": unavailable.clone(),
                "last_call_age_seconds": unavailable.clone(),
                "engine_internals": unavailable.clone(),
                "quiesce_action": disabled.clone(),
                "unload_action": disabled.clone(),
                "compatible_adapter_swap_action": disabled.clone(),
                "inspect_engine_internals_action": disabled.clone(),
            })
        }).collect();
        let projection = serde_json::from_value(json!({
            "schema_id": model_runtime_panel::PROJECTION_SCHEMA_ID,
            "generated_at_utc": "component-observation",
            "catalog_revision": 20,
            "selection_receipt_ref": "eventledger://kernel/component-observation",
            "rows": rows,
        })).unwrap();
        model_runtime_panel::validate_projection_for_native_surface(&projection).unwrap();
        projection
    }

    fn harness(transport: Arc<Transport>) -> Harness<'static, ModelRuntimePaneFactory> {
        Harness::builder().with_size(egui::vec2(1200.0, 2400.0)).build_state(
            |ctx, panel: &mut ModelRuntimePaneFactory| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let record = PaneRecord::new(
                        Arc::from(PANE), PaneType::ModelRuntime, "component-workspace", None,
                        LockState::Unlocked, DirtyState::Clean, PaneAuthority::Human,
                    );
                    panel.render(ui, &PaneRenderContext { record: &record, egui_id: egui::Id::new(PANE) });
                });
            },
            ModelRuntimePaneFactory::with_transport(transport),
        )
    }

    fn node<'a>(harness: &'a Harness<'_, ModelRuntimePaneFactory>, id: &str) -> egui_kittest::Node<'a> {
        harness.root().children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(id))
            .unwrap_or_else(|| panic!("missing production control {id}"))
    }

    fn has_node(harness: &Harness<'_, ModelRuntimePaneFactory>, id: &str) -> bool {
        harness.root().children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some(id))
    }

    fn switch_id(model: &str) -> String {
        let digit = if model == "current" { "a" } else { "b" };
        model_runtime_panel::row_switch_author_id(PANE, &digit.repeat(64))
    }

    fn deliver_fetch(transport: &Transport, projection: ModelRuntimeRegistryProjection) {
        let cell = transport.fetches.lock().unwrap().pop().expect("actual renderer requested registry");
        *cell.lock().unwrap() = Some(Ok(projection));
    }

    #[test]
    fn displayed_revision_and_entire_uncertain_retry_intent_reach_transport() {
        let transport = Arc::new(Transport::default());
        let mut harness = harness(transport.clone());
        harness.run();
        deliver_fetch(&transport, projection("current", Some(7)));
        harness.run();
        node(&harness, &switch_id("target")).click_accesskit();
        harness.run();
        let (first, cell) = transport.selections.lock().unwrap()[0].clone();
        assert_eq!(first.target_model_id, "target");
        assert_eq!(first.expected_selection_revision, 7);
        assert!(!first.request_id.is_nil());
        *cell.lock().unwrap() = Some(Err("opaque transport failure; certainty unavailable".to_owned()));
        harness.run();
        assert!(node(&harness, &switch_id("target")).accesskit_node().is_disabled());
        node(&harness, &model_runtime_panel::refresh_author_id(PANE)).click_accesskit();
        harness.run();
        deliver_fetch(&transport, projection("current", Some(7)));
        harness.run();
        node(&harness, &model_runtime_panel::selection_retry_author_id(PANE)).click_accesskit();
        harness.run();
        let (retry, cell) = transport.selections.lock().unwrap()[1].clone();
        assert_eq!(retry, first, "retry must not rebase or remint an uncertain intent");
        *cell.lock().unwrap() = Some(Ok(projection("target", Some(8))));
        harness.run();
        assert!(!has_node(&harness, &model_runtime_panel::selection_retry_author_id(PANE)));
        node(&harness, &switch_id("current")).click_accesskit();
        harness.run();
        let next = transport.selections.lock().unwrap()[2].0.clone();
        assert_eq!(next.expected_selection_revision, 8);
        assert_eq!(next.target_model_id, "current");
        assert_ne!(next.request_id, first.request_id);
        let next_cell = transport.selections.lock().unwrap()[2].1.clone();
        *next_cell.lock().unwrap() = Some(Err("uncertain second selection".to_owned()));
        harness.run();
        node(&harness, &model_runtime_panel::refresh_author_id(PANE)).click_accesskit();
        harness.run();
        deliver_fetch(&transport, projection("target", Some(100)));
        harness.run();
        node(&harness, &model_runtime_panel::selection_retry_author_id(PANE)).click_accesskit();
        harness.run();
        assert_eq!(transport.selections.lock().unwrap()[3].0, next,
            "a newly displayed revision must not rebase the retained retry");
    }

    #[test]
    fn absent_canonical_revision_disables_selection_but_observed_zero_is_retained() {
        for revision in [None, Some(0)] {
            let transport = Arc::new(Transport::default());
            let mut harness = harness(transport.clone());
            harness.run();
            deliver_fetch(&transport, projection("current", revision));
            harness.run();
            let target = switch_id("target");
            if revision.is_none() {
                assert!(node(&harness, &target).accesskit_node().is_disabled());
                assert!(transport.selections.lock().unwrap().is_empty());
            } else {
                node(&harness, &target).click_accesskit();
                harness.run();
                assert_eq!(transport.selections.lock().unwrap()[0].0.expected_selection_revision, 0);
            }
        }
    }

    #[test]
    fn mismatched_success_keeps_retry_and_explicit_discard_requires_refresh() {
        let transport = Arc::new(Transport::default());
        let mut harness = harness(transport.clone());
        harness.run();
        deliver_fetch(&transport, projection("current", Some(7)));
        harness.run();
        node(&harness, &switch_id("target")).click_accesskit();
        harness.run();
        let (first, cell) = transport.selections.lock().unwrap()[0].clone();
        *cell.lock().unwrap() = Some(Ok(projection("current", Some(8))));
        harness.run();
        assert!(has_node(&harness, &model_runtime_panel::selection_retry_author_id(PANE)));
        node(&harness, &model_runtime_panel::selection_retry_author_id(PANE)).click_accesskit();
        harness.run();
        let (retry, cell) = transport.selections.lock().unwrap()[1].clone();
        assert_eq!(retry, first);
        *cell.lock().unwrap() = Some(Ok(projection("target", Some(44))));
        harness.run();
        assert!(has_node(&harness, &model_runtime_panel::selection_retry_author_id(PANE)));
        node(&harness, &model_runtime_panel::selection_discard_author_id(PANE)).click_accesskit();
        harness.run();
        assert!(!has_node(&harness, &model_runtime_panel::selection_retry_author_id(PANE)));
        assert!(node(&harness, &switch_id("target")).accesskit_node().is_disabled());
        node(&harness, &model_runtime_panel::refresh_author_id(PANE)).click_accesskit();
        harness.run();
        deliver_fetch(&transport, projection("current", Some(44)));
        harness.run();
        node(&harness, &switch_id("target")).click_accesskit();
        harness.run();
        let next = transport.selections.lock().unwrap()[2].0.clone();
        assert_eq!(next.expected_selection_revision, 44);
        assert_ne!(next.request_id, first.request_id);
    }

    #[test]
    fn replacement_factory_does_not_consume_prior_selection_delivery() {
        let old_transport = Arc::new(Transport::default());
        let mut old = harness(old_transport.clone());
        old.run();
        deliver_fetch(&old_transport, projection("current", Some(7)));
        old.run();
        node(&old, &switch_id("target")).click_accesskit();
        old.run();
        let old_cell = old_transport.selections.lock().unwrap()[0].1.clone();
        drop(old);
        let transport = Arc::new(Transport::default());
        let mut replacement = harness(transport.clone());
        replacement.run();
        *old_cell.lock().unwrap() = Some(Ok(projection("target", Some(8))));
        deliver_fetch(&transport, projection("current", Some(40)));
        replacement.run();
        assert!(!has_node(&replacement, &model_runtime_panel::selection_retry_author_id(PANE)));
        node(&replacement, &switch_id("target")).click_accesskit();
        replacement.run();
        assert_eq!(transport.selections.lock().unwrap()[0].0.expected_selection_revision, 40);
    }
}
