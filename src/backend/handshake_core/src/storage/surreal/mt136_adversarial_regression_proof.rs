use std::{collections::HashSet, future::Future, sync::Arc, time::Duration};

use surrealdb::types::{Array as SurrealArray, RecordId, SurrealValue, Uuid as SurrealUuid};
use tokio::{sync::Barrier, task::JoinSet};
use uuid::Uuid;

use super::mt136_proof_harness::embedded_proof_backend;
use crate::{
    atelier::{
        intake::{
            intake_event_family, IntakeBatch, IntakeBatchMode, IntakeItem, IntakeLane,
            IntakeProfileMode, NewIntakeBatch, NewIntakeItem,
        },
        pose::{
            CalibrationState, CanvasSize, DetectorStatus, IdentityCropBox, IdentityCropLandmark,
            IdentityProfileKind, NewIdentityCropArtifact, NewIdentityProfile, NewPoseRig,
            BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT,
            IDENTITY_CROP_ARTIFACT_RECORDED,
        },
        AtelierError, AtelierStore, NewCharacter,
    },
    storage::{StorageError, StorageResult},
};

fn api_error(error: AtelierError) -> StorageError {
    StorageError::Database(format!("MT-136 Atelier regression proof failed: {error}"))
}

fn require(condition: bool, message: &'static str) -> StorageResult<()> {
    if condition {
        Ok(())
    } else {
        Err(StorageError::Database(message.to_owned()))
    }
}

const CONTENTION_CONTENDERS: usize = 8;
const CONTENTION_PHASE_TIMEOUT: Duration = Duration::from_secs(60);

async fn bounded_contention_phase<T, F>(label: &'static str, future: F) -> StorageResult<T>
where
    F: Future<Output = StorageResult<T>>,
{
    match tokio::time::timeout(CONTENTION_PHASE_TIMEOUT, future).await {
        Ok(result) => result,
        Err(_) => Err(StorageError::Database(format!(
            "MT-136 {label} contention phase exceeded {} seconds",
            CONTENTION_PHASE_TIMEOUT.as_secs()
        ))),
    }
}

fn combine_proof_and_cleanup(
    proof: StorageResult<()>,
    cleanup: StorageResult<()>,
) -> StorageResult<()> {
    match (proof, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(proof_error), Err(cleanup_error)) => Err(StorageError::Database(format!(
            "MT-136 proof failed: {proof_error}; cleanup also failed: {cleanup_error}"
        ))),
    }
}

#[derive(Clone, SurrealValue)]
struct AssertionSetupBindings {
    character_one: RecordId,
    character_one_id: SurrealUuid,
    character_two: RecordId,
    character_two_id: SurrealUuid,
    tag_one: RecordId,
    tag_one_id: SurrealUuid,
    tag_two: RecordId,
    tag_two_id: SurrealUuid,
    asset_one: RecordId,
    asset_one_id: SurrealUuid,
    asset_two: RecordId,
    asset_two_id: SurrealUuid,
    batch: RecordId,
    batch_id: SurrealUuid,
    item_one: RecordId,
    item_one_id: SurrealUuid,
    item_two: RecordId,
    item_two_id: SurrealUuid,
    workspace: RecordId,
    block_one: RecordId,
    block_one_id: String,
    block_two: RecordId,
    block_two_id: String,
}

#[derive(Clone, SurrealValue)]
struct AssertionWriteBindings {
    record_id: RecordId,
    first_ref: RecordId,
    second_ref: Option<RecordId>,
    workspace_ref: Option<RecordId>,
}

#[derive(SurrealValue)]
struct BatchKeyCountBindings {
    idempotency_key: String,
}

#[derive(SurrealValue)]
struct EventIdentityCountBindings {
    event_family: String,
    aggregate_type: String,
    aggregate_id: String,
}

#[derive(SurrealValue)]
struct IntakeItemRefBindings {
    item_ref: RecordId,
    lane_reason: String,
}

