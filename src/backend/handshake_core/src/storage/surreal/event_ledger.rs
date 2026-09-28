//! Embedded EventLedger primitives shared by storage-domain transactions.

use surrealdb::types::{Datetime, RecordId, SurrealValue, Value};

use super::SurrealStorage;
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{StorageError, StorageResult};

const EVENT_TABLE: &str = "kernel_event_ledger";

/// MT-109 C3: the authenticated session principal and workspace of an account-scoped Loom route.
#[derive(Clone)]
struct LoomSessionReceipt {
    actor: KernelActor,
    workspace_id: String,
}

tokio::task_local! {
    /// Set only by account-scoped Loom routes. Every receipt prepared inside such a route carries
    /// the session principal as its actor and the route workspace as its single `wsids` entry,
    /// which `fn::mt120_loom_receipt` requires (Master Spec 02-system-architecture.md:2773).
    static LOOM_SESSION_RECEIPT: LoomSessionReceipt;
}

/// Runs `operation` so that its EventLedger receipts are stamped with the session principal
/// `actor` and bound to `workspace_id` (MT-109 C3).
pub(crate) async fn with_loom_session_receipt<T>(
    actor: KernelActor,
    workspace_id: String,
    operation: impl std::future::Future<Output = T>,
) -> T {
    LOOM_SESSION_RECEIPT
        .scope(
            LoomSessionReceipt {
                actor,
                workspace_id,
            },
            operation,
        )
        .await
}

/// MT-153/MT-156: the account authority active on the current task (record-user scope plus the
/// session receipt principal), captured so work the request hands to a spawned task (a background
/// kernel job) keeps running as the same account record user instead of falling back to root
/// (Master Spec 02-system-architecture.md:2773).
#[derive(Clone)]
pub(crate) struct CapturedAccountAuthority {
    scope: super::resource_authority::RecordUserScope,
    receipt: Option<LoomSessionReceipt>,
}

/// Captures the current task's account authority, if any.
pub(crate) fn capture_account_authority() -> Option<CapturedAccountAuthority> {
    let scope = super::current_record_user_scope()?;
    Some(CapturedAccountAuthority {
        scope,
        receipt: LOOM_SESSION_RECEIPT.try_with(Clone::clone).ok(),
    })
}

/// Runs `operation` under a previously captured account authority; without one it runs as is.
pub(crate) async fn with_captured_account_authority<T>(
    storage: &SurrealStorage,
    captured: Option<CapturedAccountAuthority>,
    operation: impl std::future::Future<Output = T>,
) -> T {
    match captured {
        None => operation.await,
        Some(CapturedAccountAuthority {
            scope,
            receipt: None,
        }) => storage.with_record_user_scope(scope, operation).await,
        Some(CapturedAccountAuthority {
            scope,
            receipt: Some(receipt),
        }) => {
            storage
                .with_record_user_scope(scope, LOOM_SESSION_RECEIPT.scope(receipt, operation))
                .await
        }
    }
}

#[derive(Clone, SurrealValue)]
pub(crate) struct LedgerWrite {
    pub(crate) record: RecordId,
    pub(crate) event_id: String,
    pub(crate) event_version: String,
    pub(crate) kernel_task_run_id: String,
    pub(crate) session_run_id: String,
    pub(crate) aggregate_type: String,
    pub(crate) aggregate_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) event_type: String,
    pub(crate) actor_kind: String,
    pub(crate) actor_id: String,
    pub(crate) causation_id: Option<String>,
    pub(crate) correlation_id: Option<String>,
    pub(crate) payload_hash: String,
    pub(crate) source_component: String,
    pub(crate) payload: serde_json::Value,
    pub(crate) wsids: Vec<String>,
    pub(crate) authority_resource_id: Option<RecordId>,
    pub(crate) authority_session_id: Option<RecordId>,
    pub(crate) authority_capability_id: Option<String>,
    pub(crate) authority_action: Option<String>,
    pub(crate) created_at: Datetime,
}

#[derive(SurrealValue)]
struct EventBindings {
    event: LedgerWrite,
}

#[derive(SurrealValue)]
struct TimedEventBindings {
    event: LedgerWrite,
    lookup_plan_enabled: bool,
}

#[derive(SurrealValue)]
struct EventBatchBindings {
    events: Vec<LedgerBulkInsert>,
    idempotency_keys: Vec<String>,
}

#[derive(SurrealValue)]
pub(crate) struct LedgerBulkInsert {
    id: RecordId,
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: serde_json::Value,
    wsids: Vec<String>,
    authority_resource_id: Option<RecordId>,
    authority_session_id: Option<RecordId>,
    authority_capability_id: Option<String>,
    authority_action: Option<String>,
    created_at: Datetime,
}

impl From<LedgerWrite> for LedgerBulkInsert {
    fn from(write: LedgerWrite) -> Self {
        Self {
            id: write.record,
            event_id: write.event_id,
            event_version: write.event_version,
            kernel_task_run_id: write.kernel_task_run_id,
            session_run_id: write.session_run_id,
            aggregate_type: write.aggregate_type,
            aggregate_id: write.aggregate_id,
            idempotency_key: write.idempotency_key,
            event_type: write.event_type,
            actor_kind: write.actor_kind,
            actor_id: write.actor_id,
            causation_id: write.causation_id,
            correlation_id: write.correlation_id,
            payload_hash: write.payload_hash,
            source_component: write.source_component,
            payload: write.payload,
            wsids: write.wsids,
            authority_resource_id: write.authority_resource_id,
            authority_session_id: write.authority_session_id,
            authority_capability_id: write.authority_capability_id,
            authority_action: write.authority_action,
            created_at: write.created_at,
        }
    }
}

#[derive(SurrealValue)]
struct EventPairBindings {
    first: LedgerWrite,
    second: LedgerWrite,
    idempotency_keys: Vec<String>,
}

#[derive(SurrealValue)]
struct EventLookupBindings {
    value: String,
}

#[derive(SurrealValue)]
struct PendingMirrorBindings {
    pending_type: String,
    completed_type: String,
    after_sequence: i64,
}

