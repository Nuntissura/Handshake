use super::{SurrealStorage, SurrealStorageError};
use crate::{
    storage::WORKSPACE_SETTINGS_SCHEMA_ID,
    swarm_orchestration::resource_scope::{ExactResourceScopeAttribution, ResourceScope},
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};
use surrealdb::types::SurrealValue;
use thiserror::Error;

const SCHEMA: &str = include_str!("workspace_settings_schema.surql");
const SCHEMA_STATE: &str = "\
DEFINE TABLE IF NOT EXISTS workspace_settings_schema_state SCHEMAFULL;\
DEFINE FIELD IF NOT EXISTS schema_version ON workspace_settings_schema_state TYPE string;\
DEFINE FIELD IF NOT EXISTS schema_revision ON workspace_settings_schema_state TYPE int;\
DEFINE FIELD IF NOT EXISTS apply_state ON workspace_settings_schema_state TYPE string;";
const SCHEMA_STATE_ID: &str = "workspace_settings_schema_state:primary";
const SCHEMA_VERSION: &str = "mt021-workspace-settings-authority-v1";
const SCHEMA_REVISION: i64 = 1;
const SETTINGS_SHAPE_ERROR: &str =
    "workspace settings_state must match hsk.workspace_settings_state@1 shape";
const KEYBINDING_ACTION_IDS: [&str; 2] = ["app.quick_switcher.open", "app.command_palette.open"];

const READ_EXACT_SCOPE: &str = "\
SELECT workspace_id, settings_state, generation, updated_at, record::id(event_ledger_event_id) AS event_ledger_event_id \
FROM workspace_settings_authority \
WHERE owner_account_id = $owner_account_id \
  AND actor_principal_id = $actor_principal_id \
  AND authenticated_session_id = $authenticated_session_id \
  AND access_space_id = $access_space_id \
  AND workspace_id = $workspace_id \
  AND storage_authority = 'embedded_surrealdb' \
  AND event_ledger_event_id.owner_account_id = $owner_account_id \
  AND event_ledger_event_id.actor_principal_id = $actor_principal_id \
  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id \
  AND event_ledger_event_id.access_space_id = $access_space_id \
  AND event_ledger_event_id.workspace_id = $workspace_id \
  AND event_ledger_event_id.aggregate_type = 'workspace_settings_state' \
  AND event_ledger_event_id.event_type = 'KNOWLEDGE_WORKSPACE_SETTINGS_STATE_RECORDED' \
LIMIT 2;";

