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
DELETE kernel_crdt_updates WHERE workspace_id = record::id($workspace);
DELETE atelier_intake_item_loom_projection WHERE workspace_id = $workspace;
DELETE loom_canvas_visual_edges WHERE workspace_id = $workspace;
DELETE loom_canvas_placements WHERE workspace_id = $workspace;
DELETE loom_canvas_boards WHERE workspace_id = $workspace;
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
        "assets",
        "media_asset_tiers",
        "calendar_activity_spans",
        "calendar_events",
        "calendar_sources",
        "canvases",
        "canvas_nodes",
        "canvas_edges",
        "knowledge_debug_breakpoints",
        "knowledge_quick_switcher_recents",
        "knowledge_wiki_projections",
        "loom_ai_suggestions",
        "loom_canvas_boards",
        "loom_canvas_placements",
        "loom_canvas_visual_edges",
        "loom_collections",
        "loom_collection_members",
        "loom_folders",
        "loom_folder_members",
        "loom_wiki_overlays",
        "stage_capture_artifacts",
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
        super::authority_mutation_submitted();
        let deleted = self
            .query_values_at::<WorkspaceRecord, _>(
                WORKSPACE_DELETE_TRANSACTION,
                WorkspaceDeleteBinding {
                    workspace: RecordId::new(WORKSPACES_TABLE, id.to_owned()),
                    anchor: workspace_write_anchor(id),
                },
                // The final `DELETE $workspace RETURN BEFORE` result. `take(index)` counts
                // `BEGIN TRANSACTION` as statement 0 (as fems_memory PROPOSAL_INSERT_TRANSACTION and
                // governance_check_store's index 3 do) and the body is one statement per line, so
                // the last body statement is index `lines().count()`. Derived so an added cascade
                // line (456dc63a, 7e73eb03) cannot shift it onto `DELETE loom_blocks` and turn every
                // trusted delete into NotFound("workspace").
                WORKSPACE_DELETE_BODY.lines().count(),
            )
            .await.map_err(|error| match error {
                SurrealStorageError::Database(error) =>
                    SurrealStorageError::Database(super::authority_mutation_error(error)),
                other => other,
            })?;
        Ok(!deleted.is_empty())
    }
}

#[derive(Clone, SurrealValue)]
struct MemorySurfaceBinding {
    kind: String,
    resource: RecordId,
    grant: RecordId,
}

impl SurrealStorage {
    pub async fn create_account_workspace(
        &self,
        context: &super::local_accounts::LocalSessionContext,
        scope: &super::resource_authority::RecordUserScope,
        workspace: NewWorkspace,
    ) -> Result<Workspace, super::resource_authority::ResourceAuthorityError> {
        self.begin_account_workspace_creation(context, scope, workspace).await?.workspace().await
    }

    pub(crate) async fn delete_account_workspace(
        &self,
        scope: &super::resource_authority::RecordUserScope,
        workspace_id: &str,
    ) -> Result<(), super::resource_authority::ResourceAuthorityError> {
        let scope = scope.clone();
        let workspace_id = workspace_id.to_owned();
        self.with_owned_authority_mutation(move |storage| async move {
            storage.delete_account_workspace_unfenced(&scope, &workspace_id).await
        }).await?
    }

