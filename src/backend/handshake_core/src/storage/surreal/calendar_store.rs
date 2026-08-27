use std::str::FromStr;

use chrono::NaiveDate;
use serde_json::Value;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::{event_ledger, SurrealStorage};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::{
    validate_calendar_event_contract, validate_calendar_source_contract, CalendarEvent,
    CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert, CalendarEventVisibility,
    CalendarEventWindowQuery, CalendarNormalizationNote, CalendarSource,
    CalendarSourceProviderType, CalendarSourceSyncState, CalendarSourceUpsert,
    CalendarSourceWritePolicy, CalendarSyncStateStage, StorageError, StorageResult, WriteActorKind,
    WriteContext,
};

const SOURCES: &str = "calendar_sources";
const EVENTS: &str = "calendar_events";
const WORKSPACES: &str = "workspaces";

#[derive(SurrealValue)]
struct SourceRow {
    id: RecordId,
    workspace_id: RecordId,
    display_name: String,
    provider_type: String,
    write_policy: String,
    default_tzid: String,
    auto_export: bool,
    credentials_ref: Option<String>,
    provider_calendar_id: Option<String>,
    capability_profile_id: Option<String>,
    config_json: Value,
    sync_state: Option<String>,
    sync_token: Option<String>,
    last_sync_ts: Option<Datetime>,
    last_full_sync_ts: Option<Datetime>,
    last_ok_at: Option<Datetime>,
    last_pull_at: Option<Datetime>,
    last_push_at: Option<Datetime>,
    last_error_at: Option<Datetime>,
    last_error_code: Option<String>,
    last_error: Option<String>,
    backoff_until: Option<Datetime>,
    consecutive_failures: Option<i64>,
    last_remote_watermark: Option<String>,
    last_local_applied_rev: Option<i64>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct EventRow {
    id: RecordId,
    workspace_id: RecordId,
    source_id: RecordId,
    external_id: Option<String>,
    external_etag: Option<String>,
    title: String,
    description: Option<String>,
    location: Option<String>,
    start_ts_utc: Datetime,
    end_ts_utc: Datetime,
    start_local: Option<String>,
    end_local: Option<String>,
    tzid: String,
    all_day: bool,
    start_date: Option<String>,
    end_date_exclusive: Option<String>,
    was_floating: bool,
    normalization_note: Option<Value>,
    status: String,
    visibility: String,
    export_mode: String,
    rrule: Option<String>,
    rdate_json: Vec<String>,
    exdate_json: Vec<String>,
    is_recurring: bool,
    series_id: Option<String>,
    instance_key: Option<String>,
    is_override: bool,
    source_last_seen_at: Option<Datetime>,
    created_by: Option<String>,
    attendees_json: Value,
    links_json: Value,
    provider_payload_json: Option<Value>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct SourceWriteBindings {
    source: RecordId,
    workspace: RecordId,
    display_name: String,
    provider_type: String,
    write_policy: String,
    default_tzid: String,
    auto_export: bool,
    credentials_ref: Option<String>,
    provider_calendar_id: Option<String>,
    capability_profile_id: Option<String>,
    config_json: Value,
    sync_state: Option<String>,
    sync_token: Option<String>,
    last_sync_ts: Option<Datetime>,
    last_full_sync_ts: Option<Datetime>,
    last_ok_at: Option<Datetime>,
    last_pull_at: Option<Datetime>,
    last_push_at: Option<Datetime>,
    last_error_at: Option<Datetime>,
    last_error_code: Option<String>,
    last_error: Option<String>,
    backoff_until: Option<Datetime>,
    consecutive_failures: Option<i64>,
    last_remote_watermark: Option<String>,
    last_local_applied_rev: Option<i64>,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
    now: Datetime,
}

#[derive(SurrealValue)]
struct WorkspaceBindings {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct SourceLookupBindings {
    workspace: RecordId,
    source: RecordId,
}

#[derive(SurrealValue)]
struct ExternalLookupBindings {
    source: RecordId,
    external_id: String,
}

#[derive(Clone, SurrealValue)]
struct EventWriteBindings {
    event: RecordId,
    workspace: RecordId,
    source: RecordId,
    external_id: Option<String>,
    external_etag: Option<String>,
    title: String,
    description: Option<String>,
    location: Option<String>,
    start_ts_utc: Datetime,
    end_ts_utc: Datetime,
    start_local: Option<String>,
    end_local: Option<String>,
    tzid: String,
    all_day: bool,
    start_date: Option<String>,
    end_date_exclusive: Option<String>,
    was_floating: bool,
    normalization_note: Option<Value>,
    status: String,
    visibility: String,
    export_mode: String,
    rrule: Option<String>,
    rdate_json: Vec<String>,
    exdate_json: Vec<String>,
    is_recurring: bool,
    series_id: Option<String>,
    instance_key: Option<String>,
    is_override: bool,
    source_last_seen_at: Option<Datetime>,
    created_by: Option<String>,
    attendees_json: Value,
    links_json: Value,
    provider_payload_json: Option<Value>,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
    now: Datetime,
}

#[derive(Clone, SurrealValue)]
struct CalendarMutationOutboxWrite {
    record: RecordId,
    idempotency_key: String,
    workspace_id: String,
    source_id: String,
    calendar_event_id: String,
    job_id: Option<String>,
    workflow_id: Option<String>,
    actor_kind: String,
    actor_id: Option<String>,
    edit_event_id: String,
    ledger_event_id: RecordId,
    payload: Value,
    created_at: Datetime,
}

#[derive(Clone, SurrealValue)]
struct EventTransactionBindings {
    event: EventWriteBindings,
    expected_source_tzid: String,
    external_insert: bool,
    ledger: event_ledger::LedgerWrite,
    outbox: CalendarMutationOutboxWrite,
    force_failure_after_event_upsert: bool,
}

#[derive(SurrealValue)]
struct EventWindowBindings {
    workspace: RecordId,
    query_start_date: String,
    query_end_date_exclusive: String,
    window_start_utc: Datetime,
    window_end_utc: Datetime,
    sources: Vec<RecordId>,
}

pub(crate) async fn upsert_source(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    source: CalendarSourceUpsert,
) -> StorageResult<CalendarSource> {
    validate_calendar_source_contract(&source)?;
    if !source.config.is_object() {
        return Err(StorageError::Validation(
            "calendar source config must be an object",
        ));
    }
    let metadata = storage
        .inner
        .guard
        .validate_write(ctx, &source.id)
        .await
        .map_err(StorageError::from)?;
    let state = source.sync_state;
    let bindings = SourceWriteBindings {
        source: RecordId::new(SOURCES, source.id),
        workspace: RecordId::new(WORKSPACES, source.workspace_id),
        display_name: source.display_name,
        provider_type: source.provider_type.as_str().to_owned(),
        write_policy: source.write_policy.as_str().to_owned(),
        default_tzid: source.default_tzid,
        auto_export: source.auto_export,
        credentials_ref: source.credentials_ref,
        provider_calendar_id: source.provider_calendar_id,
        capability_profile_id: source.capability_profile_id,
        config_json: source.config,
        sync_state: state.state.map(|value| value.as_str().to_owned()),
        sync_token: state.sync_token,
        last_sync_ts: state.last_synced_at.map(Datetime::from),
        last_full_sync_ts: state.last_full_sync_at.map(Datetime::from),
        last_ok_at: state.last_ok_at.map(Datetime::from),
        last_pull_at: state.last_pull_at.map(Datetime::from),
        last_push_at: state.last_push_at.map(Datetime::from),
        last_error_at: state.last_error_at.map(Datetime::from),
        last_error_code: state.last_error_code,
        last_error: state.last_error,
        backoff_until: state.backoff_until.map(Datetime::from),
        consecutive_failures: state.consecutive_failures,
        last_remote_watermark: state.last_remote_watermark,
        last_local_applied_rev: state.last_local_applied_rev,
        actor_kind: metadata.actor_kind.as_str().to_owned(),
        actor_id: metadata.actor_id,
        job_id: metadata.job_id.map(|value| value.to_string()),
        workflow_id: metadata.workflow_id.map(|value| value.to_string()),
        edit_event_id: metadata.edit_event_id.to_string(),
        now: Datetime::from(metadata.timestamp),
    };
    let row: Option<SourceRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database.query_first(
                    "RETURN IF $source.workspace_id != NONE AND $source.workspace_id != $workspace { \
                       THROW 'HSK-409-CALENDAR-SOURCE-WORKSPACE'; \
                     } ELSE { UPSERT $source SET workspace_id = $workspace, display_name = $display_name, \
                     provider_type = $provider_type, write_policy = $write_policy, default_tzid = $default_tzid, \
                     auto_export = $auto_export, credentials_ref = $credentials_ref, \
                     provider_calendar_id = $provider_calendar_id, capability_profile_id = $capability_profile_id, \
                     config_json = $config_json, sync_state = $sync_state, sync_token = $sync_token, \
                     last_sync_ts = $last_sync_ts, last_full_sync_ts = $last_full_sync_ts, last_ok_at = $last_ok_at, \
                     last_pull_at = $last_pull_at, last_push_at = $last_push_at, last_error_at = $last_error_at, \
                     last_error_code = $last_error_code, last_error = $last_error, backoff_until = $backoff_until, \
                     consecutive_failures = $consecutive_failures, last_remote_watermark = $last_remote_watermark, \
                     last_local_applied_rev = $last_local_applied_rev, last_actor_kind = $actor_kind, \
                     last_actor_id = $actor_id, last_job_id = $job_id, last_workflow_id = $workflow_id, \
                     edit_event_id = $edit_event_id, updated_at = $now RETURN AFTER; };",
                    bindings,
                ).await
            })
        })
        .await
        .map_err(|error| {
            if error
                .to_string()
                .contains("HSK-409-CALENDAR-SOURCE-WORKSPACE")
            {
                StorageError::Conflict("HSK-409-CALENDAR-SOURCE-WORKSPACE")
            } else {
                StorageError::from(error)
            }
        })?;
    row.map(map_source)
        .transpose()?
        .ok_or_else(|| StorageError::Database("calendar source upsert returned no row".to_owned()))
}

