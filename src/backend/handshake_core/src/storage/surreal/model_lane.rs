use std::collections::{BTreeMap, BTreeSet};

use super::{SurrealStorage, SurrealStorageError};
use crate::kernel::context_bundle::canonical_json_bytes;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::SurrealValue;

const SCHEMA: &str = include_str!("model_lane_schema.surql");
const SCHEMA_STATE: &str = "\
DEFINE TABLE IF NOT EXISTS model_lane_schema_state SCHEMAFULL;\
DEFINE FIELD IF NOT EXISTS schema_version ON model_lane_schema_state TYPE string;\
DEFINE FIELD IF NOT EXISTS schema_revision ON model_lane_schema_state TYPE int;\
DEFINE FIELD IF NOT EXISTS apply_state ON model_lane_schema_state TYPE string;";
const SCHEMA_STATE_ID: &str = "model_lane_schema_state:primary";
const SCHEMA_VERSION: &str = "mt003-model-lane-authority-v7";
const SCHEMA_REVISION: i64 = 7;
const PREVIOUS_SCHEMA_VERSION: &str = "mt003-model-lane-authority-v6";
const PREVIOUS_SCHEMA_REVISION: i64 = 6;
const LEGACY_SCHEMA_VERSION: &str = "mt003-model-lane-authority-v5";
const LEGACY_SCHEMA_REVISION: i64 = 5;
const OLDER_SCHEMA_VERSION: &str = "mt003-model-lane-authority-v4";
const OLDER_SCHEMA_REVISION: i64 = 4;
const OLDEST_SCHEMA_VERSION: &str = "mt003-model-lane-authority-v3";
const OLDEST_SCHEMA_REVISION: i64 = 3;
const PAIR_IMMUTABLE_QUERY: &str = "\
BEGIN TRANSACTION;\
LET $first_existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $first_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
LET $second_existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $second_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
IF array::len($first_existing) = 1 AND array::len($second_existing) = 1 {\
  RETURN array::concat($first_existing, $second_existing);\
} ELSE {\
  IF array::len($first_existing) > 0 OR array::len($second_existing) > 0 {\
    THROW 'model-lane immutable pair is partially present';\
  } ELSE {\
  LET $first_ledger = CREATE type::record('kernel_event_ledger', $first_event_id) CONTENT { event_id: $first_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $first_run_id, session_run_id: $first_run_id, aggregate_type: $first_record_kind, aggregate_id: $first_aggregate_id, idempotency_key: $first_event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $first_event_payload_hash, source_component: 'model_lane', payload: { record_kind: $first_record_kind, run_id: $first_run_id, event_stream_version: 1, event_payload_json: $first_event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
  LET $first_seq = $first_ledger[0].event_sequence;\
  LET $first_row = CREATE type::record('model_lane_authority', $first_record_id) CONTENT { record_kind: $first_record_kind, aggregate_id: $first_aggregate_id, run_id: $first_run_id, idempotency_key: $first_idempotency_key, record_json: $first_record_json, search_terms: $first_search_terms, event_id: $first_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $first_event_id), event_seq: $first_seq, event_stream_version: 1, transaction_seq: $first_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
  LET $second_ledger = CREATE type::record('kernel_event_ledger', $second_event_id) CONTENT { event_id: $second_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $second_run_id, session_run_id: $second_run_id, aggregate_type: $second_record_kind, aggregate_id: $second_aggregate_id, idempotency_key: $second_event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $first_event_id, correlation_id: NONE, payload_hash: $second_event_payload_hash, source_component: 'model_lane', payload: { record_kind: $second_record_kind, run_id: $second_run_id, event_stream_version: 1, event_payload_json: $second_event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
  LET $second_seq = $second_ledger[0].event_sequence;\
  LET $second_row = CREATE type::record('model_lane_authority', $second_record_id) CONTENT { record_kind: $second_record_kind, aggregate_id: $second_aggregate_id, run_id: $second_run_id, idempotency_key: $second_idempotency_key, record_json: $second_record_json, search_terms: $second_search_terms, event_id: $second_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $second_event_id), event_seq: $second_seq, event_stream_version: 1, transaction_seq: $second_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
    RETURN array::concat($first_row, $second_row);\
  };\
};\
COMMIT TRANSACTION;";
const GUARDED_MESSAGE_QUERY: &str = "\
BEGIN TRANSACTION;\
LET $result = (\
  LET $existing_message = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $message_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
  IF array::len($existing_message) = 1 {\
    IF $has_payload_binding {\
      LET $existing_binding = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $binding_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
      IF array::len($existing_binding) != 1 { THROW 'model-lane message payload binding is partially present'; };\
      array::concat($existing_message, $existing_binding)\
    } ELSE { $existing_message };\
  } ELSE {\
    LET $source = (SELECT VALUE id FROM type::record('model_lane_authority', $source_lane_record_id) WHERE record_kind = 'lane' AND record_json = $source_lane_record_json AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
    LET $session_owners = (SELECT VALUE id FROM model_lane_authority WHERE record_kind = 'lane' AND ($source_session_term IN search_terms OR $source_model_session_term IN search_terms) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
    LET $promotion = IF $has_promotion { (SELECT VALUE id FROM type::record('model_lane_authority', $promotion_record_id) WHERE record_kind = 'promotion_decision' AND record_json = $promotion_record_json AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id) } ELSE { [true] };\
    LET $update = IF $has_crdt { (SELECT VALUE id FROM kernel_crdt_updates WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id AND update_bytes_ref = $crdt_update_ref AND update_id = $crdt_update_id AND update_sha256 = $crdt_update_sha256 AND state_vector_after = $crdt_state_vector AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.event_type = 'KNOWLEDGE_CRDT_UPDATE_RECORDED' AND event_ledger_event_id.aggregate_type = 'knowledge_crdt_document' AND event_ledger_event_id.aggregate_id = crdt_document_id AND event_ledger_event_id.actor_id = actor_id AND event_ledger_event_id.correlation_id = trace_id AND event_ledger_event_id.payload.update_id = update_id AND event_ledger_event_id.payload.update_sha256 = update_sha256 AND event_ledger_event_id.payload.state_vector_after = state_vector_after LIMIT 2) } ELSE { [true] };\
    LET $snapshot = IF $has_crdt { (SELECT VALUE id FROM kernel_crdt_snapshots WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id AND snapshot_bytes_ref = $crdt_snapshot_ref AND snapshot_id = $crdt_snapshot_id AND snapshot_sha256 = $crdt_snapshot_sha256 AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.event_type = 'KNOWLEDGE_CRDT_SNAPSHOT_RECORDED' AND event_ledger_event_id.aggregate_type = 'knowledge_crdt_document' AND event_ledger_event_id.aggregate_id = crdt_document_id AND event_ledger_event_id.actor_id = actor_id AND event_ledger_event_id.payload.snapshot_id = snapshot_id AND event_ledger_event_id.payload.snapshot_sha256 = snapshot_sha256 AND event_ledger_event_id.payload.state_vector = state_vector LIMIT 2) } ELSE { [true] };\
    LET $lease = IF $has_crdt { (SELECT VALUE id FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND lease_id = $crdt_lease_id AND lane_id = $source_lane_id AND actor_id = $crdt_actor_id AND actor_kind = $crdt_actor_kind AND session_id = $crdt_session_id AND correlation_id = $crdt_trace_id AND scope_kind = $crdt_lease_scope_kind AND scope_id = $crdt_lease_scope_id AND ((scope_kind = 'workspace' AND scope_id = $workspace_id) OR (scope_kind = 'document' AND scope_id = $crdt_crdt_document_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id)) AND claimed_at_utc = $crdt_lease_claimed_at_utc AND expires_at_utc >= $crdt_lease_expires_at_utc AND claimed_at_utc <= $crdt_lease_admitted_at_utc AND expires_at_utc > $crdt_lease_admitted_at_utc AND expires_at_utc > time::now() AND released_at_utc IS NONE AND expired_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND recorded_event_id.event_type = 'KNOWLEDGE_CRDT_LEASE_CLAIMED' AND recorded_event_id.aggregate_type = 'knowledge_crdt_lease' AND recorded_event_id.aggregate_id = lease_id AND recorded_event_id.actor_id = actor_id AND recorded_event_id.correlation_id = correlation_id AND recorded_event_id.payload.lease_id = lease_id AND recorded_event_id.payload.actor_id = actor_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id AND last_transition_event_id.event_type IN ['KNOWLEDGE_CRDT_LEASE_CLAIMED', 'KNOWLEDGE_CRDT_LEASE_RENEWED'] AND last_transition_event_id.aggregate_type = 'knowledge_crdt_lease' AND last_transition_event_id.aggregate_id = lease_id AND last_transition_event_id.actor_id = actor_id AND last_transition_event_id.correlation_id = correlation_id AND last_transition_event_id.payload.lease_id = lease_id AND last_transition_event_id.payload.actor_id = actor_id LIMIT 2) } ELSE { [true] };\
    LET $proposal = IF $has_crdt_proposal { (SELECT VALUE id FROM knowledge_crdt_ai_edit_proposals WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND proposal_id = $crdt_proposal_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id AND actor_id = $crdt_actor_id AND actor_kind = $crdt_actor_kind AND session_id = $crdt_session_id AND correlation_id = $crdt_trace_id AND review_state IN ['approved', 'promoted'] AND diff_sha256 = $crdt_proposal_diff_sha256 AND applied_update_id = $crdt_update_id AND applied_update_sha256 = $crdt_proposal_diff_sha256 AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND recorded_event_id.event_type = 'AI_EDIT_PROPOSAL_RECORDED' AND recorded_event_id.aggregate_type = 'knowledge_crdt_ai_edit_proposal' AND recorded_event_id.aggregate_id = proposal_id AND recorded_event_id.actor_id = actor_id AND recorded_event_id.correlation_id = correlation_id AND recorded_event_id.payload.proposal_id = proposal_id AND recorded_event_id.payload.diff_sha256 = diff_sha256 AND decided_event_id.owner_account_id = $owner_account_id AND decided_event_id.actor_principal_id = $actor_principal_id AND decided_event_id.authenticated_session_id = $authenticated_session_id AND decided_event_id.access_space_id = $access_space_id AND decided_event_id.workspace_id = $workspace_id AND decided_event_id.event_type = 'AI_EDIT_PROPOSAL_DECIDED' AND decided_event_id.aggregate_type = 'knowledge_crdt_ai_edit_proposal' AND decided_event_id.aggregate_id = proposal_id AND decided_event_id.payload.proposal_id = proposal_id AND decided_event_id.payload.review_state = 'approved' AND applied_event_id.owner_account_id = $owner_account_id AND applied_event_id.actor_principal_id = $actor_principal_id AND applied_event_id.authenticated_session_id = $authenticated_session_id AND applied_event_id.access_space_id = $access_space_id AND applied_event_id.workspace_id = $workspace_id AND applied_event_id.event_type = 'AI_EDIT_PROPOSAL_DECIDED' AND applied_event_id.aggregate_type = 'knowledge_crdt_ai_edit_proposal' AND applied_event_id.aggregate_id = proposal_id AND applied_event_id.actor_id = actor_id AND applied_event_id.correlation_id = correlation_id AND applied_event_id.payload.proposal_id = proposal_id AND applied_event_id.payload.applied_update_id = applied_update_id AND applied_event_id.payload.applied_update_sha256 = diff_sha256 AND applied_event_id.payload.approved_diff_sha256 = diff_sha256 AND applied_event_id.payload.yjs_update_sha256 = $crdt_update_sha256 AND ((review_state = 'approved' AND last_transition_event_id = applied_event_id) OR (review_state = 'promoted' AND promotion_requested_event_id != promotion_accepted_event_id AND last_transition_event_id = promotion_accepted_event_id AND promotion_requested_event_id.owner_account_id = $owner_account_id AND promotion_requested_event_id.actor_principal_id = $actor_principal_id AND promotion_requested_event_id.authenticated_session_id = $authenticated_session_id AND promotion_requested_event_id.access_space_id = $access_space_id AND promotion_requested_event_id.workspace_id = $workspace_id AND promotion_requested_event_id.event_type = 'PROMOTION_REQUESTED' AND promotion_requested_event_id.aggregate_type = 'knowledge_ai_edit_promotion' AND promotion_requested_event_id.aggregate_id = proposal_id AND promotion_requested_event_id.source_component = 'knowledge_crdt_ai_edit_proposal' AND promotion_requested_event_id.correlation_id = correlation_id AND promotion_requested_event_id.payload.proposal_id = proposal_id AND promotion_requested_event_id.payload.diff_sha256 = diff_sha256 AND promotion_requested_event_id.payload.base_update_seq = base_update_seq AND promotion_requested_event_id.payload.base_state_vector = base_state_vector AND promotion_requested_event_id.payload.decided_by = promotion_requested_event_id.actor_id AND promotion_accepted_event_id.owner_account_id = $owner_account_id AND promotion_accepted_event_id.actor_principal_id = $actor_principal_id AND promotion_accepted_event_id.authenticated_session_id = $authenticated_session_id AND promotion_accepted_event_id.access_space_id = $access_space_id AND promotion_accepted_event_id.workspace_id = $workspace_id AND promotion_accepted_event_id.event_type = 'PROMOTION_ACCEPTED' AND promotion_accepted_event_id.aggregate_type = 'knowledge_ai_edit_promotion' AND promotion_accepted_event_id.aggregate_id = proposal_id AND promotion_accepted_event_id.source_component = 'knowledge_crdt_ai_edit_proposal' AND promotion_accepted_event_id.actor_id = promotion_requested_event_id.actor_id AND promotion_accepted_event_id.correlation_id = correlation_id AND promotion_accepted_event_id.causation_id = record::id(promotion_requested_event_id) AND promotion_accepted_event_id.payload.proposal_id = proposal_id AND promotion_accepted_event_id.payload.review_state = 'promoted' AND promotion_accepted_event_id.payload.decided_by = promotion_accepted_event_id.actor_id AND promotion_accepted_event_id.payload.diff_sha256 = diff_sha256 AND promotion_accepted_event_id.payload.applied_update_id = applied_update_id AND promotion_accepted_event_id.payload.applied_update_sha256 = applied_update_sha256)) AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };\
    IF array::len($source) != 1 OR array::len($session_owners) != 1 OR array::len($promotion) != 1 OR array::len($update) != 1 OR array::len($snapshot) != 1 OR array::len($lease) != 1 OR array::len($proposal) != 1 { THROW 'model-lane message authority changed before admission'; };\
    LET $message_ledger = CREATE type::record('kernel_event_ledger', $message_event_id) CONTENT { event_id: $message_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $message_run_id, session_run_id: $message_run_id, aggregate_type: 'message', aggregate_id: $message_aggregate_id, idempotency_key: $message_event_id, event_type: 'MODEL_LANE_MESSAGE_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $message_event_payload_hash, source_component: 'model_lane', payload: { record_kind: 'message', run_id: $message_run_id, event_stream_version: 1, event_payload_json: $message_event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
    LET $message_seq = $message_ledger[0].event_sequence;\
    LET $message = CREATE type::record('model_lane_authority', $message_record_id) CONTENT { record_kind: 'message', aggregate_id: $message_aggregate_id, run_id: $message_run_id, idempotency_key: $message_idempotency_key, record_json: $message_record_json, search_terms: $message_search_terms, event_id: $message_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $message_event_id), event_seq: $message_seq, event_stream_version: 1, transaction_seq: $message_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
    IF $has_payload_binding {\
      LET $binding_ledger = CREATE type::record('kernel_event_ledger', $binding_event_id) CONTENT { event_id: $binding_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $binding_run_id, session_run_id: $binding_run_id, aggregate_type: $binding_record_kind, aggregate_id: $binding_aggregate_id, idempotency_key: $binding_event_id, event_type: 'MODEL_LANE_PAYLOAD_BOUND', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $message_event_id, correlation_id: NONE, payload_hash: $binding_event_payload_hash, source_component: 'model_lane', payload: { record_kind: $binding_record_kind, run_id: $binding_run_id, event_stream_version: 1, event_payload_json: $binding_event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
      LET $binding_seq = $binding_ledger[0].event_sequence;\
      LET $binding = CREATE type::record('model_lane_authority', $binding_record_id) CONTENT { record_kind: $binding_record_kind, aggregate_id: $binding_aggregate_id, run_id: $binding_run_id, idempotency_key: $binding_idempotency_key, record_json: $binding_record_json, search_terms: $binding_search_terms, event_id: $binding_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $binding_event_id), event_seq: $binding_seq, event_stream_version: 1, transaction_seq: $binding_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
      array::concat($message, $binding)\
    } ELSE { $message };\
  };\
);\
RETURN $result;\
COMMIT TRANSACTION;";
const ROUTING_COMMIT_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $current = (SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq, event_ledger_event_id.event_id AS linked_event_id, event_ledger_event_id.event_version AS linked_event_version, event_ledger_event_id.kernel_task_run_id AS linked_kernel_task_run_id, event_ledger_event_id.session_run_id AS linked_session_run_id, event_ledger_event_id.aggregate_type AS linked_aggregate_type, event_ledger_event_id.aggregate_id AS linked_aggregate_id, event_ledger_event_id.idempotency_key AS linked_idempotency_key, event_ledger_event_id.event_type AS linked_event_type, event_ledger_event_id.actor_kind AS linked_actor_kind, event_ledger_event_id.actor_id AS linked_actor_id, event_ledger_event_id.causation_id AS linked_causation_id, event_ledger_event_id.correlation_id AS linked_correlation_id, event_ledger_event_id.payload_hash AS linked_payload_hash, event_ledger_event_id.source_component AS linked_source_component, event_ledger_event_id.payload AS linked_payload FROM type::record('model_lane_routing_execution', $execution_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $execution.execution_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2);
LET $attempt_existing = (SELECT attempt_id, execution_id, run_id, stage_id, attempt, state, lease_owner, fencing_token, lease_expires_at_unix_ms, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq, event_ledger_event_id.event_id AS linked_event_id, event_ledger_event_id.event_version AS linked_event_version, event_ledger_event_id.kernel_task_run_id AS linked_kernel_task_run_id, event_ledger_event_id.session_run_id AS linked_session_run_id, event_ledger_event_id.aggregate_type AS linked_aggregate_type, event_ledger_event_id.aggregate_id AS linked_aggregate_id, event_ledger_event_id.idempotency_key AS linked_idempotency_key, event_ledger_event_id.event_type AS linked_event_type, event_ledger_event_id.actor_kind AS linked_actor_kind, event_ledger_event_id.actor_id AS linked_actor_id, event_ledger_event_id.causation_id AS linked_causation_id, event_ledger_event_id.correlation_id AS linked_correlation_id, event_ledger_event_id.payload_hash AS linked_payload_hash, event_ledger_event_id.source_component AS linked_source_component, event_ledger_event_id.payload AS linked_payload FROM type::record('model_lane_routing_stage_attempt', $attempt_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_stage_attempt' AND event_ledger_event_id.aggregate_id = $attempt.attempt_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2);
LET $outbox_existing = (SELECT command_id, execution_id, run_id, stage_id, attempt, status, lease_owner, fencing_token, lease_expires_at_unix_ms, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq, created_at_unix_ms, updated_at_unix_ms, event_ledger_event_id.event_id AS linked_event_id, event_ledger_event_id.event_version AS linked_event_version, event_ledger_event_id.kernel_task_run_id AS linked_kernel_task_run_id, event_ledger_event_id.session_run_id AS linked_session_run_id, event_ledger_event_id.aggregate_type AS linked_aggregate_type, event_ledger_event_id.aggregate_id AS linked_aggregate_id, event_ledger_event_id.idempotency_key AS linked_idempotency_key, event_ledger_event_id.event_type AS linked_event_type, event_ledger_event_id.actor_kind AS linked_actor_kind, event_ledger_event_id.actor_id AS linked_actor_id, event_ledger_event_id.causation_id AS linked_causation_id, event_ledger_event_id.correlation_id AS linked_correlation_id, event_ledger_event_id.payload_hash AS linked_payload_hash, event_ledger_event_id.source_component AS linked_source_component, event_ledger_event_id.payload AS linked_payload FROM type::record('model_lane_routing_outbox', $outbox_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_outbox' AND event_ledger_event_id.aggregate_id = $outbox.command_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2);
LET $claim_attempt = IF $has_expected_claim { (SELECT attempt_id, execution_id, stage_id, attempt, lease_owner, fencing_token, lease_expires_at_unix_ms, event_ledger_seq FROM model_lane_routing_stage_attempt WHERE attempt_id = $attempt.attempt_id AND execution_id = $execution.execution_id AND stage_id = $expected_claim.stage_id AND attempt = $expected_claim.attempt AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_stage_attempt' AND event_ledger_event_id.aggregate_id = attempt_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2) } ELSE { [] };
LET $message_existing = IF $has_message { (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $message.record_id) WHERE record_kind = 'message' AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'message' AND event_ledger_event_id.aggregate_id = $message.aggregate_id LIMIT 2) } ELSE { [] };
LET $binding_existing = IF $has_binding { (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $binding.record_id) WHERE record_kind = $binding.record_kind AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = $binding.record_kind AND event_ledger_event_id.aggregate_id = $binding.aggregate_id LIMIT 2) } ELSE { [] };
LET $extra_event_links = (SELECT idempotency_key, record::id(event_ledger_event_id) AS event_id, event_ledger_seq FROM model_lane_routing_extra_event_link WHERE execution_id = $execution.execution_id AND run_id = $execution.run_id AND revision = $execution.revision AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 4097);
LET $run_authority = (SELECT VALUE id FROM type::record('model_lane_authority', $run_record_id) WHERE record_kind = 'run' AND aggregate_id = $execution.run_id AND run_id = $execution.run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'run' AND event_ledger_event_id.aggregate_id = $execution.run_id LIMIT 2);
LET $source_lane = IF $has_message { (SELECT VALUE id FROM type::record('model_lane_authority', $message_guard.source_lane_record_id) WHERE record_kind = 'lane' AND aggregate_id = $message_guard.source_lane_id AND record_json = $message_guard.source_lane_record_json AND $message_guard.source_session_term IN search_terms AND $message_guard.source_model_session_term IN search_terms AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'lane' AND event_ledger_event_id.aggregate_id = $message_guard.source_lane_id LIMIT 2) } ELSE { [true] };
LET $result = IF array::len($current) > 1 OR array::len($attempt_existing) > 1 OR array::len($outbox_existing) > 1 OR array::len($message_existing) > 1 OR array::len($binding_existing) > 1 OR array::len($run_authority) != 1 OR array::len($source_lane) != 1 {
    THROW 'model-lane routing authority is ambiguous or incomplete';
} ELSE IF array::len($current) = 1 AND $current[0].revision = $execution.revision {
    IF $current[0].run_id != $execution.run_id OR $current[0].context_hash != $execution.context_hash OR $current[0].record_json != $execution.record_json OR $current[0].event_ledger_event_id != $execution_event.event_id OR $current[0].linked_event_id != $execution_event.event_id OR $current[0].linked_event_version != $execution_event.event_version OR $current[0].linked_kernel_task_run_id != $execution_event.kernel_task_run_id OR $current[0].linked_session_run_id != $execution_event.session_run_id OR $current[0].linked_aggregate_type != $execution_event.aggregate_type OR $current[0].linked_aggregate_id != $execution_event.aggregate_id OR $current[0].linked_idempotency_key != $execution_event.idempotency_key OR $current[0].linked_event_type != $execution_event.event_type OR $current[0].linked_actor_kind != $execution_event.actor_kind OR $current[0].linked_actor_id != $execution_event.actor_id OR $current[0].linked_causation_id != $execution_event.causation_id OR $current[0].linked_correlation_id != $execution_event.correlation_id OR $current[0].linked_payload_hash != $execution_event.payload_hash OR $current[0].linked_source_component != $execution_event.source_component OR $current[0].linked_payload != $execution_event.payload { THROW 'model-lane routing execution retry conflict'; };
    IF array::len($attempt_existing) != 1 OR $attempt_existing[0].attempt_id != $attempt.attempt_id OR $attempt_existing[0].execution_id != $attempt.execution_id OR $attempt_existing[0].run_id != $attempt.run_id OR $attempt_existing[0].stage_id != $attempt.stage_id OR $attempt_existing[0].attempt != $attempt.attempt OR $attempt_existing[0].state != $attempt.state OR $attempt_existing[0].lease_owner != $attempt.lease_owner OR $attempt_existing[0].fencing_token != $attempt.fencing_token OR $attempt_existing[0].lease_expires_at_unix_ms != $attempt.lease_expires_at_unix_ms OR $attempt_existing[0].record_json != $attempt.record_json OR $attempt_existing[0].event_ledger_event_id != $attempt_event.event_id OR $attempt_existing[0].linked_event_id != $attempt_event.event_id OR $attempt_existing[0].linked_event_version != $attempt_event.event_version OR $attempt_existing[0].linked_kernel_task_run_id != $attempt_event.kernel_task_run_id OR $attempt_existing[0].linked_session_run_id != $attempt_event.session_run_id OR $attempt_existing[0].linked_aggregate_type != $attempt_event.aggregate_type OR $attempt_existing[0].linked_aggregate_id != $attempt_event.aggregate_id OR $attempt_existing[0].linked_idempotency_key != $attempt_event.idempotency_key OR $attempt_existing[0].linked_event_type != $attempt_event.event_type OR $attempt_existing[0].linked_actor_kind != $attempt_event.actor_kind OR $attempt_existing[0].linked_actor_id != $attempt_event.actor_id OR $attempt_existing[0].linked_causation_id != $attempt_event.causation_id OR $attempt_existing[0].linked_correlation_id != $attempt_event.correlation_id OR $attempt_existing[0].linked_payload_hash != $attempt_event.payload_hash OR $attempt_existing[0].linked_source_component != $attempt_event.source_component OR $attempt_existing[0].linked_payload != $attempt_event.payload { THROW 'model-lane routing attempt retry conflict'; };
    IF array::len($outbox_existing) != 1 OR $outbox_existing[0].command_id != $outbox.command_id OR $outbox_existing[0].execution_id != $outbox.execution_id OR $outbox_existing[0].run_id != $outbox.run_id OR $outbox_existing[0].stage_id != $outbox.stage_id OR $outbox_existing[0].attempt != $outbox.attempt OR $outbox_existing[0].status != $outbox.status OR $outbox_existing[0].lease_owner != $outbox.lease_owner OR $outbox_existing[0].fencing_token != $outbox.fencing_token OR $outbox_existing[0].lease_expires_at_unix_ms != $outbox.lease_expires_at_unix_ms OR $outbox_existing[0].created_at_unix_ms != $outbox.created_at_unix_ms OR $outbox_existing[0].updated_at_unix_ms != $outbox.updated_at_unix_ms OR $outbox_existing[0].event_ledger_event_id != $outbox_event.event_id OR $outbox_existing[0].linked_event_id != $outbox_event.event_id OR $outbox_existing[0].linked_event_version != $outbox_event.event_version OR $outbox_existing[0].linked_kernel_task_run_id != $outbox_event.kernel_task_run_id OR $outbox_existing[0].linked_session_run_id != $outbox_event.session_run_id OR $outbox_existing[0].linked_aggregate_type != $outbox_event.aggregate_type OR $outbox_existing[0].linked_aggregate_id != $outbox_event.aggregate_id OR $outbox_existing[0].linked_idempotency_key != $outbox_event.idempotency_key OR $outbox_existing[0].linked_event_type != $outbox_event.event_type OR $outbox_existing[0].linked_actor_kind != $outbox_event.actor_kind OR $outbox_existing[0].linked_actor_id != $outbox_event.actor_id OR $outbox_existing[0].linked_causation_id != $outbox_event.causation_id OR $outbox_existing[0].linked_correlation_id != $outbox_event.correlation_id OR $outbox_existing[0].linked_payload_hash != $outbox_event.payload_hash OR $outbox_existing[0].linked_source_component != $outbox_event.source_component OR $outbox_existing[0].linked_payload != $outbox_event.payload { THROW 'model-lane routing outbox retry conflict'; };
    IF $has_message AND (array::len($message_existing) != 1 OR $message_existing[0].aggregate_id != $message.aggregate_id OR $message_existing[0].run_id != $message.run_id OR $message_existing[0].idempotency_key != $message.idempotency_key OR $message_existing[0].record_json != $message.record_json) { THROW 'model-lane routing message retry conflict'; };
    IF $has_binding AND (array::len($binding_existing) != 1 OR $binding_existing[0].aggregate_id != $binding.aggregate_id OR $binding_existing[0].run_id != $binding.run_id OR $binding_existing[0].idempotency_key != $binding.idempotency_key OR $binding_existing[0].record_json != $binding.record_json) { THROW 'model-lane routing binding retry conflict'; };
    IF array::len($extra_event_links) != array::len($extra_events) { THROW 'model-lane routing extra-event retry set conflict'; };
    FOR $event IN $extra_events {
        LET $receipt = (SELECT event_id, event_sequence, event_version, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, causation_id, correlation_id, payload_hash, source_component, payload FROM type::record('kernel_event_ledger', $event.event_id) WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
        LET $link = (SELECT idempotency_key, record::id(event_ledger_event_id) AS event_id, event_ledger_seq FROM type::record('model_lane_routing_extra_event_link', $event.idempotency_key) WHERE execution_id = $execution.execution_id AND run_id = $execution.run_id AND revision = $execution.revision AND idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2);
        IF array::len($receipt) != 1 OR array::len($link) != 1 OR $link[0].idempotency_key != $event.idempotency_key OR $link[0].event_id != $event.event_id OR $link[0].event_id != $receipt[0].event_id OR $link[0].event_ledger_seq != $receipt[0].event_sequence OR $receipt[0].idempotency_key != $event.idempotency_key OR $receipt[0].event_version != $event.event_version OR $receipt[0].kernel_task_run_id != $event.kernel_task_run_id OR $receipt[0].session_run_id != $event.session_run_id OR $receipt[0].aggregate_type != $event.aggregate_type OR $receipt[0].aggregate_id != $event.aggregate_id OR $receipt[0].event_type != $event.event_type OR $receipt[0].actor_kind != $event.actor_kind OR $receipt[0].actor_id != $event.actor_id OR $receipt[0].causation_id != $event.causation_id OR $receipt[0].correlation_id != $event.correlation_id OR $receipt[0].payload_hash != $event.payload_hash OR $receipt[0].source_component != $event.source_component OR $receipt[0].payload != $event.payload { THROW 'model-lane routing extra-event retry conflict'; };
    };
    $current
} ELSE {
    IF $execution.revision != $expected_revision + 1 { THROW 'stale model-lane routing execution revision'; };
    IF array::len($current) = 0 {
        IF $expected_revision != 0 OR $execution.revision != 1 { THROW 'model-lane routing initial creation requires expected revision zero and next revision one'; };
        IF $has_expected_claim OR array::len($attempt_existing) != 0 OR array::len($outbox_existing) != 0 OR array::len($message_existing) != 0 OR array::len($binding_existing) != 0 { THROW 'model-lane routing initial creation found orphan durable state'; };
    } ELSE IF $current[0].revision != $expected_revision {
        THROW 'stale model-lane routing execution revision';
    };
    IF array::len($current) = 1 AND $current[0].context_hash != $execution.context_hash { THROW 'model-lane routing immutable context changed'; };
    IF $execution.execution_id != $attempt.execution_id OR $execution.execution_id != $outbox.execution_id OR $execution.run_id != $attempt.run_id OR $execution.run_id != $outbox.run_id OR $attempt.stage_id != $outbox.stage_id OR $attempt.attempt != $outbox.attempt { THROW 'mixed model-lane routing mutation identity'; };
    IF (array::len($attempt_existing) = 0) != (array::len($outbox_existing) = 0) { THROW 'partial model-lane routing attempt/outbox authority'; };
    IF $has_expected_claim AND (array::len($claim_attempt) != 1 OR $claim_attempt[0].lease_owner != $expected_claim.lease_owner OR $claim_attempt[0].fencing_token != $expected_claim.fencing_token OR $claim_attempt[0].lease_expires_at_unix_ms = NONE OR $claim_attempt[0].lease_expires_at_unix_ms < $expected_claim.observed_at_unix_ms) { THROW 'stale model-lane routing claim'; };
    IF array::len($message_existing) != 0 OR array::len($binding_existing) != 0 { THROW 'model-lane routing mutation found orphan message or binding'; };
    IF array::len($extra_event_links) != 0 { THROW 'model-lane routing extra-event link exists without its complete revision'; };
    LET $execution_event_existing = (SELECT id FROM kernel_event_ledger WHERE idempotency_key = $execution_event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    LET $attempt_event_existing = (SELECT id FROM kernel_event_ledger WHERE idempotency_key = $attempt_event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    LET $outbox_event_existing = (SELECT id FROM kernel_event_ledger WHERE idempotency_key = $outbox_event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($execution_event_existing) != 0 OR array::len($attempt_event_existing) != 0 OR array::len($outbox_event_existing) != 0 { THROW 'model-lane routing event exists without its complete projection'; };
    LET $execution_ledger = CREATE type::record('kernel_event_ledger', $execution_event.event_id) CONTENT { event_id: $execution_event.event_id, event_version: $execution_event.event_version, kernel_task_run_id: $execution_event.kernel_task_run_id, session_run_id: $execution_event.session_run_id, aggregate_type: $execution_event.aggregate_type, aggregate_id: $execution_event.aggregate_id, idempotency_key: $execution_event.idempotency_key, event_type: $execution_event.event_type, actor_kind: $execution_event.actor_kind, actor_id: $execution_event.actor_id, causation_id: $execution_event.causation_id, correlation_id: $execution_event.correlation_id, payload_hash: $execution_event.payload_hash, source_component: $execution_event.source_component, payload: $execution_event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: $execution_event.created_at };
    LET $attempt_ledger = CREATE type::record('kernel_event_ledger', $attempt_event.event_id) CONTENT { event_id: $attempt_event.event_id, event_version: $attempt_event.event_version, kernel_task_run_id: $attempt_event.kernel_task_run_id, session_run_id: $attempt_event.session_run_id, aggregate_type: $attempt_event.aggregate_type, aggregate_id: $attempt_event.aggregate_id, idempotency_key: $attempt_event.idempotency_key, event_type: $attempt_event.event_type, actor_kind: $attempt_event.actor_kind, actor_id: $attempt_event.actor_id, causation_id: $attempt_event.causation_id, correlation_id: $attempt_event.correlation_id, payload_hash: $attempt_event.payload_hash, source_component: $attempt_event.source_component, payload: $attempt_event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: $attempt_event.created_at };
    LET $outbox_ledger = CREATE type::record('kernel_event_ledger', $outbox_event.event_id) CONTENT { event_id: $outbox_event.event_id, event_version: $outbox_event.event_version, kernel_task_run_id: $outbox_event.kernel_task_run_id, session_run_id: $outbox_event.session_run_id, aggregate_type: $outbox_event.aggregate_type, aggregate_id: $outbox_event.aggregate_id, idempotency_key: $outbox_event.idempotency_key, event_type: $outbox_event.event_type, actor_kind: $outbox_event.actor_kind, actor_id: $outbox_event.actor_id, causation_id: $outbox_event.causation_id, correlation_id: $outbox_event.correlation_id, payload_hash: $outbox_event.payload_hash, source_component: $outbox_event.source_component, payload: $outbox_event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: $outbox_event.created_at };
    FOR $event IN $extra_events {
        LET $event_existing = (SELECT id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
        IF array::len($event_existing) != 0 { THROW 'model-lane routing extra event exists without its complete projection'; };
        LET $extra_ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: $event.created_at };
        LET $extra_link = CREATE type::record('model_lane_routing_extra_event_link', $event.idempotency_key) CONTENT { execution_id: $execution.execution_id, run_id: $execution.run_id, revision: $execution.revision, idempotency_key: $event.idempotency_key, event_ledger_event_id: type::record('kernel_event_ledger', $event.event_id), event_ledger_seq: $extra_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    };
    LET $stored_execution = IF array::len($current) = 0 { CREATE type::record('model_lane_routing_execution', $execution_record_id) CONTENT { execution_id: $execution.execution_id, run_id: $execution.run_id, revision: $execution.revision, context_hash: $execution.context_hash, record_json: $execution.record_json, event_ledger_event_id: type::record('kernel_event_ledger', $execution_event.event_id), event_ledger_seq: $execution_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } } ELSE { UPDATE type::record('model_lane_routing_execution', $execution_record_id) CONTENT { execution_id: $execution.execution_id, run_id: $execution.run_id, revision: $execution.revision, context_hash: $execution.context_hash, record_json: $execution.record_json, event_ledger_event_id: type::record('kernel_event_ledger', $execution_event.event_id), event_ledger_seq: $execution_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } };
    LET $stored_attempt = IF array::len($current) = 0 { CREATE type::record('model_lane_routing_stage_attempt', $attempt_record_id) CONTENT { attempt_id: $attempt.attempt_id, execution_id: $attempt.execution_id, run_id: $attempt.run_id, stage_id: $attempt.stage_id, attempt: $attempt.attempt, state: $attempt.state, lease_owner: $attempt.lease_owner, fencing_token: $attempt.fencing_token, lease_expires_at_unix_ms: $attempt.lease_expires_at_unix_ms, record_json: $attempt.record_json, event_ledger_event_id: type::record('kernel_event_ledger', $attempt_event.event_id), event_ledger_seq: $attempt_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } } ELSE { UPSERT type::record('model_lane_routing_stage_attempt', $attempt_record_id) CONTENT { attempt_id: $attempt.attempt_id, execution_id: $attempt.execution_id, run_id: $attempt.run_id, stage_id: $attempt.stage_id, attempt: $attempt.attempt, state: $attempt.state, lease_owner: $attempt.lease_owner, fencing_token: $attempt.fencing_token, lease_expires_at_unix_ms: $attempt.lease_expires_at_unix_ms, record_json: $attempt.record_json, event_ledger_event_id: type::record('kernel_event_ledger', $attempt_event.event_id), event_ledger_seq: $attempt_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } };
    LET $stored_outbox = IF array::len($current) = 0 { CREATE type::record('model_lane_routing_outbox', $outbox_record_id) CONTENT { command_id: $outbox.command_id, execution_id: $outbox.execution_id, run_id: $outbox.run_id, stage_id: $outbox.stage_id, attempt: $outbox.attempt, status: $outbox.status, lease_owner: $outbox.lease_owner, fencing_token: $outbox.fencing_token, lease_expires_at_unix_ms: $outbox.lease_expires_at_unix_ms, event_ledger_event_id: type::record('kernel_event_ledger', $outbox_event.event_id), event_ledger_seq: $outbox_ledger[0].event_sequence, created_at_unix_ms: $outbox.created_at_unix_ms, updated_at_unix_ms: $outbox.updated_at_unix_ms, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } } ELSE { UPSERT type::record('model_lane_routing_outbox', $outbox_record_id) CONTENT { command_id: $outbox.command_id, execution_id: $outbox.execution_id, run_id: $outbox.run_id, stage_id: $outbox.stage_id, attempt: $outbox.attempt, status: $outbox.status, lease_owner: $outbox.lease_owner, fencing_token: $outbox.fencing_token, lease_expires_at_unix_ms: $outbox.lease_expires_at_unix_ms, event_ledger_event_id: type::record('kernel_event_ledger', $outbox_event.event_id), event_ledger_seq: $outbox_ledger[0].event_sequence, created_at_unix_ms: IF array::len($outbox_existing) = 1 { $outbox_existing[0].created_at_unix_ms } ELSE { $outbox.created_at_unix_ms }, updated_at_unix_ms: $outbox.updated_at_unix_ms, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } };
    IF $has_message {
        LET $message_ledger = CREATE type::record('kernel_event_ledger', $message.event_id) CONTENT { event_id: $message.event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $message.run_id, session_run_id: $message.run_id, aggregate_type: 'message', aggregate_id: $message.aggregate_id, idempotency_key: $message.event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $message.event_payload_hash, source_component: 'model_lane', payload: { record_kind: 'message', run_id: $message.run_id, event_stream_version: 1, event_payload_json: $message.event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
        LET $stored_message = CREATE type::record('model_lane_authority', $message.record_id) CONTENT { record_kind: 'message', aggregate_id: $message.aggregate_id, run_id: $message.run_id, idempotency_key: $message.idempotency_key, record_json: $message.record_json, search_terms: $message.search_terms, event_id: $message.event_id, event_ledger_event_id: type::record('kernel_event_ledger', $message.event_id), event_seq: $message_ledger[0].event_sequence, event_stream_version: 1, transaction_seq: $message_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    };
    IF $has_binding {
        LET $binding_ledger = CREATE type::record('kernel_event_ledger', $binding.event_id) CONTENT { event_id: $binding.event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $binding.run_id, session_run_id: $binding.run_id, aggregate_type: $binding.record_kind, aggregate_id: $binding.aggregate_id, idempotency_key: $binding.event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $binding.event_payload_hash, source_component: 'model_lane', payload: { record_kind: $binding.record_kind, run_id: $binding.run_id, event_stream_version: 1, event_payload_json: $binding.event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
        LET $stored_binding = CREATE type::record('model_lane_authority', $binding.record_id) CONTENT { record_kind: $binding.record_kind, aggregate_id: $binding.aggregate_id, run_id: $binding.run_id, idempotency_key: $binding.idempotency_key, record_json: $binding.record_json, search_terms: $binding.search_terms, event_id: $binding.event_id, event_ledger_event_id: type::record('kernel_event_ledger', $binding.event_id), event_seq: $binding_ledger[0].event_sequence, event_stream_version: 1, transaction_seq: $binding_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    };
    SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM type::record('model_lane_routing_execution', $execution_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $execution.execution_id AND event_ledger_seq = event_ledger_event_id.event_sequence
};
RETURN SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM type::record('model_lane_routing_execution', $execution_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $execution.execution_id AND event_ledger_seq = event_ledger_event_id.event_sequence;
COMMIT TRANSACTION;
"#;
#[cfg(feature = "surreal-test-support")]
const ROUTING_TEST_CORRUPTION_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $execution = (SELECT record::id(event_ledger_event_id) AS event_id FROM type::record('model_lane_routing_execution', $execution_record_id) WHERE execution_id = $execution_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $execution_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2);
LET $attempt = (SELECT record::id(event_ledger_event_id) AS event_id FROM type::record('model_lane_routing_stage_attempt', $attempt_record_id) WHERE attempt_id = $attempt_id AND execution_id = $execution_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_stage_attempt' AND event_ledger_event_id.aggregate_id = $attempt_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2);
LET $outbox = (SELECT record::id(event_ledger_event_id) AS event_id FROM type::record('model_lane_routing_outbox', $outbox_record_id) WHERE command_id = $outbox_id AND execution_id = $execution_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_outbox' AND event_ledger_event_id.aggregate_id = $outbox_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2);
IF array::len($execution) != 1 OR array::len($attempt) != 1 OR array::len($outbox) != 1 { THROW 'routing test corruption target is incomplete or already corrupt'; };
IF $corruption = 'attempt_event_aggregate_type' {
    UPDATE type::record('kernel_event_ledger', $attempt[0].event_id) SET aggregate_type = 'routing_test_corrupt_aggregate';
} ELSE IF $corruption = 'attempt_event_aggregate_id' {
    UPDATE type::record('kernel_event_ledger', $attempt[0].event_id) SET aggregate_id = 'routing-test-corrupt-attempt';
} ELSE IF $corruption = 'execution_event_sequence' {
    UPDATE type::record('model_lane_routing_execution', $execution_record_id) SET event_ledger_seq += 1;
} ELSE IF $corruption = 'attempt_event_sequence' {
    UPDATE type::record('model_lane_routing_stage_attempt', $attempt_record_id) SET event_ledger_seq += 1;
} ELSE IF $corruption = 'outbox_event_sequence' {
    UPDATE type::record('model_lane_routing_outbox', $outbox_record_id) SET event_ledger_seq += 1;
} ELSE {
    THROW 'unsupported routing test corruption';
};
COMMIT TRANSACTION;
"#;
#[cfg(feature = "surreal-test-support")]
const CRDT_PROPOSAL_TEST_CORRUPTION_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $proposal = (SELECT VALUE id FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
LET $update = (SELECT VALUE id FROM kernel_crdt_updates WHERE update_id = $update_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
IF array::len($proposal) != 1 OR array::len($update) != 1 { THROW 'CRDT proposal test corruption target is unavailable in exact scope'; };
LET $links = (SELECT record::id(recorded_event_id) AS recorded_event_id, record::id(applied_event_id) AS applied_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id FROM $proposal[0]);
LET $recorded_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $links[0].recorded_event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
LET $applied_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $links[0].applied_event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
LET $promotion_accepted_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $links[0].promotion_accepted_event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
IF $corruption = 'recorded_receipt_aggregate' {
    IF array::len($recorded_event) != 1 { THROW 'recorded receipt corruption target is unavailable in exact scope'; };
    UPDATE $recorded_event[0] SET aggregate_id = 'tampered-crdt-proposal';
} ELSE IF $corruption = 'applied_receipt_payload_hash' {
    IF array::len($applied_event) != 1 { THROW 'applied receipt corruption target is unavailable in exact scope'; };
    UPDATE $applied_event[0] SET payload_hash = $tampered_sha256;
} ELSE IF $corruption = 'proposal_diff_hash' {
    UPDATE $proposal[0] SET diff_sha256 = $tampered_sha256;
} ELSE IF $corruption = 'update_content_hash' {
    UPDATE $update[0] SET update_sha256 = $tampered_sha256;
} ELSE IF $corruption = 'proposal_incomplete_attribution' {
    UPDATE $proposal[0] SET owner_account_id = 'legacy-unattributed';
} ELSE IF $corruption = 'applied_receipt_mixed_scope' {
    IF array::len($applied_event) != 1 { THROW 'applied receipt corruption target is unavailable in exact scope'; };
    UPDATE $applied_event[0] SET actor_principal_id = 'foreign-principal';
} ELSE IF $corruption = 'proposal_actor_identity' {
    UPDATE $proposal[0] SET actor_id = 'local_model:counterfactual-actor';
} ELSE IF $corruption = 'proposal_session_identity' {
    UPDATE $proposal[0] SET session_id = 'counterfactual-session';
} ELSE IF $corruption = 'proposal_trace_identity' {
    UPDATE $proposal[0] SET correlation_id = 'counterfactual-trace';
} ELSE IF $corruption = 'proposal_document_identity' {
    UPDATE $proposal[0] SET document_id = 'counterfactual-document';
} ELSE IF $corruption = 'promotion_accepted_causation' {
    IF array::len($promotion_accepted_event) != 1 { THROW 'promotion receipt corruption target is unavailable in exact scope'; };
    UPDATE $promotion_accepted_event[0] SET causation_id = 'tampered-promotion-request';
} ELSE {
    THROW 'unsupported CRDT proposal test corruption';
};
COMMIT TRANSACTION;
"#;

#[cfg(feature = "surreal-test-support")]
const AUTHORITY_TEST_CORRUPTION_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $row = (SELECT VALUE id FROM type::record('model_lane_authority', $record_id) WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
IF array::len($row) != 1 { THROW 'ModelLane authority corruption target is unavailable in exact scope'; };
LET $event = (SELECT VALUE event_ledger_event_id FROM $row[0]);
IF array::len($event) != 1 { THROW 'ModelLane authority corruption target has no canonical receipt'; };
IF $corruption = 'projection_event_sequence' {
    UPDATE $row[0] SET event_seq += 1;
} ELSE IF $corruption = 'projection_scope' {
    UPDATE $row[0] SET actor_principal_id = 'counterfactual-projection-principal';
} ELSE IF $corruption = 'receipt_payload_hash' {
    UPDATE $event[0] SET payload_hash = $tampered_sha256;
} ELSE IF $corruption = 'receipt_scope' {
    UPDATE $event[0] SET actor_principal_id = 'counterfactual-receipt-principal';
} ELSE {
    THROW 'unsupported ModelLane authority test corruption';
};
COMMIT TRANSACTION;
"#;
const RECORD_RECOVERY_CHECKPOINT_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $result = (
    LET $existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
    IF array::len($existing) = 1 {
        $existing
    } ELSE {
        IF array::len($existing) != 0 { THROW 'model-lane recovery checkpoint identity is ambiguous'; };
        LET $run = (SELECT VALUE id FROM type::record('model_lane_authority', $run_record_id) WHERE record_kind = 'run' AND aggregate_id = $run_id AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
        LET $lane = IF $has_lane { (SELECT VALUE id FROM type::record('model_lane_authority', $lane_record_id) WHERE record_kind = 'lane' AND aggregate_id = $lane_id AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };
        LET $head = (SELECT event_sequence AS event_seq FROM kernel_event_ledger WHERE source_component = 'model_lane' AND session_run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_sequence DESC LIMIT 1);
        IF array::len($run) != 1 OR array::len($lane) != 1 OR array::len($head) != 1 OR $head[0].event_seq != $expected_last_event_seq { THROW 'model-lane recovery checkpoint authority or high watermark changed'; };
        LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: $run_id, aggregate_type: 'recovery_checkpoint', aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_RECOVERY_CHECKPOINT_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: 'recovery_checkpoint', run_id: $run_id, event_stream_version: 1, event_payload_json: $event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
        LET $seq = $ledger[0].event_sequence;
        CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: 'recovery_checkpoint', aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $record_json, search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }
    };
);
RETURN $result;
COMMIT TRANSACTION;
"#;
const RECORD_RECOVERY_EVENT_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $result = (
    LET $existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
    IF array::len($existing) = 1 {
        $existing
    } ELSE {
        IF array::len($existing) != 0 { THROW 'model-lane recovery event identity is ambiguous'; };
        LET $run = (SELECT VALUE id FROM type::record('model_lane_authority', $run_record_id) WHERE record_kind = 'run' AND aggregate_id = $run_id AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
        LET $lane = IF $has_lane { (SELECT VALUE id FROM type::record('model_lane_authority', $lane_record_id) WHERE record_kind = 'lane' AND aggregate_id = $lane_id AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };
        LET $source = IF $has_source_event { (SELECT VALUE id FROM kernel_event_ledger WHERE source_component = 'model_lane' AND session_run_id = $run_id AND event_sequence = $source_event_seq AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };
        LET $order = (SELECT next_value FROM type::record('model_lane_recovery_order', $order_record_id) WHERE run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
        LET $current_order = IF array::len($order) = 0 { 0 } ELSE { $order[0].next_value };
        IF array::len($run) != 1 OR array::len($lane) != 1 OR array::len($source) != 1 { THROW 'model-lane recovery event authority changed'; };
        IF $expected_replay_order_seq != $current_order + 1 {
            []
        } ELSE {
            LET $order_counter = UPSERT type::record('model_lane_recovery_order', $order_record_id) CONTENT { run_id: $run_id, next_value: $expected_replay_order_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
            LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: $run_id, aggregate_type: 'recovery_event', aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_RECOVERY_EVENT_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: 'recovery_event', run_id: $run_id, event_stream_version: 1, source_event_seq: IF $has_source_event { $source_event_seq } ELSE { NONE }, replay_order_seq: $expected_replay_order_seq, event_payload_json: $event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
            LET $seq = $ledger[0].event_sequence;
            CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: 'recovery_event', aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $record_json, search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, ordering_seq: $expected_replay_order_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }
        }
    };
);
RETURN $result;
COMMIT TRANSACTION;
"#;
const APPEND_CRDT_UPDATE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM kernel_crdt_updates WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id AND update_id = $row.update_id AND event_ledger_event_id.owner_account_id = $row.owner_account_id AND event_ledger_event_id.actor_principal_id = $row.actor_principal_id AND event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id AND event_ledger_event_id.access_space_id = $row.access_space_id AND event_ledger_event_id.workspace_id = $row.workspace_id LIMIT 2);
LET $head = (SELECT update_seq, state_vector_after FROM kernel_crdt_updates WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id AND event_ledger_event_id.owner_account_id = $row.owner_account_id AND event_ledger_event_id.actor_principal_id = $row.actor_principal_id AND event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id AND event_ledger_event_id.access_space_id = $row.access_space_id AND event_ledger_event_id.workspace_id = $row.workspace_id ORDER BY update_seq DESC LIMIT 1);
LET $head_update_seq = IF array::len($head) = 0 { 0 } ELSE { $head[0].update_seq };
LET $head_state_vector = IF array::len($head) = 0 { 'hsk-sv1:' } ELSE { $head[0].state_vector_after };
LET $result = IF array::len($existing) > 1 {
    THROW 'model-lane CRDT update identity is ambiguous';
} ELSE IF array::len($existing) = 1 {
    IF $existing[0].schema_id = $row.schema_id AND $existing[0].update_seq = $row.update_seq AND $existing[0].update_sha256 = $row.update_sha256 AND $existing[0].update_bytes_ref = $row.update_bytes_ref AND $existing[0].update_bytes_b64 = $row.update_bytes_b64 AND $existing[0].actor_id = $row.actor_id AND $existing[0].actor_kind = $row.actor_kind AND $existing[0].session_id = $row.session_id AND $existing[0].trace_id = $row.trace_id AND $existing[0].state_vector_before = $row.state_vector_before AND $existing[0].state_vector_after = $row.state_vector_after AND $existing[0].replay_order_key = $row.replay_order_key AND $existing[0].dependency_update_ids = $row.dependency_update_ids AND $existing[0].replay_encoding = $row.replay_encoding AND $existing[0].replay_schema_version = $row.replay_schema_version AND $existing[0].event_ledger_stream_id = $row.event_ledger_stream_id AND $existing[0].event_ledger_event_id = $row.event_ledger_event_id AND $existing[0].storage_authority = 'embedded_surrealdb' {
        { outcome: 'already_stored', head_update_seq: $head_update_seq, head_state_vector: $head_state_vector, record: $existing[0] }
    } ELSE {
        { outcome: 'content_mismatch', head_update_seq: $head_update_seq, head_state_vector: $head_state_vector, record: NONE }
    }
} ELSE IF $head_update_seq != $expected_head_update_seq OR $head_state_vector != $expected_head_state_vector OR $row.update_seq != $head_update_seq + 1 {
    { outcome: 'stale_head', head_update_seq: $head_update_seq, head_state_vector: $head_state_vector, record: NONE }
} ELSE {
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT event exists without its update'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
    LET $created = CREATE type::record('kernel_crdt_updates', $record_id) CONTENT { schema_id: $row.schema_id, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id, document_id: $row.document_id, crdt_document_id: $row.crdt_document_id, update_id: $row.update_id, update_seq: $row.update_seq, update_sha256: $row.update_sha256, update_bytes_ref: $row.update_bytes_ref, update_bytes: encoding::base64::decode($row.update_bytes_b64), actor_id: $row.actor_id, actor_kind: $row.actor_kind, session_id: $row.session_id, trace_id: $row.trace_id, state_vector_before: $row.state_vector_before, state_vector_after: $row.state_vector_after, replay_metadata_json: { replay_order_key: $row.replay_order_key, dependency_update_ids: $row.dependency_update_ids, encoding: $row.replay_encoding, schema_version: $row.replay_schema_version }, event_ledger_stream_id: $row.event_ledger_stream_id, event_ledger_event_id: type::record('kernel_event_ledger', $event.event_id), storage_authority: 'embedded_surrealdb' };
    LET $stored = (SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM type::record('kernel_crdt_updates', $record_id) WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND event_ledger_event_id.owner_account_id = $row.owner_account_id AND event_ledger_event_id.actor_principal_id = $row.actor_principal_id AND event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id AND event_ledger_event_id.access_space_id = $row.access_space_id AND event_ledger_event_id.workspace_id = $row.workspace_id);
    { outcome: 'stored', head_update_seq: $row.update_seq, head_state_vector: $row.state_vector_after, record: $stored[0] }
};
RETURN $result;
COMMIT TRANSACTION;
"#;
const APPEND_CRDT_SNAPSHOT_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT schema_id, snapshot_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq, state_vector, snapshot_sha256, snapshot_bytes_ref, encoding::base64::encode(snapshot_bytes) AS snapshot_bytes_b64, actor_id, actor_kind, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, promotion_evidence_update_ids, storage_authority, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.document_id AS ledger_document_id, event_ledger_event_id.payload.state_vector AS ledger_state_vector, event_ledger_event_id.payload.covered_update_seq AS ledger_covered_update_seq FROM kernel_crdt_snapshots WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id AND snapshot_id = $row.snapshot_id AND event_ledger_event_id.owner_account_id = $row.owner_account_id AND event_ledger_event_id.actor_principal_id = $row.actor_principal_id AND event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id AND event_ledger_event_id.access_space_id = $row.access_space_id AND event_ledger_event_id.workspace_id = $row.workspace_id LIMIT 2);
LET $evidence = (SELECT VALUE update_id FROM kernel_crdt_updates WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id AND update_id IN $row.promotion_evidence_update_ids AND update_seq <= $row.covered_update_seq AND event_ledger_event_id.owner_account_id = $row.owner_account_id AND event_ledger_event_id.actor_principal_id = $row.actor_principal_id AND event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id AND event_ledger_event_id.access_space_id = $row.access_space_id AND event_ledger_event_id.workspace_id = $row.workspace_id);
IF array::len($existing) > 1 {
    THROW 'model-lane CRDT snapshot identity is ambiguous';
} ELSE IF array::len($existing) = 1 {
    IF $existing[0].schema_id != $row.schema_id OR $existing[0].covered_update_seq != $row.covered_update_seq OR $existing[0].state_vector != $row.state_vector OR $existing[0].snapshot_sha256 != $row.snapshot_sha256 OR $existing[0].snapshot_bytes_ref != $row.snapshot_bytes_ref OR $existing[0].snapshot_bytes_b64 != $row.snapshot_bytes_b64 OR $existing[0].actor_id != $row.actor_id OR $existing[0].actor_kind != $row.actor_kind OR $existing[0].event_ledger_stream_id != $row.event_ledger_stream_id OR $existing[0].event_ledger_event_id != $row.event_ledger_event_id OR $existing[0].promotion_evidence_update_ids != $row.promotion_evidence_update_ids OR $existing[0].storage_authority != 'embedded_surrealdb' { THROW 'model-lane CRDT snapshot immutable identity conflict'; };
    RETURN $existing;
} ELSE {
    IF array::len($evidence) != array::len($row.promotion_evidence_update_ids) { THROW 'model-lane CRDT snapshot promotion evidence is unavailable in exact scope'; };
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT event exists without its snapshot'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
    LET $created = CREATE type::record('kernel_crdt_snapshots', $record_id) CONTENT { schema_id: $row.schema_id, snapshot_id: $row.snapshot_id, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id, document_id: $row.document_id, crdt_document_id: $row.crdt_document_id, covered_update_seq: $row.covered_update_seq, state_vector: $row.state_vector, snapshot_sha256: $row.snapshot_sha256, snapshot_bytes_ref: $row.snapshot_bytes_ref, snapshot_bytes: encoding::base64::decode($row.snapshot_bytes_b64), actor_id: $row.actor_id, actor_kind: $row.actor_kind, event_ledger_stream_id: $row.event_ledger_stream_id, event_ledger_event_id: type::record('kernel_event_ledger', $event.event_id), promotion_evidence_update_ids: $row.promotion_evidence_update_ids, storage_authority: 'embedded_surrealdb' };
    RETURN SELECT schema_id, snapshot_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq, state_vector, snapshot_sha256, snapshot_bytes_ref, encoding::base64::encode(snapshot_bytes) AS snapshot_bytes_b64, actor_id, actor_kind, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, promotion_evidence_update_ids, storage_authority, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.document_id AS ledger_document_id, event_ledger_event_id.payload.state_vector AS ledger_state_vector, event_ledger_event_id.payload.covered_update_seq AS ledger_covered_update_seq FROM type::record('kernel_crdt_snapshots', $record_id) WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND event_ledger_event_id.owner_account_id = $row.owner_account_id AND event_ledger_event_id.actor_principal_id = $row.actor_principal_id AND event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id AND event_ledger_event_id.access_space_id = $row.access_space_id AND event_ledger_event_id.workspace_id = $row.workspace_id;
};
COMMIT TRANSACTION;
"#;
const CLAIM_CRDT_LEASE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $row.lease_id AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND recorded_event_id.owner_account_id = $row.owner_account_id AND recorded_event_id.actor_principal_id = $row.actor_principal_id AND recorded_event_id.authenticated_session_id = $row.authenticated_session_id AND recorded_event_id.access_space_id = $row.access_space_id AND recorded_event_id.workspace_id = $row.workspace_id AND last_transition_event_id.owner_account_id = $row.owner_account_id AND last_transition_event_id.actor_principal_id = $row.actor_principal_id AND last_transition_event_id.authenticated_session_id = $row.authenticated_session_id AND last_transition_event_id.access_space_id = $row.access_space_id AND last_transition_event_id.workspace_id = $row.workspace_id LIMIT 2);
LET $holder = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND scope_kind = $row.scope_kind AND scope_id = $row.scope_id AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc IS NONE AND expired_at_utc IS NONE AND recorded_event_id.owner_account_id = $row.owner_account_id AND recorded_event_id.actor_principal_id = $row.actor_principal_id AND recorded_event_id.authenticated_session_id = $row.authenticated_session_id AND recorded_event_id.access_space_id = $row.access_space_id AND recorded_event_id.workspace_id = $row.workspace_id LIMIT 2);
IF array::len($existing) > 1 OR array::len($holder) > 1 { THROW 'model-lane CRDT lease identity is ambiguous'; } ELSE IF array::len($existing) = 1 {
    IF $existing[0].lane_id != $row.lane_id OR $existing[0].actor_id != $row.actor_id OR $existing[0].actor_kind != $row.actor_kind OR $existing[0].session_id != $row.session_id OR $existing[0].correlation_id != $row.correlation_id OR $existing[0].scope_kind != $row.scope_kind OR $existing[0].scope_id != $row.scope_id OR $existing[0].document_id != $row.document_id OR $existing[0].crdt_document_id != $row.crdt_document_id OR $existing[0].takeover_of != $row.takeover_of { THROW 'model-lane CRDT lease immutable identity conflict'; };
    RETURN [{ outcome: 'already_claimed', record: $existing[0] }];
} ELSE IF array::len($holder) = 1 {
    RETURN [{ outcome: 'scope_held', record: $holder[0] }];
} ELSE {
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT event exists without its lease'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
    LET $created = CREATE type::record('knowledge_crdt_agent_lane_leases', $record_id) CONTENT { lease_id: $row.lease_id, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id, document_id: $row.document_id, crdt_document_id: $row.crdt_document_id, lane_id: $row.lane_id, actor_id: $row.actor_id, actor_kind: $row.actor_kind, session_id: $row.session_id, correlation_id: $row.correlation_id, scope_kind: $row.scope_kind, scope_id: $row.scope_id, claimed_at_utc: time::now(), expires_at_utc: $expires_at_utc, renewal_count: 0, released_at_utc: NONE, expired_at_utc: NONE, takeover_of: IF $row.takeover_of = NONE { NONE } ELSE { type::record('knowledge_crdt_agent_lane_leases', $row.takeover_of) }, recorded_event_id: type::record('kernel_event_ledger', $event.event_id), last_transition_event_id: type::record('kernel_event_ledger', $event.event_id) };
    LET $stored = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM type::record('knowledge_crdt_agent_lane_leases', $record_id) WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id);
    RETURN [{ outcome: 'claimed', record: $stored[0] }];
};
COMMIT TRANSACTION;
"#;
const RENEW_CRDT_LEASE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $current = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc IS NONE AND expired_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
IF array::len($current) > 1 { THROW 'model-lane CRDT lease identity is ambiguous'; } ELSE IF array::len($current) = 0 { RETURN []; } ELSE IF $current[0].last_transition_event_id = $event.event_id { RETURN $current; } ELSE {
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT renewal event exists without lease transition'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $updated = UPDATE knowledge_crdt_agent_lane_leases SET expires_at_utc = $expires_at_utc, renewal_count += 1, last_transition_event_id = type::record('kernel_event_ledger', $event.event_id) WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
COMMIT TRANSACTION;
"#;
const RELEASE_CRDT_LEASE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $current = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND released_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
IF array::len($current) > 1 { THROW 'model-lane CRDT lease identity is ambiguous'; } ELSE IF array::len($current) = 0 { RETURN []; } ELSE IF $current[0].last_transition_event_id = $event.event_id { RETURN $current; } ELSE {
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT release event exists without lease transition'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $updated = UPDATE knowledge_crdt_agent_lane_leases SET released_at_utc = time::now(), last_transition_event_id = type::record('kernel_event_ledger', $event.event_id) WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
COMMIT TRANSACTION;
"#;
const RECORD_CRDT_PROPOSAL_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, record::id(lease_id) AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $row.proposal_id AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND recorded_event_id.owner_account_id = $row.owner_account_id AND recorded_event_id.actor_principal_id = $row.actor_principal_id AND recorded_event_id.authenticated_session_id = $row.authenticated_session_id AND recorded_event_id.access_space_id = $row.access_space_id AND recorded_event_id.workspace_id = $row.workspace_id AND last_transition_event_id.owner_account_id = $row.owner_account_id AND last_transition_event_id.actor_principal_id = $row.actor_principal_id AND last_transition_event_id.authenticated_session_id = $row.authenticated_session_id AND last_transition_event_id.access_space_id = $row.access_space_id AND last_transition_event_id.workspace_id = $row.workspace_id LIMIT 2);
LET $lease = (SELECT VALUE id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $row.lease_id AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND actor_id = $row.actor_id AND actor_kind = $row.actor_kind AND session_id = $row.session_id AND correlation_id = $row.correlation_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc IS NONE AND expired_at_utc IS NONE AND ((scope_kind = 'workspace' AND scope_id = $row.workspace_id) OR (scope_kind = 'document' AND scope_id = $row.crdt_document_id AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id)) AND recorded_event_id.owner_account_id = $row.owner_account_id AND recorded_event_id.actor_principal_id = $row.actor_principal_id AND recorded_event_id.authenticated_session_id = $row.authenticated_session_id AND recorded_event_id.access_space_id = $row.access_space_id AND recorded_event_id.workspace_id = $row.workspace_id AND last_transition_event_id.owner_account_id = $row.owner_account_id AND last_transition_event_id.actor_principal_id = $row.actor_principal_id AND last_transition_event_id.authenticated_session_id = $row.authenticated_session_id AND last_transition_event_id.access_space_id = $row.access_space_id AND last_transition_event_id.workspace_id = $row.workspace_id LIMIT 2);
LET $retry_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $event.event_id AND event_version = $event.event_version AND kernel_task_run_id = $event.kernel_task_run_id AND session_run_id = $event.session_run_id AND aggregate_type = $event.aggregate_type AND aggregate_id = $event.aggregate_id AND idempotency_key = $event.idempotency_key AND event_type = $event.event_type AND actor_kind = $event.actor_kind AND actor_id = $event.actor_id AND causation_id = $event.causation_id AND correlation_id = $event.correlation_id AND payload_hash = $event.payload_hash AND source_component = $event.source_component AND payload = $event.payload AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id LIMIT 2);
IF array::len($existing) > 1 { THROW 'model-lane CRDT proposal identity is ambiguous'; } ELSE IF array::len($existing) = 1 {
    IF $existing[0].document_id != $row.document_id OR $existing[0].crdt_document_id != $row.crdt_document_id OR $existing[0].base_update_seq != $row.base_update_seq OR $existing[0].base_state_vector != $row.base_state_vector OR $existing[0].proposed_diff != $row.proposed_diff OR $existing[0].diff_sha256 != $row.diff_sha256 OR $existing[0].source_span_citations != $row.source_span_citations OR $existing[0].actor_id != $row.actor_id OR $existing[0].actor_kind != $row.actor_kind OR $existing[0].session_id != $row.session_id OR $existing[0].correlation_id != $row.correlation_id OR $existing[0].lease_id != $row.lease_id OR $existing[0].recorded_event_id != $event.event_id OR array::len($retry_event) != 1 { THROW 'model-lane CRDT proposal immutable identity conflict'; };
    RETURN $existing;
} ELSE {
    IF array::len($lease) != 1 { THROW 'model-lane CRDT proposal lease is unavailable in exact scope'; };
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT event exists without its proposal'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
    LET $created = CREATE type::record('knowledge_crdt_ai_edit_proposals', $record_id) CONTENT { proposal_id: $row.proposal_id, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id, document_id: $row.document_id, crdt_document_id: $row.crdt_document_id, base_update_seq: $row.base_update_seq, base_state_vector: $row.base_state_vector, proposed_diff: $row.proposed_diff, diff_sha256: $row.diff_sha256, source_span_citations: $row.source_span_citations, actor_id: $row.actor_id, actor_kind: $row.actor_kind, session_id: $row.session_id, correlation_id: $row.correlation_id, lease_id: type::record('knowledge_crdt_agent_lane_leases', $row.lease_id), review_state: 'proposed', decided_by: NONE, decided_at_utc: NONE, decision_reason: NONE, recorded_event_id: type::record('kernel_event_ledger', $event.event_id), decided_event_id: NONE, promotion_requested_event_id: NONE, promotion_accepted_event_id: NONE, applied_update_id: NONE, applied_update_sha256: NONE, applied_event_id: NONE, last_transition_event_id: type::record('kernel_event_ledger', $event.event_id) };
    RETURN SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, record::id(lease_id) AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM type::record('knowledge_crdt_ai_edit_proposals', $record_id) WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id;
};
COMMIT TRANSACTION;
"#;
const DECIDE_CRDT_PROPOSAL_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $current = (SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, record::id(lease_id) AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
LET $retry_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $event.event_id AND event_version = $event.event_version AND kernel_task_run_id = $event.kernel_task_run_id AND session_run_id = $event.session_run_id AND aggregate_type = $event.aggregate_type AND aggregate_id = $event.aggregate_id AND idempotency_key = $event.idempotency_key AND event_type = $event.event_type AND actor_kind = $event.actor_kind AND actor_id = $event.actor_id AND causation_id = $event.causation_id AND correlation_id = $event.correlation_id AND payload_hash = $event.payload_hash AND source_component = $event.source_component AND payload = $event.payload AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
LET $retry_promotion_accepted = IF $review_state = 'promoted' { (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $promotion_accepted_event.event_id AND event_version = $promotion_accepted_event.event_version AND kernel_task_run_id = $promotion_accepted_event.kernel_task_run_id AND session_run_id = $promotion_accepted_event.session_run_id AND aggregate_type = $promotion_accepted_event.aggregate_type AND aggregate_id = $promotion_accepted_event.aggregate_id AND idempotency_key = $promotion_accepted_event.idempotency_key AND event_type = $promotion_accepted_event.event_type AND actor_kind = $promotion_accepted_event.actor_kind AND actor_id = $promotion_accepted_event.actor_id AND causation_id = $promotion_accepted_event.causation_id AND correlation_id = $promotion_accepted_event.correlation_id AND payload_hash = $promotion_accepted_event.payload_hash AND source_component = $promotion_accepted_event.source_component AND payload = $promotion_accepted_event.payload AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };
LET $identical_retry = IF $review_state = 'promoted' { $current[0].review_state = 'promoted' AND $current[0].promotion_requested_event_id = $event.event_id AND $current[0].promotion_accepted_event_id = $promotion_accepted_event.event_id AND $current[0].last_transition_event_id = $promotion_accepted_event.event_id AND array::len($retry_event) = 1 AND array::len($retry_promotion_accepted) = 1 } ELSE { $current[0].review_state = $review_state AND $current[0].last_transition_event_id = $event.event_id AND array::len($retry_event) = 1 };
IF array::len($current) > 1 { THROW 'model-lane CRDT proposal identity is ambiguous'; } ELSE IF array::len($current) = 0 { RETURN []; } ELSE IF $identical_retry { RETURN $current; } ELSE {
    LET $decision_allowed = $current[0].review_state = 'proposed' AND $review_state IN ['approved', 'rejected'];
    LET $promotion_allowed = $current[0].review_state = 'approved' AND $review_state = 'promoted' AND $current[0].applied_update_id != NONE AND $current[0].applied_update_sha256 = $current[0].diff_sha256 AND $current[0].applied_event_id != NONE;
    IF $decision_allowed = false AND $promotion_allowed = false { THROW 'model-lane CRDT proposal transition is not allowed'; };
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    LET $promotion_accepted_existing = IF $review_state = 'promoted' { (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $promotion_accepted_event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [] };
    IF array::len($event_existing) != 0 OR array::len($promotion_accepted_existing) != 0 { THROW 'model-lane CRDT decision event exists without proposal transition'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $accepted_ledger = IF $review_state = 'promoted' { CREATE type::record('kernel_event_ledger', $promotion_accepted_event.event_id) CONTENT { event_id: $promotion_accepted_event.event_id, event_version: $promotion_accepted_event.event_version, kernel_task_run_id: $promotion_accepted_event.kernel_task_run_id, session_run_id: $promotion_accepted_event.session_run_id, aggregate_type: $promotion_accepted_event.aggregate_type, aggregate_id: $promotion_accepted_event.aggregate_id, idempotency_key: $promotion_accepted_event.idempotency_key, event_type: $promotion_accepted_event.event_type, actor_kind: $promotion_accepted_event.actor_kind, actor_id: $promotion_accepted_event.actor_id, causation_id: $promotion_accepted_event.causation_id, correlation_id: $promotion_accepted_event.correlation_id, payload_hash: $promotion_accepted_event.payload_hash, source_component: $promotion_accepted_event.source_component, payload: $promotion_accepted_event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } } ELSE { [] };
    LET $updated = UPDATE knowledge_crdt_ai_edit_proposals SET review_state = $review_state, decided_by = IF $review_state = 'promoted' { decided_by } ELSE { $decided_by }, decided_at_utc = IF $review_state = 'promoted' { decided_at_utc } ELSE { time::now() }, decision_reason = IF $review_state = 'promoted' { decision_reason } ELSE { $decision_reason }, decided_event_id = IF $review_state = 'promoted' { decided_event_id } ELSE { type::record('kernel_event_ledger', $event.event_id) }, promotion_requested_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $event.event_id) } ELSE { promotion_requested_event_id }, promotion_accepted_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $promotion_accepted_event.event_id) } ELSE { promotion_accepted_event_id }, last_transition_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $promotion_accepted_event.event_id) } ELSE { type::record('kernel_event_ledger', $event.event_id) } WHERE proposal_id = $proposal_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, record::id(lease_id) AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
COMMIT TRANSACTION;
"#;
const BIND_CRDT_PROPOSAL_UPDATE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $proposal = (SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, record::id(lease_id) AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
LET $update = IF array::len($proposal) = 1 { (SELECT VALUE id FROM kernel_crdt_updates WHERE update_id = $applied_update_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND document_id = $proposal[0].document_id AND crdt_document_id = $proposal[0].crdt_document_id AND actor_id = $proposal[0].actor_id AND actor_kind = $proposal[0].actor_kind AND session_id = $proposal[0].session_id AND trace_id = $proposal[0].correlation_id AND update_seq = $proposal[0].base_update_seq + 1 AND state_vector_before = $proposal[0].base_state_vector AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.event_type = 'KNOWLEDGE_CRDT_UPDATE_RECORDED' AND event_ledger_event_id.aggregate_type = 'knowledge_crdt_document' AND event_ledger_event_id.aggregate_id = crdt_document_id AND event_ledger_event_id.actor_id = actor_id AND event_ledger_event_id.correlation_id = trace_id AND event_ledger_event_id.payload.update_id = update_id AND event_ledger_event_id.payload.update_sha256 = update_sha256 LIMIT 2) } ELSE { [] };
LET $retry_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $event.event_id AND event_version = $event.event_version AND kernel_task_run_id = $event.kernel_task_run_id AND session_run_id = $event.session_run_id AND aggregate_type = $event.aggregate_type AND aggregate_id = $event.aggregate_id AND idempotency_key = $event.idempotency_key AND event_type = $event.event_type AND actor_kind = $event.actor_kind AND actor_id = $event.actor_id AND causation_id = $event.causation_id AND correlation_id = $event.correlation_id AND payload_hash = $event.payload_hash AND source_component = $event.source_component AND payload = $event.payload AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
IF array::len($proposal) > 1 OR array::len($update) > 1 { THROW 'model-lane CRDT proposal application identity is ambiguous'; } ELSE IF array::len($proposal) = 0 { RETURN []; } ELSE IF $proposal[0].applied_update_id = $applied_update_id AND $proposal[0].applied_update_sha256 = $applied_update_sha256 AND $proposal[0].applied_event_id = $event.event_id AND $proposal[0].last_transition_event_id = $event.event_id AND array::len($retry_event) = 1 { RETURN $proposal; } ELSE {
    IF $proposal[0].review_state NOT IN ['approved', 'promoted'] OR $proposal[0].diff_sha256 != $applied_update_sha256 OR $expected_actor_id != $proposal[0].actor_id OR $event.actor_id != $expected_actor_id OR array::len($update) != 1 OR $proposal[0].applied_update_id != NONE { THROW 'model-lane CRDT proposal application authority is unavailable'; };
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT apply event exists without proposal binding'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $updated = UPDATE knowledge_crdt_ai_edit_proposals SET applied_update_id = $applied_update_id, applied_update_sha256 = $applied_update_sha256, applied_event_id = type::record('kernel_event_ledger', $event.event_id), last_transition_event_id = type::record('kernel_event_ledger', $event.event_id) WHERE proposal_id = $proposal_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, record::id(lease_id) AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
COMMIT TRANSACTION;
"#;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, SurrealValue)]
struct ModelLaneSchemaState {
    schema_version: String,
    schema_revision: i64,
    apply_state: String,
}

#[derive(Debug, SurrealValue)]
struct SchemaStateBindings {
    schema_version: String,
    schema_revision: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ModelLaneRecordKind {
    Run,
    Lane,
    Message,
    PromotionDecision,
    ContextArtifact,
    ContextHandoff,
    RecoveryCheckpoint,
    RecoveryEvent,
    Lease,
    DiagnosticTier,
    MtRuntimeStatus,
    SessionCleanupReceipt,
    SelectionAudit,
    RoutingExecution,
    CloudProjectionPlan,
    CloudConsentReceipt,
    CloudConsentDenial,
}

impl ModelLaneRecordKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Lane => "lane",
            Self::Message => "message",
            Self::PromotionDecision => "promotion_decision",
            Self::ContextArtifact => "context_artifact",
            Self::ContextHandoff => "context_handoff",
            Self::RecoveryCheckpoint => "recovery_checkpoint",
            Self::RecoveryEvent => "recovery_event",
            Self::Lease => "lease",
            Self::DiagnosticTier => "diagnostic_tier",
            Self::MtRuntimeStatus => "mt_runtime_status",
            Self::SessionCleanupReceipt => "session_cleanup_receipt",
            Self::SelectionAudit => "selection_audit",
            Self::RoutingExecution => "routing_execution",
            Self::CloudProjectionPlan => "cloud_projection_plan",
            Self::CloudConsentReceipt => "cloud_consent_receipt",
            Self::CloudConsentDenial => "cloud_consent_denial",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelLaneScope {
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingExecutionWrite {
    pub execution_id: String,
    pub run_id: String,
    pub revision: i64,
    pub context_hash: String,
    pub record_json: Value,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingAttemptWrite {
    pub attempt_id: String,
    pub execution_id: String,
    pub run_id: String,
    pub stage_id: String,
    pub attempt: i64,
    pub state: String,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<String>,
    pub lease_expires_at_unix_ms: Option<i64>,
    pub record_json: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingOutboxWrite {
    pub command_id: String,
    pub execution_id: String,
    pub run_id: String,
    pub stage_id: String,
    pub attempt: i64,
    pub status: String,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<String>,
    pub lease_expires_at_unix_ms: Option<i64>,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingClaim {
    pub stage_id: String,
    pub attempt: i64,
    pub lease_owner: String,
    pub fencing_token: String,
    pub observed_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingEventWrite {
    pub event_id: String,
    pub event_version: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub idempotency_key: String,
    pub event_type: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub payload_hash: String,
    pub source_component: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SurrealModelLaneRoutingCommit {
    pub expected_revision: i64,
    pub expected_claim: Option<SurrealModelLaneRoutingClaim>,
    pub execution: SurrealModelLaneRoutingExecutionWrite,
    pub attempt: SurrealModelLaneRoutingAttemptWrite,
    pub outbox: SurrealModelLaneRoutingOutboxWrite,
    pub events: Vec<SurrealModelLaneRoutingEventWrite>,
    pub message: Option<SurrealModelLaneWrite>,
    pub binding: Option<SurrealModelLaneWrite>,
    pub message_guard: Option<SurrealModelLaneMessageGuard>,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingExecutionRow {
    pub execution_id: String,
    pub run_id: String,
    pub revision: i64,
    pub context_hash: String,
    pub record_json: Value,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingAttemptRow {
    pub attempt_id: String,
    pub execution_id: String,
    pub run_id: String,
    pub stage_id: String,
    pub attempt: i64,
    pub state: String,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<String>,
    pub lease_expires_at_unix_ms: Option<i64>,
    pub record_json: Value,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneRoutingOutboxRow {
    pub command_id: String,
    pub execution_id: String,
    pub run_id: String,
    pub stage_id: String,
    pub attempt: i64,
    pub status: String,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<String>,
    pub lease_expires_at_unix_ms: Option<i64>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SurrealModelLaneRecord {
    pub aggregate_id: String,
    pub run_id: String,
    pub idempotency_key: String,
    pub record_json: String,
    pub event_id: String,
    pub event_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneScopedAuthorityReceipt {
    pub record_kind: String,
    pub aggregate_id: String,
    pub run_id: String,
    pub event_id: String,
    pub event_ledger_seq: i64,
    pub event_type: String,
    pub payload_hash: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SurrealModelLaneWrite {
    pub kind: ModelLaneRecordKind,
    pub aggregate_id: String,
    pub run_id: String,
    pub idempotency_key: String,
    pub record_json: String,
    pub search_terms: Vec<String>,
    pub event_payload_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtUpdate {
    pub schema_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub update_id: String,
    pub update_seq: i64,
    pub update_sha256: String,
    pub update_bytes_ref: String,
    pub update_bytes_b64: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub trace_id: String,
    pub state_vector_before: String,
    pub state_vector_after: String,
    pub replay_order_key: String,
    pub dependency_update_ids: Vec<String>,
    pub replay_encoding: String,
    pub replay_schema_version: String,
    pub event_ledger_stream_id: String,
    pub event_ledger_event_id: String,
    pub storage_authority: String,
    pub ledger_session_run_id: String,
    pub ledger_event_type: String,
    pub ledger_aggregate_type: String,
    pub ledger_aggregate_id: String,
    pub ledger_actor_kind: String,
    pub ledger_actor_id: String,
    pub ledger_correlation_id: Option<String>,
    pub ledger_payload_hash: String,
    pub ledger_update_id: String,
    pub ledger_update_seq: i64,
    pub ledger_actor_payload_id: String,
    pub ledger_update_sha256: String,
    pub ledger_state_vector_before: String,
    pub ledger_state_vector_after: String,
    pub ledger_site_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtSnapshot {
    pub schema_id: String,
    pub snapshot_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub covered_update_seq: i64,
    pub state_vector: String,
    pub snapshot_sha256: String,
    pub snapshot_bytes_ref: String,
    pub snapshot_bytes_b64: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub event_ledger_stream_id: String,
    pub event_ledger_event_id: String,
    pub promotion_evidence_update_ids: Vec<String>,
    pub storage_authority: String,
    pub ledger_event_type: String,
    pub ledger_aggregate_type: String,
    pub ledger_aggregate_id: String,
    pub ledger_actor_kind: String,
    pub ledger_actor_id: String,
    pub ledger_payload_hash: String,
    pub ledger_document_id: String,
    pub ledger_state_vector: String,
    pub ledger_covered_update_seq: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtLease {
    pub lease_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: Option<String>,
    pub crdt_document_id: Option<String>,
    pub lane_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub claimed_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub admitted_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtLeaseHistory {
    pub lease_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: Option<String>,
    pub crdt_document_id: Option<String>,
    pub lane_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub claimed_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub renewal_count: i64,
    pub released_at_utc: Option<DateTime<Utc>>,
    pub expired_at_utc: Option<DateTime<Utc>>,
    pub takeover_of: Option<String>,
    pub recorded_event_id: String,
    pub last_transition_event_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SurrealCrdtLeaseClaimOutcome {
    Claimed(SurrealModelLaneCrdtLeaseHistory),
    AlreadyClaimed(SurrealModelLaneCrdtLeaseHistory),
    ScopeHeld(SurrealModelLaneCrdtLeaseHistory),
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtLeaseWrite {
    pub lease_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: Option<String>,
    pub crdt_document_id: Option<String>,
    pub lane_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub takeover_of: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtProposal {
    pub proposal_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub review_state: String,
    pub diff_sha256: String,
    pub applied_update_id: Option<String>,
    pub applied_update_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtProposalWrite {
    pub proposal_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub base_update_seq: i64,
    pub base_state_vector: String,
    pub proposed_diff: Value,
    pub diff_sha256: String,
    pub source_span_citations: Vec<String>,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: String,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtProposalRecord {
    pub proposal_id: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub base_update_seq: i64,
    pub base_state_vector: String,
    pub proposed_diff: Value,
    pub diff_sha256: String,
    pub source_span_citations: Vec<String>,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: Option<String>,
    pub review_state: String,
    pub decided_by: Option<String>,
    pub decided_at_utc: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
    pub recorded_event_id: String,
    pub decided_event_id: Option<String>,
    pub promotion_requested_event_id: Option<String>,
    pub promotion_accepted_event_id: Option<String>,
    pub applied_update_id: Option<String>,
    pub applied_update_sha256: Option<String>,
    #[serde(default)]
    pub applied_event_id: Option<String>,
    pub last_transition_event_id: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtEventWrite {
    pub event_id: String,
    pub event_version: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub idempotency_key: String,
    pub event_type: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub payload_hash: String,
    pub source_component: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SurrealCrdtUpdateAppendOutcome {
    Stored(SurrealModelLaneCrdtUpdate),
    AlreadyStored(SurrealModelLaneCrdtUpdate),
    ContentMismatch { update_id: String },
    StaleHead {
        head_update_seq: i64,
        head_state_vector: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SurrealModelLaneMessageGuard {
    pub source_lane_id: String,
    pub source_lane_record_json: String,
    pub source_session_id: String,
    pub source_model_session_id: String,
    pub promotion_decision_id: Option<String>,
    pub promotion_record_json: Option<String>,
    pub crdt: Option<SurrealModelLaneCrdtGuard>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SurrealModelLaneCrdtGuard {
    pub update_ref: String,
    pub update_id: String,
    pub update_sha256: String,
    pub state_vector: String,
    pub snapshot_ref: String,
    pub snapshot_id: String,
    pub snapshot_sha256: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub trace_id: String,
    pub lease_id: String,
    pub lease_scope_kind: String,
    pub lease_scope_id: String,
    pub lease_claimed_at_utc: DateTime<Utc>,
    pub lease_expires_at_utc: DateTime<Utc>,
    pub lease_admitted_at_utc: DateTime<Utc>,
    pub proposal_id: Option<String>,
    pub proposal_diff_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SurrealModelLaneSchemaRow {
    pub schema_id: String,
    pub schema_version: i64,
    pub record_kind: String,
    pub table_name: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SurrealModelLaneStore {
    storage: SurrealStorage,
}

#[derive(Debug, SurrealValue)]
struct WriteBindings {
    record_id: String,
    event_id: String,
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    search_terms: Vec<String>,
    event_payload_json: String,
    event_payload_hash: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct CompareAndSwapBindings {
    record_id: String,
    event_id: String,
    record_kind: String,
    aggregate_id: String,
    expected_event_stream_version: i64,
    record_json: String,
    search_terms: Vec<String>,
    event_payload_json: String,
    event_payload_hash: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Clone, Debug, SurrealValue)]
struct RoutingAuthorityWriteBindings {
    record_id: String,
    event_id: String,
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    search_terms: Vec<String>,
    event_payload_json: String,
    event_payload_hash: String,
}

#[derive(Clone, Debug, SurrealValue)]
struct RoutingMessageGuardBindings {
    source_lane_record_id: String,
    source_lane_id: String,
    source_lane_record_json: String,
    source_session_term: String,
    source_model_session_term: String,
}

#[derive(Debug, SurrealValue)]
struct RoutingCommitBindings {
    expected_revision: i64,
    has_expected_claim: bool,
    expected_claim: SurrealModelLaneRoutingClaim,
    run_record_id: String,
    execution_record_id: String,
    attempt_record_id: String,
    outbox_record_id: String,
    execution: SurrealModelLaneRoutingExecutionWrite,
    attempt: SurrealModelLaneRoutingAttemptWrite,
    outbox: SurrealModelLaneRoutingOutboxWrite,
    execution_event: SurrealModelLaneRoutingEventWrite,
    attempt_event: SurrealModelLaneRoutingEventWrite,
    outbox_event: SurrealModelLaneRoutingEventWrite,
    extra_events: Vec<SurrealModelLaneRoutingEventWrite>,
    has_message: bool,
    message: RoutingAuthorityWriteBindings,
    has_binding: bool,
    binding: RoutingAuthorityWriteBindings,
    message_guard: RoutingMessageGuardBindings,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Debug, SurrealValue)]
struct RoutingTestCorruptionBindings {
    corruption: String,
    execution_record_id: String,
    attempt_record_id: String,
    outbox_record_id: String,
    execution_id: String,
    attempt_id: String,
    outbox_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Debug, SurrealValue)]
struct CrdtProposalTestCorruptionBindings {
    proposal_id: String,
    update_id: String,
    corruption: String,
    tampered_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Debug, SurrealValue)]
struct AuthorityTestCorruptionBindings {
    record_id: String,
    record_kind: String,
    aggregate_id: String,
    corruption: String,
    tampered_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct SurrealModelLaneCrdtAuthorityCounts {
    pub proposal_rows: i64,
    pub update_rows: i64,
    pub snapshot_rows: i64,
    pub lease_rows: i64,
    pub event_rows: i64,
}

#[derive(Debug, SurrealValue)]
struct RecoveryCheckpointWriteBindings {
    record_id: String,
    run_record_id: String,
    lane_record_id: String,
    has_lane: bool,
    lane_id: String,
    event_id: String,
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    search_terms: Vec<String>,
    event_payload_json: String,
    event_payload_hash: String,
    expected_last_event_seq: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct RecoveryEventWriteBindings {
    record_id: String,
    run_record_id: String,
    lane_record_id: String,
    order_record_id: String,
    has_lane: bool,
    lane_id: String,
    has_source_event: bool,
    source_event_seq: i64,
    expected_replay_order_seq: i64,
    event_id: String,
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    search_terms: Vec<String>,
    event_payload_json: String,
    event_payload_hash: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct PairWriteBindings {
    first_record_id: String,
    first_event_id: String,
    first_record_kind: String,
    first_aggregate_id: String,
    first_run_id: String,
    first_idempotency_key: String,
    first_record_json: String,
    first_search_terms: Vec<String>,
    first_event_payload_json: String,
    first_event_payload_hash: String,
    second_record_id: String,
    second_event_id: String,
    second_record_kind: String,
    second_aggregate_id: String,
    second_run_id: String,
    second_idempotency_key: String,
    second_record_json: String,
    second_search_terms: Vec<String>,
    second_event_payload_json: String,
    second_event_payload_hash: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct GuardedMessageBindings {
    message_record_id: String,
    message_event_id: String,
    message_aggregate_id: String,
    message_run_id: String,
    message_idempotency_key: String,
    message_record_json: String,
    message_search_terms: Vec<String>,
    message_event_payload_json: String,
    message_event_payload_hash: String,
    has_payload_binding: bool,
    binding_record_id: String,
    binding_event_id: String,
    binding_record_kind: String,
    binding_aggregate_id: String,
    binding_run_id: String,
    binding_idempotency_key: String,
    binding_record_json: String,
    binding_search_terms: Vec<String>,
    binding_event_payload_json: String,
    binding_event_payload_hash: String,
    source_lane_record_id: String,
    source_lane_id: String,
    source_lane_record_json: String,
    source_session_term: String,
    source_model_session_term: String,
    has_promotion: bool,
    promotion_record_id: String,
    promotion_record_json: String,
    has_crdt: bool,
    crdt_update_ref: String,
    crdt_update_id: String,
    crdt_update_sha256: String,
    crdt_state_vector: String,
    crdt_snapshot_ref: String,
    crdt_snapshot_id: String,
    crdt_snapshot_sha256: String,
    crdt_document_id: String,
    crdt_crdt_document_id: String,
    crdt_actor_id: String,
    crdt_actor_kind: String,
    crdt_session_id: String,
    crdt_trace_id: String,
    crdt_lease_id: String,
    crdt_lease_scope_kind: String,
    crdt_lease_scope_id: String,
    crdt_lease_claimed_at_utc: DateTime<Utc>,
    crdt_lease_expires_at_utc: DateTime<Utc>,
    crdt_lease_admitted_at_utc: DateTime<Utc>,
    has_crdt_proposal: bool,
    crdt_proposal_id: String,
    crdt_proposal_diff_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ScopedBindings {
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    search_term: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Debug, SurrealValue)]
struct ScopedLimitBindings {
    record_kind: String,
    run_id: String,
    row_limit: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct CrdtReferenceBindings {
    authority_ref: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Debug, SurrealValue)]
struct CrdtScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct CrdtChainBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    covered_update_seq: i64,
    update_seq: i64,
}

#[derive(Debug, SurrealValue)]
struct CrdtLeaseBindings {
    lane_id: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    correlation_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
}

#[derive(Debug, SurrealValue)]
struct CrdtLeaseHistoryBindings {
    authority_ref: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
}

#[derive(Debug, SurrealValue)]
struct CrdtUpdateWriteBindings {
    record_id: String,
    expected_head_update_seq: i64,
    expected_head_state_vector: String,
    row: SurrealModelLaneCrdtUpdate,
    event: SurrealModelLaneCrdtEventWrite,
}

#[derive(Debug, SurrealValue)]
struct CrdtUpdateMutationResult {
    outcome: String,
    head_update_seq: i64,
    head_state_vector: String,
    record: Option<SurrealModelLaneCrdtUpdate>,
}

#[derive(Debug, SurrealValue)]
struct CrdtSnapshotWriteBindings {
    record_id: String,
    row: SurrealModelLaneCrdtSnapshot,
    event: SurrealModelLaneCrdtEventWrite,
}

#[derive(Debug, SurrealValue)]
struct CrdtLeaseClaimBindings {
    record_id: String,
    expires_at_utc: DateTime<Utc>,
    row: SurrealModelLaneCrdtLeaseWrite,
    event: SurrealModelLaneCrdtEventWrite,
}

#[derive(Debug, SurrealValue)]
struct CrdtLeaseMutationBindings {
    lease_id: String,
    actor_id: String,
    transition: String,
    expires_at_utc: DateTime<Utc>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    event: SurrealModelLaneCrdtEventWrite,
}

#[derive(Debug, SurrealValue)]
struct CrdtLeaseClaimResult {
    outcome: String,
    record: SurrealModelLaneCrdtLeaseHistory,
}

#[derive(Debug, SurrealValue)]
struct CrdtProposalWriteBindings {
    record_id: String,
    row: SurrealModelLaneCrdtProposalWrite,
    event: SurrealModelLaneCrdtEventWrite,
}

#[derive(Debug, SurrealValue)]
struct CrdtProposalDecisionBindings {
    proposal_id: String,
    review_state: String,
    decided_by: String,
    decision_reason: Option<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    event: SurrealModelLaneCrdtEventWrite,
    promotion_accepted_event: Option<SurrealModelLaneCrdtEventWrite>,
}

#[derive(Debug, SurrealValue)]
struct CrdtProposalApplyBindings {
    proposal_id: String,
    expected_actor_id: String,
    applied_update_id: String,
    applied_update_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    event: SurrealModelLaneCrdtEventWrite,
}

#[derive(Debug, SurrealValue)]
struct CrdtProposalReceiptBindings {
    event_ids: Vec<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Clone, Debug, SurrealValue)]
struct CrdtProposalReceiptRow {
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
    payload: Value,
}

#[derive(Debug, SurrealValue)]
struct CrdtUpdateIdRow {
    update_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredRow {
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    event_id: String,
    event_seq: i64,
    event_stream_version: i64,
    transaction_seq: i64,
}

impl From<StoredRow> for SurrealModelLaneRecord {
    fn from(value: StoredRow) -> Self {
        Self {
            aggregate_id: value.aggregate_id,
            run_id: value.run_id,
            idempotency_key: value.idempotency_key,
            record_json: value.record_json,
            event_id: value.event_id,
            event_seq: value.event_seq,
            event_stream_version: value.event_stream_version,
            transaction_seq: value.transaction_seq,
        }
    }
}

#[derive(Debug, SurrealValue)]
struct StoredSchemaRow {
    schema_id: String,
    schema_version: i64,
    record_kind: String,
    table_name: String,
}

#[derive(Debug, SurrealValue)]
struct StoredSchemaWrite {
    schema_id: String,
    schema_version: i64,
    record_kind: String,
    table_name: String,
    source_component: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct EventLinkBindings {
    event_id: String,
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredEventLink {
    event_id: String,
    event_seq: i64,
    event_stream_version: i64,
    transaction_seq: i64,
    aggregate_id: String,
    run_id: String,
    payload_hash: String,
    event_payload_json: String,
}

#[derive(Debug, SurrealValue)]
struct RunEventBindings {
    run_id: String,
    event_seq: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredEventSequence {
    event_seq: i64,
}

#[derive(Debug, SurrealValue)]
struct ProcessClosureBindings {
    process_uuid: uuid::Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredProcessClosure {
    stopped_at: Option<DateTime<Utc>>,
    exit_code: Option<i64>,
    stop_reason: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct StoredRecoveryOrder {
    next_value: i64,
}

const CANONICAL_MODEL_LANE_SCHEMAS: &[(&str, i64)] = &[
    ("hsk.model_lane_run@1", 1),
    ("hsk.model_lane@1", 1),
    ("hsk.model_lane_message@1", 1),
    ("hsk.model_lane_terminal@1", 1),
    ("hsk.model_lane_promotion_decision@1", 1),
    ("hsk.model_lane_context_bundle_artifact@1", 1),
    ("hsk.model_lane_context_bundle_handoff@1", 1),
    ("hsk.model_lane_cloud_projection_plan@1", 1),
    ("hsk.model_lane_cloud_consent_receipt@1", 1),
    ("hsk.model_lane_cloud_consent_denial@1", 1),
    ("hsk.model_lane_recovery_checkpoint@1", 1),
    ("hsk.model_lane_recovery_event@1", 1),
    ("hsk.model_lane_lease@1", 1),
    ("hsk.model_lane_diagnostic_tier@1", 1),
    ("hsk.model_lane_mt_runtime_status@1", 1),
    ("hsk.swarm_session_cleanup_receipt@1", 1),
    ("hsk.model_lane_selection_audit@1", 1),
    ("hsk.model_lane_cloud_projection_plan@2", 2),
    ("hsk.model_lane_cloud_consent_receipt@2", 2),
    ("hsk.model_lane_recovery_event@2", 2),
    ("hsk.model_lane_routing_execution@5", 5),
    ("hsk.model_lane_routing_outbox@4", 4),
    ("hsk.model_lane_routing_stage_attempt@4", 4),
    ("hsk.model_lane_run_extension@1", 1),
];

impl From<StoredSchemaRow> for SurrealModelLaneSchemaRow {
    fn from(value: StoredSchemaRow) -> Self {
        Self {
            schema_id: value.schema_id,
            schema_version: value.schema_version,
            record_kind: value.record_kind,
            table_name: value.table_name,
        }
    }
}

pub(crate) async fn bootstrap_model_lane_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                database.query(SCHEMA_STATE).await?;
                let mut response = database
                    .query(format!("SELECT * FROM ONLY {SCHEMA_STATE_ID};"))
                    .await?;
                let state: Option<ModelLaneSchemaState> = response.take(0)?;
                let advance_schema_state = match state.as_ref() {
                    None => true,
                    Some(state)
                        if state.schema_version == SCHEMA_VERSION
                            && state.schema_revision == SCHEMA_REVISION
                            && state.apply_state == "complete" =>
                    {
                        false
                    }
                    Some(state)
                        if ((state.schema_version == PREVIOUS_SCHEMA_VERSION
                            && state.schema_revision == PREVIOUS_SCHEMA_REVISION)
                            || (state.schema_version == LEGACY_SCHEMA_VERSION
                                && state.schema_revision == LEGACY_SCHEMA_REVISION)
                            || (state.schema_version == OLDER_SCHEMA_VERSION
                                && state.schema_revision == OLDER_SCHEMA_REVISION)
                            || (state.schema_version == OLDEST_SCHEMA_VERSION
                                && state.schema_revision == OLDEST_SCHEMA_REVISION))
                            && state.apply_state == "complete" =>
                    {
                        true
                    }
                    Some(_) => {
                        return Err(SurrealStorageError::InvalidModelLaneRecord {
                            reason: "model-lane schema state version/revision mismatch",
                        });
                    }
                };
                database.query(SCHEMA).await?;
                if advance_schema_state {
                    database
                        .query_bound(
                            "UPSERT model_lane_schema_state:primary CONTENT { schema_version: $schema_version, schema_revision: $schema_revision, apply_state: 'complete' };",
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

impl SurrealModelLaneStore {
    pub(crate) async fn initialize(storage: SurrealStorage) -> Result<Self, SurrealStorageError> {
        bootstrap_model_lane_schema(&storage).await?;
        Ok(Self { storage })
    }

    pub(crate) fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub(crate) async fn routing_execution_snapshot(
        &self,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRoutingExecutionRow>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(execution_id, execution_id)?;
        let rows = self
            .storage
            .with_data_operation(|database| {
                let bindings = routing_identity_bindings(execution_id, scope);
                Box::pin(async move {
                    database
                        .query_values::<SurrealModelLaneRoutingExecutionRow, _>(
                            "SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM model_lane_routing_execution WHERE execution_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $aggregate_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        single_routing_row(rows, "routing execution identity is ambiguous")
    }

    pub(crate) async fn routing_executions_for_run(
        &self,
        run_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingExecutionRow>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(run_id, run_id)?;
        let bindings = routing_identity_bindings(run_id, scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<SurrealModelLaneRoutingExecutionRow, _>(
                            "SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM model_lane_routing_execution WHERE run_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = execution_id AND event_ledger_seq = event_ledger_event_id.event_sequence ORDER BY event_ledger_seq ASC, execution_id ASC LIMIT 4097;",
                            bindings,
                        )
                        .await
                })
            })
            .await
    }

    pub(crate) async fn routing_attempts_for_execution(
        &self,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingAttemptRow>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(execution_id, execution_id)?;
        let bindings = routing_identity_bindings(execution_id, scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<SurrealModelLaneRoutingAttemptRow, _>(
                            "SELECT attempt_id, execution_id, run_id, stage_id, attempt, state, lease_owner, fencing_token, lease_expires_at_unix_ms, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM model_lane_routing_stage_attempt WHERE execution_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_stage_attempt' AND event_ledger_event_id.aggregate_id = attempt_id AND event_ledger_seq = event_ledger_event_id.event_sequence ORDER BY stage_id ASC, attempt ASC LIMIT 4097;",
                            bindings,
                        )
                        .await
                })
            })
            .await
    }

    pub(crate) async fn routing_outbox_for_execution(
        &self,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingOutboxRow>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(execution_id, execution_id)?;
        let bindings = routing_identity_bindings(execution_id, scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<SurrealModelLaneRoutingOutboxRow, _>(
                            "SELECT command_id, execution_id, run_id, stage_id, attempt, status, lease_owner, fencing_token, lease_expires_at_unix_ms, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq, created_at_unix_ms, updated_at_unix_ms FROM model_lane_routing_outbox WHERE execution_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_outbox' AND event_ledger_event_id.aggregate_id = command_id AND event_ledger_seq = event_ledger_event_id.event_sequence ORDER BY stage_id ASC, attempt ASC LIMIT 4097;",
                            bindings,
                        )
                        .await
                })
            })
            .await
    }

    pub(crate) async fn commit_routing_atomic(
        &self,
        commit: SurrealModelLaneRoutingCommit,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRoutingExecutionRow, SurrealStorageError> {
        validate_routing_commit(&commit, scope)?;
        let bindings = routing_commit_bindings(commit, scope)?;
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<SurrealModelLaneRoutingExecutionRow, _>(
                            ROUTING_COMMIT_QUERY,
                            bindings,
                            11,
                        )
                        .await
                })
            })
            .await?;
        single_routing_row(rows, "atomic routing commit returned ambiguous authority")?.ok_or(
            SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic routing commit returned no execution authority",
            },
        )
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn test_corrupt_routing_authority(
        &self,
        execution_id: &str,
        stage_id: &str,
        attempt: i64,
        corruption: &str,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(execution_id, execution_id)?;
        validate_identity(stage_id, stage_id)?;
        if attempt <= 0
            || !matches!(
                corruption,
                "attempt_event_aggregate_type"
                    | "attempt_event_aggregate_id"
                    | "execution_event_sequence"
                    | "attempt_event_sequence"
                    | "outbox_event_sequence"
            )
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing test corruption selector is invalid",
            });
        }
        let attempt_id = format!("{execution_id}:{stage_id}:{attempt}");
        let outbox_id = format!("routing-command:{execution_id}:{stage_id}:{attempt}");
        let bindings = RoutingTestCorruptionBindings {
            corruption: corruption.to_owned(),
            execution_record_id: stable_record_id("routing_execution", execution_id, scope),
            attempt_record_id: stable_record_id("routing_attempt", &attempt_id, scope),
            outbox_record_id: stable_record_id("routing_outbox", &outbox_id, scope),
            execution_id: execution_id.to_owned(),
            attempt_id,
            outbox_id,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(ROUTING_TEST_CORRUPTION_QUERY, bindings)
                        .await
                })
            })
            .await
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn test_corrupt_scoped_authority(
        &self,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        corruption: &str,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(aggregate_id, aggregate_id)?;
        if !matches!(
            corruption,
            "projection_event_sequence"
                | "projection_scope"
                | "receipt_payload_hash"
                | "receipt_scope"
        ) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "ModelLane authority test corruption selector is invalid",
            });
        }
        if self.get(kind, aggregate_id, scope).await?.is_none() {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "ModelLane authority test corruption target is unavailable",
            });
        }
        let bindings = AuthorityTestCorruptionBindings {
            record_id: stable_record_id(kind.as_str(), aggregate_id, scope),
            record_kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            corruption: corruption.to_owned(),
            tampered_sha256: "0".repeat(64),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(AUTHORITY_TEST_CORRUPTION_QUERY, bindings)
                        .await
                })
            })
            .await
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn test_crdt_authority_counts(
        &self,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtAuthorityCounts, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtScopeBindings {
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<SurrealModelLaneCrdtAuthorityCounts, _>(
                            "SELECT array::len((SELECT VALUE id FROM knowledge_crdt_ai_edit_proposals WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id)) AS proposal_rows, array::len((SELECT VALUE id FROM kernel_crdt_updates WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id)) AS update_rows, array::len((SELECT VALUE id FROM kernel_crdt_snapshots WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id)) AS snapshot_rows, array::len((SELECT VALUE id FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id)) AS lease_rows, array::len((SELECT VALUE id FROM kernel_event_ledger WHERE source_component = 'model_lane_crdt' AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id)) AS event_rows FROM ONLY model_lane_schema_state:primary;",
                            bindings,
                        )
                        .await
                })
            })
            .await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT authority count inspection returned no scoped projection",
            })
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn test_corrupt_crdt_proposal_authority(
        &self,
        proposal_id: &str,
        update_id: &str,
        corruption: &str,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(proposal_id, proposal_id)?;
        validate_identity(update_id, update_id)?;
        if !matches!(
            corruption,
            "recorded_receipt_aggregate"
                | "applied_receipt_payload_hash"
                | "proposal_diff_hash"
                | "update_content_hash"
                | "proposal_incomplete_attribution"
                | "applied_receipt_mixed_scope"
        ) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal test corruption selector is invalid",
            });
        }
        let bindings = CrdtProposalTestCorruptionBindings {
            proposal_id: proposal_id.to_owned(),
            update_id: update_id.to_owned(),
            corruption: corruption.to_owned(),
            tampered_sha256: "0".repeat(64),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(CRDT_PROPOSAL_TEST_CORRUPTION_QUERY, bindings)
                        .await
                })
            })
            .await
    }

    pub(crate) async fn append_crdt_update_atomic(
        &self,
        expected_head_update_seq: i64,
        expected_head_state_vector: &str,
        row: SurrealModelLaneCrdtUpdate,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<SurrealCrdtUpdateAppendOutcome, SurrealStorageError> {
        validate_crdt_update_write(&row, &event, scope)?;
        let update_id = row.update_id.clone();
        let bindings = CrdtUpdateWriteBindings {
            record_id: stable_record_id("crdt_update", &update_id, scope),
            expected_head_update_seq,
            expected_head_state_vector: expected_head_state_vector.to_owned(),
            row,
            event,
        };
        let mut results = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<CrdtUpdateMutationResult, _>(
                            APPEND_CRDT_UPDATE_QUERY,
                            bindings,
                            6,
                        )
                        .await
                })
            })
            .await?;
        let result = results
            .pop()
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic CRDT update transaction returned no outcome",
            })?;
        match result.outcome.as_str() {
            "stored" => Ok(SurrealCrdtUpdateAppendOutcome::Stored(
                result
                    .record
                    .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                        reason: "atomic CRDT update stored outcome returned no row",
                    })?,
            )),
            "already_stored" => Ok(SurrealCrdtUpdateAppendOutcome::AlreadyStored(
                result
                    .record
                    .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                        reason: "atomic CRDT update retry returned no row",
                    })?,
            )),
            "content_mismatch" => Ok(SurrealCrdtUpdateAppendOutcome::ContentMismatch {
                update_id,
            }),
            "stale_head" => Ok(SurrealCrdtUpdateAppendOutcome::StaleHead {
                head_update_seq: result.head_update_seq,
                head_state_vector: result.head_state_vector,
            }),
            _ => Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic CRDT update transaction returned an unknown outcome",
            }),
        }
    }

    pub(crate) async fn append_crdt_snapshot_atomic(
        &self,
        row: SurrealModelLaneCrdtSnapshot,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtSnapshot, SurrealStorageError> {
        validate_crdt_snapshot_write(&row, &event, scope)?;
        let bindings = CrdtSnapshotWriteBindings {
            record_id: stable_record_id("crdt_snapshot", &row.snapshot_id, scope),
            row,
            event,
        };
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<SurrealModelLaneCrdtSnapshot, _>(
                            APPEND_CRDT_SNAPSHOT_QUERY,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        rows.pop()
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic CRDT snapshot transaction returned no row",
            })
    }

    pub(crate) async fn claim_crdt_lease_atomic(
        &self,
        row: SurrealModelLaneCrdtLeaseWrite,
        ttl_seconds: i64,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<SurrealCrdtLeaseClaimOutcome, SurrealStorageError> {
        validate_crdt_lease_write(&row, ttl_seconds, &event, scope)?;
        let bindings = CrdtLeaseClaimBindings {
            record_id: stable_record_id("crdt_lease", &row.lease_id, scope),
            expires_at_utc: Utc::now() + chrono::Duration::seconds(ttl_seconds),
            row,
            event,
        };
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<CrdtLeaseClaimResult, _>(
                            CLAIM_CRDT_LEASE_QUERY,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        let result = rows
            .pop()
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic CRDT lease claim returned no outcome",
            })?;
        match result.outcome.as_str() {
            "claimed" => Ok(SurrealCrdtLeaseClaimOutcome::Claimed(result.record)),
            "already_claimed" => Ok(SurrealCrdtLeaseClaimOutcome::AlreadyClaimed(result.record)),
            "scope_held" => Ok(SurrealCrdtLeaseClaimOutcome::ScopeHeld(result.record)),
            _ => Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic CRDT lease claim returned an unknown outcome",
            }),
        }
    }

    pub(crate) async fn renew_crdt_lease_atomic(
        &self,
        lease_id: &str,
        actor_id: &str,
        ttl_seconds: i64,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_crdt_transition_event(lease_id, actor_id, ttl_seconds, &event)?;
        validate_crdt_event_kind(
            &event,
            "knowledge_crdt_lease",
            "KNOWLEDGE_CRDT_LEASE_RENEWED",
        )?;
        self.mutate_crdt_lease(
            RENEW_CRDT_LEASE_QUERY,
            "renew",
            lease_id,
            actor_id,
            ttl_seconds,
            event,
            scope,
        )
        .await
    }

    pub(crate) async fn release_crdt_lease_atomic(
        &self,
        lease_id: &str,
        actor_id: &str,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_crdt_transition_event(lease_id, actor_id, 1, &event)?;
        validate_crdt_event_kind(
            &event,
            "knowledge_crdt_lease",
            "KNOWLEDGE_CRDT_LEASE_RELEASED",
        )?;
        self.mutate_crdt_lease(
            RELEASE_CRDT_LEASE_QUERY,
            "release",
            lease_id,
            actor_id,
            1,
            event,
            scope,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn mutate_crdt_lease(
        &self,
        statement: &'static str,
        transition: &str,
        lease_id: &str,
        actor_id: &str,
        ttl_seconds: i64,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        let bindings = CrdtLeaseMutationBindings {
            lease_id: lease_id.to_owned(),
            actor_id: actor_id.to_owned(),
            transition: transition.to_owned(),
            expires_at_utc: Utc::now() + chrono::Duration::seconds(ttl_seconds),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            event,
        };
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<SurrealModelLaneCrdtLeaseHistory, _>(
                            statement, bindings, 2,
                        )
                        .await
                })
            })
            .await?;
        Ok(rows.pop())
    }

    pub(crate) async fn record_crdt_proposal_atomic(
        &self,
        row: SurrealModelLaneCrdtProposalWrite,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtProposalRecord, SurrealStorageError> {
        validate_crdt_proposal_write(&row, &event, scope)?;
        let bindings = CrdtProposalWriteBindings {
            record_id: stable_record_id("crdt_proposal", &row.proposal_id, scope),
            row,
            event,
        };
        let stored = self
            .query_crdt_proposal_mutation(RECORD_CRDT_PROPOSAL_QUERY, bindings, 3)
            .await?;
        self.validate_crdt_proposal_receipts(&stored, scope).await?;
        Ok(stored)
    }

    pub(crate) async fn decide_crdt_proposal_atomic(
        &self,
        proposal_id: &str,
        review_state: &str,
        decided_by: &str,
        decision_reason: Option<String>,
        event: SurrealModelLaneCrdtEventWrite,
        promotion_accepted_event: Option<SurrealModelLaneCrdtEventWrite>,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        if !matches!(review_state, "approved" | "rejected" | "promoted") {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal decision state is invalid",
            });
        }
        if review_state == "promoted" {
            let accepted = promotion_accepted_event.as_ref().ok_or(
                SurrealStorageError::InvalidModelLaneRecord {
                    reason: "CRDT proposal promotion requires an atomic requested/accepted receipt pair",
                },
            )?;
            validate_crdt_promotion_event_pair(proposal_id, decided_by, &event, accepted)?;
        } else {
            if promotion_accepted_event.is_some() {
                return Err(SurrealStorageError::InvalidModelLaneRecord {
                    reason: "CRDT proposal review decision cannot carry promotion receipts",
                });
            }
            validate_crdt_transition_event(proposal_id, decided_by, 1, &event)?;
            validate_crdt_event_kind(
                &event,
                "knowledge_crdt_ai_edit_proposal",
                "AI_EDIT_PROPOSAL_DECIDED",
            )?;
        }
        if self
            .crdt_proposal_record(proposal_id, scope)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let bindings = CrdtProposalDecisionBindings {
            proposal_id: proposal_id.to_owned(),
            review_state: review_state.to_owned(),
            decided_by: decided_by.to_owned(),
            decision_reason,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            event,
            promotion_accepted_event,
        };
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<SurrealModelLaneCrdtProposalRecord, _>(
                            DECIDE_CRDT_PROPOSAL_QUERY,
                            bindings,
                            2,
                        )
                        .await
                })
            })
            .await?;
        let row = rows.pop();
        if let Some(row) = row.as_ref() {
            self.validate_crdt_proposal_receipts(row, scope).await?;
        }
        Ok(row)
    }

    pub(crate) async fn bind_crdt_proposal_update_atomic(
        &self,
        proposal_id: &str,
        applied_update_id: &str,
        applied_update_sha256: &str,
        actor_id: &str,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_crdt_transition_event(proposal_id, actor_id, 1, &event)?;
        validate_crdt_event_kind(
            &event,
            "knowledge_crdt_ai_edit_proposal",
            "AI_EDIT_PROPOSAL_DECIDED",
        )?;
        if applied_update_id.trim().is_empty() || !is_sha256_hex(applied_update_sha256) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal applied-update binding is invalid",
            });
        }
        if self.crdt_proposal_record(proposal_id, scope).await?.is_none() {
            return Ok(None);
        }
        let bindings = CrdtProposalApplyBindings {
            proposal_id: proposal_id.to_owned(),
            expected_actor_id: actor_id.to_owned(),
            applied_update_id: applied_update_id.to_owned(),
            applied_update_sha256: applied_update_sha256.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            event,
        };
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<SurrealModelLaneCrdtProposalRecord, _>(
                            BIND_CRDT_PROPOSAL_UPDATE_QUERY,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        let row = rows.pop();
        if let Some(row) = row.as_ref() {
            self.validate_crdt_proposal_receipts(row, scope).await?;
        }
        Ok(row)
    }

    async fn query_crdt_proposal_mutation<B: SurrealValue + Send + 'static>(
        &self,
        statement: &'static str,
        bindings: B,
        statement_index: usize,
    ) -> Result<SurrealModelLaneCrdtProposalRecord, SurrealStorageError> {
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<SurrealModelLaneCrdtProposalRecord, _>(
                            statement,
                            bindings,
                            statement_index,
                        )
                        .await
                })
            })
            .await?;
        rows.pop()
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic CRDT proposal transaction returned no row",
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn put_immutable(
        &self,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        run_id: &str,
        idempotency_key: &str,
        record_json: String,
        search_terms: Vec<String>,
        event_payload_json: String,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRecord, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(aggregate_id, idempotency_key)?;
        let expected_record_json = record_json.clone();
        let event_payload_hash = sha256_hex(event_payload_json.as_bytes());
        let bindings = WriteBindings {
            record_id: stable_record_id(kind.as_str(), aggregate_id, scope),
            event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            record_kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            run_id: run_id.to_owned(),
            idempotency_key: idempotency_key.to_owned(),
            record_json,
            search_terms,
            event_payload_json,
            event_payload_hash,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredRow, _>(
                "BEGIN TRANSACTION;\
                 LET $existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
                 IF array::len($existing) > 0 { RETURN $existing; } ELSE {\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: $run_id, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $run_id, event_stream_version: 1, event_payload_json: $event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                   LET $seq = $ledger[0].event_sequence;\
                   RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: $record_kind, aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $record_json, search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                 };\
                 COMMIT TRANSACTION;",
                bindings,
                2,
            ).await
        })).await?;
        let row: SurrealModelLaneRecord = rows.into_iter().next().map(Into::into).ok_or(
            SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane authority write returned no row",
            },
        )?;
        if row.aggregate_id != aggregate_id
            || row.run_id != run_id
            || row.idempotency_key != idempotency_key
            || row.record_json != expected_record_json
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane immutable identity or idempotency conflict",
            });
        }
        Ok(row)
    }

    pub(crate) async fn put_recovery_checkpoint_atomic(
        &self,
        write: SurrealModelLaneWrite,
        lane_id: Option<&str>,
        expected_last_event_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRecord, SurrealStorageError> {
        validate_scope(scope)?;
        if write.kind != ModelLaneRecordKind::RecoveryCheckpoint {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "recovery checkpoint writer requires checkpoint record kind",
            });
        }
        validate_identity(&write.aggregate_id, &write.idempotency_key)?;
        let expected = write.clone();
        let event_payload_hash = sha256_hex(write.event_payload_json.as_bytes());
        let lane_id = lane_id.unwrap_or_default();
        let bindings = RecoveryCheckpointWriteBindings {
            record_id: stable_record_id(write.kind.as_str(), &write.aggregate_id, scope),
            run_record_id: stable_record_id(ModelLaneRecordKind::Run.as_str(), &write.run_id, scope),
            lane_record_id: stable_record_id(ModelLaneRecordKind::Lane.as_str(), lane_id, scope),
            has_lane: !lane_id.is_empty(),
            lane_id: lane_id.to_owned(),
            event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            aggregate_id: write.aggregate_id,
            run_id: write.run_id,
            idempotency_key: write.idempotency_key,
            record_json: write.record_json,
            search_terms: write.search_terms,
            event_payload_json: write.event_payload_json,
            event_payload_hash,
            expected_last_event_seq,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRow, _>(
                            RECORD_RECOVERY_CHECKPOINT_QUERY,
                            bindings,
                            2,
                        )
                        .await
                })
            })
            .await?;
        let row = rows.into_iter().next().map(Into::into).ok_or(
            SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic recovery checkpoint write returned no row",
            },
        )?;
        if !record_matches_write(&row, &expected) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "recovery checkpoint immutable identity or idempotency conflict",
            });
        }
        Ok(row)
    }

    pub(crate) async fn next_recovery_order_seq(
        &self,
        run_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<i64, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = run_event_bindings(run_id, 0, scope);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredRecoveryOrder, _>(
                            "SELECT next_value FROM model_lane_recovery_order WHERE run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() > 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "recovery replay-order authority is ambiguous",
            });
        }
        Ok(rows.first().map_or(1, |row| row.next_value + 1))
    }

    pub(crate) async fn put_recovery_event_atomic(
        &self,
        write: SurrealModelLaneWrite,
        lane_id: Option<&str>,
        source_event_seq: Option<i64>,
        expected_replay_order_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        if write.kind != ModelLaneRecordKind::RecoveryEvent {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "recovery event writer requires recovery-event record kind",
            });
        }
        validate_identity(&write.aggregate_id, &write.idempotency_key)?;
        let expected = write.clone();
        let event_payload_hash = sha256_hex(write.event_payload_json.as_bytes());
        let lane_id = lane_id.unwrap_or_default();
        let bindings = RecoveryEventWriteBindings {
            record_id: stable_record_id(write.kind.as_str(), &write.aggregate_id, scope),
            run_record_id: stable_record_id(ModelLaneRecordKind::Run.as_str(), &write.run_id, scope),
            lane_record_id: stable_record_id(ModelLaneRecordKind::Lane.as_str(), lane_id, scope),
            order_record_id: stable_record_id("recovery_order", &write.run_id, scope),
            has_lane: !lane_id.is_empty(),
            lane_id: lane_id.to_owned(),
            has_source_event: source_event_seq.is_some(),
            source_event_seq: source_event_seq.unwrap_or_default(),
            expected_replay_order_seq,
            event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            aggregate_id: write.aggregate_id,
            run_id: write.run_id,
            idempotency_key: write.idempotency_key,
            record_json: write.record_json,
            search_terms: write.search_terms,
            event_payload_json: write.event_payload_json,
            event_payload_hash,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRow, _>(RECORD_RECOVERY_EVENT_QUERY, bindings, 2)
                        .await
                })
            })
            .await?;
        let Some(row) = rows.into_iter().next().map(Into::into) else {
            return Ok(None);
        };
        if !record_matches_write(&row, &expected) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "recovery event immutable identity or idempotency conflict",
            });
        }
        Ok(Some(row))
    }

    pub(crate) async fn put_pair_immutable(
        &self,
        first: SurrealModelLaneWrite,
        second: SurrealModelLaneWrite,
        scope: &ModelLaneScope,
    ) -> Result<(SurrealModelLaneRecord, SurrealModelLaneRecord), SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(&first.aggregate_id, &first.idempotency_key)?;
        validate_identity(&second.aggregate_id, &second.idempotency_key)?;
        let expected_first = first.clone();
        let expected_second = second.clone();
        let first_event_payload_hash = sha256_hex(first.event_payload_json.as_bytes());
        let second_event_payload_hash = sha256_hex(second.event_payload_json.as_bytes());
        let bindings = PairWriteBindings {
            first_record_id: stable_record_id(first.kind.as_str(), &first.aggregate_id, scope),
            first_event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            first_record_kind: first.kind.as_str().to_owned(),
            first_aggregate_id: first.aggregate_id,
            first_run_id: first.run_id,
            first_idempotency_key: first.idempotency_key,
            first_record_json: first.record_json,
            first_search_terms: first.search_terms,
            first_event_payload_json: first.event_payload_json,
            first_event_payload_hash,
            second_record_id: stable_record_id(second.kind.as_str(), &second.aggregate_id, scope),
            second_event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            second_record_kind: second.kind.as_str().to_owned(),
            second_aggregate_id: second.aggregate_id,
            second_run_id: second.run_id,
            second_idempotency_key: second.idempotency_key,
            second_record_json: second.record_json,
            second_search_terms: second.search_terms,
            second_event_payload_json: second.event_payload_json,
            second_event_payload_hash,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredRow, _>(PAIR_IMMUTABLE_QUERY, bindings, 3).await
        })).await?;
        validate_pair_result(rows, &expected_first, &expected_second)
    }

    pub(crate) async fn put_message_immutable_guarded(
        &self,
        message: SurrealModelLaneWrite,
        payload_binding: Option<SurrealModelLaneWrite>,
        guard: SurrealModelLaneMessageGuard,
        scope: &ModelLaneScope,
    ) -> Result<(SurrealModelLaneRecord, Option<SurrealModelLaneRecord>), SurrealStorageError> {
        validate_scope(scope)?;
        if message.kind != ModelLaneRecordKind::Message {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "guarded model-lane message write requires message record kind",
            });
        }
        validate_identity(&message.aggregate_id, &message.idempotency_key)?;
        if let Some(binding) = payload_binding.as_ref() {
            validate_identity(&binding.aggregate_id, &binding.idempotency_key)?;
        }
        if guard.promotion_decision_id.is_some() != guard.promotion_record_json.is_some() {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "promotion message guard must carry complete decision identity",
            });
        }

        let expected_message = message.clone();
        let expected_binding = payload_binding.clone();
        let message_event_payload_hash = sha256_hex(message.event_payload_json.as_bytes());
        let binding = payload_binding.unwrap_or_else(|| SurrealModelLaneWrite {
            kind: ModelLaneRecordKind::ContextArtifact,
            aggregate_id: String::new(),
            run_id: String::new(),
            idempotency_key: String::new(),
            record_json: String::new(),
            search_terms: Vec::new(),
            event_payload_json: String::new(),
        });
        let binding_event_payload_hash = sha256_hex(binding.event_payload_json.as_bytes());
        let now = Utc::now();
        let crdt = guard.crdt.clone().unwrap_or(SurrealModelLaneCrdtGuard {
            update_ref: String::new(),
            update_id: String::new(),
            update_sha256: String::new(),
            state_vector: String::new(),
            snapshot_ref: String::new(),
            snapshot_id: String::new(),
            snapshot_sha256: String::new(),
            document_id: String::new(),
            crdt_document_id: String::new(),
            actor_id: String::new(),
            actor_kind: String::new(),
            session_id: String::new(),
            trace_id: String::new(),
            lease_id: String::new(),
            lease_scope_kind: String::new(),
            lease_scope_id: String::new(),
            lease_claimed_at_utc: now.clone(),
            lease_expires_at_utc: now.clone(),
            lease_admitted_at_utc: now,
            proposal_id: None,
            proposal_diff_sha256: None,
        });
        if crdt.proposal_id.is_some() != crdt.proposal_diff_sha256.is_some() {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal message guard must carry complete proposal identity",
            });
        }
        let bindings = GuardedMessageBindings {
            message_record_id: stable_record_id(
                message.kind.as_str(),
                &message.aggregate_id,
                scope,
            ),
            message_event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            message_aggregate_id: message.aggregate_id,
            message_run_id: message.run_id,
            message_idempotency_key: message.idempotency_key,
            message_record_json: message.record_json,
            message_search_terms: message.search_terms,
            message_event_payload_json: message.event_payload_json,
            message_event_payload_hash,
            has_payload_binding: expected_binding.is_some(),
            binding_record_id: stable_record_id(
                binding.kind.as_str(),
                &binding.aggregate_id,
                scope,
            ),
            binding_event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            binding_record_kind: binding.kind.as_str().to_owned(),
            binding_aggregate_id: binding.aggregate_id,
            binding_run_id: binding.run_id,
            binding_idempotency_key: binding.idempotency_key,
            binding_record_json: binding.record_json,
            binding_search_terms: binding.search_terms,
            binding_event_payload_json: binding.event_payload_json,
            binding_event_payload_hash,
            source_lane_record_id: stable_record_id(
                ModelLaneRecordKind::Lane.as_str(),
                &guard.source_lane_id,
                scope,
            ),
            source_lane_id: guard.source_lane_id,
            source_lane_record_json: guard.source_lane_record_json,
            source_session_term: format!("session_id={}", guard.source_session_id),
            source_model_session_term: format!(
                "model_session_id={}",
                guard.source_model_session_id
            ),
            has_promotion: guard.promotion_decision_id.is_some(),
            promotion_record_id: guard
                .promotion_decision_id
                .as_deref()
                .map(|id| stable_record_id(ModelLaneRecordKind::PromotionDecision.as_str(), id, scope))
                .unwrap_or_default(),
            promotion_record_json: guard.promotion_record_json.unwrap_or_default(),
            has_crdt: guard.crdt.is_some(),
            crdt_update_ref: crdt.update_ref,
            crdt_update_id: crdt.update_id,
            crdt_update_sha256: crdt.update_sha256,
            crdt_state_vector: crdt.state_vector,
            crdt_snapshot_ref: crdt.snapshot_ref,
            crdt_snapshot_id: crdt.snapshot_id,
            crdt_snapshot_sha256: crdt.snapshot_sha256,
            crdt_document_id: crdt.document_id,
            crdt_crdt_document_id: crdt.crdt_document_id,
            crdt_actor_id: crdt.actor_id,
            crdt_actor_kind: crdt.actor_kind,
            crdt_session_id: crdt.session_id,
            crdt_trace_id: crdt.trace_id,
            crdt_lease_id: crdt.lease_id,
            crdt_lease_scope_kind: crdt.lease_scope_kind,
            crdt_lease_scope_id: crdt.lease_scope_id,
            crdt_lease_claimed_at_utc: crdt.lease_claimed_at_utc,
            crdt_lease_expires_at_utc: crdt.lease_expires_at_utc,
            crdt_lease_admitted_at_utc: crdt.lease_admitted_at_utc,
            has_crdt_proposal: crdt.proposal_id.is_some(),
            crdt_proposal_id: crdt.proposal_id.unwrap_or_default(),
            crdt_proposal_diff_sha256: crdt.proposal_diff_sha256.unwrap_or_default(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredRow, _>(GUARDED_MESSAGE_QUERY, bindings, 2).await
        })).await?;
        if let Some(expected_binding) = expected_binding.as_ref() {
            let (message, binding) =
                validate_pair_result(rows, &expected_message, expected_binding)?;
            Ok((message, Some(binding)))
        } else {
            let mut rows = rows.into_iter().map(SurrealModelLaneRecord::from);
            let row = rows.next().ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "guarded model-lane message write returned no row",
            })?;
            if rows.next().is_some() || !record_matches_write(&row, &expected_message) {
                return Err(SurrealStorageError::InvalidModelLaneRecord {
                    reason: "guarded model-lane message identity or idempotency conflict",
                });
            }
            Ok((row, None))
        }
    }

    pub(crate) async fn replace(
        &self,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        record_json: String,
        search_terms: Vec<String>,
        event_payload_json: String,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        self.replace_with_event_id(
            kind,
            aggregate_id,
            record_json,
            search_terms,
            event_payload_json,
            format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            scope,
        )
        .await
    }

    pub(crate) async fn replace_with_event_id(
        &self,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        record_json: String,
        search_terms: Vec<String>,
        event_payload_json: String,
        event_id: String,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        if aggregate_id.trim().is_empty() {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane aggregate id must not be blank",
            });
        }
        let event_payload_hash = sha256_hex(event_payload_json.as_bytes());
        let bindings = WriteBindings {
            record_id: stable_record_id(kind.as_str(), aggregate_id, scope),
            event_id,
            record_kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            run_id: String::new(),
            idempotency_key: String::new(),
            record_json,
            search_terms,
            event_payload_json,
            event_payload_hash,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredRow, _>(
                "BEGIN TRANSACTION;\
                 LET $target = type::record('model_lane_authority', $record_id);\
                 LET $current = (SELECT * FROM ONLY $target WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
                 IF $current IS NONE { RETURN []; } ELSE {\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $current.run_id, session_run_id: $current.run_id, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_REPLACED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $current.event_id, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $current.run_id, event_stream_version: $current.event_stream_version + 1, event_payload_json: $event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                   LET $seq = $ledger[0].event_sequence;\
                   RETURN UPDATE $target SET record_json = $record_json, search_terms = $search_terms, event_id = $event_id, event_ledger_event_id = type::record('kernel_event_ledger', $event_id), event_seq = $seq, event_stream_version += 1, transaction_seq = $seq;\
                 };\
                 COMMIT TRANSACTION;",
                bindings,
                3,
            ).await
        })).await?;
        Ok(rows.into_iter().next().map(Into::into))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn replace_if_version(
        &self,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        expected_event_stream_version: i64,
        record_json: String,
        search_terms: Vec<String>,
        event_payload_json: String,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        self.replace_if_version_with_event_id(
            kind,
            aggregate_id,
            expected_event_stream_version,
            record_json,
            search_terms,
            event_payload_json,
            format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            scope,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn replace_if_version_with_event_id(
        &self,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        expected_event_stream_version: i64,
        record_json: String,
        search_terms: Vec<String>,
        event_payload_json: String,
        event_id: String,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        if aggregate_id.trim().is_empty()
            || event_id.trim().is_empty()
            || expected_event_stream_version < 1
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane compare-and-swap identity or version is invalid",
            });
        }
        let expected_record_json = record_json.clone();
        let event_payload_hash = sha256_hex(event_payload_json.as_bytes());
        let bindings = CompareAndSwapBindings {
            record_id: stable_record_id(kind.as_str(), aggregate_id, scope),
            event_id,
            record_kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            expected_event_stream_version,
            record_json,
            search_terms,
            event_payload_json,
            event_payload_hash,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredRow, _>(
                "BEGIN TRANSACTION;\
                 LET $target = type::record('model_lane_authority', $record_id);\
                 LET $current = (SELECT * FROM ONLY $target WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
                 IF $current IS NONE OR $current.event_stream_version != $expected_event_stream_version { RETURN []; } ELSE {\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $current.run_id, session_run_id: $current.run_id, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_REPLACED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $current.event_id, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $current.run_id, event_stream_version: $current.event_stream_version + 1, event_payload_json: $event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                   LET $seq = $ledger[0].event_sequence;\
                   RETURN UPDATE $target SET record_json = $record_json, search_terms = $search_terms, event_id = $event_id, event_ledger_event_id = type::record('kernel_event_ledger', $event_id), event_seq = $seq, event_stream_version += 1, transaction_seq = $seq;\
                 };\
                 COMMIT TRANSACTION;",
                bindings,
                3,
            ).await
        })).await?;
        let Some(row) = rows.into_iter().next().map(SurrealModelLaneRecord::from) else {
            return Ok(None);
        };
        if row.aggregate_id != aggregate_id
            || row.event_stream_version != expected_event_stream_version + 1
            || row.record_json != expected_record_json
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane compare-and-swap returned a mismatched authority row",
            });
        }
        Ok(Some(row))
    }

    pub(crate) async fn get(
        &self,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        self.query_first(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 1;",
            scoped_bindings(kind, aggregate_id, "", "", scope),
        ).await
    }

    pub(crate) async fn list_run(
        &self,
        kind: ModelLaneRecordKind,
        run_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        self.query_rows(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = $record_kind AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC;",
            scoped_bindings(kind, "", run_id, "", scope),
        ).await
    }

    pub(crate) async fn list_kind(
        &self,
        kind: ModelLaneRecordKind,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        self.query_rows(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = $record_kind AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC;",
            scoped_bindings(kind, "", "", "", scope),
        ).await
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn test_list_kind_bounded(
        &self,
        kind: ModelLaneRecordKind,
        max_rows: usize,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        if !(1..=256).contains(&max_rows) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane test inspection limit must be between 1 and 256",
            });
        }
        let bindings = ScopedLimitBindings {
            record_kind: kind.as_str().to_owned(),
            run_id: String::new(),
            row_limit: i64::try_from(max_rows + 1).map_err(|_| {
                SurrealStorageError::InvalidModelLaneRecord {
                    reason: "model-lane test inspection limit overflow",
                }
            })?,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredRow, _>(
                            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = $record_kind AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC LIMIT $row_limit;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() > max_rows {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane test inspection exceeded its bounded row limit",
            });
        }
        Ok(rows.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn test_scoped_authority_receipts(
        &self,
        run_id: &str,
        max_rows: usize,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneScopedAuthorityReceipt>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(run_id, run_id)?;
        if !(1..=256).contains(&max_rows) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane authority receipt limit must be between 1 and 256",
            });
        }
        let bindings = ScopedLimitBindings {
            record_kind: String::new(),
            run_id: run_id.to_owned(),
            row_limit: i64::try_from(max_rows + 1).map_err(|_| {
                SurrealStorageError::InvalidModelLaneRecord {
                    reason: "model-lane authority receipt limit overflow",
                }
            })?,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<SurrealModelLaneScopedAuthorityReceipt, _>(
                "SELECT record_kind, aggregate_id, run_id, event_id, event_ledger_event_id.event_sequence AS event_ledger_seq, event_ledger_event_id.event_type AS event_type, event_ledger_event_id.payload_hash AS payload_hash, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id FROM model_lane_authority WHERE run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.aggregate_type = record_kind AND event_ledger_event_id.aggregate_id = aggregate_id AND event_ledger_event_id.session_run_id = run_id AND event_seq = event_ledger_event_id.event_sequence ORDER BY event_seq ASC LIMIT $row_limit;",
                bindings,
            ).await
        })).await?;
        if rows.len() > max_rows {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane authority receipt inspection exceeded its bounded row limit",
            });
        }
        Ok(rows)
    }

    pub(crate) async fn find_by_term(
        &self,
        kind: ModelLaneRecordKind,
        search_term: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        self.query_rows(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = $record_kind AND $search_term IN search_terms AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC;",
            scoped_bindings(kind, "", "", search_term, scope),
        ).await
    }

    pub(crate) async fn crdt_update_by_ref(
        &self,
        update_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            authority_ref: update_bytes_ref.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<SurrealModelLaneCrdtUpdate, _>(
                            "SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM kernel_crdt_updates WHERE update_bytes_ref = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            })
            .await
    }

    pub(crate) async fn crdt_update_by_id(
        &self,
        update_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            authority_ref: update_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<SurrealModelLaneCrdtUpdate, _>(
                            "SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM kernel_crdt_updates WHERE update_id = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            })
            .await
    }

    pub(crate) async fn crdt_snapshot_by_ref(
        &self,
        snapshot_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtSnapshot>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            authority_ref: snapshot_bytes_ref.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<SurrealModelLaneCrdtSnapshot, _>(
                            "SELECT schema_id, snapshot_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq, state_vector, snapshot_sha256, snapshot_bytes_ref, encoding::base64::encode(snapshot_bytes) AS snapshot_bytes_b64, actor_id, actor_kind, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, promotion_evidence_update_ids, storage_authority, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.document_id AS ledger_document_id, event_ledger_event_id.payload.state_vector AS ledger_state_vector, event_ledger_event_id.payload.covered_update_seq AS ledger_covered_update_seq FROM kernel_crdt_snapshots WHERE snapshot_bytes_ref = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            })
            .await
    }

    pub(crate) async fn crdt_update_chain(
        &self,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtChainBindings {
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: document_id.to_owned(),
            crdt_document_id: crdt_document_id.to_owned(),
            covered_update_seq,
            update_seq,
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<SurrealModelLaneCrdtUpdate, _>(
                            "SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM kernel_crdt_updates WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND update_seq > $covered_update_seq AND update_seq <= $update_seq ORDER BY update_seq ASC;",
                            bindings,
                        )
                        .await
                })
            })
            .await
    }

    pub(crate) async fn crdt_seen_update_ids(
        &self,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<String>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtChainBindings {
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: document_id.to_owned(),
            crdt_document_id: crdt_document_id.to_owned(),
            covered_update_seq,
            update_seq: covered_update_seq,
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<CrdtUpdateIdRow, _>(
                "SELECT update_id FROM kernel_crdt_updates WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND update_seq <= $covered_update_seq ORDER BY update_seq ASC;",
                bindings,
            ).await
        })).await?;
        Ok(rows.into_iter().map(|row| row.update_id).collect())
    }

    pub(crate) async fn active_crdt_leases(
        &self,
        lane_id: &str,
        actor_id: &str,
        actor_kind: &str,
        session_id: &str,
        correlation_id: &str,
        document_id: &str,
        crdt_document_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneCrdtLease>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtLeaseBindings {
            lane_id: lane_id.to_owned(),
            actor_id: actor_id.to_owned(),
            actor_kind: actor_kind.to_owned(),
            session_id: session_id.to_owned(),
            correlation_id: correlation_id.to_owned(),
            workspace_id: scope.workspace_id.clone(),
            document_id: document_id.to_owned(),
            crdt_document_id: crdt_document_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
        };
        self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<SurrealModelLaneCrdtLease, _>(
                "SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, time::now() AS admitted_at_utc FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND lane_id = $lane_id AND actor_id = $actor_id AND actor_kind = $actor_kind AND session_id = $session_id AND correlation_id = $correlation_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc IS NONE AND expired_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id AND ((scope_kind = 'workspace' AND scope_id = $workspace_id) OR (scope_kind = 'document' AND scope_id = $crdt_document_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id)) ORDER BY claimed_at_utc ASC, lease_id ASC;",
                bindings,
            ).await
        })).await
    }

    pub(crate) async fn crdt_proposal(
        &self,
        proposal_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposal>, SurrealStorageError> {
        Ok(self
            .crdt_proposal_record(proposal_id, scope)
            .await?
            .map(|row| SurrealModelLaneCrdtProposal {
                proposal_id: row.proposal_id,
                owner_account_id: row.owner_account_id,
                actor_principal_id: row.actor_principal_id,
                authenticated_session_id: row.authenticated_session_id,
                access_space_id: row.access_space_id,
                workspace_id: row.workspace_id,
                document_id: row.document_id,
                crdt_document_id: row.crdt_document_id,
                actor_id: row.actor_id,
                actor_kind: row.actor_kind,
                session_id: row.session_id,
                correlation_id: row.correlation_id,
                review_state: row.review_state,
                diff_sha256: row.diff_sha256,
                applied_update_id: row.applied_update_id,
                applied_update_sha256: row.applied_update_sha256,
            }))
    }

    pub(crate) async fn crdt_proposal_record(
        &self,
        proposal_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            authority_ref: proposal_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let row = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<SurrealModelLaneCrdtProposalRecord, _>(
                            "SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, record::id(lease_id) AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        if let Some(row) = row.as_ref() {
            self.validate_crdt_proposal_receipts(row, scope).await?;
        }
        Ok(row)
    }

    async fn validate_crdt_proposal_receipts(
        &self,
        proposal: &SurrealModelLaneCrdtProposalRecord,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        let event_ids = proposal_receipt_ids(proposal)?;
        let bindings = CrdtProposalReceiptBindings {
            event_ids,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let receipts = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<CrdtProposalReceiptRow, _>(
                            "SELECT event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, causation_id, correlation_id, payload_hash, source_component, payload FROM kernel_event_ledger WHERE event_id IN $event_ids AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        validate_crdt_proposal_receipt_rows(proposal, receipts)
    }

    pub(crate) async fn crdt_lease_history(
        &self,
        lease_id: &str,
        document_id: &str,
        crdt_document_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtLeaseHistoryBindings {
            authority_ref: lease_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: document_id.to_owned(),
            crdt_document_id: crdt_document_id.to_owned(),
        };
        self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_first::<SurrealModelLaneCrdtLeaseHistory, _>(
                "SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id AND ((scope_kind = 'workspace' AND scope_id = $workspace_id) OR (scope_kind = 'document' AND scope_id = $crdt_document_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id)) LIMIT 1;",
                bindings,
            ).await
        })).await
    }

    pub(crate) async fn validate_event_link(
        &self,
        kind: ModelLaneRecordKind,
        record: &SurrealModelLaneRecord,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = EventLinkBindings {
            event_id: record.event_id.clone(),
            record_kind: kind.as_str().to_owned(),
            aggregate_id: record.aggregate_id.clone(),
            run_id: record.run_id.clone(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredEventLink, _>(
                            "SELECT event_ledger_event_id.event_id AS event_id, event_ledger_event_id.event_sequence AS event_seq, event_ledger_event_id.payload.event_stream_version AS event_stream_version, event_ledger_event_id.event_sequence AS transaction_seq, event_ledger_event_id.aggregate_id AS aggregate_id, event_ledger_event_id.session_run_id AS run_id, event_ledger_event_id.payload_hash AS payload_hash, event_ledger_event_id.payload.event_payload_json AS event_payload_json FROM model_lane_authority WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id AND run_id = $run_id AND event_id = $event_id AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.aggregate_type = $record_kind AND event_ledger_event_id.aggregate_id = $aggregate_id AND event_ledger_event_id.session_run_id = $run_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() != 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane authority row lacks one exact-scope EventLedger envelope",
            });
        }
        let event = &rows[0];
        if event.event_id != record.event_id
            || event.aggregate_id != record.aggregate_id
            || event.run_id != record.run_id
            || event.event_seq != record.event_seq
            || event.event_stream_version != record.event_stream_version
            || event.transaction_seq != record.transaction_seq
            || event.payload_hash != sha256_hex(event.event_payload_json.as_bytes())
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "model-lane authority/EventLedger envelope mismatch",
            });
        }
        Ok(())
    }

    pub(crate) async fn run_event_high_watermark(
        &self,
        run_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<i64>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = run_event_bindings(run_id, 0, scope);
        let row = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<StoredEventSequence, _>(
                            "SELECT event_sequence AS event_seq FROM kernel_event_ledger WHERE source_component = 'model_lane' AND session_run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_sequence DESC LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        Ok(row.map(|row| row.event_seq))
    }

    pub(crate) async fn run_contains_event_sequence(
        &self,
        run_id: &str,
        event_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<bool, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = run_event_bindings(run_id, event_seq, scope);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredEventSequence, _>(
                            "SELECT event_sequence AS event_seq FROM kernel_event_ledger WHERE source_component = 'model_lane' AND session_run_id = $run_id AND event_sequence = $event_seq AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        Ok(rows.len() == 1)
    }

    pub(crate) async fn process_is_durably_closed(
        &self,
        process_uuid: uuid::Uuid,
        scope: &ModelLaneScope,
    ) -> Result<bool, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = ProcessClosureBindings {
            process_uuid,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredProcessClosure, _>(
                            "SELECT stopped_at, exit_code, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() > 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "process closure identity is ambiguous in exact scope",
            });
        }
        Ok(rows.first().is_some_and(|row| {
            row.stopped_at.is_some() && row.exit_code.is_some() && row.stop_reason.is_some()
        }))
    }

    pub(crate) async fn schema_registry(
        &self,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneSchemaRow>, SurrealStorageError> {
        validate_scope(scope)?;
        let mut seeds = Vec::with_capacity(CANONICAL_MODEL_LANE_SCHEMAS.len());
        for (schema_id, schema_version) in CANONICAL_MODEL_LANE_SCHEMAS {
            let record_id = sha256_hex(
                format!(
                    "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                    schema_id,
                    scope.owner_account_id,
                    scope.actor_principal_id,
                    scope.authenticated_session_id,
                    scope.access_space_id,
                    scope.workspace_id,
                )
                .as_bytes(),
            );
            seeds.push((
                record_id,
                StoredSchemaWrite {
                    schema_id: (*schema_id).to_owned(),
                    schema_version: *schema_version,
                    record_kind: "model_lane".to_owned(),
                    table_name: "model_lane_authority".to_owned(),
                    source_component: "swarm_orchestration::model_lane".to_owned(),
                    owner_account_id: scope.owner_account_id.clone(),
                    actor_principal_id: scope.actor_principal_id.clone(),
                    authenticated_session_id: scope.authenticated_session_id.clone(),
                    access_space_id: scope.access_space_id.clone(),
                    workspace_id: scope.workspace_id.clone(),
                },
            ));
        }
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    for (record_id, seed) in seeds {
                        let _: Option<StoredSchemaRow> = database
                            .upsert_one("model_lane_schema_registry", &record_id, seed)
                            .await?;
                    }
                    Ok(())
                })
            })
            .await?;
        let bindings = scoped_bindings(ModelLaneRecordKind::Run, "", "", "", scope);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredSchemaRow, _>(
                            "SELECT schema_id, schema_version, record_kind, table_name FROM model_lane_schema_registry WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY schema_id ASC;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        let mut rows: Vec<SurrealModelLaneSchemaRow> = rows.into_iter().map(Into::into).collect();
        rows.sort_by(|left, right| left.schema_id.cmp(&right.schema_id));
        Ok(rows)
    }

    async fn query_first(
        &self,
        statement: &'static str,
        bindings: ScopedBindings,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        Ok(self
            .query_rows(statement, bindings)
            .await?
            .into_iter()
            .next())
    }

    async fn query_rows(
        &self,
        statement: &'static str,
        bindings: ScopedBindings,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredRow, _>(statement, bindings)
                        .await
                })
            })
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

