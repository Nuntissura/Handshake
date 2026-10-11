//! WP-1 MT-018: the ONE canonical, reusable admissible-CRDT posture builder for
//! ModelLane proofs, shared by every `tests/*` binary that needs a CRDT-bearing
//! `NewModelLaneMessage` that PASSES `validate_message_crdt_authority_tx`.
//!
//! Why this module exists. Building an admissible CRDT posture by hand is easy
//! to get subtly wrong, and every hand-rolled variant drifts:
//!   * the seeded update's `session_id` must equal the source lane's
//!     `session_id` (`validate_crdt_lane_session_uniqueness_tx`),
//!   * the knowledge-agent lease's `correlation_id` must equal the update's
//!     `trace_id` (`trace-{update_id}`) and its actor/session/scope must match
//!     (`resolve_active_crdt_actor_lane_lease_tx`),
//!   * the message must link that trace in `linked_span_contexts`
//!     (`bind_crdt_authority_to_lane`), and
//!   * for a Proposal-kind message, `crdt_proposal_ref` must resolve to an
//!     APPROVED proposal whose workspace/document/crdt_document/actor/
//!     actor_kind/session/correlation all equal the resolved update's, whose
//!     `applied_update_id` equals the resolved `update_id`, and whose
//!     `applied_update_sha256` equals its own `diff_sha256`.
//!
//! Everything here is durable PostgreSQL/EventLedger authority created through
//! the REAL product APIs (`push_yjs_update`, `append_kernel_crdt_snapshot`,
//! `claim_lease`, `record_ai_edit_proposal` -> `decide_ai_edit_proposal` ->
//! `apply_approved_ai_edit`). No raw `INSERT` mints authority in this module, so
//! a proof built on it cannot be a scaffold.

#![allow(dead_code)]