pub(crate) async fn list_sources(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Vec<CalendarSource>> {
    let bindings = WorkspaceBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    let rows: Vec<SourceRow> = storage
        .with_data_operation(move |database| Box::pin(async move {
            database.query_values(
                "SELECT * FROM calendar_sources WHERE workspace_id = $workspace ORDER BY display_name ASC, id ASC;",
                bindings,
            ).await
        }))
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_source).collect()
}

pub(crate) async fn get_source(
    storage: &SurrealStorage,
    workspace_id: &str,
    source_id: &str,
) -> StorageResult<Option<CalendarSource>> {
    let bindings = SourceLookupBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        source: RecordId::new(SOURCES, source_id.to_owned()),
    };
    let row: Option<SourceRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM $source WHERE workspace_id = $workspace LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_source).transpose()
}

pub(crate) async fn upsert_event(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    event: CalendarEventUpsert,
) -> StorageResult<CalendarEvent> {
    upsert_event_inner(storage, ctx, event, false, None).await
}

async fn upsert_event_inner(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    event: CalendarEventUpsert,
    force_failure_after_event_upsert: bool,
    race_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
) -> StorageResult<CalendarEvent> {
    let source = get_source(storage, &event.workspace_id, &event.source_id)
        .await?
        .ok_or(StorageError::NotFound("calendar_source"))?;
    validate_calendar_event_contract(&event, &source.default_tzid)?;
    if !event.attendees.is_array() || !event.links.is_array() {
        return Err(StorageError::Validation(
            "calendar attendees and links must be arrays",
        ));
    }
    if event
        .provider_payload
        .as_ref()
        .is_some_and(|payload| !payload.is_object())
    {
        return Err(StorageError::Validation(
            "calendar provider payload must be an object",
        ));
    }
    let metadata = storage
        .inner
        .guard
        .validate_write(ctx, &event.id)
        .await
        .map_err(StorageError::from)?;

    let workspace_id = event.workspace_id.clone();
    let source_id = event.source_id.clone();
    let requested_event_id = event.id.clone();
    let actor_kind = metadata.actor_kind.as_str().to_owned();
    let actor_id = metadata.actor_id.clone();
    let job_id = metadata.job_id.map(|value| value.to_string());
    let workflow_id = metadata.workflow_id.map(|value| value.to_string());
    let edit_event_id = metadata.edit_event_id.to_string();
    let mutation_payload = serde_json::json!({
        "type": "calendar_mutation",
        "action": "upsert_event",
        "workspace_id": workspace_id,
        "source_id": source_id,
        "event_id": requested_event_id,
        "job_id": job_id,
        "workflow_id": workflow_id,
        "edit_event_id": edit_event_id,
        "actor_kind": actor_kind,
        "actor_id": actor_id,
        "start_ts_utc": event.start_ts_utc,
        "end_ts_utc": event.end_ts_utc,
        "start_date": event.start_date,
        "end_date_exclusive": event.end_date_exclusive,
        "tzid": event.tzid,
        "all_day": event.all_day,
        "was_floating": event.was_floating,
    });
    let source_record = RecordId::new(SOURCES, source_id.clone());
    let existing_external = match event.external_id.as_deref() {
        Some(external_id) => lookup_external_event(storage, &source_record, external_id).await?,
        None => None,
    };
    let external_insert = event.external_id.is_some() && existing_external.is_none();
    let target_id =
        existing_external.unwrap_or_else(|| RecordId::new(EVENTS, requested_event_id.clone()));
    if let Some(barrier) = race_barrier {
        barrier.wait().await;
    }
    let normalization_note = event
        .normalization_note
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let persisted_event_id = key_ref(&target_id)?.to_owned();
    let run_id = job_id
        .clone()
        .or_else(|| workflow_id.clone())
        .unwrap_or_else(|| edit_event_id.clone());
    let mut receipt = NewKernelEvent::builder(
        run_id.clone(),
        workflow_id.clone().unwrap_or_else(|| run_id.clone()),
        KernelEventType::AtelierDomainEventRecorded,
        kernel_actor_for_calendar(ctx),
    )
    .aggregate("calendar_event", persisted_event_id.clone())
    .idempotency_key(format!("KEI-calendar-mutation-{edit_event_id}"))
    .source_component("calendar_workflow")
    .payload(mutation_payload);
    if let Some(workflow_id) = workflow_id.as_deref() {
        receipt = receipt.correlation_id(workflow_id);
    }
    let receipt = receipt
        .build()
        .map_err(|_| StorageError::Validation("calendar mutation receipt build failed"))?;
    let (_, ledger) = event_ledger::prepare_event(receipt)?;
    let outbox_idempotency_key = format!("calendar-mutation-{edit_event_id}");
    let outbox = CalendarMutationOutboxWrite {
        record: RecordId::new("calendar_mutation_outbox", outbox_idempotency_key.clone()),
        idempotency_key: outbox_idempotency_key,
        workspace_id: workspace_id.clone(),
        source_id: source_id.clone(),
        calendar_event_id: persisted_event_id.clone(),
        job_id: job_id.clone(),
        workflow_id: workflow_id.clone(),
        actor_kind: actor_kind.clone(),
        actor_id: actor_id.clone(),
        edit_event_id: edit_event_id.clone(),
        ledger_event_id: ledger.record.clone(),
        payload: serde_json::json!({
            "message": "calendar_mutation",
            "event_id": persisted_event_id,
            "workspace_id": workspace_id,
            "source_id": source_id,
            "job_id": job_id,
            "workflow_id": workflow_id,
            "edit_event_id": edit_event_id,
            "ledger_event_id": ledger.event_id,
        }),
        created_at: Datetime::from(metadata.timestamp),
    };
    let event_write = EventWriteBindings {
        event: target_id,
        workspace: RecordId::new(WORKSPACES, event.workspace_id),
        source: source_record.clone(),
        external_id: event.external_id,
        external_etag: event.external_etag,
        title: event.title,
        description: event.description,
        location: event.location,
        start_ts_utc: Datetime::from(event.start_ts_utc),
        end_ts_utc: Datetime::from(event.end_ts_utc),
        start_local: event.start_local,
        end_local: event.end_local,
        tzid: event.tzid,
        all_day: event.all_day,
        start_date: event.start_date.map(|date| date.to_string()),
        end_date_exclusive: event.end_date_exclusive.map(|date| date.to_string()),
        was_floating: event.was_floating,
        normalization_note,
        status: event.status.as_str().to_owned(),
        visibility: event.visibility.as_str().to_owned(),
        export_mode: event.export_mode.as_str().to_owned(),
        rrule: event.rrule,
        rdate_json: event.rdate,
        exdate_json: event.exdate,
        is_recurring: event.is_recurring,
        series_id: event.series_id,
        instance_key: event.instance_key,
        is_override: event.is_override,
        source_last_seen_at: event.source_last_seen_at.map(Datetime::from),
        created_by: actor_id.clone(),
        attendees_json: event.attendees,
        links_json: event.links,
        provider_payload_json: event.provider_payload,
        actor_kind,
        actor_id,
        job_id,
        workflow_id,
        edit_event_id,
        now: Datetime::from(metadata.timestamp),
    };
    let mut transaction = EventTransactionBindings {
        event: event_write,
        expected_source_tzid: source.default_tzid,
        external_insert,
        ledger,
        outbox,
        force_failure_after_event_upsert,
    };
    match run_event_transaction(storage, transaction.clone()).await {
        Ok(row) => map_event(row),
        Err(first_error) if transaction.external_insert => {
            if let Some(external_id) = transaction.event.external_id.as_deref() {
                if let Some(winner) =
                    lookup_external_event(storage, &source_record, external_id).await?
                {
                    if winner != transaction.event.event {
                        transaction.event.event = winner.clone();
                        transaction.external_insert = false;
                        transaction.outbox.calendar_event_id = key_ref(&winner)?.to_owned();
                        transaction.outbox.payload["event_id"] =
                            Value::String(transaction.outbox.calendar_event_id.clone());
                        transaction.ledger.aggregate_id =
                            transaction.outbox.calendar_event_id.clone();
                        return run_event_transaction(storage, transaction)
                            .await
                            .map_err(map_calendar_transaction_error)
                            .and_then(map_event);
                    }
                }
            }
            Err(map_calendar_transaction_error(first_error))
        }
        Err(error) => Err(map_calendar_transaction_error(error)),
    }
}