const WRITE_EXACT_SCOPE: &str = r#"
BEGIN TRANSACTION;
LET $current = (
    SELECT workspace_id, settings_state, generation, updated_at,
           record::id(event_ledger_event_id) AS event_ledger_event_id
    FROM type::record('workspace_settings_authority', $record_id)
    WHERE owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id
      AND storage_authority = 'embedded_surrealdb'
      AND event_ledger_event_id.owner_account_id = $owner_account_id
      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
      AND event_ledger_event_id.access_space_id = $access_space_id
      AND event_ledger_event_id.workspace_id = $workspace_id
      AND event_ledger_event_id.aggregate_type = 'workspace_settings_state'
      AND event_ledger_event_id.event_type = 'KNOWLEDGE_WORKSPACE_SETTINGS_STATE_RECORDED'
    LIMIT 2
);
LET $prior = (
    SELECT payload_hash AS settings_state_sha256
    FROM kernel_event_ledger
    WHERE owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id
      AND idempotency_key = $idempotency_key
      AND aggregate_type = 'workspace_settings_state'
      AND event_type = 'KNOWLEDGE_WORKSPACE_SETTINGS_STATE_RECORDED'
      AND source_component = 'workspace_settings_state'
    LIMIT 2
);
IF array::len($current) > 1 OR array::len($prior) > 1 {
    RETURN { outcome: 'ambiguous', actual_generation: -1, record: NONE };
} ELSE IF array::len($prior) = 1 {
    IF $prior[0].settings_state_sha256 != $settings_state_sha256 {
        RETURN { outcome: 'idempotency_conflict', actual_generation: IF array::len($current) = 0 { 0 } ELSE { $current[0].generation }, record: NONE };
    };
    RETURN { outcome: 'already_applied', actual_generation: IF array::len($current) = 0 { 0 } ELSE { $current[0].generation }, record: IF array::len($current) = 0 { NONE } ELSE { $current[0] } };
} ELSE {
    LET $actual_generation = IF array::len($current) = 0 { 0 } ELSE { $current[0].generation };
    IF $expected_generation >= 0 AND $expected_generation != $actual_generation {
        RETURN { outcome: 'stale', actual_generation: $actual_generation, record: NONE };
    };
    LET $next_generation = $actual_generation + 1;
    LET $ledger = CREATE type::record('kernel_event_ledger', $event_record_id) CONTENT {
        event_id: $event_id,
        event_version: 'kernel_event_v1',
        kernel_task_run_id: $kernel_task_run_id,
        session_run_id: $authenticated_session_id,
        idempotency_key: $idempotency_key,
        aggregate_type: 'workspace_settings_state',
        aggregate_id: $workspace_id,
        event_type: 'KNOWLEDGE_WORKSPACE_SETTINGS_STATE_RECORDED',
        actor_kind: 'principal',
        actor_id: $actor_principal_id,
        causation_id: NONE,
        correlation_id: NONE,
        payload_hash: $settings_state_sha256,
        source_component: 'workspace_settings_state',
        owner_account_id: $owner_account_id,
        actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id,
        access_space_id: $access_space_id,
        workspace_id: $workspace_id,
        payload: {
            type: 'knowledge_workspace_settings_state_recorded',
            workspace_id: $workspace_id,
            generation: $next_generation,
            settings_state: $settings_state
        },
        created_at: time::now()
    };
    LET $stored = IF array::len($current) = 0 {
        CREATE type::record('workspace_settings_authority', $record_id) CONTENT {
            owner_account_id: $owner_account_id,
            actor_principal_id: $actor_principal_id,
            authenticated_session_id: $authenticated_session_id,
            access_space_id: $access_space_id,
            workspace_id: $workspace_id,
            settings_state: $settings_state,
            generation: $next_generation,
            updated_at: time::now(),
            event_ledger_event_id: type::record('kernel_event_ledger', $event_record_id),
            storage_authority: 'embedded_surrealdb'
        }
    } ELSE {
        UPDATE type::record('workspace_settings_authority', $record_id) CONTENT {
            owner_account_id: $owner_account_id,
            actor_principal_id: $actor_principal_id,
            authenticated_session_id: $authenticated_session_id,
            access_space_id: $access_space_id,
            workspace_id: $workspace_id,
            settings_state: $settings_state,
            generation: $next_generation,
            updated_at: time::now(),
            event_ledger_event_id: type::record('kernel_event_ledger', $event_record_id),
            storage_authority: 'embedded_surrealdb'
        }
        WHERE owner_account_id = $owner_account_id
          AND actor_principal_id = $actor_principal_id
          AND authenticated_session_id = $authenticated_session_id
          AND access_space_id = $access_space_id
          AND workspace_id = $workspace_id
          AND storage_authority = 'embedded_surrealdb'
    };
    LET $verified = (
        SELECT workspace_id, settings_state, generation, updated_at,
               record::id(event_ledger_event_id) AS event_ledger_event_id
        FROM type::record('workspace_settings_authority', $record_id)
        WHERE owner_account_id = $owner_account_id
          AND actor_principal_id = $actor_principal_id
          AND authenticated_session_id = $authenticated_session_id
          AND access_space_id = $access_space_id
          AND workspace_id = $workspace_id
          AND storage_authority = 'embedded_surrealdb'
          AND event_ledger_event_id.owner_account_id = $owner_account_id
          AND event_ledger_event_id.actor_principal_id = $actor_principal_id
          AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
          AND event_ledger_event_id.access_space_id = $access_space_id
          AND event_ledger_event_id.workspace_id = $workspace_id
          AND event_ledger_event_id.aggregate_type = 'workspace_settings_state'
          AND event_ledger_event_id.event_type = 'KNOWLEDGE_WORKSPACE_SETTINGS_STATE_RECORDED'
        LIMIT 2
    );
    IF array::len($verified) != 1 { THROW 'workspace-settings exact-scope mutation verification failed'; };
    RETURN { outcome: 'stored', actual_generation: $next_generation, record: $verified[0] };
};
COMMIT TRANSACTION;
"#;