async fn require_one_domain_and_ledger_event(
    store: &AtelierStore,
    event_family: &str,
    aggregate_type: &str,
    aggregate_id: &str,
) -> StorageResult<()> {
    let domain_event_count = store
        .count_events_for_aggregate(event_family, aggregate_type, aggregate_id)
        .await
        .map_err(api_error)?;
    if domain_event_count != 1 {
        return Err(StorageError::Database(format!(
            "MT-136 {event_family} contention proof expected one atelier event, got {domain_event_count}"
        )));
    }
    let bindings = EventIdentityCountBindings {
        event_family: event_family.to_owned(),
        aggregate_type: aggregate_type.to_owned(),
        aggregate_id: aggregate_id.to_owned(),
    };
    let ledger_event_count: Option<i64> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_first(
                    "RETURN count(SELECT id FROM kernel_event_ledger \
                     WHERE aggregate_type = $aggregate_type \
                       AND aggregate_id = $aggregate_id \
                       AND payload.event_family = $event_family);",
                    bindings,
                )
                .await
            })
        })
        .await?;
    let ledger_event_count = ledger_event_count.unwrap_or_default();
    if ledger_event_count != 1 {
        return Err(StorageError::Database(format!(
            "MT-136 {event_family} contention proof expected one kernel ledger event, got {ledger_event_count}"
        )));
    }
    Ok(())
}

const ASSERTION_SETUP: &str = "RETURN {
    CREATE $character_one CONTENT { internal_id: $character_one_id, public_id: 'mt136-character-one', display_name: 'MT136 Character One' };
    CREATE $character_two CONTENT { internal_id: $character_two_id, public_id: 'mt136-character-two', display_name: 'MT136 Character Two' };
    CREATE $tag_one CONTENT { tag_id: $tag_one_id, text: 'mt136-tag-one' };
    CREATE $tag_two CONTENT { tag_id: $tag_two_id, text: 'mt136-tag-two' };
    CREATE $asset_one CONTENT { asset_id: $asset_one_id, content_hash: 'mt136-asset-one', mime: 'image/png', byte_len: 1, artifact_ref: 'artifact://.handshake/artifacts/mt136-asset-one/payload' };
    CREATE $asset_two CONTENT { asset_id: $asset_two_id, content_hash: 'mt136-asset-two', mime: 'image/png', byte_len: 1, artifact_ref: 'artifact://.handshake/artifacts/mt136-asset-two/payload' };
    CREATE $batch CONTENT { batch_id: $batch_id, idempotency_key: 'mt136-batch', source_label: 'mt136', source_ref: 'source://mt136', mode: 'manual', profile_mode: 'loose_profile' };
    CREATE $item_one CONTENT { item_id: $item_one_id, batch_id: $batch, source_path: 'mt136/item-one.png', file_name: 'item-one.png' };
    CREATE $item_two CONTENT { item_id: $item_two_id, batch_id: $batch, source_path: 'mt136/item-two.png', file_name: 'item-two.png' };
    CREATE $workspace CONTENT { name: 'MT136 assertion workspace' };
    CREATE $block_one CONTENT { block_id: $block_one_id, workspace_id: $workspace, content_type: 'note', title: 'MT136 block one' };
    CREATE $block_two CONTENT { block_id: $block_two_id, workspace_id: $workspace, content_type: 'note', title: 'MT136 block two' };
    RETURN true;
};";

const CREATE_CHARACTER_TAG: &str = "CREATE $record_id CONTENT {
    character_internal_id: $first_ref,
    tag_id: $second_ref,
    tag_type: 'manual'
};";

const CREATE_REVIEW_METADATA: &str = "CREATE $record_id CONTENT {
    asset_id: $first_ref,
    favorite: false,
    rating: 0,
    frontpage: false,
    carousel: false,
    review_status: 'unreviewed',
    updated_by: 'mt136-proof'
};";

const CREATE_PROVENANCE: &str = "CREATE $record_id CONTENT {
    asset_id: $first_ref,
    source_note_ref: 'source://mt136/provenance',
    updated_by: 'mt136-proof'
};";

const CREATE_INTAKE_PROJECTION: &str = "CREATE $record_id CONTENT {
    item_id: $first_ref,
    loom_block_id: $second_ref,
    workspace_id: $workspace_ref,
    linked_by: 'mt136-proof'
};";