    async fn delete_account_workspace_unfenced(
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
        // MT-154 AC-154-6(b): keep the fems_workspace_write_anchors race guard (the anchor UPSERT puts
        // the workspace key in this transaction's write set so a concurrent FEMS write conflicts)
        // instead of slicing it off by line count; each record-user write here is silent-deny checked.
        let anchor = workspace_write_anchor(workspace_id);
        let delete_body = WORKSPACE_DELETE_BODY
            .replacen(
                "UPSERT type::record('fems_workspace_write_anchors', $anchor.key)",
                "IF array::len(UPSERT type::record('fems_workspace_write_anchors', $anchor.key)",
                1,
            )
            .replacen(
                "updated_at = time::now() RETURN NONE;\nDELETE type::record('fems_workspace_write_anchors', $anchor.key) RETURN NONE;",
                "updated_at = time::now() RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };\nIF array::len(DELETE type::record('fems_workspace_write_anchors', $anchor.key) RETURN BEFORE) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };",
                1,
            )
            .replace("DELETE $workspace RETURN BEFORE;", "IF array::len(DELETE $workspace RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };");
        debug_assert!(
            delete_body
                .starts_with("IF array::len(UPSERT type::record('fems_workspace_write_anchors'")
                && delete_body.contains("RETURN BEFORE) != 1"),
            "workspace delete keeps the checked FEMS write-anchor race guard"
        );
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
LET $direct_children = (SELECT id, resource_kind FROM protected_resources WHERE parent_resource_id = $resource);
LET $direct_child_ids = $direct_children.id;
LET $children = array::concat($direct_children, (SELECT id, resource_kind FROM protected_resources WHERE parent_resource_id IN $direct_child_ids));
FOR $child IN $children {
    LET $child_action = IF $child.resource_kind = 'flight_recorder' { 'read' } ELSE { 'delete' };
    LET $child_capability = IF $child.resource_kind = 'flight_recorder' { 'fr.read' } ELSE { 'fs.write' };
    LET $child_id = $child.id;
    LET $child_grant = (SELECT VALUE id FROM (SELECT * FROM resource_grants WHERE account_id = $account
        AND principal_id = $principal AND access_space_id = $space AND resource_id = $child_id)
        WHERE status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now())
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
            #[cfg(test)]
            let probe_external = external.clone();
            super::authority_mutation_submitted();
            let mut result = broker.query(query).bind(("account_session", session)).bind(("account", account)).bind(("principal", principal))
                .bind(("space", space)).bind(("resource", resource)).bind(("workspace", workspace)).bind(("external", external))
                .bind(("grant", grant)).bind(("anchor", anchor)).await?;
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
                // MT-154 diagnostic (IV): test builds print the failing statement so a union run
                // records it (owned_workspace_delete_cascades 403) without a focused rerun.
                #[cfg(test)]
                eprintln!(
                    "HSK_WORKSPACE_DELETE_TRANSACTION_FAILED statement_index={statement_index} error={error}"
                );
                // MT-157 C1-FDELETE (test builds only): statement 10 is the checked anchor DELETE.
                // The cascade test installs this diagnostic copy; evaluate it on this exact
                // record-user connection inside a SELECT permission predicate after rollback.
                // This observes restored state, not the failed transaction's intermediate state.
                // Other tests may not install the copy,
                // in which case the probe error is diagnostic only; retain the original error.
                #[cfg(test)]
                if statement_index == 10 || std::env::var_os("HSK_C1_FDELETE_PROBE").is_some() {
                    let verdict = match broker
                        .query("SELECT * FROM type::record('c1_workspace_delete_probe', $external);")
                        .bind(("external", probe_external))
                        .await
                    {
                        Ok(probe) => match probe.check() {
                            Err(probe_error) => format!("predicate marker/error: {probe_error}"),
                            Ok(_) => "inconclusive: predicate emitted no marker".to_owned(),
                        },
                        Err(probe_error) => format!("probe error: {probe_error}"),
                    };
                    eprintln!("C1_FDELETE_ROUTE_PROBE phase=after-rollback statement_index={statement_index} verdict={verdict}");
                }
                return Err(super::authority_mutation_error(error).into());
            }
            Ok(())
        }))).await?;
        super::authority_mutation_submitted();
        self.resource_access_lifecycle().invalidate_workspace(workspace_id)
            .map_err(|_| ResourceAuthorityError::InvalidInput("lifecycle authority unavailable"))?;
        super::authority_mutation_settled();
        Ok(())
    }

    pub async fn list_account_workspaces(
        &self,
        token: &str,
        channel_hash: &str,
    ) -> Result<Vec<Workspace>, super::resource_authority::ResourceAuthorityError> {
        use super::resource_authority::{
            AuthorizationRequest, ResourceAction, ResourceAuthorityError, ResourceKind,
            SigninParams, AUTHORITY_ACCESS_METHOD,
        };
        use sha2::{Digest, Sha256};
        use surrealdb::opt::auth::Record;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
        let channel_binding_hash = Some(channel_hash.to_owned());
        let candidates: Vec<Workspace> = self
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
                    // Table RLS evaluates grants with permission-predicate visibility. A normal WHERE
                    // invocation instead sees protected_resources.external_resource_id as NONE.
                    let mut result = ordinary
                        .query("SELECT * FROM workspaces ORDER BY created_at, id;")
                        .await?
                        .check()?;
                    let rows: Vec<WorkspaceRecord> = result.take(0)?;
                    rows.into_iter().map(TryInto::try_into).collect()
                })
            })
            .await?;
        let mut visible = Vec::with_capacity(candidates.len());
        for workspace in candidates {
            // Table RLS also admits memory/fr readers; this route still requires fs.read.
            match self
                .authorize_protected_resource(AuthorizationRequest {
                    session_token: token.to_owned(),
                    channel_binding_hash: Some(channel_hash.to_owned()),
                    capability_id: "fs.read".to_owned(),
                    resource_kind: ResourceKind::Workspace,
                    external_resource_id: workspace.id.clone(),
                    action: ResourceAction::Read,
                })
                .await
            {
                Ok(_) => visible.push(workspace),
                Err(ResourceAuthorityError::Denied { .. }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(visible)
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
        let ctx = ctx.clone();
        let id = id.to_owned();
        self.with_owned_authority_mutation(move |storage| async move {
            storage.delete_workspace_unfenced(&ctx, &id).await
        }).await.map_err(map_storage_error)?
    }

    async fn delete_workspace_unfenced(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
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
        super::authority_mutation_submitted();
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
        super::authority_mutation_submitted();
        self.resource_access_lifecycle().invalidate_workspace(id)
            .map_err(|_| map_storage_error(SurrealStorageError::ResourceAuthorityUnavailable))?;
        super::authority_mutation_settled();
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
        assert!(query.contains("FROM loom_folders WHERE workspace_id IN"));
        assert!(query.contains("FROM knowledge_rich_document_drafts WHERE rich_document_id IN"));
        assert!(query.contains("FROM knowledge_code_files WHERE workspace_id = $workspace"));
        assert!(query.contains("FROM calendar_sources WHERE workspace_id IN"));
        assert!(query.contains("AND (workspace_id = $workspace) != true"));
        let privileged_checks = query.replace("THROW 'HSK-403-PROTECTED-RESOURCE'", "RETURN false");
        for guard in privileged_checks.lines() {
            assert!(
                include_str!("schema.surql")
                    .lines()
                    .any(|line| line == guard),
                "schema permission closure is missing declarative graph guard: {guard}"
            );
        }
    }
}



use std::sync::Arc;
use super::resource_authority::{CanonicalResourceLocator, RecordUserScope, ResourceAction, ResourceAuthorityError};

#[derive(Clone, SurrealValue)]
struct WorkspaceCreationBindings {
    account_session: RecordId,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    workspace: RecordId,
    external: String,
    name: String,
    actor: String,
    edit: String,
    resource: RecordId,
    fr: RecordId,
    workspace_grant: RecordId,
    fr_grant: RecordId,
    capabilities: Vec<String>,
    memory_surfaces: Vec<MemorySurfaceBinding>,
    operation_id: String,
    channel_hash: String,
    deadline: Datetime,
    first_owner_workspace_receipt_read: bool,
}
#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct FirstOwnerWorkspaceReceipt {
    setup_operation_id: String,
    account: RecordId,
    operator_principal: RecordId,
    space: RecordId,
    setup_receipt: RecordId,
    setup_policy_version: i64,
    created_at: Datetime,
    first_workspace_operation_id: String,
    first_workspace: RecordId,
    first_workspace_resource: RecordId,
    first_workspace_grant: RecordId,
    first_workspace_session: RecordId,
    first_workspace_channel_hash: String,
    first_workspace_name: String,
    first_workspace_deadline: Datetime,
    first_workspace_policy_version: i64,
    first_workspace_receipt_nonce: i64,
    first_workspace_original: surrealdb::types::Object,
}
struct FirstOwnerWorkspaceOriginal {
    storage: SurrealStorage,
    scope: RecordUserScope,
    context: super::local_accounts::LocalSessionContext,
    operation_id: Uuid,
    bindings: WorkspaceCreationBindings,
    deadline: std::time::Instant,
    cancellation: Arc<super::ContextOperationCancellation>,
    expected: std::sync::OnceLock<Option<FirstOwnerWorkspaceReceipt>>,
}
struct FirstOwnerWorkspaceProofSource {
    storage: SurrealStorage,
    context: super::local_accounts::LocalSessionContext,
    operation_id: Uuid,
    bindings: WorkspaceCreationBindings,
    deadline: Option<std::time::Instant>,
}
pub(super) struct FirstOwnerWorkspaceCreationWitness {
    original: Arc<FirstOwnerWorkspaceProofSource>,
    receipt: FirstOwnerWorkspaceReceipt,
    parent: CanonicalResourceLocator,
}
impl FirstOwnerWorkspaceCreationWitness {
    pub(super) fn parent_workspace(&self) -> &CanonicalResourceLocator { &self.parent }
    pub(super) fn parent_workspace_external_id(&self) -> &str { &self.original.bindings.external }
    pub(super) fn setup_operation_id(&self) -> Uuid {
        Uuid::parse_str(&self.receipt.setup_operation_id).expect("producer checked setup operation")
    }
    pub(super) fn operation_id(&self) -> Uuid { self.original.operation_id }
    pub(super) fn original_deadline(&self) -> Option<std::time::Instant> { self.original.deadline }
    pub(super) fn original_utc_deadline(&self) -> &Datetime { &self.original.bindings.deadline }
    pub(super) async fn require_current_in_transaction(
        &self, tx: &super::SurrealTransactionContext<'_>, storage: &SurrealStorage,
        session: &super::local_accounts::LocalSessionContext, channel_hash: &str,
    ) -> Result<(), ResourceAuthorityError> {
        let original = &self.original;
        if !Arc::ptr_eq(&storage.inner, &original.storage.inner)
            || session.identity.account_id != original.context.identity.account_id
            || session.identity.principal_id != original.context.identity.principal_id
            || session.identity.access_space_id != original.context.identity.access_space_id
            || session.session_id != original.context.session_id
            || channel_hash != original.bindings.channel_hash
        { return Err(workspace_creation_denied()); }
        let rows: Vec<FirstOwnerWorkspaceReceipt> = tx.query_values_at(
            FIRST_OWNER_WORKSPACE_READ_STRICT, original.bindings.clone(), 0,
        ).await?;
        if rows.as_slice() != [self.receipt.clone()] { return Err(workspace_creation_denied()); }
        Ok(())
    }
}
pub(super) struct WorkspaceCreationSuccess {
    workspace: Workspace,
    witness: Option<Arc<FirstOwnerWorkspaceCreationWitness>>,
}
pub(super) enum FirstOwnerWorkspaceFailure {
    Operation(ResourceAuthorityError),
}
impl From<SurrealStorageError> for FirstOwnerWorkspaceFailure {
    fn from(error: SurrealStorageError) -> Self { Self::Operation(error.into()) }
}
impl From<ResourceAuthorityError> for FirstOwnerWorkspaceFailure {
    fn from(error: ResourceAuthorityError) -> Self { Self::Operation(error) }
}
#[derive(Clone)]
pub(super) enum FirstOwnerWorkspaceObservation {
    Complete(Arc<Result<WorkspaceCreationSuccess, FirstOwnerWorkspaceFailure>>),
    Join(Arc<tokio::task::JoinError>),
}
enum FirstOwnerWorkspaceState {
    Starting,
    Running(tokio::task::JoinHandle<Result<WorkspaceCreationSuccess, FirstOwnerWorkspaceFailure>>),
    Observed(FirstOwnerWorkspaceObservation),
}
pub(super) struct FirstOwnerWorkspaceCreationOperation {
    original: Arc<FirstOwnerWorkspaceOriginal>,
    state: tokio::sync::Mutex<FirstOwnerWorkspaceState>,
}
fn workspace_creation_denied() -> ResourceAuthorityError {
    ResourceAuthorityError::InvalidInput("first owner workspace creation denied")
}
impl FirstOwnerWorkspaceCreationOperation {
    pub(super) async fn wait(&self) -> FirstOwnerWorkspaceObservation {
        let mut state = self.state.lock().await;
        if let FirstOwnerWorkspaceState::Running(handle) = &mut *state {
            let observed = match handle.await {
                Ok(value) => FirstOwnerWorkspaceObservation::Complete(Arc::new(value)),
                Err(error) => FirstOwnerWorkspaceObservation::Join(Arc::new(error)),
            };
            *state = FirstOwnerWorkspaceState::Observed(observed);
        }
        match &*state {
            FirstOwnerWorkspaceState::Observed(value) => value.clone(),
            _ => unreachable!("workspace worker published before observation"),
        }
    }
    async fn workspace(&self) -> Result<Workspace, ResourceAuthorityError> {
        match self.wait().await {
            FirstOwnerWorkspaceObservation::Complete(result) => match result.as_ref() {
                Ok(success) => Ok(success.workspace.clone()),
                Err(_) => Err(SurrealStorageError::TransactionSettlementPending {
                    operation_id: self.original.operation_id,
                }.into()),
            },
            _ => Err(SurrealStorageError::TransactionSettlementPending {
                operation_id: self.original.operation_id,
            }.into()),
        }
    }
    pub(super) async fn drain(&self) -> Result<(), ResourceAuthorityError> {
        self.workspace().await.map(|_| ())
    }
}
impl SurrealStorage {
    pub(super) async fn begin_account_workspace_creation(
        &self, context: &super::local_accounts::LocalSessionContext,
        scope: &RecordUserScope, workspace: NewWorkspace,
    ) -> Result<Arc<FirstOwnerWorkspaceCreationOperation>, ResourceAuthorityError> {
        let id = scope.workspace_id.clone().ok_or_else(workspace_creation_denied)?;
        let operation_id = Uuid::parse_str(&id).map_err(|_| workspace_creation_denied())?;
        if scope.action != ResourceAction::Create || scope.capability_id != "fs.write"
            || scope.session_id != context.session_id || scope.grant_id.is_some()
        { return Err(workspace_creation_denied()); }
        {
            let retained = self.inner.transaction_settlements.lock().map_err(|_| workspace_creation_denied())?;
            if let Some(entry) = retained.get(&operation_id) {
                return match entry {
                    Some(super::AuthenticatedTransactionRetention::FirstOwnerWorkspace(owner))
                        if owner.original.scope.session_token == scope.session_token
                        && owner.original.scope.channel_binding_hash == scope.channel_binding_hash
                        && owner.original.scope.resource_id == scope.resource_id
                        && owner.original.scope.session_id == scope.session_id
                        && owner.original.scope.workspace_id == scope.workspace_id
                        && owner.original.bindings.name == workspace.name
                        && owner.original.context.identity.account_id == context.identity.account_id
                        && owner.original.context.identity.principal_id == context.identity.principal_id
                        && owner.original.context.identity.access_space_id == context.identity.access_space_id
                        => Ok(owner.clone()),
                    _ => Err(workspace_creation_denied()),
                };
            }
        }
        let channel_hash = scope.channel_binding_hash.clone().ok_or_else(workspace_creation_denied)?;
        let deadline = std::time::Instant::now() + self.effective_statement_timeout();
        let utc_deadline = chrono::Utc::now() + chrono::Duration::from_std(
            deadline.saturating_duration_since(std::time::Instant::now()),
        ).map_err(|_| workspace_creation_denied())?;
        let metadata = self.inner.guard.validate_write(
            &WriteContext::human(Some(context.actor_id.clone())), &id,
        ).await.map_err(|_| workspace_creation_denied())?;
        let bindings = WorkspaceCreationBindings {
            account_session: RecordId::new("authenticated_sessions", context.session_id.clone()),
            account: RecordId::new("local_accounts", context.identity.account_id.clone()),
            principal: RecordId::new("principals", context.identity.principal_id.clone()),
            space: RecordId::new("access_spaces", context.identity.access_space_id.clone()),
            workspace: RecordId::new("workspaces", id.clone()), external: id,
            name: workspace.name, actor: context.actor_id.clone(), edit: metadata.edit_event_id.to_string(),
            resource: RecordId::new("protected_resources", scope.resource_id.clone()),
            fr: RecordId::new("protected_resources", Uuid::now_v7().to_string()),
            workspace_grant: RecordId::new("resource_grants", Uuid::now_v7().to_string()),
            fr_grant: RecordId::new("resource_grants", Uuid::now_v7().to_string()),
            capabilities: ["fs.read", "fs.write", "model_lane.navigation.read", "fr.read", "fr.ingest.runtime_chat", "fr.ingest.native_editor", "memory.read", "memory.propose", "memory.review", "memory.commit"].into_iter().map(str::to_owned).collect(),
            memory_surfaces: ["memory_pack", "memory_proposal", "memory_commit_report", "memory_item", "memory_item_count"].into_iter().map(|kind| MemorySurfaceBinding {
                kind: kind.to_owned(), resource: RecordId::new("protected_resources", Uuid::now_v7().to_string()),
                grant: RecordId::new("resource_grants", Uuid::now_v7().to_string()),
            }).collect(), operation_id: operation_id.to_string(), channel_hash, deadline: utc_deadline.into(), first_owner_workspace_receipt_read: false,
        };
        validate_workspace_original_shape(&bindings)?;
        let original = Arc::new(FirstOwnerWorkspaceOriginal {
            storage: self.clone(), scope: scope.clone(), context: context.clone(), operation_id,
            bindings, deadline, cancellation: super::ContextOperationCancellation::new(),
            expected: std::sync::OnceLock::new(),
        });
        let operation = Arc::new(FirstOwnerWorkspaceCreationOperation {
            original: original.clone(), state: tokio::sync::Mutex::new(FirstOwnerWorkspaceState::Starting),
        });
        {
            let admission = self.inner.member_login_admission.lock().map_err(|_| workspace_creation_denied())?;
            if !*admission { return Err(SurrealStorageError::Closed.into()); }
            self.reserve_transaction_settlement(operation_id, Some(
                super::AuthenticatedTransactionRetention::FirstOwnerWorkspace(operation.clone()),
            ))?;
            let mut state = operation.state.try_lock().map_err(|_| workspace_creation_denied())?;
            *state = FirstOwnerWorkspaceState::Running(tokio::spawn(async move {
                original.storage.clone().run_first_owner_workspace_creation(original).await
            }));
            drop(state);
            drop(admission);
        }
        Ok(operation)
    }
    async fn run_first_owner_workspace_creation(
        &self, original: Arc<FirstOwnerWorkspaceOriginal>,
    ) -> Result<WorkspaceCreationSuccess, FirstOwnerWorkspaceFailure> {
        let write = original.bindings.clone();
        let captured = original.clone();
        self.transaction_worker::<(), FirstOwnerWorkspaceFailure, _>(
            super::TransactionInvocation::Authenticated {
                scope: original.scope.clone(), deadline: original.deadline, operation_id: original.operation_id,
            }, Some(original.operation_id), Some(original.cancellation.clone()),
            super::TransactionSettlementMode::Mutation,
            move |tx| Box::pin(async move {
                let accounts: Vec<WorkspaceApplicability> = tx.query_values_at(
                    FIRST_OWNER_WORKSPACE_APPLICABILITY, write.clone(), 0,
                ).await?;
                let [account] = accounts.as_slice() else { return Err(workspace_creation_denied().into()); };
                if account.id != write.account { return Err(workspace_creation_denied().into()); }
                let mut first_claim = false;
                if let Some(origin) = &account.first_owner_administration_origin {
                    if *origin != RecordId::new("first_owner_administration_origins", "primary") {
                        return Err(workspace_creation_denied().into());
                    }
                    let origins: Vec<FirstOwnerWorkspaceOriginState> = tx.query_values_at(
                        FIRST_OWNER_WORKSPACE_ORIGIN_STATE, write.clone(), 0,
                    ).await?;
                    let [origin] = origins.as_slice() else { return Err(workspace_creation_denied().into()); };
                    if origin.account != write.account || origin.operator_principal != write.principal
                        || origin.space != write.space || origin.purpose != "prospective_first_owner_administration"
                        || origin.version != 1 { return Err(workspace_creation_denied().into()); }
                    match origin.first_workspace_state.as_str() {
                        "unclaimed" if origin.first_workspace_original.is_none() => first_claim = true,
                        "created" => {
                            let stored = decode_workspace_original(origin.first_workspace_original.as_ref()
                                .ok_or_else(workspace_creation_denied)?)?;
                            let prior: Vec<FirstOwnerWorkspaceReceipt> = tx.query_values_at(
                                FIRST_OWNER_WORKSPACE_READ, stored.receipt_read(), 0,
                            ).await?;
                            if prior.len() != 1 { return Err(workspace_creation_denied().into()); }
                        }
                        _ => return Err(workspace_creation_denied().into()),
                    }
                }
                let response = tx.transaction.query(FIRST_OWNER_WORKSPACE_CREATE)
                    .bind(super::original_operation_bindings(write.clone().into_value()))
                    .bind(("first_claim", first_claim)).await.map_err(SurrealStorageError::from)?;
                response.check().map_err(SurrealStorageError::from)?;
                let expected = if first_claim {
                    let receipts: Vec<FirstOwnerWorkspaceReceipt> = tx.query_values_at(
                        FIRST_OWNER_WORKSPACE_READ, write.receipt_read(), 0,
                    ).await?;
                    let [receipt] = receipts.as_slice() else { return Err(workspace_creation_denied().into()); };
                    if receipt.first_workspace_original.clone().into_value() != write.clone().into_value() {
                        return Err(workspace_creation_denied().into());
                    }
                    Some(receipt.clone())
                } else { None };
                captured.expected.set(expected).map_err(|_| workspace_creation_denied())?;
                Ok(())
            }),
        ).await?;
        let read = original.bindings.clone();
        let expected_receipt = original.expected.get().ok_or_else(workspace_creation_denied)?.is_some();
        let (workspace, receipt) = self.with_record_user_scope(original.scope.clone(),
            self.with_data_operation(move |db| Box::pin(async move {
                let rows: Vec<WorkspaceRecord> = db.query_values_at(
                    "SELECT * FROM $workspace;", read.clone(), 0,
                ).await?;
                let [workspace] = rows.as_slice() else {
                    return Err(SurrealStorageError::InvalidWorkspaceRecord { reason: "workspace canonical read missing" });
                };
                let workspace = Workspace {
                    id: read.external.clone(), name: workspace.name.clone(),
                    created_at: workspace.created_at.clone().into_inner(), updated_at: workspace.updated_at.clone().into_inner(),
                };
                let receipt: Vec<FirstOwnerWorkspaceReceipt> = if expected_receipt {
                    db.query_values_at(FIRST_OWNER_WORKSPACE_READ, read.receipt_read(), 0).await?
                } else { Vec::new() };
                Ok((workspace, receipt))
            })),
        ).await?;
        if receipt.as_slice() != original.expected.get().and_then(Option::as_ref).map(std::slice::from_ref).unwrap_or(&[]) { return Err(workspace_creation_denied().into()); }
        let witness = if let [receipt] = receipt.as_slice() {
            Uuid::parse_str(&receipt.setup_operation_id).map_err(|_| workspace_creation_denied())?;
            Some(Arc::new(FirstOwnerWorkspaceCreationWitness {
                original: Arc::new(FirstOwnerWorkspaceProofSource {
                    storage: original.storage.clone(), context: original.context.clone(),
                    operation_id: original.operation_id, bindings: original.bindings.clone(),
                    deadline: Some(original.deadline),
                }), receipt: receipt.clone(),
                parent: CanonicalResourceLocator {
                    id: original.bindings.resource.clone(), external_resource_id: original.bindings.external.clone(),
                    owner_account_id: original.bindings.account.clone(), access_space_id: original.bindings.space.clone(),
                    parent_resource_id: None, lifecycle_state: "active".to_owned(),
                    policy_version: receipt.first_workspace_policy_version,
                },
            }))
        } else if receipt.is_empty() { None } else { return Err(workspace_creation_denied().into()); };
        Ok(WorkspaceCreationSuccess { workspace, witness })
    }
    pub(super) async fn first_owner_workspace_creation_witness(
        &self, operation_id: Uuid,
    ) -> Result<Arc<FirstOwnerWorkspaceCreationWitness>, ResourceAuthorityError> {
        let operation = {
            let retained = self.inner.transaction_settlements.lock().map_err(|_| workspace_creation_denied())?;
            match retained.get(&operation_id) {
                Some(Some(super::AuthenticatedTransactionRetention::FirstOwnerWorkspace(owner))) => owner.clone(),
                Some(Some(super::AuthenticatedTransactionRetention::FirstOwnerWorkspaceRecovery(witness))) => return Ok(witness.clone()),
                _ => return Err(workspace_creation_denied()),
            }
        };
        match operation.wait().await {
            FirstOwnerWorkspaceObservation::Complete(result) => match result.as_ref() {
                Ok(success) => success.witness.clone().ok_or_else(workspace_creation_denied),
                Err(_) => Err(workspace_creation_denied()),
            },
            _ => Err(workspace_creation_denied()),
        }
    }
    pub(crate) async fn drain_first_owner_workspace_operations(&self) -> Result<(), ResourceAuthorityError> {
        self.close_member_login_admission()?;
        let owners: Vec<_> = {
            let retained = self.inner.transaction_settlements.lock().map_err(|_| workspace_creation_denied())?;
            retained.values().filter_map(|entry| match entry {
                Some(super::AuthenticatedTransactionRetention::FirstOwnerWorkspace(owner)) => Some(owner.clone()),
                _ => None,
            }).collect()
        };
        let mut first_error = None;
        for owner in owners {
            if let Err(error) = owner.drain().await { if first_error.is_none() { first_error = Some(error); } }
        }
        first_error.map_or(Ok(()), Err)
    }
}