async fn lookup_external_event(
    storage: &SurrealStorage,
    source: &RecordId,
    external_id: &str,
) -> StorageResult<Option<RecordId>> {
    let bindings = ExternalLookupBindings {
        source: source.clone(),
        external_id: external_id.to_owned(),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT VALUE id FROM calendar_events WHERE source_id = $source \
                         AND external_id = $external_id LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

async fn run_event_transaction(
    storage: &SurrealStorage,
    bindings: EventTransactionBindings,
) -> StorageResult<EventRow> {
    let rows: Vec<EventRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $event.source)[0] = NONE { \
                            THROW 'HSK-CALENDAR-SOURCE-MISSING'; \
                         }; \
                         IF (SELECT VALUE workspace_id FROM $event.source)[0] != $event.workspace { \
                            THROW 'HSK-CALENDAR-SOURCE-WORKSPACE'; \
                         }; \
                         IF (SELECT VALUE default_tzid FROM $event.source)[0] != $expected_source_tzid { \
                            THROW 'HSK-CALENDAR-SOURCE-TZID-CHANGED'; \
                         }; \
                         IF $external_insert AND (SELECT VALUE id FROM $event.event)[0] != NONE { \
                            THROW 'HSK-CALENDAR-EVENT-ID-CONFLICT'; \
                         }; \
                         UPSERT $event.event SET workspace_id = $event.workspace, source_id = $event.source, \
                            external_id = $event.external_id, external_etag = $event.external_etag, \
                            title = $event.title, description = $event.description, location = $event.location, \
                            start_ts_utc = $event.start_ts_utc, end_ts_utc = $event.end_ts_utc, \
                            start_local = $event.start_local, end_local = $event.end_local, tzid = $event.tzid, \
                            all_day = $event.all_day, start_date = $event.start_date, \
                            end_date_exclusive = $event.end_date_exclusive, was_floating = $event.was_floating, \
                            normalization_note = $event.normalization_note, \
                            temporal_contract_version = 'calendar-v02.201', status = $event.status, \
                            visibility = $event.visibility, export_mode = $event.export_mode, rrule = $event.rrule, \
                            rdate_json = $event.rdate_json, exdate_json = $event.exdate_json, \
                            is_recurring = $event.is_recurring, series_id = $event.series_id, \
                            instance_key = $event.instance_key, is_override = $event.is_override, \
                            source_last_seen_at = $event.source_last_seen_at, \
                            created_by = created_by ?? $event.created_by, attendees_json = $event.attendees_json, \
                            links_json = $event.links_json, provider_payload_json = $event.provider_payload_json, \
                            last_actor_kind = $event.actor_kind, last_actor_id = $event.actor_id, \
                            last_job_id = $event.job_id, last_workflow_id = $event.workflow_id, \
                            edit_event_id = $event.edit_event_id, updated_at = $event.now RETURN AFTER; \
                         IF $force_failure_after_event_upsert { \
                            THROW 'HSK-CALENDAR-FORCED-ROLLBACK'; \
                         }; \
                         CREATE $ledger.record CONTENT { \
                            event_id: $ledger.event_id, event_version: $ledger.event_version, \
                            kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, \
                            aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, \
                            idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, \
                            actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, \
                            causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, \
                            payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, \
                            payload: $ledger.payload, created_at: $ledger.created_at \
                         }; \
                         CREATE $outbox.record CONTENT { \
                            idempotency_key: $outbox.idempotency_key, workspace_id: $outbox.workspace_id, \
                            source_id: $outbox.source_id, calendar_event_id: $outbox.calendar_event_id, \
                            job_id: $outbox.job_id, workflow_id: $outbox.workflow_id, \
                            actor_kind: $outbox.actor_kind, actor_id: $outbox.actor_id, \
                            edit_event_id: $outbox.edit_event_id, ledger_event_id: $outbox.ledger_event_id, \
                            payload: $outbox.payload, created_at: $outbox.created_at \
                         }; \
                         COMMIT TRANSACTION;",
                        bindings,
                        5,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    match rows.as_slice() {
        [row] => Ok(EventRow {
            id: row.id.clone(),
            workspace_id: row.workspace_id.clone(),
            source_id: row.source_id.clone(),
            external_id: row.external_id.clone(),
            external_etag: row.external_etag.clone(),
            title: row.title.clone(),
            description: row.description.clone(),
            location: row.location.clone(),
            start_ts_utc: row.start_ts_utc.clone(),
            end_ts_utc: row.end_ts_utc.clone(),
            start_local: row.start_local.clone(),
            end_local: row.end_local.clone(),
            tzid: row.tzid.clone(),
            all_day: row.all_day,
            start_date: row.start_date.clone(),
            end_date_exclusive: row.end_date_exclusive.clone(),
            was_floating: row.was_floating,
            normalization_note: row.normalization_note.clone(),
            status: row.status.clone(),
            visibility: row.visibility.clone(),
            export_mode: row.export_mode.clone(),
            rrule: row.rrule.clone(),
            rdate_json: row.rdate_json.clone(),
            exdate_json: row.exdate_json.clone(),
            is_recurring: row.is_recurring,
            series_id: row.series_id.clone(),
            instance_key: row.instance_key.clone(),
            is_override: row.is_override,
            source_last_seen_at: row.source_last_seen_at.clone(),
            created_by: row.created_by.clone(),
            attendees_json: row.attendees_json.clone(),
            links_json: row.links_json.clone(),
            provider_payload_json: row.provider_payload_json.clone(),
            last_job_id: row.last_job_id.clone(),
            last_workflow_id: row.last_workflow_id.clone(),
            last_actor_id: row.last_actor_id.clone(),
            edit_event_id: row.edit_event_id.clone(),
            last_actor_kind: row.last_actor_kind.clone(),
            created_at: row.created_at.clone(),
            updated_at: row.updated_at.clone(),
        }),
        _ => Err(StorageError::Database(
            "calendar transaction did not return exactly one event row".to_owned(),
        )),
    }
}

fn map_calendar_transaction_error(error: StorageError) -> StorageError {
    let message = error.to_string();
    if message.contains("HSK-CALENDAR-SOURCE-MISSING") {
        StorageError::NotFound("calendar_source")
    } else if message.contains("HSK-CALENDAR-SOURCE-WORKSPACE") {
        StorageError::Validation("calendar event source belongs to a different workspace")
    } else if message.contains("HSK-CALENDAR-SOURCE-TZID-CHANGED") {
        StorageError::Conflict("calendar source timezone changed during event upsert")
    } else if message.contains("HSK-CALENDAR-EVENT-ID-CONFLICT") {
        StorageError::Conflict("calendar event id already belongs to a different external event")
    } else {
        error
    }
}

fn kernel_actor_for_calendar(ctx: &WriteContext) -> KernelActor {
    let actor_id = ctx
        .actor_id
        .clone()
        .unwrap_or_else(|| "calendar_workflow".to_owned());
    match ctx.actor_kind {
        WriteActorKind::Human => KernelActor::Operator(actor_id),
        WriteActorKind::Ai => KernelActor::ModelAdapter(actor_id),
        WriteActorKind::System => KernelActor::System(actor_id),
    }
}

pub(crate) async fn query_events(
    storage: &SurrealStorage,
    query: CalendarEventWindowQuery,
) -> StorageResult<Vec<CalendarEvent>> {
    if query.workspace_id.trim().is_empty()
        || query.query_end_date_exclusive <= query.query_start_date
        || query.window_end_utc <= query.window_start_utc
    {
        return Err(StorageError::Validation("invalid calendar event window"));
    }
    let bindings = EventWindowBindings {
        workspace: RecordId::new(WORKSPACES, query.workspace_id),
        query_start_date: query.query_start_date.to_string(),
        query_end_date_exclusive: query.query_end_date_exclusive.to_string(),
        window_start_utc: Datetime::from(query.window_start_utc),
        window_end_utc: Datetime::from(query.window_end_utc),
        sources: query
            .source_ids
            .into_iter()
            .map(|id| RecordId::new(SOURCES, id))
            .collect(),
    };
    let rows: Vec<EventRow> = storage
        .with_data_operation(move |database| Box::pin(async move {
            database.query_values(
                "SELECT * FROM calendar_events WHERE workspace_id = $workspace \
                 AND ((all_day = false AND start_ts_utc < $window_end_utc AND end_ts_utc > $window_start_utc) \
                   OR (all_day = true AND start_date < $query_end_date_exclusive \
                       AND end_date_exclusive > $query_start_date)) \
                 AND (array::len($sources) = 0 OR source_id IN $sources) \
                 ORDER BY start_ts_utc ASC, end_ts_utc ASC, id ASC;",
                bindings,
            ).await
        }))
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_event).collect()
}