fn scoped_bindings(
    kind: ModelLaneRecordKind,
    aggregate_id: &str,
    run_id: &str,
    search_term: &str,
    scope: &ModelLaneScope,
) -> ScopedBindings {
    ScopedBindings {
        record_kind: kind.as_str().to_owned(),
        aggregate_id: aggregate_id.to_owned(),
        run_id: run_id.to_owned(),
        search_term: search_term.to_owned(),
        owner_account_id: scope.owner_account_id.clone(),
        actor_principal_id: scope.actor_principal_id.clone(),
        authenticated_session_id: scope.authenticated_session_id.clone(),
        access_space_id: scope.access_space_id.clone(),
        workspace_id: scope.workspace_id.clone(),
    }
}

fn routing_identity_bindings(identity: &str, scope: &ModelLaneScope) -> ScopedBindings {
    scoped_bindings(
        ModelLaneRecordKind::RoutingExecution,
        identity,
        identity,
        identity,
        scope,
    )
}

fn single_routing_row(
    rows: Vec<SurrealModelLaneRoutingExecutionRow>,
    reason: &'static str,
) -> Result<Option<SurrealModelLaneRoutingExecutionRow>, SurrealStorageError> {
    if rows.len() > 1 {
        return Err(SurrealStorageError::InvalidModelLaneRecord { reason });
    }
    Ok(rows.into_iter().next())
}