async fn record_key_assertions_accept_valid_and_reject_mismatched_refs() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let proof = async {
        let ids = [
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
        ];
        let assets = [Uuid::now_v7(), Uuid::now_v7()];
        let batch_id = Uuid::now_v7();
        let items = [Uuid::now_v7(), Uuid::now_v7()];
        let workspace_id = format!("mt136-workspace-{}", Uuid::now_v7().simple());
        let block_ids = [
            format!("mt136-block-{}", Uuid::now_v7().simple()),
            format!("mt136-block-{}", Uuid::now_v7().simple()),
        ];
        let character_refs = [
            RecordId::new("atelier_character", SurrealUuid::from(ids[0])),
            RecordId::new("atelier_character", SurrealUuid::from(ids[1])),
        ];
        let tag_refs = [
            RecordId::new("atelier_tag", SurrealUuid::from(ids[2])),
            RecordId::new("atelier_tag", SurrealUuid::from(ids[3])),
        ];
        let asset_refs = [
            RecordId::new("atelier_media_asset", SurrealUuid::from(assets[0])),
            RecordId::new("atelier_media_asset", SurrealUuid::from(assets[1])),
        ];
        let batch_ref = RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id));
        let item_refs = [
            RecordId::new("atelier_intake_item", SurrealUuid::from(items[0])),
            RecordId::new("atelier_intake_item", SurrealUuid::from(items[1])),
        ];
        let workspace_ref = RecordId::new("workspaces", workspace_id.clone());
        let block_refs = [
            RecordId::new("loom_blocks", block_ids[0].clone()),
            RecordId::new("loom_blocks", block_ids[1].clone()),
        ];

        backend
            .storage
            .with_data_operation({
                let bindings = AssertionSetupBindings {
                    character_one: character_refs[0].clone(),
                    character_one_id: SurrealUuid::from(ids[0]),
                    character_two: character_refs[1].clone(),
                    character_two_id: SurrealUuid::from(ids[1]),
                    tag_one: tag_refs[0].clone(),
                    tag_one_id: SurrealUuid::from(ids[2]),
                    tag_two: tag_refs[1].clone(),
                    tag_two_id: SurrealUuid::from(ids[3]),
                    asset_one: asset_refs[0].clone(),
                    asset_one_id: SurrealUuid::from(assets[0]),
                    asset_two: asset_refs[1].clone(),
                    asset_two_id: SurrealUuid::from(assets[1]),
                    batch: batch_ref,
                    batch_id: SurrealUuid::from(batch_id),
                    item_one: item_refs[0].clone(),
                    item_one_id: SurrealUuid::from(items[0]),
                    item_two: item_refs[1].clone(),
                    item_two_id: SurrealUuid::from(items[1]),
                    workspace: workspace_ref.clone(),
                    block_one: block_refs[0].clone(),
                    block_one_id: block_ids[0].clone(),
                    block_two: block_refs[1].clone(),
                    block_two_id: block_ids[1].clone(),
                };
                move |ctx| {
                    Box::pin(
                        async move { ctx.query_first::<bool, _>(ASSERTION_SETUP, bindings).await },
                    )
                }
            })
            .await?
            .ok_or(StorageError::Database(
                "MT-136 schema assertion setup returned no result".to_owned(),
            ))?;

        let valid_writes = [
            (
                CREATE_CHARACTER_TAG,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_character_tag",
                        SurrealArray::from(vec![
                            SurrealUuid::from(ids[0]),
                            SurrealUuid::from(ids[2]),
                        ]),
                    ),
                    first_ref: character_refs[0].clone(),
                    second_ref: Some(tag_refs[0].clone()),
                    workspace_ref: None,
                },
            ),
            (
                CREATE_REVIEW_METADATA,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_media_review_metadata",
                        SurrealUuid::from(assets[0]),
                    ),
                    first_ref: asset_refs[0].clone(),
                    second_ref: None,
                    workspace_ref: None,
                },
            ),
            (
                CREATE_PROVENANCE,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_media_source_provenance_ref",
                        SurrealUuid::from(assets[0]),
                    ),
                    first_ref: asset_refs[0].clone(),
                    second_ref: None,
                    workspace_ref: None,
                },
            ),
            (
                CREATE_INTAKE_PROJECTION,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_intake_item_loom_projection",
                        SurrealUuid::from(items[0]),
                    ),
                    first_ref: item_refs[0].clone(),
                    second_ref: Some(block_refs[0].clone()),
                    workspace_ref: Some(workspace_ref.clone()),
                },
            ),
        ];
        let invalid_writes = [
            (
                CREATE_CHARACTER_TAG,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_character_tag",
                        SurrealArray::from(vec![
                            SurrealUuid::from(ids[0]),
                            SurrealUuid::from(ids[3]),
                        ]),
                    ),
                    first_ref: character_refs[1].clone(),
                    second_ref: Some(tag_refs[1].clone()),
                    workspace_ref: None,
                },
                "character_internal_id",
            ),
            (
                CREATE_REVIEW_METADATA,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_media_review_metadata",
                        SurrealUuid::from(assets[1]),
                    ),
                    first_ref: asset_refs[0].clone(),
                    second_ref: None,
                    workspace_ref: None,
                },
                "asset_id",
            ),
            (
                CREATE_PROVENANCE,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_media_source_provenance_ref",
                        SurrealUuid::from(assets[1]),
                    ),
                    first_ref: asset_refs[0].clone(),
                    second_ref: None,
                    workspace_ref: None,
                },
                "asset_id",
            ),
            (
                CREATE_INTAKE_PROJECTION,
                AssertionWriteBindings {
                    record_id: RecordId::new(
                        "atelier_intake_item_loom_projection",
                        SurrealUuid::from(items[1]),
                    ),
                    first_ref: item_refs[0].clone(),
                    second_ref: Some(block_refs[1].clone()),
                    workspace_ref: Some(workspace_ref),
                },
                "item_id",
            ),
        ];
        for (statement, bindings, asserted_field) in invalid_writes {
            let result = backend
                .storage
                .with_data_operation(move |ctx| {
                    Box::pin(async move {
                        ctx.query_values::<surrealdb::types::Value, _>(statement, bindings)
                            .await
                    })
                })
                .await;
            match result {
                Ok(_) => {
                    return Err(StorageError::Database(format!(
                        "record/key mismatch was accepted by the locked SurrealDB engine: {statement}"
                    )));
                }
                Err(error) if !error.to_string().contains(asserted_field) => {
                    return Err(StorageError::Database(format!(
                        "record/key mismatch failed for the wrong reason; expected assertion field {asserted_field}, got: {error}"
                    )));
                }
                Err(_) => {}
            }
        }
        for (statement, bindings) in valid_writes {
            backend
                .storage
                .with_data_operation(move |ctx| {
                    Box::pin(async move {
                        ctx.query_values::<surrealdb::types::Value, _>(statement, bindings)
                            .await
                    })
                })
                .await?;
        }
        Ok::<(), StorageError>(())
    }
    .await;
    let cleanup = backend.close_and_remove().await;
    combine_proof_and_cleanup(proof, cleanup)
}