use base64::Engine;
use handshake_core::kernel::crdt::actor_site::{
    derive_knowledge_site_id, knowledge_crdt_identity, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::agent_lease::{
    claim_lease, release_lease, KnowledgeLeaseScopeKind, LeaseClaimOutcomeV1, LeaseClaimRequestV1,
};
use handshake_core::kernel::crdt::ai_edit_proposal::{
    apply_approved_ai_edit, decide_ai_edit_proposal, record_ai_edit_proposal, AiEditApplyOutcomeV1,
    AiEditProposalRequestV1, RecordAiEditProposalOutcomeV1,
};
use handshake_core::kernel::crdt::snapshot::{new_crdt_snapshot_record, CrdtSnapshotRecordInputV1};
use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
use handshake_core::kernel::crdt::yjs_bridge::{
    push_yjs_update, YjsPushOutcomeV1, YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1,
    YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::kernel::{KernelEventType, NewKernelEvent};
use handshake_core::storage::Database;
use handshake_core::swarm_orchestration::model_lane::{ModelLaneMessageKind, NewModelLaneMessage};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use yrs::updates::{decoder::Decode, encoder::Encode};
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

/// Yjs text field name shared by every CRDT document this module seeds.
pub const CRDT_TEXT_NAME: &str = "mt009-shared-document";
/// Document schema id stamped on every seeded CRDT document.
pub const DOCUMENT_SCHEMA_ID: &str = "hsk.doc.rich_document@1";

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Append one real Yjs v1 text update to `canonical` from an author replica with
/// `client_id`, returning the encoded update bytes.
pub fn append_yjs_text_update(canonical: &Doc, client_id: u64, text: &str) -> Vec<u8> {
    let canonical_state = canonical
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let author = Doc::with_client_id(client_id);
    let author_text = author.get_or_insert_text(CRDT_TEXT_NAME);
    if !canonical_state.is_empty() {
        author
            .transact_mut()
            .apply_update(Update::decode_v1(&canonical_state).expect("decode canonical Yjs state"))
            .expect("apply canonical Yjs state to author replica");
    }

    let before = author.transact().state_vector();
    {
        let mut transaction = author.transact_mut();
        let offset = author_text.len(&transaction);
        author_text.insert(&mut transaction, offset, text);
    }
    let update = author.transact().encode_diff_v1(&before);
    canonical
        .transact_mut()
        .apply_update(Update::decode_v1(&update).expect("decode generated Yjs update"))
        .expect("apply generated Yjs update to canonical replica");
    update
}

/// Materialize `(text, state_vector_b64)` from a live Yjs `Doc`.
pub fn yjs_materialize_doc(doc: &Doc) -> (String, String) {
    let text = doc.get_or_insert_text(CRDT_TEXT_NAME);
    let transaction = doc.transact();
    (
        text.get_string(&transaction),
        base64::engine::general_purpose::STANDARD.encode(transaction.state_vector().encode_v1()),
    )
}

/// Apply persisted Yjs bytes exactly as returned by PostgreSQL. Deliberately
/// reads no ModelLane diagnostic metadata, so a bogus label cannot make a
/// corrupt update look materialized.
pub fn yjs_materialize_updates(updates: &[Vec<u8>]) -> (String, String) {
    let document = Doc::new();
    for update_bytes in updates {
        document
            .transact_mut()
            .apply_update(Update::decode_v1(update_bytes).expect("decode persisted Yjs update"))
            .expect("apply persisted Yjs update");
    }
    yjs_materialize_doc(&document)
}

#[allow(clippy::too_many_arguments)]
pub fn yjs_envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    document_schema_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
    update_bytes: &[u8],
    before: &KnowledgeStateVectorV1,
    after: &KnowledgeStateVectorV1,
) -> YjsUpdateEnvelopeV1 {
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, actor);
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        update_id: update_id.to_string(),
        actor_id: actor.canonical(),
        site_id: site.site_id,
        session_id: session_id.to_string(),
        trace_id: format!("trace-{update_id}"),
        document_schema_id: document_schema_id.to_string(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(update_bytes),
        update_sha256: sha256_hex(update_bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    }
}

/// Push one real Yjs update through the product `push_yjs_update` path and
/// assert it landed at `expected_seq`.
#[allow(clippy::too_many_arguments)]
pub async fn push_yjs_update_for_test(
    db: &(dyn Database + '_),
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    document_schema_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    site_id: &str,
    session_id: &str,
    update_bytes: &[u8],
    state_vector: &mut KnowledgeStateVectorV1,
    expected_seq: u64,
) -> YjsUpdateEnvelopeV1 {
    let before = state_vector.clone();
    state_vector.increment(site_id);
    let envelope = yjs_envelope(
        workspace_id,
        document_id,
        crdt_document_id,
        document_schema_id,
        update_id,
        actor,
        session_id,
        update_bytes,
        &before,
        state_vector,
    );
    match push_yjs_update(db, &envelope)
        .await
        .expect("store real Yjs update in PostgreSQL/EventLedger")
    {
        YjsPushOutcomeV1::Stored { update_seq, .. } => {
            assert_eq!(update_seq, expected_seq, "Yjs updates must be sequenced")
        }
        other => panic!("expected stored Yjs update, got {other:?}"),
    }
    envelope
}

/// Durable receipts persisted for one real CRDT document by
/// [`seed_real_crdt_document`]. Everything here is a genuine
/// PostgreSQL/EventLedger row created through `push_yjs_update` and
/// `append_kernel_crdt_snapshot`, so a message that references these values
/// exercises the real resolver, not a fabricated shortcut.
pub struct RealCrdtReceipts {
    /// `snapshot_bytes_ref` of a real snapshot covering `snapshot_covered_seq`.
    pub snapshot_bytes_ref: String,
    /// The snapshot's `covered_update_seq` (strictly less than the post-update
    /// seq, so the resolver's causal-ordering guard is satisfied).
    pub snapshot_covered_seq: i64,
    /// `update_bytes_ref` of a real post-snapshot update (seq == 2) that fully
    /// validates against its EventLedger event.
    pub post_update_bytes_ref: String,
    /// The post-snapshot update's server-derived `state_vector_after`.
    pub post_update_state_vector_after: String,
    /// The post-snapshot update's `kernel_crdt_updates.update_id`.
    pub post_update_id: String,
    /// The post-snapshot update's server-stamped `trace_id`
    /// (`trace-{post_update_id}`), which a lease's `correlation_id` and the
    /// message's `linked_span_contexts` must carry.
    pub post_update_trace_id: String,
    /// Durable identity of the seeded document, needed to build proposal rows
    /// that match the resolved update.
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    /// The seeding model actor (`local_model:{label}-local`).
    pub actor: KnowledgeActorIdV1,
    /// The session that owns every seeded update (`session-{label}`).
    pub session_id: String,
    /// Server-derived state vector after the last persisted update.
    pub state_vector: KnowledgeStateVectorV1,
    /// Derived Yjs client id for the seeding actor's site.
    pub yjs_client_id: u64,
    /// Derived site id for the seeding actor.
    pub site_id: String,
    /// Sequence number of the last persisted update (2 right after seeding).
    pub last_update_seq: u64,
}

/// Persist one real CRDT document into the isolated schema behind `db`: a
/// pre-snapshot update (seq 1), a snapshot covering seq 1, and a post-snapshot
/// update (seq 2).
pub async fn seed_real_crdt_document(
    db: &(dyn Database + '_),
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    label: &str,
) -> RealCrdtReceipts {
    let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, &format!("{label}-local"))
        .expect("typed local model actor for real CRDT seed");
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, &actor);
    let session_id = format!("session-{label}");
    let mut state_vector = KnowledgeStateVectorV1::new();
    let canonical = Doc::new();

    let pre_update_id = format!("{label}-yjs-pre");
    let pre_bytes = append_yjs_text_update(
        &canonical,
        u64::from(site.yjs_client_id),
        &format!("[{label}-pre]"),
    );
    push_yjs_update_for_test(
        db,
        workspace_id,
        document_id,
        crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &pre_update_id,
        &actor,
        &site.site_id,
        &session_id,
        &pre_bytes,
        &mut state_vector,
        1,
    )
    .await;

    let snapshot_state_vector = state_vector.encode();
    let snapshot_bytes = canonical
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let snapshot_identity = knowledge_crdt_identity(
        workspace_id,
        document_id,
        crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &actor,
        &format!("trace-{label}-snapshot"),
    );
    let snapshot_event = NewKernelEvent::builder(
        format!("KTR-{}-SNAP", label.to_uppercase()),
        session_id.clone(),
        KernelEventType::KnowledgeCrdtSnapshotRecorded,
        actor.to_kernel_actor(),
    )
    .aggregate("knowledge_crdt_document", crdt_document_id.to_string())
    .idempotency_key(format!("{label}:snapshot"))
    .source_component("model_lane_crdt_support")
    .payload(json!({
        "covered_update_seq": 1,
        "state_vector": &snapshot_state_vector,
        "document_id": document_id,
    }))
    .build()
    .expect("build real CRDT snapshot EventLedger event");
    let snapshot_event = db
        .append_kernel_event(snapshot_event)
        .await
        .expect("append real CRDT snapshot EventLedger event");
    let snapshot = new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity: &snapshot_identity,
        snapshot_id: &format!("{label}-snapshot-1"),
        covered_update_seq: 1,
        snapshot_bytes: &snapshot_bytes,
        snapshot_bytes_ref: &format!(
            "postgres://kernel_crdt_snapshots/{crdt_document_id}/{label}-snapshot-1"
        ),
        state_vector: &snapshot_state_vector,
        event_ledger_event_id: &snapshot_event.event_id,
        promotion_evidence_update_ids: &[pre_update_id.as_str()],
    });
    db.append_kernel_crdt_snapshot(snapshot.clone(), snapshot_bytes.clone())
        .await
        .expect("persist real CRDT snapshot receipt and bytes");

    let post_update_id = format!("{label}-yjs-post");
    let post_bytes = append_yjs_text_update(
        &canonical,
        u64::from(site.yjs_client_id),
        &format!("[{label}-post]"),
    );
    push_yjs_update_for_test(
        db,
        workspace_id,
        document_id,
        crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &post_update_id,
        &actor,
        &site.site_id,
        &session_id,
        &post_bytes,
        &mut state_vector,
        2,
    )
    .await;

    let records = db
        .list_kernel_crdt_updates(workspace_id, document_id, crdt_document_id)
        .await
        .expect("list persisted real CRDT updates");
    let post = records
        .iter()
        .find(|record| record.update_id == post_update_id)
        .expect("post-snapshot update is durably persisted");

    RealCrdtReceipts {
        snapshot_bytes_ref: snapshot.snapshot_bytes_ref.clone(),
        snapshot_covered_seq: 1,
        post_update_bytes_ref: post.update_bytes_ref.clone(),
        post_update_state_vector_after: post.state_vector_after.clone(),
        post_update_id: post_update_id.clone(),
        post_update_trace_id: format!("trace-{post_update_id}"),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        actor,
        session_id,
        state_vector,
        yjs_client_id: u64::from(site.yjs_client_id),
        site_id: site.site_id,
        last_update_seq: 2,
    }
}