fn validate_routing_commit(
    commit: &SurrealModelLaneRoutingCommit,
    scope: &ModelLaneScope,
) -> Result<(), SurrealStorageError> {
    validate_scope(scope)?;
    validate_identity(&commit.execution.execution_id, &commit.execution.execution_id)?;
    validate_identity(&commit.execution.run_id, &commit.execution.run_id)?;
    validate_identity(&commit.attempt.attempt_id, &commit.attempt.attempt_id)?;
    validate_identity(&commit.attempt.stage_id, &commit.attempt.stage_id)?;
    validate_identity(&commit.outbox.command_id, &commit.outbox.command_id)?;
    if commit.expected_revision < 0
        || commit.execution.revision != commit.expected_revision + 1
        || commit.attempt.attempt <= 0
        || commit.outbox.attempt <= 0
        || commit.execution.execution_id != commit.attempt.execution_id
        || commit.execution.execution_id != commit.outbox.execution_id
        || commit.execution.run_id != commit.attempt.run_id
        || commit.execution.run_id != commit.outbox.run_id
        || commit.attempt.stage_id != commit.outbox.stage_id
        || commit.attempt.attempt != commit.outbox.attempt
        || commit.attempt.lease_owner != commit.outbox.lease_owner
        || commit.attempt.fencing_token != commit.outbox.fencing_token
        || commit.attempt.lease_expires_at_unix_ms != commit.outbox.lease_expires_at_unix_ms
        || !matches!(
            commit.outbox.status.as_str(),
            "pending" | "claimed" | "acked" | "cancelled" | "compensated"
        )
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing commit carries mixed identity, revision, lease, or outbox state",
        });
    }
    if commit.execution.record_json.get("execution_id").and_then(Value::as_str)
        != Some(commit.execution.execution_id.as_str())
        || commit.execution.record_json.get("run_id").and_then(Value::as_str)
            != Some(commit.execution.run_id.as_str())
        || commit.execution.record_json.get("revision").and_then(Value::as_i64)
            != Some(commit.execution.revision)
        || commit.attempt.record_json.get("stage_id").and_then(Value::as_str)
            != Some(commit.attempt.stage_id.as_str())
        || commit.attempt.record_json.get("attempt").and_then(Value::as_i64)
            != Some(commit.attempt.attempt)
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing commit JSON projection does not bind its durable identity",
        });
    }
    if commit.execution.context_hash
        != canonical_routing_context_hash(&commit.execution.record_json)?
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing immutable context hash does not bind every canonical context field",
        });
    }
    if let Some(claim) = commit.expected_claim.as_ref() {
        if claim.stage_id != commit.attempt.stage_id
            || claim.attempt != commit.attempt.attempt
            || claim.lease_owner.trim().is_empty()
            || claim.fencing_token.trim().is_empty()
            || claim.observed_at_unix_ms <= 0
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing commit expected claim does not bind the changed attempt",
            });
        }
    }
    validate_routing_events(commit)?;
    validate_routing_optional_authority(commit)?;
    Ok(())
}