pub(crate) async fn delete_source(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    source_id: &str,
) -> StorageResult<()> {
    storage
        .inner
        .guard
        .validate_write(ctx, source_id)
        .await
        .map_err(StorageError::from)?;
    let bindings = SourceLookupBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        source: RecordId::new(SOURCES, source_id.to_owned()),
    };
    let rows: Vec<SourceRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "DELETE $source WHERE workspace_id = $workspace RETURN BEFORE;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    if rows.is_empty() {
        Err(StorageError::NotFound("calendar_source"))
    } else {
        Ok(())
    }
}

fn key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "embedded calendar record has a non-string id".to_owned(),
        )),
    }
}

fn key_ref(record: &RecordId) -> StorageResult<&str> {
    match &record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "embedded calendar record has a non-string id".to_owned(),
        )),
    }
}

fn map_source(row: SourceRow) -> StorageResult<CalendarSource> {
    Ok(CalendarSource {
        id: key(row.id)?,
        workspace_id: key(row.workspace_id)?,
        display_name: row.display_name,
        provider_type: CalendarSourceProviderType::from_str(&row.provider_type)?,
        write_policy: CalendarSourceWritePolicy::from_str(&row.write_policy)?,
        default_tzid: row.default_tzid,
        auto_export: row.auto_export,
        credentials_ref: row.credentials_ref,
        provider_calendar_id: row.provider_calendar_id,
        capability_profile_id: row.capability_profile_id,
        config: row.config_json,
        sync_state: CalendarSourceSyncState {
            state: row
                .sync_state
                .as_deref()
                .map(CalendarSyncStateStage::from_str)
                .transpose()?,
            sync_token: row.sync_token,
            last_synced_at: row.last_sync_ts.map(|value| value.into_inner()),
            last_full_sync_at: row.last_full_sync_ts.map(|value| value.into_inner()),
            last_ok_at: row.last_ok_at.map(|value| value.into_inner()),
            last_pull_at: row.last_pull_at.map(|value| value.into_inner()),
            last_push_at: row.last_push_at.map(|value| value.into_inner()),
            last_error_at: row.last_error_at.map(|value| value.into_inner()),
            last_error_code: row.last_error_code,
            last_error: row.last_error,
            backoff_until: row.backoff_until.map(|value| value.into_inner()),
            consecutive_failures: row.consecutive_failures,
            last_remote_watermark: row.last_remote_watermark,
            last_local_applied_rev: row.last_local_applied_rev,
        },
        last_job_id: row.last_job_id,
        last_workflow_id: row.last_workflow_id,
        last_actor_id: row.last_actor_id,
        edit_event_id: row.edit_event_id,
        last_actor_kind: row.last_actor_kind,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn map_event(row: EventRow) -> StorageResult<CalendarEvent> {
    Ok(CalendarEvent {
        id: key(row.id)?,
        workspace_id: key(row.workspace_id)?,
        source_id: key(row.source_id)?,
        external_id: row.external_id,
        external_etag: row.external_etag,
        title: row.title,
        description: row.description,
        location: row.location,
        start_ts_utc: row.start_ts_utc.into_inner(),
        end_ts_utc: row.end_ts_utc.into_inner(),
        start_local: row.start_local,
        end_local: row.end_local,
        tzid: row.tzid,
        all_day: row.all_day,
        start_date: row
            .start_date
            .as_deref()
            .map(NaiveDate::from_str)
            .transpose()
            .map_err(|_| StorageError::Validation("invalid stored calendar start_date"))?,
        end_date_exclusive: row
            .end_date_exclusive
            .as_deref()
            .map(NaiveDate::from_str)
            .transpose()
            .map_err(|_| StorageError::Validation("invalid stored calendar end_date_exclusive"))?,
        was_floating: row.was_floating,
        normalization_note: row
            .normalization_note
            .map(serde_json::from_value::<CalendarNormalizationNote>)
            .transpose()?,
        status: CalendarEventStatus::from_str(&row.status)?,
        visibility: CalendarEventVisibility::from_str(&row.visibility)?,
        export_mode: CalendarEventExportMode::from_str(&row.export_mode)?,
        rrule: row.rrule,
        rdate: row.rdate_json,
        exdate: row.exdate_json,
        is_recurring: row.is_recurring,
        series_id: row.series_id,
        instance_key: row.instance_key,
        is_override: row.is_override,
        source_last_seen_at: row.source_last_seen_at.map(|value| value.into_inner()),
        created_by: row.created_by,
        attendees: row.attendees_json,
        links: row.links_json,
        provider_payload: row.provider_payload_json,
        last_job_id: row.last_job_id,
        last_workflow_id: row.last_workflow_id,
        last_actor_id: row.last_actor_id,
        edit_event_id: row.edit_event_id,
        last_actor_kind: row.last_actor_kind,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{surreal::schema, CalendarSourceSyncState, NewWorkspace};
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    #[derive(SurrealValue)]
    struct OutboxProofBindings {
        workspace_id: String,
        source_id: String,
    }

    #[derive(SurrealValue)]
    struct OutboxProofRow {
        idempotency_key: String,
        workspace_id: String,
        source_id: String,
        calendar_event_id: String,
        edit_event_id: String,
        ledger_event_id: RecordId,
        payload: Value,
    }

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            super::super::SurrealStorageConfig::with_path(path).expect("valid embedded test path"),
        )
        .await
        .expect("open embedded SurrealDB");
        schema::bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded schema");
        storage
    }

    async fn create_calendar_fixture(storage: &SurrealStorage) -> (String, String) {
        let ctx = WriteContext::system(Some("mt-136-calendar-fixture".to_owned()));
        let workspace = storage
            .create_workspace(
                &ctx,
                NewWorkspace {
                    name: "MT-136 calendar authority".to_owned(),
                },
            )
            .await
            .expect("create calendar workspace");
        let source_id = "mt-136-calendar-source".to_owned();
        upsert_source(
            storage,
            &ctx,
            CalendarSourceUpsert {
                id: source_id.clone(),
                workspace_id: workspace.id.clone(),
                display_name: "MT-136 calendar".to_owned(),
                provider_type: CalendarSourceProviderType::Local,
                write_policy: CalendarSourceWritePolicy::TwoWayMirror,
                default_tzid: "UTC".to_owned(),
                auto_export: false,
                credentials_ref: None,
                provider_calendar_id: None,
                capability_profile_id: None,
                config: json!({}),
                sync_state: CalendarSourceSyncState::default(),
            },
        )
        .await
        .expect("create calendar source");
        (workspace.id, source_id)
    }

    fn event(
        id: &str,
        workspace_id: &str,
        source_id: &str,
        external_id: &str,
        title: &str,
    ) -> CalendarEventUpsert {
        CalendarEventUpsert {
            id: id.to_owned(),
            workspace_id: workspace_id.to_owned(),
            source_id: source_id.to_owned(),
            external_id: Some(external_id.to_owned()),
            external_etag: Some(format!("etag-{title}")),
            title: title.to_owned(),
            description: None,
            location: None,
            start_ts_utc: Utc.with_ymd_and_hms(2026, 8, 20, 9, 0, 0).unwrap(),
            end_ts_utc: Utc.with_ymd_and_hms(2026, 8, 20, 10, 0, 0).unwrap(),
            start_local: Some("2026-08-20T09:00:00".to_owned()),
            end_local: Some("2026-08-20T10:00:00".to_owned()),
            tzid: "UTC".to_owned(),
            all_day: false,
            start_date: None,
            end_date_exclusive: None,
            was_floating: false,
            normalization_note: None,
            status: CalendarEventStatus::Confirmed,
            visibility: CalendarEventVisibility::Private,
            export_mode: CalendarEventExportMode::FullExport,
            rrule: None,
            rdate: Vec::new(),
            exdate: Vec::new(),
            is_recurring: false,
            series_id: None,
            instance_key: None,
            is_override: false,
            source_last_seen_at: None,
            attendees: json!([]),
            links: json!([]),
            provider_payload: None,
        }
    }

    async fn outbox_rows(
        storage: &SurrealStorage,
        workspace_id: &str,
        source_id: &str,
    ) -> Vec<OutboxProofRow> {
        let bindings = OutboxProofBindings {
            workspace_id: workspace_id.to_owned(),
            source_id: source_id.to_owned(),
        };
        storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT idempotency_key, workspace_id, source_id, calendar_event_id, \
                             edit_event_id, ledger_event_id, payload, created_at, id FROM calendar_mutation_outbox \
                             WHERE workspace_id = $workspace_id AND source_id = $source_id \
                             ORDER BY created_at ASC, id ASC;",
                            bindings,
                        )
                        .await
                })
            })
            .await
            .expect("read calendar mutation outbox")
    }

    #[tokio::test]
    async fn event_row_ledger_and_outbox_are_atomic_and_survive_close_reopen() {
        let directory = tempfile::tempdir().expect("temporary MT-136 calendar root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let (workspace_id, source_id) = create_calendar_fixture(&storage).await;
        let ctx = WriteContext::ai(
            Some("calendar_sync".to_owned()),
            Some(uuid::Uuid::now_v7()),
            Some(uuid::Uuid::now_v7()),
        );
        let stored = upsert_event(
            &storage,
            &ctx,
            event(
                "calendar-atomic-event",
                &workspace_id,
                &source_id,
                "provider-atomic-event",
                "Atomic calendar event",
            ),
        )
        .await
        .expect("atomic calendar mutation");
        let outbox = outbox_rows(&storage, &workspace_id, &source_id).await;
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].calendar_event_id, stored.id);
        assert_eq!(
            outbox[0].idempotency_key,
            format!("calendar-mutation-{}", outbox[0].edit_event_id)
        );
        assert_eq!(outbox[0].workspace_id, workspace_id);
        assert_eq!(outbox[0].source_id, source_id);
        assert_eq!(outbox[0].payload["event_id"], stored.id);
        let ledger_key = format!("KEI-calendar-mutation-{}", outbox[0].edit_event_id);
        let ledger = event_ledger::get_by_idempotency(&storage, &ledger_key)
            .await
            .expect("read calendar ledger receipt")
            .expect("calendar ledger receipt exists");
        assert_eq!(ledger.aggregate_type, "calendar_event");
        assert_eq!(ledger.aggregate_id, stored.id);
        assert_eq!(
            ledger.actor,
            KernelActor::ModelAdapter("calendar_sync".to_owned())
        );
        assert_eq!(ledger.source_component, "calendar_workflow");
        assert_eq!(
            key_ref(&outbox[0].ledger_event_id).unwrap(),
            ledger.event_id
        );

        storage
            .shutdown()
            .await
            .expect("close embedded calendar store");
        drop(storage);
        let reopened = open(&path).await;
        let persisted = lookup_external_event(
            &reopened,
            &RecordId::new(SOURCES, source_id.clone()),
            "provider-atomic-event",
        )
        .await
        .expect("read reopened calendar event")
        .expect("calendar event survives reopen");
        assert_eq!(key_ref(&persisted).unwrap(), stored.id);
        assert_eq!(
            outbox_rows(&reopened, &workspace_id, &source_id)
                .await
                .len(),
            1
        );
        assert!(event_ledger::get_by_idempotency(&reopened, &ledger_key)
            .await
            .expect("read reopened calendar ledger")
            .is_some());
        reopened
            .shutdown()
            .await
            .expect("close reopened calendar store");
    }

    #[tokio::test]
    async fn forced_failure_rolls_back_event_ledger_and_outbox() {
        let directory = tempfile::tempdir().expect("temporary MT-136 calendar root");
        let storage = open(&directory.path().join("store")).await;
        let (workspace_id, source_id) = create_calendar_fixture(&storage).await;
        let ctx = WriteContext::system(Some("calendar-rollback-proof".to_owned()));
        let result = upsert_event_inner(
            &storage,
            &ctx,
            event(
                "calendar-rollback-event",
                &workspace_id,
                &source_id,
                "provider-rollback-event",
                "Must roll back",
            ),
            true,
            None,
        )
        .await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HSK-CALENDAR-FORCED-ROLLBACK"));
        assert!(lookup_external_event(
            &storage,
            &RecordId::new(SOURCES, source_id.clone()),
            "provider-rollback-event",
        )
        .await
        .expect("read rolled-back external id")
        .is_none());
        assert!(outbox_rows(&storage, &workspace_id, &source_id)
            .await
            .is_empty());
        assert!(event_ledger::list_for_aggregate(
            &storage,
            "calendar_event",
            "calendar-rollback-event",
        )
        .await
        .expect("read rolled-back calendar ledger")
        .is_empty());
        storage
            .shutdown()
            .await
            .expect("close embedded calendar store");
    }

    #[tokio::test]
    async fn concurrent_external_id_race_converges_and_id_reuse_conflicts() {
        let directory = tempfile::tempdir().expect("temporary MT-136 calendar root");
        let storage = open(&directory.path().join("store")).await;
        let (workspace_id, source_id) = create_calendar_fixture(&storage).await;
        let barrier = Arc::new(Barrier::new(2));
        let left_storage = storage.clone();
        let left_workspace = workspace_id.clone();
        let left_source = source_id.clone();
        let left_barrier = barrier.clone();
        let left = tokio::spawn(async move {
            upsert_event_inner(
                &left_storage,
                &WriteContext::system(Some("calendar-race-left".to_owned())),
                event(
                    "calendar-race-left",
                    &left_workspace,
                    &left_source,
                    "provider-race-event",
                    "Left race value",
                ),
                false,
                Some(left_barrier),
            )
            .await
        });
        let right_storage = storage.clone();
        let right_workspace = workspace_id.clone();
        let right_source = source_id.clone();
        let right = tokio::spawn(async move {
            upsert_event_inner(
                &right_storage,
                &WriteContext::system(Some("calendar-race-right".to_owned())),
                event(
                    "calendar-race-right",
                    &right_workspace,
                    &right_source,
                    "provider-race-event",
                    "Right race value",
                ),
                false,
                Some(barrier),
            )
            .await
        });
        let left = left
            .await
            .expect("left race task")
            .expect("left race write");
        let right = right
            .await
            .expect("right race task")
            .expect("right race write");
        assert_eq!(left.id, right.id);
        let winner = lookup_external_event(
            &storage,
            &RecordId::new(SOURCES, source_id.clone()),
            "provider-race-event",
        )
        .await
        .expect("read external-id winner")
        .expect("external-id winner exists");
        assert_eq!(key_ref(&winner).unwrap(), left.id);
        assert_eq!(
            outbox_rows(&storage, &workspace_id, &source_id).await.len(),
            2
        );
        assert_eq!(
            event_ledger::list_for_aggregate(&storage, "calendar_event", &left.id)
                .await
                .expect("read converged receipts")
                .len(),
            2
        );

        let conflict = upsert_event(
            &storage,
            &WriteContext::system(Some("calendar-id-conflict".to_owned())),
            event(
                &left.id,
                &workspace_id,
                &source_id,
                "provider-different-event",
                "Must conflict",
            ),
        )
        .await;
        assert!(matches!(conflict, Err(StorageError::Conflict(_))));
        assert_eq!(
            outbox_rows(&storage, &workspace_id, &source_id).await.len(),
            2
        );
        storage
            .shutdown()
            .await
            .expect("close embedded calendar store");
    }
}