fn crop_input(profile_id: Uuid, suffix: &str) -> NewIdentityCropArtifact {
    let artifact_ref = format!("artifact://.handshake/artifacts/mt136-pose-{suffix}/payload");
    NewIdentityCropArtifact {
        profile_id,
        source_ref: format!("source://mt136-pose-{suffix}"),
        crop_box: IdentityCropBox {
            x: 0,
            y: 0,
            width: 512,
            height: 512,
        },
        landmarks: vec![IdentityCropLandmark {
            name: "left_eye".to_owned(),
            x: 128.0,
            y: 128.0,
            confidence: Some(1.0),
        }],
        manifest_ref: artifact_ref.replace("/payload", "/artifact.json"),
        artifact_ref,
        content_hash: format!("sha256-mt136-pose-{suffix}"),
        byte_len: 1,
        mime: "image/png".to_owned(),
        width: 512,
        height: 512,
        created_by: "mt136-proof".to_owned(),
    }
}

async fn concurrent_intake_batch_phase(store: &AtelierStore) -> StorageResult<IntakeBatch> {
    let suffix = Uuid::now_v7().simple().to_string();
    let request = NewIntakeBatch {
        idempotency_key: format!("mt136-contention-batch-{suffix}"),
        source_label: "MT-136 steady-state intake contention".to_owned(),
        source_ref: Some(format!("source://mt136-contention-batch-{suffix}")),
        mode: IntakeBatchMode::Manual,
        profile_mode: IntakeProfileMode::LooseProfile,
        character_internal_id: None,
        target_character_id: None,
        target_sheet_version_id: None,
        target_collection_id: None,
        resume_cursor: None,
    };
    let barrier = Arc::new(Barrier::new(CONTENTION_CONTENDERS));
    let mut tasks = JoinSet::new();
    for _ in 0..CONTENTION_CONTENDERS {
        let store = store.clone();
        let request = request.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            store.open_intake_batch(&request).await
        });
    }
    let mut returned_ids = HashSet::new();
    let mut winner = None;
    let mut first_error = None;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(batch)) => {
                returned_ids.insert(batch.batch_id);
                winner = Some(batch);
            }
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(api_error(error));
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(StorageError::Database(format!(
                        "MT-136 intake-batch contender task failed: {error}"
                    )));
                }
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    require(
        returned_ids.len() == 1,
        "MT-136 intake-batch contenders returned more than one canonical id",
    )?;
    let winner = winner.ok_or_else(|| {
        StorageError::Database("MT-136 intake-batch phase returned no winner".to_owned())
    })?;
    let bindings = BatchKeyCountBindings {
        idempotency_key: request.idempotency_key,
    };
    let domain_count: Option<i64> = store
        .store()
        .with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_first(
                    "RETURN count(SELECT id FROM atelier_intake_batch \
                     WHERE idempotency_key = $idempotency_key);",
                    bindings,
                )
                .await
            })
        })
        .await?;
    require(
        domain_count.unwrap_or_default() == 1,
        "MT-136 intake-batch contention created more than one domain row",
    )?;
    require_one_domain_and_ledger_event(
        store,
        intake_event_family::INTAKE_BATCH_CREATED,
        "atelier_intake_batch",
        &winner.batch_id.to_string(),
    )
    .await?;
    Ok(winner)
}