fn canonical_routing_context_hash(record_json: &Value) -> Result<String, SurrealStorageError> {
    const IMMUTABLE_FIELDS: [&str; 21] = [
        "schema_id",
        "execution_id",
        "run_id",
        "selecting_decision_id",
        "selecting_decision_event_id",
        "selecting_decision_event_seq",
        "trace_id",
        "run_span_id",
        "coordinator_session_id",
        "locus_ref",
        "work_packet_id",
        "micro_task_id",
        "task_board_id",
        "owner_session",
        "canonical_graph",
        "canonical_graph_sha256",
        "canonical_launch_plan",
        "canonical_launch_plan_sha256",
        "authority",
        "initial_input_ref",
        "initial_input_sha256",
    ];
    let object = record_json.as_object().ok_or(
        SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing execution projection must be an object",
        },
    )?;
    let mut immutable = serde_json::Map::new();
    for field in IMMUTABLE_FIELDS {
        let value = object
            .get(field)
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing execution projection is missing immutable context",
            })?;
        immutable.insert(field.to_owned(), value.clone());
    }
    Ok(sha256_hex(&canonical_json_bytes(&Value::Object(immutable))))
}

fn validate_routing_events(
    commit: &SurrealModelLaneRoutingCommit,
) -> Result<(), SurrealStorageError> {
    if !(3..=4096).contains(&commit.events.len()) {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing commit requires a bounded execution, attempt, outbox, and optional extra-event set",
        });
    }
    let expected = [
        (
            "model_lane_routing_execution",
            commit.execution.execution_id.as_str(),
        ),
        (
            "model_lane_routing_stage_attempt",
            commit.attempt.attempt_id.as_str(),
        ),
        ("model_lane_routing_outbox", commit.outbox.command_id.as_str()),
    ];
    for (aggregate_type, aggregate_id) in expected.iter().copied() {
        let matching = commit
            .events
            .iter()
            .filter(|event| {
                event.aggregate_type == aggregate_type && event.aggregate_id == aggregate_id
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing commit event set has missing or duplicate aggregate authority",
            });
        }
        let event = matching[0];
        if event.event_id.trim().is_empty()
            || event.event_version.trim().is_empty()
            || event.kernel_task_run_id != commit.execution.run_id
            || event.session_run_id != commit.execution.execution_id
            || event.idempotency_key.trim().is_empty()
            || event.actor_kind.trim().is_empty()
            || event.actor_id.trim().is_empty()
            || event.source_component.trim().is_empty()
            || !routing_event_payload_hash_is_canonical(event)
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing commit event does not bind run, execution, actor, or payload",
            });
        }
    }
    let mut event_ids = BTreeSet::new();
    let mut idempotency_keys = BTreeSet::new();
    for event in &commit.events {
        let is_required_projection_event = expected.iter().any(|(aggregate_type, aggregate_id)| {
            event.aggregate_type == *aggregate_type && event.aggregate_id == *aggregate_id
        });
        if event.event_id.trim().is_empty()
            || event.event_version.trim().is_empty()
            || event.kernel_task_run_id != commit.execution.run_id
            || event.session_run_id != commit.execution.execution_id
            || event.aggregate_type.trim().is_empty()
            || event.aggregate_id.trim().is_empty()
            || event.idempotency_key.trim().is_empty()
            || event.actor_kind.trim().is_empty()
            || event.actor_id.trim().is_empty()
            || event.source_component.trim().is_empty()
            || !routing_event_payload_hash_is_canonical(event)
            || !event_ids.insert(event.event_id.as_str())
            || !idempotency_keys.insert(event.idempotency_key.as_str())
            || (!is_required_projection_event
                && event
                    .correlation_id
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty()))
            || (!is_required_projection_event && !event.payload.is_object())
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing commit event does not bind its unique run, execution, actor, correlation, or payload authority",
            });
        }
    }
    Ok(())
}