/// One extra real persisted Yjs update on an already-seeded document. Used by
/// negatives that need TWO genuinely distinct, fully valid update rows.
pub struct ExtraRealUpdate {
    pub update_id: String,
    pub update_bytes_ref: String,
    pub state_vector_after: String,
    pub trace_id: String,
    pub update_seq: u64,
}

/// Append one more REAL Yjs update to a document seeded by
/// [`seed_real_crdt_document`], reusing the same actor/site/session so the
/// resolver's identity gates still pass for the new update.
///
/// The author replica is rebuilt from the bytes PostgreSQL actually holds, so
/// the new update is a genuine causal successor of the persisted document
/// rather than of an in-memory convenience replica.
pub async fn append_extra_real_update(
    db: &(dyn Database + '_),
    receipts: &mut RealCrdtReceipts,
    update_id: &str,
    text: &str,
) -> ExtraRealUpdate {
    let persisted = db
        .list_kernel_crdt_updates(
            &receipts.workspace_id,
            &receipts.document_id,
            &receipts.crdt_document_id,
        )
        .await
        .expect("list persisted real CRDT updates before appending another");
    let canonical = Doc::new();
    for record in &persisted {
        let bytes = db
            .read_kernel_crdt_update_bytes(&record.update_bytes_ref)
            .await
            .expect("read persisted CRDT update bytes from PostgreSQL");
        canonical
            .transact_mut()
            .apply_update(Update::decode_v1(&bytes).expect("decode persisted Yjs update"))
            .expect("apply persisted Yjs update to rebuilt replica");
    }
    let bytes = append_yjs_text_update(&canonical, receipts.yjs_client_id, text);
    let expected_seq = receipts.last_update_seq + 1;
    push_yjs_update_for_test(
        db,
        &receipts.workspace_id,
        &receipts.document_id,
        &receipts.crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        update_id,
        &receipts.actor,
        &receipts.site_id,
        &receipts.session_id,
        &bytes,
        &mut receipts.state_vector,
        expected_seq,
    )
    .await;
    receipts.last_update_seq = expected_seq;

    let records = db
        .list_kernel_crdt_updates(
            &receipts.workspace_id,
            &receipts.document_id,
            &receipts.crdt_document_id,
        )
        .await
        .expect("list persisted real CRDT updates after extra push");
    let record = records
        .iter()
        .find(|record| record.update_id == update_id)
        .expect("extra update is durably persisted");
    ExtraRealUpdate {
        update_id: update_id.to_string(),
        update_bytes_ref: record.update_bytes_ref.clone(),
        state_vector_after: record.state_vector_after.clone(),
        trace_id: format!("trace-{update_id}"),
        update_seq: expected_seq,
    }
}

/// A proposal minted through the real product API, plus the exact approved diff
/// payload a later `apply_approved_ai_edit` must present.
pub struct MintedProposal {
    pub proposal_id: String,
    pub proposed_diff: Value,
    /// The workspace-scoped lease `record_ai_edit_proposal` required. Still
    /// ACTIVE; release it with [`release_proposal_lease`].
    pub workspace_lease_id: String,
}

/// Record ONE `knowledge_crdt_ai_edit_proposals` draft row through the REAL
/// product API (`record_ai_edit_proposal`), never a raw INSERT. Leaves the row
/// in `review_state = 'proposed'`.
///
/// `anchor_trace_id` is the trace the proposal is correlated to; the ModelLane
/// resolver requires `proposal.correlation_id == resolved.trace_id`, so it must
/// be the `trace_id` of the update the MESSAGE cites. Actor, actor_kind,
/// session, workspace, document and crdt_document all come from `receipts`, so
/// they match the resolved update by construction.
pub async fn record_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    receipts: &RealCrdtReceipts,
    anchor_trace_id: &str,
    label: &str,
) -> MintedProposal {
    // Workspace-scoped lease chokepoint required by `record_ai_edit_proposal`.
    let workspace_lease = match claim_lease(
        db,
        pool,
        LeaseClaimRequestV1 {
            lane_id: format!("lane-proposal-{label}"),
            actor: receipts.actor.clone(),
            session_id: receipts.session_id.clone(),
            correlation_id: anchor_trace_id.to_string(),
            scope_kind: KnowledgeLeaseScopeKind::Workspace,
            scope_id: receipts.workspace_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim workspace lease for AI edit proposal minting")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("AI edit proposal workspace lease must claim, got {other:?}"),
    };

    let proposed_diff: Value = json!({
        "schema": "prosemirror-steps@1",
        "steps": [{"insert": format!("[{label}-approved-edit]")}],
    });
    let proposal = match record_ai_edit_proposal(
        db,
        pool,
        AiEditProposalRequestV1 {
            workspace_id: receipts.workspace_id.clone(),
            document_id: receipts.document_id.clone(),
            crdt_document_id: receipts.crdt_document_id.clone(),
            base_update_seq: 1,
            base_state_vector: "hsk-sv1:".to_string(),
            proposed_diff: proposed_diff.clone(),
            source_span_citations: vec![format!("KSP-{}", "0".repeat(32))],
            actor: receipts.actor.clone(),
            session_id: receipts.session_id.clone(),
            correlation_id: anchor_trace_id.to_string(),
            lease_id: Some(workspace_lease.lease_id.clone()),
        },
    )
    .await
    .expect("record AI edit proposal through the real product API")
    {
        RecordAiEditProposalOutcomeV1::Recorded(row) => *row,
        other => panic!("AI edit proposal must be recorded, got {other:?}"),
    };

    MintedProposal {
        proposal_id: proposal.proposal_id,
        proposed_diff,
        workspace_lease_id: workspace_lease.lease_id,
    }
}

/// Approve a recorded proposal through `decide_ai_edit_proposal` (operator
/// reviewer; models cannot self-approve).
pub async fn approve_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    proposal_id: &str,
    label: &str,
) {
    let reviewer = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, &format!("{label}-op"))
        .expect("typed operator reviewer");
    decide_ai_edit_proposal(
        db,
        pool,
        proposal_id,
        true,
        &reviewer,
        &format!("session-review-{label}"),
        "approved for MT-018 CRDT admission proof",
    )
    .await
    .expect("decide AI edit proposal through the real product API")
    .expect("AI edit proposal approval succeeds");
}