#[derive(Clone, SurrealValue)]
struct LedgerRow {
    event_id: String,
    event_sequence: i64,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: serde_json::Value,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct TimedLedgerAppend {
    rows: Value,
    replay: bool,
    lookup_elapsed_us: i64,
    operation_elapsed_us: i64,
    lookup_plan: Value,
}

fn plan_string(value: Option<&Value>) -> Option<&str> {
    match value {
        Some(Value::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

/// Interpret the pinned engine's legacy and physical SELECT EXPLAIN shapes. Unknown data is
/// unsupported, never formatted in an error or copied into an observation.
fn summarize_receipt_lookup_plan(
    value: &Value,
) -> handshake_storage_support::diagnostics::ReceiptLookupPlanSummary {
    use handshake_storage_support::diagnostics::{ReceiptLookupPlanKind, ReceiptLookupPlanSummary};
    const MAX_ENTRIES: usize = 16;
    let mut summary = ReceiptLookupPlanSummary {
        plan_kind: ReceiptLookupPlanKind::Other,
        expected_index_used: false,
        supported_shape: false,
        entry_count: 0,
        explain_elapsed_us: -1,
    };
    let Value::Object(envelope) = value else {
        return summary;
    };
    let Some(Value::Number(surrealdb::types::Number::Int(elapsed))) = envelope.get("elapsed_us")
    else {
        return summary;
    };
    summary.explain_elapsed_us = *elapsed;
    if envelope.len() != 2 {
        return summary;
    }
    if let Some(Value::Object(plan)) = envelope.get("entries") {
        if let Some((kind, expected_index)) =
            summarize_physical_receipt_plan(plan, &mut summary.entry_count)
        {
            summary.plan_kind = kind;
            summary.expected_index_used = expected_index;
            summary.supported_shape = true;
        }
        return summary;
    }
    let Some(Value::Array(entries)) = envelope.get("entries") else {
        return summary;
    };
    summary.entry_count = entries.len().min(MAX_ENTRIES) as u8;
    if entries.is_empty() || entries.len() > MAX_ENTRIES {
        return summary;
    }
    let mut kind = ReceiptLookupPlanKind::None;
    let mut expected_index = false;
    let mut iterators = 0;
    let mut collectors = 0;
    let mut fallbacks = 0;
    for entry in entries.iter() {
        let Value::Object(entry) = entry else {
            return summary;
        };
        let Some(Value::Object(detail)) = entry.get("detail") else {
            return summary;
        };
        if entry.len() != 2 {
            return summary;
        }
        match plan_string(entry.get("operation")) {
            Some("Iterate Index" | "Iterate Index Keys" | "Iterate Index Count") => {
                let Some(Value::Object(plan)) = detail.get("plan") else {
                    return summary;
                };
                if detail.len() != 2
                    || plan_string(detail.get("table")) != Some(EVENT_TABLE)
                    || plan.len() != 3
                    || plan_string(plan.get("operator")) != Some("=")
                    || plan_string(plan.get("index")).is_none()
                    || !plan.contains_key("value")
                {
                    return summary;
                }
                // The value is intentionally neither traversed nor retained.
                expected_index =
                    plan_string(plan.get("index")) == Some("idx_kernel_event_ledger_idempotency");
                kind = ReceiptLookupPlanKind::Index;
                iterators += 1;
            }
            Some("Iterate Table" | "Iterate Table Keys" | "Iterate Table Count") => {
                if detail.len() != 2
                    || plan_string(detail.get("table")) != Some(EVENT_TABLE)
                    || !matches!(
                        plan_string(detail.get("direction")),
                        Some("forward" | "backward")
                    )
                {
                    return summary;
                }
                kind = ReceiptLookupPlanKind::Table;
                iterators += 1;
            }
            Some("Collector") => {
                if detail.len() != 1 || plan_string(detail.get("type")) != Some("Memory") {
                    return summary;
                }
                collectors += 1;
            }
            Some("Fallback") => {
                if detail.len() != 1 || plan_string(detail.get("reason")).is_none() {
                    return summary;
                }
                fallbacks += 1;
            }
            _ => return summary,
        }
    }
    if collectors != 1 || iterators > 1 || fallbacks > 1 {
        return summary;
    }
    // `none` means no iterable was selected, not that the lookup returned no rows.
    summary.plan_kind = kind;
    summary.expected_index_used = expected_index;
    summary.supported_shape = true;
    summary
}

/// This parser belongs only to the fixed ledger SELECT below. Physical IndexScan
/// omits its table; table provenance comes from that query, not from plan metadata.
fn summarize_physical_receipt_plan(
    mut node: &surrealdb::types::Object,
    count: &mut u8,
) -> Option<(
    handshake_storage_support::diagnostics::ReceiptLookupPlanKind,
    bool,
)> {
    use handshake_storage_support::diagnostics::ReceiptLookupPlanKind;
    const MAX_NODES: u8 = 16;
    while *count < MAX_NODES {
        *count += 1;
        if node.keys().any(|key| {
            !matches!(
                key.as_str(),
                "operator" | "context" | "attributes" | "expressions" | "children"
            )
        }) || !matches!(plan_string(node.get("context")), Some("Rt" | "Ns" | "Db"))
        {
            return None;
        }
        let operator = plan_string(node.get("operator"))?;
        let allowed_attributes: &[&str] = match operator {
            "ProjectValue" => &["expr"],
            "Project" | "EmptyScan" => &[],
            "SelectProject" => &["projections"],
            "Filter" => &["predicate"],
            "Limit" => &["limit", "offset"],
            "IndexScan" => &["index", "access", "direction", "limit", "offset"],
            "TableScan" => &[
                "table",
                "direction",
                "predicate",
                "limit",
                "offset",
                "pre_decode_filter",
                "topk_pushdown",
            ],
            // DynamicScan has not resolved its source at plan time.
            _ => return None,
        };
        let attributes = match node.get("attributes") {
            None => None,
            Some(Value::Object(attributes))
                if !attributes.is_empty()
                    && attributes.len() <= allowed_attributes.len()
                    && attributes.iter().all(|(key, value)| {
                        allowed_attributes.contains(&key.as_str())
                            && matches!(value, Value::String(_))
                    }) =>
            {
                Some(attributes)
            }
            _ => return None,
        };
        let attribute = |key: &str| plan_string(attributes.and_then(|attrs| attrs.get(key)));
        if let Some(expressions) = node.get("expressions") {
            let Value::Array(expressions) = expressions else {
                return None;
            };
            let roles: &[&str] = match operator {
                "ProjectValue" => &["expr"],
                "Project" => &["field"],
                "Filter" => &["predicate"],
                "Limit" => &["limit", "offset"],
                _ => return None,
            };
            if expressions.is_empty() || expressions.len() > MAX_NODES as usize {
                return None;
            }
            for expression in expressions.iter() {
                let Value::Object(expression) = expression else {
                    return None;
                };
                // SQL is type-checked only; embedded plans are outside this fixed query.
                if expression.len() != 2
                    || !plan_string(expression.get("role"))
                        .is_some_and(|role| roles.contains(&role))
                    || plan_string(expression.get("sql")).is_none()
                {
                    return None;
                }
            }
        }
        match operator {
            "IndexScan" | "TableScan" | "EmptyScan" => {
                if node.contains_key("children") || node.contains_key("expressions") {
                    return None;
                }
                return match operator {
                    "IndexScan" => {
                        let index = attribute("index")?;
                        let access = attribute("access")?;
                        if index.is_empty()
                            || !access.starts_with("= ")
                            || access.len() <= 2
                            || !matches!(attribute("direction"), Some("Forward" | "Backward"))
                        {
                            return None;
                        }
                        // Inspect only the equality prefix; never retain its bound-value suffix.
                        Some((
                            ReceiptLookupPlanKind::Index,
                            index == "idx_kernel_event_ledger_idempotency",
                        ))
                    }
                    "TableScan"
                        if attribute("table") == Some(EVENT_TABLE)
                            && matches!(attribute("direction"), Some("Forward" | "Backward")) =>
                    {
                        Some((ReceiptLookupPlanKind::Table, false))
                    }
                    "EmptyScan" if attributes.is_none() => {
                        Some((ReceiptLookupPlanKind::None, false))
                    }
                    _ => None,
                };
            }
            "ProjectValue" if attribute("expr").is_none() => return None,
            "SelectProject" if attribute("projections").is_none() => return None,
            "Filter" if attribute("predicate").is_none() => return None,
            "Limit" if attributes.is_none() => return None,
            _ => {}
        }
        let Some(Value::Array(children)) = node.get("children") else {
            return None;
        };
        let [Value::Object(child)] = children.as_slice() else {
            return None;
        };
        node = child;
    }
    None
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
enum ReceiptPlanShape {
    Disabled,
    LegacyArray,
    PhysicalObject,
    Other,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct ReceiptPlanTestObservation {
    shape: ReceiptPlanShape,
    summary: Option<handshake_storage_support::diagnostics::ReceiptLookupPlanSummary>,
    replay: bool,
    lookup_elapsed_us: i64,
    operation_elapsed_us: i64,
}

#[cfg(test)]
tokio::task_local! {
    static RECEIPT_PLAN_TEST_OBSERVATIONS: std::cell::RefCell<Vec<ReceiptPlanTestObservation>>;
}

fn decode_timed_append(
    mut envelopes: Vec<TimedLedgerAppend>,
    timing: &handshake_storage_support::diagnostics::ReceiptTimingContext,
) -> Result<Option<LedgerRow>, super::SurrealStorageError> {
    if envelopes.len() > 1 {
        return Err(
            surrealdb::Error::internal("multiple timed receipt envelopes".to_owned()).into(),
        );
    }
    let Some(measured) = envelopes.pop() else {
        return Ok(None);
    };
    let rows = match measured.rows {
        Value::None => Vec::new(),
        Value::Array(rows) => rows.into_iter().collect::<Vec<_>>(),
        _ => {
            return Err(surrealdb::Error::internal("invalid timed receipt rows".to_owned()).into())
        }
    };
    if rows.len() > 1 {
        return Err(surrealdb::Error::internal("multiple timed receipt rows".to_owned()).into());
    }
    let row = super::decode_first_value(rows)?;
    timing.emit(
        measured.replay,
        measured.lookup_elapsed_us,
        measured.operation_elapsed_us,
    );
    let plan_summary = (!matches!(measured.lookup_plan, Value::None))
        .then(|| summarize_receipt_lookup_plan(&measured.lookup_plan));
    #[cfg(test)]
    let _ = RECEIPT_PLAN_TEST_OBSERVATIONS.try_with(|observations| {
        let shape = match &measured.lookup_plan {
            Value::None => ReceiptPlanShape::Disabled,
            Value::Object(envelope) => match envelope.get("entries") {
                Some(Value::Array(_)) => ReceiptPlanShape::LegacyArray,
                Some(Value::Object(_)) => ReceiptPlanShape::PhysicalObject,
                _ => ReceiptPlanShape::Other,
            },
            _ => ReceiptPlanShape::Other,
        };
        observations.borrow_mut().push(ReceiptPlanTestObservation {
            shape,
            summary: plan_summary,
            replay: measured.replay,
            lookup_elapsed_us: measured.lookup_elapsed_us,
            operation_elapsed_us: measured.operation_elapsed_us,
        });
    });
    if let Some(summary) = plan_summary {
        timing.emit_plan(summary);
    }
    Ok(row)
}

pub(crate) fn prepare_event(
    mut event: NewKernelEvent,
) -> StorageResult<(KernelEvent, LedgerWrite)> {
    let loom_session = LOOM_SESSION_RECEIPT.try_with(Clone::clone).ok();
    if let Some(session) = &loom_session {
        event.actor = session.actor.clone();
    }
    event
        .validate()
        .map_err(|_| StorageError::Validation("invalid kernel event"))?;
    let stored = KernelEvent::from_new(event.clone());
    let authority = super::current_record_user_scope();
    let write = LedgerWrite {
        record: RecordId::new(EVENT_TABLE, stored.event_id.clone()),
        event_id: stored.event_id.clone(),
        event_version: event.event_version,
        kernel_task_run_id: event.kernel_task_run_id,
        session_run_id: event.session_run_id,
        aggregate_type: event.aggregate_type,
        aggregate_id: event.aggregate_id,
        idempotency_key: event.idempotency_key,
        event_type: event.event_type.as_str().to_owned(),
        actor_kind: event.actor.actor_kind().to_owned(),
        actor_id: event.actor.actor_id().to_owned(),
        causation_id: event.causation_id,
        correlation_id: event.correlation_id,
        payload_hash: event.payload_hash,
        source_component: event.source_component,
        payload: event.payload,
        wsids: match loom_session {
            Some(session) => vec![session.workspace_id],
            None => authority
                .as_ref()
                .and_then(|scope| scope.workspace_id.clone())
                .into_iter()
                .collect(),
        },
        authority_resource_id: authority
            .as_ref()
            .map(|scope| RecordId::new("protected_resources", scope.resource_id.clone())),
        authority_session_id: authority
            .as_ref()
            .map(|scope| RecordId::new("authenticated_sessions", scope.session_id.clone())),
        authority_capability_id: authority.as_ref().map(|scope| scope.capability_id.clone()),
        authority_action: authority.map(|scope| scope.action.as_str().to_owned()),
        created_at: Datetime::from(stored.created_at),
    };
    Ok((stored, write))
}

pub(crate) async fn append(
    storage: &SurrealStorage,
    event: NewKernelEvent,
) -> StorageResult<KernelEvent> {
    let (candidate, write) = prepare_event(event)?;
    let idempotency_key = candidate.idempotency_key.clone();
    let timing = handshake_storage_support::diagnostics::ReceiptTimingContext::capture();
    let lookup_plan_enabled = handshake_storage_support::diagnostics::receipt_lookup_plan_enabled();
    let result: Result<Option<LedgerRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                if let Some(timing) = timing {
                    // One top-level IF retains the original implicit write transaction.
                    // The optional EXPLAIN uses this same authenticated facade and binding.
                    // It skips iteration, adds diagnostic latency, and can see warmed planner
                    // state. Its duration does not measure per-row permission evaluation.
                    let measured: Vec<TimedLedgerAppend> = database
                        .query_values(
                            "IF true { \
                                 LET $lookup_started = time::micros(); \
                                 LET $existing = (SELECT VALUE id FROM kernel_event_ledger \
                                     WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0]; \
                                 LET $lookup_finished = time::micros(); \
                                 LET $lookup_plan = IF $lookup_plan_enabled { \
                                     LET $explain_started = time::micros(); \
                                     LET $entries = (SELECT VALUE id FROM kernel_event_ledger \
                                         WHERE idempotency_key = $event.idempotency_key LIMIT 1 EXPLAIN); \
                                     LET $explain_finished = time::micros(); \
                                     { entries: $entries, elapsed_us: $explain_finished - $explain_started }; \
                                 } ELSE { NONE }; \
                                 LET $operation_started = IF $lookup_plan_enabled { time::micros() } ELSE { $lookup_finished }; \
                                 IF $existing != NONE { \
                                     LET $rows = (SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                                         session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                                         actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                                         source_component, payload, created_at FROM kernel_event_ledger \
                                         WHERE idempotency_key = $event.idempotency_key LIMIT 1); \
                                     LET $operation_finished = time::micros(); \
                                     RETURN { rows: $rows, replay: true, \
                                         lookup_elapsed_us: $lookup_finished - $lookup_started, \
                                         operation_elapsed_us: $operation_finished - $operation_started, \
                                         lookup_plan: $lookup_plan }; \
                                 } ELSE { \
                                     LET $rows = (CREATE $event.record CONTENT { \
                                         event_id: $event.event_id, event_version: $event.event_version, \
                                         kernel_task_run_id: $event.kernel_task_run_id, \
                                         session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                                         aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                                         event_type: $event.event_type, actor_kind: $event.actor_kind, \
                                         actor_id: $event.actor_id, causation_id: $event.causation_id, \
                                         correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                                         source_component: $event.source_component, payload: $event.payload, \
                                         wsids: $event.wsids, \
                                         authority_resource_id: $event.authority_resource_id, \
                                         authority_session_id: $event.authority_session_id, \
                                         authority_capability_id: $event.authority_capability_id, \
                                         authority_action: $event.authority_action, \
                                         created_at: $event.created_at \
                                     }); \
                                     LET $operation_finished = time::micros(); \
                                     RETURN { rows: $rows, replay: false, \
                                         lookup_elapsed_us: $lookup_finished - $lookup_started, \
                                         operation_elapsed_us: $operation_finished - $operation_started, \
                                         lookup_plan: $lookup_plan }; \
                                 }; \
                             };",
                            TimedEventBindings { event: write, lookup_plan_enabled },
                        )
                        .await?;
                    return decode_timed_append(measured, &timing);
                }
                database
                    .query_first(
                        "IF (SELECT VALUE id FROM kernel_event_ledger \
                             WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0] != NONE { \
                             RETURN SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                                 session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                                 actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                                 source_component, payload, created_at FROM kernel_event_ledger \
                                 WHERE idempotency_key = $event.idempotency_key LIMIT 1; \
                         } ELSE { \
                             RETURN CREATE $event.record CONTENT { \
                                 event_id: $event.event_id, event_version: $event.event_version, \
                                 kernel_task_run_id: $event.kernel_task_run_id, \
                                 session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                                 aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                                 event_type: $event.event_type, actor_kind: $event.actor_kind, \
                                 actor_id: $event.actor_id, causation_id: $event.causation_id, \
                                 correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                                  source_component: $event.source_component, payload: $event.payload, \
                                  wsids: $event.wsids, \
                                  authority_resource_id: $event.authority_resource_id, \
                                  authority_session_id: $event.authority_session_id, \
                                  authority_capability_id: $event.authority_capability_id, \
                                  authority_action: $event.authority_action, \
                                  created_at: $event.created_at \
                             }; \
                         };",
                        EventBindings { event: write },
                    )
                    .await
            })
        })
        .await;
    let row = match result {
        Ok(row) => row,
        Err(error) => {
            // A concurrent exact replay can lose the unique-index race after
            // both callers observe the idempotency key as absent. Re-read the
            // winner and accept it only when every immutable event dimension
            // matches; otherwise retain the original database failure.
            if let Some(stored) = get_by_idempotency(storage, &idempotency_key).await? {
                ensure_same_event(&stored, &candidate)?;
                return Ok(stored);
            }
            return Err(StorageError::from(error));
        }
    };
    let stored = row.map(row_to_event).transpose()?.ok_or_else(|| {
        // spec_ruling_c3_silent_deny (Master Spec 02-system-architecture.md:2758): a record
        // user's receipt CREATE denied by the kernel_event_ledger predicates returns no row.
        if super::current_record_user_scope().is_some() {
            StorageError::Guard("HSK-403-PROTECTED-RESOURCE")
        } else {
            StorageError::Database("EventLedger append returned no row".to_owned())
        }
    })?;
    ensure_same_event(&stored, &candidate)?;
    Ok(stored)
}

/// Appends a batch as one canonical SurrealDB EventLedger transaction. An exact
/// idempotent replay returns the original stored event, while any immutable-
/// content mismatch aborts the entire batch.
pub(crate) async fn append_atomic(
    storage: &SurrealStorage,
    events: Vec<NewKernelEvent>,
) -> StorageResult<Vec<KernelEvent>> {
    if events.is_empty() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::with_capacity(events.len());
    let mut writes = Vec::with_capacity(events.len());
    let mut idempotency_keys = Vec::with_capacity(events.len());
    for event in events {
        let (candidate, write) = prepare_event(event)?;
        idempotency_keys.push(candidate.idempotency_key.clone());
        candidates.push(candidate);
        writes.push(write);
    }

    let existing_rows = read_by_idempotency_keys(storage, idempotency_keys.clone()).await?;
    let existing_by_key = existing_rows
        .iter()
        .map(|row| (row.idempotency_key.as_str(), row))
        .collect::<std::collections::HashMap<_, _>>();
    let mut first_candidate_by_key = std::collections::HashMap::new();
    let mut inserts = Vec::with_capacity(writes.len());
    for (index, (candidate, write)) in candidates.iter().zip(writes).enumerate() {
        if let Some(first_index) = first_candidate_by_key.get(&candidate.idempotency_key) {
            ensure_same_event(&candidates[*first_index], candidate)?;
            continue;
        }
        first_candidate_by_key.insert(candidate.idempotency_key.clone(), index);
        if let Some(row) = existing_by_key.get(candidate.idempotency_key.as_str()) {
            ensure_same_event(&row_to_event((*row).clone())?, candidate)?;
        } else {
            inserts.push(LedgerBulkInsert::from(write));
        }
    }
    drop(existing_by_key);

    if inserts.is_empty() {
        return order_and_validate(existing_rows, &candidates);
    }

    let bindings = EventBatchBindings {
        events: inserts,
        idempotency_keys,
    };
    let result: Result<Vec<LedgerRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         INSERT INTO kernel_event_ledger $events RETURN NONE; \
                         COMMIT TRANSACTION; \
                         SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                           session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                           actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                           source_component, payload, created_at FROM kernel_event_ledger \
                           WHERE idempotency_key IN $idempotency_keys;",
                        bindings,
                        3,
                    )
                    .await
            })
        })
        .await;

    match result {
        Ok(rows) => order_and_validate(rows, &candidates),
        Err(error) if is_idempotency_conflict(&error.to_string()) => Err(idempotency_conflict()),
        Err(error) => {
            // A concurrent exact replay may win a unique-index race. Match the
            // single-append contract by accepting only a complete, exact
            // winner set; a partial or conflicting set retains the failure.
            if let Some(stored) = read_and_validate(storage, &candidates).await? {
                return Ok(stored);
            }
            Err(StorageError::from(error))
        }
    }
}