const FIRST_OWNER_WORKSPACE_CREATE: &str = r#"
IF $first_claim {
    IF array::len(SELECT VALUE id FROM protected_resources WHERE resource_kind = 'workspace'
        AND owner_account_id = $account LIMIT 1) != 0 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
    IF array::len(UPDATE first_owner_administration_origins:primary SET
        first_workspace_state = 'created', first_workspace_operation_id = $operation_id,
        first_workspace = $workspace, first_workspace_resource = $resource,
        first_workspace_grant = $workspace_grant, first_workspace_session = $account_session,
        first_workspace_channel_hash = $channel_hash, first_workspace_name = $name,
        first_workspace_deadline = $deadline, first_workspace_policy_version = $auth.policy_version,
        first_workspace_receipt_nonce = 1,
        first_workspace_original = { account_session: $account_session, account: $account,
          principal: $principal, space: $space, workspace: $workspace, external: $external,
          name: $name, actor: $actor, edit: $edit, resource: $resource, fr: $fr,
          workspace_grant: $workspace_grant, fr_grant: $fr_grant, capabilities: $capabilities,
          memory_surfaces: $memory_surfaces, operation_id: $operation_id, channel_hash: $channel_hash,
          deadline: $deadline, first_owner_workspace_receipt_read: false }
        WHERE first_workspace_state = 'unclaimed' AND first_workspace_receipt_nonce = 0
        AND first_workspace_original = NONE AND account = $account
        AND operator_principal = $principal AND space = $space
        RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
};
LET $live = $auth;
LET $granted = IF $live.delegated_capabilities CONTAINS '*' AND $live.principal_id.delegated_capabilities CONTAINS '*' { $capabilities } ELSE { array::filter($capabilities, |$capability| ($live.delegated_capabilities CONTAINS '*' OR $live.delegated_capabilities CONTAINS $capability) AND ($live.principal_id.delegated_capabilities CONTAINS '*' OR $live.principal_id.delegated_capabilities CONTAINS $capability)) };
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
    resource_id = $resource, actions = ['create','read','update','delete'], capability_ids = $granted,
    delegation_chain = $live.delegation_chain, status = 'active', grant_version = 1, policy_version = $live.policy_version,
    expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len((CREATE $fr_grant SET account_id = $account, principal_id = $principal, access_space_id = $space,
    resource_id = $fr, actions = ['create','read'], capability_ids = ['fr.read','fr.ingest.runtime_chat','fr.ingest.native_editor'],
    delegation_chain = $live.delegation_chain, status = 'active', grant_version = 1, policy_version = $live.policy_version,
    expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
FOR $surface IN $memory_surfaces {
    LET $memory_resource = $surface.resource;
    LET $memory_grant = $surface.grant;
    CREATE $memory_resource SET resource_kind = $surface.kind, external_resource_id = $external, owner_account_id = $account,
        created_by_principal_id = $principal, created_in_session_id = $account_session, access_space_id = $space,
        creator_grant_id = $memory_grant, parent_resource_id = $resource, schema_version = 1, lifecycle_state = 'active', policy_version = $live.policy_version,
        classification = 'account_private', storage_locator_hash = crypto::sha256($surface.kind + ':' + $external), created_at = time::now(), updated_at = time::now() RETURN NONE;
    IF array::len((CREATE $memory_grant SET account_id = $account, principal_id = $principal, access_space_id = $space,
        resource_id = $memory_resource, actions = ['create','read','update','delete'],
        capability_ids = array::filter(['memory.read','memory.propose','memory.review','memory.commit','fs.write'], |$capability| $granted CONTAINS $capability),
        delegation_chain = $live.delegation_chain, status = 'active', grant_version = 1, policy_version = $live.policy_version,
        expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
};
IF array::len(UPDATE $account_session SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(UPDATE $space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
IF array::len(SELECT VALUE id FROM $workspace) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };


"#;
const FIRST_OWNER_WORKSPACE_READ: &str = r#"
SELECT setup_operation_id, account, operator_principal, space, setup_receipt, setup_policy_version, created_at,
 first_workspace_operation_id, first_workspace, first_workspace_resource, first_workspace_grant,
 first_workspace_session, first_workspace_channel_hash, first_workspace_name, first_workspace_deadline,
 first_workspace_policy_version, first_workspace_receipt_nonce, first_workspace_original
FROM first_owner_administration_origins:primary
WHERE first_workspace_operation_id = $operation_id AND first_workspace = $workspace
 AND first_workspace_resource = $resource AND first_workspace_grant = $workspace_grant
 AND first_workspace_session = $account_session AND first_workspace_channel_hash = $channel_hash
 AND first_workspace_name = $name AND first_workspace_deadline = $deadline;
"#;

const FIRST_OWNER_WORKSPACE_READ_STRICT: &str = r#"
SELECT setup_operation_id, account, operator_principal, space, setup_receipt, setup_policy_version, created_at,
 first_workspace_operation_id, first_workspace, first_workspace_resource, first_workspace_grant,
 first_workspace_session, first_workspace_channel_hash, first_workspace_name, first_workspace_deadline,
 first_workspace_policy_version, first_workspace_receipt_nonce, first_workspace_original
FROM first_owner_administration_origins:primary
WHERE fn::wp012_first_owner_workspace_receipt($this, $operation_id, $workspace, $resource,
 $workspace_grant, $account_session, $channel_hash, $name, $deadline);
"#;

#[derive(SurrealValue)]
struct WorkspaceApplicability {
    id: RecordId,
    first_owner_administration_origin: Option<RecordId>,
}
#[derive(SurrealValue)]
struct FirstOwnerWorkspaceOriginState {
    account: RecordId,
    operator_principal: RecordId,
    space: RecordId,
    purpose: String,
    version: i64,
    first_workspace_state: String,
    first_workspace_original: Option<surrealdb::types::Object>,
}
impl WorkspaceCreationBindings {
    fn receipt_read(&self) -> Self {
        let mut read = self.clone();
        read.first_owner_workspace_receipt_read = true;
        read
    }
}
fn decode_workspace_original(value: &surrealdb::types::Object) -> Result<WorkspaceCreationBindings, ResourceAuthorityError> {
    let raw = value.clone().into_value();
    let original = WorkspaceCreationBindings::from_value(raw.clone()).map_err(|_| workspace_creation_denied())?;
    if original.clone().into_value() != raw || original.first_owner_workspace_receipt_read {
        return Err(workspace_creation_denied());
    }
    Ok(original)
}
const FIRST_OWNER_WORKSPACE_APPLICABILITY: &str = r#"
SELECT id, first_owner_administration_origin FROM local_accounts
 WHERE id = $account AND id = $auth.account_id AND $auth.id = $account_session
 AND $auth.principal_id = $principal AND $auth.access_space_id = $space
 AND fn::mt109_live_session();
"#;
const FIRST_OWNER_WORKSPACE_ORIGIN_STATE: &str = r#"
SELECT account, operator_principal, space, purpose, version, first_workspace_state, first_workspace_original
 FROM first_owner_administration_origins:primary;
"#;


#[derive(SurrealValue)]
struct FirstOwnerWorkspaceRecoveryRead {
    operation_id: String,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    session: RecordId,
    channel_hash: String,
}
impl SurrealStorage {
    pub(super) async fn recover_first_owner_workspace_creation_witness(
        &self, transaction: &super::SurrealTransactionContext<'_>,
        session: &super::local_accounts::LocalSessionContext, channel_hash: &str, operation_id: Uuid,
    ) -> Result<Arc<FirstOwnerWorkspaceCreationWitness>, ResourceAuthorityError> {
        // A failed, Unknown, live or already-recovered owner must use the original getter.
        // Only a truly absent retained original permits reading the durable prospective receipt.
        {
            let retained = self.inner.transaction_settlements.lock().map_err(|_| workspace_creation_denied())?;
            if retained.contains_key(&operation_id) { return Err(workspace_creation_denied()); }
        }
        let rows: Vec<FirstOwnerWorkspaceReceipt> = transaction.query_values_at(
            FIRST_OWNER_WORKSPACE_RECOVERY_READ, FirstOwnerWorkspaceRecoveryRead {
                operation_id: operation_id.to_string(),
                account: RecordId::new("local_accounts", session.identity.account_id.clone()),
                principal: RecordId::new("principals", session.identity.principal_id.clone()),
                space: RecordId::new("access_spaces", session.identity.access_space_id.clone()),
                session: RecordId::new("authenticated_sessions", session.session_id.clone()),
                channel_hash: channel_hash.to_owned(),
            }, 0,
        ).await?;
        let [receipt] = rows.as_slice() else { return Err(workspace_creation_denied()); };
        let bindings = decode_workspace_original(&receipt.first_workspace_original)?;
        validate_workspace_original_shape(&bindings)?;
        Uuid::parse_str(&receipt.setup_operation_id).map_err(|_| workspace_creation_denied())?;
        if bindings.operation_id != operation_id.to_string()
            || bindings.account != RecordId::new("local_accounts", session.identity.account_id.clone())
            || bindings.principal != RecordId::new("principals", session.identity.principal_id.clone())
            || bindings.space != RecordId::new("access_spaces", session.identity.access_space_id.clone())
            || bindings.account_session != RecordId::new("authenticated_sessions", session.session_id.clone())
            || bindings.channel_hash != channel_hash || bindings.actor != session.actor_id
        { return Err(workspace_creation_denied()); }
        let canonical: Vec<FirstOwnerWorkspaceReceipt> = transaction.query_values_at(
            FIRST_OWNER_WORKSPACE_READ_STRICT, bindings.clone(), 0,
        ).await?;
        if canonical.as_slice() != std::slice::from_ref(receipt) { return Err(workspace_creation_denied()); }
        let witness = Arc::new(FirstOwnerWorkspaceCreationWitness {
            original: Arc::new(FirstOwnerWorkspaceProofSource {
                storage: self.clone(), context: session.clone(), operation_id,
                bindings: bindings.clone(), deadline: None,
            }), receipt: receipt.clone(),
            parent: CanonicalResourceLocator {
                id: bindings.resource, external_resource_id: bindings.external,
                owner_account_id: bindings.account, access_space_id: bindings.space,
                parent_resource_id: None, lifecycle_state: "active".to_owned(),
                policy_version: receipt.first_workspace_policy_version,
            },
        });
        // Atomic publication also rejects an original owner that appeared during canonical read.
        // No SDK mutation, UUID, execution deadline, password or session token is produced here.
        let admission = self.inner.member_login_admission.lock().map_err(|_| workspace_creation_denied())?;
        if !*admission { return Err(SurrealStorageError::Closed.into()); }
        self.reserve_transaction_settlement(operation_id, Some(
            super::AuthenticatedTransactionRetention::FirstOwnerWorkspaceRecovery(witness.clone()),
        ))?;
        drop(admission);
        Ok(witness)
    }
}
fn validate_workspace_original_shape(original: &WorkspaceCreationBindings) -> Result<(), ResourceAuthorityError> {
    let expected_capabilities: Vec<String> = ["fs.read", "fs.write", "model_lane.navigation.read", "fr.read",
        "fr.ingest.runtime_chat", "fr.ingest.native_editor", "memory.read", "memory.propose", "memory.review", "memory.commit"]
        .into_iter().map(str::to_owned).collect();
    if original.capabilities != expected_capabilities || original.first_owner_workspace_receipt_read
        || original.workspace != RecordId::new("workspaces", original.external.clone())
        || original.operation_id != original.external
        || Uuid::parse_str(&original.operation_id).is_err() || Uuid::parse_str(&original.edit).is_err()
        || original.channel_hash.is_empty() || original.name.is_empty()
        || original.account.table.as_str() != "local_accounts" || original.principal.table.as_str() != "principals"
        || original.space.table.as_str() != "access_spaces" || original.account_session.table.as_str() != "authenticated_sessions"
        || original.resource.table.as_str() != "protected_resources" || original.fr.table.as_str() != "protected_resources"
        || original.workspace_grant.table.as_str() != "resource_grants" || original.fr_grant.table.as_str() != "resource_grants"
        || original.memory_surfaces.len() != 5
    { return Err(workspace_creation_denied()); }
    let mut resources = vec![original.resource.clone(), original.fr.clone()];
    let mut grants = vec![original.workspace_grant.clone(), original.fr_grant.clone()];
    for (surface, expected) in original.memory_surfaces.iter().zip(
        ["memory_pack", "memory_proposal", "memory_commit_report", "memory_item", "memory_item_count"],
    ) {
        if surface.kind != expected || surface.resource.table.as_str() != "protected_resources"
            || surface.grant.table.as_str() != "resource_grants"
            || resources.contains(&surface.resource) || grants.contains(&surface.grant)
        { return Err(workspace_creation_denied()); }
        resources.push(surface.resource.clone()); grants.push(surface.grant.clone());
    }
    if original.resource == original.fr || original.workspace_grant == original.fr_grant {
        return Err(workspace_creation_denied());
    }
    Ok(())
}
const FIRST_OWNER_WORKSPACE_RECOVERY_READ: &str = r#"
SELECT setup_operation_id, account, operator_principal, space, setup_receipt, setup_policy_version, created_at,
 first_workspace_operation_id, first_workspace, first_workspace_resource, first_workspace_grant,
 first_workspace_session, first_workspace_channel_hash, first_workspace_name, first_workspace_deadline,
 first_workspace_policy_version, first_workspace_receipt_nonce, first_workspace_original
FROM first_owner_administration_origins:primary
 WHERE purpose = 'prospective_first_owner_administration' AND version = 1
 AND account = $account AND operator_principal = $principal AND space = $space
 AND account.first_owner_administration_origin = first_owner_administration_origins:primary
 AND first_workspace_state = 'created' AND first_workspace_receipt_nonce = 1
 AND first_workspace_operation_id = $operation_id AND first_workspace_original != NONE
 AND first_workspace_session = $session AND first_workspace_channel_hash = $channel_hash;
"#;