/// Bind an approved proposal to a REAL persisted update through
/// `apply_approved_ai_edit`, asserting the binding landed and stayed internally
/// consistent with the proposal's own approved diff.
pub async fn apply_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    receipts: &RealCrdtReceipts,
    minted: &MintedProposal,
    applied_update_id: &str,
    anchor_trace_id: &str,
) {
    match apply_approved_ai_edit(
        db,
        pool,
        &minted.proposal_id,
        applied_update_id,
        &minted.proposed_diff,
        &receipts.actor,
        &receipts.session_id,
        anchor_trace_id,
    )
    .await
    .expect("apply approved AI edit through the real product API")
    {
        AiEditApplyOutcomeV1::Bound(row) => {
            assert_eq!(row.applied_update_id.as_deref(), Some(applied_update_id));
            assert_eq!(
                row.applied_update_sha256.as_deref(),
                Some(row.diff_sha256.as_str()),
                "the applied binding must stay internally consistent with its own approved diff"
            );
        }
        other => panic!("approved AI edit must bind to the real update, got {other:?}"),
    }
}

/// Release the workspace lease taken by [`record_proposal`].
pub async fn release_proposal_lease(
    db: &(dyn Database + '_),
    pool: &PgPool,
    receipts: &RealCrdtReceipts,
    minted: &MintedProposal,
) {
    release_lease(db, pool, &minted.workspace_lease_id, &receipts.actor)
        .await
        .expect("release AI edit proposal workspace lease")
        .expect("AI edit proposal workspace lease exists");
}

/// Mint one APPROVED + APPLIED `knowledge_crdt_ai_edit_proposals` row through
/// the REAL product API chain — `record_ai_edit_proposal` ->
/// `decide_ai_edit_proposal` -> `apply_approved_ai_edit` — never a raw INSERT.
///
/// `applied_update_id` is the update the proposal claims to have applied; for a
/// consistent positive posture it equals the cited update's id, and negatives
/// deliberately point it elsewhere.
pub async fn mint_approved_applied_proposal(
    db: &(dyn Database + '_),
    pool: &PgPool,
    receipts: &RealCrdtReceipts,
    anchor_trace_id: &str,
    applied_update_id: &str,
    label: &str,
) -> String {
    let minted = record_proposal(db, pool, receipts, anchor_trace_id, label).await;
    approve_proposal(db, pool, &minted.proposal_id, label).await;
    apply_proposal(
        db,
        pool,
        receipts,
        &minted,
        applied_update_id,
        anchor_trace_id,
    )
    .await;
    release_proposal_lease(db, pool, receipts, &minted).await;
    minted.proposal_id
}

/// The durable CRDT posture a `NewModelLaneMessage` must carry to be ADMITTED,
/// plus the active lease that authorises it.
pub struct AdmissibleCrdtPosture {
    pub crdt_update_ref: String,
    pub crdt_base_snapshot_ref: String,
    pub crdt_state_vector: String,
    /// `Some` for a Proposal-anchored posture, `None` for a plain Status
    /// posture. Format: `crdt-proposal://<proposal_id>`.
    pub crdt_proposal_ref: Option<String>,
    /// The trace the message must link in `linked_span_contexts`.
    pub linked_trace_id: String,
    /// The active `knowledge_crdt_agent_lane_leases` lease id.
    pub lease_id: String,
}

/// How a caller wants the canonical posture anchored.
pub enum CrdtProposalAnchoring {
    /// No `crdt_proposal_ref`; the message is Status-kind CRDT authority.
    None,
    /// Mint a real approved + applied proposal bound to the cited update and
    /// anchor the message to it (the Proposal-kind path MT-018 unblocks).
    ApprovedApplied,
    /// Use a caller-supplied `crdt-proposal://` ref verbatim (negatives).
    Explicit(String),
}

/// Build the canonical admissible CRDT posture for a document already seeded by
/// [`seed_real_crdt_document`]: claim the exact active knowledge-agent lane
/// lease, optionally mint a real approved + applied proposal, and return the
/// refs a message must carry.
///
/// `lane_id` MUST be the id of a real ModelLane whose `session_id` equals
/// `receipts.session_id`, because `validate_crdt_lane_session_uniqueness_tx`
/// binds the update's session to exactly one lane.
pub async fn build_admissible_crdt_posture(
    db: &(dyn Database + '_),
    pool: &PgPool,
    receipts: &RealCrdtReceipts,
    lane_id: &str,
    label: &str,
    anchoring: CrdtProposalAnchoring,
) -> AdmissibleCrdtPosture {
    let crdt_proposal_ref = match anchoring {
        CrdtProposalAnchoring::None => None,
        CrdtProposalAnchoring::Explicit(reference) => Some(reference),
        CrdtProposalAnchoring::ApprovedApplied => {
            let proposal_id = mint_approved_applied_proposal(
                db,
                pool,
                receipts,
                &receipts.post_update_trace_id,
                &receipts.post_update_id,
                label,
            )
            .await;
            Some(format!("crdt-proposal://{proposal_id}"))
        }
    };

    let lease = match claim_lease(
        db,
        pool,
        LeaseClaimRequestV1 {
            lane_id: lane_id.to_string(),
            actor: receipts.actor.clone(),
            session_id: receipts.session_id.clone(),
            correlation_id: receipts.post_update_trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: receipts.crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim the exact active knowledge-agent lane lease for the admissible posture")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("admissible CRDT posture lease must claim, got {other:?}"),
    };

    AdmissibleCrdtPosture {
        crdt_update_ref: receipts.post_update_bytes_ref.clone(),
        crdt_base_snapshot_ref: receipts.snapshot_bytes_ref.clone(),
        crdt_state_vector: receipts.post_update_state_vector_after.clone(),
        crdt_proposal_ref,
        linked_trace_id: receipts.post_update_trace_id.clone(),
        lease_id: lease.lease_id,
    }
}

/// Stamp a canonical posture onto any binary's `NewModelLaneMessage` fixture.
/// The message `kind` is set from `kind`; a Proposal-kind message keeps the
/// `crdt_proposal_ref` the posture carries (the resolver rejects a Proposal-kind
/// CRDT message without one).
pub fn apply_crdt_posture(
    message: &mut NewModelLaneMessage,
    posture: &AdmissibleCrdtPosture,
    kind: ModelLaneMessageKind,
) {
    message.kind = kind;
    message.crdt_update_ref = Some(posture.crdt_update_ref.clone());
    message.crdt_base_snapshot_ref = Some(posture.crdt_base_snapshot_ref.clone());
    message.crdt_state_vector = Some(posture.crdt_state_vector.clone());
    message.crdt_proposal_ref = posture.crdt_proposal_ref.clone();
    message.crdt_stale_base_ref = None;
    message
        .linked_span_contexts
        .push(posture.linked_trace_id.clone());
}