/// Atomically appends two events and binds the second event's causation to
/// the actual stored first event. This matters when the first event is an
/// idempotent replay whose durable event id differs from the fresh candidate.
pub(crate) async fn append_pair_atomic_with_causation(
    storage: &SurrealStorage,
    first: NewKernelEvent,
    mut second: NewKernelEvent,
) -> StorageResult<Vec<KernelEvent>> {
    let (first_candidate, first_write) = prepare_event(first)?;
    // Validate the second event with a syntactically valid ledger id before
    // the transaction. The transaction replaces this provisional causation
    // value with the actual stored first event id and validates replay content
    // against that same durable id.
    second.causation_id = Some(first_candidate.event_id.clone());
    let (second_candidate, second_write) = prepare_event(second)?;
    let candidates = [first_candidate, second_candidate];
    let bindings = EventPairBindings {
        idempotency_keys: candidates
            .iter()
            .map(|event| event.idempotency_key.clone())
            .collect(),
        first: first_write,
        second: second_write,
    };
    let result: Result<Vec<LedgerRow>, _> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         LET $first_stored = (SELECT event_id, event_sequence, event_version, \
                           kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, \
                           idempotency_key, event_type, actor_kind, actor_id, causation_id, \
                           correlation_id, payload_hash, source_component, payload, created_at \
                           FROM kernel_event_ledger \
                           WHERE idempotency_key = $first.idempotency_key LIMIT 1)[0]; \
                         IF $first_stored != NONE { \
                           IF $first_stored.event_version != $first.event_version \
                              OR $first_stored.kernel_task_run_id != $first.kernel_task_run_id \
                              OR $first_stored.session_run_id != $first.session_run_id \
                              OR $first_stored.aggregate_type != $first.aggregate_type \
                              OR $first_stored.aggregate_id != $first.aggregate_id \
                              OR $first_stored.event_type != $first.event_type \
                              OR $first_stored.actor_kind != $first.actor_kind \
                              OR $first_stored.actor_id != $first.actor_id \
                              OR $first_stored.causation_id != $first.causation_id \
                              OR $first_stored.correlation_id != $first.correlation_id \
                              OR $first_stored.payload_hash != $first.payload_hash \
                              OR $first_stored.source_component != $first.source_component { \
                             THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; \
                           }; \
                         } ELSE { \
                           CREATE $first.record CONTENT { \
                             event_id: $first.event_id, event_version: $first.event_version, \
                             kernel_task_run_id: $first.kernel_task_run_id, \
                             session_run_id: $first.session_run_id, aggregate_type: $first.aggregate_type, \
                             aggregate_id: $first.aggregate_id, idempotency_key: $first.idempotency_key, \
                             event_type: $first.event_type, actor_kind: $first.actor_kind, \
                             actor_id: $first.actor_id, causation_id: $first.causation_id, \
                             correlation_id: $first.correlation_id, payload_hash: $first.payload_hash, \
                             source_component: $first.source_component, payload: $first.payload, \
                             wsids: $first.wsids, \
                                  authority_resource_id: $first.authority_resource_id, \
                             authority_session_id: $first.authority_session_id, \
                             authority_capability_id: $first.authority_capability_id, \
                             authority_action: $first.authority_action, \
                             created_at: $first.created_at \
                           } RETURN NONE; \
                         }; \
                         LET $actual_first = (SELECT event_id FROM kernel_event_ledger \
                           WHERE idempotency_key = $first.idempotency_key LIMIT 1)[0]; \
                         LET $second_stored = (SELECT event_id, event_sequence, event_version, \
                           kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, \
                           idempotency_key, event_type, actor_kind, actor_id, causation_id, \
                           correlation_id, payload_hash, source_component, payload, created_at \
                           FROM kernel_event_ledger \
                           WHERE idempotency_key = $second.idempotency_key LIMIT 1)[0]; \
                         IF $second_stored != NONE { \
                           IF $second_stored.event_version != $second.event_version \
                              OR $second_stored.kernel_task_run_id != $second.kernel_task_run_id \
                              OR $second_stored.session_run_id != $second.session_run_id \
                              OR $second_stored.aggregate_type != $second.aggregate_type \
                              OR $second_stored.aggregate_id != $second.aggregate_id \
                              OR $second_stored.event_type != $second.event_type \
                              OR $second_stored.actor_kind != $second.actor_kind \
                              OR $second_stored.actor_id != $second.actor_id \
                              OR $second_stored.causation_id != $actual_first.event_id \
                              OR $second_stored.correlation_id != $second.correlation_id \
                              OR $second_stored.payload_hash != $second.payload_hash \
                              OR $second_stored.source_component != $second.source_component { \
                             THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; \
                           }; \
                         } ELSE { \
                           CREATE $second.record CONTENT { \
                             event_id: $second.event_id, event_version: $second.event_version, \
                             kernel_task_run_id: $second.kernel_task_run_id, \
                             session_run_id: $second.session_run_id, aggregate_type: $second.aggregate_type, \
                             aggregate_id: $second.aggregate_id, idempotency_key: $second.idempotency_key, \
                             event_type: $second.event_type, actor_kind: $second.actor_kind, \
                             actor_id: $second.actor_id, causation_id: $actual_first.event_id, \
                             correlation_id: $second.correlation_id, payload_hash: $second.payload_hash, \
                             source_component: $second.source_component, payload: $second.payload, \
                             wsids: $second.wsids, \
                                  authority_resource_id: $second.authority_resource_id, \
                             authority_session_id: $second.authority_session_id, \
                             authority_capability_id: $second.authority_capability_id, \
                             authority_action: $second.authority_action, \
                             created_at: $second.created_at \
                           } RETURN NONE; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                           session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                           actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                           source_component, payload, created_at FROM kernel_event_ledger \
                           WHERE idempotency_key IN $idempotency_keys;",
                        bindings,
                        7,
                    )
                    .await
            })
        })
        .await;

    match result {
        Ok(rows) => order_pair_and_validate(rows, &candidates),
        Err(error) if is_idempotency_conflict(&error.to_string()) => Err(idempotency_conflict()),
        Err(error) => {
            if let Some(stored) = read_pair_and_validate(storage, &candidates).await? {
                return Ok(stored);
            }
            Err(StorageError::from(error))
        }
    }
}