#[derive(Clone, Debug, PartialEq)]
pub struct SurrealWorkspaceSettingsState {
    pub workspace_id: String,
    pub settings_state: Value,
    pub generation: i64,
    pub updated_at: DateTime<Utc>,
    pub event_ledger_event_id: String,
}

#[derive(Clone, Debug)]
pub struct SurrealWorkspaceSettingsWrite {
    pub settings_state: Value,
    pub expected_generation: Option<i64>,
    pub idempotency_key: String,
}

#[derive(Debug, Error)]
pub enum SurrealWorkspaceSettingsError {
    #[error("embedded workspace-settings storage failed: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("workspace settings require an exact five-field ResourceScope")]
    IncompleteScope,
    #[error("workspace path does not match authenticated scope")]
    WorkspaceScopeMismatch,
    #[error("{0}")]
    Validation(&'static str),
    #[error("workspace settings write is stale (expected {expected}, current {actual})")]
    StaleGeneration { expected: i64, actual: i64 },
    #[error("workspace settings idempotency key was reused for different content")]
    IdempotencyConflict,
    #[error("embedded workspace-settings authority is ambiguous or incomplete")]
    CorruptAuthority,
}

#[derive(Clone)]
pub struct SurrealWorkspaceSettingsStore {
    storage: SurrealStorage,
}

#[derive(Clone, Debug, SurrealValue)]
struct ScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct WriteBindings {
    record_id: String,
    event_record_id: String,
    event_id: String,
    kernel_task_run_id: String,
    idempotency_key: String,
    settings_state_sha256: String,
    settings_state: Value,
    expected_generation: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Clone, Debug, SurrealValue)]
struct StoredStateValue {
    workspace_id: String,
    settings_state: Value,
    generation: i64,
    updated_at: DateTime<Utc>,
    event_ledger_event_id: String,
}

impl From<StoredStateValue> for SurrealWorkspaceSettingsState {
    fn from(value: StoredStateValue) -> Self {
        Self {
            workspace_id: value.workspace_id,
            settings_state: value.settings_state,
            generation: value.generation,
            updated_at: value.updated_at,
            event_ledger_event_id: value.event_ledger_event_id,
        }
    }
}

#[derive(Debug, SurrealValue)]
struct MutationResult {
    outcome: String,
    actual_generation: i64,
    record: Option<StoredStateValue>,
}

#[derive(Clone, Debug, SurrealValue)]
struct SchemaState {
    schema_version: String,
    schema_revision: i64,
    apply_state: String,
}

#[derive(Debug, SurrealValue)]
struct SchemaStateBindings {
    schema_version: String,
    schema_revision: i64,
}

pub async fn bootstrap_workspace_settings_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    super::bootstrap_schema(storage).await?;
    storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                database.query(SCHEMA_STATE).await?;
                let mut response = database
                    .query(format!("SELECT * FROM ONLY {SCHEMA_STATE_ID};"))
                    .await?;
                let state: Option<SchemaState> = response.take(0)?;
                if let Some(state) = state.as_ref() {
                    if state.schema_version != SCHEMA_VERSION
                        || state.schema_revision != SCHEMA_REVISION
                        || state.apply_state != "complete"
                    {
                        return Err(SurrealStorageError::InvalidWorkspaceRecord {
                            reason: "workspace-settings schema state version/revision mismatch",
                        });
                    }
                }
                database.query(SCHEMA).await?;
                if state.is_none() {
                    database
                        .query_bound(
                            "UPSERT workspace_settings_schema_state:primary CONTENT { schema_version: $schema_version, schema_revision: $schema_revision, apply_state: 'complete' };",
                            SchemaStateBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                            },
                        )
                        .await?;
                }
                Ok(())
            })
        })
        .await
}

impl SurrealWorkspaceSettingsStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub async fn initialize(
        storage: SurrealStorage,
    ) -> Result<Self, SurrealWorkspaceSettingsError> {
        bootstrap_workspace_settings_schema(&storage).await?;
        Ok(Self { storage })
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub async fn get(
        &self,
        scope: &ResourceScope,
        workspace_id: &str,
    ) -> Result<Option<SurrealWorkspaceSettingsState>, SurrealWorkspaceSettingsError> {
        let bindings = exact_scope_bindings(scope, workspace_id)?;
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredStateValue, _>(READ_EXACT_SCOPE, bindings)
                        .await
                })
            })
            .await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(rows.into_iter().next().map(Into::into)),
            _ => Err(SurrealWorkspaceSettingsError::CorruptAuthority),
        }
    }

    pub async fn save(
        &self,
        scope: &ResourceScope,
        workspace_id: &str,
        write: SurrealWorkspaceSettingsWrite,
    ) -> Result<SurrealWorkspaceSettingsState, SurrealWorkspaceSettingsError> {
        validate_settings_state(&write.settings_state)?;
        if write.idempotency_key.trim().is_empty() {
            return Err(SurrealWorkspaceSettingsError::Validation(
                "workspace settings idempotency key must not be blank",
            ));
        }
        if matches!(write.expected_generation, Some(value) if value < 0) {
            return Err(SurrealWorkspaceSettingsError::Validation(
                "workspace settings expected_generation must be non-negative",
            ));
        }
        let scope_bindings = exact_scope_bindings(scope, workspace_id)?;
        let state_bytes = serde_json::to_vec(&write.settings_state)
            .map_err(|_| SurrealWorkspaceSettingsError::Validation(SETTINGS_SHAPE_ERROR))?;
        let settings_state_sha256 = format!("{:x}", Sha256::digest(state_bytes));
        let record_id = stable_scope_id(&scope_bindings);
        let event_record_id = stable_event_id(&scope_bindings, &write.idempotency_key);
        let event_id = event_record_id.clone();
        let idempotency_key = format!("workspace-settings:{event_record_id}");
        let kernel_task_run_id = format!("WORKSPACE-SETTINGS-{workspace_id}");
        let expected_generation = write.expected_generation.unwrap_or(-1);
        let bindings = WriteBindings {
            record_id,
            event_record_id,
            event_id,
            kernel_task_run_id,
            idempotency_key,
            settings_state_sha256,
            settings_state: write.settings_state,
            expected_generation,
            owner_account_id: scope_bindings.owner_account_id,
            actor_principal_id: scope_bindings.actor_principal_id,
            authenticated_session_id: scope_bindings.authenticated_session_id,
            access_space_id: scope_bindings.access_space_id,
            workspace_id: scope_bindings.workspace_id,
        };
        let mut results = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<MutationResult, _>(WRITE_EXACT_SCOPE, bindings, 3)
                        .await
                })
            })
            .await?;
        let result = results
            .pop()
            .ok_or(SurrealWorkspaceSettingsError::CorruptAuthority)?;
        match result.outcome.as_str() {
            "stored" | "already_applied" => result
                .record
                .map(Into::into)
                .ok_or(SurrealWorkspaceSettingsError::CorruptAuthority),
            "stale" => Err(SurrealWorkspaceSettingsError::StaleGeneration {
                expected: expected_generation,
                actual: result.actual_generation,
            }),
            "idempotency_conflict" => Err(SurrealWorkspaceSettingsError::IdempotencyConflict),
            _ => Err(SurrealWorkspaceSettingsError::CorruptAuthority),
        }
    }
}