fn validate_routing_optional_authority(
    commit: &SurrealModelLaneRoutingCommit,
) -> Result<(), SurrealStorageError> {
    if commit.message.is_some() != commit.message_guard.is_some() {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing message requires complete source-lane authority",
        });
    }
    if let Some(message) = commit.message.as_ref() {
        if message.kind != ModelLaneRecordKind::Message
            || message.run_id != commit.execution.run_id
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing message does not bind the committed execution run",
            });
        }
        let guard = commit.message_guard.as_ref().expect("checked above");
        if guard.promotion_decision_id.is_some() || guard.crdt.is_some() {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing atomic message path does not accept promotion or CRDT authority",
            });
        }
    }
    if let Some(binding) = commit.binding.as_ref() {
        if binding.kind != ModelLaneRecordKind::ContextArtifact
            || binding.run_id != commit.execution.run_id
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "routing binding does not bind the committed execution run",
            });
        }
    }
    Ok(())
}

fn routing_commit_bindings(
    commit: SurrealModelLaneRoutingCommit,
    scope: &ModelLaneScope,
) -> Result<RoutingCommitBindings, SurrealStorageError> {
    let SurrealModelLaneRoutingCommit {
        expected_revision,
        expected_claim,
        execution,
        attempt,
        outbox,
        mut events,
        message,
        binding,
        message_guard,
    } = commit;
    scope_routing_event_identities(&mut events, scope);
    let execution_event = take_routing_event(
        &mut events,
        "model_lane_routing_execution",
        &execution.execution_id,
    )?;
    let attempt_event = take_routing_event(
        &mut events,
        "model_lane_routing_stage_attempt",
        &attempt.attempt_id,
    )?;
    let outbox_event = take_routing_event(
        &mut events,
        "model_lane_routing_outbox",
        &outbox.command_id,
    )?;
    let has_expected_claim = expected_claim.is_some();
    let expected_claim = expected_claim.unwrap_or(SurrealModelLaneRoutingClaim {
        stage_id: String::new(),
        attempt: 0,
        lease_owner: String::new(),
        fencing_token: String::new(),
        observed_at_unix_ms: 0,
    });
    let has_message = message.is_some();
    let message = message
        .map(|write| routing_authority_write_bindings(write, scope))
        .unwrap_or_else(empty_routing_authority_write_bindings);
    let has_binding = binding.is_some();
    let binding = binding
        .map(|write| routing_authority_write_bindings(write, scope))
        .unwrap_or_else(empty_routing_authority_write_bindings);
    let message_guard = message_guard
        .map(|guard| routing_message_guard_bindings(guard, scope))
        .unwrap_or_else(empty_routing_message_guard_bindings);
    Ok(RoutingCommitBindings {
        expected_revision,
        has_expected_claim,
        expected_claim,
        run_record_id: stable_record_id(
            ModelLaneRecordKind::Run.as_str(),
            &execution.run_id,
            scope,
        ),
        execution_record_id: stable_record_id(
            "routing_execution",
            &execution.execution_id,
            scope,
        ),
        attempt_record_id: stable_record_id("routing_attempt", &attempt.attempt_id, scope),
        outbox_record_id: stable_record_id("routing_outbox", &outbox.command_id, scope),
        execution,
        attempt,
        outbox,
        execution_event,
        attempt_event,
        outbox_event,
        extra_events: events,
        has_message,
        message,
        has_binding,
        binding,
        message_guard,
        owner_account_id: scope.owner_account_id.clone(),
        actor_principal_id: scope.actor_principal_id.clone(),
        authenticated_session_id: scope.authenticated_session_id.clone(),
        access_space_id: scope.access_space_id.clone(),
        workspace_id: scope.workspace_id.clone(),
    })
}