async fn concurrent_intake_item_phase(
    store: &AtelierStore,
    batch: &IntakeBatch,
) -> StorageResult<IntakeItem> {
    let suffix = Uuid::now_v7().simple().to_string();
    let request = NewIntakeItem {
        source_path: format!("source://mt136-contention-item-{suffix}.png"),
        file_name: "mt136-contention.png".to_owned(),
        byte_len: 4096,
        content_hash: Some(format!("sha256-mt136-contention-item-{suffix}")),
    };
    let barrier = Arc::new(Barrier::new(CONTENTION_CONTENDERS));
    let mut tasks = JoinSet::new();
    for _ in 0..CONTENTION_CONTENDERS {
        let store = store.clone();
        let request = request.clone();
        let barrier = barrier.clone();
        let batch_id = batch.batch_id;
        tasks.spawn(async move {
            barrier.wait().await;
            store.add_intake_item(batch_id, &request).await
        });
    }
    let mut returned_ids = HashSet::new();
    let mut winner = None;
    let mut first_error = None;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(item)) => {
                returned_ids.insert(item.item_id);
                winner = Some(item);
            }
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(api_error(error));
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(StorageError::Database(format!(
                        "MT-136 intake-item contender task failed: {error}"
                    )));
                }
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    require(
        returned_ids.len() == 1,
        "MT-136 intake-item contenders returned more than one canonical id",
    )?;
    let winner = winner.ok_or_else(|| {
        StorageError::Database("MT-136 intake-item phase returned no winner".to_owned())
    })?;
    require(
        store
            .list_intake_items(batch.batch_id, None)
            .await
            .map_err(api_error)?
            .len()
            == 1,
        "MT-136 intake-item contention created more than one domain row",
    )?;
    require_one_domain_and_ledger_event(
        store,
        intake_event_family::INTAKE_ITEM_ADDED,
        "atelier_intake_item",
        &winner.item_id.to_string(),
    )
    .await?;
    Ok(winner)
}