fn exact_scope_bindings(
    scope: &ResourceScope,
    workspace_id: &str,
) -> Result<ScopeBindings, SurrealWorkspaceSettingsError> {
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope)
        .map_err(|_| SurrealWorkspaceSettingsError::IncompleteScope)?;
    if exact.workspace_id.as_str() != workspace_id {
        return Err(SurrealWorkspaceSettingsError::WorkspaceScopeMismatch);
    }
    Ok(ScopeBindings {
        owner_account_id: exact.owner_account_id.to_string(),
        actor_principal_id: exact.actor_principal_id.to_string(),
        authenticated_session_id: exact.authenticated_session_id.to_string(),
        access_space_id: exact.access_space_id.to_string(),
        workspace_id: exact.workspace_id.as_str().to_owned(),
    })
}

fn stable_scope_id(scope: &ScopeBindings) -> String {
    let mut hasher = Sha256::new();
    for value in [
        scope.owner_account_id.as_str(),
        scope.actor_principal_id.as_str(),
        scope.authenticated_session_id.as_str(),
        scope.access_space_id.as_str(),
        scope.workspace_id.as_str(),
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("scope-{:x}", hasher.finalize())
}

fn stable_event_id(scope: &ScopeBindings, idempotency_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(stable_scope_id(scope).as_bytes());
    hasher.update((idempotency_key.len() as u64).to_be_bytes());
    hasher.update(idempotency_key.as_bytes());
    format!("event-{:x}", hasher.finalize())
}

fn validate_settings_state(value: &Value) -> Result<(), SurrealWorkspaceSettingsError> {
    let Some(object) = value.as_object() else {
        return Err(SurrealWorkspaceSettingsError::Validation(
            "workspace settings_state must be a JSON object",
        ));
    };
    if object.get("schema_id").and_then(Value::as_str) != Some(WORKSPACE_SETTINGS_SCHEMA_ID) {
        return Err(SurrealWorkspaceSettingsError::Validation(
            "workspace settings_state schema_id must be hsk.workspace_settings_state@1",
        ));
    }
    if !matches!(
        object.get("theme").and_then(Value::as_str),
        Some("light" | "dark")
    ) {
        return Err(SurrealWorkspaceSettingsError::Validation(
            SETTINGS_SHAPE_ERROR,
        ));
    }
    let Some(custom_theme_tokens) = object.get("custom_theme_tokens").and_then(Value::as_object)
    else {
        return Err(SurrealWorkspaceSettingsError::Validation(
            SETTINGS_SHAPE_ERROR,
        ));
    };
    if !custom_theme_tokens
        .iter()
        .all(|(key, token)| key.starts_with("--hs-color-") && token.is_string())
    {
        return Err(SurrealWorkspaceSettingsError::Validation(
            SETTINGS_SHAPE_ERROR,
        ));
    }
    let Some(keybindings) = object.get("keybindings").and_then(Value::as_object) else {
        return Err(SurrealWorkspaceSettingsError::Validation(
            SETTINGS_SHAPE_ERROR,
        ));
    };
    if !keybindings
        .keys()
        .all(|key| KEYBINDING_ACTION_IDS.contains(&key.as_str()))
        || !KEYBINDING_ACTION_IDS
            .iter()
            .all(|action| keybindings.contains_key(*action))
    {
        return Err(SurrealWorkspaceSettingsError::Validation(
            SETTINGS_SHAPE_ERROR,
        ));
    }
    let mut normalized = HashSet::new();
    for action in KEYBINDING_ACTION_IDS {
        let chord = keybindings
            .get(action)
            .and_then(Value::as_str)
            .and_then(normalize_chord)
            .ok_or(SurrealWorkspaceSettingsError::Validation(
                SETTINGS_SHAPE_ERROR,
            ))?;
        if !normalized.insert(chord) {
            return Err(SurrealWorkspaceSettingsError::Validation(
                "workspace settings_state duplicate keybinding chord",
            ));
        }
    }
    let Some(settings) = object.get("settings").and_then(Value::as_object) else {
        return Err(SurrealWorkspaceSettingsError::Validation(
            SETTINGS_SHAPE_ERROR,
        ));
    };
    if !matches!(
        settings.get("view_mode").and_then(Value::as_str),
        Some("NSFW" | "SFW")
    ) || !settings
        .get("swarm_board_default_open")
        .is_some_and(Value::is_boolean)
    {
        return Err(SurrealWorkspaceSettingsError::Validation(
            SETTINGS_SHAPE_ERROR,
        ));
    }
    Ok(())
}

fn normalize_chord(value: &str) -> Option<String> {
    let mut parts = value
        .split('-')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let key = parts.pop()?;
    let mut modifiers = BTreeSet::new();
    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "mod" | "cmd" | "command" | "meta" | "ctrl" | "control" => {
                modifiers.insert("Mod");
            }
            "alt" | "option" => {
                modifiers.insert("Alt");
            }
            "shift" => {
                modifiers.insert("Shift");
            }
            _ => return None,
        }
    }
    let key = if key.chars().count() == 1 {
        key.to_ascii_lowercase()
    } else {
        key.to_owned()
    };
    let mut normalized = ["Mod", "Alt", "Shift"]
        .into_iter()
        .filter(|modifier| modifiers.contains(modifier))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    normalized.push(key);
    Some(normalized.join("-"))
}