fn scope_routing_event_identities(
    events: &mut [SurrealModelLaneRoutingEventWrite],
    scope: &ModelLaneScope,
) {
    for event in events {
        let logical_identity = format!(
            "{}\u{1f}{}\u{1f}{}",
            event.idempotency_key, event.aggregate_type, event.aggregate_id
        );
        event.event_id = format!(
            "routing-event-{}",
            stable_record_id("routing_event", &logical_identity, scope)
        );
        event.idempotency_key = format!(
            "routing-idempotency-{}",
            stable_record_id("routing_event_idempotency", &logical_identity, scope)
        );
    }
}

fn canonical_routing_payload_hash(payload: &Value) -> String {
    sha256_hex(&canonical_json_bytes(payload))
}

fn routing_event_payload_hash_is_canonical(event: &SurrealModelLaneRoutingEventWrite) -> bool {
    event.payload_hash == canonical_routing_payload_hash(&event.payload)
}

fn take_routing_event(
    events: &mut Vec<SurrealModelLaneRoutingEventWrite>,
    aggregate_type: &str,
    aggregate_id: &str,
) -> Result<SurrealModelLaneRoutingEventWrite, SurrealStorageError> {
    let position = events
        .iter()
        .position(|event| {
            event.aggregate_type == aggregate_type && event.aggregate_id == aggregate_id
        })
        .ok_or(SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing commit is missing required event authority",
        })?;
    Ok(events.remove(position))
}