async fn concurrent_rejection_audit_phase(
    store: &AtelierStore,
    item: &IntakeItem,
) -> StorageResult<()> {
    let mut rejected_item = item.clone();
    rejected_item.lane = IntakeLane::Rejected;
    let rejection_reason = "MT-136 concurrent rejection audit".to_owned();
    rejected_item.lane_reason = Some(rejection_reason.clone());
    let updated = store
        .store()
        .with_data_operation({
            let bindings = IntakeItemRefBindings {
                item_ref: RecordId::new("atelier_intake_item", SurrealUuid::from(item.item_id)),
                lane_reason: rejection_reason,
            };
            move |ctx| {
                Box::pin(async move {
                    ctx.execute_returning(
                        "UPDATE $item_ref SET lane = 'rejected', lane_reason = $lane_reason \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
                })
            }
        })
        .await?;
    require(
        updated == 1,
        "MT-136 rejection-audit setup did not update exactly one intake item",
    )?;
    let barrier = Arc::new(Barrier::new(CONTENTION_CONTENDERS));
    let mut tasks = JoinSet::new();
    for _ in 0..CONTENTION_CONTENDERS {
        let store = store.clone();
        let item = rejected_item.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            store.mt136_insert_rejection_audit_for_proof(&item).await
        });
    }
    let mut returned_ids = HashSet::new();
    let mut first_error = None;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(Some((audit, _)))) => {
                returned_ids.insert(audit.audit_id);
            }
            Ok(Ok(None)) => {
                if first_error.is_none() {
                    first_error = Some(StorageError::Database(
                        "MT-136 rejection-audit contender returned no audit".to_owned(),
                    ));
                }
            }
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(api_error(error));
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(StorageError::Database(format!(
                        "MT-136 rejection-audit contender task failed: {error}"
                    )));
                }
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    require(
        returned_ids.len() == 1,
        "MT-136 rejection-audit contenders returned more than one canonical id",
    )?;
    require(
        store
            .list_intake_rejection_audits(item.batch_id)
            .await
            .map_err(api_error)?
            .len()
            == 1,
        "MT-136 rejection-audit contention created more than one domain row",
    )?;
    require_one_domain_and_ledger_event(
        store,
        intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
        "atelier_intake_item",
        &item.item_id.to_string(),
    )
    .await
}

async fn concurrent_identity_crop_phase(
    store: &AtelierStore,
    character_internal_id: Uuid,
) -> StorageResult<()> {
    let suffix = Uuid::now_v7().simple().to_string();
    let profile = store
        .append_identity_profile(&NewIdentityProfile {
            character_internal_id,
            kind: IdentityProfileKind::Face,
            name: "MT136 contention profile".to_owned(),
            description: "Steady-state identity-crop contention proof".to_owned(),
            reference_asset_id: None,
            reference_ref: format!("reference://mt136-contention-{suffix}"),
            source_ref: None,
            crop_ref: None,
            artifact_ref: None,
            provenance: "mt136-contention-proof".to_owned(),
        })
        .await
        .map_err(api_error)?;
    let request = crop_input(profile.profile_id, &format!("contention-{suffix}"));
    let barrier = Arc::new(Barrier::new(CONTENTION_CONTENDERS));
    let mut tasks = JoinSet::new();
    for _ in 0..CONTENTION_CONTENDERS {
        let store = store.clone();
        let request = request.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            store.record_identity_crop_artifact(&request).await
        });
    }
    let mut returned_ids = HashSet::new();
    let mut winner_id = None;
    let mut first_error = None;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(crop)) => {
                returned_ids.insert(crop.crop_id);
                winner_id = Some(crop.crop_id);
            }
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(api_error(error));
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(StorageError::Database(format!(
                        "MT-136 identity-crop contender task failed: {error}"
                    )));
                }
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    require(
        returned_ids.len() == 1,
        "MT-136 identity-crop contenders returned more than one canonical id",
    )?;
    let winner_id = winner_id.ok_or_else(|| {
        StorageError::Database("MT-136 identity-crop phase returned no winner".to_owned())
    })?;
    require(
        store
            .list_identity_crop_artifacts(profile.profile_id)
            .await
            .map_err(api_error)?
            .len()
            == 1,
        "MT-136 identity-crop contention created more than one domain row",
    )?;
    require_one_domain_and_ledger_event(
        store,
        IDENTITY_CROP_ARTIFACT_RECORDED,
        "atelier_identity_crop_artifact",
        &winner_id.to_string(),
    )
    .await
}

