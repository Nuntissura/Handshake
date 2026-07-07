//! Dexterity model-lane persistence.
//!
//! Dexterity is the operator-facing name for the internal kernel that launches,
//! switches, and records local, cloud, CLI, human, subagent, and validator
//! lanes. The stable wire/schema names remain `ModelLaneRun`, `ModelLane`, and
//! `ModelLaneMessage`.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
};

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{
    context_bundle::{canonical_json_bytes, ContextBundle},
    KernelActor, KernelEventType, NewKernelEvent,
};
use crate::model_runtime::ProviderKind;
use crate::storage::postgres::append_kernel_event_with_executor;
use crate::storage::StorageError;

use super::error::SwarmError;
use super::factory::LiveSession;
use super::ids::{ByokCloudProvider, SpawnRequest};

const SOURCE_COMPONENT: &str = "dexterity_model_lane";
const MAX_CONTEXT_BUNDLE_LOOM_REFS: usize = 64;
const MAX_CONTEXT_BUNDLE_MEMORY_PACK_REFS: usize = 16;

#[derive(Debug, Error)]
pub enum ModelLaneError {
    #[error("invalid model lane input: {0}")]
    InvalidInput(String),
    #[error("model lane idempotency conflict: {0}")]
    IdempotencyConflict(String),
    #[error("model lane ambiguous lookup: {0}")]
    AmbiguousLookup(String),
    #[error("model lane not found: {0}")]
    NotFound(String),
    #[error("model lane storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("model lane database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("model lane json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ModelLaneResult<T> = Result<T, ModelLaneError>;

#[derive(Debug, Clone)]
pub struct ModelLaneStore {
    pool: PgPool,
}

impl ModelLaneStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record_successful_launch(
        &self,
        request: &SpawnRequest,
        live: &LiveSession,
    ) -> ModelLaneResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        let records = build_successful_launch_records(request, live)?;
        self.record_prepared_launch(records).await
    }

    pub async fn record_prepared_launch(
        &self,
        records: (NewModelLaneRun, NewModelLane),
    ) -> ModelLaneResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        validate_run(&records.0)?;
        validate_lane(&records.1)?;
        validate_prepared_launch_pair(&records.0, &records.1)?;
        if is_cloud_lane(&records.1) {
            self.preflight_cloud_launch_records(&records.0, &records.1)
                .await?;
        }
        let mut tx = self.pool.begin().await?;
        let stored_run = record_run_tx(&mut tx, records.0).await?;
        let stored_lane = record_lane_tx(&mut tx, records.1).await?;
        tx.commit().await?;
        Ok((stored_run, stored_lane))
    }

    pub async fn record_normalized_launch(
        &self,
        launch: DexterityNormalizedLaunch,
    ) -> ModelLaneResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        self.record_prepared_launch(launch.to_records()?).await
    }

    pub async fn record_run(&self, input: NewModelLaneRun) -> ModelLaneResult<ModelLaneRunRecord> {
        validate_run(&input)?;
        let mut tx = self.pool.begin().await?;
        let stored = record_run_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(stored)
    }

    pub async fn record_lane(&self, input: NewModelLane) -> ModelLaneResult<ModelLaneRecord> {
        validate_lane(&input)?;
        if is_cloud_lane(&input) {
            self.preflight_cloud_lane_record(&input).await?;
        }
        let mut tx = self.pool.begin().await?;
        let stored = record_lane_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(stored)
    }

    pub async fn record_message(
        &self,
        input: NewModelLaneMessage,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        validate_message(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) =
            message_by_idempotency_key_tx(&mut tx, &input.idempotency_key).await?
        {
            if existing.payload_sha256 == input.payload_sha256 {
                // Spec 4.3.9.2.5: "Duplicate retries with the same
                // idempotency_key and payload hash MUST be idempotent." The
                // idempotency_key is the caller's dedup token; message_id and
                // message_span_id identify a single delivery attempt (the
                // coordinator may assign a fresh id/span per retry), so they must
                // not defeat idempotent replay. All payload-authority and routing
                // fields (to_lane, authority, locus, crdt, payload_ref, ...) are
                // still compared and MUST match or the retry fails closed.
                let mut retry_identity = input.clone();
                retry_identity.message_id = existing.message_id.clone();
                retry_identity.message_span_id = existing.message_span_id.clone();
                ensure_idempotent_input_matches(
                    "model_lane_message",
                    &input.idempotency_key,
                    &existing.inner,
                    &retry_identity,
                )?;
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to payload_sha256 {}",
                input.idempotency_key, existing.payload_sha256
            )));
        }
        let source_lane = lane_by_id_tx(&mut tx, &input.from_lane_id).await?;
        require_equal(
            "message.run_id",
            &input.run_id,
            "source_lane.run_id",
            &source_lane.run_id,
        )?;
        let source_run = run_by_id_tx(&mut tx, &input.run_id).await?;
        require_equal(
            "message.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "source_lane.event_ledger_stream_id",
            &source_lane.event_ledger_stream_id,
        )?;
        require_equal(
            "message.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "source_run.event_ledger_stream_id",
            &source_run.event_ledger_stream_id,
        )?;
        if let ModelLaneTarget::Lane(target_lane_id) = &input.to_lane {
            let target_lane = lane_by_id_for_run_tx(&mut tx, &input.run_id, target_lane_id).await?;
            require_equal(
                "message.event_ledger_stream_id",
                &input.event_ledger_stream_id,
                "target_lane.event_ledger_stream_id",
                &target_lane.event_ledger_stream_id,
            )?;
        }
        let cloud_source = is_cloud_lane_record(&source_lane);
        match input.authority {
            ModelLaneAuthority::Promoted => {
                ensure_promoted_message_has_decision_tx(&mut tx, &input).await?;
            }
            ModelLaneAuthority::OperatorDecision | ModelLaneAuthority::ValidatorVerdict
                if cloud_source =>
            {
                tx.rollback().await?;
                return Err(ModelLaneError::InvalidInput(
                    "Cloud ModelLaneMessage authority must remain advisory or promotion_candidate until an approved PromotionGate writes promoted authority"
                        .into(),
                ));
            }
            _ => {}
        }

        let payload = json!({
            "schema_id": "hsk.model_lane_message@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ModelResponseRecorded,
            "model_lane_message",
            &input.message_id,
            &input.idempotency_key,
            input.work_packet_id.as_deref().unwrap_or(&input.run_id),
            &input.event_ledger_stream_id,
            payload,
        )?;

        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneMessageRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            inner: input,
        };

        let inserted = sqlx::query(
            r#"
            INSERT INTO model_lane_messages (
                message_id, run_id, trace_id, message_span_id, from_lane_id,
                coordinator_session_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                payload_sha256, replay_order_key, authority,
                event_ledger_stream_id, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq,
                record_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#,
        )
        .bind(&record.message_id)
        .bind(&record.run_id)
        .bind(&record.trace_id)
        .bind(&record.message_span_id)
        .bind(&record.from_lane_id)
        .bind(&record.coordinator_session_id)
        .bind(record.work_packet_id.as_deref())
        .bind(record.micro_task_id.as_deref())
        .bind(record.task_board_id.as_deref())
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.payload_sha256)
        .bind(&record.replay_order_key)
        .bind(record.authority.as_str())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing = message_by_idempotency_key_tx(&mut tx, &record.idempotency_key).await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.payload_sha256 == record.payload_sha256 {
                ensure_idempotent_input_matches(
                    "model_lane_message",
                    &record.idempotency_key,
                    &existing.inner,
                    &record.inner,
                )?;
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to payload_sha256 {}",
                    record.idempotency_key, existing.payload_sha256
                )));
            }
        };

        tx.commit().await?;
        Ok(stored)
    }

    pub async fn record_cloud_projection_plan(
        &self,
        input: NewModelLaneCloudProjectionPlan,
    ) -> ModelLaneResult<ModelLaneCloudProjectionPlanRecord> {
        validate_cloud_projection_plan(&input)?;
        let projection_plan_hash = cloud_projection_plan_hash(&input)?;
        let prepared = ModelLaneCloudProjectionPlanRecord {
            inner: input,
            projection_plan_hash,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &prepared.idempotency_key).await?;

        if let Some(existing) =
            cloud_projection_plan_by_idempotency_key_tx(&mut tx, &prepared.idempotency_key).await?
        {
            if existing.projection_plan_hash == prepared.projection_plan_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to projection_plan_hash {}",
                prepared.idempotency_key, existing.projection_plan_hash
            )));
        }

        let event = model_lane_event(
            KernelEventType::ArtifactStored,
            "model_lane_cloud_projection_plan",
            &prepared.projection_plan_id,
            &prepared.idempotency_key,
            &prepared.work_packet_id,
            &prepared.event_ledger_stream_id,
            cloud_projection_plan_event_payload(&prepared),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneCloudProjectionPlanRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            cloud_projection_plan_event_payload(&record),
        )
        .await?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO model_lane_cloud_projection_plans (
                projection_plan_id, run_id, trace_id, lane_id,
                model_session_id, provider_kind, requested_model_id,
                scope_hash, source_artifact_refs, payload_artifact_ref,
                payload_sha256, redaction_policy_ref, redaction_summary,
                retention_policy, export_posture, provider_profile_ref,
                fan_out_targets, consent_scope, status,
                event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                created_at_utc, user_manual_behavior_ref, diagnostic_payload,
                projection_plan_hash, event_ledger_event_id, event_ledger_seq,
                event_stream_version, transaction_seq, record_json
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                $17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                $31,$32,$33,$34
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#,
        )
        .bind(&record.projection_plan_id)
        .bind(&record.run_id)
        .bind(&record.trace_id)
        .bind(&record.lane_id)
        .bind(&record.model_session_id)
        .bind(&record.provider_kind)
        .bind(&record.requested_model_id)
        .bind(&record.scope_hash)
        .bind(serde_json::to_value(&record.source_artifact_refs)?)
        .bind(&record.payload_artifact_ref)
        .bind(&record.payload_sha256)
        .bind(&record.redaction_policy_ref)
        .bind(&record.redaction_summary)
        .bind(record.retention_policy.as_str())
        .bind(record.export_posture.as_str())
        .bind(&record.provider_profile_ref)
        .bind(serde_json::to_value(&record.fan_out_targets)?)
        .bind(record.consent_scope.as_str())
        .bind(record.status.as_str())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.created_at_utc)
        .bind(&record.user_manual_behavior_ref)
        .bind(&record.diagnostic_payload)
        .bind(&record.projection_plan_hash)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing =
                cloud_projection_plan_by_idempotency_key_tx(&mut tx, &record.idempotency_key)
                    .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after cloud projection insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.projection_plan_hash == record.projection_plan_hash {
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to projection_plan_hash {}",
                    record.idempotency_key, existing.projection_plan_hash
                )));
            }
        };

        tx.commit().await?;
        Ok(stored)
    }

    pub async fn record_cloud_consent_receipt(
        &self,
        input: NewModelLaneCloudConsentReceipt,
    ) -> ModelLaneResult<ModelLaneCloudConsentReceiptRecord> {
        validate_cloud_consent_receipt(&input)?;
        let consent_receipt_hash = cloud_consent_receipt_hash(&input)?;
        let prepared = ModelLaneCloudConsentReceiptRecord {
            inner: input,
            consent_receipt_hash,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &prepared.idempotency_key).await?;

        if let Some(existing) =
            cloud_consent_receipt_by_idempotency_key_tx(&mut tx, &prepared.idempotency_key).await?
        {
            if existing.consent_receipt_hash == prepared.consent_receipt_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to consent_receipt_hash {}",
                prepared.idempotency_key, existing.consent_receipt_hash
            )));
        }

        let event = model_lane_event(
            KernelEventType::ArtifactStored,
            "model_lane_cloud_consent_receipt",
            &prepared.consent_receipt_id,
            &prepared.idempotency_key,
            &prepared.work_packet_id,
            &prepared.event_ledger_stream_id,
            cloud_consent_receipt_event_payload(&prepared),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneCloudConsentReceiptRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            cloud_consent_receipt_event_payload(&record),
        )
        .await?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO model_lane_cloud_consent_receipts (
                consent_receipt_id, projection_plan_id, projection_plan_hash,
                run_id, trace_id, lane_id, model_session_id, provider_kind,
                requested_model_id, scope_hash, consent_scope, retention_policy,
                export_posture, fan_out_targets, approved, approved_by_ref,
                approved_at_utc, valid_from_utc, valid_until_utc,
                revoked_at_utc, revocation_ref, status, event_ledger_stream_id,
                work_packet_id, micro_task_id, task_board_id, owner_session,
                idempotency_key, created_at_utc, user_manual_behavior_ref,
                diagnostic_payload, consent_receipt_hash, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq,
                record_json
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                $17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                $31,$32,$33,$34,$35,$36,$37
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#,
        )
        .bind(&record.consent_receipt_id)
        .bind(&record.projection_plan_id)
        .bind(&record.projection_plan_hash)
        .bind(&record.run_id)
        .bind(&record.trace_id)
        .bind(&record.lane_id)
        .bind(&record.model_session_id)
        .bind(&record.provider_kind)
        .bind(&record.requested_model_id)
        .bind(&record.scope_hash)
        .bind(record.consent_scope.as_str())
        .bind(record.retention_policy.as_str())
        .bind(record.export_posture.as_str())
        .bind(serde_json::to_value(&record.fan_out_targets)?)
        .bind(record.approved)
        .bind(&record.approved_by_ref)
        .bind(&record.approved_at_utc)
        .bind(&record.valid_from_utc)
        .bind(&record.valid_until_utc)
        .bind(record.revoked_at_utc.as_deref())
        .bind(record.revocation_ref.as_deref())
        .bind(record.status.as_str())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.created_at_utc)
        .bind(&record.user_manual_behavior_ref)
        .bind(&record.diagnostic_payload)
        .bind(&record.consent_receipt_hash)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing =
                cloud_consent_receipt_by_idempotency_key_tx(&mut tx, &record.idempotency_key)
                    .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after cloud consent insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.consent_receipt_hash == record.consent_receipt_hash {
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to consent_receipt_hash {}",
                    record.idempotency_key, existing.consent_receipt_hash
                )));
            }
        };

        tx.commit().await?;
        Ok(stored)
    }

    pub async fn replay_cloud_consent_authority(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneCloudConsentAuthorityReplay> {
        require_token("run_id", run_id)?;
        let projection_plans = sqlx::query(
            r#"
            SELECT record_json
            FROM model_lane_cloud_projection_plans
            WHERE run_id = $1
            ORDER BY event_ledger_seq ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneCloudProjectionPlanRecord>>>()?;

        let consent_receipts = sqlx::query(
            r#"
            SELECT record_json
            FROM model_lane_cloud_consent_receipts
            WHERE run_id = $1
            ORDER BY event_ledger_seq ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneCloudConsentReceiptRecord>>>()?;

        Ok(ModelLaneCloudConsentAuthorityReplay {
            projection_plans,
            consent_receipts,
        })
    }

    pub async fn preflight_cloud_spawn_request(
        &self,
        request: &SpawnRequest,
    ) -> ModelLaneResult<()> {
        if request.provider != Some(ProviderKind::ByokCloud) {
            return Ok(());
        }
        let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "CX-MM-007 cloud launch requires Dexterity launch contract before provider call"
                    .into(),
            )
        })?;
        let provider_kind = match request.byok_cloud_provider {
            Some(ByokCloudProvider::OpenAi) => "openai",
            Some(ByokCloudProvider::Anthropic) => "anthropic",
            None => {
                let mut check = CloudLaunchAuthorityCheck::from_contract(
                    contract,
                    "unknown",
                    "",
                    runtime_session_id(request),
                )?;
                check.work_packet_id = request
                    .wp_id
                    .clone()
                    .unwrap_or_else(|| contract.run_id.clone());
                check.micro_task_id = request.mt_id.clone();
                check.owner_session = request.owner_role.clone();
                return self
                    .deny_cloud_launch(check, "missing_byok_cloud_provider")
                    .await;
            }
        };
        let requested_model_id = dexterity_candidate_model_ids(request)
            .into_iter()
            .next()
            .unwrap_or_else(|| request.instance_id.model_id.to_string());
        let mut check = CloudLaunchAuthorityCheck::from_contract(
            contract,
            provider_kind,
            &requested_model_id,
            runtime_session_id(request),
        )?;
        check.work_packet_id = request
            .wp_id
            .clone()
            .unwrap_or_else(|| contract.run_id.clone());
        check.micro_task_id = request.mt_id.clone();
        check.owner_session = request.owner_role.clone();
        self.preflight_cloud_launch(check).await
    }

    pub async fn revoke_cloud_consent_receipt(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        require_token("consent_receipt_id", consent_receipt_id)?;
        require_token("revoked_by_ref", revoked_by_ref)?;
        require_token("reason", reason)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(
            &mut tx,
            &format!("model-lane-cloud-consent-revoke:{consent_receipt_id}"),
        )
        .await?;
        let existing = cloud_consent_receipt_by_id_tx(&mut tx, consent_receipt_id)
            .await?
            .ok_or_else(|| {
                ModelLaneError::NotFound(format!("consent_receipt_id {consent_receipt_id}"))
            })?;
        if existing.status == ModelLaneCloudConsentReceiptStatus::Revoked {
            tx.commit().await?;
            return Ok(Vec::new());
        }
        let mut receipt_inner = existing.inner.clone();
        receipt_inner.status = ModelLaneCloudConsentReceiptStatus::Revoked;
        receipt_inner.revoked_at_utc = Some(Utc::now().to_rfc3339());
        receipt_inner.revocation_ref = Some(revoked_by_ref.to_string());
        receipt_inner.diagnostic_payload = merge_diagnostic_payload(
            receipt_inner.diagnostic_payload,
            json!({
                "consent_status": "CX-MM-007",
                "revocation_reason": reason,
                "revoked_by_ref": revoked_by_ref,
                "provider_call_attempted": false
            }),
        );
        let consent_receipt_hash = cloud_consent_receipt_hash(&receipt_inner)?;
        let revocation_event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_cloud_consent_receipt",
            consent_receipt_id,
            &format!("model-lane-cloud-consent-revoked:{consent_receipt_id}"),
            &receipt_inner.work_packet_id,
            &receipt_inner.event_ledger_stream_id,
            json!({
                "schema_id": "hsk.model_lane_cloud_consent_receipt@1",
                "dexterity_kernel": "Dexterity",
                "reason_code": "CX-MM-007",
                "consent_status": "CX-MM-007",
                "revoked_by_ref": revoked_by_ref,
                "reason": reason,
                "record": &receipt_inner,
            }),
        )?;
        let stored_revocation_event =
            append_kernel_event_with_executor(&mut *tx, revocation_event).await?;
        let revoked_receipt = ModelLaneCloudConsentReceiptRecord {
            inner: receipt_inner,
            consent_receipt_hash,
            event_ledger_event_id: stored_revocation_event.event_id.clone(),
            event_ledger_seq: stored_revocation_event.event_sequence,
            event_stream_version: stored_revocation_event.event_sequence,
            transaction_seq: stored_revocation_event.event_sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &revoked_receipt.event_ledger_event_id,
            cloud_consent_receipt_event_payload(&revoked_receipt),
        )
        .await?;
        sqlx::query(
            r#"
            UPDATE model_lane_cloud_consent_receipts
            SET approved = $2,
                revoked_at_utc = $3,
                revocation_ref = $4,
                status = $5,
                diagnostic_payload = $6,
                consent_receipt_hash = $7,
                event_ledger_event_id = $8,
                event_ledger_seq = $9,
                event_stream_version = $10,
                transaction_seq = $11,
                record_json = $12,
                updated_at = NOW()
            WHERE consent_receipt_id = $1
            "#,
        )
        .bind(consent_receipt_id)
        .bind(revoked_receipt.approved)
        .bind(revoked_receipt.revoked_at_utc.as_deref())
        .bind(revoked_receipt.revocation_ref.as_deref())
        .bind(revoked_receipt.status.as_str())
        .bind(&revoked_receipt.diagnostic_payload)
        .bind(&revoked_receipt.consent_receipt_hash)
        .bind(&revoked_receipt.event_ledger_event_id)
        .bind(revoked_receipt.event_ledger_seq)
        .bind(revoked_receipt.event_stream_version)
        .bind(revoked_receipt.transaction_seq)
        .bind(serde_json::to_value(&revoked_receipt)?)
        .execute(&mut *tx)
        .await?;

        let lanes = sqlx::query(
            r#"
            SELECT record_json
            FROM model_lanes
            WHERE record_json->>'consent_receipt_ref' = $1
              AND status NOT IN ('completed', 'failed', 'cancelled')
            ORDER BY event_ledger_seq ASC
            FOR UPDATE
            "#,
        )
        .bind(consent_receipt_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneRecord>>>()?;

        let mut cancelled = Vec::with_capacity(lanes.len());
        for existing_lane in lanes {
            let mut lane = existing_lane.inner.clone();
            lane.status = ModelLaneStatus::Cancelled;
            lane.recovery_state = ModelLaneRecoveryState::Terminal;
            lane.failstate_code = Some("CX-MM-007".into());
            lane.reason_ref = Some(format!(
                "cloud-consent-revoked://dexterity/{}/{}",
                lane.run_id, lane.lane_id
            ));
            lane.recovery_hint_ref =
                Some("usermanual://model-lane-cloud-projection-consent#recovery".into());
            lane.last_runtime_status_ref = Some(format!(
                "runtime-status://dexterity/{}/cloud-consent-revoked",
                lane.lane_id
            ));
            validate_lane(&lane)?;
            let terminal_event = model_lane_event(
                KernelEventType::SessionCancelled,
                "model_lane_terminal",
                &lane.lane_id,
                &format!(
                    "model-lane-cloud-consent-revoked:{consent_receipt_id}:{}",
                    lane.lane_id
                ),
                lane.work_packet_id.as_deref().unwrap_or(&lane.run_id),
                &lane.event_ledger_stream_id,
                json!({
                    "schema_id": "hsk.model_lane_terminal@1",
                    "dexterity_kernel": "Dexterity",
                    "lane_id": &lane.lane_id,
                    "run_id": &lane.run_id,
                    "status": "cancelled",
                    "reason": reason,
                    "reason_code": "CX-MM-007",
                    "consent_status": "CX-MM-007",
                    "consent_receipt_id": consent_receipt_id,
                    "projection_plan_id": &revoked_receipt.projection_plan_id,
                    "provider_call_cancelled": true,
                    "flight_recorder": "EventLedger",
                    "previous_event_ledger_event_id": &existing_lane.event_ledger_event_id,
                    "previous_event_ledger_seq": existing_lane.event_ledger_seq,
                }),
            )?;
            let stored_terminal_event =
                append_kernel_event_with_executor(&mut *tx, terminal_event).await?;
            lane.last_recovery_event_ref = Some(stored_terminal_event.event_id.clone());
            let record = ModelLaneRecord {
                event_ledger_event_id: stored_terminal_event.event_id.clone(),
                event_ledger_seq: stored_terminal_event.event_sequence,
                inner: lane,
            };
            // Same EventLedger-authority invariant as record_lane_terminal_status:
            // the row is repointed to this terminal event, so its payload must
            // carry the full updated lane `record` for replay/diagnostics
            // authority validation.
            stamp_kernel_event_payload_tx(
                &mut tx,
                &record.event_ledger_event_id,
                json!({
                    "schema_id": "hsk.model_lane_terminal@1",
                    "dexterity_kernel": "Dexterity",
                    "lane_id": &record.lane_id,
                    "run_id": &record.run_id,
                    "status": "cancelled",
                    "reason": reason,
                    "reason_code": "CX-MM-007",
                    "consent_status": "CX-MM-007",
                    "consent_receipt_id": consent_receipt_id,
                    "projection_plan_id": &revoked_receipt.projection_plan_id,
                    "provider_call_cancelled": true,
                    "flight_recorder": "EventLedger",
                    "previous_event_ledger_event_id": &existing_lane.event_ledger_event_id,
                    "previous_event_ledger_seq": existing_lane.event_ledger_seq,
                    "record": serde_json::to_value(&record.inner)?,
                }),
            )
            .await?;
            sqlx::query(
                r#"
                UPDATE model_lanes
                SET status = $2,
                    event_ledger_event_id = $3,
                    event_ledger_seq = $4,
                    record_json = $5,
                    updated_at = NOW()
                WHERE lane_id = $1
                "#,
            )
            .bind(&record.lane_id)
            .bind(record.status.as_str())
            .bind(&record.event_ledger_event_id)
            .bind(record.event_ledger_seq)
            .bind(serde_json::to_value(&record)?)
            .execute(&mut *tx)
            .await?;
            cancelled.push(record);
        }

        tx.commit().await?;
        Ok(cancelled)
    }

    pub async fn record_promotion_decision(
        &self,
        input: NewModelLanePromotionDecision,
    ) -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
        validate_promotion_decision(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        let prepared = prepare_promotion_decision_tx(&mut tx, input).await?;

        if let Some(existing) =
            promotion_decision_by_idempotency_key_tx(&mut tx, &prepared.idempotency_key).await?
        {
            if existing.canonical_decision_hash == prepared.canonical_decision_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to canonical_decision_hash {}",
                prepared.idempotency_key, existing.canonical_decision_hash
            )));
        }

        if let Some(existing) = promotion_decision_by_id_tx(&mut tx, &prepared.decision_id).await? {
            if existing.canonical_decision_hash == prepared.canonical_decision_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "decision_id {} already belongs to idempotency_key {}",
                prepared.decision_id, existing.idempotency_key
            )));
        }

        let event_type = match prepared.outcome {
            ModelLanePromotionOutcome::Approved => KernelEventType::PromotionAccepted,
            ModelLanePromotionOutcome::Denied => KernelEventType::PromotionRejected,
        };
        let payload = json!({
            "schema_id": "hsk.model_lane_promotion_decision@1",
            "dexterity_kernel": "Dexterity",
            "record": &prepared,
        });
        let event = model_lane_event(
            event_type,
            "model_lane_promotion_decision",
            &prepared.decision_id,
            &prepared.idempotency_key,
            prepared
                .work_packet_id
                .as_deref()
                .unwrap_or(&prepared.run_id),
            &prepared.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLanePromotionDecisionRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };

        let inserted = sqlx::query(
            r#"
            INSERT INTO model_lane_promotion_decisions (
                decision_id, run_id, trace_id, decision_span_id,
                coordinator_session_id, routing_policy, outcome, final_state,
                denial_reason, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                canonical_decision_hash, expected_event_ledger_aggregate_type,
                expected_event_ledger_aggregate_id, expected_event_ledger_version,
                current_event_ledger_version, schema_id, current_schema_id,
                base_snapshot_ref, current_base_snapshot_ref, state_vector,
                current_state_vector, promotion_gate_ref, promotion_receipt_ref,
                event_ledger_stream_id, event_ledger_event_id, event_ledger_seq,
                event_stream_version, transaction_seq, record_json
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                $17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                $31,$32,$33
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#,
        )
        .bind(&record.decision_id)
        .bind(&record.run_id)
        .bind(&record.trace_id)
        .bind(&record.decision_span_id)
        .bind(&record.coordinator_session_id)
        .bind(record.routing_policy.as_str())
        .bind(record.outcome.as_str())
        .bind(record.final_state.as_str())
        .bind(record.denial_reason.as_ref().map(|reason| reason.as_str()))
        .bind(record.work_packet_id.as_deref())
        .bind(record.micro_task_id.as_deref())
        .bind(record.task_board_id.as_deref())
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.canonical_decision_hash)
        .bind(&record.expected_event_ledger_aggregate_type)
        .bind(&record.expected_event_ledger_aggregate_id)
        .bind(record.expected_event_ledger_version)
        .bind(record.current_event_ledger_version)
        .bind(&record.schema_id)
        .bind(record.current_schema_id.as_deref())
        .bind(&record.base_snapshot_ref)
        .bind(&record.current_base_snapshot_ref)
        .bind(&record.state_vector)
        .bind(&record.current_state_vector)
        .bind(&record.promotion_gate_ref)
        .bind(record.promotion_receipt_ref.as_deref())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing =
                promotion_decision_by_idempotency_key_tx(&mut tx, &record.idempotency_key).await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after promotion decision insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.canonical_decision_hash == record.canonical_decision_hash {
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to canonical_decision_hash {}",
                    record.idempotency_key, existing.canonical_decision_hash
                )));
            }
        };

        tx.commit().await?;
        Ok(stored)
    }

    pub async fn replay_promotion_decisions(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<Vec<ModelLanePromotionDecisionRecord>> {
        require_token("run_id", run_id)?;
        sqlx::query(
            r#"
            SELECT record_json
            FROM model_lane_promotion_decisions
            WHERE run_id = $1
            ORDER BY event_ledger_seq ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect()
    }

    pub async fn record_context_bundle_artifact_binding(
        &self,
        input: NewModelLaneContextBundleArtifactBinding,
    ) -> ModelLaneResult<ModelLaneContextBundleArtifactBindingRecord> {
        validate_context_bundle_artifact_binding(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        run_by_id_tx(&mut tx, &input.run_id).await?;
        let binding_hash = context_bundle_artifact_binding_hash(&input)?;
        let prepared = ModelLaneContextBundleArtifactBindingRecord {
            inner: input,
            artifact_binding_hash: binding_hash,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };

        if let Some(existing) = context_bundle_artifact_binding_by_idempotency_key_tx(
            &mut tx,
            &prepared.idempotency_key,
        )
        .await?
        {
            if existing.artifact_binding_hash == prepared.artifact_binding_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to artifact_binding_hash {}",
                prepared.idempotency_key, existing.artifact_binding_hash
            )));
        }

        let event = model_lane_event(
            KernelEventType::ArtifactStored,
            "model_lane_context_bundle_artifact",
            &prepared.artifact_binding_id,
            &prepared.idempotency_key,
            &prepared.work_packet_id,
            &prepared.event_ledger_stream_id,
            context_bundle_artifact_binding_event_payload(&prepared),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneContextBundleArtifactBindingRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            context_bundle_artifact_binding_event_payload(&record),
        )
        .await?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO model_lane_context_bundle_artifacts (
                artifact_binding_id, run_id, trace_id, artifact_ref,
                artifact_sha256, content_hash, artifact_kind,
                artifact_manifest_ref, artifact_payload_ref, payload_json,
                event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                artifact_binding_hash, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq,
                record_json
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,
                $13,$14,$15,$16,$17,$18,$19,$20,$21,$22
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#,
        )
        .bind(&record.artifact_binding_id)
        .bind(&record.run_id)
        .bind(&record.trace_id)
        .bind(&record.artifact_ref)
        .bind(&record.artifact_sha256)
        .bind(&record.content_hash)
        .bind(&record.artifact_kind)
        .bind(&record.artifact_manifest_ref)
        .bind(&record.artifact_payload_ref)
        .bind(&record.payload_json)
        .bind(&record.event_ledger_stream_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.artifact_binding_hash)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing = context_bundle_artifact_binding_by_idempotency_key_tx(
                &mut tx,
                &record.idempotency_key,
            )
            .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after artifact binding insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.artifact_binding_hash == record.artifact_binding_hash {
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to artifact_binding_hash {}",
                    record.idempotency_key, existing.artifact_binding_hash
                )));
            }
        };

        tx.commit().await?;
        Ok(stored)
    }

    pub async fn record_context_bundle_handoff(
        &self,
        input: NewModelLaneContextBundleHandoff,
    ) -> ModelLaneResult<ModelLaneContextBundleHandoffRecord> {
        validate_context_bundle_handoff(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        let prepared = prepare_context_bundle_handoff_tx(&mut tx, input).await?;

        if let Some(existing) =
            context_bundle_handoff_by_idempotency_key_tx(&mut tx, &prepared.idempotency_key).await?
        {
            if existing.context_bundle_hash == prepared.context_bundle_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to context_bundle_hash {}",
                prepared.idempotency_key, existing.context_bundle_hash
            )));
        }

        let event = model_lane_event(
            KernelEventType::ContextBundleRecorded,
            "model_lane_context_bundle_handoff",
            &prepared.handoff_id,
            &prepared.idempotency_key,
            &prepared.work_packet_id,
            &prepared.event_ledger_stream_id,
            context_bundle_handoff_event_payload(&prepared),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneContextBundleHandoffRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            context_bundle_handoff_event_payload(&record),
        )
        .await?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO model_lane_context_bundle_handoffs (
                handoff_id, context_bundle_id, run_id, trace_id, handoff_span_id,
                downstream_lane_id, source_lane_id, source_message_id,
                artifact_ref, artifact_sha256, content_hash, source_kind, authority_state,
                selection_state, reason_code, decision_ref, reviewer_ref,
                work_packet_id, micro_task_id, task_board_id, owner_session,
                idempotency_key, context_bundle_hash, event_ledger_stream_id,
                event_ledger_event_id, event_ledger_seq, event_stream_version,
                transaction_seq, record_json
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                $17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#,
        )
        .bind(&record.handoff_id)
        .bind(&record.context_bundle_id)
        .bind(&record.run_id)
        .bind(&record.trace_id)
        .bind(&record.handoff_span_id)
        .bind(&record.downstream_lane_id)
        .bind(&record.source_lane_id)
        .bind(&record.source_message_id)
        .bind(&record.artifact_ref)
        .bind(&record.artifact_sha256)
        .bind(&record.content_hash)
        .bind(record.source_kind.as_str())
        .bind(record.authority_state.as_str())
        .bind(record.selection_state.as_str())
        .bind(&record.reason_code)
        .bind(record.decision_ref.as_deref())
        .bind(record.reviewer_ref.as_deref())
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.context_bundle_hash)
        .bind(&record.event_ledger_stream_id)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing =
                context_bundle_handoff_by_idempotency_key_tx(&mut tx, &record.idempotency_key)
                    .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after context bundle handoff insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.context_bundle_hash == record.context_bundle_hash {
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to context_bundle_hash {}",
                    record.idempotency_key, existing.context_bundle_hash
                )));
            }
        };

        tx.commit().await?;
        Ok(stored)
    }

    pub async fn consume_context_bundle_for_downstream(
        &self,
        run_id: &str,
        context_bundle_id: &str,
        downstream_lane_id: &str,
    ) -> ModelLaneResult<ModelLaneDownstreamContextBundle> {
        require_token("run_id", run_id)?;
        require_token("context_bundle_id", context_bundle_id)?;
        require_token("downstream_lane_id", downstream_lane_id)?;
        let mut tx = self.pool.begin().await?;
        let lane = lane_by_id_tx(&mut tx, downstream_lane_id)
            .await
            .map_err(|err| match err {
                ModelLaneError::NotFound(message) => ModelLaneError::InvalidInput(format!(
                    "downstream_lane_id {downstream_lane_id} is not replayable: {message}"
                )),
                other => other,
            })?;
        require_equal("downstream.run_id", &lane.run_id, "run_id", run_id)?;
        let records = sqlx::query(
            r#"
            SELECT record_json
            FROM model_lane_context_bundle_handoffs
            WHERE run_id = $1
              AND context_bundle_id = $2
              AND downstream_lane_id = $3
            ORDER BY event_ledger_seq ASC
            "#,
        )
        .bind(run_id)
        .bind(context_bundle_id)
        .bind(downstream_lane_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>>>()?;
        if records.is_empty() {
            tx.rollback().await?;
            return Err(ModelLaneError::InvalidInput(format!(
                "context_bundle_id {context_bundle_id} has no replayable handoffs for downstream_lane_id {downstream_lane_id}"
            )));
        }
        for record in &records {
            let artifact = context_bundle_artifact_binding_by_ref_tx(
                &mut tx,
                &record.run_id,
                &record.artifact_ref,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "artifact_ref {} is not backed by ArtifactStore/EventLedger authority",
                    record.artifact_ref
                ))
            })?;
            require_equal(
                "replay.artifact_sha256",
                &record.artifact_sha256,
                "artifact_binding.artifact_sha256",
                &artifact.artifact_sha256,
            )?;
            require_equal(
                "replay.content_hash",
                &record.content_hash,
                "artifact_binding.content_hash",
                &artifact.content_hash,
            )?;
        }
        tx.commit().await?;
        Ok(build_downstream_context_bundle(
            run_id,
            context_bundle_id,
            downstream_lane_id,
            records,
        )?)
    }

    pub async fn replay_context_bundle_handoffs(
        &self,
        run_id: &str,
        context_bundle_id: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        require_token("run_id", run_id)?;
        require_token("context_bundle_id", context_bundle_id)?;
        sqlx::query(
            r#"
            SELECT record_json
            FROM model_lane_context_bundle_handoffs
            WHERE run_id = $1 AND context_bundle_id = $2
            ORDER BY event_ledger_seq ASC
            "#,
        )
        .bind(run_id)
        .bind(context_bundle_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect()
    }

    pub async fn record_lane_terminal_status(
        &self,
        lane_id: &str,
        status: ModelLaneStatus,
        reason: &str,
    ) -> ModelLaneResult<ModelLaneRecord> {
        require_token("lane_id", lane_id)?;
        require_token("terminal_reason", reason)?;
        if !matches!(
            status,
            ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
        ) {
            return Err(ModelLaneError::InvalidInput(format!(
                "terminal lane update requires completed, failed, or cancelled status; got {}",
                status.as_str()
            )));
        }

        let mut tx = self.pool.begin().await?;
        let terminal_idempotency_key = format!("model-lane-terminal:{lane_id}");
        lock_idempotency_key_tx(&mut tx, &terminal_idempotency_key).await?;
        let existing = lane_by_id_tx(&mut tx, lane_id).await?;
        if matches!(
            existing.status,
            ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
        ) {
            if existing.status == status {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "lane_id {lane_id} is already terminal as {}",
                existing.status.as_str()
            )));
        }

        let mut lane = existing.inner.clone();
        lane.status = status.clone();
        lane.recovery_state = recovery_for_status(&status);
        lane.failstate_code = match status {
            ModelLaneStatus::Completed => None,
            ModelLaneStatus::Failed => Some("failed".into()),
            ModelLaneStatus::Cancelled => Some("cancelled".into()),
            _ => unreachable!("terminal status validated above"),
        };
        if status == ModelLaneStatus::Failed && lane.startup_failure_ref.is_none() {
            lane.startup_failure_ref = Some(format!("terminal-failure://dexterity/{lane_id}"));
        }
        lane.reason_ref = Some(format!(
            "terminal-reason://dexterity/{lane_id}/{}",
            status.as_str()
        ));
        lane.recovery_hint_ref = Some("usermanual://model-lane-launch-adapters#recovery".into());
        lane.last_runtime_status_ref = Some(format!(
            "runtime-status://dexterity/{lane_id}/{}",
            status.as_str()
        ));
        lane.last_recovery_event_ref = Some(format!(
            "event-ledger://dexterity/{lane_id}/{}",
            status.as_str()
        ));
        validate_lane(&lane)?;

        let event_type = match status {
            ModelLaneStatus::Completed => KernelEventType::SessionCompleted,
            ModelLaneStatus::Failed => KernelEventType::SessionFailed,
            ModelLaneStatus::Cancelled => KernelEventType::SessionCancelled,
            _ => unreachable!("terminal status validated above"),
        };
        let payload = json!({
            "schema_id": "hsk.model_lane_terminal@1",
            "dexterity_kernel": "Dexterity",
            "lane_id": &lane.lane_id,
            "run_id": &lane.run_id,
            "status": status.as_str(),
            "reason": reason,
            "previous_event_ledger_event_id": &existing.event_ledger_event_id,
            "previous_event_ledger_seq": existing.event_ledger_seq,
        });
        let event = model_lane_event(
            event_type,
            "model_lane_terminal",
            &lane.lane_id,
            &terminal_idempotency_key,
            lane.work_packet_id.as_deref().unwrap_or(&lane.run_id),
            &lane.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        lane.last_recovery_event_ref = Some(stored_event.event_id.clone());
        let record = ModelLaneRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: stored_event.event_sequence,
            inner: lane,
        };

        // The mutable `model_lanes` row is repointed below to this terminal
        // event. `validate_diagnostics_row_eventledger_authority` (invoked by
        // replay_run/diagnostics_projection) requires the row's
        // event_ledger_event_id to resolve to an EventLedger payload whose
        // `record` matches the row. Re-stamp the terminal event payload with the
        // full updated lane record so that invariant holds instead of failing
        // with "model_lane EventLedger payload missing record".
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            json!({
                "schema_id": "hsk.model_lane_terminal@1",
                "dexterity_kernel": "Dexterity",
                "lane_id": &record.lane_id,
                "run_id": &record.run_id,
                "status": status.as_str(),
                "reason": reason,
                "previous_event_ledger_event_id": &existing.event_ledger_event_id,
                "previous_event_ledger_seq": existing.event_ledger_seq,
                "record": serde_json::to_value(&record.inner)?,
            }),
        )
        .await?;

        let row = sqlx::query(
            r#"
            UPDATE model_lanes
            SET status = $2,
                event_ledger_event_id = $3,
                event_ledger_seq = $4,
                record_json = $5,
                updated_at = NOW()
            WHERE lane_id = $1
            RETURNING record_json
            "#,
        )
        .bind(lane_id)
        .bind(record.status.as_str())
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn replay_run(&self, run_id: &str) -> ModelLaneResult<ModelLaneReplay> {
        require_token("run_id", run_id)?;
        validate_diagnostics_row_eventledger_authority(&self.pool, run_id).await?;
        let run = sqlx::query("SELECT record_json FROM model_lane_runs WHERE run_id = $1")
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await?
            .map(|row| row_to_json(row, "record_json"))
            .transpose()?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))?;

        let lanes = sqlx::query(
            "SELECT record_json FROM model_lanes WHERE run_id = $1 ORDER BY event_ledger_seq ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneRecord>>>()?;

        let messages = sqlx::query(
            "SELECT record_json FROM model_lane_messages WHERE run_id = $1 ORDER BY event_ledger_seq ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into)))
        .collect::<ModelLaneResult<Vec<ModelLaneMessageRecord>>>()?;

        Ok(ModelLaneReplay {
            run: serde_json::from_value(run)?,
            lanes,
            messages,
        })
    }

    pub async fn latest_diagnostics_projection(
        &self,
    ) -> ModelLaneResult<ModelLaneDiagnosticsProjection> {
        let run_id: String = sqlx::query_scalar(
            "SELECT run_id FROM model_lane_runs ORDER BY event_ledger_seq DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ModelLaneError::NotFound("no model lane runs recorded".into()))?;
        self.diagnostics_projection(&run_id).await
    }

    pub async fn diagnostics_projection(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneDiagnosticsProjection> {
        validate_diagnostics_row_eventledger_authority(&self.pool, run_id).await?;
        let replay = self.replay_run(run_id).await?;
        let tier_posture = self
            .validate_diagnostic_tier_posture(run_id, "HBR-INT-009")
            .await?;
        let mt_runtime_statuses = sqlx::query(
            "SELECT record_json FROM model_lane_mt_runtime_statuses WHERE run_id = $1 ORDER BY event_ledger_seq ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneMtRuntimeStatusRecord>>>()?;
        let leases = sqlx::query(
            "SELECT record_json FROM model_lane_leases WHERE run_id = $1 ORDER BY event_ledger_seq ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneLeaseRecord>>>()?;
        let active_lease_count = leases
            .iter()
            .filter(|lease| lease.state == ModelLaneLeaseState::Active)
            .count();
        let reclaimable_leases = leases
            .iter()
            .filter(|lease| {
                lease.state == ModelLaneLeaseState::Active
                    && parse_utc("lease_expires_at_utc", &lease.lease_expires_at_utc)
                        .map(|expires| expires <= Utc::now())
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        let reclaimable_lane_ids = reclaimable_leases
            .iter()
            .filter_map(|lease| lease.lane_id.as_ref())
            .cloned()
            .collect::<BTreeSet<_>>();
        let reclaimable_lease_ids = reclaimable_leases
            .iter()
            .map(|lease| lease.lease_id.clone())
            .collect::<Vec<_>>();
        let messages_by_lane = replay.messages.iter().fold(
            BTreeMap::<String, Vec<&ModelLaneMessageRecord>>::new(),
            |mut acc, msg| {
                acc.entry(msg.from_lane_id.clone()).or_default().push(msg);
                acc
            },
        );
        let lanes = replay
            .lanes
            .iter()
            .map(|lane| {
                let lane_messages = messages_by_lane
                    .get(&lane.lane_id)
                    .cloned()
                    .unwrap_or_default();
                let payload_error_count = lane_messages
                    .iter()
                    .filter(|msg| {
                        msg.failstate_code.is_some()
                            || msg
                                .diagnostic_payload
                                .get("payload_error")
                                .and_then(Value::as_str)
                                .is_some()
                    })
                    .count();
                let last_activity_utc = lane_messages
                    .iter()
                    .map(|msg| msg.created_at_utc.clone())
                    .max()
                    .or_else(|| lane.heartbeat_at_utc.clone());
                ModelLaneDiagnosticsLane {
                    lane_id: lane.lane_id.clone(),
                    kind: lane.kind.as_str().to_owned(),
                    role: lane.role.clone(),
                    backend: lane.backend.clone(),
                    status: lane.status.as_str().to_owned(),
                    recovery_state: lane.recovery_state.as_str().to_owned(),
                    model_id: lane.model_id.clone(),
                    session_id: lane.session_id.clone(),
                    model_session_id: lane.model_session_id.clone(),
                    adapter_id: lane.adapter_id.clone(),
                    provider_kind: lane.provider_kind.as_str().to_owned(),
                    runtime_binding: lane.runtime_binding.as_str().to_owned(),
                    launch_authority: lane.launch_authority.as_str().to_owned(),
                    capability_token_ids: lane.capability_token_ids.clone(),
                    effective_capability_snapshot_ref: lane
                        .effective_capability_snapshot_ref
                        .clone(),
                    capability_negotiation_ref: lane.capability_negotiation_ref.clone(),
                    provider_feature_profile_ref: lane.provider_feature_profile_ref.clone(),
                    requested_execution_policy_ref: lane.requested_execution_policy_ref.clone(),
                    effective_execution_policy_ref: lane.effective_execution_policy_ref.clone(),
                    projection_plan_ref: lane.projection_plan_ref.clone(),
                    consent_receipt_ref: lane.consent_receipt_ref.clone(),
                    tool_gate_decision_refs: lane.tool_gate_decision_refs.clone(),
                    trace_id: lane.trace_id.clone(),
                    lane_span_id: lane.lane_span_id.clone(),
                    event_ledger_event_id: lane.event_ledger_event_id.clone(),
                    event_ledger_seq: lane.event_ledger_seq,
                    flight_recorder_correlation_id: lane.event_ledger_event_id.clone(),
                    last_activity_utc,
                    message_count: lane_messages.len(),
                    payload_error_count,
                    orphan_state: if reclaimable_lane_ids.contains(&lane.lane_id) {
                        "reclaimable"
                    } else {
                        "none"
                    }
                    .to_owned(),
                    cancellation_ref: lane.cancellation_ref.clone(),
                    reclaim_policy_ref: lane.reclaim_policy_ref.clone(),
                    terminal_status_mapping_ref: lane.terminal_status_mapping_ref.clone(),
                    process_ownership_ref: lane.process_ownership_ref.clone(),
                    no_os_process_reason_ref: lane.no_os_process_reason_ref.clone(),
                    last_runtime_status_ref: lane.last_runtime_status_ref.clone(),
                    last_recovery_event_ref: lane.last_recovery_event_ref.clone(),
                    failstate_code: lane.failstate_code.clone(),
                    startup_failure_ref: lane.startup_failure_ref.clone(),
                    reason_ref: lane.reason_ref.clone(),
                    recovery_hint_ref: lane.recovery_hint_ref.clone(),
                    work_packet_id: lane.work_packet_id.clone(),
                    micro_task_id: lane.micro_task_id.clone(),
                    task_board_id: lane.task_board_id.clone(),
                    owner_session: lane.owner_session.clone(),
                    locus_ref: lane
                        .locus_binding
                        .as_ref()
                        .map(|binding| binding.locus_binding_ref.clone()),
                }
            })
            .collect::<Vec<_>>();
        let messages = replay
            .messages
            .iter()
            .map(|message| ModelLaneDiagnosticsMessage {
                message_id: message.message_id.clone(),
                from_lane_id: message.from_lane_id.clone(),
                to_lane: model_lane_target_label(&message.to_lane),
                routing_target_role: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.target_role.clone()),
                routing_target_session: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.target_session.clone()),
                routing_correlation_id: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.correlation_id.clone()),
                routing_requires_ack: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.requires_ack)
                    .unwrap_or(false),
                routing_ack_for: message
                    .routing
                    .as_ref()
                    .and_then(|routing| routing.ack_for.clone()),
                kind: message.kind.as_str().to_owned(),
                authority: message.authority.as_str().to_owned(),
                promotion_state: message
                    .promotion_decision_id
                    .as_ref()
                    .map(|_| "decision_recorded")
                    .unwrap_or_else(|| message.authority.as_str())
                    .to_owned(),
                payload_ref: message.payload_ref.clone(),
                payload_sha256: message.payload_sha256.clone(),
                artifact_ref: message
                    .promoted_artifact_ref
                    .clone()
                    .or_else(|| json_string(&message.diagnostic_payload, "artifact_ref")),
                promotion_decision_id: message.promotion_decision_id.clone(),
                promotion_gate_ref: message.promotion_gate_ref.clone(),
                promotion_receipt_ref: message.promotion_receipt_ref.clone(),
                validator_verdict_ref: message.validator_verdict_ref.clone(),
                operator_decision_ref: message.operator_decision_ref.clone(),
                promoted_artifact_sha256: message.promoted_artifact_sha256.clone(),
                promoted_artifact_version: message.promoted_artifact_version.clone(),
                tool_gate_decision_refs: message.tool_gate_decision_refs.clone(),
                coordinator_session_id: message.coordinator_session_id.clone(),
                work_packet_id: message.work_packet_id.clone(),
                micro_task_id: message.micro_task_id.clone(),
                task_board_id: message.task_board_id.clone(),
                owner_session: message.owner_session.clone(),
                trace_id: message.trace_id.clone(),
                message_span_id: message.message_span_id.clone(),
                parent_span_id: message.parent_span_id.clone(),
                linked_span_contexts: message.linked_span_contexts.clone(),
                event_ledger_event_id: message.event_ledger_event_id.clone(),
                event_ledger_seq: message.event_ledger_seq,
                flight_recorder_correlation_id: message.event_ledger_event_id.clone(),
                locus_ref: message
                    .locus_binding
                    .as_ref()
                    .map(|binding| binding.locus_binding_ref.clone())
                    .or_else(|| json_string(&message.diagnostic_payload, "locus_ref")),
                loom_ref: json_string(&message.diagnostic_payload, "loom_ref"),
                fems_ref: json_string(&message.diagnostic_payload, "fems_ref"),
                proposal_ref: message.proposal_ref.clone(),
                crdt_update_ref: message.crdt_update_ref.clone(),
                crdt_base_snapshot_ref: message.crdt_base_snapshot_ref.clone(),
                crdt_state_vector: message.crdt_state_vector.clone(),
                crdt_proposal_ref: message.crdt_proposal_ref.clone(),
                crdt_stale_base_ref: message.crdt_stale_base_ref.clone(),
                payload_error: message
                    .failstate_code
                    .clone()
                    .or_else(|| json_string(&message.diagnostic_payload, "payload_error")),
                reason_ref: message.reason_ref.clone(),
                recovery_hint_ref: message.recovery_hint_ref.clone(),
                created_at_utc: message.created_at_utc.clone(),
            })
            .collect::<Vec<_>>();

        Ok(ModelLaneDiagnosticsProjection {
            schema_id: "hsk.model_lane_diagnostics_projection@1".to_owned(),
            surface_contract_id: "native_swarm_lane_diagnostics".to_owned(),
            run: ModelLaneDiagnosticsRun {
                run_id: replay.run.run_id.clone(),
                trace_id: replay.run.trace_id.clone(),
                run_span_id: replay.run.run_span_id.clone(),
                coordinator_session_id: replay.run.coordinator_session_id.clone(),
                routing_policy: replay.run.routing_policy.clone(),
                artifact_namespace: replay.run.artifact_namespace.clone(),
                projection_plan_ref: replay.run.projection_plan_ref.clone(),
                consent_receipt_ref: replay.run.consent_receipt_ref.clone(),
                work_packet_id: replay.run.work_packet_id.clone(),
                micro_task_id: replay.run.micro_task_id.clone(),
                task_board_id: replay.run.task_board_id.clone(),
                owner_session: replay.run.owner_session.clone(),
                event_ledger_event_id: replay.run.event_ledger_event_id.clone(),
                event_ledger_seq: replay.run.event_ledger_seq,
                flight_recorder_correlation_id: replay.run.event_ledger_event_id.clone(),
                context_bundle_id: replay.run.context_bundle_id.clone(),
                memory_pack_ref: replay.run.memory_pack_ref.clone(),
                memory_pack_hash: replay.run.memory_pack_hash.clone(),
                locus_ref: replay
                    .run
                    .locus_binding
                    .as_ref()
                    .map(|binding| binding.locus_binding_ref.clone()),
                loom_ref: None,
                fems_ref: None,
                status: replay.run.recovery_state.as_str().to_owned(),
                recovery_hint_ref: replay.run.recovery_hint_ref.clone(),
                selected_model_id: replay.run.selected_model_id.clone(),
                candidate_model_ids: replay.run.candidate_model_ids.clone(),
                budget_summary_ref: replay.run.budget_summary_ref.clone(),
                determinism_mode: replay.run.determinism_mode.clone(),
            },
            lanes,
            messages,
            diagnostic_tiers: tier_posture
                .tiers
                .into_iter()
                .map(|tier| ModelLaneDiagnosticsTier {
                    tier: tier.tier.as_str().to_owned(),
                    state: tier.state.as_str().to_owned(),
                    reason: tier.reason.clone(),
                    evidence_ref: tier.evidence_ref.clone(),
                    follow_up_ref: tier.follow_up_ref.clone(),
                })
                .collect(),
            mt_runtime_statuses: mt_runtime_statuses
                .into_iter()
                .map(|status| ModelLaneDiagnosticsMtStatus {
                    micro_task_id: status.micro_task_id.clone(),
                    status: status.status.as_str().to_owned(),
                    proof_status_ref: status.proof_status_ref.clone(),
                    hbr_status_ref: status.hbr_status_ref.clone(),
                    event_ledger_event_id: status.event_ledger_event_id.clone(),
                    event_ledger_seq: status.event_ledger_seq,
                })
                .collect(),
            active_lease_count,
            orphan_state: if reclaimable_lease_ids.is_empty() {
                "none".to_owned()
            } else {
                "reclaimable".to_owned()
            },
            reclaimable_lease_ids,
        })
    }

    pub async fn navigation_by_run(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        self.navigation_projection_for_run("model_lane.navigation.run", "run", run_id, run_id)
            .await
    }

    pub async fn navigation_by_lane(
        &self,
        lane_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("lane_id", lane_id)?;
        let lane = select_record_by_column::<ModelLaneRecord>(
            &self.pool,
            "model_lanes",
            "lane_id",
            lane_id,
        )
        .await?
        .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.lane",
                "lane",
                lane_id,
                &lane.run_id,
            )
            .await?;
        projection.lanes.retain(|row| row.lane_id == lane_id);
        projection
            .messages
            .retain(|row| message_mentions_lane(row, lane_id));
        projection
            .recovery_checkpoints
            .retain(|row| row.lane_id.as_deref() == Some(lane_id));
        projection
            .recovery_events
            .retain(|row| row.lane_id.as_deref() == Some(lane_id));
        projection
            .leases
            .retain(|row| row.lane_id.as_deref() == Some(lane_id));
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_message(
        &self,
        message_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("message_id", message_id)?;
        let message = select_record_by_column::<ModelLaneMessageRecord>(
            &self.pool,
            "model_lane_messages",
            "message_id",
            message_id,
        )
        .await?
        .ok_or_else(|| ModelLaneError::NotFound(format!("message_id {message_id}")))?;
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.message",
                "message",
                message_id,
                &message.run_id,
            )
            .await?;
        projection
            .messages
            .retain(|row| row.message_id == message_id);
        projection
            .lanes
            .retain(|row| message_mentions_lane(&message, &row.lane_id));
        projection.artifacts.retain(|row| {
            row.artifact_ref == message.payload_ref
                || row.artifact_payload_ref == message.payload_ref
                || row.artifact_sha256 == message.payload_sha256
                || row.content_hash == message.payload_sha256
        });
        projection
            .context_handoffs
            .retain(|row| row.source_message_id == message_id);
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_artifact_or_context(
        &self,
        artifact_ref: Option<&str>,
        context_bundle_id: Option<&str>,
        run_id: Option<&str>,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        if artifact_ref.is_none() && context_bundle_id.is_none() {
            return Err(ModelLaneError::InvalidInput(
                "artifact_ref or context_bundle_id is required".into(),
            ));
        }
        if let Some(value) = artifact_ref {
            require_token("artifact_ref", value)?;
        }
        if let Some(value) = context_bundle_id {
            require_token("context_bundle_id", value)?;
        }
        if let Some(value) = run_id {
            require_token("run_id", value)?;
        }

        let artifacts = match artifact_ref {
            Some(value) => self.context_artifacts_by_ref(value).await?,
            None => Vec::new(),
        };
        let mut handoffs = match context_bundle_id {
            Some(value) => self.context_handoffs_by_context(value).await?,
            None => Vec::new(),
        };
        if let Some(value) = artifact_ref {
            handoffs.extend(self.context_handoffs_by_artifact_ref(value).await?);
        }
        dedupe_context_handoffs(&mut handoffs);
        let context_run = if let Some(value) = context_bundle_id {
            select_record_by_json_field::<ModelLaneRunRecord>(
                &self.pool,
                "model_lane_runs",
                "context_bundle_id",
                value,
            )
            .await?
        } else {
            None
        };

        let derived_run_id = if let Some(value) = run_id {
            value.to_owned()
        } else {
            let mut run_ids = artifacts
                .iter()
                .map(|row| row.run_id.clone())
                .collect::<Vec<_>>();
            run_ids.extend(handoffs.iter().map(|row| row.run_id.clone()));
            if let Some(row) = context_run.as_ref() {
                run_ids.push(row.run_id.clone());
            }
            unique_run_id_for_lookup(
                "artifact_context",
                artifact_ref
                    .or(context_bundle_id)
                    .unwrap_or("artifact_context"),
                run_ids,
            )?
            .ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "artifact_ref {:?} context_bundle_id {:?}",
                    artifact_ref, context_bundle_id
                ))
            })?
        };
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.artifact_context",
                "artifact_context",
                artifact_ref
                    .or(context_bundle_id)
                    .unwrap_or("artifact_context"),
                &derived_run_id,
            )
            .await?;
        if let Some(value) = artifact_ref {
            projection
                .artifacts
                .retain(|row| artifact_matches(row, value));
            projection.context_handoffs.retain(|row| {
                row.artifact_ref == value
                    || row.artifact_sha256 == value
                    || row.content_hash == value
            });
            let artifact_message_refs: BTreeSet<String> = projection
                .artifacts
                .iter()
                .flat_map(|artifact| {
                    [
                        artifact.artifact_ref.as_str(),
                        artifact.artifact_manifest_ref.as_str(),
                        artifact.artifact_payload_ref.as_str(),
                        artifact.artifact_sha256.as_str(),
                        artifact.content_hash.as_str(),
                    ]
                })
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect();
            projection.messages.retain(|row| {
                artifact_message_refs.contains(&row.payload_ref)
                    || artifact_message_refs.contains(&row.payload_sha256)
                    || row.payload_ref == value
                    || row.payload_sha256 == value
            });
        }
        if let Some(value) = context_bundle_id {
            projection
                .context_handoffs
                .retain(|row| row.context_bundle_id == value);
        }
        let artifact_matched = artifact_ref.is_none()
            || !projection.artifacts.is_empty()
            || !projection.context_handoffs.is_empty()
            || !projection.messages.is_empty();
        let context_matched = context_bundle_id.is_none()
            || context_bundle_id.is_some_and(|value| {
                projection
                    .run
                    .as_ref()
                    .is_some_and(|row| row.context_bundle_id == value)
            })
            || !projection.context_handoffs.is_empty();
        if !artifact_matched || !context_matched {
            return Err(ModelLaneError::NotFound(format!(
                "artifact_ref {:?} context_bundle_id {:?} run_id {:?}",
                artifact_ref, context_bundle_id, run_id
            )));
        }
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_trace(
        &self,
        trace_id: &str,
        span_id: Option<&str>,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("trace_id", trace_id)?;
        if let Some(value) = span_id {
            require_token("span_id", value)?;
        }
        let run = select_record_by_column::<ModelLaneRunRecord>(
            &self.pool,
            "model_lane_runs",
            "trace_id",
            trace_id,
        )
        .await?;
        let run_id = if let Some(run) = run {
            run.run_id.clone()
        } else if let Some(lane) = select_record_by_column::<ModelLaneRecord>(
            &self.pool,
            "model_lanes",
            "trace_id",
            trace_id,
        )
        .await?
        {
            lane.run_id.clone()
        } else if let Some(message) = select_record_by_column::<ModelLaneMessageRecord>(
            &self.pool,
            "model_lane_messages",
            "trace_id",
            trace_id,
        )
        .await?
        {
            message.run_id.clone()
        } else {
            return Err(ModelLaneError::NotFound(format!("trace_id {trace_id}")));
        };
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.trace_span",
                "trace_span",
                span_id.unwrap_or(trace_id),
                &run_id,
            )
            .await?;
        projection.run = projection
            .run
            .filter(|row| row.trace_id == trace_id && span_matches(span_id, &row.run_span_id));
        projection
            .lanes
            .retain(|row| row.trace_id == trace_id && span_matches(span_id, &row.lane_span_id));
        projection.messages.retain(|row| {
            row.trace_id == trace_id
                && (span_matches(span_id, &row.message_span_id)
                    || row.parent_span_id.as_deref() == span_id
                    || row
                        .linked_span_contexts
                        .iter()
                        .any(|linked| Some(linked.as_str()) == span_id))
        });
        projection
            .context_handoffs
            .retain(|row| row.trace_id == trace_id && span_matches(span_id, &row.handoff_span_id));
        projection
            .recovery_events
            .retain(|row| row.trace_id == trace_id && span_matches(span_id, &row.span_id));
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_diagnostics(
        &self,
        run_id: &str,
        behavior_id: Option<&str>,
        tier: Option<&str>,
        mt_id: Option<&str>,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("run_id", run_id)?;
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.diagnostic_tier",
                "diagnostic_tier",
                behavior_id.or(tier).or(mt_id).unwrap_or(run_id),
                run_id,
            )
            .await?;
        if let Some(value) = behavior_id {
            projection
                .diagnostic_tiers
                .retain(|row| row.behavior_id == value);
        }
        if let Some(value) = tier {
            projection
                .diagnostic_tiers
                .retain(|row| row.tier.as_str() == value);
        }
        if let Some(value) = mt_id {
            projection
                .mt_runtime_statuses
                .retain(|row| row.micro_task_id == value);
        }
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_recovery(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        self.navigation_projection_for_run(
            "model_lane.navigation.recovery",
            "recovery",
            run_id,
            run_id,
        )
        .await
    }

    pub async fn navigation_by_lookup(
        &self,
        lookup: ModelLaneNavigationLookup,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        let (lookup_kind, lookup_ref, run_id) = self.resolve_navigation_lookup(lookup).await?;
        self.navigation_projection_for_run(
            "model_lane.navigation.lookup",
            &lookup_kind,
            &lookup_ref,
            &run_id,
        )
        .await
    }

    async fn resolve_navigation_lookup(
        &self,
        lookup: ModelLaneNavigationLookup,
    ) -> ModelLaneResult<(String, String, String)> {
        let requested = lookup.requested()?;
        let (lookup_kind, lookup_ref) = requested;
        let run_id = match lookup_kind.as_str() {
            "run_id" => lookup_ref.clone(),
            "lane_id" => select_record_by_column::<ModelLaneRecord>(
                &self.pool,
                "model_lanes",
                "lane_id",
                &lookup_ref,
            )
            .await?
            .map(|row| row.run_id.clone())
            .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lookup_ref}")))?,
            "message_id" => select_record_by_column::<ModelLaneMessageRecord>(
                &self.pool,
                "model_lane_messages",
                "message_id",
                &lookup_ref,
            )
            .await?
            .map(|row| row.run_id.clone())
            .ok_or_else(|| ModelLaneError::NotFound(format!("message_id {lookup_ref}")))?,
            "model_session_id" => self
                .run_id_by_model_session_id(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("model_session_id {lookup_ref}"))
                })?,
            "session_id" => self
                .run_id_by_session_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("session_id {lookup_ref}")))?,
            "wp_id" | "work_packet_id" => self
                .run_id_by_work_packet_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("wp_id {lookup_ref}")))?,
            "mt_id" | "micro_task_id" => self
                .run_id_by_micro_task_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("mt_id {lookup_ref}")))?,
            "task_board_id" => self
                .run_id_by_task_board_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("task_board_id {lookup_ref}")))?,
            "artifact_ref" => self
                .run_id_by_artifact_ref(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("artifact_ref {lookup_ref}")))?,
            "context_bundle_id" => self
                .run_id_by_context_bundle_id(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("context_bundle_id {lookup_ref}"))
                })?,
            "locus_ref" | "locus_binding_ref" => self
                .run_id_by_locus_ref(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("locus_ref {lookup_ref}")))?,
            "loom_ref" => self
                .run_id_by_diagnostic_payload_ref(&lookup_ref, &["loom_ref", "loom_block_id"])
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("loom_ref {lookup_ref}")))?,
            "loom_block_id" => self
                .run_id_by_loom_block_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("loom_block_id {lookup_ref}")))?,
            "fems_ref" => self
                .run_id_by_diagnostic_payload_ref(&lookup_ref, &["fems_ref", "memory_pack_ref"])
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("fems_ref {lookup_ref}")))?,
            "memory_pack_ref" => self
                .run_id_by_memory_pack_ref(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("memory_pack_ref {lookup_ref}")))?,
            "memory_pack_hash" => self
                .run_id_by_memory_pack_hash(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("memory_pack_hash {lookup_ref}"))
                })?,
            "event_ledger_event_id" => self
                .run_id_by_event_ledger_event_id(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("event_ledger_event_id {lookup_ref}"))
                })?,
            "event_ledger_seq" => {
                let seq = lookup_ref.parse::<i64>().map_err(|err| {
                    ModelLaneError::InvalidInput(format!("event_ledger_seq must be i64: {err}"))
                })?;
                self.run_id_by_event_ledger_seq(seq).await?.ok_or_else(|| {
                    ModelLaneError::NotFound(format!("event_ledger_seq {lookup_ref}"))
                })?
            }
            "trace_id" => self
                .run_id_by_trace_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("trace_id {lookup_ref}")))?,
            "span_id" => self
                .run_id_by_span_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("span_id {lookup_ref}")))?,
            "error_code" => self
                .run_id_by_error_code(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("error_code {lookup_ref}")))?,
            other => {
                return Err(ModelLaneError::InvalidInput(format!(
                    "unsupported ModelLane navigation lookup kind {other}"
                )));
            }
        };
        Ok((lookup_kind, lookup_ref, run_id))
    }

    async fn run_id_by_model_session_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `model_lanes` stores model_session_id only inside record_json; the
        // recovery tables carry it as physical columns.
        if let Some(row) = select_record_by_json_field::<ModelLaneRecord>(
            &self.pool,
            "model_lanes",
            "model_session_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        if let Some(row) = select_record_by_column::<ModelLaneRecoveryEventRecord>(
            &self.pool,
            "model_lane_recovery_events",
            "model_session_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        select_record_by_column::<ModelLaneRecoveryCheckpointRecord>(
            &self.pool,
            "model_lane_recovery_checkpoints",
            "model_session_id",
            value,
        )
        .await
        .map(|row| row.map(|row| row.run_id.clone()))
    }

    async fn run_id_by_session_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `model_lanes` stores session_id only inside record_json; the recovery
        // tables carry it as physical columns.
        if let Some(row) = select_record_by_json_field::<ModelLaneRecord>(
            &self.pool,
            "model_lanes",
            "session_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        if let Some(row) = select_record_by_column::<ModelLaneRecoveryEventRecord>(
            &self.pool,
            "model_lane_recovery_events",
            "session_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        select_record_by_column::<ModelLaneRecoveryCheckpointRecord>(
            &self.pool,
            "model_lane_recovery_checkpoints",
            "session_id",
            value,
        )
        .await
        .map(|row| row.map(|row| row.run_id.clone()))
    }

    async fn run_id_by_work_packet_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let run_ids =
            select_run_ids_by_column(&self.pool, "model_lane_runs", "work_packet_id", value)
                .await?;
        unique_run_id_for_lookup("wp_id", value, run_ids)
    }

    async fn run_id_by_micro_task_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids =
            select_run_ids_by_column(&self.pool, "model_lane_runs", "micro_task_id", value).await?;
        run_ids.extend(
            select_run_ids_by_column(
                &self.pool,
                "model_lane_mt_runtime_statuses",
                "micro_task_id",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("mt_id", value, run_ids)
    }

    async fn run_id_by_task_board_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids =
            select_run_ids_by_column(&self.pool, "model_lane_runs", "task_board_id", value).await?;
        run_ids.extend(
            select_run_ids_by_column(
                &self.pool,
                "model_lane_mt_runtime_statuses",
                "task_board_id",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("task_board_id", value, run_ids)
    }

    async fn run_id_by_artifact_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids = select_records_by_any_artifact_ref(&self.pool, value)
            .await?
            .into_iter()
            // MT-003 unblock (out-of-scope, pre-existing WIP commit 0adac5d8):
            // `select_records_by_any_artifact_ref` yields borrowed rows, so
            // `run_id` (String, not Copy) must be cloned out. Compiler-suggested
            // fix; behavior-preserving.
            .map(|row| row.run_id.clone())
            .collect::<Vec<_>>();
        // `payload_ref` is stored only inside record_json; `payload_sha256` is a
        // physical column on model_lane_messages.
        run_ids.extend(
            select_run_ids_by_json_field(&self.pool, "model_lane_messages", "payload_ref", value)
                .await?,
        );
        run_ids.extend(
            select_run_ids_by_column(&self.pool, "model_lane_messages", "payload_sha256", value)
                .await?,
        );
        unique_run_id_for_lookup("artifact_ref", value, run_ids)
    }

    async fn run_id_by_context_bundle_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `model_lane_runs` carries context_bundle_id only inside record_json; the
        // handoff table exposes it as a physical column.
        let mut run_ids =
            select_run_ids_by_json_field(&self.pool, "model_lane_runs", "context_bundle_id", value)
                .await?;
        run_ids.extend(
            select_run_ids_by_column(
                &self.pool,
                "model_lane_context_bundle_handoffs",
                "context_bundle_id",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("context_bundle_id", value, run_ids)
    }

    async fn run_id_by_memory_pack_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let run_ids =
            select_run_ids_by_json_field(&self.pool, "model_lane_runs", "memory_pack_ref", value)
                .await?;
        unique_run_id_for_lookup("memory_pack_ref", value, run_ids)
    }

    async fn run_id_by_memory_pack_hash(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let run_ids =
            select_run_ids_by_json_field(&self.pool, "model_lane_runs", "memory_pack_hash", value)
                .await?;
        unique_run_id_for_lookup("memory_pack_hash", value, run_ids)
    }

    async fn run_id_by_trace_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        if let Some(row) = select_record_by_column::<ModelLaneRunRecord>(
            &self.pool,
            "model_lane_runs",
            "trace_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        if let Some(row) =
            select_record_by_column::<ModelLaneRecord>(&self.pool, "model_lanes", "trace_id", value)
                .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        select_record_by_column::<ModelLaneMessageRecord>(
            &self.pool,
            "model_lane_messages",
            "trace_id",
            value,
        )
        .await
        .map(|row| row.map(|row| row.run_id.clone()))
    }

    async fn run_id_by_span_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        if let Some(row) = select_record_by_column::<ModelLaneRunRecord>(
            &self.pool,
            "model_lane_runs",
            "run_span_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        if let Some(row) = select_record_by_column::<ModelLaneRecord>(
            &self.pool,
            "model_lanes",
            "lane_span_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        if let Some(row) = select_record_by_column::<ModelLaneMessageRecord>(
            &self.pool,
            "model_lane_messages",
            "message_span_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        select_record_by_column::<ModelLaneRecoveryEventRecord>(
            &self.pool,
            "model_lane_recovery_events",
            "span_id",
            value,
        )
        .await
        .map(|row| row.map(|row| row.run_id.clone()))
    }

    async fn run_id_by_error_code(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `error_code` is a physical column on model_lane_recovery_events, but the
        // run/lane failstate_code lives only inside record_json.
        let mut run_ids = select_run_ids_by_column(
            &self.pool,
            "model_lane_recovery_events",
            "error_code",
            value,
        )
        .await?;
        run_ids.extend(
            select_run_ids_by_json_field(&self.pool, "model_lane_runs", "failstate_code", value)
                .await?,
        );
        run_ids.extend(
            select_run_ids_by_json_field(&self.pool, "model_lanes", "failstate_code", value)
                .await?,
        );
        unique_run_id_for_lookup("error_code", value, run_ids)
    }

    async fn run_id_by_locus_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        query_optional_run_id(
            &self.pool,
            r#"
            SELECT record_json->>'run_id' AS run_id
            FROM model_lane_runs
            WHERE record_json #>> '{locus_binding,locus_binding_ref}' = $1
            UNION ALL
            SELECT record_json->>'run_id' AS run_id
            FROM model_lanes
            WHERE record_json #>> '{locus_binding,locus_binding_ref}' = $1
            UNION ALL
            SELECT record_json->>'run_id' AS run_id
            FROM model_lane_messages
            WHERE record_json #>> '{locus_binding,locus_binding_ref}' = $1
               OR record_json #>> '{diagnostic_payload,locus_ref}' = $1
            LIMIT 1
            "#,
            value,
        )
        .await
    }

    async fn run_id_by_loom_block_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT run_id
            FROM model_lane_context_bundle_handoffs
            WHERE EXISTS (
                SELECT 1
                FROM jsonb_array_elements(COALESCE(record_json->'loom_refs', '[]'::jsonb)) AS loom_ref
                WHERE loom_ref->>'block_id' = $1
            )
            ORDER BY run_id ASC
            "#,
        )
        .bind(value)
        .fetch_all(&self.pool)
        .await?;
        run_ids.extend(
            sqlx::query_scalar::<_, String>(
                r#"
                SELECT DISTINCT run_id
                FROM (
                    SELECT run_id
                    FROM model_lane_messages
                    WHERE record_json #>> '{diagnostic_payload,loom_block_id}' = $1
                    UNION ALL
                    SELECT run_id
                    FROM model_lane_context_bundle_handoffs
                    WHERE record_json #>> '{diagnostic_payload,loom_block_id}' = $1
                ) legacy_refs
                ORDER BY run_id ASC
                "#,
            )
            .bind(value)
            .fetch_all(&self.pool)
            .await?,
        );
        unique_run_id_for_lookup("loom_block_id", value, run_ids)
    }

    async fn run_id_by_diagnostic_payload_ref(
        &self,
        value: &str,
        keys: &[&str],
    ) -> ModelLaneResult<Option<String>> {
        for key in keys {
            if let Some(run_id) = query_optional_run_id(
                &self.pool,
                &format!(
                    r#"
                    SELECT run_id
                    FROM model_lane_messages
                    WHERE record_json #>> '{{diagnostic_payload,{key}}}' = $1
                    UNION ALL
                    SELECT run_id
                    FROM model_lane_context_bundle_handoffs
                    WHERE record_json #>> '{{diagnostic_payload,{key}}}' = $1
                    LIMIT 1
                    "#
                ),
                value,
            )
            .await?
            {
                return Ok(Some(run_id));
            }
        }
        Ok(None)
    }

    async fn run_id_by_event_ledger_event_id(
        &self,
        value: &str,
    ) -> ModelLaneResult<Option<String>> {
        let payload = sqlx::query_scalar::<_, Value>(
            "SELECT payload FROM kernel_event_ledger WHERE event_id = $1 LIMIT 1",
        )
        .bind(value)
        .fetch_optional(&self.pool)
        .await?;
        Ok(payload.and_then(|payload| event_payload_run_id(&payload)))
    }

    async fn run_id_by_event_ledger_seq(&self, value: i64) -> ModelLaneResult<Option<String>> {
        let payload = sqlx::query_scalar::<_, Value>(
            "SELECT payload FROM kernel_event_ledger WHERE event_sequence = $1 ORDER BY event_id ASC LIMIT 1",
        )
        .bind(value)
        .fetch_optional(&self.pool)
        .await?;
        Ok(payload.and_then(|payload| event_payload_run_id(&payload)))
    }

    async fn navigation_projection_for_run(
        &self,
        route_id: &str,
        lookup_kind: &str,
        lookup_ref: &str,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        let replay = self.replay_run(run_id).await?;
        let artifacts = select_records_by_column::<ModelLaneContextBundleArtifactBindingRecord>(
            &self.pool,
            "model_lane_context_bundle_artifacts",
            "run_id",
            run_id,
        )
        .await?;
        let context_handoffs = select_records_by_column::<ModelLaneContextBundleHandoffRecord>(
            &self.pool,
            "model_lane_context_bundle_handoffs",
            "run_id",
            run_id,
        )
        .await?;
        let recovery_checkpoints = select_records_by_column::<ModelLaneRecoveryCheckpointRecord>(
            &self.pool,
            "model_lane_recovery_checkpoints",
            "run_id",
            run_id,
        )
        .await?;
        let recovery_events = select_records_by_column::<ModelLaneRecoveryEventRecord>(
            &self.pool,
            "model_lane_recovery_events",
            "run_id",
            run_id,
        )
        .await?;
        let leases = select_records_by_column::<ModelLaneLeaseRecord>(
            &self.pool,
            "model_lane_leases",
            "run_id",
            run_id,
        )
        .await?;
        let diagnostic_tiers = select_records_by_column::<ModelLaneDiagnosticTierStatusRecord>(
            &self.pool,
            "model_lane_diagnostic_tier_statuses",
            "run_id",
            run_id,
        )
        .await?;
        let mt_runtime_statuses = select_records_by_column::<ModelLaneMtRuntimeStatusRecord>(
            &self.pool,
            "model_lane_mt_runtime_statuses",
            "run_id",
            run_id,
        )
        .await?;
        let mut projection = ModelLaneNavigationProjection {
            schema_id: "hsk.model_lane_navigation@1".into(),
            surface_contract_id: "native_swarm_lane_diagnostics".into(),
            route_id: route_id.into(),
            lookup_kind: lookup_kind.into(),
            lookup_ref: lookup_ref.into(),
            input_schema_ref: "hsk.model_lane_navigation_request@1".into(),
            output_schema_ref: "hsk.model_lane_navigation@1".into(),
            manual_refs: vec![
                "usermanual://model-lane-navigation".into(),
                "usermanual://model-lane-diagnostics".into(),
                "usermanual://model-lane-recovery".into(),
                "usermanual://model-lane-validation-harness".into(),
            ],
            run: Some(replay.run),
            lanes: replay.lanes,
            messages: replay.messages,
            artifacts,
            context_handoffs,
            recovery_checkpoints,
            recovery_events,
            leases,
            diagnostic_tiers,
            mt_runtime_statuses,
            event_ledger_refs: Vec::new(),
            flight_recorder_refs: Vec::new(),
            error_codes: Vec::new(),
            recovery_routes: vec![
                "GET /swarm/model-lanes/navigation/recovery/{run_id}".into(),
                "GET /swarm/model-lanes/diagnostics/{run_id}".into(),
                "ModelLaneStore::recover_run_after_restart".into(),
                "ModelLaneStore::replay_run".into(),
            ],
        };
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    async fn context_artifacts_by_ref(
        &self,
        value: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleArtifactBindingRecord>> {
        select_records_by_any_artifact_ref(&self.pool, value).await
    }

    async fn context_handoffs_by_context(
        &self,
        context_bundle_id: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        select_records_by_column::<ModelLaneContextBundleHandoffRecord>(
            &self.pool,
            "model_lane_context_bundle_handoffs",
            "context_bundle_id",
            context_bundle_id,
        )
        .await
    }

    async fn context_handoffs_by_artifact_ref(
        &self,
        value: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        select_records_by_any_handoff_artifact_ref(&self.pool, value).await
    }

    pub async fn record_recovery_checkpoint(
        &self,
        input: NewModelLaneRecoveryCheckpoint,
    ) -> ModelLaneResult<ModelLaneRecoveryCheckpointRecord> {
        validate_recovery_checkpoint(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) =
            recovery_checkpoint_by_idempotency_key_tx(&mut tx, &input.idempotency_key).await?
        {
            ensure_idempotent_input_matches(
                "model_lane_recovery_checkpoint",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run = run_by_id_tx(&mut tx, &input.run_id).await?;
        require_equal(
            "model_lane_recovery_checkpoint.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            lane_by_id_for_run_tx(&mut tx, &input.run_id, lane_id).await?;
        }
        ensure_event_ledger_sequence_in_stream_tx(
            &mut tx,
            input.last_event_ledger_seq,
            &input.event_ledger_stream_id,
        )
        .await?;
        let payload = json!({
            "schema_id": "hsk.model_lane_recovery_checkpoint@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_recovery_checkpoint",
            &input.checkpoint_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneRecoveryCheckpointRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            recovery_checkpoint_event_payload(&record),
        )
        .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO model_lane_recovery_checkpoints (
                checkpoint_id, run_id, lane_id, session_id, model_session_id,
                lane_status, checkpoint_status, last_event_ledger_seq,
                last_message_id, open_payload_refs, lease_id,
                idempotency_scope, recovery_state, recovery_event_ref,
                event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key, created_at_utc,
                recovery_hint_ref, diagnostic_payload, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq, record_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21::timestamptz,$22,$23,$24,$25,$26,$27,$28)
            RETURNING record_json
            "#,
        )
        .bind(&record.checkpoint_id)
        .bind(&record.run_id)
        .bind(record.lane_id.as_deref())
        .bind(&record.session_id)
        .bind(&record.model_session_id)
        .bind(record.lane_status.as_str())
        .bind(record.checkpoint_status.as_str())
        .bind(record.last_event_ledger_seq)
        .bind(record.last_message_id.as_deref())
        .bind(serde_json::to_value(&record.open_payload_refs)?)
        .bind(record.lease_id.as_deref())
        .bind(&record.idempotency_scope)
        .bind(record.recovery_state.as_str())
        .bind(record.recovery_event_ref.as_deref())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.created_at_utc)
        .bind(record.recovery_hint_ref.as_deref())
        .bind(&record.diagnostic_payload)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn record_recovery_event(
        &self,
        input: NewModelLaneRecoveryEvent,
    ) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
        validate_recovery_event(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) =
            recovery_event_by_idempotency_key_tx(&mut tx, &input.idempotency_key).await?
        {
            ensure_idempotent_input_matches(
                "model_lane_recovery_event",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run = run_by_id_tx(&mut tx, &input.run_id).await?;
        require_equal(
            "model_lane_recovery_event.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            lane_by_id_for_run_tx(&mut tx, &input.run_id, lane_id).await?;
        }
        if let Some(source_event_ledger_seq) = input.source_event_ledger_seq {
            ensure_event_ledger_sequence_in_stream_tx(
                &mut tx,
                source_event_ledger_seq,
                &input.event_ledger_stream_id,
            )
            .await?;
        }
        let payload = json!({
            "schema_id": "hsk.model_lane_recovery_event@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_recovery_event",
            &input.recovery_event_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneRecoveryEventRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            recovery_event_event_payload(&record),
        )
        .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO model_lane_recovery_events (
                recovery_event_id, run_id, lane_id, trace_id, span_id,
                parent_span_id, linked_span_contexts, session_id, model_session_id,
                event_kind, recovery_status, replay_order_seq,
                source_event_ledger_seq, payload_refs, artifact_refs,
                crdt_base_snapshot_ref, crdt_state_vector, crdt_stale_base_ref,
                lease_id, failure_kind, error_code, replay_hint,
                event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                recovery_hint_ref, diagnostic_payload, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq, record_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35)
            RETURNING record_json
            "#,
        )
        .bind(&record.recovery_event_id)
        .bind(&record.run_id)
        .bind(record.lane_id.as_deref())
        .bind(&record.trace_id)
        .bind(&record.span_id)
        .bind(record.parent_span_id.as_deref())
        .bind(serde_json::to_value(&record.linked_span_contexts)?)
        .bind(record.session_id.as_deref())
        .bind(record.model_session_id.as_deref())
        .bind(record.event_kind.as_str())
        .bind(record.recovery_status.as_str())
        .bind(record.replay_order_seq)
        .bind(record.source_event_ledger_seq)
        .bind(serde_json::to_value(&record.payload_refs)?)
        .bind(serde_json::to_value(&record.artifact_refs)?)
        .bind(record.crdt_base_snapshot_ref.as_deref())
        .bind(record.crdt_state_vector.as_deref())
        .bind(record.crdt_stale_base_ref.as_deref())
        .bind(record.lease_id.as_deref())
        .bind(record.failure_kind.map(|kind| kind.as_str()))
        .bind(record.error_code.as_deref())
        .bind(&record.replay_hint)
        .bind(&record.event_ledger_stream_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(record.recovery_hint_ref.as_deref())
        .bind(&record.diagnostic_payload)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn record_lane_lease(
        &self,
        input: NewModelLaneLease,
    ) -> ModelLaneResult<ModelLaneLeaseRecord> {
        validate_lane_lease(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) =
            lane_lease_by_idempotency_key_tx(&mut tx, &input.idempotency_key).await?
        {
            ensure_idempotent_input_matches(
                "model_lane_lease",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run = run_by_id_tx(&mut tx, &input.run_id).await?;
        require_equal(
            "model_lane_lease.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            lane_by_id_for_run_tx(&mut tx, &input.run_id, lane_id).await?;
        }
        let payload = json!({
            "schema_id": "hsk.model_lane_lease@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_lease",
            &input.lease_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneLeaseRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            lane_lease_event_payload(&record),
        )
        .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO model_lane_leases (
                lease_id, run_id, lane_id, scope, scope_ref, holder_actor_id,
                holder_session_id, lease_expires_at_utc, takeover_policy_ref,
                state, event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key, recovery_hint_ref,
                diagnostic_payload, event_ledger_event_id, event_ledger_seq,
                event_stream_version, transaction_seq, record_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8::timestamptz,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)
            RETURNING record_json
            "#,
        )
        .bind(&record.lease_id)
        .bind(&record.run_id)
        .bind(record.lane_id.as_deref())
        .bind(record.scope.as_str())
        .bind(&record.scope_ref)
        .bind(&record.holder_actor_id)
        .bind(&record.holder_session_id)
        .bind(&record.lease_expires_at_utc)
        .bind(&record.takeover_policy_ref)
        .bind(record.state.as_str())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(record.recovery_hint_ref.as_deref())
        .bind(&record.diagnostic_payload)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn record_diagnostic_tier_status(
        &self,
        input: NewModelLaneDiagnosticTierStatus,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierStatusRecord> {
        validate_diagnostic_tier_status(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) =
            diagnostic_tier_by_idempotency_key_tx(&mut tx, &input.idempotency_key).await?
        {
            ensure_idempotent_input_matches(
                "model_lane_diagnostic_tier",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run = run_by_id_tx(&mut tx, &input.run_id).await?;
        require_equal(
            "diagnostic_tier.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        let payload = json!({
            "schema_id": "hsk.model_lane_diagnostic_tier@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_diagnostic_tier",
            &input.diagnostic_status_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneDiagnosticTierStatusRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            diagnostic_tier_event_payload(&record),
        )
        .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO model_lane_diagnostic_tier_statuses (
                diagnostic_status_id, behavior_id, run_id, tier, state, reason,
                evidence_ref, follow_up_ref, event_ledger_stream_id,
                work_packet_id, micro_task_id, task_board_id, owner_session,
                idempotency_key, diagnostic_payload, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq, record_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
            RETURNING record_json
            "#,
        )
        .bind(&record.diagnostic_status_id)
        .bind(&record.behavior_id)
        .bind(&record.run_id)
        .bind(record.tier.as_str())
        .bind(record.state.as_str())
        .bind(&record.reason)
        .bind(&record.evidence_ref)
        .bind(record.follow_up_ref.as_deref())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.diagnostic_payload)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn diagnostic_tier_posture(
        &self,
        run_id: &str,
        behavior_id: &str,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierPosture> {
        require_token("run_id", run_id)?;
        require_token("behavior_id", behavior_id)?;
        let tiers = sqlx::query(
            r#"
            SELECT DISTINCT ON (tier) record_json
            FROM model_lane_diagnostic_tier_statuses
            WHERE run_id = $1
              AND behavior_id = $2
            ORDER BY tier, event_ledger_seq DESC
            "#,
        )
        .bind(run_id)
        .bind(behavior_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect::<ModelLaneResult<Vec<ModelLaneDiagnosticTierStatusRecord>>>()?;
        Ok(ModelLaneDiagnosticTierPosture {
            run_id: run_id.to_string(),
            behavior_id: behavior_id.to_string(),
            tiers,
        })
    }

    pub async fn validate_diagnostic_tier_posture(
        &self,
        run_id: &str,
        behavior_id: &str,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierPosture> {
        let posture = self.diagnostic_tier_posture(run_id, behavior_id).await?;
        let have_flight = posture
            .tiers
            .iter()
            .any(|tier| tier.tier == ModelLaneDiagnosticTier::FlightRecorder);
        let have_internal = posture
            .tiers
            .iter()
            .any(|tier| tier.tier == ModelLaneDiagnosticTier::InternalDiagnostics);
        let have_palmistry = posture
            .tiers
            .iter()
            .any(|tier| tier.tier == ModelLaneDiagnosticTier::Palmistry);
        if posture
            .tiers
            .iter()
            .any(|tier| tier.state == ModelLaneDiagnosticTierState::Missing)
        {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} contains missing tier state"
            )));
        }
        if !have_flight {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} requires FlightRecorder/EventLedger tier"
            )));
        }
        if have_flight && (!have_internal || !have_palmistry) {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} is FlightRecorder-only; missing internal_diagnostics or palmistry tier"
            )));
        }
        if !have_internal || !have_palmistry {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} requires internal_diagnostics and palmistry tier records"
            )));
        }
        for tier in &posture.tiers {
            if tier.state == ModelLaneDiagnosticTierState::DeferredWithReason
                && tier.follow_up_ref.is_none()
            {
                return Err(ModelLaneError::InvalidInput(format!(
                    "HBR-INT-009 deferred tier {} for {behavior_id} requires follow_up_ref",
                    tier.tier.as_str()
                )));
            }
        }
        Ok(posture)
    }

    pub async fn record_mt_runtime_status(
        &self,
        input: NewModelLaneMtRuntimeStatus,
    ) -> ModelLaneResult<ModelLaneMtRuntimeStatusRecord> {
        validate_mt_runtime_status(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) =
            mt_runtime_status_by_idempotency_key_tx(&mut tx, &input.idempotency_key).await?
        {
            ensure_idempotent_input_matches(
                "model_lane_mt_runtime_status",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run = run_by_id_tx(&mut tx, &input.run_id).await?;
        require_equal(
            "model_lane_mt_runtime_status.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        let payload = json!({
            "schema_id": "hsk.model_lane_mt_runtime_status@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_mt_runtime_status",
            &input.mt_status_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneMtRuntimeStatusRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            mt_runtime_status_event_payload(&record),
        )
        .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO model_lane_mt_runtime_statuses (
                mt_status_id, run_id, work_packet_id, micro_task_id,
                task_board_id, status, claimed_by_ref, blocker_ref,
                missing_resource_ref, proof_status_ref, hbr_status_ref,
                last_recovery_event_ref, last_runtime_status_ref,
                event_ledger_stream_id, owner_session, idempotency_key,
                diagnostic_payload, event_ledger_event_id, event_ledger_seq,
                event_stream_version, transaction_seq, record_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22)
            RETURNING record_json
            "#,
        )
        .bind(&record.mt_status_id)
        .bind(&record.run_id)
        .bind(&record.work_packet_id)
        .bind(&record.micro_task_id)
        .bind(&record.task_board_id)
        .bind(record.status.as_str())
        .bind(record.claimed_by_ref.as_deref())
        .bind(record.blocker_ref.as_deref())
        .bind(record.missing_resource_ref.as_deref())
        .bind(record.proof_status_ref.as_deref())
        .bind(record.hbr_status_ref.as_deref())
        .bind(record.last_recovery_event_ref.as_deref())
        .bind(record.last_runtime_status_ref.as_deref())
        .bind(&record.event_ledger_stream_id)
        .bind(&record.owner_session)
        .bind(&record.idempotency_key)
        .bind(&record.diagnostic_payload)
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(record.event_stream_version)
        .bind(record.transaction_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn recover_run_after_restart(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneRecoveredRun> {
        require_token("run_id", run_id)?;
        validate_diagnostics_row_eventledger_authority(&self.pool, run_id).await?;
        let canonical_run = canonical_run_for_recovery(&self.pool, run_id).await?;
        let checkpoint =
            latest_recovery_checkpoint(&self.pool, run_id, &canonical_run.event_ledger_stream_id)
                .await?;
        require_equal(
            "recovery_checkpoint.event_ledger_stream_id",
            &checkpoint.event_ledger_stream_id,
            "canonical_run.event_ledger_stream_id",
            &canonical_run.event_ledger_stream_id,
        )?;
        validate_recovery_checkpoint_record(&self.pool, &checkpoint).await?;
        // Spec 4.3.9.2.5 + MT-007 acceptance define a PER-KIND recovery boundary, not a
        // single blunt cut at the checkpoint high-watermark. Neither "replay the whole
        // stream high-watermark" nor "bound everything at the checkpoint" is correct.
        //
        // * CATCH UP (forward stream): "Replay MUST load the latest checkpoint, apply
        //   EventLedger records AFTER that sequence in order." When the run's
        //   coordinator-owned ModelLaneMessage stream genuinely advanced past the
        //   checkpoint (a NEW message was committed), post-checkpoint forward state MUST
        //   be replayed -- messages, recovery events, MT runtime status, and the payload
        //   authority for those NEW messages/events catch up to the current stream
        //   high-watermark. Absent real forward-message progress there is nothing to
        //   catch up and the bound stays at the checkpoint.
        // * EXCLUDE (current-state adjunct): lane leases and cloud-consent denials are
        //   current-state markers, never forward replay input, so they stay bounded at
        //   the checkpoint and post-checkpoint rows are excluded.
        // * REJECT (repairs of already-checkpointed refs): a payload ref that was open
        //   AT the checkpoint, and the CRDT base a recovery event replays against, MUST
        //   have been satisfied at/before the checkpoint. A post-checkpoint artifact or
        //   CRDT "repair" of such a checkpointed ref fails closed, so those two checks
        //   stay bounded at the checkpoint.
        let checkpoint_bound_event_ledger_seq = checkpoint.last_event_ledger_seq;
        let forward_bound_event_ledger_seq = if has_post_checkpoint_forward_messages(
            &self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            checkpoint_bound_event_ledger_seq,
        )
        .await?
        {
            recovery_stream_high_watermark(&self.pool, &checkpoint.event_ledger_stream_id).await?
        } else {
            checkpoint_bound_event_ledger_seq
        };
        let mut recovery_events = recovery_events_for_run(
            &self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            forward_bound_event_ledger_seq,
        )
        .await?;
        validate_recovery_event_stream(
            &self.pool,
            run_id,
            forward_bound_event_ledger_seq,
            &recovery_events,
        )
        .await?;
        validate_recovery_payload_refs(
            &self.pool,
            run_id,
            &checkpoint,
            checkpoint_bound_event_ledger_seq,
            forward_bound_event_ledger_seq,
            &recovery_events,
        )
        .await?;
        validate_recovery_crdt_posture(
            &self.pool,
            run_id,
            &checkpoint,
            checkpoint_bound_event_ledger_seq,
            &recovery_events,
        )
        .await?;
        let replay = replay_run_at_recovery_bound(
            &self.pool,
            run_id,
            &checkpoint,
            forward_bound_event_ledger_seq,
        )
        .await?;
        validate_replay_message_payload_authority(
            &self.pool,
            run_id,
            &checkpoint,
            forward_bound_event_ledger_seq,
            &replay.messages,
        )
        .await?;
        validate_replay_message_crdt_posture(&replay.messages)?;
        let leases = lane_leases_for_run(
            &self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            checkpoint_bound_event_ledger_seq,
        )
        .await?;
        let now = Utc::now();
        let mut active_leases = Vec::new();
        let mut reclaimable_lease_ids = Vec::new();
        for lease in leases {
            if lease.state != ModelLaneLeaseState::Active {
                continue;
            }
            let expires = parse_utc("lease_expires_at_utc", &lease.lease_expires_at_utc)?;
            if expires > now {
                active_leases.push(lease);
            } else {
                if !recovery_events.iter().any(|event| {
                    event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
                        && event.lease_id.as_deref() == Some(lease.lease_id.as_str())
                }) {
                    let replay_order_seq =
                        recovery_events.len() as i64 + reclaimable_lease_ids.len() as i64 + 1;
                    let orphan_event = self
                        .record_orphan_recovery_event(&checkpoint, &lease, replay_order_seq)
                        .await?;
                    recovery_events.push(orphan_event);
                }
                reclaimable_lease_ids.push(lease.lease_id.clone());
            }
        }
        let cloud_consent_denials = cloud_consent_denials_for_run(
            &self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            checkpoint_bound_event_ledger_seq,
        )
        .await?;
        let mt_runtime_statuses = mt_runtime_statuses_for_run(
            &self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            forward_bound_event_ledger_seq,
        )
        .await?;
        Ok(ModelLaneRecoveredRun {
            replay,
            checkpoint,
            recovery_events,
            active_leases,
            reclaimable_lease_ids,
            cloud_consent_denials,
            mt_runtime_statuses,
        })
    }

    async fn record_orphan_recovery_event(
        &self,
        checkpoint: &ModelLaneRecoveryCheckpointRecord,
        lease: &ModelLaneLeaseRecord,
        replay_order_seq: i64,
    ) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
        self.record_recovery_event(NewModelLaneRecoveryEvent {
            recovery_event_id: format!(
                "recovery-event-orphan-{}-{}",
                checkpoint.checkpoint_id, lease.lease_id
            ),
            run_id: checkpoint.run_id.clone(),
            lane_id: lease.lane_id.clone(),
            trace_id: format!("trace-{}", checkpoint.run_id),
            span_id: format!("span-orphan-{}", lease.lease_id),
            parent_span_id: lease.lane_id.as_ref().map(|lane| format!("span-{lane}")),
            linked_span_contexts: vec![format!(
                "eventledger://{}/{}",
                checkpoint.event_ledger_stream_id, lease.event_ledger_seq
            )],
            session_id: Some(lease.holder_session_id.clone()),
            model_session_id: checkpoint.lane_id.as_ref().map(|lane| format!("model-session-{lane}")),
            event_kind: ModelLaneRecoveryEventKind::OrphanDetected,
            recovery_status: ModelLaneRecoveryStatus::Observed,
            replay_order_seq,
            source_event_ledger_seq: Some(lease.event_ledger_seq),
            payload_refs: Vec::new(),
            artifact_refs: vec![lease.scope_ref.clone()],
            crdt_base_snapshot_ref: None,
            crdt_state_vector: None,
            crdt_stale_base_ref: None,
            lease_id: Some(lease.lease_id.clone()),
            failure_kind: Some(ModelLaneRecoveryFailureKind::OrphanedSubagent),
            error_code: Some(ModelLaneRecoveryFailureKind::OrphanedSubagent.code().into()),
            replay_hint: "Expired active lease detected during checkpoint recovery; lane is reclaimable before relaunch".into(),
            event_ledger_stream_id: checkpoint.event_ledger_stream_id.clone(),
            work_packet_id: lease.work_packet_id.clone(),
            micro_task_id: lease.micro_task_id.clone(),
            task_board_id: lease.task_board_id.clone(),
            owner_session: lease.owner_session.clone(),
            idempotency_key: format!(
                "model-lane-orphan-recovery:{}:{}:{}",
                checkpoint.run_id, checkpoint.checkpoint_id, lease.lease_id
            ),
            recovery_hint_ref: Some("usermanual://dexterity/recovery#orphan-reclaim".into()),
            diagnostic_payload: json!({
                "flight_recorder": "EventLedger",
                "reason_code": ModelLaneRecoveryFailureKind::OrphanedSubagent.code(),
                "lease_event_ledger_seq": lease.event_ledger_seq,
                "checkpoint_id": checkpoint.checkpoint_id,
                "reclaimable": true
            }),
        })
        .await
    }

    async fn preflight_cloud_launch_records(
        &self,
        run: &NewModelLaneRun,
        lane: &NewModelLane,
    ) -> ModelLaneResult<()> {
        let check = CloudLaunchAuthorityCheck {
            run_id: run.run_id.clone(),
            lane_id: lane.lane_id.clone(),
            model_session_id: lane.model_session_id.clone(),
            provider_kind: lane.provider_kind.as_str().to_string(),
            requested_model_id: lane.model_id.clone().unwrap_or_default(),
            projection_plan_ref: lane.projection_plan_ref.clone(),
            consent_receipt_ref: lane.consent_receipt_ref.clone(),
            event_ledger_stream_id: lane.event_ledger_stream_id.clone(),
            work_packet_id: lane
                .work_packet_id
                .clone()
                .or_else(|| run.work_packet_id.clone())
                .unwrap_or_else(|| run.run_id.clone()),
            micro_task_id: lane
                .micro_task_id
                .clone()
                .or_else(|| run.micro_task_id.clone()),
            owner_session: lane.owner_session.clone(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
        };
        self.preflight_cloud_launch(check).await
    }

    async fn preflight_cloud_lane_record(&self, lane: &NewModelLane) -> ModelLaneResult<()> {
        let check = CloudLaunchAuthorityCheck {
            run_id: lane.run_id.clone(),
            lane_id: lane.lane_id.clone(),
            model_session_id: lane.model_session_id.clone(),
            provider_kind: lane.provider_kind.as_str().to_string(),
            requested_model_id: lane.model_id.clone().unwrap_or_default(),
            projection_plan_ref: lane.projection_plan_ref.clone(),
            consent_receipt_ref: lane.consent_receipt_ref.clone(),
            event_ledger_stream_id: lane.event_ledger_stream_id.clone(),
            work_packet_id: lane
                .work_packet_id
                .clone()
                .unwrap_or_else(|| lane.run_id.clone()),
            micro_task_id: lane.micro_task_id.clone(),
            owner_session: lane.owner_session.clone(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
        };
        self.preflight_cloud_launch(check).await
    }

    async fn preflight_cloud_launch(
        &self,
        check: CloudLaunchAuthorityCheck,
    ) -> ModelLaneResult<()> {
        let mut tx = self.pool.begin().await?;
        let result = ensure_cloud_launch_authority_tx(&mut tx, &check).await;
        match result {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(reason) => {
                tx.rollback().await?;
                self.deny_cloud_launch(check, &reason.to_string()).await
            }
        }
    }

    async fn deny_cloud_launch(
        &self,
        check: CloudLaunchAuthorityCheck,
        reason: &str,
    ) -> ModelLaneResult<()> {
        record_cloud_consent_denial(
            &self.pool,
            &check,
            reason,
            "CX-MM-007 cloud lane launch denied before provider call",
        )
        .await?;
        Err(ModelLaneError::InvalidInput(format!(
            "CX-MM-007 cloud lane launch denied for run_id {} lane_id {}: {reason}",
            check.run_id, check.lane_id
        )))
    }

    pub async fn schema_registry_rows(&self) -> ModelLaneResult<Vec<ModelLaneSchemaRegistryRow>> {
        sqlx::query(
            r#"
            SELECT schema_id, schema_version, record_kind, table_name
            FROM model_lane_schema_registry
            ORDER BY schema_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ModelLaneSchemaRegistryRow {
                schema_id: row.try_get("schema_id")?,
                schema_version: row.try_get("schema_version")?,
                record_kind: row.try_get("record_kind")?,
                table_name: row.try_get("table_name")?,
            })
        })
        .collect()
    }
}

async fn record_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: NewModelLaneRun,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let payload = json!({
        "schema_id": "hsk.model_lane_run@1",
        "dexterity_kernel": "Dexterity",
        "record": input,
    });
    let event = model_lane_event(
        KernelEventType::SessionStarted,
        "model_lane_run",
        &input.run_id,
        &input.idempotency_key,
        input.work_packet_id.as_deref().unwrap_or(&input.run_id),
        &input.event_ledger_stream_id,
        payload,
    )?;

    lock_idempotency_key_tx(tx, &input.idempotency_key).await?;
    let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
    let record = ModelLaneRunRecord {
        event_ledger_event_id: stored_event.event_id.clone(),
        event_ledger_seq: stored_event.event_sequence,
        inner: input,
    };
    let inserted = sqlx::query(
        r#"
        INSERT INTO model_lane_runs (
            run_id, trace_id, run_span_id, coordinator_session_id,
            work_packet_id, micro_task_id, task_board_id, owner_session,
            idempotency_key, replay_order_key, event_ledger_stream_id,
            event_ledger_event_id, event_ledger_seq, record_json
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        ON CONFLICT (run_id) DO NOTHING
        RETURNING record_json
        "#,
    )
    .bind(&record.run_id)
    .bind(&record.trace_id)
    .bind(&record.run_span_id)
    .bind(&record.coordinator_session_id)
    .bind(record.work_packet_id.as_deref())
    .bind(record.micro_task_id.as_deref())
    .bind(record.task_board_id.as_deref())
    .bind(&record.owner_session)
    .bind(&record.idempotency_key)
    .bind(&record.replay_order_key)
    .bind(&record.event_ledger_stream_id)
    .bind(&record.event_ledger_event_id)
    .bind(record.event_ledger_seq)
    .bind(serde_json::to_value(&record)?)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(row) = inserted {
        return serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into);
    }

    let existing = run_by_id_tx(tx, &record.run_id).await?;
    if existing == record {
        Ok(existing)
    } else {
        Err(ModelLaneError::IdempotencyConflict(format!(
            "run_id {} already belongs to idempotency_key {}",
            record.run_id, existing.idempotency_key
        )))
    }
}

async fn record_lane_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: NewModelLane,
) -> ModelLaneResult<ModelLaneRecord> {
    let event_idempotency_key = format!("model-lane:{}:{}", input.run_id, input.lane_id);
    let payload = json!({
        "schema_id": "hsk.model_lane@1",
        "dexterity_kernel": "Dexterity",
        "record": input,
    });
    let event = model_lane_event(
        KernelEventType::ModelAdapterInvoked,
        "model_lane",
        &input.lane_id,
        &event_idempotency_key,
        input.work_packet_id.as_deref().unwrap_or(&input.run_id),
        &input.event_ledger_stream_id,
        payload,
    )?;

    lock_idempotency_key_tx(tx, &event_idempotency_key).await?;
    let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
    let record = ModelLaneRecord {
        event_ledger_event_id: stored_event.event_id.clone(),
        event_ledger_seq: stored_event.event_sequence,
        inner: input,
    };

    let inserted = sqlx::query(
        r#"
        INSERT INTO model_lanes (
            lane_id, run_id, trace_id, lane_span_id, kind,
            runtime_binding, launch_authority, status, work_packet_id,
            micro_task_id, task_board_id, owner_session, event_ledger_stream_id,
            event_ledger_event_id, event_ledger_seq, record_json
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
        ON CONFLICT (lane_id) DO NOTHING
        RETURNING record_json
        "#,
    )
    .bind(&record.lane_id)
    .bind(&record.run_id)
    .bind(&record.trace_id)
    .bind(&record.lane_span_id)
    .bind(record.kind.as_str())
    .bind(record.runtime_binding.as_str())
    .bind(record.launch_authority.as_str())
    .bind(record.status.as_str())
    .bind(record.work_packet_id.as_deref())
    .bind(record.micro_task_id.as_deref())
    .bind(record.task_board_id.as_deref())
    .bind(&record.owner_session)
    .bind(&record.event_ledger_stream_id)
    .bind(&record.event_ledger_event_id)
    .bind(record.event_ledger_seq)
    .bind(serde_json::to_value(&record)?)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(row) = inserted {
        return serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into);
    }

    let existing = lane_by_id_tx(tx, &record.lane_id).await?;
    if existing == record {
        Ok(existing)
    } else {
        Err(ModelLaneError::IdempotencyConflict(format!(
            "lane_id {} already belongs to run_id {}",
            record.lane_id, existing.run_id
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneKind {
    LocalModel,
    CloudModel,
    CliModel,
    HumanOperator,
    Subagent,
    Validator,
}

impl ModelLaneKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalModel => "local_model",
            Self::CloudModel => "cloud_model",
            Self::CliModel => "cli_model",
            Self::HumanOperator => "human_operator",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBinding {
    Local,
    Cloud,
    CliBridge,
    Human,
    Subagent,
    Validator,
}

impl RuntimeBinding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Cloud => "cloud",
            Self::CliBridge => "cli_bridge",
            Self::Human => "human",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchAuthority {
    ModelRuntime,
    CloudLane,
    CliBridge,
    Operator,
    SubagentManager,
    ValidatorRunner,
}

impl LaunchAuthority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModelRuntime => "model_runtime",
            Self::CloudLane => "cloud_lane",
            Self::CliBridge => "cli_bridge",
            Self::Operator => "operator",
            Self::SubagentManager => "subagent_manager",
            Self::ValidatorRunner => "validator_runner",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneProviderKind {
    OpenAi,
    Anthropic,
    LocalRuntime,
    OfficialCli,
    Human,
    Subagent,
    Validator,
    Other,
}

impl ModelLaneProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::LocalRuntime => "local_runtime",
            Self::OfficialCli => "official_cli",
            Self::Human => "human",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DexterityLaunchAdapterKind {
    LocalModelRuntime,
    ByokCloudOpenAi,
    ByokCloudAnthropic,
    OfficialCliBridge,
    CliBridge,
    HumanOperator,
    Subagent,
    Validator,
    DirectEndpoint,
    FrontendAppSrc,
    AppSrcTauri,
    TerminalOnly,
    ExternalCompat,
}

impl DexterityLaunchAdapterKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalModelRuntime => "local_model_runtime",
            Self::ByokCloudOpenAi => "byok_cloud_openai",
            Self::ByokCloudAnthropic => "byok_cloud_anthropic",
            Self::OfficialCliBridge => "official_cli_bridge",
            Self::CliBridge => "cli_bridge",
            Self::HumanOperator => "human_operator",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
            Self::DirectEndpoint => "direct_endpoint",
            Self::FrontendAppSrc => "frontend_app_src",
            Self::AppSrcTauri => "app_src_tauri",
            Self::TerminalOnly => "terminal_only",
            Self::ExternalCompat => "external_compat",
        }
    }

    fn is_bypass(&self) -> bool {
        matches!(
            self,
            Self::DirectEndpoint
                | Self::FrontendAppSrc
                | Self::AppSrcTauri
                | Self::TerminalOnly
                | Self::ExternalCompat
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityLaunchAdapterDescriptor {
    pub adapter_kind: DexterityLaunchAdapterKind,
    pub kind: ModelLaneKind,
    pub runtime_binding: RuntimeBinding,
    pub launch_authority: LaunchAuthority,
    pub provider_kind: ModelLaneProviderKind,
    pub default_backend: String,
    pub default_adapter_id: String,
    pub required_capability_tokens: Vec<String>,
    pub supported_tool_capability_tokens: Vec<String>,
    pub provider_feature_profile_ref: String,
    pub requested_execution_policy_ref: String,
    pub effective_execution_policy_ref: String,
    pub requires_projection_plan: bool,
    pub requires_consent_receipt: bool,
    pub requires_process_ownership: bool,
    pub no_os_process_reason_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DexterityLaunchAdapterRegistry {
    descriptors: BTreeMap<DexterityLaunchAdapterKind, DexterityLaunchAdapterDescriptor>,
}

impl DexterityLaunchAdapterRegistry {
    pub fn standard() -> Self {
        let descriptors = [
            descriptor(
                DexterityLaunchAdapterKind::LocalModelRuntime,
                ModelLaneKind::LocalModel,
                RuntimeBinding::Local,
                LaunchAuthority::ModelRuntime,
                ModelLaneProviderKind::LocalRuntime,
                "model_runtime",
                "model_runtime",
                ["capability://dexterity/local-generate"],
                ["tool-capability://read-context"],
                false,
                false,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::ByokCloudOpenAi,
                ModelLaneKind::CloudModel,
                RuntimeBinding::Cloud,
                LaunchAuthority::CloudLane,
                ModelLaneProviderKind::OpenAi,
                "cloud_lane_openai",
                "openai_byok",
                ["capability://dexterity/cloud-generate"],
                ["tool-capability://read-context"],
                true,
                true,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::ByokCloudAnthropic,
                ModelLaneKind::CloudModel,
                RuntimeBinding::Cloud,
                LaunchAuthority::CloudLane,
                ModelLaneProviderKind::Anthropic,
                "cloud_lane_anthropic",
                "anthropic_byok",
                ["capability://dexterity/cloud-generate"],
                ["tool-capability://read-context"],
                true,
                true,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::OfficialCliBridge,
                ModelLaneKind::CliModel,
                RuntimeBinding::CliBridge,
                LaunchAuthority::CliBridge,
                ModelLaneProviderKind::OfficialCli,
                "official_cli_bridge",
                "official_cli_bridge",
                ["capability://dexterity/cli-generate"],
                ["tool-capability://read-context"],
                false,
                false,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::CliBridge,
                ModelLaneKind::CliModel,
                RuntimeBinding::CliBridge,
                LaunchAuthority::CliBridge,
                ModelLaneProviderKind::OfficialCli,
                "cli_bridge",
                "cli_bridge",
                ["capability://dexterity/cli-bridge"],
                ["tool-capability://read-context"],
                false,
                false,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::HumanOperator,
                ModelLaneKind::HumanOperator,
                RuntimeBinding::Human,
                LaunchAuthority::Operator,
                ModelLaneProviderKind::Human,
                "operator_lane",
                "operator",
                ["capability://dexterity/operator-participant"],
                ["tool-capability://read-context"],
                false,
                false,
                false,
                Some("no-os-process://operator-lane".to_string()),
            ),
            descriptor(
                DexterityLaunchAdapterKind::Subagent,
                ModelLaneKind::Subagent,
                RuntimeBinding::Subagent,
                LaunchAuthority::SubagentManager,
                ModelLaneProviderKind::Subagent,
                "subagent_manager",
                "subagent_manager",
                ["capability://dexterity/subagent-participant"],
                ["tool-capability://read-context"],
                false,
                false,
                false,
                Some("no-os-process://subagent-manager-owned".to_string()),
            ),
            descriptor(
                DexterityLaunchAdapterKind::Validator,
                ModelLaneKind::Validator,
                RuntimeBinding::Validator,
                LaunchAuthority::ValidatorRunner,
                ModelLaneProviderKind::Validator,
                "validator_runner",
                "validator_runner",
                ["capability://dexterity/validator-participant"],
                ["tool-capability://read-context"],
                false,
                false,
                false,
                Some("no-os-process://validator-runner-owned".to_string()),
            ),
        ]
        .into_iter()
        .map(|entry| (entry.adapter_kind.clone(), entry))
        .collect();
        Self { descriptors }
    }

    pub fn descriptor(
        &self,
        kind: &DexterityLaunchAdapterKind,
    ) -> ModelLaneResult<&DexterityLaunchAdapterDescriptor> {
        if kind.is_bypass() {
            return Err(ModelLaneError::InvalidInput(format!(
                "Dexterity rejects {} launch bypass; launch authority must be Rust SwarmCoordinator, ModelRuntime, CloudLane, CLI bridge, operator, subagent, or validator runner",
                kind.as_str()
            )));
        }
        self.descriptors.get(kind).ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "Dexterity launch adapter {} is not registered",
                kind.as_str()
            ))
        })
    }

    pub fn adapter_kind_for_spawn_request(
        &self,
        request: &SpawnRequest,
    ) -> ModelLaneResult<DexterityLaunchAdapterKind> {
        match request.provider.unwrap_or(ProviderKind::Local) {
            ProviderKind::Local => Ok(DexterityLaunchAdapterKind::LocalModelRuntime),
            ProviderKind::ByokCloud => match request.byok_cloud_provider {
                Some(ByokCloudProvider::OpenAi) => Ok(DexterityLaunchAdapterKind::ByokCloudOpenAi),
                Some(ByokCloudProvider::Anthropic) => {
                    Ok(DexterityLaunchAdapterKind::ByokCloudAnthropic)
                }
                None => Err(ModelLaneError::InvalidInput(
                    "BYOK cloud Dexterity launch requires an explicit byok_cloud_provider".into(),
                )),
            },
            ProviderKind::OfficialCli => Ok(DexterityLaunchAdapterKind::OfficialCliBridge),
            ProviderKind::ExternalCompat => Err(ModelLaneError::InvalidInput(
                "Dexterity rejects external_compat launch bypass; use a registered Rust adapter"
                    .into(),
            )),
        }
    }

    pub fn preflight_spawn_request(
        &self,
        request: &SpawnRequest,
    ) -> ModelLaneResult<&DexterityLaunchAdapterDescriptor> {
        let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires SpawnRequest::dexterity_launch".into(),
            )
        })?;
        let adapter_kind = self.adapter_kind_for_spawn_request(request)?;
        let descriptor = self.descriptor(&adapter_kind)?;
        if contract.capability_token_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires capability_token_ids".into(),
            ));
        }
        contract.preflight_for_spawn_request(request, descriptor)?;
        require_token(
            "effective_capability_snapshot_ref",
            &contract.effective_capability_snapshot_ref,
        )?;
        if descriptor.requires_projection_plan {
            require_optional_token(
                "projection_plan_ref",
                contract.projection_plan_ref.as_deref(),
            )?;
        }
        if descriptor.requires_consent_receipt {
            require_optional_token(
                "consent_receipt_ref",
                contract.consent_receipt_ref.as_deref(),
            )?;
        }
        Ok(descriptor)
    }

    pub fn normalize(
        &self,
        mut request: DexterityLaunchAdapterRequest,
    ) -> ModelLaneResult<DexterityNormalizedLaunch> {
        let descriptor = self.descriptor(&request.adapter_kind)?.clone();
        for capability in &request.requested_tool_capability_tokens {
            if !descriptor
                .supported_tool_capability_tokens
                .contains(capability)
            {
                return Err(ModelLaneError::InvalidInput(format!(
                    "unsupported tool capability {capability} for Dexterity adapter {}",
                    request.adapter_kind.as_str()
                )));
            }
        }
        if descriptor.requires_projection_plan {
            require_optional_token(
                "projection_plan_ref",
                request.projection_plan_ref.as_deref(),
            )?;
        }
        if descriptor.requires_consent_receipt {
            require_optional_token(
                "consent_receipt_ref",
                request.consent_receipt_ref.as_deref(),
            )?;
        }
        let status = request.status.unwrap_or(ModelLaneStatus::Ready);
        request.heartbeat_at_utc = request
            .heartbeat_at_utc
            .or_else(|| Some(chrono::Utc::now().to_rfc3339()));
        request.cancellation_ref = request
            .cancellation_ref
            .or_else(|| Some(format!("cancel-token://{}", request.lane_id)));
        request.reclaim_policy_ref = request.reclaim_policy_ref.or_else(|| {
            Some(format!(
                "reclaim-policy://dexterity/{}",
                request.adapter_kind.as_str()
            ))
        });
        request.terminal_status_mapping_ref = request.terminal_status_mapping_ref.or_else(|| {
            Some(format!(
                "terminal-status://session-broker/{}",
                descriptor.runtime_binding.as_str()
            ))
        });
        request.capability_negotiation_ref = request.capability_negotiation_ref.or_else(|| {
            Some(format!(
                "capability-negotiation://dexterity/{}",
                request.lane_id
            ))
        });
        request.effective_capability_snapshot_ref =
            request.effective_capability_snapshot_ref.or_else(|| {
                Some(format!(
                    "capability-snapshot://dexterity/{}",
                    request.lane_id
                ))
            });
        if descriptor.requires_process_ownership {
            require_optional_token(
                "process_ownership_ref",
                request.process_ownership_ref.as_deref(),
            )?;
        } else {
            request.no_os_process_reason_ref =
                Some(descriptor.no_os_process_reason_ref.clone().ok_or_else(|| {
                    ModelLaneError::InvalidInput(format!(
                        "adapter {} requires no_os_process_reason_ref",
                        request.adapter_kind.as_str()
                    ))
                })?);
            request.process_ownership_ref = None;
        }
        let mut capability_token_ids = descriptor.required_capability_tokens.clone();
        capability_token_ids.extend(request.extra_capability_token_ids.iter().cloned());
        capability_token_ids.sort();
        capability_token_ids.dedup();
        if capability_token_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch requires at least one negotiated capability".into(),
            ));
        }
        let selected_model_id = request
            .selected_model_id
            .clone()
            .or_else(|| request.model_id.clone());
        let mut candidate_model_ids = request.candidate_model_ids.clone();
        if candidate_model_ids.is_empty() {
            if let Some(model_id) = selected_model_id.clone() {
                candidate_model_ids.push(model_id);
            } else {
                candidate_model_ids.push(format!(
                    "lane://{}:{}",
                    request.adapter_kind.as_str(),
                    request.lane_id
                ));
            }
        }
        Ok(DexterityNormalizedLaunch {
            adapter_kind: request.adapter_kind,
            run_id: request.run_id,
            lane_id: request.lane_id,
            trace_id: request.trace_id,
            run_span_id: request.run_span_id,
            lane_span_id: request.lane_span_id,
            coordinator_session_id: request.coordinator_session_id,
            routing_policy: request.routing_policy,
            context_bundle_id: request.context_bundle_id,
            event_ledger_stream_id: request.event_ledger_stream_id,
            artifact_namespace: request.artifact_namespace,
            work_packet_id: request.work_packet_id,
            micro_task_id: request.micro_task_id,
            task_board_id: request.task_board_id,
            owner_session: request.owner_session,
            locus_binding_ref: request.locus_binding_ref,
            role: request.role,
            backend: request.backend.unwrap_or(descriptor.default_backend),
            adapter_id: request.adapter_id.unwrap_or(descriptor.default_adapter_id),
            model_id: request.model_id,
            session_id: request.session_id,
            model_session_id: request.model_session_id,
            capability_token_ids,
            effective_capability_snapshot_ref: request.effective_capability_snapshot_ref,
            capability_negotiation_ref: request.capability_negotiation_ref,
            provider_feature_profile_ref: request
                .provider_feature_profile_ref
                .unwrap_or(descriptor.provider_feature_profile_ref),
            requested_execution_policy_ref: request
                .requested_execution_policy_ref
                .unwrap_or(descriptor.requested_execution_policy_ref),
            effective_execution_policy_ref: request
                .effective_execution_policy_ref
                .unwrap_or(descriptor.effective_execution_policy_ref),
            projection_plan_ref: request.projection_plan_ref,
            consent_receipt_ref: request.consent_receipt_ref,
            tool_gate_decision_refs: request.tool_gate_decision_refs,
            status,
            heartbeat_at_utc: request.heartbeat_at_utc,
            lease_expires_at_utc: request.lease_expires_at_utc,
            reclaim_after_utc: request.reclaim_after_utc,
            restart_generation: request.restart_generation,
            cancellation_ref: request.cancellation_ref,
            reclaim_policy_ref: request.reclaim_policy_ref,
            terminal_status_mapping_ref: request.terminal_status_mapping_ref,
            process_ownership_ref: request.process_ownership_ref,
            no_os_process_reason_ref: request.no_os_process_reason_ref,
            backpressure_ref: request.backpressure_ref,
            loop_counter_ref: request.loop_counter_ref,
            last_runtime_status_ref: request.last_runtime_status_ref,
            last_recovery_event_ref: request.last_recovery_event_ref,
            startup_failure_code: request.startup_failure_code,
            startup_failure_ref: request.startup_failure_ref,
            reason_ref: request.reason_ref,
            run_recovery_hint_ref: request.run_recovery_hint_ref,
            lane_recovery_hint_ref: request.lane_recovery_hint_ref,
            memory_pack_ref: request.memory_pack_ref,
            memory_pack_hash: request.memory_pack_hash,
            determinism_mode: request.determinism_mode,
            budget_summary_ref: request.budget_summary_ref,
            selected_model_id,
            candidate_model_ids,
            procedural_review_status: request.procedural_review_status,
            truncation_warning_ref: request.truncation_warning_ref,
            rejection_reason_refs: request.rejection_reason_refs,
        })
    }
}

fn descriptor(
    adapter_kind: DexterityLaunchAdapterKind,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
    provider_kind: ModelLaneProviderKind,
    default_backend: &str,
    default_adapter_id: &str,
    required_capability_tokens: impl IntoIterator<Item = &'static str>,
    supported_tool_capability_tokens: impl IntoIterator<Item = &'static str>,
    requires_projection_plan: bool,
    requires_consent_receipt: bool,
    requires_process_ownership: bool,
    no_os_process_reason_ref: Option<String>,
) -> DexterityLaunchAdapterDescriptor {
    DexterityLaunchAdapterDescriptor {
        provider_feature_profile_ref: format!(
            "provider-feature-profile://{}",
            provider_kind.as_str()
        ),
        requested_execution_policy_ref: format!(
            "execution-policy://requested/{}",
            runtime_binding.as_str()
        ),
        effective_execution_policy_ref: format!(
            "execution-policy://effective/{}",
            launch_authority.as_str()
        ),
        adapter_kind,
        kind,
        runtime_binding,
        launch_authority,
        provider_kind,
        default_backend: default_backend.to_string(),
        default_adapter_id: default_adapter_id.to_string(),
        required_capability_tokens: required_capability_tokens
            .into_iter()
            .map(str::to_string)
            .collect(),
        supported_tool_capability_tokens: supported_tool_capability_tokens
            .into_iter()
            .map(str::to_string)
            .collect(),
        requires_projection_plan,
        requires_consent_receipt,
        requires_process_ownership,
        no_os_process_reason_ref,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityLaunchAdapterRequest {
    pub adapter_kind: DexterityLaunchAdapterKind,
    pub run_id: String,
    pub lane_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub lane_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding_ref: String,
    pub role: String,
    pub backend: Option<String>,
    pub adapter_id: Option<String>,
    pub model_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub extra_capability_token_ids: Vec<String>,
    pub requested_tool_capability_tokens: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub status: Option<ModelLaneStatus>,
    pub heartbeat_at_utc: Option<String>,
    pub lease_expires_at_utc: Option<String>,
    pub reclaim_after_utc: Option<String>,
    pub restart_generation: i64,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub backpressure_ref: Option<String>,
    pub loop_counter_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub startup_failure_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub run_recovery_hint_ref: Option<String>,
    pub lane_recovery_hint_ref: Option<String>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityNormalizedLaunch {
    pub adapter_kind: DexterityLaunchAdapterKind,
    pub run_id: String,
    pub lane_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub lane_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding_ref: String,
    pub role: String,
    pub backend: String,
    pub adapter_id: String,
    pub model_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: String,
    pub requested_execution_policy_ref: String,
    pub effective_execution_policy_ref: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub status: ModelLaneStatus,
    pub heartbeat_at_utc: Option<String>,
    pub lease_expires_at_utc: Option<String>,
    pub reclaim_after_utc: Option<String>,
    pub restart_generation: i64,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub backpressure_ref: Option<String>,
    pub loop_counter_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub startup_failure_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub run_recovery_hint_ref: Option<String>,
    pub lane_recovery_hint_ref: Option<String>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
}

impl DexterityNormalizedLaunch {
    pub fn to_records(self) -> ModelLaneResult<(NewModelLaneRun, NewModelLane)> {
        let descriptor = DexterityLaunchAdapterRegistry::standard()
            .descriptor(&self.adapter_kind)?
            .clone();
        let locus = self.locus()?;
        let run = NewModelLaneRun {
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            run_span_id: self.run_span_id.clone(),
            coordinator_session_id: self.coordinator_session_id.clone(),
            routing_policy: self.routing_policy.clone(),
            context_bundle_id: self.context_bundle_id.clone(),
            lane_ids: vec![self.lane_id.clone()],
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            artifact_namespace: self.artifact_namespace.clone(),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            work_packet_id: self.work_packet_id.clone(),
            micro_task_id: self.micro_task_id.clone(),
            task_board_id: self.task_board_id.clone(),
            owner_session: self.owner_session.clone(),
            idempotency_key: format!("dexterity-normalized-launch-run:{}", self.run_id),
            replay_order_key: format!("{}:00000000:run", self.run_id),
            replay_after_event_ledger_seq: None,
            recovery_state: recovery_for_status(&self.status),
            failstate_code: self.startup_failure_code.clone(),
            reason_ref: self.reason_ref.clone(),
            recovery_hint_ref: self.run_recovery_hint_ref.clone(),
            locus_binding: Some(locus.clone()),
            memory_pack_ref: self.memory_pack_ref.clone(),
            memory_pack_hash: self.memory_pack_hash.clone(),
            determinism_mode: self.determinism_mode.clone(),
            budget_summary_ref: self.budget_summary_ref.clone(),
            selected_model_id: self.selected_model_id.clone(),
            candidate_model_ids: self.candidate_model_ids.clone(),
            procedural_review_status: self.procedural_review_status.clone(),
            truncation_warning_ref: self.truncation_warning_ref.clone(),
            rejection_reason_refs: self.rejection_reason_refs.clone(),
        };
        let lane = NewModelLane {
            lane_id: self.lane_id.clone(),
            run_id: self.run_id,
            trace_id: self.trace_id,
            lane_span_id: self.lane_span_id,
            event_ledger_stream_id: self.event_ledger_stream_id,
            kind: descriptor.kind,
            role: self.role,
            backend: self.backend,
            model_id: self.model_id,
            session_id: self.session_id,
            model_session_id: self.model_session_id,
            adapter_id: self.adapter_id,
            runtime_binding: descriptor.runtime_binding,
            launch_authority: descriptor.launch_authority,
            provider_kind: descriptor.provider_kind,
            capability_token_ids: self.capability_token_ids,
            effective_capability_snapshot_ref: self.effective_capability_snapshot_ref,
            capability_negotiation_ref: self.capability_negotiation_ref,
            provider_feature_profile_ref: Some(self.provider_feature_profile_ref),
            requested_execution_policy_ref: Some(self.requested_execution_policy_ref),
            effective_execution_policy_ref: Some(self.effective_execution_policy_ref),
            projection_plan_ref: self.projection_plan_ref,
            consent_receipt_ref: self.consent_receipt_ref,
            tool_gate_decision_refs: self.tool_gate_decision_refs,
            status: self.status.clone(),
            recovery_state: recovery_for_status(&self.status),
            heartbeat_at_utc: self.heartbeat_at_utc,
            lease_expires_at_utc: self.lease_expires_at_utc,
            reclaim_after_utc: self.reclaim_after_utc,
            restart_generation: self.restart_generation,
            cancellation_ref: self.cancellation_ref,
            reclaim_policy_ref: self.reclaim_policy_ref,
            terminal_status_mapping_ref: self.terminal_status_mapping_ref,
            process_ownership_ref: self.process_ownership_ref,
            no_os_process_reason_ref: self.no_os_process_reason_ref,
            backpressure_ref: self.backpressure_ref,
            loop_counter_ref: self.loop_counter_ref,
            last_runtime_status_ref: self.last_runtime_status_ref,
            last_recovery_event_ref: self.last_recovery_event_ref,
            failstate_code: self.startup_failure_code,
            startup_failure_ref: self.startup_failure_ref,
            reason_ref: self.reason_ref,
            recovery_hint_ref: self.lane_recovery_hint_ref,
            work_packet_id: self.work_packet_id,
            micro_task_id: self.micro_task_id,
            task_board_id: self.task_board_id,
            owner_session: self.owner_session,
            locus_binding: Some(locus),
        };
        Ok((run, lane))
    }

    fn locus(&self) -> ModelLaneResult<ModelLaneLocusBinding> {
        Ok(ModelLaneLocusBinding {
            work_packet_id: require_optional_token(
                "work_packet_id",
                self.work_packet_id.as_deref(),
            )?,
            micro_task_id: require_optional_token("micro_task_id", self.micro_task_id.as_deref())?,
            task_board_id: self.task_board_id.clone(),
            coordinator_session_id: self.coordinator_session_id.clone(),
            session_id: self.session_id.clone(),
            model_session_id: self.model_session_id.clone(),
            owner_session: self.owner_session.clone(),
            locus_binding_ref: self.locus_binding_ref.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneStatus {
    Planned,
    Ready,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
    Reclaimable,
}

impl ModelLaneStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Reclaimable => "reclaimable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryState {
    Restartable,
    Reclaimable,
    Terminal,
    Blocked,
}

impl ModelLaneRecoveryState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Restartable => "restartable",
            Self::Reclaimable => "reclaimable",
            Self::Terminal => "terminal",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryStatus {
    Observed,
    Checkpointed,
    Recovered,
    Failed,
}

impl ModelLaneRecoveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Checkpointed => "checkpointed",
            Self::Recovered => "recovered",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryEventKind {
    RunCreated,
    RunCompleted,
    RunFailed,
    LanePlanned,
    LaneStarted,
    LaneStatusChanged,
    LaneCompleted,
    LaneFailed,
    LaneCancelled,
    OrphanDetected,
    MessageRecorded,
    PayloadRefRecorded,
    PayloadRefMissing,
    RecoveryRequested,
    ReplayReconstructed,
    RecoveryFailed,
    CheckpointRestored,
    CrdtUpdateObserved,
    PayloadRefObserved,
    LeaseObserved,
    CloudConsentDenied,
    MtStatusRestored,
}

impl ModelLaneRecoveryEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RunCreated => "run_created",
            Self::RunCompleted => "run_completed",
            Self::RunFailed => "run_failed",
            Self::LanePlanned => "lane_planned",
            Self::LaneStarted => "lane_started",
            Self::LaneStatusChanged => "lane_status_changed",
            Self::LaneCompleted => "lane_completed",
            Self::LaneFailed => "lane_failed",
            Self::LaneCancelled => "lane_cancelled",
            Self::OrphanDetected => "orphan_detected",
            Self::MessageRecorded => "message_recorded",
            Self::PayloadRefRecorded => "payload_ref_recorded",
            Self::PayloadRefMissing => "payload_ref_missing",
            Self::RecoveryRequested => "recovery_requested",
            Self::ReplayReconstructed => "replay_reconstructed",
            Self::RecoveryFailed => "recovery_failed",
            Self::CheckpointRestored => "checkpoint_restored",
            Self::CrdtUpdateObserved => "crdt_update_observed",
            Self::PayloadRefObserved => "payload_ref_observed",
            Self::LeaseObserved => "lease_observed",
            Self::CloudConsentDenied => "cloud_consent_denied",
            Self::MtStatusRestored => "mt_status_restored",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryFailureKind {
    EventLedgerSequenceGap,
    MissingPayloadAuthority,
    StaleCrdtBase,
    CorruptCheckpoint,
    MissingCheckpoint,
    MissingEventLedgerRow,
    OrphanedSubagent,
    CancelledProcess,
    CrashedProcess,
    NeverStartedLane,
}

impl ModelLaneRecoveryFailureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EventLedgerSequenceGap => "event_ledger_sequence_gap",
            Self::MissingPayloadAuthority => "missing_payload_authority",
            Self::StaleCrdtBase => "stale_crdt_base",
            Self::CorruptCheckpoint => "corrupt_checkpoint",
            Self::MissingCheckpoint => "missing_checkpoint",
            Self::MissingEventLedgerRow => "missing_event_ledger_row",
            Self::OrphanedSubagent => "orphaned_subagent",
            Self::CancelledProcess => "cancelled_process",
            Self::CrashedProcess => "crashed_process",
            Self::NeverStartedLane => "never_started_lane",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::EventLedgerSequenceGap => "CX-MM-003",
            Self::MissingPayloadAuthority => "CX-MM-006",
            Self::StaleCrdtBase => "CX-MM-008",
            Self::CorruptCheckpoint => "CX-MM-009",
            Self::MissingCheckpoint => "CX-MM-010",
            Self::MissingEventLedgerRow => "CX-MM-011",
            Self::OrphanedSubagent => "CX-MM-009",
            Self::CancelledProcess => "CX-MM-012",
            Self::CrashedProcess => "CX-MM-013",
            Self::NeverStartedLane => "CX-MM-014",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneLeaseScope {
    Run,
    Lane,
}

impl ModelLaneLeaseScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Lane => "lane",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneLeaseState {
    Active,
    Released,
    Reclaimed,
    Cancelled,
}

impl ModelLaneLeaseState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Released => "released",
            Self::Reclaimed => "reclaimed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneDiagnosticTier {
    FlightRecorder,
    InternalDiagnostics,
    Palmistry,
}

impl ModelLaneDiagnosticTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FlightRecorder => "flight_recorder",
            Self::InternalDiagnostics => "internal_diagnostics",
            Self::Palmistry => "palmistry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneDiagnosticTierState {
    Wired,
    NotApplicableWithReason,
    DeferredWithReason,
    Missing,
}

impl ModelLaneDiagnosticTierState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wired => "wired",
            Self::NotApplicableWithReason => "not_applicable_with_reason",
            Self::DeferredWithReason => "deferred_with_reason",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneMtRuntimeStatus {
    Pending,
    Claimed,
    Blocked,
    ProofRunning,
    ReadyForValidation,
    Completed,
}

impl ModelLaneMtRuntimeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Blocked => "blocked",
            Self::ProofRunning => "proof_running",
            Self::ReadyForValidation => "ready_for_validation",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneMessageKind {
    Proposal,
    Critique,
    ToolRequest,
    ToolResult,
    Status,
    PromotionRequest,
    Recovery,
}

impl ModelLaneMessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposal => "proposal",
            Self::Critique => "critique",
            Self::ToolRequest => "tool_request",
            Self::ToolResult => "tool_result",
            Self::Status => "status",
            Self::PromotionRequest => "promotion_request",
            Self::Recovery => "recovery",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneAuthority {
    Advisory,
    PromotionCandidate,
    Promoted,
    OperatorDecision,
    ValidatorVerdict,
}

impl ModelLaneAuthority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::PromotionCandidate => "promotion_candidate",
            Self::Promoted => "promoted",
            Self::OperatorDecision => "operator_decision",
            Self::ValidatorVerdict => "validator_verdict",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingPolicy {
    LocalFirst,
    CloudReview,
    CloudPlanLocalExecute,
    ParallelDebate,
    ValidatorLane,
    OperatorLane,
}

impl ModelLaneRoutingPolicy {
    pub fn all() -> &'static [Self] {
        &[
            Self::LocalFirst,
            Self::CloudReview,
            Self::CloudPlanLocalExecute,
            Self::ParallelDebate,
            Self::ValidatorLane,
            Self::OperatorLane,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalFirst => "local_first",
            Self::CloudReview => "cloud_review",
            Self::CloudPlanLocalExecute => "cloud_plan_local_execute",
            Self::ParallelDebate => "parallel_debate",
            Self::ValidatorLane => "validator_lane",
            Self::OperatorLane => "operator_lane",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLanePromotionState {
    Advisory,
    PromotionRequested,
    PendingPolicy,
    PendingApproval,
    Approved,
    Denied,
    Expired,
    Executing,
    Executed,
    Skipped,
    Unsupported,
}

impl ModelLanePromotionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::PromotionRequested => "promotion_requested",
            Self::PendingPolicy => "pending_policy",
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
            Self::Executing => "executing",
            Self::Executed => "executed",
            Self::Skipped => "skipped",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLanePromotionOutcome {
    Approved,
    Denied,
}

impl ModelLanePromotionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLanePromotionDenialReason {
    StaleBase,
    StaleStateVector,
    SchemaMismatch,
    AggregateVersionMismatch,
    InputRefMismatch,
    DirectAuthorityMutation,
    MissingPromotionAuthority,
    MissingPromotedArtifactBinding,
}

impl ModelLanePromotionDenialReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StaleBase => "stale_base",
            Self::StaleStateVector => "stale_state_vector",
            Self::SchemaMismatch => "schema_mismatch",
            Self::AggregateVersionMismatch => "aggregate_version_mismatch",
            Self::InputRefMismatch => "input_ref_mismatch",
            Self::DirectAuthorityMutation => "direct_authority_mutation",
            Self::MissingPromotionAuthority => "missing_promotion_authority",
            Self::MissingPromotedArtifactBinding => "missing_promoted_artifact_binding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneHandoffSelectionState {
    Selected,
    Rejected,
    Unresolved,
    Superseded,
}

impl ModelLaneHandoffSelectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Rejected => "rejected",
            Self::Unresolved => "unresolved",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneHandoffSourceKind {
    Proposal,
    Critique,
    ToolRequest,
    ToolResult,
    Status,
    PromotionRequest,
    Recovery,
}

impl ModelLaneHandoffSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposal => "proposal",
            Self::Critique => "critique",
            Self::ToolRequest => "tool_request",
            Self::ToolResult => "tool_result",
            Self::Status => "status",
            Self::PromotionRequest => "promotion_request",
            Self::Recovery => "recovery",
        }
    }

    fn from_message_kind(kind: &ModelLaneMessageKind) -> Self {
        match kind {
            ModelLaneMessageKind::Proposal => Self::Proposal,
            ModelLaneMessageKind::Critique => Self::Critique,
            ModelLaneMessageKind::ToolRequest => Self::ToolRequest,
            ModelLaneMessageKind::ToolResult => Self::ToolResult,
            ModelLaneMessageKind::Status => Self::Status,
            ModelLaneMessageKind::PromotionRequest => Self::PromotionRequest,
            ModelLaneMessageKind::Recovery => Self::Recovery,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target_kind", content = "target_id", rename_all = "snake_case")]
pub enum ModelLaneTarget {
    Lane(String),
    Broadcast,
    Coordinator,
}

fn model_lane_target_label(target: &ModelLaneTarget) -> String {
    match target {
        ModelLaneTarget::Lane(lane_id) => format!("lane:{lane_id}"),
        ModelLaneTarget::Broadcast => "broadcast".to_owned(),
        ModelLaneTarget::Coordinator => "coordinator".to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneLocusBinding {
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: Option<String>,
    pub coordinator_session_id: String,
    pub session_id: String,
    pub model_session_id: String,
    pub owner_session: String,
    pub locus_binding_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingMetadata {
    pub target_role: String,
    pub target_session: String,
    pub correlation_id: String,
    pub requires_ack: bool,
    pub ack_for: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityLaunchContract {
    pub run_id: String,
    pub lane_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub lane_span_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub task_board_id: String,
    pub locus_binding_ref: String,
    pub role: String,
    pub backend: String,
    pub adapter_id: String,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
    pub run_recovery_hint_ref: Option<String>,
    pub lane_recovery_hint_ref: Option<String>,
}

impl DexterityLaunchContract {
    pub fn attach_to_spawn_request(
        mut request: SpawnRequest,
        work_packet_id: impl Into<String>,
        micro_task_id: impl Into<String>,
    ) -> ModelLaneResult<SpawnRequest> {
        request = request.with_wp(work_packet_id).with_mt(micro_task_id);
        let contract = Self::from_spawn_request(&request)?;
        Ok(request.with_dexterity_launch(contract))
    }

    pub fn from_spawn_request(request: &SpawnRequest) -> ModelLaneResult<Self> {
        required_request_field("wp_id", request.wp_id.as_deref())?;
        required_request_field("mt_id", request.mt_id.as_deref())?;
        let adapter_kind = dexterity_adapter_kind_for_spawn(request)?;
        let registry = DexterityLaunchAdapterRegistry::standard();
        let descriptor = registry.descriptor(&adapter_kind)?;
        let run_uuid = Uuid::now_v7();
        let lane_uuid = Uuid::now_v7();
        let run_id = format!("dexterity-run-{run_uuid}");
        let lane_id = format!(
            "dexterity-lane-{}-{lane_uuid}",
            descriptor.adapter_kind.as_str()
        );
        let trace_id = format!("trace-dexterity-{run_uuid}");
        let task_board_id = request
            .swarm_id
            .as_deref()
            .map(|swarm| format!("task-board://swarm-runtime/{swarm}"))
            .unwrap_or_else(|| "task-board://swarm-runtime/unassigned".to_string());
        let candidate_model_ids = dexterity_candidate_model_ids(request);
        let projection_plan_ref = descriptor
            .requires_projection_plan
            .then(|| format!("projection-plan://dexterity/{lane_id}"));
        let consent_receipt_ref = descriptor.requires_consent_receipt.then(|| {
            format!(
                "consent://dexterity/{}/{}",
                descriptor.provider_kind.as_str(),
                lane_id
            )
        });
        let memory_pack_ref = format!("memory-pack://dexterity/{run_id}");
        let memory_pack_hash = dexterity_sha256_hex(format!(
            "{}:{}:{}:{}",
            request.instance_id,
            request.parent_session_id,
            descriptor.adapter_kind.as_str(),
            request
                .model_artifact_sha256
                .as_deref()
                .or(request.cloud_model_name.as_deref())
                .unwrap_or("no-model-material")
        ));
        Ok(Self {
            run_id: run_id.clone(),
            lane_id: lane_id.clone(),
            trace_id,
            run_span_id: format!("span-{run_id}-run"),
            lane_span_id: format!("span-{lane_id}-lane"),
            routing_policy: format!("dexterity_{}", descriptor.runtime_binding.as_str()),
            context_bundle_id: format!("context-bundle://dexterity/{}", request.parent_session_id),
            event_ledger_stream_id: format!("event-ledger://dexterity/{run_id}"),
            artifact_namespace: format!("artifact://dexterity/{run_id}"),
            task_board_id,
            locus_binding_ref: format!(
                "locus://dexterity/{}/{}/{}",
                request.wp_id.as_deref().unwrap_or("unknown-wp"),
                request.mt_id.as_deref().unwrap_or("unknown-mt"),
                lane_id
            ),
            role: request.owner_role.clone(),
            backend: descriptor.default_backend.clone(),
            adapter_id: descriptor.default_adapter_id.clone(),
            capability_token_ids: descriptor.required_capability_tokens.clone(),
            effective_capability_snapshot_ref: format!("capability-snapshot://dexterity/{lane_id}"),
            projection_plan_ref,
            consent_receipt_ref,
            tool_gate_decision_refs: vec![format!("toolgate://dexterity/{lane_id}/read-context")],
            memory_pack_ref,
            memory_pack_hash,
            determinism_mode: "deterministic_replay".into(),
            budget_summary_ref: format!("budget://dexterity/{run_id}"),
            candidate_model_ids,
            procedural_review_status: "runtime_preflight".into(),
            truncation_warning_ref: None,
            rejection_reason_refs: vec!["rejection://dexterity/no-bypass-authority".into()],
            run_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#recovery".into()),
            lane_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#recovery".into()),
        })
    }

    fn preflight_for_spawn_request(
        &self,
        request: &SpawnRequest,
        descriptor: &DexterityLaunchAdapterDescriptor,
    ) -> ModelLaneResult<()> {
        required_request_field("wp_id", request.wp_id.as_deref())?;
        required_request_field("mt_id", request.mt_id.as_deref())?;
        require_token("parent_session_id", &request.parent_session_id)?;
        require_token("owner_role", &request.owner_role)?;
        require_token("run_id", &self.run_id)?;
        require_token("lane_id", &self.lane_id)?;
        require_token("trace_id", &self.trace_id)?;
        require_token("run_span_id", &self.run_span_id)?;
        require_token("lane_span_id", &self.lane_span_id)?;
        require_token("routing_policy", &self.routing_policy)?;
        require_token("context_bundle_id", &self.context_bundle_id)?;
        require_token("event_ledger_stream_id", &self.event_ledger_stream_id)?;
        require_token("artifact_namespace", &self.artifact_namespace)?;
        require_token("task_board_id", &self.task_board_id)?;
        require_token("locus_binding_ref", &self.locus_binding_ref)?;
        require_token("role", &self.role)?;
        require_token("backend", &self.backend)?;
        require_token("adapter_id", &self.adapter_id)?;
        require_token(
            "effective_capability_snapshot_ref",
            &self.effective_capability_snapshot_ref,
        )?;
        require_token("memory_pack_ref", &self.memory_pack_ref)?;
        validate_sha256("memory_pack_hash", &self.memory_pack_hash)?;
        require_token("determinism_mode", &self.determinism_mode)?;
        require_token("budget_summary_ref", &self.budget_summary_ref)?;
        require_token("procedural_review_status", &self.procedural_review_status)?;
        if self.capability_token_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires capability_token_ids".into(),
            ));
        }
        if self.tool_gate_decision_refs.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires tool_gate_decision_refs".into(),
            ));
        }
        if self.candidate_model_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires candidate_model_ids".into(),
            ));
        }
        for capability in &self.capability_token_ids {
            require_token("capability_token_ids[]", capability)?;
        }
        for decision_ref in &self.tool_gate_decision_refs {
            require_token("tool_gate_decision_refs[]", decision_ref)?;
        }
        if descriptor.requires_projection_plan {
            require_optional_token("projection_plan_ref", self.projection_plan_ref.as_deref())?;
        }
        if descriptor.requires_consent_receipt {
            require_optional_token("consent_receipt_ref", self.consent_receipt_ref.as_deref())?;
        }
        Ok(())
    }

    fn to_run(
        &self,
        request: &SpawnRequest,
        live: &LiveSession,
    ) -> ModelLaneResult<NewModelLaneRun> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let locus = self.locus(request, live)?;
        Ok(NewModelLaneRun {
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            run_span_id: self.run_span_id.clone(),
            coordinator_session_id: request.parent_session_id.clone(),
            routing_policy: self.routing_policy.clone(),
            context_bundle_id: self.context_bundle_id.clone(),
            lane_ids: vec![self.lane_id.clone()],
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            artifact_namespace: self.artifact_namespace.clone(),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            idempotency_key: format!("dexterity-launch-run:{}:{}", self.run_id, self.lane_id),
            replay_order_key: format!("{}:00000000:run", self.run_id),
            replay_after_event_ledger_seq: None,
            recovery_state: ModelLaneRecoveryState::Restartable,
            failstate_code: None,
            reason_ref: None,
            recovery_hint_ref: self.run_recovery_hint_ref.clone(),
            locus_binding: Some(locus),
            memory_pack_ref: self.memory_pack_ref.clone(),
            memory_pack_hash: self.memory_pack_hash.clone(),
            determinism_mode: self.determinism_mode.clone(),
            budget_summary_ref: self.budget_summary_ref.clone(),
            selected_model_id: Some(self.persisted_model_id(request, live)),
            candidate_model_ids: self.candidate_model_ids.clone(),
            procedural_review_status: self.procedural_review_status.clone(),
            truncation_warning_ref: self.truncation_warning_ref.clone(),
            rejection_reason_refs: self.rejection_reason_refs.clone(),
        })
    }

    fn to_failed_run(
        &self,
        request: &SpawnRequest,
        failure_code: &str,
        reason_ref: &str,
    ) -> ModelLaneResult<NewModelLaneRun> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let model_session_id = failed_model_session_id(request);
        let locus = self.failed_locus(request, &model_session_id)?;
        let candidate_model_ids = self.candidate_model_ids(request);
        Ok(NewModelLaneRun {
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            run_span_id: self.run_span_id.clone(),
            coordinator_session_id: request.parent_session_id.clone(),
            routing_policy: self.routing_policy.clone(),
            context_bundle_id: self.context_bundle_id.clone(),
            lane_ids: vec![self.lane_id.clone()],
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            artifact_namespace: self.artifact_namespace.clone(),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            idempotency_key: format!(
                "dexterity-launch-failed-run:{}:{}",
                self.run_id, self.lane_id
            ),
            replay_order_key: format!("{}:00000000:failed-run", self.run_id),
            replay_after_event_ledger_seq: None,
            recovery_state: ModelLaneRecoveryState::Reclaimable,
            failstate_code: Some(failure_code.to_string()),
            reason_ref: Some(reason_ref.to_string()),
            recovery_hint_ref: self.run_recovery_hint_ref.clone(),
            locus_binding: Some(locus),
            memory_pack_ref: self.memory_pack_ref.clone(),
            memory_pack_hash: self.memory_pack_hash.clone(),
            determinism_mode: self.determinism_mode.clone(),
            budget_summary_ref: self.budget_summary_ref.clone(),
            selected_model_id: Some(request.instance_id.model_id.to_string()),
            candidate_model_ids,
            procedural_review_status: self.procedural_review_status.clone(),
            truncation_warning_ref: self.truncation_warning_ref.clone(),
            rejection_reason_refs: self.rejection_reason_refs.clone(),
        })
    }

    fn to_lane(&self, request: &SpawnRequest, live: &LiveSession) -> ModelLaneResult<NewModelLane> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let mapped = map_spawn_provider(request)?;
        let heartbeat = chrono::Utc::now();
        let process_ownership_ref =
            format!("process-ledger://{}", live.process_record_id.as_uuid());
        let provider_feature_profile_ref = format!(
            "provider-feature-profile://{}",
            mapped.provider_kind.as_str()
        );
        let requested_execution_policy_ref = format!(
            "execution-policy://requested/{}",
            mapped.runtime_binding.as_str()
        );
        let effective_execution_policy_ref = format!(
            "execution-policy://effective/{}",
            mapped.launch_authority.as_str()
        );
        let terminal_status_mapping_ref = format!(
            "terminal-status://session-broker/{}",
            mapped.runtime_binding.as_str()
        );
        Ok(NewModelLane {
            lane_id: self.lane_id.clone(),
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            lane_span_id: self.lane_span_id.clone(),
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            kind: mapped.kind,
            role: self.role.clone(),
            backend: self.backend.clone(),
            model_id: Some(self.persisted_model_id(request, live)),
            session_id: runtime_session_id(request),
            model_session_id: dexterity_spawn_model_session_id(request),
            adapter_id: self.adapter_id.clone(),
            runtime_binding: mapped.runtime_binding,
            launch_authority: mapped.launch_authority,
            provider_kind: mapped.provider_kind,
            capability_token_ids: self.capability_token_ids.clone(),
            effective_capability_snapshot_ref: Some(self.effective_capability_snapshot_ref.clone()),
            capability_negotiation_ref: Some(format!(
                "capability-negotiation://{}",
                self.effective_capability_snapshot_ref
            )),
            provider_feature_profile_ref: Some(provider_feature_profile_ref),
            requested_execution_policy_ref: Some(requested_execution_policy_ref),
            effective_execution_policy_ref: Some(effective_execution_policy_ref),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            tool_gate_decision_refs: self.tool_gate_decision_refs.clone(),
            status: ModelLaneStatus::Ready,
            recovery_state: ModelLaneRecoveryState::Restartable,
            heartbeat_at_utc: Some(heartbeat.to_rfc3339()),
            lease_expires_at_utc: Some((heartbeat + chrono::Duration::minutes(5)).to_rfc3339()),
            reclaim_after_utc: Some((heartbeat + chrono::Duration::minutes(6)).to_rfc3339()),
            restart_generation: 0,
            cancellation_ref: Some(format!("cancel-token://{}", self.lane_id)),
            reclaim_policy_ref: Some("reclaim-policy://swarm-coordinator-lease".into()),
            terminal_status_mapping_ref: Some(terminal_status_mapping_ref),
            process_ownership_ref: Some(process_ownership_ref.clone()),
            no_os_process_reason_ref: None,
            backpressure_ref: None,
            loop_counter_ref: Some(format!("budget://{}", self.budget_summary_ref)),
            last_runtime_status_ref: Some(process_ownership_ref),
            last_recovery_event_ref: None,
            failstate_code: None,
            startup_failure_ref: None,
            reason_ref: None,
            recovery_hint_ref: self.lane_recovery_hint_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            locus_binding: Some(self.locus(request, live)?),
        })
    }

    fn to_failed_lane(
        &self,
        request: &SpawnRequest,
        failure_code: &str,
        startup_failure_ref: &str,
        reason_ref: &str,
    ) -> ModelLaneResult<NewModelLane> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let mapped = map_spawn_provider(request)?;
        let heartbeat = chrono::Utc::now();
        let model_session_id = failed_model_session_id(request);
        let runtime_binding = mapped.runtime_binding.clone();
        let launch_authority = mapped.launch_authority.clone();
        let provider_kind = mapped.provider_kind.clone();
        let terminal_status_mapping_ref = format!(
            "terminal-status://session-broker/{}",
            runtime_binding.as_str()
        );
        let provider_feature_profile_ref =
            format!("provider-feature-profile://{}", provider_kind.as_str());
        let requested_execution_policy_ref =
            format!("execution-policy://requested/{}", runtime_binding.as_str());
        let effective_execution_policy_ref =
            format!("execution-policy://effective/{}", launch_authority.as_str());
        Ok(NewModelLane {
            lane_id: self.lane_id.clone(),
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            lane_span_id: self.lane_span_id.clone(),
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            kind: mapped.kind,
            role: self.role.clone(),
            backend: self.backend.clone(),
            model_id: Some(request.instance_id.model_id.to_string()),
            session_id: runtime_session_id(request),
            model_session_id: model_session_id.clone(),
            adapter_id: self.adapter_id.clone(),
            runtime_binding,
            launch_authority,
            provider_kind,
            capability_token_ids: self.capability_token_ids.clone(),
            effective_capability_snapshot_ref: Some(self.effective_capability_snapshot_ref.clone()),
            capability_negotiation_ref: Some(format!(
                "capability-negotiation://{}",
                self.effective_capability_snapshot_ref
            )),
            provider_feature_profile_ref: Some(provider_feature_profile_ref),
            requested_execution_policy_ref: Some(requested_execution_policy_ref),
            effective_execution_policy_ref: Some(effective_execution_policy_ref),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            tool_gate_decision_refs: self.tool_gate_decision_refs.clone(),
            status: ModelLaneStatus::Failed,
            recovery_state: ModelLaneRecoveryState::Reclaimable,
            heartbeat_at_utc: Some(heartbeat.to_rfc3339()),
            lease_expires_at_utc: Some((heartbeat + chrono::Duration::minutes(5)).to_rfc3339()),
            reclaim_after_utc: Some((heartbeat + chrono::Duration::minutes(6)).to_rfc3339()),
            restart_generation: 0,
            cancellation_ref: Some(format!("cancel-token://{}", self.lane_id)),
            reclaim_policy_ref: Some("reclaim-policy://failed-startup".into()),
            terminal_status_mapping_ref: Some(terminal_status_mapping_ref),
            process_ownership_ref: None,
            no_os_process_reason_ref: Some(format!(
                "no-os-process://factory-create-failed/{}",
                self.lane_id
            )),
            backpressure_ref: None,
            loop_counter_ref: Some(format!("budget://{}", self.budget_summary_ref)),
            last_runtime_status_ref: Some(startup_failure_ref.to_string()),
            last_recovery_event_ref: None,
            failstate_code: Some(failure_code.to_string()),
            startup_failure_ref: Some(startup_failure_ref.to_string()),
            reason_ref: Some(reason_ref.to_string()),
            recovery_hint_ref: self.lane_recovery_hint_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            locus_binding: Some(self.failed_locus(request, &model_session_id)?),
        })
    }

    fn locus(
        &self,
        request: &SpawnRequest,
        _live: &LiveSession,
    ) -> ModelLaneResult<ModelLaneLocusBinding> {
        Ok(ModelLaneLocusBinding {
            work_packet_id: required_request_field("wp_id", request.wp_id.as_deref())?,
            micro_task_id: required_request_field("mt_id", request.mt_id.as_deref())?,
            task_board_id: Some(self.task_board_id.clone()),
            coordinator_session_id: request.parent_session_id.clone(),
            session_id: runtime_session_id(request),
            model_session_id: dexterity_spawn_model_session_id(request),
            owner_session: request.owner_role.clone(),
            locus_binding_ref: self.locus_binding_ref.clone(),
        })
    }

    fn failed_locus(
        &self,
        request: &SpawnRequest,
        model_session_id: &str,
    ) -> ModelLaneResult<ModelLaneLocusBinding> {
        Ok(ModelLaneLocusBinding {
            work_packet_id: required_request_field("wp_id", request.wp_id.as_deref())?,
            micro_task_id: required_request_field("mt_id", request.mt_id.as_deref())?,
            task_board_id: Some(self.task_board_id.clone()),
            coordinator_session_id: request.parent_session_id.clone(),
            session_id: runtime_session_id(request),
            model_session_id: model_session_id.to_string(),
            owner_session: request.owner_role.clone(),
            locus_binding_ref: self.locus_binding_ref.clone(),
        })
    }

    fn candidate_model_ids(&self, request: &SpawnRequest) -> Vec<String> {
        if self.candidate_model_ids.is_empty() {
            vec![request.instance_id.model_id.to_string()]
        } else {
            self.candidate_model_ids.clone()
        }
    }

    fn persisted_model_id(&self, request: &SpawnRequest, live: &LiveSession) -> String {
        if request.provider == Some(ProviderKind::ByokCloud) {
            return self
                .candidate_model_ids(request)
                .into_iter()
                .next()
                .unwrap_or_else(|| live.model_id.to_string());
        }
        live.model_id.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneRun {
    pub run_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub lane_ids: Vec<String>,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub replay_after_event_ledger_seq: Option<i64>,
    pub recovery_state: ModelLaneRecoveryState,
    pub failstate_code: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub locus_binding: Option<ModelLaneLocusBinding>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRunRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneRun,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

impl Deref for ModelLaneRunRecord {
    type Target = NewModelLaneRun;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLane {
    pub lane_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub lane_span_id: String,
    pub event_ledger_stream_id: String,
    pub kind: ModelLaneKind,
    pub role: String,
    pub backend: String,
    pub model_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub adapter_id: String,
    pub runtime_binding: RuntimeBinding,
    pub launch_authority: LaunchAuthority,
    pub provider_kind: ModelLaneProviderKind,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub status: ModelLaneStatus,
    pub recovery_state: ModelLaneRecoveryState,
    pub heartbeat_at_utc: Option<String>,
    pub lease_expires_at_utc: Option<String>,
    pub reclaim_after_utc: Option<String>,
    pub restart_generation: i64,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub backpressure_ref: Option<String>,
    pub loop_counter_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding: Option<ModelLaneLocusBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecord {
    #[serde(flatten)]
    pub inner: NewModelLane,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

impl Deref for ModelLaneRecord {
    type Target = NewModelLane;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneMessage {
    pub message_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub message_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub from_lane_id: String,
    pub to_lane: ModelLaneTarget,
    #[serde(default)]
    pub routing: Option<ModelLaneRoutingMetadata>,
    pub kind: ModelLaneMessageKind,
    pub payload_ref: String,
    pub payload_sha256: String,
    pub event_ledger_stream_id: String,
    pub summary: String,
    pub authority: ModelLaneAuthority,
    #[serde(default)]
    pub promotion_decision_id: Option<String>,
    pub promotion_gate_ref: Option<String>,
    pub promotion_receipt_ref: Option<String>,
    pub validator_verdict_ref: Option<String>,
    pub operator_decision_ref: Option<String>,
    pub promoted_artifact_ref: Option<String>,
    pub promoted_artifact_sha256: Option<String>,
    pub promoted_artifact_version: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub coordinator_session_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding: Option<ModelLaneLocusBinding>,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub replay_after_event_ledger_seq: Option<i64>,
    pub proposal_ref: Option<String>,
    pub crdt_update_ref: Option<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_proposal_ref: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneMessageRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneMessage,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneMessageRecord {
    type Target = NewModelLaneMessage;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneRecoveryCheckpoint {
    pub checkpoint_id: String,
    pub run_id: String,
    pub lane_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub lane_status: ModelLaneStatus,
    pub checkpoint_status: ModelLaneRecoveryStatus,
    pub last_event_ledger_seq: i64,
    pub last_message_id: Option<String>,
    pub open_payload_refs: Vec<String>,
    pub lease_id: Option<String>,
    pub idempotency_scope: String,
    pub recovery_state: ModelLaneRecoveryState,
    pub recovery_event_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub recovery_hint_ref: Option<String>,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecoveryCheckpointRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneRecoveryCheckpoint,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneRecoveryCheckpointRecord {
    type Target = NewModelLaneRecoveryCheckpoint;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneRecoveryEvent {
    pub recovery_event_id: String,
    pub run_id: String,
    pub lane_id: Option<String>,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub session_id: Option<String>,
    pub model_session_id: Option<String>,
    pub event_kind: ModelLaneRecoveryEventKind,
    pub recovery_status: ModelLaneRecoveryStatus,
    pub replay_order_seq: i64,
    pub source_event_ledger_seq: Option<i64>,
    pub payload_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub lease_id: Option<String>,
    pub failure_kind: Option<ModelLaneRecoveryFailureKind>,
    pub error_code: Option<String>,
    pub replay_hint: String,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub recovery_hint_ref: Option<String>,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecoveryEventRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneRecoveryEvent,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneRecoveryEventRecord {
    type Target = NewModelLaneRecoveryEvent;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneLease {
    pub lease_id: String,
    pub run_id: String,
    pub lane_id: Option<String>,
    pub scope: ModelLaneLeaseScope,
    pub scope_ref: String,
    pub holder_actor_id: String,
    pub holder_session_id: String,
    pub lease_expires_at_utc: String,
    pub takeover_policy_ref: String,
    pub state: ModelLaneLeaseState,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub recovery_hint_ref: Option<String>,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneLeaseRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneLease,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneLeaseRecord {
    type Target = NewModelLaneLease;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneDiagnosticTierStatus {
    pub diagnostic_status_id: String,
    pub behavior_id: String,
    pub run_id: String,
    pub tier: ModelLaneDiagnosticTier,
    pub state: ModelLaneDiagnosticTierState,
    pub reason: String,
    pub evidence_ref: String,
    pub follow_up_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticTierStatusRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneDiagnosticTierStatus,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneDiagnosticTierStatusRecord {
    type Target = NewModelLaneDiagnosticTierStatus;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticTierPosture {
    pub run_id: String,
    pub behavior_id: String,
    pub tiers: Vec<ModelLaneDiagnosticTierStatusRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneMtRuntimeStatus {
    pub mt_status_id: String,
    pub run_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub status: ModelLaneMtRuntimeStatus,
    pub claimed_by_ref: Option<String>,
    pub blocker_ref: Option<String>,
    pub missing_resource_ref: Option<String>,
    pub proof_status_ref: Option<String>,
    pub hbr_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneMtRuntimeStatusRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneMtRuntimeStatus,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneMtRuntimeStatusRecord {
    type Target = NewModelLaneMtRuntimeStatus;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudConsentDenialRecord {
    pub event_id: String,
    pub event_ledger_seq: i64,
    pub run_id: String,
    pub lane_id: String,
    pub reason_code: String,
    pub failure_kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecoveredRun {
    pub replay: ModelLaneReplay,
    pub checkpoint: ModelLaneRecoveryCheckpointRecord,
    pub recovery_events: Vec<ModelLaneRecoveryEventRecord>,
    pub active_leases: Vec<ModelLaneLeaseRecord>,
    pub reclaimable_lease_ids: Vec<String>,
    pub cloud_consent_denials: Vec<ModelLaneCloudConsentDenialRecord>,
    pub mt_runtime_statuses: Vec<ModelLaneMtRuntimeStatusRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsProjection {
    pub schema_id: String,
    pub surface_contract_id: String,
    pub run: ModelLaneDiagnosticsRun,
    pub lanes: Vec<ModelLaneDiagnosticsLane>,
    pub messages: Vec<ModelLaneDiagnosticsMessage>,
    pub diagnostic_tiers: Vec<ModelLaneDiagnosticsTier>,
    pub mt_runtime_statuses: Vec<ModelLaneDiagnosticsMtStatus>,
    pub active_lease_count: usize,
    pub reclaimable_lease_ids: Vec<String>,
    pub orphan_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsRun {
    pub run_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub artifact_namespace: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub context_bundle_id: String,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub status: String,
    pub recovery_hint_ref: Option<String>,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub budget_summary_ref: String,
    pub determinism_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsLane {
    pub lane_id: String,
    pub kind: String,
    pub role: String,
    pub backend: String,
    pub status: String,
    pub recovery_state: String,
    pub model_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub adapter_id: String,
    pub provider_kind: String,
    pub runtime_binding: String,
    pub launch_authority: String,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub trace_id: String,
    pub lane_span_id: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub last_activity_utc: Option<String>,
    pub message_count: usize,
    pub payload_error_count: usize,
    pub orphan_state: String,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsMessage {
    pub message_id: String,
    pub from_lane_id: String,
    pub to_lane: String,
    pub routing_target_role: Option<String>,
    pub routing_target_session: Option<String>,
    pub routing_correlation_id: Option<String>,
    pub routing_requires_ack: bool,
    pub routing_ack_for: Option<String>,
    pub kind: String,
    pub authority: String,
    pub promotion_state: String,
    pub payload_ref: String,
    pub payload_sha256: String,
    pub artifact_ref: Option<String>,
    pub promotion_decision_id: Option<String>,
    pub promotion_gate_ref: Option<String>,
    pub promotion_receipt_ref: Option<String>,
    pub validator_verdict_ref: Option<String>,
    pub operator_decision_ref: Option<String>,
    pub promoted_artifact_sha256: Option<String>,
    pub promoted_artifact_version: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub coordinator_session_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub trace_id: String,
    pub message_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub proposal_ref: Option<String>,
    pub crdt_update_ref: Option<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_proposal_ref: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub payload_error: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsTier {
    pub tier: String,
    pub state: String,
    pub reason: String,
    pub evidence_ref: String,
    pub follow_up_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsMtStatus {
    pub micro_task_id: String,
    pub status: String,
    pub proof_status_ref: Option<String>,
    pub hbr_status_ref: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneNavigationProjection {
    pub schema_id: String,
    pub surface_contract_id: String,
    pub route_id: String,
    pub lookup_kind: String,
    pub lookup_ref: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub manual_refs: Vec<String>,
    pub run: Option<ModelLaneRunRecord>,
    pub lanes: Vec<ModelLaneRecord>,
    pub messages: Vec<ModelLaneMessageRecord>,
    pub artifacts: Vec<ModelLaneContextBundleArtifactBindingRecord>,
    pub context_handoffs: Vec<ModelLaneContextBundleHandoffRecord>,
    pub recovery_checkpoints: Vec<ModelLaneRecoveryCheckpointRecord>,
    pub recovery_events: Vec<ModelLaneRecoveryEventRecord>,
    pub leases: Vec<ModelLaneLeaseRecord>,
    pub diagnostic_tiers: Vec<ModelLaneDiagnosticTierStatusRecord>,
    pub mt_runtime_statuses: Vec<ModelLaneMtRuntimeStatusRecord>,
    pub event_ledger_refs: Vec<String>,
    pub flight_recorder_refs: Vec<String>,
    pub error_codes: Vec<String>,
    pub recovery_routes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneNavigationLookup {
    pub lookup_kind: Option<String>,
    pub lookup_ref: Option<String>,
    pub run_id: Option<String>,
    pub lane_id: Option<String>,
    pub message_id: Option<String>,
    pub model_session_id: Option<String>,
    pub session_id: Option<String>,
    pub wp_id: Option<String>,
    pub work_packet_id: Option<String>,
    pub mt_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub artifact_ref: Option<String>,
    pub context_bundle_id: Option<String>,
    pub locus_ref: Option<String>,
    pub locus_binding_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub loom_block_id: Option<String>,
    pub fems_ref: Option<String>,
    pub memory_pack_ref: Option<String>,
    pub memory_pack_hash: Option<String>,
    pub event_ledger_event_id: Option<String>,
    pub event_ledger_seq: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub error_code: Option<String>,
}

impl ModelLaneNavigationLookup {
    fn requested(&self) -> ModelLaneResult<(String, String)> {
        let mut requested = Vec::new();
        if let (Some(kind), Some(value)) = (
            nonempty_lookup_value(self.lookup_kind.as_deref()),
            nonempty_lookup_value(self.lookup_ref.as_deref()),
        ) {
            requested.push((kind, value));
        }
        for (kind, value) in [
            ("run_id", self.run_id.as_deref()),
            ("lane_id", self.lane_id.as_deref()),
            ("message_id", self.message_id.as_deref()),
            ("model_session_id", self.model_session_id.as_deref()),
            ("session_id", self.session_id.as_deref()),
            ("wp_id", self.wp_id.as_deref()),
            ("work_packet_id", self.work_packet_id.as_deref()),
            ("mt_id", self.mt_id.as_deref()),
            ("micro_task_id", self.micro_task_id.as_deref()),
            ("task_board_id", self.task_board_id.as_deref()),
            ("artifact_ref", self.artifact_ref.as_deref()),
            ("context_bundle_id", self.context_bundle_id.as_deref()),
            ("locus_ref", self.locus_ref.as_deref()),
            ("locus_binding_ref", self.locus_binding_ref.as_deref()),
            ("loom_ref", self.loom_ref.as_deref()),
            ("loom_block_id", self.loom_block_id.as_deref()),
            ("fems_ref", self.fems_ref.as_deref()),
            ("memory_pack_ref", self.memory_pack_ref.as_deref()),
            ("memory_pack_hash", self.memory_pack_hash.as_deref()),
            (
                "event_ledger_event_id",
                self.event_ledger_event_id.as_deref(),
            ),
            ("event_ledger_seq", self.event_ledger_seq.as_deref()),
            ("trace_id", self.trace_id.as_deref()),
            ("span_id", self.span_id.as_deref()),
            ("error_code", self.error_code.as_deref()),
        ] {
            if let Some(value) = nonempty_lookup_value(value) {
                requested.push((kind.to_string(), value));
            }
        }
        match requested.len() {
            1 => Ok(requested.remove(0)),
            0 => Err(ModelLaneError::InvalidInput(
                "ModelLane navigation lookup requires exactly one selector".into(),
            )),
            _ => Err(ModelLaneError::InvalidInput(
                "ModelLane navigation lookup accepts exactly one selector".into(),
            )),
        }
    }
}

impl ModelLaneNavigationProjection {
    fn rebuild_navigation_evidence(&mut self) {
        let mut event_ledger_refs = BTreeSet::new();
        let mut flight_recorder_refs = BTreeSet::new();
        let mut error_codes = BTreeSet::new();

        if let Some(run) = &self.run {
            push_event_ref(&mut event_ledger_refs, &run.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, run.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, run.recovery_hint_ref.as_deref());
            push_optional_string(&mut flight_recorder_refs, Some(&run.memory_pack_ref));
            push_optional_string(&mut error_codes, run.failstate_code.as_deref());
        }
        for lane in &self.lanes {
            push_event_ref(&mut event_ledger_refs, &lane.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, lane.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                lane.process_ownership_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                lane.last_runtime_status_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                lane.last_recovery_event_ref.as_deref(),
            );
            push_optional_string(&mut flight_recorder_refs, lane.recovery_hint_ref.as_deref());
            push_optional_string(&mut error_codes, lane.failstate_code.as_deref());
        }
        for message in &self.messages {
            push_event_ref(&mut event_ledger_refs, &message.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, message.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&message.payload_ref));
            push_optional_string(
                &mut flight_recorder_refs,
                message.recovery_hint_ref.as_deref(),
            );
            push_optional_string(&mut flight_recorder_refs, message.proposal_ref.as_deref());
            push_optional_string(
                &mut flight_recorder_refs,
                message.crdt_update_ref.as_deref(),
            );
            push_optional_string(&mut error_codes, message.failstate_code.as_deref());
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "flight_recorder",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "internal_diagnostics",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "palmistry",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "locus_ref",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "loom_ref",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "fems_ref",
            );
        }
        for artifact in &self.artifacts {
            push_event_ref(&mut event_ledger_refs, &artifact.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, artifact.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&artifact.artifact_ref));
            push_optional_string(
                &mut flight_recorder_refs,
                Some(&artifact.artifact_manifest_ref),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                Some(&artifact.artifact_payload_ref),
            );
        }
        for handoff in &self.context_handoffs {
            push_event_ref(&mut event_ledger_refs, &handoff.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, handoff.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&handoff.context_bundle_id));
            push_optional_string(&mut flight_recorder_refs, Some(&handoff.artifact_ref));
            push_optional_json_string(
                &mut flight_recorder_refs,
                &handoff.diagnostic_payload,
                "flight_recorder",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &handoff.diagnostic_payload,
                "internal_diagnostics",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &handoff.diagnostic_payload,
                "palmistry",
            );
            push_optional_string(&mut error_codes, Some(&handoff.reason_code));
        }
        for checkpoint in &self.recovery_checkpoints {
            push_event_ref(&mut event_ledger_refs, &checkpoint.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, checkpoint.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                checkpoint.recovery_hint_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                checkpoint.recovery_event_ref.as_deref(),
            );
        }
        for event in &self.recovery_events {
            push_event_ref(&mut event_ledger_refs, &event.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, event.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                event.recovery_hint_ref.as_deref(),
            );
            push_optional_string(&mut error_codes, event.error_code.as_deref());
            push_optional_string(
                &mut error_codes,
                event
                    .failure_kind
                    .as_ref()
                    .map(ModelLaneRecoveryFailureKind::code),
            );
        }
        for lease in &self.leases {
            push_event_ref(&mut event_ledger_refs, &lease.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, lease.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                lease.recovery_hint_ref.as_deref(),
            );
        }
        for tier in &self.diagnostic_tiers {
            push_event_ref(&mut event_ledger_refs, &tier.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, tier.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&tier.evidence_ref));
            push_optional_string(&mut flight_recorder_refs, tier.follow_up_ref.as_deref());
        }
        for status in &self.mt_runtime_statuses {
            push_event_ref(&mut event_ledger_refs, &status.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, status.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                status.proof_status_ref.as_deref(),
            );
            push_optional_string(&mut flight_recorder_refs, status.hbr_status_ref.as_deref());
            push_optional_string(
                &mut flight_recorder_refs,
                status.last_recovery_event_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                status.last_runtime_status_ref.as_deref(),
            );
        }

        self.event_ledger_refs = event_ledger_refs.into_iter().collect();
        self.flight_recorder_refs = flight_recorder_refs.into_iter().collect();
        self.error_codes = error_codes.into_iter().collect();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudProjectionPlanStatus {
    Active,
    Superseded,
}

impl ModelLaneCloudProjectionPlanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudConsentReceiptStatus {
    Approved,
    Revoked,
}

impl ModelLaneCloudConsentReceiptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudConsentScope {
    SingleLane,
    SingleRun,
}

impl ModelLaneCloudConsentScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SingleLane => "single_lane",
            Self::SingleRun => "single_run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudRetentionPolicy {
    NoTrainingEphemeral,
    ProviderDefault,
}

impl ModelLaneCloudRetentionPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoTrainingEphemeral => "no_training_ephemeral",
            Self::ProviderDefault => "provider_default",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudExportPosture {
    RedactedContextOnly,
    NoExport,
}

impl ModelLaneCloudExportPosture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RedactedContextOnly => "redacted_context_only",
            Self::NoExport => "no_export",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneCloudProjectionPlan {
    pub projection_plan_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub lane_id: String,
    pub model_session_id: String,
    pub provider_kind: String,
    pub requested_model_id: String,
    pub scope_hash: String,
    pub source_artifact_refs: Vec<String>,
    pub payload_artifact_ref: String,
    pub payload_sha256: String,
    pub redaction_policy_ref: String,
    pub redaction_summary: String,
    pub retention_policy: ModelLaneCloudRetentionPolicy,
    pub export_posture: ModelLaneCloudExportPosture,
    pub provider_profile_ref: String,
    pub fan_out_targets: Vec<String>,
    pub consent_scope: ModelLaneCloudConsentScope,
    pub status: ModelLaneCloudProjectionPlanStatus,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub user_manual_behavior_ref: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudProjectionPlanRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneCloudProjectionPlan,
    pub projection_plan_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneCloudProjectionPlanRecord {
    type Target = NewModelLaneCloudProjectionPlan;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneCloudConsentReceipt {
    pub consent_receipt_id: String,
    pub projection_plan_id: String,
    pub projection_plan_hash: String,
    pub run_id: String,
    pub trace_id: String,
    pub lane_id: String,
    pub model_session_id: String,
    pub provider_kind: String,
    pub requested_model_id: String,
    pub scope_hash: String,
    pub consent_scope: ModelLaneCloudConsentScope,
    pub retention_policy: ModelLaneCloudRetentionPolicy,
    pub export_posture: ModelLaneCloudExportPosture,
    pub fan_out_targets: Vec<String>,
    pub approved: bool,
    pub approved_by_ref: String,
    pub approved_at_utc: String,
    pub valid_from_utc: String,
    pub valid_until_utc: String,
    pub revoked_at_utc: Option<String>,
    pub revocation_ref: Option<String>,
    pub status: ModelLaneCloudConsentReceiptStatus,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub user_manual_behavior_ref: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudConsentReceiptRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneCloudConsentReceipt,
    pub consent_receipt_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneCloudConsentReceiptRecord {
    type Target = NewModelLaneCloudConsentReceipt;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudConsentAuthorityReplay {
    pub projection_plans: Vec<ModelLaneCloudProjectionPlanRecord>,
    pub consent_receipts: Vec<ModelLaneCloudConsentReceiptRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCrdtHandoffMetadata {
    pub schema_id: String,
    pub document_id: String,
    pub workspace_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub lane_id: String,
    pub crdt_site_id: String,
    pub update_seq: i64,
    pub update_bytes_ref: String,
    pub update_sha256: String,
    pub state_vector: String,
    pub base_snapshot_ref: String,
    pub materialized_projection_hash: String,
    pub replay_metadata: Value,
    pub promotion_gate_ref: String,
    pub promotion_receipt_ref: Option<String>,
    pub validation_runner_ref: String,
    pub authority_effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneLoomHandoffRef {
    pub workspace_id: String,
    pub block_id: String,
    pub source_block_id: Option<String>,
    pub target_block_id: Option<String>,
    pub artifact_ref: Option<String>,
    pub content_hash: String,
    pub version: String,
    pub event_ledger_evidence_ref: String,
    pub flight_recorder_evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneMemoryPackHandoffRef {
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub scope_tag: String,
    pub review_status: String,
    pub cloud_safe: bool,
    pub classification: String,
    pub projection_ref: Option<String>,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneContextBundleArtifactBinding {
    pub artifact_binding_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub artifact_ref: String,
    pub artifact_sha256: String,
    pub content_hash: String,
    pub artifact_kind: String,
    pub artifact_manifest_ref: String,
    pub artifact_payload_ref: String,
    pub payload_json: Value,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneContextBundleArtifactBindingRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneContextBundleArtifactBinding,
    pub artifact_binding_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneContextBundleArtifactBindingRecord {
    type Target = NewModelLaneContextBundleArtifactBinding;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneContextBundleHandoff {
    pub handoff_id: String,
    pub context_bundle_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub handoff_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub downstream_lane_id: String,
    pub source_lane_id: String,
    pub source_message_id: String,
    pub artifact_ref: String,
    pub artifact_sha256: String,
    pub content_hash: String,
    pub source_kind: ModelLaneHandoffSourceKind,
    pub authority_state: ModelLaneAuthority,
    pub selection_state: ModelLaneHandoffSelectionState,
    pub reason_code: String,
    pub decision_ref: Option<String>,
    pub reviewer_ref: Option<String>,
    pub replay_hint: String,
    pub crdt_payload: Option<ModelLaneCrdtHandoffMetadata>,
    pub loom_refs: Vec<ModelLaneLoomHandoffRef>,
    pub memory_pack_refs: Vec<ModelLaneMemoryPackHandoffRef>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneContextBundleHandoffRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneContextBundleHandoff,
    pub context_bundle_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneContextBundleHandoffRecord {
    type Target = NewModelLaneContextBundleHandoff;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneDownstreamContextBundle {
    pub run_id: String,
    pub context_bundle_id: String,
    pub downstream_lane_id: String,
    pub context_hash: String,
    pub allowed_context: Value,
    pub records: Vec<ModelLaneContextBundleHandoffRecord>,
}

impl ModelLaneDownstreamContextBundle {
    pub fn to_kernel_context_bundle(&self) -> crate::kernel::KernelResult<ContextBundle> {
        ContextBundle::new(
            self.run_id.clone(),
            self.downstream_lane_id.clone(),
            self.allowed_context.clone(),
        )
    }
}

pub fn model_lane_context_bundle_id_for_handoff(
    input: &NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<String> {
    let hash = dexterity_sha256_hex(serde_json::to_vec(&context_bundle_identity_hash_basis(
        input,
    ))?);
    Ok(format!("CTX-{}", &hash[..16]))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLanePromotionDecision {
    pub decision_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub decision_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub coordinator_session_id: String,
    pub routing_policy: ModelLaneRoutingPolicy,
    pub input_refs: Vec<String>,
    pub selected_input_refs: Vec<String>,
    pub rejected_input_refs: Vec<String>,
    pub validator_authority_ref: Option<String>,
    pub operator_authority_ref: Option<String>,
    pub expected_event_ledger_aggregate_type: String,
    pub expected_event_ledger_aggregate_id: String,
    pub expected_event_ledger_version: i64,
    pub base_snapshot_ref: String,
    pub current_base_snapshot_ref: String,
    pub state_vector: String,
    pub current_state_vector: String,
    pub schema_id: String,
    pub deterministic_tie_break_rule: String,
    pub promotion_gate_ref: String,
    pub promotion_receipt_ref: Option<String>,
    #[serde(default)]
    pub promoted_artifact_ref: Option<String>,
    #[serde(default)]
    pub promoted_artifact_sha256: Option<String>,
    #[serde(default)]
    pub promoted_artifact_version: Option<String>,
    pub direct_authority_mutation_attempt_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLanePromotionDecisionRecord {
    #[serde(flatten)]
    pub inner: NewModelLanePromotionDecision,
    pub outcome: ModelLanePromotionOutcome,
    pub final_state: ModelLanePromotionState,
    pub denial_reason: Option<ModelLanePromotionDenialReason>,
    pub state_history: Vec<ModelLanePromotionState>,
    pub canonical_input_refs: Vec<String>,
    pub canonical_hash_basis: Value,
    pub canonical_decision_hash: String,
    pub current_event_ledger_version: Option<i64>,
    pub current_schema_id: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLanePromotionDecisionRecord {
    type Target = NewModelLanePromotionDecision;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneReplay {
    pub run: ModelLaneRunRecord,
    pub lanes: Vec<ModelLaneRecord>,
    pub messages: Vec<ModelLaneMessageRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneSchemaRegistryRow {
    pub schema_id: String,
    pub schema_version: i32,
    pub record_kind: String,
    pub table_name: String,
}

pub fn build_successful_launch_records(
    request: &SpawnRequest,
    live: &LiveSession,
) -> ModelLaneResult<(NewModelLaneRun, NewModelLane)> {
    DexterityLaunchAdapterRegistry::standard().preflight_spawn_request(request)?;
    let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
        ModelLaneError::InvalidInput(
            "Dexterity launch recording requires SpawnRequest::dexterity_launch".into(),
        )
    })?;
    Ok((
        contract.to_run(request, live)?,
        contract.to_lane(request, live)?,
    ))
}

pub fn build_failed_launch_records(
    request: &SpawnRequest,
    err: &SwarmError,
) -> ModelLaneResult<(NewModelLaneRun, NewModelLane)> {
    DexterityLaunchAdapterRegistry::standard().preflight_spawn_request(request)?;
    let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
        ModelLaneError::InvalidInput(
            "Dexterity failed launch recording requires SpawnRequest::dexterity_launch".into(),
        )
    })?;
    let failure_code = err.class().as_str();
    let reason_ref = format!(
        "reason://dexterity/{}/{}/{}",
        contract.run_id, contract.lane_id, failure_code
    );
    let startup_failure_ref = format!(
        "startup-failure://dexterity/{}/{}/{}",
        contract.run_id, contract.lane_id, failure_code
    );
    Ok((
        contract.to_failed_run(request, failure_code, &reason_ref)?,
        contract.to_failed_lane(request, failure_code, &startup_failure_ref, &reason_ref)?,
    ))
}

struct MappedSpawnProvider {
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
    provider_kind: ModelLaneProviderKind,
}

fn map_spawn_provider(request: &SpawnRequest) -> ModelLaneResult<MappedSpawnProvider> {
    match request.provider.unwrap_or(ProviderKind::Local) {
        ProviderKind::Local => Ok(MappedSpawnProvider {
            kind: ModelLaneKind::LocalModel,
            runtime_binding: RuntimeBinding::Local,
            launch_authority: LaunchAuthority::ModelRuntime,
            provider_kind: ModelLaneProviderKind::LocalRuntime,
        }),
        ProviderKind::ByokCloud => {
            let provider_kind = match request.byok_cloud_provider {
                Some(ByokCloudProvider::Anthropic) => ModelLaneProviderKind::Anthropic,
                Some(ByokCloudProvider::OpenAi) => ModelLaneProviderKind::OpenAi,
                None => {
                    return Err(ModelLaneError::InvalidInput(
                        "BYOK cloud Dexterity launch requires byok_cloud_provider".into(),
                    ));
                }
            };
            Ok(MappedSpawnProvider {
                kind: ModelLaneKind::CloudModel,
                runtime_binding: RuntimeBinding::Cloud,
                launch_authority: LaunchAuthority::CloudLane,
                provider_kind,
            })
        }
        ProviderKind::OfficialCli => Ok(MappedSpawnProvider {
            kind: ModelLaneKind::CliModel,
            runtime_binding: RuntimeBinding::CliBridge,
            launch_authority: LaunchAuthority::CliBridge,
            provider_kind: ModelLaneProviderKind::OfficialCli,
        }),
        ProviderKind::ExternalCompat => Err(ModelLaneError::InvalidInput(
            "Dexterity model-lane schema does not support external_compat provider".into(),
        )),
    }
}

fn dexterity_adapter_kind_for_spawn(
    request: &SpawnRequest,
) -> ModelLaneResult<DexterityLaunchAdapterKind> {
    match request.provider.unwrap_or(ProviderKind::Local) {
        ProviderKind::Local => Ok(DexterityLaunchAdapterKind::LocalModelRuntime),
        ProviderKind::ByokCloud => match request.byok_cloud_provider {
            Some(ByokCloudProvider::Anthropic) => {
                Ok(DexterityLaunchAdapterKind::ByokCloudAnthropic)
            }
            Some(ByokCloudProvider::OpenAi) => Ok(DexterityLaunchAdapterKind::ByokCloudOpenAi),
            None => Err(ModelLaneError::InvalidInput(
                "BYOK cloud Dexterity launch requires byok_cloud_provider".into(),
            )),
        },
        ProviderKind::OfficialCli => Ok(DexterityLaunchAdapterKind::OfficialCliBridge),
        ProviderKind::ExternalCompat => Err(ModelLaneError::InvalidInput(
            "Dexterity model-lane schema does not support external_compat provider".into(),
        )),
    }
}

fn dexterity_candidate_model_ids(request: &SpawnRequest) -> Vec<String> {
    if let Some(model_name) = request.cloud_model_name.as_deref() {
        return vec![format!(
            "model://dexterity/{}/{}",
            dexterity_provider_kind_label(request.provider.unwrap_or(ProviderKind::Local)),
            model_name
        )];
    }
    vec![request.instance_id.model_id.to_string()]
}

fn dexterity_provider_kind_label(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Local => "local",
        ProviderKind::ByokCloud => "byok_cloud",
        ProviderKind::OfficialCli => "official_cli",
        ProviderKind::ExternalCompat => "external_compat",
    }
}

fn dexterity_sha256_hex(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_ref());
    format!("{:x}", hasher.finalize())
}

pub fn dexterity_spawn_model_session_id(request: &SpawnRequest) -> String {
    format!("swarm-session:{}", request.instance_id)
}

fn runtime_session_id(request: &SpawnRequest) -> String {
    dexterity_spawn_model_session_id(request)
}

fn failed_model_session_id(request: &SpawnRequest) -> String {
    format!("failed-model-session:{}", request.instance_id)
}

fn required_request_field(field: &str, value: Option<&str>) -> ModelLaneResult<String> {
    let value = value.ok_or_else(|| {
        ModelLaneError::InvalidInput(format!("Dexterity launch requires SpawnRequest::{field}"))
    })?;
    require_token(field, value)?;
    Ok(value.to_string())
}

fn model_lane_event(
    event_type: KernelEventType,
    aggregate_type: &str,
    aggregate_id: &str,
    idempotency_key: &str,
    kernel_task_run_id: &str,
    session_run_id: &str,
    payload: Value,
) -> ModelLaneResult<NewKernelEvent> {
    NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        event_type,
        KernelActor::ModelAdapter("Dexterity".into()),
    )
    .aggregate(aggregate_type, aggregate_id)
    .idempotency_key(idempotency_key)
    .correlation_id(format!("dexterity:{kernel_task_run_id}:{session_run_id}"))
    .source_component(SOURCE_COMPONENT)
    .payload(payload)
    .build()
    .map_err(|err| ModelLaneError::InvalidInput(err.to_string()))
}

#[derive(Debug, Clone)]
struct CloudLaunchAuthorityCheck {
    run_id: String,
    lane_id: String,
    model_session_id: String,
    provider_kind: String,
    requested_model_id: String,
    projection_plan_ref: Option<String>,
    consent_receipt_ref: Option<String>,
    event_ledger_stream_id: String,
    work_packet_id: String,
    micro_task_id: Option<String>,
    owner_session: String,
    user_manual_behavior_ref: String,
}

impl CloudLaunchAuthorityCheck {
    fn from_contract(
        contract: &DexterityLaunchContract,
        provider_kind: &str,
        requested_model_id: &str,
        model_session_id: String,
    ) -> ModelLaneResult<Self> {
        require_token("run_id", &contract.run_id)?;
        require_token("lane_id", &contract.lane_id)?;
        require_token("event_ledger_stream_id", &contract.event_ledger_stream_id)?;
        Ok(Self {
            run_id: contract.run_id.clone(),
            lane_id: contract.lane_id.clone(),
            model_session_id,
            provider_kind: provider_kind.to_string(),
            requested_model_id: requested_model_id.to_string(),
            projection_plan_ref: contract.projection_plan_ref.clone(),
            consent_receipt_ref: contract.consent_receipt_ref.clone(),
            event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
            work_packet_id: contract.run_id.clone(),
            micro_task_id: None,
            owner_session: String::new(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
        })
    }
}

async fn ensure_cloud_launch_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    check: &CloudLaunchAuthorityCheck,
) -> ModelLaneResult<()> {
    require_token("cloud.run_id", &check.run_id)?;
    require_token("cloud.lane_id", &check.lane_id)?;
    require_token("cloud.model_session_id", &check.model_session_id)?;
    require_token("cloud.provider_kind", &check.provider_kind)?;
    require_token("cloud.requested_model_id", &check.requested_model_id)?;
    let projection_plan_id =
        require_optional_token("projection_plan_ref", check.projection_plan_ref.as_deref())?;
    let consent_receipt_id =
        require_optional_token("consent_receipt_ref", check.consent_receipt_ref.as_deref())?;
    let projection = cloud_projection_plan_by_id_tx(tx, &projection_plan_id)
        .await?
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "ProjectionPlan {projection_plan_id} is not durable"
            ))
        })?;
    let consent = cloud_consent_receipt_by_id_tx(tx, &consent_receipt_id)
        .await?
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "ConsentReceipt {consent_receipt_id} is not durable"
            ))
        })?;

    if projection.status != ModelLaneCloudProjectionPlanStatus::Active {
        return Err(ModelLaneError::InvalidInput(
            "ProjectionPlan is not active".into(),
        ));
    }
    require_equal(
        "ProjectionPlan.run_id",
        &projection.run_id,
        "lane.run_id",
        &check.run_id,
    )?;
    require_equal(
        "ProjectionPlan.lane_id",
        &projection.lane_id,
        "lane.lane_id",
        &check.lane_id,
    )?;
    require_equal(
        "ProjectionPlan.model_session_id",
        &projection.model_session_id,
        "lane.model_session_id",
        &check.model_session_id,
    )?;
    require_equal(
        "ProjectionPlan.provider_kind",
        &projection.provider_kind,
        "lane.provider_kind",
        &check.provider_kind,
    )?;
    require_equal(
        "ProjectionPlan.requested_model_id",
        &projection.requested_model_id,
        "lane.model_id",
        &check.requested_model_id,
    )?;
    if consent.status != ModelLaneCloudConsentReceiptStatus::Approved || !consent.approved {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt is not approved".into(),
        ));
    }
    if consent.revoked_at_utc.is_some() || consent.revocation_ref.is_some() {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt is revoked".into(),
        ));
    }
    require_equal(
        "ConsentReceipt.projection_plan_id",
        &consent.projection_plan_id,
        "ProjectionPlan.projection_plan_id",
        &projection.projection_plan_id,
    )?;
    require_equal(
        "ConsentReceipt.projection_plan_hash",
        &consent.projection_plan_hash,
        "ProjectionPlan.projection_plan_hash",
        &projection.projection_plan_hash,
    )?;
    require_equal(
        "ConsentReceipt.run_id",
        &consent.run_id,
        "lane.run_id",
        &check.run_id,
    )?;
    require_equal(
        "ConsentReceipt.lane_id",
        &consent.lane_id,
        "lane.lane_id",
        &check.lane_id,
    )?;
    require_equal(
        "ConsentReceipt.model_session_id",
        &consent.model_session_id,
        "lane.model_session_id",
        &check.model_session_id,
    )?;
    require_equal(
        "ConsentReceipt.provider_kind",
        &consent.provider_kind,
        "lane.provider_kind",
        &check.provider_kind,
    )?;
    require_equal(
        "ConsentReceipt.requested_model_id",
        &consent.requested_model_id,
        "lane.model_id",
        &check.requested_model_id,
    )?;
    require_equal(
        "ConsentReceipt.scope_hash",
        &consent.scope_hash,
        "ProjectionPlan.scope_hash",
        &projection.scope_hash,
    )?;
    if consent.consent_scope != projection.consent_scope
        || consent.retention_policy != projection.retention_policy
        || consent.export_posture != projection.export_posture
        || consent.fan_out_targets != projection.fan_out_targets
    {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt policy fields must match ProjectionPlan scope, retention, export, and fan-out"
                .into(),
        ));
    }
    let now = Utc::now();
    let valid_from = parse_utc("ConsentReceipt.valid_from_utc", &consent.valid_from_utc)?;
    let valid_until = parse_utc("ConsentReceipt.valid_until_utc", &consent.valid_until_utc)?;
    if now < valid_from || now > valid_until {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt validity window is not current".into(),
        ));
    }
    Ok(())
}

async fn record_cloud_consent_denial(
    pool: &PgPool,
    check: &CloudLaunchAuthorityCheck,
    failure_kind: &str,
    detail: &str,
) -> ModelLaneResult<()> {
    let mut tx = pool.begin().await?;
    let failure_kind_hash = dexterity_sha256_hex(failure_kind.as_bytes());
    let stable_basis = json!({
        "run_id": &check.run_id,
        "lane_id": &check.lane_id,
        "model_session_id": &check.model_session_id,
        "provider_kind": &check.provider_kind,
        "requested_model_id": &check.requested_model_id,
        "projection_plan_ref": &check.projection_plan_ref,
        "consent_receipt_ref": &check.consent_receipt_ref,
        "failure_kind_hash": &failure_kind_hash,
    });
    let idempotency_key = format!(
        "model-lane-cloud-consent-denial:{}:{}:{}",
        check.run_id,
        check.lane_id,
        dexterity_sha256_hex(canonical_json_bytes(&stable_basis))
    );
    let payload = json!({
        "schema_id": "hsk.model_lane_cloud_consent_denial@1",
        "dexterity_kernel": "Dexterity",
        "reason_code": "CX-MM-007",
        "consent_status": "CX-MM-007",
        "failure_kind": failure_kind,
        "failure_kind_hash": failure_kind_hash,
        "detail": detail,
        "run_id": &check.run_id,
        "lane_id": &check.lane_id,
        "model_session_id": &check.model_session_id,
        "provider_kind": &check.provider_kind,
        "requested_model_id": &check.requested_model_id,
        "projection_plan_ref": &check.projection_plan_ref,
        "consent_receipt_ref": &check.consent_receipt_ref,
        "provider_call_attempted": false,
        "partial_authority_state_created": false,
        "flight_recorder": "EventLedger",
        "internal_diagnostics": "deferred: backend event payload exposes denial; native diagnostic surface ships separately",
        "palmistry": "deferred: external watcher will link by run_id/lane_id when available",
        "user_manual_behavior_ref": &check.user_manual_behavior_ref,
        "micro_task_id": &check.micro_task_id,
        "owner_session": &check.owner_session,
    });
    let event = model_lane_event(
        KernelEventType::ValidationRecorded,
        "model_lane_cloud_consent_denial",
        &check.lane_id,
        &idempotency_key,
        &check.work_packet_id,
        &check.event_ledger_stream_id,
        payload,
    )?;
    append_kernel_event_with_executor(&mut *tx, event).await?;
    tx.commit().await?;
    Ok(())
}

async fn recovery_checkpoint_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneRecoveryCheckpointRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_recovery_checkpoints WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn recovery_event_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneRecoveryEventRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_recovery_events WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn lane_lease_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneLeaseRecord>> {
    sqlx::query("SELECT record_json FROM model_lane_leases WHERE idempotency_key = $1 LIMIT 1")
        .bind(idempotency_key)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()
}

async fn diagnostic_tier_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneDiagnosticTierStatusRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_diagnostic_tier_statuses WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn mt_runtime_status_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneMtRuntimeStatusRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_mt_runtime_statuses WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn canonical_run_for_recovery(
    pool: &PgPool,
    run_id: &str,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let run =
        select_record_by_column::<ModelLaneRunRecord>(pool, "model_lane_runs", "run_id", run_id)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))?;
    let ledger_session_run_id: String = sqlx::query_scalar(
        r#"
        SELECT session_run_id
        FROM kernel_event_ledger
        WHERE event_id = $1
          AND aggregate_type = 'model_lane_run'
          AND aggregate_id = $2
        "#,
    )
    .bind(&run.event_ledger_event_id)
    .bind(run_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ModelLaneError::InvalidInput(format!(
            "model_lane_run {run_id} has no canonical EventLedger session_run_id"
        ))
    })?;
    require_equal(
        "model_lane_run.session_run_id",
        &ledger_session_run_id,
        "record.event_ledger_stream_id",
        &run.event_ledger_stream_id,
    )?;
    Ok(run)
}

async fn latest_recovery_checkpoint(
    pool: &PgPool,
    run_id: &str,
    canonical_event_ledger_stream_id: &str,
) -> ModelLaneResult<ModelLaneRecoveryCheckpointRecord> {
    sqlx::query(
        r#"
        SELECT aggregate_id, session_run_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_recovery_checkpoint'
          AND payload->'record'->>'run_id' = $1
          AND session_run_id = $2
          AND payload->'record'->>'event_ledger_stream_id' = $2
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(run_id)
    .bind(canonical_event_ledger_stream_id)
    .fetch_optional(pool)
    .await?
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let session_run_id: String = row.try_get("session_run_id")?;
        let payload: Value = row.try_get("payload")?;
        let record: ModelLaneRecoveryCheckpointRecord =
            event_payload_record(&payload, "model_lane_recovery_checkpoint", &aggregate_id)?;
        require_equal(
            "checkpoint.session_run_id",
            &session_run_id,
            "checkpoint.record.event_ledger_stream_id",
            &record.event_ledger_stream_id,
        )?;
        // MT-003 unblock (out-of-scope, pre-existing WIP commit 0adac5d8): the
        // closure's error type is ambiguous (ModelLaneError has From<sqlx::Error>
        // + From<StorageError> + From<serde_json::Error>), so pin it to the
        // function's own ModelLaneResult error type. Compiler-suggested fix.
        Ok::<_, ModelLaneError>(record)
    })
    .transpose()?
    .ok_or_else(|| {
        let failure = ModelLaneRecoveryFailureKind::MissingCheckpoint;
        ModelLaneError::InvalidInput(format!(
            "{} {} no recovery checkpoint exists for run_id {run_id}",
            failure.code(),
            failure.as_str()
        ))
    })
}

/// Current committed high-watermark (max global EventLedger `event_sequence`) for a
/// ModelLaneRun stream. Used as the forward catch-up bound when the run advanced past
/// its last checkpoint (spec 4.3.9.2.5: "apply EventLedger records after that sequence
/// in order").
async fn recovery_stream_high_watermark(
    pool: &PgPool,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<i64> {
    let high_watermark: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(event_sequence), 0)
        FROM kernel_event_ledger
        WHERE session_run_id = $1
        "#,
    )
    .bind(event_ledger_stream_id)
    .fetch_one(pool)
    .await?;
    Ok(high_watermark)
}

/// True when the coordinator-owned ModelLaneMessage stream genuinely advanced past the
/// checkpoint (a NEW `model_lane_message` was committed after
/// `checkpoint_bound_event_ledger_seq`). Only real forward-message progress triggers
/// catch-up. Current-state adjunct writes recorded after a checkpoint with no new
/// message (post-checkpoint leases, MT status, cloud denials) are NOT forward progress
/// and stay excluded; this is what distinguishes a legitimate post-checkpoint catch-up
/// from stale adjunct state.
async fn has_post_checkpoint_forward_messages(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    checkpoint_bound_event_ledger_seq: i64,
) -> ModelLaneResult<bool> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM kernel_event_ledger
            WHERE aggregate_type = 'model_lane_message'
              AND session_run_id = $2
              AND payload->'record'->>'run_id' = $1
              AND event_sequence > $3
        )
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(checkpoint_bound_event_ledger_seq)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

async fn recovery_events_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<Vec<ModelLaneRecoveryEventRecord>> {
    sqlx::query(
        r#"
        SELECT aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_recovery_event'
          AND session_run_id = $2
          AND payload->'record'->>'run_id' = $1
          AND event_sequence <= $3
        ORDER BY (payload->'record'->>'replay_order_seq')::bigint ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        event_payload_record(&payload, "model_lane_recovery_event", &aggregate_id)
    })
    .collect()
}

async fn lane_leases_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<Vec<ModelLaneLeaseRecord>> {
    sqlx::query(
        r#"
        SELECT aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_lease'
          AND session_run_id = $2
          AND payload->'record'->>'run_id' = $1
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        event_payload_record(&payload, "model_lane_lease", &aggregate_id)
    })
    .collect()
}

async fn mt_runtime_statuses_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<Vec<ModelLaneMtRuntimeStatusRecord>> {
    sqlx::query(
        r#"
        SELECT aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_mt_runtime_status'
          AND session_run_id = $2
          AND payload->'record'->>'run_id' = $1
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        event_payload_record(&payload, "model_lane_mt_runtime_status", &aggregate_id)
    })
    .collect()
}

async fn cloud_consent_denials_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<Vec<ModelLaneCloudConsentDenialRecord>> {
    sqlx::query(
        r#"
        SELECT event_id, event_sequence, aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_cloud_consent_denial'
          AND session_run_id = $2
          AND payload->>'run_id' = $1
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let payload: Value = row.try_get("payload")?;
        require_json_string(
            &payload,
            "schema_id",
            "hsk.model_lane_cloud_consent_denial@1",
        )?;
        require_json_string(&payload, "reason_code", "CX-MM-007")?;
        if payload.get("provider_call_attempted").and_then(Value::as_bool) != Some(false) {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} cloud consent denial for run_id {run_id} must prove provider_call_attempted=false",
                failure.code(),
                failure.as_str()
            )));
        }
        let lane_id = required_json_text(&payload, "lane_id")?;
        let aggregate_id: String = row.try_get("aggregate_id")?;
        // Fail closed when a cloud-consent-denial ledger row's aggregate_id was
        // tampered off its own lane_id. Phrased as an "aggregate_id mismatch" for
        // parity with event_payload_record's aggregate-id integrity diagnosis.
        if aggregate_id != lane_id {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} model_lane_cloud_consent_denial aggregate_id mismatch: ledger aggregate_id {aggregate_id}, payload lane_id {lane_id}",
                failure.code(),
                failure.as_str()
            )));
        }
        let failure_kind = required_json_text(&payload, "failure_kind")?;
        Ok(ModelLaneCloudConsentDenialRecord {
            event_id: row.try_get("event_id")?,
            event_ledger_seq: row.try_get("event_sequence")?,
            run_id: run_id.to_string(),
            lane_id,
            reason_code: "CX-MM-007".into(),
            failure_kind,
        })
    })
    .collect()
}

async fn replay_run_at_recovery_bound(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<ModelLaneReplay> {
    let run_row = sqlx::query(
        r#"
        SELECT event_id, event_sequence, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_run'
          AND aggregate_id = $1
          AND session_run_id = $2
          AND event_sequence <= $3
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(run_id)
    .bind(&checkpoint.event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id} before checkpoint")))?;
    let run_event_id: String = run_row.try_get("event_id")?;
    let run_event_seq: i64 = run_row.try_get("event_sequence")?;
    let run_payload: Value = run_row.try_get("payload")?;
    let run_record = ModelLaneRunRecord {
        inner: event_payload_record(&run_payload, "model_lane_run", run_id)?,
        event_ledger_event_id: run_event_id,
        event_ledger_seq: run_event_seq,
    };

    let lanes = sqlx::query(
        r#"
        SELECT event_id, event_sequence, aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane'
          AND session_run_id = $1
          AND payload->'record'->>'run_id' = $2
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(&checkpoint.event_ledger_stream_id)
    .bind(run_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let event_id: String = row.try_get("event_id")?;
        let event_seq: i64 = row.try_get("event_sequence")?;
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        let lane: NewModelLane = event_payload_record(&payload, "model_lane", &aggregate_id)?;
        Ok(ModelLaneRecord {
            inner: lane,
            event_ledger_event_id: event_id,
            event_ledger_seq: event_seq,
        })
    })
    .collect::<ModelLaneResult<Vec<ModelLaneRecord>>>()?;

    let messages = sqlx::query(
        r#"
        SELECT event_id, event_sequence, aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_message'
          AND session_run_id = $1
          AND payload->'record'->>'run_id' = $2
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(&checkpoint.event_ledger_stream_id)
    .bind(run_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let event_id: String = row.try_get("event_id")?;
        let event_seq: i64 = row.try_get("event_sequence")?;
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        let message: NewModelLaneMessage =
            event_payload_record(&payload, "model_lane_message", &aggregate_id)?;
        Ok(ModelLaneMessageRecord {
            inner: message,
            event_ledger_event_id: event_id,
            event_ledger_seq: event_seq,
            event_stream_version: event_seq,
            transaction_seq: event_seq,
        })
    })
    .collect::<ModelLaneResult<Vec<ModelLaneMessageRecord>>>()?;

    if let Some(lane_id) = checkpoint.lane_id.as_deref() {
        let lane = lanes
            .iter()
            .find(|lane| lane.lane_id == lane_id)
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "CX-MM-009 checkpoint {} references missing lane {lane_id}",
                    checkpoint.checkpoint_id
                ))
            })?;
        require_equal(
            "checkpoint.session_id",
            &checkpoint.session_id,
            "lane.session_id",
            &lane.session_id,
        )?;
        require_equal(
            "checkpoint.model_session_id",
            &checkpoint.model_session_id,
            "lane.model_session_id",
            &lane.model_session_id,
        )?;
        require_equal(
            "checkpoint.lane_status",
            checkpoint.lane_status.as_str(),
            "lane.status",
            lane.status.as_str(),
        )?;
    }
    if let Some(last_message_id) = checkpoint.last_message_id.as_deref() {
        if !messages
            .iter()
            .any(|message| message.message_id == last_message_id)
        {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} checkpoint {} last_message_id {last_message_id} is not replayable before checkpoint",
                failure.code(),
                failure.as_str(),
                checkpoint.checkpoint_id
            )));
        }
    }

    Ok(ModelLaneReplay {
        run: run_record,
        lanes,
        messages,
    })
}

fn event_payload_record<T>(
    payload: &Value,
    aggregate_type: &str,
    aggregate_id: &str,
) -> ModelLaneResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let record = payload.get("record").ok_or_else(|| {
        ModelLaneError::InvalidInput(format!(
            "{aggregate_type} EventLedger payload missing record"
        ))
    })?;
    if !aggregate_id.is_empty() {
        let payload_id = match aggregate_type {
            "model_lane_run" => record.get("run_id"),
            "model_lane" => record.get("lane_id"),
            "model_lane_message" => record.get("message_id"),
            "model_lane_context_bundle_artifact" => record.get("artifact_binding_id"),
            "model_lane_recovery_checkpoint" => record.get("checkpoint_id"),
            "model_lane_recovery_event" => record.get("recovery_event_id"),
            "model_lane_lease" => record.get("lease_id"),
            "model_lane_diagnostic_tier" => record.get("diagnostic_status_id"),
            "model_lane_mt_runtime_status" => record.get("mt_status_id"),
            _ => None,
        }
        .and_then(Value::as_str)
        .unwrap_or_default();
        if payload_id != aggregate_id {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} EventLedger payload aggregate_id mismatch: payload record id {payload_id}, ledger aggregate_id {aggregate_id}"
            )));
        }
    }
    serde_json::from_value(record.clone()).map_err(Into::into)
}

async fn validate_diagnostics_row_eventledger_authority(
    pool: &PgPool,
    run_id: &str,
) -> ModelLaneResult<()> {
    validate_diagnostics_row_eventledger_authority_for::<ModelLaneRunRecord, NewModelLaneRun>(
        pool,
        run_id,
        "model_lane_run",
        "model_lane_runs",
        "run_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<ModelLaneRecord, NewModelLane>(
        pool,
        run_id,
        "model_lane",
        "model_lanes",
        "lane_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<
        ModelLaneMessageRecord,
        NewModelLaneMessage,
    >(
        pool,
        run_id,
        "model_lane_message",
        "model_lane_messages",
        "message_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<ModelLaneLeaseRecord, NewModelLaneLease>(
        pool,
        run_id,
        "model_lane_lease",
        "model_lane_leases",
        "lease_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<
        ModelLaneDiagnosticTierStatusRecord,
        NewModelLaneDiagnosticTierStatus,
    >(
        pool,
        run_id,
        "model_lane_diagnostic_tier",
        "model_lane_diagnostic_tier_statuses",
        "diagnostic_status_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<
        ModelLaneMtRuntimeStatusRecord,
        NewModelLaneMtRuntimeStatus,
    >(
        pool,
        run_id,
        "model_lane_mt_runtime_status",
        "model_lane_mt_runtime_statuses",
        "mt_status_id",
        "run_id",
    )
    .await
}

async fn validate_diagnostics_row_eventledger_authority_for<R, I>(
    pool: &PgPool,
    run_id: &str,
    aggregate_type: &'static str,
    table_name: &'static str,
    id_field: &'static str,
    run_field: &'static str,
) -> ModelLaneResult<()>
where
    R: for<'de> Deserialize<'de> + Deref<Target = I>,
    I: for<'de> Deserialize<'de> + PartialEq,
{
    let row_sequence_metadata = match table_name {
        "model_lane_messages"
        | "model_lane_leases"
        | "model_lane_diagnostic_tier_statuses"
        | "model_lane_mt_runtime_statuses" => {
            "rows.event_stream_version AS row_event_stream_version,
               rows.transaction_seq AS row_transaction_seq,"
        }
        _ => {
            "NULL::BIGINT AS row_event_stream_version,
               NULL::BIGINT AS row_transaction_seq,"
        }
    };
    let sql = format!(
        r#"
        SELECT rows.{id_field} AS row_id,
               rows.record_json AS record_json,
               rows.event_ledger_event_id AS row_event_ledger_event_id,
               rows.event_ledger_seq AS row_event_ledger_seq,
               {row_sequence_metadata}
               ledger.aggregate_id AS aggregate_id,
               ledger.event_id AS ledger_event_id,
               ledger.event_sequence AS ledger_event_sequence,
               ledger.payload AS payload
        FROM {table_name} rows
        LEFT JOIN kernel_event_ledger ledger
          ON ledger.event_id = rows.event_ledger_event_id
        WHERE rows.{run_field} = $1
        ORDER BY rows.event_ledger_seq ASC
        "#
    );
    for row in sqlx::query(&sql).bind(run_id).fetch_all(pool).await? {
        let sql_row_id: String = row.try_get("row_id")?;
        let record_json: Value = row.try_get("record_json")?;
        let row_event_ledger_event_id: String = row.try_get("row_event_ledger_event_id")?;
        let row_event_ledger_seq: i64 = row.try_get("row_event_ledger_seq")?;
        let row_event_stream_version: Option<i64> = row.try_get("row_event_stream_version")?;
        let row_transaction_seq: Option<i64> = row.try_get("row_transaction_seq")?;
        let aggregate_id: Option<String> = row.try_get("aggregate_id")?;
        let ledger_event_id: Option<String> = row.try_get("ledger_event_id")?;
        let ledger_event_sequence: Option<i64> = row.try_get("ledger_event_sequence")?;
        let payload: Option<Value> = row.try_get("payload")?;
        let (Some(aggregate_id), Some(ledger_event_id), Some(ledger_event_sequence), Some(payload)) = (
            aggregate_id,
            ledger_event_id,
            ledger_event_sequence,
            payload,
        ) else {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {sql_row_id} diagnostics projection row drift: row EventLedger columns do not resolve to kernel_event_ledger"
            )));
        };
        let ledger_record: I = event_payload_record(&payload, aggregate_type, &aggregate_id)?;
        let row_id = payload
            .get("record")
            .and_then(|record| record.get(id_field))
            .and_then(Value::as_str)
            .unwrap_or(aggregate_id.as_str());
        // Validate row IDENTITY against the ledger before deserializing/comparing the
        // mutable record body. A mutable row whose primary-key id was aliased onto
        // another valid ledger event is an identity tamper and MUST surface as the typed
        // "SQL row <id> does not match kernel_event_ledger" drift diagnosis -- not as a
        // raw deserialization error on the aliased (foreign-shaped) body. Identity is
        // logically prior to body equality: comparing bodies is meaningless once the row
        // points at the wrong ledger event. Per spec 4.3.9.2.5 recovery diagnostics MUST
        // be structured, not inferred from prose (a raw serde "missing field" is not).
        if sql_row_id != row_id || sql_row_id != aggregate_id {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {sql_row_id} diagnostics projection row drift: SQL row {id_field} does not match kernel_event_ledger aggregate/payload id {row_id}"
            )));
        }
        validate_record_json_eventledger_metadata(
            aggregate_type,
            row_id,
            &record_json,
            &row_event_ledger_event_id,
            row_event_ledger_seq,
            row_event_stream_version,
            row_transaction_seq,
            &ledger_event_id,
            ledger_event_sequence,
        )?;
        let row_record: R = serde_json::from_value(record_json.clone())?;
        if row_record.deref() != &ledger_record {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {row_id} diagnostics projection row drift: mutable row does not match kernel_event_ledger payload"
            )));
        }
    }
    Ok(())
}

fn validate_record_json_eventledger_metadata(
    aggregate_type: &str,
    row_id: &str,
    record_json: &Value,
    row_event_ledger_event_id: &str,
    row_event_ledger_seq: i64,
    row_event_stream_version: Option<i64>,
    row_transaction_seq: Option<i64>,
    ledger_event_id: &str,
    ledger_event_sequence: i64,
) -> ModelLaneResult<()> {
    if row_event_ledger_event_id != ledger_event_id || row_event_ledger_seq != ledger_event_sequence
    {
        return Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: row EventLedger columns do not match kernel_event_ledger"
        )));
    }
    if let Some(actual) = row_event_stream_version {
        if actual != ledger_event_sequence {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {row_id} diagnostics projection row drift: row event_stream_version does not match kernel_event_ledger"
            )));
        }
    }
    if let Some(actual) = row_transaction_seq {
        if actual != ledger_event_sequence {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {row_id} diagnostics projection row drift: row transaction_seq does not match kernel_event_ledger"
            )));
        }
    }
    let Some(record_event_id) = record_json
        .get("event_ledger_event_id")
        .and_then(Value::as_str)
    else {
        return Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json missing event_ledger_event_id"
        )));
    };
    if record_event_id != ledger_event_id {
        return Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json event_ledger_event_id does not match kernel_event_ledger"
        )));
    }
    validate_record_json_i64_metadata(
        aggregate_type,
        row_id,
        record_json,
        "event_ledger_seq",
        ledger_event_sequence,
    )?;
    validate_optional_record_json_i64_metadata(
        aggregate_type,
        row_id,
        record_json,
        "event_stream_version",
        ledger_event_sequence,
    )?;
    validate_optional_record_json_i64_metadata(
        aggregate_type,
        row_id,
        record_json,
        "transaction_seq",
        ledger_event_sequence,
    )
}

fn validate_record_json_i64_metadata(
    aggregate_type: &str,
    row_id: &str,
    record_json: &Value,
    field: &str,
    expected: i64,
) -> ModelLaneResult<()> {
    match record_json.get(field).and_then(Value::as_i64) {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json {field} does not match kernel_event_ledger"
        ))),
        None => Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json missing {field}"
        ))),
    }
}

fn validate_optional_record_json_i64_metadata(
    aggregate_type: &str,
    row_id: &str,
    record_json: &Value,
    field: &str,
    expected: i64,
) -> ModelLaneResult<()> {
    match record_json.get(field).and_then(Value::as_i64) {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json {field} does not match kernel_event_ledger"
        ))),
        None => Ok(()),
    }
}

async fn select_record_by_column<T>(
    pool: &PgPool,
    table_name: &'static str,
    column_name: &'static str,
    value: &str,
) -> ModelLaneResult<Option<T>>
where
    T: DeserializeOwned,
{
    let sql = format!(
        "SELECT record_json FROM {table_name} WHERE {column_name} = $1 ORDER BY event_ledger_seq ASC LIMIT 1"
    );
    sqlx::query(&sql)
        .bind(value)
        .fetch_optional(pool)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()
}

/// Look up a single record by a field that lives inside the `record_json` JSONB
/// payload rather than as a physical column. Several ModelLane navigation
/// identifiers (`context_bundle_id`, `model_session_id`, `session_id`,
/// `memory_pack_ref`, `failstate_code`, ...) are stored only in `record_json`;
/// querying them as physical columns raises a fail-closed "column does not
/// exist" database error that surfaces to callers as a 500. Resolving through
/// the JSONB text accessor keeps a valid query from ever 500-ing.
async fn select_record_by_json_field<T>(
    pool: &PgPool,
    table_name: &'static str,
    json_field: &'static str,
    value: &str,
) -> ModelLaneResult<Option<T>>
where
    T: DeserializeOwned,
{
    let sql = format!(
        "SELECT record_json FROM {table_name} WHERE record_json->>'{json_field}' = $1 ORDER BY event_ledger_seq ASC LIMIT 1"
    );
    sqlx::query(&sql)
        .bind(value)
        .fetch_optional(pool)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()
}

/// `run_id` companion to [`select_record_by_json_field`] for aggregate lookups
/// that resolve a set of run ids by a `record_json`-only field. `run_id` remains
/// a physical column on every ModelLane table, so only the WHERE predicate moves
/// into the JSONB payload.
async fn select_run_ids_by_json_field(
    pool: &PgPool,
    table_name: &'static str,
    json_field: &'static str,
    value: &str,
) -> ModelLaneResult<Vec<String>> {
    let sql = format!(
        "SELECT DISTINCT run_id FROM {table_name} WHERE record_json->>'{json_field}' = $1 ORDER BY run_id ASC"
    );
    sqlx::query_scalar::<_, String>(&sql)
        .bind(value)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

async fn select_run_ids_by_column(
    pool: &PgPool,
    table_name: &'static str,
    column_name: &'static str,
    value: &str,
) -> ModelLaneResult<Vec<String>> {
    let sql = format!(
        "SELECT DISTINCT run_id FROM {table_name} WHERE {column_name} = $1 ORDER BY run_id ASC"
    );
    sqlx::query_scalar::<_, String>(&sql)
        .bind(value)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

fn unique_run_id_for_lookup(
    lookup_kind: &str,
    lookup_ref: &str,
    run_ids: Vec<String>,
) -> ModelLaneResult<Option<String>> {
    let unique = run_ids.into_iter().collect::<BTreeSet<_>>();
    match unique.len() {
        0 => Ok(None),
        1 => Ok(unique.into_iter().next()),
        _ => {
            let candidates = unique.into_iter().collect::<Vec<_>>();
            Err(ModelLaneError::AmbiguousLookup(format!(
                "{lookup_kind} {lookup_ref} resolves to multiple runs: {}",
                candidates.join(", ")
            )))
        }
    }
}

async fn select_records_by_column<T>(
    pool: &PgPool,
    table_name: &'static str,
    column_name: &'static str,
    value: &str,
) -> ModelLaneResult<Vec<T>>
where
    T: DeserializeOwned,
{
    let sql = format!(
        "SELECT record_json FROM {table_name} WHERE {column_name} = $1 ORDER BY event_ledger_seq ASC"
    );
    sqlx::query(&sql)
        .bind(value)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .collect()
}

async fn select_records_by_any_artifact_ref(
    pool: &PgPool,
    value: &str,
) -> ModelLaneResult<Vec<ModelLaneContextBundleArtifactBindingRecord>> {
    sqlx::query(
        r#"
        SELECT record_json
        FROM model_lane_context_bundle_artifacts
        WHERE artifact_ref = $1
           OR artifact_payload_ref = $1
           OR artifact_manifest_ref = $1
           OR artifact_binding_id = $1
           OR artifact_sha256 = $1
           OR content_hash = $1
        ORDER BY event_ledger_seq ASC
        "#,
    )
    .bind(value)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .collect()
}

async fn select_records_by_any_handoff_artifact_ref(
    pool: &PgPool,
    value: &str,
) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
    sqlx::query(
        r#"
        SELECT record_json
        FROM model_lane_context_bundle_handoffs
        WHERE artifact_ref = $1
           OR artifact_sha256 = $1
           OR content_hash = $1
        ORDER BY event_ledger_seq ASC
        "#,
    )
    .bind(value)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .collect()
}

fn dedupe_context_handoffs(rows: &mut Vec<ModelLaneContextBundleHandoffRecord>) {
    let mut seen = BTreeSet::new();
    rows.retain(|row| seen.insert(row.handoff_id.clone()));
}

fn artifact_matches(row: &ModelLaneContextBundleArtifactBindingRecord, value: &str) -> bool {
    row.artifact_ref == value
        || row.artifact_binding_id == value
        || row.artifact_manifest_ref == value
        || row.artifact_payload_ref == value
        || row.artifact_sha256 == value
        || row.content_hash == value
}

fn message_mentions_lane(row: &ModelLaneMessageRecord, lane_id: &str) -> bool {
    row.from_lane_id == lane_id
        || matches!(&row.to_lane, ModelLaneTarget::Lane(target_lane_id) if target_lane_id == lane_id)
}

fn span_matches(span_id: Option<&str>, actual: &str) -> bool {
    span_id.map_or(true, |expected| expected == actual)
}

fn push_event_ref(refs: &mut BTreeSet<String>, event_id: &str) {
    if !event_id.is_empty() {
        refs.insert(format!("eventledger://kernel/{event_id}"));
    }
}

fn push_event_seq_ref(refs: &mut BTreeSet<String>, event_seq: i64) {
    if event_seq > 0 {
        refs.insert(format!("eventledger://kernel/seq/{event_seq}"));
    }
}

fn push_optional_string(refs: &mut BTreeSet<String>, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        refs.insert(value.to_owned());
    }
}

fn push_optional_json_string(refs: &mut BTreeSet<String>, payload: &Value, key: &str) {
    push_optional_string(refs, payload.get(key).and_then(Value::as_str));
}

fn nonempty_lookup_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

async fn query_optional_run_id(
    pool: &PgPool,
    sql: &str,
    value: &str,
) -> ModelLaneResult<Option<String>> {
    sqlx::query_scalar::<_, String>(sql)
        .bind(value)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

fn event_payload_run_id(payload: &Value) -> Option<String> {
    payload
        .get("record")
        .and_then(|record| record.get("run_id"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("run_id").and_then(Value::as_str))
        .map(str::to_owned)
}

async fn validate_recovery_event_stream(
    pool: &PgPool,
    run_id: &str,
    recovery_bound_event_ledger_seq: i64,
    events: &[ModelLaneRecoveryEventRecord],
) -> ModelLaneResult<()> {
    let mut expected = 1_i64;
    for event in events {
        if event.replay_order_seq != expected {
            let failure = ModelLaneRecoveryFailureKind::EventLedgerSequenceGap;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery replay gap for run_id {run_id}: expected replay_order_seq {expected}, got {}",
                failure.code(),
                failure.as_str(),
                event.replay_order_seq
            )));
        }
        expected += 1;
        if event.event_ledger_seq > recovery_bound_event_ledger_seq {
            let failure = ModelLaneRecoveryFailureKind::EventLedgerSequenceGap;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} is after recovery high-watermark {}",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id,
                recovery_bound_event_ledger_seq
            )));
        }
        let row = sqlx::query(
            r#"
            SELECT event_sequence, session_run_id, payload
            FROM kernel_event_ledger
            WHERE event_id = $1
              AND aggregate_type = 'model_lane_recovery_event'
              AND aggregate_id = $2
            "#,
        )
        .bind(&event.event_ledger_event_id)
        .bind(&event.recovery_event_id)
        .fetch_optional(pool)
        .await?;
        let Some(row) = row else {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} is not backed by matching kernel_event_ledger row",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        };
        let ledger_seq: i64 = row.try_get("event_sequence")?;
        let session_run_id: String = row.try_get("session_run_id")?;
        if ledger_seq != event.event_ledger_seq || session_run_id != event.event_ledger_stream_id {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} is not backed by matching kernel_event_ledger row",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        }
        let ledger_payload: Value = row.try_get("payload")?;
        let ledger_record: ModelLaneRecoveryEventRecord = event_payload_record(
            &ledger_payload,
            "model_lane_recovery_event",
            &event.recovery_event_id,
        )?;
        if &ledger_record != event {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} mutable row differs from EventLedger payload",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        }
        if let Some(source_event_ledger_seq) = event.source_event_ledger_seq {
            let source_stream: Option<String> = sqlx::query_scalar(
                "SELECT session_run_id FROM kernel_event_ledger WHERE event_sequence = $1",
            )
            .bind(source_event_ledger_seq)
            .fetch_optional(pool)
            .await?;
            if source_stream.as_deref() != Some(event.event_ledger_stream_id.as_str())
                || source_event_ledger_seq > event.event_ledger_seq
            {
                let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
                return Err(ModelLaneError::InvalidInput(format!(
                    "{} {} source_event_ledger_seq {source_event_ledger_seq} for recovery_event_id {} is missing, cross-stream, or after the recovery event",
                    failure.code(),
                    failure.as_str(),
                    event.recovery_event_id
                )));
            }
        }
    }
    Ok(())
}

async fn validate_recovery_payload_refs(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    checkpoint_bound_event_ledger_seq: i64,
    forward_bound_event_ledger_seq: i64,
    events: &[ModelLaneRecoveryEventRecord],
) -> ModelLaneResult<()> {
    // Payload refs that were OPEN at the checkpoint MUST have been satisfied by
    // ArtifactStore/EventLedger authority at/before the checkpoint. A post-checkpoint
    // artifact "repair" of such an already-checkpointed ref fails closed, so these
    // stay bounded at the checkpoint.
    let checkpoint_refs: BTreeSet<String> = checkpoint.open_payload_refs.iter().cloned().collect();
    validate_payload_authority_refs(
        pool,
        run_id,
        checkpoint,
        checkpoint_bound_event_ledger_seq,
        checkpoint_refs,
    )
    .await?;
    // Caught-up (post-checkpoint) recovery events reference NEW forward-stream payloads;
    // their authority is validated at the forward catch-up bound so genuine post-checkpoint
    // progress replays while checkpointed-ref repairs above still fail closed.
    let mut forward_refs = BTreeSet::new();
    for event in events {
        forward_refs.extend(event.payload_refs.iter().cloned());
    }
    validate_payload_authority_refs(
        pool,
        run_id,
        checkpoint,
        forward_bound_event_ledger_seq,
        forward_refs,
    )
    .await
}

async fn validate_replay_message_payload_authority(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    messages: &[ModelLaneMessageRecord],
) -> ModelLaneResult<()> {
    let mut refs = BTreeSet::new();
    let mut expected_hashes = BTreeMap::new();
    for message in messages {
        refs.insert(message.payload_ref.clone());
        if let Some(existing_hash) =
            expected_hashes.insert(message.payload_ref.clone(), message.payload_sha256.clone())
        {
            require_equal(
                "message.payload_sha256",
                &message.payload_sha256,
                "existing.payload_sha256",
                &existing_hash,
            )?;
        }
    }
    validate_payload_authority_refs(
        pool,
        run_id,
        checkpoint,
        recovery_bound_event_ledger_seq,
        refs,
    )
    .await?;
    validate_payload_authority_hashes(
        pool,
        run_id,
        checkpoint,
        recovery_bound_event_ledger_seq,
        expected_hashes,
    )
    .await
}

async fn validate_payload_authority_refs(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    refs: BTreeSet<String>,
) -> ModelLaneResult<()> {
    for payload_ref in refs {
        require_token("recovery.payload_ref", &payload_ref)?;
        let row = sqlx::query(
            r#"
            SELECT artifacts.record_json AS artifact_record_json,
                   ledger.aggregate_id AS ledger_aggregate_id,
                   ledger.payload AS ledger_payload
            FROM model_lane_context_bundle_artifacts artifacts
            JOIN kernel_event_ledger ledger
              ON ledger.event_id = artifacts.event_ledger_event_id
             AND ledger.event_sequence = artifacts.event_ledger_seq
             AND ledger.aggregate_type = 'model_lane_context_bundle_artifact'
            WHERE artifacts.run_id = $1
              AND (artifacts.artifact_ref = $2 OR artifacts.artifact_payload_ref = $2)
              AND artifacts.event_ledger_stream_id = $3
              AND artifacts.event_ledger_seq <= $4
              AND ledger.session_run_id = $3
            ORDER BY artifacts.event_ledger_seq DESC
            LIMIT 1
            "#,
        )
        .bind(run_id)
        .bind(&payload_ref)
        .bind(&checkpoint.event_ledger_stream_id)
        .bind(recovery_bound_event_ledger_seq)
        .fetch_optional(pool)
        .await?;
        let Some(row) = row else {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} is not backed by recovery-bounded ArtifactStore/EventLedger authority",
                failure.code(),
                failure.as_str()
            )));
        };
        let artifact_record_json: Value = row.try_get("artifact_record_json")?;
        let artifact_record: ModelLaneContextBundleArtifactBindingRecord =
            serde_json::from_value(artifact_record_json)?;
        let ledger_aggregate_id: String = row.try_get("ledger_aggregate_id")?;
        let ledger_payload: Value = row.try_get("ledger_payload")?;
        let ledger_record: ModelLaneContextBundleArtifactBindingRecord = event_payload_record(
            &ledger_payload,
            "model_lane_context_bundle_artifact",
            &ledger_aggregate_id,
        )?;
        require_equal(
            "artifact.sql_row_id",
            &artifact_record.artifact_binding_id,
            "ledger.aggregate_id",
            &ledger_aggregate_id,
        )?;
        if ledger_record != artifact_record {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} artifact row differs from EventLedger payload",
                failure.code(),
                failure.as_str()
            )));
        }
    }
    Ok(())
}

async fn validate_payload_authority_hashes(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    expected_hashes: BTreeMap<String, String>,
) -> ModelLaneResult<()> {
    for (payload_ref, payload_sha256) in expected_hashes {
        require_token("recovery.payload_ref", &payload_ref)?;
        validate_sha256("message.payload_sha256", &payload_sha256)?;
        let row = sqlx::query(
            r#"
            SELECT artifacts.record_json AS artifact_record_json
            FROM model_lane_context_bundle_artifacts artifacts
            WHERE artifacts.run_id = $1
              AND (artifacts.artifact_ref = $2 OR artifacts.artifact_payload_ref = $2)
              AND artifacts.event_ledger_stream_id = $3
              AND artifacts.event_ledger_seq <= $4
            ORDER BY artifacts.event_ledger_seq DESC
            LIMIT 1
            "#,
        )
        .bind(run_id)
        .bind(&payload_ref)
        .bind(&checkpoint.event_ledger_stream_id)
        .bind(recovery_bound_event_ledger_seq)
        .fetch_optional(pool)
        .await?;
        let Some(row) = row else {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} is not backed by recovery-bounded ArtifactStore authority",
                failure.code(),
                failure.as_str()
            )));
        };
        let artifact_record_json: Value = row.try_get("artifact_record_json")?;
        let artifact_record: ModelLaneContextBundleArtifactBindingRecord =
            serde_json::from_value(artifact_record_json)?;
        if artifact_record.content_hash != payload_sha256
            || artifact_record.artifact_sha256 != payload_sha256
        {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} hash mismatch: message payload_sha256 {} does not match ArtifactStore content_hash {} and artifact_sha256 {}",
                failure.code(),
                failure.as_str(),
                payload_sha256,
                artifact_record.content_hash,
                artifact_record.artifact_sha256
            )));
        }
    }
    Ok(())
}

fn validate_replay_message_crdt_posture(
    messages: &[ModelLaneMessageRecord],
) -> ModelLaneResult<()> {
    for message in messages {
        let has_crdt_ref = message.crdt_update_ref.is_some()
            || message.crdt_base_snapshot_ref.is_some()
            || message.crdt_state_vector.is_some()
            || message.crdt_proposal_ref.is_some()
            || message.crdt_stale_base_ref.is_some();
        if !has_crdt_ref {
            continue;
        }
        if message.crdt_stale_base_ref.is_some()
            || message.crdt_base_snapshot_ref.is_none()
            || message.crdt_state_vector.is_none()
        {
            let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} replayed message_id {} cannot be recovered against a stale or missing CRDT base",
                failure.code(),
                failure.as_str(),
                message.message_id
            )));
        }
    }
    Ok(())
}

async fn validate_recovery_crdt_posture(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    events: &[ModelLaneRecoveryEventRecord],
) -> ModelLaneResult<()> {
    for event in events {
        if event.crdt_stale_base_ref.is_some()
            || (event.event_kind == ModelLaneRecoveryEventKind::CrdtUpdateObserved
                && (event.crdt_base_snapshot_ref.is_none() || event.crdt_state_vector.is_none()))
        {
            let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} cannot be replayed against a stale or missing CRDT base",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        }
        if event.event_kind == ModelLaneRecoveryEventKind::CrdtUpdateObserved {
            let row = sqlx::query(
                r#"
                SELECT messages.record_json AS message_record_json,
                       ledger.payload AS ledger_payload
                FROM model_lane_messages messages
                JOIN kernel_event_ledger ledger
                  ON ledger.event_id = messages.event_ledger_event_id
                 AND ledger.event_sequence = messages.event_ledger_seq
                 AND ledger.aggregate_type = 'model_lane_message'
                WHERE messages.run_id = $1
                  AND ($2::text IS NULL OR messages.from_lane_id = $2)
                  AND messages.record_json->>'crdt_base_snapshot_ref' = $3
                  AND messages.record_json->>'crdt_state_vector' = $4
                  AND messages.record_json->>'crdt_stale_base_ref' IS NULL
                  AND messages.event_ledger_stream_id = $5
                  AND messages.event_ledger_seq <= $6
                  AND ledger.session_run_id = $5
                ORDER BY messages.event_ledger_seq DESC
                LIMIT 1
                "#,
            )
            .bind(run_id)
            .bind(event.lane_id.as_deref())
            .bind(event.crdt_base_snapshot_ref.as_deref())
            .bind(event.crdt_state_vector.as_deref())
            .bind(&checkpoint.event_ledger_stream_id)
            .bind(recovery_bound_event_ledger_seq)
            .fetch_optional(pool)
            .await?;
            let Some(row) = row else {
                let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
                return Err(ModelLaneError::InvalidInput(format!(
                    "{} {} recovery_event_id {} does not match any recovery-bounded non-stale ModelLaneMessage CRDT base/state vector",
                    failure.code(),
                    failure.as_str(),
                    event.recovery_event_id
                )));
            };
            let message_record_json: Value = row.try_get("message_record_json")?;
            let message_record: ModelLaneMessageRecord =
                serde_json::from_value(message_record_json)?;
            let ledger_payload: Value = row.try_get("ledger_payload")?;
            let ledger_message: NewModelLaneMessage = event_payload_record(
                &ledger_payload,
                "model_lane_message",
                &message_record.message_id,
            )?;
            if ledger_message != message_record.inner {
                let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
                return Err(ModelLaneError::InvalidInput(format!(
                    "{} {} recovery_event_id {} CRDT message row differs from EventLedger payload",
                    failure.code(),
                    failure.as_str(),
                    event.recovery_event_id
                )));
            }
        }
    }
    Ok(())
}

async fn validate_recovery_checkpoint_record(
    pool: &PgPool,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
) -> ModelLaneResult<()> {
    if checkpoint.last_event_ledger_seq <= 0 {
        let failure = ModelLaneRecoveryFailureKind::CorruptCheckpoint;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} has non-positive last_event_ledger_seq",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    if checkpoint.last_event_ledger_seq > checkpoint.event_ledger_seq {
        let failure = ModelLaneRecoveryFailureKind::CorruptCheckpoint;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} high-watermark is after its checkpoint event",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    let stream: Option<String> = sqlx::query_scalar(
        "SELECT session_run_id FROM kernel_event_ledger WHERE event_sequence = $1",
    )
    .bind(checkpoint.last_event_ledger_seq)
    .fetch_optional(pool)
    .await?;
    if stream.as_deref() != Some(checkpoint.event_ledger_stream_id.as_str()) {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} high-watermark {} is missing or cross-stream",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id,
            checkpoint.last_event_ledger_seq
        )));
    }
    let row = sqlx::query(
        r#"
        SELECT event_sequence, session_run_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
          AND aggregate_type = 'model_lane_recovery_checkpoint'
          AND aggregate_id = $2
        "#,
    )
    .bind(&checkpoint.event_ledger_event_id)
    .bind(&checkpoint.checkpoint_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} is not backed by matching kernel_event_ledger row",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    };
    let ledger_seq: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    if ledger_seq != checkpoint.event_ledger_seq
        || session_run_id != checkpoint.event_ledger_stream_id
    {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} is not backed by matching kernel_event_ledger row",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    let ledger_payload: Value = row.try_get("payload")?;
    let ledger_record: ModelLaneRecoveryCheckpointRecord = event_payload_record(
        &ledger_payload,
        "model_lane_recovery_checkpoint",
        &checkpoint.checkpoint_id,
    )?;
    if &ledger_record != checkpoint {
        let failure = ModelLaneRecoveryFailureKind::CorruptCheckpoint;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} mutable row differs from EventLedger payload",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    Ok(())
}

async fn run_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
) -> ModelLaneResult<ModelLaneRunRecord> {
    sqlx::query("SELECT record_json FROM model_lane_runs WHERE run_id = $1")
        .bind(run_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))
}

async fn lock_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn ensure_idempotent_input_matches<T>(
    entity: &str,
    idempotency_key: &str,
    existing: &T,
    input: &T,
) -> ModelLaneResult<()>
where
    T: Serialize + PartialEq,
{
    if existing == input {
        return Ok(());
    }
    let existing_hash =
        dexterity_sha256_hex(canonical_json_bytes(&serde_json::to_value(existing)?));
    let input_hash = dexterity_sha256_hex(canonical_json_bytes(&serde_json::to_value(input)?));
    Err(ModelLaneError::IdempotencyConflict(format!(
        "{entity} idempotency_key {idempotency_key} already belongs to semantic_hash {existing_hash}, retry supplied {input_hash}"
    )))
}

async fn ensure_event_ledger_sequence_in_stream_tx(
    tx: &mut Transaction<'_, Postgres>,
    event_ledger_seq: i64,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<()> {
    let row = sqlx::query(
        r#"
        SELECT session_run_id
        FROM kernel_event_ledger
        WHERE event_sequence = $1
        "#,
    )
    .bind(event_ledger_seq)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} event_ledger_seq {event_ledger_seq} does not exist",
            failure.code(),
            failure.as_str()
        )));
    };
    let session_run_id: String = row.try_get("session_run_id")?;
    require_equal(
        "event_ledger.session_run_id",
        &session_run_id,
        "record.event_ledger_stream_id",
        event_ledger_stream_id,
    )
}

async fn lane_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane_id: &str,
) -> ModelLaneResult<ModelLaneRecord> {
    sqlx::query("SELECT record_json FROM model_lanes WHERE lane_id = $1 FOR UPDATE")
        .bind(lane_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))
}

async fn lane_by_id_for_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    lane_id: &str,
) -> ModelLaneResult<ModelLaneRecord> {
    let lane = lane_by_id_tx(tx, lane_id).await?;
    require_equal("lane.run_id", &lane.run_id, "record.run_id", run_id)?;
    Ok(lane)
}

async fn cloud_projection_plan_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    projection_plan_id: &str,
) -> ModelLaneResult<Option<ModelLaneCloudProjectionPlanRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_cloud_projection_plans WHERE projection_plan_id = $1 FOR UPDATE",
    )
    .bind(projection_plan_id)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn cloud_projection_plan_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneCloudProjectionPlanRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_cloud_projection_plans WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn cloud_consent_receipt_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    consent_receipt_id: &str,
) -> ModelLaneResult<Option<ModelLaneCloudConsentReceiptRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_cloud_consent_receipts WHERE consent_receipt_id = $1 FOR UPDATE",
    )
    .bind(consent_receipt_id)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn cloud_consent_receipt_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneCloudConsentReceiptRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_cloud_consent_receipts WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn message_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneMessageRecord>> {
    sqlx::query("SELECT record_json FROM model_lane_messages WHERE idempotency_key = $1 LIMIT 1")
        .bind(idempotency_key)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()
}

async fn context_bundle_artifact_binding_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneContextBundleArtifactBindingRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_context_bundle_artifacts WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn context_bundle_artifact_binding_by_ref_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    artifact_ref: &str,
) -> ModelLaneResult<Option<ModelLaneContextBundleArtifactBindingRecord>> {
    sqlx::query(
        r#"
        SELECT record_json
        FROM model_lane_context_bundle_artifacts
        WHERE run_id = $1 AND artifact_ref = $2
        FOR UPDATE
        "#,
    )
    .bind(run_id)
    .bind(artifact_ref)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn promotion_decision_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLanePromotionDecisionRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_promotion_decisions WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn message_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    message_id: &str,
) -> ModelLaneResult<Option<ModelLaneMessageRecord>> {
    sqlx::query("SELECT record_json FROM model_lane_messages WHERE message_id = $1 FOR UPDATE")
        .bind(message_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()
}

async fn promotion_decision_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    decision_id: &str,
) -> ModelLaneResult<Option<ModelLanePromotionDecisionRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_promotion_decisions WHERE decision_id = $1 LIMIT 1",
    )
    .bind(decision_id)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn context_bundle_handoff_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneContextBundleHandoffRecord>> {
    sqlx::query(
        "SELECT record_json FROM model_lane_context_bundle_handoffs WHERE idempotency_key = $1 LIMIT 1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| {
        row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
    })
    .transpose()
}

async fn stamp_kernel_event_payload_tx(
    tx: &mut Transaction<'_, Postgres>,
    event_id: &str,
    payload: Value,
) -> ModelLaneResult<()> {
    let payload_hash = dexterity_sha256_hex(canonical_json_bytes(&payload));
    sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = $2,
            payload_hash = $3
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .bind(payload)
    .bind(payload_hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn ensure_promoted_message_has_decision_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: &NewModelLaneMessage,
) -> ModelLaneResult<()> {
    let promotion_decision_id =
        require_optional_token("promotion_decision_id", input.promotion_decision_id.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_decision_id is required"
                        .into(),
                )
            })?;
    let promotion_gate_ref =
        require_optional_token("promotion_gate_ref", input.promotion_gate_ref.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_gate_ref is required"
                        .into(),
                )
            })?;
    let promotion_receipt_ref =
        require_optional_token("promotion_receipt_ref", input.promotion_receipt_ref.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_receipt_ref is required"
                        .into(),
                )
            })?;
    let promoted_artifact_ref =
        require_optional_token("promoted_artifact_ref", input.promoted_artifact_ref.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_ref is required"
                        .into(),
                )
            })?;
    let promoted_artifact_sha256 = require_optional_token(
        "promoted_artifact_sha256",
        input.promoted_artifact_sha256.as_deref(),
    )
    .map_err(|_| {
        ModelLaneError::InvalidInput(
            "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_sha256 is required"
                .into(),
        )
    })?;
    let promoted_artifact_version = require_optional_token(
        "promoted_artifact_version",
        input.promoted_artifact_version.as_deref(),
    )
    .map_err(|_| {
        ModelLaneError::InvalidInput(
            "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_version is required"
                .into(),
        )
    })?;
    let decision = promotion_decision_by_id_tx(tx, &promotion_decision_id)
        .await?
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "Promoted ModelLaneMessage requires approved PromotionGate resolution for promotion_decision_id {promotion_decision_id}"
            ))
        })?;
    if decision.run_id == input.run_id
        && decision.outcome == ModelLanePromotionOutcome::Approved
        && decision.final_state == ModelLanePromotionState::Executed
        && decision.denial_reason.is_none()
        && decision.promotion_gate_ref == promotion_gate_ref
        && decision.promotion_receipt_ref.as_deref() == Some(promotion_receipt_ref.as_str())
        && decision.promoted_artifact_ref.as_deref() == Some(promoted_artifact_ref.as_str())
        && decision.promoted_artifact_sha256.as_deref() == Some(promoted_artifact_sha256.as_str())
        && decision.promoted_artifact_version.as_deref() == Some(promoted_artifact_version.as_str())
    {
        Ok(())
    } else {
        Err(ModelLaneError::InvalidInput(format!(
            "Promoted ModelLaneMessage requires exact approved PromotionGate resolution and artifact binding for promotion_decision_id {promotion_decision_id}"
        )))
    }
}

async fn prepare_context_bundle_handoff_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<ModelLaneContextBundleHandoffRecord> {
    run_by_id_tx(tx, &input.run_id).await?;
    let downstream_lane = lane_by_id_tx(tx, &input.downstream_lane_id).await?;
    require_equal(
        "handoff.run_id",
        &input.run_id,
        "downstream.run_id",
        &downstream_lane.run_id,
    )?;
    let source = message_by_id_tx(tx, &input.source_message_id)
        .await?
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "source_message_id {} is not replayable",
                input.source_message_id
            ))
        })?;
    require_equal(
        "handoff.run_id",
        &input.run_id,
        "source.run_id",
        &source.run_id,
    )?;
    require_equal(
        "handoff.source_lane_id",
        &input.source_lane_id,
        "source.from_lane_id",
        &source.from_lane_id,
    )?;
    require_equal(
        "handoff.artifact_ref",
        &input.artifact_ref,
        "source.payload_ref",
        &source.payload_ref,
    )?;
    require_equal(
        "handoff.artifact_sha256",
        &input.artifact_sha256,
        "source.payload_sha256",
        &source.payload_sha256,
    )?;
    require_equal(
        "handoff.content_hash",
        &input.content_hash,
        "source.payload_sha256",
        &source.payload_sha256,
    )?;
    let artifact_binding =
        context_bundle_artifact_binding_by_ref_tx(tx, &input.run_id, &input.artifact_ref)
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "artifact_ref {} is not backed by ArtifactStore/EventLedger authority",
                    input.artifact_ref
                ))
            })?;
    require_equal(
        "handoff.artifact_sha256",
        &input.artifact_sha256,
        "artifact_binding.artifact_sha256",
        &artifact_binding.artifact_sha256,
    )?;
    require_equal(
        "handoff.content_hash",
        &input.content_hash,
        "artifact_binding.content_hash",
        &artifact_binding.content_hash,
    )?;
    require_equal(
        "handoff.source_kind",
        input.source_kind.as_str(),
        "source.kind",
        ModelLaneHandoffSourceKind::from_message_kind(&source.kind).as_str(),
    )?;
    require_equal(
        "handoff.authority_state",
        input.authority_state.as_str(),
        "source.authority",
        source.authority.as_str(),
    )?;
    if source.crdt_proposal_ref.is_some() || source.crdt_update_ref.is_some() {
        let crdt = input.crdt_payload.as_ref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "CRDT ModelLaneMessage handoff requires crdt_payload metadata".into(),
            )
        })?;
        require_equal(
            "crdt_payload.lane_id",
            &crdt.lane_id,
            "handoff.source_lane_id",
            &input.source_lane_id,
        )?;
        if let Some(source_state_vector) = source.crdt_state_vector.as_deref() {
            require_equal(
                "crdt_payload.state_vector",
                &crdt.state_vector,
                "source.crdt_state_vector",
                source_state_vector,
            )?;
        }
        if let Some(source_base_snapshot_ref) = source.crdt_base_snapshot_ref.as_deref() {
            require_equal(
                "crdt_payload.base_snapshot_ref",
                &crdt.base_snapshot_ref,
                "source.crdt_base_snapshot_ref",
                source_base_snapshot_ref,
            )?;
        }
        if let Some(source_update_ref) = source.crdt_update_ref.as_deref() {
            require_equal(
                "crdt_payload.update_bytes_ref",
                &crdt.update_bytes_ref,
                "source.crdt_update_ref",
                source_update_ref,
            )?;
        }
    }
    let cloud_downstream = downstream_lane.runtime_binding == RuntimeBinding::Cloud
        || matches!(
            downstream_lane.provider_kind,
            ModelLaneProviderKind::OpenAi | ModelLaneProviderKind::Anthropic
        );
    if cloud_downstream && input.memory_pack_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "cloud downstream handoff requires explicit cloud_safe MemoryPack refs".into(),
        ));
    }
    if cloud_downstream
        && input
            .memory_pack_refs
            .iter()
            .any(|memory_pack| !memory_pack.cloud_safe)
    {
        return Err(ModelLaneError::InvalidInput(
            "cloud downstream handoff requires every MemoryPack ref to be cloud_safe".into(),
        ));
    }
    if cloud_downstream
        && input
            .memory_pack_refs
            .iter()
            .any(|memory_pack| memory_pack.classification == "local_only_context")
    {
        return Err(ModelLaneError::InvalidInput(
            "cloud downstream handoff cannot use local_only_context MemoryPack refs".into(),
        ));
    }
    let context_bundle_hash = context_bundle_handoff_hash(&input)?;
    Ok(ModelLaneContextBundleHandoffRecord {
        inner: input,
        context_bundle_hash,
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
        event_stream_version: 0,
        transaction_seq: 0,
    })
}

#[derive(Debug, Clone)]
struct PromotionInputResolution {
    denial_reason: Option<ModelLanePromotionDenialReason>,
    current_base_snapshot_ref: Option<String>,
    current_state_vector: Option<String>,
    selected_message_ids: Vec<String>,
}

async fn prepare_promotion_decision_tx(
    tx: &mut Transaction<'_, Postgres>,
    mut input: NewModelLanePromotionDecision,
) -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
    run_by_id_tx(tx, &input.run_id).await?;
    let canonical_input_refs = canonicalize_refs("input_refs", &input.input_refs)?;
    let selected_input_refs = canonicalize_refs("selected_input_refs", &input.selected_input_refs)?;
    let rejected_input_refs = canonicalize_refs("rejected_input_refs", &input.rejected_input_refs)?;
    if selected_input_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "selected_input_refs must contain at least one advisory input".into(),
        ));
    }
    require_refs_subset(
        "selected_input_refs",
        &selected_input_refs,
        &canonical_input_refs,
    )?;
    require_refs_subset(
        "rejected_input_refs",
        &rejected_input_refs,
        &canonical_input_refs,
    )?;
    require_refs_disjoint(
        "selected_input_refs",
        &selected_input_refs,
        "rejected_input_refs",
        &rejected_input_refs,
    )?;
    let resolution = resolve_promotion_input_refs_tx(
        tx,
        &input.run_id,
        &canonical_input_refs,
        &selected_input_refs,
    )
    .await?;
    input.input_refs = canonical_input_refs.clone();
    input.selected_input_refs = selected_input_refs;
    input.rejected_input_refs = rejected_input_refs;
    if let Some(current_base_snapshot_ref) = resolution.current_base_snapshot_ref.clone() {
        input.current_base_snapshot_ref = current_base_snapshot_ref;
    }
    if let Some(current_state_vector) = resolution.current_state_vector.clone() {
        input.current_state_vector = current_state_vector;
    }

    let current_event_ledger_version = latest_event_ledger_version_tx(
        tx,
        &input.expected_event_ledger_aggregate_type,
        &input.expected_event_ledger_aggregate_id,
    )
    .await?;
    let current_schema_id =
        current_schema_id_for_aggregate_tx(tx, &input.expected_event_ledger_aggregate_type).await?;
    let expected_aggregate_matches_selected = input.expected_event_ledger_aggregate_type
        == "model_lane_message"
        && resolution
            .selected_message_ids
            .iter()
            .any(|id| id == &input.expected_event_ledger_aggregate_id);
    let denial_reason = if let Some(reason) = resolution.denial_reason {
        Some(reason)
    } else if !expected_aggregate_matches_selected {
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
    } else if current_event_ledger_version != Some(input.expected_event_ledger_version) {
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
    } else if current_schema_id.as_deref() != Some(input.schema_id.as_str()) {
        Some(ModelLanePromotionDenialReason::SchemaMismatch)
    } else if input.base_snapshot_ref != input.current_base_snapshot_ref {
        Some(ModelLanePromotionDenialReason::StaleBase)
    } else if input.state_vector != input.current_state_vector {
        Some(ModelLanePromotionDenialReason::StaleStateVector)
    } else if input.direct_authority_mutation_attempt_ref.is_some() {
        Some(ModelLanePromotionDenialReason::DirectAuthorityMutation)
    } else if input.validator_authority_ref.is_none() && input.operator_authority_ref.is_none() {
        Some(ModelLanePromotionDenialReason::MissingPromotionAuthority)
    } else if missing_promoted_artifact_binding(&input) {
        Some(ModelLanePromotionDenialReason::MissingPromotedArtifactBinding)
    } else {
        None
    };
    let outcome = if denial_reason.is_some() {
        ModelLanePromotionOutcome::Denied
    } else {
        ModelLanePromotionOutcome::Approved
    };
    let state_history = promotion_state_history(outcome);
    let final_state = *state_history
        .last()
        .ok_or_else(|| ModelLaneError::InvalidInput("empty promotion state history".into()))?;
    let canonical_hash_basis = promotion_canonical_hash_basis(
        &input,
        outcome,
        final_state,
        denial_reason,
        current_event_ledger_version,
        current_schema_id.as_deref(),
    );
    let canonical_decision_hash = dexterity_sha256_hex(serde_json::to_vec(&canonical_hash_basis)?);

    Ok(ModelLanePromotionDecisionRecord {
        inner: input,
        outcome,
        final_state,
        denial_reason,
        state_history,
        canonical_input_refs,
        canonical_hash_basis,
        canonical_decision_hash,
        current_event_ledger_version,
        current_schema_id,
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
        event_stream_version: 0,
        transaction_seq: 0,
    })
}

async fn latest_event_ledger_version_tx(
    tx: &mut Transaction<'_, Postgres>,
    aggregate_type: &str,
    aggregate_id: &str,
) -> ModelLaneResult<Option<i64>> {
    sqlx::query_scalar(
        r#"
        SELECT event_sequence
        FROM kernel_event_ledger
        WHERE aggregate_type = $1 AND aggregate_id = $2
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(aggregate_type)
    .bind(aggregate_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(ModelLaneError::from)
}

async fn current_schema_id_for_aggregate_tx(
    tx: &mut Transaction<'_, Postgres>,
    aggregate_type: &str,
) -> ModelLaneResult<Option<String>> {
    let table_name = match aggregate_type {
        "model_lane_run" => "model_lane_runs",
        "model_lane" => "model_lanes",
        "model_lane_message" => "model_lane_messages",
        "model_lane_promotion_decision" => "model_lane_promotion_decisions",
        "model_lane_context_bundle_artifact" => "model_lane_context_bundle_artifacts",
        "model_lane_context_bundle_handoff" => "model_lane_context_bundle_handoffs",
        "model_lane_cloud_projection_plan" => "model_lane_cloud_projection_plans",
        "model_lane_cloud_consent_receipt" => "model_lane_cloud_consent_receipts",
        "model_lane_cloud_consent_denial" => "kernel_event_ledger",
        "model_lane_recovery_checkpoint" => "model_lane_recovery_checkpoints",
        "model_lane_recovery_event" => "model_lane_recovery_events",
        "model_lane_lease" => "model_lane_leases",
        "model_lane_diagnostic_tier" => "model_lane_diagnostic_tier_statuses",
        "model_lane_mt_runtime_status" => "model_lane_mt_runtime_statuses",
        _ => return Ok(None),
    };
    sqlx::query_scalar(
        r#"
        SELECT schema_id
        FROM model_lane_schema_registry
        WHERE table_name = $1
        ORDER BY schema_version DESC
        LIMIT 1
        "#,
    )
    .bind(table_name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(ModelLaneError::from)
}

fn canonicalize_refs(field: &str, refs: &[String]) -> ModelLaneResult<Vec<String>> {
    let mut out = BTreeSet::new();
    for reference in refs {
        require_token(field, reference)?;
        out.insert(reference.clone());
    }
    Ok(out.into_iter().collect())
}

fn require_refs_subset(field: &str, refs: &[String], input_refs: &[String]) -> ModelLaneResult<()> {
    for reference in refs {
        if !input_refs.iter().any(|candidate| candidate == reference) {
            return Err(ModelLaneError::InvalidInput(format!(
                "{field} contains {reference}, which is not present in input_refs"
            )));
        }
    }
    Ok(())
}

fn require_refs_disjoint(
    left_field: &str,
    left: &[String],
    right_field: &str,
    right: &[String],
) -> ModelLaneResult<()> {
    for reference in left {
        if right.iter().any(|candidate| candidate == reference) {
            return Err(ModelLaneError::InvalidInput(format!(
                "{left_field} and {right_field} both contain {reference}"
            )));
        }
    }
    Ok(())
}

fn validate_recovery_checkpoint(input: &NewModelLaneRecoveryCheckpoint) -> ModelLaneResult<()> {
    require_token("checkpoint_id", &input.checkpoint_id)?;
    require_token("run_id", &input.run_id)?;
    if let Some(lane_id) = input.lane_id.as_deref() {
        require_token("lane_id", lane_id)?;
    }
    require_token("session_id", &input.session_id)?;
    require_token("model_session_id", &input.model_session_id)?;
    if input.last_event_ledger_seq <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "recovery checkpoint last_event_ledger_seq must be positive".into(),
        ));
    }
    if let Some(last_message_id) = input.last_message_id.as_deref() {
        require_token("last_message_id", last_message_id)?;
    }
    for payload_ref in &input.open_payload_refs {
        require_token("open_payload_refs[]", payload_ref)?;
    }
    if let Some(lease_id) = input.lease_id.as_deref() {
        require_token("lease_id", lease_id)?;
    }
    require_token("idempotency_scope", &input.idempotency_scope)?;
    if let Some(recovery_event_ref) = input.recovery_event_ref.as_deref() {
        require_token("recovery_event_ref", recovery_event_ref)?;
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    parse_utc("created_at_utc", &input.created_at_utc)?;
    if let Some(recovery_hint_ref) = input.recovery_hint_ref.as_deref() {
        require_token("recovery_hint_ref", recovery_hint_ref)?;
    }
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_recovery_event(input: &NewModelLaneRecoveryEvent) -> ModelLaneResult<()> {
    require_token("recovery_event_id", &input.recovery_event_id)?;
    require_token("run_id", &input.run_id)?;
    if let Some(lane_id) = input.lane_id.as_deref() {
        require_token("lane_id", lane_id)?;
    }
    require_token("trace_id", &input.trace_id)?;
    require_token("span_id", &input.span_id)?;
    if let Some(parent_span_id) = input.parent_span_id.as_deref() {
        require_token("parent_span_id", parent_span_id)?;
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
    }
    if let Some(session_id) = input.session_id.as_deref() {
        require_token("session_id", session_id)?;
    }
    if let Some(model_session_id) = input.model_session_id.as_deref() {
        require_token("model_session_id", model_session_id)?;
    }
    if input.replay_order_seq <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "recovery event replay_order_seq must be positive".into(),
        ));
    }
    if input.source_event_ledger_seq.is_some_and(|seq| seq <= 0) {
        return Err(ModelLaneError::InvalidInput(
            "recovery event source_event_ledger_seq must be positive when present".into(),
        ));
    }
    for payload_ref in &input.payload_refs {
        require_token("payload_refs[]", payload_ref)?;
    }
    for artifact_ref in &input.artifact_refs {
        require_token("artifact_refs[]", artifact_ref)?;
    }
    if let Some(crdt_base_snapshot_ref) = input.crdt_base_snapshot_ref.as_deref() {
        require_token("crdt_base_snapshot_ref", crdt_base_snapshot_ref)?;
    }
    if let Some(crdt_state_vector) = input.crdt_state_vector.as_deref() {
        require_token("crdt_state_vector", crdt_state_vector)?;
    }
    if let Some(crdt_stale_base_ref) = input.crdt_stale_base_ref.as_deref() {
        require_token("crdt_stale_base_ref", crdt_stale_base_ref)?;
    }
    if let Some(lease_id) = input.lease_id.as_deref() {
        require_token("lease_id", lease_id)?;
    }
    if let Some(error_code) = input.error_code.as_deref() {
        require_token("error_code", error_code)?;
    }
    require_token("replay_hint", &input.replay_hint)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    if let Some(recovery_hint_ref) = input.recovery_hint_ref.as_deref() {
        require_token("recovery_hint_ref", recovery_hint_ref)?;
    }
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_lane_lease(input: &NewModelLaneLease) -> ModelLaneResult<()> {
    require_token("lease_id", &input.lease_id)?;
    require_token("run_id", &input.run_id)?;
    if let Some(lane_id) = input.lane_id.as_deref() {
        require_token("lane_id", lane_id)?;
    } else if input.scope == ModelLaneLeaseScope::Lane {
        return Err(ModelLaneError::InvalidInput(
            "lane-scoped lease requires lane_id".into(),
        ));
    }
    require_token("scope_ref", &input.scope_ref)?;
    require_token("holder_actor_id", &input.holder_actor_id)?;
    require_token("holder_session_id", &input.holder_session_id)?;
    parse_utc("lease_expires_at_utc", &input.lease_expires_at_utc)?;
    require_token("takeover_policy_ref", &input.takeover_policy_ref)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    if let Some(recovery_hint_ref) = input.recovery_hint_ref.as_deref() {
        require_token("recovery_hint_ref", recovery_hint_ref)?;
    }
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_diagnostic_tier_status(
    input: &NewModelLaneDiagnosticTierStatus,
) -> ModelLaneResult<()> {
    require_token("diagnostic_status_id", &input.diagnostic_status_id)?;
    require_token("behavior_id", &input.behavior_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("reason", &input.reason)?;
    require_token("evidence_ref", &input.evidence_ref)?;
    if input.tier == ModelLaneDiagnosticTier::FlightRecorder
        && input.evidence_ref.starts_with("flight-recorder://")
    {
        return Err(ModelLaneError::InvalidInput(
            "FlightRecorder tier must point at kernel_event_ledger/EventLedger evidence, not a detached flight-recorder-only ref".into(),
        ));
    }
    if let Some(follow_up_ref) = input.follow_up_ref.as_deref() {
        require_token("follow_up_ref", follow_up_ref)?;
    }
    if input.state == ModelLaneDiagnosticTierState::Missing {
        return Err(ModelLaneError::InvalidInput(
            "HBR-INT-009 diagnostic tier status cannot be missing".into(),
        ));
    }
    if input.state == ModelLaneDiagnosticTierState::DeferredWithReason
        && input.follow_up_ref.is_none()
    {
        return Err(ModelLaneError::InvalidInput(
            "deferred diagnostic tier requires follow_up_ref".into(),
        ));
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_mt_runtime_status(input: &NewModelLaneMtRuntimeStatus) -> ModelLaneResult<()> {
    require_token("mt_status_id", &input.mt_status_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    if let Some(claimed_by_ref) = input.claimed_by_ref.as_deref() {
        require_token("claimed_by_ref", claimed_by_ref)?;
    }
    if let Some(blocker_ref) = input.blocker_ref.as_deref() {
        require_token("blocker_ref", blocker_ref)?;
    }
    if let Some(missing_resource_ref) = input.missing_resource_ref.as_deref() {
        require_token("missing_resource_ref", missing_resource_ref)?;
    }
    if let Some(proof_status_ref) = input.proof_status_ref.as_deref() {
        require_token("proof_status_ref", proof_status_ref)?;
    }
    if let Some(hbr_status_ref) = input.hbr_status_ref.as_deref() {
        require_token("hbr_status_ref", hbr_status_ref)?;
    }
    if let Some(last_recovery_event_ref) = input.last_recovery_event_ref.as_deref() {
        require_token("last_recovery_event_ref", last_recovery_event_ref)?;
    }
    if let Some(last_runtime_status_ref) = input.last_runtime_status_ref.as_deref() {
        require_token("last_runtime_status_ref", last_runtime_status_ref)?;
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_cloud_projection_plan(input: &NewModelLaneCloudProjectionPlan) -> ModelLaneResult<()> {
    require_token("projection_plan_id", &input.projection_plan_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("lane_id", &input.lane_id)?;
    require_token("model_session_id", &input.model_session_id)?;
    validate_cloud_provider_kind(&input.provider_kind)?;
    require_token("requested_model_id", &input.requested_model_id)?;
    validate_sha256("scope_hash", &input.scope_hash)?;
    validate_sha256("payload_sha256", &input.payload_sha256)?;
    if input.source_artifact_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "ProjectionPlan requires source_artifact_refs".into(),
        ));
    }
    for reference in &input.source_artifact_refs {
        require_token("source_artifact_refs[]", reference)?;
        reject_hidden_provider_ref("source_artifact_refs[]", reference)?;
    }
    require_token("payload_artifact_ref", &input.payload_artifact_ref)?;
    reject_hidden_provider_ref("payload_artifact_ref", &input.payload_artifact_ref)?;
    require_token("redaction_policy_ref", &input.redaction_policy_ref)?;
    require_token("redaction_summary", &input.redaction_summary)?;
    require_token("provider_profile_ref", &input.provider_profile_ref)?;
    if input.fan_out_targets.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "ProjectionPlan requires fan_out_targets".into(),
        ));
    }
    for target in &input.fan_out_targets {
        require_token("fan_out_targets[]", target)?;
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    parse_utc("created_at_utc", &input.created_at_utc)?;
    require_token("user_manual_behavior_ref", &input.user_manual_behavior_ref)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_cloud_consent_receipt(input: &NewModelLaneCloudConsentReceipt) -> ModelLaneResult<()> {
    require_token("consent_receipt_id", &input.consent_receipt_id)?;
    require_token("projection_plan_id", &input.projection_plan_id)?;
    validate_sha256("projection_plan_hash", &input.projection_plan_hash)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("lane_id", &input.lane_id)?;
    require_token("model_session_id", &input.model_session_id)?;
    validate_cloud_provider_kind(&input.provider_kind)?;
    require_token("requested_model_id", &input.requested_model_id)?;
    validate_sha256("scope_hash", &input.scope_hash)?;
    if input.fan_out_targets.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt requires fan_out_targets".into(),
        ));
    }
    for target in &input.fan_out_targets {
        require_token("fan_out_targets[]", target)?;
    }
    require_token("approved_by_ref", &input.approved_by_ref)?;
    parse_utc("approved_at_utc", &input.approved_at_utc)?;
    let valid_from = parse_utc("valid_from_utc", &input.valid_from_utc)?;
    let valid_until = parse_utc("valid_until_utc", &input.valid_until_utc)?;
    if valid_until <= valid_from {
        return Err(ModelLaneError::InvalidInput(
            "valid_until_utc must be after valid_from_utc".into(),
        ));
    }
    if let Some(revoked_at_utc) = input.revoked_at_utc.as_deref() {
        parse_utc("revoked_at_utc", revoked_at_utc)?;
    }
    if input.status == ModelLaneCloudConsentReceiptStatus::Revoked {
        require_optional_token("revocation_ref", input.revocation_ref.as_deref())?;
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    parse_utc("created_at_utc", &input.created_at_utc)?;
    require_token("user_manual_behavior_ref", &input.user_manual_behavior_ref)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_cloud_provider_kind(provider_kind: &str) -> ModelLaneResult<()> {
    require_token("provider_kind", provider_kind)?;
    match provider_kind {
        "openai" | "anthropic" => Ok(()),
        other => Err(ModelLaneError::InvalidInput(format!(
            "cloud provider_kind {other} is not supported by Dexterity cloud consent"
        ))),
    }
}

fn cloud_projection_plan_hash(input: &NewModelLaneCloudProjectionPlan) -> ModelLaneResult<String> {
    Ok(dexterity_sha256_hex(canonical_json_bytes(
        &serde_json::to_value(input)?,
    )))
}

fn cloud_consent_receipt_hash(input: &NewModelLaneCloudConsentReceipt) -> ModelLaneResult<String> {
    Ok(dexterity_sha256_hex(canonical_json_bytes(
        &serde_json::to_value(input)?,
    )))
}

fn cloud_projection_plan_event_payload(record: &ModelLaneCloudProjectionPlanRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_cloud_projection_plan@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "user_manual_behavior_ref": &record.user_manual_behavior_ref,
        "record": record,
    })
}

fn cloud_consent_receipt_event_payload(record: &ModelLaneCloudConsentReceiptRecord) -> Value {
    let mut payload = json!({
        "schema_id": "hsk.model_lane_cloud_consent_receipt@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "user_manual_behavior_ref": &record.user_manual_behavior_ref,
        "record": record,
    });
    if record.status == ModelLaneCloudConsentReceiptStatus::Revoked {
        if let Some(object) = payload.as_object_mut() {
            object.insert("reason_code".into(), json!("CX-MM-007"));
            object.insert("consent_status".into(), json!("CX-MM-007"));
            object.insert(
                "revocation_ref".into(),
                json!(record.revocation_ref.as_deref()),
            );
            object.insert("provider_call_attempted".into(), json!(false));
        }
    }
    payload
}

fn recovery_checkpoint_event_payload(record: &ModelLaneRecoveryCheckpointRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_recovery_checkpoint@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "record": record,
    })
}

fn recovery_event_event_payload(record: &ModelLaneRecoveryEventRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_recovery_event@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "record": record,
    })
}

fn lane_lease_event_payload(record: &ModelLaneLeaseRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_lease@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "record": record,
    })
}

fn diagnostic_tier_event_payload(record: &ModelLaneDiagnosticTierStatusRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_diagnostic_tier@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "record": record,
    })
}

fn mt_runtime_status_event_payload(record: &ModelLaneMtRuntimeStatusRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_mt_runtime_status@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "record": record,
    })
}

fn parse_utc(field: &str, value: &str) -> ModelLaneResult<DateTime<Utc>> {
    require_token(field, value)?;
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|err| ModelLaneError::InvalidInput(format!("{field} must be RFC3339 UTC: {err}")))
}

fn ensure_object_payload(field: &str, payload: &Value) -> ModelLaneResult<()> {
    if payload.is_object() {
        Ok(())
    } else {
        Err(ModelLaneError::InvalidInput(format!(
            "{field} must be a JSON object"
        )))
    }
}

fn required_json_text(payload: &Value, field: &str) -> ModelLaneResult<String> {
    let value = payload
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default();
    require_token(field, value)?;
    Ok(value.to_string())
}

fn require_json_string(payload: &Value, field: &str, expected: &str) -> ModelLaneResult<()> {
    let actual = required_json_text(payload, field)?;
    require_equal(field, &actual, "expected", expected)
}

fn json_string(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn merge_diagnostic_payload(mut base: Value, overlay: Value) -> Value {
    match (&mut base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                base_map.insert(key, value);
            }
            base
        }
        (_, overlay) => overlay,
    }
}

fn is_cloud_lane(input: &NewModelLane) -> bool {
    input.runtime_binding == RuntimeBinding::Cloud
        || matches!(
            input.provider_kind,
            ModelLaneProviderKind::OpenAi | ModelLaneProviderKind::Anthropic
        )
}

fn is_cloud_lane_record(record: &ModelLaneRecord) -> bool {
    is_cloud_lane(&record.inner)
}

fn reject_hidden_provider_ref(field: &str, reference: &str) -> ModelLaneResult<()> {
    let normalized = reference.trim().to_ascii_lowercase();
    if normalized.starts_with("provider-session://") || normalized.starts_with("provider-memory://")
    {
        return Err(ModelLaneError::InvalidInput(format!(
            "{field} cannot use hidden provider/session memory"
        )));
    }
    Ok(())
}

async fn resolve_promotion_input_refs_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    input_refs: &[String],
    selected_input_refs: &[String],
) -> ModelLaneResult<PromotionInputResolution> {
    let mut records_by_ref = BTreeMap::new();
    let mut denial_reason = None;
    for reference in input_refs {
        let message_id = message_id_from_ref("input_refs[]", reference)?;
        match message_by_id_tx(tx, &message_id).await? {
            Some(record) if record.run_id == run_id => {
                if !matches!(
                    record.authority,
                    ModelLaneAuthority::Advisory | ModelLaneAuthority::PromotionCandidate
                ) {
                    denial_reason =
                        denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
                }
                records_by_ref.insert(reference.clone(), record);
            }
            Some(_) | None => {
                denial_reason =
                    denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
            }
        }
    }

    let mut selected_records = Vec::new();
    for reference in selected_input_refs {
        if let Some(record) = records_by_ref.get(reference) {
            selected_records.push(record.clone());
        } else {
            denial_reason =
                denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
        }
    }
    if selected_records.is_empty() {
        denial_reason = denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
    }

    let mut current_base_snapshot_ref: Option<String> = None;
    let mut current_state_vector: Option<String> = None;
    for record in &selected_records {
        match (
            record.crdt_base_snapshot_ref.as_deref(),
            record.crdt_state_vector.as_deref(),
        ) {
            (Some(base_snapshot_ref), Some(state_vector)) => {
                require_token("selected.crdt_base_snapshot_ref", base_snapshot_ref)?;
                require_token("selected.crdt_state_vector", state_vector)?;
                if current_base_snapshot_ref
                    .as_deref()
                    .is_some_and(|current| current != base_snapshot_ref)
                    || current_state_vector
                        .as_deref()
                        .is_some_and(|current| current != state_vector)
                {
                    denial_reason =
                        denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
                }
                current_base_snapshot_ref.get_or_insert_with(|| base_snapshot_ref.to_string());
                current_state_vector.get_or_insert_with(|| state_vector.to_string());
            }
            _ => {
                denial_reason =
                    denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
            }
        }
    }

    selected_records.sort_by(|left, right| {
        left.event_ledger_seq
            .cmp(&right.event_ledger_seq)
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    Ok(PromotionInputResolution {
        denial_reason,
        current_base_snapshot_ref,
        current_state_vector,
        selected_message_ids: selected_records
            .into_iter()
            .map(|record| record.message_id.clone())
            .collect(),
    })
}

fn message_id_from_ref(field: &str, reference: &str) -> ModelLaneResult<String> {
    require_token(field, reference)?;
    let message_id = reference
        .strip_prefix("model-lane-message://")
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "{field} must use model-lane-message://<message_id>"
            ))
        })?;
    require_token(field, message_id)?;
    Ok(message_id.to_string())
}

fn missing_promoted_artifact_binding(input: &NewModelLanePromotionDecision) -> bool {
    input.promoted_artifact_ref.is_none()
        || input.promoted_artifact_sha256.is_none()
        || input.promoted_artifact_version.is_none()
}

fn promotion_state_history(outcome: ModelLanePromotionOutcome) -> Vec<ModelLanePromotionState> {
    match outcome {
        ModelLanePromotionOutcome::Approved => vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::PendingApproval,
            ModelLanePromotionState::Approved,
            ModelLanePromotionState::Executing,
            ModelLanePromotionState::Executed,
        ],
        ModelLanePromotionOutcome::Denied => vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::Denied,
        ],
    }
}

fn promotion_canonical_hash_basis(
    input: &NewModelLanePromotionDecision,
    outcome: ModelLanePromotionOutcome,
    final_state: ModelLanePromotionState,
    denial_reason: Option<ModelLanePromotionDenialReason>,
    current_event_ledger_version: Option<i64>,
    current_schema_id: Option<&str>,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_promotion_decision@1",
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "coordinator_session_id": &input.coordinator_session_id,
        "routing_policy": input.routing_policy.as_str(),
        "input_refs": &input.input_refs,
        "selected_input_refs": &input.selected_input_refs,
        "rejected_input_refs": &input.rejected_input_refs,
        "validator_authority_ref": &input.validator_authority_ref,
        "operator_authority_ref": &input.operator_authority_ref,
        "expected_event_ledger": {
            "aggregate_type": &input.expected_event_ledger_aggregate_type,
            "aggregate_id": &input.expected_event_ledger_aggregate_id,
            "version": input.expected_event_ledger_version,
            "current_version": current_event_ledger_version,
        },
        "crdt": {
            "base_snapshot_ref": &input.base_snapshot_ref,
            "current_base_snapshot_ref": &input.current_base_snapshot_ref,
            "state_vector": &input.state_vector,
            "current_state_vector": &input.current_state_vector,
        },
        "schema_guard": {
            "expected_schema_id": &input.schema_id,
            "current_schema_id": current_schema_id,
        },
        "deterministic_tie_break_rule": &input.deterministic_tie_break_rule,
        "promotion_gate_ref": &input.promotion_gate_ref,
        "promotion_receipt_ref": &input.promotion_receipt_ref,
        "promoted_artifact": {
            "ref": &input.promoted_artifact_ref,
            "sha256": &input.promoted_artifact_sha256,
            "version": &input.promoted_artifact_version,
        },
        "direct_authority_mutation_attempt_ref": &input.direct_authority_mutation_attempt_ref,
        "outcome": outcome.as_str(),
        "final_state": final_state.as_str(),
        "denial_reason": denial_reason.map(|reason| reason.as_str()),
    })
}

fn validate_run(input: &NewModelLaneRun) -> ModelLaneResult<()> {
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("run_span_id", &input.run_span_id)?;
    require_token("coordinator_session_id", &input.coordinator_session_id)?;
    require_token("context_bundle_id", &input.context_bundle_id)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("artifact_namespace", &input.artifact_namespace)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    let work_packet_id = require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    let micro_task_id = require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    let task_board_id = require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    require_token("memory_pack_ref", &input.memory_pack_ref)?;
    validate_sha256("memory_pack_hash", &input.memory_pack_hash)?;
    require_token("determinism_mode", &input.determinism_mode)?;
    require_token("budget_summary_ref", &input.budget_summary_ref)?;
    require_token("procedural_review_status", &input.procedural_review_status)?;
    if input.candidate_model_ids.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "candidate_model_ids must contain at least one model id".into(),
        ));
    }
    if input.lane_ids.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "lane_ids must contain at least one lane".into(),
        ));
    }
    let locus = validate_locus(input.locus_binding.as_ref(), "run")?;
    validate_locus_common(
        locus,
        &work_packet_id,
        &micro_task_id,
        Some(&task_board_id),
        &input.coordinator_session_id,
        &input.owner_session,
    )?;
    Ok(())
}

fn validate_lane(input: &NewModelLane) -> ModelLaneResult<()> {
    require_token("lane_id", &input.lane_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("lane_span_id", &input.lane_span_id)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("role", &input.role)?;
    require_token("backend", &input.backend)?;
    require_token("session_id", &input.session_id)?;
    require_token("model_session_id", &input.model_session_id)?;
    require_token("adapter_id", &input.adapter_id)?;
    require_token("owner_session", &input.owner_session)?;
    let work_packet_id = require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    let micro_task_id = require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    let task_board_id = require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    validate_lane_runtime_contract(input)?;
    let locus = validate_locus(input.locus_binding.as_ref(), "lane")?;
    validate_locus_common(
        locus,
        &work_packet_id,
        &micro_task_id,
        Some(&task_board_id),
        &locus.coordinator_session_id,
        &input.owner_session,
    )?;
    require_equal(
        "locus.session_id",
        &locus.session_id,
        "lane.session_id",
        &input.session_id,
    )?;
    require_equal(
        "locus.model_session_id",
        &locus.model_session_id,
        "lane.model_session_id",
        &input.model_session_id,
    )?;
    Ok(())
}

fn validate_prepared_launch_pair(
    run: &NewModelLaneRun,
    lane: &NewModelLane,
) -> ModelLaneResult<()> {
    require_equal("lane.run_id", &lane.run_id, "run.run_id", &run.run_id)?;
    if !run.lane_ids.iter().any(|id| id == &lane.lane_id) {
        return Err(ModelLaneError::InvalidInput(format!(
            "run.lane_ids must include lane.lane_id {}",
            lane.lane_id
        )));
    }
    require_equal(
        "lane.trace_id",
        &lane.trace_id,
        "run.trace_id",
        &run.trace_id,
    )?;
    require_equal(
        "lane.event_ledger_stream_id",
        &lane.event_ledger_stream_id,
        "run.event_ledger_stream_id",
        &run.event_ledger_stream_id,
    )?;
    require_equal(
        "lane.owner_session",
        &lane.owner_session,
        "run.owner_session",
        &run.owner_session,
    )?;
    require_equal(
        "lane.work_packet_id",
        lane.work_packet_id.as_deref().unwrap_or(""),
        "run.work_packet_id",
        run.work_packet_id.as_deref().unwrap_or(""),
    )?;
    require_equal(
        "lane.micro_task_id",
        lane.micro_task_id.as_deref().unwrap_or(""),
        "run.micro_task_id",
        run.micro_task_id.as_deref().unwrap_or(""),
    )?;
    require_equal(
        "lane.task_board_id",
        lane.task_board_id.as_deref().unwrap_or(""),
        "run.task_board_id",
        run.task_board_id.as_deref().unwrap_or(""),
    )?;
    Ok(())
}

fn validate_message(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    require_token("message_id", &input.message_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("message_span_id", &input.message_span_id)?;
    require_token("from_lane_id", &input.from_lane_id)?;
    require_token("payload_ref", &input.payload_ref)?;
    reject_hidden_provider_ref("payload_ref", &input.payload_ref)?;
    validate_sha256("payload_sha256", &input.payload_sha256)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("coordinator_session_id", &input.coordinator_session_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    let work_packet_id = require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    let micro_task_id = require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    let task_board_id = require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    validate_message_trace(input)?;
    validate_message_routing(input)?;
    for (field, value) in [
        ("proposal_ref", input.proposal_ref.as_deref()),
        ("crdt_update_ref", input.crdt_update_ref.as_deref()),
        (
            "crdt_base_snapshot_ref",
            input.crdt_base_snapshot_ref.as_deref(),
        ),
        ("crdt_proposal_ref", input.crdt_proposal_ref.as_deref()),
        ("crdt_stale_base_ref", input.crdt_stale_base_ref.as_deref()),
        (
            "promoted_artifact_ref",
            input.promoted_artifact_ref.as_deref(),
        ),
    ] {
        if let Some(reference) = value {
            reject_hidden_provider_ref(field, reference)?;
        }
    }
    validate_message_authority(input)?;
    let locus = validate_locus(input.locus_binding.as_ref(), "message")?;
    validate_locus_common(
        locus,
        &work_packet_id,
        &micro_task_id,
        Some(&task_board_id),
        &input.coordinator_session_id,
        &input.owner_session,
    )?;
    Ok(())
}

fn validate_promotion_decision(input: &NewModelLanePromotionDecision) -> ModelLaneResult<()> {
    require_token("decision_id", &input.decision_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("decision_span_id", &input.decision_span_id)?;
    require_token("coordinator_session_id", &input.coordinator_session_id)?;
    require_token(
        "expected_event_ledger_aggregate_type",
        &input.expected_event_ledger_aggregate_type,
    )?;
    require_token(
        "expected_event_ledger_aggregate_id",
        &input.expected_event_ledger_aggregate_id,
    )?;
    if input.expected_event_ledger_version <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "expected_event_ledger_version must be positive".into(),
        ));
    }
    require_token("base_snapshot_ref", &input.base_snapshot_ref)?;
    require_token(
        "current_base_snapshot_ref",
        &input.current_base_snapshot_ref,
    )?;
    require_token("state_vector", &input.state_vector)?;
    require_token("current_state_vector", &input.current_state_vector)?;
    require_token("schema_id", &input.schema_id)?;
    require_token(
        "deterministic_tie_break_rule",
        &input.deterministic_tie_break_rule,
    )?;
    require_token("promotion_gate_ref", &input.promotion_gate_ref)?;
    require_optional_token(
        "promotion_receipt_ref",
        input.promotion_receipt_ref.as_deref(),
    )?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    require_optional_token("recovery_hint_ref", input.recovery_hint_ref.as_deref())?;
    if let Some(validator_ref) = input.validator_authority_ref.as_deref() {
        require_token("validator_authority_ref", validator_ref)?;
    }
    if let Some(operator_ref) = input.operator_authority_ref.as_deref() {
        require_token("operator_authority_ref", operator_ref)?;
    }
    let parent_span_id = require_optional_token("parent_span_id", input.parent_span_id.as_deref())?;
    if parent_span_id == input.decision_span_id {
        return Err(ModelLaneError::InvalidInput(
            "parent_span_id must not equal decision_span_id".into(),
        ));
    }
    if input.linked_span_contexts.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "linked_span_contexts must include at least one span".into(),
        ));
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
        if linked == &input.decision_span_id {
            return Err(ModelLaneError::InvalidInput(
                "linked_span_contexts must not include decision_span_id".into(),
            ));
        }
    }
    if input.input_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "input_refs must contain at least one advisory input".into(),
        ));
    }
    if input.selected_input_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "selected_input_refs must contain at least one advisory input".into(),
        ));
    }
    for reference in &input.input_refs {
        require_token("input_refs[]", reference)?;
    }
    for reference in &input.selected_input_refs {
        require_token("selected_input_refs[]", reference)?;
    }
    for reference in &input.rejected_input_refs {
        require_token("rejected_input_refs[]", reference)?;
    }
    if let Some(attempt_ref) = input.direct_authority_mutation_attempt_ref.as_deref() {
        require_token("direct_authority_mutation_attempt_ref", attempt_ref)?;
    }
    if let Some(artifact_ref) = input.promoted_artifact_ref.as_deref() {
        require_token("promoted_artifact_ref", artifact_ref)?;
        reject_hidden_provider_ref("promoted_artifact_ref", artifact_ref)?;
    }
    if let Some(artifact_sha256) = input.promoted_artifact_sha256.as_deref() {
        validate_sha256("promoted_artifact_sha256", artifact_sha256)?;
    }
    if let Some(artifact_version) = input.promoted_artifact_version.as_deref() {
        require_token("promoted_artifact_version", artifact_version)?;
    }
    Ok(())
}

fn validate_context_bundle_artifact_binding(
    input: &NewModelLaneContextBundleArtifactBinding,
) -> ModelLaneResult<()> {
    require_token("artifact_binding_id", &input.artifact_binding_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("artifact_ref", &input.artifact_ref)?;
    validate_sha256("artifact_sha256", &input.artifact_sha256)?;
    validate_sha256("content_hash", &input.content_hash)?;
    require_equal(
        "artifact_sha256",
        &input.artifact_sha256,
        "content_hash",
        &input.content_hash,
    )?;
    require_token("artifact_kind", &input.artifact_kind)?;
    require_token("artifact_manifest_ref", &input.artifact_manifest_ref)?;
    require_token("artifact_payload_ref", &input.artifact_payload_ref)?;
    require_equal(
        "artifact_ref",
        &input.artifact_ref,
        "artifact_payload_ref",
        &input.artifact_payload_ref,
    )?;
    let payload_hash = dexterity_sha256_hex(canonical_json_bytes(&input.payload_json));
    require_equal(
        "payload_json sha256",
        &payload_hash,
        "content_hash",
        &input.content_hash,
    )?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    if !input.diagnostic_payload.is_object() {
        return Err(ModelLaneError::InvalidInput(
            "diagnostic_payload must be a JSON object".into(),
        ));
    }
    Ok(())
}

fn validate_context_bundle_handoff(
    input: &NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<()> {
    require_token("handoff_id", &input.handoff_id)?;
    require_token("context_bundle_id", &input.context_bundle_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("handoff_span_id", &input.handoff_span_id)?;
    require_token("downstream_lane_id", &input.downstream_lane_id)?;
    require_token("source_lane_id", &input.source_lane_id)?;
    require_token("source_message_id", &input.source_message_id)?;
    require_token("artifact_ref", &input.artifact_ref)?;
    validate_sha256("artifact_sha256", &input.artifact_sha256)?;
    validate_sha256("content_hash", &input.content_hash)?;
    require_token("reason_code", &input.reason_code)?;
    if let Some(decision_ref) = input.decision_ref.as_deref() {
        require_token("decision_ref", decision_ref)?;
    }
    if let Some(reviewer_ref) = input.reviewer_ref.as_deref() {
        require_token("reviewer_ref", reviewer_ref)?;
    }
    require_token("replay_hint", &input.replay_hint)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    let expected_context_bundle_id = model_lane_context_bundle_id_for_handoff(input)?;
    require_equal(
        "context_bundle_id",
        &input.context_bundle_id,
        "derived context bundle id",
        &expected_context_bundle_id,
    )?;
    let parent_span_id = require_optional_token("parent_span_id", input.parent_span_id.as_deref())?;
    if parent_span_id == input.handoff_span_id {
        return Err(ModelLaneError::InvalidInput(
            "parent_span_id must not equal handoff_span_id".into(),
        ));
    }
    if input.linked_span_contexts.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "linked_span_contexts must include at least one span".into(),
        ));
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
        if linked == &input.handoff_span_id {
            return Err(ModelLaneError::InvalidInput(
                "linked_span_contexts must not include handoff_span_id".into(),
            ));
        }
    }
    if matches!(
        input.selection_state,
        ModelLaneHandoffSelectionState::Selected
            | ModelLaneHandoffSelectionState::Rejected
            | ModelLaneHandoffSelectionState::Superseded
    ) {
        require_optional_token("decision_ref", input.decision_ref.as_deref())?;
        require_optional_token("reviewer_ref", input.reviewer_ref.as_deref())?;
    }
    if !input.diagnostic_payload.is_object() {
        return Err(ModelLaneError::InvalidInput(
            "diagnostic_payload must be a JSON object".into(),
        ));
    }
    if let Some(crdt) = input.crdt_payload.as_ref() {
        validate_crdt_handoff_metadata(crdt)?;
    }
    if input.loom_refs.len() > MAX_CONTEXT_BUNDLE_LOOM_REFS {
        return Err(ModelLaneError::InvalidInput(format!(
            "loom_refs exceeds bounded limit {MAX_CONTEXT_BUNDLE_LOOM_REFS}"
        )));
    }
    for loom_ref in &input.loom_refs {
        validate_loom_handoff_ref(loom_ref)?;
    }
    if input.memory_pack_refs.len() > MAX_CONTEXT_BUNDLE_MEMORY_PACK_REFS {
        return Err(ModelLaneError::InvalidInput(format!(
            "memory_pack_refs exceeds bounded FEMS limit {MAX_CONTEXT_BUNDLE_MEMORY_PACK_REFS}"
        )));
    }
    for memory_pack_ref in &input.memory_pack_refs {
        validate_memory_pack_handoff_ref(memory_pack_ref)?;
    }
    Ok(())
}

fn validate_crdt_handoff_metadata(crdt: &ModelLaneCrdtHandoffMetadata) -> ModelLaneResult<()> {
    require_token("crdt_payload.schema_id", &crdt.schema_id)?;
    if crdt.schema_id != "hsk.model_lane_crdt_payload@1" {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.schema_id must be hsk.model_lane_crdt_payload@1".into(),
        ));
    }
    require_token("crdt_payload.document_id", &crdt.document_id)?;
    require_token("crdt_payload.workspace_id", &crdt.workspace_id)?;
    require_token("crdt_payload.actor_id", &crdt.actor_id)?;
    require_token("crdt_payload.actor_kind", &crdt.actor_kind)?;
    require_token("crdt_payload.lane_id", &crdt.lane_id)?;
    require_token("crdt_payload.crdt_site_id", &crdt.crdt_site_id)?;
    if crdt.update_seq <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.update_seq must be positive".into(),
        ));
    }
    require_token("crdt_payload.update_bytes_ref", &crdt.update_bytes_ref)?;
    validate_sha256("crdt_payload.update_sha256", &crdt.update_sha256)?;
    require_token("crdt_payload.state_vector", &crdt.state_vector)?;
    require_token("crdt_payload.base_snapshot_ref", &crdt.base_snapshot_ref)?;
    validate_sha256(
        "crdt_payload.materialized_projection_hash",
        &crdt.materialized_projection_hash,
    )?;
    if !crdt.replay_metadata.is_object() {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.replay_metadata must be a JSON object".into(),
        ));
    }
    let format = crdt
        .replay_metadata
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let yjs_compatible = crdt
        .replay_metadata
        .get("yjs_compatible")
        .and_then(Value::as_bool)
        == Some(true);
    if !yjs_compatible || !matches!(format, "yjs_update_v1" | "yjs_update_v2") {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.replay_metadata must declare Yjs-compatible format yjs_update_v1 or yjs_update_v2".into(),
        ));
    }
    require_token("crdt_payload.promotion_gate_ref", &crdt.promotion_gate_ref)?;
    if let Some(promotion_receipt_ref) = crdt.promotion_receipt_ref.as_deref() {
        require_token("crdt_payload.promotion_receipt_ref", promotion_receipt_ref)?;
    }
    require_token(
        "crdt_payload.validation_runner_ref",
        &crdt.validation_runner_ref,
    )?;
    require_token("crdt_payload.authority_effect", &crdt.authority_effect)?;
    if crdt.authority_effect != "advisory_only" {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.authority_effect must be advisory_only before promotion".into(),
        ));
    }
    Ok(())
}

fn validate_loom_handoff_ref(loom_ref: &ModelLaneLoomHandoffRef) -> ModelLaneResult<()> {
    require_token("loom_ref.workspace_id", &loom_ref.workspace_id)?;
    require_token("loom_ref.block_id", &loom_ref.block_id)?;
    if let Some(source_block_id) = loom_ref.source_block_id.as_deref() {
        require_token("loom_ref.source_block_id", source_block_id)?;
    }
    if let Some(target_block_id) = loom_ref.target_block_id.as_deref() {
        require_token("loom_ref.target_block_id", target_block_id)?;
    }
    if let Some(artifact_ref) = loom_ref.artifact_ref.as_deref() {
        require_token("loom_ref.artifact_ref", artifact_ref)?;
    }
    validate_sha256("loom_ref.content_hash", &loom_ref.content_hash)?;
    require_token("loom_ref.version", &loom_ref.version)?;
    require_token(
        "loom_ref.event_ledger_evidence_ref",
        &loom_ref.event_ledger_evidence_ref,
    )?;
    if !loom_ref
        .event_ledger_evidence_ref
        .starts_with("eventledger://")
    {
        return Err(ModelLaneError::InvalidInput(
            "loom_ref.event_ledger_evidence_ref must use eventledger://".into(),
        ));
    }
    require_token(
        "loom_ref.flight_recorder_evidence_ref",
        &loom_ref.flight_recorder_evidence_ref,
    )?;
    if !loom_ref
        .flight_recorder_evidence_ref
        .starts_with("flight-recorder://")
    {
        return Err(ModelLaneError::InvalidInput(
            "loom_ref.flight_recorder_evidence_ref must use flight-recorder://".into(),
        ));
    }
    Ok(())
}

fn validate_memory_pack_handoff_ref(
    memory_pack: &ModelLaneMemoryPackHandoffRef,
) -> ModelLaneResult<()> {
    require_token("memory_pack_ref", &memory_pack.memory_pack_ref)?;
    if is_hidden_memory_pack_ref(&memory_pack.memory_pack_ref) {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff cannot use hidden provider/session memory as authority".into(),
        ));
    }
    validate_sha256("memory_pack_hash", &memory_pack.memory_pack_hash)?;
    require_token("memory_pack.scope_tag", &memory_pack.scope_tag)?;
    require_token("memory_pack.review_status", &memory_pack.review_status)?;
    if !matches!(
        memory_pack.review_status.as_str(),
        "reviewed" | "operator_reviewed" | "validator_reviewed"
    ) {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff requires review_status reviewed, operator_reviewed, or validator_reviewed".into(),
        ));
    }
    require_token("memory_pack.classification", &memory_pack.classification)?;
    if !matches!(
        memory_pack.classification.as_str(),
        "cloud_safe_context" | "local_only_context" | "operator_reviewed_context"
    ) {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff classification must be cloud_safe_context, local_only_context, or operator_reviewed_context".into(),
        ));
    }
    if let Some(projection_ref) = memory_pack.projection_ref.as_deref() {
        require_token("memory_pack.projection_ref", projection_ref)?;
        if is_hidden_memory_pack_ref(projection_ref) {
            return Err(ModelLaneError::InvalidInput(
                "MemoryPack handoff projection_ref cannot use hidden provider/session memory as authority".into(),
            ));
        }
    }
    require_token("memory_pack.evidence_ref", &memory_pack.evidence_ref)?;
    if !memory_pack.evidence_ref.starts_with("eventledger://")
        && !memory_pack.evidence_ref.starts_with("flight-recorder://")
    {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff evidence_ref must use eventledger:// or flight-recorder://".into(),
        ));
    }
    Ok(())
}

fn is_hidden_memory_pack_ref(reference: &str) -> bool {
    let normalized = reference.trim().to_ascii_lowercase();
    [
        "hidden://",
        "provider-session://",
        "provider_memory://",
        "session-memory://",
        "chat-history://",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}

fn context_bundle_artifact_binding_hash(
    input: &NewModelLaneContextBundleArtifactBinding,
) -> ModelLaneResult<String> {
    Ok(dexterity_sha256_hex(canonical_json_bytes(
        &context_bundle_artifact_binding_hash_basis(input),
    )))
}

fn context_bundle_artifact_binding_hash_basis(
    input: &NewModelLaneContextBundleArtifactBinding,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_artifact@1",
        "dexterity_kernel": "Dexterity",
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "artifact_ref": &input.artifact_ref,
        "artifact_sha256": &input.artifact_sha256,
        "content_hash": &input.content_hash,
        "artifact_kind": &input.artifact_kind,
        "artifact_manifest_ref": &input.artifact_manifest_ref,
        "artifact_payload_ref": &input.artifact_payload_ref,
        "payload_json": &input.payload_json,
        "event_ledger_stream_id": &input.event_ledger_stream_id,
        "work_packet_id": &input.work_packet_id,
        "micro_task_id": &input.micro_task_id,
        "task_board_id": &input.task_board_id,
        "owner_session": &input.owner_session,
        "diagnostic_payload": &input.diagnostic_payload,
    })
}

fn context_bundle_artifact_binding_event_payload(
    record: &ModelLaneContextBundleArtifactBindingRecord,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_artifact@1",
        "dexterity_kernel": "Dexterity",
        "record": record,
    })
}

fn context_bundle_handoff_event_payload(record: &ModelLaneContextBundleHandoffRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_handoff@1",
        "dexterity_kernel": "Dexterity",
        "record": record,
    })
}

fn build_downstream_context_bundle(
    run_id: &str,
    context_bundle_id: &str,
    downstream_lane_id: &str,
    records: Vec<ModelLaneContextBundleHandoffRecord>,
) -> ModelLaneResult<ModelLaneDownstreamContextBundle> {
    let selected: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Selected)
        .cloned()
        .collect();
    let rejected: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Rejected)
        .cloned()
        .collect();
    let unresolved: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Unresolved)
        .cloned()
        .collect();
    let superseded: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Superseded)
        .cloned()
        .collect();
    let allowed_context = json!({
        "schema_id": "hsk.model_lane_downstream_context_bundle@1",
        "dexterity_kernel": "Dexterity",
        "run_id": run_id,
        "context_bundle_id": context_bundle_id,
        "downstream_lane_id": downstream_lane_id,
        "handoffs": &records,
        "selected": selected,
        "rejected": rejected,
        "unresolved": unresolved,
        "superseded": superseded,
    });
    let context_hash = dexterity_sha256_hex(canonical_json_bytes(&allowed_context));
    Ok(ModelLaneDownstreamContextBundle {
        run_id: run_id.to_string(),
        context_bundle_id: context_bundle_id.to_string(),
        downstream_lane_id: downstream_lane_id.to_string(),
        context_hash,
        allowed_context,
        records,
    })
}

fn context_bundle_identity_hash_basis(input: &NewModelLaneContextBundleHandoff) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_identity@1",
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "downstream_lane_id": &input.downstream_lane_id,
        "work_packet_id": &input.work_packet_id,
        "micro_task_id": &input.micro_task_id,
        "task_board_id": &input.task_board_id,
        "owner_session": &input.owner_session,
        "event_ledger_stream_id": &input.event_ledger_stream_id,
    })
}

fn context_bundle_handoff_hash(
    input: &NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<String> {
    let basis = context_bundle_handoff_hash_basis(input);
    Ok(dexterity_sha256_hex(serde_json::to_vec(&basis)?))
}

fn context_bundle_handoff_hash_basis(input: &NewModelLaneContextBundleHandoff) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_handoff@1",
        "dexterity_kernel": "Dexterity",
        "context_bundle_id": &input.context_bundle_id,
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "handoff_span_id": &input.handoff_span_id,
        "parent_span_id": &input.parent_span_id,
        "linked_span_contexts": &input.linked_span_contexts,
        "downstream_lane_id": &input.downstream_lane_id,
        "source_lane_id": &input.source_lane_id,
        "source_message_id": &input.source_message_id,
        "artifact_ref": &input.artifact_ref,
        "artifact_sha256": &input.artifact_sha256,
        "content_hash": &input.content_hash,
        "source_kind": input.source_kind.as_str(),
        "authority_state": input.authority_state.as_str(),
        "selection_state": input.selection_state.as_str(),
        "reason_code": &input.reason_code,
        "decision_ref": &input.decision_ref,
        "reviewer_ref": &input.reviewer_ref,
        "replay_hint": &input.replay_hint,
        "crdt_payload": &input.crdt_payload,
        "loom_refs": &input.loom_refs,
        "memory_pack_refs": &input.memory_pack_refs,
        "event_ledger_stream_id": &input.event_ledger_stream_id,
        "work_packet_id": &input.work_packet_id,
        "micro_task_id": &input.micro_task_id,
        "task_board_id": &input.task_board_id,
        "owner_session": &input.owner_session,
        "replay_order_key": &input.replay_order_key,
        "diagnostic_payload": &input.diagnostic_payload,
    })
}

fn validate_locus<'a>(
    locus: Option<&'a ModelLaneLocusBinding>,
    owner_kind: &str,
) -> ModelLaneResult<&'a ModelLaneLocusBinding> {
    let locus = locus.ok_or_else(|| {
        ModelLaneError::InvalidInput(format!("{owner_kind} requires locus_binding_ref"))
    })?;
    require_token("locus.work_packet_id", &locus.work_packet_id)?;
    require_token("locus.micro_task_id", &locus.micro_task_id)?;
    require_optional_token("locus.task_board_id", locus.task_board_id.as_deref())?;
    require_token(
        "locus.coordinator_session_id",
        &locus.coordinator_session_id,
    )?;
    require_token("locus.session_id", &locus.session_id)?;
    require_token("locus.model_session_id", &locus.model_session_id)?;
    require_token("locus.owner_session", &locus.owner_session)?;
    require_token("locus_binding_ref", &locus.locus_binding_ref)?;
    Ok(locus)
}

fn validate_locus_common(
    locus: &ModelLaneLocusBinding,
    work_packet_id: &str,
    micro_task_id: &str,
    task_board_id: Option<&str>,
    coordinator_session_id: &str,
    owner_session: &str,
) -> ModelLaneResult<()> {
    require_equal(
        "locus.work_packet_id",
        &locus.work_packet_id,
        "record.work_packet_id",
        work_packet_id,
    )?;
    require_equal(
        "locus.micro_task_id",
        &locus.micro_task_id,
        "record.micro_task_id",
        micro_task_id,
    )?;
    if let Some(task_board_id) = task_board_id {
        require_equal(
            "locus.task_board_id",
            locus.task_board_id.as_deref().unwrap_or(""),
            "record.task_board_id",
            task_board_id,
        )?;
    }
    require_equal(
        "locus.coordinator_session_id",
        &locus.coordinator_session_id,
        "record.coordinator_session_id",
        coordinator_session_id,
    )?;
    require_equal(
        "locus.owner_session",
        &locus.owner_session,
        "record.owner_session",
        owner_session,
    )
}

fn validate_lane_runtime_contract(input: &NewModelLane) -> ModelLaneResult<()> {
    if input.provider_kind == ModelLaneProviderKind::Other {
        return Err(ModelLaneError::InvalidInput(
            "provider_kind other is not supported by Dexterity".into(),
        ));
    }
    if input.capability_token_ids.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "capability_token_ids must include at least one capability token".into(),
        ));
    }
    require_optional_token(
        "effective_capability_snapshot_ref",
        input.effective_capability_snapshot_ref.as_deref(),
    )?;
    require_optional_token(
        "capability_negotiation_ref",
        input.capability_negotiation_ref.as_deref(),
    )?;
    require_optional_token(
        "provider_feature_profile_ref",
        input.provider_feature_profile_ref.as_deref(),
    )?;
    require_optional_token(
        "requested_execution_policy_ref",
        input.requested_execution_policy_ref.as_deref(),
    )?;
    require_optional_token(
        "effective_execution_policy_ref",
        input.effective_execution_policy_ref.as_deref(),
    )?;
    require_optional_token("cancellation_ref", input.cancellation_ref.as_deref())?;
    require_optional_token("reclaim_policy_ref", input.reclaim_policy_ref.as_deref())?;
    require_optional_token(
        "terminal_status_mapping_ref",
        input.terminal_status_mapping_ref.as_deref(),
    )?;
    if input.tool_gate_decision_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "tool_gate_decision_refs must include at least one ToolGate decision".into(),
        ));
    }
    for decision_ref in &input.tool_gate_decision_refs {
        require_token("tool_gate_decision_refs[]", decision_ref)?;
    }
    let expected = match input.runtime_binding {
        RuntimeBinding::Local => (
            ModelLaneKind::LocalModel,
            LaunchAuthority::ModelRuntime,
            vec![ModelLaneProviderKind::LocalRuntime],
        ),
        RuntimeBinding::Cloud => (
            ModelLaneKind::CloudModel,
            LaunchAuthority::CloudLane,
            vec![
                ModelLaneProviderKind::OpenAi,
                ModelLaneProviderKind::Anthropic,
            ],
        ),
        RuntimeBinding::CliBridge => (
            ModelLaneKind::CliModel,
            LaunchAuthority::CliBridge,
            vec![ModelLaneProviderKind::OfficialCli],
        ),
        RuntimeBinding::Human => (
            ModelLaneKind::HumanOperator,
            LaunchAuthority::Operator,
            vec![ModelLaneProviderKind::Human],
        ),
        RuntimeBinding::Subagent => (
            ModelLaneKind::Subagent,
            LaunchAuthority::SubagentManager,
            vec![ModelLaneProviderKind::Subagent],
        ),
        RuntimeBinding::Validator => (
            ModelLaneKind::Validator,
            LaunchAuthority::ValidatorRunner,
            vec![ModelLaneProviderKind::Validator],
        ),
    };
    if input.kind != expected.0 || input.launch_authority != expected.1 {
        return Err(ModelLaneError::InvalidInput(format!(
            "runtime_binding {:?} does not match kind {:?} and launch_authority {:?}",
            input.runtime_binding, input.kind, input.launch_authority
        )));
    }
    if !expected.2.contains(&input.provider_kind) {
        return Err(ModelLaneError::InvalidInput(format!(
            "provider_kind {:?} is not supported for runtime_binding {:?}",
            input.provider_kind, input.runtime_binding
        )));
    }
    match input.runtime_binding {
        RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge => {
            if input.process_ownership_ref.is_some() {
                require_optional_token(
                    "process_ownership_ref",
                    input.process_ownership_ref.as_deref(),
                )?;
                if input.no_os_process_reason_ref.is_some() {
                    return Err(ModelLaneError::InvalidInput(
                        "process-backed lanes must not use no_os_process_reason_ref when process_ownership_ref exists".into(),
                    ));
                }
            } else if input.status == ModelLaneStatus::Failed && input.startup_failure_ref.is_some()
            {
                require_optional_token(
                    "no_os_process_reason_ref",
                    input.no_os_process_reason_ref.as_deref(),
                )?;
            } else {
                return Err(ModelLaneError::InvalidInput(
                    "process-backed lanes require process_ownership_ref unless startup failed before OS ownership".into(),
                ));
            }
        }
        RuntimeBinding::Human | RuntimeBinding::Subagent | RuntimeBinding::Validator => {
            require_optional_token(
                "no_os_process_reason_ref",
                input.no_os_process_reason_ref.as_deref(),
            )?;
            if input.process_ownership_ref.is_some() {
                return Err(ModelLaneError::InvalidInput(
                    "no-OS-process lanes must not use process_ownership_ref".into(),
                ));
            }
        }
    }
    if input.runtime_binding == RuntimeBinding::Cloud {
        require_optional_token("projection_plan_ref", input.projection_plan_ref.as_deref())?;
        require_optional_token("consent_receipt_ref", input.consent_receipt_ref.as_deref())?;
    }
    if matches!(
        input.status,
        ModelLaneStatus::Failed | ModelLaneStatus::Cancelled | ModelLaneStatus::Reclaimable
    ) {
        require_optional_token("failstate_code", input.failstate_code.as_deref())?;
        require_optional_token("reason_ref", input.reason_ref.as_deref())?;
    }
    if input.status == ModelLaneStatus::Failed {
        require_optional_token("startup_failure_ref", input.startup_failure_ref.as_deref())?;
    }
    Ok(())
}

fn recovery_for_status(status: &ModelLaneStatus) -> ModelLaneRecoveryState {
    match status {
        ModelLaneStatus::Planned
        | ModelLaneStatus::Ready
        | ModelLaneStatus::Running
        | ModelLaneStatus::Waiting => ModelLaneRecoveryState::Restartable,
        ModelLaneStatus::Failed | ModelLaneStatus::Reclaimable => {
            ModelLaneRecoveryState::Reclaimable
        }
        ModelLaneStatus::Cancelled | ModelLaneStatus::Completed => ModelLaneRecoveryState::Terminal,
    }
}

fn validate_message_trace(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    let parent_span_id = require_optional_token("parent_span_id", input.parent_span_id.as_deref())?;
    if parent_span_id == input.message_span_id {
        return Err(ModelLaneError::InvalidInput(
            "parent_span_id must not equal message_span_id".into(),
        ));
    }
    if input.linked_span_contexts.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "linked_span_contexts must include at least one span".into(),
        ));
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
        if linked == &input.message_span_id {
            return Err(ModelLaneError::InvalidInput(
                "linked_span_contexts must not include message_span_id".into(),
            ));
        }
    }
    Ok(())
}

fn validate_message_routing(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    if let ModelLaneTarget::Lane(lane_id) = &input.to_lane {
        require_token("to_lane.lane_id", lane_id)?;
    }
    let routing = input
        .routing
        .as_ref()
        .ok_or_else(|| ModelLaneError::InvalidInput("routing metadata is required".into()))?;
    require_token("routing.target_role", &routing.target_role)?;
    require_token("routing.target_session", &routing.target_session)?;
    require_token("routing.correlation_id", &routing.correlation_id)?;
    if let Some(ack_for) = routing.ack_for.as_deref() {
        require_token("routing.ack_for", ack_for)?;
    }
    Ok(())
}

fn validate_message_authority(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    if input.kind == ModelLaneMessageKind::Proposal {
        require_optional_token("proposal_ref", input.proposal_ref.as_deref())?;
        require_optional_token("crdt_update_ref", input.crdt_update_ref.as_deref())?;
        require_optional_token(
            "crdt_base_snapshot_ref",
            input.crdt_base_snapshot_ref.as_deref(),
        )?;
        require_optional_token("crdt_state_vector", input.crdt_state_vector.as_deref())?;
        require_optional_token("crdt_proposal_ref", input.crdt_proposal_ref.as_deref())?;
    }
    if matches!(
        input.kind,
        ModelLaneMessageKind::ToolRequest | ModelLaneMessageKind::ToolResult
    ) && input.tool_gate_decision_refs.is_empty()
    {
        return Err(ModelLaneError::InvalidInput(
            "tool messages require tool_gate_decision_refs".into(),
        ));
    }
    match input.authority {
        ModelLaneAuthority::Advisory => Ok(()),
        ModelLaneAuthority::PromotionCandidate => {
            require_optional_token("proposal_ref", input.proposal_ref.as_deref())?;
            require_optional_token("promotion_gate_ref", input.promotion_gate_ref.as_deref())?;
            Ok(())
        }
        ModelLaneAuthority::Promoted => {
            require_optional_token(
                "promotion_decision_id",
                input.promotion_decision_id.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_decision_id is required"
                        .into(),
                )
            })?;
            require_optional_token("promotion_gate_ref", input.promotion_gate_ref.as_deref())
                .map_err(|_| {
                    ModelLaneError::InvalidInput(
                        "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_gate_ref is required"
                            .into(),
                    )
                })?;
            require_optional_token(
                "promotion_receipt_ref",
                input.promotion_receipt_ref.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_receipt_ref is required"
                        .into(),
                )
            })?;
            require_optional_token(
                "promoted_artifact_ref",
                input.promoted_artifact_ref.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_ref is required"
                        .into(),
                )
            })?;
            validate_sha256(
                "promoted_artifact_sha256",
                require_optional_token(
                    "promoted_artifact_sha256",
                    input.promoted_artifact_sha256.as_deref(),
                )?
                .as_str(),
            )?;
            require_optional_token(
                "promoted_artifact_version",
                input.promoted_artifact_version.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_version is required"
                        .into(),
                )
            })?;
            Ok(())
        }
        ModelLaneAuthority::OperatorDecision => {
            require_optional_token(
                "operator_decision_ref",
                input.operator_decision_ref.as_deref(),
            )?;
            Ok(())
        }
        ModelLaneAuthority::ValidatorVerdict => {
            require_optional_token(
                "validator_verdict_ref",
                input.validator_verdict_ref.as_deref(),
            )?;
            Ok(())
        }
    }
}

fn require_token(field: &str, value: &str) -> ModelLaneResult<()> {
    if value.trim().is_empty() {
        return Err(ModelLaneError::InvalidInput(format!("{field} is required")));
    }
    if value.len() > 512 {
        return Err(ModelLaneError::InvalidInput(format!(
            "{field} exceeds 512 bytes"
        )));
    }
    Ok(())
}

fn require_optional_token(field: &str, value: Option<&str>) -> ModelLaneResult<String> {
    let value =
        value.ok_or_else(|| ModelLaneError::InvalidInput(format!("{field} is required")))?;
    require_token(field, value)?;
    Ok(value.to_string())
}

fn require_equal(
    left_field: &str,
    left: &str,
    right_field: &str,
    right: &str,
) -> ModelLaneResult<()> {
    if left == right {
        return Ok(());
    }
    Err(ModelLaneError::InvalidInput(format!(
        "{left_field} must match {right_field}"
    )))
}

fn validate_sha256(field: &str, value: &str) -> ModelLaneResult<()> {
    if value.len() == 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    {
        return Ok(());
    }
    Err(ModelLaneError::InvalidInput(format!(
        "{field} must be lowercase sha256 hex"
    )))
}

fn row_to_json(row: sqlx::postgres::PgRow, column: &str) -> ModelLaneResult<Value> {
    row.try_get::<Value, _>(column)
        .map_err(ModelLaneError::from)
}