fn routing_authority_write_bindings(
    write: SurrealModelLaneWrite,
    scope: &ModelLaneScope,
) -> RoutingAuthorityWriteBindings {
    let event_identity = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        write.kind.as_str(), write.aggregate_id, write.run_id, write.idempotency_key
    );
    RoutingAuthorityWriteBindings {
        record_id: stable_record_id(write.kind.as_str(), &write.aggregate_id, scope),
        event_id: format!(
            "evt-model-lane-routing-{}",
            stable_record_id("routing_authority_event", &event_identity, scope)
        ),
        record_kind: write.kind.as_str().to_owned(),
        aggregate_id: write.aggregate_id,
        run_id: write.run_id,
        idempotency_key: write.idempotency_key,
        record_json: write.record_json,
        search_terms: write.search_terms,
        event_payload_hash: sha256_hex(write.event_payload_json.as_bytes()),
        event_payload_json: write.event_payload_json,
    }
}

fn routing_message_guard_bindings(
    guard: SurrealModelLaneMessageGuard,
    scope: &ModelLaneScope,
) -> RoutingMessageGuardBindings {
    RoutingMessageGuardBindings {
        source_lane_record_id: stable_record_id(
            ModelLaneRecordKind::Lane.as_str(),
            &guard.source_lane_id,
            scope,
        ),
        source_lane_id: guard.source_lane_id,
        source_lane_record_json: guard.source_lane_record_json,
        source_session_term: format!("session_id={}", guard.source_session_id),
        source_model_session_term: format!(
            "model_session_id={}",
            guard.source_model_session_id
        ),
    }
}

fn empty_routing_authority_write_bindings() -> RoutingAuthorityWriteBindings {
    RoutingAuthorityWriteBindings {
        record_id: "unused".to_owned(),
        event_id: "unused".to_owned(),
        record_kind: "unused".to_owned(),
        aggregate_id: "unused".to_owned(),
        run_id: "unused".to_owned(),
        idempotency_key: "unused".to_owned(),
        record_json: "{}".to_owned(),
        search_terms: Vec::new(),
        event_payload_json: "{}".to_owned(),
        event_payload_hash: sha256_hex(b"{}"),
    }
}

fn empty_routing_message_guard_bindings() -> RoutingMessageGuardBindings {
    RoutingMessageGuardBindings {
        source_lane_record_id: "unused".to_owned(),
        source_lane_id: "unused".to_owned(),
        source_lane_record_json: "{}".to_owned(),
        source_session_term: "unused".to_owned(),
        source_model_session_term: "unused".to_owned(),
    }
}

fn run_event_bindings(
    run_id: &str,
    event_seq: i64,
    scope: &ModelLaneScope,
) -> RunEventBindings {
    RunEventBindings {
        run_id: run_id.to_owned(),
        event_seq,
        owner_account_id: scope.owner_account_id.clone(),
        actor_principal_id: scope.actor_principal_id.clone(),
        authenticated_session_id: scope.authenticated_session_id.clone(),
        access_space_id: scope.access_space_id.clone(),
        workspace_id: scope.workspace_id.clone(),
    }
}

fn stable_record_id(kind: &str, aggregate_id: &str, scope: &ModelLaneScope) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"handshake.model-lane.authority.v1\0");
    hasher.update(kind.as_bytes());
    hasher.update([0]);
    hasher.update(aggregate_id.as_bytes());
    for value in [
        &scope.owner_account_id,
        &scope.actor_principal_id,
        &scope.authenticated_session_id,
        &scope.access_space_id,
        &scope.workspace_id,
    ] {
        hasher.update([0]);
        hasher.update(value.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn validate_identity(aggregate_id: &str, idempotency_key: &str) -> Result<(), SurrealStorageError> {
    if aggregate_id.trim().is_empty() || idempotency_key.trim().is_empty() {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "model-lane aggregate id and idempotency key must not be blank",
        });
    }
    Ok(())
}

fn validate_pair_result(
    rows: Vec<StoredRow>,
    first: &SurrealModelLaneWrite,
    second: &SurrealModelLaneWrite,
) -> Result<(SurrealModelLaneRecord, SurrealModelLaneRecord), SurrealStorageError> {
    let mut rows = rows.into_iter().map(SurrealModelLaneRecord::from);
    let first_row = rows.next().ok_or(SurrealStorageError::InvalidModelLaneRecord {
        reason: "model-lane immutable pair returned no first row",
    })?;
    let second_row = rows.next().ok_or(SurrealStorageError::InvalidModelLaneRecord {
        reason: "model-lane immutable pair returned no second row",
    })?;
    if rows.next().is_some()
        || !record_matches_write(&first_row, first)
        || !record_matches_write(&second_row, second)
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "model-lane immutable pair identity or idempotency conflict",
        });
    }
    Ok((first_row, second_row))
}

fn record_matches_write(row: &SurrealModelLaneRecord, write: &SurrealModelLaneWrite) -> bool {
    row.aggregate_id == write.aggregate_id
        && row.run_id == write.run_id
        && row.idempotency_key == write.idempotency_key
        && row.record_json == write.record_json
}

fn validate_scope(scope: &ModelLaneScope) -> Result<(), SurrealStorageError> {
    if [
        scope.owner_account_id.as_str(),
        scope.actor_principal_id.as_str(),
        scope.authenticated_session_id.as_str(),
        scope.access_space_id.as_str(),
        scope.workspace_id.as_str(),
    ]
    .into_iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "model-lane authority requires complete five-field resource scope",
        });
    }
    Ok(())
}

fn validate_crdt_update_write(
    row: &SurrealModelLaneCrdtUpdate,
    event: &SurrealModelLaneCrdtEventWrite,
    scope: &ModelLaneScope,
) -> Result<(), SurrealStorageError> {
    validate_scope(scope)?;
    if row.owner_account_id != scope.owner_account_id
        || row.actor_principal_id != scope.actor_principal_id
        || row.authenticated_session_id != scope.authenticated_session_id
        || row.access_space_id != scope.access_space_id
        || row.workspace_id != scope.workspace_id
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT update write scope does not match the injected five-field authority",
        });
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&row.update_bytes_b64)
        .map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT update bytes are not valid base64",
        })?;
    let payload_bytes = canonical_json_bytes(&event.payload);
    if row.update_seq <= 0
        || row.update_sha256 != sha256_hex(&bytes)
        || row.storage_authority != "embedded_surrealdb"
        || !row.update_bytes_ref.starts_with("surreal://")
        || row.event_ledger_event_id != event.event_id
        || row.ledger_session_run_id != event.session_run_id
        || row.ledger_event_type != event.event_type
        || row.ledger_aggregate_type != event.aggregate_type
        || row.ledger_aggregate_id != event.aggregate_id
        || row.ledger_actor_kind != event.actor_kind
        || row.ledger_actor_id != event.actor_id
        || row.ledger_correlation_id != event.correlation_id
        || row.ledger_payload_hash != event.payload_hash
        || event.payload_hash != sha256_hex(&payload_bytes)
        || event.event_version != "kernel_event_v1"
        || event.event_type != "KNOWLEDGE_CRDT_UPDATE_RECORDED"
        || event.aggregate_type != "knowledge_crdt_document"
        || event.aggregate_id != row.crdt_document_id
        || event.actor_id != row.actor_id
        || event.correlation_id.as_deref() != Some(row.trace_id.as_str())
        || event.source_component != "model_lane_crdt"
        || row.ledger_update_id != row.update_id
        || row.ledger_update_seq != row.update_seq
        || row.ledger_actor_payload_id != row.actor_id
        || row.ledger_update_sha256 != row.update_sha256
        || row.ledger_state_vector_before != row.state_vector_before
        || row.ledger_state_vector_after != row.state_vector_after
        || row.replay_encoding != "yjs-update-v1"
        || row.replay_schema_version != "kernel-crdt-update-v1"
        || [
            row.schema_id.as_str(),
            row.document_id.as_str(),
            row.crdt_document_id.as_str(),
            row.update_id.as_str(),
            row.actor_id.as_str(),
            row.actor_kind.as_str(),
            row.session_id.as_str(),
            row.trace_id.as_str(),
            row.state_vector_before.as_str(),
            row.state_vector_after.as_str(),
            row.event_ledger_stream_id.as_str(),
            row.ledger_site_id.as_str(),
            event.event_id.as_str(),
            event.idempotency_key.as_str(),
            event.kernel_task_run_id.as_str(),
            event.source_component.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT update or atomic EventLedger linkage is non-canonical",
        });
    }
    Ok(())
}