async fn steady_state_concurrent_idempotency_proofs(
    store: &AtelierStore,
    character_internal_id: Uuid,
) -> StorageResult<()> {
    eprintln!("MT136_PROOF_STEP_START adversarial.concurrent_intake_batch");
    let batch =
        bounded_contention_phase("intake-batch", concurrent_intake_batch_phase(store)).await?;
    eprintln!("MT136_PROOF_STEP_PASS adversarial.concurrent_intake_batch");

    eprintln!("MT136_PROOF_STEP_START adversarial.concurrent_intake_item");
    let item = bounded_contention_phase("intake-item", concurrent_intake_item_phase(store, &batch))
        .await?;
    eprintln!("MT136_PROOF_STEP_PASS adversarial.concurrent_intake_item");

    eprintln!("MT136_PROOF_STEP_START adversarial.concurrent_rejection_audit");
    bounded_contention_phase(
        "rejection-audit",
        concurrent_rejection_audit_phase(store, &item),
    )
    .await?;
    eprintln!("MT136_PROOF_STEP_PASS adversarial.concurrent_rejection_audit");

    eprintln!("MT136_PROOF_STEP_START adversarial.concurrent_identity_crop");
    bounded_contention_phase(
        "identity-crop",
        concurrent_identity_crop_phase(store, character_internal_id),
    )
    .await?;
    eprintln!("MT136_PROOF_STEP_PASS adversarial.concurrent_identity_crop");
    Ok(())
}

