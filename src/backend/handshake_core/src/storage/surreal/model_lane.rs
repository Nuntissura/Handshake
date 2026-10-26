use std::collections::{BTreeMap, BTreeSet};

use super::{SurrealStorage, SurrealStorageError};
use crate::kernel::context_bundle::canonical_json_bytes;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::SurrealValue;

const SCHEMA: &str = include_str!("model_lane_schema.surql");
pub(super) const SCHEMA_STATE: &str = "\
DEFINE TABLE IF NOT EXISTS model_lane_schema_state SCHEMAFULL;\
DEFINE FIELD IF NOT EXISTS schema_version ON model_lane_schema_state TYPE string;\
DEFINE FIELD IF NOT EXISTS schema_revision ON model_lane_schema_state TYPE int;\
DEFINE FIELD IF NOT EXISTS apply_state ON model_lane_schema_state TYPE string;";
const SCHEMA_STATE_ID: &str = "model_lane_schema_state:primary";
const SCHEMA_VERSION: &str = "mt003-model-lane-authority-v10";
const SCHEMA_REVISION: i64 = 10;
const ROUTING_PREVIOUS_SCHEMA_VERSION: &str = "mt003-model-lane-authority-v9";
const ROUTING_PREVIOUS_SCHEMA_REVISION: i64 = 9;
const RUN_MEMORY_PREVIOUS_SCHEMA_VERSION: &str = "mt003-model-lane-authority-v8";
const RUN_MEMORY_PREVIOUS_SCHEMA_REVISION: i64 = 8;
const NAVIGATION_PREVIOUS_SCHEMA_VERSION: &str = "mt003-model-lane-authority-v7";
const NAVIGATION_PREVIOUS_SCHEMA_REVISION: i64 = 7;
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
  LET $first_ledger = CREATE type::record('kernel_event_ledger', $first_event_id) CONTENT { event_id: $first_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $first_run_id, session_run_id: IF $first_event_ledger_stream_id = NONE { $first_run_id } ELSE { $first_event_ledger_stream_id }, aggregate_type: $first_record_kind, aggregate_id: $first_aggregate_id, idempotency_key: $first_event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $first_event_payload_hash, source_component: 'model_lane', payload: { record_kind: $first_record_kind, run_id: $first_run_id, event_stream_version: 1, event_payload_json: $first_event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
  LET $first_seq = $first_ledger[0].event_sequence;\
  LET $first_row = CREATE type::record('model_lane_authority', $first_record_id) CONTENT { record_kind: $first_record_kind, aggregate_id: $first_aggregate_id, run_id: $first_run_id, idempotency_key: $first_idempotency_key, record_json: $first_record_json, search_terms: $first_search_terms, event_id: $first_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $first_event_id), event_seq: $first_seq, event_stream_version: 1, transaction_seq: $first_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
  LET $second_ledger = CREATE type::record('kernel_event_ledger', $second_event_id) CONTENT { event_id: $second_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $second_run_id, session_run_id: IF $second_event_ledger_stream_id = NONE { $second_run_id } ELSE { $second_event_ledger_stream_id }, aggregate_type: $second_record_kind, aggregate_id: $second_aggregate_id, idempotency_key: $second_event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $first_event_id, correlation_id: NONE, payload_hash: $second_event_payload_hash, source_component: 'model_lane', payload: { record_kind: $second_record_kind, run_id: $second_run_id, event_stream_version: 1, event_payload_json: $second_event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
  LET $second_seq = $second_ledger[0].event_sequence;\
  LET $second_row = CREATE type::record('model_lane_authority', $second_record_id) CONTENT { record_kind: $second_record_kind, aggregate_id: $second_aggregate_id, run_id: $second_run_id, idempotency_key: $second_idempotency_key, record_json: $second_record_json, search_terms: $second_search_terms, event_id: $second_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $second_event_id), event_seq: $second_seq, event_stream_version: 1, transaction_seq: $second_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
    RETURN array::concat($first_row, $second_row);\
  };\
};\
COMMIT TRANSACTION;";
const GUARDED_MESSAGE_QUERY: &str = "\
BEGIN TRANSACTION;\
  LET $existing_message = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $message_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
IF array::len($existing_message) = 1 {\
    IF $has_payload_binding {\
      LET $existing_binding = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $binding_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
      IF array::len($existing_binding) != 1 { THROW 'model-lane message payload binding is partially present'; };\
      RETURN array::concat($existing_message, $existing_binding);\
    } ELSE { RETURN $existing_message; };\
  } ELSE {\
    IF !fn::wp012_model_lane_crdt_admission({ record_json: $message_record_json, run_id: $message_run_id, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }) { THROW 'model-lane CRDT admission fence changed before publication'; };\
    LET $source = (SELECT VALUE id FROM type::record('model_lane_authority', $source_lane_record_id) WHERE record_kind = 'lane' AND record_json = $source_lane_record_json AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
    LET $session_owners = (SELECT VALUE id FROM model_lane_authority WHERE record_kind = 'lane' AND ($source_session_term IN search_terms OR $source_model_session_term IN search_terms) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
    LET $promotion = IF $has_promotion { (SELECT VALUE id FROM type::record('model_lane_authority', $promotion_record_id) WHERE record_kind = 'promotion_decision' AND record_json = $promotion_record_json AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id) } ELSE { [true] };\
    LET $update = IF $has_crdt { (SELECT VALUE id FROM kernel_crdt_updates WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id AND update_bytes_ref = $crdt_update_ref AND update_id = $crdt_update_id AND update_sha256 = $crdt_update_sha256 AND state_vector_after = $crdt_state_vector AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.event_type = 'KNOWLEDGE_CRDT_UPDATE_RECORDED' AND event_ledger_event_id.aggregate_type = 'knowledge_crdt_document' AND event_ledger_event_id.aggregate_id = crdt_document_id AND event_ledger_event_id.actor_id = actor_id AND event_ledger_event_id.correlation_id = trace_id AND event_ledger_event_id.payload.update_id = update_id AND event_ledger_event_id.payload.update_sha256 = update_sha256 AND event_ledger_event_id.payload.state_vector_after = state_vector_after LIMIT 2) } ELSE { [true] };\
    LET $snapshot = IF $has_crdt { (SELECT VALUE id FROM kernel_crdt_snapshots WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id AND snapshot_bytes_ref = $crdt_snapshot_ref AND snapshot_id = $crdt_snapshot_id AND snapshot_sha256 = $crdt_snapshot_sha256 AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.event_type = 'KNOWLEDGE_CRDT_SNAPSHOT_RECORDED' AND event_ledger_event_id.aggregate_type = 'knowledge_crdt_document' AND event_ledger_event_id.aggregate_id = crdt_document_id AND event_ledger_event_id.actor_id = actor_id AND event_ledger_event_id.payload.snapshot_id = snapshot_id AND event_ledger_event_id.payload.snapshot_sha256 = snapshot_sha256 AND event_ledger_event_id.payload.state_vector = state_vector LIMIT 2) } ELSE { [true] };\
    LET $lease = IF $has_crdt { (SELECT VALUE id FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND lease_id = $crdt_lease_id AND lane_id = $source_lane_id AND actor_id = $crdt_actor_id AND actor_kind = $crdt_actor_kind AND session_id = $crdt_session_id AND correlation_id = $crdt_trace_id AND scope_kind = $crdt_lease_scope_kind AND scope_id = $crdt_lease_scope_id AND ((scope_kind = 'workspace' AND scope_id = $workspace_id) OR (scope_kind = 'document' AND scope_id = $crdt_crdt_document_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id)) AND claimed_at_utc = $crdt_lease_claimed_at_utc AND expires_at_utc >= $crdt_lease_expires_at_utc AND claimed_at_utc <= $crdt_lease_admitted_at_utc AND expires_at_utc > $crdt_lease_admitted_at_utc AND released_at_utc IS NONE AND expired_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND recorded_event_id.event_type = 'KNOWLEDGE_CRDT_LEASE_CLAIMED' AND recorded_event_id.aggregate_type = 'knowledge_crdt_lease' AND recorded_event_id.aggregate_id = lease_id AND recorded_event_id.actor_id = actor_id AND recorded_event_id.correlation_id = correlation_id AND recorded_event_id.payload.lease_id = lease_id AND recorded_event_id.payload.actor_id = actor_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id AND last_transition_event_id.event_type IN ['KNOWLEDGE_CRDT_LEASE_CLAIMED', 'KNOWLEDGE_CRDT_LEASE_RENEWED'] AND last_transition_event_id.aggregate_type = 'knowledge_crdt_lease' AND last_transition_event_id.aggregate_id = lease_id AND last_transition_event_id.actor_id = actor_id AND last_transition_event_id.correlation_id = correlation_id AND last_transition_event_id.payload.lease_id = lease_id AND last_transition_event_id.payload.actor_id = actor_id LIMIT 2) } ELSE { [true] };\
    LET $proposal = IF $has_crdt_proposal { (SELECT VALUE id FROM knowledge_crdt_ai_edit_proposals WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND proposal_id = $crdt_proposal_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id AND actor_id = $crdt_actor_id AND actor_kind = $crdt_actor_kind AND session_id = $crdt_session_id AND correlation_id = $crdt_trace_id AND review_state IN ['approved', 'promoted'] AND diff_sha256 = $crdt_proposal_diff_sha256 AND applied_update_id = $crdt_update_id AND applied_update_sha256 = $crdt_proposal_diff_sha256 AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND recorded_event_id.event_type = 'AI_EDIT_PROPOSAL_RECORDED' AND recorded_event_id.aggregate_type = 'knowledge_crdt_ai_edit_proposal' AND recorded_event_id.aggregate_id = proposal_id AND recorded_event_id.actor_id = actor_id AND recorded_event_id.correlation_id = correlation_id AND recorded_event_id.payload.proposal_id = proposal_id AND recorded_event_id.payload.diff_sha256 = diff_sha256 AND decided_event_id.owner_account_id = $owner_account_id AND decided_event_id.actor_principal_id = $actor_principal_id AND decided_event_id.authenticated_session_id = $authenticated_session_id AND decided_event_id.access_space_id = $access_space_id AND decided_event_id.workspace_id = $workspace_id AND decided_event_id.event_type = 'AI_EDIT_PROPOSAL_DECIDED' AND decided_event_id.aggregate_type = 'knowledge_crdt_ai_edit_proposal' AND decided_event_id.aggregate_id = proposal_id AND decided_event_id.payload.proposal_id = proposal_id AND decided_event_id.payload.review_state = 'approved' AND decided_event_id.actor_kind = 'operator' AND decided_event_id.actor_id = decided_by AND decided_event_id.payload.reviewer_principal_id = decided_event_id.actor_principal_id AND decided_event_id.payload.reviewer_authenticated_session_id = decided_event_id.authenticated_session_id AND decided_event_id.session_run_id = decided_event_id.authenticated_session_id AND decided_event_id.payload.reviewer_resource_id = record::id(decided_event_id.authority_resource_id) AND decided_event_id.payload.reviewer_grant_id = record::id(decided_event_id.authority_grant_id) AND applied_event_id.owner_account_id = $owner_account_id AND applied_event_id.actor_principal_id = $actor_principal_id AND applied_event_id.authenticated_session_id = $authenticated_session_id AND applied_event_id.access_space_id = $access_space_id AND applied_event_id.workspace_id = $workspace_id AND applied_event_id.event_type = 'AI_EDIT_PROPOSAL_DECIDED' AND applied_event_id.aggregate_type = 'knowledge_crdt_ai_edit_proposal' AND applied_event_id.aggregate_id = proposal_id AND applied_event_id.actor_id = actor_id AND applied_event_id.correlation_id = correlation_id AND applied_event_id.payload.proposal_id = proposal_id AND applied_event_id.payload.applied_update_id = applied_update_id AND applied_event_id.payload.applied_update_sha256 = diff_sha256 AND applied_event_id.payload.approved_diff_sha256 = diff_sha256 AND applied_event_id.payload.yjs_update_sha256 = $crdt_update_sha256 AND ((review_state = 'approved' AND last_transition_event_id = applied_event_id) OR (review_state = 'promoted' AND promotion_requested_event_id != promotion_accepted_event_id AND last_transition_event_id = promotion_accepted_event_id AND promotion_requested_event_id.owner_account_id = $owner_account_id AND promotion_requested_event_id.actor_principal_id = $actor_principal_id AND promotion_requested_event_id.authenticated_session_id = $authenticated_session_id AND promotion_requested_event_id.access_space_id = $access_space_id AND promotion_requested_event_id.workspace_id = $workspace_id AND promotion_requested_event_id.event_type = 'PROMOTION_REQUESTED' AND promotion_requested_event_id.aggregate_type = 'knowledge_ai_edit_promotion' AND promotion_requested_event_id.aggregate_id = proposal_id AND promotion_requested_event_id.source_component = 'knowledge_crdt_ai_edit_proposal' AND promotion_requested_event_id.correlation_id = correlation_id AND promotion_requested_event_id.payload.proposal_id = proposal_id AND promotion_requested_event_id.payload.diff_sha256 = diff_sha256 AND promotion_requested_event_id.payload.base_update_seq = base_update_seq AND promotion_requested_event_id.payload.base_state_vector = base_state_vector AND promotion_requested_event_id.payload.decided_by = promotion_requested_event_id.actor_id AND promotion_accepted_event_id.owner_account_id = $owner_account_id AND promotion_accepted_event_id.actor_principal_id = $actor_principal_id AND promotion_accepted_event_id.authenticated_session_id = $authenticated_session_id AND promotion_accepted_event_id.access_space_id = $access_space_id AND promotion_accepted_event_id.workspace_id = $workspace_id AND promotion_accepted_event_id.event_type = 'PROMOTION_ACCEPTED' AND promotion_accepted_event_id.aggregate_type = 'knowledge_ai_edit_promotion' AND promotion_accepted_event_id.aggregate_id = proposal_id AND promotion_accepted_event_id.source_component = 'knowledge_crdt_ai_edit_proposal' AND promotion_accepted_event_id.actor_id = promotion_requested_event_id.actor_id AND promotion_accepted_event_id.correlation_id = correlation_id AND promotion_accepted_event_id.causation_id = record::id(promotion_requested_event_id) AND promotion_accepted_event_id.payload.proposal_id = proposal_id AND promotion_accepted_event_id.payload.review_state = 'promoted' AND promotion_accepted_event_id.payload.decided_by = promotion_accepted_event_id.actor_id AND promotion_accepted_event_id.payload.diff_sha256 = diff_sha256 AND promotion_accepted_event_id.payload.applied_update_id = applied_update_id AND promotion_accepted_event_id.payload.applied_update_sha256 = applied_update_sha256)) AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };\
    LET $covering = IF $has_crdt { (SELECT VALUE id FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND claimed_at_utc <= $crdt_lease_admitted_at_utc AND expires_at_utc > $crdt_lease_admitted_at_utc AND released_at_utc IS NONE AND expired_at_utc IS NONE AND ((scope_kind = 'workspace' AND scope_id = $workspace_id) OR (scope_kind = 'document' AND scope_id = $crdt_crdt_document_id AND document_id = $crdt_document_id AND crdt_document_id = $crdt_crdt_document_id))) } ELSE { [true] };\
    IF array::len($source) != 1 OR array::len($session_owners) != 1 OR array::len($promotion) != 1 OR array::len($update) != 1 OR array::len($snapshot) != 1 OR array::len($lease) != 1 OR array::len($covering) != 1 OR array::len($proposal) != 1 { THROW 'model-lane message authority changed before admission'; };\
    LET $message_ledger = CREATE type::record('kernel_event_ledger', $message_event_id) CONTENT { event_id: $message_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $message_run_id, session_run_id: IF $message_event_ledger_stream_id = NONE { $message_run_id } ELSE { $message_event_ledger_stream_id }, aggregate_type: 'message', aggregate_id: $message_aggregate_id, idempotency_key: $message_event_id, event_type: 'MODEL_LANE_MESSAGE_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $message_event_payload_hash, source_component: 'model_lane', payload: { record_kind: 'message', run_id: $message_run_id, event_stream_version: 1, event_payload_json: $message_event_payload_json }, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
    LET $message_seq = $message_ledger[0].event_sequence;\
    LET $message = CREATE type::record('model_lane_authority', $message_record_id) CONTENT { record_kind: 'message', aggregate_id: $message_aggregate_id, run_id: $message_run_id, idempotency_key: $message_idempotency_key, record_json: $message_record_json, search_terms: $message_search_terms, event_id: $message_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $message_event_id), event_seq: $message_seq, event_stream_version: 1, transaction_seq: $message_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
    IF $has_payload_binding {\
      LET $binding_ledger = CREATE type::record('kernel_event_ledger', $binding_event_id) CONTENT { event_id: $binding_event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $binding_run_id, session_run_id: IF $binding_event_ledger_stream_id = NONE { $binding_run_id } ELSE { $binding_event_ledger_stream_id }, aggregate_type: $binding_record_kind, aggregate_id: $binding_aggregate_id, idempotency_key: $binding_event_id, event_type: 'MODEL_LANE_PAYLOAD_BOUND', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $message_event_id, correlation_id: NONE, payload_hash: $binding_event_payload_hash, source_component: 'model_lane', payload: { record_kind: $binding_record_kind, run_id: $binding_run_id, event_stream_version: 1, event_payload_json: $binding_event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
      LET $binding_seq = $binding_ledger[0].event_sequence;\
      LET $binding = CREATE type::record('model_lane_authority', $binding_record_id) CONTENT { record_kind: $binding_record_kind, aggregate_id: $binding_aggregate_id, run_id: $binding_run_id, idempotency_key: $binding_idempotency_key, record_json: $binding_record_json, search_terms: $binding_search_terms, event_id: $binding_event_id, event_ledger_event_id: type::record('kernel_event_ledger', $binding_event_id), event_seq: $binding_seq, event_stream_version: 1, transaction_seq: $binding_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
      RETURN array::concat($message, $binding);\
    } ELSE { RETURN $message; };\
  };\
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
        LET $message_ledger = CREATE type::record('kernel_event_ledger', $message.event_id) CONTENT { event_id: $message.event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $message.run_id, session_run_id: $message.event_ledger_stream_id, aggregate_type: 'message', aggregate_id: $message.aggregate_id, idempotency_key: $message.event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $message.event_payload_hash, source_component: 'model_lane', payload: { record_kind: 'message', run_id: $message.run_id, event_stream_version: 1, event_payload_json: $message.event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
        LET $stored_message = CREATE type::record('model_lane_authority', $message.record_id) CONTENT { record_kind: 'message', aggregate_id: $message.aggregate_id, run_id: $message.run_id, idempotency_key: $message.idempotency_key, record_json: $message.record_json, search_terms: $message.search_terms, event_id: $message.event_id, event_ledger_event_id: type::record('kernel_event_ledger', $message.event_id), event_seq: $message_ledger[0].event_sequence, event_stream_version: 1, transaction_seq: $message_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    };
    IF $has_binding {
        LET $binding_ledger = CREATE type::record('kernel_event_ledger', $binding.event_id) CONTENT { event_id: $binding.event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $binding.run_id, session_run_id: $binding.event_ledger_stream_id, aggregate_type: $binding.record_kind, aggregate_id: $binding.aggregate_id, idempotency_key: $binding.event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $binding.event_payload_hash, source_component: 'model_lane', payload: { record_kind: $binding.record_kind, run_id: $binding.run_id, event_stream_version: 1, event_payload_json: $binding.event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
        LET $stored_binding = CREATE type::record('model_lane_authority', $binding.record_id) CONTENT { record_kind: $binding.record_kind, aggregate_id: $binding.aggregate_id, run_id: $binding.run_id, idempotency_key: $binding.idempotency_key, record_json: $binding.record_json, search_terms: $binding.search_terms, event_id: $binding.event_id, event_ledger_event_id: type::record('kernel_event_ledger', $binding.event_id), event_seq: $binding_ledger[0].event_sequence, event_stream_version: 1, transaction_seq: $binding_ledger[0].event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    };
    SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM type::record('model_lane_routing_execution', $execution_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $execution.execution_id AND event_ledger_seq = event_ledger_event_id.event_sequence
};
RETURN SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM type::record('model_lane_routing_execution', $execution_record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $execution.execution_id AND event_ledger_seq = event_ledger_event_id.event_sequence;
COMMIT TRANSACTION;"#;
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
} ELSE IF $corruption = 'incomplete_attribution' {
    UPDATE $row[0] SET authenticated_session_id = 'legacy-unattributed', access_space_id = 'legacy-unattributed';
} ELSE IF $corruption = 'blank_attribution' {
    UPDATE $row[0] SET owner_account_id = '';
} ELSE {
    THROW 'unsupported ModelLane authority test corruption';
};
COMMIT TRANSACTION;
"#;
const RECORD_RECOVERY_CHECKPOINT_QUERY: &str = r#"
BEGIN TRANSACTION;
    LET $existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
IF array::len($existing) = 1 {
        RETURN $existing;
    } ELSE {
        IF array::len($existing) != 0 { THROW 'model-lane recovery checkpoint identity is ambiguous'; };
        LET $run = (SELECT VALUE id FROM type::record('model_lane_authority', $run_record_id) WHERE record_kind = 'run' AND aggregate_id = $run_id AND run_id = $run_id AND encoding::json::decode(record_json).event_ledger_stream_id = $event_ledger_stream_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
        LET $lane = IF $has_lane { (SELECT VALUE id FROM type::record('model_lane_authority', $lane_record_id) WHERE record_kind = 'lane' AND aggregate_id = $lane_id AND run_id = $run_id AND encoding::json::decode(record_json).event_ledger_stream_id = $event_ledger_stream_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };
        LET $head = (SELECT event_sequence AS event_seq FROM kernel_event_ledger WHERE source_component = 'model_lane' AND kernel_task_run_id = $run_id AND session_run_id = $event_ledger_stream_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_sequence DESC LIMIT 1);
        IF array::len($run) != 1 OR array::len($lane) != 1 OR array::len($head) != 1 OR $head[0].event_seq != $expected_last_event_seq { THROW 'model-lane recovery checkpoint authority or high watermark changed'; };
        LET $membership = fn::wp012_cloud_checkpoint_denial_membership($run_id, $event_ledger_stream_id, $expected_last_event_seq, $aggregate_id, $owner_account_id, $actor_principal_id, $authenticated_session_id, $access_space_id, $workspace_id);
        LET $checkpoint_record = object::extend(encoding::json::decode($record_json), { cloud_consent_denial_membership: $membership });
        LET $checkpoint_record_json = encoding::json::encode($checkpoint_record);
        LET $checkpoint_payload = object::extend(encoding::json::decode($event_payload_json), { record: $checkpoint_record });
        LET $checkpoint_payload_json = encoding::json::encode($checkpoint_payload);
        LET $checkpoint_payload_hash = crypto::sha256($checkpoint_payload_json);
        LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: IF $event_ledger_stream_id = NONE { $run_id } ELSE { $event_ledger_stream_id }, aggregate_type: 'recovery_checkpoint', aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_RECOVERY_CHECKPOINT_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $checkpoint_payload_hash, source_component: 'model_lane', payload: { record_kind: 'recovery_checkpoint', run_id: $run_id, event_stream_version: 1, event_payload_json: $checkpoint_payload_json }, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
        LET $seq = $ledger[0].event_sequence;
        RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: 'recovery_checkpoint', aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $checkpoint_record_json, search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }
    };
COMMIT TRANSACTION;
"#;
const RECORD_RECOVERY_EVENT_QUERY: &str = r#"
BEGIN TRANSACTION;
    LET $existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
IF array::len($existing) = 1 {
        RETURN $existing;
    } ELSE {
        IF array::len($existing) != 0 { THROW 'model-lane recovery event identity is ambiguous'; };
        LET $run = (SELECT VALUE id FROM type::record('model_lane_authority', $run_record_id) WHERE record_kind = 'run' AND aggregate_id = $run_id AND run_id = $run_id AND encoding::json::decode(record_json).event_ledger_stream_id = $event_ledger_stream_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
        LET $lane = IF $has_lane { (SELECT VALUE id FROM type::record('model_lane_authority', $lane_record_id) WHERE record_kind = 'lane' AND aggregate_id = $lane_id AND run_id = $run_id AND encoding::json::decode(record_json).event_ledger_stream_id = $event_ledger_stream_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };
        LET $source = IF $has_source_event { (SELECT VALUE id FROM kernel_event_ledger WHERE source_component = 'model_lane' AND kernel_task_run_id = $run_id AND session_run_id = $event_ledger_stream_id AND event_sequence = $source_event_seq AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2) } ELSE { [true] };
        LET $order = (SELECT next_value FROM type::record('model_lane_recovery_order', $order_record_id) WHERE run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
        LET $current_order = IF array::len($order) = 0 { 0 } ELSE { $order[0].next_value };
        IF array::len($run) != 1 OR array::len($lane) != 1 OR array::len($source) != 1 { THROW 'model-lane recovery event authority changed'; };
        IF $expected_replay_order_seq != $current_order + 1 {
            RETURN [];
        } ELSE {
            LET $order_counter = UPSERT type::record('model_lane_recovery_order', $order_record_id) CONTENT { run_id: $run_id, next_value: $expected_replay_order_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
            LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: IF $event_ledger_stream_id = NONE { $run_id } ELSE { $event_ledger_stream_id }, aggregate_type: 'recovery_event', aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_RECOVERY_EVENT_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: 'recovery_event', run_id: $run_id, event_stream_version: 1, source_event_seq: IF $has_source_event { $source_event_seq } ELSE { NONE }, replay_order_seq: $expected_replay_order_seq, event_payload_json: $event_payload_json }, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
            LET $seq = $ledger[0].event_sequence;
            RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: 'recovery_event', aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $record_json, search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, ordering_seq: $expected_replay_order_seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }
        }
    };
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
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$row.workspace_id], owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
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
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$row.workspace_id], owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
    LET $created = CREATE type::record('kernel_crdt_snapshots', $record_id) CONTENT { schema_id: $row.schema_id, snapshot_id: $row.snapshot_id, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id, document_id: $row.document_id, crdt_document_id: $row.crdt_document_id, covered_update_seq: $row.covered_update_seq, state_vector: $row.state_vector, snapshot_sha256: $row.snapshot_sha256, snapshot_bytes_ref: $row.snapshot_bytes_ref, snapshot_bytes: encoding::base64::decode($row.snapshot_bytes_b64), actor_id: $row.actor_id, actor_kind: $row.actor_kind, event_ledger_stream_id: $row.event_ledger_stream_id, event_ledger_event_id: type::record('kernel_event_ledger', $event.event_id), promotion_evidence_update_ids: $row.promotion_evidence_update_ids, storage_authority: 'embedded_surrealdb' };
    RETURN SELECT schema_id, snapshot_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq, state_vector, snapshot_sha256, snapshot_bytes_ref, encoding::base64::encode(snapshot_bytes) AS snapshot_bytes_b64, actor_id, actor_kind, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, promotion_evidence_update_ids, storage_authority, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.document_id AS ledger_document_id, event_ledger_event_id.payload.state_vector AS ledger_state_vector, event_ledger_event_id.payload.covered_update_seq AS ledger_covered_update_seq FROM type::record('kernel_crdt_snapshots', $record_id) WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND event_ledger_event_id.owner_account_id = $row.owner_account_id AND event_ledger_event_id.actor_principal_id = $row.actor_principal_id AND event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id AND event_ledger_event_id.access_space_id = $row.access_space_id AND event_ledger_event_id.workspace_id = $row.workspace_id;
};
COMMIT TRANSACTION;
"#;
const CLAIM_CRDT_LEASE_QUERY: &str = r#"

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
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$row.workspace_id], owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
    LET $created = CREATE type::record('knowledge_crdt_agent_lane_leases', $record_id) CONTENT { authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, authority_subject_kind: $wp012_original_subject_kind, authority_subject_id: $wp012_original_subject_id, transition_revision: 1, lease_id: $row.lease_id, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id, document_id: $row.document_id, crdt_document_id: $row.crdt_document_id, lane_id: $row.lane_id, actor_id: $row.actor_id, actor_kind: $row.actor_kind, session_id: $row.session_id, correlation_id: $row.correlation_id, scope_kind: $row.scope_kind, scope_id: $row.scope_id, claimed_at_utc: time::now(), expires_at_utc: time::now() + duration::from_secs($ttl_seconds), renewal_count: 0, released_at_utc: NONE, expired_at_utc: NONE, takeover_of: IF $row.takeover_of = NONE { NONE } ELSE { type::record('knowledge_crdt_agent_lane_leases', $row.takeover_of) }, recorded_event_id: type::record('kernel_event_ledger', $event.event_id), last_transition_event_id: type::record('kernel_event_ledger', $event.event_id) };
    LET $stored = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM type::record('knowledge_crdt_agent_lane_leases', $record_id) WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id);
    RETURN [{ outcome: 'claimed', record: $stored[0] }];
};

"#;
const RENEW_CRDT_LEASE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $transition_at = time::now();
LET $current = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc IS NONE AND expired_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
IF array::len($current) > 1 { THROW 'model-lane CRDT lease identity is ambiguous'; } ELSE IF array::len($current) = 0 { RETURN []; } ELSE IF $current[0].last_transition_event_id = $event.event_id { RETURN $current; } ELSE {
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT renewal event exists without lease transition'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $updated = UPDATE knowledge_crdt_agent_lane_leases SET transition_revision += 1, expires_at_utc = $transition_at + duration::from_secs($ttl_seconds), renewal_count += 1, last_transition_event_id = type::record('kernel_event_ledger', $event.event_id) WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
COMMIT TRANSACTION;
"#;
const RELEASE_CRDT_LEASE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $transition_at = time::now();
LET $current = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND released_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
IF array::len($current) > 1 { THROW 'model-lane CRDT lease identity is ambiguous'; } ELSE IF array::len($current) = 0 { RETURN []; } ELSE IF $current[0].last_transition_event_id = $event.event_id { RETURN $current; } ELSE {
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT release event exists without lease transition'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $updated = UPDATE knowledge_crdt_agent_lane_leases SET transition_revision += 1, released_at_utc = $transition_at, last_transition_event_id = type::record('kernel_event_ledger', $event.event_id) WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
COMMIT TRANSACTION;
"#;
const EXPIRE_CRDT_LEASE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $transition_at = time::now();
LET $current = (SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND released_at_utc IS NONE AND expired_at_utc IS NONE AND expires_at_utc <= time::now() AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
IF array::len($current) > 1 { THROW 'model-lane CRDT lease identity is ambiguous'; } ELSE IF array::len($current) = 0 { RETURN []; } ELSE IF $current[0].last_transition_event_id = $event.event_id { RETURN $current; } ELSE {
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT expiry event exists without lease transition'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $updated = UPDATE knowledge_crdt_agent_lane_leases SET transition_revision += 1, expired_at_utc = $transition_at, last_transition_event_id = type::record('kernel_event_ledger', $event.event_id) WHERE lease_id = $lease_id AND actor_id = $actor_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $lease_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
COMMIT TRANSACTION;
"#;
const RECORD_CRDT_PROPOSAL_QUERY: &str = r#"
LET $existing = (SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, lease_id.lease_id AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $row.proposal_id AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND recorded_event_id.owner_account_id = $row.owner_account_id AND recorded_event_id.actor_principal_id = $row.actor_principal_id AND recorded_event_id.authenticated_session_id = $row.authenticated_session_id AND recorded_event_id.access_space_id = $row.access_space_id AND recorded_event_id.workspace_id = $row.workspace_id AND last_transition_event_id.owner_account_id = $row.owner_account_id AND last_transition_event_id.actor_principal_id = $row.actor_principal_id AND last_transition_event_id.authenticated_session_id = $row.authenticated_session_id AND last_transition_event_id.access_space_id = $row.access_space_id AND last_transition_event_id.workspace_id = $row.workspace_id LIMIT 2);
LET $lease = (SELECT VALUE id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $row.lease_id AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND actor_id = $row.actor_id AND actor_kind = $row.actor_kind AND session_id = $row.session_id AND correlation_id = $row.correlation_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc IS NONE AND expired_at_utc IS NONE AND ((scope_kind = 'workspace' AND scope_id = $row.workspace_id) OR (scope_kind = 'document' AND scope_id = $row.crdt_document_id AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id)) AND recorded_event_id.owner_account_id = $row.owner_account_id AND recorded_event_id.actor_principal_id = $row.actor_principal_id AND recorded_event_id.authenticated_session_id = $row.authenticated_session_id AND recorded_event_id.access_space_id = $row.access_space_id AND recorded_event_id.workspace_id = $row.workspace_id AND last_transition_event_id.owner_account_id = $row.owner_account_id AND last_transition_event_id.actor_principal_id = $row.actor_principal_id AND last_transition_event_id.authenticated_session_id = $row.authenticated_session_id AND last_transition_event_id.access_space_id = $row.access_space_id AND last_transition_event_id.workspace_id = $row.workspace_id LIMIT 2);
LET $retry_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $event.event_id AND event_version = $event.event_version AND kernel_task_run_id = $event.kernel_task_run_id AND session_run_id = $event.session_run_id AND aggregate_type = $event.aggregate_type AND aggregate_id = $event.aggregate_id AND idempotency_key = $event.idempotency_key AND event_type = $event.event_type AND actor_kind = $event.actor_kind AND actor_id = $event.actor_id AND causation_id = $event.causation_id AND correlation_id = $event.correlation_id AND payload_hash = $event.payload_hash AND source_component = $event.source_component AND payload = $event.payload AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id LIMIT 2);
IF array::len($existing) > 1 { THROW 'model-lane CRDT proposal identity is ambiguous'; } ELSE IF array::len($existing) = 1 {
    IF $existing[0].document_id != $row.document_id OR $existing[0].crdt_document_id != $row.crdt_document_id OR $existing[0].base_update_seq != $row.base_update_seq OR $existing[0].base_state_vector != $row.base_state_vector OR $existing[0].proposed_diff != $row.proposed_diff OR $existing[0].diff_sha256 != $row.diff_sha256 OR $existing[0].source_span_citations != $row.source_span_citations OR $existing[0].actor_id != $row.actor_id OR $existing[0].actor_kind != $row.actor_kind OR $existing[0].session_id != $row.session_id OR $existing[0].correlation_id != $row.correlation_id OR $existing[0].lease_id != $row.lease_id OR $existing[0].recorded_event_id != $event.event_id OR array::len($retry_event) != 1 { THROW 'model-lane CRDT proposal immutable identity conflict'; };
    RETURN $existing;
} ELSE {
    LET $covering = SELECT VALUE id FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc = NONE AND expired_at_utc = NONE AND ((scope_kind = 'workspace' AND scope_id = $row.workspace_id) OR (scope_kind = 'document' AND document_id = $row.document_id AND crdt_document_id = $row.crdt_document_id AND scope_id = $row.crdt_document_id));
    IF array::len($lease) != 1 OR array::len($covering) != 1 OR $covering[0] != $lease[0] OR $wp012_original_subject_id != $row.document_id { THROW 'model-lane CRDT proposal lease is unavailable in exact scope'; };
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT event exists without its proposal'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$row.workspace_id], owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id };
    LET $created = CREATE type::record('knowledge_crdt_ai_edit_proposals', $record_id) CONTENT { proposal_id: $row.proposal_id, owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id, authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id, workspace_id: $row.workspace_id, document_id: $row.document_id, crdt_document_id: $row.crdt_document_id, base_update_seq: $row.base_update_seq, base_state_vector: $row.base_state_vector, proposed_diff: $row.proposed_diff, diff_sha256: $row.diff_sha256, source_span_citations: $row.source_span_citations, actor_id: $row.actor_id, actor_kind: $row.actor_kind, session_id: $row.session_id, correlation_id: $row.correlation_id, lease_id: $lease[0], review_state: 'proposed', decided_by: NONE, decided_at_utc: NONE, decision_reason: NONE, recorded_event_id: type::record('kernel_event_ledger', $event.event_id), decided_event_id: NONE, promotion_requested_event_id: NONE, promotion_accepted_event_id: NONE, applied_update_id: NONE, applied_update_sha256: NONE, applied_event_id: NONE, last_transition_event_id: type::record('kernel_event_ledger', $event.event_id) };
    RETURN SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, lease_id.lease_id AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM type::record('knowledge_crdt_ai_edit_proposals', $record_id) WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id;
};
"#;
const DECIDE_CRDT_PROPOSAL_QUERY: &str = r#"
LET $current = (SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, lease_id.lease_id AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND document_id = $wp012_original_subject_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
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
    IF $review_state = 'promoted' {
        LET $head = SELECT update_id, update_seq FROM kernel_crdt_updates WHERE document_id = $current[0].document_id AND crdt_document_id = $current[0].crdt_document_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY update_seq DESC LIMIT 1;
        IF array::len($head) != 1 OR $head[0].update_id != $current[0].applied_update_id OR $head[0].update_seq != $current[0].base_update_seq + 1 { THROW 'model-lane CRDT promotion applied head is stale'; };
    };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $accepted_ledger = IF $review_state = 'promoted' { CREATE type::record('kernel_event_ledger', $promotion_accepted_event.event_id) CONTENT { event_id: $promotion_accepted_event.event_id, event_version: $promotion_accepted_event.event_version, kernel_task_run_id: $promotion_accepted_event.kernel_task_run_id, session_run_id: $promotion_accepted_event.session_run_id, aggregate_type: $promotion_accepted_event.aggregate_type, aggregate_id: $promotion_accepted_event.aggregate_id, idempotency_key: $promotion_accepted_event.idempotency_key, event_type: $promotion_accepted_event.event_type, actor_kind: $promotion_accepted_event.actor_kind, actor_id: $promotion_accepted_event.actor_id, causation_id: $promotion_accepted_event.causation_id, correlation_id: $promotion_accepted_event.correlation_id, payload_hash: $promotion_accepted_event.payload_hash, source_component: $promotion_accepted_event.source_component, payload: $promotion_accepted_event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id } } ELSE { [] };
    LET $updated = UPDATE knowledge_crdt_ai_edit_proposals SET review_state = $review_state, decided_by = IF $review_state = 'promoted' { decided_by } ELSE { $decided_by }, decided_at_utc = IF $review_state = 'promoted' { decided_at_utc } ELSE { time::now() }, decision_reason = IF $review_state = 'promoted' { decision_reason } ELSE { $decision_reason }, decided_event_id = IF $review_state = 'promoted' { decided_event_id } ELSE { type::record('kernel_event_ledger', $event.event_id) }, promotion_requested_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $event.event_id) } ELSE { promotion_requested_event_id }, promotion_accepted_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $promotion_accepted_event.event_id) } ELSE { promotion_accepted_event_id }, last_transition_event_id = IF $review_state = 'promoted' { type::record('kernel_event_ledger', $promotion_accepted_event.event_id) } ELSE { type::record('kernel_event_ledger', $event.event_id) } WHERE proposal_id = $proposal_id AND document_id = $wp012_original_subject_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, lease_id.lease_id AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND document_id = $wp012_original_subject_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
"#;
const BIND_CRDT_PROPOSAL_UPDATE_QUERY: &str = r#"
LET $proposal = (SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, lease_id.lease_id AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND document_id = $wp012_original_subject_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2);
LET $update = IF array::len($proposal) = 1 { (SELECT VALUE id FROM kernel_crdt_updates WHERE update_id = $applied_update_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND document_id = $proposal[0].document_id AND crdt_document_id = $proposal[0].crdt_document_id AND actor_id = $proposal[0].actor_id AND actor_kind = $proposal[0].actor_kind AND session_id = $proposal[0].session_id AND trace_id = $proposal[0].correlation_id AND update_seq = $proposal[0].base_update_seq + 1 AND state_vector_before = $proposal[0].base_state_vector AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.event_type = 'KNOWLEDGE_CRDT_UPDATE_RECORDED' AND event_ledger_event_id.aggregate_type = 'knowledge_crdt_document' AND event_ledger_event_id.aggregate_id = crdt_document_id AND event_ledger_event_id.actor_id = actor_id AND event_ledger_event_id.correlation_id = trace_id AND event_ledger_event_id.payload.update_id = update_id AND event_ledger_event_id.payload.update_sha256 = update_sha256 LIMIT 2) } ELSE { [] };
LET $retry_event = (SELECT VALUE id FROM kernel_event_ledger WHERE record::id(id) = $event.event_id AND event_version = $event.event_version AND kernel_task_run_id = $event.kernel_task_run_id AND session_run_id = $event.session_run_id AND aggregate_type = $event.aggregate_type AND aggregate_id = $event.aggregate_id AND idempotency_key = $event.idempotency_key AND event_type = $event.event_type AND actor_kind = $event.actor_kind AND actor_id = $event.actor_id AND causation_id = $event.causation_id AND correlation_id = $event.correlation_id AND payload_hash = $event.payload_hash AND source_component = $event.source_component AND payload = $event.payload AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
IF array::len($proposal) > 1 OR array::len($update) > 1 { THROW 'model-lane CRDT proposal application identity is ambiguous'; } ELSE IF array::len($proposal) = 0 { RETURN []; } ELSE IF $proposal[0].applied_update_id = $applied_update_id AND $proposal[0].applied_update_sha256 = $applied_update_sha256 AND $proposal[0].applied_event_id = $event.event_id AND $proposal[0].last_transition_event_id = $event.event_id AND array::len($retry_event) = 1 { RETURN $proposal; } ELSE {
    IF $proposal[0].review_state != 'approved' OR $proposal[0].diff_sha256 != $applied_update_sha256 OR $expected_actor_id != $proposal[0].actor_id OR $event.actor_id != $expected_actor_id OR array::len($update) != 1 OR $proposal[0].applied_update_id != NONE { THROW 'model-lane CRDT proposal application authority is unavailable'; };
    LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
    IF array::len($event_existing) != 0 { THROW 'model-lane CRDT apply event exists without proposal binding'; };
    IF $event.payload.application_evidence.schema_id != 'hsk.model_lane_proposal_application@1'
        OR $event.payload.application_evidence.proposal_id != $proposal[0].proposal_id
        OR $event.payload.application_evidence.approved_diff_sha256 != $proposal[0].diff_sha256
        OR $event.payload.application_evidence.update_id != $applied_update_id
        OR $event.payload.application_evidence.approval_event_id != $proposal[0].decided_event_id
        OR $event.payload.application_evidence.operation != 'append_text'
        OR $event.payload.application_evidence.path != $proposal[0].proposed_diff.path {
        THROW 'model-lane CRDT proposal application evidence mismatch';
    };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    LET $updated = UPDATE knowledge_crdt_ai_edit_proposals SET applied_update_id = $applied_update_id, applied_update_sha256 = $applied_update_sha256, applied_event_id = type::record('kernel_event_ledger', $event.event_id), last_transition_event_id = type::record('kernel_event_ledger', $event.event_id) WHERE proposal_id = $proposal_id AND document_id = $wp012_original_subject_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
    RETURN SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, lease_id.lease_id AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $proposal_id AND document_id = $wp012_original_subject_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;
};
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
    LaunchAdmission,
    LaunchAdmissionTransition,
    RoutingExecution,
    CloudProjectionPlan,
    CloudConsentReceipt,
    CloudConsentDenial,
    RoleEvidence,
    ValidatorDesignation,
    RoleEvidenceConsumption,
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
            Self::LaunchAdmission => "launch_admission",
            Self::LaunchAdmissionTransition => "launch_admission_transition",
            Self::RoutingExecution => "routing_execution",
            Self::CloudProjectionPlan => "cloud_projection_plan",
            Self::CloudConsentReceipt => "cloud_consent_receipt",
            Self::CloudConsentDenial => "cloud_consent_denial",
            Self::RoleEvidence => "role_evidence",
            Self::ValidatorDesignation => "validator_designation",
            Self::RoleEvidenceConsumption => "role_evidence_consumption",
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


#[derive(Debug, SurrealValue)]
struct RoleEvidenceLookupBindings {
    event_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct ValidatorDesignationLookupBindings {
    event_id: String,
    owner_account_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct RoutingPriorLookupBindings {
    routing_prior_stage_id: String,
    event_id: String,
    owner_account_id: String,
    workspace_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
}

#[derive(SurrealValue)]
struct CanonicalRoleEvidenceRow {
    record_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    event_id: String,
    event_seq: i64,
    event_stream_version: i64,
    transaction_seq: i64,
    ledger_event_seq: i64,
    ledger_event_id: String,
    ledger_event_stream_version: i64,
    ledger_stream_id: String,
    payload_json: String,
    payload_hash: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    issuance_operation_id: String,
}


#[derive(SurrealValue)]
struct RoleEvidenceInsertBindings {
    record_id: String,
    event_id: String,
    event_sequence: i64,
    operation_id: String,
    aggregate_id: String,
    run_id: String,
    stream_id: String,
    idempotency_key: String,
    record_json: String,
    payload_hash: String,
    causation_id: String,
    correlation_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct ProtectedRoleTargetRow {
    record_id: String,
    record: StoredRow,
}

impl ProtectedRoleEvidenceReadback {
    pub(crate) fn binding_event_id(&self) -> &str { &self.record.event_id }
}

pub(crate) struct ProtectedRoleEvidenceReadback {
    operation_id: uuid::Uuid,
    scope: ModelLaneScope,
    record: SurrealModelLaneRecord,
}

pub(crate) struct RunnerDesignationReadback {
    operation_id: uuid::Uuid,
    scope: ModelLaneScope,
    request: crate::swarm_orchestration::model_lane::WorkspaceValidatorRunnerDesignationRequest,
    record: SurrealModelLaneRecord,
}

enum RoleEvidenceCreationAuthority<'a, 'tx> {
    Owner(&'a super::resource_authority::OwnerIssuanceSeal<'tx>),
    Consumer(&'a super::resource_authority::CurrentConsumerSeal<'tx>),
}
impl RoleEvidenceCreationAuthority<'_, '_> {
    async fn with_creation<T, F>(&self, record_id: &str, event_id: &str, hash: &str, future: F)
        -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        match self {
            Self::Owner(seal) => seal.with_protected_role_authority_creation("role_evidence", record_id, event_id, hash, future).await,
            Self::Consumer(seal) => seal.with_protected_role_authority_creation("role_evidence", record_id, event_id, hash, future).await,
        }
    }
}
pub(crate) struct PendingOwnerRoleEvidence {
    readback: RoleEvidenceReadback,
    verified: handshake_role_authority::domain::PureVerifiedRoleEvidence,
}

impl PendingOwnerRoleEvidence {
    pub(crate) fn receipt(&self) -> &handshake_role_authority::domain::RoleReceipt { &self.readback.receipt }
    pub(crate) fn into_readback(self) -> RoleEvidenceReadback { self.readback }
}

pub(crate) struct TxCheckedRoleEvidence<'tx> {
    transaction: &'tx super::SurrealTransactionContext<'tx>,
    operation_id: uuid::Uuid,
    current_consumer: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    selector: crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    verified: handshake_role_authority::domain::PureVerifiedRoleEvidence,
    issuance_operation_id: uuid::Uuid,
    command: crate::swarm_orchestration::model_lane::RoleConsumptionCommand,
}

impl TxCheckedRoleEvidence<'_> {
    pub(crate) fn protected_consumption_target(&self) -> Result<super::resource_authority::ProtectedRoleEvidenceTarget, SurrealStorageError> {
        let receipt = self.receipt();
        let bytes = handshake_role_authority::canonical::canonical_receipt_bytes(receipt)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let scope = role_storage_scope(&receipt.body.scope);
        Ok(super::resource_authority::ProtectedRoleEvidenceTarget {
            record_kind: ModelLaneRecordKind::RoleEvidence.as_str().to_owned(),
            record_id: stable_record_id(ModelLaneRecordKind::RoleEvidence.as_str(), &receipt.body.action.idempotency_key, &scope),
            event_id: receipt.event.event_id.clone(), record_sha256: sha256_hex(&bytes), issuer_scope: receipt.body.scope.clone(),
        })
    }
    pub(crate) fn operation_id(&self) -> uuid::Uuid { self.operation_id }
    pub(crate) fn receipt(&self) -> &handshake_role_authority::domain::RoleReceipt { self.verified.receipt() }
    pub(crate) fn issuer_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.verified.receipt().body.scope
    }
    pub(crate) fn consumer_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.current_consumer
    }
    pub(crate) fn require_matches(
        &self, transaction: &super::SurrealTransactionContext<'_>, operation_id: uuid::Uuid,
        selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    ) -> Result<(), SurrealStorageError> {
        if !std::ptr::eq(self.transaction, transaction) || self.operation_id != operation_id
            || !role_selectors_match(&self.selector, selector) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        Ok(())
    }

    pub(crate) fn require_stage_ack(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        operation_id: uuid::Uuid,
        prior_selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
        acknowledgement: &crate::swarm_orchestration::model_lane::RoutingStageAcknowledgementIntent,
    ) -> Result<(), SurrealStorageError> {
        self.require_matches(transaction, operation_id, &self.selector)?;
        if !stage_ack_command_matches(&self.command, operation_id, prior_selector, acknowledgement)
            || &self.current_consumer != acknowledgement.claimed_scope()
            || self.verified.reference() == &prior_selector.evidence_ref {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let expected = match acknowledgement {
            crate::swarm_orchestration::model_lane::RoutingStageAcknowledgementIntent::Operator(intent) =>
                handshake_role_authority::canonical::prepare_operator_ack(intent),
            crate::swarm_orchestration::model_lane::RoutingStageAcknowledgementIntent::Validator(intent) =>
                handshake_role_authority::canonical::prepare_validator_ack(intent),
        }.map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "captured routing acknowledgement integrity is invalid",
        })?;
        if &self.verified.receipt().body != expected.body()
            || acknowledgement.stage().prior_receipt_ref != prior_selector.evidence_ref {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "new routing acknowledgement differs from its captured command or prior receipt",
            });
        }
        Ok(())
    }

    pub(crate) fn require_promotion_input(
        &self, input: &crate::swarm_orchestration::model_lane::NewModelLanePromotionDecision,
    ) -> Result<(), SurrealStorageError> {
        match &self.command {
            crate::swarm_orchestration::model_lane::RoleConsumptionCommand::ResolvePromotion { input: bound, .. }
                if bound == input => Ok(()),
            _ => Err(SurrealStorageError::TransactionAuthorityRequired),
        }
    }

    // This is a readback obligation, not an authorization witness.
    pub(crate) fn into_readback(self) -> RoleEvidenceReadback {
        RoleEvidenceReadback {
            operation_id: self.operation_id, current_consumer: self.current_consumer,
            selector: self.selector, receipt: self.verified.receipt().clone(),
            issuance_operation_id: self.issuance_operation_id,
        }
    }
}

pub(crate) struct RoleEvidenceReadback {
    operation_id: uuid::Uuid,
    current_consumer: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    selector: crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    receipt: handshake_role_authority::domain::RoleReceipt,
    issuance_operation_id: uuid::Uuid,
}

pub(crate) struct PublishedRoleEvidence {
    operation_id: uuid::Uuid,
    receipt: handshake_role_authority::domain::RoleReceipt,
}

impl PublishedRoleEvidence {
    pub(crate) fn operation_id(&self) -> uuid::Uuid { self.operation_id }
    pub(crate) fn receipt(&self) -> &handshake_role_authority::domain::RoleReceipt { &self.receipt }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SurrealModelLaneWrite {
    pub kind: ModelLaneRecordKind,
    pub aggregate_id: String,
    pub run_id: String,
    pub event_ledger_stream_id: Option<String>,
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
    ContentMismatch {
        update_id: String,
    },
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
    #[cfg(feature = "test-utils")]
    admission_test_control: ContextAdmissionTestControl,
    #[cfg(feature = "test-utils")]
    completion_test_control: ContextCompletionTestControl,
}

#[cfg(test)]
pub(crate) struct ContextGuardObservation {
    guards: std::sync::Arc<std::sync::Mutex<Vec<ContextCoordinationGuard>>>,
    storage: SurrealStorage,
}

#[cfg(test)]
impl ContextGuardObservation {
    pub(crate) fn same_guards_and_owner(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.guards, &other.guards)
            && std::sync::Arc::ptr_eq(&self.storage.inner, &other.storage.inner)
    }

    pub(crate) fn belongs_to(&self, provider: &SurrealModelLaneStore) -> bool {
        std::sync::Arc::ptr_eq(&self.storage.inner, &provider.storage.inner)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ContextCoordinationPurpose {
    Consume, SourceRead, NavigationRead, FinalPromotionRead, FinalPromotionRecord, PromotionDecisionRead, RoutingDiagnosticsRead, RoutingCreate, RoutingTransition, RunAttachment, RecoveryExecute, DiagnosticRecord, RecoveryCheckpointRecord, ArtifactBinding, Handoff, GuardedMessage, RunCreate, LaneCreate, ProfileSessionWrite, ProfileAdmissionCreate, ProfileAdmissionTransition, ProfileAdmissionFinalization, CrdtUpdate, CrdtSnapshot,
    DocumentClaim, WorkspaceClaim, SourceRootClaim, IndexRunClaim, LeaseRenew, LeaseRelease, LeaseExpiry,
    ProposalRecord, ProposalDecide, ProposalApply,
    CloudProjectionPlan, CloudConsentReceipt, CloudRun, CloudLane, CloudConsentDenial,
}

impl ContextCoordinationPurpose {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Consume => "consume", Self::SourceRead => "source_read", Self::NavigationRead => "navigation_read", Self::ArtifactBinding => "artifact_binding",
            Self::FinalPromotionRead => "final_promotion_read",
            Self::PromotionDecisionRead => "promotion_decision_read",
            Self::FinalPromotionRecord => "final_promotion_record",
            Self::RoutingDiagnosticsRead => "routing_diagnostics_read",
            Self::RoutingCreate => "routing_create", Self::RoutingTransition => "routing_transition",
            Self::RunAttachment => "run_attachment",
            Self::RecoveryExecute => "recovery_execute",
            Self::DiagnosticRecord => "diagnostic_record", Self::RecoveryCheckpointRecord => "recovery_checkpoint_record",
            Self::Handoff => "handoff", Self::GuardedMessage => "guarded_message",
            Self::RunCreate => "run_create", Self::LaneCreate => "lane_create",
            Self::ProfileSessionWrite => "profile_session_write",
            Self::ProfileAdmissionCreate => "profile_admission_create", Self::ProfileAdmissionTransition => "profile_admission_transition",
            Self::ProfileAdmissionFinalization => "profile_admission_finalization",
            Self::CrdtUpdate => "crdt_update", Self::CrdtSnapshot => "crdt_snapshot",
            Self::ProposalRecord => "proposal_record", Self::ProposalDecide => "proposal_decide", Self::ProposalApply => "proposal_apply",
            Self::CloudProjectionPlan => "cloud_projection_plan", Self::CloudConsentReceipt => "cloud_consent_receipt",
            Self::CloudRun => "cloud_run", Self::CloudLane => "cloud_lane", Self::CloudConsentDenial => "cloud_consent_denial",
            Self::DocumentClaim => "document_claim", Self::WorkspaceClaim => "workspace_claim",
            Self::SourceRootClaim => "source_root_claim", Self::IndexRunClaim => "index_run_claim",
            Self::LeaseRenew => "lease_renew", Self::LeaseRelease => "lease_release", Self::LeaseExpiry => "lease_expiry",
        }
    }

    fn accepts_original(self, kind: super::resource_authority::ResourceKind,
        action: super::resource_authority::ResourceAction) -> bool {
        use super::resource_authority::{ResourceAction, ResourceKind};
        match self {
            Self::ProfileSessionWrite => kind == ResourceKind::Workspace && matches!(action, ResourceAction::Create | ResourceAction::Update),
            Self::Consume | Self::SourceRead | Self::NavigationRead | Self::FinalPromotionRead | Self::PromotionDecisionRead | Self::RoutingDiagnosticsRead => kind == ResourceKind::Workspace && action == ResourceAction::Read,
            Self::DocumentClaim | Self::CrdtUpdate | Self::CrdtSnapshot
                | Self::ProposalRecord | Self::ProposalDecide | Self::ProposalApply =>
                kind == ResourceKind::RichDocument && action == ResourceAction::Update,
            Self::LeaseRenew | Self::LeaseRelease | Self::LeaseExpiry =>
                matches!(kind, ResourceKind::Workspace | ResourceKind::RichDocument) && action == ResourceAction::Update,
            Self::WorkspaceClaim | Self::IndexRunClaim | Self::ArtifactBinding | Self::Handoff
                | Self::GuardedMessage | Self::RunCreate | Self::LaneCreate | Self::ProfileAdmissionCreate | Self::ProfileAdmissionTransition | Self::ProfileAdmissionFinalization | Self::DiagnosticRecord | Self::RecoveryCheckpointRecord
                | Self::CloudProjectionPlan | Self::CloudConsentReceipt | Self::CloudRun | Self::CloudLane | Self::CloudConsentDenial =>
                kind == ResourceKind::Workspace && action == ResourceAction::Create,
            Self::FinalPromotionRecord | Self::RoutingCreate | Self::RunAttachment => kind == ResourceKind::Workspace && action == ResourceAction::Create,
            Self::RoutingTransition | Self::RecoveryExecute => kind == ResourceKind::Workspace && action == ResourceAction::Update,
            Self::SourceRootClaim => false,
        }
    }

}

// Tokens never enter Debug output, canonical payloads or caller projections.
#[derive(Clone)]
pub(crate) struct ContextCoordinationGuard {
    row: ContextCoordinationRow,
    token: zeroize::Zeroizing<String>,
    operation_id: uuid::Uuid,
    authority: super::resource_authority::RecordUserScope,
    purpose: ContextCoordinationPurpose,
    subject: super::ModelLaneOperationSubject,
    routing_sources: Option<std::sync::Arc<super::resource_authority::RoutingOriginalSources>>,
}

pub(crate) struct PendingContextAcquisition {
    token: zeroize::Zeroizing<String>,
    operation_id: uuid::Uuid,
    authority: super::resource_authority::RecordUserScope,
    purpose: ContextCoordinationPurpose,
    subject: super::ModelLaneOperationSubject,
    bindings: ContextCoordinationBindings,
}

#[derive(Clone, SurrealValue)]
struct ContextCoordinationRow {
    domain_key: String,
    generation: i64,
    revision: i64,
    admitted_at: DateTime<Utc>,
    validated_admission_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub(super) struct ContextFenceProof {
    domain_key: String,
    operation_id: uuid::Uuid,
    generation: i64,
    revision: i64,
    token: zeroize::Zeroizing<String>,
}

impl ContextFenceProof {
    pub(super) fn into_binding(self) -> surrealdb::types::Value {
        let mut object = surrealdb::types::Object::new();
        object.insert("domain_key", self.domain_key);
        object.insert("operation_id", self.operation_id);
        object.insert("generation", self.generation);
        object.insert("revision", self.revision);
        object.insert("token", self.token.to_string());
        surrealdb::types::Value::Object(object)
    }
}

#[derive(SurrealValue)]
struct ContextCoordinationTransition {
    domain_key: String,
    operation_id: uuid::Uuid,
    generation: i64,
    revision: i64,
    token_proof: String,
    token_hash: String,
    resource_id: String,
    subject_kind: String,
    subject_id: String,
    grant_id: String,
    capability_id: String,
    coordination_action: String,
    coordination_purpose: String,
}

#[cfg(feature = "test-utils")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextCoordinationBusy {
    pub domain_kind: String,
    pub domain_id: String,
    pub operation_id: uuid::Uuid,
}
#[cfg(feature = "test-utils")]
tokio::task_local! {
    pub(crate) static CONTEXT_COORDINATION_BUSY_OBSERVER: tokio::sync::mpsc::UnboundedSender<ContextCoordinationBusy>;
}

#[cfg(feature = "test-utils")]
#[derive(Clone, Debug)]
pub struct ContextAdmissionTestObservation {
    pub document_id: String,
    pub crdt_document_id: String,
    pub lease_id: String,
    pub admitted_at: DateTime<Utc>,
}
#[cfg(feature = "test-utils")]
#[derive(Clone, Default)]
pub struct ContextAdmissionTestControl {
    pause: std::sync::Arc<std::sync::Mutex<Option<(
        tokio::sync::oneshot::Sender<ContextAdmissionTestObservation>,
        tokio::sync::oneshot::Receiver<()>,
    )>>>,
}
#[cfg(feature = "test-utils")]
impl std::fmt::Debug for ContextAdmissionTestControl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ContextAdmissionTestControl").finish_non_exhaustive()
    }
}
#[cfg(feature = "test-utils")]
impl ContextAdmissionTestControl {
    pub fn pause_next(&self) -> Result<(tokio::sync::oneshot::Receiver<ContextAdmissionTestObservation>, tokio::sync::oneshot::Sender<()>), SurrealStorageError> {
        let mut pause = self.pause.lock().map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "admission test control poisoned" })?;
        if pause.is_some() { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "admission test pause already installed" }); }
        let (entered, observation) = tokio::sync::oneshot::channel();
        let (resume, resumed) = tokio::sync::oneshot::channel();
        *pause = Some((entered, resumed));
        Ok((observation, resume))
    }
    async fn after_canonical_admission(&self, observation: ContextAdmissionTestObservation) -> Result<(), SurrealStorageError> {
        let pause = self.pause.lock().map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "admission test control poisoned" })?.take();
        if let Some((entered, resumed)) = pause {
            entered.send(observation).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "admission test observer dropped" })?;
            resumed.await.map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "admission test resume dropped" })?;
        }
        Ok(())
    }
}

#[cfg(feature = "test-utils")]
#[derive(Clone, Default)]
pub struct ContextCompletionTestControl {
    pause: std::sync::Arc<std::sync::Mutex<Option<(tokio::sync::oneshot::Sender<uuid::Uuid>, tokio::sync::oneshot::Receiver<()>)>>>,
    acquisition_pause: std::sync::Arc<std::sync::Mutex<Option<(usize, tokio::sync::oneshot::Sender<uuid::Uuid>, tokio::sync::oneshot::Receiver<()>)>>>,
}
#[cfg(feature = "test-utils")]
impl std::fmt::Debug for ContextCompletionTestControl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ContextCompletionTestControl").finish_non_exhaustive()
    }
}
#[cfg(feature = "test-utils")]
impl ContextCompletionTestControl {
    pub fn pause_acquisition_after(&self, completed: usize) -> Result<(tokio::sync::oneshot::Receiver<uuid::Uuid>, tokio::sync::oneshot::Sender<()>), SurrealStorageError> {
        let mut pause = self.acquisition_pause.lock().map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "acquisition control poisoned" })?;
        if pause.is_some() { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "acquisition pause already installed" }); }
        let (entered, observation) = tokio::sync::oneshot::channel(); let (resume, resumed) = tokio::sync::oneshot::channel();
        *pause = Some((completed, entered, resumed)); Ok((observation, resume))
    }
    async fn before_acquisition_commit(&self, operation_id: uuid::Uuid) {
        let pause = {
            let mut state = self.acquisition_pause.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            match state.as_mut() {
                Some((remaining, _, _)) if *remaining > 0 => { *remaining -= 1; None },
                Some(_) => state.take(),
                None => None,
            }
        };
        if let Some((_, entered, resumed)) = pause {
            if entered.send(operation_id).is_ok() { let _ = resumed.await; }
        }
    }
    pub fn pause_next(&self) -> Result<(tokio::sync::oneshot::Receiver<uuid::Uuid>, tokio::sync::oneshot::Sender<()>), SurrealStorageError> {
        let mut pause = self.pause.lock().map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "completion control poisoned" })?;
        if pause.is_some() { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "completion pause already installed" }); }
        let (entered, observation) = tokio::sync::oneshot::channel(); let (resume, resumed) = tokio::sync::oneshot::channel();
        *pause = Some((entered, resumed)); Ok((observation, resume))
    }
    async fn after_observed_commit(&self, operation_id: uuid::Uuid) {
        let pause = self.pause.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
        if let Some((entered, resumed)) = pause {
            if entered.send(operation_id).is_ok() { let _ = resumed.await; }
        }
    }
}

#[cfg(feature = "test-utils")]
pub struct ContextCoordinationTestHolder {
    storage: SurrealStorage,
    guard: Option<ContextCoordinationGuard>,
}
#[cfg(feature = "test-utils")]
impl ContextCoordinationTestHolder {
    pub async fn release(&mut self) -> Result<(), SurrealStorageError> {
        let guard = self.guard.as_ref().ok_or(SurrealStorageError::InvalidModelLaneRecord {
            reason: "test coordination holder was already released",
        })?.clone();
        super::with_model_lane_subject(guard.subject.clone(),
            super::with_model_lane_operation(guard.purpose, guard.release(&self.storage))).await?;
        self.guard = None;
        Ok(())
    }
}
#[cfg(feature = "test-utils")]
pub async fn observe_context_coordination_busy_for_test<T>(
    observer: tokio::sync::mpsc::UnboundedSender<ContextCoordinationBusy>,
    operation: impl std::future::Future<Output = T>,
) -> T {
    CONTEXT_COORDINATION_BUSY_OBSERVER.scope(observer, operation).await
}

impl ContextCoordinationGuard {
    pub(crate) fn admitted_at(&self) -> DateTime<Utc> { self.row.admitted_at }

    pub(super) fn proof(&self) -> ContextFenceProof {
        ContextFenceProof { domain_key: self.row.domain_key.clone(), operation_id: self.operation_id,
            generation: self.row.generation, revision: self.row.revision, token: self.token.clone() }
    }

    fn require_original_envelope(&self) -> Result<(), SurrealStorageError> {
        let current = super::current_record_user_scope().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if current.resource_id != self.authority.resource_id || current.grant_id != self.authority.grant_id
            || current.session_id != self.authority.session_id || current.capability_id != self.authority.capability_id
            || current.action != self.authority.action || current.channel_binding_hash != self.authority.channel_binding_hash
            || current.session_token != self.authority.session_token
            || (self.purpose == ContextCoordinationPurpose::RecoveryExecute
                && (self.subject.kind != super::resource_authority::ResourceKind::Workspace
                    || current.action != super::resource_authority::ResourceAction::Update
                    || current.capability_id != "model_lane.recovery.execute")) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        Ok(())
    }

    // Return the rotated guard through the controller callback: it becomes usable
    // only after that enclosing transaction's actual commit has been observed.
    pub(crate) async fn fence_in_transaction(
        mut self,
        transaction: &super::SurrealTransactionContext<'_>,
    ) -> Result<Self, SurrealStorageError> {
        self.require_original_envelope()?;
        if let Some(sources) = &self.routing_sources {
            sources.revalidate().await.map_err(|error| match error {
                super::resource_authority::ResourceAuthorityError::Storage(error) => error,
                _ => SurrealStorageError::TransactionAuthorityRequired,
            })?;
        }
        let next_token = coordination_token()?;
        let bindings = self.transition_bindings(self.token.as_str().to_owned(), sha256_hex(next_token.as_bytes()));
        let rows = transaction.query_values_at::<ContextCoordinationRow, _>(
            "UPDATE type::record('model_lane_context_coordination', $domain_key) SET revision = revision + 1, token_proof = $token_proof, token_hash = $token_hash WHERE fn::wp012_context_coordination_candidate($this, type::record('protected_resources', $resource_id), type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND fn::wp012_context_coordination_candidate_is_holder($this, type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND state = 'held' AND holder_operation_id = $operation_id AND generation = $generation AND revision = $revision AND deadline > time::now() AND token_hash = crypto::sha256($token_proof) RETURN domain_key, generation, revision, admitted_at, validated_admission_at;",
            bindings, 0,
        ).await?;
        if rows.len() != 1 || rows[0].domain_key != self.row.domain_key
            || rows[0].generation != self.row.generation || rows[0].revision != self.row.revision + 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "ContextBundle coordination fence is not current" });
        }
        self.row = rows.into_iter().next().expect("one validated fence");
        self.token = next_token;
        Ok(self)
    }

    async fn stamp_crdt_admission(
        mut self, storage: &SurrealStorage, document_id: &str, crdt_document_id: &str,
    ) -> Result<Self, SurrealStorageError> {
        self.require_original_envelope()?;
        if self.purpose != ContextCoordinationPurpose::GuardedMessage || self.row.validated_admission_at.is_some() {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "CRDT admission timestamp is already assigned or has the wrong purpose" });
        }
        let next_token = coordination_token()?;
        let bindings = self.transition_bindings(self.token.to_string(), sha256_hex(next_token.as_bytes())).into_value();
        let surrealdb::types::Value::Object(mut bindings) = bindings else { unreachable!("typed transition bindings are an object"); };
        bindings.insert("admission_document_id", document_id.to_owned());
        bindings.insert("admission_crdt_document_id", crdt_document_id.to_owned());
        let bindings = surrealdb::types::Value::Object(bindings);
        let mut rows = storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<ContextCoordinationRow, _>(
                "LET $gate = (SELECT * FROM ONLY type::record('model_lane_context_coordination', $domain_key)); \
                 LET $covering = SELECT * FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $gate.owner_account_id AND actor_principal_id = $gate.actor_principal_id AND authenticated_session_id = $gate.authenticated_session_id AND access_space_id = $gate.access_space_id AND workspace_id = $gate.workspace_id AND claimed_at_utc <= time::now() AND expires_at_utc > time::now() AND released_at_utc = NONE AND expired_at_utc = NONE AND ((scope_kind = 'workspace' AND scope_id = $gate.workspace_id) OR (scope_kind = 'document' AND document_id = $admission_document_id AND crdt_document_id = $admission_crdt_document_id AND scope_id = $admission_crdt_document_id)); \
                 IF array::len($covering) != 1 OR !fn::wp012_model_lane_crdt_lease_identity($covering[0]) { THROW 'HSK-403-CONTEXT-ADMISSION'; } ELSE { \
                   RETURN UPDATE type::record('model_lane_context_coordination', $domain_key) SET validated_admission_at = time::now(), revision = revision + 1, token_proof = $token_proof, token_hash = $token_hash WHERE fn::wp012_context_coordination_candidate($this, type::record('protected_resources', $resource_id), type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND fn::wp012_context_coordination_candidate_is_holder($this, type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND domain_kind = 'workspace' AND state = 'held' AND holder_operation_id = $operation_id AND generation = $generation AND revision = $revision AND deadline > time::now() AND token_hash = crypto::sha256($token_proof) AND validated_admission_at = NONE RETURN domain_key, generation, revision, admitted_at, validated_admission_at; \
                 };",
                bindings, 2,
            ).await
        })).await?;
        if rows.len() != 1 || rows[0].domain_key != self.row.domain_key || rows[0].generation != self.row.generation
            || rows[0].revision != self.row.revision + 1 || rows[0].validated_admission_at.is_none() {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "CRDT admission timestamp has no canonical fenced acknowledgement" });
        }
        self.row = rows.pop().expect("one canonical admission acknowledgement");
        self.token = next_token;
        Ok(self)
    }

    fn transition_bindings(&self, token_proof: String, token_hash: String) -> ContextCoordinationTransition {
        ContextCoordinationTransition { domain_key: self.row.domain_key.clone(), operation_id: self.operation_id,
            generation: self.row.generation, revision: self.row.revision, token_proof, token_hash,
            resource_id: self.authority.resource_id.clone(), subject_kind: self.subject.kind.as_str().to_owned(),
            subject_id: self.subject.external_id.clone(), grant_id: self.authority.grant_id.clone().expect("validated original grant"),
            capability_id: self.authority.capability_id.clone(), coordination_action: self.authority.action.as_str().to_owned(),
            coordination_purpose: self.purpose.as_str().to_owned() }
    }

    pub(crate) async fn release(self, storage: &SurrealStorage) -> Result<(), SurrealStorageError> {
        self.require_original_envelope()?;
        let next_token = coordination_token()?;
        let bindings = self.transition_bindings(self.token.as_str().to_owned(), sha256_hex(next_token.as_bytes()));
        let transition_id = uuid::Uuid::now_v7();
        let rows = super::with_model_lane_subject(self.subject.clone(),
            storage.with_authenticated_transaction(transition_id, move |transaction| Box::pin(async move {
            transaction.query_values_at::<ContextCoordinationRow, _>(
                "UPDATE type::record('model_lane_context_coordination', $domain_key) SET state = 'released', revision = revision + 1, token_proof = $token_proof, token_hash = $token_hash, released_at = time::now() WHERE fn::wp012_context_coordination_candidate($this, type::record('protected_resources', $resource_id), type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND fn::wp012_context_coordination_candidate_is_holder($this, type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND state = 'held' AND holder_operation_id = $operation_id AND generation = $generation AND revision = $revision AND deadline > time::now() AND token_hash = crypto::sha256($token_proof) RETURN domain_key, generation, revision, admitted_at, validated_admission_at;",
                bindings, 0).await
        }))).await?;
        if rows.len() != 1 || rows[0].domain_key != self.row.domain_key
            || rows[0].generation != self.row.generation || rows[0].revision != self.row.revision + 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "ContextBundle coordination release was not confirmed" });
        }
        Ok(())
    }
}

fn coordination_token() -> Result<zeroize::Zeroizing<String>, SurrealStorageError> {
    let mut entropy = zeroize::Zeroizing::new([0_u8; 32]);
    getrandom::getrandom(entropy.as_mut()).map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
        reason: "ContextBundle coordination entropy unavailable",
    })?;
    Ok(zeroize::Zeroizing::new(hex::encode(entropy.as_ref())))
}

#[derive(SurrealValue)]
struct ContextResourceBindings {
    resource_id: String,
    subject_kind: String,
    subject_id: String,
    workspace_id: String,
}

#[derive(Clone, SurrealValue)]
struct ContextCoordinationBindings {
    domain_kind: String,
    domain_id: String,
    run_id: Option<String>,
    canonical_run_id: Option<String>,
    canonical_lane_id: Option<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    canonical_workspace_id: String,
    resource_id: String,
    subject_kind: String,
    subject_id: String,
    grant_id: String,
    capability_id: String,
    operation_id: uuid::Uuid,
    token_hash: String,
    coordination_action: String,
    coordination_purpose: String,
}

const ACQUIRE_CONTEXT_COORDINATION: &str = "\
LET $domain = { domain_kind: $domain_kind, domain_id: $domain_id, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
LET $key = fn::wp012_context_coordination_key($domain);\
LET $record = type::record('model_lane_context_coordination', $key);\
LET $existing = SELECT domain_key, generation, revision, state, deadline FROM $record;\
IF array::len($existing) = 0 {\
  RETURN CREATE $record CONTENT { domain_key: $key, domain_kind: $domain_kind, domain_id: $domain_id, run_id: $run_id, canonical_run: IF $canonical_run_id = NONE { NONE } ELSE { type::record('model_lane_authority', $canonical_run_id) }, canonical_lane: IF $canonical_lane_id = NONE { NONE } ELSE { type::record('model_lane_authority', $canonical_lane_id) }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, canonical_workspace: type::record('protected_resources', $canonical_workspace_id), authority_subject_kind: $subject_kind, authority_subject_id: $subject_id, authority_resource_id: type::record('protected_resources', $resource_id), authority_grant_id: type::record('resource_grants', $grant_id), authority_session_id: type::record('authenticated_sessions', $authenticated_session_id), authority_capability_id: $capability_id, authority_action: $coordination_action, authority_purpose: $coordination_purpose, state: 'held', holder_operation_id: $operation_id, generation: 1, revision: 1, token_hash: $token_hash, admitted_at: time::now(), deadline: time::now() + 5m, released_at: NONE } RETURN domain_key, generation, revision, admitted_at, validated_admission_at;\
} ELSE {\
  IF array::len($existing) != 1 { THROW 'HSK-403-CONTEXT-COORDINATION'; };\
  IF $existing[0].state = 'held' AND $existing[0].deadline > time::now() { RETURN []; };\
  RETURN UPDATE $record SET authority_resource_id = type::record('protected_resources', $resource_id), authority_subject_kind = $subject_kind, authority_subject_id = $subject_id, authority_grant_id = type::record('resource_grants', $grant_id), authority_capability_id = $capability_id, authority_action = $coordination_action, authority_purpose = $coordination_purpose, state = 'held', holder_operation_id = $operation_id, generation = generation + 1, revision = revision + 1, token_hash = $token_hash, token_proof = NONE, validated_admission_at = NONE, admitted_at = time::now(), deadline = time::now() + 5m, released_at = NONE WHERE generation = $existing[0].generation AND revision = $existing[0].revision AND (state = 'released' OR deadline <= time::now()) RETURN domain_key, generation, revision, admitted_at, validated_admission_at;\
};";

#[derive(Debug, SurrealValue)]
struct WriteBindings {
    record_id: String,
    event_id: String,
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    event_ledger_stream_id: Option<String>,
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
    event_ledger_stream_id: Option<String>,
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
    event_ledger_stream_id: Option<String>,
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

#[derive(SurrealValue)]
struct RunAttachmentSelection {
    run_id: String, lane_id: String, expected_record_json: String, next_record_json: String,
    input_run_record_json: String, lane_record_json: String, expected_event_stream_version: i64,
}
#[derive(SurrealValue)]
struct RunAttachmentBindings {
    run_attachment: RunAttachmentSelection, record_id: String, event_id: String,
    search_terms: Vec<String>, event_payload_json: String, event_payload_hash: String,
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
    access_space_id: String, workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct RoutingCommitBindings {
    routing_source_resource: String,
    routing_source_grant: String,
    routing_source_capability: String,
    routing_source_action: String,
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
    event_ledger_stream_id: Option<String>,
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
    event_ledger_stream_id: Option<String>,
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
    first_event_ledger_stream_id: Option<String>,
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
    second_event_ledger_stream_id: Option<String>,
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
    message_event_ledger_stream_id: Option<String>,
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
    binding_event_ledger_stream_id: Option<String>,
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

// The original envelope is supplied by the transaction controller, never minted here.
const NAVIGATION_AUTHORITY_SNAPSHOT: &str = "SELECT type::record('resource_grants', $wp012_original_grant).grant_version AS grant_version, type::record('resource_grants', $wp012_original_grant).policy_version AS grant_policy_version, type::record('protected_resources', $wp012_original_resource).policy_version AS resource_policy_version, $auth.policy_version AS session_policy_version, $auth.account_id.policy_version AS account_policy_version, $auth.principal_id.policy_version AS principal_policy_version, $auth.access_space_id.policy_version AS space_policy_version, $auth.account_id.revocation_epoch AS account_revocation_epoch, $auth.principal_id.revocation_epoch AS principal_revocation_epoch, $auth.access_space_id.revocation_epoch AS space_revocation_epoch, type::record('resource_grants', $wp012_original_grant).status AS grant_status, type::record('resource_grants', $wp012_original_grant).expires_at AS grant_expires_at, type::record('resource_grants', $wp012_original_grant).revoked_at AS grant_revoked_at, type::record('protected_resources', $wp012_original_resource).lifecycle_state AS resource_lifecycle_state, $auth.status AS session_status, $auth.expires_at AS session_expires_at, $auth.revoked_at AS session_revoked_at FROM model_lane_schema_registry WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 1;";

const NAVIGATION_GRANT_SNAPSHOT: &str = "SELECT record::id(id) AS grant_id, record::id(resource_id) AS resource_id, grant_version, policy_version AS grant_policy_version, status, expires_at, revoked_at, actions, capability_ids, delegation_chain, resource_id.policy_version AS resource_policy_version, resource_id.lifecycle_state AS resource_lifecycle_state FROM resource_grants WHERE grant_version != NONE ORDER BY id ASC;";

#[derive(PartialEq, Eq, SurrealValue)]
struct NavigationGrantSnapshot {
    grant_id: String,
    resource_id: String,
    grant_version: i64,
    grant_policy_version: i64,
    status: String,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    actions: Vec<String>,
    capability_ids: Vec<String>,
    delegation_chain: Vec<String>,
    resource_policy_version: i64,
    resource_lifecycle_state: String,
}

#[derive(Clone, Debug, serde::Serialize, SurrealValue)]
pub(crate) struct NavigationReadSelector {
    pub lookup_kind: String,
    pub lookup_ref: String,
    pub run_id: Option<String>,
    pub span_id: Option<String>,
    pub behavior_id: Option<String>,
    pub tier: Option<String>,
    pub micro_task_id: Option<String>,
    pub artifact_ref: Option<String>,
    pub context_bundle_id: Option<String>,
}

impl NavigationReadSelector {
    fn validate(&self) -> Result<(), SurrealStorageError> {
        if !matches!(self.lookup_kind.as_str(), "run_id" | "lane_id" | "message_id"
            | "model_session_id" | "session_id" | "wp_id" | "work_packet_id"
            | "mt_id" | "micro_task_id" | "task_board_id" | "trace_id"
            | "diagnostic_tier" | "recovery" | "context_bundle_id" | "artifact_ref"
            | "document_id" | "loom_block_id" | "memory_pack_ref" | "memory_pack_hash"
            | "loom_ref" | "fems_ref" | "locus_ref" | "locus_binding_ref"
            | "event_ledger_event_id" | "event_ledger_seq" | "span_id" | "error_code")
            || self.lookup_ref.trim().is_empty()
            || [&self.run_id, &self.span_id, &self.behavior_id, &self.tier, &self.micro_task_id,
                &self.artifact_ref, &self.context_bundle_id]
                .into_iter().any(|value| value.as_deref().is_some_and(|value| value.trim().is_empty()))
            || (self.span_id.is_some() && self.lookup_kind != "trace_id")
            || ((self.behavior_id.is_some() || self.tier.is_some() || self.micro_task_id.is_some())
                && self.lookup_kind != "diagnostic_tier")
            || ((self.artifact_ref.is_some() || self.context_bundle_id.is_some())
                && !matches!(self.lookup_kind.as_str(), "artifact_ref" | "context_bundle_id")) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "invalid navigation selector shape" });
        }
        Ok(())
    }
}

tokio::task_local! {
    static NAVIGATION_READ_SELECTOR: NavigationReadSelector;
}

#[derive(Clone, SurrealValue)]
pub(crate) struct NavigationReadBindings {
    navigation_selection: NavigationReadSelector,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl NavigationReadBindings {
    pub(crate) fn artifact_selection_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(&self.navigation_selection)
    }

    pub(crate) fn artifact_scope_matches(
        &self,
        exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    ) -> bool {
        self.owner_account_id == exact.owner_account_id.to_string()
            && self.actor_principal_id == exact.actor_principal_id.to_string()
            && self.authenticated_session_id == exact.authenticated_session_id.to_string()
            && self.access_space_id == exact.access_space_id.to_string()
            && self.workspace_id == exact.workspace_id.as_str()
    }
}

#[derive(Clone, SurrealValue)]
struct NavigationQueryBindings {
    navigation_selection: NavigationReadSelector,
    record_kind: String,
    run_id: Option<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

// These are scalar columns, not substitute canonical records. Hidden payloads
// are never defaulted into an empty record or assigned a rewritten source hash.
#[derive(Clone, PartialEq, Eq, SurrealValue)]
pub(crate) struct SurrealModelLaneNavigationRow {
    pub record_kind: String,
    pub aggregate_id: String,
    pub run_id: String,
    pub event_id: String,
    pub event_seq: i64,
    pub ordering_seq: Option<i64>,
}

#[derive(SurrealValue)]
struct NavigationReceiptBindings {
    navigation_selection: NavigationReadSelector,
    event_id: String,
    event_seq: i64,
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(PartialEq, Eq, SurrealValue)]
pub(crate) struct SurrealModelLaneNavigationReceipt {
    pub event_id: String,
    pub event_seq: i64,
}

#[derive(PartialEq, Eq, SurrealValue)]
struct NavigationDependencyReceipt {
    event_id: String,
    event_sequence: i64,
    event_version: String,
    aggregate_type: String,
    aggregate_id: String,
    event_type: String,
    source_component: String,
}

const NAVIGATION_RECEIPT_SNAPSHOT: &str = "SELECT event_id, event_sequence, event_version, aggregate_type, aggregate_id, event_type, source_component FROM kernel_event_ledger WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_id ASC;";
const NAVIGATION_INPUT_SNAPSHOT: &str = "SELECT navigation_source_input AS input FROM model_lane_authority WHERE navigation_source_input != NONE AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_id ASC;";
const NAVIGATION_MEMORY_DOCUMENT_LINK_SNAPSHOT: &str = "RETURN fn::wp012_model_lane_navigation_memory_document_links({ owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }, $navigation_selection);";
const NAVIGATION_MEMORY_PACK_SNAPSHOT: &str = "RETURN fn::wp012_model_lane_navigation_memory_packs({ owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }, $navigation_selection);";
const NAVIGATION_SOURCE_SNAPSHOT: &str = "SELECT record_kind, aggregate_id, run_id, event_id, event_seq, ordering_seq FROM model_lane_authority WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_id ASC;";

#[derive(PartialEq, Eq, SurrealValue)]
struct NavigationAuthoritySnapshot {
    grant_version: i64,
    grant_policy_version: i64,
    resource_policy_version: i64,
    session_policy_version: i64,
    account_policy_version: i64,
    principal_policy_version: i64,
    space_policy_version: i64,
    account_revocation_epoch: i64,
    principal_revocation_epoch: i64,
    space_revocation_epoch: i64,
    grant_status: String,
    grant_expires_at: Option<DateTime<Utc>>,
    grant_revoked_at: Option<DateTime<Utc>>,
    resource_lifecycle_state: String,
    session_status: String,
    session_expires_at: DateTime<Utc>,
    session_revoked_at: Option<DateTime<Utc>>,
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
    navigation_selection: Option<NavigationReadSelector>,
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
    navigation_selection: Option<NavigationReadSelector>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct CrdtChainBindings {
    navigation_selection: Option<NavigationReadSelector>,
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
    navigation_selection: Option<NavigationReadSelector>,
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
    ttl_seconds: i64,
    row: SurrealModelLaneCrdtLeaseWrite,
    event: SurrealModelLaneCrdtEventWrite,
}

#[derive(Debug, SurrealValue)]
struct CrdtLeaseMutationBindings {
    lease_id: String,
    actor_id: String,
    transition: String,
    ttl_seconds: i64,
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
    navigation_selection: Option<NavigationReadSelector>,
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

// Internal canonical inputs only; callers must run the owning byte/site/Yjs
// validator before publishing any CRDT-derived navigation evidence.
#[derive(PartialEq, SurrealValue)]
struct NavigationSourceInputRow {
    input: Value,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationRunInput {
    pub(crate) source_kind: String,
    pub(crate) kind: String,
    pub(crate) run_id: String,
    pub(crate) source_event_id: String,
    pub(crate) source_event_seq: i64,
    pub(crate) memory_pack_ref: String,
    pub(crate) memory_pack_hash: String,
}

#[derive(PartialEq, SurrealValue)]
struct NavigationMemoryPackRow {
    run_id: String,
    source_event_id: String,
    source_event_seq: i64,
    memory_pack_ref: String,
    memory_pack_hash: String,
    record_pack_id: String,
    record_workspace_id: String,
    source_membership: Value,
    pack: Option<Value>,
}

#[derive(PartialEq, Eq, SurrealValue)]
struct NavigationMemoryPackDocumentLink {
    source_id: String,
    block_key: String,
    document_key: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub(crate) struct NavigationMemoryPackInput {
    run_id: String,
    source_event_id: String,
    source_event_seq: i64,
    memory_pack_ref: String,
    memory_pack_hash: String,
    memory_pack: crate::ace::MemoryPack,
}

impl NavigationMemoryPackInput {
    pub(crate) fn run_id(&self) -> &str { &self.run_id }
    pub(crate) fn source_event_id(&self) -> &str { &self.source_event_id }
    pub(crate) fn source_event_seq(&self) -> i64 { self.source_event_seq }
    pub(crate) fn memory_pack_ref(&self) -> &str { &self.memory_pack_ref }
    pub(crate) fn memory_pack_hash(&self) -> &str { &self.memory_pack_hash }
    pub(crate) fn memory_pack(&self) -> &crate::ace::MemoryPack { &self.memory_pack }
}

#[derive(Clone, SurrealValue)]
struct NavigationMemoryAuthorizationBindings {
    navigation_selection: NavigationReadSelector,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    document_id: String,
}

#[derive(SurrealValue)]
struct NavigationMemoryAuthorizationRow {
    allowed: bool,
}

async fn navigation_memory_pack_authorized(
    transaction: &super::SurrealTransactionContext<'_>,
    bindings: &NavigationReadBindings,
    input: &NavigationMemoryPackInput,
    document_links: &[NavigationMemoryPackDocumentLink],
) -> Result<bool, SurrealStorageError> {
    let mut authority = NavigationMemoryAuthorizationBindings {
        navigation_selection: bindings.navigation_selection.clone(),
        owner_account_id: bindings.owner_account_id.clone(),
        actor_principal_id: bindings.actor_principal_id.clone(),
        authenticated_session_id: bindings.authenticated_session_id.clone(),
        access_space_id: bindings.access_space_id.clone(),
        workspace_id: bindings.workspace_id.clone(), document_id: String::new(),
    };
    let own = transaction.query_first::<NavigationMemoryAuthorizationRow, _>(
        "RETURN { allowed: fn::mt109_has_grant('memory_pack', $workspace_id, 'read', 'memory.read') };",
        authority.clone(),
    ).await?;
    if !own.is_some_and(|row| row.allowed) { return Ok(false); }
    let mut documents = BTreeSet::new();
    for source in input.memory_pack.items.iter().flat_map(|item| item.source_refs.iter()) {
        if source.kind == crate::ace::FemsSourceRefKind::Artifact { continue; }
        if source.kind != crate::ace::FemsSourceRefKind::DocBlock { return Ok(false); }
        let mut matches = document_links.iter().filter(|link| link.source_id == source.id);
        let Some(link) = matches.next() else { return Ok(false); };
        if matches.next().is_some() || link.block_key.is_empty() || link.document_key.is_empty()
            || link.updated_at < link.created_at {
            return Ok(false);
        }
        documents.insert(link.document_key.clone());
    }
    for document in documents {
        authority.document_id = document;
        let child = transaction.query_first::<NavigationMemoryAuthorizationRow, _>(
            "RETURN { allowed: fn::wp012_model_lane_navigation_document_read({ owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }, $document_id) };",
            authority.clone(),
        ).await?;
        if !child.is_some_and(|row| row.allowed) { return Ok(false); }
    }
    Ok(true)
}

async fn navigation_memory_artifact_registrations<'tx>(
    transaction: &'tx super::SurrealTransactionContext<'tx>,
    original_bindings: &NavigationReadBindings,
    exact_scope: Option<&crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution>,
    selection: &Value,
    input: &NavigationMemoryPackInput,
) -> Result<Option<Vec<super::resource_authority::NavigationArtifactRegistration>>, SurrealStorageError> {
    let mut registrations = Vec::new();
    for source in input.memory_pack.items.iter().flat_map(|item| item.source_refs.iter()) {
        if source.kind != crate::ace::FemsSourceRefKind::Artifact { continue; }
        let Some(exact_scope) = exact_scope else { return Ok(None); };
        let Some(permit) = super::resource_authority::authorize_navigation_artifact_read(
            transaction, original_bindings, exact_scope, selection, source,
        ).await.map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "navigation artifact authorization failed",
        })? else { return Ok(None); };
        let Some(registration) = super::resource_authority::read_navigation_artifact(
            transaction, &permit,
        ).await.map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "navigation artifact canonical read failed",
        })? else { return Ok(None); };
        if registration.manifest().artifact_id.to_string() != source.id
            || source.hash.as_deref() != Some(registration.manifest().content_hash.as_str()) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "navigation artifact source integrity mismatch",
            });
        }
        registrations.push(registration);
    }
    Ok(Some(registrations))
}

fn navigation_memory_pack_inputs(
    workspace_id: &str,
    sources: &[SurrealModelLaneNavigationRow],
    inputs: &[NavigationSourceInputRow],
    packs: &[NavigationMemoryPackRow],
) -> Result<Vec<NavigationMemoryPackInput>, SurrealStorageError> {
    let mut runs = Vec::new();
    for input in inputs {
        if input.input.get("source_kind").and_then(Value::as_str) == Some("run") {
            let Ok(run) = serde_json::from_value::<NavigationRunInput>(input.input.clone()) else { continue; };
            if run.kind != "run" { continue; }
            runs.push(run);
        }
    }
    let mut witnessed = BTreeSet::new();
    let mut verified: Vec<NavigationMemoryPackInput> = Vec::new();
    for row in packs {
        let Some(raw) = row.pack.as_ref() else { continue; };
        let matching_runs: Vec<_> = runs.iter().filter(|run|
            run.run_id == row.run_id && run.source_event_id == row.source_event_id
                && run.source_event_seq == row.source_event_seq
                && run.memory_pack_ref == row.memory_pack_ref
                && run.memory_pack_hash == row.memory_pack_hash
        ).collect();
        if matching_runs.len() != 1 || sources.iter().filter(|source|
            source.record_kind == "run" && source.run_id == row.run_id && source.aggregate_id == row.run_id
                && source.event_id == row.source_event_id && source.event_seq == row.source_event_seq
        ).count() != 1 || row.record_pack_id != row.memory_pack_ref {
            continue;
        }
        let pack = match crate::storage::fems_memory::verify_persisted_memory_pack(
            raw, &row.record_pack_id, &row.record_workspace_id, workspace_id,
            &row.memory_pack_ref, &row.memory_pack_hash,
        ) {
            Ok(pack) => pack,
            Err(_) => continue,
        };
        if pack.items.iter().any(|item| item.source_refs.is_empty()
            || item.source_refs.iter().any(|source| !matches!(source.kind,
                crate::ace::FemsSourceRefKind::DocBlock | crate::ace::FemsSourceRefKind::Artifact))) {
            continue;
        }
        let membership = serde_json::to_value(pack.items.iter()
            .flat_map(|item| item.source_refs.iter()).collect::<Vec<_>>())
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "navigation memory source membership encoding failed" })?;
        if membership != row.source_membership {
            continue;
        }
        if !witnessed.insert((row.run_id.clone(), row.source_event_id.clone(), row.source_event_seq)) {
            verified.retain(|input| input.run_id != row.run_id || input.source_event_id != row.source_event_id
                || input.source_event_seq != row.source_event_seq);
            continue;
        }
        verified.push(NavigationMemoryPackInput {
            run_id: row.run_id.clone(), source_event_id: row.source_event_id.clone(),
            source_event_seq: row.source_event_seq, memory_pack_ref: row.memory_pack_ref.clone(),
            memory_pack_hash: row.memory_pack_hash.clone(), memory_pack: pack,
        });
    }
    Ok(verified)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationMessageInput {
    pub(crate) source_kind: String,
    pub(crate) message_id: String,
    pub(crate) source_event_id: Option<String>,
    pub(crate) source_event_seq: Option<i64>,
    pub(crate) run_id: String,
    pub(crate) from_lane_id: String,
    pub(crate) to_lane: crate::swarm_orchestration::model_lane::ModelLaneTarget,
    pub(crate) source_lane: Option<NavigationSenderLaneInput>,
    pub(crate) kind: String,
    pub(crate) crdt_update_ref: Option<String>,
    pub(crate) crdt_base_snapshot_ref: Option<String>,
    pub(crate) crdt_state_vector: Option<String>,
    pub(crate) crdt_proposal_ref: Option<String>,
    pub(crate) crdt_stale_base_ref: Option<String>,
    pub(crate) binding: Option<crate::swarm_orchestration::model_lane::ModelLaneCrdtAuthorityBinding>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationContextInputs {
    pub(crate) source_kind: String,
    pub(crate) run_id: String,
    pub(crate) kind: String,
    pub(crate) inputs: Vec<NavigationContextSource>,
    pub(crate) artifact: Option<NavigationArtifactInput>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationArtifactInput {
    pub(crate) source_kind: String,
    pub(crate) run_id: String,
    pub(crate) artifact_binding_id: String,
    pub(crate) source_event_id: String,
    pub(crate) source_event_seq: i64,
    pub(crate) artifact_ref: String,
    pub(crate) artifact_manifest_ref: String,
    pub(crate) artifact_payload_ref: String,
    pub(crate) artifact_sha256: String,
    pub(crate) content_hash: String,
    pub(crate) payload_json: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationContextSource {
    pub(crate) context_event_id: String,
    pub(crate) context_event_seq: i64,
    pub(crate) input: Option<NavigationMessageInput>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationSenderLaneInput {
    pub(crate) lane: NavigationLaneInput,
    pub(crate) event_id: String,
    pub(crate) event_seq: i64,
    pub(crate) ordering_seq: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationLaneInput {
    pub(crate) source_kind: String,
    pub(crate) lane_id: String,
    pub(crate) run_id: String,
    pub(crate) session_id: String,
    pub(crate) model_session_id: String,
    pub(crate) trace_id: String,
    pub(crate) kind: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationMtRuntimeInput {
    pub(crate) source_kind: String,
    pub(crate) kind: String,
    pub(crate) mt_status_id: String,
    pub(crate) run_id: String,
    pub(crate) source_event_id: String,
    pub(crate) source_event_seq: i64,
    pub(crate) micro_task_id: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct NavigationErrorInput {
    pub(crate) source_kind: String,
    pub(crate) run_id: String,
    pub(crate) failstate_code: Option<String>,
    pub(crate) error_code: Option<String>,
}

#[derive(SurrealValue)]
struct NavigationApplicationReceiptRow {
    event_id: String,
    event_sequence: i64,
    semantic: Option<CrdtProposalSemanticInput>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, SurrealValue)]
struct NavigationApplicationEvidence {
    schema_id: String,
    operation: String,
    proposal_id: String,
    document_id: String,
    crdt_document_id: String,
    recorded_event_id: String,
    approval_event_id: String,
    base_update_id: String,
    base_snapshot_id: String,
    base_snapshot_event_id: String,
    base_update_event_id: String,
    update_id: String,
    update_event_id: String,
    approved_diff_sha256: String,
    yjs_update_sha256: String,
    canonical_base_materialization_sha256: String,
    canonical_post_materialization_sha256: String,
    base_update_seq: i64,
    update_seq: i64,
    approval_event_sequence: i64,
    update_event_sequence: i64,
    path: Vec<String>,
}

#[derive(Clone, serde::Deserialize, SurrealValue)]
struct CrdtProposalSemanticInput {
    schema_id: String,
    proposal_id: Option<String>,
    diff_sha256: Option<String>,
    document_id: Option<String>,
    crdt_document_id: Option<String>,
    recorded_event_id: Option<String>,
    reviewer_principal_id: Option<String>,
    reviewer_authenticated_session_id: Option<String>,
    reviewer_action: Option<String>,
    reviewer_capability_id: Option<String>,
    reviewer_resource_id: Option<String>,
    reviewer_grant_id: Option<String>,
    review_state: Option<String>,
    applied_update_id: Option<String>,
    applied_update_sha256: Option<String>,
    approved_diff_sha256: Option<String>,
    base_state_vector: Option<String>,
    decided_by: Option<String>,
    base_update_seq: Option<i64>,
    application_evidence: Option<NavigationApplicationEvidence>,
}
impl CrdtProposalSemanticInput {
    fn string(&self, field: &str) -> Option<&str> {
        match field {
            "proposal_id" => self.proposal_id.as_deref(),
            "diff_sha256" => self.diff_sha256.as_deref(),
            "document_id" => self.document_id.as_deref(),
            "crdt_document_id" => self.crdt_document_id.as_deref(),
            "recorded_event_id" => self.recorded_event_id.as_deref(),
            "reviewer_principal_id" => self.reviewer_principal_id.as_deref(),
            "reviewer_authenticated_session_id" => self.reviewer_authenticated_session_id.as_deref(),
            "reviewer_action" => self.reviewer_action.as_deref(),
            "reviewer_capability_id" => self.reviewer_capability_id.as_deref(),
            "reviewer_resource_id" => self.reviewer_resource_id.as_deref(),
            "reviewer_grant_id" => self.reviewer_grant_id.as_deref(),
            "review_state" => self.review_state.as_deref(),
            "applied_update_id" => self.applied_update_id.as_deref(),
            "applied_update_sha256" => self.applied_update_sha256.as_deref(),
            "approved_diff_sha256" => self.approved_diff_sha256.as_deref(),
            "base_state_vector" => self.base_state_vector.as_deref(),
            "decided_by" => self.decided_by.as_deref(),
            _ => None,
        }
    }
}

#[derive(Clone, SurrealValue)]
struct CrdtProposalSemanticReceiptRow {
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
    source_component: String,
    semantic: CrdtProposalSemanticInput,
}
impl CrdtProposalReceiptRow {
    fn into_semantic(self) -> Result<CrdtProposalSemanticReceiptRow, SurrealStorageError> {
        if self.payload_hash != sha256_hex(&canonical_json_bytes(&self.payload)) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "CRDT proposal canonical EventLedger payload hash differs" });
        }
        let mut original = self.payload;
        if !original.is_object() {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "CRDT proposal semantic input is not an object" });
        }
        original["schema_id"] = Value::String("hsk.model_lane_navigation_receipt_input@1".into());
        let semantic = serde_json::from_value(original).map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal semantic input has invalid types",
        })?;
        Ok(CrdtProposalSemanticReceiptRow {
            event_id: self.event_id,
            event_version: self.event_version,
            kernel_task_run_id: self.kernel_task_run_id,
            session_run_id: self.session_run_id,
            aggregate_type: self.aggregate_type,
            aggregate_id: self.aggregate_id,
            idempotency_key: self.idempotency_key,
            event_type: self.event_type,
            actor_kind: self.actor_kind,
            actor_id: self.actor_id,
            causation_id: self.causation_id, correlation_id: self.correlation_id,
            source_component: self.source_component, semantic,
        })
    }
}

#[derive(Debug, SurrealValue)]
struct CrdtUpdateIdRow {
    update_id: String,
}

#[derive(Clone, Debug, SurrealValue)]
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
    source_component: String,
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
    event_ledger_stream_id: Option<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
pub(crate) struct StoredEventLink {
    pub(crate) event_id: String,
    pub(crate) event_seq: i64,
    pub(crate) event_stream_version: i64,
    pub(crate) transaction_seq: i64,
    pub(crate) aggregate_id: String,
    pub(crate) run_id: String,
    pub(crate) session_run_id: String,
    pub(crate) kernel_task_run_id: String,
    pub(crate) payload_hash: String,
    pub(crate) event_payload_json: String,
}

#[derive(SurrealValue)]
struct CanonicalScopeRecord {
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    event_id: String,
    event_seq: i64,
    event_stream_version: i64,
    transaction_seq: i64,
}

#[derive(SurrealValue)]
struct CanonicalScopeQueryResult {
    scope_records: Vec<CanonicalScopeRecord>,
    ordered_ledger: Vec<StoredEventLink>,
}

pub(crate) struct SurrealModelLaneCanonicalSet {
    scope_records: Vec<(ModelLaneRecordKind, SurrealModelLaneRecord)>,
    ordered_ledger: Vec<StoredEventLink>,
}

impl SurrealModelLaneCanonicalSet {
    pub(crate) fn into_parts(self) -> (
        Vec<(ModelLaneRecordKind, SurrealModelLaneRecord)>, Vec<StoredEventLink>,
    ) {
        (self.scope_records, self.ordered_ledger)
    }
}

#[derive(Debug, SurrealValue)]
struct RunEventBindings {
    run_id: String,
    event_ledger_stream_id: Option<String>,
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
    ("hsk.operator_chat_launch_admission@1", 1),
    ("hsk.operator_chat_launch_admission_transition@1", 1),
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

macro_rules! model_lane_derived_schema_refresh {
    () => { "UPDATE model_lane_authority SET navigation_source_input = fn::wp012_model_lane_navigation_source_input($this) WHERE navigation_source_input != fn::wp012_model_lane_navigation_source_input($this); UPDATE kernel_event_ledger SET navigation_semantic_input = fn::wp012_navigation_receipt_semantics($this) WHERE navigation_semantic_input != fn::wp012_navigation_receipt_semantics($this);" };
}

macro_rules! model_lane_registry_seed_write {
    () => { "LET $id = type::record('model_lane_schema_registry', $record_id); LET $existing = SELECT * FROM ONLY $id; IF $existing = NONE { CREATE ONLY $id CONTENT $seed; } ELSE IF $existing.schema_id != $seed.schema_id OR $existing.schema_version != $seed.schema_version OR $existing.record_kind != $seed.record_kind OR $existing.table_name != $seed.table_name OR $existing.source_component != $seed.source_component OR $existing.owner_account_id != $seed.owner_account_id OR $existing.actor_principal_id != $seed.actor_principal_id OR $existing.authenticated_session_id != $seed.authenticated_session_id OR $existing.access_space_id != $seed.access_space_id OR $existing.workspace_id != $seed.workspace_id { THROW 'model-lane schema registry metadata drift'; };" };
}

#[derive(SurrealValue)]
struct ModelLaneSchemaRegistrySeed {
    record_id: String,
    seed: StoredSchemaWrite,
}

#[derive(SurrealValue)]
struct ModelLaneSchemaRegistrySource {
    session: SchemaSessionScope,
    workspaces: Vec<SchemaWorkspace>,
}

#[derive(SurrealValue)]
struct ModelLaneSchemaRegistryInputs {
    sources: Vec<ModelLaneSchemaRegistrySource>,
    seeds: Vec<ModelLaneSchemaRegistrySeed>,
}

async fn model_lane_schema_registry_inputs(
    database: &super::SurrealAdminContext<'_>,
) -> Result<ModelLaneSchemaRegistryInputs, SurrealStorageError> {
    // Only administrative preparation discovers persisted session/workspace tuples.
    // Runtime reads neither create missing metadata nor repair drifted metadata.
    let mut response = database.query("SELECT record::id(account_id) AS owner_account_id, record::id(principal_id) AS actor_principal_id, record::id(id) AS authenticated_session_id, record::id(access_space_id) AS access_space_id FROM authenticated_sessions;").await?;
    let sessions: Vec<SchemaSessionScope> = response.take(0)?;
    let mut sources = Vec::with_capacity(sessions.len());
    let mut registry_seeds = Vec::new();
    for session in sessions {
        let mut response = database.query_bound(
            "SELECT external_resource_id AS workspace_id FROM protected_resources WHERE resource_kind = 'workspace' AND owner_account_id = type::record('local_accounts', $owner_account_id) AND access_space_id = type::record('access_spaces', $access_space_id) AND lifecycle_state = 'active';",
            SchemaWorkspaceBindings { owner_account_id: session.owner_account_id.clone(), access_space_id: session.access_space_id.clone() },
        ).await?;
        let workspaces: Vec<SchemaWorkspace> = response.take(0)?;
        for workspace in &workspaces {
            let scope = ModelLaneScope {
                owner_account_id: session.owner_account_id.clone(), actor_principal_id: session.actor_principal_id.clone(),
                authenticated_session_id: session.authenticated_session_id.clone(), access_space_id: session.access_space_id.clone(),
                workspace_id: workspace.workspace_id.clone(),
            };
            validate_scope(&scope)?;
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
            seeds.push(ModelLaneSchemaRegistrySeed {
                record_id,
                seed:
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
            });
        }

            registry_seeds.extend(seeds);
        }
        sources.push(ModelLaneSchemaRegistrySource { session, workspaces });
    }
    Ok(ModelLaneSchemaRegistryInputs { sources, seeds: registry_seeds })
}

async fn prepare_model_lane_schema_registry(
    database: &super::SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    for seed in model_lane_schema_registry_inputs(database).await?.seeds {
        let mut bindings = surrealdb::types::Object::new();
        bindings.insert("record_id", seed.record_id);
        bindings.insert("seed", seed.seed);
        database.query_bound(model_lane_registry_seed_write!(), bindings).await?;
    }
    Ok(())
}

#[derive(SurrealValue)]
pub(super) struct PreparedModelLaneSchemaUpgrade {
    model_row: surrealdb::types::Object,
    global_row: surrealdb::types::Object,
    registry: ModelLaneSchemaRegistryInputs,
    schema_version: String,
    schema_revision: i64,
}

pub(super) async fn prepare_model_lane_schema_upgrade(
    database: &super::SurrealAdminContext<'_>,
) -> Result<PreparedModelLaneSchemaUpgrade, SurrealStorageError> {
    prepare_model_lane_schema_upgrade_from(database, RUN_MEMORY_PREVIOUS_SCHEMA_VERSION,
        RUN_MEMORY_PREVIOUS_SCHEMA_REVISION, "model-lane schema upgrade requires the complete v8 primary receipt").await
}

pub(super) async fn prepare_routing_model_lane_schema_upgrade(
    database: &super::SurrealAdminContext<'_>,
) -> Result<PreparedModelLaneSchemaUpgrade, SurrealStorageError> {
    prepare_model_lane_schema_upgrade_from(database, ROUTING_PREVIOUS_SCHEMA_VERSION,
        ROUTING_PREVIOUS_SCHEMA_REVISION, "routing schema upgrade requires the complete v9 primary receipt").await
}

async fn prepare_model_lane_schema_upgrade_from(
    database: &super::SurrealAdminContext<'_>, version: &'static str, revision: i64, reason: &'static str,
) -> Result<PreparedModelLaneSchemaUpgrade, SurrealStorageError> {
    let mut bindings = surrealdb::types::Object::new();
    bindings.insert("schema_version", version.to_owned());
    bindings.insert("schema_revision", revision);
    let mut response = database.query_bound(
        "SELECT * FROM model_lane_schema_state WHERE id = model_lane_schema_state:primary AND schema_version = $schema_version AND schema_revision = $schema_revision AND apply_state = 'complete';",
        bindings,
    ).await?;
    let mut rows: Vec<surrealdb::types::Object> = response.take(0)?;
    if rows.len() != 1 {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason,
        });
    }
    let model_row = rows.remove(0);
    let mut response = database.query("SELECT * FROM ONLY handshake_schema_state:primary;").await?;
    let global_row: Option<surrealdb::types::Object> = response.take(0)?;
    let global_row = global_row.ok_or(SurrealStorageError::InvalidModelLaneRecord {
        reason: "model-lane schema upgrade requires the existing global receipt",
    })?;
    Ok(PreparedModelLaneSchemaUpgrade {
        model_row,
        global_row,
        registry: model_lane_schema_registry_inputs(database).await?,
        schema_version: SCHEMA_VERSION.to_owned(),
        schema_revision: SCHEMA_REVISION,
    })
}

impl PreparedModelLaneSchemaUpgrade {
    pub(super) fn routing_sql_fragment(&self) -> String {
        format!("{}\n{}", SCHEMA, self.sql_fragment())
    }

    pub(super) fn sql_fragment(&self) -> &'static str {
        concat!(
            r#"
LET $model_upgrade = $wp012_model_lane_upgrade;
LET $model_before = SELECT * FROM ONLY model_lane_schema_state:primary;
LET $global_before = SELECT * FROM ONLY handshake_schema_state:primary;
IF $model_before != $model_upgrade.model_row OR $global_before != $model_upgrade.global_row {
    THROW 'model-lane schema upgrade receipt changed';
};
LET $sessions = SELECT record::id(account_id) AS owner_account_id, record::id(principal_id) AS actor_principal_id, record::id(id) AS authenticated_session_id, record::id(access_space_id) AS access_space_id FROM authenticated_sessions;
IF array::len($sessions) != array::len($model_upgrade.registry.sources) {
    THROW 'model-lane schema upgrade session sources changed';
};
FOR $source IN $model_upgrade.registry.sources {
    IF array::len($sessions[WHERE $this = $source.session]) != 1 {
        THROW 'model-lane schema upgrade session source changed';
    };
    LET $workspaces = SELECT external_resource_id AS workspace_id FROM protected_resources WHERE resource_kind = 'workspace' AND owner_account_id = type::record('local_accounts', $source.session.owner_account_id) AND access_space_id = type::record('access_spaces', $source.session.access_space_id) AND lifecycle_state = 'active';
    IF array::len($workspaces) != array::len($source.workspaces) {
        THROW 'model-lane schema upgrade workspace sources changed';
    };
    FOR $workspace IN $source.workspaces {
        IF array::len($workspaces[WHERE $this = $workspace]) != array::len($source.workspaces[WHERE $this = $workspace]) {
            THROW 'model-lane schema upgrade workspace source changed';
        };
    };
};
"#,
            model_lane_derived_schema_refresh!(),
            " FOR $registry_seed IN $model_upgrade.registry.seeds { LET $record_id = $registry_seed.record_id; LET $seed = $registry_seed.seed; ",
            model_lane_registry_seed_write!(),
            r#"
};
UPDATE ONLY model_lane_schema_state:primary SET schema_version = $model_upgrade.schema_version, schema_revision = $model_upgrade.schema_revision, apply_state = 'complete';
"#
        )
    }

    pub(super) fn append_native_bindings(self, bindings: &mut surrealdb::types::Object) {
        bindings.insert("wp012_model_lane_upgrade", self);
    }
}

#[derive(SurrealValue)]
struct SchemaSessionScope { owner_account_id: String, actor_principal_id: String, authenticated_session_id: String, access_space_id: String }
#[derive(SurrealValue)]
struct SchemaWorkspaceBindings { owner_account_id: String, access_space_id: String }
#[derive(SurrealValue)]
struct SchemaWorkspace { workspace_id: String }

// FINAL catalog reuse requires the owning durable receipt; this never installs DDL.
pub(super) async fn verify_schema_receipt(
    database: &super::SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database.query(format!("SELECT * FROM ONLY {SCHEMA_STATE_ID};")).await?;
    let state: Option<ModelLaneSchemaState> = response.take(0)?;
    if !state.as_ref().is_some_and(|state| {
        state.schema_version == SCHEMA_VERSION
            && state.schema_revision == SCHEMA_REVISION
            && state.apply_state == "complete"
    }) {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "model-lane schema receipt missing or version/revision/state mismatch",
        });
    }
    Ok(())
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
                        if ((state.schema_version == ROUTING_PREVIOUS_SCHEMA_VERSION
                            && state.schema_revision == ROUTING_PREVIOUS_SCHEMA_REVISION)
                            || (state.schema_version == RUN_MEMORY_PREVIOUS_SCHEMA_VERSION
                            && state.schema_revision == RUN_MEMORY_PREVIOUS_SCHEMA_REVISION)
                            || (state.schema_version == NAVIGATION_PREVIOUS_SCHEMA_VERSION
                            && state.schema_revision == NAVIGATION_PREVIOUS_SCHEMA_REVISION)
                            || (state.schema_version == PREVIOUS_SCHEMA_VERSION
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
                // Re-evaluate only derived inputs on retained rows. Original canonical
                // JSON, receipt payload/hash and event sequence are never replaced.
                database.query(model_lane_derived_schema_refresh!()).await?;
                prepare_model_lane_schema_registry(&database).await?;
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
    #[cfg(test)]
    pub(crate) fn observe_current_context_guards(&self) -> Option<ContextGuardObservation> {
        super::MODEL_LANE_CURRENT_GUARDS
            .try_with(|guards| ContextGuardObservation {
                guards: std::sync::Arc::clone(guards),
                storage: self.storage.clone(),
            })
            .ok()
    }

    pub(crate) async fn initialize(storage: SurrealStorage) -> Result<Self, SurrealStorageError> {
        // Schema and canonical registry preparation belong to administrative bootstrap, not an authenticated business access.
        Ok(Self { storage, #[cfg(feature = "test-utils")] admission_test_control: ContextAdmissionTestControl::default(),
            #[cfg(feature = "test-utils")] completion_test_control: ContextCompletionTestControl::default() })
    }

    pub(crate) fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub(crate) fn run_attachment_is_guarded(&self, sources: &std::sync::Arc<super::resource_authority::RoutingOriginalSources>) -> bool {
        super::MODEL_LANE_CURRENT_GUARDS.try_with(|guards| {
            guards.lock().map(|guards| guards.iter().any(|guard| {
                guard.purpose == ContextCoordinationPurpose::RunAttachment
                    && guard.routing_sources.as_ref().is_some_and(|held| std::sync::Arc::ptr_eq(held, sources))
                    && guard.require_original_envelope().is_ok()
            })).unwrap_or(false)
        }).unwrap_or(false)
    }

    pub(crate) fn routing_operation_is_guarded(
        &self, sources: &std::sync::Arc<super::resource_authority::RoutingOriginalSources>,
    ) -> bool {
        super::MODEL_LANE_CURRENT_GUARDS.try_with(|guards| {
            guards.lock().ok().is_some_and(|guards| guards.iter().any(|guard|
                matches!(guard.purpose, ContextCoordinationPurpose::RoutingCreate | ContextCoordinationPurpose::RoutingTransition)
                && guard.routing_sources.as_ref().is_some_and(|owner| std::sync::Arc::ptr_eq(owner, sources))
                && guard.require_original_envelope().is_ok()))
        }).unwrap_or(false)
    }

    pub(crate) async fn routing_execution_snapshot(
        &self,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRoutingExecutionRow>, SurrealStorageError> {
        self.routing_execution_snapshot_with_query(&ModelLaneQuery::Storage(&self.storage), execution_id, scope).await
    }

    pub(crate) async fn routing_execution_snapshot_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRoutingExecutionRow>, SurrealStorageError> {
        self.routing_execution_snapshot_with_query(&ModelLaneQuery::Transaction(transaction), execution_id, scope).await
    }

    async fn routing_execution_snapshot_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRoutingExecutionRow>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(execution_id, execution_id)?;
        let bindings = routing_identity_bindings(execution_id, scope);
        let rows = query
                        .query_values::<SurrealModelLaneRoutingExecutionRow, _>(
                            "SELECT execution_id, run_id, revision, context_hash, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM model_lane_routing_execution WHERE execution_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_execution' AND event_ledger_event_id.aggregate_id = $aggregate_id AND event_ledger_seq = event_ledger_event_id.event_sequence LIMIT 2;",
                            bindings,
                        )
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
        self.routing_attempts_for_execution_with_query(&ModelLaneQuery::Storage(&self.storage), execution_id, scope).await
    }

    pub(crate) async fn routing_attempts_for_execution_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingAttemptRow>, SurrealStorageError> {
        self.routing_attempts_for_execution_with_query(&ModelLaneQuery::Transaction(transaction), execution_id, scope).await
    }

    async fn routing_attempts_for_execution_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingAttemptRow>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(execution_id, execution_id)?;
        let bindings = routing_identity_bindings(execution_id, scope);
        query
                        .query_values::<SurrealModelLaneRoutingAttemptRow, _>(
                            "SELECT attempt_id, execution_id, run_id, stage_id, attempt, state, lease_owner, fencing_token, lease_expires_at_unix_ms, record_json, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq FROM model_lane_routing_stage_attempt WHERE execution_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_stage_attempt' AND event_ledger_event_id.aggregate_id = attempt_id AND event_ledger_seq = event_ledger_event_id.event_sequence ORDER BY stage_id ASC, attempt ASC LIMIT 4097;",
                            bindings,
                        )
                        .await
    }

    pub(crate) async fn routing_outbox_for_execution(
        &self,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingOutboxRow>, SurrealStorageError> {
        self.routing_outbox_for_execution_with_query(&ModelLaneQuery::Storage(&self.storage), execution_id, scope).await
    }

    pub(crate) async fn routing_outbox_for_execution_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingOutboxRow>, SurrealStorageError> {
        self.routing_outbox_for_execution_with_query(&ModelLaneQuery::Transaction(transaction), execution_id, scope).await
    }

    async fn routing_outbox_for_execution_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        execution_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRoutingOutboxRow>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_identity(execution_id, execution_id)?;
        let bindings = routing_identity_bindings(execution_id, scope);
        query
                        .query_values::<SurrealModelLaneRoutingOutboxRow, _>(
                            "SELECT command_id, execution_id, run_id, stage_id, attempt, status, lease_owner, fencing_token, lease_expires_at_unix_ms, record::id(event_ledger_event_id) AS event_ledger_event_id, event_ledger_seq, created_at_unix_ms, updated_at_unix_ms FROM model_lane_routing_outbox WHERE execution_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.aggregate_type = 'model_lane_routing_outbox' AND event_ledger_event_id.aggregate_id = command_id AND event_ledger_seq = event_ledger_event_id.event_sequence ORDER BY stage_id ASC, attempt ASC LIMIT 4097;",
                            bindings,
                        )
                        .await
    }

    pub(crate) async fn commit_routing_atomic(
        &self,
        commit: SurrealModelLaneRoutingCommit,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRoutingExecutionRow, SurrealStorageError> {
        self.commit_routing_atomic_with_query(&ModelLaneQuery::Storage(&self.storage), commit, scope).await
    }

    pub(crate) async fn commit_routing_atomic_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        commit: SurrealModelLaneRoutingCommit,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRoutingExecutionRow, SurrealStorageError> {
        self.commit_routing_atomic_with_query(&ModelLaneQuery::Transaction(transaction), commit, scope).await
    }

    async fn commit_routing_atomic_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        commit: SurrealModelLaneRoutingCommit,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRoutingExecutionRow, SurrealStorageError> {
        validate_routing_commit(&commit, scope)?;
        let bindings = routing_commit_bindings(commit, scope)?;
        let rows = query
                        .query_values_at::<SurrealModelLaneRoutingExecutionRow, _>(
                            ROUTING_COMMIT_QUERY,
                            bindings,
                            11,
                        )
                        .await?;
        single_routing_row(rows, "atomic routing commit returned ambiguous authority")?.ok_or(
            SurrealStorageError::InvalidModelLaneRecord {
                reason: "atomic routing commit returned no execution authority",
            },
        )
    }


    // Account retains the publication fence and installs the original authenticated scope.
    // Expected rows are captured after the actual CAS, never constructed from the request.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn readback_routing_commit(
        &self,
        publication: &super::resource_authority::RolePublicationGuard<'_>,
        operation_id: uuid::Uuid,
        issuer: &ModelLaneScope,
        expected_execution: &SurrealModelLaneRoutingExecutionRow,
        expected_attempts: &[SurrealModelLaneRoutingAttemptRow],
        expected_outbox: &[SurrealModelLaneRoutingOutboxRow],
    ) -> Result<(
        SurrealModelLaneRoutingExecutionRow,
        Vec<SurrealModelLaneRoutingAttemptRow>,
        Vec<SurrealModelLaneRoutingOutboxRow>,
    ), SurrealStorageError> {
        validate_scope(issuer)?;
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        if publication.operation_id() != operation_id
            || publication.current_scope().owner_account_id.to_string() != issuer.owner_account_id
            || publication.current_scope().workspace_id.as_str() != issuer.workspace_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let before = self.routing_execution_snapshot(&expected_execution.execution_id, issuer).await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "independent routing readback is missing the committed execution",
            })?;
        let attempts = self.routing_attempts_for_execution(&expected_execution.execution_id, issuer).await?;
        let outbox = self.routing_outbox_for_execution(&expected_execution.execution_id, issuer).await?;
        // LIMIT 4097 is an overflow sentinel, not evidence of a complete child set.
        if attempts.len() >= 4097 || outbox.len() >= 4097 {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "independent routing readback reached the attempt/outbox query limit",
            });
        }
        let after = self.routing_execution_snapshot(&expected_execution.execution_id, issuer).await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "independent routing readback lost the committed execution",
            })?;
        if &before != expected_execution || after != before
            || attempts.as_slice() != expected_attempts || outbox.as_slice() != expected_outbox {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "independent routing mutation readback differs from the actual CAS result",
            });
        }
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        Ok((after, attempts, outbox))
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
                        .map(|_| ())
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
                | "incomplete_attribution"
                | "blank_attribution"
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
                        .map(|_| ())
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
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
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
                        .map(|_| ())
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
            "content_mismatch" => Ok(SurrealCrdtUpdateAppendOutcome::ContentMismatch { update_id }),
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
        let purpose = match row.scope_kind.as_str() {
            "document" => ContextCoordinationPurpose::DocumentClaim,
            "workspace" => ContextCoordinationPurpose::WorkspaceClaim,
            "source_root" => ContextCoordinationPurpose::SourceRootClaim,
            "index_run" => ContextCoordinationPurpose::IndexRunClaim,
            _ => return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "CRDT claim scope kind is unsupported" }),
        };
        let store = self.clone();
        let owned_scope = scope.clone();
        self.with_crdt_covering_transaction(purpose, scope.clone(), move || async move {
            store.claim_crdt_lease_atomic_fenced(row, ttl_seconds, event, &owned_scope).await
        }).await
    }

    pub(crate) async fn claim_crdt_lease_atomic_fenced(
        &self,
        row: SurrealModelLaneCrdtLeaseWrite,
        ttl_seconds: i64,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<SurrealCrdtLeaseClaimOutcome, SurrealStorageError> {
        if !super::PROTECTED_TRANSACTION.try_with(|transaction| {
            std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)
        }).unwrap_or(false) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        validate_crdt_lease_write(&row, ttl_seconds, &event, scope)?;
        let bindings = CrdtLeaseClaimBindings {
            record_id: stable_record_id("crdt_lease", &row.lease_id, scope),
            ttl_seconds,
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
                            2,
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

    pub(crate) async fn crdt_lease_by_id(
        &self, lease_id: &str, scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            authority_ref: lease_id.to_owned(), owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(), authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
        };
        let mut rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<SurrealModelLaneCrdtLeaseHistory, _>(
                "SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 2;",
                bindings,
            ).await
        })).await?;
        if rows.len() > 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "CRDT lease identity is ambiguous" });
        }
        Ok(rows.pop())
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

    pub(crate) async fn expire_crdt_lease_atomic(
        &self,
        lease_id: &str,
        actor_id: &str,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        validate_scope(scope)?;
        validate_crdt_transition_event(lease_id, actor_id, 1, &event)?;
        validate_crdt_event_kind(&event, "knowledge_crdt_lease", "KNOWLEDGE_CRDT_LEASE_EXPIRED")?;
        self.mutate_crdt_lease(
            EXPIRE_CRDT_LEASE_QUERY, "expiry", lease_id, actor_id, 1, event, scope,
        ).await
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
        let purpose = match transition {
            "renew" => ContextCoordinationPurpose::LeaseRenew,
            "release" => ContextCoordinationPurpose::LeaseRelease,
            "expiry" => ContextCoordinationPurpose::LeaseExpiry,
            _ => return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT lease transition is unsupported",
            }),
        };
        if super::PROTECTED_TRANSACTION.try_with(|transaction| {
            std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)
        }).unwrap_or(false) {
            if !super::MODEL_LANE_OPERATION_PURPOSE.try_with(|actual| actual.as_str() == purpose.as_str()).unwrap_or(false) {
                return Err(SurrealStorageError::TransactionAuthorityRequired);
            }
            return self.mutate_crdt_lease_fenced(statement, transition, lease_id, actor_id, ttl_seconds, event, scope).await;
        }
        let store = self.clone();
        let owned_scope = scope.clone();
        let transition = transition.to_owned();
        let lease_id = lease_id.to_owned();
        let actor_id = actor_id.to_owned();
        self.with_crdt_covering_transaction(purpose, scope.clone(), move || async move {
            store.mutate_crdt_lease_fenced(
                statement, &transition, &lease_id, &actor_id, ttl_seconds, event, &owned_scope,
            ).await
        }).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn mutate_crdt_lease_fenced(
        &self,
        statement: &'static str,
        transition: &str,
        lease_id: &str,
        actor_id: &str,
        ttl_seconds: i64,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        if !super::PROTECTED_TRANSACTION.try_with(|transaction| {
            std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)
        }).unwrap_or(false) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let statement = statement.strip_prefix("\nBEGIN TRANSACTION;\n")
            .and_then(|query| query.strip_suffix("\nCOMMIT TRANSACTION;\n"))
            .ok_or(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT lease transaction delimiters are invalid",
            })?;
        let bindings = CrdtLeaseMutationBindings {
            lease_id: lease_id.to_owned(),
            actor_id: actor_id.to_owned(),
            transition: transition.to_owned(),
            ttl_seconds,
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

    async fn require_proposal_operation(
        &self, purpose: ContextCoordinationPurpose, scope: &ModelLaneScope, document_id: Option<&str>,
    ) -> Result<super::resource_authority::AuthorizationDecision, SurrealStorageError> {
        use super::resource_authority::{AuthorizationRequest, ResourceAction, ResourceKind};
        let original = super::current_record_user_scope().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let subject = super::current_model_lane_subject().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if !super::PROTECTED_TRANSACTION.try_with(|transaction| std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)).unwrap_or(false)
            || super::MODEL_LANE_OPERATION_PURPOSE.try_with(|actual| *actual != purpose).unwrap_or(true)
            || subject.kind != ResourceKind::RichDocument || original.action != ResourceAction::Update
            || original.capability_id != "fs.write" || document_id.is_some_and(|id| id != subject.external_id) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let decision = self.storage.authorize_protected_resource(AuthorizationRequest {
            session_token: original.session_token.clone(), channel_binding_hash: original.channel_binding_hash.clone(),
            capability_id: original.capability_id.clone(), resource_kind: subject.kind,
            external_resource_id: subject.external_id, action: original.action,
        }).await.map_err(SurrealStorageError::from)?;
        if decision.resource_id != original.resource_id || Some(&decision.grant_id) != original.grant_id.as_ref()
            || decision.account_id != scope.owner_account_id || decision.principal_id != scope.actor_principal_id
            || decision.session_id != scope.authenticated_session_id || decision.access_space_id != scope.access_space_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        Ok(decision)
    }

    pub(crate) async fn record_crdt_proposal_atomic(
        &self, row: SurrealModelLaneCrdtProposalWrite, event: SurrealModelLaneCrdtEventWrite, scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtProposalRecord, SurrealStorageError> {
        if super::PROTECTED_TRANSACTION.try_with(|transaction| std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)).unwrap_or(false) {
            self.require_proposal_operation(ContextCoordinationPurpose::ProposalRecord, scope, None).await?;
            return self.record_crdt_proposal_atomic_fenced(row, event, scope).await;
        }
        let store = self.clone();
        let owned_scope = scope.clone();
        
        self.with_crdt_covering_transaction(ContextCoordinationPurpose::ProposalRecord, scope.clone(), move || async move {
            store.record_crdt_proposal_atomic_fenced(row, event, &owned_scope).await
        }).await
    }

    async fn record_crdt_proposal_atomic_fenced(
        &self,
        row: SurrealModelLaneCrdtProposalWrite,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtProposalRecord, SurrealStorageError> {
        self.require_proposal_operation(ContextCoordinationPurpose::ProposalRecord, scope, Some(&row.document_id)).await?;
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
        &self, proposal_id: &str, review_state: &str, decided_by: &str, decision_reason: Option<String>, event: SurrealModelLaneCrdtEventWrite, promotion_accepted_event: Option<SurrealModelLaneCrdtEventWrite>, scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        if super::PROTECTED_TRANSACTION.try_with(|transaction| std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)).unwrap_or(false) {
            self.require_proposal_operation(ContextCoordinationPurpose::ProposalDecide, scope, None).await?;
            return self.decide_crdt_proposal_atomic_fenced(proposal_id, review_state, decided_by, decision_reason, event, promotion_accepted_event, scope).await;
        }
        let store = self.clone();
        let owned_scope = scope.clone();
        let (proposal_id, review_state, decided_by) = (proposal_id.to_owned(), review_state.to_owned(), decided_by.to_owned());
        self.with_crdt_covering_transaction(ContextCoordinationPurpose::ProposalDecide, scope.clone(), move || async move {
            store.decide_crdt_proposal_atomic_fenced(&proposal_id, &review_state, &decided_by, decision_reason, event, promotion_accepted_event, &owned_scope).await
        }).await
    }

    async fn decide_crdt_proposal_atomic_fenced(
        &self,
        proposal_id: &str,
        review_state: &str,
        decided_by: &str,
        decision_reason: Option<String>,
        event: SurrealModelLaneCrdtEventWrite,
        promotion_accepted_event: Option<SurrealModelLaneCrdtEventWrite>,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        let reviewer = self.require_proposal_operation(ContextCoordinationPurpose::ProposalDecide, scope, None).await?;
        let eligible = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_first::<bool, _>("RETURN fn::mt109_live_session() AND $auth.principal_id.status = 'enabled' AND $auth.principal_id.principal_kind IN ['human_account', 'break_glass_operator'] AND $auth.principal_id.actor_kind = 'operator';", ()).await
        })).await?;
        if eligible != Some(true) { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        if reviewer.actor_kind != "operator" || reviewer.actor_id != decided_by
            || event.actor_kind != reviewer.actor_kind || event.actor_id != reviewer.actor_id
            || event.session_run_id != reviewer.session_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
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
                            4,
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
        &self, proposal_id: &str, applied_update_id: &str, applied_update_sha256: &str, actor_id: &str, event: SurrealModelLaneCrdtEventWrite, scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        if super::PROTECTED_TRANSACTION.try_with(|transaction| std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)).unwrap_or(false) {
            self.require_proposal_operation(ContextCoordinationPurpose::ProposalApply, scope, None).await?;
            return self.bind_crdt_proposal_update_atomic_fenced(proposal_id, applied_update_id, applied_update_sha256, actor_id, event, scope).await;
        }
        let store = self.clone();
        let owned_scope = scope.clone();
        let (proposal_id, applied_update_id, applied_update_sha256, actor_id) = (proposal_id.to_owned(), applied_update_id.to_owned(), applied_update_sha256.to_owned(), actor_id.to_owned());
        self.with_crdt_covering_transaction(ContextCoordinationPurpose::ProposalApply, scope.clone(), move || async move {
            store.bind_crdt_proposal_update_atomic_fenced(&proposal_id, &applied_update_id, &applied_update_sha256, &actor_id, event, &owned_scope).await
        }).await
    }

    async fn bind_crdt_proposal_update_atomic_fenced(
        &self,
        proposal_id: &str,
        applied_update_id: &str,
        applied_update_sha256: &str,
        actor_id: &str,
        event: SurrealModelLaneCrdtEventWrite,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        self.require_proposal_operation(ContextCoordinationPurpose::ProposalApply, scope, None).await?;
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
        if self
            .crdt_proposal_record(proposal_id, scope)
            .await?
            .is_none()
        {
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

    #[cfg(feature = "test-utils")]
    pub(crate) fn completion_test_control(&self) -> ContextCompletionTestControl { self.completion_test_control.clone() }

    pub(crate) async fn with_context_bundle_transaction<T, E, F, Fut>(
        &self,
        purpose: ContextCoordinationPurpose,
        run_id: String,
        mut lane_ids: Vec<String>,
        scope: ModelLaneScope,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        lane_ids.sort();
        lane_ids.dedup();
        let mut domains = vec![("run", run_id.clone(), Some(run_id.clone()))];
        domains.extend(lane_ids.into_iter().map(|lane| ("lane", lane, Some(run_id.clone()))));
        self.supervise_context_operation(purpose, scope, domains, operation).await
    }

    async fn supervise_context_operation<T, E, F, Fut>(
        &self, purpose: ContextCoordinationPurpose, scope: ModelLaneScope,
        domains: Vec<(&'static str, String, Option<String>)>, operation: F,
    ) -> Result<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static, Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        let authority = super::current_record_user_scope().ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let subject = super::current_model_lane_subject();
        let routing_sources = super::resource_authority::current_routing_original_sources();
        let configured_registry = super::configured_capability_registry();
        let budget = self.storage.effective_statement_timeout();
        if budget.is_zero() { return Err(E::from(SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 })); }
        let deadline = std::time::Instant::now() + budget;
        let supervisor_id = uuid::Uuid::now_v7();
        let cancellation = super::ContextOperationCancellation::new();
        struct CallerLifetime(std::sync::Arc<super::ContextOperationCancellation>);
        impl Drop for CallerLifetime { fn drop(&mut self) { self.0.cancel(); } }
        let caller = CallerLifetime(cancellation.clone());
        let provider = self.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        #[cfg(feature = "test-utils")]
        let busy_observer = CONTEXT_COORDINATION_BUSY_OBSERVER.try_with(Clone::clone).ok();
        tokio::spawn(async move {
            let storage = provider.storage.clone();
            let work = async move {
                let mut guards = Vec::new();
                let mut domains = std::collections::VecDeque::from(domains);
                while let Some((kind, id, run)) = domains.pop_front() {
                    if cancellation.is_cancelled() {
                        provider.release_owned_guards(supervisor_id, guards).await;
                        return Err(E::from(SurrealStorageError::TransactionCancelled));
                    }
                    // Do not abandon an in-flight acquisition: observe its outcome
                    // before deciding which real guards are owned and releasable.
                    match provider.acquire_context_coordination(purpose, kind, &id, run.as_deref(), &scope).await {
                        Ok(guard) => guards.push(guard),
                        Err(error) => {
                            if matches!(error, SurrealStorageError::TransactionSettlementPending { .. }) {
                                provider.storage.retain_context_reconciliation(supervisor_id, guards);
                            } else {
                                provider.release_owned_guards(supervisor_id, guards).await;
                            }
                            return Err(E::from(error));
                        }
                    }
                    if matches!(purpose, ContextCoordinationPurpose::RecoveryExecute | ContextCoordinationPurpose::RecoveryCheckpointRecord) && kind == "run" {
                        let current = std::sync::Arc::new(std::sync::Mutex::new(guards.clone()));
                        let discovered = super::MODEL_LANE_CURRENT_GUARDS.scope(current, async {
                            let run = provider.get(ModelLaneRecordKind::Run, &id, &scope).await?
                                .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
                            let body: serde_json::Value = serde_json::from_str(&run.record_json)
                                .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "recovery run envelope is malformed" })?;
                            let declared = body.get("lane_ids").and_then(serde_json::Value::as_array)
                                .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "recovery run lane identities are missing" })?;
                            for lane in declared {
                                lane.as_str().filter(|lane| !lane.trim().is_empty())
                                    .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "recovery run lane identity is malformed" })?;
                            }
                            // Declared lanes may never have been created; only canonical rows own lane fences.
                            let mut lanes = std::collections::BTreeSet::new();
                            for lane in provider.list_run(ModelLaneRecordKind::Lane, &id, &scope).await? {
                                lanes.insert(lane.aggregate_id);
                            }
                            Ok::<_, SurrealStorageError>(lanes)
                        }).await;
                        match discovered {
                            Ok(mut lanes) => {
                                lanes.extend(domains.iter().filter(|(kind, _, _)| *kind == "lane")
                                    .map(|(_, lane, _)| lane.clone()));
                                domains.retain(|(kind, _, _)| *kind != "lane");
                                domains.extend(lanes.into_iter().map(|lane| ("lane", lane, Some(id.clone()))));
                            }
                            Err(error) => {
                                if matches!(error, SurrealStorageError::TransactionSettlementPending { .. }) {
                                    provider.storage.retain_context_reconciliation(supervisor_id, guards);
                                } else {
                                    provider.release_owned_guards(supervisor_id, guards).await;
                                }
                                return Err(E::from(error));
                            }
                        }
                    }
                }
                if cancellation.is_cancelled() {
                    provider.release_owned_guards(supervisor_id, guards).await;
                    return Err(E::from(SurrealStorageError::TransactionCancelled));
                }
                provider.execute_context_operation(guards, cancellation, operation).await
            };
            let scoped_work = async move {
                let work = async move {
                    let work = async move {
                        if let Some(subject) = subject { super::with_model_lane_subject(subject, work).await }
                        else { work.await }
                    };
                    if let Some(sources) = routing_sources {
                        super::resource_authority::with_routing_original_sources(sources, work).await
                    } else { work.await }
                };
                if let Some(registry) = configured_registry { super::with_configured_capability_registry(registry, work).await }
                else { work.await }
            };
            let observed_work = async move {
                #[cfg(feature = "test-utils")]
                if let Some(observer) = busy_observer { return CONTEXT_COORDINATION_BUSY_OBSERVER.scope(observer, scoped_work).await; }
                scoped_work.await
            };
            let outcome = storage.with_record_user_scope(authority,
                super::SurrealStorage::with_operation_deadline(deadline,
                    super::with_model_lane_operation(purpose, observed_work))).await;
            if sender.send(outcome).is_err() {
                tracing::info!(%supervisor_id, "ContextBundle caller detached; owned settlement and cleanup completed or retained for reconciliation");
            }
        });
        let outcome = tokio::time::timeout_at((deadline + self.storage.config().statement_timeout()).into(), receiver).await;
        drop(caller);
        match outcome {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(E::from(SurrealStorageError::TransactionWorker(error.to_string()))),
            Err(_) => Err(E::from(SurrealStorageError::TransactionSettlementPending { operation_id: supervisor_id })),
        }
    }

    async fn release_owned_guards(&self, operation_id: uuid::Uuid, guards: Vec<ContextCoordinationGuard>) {
        let retained = guards.clone();
        let mut pending = false;
        // Publication's exhausted deadline cannot prevent separately bounded
        // finalization under the unchanged original authority and subject.
        let release_deadline = std::time::Instant::now() + self.storage.config().statement_timeout();
        for guard in guards.into_iter().rev() {
            if let Err(error) = super::SurrealStorage::with_operation_deadline(
                release_deadline, guard.release(&self.storage),
            ).await {
                pending = true;
                tracing::error!(%operation_id, %error, "owned ContextBundle release remains pending");
            }
        }
        if pending { self.storage.retain_context_reconciliation(operation_id, retained); }
    }

    async fn execute_context_operation<T, E, F, Fut>(
        &self,
        guards: Vec<ContextCoordinationGuard>,
        cancellation: std::sync::Arc<super::ContextOperationCancellation>,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        let rollback_guards = guards.clone();
        let operation_id = guards.iter().find_map(|guard| guard.routing_sources.as_ref())
            .map(|sources| sources.operation_id()).unwrap_or_else(uuid::Uuid::now_v7);
        let routing_sources = guards.iter().find_map(|guard| guard.routing_sources.clone());
        let outcome = self.storage.with_owned_authenticated_transaction(operation_id, cancellation, move |transaction| Box::pin(async move {
            let work = async move {
            let mut current = Vec::with_capacity(guards.len());
            for guard in guards {
                current.push(guard.fence_in_transaction(&transaction).await.map_err(E::from)?);
            }
            let current = std::sync::Arc::new(std::sync::Mutex::new(current));
            let value = super::MODEL_LANE_CURRENT_GUARDS.scope(current.clone(), async move { operation().await }).await?;
            let current = current.lock().map_err(|_| E::from(SurrealStorageError::InvalidModelLaneRecord { reason: "coordination guard state poisoned" }))?.clone();
            // Final engine authorization/token checks share the transaction
            // that read the canonical dependencies and publishes the result.
            let mut final_guards = Vec::with_capacity(current.len());
            for guard in current {
                final_guards.push(guard.fence_in_transaction(&transaction).await.map_err(E::from)?);
            }
            Ok((value, final_guards))
            };
            if let Some(sources) = routing_sources {
                super::with_configured_capability_registry(sources.registry(),
                    super::resource_authority::with_routing_original_sources(sources, work)).await
            } else { work.await }
        })).await;
        match outcome {
            Ok((value, guards)) => {
                #[cfg(feature = "test-utils")]
                self.completion_test_control.after_observed_commit(operation_id).await;
                let publication_sources = guards.iter().filter_map(|guard| guard.routing_sources.clone()).collect::<Vec<_>>();
                self.release_owned_guards(operation_id, guards).await;
                if self.storage.context_reconciliation_pending(operation_id) {
                    return Err(E::from(SurrealStorageError::TransactionSettlementPending { operation_id }));
                }
                for sources in publication_sources {
                    sources.revalidate().await.map_err(|error| E::from(match error {
                        super::resource_authority::ResourceAuthorityError::Storage(error) => error,
                        _ => SurrealStorageError::TransactionAuthorityRequired,
                    }))?;
                }
                Ok(value)
            }
            Err(error) => {
                // Unknown settlement retains ownership. Only observed rollback
                // permits release with the original, unrotated guards.
                if !self.storage.transaction_settlement_pending(operation_id) {
                    self.release_owned_guards(operation_id, rollback_guards).await;
                } else {
                    self.storage.retain_context_reconciliation(operation_id, rollback_guards);
                }
                Err(error)
            }
        }
    }

    // Both durable checks and nested reads share the original authenticated
    // transaction. The controller releases the result only after observed commit.
    pub(crate) async fn with_navigation_read_transaction<T, E, F, Fut>(
        &self, scope: ModelLaneScope, selector: NavigationReadSelector, operation: F,
    ) -> Result<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: FnOnce(Vec<NavigationMemoryPackInput>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        use super::resource_authority::{ResourceAction, ResourceKind};
        validate_scope(&scope).map_err(E::from)?;
        selector.validate().map_err(E::from)?;
        let original = super::current_record_user_scope()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let subject = super::current_model_lane_subject()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        if subject.kind != ResourceKind::Workspace || subject.external_id != scope.workspace_id
            || original.action != ResourceAction::Read
            || original.capability_id != "model_lane.navigation.read" || original.grant_id.is_none()
            || original.workspace_id.as_deref() != Some(scope.workspace_id.as_str())
            || original.session_id != scope.authenticated_session_id {
            return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
        }
        let expected_grant_id = original.grant_id
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let bindings = NavigationReadBindings {
            navigation_selection: selector.clone(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id, workspace_id: scope.workspace_id,
        };
        let association = super::RESOURCE_ACCESS_AUTHORIZATION.try_with(Clone::clone).ok();
        let lifecycle = self.storage.resource_access_lifecycle();
        super::with_model_lane_operation(ContextCoordinationPurpose::NavigationRead,
            self.storage.with_authenticated_transaction(uuid::Uuid::now_v7(), move |transaction| Box::pin(async move {
                let before = transaction.query_first::<NavigationAuthoritySnapshot, _>(
                    NAVIGATION_AUTHORITY_SNAPSHOT, bindings.clone(),
                ).await.map_err(E::from)?.ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
                let before_grants = transaction.query_values_at::<NavigationGrantSnapshot, _>(
                    NAVIGATION_GRANT_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                if !before_grants.iter().any(|grant| grant.grant_id == expected_grant_id) {
                    return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                }
                if let Some(association) = &association {
                    match lifecycle.authorize_association(association) {
                        Ok(()) => {},
                        Err(crate::swarm_orchestration::resource_scope::ScopeDenied::LifecycleAuthorityUnavailable) => return Err(E::from(SurrealStorageError::ResourceAuthorityUnavailable)),
                        Err(_) => return Err(E::from(SurrealStorageError::TransactionAuthorityRequired)),
                    }
                    if association.grant_id != expected_grant_id || association.resource_id != original.resource_id
                        || association.exact.owner_account_id.to_string() != bindings.owner_account_id
                        || association.exact.actor_principal_id.to_string() != bindings.actor_principal_id
                        || association.exact.authenticated_session_id.to_string() != bindings.authenticated_session_id
                        || association.exact.access_space_id.to_string() != bindings.access_space_id
                        || association.exact.workspace_id.as_str() != bindings.workspace_id
                        || association.capability_id != original.capability_id || association.action != "read"
                        || association.purpose != "navigation_read"
                        || !before_grants.iter().any(|grant| grant.grant_id == association.grant_id
                            && grant.resource_id == association.resource_id && grant.grant_version == association.grant_version
                            && grant.grant_policy_version == association.policy_version)
                        {
                        return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                    }
                }
                let before_sources = transaction.query_values_at::<SurrealModelLaneNavigationRow, _>(
                    NAVIGATION_SOURCE_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let before_receipts = transaction.query_values_at::<NavigationDependencyReceipt, _>(
                    NAVIGATION_RECEIPT_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                // Includes actual sender lane/receipt identity even when the route
                // hides that lane from the public scalar collection.
                let before_inputs = transaction.query_values_at::<NavigationSourceInputRow, _>(
                    NAVIGATION_INPUT_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let before_packs = transaction.query_values_at::<NavigationMemoryPackRow, _>(
                    NAVIGATION_MEMORY_PACK_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let before_document_links = transaction.query_values_at::<NavigationMemoryPackDocumentLink, _>(
                    NAVIGATION_MEMORY_DOCUMENT_LINK_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let candidates = navigation_memory_pack_inputs(
                    &bindings.workspace_id, &before_sources, &before_inputs, &before_packs,
                ).map_err(E::from)?;
                let selection = bindings.artifact_selection_value()
                    .map_err(|_| E::from(SurrealStorageError::InvalidModelLaneRecord {
                        reason: "navigation artifact selection encoding failed",
                    }))?;
                let mut memory_packs = Vec::with_capacity(candidates.len());
                let mut consulted_memory_packs = Vec::with_capacity(candidates.len());
                for input in candidates {
                    if navigation_memory_pack_authorized(&transaction, &bindings, &input, &before_document_links).await.map_err(E::from)? {
                        if let Some(registrations) = navigation_memory_artifact_registrations(
                            &transaction, &bindings, association.as_ref().map(|value| &value.exact), &selection, &input,
                        ).await.map_err(E::from)? {
                            consulted_memory_packs.push((input.clone(), registrations));
                            memory_packs.push(input);
                        }
                    }
                }
                let value = NAVIGATION_READ_SELECTOR.scope(selector, operation(memory_packs)).await?;
                let after = transaction.query_first::<NavigationAuthoritySnapshot, _>(
                    NAVIGATION_AUTHORITY_SNAPSHOT, bindings.clone(),
                ).await.map_err(E::from)?.ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
                let after_grants = transaction.query_values_at::<NavigationGrantSnapshot, _>(
                    NAVIGATION_GRANT_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let after_sources = transaction.query_values_at::<SurrealModelLaneNavigationRow, _>(
                    NAVIGATION_SOURCE_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let after_inputs = transaction.query_values_at::<NavigationSourceInputRow, _>(
                    NAVIGATION_INPUT_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let after_receipts = transaction.query_values_at::<NavigationDependencyReceipt, _>(
                    NAVIGATION_RECEIPT_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let after_packs = transaction.query_values_at::<NavigationMemoryPackRow, _>(
                    NAVIGATION_MEMORY_PACK_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                let after_document_links = transaction.query_values_at::<NavigationMemoryPackDocumentLink, _>(
                    NAVIGATION_MEMORY_DOCUMENT_LINK_SNAPSHOT, bindings.clone(), 0,
                ).await.map_err(E::from)?;
                if before != after || before_grants != after_grants
                    || before_sources != after_sources || before_receipts != after_receipts
                    || before_inputs != after_inputs || before_packs != after_packs
                    || before_document_links != after_document_links {
                    return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                }
                for (input, before_registrations) in &consulted_memory_packs {
                    if !navigation_memory_pack_authorized(&transaction, &bindings, input, &after_document_links).await.map_err(E::from)? {
                        return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                    }
                    let after_registrations = navigation_memory_artifact_registrations(
                        &transaction, &bindings, association.as_ref().map(|value| &value.exact), &selection, input,
                    ).await.map_err(E::from)?
                        .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
                    if before_registrations.len() != after_registrations.len()
                        || before_registrations.iter().zip(&after_registrations)
                            .any(|(before, after)| !before.same_registration(after)) {
                        return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                    }
                }
                if let Some(association) = &association {
                    match lifecycle.authorize_association(association) {
                        Ok(()) => {},
                        Err(crate::swarm_orchestration::resource_scope::ScopeDenied::LifecycleAuthorityUnavailable) => return Err(E::from(SurrealStorageError::ResourceAuthorityUnavailable)),
                        Err(_) => return Err(E::from(SurrealStorageError::TransactionAuthorityRequired)),
                    }
                }
                Ok(value)
            })),
        ).await
    }

    pub(crate) async fn navigation_scalar_rows(
        &self, scope: &ModelLaneScope, kind: ModelLaneRecordKind, run_id: Option<&str>,
    ) -> Result<Vec<SurrealModelLaneNavigationRow>, SurrealStorageError> {
        validate_scope(scope)?;
        let navigation_selection = NAVIGATION_READ_SELECTOR.try_with(Clone::clone)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let bindings = NavigationQueryBindings {
            navigation_selection, record_kind: kind.as_str().to_owned(), run_id: run_id.map(str::to_owned),
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<SurrealModelLaneNavigationRow, _>(
                "SELECT record_kind, aggregate_id, run_id, event_id, event_seq, ordering_seq FROM model_lane_authority WHERE record_kind = $record_kind AND ($run_id = NONE OR run_id = $run_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC;",
                bindings,
            ).await
        })).await
    }

    // A receipt URL can only be derived from this independently visible canonical
    // ledger row, never merely from a source row's stored link.
    async fn navigation_source_input<R: serde::de::DeserializeOwned>(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<R>, SurrealStorageError> {
        validate_scope(scope)?;
        let navigation_selection = NAVIGATION_READ_SELECTOR.try_with(Clone::clone)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let bindings = NavigationReceiptBindings {
            navigation_selection, event_id: row.event_id.clone(), event_seq: row.event_seq,
            record_kind: row.record_kind.clone(), aggregate_id: row.aggregate_id.clone(), run_id: row.run_id.clone(),
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<NavigationSourceInputRow, _>(
                "SELECT navigation_source_input AS input FROM model_lane_authority WHERE navigation_source_input != NONE AND event_id = $event_id AND event_seq = $event_seq AND record_kind = $record_kind AND aggregate_id = $aggregate_id AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;", bindings,
            ).await
        })).await?;
        if rows.len() > 1 { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation source input is ambiguous" }); }
        rows.into_iter().next().map(|row| {
            let bytes = serde_json::to_vec(&row.input).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "navigation source input encoding failed" })?;
            if bytes.len() > 1024 * 1024 { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation source input exceeds bound" }); }
            serde_json::from_slice(&bytes).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "navigation source input has invalid canonical types" })
        }).transpose()
    }

    pub(crate) async fn navigation_run_input(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<NavigationRunInput>, SurrealStorageError> {
        if row.record_kind != "run" {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation run input requires run kind" });
        }
        let input: Option<NavigationRunInput> = self.navigation_source_input(scope, row).await?;
        if input.as_ref().is_some_and(|input| input.source_kind != "run" || input.kind != "run"
            || input.run_id != row.run_id || input.run_id != row.aggregate_id
            || input.source_event_id != row.event_id || input.source_event_seq != row.event_seq) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation run input differs from canonical source identity" });
        }
        Ok(input)
    }

    pub(crate) async fn navigation_message_input(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<NavigationMessageInput>, SurrealStorageError> {
        if row.record_kind != "message" { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation message input requires message kind" }); }
        self.navigation_source_input(scope, row).await
    }

    pub(crate) async fn navigation_context_inputs(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<NavigationContextInputs>, SurrealStorageError> {
        if !matches!(row.record_kind.as_str(), "context_handoff" | "context_artifact") {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation context source kind differs" });
        }
        let input: Option<NavigationContextInputs> = self.navigation_source_input(scope, row).await?;
        if let Some(context) = input.as_ref() {
            if context.source_kind != "context_sources" || context.run_id != row.run_id || context.kind != row.record_kind {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation context input differs from canonical source identity" });
            }
            if row.record_kind == "context_handoff" && context.artifact.is_some() {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation handoff cannot supply an artifact row witness" });
            }
            if context.artifact.as_ref().is_some_and(|artifact| artifact.source_kind != "context_artifact"
                || artifact.run_id != row.run_id || artifact.artifact_binding_id != row.aggregate_id
                || artifact.source_event_id != row.event_id || artifact.source_event_seq != row.event_seq) {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation artifact input differs from canonical source identity" });
            }
        }
        Ok(input)
    }

    pub(crate) async fn navigation_lane_input(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<NavigationLaneInput>, SurrealStorageError> {
        if row.record_kind != "lane" { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation lane input requires lane kind" }); }
        let input: Option<Value> = self.navigation_source_input(scope, row).await?;
        input.map(|mut input| {
            if let Some(fields) = input.as_object_mut() {
                fields.remove("failstate_code");
            }
            serde_json::from_value(input).map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
                reason: "navigation lane input has invalid canonical types",
            })
        }).transpose()
    }

    pub(crate) async fn navigation_mt_runtime_input(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<NavigationMtRuntimeInput>, SurrealStorageError> {
        if row.record_kind != "mt_runtime_status" {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation runtime input requires runtime status kind" });
        }
        let input: Option<NavigationMtRuntimeInput> = self.navigation_source_input(scope, row).await?;
        if input.as_ref().is_some_and(|input| input.source_kind != "mt_runtime_status" || input.kind != "mt_runtime_status"
            || input.run_id != row.run_id || input.mt_status_id != row.aggregate_id
            || input.source_event_id != row.event_id || input.source_event_seq != row.event_seq) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation runtime input differs from canonical source identity" });
        }
        Ok(input)
    }

    pub(crate) async fn navigation_error_input(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<NavigationErrorInput>, SurrealStorageError> {
        if !matches!(row.record_kind.as_str(), "lane" | "recovery_event") {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "navigation error input requires lane or recovery event kind",
            });
        }
        self.navigation_source_input(scope, row).await
    }

    pub(crate) async fn navigation_visible_receipt(
        &self, scope: &ModelLaneScope, row: &SurrealModelLaneNavigationRow,
    ) -> Result<Option<SurrealModelLaneNavigationReceipt>, SurrealStorageError> {
        validate_scope(scope)?;
        let navigation_selection = NAVIGATION_READ_SELECTOR.try_with(Clone::clone)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let bindings = NavigationReceiptBindings {
            navigation_selection, event_id: row.event_id.clone(), event_seq: row.event_seq,
            record_kind: row.record_kind.clone(), aggregate_id: row.aggregate_id.clone(), run_id: row.run_id.clone(),
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let mut receipts = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<SurrealModelLaneNavigationReceipt, _>(
                "SELECT event_id, event_sequence AS event_seq FROM kernel_event_ledger WHERE id = type::record('kernel_event_ledger', $event_id) AND event_id = $event_id AND event_sequence = $event_seq AND source_component = 'model_lane' AND aggregate_type = $record_kind AND aggregate_id = $aggregate_id AND kernel_task_run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;",
                bindings,
            ).await
        })).await?;
        if receipts.len() > 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "navigation receipt is not uniquely visible" });
        }
        Ok(receipts.pop())
    }

    pub(crate) async fn with_cloud_authority_transaction<T, E, F, Fut>(
        &self, purpose: ContextCoordinationPurpose, scope: ModelLaneScope, operation: F,
    ) -> Result<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        if !matches!(purpose, ContextCoordinationPurpose::CloudProjectionPlan
            | ContextCoordinationPurpose::CloudConsentReceipt | ContextCoordinationPurpose::CloudRun
            | ContextCoordinationPurpose::CloudLane | ContextCoordinationPurpose::CloudConsentDenial) {
            return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
        }
        let domains = vec![("workspace", scope.workspace_id.clone(), None)];
        self.supervise_context_operation(purpose, scope, domains, operation).await
    }

    pub(crate) async fn with_crdt_covering_transaction<T, E, F, Fut>(
        &self,
        purpose: ContextCoordinationPurpose,
        scope: ModelLaneScope,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        if !matches!(purpose, ContextCoordinationPurpose::GuardedMessage
            | ContextCoordinationPurpose::DocumentClaim | ContextCoordinationPurpose::WorkspaceClaim
            | ContextCoordinationPurpose::SourceRootClaim | ContextCoordinationPurpose::IndexRunClaim
            | ContextCoordinationPurpose::LeaseRenew | ContextCoordinationPurpose::LeaseRelease
            | ContextCoordinationPurpose::LeaseExpiry | ContextCoordinationPurpose::ProposalRecord
            | ContextCoordinationPurpose::ProposalDecide | ContextCoordinationPurpose::ProposalApply) {
            return Err(E::from(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT covering coordination requires its original operation purpose",
            }));
        }
        let domains = vec![("workspace", scope.workspace_id.clone(), None)];
        self.supervise_context_operation(purpose, scope, domains, operation).await
    }

    #[cfg(feature = "test-utils")]
    pub(crate) async fn hold_context_coordination_for_test(
        &self, domain_kind: &str, domain_id: &str, run_id: &str, scope: &ModelLaneScope,
    ) -> Result<ContextCoordinationTestHolder, SurrealStorageError> {
        let guard = super::with_model_lane_operation(ContextCoordinationPurpose::Consume,
            self.acquire_context_coordination(ContextCoordinationPurpose::Consume,
                domain_kind, domain_id, Some(run_id), scope)).await?;
        Ok(ContextCoordinationTestHolder { storage: self.storage.clone(), guard: Some(guard) })
    }

    pub(crate) async fn acquire_context_coordination(
        &self,
        purpose: ContextCoordinationPurpose,
        domain_kind: &str,
        domain_id: &str,
        run_id: Option<&str>,
        scope: &ModelLaneScope,
    ) -> Result<ContextCoordinationGuard, SurrealStorageError> {
        validate_scope(scope)?;
        if !matches!(domain_kind, "run" | "lane" | "workspace") || domain_id.trim().is_empty() {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "invalid ContextBundle coordination domain" });
        }
        let authority = super::current_record_user_scope()
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let subject = super::MODEL_LANE_OPERATION_SUBJECT.try_with(Clone::clone).ok()
            .or_else(|| matches!(purpose, ContextCoordinationPurpose::Consume | ContextCoordinationPurpose::ArtifactBinding
                | ContextCoordinationPurpose::Handoff | ContextCoordinationPurpose::GuardedMessage
                | ContextCoordinationPurpose::RunCreate | ContextCoordinationPurpose::LaneCreate | ContextCoordinationPurpose::ProfileSessionWrite | ContextCoordinationPurpose::ProfileAdmissionCreate | ContextCoordinationPurpose::ProfileAdmissionTransition | ContextCoordinationPurpose::ProfileAdmissionFinalization
                | ContextCoordinationPurpose::CloudProjectionPlan | ContextCoordinationPurpose::CloudConsentReceipt
                | ContextCoordinationPurpose::CloudRun | ContextCoordinationPurpose::CloudLane | ContextCoordinationPurpose::CloudConsentDenial
                | ContextCoordinationPurpose::WorkspaceClaim | ContextCoordinationPurpose::IndexRunClaim)
                .then(|| super::ModelLaneOperationSubject {
                    kind: super::resource_authority::ResourceKind::Workspace,
                    external_id: scope.workspace_id.clone(),
                }))
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if (purpose == ContextCoordinationPurpose::RecoveryExecute
                && (authority.capability_id != "model_lane.recovery.execute"
                    || subject.external_id != scope.workspace_id))
            || !purpose.accepts_original(subject.kind, authority.action)
            || subject.external_id.trim().is_empty()
            || authority.workspace_id.as_deref() != Some(scope.workspace_id.as_str())
            || authority.session_id != scope.authenticated_session_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let grant_id = authority.grant_id.clone()
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let decision = self.storage.authorize_protected_resource(super::resource_authority::AuthorizationRequest {
            session_token: authority.session_token.clone(), channel_binding_hash: authority.channel_binding_hash.clone(),
            capability_id: authority.capability_id.clone(), resource_kind: subject.kind,
            external_resource_id: subject.external_id.clone(), action: authority.action,
        }).await.map_err(SurrealStorageError::from)?;
        if decision.resource_id != authority.resource_id || decision.grant_id != grant_id
            || decision.account_id != scope.owner_account_id || decision.principal_id != scope.actor_principal_id
            || decision.session_id != scope.authenticated_session_id || decision.access_space_id != scope.access_space_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let resource_bindings = ContextResourceBindings {
            resource_id: authority.resource_id.clone(), subject_kind: subject.kind.as_str().to_owned(),
            subject_id: subject.external_id.clone(), workspace_id: scope.workspace_id.clone(),
        };
        let workspaces = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_first::<String, _>(
                "SELECT VALUE record::id(IF resource_kind = 'workspace' { id } ELSE { parent_resource_id }) FROM type::record('protected_resources', $resource_id) WHERE resource_kind = $subject_kind AND external_resource_id = $subject_id AND lifecycle_state = 'active' AND owner_account_id = $auth.account_id AND access_space_id = $auth.access_space_id AND created_by_principal_id.status = 'enabled' AND ((resource_kind = 'workspace' AND external_resource_id = $workspace_id) OR (resource_kind = 'rich_document' AND parent_resource_id.resource_kind = 'workspace' AND parent_resource_id.external_resource_id = $workspace_id AND parent_resource_id.lifecycle_state = 'active' AND parent_resource_id.owner_account_id = $auth.account_id AND parent_resource_id.access_space_id = $auth.access_space_id AND parent_resource_id.created_by_principal_id.status = 'enabled')) LIMIT 1;",
                resource_bindings,
            ).await
        })).await?;
        let canonical_workspace_id = workspaces.ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let token = coordination_token()?;
        let token_hash = sha256_hex(token.as_bytes());
        let operation_id = uuid::Uuid::now_v7();
        let started = std::time::Instant::now();
        loop {
            let bindings = ContextCoordinationBindings {
                domain_kind: domain_kind.to_owned(), domain_id: domain_id.to_owned(),
                run_id: run_id.map(str::to_owned),
                canonical_run_id: run_id.map(|id| stable_record_id("run", id, scope)),
                canonical_lane_id: (domain_kind == "lane").then(|| stable_record_id("lane", domain_id, scope)),
                owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
                authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
                workspace_id: scope.workspace_id.clone(), canonical_workspace_id: canonical_workspace_id.clone(),
                resource_id: authority.resource_id.clone(), subject_kind: subject.kind.as_str().to_owned(),
                subject_id: subject.external_id.clone(),
                grant_id: grant_id.clone(), capability_id: authority.capability_id.clone(),
                operation_id, token_hash: token_hash.clone(), coordination_action: authority.action.as_str().to_owned(),
                coordination_purpose: purpose.as_str().to_owned(),
            };
            // Each attempt is a committed database acquisition. Unknown settlement
            // is propagated, never treated as a busy result or retried.
            let pending = PendingContextAcquisition { token: token.clone(), operation_id,
                authority: authority.clone(), purpose, subject: subject.clone(), bindings: bindings.clone() };
            #[cfg(feature = "test-utils")]
            let acquisition_control = self.completion_test_control.clone();
            let outcome = self.storage.with_owned_authenticated_transaction(
                operation_id, super::ContextOperationCancellation::new(), move |transaction| {
                Box::pin(async move {
                    let rows = transaction.query_values_at::<ContextCoordinationRow, _>(
                        ACQUIRE_CONTEXT_COORDINATION, bindings, 4).await?;
                    #[cfg(feature = "test-utils")]
                    if rows.len() == 1 { acquisition_control.before_acquisition_commit(operation_id).await; }
                    Ok::<_, SurrealStorageError>(rows)
                })
            }).await;
            let rows = match outcome {
                Ok(rows) => rows,
                Err(error) if self.storage.transaction_settlement_pending(operation_id) => {
                    self.storage.retain_context_acquisition(operation_id, pending);
                    return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
                }
                Err(error) => return Err(error),
            };
            if rows.len() == 1 {
                return Ok(ContextCoordinationGuard { row: rows.into_iter().next().unwrap(), token, operation_id, authority, purpose, subject,
                    routing_sources: super::resource_authority::current_routing_original_sources() });
            }
            if !rows.is_empty() || started.elapsed() >= self.storage.config().statement_timeout() {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "ContextBundle coordination acquisition unavailable" });
            }
            tracing::debug!(%operation_id, domain_kind, domain_id, "ContextBundle durable coordination busy");
            #[cfg(feature = "test-utils")]
            let _ = CONTEXT_COORDINATION_BUSY_OBSERVER.try_with(|observer| observer.send(ContextCoordinationBusy {
                domain_kind: domain_kind.to_owned(), domain_id: domain_id.to_owned(), operation_id,
            }));
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
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
        self.put_immutable_with_query(&ModelLaneQuery::Storage(&self.storage), kind, aggregate_id, run_id, idempotency_key, record_json, search_terms, event_payload_json, scope).await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn put_immutable_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        run_id: &str,
        idempotency_key: &str,
        record_json: String,
        search_terms: Vec<String>,
        event_payload_json: String,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRecord, SurrealStorageError> {
        self.put_immutable_with_query(&ModelLaneQuery::Transaction(transaction), kind, aggregate_id, run_id, idempotency_key, record_json, search_terms, event_payload_json, scope).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn put_immutable_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
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
            event_ledger_stream_id: declared_event_stream(kind, &record_json)?,
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
        let rows = query.query_values_at::<StoredRow, _>(
                "BEGIN TRANSACTION;\
                 LET $existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
                 IF array::len($existing) > 0 { RETURN $existing; } ELSE {\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: IF $event_ledger_stream_id = NONE { $run_id } ELSE { $event_ledger_stream_id }, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $run_id, event_stream_version: 1, event_payload_json: $event_payload_json }, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                   LET $seq = $ledger[0].event_sequence;\
                   RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: $record_kind, aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $record_json, search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                 };\
                 COMMIT TRANSACTION;",
                bindings,
                2,
            ).await?;
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

    pub(crate) async fn put_context_immutable(
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
        if !matches!(kind, ModelLaneRecordKind::ContextArtifact | ModelLaneRecordKind::ContextHandoff) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "context finalization requires artifact or handoff" });
        }
        let expected_purpose = match kind {
            ModelLaneRecordKind::ContextArtifact => ContextCoordinationPurpose::ArtifactBinding,
            ModelLaneRecordKind::ContextHandoff => ContextCoordinationPurpose::Handoff,
            _ => unreachable!("context record kind checked above"),
        };
        let sealed = super::PROTECTED_TRANSACTION.try_with(|transaction| {
            std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)
        }).unwrap_or(false);
        if !sealed || !super::MODEL_LANE_OPERATION_PURPOSE.try_with(|purpose| purpose.as_str() == expected_purpose.as_str()).unwrap_or(false) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "context finalization requires its original authenticated fenced transaction",
            });
        }
        validate_scope(scope)?;
        validate_identity(aggregate_id, idempotency_key)?;
        let expected: Value = serde_json::from_str(&record_json).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "context prepared record JSON is malformed" })?;
        let bindings = WriteBindings {
            event_ledger_stream_id: declared_event_stream(kind, &record_json)?,
            record_id: stable_record_id(kind.as_str(), aggregate_id, scope),
            event_id: String::new(), record_kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(), run_id: run_id.to_owned(), idempotency_key: idempotency_key.to_owned(),
            record_json, search_terms, event_payload_json, event_payload_hash: String::new(),
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredRow, _>(
                "LET $existing = SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;\
                 IF array::len($existing) = 1 { RETURN $existing; } ELSE {\
                   IF array::len($existing) != 0 { THROW 'context authority identity is ambiguous'; };\
                   LET $event_id = string::concat('evt-model-lane-', type::string(rand::uuid::v7()));\
                   LET $seq = sequence::nextval('kernel_event_sequence');\
                   LET $final_record = object::extend(encoding::json::decode($record_json), { event_ledger_event_id: $event_id, event_ledger_seq: $seq, event_stream_version: 1, transaction_seq: $seq });\
                   LET $final_payload = encoding::json::encode(object::extend(encoding::json::decode($event_payload_json), { record: $final_record }));\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_sequence: $seq, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: $event_ledger_stream_id, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: crypto::sha256($final_payload), source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $run_id, event_stream_version: 1, event_payload_json: $final_payload }, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                   IF array::len($ledger) != 1 OR $ledger[0].event_id != $event_id OR $ledger[0].event_sequence != $seq { THROW 'context final event allocation is not canonical'; };\
                   RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: $record_kind, aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: encoding::json::encode($final_record), search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                 };", bindings, 1,
            ).await
        })).await?;
        if rows.len() != 1 { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "context finalization returned no unique canonical record" }); }
        let row: SurrealModelLaneRecord = rows.into_iter().next().expect("one canonical context row").into();
        let mut actual: Value = serde_json::from_str(&row.record_json).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "context finalized record JSON is malformed" })?;
        for field in ["event_ledger_event_id", "event_ledger_seq", "event_stream_version", "transaction_seq"] {
            let value = expected.get(field).ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "context prepared metadata field is absent" })?.clone();
            actual.as_object_mut().ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "context finalized record is not an object" })?.insert(field.to_owned(), value);
        }
        if actual != expected || row.aggregate_id != aggregate_id || row.run_id != run_id || row.idempotency_key != idempotency_key {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "context immutable identity or idempotency conflict" });
        }
        self.validate_event_link(kind, &row, scope).await?;
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
        let caller_record: Value = serde_json::from_str(&write.record_json)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "checkpoint input JSON is malformed" })?;
        if caller_record.get("cloud_consent_denial_membership").is_some() {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "checkpoint membership is server-derived only" });
        }
        let expected = write.clone();
        let event_payload_hash = sha256_hex(write.event_payload_json.as_bytes());
        let lane_id = lane_id.unwrap_or_default();
        let bindings = RecoveryCheckpointWriteBindings {
            event_ledger_stream_id: checked_write_stream(&write)?,
            record_id: stable_record_id(write.kind.as_str(), &write.aggregate_id, scope),
            run_record_id: stable_record_id(
                ModelLaneRecordKind::Run.as_str(),
                &write.run_id,
                scope,
            ),
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
        if !checkpoint_record_matches_write(&row, &expected, scope) {
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
        let bindings = run_event_bindings(run_id, None, 0, scope);
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
            event_ledger_stream_id: checked_write_stream(&write)?,
            record_id: stable_record_id(write.kind.as_str(), &write.aggregate_id, scope),
            run_record_id: stable_record_id(
                ModelLaneRecordKind::Run.as_str(),
                &write.run_id,
                scope,
            ),
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
        checked_write_stream(&first)?;
        checked_write_stream(&second)?;
        let expected_first = first.clone();
        let expected_second = second.clone();
        let first_event_payload_hash = sha256_hex(first.event_payload_json.as_bytes());
        let second_event_payload_hash = sha256_hex(second.event_payload_json.as_bytes());
        let bindings = PairWriteBindings {
            first_record_id: stable_record_id(first.kind.as_str(), &first.aggregate_id, scope),
            first_event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
            first_record_kind: first.kind.as_str().to_owned(),
            first_aggregate_id: first.aggregate_id,
            first_event_ledger_stream_id: first.event_ledger_stream_id,
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
            second_event_ledger_stream_id: second.event_ledger_stream_id,
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
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRow, _>(PAIR_IMMUTABLE_QUERY, bindings, 3)
                        .await
                })
            })
            .await?;
        validate_pair_result(rows, &expected_first, &expected_second)
    }

    pub(crate) async fn put_message_immutable_guarded(
        &self,
        message: SurrealModelLaneWrite,
        payload_binding: Option<SurrealModelLaneWrite>,
        guard: SurrealModelLaneMessageGuard,
        scope: &ModelLaneScope,
    ) -> Result<(SurrealModelLaneRecord, Option<SurrealModelLaneRecord>), SurrealStorageError> {
        let coordinated = guard.crdt.is_some();
        if coordinated && !super::PROTECTED_TRANSACTION.try_with(|transaction| {
            std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)
        }).unwrap_or(false) {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "CRDT admission requires the original authenticated covering transaction",
            });
        }
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

        checked_write_stream(&message)?;
        if let Some(binding) = payload_binding.as_ref() { checked_write_stream(binding)?; }
        let expected_message = message.clone();
        let expected_binding = payload_binding.clone();
        let message_event_payload_hash = sha256_hex(message.event_payload_json.as_bytes());
        let binding = payload_binding.unwrap_or_else(|| SurrealModelLaneWrite {
            kind: ModelLaneRecordKind::ContextArtifact,
            event_ledger_stream_id: None,
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
            message_event_ledger_stream_id: message.event_ledger_stream_id,
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
            binding_event_ledger_stream_id: binding.event_ledger_stream_id,
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
                .map(|id| {
                    stable_record_id(ModelLaneRecordKind::PromotionDecision.as_str(), id, scope)
                })
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
        let (statement, result_index) = if coordinated {
            (GUARDED_MESSAGE_QUERY.strip_prefix("BEGIN TRANSACTION;")
                .and_then(|query| query.strip_suffix("COMMIT TRANSACTION;"))
                .expect("guarded message transaction delimiters are fixed"), 1)
        } else {
            (GUARDED_MESSAGE_QUERY, 2)
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRow, _>(statement, bindings, result_index)
                        .await
                })
            })
            .await?;
        if let Some(expected_binding) = expected_binding.as_ref() {
            let (message, binding) =
                validate_pair_result(rows, &expected_message, expected_binding)?;
            Ok((message, Some(binding)))
        } else {
            let mut rows = rows.into_iter().map(SurrealModelLaneRecord::from);
            let row = rows
                .next()
                .ok_or(SurrealStorageError::InvalidModelLaneRecord {
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn replace_run_for_attachment(
        &self, run_id: &str, lane_id: &str, expected_record_json: String,
        expected_event_stream_version: i64, next_record_json: String, input_run_record_json: String, lane_record_json: String,
        search_terms: Vec<String>, event_payload_json: String, scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        let sources = super::resource_authority::current_routing_original_sources()
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if sources.run_id() != run_id || !self.run_attachment_is_guarded(&sources)
            || expected_event_stream_version < 1 {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        sources.revalidate().await.map_err(|error| match error {
            super::resource_authority::ResourceAuthorityError::Storage(error) => error,
            _ => SurrealStorageError::TransactionAuthorityRequired,
        })?;
        let bindings = RunAttachmentBindings {
            run_attachment: RunAttachmentSelection {
                run_id: run_id.to_owned(), lane_id: lane_id.to_owned(), expected_record_json,
                next_record_json, input_run_record_json, lane_record_json, expected_event_stream_version,
            },
            record_id: stable_record_id(ModelLaneRecordKind::Run.as_str(), run_id, scope),
            event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()), search_terms,
            event_payload_hash: sha256_hex(event_payload_json.as_bytes()), event_payload_json,
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredRow, _>(r#"
BEGIN TRANSACTION;
LET $target = type::record('model_lane_authority', $record_id);
LET $current = (SELECT * FROM ONLY $target WHERE record_kind = 'run' AND aggregate_id = $run_attachment.run_id AND run_id = $run_attachment.run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
IF $current = NONE OR $current.record_json != $run_attachment.expected_record_json OR $current.event_stream_version != $run_attachment.expected_event_stream_version { RETURN []; } ELSE {
    LET $update = $wp012_routing_original.source_update;
    IF $update = NONE { THROW 'run attachment actual Update authority is absent'; };
    LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
        event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $current.run_id,
        session_run_id: encoding::json::decode($current.record_json).event_ledger_stream_id,
        aggregate_type: 'run', aggregate_id: $current.aggregate_id, idempotency_key: $event_id,
        event_type: 'MODEL_LANE_AUTHORITY_REPLACED', actor_kind: 'principal', actor_id: $actor_principal_id,
        causation_id: $current.event_id, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane',
        payload: { record_kind: 'run', run_id: $current.run_id, event_stream_version: $current.event_stream_version + 1, event_payload_json: $event_payload_json },
        authority_resource_id: $update.resource_id, authority_grant_id: $update.grant_id,
        authority_session_id: $auth.id, authority_capability_id: 'fs.write', authority_action: 'update',
        wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id
    };
    IF array::len($ledger) != 1 { THROW 'run attachment canonical receipt missing'; };
    RETURN UPDATE $target SET record_json = $run_attachment.next_record_json, search_terms = $search_terms,
        event_id = $event_id, event_ledger_event_id = type::record('kernel_event_ledger', $event_id),
        event_seq = $ledger[0].event_sequence, event_stream_version += 1, transaction_seq = $ledger[0].event_sequence
        WHERE record_json = $run_attachment.expected_record_json AND event_stream_version = $run_attachment.expected_event_stream_version;
};
COMMIT TRANSACTION;
"#, bindings, 3).await
        })).await?;
        if rows.len() > 1 { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "run attachment CAS is ambiguous" }); }
        Ok(rows.into_iter().next().map(Into::into))
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
            event_ledger_stream_id: declared_event_stream(kind, &record_json)?,
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
                   IF $record_kind != 'session_cleanup_receipt' AND (IF $record_kind = 'message' { encoding::json::decode($current.record_json).inner.event_ledger_stream_id } ELSE { encoding::json::decode($current.record_json).event_ledger_stream_id }) != $event_ledger_stream_id { THROW 'model-lane immutable declared stream changed'; };\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $current.run_id, session_run_id: IF $event_ledger_stream_id = NONE { $current.run_id } ELSE { $event_ledger_stream_id }, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_REPLACED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $current.event_id, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $current.run_id, event_stream_version: $current.event_stream_version + 1, event_payload_json: $event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
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
            event_ledger_stream_id: declared_event_stream(kind, &record_json)?,
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
                   IF $record_kind != 'session_cleanup_receipt' AND (IF $record_kind = 'message' { encoding::json::decode($current.record_json).inner.event_ledger_stream_id } ELSE { encoding::json::decode($current.record_json).event_ledger_stream_id }) != $event_ledger_stream_id { THROW 'model-lane immutable declared stream changed'; };\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $current.run_id, session_run_id: IF $event_ledger_stream_id = NONE { $current.run_id } ELSE { $event_ledger_stream_id }, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_REPLACED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $current.event_id, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $current.run_id, event_stream_version: $current.event_stream_version + 1, event_payload_json: $event_payload_json }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
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

    // The caller owns current authorization and the transaction; issuer attribution stays intact.


    pub(crate) async fn put_operator_approval(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        issuer: &super::resource_authority::OwnerIssuanceSeal<'_>,
    ) -> Result<PendingOwnerRoleEvidence, SurrealStorageError> {
        use handshake_role_authority::canonical;
        let request = match issuer.command() {
            crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueArtifactApproval(request)
                if issuer.matches_transaction(transaction) && issuer.current_scope() == &request.approval.scope => request,
            _ => return Err(SurrealStorageError::TransactionAuthorityRequired),
        };
        let prepared = canonical::prepare_operator(&request.approval)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator approval command integrity is invalid" })?;
        self.put_prepared_operator_evidence_in_transaction(
            transaction, issuer.operation_id(), issuer.current_scope(), &prepared, &RoleEvidenceCreationAuthority::Owner(issuer),
        ).await
    }

    async fn put_prepared_operator_evidence_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        operation_id: uuid::Uuid,
        current_scope: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        prepared: &handshake_role_authority::domain::PurePreparedRoleIntent,
        creation: &RoleEvidenceCreationAuthority<'_, '_>,
    ) -> Result<PendingOwnerRoleEvidence, SurrealStorageError> {
        use handshake_role_authority::{canonical, domain};
        let body = prepared.body();
        if &body.scope != current_scope {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let scope = ModelLaneScope {
            owner_account_id: body.scope.owner_account_id.to_string(),
            actor_principal_id: body.scope.actor_principal_id.to_string(),
            authenticated_session_id: body.scope.authenticated_session_id.to_string(),
            access_space_id: body.scope.access_space_id.to_string(),
            workspace_id: body.scope.workspace_id.as_str().to_owned(),
        };
        let mut existing = self.find_by_term_in_transaction(
            transaction, ModelLaneRecordKind::RoleEvidence,
            &format!("idempotency_key={}", body.action.idempotency_key), &scope,
        ).await?;
        if existing.len() > 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "Operator idempotency key resolves to ambiguous authority" });
        }
        if existing.is_empty() && self.get_in_transaction(
            transaction, ModelLaneRecordKind::RoleEvidence, &body.action.action_id, &scope,
        ).await?.is_some() {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "Operator immutable action changed its idempotency key" });
        }
        let event = if let Some(existing) = existing.pop() {
            let receipt = serde_json::from_str::<domain::RoleReceipt>(&existing.record_json)
                .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "existing Operator receipt is malformed" })?;
            handshake_role_authority::resolution::compare_retry(&receipt, prepared)
                .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator full-command immutable retry conflicts" })?;
            receipt.event
        } else {
            let sequences = transaction.query_values_at::<i64, _>(
                "RETURN sequence::nextval('kernel_event_sequence');", (), 0,
            ).await?;
            let sequence = match sequences.as_slice() {
                [sequence] if *sequence > 0 => *sequence,
                _ => return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role event sequence allocation is ambiguous" }),
            };
            domain::CanonicalEventKey {
                event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
                stream_id: format!("role-authority:{}", body.run.run_id),
                event_seq: sequence as u64, transaction_seq: sequence as u64,
            }
        };
        let receipt = domain::RoleReceipt {
            body: body.clone(),
            content_hash: canonical::body_content_hash(body)
                .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator approval body cannot be hashed" })?,
            event,
        };
        let selector = crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector {
            evidence_ref: domain::RoleEvidenceRef::parse(&format!("eventledger://role-authority/{}", receipt.event.event_id))
                .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator approval event reference is malformed" })?,
            expectation: domain::ResolutionExpectation {
                receipt_scope: body.scope.clone(), role: body.role.clone(), run: body.run.clone(),
                subject: body.subject.clone(), purpose: body.purpose.clone(), expected: body.expected.clone(),
            },
            designation_ref: None, producer: None,
        };
        let record_json = String::from_utf8(canonical::canonical_receipt_bytes(&receipt)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator approval receipt cannot be canonicalized" })?)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator approval receipt is not UTF-8" })?;
        let bindings = RoleEvidenceInsertBindings {
            record_id: stable_record_id(ModelLaneRecordKind::RoleEvidence.as_str(), &body.action.idempotency_key, &scope),
            event_id: receipt.event.event_id.clone(),
            event_sequence: i64::try_from(receipt.event.event_seq)
                .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator event sequence exceeds canonical storage range" })?,
            operation_id: operation_id.to_string(), aggregate_id: body.action.action_id.clone(),
            run_id: body.run.run_id.clone(), stream_id: receipt.event.stream_id.clone(),
            idempotency_key: body.action.idempotency_key.clone(),
            payload_hash: sha256_hex(record_json.as_bytes()), record_json,
            causation_id: match &body.purpose {
                domain::Purpose::PriorSelection => body.action.request_id.clone(),
                domain::Purpose::StageAcknowledgement(stage) => stage.authority_request_message_ref.clone(),
            }, correlation_id: body.run.trace_id.clone(),
            owner_account_id: scope.owner_account_id, actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id, access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        let creation_record_id = bindings.record_id.clone();
        let creation_event_id = bindings.event_id.clone();
        let creation_hash = bindings.payload_hash.clone();
        let rows = creation.with_creation(&creation_record_id, &creation_event_id, &creation_hash, transaction.query_values_at::<StoredRow, _>(
            "LET $existing = SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id; IF array::len($existing) = 1 { RETURN $existing; } ELSE { IF array::len($existing) != 0 { THROW 'role authority identity is ambiguous'; }; LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_sequence: $event_sequence, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: $stream_id, aggregate_type: 'role_evidence', aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: $causation_id, correlation_id: $correlation_id, payload_hash: $payload_hash, source_component: 'model_lane', payload: { record_kind: 'role_evidence', run_id: $run_id, event_stream_version: 1, event_payload_json: $record_json, issuance_operation_id: $operation_id }, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }; IF array::len($ledger) != 1 OR $ledger[0].event_sequence != $event_sequence { THROW 'role event allocation differs'; }; RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: 'role_evidence', aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $record_json, search_terms: [string::concat('idempotency_key=', $idempotency_key)], event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $event_sequence, event_stream_version: 1, transaction_seq: $event_sequence, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }; };",
            bindings, 1,
        )).await?;
        if rows.len() != 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "Operator issuance returned missing or ambiguous authority" });
        }
        let query = ModelLaneQuery::Transaction(transaction);
        let (verified, issuance_operation_id) = creation.with_creation(&creation_record_id, &creation_event_id, &creation_hash,
            self.read_role_evidence_with_query(&query, &selector)).await?;
        handshake_role_authority::resolution::compare_retry(verified.receipt(), prepared)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Operator full-command immutable retry conflicts" })?;
        if issuance_operation_id != operation_id {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "Operator retry changed the original issuance operation" });
        }
        Ok(PendingOwnerRoleEvidence { readback: RoleEvidenceReadback {
            operation_id: operation_id, current_consumer: current_scope.clone(),
            selector, receipt: verified.receipt().clone(), issuance_operation_id,
        }, verified })
    }

    pub(crate) async fn put_operator_stage_ack<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'tx>,
        prior: &TxCheckedRoleEvidence<'tx>,
        acknowledgement: &handshake_role_authority::domain::AcknowledgeOperatorStage,
    ) -> Result<TxCheckedRoleEvidence<'tx>, SurrealStorageError> {
        use handshake_role_authority::{canonical, domain};
        let intent = crate::swarm_orchestration::model_lane::RoutingStageAcknowledgementIntent::Operator(
            acknowledgement.clone(),
        );
        let prior_selector = match consumer.command() {
            crate::swarm_orchestration::model_lane::RoleConsumptionCommand::AcknowledgeRoutingStage {
                selector, ..
            } => selector,
            _ => return Err(SurrealStorageError::TransactionAuthorityRequired),
        };
        if !consumer.matches_transaction(transaction)
            || !stage_ack_command_matches(consumer.command(), consumer.operation_id(), prior_selector, &intent)
            || consumer.current_scope() != &acknowledgement.scope
            || prior.consumer_scope() != consumer.current_scope()
            || !stage_ack_command_matches(&prior.command, consumer.operation_id(), prior_selector, &intent) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        prior.require_matches(transaction, consumer.operation_id(), prior_selector)?;
        let previous = &prior.receipt().body;
        if previous.role != domain::Role::Operator || previous.purpose != domain::Purpose::PriorSelection
            || prior.verified.reference() != &acknowledgement.approval
            || acknowledgement.stage.prior_receipt_ref != acknowledgement.approval
            || previous.subject != acknowledgement.subject || previous.run != acknowledgement.run
            || previous.scope.owner_account_id != acknowledgement.scope.owner_account_id
            || previous.scope.workspace_id != acknowledgement.scope.workspace_id
            || previous.expected.aggregate_id != acknowledgement.expected.aggregate_id
            || previous.expected.base_ref != acknowledgement.expected.base_ref
            || previous.expected.base_vector_ref != acknowledgement.expected.base_vector_ref
            || previous.expected.schema_id != acknowledgement.expected.schema_id {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "Operator stage acknowledgement does not bind its actual prior approval",
            });
        }
        let prepared = canonical::prepare_operator_ack(acknowledgement)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
                reason: "Operator stage acknowledgement command integrity is invalid",
            })?;
        let pending = self.put_prepared_operator_evidence_in_transaction(
            transaction, consumer.operation_id(), consumer.current_scope(), &prepared, &RoleEvidenceCreationAuthority::Consumer(consumer),
        ).await?;
        let PendingOwnerRoleEvidence { readback, verified } = pending;
        let checked = TxCheckedRoleEvidence {
            transaction, operation_id: readback.operation_id, current_consumer: readback.current_consumer,
            selector: readback.selector, verified, issuance_operation_id: readback.issuance_operation_id,
            command: consumer.command().clone(),
        };
        checked.require_stage_ack(transaction, consumer.operation_id(), prior_selector, &intent)?;
        Ok(checked)
    }


    pub(crate) async fn resolve_operator_role_evidence<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'tx>,
        selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    ) -> Result<TxCheckedRoleEvidence<'tx>, SurrealStorageError> {
        if !consumer.matches_transaction(transaction)
            || !consumer.command().evidence_selectors().into_iter().any(|expected| role_selectors_match(expected, selector))
            || selector.expectation.role != handshake_role_authority::domain::Role::Operator
            || selector.designation_ref.is_some() || selector.producer.is_some()
            || selector.expectation.receipt_scope.owner_account_id != consumer.current_scope().owner_account_id
            || selector.expectation.receipt_scope.workspace_id != consumer.current_scope().workspace_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let query = ModelLaneQuery::Transaction(transaction);
        let read = self.read_role_evidence_with_query(&query, selector);
        let (verified, issuance_operation_id) = if consumer.protected_consumption_body().is_some() {
            let event = selector.evidence_ref.as_str().strip_prefix("eventledger://role-authority/")
                .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            consumer.with_protected_role_evidence_consumption(event, read).await?
        } else if consumer.is_routing_prior(selector) {
            consumer.with_routing_prior(selector, read).await?
        } else { read.await? };
        Ok(TxCheckedRoleEvidence {
            transaction, operation_id: consumer.operation_id(),
            current_consumer: consumer.current_scope().clone(), selector: selector.clone(), verified, issuance_operation_id,
            command: consumer.command().clone(),
        })
    }

    pub(crate) async fn capture_protected_role_evidence_targets(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        seal: &super::resource_authority::ProtectedRoleEvidenceTargetReadSeal<'_>,
    ) -> Result<Vec<super::resource_authority::ProtectedRoleEvidenceTarget>, SurrealStorageError> {
        if !seal.matches_transaction(transaction) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let issuance = seal.issuance();
        let operator = issuance.operator_prior();
        if seal.current_scope() != &operator.expectation.receipt_scope {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let query = ModelLaneQuery::Transaction(transaction);
        let event_id = operator.evidence_ref.as_str().strip_prefix("eventledger://role-authority/")
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        seal.with_protected_role_evidence_capture("role_evidence", event_id,
            self.read_role_evidence_with_query(&query, operator)).await?;
        let designation_event = issuance.verdict().designation_ref.as_str()
            .strip_prefix("eventledger://validator-designation/")
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let scope = role_storage_scope(seal.current_scope());
        let mut targets = Vec::new();
        for (kind, event, statement) in [
            (ModelLaneRecordKind::RoleEvidence, event_id,
             "SELECT record::id(id) AS record_id, { aggregate_id: aggregate_id, run_id: run_id, idempotency_key: idempotency_key, record_json: record_json, event_id: event_id, event_seq: event_seq, event_stream_version: event_stream_version, transaction_seq: transaction_seq } AS record FROM model_lane_authority WHERE record_kind = 'role_evidence' AND event_id = $event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;"),
            (ModelLaneRecordKind::ValidatorDesignation, designation_event,
             "SELECT record::id(id) AS record_id, { aggregate_id: aggregate_id, run_id: run_id, idempotency_key: idempotency_key, record_json: record_json, event_id: event_id, event_seq: event_seq, event_stream_version: event_stream_version, transaction_seq: transaction_seq } AS record FROM model_lane_authority WHERE record_kind = 'validator_designation' AND event_id = $event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;"),
            (ModelLaneRecordKind::ContextArtifact, issuance.verdict().execution.execution_event.event_id.as_str(),
             "SELECT record::id(id) AS record_id, { aggregate_id: aggregate_id, run_id: run_id, idempotency_key: idempotency_key, record_json: record_json, event_id: event_id, event_seq: event_seq, event_stream_version: event_stream_version, transaction_seq: transaction_seq } AS record FROM model_lane_authority WHERE record_kind = 'context_artifact' AND event_id = $event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;"),        ] {
            let read = async {
            let rows = transaction.query_values_at::<ProtectedRoleTargetRow, _>(statement, RoleEvidenceLookupBindings {
                event_id: event.to_owned(), owner_account_id: scope.owner_account_id.clone(),
                actor_principal_id: scope.actor_principal_id.clone(), authenticated_session_id: scope.authenticated_session_id.clone(),
                access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
            }, 0).await?;
            let [row] = rows.as_slice() else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
            let record: SurrealModelLaneRecord = row.record.clone().into();
            let aggregate = if kind == ModelLaneRecordKind::RoleEvidence { &record.idempotency_key } else { &record.aggregate_id };
            if row.record_id != stable_record_id(kind.as_str(), aggregate, &scope) || record.event_id != event {
                return Err(SurrealStorageError::TransactionAuthorityRequired);
            }
            self.validate_event_link_in_transaction(transaction, kind, &record, &scope).await?;
            Ok(super::resource_authority::ProtectedRoleEvidenceTarget {
                record_kind: kind.as_str().to_owned(), record_id: row.record_id.clone(),
                event_id: record.event_id.clone(), record_sha256: sha256_hex(record.record_json.as_bytes()),
                issuer_scope: seal.current_scope().clone(),
            })
            };
            let target = if matches!(kind, ModelLaneRecordKind::RoleEvidence | ModelLaneRecordKind::ValidatorDesignation) {
                seal.with_protected_role_evidence_capture(kind.as_str(), event, read).await?
            } else { read.await? };
            targets.push(target);
        }
        let verdict = issuance.verdict();
        let request_id = verdict.execution.request_message_ref.strip_prefix("model-lane-message://")
            .unwrap_or(&verdict.execution.request_message_ref);
        let request_record = self.get_in_transaction(transaction, ModelLaneRecordKind::Message, request_id, &scope).await?
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let request = crate::swarm_orchestration::model_lane::surreal_message_record(request_record.clone())
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?.inner;
        if request.run_id != verdict.run.run_id
            || request.to_lane != crate::swarm_orchestration::model_lane::ModelLaneTarget::Lane(verdict.execution.runner_lane_id.clone()) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let sender_record = self.get_in_transaction(transaction, ModelLaneRecordKind::Lane, &request.from_lane_id, &scope).await?
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let sender: crate::swarm_orchestration::model_lane::NewModelLane = serde_json::from_str(&sender_record.record_json)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        if sender.run_id != verdict.run.run_id || sender.kind != crate::swarm_orchestration::model_lane::ModelLaneKind::HumanOperator {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        for (kind, record) in [(ModelLaneRecordKind::Message, request_record), (ModelLaneRecordKind::Lane, sender_record)] {
            self.validate_event_link_in_transaction(transaction, kind, &record, &scope).await?;
            targets.push(super::resource_authority::ProtectedRoleEvidenceTarget {
                record_kind: kind.as_str().to_owned(), record_id: stable_record_id(kind.as_str(), &record.aggregate_id, &scope),
                event_id: record.event_id.clone(), record_sha256: sha256_hex(record.record_json.as_bytes()),
                issuer_scope: seal.current_scope().clone(),
            });
        }
        Ok(targets)
    }
    pub(crate) async fn put_current_promotion_consumption(
        &self, transaction: &super::SurrealTransactionContext<'_>,
        seal: &super::resource_authority::CurrentConsumerSeal<'_>,
        operator: &TxCheckedRoleEvidence<'_>, validator: &TxCheckedRoleEvidence<'_>,
    ) -> Result<SurrealModelLaneRecord, SurrealStorageError> {
        if !seal.matches_transaction(transaction) { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let body = seal.current_promotion_consumption_body(operator, validator)?;
        let scope = role_storage_scope(seal.current_scope());
        let operation_id = body.operation_id.to_string();
        let kind = ModelLaneRecordKind::RoleEvidenceConsumption;
        let record_id = stable_record_id(kind.as_str(), &operation_id, &scope);
        let event_id = format!("evt-{record_id}");
        let record_json = serde_json::to_string(&body).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let bindings = WriteBindings {
            event_ledger_stream_id: Some(body.run.run_id.clone()), record_id,
            event_id: event_id.clone(), record_kind: kind.as_str().to_owned(), aggregate_id: operation_id.clone(),
            run_id: body.run.run_id.clone(), idempotency_key: operation_id.clone(), record_json: record_json.clone(),
            search_terms: vec![format!("idempotency_key={operation_id}")], event_payload_json: record_json.clone(),
            event_payload_hash: sha256_hex(record_json.as_bytes()), owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(), authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
        };
        let record = seal.with_current_promotion_consumption_creation(&body, &event_id, operator, validator, async {
            let rows = transaction.query_values_at::<StoredRow, _>(DESIGNATION_INSERT_QUERY, bindings, 1).await?;
            let [row] = rows.as_slice() else {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "current promotion consumption is missing or ambiguous" });
            };
            let record: SurrealModelLaneRecord = row.clone().into();
            if record.record_json != record_json || record.aggregate_id != operation_id || record.idempotency_key != operation_id
                || record.event_id != event_id || record.run_id != body.run.run_id {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "current promotion consumption immutable operation conflicts" });
            }
            self.validate_event_link_in_transaction(transaction, kind, &record, &scope).await?;
            Ok(record)
        }).await?;
        seal.observe_current_promotion_consumption(body, record.event_id.clone())?;
        Ok(record)
    }

    pub(crate) async fn committed_promotion_retry_records(
        &self, operation_id: uuid::Uuid, decision_id: &str, scope: &ModelLaneScope,
    ) -> Result<(SurrealModelLaneRecord, SurrealModelLaneRecord, String), SurrealStorageError> {
        let current = self.get(ModelLaneRecordKind::RoleEvidenceConsumption, &operation_id.to_string(), scope).await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "original committed promotion consumption unavailable" })?;
        let current_payload = self.validated_event_payload(ModelLaneRecordKind::RoleEvidenceConsumption, &current, scope).await?;
        if current_payload != current.record_json || current.aggregate_id != operation_id.to_string()
            || current.idempotency_key != operation_id.to_string() || current.event_stream_version != 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "original promotion consumption is not immutable canonical state" });
        }
        let decision = self.get(ModelLaneRecordKind::PromotionDecision, decision_id, scope).await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "original committed promotion decision unavailable" })?;
        let payload = self.validated_event_payload(ModelLaneRecordKind::PromotionDecision, &decision, scope).await?;
        Ok((decision, current, payload))
    }

    pub(crate) async fn readback_current_promotion_consumption(
        &self, publication: &super::resource_authority::RolePublicationGuard<'_>,
    ) -> Result<(), SurrealStorageError> {
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let (body, event_id) = publication.current_promotion_consumption()?;
        if body.operation_id != publication.operation_id() || &body.exact_consumer_scope != publication.current_scope() {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let scope = role_storage_scope(publication.current_scope());
        let kind = ModelLaneRecordKind::RoleEvidenceConsumption;
        let current = self.get(kind, &body.operation_id.to_string(), &scope).await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "current promotion consumption disappeared" })?;
        if current.event_id != event_id || current.record_json != serde_json::to_string(body)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)? {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "current promotion consumption readback differs" });
        }
        self.validate_event_link(kind, &current, &scope).await?;
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        Ok(())
    }

    pub(crate) async fn put_protected_role_evidence_consumption(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        issuer: &super::resource_authority::ProtectedRoleEvidenceCreationSeal<'_>,
    ) -> Result<ProtectedRoleEvidenceReadback, SurrealStorageError> {
        let body = issuer.body();
        if !issuer.matches_transaction(transaction) || issuer.current_scope() != &body.issuer_scope
            || issuer.operation_id() != body.operation_id || body.operation_id.is_nil()
            || body.consumer_operation_id.is_nil() || body.targets.is_empty()
            || body.schema_id != "handshake.role-evidence-consumption@1"
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let scope = role_storage_scope(&body.issuer_scope);
        let operation_id = body.operation_id.to_string();
        let kind = ModelLaneRecordKind::RoleEvidenceConsumption;
        let record_id = stable_record_id(kind.as_str(), &operation_id, &scope);
        let event_id = format!("evt-{record_id}");
        let record_json = serde_json::to_string(body).map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "protected role consumption body cannot be encoded",
        })?;
        let bindings = WriteBindings {
            event_ledger_stream_id: Some(body.run.run_id.clone()), record_id,
            event_id: event_id.clone(), record_kind: kind.as_str().to_owned(),
            aggregate_id: operation_id.clone(), run_id: body.run.run_id.clone(),
            idempotency_key: operation_id.clone(), record_json: record_json.clone(),
            search_terms: vec![format!("idempotency_key={operation_id}")],
            event_payload_json: record_json.clone(), event_payload_hash: sha256_hex(record_json.as_bytes()),
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let record_id = bindings.record_id.clone();
        let payload_hash = bindings.event_payload_hash.clone();
        let rows = issuer.with_protected_role_evidence_creation(&record_id, &event_id, &payload_hash,
            transaction.query_values_at::<StoredRow, _>(DESIGNATION_INSERT_QUERY, bindings, 1)).await?;
        let [row] = rows.as_slice() else {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "protected role consumption publication is missing or ambiguous" });
        };
        let record: SurrealModelLaneRecord = row.clone().into();
        if record.record_json != record_json || record.aggregate_id != operation_id
            || record.idempotency_key != operation_id || record.event_id != event_id
            || record.run_id != body.run.run_id
        { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "protected role consumption immutable retry conflicts" }); }
        issuer.with_protected_role_evidence_creation(&record_id, &event_id, &payload_hash,
            self.validate_event_link_in_transaction(transaction, kind, &record, &scope)).await?;
        Ok(ProtectedRoleEvidenceReadback { operation_id: body.operation_id, scope, record })
    }

    pub(crate) async fn readback_protected_role_evidence_consumption(
        &self,
        publication: &super::resource_authority::RolePublicationGuard<'_>,
        expected: &ProtectedRoleEvidenceReadback,
    ) -> Result<ProtectedRoleEvidenceReadback, SurrealStorageError> {
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        if publication.operation_id() != expected.operation_id || role_storage_scope(publication.current_scope()) != expected.scope {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let scope = role_storage_scope(publication.current_scope());
        let kind = ModelLaneRecordKind::RoleEvidenceConsumption;
        let record_id = stable_record_id(kind.as_str(), &expected.record.aggregate_id, &scope);
        let hash = sha256_hex(expected.record.record_json.as_bytes());
        publication.with_protected_role_authority_read(kind.as_str(), &record_id, &expected.record.event_id, &hash, async {
        let actual = self.get(kind, &expected.record.aggregate_id, &scope).await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "protected role consumption disappeared" })?;
        if actual != expected.record {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "protected role consumption publication differs" });
        }
        self.validate_event_link(kind, &actual, &scope).await?;
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        Ok(ProtectedRoleEvidenceReadback { operation_id: expected.operation_id, scope, record: actual })
        }).await
    }
    pub(crate) async fn put_runner_designation(
        &self, transaction: &super::SurrealTransactionContext<'_>,
        issuer: &super::resource_authority::OwnerIssuanceSeal<'_>,
    ) -> Result<RunnerDesignationReadback, SurrealStorageError> {
        self.put_runner_designation_with_event_custody(transaction, issuer, None, None).await
    }

    pub(crate) async fn put_runner_designation_with_original_event(
        &self, transaction: &super::SurrealTransactionContext<'_>,
        issuer: &super::resource_authority::OwnerIssuanceSeal<'_>, original_event_id: &str,
        custody: &super::resource_authority::ValidatorAssociationTransactionRetention,
    ) -> Result<RunnerDesignationReadback, SurrealStorageError> {
        self.put_runner_designation_with_event_custody(transaction, issuer, Some(original_event_id), Some(custody)).await
    }

    async fn put_runner_designation_with_event_custody(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        issuer: &super::resource_authority::OwnerIssuanceSeal<'_>,
        original_event_id: Option<&str>,
        custody: Option<&super::resource_authority::ValidatorAssociationTransactionRetention>,
    ) -> Result<RunnerDesignationReadback, SurrealStorageError> {
        match (original_event_id, custody) {
            (Some(event), Some(owner)) if !event.trim().is_empty()
                && owner.matches_designation_issuer(issuer, event) => {},
            (None, None) => {},
            _ => return Err(SurrealStorageError::TransactionAuthorityRequired),
        }
        let event_id = original_event_id.map(str::to_owned)
            .unwrap_or_else(|| format!("evt-model-lane-{}", uuid::Uuid::now_v7()));
        let crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueRunnerDesignation(request) = issuer.command() else {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        };
        if !issuer.matches_transaction(transaction) || issuer.current_scope() != &request.scope
            || issuer.operation_id() != request.operation_id
            || request.runner.producer_registration_ref.trim().is_empty()
            || request.runner.producer_instance_ref.trim().is_empty()
            || request.runner.producer_spawn_generation == 0 {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        // Reuse the role contract's validation for the shared action/run/artifact/base fields.
        handshake_role_authority::canonical::prepare_operator(&handshake_role_authority::domain::ApproveOperatorArtifact {
            action: request.action.clone(), scope: request.scope.clone(), run: request.run.clone(),
            subject: request.subject.clone(), expected: request.expected.clone(), retention: request.retention.clone(),
        }).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "invalid Owner designation binding" })?;
        let scope = role_storage_scope(&request.scope);
        let record_json = serde_json::to_string(&serde_json::json!({
            "request": request, "event_ledger_stream_id": request.run.run_id,
        })).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "designation cannot be encoded" })?;
        let existing = self.find_by_term_in_transaction(transaction, ModelLaneRecordKind::ValidatorDesignation,
            &format!("idempotency_key={}", request.action.idempotency_key), &scope).await?;
        if existing.len() > 1 || existing.iter().any(|record| record.aggregate_id != request.action.action_id || record.record_json != record_json) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "designation immutable retry conflicts" });
        }
        let bindings = WriteBindings {
            event_ledger_stream_id: Some(request.run.run_id.clone()),
            record_id: stable_record_id(ModelLaneRecordKind::ValidatorDesignation.as_str(), &request.action.action_id, &scope),
            event_id: event_id.clone(),
            record_kind: ModelLaneRecordKind::ValidatorDesignation.as_str().to_owned(),
            aggregate_id: request.action.action_id.clone(), run_id: request.run.run_id.clone(),
            idempotency_key: request.action.idempotency_key.clone(), record_json: record_json.clone(),
            search_terms: vec![format!("idempotency_key={}", request.action.idempotency_key)],
            event_payload_json: record_json.clone(), event_payload_hash: sha256_hex(record_json.as_bytes()),
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let creation_record_id = bindings.record_id.clone();
        let creation_event_id = bindings.event_id.clone();
        let creation_hash = bindings.event_payload_hash.clone();
        let observation = custody.map(|owner| owner.retain_designation_query::<StoredRow>()).transpose()?;
        let query_result = issuer.with_protected_role_authority_creation("validator_designation", &creation_record_id, &creation_event_id, &creation_hash,
            transaction.query_values_at::<StoredRow, _>(DESIGNATION_INSERT_QUERY, bindings, 1)).await;
        let rows = if let Some(observation) = &observation {
            observation.retain_query(query_result);
            observation.with_observation(|query, _| query.and_then(|result| result.as_ref().ok()).cloned())
                .ok_or(SurrealStorageError::TransactionSettlementPending { operation_id: issuer.operation_id() })?
        } else { query_result? };
        let [row] = rows.as_slice() else {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "designation insert is missing or ambiguous" });
        };
        let record: SurrealModelLaneRecord = row.clone().into();
        if original_event_id.is_some_and(|expected| record.event_id != expected)
            || record.record_json != record_json || record.aggregate_id != request.action.action_id
            || record.idempotency_key != request.action.idempotency_key || record.run_id != request.run.run_id {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "designation immutable identity conflicts" });
        }
        let link_result = issuer.with_protected_role_authority_creation("validator_designation", &creation_record_id, &creation_event_id, &creation_hash,
            self.validate_event_link_in_transaction(transaction, ModelLaneRecordKind::ValidatorDesignation, &record, &scope)).await;
        if let Some(observation) = &observation {
            observation.retain_link(link_result);
            if !observation.with_observation(|_, link| matches!(link, Some(Ok(())))) {
                return Err(SurrealStorageError::TransactionSettlementPending { operation_id: issuer.operation_id() });
            }
        } else { link_result?; }
        Ok(RunnerDesignationReadback { operation_id: issuer.operation_id(), scope, request: request.clone(), record })
    }

    pub(crate) async fn readback_runner_designation(
        &self,
        publication: &super::resource_authority::RolePublicationGuard<'_>,
        expected: &RunnerDesignationReadback,
    ) -> Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, SurrealStorageError> {
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        if publication.operation_id() != expected.operation_id || publication.current_scope() != &expected.request.scope {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let record_id = stable_record_id(ModelLaneRecordKind::ValidatorDesignation.as_str(), &expected.record.aggregate_id, &expected.scope);
        let hash = sha256_hex(expected.record.record_json.as_bytes());
        publication.with_protected_role_authority_read("validator_designation", &record_id, &expected.record.event_id, &hash, async {
        let record = self.get(ModelLaneRecordKind::ValidatorDesignation, &expected.record.aggregate_id, &expected.scope).await?
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "canonical designation disappeared" })?;
        if record != expected.record {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "designation publication differs from commit" });
        }
        self.validate_event_link(ModelLaneRecordKind::ValidatorDesignation, &record, &expected.scope).await?;
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        Ok(crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult::RunnerDesignation {
            request: expected.request.clone(), content_hash: handshake_role_authority::domain::ContentSha256::from_hex(&sha256_hex(record.record_json.as_bytes()))
                .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "designation hash is invalid" })?,
            event: handshake_role_authority::domain::CanonicalEventKey {
                event_id: record.event_id, stream_id: expected.request.run.run_id.clone(),
                event_seq: u64::try_from(record.event_seq).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "designation sequence is invalid" })?,
                transaction_seq: u64::try_from(record.transaction_seq).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "designation transaction sequence is invalid" })?,
            },
        })
        }).await
    }

    async fn validate_validator_designation(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
        body: &handshake_role_authority::domain::RoleEvidenceBody,
    ) -> Result<(), SurrealStorageError> {
        let designation = selector.designation_ref.as_ref().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let producer = selector.producer.as_ref().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let execution = body.validator_execution.as_ref().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if body.designation.as_ref() != Some(designation) || execution.runner_principal != producer.producer_registered_principal {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let event_id = designation.as_str().strip_prefix("eventledger://validator-designation/")
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let scope = &selector.expectation.receipt_scope;
        let rows = query.query_values::<StoredRow, _>(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = 'validator_designation' AND event_id = $event_id AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id) AND owner_account_id = $owner_account_id AND workspace_id = $workspace_id LIMIT 2;",
            ValidatorDesignationLookupBindings { event_id: event_id.to_owned(), owner_account_id: scope.owner_account_id.to_string(), workspace_id: scope.workspace_id.as_str().to_owned() },
        ).await?;
        let [row] = rows.as_slice() else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
        let value: Value = serde_json::from_str(&row.record_json).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "canonical designation is malformed" })?;
        let request: crate::swarm_orchestration::model_lane::WorkspaceValidatorRunnerDesignationRequest = serde_json::from_value(value.get("request").cloned().ok_or(SurrealStorageError::TransactionAuthorityRequired)?)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "canonical designation request is malformed" })?;
        let expected_matches = match &body.purpose {
            handshake_role_authority::domain::Purpose::PriorSelection => request.expected == body.expected,
            handshake_role_authority::domain::Purpose::StageAcknowledgement(_) =>
                request.expected.aggregate_id == body.expected.aggregate_id
                    && request.expected.base_ref == body.expected.base_ref
                    && request.expected.base_vector_ref == body.expected.base_vector_ref
                    && request.expected.schema_id == body.expected.schema_id,
        };
        if request.runner != *producer || request.run != body.run || request.subject != body.subject
            || !expected_matches || request.scope.owner_account_id != scope.owner_account_id
            || request.scope.workspace_id != scope.workspace_id || row.aggregate_id != request.action.action_id
            || row.idempotency_key != request.action.idempotency_key || row.run_id != request.run.run_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let record: SurrealModelLaneRecord = row.clone().into();
        match query {
            ModelLaneQuery::Transaction(transaction) => self.validate_event_link_in_transaction(transaction, ModelLaneRecordKind::ValidatorDesignation, &record, &role_storage_scope(&request.scope)).await?,
            ModelLaneQuery::Storage(_) => self.validate_event_link(ModelLaneRecordKind::ValidatorDesignation, &record, &role_storage_scope(&request.scope)).await?,
        }
        Ok(())
    }

    async fn read_initial_validator_parent_record(
        &self, transaction: &super::SurrealTransactionContext<'_>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'_>,
        kind: ModelLaneRecordKind, aggregate_id: &str, scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneRecord, SurrealStorageError> {
        let body = consumer.protected_consumption_body().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let record_id = stable_record_id(kind.as_str(), aggregate_id, scope);
        let target = body.targets.iter().find(|target| target.record_kind == kind.as_str()
            && target.record_id == record_id && role_storage_scope(&target.issuer_scope) == *scope)
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        consumer.with_protected_role_evidence_consumption(&target.event_id, async {
            let record = self.get_in_transaction(transaction, kind, aggregate_id, scope).await?
                .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            if record.event_id != target.event_id || sha256_hex(record.record_json.as_bytes()) != target.record_sha256 {
                return Err(SurrealStorageError::TransactionAuthorityRequired);
            }
            self.validate_event_link_in_transaction(transaction, kind, &record, scope).await?;
            Ok(record)
        }).await
    }

    async fn validate_designation_for_consumer(
        &self, transaction: &super::SurrealTransactionContext<'_>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'_>,
        selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
        body: &handshake_role_authority::domain::RoleEvidenceBody,
    ) -> Result<(), SurrealStorageError> {
        let query = ModelLaneQuery::Transaction(transaction);
        let read = self.validate_validator_designation(&query, selector, body);
        if consumer.protected_consumption_body().is_some() {
            let event = selector.designation_ref.as_ref().and_then(|reference|
                reference.as_str().strip_prefix("eventledger://validator-designation/"))
                .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            consumer.with_protected_role_evidence_consumption(event, read).await
        } else { read.await }
    }
    pub(crate) async fn put_initial_validator_verdict<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'tx>,
        operator_prior: &TxCheckedRoleEvidence<'tx>,
        issuance: &super::resource_authority::InitialValidatorIssuance,
    ) -> Result<TxCheckedRoleEvidence<'tx>, SurrealStorageError> {
        use handshake_role_authority::domain::{Role, Purpose};
        let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict {
            operator, issuance: captured, ..
        } = consumer.command() else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
        operator_prior.require_matches(transaction, consumer.operation_id(), operator)?;
        let verdict = issuance.verdict();
        let prior = &operator_prior.receipt().body;
        if !std::ptr::eq(captured.as_ref(), issuance) || !consumer.matches_transaction(transaction)
            || consumer.current_scope() != &verdict.scope || prior.role != Role::Operator
            || prior.purpose != Purpose::PriorSelection || operator.designation_ref.is_some() || operator.producer.is_some()
            || prior.run != verdict.run || prior.subject != verdict.subject || prior.expected != verdict.expected
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let scope = role_storage_scope(issuance.delegating_scope());
        let event = &verdict.execution.execution_event;
        let rows: Vec<StoredRow> = consumer.with_protected_role_evidence_consumption(&event.event_id, transaction.query_values_at(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = 'context_artifact' AND event_id = $event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;",
            RoleEvidenceLookupBindings {
                event_id: event.event_id.clone(), owner_account_id: scope.owner_account_id.clone(),
                actor_principal_id: scope.actor_principal_id.clone(), authenticated_session_id: scope.authenticated_session_id.clone(),
                access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
            }, 0,
        )).await?;
        let [output] = rows.as_slice() else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
        let output: SurrealModelLaneRecord = output.clone().into();
        consumer.with_protected_role_evidence_consumption(&event.event_id,
            self.validate_event_link_in_transaction(transaction, ModelLaneRecordKind::ContextArtifact, &output, &scope)).await?;
        let value: serde_json::Value = serde_json::from_str(&output.record_json)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Validator output artifact is malformed" })?;
        if output.run_id != verdict.run.run_id
            || value.get("artifact_kind").and_then(serde_json::Value::as_str) != Some("validator_review_output")
            || u64::try_from(output.event_seq).ok() != Some(event.event_seq)
            || u64::try_from(output.transaction_seq).ok() != Some(event.transaction_seq)
            || value.get("event_ledger_stream_id").and_then(serde_json::Value::as_str) != Some(event.stream_id.as_str())
            || value.get("artifact_ref").and_then(serde_json::Value::as_str) != Some(verdict.execution.verdict_artifact.artifact_ref.as_str())
            || value.get("artifact_sha256").and_then(serde_json::Value::as_str) != Some(verdict.execution.verdict_artifact.sha256.to_hex().as_str())
            || value.get("content_hash").and_then(serde_json::Value::as_str) != Some(verdict.execution.verdict_artifact.sha256.to_hex().as_str())
            || value.get("payload_json").and_then(serde_json::Value::as_str) != Some(issuance.output())
            || verdict.execution.verdict_artifact.version != output.event_stream_version.to_string()
            || value.get("artifact_payload_ref").and_then(serde_json::Value::as_str) != Some(verdict.execution.verdict_artifact.artifact_ref.as_str())
            || value.get("artifact_manifest_ref").and_then(serde_json::Value::as_str) != Some(format!("artifact-store://validator-review/{}/manifest", consumer.operation_id()).as_str())
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let request_id = verdict.execution.request_message_ref.strip_prefix("model-lane-message://")
            .unwrap_or(&verdict.execution.request_message_ref);
        let request = self.read_initial_validator_parent_record(transaction, consumer, ModelLaneRecordKind::Message, request_id, &scope).await?;
        let request = crate::swarm_orchestration::model_lane::surreal_message_record(request.clone())
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Validator request message is malformed" })?.inner;
        if request.run_id != verdict.run.run_id
            || request.to_lane != crate::swarm_orchestration::model_lane::ModelLaneTarget::Lane(verdict.execution.runner_lane_id.clone())
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let sender = self.read_initial_validator_parent_record(transaction, consumer, ModelLaneRecordKind::Lane, &request.from_lane_id, &scope).await?;
        let sender: crate::swarm_orchestration::model_lane::NewModelLane = serde_json::from_str(&sender.record_json)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Validator Operator sender lane is malformed" })?;
        if sender.run_id != verdict.run.run_id || sender.kind != crate::swarm_orchestration::model_lane::ModelLaneKind::HumanOperator {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let prepared = handshake_role_authority::canonical::prepare_validator_verdict_declared(verdict)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "initial Validator verdict is invalid" })?;
        let mut selector = crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector {
            evidence_ref: operator.evidence_ref.clone(),
            expectation: handshake_role_authority::domain::ResolutionExpectation {
                receipt_scope: verdict.scope.clone(), role: Role::Validator, run: verdict.run.clone(),
                subject: verdict.subject.clone(), purpose: verdict.purpose.clone(), expected: verdict.expected.clone(),
            },
            designation_ref: Some(verdict.designation_ref.clone()), producer: Some(issuance.producer_binding()),
        };
        self.validate_designation_for_consumer(transaction, consumer, &selector, prepared.body()).await?;
        let PendingOwnerRoleEvidence { mut readback, verified } = self.put_prepared_operator_evidence_in_transaction(
            transaction, consumer.operation_id(), consumer.current_scope(), &prepared, &RoleEvidenceCreationAuthority::Consumer(consumer),
        ).await?;
        selector.evidence_ref = readback.selector.evidence_ref;
        readback.selector = selector;
        Ok(TxCheckedRoleEvidence {
            transaction, operation_id: readback.operation_id, current_consumer: readback.current_consumer,
            selector: readback.selector, verified, issuance_operation_id: readback.issuance_operation_id,
            command: consumer.command().clone(),
        })
    }

    pub(crate) async fn put_validator_verdict<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'tx>,
        prior: &TxCheckedRoleEvidence<'tx>,
        verdict: &handshake_role_authority::domain::RecordValidatorVerdict,
    ) -> Result<TxCheckedRoleEvidence<'tx>, SurrealStorageError> {
        let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::RecordReviewCompletion {
            selector, verdict: captured, ..
        } = consumer.command() else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
        prior.require_matches(transaction, consumer.operation_id(), selector)?;
        if !consumer.matches_transaction(transaction) || captured != verdict || consumer.current_scope() != &verdict.scope
            || prior.receipt().body.run != verdict.run || prior.receipt().body.subject != verdict.subject
            || prior.receipt().body.expected != verdict.expected
            || selector.designation_ref.as_ref() != Some(&verdict.designation_ref) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let prepared = handshake_role_authority::canonical::prepare_validator_verdict_declared(verdict)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Validator verdict declaration is invalid" })?;
        self.put_validator_prepared(transaction, consumer, selector, &prepared).await
    }

    pub(crate) async fn put_validator_stage_ack<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'tx>,
        prior: &TxCheckedRoleEvidence<'tx>,
        acknowledgement: &handshake_role_authority::domain::AcknowledgeValidatorStage,
    ) -> Result<TxCheckedRoleEvidence<'tx>, SurrealStorageError> {
        let intent = crate::swarm_orchestration::model_lane::RoutingStageAcknowledgementIntent::Validator(acknowledgement.clone());
        let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::AcknowledgeRoutingStage { selector, .. } = consumer.command() else {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        };
        prior.require_matches(transaction, consumer.operation_id(), selector)?;
        let previous = &prior.receipt().body;
        if !consumer.matches_transaction(transaction)
            || !stage_ack_command_matches(consumer.command(), consumer.operation_id(), selector, &intent)
            || consumer.current_scope() != &acknowledgement.scope
            || previous.role != handshake_role_authority::domain::Role::Validator
            || previous.purpose != handshake_role_authority::domain::Purpose::PriorSelection
            || prior.verified.reference() != &acknowledgement.verdict
            || previous.designation.as_ref() != Some(&acknowledgement.designation_ref)
            || previous.validator_execution.as_ref() != Some(&acknowledgement.execution)
            || previous.subject != acknowledgement.subject || previous.run != acknowledgement.run
            || previous.expected.aggregate_id != acknowledgement.expected.aggregate_id
            || previous.expected.base_ref != acknowledgement.expected.base_ref
            || previous.expected.base_vector_ref != acknowledgement.expected.base_vector_ref
            || previous.expected.schema_id != acknowledgement.expected.schema_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let prepared = handshake_role_authority::canonical::prepare_validator_ack(acknowledgement)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "Validator acknowledgement declaration is invalid" })?;
        let checked = self.put_validator_prepared(transaction, consumer, selector, &prepared).await?;
        checked.require_stage_ack(transaction, consumer.operation_id(), selector, &intent)?;
        Ok(checked)
    }

    async fn put_validator_prepared<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'tx>,
        prior_selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
        prepared: &handshake_role_authority::domain::PurePreparedRoleIntent,
    ) -> Result<TxCheckedRoleEvidence<'tx>, SurrealStorageError> {
        let mut selector = crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector {
            evidence_ref: prior_selector.evidence_ref.clone(),
            expectation: handshake_role_authority::domain::ResolutionExpectation {
                receipt_scope: prepared.body().scope.clone(), role: prepared.body().role.clone(),
                run: prepared.body().run.clone(), subject: prepared.body().subject.clone(),
                purpose: prepared.body().purpose.clone(), expected: prepared.body().expected.clone(),
            },
            designation_ref: prepared.body().designation.clone(), producer: prior_selector.producer.clone(),
        };
        // The canonical Owner designation must bind the actual runner before recording any verdict.
        self.validate_designation_for_consumer(transaction, consumer, &selector, prepared.body()).await?;
        let PendingOwnerRoleEvidence { mut readback, verified } = self.put_prepared_operator_evidence_in_transaction(
            transaction, consumer.operation_id(), consumer.current_scope(), prepared, &RoleEvidenceCreationAuthority::Consumer(consumer),
        ).await?;
        selector.evidence_ref = readback.selector.evidence_ref;
        readback.selector = selector;
        Ok(TxCheckedRoleEvidence {
            transaction, operation_id: readback.operation_id, current_consumer: readback.current_consumer,
            selector: readback.selector, verified, issuance_operation_id: readback.issuance_operation_id,
            command: consumer.command().clone(),
        })
    }

    pub(crate) async fn resolve_role_evidence<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        consumer: &super::resource_authority::CurrentConsumerSeal<'tx>,
        selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    ) -> Result<TxCheckedRoleEvidence<'tx>, SurrealStorageError> {
        use handshake_role_authority::domain::Role;
        if selector.expectation.role == Role::Operator {
            return self.resolve_operator_role_evidence(transaction, consumer, selector).await;
        }
        if !consumer.matches_transaction(transaction)
            || !consumer.command().evidence_selectors().into_iter().any(|expected| role_selectors_match(expected, selector))
            || selector.expectation.receipt_scope.owner_account_id != consumer.current_scope().owner_account_id
            || selector.expectation.receipt_scope.workspace_id != consumer.current_scope().workspace_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let query = ModelLaneQuery::Transaction(transaction);
        let read = self.read_role_evidence_with_query(&query, selector);
        let (verified, issuance_operation_id) = if consumer.protected_consumption_body().is_some() {
            let event = selector.evidence_ref.as_str().strip_prefix("eventledger://role-authority/")
                .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            consumer.with_protected_role_evidence_consumption(event, read).await?
        } else { read.await? };
        self.validate_designation_for_consumer(transaction, consumer, selector, &verified.receipt().body).await?;
        Ok(TxCheckedRoleEvidence {
            transaction, operation_id: consumer.operation_id(), current_consumer: consumer.current_scope().clone(),
            selector: selector.clone(), verified, issuance_operation_id, command: consumer.command().clone(),
        })
    }

    pub(crate) async fn readback_role_evidence(
        &self,
        publication: &super::resource_authority::RolePublicationGuard<'_>,
        expected: &RoleEvidenceReadback,
    ) -> Result<PublishedRoleEvidence, SurrealStorageError> {
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        if publication.operation_id() != expected.operation_id || publication.current_scope() != &expected.current_consumer {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let scope = role_storage_scope(&expected.receipt.body.scope);
        let record_id = stable_record_id(ModelLaneRecordKind::RoleEvidence.as_str(), &expected.receipt.body.action.idempotency_key, &scope);
        let bytes = handshake_role_authority::canonical::canonical_receipt_bytes(&expected.receipt)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let hash = sha256_hex(&bytes);
        let read = async {
        let (verified, issuance_operation_id) = self.read_role_evidence_with_query(
            &ModelLaneQuery::Storage(&self.storage), &expected.selector,
        ).await?;
        if expected.selector.expectation.role == handshake_role_authority::domain::Role::Validator {
            let query = ModelLaneQuery::Storage(&self.storage);
            let read = self.validate_validator_designation(&query, &expected.selector, &verified.receipt().body);
            let event = expected.selector.designation_ref.as_ref().and_then(|reference|
                reference.as_str().strip_prefix("eventledger://validator-designation/"))
                .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            if publication.has_protected_role_evidence_target(event) {
                publication.with_protected_role_evidence_consumption(event, read).await?;
            } else { read.await?; }
        }
        if verified.receipt() != &expected.receipt || issuance_operation_id != expected.issuance_operation_id {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "independent role evidence readback differs from the transaction result",
            });
        }
        publication.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        Ok(PublishedRoleEvidence { operation_id: expected.operation_id, receipt: verified.receipt().clone() })
        };
        if publication.is_routing_prior(&expected.selector) {
            publication.with_routing_prior(&expected.selector, read).await
        } else if publication.is_final_promotion() {
            read.await
        } else {
            publication.with_protected_role_authority_read("role_evidence", &record_id,
                &expected.receipt.event.event_id, &hash, read).await
        }
    }

    #[cfg(feature = "test-utils")]
    pub(crate) async fn routing_ack_receipt(
        &self, execution_id: &str, stage_id: &str, scope: &ModelLaneScope,
    ) -> Result<(handshake_role_authority::domain::RoleReceipt, uuid::Uuid), SurrealStorageError> {
        #[derive(SurrealValue)]
        struct ReadBindings {
            execution_id: String, stage_id: String, owner_account_id: String, actor_principal_id: String,
            authenticated_session_id: String, access_space_id: String, workspace_id: String,
        }
        let rows = ModelLaneQuery::Storage(&self.storage).query_values::<CanonicalRoleEvidenceRow, _>(
            "SELECT record::id(id) AS record_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq, event_ledger_event_id.event_sequence AS ledger_event_seq, event_ledger_event_id.event_id AS ledger_event_id, event_ledger_event_id.payload.event_stream_version AS ledger_event_stream_version, event_ledger_event_id.session_run_id AS ledger_stream_id, event_ledger_event_id.payload.event_payload_json AS payload_json, event_ledger_event_id.payload_hash AS payload_hash, event_ledger_event_id.causation_id AS causation_id, event_ledger_event_id.correlation_id AS correlation_id, event_ledger_event_id.payload.issuance_operation_id AS issuance_operation_id FROM model_lane_authority WHERE record_kind = 'role_evidence' AND fn::wp012_routing_ack_receipt_read($this, $execution_id, $stage_id) AND event_ledger_event_id = type::record('kernel_event_ledger', event_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.event_version = 'kernel_event_v1' AND event_ledger_event_id.event_type = 'MODEL_LANE_AUTHORITY_RECORDED' AND event_ledger_event_id.actor_kind = 'principal' AND event_ledger_event_id.actor_id = $actor_principal_id AND event_ledger_event_id.aggregate_type = 'role_evidence' AND event_ledger_event_id.aggregate_id = aggregate_id AND event_ledger_event_id.kernel_task_run_id = run_id AND event_ledger_event_id.payload.run_id = run_id AND event_ledger_event_id.payload.record_kind = record_kind LIMIT 2;", ReadBindings {
                execution_id: execution_id.to_owned(), stage_id: stage_id.to_owned(), owner_account_id: scope.owner_account_id.clone(),
                actor_principal_id: scope.actor_principal_id.clone(), authenticated_session_id: scope.authenticated_session_id.clone(),
                access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
            }).await?;
        let [row] = rows.as_slice() else {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "selected canonical routing ACK receipt is missing or ambiguous" });
        };
        let receipt: handshake_role_authority::domain::RoleReceipt = serde_json::from_str(&row.record_json)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        if &role_storage_scope(&receipt.body.scope) != scope { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let handshake_role_authority::domain::Purpose::StageAcknowledgement(stage) = &receipt.body.purpose else {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        };
        if stage.execution_id != execution_id || stage.stage_id != stage_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let selector = crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector {
            evidence_ref: handshake_role_authority::domain::RoleEvidenceRef::parse(&format!("eventledger://role-authority/{}", row.event_id))
                .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?,
            expectation: handshake_role_authority::domain::ResolutionExpectation {
                receipt_scope: receipt.body.scope.clone(), role: receipt.body.role.clone(), run: receipt.body.run.clone(),
                subject: receipt.body.subject.clone(), purpose: receipt.body.purpose.clone(), expected: receipt.body.expected.clone(),
            }, designation_ref: receipt.body.designation.clone(), producer: None,
        };
        let (verified, operation_id) = self.validate_role_evidence_row(row, &row.event_id, &selector)?;
        if operation_id.is_nil() { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        Ok((verified.receipt().clone(), operation_id))
    }

    pub(crate) async fn read_routing_prior_in_transaction<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        seal: &super::resource_authority::RoutingAuthorityCaptureReadSeal<'tx>,
        canonical_prior_ref: &handshake_role_authority::domain::RoleEvidenceRef,
    ) -> Result<(handshake_role_authority::domain::RoleReceipt, uuid::Uuid), SurrealStorageError> {
        if !seal.matches_transaction(transaction) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let reference = canonical_prior_ref.to_string();
        let event_id = reference.strip_prefix("eventledger://role-authority/")
            .filter(|value| !value.is_empty())
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "routing prior reference is malformed" })?;
        let current = seal.current_scope();
        let bindings = RoutingPriorLookupBindings {
            routing_prior_stage_id: seal.stage_id().to_owned(),
            event_id: event_id.to_owned(),
            owner_account_id: current.owner_account_id.to_string(),
            actor_principal_id: current.actor_principal_id.to_string(),
            authenticated_session_id: current.authenticated_session_id.to_string(),
            access_space_id: current.access_space_id.to_string(),
            workspace_id: current.workspace_id.as_str().to_owned(),
        };
        let rows = transaction.query_values_at::<CanonicalRoleEvidenceRow, _>(
            "SELECT record::id(id) AS record_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq, event_ledger_event_id.event_sequence AS ledger_event_seq, event_ledger_event_id.event_id AS ledger_event_id, event_ledger_event_id.payload.event_stream_version AS ledger_event_stream_version, event_ledger_event_id.session_run_id AS ledger_stream_id, event_ledger_event_id.payload.event_payload_json AS payload_json, event_ledger_event_id.payload_hash AS payload_hash, event_ledger_event_id.causation_id AS causation_id, event_ledger_event_id.correlation_id AS correlation_id, event_ledger_event_id.payload.issuance_operation_id AS issuance_operation_id FROM model_lane_authority WHERE record_kind = 'role_evidence' AND event_id = $event_id AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = owner_account_id AND event_ledger_event_id.actor_principal_id = actor_principal_id AND event_ledger_event_id.authenticated_session_id = authenticated_session_id AND event_ledger_event_id.access_space_id = access_space_id AND event_ledger_event_id.workspace_id = workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.event_version = 'kernel_event_v1' AND event_ledger_event_id.event_type = 'MODEL_LANE_AUTHORITY_RECORDED' AND event_ledger_event_id.actor_kind = 'principal' AND event_ledger_event_id.actor_id = actor_principal_id AND event_ledger_event_id.aggregate_type = 'role_evidence' AND event_ledger_event_id.aggregate_id = aggregate_id AND event_ledger_event_id.kernel_task_run_id = run_id AND event_ledger_event_id.payload.run_id = run_id AND event_ledger_event_id.payload.record_kind = record_kind LIMIT 2;", bindings, 0,
        ).await?;
        if rows.len() != 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "canonical routing prior is missing or ambiguous" });
        }
        let row = &rows[0];
        if row.owner_account_id != current.owner_account_id.to_string()
            || row.workspace_id != current.workspace_id.as_str() {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let receipt: handshake_role_authority::domain::RoleReceipt = serde_json::from_str(&row.record_json)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "canonical routing prior receipt is malformed" })?;
        let issuer = &receipt.body.scope;
        let issuer_scope = ModelLaneScope {
            owner_account_id: issuer.owner_account_id.to_string(),
            actor_principal_id: issuer.actor_principal_id.to_string(),
            authenticated_session_id: issuer.authenticated_session_id.to_string(),
            access_space_id: issuer.access_space_id.to_string(),
            workspace_id: issuer.workspace_id.as_str().to_owned(),
        };
        if row.owner_account_id != issuer_scope.owner_account_id
            || row.actor_principal_id != issuer_scope.actor_principal_id
            || row.authenticated_session_id != issuer_scope.authenticated_session_id
            || row.access_space_id != issuer_scope.access_space_id
            || row.workspace_id != issuer_scope.workspace_id
            || row.record_id != stable_record_id(ModelLaneRecordKind::RoleEvidence.as_str(), &receipt.body.action.idempotency_key, &issuer_scope) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "canonical routing prior immutable issuer or record identity differs" });
        }
        // Decoded expectations check integrity only; Account binds the canonical stage before command capture.
        let selector = crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector {
            evidence_ref: canonical_prior_ref.clone(),
            expectation: handshake_role_authority::domain::ResolutionExpectation {
                receipt_scope: issuer.clone(), role: receipt.body.role.clone(),
                run: receipt.body.run.clone(), subject: receipt.body.subject.clone(),
                purpose: receipt.body.purpose.clone(), expected: receipt.body.expected.clone(),
            },
            designation_ref: None, producer: None,
        };
        let (verified, issuance_operation_id) = self.validate_role_evidence_row(row, event_id, &selector)?;
        Ok((verified.receipt().clone(), issuance_operation_id))
    }

    async fn read_role_evidence_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    ) -> Result<(handshake_role_authority::domain::PureVerifiedRoleEvidence, uuid::Uuid), SurrealStorageError> {
        let reference = selector.evidence_ref.to_string();
        let event_id = reference.strip_prefix("eventledger://role-authority/")
            .filter(|value| !value.is_empty())
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "role evidence reference is malformed" })?;
        let scope = &selector.expectation.receipt_scope;
        let bindings = RoleEvidenceLookupBindings {
            event_id: event_id.to_owned(),
            owner_account_id: scope.owner_account_id.to_string(),
            actor_principal_id: scope.actor_principal_id.to_string(),
            authenticated_session_id: scope.authenticated_session_id.to_string(),
            access_space_id: scope.access_space_id.to_string(),
            workspace_id: scope.workspace_id.as_str().to_owned(),
        };
        let rows = query.query_values::<CanonicalRoleEvidenceRow, _>(
            "SELECT record::id(id) AS record_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq, event_ledger_event_id.event_sequence AS ledger_event_seq, event_ledger_event_id.event_id AS ledger_event_id, event_ledger_event_id.payload.event_stream_version AS ledger_event_stream_version, event_ledger_event_id.session_run_id AS ledger_stream_id, event_ledger_event_id.payload.event_payload_json AS payload_json, event_ledger_event_id.payload_hash AS payload_hash, event_ledger_event_id.causation_id AS causation_id, event_ledger_event_id.correlation_id AS correlation_id, event_ledger_event_id.payload.issuance_operation_id AS issuance_operation_id FROM model_lane_authority WHERE record_kind = 'role_evidence' AND event_id = $event_id AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.event_version = 'kernel_event_v1' AND event_ledger_event_id.event_type = 'MODEL_LANE_AUTHORITY_RECORDED' AND event_ledger_event_id.actor_kind = 'principal' AND event_ledger_event_id.actor_id = $actor_principal_id AND event_ledger_event_id.aggregate_type = 'role_evidence' AND event_ledger_event_id.aggregate_id = aggregate_id AND event_ledger_event_id.kernel_task_run_id = run_id AND event_ledger_event_id.payload.run_id = run_id AND event_ledger_event_id.payload.record_kind = record_kind LIMIT 2;",
            bindings,
        ).await?;
        if rows.len() != 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role evidence is missing or ambiguous" });
        }
        self.validate_role_evidence_row(&rows[0], event_id, selector)
    }

    fn validate_role_evidence_row(
        &self,
        row: &CanonicalRoleEvidenceRow,
        event_id: &str,
        selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    ) -> Result<(handshake_role_authority::domain::PureVerifiedRoleEvidence, uuid::Uuid), SurrealStorageError> {
        let receipt: handshake_role_authority::domain::RoleReceipt = serde_json::from_str(&row.record_json)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role receipt JSON is malformed" })?;
        let payload_hash = handshake_role_authority::domain::ContentSha256::from_hex(&row.payload_hash)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role event hash is malformed" })?;
        let payload = handshake_role_authority::domain::CanonicalEventPayload {
            key: receipt.event.clone(), payload_json: row.payload_json.clone(), payload_hash,
        };
        let expected_causation = match &receipt.body.purpose {
            handshake_role_authority::domain::Purpose::PriorSelection => receipt.body.action.request_id.as_str(),
            handshake_role_authority::domain::Purpose::StageAcknowledgement(stage) => stage.authority_request_message_ref.as_str(),
        };
        if row.record_json.as_bytes() != row.payload_json.as_bytes()
            || row.event_id != receipt.event.event_id || row.event_id != event_id || row.ledger_event_id != row.event_id
            || u64::try_from(row.event_seq).ok() != Some(receipt.event.event_seq)
            || row.ledger_event_seq != row.event_seq
            || u64::try_from(row.transaction_seq).ok() != Some(receipt.event.transaction_seq)
            || row.event_stream_version <= 0 || row.event_stream_version != row.ledger_event_stream_version
            || row.ledger_stream_id != receipt.event.stream_id
            || row.run_id != receipt.body.run.run_id || row.aggregate_id != receipt.body.action.action_id
            || row.idempotency_key != receipt.body.action.idempotency_key
            || row.causation_id.as_deref() != Some(expected_causation)
            || row.correlation_id.as_deref() != Some(receipt.body.run.trace_id.as_str()) {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role record, event, request, or causation binding differs" });
        }
        let verified = handshake_role_authority::resolution::verify_receipt(&receipt, &payload, &selector.expectation)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role receipt integrity or expected binding differs" })?;
        if verified.reference() != &selector.evidence_ref {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role receipt reference differs" });
        }
        let issuance_operation_id = uuid::Uuid::parse_str(&row.issuance_operation_id)
            .map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "canonical role issuance operation is malformed" })?;
        Ok((verified, issuance_operation_id))
    }

    pub(crate) async fn get_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        kind: ModelLaneRecordKind,
        aggregate_id: &str,
        issuer: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(issuer)?;
        let rows = transaction.query_values_at::<StoredRow, _>(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;",
            scoped_bindings(kind, aggregate_id, "", "", issuer),
            0,
        ).await?;
        if rows.len() > 1 {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "provided-transaction authority lookup is ambiguous",
            });
        }
        Ok(rows.into_iter().next().map(Into::into))
    }

    pub(crate) async fn original_run_create_canonical_records(
        &self,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCanonicalSet, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = scoped_bindings(ModelLaneRecordKind::Run, "", "", "", scope);
        let observed = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_first::<CanonicalScopeQueryResult, _>(
                "RETURN { scope_records: (SELECT record_kind, aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind IN ['run', 'lane', 'message', 'recovery_checkpoint', 'recovery_event', 'lease'] AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC, aggregate_id ASC), ordered_ledger: (SELECT event_id, event_sequence AS event_seq, payload.event_stream_version AS event_stream_version, event_sequence AS transaction_seq, aggregate_id, payload.run_id AS run_id, session_run_id, kernel_task_run_id, payload_hash, payload.event_payload_json AS event_payload_json FROM kernel_event_ledger WHERE source_component = 'model_lane' AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_sequence ASC, event_id ASC) };",
                bindings,
            ).await
        })).await?.ok_or(SurrealStorageError::InvalidModelLaneRecord {
            reason: "canonical scope observation returned no result",
        })?;
        let mut scope_records = Vec::with_capacity(observed.scope_records.len());
        for row in observed.scope_records {
            let kind = match row.record_kind.as_str() {
                "run" => ModelLaneRecordKind::Run,
                "lane" => ModelLaneRecordKind::Lane,
                "message" => ModelLaneRecordKind::Message,
                "recovery_checkpoint" => ModelLaneRecordKind::RecoveryCheckpoint,
                "recovery_event" => ModelLaneRecordKind::RecoveryEvent,
                "lease" => ModelLaneRecordKind::Lease,
                _ => return Err(SurrealStorageError::InvalidModelLaneRecord {
                    reason: "canonical scope observation has an unexpected record kind",
                }),
            };
            scope_records.push((kind, SurrealModelLaneRecord {
                aggregate_id: row.aggregate_id, run_id: row.run_id,
                idempotency_key: row.idempotency_key, record_json: row.record_json,
                event_id: row.event_id, event_seq: row.event_seq,
                event_stream_version: row.event_stream_version, transaction_seq: row.transaction_seq,
            }));
        }
        Ok(SurrealModelLaneCanonicalSet { scope_records, ordered_ledger: observed.ordered_ledger })
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
                "SELECT record_kind, aggregate_id, run_id, event_id, event_ledger_event_id.event_sequence AS event_ledger_seq, event_ledger_event_id.event_type AS event_type, event_ledger_event_id.payload_hash AS payload_hash, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id FROM model_lane_authority WHERE run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.aggregate_type = record_kind AND event_ledger_event_id.aggregate_id = aggregate_id AND event_ledger_event_id.kernel_task_run_id = run_id AND event_ledger_event_id.payload.run_id = run_id AND event_ledger_event_id.session_run_id = IF record_kind IN ['session_cleanup_receipt', 'routing_execution'] { run_id } ELSE IF record_kind = 'message' { encoding::json::decode(record_json).inner.event_ledger_stream_id } ELSE { encoding::json::decode(record_json).event_ledger_stream_id } AND event_seq = event_ledger_event_id.event_sequence ORDER BY event_ledger_seq ASC LIMIT $row_limit;",
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
        self.find_by_term_with_query(&ModelLaneQuery::Storage(&self.storage), kind, search_term, scope).await
    }

    pub(crate) async fn find_by_term_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        kind: ModelLaneRecordKind,
        search_term: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        self.find_by_term_with_query(&ModelLaneQuery::Transaction(transaction), kind, search_term, scope).await
    }

    async fn find_by_term_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        kind: ModelLaneRecordKind,
        search_term: &str,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        query.query_values::<StoredRow, _>(
            "SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM model_lane_authority WHERE record_kind = $record_kind AND $search_term IN search_terms AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC;",
            scoped_bindings(kind, "", "", search_term, scope),
        ).await.map(|rows| rows.into_iter().map(Into::into).collect())
    }

    pub(crate) async fn crdt_update_by_ref(
        &self,
        update_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        self.crdt_update_by_ref_with_query(&ModelLaneQuery::Storage(&self.storage), update_bytes_ref, scope).await
    }

    pub(crate) async fn crdt_update_by_ref_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        update_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        self.crdt_update_by_ref_with_query(&ModelLaneQuery::Transaction(transaction), update_bytes_ref, scope).await
    }

    async fn crdt_update_by_ref_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        update_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            authority_ref: update_bytes_ref.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        query
                        .query_first::<SurrealModelLaneCrdtUpdate, _>(
                            "SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM kernel_crdt_updates WHERE update_bytes_ref = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await
    }

    pub(crate) async fn crdt_update_by_id(
        &self,
        update_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        self.crdt_update_by_id_with_query(&ModelLaneQuery::Storage(&self.storage), update_id, scope).await
    }

    pub(crate) async fn crdt_update_by_id_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        update_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        self.crdt_update_by_id_with_query(&ModelLaneQuery::Transaction(transaction), update_id, scope).await
    }

    async fn crdt_update_by_id_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        update_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            authority_ref: update_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        query
                        .query_first::<SurrealModelLaneCrdtUpdate, _>(
                            "SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM kernel_crdt_updates WHERE update_id = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await
    }

    pub(crate) async fn crdt_snapshot_by_ref(
        &self,
        snapshot_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtSnapshot>, SurrealStorageError> {
        self.crdt_snapshot_by_ref_with_query(&ModelLaneQuery::Storage(&self.storage), snapshot_bytes_ref, scope).await
    }

    pub(crate) async fn crdt_snapshot_by_ref_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        snapshot_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtSnapshot>, SurrealStorageError> {
        self.crdt_snapshot_by_ref_with_query(&ModelLaneQuery::Transaction(transaction), snapshot_bytes_ref, scope).await
    }

    async fn crdt_snapshot_by_ref_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        snapshot_bytes_ref: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtSnapshot>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            authority_ref: snapshot_bytes_ref.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        query
                        .query_first::<SurrealModelLaneCrdtSnapshot, _>(
                            "SELECT schema_id, snapshot_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq, state_vector, snapshot_sha256, snapshot_bytes_ref, encoding::base64::encode(snapshot_bytes) AS snapshot_bytes_b64, actor_id, actor_kind, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, promotion_evidence_update_ids, storage_authority, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.document_id AS ledger_document_id, event_ledger_event_id.payload.state_vector AS ledger_state_vector, event_ledger_event_id.payload.covered_update_seq AS ledger_covered_update_seq FROM kernel_crdt_snapshots WHERE snapshot_bytes_ref = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await
    }

    pub(crate) async fn crdt_proposal_base_snapshot(
        &self, document_id: &str, crdt_document_id: &str, base_update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtSnapshot, SurrealStorageError> {
        self.crdt_proposal_base_snapshot_with_query(&ModelLaneQuery::Storage(&self.storage), document_id, crdt_document_id, base_update_seq, scope).await
    }

    pub(crate) async fn crdt_proposal_base_snapshot_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>, document_id: &str, crdt_document_id: &str, base_update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtSnapshot, SurrealStorageError> {
        self.crdt_proposal_base_snapshot_with_query(&ModelLaneQuery::Transaction(transaction), document_id, crdt_document_id, base_update_seq, scope).await
    }

    async fn crdt_proposal_base_snapshot_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>, document_id: &str, crdt_document_id: &str, base_update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<SurrealModelLaneCrdtSnapshot, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtChainBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(), document_id: document_id.to_owned(),
            crdt_document_id: crdt_document_id.to_owned(), covered_update_seq: base_update_seq, update_seq: base_update_seq,
        };
        let rows = query.query_values::<SurrealModelLaneCrdtSnapshot, _>(
                "SELECT schema_id, snapshot_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq, state_vector, snapshot_sha256, snapshot_bytes_ref, encoding::base64::encode(snapshot_bytes) AS snapshot_bytes_b64, actor_id, actor_kind, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, promotion_evidence_update_ids, storage_authority, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.document_id AS ledger_document_id, event_ledger_event_id.payload.state_vector AS ledger_state_vector, event_ledger_event_id.payload.covered_update_seq AS ledger_covered_update_seq FROM kernel_crdt_snapshots WHERE document_id = $document_id AND crdt_document_id = $crdt_document_id AND covered_update_seq = $covered_update_seq AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 2;", bindings,
            ).await?;
        let mut rows = rows;
        if rows.len() != 1 { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "proposal base snapshot is missing or ambiguous" }); }
        Ok(rows.remove(0))
    }

    pub(crate) async fn crdt_update_chain(
        &self,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        self.crdt_update_chain_with_query(&ModelLaneQuery::Storage(&self.storage), document_id, crdt_document_id, covered_update_seq, update_seq, scope).await
    }

    pub(crate) async fn crdt_update_chain_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        self.crdt_update_chain_with_query(&ModelLaneQuery::Transaction(transaction), document_id, crdt_document_id, covered_update_seq, update_seq, scope).await
    }

    async fn crdt_update_chain_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<SurrealModelLaneCrdtUpdate>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtChainBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
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
        query
                        .query_values::<SurrealModelLaneCrdtUpdate, _>(
                            "SELECT schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_id, update_seq, update_sha256, update_bytes_ref, encoding::base64::encode(update_bytes) AS update_bytes_b64, actor_id, actor_kind, session_id, trace_id, state_vector_before, state_vector_after, replay_metadata_json.replay_order_key AS replay_order_key, replay_metadata_json.dependency_update_ids AS dependency_update_ids, replay_metadata_json.encoding AS replay_encoding, replay_metadata_json.schema_version AS replay_schema_version, event_ledger_stream_id, record::id(event_ledger_event_id) AS event_ledger_event_id, storage_authority, event_ledger_event_id.session_run_id AS ledger_session_run_id, event_ledger_event_id.event_type AS ledger_event_type, event_ledger_event_id.aggregate_type AS ledger_aggregate_type, event_ledger_event_id.aggregate_id AS ledger_aggregate_id, event_ledger_event_id.actor_kind AS ledger_actor_kind, event_ledger_event_id.actor_id AS ledger_actor_id, event_ledger_event_id.correlation_id AS ledger_correlation_id, event_ledger_event_id.payload_hash AS ledger_payload_hash, event_ledger_event_id.payload.update_id AS ledger_update_id, event_ledger_event_id.payload.update_seq AS ledger_update_seq, event_ledger_event_id.payload.actor_id AS ledger_actor_payload_id, event_ledger_event_id.payload.update_sha256 AS ledger_update_sha256, event_ledger_event_id.payload.state_vector_before AS ledger_state_vector_before, event_ledger_event_id.payload.state_vector_after AS ledger_state_vector_after, event_ledger_event_id.payload.site_id AS ledger_site_id FROM kernel_crdt_updates WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND update_seq > $covered_update_seq AND update_seq <= $update_seq ORDER BY update_seq ASC;",
                            bindings,
                        )
                        .await
    }

    pub(crate) async fn crdt_seen_update_ids(
        &self,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<String>, SurrealStorageError> {
        self.crdt_seen_update_ids_with_query(&ModelLaneQuery::Storage(&self.storage), document_id, crdt_document_id, covered_update_seq, scope).await
    }

    pub(crate) async fn crdt_seen_update_ids_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<String>, SurrealStorageError> {
        self.crdt_seen_update_ids_with_query(&ModelLaneQuery::Transaction(transaction), document_id, crdt_document_id, covered_update_seq, scope).await
    }

    async fn crdt_seen_update_ids_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        document_id: &str,
        crdt_document_id: &str,
        covered_update_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<Vec<String>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtChainBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
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
        let rows = query.query_values::<CrdtUpdateIdRow, _>(
                "SELECT update_id FROM kernel_crdt_updates WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id AND update_seq <= $covered_update_seq ORDER BY update_seq ASC;",
                bindings,
            ).await?;
        Ok(rows.into_iter().map(|row| row.update_id).collect())
    }

    #[cfg(feature = "test-utils")]
    pub(crate) fn admission_test_control(&self) -> ContextAdmissionTestControl { self.admission_test_control.clone() }

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
        let mut rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<SurrealModelLaneCrdtLease, _>(
                "LET $admission = time::now(); SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, $admission AS admitted_at_utc FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND claimed_at_utc <= $admission AND expires_at_utc > $admission AND released_at_utc IS NONE AND expired_at_utc IS NONE AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id AND ((scope_kind = 'workspace' AND scope_id = $workspace_id) OR (scope_kind = 'document' AND scope_id = $crdt_document_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id)) ORDER BY claimed_at_utc ASC, lease_id ASC;",
                bindings,
                1,
            ).await
        })).await?;
        if rows.len() == 1 {
            let guards = super::MODEL_LANE_CURRENT_GUARDS.try_with(Clone::clone)
                .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
            let original = {
                let guards = guards.lock().map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "coordination guard state poisoned" })?;
                if guards.len() != 1 { return Err(SurrealStorageError::TransactionAuthorityRequired); }
                guards[0].clone()
            };
            let updated = original.clone().stamp_crdt_admission(&self.storage, document_id, crdt_document_id).await?;
            rows[0].admitted_at_utc = updated.row.validated_admission_at.expect("canonical engine admission timestamp");
            {
                let mut guards = guards.lock().map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "coordination guard state poisoned" })?;
                if guards.len() != 1 || guards[0].operation_id != original.operation_id
                    || guards[0].row.generation != original.row.generation || guards[0].row.revision != original.row.revision {
                    return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "CRDT admission guard changed before canonical acknowledgement" });
                }
                guards[0] = updated;
            }
            #[cfg(feature = "test-utils")]
            self.admission_test_control.after_canonical_admission(ContextAdmissionTestObservation {
                document_id: document_id.to_owned(), crdt_document_id: crdt_document_id.to_owned(),
                lease_id: rows[0].lease_id.clone(), admitted_at: rows[0].admitted_at_utc,
            }).await?;
        }
        Ok(rows)
    }

    pub(crate) async fn crdt_proposal(
        &self,
        proposal_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposal>, SurrealStorageError> {
        self.crdt_proposal_with_query(&ModelLaneQuery::Storage(&self.storage), proposal_id, scope).await
    }

    pub(crate) async fn crdt_proposal_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        proposal_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposal>, SurrealStorageError> {
        self.crdt_proposal_with_query(&ModelLaneQuery::Transaction(transaction), proposal_id, scope).await
    }

    async fn crdt_proposal_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        proposal_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposal>, SurrealStorageError> {
        Ok(self
            .crdt_proposal_record_with_query(query, proposal_id, scope)
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
        self.crdt_proposal_record_with_query(&ModelLaneQuery::Storage(&self.storage), proposal_id, scope).await
    }

    pub(crate) async fn crdt_proposal_record_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        proposal_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        self.crdt_proposal_record_with_query(&ModelLaneQuery::Transaction(transaction), proposal_id, scope).await
    }

    async fn crdt_proposal_record_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        proposal_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtProposalRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtReferenceBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            authority_ref: proposal_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let row = query
                        .query_first::<SurrealModelLaneCrdtProposalRecord, _>(
                            "SELECT proposal_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, base_update_seq, base_state_vector, proposed_diff, diff_sha256, source_span_citations, actor_id, actor_kind, session_id, correlation_id, lease_id.lease_id AS lease_id, review_state, decided_by, decided_at_utc, decision_reason, record::id(recorded_event_id) AS recorded_event_id, record::id(decided_event_id) AS decided_event_id, record::id(promotion_requested_event_id) AS promotion_requested_event_id, record::id(promotion_accepted_event_id) AS promotion_accepted_event_id, applied_update_id, applied_update_sha256, record::id(applied_event_id) AS applied_event_id, record::id(last_transition_event_id) AS last_transition_event_id, created_at_utc FROM knowledge_crdt_ai_edit_proposals WHERE proposal_id = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id LIMIT 1;",
                            bindings,
                        )
                        .await?;
        if let Some(row) = row.as_ref() {
            self.validate_crdt_proposal_receipts_with_query(query, row, scope).await?;
        }
        Ok(row)
    }

    pub(crate) async fn crdt_proposal_application_links(
        &self, proposal: &SurrealModelLaneCrdtProposalRecord, update: &SurrealModelLaneCrdtUpdate,
        scope: &ModelLaneScope,
    ) -> Result<(i64, i64, Option<Value>), SurrealStorageError> {
        self.crdt_proposal_application_links_with_query(&ModelLaneQuery::Storage(&self.storage), proposal, update, scope).await
    }

    pub(crate) async fn crdt_proposal_application_links_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>, proposal: &SurrealModelLaneCrdtProposalRecord, update: &SurrealModelLaneCrdtUpdate,
        scope: &ModelLaneScope,
    ) -> Result<(i64, i64, Option<Value>), SurrealStorageError> {
        self.crdt_proposal_application_links_with_query(&ModelLaneQuery::Transaction(transaction), proposal, update, scope).await
    }

    async fn crdt_proposal_application_links_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>, proposal: &SurrealModelLaneCrdtProposalRecord, update: &SurrealModelLaneCrdtUpdate,
        scope: &ModelLaneScope,
    ) -> Result<(i64, i64, Option<Value>), SurrealStorageError> {
        validate_scope(scope)?;
        let approval = proposal.decided_event_id.as_ref().ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "proposal approval receipt is missing" })?;
        let mut event_ids = vec![approval.clone(), update.event_ledger_event_id.clone()];
        if let Some(application) = &proposal.applied_event_id { event_ids.push(application.clone()); }
        let bindings = CrdtProposalReceiptBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            event_ids: event_ids.clone(), owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(), authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
        };
        if bindings.navigation_selection.is_some() {
            let rows = query.query_values::<NavigationApplicationReceiptRow, _>(
                    "SELECT event_id, event_sequence, navigation_semantic_input AS semantic FROM kernel_event_ledger WHERE event_id IN $event_ids AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;", bindings,
                ).await?;
            let by_id = rows.into_iter().map(|row| (row.event_id.clone(), row)).collect::<BTreeMap<_, _>>();
            if by_id.len() != event_ids.len() || event_ids.iter().any(|id| !by_id.contains_key(id)) {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "proposal application canonical receipts are missing or ambiguous" });
            }
            let approval_seq = by_id[approval].event_sequence;
            let update_seq = by_id[&update.event_ledger_event_id].event_sequence;
            if approval_seq >= update_seq {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "applied update precedes canonical approval" });
            }
            let application = if let Some(id) = &proposal.applied_event_id {
                let row = &by_id[id];
                if row.event_sequence <= update_seq {
                    return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "application does not follow canonical update" });
                }
                let input = row.semantic.as_ref().filter(|input| input.schema_id == "hsk.model_lane_navigation_receipt_input@1")
                    .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "application canonical semantic input is missing" })?;
                let evidence = input.application_evidence.as_ref()
                    .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "application evidence is missing" })?;
                Some(serde_json::to_value(evidence).map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
                    reason: "application canonical semantic input serialization failed",
                })?)
            } else { None };
            return Ok((approval_seq, update_seq, application));
        }
        let rows = query.query_values::<Value, _>(
                "SELECT event_id, event_sequence, payload, payload_hash FROM kernel_event_ledger WHERE event_id IN $event_ids AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;", bindings,
            ).await?;
        if rows.len() != event_ids.len() { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "proposal application canonical receipts are missing or ambiguous" }); }
        let find = |id: &str| rows.iter().find(|row| row.get("event_id").and_then(Value::as_str) == Some(id));
        let approval_seq = find(approval).and_then(|row| row.get("event_sequence")).and_then(Value::as_i64)
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "approval canonical sequence is missing" })?;
        let update_seq = find(&update.event_ledger_event_id).and_then(|row| row.get("event_sequence")).and_then(Value::as_i64)
            .ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "update canonical sequence is missing" })?;
        if approval_seq >= update_seq { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "applied update precedes canonical approval" }); }
        let application = if let Some(id) = &proposal.applied_event_id {
            let row = find(id).ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "application canonical receipt is missing" })?;
            if row.get("event_sequence").and_then(Value::as_i64).is_none_or(|sequence| sequence <= update_seq) {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "application does not follow canonical update" });
            }
            let payload = row.get("payload").ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "application canonical payload is missing" })?;
            if row.get("payload_hash").and_then(Value::as_str) != Some(sha256_hex(serde_json::to_string(payload).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "application canonical payload JSON serialization failed" })?.as_bytes()).as_str()) {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "application canonical payload hash differs" });
            }
            Some(payload.get("application_evidence").cloned().ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "application evidence is missing" })?)
        } else { None };
        Ok((approval_seq, update_seq, application))
    }

    async fn validate_crdt_proposal_receipts(
        &self,
        proposal: &SurrealModelLaneCrdtProposalRecord,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        self.validate_crdt_proposal_receipts_with_query(&ModelLaneQuery::Storage(&self.storage), proposal, scope).await
    }

    async fn validate_crdt_proposal_receipts_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        proposal: &SurrealModelLaneCrdtProposalRecord,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        self.validate_crdt_proposal_receipts_with_query(&ModelLaneQuery::Transaction(transaction), proposal, scope).await
    }

    async fn validate_crdt_proposal_receipts_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        proposal: &SurrealModelLaneCrdtProposalRecord,
        scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        let event_ids = proposal_receipt_ids(proposal)?;
        let navigation = NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok();
        let bindings = CrdtProposalReceiptBindings {
            navigation_selection: navigation.clone(),
            event_ids,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let receipts = if navigation.is_some() {
                query.query_values::<CrdtProposalSemanticReceiptRow, _>(
                    "SELECT event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, causation_id, correlation_id, source_component, navigation_semantic_input AS semantic FROM kernel_event_ledger WHERE navigation_semantic_input != NONE AND event_id IN $event_ids AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;", bindings,
                ).await
            } else {
                query.query_values::<CrdtProposalReceiptRow, _>(
                    "SELECT event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, causation_id, correlation_id, payload_hash, source_component, payload FROM kernel_event_ledger WHERE event_id IN $event_ids AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;", bindings,
                ).await?.into_iter().map(CrdtProposalReceiptRow::into_semantic).collect()
            }?;
        validate_crdt_proposal_receipt_rows(proposal, receipts)?;
        if let Some(update_id) = &proposal.applied_update_id {
            let update = self.crdt_update_by_id_with_query(query, update_id, scope).await?.ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "applied canonical Yjs update is missing" })?;
            let snapshot = self.crdt_proposal_base_snapshot_with_query(query, &proposal.document_id, &proposal.crdt_document_id, proposal.base_update_seq, scope).await?;
            let base = self.crdt_update_chain_with_query(query, &proposal.document_id, &proposal.crdt_document_id, 0, proposal.base_update_seq, scope).await?;
            if base.last().is_none_or(|base| base.state_vector_after != proposal.base_state_vector) || snapshot.state_vector != proposal.base_state_vector {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "applied proposal canonical base is invalid" });
            }
            let seen = self.crdt_seen_update_ids_with_query(query, &proposal.document_id, &proposal.crdt_document_id, proposal.base_update_seq, scope).await?.into_iter().collect();
            let (approval_sequence, update_sequence, actual) = self.crdt_proposal_application_links_with_query(query, proposal, &update, scope).await?;
            let expected = crate::swarm_orchestration::model_lane::derive_crdt_proposal_application_evidence(
                proposal, &update, &snapshot, &base, scope, seen, approval_sequence, update_sequence,
            ).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "applied proposal does not realize its canonical approved operation" })?;
            if actual.as_ref() != Some(&expected) { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "application receipt differs from canonical realization" }); }
        }
        Ok(())
    }

    pub(crate) async fn crdt_lease_history(
        &self,
        lease_id: &str,
        document_id: &str,
        crdt_document_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        self.crdt_lease_history_with_query(&ModelLaneQuery::Storage(&self.storage), lease_id, document_id, crdt_document_id, scope).await
    }

    pub(crate) async fn crdt_lease_history_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        lease_id: &str,
        document_id: &str,
        crdt_document_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        self.crdt_lease_history_with_query(&ModelLaneQuery::Transaction(transaction), lease_id, document_id, crdt_document_id, scope).await
    }

    async fn crdt_lease_history_with_query(
        &self,
        query: &ModelLaneQuery<'_, '_>,
        lease_id: &str,
        document_id: &str,
        crdt_document_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<SurrealModelLaneCrdtLeaseHistory>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = CrdtLeaseHistoryBindings {
            navigation_selection: NAVIGATION_READ_SELECTOR.try_with(Clone::clone).ok(),
            authority_ref: lease_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: document_id.to_owned(),
            crdt_document_id: crdt_document_id.to_owned(),
        };
        query.query_first::<SurrealModelLaneCrdtLeaseHistory, _>(
                "SELECT lease_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, lane_id, actor_id, actor_kind, session_id, correlation_id, scope_kind, scope_id, claimed_at_utc, expires_at_utc, renewal_count, released_at_utc, expired_at_utc, record::id(takeover_of) AS takeover_of, record::id(recorded_event_id) AS recorded_event_id, record::id(last_transition_event_id) AS last_transition_event_id FROM knowledge_crdt_agent_lane_leases WHERE lease_id = $authority_ref AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND recorded_event_id.owner_account_id = $owner_account_id AND recorded_event_id.actor_principal_id = $actor_principal_id AND recorded_event_id.authenticated_session_id = $authenticated_session_id AND recorded_event_id.access_space_id = $access_space_id AND recorded_event_id.workspace_id = $workspace_id AND last_transition_event_id.owner_account_id = $owner_account_id AND last_transition_event_id.actor_principal_id = $actor_principal_id AND last_transition_event_id.authenticated_session_id = $authenticated_session_id AND last_transition_event_id.access_space_id = $access_space_id AND last_transition_event_id.workspace_id = $workspace_id AND ((scope_kind = 'workspace' AND scope_id = $workspace_id) OR (scope_kind = 'document' AND scope_id = $crdt_document_id AND document_id = $document_id AND crdt_document_id = $crdt_document_id)) LIMIT 1;",
                bindings,
            ).await
    }

    pub(crate) async fn validate_event_link(
        &self, kind: ModelLaneRecordKind, record: &SurrealModelLaneRecord, scope: &ModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        self.validated_event_payload(kind, record, scope).await.map(|_| ())
    }

    pub(crate) async fn validated_event_payload(
        &self,
        kind: ModelLaneRecordKind,
        record: &SurrealModelLaneRecord,
        scope: &ModelLaneScope,
    ) -> Result<String, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = EventLinkBindings {
            event_id: record.event_id.clone(),
            record_kind: kind.as_str().to_owned(),
            aggregate_id: record.aggregate_id.clone(),
            run_id: record.run_id.clone(),
            event_ledger_stream_id: declared_event_stream(kind, &record.record_json)?,
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
                            "SELECT event_ledger_event_id.event_id AS event_id, event_ledger_event_id.event_sequence AS event_seq, event_ledger_event_id.payload.event_stream_version AS event_stream_version, event_ledger_event_id.event_sequence AS transaction_seq, event_ledger_event_id.aggregate_id AS aggregate_id, event_ledger_event_id.payload.run_id AS run_id, event_ledger_event_id.kernel_task_run_id AS kernel_task_run_id, event_ledger_event_id.session_run_id AS session_run_id, event_ledger_event_id.payload_hash AS payload_hash, event_ledger_event_id.payload.event_payload_json AS event_payload_json FROM model_lane_authority WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id AND run_id = $run_id AND event_id = $event_id AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.aggregate_type = $record_kind AND event_ledger_event_id.aggregate_id = $aggregate_id AND event_ledger_event_id.kernel_task_run_id = $run_id AND event_ledger_event_id.payload.run_id = $run_id AND event_ledger_event_id.session_run_id = IF $event_ledger_stream_id = NONE { $run_id } ELSE { $event_ledger_stream_id } AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND ($record_kind != 'run' OR event_stream_version = 1 OR (event_stream_version > 1 AND event_ledger_event_id.event_type = 'MODEL_LANE_AUTHORITY_REPLACED' AND encoding::json::decode(event_ledger_event_id.payload.event_payload_json).schema_id = 'hsk.model_lane_run_extension@1' AND type::is_string(event_ledger_event_id.causation_id) AND string::trim(event_ledger_event_id.causation_id) != '' AND record::exists(type::record('kernel_event_ledger', event_ledger_event_id.causation_id)))) LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        validate_stored_event_link(kind, record, scope, &rows)?;
        Ok(rows[0].event_payload_json.clone())
    }

    pub(crate) async fn validate_event_link_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
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
            event_ledger_stream_id: declared_event_stream(kind, &record.record_json)?,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = transaction.query_values_at::<StoredEventLink, _>(
            "SELECT event_ledger_event_id.event_id AS event_id, event_ledger_event_id.event_sequence AS event_seq, event_ledger_event_id.payload.event_stream_version AS event_stream_version, event_ledger_event_id.event_sequence AS transaction_seq, event_ledger_event_id.aggregate_id AS aggregate_id, event_ledger_event_id.payload.run_id AS run_id, event_ledger_event_id.kernel_task_run_id AS kernel_task_run_id, event_ledger_event_id.session_run_id AS session_run_id, event_ledger_event_id.payload_hash AS payload_hash, event_ledger_event_id.payload.event_payload_json AS event_payload_json FROM model_lane_authority WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id AND run_id = $run_id AND event_id = $event_id AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'model_lane' AND event_ledger_event_id.aggregate_type = $record_kind AND event_ledger_event_id.aggregate_id = $aggregate_id AND event_ledger_event_id.kernel_task_run_id = $run_id AND event_ledger_event_id.payload.run_id = $run_id AND event_ledger_event_id.session_run_id = IF $event_ledger_stream_id = NONE { $run_id } ELSE { $event_ledger_stream_id } AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND ($record_kind != 'run' OR event_stream_version = 1 OR (event_stream_version > 1 AND event_ledger_event_id.event_type = 'MODEL_LANE_AUTHORITY_REPLACED' AND encoding::json::decode(event_ledger_event_id.payload.event_payload_json).schema_id = 'hsk.model_lane_run_extension@1' AND type::is_string(event_ledger_event_id.causation_id) AND string::trim(event_ledger_event_id.causation_id) != '' AND record::exists(type::record('kernel_event_ledger', event_ledger_event_id.causation_id)))) LIMIT 2;",
            bindings,
            0,
        ).await?;
        validate_stored_event_link(kind, record, scope, &rows)
    }

    pub(crate) async fn run_event_high_watermark(
        &self,
        run_id: &str,
        event_ledger_stream_id: &str,
        scope: &ModelLaneScope,
    ) -> Result<Option<i64>, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = run_event_bindings(run_id, Some(event_ledger_stream_id), 0, scope);
        let row = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<StoredEventSequence, _>(
                            "SELECT event_sequence AS event_seq FROM kernel_event_ledger WHERE source_component = 'model_lane' AND kernel_task_run_id = $run_id AND session_run_id = $event_ledger_stream_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_sequence DESC LIMIT 1;",
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
        event_ledger_stream_id: &str,
        event_seq: i64,
        scope: &ModelLaneScope,
    ) -> Result<bool, SurrealStorageError> {
        validate_scope(scope)?;
        let bindings = run_event_bindings(run_id, Some(event_ledger_stream_id), event_seq, scope);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredEventSequence, _>(
                            "SELECT event_sequence AS event_seq FROM kernel_event_ledger WHERE source_component = 'model_lane' AND kernel_task_run_id = $run_id AND session_run_id = $event_ledger_stream_id AND event_sequence = $event_seq AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;",
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
        let bindings = scoped_bindings(ModelLaneRecordKind::Run, "", "", "", scope);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredSchemaRow, _>(
                            "SELECT schema_id, schema_version, record_kind, table_name, source_component FROM model_lane_schema_registry WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY schema_id ASC;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() != CANONICAL_MODEL_LANE_SCHEMAS.len()
            || rows.iter().any(|row| row.record_kind != "model_lane"
                || row.table_name != "model_lane_authority"
                || row.source_component != "swarm_orchestration::model_lane"
                || !CANONICAL_MODEL_LANE_SCHEMAS.iter().any(|(id, version)| *id == row.schema_id && *version == row.schema_version))
            || rows.iter().map(|row| &row.schema_id).collect::<BTreeSet<_>>().len() != rows.len()
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "model-lane schema registry canonical metadata missing or drifted" });
        }
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

// Supplied transactions retain the caller's controller across every dependent read.
enum ModelLaneQuery<'a, 'tx> {
    Storage(&'a SurrealStorage),
    Transaction(&'a super::SurrealTransactionContext<'tx>),
}

impl ModelLaneQuery<'_, '_> {
    async fn query_values<R, B>(
        &self, statement: &'static str, bindings: B,
    ) -> Result<Vec<R>, SurrealStorageError>
    where R: SurrealValue + Send + 'static, B: SurrealValue + Send + 'static,
    {
        self.query_values_at(statement, bindings, 0).await
    }

    async fn query_first<R, B>(
        &self, statement: &'static str, bindings: B,
    ) -> Result<Option<R>, SurrealStorageError>
    where R: SurrealValue + Send + 'static, B: SurrealValue + Send + 'static,
    {
        super::decode_first_value(self.query_values(statement, bindings).await?)
    }

    async fn query_values_at<R, B>(
        &self, statement: &'static str, bindings: B, statement_index: usize,
    ) -> Result<Vec<R>, SurrealStorageError>
    where R: SurrealValue + Send + 'static, B: SurrealValue + Send + 'static,
    {
        match self {
            Self::Storage(storage) => storage.with_data_operation(|database| Box::pin(async move {
                database.query_values_at(statement, bindings, statement_index).await
            })).await,
            Self::Transaction(transaction) => {
                let (statement, statement_index) = if let Some(body) = statement.trim().strip_prefix("BEGIN TRANSACTION;") {
                    let body = body.strip_suffix("COMMIT TRANSACTION;").ok_or(
                        SurrealStorageError::InvalidModelLaneRecord {
                            reason: "provided transaction query has an unmatched legacy wrapper",
                        },
                    )?;
                    let index = statement_index.checked_sub(1).ok_or(
                        SurrealStorageError::InvalidModelLaneRecord {
                            reason: "provided transaction query has an invalid legacy result index",
                        },
                    )?;
                    (body, index)
                } else { (statement, statement_index) };
                transaction.query_values_at(statement, bindings, statement_index).await
            }
        }
    }
}

fn stage_ack_command_matches(
    command: &crate::swarm_orchestration::model_lane::RoleConsumptionCommand,
    operation_id: uuid::Uuid,
    prior_selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    acknowledgement: &crate::swarm_orchestration::model_lane::RoutingStageAcknowledgementIntent,
) -> bool {
    match command {
        crate::swarm_orchestration::model_lane::RoleConsumptionCommand::AcknowledgeRoutingStage {
            operation_id: captured_operation, selector, execution_id, stage_id, message_id: _,
            acknowledgement: captured_acknowledgement,
        } => *captured_operation == operation_id && role_selectors_match(selector, prior_selector)
            && captured_acknowledgement == acknowledgement
            && execution_id == &acknowledgement.stage().execution_id
            && stage_id == &acknowledgement.stage().stage_id,
        _ => false,
    }
}

fn role_selectors_match(
    left: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    right: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
) -> bool {
    left.evidence_ref == right.evidence_ref && left.expectation == right.expectation
        && left.designation_ref == right.designation_ref && left.producer == right.producer
}

fn validate_stored_event_link(
    kind: ModelLaneRecordKind,
    record: &SurrealModelLaneRecord,
    scope: &ModelLaneScope,
    rows: &[StoredEventLink],
) -> Result<(), SurrealStorageError> {
    if rows.len() != 1 {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "model-lane authority row lacks one exact-scope EventLedger envelope",
        });
    }
    let event = &rows[0];
    if event.event_id != record.event_id
        || event.aggregate_id != record.aggregate_id
        || event.run_id != record.run_id
        || event.kernel_task_run_id != record.run_id
        || event.session_run_id != declared_event_stream(kind, &record.record_json)?.as_deref().unwrap_or(&record.run_id)
        || event.event_seq != record.event_seq
        || event.event_stream_version != record.event_stream_version
        || event.transaction_seq != record.transaction_seq
        || event.payload_hash != sha256_hex(event.event_payload_json.as_bytes())
    {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "model-lane authority/EventLedger envelope mismatch",
        });
    }
    if matches!(kind, ModelLaneRecordKind::Run | ModelLaneRecordKind::Lane | ModelLaneRecordKind::Message
        | ModelLaneRecordKind::ContextArtifact | ModelLaneRecordKind::ContextHandoff) {
        let payload: Value = serde_json::from_str(&event.event_payload_json).map_err(|_|
            SurrealStorageError::InvalidModelLaneRecord { reason: "canonical source receipt payload is malformed" })?;
        for (field, expected) in [
            ("owner_account_id", scope.owner_account_id.as_str()),
            ("actor_principal_id", scope.actor_principal_id.as_str()),
            ("authenticated_session_id", scope.authenticated_session_id.as_str()),
            ("access_space_id", scope.access_space_id.as_str()),
            ("workspace_id", scope.workspace_id.as_str()),
        ] {
            if payload.get(field).and_then(Value::as_str) != Some(expected) {
                return Err(SurrealStorageError::InvalidModelLaneRecord {
                    reason: "canonical source receipt immutable payload scope mismatch",
                });
            }
        }
        if kind == ModelLaneRecordKind::Run && record.event_stream_version > 1 {
            let canonical: Value = serde_json::from_str(&record.record_json).map_err(|_|
                SurrealStorageError::InvalidModelLaneRecord { reason: "run extension record JSON is malformed" })?;
            if payload.get("schema_id").and_then(Value::as_str) != Some("hsk.model_lane_run_extension@1")
                || payload.get("dexterity_kernel").and_then(Value::as_str) != Some("Dexterity")
                || payload.get("record") != Some(&canonical)
                || !payload.get("run_attachment_lane").is_some_and(Value::is_object)
                || !payload.get("run_attachment_run").is_some_and(Value::is_object)
                || payload.get("predecessor_event_stream_version").and_then(Value::as_i64) != Some(record.event_stream_version - 1) {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "run extension durable merge witness differs" });
            }
        }
        if matches!(kind, ModelLaneRecordKind::ContextArtifact | ModelLaneRecordKind::ContextHandoff) {
            let canonical: Value = serde_json::from_str(&record.record_json).map_err(|_|
                SurrealStorageError::InvalidModelLaneRecord { reason: "canonical context record JSON is malformed" })?;
            let schema = if kind == ModelLaneRecordKind::ContextArtifact {
                "hsk.model_lane_context_bundle_artifact@1"
            } else { "hsk.model_lane_context_bundle_handoff@1" };
            if kind == ModelLaneRecordKind::ContextHandoff
                && payload.get("record").and_then(|record| record.get("context_bundle_hash"))
                    != canonical.get("context_bundle_hash") {
                return Err(SurrealStorageError::InvalidModelLaneRecord {
                    reason: "context_bundle_hash differs from immutable CONTEXT_BUNDLE_RECORDED authority",
                });
            }
            if payload.get("schema_id").and_then(Value::as_str) != Some(schema)
                || payload.get("dexterity_kernel").and_then(Value::as_str) != Some("Dexterity")
                || payload.get("record") != Some(&canonical) {
                return Err(SurrealStorageError::InvalidModelLaneRecord {
                    reason: "canonical context receipt does not contain its exact finalized record",
                });
            }
        }
    }
    Ok(())
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
    validate_identity(
        &commit.execution.execution_id,
        &commit.execution.execution_id,
    )?;
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
    if commit
        .execution
        .record_json
        .get("execution_id")
        .and_then(Value::as_str)
        != Some(commit.execution.execution_id.as_str())
        || commit
            .execution
            .record_json
            .get("run_id")
            .and_then(Value::as_str)
            != Some(commit.execution.run_id.as_str())
        || commit
            .execution
            .record_json
            .get("revision")
            .and_then(Value::as_i64)
            != Some(commit.execution.revision)
        || commit
            .attempt
            .record_json
            .get("stage_id")
            .and_then(Value::as_str)
            != Some(commit.attempt.stage_id.as_str())
        || commit
            .attempt
            .record_json
            .get("attempt")
            .and_then(Value::as_i64)
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
    let object = record_json
        .as_object()
        .ok_or(SurrealStorageError::InvalidModelLaneRecord {
            reason: "routing execution projection must be an object",
        })?;
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
        (
            "model_lane_routing_outbox",
            commit.outbox.command_id.as_str(),
        ),
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
        if message.kind != ModelLaneRecordKind::Message || message.run_id != commit.execution.run_id
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
    let outbox_event =
        take_routing_event(&mut events, "model_lane_routing_outbox", &outbox.command_id)?;
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
        .transpose()?
        .unwrap_or_else(empty_routing_authority_write_bindings);
    let has_binding = binding.is_some();
    let binding = binding
        .map(|write| routing_authority_write_bindings(write, scope))
        .transpose()?
        .unwrap_or_else(empty_routing_authority_write_bindings);
    let message_guard = message_guard
        .map(|guard| routing_message_guard_bindings(guard, scope))
        .unwrap_or_else(empty_routing_message_guard_bindings);
    let source_scope = if has_message || has_binding {
        Some(super::resource_authority::current_routing_original_sources()
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?
            .create_scope().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?)
    } else { None };
    Ok(RoutingCommitBindings {
        routing_source_resource: source_scope.as_ref().map(|scope| scope.resource_id.clone()).unwrap_or_default(),
        routing_source_grant: source_scope.as_ref().and_then(|scope| scope.grant_id.clone()).unwrap_or_default(),
        routing_source_capability: source_scope.as_ref().map(|scope| scope.capability_id.clone()).unwrap_or_default(),
        routing_source_action: source_scope.as_ref().map(|scope| scope.action.as_str().to_owned()).unwrap_or_default(),
        expected_revision,
        has_expected_claim,
        expected_claim,
        run_record_id: stable_record_id(
            ModelLaneRecordKind::Run.as_str(),
            &execution.run_id,
            scope,
        ),
        execution_record_id: stable_record_id("routing_execution", &execution.execution_id, scope),
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
) -> Result<RoutingAuthorityWriteBindings, SurrealStorageError> {
    let event_ledger_stream_id = checked_write_stream(&write)?;
    let event_identity = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        write.kind.as_str(),
        write.aggregate_id,
        write.run_id,
        write.idempotency_key
    );
    Ok(RoutingAuthorityWriteBindings {
        record_id: stable_record_id(write.kind.as_str(), &write.aggregate_id, scope),
        event_id: format!(
            "evt-model-lane-routing-{}",
            stable_record_id("routing_authority_event", &event_identity, scope)
        ),
        record_kind: write.kind.as_str().to_owned(),
        event_ledger_stream_id,
        aggregate_id: write.aggregate_id,
        run_id: write.run_id,
        idempotency_key: write.idempotency_key,
        record_json: write.record_json,
        search_terms: write.search_terms,
        event_payload_hash: sha256_hex(write.event_payload_json.as_bytes()),
        event_payload_json: write.event_payload_json,
    })
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
        source_model_session_term: format!("model_session_id={}", guard.source_model_session_id),
    }
}

fn empty_routing_authority_write_bindings() -> RoutingAuthorityWriteBindings {
    RoutingAuthorityWriteBindings {
        record_id: "unused".to_owned(),
        event_id: "unused".to_owned(),
        record_kind: "unused".to_owned(),
        event_ledger_stream_id: None,
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

fn run_event_bindings(run_id: &str, event_ledger_stream_id: Option<&str>, event_seq: i64, scope: &ModelLaneScope) -> RunEventBindings {
    RunEventBindings {
        run_id: run_id.to_owned(),
        event_ledger_stream_id: event_ledger_stream_id.map(str::to_owned),
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
    let first_row = rows
        .next()
        .ok_or(SurrealStorageError::InvalidModelLaneRecord {
            reason: "model-lane immutable pair returned no first row",
        })?;
    let second_row = rows
        .next()
        .ok_or(SurrealStorageError::InvalidModelLaneRecord {
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

fn checkpoint_record_matches_write(
    row: &SurrealModelLaneRecord,
    write: &SurrealModelLaneWrite,
    scope: &ModelLaneScope,
) -> bool {
    if row.aggregate_id != write.aggregate_id || row.run_id != write.run_id
        || row.idempotency_key != write.idempotency_key
    {
        return false;
    }
    let Ok(mut actual) = serde_json::from_str::<Value>(&row.record_json) else { return false; };
    let Ok(expected) = serde_json::from_str::<Value>(&write.record_json) else { return false; };
    let Some(membership) = actual.as_object_mut().and_then(|record| record.remove("cloud_consent_denial_membership")) else { return false; };
    actual == expected
        && membership.get("schema_version").and_then(Value::as_u64) == Some(1)
        && membership.get("observation").and_then(Value::as_str) == Some("checkpoint_prewrite")
        && membership.get("checkpoint_id").and_then(Value::as_str) == Some(write.aggregate_id.as_str())
        && membership.get("run_id").and_then(Value::as_str) == Some(write.run_id.as_str())
        && membership.get("event_ledger_stream_id") == expected.get("event_ledger_stream_id")
        && membership.get("last_event_ledger_seq") == expected.get("last_event_ledger_seq")
        && membership.get("owner_account_id").and_then(Value::as_str) == Some(scope.owner_account_id.as_str())
        && membership.get("actor_principal_id").and_then(Value::as_str) == Some(scope.actor_principal_id.as_str())
        && membership.get("authenticated_session_id").and_then(Value::as_str) == Some(scope.authenticated_session_id.as_str())
        && membership.get("access_space_id").and_then(Value::as_str) == Some(scope.access_space_id.as_str())
        && membership.get("workspace_id").and_then(Value::as_str) == Some(scope.workspace_id.as_str())
        && membership.get("receipts").and_then(Value::as_array).is_some()
}

fn record_matches_write(row: &SurrealModelLaneRecord, write: &SurrealModelLaneWrite) -> bool {
    row.aggregate_id == write.aggregate_id
        && row.run_id == write.run_id
        && row.idempotency_key == write.idempotency_key
        && row.record_json == write.record_json
}

fn declared_event_stream(kind: ModelLaneRecordKind, record_json: &str) -> Result<Option<String>, SurrealStorageError> {
    if kind == ModelLaneRecordKind::SessionCleanupReceipt || kind == ModelLaneRecordKind::RoutingExecution {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(record_json).map_err(|_| SurrealStorageError::InvalidModelLaneRecord { reason: "model-lane canonical record JSON is malformed" })?;
    let record = if kind == ModelLaneRecordKind::Message { value.get("inner") } else { Some(&value) };
    let stream = record.and_then(|record| record.get("event_ledger_stream_id")).and_then(Value::as_str).filter(|stream| !stream.trim().is_empty()).ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "model-lane canonical record lacks its declared EventLedger stream" })?;
    Ok(Some(stream.to_owned()))
}

fn checked_write_stream(write: &SurrealModelLaneWrite) -> Result<Option<String>, SurrealStorageError> {
    let stream = declared_event_stream(write.kind, &write.record_json)?;
    if stream != write.event_ledger_stream_id { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "model-lane write stream differs from its canonical record" }); }
    Ok(stream)
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
            row.document_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
                && row.crdt_document_id.as_deref() == Some(row.scope_id.as_str())
        }
        "source_root" | "index_run" => !row.scope_id.trim().is_empty()
            && row.document_id.is_none() && row.crdt_document_id.is_none(),
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
            (match (
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
            }) && proposal.promotion_requested_event_id.is_none()
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
    receipts: Vec<CrdtProposalSemanticReceiptRow>,
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
        if receipt.semantic.schema_id != "hsk.model_lane_navigation_receipt_input@1"
            || receipt.event_version != "kernel_event_v1"
            || receipt.aggregate_type != expected_aggregate_type
            || receipt.aggregate_id != proposal.proposal_id
            || receipt.source_component != expected_source_component
            || receipt.correlation_id.as_deref() != Some(proposal.correlation_id.as_str())
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
        || !semantic_matches(recorded, "proposal_id", &proposal.proposal_id)
        || !semantic_matches(recorded, "diff_sha256", &proposal.diff_sha256)
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
            || decided.actor_kind != "operator"
            || !semantic_matches(decided, "document_id", &proposal.document_id)
            || !semantic_matches(decided, "crdt_document_id", &proposal.crdt_document_id)
            || !semantic_matches(decided, "recorded_event_id", &proposal.recorded_event_id)
            || !semantic_matches(decided, "reviewer_principal_id", &proposal.actor_principal_id)
            || !semantic_matches(decided, "reviewer_authenticated_session_id", &decided.session_run_id)
            || !semantic_matches(decided, "reviewer_action", "update")
            || !semantic_matches(decided, "reviewer_capability_id", "fs.write")
            || ["reviewer_resource_id", "reviewer_grant_id"].iter().any(|field|
                decided.semantic.string(field).is_none_or(|value| value.trim().is_empty()))
            || proposal.decided_by.as_deref() != Some(decided.actor_id.as_str())
            || !semantic_matches(decided, "proposal_id", &proposal.proposal_id)
            || !semantic_matches(decided, "review_state", expected_state)
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
            || !semantic_matches(applied, "proposal_id", &proposal.proposal_id)
            || !semantic_matches(
                applied,
                "applied_update_id",
                proposal.applied_update_id.as_deref().ok_or(
                    SurrealStorageError::InvalidModelLaneRecord {
                        reason: "CRDT proposal applied receipt lacks update identity",
                    },
                )?,
            )
            || !semantic_matches(applied, "applied_update_sha256", &proposal.diff_sha256)
            || !semantic_matches(applied, "approved_diff_sha256", &proposal.diff_sha256)
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
            || !semantic_matches(requested, "proposal_id", &proposal.proposal_id)
            || !semantic_matches(requested, "diff_sha256", &proposal.diff_sha256)
            || requested.semantic.base_update_seq
                != Some(proposal.base_update_seq)
            || !semantic_matches(requested, "base_state_vector", &proposal.base_state_vector)
            || !semantic_matches(requested, "decided_by", &requested.actor_id)
            || !semantic_matches(accepted, "proposal_id", &proposal.proposal_id)
            || !semantic_matches(accepted, "review_state", "promoted")
            || !semantic_matches(accepted, "decided_by", &accepted.actor_id)
            || !semantic_matches(accepted, "diff_sha256", &proposal.diff_sha256)
            || accepted.semantic.applied_update_id.as_deref()
                != proposal.applied_update_id.as_deref()
            || accepted.semantic.applied_update_sha256.as_deref()
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
    receipts: &'a BTreeMap<String, CrdtProposalSemanticReceiptRow>,
    event_id: &str,
) -> Result<&'a CrdtProposalSemanticReceiptRow, SurrealStorageError> {
    receipts
        .get(event_id)
        .ok_or(SurrealStorageError::InvalidModelLaneRecord {
            reason: "CRDT proposal canonical EventLedger receipt is unavailable in exact scope",
        })
}

fn semantic_matches(receipt: &CrdtProposalSemanticReceiptRow, field: &str, expected: &str) -> bool {
    receipt.semantic.string(field) == Some(expected)
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

    fn scoped_identity(
        mut event: SurrealModelLaneRoutingEventWrite,
        scope: &ModelLaneScope,
    ) -> (String, String) {
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
        assert!(ROUTING_COMMIT_QUERY
            .contains("event_ledger_event_id.aggregate_type = 'model_lane_routing_stage_attempt'"));
        assert!(ROUTING_COMMIT_QUERY.contains("event_ledger_event_id.aggregate_id = attempt_id"));
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

const DESIGNATION_INSERT_QUERY: &str = "LET $existing = (SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
                 IF array::len($existing) > 0 { RETURN $existing; } ELSE {\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id, session_run_id: IF $event_ledger_stream_id = NONE { $run_id } ELSE { $event_ledger_stream_id }, aggregate_type: $record_kind, aggregate_id: $aggregate_id, idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $event_payload_hash, source_component: 'model_lane', payload: { record_kind: $record_kind, run_id: $run_id, event_stream_version: 1, event_payload_json: $event_payload_json }, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, wsids: [$workspace_id], owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                   LET $seq = $ledger[0].event_sequence;\
                   RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT { record_kind: $record_kind, aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key, record_json: $record_json, search_terms: $search_terms, event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                 };";

fn role_storage_scope(scope: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution) -> ModelLaneScope {
    ModelLaneScope { owner_account_id: scope.owner_account_id.to_string(), actor_principal_id: scope.actor_principal_id.to_string(), authenticated_session_id: scope.authenticated_session_id.to_string(), access_space_id: scope.access_space_id.to_string(), workspace_id: scope.workspace_id.as_str().to_owned() }
}

// RAM-only provider wire preparation. No authority is minted by these values.
struct ValidatorAssociationEnvelope {
    record_json: String,
    event_payload_json: String,
    outer_json: String,
    event_payload_sha256: String,
}

fn validator_association_json<T: serde::Serialize + ?Sized>(
    value: &T,
) -> Result<String, SurrealStorageError> {
    fn ordered(value: Value) -> Value {
        match value {
            Value::Array(values) => Value::Array(values.into_iter().map(ordered).collect()),
            Value::Object(values) => {
                let sorted: BTreeMap<_, _> = values.into_iter()
                    .map(|(key, value)| (key, ordered(value))).collect();
                Value::Object(sorted.into_iter().collect())
            }
            value => value,
        }
    }
    let value = serde_json::to_value(value).map_err(|_| {
        SurrealStorageError::InvalidModelLaneRecord {
            reason: "producer association cannot be encoded",
        }
    })?;
    serde_json::to_string(&ordered(value)).map_err(|_| {
        SurrealStorageError::InvalidModelLaneRecord {
            reason: "producer association cannot be encoded",
        }
    })
}

fn validator_association_envelope(
    body: &handshake_producer_association::AssociationBodyV1,
    original_stream: &str,
) -> Result<ValidatorAssociationEnvelope, SurrealStorageError> {
    use handshake_producer_association::{
        AssociationEventPayloadV1, AssociationOuterV1, AssociationRecordV1,
    };
    let record = AssociationRecordV1 {
        run_id: body.run.run_id.clone(),
        event_ledger_stream_id: original_stream.to_owned(),
        association: body.clone(),
    };
    let issuer = &body.original_issuer;
    let payload = AssociationEventPayloadV1 {
        dexterity_kernel: "Dexterity".to_owned(),
        owner_account_id: issuer.owner_account_id.clone(),
        actor_principal_id: issuer.actor_principal_id.clone(),
        authenticated_session_id: issuer.authenticated_session_id.clone(),
        access_space_id: issuer.access_space_id.clone(),
        workspace_id: issuer.workspace_id.clone(),
        record,
        original_operation_id: body.original_operation_id,
        request_id: body.request_id.clone(),
        causation_ref: body.causation_ref.clone(),
        trace_id: body.run.trace_id.clone(),
    };
    let record_json = validator_association_json(&payload.record)?;
    let event_payload_json = validator_association_json(&payload)?;
    let event_payload_sha256 = sha256_hex(event_payload_json.as_bytes());
    let outer_json = validator_association_json(&AssociationOuterV1 {
        record_kind: "producer_association".to_owned(),
        run_id: body.run.run_id.clone(),
        event_stream_version: 1,
        event_payload_json: event_payload_json.clone(),
        issuance_operation_id: body.original_operation_id,
    })?;
    Ok(ValidatorAssociationEnvelope {
        record_json, event_payload_json, outer_json, event_payload_sha256,
    })
}

#[derive(Clone, SurrealValue)]
struct ValidatorAssociationObserved {
    record_id: String,
    record_kind: String,
    aggregate_id: String,
    run_id: String,
    idempotency_key: String,
    record_json: String,
    event_id: String,
    event_seq: i64,
    transaction_seq: i64,
    event_stream_version: i64,
    ledger_record_id: String,
    ledger_event_id: String,
    ledger_event_sequence: i64,
    ledger_source_component: String,
    ledger_aggregate_type: String,
    ledger_aggregate_id: String,
    ledger_event_type: String,
    ledger_event_version: String,
    ledger_payload_sha256: String,
    ledger_run_id: String,
    ledger_stream_id: String,
    outer_json: String,
}

impl ValidatorAssociationObserved {
    fn decode(
        &self,
        body: &handshake_producer_association::AssociationBodyV1,
        original_stream: &str,
        expected_record_id: &str,
        expected_event_id: &str,
        expected: &handshake_producer_association::ExpectedAssociationInventoryV1,
    ) -> Result<handshake_producer_association::CheckedProducerAssociation, SurrealStorageError> {
        let denied = || SurrealStorageError::InvalidModelLaneRecord {
            reason: "producer association canonical identity or event link differs",
        };
        if self.record_kind != "producer_association"
            || self.aggregate_id != body.original_operation_id.to_string()
            || self.idempotency_key != body.idempotency_key
            || self.run_id != body.run.run_id
            || self.ledger_run_id != body.run.run_id
            || self.ledger_stream_id != original_stream
            || self.event_id != expected_event_id
        { return Err(denied()); }
        let links = handshake_producer_association::AssociationLinksV1 {
            record_id: self.record_id.clone(),
            row_event_id: self.event_id.clone(),
            row_event_seq: self.event_seq,
            row_transaction_seq: self.transaction_seq,
            row_event_stream_version: u32::try_from(self.event_stream_version).map_err(|_| denied())?,
            ledger_event_id: self.ledger_event_id.clone(),
            ledger_event_sequence: self.ledger_event_sequence,
            ledger_record_id: self.ledger_record_id.clone(),
            ledger_source_component: self.ledger_source_component.clone(),
            ledger_aggregate_type: self.ledger_aggregate_type.clone(),
            ledger_aggregate_id: self.ledger_aggregate_id.clone(),
            ledger_event_type: self.ledger_event_type.clone(),
            ledger_event_version: self.ledger_event_version.clone(),
            ledger_payload_sha256: self.ledger_payload_sha256.clone(),
        };
        // This serializes the complete protected native outer observation.
        // No field projection is substituted for the codec's full outer input.
        let native_outer: Value = serde_json::from_str(&self.outer_json).map_err(|_| denied())?;
        let outer_json = validator_association_json(&native_outer)?;
        let checked = handshake_producer_association::decode_checked(
            &self.record_json, &outer_json, expected_record_id, expected, &links,
        ).map_err(|_| denied())?;
        let encoded = handshake_producer_association::encode_checked(
            body.clone(), original_stream, expected_record_id, expected, &links,
        ).map_err(|_| denied())?;
        if checked.body() != body
            || checked.record_json() != encoded.record_json()
            || checked.outer_json() != encoded.outer_json()
            || checked.event_payload_json() != encoded.event_payload_json()
            || checked.event_payload_sha256() != encoded.event_payload_sha256()
        { return Err(denied()); }
        Ok(checked)
    }
}

pub(crate) struct ValidatorAssociationCommitReadback {
    operation_id: uuid::Uuid,
    scope: ModelLaneScope,
    record_id: String,
    event_id: String,
    record_json: String,
    outer_json: String,
    observed: super::resource_authority::RetainedValidatorAssociationQuery<ValidatorAssociationObserved>,
}

fn validator_association_rows<R: Clone>(
    retained: &super::resource_authority::RetainedValidatorAssociationQuery<R>,
    operation_id: uuid::Uuid,
) -> Result<Vec<R>, SurrealStorageError> {
    retained.with_observation(|pre, query, post| match (pre, query, post) {
        (Some(Ok(())), Some(Ok(rows)), Some(Ok(()))) => Ok(rows.clone()),
        _ => Err(SurrealStorageError::TransactionSettlementPending { operation_id }),
    })
}

fn validator_association_bindings(
    body: &handshake_producer_association::AssociationBodyV1,
    scope: &ModelLaneScope,
    original_stream: &str,
    record_id: &str,
    event_id: &str,
    envelope: &ValidatorAssociationEnvelope,
) -> surrealdb::types::Object {
    let mut bindings = surrealdb::types::Object::new();
    bindings.insert("record_id", record_id.to_owned());
    bindings.insert("event_id", event_id.to_owned());
    bindings.insert("aggregate_id", body.original_operation_id.to_string());
    bindings.insert("run_id", body.run.run_id.clone());
    bindings.insert("event_stream_id", original_stream.to_owned());
    bindings.insert("idempotency_key", body.idempotency_key.clone());
    bindings.insert("record_json", envelope.record_json.clone());
    bindings.insert("outer_json", envelope.outer_json.clone());
    bindings.insert("payload_hash", envelope.event_payload_sha256.clone());
    bindings.insert("owner_account_id", scope.owner_account_id.clone());
    bindings.insert("actor_principal_id", scope.actor_principal_id.clone());
    bindings.insert("authenticated_session_id", scope.authenticated_session_id.clone());
    bindings.insert("access_space_id", scope.access_space_id.clone());
    bindings.insert("workspace_id", scope.workspace_id.clone());
    bindings
}

fn validator_association_scope_matches(
    body: &handshake_producer_association::AssociationBodyV1,
    scope: &ModelLaneScope,
    operation_id: uuid::Uuid,
) -> bool {
    let original = &body.original_issuer;
    !operation_id.is_nil() && body.original_operation_id == operation_id
        && original.owner_account_id == scope.owner_account_id
        && original.actor_principal_id == scope.actor_principal_id
        && original.authenticated_session_id == scope.authenticated_session_id
        && original.access_space_id == scope.access_space_id
        && original.workspace_id == scope.workspace_id
}

impl SurrealModelLaneStore {
    pub(crate) async fn put_validator_association(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        issuer: &super::resource_authority::CurrentValidatorAssociationSeal<'_>,
    ) -> Result<ValidatorAssociationCommitReadback, SurrealStorageError> {
        let operation_id = issuer.operation_id();
        let scope = role_storage_scope(issuer.original_scope());
        if !issuer.matches_transaction(transaction)
            || !super::PROTECTED_TRANSACTION.try_with(|current| std::sync::Arc::ptr_eq(&current.storage.inner, &self.storage.inner)).unwrap_or(false)
            || std::time::Instant::now() >= issuer.original_deadline()
            || !validator_association_scope_matches(issuer.body(), &scope, operation_id)
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        validate_scope(&scope)?;
        validate_identity(&operation_id.to_string(), &issuer.body().idempotency_key)?;
        let record_id = stable_record_id("producer_association", &operation_id.to_string(), &scope);
        let event_id = issuer.association_event_id();
        if event_id.is_empty() { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let envelope = validator_association_envelope(issuer.body(), issuer.original_stream())?;
        let inserted = issuer.query_values_at::<StoredRow, _>(
            VALIDATOR_ASSOCIATION_INSERT_QUERY,
            validator_association_bindings(
                issuer.body(), &scope, issuer.original_stream(), &record_id, event_id, &envelope,
            ), 1,
        ).await;
        if !issuer.matches_observation(&inserted) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let rows = validator_association_rows(&inserted, operation_id)?;
        let [row] = rows.as_slice() else {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "producer association write returned no unique original row",
            });
        };
        if row.aggregate_id != operation_id.to_string()
            || row.run_id != issuer.body().run.run_id
            || row.idempotency_key != issuer.body().idempotency_key
            || row.record_json != envelope.record_json
            || row.event_id != event_id
        {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "producer association immutable retry conflicts",
            });
        }
        let observed = issuer.query_values_at::<ValidatorAssociationObserved, _>(
            VALIDATOR_ASSOCIATION_OBSERVE_QUERY,
            validator_association_bindings(
                issuer.body(), &scope, issuer.original_stream(), &record_id, event_id, &envelope,
            ), 3,
        ).await;
        if !issuer.matches_observation(&observed) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let rows = validator_association_rows(&observed, operation_id)?;
        let [row] = rows.as_slice() else {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "producer association observation is missing or ambiguous",
            });
        };
        let checked = row.decode(issuer.body(), issuer.original_stream(), &record_id,
            event_id, issuer.expected_inventory())?;
        if checked.record_json() != envelope.record_json
            || checked.outer_json() != envelope.outer_json
            || checked.event_payload_json() != envelope.event_payload_json
            || checked.event_payload_sha256() != envelope.event_payload_sha256
            || std::time::Instant::now() >= issuer.original_deadline()
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        Ok(ValidatorAssociationCommitReadback {
            operation_id, scope, record_id, event_id: event_id.to_owned(),
            record_json: checked.record_json().to_owned(), outer_json: checked.outer_json().to_owned(),
            observed,
        })
    }

    pub(crate) async fn readback_validator_association(
        &self,
        original: &super::resource_authority::ValidatorAssociationReadbackGuard<'_>,
        expected: &ValidatorAssociationCommitReadback,
    ) -> Result<handshake_producer_association::CheckedProducerAssociation, SurrealStorageError> {
        let operation_id = original.operation_id();
        let scope = role_storage_scope(original.original_scope());
        if !original.matches_transaction(original.transaction())
            || !super::PROTECTED_TRANSACTION.try_with(|current| std::sync::Arc::ptr_eq(&current.storage.inner, &self.storage.inner)).unwrap_or(false)
            || !original.matches_observation(&expected.observed)
            || operation_id != expected.operation_id || scope != expected.scope
            || original.association_event_id() != expected.event_id
            || std::time::Instant::now() >= original.original_deadline()
            || !validator_association_scope_matches(original.body(), &scope, operation_id)
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let record_id = stable_record_id("producer_association", &operation_id.to_string(), &scope);
        if record_id != expected.record_id { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        let envelope = validator_association_envelope(original.body(), original.original_stream())?;
        if envelope.record_json != expected.record_json || envelope.outer_json != expected.outer_json {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let observed = original.query_values_at::<ValidatorAssociationObserved, _>(
            VALIDATOR_ASSOCIATION_OBSERVE_QUERY,
            validator_association_bindings(
                original.body(), &scope, original.original_stream(),
                &record_id, original.association_event_id(), &envelope,
            ), 3,
        ).await;
        if !original.matches_observation(&observed) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let rows = validator_association_rows(&observed, operation_id)?;
        let [row] = rows.as_slice() else {
            return Err(SurrealStorageError::InvalidModelLaneRecord {
                reason: "producer association readback is missing or ambiguous",
            });
        };
        let checked = row.decode(original.body(), original.original_stream(), &record_id,
            original.association_event_id(), original.expected_inventory())?;
        if checked.record_json() != expected.record_json
            || checked.outer_json() != expected.outer_json
            || std::time::Instant::now() >= original.original_deadline()
        { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        Ok(checked)
    }
}

const VALIDATOR_ASSOCIATION_INSERT_QUERY: &str = r#"
LET $existing = SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq FROM type::record('model_lane_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
IF array::len($existing) > 0 {
    RETURN $existing;
} ELSE {
    LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
        event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id,
        session_run_id: $event_stream_id, aggregate_type: 'producer_association',
        aggregate_id: $aggregate_id, idempotency_key: $event_id,
        event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal',
        actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE,
        payload_hash: $payload_hash, source_component: 'model_lane',
        payload: encoding::json::decode($outer_json),
        authority_resource_id: type::record('protected_resources', $wp012_original_resource),
        authority_grant_id: type::record('resource_grants', $wp012_original_grant),
        authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability,
        authority_action: $wp012_original_action, wsids: [$workspace_id],
        owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id,
        workspace_id: $workspace_id
    };
    IF array::len($ledger) != 1 { THROW 'producer association event is not unique'; };
    LET $seq = $ledger[0].event_sequence;
    RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT {
        record_kind: 'producer_association', aggregate_id: $aggregate_id, run_id: $run_id,
        idempotency_key: $idempotency_key, record_json: $record_json, search_terms: [],
        event_id: $event_id, event_ledger_event_id: type::record('kernel_event_ledger', $event_id),
        event_seq: $seq, event_stream_version: 1, transaction_seq: $seq,
        owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id,
        workspace_id: $workspace_id
    };
};
"#;

// Schema-owner compatibility of this existing reverse relation remains unresolved.
// No new native field or copied expected key is used as an observation.
const VALIDATOR_ASSOCIATION_OBSERVE_QUERY: &str = r#"
LET $ledger = SELECT * FROM ONLY type::record('kernel_event_ledger', $event_id);
LET $reverse = SELECT id FROM model_lane_authority WHERE event_ledger_event_id = $ledger.id AND record_kind = 'producer_association' AND aggregate_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;
IF array::len($reverse) != 1 { THROW 'producer association ledger reverse link is not unique'; };
SELECT record::id(id) AS record_id, record_kind, aggregate_id, run_id, idempotency_key, record_json,
    event_id, event_seq, transaction_seq, event_stream_version,
    record::id($reverse[0].id) AS ledger_record_id,
    event_ledger_event_id.event_id AS ledger_event_id,
    event_ledger_event_id.event_sequence AS ledger_event_sequence,
    event_ledger_event_id.source_component AS ledger_source_component,
    event_ledger_event_id.aggregate_type AS ledger_aggregate_type,
    event_ledger_event_id.aggregate_id AS ledger_aggregate_id,
    event_ledger_event_id.event_type AS ledger_event_type,
    event_ledger_event_id.event_version AS ledger_event_version,
    event_ledger_event_id.payload_hash AS ledger_payload_sha256,
    event_ledger_event_id.kernel_task_run_id AS ledger_run_id,
    event_ledger_event_id.session_run_id AS ledger_stream_id,
    encoding::json::encode(event_ledger_event_id.payload) AS outer_json
FROM type::record('model_lane_authority', $record_id)
WHERE id = $reverse[0].id AND event_ledger_event_id = $ledger.id
    AND event_id = $event_id AND record_kind = 'producer_association' AND aggregate_id = $aggregate_id
    AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id AND event_ledger_event_id.owner_account_id = $owner_account_id
    AND event_ledger_event_id.actor_principal_id = $actor_principal_id
    AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
    AND event_ledger_event_id.access_space_id = $access_space_id
    AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 2;
"#;

pub(super) fn validator_association_native_binding_data(
    body: &handshake_producer_association::AssociationBodyV1,
    original: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    stream: &str,
) -> Result<(String, String, String), SurrealStorageError> {
    let scope = role_storage_scope(original);
    if !validator_association_scope_matches(body, &scope, body.original_operation_id) {
        return Err(SurrealStorageError::TransactionAuthorityRequired);
    }
    validate_scope(&scope)?;
    validate_identity(&body.original_operation_id.to_string(), &body.idempotency_key)?;
    let record_id = stable_record_id(
        "producer_association", &body.original_operation_id.to_string(), &scope,
    );
    let envelope = validator_association_envelope(body, stream)?;
    Ok((record_id, envelope.record_json, envelope.outer_json))
}


#[derive(SurrealValue)]
struct ProfileAdmissionStateBindings {
    state_record: String,
    origin_record: String,
    expected_origin_json: String,
    expected_generation: i64,
    expected_status: String,
    next_status: String,
    transition: WriteBindings,
}

impl SurrealModelLaneStore {
    pub(crate) async fn stage_profile_admission_atomic(
        &self,
        writes: [SurrealModelLaneWrite; 3],
        scope: &ModelLaneScope,
        operation_id: uuid::Uuid,
        original_owner: std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> Result<Vec<SurrealModelLaneRecord>, SurrealStorageError> {
        validate_scope(scope)?;
        if writes[0].kind != ModelLaneRecordKind::Run
            || writes[1].kind != ModelLaneRecordKind::LaunchAdmission
            || writes[2].kind != ModelLaneRecordKind::SelectionAudit
            || writes.iter().any(|write| write.run_id != writes[0].run_id)
        { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission requires its same-run origin/admission/audit triplet" }); }
        let expected = writes.clone();
        let mut bindings = Vec::new();
        for write in writes {
            validate_identity(&write.aggregate_id, &write.idempotency_key)?;
            checked_write_stream(&write)?;
            bindings.push(WriteBindings {
                record_id: stable_record_id(write.kind.as_str(), &write.aggregate_id, scope),
                event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
                record_kind: write.kind.as_str().to_owned(), aggregate_id: write.aggregate_id,
                run_id: write.run_id, event_ledger_stream_id: write.event_ledger_stream_id,
                idempotency_key: write.idempotency_key, record_json: write.record_json,
                search_terms: write.search_terms,
                event_payload_hash: sha256_hex(write.event_payload_json.as_bytes()),
                event_payload_json: write.event_payload_json,
                owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
                authenticated_session_id: scope.authenticated_session_id.clone(),
                access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone(),
            });
        }
        let owner_for_proof = original_owner.clone();
        let origin_record = bindings[1].record_id.clone();
        let storage = self.storage.clone();
        let outcome = self.storage.with_original_runcreate_transaction(
            operation_id, original_owner, move |transaction| Box::pin(async move {
                let mut result = Vec::new();
                for binding in bindings {
                    let rows = transaction.query_values_at::<StoredRow, _>(PROFILE_ADMISSION_ORIGIN_PIECE, binding, 1).await?;
                    let [row] = rows.as_slice() else { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission triplet piece is absent or duplicated" }); };
                    let index = result.len();
                    if row.aggregate_id != expected[index].aggregate_id || row.run_id != expected[index].run_id
                        || row.idempotency_key != expected[index].idempotency_key || row.record_json != expected[index].record_json
                    { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission immutable origin or idempotency conflict" }); }
                    result.push(SurrealModelLaneRecord::from(rows.into_iter().next().expect("one row checked")));
                }
                transaction.query_values_at::<StoredRow, _>(
                    "LET $origin = SELECT * FROM type::record('model_lane_authority', $record_id); IF array::len($origin) != 1 { THROW 'PROFILE_ADMISSION_ORIGIN_MISSING'; }; LET $existing = SELECT * FROM type::record('model_lane_profile_admission_state', $record_id); IF array::len($existing) = 0 { CREATE type::record('model_lane_profile_admission_state', $record_id) CONTENT { origin_id: type::record('model_lane_authority', $record_id), generation: 1, status: 'staged', receipt_id: $origin[0].event_ledger_event_id }; }; RETURN [];",
                    ProfileAdmissionRecordBinding { record_id: origin_record }, 4,
                ).await?;
                Ok::<_, SurrealStorageError>(result)
            }),
        ).await;
        let (result, proof) = outcome.into_parts();
        let proof = proof.ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission transaction finalization remains unproven" })?;
        if !proof.matches_original(&storage, operation_id, &owner_for_proof)
            || !matches!((result.is_ok(), proof.finalization()),
                (true, super::OriginalRunCreateFinalization::Commit)
                    | (false, super::OriginalRunCreateFinalization::Rollback))
        { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission lacks its original outcome-matched finalization proof" }); }
        storage.acknowledge_profile_admission_finalization(proof, &owner_for_proof)?;
        result
    }

    pub(crate) async fn transition_profile_admission_atomic(
        &self,
        scope: &ModelLaneScope,
        origin_id: &str,
        expected_origin_json: String,
        expected_generation: i64,
        expected_status: &str,
        next_status: &str,
        transition: SurrealModelLaneWrite,
        operation_id: uuid::Uuid,
        original_owner: std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> Result<(), SurrealStorageError> {
        validate_scope(scope)?;
        if transition.kind != ModelLaneRecordKind::LaunchAdmissionTransition || expected_generation < 1 || expected_generation.checked_add(1).is_none() {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission transition identity is invalid" });
        }
        checked_write_stream(&transition)?;
        let state_record = stable_record_id("launch_admission", origin_id, scope);
        let binding = ProfileAdmissionStateBindings {
            origin_record: state_record.clone(), state_record, expected_origin_json,
            expected_generation, expected_status: expected_status.to_owned(), next_status: next_status.to_owned(),
            transition: WriteBindings {
                record_id: stable_record_id(transition.kind.as_str(), &transition.aggregate_id, scope),
                event_id: format!("evt-model-lane-{}", uuid::Uuid::now_v7()),
                record_kind: transition.kind.as_str().to_owned(), aggregate_id: transition.aggregate_id,
                run_id: transition.run_id, event_ledger_stream_id: transition.event_ledger_stream_id,
                idempotency_key: transition.idempotency_key, record_json: transition.record_json,
                search_terms: transition.search_terms,
                event_payload_hash: sha256_hex(transition.event_payload_json.as_bytes()), event_payload_json: transition.event_payload_json,
                owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
                authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
                workspace_id: scope.workspace_id.clone(),
            },
        };
        let owner_for_proof = original_owner.clone();
        let storage = self.storage.clone();
        let outcome = self.storage.with_original_runcreate_transaction(operation_id, original_owner, move |transaction| Box::pin(async move {
            let rows = transaction.query_values_at::<ProfileAdmissionGenerationRow, _>(PROFILE_ADMISSION_TRANSITION, binding, 4).await?;
            if rows.len() != 1 || rows[0].generation != expected_generation + 1 {
                return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission transition CAS was not confirmed" });
            }
            Ok::<_, SurrealStorageError>(())
        })).await;
        let (result, proof) = outcome.into_parts();
        let proof = proof.ok_or(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission transition finalization remains unproven" })?;
        if !proof.matches_original(&storage, operation_id, &owner_for_proof)
            || !matches!((result.is_ok(), proof.finalization()),
                (true, super::OriginalRunCreateFinalization::Commit)
                    | (false, super::OriginalRunCreateFinalization::Rollback))
        { return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "profile admission transition lacks its original outcome-matched finalization proof" }); }
        storage.acknowledge_profile_admission_finalization(proof, &owner_for_proof)?;
        result
    }
}

#[derive(SurrealValue)]
struct ProfileAdmissionRecordBinding { record_id: String }
#[derive(SurrealValue)]
struct ProfileAdmissionGenerationRow { generation: i64 }

const PROFILE_ADMISSION_ORIGIN_PIECE: &str = r#"
LET $existing = SELECT aggregate_id, run_id, idempotency_key, record_json, event_id, event_seq, event_stream_version, transaction_seq
    FROM type::record('model_lane_authority', $record_id);
IF array::len($existing) > 0 { RETURN $existing; } ELSE {
    LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
        event_id: $event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $run_id,
        session_run_id: $event_ledger_stream_id, aggregate_type: $record_kind, aggregate_id: $aggregate_id,
        idempotency_key: $event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id,
        payload_hash: $event_payload_hash, source_component: 'model_lane',
        payload: { record_kind: $record_kind, run_id: $run_id, event_stream_version: 1, event_payload_json: $event_payload_json },
        authority_resource_id: type::record('protected_resources', $wp012_original_resource),
        authority_grant_id: type::record('resource_grants', $wp012_original_grant),
        owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id
    };
    LET $seq = $ledger[0].event_sequence;
    RETURN CREATE type::record('model_lane_authority', $record_id) CONTENT {
        record_kind: $record_kind, aggregate_id: $aggregate_id, run_id: $run_id, idempotency_key: $idempotency_key,
        record_json: $record_json, search_terms: $search_terms, event_id: $event_id,
        event_ledger_event_id: type::record('kernel_event_ledger', $event_id), event_seq: $seq, event_stream_version: 1, transaction_seq: $seq,
        owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id
    };
};
"#;

const PROFILE_ADMISSION_TRANSITION: &str = r#"
LET $current = SELECT * FROM type::record('model_lane_profile_admission_state', $state_record);
LET $origin = SELECT record_json FROM type::record('model_lane_authority', $origin_record);
IF array::len($current) != 1 OR array::len($origin) != 1 OR $origin[0].record_json != $expected_origin_json
    OR $current[0].generation != $expected_generation OR $current[0].status != $expected_status {
    THROW 'PROFILE_ADMISSION_CAS_CONFLICT';
};
LET $ledger = CREATE type::record('kernel_event_ledger', $transition.event_id) CONTENT {
    event_id: $transition.event_id, event_version: 'kernel_event_v1', kernel_task_run_id: $transition.run_id,
    session_run_id: $transition.event_ledger_stream_id, aggregate_type: $transition.record_kind, aggregate_id: $transition.aggregate_id,
    idempotency_key: $transition.event_id, event_type: 'MODEL_LANE_AUTHORITY_RECORDED', actor_kind: 'principal', actor_id: $transition.actor_principal_id,
    payload_hash: $transition.event_payload_hash, source_component: 'model_lane',
    payload: { record_kind: $transition.record_kind, run_id: $transition.run_id, event_stream_version: 1, event_payload_json: $transition.event_payload_json },
    authority_resource_id: type::record('protected_resources', $wp012_original_resource),
    authority_grant_id: type::record('resource_grants', $wp012_original_grant),
    owner_account_id: $transition.owner_account_id, actor_principal_id: $transition.actor_principal_id,
    authenticated_session_id: $transition.authenticated_session_id, access_space_id: $transition.access_space_id, workspace_id: $transition.workspace_id
};
UPDATE type::record('model_lane_profile_admission_state', $state_record)
    SET generation = $expected_generation + 1, status = $next_status, receipt_id = type::record('kernel_event_ledger', $transition.event_id)
    WHERE generation = $expected_generation AND status = $expected_status RETURN generation;
"#;