fn validate_crdt_snapshot_write(
    row: &SurrealModelLaneCrdtSnapshot,
    event: &SurrealModelLaneCrdtEventWrite,
    scope: &ModelLaneScope,
) -> Result<(), SurrealStorageError> {
    validate_scope(scope)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&row.snapshot_bytes_b64)
        .map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT snapshot bytes are not valid base64",
        })?;
    let payload_bytes = canonical_json_bytes(&event.payload);
    if row.owner_account_id != scope.owner_account_id
        || row.actor_principal_id != scope.actor_principal_id
        || row.authenticated_session_id != scope.authenticated_session_id
        || row.access_space_id != scope.access_space_id
        || row.workspace_id != scope.workspace_id
        || row.covered_update_seq < 0
        || row.snapshot_sha256 != sha256_hex(&bytes)
        || row.storage_authority != "embedded_surrealdb"
        || !row.snapshot_bytes_ref.starts_with("surreal://")
        || row.event_ledger_event_id != event.event_id
        || row.ledger_event_type != event.event_type
        || row.ledger_aggregate_type != event.aggregate_type
        || row.ledger_aggregate_id != event.aggregate_id
        || row.ledger_actor_kind != event.actor_kind
        || row.ledger_actor_id != event.actor_id
        || row.ledger_payload_hash != event.payload_hash
        || event.payload_hash != sha256_hex(&payload_bytes)
        || event.event_version != "kernel_event_v1"
        || event.event_type != "KNOWLEDGE_CRDT_SNAPSHOT_RECORDED"
        || event.aggregate_type != "knowledge_crdt_document"
        || event.aggregate_id != row.crdt_document_id
        || event.actor_id != row.actor_id
        || event.source_component != "model_lane_crdt"
        || row.ledger_document_id != row.document_id
        || row.ledger_state_vector != row.state_vector
        || row.ledger_covered_update_seq != row.covered_update_seq
        || [
            row.schema_id.as_str(),
            row.snapshot_id.as_str(),
            row.document_id.as_str(),
            row.crdt_document_id.as_str(),
            row.state_vector.as_str(),
            row.actor_id.as_str(),
            row.actor_kind.as_str(),
            row.event_ledger_stream_id.as_str(),
            event.event_id.as_str(),
            event.session_run_id.as_str(),
            event.idempotency_key.as_str(),
            event.source_component.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT snapshot or atomic EventLedger linkage is non-canonical",
        });
    }
    Ok(())
}

fn validate_crdt_lease_write(
    row: &SurrealModelLaneCrdtLeaseWrite,
    ttl_seconds: i64,
    event: &SurrealModelLaneCrdtEventWrite,
    scope: &ModelLaneScope,
) -> Result<(), SurrealStorageError> {
    validate_scope(scope)?;
    validate_crdt_transition_event(&row.lease_id, &row.actor_id, ttl_seconds, event)?;
    validate_crdt_event_kind(
        event,
        "knowledge_crdt_lease",
        "KNOWLEDGE_CRDT_LEASE_CLAIMED",
    )?;
    let document_scope_valid = match row.scope_kind.as_str() {
        "workspace" => row.scope_id == scope.workspace_id,
        "document" => {
            row.document_id.as_deref().is_some_and(|value| !value.trim().is_empty())
                && row.crdt_document_id.as_deref() == Some(row.scope_id.as_str())
        }
        _ => false,
    };
    if row.owner_account_id != scope.owner_account_id
        || row.actor_principal_id != scope.actor_principal_id
        || row.authenticated_session_id != scope.authenticated_session_id
        || row.access_space_id != scope.access_space_id
        || row.workspace_id != scope.workspace_id
        || !document_scope_valid
        || [
            row.lane_id.as_str(),
            row.actor_kind.as_str(),
            row.session_id.as_str(),
            row.correlation_id.as_str(),
            row.scope_id.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT lease mutation requires exact five-field and document/workspace scope",
        });
    }
    Ok(())
}

fn validate_crdt_transition_event(
    identity: &str,
    actor_id: &str,
    ttl_seconds: i64,
    event: &SurrealModelLaneCrdtEventWrite,
) -> Result<(), SurrealStorageError> {
    let payload_bytes = canonical_json_bytes(&event.payload);
    if ttl_seconds <= 0
        || event.aggregate_id != identity
        || event.actor_id != actor_id
        || event.payload_hash != sha256_hex(&payload_bytes)
        || event.event_version != "kernel_event_v1"
        || event.source_component != "model_lane_crdt"
        || [
            identity,
            actor_id,
            event.event_id.as_str(),
            event.event_version.as_str(),
            event.kernel_task_run_id.as_str(),
            event.session_run_id.as_str(),
            event.aggregate_type.as_str(),
            event.idempotency_key.as_str(),
            event.event_type.as_str(),
            event.actor_kind.as_str(),
            event.source_component.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT transition EventLedger linkage is non-canonical",
        });
    }
    Ok(())
}

fn validate_crdt_event_kind(
    event: &SurrealModelLaneCrdtEventWrite,
    aggregate_type: &str,
    event_type: &str,
) -> Result<(), SurrealStorageError> {
    if event.aggregate_type != aggregate_type || event.event_type != event_type {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT mutation EventLedger event kind is non-canonical",
        });
    }
    Ok(())
}

fn validate_crdt_promotion_event_pair(
    proposal_id: &str,
    actor_id: &str,
    requested: &SurrealModelLaneCrdtEventWrite,
    accepted: &SurrealModelLaneCrdtEventWrite,
) -> Result<(), SurrealStorageError> {
    let requested_hash = sha256_hex(&canonical_json_bytes(&requested.payload));
    let accepted_hash = sha256_hex(&canonical_json_bytes(&accepted.payload));
    let canonical = requested.event_version == "kernel_event_v1"
        && accepted.event_version == "kernel_event_v1"
        && requested.event_type == "PROMOTION_REQUESTED"
        && accepted.event_type == "PROMOTION_ACCEPTED"
        && requested.aggregate_type == "knowledge_ai_edit_promotion"
        && accepted.aggregate_type == "knowledge_ai_edit_promotion"
        && requested.aggregate_id == proposal_id
        && accepted.aggregate_id == proposal_id
        && requested.actor_id == actor_id
        && accepted.actor_id == actor_id
        && requested.actor_kind == accepted.actor_kind
        && requested.kernel_task_run_id == accepted.kernel_task_run_id
        && requested.session_run_id == accepted.session_run_id
        && requested.correlation_id == accepted.correlation_id
        && requested.source_component == "knowledge_crdt_ai_edit_proposal"
        && accepted.source_component == "knowledge_crdt_ai_edit_proposal"
        && requested.causation_id.is_none()
        && accepted.causation_id.as_deref() == Some(requested.event_id.as_str())
        && requested.event_id != accepted.event_id
        && requested.idempotency_key != accepted.idempotency_key
        && requested.payload_hash == requested_hash
        && accepted.payload_hash == accepted_hash
        && payload_value_matches(&requested.payload, "proposal_id", proposal_id)
        && payload_value_matches(&accepted.payload, "proposal_id", proposal_id);
    if !canonical {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal promotion EventLedger receipt pair is non-canonical",
        });
    }
    Ok(())
}

fn payload_value_matches(payload: &Value, field: &str, expected: &str) -> bool {
    payload.get(field).and_then(Value::as_str) == Some(expected)
}

fn validate_crdt_proposal_write(
    row: &SurrealModelLaneCrdtProposalWrite,
    event: &SurrealModelLaneCrdtEventWrite,
    scope: &ModelLaneScope,
) -> Result<(), SurrealStorageError> {
    validate_scope(scope)?;
    validate_crdt_transition_event(&row.proposal_id, &row.actor_id, 1, event)?;
    validate_crdt_event_kind(
        event,
        "knowledge_crdt_ai_edit_proposal",
        "AI_EDIT_PROPOSAL_RECORDED",
    )?;
    let diff_bytes = canonical_json_bytes(&row.proposed_diff);
    if row.owner_account_id != scope.owner_account_id
        || row.actor_principal_id != scope.actor_principal_id
        || row.authenticated_session_id != scope.authenticated_session_id
        || row.access_space_id != scope.access_space_id
        || row.workspace_id != scope.workspace_id
        || row.base_update_seq < 0
        || !row.base_state_vector.starts_with("hsk-sv1:")
        || !row.proposed_diff.is_object()
        || row.diff_sha256 != sha256_hex(&diff_bytes)
        || !matches!(row.actor_kind.as_str(), "local_model" | "cloud_model")
        || row.source_span_citations.is_empty()
        || [
            row.document_id.as_str(),
            row.crdt_document_id.as_str(),
            row.session_id.as_str(),
            row.correlation_id.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal mutation requires canonical content and exact five-field scope",
        });
    }
    Ok(())
}

fn proposal_receipt_ids(
    proposal: &SurrealModelLaneCrdtProposalRecord,
) -> Result<Vec<String>, SurrealStorageError> {
    let mut ids = BTreeSet::from([
        proposal.recorded_event_id.clone(),
        proposal.last_transition_event_id.clone(),
    ]);
    for event_id in [
        proposal.decided_event_id.as_ref(),
        proposal.applied_event_id.as_ref(),
        proposal.promotion_requested_event_id.as_ref(),
        proposal.promotion_accepted_event_id.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        ids.insert(event_id.clone());
    }
    let state_is_canonical = match proposal.review_state.as_str() {
        "proposed" => {
            proposal.decided_event_id.is_none()
                && proposal.applied_update_id.is_none()
                && proposal.applied_update_sha256.is_none()
                && proposal.applied_event_id.is_none()
                && proposal.promotion_requested_event_id.is_none()
                && proposal.promotion_accepted_event_id.is_none()
                && proposal.last_transition_event_id == proposal.recorded_event_id
        }
        "approved" => {
            match (
                proposal.decided_event_id.as_ref(),
                proposal.applied_update_id.as_ref(),
                proposal.applied_update_sha256.as_ref(),
                proposal.applied_event_id.as_ref(),
            ) {
                (Some(decided_event_id), None, None, None) => {
                    proposal.last_transition_event_id == *decided_event_id
                }
                (Some(_), Some(_), Some(hash), Some(event_id)) => {
                    hash == &proposal.diff_sha256 && proposal.last_transition_event_id == *event_id
                }
                _ => false,
            }
            && proposal.promotion_requested_event_id.is_none()
                && proposal.promotion_accepted_event_id.is_none()
        }
        "rejected" => {
            proposal
                .decided_event_id
                .as_ref()
                .is_some_and(|event_id| proposal.last_transition_event_id == *event_id)
                && proposal.applied_update_id.is_none()
                && proposal.applied_update_sha256.is_none()
                && proposal.applied_event_id.is_none()
                && proposal.promotion_requested_event_id.is_none()
                && proposal.promotion_accepted_event_id.is_none()
        }
        "promoted" => {
            proposal.decided_event_id.is_some()
                && proposal.applied_update_id.is_some()
                && proposal.applied_update_sha256.as_deref() == Some(proposal.diff_sha256.as_str())
                && proposal.applied_event_id.is_some()
                && proposal.promotion_requested_event_id.is_some()
                && proposal
                    .promotion_accepted_event_id
                    .as_ref()
                    .is_some_and(|event_id| proposal.last_transition_event_id == *event_id)
                && proposal.promotion_requested_event_id != proposal.promotion_accepted_event_id
        }
        _ => false,
    };
    if !state_is_canonical || ids.iter().any(|id| id.trim().is_empty()) {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal receipt state is incomplete or mixed",
        });
    }
    Ok(ids.into_iter().collect())
}

fn validate_crdt_proposal_receipt_rows(
    proposal: &SurrealModelLaneCrdtProposalRecord,
    receipts: Vec<CrdtProposalReceiptRow>,
) -> Result<(), SurrealStorageError> {
    let expected_ids = proposal_receipt_ids(proposal)?;
    let receipts = receipts
        .into_iter()
        .map(|receipt| (receipt.event_id.clone(), receipt))
        .collect::<BTreeMap<_, _>>();
    if receipts.len() != expected_ids.len()
        || expected_ids.iter().any(|id| !receipts.contains_key(id))
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal canonical EventLedger receipt is unavailable in exact scope",
        });
    }
    for receipt in receipts.values() {
        let is_promotion_receipt = matches!(
            receipt.event_type.as_str(),
            "PROMOTION_REQUESTED" | "PROMOTION_ACCEPTED"
        );
        let expected_aggregate_type = if is_promotion_receipt {
            "knowledge_ai_edit_promotion"
        } else {
            "knowledge_crdt_ai_edit_proposal"
        };
        let expected_source_component = if is_promotion_receipt {
            "knowledge_crdt_ai_edit_proposal"
        } else {
            "model_lane_crdt"
        };
        if receipt.event_version != "kernel_event_v1"
            || receipt.aggregate_type != expected_aggregate_type
            || receipt.aggregate_id != proposal.proposal_id
            || receipt.source_component != expected_source_component
            || receipt.correlation_id.as_deref() != Some(proposal.correlation_id.as_str())
            || receipt.payload_hash != sha256_hex(&canonical_json_bytes(&receipt.payload))
            || [
                receipt.event_id.as_str(),
                receipt.kernel_task_run_id.as_str(),
                receipt.session_run_id.as_str(),
                receipt.idempotency_key.as_str(),
                receipt.actor_kind.as_str(),
                receipt.actor_id.as_str(),
            ]
            .into_iter()
            .any(|value| value.trim().is_empty())
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal canonical EventLedger receipt is corrupt",
            });
        }
    }
    let recorded = crdt_proposal_receipt(&receipts, &proposal.recorded_event_id)?;
    if recorded.event_type != "AI_EDIT_PROPOSAL_RECORDED"
        || recorded.actor_id != proposal.actor_id
        || !payload_matches(recorded, "proposal_id", &proposal.proposal_id)
        || !payload_matches(recorded, "diff_sha256", &proposal.diff_sha256)
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal recorded receipt is non-canonical",
        });
    }
    if let Some(event_id) = proposal.decided_event_id.as_ref() {
        let decided = crdt_proposal_receipt(&receipts, event_id)?;
        let expected_state = if proposal.review_state == "rejected" {
            "rejected"
        } else {
            "approved"
        };
        if decided.event_type != "AI_EDIT_PROPOSAL_DECIDED"
            || proposal.decided_by.as_deref() != Some(decided.actor_id.as_str())
            || !payload_matches(decided, "proposal_id", &proposal.proposal_id)
            || !payload_matches(decided, "review_state", expected_state)
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal decision receipt is non-canonical",
            });
        }
    }
    if let Some(event_id) = proposal.applied_event_id.as_ref() {
        let applied = crdt_proposal_receipt(&receipts, event_id)?;
        if applied.event_type != "AI_EDIT_PROPOSAL_DECIDED"
            || applied.actor_id != proposal.actor_id
            || !payload_matches(applied, "proposal_id", &proposal.proposal_id)
            || !payload_matches(
                applied,
                "applied_update_id",
                proposal.applied_update_id.as_deref().ok_or(
                    SurrealStorageError::InvalidModelLaneRecord {
                        reason: "CRDT proposal applied receipt lacks update identity",
                    },
                )?,
            )
            || !payload_matches(applied, "applied_update_sha256", &proposal.diff_sha256)
            || !payload_matches(applied, "approved_diff_sha256", &proposal.diff_sha256)
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal applied-update receipt is non-canonical",
            });
        }
    }
    if proposal.review_state == "promoted" {
        let requested_event_id = proposal.promotion_requested_event_id.as_ref().ok_or(
            SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal promotion requested receipt is missing",
            },
        )?;
        let accepted_event_id = proposal.promotion_accepted_event_id.as_ref().ok_or(
            SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal promotion accepted receipt is missing",
            },
        )?;
        let requested = crdt_proposal_receipt(&receipts, requested_event_id)?;
        let accepted = crdt_proposal_receipt(&receipts, accepted_event_id)?;
        if requested_event_id == accepted_event_id
            || requested.event_type != "PROMOTION_REQUESTED"
            || accepted.event_type != "PROMOTION_ACCEPTED"
            || requested.actor_id != accepted.actor_id
            || requested.actor_kind != accepted.actor_kind
            || requested.kernel_task_run_id != accepted.kernel_task_run_id
            || requested.session_run_id != accepted.session_run_id
            || requested.correlation_id != accepted.correlation_id
            || requested.idempotency_key == accepted.idempotency_key
            || requested.causation_id.is_some()
            || accepted.causation_id.as_deref() != Some(requested_event_id.as_str())
            || !payload_matches(requested, "proposal_id", &proposal.proposal_id)
            || !payload_matches(requested, "diff_sha256", &proposal.diff_sha256)
            || requested
                .payload
                .get("base_update_seq")
                .and_then(Value::as_i64)
                != Some(proposal.base_update_seq)
            || !payload_matches(requested, "base_state_vector", &proposal.base_state_vector)
            || !payload_matches(requested, "decided_by", &requested.actor_id)
            || !payload_matches(accepted, "proposal_id", &proposal.proposal_id)
            || !payload_matches(accepted, "review_state", "promoted")
            || !payload_matches(accepted, "decided_by", &accepted.actor_id)
            || !payload_matches(accepted, "diff_sha256", &proposal.diff_sha256)
            || accepted
                .payload
                .get("applied_update_id")
                .and_then(Value::as_str)
                != proposal.applied_update_id.as_deref()
            || accepted
                .payload
                .get("applied_update_sha256")
                .and_then(Value::as_str)
                != proposal.applied_update_sha256.as_deref()
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT proposal promotion receipt is non-canonical",
            });
        }
    }
    Ok(())
}

fn crdt_proposal_receipt<'a>(
    receipts: &'a BTreeMap<String, CrdtProposalReceiptRow>,
    event_id: &str,
) -> Result<&'a CrdtProposalReceiptRow, SurrealStorageError> {
    receipts
        .get(event_id)
        .ok_or(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal canonical EventLedger receipt is unavailable in exact scope",
        })
}

fn payload_matches(receipt: &CrdtProposalReceiptRow, field: &str, expected: &str) -> bool {
    receipt.payload.get(field).and_then(Value::as_str) == Some(expected)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod routing_provider_contract_tests {
    use super::*;
    use serde_json::json;

    fn scope() -> ModelLaneScope {
        ModelLaneScope {
            owner_account_id: "account-a".into(),
            actor_principal_id: "actor-a".into(),
            authenticated_session_id: "session-a".into(),
            access_space_id: "space-a".into(),
            workspace_id: "workspace-a".into(),
        }
    }

    fn event() -> SurrealModelLaneRoutingEventWrite {
        let payload = json!({"a": 1, "b": 2});
        SurrealModelLaneRoutingEventWrite {
            event_id: "nondeterministic-input-id".into(),
            event_version: "kernel_event_v1".into(),
            kernel_task_run_id: "run-a".into(),
            session_run_id: "execution-a".into(),
            aggregate_type: "model_lane_routing_execution".into(),
            aggregate_id: "execution-a".into(),
            idempotency_key: "logical-revision-1".into(),
            event_type: "MODEL_LANE_ROUTING_EXECUTION_COMMITTED".into(),
            actor_kind: "principal".into(),
            actor_id: "actor-a".into(),
            causation_id: Some("cause-a".into()),
            correlation_id: Some("correlation-a".into()),
            payload_hash: canonical_routing_payload_hash(&payload),
            source_component: "routing_execution".into(),
            payload,
            created_at: Utc::now(),
        }
    }

    fn scoped_identity(mut event: SurrealModelLaneRoutingEventWrite, scope: &ModelLaneScope) -> (String, String) {
        scope_routing_event_identities(std::slice::from_mut(&mut event), scope);
        (event.event_id, event.idempotency_key)
    }

    #[test]
    fn routing_event_identity_changes_for_each_scope_field() {
        let baseline_scope = scope();
        let baseline = scoped_identity(event(), &baseline_scope);
        assert_eq!(baseline, scoped_identity(event(), &baseline_scope));

        let mut counterfactuals = Vec::new();
        let mut changed = baseline_scope.clone();
        changed.owner_account_id = "account-b".into();
        counterfactuals.push(changed);
        let mut changed = baseline_scope.clone();
        changed.actor_principal_id = "actor-b".into();
        counterfactuals.push(changed);
        let mut changed = baseline_scope.clone();
        changed.authenticated_session_id = "session-b".into();
        counterfactuals.push(changed);
        let mut changed = baseline_scope.clone();
        changed.access_space_id = "space-b".into();
        counterfactuals.push(changed);
        let mut changed = baseline_scope;
        changed.workspace_id = "workspace-b".into();
        counterfactuals.push(changed);

        for changed_scope in counterfactuals {
            let changed_identity = scoped_identity(event(), &changed_scope);
            assert_ne!(baseline.0, changed_identity.0);
            assert_ne!(baseline.1, changed_identity.1);
        }
    }

    #[test]
    fn valid_looking_mismatched_payload_hash_is_rejected_by_canonical_check() {
        let mut event = event();
        event.payload_hash = "0".repeat(64);
        assert!(!routing_event_payload_hash_is_canonical(&event));
        event.payload_hash = canonical_routing_payload_hash(&event.payload);
        assert!(routing_event_payload_hash_is_canonical(&event));
    }

    #[test]
    fn routing_query_fences_claim_receipts_and_readbacks() {
        assert!(ROUTING_COMMIT_QUERY.contains(
            "event_ledger_event_id.aggregate_type = 'model_lane_routing_stage_attempt'"
        ));
        assert!(ROUTING_COMMIT_QUERY.contains(
            "event_ledger_event_id.aggregate_id = attempt_id"
        ));
        assert!(
            ROUTING_COMMIT_QUERY
                .matches("event_ledger_seq = event_ledger_event_id.event_sequence")
                .count()
                >= 8
        );
        for field in [
            "linked_event_version",
            "linked_kernel_task_run_id",
            "linked_session_run_id",
            "linked_aggregate_type",
            "linked_aggregate_id",
            "linked_idempotency_key",
            "linked_event_type",
            "linked_actor_kind",
            "linked_actor_id",
            "linked_causation_id",
            "linked_correlation_id",
            "linked_payload_hash",
            "linked_source_component",
            "linked_payload",
        ] {
            assert!(ROUTING_COMMIT_QUERY.contains(field));
        }
    }

    #[test]
    fn incomplete_immutable_context_cannot_produce_a_context_hash() {
        assert!(canonical_routing_context_hash(&json!({})).is_err());
    }

    #[test]
    fn crdt_proposal_message_guard_fences_canonical_receipts_and_hash_domains() {
        for predicate in [
            "recorded_event_id.event_type = 'AI_EDIT_PROPOSAL_RECORDED'",
            "decided_event_id.event_type = 'AI_EDIT_PROPOSAL_DECIDED'",
            "applied_event_id.payload.applied_update_sha256 = diff_sha256",
            "applied_event_id.payload.yjs_update_sha256 = $crdt_update_sha256",
            "promotion_requested_event_id.event_type = 'PROMOTION_REQUESTED'",
            "promotion_accepted_event_id.event_type = 'PROMOTION_ACCEPTED'",
            "promotion_requested_event_id.aggregate_type = 'knowledge_ai_edit_promotion'",
            "promotion_accepted_event_id.aggregate_type = 'knowledge_ai_edit_promotion'",
            "promotion_accepted_event_id.causation_id = record::id(promotion_requested_event_id)",
            "last_transition_event_id = applied_event_id",
        ] {
            assert!(
                GUARDED_MESSAGE_QUERY.contains(predicate),
                "missing {predicate}"
            );
        }
    }

    #[test]
    fn crdt_proposal_promotion_is_one_atomic_distinct_causation_linked_pair() {
        for predicate in [
            "BEGIN TRANSACTION",
            "$current[0].promotion_requested_event_id = $event.event_id",
            "$current[0].promotion_accepted_event_id = $promotion_accepted_event.event_id",
            "promotion_requested_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $event.event_id) }",
            "promotion_accepted_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $promotion_accepted_event.event_id) }",
            "last_transition_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $promotion_accepted_event.event_id) }",
            "COMMIT TRANSACTION",
        ] {
            assert!(
                DECIDE_CRDT_PROPOSAL_QUERY.contains(predicate),
                "missing {predicate}"
            );
        }
    }

    #[test]
    fn crdt_proposal_binding_requires_next_update_from_approved_base() {
        assert!(BIND_CRDT_PROPOSAL_UPDATE_QUERY
            .contains("update_seq = $proposal[0].base_update_seq + 1"));
        assert!(BIND_CRDT_PROPOSAL_UPDATE_QUERY
            .contains("state_vector_before = $proposal[0].base_state_vector"));
        assert!(BIND_CRDT_PROPOSAL_UPDATE_QUERY.contains("$expected_actor_id"));
    }
}
