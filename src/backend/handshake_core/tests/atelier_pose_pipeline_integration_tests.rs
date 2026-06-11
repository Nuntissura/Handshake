//! WP-KERNEL-005 MT-118 — cross-module Pose ↔ ComfyUI pipeline integration
//! proof against live PostgreSQL/EventLedger.
//!
//! This is the ONE end-to-end "Pose Integration Smoke Path" the MT-118 contract
//! names (`acceptance_criteria`: source image -> rig -> sidecar -> workflow
//! receipt -> history). It chains the REAL `AtelierStore` API across three
//! atelier submodules — `media`, `pose`, `comfy` — and asserts that each stage
//! both (a) persisted and (b) links correctly to the previous stage's record:
//!
//!   1. media   : `materialize_media_asset`     -> the source image asset
//!   2. pose    : `ingest_pose_rig`             -> rig references the source asset
//!   3. pose    : `record_pose_sidecar`         -> sidecar references the rig
//!   4. comfy   : `record_intake_output`        -> the generated/routed output
//!   5. comfy   : `record_comfy_workflow_receipt` -> receipt captures the output
//!   6. comfy   : `list_comfy_workflow_history` -> the receipt is discoverable
//!
//! Run-scoped: every id/hash/ref is unique per run (`Uuid::new_v4()` /
//! content-addressed artifacts), and every assertion is scoped to THIS run's own
//! rows (rig_id, run_id, character_ref). No global counts are taken — tables
//! persist between runs, so a global count would be flaky. Gated on
//! `atelier_pg_support::database_url()`; SKIPs cleanly when PostgreSQL is
//! unavailable. Only `handshake_core` + `tokio` + `uuid` + `serde_json` (+ std)
//! are used; sqlx is never imported directly.

mod atelier_pg_support;

use handshake_core::atelier::comfy::{
    ComfyWorkflowHistoryQuery, ComfyWorkflowStatus, MediaKind, NewComfyWorkflowReceipt,
    NewIntakeOutput, RoutingIntent,
};
use handshake_core::atelier::pose::{
    CanvasSize, DetectorStatus, NewPoseRig, NewPoseSidecar, PoseSidecarKind, PoseSidecarStatus,
    BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT,
};
use handshake_core::atelier::{AtelierStore, NewCharacter, NewMediaAsset};
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// A valid OpenPose keypoint payload (legacy `rigToOpenposeJson`): body-18 (the
/// only required field) plus zero-filled face-70 and both hands-21.
fn valid_keypoints() -> serde_json::Value {
    serde_json::json!({
        "people": [{
            "pose_keypoints_2d": vec![0.0_f64; BODY_KEYPOINT_COUNT * 3],
            "face_keypoints_2d": vec![0.0_f64; FACE_KEYPOINT_COUNT * 3],
            "hand_left_keypoints_2d": vec![0.0_f64; HAND_KEYPOINT_COUNT * 3],
            "hand_right_keypoints_2d": vec![0.0_f64; HAND_KEYPOINT_COUNT * 3],
        }]
    })
}

/// Derive the ArtifactStore manifest handle from a payload handle, the
/// convention pose sidecar refs require (`.../payload` -> `.../artifact.json`).
fn artifact_manifest_ref(artifact_ref: &str) -> String {
    artifact_ref.replace("/payload", "/artifact.json")
}