#[cfg(feature = "surreal-test-support")]
pub async fn workspace_settings_test_event_count(
    storage: &SurrealStorage,
    scope: &ResourceScope,
    workspace_id: &str,
) -> Result<usize, SurrealWorkspaceSettingsError> {
    #[derive(Debug, SurrealValue)]
    struct CountValue {
        count: i64,
    }

    let bindings = exact_scope_bindings(scope, workspace_id)?;
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<CountValue, _>(
                        "SELECT count() AS count FROM kernel_event_ledger \
                         WHERE owner_account_id = $owner_account_id \
                           AND actor_principal_id = $actor_principal_id \
                           AND authenticated_session_id = $authenticated_session_id \
                           AND access_space_id = $access_space_id \
                           AND workspace_id = $workspace_id \
                           AND aggregate_type = 'workspace_settings_state' \
                           AND event_type = 'KNOWLEDGE_WORKSPACE_SETTINGS_STATE_RECORDED' \
                           AND source_component = 'workspace_settings_state' GROUP ALL;",
                        bindings,
                    )
                    .await
            })
        })
        .await?;
    let count = rows.first().map_or(0, |value| value.count);
    usize::try_from(count).map_err(|_| SurrealWorkspaceSettingsError::CorruptAuthority)
}

/// Seeds one pre-schema row with no ownership attribution. This exists only in
/// the explicit Surreal test-support feature so fail-closed legacy behavior can
/// be proven without adding a production unscoped mutation surface.
#[cfg(feature = "surreal-test-support")]
pub async fn workspace_settings_test_seed_legacy_unscoped_row(
    storage: &SurrealStorage,
    workspace_id: &str,
    settings_state: Value,
) -> Result<(), SurrealStorageError> {
    #[derive(Debug, SurrealValue)]
    struct LegacyBindings {
        workspace_id: String,
        settings_state: Value,
    }

    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_bound(
                        "CREATE workspace_settings_authority:legacy CONTENT { \
                           workspace_id: $workspace_id, settings_state: $settings_state, \
                           generation: 1, updated_at: time::now(), storage_authority: 'embedded_surrealdb' \
                         };",
                        LegacyBindings {
                            workspace_id: workspace_id.to_owned(),
                            settings_state,
                        },
                    )
                    .await?;
                Ok(())
            })
        })
        .await
}