async fn read_and_validate(
    storage: &SurrealStorage,
    candidates: &[KernelEvent],
) -> StorageResult<Option<Vec<KernelEvent>>> {
    let keys = candidates
        .iter()
        .map(|event| event.idempotency_key.clone())
        .collect();
    let rows = read_by_idempotency_keys(storage, keys).await?;
    let expected_count = candidates
        .iter()
        .map(|event| event.idempotency_key.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    if rows.len() < expected_count {
        return Ok(None);
    }
    order_and_validate(rows, candidates).map(Some)
}

async fn read_pair_and_validate(
    storage: &SurrealStorage,
    candidates: &[KernelEvent; 2],
) -> StorageResult<Option<Vec<KernelEvent>>> {
    let keys = candidates
        .iter()
        .map(|event| event.idempotency_key.clone())
        .collect();
    let rows = read_by_idempotency_keys(storage, keys).await?;
    let expected_count = candidates
        .iter()
        .map(|event| event.idempotency_key.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    if rows.len() < expected_count {
        return Ok(None);
    }
    order_pair_and_validate(rows, candidates).map(Some)
}

async fn read_by_idempotency_keys(
    storage: &SurrealStorage,
    idempotency_keys: Vec<String>,
) -> StorageResult<Vec<LedgerRow>> {
    #[derive(SurrealValue)]
    struct IdempotencyKeysBindings {
        idempotency_keys: Vec<String>,
    }

    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                           session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                           actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                           source_component, payload, created_at FROM kernel_event_ledger \
                           WHERE idempotency_key IN $idempotency_keys;",
                        IdempotencyKeysBindings { idempotency_keys },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

fn order_and_validate(
    rows: Vec<LedgerRow>,
    candidates: &[KernelEvent],
) -> StorageResult<Vec<KernelEvent>> {
    let mut stored_by_key = std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let event = row_to_event(row)?;
        stored_by_key.insert(event.idempotency_key.clone(), event);
    }
    candidates
        .iter()
        .map(|candidate| {
            let stored = stored_by_key
                .get(&candidate.idempotency_key)
                .ok_or_else(|| {
                    StorageError::Database(
                        "atomic EventLedger append returned an incomplete result set".to_owned(),
                    )
                })?
                .clone();
            ensure_same_event(&stored, candidate)?;
            Ok(stored)
        })
        .collect()
}

fn order_pair_and_validate(
    rows: Vec<LedgerRow>,
    candidates: &[KernelEvent; 2],
) -> StorageResult<Vec<KernelEvent>> {
    let mut stored_by_key = std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let event = row_to_event(row)?;
        stored_by_key.insert(event.idempotency_key.clone(), event);
    }
    let first = stored_by_key
        .get(&candidates[0].idempotency_key)
        .ok_or_else(|| {
            StorageError::Database(
                "atomic EventLedger pair append returned no first event".to_owned(),
            )
        })?
        .clone();
    ensure_same_event(&first, &candidates[0])?;

    let second = stored_by_key
        .get(&candidates[1].idempotency_key)
        .ok_or_else(|| {
            StorageError::Database(
                "atomic EventLedger pair append returned no second event".to_owned(),
            )
        })?
        .clone();
    let mut expected_second = candidates[1].clone();
    expected_second.causation_id = Some(first.event_id.clone());
    ensure_same_event(&second, &expected_second)?;
    Ok(vec![first, second])
}

fn is_idempotency_conflict(error: &str) -> bool {
    error.contains("HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT")
}

fn idempotency_conflict() -> StorageError {
    StorageError::Conflict("kernel event idempotency key was reused with different event content")
}

pub(crate) async fn list_for_session(
    storage: &SurrealStorage,
    session_run_id: &str,
) -> StorageResult<Vec<KernelEvent>> {
    list(
        storage,
        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, \
         aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, \
         causation_id, correlation_id, payload_hash, source_component, payload, created_at \
         FROM kernel_event_ledger WHERE session_run_id = $value ORDER BY event_sequence ASC;",
        session_run_id,
    )
    .await
}

impl SurrealStorage {
    /// MT-109 C2 (Master Spec 02-system-architecture:2773/:2776): the aggregate EventLedger read runs
    /// as the calling record user, so the `kernel_event_ledger` read permissions return only receipts
    /// the caller's account may see (its authorized workspaces), never another account's ledger.
    pub async fn list_account_kernel_events_for_aggregate(
        &self,
        token: &str,
        channel_hash: &str,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> StorageResult<Vec<KernelEvent>> {
        use super::resource_authority::{SigninParams, AUTHORITY_ACCESS_METHOD};
        use sha2::{Digest, Sha256};
        use surrealdb::opt::auth::Record;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
        let channel_binding_hash = Some(channel_hash.to_owned());
        let aggregate_type = aggregate_type.to_owned();
        let aggregate_id = aggregate_id.to_owned();
        let rows: Vec<LedgerRow> = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let ordinary = client.clone();
                    ordinary
                        .use_ns(namespace.clone())
                        .use_db(database.clone())
                        .await?;
                    ordinary
                        .signin(Record {
                            namespace,
                            database,
                            access: AUTHORITY_ACCESS_METHOD.to_owned(),
                            params: SigninParams {
                                token_hash,
                                channel_binding_hash,
                            },
                        })
                        .await?;
                    let mut result = ordinary
                        .query(
                            "SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, \
                             aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, \
                             causation_id, correlation_id, payload_hash, source_component, payload, created_at \
                             FROM kernel_event_ledger WHERE aggregate_type = $aggregate_type \
                             AND aggregate_id = $aggregate_id ORDER BY event_sequence ASC;",
                        )
                        .bind(("aggregate_type", aggregate_type))
                        .bind(("aggregate_id", aggregate_id))
                        .await?
                        .check()?;
                    Ok(result.take(0)?)
                })
            })
            .await
            .map_err(StorageError::from)?;
        rows.into_iter().map(row_to_event).collect()
    }
}