async fn pose_concrete_record_getters_preserve_option_and_not_found_contracts() -> StorageResult<()>
{
    let backend = embedded_proof_backend().await?;
    let proof = async {
        let store = AtelierStore::new(backend.storage.clone());
        store.ensure_schema().await.map_err(api_error)?;

        let absent = Uuid::now_v7();
        require(
            matches!(
                store.get_pose_rig(absent).await,
                Err(AtelierError::NotFound(_))
            ),
            "missing pose rig did not return NotFound",
        )?;
        require(
            store
                .get_head_pose(absent)
                .await
                .map_err(api_error)?
                .is_none(),
            "missing head pose did not return None",
        )?;
        require(
            store
                .get_calibration(absent)
                .await
                .map_err(api_error)?
                .is_none(),
            "missing calibration did not return None",
        )?;
        require(
            store
                .get_identity_profile(absent)
                .await
                .map_err(api_error)?
                .is_none(),
            "missing identity profile did not return None",
        )?;
        require(
            store
                .get_identity_crop_artifact(absent)
                .await
                .map_err(api_error)?
                .is_none(),
            "missing identity crop did not return None",
        )?;
        require(
            matches!(
                store
                    .record_identity_crop_artifact(&crop_input(absent, "missing-parent"))
                    .await,
                Err(AtelierError::NotFound(_))
            ),
            "identity crop with a missing profile did not return NotFound",
        )?;

        let character = store
            .create_character(&NewCharacter {
                public_id: format!("mt136-pose-{}", Uuid::now_v7().simple()),
                display_name: "MT136 Pose".to_owned(),
            })
            .await
            .map_err(api_error)?;
        let rig = store
            .ingest_pose_rig(&NewPoseRig {
                character_internal_id: character.internal_id,
                source_asset_id: None,
                source_ref: format!("source://mt136-rig-{}", Uuid::now_v7().simple()),
                content_hash: format!("sha256-mt136-rig-{}", Uuid::now_v7().simple()),
                canvas: CanvasSize {
                    width: 1024,
                    height: 1024,
                },
                detector_provider: "mt136-proof".to_owned(),
                detector_model: "mt136-proof".to_owned(),
                detector_model_version: "1".to_owned(),
                source_asset_version_ref: None,
                source_asset_path_ref: None,
                confidence_available: true,
                detector_status: DetectorStatus::Detected,
                error_reason: None,
                keypoints_json: serde_json::json!({
                    "people": [{
                        "pose_keypoints_2d": vec![0.0_f64; BODY_KEYPOINT_COUNT * 3],
                        "face_keypoints_2d": vec![0.0_f64; FACE_KEYPOINT_COUNT * 3],
                        "hand_left_keypoints_2d": vec![0.0_f64; HAND_KEYPOINT_COUNT * 3],
                        "hand_right_keypoints_2d": vec![0.0_f64; HAND_KEYPOINT_COUNT * 3]
                    }]
                }),
                sidecar_ref: None,
            })
            .await
            .map_err(api_error)?;
        require(
            store.get_pose_rig(rig.rig_id).await.map_err(api_error)? == rig,
            "stored pose rig did not round-trip",
        )?;
        let head = store
            .record_head_pose(rig.rig_id, 0.0, 0.0, 0.0, [0.0, 0.0, 0.0, 1.0])
            .await
            .map_err(api_error)?;
        require(
            store.get_head_pose(rig.rig_id).await.map_err(api_error)? == Some(head),
            "stored head pose did not round-trip",
        )?;
        let calibration = store
            .set_calibration(
                rig.rig_id,
                CalibrationState::Unresolved,
                Some("mt136-proof-unresolved"),
            )
            .await
            .map_err(api_error)?;
        require(
            store.get_calibration(rig.rig_id).await.map_err(api_error)? == Some(calibration),
            "stored calibration did not round-trip",
        )?;

        let deleted_profile = store
            .append_identity_profile(&NewIdentityProfile {
                character_internal_id: character.internal_id,
                kind: IdentityProfileKind::Face,
                name: "MT136 deleted profile".to_owned(),
                description: String::new(),
                reference_asset_id: None,
                reference_ref: format!("reference://mt136-{}", Uuid::now_v7().simple()),
                source_ref: None,
                crop_ref: None,
                artifact_ref: None,
                provenance: "mt136-proof".to_owned(),
            })
            .await
            .map_err(api_error)?;
        require(
            store
                .get_identity_profile(deleted_profile.profile_id)
                .await
                .map_err(api_error)?
                .is_some(),
            "stored identity profile was not readable",
        )?;
        require(
            store
                .delete_identity_profile(deleted_profile.profile_id, "mt136-proof")
                .await
                .map_err(api_error)?,
            "identity profile delete did not report a mutation",
        )?;
        require(
            store
                .get_identity_profile(deleted_profile.profile_id)
                .await
                .map_err(api_error)?
                .is_none(),
            "deleted identity profile remained readable",
        )?;

        let active_profile = store
            .append_identity_profile(&NewIdentityProfile {
                character_internal_id: character.internal_id,
                kind: IdentityProfileKind::Face,
                name: "MT136 active profile".to_owned(),
                description: String::new(),
                reference_asset_id: None,
                reference_ref: format!("reference://mt136-{}", Uuid::now_v7().simple()),
                source_ref: None,
                crop_ref: None,
                artifact_ref: None,
                provenance: "mt136-proof".to_owned(),
            })
            .await
            .map_err(api_error)?;
        let crop = store
            .record_identity_crop_artifact(&crop_input(active_profile.profile_id, "present"))
            .await
            .map_err(api_error)?;
        require(
            store
                .get_identity_crop_artifact(crop.crop_id)
                .await
                .map_err(api_error)?
                == Some(crop),
            "stored identity crop did not round-trip",
        )?;

        steady_state_concurrent_idempotency_proofs(&store, character.internal_id).await?;

        drop(store);
        Ok::<(), StorageError>(())
    }
    .await;
    let cleanup = backend.close_and_remove().await;
    combine_proof_and_cleanup(proof, cleanup)
}

pub(super) async fn run_all() -> StorageResult<()> {
    eprintln!("MT136_PROOF_STEP_START adversarial.record_key_assertions");
    record_key_assertions_accept_valid_and_reject_mismatched_refs().await?;
    eprintln!("MT136_PROOF_STEP_PASS adversarial.record_key_assertions");
    eprintln!("MT136_PROOF_STEP_START adversarial.pose_concrete_record_getters");
    pose_concrete_record_getters_preserve_option_and_not_found_contracts().await?;
    eprintln!("MT136_PROOF_STEP_PASS adversarial.pose_concrete_record_getters");
    Ok(())
}