/// MT-118 — Pose ↔ ComfyUI pipeline integration chain.
///
/// Proves one minimal pipeline end-to-end against live PostgreSQL/EventLedger,
/// asserting at every hop that the cross-module reference links back to the
/// prior stage's persisted record.
#[tokio::test]
async fn mt118_pose_comfy_pipeline_integration_chain() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt118_pose_comfy_pipeline_integration_chain: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    // --- run-scoped identity: a fresh character (pose rows FK to character) ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-mt118-{}", Uuid::new_v4()),
            display_name: "MT-118 Pipeline Subject".to_string(),
        })
        .await
        .expect("create character");
    let character_internal_id = character.internal_id;
    // Stable, run-unique character ref used to scope the receipt + history query.
    let character_ref = format!("character://atelier/{}", Uuid::new_v4());

    // ======================================================================
    // STAGE 1 — media: materialize the SOURCE IMAGE as a native media asset.
    // ======================================================================
    let source_artifact = atelier_pg_support::write_native_media_artifact(b"mt118-source-image");
    let source_asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: source_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: source_artifact.byte_len,
            source_provenance: Some("mt-118 pipeline source asset".to_string()),
            artifact_ref: source_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize source image media asset");
    assert_eq!(
        source_asset.content_hash, source_artifact.content_hash,
        "STAGE 1: source image persisted with its content-addressed hash"
    );

    // ======================================================================
    // STAGE 2 — pose: ingest a RIG that references the source image asset.
    // ======================================================================
    let source_ref = format!("media://{}", source_asset.asset_id);
    let source_asset_version_ref = format!("media-version://{}/v1", source_asset.asset_id);
    let rig = store
        .ingest_pose_rig(&NewPoseRig {
            character_internal_id,
            source_asset_id: Some(source_asset.asset_id),
            source_ref: source_ref.clone(),
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            canvas: CanvasSize {
                width: 768,
                height: 1024,
            },
            detector_provider: "mediapipe.tasks-vision.pose".to_string(),
            detector_model: "BlazePose GHUM".to_string(),
            detector_model_version: "mediapipe-tasks-vision-0.10.20".to_string(),
            source_asset_version_ref: Some(source_asset_version_ref.clone()),
            source_asset_path_ref: Some(format!(
                "source://operator-inbox/{}/mt118-portrait.png",
                Uuid::new_v4()
            )),
            confidence_available: true,
            detector_status: DetectorStatus::Detected,
            error_reason: None,
            keypoints_json: valid_keypoints(),
            sidecar_ref: Some(source_artifact.artifact_ref.clone()),
        })
        .await
        .expect("ingest pose rig");
    // CROSS-MODULE LINK 1: the rig references the STAGE-1 source asset.
    assert_eq!(
        rig.source_asset_id,
        Some(source_asset.asset_id),
        "STAGE 2: rig links back to the source image media asset"
    );
    assert_eq!(rig.source_ref, source_ref);
    // Persistence proof: the rig is re-readable from Postgres.
    let fetched_rig = store
        .get_pose_rig(rig.rig_id)
        .await
        .expect("re-read rig from Postgres");
    assert_eq!(fetched_rig.rig_id, rig.rig_id);
    assert_eq!(
        fetched_rig.source_asset_id,
        Some(source_asset.asset_id),
        "STAGE 2: persisted rig still references the source asset after round-trip"
    );

    // ======================================================================
    // STAGE 3 — pose: register a conditioning SIDECAR that references the rig.
    // ======================================================================
    let conditioning_artifact =
        atelier_pg_support::write_native_media_artifact(b"mt118-openpose-conditioning");
    let sidecar = store
        .record_pose_sidecar(&NewPoseSidecar {
            rig_id: rig.rig_id,
            kind: PoseSidecarKind::ConditioningPng,
            artifact_ref: conditioning_artifact.artifact_ref.clone(),
            manifest_ref: artifact_manifest_ref(&conditioning_artifact.artifact_ref),
            content_hash: conditioning_artifact.content_hash.clone(),
            byte_len: conditioning_artifact.byte_len,
            mime: "image/png".to_string(),
            width: rig.canvas.width,
            height: rig.canvas.height,
            status: PoseSidecarStatus::Rendered,
            error_message: None,
        })
        .await
        .expect("register conditioning sidecar for the rig");
    // CROSS-MODULE LINK 2: the sidecar references the STAGE-2 rig, and the
    // store back-fills the rig's source identity onto the sidecar row.
    assert_eq!(
        sidecar.rig_id, rig.rig_id,
        "STAGE 3: sidecar references its parent rig"
    );
    assert_eq!(
        sidecar.source_ref, rig.source_ref,
        "STAGE 3: sidecar inherits the rig's source-image ref (rig -> source link preserved)"
    );
    assert_eq!(
        sidecar.source_asset_id,
        Some(source_asset.asset_id),
        "STAGE 3: sidecar carries the source asset id transitively from the rig"
    );
    // Persistence proof: the sidecar is listed under its rig.
    let listed_sidecars = store
        .list_pose_sidecars(rig.rig_id)
        .await
        .expect("list sidecars for the rig");
    assert!(
        listed_sidecars
            .iter()
            .any(|s| s.sidecar_id == sidecar.sidecar_id),
        "STAGE 3: registered sidecar is discoverable under its rig"
    );

    // ======================================================================
    // STAGE 4 — comfy: record the routed generation OUTPUT for this run. The
    // output's prompt_json_ref pins the STAGE-3 conditioning sidecar artifact,
    // linking the generated image back to the pose conditioning input.
    // ======================================================================
    let workflow_run_id = Uuid::new_v4();
    let output_artifact_ref = format!("artifact://atelier/comfy/{}", Uuid::new_v4());
    let output_content_hash = format!("sha256-{}", Uuid::new_v4());
    let record = store
        .record_intake_output(&NewIntakeOutput {
            workflow_run_id,
            node_execution_id: format!("nodeexec-{}", Uuid::new_v4()),
            registration_id: None,
            source_node_instance_id: "mt118-saveimage".to_string(),
            source_output_slot: "IMAGE".to_string(),
            media_kind: MediaKind::Image,
            mime: "image/png".to_string(),
            artifact_ref: output_artifact_ref.clone(),
            artifact_manifest_ref: format!("manifest://atelier/comfy/{}", Uuid::new_v4()),
            content_hash: output_content_hash.clone(),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            // Pin the pose conditioning sidecar artifact as the workflow input,
            // wiring comfy output -> pose sidecar -> rig -> source asset.
            prompt_json_ref: Some(conditioning_artifact.artifact_ref.clone()),
            graph_hash: Some(format!("graph-{}", Uuid::new_v4())),
            seed: Some(118_118),
            identity_metadata: None,
        })
        .await
        .expect("record routed comfy intake output");
    assert!(
        !record.deduplicated,
        "STAGE 4: first delivery is a fresh materialization"
    );
    let output = record.output;
    // CROSS-MODULE LINK 3: the comfy output's pinned workflow input references
    // the STAGE-3 conditioning sidecar artifact.
    assert_eq!(
        output.prompt_json_ref.as_deref(),
        Some(conditioning_artifact.artifact_ref.as_str()),
        "STAGE 4: comfy output references the pose conditioning sidecar artifact as its input"
    );
    assert_eq!(output.workflow_run_id, workflow_run_id);
    assert_eq!(output.seed, Some(118_118), "STAGE 4: seed pin round-trips");
    // Persistence proof: the output is listed for this run.
    let listed_outputs = store
        .list_intake_outputs(workflow_run_id)
        .await
        .expect("list intake outputs for the run");
    assert_eq!(
        listed_outputs.len(),
        1,
        "STAGE 4: exactly this run's one output row is present"
    );
    assert_eq!(listed_outputs[0].intake_output_id, output.intake_output_id);

    // ======================================================================
    // STAGE 5 — comfy: record the durable WORKFLOW RECEIPT. The receipt
    // captures the STAGE-4 output row and is tagged with this run's character.
    // ======================================================================
    let workflow_spec_ref = format!("workflow-spec://pose-comfy/{}", Uuid::new_v4());
    let receipt = store
        .record_comfy_workflow_receipt(&NewComfyWorkflowReceipt {
            system_id: "comfyui".to_string(),
            workflow_run_id,
            workflow_spec_ref: workflow_spec_ref.clone(),
            workflow_json_ref: format!("artifact://atelier/workflow-json/{}", Uuid::new_v4()),
            prompt_ref: format!("prompt://{}", Uuid::new_v4()),
            status: ComfyWorkflowStatus::Succeeded,
            error_ref: None,
            evidence: serde_json::json!({
                "character_ref": &character_ref,
                "executor": "fake-adapter",
                "mt": "MT-118",
            }),
        })
        .await
        .expect("record durable workflow receipt");
    // CROSS-MODULE LINK 4: the receipt is for the STAGE-4 run and captures its
    // output (comfy receipt -> comfy output).
    assert_eq!(
        receipt.workflow_run_id, workflow_run_id,
        "STAGE 5: receipt is bound to the same workflow run as the output"
    );
    assert_eq!(receipt.status, ComfyWorkflowStatus::Succeeded);
    assert_eq!(
        receipt.outputs.len(),
        1,
        "STAGE 5: receipt captures this run's single output row"
    );
    assert_eq!(
        receipt.outputs[0]["artifact_ref"], output.artifact_ref,
        "STAGE 5: receipt output artifact_ref matches the recorded output"
    );
    assert_eq!(
        receipt.outputs[0]["prompt_json_ref"], conditioning_artifact.artifact_ref,
        "STAGE 5: receipt preserves the pose conditioning input ref on the captured output"
    );
    assert_eq!(
        receipt.character_ref.as_deref(),
        Some(character_ref.as_str()),
        "STAGE 5: receipt is tagged with this run's character ref"
    );
    // Persistence proof: the receipt is re-readable by run id.
    let fetched_receipt = store
        .get_comfy_workflow_receipt(workflow_run_id)
        .await
        .expect("re-read workflow receipt")
        .expect("workflow receipt present");
    assert_eq!(fetched_receipt, receipt);

    // ======================================================================
    // STAGE 6 — comfy: the receipt is discoverable in HISTORY filtered to this
    // run's character + spec. Scoped query => no global counts.
    // ======================================================================
    let history = store
        .list_comfy_workflow_history(&ComfyWorkflowHistoryQuery {
            character_ref: Some(character_ref.clone()),
            workflow_spec_ref: Some(workflow_spec_ref.clone()),
            status: None,
            from_utc: None,
            to_utc: None,
        })
        .await
        .expect("list workflow history scoped to this run");
    // CROSS-MODULE LINK 5: history (filtered to our character/spec) surfaces
    // exactly the STAGE-5 receipt for the STAGE-4 run.
    assert_eq!(
        history.len(),
        1,
        "STAGE 6: history scoped to this run's character + spec returns only our receipt"
    );
    assert_eq!(
        history[0].workflow_run_id, workflow_run_id,
        "STAGE 6: the discoverable history receipt is the one produced by this pipeline run"
    );
    assert_eq!(
        history[0].receipt_id, receipt.receipt_id,
        "STAGE 6: history returns the same persisted receipt row"
    );
}