pub(crate) async fn list_for_aggregate(
    storage: &SurrealStorage,
    aggregate_type: &str,
    aggregate_id: &str,
) -> StorageResult<Vec<KernelEvent>> {
    #[derive(SurrealValue)]
    struct AggregateBindings {
        aggregate_type: String,
        aggregate_id: String,
    }
    let bindings = AggregateBindings {
        aggregate_type: aggregate_type.to_owned(),
        aggregate_id: aggregate_id.to_owned(),
    };
    let rows: Vec<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, \
                         aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, \
                         causation_id, correlation_id, payload_hash, source_component, payload, created_at \
                         FROM kernel_event_ledger WHERE aggregate_type = $aggregate_type \
                         AND aggregate_id = $aggregate_id ORDER BY event_sequence ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(row_to_event).collect()
}

pub(crate) async fn list_pending_native_editor_mirrors(
    storage: &SurrealStorage,
    after_event_sequence: i64,
    limit: i64,
) -> StorageResult<Vec<KernelEvent>> {
    let bindings = PendingMirrorBindings {
        pending_type: KernelEventType::FlightRecorderMirrorPending
            .as_str()
            .to_owned(),
        completed_type: KernelEventType::FlightRecorderMirrorRecorded
            .as_str()
            .to_owned(),
        after_sequence: after_event_sequence.max(0),
    };
    let rows: Vec<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, \
                         aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, \
                         causation_id, correlation_id, payload_hash, source_component, payload, created_at \
                         FROM kernel_event_ledger WHERE \
                           (event_type = $pending_type AND aggregate_type = 'native_editor_event' \
                            AND event_sequence > $after_sequence) \
                           OR event_type = $completed_type \
                         ORDER BY event_sequence ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;

    let mut pending = Vec::new();
    let mut completed = Vec::new();
    for row in rows {
        let event = row_to_event(row)?;
        if event.event_type == KernelEventType::FlightRecorderMirrorPending {
            pending.push(event);
        } else if event.event_type == KernelEventType::FlightRecorderMirrorRecorded {
            completed.push(event);
        }
    }

    let limit = usize::try_from(limit.clamp(1, 1_000)).unwrap_or(1_000);
    pending.retain(|candidate| {
        !completed
            .iter()
            .any(|receipt| native_editor_completion_matches(candidate, receipt))
    });
    pending.truncate(limit);
    Ok(pending)
}

fn native_editor_completion_matches(pending: &KernelEvent, completed: &KernelEvent) -> bool {
    let Some(expected_hash) = pending
        .payload
        .get("expected_completion_payload_hash")
        .and_then(serde_json::Value::as_str)
    else {
        // Legacy pending receipts deliberately remain visible so the reconciler
        // can revalidate them without rewriting append-only EventLedger rows.
        return false;
    };
    let expected_payload = serde_json::json!({
        "receipt_kind": "native_editor_flight_recorder_recorded",
        "fr_event_id": pending.aggregate_id,
        "fr_event_type": "system",
        "envelope": pending.payload.get("envelope").cloned().unwrap_or(serde_json::Value::Null),
    });
    completed.aggregate_type == pending.aggregate_type
        && completed.aggregate_id == pending.aggregate_id
        && completed.event_version == pending.event_version
        && completed.kernel_task_run_id == pending.kernel_task_run_id
        && completed.session_run_id == pending.session_run_id
        && completed.idempotency_key
            == format!("native-editor-fr-complete:{}", pending.aggregate_id)
        && completed.source_component == "native_editor_fr_ingestion"
        && completed.actor == pending.actor
        && completed.causation_id.as_deref() == Some(pending.event_id.as_str())
        && completed.correlation_id.as_deref()
            == Some(
                pending
                    .correlation_id
                    .as_deref()
                    .unwrap_or(pending.aggregate_id.as_str()),
            )
        && completed.payload_hash == expected_hash
        && completed.payload == expected_payload
}

pub(crate) async fn get_by_idempotency(
    storage: &SurrealStorage,
    idempotency_key: &str,
) -> StorageResult<Option<KernelEvent>> {
    let bindings = EventLookupBindings {
        value: idempotency_key.to_owned(),
    };
    let row: Option<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT event_id, event_sequence, event_version, kernel_task_run_id, \
                         session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, \
                         actor_kind, actor_id, causation_id, correlation_id, payload_hash, \
                         source_component, payload, created_at FROM kernel_event_ledger \
                         WHERE idempotency_key = $value LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(row_to_event).transpose()
}

async fn list(
    storage: &SurrealStorage,
    statement: &'static str,
    value: &str,
) -> StorageResult<Vec<KernelEvent>> {
    let bindings = EventLookupBindings {
        value: value.to_owned(),
    };
    let rows: Vec<LedgerRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.query_values(statement, bindings).await })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(row_to_event).collect()
}

