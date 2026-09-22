use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::keyed_lock::KeyedLockRegistry;
use super::retry::Replay;
use super::{SurrealDataContext, SurrealDatabase, SurrealStorage, SurrealStorageError};
use crate::storage::fems_memory::{workspace_write_anchor, WorkspaceWriteAnchor};
use crate::storage::{NewWorkspace, StorageError, StorageResult, Workspace, WriteContext};

const WORKSPACES_TABLE: &str = "workspaces";

macro_rules! workspace_delete_body { () => { r#"UPSERT type::record('fems_workspace_write_anchors', $anchor.key) SET anchor_key = $anchor.key, workspace_key = $anchor.key, claim_nonce = $anchor.nonce, updated_at = time::now() RETURN NONE;
DELETE type::record('fems_workspace_write_anchors', $anchor.key) RETURN NONE;
DELETE atelier_intake_item_loom_projection WHERE workspace_id = $workspace;
DELETE loom_canvas_visual_edges WHERE workspace_id = $workspace;
DELETE loom_canvas_placements WHERE workspace_id = $workspace;
DELETE loom_edges WHERE workspace_id = $workspace;
DELETE loom_block_search_index WHERE workspace_id = $workspace;
DELETE loom_block_view_fr_outbox WHERE workspace_id = $workspace;
DELETE loom_blocks WHERE workspace_id = $workspace;
DELETE $workspace RETURN BEFORE;"# }; }
const WORKSPACE_DELETE_BODY: &str = workspace_delete_body!();
const WORKSPACE_DELETE_TRANSACTION: &str = concat!(
    "BEGIN TRANSACTION; ",
    workspace_delete_body!(),
    " COMMIT TRANSACTION;"
);

/// Check the physical cascade graph before the trusted delete, including incoming
/// references from another workspace. Unknown source families remain fail-closed.
#[cfg(test)]
fn workspace_cascade_guards() -> Result<String, SurrealStorageError> {
    use std::collections::{BTreeMap, BTreeSet};
    let edges = super::schema::workspace_cascade_edges()
        .map_err(|error| SurrealStorageError::TransactionWorker(error))?;
    let mut reachable = BTreeSet::from(["workspaces".to_owned()]);
    loop {
        let before = reachable.len();
        for (parent, child, _) in &edges {
            if reachable.contains(parent) {
                reachable.insert(child.clone());
            }
        }
        if reachable.len() == before {
            break;
        }
    }
    let mut scopes = BTreeMap::from([("workspaces".to_owned(), "id = $workspace".to_owned())]);
    for (parent, child, field) in &edges {
        if parent == "workspaces" {
            scopes.insert(child.clone(), format!("{field} = $workspace"));
        }
    }
    // Resolve only acyclic provenance paths. Self-cycles already have direct workspace scope.
    loop {
        let before = scopes.len();
        for (parent, child, field) in &edges {
            if reachable.contains(child) && !scopes.contains_key(child) {
                if let Some(parent_scope) = scopes.get(parent) {
                    scopes.insert(
                        child.clone(),
                        format!("{field} IN (SELECT VALUE id FROM {parent} WHERE {parent_scope})"),
                    );
                }
            }
        }
        if scopes.len() == before {
            break;
        }
    }
    let supported = BTreeSet::from([
        "workspaces",
        "knowledge_rich_documents",
        "knowledge_rich_document_versions",
        "knowledge_rich_document_drafts",
        "knowledge_document_embeds",
        "knowledge_editor_code_nodes",
        "knowledge_rich_document_title_anchors",
        "knowledge_idempotency_keys",
        "knowledge_document_backlinks",
        "knowledge_workbench_layout_states",
        "knowledge_workspace_settings_states",
        "knowledge_workspace_search_bookmark_states",
        "loom_blocks",
        "loom_edges",
        "loom_block_search_index",
        "loom_block_view_fr_outbox",
        "knowledge_sources",
        "knowledge_source_roots",
        "knowledge_index_runs",
        "knowledge_ingestion_root_policies",
        "knowledge_ingestion_policy_decisions",
        "knowledge_ingestion_receipts",
        "knowledge_ingestion_spans",
        "knowledge_ingestion_repair_queue",
        "knowledge_code_repair_queue",
        "knowledge_spans",
        "knowledge_entities",
        "knowledge_entity_spans",
        "knowledge_edges",
        "knowledge_edge_spans",
        "knowledge_code_files",
        "fems_memory_packs",
        "fems_memory_proposals",
        "fems_memory_items",
        "fems_memory_commit_reports",
        "fems_memory_commit_fr_outbox",
        "fems_memory_lifecycle_fr_outbox",
    ]);
    let mut sql = String::from("IF array::len(SELECT id FROM atelier_intake_item_loom_projection WHERE workspace_id = $workspace) > 0 { THROW 'HSK-403-PROTECTED-RESOURCE'; };\n");
    for table in &reachable {
        let scope = scopes.get(table).ok_or_else(|| {
            SurrealStorageError::TransactionWorker("unresolved cascade provenance".to_owned())
        })?;
        if !supported.contains(table.as_str()) {
            sql.push_str(&format!("IF array::len(SELECT id FROM {table} WHERE {scope}) > 0 {{ THROW 'HSK-403-PROTECTED-RESOURCE'; }};\n"));
        }
    }
    for (parent, child, field) in &edges {
        if !reachable.contains(parent) {
            continue;
        }
        let parent_scope = &scopes[parent];
        let child_scope = &scopes[child];
        sql.push_str(&format!("IF array::len(SELECT id FROM {child} WHERE {field} IN (SELECT VALUE id FROM {parent} WHERE {parent_scope}) AND ({child_scope}) != true) > 0 {{ THROW 'HSK-403-PROTECTED-RESOURCE'; }};\n"));
    }
    for (table, kind, external) in [
        (
            "knowledge_sources",
            "knowledge_source",
            "record::id($source.id)",
        ),
        (
            "knowledge_code_files",
            "knowledge_code_file",
            "record::id($source.id)",
        ),
        ("fems_memory_packs", "memory_pack", "$external"),
        ("fems_memory_proposals", "memory_proposal", "$external"),
        ("fems_memory_items", "memory_item", "$external"),
        (
            "fems_memory_commit_reports",
            "memory_commit_report",
            "$external",
        ),
    ] {
        sql.push_str(&format!("FOR $source IN (SELECT id FROM {table} WHERE workspace_id = $workspace) {{ IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = '{kind}' AND external_resource_id = {external} AND lifecycle_state = 'active') != 1 {{ THROW 'HSK-403-PROTECTED-RESOURCE'; }}; }};\n"));
    }
    Ok(sql)
}

/// MT-152 race-proof pause point between a read-decide step and its transaction; a no-op
/// outside `race_test_support::with_pause_after_decision`. Called with `first_attempt` so
/// only the FIRST attempt of a retried mutation parks on the two-party barrier: a retried
/// attempt (the loser of the engine-level collision the proof provokes) must not wait for a
/// second party that has already left.
async fn pause_after_decision(first_attempt: bool) {
    #[cfg(any(test, feature = "surreal-test-support"))]
    if first_attempt {
        super::keyed_lock::race_test_support::pause_after_decision().await;
    }
    #[cfg(not(any(test, feature = "surreal-test-support")))]
    let _ = first_attempt;
}

#[derive(SurrealValue)]
struct WorkspaceCreate {
    name: String,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
}

#[derive(Debug, SurrealValue)]
struct WorkspaceRecord {
    id: RecordId,
    name: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct WorkspaceDeleteBinding {
    workspace: RecordId,
    anchor: WorkspaceWriteAnchor,
}

impl TryFrom<WorkspaceRecord> for Workspace {
    type Error = SurrealStorageError;

    fn try_from(record: WorkspaceRecord) -> Result<Self, Self::Error> {
        if record.id.table.as_str() != WORKSPACES_TABLE {
            return Err(SurrealStorageError::InvalidWorkspaceRecord {
                reason: "record id belongs to a different table",
            });
        }
        let RecordIdKey::String(id) = record.id.key else {
            return Err(SurrealStorageError::InvalidWorkspaceRecord {
                reason: "record id is not a string key",
            });
        };

        Ok(Self {
            id,
            name: record.name,
            created_at: record.created_at.into_inner(),
            updated_at: record.updated_at.into_inner(),
        })
    }
}

impl SurrealDataContext<'_> {
    async fn create_workspace_record(
        &self,
        id: &str,
        content: WorkspaceCreate,
    ) -> Result<Workspace, SurrealStorageError> {
        let created: Option<WorkspaceRecord> = self
            .client
            .create((WORKSPACES_TABLE, id))
            .content(content)
            .await?;
        created
            .ok_or(SurrealStorageError::InvalidWorkspaceRecord {
                reason: "CREATE returned no record",
            })?
            .try_into()
    }

    async fn get_workspace_record(
        &self,
        id: &str,
    ) -> Result<Option<Workspace>, SurrealStorageError> {
        let record: Option<WorkspaceRecord> = self.client.select((WORKSPACES_TABLE, id)).await?;
        record.map(TryInto::try_into).transpose()
    }

    async fn list_workspace_records(&self) -> Result<Vec<Workspace>, SurrealStorageError> {
        let records: Vec<WorkspaceRecord> = self.client.select(WORKSPACES_TABLE).await?;
        // Named explicitly: with only `Vec<_>` the element type is not pinned
        // until the `Ok(workspaces)` at the end, so the `sort_by` closure below
        // has nothing to resolve `left`/`right` against and inference fails.
        let mut workspaces = records
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Workspace>, SurrealStorageError>>()?;
        workspaces.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(workspaces)
    }

    async fn delete_workspace_record(&self, id: &str) -> Result<bool, SurrealStorageError> {
        // A workspace owns Loom blocks through ON DELETE CASCADE, while placements and Atelier
        // projections deliberately REJECT deletion of a referenced block. Remove those dependants,
        // then the high-cardinality CASCADE dependants before the blocks. Letting each block discover
        // and cascade its edges and search projection makes large workspace teardown effectively
        // quadratic. The final workspace delete can then cascade the remaining workspace-owned rows
        // atomically.
        //
        // MT-152 I-152-4 (D-146-1 database-side guard): the transaction first UPSERTs and then
        // DELETEs this workspace's `fems_workspace_write_anchors` row, so that key is in its
        // write set whether or not the row existed. Every FEMS transaction that creates a row
        // referencing the workspace UPSERTs the same key, so an overlapping FEMS insert and this
        // delete collide at commit (the pinned engine validates written keys only); the loser's
        // bounded retry re-reads - the insert fails closed on `record::exists`, this delete's
        // cascade now sees the committed row. The removed FEMS process-global mutex used to order the two.
        let deleted = self
            .query_values_at::<WorkspaceRecord, _>(
                WORKSPACE_DELETE_TRANSACTION,
                WorkspaceDeleteBinding {
                    workspace: RecordId::new(WORKSPACES_TABLE, id.to_owned()),
                    anchor: workspace_write_anchor(id),
                },
                10,
            )
            .await?;
        Ok(!deleted.is_empty())
    }
}

impl SurrealStorage {
    /// Creates only a fresh server-generated source and its exact grants in one broker transaction.
    pub async fn create_account_workspace(
        &self,
        context: &super::local_accounts::LocalSessionContext,
        scope: &super::resource_authority::RecordUserScope,
        workspace: NewWorkspace,
    ) -> Result<Workspace, super::resource_authority::ResourceAuthorityError> {
        use super::resource_authority::ResourceAuthorityError;
        let started = std::time::Instant::now();
        let id = scope
            .workspace_id
            .clone()
            .ok_or(ResourceAuthorityError::InvalidInput(
                "workspace create scope missing",
            ))?;
        if scope.action != super::resource_authority::ResourceAction::Create
            || scope.capability_id != "fs.write"
            || scope.session_id != context.session_id
            || scope.grant_id.is_some()
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "workspace create scope mismatch",
            ));
        }
        let resource_id = scope.resource_id.clone();
        let metadata = self
            .inner
            .guard
            .validate_write(&WriteContext::human(Some(context.actor_id.clone())), &id)
            .await
            .map_err(|_| {
                tracing::warn!(
                    target: "handshake_core",
                    elapsed_ms = started.elapsed().as_millis(),
                    "workspace create write guard denied"
                );
                ResourceAuthorityError::Denied {
                    decision_id: Uuid::now_v7().to_string(),
                }
            })?;
        tracing::info!(
            target: "handshake_core",
            elapsed_ms = started.elapsed().as_millis(),
            "workspace create write guard completed"
        );
        let account = RecordId::new("local_accounts", context.identity.account_id.clone());
        let principal = RecordId::new("principals", context.identity.principal_id.clone());
        let space = RecordId::new("access_spaces", context.identity.access_space_id.clone());
        let session = RecordId::new("authenticated_sessions", context.session_id.clone());
        let actor = context.actor_id.clone();
        let created = self.with_record_user_scope(scope.clone(), self.with_data_operation(move |database| Box::pin(async move {
            let broker = database.client;
            let mut result = broker.query(r#"
BEGIN TRANSACTION;
LET $live = $auth;
CREATE $workspace SET created_in_session_id = $account_session, name = $name, last_actor_id = $actor, last_actor_kind = 'HUMAN', edit_event_id = $edit RETURN NONE;
CREATE $resource SET resource_kind = 'workspace', external_resource_id = $external, owner_account_id = $account,
    created_by_principal_id = $principal, created_in_session_id = $account_session, access_space_id = $space,
    creator_grant_id = $workspace_grant, parent_resource_id = NONE, schema_version = 1, lifecycle_state = 'active', policy_version = $live.policy_version,
    classification = 'account_private', storage_locator_hash = crypto::sha256('workspace:' + $external), created_at = time::now(), updated_at = time::now() RETURN NONE;
CREATE $fr SET resource_kind = 'flight_recorder', external_resource_id = $external, owner_account_id = $account,
    created_by_principal_id = $principal, created_in_session_id = $account_session, access_space_id = $space,
    creator_grant_id = $fr_grant, parent_resource_id = $resource, schema_version = 1, lifecycle_state = 'active', policy_version = $live.policy_version,
    classification = 'account_private', storage_locator_hash = crypto::sha256('flight_recorder:' + $external), created_at = time::now(), updated_at = time::now() RETURN NONE;
IF array::len((CREATE $workspace_grant SET account_id = $account, principal_id = $principal, access_space_id = $space,
    resource_id = $resource, actions = ['create','read','update','delete'], capability_ids = $capabilities,
    delegation_chain = $live.delegation_chain, status = 'active', grant_version = 1, policy_version = $live.policy_version,
    expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len((CREATE $fr_grant SET account_id = $account, principal_id = $principal, access_space_id = $space,
    resource_id = $fr, actions = ['create','read'], capability_ids = ['fr.read','fr.ingest.runtime_chat','fr.ingest.native_editor'],
    delegation_chain = $live.delegation_chain, status = 'active', grant_version = 1, policy_version = $live.policy_version,
    expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $account_session SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(SELECT VALUE id FROM $workspace) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
SELECT * FROM ONLY $workspace;
COMMIT TRANSACTION;
"#).bind(("account_session", session)).bind(("account", account)).bind(("principal", principal)).bind(("space", space))
                .bind(("workspace", RecordId::new("workspaces", id.clone())))
                .bind(("external", id)).bind(("name", workspace.name)).bind(("actor", actor))
                .bind(("edit", metadata.edit_event_id.to_string()))
                .bind(("resource", RecordId::new("protected_resources", resource_id)))
                .bind(("fr", RecordId::new("protected_resources", Uuid::now_v7().to_string())))
                .bind(("workspace_grant", RecordId::new("resource_grants", Uuid::now_v7().to_string())))
                .bind(("fr_grant", RecordId::new("resource_grants", Uuid::now_v7().to_string())))
                .bind(("capabilities", vec!["fs.read", "fs.write", "fr.read", "fr.ingest.runtime_chat", "fr.ingest.native_editor", "memory.read", "memory.propose"]))
                .await?;
            let mut errors = result.take_errors().into_iter().collect::<Vec<_>>();
            errors.sort_by_key(|(statement_index, _)| *statement_index);
            if !errors.is_empty() {
                let meaningful = errors.iter().position(|(_, error)| !error.to_string().to_ascii_lowercase().contains("query was not executed due to a failed transaction")).unwrap_or(0);
                let (statement_index, error) = errors.swap_remove(meaningful);
                #[cfg(test)] eprintln!("workspace-create transactionfailed statement_index={statement_index} error={error}");
                #[cfg(not(test))] let _ = statement_index;
                return Err(error.into());
            }
            let created: Option<WorkspaceRecord> = result.take(12)?;
            created.ok_or(SurrealStorageError::InvalidWorkspaceRecord { reason: "atomic workspace create returned no row" })?.try_into()
        }))).await;
        match &created {
            Ok(_) => tracing::info!(
                target: "handshake_core",
                elapsed_ms = started.elapsed().as_millis(),
                "workspace create record-user signin and transaction completed"
            ),
            Err(_) => tracing::warn!(
                target: "handshake_core",
                elapsed_ms = started.elapsed().as_millis(),
                "workspace create record-user signin or transaction failed"
            ),
        }
        created.map_err(Into::into)
    }

    pub(crate) async fn delete_account_workspace(
        &self,
        scope: &super::resource_authority::RecordUserScope,
        workspace_id: &str,
    ) -> Result<(), super::resource_authority::ResourceAuthorityError> {
        use super::resource_authority::{ResourceAction, ResourceAuthorityError};
        if scope.action != ResourceAction::Delete
            || scope.capability_id != "fs.write"
            || scope.workspace_id.as_deref() != Some(workspace_id)
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "workspace delete scope mismatch",
            ));
        }
        let context = self
            .authenticate_local_session(
                &scope.session_token,
                scope.channel_binding_hash.as_deref().ok_or(
                    ResourceAuthorityError::InvalidInput("channel binding missing"),
                )?,
            )
            .await?;
        if context.session_id != scope.session_id {
            return Err(ResourceAuthorityError::InvalidInput(
                "workspace delete session mismatch",
            ));
        }
        let account = RecordId::new("local_accounts", context.identity.account_id);
        let principal = RecordId::new("principals", context.identity.principal_id);
        let space = RecordId::new("access_spaces", context.identity.access_space_id);
        let session = RecordId::new("authenticated_sessions", context.session_id);
        let resource = RecordId::new("protected_resources", scope.resource_id.clone());
        let workspace = RecordId::new("workspaces", workspace_id.to_owned());
        let external = workspace_id.to_owned();
        let grant = RecordId::new(
            "resource_grants",
            scope
                .grant_id
                .clone()
                .ok_or(ResourceAuthorityError::InvalidInput(
                    "workspace delete grant missing",
                ))?,
        );
        let delete_body = WORKSPACE_DELETE_BODY.lines().skip(2).collect::<Vec<_>>().join("\n")
            .replace("DELETE $workspace RETURN BEFORE;", "IF array::len(DELETE $workspace RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };");
        self.with_record_user_scope(scope.clone(), self.with_data_operation(move |database| Box::pin(async move {
            let broker = database.client;
            let query = r#"
BEGIN TRANSACTION;
IF array::len(UPDATE $account_session SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $resource SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
LET $children = (SELECT id, resource_kind FROM protected_resources WHERE parent_resource_id = $resource OR parent_resource_id.parent_resource_id = $resource);
FOR $child IN $children {
    LET $child_action = IF $child.resource_kind = 'flight_recorder' { 'read' } ELSE { 'delete' };
    LET $child_capability = IF $child.resource_kind = 'flight_recorder' { 'fr.read' } ELSE { 'fs.write' };
    LET $child_grant = (SELECT VALUE id FROM resource_grants WHERE resource_id = $child.id
        AND principal_id = $principal AND account_id = $account AND access_space_id = $space
        AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now())
        AND actions CONTAINS $child_action AND capability_ids CONTAINS $child_capability
        AND delegation_chain = $auth.delegation_chain AND resource_id.policy_version <= policy_version
        AND policy_version <= $auth.policy_version LIMIT 1)[0];
    IF $child_grant = NONE { THROW 'HSK-403-PROTECTED-RESOURCE'; };
    IF array::len(UPDATE $child_grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
    LET $child_resource = $child.id;
    IF array::len(UPDATE $child_resource SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
};
__WORKSPACE_DELETE_BODY__
COMMIT TRANSACTION;
"#.replace("__WORKSPACE_DELETE_BODY__", &delete_body);
            let mut result = broker.query(query).bind(("account_session", session)).bind(("account", account)).bind(("principal", principal))
                .bind(("space", space)).bind(("resource", resource)).bind(("workspace", workspace)).bind(("external", external))
                .bind(("grant", grant)).await?;
            let mut errors = result.take_errors().into_iter().collect::<Vec<_>>();
            errors.sort_by_key(|(statement_index, _)| *statement_index);
            if !errors.is_empty() {
                let meaningful = errors
                    .iter()
                    .position(|(_, error)| {
                        !error
                            .to_string()
                            .to_ascii_lowercase()
                            .contains("query was not executed due to a failed transaction")
                    })
                    .unwrap_or(0);
                let (statement_index, error) = errors.swap_remove(meaningful);
                // Local operator diagnostic only; the API response stays the constant denial.
                tracing::warn!(
                    target: "handshake_core",
                    statement_index,
                    %error,
                    "workspace delete transaction failed"
                );
                return Err(error.into());
            }
            Ok(())
        }))).await.map_err(Into::into)
    }

    pub async fn list_account_workspaces(
        &self,
        token: &str,
        channel_hash: &str,
    ) -> Result<Vec<Workspace>, super::resource_authority::ResourceAuthorityError> {
        use super::resource_authority::{SigninParams, AUTHORITY_ACCESS_METHOD};
        use sha2::{Digest, Sha256};
        use surrealdb::opt::auth::Record;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
        let channel_binding_hash = Some(channel_hash.to_owned());
        self.with_lease(move |client| Box::pin(async move {
            let ordinary = client.clone();
            ordinary.use_ns(namespace.clone()).use_db(database.clone()).await?;
            ordinary.signin(Record { namespace, database, access: AUTHORITY_ACCESS_METHOD.to_owned(),
                params: SigninParams { token_hash, channel_binding_hash } }).await?;
            let mut result = ordinary.query("SELECT * FROM workspaces WHERE fn::mt109_has_workspace_access(record::id(id), 'read', 'fs.read') ORDER BY created_at, id;").await?.check()?;
            let rows: Vec<WorkspaceRecord> = result.take(0)?;
            rows.into_iter().map(TryInto::try_into).collect()
        })).await.map_err(Into::into)
    }

    pub async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace> {
        let id = Uuid::now_v7().to_string();
        let metadata = self
            .inner
            .guard
            .validate_write(ctx, &id)
            .await
            .map_err(StorageError::from)?;
        let content = WorkspaceCreate {
            name: workspace.name,
            last_job_id: metadata.job_id.map(|value| value.to_string()),
            last_workflow_id: metadata.workflow_id.map(|value| value.to_string()),
            last_actor_id: metadata.actor_id,
            edit_event_id: metadata.edit_event_id.to_string(),
            last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        };
        self.with_data_operation(move |database| {
            Box::pin(async move { database.create_workspace_record(&id, content).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>> {
        let id = id.to_owned();
        self.with_data_operation(move |database| {
            Box::pin(async move { database.get_workspace_record(&id).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>> {
        self.with_data_operation(|database| {
            Box::pin(async move { database.list_workspace_records().await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
        self.inner
            .guard
            .validate_write(ctx, id)
            .await
            .map_err(StorageError::from)?;
        // WP-KERNEL-012 MT-146 D-146-1. SurrealDB gives snapshot isolation with write-write
        // conflict detection only, so a proposal insert racing this delete writes disjoint keys
        // and nothing aborts: the insert's `record::exists` assert reads a stale snapshot that
        // still holds the workspace, while this delete's REFERENCE cascade is a snapshot-bound
        // scan that cannot see the insert's reference key. Both commit and the proposal outlives
        // its workspace. MT-146 ordered the two snapshots with the process-global FEMS mutation
        // lock; MT-152 I-152-4 replaced that with the `fems_workspace_write_anchors` key both
        // transactions write (see `delete_workspace_record`), so the race is decided at commit
        // for any two callers, in-process or not, and the loser converges through the bounded
        // MT-142 retry: this delete re-runs and cascades the committed insert, or the insert
        // re-runs and fails closed with NotFound. No keyed lock is taken (empty key set): a
        // workspace delete is a rare, heavy teardown and must not serialize ordinary writes.
        let replay_key = format!("workspace-delete:{id}");
        let mut attempts = 0u32;
        let deleted =
            SurrealDatabase::with_lock_registry(self.clone(), KeyedLockRegistry::disabled())
                .guarded_mutation(Vec::new(), Replay::idempotent(replay_key), None, || {
                    let owned_id = id.to_owned();
                    attempts += 1;
                    let first_attempt = attempts == 1;
                    async move {
                        pause_after_decision(first_attempt).await;
                        self.with_data_operation(move |database| {
                            Box::pin(
                                async move { database.delete_workspace_record(&owned_id).await },
                            )
                        })
                        .await
                        .map_err(map_storage_error)
                    }
                })
                .await?;
        if !deleted {
            return Err(StorageError::NotFound("workspace"));
        }
        Ok(())
    }
}

fn map_storage_error(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

#[cfg(test)]
mod cascade_guard_tests {
    #[test]
    fn workspace_cascade_graph_is_cycle_safe_and_checks_transitive_provenance() {
        let query = super::workspace_cascade_guards().expect("closed schema graph");
        assert!(query.contains("FROM loom_folders WHERE workspace_id = $workspace"));
        assert!(query.contains("FROM knowledge_rich_document_drafts WHERE rich_document_id IN"));
        assert!(query.contains("FROM knowledge_code_files WHERE workspace_id = $workspace"));
        assert!(query.contains("FROM calendar_sources WHERE workspace_id = $workspace"));
        assert!(query.contains("AND (workspace_id = $workspace) != true"));
        let privileged_checks = query.replace("THROW 'HSK-403-PROTECTED-RESOURCE'", "RETURN false");
        assert!(
            include_str!("schema.surql").contains(&privileged_checks),
            "schema permission closure must contain every current declarative graph guard"
        );
    }
}