fn row_to_event(row: LedgerRow) -> StorageResult<KernelEvent> {
    let payload = normalize_self_describing_payload(row.payload)?;
    Ok(KernelEvent {
        event_id: row.event_id,
        event_sequence: row.event_sequence,
        event_version: row.event_version,
        kernel_task_run_id: row.kernel_task_run_id,
        session_run_id: row.session_run_id,
        aggregate_type: row.aggregate_type,
        aggregate_id: row.aggregate_id,
        idempotency_key: row.idempotency_key,
        event_type: KernelEventType::try_from(row.event_type.as_str())
            .map_err(|_| StorageError::Validation("invalid kernel event_type"))?,
        actor: actor_from_parts(&row.actor_kind, row.actor_id)?,
        causation_id: row.causation_id,
        correlation_id: row.correlation_id,
        payload_hash: row.payload_hash,
        source_component: row.source_component,
        payload,
        created_at: row.created_at.into_inner(),
    })
}

fn normalize_self_describing_payload(
    mut payload: serde_json::Value,
) -> StorageResult<serde_json::Value> {
    let is_float_preference_event = payload.get("type").and_then(serde_json::Value::as_str)
        == Some("preference_record_changed")
        && payload
            .get("value_type")
            .and_then(serde_json::Value::as_str)
            == Some("float");
    if !is_float_preference_event {
        return Ok(payload);
    }

    let object = payload.as_object_mut().ok_or(StorageError::Validation(
        "preference event payload is not an object",
    ))?;
    for field in ["old_value_ref", "new_value_ref"] {
        let Some(value) = object.get_mut(field) else {
            continue;
        };
        let Some(number) = value.as_f64() else {
            continue;
        };
        let number = serde_json::Number::from_f64(number).ok_or(StorageError::Validation(
            "preference event float value is not finite",
        ))?;
        *value = serde_json::Value::Number(number);
    }
    Ok(payload)
}

fn actor_from_parts(kind: &str, id: String) -> StorageResult<KernelActor> {
    match kind {
        "operator" => Ok(KernelActor::Operator(id)),
        "system" => Ok(KernelActor::System(id)),
        "session_broker" => Ok(KernelActor::SessionBroker(id)),
        "model_adapter" => Ok(KernelActor::ModelAdapter(id)),
        "toolgate" => Ok(KernelActor::ToolGate(id)),
        "validation_runner" => Ok(KernelActor::ValidationRunner(id)),
        "promotion_gate" => Ok(KernelActor::PromotionGate(id)),
        _ => Err(StorageError::Validation("invalid kernel actor_kind")),
    }
}

fn ensure_same_event(stored: &KernelEvent, candidate: &KernelEvent) -> StorageResult<()> {
    // Match the Handshake canonical event contract: payload_hash is computed
    // from canonical JSON, while SurrealDB may normalize the decoded numeric
    // JSON representation. Comparing payload Value directly would therefore
    // reject a semantically exact replay after harmless storage normalization.
    let same = stored.event_version == candidate.event_version
        && stored.kernel_task_run_id == candidate.kernel_task_run_id
        && stored.session_run_id == candidate.session_run_id
        && stored.aggregate_type == candidate.aggregate_type
        && stored.aggregate_id == candidate.aggregate_id
        && stored.idempotency_key == candidate.idempotency_key
        && stored.event_type == candidate.event_type
        && stored.actor == candidate.actor
        && stored.causation_id == candidate.causation_id
        && stored.correlation_id == candidate.correlation_id
        && stored.payload_hash == candidate.payload_hash
        && stored.source_component == candidate.source_component;
    if same {
        Ok(())
    } else {
        Err(StorageError::ConflictDetails {
            code: "kernel event idempotency key was reused with different event content",
            detail: format!(
                "idempotency_key={:?}; existing_payload_hash={}; new_payload_hash={}; \
                 existing_aggregate_type={:?}; existing_aggregate_id={:?}; \
                 new_aggregate_type={:?}; new_aggregate_id={:?}; \
                 existing_event_id={:?}; new_event_id={:?}",
                candidate.idempotency_key,
                stored.payload_hash,
                candidate.payload_hash,
                stored.aggregate_type,
                stored.aggregate_id,
                candidate.aggregate_type,
                candidate.aggregate_id,
                stored.event_id,
                candidate.event_id,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::SurrealStorageConfig;
    use super::*;
    use serde_json::json;
    use std::future::Future;
    use std::time::Duration;

    fn event(idempotency_key: &str, payload: serde_json::Value) -> NewKernelEvent {
        NewKernelEvent::builder(
            "mt-136-proof-task",
            "mt-136-proof-session",
            KernelEventType::ArtifactStored,
            KernelActor::System("mt-136-proof".to_owned()),
        )
        .aggregate("mt_136_storage_proof", "durable-event")
        .idempotency_key(idempotency_key)
        .source_component("storage_mt_136_proof")
        .payload(payload)
        .build()
        .expect("valid MT-136 EventLedger fixture")
    }

    fn row_from_event(event: KernelEvent, event_sequence: i64) -> LedgerRow {
        LedgerRow {
            event_id: event.event_id,
            event_sequence,
            event_version: event.event_version,
            kernel_task_run_id: event.kernel_task_run_id,
            session_run_id: event.session_run_id,
            aggregate_type: event.aggregate_type,
            aggregate_id: event.aggregate_id,
            idempotency_key: event.idempotency_key,
            event_type: event.event_type.as_str().to_owned(),
            actor_kind: event.actor.actor_kind().to_owned(),
            actor_id: event.actor.actor_id().to_owned(),
            causation_id: event.causation_id,
            correlation_id: event.correlation_id,
            payload_hash: event.payload_hash,
            source_component: event.source_component,
            payload: event.payload,
            created_at: Datetime::from(event.created_at),
        }
    }

    #[test]
    fn atomic_batch_results_follow_caller_order() {
        let (first, _) = prepare_event(event("mt-136-batch-first", json!({"ordinal": 1})))
            .expect("prepare first event");
        let (second, _) = prepare_event(event("mt-136-batch-second", json!({"ordinal": 2})))
            .expect("prepare second event");
        let mut stored_first = first.clone();
        stored_first.event_id = "KE-stored-first".to_owned();
        let mut stored_second = second.clone();
        stored_second.event_id = "KE-stored-second".to_owned();

        let ordered = order_and_validate(
            vec![
                row_from_event(stored_second, 42),
                row_from_event(stored_first, 41),
            ],
            &[first, second],
        )
        .expect("order stored events");

        assert_eq!(ordered[0].event_id, "KE-stored-first");
        assert_eq!(ordered[0].event_sequence, 41);
        assert_eq!(ordered[1].event_id, "KE-stored-second");
        assert_eq!(ordered[1].event_sequence, 42);
    }

    #[test]
    fn atomic_pair_uses_actual_replayed_first_event_as_causation() {
        let (first, _) = prepare_event(event("mt-136-pair-first", json!({"ordinal": 1})))
            .expect("prepare first event");
        let mut second_new = event("mt-136-pair-second", json!({"ordinal": 2}));
        second_new.causation_id = Some(first.event_id.clone());
        let (second, _) = prepare_event(second_new).expect("prepare second event");

        let mut stored_first = first.clone();
        stored_first.event_id = "KE-durable-first".to_owned();
        let mut stored_second = second.clone();
        stored_second.event_id = "KE-durable-second".to_owned();
        stored_second.causation_id = Some(stored_first.event_id.clone());

        let ordered = order_pair_and_validate(
            vec![
                row_from_event(stored_second, 12),
                row_from_event(stored_first, 11),
            ],
            &[first, second],
        )
        .expect("order stored pair");

        assert_eq!(ordered[0].event_id, "KE-durable-first");
        assert_eq!(ordered[1].causation_id.as_deref(), Some("KE-durable-first"));
    }

    async fn open(path: &std::path::Path) -> SurrealStorage {
        eprintln!("event-ledger-test stage=storage-open state=start");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid embedded test path"),
        )
        .await
        .expect("open embedded SurrealDB");
        eprintln!("event-ledger-test stage=storage-open state=complete");
        eprintln!("event-ledger-test stage=schema-bootstrap state=start");
        let (_, after_start) = include_str!("schema.surql")
            .split_once("-- 0018_kernel_event_ledger")
            .expect("compiled schema contains EventLedger start marker");
        let (ddl, _) = after_start
            .split_once("-- 0019_kernel_session_queue")
            .expect("compiled schema contains EventLedger end marker");
        let mut ddl = ddl.to_owned();
        // The MT-109 authority-scope fields of `kernel_event_ledger` (`authority_*`,
        // `wsids`) are defined at the end of the compiled schema, outside the 0018
        // slice, and the writer always binds them; include every field definition
        // the compiled schema declares for the table (MT-141 V2-R2).
        for line in include_str!("schema.surql").lines() {
            if line.starts_with("DEFINE FIELD OVERWRITE")
                && line.contains(" ON TABLE kernel_event_ledger TYPE")
                && !ddl.contains(line)
            {
                ddl.push('\n');
                ddl.push_str(line);
            }
        }
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    database.query(ddl).await?;
                    Ok(())
                })
            })
            .await
            .expect("bootstrap authoritative EventLedger schema slice");
        eprintln!("event-ledger-test stage=schema-bootstrap state=complete");
        storage
    }

    async fn within<T>(stage: &str, future: impl Future<Output = T>) -> T {
        eprintln!("event-ledger-test stage={stage} state=start");
        let result = tokio::time::timeout(Duration::from_secs(120), future)
            .await
            .unwrap_or_else(|_| panic!("event-ledger-test stage={stage} timed out after 120s"));
        eprintln!("event-ledger-test stage={stage} state=complete");
        result
    }

    #[test]
    fn receipt_lookup_plan_summary_rejects_unsafe_shapes_without_disclosing_values() {
        use handshake_storage_support::diagnostics::ReceiptLookupPlanKind;
        const SECRET: &str = "private-bound-value-must-not-be-emitted";
        let collector = json!({"operation": "Collector", "detail": {"type": "Memory"}});
        let index = json!({"operation": "Iterate Index", "detail": {
            "table": "kernel_event_ledger", "plan": {
                "index": "idx_kernel_event_ledger_idempotency", "operator": "=", "value": SECRET
            }
        }});
        let table = json!({"operation": "Iterate Table", "detail": {
            "table": "kernel_event_ledger", "direction": "forward"
        }});
        let envelope = |entries| json!({"entries": entries, "elapsed_us": 17}).into_value();
        let summarize = |value: Value| {
            let summary = summarize_receipt_lookup_plan(&value);
            // The exact type passed to the log emitter has no raw value/string fields.
            assert!(!format!("{summary:?}").contains(SECRET));
            summary
        };
        let expected = summarize(envelope(json!([index, collector])));
        assert!(expected.supported_shape && expected.expected_index_used);
        assert_eq!(expected.plan_kind, ReceiptLookupPlanKind::Index);
        assert_eq!(expected.entry_count, 2);
        assert_eq!(expected.explain_elapsed_us, 17);
        let scanned = summarize(envelope(json!([table, collector])));
        assert!(scanned.supported_shape && !scanned.expected_index_used);
        assert_eq!(scanned.plan_kind, ReceiptLookupPlanKind::Table);
        let none = summarize(envelope(json!([collector])));
        assert!(none.supported_shape && !none.expected_index_used);
        assert_eq!(none.plan_kind, ReceiptLookupPlanKind::None);
        let fallback = json!({"operation": "Fallback", "detail": {"reason": SECRET}});
        assert!(summarize(envelope(json!([table, fallback, collector]))).supported_shape);
        let mut other_index = index.clone();
        other_index["detail"]["plan"]["index"] = json!(SECRET);
        let other_index = summarize(envelope(json!([other_index, collector])));
        assert!(other_index.supported_shape && !other_index.expected_index_used);
        let mut foreign_table = index.clone();
        foreign_table["detail"]["table"] = json!(SECRET);
        let mut extra_field = index.clone();
        extra_field["detail"]["plan"]["unexpected"] = json!(SECRET);
        let mut wrong_operator = index.clone();
        wrong_operator["detail"]["plan"]["operator"] = json!(SECRET);
        for malformed in [
            Value::Null,
            envelope(json!([])),
            envelope(json!([{"operation": SECRET, "detail": {}}])),
            envelope(json!([foreign_table, collector])),
            envelope(json!([extra_field, collector])),
            envelope(json!([wrong_operator, collector])),
            envelope(json!([index, table, collector])),
            envelope(json!([index, collector, collector])),
            envelope(json!(vec![collector.clone(); 17])),
            json!({"entries": [index, collector], "elapsed_us": SECRET}).into_value(),
        ] {
            let unsupported = summarize(malformed);
            assert!(!unsupported.supported_shape && !unsupported.expected_index_used);
            assert_eq!(unsupported.plan_kind, ReceiptLookupPlanKind::Other);
            assert!(unsupported.entry_count <= 16);
        }
        let invalid_clock =
            summarize(json!({"entries": [index, collector], "elapsed_us": -1}).into_value());
        assert!(invalid_clock.supported_shape);
        assert_eq!(invalid_clock.explain_elapsed_us, -1);

        let physical_index = json!({"operator": "IndexScan", "context": "Db", "attributes": {
            "index": "idx_kernel_event_ledger_idempotency", "access": format!("= '{SECRET}'"),
            "direction": "Forward", "limit": "1"
        }});
        let project = |child| {
            json!({"operator": "ProjectValue", "context": "Db",
            "attributes": {"expr": SECRET},
            "expressions": [{"role": "expr", "sql": SECRET}], "children": [child]})
        };
        let expected = summarize(envelope(project(physical_index.clone())));
        assert!(expected.supported_shape && expected.expected_index_used);
        assert_eq!(expected.plan_kind, ReceiptLookupPlanKind::Index);
        assert_eq!(expected.entry_count, 2);
        let limited = summarize(envelope(json!({"operator": "Limit", "context": "Db",
            "attributes": {"limit": "1"},
            "expressions": [{"role": "limit", "sql": "1"}],
            "children": [physical_index]})));
        assert!(limited.supported_shape && limited.expected_index_used);
        let physical_table = json!({"operator": "TableScan", "context": "Db", "attributes": {
            "table": "kernel_event_ledger", "direction": "Forward", "predicate": SECRET
        }});
        let scanned = summarize(envelope(project(physical_table.clone())));
        assert!(scanned.supported_shape && !scanned.expected_index_used);
        assert_eq!(scanned.plan_kind, ReceiptLookupPlanKind::Table);
        let none = summarize(envelope(json!({"operator": "EmptyScan", "context": "Rt"})));
        assert!(none.supported_shape && !none.expected_index_used);
        assert_eq!(none.plan_kind, ReceiptLookupPlanKind::None);
        let mut other_index = physical_index.clone();
        other_index["attributes"]["index"] = json!(SECRET);
        let other_index = summarize(envelope(other_index));
        assert!(other_index.supported_shape && !other_index.expected_index_used);
        let mut wrong_access = physical_index.clone();
        wrong_access["attributes"]["access"] = json!(format!("> '{SECRET}'"));
        let mut extra_attribute = physical_index.clone();
        extra_attribute["attributes"]["unexpected"] = json!(SECRET);
        let mut foreign_table = physical_table.clone();
        foreign_table["attributes"]["table"] = json!(SECRET);
        let mut embedded = project(physical_index.clone());
        embedded["expressions"][0]["embedded_operators"] =
            json!([{"role": SECRET, "plan": physical_index}]);
        let mut multiple = project(physical_index.clone());
        multiple["children"] = json!([physical_index, physical_table]);
        let mut oversized = physical_index.clone();
        for _ in 0..16 {
            oversized = project(oversized);
        }
        for malformed in [
            json!({"operator": "DynamicScan", "context": "Db", "attributes": {"source": SECRET}}),
            json!({"operator": SECRET, "context": "Db"}),
            json!({"operator": "IndexScan", "context": "Db", "attributes": SECRET}),
            wrong_access,
            extra_attribute,
            foreign_table,
            embedded,
            multiple,
            oversized,
        ] {
            let unsupported = summarize(envelope(malformed));
            assert!(!unsupported.supported_shape && !unsupported.expected_index_used);
            assert_eq!(unsupported.plan_kind, ReceiptLookupPlanKind::Other);
            assert!(unsupported.entry_count <= 16);
            assert_eq!(unsupported.explain_elapsed_us, 17);
        }
    }

    #[tokio::test]
    async fn measured_receipt_decoding_preserves_empty_denial_and_rejects_extra_rows() {
        use handshake_storage_support::diagnostics::{
            observe_result, ReceiptTimingContext, DOCUMENT_REQUEST_ID,
        };
        DOCUMENT_REQUEST_ID
            .scope(
                uuid::Uuid::new_v4().to_string(),
                observe_result("receipt_append", async {
                    let timing =
                        ReceiptTimingContext::capture().expect("both diagnostic scopes entered");
                    let envelope = |rows| TimedLedgerAppend {
                        rows,
                        replay: false,
                        lookup_elapsed_us: -1,
                        operation_elapsed_us: 0,
                        lookup_plan: Value::None,
                    };
                    assert!(decode_timed_append(vec![envelope(Value::None)], &timing)
                        .unwrap()
                        .is_none());
                    let mut unsupported_plan = envelope(Value::None);
                    unsupported_plan.lookup_plan = Value::Null;
                    assert!(decode_timed_append(vec![unsupported_plan], &timing)
                        .unwrap()
                        .is_none());
                    assert!(decode_timed_append(
                        vec![envelope(Vec::<Value>::new().into_value())],
                        &timing
                    )
                    .unwrap()
                    .is_none());
                    assert!(decode_timed_append(vec![envelope(Value::Null)], &timing).is_err());
                    assert!(decode_timed_append(
                        vec![envelope(Value::None), envelope(Value::None)],
                        &timing
                    )
                    .is_err());
                    let (candidate, _) =
                        prepare_event(event("mt032-measured-decode", json!({"value": 1}))).unwrap();
                    let row = row_from_event(candidate.clone(), 1).into_value();
                    assert!(decode_timed_append(
                        vec![envelope(vec![row.clone(), row.clone()].into_value())],
                        &timing
                    )
                    .is_err());
                    // Invalid wall-clock samples must not change an otherwise valid receipt result.
                    let decoded =
                        decode_timed_append(vec![envelope(vec![row].into_value())], &timing)
                            .unwrap()
                            .unwrap();
                    assert_eq!(decoded.event_id, candidate.event_id);
                    Ok::<_, StorageError>(())
                }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn measured_receipt_append_preserves_exact_replay_and_conflict() {
        use handshake_storage_support::diagnostics::{
            observe_result, ReceiptTimingContext, DOCUMENT_REQUEST_ID,
        };
        let directory = tempfile::tempdir().expect("temporary measured receipt store");
        let storage = open(&directory.path().join("store")).await;
        let enabled = handshake_storage_support::diagnostics::receipt_lookup_plan_enabled();
        let run = |payload, expected_replay| {
            let storage = &storage;
            RECEIPT_PLAN_TEST_OBSERVATIONS.scope(std::cell::RefCell::new(Vec::new()), async move {
                let result = DOCUMENT_REQUEST_ID
                    .scope(
                        uuid::Uuid::new_v4().to_string(),
                        observe_result("receipt_append", async move {
                            assert!(ReceiptTimingContext::capture().is_some());
                            append(storage, event("mt032-measured-replay", payload)).await
                        }),
                    )
                    .await;
                RECEIPT_PLAN_TEST_OBSERVATIONS.with(|observations| {
                    let observations = observations.borrow();
                    assert_eq!(
                        observations.len(),
                        1,
                        "one live decoder observation per append"
                    );
                    let observation = observations[0];
                    // The capture contains only closed types and timings, never the raw plan.
                    assert_eq!(observation.replay, expected_replay);
                    assert!(
                        observation.lookup_elapsed_us >= 0 && observation.operation_elapsed_us >= 0,
                        "invalid live receipt timings: {observation:?}"
                    );
                    if enabled {
                        assert!(
                            matches!(
                                observation.shape,
                                ReceiptPlanShape::LegacyArray | ReceiptPlanShape::PhysicalObject
                            ),
                            "unsupported live plan envelope: {observation:?}"
                        );
                        let summary = observation.summary.expect("enabled live plan observation");
                        assert!(
                            summary.supported_shape
                                && summary.entry_count > 0
                                && summary.entry_count <= 16
                                && summary.explain_elapsed_us >= 0,
                            "unsupported live plan or invalid timing: {observation:?}"
                        );
                    } else {
                        assert!(matches!(observation.shape, ReceiptPlanShape::Disabled));
                        assert!(observation.summary.is_none());
                    }
                });
                result
            })
        };
        let inserted = run(json!({"value": 1}), false)
            .await
            .expect("measured CREATE");
        let replay = run(json!({"value": 1}), true)
            .await
            .expect("measured replay SELECT");
        assert_eq!(inserted.event_id, replay.event_id);
        assert_eq!(inserted.event_sequence, replay.event_sequence);
        assert!(matches!(
            run(json!({"value": 2}), true).await,
            Err(StorageError::Conflict(_) | StorageError::ConflictDetails { .. })
        ));
        let persisted = get_by_idempotency(&storage, "mt032-measured-replay")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted.event_id, inserted.event_id);
        assert_eq!(persisted.payload, json!({"value": 1}));
        storage
            .shutdown()
            .await
            .expect("close measured receipt store");
    }

    #[tokio::test]
    async fn event_ledger_round_trip_survives_shutdown_and_reopen() {
        let directory = tempfile::tempdir().expect("temporary MT-136 store root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let inserted = append(
            &storage,
            event("mt-136-durable-event", json!({"proof": "before-reopen"})),
        )
        .await
        .expect("append EventLedger row");
        assert!(inserted.event_sequence > 0);
        storage.shutdown().await.expect("close embedded store");
        drop(storage);

        let reopened = open(&path).await;
        let persisted = get_by_idempotency(&reopened, "mt-136-durable-event")
            .await
            .expect("read reopened EventLedger")
            .expect("durable EventLedger row");
        assert_eq!(persisted.event_id, inserted.event_id);
        assert_eq!(persisted.event_sequence, inserted.event_sequence);
        assert_eq!(persisted.payload, json!({"proof": "before-reopen"}));
        reopened.shutdown().await.expect("close reopened store");
    }

    #[tokio::test]
    async fn concurrent_exact_replays_converge_and_conflicting_replay_fails_closed() {
        let directory = tempfile::tempdir().expect("temporary MT-136 store root");
        let storage = open(&directory.path().join("store")).await;
        let left_storage = storage.clone();
        let right_storage = storage.clone();
        let left = tokio::spawn(async move {
            append(
                &left_storage,
                event("mt-136-concurrent-event", json!({"value": 1})),
            )
            .await
        });
        let right = tokio::spawn(async move {
            append(
                &right_storage,
                event("mt-136-concurrent-event", json!({"value": 1})),
            )
            .await
        });
        let left = left.await.expect("left append task").expect("left append");
        let right = right
            .await
            .expect("right append task")
            .expect("right append");
        assert_eq!(left.event_id, right.event_id);
        assert_eq!(left.event_sequence, right.event_sequence);

        let conflict = append(
            &storage,
            event("mt-136-concurrent-event", json!({"value": 2})),
        )
        .await;
        assert!(matches!(
            conflict,
            Err(StorageError::Conflict(_) | StorageError::ConflictDetails { .. })
        ));
        storage.shutdown().await.expect("close embedded store");
    }

    #[tokio::test]
    async fn atomic_batch_bulk_insert_preserves_replay_and_rollback_contracts() {
        let directory = tempfile::tempdir().expect("temporary atomic batch store root");
        let storage = within("open", open(&directory.path().join("store"))).await;

        let inserted = within(
            "initial-insert",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 1})),
                    event("mt-136-bulk-second", json!({"ordinal": 2})),
                ],
            ),
        )
        .await
        .expect("bulk insert events");
        assert_eq!(inserted.len(), 2);
        assert!(inserted.iter().all(|stored| stored.event_sequence > 0));

        let replayed = within(
            "exact-replay",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 1})),
                    event("mt-136-bulk-second", json!({"ordinal": 2})),
                ],
            ),
        )
        .await
        .expect("exact bulk replay");
        assert_eq!(
            replayed
                .iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>(),
            inserted
                .iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>()
        );

        let mixed = within(
            "mixed-replay-insert",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 1})),
                    event("mt-136-bulk-third", json!({"ordinal": 3})),
                ],
            ),
        )
        .await
        .expect("mixed replay and insert");
        assert_eq!(mixed[0].event_id, inserted[0].event_id);
        assert_ne!(mixed[1].event_id, inserted[0].event_id);

        let duplicate = within(
            "internal-exact-duplicate",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-duplicate", json!({"ordinal": 4})),
                    event("mt-136-bulk-duplicate", json!({"ordinal": 4})),
                ],
            ),
        )
        .await
        .expect("exact duplicate inside one batch");
        assert_eq!(duplicate[0].event_id, duplicate[1].event_id);

        let conflict = within(
            "stored-conflict",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-first", json!({"ordinal": 999})),
                    event("mt-136-bulk-must-rollback", json!({"ordinal": 5})),
                ],
            ),
        )
        .await;
        assert!(matches!(
            conflict,
            Err(StorageError::Conflict(_) | StorageError::ConflictDetails { .. })
        ));
        assert!(get_by_idempotency(&storage, "mt-136-bulk-must-rollback")
            .await
            .expect("read rolled-back event")
            .is_none());

        let internal_conflict = within(
            "internal-conflict",
            append_atomic(
                &storage,
                vec![
                    event("mt-136-bulk-internal-conflict", json!({"ordinal": 6})),
                    event("mt-136-bulk-internal-conflict", json!({"ordinal": 7})),
                    event("mt-136-bulk-internal-must-rollback", json!({"ordinal": 8})),
                ],
            ),
        )
        .await;
        assert!(matches!(
            internal_conflict,
            Err(StorageError::Conflict(_) | StorageError::ConflictDetails { .. })
        ));
        assert!(
            get_by_idempotency(&storage, "mt-136-bulk-internal-conflict")
                .await
                .expect("read conflicting event")
                .is_none()
        );
        assert!(
            get_by_idempotency(&storage, "mt-136-bulk-internal-must-rollback")
                .await
                .expect("read internal rollback event")
                .is_none()
        );

        let left_storage = storage.clone();
        let right_storage = storage.clone();
        let (left, right) = within("concurrent-exact-batches", async move {
            tokio::join!(
                append_atomic(
                    &left_storage,
                    vec![
                        event("mt-136-bulk-race-first", json!({"ordinal": 9})),
                        event("mt-136-bulk-race-second", json!({"ordinal": 10})),
                    ],
                ),
                append_atomic(
                    &right_storage,
                    vec![
                        event("mt-136-bulk-race-first", json!({"ordinal": 9})),
                        event("mt-136-bulk-race-second", json!({"ordinal": 10})),
                    ],
                ),
            )
        })
        .await;
        let left = left.expect("left concurrent bulk insert");
        let right = right.expect("right concurrent bulk insert");
        assert_eq!(
            left.iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>(),
            right
                .iter()
                .map(|stored| stored.event_id.as_str())
                .collect::<Vec<_>>()
        );

        storage.shutdown().await.expect("close embedded store");
    }
}
