//! Native Atelier main panel.
//!
//! The shell-level Atelier module hosts sibling tool tabs inside one filling pane. CKC reuses the
//! existing Atelier intake/drag-source widget and canvas board; Posekit and Ingest expose stable,
//! nonblank native control surfaces so agents can address and inspect them before deeper parity work.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use egui::accesskit;
use sha2::{Digest, Sha256};

use crate::atelier_side_panel::AtelierSidePanel;
use crate::backend_client::{
    AtelierCharacterRow, AtelierCkcAppendCell, AtelierCkcCell, AtelierCkcCharacterDocumentCell,
    AtelierCkcCharacterDocumentRow, AtelierCkcCharacterSheetRow, AtelierCkcCreateCell,
    AtelierCkcExportCell, AtelierCkcFieldSuggestionsCell, AtelierCkcImportCell,
    AtelierCkcMediaAlbumCreateCell, AtelierCkcMediaAlbumItemsCell,
    AtelierCkcMediaAlbumListPageCell, AtelierCkcMediaAlbumListPageRow, AtelierCkcMediaAlbumRow,
    AtelierCkcMediaMemberRow, AtelierCkcMediaNotesCell, AtelierCkcMediaNotesTagsRow,
    AtelierCkcMoodboardSnapshotCell, AtelierCkcMoodboardSnapshotRow, AtelierCkcSafeSubsetCell,
    AtelierCkcSearchCell, AtelierCkcSearchResponse, AtelierCkcSearchResultRow,
    AtelierCkcSheetArtifactLinkRow, AtelierCkcSheetArtifactLinksCell, AtelierCkcStoryBeatCell,
    AtelierCkcStoryBeatRow, AtelierCkcStoryCardCell, AtelierCkcStoryCardRow, AtelierCkcTagNoteCell,
    AtelierCkcTagNoteRow, AtelierCkcTemplateCell, AtelierClient, AtelierContactSheetExportCell,
    AtelierContactSheetExportRow, AtelierContactSheetItem, AtelierFacialCommandCell,
    AtelierFacialCommandResponseRow, AtelierFacialFeatureListCell, AtelierFacialFeatureListRow,
    AtelierFacialIngestAnalysisCell, AtelierFacialIngestAnalysisRow,
    AtelierIntakeClassificationCell, AtelierIntakeClassificationDecision, AtelierItemRow,
    AtelierModelOperationCell, AtelierModelOperationLeaseRow, AtelierModelOperationResult,
    AtelierPoseSourceBytesCell, AtelierPosekitExportCell, AtelierPosekitExportRow,
    AtelierPreferenceMutationCell, AtelierPreferenceRow, AtelierPreferenceSaveCell,
    AtelierPreferencesCell, AtelierPromptFeedbackCaseRow as BackendPromptFeedbackCaseRow,
    AtelierPromptFeedbackExportCell, AtelierPromptFeedbackImportCell,
    AtelierPromptFeedbackRewriteCell, AtelierSheetExportRow, AtelierSheetFieldSuggestionRow,
    AtelierSheetVersionRow, ModelOperationContext, HSK_HEADER_ACTOR_ID,
};
use crate::editor_pane_factories::SharedPalette;
use crate::graph::canvas_board::{
    CanvasEvent, CanvasMutationPolicy, CanvasPlacementCard, LoomCanvasBoard, VisualEdge,
    DEFAULT_CARD_H, DEFAULT_CARD_W,
};
use crate::interop::{AtelierItemKind, AtelierRef, DragPayload};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsPalette;
use uuid::Uuid;

const LOCAL_CKC_SAFE_SUBSET_V2_JSON: &str = include_str!(
    "../../../backend/handshake_core/src/atelier/templates/LLM_SAFE_SUBSET__v2.00.json"
);
const LOCAL_CKC_TEMPLATE_VERSION: &str = "v2.00";

pub const ATELIER_PANEL_AUTHOR_ID: &str = "atelier-main-panel";
pub const ATELIER_TABLIST_AUTHOR_ID: &str = "atelier-tab-list";
pub const ATELIER_TAB_CKC_AUTHOR_ID: &str = "atelier-tab-ckc";
pub const ATELIER_TAB_POSEKIT_AUTHOR_ID: &str = "atelier-tab-posekit";
pub const ATELIER_TAB_INGEST_AUTHOR_ID: &str = "atelier-tab-ingest";
pub const ATELIER_CONTENT_CKC_AUTHOR_ID: &str = "atelier-content-ckc";
pub const ATELIER_CONTENT_POSEKIT_AUTHOR_ID: &str = "atelier-content-posekit";
pub const ATELIER_CONTENT_INGEST_AUTHOR_ID: &str = "atelier-content-ingest";
// WP-CKC MT-042: operator-facing "Settings / Defaults" region (panel-wide, not a 4th tab).
pub const ATELIER_SETTINGS_REGION_AUTHOR_ID: &str = "atelier-settings-region";
pub const ATELIER_SETTINGS_DEFAULT_TAB_AUTHOR_ID: &str = "atelier-settings-default-tab";
pub const ATELIER_SETTINGS_CKC_BOOK_MODE_AUTHOR_ID: &str = "atelier-settings-ckc-book-mode";
pub const ATELIER_SETTINGS_POSEKIT_LENS_AUTHOR_ID: &str = "atelier-settings-posekit-lens";
pub const ATELIER_SETTINGS_POSEKIT_FRAMING_PRESET_AUTHOR_ID: &str =
    "atelier-settings-posekit-framing-preset";
pub const ATELIER_SETTINGS_POSEKIT_MARKERS_FACE_AUTHOR_ID: &str =
    "atelier-settings-posekit-markers-face";
pub const ATELIER_SETTINGS_POSEKIT_MARKERS_BODY_AUTHOR_ID: &str =
    "atelier-settings-posekit-markers-body";
pub const ATELIER_SETTINGS_POSEKIT_MARKERS_HANDS_AUTHOR_ID: &str =
    "atelier-settings-posekit-markers-hands";
pub const ATELIER_SETTINGS_POSEKIT_PADDING_TOP_AUTHOR_ID: &str =
    "atelier-settings-posekit-padding-top";
pub const ATELIER_SETTINGS_POSEKIT_PADDING_RIGHT_AUTHOR_ID: &str =
    "atelier-settings-posekit-padding-right";
pub const ATELIER_SETTINGS_POSEKIT_PADDING_BOTTOM_AUTHOR_ID: &str =
    "atelier-settings-posekit-padding-bottom";
pub const ATELIER_SETTINGS_POSEKIT_PADDING_LEFT_AUTHOR_ID: &str =
    "atelier-settings-posekit-padding-left";
pub const ATELIER_SETTINGS_INGEST_BATCH_TAGS_AUTHOR_ID: &str = "atelier-settings-ingest-batch-tags";
pub const ATELIER_SETTINGS_INGEST_POLICY_AUTHOR_ID: &str = "atelier-settings-ingest-policy";
pub const ATELIER_SETTINGS_SAVE_AUTHOR_ID: &str = "atelier-settings-save";
pub const ATELIER_SETTINGS_STATUS_AUTHOR_ID: &str = "atelier-settings-status";
pub const ATELIER_MODEL_OPS_REGION_AUTHOR_ID: &str = "atelier-model-ops-region";
pub const ATELIER_MODEL_OPS_ACTOR_AUTHOR_ID: &str = "atelier-model-ops-actor";
pub const ATELIER_MODEL_OPS_SESSION_AUTHOR_ID: &str = "atelier-model-ops-session";
pub const ATELIER_MODEL_OPS_THREAD_AUTHOR_ID: &str = "atelier-model-ops-thread";
pub const ATELIER_MODEL_OPS_LEASE_AUTHOR_ID: &str = "atelier-model-ops-lease";
pub const ATELIER_MODEL_OPS_TTL_AUTHOR_ID: &str = "atelier-model-ops-ttl";
pub const ATELIER_MODEL_OPS_ACTION_AUTHOR_ID: &str = "atelier-model-ops-action";
pub const ATELIER_MODEL_OPS_RECEIPT_STATUS_AUTHOR_ID: &str = "atelier-model-ops-receipt-status";
pub const ATELIER_MODEL_OPS_RECEIPT_ERROR_CLASS_AUTHOR_ID: &str =
    "atelier-model-ops-receipt-error-class";
pub const ATELIER_MODEL_OPS_RECEIPT_RECOVERY_HINT_AUTHOR_ID: &str =
    "atelier-model-ops-receipt-recovery-hint";
pub const ATELIER_MODEL_OPS_STATE_AUTHOR_ID: &str = "atelier-model-ops-state";
pub const ATELIER_MODEL_OPS_CLAIM_AUTHOR_ID: &str = "atelier-model-ops-claim";
pub const ATELIER_MODEL_OPS_RELEASE_AUTHOR_ID: &str = "atelier-model-ops-release";
pub const ATELIER_MODEL_OPS_RECEIPT_AUTHOR_ID: &str = "atelier-model-ops-receipt";
pub const ATELIER_MODEL_OPS_STATUS_AUTHOR_ID: &str = "atelier-model-ops-status";

pub fn model_ops_next_active_lease_id(
    current_lease_id: &str,
    response_claim_id: &str,
    effective_state: &str,
    lease_expired: bool,
) -> Option<String> {
    if effective_state.eq_ignore_ascii_case("active") && !lease_expired {
        return Some(response_claim_id.to_owned());
    }
    if current_lease_id.trim() == response_claim_id {
        return Some(String::new());
    }
    None
}

pub fn model_ops_matching_active_lease_id<'a>(
    actor_id: &str,
    session_id: &str,
    leases: &'a [AtelierModelOperationLeaseRow],
) -> Option<&'a str> {
    let actor_id = actor_id.trim();
    let session_id = session_id.trim();
    if actor_id.is_empty() || session_id.is_empty() {
        return None;
    }
    leases
        .iter()
        .find(|lease| {
            lease.effective_state.eq_ignore_ascii_case("active")
                && !lease.lease_expired
                && lease.actor_id.trim() == actor_id
                && lease.session_id.trim() == session_id
        })
        .map(|lease| lease.claim_id.as_str())
}

pub fn preference_model_ops_thread_id(key: &str) -> String {
    format!("atelier.preferences.{key}")
}

pub fn stable_model_ops_thread_segment(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub fn ckc_character_create_model_ops_thread_id(public_id: &str) -> String {
    format!(
        "atelier.ckc.character.public.{}",
        stable_model_ops_thread_segment(public_id)
    )
}

pub fn ckc_character_model_ops_thread_id(character_internal_id: &str) -> String {
    format!("atelier.ckc.character.{character_internal_id}")
}

pub fn ckc_document_model_ops_thread_id(document_id: &str) -> String {
    format!("atelier.ckc.document.{document_id}")
}

pub fn ckc_media_album_model_ops_thread_id(collection_id: &str) -> String {
    format!("atelier.ckc.media-album.{collection_id}")
}

pub fn ckc_sheet_artifacts_model_ops_thread_id(version_id: &str) -> String {
    format!("atelier.ckc.sheet-version.{version_id}.artifacts")
}

pub fn ckc_tag_note_model_ops_thread_id(tag_text: &str, scope_ref: Option<&str>) -> String {
    let scope = scope_ref
        .map(stable_model_ops_thread_segment)
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "atelier.ckc.tag-note.{}.scope.{scope}",
        stable_model_ops_thread_segment(tag_text)
    )
}

pub fn posekit_openpose_model_ops_thread_id(source_ref: &str, rig_id: Option<&str>) -> String {
    match rig_id {
        Some(rig_id) if !rig_id.trim().is_empty() => {
            format!("atelier.posekit.rig.{}.openpose", rig_id.trim())
        }
        _ => format!(
            "atelier.posekit.source.{}.openpose",
            stable_model_ops_thread_segment(source_ref)
        ),
    }
}

pub fn contact_sheet_model_ops_thread_id(source_kind: &str, source_ref: &str) -> String {
    format!(
        "atelier.contact-sheet.{}.{}",
        stable_model_ops_thread_segment(source_kind),
        stable_model_ops_thread_segment(source_ref)
    )
}

pub fn intake_batch_model_ops_thread_id(batch_id: &str) -> String {
    format!("atelier.intake.batch.{batch_id}")
}

pub fn facial_review_session_model_ops_thread_id(session_id: &str) -> String {
    format!("atelier.facial.review.session.{session_id}")
}

fn prompt_feedback_project_model_ops_thread_id(project_id: &str) -> String {
    format!("atelier.prompt-feedback.project.{project_id}")
}

fn prompt_feedback_case_model_ops_thread_id(case_id: &str) -> String {
    format!("atelier.prompt-feedback.case.{case_id}")
}

fn prompt_feedback_export_model_ops_thread_id(
    rule_pack_id: &str,
    rule_pack_version: i32,
    case_ids: &[String],
) -> String {
    let mut sorted_case_ids = case_ids.to_vec();
    sorted_case_ids.sort();
    format!(
        "atelier.prompt-feedback.export.{}.{}.{}",
        stable_model_ops_thread_segment(rule_pack_id),
        rule_pack_version,
        stable_model_ops_thread_segment(&sorted_case_ids.join("|"))
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOpsMutationContext {
    pub actor_id: String,
    pub context: Option<ModelOperationContext>,
}

pub fn model_ops_context_for_expected_thread(
    session_id: &str,
    lease_claim_id: &str,
    lease_thread_id: &str,
    expected_thread_id: &str,
) -> Result<Option<ModelOperationContext>, String> {
    let session_id = session_id.trim();
    let lease_claim_id = lease_claim_id.trim();
    if session_id.is_empty() || lease_claim_id.is_empty() {
        return Ok(None);
    }
    let lease_thread_id = lease_thread_id.trim();
    if lease_thread_id.is_empty() {
        return Err(format!(
            "Cannot attach Model Ops lease {lease_claim_id}: active lease thread is unknown; read state or claim {expected_thread_id}."
        ));
    }
    if lease_thread_id != expected_thread_id.trim() {
        return Err(format!(
            "Cannot attach Model Ops lease {lease_claim_id}: lease thread {lease_thread_id} does not match required thread {expected_thread_id}."
        ));
    }
    Ok(Some(ModelOperationContext::new_with_thread(
        session_id,
        lease_claim_id,
        lease_thread_id,
    )))
}

pub fn model_ops_mutation_context_for_expected_thread(
    model_ops_actor_id: &str,
    model_ops_session_id: &str,
    model_ops_lease_id: &str,
    model_ops_active_lease_thread_id: &str,
    fallback_actor_id: &str,
    expected_thread_id: &str,
) -> Result<ModelOpsMutationContext, String> {
    let fallback_actor_id = fallback_actor_id.trim();
    let configured_actor_id = model_ops_actor_id.trim();
    let actor_id = if configured_actor_id.is_empty() {
        fallback_actor_id
    } else {
        configured_actor_id
    };
    if actor_id.is_empty() {
        return Err("Cannot mutate: actor id is empty.".to_owned());
    }
    let lease_configured = !model_ops_lease_id.trim().is_empty()
        || !model_ops_active_lease_thread_id.trim().is_empty();
    let actor_overridden =
        !configured_actor_id.is_empty() && configured_actor_id != fallback_actor_id;
    if !lease_configured && actor_id != "operator" {
        let actor_context = if actor_overridden {
            format!("Model Ops actor {configured_actor_id}")
        } else {
            format!("actor {fallback_actor_id}")
        };
        return Err(format!(
            "Cannot mutate as {actor_context}: lease-free Model Ops mutation is reserved for actor operator; claim an exclusive lease for {expected_thread_id}."
        ));
    }
    if !lease_configured {
        return Ok(ModelOpsMutationContext {
            actor_id: actor_id.to_owned(),
            context: None,
        });
    }

    let context = model_ops_context_for_expected_thread(
        model_ops_session_id,
        model_ops_lease_id,
        model_ops_active_lease_thread_id,
        expected_thread_id,
    )?
    .ok_or_else(|| {
        format!(
            "Cannot mutate {expected_thread_id}: Model Ops session and lease id are required; read state or claim an exclusive lease first."
        )
    })?;
    Ok(ModelOpsMutationContext {
        actor_id: actor_id.to_owned(),
        context: Some(context),
    })
}

pub fn model_ops_receipt_dispatch_blocker(
    lease_id: &str,
    session_id: &str,
    thread_id: &str,
    action_id: &str,
    status: &str,
    error_class: &str,
    recovery_hint: &str,
    target_refs: &[String],
    evidence_refs: &[String],
    result_refs: &[String],
) -> Option<&'static str> {
    if session_id.trim().is_empty() {
        return Some("Cannot record receipt: session id is empty.");
    }
    if lease_id.trim().is_empty() {
        return Some("Cannot record receipt: lease id is empty.");
    }
    if thread_id.trim().is_empty() {
        return Some("Cannot record receipt: thread id is empty.");
    }
    if action_id.trim().is_empty() {
        return Some("Cannot record receipt: action id is empty.");
    }
    if !matches!(status.trim(), "succeeded" | "failed" | "rejected") {
        return Some(
            "Cannot record receipt: receipt status must be succeeded, failed, or rejected.",
        );
    }
    if matches!(status.trim(), "failed" | "rejected") && error_class.trim().is_empty() {
        return Some("Cannot record receipt: failed/rejected receipts require error class.");
    }
    if matches!(status.trim(), "failed" | "rejected") && recovery_hint.trim().is_empty() {
        return Some("Cannot record receipt: failed/rejected receipts require recovery hint.");
    }
    if target_refs.iter().all(|value| value.trim().is_empty()) {
        return Some("Cannot record receipt: target refs are empty.");
    }
    if evidence_refs.iter().all(|value| value.trim().is_empty()) {
        return Some("Cannot record receipt: evidence refs are empty.");
    }
    if result_refs.iter().all(|value| value.trim().is_empty()) {
        return Some("Cannot record receipt: result refs are empty.");
    }
    None
}

fn model_ops_mutation_context_from_state(
    state: &AtelierPanelState,
    fallback_actor_id: &str,
    expected_thread_id: &str,
) -> Result<ModelOpsMutationContext, String> {
    model_ops_mutation_context_for_expected_thread(
        &state.model_ops_actor_id,
        &state.model_ops_session_id,
        &state.model_ops_lease_id,
        &state.model_ops_active_lease_thread_id,
        fallback_actor_id,
        expected_thread_id,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelOpsUiFields {
    actor_id: String,
    session_id: String,
    lease_id: String,
    active_lease_thread_id: String,
}

impl ModelOpsUiFields {
    fn from_state(state: &AtelierPanelState) -> Self {
        Self {
            actor_id: state.model_ops_actor_id.clone(),
            session_id: state.model_ops_session_id.clone(),
            lease_id: state.model_ops_lease_id.clone(),
            active_lease_thread_id: state.model_ops_active_lease_thread_id.clone(),
        }
    }

    fn mutation_context(
        &self,
        fallback_actor_id: &str,
        expected_thread_id: &str,
    ) -> Result<ModelOpsMutationContext, String> {
        model_ops_mutation_context_for_expected_thread(
            &self.actor_id,
            &self.session_id,
            &self.lease_id,
            &self.active_lease_thread_id,
            fallback_actor_id,
            expected_thread_id,
        )
    }
}

fn actor_spec_with_model_ops(
    mut spec: crate::backend_client::ActorRequestSpec,
    model_ops: ModelOpsMutationContext,
) -> crate::backend_client::ActorRequestSpec {
    spec.headers.retain(|(name, _)| name != HSK_HEADER_ACTOR_ID);
    spec.headers
        .push((HSK_HEADER_ACTOR_ID.to_owned(), model_ops.actor_id));
    spec.with_model_operation_context(model_ops.context.as_ref())
}

pub fn facial_review_model_ops_thread_for_session_id(
    configured_thread_id: &str,
    session_id: &str,
) -> Result<String, String> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Err(
            "Facial review Model Ops thread cannot be derived without a session id.".to_owned(),
        );
    }
    let expected_thread_id = facial_review_session_model_ops_thread_id(session_id);
    let configured_thread_id = configured_thread_id.trim();
    if configured_thread_id.is_empty() {
        return Ok(expected_thread_id);
    }
    if configured_thread_id != expected_thread_id {
        return Err(format!(
            "Facial review command for session {session_id} requires Model Ops thread {expected_thread_id}; current Model Ops thread is {configured_thread_id}."
        ));
    }
    Ok(expected_thread_id)
}

fn facial_review_model_ops_thread_from_state(
    state: &AtelierPanelState,
    session_id: &str,
) -> Result<String, String> {
    facial_review_model_ops_thread_for_session_id(&state.model_ops_thread_id, session_id)
}

pub const ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID: &str = "atelier-ckc-character-list";
// MT-037: the CKC backend-error / local-fallback status surface, now Argus-readable. Previously the
// error label was human-only (red text with no author_id); a parallel model could not read local-only,
// non-persistent create results. The character-create local branch sets this to a non-persistence status.
pub const ATELIER_CKC_ERROR_AUTHOR_ID: &str = "atelier-ckc-error";
pub const ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID: &str = "atelier-ckc-selected-character";
pub const ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID: &str = "atelier-ckc-character-create-name";
pub const ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID: &str = "atelier-ckc-character-create";
pub const ATELIER_CKC_CHARACTER_REF_AUTHOR_ID: &str = "atelier-ckc-character-ref";
pub const ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID: &str = "atelier-ckc-sheet-version-ref";
pub const ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID: &str = "atelier-ckc-sheet-editor";
pub const ATELIER_CKC_SHEET_SAVE_AUTHOR_ID: &str = "atelier-ckc-sheet-save-version";
// MT-037 (F1): CKC sheet-version local-fallback containment. The local "Append sheet version" branch used
// to SILENTLY mint a backend-looking sheet_version_ref with no status and no receipt (silent data loss).
// These surface a non-persistent status plus a persistence-mode receipt mirroring the story/moodboard node.
pub const ATELIER_CKC_SHEET_STATUS_AUTHOR_ID: &str = "atelier-ckc-sheet-status";
pub const ATELIER_CKC_SHEET_BACKEND_MODE_AUTHOR_ID: &str = "atelier-ckc-sheet-backend-mode";
pub const ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID: &str = "atelier-ckc-typed-ref-kind";
pub const ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID: &str = "atelier-ckc-template-status";
pub const ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID: &str = "atelier-ckc-template-load";
pub const ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID: &str = "atelier-ckc-safe-subset-load";
pub const ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID: &str = "atelier-ckc-import-editor";
pub const ATELIER_CKC_IMPORT_AUTHOR_ID: &str = "atelier-ckc-import-sheet-version";
pub const ATELIER_CKC_EXPORT_TXT_AUTHOR_ID: &str = "atelier-ckc-export-txt";
pub const ATELIER_CKC_EXPORT_JSON_AUTHOR_ID: &str = "atelier-ckc-export-json";
pub const ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID: &str = "atelier-ckc-export-safe-txt";
pub const ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID: &str = "atelier-ckc-export-safe-json";
pub const ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID: &str = "atelier-ckc-export-status";
pub const ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID: &str = "atelier-ckc-export-preview";
pub const ATELIER_CKC_EXPORT_REF_AUTHOR_ID: &str = "atelier-ckc-export-ref";
pub const ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID: &str = "atelier-ckc-field-suggestion-field";
pub const ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID: &str = "atelier-ckc-field-suggestions-load";
pub const ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID: &str = "atelier-ckc-field-suggestions-list";
pub const ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID: &str = "atelier-ckc-linked-media-list";
pub const ATELIER_CKC_SHEET_ARTIFACT_LIST_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-list";
pub const ATELIER_CKC_SHEET_ARTIFACT_KIND_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-kind";
pub const ATELIER_CKC_SHEET_ARTIFACT_REF_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-ref";
pub const ATELIER_CKC_SHEET_ARTIFACT_MANIFEST_AUTHOR_ID: &str =
    "atelier-ckc-sheet-artifact-manifest";
pub const ATELIER_CKC_SHEET_ARTIFACT_LABEL_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-label";
pub const ATELIER_CKC_SHEET_ARTIFACT_ROLE_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-role";
pub const ATELIER_CKC_SHEET_ARTIFACT_ACTOR_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-actor";
pub const ATELIER_CKC_SHEET_ARTIFACT_ATTACH_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-attach";
pub const ATELIER_CKC_SHEET_ARTIFACT_ATTACH_POSE_AUTHOR_ID: &str =
    "atelier-ckc-sheet-artifact-attach-posekit";
pub const ATELIER_CKC_SHEET_ARTIFACT_DETACH_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-detach";
pub const ATELIER_CKC_SHEET_ARTIFACT_REUSE_REF_AUTHOR_ID: &str =
    "atelier-ckc-sheet-artifact-reuse-ref";
pub const ATELIER_CKC_SHEET_ARTIFACT_STATUS_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-status";
pub const ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID: &str = "atelier-ckc-album-status";
pub const ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID: &str = "atelier-ckc-album-create-name";
pub const ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID: &str = "atelier-ckc-album-create-notes";
pub const ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID: &str = "atelier-ckc-album-create-tags";
pub const ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID: &str = "atelier-ckc-album-create";
pub const ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID: &str = "atelier-ckc-album-link-asset-ids";
pub const ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID: &str = "atelier-ckc-album-link-source-path";
pub const ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID: &str = "atelier-ckc-album-link-source-url";
pub const ATELIER_CKC_ALBUM_LINK_AUTHOR_ID: &str = "atelier-ckc-album-link-assets";
pub const ATELIER_CKC_SELECTED_MEDIA_OCCURRENCE_AUTHOR_ID: &str =
    "atelier-ckc-selected-media-occurrence";
pub const ATELIER_CKC_ALBUM_LINK_EDIT_SOURCE_PATH_AUTHOR_ID: &str =
    "atelier-ckc-album-link-edit-source-path";
pub const ATELIER_CKC_ALBUM_LINK_EDIT_SOURCE_URL_AUTHOR_ID: &str =
    "atelier-ckc-album-link-edit-source-url";
pub const ATELIER_CKC_ALBUM_LINK_EDIT_CLEAR_SOURCE_PATH_AUTHOR_ID: &str =
    "atelier-ckc-album-link-edit-clear-source-path";
pub const ATELIER_CKC_ALBUM_LINK_EDIT_CLEAR_SOURCE_URL_AUTHOR_ID: &str =
    "atelier-ckc-album-link-edit-clear-source-url";
pub const ATELIER_CKC_ALBUM_LINK_EDIT_SAVE_AUTHOR_ID: &str = "atelier-ckc-album-link-edit-save";
pub const ATELIER_CKC_ALBUM_LINK_EDIT_AUTHOR_ID: &str = ATELIER_CKC_ALBUM_LINK_EDIT_SAVE_AUTHOR_ID;
pub const ATELIER_CKC_ALBUM_UNLINK_AUTHOR_ID: &str = "atelier-ckc-album-unlink-selected";
pub const ATELIER_CKC_ALBUM_MOVE_UP_AUTHOR_ID: &str = "atelier-ckc-album-move-selected-up";
pub const ATELIER_CKC_ALBUM_MOVE_DOWN_AUTHOR_ID: &str = "atelier-ckc-album-move-selected-down";
pub const ATELIER_CKC_ALBUM_LINK_TARGET_AUTHOR_ID: &str = "atelier-ckc-album-link-target";
pub const ATELIER_CKC_ALBUM_LIST_PAGE_STATE_AUTHOR_ID: &str = "atelier-ckc-album-list-page-state";
pub const ATELIER_CKC_ALBUM_LIST_LOAD_MORE_AUTHOR_ID: &str = "atelier-ckc-album-list-load-more";
pub const ATELIER_CKC_MEDIA_BACKEND_MODE_AUTHOR_ID: &str = "atelier-ckc-media-backend-mode";
pub const ATELIER_CKC_MEDIA_ACTOR_AUTHOR_ID: &str = "atelier-ckc-media-actor";
pub const ATELIER_CKC_SELECTED_ALBUM_REF_AUTHOR_ID: &str = "atelier-ckc-selected-album-ref";
pub const ATELIER_CKC_SELECTED_MEDIA_REF_AUTHOR_ID: &str = "atelier-ckc-selected-media-ref";
pub const ATELIER_CKC_SELECTED_FOLDER_REF_AUTHOR_ID: &str = "atelier-ckc-selected-folder-ref";
pub const ATELIER_CKC_SELECTED_SOURCE_URL_REF_AUTHOR_ID: &str =
    "atelier-ckc-selected-source-url-ref";
pub const ATELIER_CKC_SELECTED_FOLDER_REF_KIND_AUTHOR_ID: &str =
    "atelier-ckc-selected-folder-ref-kind";
pub const ATELIER_CKC_SELECTED_SOURCE_URL_REF_KIND_AUTHOR_ID: &str =
    "atelier-ckc-selected-source-url-ref-kind";
pub const ATELIER_CKC_SELECTED_FOLDER_REF_ORIGIN_AUTHOR_ID: &str =
    "atelier-ckc-selected-folder-ref-origin";
pub const ATELIER_CKC_SELECTED_SOURCE_URL_REF_ORIGIN_AUTHOR_ID: &str =
    "atelier-ckc-selected-source-url-ref-origin";
pub const ATELIER_CKC_SELECTED_ALBUM_ATTRIBUTION_AUTHOR_ID: &str =
    "atelier-ckc-selected-album-attribution";
pub const ATELIER_CKC_SELECTED_MEDIA_ATTRIBUTION_AUTHOR_ID: &str =
    "atelier-ckc-selected-media-attribution";
pub const ATELIER_CKC_SELECTED_MEDIA_STATUS_AUTHOR_ID: &str = "atelier-ckc-selected-media-status";
pub const ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID: &str = "atelier-ckc-media-notes-editor";
pub const ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID: &str = "atelier-ckc-media-tags-editor";
pub const ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID: &str = "atelier-ckc-media-save";
pub const ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID: &str = "atelier-ckc-story-doc-ref";
pub const ATELIER_CKC_STORY_EDITOR_AUTHOR_ID: &str = "atelier-ckc-story-editor";
pub const ATELIER_CKC_STORY_SAVE_AUTHOR_ID: &str = "atelier-ckc-story-save";
pub const ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID: &str = "atelier-ckc-story-card-list";
pub const ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID: &str = "atelier-ckc-story-card-title";
pub const ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID: &str = "atelier-ckc-story-card-body";
pub const ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID: &str = "atelier-ckc-story-card-save";
pub const ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID: &str = "atelier-ckc-story-beat-editor";
pub const ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID: &str = "atelier-ckc-story-beat-save";
// MT-037: CKC story local-fallback containment. A per-surface status readout plus a persistence-mode
// receipt (backend:persistent vs local-preview:not-persisted) a model reads so a local-only story draft
// is never mistaken for a persisted backend write. Mirrors the media backend_mode value node.
pub const ATELIER_CKC_STORY_STATUS_AUTHOR_ID: &str = "atelier-ckc-story-status";
pub const ATELIER_CKC_STORY_BACKEND_MODE_AUTHOR_ID: &str = "atelier-ckc-story-backend-mode";
pub const ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID: &str = "atelier-ckc-moodboard-doc-ref";
pub const ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID: &str = "atelier-ckc-moodboard-latest-ref";
pub const ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID: &str = "atelier-ckc-moodboard-editor";
pub const ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID: &str = "atelier-ckc-moodboard-save";
pub const ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID: &str = "atelier-ckc-moodboard-open";
pub const ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID: &str = "atelier-ckc-moodboard-canvas";
// MT-037: CKC moodboard local-fallback containment. Status readout + persistence-mode receipt, matching
// the media backend_mode value node so a model never treats a local-only moodboard save as persisted.
pub const ATELIER_CKC_MOODBOARD_STATUS_AUTHOR_ID: &str = "atelier-ckc-moodboard-status";
pub const ATELIER_CKC_MOODBOARD_BACKEND_MODE_AUTHOR_ID: &str = "atelier-ckc-moodboard-backend-mode";
pub const ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID: &str = "atelier-ckc-book-layout";
pub const ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID: &str = "atelier-ckc-book-left-media";
pub const ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID: &str = "atelier-ckc-book-middle";
pub const ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID: &str = "atelier-ckc-book-right-sheet";
pub const ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID: &str = "atelier-ckc-media-viewer";
pub const ATELIER_CKC_MEDIA_IMAGE_STATUS_AUTHOR_ID: &str = "atelier-ckc-media-image-status";
pub const ATELIER_CKC_MODE_SHEET_AUTHOR_ID: &str = "atelier-ckc-mode-sheet";
pub const ATELIER_CKC_MODE_STORY_AUTHOR_ID: &str = "atelier-ckc-mode-story";
pub const ATELIER_CKC_MODE_NOTES_AUTHOR_ID: &str = "atelier-ckc-mode-notes";
pub const ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID: &str = "atelier-ckc-mode-moodboard";
pub const ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID: &str = "atelier-ckc-character-notes-editor";
pub const ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID: &str = "atelier-ckc-character-notes-apply";
pub const ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID: &str = "atelier-ckc-search-query";
pub const ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID: &str = "atelier-ckc-search-tags";
pub const ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID: &str =
    "atelier-ckc-search-filter-character";
pub const ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID: &str =
    "atelier-ckc-search-filter-collection";
pub const ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID: &str = "atelier-ckc-search-filter-media";
pub const ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID: &str =
    "atelier-ckc-search-filter-similarity";
pub const ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID: &str = "atelier-ckc-search-mode-fuzzy";
pub const ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID: &str = "atelier-ckc-search-mode-vector";
pub const ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID: &str = "atelier-ckc-search-mode-combined";
pub const ATELIER_CKC_SEARCH_RUN_AUTHOR_ID: &str = "atelier-ckc-search-run";
pub const ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID: &str = "atelier-ckc-search-status";
pub const ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID: &str = "atelier-ckc-search-results";
pub const ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID: &str = "atelier-ckc-tag-note-tag";
pub const ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID: &str = "atelier-ckc-tag-note-scope";
pub const ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID: &str = "atelier-ckc-tag-note-editor";
pub const ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID: &str = "atelier-ckc-tag-note-save";
pub const ATELIER_POSE_YAW_MINUS_AUTHOR_ID: &str = "atelier-pose-yaw-minus";
pub const ATELIER_POSE_YAW_PLUS_AUTHOR_ID: &str = "atelier-pose-yaw-plus";
pub const ATELIER_POSE_RESET_AUTHOR_ID: &str = "atelier-pose-reset";
pub const ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID: &str = "atelier-pose-face-toggle";
pub const ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID: &str = "atelier-pose-body-toggle";
pub const ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID: &str = "atelier-pose-hands-toggle";
pub const ATELIER_POSE_YAW_SLIDER_AUTHOR_ID: &str = "atelier-pose-yaw-slider";
pub const ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID: &str = "atelier-pose-pitch-slider";
pub const ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID: &str = "atelier-pose-zoom-slider";
pub const ATELIER_POSE_MARKER_FAMILY_AUTHOR_ID: &str = "atelier-pose-marker-family";
pub const ATELIER_POSE_MARKER_INDEX_AUTHOR_ID: &str = "atelier-pose-marker-index";
pub const ATELIER_POSE_MARKER_X_AUTHOR_ID: &str = "atelier-pose-marker-x";
pub const ATELIER_POSE_MARKER_Y_AUTHOR_ID: &str = "atelier-pose-marker-y";
pub const ATELIER_POSE_MARKER_CONFIDENCE_AUTHOR_ID: &str = "atelier-pose-marker-confidence";
pub const ATELIER_POSE_MARKER_APPLY_AUTHOR_ID: &str = "atelier-pose-marker-apply";
pub const ATELIER_POSE_MARKER_ADD_AUTHOR_ID: &str = "atelier-pose-marker-add";
pub const ATELIER_POSE_MARKER_REMOVE_AUTHOR_ID: &str = "atelier-pose-marker-remove";
pub const ATELIER_POSE_MARKER_RESET_AUTHOR_ID: &str = "atelier-pose-marker-reset";
pub const ATELIER_POSE_MARKER_NUDGE_LEFT_AUTHOR_ID: &str = "atelier-pose-marker-nudge-left";
pub const ATELIER_POSE_MARKER_NUDGE_RIGHT_AUTHOR_ID: &str = "atelier-pose-marker-nudge-right";
pub const ATELIER_POSE_MARKER_NUDGE_UP_AUTHOR_ID: &str = "atelier-pose-marker-nudge-up";
pub const ATELIER_POSE_MARKER_NUDGE_DOWN_AUTHOR_ID: &str = "atelier-pose-marker-nudge-down";
pub const ATELIER_POSE_MARKER_STATUS_AUTHOR_ID: &str = "atelier-pose-marker-status";
pub const ATELIER_POSE_FRAMING_PRESET_AUTHOR_ID: &str = "atelier-pose-framing-preset";
pub const ATELIER_POSE_FRAMING_LENS_AUTHOR_ID: &str = "atelier-pose-framing-lens";
pub const ATELIER_POSE_FRAMING_PADDING_TOP_AUTHOR_ID: &str = "atelier-pose-framing-padding-top";
pub const ATELIER_POSE_FRAMING_PADDING_RIGHT_AUTHOR_ID: &str = "atelier-pose-framing-padding-right";
pub const ATELIER_POSE_FRAMING_PADDING_BOTTOM_AUTHOR_ID: &str =
    "atelier-pose-framing-padding-bottom";
pub const ATELIER_POSE_FRAMING_PADDING_LEFT_AUTHOR_ID: &str = "atelier-pose-framing-padding-left";
pub const ATELIER_POSE_FRAMING_READOUT_AUTHOR_ID: &str = "atelier-pose-framing-readout";
pub const ATELIER_POSE_SOURCE_REF_AUTHOR_ID: &str = "atelier-pose-source-ref";
pub const ATELIER_POSE_SOURCE_STATUS_AUTHOR_ID: &str = "atelier-pose-source-status";
pub const ATELIER_POSE_RIG_ID_AUTHOR_ID: &str = "atelier-pose-rig-id";
pub const ATELIER_POSE_STATE_READOUT_AUTHOR_ID: &str = "atelier-pose-state-readout";
pub const ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID: &str = "atelier-pose-split-view";
pub const ATELIER_POSE_3D_VIEWPORT_AUTHOR_ID: &str = "atelier-pose-3d-viewport";
pub const ATELIER_POSE_OPENPOSE_VIEWPORT_AUTHOR_ID: &str = "atelier-pose-openpose-viewport";
pub const ATELIER_POSE_OPENPOSE_PNG_STATUS_AUTHOR_ID: &str = "atelier-pose-openpose-png-status";
pub const ATELIER_POSE_EXPORT_AUTHOR_ID: &str = "atelier-pose-export-openpose";
pub const ATELIER_POSE_EXPORT_STATUS_AUTHOR_ID: &str = "atelier-pose-export-status";
pub const ATELIER_POSE_EXPORT_REF_AUTHOR_ID: &str = "atelier-pose-export-ref";
pub const ATELIER_POSE_EXPORT_PREVIEW_AUTHOR_ID: &str = "atelier-pose-export-preview";
pub const ATELIER_INGEST_PASS_AUTHOR_ID: &str = "atelier-ingest-pass";
pub const ATELIER_INGEST_REJECT_AUTHOR_ID: &str = "atelier-ingest-reject";
pub const ATELIER_INGEST_UNSURE_AUTHOR_ID: &str = "atelier-ingest-unsure";
pub const ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID: &str = "atelier-ingest-batch-tags";
pub const ATELIER_INGEST_DATASET_REF_AUTHOR_ID: &str = "atelier-ingest-dataset-ref";
pub const ATELIER_INGEST_CHARACTER_REF_AUTHOR_ID: &str = "atelier-ingest-character-ref";
pub const ATELIER_INGEST_ACTOR_AUTHOR_ID: &str = "atelier-ingest-actor";
pub const ATELIER_INGEST_BATCH_NOTE_AUTHOR_ID: &str = "atelier-ingest-batch-note";
pub const ATELIER_INGEST_EVENT_AUTHOR_ID: &str = "atelier-ingest-event";
pub const ATELIER_INGEST_DATE_AUTHOR_ID: &str = "atelier-ingest-date";
pub const ATELIER_INGEST_LOCATION_AUTHOR_ID: &str = "atelier-ingest-location";
pub const ATELIER_INGEST_LINK_PASSED_AUTHOR_ID: &str = "atelier-ingest-link-passed";
pub const ATELIER_INGEST_APPLY_BATCH_AUTHOR_ID: &str = "atelier-ingest-apply-batch";
pub const ATELIER_INGEST_CONTACT_ROWS_AUTHOR_ID: &str = "atelier-ingest-contact-rows";
pub const ATELIER_INGEST_CONTACT_COLUMNS_AUTHOR_ID: &str = "atelier-ingest-contact-columns";
pub const ATELIER_INGEST_CONTACT_DPI_AUTHOR_ID: &str = "atelier-ingest-contact-dpi";
pub const ATELIER_INGEST_CONTACT_LABELS_AUTHOR_ID: &str = "atelier-ingest-contact-labels";
pub const ATELIER_INGEST_CONTACT_FIT_AUTHOR_ID: &str = "atelier-ingest-contact-fit";
pub const ATELIER_INGEST_CONTACT_OUTPUT_AUTHOR_ID: &str = "atelier-ingest-contact-output";
pub const ATELIER_INGEST_CONTACT_EXPORT_AUTHOR_ID: &str = "atelier-ingest-contact-export";
pub const ATELIER_INGEST_CONTACT_PREVIEW_AUTHOR_ID: &str = "atelier-ingest-contact-preview";
pub const ATELIER_INGEST_CONTACT_RECEIPT_AUTHOR_ID: &str = "atelier-ingest-contact-receipt";
pub const ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID: &str = "atelier-ingest-facial-profile";
pub const ATELIER_INGEST_FACIAL_ANALYZE_AUTHOR_ID: &str = "atelier-ingest-facial-analyze";
pub const ATELIER_INGEST_FACIAL_SUMMARY_AUTHOR_ID: &str = "atelier-ingest-facial-summary";
pub const ATELIER_INGEST_FACIAL_RECEIPT_AUTHOR_ID: &str = "atelier-ingest-facial-receipt";
pub const ATELIER_INGEST_QUEUE_READOUT_AUTHOR_ID: &str = "atelier-ingest-queue-readout";
pub const ATELIER_INGEST_BATCH_SUMMARY_AUTHOR_ID: &str = "atelier-ingest-batch-summary";
pub const ATELIER_INGEST_STATUS_AUTHOR_ID: &str = "atelier-ingest-status";
pub const ATELIER_INGEST_LAST_RECEIPT_AUTHOR_ID: &str = "atelier-ingest-last-receipt";

// MT-030 Facial Review sub-mode (nested under INGEST): the review-queue / montage / export command
// family (backend MT-028/MT-029) that had backend + backend-manual coverage but no frontend surface.
// The INGEST sub-mode selector button for Facial Review.
pub const ATELIER_INGEST_MODE_FACIAL_REVIEW_AUTHOR_ID: &str = "atelier-ingest-mode-facial-review";
// Facial capability/health registry (GET /atelier/facial/features).
pub const ATELIER_INGEST_FACIAL_FEATURES_LOAD_AUTHOR_ID: &str =
    "atelier-ingest-facial-features-load";
pub const ATELIER_INGEST_FACIAL_FEATURES_READOUT_AUTHOR_ID: &str =
    "atelier-ingest-facial-features-readout";
// Identity/quality/dedupe provenance readout (real vs unavailable), sourced from the last analysis.
pub const ATELIER_INGEST_FACIAL_PROVENANCE_AUTHOR_ID: &str = "atelier-ingest-facial-provenance";
// Shared last-command receipt / degraded readout for the review command family.
pub const ATELIER_INGEST_FACIAL_COMMAND_RECEIPT_AUTHOR_ID: &str =
    "atelier-ingest-facial-command-receipt";
// Accumulated review lineage refs (session + claim + decision refs) a model chains commands with.
pub const ATELIER_INGEST_FACIAL_REVIEW_REFS_AUTHOR_ID: &str = "atelier-ingest-facial-review-refs";
// Session create controls.
pub const ATELIER_INGEST_FACIAL_SHARD_COUNT_AUTHOR_ID: &str = "atelier-ingest-facial-shard-count";
pub const ATELIER_INGEST_FACIAL_TTL_AUTHOR_ID: &str = "atelier-ingest-facial-claim-ttl";
pub const ATELIER_INGEST_FACIAL_SESSION_START_AUTHOR_ID: &str =
    "atelier-ingest-facial-session-start";
pub const ATELIER_INGEST_FACIAL_SESSION_REF_AUTHOR_ID: &str = "atelier-ingest-facial-session-ref";
pub const ATELIER_INGEST_FACIAL_SESSION_ID_AUTHOR_ID: &str = "atelier-ingest-facial-session-id";
// Claim controls.
pub const ATELIER_INGEST_FACIAL_CLAIM_SHARD_AUTHOR_ID: &str = "atelier-ingest-facial-claim-shard";
pub const ATELIER_INGEST_FACIAL_CLAIM_STEAL_AUTHOR_ID: &str = "atelier-ingest-facial-claim-steal";
pub const ATELIER_INGEST_FACIAL_CLAIM_START_AUTHOR_ID: &str = "atelier-ingest-facial-claim-start";
pub const ATELIER_INGEST_FACIAL_CLAIM_REF_AUTHOR_ID: &str = "atelier-ingest-facial-claim-ref";
// Decision controls.
pub const ATELIER_INGEST_FACIAL_DECISION_ITEM_AUTHOR_ID: &str =
    "atelier-ingest-facial-decision-item";
pub const ATELIER_INGEST_FACIAL_DECISION_VERDICT_AUTHOR_ID: &str =
    "atelier-ingest-facial-decision-verdict";
pub const ATELIER_INGEST_FACIAL_DECISION_REASON_AUTHOR_ID: &str =
    "atelier-ingest-facial-decision-reason";
pub const ATELIER_INGEST_FACIAL_DECISION_TAGS_AUTHOR_ID: &str =
    "atelier-ingest-facial-decision-tags";
pub const ATELIER_INGEST_FACIAL_DECISION_NOTES_AUTHOR_ID: &str =
    "atelier-ingest-facial-decision-notes";
pub const ATELIER_INGEST_FACIAL_DECISION_RECORD_AUTHOR_ID: &str =
    "atelier-ingest-facial-decision-record";
// Status replay control + readout.
pub const ATELIER_INGEST_FACIAL_STATUS_REPLAY_AUTHOR_ID: &str =
    "atelier-ingest-facial-status-replay";
pub const ATELIER_INGEST_FACIAL_REVIEW_STATUS_READOUT_AUTHOR_ID: &str =
    "atelier-ingest-facial-review-status";
pub const ATELIER_INGEST_FACIAL_MONTAGE_READOUT_AUTHOR_ID: &str =
    "atelier-ingest-facial-montage-readout";
pub const ATELIER_INGEST_FACIAL_EXPORT_READOUT_AUTHOR_ID: &str =
    "atelier-ingest-facial-export-readout";
pub const ATELIER_INGEST_FACIAL_PARSE_NOTES_AUTHOR_ID: &str = "atelier-ingest-facial-parse-notes";
// Montage controls.
pub const ATELIER_INGEST_FACIAL_MONTAGE_PAGE_AUTHOR_ID: &str = "atelier-ingest-facial-montage-page";
pub const ATELIER_INGEST_FACIAL_MONTAGE_COLUMNS_AUTHOR_ID: &str =
    "atelier-ingest-facial-montage-columns";
pub const ATELIER_INGEST_FACIAL_MONTAGE_ROWS_AUTHOR_ID: &str = "atelier-ingest-facial-montage-rows";
pub const ATELIER_INGEST_FACIAL_MONTAGE_FILTER_AUTHOR_ID: &str =
    "atelier-ingest-facial-montage-filter";
pub const ATELIER_INGEST_FACIAL_MONTAGE_BUILD_AUTHOR_ID: &str =
    "atelier-ingest-facial-montage-build";
// Export controls.
pub const ATELIER_INGEST_FACIAL_EXPORT_DATASET_AUTHOR_ID: &str =
    "atelier-ingest-facial-export-dataset";
pub const ATELIER_INGEST_FACIAL_EXPORT_REPEATS_AUTHOR_ID: &str =
    "atelier-ingest-facial-export-repeats";
pub const ATELIER_INGEST_FACIAL_EXPORT_OUTPUT_AUTHOR_ID: &str =
    "atelier-ingest-facial-export-output";
pub const ATELIER_INGEST_FACIAL_EXPORT_ALLOW_PARTIAL_AUTHOR_ID: &str =
    "atelier-ingest-facial-export-allow-partial";
pub const ATELIER_INGEST_FACIAL_EXPORT_BUILD_AUTHOR_ID: &str = "atelier-ingest-facial-export-build";

pub fn ingest_item_row_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}",
        crate::project_tree::stable_part(item_id)
    )
}

pub fn ingest_item_pass_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}-pass",
        crate::project_tree::stable_part(item_id)
    )
}

pub fn ingest_item_reject_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}-reject",
        crate::project_tree::stable_part(item_id)
    )
}

pub fn ingest_item_unsure_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}-unsure",
        crate::project_tree::stable_part(item_id)
    )
}

// --- MT-020 prompt-feedback sub-mode (nested under INGEST) -----------------

/// Seed rule pack id (mirrors `handshake_core::atelier::prompt_feedback::engine`).
pub const PROMPT_FEEDBACK_SEED_RULE_PACK_ID: &str = "prompt-feedback.seed";

pub const ATELIER_PROMPTFEEDBACK_MODE_INGEST_AUTHOR_ID: &str = "atelier-promptfeedback-mode-ingest";
pub const ATELIER_PROMPTFEEDBACK_MODE_FEEDBACK_AUTHOR_ID: &str =
    "atelier-promptfeedback-mode-feedback";
pub const ATELIER_PROMPTFEEDBACK_HEADER_AUTHOR_ID: &str = "atelier-promptfeedback-header";
pub const ATELIER_PROMPTFEEDBACK_PROJECT_AUTHOR_ID: &str = "atelier-promptfeedback-project";
pub const ATELIER_PROMPTFEEDBACK_ADAPTER_AUTHOR_ID: &str = "atelier-promptfeedback-adapter";
pub const ATELIER_PROMPTFEEDBACK_ROWS_AUTHOR_ID: &str = "atelier-promptfeedback-rows";
pub const ATELIER_PROMPTFEEDBACK_IMPORT_AUTHOR_ID: &str = "atelier-promptfeedback-import";
pub const ATELIER_PROMPTFEEDBACK_RULEPACK_AUTHOR_ID: &str = "atelier-promptfeedback-rulepack";
pub const ATELIER_PROMPTFEEDBACK_CASE_LIST_AUTHOR_ID: &str = "atelier-promptfeedback-case-list";
pub const ATELIER_PROMPTFEEDBACK_REWRITE_AUTHOR_ID: &str = "atelier-promptfeedback-rewrite";
pub const ATELIER_PROMPTFEEDBACK_REWRITE_PREVIEW_AUTHOR_ID: &str =
    "atelier-promptfeedback-rewrite-preview";
pub const ATELIER_PROMPTFEEDBACK_TRACE_AUTHOR_ID: &str = "atelier-promptfeedback-trace";
pub const ATELIER_PROMPTFEEDBACK_EXPORT_AUTHOR_ID: &str = "atelier-promptfeedback-export";
pub const ATELIER_PROMPTFEEDBACK_EXPORT_STATUS_AUTHOR_ID: &str =
    "atelier-promptfeedback-export-status";
pub const ATELIER_PROMPTFEEDBACK_STATUS_AUTHOR_ID: &str = "atelier-promptfeedback-status";

/// Fixed failure-tag vocabulary for the picker (handoff failure classes: technical
/// artifacts, bland, scene incoherence, target-blocking outfits, runner leakage).
pub const PROMPT_FEEDBACK_FAILURE_TAGS: &[&str] = &[
    "bad_hands",
    "smeared_fingers",
    "broken_limbs",
    "bad_genital_detail",
    "plastic_skin",
    "face_smear",
    "bland",
    "generic_nude",
    "incoherent_wet_scene",
    "target_blocked_by_outfit",
    "protected_runner_leakage",
    "action_claim_without_contact_proof",
];

pub fn prompt_feedback_case_row_author_id(case_key: &str) -> String {
    format!(
        "atelier-promptfeedback-case-{}",
        crate::project_tree::stable_part(case_key)
    )
}

pub fn prompt_feedback_failure_tag_author_id(tag: &str) -> String {
    format!(
        "atelier-promptfeedback-tag-{}",
        crate::project_tree::stable_part(tag)
    )
}

/// A minimal display row for the prompt-feedback case list (grouped by
/// segment/cell/render-stack). Seeded for the WIP surface; the live backend list
/// arrives via `AtelierClient::prompt_feedback_cases_request`.
#[derive(Debug, Clone)]
pub struct PromptFeedbackCaseRow {
    pub case_id: String,
    pub project_id: String,
    pub source_case_id: String,
    pub segment: String,
    pub cell: String,
    pub render_stack: String,
    pub identity_judgement_allowed: bool,
    pub prompt_quality_review_allowed: bool,
    pub failure_tags: Vec<String>,
    pub imported_by: String,
    pub persistence_source: String,
}

fn seeded_prompt_feedback_cases() -> Vec<PromptFeedbackCaseRow> {
    vec![
        PromptFeedbackCaseRow {
            case_id: "no_detail:0_closeup:1".to_owned(),
            project_id: "leeseo".to_owned(),
            source_case_id: "no_detail:0_closeup:1".to_owned(),
            segment: "standard".to_owned(),
            cell: "0_closeup".to_owned(),
            render_stack: "no_detail".to_owned(),
            identity_judgement_allowed: true,
            prompt_quality_review_allowed: true,
            failure_tags: Vec::new(),
            imported_by: "local-preview".to_owned(),
            persistence_source: "local_preview".to_owned(),
        },
        PromptFeedbackCaseRow {
            case_id: "with_detail_faceid:0_closeup:1".to_owned(),
            project_id: "leeseo".to_owned(),
            source_case_id: "with_detail_faceid:0_closeup:1".to_owned(),
            segment: "prompt_stress".to_owned(),
            cell: "0_closeup".to_owned(),
            render_stack: "FaceDetailer+FaceID".to_owned(),
            identity_judgement_allowed: false,
            prompt_quality_review_allowed: true,
            failure_tags: Vec::new(),
            imported_by: "local-preview".to_owned(),
            persistence_source: "local_preview".to_owned(),
        },
    ]
}

fn prompt_feedback_case_row_from_backend(
    row: BackendPromptFeedbackCaseRow,
) -> PromptFeedbackCaseRow {
    PromptFeedbackCaseRow {
        case_id: row.case_id,
        project_id: row.project_id,
        source_case_id: row.source_case_id,
        segment: row.segment,
        cell: row.cell,
        render_stack: row.render_stack,
        identity_judgement_allowed: row.identity_judgement_allowed,
        prompt_quality_review_allowed: row.prompt_quality_review_allowed,
        failure_tags: row.failure_tags,
        imported_by: row.imported_by,
        persistence_source: "backend_persisted".to_owned(),
    }
}

fn prompt_feedback_source_system(project_id: &str, adapter_id: &str) -> String {
    adapter_id
        .split('.')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(project_id.trim())
        .to_owned()
}

fn prompt_feedback_import_fingerprint(project_id: &str, adapter_id: &str, rows: &str) -> String {
    stable_model_ops_thread_segment(&format!(
        "project_id={}\nadapter_id={}\nrows={}",
        project_id.trim(),
        adapter_id.trim(),
        rows.trim()
    ))
}

fn prompt_feedback_rewrite_fingerprint(
    rule_pack_id: &str,
    selected_case_id: Option<&str>,
    selected_tags: &BTreeSet<String>,
) -> String {
    stable_model_ops_thread_segment(&format!(
        "rule_pack_id={}\nselected_case_id={}\nselected_tags={}",
        rule_pack_id.trim(),
        selected_case_id.unwrap_or("<none>"),
        selected_tags.iter().cloned().collect::<Vec<_>>().join("|")
    ))
}

fn prompt_feedback_export_case_ids(
    cases: &[PromptFeedbackCaseRow],
    selected_case_id: Option<&str>,
) -> Vec<String> {
    let mut case_ids = Vec::new();
    if let Some(selected_case_id) = selected_case_id {
        if let Some(case) = cases
            .iter()
            .find(|case| case.case_id == selected_case_id)
            .filter(|case| case.persistence_source == "backend_persisted")
        {
            case_ids.push(case.case_id.clone());
        }
    }
    if case_ids.is_empty() {
        case_ids = cases
            .iter()
            .filter(|case| case.persistence_source == "backend_persisted")
            .map(|case| case.case_id.clone())
            .collect();
    }
    case_ids
}

fn prompt_feedback_export_fingerprint(rule_pack_id: &str, case_ids: &[String]) -> String {
    let mut sorted_case_ids = case_ids.to_vec();
    sorted_case_ids.sort();
    stable_model_ops_thread_segment(&format!(
        "rule_pack_id={}\ncase_ids={}",
        rule_pack_id.trim(),
        sorted_case_ids.join("|")
    ))
}

fn parse_prompt_feedback_rows_input(input: &str) -> Result<serde_json::Value, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("rows input is empty".to_owned());
    }
    if trimmed.starts_with('[') {
        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|err| format!("invalid JSON rows array: {err}"))?;
        if value.as_array().is_none() {
            return Err("rows JSON must be an array or newline-delimited objects".to_owned());
        }
        return Ok(value);
    }
    let mut rows = Vec::new();
    for (index, line) in trimmed.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|err| format!("invalid JSONL row {}: {err}", index + 1))?;
        if !value.is_object() {
            return Err(format!("JSONL row {} must be an object", index + 1));
        }
        rows.push(value);
    }
    if rows.is_empty() {
        return Err("rows input has no JSON objects".to_owned());
    }
    Ok(serde_json::Value::Array(rows))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtelierPanelTab {
    CastkitCodex,
    Posekit,
    Ingest,
}

impl AtelierPanelTab {
    pub const ALL: [Self; 3] = [Self::CastkitCodex, Self::Posekit, Self::Ingest];

    fn label(self) -> &'static str {
        match self {
            Self::CastkitCodex => "CKC",
            Self::Posekit => "Posekit",
            Self::Ingest => "Ingest",
        }
    }

    fn tab_author_id(self) -> &'static str {
        match self {
            Self::CastkitCodex => ATELIER_TAB_CKC_AUTHOR_ID,
            Self::Posekit => ATELIER_TAB_POSEKIT_AUTHOR_ID,
            Self::Ingest => ATELIER_TAB_INGEST_AUTHOR_ID,
        }
    }

    fn content_author_id(self) -> &'static str {
        match self {
            Self::CastkitCodex => ATELIER_CONTENT_CKC_AUTHOR_ID,
            Self::Posekit => ATELIER_CONTENT_POSEKIT_AUTHOR_ID,
            Self::Ingest => ATELIER_CONTENT_INGEST_AUTHOR_ID,
        }
    }

    /// WP-CKC MT-042 stable token persisted as the `atelier-ui.landing-tab` default.
    fn settings_token(self) -> &'static str {
        match self {
            Self::CastkitCodex => "castkit-codex",
            Self::Posekit => "posekit",
            Self::Ingest => "ingest",
        }
    }

    fn from_settings_token(token: &str) -> Option<Self> {
        match token.trim() {
            "castkit-codex" => Some(Self::CastkitCodex),
            "posekit" => Some(Self::Posekit),
            "ingest" => Some(Self::Ingest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CkcBookMode {
    Sheet,
    Story,
    Notes,
    Moodboard,
}

impl CkcBookMode {
    const ALL: [Self; 4] = [Self::Sheet, Self::Story, Self::Notes, Self::Moodboard];

    fn label(self) -> &'static str {
        match self {
            Self::Sheet => "Sheet",
            Self::Story => "Story",
            Self::Notes => "Notes",
            Self::Moodboard => "Moodboard",
        }
    }

    fn author_id(self) -> &'static str {
        match self {
            Self::Sheet => ATELIER_CKC_MODE_SHEET_AUTHOR_ID,
            Self::Story => ATELIER_CKC_MODE_STORY_AUTHOR_ID,
            Self::Notes => ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
            Self::Moodboard => ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID,
        }
    }

    fn middle_label(self) -> &'static str {
        match self {
            Self::Sheet => "No middle panel",
            Self::Story => "Story work surface",
            Self::Notes => "Character sheet notes",
            Self::Moodboard => "Moodboard work surface",
        }
    }

    fn has_middle_panel(self) -> bool {
        !matches!(self, Self::Sheet)
    }

    /// WP-CKC MT-042 stable token persisted as the `ckc.book-mode` default.
    fn settings_token(self) -> &'static str {
        match self {
            Self::Sheet => "sheet",
            Self::Story => "story",
            Self::Notes => "notes",
            Self::Moodboard => "moodboard",
        }
    }

    fn from_settings_token(token: &str) -> Option<Self> {
        match token.trim() {
            "sheet" => Some(Self::Sheet),
            "story" => Some(Self::Story),
            "notes" => Some(Self::Notes),
            "moodboard" => Some(Self::Moodboard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CkcBookPageWidths {
    left: f32,
    middle: f32,
    right: f32,
}

fn ckc_book_page_widths(available_width: f32, has_middle: bool) -> CkcBookPageWidths {
    let available = available_width.max(1.0);
    let gap_budget = if has_middle { 32.0 } else { 16.0 };
    let usable = (available - gap_budget).max(1.0);

    if has_middle {
        if usable < 760.0 {
            let page = usable.max(1.0);
            return CkcBookPageWidths {
                left: 220.0_f32.min(page),
                middle: 260.0_f32.min(page),
                right: 300.0_f32.min(page),
            };
        }
        let left = (usable * 0.30).clamp(220.0, 340.0);
        let middle = (usable * 0.30).clamp(260.0, 430.0);
        let right = (usable - left - middle).max(260.0);
        if left + middle + right <= usable {
            CkcBookPageWidths {
                left,
                middle,
                right,
            }
        } else {
            CkcBookPageWidths {
                left: usable * 0.30,
                middle: usable * 0.32,
                right: usable * 0.38,
            }
        }
    } else {
        if usable < 700.0 {
            return CkcBookPageWidths {
                left: usable * 0.42,
                middle: 0.0,
                right: usable * 0.58,
            };
        }
        let left = (usable * 0.42).clamp(280.0, 460.0);
        let right = (usable - left).max(300.0);
        if left + right <= usable {
            CkcBookPageWidths {
                left,
                middle: 0.0,
                right,
            }
        } else {
            CkcBookPageWidths {
                left: usable * 0.42,
                middle: 0.0,
                right: usable * 0.58,
            }
        }
    }
}

fn constrain_ckc_book_page(ui: &mut egui::Ui, width: f32) {
    ui.set_min_width(width);
    ui.set_max_width(width);
    ui.set_width(width);
}

fn compact_visual_cell(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let tail_len = max_chars.saturating_sub(3).max(1);
    let tail_rev: String = value.chars().rev().take(tail_len).collect();
    let tail: String = tail_rev.chars().rev().collect();
    format!("...{tail}")
}

/// MT-020 + MT-030: the INGEST tab sub-mode. Nested UNDER `AtelierPanelTab::Ingest` (NOT a 4th
/// top-level tab — `AtelierPanelTab::ALL` is asserted as `[Self; 3]`). Supersedes the earlier MT-020
/// `prompt_feedback_mode: bool` toggle with a three-way selector so MT-030 can add the Facial Review
/// review-queue/montage/export surface as a peer sub-mode without a new top-level tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IngestMode {
    DatasetIntake,
    PromptFeedback,
    FacialReview,
}

impl IngestMode {
    const ALL: [Self; 3] = [
        Self::DatasetIntake,
        Self::PromptFeedback,
        Self::FacialReview,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::DatasetIntake => "Dataset intake",
            Self::PromptFeedback => "Prompt feedback",
            Self::FacialReview => "Facial review",
        }
    }

    fn author_id(self) -> &'static str {
        match self {
            Self::DatasetIntake => ATELIER_PROMPTFEEDBACK_MODE_INGEST_AUTHOR_ID,
            Self::PromptFeedback => ATELIER_PROMPTFEEDBACK_MODE_FEEDBACK_AUTHOR_ID,
            Self::FacialReview => ATELIER_INGEST_MODE_FACIAL_REVIEW_AUTHOR_ID,
        }
    }
}

const POSEKIT_EXPORT_WIDTH: i32 = 768;
const POSEKIT_EXPORT_HEIGHT: i32 = 768;
const POSEKIT_BODY_KEYPOINT_COUNT: usize = 18;
const POSEKIT_FACE_KEYPOINT_COUNT: usize = 70;
const POSEKIT_HAND_KEYPOINT_COUNT: usize = 21;

#[derive(Debug, Clone)]
struct PosekitExportSnapshot {
    source_ref: String,
    rig_id: Option<String>,
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
    face: bool,
    body: bool,
    hands: bool,
    png_artifact_ref: String,
    png_manifest_ref: String,
    json_artifact_ref: String,
    json_manifest_ref: String,
    receipt_ref: String,
    content_hash: String,
    openpose_json: serde_json::Value,
    framing: serde_json::Value,
    applied_marker_edit_count: usize,
}

impl PosekitExportSnapshot {
    fn marker_layers(&self) -> String {
        marker_layer_summary(self.face, self.body, self.hands)
    }
}

/// Panel-owned decode cache for the Posekit source image (left viewport). Holds the raw source
/// bytes (PNG/JPEG/WebP/GIF) plus the decoded `egui::TextureHandle` so the real image is uploaded to
/// the GPU once per byte change rather than every frame. Default = no source loaded (empty state).
///
/// MT-014 de-scaffold: this replaces the previous hash-derived colour tile. The left viewport now
/// shows the REAL decoded source image when bytes are present, or an explicit empty state — never a
/// fabricated tile. The bytes are supplied by [`AtelierPanel::set_pose_source_image_bytes`]; wiring a
/// DAM/`ckc_client` fetch that populates them from `source_asset_id` is tracked as follow-up (there is
/// no source-asset byte-fetch route in the frontend client yet).
#[derive(Default)]
struct PoseSourceImageCache {
    bytes: Option<Vec<u8>>,
    hash: Option<u64>,
    texture: Option<egui::TextureHandle>,
    dims: Option<(usize, usize)>,
    error: Option<String>,
}

const CKC_MEDIA_IMAGE_CACHE_ENTRY_LIMIT: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierImageCacheDebugSnapshot {
    pub ckc_entry_limit: usize,
    pub ckc_entry_count: usize,
    pub ckc_asset_ids: Vec<String>,
    pub pose_source_slot_limit: usize,
    pub pose_source_has_bytes: bool,
    pub pose_source_has_texture: bool,
}

/// What the left (source-image) viewport should paint this frame — a real decoded texture, an
/// explicit decode error, or an explicit empty state. There is deliberately no "fake tile" variant.
/// Reused by the CKC linked-media viewer (MT-010 de-scaffold) — the two viewports paint identical
/// states (real texture / empty / decode error).
enum PoseSourceRender {
    Empty,
    Loaded {
        texture: egui::TextureHandle,
        width: usize,
        height: usize,
        byte_len: usize,
    },
    DecodeError(String),
}

/// MT-010 de-scaffold: decode cache for CKC linked-media images, keyed by media-asset UUID so the
/// selected-image viewer paints the REAL decoded image (never a placeholder tile) and re-selecting a
/// previously fetched asset repaints instantly without a refetch. Each entry mirrors
/// [`PoseSourceImageCache`] (bytes + hash + texture + dims + error) and is decoded/uploaded lazily on
/// first paint. Interior-mutable so the `&self` render path can (re)upload a texture when bytes change.
#[derive(Default)]
struct CkcMediaImageCache {
    entries: BTreeMap<String, CkcMediaImageEntry>,
    access_counter: u64,
}

impl CkcMediaImageCache {
    fn next_access(&mut self) -> u64 {
        self.access_counter = self.access_counter.saturating_add(1);
        self.access_counter
    }

    fn evict_to_limit(&mut self, protected_asset_id: Option<&str>) {
        while self.entries.len() > CKC_MEDIA_IMAGE_CACHE_ENTRY_LIMIT {
            let Some(victim) = self
                .entries
                .iter()
                .filter(|(asset_id, _)| Some(asset_id.as_str()) != protected_asset_id)
                .min_by(|(left_id, left), (right_id, right)| {
                    left.last_access
                        .cmp(&right.last_access)
                        .then_with(|| left_id.cmp(right_id))
                })
                .map(|(asset_id, _)| asset_id.clone())
            else {
                break;
            };
            self.entries.remove(&victim);
        }
    }
}

/// One cached CKC linked-media image (per asset). Mirrors [`PoseSourceImageCache`] field-for-field.
#[derive(Default)]
struct CkcMediaImageEntry {
    bytes: Option<Vec<u8>>,
    hash: Option<u64>,
    texture: Option<egui::TextureHandle>,
    dims: Option<(usize, usize)>,
    error: Option<String>,
    last_access: u64,
}

#[derive(Debug, Clone)]
struct ContactSheetExportSnapshot {
    source_kind: String,
    source_ref: String,
    rows: usize,
    columns: usize,
    dpi: usize,
    include_labels: bool,
    thumbnail_fit: String,
    output_path: Option<String>,
    item_count: usize,
    rendered_item_count: usize,
    omitted_item_count: usize,
    svg_artifact_ref: String,
    svg_manifest_ref: String,
    receipt_ref: String,
    receipt_manifest_ref: String,
    svg_sha256: String,
    receipt_sha256: String,
    content_hash: String,
    source_lineage_preview: String,
}

#[derive(Debug, Clone)]
struct FacialIngestAnalysisSnapshot {
    batch_id: String,
    profile: String,
    profile_tokens: String,
    item_count: usize,
    decoded_count: usize,
    duplicate_group_count: usize,
    duplicate_item_count: usize,
    keep_count: usize,
    review_count: usize,
    cull_count: usize,
    quality_source: String,
    identity_source: String,
    dedupe_source: String,
    analysis_artifact_ref: String,
    analysis_manifest_ref: String,
    receipt_ref: String,
    receipt_manifest_ref: String,
    analysis_sha256: String,
    receipt_sha256: String,
    content_hash: String,
}

#[derive(Debug, Clone)]
struct PosekitMarkerEditRecord {
    family: String,
    index: usize,
    action: String,
    x: Option<f32>,
    y: Option<f32>,
    confidence: Option<f32>,
}

#[derive(Debug, Clone)]
struct CkcCharacterRecord {
    public_id: String,
    display_name: String,
    character_internal_id: String,
    character_ref: String,
    sheet_version_id: Option<String>,
    parent_sheet_version_id: Option<String>,
    sheet_seq: i64,
    sheet_editor_text: String,
    sheet_version_ref: Option<String>,
    sheet_artifact_links: Vec<CkcSheetArtifactLinkRecord>,
    media_album_offset: i64,
    media_album_limit: i64,
    media_album_member_limit: i64,
    media_album_count: usize,
    media_albums_next_offset: Option<i64>,
    media_albums: Vec<CkcMediaAlbumRecord>,
    story_documents: Vec<CkcStoryDocumentRecord>,
    moodboard_documents: Vec<CkcMoodboardDocumentRecord>,
}

#[derive(Debug, Clone)]
struct CkcSheetArtifactLinkRecord {
    link_id: String,
    character_internal_id: String,
    character_ref: String,
    sheet_version_id: String,
    sheet_version_ref: String,
    typed_ref: String,
    artifact_kind: String,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    linked_by: String,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
struct CkcMediaAlbumRecord {
    collection_id: String,
    collection_ref: String,
    name: String,
    description: String,
    tags: Vec<String>,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<CkcMediaMemberRecord>,
    created_by: String,
    updated_by: String,
}

#[derive(Debug, Clone)]
struct CkcMediaMemberRecord {
    asset_id: String,
    media_ref: String,
    display_label: String,
    sort_order: i64,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    link_source_path_ref: Option<String>,
    link_source_url_ref: Option<String>,
    link_source_path_ref_status: String,
    link_source_url_ref_status: String,
    asset_source_path_ref_status: String,
    asset_source_url_ref_status: String,
    source_path_ref_origin: String,
    source_url_ref_origin: String,
    linked_by: String,
    member_updated_by: String,
    member_updated_at_utc: Option<String>,
    notes_updated_by: String,
    notes_updated_at_utc: Option<String>,
    notes: String,
    review_status: Option<String>,
    tags_buffer: String,
}

#[derive(Debug, Clone)]
struct CkcStoryDocumentRecord {
    document_id: String,
    document_ref: String,
    current_version_id: Option<String>,
    current_version_seq: i64,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    cards: Vec<CkcStoryCardRecord>,
    beats: Vec<CkcStoryBeatRecord>,
}

#[derive(Debug, Clone)]
struct CkcStoryCardRecord {
    card_id: String,
    card_ref: String,
    story_document_id: String,
    story_document_ref: String,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct CkcStoryBeatRecord {
    beat_id: String,
    beat_ref: String,
    story_document_id: String,
    story_document_ref: String,
    card_id: Option<String>,
    beat_text: String,
}

#[derive(Debug, Clone)]
struct CkcMoodboardDocumentRecord {
    document_id: String,
    document_ref: String,
    current_version_id: Option<String>,
    current_version_seq: i64,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    latest_snapshot_id: Option<String>,
    latest_snapshot_ref: Option<String>,
    moodboard_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CkcMoodboardCanvasProjection {
    pub placements: Vec<CanvasPlacementCard>,
    pub visual_edges: Vec<VisualEdge>,
    pub section_labels: BTreeMap<String, String>,
    pub pan: egui::Vec2,
    pub zoom: f32,
}

impl CkcMoodboardCanvasProjection {
    fn apply_to_board(self, board: &mut LoomCanvasBoard, snapshot_ref: &str) {
        board.set_section_labels(self.section_labels);
        board.set_board(self.placements, self.visual_edges, self.pan, self.zoom);
        board.status = format!(
            "CKC moodboard snapshot loaded: {snapshot_ref}; add/place/connect/group/remove controls hidden because this canvas persists existing CKC moodboard positions and image/shape sizes only."
        );
    }
}

fn apply_ckc_moodboard_snapshot_to_board(
    canvas_board: &Arc<Mutex<LoomCanvasBoard>>,
    raw_json_text: &str,
    snapshot_ref: &str,
) -> Result<(), String> {
    let projection = ckc_moodboard_snapshot_to_canvas_projection(raw_json_text)?;
    let mut board = canvas_board
        .lock()
        .map_err(|err| format!("CKC moodboard canvas lock failed: {err}"))?;
    projection.apply_to_board(&mut board, snapshot_ref);
    Ok(())
}

#[derive(Debug, Clone)]
struct CkcMediaSaveRequest {
    asset_id: String,
    notes: String,
    tags: Vec<String>,
    review_status: Option<String>,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcAlbumCreateRequest {
    character_internal_id: String,
    name: String,
    notes: Option<String>,
    sheet_version_id: Option<String>,
    tags: Vec<String>,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcAlbumLinkAssetsRequest {
    collection_id: String,
    asset_ids: Vec<String>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcAlbumUnlinkRequest {
    collection_id: String,
    asset_id: String,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcAlbumLinkEditRequest {
    collection_id: String,
    asset_id: String,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    clear_source_path_ref: bool,
    clear_source_url_ref: bool,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcAlbumReorderRequest {
    collection_id: String,
    items: Vec<(String, i64)>,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcAlbumPageRequest {
    collection_id: String,
    offset: i64,
}

#[derive(Debug, Clone)]
struct CkcAlbumListPageRequest {
    character_internal_id: String,
    offset: i64,
}

#[derive(Debug, Clone)]
struct CkcSheetArtifactAttachRequest {
    sheet_version_id: String,
    artifact_kind: String,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    metadata: serde_json::Value,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcSheetArtifactDetachRequest {
    sheet_version_id: String,
    link_id: String,
    actor_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CkcSearchMode {
    Fuzzy,
    Vector,
    Combined,
}

impl CkcSearchMode {
    const ALL: [Self; 3] = [Self::Fuzzy, Self::Vector, Self::Combined];

    fn label(self) -> &'static str {
        match self {
            Self::Fuzzy => "Fuzzy",
            Self::Vector => "Vector",
            Self::Combined => "Combined",
        }
    }

    fn backend_value(self) -> &'static str {
        match self {
            Self::Fuzzy => "fuzzy",
            Self::Vector => "vector",
            Self::Combined => "combined",
        }
    }

    fn author_id(self) -> &'static str {
        match self {
            Self::Fuzzy => ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID,
            Self::Vector => ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID,
            Self::Combined => ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
        }
    }
}

#[derive(Debug, Clone)]
struct CkcSearchResultRecord {
    target_kind: String,
    target_ref: String,
    title: String,
    snippet: String,
    character_ref: Option<String>,
    sheet_version_ref: Option<String>,
    collection_ref: Option<String>,
    media_ref: Option<String>,
    tag_ref: Option<String>,
    tags: Vec<String>,
    tag_notes: Vec<CkcTagNoteRecord>,
    match_modes: Vec<String>,
}

#[derive(Debug, Clone)]
struct CkcTagNoteRecord {
    tag_ref: String,
    tag_text: String,
    scope_ref: Option<String>,
    note: String,
}

#[derive(Debug, Clone)]
struct CkcTagNoteSaveRequest {
    tag_text: String,
    scope_ref: Option<String>,
    note: String,
}

#[derive(Debug, Clone, Default)]
struct CkcSearchFilterRefs {
    character_internal_id: Option<String>,
    character_ref: Option<String>,
    collection_id: Option<String>,
    collection_ref: Option<String>,
    media_asset_id: Option<String>,
    media_ref: Option<String>,
}

fn selected_ckc_search_filter_refs(state: &AtelierPanelState) -> CkcSearchFilterRefs {
    let Some(character) = state.ckc_characters.get(
        state
            .ckc_selected_index
            .min(state.ckc_characters.len().saturating_sub(1)),
    ) else {
        return CkcSearchFilterRefs::default();
    };
    let mut refs = CkcSearchFilterRefs {
        character_internal_id: Some(character.character_internal_id.clone()),
        character_ref: Some(character.character_ref()),
        ..Default::default()
    };
    let media_location = state
        .ckc_selected_media_key
        .as_deref()
        .and_then(|media_key| character.media_location(media_key))
        .or_else(|| character.first_media_location());
    if let Some((album_idx, member_idx)) = media_location {
        let album = &character.media_albums[album_idx];
        let member = &album.members[member_idx];
        refs.collection_id = Some(album.collection_id.clone());
        refs.collection_ref = Some(album.collection_ref.clone());
        refs.media_asset_id = Some(member.asset_id.clone());
        refs.media_ref = Some(member.media_ref.clone());
    }
    refs
}

fn ckc_result_has_ref(result: &CkcSearchResultRecord, expected: Option<&str>) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    [
        Some(result.target_ref.as_str()),
        result.character_ref.as_deref(),
        result.sheet_version_ref.as_deref(),
        result.collection_ref.as_deref(),
        result.media_ref.as_deref(),
        result.tag_ref.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|actual| actual == expected)
}

fn ckc_scope_matches_result_refs(
    scope_ref: Option<&str>,
    target_ref: &str,
    character_ref: Option<&str>,
    sheet_version_ref: Option<&str>,
    collection_ref: Option<&str>,
    media_ref: Option<&str>,
    tag_ref: Option<&str>,
) -> bool {
    let Some(scope_ref) = scope_ref else {
        return true;
    };
    [
        Some(target_ref),
        character_ref,
        sheet_version_ref,
        collection_ref,
        media_ref,
        tag_ref,
    ]
    .into_iter()
    .flatten()
    .any(|actual| actual == scope_ref)
}

fn ckc_search_result_matches_filters(
    result: &CkcSearchResultRecord,
    filters: &CkcSearchFilterRefs,
    use_character: bool,
    use_collection: bool,
    use_media: bool,
) -> bool {
    (!use_character || ckc_result_has_ref(result, filters.character_ref.as_deref()))
        && (!use_collection || ckc_result_has_ref(result, filters.collection_ref.as_deref()))
        && (!use_media || ckc_result_has_ref(result, filters.media_ref.as_deref()))
}

impl CkcCharacterRecord {
    fn character_ref(&self) -> String {
        if self.character_ref.is_empty() {
            format!("atelier://character/{}", self.character_internal_id)
        } else {
            self.character_ref.clone()
        }
    }

    fn sheet_version_ref(&self) -> Option<String> {
        self.sheet_version_ref.clone().or_else(|| {
            self.sheet_version_id.as_ref().map(|version_id| {
                format!(
                    "atelier://sheet/{}/{}",
                    self.character_internal_id, version_id
                )
            })
        })
    }

    fn sheet_atelier_ref(&self) -> Option<AtelierRef> {
        self.sheet_version_id.as_ref().map(|sheet_version_id| {
            AtelierRef::character_sheet_version(
                &self.character_internal_id,
                sheet_version_id,
                format!("{} sheet v{}", self.display_name, self.sheet_seq),
            )
        })
    }

    fn from_backend(row: AtelierCkcCharacterSheetRow) -> Self {
        let AtelierCkcCharacterSheetRow {
            character,
            latest_sheet,
            sheet_artifact_links,
            media_album_offset,
            media_album_limit,
            media_album_member_limit,
            media_album_count,
            media_albums_next_offset,
            media_albums,
            story_documents,
            moodboard_documents,
            moodboard_snapshots,
        } = row;
        let (
            sheet_version_id,
            parent_sheet_version_id,
            sheet_seq,
            sheet_editor_text,
            sheet_version_ref,
        ) = latest_sheet
            .map(
                |AtelierSheetVersionRow {
                     version_id,
                     parent_version_id,
                     seq,
                     raw_text,
                     sheet_version_ref,
                     ..
                 }| {
                    (
                        Some(version_id),
                        parent_version_id,
                        seq,
                        raw_text,
                        Some(sheet_version_ref),
                    )
                },
            )
            .unwrap_or_else(|| (None, None, 0, String::new(), None));
        let story_documents = story_documents
            .into_iter()
            .map(CkcStoryDocumentRecord::from_backend)
            .collect();
        let moodboard_documents = moodboard_documents
            .into_iter()
            .map(|document| {
                let snapshot = moodboard_snapshots
                    .iter()
                    .find(|snapshot| snapshot.document_id == document.document_id);
                CkcMoodboardDocumentRecord::from_backend(document, snapshot)
            })
            .collect();
        Self {
            public_id: character.public_id,
            display_name: character.display_name,
            character_internal_id: character.internal_id,
            character_ref: character.character_ref,
            sheet_version_id,
            parent_sheet_version_id,
            sheet_seq,
            sheet_editor_text,
            sheet_version_ref,
            sheet_artifact_links: sheet_artifact_links
                .into_iter()
                .map(CkcSheetArtifactLinkRecord::from_backend)
                .collect(),
            media_album_offset,
            media_album_limit,
            media_album_member_limit,
            media_album_count,
            media_albums_next_offset,
            media_albums: media_albums
                .into_iter()
                .map(CkcMediaAlbumRecord::from_backend)
                .collect(),
            story_documents,
            moodboard_documents,
        }
    }

    fn from_created_character(character: AtelierCharacterRow) -> Self {
        Self {
            public_id: character.public_id,
            display_name: character.display_name.clone(),
            character_internal_id: character.internal_id,
            character_ref: character.character_ref,
            sheet_version_id: None,
            parent_sheet_version_id: None,
            sheet_seq: 0,
            sheet_editor_text: format!(
                "name: {}\nrole: reusable character/avatar\npipelines: ComfyUI, Unreal, Blender\nnotes: ",
                character.display_name
            ),
            sheet_version_ref: None,
            sheet_artifact_links: Vec::new(),
            media_album_offset: 0,
            media_album_limit: 0,
            media_album_member_limit: 0,
            media_album_count: 0,
            media_albums_next_offset: None,
            media_albums: Vec::new(),
            story_documents: Vec::new(),
            moodboard_documents: Vec::new(),
        }
    }

    fn apply_sheet_version(&mut self, sheet: AtelierSheetVersionRow) {
        self.character_internal_id = sheet.character_internal_id;
        self.character_ref = sheet.character_ref;
        self.parent_sheet_version_id = sheet.parent_version_id;
        self.sheet_version_id = Some(sheet.version_id);
        self.sheet_seq = sheet.seq;
        self.sheet_editor_text = sheet.raw_text;
        self.sheet_version_ref = Some(sheet.sheet_version_ref);
        self.sheet_artifact_links.clear();
    }

    fn first_media_location(&self) -> Option<(usize, usize)> {
        self.media_albums
            .iter()
            .enumerate()
            .find_map(|(album_idx, album)| {
                if album.members.is_empty() {
                    None
                } else {
                    Some((album_idx, 0))
                }
            })
    }

    fn media_location(&self, media_key: &str) -> Option<(usize, usize)> {
        self.media_albums
            .iter()
            .enumerate()
            .find_map(|(album_idx, album)| {
                album
                    .members
                    .iter()
                    .position(|member| {
                        ckc_media_occurrence_key(&album.collection_id, &member.asset_id)
                            == media_key
                    })
                    .map(|member_idx| (album_idx, member_idx))
            })
    }

    fn selected_or_first_media_location(
        &self,
        selected_media_key: Option<&str>,
    ) -> Option<(usize, usize)> {
        match selected_media_key {
            Some(media_key) => self.media_location(media_key),
            None => self.first_media_location(),
        }
    }
}

impl CkcSheetArtifactLinkRecord {
    fn from_backend(row: AtelierCkcSheetArtifactLinkRow) -> Self {
        Self {
            link_id: row.link_id,
            character_internal_id: row.character_internal_id,
            character_ref: row.character_ref,
            sheet_version_id: row.sheet_version_id,
            sheet_version_ref: row.sheet_version_ref,
            typed_ref: row.typed_ref,
            artifact_kind: row.artifact_kind,
            artifact_ref: row.artifact_ref,
            manifest_ref: row.manifest_ref,
            source_ref: row.source_ref,
            label: row.label,
            reuse_role: row.reuse_role,
            linked_by: row.linked_by,
            metadata: row.metadata,
        }
    }

    fn local(
        character: &CkcCharacterRecord,
        artifact_kind: String,
        artifact_ref: String,
        manifest_ref: Option<String>,
        source_ref: Option<String>,
        label: Option<String>,
        reuse_role: Option<String>,
        metadata: serde_json::Value,
        actor_id: String,
    ) -> Option<Self> {
        let sheet_version_id = character.sheet_version_id.clone()?;
        let sheet_version_ref = character.sheet_version_ref()?;
        let link_id = Uuid::new_v4().to_string();
        Some(Self {
            link_id: link_id.clone(),
            character_internal_id: character.character_internal_id.clone(),
            character_ref: character.character_ref(),
            sheet_version_id,
            sheet_version_ref,
            typed_ref: format!("atelier://sheet-artifact/{link_id}"),
            artifact_kind,
            artifact_ref,
            manifest_ref,
            source_ref,
            label,
            reuse_role,
            linked_by: actor_id,
            metadata,
        })
    }

    fn summary(&self) -> String {
        format!(
            "{} | {} | {} | {}",
            self.artifact_kind,
            self.reuse_role.as_deref().unwrap_or("reuse"),
            self.typed_ref,
            self.artifact_ref
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CkcSheetArtifactApplyOutcome {
    count: usize,
    current_selection_owns_target: bool,
    target_found: bool,
}

fn apply_ckc_sheet_artifact_link_rows_to_state(
    state: &mut AtelierPanelState,
    target_sheet_version_id: &str,
    rows: Vec<AtelierCkcSheetArtifactLinkRow>,
) -> CkcSheetArtifactApplyOutcome {
    let selected_index = state.ckc_selected_index;
    let current_selection_owns_target = state
        .ckc_characters
        .get(selected_index)
        .and_then(|character| character.sheet_version_id.as_deref())
        == Some(target_sheet_version_id);
    let selected_link_id = state.ckc_selected_sheet_artifact_link_id.clone();
    let records: Vec<CkcSheetArtifactLinkRecord> = rows
        .into_iter()
        .map(CkcSheetArtifactLinkRecord::from_backend)
        .collect();
    let count = records.len();
    let mut next_selected_link_id = None;
    let mut next_reuse_ref = String::new();
    let target_index = state.ckc_characters.iter().position(|character| {
        character.sheet_version_id.as_deref() == Some(target_sheet_version_id)
    });
    if let Some(target_index) = target_index {
        let character = &mut state.ckc_characters[target_index];
        character.sheet_artifact_links = records;
        if current_selection_owns_target {
            next_selected_link_id = if let Some(selected_link_id) = selected_link_id {
                if character
                    .sheet_artifact_links
                    .iter()
                    .any(|link| link.link_id == selected_link_id)
                {
                    Some(selected_link_id)
                } else {
                    character
                        .sheet_artifact_links
                        .first()
                        .map(|link| link.link_id.clone())
                }
            } else {
                character
                    .sheet_artifact_links
                    .first()
                    .map(|link| link.link_id.clone())
            };
            next_reuse_ref = next_selected_link_id
                .as_ref()
                .and_then(|link_id| {
                    character
                        .sheet_artifact_links
                        .iter()
                        .find(|link| &link.link_id == link_id)
                })
                .map(|link| link.typed_ref.clone())
                .unwrap_or_default();
        }
    }
    if current_selection_owns_target {
        state.ckc_selected_sheet_artifact_link_id = next_selected_link_id;
        state.ckc_sheet_artifact_reuse_ref = next_reuse_ref;
    }
    CkcSheetArtifactApplyOutcome {
        count,
        current_selection_owns_target,
        target_found: target_index.is_some(),
    }
}

impl CkcMediaAlbumRecord {
    fn from_backend(row: AtelierCkcMediaAlbumRow) -> Self {
        Self {
            collection_id: row.collection_id,
            collection_ref: row.collection_ref,
            name: row.name,
            description: row.description.unwrap_or_default(),
            tags: row.tags,
            member_count: row.member_count,
            members_next_offset: row.members_next_offset,
            members: row
                .members
                .into_iter()
                .map(CkcMediaMemberRecord::from_backend)
                .collect(),
            created_by: row.created_by,
            updated_by: row.updated_by,
        }
    }
}

impl CkcMediaMemberRecord {
    fn from_backend(row: AtelierCkcMediaMemberRow) -> Self {
        Self {
            asset_id: row.asset_id,
            media_ref: row.media_ref,
            display_label: row.file_name,
            sort_order: row.sort_order,
            source_path_ref: row.source_path_ref,
            source_url_ref: row.source_url_ref,
            link_source_path_ref: row.link_source_path_ref,
            link_source_url_ref: row.link_source_url_ref,
            link_source_path_ref_status: row.link_source_path_ref_status,
            link_source_url_ref_status: row.link_source_url_ref_status,
            asset_source_path_ref_status: row.asset_source_path_ref_status,
            asset_source_url_ref_status: row.asset_source_url_ref_status,
            source_path_ref_origin: row.source_path_ref_origin,
            source_url_ref_origin: row.source_url_ref_origin,
            linked_by: row.linked_by,
            member_updated_by: row.member_updated_by,
            member_updated_at_utc: row.member_updated_at_utc,
            notes_updated_by: row.notes_updated_by.unwrap_or_else(|| "none".to_owned()),
            notes_updated_at_utc: row.notes_updated_at_utc,
            notes: row.notes.unwrap_or_default(),
            review_status: row.review_status,
            tags_buffer: row.tags.join(", "),
        }
    }

    fn apply_notes_tags(&mut self, row: &AtelierCkcMediaNotesTagsRow) {
        self.media_ref = row.media_ref.clone();
        self.notes = row.notes.clone().unwrap_or_default();
        self.review_status = row.review_status.clone();
        self.tags_buffer = row.tags.join(", ");
        self.notes_updated_by = row
            .updated_by
            .clone()
            .unwrap_or_else(|| "unknown".to_owned());
        self.notes_updated_at_utc = row.updated_at_utc.clone();
    }
}

impl CkcStoryDocumentRecord {
    fn from_backend(row: AtelierCkcCharacterDocumentRow) -> Self {
        let body_raw_text = row
            .current_version
            .as_ref()
            .map(|version| version.body_raw_text.clone())
            .unwrap_or_default();
        let cards = row
            .story_cards
            .into_iter()
            .map(CkcStoryCardRecord::from_backend)
            .collect();
        let beats = row
            .story_beats
            .into_iter()
            .map(CkcStoryBeatRecord::from_backend)
            .collect();
        Self {
            document_id: row.document_id,
            document_ref: row.document_ref,
            current_version_id: Some(row.current_version_id),
            current_version_seq: row.current_version_seq,
            title: row.title,
            body_raw_text,
            tags: row.tags,
            cards,
            beats,
        }
    }
}

impl CkcStoryCardRecord {
    fn from_backend(row: AtelierCkcStoryCardRow) -> Self {
        Self {
            card_id: row.card_id,
            card_ref: row.card_ref,
            story_document_id: row.story_document_id,
            story_document_ref: row.story_document_ref,
            title: row.title,
            body_raw_text: row.body_raw_text,
            tags: row.tags,
        }
    }
}

impl CkcStoryBeatRecord {
    fn from_backend(row: AtelierCkcStoryBeatRow) -> Self {
        Self {
            beat_id: row.beat_id,
            beat_ref: row.beat_ref,
            story_document_id: row.story_document_id,
            story_document_ref: row.story_document_ref,
            card_id: row.card_id,
            beat_text: row.beat_text,
        }
    }
}

impl CkcMoodboardDocumentRecord {
    fn from_backend(
        row: AtelierCkcCharacterDocumentRow,
        snapshot: Option<&AtelierCkcMoodboardSnapshotRow>,
    ) -> Self {
        let body_raw_text = snapshot
            .map(|snapshot| snapshot.raw_json_text.clone())
            .or_else(|| {
                row.current_version
                    .as_ref()
                    .map(|version| version.body_raw_text.clone())
            })
            .unwrap_or_default();
        Self {
            document_id: row.document_id,
            document_ref: row.document_ref,
            current_version_id: Some(row.current_version_id),
            current_version_seq: row.current_version_seq,
            title: row.title,
            body_raw_text,
            tags: row.tags,
            latest_snapshot_id: snapshot.map(|snapshot| snapshot.snapshot_id.clone()),
            latest_snapshot_ref: snapshot.map(|snapshot| snapshot.moodboard_ref.clone()),
            moodboard_name: snapshot
                .map(|snapshot| snapshot.moodboard_name.clone())
                .unwrap_or_else(|| "Moodboard".to_owned()),
        }
    }
}

fn json_field_str<'a>(row: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    row.get(field)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
}

fn json_field_f32(row: &serde_json::Value, field: &str, fallback: f32) -> f32 {
    row.get(field)
        .and_then(|value| value.as_f64())
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

fn moodboard_position(row: &serde_json::Value) -> (f32, f32) {
    row.get("position")
        .map(|position| {
            (
                json_field_f32(position, "x", 40.0),
                json_field_f32(position, "y", 40.0),
            )
        })
        .unwrap_or((40.0, 40.0))
}

/// WP-CKC MT-012 (F2): the minimum on-canvas render size the forward projection clamps moodboard
/// element cards to (a valid `hsk.atelier.moodboard@1` element may store any `width`/`height`
/// `exclusiveMinimum:0`, but a sub-minimum card would be unclickable/invisible on the board). The reverse
/// projection ([`merge_moodboard_size`]) MUST mirror this clamp when deciding whether a size changed, so a
/// stored 30x20 element that the board renders at 48x32 round-trips as a no-op instead of being silently
/// GROWN to 48x32 on save. This projection floor is deliberately separate from the LIVE-resize floor
/// (`canvas_board::MIN_CARD_W`=80 / `MIN_CARD_H`=48): the resize floor only bounds a drag gesture and never
/// rewrites stored geometry on its own, whereas this floor governs the initial projection of stored sizes.
const MOODBOARD_MIN_ELEMENT_W: f32 = 48.0;
const MOODBOARD_MIN_ELEMENT_H: f32 = 32.0;

fn moodboard_size(row: &serde_json::Value, fallback_w: f32, fallback_h: f32) -> (f32, f32) {
    row.get("size")
        .map(|size| {
            (
                json_field_f32(size, "width", fallback_w).max(MOODBOARD_MIN_ELEMENT_W),
                json_field_f32(size, "height", fallback_h).max(MOODBOARD_MIN_ELEMENT_H),
            )
        })
        .unwrap_or((fallback_w, fallback_h))
}

fn moodboard_layer_order(layer_orders: &BTreeMap<String, i32>, layer_id: Option<&str>) -> i32 {
    layer_id
        .and_then(|id| layer_orders.get(id).copied())
        .unwrap_or_default()
}

fn moodboard_element_placement_id(kind: &str, element_id: &str) -> String {
    format!("moodboard-{kind}-{element_id}")
}

fn moodboard_element_title(prefix: &str, value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        prefix.to_owned()
    } else {
        let first_line = trimmed.lines().next().unwrap_or(trimmed);
        let short: String = first_line.chars().take(72).collect();
        format!("{prefix}: {short}")
    }
}

pub fn ckc_moodboard_snapshot_to_canvas_projection(
    raw_json_text: &str,
) -> Result<CkcMoodboardCanvasProjection, String> {
    let value: serde_json::Value = serde_json::from_str(raw_json_text)
        .map_err(|err| format!("moodboard snapshot JSON parse failed: {err}"))?;
    ckc_moodboard_json_to_canvas_projection(&value)
}

pub fn ckc_moodboard_json_to_canvas_projection(
    value: &serde_json::Value,
) -> Result<CkcMoodboardCanvasProjection, String> {
    let schema_id = json_field_str(value, "schema_id").unwrap_or_default();
    if schema_id != "hsk.atelier.moodboard@1" {
        return Err(format!(
            "unsupported moodboard schema_id {schema_id:?}; expected hsk.atelier.moodboard@1"
        ));
    }

    let mut section_labels = BTreeMap::new();
    let mut layer_orders = BTreeMap::new();
    if let Some(layers) = value.get("layers").and_then(|layers| layers.as_array()) {
        for layer in layers {
            let Some(layer_id) = json_field_str(layer, "layer_id") else {
                continue;
            };
            let name = json_field_str(layer, "name").unwrap_or(layer_id);
            section_labels.insert(layer_id.to_owned(), name.to_owned());
            layer_orders.insert(
                layer_id.to_owned(),
                layer
                    .get("order")
                    .and_then(|order| order.as_i64())
                    .unwrap_or_default() as i32,
            );
        }
    }

    let mut placements = Vec::new();
    let mut element_to_placement = BTreeMap::new();

    if let Some(text_items) = value.get("text").and_then(|items| items.as_array()) {
        for (idx, item) in text_items.iter().enumerate() {
            let Some(element_id) = json_field_str(item, "element_id") else {
                continue;
            };
            let layer_id = json_field_str(item, "layer_id");
            let content = json_field_str(item, "content").unwrap_or("Moodboard text");
            let (x, y) = moodboard_position(item);
            let width = DEFAULT_CARD_W.max((content.chars().count() as f32 * 7.0).min(440.0));
            let height = DEFAULT_CARD_H;
            let placement_id = moodboard_element_placement_id("text", element_id);
            let mut card = CanvasPlacementCard::new(
                placement_id.clone(),
                format!("moodboard-text-{element_id}"),
                x,
                y,
                width,
                height,
            )
            .as_text_card(content.to_owned());
            card.live_title = Some(moodboard_element_title("Text", content));
            card.live_content_type = Some("moodboard_text".to_owned());
            card.group_id = layer_id.map(ToOwned::to_owned);
            card.z_index = moodboard_layer_order(&layer_orders, layer_id) * 1000 + idx as i32;
            element_to_placement.insert(element_id.to_owned(), placement_id);
            placements.push(card);
        }
    }

    if let Some(images) = value.get("images").and_then(|items| items.as_array()) {
        for (idx, item) in images.iter().enumerate() {
            let Some(element_id) = json_field_str(item, "element_id") else {
                continue;
            };
            let layer_id = json_field_str(item, "layer_id");
            let source = json_field_str(item, "source")
                .or_else(|| json_field_str(item, "url"))
                .unwrap_or("image");
            let block_id = json_field_str(item, "asset_id").unwrap_or(element_id);
            let (x, y) = moodboard_position(item);
            let (w, h) = moodboard_size(item, DEFAULT_CARD_W, DEFAULT_CARD_H);
            let placement_id = moodboard_element_placement_id("image", element_id);
            let mut card = CanvasPlacementCard::new(
                placement_id.clone(),
                format!("moodboard-image-{block_id}"),
                x,
                y,
                w,
                h,
            );
            card.live_title = Some(moodboard_element_title("Image", source));
            card.live_content_type = Some("moodboard_image".to_owned());
            card.group_id = layer_id.map(ToOwned::to_owned);
            card.z_index = moodboard_layer_order(&layer_orders, layer_id) * 1000 + idx as i32;
            element_to_placement.insert(element_id.to_owned(), placement_id);
            placements.push(card);
        }
    }

    if let Some(shapes) = value.get("shapes").and_then(|items| items.as_array()) {
        for (idx, item) in shapes.iter().enumerate() {
            let Some(element_id) = json_field_str(item, "element_id") else {
                continue;
            };
            let layer_id = json_field_str(item, "layer_id");
            let shape_type = json_field_str(item, "shape_type").unwrap_or("shape");
            let (x, y) = moodboard_position(item);
            let (w, h) = moodboard_size(item, DEFAULT_CARD_W, DEFAULT_CARD_H);
            let placement_id = moodboard_element_placement_id("shape", element_id);
            let mut card = CanvasPlacementCard::new(
                placement_id.clone(),
                format!("moodboard-shape-{element_id}"),
                x,
                y,
                w,
                h,
            );
            card.live_title = Some(moodboard_element_title("Shape", shape_type));
            card.live_content_type = Some("moodboard_shape".to_owned());
            card.group_id = layer_id.map(ToOwned::to_owned);
            card.z_index = moodboard_layer_order(&layer_orders, layer_id) * 1000 + idx as i32;
            element_to_placement.insert(element_id.to_owned(), placement_id);
            placements.push(card);
        }
    }

    let mut visual_edges = Vec::new();
    if let Some(connectors) = value.get("connectors").and_then(|items| items.as_array()) {
        for item in connectors {
            let Some(connector_id) = json_field_str(item, "connector_id") else {
                continue;
            };
            let Some(from_element_id) = json_field_str(item, "from_element_id") else {
                continue;
            };
            let Some(to_element_id) = json_field_str(item, "to_element_id") else {
                continue;
            };
            let Some(from_placement_id) = element_to_placement.get(from_element_id) else {
                continue;
            };
            let Some(to_placement_id) = element_to_placement.get(to_element_id) else {
                continue;
            };
            visual_edges.push(VisualEdge {
                visual_edge_id: format!("moodboard-connector-{connector_id}"),
                from_placement_id: from_placement_id.clone(),
                to_placement_id: to_placement_id.clone(),
            });
        }
    }

    Ok(CkcMoodboardCanvasProjection {
        placements,
        visual_edges,
        section_labels,
        pan: egui::Vec2::ZERO,
        zoom: 1.0,
    })
}

/// WP-CKC MT-012 (FIX 1): overwrite `item.position` ONLY when the live f32 geometry differs from the
/// stored value read back as f32. The forward projection cast each stored `f64` through `f32`, so an
/// element that was NOT moved compares bit-equal here and its JSON number is left byte-untouched — that
/// is what keeps a load -> edit-nothing -> save idempotent (identical `content_sha256`, backend dedup).
fn merge_moodboard_position(item: &mut serde_json::Value, x: f32, y: f32) {
    let stored = item.get("position");
    let sx = stored
        .and_then(|p| p.get("x"))
        .and_then(serde_json::Value::as_f64);
    let sy = stored
        .and_then(|p| p.get("y"))
        .and_then(serde_json::Value::as_f64);
    if sx.map(|v| v as f32) == Some(x) && sy.map(|v| v as f32) == Some(y) {
        return;
    }
    if let Some(obj) = item.as_object_mut() {
        obj.insert(
            "position".to_owned(),
            serde_json::json!({ "x": x as f64, "y": y as f64 }),
        );
    }
}

/// WP-CKC MT-012 (FIX 1 + F2): precision-guarded merge of the `item.size` (`width`/`height`) box for
/// image and shape elements. Compares the LIVE size against the stored size AFTER applying the SAME
/// forward-projection floor ([`MOODBOARD_MIN_ELEMENT_W`]/[`MOODBOARD_MIN_ELEMENT_H`]) that produced the
/// live value, so a stored sub-minimum element (e.g. 30x20 rendered at 48x32) compares equal and is left
/// byte-untouched. Without mirroring the clamp, EVERY save would silently GROW such elements to the floor
/// and change the content_sha256 (defeating the no-op dedup). A genuine resize still differs from the
/// clamped-stored value and is written through.
fn merge_moodboard_size(item: &mut serde_json::Value, w: f32, h: f32) {
    let stored = item.get("size");
    let sw = stored
        .and_then(|p| p.get("width"))
        .and_then(serde_json::Value::as_f64);
    let sh = stored
        .and_then(|p| p.get("height"))
        .and_then(serde_json::Value::as_f64);
    let unchanged = sw.map(|v| (v as f32).max(MOODBOARD_MIN_ELEMENT_W)) == Some(w)
        && sh.map(|v| (v as f32).max(MOODBOARD_MIN_ELEMENT_H)) == Some(h);
    if unchanged {
        return;
    }
    if let Some(obj) = item.as_object_mut() {
        obj.insert(
            "size".to_owned(),
            serde_json::json!({ "width": w as f64, "height": h as f64 }),
        );
    }
}

/// WP-CKC MT-012 (FIX 1): the REVERSE of [`ckc_moodboard_json_to_canvas_projection`]. Reads the LIVE
/// moodboard [`LoomCanvasBoard`] and folds its per-element geometry (position for text; position + size
/// for images/shapes) back onto `prev_snapshot_json`, which supplies every NON-visual field
/// (schema_id, moodboard_id, canvas, layers, connectors, folders, guides, flags, style, history and —
/// crucially — each image's ArtifactStore `source`/`asset_id`/`url` ref, per the MT-043
/// `atelier://media/<uuid>` scheme, NEVER a Loom `placed_block_id`). Elements are matched by the
/// `element_id` embedded in each placement id (`moodboard-{kind}-{element_id}`, see
/// [`moodboard_element_placement_id`]); a placement with no matching element is ignored, and an element
/// with no matching placement keeps its stored geometry (so an unopened/empty board never strips a
/// snapshot).
///
/// A no-op save (nothing moved/resized) re-serializes byte-identically to the stored snapshot. This holds
/// NOT because keys are sorted — the `handshake_native` crate resolves `serde_json` with the
/// `preserve_order` feature (indexmap), so object keys keep INSERTION order — but by three invariants
/// working together: (1) the backend hashes, stores, and returns the exact submitted `raw_json_text`
/// bytes verbatim (`content_sha256` is over those bytes), and `prev_snapshot_json` is parsed from that
/// text, so `preserve_order` reproduces its key order on re-`to_string`; (2) we emit COMPACT
/// `serde_json::to_string` (no whitespace), matching every producer in this module
/// (`local_ckc_moodboard_snapshot_json` and prior saves); (3) the f32 read-back guard
/// ([`merge_moodboard_position`]) and the clamp-mirroring guard ([`merge_moodboard_size`]) leave an
/// unedited element's numbers untouched, so no number is re-formatted. Together these make a no-op the
/// same bytes -> same `content_sha256` -> backend dedup suppresses a redundant snapshot.
///
/// `pan`/`zoom` are intentionally NOT written back: `hsk.atelier.moodboard@1` has no viewport field
/// (`additionalProperties:false`), so persisting them would make the snapshot fail Draft 2020-12
/// validation; the forward projection re-centres the viewport on load.
pub fn board_to_ckc_moodboard_json(
    board: &LoomCanvasBoard,
    prev_snapshot_json: &serde_json::Value,
) -> String {
    let mut out = prev_snapshot_json.clone();

    // Index the live placements by their moodboard element_id, split by kind so text (position only)
    // and image/shape (position + size) update the correct fields.
    let mut text_pos: BTreeMap<String, (f32, f32)> = BTreeMap::new();
    let mut boxed_geom: BTreeMap<String, (f32, f32, f32, f32)> = BTreeMap::new();
    for card in &board.placements {
        if let Some(element_id) = card.placement_id.strip_prefix("moodboard-text-") {
            text_pos.insert(element_id.to_owned(), (card.x, card.y));
        } else if let Some(element_id) = card.placement_id.strip_prefix("moodboard-image-") {
            boxed_geom.insert(element_id.to_owned(), (card.x, card.y, card.w, card.h));
        } else if let Some(element_id) = card.placement_id.strip_prefix("moodboard-shape-") {
            boxed_geom.insert(element_id.to_owned(), (card.x, card.y, card.w, card.h));
        }
    }

    if let Some(items) = out.get_mut("text").and_then(|v| v.as_array_mut()) {
        for item in items {
            let Some(element_id) = item
                .get("element_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
            else {
                continue;
            };
            if let Some((x, y)) = text_pos.get(&element_id).copied() {
                merge_moodboard_position(item, x, y);
            }
        }
    }

    for key in ["images", "shapes"] {
        if let Some(items) = out.get_mut(key).and_then(|v| v.as_array_mut()) {
            for item in items {
                let Some(element_id) = item
                    .get("element_id")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
                else {
                    continue;
                };
                if let Some((x, y, w, h)) = boxed_geom.get(&element_id).copied() {
                    merge_moodboard_position(item, x, y);
                    merge_moodboard_size(item, w, h);
                }
            }
        }
    }

    serde_json::to_string(&out).unwrap_or_else(|_| prev_snapshot_json.to_string())
}

impl CkcSearchResultRecord {
    fn from_backend(row: AtelierCkcSearchResultRow) -> Self {
        let AtelierCkcSearchResultRow {
            target_kind,
            target_ref,
            title,
            snippet,
            character_ref,
            sheet_version_ref,
            collection_ref,
            media_ref,
            tag_ref,
            tags,
            tag_notes,
            match_modes,
            fuzzy_score: _,
            vector_score: _,
        } = row;
        let tag_notes = tag_notes
            .into_iter()
            .filter(|note| {
                ckc_scope_matches_result_refs(
                    note.scope_ref.as_deref(),
                    &target_ref,
                    character_ref.as_deref(),
                    sheet_version_ref.as_deref(),
                    collection_ref.as_deref(),
                    media_ref.as_deref(),
                    tag_ref.as_deref(),
                )
            })
            .map(CkcTagNoteRecord::from_backend)
            .collect();
        Self {
            target_kind,
            target_ref,
            title,
            snippet,
            character_ref,
            sheet_version_ref,
            collection_ref,
            media_ref,
            tag_ref,
            tags,
            tag_notes,
            match_modes,
        }
    }

    fn summary_label(&self) -> String {
        let modes = if self.match_modes.is_empty() {
            "match".to_owned()
        } else {
            self.match_modes.join("+")
        };
        format!(
            "{}: {} [{}] {}",
            self.target_kind, self.title, modes, self.target_ref
        )
    }
}

impl CkcTagNoteRecord {
    fn from_backend(row: AtelierCkcTagNoteRow) -> Self {
        Self {
            tag_ref: row.tag_ref,
            tag_text: row.tag_text,
            scope_ref: row.scope_ref,
            note: row.note,
        }
    }
}

fn seeded_ckc_search_results(characters: &[CkcCharacterRecord]) -> Vec<CkcSearchResultRecord> {
    local_ckc_search(
        characters,
        "mira reference",
        CkcSearchMode::Fuzzy,
        &["reference".to_owned()],
    )
}

fn local_ckc_search(
    characters: &[CkcCharacterRecord],
    query: &str,
    mode: CkcSearchMode,
    tags: &[String],
) -> Vec<CkcSearchResultRecord> {
    let query = query.trim().to_ascii_lowercase();
    let tags: Vec<String> = tags
        .iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();
    let mut out = Vec::new();
    for character in characters {
        let character_tags = ["character".to_owned(), "sheet".to_owned()];
        let haystack = format!(
            "{}\n{}\n{}",
            character.display_name, character.public_id, character.sheet_editor_text
        )
        .to_ascii_lowercase();
        if local_match(&haystack, &query, mode)
            && tags.iter().all(|tag| {
                character_tags.iter().any(|candidate| candidate == tag)
                    || haystack.contains(tag.as_str())
            })
        {
            out.push(CkcSearchResultRecord {
                target_kind: "character".to_owned(),
                target_ref: character.character_ref(),
                title: character.display_name.clone(),
                snippet: character
                    .sheet_editor_text
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
                character_ref: Some(character.character_ref()),
                sheet_version_ref: character.sheet_version_ref(),
                collection_ref: None,
                media_ref: None,
                tag_ref: None,
                tags: character_tags.to_vec(),
                tag_notes: Vec::new(),
                match_modes: vec![mode.backend_value().to_owned()],
            });
        }

        for album in &character.media_albums {
            let album_haystack = format!(
                "{}\n{}\n{}\n{}",
                album.name,
                album.description,
                album.tags.join(" "),
                character.display_name
            )
            .to_ascii_lowercase();
            if local_match(&album_haystack, &query, mode)
                && tags
                    .iter()
                    .all(|tag| album.tags.iter().any(|candidate| candidate == tag))
            {
                out.push(CkcSearchResultRecord {
                    target_kind: "album".to_owned(),
                    target_ref: album.collection_ref.clone(),
                    title: album.name.clone(),
                    snippet: album.description.clone(),
                    character_ref: Some(character.character_ref()),
                    sheet_version_ref: character.sheet_version_ref(),
                    collection_ref: Some(album.collection_ref.clone()),
                    media_ref: None,
                    tag_ref: None,
                    tags: album.tags.clone(),
                    tag_notes: seeded_local_tag_notes(album),
                    match_modes: vec![mode.backend_value().to_owned()],
                });
            }
            for member in &album.members {
                let member_tags = ckc_tags_from_buffer(&member.tags_buffer);
                let member_haystack = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    member.display_label,
                    member.notes,
                    member_tags.join(" "),
                    album.name,
                    character.display_name
                )
                .to_ascii_lowercase();
                if local_match(&member_haystack, &query, mode)
                    && tags
                        .iter()
                        .all(|tag| member_tags.iter().any(|candidate| candidate == tag))
                {
                    out.push(CkcSearchResultRecord {
                        target_kind: "media".to_owned(),
                        target_ref: member.media_ref.clone(),
                        title: member.display_label.clone(),
                        snippet: member.notes.clone(),
                        character_ref: Some(character.character_ref()),
                        sheet_version_ref: character.sheet_version_ref(),
                        collection_ref: Some(album.collection_ref.clone()),
                        media_ref: Some(member.media_ref.clone()),
                        tag_ref: None,
                        tags: member_tags,
                        tag_notes: seeded_local_tag_notes(album),
                        match_modes: vec![mode.backend_value().to_owned()],
                    });
                }
            }
        }

        for story in &character.story_documents {
            let story_haystack = format!(
                "{}\n{}\n{}\n{}",
                story.title,
                story.body_raw_text,
                story.tags.join(" "),
                character.display_name
            )
            .to_ascii_lowercase();
            if local_match(&story_haystack, &query, mode)
                && tags
                    .iter()
                    .all(|tag| story.tags.iter().any(|candidate| candidate == tag))
            {
                out.push(CkcSearchResultRecord {
                    target_kind: "story".to_owned(),
                    target_ref: story.document_ref.clone(),
                    title: story.title.clone(),
                    snippet: story.body_raw_text.clone(),
                    character_ref: Some(character.character_ref()),
                    sheet_version_ref: character.sheet_version_ref(),
                    collection_ref: None,
                    media_ref: None,
                    tag_ref: None,
                    tags: story.tags.clone(),
                    tag_notes: Vec::new(),
                    match_modes: vec![mode.backend_value().to_owned()],
                });
            }
            for card in &story.cards {
                let card_haystack = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    card.title,
                    card.body_raw_text,
                    card.tags.join(" "),
                    story.title,
                    character.display_name
                )
                .to_ascii_lowercase();
                if local_match(&card_haystack, &query, mode)
                    && tags
                        .iter()
                        .all(|tag| card.tags.iter().any(|candidate| candidate == tag))
                {
                    out.push(CkcSearchResultRecord {
                        target_kind: "story_card".to_owned(),
                        target_ref: card.card_ref.clone(),
                        title: card.title.clone(),
                        snippet: card.body_raw_text.clone(),
                        character_ref: Some(character.character_ref()),
                        sheet_version_ref: character.sheet_version_ref(),
                        collection_ref: None,
                        media_ref: None,
                        tag_ref: None,
                        tags: card.tags.clone(),
                        tag_notes: Vec::new(),
                        match_modes: vec![mode.backend_value().to_owned()],
                    });
                }
            }
        }

        for moodboard in &character.moodboard_documents {
            let moodboard_haystack = format!(
                "{}\n{}\n{}\n{}\n{}",
                moodboard.title,
                moodboard.moodboard_name,
                moodboard.body_raw_text,
                moodboard.tags.join(" "),
                character.display_name
            )
            .to_ascii_lowercase();
            if local_match(&moodboard_haystack, &query, mode)
                && tags
                    .iter()
                    .all(|tag| moodboard.tags.iter().any(|candidate| candidate == tag))
            {
                out.push(CkcSearchResultRecord {
                    target_kind: "moodboard".to_owned(),
                    target_ref: moodboard
                        .latest_snapshot_ref
                        .clone()
                        .unwrap_or_else(|| moodboard.document_ref.clone()),
                    title: moodboard.title.clone(),
                    snippet: moodboard.body_raw_text.clone(),
                    character_ref: Some(character.character_ref()),
                    sheet_version_ref: character.sheet_version_ref(),
                    collection_ref: None,
                    media_ref: None,
                    tag_ref: None,
                    tags: moodboard.tags.clone(),
                    tag_notes: Vec::new(),
                    match_modes: vec![mode.backend_value().to_owned()],
                });
            }
        }
    }
    out.truncate(8);
    out
}

fn local_match(haystack: &str, query: &str, mode: CkcSearchMode) -> bool {
    if query.is_empty() {
        return true;
    }
    if haystack.contains(query) {
        return true;
    }
    match mode {
        CkcSearchMode::Fuzzy | CkcSearchMode::Combined => query.split_whitespace().all(|needle| {
            haystack
                .split_whitespace()
                .any(|word| fuzzy_word_match(word, needle))
        }),
        CkcSearchMode::Vector => query.split_whitespace().any(|needle| {
            haystack
                .split_whitespace()
                .any(|word| fuzzy_word_match(word, needle))
        }),
    }
}

fn fuzzy_word_match(word: &str, needle: &str) -> bool {
    if needle.len() <= 2 {
        return word == needle;
    }
    if word.contains(needle) || needle.contains(word) {
        return true;
    }
    let common = needle.chars().filter(|ch| word.contains(*ch)).count();
    common + 1 >= needle.len()
}

fn seeded_local_tag_notes(album: &CkcMediaAlbumRecord) -> Vec<CkcTagNoteRecord> {
    album
        .tags
        .iter()
        .filter(|tag| tag.as_str() == "training" || tag.as_str() == "reference")
        .map(|tag| CkcTagNoteRecord {
            tag_ref: format!("atelier://tag/local-{tag}"),
            tag_text: tag.clone(),
            scope_ref: Some(album.collection_ref.clone()),
            note: format!("{tag} applies to reusable CKC media for this album."),
        })
        .collect()
}

fn local_import_ckc_sheet(state: &mut AtelierPanelState, selected_index: usize) {
    let import_text = match local_import_raw_text(&state.ckc_import_text) {
        Ok(raw_text) => raw_text,
        Err(err) => {
            state.ckc_export_status = err;
            return;
        }
    };
    let next_seq = {
        let Some(character) = state.ckc_characters.get_mut(selected_index) else {
            state.ckc_export_status = "No CKC character selected for import.".to_owned();
            return;
        };
        if let Err(err) = local_validate_ckc_sheet_owner(character, &import_text) {
            state.ckc_export_status = err;
            return;
        }
        character.parent_sheet_version_id = character.sheet_version_id.clone();
        let next_sheet_version_id = Uuid::new_v4().to_string();
        character.sheet_version_id = Some(next_sheet_version_id.clone());
        character.sheet_seq += 1;
        character.sheet_editor_text = import_text;
        character.sheet_version_ref = Some(format!(
            "atelier://sheet/{}/{}",
            character.character_internal_id, next_sheet_version_id
        ));
        character.sheet_seq
    };
    state.ckc_last_export = None;
    state.ckc_export_status = format!(
        "Imported CKC sheet locally as append-only version v{}",
        next_seq
    );
}

fn local_import_raw_text(import_text: &str) -> Result<String, String> {
    let trimmed = import_text.trim();
    if trimmed.is_empty() {
        return Err("CKC import text is empty.".to_owned());
    }
    if !trimmed.starts_with('{') {
        return Ok(import_text.to_owned());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("CKC sheet import JSON is invalid: {err}"))?;
    local_raw_text_from_export_json(&value)
        .ok_or_else(|| "CKC sheet import JSON must contain raw_text or content.".to_owned())
}

fn local_raw_text_from_export_json(value: &serde_json::Value) -> Option<String> {
    if let Some(raw_text) = value.get("raw_text").and_then(|value| value.as_str()) {
        return Some(raw_text.to_owned());
    }
    let content = value.get("content").and_then(|value| value.as_str())?;
    if content.trim_start().starts_with('{') {
        serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|nested| local_raw_text_from_export_json(&nested))
            .or_else(|| Some(content.to_owned()))
    } else {
        Some(content.to_owned())
    }
}

fn local_validate_ckc_sheet_owner(
    character: &CkcCharacterRecord,
    raw_text: &str,
) -> Result<(), String> {
    let character_ids = sheet_field_values(raw_text, "CHAR-ID-001");
    if character_ids.is_empty() {
        return Err(
            "CKC sheet import must include CHAR-ID-001 for character ownership.".to_owned(),
        );
    }
    if character_ids.len() > 1 {
        return Err(format!(
            "CKC sheet import must include exactly one CHAR-ID-001 for character ownership; found {}",
            character_ids.len()
        ));
    }
    let Some(character_id) = character_ids.into_iter().next() else {
        return Err(
            "CKC sheet import must include CHAR-ID-001 for character ownership.".to_owned(),
        );
    };
    if character_id != character.public_id {
        return Err(format!(
            "CKC sheet CHAR-ID-001={character_id} does not match character public_id={}",
            character.public_id
        ));
    }
    Ok(())
}

fn local_content_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn local_export_ckc_sheet(character: &CkcCharacterRecord, format: &str) -> AtelierSheetExportRow {
    let version_id = character
        .sheet_version_id
        .clone()
        .unwrap_or_else(|| "local-unsaved-sheet".to_owned());
    let sheet_version_ref = character.sheet_version_ref.clone().unwrap_or_else(|| {
        format!(
            "atelier://sheet/{}/{}",
            character.character_internal_id, version_id
        )
    });
    let (format_label, file_ext, raw_text) = match format {
        "json" => ("json", "json", character.sheet_editor_text.clone()),
        "safe-txt" => (
            "safe-txt",
            "safe.txt",
            local_safe_subset_sheet_text(&character.sheet_editor_text),
        ),
        "safe-json" => (
            "safe-json",
            "safe.json",
            local_safe_subset_sheet_text(&character.sheet_editor_text),
        ),
        _ => ("txt", "txt", character.sheet_editor_text.clone()),
    };
    let content = if format_label.ends_with("json") {
        local_export_sheet_json(
            character,
            &version_id,
            &sheet_version_ref,
            &raw_text,
            format_label,
        )
    } else {
        raw_text
    };
    AtelierSheetExportRow {
        version_id: version_id.clone(),
        format: format_label.to_owned(),
        file_name: format!("ckc-sheet-{version_id}.{file_ext}"),
        content_hash: local_content_hash(&content),
        content,
        character_ref: character.character_ref.clone(),
        sheet_version_ref,
    }
}

fn local_export_sheet_json(
    character: &CkcCharacterRecord,
    version_id: &str,
    sheet_version_ref: &str,
    raw_text: &str,
    format_label: &str,
) -> String {
    let export_format = if format_label == "safe-json" {
        "ckc-sheet-safe-export.v1"
    } else {
        "ckc-sheet-export.v1"
    };
    serde_json::to_string_pretty(&serde_json::json!({
        "export_format": export_format,
        "template_version": LOCAL_CKC_TEMPLATE_VERSION,
        "version_id": version_id,
        "character_internal_id": &character.character_internal_id,
        "parent_version_id": &character.parent_sheet_version_id,
        "seq": character.sheet_seq,
        "author": "handshake-native-atelier-ckc-local",
        "tool": "handshake-native-atelier-local-export",
        "character_ref": &character.character_ref,
        "sheet_version_ref": sheet_version_ref,
        "raw_text": raw_text,
        "created_at_utc": "local-no-backend",
    }))
    .unwrap_or_else(|_| raw_text.to_owned())
}

fn local_safe_subset_sheet_text(raw_text: &str) -> String {
    let safe_ids = local_safe_subset_ids();
    let mut out = String::with_capacity(raw_text.len());
    for segment in raw_text.split_inclusive('\n') {
        let trimmed_line = segment.trim_end_matches(['\r', '\n']);
        match local_sheet_field_id_from_line(trimmed_line) {
            Some(field_id) if safe_ids.contains(&field_id) => out.push_str(segment),
            Some(_) => {}
            None if local_sheet_line_looks_like_field(trimmed_line) => {}
            None => out.push_str(segment),
        }
    }
    out
}

fn local_safe_subset_ids() -> std::collections::HashSet<String> {
    serde_json::from_str::<Vec<String>>(LOCAL_CKC_SAFE_SUBSET_V2_JSON)
        .unwrap_or_default()
        .into_iter()
        .map(|field_id| field_id.to_ascii_uppercase())
        .collect()
}

fn local_field_suggestions(
    characters: &[CkcCharacterRecord],
    field_id: &str,
) -> Vec<AtelierSheetFieldSuggestionRow> {
    let mut out = Vec::new();
    let field_id = field_id.trim().to_ascii_uppercase();
    for character in characters {
        if let Some(value) = sheet_field_value(&character.sheet_editor_text, &field_id) {
            if let Some(index) = out
                .iter()
                .position(|row: &AtelierSheetFieldSuggestionRow| row.value == value)
            {
                out[index].occurrences += 1;
            } else {
                out.push(AtelierSheetFieldSuggestionRow {
                    field_id: field_id.clone(),
                    value,
                    occurrences: 1,
                });
            }
        }
    }
    out.truncate(8);
    out
}

fn sheet_field_value(raw_text: &str, field_id: &str) -> Option<String> {
    sheet_field_values(raw_text, field_id).into_iter().next()
}

fn sheet_field_values(raw_text: &str, field_id: &str) -> Vec<String> {
    let field_id = field_id.trim();
    if field_id.is_empty() {
        return Vec::new();
    }
    let mut values = Vec::new();
    for line in raw_text.lines().map(str::trim) {
        let Some((parsed_field_id, value)) = local_split_field_line(line) else {
            continue;
        };
        if parsed_field_id.eq_ignore_ascii_case(field_id) {
            values.push(value);
        }
    }
    values
}

fn local_sheet_field_id_from_line(line: &str) -> Option<String> {
    let (field_id, _) = local_split_field_line(line.trim())?;
    Some(field_id)
}

fn local_sheet_line_looks_like_field(line: &str) -> bool {
    let Some(colon) = line.find(':') else {
        return false;
    };
    let before_colon = line[..colon].trim();
    let Some(id_end) = local_field_id_end(before_colon) else {
        return false;
    };
    before_colon[id_end..]
        .chars()
        .any(|ch| matches!(ch, '\u{2014}' | '\u{2013}' | '-'))
}

fn local_split_field_line(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let before_colon = line[..colon].trim();
    let descriptor = line[colon + 1..].trim();
    let id_end = local_field_id_end(before_colon)?;
    let id = before_colon[..id_end].trim();
    let after_id = before_colon[id_end..].trim_start();
    let separator = after_id.chars().next()?;
    if !matches!(separator, '\u{2014}' | '\u{2013}' | '-') {
        return None;
    }
    let label = after_id[separator.len_utf8()..].trim();
    if label.is_empty() {
        return None;
    }
    let value = local_normalize_field_value(descriptor)?;
    Some((id.to_ascii_uppercase(), value))
}

fn local_field_id_end(before_colon: &str) -> Option<usize> {
    let mut idx = 0usize;
    for segment_idx in 0..3 {
        let segment_start = idx;
        while let Some(ch) = before_colon[idx..].chars().next() {
            if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
                idx += ch.len_utf8();
            } else {
                break;
            }
        }
        if idx == segment_start {
            return None;
        }
        if segment_idx < 2 {
            if before_colon[idx..].starts_with('-') {
                idx += 1;
            } else {
                return None;
            }
        }
    }
    Some(idx)
}

fn local_normalize_field_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 500 {
        return None;
    }
    if value.starts_with('<') && value.ends_with('>') {
        return None;
    }
    Some(value.to_owned())
}

fn short_hash(hash: &str) -> &str {
    hash.get(..hash.len().min(12)).unwrap_or(hash)
}

pub fn ckc_field_suggestion_row_author_id(field_id: &str, value: &str) -> String {
    format!(
        "atelier-ckc-field-suggestion-{}-{}",
        stable_author_id_suffix(field_id),
        stable_author_id_suffix(value)
    )
}

fn ckc_search_status_from_response(response: &AtelierCkcSearchResponse) -> String {
    let modes = if response.modes.is_empty() {
        "fuzzy".to_owned()
    } else {
        response.modes.join("+")
    };
    let vector = response
        .vector_source
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("no vector source");
    format!(
        "CKC search returned {} result(s) for '{}' via {modes}; semantic_available={} ({vector})",
        response.result_count, response.query, response.semantic_available
    )
}

fn attach_tag_note_to_visible_results(
    results: &mut [CkcSearchResultRecord],
    note: CkcTagNoteRecord,
) {
    for result in results {
        let tag_matches = result.tags.iter().any(|tag| tag == &note.tag_text)
            || result
                .tag_ref
                .as_deref()
                .is_some_and(|tag_ref| tag_ref == note.tag_ref);
        let scope_matches = note
            .scope_ref
            .as_deref()
            .map(|scope_ref| ckc_result_has_ref(result, Some(scope_ref)))
            .unwrap_or(true);
        if tag_matches && scope_matches {
            result.tag_notes.retain(|existing| {
                !(existing.tag_text == note.tag_text && existing.scope_ref == note.scope_ref)
            });
            result.tag_notes.push(note.clone());
        }
    }
}

fn apply_ckc_character_document_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcCharacterDocumentRow,
) -> Option<String> {
    let character_internal_id = row.character_internal_id.clone();
    let document_id = row.document_id.clone();
    let doc_type = row.doc_type.clone();
    let target = characters.iter_mut().find(|character| {
        character.character_internal_id == character_internal_id
            || character
                .story_documents
                .iter()
                .any(|document| document.document_id == document_id)
            || character
                .moodboard_documents
                .iter()
                .any(|document| document.document_id == document_id)
    })?;
    match doc_type.as_str() {
        "story" => {
            let mut updated = CkcStoryDocumentRecord::from_backend(row);
            let existing_index = target
                .story_documents
                .iter()
                .position(|document| document.document_id == document_id)
                .or_else(|| {
                    target
                        .story_documents
                        .iter()
                        .position(|document| is_pending_ckc_document_id(&document.document_id))
                });
            if let Some(existing_index) = existing_index {
                let existing = &mut target.story_documents[existing_index];
                if updated.cards.is_empty() {
                    updated.cards = existing.cards.clone();
                }
                if updated.beats.is_empty() {
                    updated.beats = existing.beats.clone();
                }
                *existing = updated;
            } else {
                target.story_documents.push(updated);
            }
            Some(format!("Saved CKC story document {document_id}"))
        }
        "moodboard" => {
            let mut updated = CkcMoodboardDocumentRecord::from_backend(row, None);
            let existing_index = target
                .moodboard_documents
                .iter()
                .position(|document| document.document_id == document_id)
                .or_else(|| {
                    target
                        .moodboard_documents
                        .iter()
                        .position(|document| is_pending_ckc_document_id(&document.document_id))
                });
            if let Some(existing_index) = existing_index {
                let existing = &mut target.moodboard_documents[existing_index];
                if updated.latest_snapshot_id.is_none() {
                    updated.latest_snapshot_id = existing.latest_snapshot_id.clone();
                    updated.latest_snapshot_ref = existing.latest_snapshot_ref.clone();
                    updated.moodboard_name = existing.moodboard_name.clone();
                }
                *existing = updated;
            } else {
                target.moodboard_documents.push(updated);
            }
            Some(format!("Saved CKC moodboard document {document_id}"))
        }
        _ => None,
    }
}

fn apply_ckc_story_card_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcStoryCardRow,
) -> Option<String> {
    let story_document_id = row.story_document_id.clone();
    let card_id = row.card_id.clone();
    for character in characters {
        if let Some(story) = character
            .story_documents
            .iter_mut()
            .find(|story| story.document_id == story_document_id)
        {
            let updated = CkcStoryCardRecord::from_backend(row);
            if let Some(existing) = story.cards.iter_mut().find(|card| card.card_id == card_id) {
                *existing = updated;
            } else {
                story.cards.push(updated);
            }
            return Some(format!("Added CKC story card {card_id}"));
        }
    }
    None
}

fn apply_ckc_story_beat_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcStoryBeatRow,
) -> Option<String> {
    let story_document_id = row.story_document_id.clone();
    let beat_id = row.beat_id.clone();
    for character in characters {
        if let Some(story) = character
            .story_documents
            .iter_mut()
            .find(|story| story.document_id == story_document_id)
        {
            let updated = CkcStoryBeatRecord::from_backend(row);
            if let Some(existing) = story.beats.iter_mut().find(|beat| beat.beat_id == beat_id) {
                *existing = updated;
            } else {
                story.beats.push(updated);
            }
            return Some(format!("Added CKC story beat {beat_id}"));
        }
    }
    None
}

fn apply_ckc_moodboard_snapshot_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcMoodboardSnapshotRow,
) -> Result<(Option<String>, CkcMoodboardCanvasProjection, String), String> {
    let projection = ckc_moodboard_snapshot_to_canvas_projection(&row.raw_json_text)?;
    let document_id = row.document_id.clone();
    let snapshot_id = row.snapshot_id.clone();
    let snapshot_ref = row.moodboard_ref.clone();
    for character in characters {
        if let Some(moodboard) = character
            .moodboard_documents
            .iter_mut()
            .find(|moodboard| moodboard.document_id == document_id)
        {
            moodboard.latest_snapshot_id = Some(row.snapshot_id);
            moodboard.latest_snapshot_ref = Some(row.moodboard_ref);
            moodboard.moodboard_name = row.moodboard_name;
            moodboard.body_raw_text = row.raw_json_text;
            // WP-CKC MT-012 (F1): advance the tracked document version to the refreshed head so the NEXT
            // save sends a fresh `expected_parent_version_id`. Without this, a
            // `stale_moodboard_document_version` conflict + auto-refresh would leave `current_version_id`
            // stale and every subsequent re-save would re-conflict forever (only a full re-open recovered).
            moodboard.current_version_id = Some(row.document_version_id);
            return Ok((
                Some(format!("Opened CKC moodboard snapshot {snapshot_id}")),
                projection,
                snapshot_ref,
            ));
        }
    }
    Ok((None, projection, snapshot_ref))
}

#[derive(Debug)]
struct AtelierPanelState {
    active_tab: AtelierPanelTab,
    ckc_book_mode: CkcBookMode,
    ckc_characters: Vec<CkcCharacterRecord>,
    ckc_selected_index: usize,
    ckc_new_display_name: String,
    ckc_backend_loaded: bool,
    ckc_load_requested: bool,
    ckc_loading: bool,
    ckc_create_pending: bool,
    ckc_append_pending: bool,
    // MT-037 (F1): transient status for the sheet-version save surface (local vs backend).
    ckc_sheet_status: String,
    ckc_template_pending: bool,
    ckc_safe_subset_pending: bool,
    ckc_template_status: String,
    ckc_import_text: String,
    ckc_import_pending: bool,
    ckc_export_pending: bool,
    ckc_export_status: String,
    ckc_last_export: Option<AtelierSheetExportRow>,
    ckc_field_suggestion_id: String,
    ckc_field_suggestion_pending: bool,
    ckc_field_suggestion_status: String,
    ckc_field_suggestions: Vec<AtelierSheetFieldSuggestionRow>,
    ckc_sheet_artifact_pending: bool,
    ckc_sheet_artifact_status: String,
    ckc_sheet_artifact_kind: String,
    ckc_sheet_artifact_ref: String,
    ckc_sheet_artifact_manifest_ref: String,
    ckc_sheet_artifact_label: String,
    ckc_sheet_artifact_reuse_role: String,
    ckc_sheet_artifact_actor_id: String,
    ckc_selected_sheet_artifact_link_id: Option<String>,
    ckc_sheet_artifact_reuse_ref: String,
    ckc_media_save_pending: bool,
    ckc_selected_media_key: Option<String>,
    /// MT-010 CKC linked-media image byte fetch (mirrors the MT-043 Posekit source-image fields).
    /// `ckc_media_loaded_ref` is the trimmed media ref we last acted on (so a fetch fires once per
    /// selection change, not every frame); `ckc_media_fetch_seq` mints monotonic request ids;
    /// `ckc_active_media_fetch` is the in-flight id so a stale response for a superseded selection is
    /// ignored; `ckc_media_fetch_asset` is the asset UUID that in-flight fetch targets (used to key the
    /// decode cache when bytes arrive); `ckc_media_image_status` is the human/Argus-readable load state.
    ckc_media_fetch_seq: u64,
    ckc_active_media_fetch: Option<u64>,
    ckc_media_fetch_asset: Option<String>,
    ckc_media_loaded_ref: Option<String>,
    ckc_media_image_status: String,
    ckc_selected_album_collection_id: Option<String>,
    ckc_album_create_name: String,
    ckc_album_create_notes: String,
    ckc_album_create_tags: String,
    ckc_album_link_asset_ids: String,
    ckc_album_link_source_path_ref: String,
    ckc_album_link_source_url_ref: String,
    ckc_album_link_edit_source_path_ref: String,
    ckc_album_link_edit_source_url_ref: String,
    ckc_album_link_edit_clear_source_path: bool,
    ckc_album_link_edit_clear_source_url: bool,
    ckc_media_actor_id: String,
    ckc_album_create_pending: bool,
    ckc_album_link_pending: bool,
    ckc_album_page_pending: bool,
    ckc_album_page_pending_collection_id: Option<String>,
    ckc_album_list_page_pending: bool,
    ckc_album_list_page_pending_character_id: Option<String>,
    ckc_album_status: String,
    ckc_story_card_title: String,
    ckc_story_card_body: String,
    ckc_story_beat_text: String,
    ckc_story_status: String,
    ckc_moodboard_status: String,
    ckc_active_story_document_id: Option<String>,
    ckc_active_moodboard_document_id: Option<String>,
    /// WP-CKC MT-012 (F3): set true when a `stale_moodboard_document_version` save conflict queued an
    /// auto-refresh of the latest snapshot. While set, the NEXT latest-snapshot result is treated as a
    /// conflict recovery: it refreshes `current_version_id`/`body_raw_text` (so the re-save carries a fresh
    /// parent) but does NOT re-project onto the live board, so the operator's just-failed canvas edits are
    /// preserved for a re-save (last-writer-wins after refresh). Cleared on the first result consumed, so a
    /// normal open never mistakes itself for a recovery and no refetch loop can form.
    ckc_moodboard_conflict_recovery: bool,
    ckc_character_notes_buffer: String,
    ckc_character_notes_source_key: Option<String>,
    ckc_character_notes_status: String,
    ckc_search_query: String,
    ckc_search_tags: String,
    ckc_search_filter_selected_character: bool,
    ckc_search_filter_selected_collection: bool,
    ckc_search_filter_selected_media: bool,
    ckc_search_use_selected_media_similarity: bool,
    ckc_search_mode: CkcSearchMode,
    ckc_search_pending: bool,
    ckc_search_status: String,
    ckc_search_results: Vec<CkcSearchResultRecord>,
    ckc_tag_note_tag: String,
    ckc_tag_note_scope_ref: String,
    ckc_tag_note_editor: String,
    ckc_tag_note_pending: bool,
    ckc_error: Option<String>,
    pose_yaw: f32,
    pose_pitch: f32,
    pose_zoom: f32,
    pose_face: bool,
    pose_body: bool,
    pose_hands: bool,
    pose_source_ref: String,
    pose_rig_id: String,
    pose_marker_family: String,
    pose_marker_index: i32,
    pose_marker_x: f32,
    pose_marker_y: f32,
    pose_marker_confidence: f32,
    pose_marker_status: String,
    pose_marker_edits: Vec<PosekitMarkerEditRecord>,
    pose_framing_preset: String,
    pose_framing_lens_mm: i32,
    pose_framing_padding_top_px: i32,
    pose_framing_padding_right_px: i32,
    pose_framing_padding_bottom_px: i32,
    pose_framing_padding_left_px: i32,
    pose_export_pending: bool,
    pose_export_request_seq: u64,
    pose_active_export_request: Option<u64>,
    pose_export_status: String,
    pose_last_export: Option<PosekitExportSnapshot>,
    /// MT-050 exported OpenPose PNG artifact byte fetch. These fields mirror the source-image fetch
    /// state but key off the backend export's `png_artifact_ref` and feed the right viewport.
    pose_export_png_fetch_seq: u64,
    pose_active_export_png_fetch: Option<u64>,
    pose_export_png_loaded_ref: Option<String>,
    pose_export_png_status: String,
    /// MT-043 Posekit source-image byte fetch. `pose_source_loaded_ref` is the trimmed source ref we
    /// last acted on (so a fetch fires once per ref change, not every frame); `pose_source_fetch_seq`
    /// mints monotonic request ids; `pose_active_source_fetch` is the in-flight id so a stale response
    /// for a superseded ref is ignored; `pose_source_status` is the human/Argus-readable load state.
    pose_source_fetch_seq: u64,
    pose_active_source_fetch: Option<u64>,
    pose_source_loaded_ref: Option<String>,
    pose_source_status: String,
    ingest_decision: IngestDecision,
    ingest_dataset_ref: String,
    ingest_character_ref: String,
    ingest_actor: String,
    ingest_tag_buffer: String,
    ingest_batch_note: String,
    ingest_event: String,
    ingest_date: String,
    ingest_location: String,
    ingest_link_passed: bool,
    ingest_contact_rows: String,
    ingest_contact_columns: String,
    ingest_contact_dpi: String,
    ingest_contact_include_labels: bool,
    ingest_contact_thumbnail_fit: String,
    ingest_contact_output_path: String,
    ingest_contact_export_pending: bool,
    ingest_contact_export_request_seq: u64,
    ingest_active_contact_export_request: Option<u64>,
    ingest_last_contact_sheet: Option<ContactSheetExportSnapshot>,
    ingest_contact_sheet_receipt: String,
    ingest_facial_profile: String,
    ingest_facial_analysis_pending: bool,
    ingest_facial_analysis_request_seq: u64,
    ingest_active_facial_analysis_request: Option<u64>,
    ingest_last_facial_analysis: Option<FacialIngestAnalysisSnapshot>,
    ingest_facial_analysis_receipt: String,
    // MT-030 Facial Review sub-mode state (review-queue / montage / export command family).
    facial_features_readout: String,
    facial_command_receipt: String,
    facial_review_shard_count: String,
    facial_review_claim_ttl: String,
    facial_review_session_ref: String,
    facial_review_session_id: String,
    facial_review_claim_shard: String,
    facial_review_steal_expired: bool,
    facial_review_claim_ref: String,
    facial_review_decision_item: String,
    facial_review_decision_verdict: String,
    facial_review_decision_reason: String,
    facial_review_decision_tags: String,
    facial_review_decision_notes: String,
    facial_review_status_readout: String,
    facial_review_montage_readout: String,
    facial_review_export_readout: String,
    facial_review_parse_notes: String,
    facial_review_montage_page: String,
    facial_review_montage_columns: String,
    facial_review_montage_rows: String,
    facial_review_montage_filter: String,
    facial_review_export_dataset: String,
    facial_review_export_repeats: String,
    facial_review_export_output: String,
    facial_review_export_allow_partial: bool,
    // Accumulated review lineage refs the model chains commands with (claim/status/montage/export).
    facial_review_claim_refs: Vec<String>,
    facial_review_decision_refs: Vec<String>,
    // Single in-flight facial-command gate + request sequencing (mirrors the analyze gate).
    facial_review_pending: bool,
    facial_review_request_seq: u64,
    facial_review_active_request: Option<u64>,
    facial_features_pending: bool,
    facial_features_request_seq: u64,
    facial_features_active_request: Option<u64>,
    ingest_status: String,
    ingest_item_decisions: BTreeMap<String, IngestDecision>,
    ingest_persisted_item_ids: BTreeSet<String>,
    ingest_apply_pending: bool,
    ingest_apply_request_id: Option<String>,
    ingest_apply_batch_id: Option<String>,
    ingest_apply_actor_id: Option<String>,
    ingest_last_apply_receipt: String,
    // WP-CKC MT-042: operator-facing "Settings / Defaults" region state. These edit
    // buffers hold the pending default values; on load/save/reset they are applied to
    // the live runtime fields above (active_tab, ckc_book_mode, pose_*, ingest_*).
    settings_load_requested: bool,
    settings_loading: bool,
    settings_loaded: bool,
    settings_save_pending: bool,
    settings_status: String,
    settings_landing_tab: String,
    settings_ckc_book_mode: String,
    settings_pose_framing_preset: String,
    settings_pose_lens_mm: String,
    settings_pose_padding_top: String,
    settings_pose_padding_right: String,
    settings_pose_padding_bottom: String,
    settings_pose_padding_left: String,
    settings_pose_face: bool,
    settings_pose_body: bool,
    settings_pose_hands: bool,
    settings_ingest_batch_tags: String,
    settings_ingest_policy: String,
    settings_rows: Vec<AtelierPreferenceRow>,
    model_ops_actor_id: String,
    model_ops_session_id: String,
    model_ops_thread_id: String,
    model_ops_lease_id: String,
    model_ops_active_lease_thread_id: String,
    model_ops_ttl_seconds: String,
    model_ops_action_id: String,
    model_ops_receipt_status: String,
    model_ops_receipt_error_class: String,
    model_ops_receipt_recovery_hint: String,
    model_ops_request_seq: u64,
    model_ops_active_request: Option<u64>,
    model_ops_status: String,
    // MT-020: prompt-feedback sub-mode (nested under the INGEST tab). A WIP
    // surface for the deterministic prompt-feedback kernel: import panel, case
    // list grouped by segment/cell/render-stack, failure-tag picker, deterministic
    // rewrite preview + trace, and export button.
    ingest_mode: IngestMode,
    prompt_feedback_project: String,
    prompt_feedback_adapter: String,
    prompt_feedback_rows: String,
    prompt_feedback_rule_pack: String,
    prompt_feedback_selected_tags: BTreeSet<String>,
    prompt_feedback_selected_case: Option<String>,
    prompt_feedback_rewrite_preview: String,
    prompt_feedback_trace: String,
    prompt_feedback_export_status: String,
    prompt_feedback_status: String,
    prompt_feedback_cases: Vec<PromptFeedbackCaseRow>,
    prompt_feedback_request_seq: u64,
    prompt_feedback_import_pending: bool,
    prompt_feedback_rewrite_pending: bool,
    prompt_feedback_export_pending: bool,
    prompt_feedback_active_import_request: Option<u64>,
    prompt_feedback_active_rewrite_request: Option<u64>,
    prompt_feedback_active_export_request: Option<u64>,
    prompt_feedback_active_import_fingerprint: Option<String>,
    prompt_feedback_active_rewrite_fingerprint: Option<String>,
    prompt_feedback_active_export_fingerprint: Option<String>,
}

impl Default for AtelierPanelState {
    fn default() -> Self {
        let ckc_characters = seeded_ckc_characters();
        let ckc_search_results = seeded_ckc_search_results(&ckc_characters);
        Self {
            active_tab: AtelierPanelTab::CastkitCodex,
            ckc_book_mode: CkcBookMode::Sheet,
            ckc_characters,
            ckc_selected_index: 0,
            ckc_new_display_name: "New character".to_owned(),
            ckc_backend_loaded: false,
            ckc_load_requested: false,
            ckc_loading: false,
            ckc_create_pending: false,
            ckc_append_pending: false,
            ckc_sheet_status: "CKC sheet-version controls ready.".to_owned(),
            ckc_template_pending: false,
            ckc_safe_subset_pending: false,
            ckc_template_status:
                "Built-in CKC template: not loaded; use Load template or Safe subset.".to_owned(),
            ckc_import_text: String::new(),
            ckc_import_pending: false,
            ckc_export_pending: false,
            ckc_export_status: "No CKC sheet export requested.".to_owned(),
            ckc_last_export: None,
            ckc_field_suggestion_id: "CHAR-ID-006".to_owned(),
            ckc_field_suggestion_pending: false,
            ckc_field_suggestion_status: "No CKC field suggestions loaded.".to_owned(),
            ckc_field_suggestions: Vec::new(),
            ckc_sheet_artifact_pending: false,
            ckc_sheet_artifact_status:
                "Sheet artifact links ready: attach Posekit/OpenPose or Comfy refs to the current sheet version."
                    .to_owned(),
            ckc_sheet_artifact_kind: "openpose_png".to_owned(),
            ckc_sheet_artifact_ref: "artifact://atelier/comfy/render/example.png".to_owned(),
            ckc_sheet_artifact_manifest_ref: "receipt://atelier/comfy/example".to_owned(),
            ckc_sheet_artifact_label: "reusable CUI artifact".to_owned(),
            ckc_sheet_artifact_reuse_role: "cui_identity_reference".to_owned(),
            ckc_sheet_artifact_actor_id: String::new(),
            ckc_selected_sheet_artifact_link_id: None,
            ckc_sheet_artifact_reuse_ref: String::new(),
            ckc_media_save_pending: false,
            ckc_selected_media_key: None,
            ckc_media_fetch_seq: 0,
            ckc_active_media_fetch: None,
            ckc_media_fetch_asset: None,
            ckc_media_loaded_ref: None,
            ckc_media_image_status: "No linked image loaded.".to_owned(),
            ckc_selected_album_collection_id: None,
            ckc_album_create_name: "Reference album".to_owned(),
            ckc_album_create_notes: String::new(),
            ckc_album_create_tags: "reference".to_owned(),
            ckc_album_link_asset_ids: String::new(),
            ckc_album_link_source_path_ref: String::new(),
            ckc_album_link_source_url_ref: String::new(),
            ckc_album_link_edit_source_path_ref: String::new(),
            ckc_album_link_edit_source_url_ref: String::new(),
            ckc_album_link_edit_clear_source_path: false,
            ckc_album_link_edit_clear_source_url: false,
            ckc_media_actor_id: "local-atelier-panel".to_owned(),
            ckc_album_create_pending: false,
            ckc_album_link_pending: false,
            ckc_album_page_pending: false,
            ckc_album_page_pending_collection_id: None,
            ckc_album_list_page_pending: false,
            ckc_album_list_page_pending_character_id: None,
            ckc_album_status: "CKC album controls ready".to_owned(),
            ckc_story_card_title: "New story card".to_owned(),
            ckc_story_card_body: "Reusable scene, continuity, or production beat.".to_owned(),
            ckc_story_beat_text: "Reusable story beat for this character.".to_owned(),
            ckc_story_status:
                "CKC story documents are separate from sheet notes, image notes, and tag notes."
                    .to_owned(),
            ckc_moodboard_status:
                "CKC moodboards use native Handshake moodboard snapshots, not Excalidraw."
                    .to_owned(),
            ckc_active_story_document_id: None,
            ckc_active_moodboard_document_id: None,
            ckc_moodboard_conflict_recovery: false,
            ckc_character_notes_buffer: String::new(),
            ckc_character_notes_source_key: None,
            ckc_character_notes_status:
                "Character sheet notes mirror the notes field inside the selected sheet.".to_owned(),
            ckc_search_query: String::new(),
            ckc_search_tags: String::new(),
            ckc_search_filter_selected_character: false,
            ckc_search_filter_selected_collection: false,
            ckc_search_filter_selected_media: false,
            ckc_search_use_selected_media_similarity: false,
            ckc_search_mode: CkcSearchMode::Fuzzy,
            ckc_search_pending: false,
            ckc_search_status: "Local CKC search ready".to_owned(),
            ckc_search_results,
            ckc_tag_note_tag: "training".to_owned(),
            ckc_tag_note_scope_ref: "atelier://collection/018f7848-1111-7000-9000-00000000a001"
                .to_owned(),
            ckc_tag_note_editor: "Use this tag for reusable CKC training/reference media."
                .to_owned(),
            ckc_tag_note_pending: false,
            ckc_error: None,
            pose_yaw: 0.0,
            pose_pitch: 0.0,
            pose_zoom: 1.0,
            pose_face: true,
            pose_body: true,
            pose_hands: false,
            pose_source_ref: "atelier://media/mira-demo/pose-source.png".to_owned(),
            pose_rig_id: String::new(),
            pose_marker_family: "face".to_owned(),
            pose_marker_index: 12,
            pose_marker_x: 321.0,
            pose_marker_y: 222.0,
            pose_marker_confidence: 0.87,
            pose_marker_status:
                "Posekit marker editor ready; staged edits apply to the next OpenPose export."
                    .to_owned(),
            pose_marker_edits: Vec::new(),
            pose_framing_preset: "standard".to_owned(),
            pose_framing_lens_mm: 50,
            pose_framing_padding_top_px: 0,
            pose_framing_padding_right_px: 0,
            pose_framing_padding_bottom_px: 0,
            pose_framing_padding_left_px: 0,
            pose_export_pending: false,
            pose_export_request_seq: 0,
            pose_active_export_request: None,
            pose_export_status: "No Posekit OpenPose export requested.".to_owned(),
            pose_last_export: None,
            pose_export_png_fetch_seq: 0,
            pose_active_export_png_fetch: None,
            pose_export_png_loaded_ref: None,
            pose_export_png_status: "No exported OpenPose PNG artifact loaded.".to_owned(),
            pose_source_fetch_seq: 0,
            pose_active_source_fetch: None,
            pose_source_loaded_ref: None,
            pose_source_status: "No source image loaded.".to_owned(),
            ingest_decision: IngestDecision::Unsure,
            ingest_dataset_ref: "dataset://atelier/inbox".to_owned(),
            ingest_character_ref: "atelier://character/mira-demo".to_owned(),
            ingest_actor: String::new(),
            ingest_tag_buffer: "event, outfit, source".to_owned(),
            ingest_batch_note:
                "Batch note applied to selected/pass/reject/unsure image review rows.".to_owned(),
            ingest_event: "source mining".to_owned(),
            ingest_date: "2026-06-30".to_owned(),
            ingest_location: "atelier intake".to_owned(),
            ingest_link_passed: false,
            ingest_contact_rows: "3".to_owned(),
            ingest_contact_columns: "4".to_owned(),
            ingest_contact_dpi: "300".to_owned(),
            ingest_contact_include_labels: true,
            ingest_contact_thumbnail_fit: "contain".to_owned(),
            ingest_contact_output_path: "artifact://atelier/contact-sheets/latest.svg".to_owned(),
            ingest_contact_export_pending: false,
            ingest_contact_export_request_seq: 0,
            ingest_active_contact_export_request: None,
            ingest_last_contact_sheet: None,
            ingest_contact_sheet_receipt: "No contact sheet export yet.".to_owned(),
            ingest_facial_profile: "quality+dedupe+identity".to_owned(),
            ingest_facial_analysis_pending: false,
            ingest_facial_analysis_request_seq: 0,
            ingest_active_facial_analysis_request: None,
            ingest_last_facial_analysis: None,
            ingest_facial_analysis_receipt: "No Facial Ingest analysis yet.".to_owned(),
            facial_features_readout:
                "No Facial capability registry loaded. Click Load Facial capabilities to read GET /atelier/facial/features."
                    .to_owned(),
            facial_command_receipt: "No Facial review command run yet.".to_owned(),
            facial_review_shard_count: "4".to_owned(),
            facial_review_claim_ttl: "900".to_owned(),
            facial_review_session_ref: String::new(),
            facial_review_session_id: String::new(),
            facial_review_claim_shard: String::new(),
            facial_review_steal_expired: false,
            facial_review_claim_ref: String::new(),
            facial_review_decision_item: String::new(),
            facial_review_decision_verdict: "pass".to_owned(),
            facial_review_decision_reason: String::new(),
            facial_review_decision_tags: String::new(),
            facial_review_decision_notes: String::new(),
            facial_review_status_readout: "No Facial review status replay yet.".to_owned(),
            facial_review_montage_readout: "No Facial montage content readout yet.".to_owned(),
            facial_review_export_readout: "No Facial export content readout yet.".to_owned(),
            facial_review_parse_notes: "numeric_parse_notes none".to_owned(),
            facial_review_montage_page: "0".to_owned(),
            facial_review_montage_columns: "5".to_owned(),
            facial_review_montage_rows: "4".to_owned(),
            facial_review_montage_filter: String::new(),
            facial_review_export_dataset: String::new(),
            facial_review_export_repeats: "10".to_owned(),
            facial_review_export_output: String::new(),
            facial_review_export_allow_partial: false,
            facial_review_claim_refs: Vec::new(),
            facial_review_decision_refs: Vec::new(),
            facial_review_pending: false,
            facial_review_request_seq: 0,
            facial_review_active_request: None,
            facial_features_pending: false,
            facial_features_request_seq: 0,
            facial_features_active_request: None,
            ingest_status:
                "Ingest ready: stage dataset metadata, set canonical batch defaults, override visible rows, export contact sheets, and run native Facial analysis."
                    .to_owned(),
            ingest_item_decisions: BTreeMap::new(),
            ingest_persisted_item_ids: BTreeSet::new(),
            ingest_apply_pending: false,
            ingest_apply_request_id: None,
            ingest_apply_batch_id: None,
            ingest_apply_actor_id: None,
            ingest_last_apply_receipt: "No backend apply receipt yet.".to_owned(),
            // WP-CKC MT-042: settings-region defaults mirror the backend registry
            // defaults, so the region is coherent before the live load resolves.
            settings_load_requested: false,
            settings_loading: false,
            settings_loaded: false,
            settings_save_pending: false,
            settings_status: "Atelier defaults: not loaded (open panel to load from PostgreSQL)."
                .to_owned(),
            settings_landing_tab: AtelierPanelTab::CastkitCodex.settings_token().to_owned(),
            settings_ckc_book_mode: CkcBookMode::Sheet.settings_token().to_owned(),
            settings_pose_framing_preset: "standard".to_owned(),
            settings_pose_lens_mm: "50".to_owned(),
            settings_pose_padding_top: "0".to_owned(),
            settings_pose_padding_right: "0".to_owned(),
            settings_pose_padding_bottom: "0".to_owned(),
            settings_pose_padding_left: "0".to_owned(),
            settings_pose_face: true,
            settings_pose_body: true,
            settings_pose_hands: false,
            settings_ingest_batch_tags: "event, outfit, source".to_owned(),
            settings_ingest_policy: "unsure".to_owned(),
            settings_rows: Vec::new(),
            model_ops_actor_id: "operator".to_owned(),
            model_ops_session_id: "atelier-model-ops-session".to_owned(),
            model_ops_thread_id: posekit_openpose_model_ops_thread_id(
                "atelier://media/default-posekit-source",
                Some("default-rig"),
            ),
            model_ops_lease_id: String::new(),
            model_ops_active_lease_thread_id: String::new(),
            model_ops_ttl_seconds: "900".to_owned(),
            model_ops_action_id: "kernel.action_catalog.view".to_owned(),
            model_ops_receipt_status: "succeeded".to_owned(),
            model_ops_receipt_error_class: String::new(),
            model_ops_receipt_recovery_hint: String::new(),
            model_ops_request_seq: 0,
            model_ops_active_request: None,
            model_ops_status:
                "Model Ops ready: inspect state, claim an exclusive semantic lease, mutate with actor/session/lease headers, then record a receipt."
                    .to_owned(),
            ingest_mode: IngestMode::DatasetIntake,
            prompt_feedback_project: "leeseo".to_owned(),
            prompt_feedback_adapter: "leeseo.cuipp.v1".to_owned(),
            prompt_feedback_rows: String::new(),
            prompt_feedback_rule_pack: PROMPT_FEEDBACK_SEED_RULE_PACK_ID.to_owned(),
            prompt_feedback_selected_tags: BTreeSet::new(),
            prompt_feedback_selected_case: None,
            prompt_feedback_rewrite_preview: String::new(),
            prompt_feedback_trace: String::new(),
            prompt_feedback_export_status: "No prompt-feedback export requested; with a backend client this dispatches POST /atelier/prompt-feedback/export, otherwise it is local preview only.".to_owned(),
            prompt_feedback_status: "Prompt-feedback ready: backend clients dispatch import/rewrite/export with Model Ops headers; no-backend mode stays local preview and never claims persistence.".to_owned(),
            prompt_feedback_cases: seeded_prompt_feedback_cases(),
            prompt_feedback_request_seq: 0,
            prompt_feedback_import_pending: false,
            prompt_feedback_rewrite_pending: false,
            prompt_feedback_export_pending: false,
            prompt_feedback_active_import_request: None,
            prompt_feedback_active_rewrite_request: None,
            prompt_feedback_active_export_request: None,
            prompt_feedback_active_import_fingerprint: None,
            prompt_feedback_active_rewrite_fingerprint: None,
            prompt_feedback_active_export_fingerprint: None,
        }
    }
}

// WP-CKC MT-042: the operator-default preference keys surfaced by the "Settings /
// Defaults" region. These match the backend `PREFERENCE_DEFINITIONS` keys exactly.
const SETTINGS_KEY_LANDING_TAB: &str = "atelier-ui.landing-tab";
const SETTINGS_KEY_CKC_BOOK_MODE: &str = "ckc.book-mode";
const SETTINGS_KEY_POSEKIT_FRAMING_PRESET: &str = "posekit.framing-preset";
const SETTINGS_KEY_POSEKIT_LENS_MM: &str = "posekit.framing-lens-mm";
const SETTINGS_KEY_POSEKIT_PADDING_TOP: &str = "posekit.framing-padding-top-px";
const SETTINGS_KEY_POSEKIT_PADDING_RIGHT: &str = "posekit.framing-padding-right-px";
const SETTINGS_KEY_POSEKIT_PADDING_BOTTOM: &str = "posekit.framing-padding-bottom-px";
const SETTINGS_KEY_POSEKIT_PADDING_LEFT: &str = "posekit.framing-padding-left-px";
const SETTINGS_KEY_POSEKIT_MARKER_FACE: &str = "posekit.marker-face";
const SETTINGS_KEY_POSEKIT_MARKER_BODY: &str = "posekit.marker-body";
const SETTINGS_KEY_POSEKIT_MARKER_HANDS: &str = "posekit.marker-hands";
const SETTINGS_KEY_INGEST_BATCH_TAGS: &str = "ingest.batch-tags";
const SETTINGS_KEY_INGEST_POLICY: &str = "ingest.default-policy";

/// The WP-CKC MT-042 keys the region owns (all 13, incl. the 4 PoseKit paddings).
/// Order drives the reset-button row layout and the changed-key save scan.
const SETTINGS_MANAGED_KEYS: &[&str] = &[
    SETTINGS_KEY_LANDING_TAB,
    SETTINGS_KEY_CKC_BOOK_MODE,
    SETTINGS_KEY_POSEKIT_FRAMING_PRESET,
    SETTINGS_KEY_POSEKIT_LENS_MM,
    SETTINGS_KEY_POSEKIT_PADDING_TOP,
    SETTINGS_KEY_POSEKIT_PADDING_RIGHT,
    SETTINGS_KEY_POSEKIT_PADDING_BOTTOM,
    SETTINGS_KEY_POSEKIT_PADDING_LEFT,
    SETTINGS_KEY_POSEKIT_MARKER_FACE,
    SETTINGS_KEY_POSEKIT_MARKER_BODY,
    SETTINGS_KEY_POSEKIT_MARKER_HANDS,
    SETTINGS_KEY_INGEST_BATCH_TAGS,
    SETTINGS_KEY_INGEST_POLICY,
];

// WP-CKC MT-042 (F6 intent): the enumerated token vocabularies, mirrored from the
// backend so the client can validate BEFORE sending and never 400 mid-batch (F1).
// The fields stay steerable TextInputs (argus.set_value is TextInput-only per
// mcp/action.rs); a native egui ComboBox would not be set_value-steerable.
const SETTINGS_LANDING_TAB_TOKENS: &[&str] = &["castkit-codex", "posekit", "ingest"];
const SETTINGS_CKC_BOOK_MODE_TOKENS: &[&str] = &["sheet", "story", "notes", "moodboard"];
const SETTINGS_INGEST_POLICY_TOKENS: &[&str] = &["pass", "reject", "unsure"];

const SETTINGS_LENS_MIN: i32 = 18;
const SETTINGS_LENS_MAX: i32 = 120;
const SETTINGS_PADDING_MIN: i32 = 0;
const SETTINGS_PADDING_MAX: i32 = 256;

/// WP-CKC MT-042 stable Argus author_id for a per-key reset button
/// (`atelier-settings-reset-<key>` with the dotted key hyphenated).
pub fn settings_reset_author_id(key: &str) -> String {
    format!("atelier-settings-reset-{}", key.replace('.', "-"))
}

/// The backend value_type token for a managed key.
fn settings_value_type(key: &str) -> &'static str {
    match key {
        SETTINGS_KEY_POSEKIT_LENS_MM
        | SETTINGS_KEY_POSEKIT_PADDING_TOP
        | SETTINGS_KEY_POSEKIT_PADDING_RIGHT
        | SETTINGS_KEY_POSEKIT_PADDING_BOTTOM
        | SETTINGS_KEY_POSEKIT_PADDING_LEFT => "integer",
        SETTINGS_KEY_POSEKIT_MARKER_FACE
        | SETTINGS_KEY_POSEKIT_MARKER_BODY
        | SETTINGS_KEY_POSEKIT_MARKER_HANDS => "bool",
        _ => "string",
    }
}

/// Normalize the current edit buffer for `key` into the exact value to persist, or
/// return a human-readable error (invalid enum / non-integer) so the whole save is
/// rejected BEFORE any PUT (F1: no partial save, no mid-batch 400).
fn settings_desired_value(state: &AtelierPanelState, key: &str) -> Result<String, String> {
    let enum_value = |buffer: &str, tokens: &[&str], label: &str| -> Result<String, String> {
        let value = buffer.trim();
        if tokens.contains(&value) {
            Ok(value.to_owned())
        } else {
            Err(format!("{label} '{value}' is not one of {tokens:?}"))
        }
    };
    let int_value = |buffer: &str, min: i32, max: i32, label: &str| -> Result<String, String> {
        match buffer.trim().parse::<i32>() {
            Ok(parsed) => Ok(parsed.clamp(min, max).to_string()),
            Err(_) => Err(format!("{label} '{}' must be an integer", buffer.trim())),
        }
    };
    match key {
        SETTINGS_KEY_LANDING_TAB => enum_value(
            &state.settings_landing_tab,
            SETTINGS_LANDING_TAB_TOKENS,
            "landing tab",
        ),
        SETTINGS_KEY_CKC_BOOK_MODE => enum_value(
            &state.settings_ckc_book_mode,
            SETTINGS_CKC_BOOK_MODE_TOKENS,
            "CKC book mode",
        ),
        // Framing preset is normalized to a canonical token (never invalid).
        SETTINGS_KEY_POSEKIT_FRAMING_PRESET => {
            Ok(posekit_framing_preset(&state.settings_pose_framing_preset))
        }
        SETTINGS_KEY_POSEKIT_LENS_MM => int_value(
            &state.settings_pose_lens_mm,
            SETTINGS_LENS_MIN,
            SETTINGS_LENS_MAX,
            "PoseKit lens mm",
        ),
        SETTINGS_KEY_POSEKIT_PADDING_TOP => int_value(
            &state.settings_pose_padding_top,
            SETTINGS_PADDING_MIN,
            SETTINGS_PADDING_MAX,
            "PoseKit top padding",
        ),
        SETTINGS_KEY_POSEKIT_PADDING_RIGHT => int_value(
            &state.settings_pose_padding_right,
            SETTINGS_PADDING_MIN,
            SETTINGS_PADDING_MAX,
            "PoseKit right padding",
        ),
        SETTINGS_KEY_POSEKIT_PADDING_BOTTOM => int_value(
            &state.settings_pose_padding_bottom,
            SETTINGS_PADDING_MIN,
            SETTINGS_PADDING_MAX,
            "PoseKit bottom padding",
        ),
        SETTINGS_KEY_POSEKIT_PADDING_LEFT => int_value(
            &state.settings_pose_padding_left,
            SETTINGS_PADDING_MIN,
            SETTINGS_PADDING_MAX,
            "PoseKit left padding",
        ),
        SETTINGS_KEY_POSEKIT_MARKER_FACE => Ok(state.settings_pose_face.to_string()),
        SETTINGS_KEY_POSEKIT_MARKER_BODY => Ok(state.settings_pose_body.to_string()),
        SETTINGS_KEY_POSEKIT_MARKER_HANDS => Ok(state.settings_pose_hands.to_string()),
        SETTINGS_KEY_INGEST_BATCH_TAGS => Ok(state.settings_ingest_batch_tags.trim().to_owned()),
        SETTINGS_KEY_INGEST_POLICY => enum_value(
            &state.settings_ingest_policy,
            SETTINGS_INGEST_POLICY_TOKENS,
            "ingest policy",
        ),
        other => Err(format!("unknown settings key {other}")),
    }
}

/// The last-loaded effective value for `key`, if a projection row is present.
fn settings_loaded_value<'a>(state: &'a AtelierPanelState, key: &str) -> Option<&'a str> {
    state
        .settings_rows
        .iter()
        .find(|row| row.key == key)
        .map(|row| row.value.as_str())
}

/// WP-CKC MT-042 (F1/F5): build the `(key, value, value_type)` entries for ONLY the
/// keys whose normalized buffer differs from the last-loaded value. Returns an error
/// (whole save rejected) if any enumerated/integer buffer is invalid, so a save is
/// atomic in intent: it never PUTs a value the backend would 400 on mid-batch.
fn settings_save_entries(
    state: &AtelierPanelState,
) -> Result<Vec<(String, String, String)>, String> {
    let mut entries = Vec::new();
    for key in SETTINGS_MANAGED_KEYS {
        let desired = settings_desired_value(state, key)?;
        let changed = settings_loaded_value(state, key)
            .map(|loaded| loaded != desired.as_str())
            .unwrap_or(true);
        if changed {
            entries.push((
                (*key).to_owned(),
                desired,
                settings_value_type(key).to_owned(),
            ));
        }
    }
    Ok(entries)
}

/// WP-CKC MT-042 (F2): sync the settings-region edit buffers AND the non-navigation
/// runtime defaults (PoseKit framing/lens/padding/markers, Ingest tags/policy) from a
/// loaded/saved projection. Deliberately does NOT touch the live-nav fields
/// (`active_tab`, `ckc_book_mode`) — those are seeded only on first load by
/// [`seed_settings_live_nav`], so Save/Reset never yank the operator's current view.
fn sync_settings_buffers(state: &mut AtelierPanelState, rows: Vec<AtelierPreferenceRow>) {
    for row in &rows {
        match row.key.as_str() {
            SETTINGS_KEY_LANDING_TAB => state.settings_landing_tab = row.value.clone(),
            SETTINGS_KEY_CKC_BOOK_MODE => state.settings_ckc_book_mode = row.value.clone(),
            SETTINGS_KEY_POSEKIT_FRAMING_PRESET => {
                state.settings_pose_framing_preset = row.value.clone();
                state.pose_framing_preset = row.value.clone();
            }
            SETTINGS_KEY_POSEKIT_LENS_MM => {
                let value = row
                    .value
                    .trim()
                    .parse::<i32>()
                    .map(|value| value.clamp(SETTINGS_LENS_MIN, SETTINGS_LENS_MAX))
                    .unwrap_or(state.pose_framing_lens_mm);
                state.pose_framing_lens_mm = value;
                state.settings_pose_lens_mm = value.to_string();
            }
            SETTINGS_KEY_POSEKIT_PADDING_TOP => {
                let value = settings_clamped_padding(row, state.pose_framing_padding_top_px);
                state.pose_framing_padding_top_px = value;
                state.settings_pose_padding_top = value.to_string();
            }
            SETTINGS_KEY_POSEKIT_PADDING_RIGHT => {
                let value = settings_clamped_padding(row, state.pose_framing_padding_right_px);
                state.pose_framing_padding_right_px = value;
                state.settings_pose_padding_right = value.to_string();
            }
            SETTINGS_KEY_POSEKIT_PADDING_BOTTOM => {
                let value = settings_clamped_padding(row, state.pose_framing_padding_bottom_px);
                state.pose_framing_padding_bottom_px = value;
                state.settings_pose_padding_bottom = value.to_string();
            }
            SETTINGS_KEY_POSEKIT_PADDING_LEFT => {
                let value = settings_clamped_padding(row, state.pose_framing_padding_left_px);
                state.pose_framing_padding_left_px = value;
                state.settings_pose_padding_left = value.to_string();
            }
            SETTINGS_KEY_POSEKIT_MARKER_FACE => {
                let enabled = row.value == "true";
                state.settings_pose_face = enabled;
                state.pose_face = enabled;
            }
            SETTINGS_KEY_POSEKIT_MARKER_BODY => {
                let enabled = row.value == "true";
                state.settings_pose_body = enabled;
                state.pose_body = enabled;
            }
            SETTINGS_KEY_POSEKIT_MARKER_HANDS => {
                let enabled = row.value == "true";
                state.settings_pose_hands = enabled;
                state.pose_hands = enabled;
            }
            SETTINGS_KEY_INGEST_BATCH_TAGS => {
                state.settings_ingest_batch_tags = row.value.clone();
                state.ingest_tag_buffer = row.value.clone();
            }
            SETTINGS_KEY_INGEST_POLICY => {
                state.settings_ingest_policy = row.value.clone();
                if let Some(decision) = IngestDecision::from_policy_token(&row.value) {
                    state.ingest_decision = decision;
                }
            }
            _ => {}
        }
    }
    state.settings_rows = rows;
}

fn settings_clamped_padding(row: &AtelierPreferenceRow, fallback: i32) -> i32 {
    row.value
        .trim()
        .parse::<i32>()
        .map(|value| value.clamp(SETTINGS_PADDING_MIN, SETTINGS_PADDING_MAX))
        .unwrap_or(fallback)
}

/// WP-CKC MT-042 (F2): seed the live-navigation defaults (`active_tab`,
/// `ckc_book_mode`) from the stored projection ONLY on the initial load, and only
/// when the operator/deep-link has NOT already navigated away from the construction
/// defaults this session. This prevents Save/Reset from yanking the active tab and
/// prevents overriding a `GO>Atelier>Posekit` deep-link (singleton panel race).
fn seed_settings_live_nav(state: &mut AtelierPanelState, rows: &[AtelierPreferenceRow]) {
    for row in rows {
        match row.key.as_str() {
            SETTINGS_KEY_LANDING_TAB => {
                if state.active_tab == AtelierPanelTab::CastkitCodex {
                    if let Some(tab) = AtelierPanelTab::from_settings_token(&row.value) {
                        state.active_tab = tab;
                    }
                }
            }
            SETTINGS_KEY_CKC_BOOK_MODE => {
                if state.ckc_book_mode == CkcBookMode::Sheet {
                    if let Some(mode) = CkcBookMode::from_settings_token(&row.value) {
                        state.ckc_book_mode = mode;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Count how many of the region's managed keys are present in a projection (F7).
fn settings_managed_key_count(rows: &[AtelierPreferenceRow]) -> usize {
    rows.iter()
        .filter(|row| SETTINGS_MANAGED_KEYS.contains(&row.key.as_str()))
        .count()
}

fn posekit_state_readout(state: &AtelierPanelState) -> String {
    let rig_id = posekit_optional_rig_id(&state.pose_rig_id).unwrap_or_else(|| "<none>".to_owned());
    let framing = posekit_framing_readout(state);
    format!(
        "source_ref={} rig_id={} yaw_deg={:.0} pitch_deg={:.0} zoom={:.2} markers={} staged_marker_edits={} {}",
        state.pose_source_ref,
        rig_id,
        state.pose_yaw,
        state.pose_pitch,
        state.pose_zoom,
        marker_layer_summary(state.pose_face, state.pose_body, state.pose_hands),
        state.pose_marker_edits.len(),
        framing
    )
}

fn posekit_optional_rig_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn marker_layer_summary(face: bool, body: bool, hands: bool) -> String {
    format!(
        "face:{} body:{} hands:{}",
        if face { "on" } else { "off" },
        if body { "on" } else { "off" },
        if hands { "on" } else { "off" }
    )
}

fn posekit_framing_preset(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "standard" => "standard".to_owned(),
        "full_body_with_feet" | "full_body" | "feet" => "full_body_with_feet".to_owned(),
        "portrait" => "portrait".to_owned(),
        "custom" => "custom".to_owned(),
        _ => "custom".to_owned(),
    }
}

fn posekit_framing_json_from_state(state: &AtelierPanelState) -> serde_json::Value {
    serde_json::json!({
        "preset": posekit_framing_preset(&state.pose_framing_preset),
        "lens_mm": state.pose_framing_lens_mm.clamp(18, 120),
        "padding_top_px": state.pose_framing_padding_top_px.clamp(0, 256),
        "padding_right_px": state.pose_framing_padding_right_px.clamp(0, 256),
        "padding_bottom_px": state.pose_framing_padding_bottom_px.clamp(0, 256),
        "padding_left_px": state.pose_framing_padding_left_px.clamp(0, 256),
    })
}

fn posekit_framing_readout(state: &AtelierPanelState) -> String {
    format!(
        "framing preset={} lens_mm={} top={} right={} bottom={} left={}",
        posekit_framing_preset(&state.pose_framing_preset),
        state.pose_framing_lens_mm.clamp(18, 120),
        state.pose_framing_padding_top_px.clamp(0, 256),
        state.pose_framing_padding_right_px.clamp(0, 256),
        state.pose_framing_padding_bottom_px.clamp(0, 256),
        state.pose_framing_padding_left_px.clamp(0, 256)
    )
}

fn posekit_marker_family(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "body" | "pose" => Ok("body".to_owned()),
        "face" | "facial" => Ok("face".to_owned()),
        "left_hand" | "lefthand" | "left" => Ok("left_hand".to_owned()),
        "right_hand" | "righthand" | "right" => Ok("right_hand".to_owned()),
        other => Err(format!(
            "unknown marker family '{other}'; use body, face, left_hand, or right_hand"
        )),
    }
}

fn posekit_marker_family_count(family: &str) -> usize {
    match family {
        "body" => POSEKIT_BODY_KEYPOINT_COUNT,
        "face" => POSEKIT_FACE_KEYPOINT_COUNT,
        "left_hand" | "right_hand" => POSEKIT_HAND_KEYPOINT_COUNT,
        _ => 0,
    }
}

fn posekit_marker_family_enabled(state: &AtelierPanelState, family: &str) -> bool {
    match family {
        "body" => state.pose_body,
        "face" => state.pose_face,
        "left_hand" | "right_hand" => state.pose_hands,
        _ => false,
    }
}

fn posekit_generated_marker_slot_is_zero(
    state: &AtelierPanelState,
    family: &str,
    index: usize,
) -> bool {
    let yaw = state.pose_yaw.clamp(-180.0, 180.0);
    let pitch = state.pose_pitch.clamp(-45.0, 45.0);
    let zoom = state.pose_zoom.clamp(0.4, 2.2);
    let keypoints = match family {
        "body" => posekit_body_keypoints(yaw, pitch, zoom, state.pose_body),
        "face" => posekit_face_keypoints(yaw, pitch, zoom, state.pose_face),
        "left_hand" => posekit_hand_keypoints(yaw, pitch, zoom, state.pose_hands, -1.0),
        "right_hand" => posekit_hand_keypoints(yaw, pitch, zoom, state.pose_hands, 1.0),
        _ => return false,
    };
    let offset = index.saturating_mul(3);
    if offset + 2 >= keypoints.len() {
        return false;
    }
    keypoints[offset] == 0.0 && keypoints[offset + 1] == 0.0 && keypoints[offset + 2] == 0.0
}

#[derive(Clone, Copy)]
struct PosekitMarkerSlotState {
    zero: bool,
    locally_mutated: bool,
}

fn posekit_staged_marker_slot_state(
    state: &AtelierPanelState,
    family: &str,
    index: usize,
) -> PosekitMarkerSlotState {
    let mut slot_state = PosekitMarkerSlotState {
        zero: posekit_generated_marker_slot_is_zero(state, family, index),
        locally_mutated: false,
    };
    for edit in &state.pose_marker_edits {
        if edit.family != family || edit.index != index {
            continue;
        }
        slot_state = match edit.action.as_str() {
            "remove" => PosekitMarkerSlotState {
                zero: true,
                locally_mutated: true,
            },
            "set" | "add" => PosekitMarkerSlotState {
                zero: false,
                locally_mutated: true,
            },
            _ => slot_state,
        };
    }
    slot_state
}

fn posekit_validate_marker_edit(
    state: &AtelierPanelState,
    action: &str,
) -> Result<PosekitMarkerEditRecord, String> {
    let family = posekit_marker_family(&state.pose_marker_family)?;
    if !posekit_marker_family_enabled(state, &family) {
        return Err(format!(
            "{family} marker layer is disabled; enable the layer before editing it"
        ));
    }
    let index = usize::try_from(state.pose_marker_index)
        .map_err(|_| "marker index must be zero or greater".to_owned())?;
    let limit = posekit_marker_family_count(&family);
    if index >= limit {
        return Err(format!(
            "{family}[{index}] is outside the supported 0..{} range",
            limit.saturating_sub(1)
        ));
    }
    let action = action.to_owned();
    if action == "remove" {
        return Ok(PosekitMarkerEditRecord {
            family,
            index,
            action,
            x: None,
            y: None,
            confidence: None,
        });
    }
    if !matches!(action.as_str(), "set" | "add") {
        return Err(format!("unsupported marker action '{action}'"));
    }
    let backend_rig_will_validate_source_slot =
        posekit_optional_rig_id(&state.pose_rig_id).is_some();
    let slot_state = posekit_staged_marker_slot_state(state, &family, index);
    if action == "add"
        && !slot_state.zero
        && !(backend_rig_will_validate_source_slot && !slot_state.locally_mutated)
    {
        return Err(format!(
            "add would overwrite existing {family}[{index}]; use apply for existing markers, remove it first, or bind a stored rig so the backend can validate the source slot"
        ));
    }
    for (label, value) in [("x", state.pose_marker_x), ("y", state.pose_marker_y)] {
        if !value.is_finite() || !(0.0..=POSEKIT_EXPORT_WIDTH as f32).contains(&value) {
            return Err(format!(
                "{label} must be a finite coordinate inside the 768px canvas"
            ));
        }
    }
    if !state.pose_marker_confidence.is_finite()
        || !(0.0..=1.0).contains(&state.pose_marker_confidence)
    {
        return Err("confidence must be finite and between 0.0 and 1.0".to_owned());
    }
    Ok(PosekitMarkerEditRecord {
        family,
        index,
        action,
        x: Some((state.pose_marker_x * 10.0).round() / 10.0),
        y: Some((state.pose_marker_y * 10.0).round() / 10.0),
        confidence: Some((state.pose_marker_confidence * 100.0).round() / 100.0),
    })
}

fn posekit_stage_marker_edit(state: &mut AtelierPanelState, action: &str) {
    match posekit_validate_marker_edit(state, action) {
        Ok(edit) => {
            let verb = match edit.action.as_str() {
                "add" => "Added marker edit",
                "remove" => "Removed marker",
                _ => "Applied marker edit",
            };
            state.pose_marker_status = format!("{verb} {}[{}]", edit.family, edit.index);
            state.pose_marker_edits.push(edit);
        }
        Err(err) => {
            state.pose_marker_status = format!("Marker edit rejected: {err}");
        }
    }
}

fn posekit_validate_staged_marker_edits_for_export(
    state: &AtelierPanelState,
    allow_backend_source_validation: bool,
) -> Result<(), String> {
    let backend_can_validate_source_slot =
        allow_backend_source_validation && posekit_optional_rig_id(&state.pose_rig_id).is_some();
    let mut marker_slot_state: std::collections::BTreeMap<(String, usize), PosekitMarkerSlotState> =
        std::collections::BTreeMap::new();

    for edit in &state.pose_marker_edits {
        let family = posekit_marker_family(&edit.family)?;
        if !posekit_marker_family_enabled(state, &family) {
            return Err(format!(
                "staged marker edit {}[{}] targets a disabled marker layer",
                family, edit.index
            ));
        }
        let limit = posekit_marker_family_count(&family);
        if edit.index >= limit {
            return Err(format!(
                "staged marker edit {}[{}] is outside the supported 0..{} range",
                family,
                edit.index,
                limit.saturating_sub(1)
            ));
        }
        let key = (family.clone(), edit.index);
        let slot_state =
            *marker_slot_state
                .entry(key.clone())
                .or_insert_with(|| PosekitMarkerSlotState {
                    zero: posekit_generated_marker_slot_is_zero(state, &family, edit.index),
                    locally_mutated: false,
                });

        match edit.action.as_str() {
            "remove" => {
                marker_slot_state.insert(
                    key,
                    PosekitMarkerSlotState {
                        zero: true,
                        locally_mutated: true,
                    },
                );
            }
            "set" => {
                posekit_validate_staged_marker_payload(edit)?;
                marker_slot_state.insert(
                    key,
                    PosekitMarkerSlotState {
                        zero: false,
                        locally_mutated: true,
                    },
                );
            }
            "add" => {
                posekit_validate_staged_marker_payload(edit)?;
                if !slot_state.zero
                    && !(backend_can_validate_source_slot && !slot_state.locally_mutated)
                {
                    return Err(format!(
                        "staged marker add {}[{}] needs an empty local slot or backend source validation",
                        family, edit.index
                    ));
                }
                marker_slot_state.insert(
                    key,
                    PosekitMarkerSlotState {
                        zero: false,
                        locally_mutated: true,
                    },
                );
            }
            other => {
                return Err(format!("unsupported staged marker action '{other}'"));
            }
        }
    }

    Ok(())
}

fn posekit_validate_staged_marker_payload(edit: &PosekitMarkerEditRecord) -> Result<(), String> {
    let Some(x) = edit.x else {
        return Err(format!(
            "staged marker edit {}[{}] is missing x",
            edit.family, edit.index
        ));
    };
    let Some(y) = edit.y else {
        return Err(format!(
            "staged marker edit {}[{}] is missing y",
            edit.family, edit.index
        ));
    };
    let Some(confidence) = edit.confidence else {
        return Err(format!(
            "staged marker edit {}[{}] is missing confidence",
            edit.family, edit.index
        ));
    };
    for (label, value, max) in [
        ("x", x, POSEKIT_EXPORT_WIDTH as f32),
        ("y", y, POSEKIT_EXPORT_HEIGHT as f32),
    ] {
        if !value.is_finite() || !(0.0..=max).contains(&value) {
            return Err(format!(
                "staged marker edit {}[{}] has invalid {label}; coordinates must stay inside the 768px canvas",
                edit.family, edit.index
            ));
        }
    }
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err(format!(
            "staged marker edit {}[{}] has invalid confidence",
            edit.family, edit.index
        ));
    }
    Ok(())
}

fn posekit_staged_edits_target_family(state: &AtelierPanelState, family: &str) -> bool {
    state
        .pose_marker_edits
        .iter()
        .any(|edit| edit.family == family)
}

fn posekit_warn_for_disabled_staged_marker_edits(state: &mut AtelierPanelState) {
    let mut disabled_families = Vec::new();
    if !state.pose_body && posekit_staged_edits_target_family(state, "body") {
        disabled_families.push("body");
    }
    if !state.pose_face && posekit_staged_edits_target_family(state, "face") {
        disabled_families.push("face");
    }
    if !state.pose_hands
        && (posekit_staged_edits_target_family(state, "left_hand")
            || posekit_staged_edits_target_family(state, "right_hand"))
    {
        disabled_families.push("hands");
    }
    if disabled_families.is_empty() {
        return;
    }
    state.pose_marker_status = format!(
        "Staged marker edits target disabled {} layer(s); re-enable the layer or clear edits before export.",
        disabled_families.join(", ")
    );
}

fn posekit_nudge_marker(state: &mut AtelierPanelState, dx: f32, dy: f32) {
    state.pose_marker_x = (state.pose_marker_x + dx).clamp(0.0, POSEKIT_EXPORT_WIDTH as f32);
    state.pose_marker_y = (state.pose_marker_y + dy).clamp(0.0, POSEKIT_EXPORT_HEIGHT as f32);
    state.pose_marker_status = format!(
        "Nudged marker candidate to x={:.1} y={:.1}; apply to stage it.",
        state.pose_marker_x, state.pose_marker_y
    );
}

fn posekit_marker_edits_json(edits: &[PosekitMarkerEditRecord]) -> Vec<serde_json::Value> {
    edits
        .iter()
        .map(|edit| {
            serde_json::json!({
                "family": edit.family.as_str(),
                "index": edit.index,
                "action": edit.action.as_str(),
                "x": edit.x,
                "y": edit.y,
                "confidence": edit.confidence,
            })
        })
        .collect()
}

fn posekit_export_snapshot(state: &AtelierPanelState) -> Result<PosekitExportSnapshot, String> {
    posekit_validate_staged_marker_edits_for_export(state, false)?;
    let yaw = state.pose_yaw.clamp(-180.0, 180.0);
    let pitch = state.pose_pitch.clamp(-45.0, 45.0);
    let zoom = state.pose_zoom.clamp(0.4, 2.2);
    let source_ref = if state.pose_source_ref.trim().is_empty() {
        "atelier://posekit/blank-source".to_owned()
    } else {
        state.pose_source_ref.trim().to_owned()
    };
    let rig_id = posekit_optional_rig_id(&state.pose_rig_id);
    let marker_edits = posekit_marker_edits_json(&state.pose_marker_edits);
    let framing = posekit_framing_json_from_state(state);
    let openpose_json = posekit_openpose_json(
        &source_ref,
        rig_id.as_deref(),
        yaw,
        pitch,
        zoom,
        state.pose_face,
        state.pose_body,
        state.pose_hands,
        &marker_edits,
        &framing,
    );
    posekit_validate_local_openpose_export(&openpose_json, state.pose_body)?;
    let hash_basis = format!(
        "{}|{}|{yaw:.0}|{pitch:.0}|{zoom:.2}|{}|{}|{}|{}|{}",
        source_ref,
        rig_id.as_deref().unwrap_or("<none>"),
        state.pose_face,
        state.pose_body,
        state.pose_hands,
        framing,
        openpose_json
    );
    let content_hash = stable_posekit_hash(&hash_basis);
    // OFFLINE-PREVIEW-ONLY refs. The no-backend path deliberately emits `preview://` (never
    // `artifact://`) and materializes no PNG bytes, so it can never be mistaken for the real backend
    // export. The real export path (`AtelierClient::export_posekit_openpose`) produces `artifact://`
    // ArtifactStore refs; the UI status labels this preview explicitly as "not a real artifact".
    let png_artifact_ref = format!("preview://atelier/posekit/openpose/{content_hash}/png/payload");
    let png_manifest_ref =
        format!("preview://atelier/posekit/openpose/{content_hash}/png/manifest");
    let json_artifact_ref =
        format!("preview://atelier/posekit/openpose/{content_hash}/json/payload");
    let json_manifest_ref =
        format!("preview://atelier/posekit/openpose/{content_hash}/json/manifest");
    let receipt_ref = format!("preview://atelier/posekit/openpose/{content_hash}/receipt");
    Ok(PosekitExportSnapshot {
        source_ref,
        rig_id,
        yaw_deg: yaw,
        pitch_deg: pitch,
        zoom,
        face: state.pose_face,
        body: state.pose_body,
        hands: state.pose_hands,
        png_artifact_ref,
        png_manifest_ref,
        json_artifact_ref,
        json_manifest_ref,
        receipt_ref,
        content_hash,
        openpose_json,
        framing,
        applied_marker_edit_count: state.pose_marker_edits.len(),
    })
}

fn posekit_export_snapshot_from_backend(row: AtelierPosekitExportRow) -> PosekitExportSnapshot {
    PosekitExportSnapshot {
        source_ref: row.source_ref,
        rig_id: row.rig_id,
        yaw_deg: row.yaw_deg as f32,
        pitch_deg: row.pitch_deg as f32,
        zoom: row.zoom_percent as f32 / 100.0,
        face: row.marker_layers.face,
        body: row.marker_layers.body,
        hands: row.marker_layers.hands,
        png_artifact_ref: row.openpose_png_artifact.artifact_ref,
        png_manifest_ref: row.openpose_png_artifact.manifest_ref,
        json_artifact_ref: row.openpose_json_artifact.artifact_ref,
        json_manifest_ref: row.openpose_json_artifact.manifest_ref,
        receipt_ref: row.receipt_ref,
        content_hash: row.content_hash,
        openpose_json: row.openpose_json,
        framing: row.framing,
        applied_marker_edit_count: row.applied_marker_edit_count,
    }
}

/// OFFLINE-PREVIEW-ONLY OpenPose JSON generator.
///
/// This is the frontend's local, no-backend preview generator. It intentionally reproduces the
/// backend `generate_posekit_openpose_export` keypoint layout so the split-view can show a faithful
/// live preview when no backend is connected, but it is NOT the real export: the produced JSON is
/// tagged `"preview_only": true`, and the local export path built on it emits `preview://` refs, never
/// `artifact://`. The backend (`AtelierClient::export_posekit_openpose` →
/// `handshake_core::atelier::pose`) remains the single source of truth for real exports; when a
/// backend export result exists the split-view binds to THAT `openpose_json` instead of this preview.
/// Keep the two in sync deliberately — this duplication exists only because the frontend cannot link
/// the backend crate's generator directly.
fn posekit_openpose_json(
    source_ref: &str,
    rig_id: Option<&str>,
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
    face: bool,
    body: bool,
    hands: bool,
    marker_edits: &[serde_json::Value],
    framing: &serde_json::Value,
) -> serde_json::Value {
    let mut openpose = serde_json::json!({
        "version": 1.3,
        "handshake_schema": "hsk.atelier.posekit.openpose_export@1",
        "preview_only": true,
        "source_ref": source_ref,
        "rig_id": rig_id,
        "canvas": {
            "width": POSEKIT_EXPORT_WIDTH,
            "height": POSEKIT_EXPORT_HEIGHT,
        },
        "pose_state": {
            "yaw_deg": yaw_deg.round(),
            "pitch_deg": pitch_deg.round(),
            "zoom": ((zoom * 100.0).round() / 100.0),
            "zoom_percent": (zoom * 100.0).round(),
            "marker_layers": {
                "face": face,
                "body": body,
                "hands": hands,
            },
            "marker_edits": marker_edits,
            "framing": framing,
        },
        "people": [{
            "pose_keypoints_2d": posekit_body_keypoints(yaw_deg, pitch_deg, zoom, body),
            "face_keypoints_2d": posekit_face_keypoints(yaw_deg, pitch_deg, zoom, face),
            "hand_left_keypoints_2d": posekit_hand_keypoints(yaw_deg, pitch_deg, zoom, hands, -1.0),
            "hand_right_keypoints_2d": posekit_hand_keypoints(yaw_deg, pitch_deg, zoom, hands, 1.0),
        }],
    });
    posekit_apply_framing_to_openpose(&mut openpose, framing);
    posekit_apply_marker_edits_to_openpose(&mut openpose, marker_edits);
    openpose
}

fn posekit_validate_local_openpose_export(
    openpose: &serde_json::Value,
    body_enabled: bool,
) -> Result<(), String> {
    let mut visible = 0usize;
    let body_visible = posekit_validate_local_openpose_field(
        openpose,
        "pose_keypoints_2d",
        POSEKIT_BODY_KEYPOINT_COUNT,
    )?;
    visible += body_visible;
    if body_enabled && body_visible == 0 {
        return Err(
            "Posekit body export cannot be all-zero after marker edits and framing".to_owned(),
        );
    }
    visible += posekit_validate_local_openpose_field(
        openpose,
        "face_keypoints_2d",
        POSEKIT_FACE_KEYPOINT_COUNT,
    )?;
    visible += posekit_validate_local_openpose_field(
        openpose,
        "hand_left_keypoints_2d",
        POSEKIT_HAND_KEYPOINT_COUNT,
    )?;
    visible += posekit_validate_local_openpose_field(
        openpose,
        "hand_right_keypoints_2d",
        POSEKIT_HAND_KEYPOINT_COUNT,
    )?;
    if visible == 0 {
        return Err(
            "Posekit OpenPose export would be blank after marker edits and framing".to_owned(),
        );
    }
    Ok(())
}

fn posekit_validate_local_openpose_field(
    openpose: &serde_json::Value,
    field: &str,
    expected_count: usize,
) -> Result<usize, String> {
    let Some(points) = openpose
        .get("people")
        .and_then(serde_json::Value::as_array)
        .and_then(|people| people.first())
        .and_then(|person| person.get(field))
        .and_then(serde_json::Value::as_array)
    else {
        return Err(format!("Posekit OpenPose field {field} is missing"));
    };
    if points.len() != expected_count.saturating_mul(3) {
        return Err(format!(
            "Posekit OpenPose field {field} has {} values but expected {}",
            points.len(),
            expected_count.saturating_mul(3)
        ));
    }
    let mut visible = 0usize;
    for triple in points.chunks_exact(3) {
        let x = posekit_json_number(&triple[0], field)?;
        let y = posekit_json_number(&triple[1], field)?;
        let confidence = posekit_json_number(&triple[2], field)?;
        if !(0.0..=1.0).contains(&confidence) {
            return Err(format!(
                "Posekit OpenPose field {field} confidence must be in 0..=1"
            ));
        }
        if confidence <= 0.0 {
            continue;
        }
        if x < 0.0 || y < 0.0 || x > POSEKIT_EXPORT_WIDTH as f64 || y > POSEKIT_EXPORT_HEIGHT as f64
        {
            return Err(format!(
                "Posekit OpenPose field {field} has a visible point outside the export canvas"
            ));
        }
        visible = visible.saturating_add(1);
    }
    Ok(visible)
}

fn posekit_json_number(value: &serde_json::Value, field: &str) -> Result<f64, String> {
    let Some(number) = value.as_f64() else {
        return Err(format!(
            "Posekit OpenPose field {field} contains a non-number"
        ));
    };
    if !number.is_finite() {
        return Err(format!(
            "Posekit OpenPose field {field} contains non-finite values"
        ));
    }
    Ok(number)
}

fn posekit_apply_framing_to_openpose(
    openpose: &mut serde_json::Value,
    framing: &serde_json::Value,
) {
    let lens_mm = framing
        .get("lens_mm")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(50.0)
        .clamp(18.0, 120.0) as f32;
    let padding_top = posekit_padding_from_framing(framing, "padding_top_px");
    let padding_right = posekit_padding_from_framing(framing, "padding_right_px");
    let padding_bottom = posekit_padding_from_framing(framing, "padding_bottom_px");
    let padding_left = posekit_padding_from_framing(framing, "padding_left_px");
    let content_width = (POSEKIT_EXPORT_WIDTH as f32 - padding_left - padding_right).max(128.0);
    let content_height = (POSEKIT_EXPORT_HEIGHT as f32 - padding_top - padding_bottom).max(128.0);
    let source_center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5;
    let source_center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.5;
    let content_center_x = padding_left + content_width * 0.5;
    let content_center_y = padding_top + content_height * 0.5;
    let lens_scale = lens_mm / 50.0;

    let Some(person) = openpose
        .get_mut("people")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|people| people.first_mut())
    else {
        return;
    };
    for field in [
        "pose_keypoints_2d",
        "face_keypoints_2d",
        "hand_left_keypoints_2d",
        "hand_right_keypoints_2d",
    ] {
        let Some(points) = person
            .get_mut(field)
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        let mut offset = 0;
        while offset + 2 < points.len() {
            let x = points[offset].as_f64().unwrap_or(0.0) as f32;
            let y = points[offset + 1].as_f64().unwrap_or(0.0) as f32;
            let confidence = points[offset + 2].as_f64().unwrap_or(0.0) as f32;
            if confidence > 0.0 {
                let framed_x = content_center_x + (x - source_center_x) * lens_scale;
                let framed_y = content_center_y + (y - source_center_y) * lens_scale;
                points[offset] = posekit_json_f32(framed_x);
                points[offset + 1] = posekit_json_f32(framed_y);
            }
            offset += 3;
        }
    }
}

fn posekit_apply_marker_edits_to_openpose(
    openpose: &mut serde_json::Value,
    marker_edits: &[serde_json::Value],
) {
    let Some(person) = openpose
        .get_mut("people")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|people| people.first_mut())
    else {
        return;
    };
    for edit in marker_edits {
        let family = edit
            .get("family")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let action = edit
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("set");
        let Some(index) = edit
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
        else {
            continue;
        };
        let Some(points) = posekit_keypoint_array_mut(person, family) else {
            continue;
        };
        let offset = index.saturating_mul(3);
        if offset + 2 >= points.len() {
            continue;
        }
        if action == "remove" {
            points[offset] = serde_json::json!(0.0);
            points[offset + 1] = serde_json::json!(0.0);
            points[offset + 2] = serde_json::json!(0.0);
            continue;
        }
        if action == "add" && !posekit_json_marker_slot_is_zero(points, offset) {
            continue;
        }
        let Some(x) = edit.get("x").and_then(serde_json::Value::as_f64) else {
            continue;
        };
        let Some(y) = edit.get("y").and_then(serde_json::Value::as_f64) else {
            continue;
        };
        let Some(confidence) = edit.get("confidence").and_then(serde_json::Value::as_f64) else {
            continue;
        };
        points[offset] = posekit_json_f32(x as f32);
        points[offset + 1] = posekit_json_f32(y as f32);
        points[offset + 2] = posekit_json_confidence(confidence as f32);
    }
}

fn posekit_padding_from_framing(framing: &serde_json::Value, field: &str) -> f32 {
    framing
        .get(field)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 256.0) as f32
}

fn posekit_keypoint_array_mut<'a>(
    person: &'a mut serde_json::Value,
    family: &str,
) -> Option<&'a mut Vec<serde_json::Value>> {
    let field = match family {
        "body" => "pose_keypoints_2d",
        "face" => "face_keypoints_2d",
        "left_hand" => "hand_left_keypoints_2d",
        "right_hand" => "hand_right_keypoints_2d",
        _ => return None,
    };
    person
        .get_mut(field)
        .and_then(serde_json::Value::as_array_mut)
}

fn posekit_json_marker_slot_is_zero(points: &[serde_json::Value], offset: usize) -> bool {
    points
        .get(offset)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        == 0.0
        && points
            .get(offset + 1)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
            == 0.0
        && points
            .get(offset + 2)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
            == 0.0
}

fn posekit_json_f32(value: f32) -> serde_json::Value {
    serde_json::json!((value * 10.0).round() / 10.0)
}

fn posekit_json_confidence(value: f32) -> serde_json::Value {
    serde_json::json!((value * 100.0).round() / 100.0)
}

/// OFFLINE-PREVIEW-ONLY body-keypoint generator (see [`posekit_openpose_json`]). Duplicates the
/// backend `posekit_body_keypoints` layout for the no-backend live preview; not authoritative.
fn posekit_body_keypoints(yaw_deg: f32, pitch_deg: f32, zoom: f32, visible: bool) -> Vec<f32> {
    if !visible {
        return zero_keypoints(POSEKIT_BODY_KEYPOINT_COUNT);
    }
    let center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5 + yaw_deg / 180.0 * 72.0;
    let center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.51 + pitch_deg / 45.0 * 42.0;
    let scale = zoom.clamp(0.4, 2.2);
    let yaw_bias = yaw_deg / 180.0;
    let shoulder = 86.0 * scale * (1.0 - yaw_bias.abs() * 0.28);
    let hip = 52.0 * scale * (1.0 - yaw_bias.abs() * 0.18);
    let points = [
        (center_x, center_y - 170.0 * scale, 0.95),
        (center_x, center_y - 102.0 * scale, 0.94),
        (center_x - shoulder, center_y - 92.0 * scale, 0.91),
        (
            center_x - shoulder - 54.0 * scale,
            center_y - 34.0 * scale,
            0.86,
        ),
        (
            center_x - shoulder - 70.0 * scale,
            center_y + 34.0 * scale,
            0.82,
        ),
        (center_x + shoulder, center_y - 92.0 * scale, 0.91),
        (
            center_x + shoulder + 54.0 * scale,
            center_y - 34.0 * scale,
            0.86,
        ),
        (
            center_x + shoulder + 70.0 * scale,
            center_y + 34.0 * scale,
            0.82,
        ),
        (center_x - hip, center_y + 46.0 * scale, 0.90),
        (
            center_x - hip - 22.0 * scale,
            center_y + 142.0 * scale,
            0.86,
        ),
        (
            center_x - hip - 18.0 * scale,
            center_y + 238.0 * scale,
            0.82,
        ),
        (center_x + hip, center_y + 46.0 * scale, 0.90),
        (
            center_x + hip + 22.0 * scale,
            center_y + 142.0 * scale,
            0.86,
        ),
        (
            center_x + hip + 18.0 * scale,
            center_y + 238.0 * scale,
            0.82,
        ),
        (
            center_x - 18.0 * scale - yaw_bias * 8.0,
            center_y - 180.0 * scale,
            0.80,
        ),
        (
            center_x + 18.0 * scale - yaw_bias * 8.0,
            center_y - 180.0 * scale,
            0.80,
        ),
        (
            center_x - 42.0 * scale - yaw_bias * 10.0,
            center_y - 164.0 * scale,
            0.76,
        ),
        (
            center_x + 42.0 * scale - yaw_bias * 10.0,
            center_y - 164.0 * scale,
            0.76,
        ),
    ];
    flatten_keypoints(&points)
}

/// OFFLINE-PREVIEW-ONLY face-keypoint generator (see [`posekit_openpose_json`]); not authoritative.
fn posekit_face_keypoints(yaw_deg: f32, pitch_deg: f32, zoom: f32, visible: bool) -> Vec<f32> {
    if !visible {
        return zero_keypoints(POSEKIT_FACE_KEYPOINT_COUNT);
    }
    let center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5 + yaw_deg / 180.0 * 72.0;
    let center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.51 + pitch_deg / 45.0 * 42.0 - 170.0 * zoom;
    let scale = zoom.clamp(0.4, 2.2);
    let yaw_bias = yaw_deg / 180.0;
    let mut points = Vec::with_capacity(POSEKIT_FACE_KEYPOINT_COUNT);
    for index in 0..POSEKIT_FACE_KEYPOINT_COUNT {
        let theta = index as f32 / POSEKIT_FACE_KEYPOINT_COUNT as f32 * std::f32::consts::TAU;
        let x = center_x
            + theta.cos() * 34.0 * scale * (1.0 - yaw_bias.abs() * 0.32)
            + yaw_bias * 14.0 * scale;
        let y = center_y + theta.sin() * 45.0 * scale;
        points.push((x, y, 0.78));
    }
    flatten_keypoints(&points)
}

/// OFFLINE-PREVIEW-ONLY hand-keypoint generator (see [`posekit_openpose_json`]); not authoritative.
fn posekit_hand_keypoints(
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
    visible: bool,
    side: f32,
) -> Vec<f32> {
    if !visible {
        return zero_keypoints(POSEKIT_HAND_KEYPOINT_COUNT);
    }
    let center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5
        + yaw_deg / 180.0 * 72.0
        + side * 158.0 * zoom.clamp(0.4, 2.2);
    let center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.51 + pitch_deg / 45.0 * 42.0 + 34.0 * zoom;
    let scale = zoom.clamp(0.4, 2.2);
    let mut points = Vec::with_capacity(POSEKIT_HAND_KEYPOINT_COUNT);
    for index in 0..POSEKIT_HAND_KEYPOINT_COUNT {
        let finger = (index / 4) as f32;
        let joint = (index % 4) as f32;
        points.push((
            center_x + side * (finger - 2.0) * 8.0 * scale,
            center_y - joint * 13.0 * scale - finger * 2.0 * scale,
            0.70,
        ));
    }
    flatten_keypoints(&points)
}

fn flatten_keypoints(points: &[(f32, f32, f32)]) -> Vec<f32> {
    let mut flattened = Vec::with_capacity(points.len() * 3);
    for (x, y, confidence) in points {
        flattened.push((x * 10.0).round() / 10.0);
        flattened.push((y * 10.0).round() / 10.0);
        flattened.push((confidence * 100.0).round() / 100.0);
    }
    flattened
}

fn zero_keypoints(count: usize) -> Vec<f32> {
    vec![0.0; count * 3]
}

fn stable_posekit_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn posekit_export_preview(snapshot: &PosekitExportSnapshot) -> String {
    let rig_id = snapshot.rig_id.as_deref().unwrap_or("<none>");
    format!(
        "schema=hsk.atelier.posekit.openpose_export@1\nsource_ref={}\nrig_id={}\nyaw_deg={:.0}\npitch_deg={:.0}\nzoom={:.2}\nmarkers={}\napplied_marker_edit_count={}\nframing={}\npng_artifact_ref={}\npng_manifest_ref={}\njson_artifact_ref={}\njson_manifest_ref={}\nreceipt_ref={}\ncontent_hash={}\npng_mime=image/png\njson_mime=application/json\nopenpose_json={}",
        snapshot.source_ref,
        rig_id,
        snapshot.yaw_deg,
        snapshot.pitch_deg,
        snapshot.zoom,
        snapshot.marker_layers(),
        snapshot.applied_marker_edit_count,
        snapshot.framing,
        snapshot.png_artifact_ref,
        snapshot.png_manifest_ref,
        snapshot.json_artifact_ref,
        snapshot.json_manifest_ref,
        snapshot.receipt_ref,
        snapshot.content_hash,
        snapshot.openpose_json
    )
}

fn seeded_ckc_characters() -> Vec<CkcCharacterRecord> {
    vec![
        CkcCharacterRecord {
            public_id: "mira-demo".to_owned(),
            display_name: "Mira Demo".to_owned(),
            character_internal_id: "018f7848-1111-7000-9000-000000000001".to_owned(),
            character_ref: "atelier://character/018f7848-1111-7000-9000-000000000001".to_owned(),
            sheet_version_id: Some("018f7848-1111-7000-9000-000000000101".to_owned()),
            parent_sheet_version_id: None,
            sheet_seq: 1,
            sheet_editor_text: "CHAR-ID-001 — Character_ID: mira-demo\nCHAR-ID-002 — Name: Mira Demo\nCHAR-ID-006 — Primary_Role: reusable character/avatar\nPIPELINES\npipelines: ComfyUI, Unreal, Blender\nnotes: seed CKC sheet for Argus and model workflow proof".to_owned(),
            sheet_version_ref: Some(
                "atelier://sheet/018f7848-1111-7000-9000-000000000001/018f7848-1111-7000-9000-000000000101".to_owned(),
            ),
            sheet_artifact_links: vec![seeded_ckc_sheet_artifact_link(
                "018f7848-1111-7000-9000-00000000e001",
                "018f7848-1111-7000-9000-000000000001",
                "018f7848-1111-7000-9000-000000000101",
                "openpose_png",
                "artifact://atelier/posekit/openpose/mira-demo-yaw45.png",
                "manifest://atelier/posekit/openpose/mira-demo-yaw45",
                "posekit://rig/mira-demo-yaw45",
                "Mira yaw +45 OpenPose",
                "cui_openpose_conditioning",
            )],
            media_album_offset: 0,
            media_album_limit: 2,
            media_album_member_limit: 1,
            media_album_count: 2,
            media_albums_next_offset: None,
            media_albums: vec![
                seeded_ckc_media_album(
                    "018f7848-1111-7000-9000-00000000a001",
                    "Mira reference album",
                    "018f7848-1111-7000-9000-00000000b001",
                    "mira-closeup-001.png",
                    "atelier://folder/mira-reference-set",
                    "https://example.invalid/reference/mira-reference-set",
                ),
                seeded_ckc_media_album(
                    "018f7848-1111-7000-9000-00000000a003",
                    "Mira expression set",
                    "018f7848-1111-7000-9000-00000000b003",
                    "mira-expression-002.png",
                    "atelier://folder/mira-expression-set",
                    "https://example.invalid/reference/mira-expression-set",
                ),
            ],
            story_documents: vec![
                seeded_ckc_story_document(
                    "018f7848-1111-7000-9000-00000000c001",
                    "Mira story bible",
                    "018f7848-1111-7000-9000-00000000c101",
                ),
                seeded_ckc_story_document(
                    "018f7848-1111-7000-9000-00000000c002",
                    "Mira production scenes",
                    "018f7848-1111-7000-9000-00000000c102",
                ),
            ],
            moodboard_documents: vec![
                seeded_ckc_moodboard_document(
                    "018f7848-1111-7000-9000-00000000d001",
                    "018f7848-1111-7000-9000-00000000d101",
                    "Mira visual continuity board",
                ),
                seeded_ckc_moodboard_document(
                    "018f7848-1111-7000-9000-00000000d003",
                    "018f7848-1111-7000-9000-00000000d103",
                    "Mira production moodboard",
                ),
            ],
        },
        CkcCharacterRecord {
            public_id: "aria-demo".to_owned(),
            display_name: "Aria Demo".to_owned(),
            character_internal_id: "018f7848-1111-7000-9000-000000000002".to_owned(),
            character_ref: "atelier://character/018f7848-1111-7000-9000-000000000002".to_owned(),
            sheet_version_id: Some("018f7848-1111-7000-9000-000000000201".to_owned()),
            parent_sheet_version_id: None,
            sheet_seq: 1,
            sheet_editor_text: "CHAR-ID-001 — Character_ID: aria-demo\nCHAR-ID-002 — Name: Aria Demo\nCHAR-ID-006 — Primary_Role: production avatar reference\nPIPELINES\npipelines: CKC albums, Posekit, ComfyUI\nnotes: second selectable sheet proves CKC is a database surface".to_owned(),
            sheet_version_ref: Some(
                "atelier://sheet/018f7848-1111-7000-9000-000000000002/018f7848-1111-7000-9000-000000000201".to_owned(),
            ),
            sheet_artifact_links: Vec::new(),
            media_album_offset: 0,
            media_album_limit: 1,
            media_album_member_limit: 1,
            media_album_count: 1,
            media_albums_next_offset: None,
            media_albums: vec![seeded_ckc_media_album(
                "018f7848-1111-7000-9000-00000000a002",
                "Aria pose references",
                "018f7848-1111-7000-9000-00000000b002",
                "aria-pose-001.png",
                "atelier://folder/aria-pose-set",
                "https://example.invalid/reference/aria-pose-set",
            )],
            story_documents: vec![seeded_ckc_story_document(
                "018f7848-1111-7000-9000-00000000c002",
                "Aria production story",
                "018f7848-1111-7000-9000-00000000c102",
            )],
            moodboard_documents: vec![seeded_ckc_moodboard_document(
                "018f7848-1111-7000-9000-00000000d002",
                "018f7848-1111-7000-9000-00000000d102",
                "Aria pose board",
            )],
        },
    ]
}

fn seeded_ckc_sheet_artifact_link(
    link_id: &str,
    character_internal_id: &str,
    sheet_version_id: &str,
    artifact_kind: &str,
    artifact_ref: &str,
    manifest_ref: &str,
    source_ref: &str,
    label: &str,
    reuse_role: &str,
) -> CkcSheetArtifactLinkRecord {
    CkcSheetArtifactLinkRecord {
        link_id: link_id.to_owned(),
        character_internal_id: character_internal_id.to_owned(),
        character_ref: format!("atelier://character/{character_internal_id}"),
        sheet_version_id: sheet_version_id.to_owned(),
        sheet_version_ref: format!("atelier://sheet/{character_internal_id}/{sheet_version_id}"),
        typed_ref: format!("atelier://sheet-artifact/{link_id}"),
        artifact_kind: artifact_kind.to_owned(),
        artifact_ref: artifact_ref.to_owned(),
        manifest_ref: Some(manifest_ref.to_owned()),
        source_ref: Some(source_ref.to_owned()),
        label: Some(label.to_owned()),
        reuse_role: Some(reuse_role.to_owned()),
        linked_by: "seed".to_owned(),
        metadata: serde_json::json!({
            "seed": true,
            "sheet_version_id": sheet_version_id,
        }),
    }
}

fn seeded_ckc_media_album(
    collection_id: &str,
    name: &str,
    asset_id: &str,
    media_label: &str,
    source_path_ref: &str,
    source_url_ref: &str,
) -> CkcMediaAlbumRecord {
    CkcMediaAlbumRecord {
        collection_id: collection_id.to_owned(),
        collection_ref: format!("atelier://collection/{collection_id}"),
        name: name.to_owned(),
        description: "Seeded CKC linked-media album for Argus inspection and model workflows"
            .to_owned(),
        tags: vec!["reference".to_owned(), "training".to_owned()],
        member_count: 1,
        members_next_offset: None,
        created_by: "seed".to_owned(),
        updated_by: "seed".to_owned(),
        members: vec![CkcMediaMemberRecord {
            asset_id: asset_id.to_owned(),
            media_ref: format!("atelier://media/{asset_id}"),
            display_label: media_label.to_owned(),
            sort_order: 0,
            source_path_ref: Some(source_path_ref.to_owned()),
            source_url_ref: Some(source_url_ref.to_owned()),
            link_source_path_ref: Some(source_path_ref.to_owned()),
            link_source_url_ref: Some(source_url_ref.to_owned()),
            link_source_path_ref_status: "present".to_owned(),
            link_source_url_ref_status: "present".to_owned(),
            asset_source_path_ref_status: "none".to_owned(),
            asset_source_url_ref_status: "none".to_owned(),
            source_path_ref_origin: "link".to_owned(),
            source_url_ref_origin: "link".to_owned(),
            linked_by: "seed".to_owned(),
            member_updated_by: "seed".to_owned(),
            member_updated_at_utc: None,
            notes_updated_by: "seed".to_owned(),
            notes_updated_at_utc: None,
            notes: format!(
                "{media_label} image note stays separate from the character sheet notes"
            ),
            review_status: Some("approved".to_owned()),
            tags_buffer: "face, reference, approved".to_owned(),
        }],
    }
}

fn seeded_ckc_story_document(
    document_id: &str,
    title: &str,
    card_id: &str,
) -> CkcStoryDocumentRecord {
    let document_ref = format!("atelier://document/{document_id}");
    CkcStoryDocumentRecord {
        document_id: document_id.to_owned(),
        document_ref: document_ref.clone(),
        current_version_id: Some(format!("{document_id}-v1")),
        current_version_seq: 1,
        title: title.to_owned(),
        body_raw_text:
            "Story bible content stays separate from the character sheet and image notes."
                .to_owned(),
        tags: vec!["story".to_owned(), "continuity".to_owned()],
        cards: vec![CkcStoryCardRecord {
            card_id: card_id.to_owned(),
            card_ref: format!("atelier://story-card/{card_id}"),
            story_document_id: document_id.to_owned(),
            story_document_ref: document_ref.clone(),
            title: "Continuity card".to_owned(),
            body_raw_text:
                "Reusable character continuity card for ComfyUI, Unreal, Blender, and story work."
                    .to_owned(),
            tags: vec!["continuity".to_owned(), "reuse".to_owned()],
        }],
        beats: vec![CkcStoryBeatRecord {
            beat_id: format!("{card_id}-beat-001"),
            beat_ref: format!("atelier://story-beat/{card_id}-beat-001"),
            story_document_id: document_id.to_owned(),
            story_document_ref: document_ref,
            card_id: Some(card_id.to_owned()),
            beat_text: "Keep character intent, scene role, and asset reuse linked but editable."
                .to_owned(),
        }],
    }
}

fn seeded_ckc_moodboard_document(
    document_id: &str,
    snapshot_id: &str,
    title: &str,
) -> CkcMoodboardDocumentRecord {
    CkcMoodboardDocumentRecord {
        document_id: document_id.to_owned(),
        document_ref: format!("atelier://document/{document_id}"),
        current_version_id: Some(format!("{document_id}-v1")),
        current_version_seq: 1,
        title: title.to_owned(),
        body_raw_text: local_ckc_moodboard_snapshot_json(
            document_id,
            title,
            "Native Handshake moodboard linked to this character sheet for visual continuity.",
        ),
        tags: vec!["moodboard".to_owned(), "visual-reference".to_owned()],
        latest_snapshot_id: Some(snapshot_id.to_owned()),
        latest_snapshot_ref: Some(format!("atelier://moodboard/{snapshot_id}")),
        moodboard_name: title.to_owned(),
    }
}

fn pending_ckc_story_document(
    character_internal_id: &str,
    display_name: &str,
) -> CkcStoryDocumentRecord {
    let document_id = format!("pending-story-document-{character_internal_id}");
    CkcStoryDocumentRecord {
        document_id: document_id.clone(),
        document_ref: format!("atelier://document/{document_id}"),
        current_version_id: None,
        current_version_seq: 0,
        title: format!("{display_name} story"),
        body_raw_text: String::new(),
        tags: vec!["story".to_owned()],
        cards: Vec::new(),
        beats: Vec::new(),
    }
}

fn pending_ckc_moodboard_document(
    character_internal_id: &str,
    display_name: &str,
) -> CkcMoodboardDocumentRecord {
    let document_id = format!("pending-moodboard-document-{character_internal_id}");
    CkcMoodboardDocumentRecord {
        document_id: document_id.clone(),
        document_ref: format!("atelier://document/{document_id}"),
        current_version_id: None,
        current_version_seq: 0,
        title: format!("{display_name} moodboard"),
        body_raw_text: String::new(),
        tags: vec!["moodboard".to_owned()],
        latest_snapshot_id: None,
        latest_snapshot_ref: None,
        moodboard_name: format!("{display_name} moodboard"),
    }
}

fn local_ckc_moodboard_snapshot_json(moodboard_id: &str, name: &str, description: &str) -> String {
    let layer_id = Uuid::new_v4().to_string();
    let text_id = Uuid::new_v4().to_string();
    let history_id = Uuid::new_v4().to_string();
    serde_json::json!({
        "schema_id": "hsk.atelier.moodboard@1",
        "schema_version": 1,
        "moodboard_id": moodboard_id,
        "name": name,
        "description": description,
        "canvas": {
            "width": 1600.0,
            "height": 1000.0,
            "background_color": "#101418"
        },
        "layers": [{
            "layer_id": layer_id,
            "name": "CKC moodboard",
            "order": 1,
            "visible": true,
            "locked": false,
            "opacity": 1.0,
            "parent_layer_id": null
        }],
        "images": [],
        "text": [{
            "element_id": text_id,
            "layer_id": layer_id,
            "content": description,
            "font": "Inter",
            "font_size": 18.0,
            "color": "#f4f7fb",
            "position": { "x": 80.0, "y": 80.0 },
            "rotation": 0.0,
            "flags": {}
        }],
        "shapes": [],
        "connectors": [],
        "folders": [],
        "guides": [],
        "flags": {
            "locked": false,
            "archived": false,
            "operator_reviewed": false
        },
        "style": {
            "dominant_colors": ["#101418", "#f4f7fb"],
            "mood_keywords": ["ckc", "continuity"],
            "style_description": "Native CKC moodboard projection",
            "suggested_presets": []
        },
        "history": [{
            "history_id": history_id,
            "at": "2026-06-29T00:00:00Z",
            "actor": "handshake-native-atelier",
            "operation": "created",
            "summary": "Materialized local CKC moodboard snapshot"
        }]
    })
    .to_string()
}

fn is_pending_ckc_document_id(document_id: &str) -> bool {
    document_id.starts_with("pending-story-document-")
        || document_id.starts_with("pending-moodboard-document-")
}

fn slugify_public_id(label: &str, fallback_index: usize) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in label.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        format!("ckc-character-{fallback_index}")
    } else {
        out
    }
}

fn ckc_character_row_author_id(character_internal_id: &str) -> String {
    format!("atelier-ckc-character-{character_internal_id}")
}

pub fn ckc_media_album_row_author_id(collection_id: &str) -> String {
    format!(
        "atelier-ckc-album-{}",
        stable_author_id_suffix(collection_id)
    )
}

pub fn ckc_media_album_load_more_author_id(collection_id: &str) -> String {
    format!(
        "atelier-ckc-album-load-more-{}",
        stable_author_id_suffix(collection_id)
    )
}

pub fn ckc_media_album_page_state_author_id(collection_id: &str) -> String {
    format!(
        "atelier-ckc-album-page-state-{}",
        stable_author_id_suffix(collection_id)
    )
}

pub fn ckc_sheet_artifact_row_author_id(link_id: &str) -> String {
    format!(
        "atelier-ckc-sheet-artifact-{}",
        stable_author_id_suffix(link_id)
    )
}

pub fn ckc_sheet_artifact_detail_author_id(link_id: &str) -> String {
    format!(
        "atelier-ckc-sheet-artifact-detail-{}",
        stable_author_id_suffix(link_id)
    )
}

fn ckc_media_occurrence_key(collection_id: &str, asset_id: &str) -> String {
    format!("{collection_id}::{asset_id}")
}

pub fn ckc_media_row_author_id(collection_id: &str, asset_id: &str) -> String {
    format!(
        "atelier-ckc-media-{}-{}",
        stable_author_id_suffix(collection_id),
        stable_author_id_suffix(asset_id)
    )
}

pub fn ckc_folder_row_author_id(collection_id: &str, asset_id: &str, folder_ref: &str) -> String {
    format!(
        "atelier-ckc-folder-{}-{}-{}",
        stable_author_id_suffix(collection_id),
        stable_author_id_suffix(asset_id),
        stable_author_id_suffix(folder_ref)
    )
}

pub fn ckc_source_url_row_author_id(
    collection_id: &str,
    asset_id: &str,
    source_url_ref: &str,
) -> String {
    format!(
        "atelier-ckc-source-url-{}-{}-{}",
        stable_author_id_suffix(collection_id),
        stable_author_id_suffix(asset_id),
        stable_author_id_suffix(source_url_ref)
    )
}

pub fn ckc_search_result_row_author_id(target_ref: &str) -> String {
    format!(
        "atelier-ckc-search-result-{}",
        stable_author_id_suffix(target_ref)
    )
}

pub fn ckc_story_card_row_author_id(document_id: &str, card_id: &str) -> String {
    format!(
        "atelier-ckc-story-card-{}-{}",
        stable_author_id_suffix(document_id),
        stable_author_id_suffix(card_id)
    )
}

pub fn ckc_story_beat_row_author_id(document_id: &str, beat_id: &str) -> String {
    format!(
        "atelier-ckc-story-beat-{}-{}",
        stable_author_id_suffix(document_id),
        stable_author_id_suffix(beat_id)
    )
}

pub fn ckc_story_document_row_author_id(document_id: &str) -> String {
    format!(
        "atelier-ckc-story-document-{}",
        stable_author_id_suffix(document_id)
    )
}

pub fn ckc_moodboard_document_row_author_id(document_id: &str) -> String {
    format!(
        "atelier-ckc-moodboard-document-{}",
        stable_author_id_suffix(document_id)
    )
}

pub fn ckc_moodboard_snapshot_row_author_id(document_id: &str, snapshot_id: &str) -> String {
    format!(
        "atelier-ckc-moodboard-snapshot-{}-{}",
        stable_author_id_suffix(document_id),
        stable_author_id_suffix(snapshot_id)
    )
}

fn stable_author_id_suffix(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "ref".to_owned()
    } else {
        out
    }
}

fn ckc_tags_from_buffer(buffer: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in buffer
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        let normalized = tag.to_ascii_lowercase();
        if !tags.iter().any(|existing| existing == &normalized) {
            tags.push(normalized);
        }
    }
    tags
}

fn ckc_asset_ids_from_buffer(buffer: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for raw in buffer
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let id = raw
            .strip_prefix("atelier://media/")
            .unwrap_or(raw)
            .trim()
            .trim_matches('/');
        if !id.is_empty() && !ids.iter().any(|existing| existing == id) {
            ids.push(id.to_owned());
        }
    }
    ids
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn ckc_ref_has_forbidden_local_shape(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let normalized = lower.replace('\\', "/");
    let has_windows_drive = value.len() >= 2
        && value.as_bytes()[1] == b':'
        && value.as_bytes()[0].is_ascii_alphabetic();
    let has_embedded_windows_drive = normalized
        .as_bytes()
        .windows(3)
        .any(|window| window[0] == b'/' && window[1].is_ascii_alphabetic() && window[2] == b':');
    has_windows_drive
        || normalized.starts_with('/')
        || normalized.starts_with("~/")
        || normalized.starts_with("//")
        || normalized.starts_with("file:")
        || normalized.contains("file://")
        || normalized.contains(":///")
        || normalized.contains("%userprofile%")
        || has_embedded_windows_drive
        || normalized.contains(".gov")
        || normalized.contains("/../")
        || normalized.starts_with("../")
        || normalized.contains("://localhost")
        || normalized.contains("://127.")
        || normalized.contains("://0.0.0.0")
        || normalized.contains("://[::1]")
        || normalized.starts_with("localhost:")
        || normalized.starts_with("127.")
        || normalized.contains(".sqlite")
        || normalized.contains(".db")
        || normalized.contains("/ckc/")
        || normalized.contains("/castkit/")
        || normalized.contains("/electron/")
        || normalized.starts_with("ckc:")
        || normalized.starts_with("castkit:")
        || normalized.starts_with("electron:")
}

fn ckc_album_source_path_ref_from_input(value: &str) -> Result<Option<String>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed != value {
        return Err("source_path_ref must not be padded".to_owned());
    }
    let lower = trimmed.to_ascii_lowercase();
    if ckc_ref_has_forbidden_local_shape(trimmed) {
        return Err("source_path_ref must be a portable folder/source-path ref".to_owned());
    }
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("source-url://")
    {
        return Err(
            "source_path_ref must be a folder/source-path ref, not a source URL ref".to_owned(),
        );
    }
    if lower.starts_with("atelier://folder/")
        || lower.starts_with("source://")
        || lower.starts_with("dataset://")
        || lower.starts_with("artifact://")
        || lower.starts_with("manifest://")
    {
        return Ok(Some(trimmed.to_owned()));
    }
    Err("source_path_ref must use atelier://folder/, source://, dataset://, artifact://, or manifest://".to_owned())
}

fn ckc_album_source_url_ref_from_input(value: &str) -> Result<Option<String>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed != value {
        return Err("source_url_ref must not be padded".to_owned());
    }
    let lower = trimmed.to_ascii_lowercase();
    if ckc_ref_has_forbidden_local_shape(trimmed) {
        return Err("source_url_ref must be a portable source URL ref".to_owned());
    }
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("source-url://")
    {
        return Ok(Some(trimmed.to_owned()));
    }
    Err("source_url_ref must be an http(s) or source-url:// ref".to_owned())
}

fn ckc_album_source_refs_from_inputs(
    source_path_ref: &str,
    source_url_ref: &str,
) -> Result<(Option<String>, Option<String>), String> {
    Ok((
        ckc_album_source_path_ref_from_input(source_path_ref)?,
        ckc_album_source_url_ref_from_input(source_url_ref)?,
    ))
}

fn actor_id_or_default(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "local-atelier-panel".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn ckc_sheet_notes_source_key(character: &CkcCharacterRecord) -> String {
    let version = character
        .sheet_version_id
        .as_deref()
        .unwrap_or("draft-sheet-version");
    format!("{}:{version}", character.character_internal_id)
}

fn extract_ckc_sheet_notes(sheet_text: &str) -> String {
    sheet_text
        .lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix("notes:")
                .map(|notes| notes.trim().to_owned())
        })
        .unwrap_or_default()
}

fn upsert_ckc_sheet_notes(sheet_text: &mut String, notes: &str) {
    let normalized_notes = notes
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let mut found = false;
    let mut out = Vec::new();
    for line in sheet_text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("notes:") {
            let prefix_len = line.len() - trimmed.len();
            let prefix = &line[..prefix_len];
            out.push(format!("{prefix}notes: {normalized_notes}"));
            found = true;
        } else {
            out.push(line.to_owned());
        }
    }
    if !found {
        out.push(format!("notes: {normalized_notes}"));
    }
    *sheet_text = out.join("\n");
}

fn local_ckc_media_member(
    asset_id: &str,
    sort_order: i64,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    actor_id: &str,
) -> CkcMediaMemberRecord {
    let actor_id = actor_id_or_default(actor_id);
    CkcMediaMemberRecord {
        asset_id: asset_id.to_owned(),
        media_ref: format!("atelier://media/{asset_id}"),
        display_label: format!("linked-media-{asset_id}"),
        sort_order,
        source_path_ref: source_path_ref.clone(),
        source_url_ref: source_url_ref.clone(),
        link_source_path_ref: source_path_ref.clone(),
        link_source_url_ref: source_url_ref.clone(),
        link_source_path_ref_status: if source_path_ref.is_some() {
            "present"
        } else {
            "none"
        }
        .to_owned(),
        link_source_url_ref_status: if source_url_ref.is_some() {
            "present"
        } else {
            "none"
        }
        .to_owned(),
        asset_source_path_ref_status: "none".to_owned(),
        asset_source_url_ref_status: "none".to_owned(),
        source_path_ref_origin: if source_path_ref.is_some() {
            "link"
        } else {
            "none"
        }
        .to_owned(),
        source_url_ref_origin: if source_url_ref.is_some() {
            "link"
        } else {
            "none"
        }
        .to_owned(),
        linked_by: actor_id.clone(),
        member_updated_by: actor_id,
        member_updated_at_utc: None,
        notes_updated_by: "none".to_owned(),
        notes_updated_at_utc: None,
        notes: String::new(),
        review_status: Some("unreviewed".to_owned()),
        tags_buffer: String::new(),
    }
}

fn sort_ckc_album_members(album: &mut CkcMediaAlbumRecord) {
    album.members.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.asset_id.cmp(&right.asset_id))
    });
}

fn replace_ckc_media_members(
    album: &mut CkcMediaAlbumRecord,
    members: Vec<AtelierCkcMediaMemberRow>,
) {
    album.members = members
        .into_iter()
        .map(CkcMediaMemberRecord::from_backend)
        .collect();
    sort_ckc_album_members(album);
}

fn ckc_album_reorder_items_for_move(
    album: &CkcMediaAlbumRecord,
    member_idx: usize,
    delta: isize,
) -> Option<Vec<(String, i64)>> {
    if album.members_next_offset.is_some() || album.members.len() != album.member_count {
        return None;
    }
    let target_idx = member_idx.checked_add_signed(delta)?;
    if target_idx >= album.members.len() {
        return None;
    }
    let mut asset_ids = album
        .members
        .iter()
        .map(|member| member.asset_id.clone())
        .collect::<Vec<_>>();
    asset_ids.swap(member_idx, target_idx);
    Some(
        asset_ids
            .into_iter()
            .enumerate()
            .map(|(idx, asset_id)| (asset_id, idx as i64))
            .collect(),
    )
}

fn apply_ckc_album_dense_order(album: &mut CkcMediaAlbumRecord, items: &[(String, i64)]) {
    for (asset_id, sort_order) in items {
        if let Some(member) = album
            .members
            .iter_mut()
            .find(|member| &member.asset_id == asset_id)
        {
            member.sort_order = *sort_order;
        }
    }
    sort_ckc_album_members(album);
}

fn merge_ckc_media_members(
    album: &mut CkcMediaAlbumRecord,
    members: Vec<AtelierCkcMediaMemberRow>,
    selected_media_key: Option<&str>,
) -> usize {
    let mut added = 0usize;
    for row in members {
        let member = CkcMediaMemberRecord::from_backend(row);
        let member_key = ckc_media_occurrence_key(&album.collection_id, &member.asset_id);
        let selected_overlap = selected_media_key == Some(member_key.as_str());
        if let Some(existing) = album
            .members
            .iter_mut()
            .find(|existing| existing.asset_id == member.asset_id)
        {
            existing.media_ref = member.media_ref;
            existing.display_label = member.display_label;
            existing.sort_order = member.sort_order;
            existing.source_path_ref = member.source_path_ref;
            existing.source_url_ref = member.source_url_ref;
            existing.link_source_path_ref = member.link_source_path_ref;
            existing.link_source_url_ref = member.link_source_url_ref;
            existing.link_source_path_ref_status = member.link_source_path_ref_status;
            existing.link_source_url_ref_status = member.link_source_url_ref_status;
            existing.asset_source_path_ref_status = member.asset_source_path_ref_status;
            existing.asset_source_url_ref_status = member.asset_source_url_ref_status;
            existing.source_path_ref_origin = member.source_path_ref_origin;
            existing.source_url_ref_origin = member.source_url_ref_origin;
            existing.linked_by = member.linked_by;
            existing.member_updated_by = member.member_updated_by;
            existing.member_updated_at_utc = member.member_updated_at_utc;
            existing.notes_updated_by = member.notes_updated_by;
            existing.notes_updated_at_utc = member.notes_updated_at_utc;
            if !selected_overlap {
                if existing.notes.trim().is_empty() {
                    existing.notes = member.notes;
                }
                let existing_status = existing.review_status.as_deref().unwrap_or_default();
                if existing_status.is_empty() || existing_status == "unreviewed" {
                    existing.review_status = member.review_status;
                }
                if existing.tags_buffer.trim().is_empty() {
                    existing.tags_buffer = member.tags_buffer;
                }
            }
        } else {
            album.members.push(member);
            added += 1;
        }
    }
    sort_ckc_album_members(album);
    added
}

fn merge_ckc_media_album_list_page(
    character: &mut CkcCharacterRecord,
    page: AtelierCkcMediaAlbumListPageRow,
    selected_media_key: Option<&str>,
) -> usize {
    character.media_album_offset = character.media_album_offset.min(page.offset);
    character.media_album_limit = page.limit;
    character.media_album_member_limit = page.member_limit;
    character.media_albums_next_offset = page.albums_next_offset;

    let mut added = 0usize;
    for row in page.albums {
        let collection_id = row.collection_id.clone();
        if let Some(existing) = character
            .media_albums
            .iter_mut()
            .find(|album| album.collection_id == collection_id)
        {
            existing.collection_ref = row.collection_ref;
            existing.name = row.name;
            existing.description = row.description.unwrap_or_default();
            existing.tags = row.tags;
            existing.member_count = row.member_count;
            existing.members_next_offset = row.members_next_offset;
            existing.created_by = row.created_by;
            existing.updated_by = row.updated_by;
            merge_ckc_media_members(existing, row.members, selected_media_key);
        } else {
            character
                .media_albums
                .push(CkcMediaAlbumRecord::from_backend(row));
            added += 1;
        }
    }

    character.media_album_count = page.album_count.max(character.media_albums.len());
    added
}

fn effective_ckc_selected_media_key(state: &AtelierPanelState) -> Option<String> {
    let selected_index = state
        .ckc_selected_index
        .min(state.ckc_characters.len().saturating_sub(1));
    let character = state.ckc_characters.get(selected_index)?;
    character
        .selected_or_first_media_location(state.ckc_selected_media_key.as_deref())
        .map(|(album_idx, member_idx)| {
            let album = &character.media_albums[album_idx];
            let member = &album.members[member_idx];
            ckc_media_occurrence_key(&album.collection_id, &member.asset_id)
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IngestDecision {
    Pass,
    Reject,
    Unsure,
}

impl IngestDecision {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "Pass",
            Self::Reject => "Reject",
            Self::Unsure => "Unsure",
        }
    }

    fn machine_label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Reject => "reject",
            Self::Unsure => "unsure",
        }
    }

    fn backend_lane(self) -> &'static str {
        match self {
            Self::Pass => "accepted",
            Self::Reject => "rejected",
            Self::Unsure => "deferred",
        }
    }

    fn from_lane(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "accepted" | "accept" | "pass" => Self::Pass,
            "rejected" | "reject" => Self::Reject,
            _ => Self::Unsure,
        }
    }

    /// WP-CKC MT-042: parse the persisted `ingest.default-policy` token
    /// (pass/reject/unsure) into a decision, or `None` for an unknown token.
    fn from_policy_token(token: &str) -> Option<Self> {
        match token.trim().to_ascii_lowercase().as_str() {
            "pass" => Some(Self::Pass),
            "reject" => Some(Self::Reject),
            "unsure" => Some(Self::Unsure),
            _ => None,
        }
    }
}

fn ingest_bounded_number(value: &str, fallback: usize, min: usize, max: usize) -> usize {
    value
        .trim()
        .parse::<usize>()
        .ok()
        .map(|parsed| parsed.clamp(min, max))
        .unwrap_or(fallback)
}

fn ingest_contact_sheet_shape(state: &AtelierPanelState) -> (usize, usize, usize, usize) {
    let rows = ingest_bounded_number(&state.ingest_contact_rows, 3, 1, 24);
    let columns = ingest_bounded_number(&state.ingest_contact_columns, 4, 1, 24);
    let dpi = ingest_bounded_number(&state.ingest_contact_dpi, 300, 72, 1200);
    (rows, columns, dpi, rows.saturating_mul(columns))
}

fn ingest_queue_readout(state: &AtelierPanelState) -> String {
    let (rows, columns, dpi, cells) = ingest_contact_sheet_shape(state);
    format!(
        "dataset_ref={} character_ref={} actor={} decision={} link_passed={} tags={} note={} event={} date={} location={} contact_sheet={}x{}@{}dpi cells={} labels={} thumbnail_fit={} output_path={} facial_profile={}",
        state.ingest_dataset_ref.trim(),
        state.ingest_character_ref.trim(),
        state.ingest_actor.trim(),
        state.ingest_decision.machine_label(),
        state.ingest_link_passed,
        state.ingest_tag_buffer.trim(),
        state.ingest_batch_note.trim(),
        state.ingest_event.trim(),
        state.ingest_date.trim(),
        state.ingest_location.trim(),
        rows,
        columns,
        dpi,
        cells,
        state.ingest_contact_include_labels,
        state.ingest_contact_thumbnail_fit.trim(),
        state.ingest_contact_output_path.trim(),
        state.ingest_facial_profile.trim()
    )
}

fn ingest_tag_values(state: &AtelierPanelState) -> Vec<String> {
    state
        .ingest_tag_buffer
        .split([',', ';'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn ingest_optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn ingest_metadata_payload(
    state: &AtelierPanelState,
    request_id: &str,
    batch_id: Option<&str>,
    loaded_item_count: usize,
) -> serde_json::Value {
    let (rows, columns, dpi, cells) = ingest_contact_sheet_shape(state);
    serde_json::json!({
        "request_id": request_id,
        "batch_id": batch_id,
        "dataset_ref": ingest_optional_string(&state.ingest_dataset_ref),
        "character_ref": ingest_optional_string(&state.ingest_character_ref),
        "link_passed": state.ingest_link_passed,
        "tags": ingest_tag_values(state),
        "note": ingest_optional_string(&state.ingest_batch_note),
        "event": ingest_optional_string(&state.ingest_event),
        "date": ingest_optional_string(&state.ingest_date),
        "location": ingest_optional_string(&state.ingest_location),
        "facial_profile": ingest_optional_string(&state.ingest_facial_profile),
        "loaded_item_count": loaded_item_count,
        "contact_sheet": {
            "rows": rows,
            "columns": columns,
            "dpi": dpi,
            "cells": cells,
            "include_labels": state.ingest_contact_include_labels,
            "thumbnail_fit": ingest_optional_string(&state.ingest_contact_thumbnail_fit),
            "output_path": ingest_optional_string(&state.ingest_contact_output_path),
        },
    })
}

fn contact_sheet_items_from_ingest(items: &[AtelierItemRow]) -> Vec<AtelierContactSheetItem> {
    items
        .iter()
        .map(|item| AtelierContactSheetItem {
            item_id: item.item_id.clone(),
            label: item.file_name.clone(),
            source_ref: item.source_path.clone(),
            media_ref: None,
        })
        .collect()
}

fn contact_sheet_local_snapshot(
    batch_id: &str,
    items: &[AtelierItemRow],
    rows: usize,
    columns: usize,
    dpi: usize,
    include_labels: bool,
    thumbnail_fit: &str,
    output_path: Option<String>,
) -> ContactSheetExportSnapshot {
    let cell_count = rows.saturating_mul(columns);
    let rendered_item_count = items.len().min(cell_count);
    let omitted_item_count = items.len().saturating_sub(rendered_item_count);
    let item_basis = items
        .iter()
        .map(|item| format!("{}:{}", item.item_id, item.source_path))
        .collect::<Vec<_>>()
        .join("|");
    let source_lineage_preview = contact_sheet_lineage_preview(
        &items
            .iter()
            .map(|item| AtelierContactSheetItem {
                item_id: item.item_id.clone(),
                label: item.file_name.clone(),
                source_ref: item.source_path.clone(),
                media_ref: None,
            })
            .collect::<Vec<_>>(),
    );
    let content_hash = stable_posekit_hash(&format!(
        "hsk.atelier.contact_sheet_export@1|ingest_batch|{batch_id}|{rows}|{columns}|{dpi}|{item_basis}"
    ));
    ContactSheetExportSnapshot {
        source_kind: "ingest_batch".to_owned(),
        source_ref: batch_id.to_owned(),
        rows,
        columns,
        dpi,
        include_labels,
        thumbnail_fit: contact_sheet_thumbnail_fit_value(thumbnail_fit),
        output_path,
        item_count: items.len(),
        rendered_item_count,
        omitted_item_count,
        svg_artifact_ref: format!("preview://atelier/contact-sheet/{content_hash}/svg/payload"),
        svg_manifest_ref: format!("preview://atelier/contact-sheet/{content_hash}/svg/manifest"),
        receipt_ref: format!("preview://atelier/contact-sheet/{content_hash}/receipt"),
        receipt_manifest_ref: format!(
            "preview://atelier/contact-sheet/{content_hash}/receipt/manifest"
        ),
        svg_sha256: content_hash.clone(),
        receipt_sha256: content_hash.clone(),
        content_hash,
        source_lineage_preview,
    }
}

fn contact_sheet_snapshot_from_backend(
    row: AtelierContactSheetExportRow,
) -> ContactSheetExportSnapshot {
    let source_lineage_preview = contact_sheet_lineage_preview(&row.source_items);
    let rows = row
        .layout
        .get("rows")
        .and_then(|value| value.as_u64())
        .unwrap_or_default() as usize;
    let columns = row
        .layout
        .get("columns")
        .and_then(|value| value.as_u64())
        .unwrap_or_default() as usize;
    let dpi = row
        .layout
        .get("dpi")
        .and_then(|value| value.as_u64())
        .unwrap_or_default() as usize;
    ContactSheetExportSnapshot {
        source_kind: row.source_kind,
        source_ref: row.source_ref,
        rows,
        columns,
        dpi,
        include_labels: row.include_labels,
        thumbnail_fit: row.thumbnail_fit,
        output_path: row.output_path,
        item_count: row.item_count,
        rendered_item_count: row.rendered_item_count,
        omitted_item_count: row.omitted_item_count,
        svg_artifact_ref: row.svg_artifact.artifact_ref,
        svg_manifest_ref: row.svg_artifact.manifest_ref,
        receipt_ref: row.receipt_ref,
        receipt_manifest_ref: row.receipt_artifact.manifest_ref,
        svg_sha256: row.svg_sha256,
        receipt_sha256: row.receipt_sha256,
        content_hash: row.content_hash,
        source_lineage_preview,
    }
}

fn contact_sheet_thumbnail_fit_value(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "cover" => "cover".to_owned(),
        "stretch" => "stretch".to_owned(),
        _ => "contain".to_owned(),
    }
}

fn contact_sheet_lineage_preview(items: &[AtelierContactSheetItem]) -> String {
    const LINEAGE_PREVIEW_LIMIT: usize = 12;
    let mut preview = items
        .iter()
        .take(LINEAGE_PREVIEW_LIMIT)
        .map(|item| format!("{}={}", item.item_id, item.source_ref))
        .collect::<Vec<_>>()
        .join(",");
    if items.len() > LINEAGE_PREVIEW_LIMIT {
        preview.push_str(&format!(
            " truncated_source_item_count={}",
            items.len() - LINEAGE_PREVIEW_LIMIT
        ));
    }
    if preview.is_empty() {
        "<none>".to_owned()
    } else {
        preview
    }
}

fn contact_sheet_export_preview(snapshot: &ContactSheetExportSnapshot) -> String {
    let output_path = snapshot.output_path.as_deref().unwrap_or("<none>");
    format!(
        "schema=hsk.atelier.contact_sheet_export@1\nsource_kind={}\nsource_ref={}\nrows={}\ncolumns={}\ndpi={}\ninclude_labels={}\nthumbnail_fit={}\noutput_path={}\nitem_count={}\nrendered_item_count={}\nomitted_item_count={}\nsource_items={}\nsvg_artifact_ref={}\nsvg_manifest_ref={}\nreceipt_ref={}\nreceipt_manifest_ref={}\nsvg_sha256={}\nreceipt_sha256={}\ncontent_hash={}\nsvg_mime=image/svg+xml\nreceipt_mime=application/json",
        snapshot.source_kind,
        snapshot.source_ref,
        snapshot.rows,
        snapshot.columns,
        snapshot.dpi,
        snapshot.include_labels,
        snapshot.thumbnail_fit,
        output_path,
        snapshot.item_count,
        snapshot.rendered_item_count,
        snapshot.omitted_item_count,
        snapshot.source_lineage_preview,
        snapshot.svg_artifact_ref,
        snapshot.svg_manifest_ref,
        snapshot.receipt_ref,
        snapshot.receipt_manifest_ref,
        snapshot.svg_sha256,
        snapshot.receipt_sha256,
        snapshot.content_hash
    )
}

fn contact_sheet_visual_preview(snapshot: Option<&ContactSheetExportSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "contact_sheet_preview=<none>".to_owned();
    };
    let output_path = snapshot.output_path.as_deref().unwrap_or("<none>");
    format!(
        "contact_sheet_preview=grid rows={} columns={} dpi={} include_labels={} thumbnail_fit={} output_path={} rendered_item_count={} omitted_item_count={} source_items={} svg_ref={}",
        snapshot.rows,
        snapshot.columns,
        snapshot.dpi,
        snapshot.include_labels,
        snapshot.thumbnail_fit,
        output_path,
        snapshot.rendered_item_count,
        snapshot.omitted_item_count,
        snapshot.source_lineage_preview,
        snapshot.svg_artifact_ref
    )
}

fn facial_analysis_snapshot_from_backend(
    row: AtelierFacialIngestAnalysisRow,
) -> FacialIngestAnalysisSnapshot {
    FacialIngestAnalysisSnapshot {
        batch_id: row.batch_id,
        profile: row.profile,
        profile_tokens: row.profile_tokens.join("+"),
        item_count: row.item_count,
        decoded_count: json_summary_count(&row.summary, "decoded_count"),
        duplicate_group_count: json_summary_count(&row.summary, "duplicate_group_count"),
        duplicate_item_count: json_summary_count(&row.summary, "duplicate_item_count"),
        keep_count: json_summary_map_count(&row.summary, "review_recommendation_counts", "keep"),
        review_count: json_summary_map_count(
            &row.summary,
            "review_recommendation_counts",
            "review",
        ),
        cull_count: json_summary_map_count(&row.summary, "review_recommendation_counts", "cull"),
        quality_source: json_summary_string(&row.summary, "quality_source")
            .unwrap_or_else(|| "unknown".to_owned()),
        identity_source: json_summary_string(&row.summary, "identity_source")
            .unwrap_or_else(|| "unknown".to_owned()),
        dedupe_source: json_summary_string(&row.summary, "dedupe_source")
            .unwrap_or_else(|| "unknown".to_owned()),
        analysis_artifact_ref: row.analysis_artifact.artifact_ref,
        analysis_manifest_ref: row.analysis_artifact.manifest_ref,
        receipt_ref: row.receipt_ref,
        receipt_manifest_ref: row.receipt_artifact.manifest_ref,
        analysis_sha256: row.analysis_sha256,
        receipt_sha256: row.receipt_sha256,
        content_hash: row.content_hash,
    }
}

fn json_summary_count(summary: &serde_json::Value, field: &str) -> usize {
    summary
        .get(field)
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0)
}

fn json_summary_map_count(summary: &serde_json::Value, map_field: &str, key: &str) -> usize {
    summary
        .get(map_field)
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0)
}

fn json_summary_string(summary: &serde_json::Value, field: &str) -> Option<String> {
    summary
        .get(field)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

fn facial_review_session_id_from_command_result(result: &serde_json::Value) -> Option<String> {
    result
        .get("session")
        .and_then(|session| json_summary_string(session, "session_id"))
        .or_else(|| json_summary_string(result, "session_id"))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn facial_analysis_summary_preview(snapshot: Option<&FacialIngestAnalysisSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "facial_analysis=<none>".to_owned();
    };
    format!(
        "facial_analysis=schema=hsk.atelier.facial_ingest_analysis@1 batch_id={} profile={} profile_tokens={} item_count={} decoded_count={} duplicate_group_count={} duplicate_item_count={} keep={} review={} cull={} quality_source={} identity_source={} dedupe_source={} analysis_ref={}",
        snapshot.batch_id,
        snapshot.profile,
        snapshot.profile_tokens,
        snapshot.item_count,
        snapshot.decoded_count,
        snapshot.duplicate_group_count,
        snapshot.duplicate_item_count,
        snapshot.keep_count,
        snapshot.review_count,
        snapshot.cull_count,
        snapshot.quality_source,
        snapshot.identity_source,
        snapshot.dedupe_source,
        snapshot.analysis_artifact_ref
    )
}

fn facial_analysis_receipt_preview(snapshot: Option<&FacialIngestAnalysisSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "facial_analysis_receipt=<none>".to_owned();
    };
    format!(
        "schema=hsk.atelier.facial_ingest_analysis@1\nbatch_id={}\nprofile={}\nprofile_tokens={}\nitem_count={}\ndecoded_count={}\nduplicate_group_count={}\nduplicate_item_count={}\nkeep={}\nreview={}\ncull={}\nquality_source={}\nidentity_source={}\ndedupe_source={}\nanalysis_artifact_ref={}\nanalysis_manifest_ref={}\nreceipt_ref={}\nreceipt_manifest_ref={}\nanalysis_sha256={}\nreceipt_sha256={}\ncontent_hash={}\nanalysis_mime=application/json\nreceipt_mime=application/json",
        snapshot.batch_id,
        snapshot.profile,
        snapshot.profile_tokens,
        snapshot.item_count,
        snapshot.decoded_count,
        snapshot.duplicate_group_count,
        snapshot.duplicate_item_count,
        snapshot.keep_count,
        snapshot.review_count,
        snapshot.cull_count,
        snapshot.quality_source,
        snapshot.identity_source,
        snapshot.dedupe_source,
        snapshot.analysis_artifact_ref,
        snapshot.analysis_manifest_ref,
        snapshot.receipt_ref,
        snapshot.receipt_manifest_ref,
        snapshot.analysis_sha256,
        snapshot.receipt_sha256,
        snapshot.content_hash
    )
}

/// MT-030: map a native Facial `identity_source` token to a clear real-vs-unavailable disposition so a
/// no-context model can tell whether identity is model-backed or degraded WITHOUT decoding the raw
/// token. Tokens are the verbatim values the MT-026/MT-027 backend emits (facial_native/identity.rs):
/// `real` (ArcFace ONNX loaded), `handshake_proxy_no_model` (no model configured — the deliberate
/// no-model DEGRADED label, paired with `identity_verdict=proxy_unverified`),
/// `handshake_identity_model_unavailable` (model configured but not loaded), and the summary rollup
/// `mixed_identity_sources`. Quality/dedupe are NOT proxies — they are native-real metadata/content-hash
/// paths (`facet_native_metadata_only_v1` / `imagededup_native_content_hash_exact_v1`).
fn facial_identity_disposition(identity_source: &str) -> &'static str {
    match identity_source {
        "real" => "real:model_backed",
        "handshake_proxy_no_model" => "unavailable:no_model_proxy",
        "handshake_identity_model_unavailable" => "unavailable:model_configured_not_loaded",
        "mixed_identity_sources" => "mixed:rows_disagree",
        "" | "unknown" => "unknown:no_analysis",
        _ => "unknown:unrecognized_source",
    }
}

/// MT-030 provenance readout: surfaces the exact `quality_source` / `dedupe_source` / `identity_source`
/// tokens from the last native Facial analysis plus an honest identity disposition, so a model can see
/// whether identity is real (model-backed) or unavailable/degraded without opening the artifact.
fn facial_provenance_preview(snapshot: Option<&FacialIngestAnalysisSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "facial_provenance=<none> identity=unknown:no_analysis \
                run Analyze Facial in Dataset intake to populate quality_source/dedupe_source/identity_source."
            .to_owned();
    };
    let verdict_hint = match snapshot.identity_source.as_str() {
        "handshake_proxy_no_model" => "proxy_unverified",
        "handshake_identity_model_unavailable" => "model_unavailable",
        "real" => "unsure_without_reference",
        _ => "n/a",
    };
    format!(
        "facial_provenance quality_source={} dedupe_source={} identity_source={} identity={} identity_verdict_hint={} note=quality+dedupe_are_native_real;identity_model_backed_only_when_identity_source=real",
        snapshot.quality_source,
        snapshot.dedupe_source,
        snapshot.identity_source,
        facial_identity_disposition(&snapshot.identity_source),
        verdict_hint
    )
}

/// MT-030 capability/health readout for `GET /atelier/facial/features`: feature count, the distinct
/// capability families, and the enumerated command routes a swarm agent can drive.
fn facial_features_preview(row: &AtelierFacialFeatureListRow) -> String {
    let capabilities = row
        .features
        .as_array()
        .map(|features| {
            let mut caps: Vec<String> = features
                .iter()
                .filter_map(|feature| feature.get("capability").and_then(|value| value.as_str()))
                .map(ToOwned::to_owned)
                .collect();
            caps.sort();
            caps.dedup();
            caps.join(",")
        })
        .unwrap_or_default();
    let commands = row
        .command_routes
        .iter()
        .map(|route| route.command.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "facial_features schema={} registry_schema={} feature_count={} capabilities={} command_route_count={} commands={}",
        row.schema_id,
        row.registry_schema_id,
        row.feature_count,
        capabilities,
        row.command_routes.len(),
        commands
    )
}

/// MT-030/MT-055 last-command receipt readout shared by every review command. `status` is FREE TEXT
/// (there is no `CommandStatus` type): succeeded/degraded/blocked/error post-context outcomes all arrive
/// as parser-visible Ok envelopes with durable receipts; transport/parse failures stay on the Err path.
fn facial_command_receipt_preview(row: &AtelierFacialCommandResponseRow) -> String {
    let result_artifact_ref = row
        .result_artifact
        .as_ref()
        .map(|artifact| artifact.artifact_ref.as_str())
        .unwrap_or("<none>");
    let receipt_ref = row.receipt_ref.as_deref().unwrap_or("<none>");
    let degraded = if row.degraded_reasons.is_empty() {
        "none".to_owned()
    } else {
        row.degraded_reasons.join("|")
    };
    format!(
        "facial_command command={} status={} actor={} receipt_ref={} result_artifact_ref={} degraded_reasons={} recovery_hint={} error={}",
        row.command,
        row.status,
        row.actor,
        receipt_ref,
        result_artifact_ref,
        degraded,
        row.recovery_hint.as_deref().unwrap_or("none"),
        row.error.as_deref().unwrap_or("none")
    )
}

/// MT-030/MT-055 review-queue status readout. A succeeded/degraded `status` command exposes queue
/// counts from `FacialReviewStatusArtifact`; a blocked/error envelope has no result and must render
/// that honestly.
fn facial_review_status_preview(row: &AtelierFacialCommandResponseRow) -> String {
    let result = &row.result;
    if !result.is_object() {
        return format!(
            "facial_review_status status={} result_available=false error={} recovery_hint={} receipt_ref={}",
            row.status,
            row.error.as_deref().unwrap_or("none"),
            row.recovery_hint.as_deref().unwrap_or("none"),
            row.receipt_ref.as_deref().unwrap_or("<none>")
        );
    }
    let count = |key: &str| json_summary_count(result, key);
    format!(
        "facial_review_status status={} result_available=true session_id={} item_count={} decided={} accepted={} rejected={} hold={} undecided={} active_claims={} expired_claims={}",
        row.status,
        json_summary_string(result, "session_id").unwrap_or_else(|| "<unknown>".to_owned()),
        count("item_count"),
        count("decided_count"),
        count("accepted_count"),
        count("rejected_count"),
        count("hold_count"),
        count("undecided_count"),
        count("active_claim_count"),
        count("expired_claim_count")
    )
}

fn facial_result_artifact_ref(row: &AtelierFacialCommandResponseRow) -> &str {
    row.result_artifact
        .as_ref()
        .map(|artifact| artifact.artifact_ref.as_str())
        .unwrap_or("<none>")
}

fn json_summary_bool(summary: &serde_json::Value, field: &str) -> Option<bool> {
    summary.get(field).and_then(|value| value.as_bool())
}

fn facial_review_montage_content_preview(row: &AtelierFacialCommandResponseRow) -> String {
    if !row.result.is_object() {
        return format!(
            "facial_montage_content status={} result_available=false result_artifact_ref={} manifest_ref=<none>",
            row.status,
            facial_result_artifact_ref(row)
        );
    }
    let tile_pairs = row
        .result
        .get("tiles")
        .and_then(|value| value.as_array())
        .map(|tiles| {
            tiles
                .iter()
                .take(6)
                .filter_map(|tile| {
                    let item_id = json_summary_string(tile, "item_id")?;
                    let tile_ref = json_summary_string(tile, "tile_ref")?;
                    Some(format!("{item_id}={tile_ref}"))
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "<none>".to_owned());
    format!(
        "facial_montage_content status={} session_id={} tile_map_ref={} manifest_ref={} result_artifact_ref={} tile_count={} page={} columns={} rows={} tiles={}",
        row.status,
        json_summary_string(&row.result, "session_id").unwrap_or_else(|| "<unknown>".to_owned()),
        json_summary_string(&row.result, "tile_map_ref").unwrap_or_else(|| "<none>".to_owned()),
        json_summary_string(&row.result, "manifest_ref").unwrap_or_else(|| "<none>".to_owned()),
        facial_result_artifact_ref(row),
        json_summary_count(&row.result, "tile_count"),
        json_summary_count(&row.result, "page"),
        json_summary_count(&row.result, "columns"),
        json_summary_count(&row.result, "rows"),
        tile_pairs
    )
}

fn facial_review_export_content_preview(row: &AtelierFacialCommandResponseRow) -> String {
    if !row.result.is_object() {
        return format!(
            "facial_export_content status={} result_available=false result_artifact_ref={} manifest_ref=<none>",
            row.status,
            facial_result_artifact_ref(row)
        );
    }
    let entry_pairs = row
        .result
        .get("entries")
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .take(6)
                .filter_map(|entry| {
                    let item_id = json_summary_string(entry, "item_id")?;
                    let output_ref = json_summary_string(entry, "output_ref")?;
                    Some(format!("{item_id}={output_ref}"))
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "<none>".to_owned());
    let source_mutation = json_summary_bool(&row.result, "source_mutation")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<unknown>".to_owned());
    format!(
        "facial_export_content status={} session_id={} dataset={} copy_mode={} source_mutation={} accepted={} skipped={} manifest_ref={} result_artifact_ref={} entries={}",
        row.status,
        json_summary_string(&row.result, "session_id").unwrap_or_else(|| "<unknown>".to_owned()),
        json_summary_string(&row.result, "dataset_name").unwrap_or_else(|| "<none>".to_owned()),
        json_summary_string(&row.result, "copy_mode").unwrap_or_else(|| "<unknown>".to_owned()),
        source_mutation,
        json_summary_count(&row.result, "accepted_count"),
        json_summary_count(&row.result, "skipped_count"),
        json_summary_string(&row.result, "manifest_ref").unwrap_or_else(|| "<none>".to_owned()),
        facial_result_artifact_ref(row),
        entry_pairs
    )
}

fn facial_review_numeric_parse_notes(notes: &[String]) -> String {
    if notes.is_empty() {
        "numeric_parse_notes none".to_owned()
    } else {
        format!("numeric_parse_notes {}", notes.join("; "))
    }
}

fn append_facial_parse_notes(base: String, parse_notes: &str) -> String {
    let parse_notes = parse_notes.trim();
    if parse_notes.is_empty() || parse_notes == "numeric_parse_notes none" {
        base
    } else {
        format!("{base} {parse_notes}")
    }
}

fn parse_facial_usize_with_note(
    raw: &str,
    field: &str,
    default: usize,
    notes: &mut Vec<String>,
) -> usize {
    let value = raw.trim();
    if value.is_empty() {
        return default;
    }
    value.parse::<usize>().unwrap_or_else(|_| {
        notes.push(format!("{field} invalid='{value}' coerced={default}"));
        default
    })
}

fn parse_facial_u32_with_note(
    raw: &str,
    field: &str,
    default: u32,
    notes: &mut Vec<String>,
) -> u32 {
    let value = raw.trim();
    if value.is_empty() {
        return default;
    }
    value.parse::<u32>().unwrap_or_else(|_| {
        notes.push(format!("{field} invalid='{value}' coerced={default}"));
        default
    })
}

fn parse_facial_optional_usize_with_note(
    raw: &str,
    field: &str,
    notes: &mut Vec<String>,
) -> Option<usize> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    match value.parse::<usize>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            notes.push(format!("{field} invalid='{value}' coerced=omitted"));
            None
        }
    }
}

fn parse_facial_optional_u64_with_note(
    raw: &str,
    field: &str,
    notes: &mut Vec<String>,
) -> Option<u64> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    match value.parse::<u64>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            notes.push(format!("{field} invalid='{value}' coerced=omitted"));
            None
        }
    }
}

/// MT-030 accumulated review lineage refs readout (session + claim + decision refs) so a model can chain
/// claim/status/montage/export without losing the artifact refs the backend requires.
fn facial_review_refs_preview(state: &AtelierPanelState) -> String {
    let session = state.facial_review_session_ref.trim();
    let session_id = state.facial_review_session_id.trim();
    let claim = state.facial_review_claim_ref.trim();
    format!(
        "facial_review_refs session_ref={} session_id={} claim_ref={} accumulated_claim_refs={} accumulated_decision_refs={}",
        if session.is_empty() {
            "<none>"
        } else {
            session
        },
        if session_id.is_empty() {
            "<none>"
        } else {
            session_id
        },
        if claim.is_empty() { "<none>" } else { claim },
        state.facial_review_claim_refs.len(),
        state.facial_review_decision_refs.len()
    )
}

fn ingest_receipt_applied_item_ids(
    rows: &[crate::backend_client::AtelierIntakeClassificationRow],
    applied_count: usize,
) -> String {
    const RECEIPT_ITEM_ID_DISPLAY_LIMIT: usize = 12;
    if rows.is_empty() && applied_count == 0 {
        return "<none>".to_owned();
    }
    let displayed_count = rows.len().min(RECEIPT_ITEM_ID_DISPLAY_LIMIT);
    let mut display = rows
        .iter()
        .take(RECEIPT_ITEM_ID_DISPLAY_LIMIT)
        .map(|row| row.item.item_id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    if display.is_empty() {
        display = "<preview-empty>".to_owned();
    }
    if applied_count > displayed_count {
        let truncated_count = applied_count - displayed_count;
        display.push_str(&format!(" truncated_count={truncated_count}"));
    }
    display
}

fn ingest_item_decision(state: &AtelierPanelState, item: &AtelierItemRow) -> IngestDecision {
    state
        .ingest_item_decisions
        .get(&item.item_id)
        .copied()
        .unwrap_or_else(|| IngestDecision::from_lane(&item.lane))
}

pub struct AtelierPanel {
    state: Mutex<AtelierPanelState>,
    side_panel: Arc<Mutex<AtelierSidePanel>>,
    /// WP-CKC MT-012 (FIX 2): the moodboard's OWN isolated canvas board. The app-wide shared Loom
    /// singleton (created in `app.rs::install_secondary_mounts` and driven by the Loom canvas pane) is
    /// deliberately NOT held here anymore: the moodboard used to render + `set_board` that shared board
    /// and push its `CanvasEvent`s into the shared `canvas_events` queue, so its visual edits were routed
    /// to the workspace `default-canvas` block (and dropped) instead of the `atelier_moodboard` snapshot,
    /// and it clobbered the Loom pane. This board's ONLY persistence path is the snapshot
    /// reverse-projection ([`board_to_ckc_moodboard_json`]) performed by "Save moodboard". De-singletoning
    /// the whole app is out of scope (MT-022); this only stops the moodboard writing to the wrong store.
    moodboard_board: Arc<Mutex<LoomCanvasBoard>>,
    ckc_client: Option<AtelierClient>,
    ckc_cell: AtelierCkcCell,
    ckc_create_cell: AtelierCkcCreateCell,
    ckc_append_cell: AtelierCkcAppendCell,
    ckc_template_cell: AtelierCkcTemplateCell,
    ckc_safe_subset_cell: AtelierCkcSafeSubsetCell,
    ckc_import_cell: AtelierCkcImportCell,
    ckc_export_cell: AtelierCkcExportCell,
    ckc_field_suggestions_cell: AtelierCkcFieldSuggestionsCell,
    ckc_sheet_artifact_links_cell: AtelierCkcSheetArtifactLinksCell,
    ckc_media_album_create_cell: AtelierCkcMediaAlbumCreateCell,
    ckc_media_album_list_page_cell: AtelierCkcMediaAlbumListPageCell,
    ckc_media_album_items_cell: AtelierCkcMediaAlbumItemsCell,
    ckc_media_album_page_cell: AtelierCkcMediaAlbumItemsCell,
    ckc_media_notes_cell: AtelierCkcMediaNotesCell,
    /// MT-010 queued delivery cell for fetched CKC linked-media image byte payloads (drained each
    /// frame and fed to [`AtelierPanel::set_ckc_media_image_bytes`]). Reuses the MT-043
    /// `AtelierPoseSourceBytesCell` shape `(request_id, Result<bytes, err>)`.
    ckc_media_bytes_cell: AtelierPoseSourceBytesCell,
    /// MT-010 per-asset decode cache for the CKC linked-media viewer (mirrors `pose_source_image`).
    /// Interior-mutable so the `&self` render path can (re)upload a texture when the bytes change.
    ckc_media_image: Mutex<CkcMediaImageCache>,
    ckc_character_document_cell: AtelierCkcCharacterDocumentCell,
    ckc_story_card_cell: AtelierCkcStoryCardCell,
    ckc_story_beat_cell: AtelierCkcStoryBeatCell,
    ckc_moodboard_latest_cell: AtelierCkcMoodboardSnapshotCell,
    ckc_search_cell: AtelierCkcSearchCell,
    ckc_tag_note_cell: AtelierCkcTagNoteCell,
    pose_export_cell: AtelierPosekitExportCell,
    /// MT-050 queued delivery cell for fetched exported OpenPose PNG ArtifactStore bytes (right
    /// viewport). Uses the same `(request_id, Result<bytes, err>)` shape as source-image fetches.
    pose_export_png_bytes_cell: AtelierPoseSourceBytesCell,
    /// MT-043 queued delivery cell for the fetched Posekit source-image bytes (drained each frame and
    /// fed to [`AtelierPanel::set_pose_source_image_bytes`]).
    pose_source_bytes_cell: AtelierPoseSourceBytesCell,
    /// Decode cache for the Posekit source image (left viewport). Interior-mutable so the `&self`
    /// render path can (re)upload the texture when the bytes change.
    pose_source_image: Mutex<PoseSourceImageCache>,
    /// Decode cache for the literal exported OpenPose PNG artifact (right viewport).
    pose_openpose_image: Mutex<PoseSourceImageCache>,
    ingest_contact_export_cell: AtelierContactSheetExportCell,
    ingest_facial_analysis_cell: AtelierFacialIngestAnalysisCell,
    // MT-030 Facial Review sub-mode delivery cells (drained each frame, HBR-QUIET).
    ingest_facial_features_cell: AtelierFacialFeatureListCell,
    ingest_facial_command_cell: AtelierFacialCommandCell,
    ingest_classification_cell: AtelierIntakeClassificationCell,
    /// WP-CKC MT-042 delivery cells: load/reload effective defaults, save-batch
    /// outcome (with the failing key on error), and single reset-to-default result.
    /// Drained each frame (HBR-QUIET).
    settings_cell: AtelierPreferencesCell,
    settings_save_cell: AtelierPreferenceSaveCell,
    settings_reset_cell: AtelierPreferenceMutationCell,
    model_ops_cell: AtelierModelOperationCell,
    prompt_feedback_import_cell: AtelierPromptFeedbackImportCell,
    prompt_feedback_rewrite_cell: AtelierPromptFeedbackRewriteCell,
    prompt_feedback_export_cell: AtelierPromptFeedbackExportCell,
}

impl AtelierPanel {
    pub fn new(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
    ) -> Self {
        Self::with_client(side_panel, canvas_board, canvas_events, None)
    }

    pub fn with_client(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        // WP-CKC MT-012 (FIX 2): the shared Loom singleton + its event queue are still accepted for
        // constructor-API stability (app.rs + the Loom pane still own/drive them), but the moodboard no
        // longer touches them — it owns `moodboard_board` below. Bound to `_` so they are provably unused.
        _canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        _canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
        ckc_client: Option<AtelierClient>,
    ) -> Self {
        let mut state = AtelierPanelState::default();
        if let Some(client) = ckc_client.as_ref() {
            state.ingest_actor = client.actor_id().to_owned();
            state.ckc_media_actor_id = client.actor_id().to_owned();
            state.ckc_characters.clear();
            state.ckc_search_results.clear();
            state.ckc_search_status = "Waiting for live CKC database load".to_owned();
            state.ckc_tag_note_scope_ref.clear();
        }
        let mut moodboard_board =
            LoomCanvasBoard::new("atelier-ckc-moodboard", "ckc-moodboard-canvas");
        moodboard_board.set_mutation_policy(CanvasMutationPolicy::SnapshotProjection);
        Self {
            state: Mutex::new(state),
            side_panel,
            // WP-CKC MT-012 (FIX 2): a fresh board dedicated to the moodboard. The ids are moodboard-local
            // (its cards reference `moodboard-*` element ids, not real Loom blocks), so it never addresses
            // or persists into the shared Loom workspace/canvas block.
            moodboard_board: Arc::new(Mutex::new(moodboard_board)),
            ckc_client,
            ckc_cell: Arc::new(Mutex::new(None)),
            ckc_create_cell: Arc::new(Mutex::new(None)),
            ckc_append_cell: Arc::new(Mutex::new(None)),
            ckc_template_cell: Arc::new(Mutex::new(None)),
            ckc_safe_subset_cell: Arc::new(Mutex::new(None)),
            ckc_import_cell: Arc::new(Mutex::new(None)),
            ckc_export_cell: Arc::new(Mutex::new(None)),
            ckc_field_suggestions_cell: Arc::new(Mutex::new(None)),
            ckc_sheet_artifact_links_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_create_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_list_page_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_items_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_page_cell: Arc::new(Mutex::new(None)),
            ckc_media_notes_cell: Arc::new(Mutex::new(None)),
            ckc_media_bytes_cell: Arc::new(Mutex::new(VecDeque::new())),
            ckc_media_image: Mutex::new(CkcMediaImageCache::default()),
            ckc_character_document_cell: Arc::new(Mutex::new(None)),
            ckc_story_card_cell: Arc::new(Mutex::new(None)),
            ckc_story_beat_cell: Arc::new(Mutex::new(None)),
            ckc_moodboard_latest_cell: Arc::new(Mutex::new(None)),
            ckc_search_cell: Arc::new(Mutex::new(None)),
            ckc_tag_note_cell: Arc::new(Mutex::new(None)),
            pose_export_cell: Arc::new(Mutex::new(None)),
            pose_export_png_bytes_cell: Arc::new(Mutex::new(VecDeque::new())),
            pose_source_bytes_cell: Arc::new(Mutex::new(VecDeque::new())),
            pose_source_image: Mutex::new(PoseSourceImageCache::default()),
            pose_openpose_image: Mutex::new(PoseSourceImageCache::default()),
            ingest_contact_export_cell: Arc::new(Mutex::new(None)),
            ingest_facial_analysis_cell: Arc::new(Mutex::new(None)),
            ingest_facial_features_cell: Arc::new(Mutex::new(None)),
            ingest_facial_command_cell: Arc::new(Mutex::new(None)),
            ingest_classification_cell: Arc::new(Mutex::new(None)),
            settings_cell: Arc::new(Mutex::new(None)),
            settings_save_cell: Arc::new(Mutex::new(None)),
            settings_reset_cell: Arc::new(Mutex::new(None)),
            model_ops_cell: Arc::new(Mutex::new(None)),
            prompt_feedback_import_cell: Arc::new(Mutex::new(None)),
            prompt_feedback_rewrite_cell: Arc::new(Mutex::new(None)),
            prompt_feedback_export_cell: Arc::new(Mutex::new(None)),
        }
    }

    pub fn active_tab(&self) -> AtelierPanelTab {
        self.state
            .lock()
            .map(|state| state.active_tab)
            .unwrap_or(AtelierPanelTab::CastkitCodex)
    }

    pub fn set_active_tab(&self, tab: AtelierPanelTab) {
        if let Ok(mut state) = self.state.lock() {
            state.active_tab = tab;
        }
    }

    pub fn set_model_ops_active_lease_thread_id(&self, thread_id: impl Into<String>) {
        if let Ok(mut state) = self.state.lock() {
            state.model_ops_active_lease_thread_id = thread_id.into();
        }
    }

    pub fn show(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let panel_id = egui::Id::new(ATELIER_PANEL_AUTHOR_ID);
        let response = ui
            .scope_builder(egui::UiBuilder::new().id_salt(panel_id), |ui| {
                self.show_inner(ui, palette);
            })
            .response;
        emit_node(
            ui.ctx(),
            response.id,
            accesskit::Role::Group,
            ATELIER_PANEL_AUTHOR_ID,
            "Atelier",
            false,
        );
    }

    fn show_inner(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        // WP-CKC MT-042: load-on-open + drain effective defaults before rendering, so a
        // freshly-loaded default is applied to the runtime fields the same frame. This runs
        // regardless of the active tab because the defaults are panel-wide.
        self.ensure_settings_load_requested();
        self.drain_settings_backend();
        self.drain_model_ops_backend();
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("Atelier").color(palette.text));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("CKC").color(palette.text_subtle));
            });
            ui.add_space(4.0);
            self.show_settings_region(ui, palette);
            self.show_model_ops_region(ui, palette);
            ui.add_space(4.0);
            self.show_tab_strip(ui);
            ui.separator();

            let active = self.active_tab();
            self.show_content_region(ui, palette, active);
        });
    }

    fn show_tab_strip(&self, ui: &mut egui::Ui) {
        let response = ui
            .horizontal(|ui| {
                let mut active = self.active_tab();
                for tab in AtelierPanelTab::ALL {
                    let selected = active == tab;
                    let button = ui.add(egui::Button::selectable(selected, tab.label()));
                    button.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            selected,
                            tab.label(),
                        )
                    });
                    emit_node(
                        ui.ctx(),
                        button.id,
                        accesskit::Role::Tab,
                        tab.tab_author_id(),
                        tab.label(),
                        selected,
                    );
                    if button.clicked() {
                        active = tab;
                    }
                }
                self.set_active_tab(active);
            })
            .response;
        emit_node(
            ui.ctx(),
            response.id,
            accesskit::Role::TabList,
            ATELIER_TABLIST_AUTHOR_ID,
            "Atelier tabs",
            false,
        );
    }

    fn show_content_region(&self, ui: &mut egui::Ui, palette: &HsPalette, tab: AtelierPanelTab) {
        let response = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(tab.content_author_id()),
                |ui| match tab {
                    AtelierPanelTab::CastkitCodex => self.show_ckc(ui, palette),
                    AtelierPanelTab::Posekit => self.show_posekit(ui, palette),
                    AtelierPanelTab::Ingest => self.show_ingest(ui, palette),
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            response.id,
            accesskit::Role::Group,
            tab.content_author_id(),
            tab.label(),
            false,
        );
    }

    /// WP-CKC MT-042: fire the one-shot effective-defaults load on first panel show
    /// (HBR-QUIET, off-thread). No-op without a live client (seeded/demo mode).
    fn ensure_settings_load_requested(&self) {
        let Some(client) = self.ckc_client.as_ref() else {
            return;
        };
        let should_request = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.settings_load_requested {
                false
            } else {
                state.settings_load_requested = true;
                state.settings_loading = true;
                state.settings_status = "Loading Atelier defaults from PostgreSQL...".to_owned();
                true
            }
        };
        if should_request {
            client.fetch_preferences(self.settings_cell.clone());
        }
    }

    /// WP-CKC MT-042: drain the load/save/reset delivery cells and apply results.
    /// - Load: sync edit buffers + non-nav runtime defaults; seed live nav ONLY on the
    ///   first load (F2). - Save: report which key failed (F1) and ALWAYS reload so the
    ///   UI re-syncs to the DB (whether some, all, or no keys persisted). - Reset:
    ///   reload so buffers + runtime re-derive from the store.
    fn drain_settings_backend(&self) {
        let load_result = self
            .settings_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = load_result {
            if let Ok(mut state) = self.state.lock() {
                state.settings_loading = false;
                match result {
                    Ok(rows) => {
                        let first_load = !state.settings_loaded;
                        if first_load {
                            // Seed live nav from the store BEFORE syncing buffers, using
                            // the pre-sync active_tab/book_mode to decide (F2 guard).
                            seed_settings_live_nav(&mut state, &rows);
                        }
                        let managed = settings_managed_key_count(&rows);
                        sync_settings_buffers(&mut state, rows);
                        state.settings_loaded = true;
                        state.settings_status =
                            format!("Atelier defaults loaded ({managed} keys).");
                    }
                    Err(err) => {
                        state.settings_status = format!("Atelier defaults load failed: {err}");
                    }
                }
            }
        }

        let mut reload = false;
        let save_outcome = self
            .settings_save_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(outcome) = save_outcome {
            if let Ok(mut state) = self.state.lock() {
                state.settings_save_pending = false;
                match (outcome.failed_key, outcome.error) {
                    (Some(failed_key), error) => {
                        // F1: report the exact failing key; a reload re-syncs so the UI
                        // never diverges from the DB (some keys before it may have saved).
                        state.settings_status = format!(
                            "Save failed at '{failed_key}': {}. Reloaded {} saved key(s) to re-sync.",
                            error.unwrap_or_else(|| "unknown error".to_owned()),
                            outcome.saved.len()
                        );
                    }
                    (None, _) => {
                        state.settings_status = format!(
                            "Saved {} default(s) to PostgreSQL + EventLedger.",
                            outcome.saved.len()
                        );
                    }
                }
                reload = self.ckc_client.is_some();
            }
        }

        let reset_result = self
            .settings_reset_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = reset_result {
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        state.settings_status =
                            format!("Reset {} to default ({}).", row.key, row.value);
                        reload = reload || self.ckc_client.is_some();
                    }
                    Err(err) => {
                        state.settings_status = format!("Atelier default reset failed: {err}");
                    }
                }
            }
        }
        if reload {
            if let Some(client) = self.ckc_client.as_ref() {
                client.fetch_preferences(self.settings_cell.clone());
            }
        }
    }

    fn settings_model_ops_context_for_key(
        &self,
        key: &str,
        fallback_actor_id: &str,
    ) -> Result<ModelOpsMutationContext, String> {
        let expected_thread_id = preference_model_ops_thread_id(key);
        let state = self
            .state
            .lock()
            .map_err(|_| "Cannot inspect Model Ops state for settings mutation.".to_owned())?;
        model_ops_mutation_context_for_expected_thread(
            &state.model_ops_actor_id,
            &state.model_ops_session_id,
            &state.model_ops_lease_id,
            &state.model_ops_active_lease_thread_id,
            fallback_actor_id,
            &expected_thread_id,
        )
        .map_err(|message| format!("Cannot mutate {key}: {message}"))
    }

    fn settings_model_ops_context_for_entries(
        &self,
        entries: &[(String, String, String)],
        fallback_actor_id: &str,
    ) -> Result<ModelOpsMutationContext, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Cannot inspect Model Ops state for settings save.".to_owned())?;
        let lease_configured = !state.model_ops_lease_id.trim().is_empty()
            || !state.model_ops_active_lease_thread_id.trim().is_empty();
        let model_ops_actor_id = state.model_ops_actor_id.trim();
        let fallback_actor_id = fallback_actor_id.trim();
        let actor_overridden =
            !model_ops_actor_id.is_empty() && model_ops_actor_id != fallback_actor_id;
        if (lease_configured || actor_overridden) && entries.len() != 1 {
            return Err(
                "Cannot save multiple defaults with one Model Ops lease; save one changed key whose lease thread is atelier.preferences.{key}, or clear the lease fields for the operator path."
                    .to_owned(),
            );
        }
        let key = entries[0].0.as_str();
        let expected_thread_id = preference_model_ops_thread_id(key);
        model_ops_mutation_context_for_expected_thread(
            &state.model_ops_actor_id,
            &state.model_ops_session_id,
            &state.model_ops_lease_id,
            &state.model_ops_active_lease_thread_id,
            fallback_actor_id,
            &expected_thread_id,
        )
        .map_err(|message| format!("Cannot mutate {key}: {message}"))
    }

    /// WP-CKC MT-042: the operator-facing collapsing "Settings / Defaults" region.
    /// Not a 4th `AtelierPanelTab` — it renders panel-wide above the tab strip. Every
    /// control is Argus-steerable by stable author_id; Save/Reset route through the
    /// atelier preference store (PostgreSQL + EventLedger).
    fn show_settings_region(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let actor = self
            .ckc_client
            .as_ref()
            .map(|client| client.actor_id().to_owned())
            .unwrap_or_default();
        let client_present = self.ckc_client.is_some();
        let mut dispatch_save: Option<Vec<(String, String, String)>> = None;
        let mut dispatch_reset: Option<String> = None;
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            let region = egui::CollapsingHeader::new("Settings / Defaults")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(
                            "Operator defaults for Atelier / CKC / PoseKit / Ingest. Persisted to \
                             PostgreSQL + EventLedger; applied to the live fields on load and save.",
                        )
                        .color(palette.text_subtle),
                    );

                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Default landing tab").color(palette.text));
                        let response = ui.text_edit_singleline(&mut state.settings_landing_tab);
                        emit_value_node(
                            ui.ctx(),
                            response.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_DEFAULT_TAB_AUTHOR_ID,
                            "Atelier default landing tab (castkit-codex | posekit | ingest)",
                            &state.settings_landing_tab,
                        );

                        ui.label(egui::RichText::new("CKC book mode").color(palette.text));
                        let response = ui.text_edit_singleline(&mut state.settings_ckc_book_mode);
                        emit_value_node(
                            ui.ctx(),
                            response.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_CKC_BOOK_MODE_AUTHOR_ID,
                            "CKC default book mode (sheet | story | notes | moodboard)",
                            &state.settings_ckc_book_mode,
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("PoseKit framing preset").color(palette.text));
                        let response =
                            ui.text_edit_singleline(&mut state.settings_pose_framing_preset);
                        emit_value_node(
                            ui.ctx(),
                            response.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_POSEKIT_FRAMING_PRESET_AUTHOR_ID,
                            "PoseKit default framing preset",
                            &state.settings_pose_framing_preset,
                        );

                        ui.label(egui::RichText::new("PoseKit lens mm").color(palette.text));
                        let response = ui.text_edit_singleline(&mut state.settings_pose_lens_mm);
                        emit_value_node(
                            ui.ctx(),
                            response.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_POSEKIT_LENS_AUTHOR_ID,
                            "PoseKit default framing lens millimeters (18..120)",
                            &state.settings_pose_lens_mm,
                        );
                    });

                    // WP-CKC MT-042 (F3): the 4 PoseKit framing-padding defaults, now
                    // functional end-to-end (synced to pose_framing_padding_*_px).
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("PoseKit padding px").color(palette.text));
                        ui.label(egui::RichText::new("Top").color(palette.text_subtle));
                        let top = ui.text_edit_singleline(&mut state.settings_pose_padding_top);
                        emit_value_node(
                            ui.ctx(),
                            top.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_POSEKIT_PADDING_TOP_AUTHOR_ID,
                            "PoseKit default top padding pixels (0..256)",
                            &state.settings_pose_padding_top,
                        );
                        ui.label(egui::RichText::new("Right").color(palette.text_subtle));
                        let right = ui.text_edit_singleline(&mut state.settings_pose_padding_right);
                        emit_value_node(
                            ui.ctx(),
                            right.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_POSEKIT_PADDING_RIGHT_AUTHOR_ID,
                            "PoseKit default right padding pixels (0..256)",
                            &state.settings_pose_padding_right,
                        );
                        ui.label(egui::RichText::new("Bottom").color(palette.text_subtle));
                        let bottom =
                            ui.text_edit_singleline(&mut state.settings_pose_padding_bottom);
                        emit_value_node(
                            ui.ctx(),
                            bottom.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_POSEKIT_PADDING_BOTTOM_AUTHOR_ID,
                            "PoseKit default bottom padding pixels (0..256)",
                            &state.settings_pose_padding_bottom,
                        );
                        ui.label(egui::RichText::new("Left").color(palette.text_subtle));
                        let left = ui.text_edit_singleline(&mut state.settings_pose_padding_left);
                        emit_value_node(
                            ui.ctx(),
                            left.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_POSEKIT_PADDING_LEFT_AUTHOR_ID,
                            "PoseKit default left padding pixels (0..256)",
                            &state.settings_pose_padding_left,
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("PoseKit default markers").color(palette.text));
                        let face = ui.checkbox(&mut state.settings_pose_face, "Face");
                        emit_node(
                            ui.ctx(),
                            face.id,
                            accesskit::Role::CheckBox,
                            ATELIER_SETTINGS_POSEKIT_MARKERS_FACE_AUTHOR_ID,
                            "PoseKit default face markers",
                            state.settings_pose_face,
                        );
                        let body = ui.checkbox(&mut state.settings_pose_body, "Body");
                        emit_node(
                            ui.ctx(),
                            body.id,
                            accesskit::Role::CheckBox,
                            ATELIER_SETTINGS_POSEKIT_MARKERS_BODY_AUTHOR_ID,
                            "PoseKit default body markers",
                            state.settings_pose_body,
                        );
                        let hands = ui.checkbox(&mut state.settings_pose_hands, "Hands");
                        emit_node(
                            ui.ctx(),
                            hands.id,
                            accesskit::Role::CheckBox,
                            ATELIER_SETTINGS_POSEKIT_MARKERS_HANDS_AUTHOR_ID,
                            "PoseKit default hand markers",
                            state.settings_pose_hands,
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Ingest batch tags").color(palette.text));
                        let response =
                            ui.text_edit_singleline(&mut state.settings_ingest_batch_tags);
                        emit_value_node(
                            ui.ctx(),
                            response.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_INGEST_BATCH_TAGS_AUTHOR_ID,
                            "Ingest default batch tags",
                            &state.settings_ingest_batch_tags,
                        );

                        ui.label(egui::RichText::new("Ingest policy").color(palette.text));
                        let response = ui.text_edit_singleline(&mut state.settings_ingest_policy);
                        emit_value_node(
                            ui.ctx(),
                            response.id,
                            accesskit::Role::TextInput,
                            ATELIER_SETTINGS_INGEST_POLICY_AUTHOR_ID,
                            "Ingest default policy (pass | reject | unsure)",
                            &state.settings_ingest_policy,
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        let save = ui.button("Save defaults");
                        emit_node(
                            ui.ctx(),
                            save.id,
                            accesskit::Role::Button,
                            ATELIER_SETTINGS_SAVE_AUTHOR_ID,
                            "Save Atelier defaults",
                            false,
                        );
                        if save.clicked() && client_present && !state.settings_save_pending {
                            // F1/F5/F6: validate + normalize client-side and send ONLY
                            // changed keys. An invalid enum/integer rejects the WHOLE
                            // save before any PUT, so there is never a partial write.
                            match settings_save_entries(&state) {
                                Ok(entries) if entries.is_empty() => {
                                    state.settings_status = "No default changes to save.".to_owned();
                                }
                                Ok(entries) => {
                                    state.settings_save_pending = true;
                                    dispatch_save = Some(entries);
                                }
                                Err(message) => {
                                    state.settings_status = format!("Cannot save: {message}.");
                                }
                            }
                        }
                        if state.settings_save_pending {
                            ui.label(
                                egui::RichText::new("saving...").color(palette.text_subtle),
                            );
                        }
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("Reset to default:").color(palette.text_subtle),
                        );
                        for key in SETTINGS_MANAGED_KEYS {
                            let button = ui.button(*key);
                            emit_node(
                                ui.ctx(),
                                button.id,
                                accesskit::Role::Button,
                                &settings_reset_author_id(key),
                                &format!("Reset {key} to default"),
                                false,
                            );
                            if button.clicked() && client_present {
                                dispatch_reset = Some((*key).to_owned());
                            }
                        }
                    });

                    let status = state.settings_status.clone();
                    let status_response =
                        ui.label(egui::RichText::new(&status).color(palette.text_subtle));
                    emit_value_node(
                        ui.ctx(),
                        status_response.id,
                        accesskit::Role::Label,
                        ATELIER_SETTINGS_STATUS_AUTHOR_ID,
                        "Atelier defaults status",
                        &status,
                    );
                });
            emit_node(
                ui.ctx(),
                region.header_response.id,
                accesskit::Role::Group,
                ATELIER_SETTINGS_REGION_AUTHOR_ID,
                "Atelier settings and defaults",
                false,
            );
        }
        if let Some(entries) = dispatch_save {
            if let Some(client) = self.ckc_client.as_ref() {
                let model_ops = match self.settings_model_ops_context_for_entries(&entries, &actor)
                {
                    Ok(model_ops) => model_ops,
                    Err(message) => {
                        if let Ok(mut state) = self.state.lock() {
                            state.settings_save_pending = false;
                            state.settings_status = message;
                        }
                        return;
                    }
                };
                client.save_preferences(
                    entries,
                    &model_ops.actor_id,
                    model_ops.context,
                    self.settings_save_cell.clone(),
                );
            }
        }
        if let Some(key) = dispatch_reset {
            if let Some(client) = self.ckc_client.as_ref() {
                let model_ops = match self.settings_model_ops_context_for_key(&key, &actor) {
                    Ok(model_ops) => model_ops,
                    Err(message) => {
                        if let Ok(mut state) = self.state.lock() {
                            state.settings_status = message;
                        }
                        return;
                    }
                };
                client.reset_preference(
                    &key,
                    &model_ops.actor_id,
                    model_ops.context,
                    self.settings_reset_cell.clone(),
                );
            }
        }
    }

    fn drain_model_ops_backend(&self) {
        let result = self
            .model_ops_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        let Some((request_id, result)) = result else {
            return;
        };
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.model_ops_active_request != Some(request_id) {
            return;
        }
        state.model_ops_active_request = None;
        match result {
            Ok(AtelierModelOperationResult::Lease(row)) => {
                if let Some(next_lease_id) = model_ops_next_active_lease_id(
                    &state.model_ops_lease_id,
                    &row.claim_id,
                    &row.effective_state,
                    row.lease_expired,
                ) {
                    if next_lease_id.is_empty() {
                        state.model_ops_active_lease_thread_id.clear();
                    } else {
                        state.model_ops_active_lease_thread_id = row.thread_id.clone();
                    }
                    state.model_ops_lease_id = next_lease_id;
                }
                state.model_ops_status = format!(
                    "lease_id={} thread_id={} actor_id={} session_id={} claim_mode={} state={} expires_at={} expired={}",
                    row.claim_id,
                    row.thread_id,
                    row.actor_id,
                    row.session_id,
                    row.claim_mode,
                    row.effective_state,
                    row.lease_expires_at_utc,
                    row.lease_expired
                );
            }
            Ok(AtelierModelOperationResult::Leases(rows)) => {
                let active = rows
                    .iter()
                    .filter(|row| row.effective_state == "active" && !row.lease_expired)
                    .count();
                state.model_ops_status = format!(
                    "lease_list thread_id={} total={} active={}",
                    state.model_ops_thread_id,
                    rows.len(),
                    active
                );
            }
            Ok(AtelierModelOperationResult::State(row)) => {
                let active = row
                    .leases
                    .iter()
                    .filter(|lease| lease.effective_state == "active" && !lease.lease_expired)
                    .count();
                let actor_id = state.model_ops_actor_id.trim().to_owned();
                let session_id = state.model_ops_session_id.trim().to_owned();
                let matching_active =
                    model_ops_matching_active_lease_id(&actor_id, &session_id, &row.leases)
                        .map(str::to_owned);
                if let Some(lease_id) = matching_active.as_ref() {
                    state.model_ops_lease_id = lease_id.clone();
                    state.model_ops_active_lease_thread_id = row.thread_id.clone();
                } else {
                    state.model_ops_lease_id.clear();
                    state.model_ops_active_lease_thread_id.clear();
                }
                let foreign_active = row.leases.iter().find(|lease| {
                    lease.effective_state.eq_ignore_ascii_case("active")
                        && !lease.lease_expired
                        && (lease.actor_id.trim() != actor_id
                            || lease.session_id.trim() != session_id)
                });
                let foreign_status = foreign_active
                    .map(|lease| {
                        format!(
                            "active_foreign=true foreign_claim_id={} foreign_actor_id={} foreign_session_id={}",
                            lease.claim_id, lease.actor_id, lease.session_id
                        )
                    })
                    .unwrap_or_else(|| "active_foreign=false".to_owned());
                state.model_ops_status = format!(
                    "state thread_id={} leases={} active={} matching_claim_id={} {} required_headers={} recovery_hint={}",
                    row.thread_id,
                    row.leases.len(),
                    active,
                    matching_active.as_deref().unwrap_or(""),
                    foreign_status,
                    row.required_headers_for_mutation.join("|"),
                    row.recovery_hint
                );
            }
            Ok(AtelierModelOperationResult::ActionReceipt(row)) => {
                state.model_ops_status = format!(
                    "receipt_id={} action_id={} status={} actor_id={} session_id={} params_sha256={} target_refs={} result_refs={}",
                    row.receipt_id,
                    row.action_id,
                    row.status,
                    row.actor_id,
                    row.session_id,
                    row.params_sha256,
                    row.target_refs.join("|"),
                    row.result_refs.join("|")
                );
            }
            Err(err) => {
                state.model_ops_status = format!("model_ops_error={err}");
            }
        }
    }

    fn show_model_ops_region(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        enum Dispatch {
            State {
                request_id: u64,
                thread_id: String,
            },
            Claim {
                request_id: u64,
                actor_id: String,
                session_id: String,
                thread_id: String,
                ttl_seconds: i64,
            },
            Release {
                request_id: u64,
                actor_id: String,
                session_id: String,
                claim_id: String,
            },
            Receipt {
                request_id: u64,
                spec: crate::backend_client::ActorRequestSpec,
            },
        }

        let client_present = self.ckc_client.is_some();
        let mut dispatch: Option<Dispatch> = None;
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            let region = egui::CollapsingHeader::new("Model Ops")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(
                            "Parallel-agent lease and receipt surface for Atelier workflow resources.",
                        )
                        .color(palette.text_subtle),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Actor").color(palette.text));
                        let actor = ui.text_edit_singleline(&mut state.model_ops_actor_id);
                        emit_value_node(
                            ui.ctx(),
                            actor.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_ACTOR_AUTHOR_ID,
                            "Model Ops actor id",
                            &state.model_ops_actor_id,
                        );
                        ui.label(egui::RichText::new("Session").color(palette.text));
                        let session = ui.text_edit_singleline(&mut state.model_ops_session_id);
                        emit_value_node(
                            ui.ctx(),
                            session.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_SESSION_AUTHOR_ID,
                            "Model Ops session id",
                            &state.model_ops_session_id,
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Thread").color(palette.text));
                        let thread = ui.text_edit_singleline(&mut state.model_ops_thread_id);
                        emit_value_node(
                            ui.ctx(),
                            thread.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_THREAD_AUTHOR_ID,
                            "Model Ops semantic resource thread id",
                            &state.model_ops_thread_id,
                        );
                        ui.label(egui::RichText::new("Lease").color(palette.text));
                        let lease = ui.text_edit_singleline(&mut state.model_ops_lease_id);
                        emit_value_node(
                            ui.ctx(),
                            lease.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_LEASE_AUTHOR_ID,
                            "Model Ops active lease id",
                            &state.model_ops_lease_id,
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("TTL").color(palette.text));
                        let ttl = ui.text_edit_singleline(&mut state.model_ops_ttl_seconds);
                        emit_value_node(
                            ui.ctx(),
                            ttl.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_TTL_AUTHOR_ID,
                            "Model Ops lease TTL seconds",
                            &state.model_ops_ttl_seconds,
                        );
                        ui.label(egui::RichText::new("Action").color(palette.text));
                        let action = ui.text_edit_singleline(&mut state.model_ops_action_id);
                        emit_value_node(
                            ui.ctx(),
                            action.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_ACTION_AUTHOR_ID,
                            "Model Ops action receipt action id",
                            &state.model_ops_action_id,
                        );
                        ui.label(egui::RichText::new("Receipt status").color(palette.text));
                        let receipt_status =
                            ui.text_edit_singleline(&mut state.model_ops_receipt_status);
                        emit_value_node(
                            ui.ctx(),
                            receipt_status.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_RECEIPT_STATUS_AUTHOR_ID,
                            "Model Ops action receipt status",
                            &state.model_ops_receipt_status,
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Error class").color(palette.text));
                        let error_class =
                            ui.text_edit_singleline(&mut state.model_ops_receipt_error_class);
                        emit_value_node(
                            ui.ctx(),
                            error_class.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_RECEIPT_ERROR_CLASS_AUTHOR_ID,
                            "Model Ops action receipt error class",
                            &state.model_ops_receipt_error_class,
                        );
                        ui.label(egui::RichText::new("Recovery hint").color(palette.text));
                        let recovery_hint =
                            ui.text_edit_singleline(&mut state.model_ops_receipt_recovery_hint);
                        emit_value_node(
                            ui.ctx(),
                            recovery_hint.id,
                            accesskit::Role::TextInput,
                            ATELIER_MODEL_OPS_RECEIPT_RECOVERY_HINT_AUTHOR_ID,
                            "Model Ops action receipt recovery hint",
                            &state.model_ops_receipt_recovery_hint,
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        let pending = state.model_ops_active_request.is_some();
                        let state_button = ui.add_enabled(client_present && !pending, egui::Button::new("Read state"));
                        emit_node(
                            ui.ctx(),
                            state_button.id,
                            accesskit::Role::Button,
                            ATELIER_MODEL_OPS_STATE_AUTHOR_ID,
                            "Read Model Ops lease state",
                            false,
                        );
                        if state_button.clicked() {
                            state.model_ops_request_seq += 1;
                            let request_id = state.model_ops_request_seq;
                            state.model_ops_active_request = Some(request_id);
                            state.model_ops_status =
                                format!("Reading model-operation state for {}...", state.model_ops_thread_id);
                            dispatch = Some(Dispatch::State {
                                request_id,
                                thread_id: state.model_ops_thread_id.clone(),
                            });
                        }

                        let claim_button =
                            ui.add_enabled(client_present && !pending, egui::Button::new("Claim lease"));
                        emit_node(
                            ui.ctx(),
                            claim_button.id,
                            accesskit::Role::Button,
                            ATELIER_MODEL_OPS_CLAIM_AUTHOR_ID,
                            "Claim exclusive Model Ops lease",
                            false,
                        );
                        if claim_button.clicked() {
                            match state.model_ops_ttl_seconds.trim().parse::<i64>() {
                                Ok(ttl_seconds) if ttl_seconds > 0 => {
                                    state.model_ops_request_seq += 1;
                                    let request_id = state.model_ops_request_seq;
                                    state.model_ops_active_request = Some(request_id);
                                    state.model_ops_status = format!(
                                        "Claiming exclusive lease for {} as {}...",
                                        state.model_ops_thread_id, state.model_ops_actor_id
                                    );
                                    dispatch = Some(Dispatch::Claim {
                                        request_id,
                                        actor_id: state.model_ops_actor_id.clone(),
                                        session_id: state.model_ops_session_id.clone(),
                                        thread_id: state.model_ops_thread_id.clone(),
                                        ttl_seconds,
                                    });
                                }
                                _ => {
                                    state.model_ops_status =
                                        "Cannot claim lease: TTL must be a positive integer.".to_owned();
                                }
                            }
                        }

                        let release_button =
                            ui.add_enabled(client_present && !pending, egui::Button::new("Release lease"));
                        emit_node(
                            ui.ctx(),
                            release_button.id,
                            accesskit::Role::Button,
                            ATELIER_MODEL_OPS_RELEASE_AUTHOR_ID,
                            "Release active Model Ops lease",
                            false,
                        );
                        if release_button.clicked() {
                            let claim_id = state.model_ops_lease_id.trim().to_owned();
                            if claim_id.is_empty() {
                                state.model_ops_status =
                                    "Cannot release lease: lease id is empty.".to_owned();
                            } else {
                                state.model_ops_request_seq += 1;
                                let request_id = state.model_ops_request_seq;
                                state.model_ops_active_request = Some(request_id);
                                state.model_ops_status = format!("Releasing lease {claim_id}...");
                                dispatch = Some(Dispatch::Release {
                                    request_id,
                                    actor_id: state.model_ops_actor_id.clone(),
                                    session_id: state.model_ops_session_id.clone(),
                                    claim_id,
                                });
                            }
                        }

                        let receipt_button =
                            ui.add_enabled(client_present && !pending, egui::Button::new("Record receipt"));
                        emit_node(
                            ui.ctx(),
                            receipt_button.id,
                            accesskit::Role::Button,
                            ATELIER_MODEL_OPS_RECEIPT_AUTHOR_ID,
                            "Record Model Ops action receipt",
                            false,
                        );
                        if receipt_button.clicked() {
                            if let Some(client) = self.ckc_client.as_ref() {
                                let receipt_actor_id =
                                    state.model_ops_actor_id.trim().to_owned();
                                let receipt_session_id =
                                    state.model_ops_session_id.trim().to_owned();
                                let receipt_thread_id =
                                    state.model_ops_thread_id.trim().to_owned();
                                let receipt_lease_id = state.model_ops_lease_id.trim().to_owned();
                                let receipt_action_id =
                                    state.model_ops_action_id.trim().to_owned();
                                let target_refs =
                                    vec![format!("atelier://model-operation/{}", receipt_thread_id)];
                                let evidence_refs = vec![
                                    "src/frontend/handshake_native/src/atelier_panel.rs".to_owned(),
                                ];
                                let result_refs = vec![format!(
                                    "atelier://model-operation/{}/receipt",
                                    receipt_thread_id
                                )];
                                let receipt_status =
                                    state.model_ops_receipt_status.trim().to_owned();
                                let receipt_error_class =
                                    state.model_ops_receipt_error_class.trim().to_owned();
                                let receipt_recovery_hint =
                                    state.model_ops_receipt_recovery_hint.trim().to_owned();
                                if let Some(blocker) = model_ops_receipt_dispatch_blocker(
                                    &receipt_lease_id,
                                    &receipt_session_id,
                                    &receipt_thread_id,
                                    &receipt_action_id,
                                    &receipt_status,
                                    &receipt_error_class,
                                    &receipt_recovery_hint,
                                    &target_refs,
                                    &evidence_refs,
                                    &result_refs,
                                ) {
                                    state.model_ops_status = blocker.to_owned();
                                    return;
                                }
                                let receipt_context = match model_ops_context_for_expected_thread(
                                    &receipt_session_id,
                                    &receipt_lease_id,
                                    &state.model_ops_active_lease_thread_id,
                                    &receipt_thread_id,
                                ) {
                                    Ok(Some(context)) => context,
                                    Ok(None) => {
                                        state.model_ops_status =
                                            "Cannot record receipt: session id and lease id are required."
                                                .to_owned();
                                        return;
                                    }
                                    Err(message) => {
                                        state.model_ops_status =
                                            format!("Cannot record receipt: {message}");
                                        return;
                                    }
                                };
                                let spec = client.model_operation_action_receipt_actor_request(
                                    &receipt_action_id,
                                    &receipt_session_id,
                                    serde_json::json!({
                                        "thread_id": receipt_thread_id.clone(),
                                        "lease_id": receipt_lease_id.clone(),
                                    }),
                                    &receipt_status,
                                    &target_refs,
                                    &evidence_refs,
                                    &result_refs,
                                    if receipt_error_class.is_empty() {
                                        None
                                    } else {
                                        Some(receipt_error_class.as_str())
                                    },
                                    if receipt_recovery_hint.is_empty() {
                                        None
                                    } else {
                                        Some(receipt_recovery_hint.as_str())
                                    },
                                    Some(&receipt_context),
                                    &receipt_actor_id,
                                );
                                state.model_ops_request_seq += 1;
                                let request_id = state.model_ops_request_seq;
                                state.model_ops_active_request = Some(request_id);
                                state.model_ops_status =
                                    format!("Recording receipt for {}...", receipt_action_id);
                                dispatch = Some(Dispatch::Receipt { request_id, spec });
                            }
                        }
                    });
                    let status = state.model_ops_status.clone();
                    let status_response =
                        ui.label(egui::RichText::new(&status).color(palette.text_subtle));
                    emit_value_node(
                        ui.ctx(),
                        status_response.id,
                        accesskit::Role::Label,
                        ATELIER_MODEL_OPS_STATUS_AUTHOR_ID,
                        "Model Ops status and last receipt",
                        &status,
                    );
                });
            emit_node(
                ui.ctx(),
                region.header_response.id,
                accesskit::Role::Button,
                ATELIER_MODEL_OPS_REGION_AUTHOR_ID,
                "Atelier Model Ops",
                false,
            );
        }
        if let (Some(client), Some(dispatch)) = (self.ckc_client.as_ref(), dispatch) {
            match dispatch {
                Dispatch::State {
                    request_id,
                    thread_id,
                } => client.dispatch_model_operation_state(
                    request_id,
                    &thread_id,
                    self.model_ops_cell.clone(),
                ),
                Dispatch::Claim {
                    request_id,
                    actor_id,
                    session_id,
                    thread_id,
                    ttl_seconds,
                } => client.dispatch_model_operation_claim_lease(
                    request_id,
                    &thread_id,
                    "local_large_model",
                    &session_id,
                    "exclusive_lease",
                    ttl_seconds,
                    "WP-CKC-posekit-overhaul",
                    "MT-022",
                    &actor_id,
                    self.model_ops_cell.clone(),
                ),
                Dispatch::Release {
                    request_id,
                    actor_id,
                    session_id,
                    claim_id,
                } => client.dispatch_model_operation_release_lease(
                    request_id,
                    &claim_id,
                    &session_id,
                    &actor_id,
                    self.model_ops_cell.clone(),
                ),
                Dispatch::Receipt { request_id, spec } => client
                    .dispatch_model_operation_action_receipt(
                        request_id,
                        spec,
                        self.model_ops_cell.clone(),
                    ),
            }
        }
    }

    fn ensure_ckc_load_requested(&self) {
        let Some(client) = self.ckc_client.as_ref() else {
            return;
        };
        let should_request = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.ckc_load_requested {
                false
            } else {
                state.ckc_load_requested = true;
                state.ckc_loading = true;
                state.ckc_error = None;
                true
            }
        };
        if should_request {
            client.fetch_ckc(self.ckc_cell.clone());
        }
    }

    fn drain_ckc_backend(&self) {
        let load_result = self.ckc_cell.lock().ok().and_then(|mut slot| slot.take());
        if let Some(result) = load_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_loading = false;
                match result {
                    Ok(data) => {
                        let selected_id = state
                            .ckc_characters
                            .get(state.ckc_selected_index)
                            .map(|row| row.character_internal_id.clone());
                        state.ckc_characters = data
                            .characters
                            .into_iter()
                            .map(CkcCharacterRecord::from_backend)
                            .collect();
                        state.ckc_selected_index = selected_id
                            .and_then(|id| {
                                state
                                    .ckc_characters
                                    .iter()
                                    .position(|row| row.character_internal_id == id)
                            })
                            .unwrap_or(0);
                        state.ckc_backend_loaded = true;
                        state.ckc_error = None;
                        state.ckc_search_results.clear();
                        state.ckc_last_export = None;
                        let filters = selected_ckc_search_filter_refs(&state);
                        state.ckc_tag_note_scope_ref = filters
                            .collection_ref
                            .clone()
                            .or(filters.character_ref.clone())
                            .unwrap_or_default();
                        state.ckc_search_status = format!(
                            "CKC database loaded ({} character(s)); run search for live refs",
                            state.ckc_characters.len()
                        );
                    }
                    Err(err) => {
                        state.ckc_backend_loaded = false;
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let create_result = self
            .ckc_create_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        let mut refresh_after_create = false;
        if let Some(result) = create_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_create_pending = false;
                match result {
                    Ok(character) => {
                        let record = CkcCharacterRecord::from_created_character(character);
                        let selected_id = record.character_internal_id.clone();
                        if let Some(existing) = state
                            .ckc_characters
                            .iter_mut()
                            .find(|row| row.character_internal_id == selected_id)
                        {
                            *existing = record;
                        } else {
                            state.ckc_characters.push(record);
                        }
                        state.ckc_selected_index = state
                            .ckc_characters
                            .iter()
                            .position(|row| row.character_internal_id == selected_id)
                            .unwrap_or(0);
                        state.ckc_new_display_name = "New character".to_owned();
                        state.ckc_backend_loaded = true;
                        state.ckc_loading = self.ckc_client.is_some();
                        state.ckc_last_export = None;
                        state.ckc_error = None;
                        refresh_after_create = self.ckc_client.is_some();
                    }
                    Err(err) => {
                        state.ckc_error = Some(err);
                    }
                }
            }
        }
        if refresh_after_create {
            if let Some(client) = self.ckc_client.as_ref() {
                client.fetch_ckc(self.ckc_cell.clone());
            }
        }

        let append_result = self
            .ckc_append_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = append_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_append_pending = false;
                match result {
                    Ok(sheet) => {
                        let character_internal_id = sheet.character_internal_id.clone();
                        if let Some(row) = state
                            .ckc_characters
                            .iter_mut()
                            .find(|row| row.character_internal_id == character_internal_id)
                        {
                            row.apply_sheet_version(sheet);
                        }
                        state.ckc_last_export = None;
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let template_result = self
            .ckc_template_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = template_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_template_pending = false;
                match result {
                    Ok(template) => {
                        let hash = short_hash(&template.template_hash);
                        state.ckc_template_status = format!(
                            "{} {} loaded: {} fields, {} sections, hash {}",
                            template.file_name,
                            template.template_version,
                            template.field_count,
                            template.section_count,
                            hash
                        );
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_template_status = format!("Template load failed: {err}");
                    }
                }
            }
        }

        let safe_subset_result = self
            .ckc_safe_subset_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = safe_subset_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_safe_subset_pending = false;
                match result {
                    Ok(subset) => {
                        state.ckc_template_status = format!(
                            "{} {} loaded: {} Field IDs in short/SFW-safe subset",
                            subset.file_name,
                            subset.template_version,
                            subset.field_ids.len()
                        );
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_template_status = format!("Safe subset load failed: {err}");
                    }
                }
            }
        }

        let import_result = self
            .ckc_import_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = import_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_import_pending = false;
                match result {
                    Ok(sheet) => {
                        let character_internal_id = sheet.character_internal_id.clone();
                        let seq = sheet.seq;
                        if let Some(row) = state
                            .ckc_characters
                            .iter_mut()
                            .find(|row| row.character_internal_id == character_internal_id)
                        {
                            row.apply_sheet_version(sheet);
                        }
                        state.ckc_export_status =
                            format!("Imported CKC sheet as append-only version v{seq}");
                        state.ckc_last_export = None;
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_export_status = format!("CKC sheet import failed: {err}");
                    }
                }
            }
        }

        let export_result = self
            .ckc_export_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = export_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_export_pending = false;
                match result {
                    Ok(export) => {
                        state.ckc_export_status = format!(
                            "Exported {} as {} ({} bytes, hash {})",
                            export.file_name,
                            export.format,
                            export.content.len(),
                            short_hash(&export.content_hash)
                        );
                        state.ckc_last_export = Some(export);
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_export_status = format!("CKC sheet export failed: {err}");
                    }
                }
            }
        }

        let suggestions_result = self
            .ckc_field_suggestions_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = suggestions_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_field_suggestion_pending = false;
                match result {
                    Ok(rows) => {
                        let field_id = state.ckc_field_suggestion_id.clone();
                        let count = rows.len();
                        state.ckc_field_suggestions = rows;
                        state.ckc_field_suggestion_status =
                            format!("Loaded {count} prior value(s) for {field_id}");
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_field_suggestions.clear();
                        state.ckc_field_suggestion_status =
                            format!("CKC field suggestions failed: {err}");
                    }
                }
            }
        }

        let artifact_links_result = self
            .ckc_sheet_artifact_links_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((target_sheet_version_id, result)) = artifact_links_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_sheet_artifact_pending = false;
                match result {
                    Ok(rows) => {
                        let outcome = apply_ckc_sheet_artifact_link_rows_to_state(
                            &mut state,
                            &target_sheet_version_id,
                            rows,
                        );
                        if outcome.current_selection_owns_target {
                            state.ckc_sheet_artifact_status =
                                format!("Loaded {} reusable sheet artifact link(s)", outcome.count);
                            state.ckc_error = None;
                        } else if !outcome.target_found {
                            state.ckc_sheet_artifact_status = format!(
                                "Ignored CKC sheet artifact result for stale sheet {target_sheet_version_id}"
                            );
                            state.ckc_error = Some(format!(
                                "No CKC character owns sheet_version_id={target_sheet_version_id}"
                            ));
                        }
                    }
                    Err(err) => {
                        let current_selection_owns_target = state
                            .ckc_characters
                            .get(state.ckc_selected_index)
                            .and_then(|character| character.sheet_version_id.as_deref())
                            == Some(target_sheet_version_id.as_str());
                        if current_selection_owns_target {
                            state.ckc_sheet_artifact_status =
                                format!("CKC sheet artifact link operation failed: {err}");
                            state.ckc_error = Some(err);
                        }
                    }
                }
            }
        }

        let album_create_result = self
            .ckc_media_album_create_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = album_create_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_album_create_pending = false;
                match result {
                    Ok(row) => {
                        let collection_id = row.collection_id.clone();
                        let character_internal_id = row.character_internal_id.clone();
                        let album = CkcMediaAlbumRecord::from_backend(row);
                        let mut applied = false;
                        for character in &mut state.ckc_characters {
                            if character.character_internal_id == character_internal_id {
                                if let Some(existing) = character
                                    .media_albums
                                    .iter_mut()
                                    .find(|existing| existing.collection_id == collection_id)
                                {
                                    *existing = album.clone();
                                } else {
                                    character.media_albums.push(album.clone());
                                }
                                character.media_album_count = character
                                    .media_album_count
                                    .max(character.media_albums.len());
                                applied = true;
                                break;
                            }
                        }
                        state.ckc_selected_album_collection_id = Some(collection_id.clone());
                        state.ckc_album_status = if applied {
                            format!("Created CKC album {collection_id}")
                        } else {
                            format!(
                                "Created CKC album {collection_id}, but its character is not visible"
                            )
                        };
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC album create failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let album_list_page_result = self
            .ckc_media_album_list_page_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = album_list_page_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_album_list_page_pending = false;
                let target_character_internal_id =
                    state.ckc_album_list_page_pending_character_id.take();
                match result {
                    Ok(page) => {
                        let selected_media_key = effective_ckc_selected_media_key(&state);
                        let offset = page.offset;
                        let next_offset = page.albums_next_offset;
                        let mut appended = 0usize;
                        let mut applied = false;
                        if let Some(target_character_internal_id) =
                            target_character_internal_id.as_deref()
                        {
                            if let Some(character_idx) =
                                state.ckc_characters.iter().position(|character| {
                                    character.character_internal_id == target_character_internal_id
                                })
                            {
                                appended = merge_ckc_media_album_list_page(
                                    &mut state.ckc_characters[character_idx],
                                    page,
                                    selected_media_key.as_deref(),
                                );
                                applied = true;
                            }
                        }
                        state.ckc_album_status = if applied {
                            format!(
                                "Loaded {appended} more CKC album(s) from offset {offset}; next offset {}",
                                next_offset
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "none".to_owned())
                            )
                        } else {
                            "Loaded CKC album page, but its character is not visible".to_owned()
                        };
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC album-list page load failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let album_items_result = self
            .ckc_media_album_items_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = album_items_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_album_link_pending = false;
                match result {
                    Ok(row) => {
                        let collection_id = row.collection_id.clone();
                        let selected_media_key = effective_ckc_selected_media_key(&state);
                        let mutation = row.mutation.clone();
                        let actor_id = row
                            .actor_id
                            .clone()
                            .unwrap_or_else(|| state.ckc_media_actor_id.clone());
                        let concurrency_policy = row
                            .concurrency_policy
                            .clone()
                            .unwrap_or_else(|| "unspecified".to_owned());
                        let target_asset_id = row.asset_id.clone();
                        let requested = row.requested;
                        let inserted = row.inserted;
                        let removed = row.removed;
                        let updated = row.updated;
                        let reordered = row.reordered;
                        let member_count = row.member_count;
                        let removed_by = row.removed_by.as_deref().unwrap_or("missing");
                        let unlink_receipt = row.unlink_receipt_id.as_deref().unwrap_or("none");
                        let unlinked_at = row.unlinked_at_utc.as_deref().unwrap_or("none");
                        let mut applied = false;
                        for character in &mut state.ckc_characters {
                            if let Some(album) = character
                                .media_albums
                                .iter_mut()
                                .find(|album| album.collection_id == collection_id)
                            {
                                album.collection_ref = row.collection_ref.clone();
                                album.member_count = row.member_count;
                                album.members_next_offset = row.members_next_offset;
                                if mutation.is_some() {
                                    replace_ckc_media_members(album, row.members);
                                } else {
                                    merge_ckc_media_members(
                                        album,
                                        row.members,
                                        selected_media_key.as_deref(),
                                    );
                                }
                                applied = true;
                                break;
                            }
                        }
                        state.ckc_selected_album_collection_id = Some(collection_id.clone());
                        if matches!(mutation.as_deref(), Some("unlink")) {
                            if selected_media_key.as_deref().is_some_and(|key| {
                                target_asset_id.as_deref().is_some_and(|asset_id| {
                                    key == ckc_media_occurrence_key(&collection_id, asset_id)
                                })
                            }) {
                                state.ckc_selected_media_key = None;
                            }
                        }
                        state.ckc_album_status = match mutation.as_deref() {
                            Some("unlink") if applied => format!(
                                "Unlinked media {} from album {collection_id}; media asset, bytes, image notes, and tags preserved; removed={removed}; member_count={member_count}; actor={actor_id}; removed_by={removed_by}; unlink_receipt_id={unlink_receipt}; unlinked_at_utc={unlinked_at}; policy={concurrency_policy}.",
                                target_asset_id.as_deref().unwrap_or("unknown")
                            ),
                            Some("unlink") => format!(
                                "Unlinked media from CKC album {collection_id}, but it is not visible; removed={removed}; actor={actor_id}; removed_by={removed_by}; unlink_receipt_id={unlink_receipt}; unlinked_at_utc={unlinked_at}; policy={concurrency_policy}."
                            ),
                            Some("link_ref_edit") if applied => format!(
                                "Updated album link provenance for media {} in album {collection_id}; asset-level provenance unchanged; updated={updated}; actor={actor_id}; policy={concurrency_policy}.",
                                target_asset_id.as_deref().unwrap_or("unknown")
                            ),
                            Some("link_ref_edit") => format!(
                                "Updated album link provenance for CKC album {collection_id}, but it is not visible; updated={updated}; actor={actor_id}; policy={concurrency_policy}."
                            ),
                            Some("reorder") if applied => format!(
                                "Reordered {reordered} CKC media item(s) in album {collection_id}; reloaded member_count={member_count}; actor={actor_id}; policy={concurrency_policy}."
                            ),
                            Some("reorder") => format!(
                                "Reordered CKC media album {collection_id}, but it is not visible; actor={actor_id}; policy={concurrency_policy}."
                            ),
                            _ if applied => format!(
                                "Linked {inserted} of {requested} requested media asset(s) into CKC album {collection_id}"
                            ),
                            _ => format!(
                                "Linked media into CKC album {collection_id}, but it is not visible"
                            ),
                        };
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC media link failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let album_page_result = self
            .ckc_media_album_page_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = album_page_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_album_page_pending = false;
                state.ckc_album_page_pending_collection_id = None;
                match result {
                    Ok(row) => {
                        let collection_id = row.collection_id.clone();
                        let selected_media_key = effective_ckc_selected_media_key(&state);
                        let mut appended = 0usize;
                        let mut applied = false;
                        for character in &mut state.ckc_characters {
                            if let Some(album) = character
                                .media_albums
                                .iter_mut()
                                .find(|album| album.collection_id == collection_id)
                            {
                                album.collection_ref = row.collection_ref.clone();
                                album.member_count = row.member_count;
                                album.members_next_offset = row.members_next_offset;
                                appended = merge_ckc_media_members(
                                    album,
                                    row.members,
                                    selected_media_key.as_deref(),
                                );
                                applied = true;
                                break;
                            }
                        }
                        state.ckc_selected_album_collection_id = Some(collection_id.clone());
                        state.ckc_album_status = if applied {
                            format!(
                                "Loaded {appended} more CKC media item(s) for album {collection_id}"
                            )
                        } else {
                            format!(
                                "Loaded CKC media page for album {collection_id}, but it is not visible"
                            )
                        };
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC media page load failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let media_result = self
            .ckc_media_notes_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = media_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_media_save_pending = false;
                match result {
                    Ok(row) => {
                        let asset_id = row.asset_id.clone();
                        let mut applied_count = 0usize;
                        for character in &mut state.ckc_characters {
                            for album in &mut character.media_albums {
                                for member in
                                    album.members.iter_mut().filter(|m| m.asset_id == asset_id)
                                {
                                    member.apply_notes_tags(&row);
                                    applied_count += 1;
                                }
                            }
                        }
                        state.ckc_error = if applied_count > 0 {
                            state.ckc_album_status = format!(
                                "Saved CKC media notes/tags for {asset_id} through backend; applied to {applied_count} visible row(s)"
                            );
                            None
                        } else {
                            state.ckc_album_status = format!(
                                "Saved CKC media notes/tags for {asset_id} through backend, but the asset is no longer visible"
                            );
                            Some(format!("saved media asset {asset_id} is no longer visible"))
                        };
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC media notes/tags save failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let document_result = self
            .ckc_character_document_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = document_result {
            // WP-CKC MT-012 (FIX 1): on an optimistic-concurrency conflict the moodboard head advanced
            // under us; capture the document id to re-fetch AFTER releasing the state lock so the operator
            // sees current backend state before re-saving (never a silent overwrite).
            let mut moodboard_conflict_refresh_doc_id: Option<String> = None;
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        let doc_type = row.doc_type.clone();
                        let document_id = row.document_id.clone();
                        let status =
                            apply_ckc_character_document_row(&mut state.ckc_characters, row);
                        match doc_type.as_str() {
                            "story" => {
                                state.ckc_active_story_document_id = Some(document_id);
                                state.ckc_story_status = status.unwrap_or_else(|| {
                                    "Saved CKC story document, but it is not visible".to_owned()
                                });
                            }
                            "moodboard" => {
                                state.ckc_active_moodboard_document_id = Some(document_id);
                                state.ckc_moodboard_status = status.unwrap_or_else(|| {
                                    "Saved CKC moodboard document, but it is not visible".to_owned()
                                });
                            }
                            _ => {}
                        }
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        if err.contains("stale_moodboard_document_version") {
                            // Typed conflict recovery: do NOT overwrite. Queue a refresh of the latest
                            // snapshot for the active moodboard document and mark the next result as a
                            // recovery so its consumer refreshes the tracked version WITHOUT clobbering the
                            // operator's live canvas edits (F3).
                            moodboard_conflict_refresh_doc_id = state
                                .ckc_active_moodboard_document_id
                                .clone()
                                .filter(|id| !is_pending_ckc_document_id(id));
                            state.ckc_moodboard_conflict_recovery =
                                moodboard_conflict_refresh_doc_id.is_some();
                            state.ckc_moodboard_status = if moodboard_conflict_refresh_doc_id
                                .is_some()
                            {
                                "CKC moodboard save conflict (stale document version): your canvas edits \
                                 are preserved; refreshing to the new head — click Save again to apply \
                                 them over it"
                                    .to_owned()
                            } else {
                                "CKC moodboard save conflict (stale document version): your edits were \
                                 NOT overwritten; re-open the moodboard to refresh before saving"
                                    .to_owned()
                            };
                            state.ckc_error = Some(err);
                        } else {
                            state.ckc_story_status =
                                format!("CKC story document save failed: {err}");
                            state.ckc_moodboard_status =
                                format!("CKC moodboard document save failed: {err}");
                            state.ckc_error = Some(err);
                        }
                    }
                }
            }
            if let (Some(client), Some(document_id)) =
                (self.ckc_client.as_ref(), moodboard_conflict_refresh_doc_id)
            {
                client.fetch_ckc_latest_moodboard_snapshot(
                    &document_id,
                    self.ckc_moodboard_latest_cell.clone(),
                );
            }
        }

        let story_card_result = self
            .ckc_story_card_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = story_card_result {
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        state.ckc_story_status =
                            apply_ckc_story_card_row(&mut state.ckc_characters, row)
                                .unwrap_or_else(|| {
                                    "Added CKC story card, but its story is not visible".to_owned()
                                });
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_story_status = format!("CKC story card save failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let story_beat_result = self
            .ckc_story_beat_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = story_beat_result {
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        state.ckc_story_status =
                            apply_ckc_story_beat_row(&mut state.ckc_characters, row)
                                .unwrap_or_else(|| {
                                    "Added CKC story beat, but its story is not visible".to_owned()
                                });
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_story_status = format!("CKC story beat save failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let moodboard_latest_result = self
            .ckc_moodboard_latest_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = moodboard_latest_result {
            if let Ok(mut state) = self.state.lock() {
                // WP-CKC MT-012 (F3): consume the recovery flag exactly once, so at most one result is
                // treated as a conflict recovery (a normal open never mistakes itself for one, and no
                // refetch loop can form).
                let conflict_recovery = state.ckc_moodboard_conflict_recovery;
                state.ckc_moodboard_conflict_recovery = false;
                match result {
                    Ok(row) => {
                        match apply_ckc_moodboard_snapshot_row(&mut state.ckc_characters, row) {
                            Ok((status, projection, snapshot_ref)) => {
                                if status.is_some() {
                                    if conflict_recovery {
                                        // Refresh already advanced current_version_id + body_raw_text (via
                                        // apply_ckc_moodboard_snapshot_row). Do NOT re-project onto the live
                                        // board — that would clobber the operator's just-failed canvas edits.
                                        // A re-save reverse-projects the preserved board over the new head.
                                        let _ = (projection, snapshot_ref);
                                        state.ckc_moodboard_status =
                                            "CKC moodboard refreshed to the new head; your canvas edits \
                                             are preserved — click Save again to apply them over it"
                                                .to_owned();
                                        state.ckc_error = None;
                                    } else if let Ok(mut board) = self.moodboard_board.lock() {
                                        projection.apply_to_board(&mut board, &snapshot_ref);
                                        state.ckc_moodboard_status = status.unwrap();
                                        state.ckc_error = None;
                                    } else {
                                        state.ckc_moodboard_status =
                                            "CKC moodboard canvas lock failed".to_owned();
                                        state.ckc_error = Some(state.ckc_moodboard_status.clone());
                                    }
                                } else {
                                    state.ckc_moodboard_status =
                                        "Opened CKC moodboard snapshot, but its document is not visible"
                                            .to_owned();
                                    state.ckc_error = Some(state.ckc_moodboard_status.clone());
                                }
                            }
                            Err(err) => {
                                state.ckc_moodboard_status =
                                    format!("CKC moodboard snapshot projection failed: {err}");
                                state.ckc_error = Some(state.ckc_moodboard_status.clone());
                            }
                        }
                    }
                    Err(err) => {
                        state.ckc_moodboard_status = format!("CKC moodboard open failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let search_result = self
            .ckc_search_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = search_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_search_pending = false;
                match result {
                    Ok(response) => {
                        let status = ckc_search_status_from_response(&response);
                        state.ckc_search_results = response
                            .results
                            .into_iter()
                            .map(CkcSearchResultRecord::from_backend)
                            .collect();
                        state.ckc_search_status = status;
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_search_results.clear();
                        state.ckc_search_status = format!("CKC search failed: {err}");
                    }
                }
            }
        }

        let tag_note_result = self
            .ckc_tag_note_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = tag_note_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_tag_note_pending = false;
                match result {
                    Ok(row) => {
                        let tag_text = row.tag_text.clone();
                        state.ckc_search_status = format!("Saved CKC tag note for {tag_text}");
                        attach_tag_note_to_visible_results(
                            &mut state.ckc_search_results,
                            CkcTagNoteRecord::from_backend(row),
                        );
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_search_status = format!("CKC tag note save failed: {err}");
                    }
                }
            }
        }
    }

    fn drain_posekit_export_backend(&self) {
        let export_result = self
            .pose_export_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = export_result {
            if let Ok(mut state) = self.state.lock() {
                if state.pose_active_export_request != Some(request_id) {
                    return;
                }
                state.pose_export_pending = false;
                state.pose_active_export_request = None;
                match result {
                    Ok(row) => {
                        let snapshot = posekit_export_snapshot_from_backend(row);
                        state.pose_export_status = format!(
                            "Exported backend Posekit OpenPose: yaw_deg={:.0} png_artifact_ref={} json_artifact_ref={} receipt_ref={}",
                            snapshot.yaw_deg,
                            snapshot.png_artifact_ref,
                            snapshot.json_artifact_ref,
                            snapshot.receipt_ref
                        );
                        state.pose_last_export = Some(snapshot);
                    }
                    Err(err) => {
                        state.pose_export_status = format!("Posekit OpenPose export failed: {err}");
                    }
                }
            }
        }
    }

    /// MT-043: drain a delivered Posekit source-image byte payload (if any) and feed it to the source
    /// viewport. On success the real bytes go to [`set_pose_source_image_bytes`] (decoded/uploaded next
    /// frame → `source_image=loaded`); on failure the image is cleared to the explicit empty state —
    /// never a fabricated placeholder. A response whose request id no longer matches the in-flight fetch
    /// (a superseded source ref) is dropped so a late reply cannot overwrite a newer source.
    fn drain_pose_source_bytes_backend(&self) {
        loop {
            let delivered = self
                .pose_source_bytes_cell
                .lock()
                .ok()
                .and_then(|mut slot| slot.pop_front());
            let Some((request_id, result)) = delivered else {
                return;
            };
            let mut clear_image = false;
            let mut set_bytes: Option<Vec<u8>> = None;
            if let Ok(mut state) = self.state.lock() {
                if state.pose_active_source_fetch != Some(request_id) {
                    // Stale response for a source ref that has since changed; keep draining so a
                    // current response queued behind it is not stranded until a later frame.
                    continue;
                }
                state.pose_active_source_fetch = None;
                match result {
                    Ok(bytes) => {
                        state.pose_source_status = format!(
                            "Loaded real source image bytes from ArtifactStore ({} bytes).",
                            bytes.len()
                        );
                        set_bytes = Some(bytes);
                    }
                    Err(err) => {
                        state.pose_source_status = format!(
                            "Source image fetch failed (kept explicit empty state, no placeholder): {err}"
                        );
                        clear_image = true;
                    }
                }
            }
            // Apply to the decode cache OUTSIDE the state lock (set_pose_source_image_bytes locks the
            // separate pose_source_image mutex; keeping the two locks non-overlapping avoids any ordering
            // coupling).
            if let Some(bytes) = set_bytes {
                self.set_pose_source_image_bytes(Some(bytes));
            } else if clear_image {
                self.set_pose_source_image_bytes(None);
            }
            return;
        }
    }

    /// MT-050: drain delivered exported OpenPose PNG bytes and feed the right-viewport decode cache.
    /// Stale responses for a superseded `png_artifact_ref` are ignored by request id, and failures keep
    /// the viewport in an explicit non-loaded state rather than falling through to a fabricated image.
    fn drain_pose_export_png_bytes_backend(&self) {
        loop {
            let delivered = self
                .pose_export_png_bytes_cell
                .lock()
                .ok()
                .and_then(|mut slot| slot.pop_front());
            let Some((request_id, result)) = delivered else {
                return;
            };
            let mut clear_image = false;
            let mut set_bytes: Option<Vec<u8>> = None;
            if let Ok(mut state) = self.state.lock() {
                if state.pose_active_export_png_fetch != Some(request_id) {
                    continue;
                }
                state.pose_active_export_png_fetch = None;
                match result {
                    Ok(bytes) => {
                        state.pose_export_png_status = format!(
                            "Loaded literal exported OpenPose PNG bytes from ArtifactStore ({} bytes).",
                            bytes.len()
                        );
                        set_bytes = Some(bytes);
                    }
                    Err(err) => {
                        state.pose_export_png_status = format!(
                            "Exported OpenPose PNG fetch failed (right viewport kept explicit non-loaded state): {err}"
                        );
                        clear_image = true;
                    }
                }
            }
            if let Some(bytes) = set_bytes {
                self.set_pose_openpose_image_bytes(Some(bytes));
            } else if clear_image {
                self.set_pose_openpose_image_bytes(None);
            }
            return;
        }
    }

    /// MT-043: when the Posekit source ref changes, resolve it and (if it is a native ArtifactStore
    /// media asset and a backend client is present) dispatch a one-shot off-thread byte fetch. Fires at
    /// most once per distinct ref (guarded by `pose_source_loaded_ref`) so the immediate-mode render loop
    /// does not spam the backend. An empty or non-resolvable ref clears the viewport to its explicit
    /// empty state instead of fetching. Never blocks the render thread and never fabricates bytes.
    ///
    /// INVARIANT (MT-043 regression fix): the panel NEVER auto-fetches the initial/default source ref.
    /// A backend byte-fetch fires ONLY for a ref the user actively changed to AFTER the Posekit tab
    /// first rendered. So opening the panel in its default/demo state — or any first render, on any tab —
    /// emits ZERO backend byte-fetch requests (no spurious startup GET, and it can never consume a
    /// single-shot test mock reserved for another request). A real user-set `atelier://media/<uuid>`
    /// still fetches + loads on the next sync after the edit.
    fn sync_pose_source_fetch(&self, state: &mut AtelierPanelState) {
        let trimmed = state.pose_source_ref.trim().to_owned();
        if state.pose_source_loaded_ref.as_deref() == Some(trimmed.as_str()) {
            return;
        }
        // First observation of the source ref (panel just initialized / Posekit first shown): record it
        // as the baseline and DO NOT fetch. The default/demo ref never triggers a backend request; only
        // a subsequent user edit does. The decode cache is left untouched (default = explicit empty
        // viewport), so the first render performs no image or network work.
        let first_observation = state.pose_source_loaded_ref.is_none();
        state.pose_source_loaded_ref = Some(trimmed.clone());
        state.pose_active_source_fetch = None;
        if first_observation {
            return;
        }

        // A user changed the ref: drop any stale image so a changed ref never lingers under old bytes.
        self.set_pose_source_image_bytes(None);

        if trimmed.is_empty() {
            state.pose_source_status = "No source image set.".to_owned();
            return;
        }
        let Some(asset_id) = parse_media_asset_id_from_ref(&trimmed) else {
            state.pose_source_status = format!(
                "Source ref is not a resolvable ArtifactStore media asset (expected atelier://media/<uuid>): {trimmed}"
            );
            return;
        };
        let Some(client) = self.ckc_client.as_ref() else {
            state.pose_source_status =
                "Backend offline; cannot fetch source image bytes (explicit empty state)."
                    .to_owned();
            return;
        };
        state.pose_source_fetch_seq = state.pose_source_fetch_seq.saturating_add(1);
        let request_id = state.pose_source_fetch_seq;
        state.pose_active_source_fetch = Some(request_id);
        state.pose_source_status = format!("Fetching source image bytes for {trimmed} ...");
        client.fetch_media_asset_bytes(&asset_id, request_id, self.pose_source_bytes_cell.clone());
    }

    /// MT-050: when a backend Posekit export exposes a new `png_artifact_ref`, fetch the literal PNG
    /// ArtifactStore payload once and render it in the right viewport. Offline preview refs are kept as
    /// honest keypoint-preview state, never treated as persisted bytes.
    fn sync_pose_export_png_fetch(&self, state: &mut AtelierPanelState) {
        let Some(snapshot) = state.pose_last_export.as_ref() else {
            if state.pose_export_png_loaded_ref.is_some() {
                state.pose_export_png_loaded_ref = None;
                state.pose_active_export_png_fetch = None;
                state.pose_export_png_status =
                    "No exported OpenPose PNG artifact loaded.".to_owned();
                self.set_pose_openpose_image_bytes(None);
            }
            return;
        };
        let artifact_ref = snapshot.png_artifact_ref.trim().to_owned();
        if state.pose_export_png_loaded_ref.as_deref() == Some(artifact_ref.as_str()) {
            return;
        }
        state.pose_export_png_loaded_ref = Some(artifact_ref.clone());
        state.pose_active_export_png_fetch = None;
        self.set_pose_openpose_image_bytes(None);

        if artifact_ref.is_empty() {
            state.pose_export_png_status =
                "No exported OpenPose PNG artifact ref available.".to_owned();
            return;
        }
        if !artifact_ref.starts_with("artifact://.handshake/artifacts/") {
            state.pose_export_png_status = format!(
                "OpenPose right viewport using keypoint preview; exported PNG ref is not ArtifactStore-backed: {artifact_ref}"
            );
            return;
        }
        let Some(client) = self.ckc_client.as_ref() else {
            state.pose_export_png_status =
                "Backend offline; cannot fetch exported OpenPose PNG bytes.".to_owned();
            return;
        };
        state.pose_export_png_fetch_seq = state.pose_export_png_fetch_seq.saturating_add(1);
        let request_id = state.pose_export_png_fetch_seq;
        state.pose_active_export_png_fetch = Some(request_id);
        state.pose_export_png_status =
            format!("Fetching exported OpenPose PNG bytes for {artifact_ref} ...");
        client.fetch_posekit_openpose_png_bytes(
            &artifact_ref,
            request_id,
            self.pose_export_png_bytes_cell.clone(),
        );
    }

    // ── MT-010 CKC linked-media image byte fetch (mirrors the MT-043 Posekit source-image path) ──────

    /// Drain a delivered CKC linked-media byte payload (if any) and feed it to the per-asset decode
    /// cache. On success the real bytes go to [`set_ckc_media_image_bytes`] (decoded/uploaded on the next
    /// paint → `media_image=loaded`); on failure the entry is cleared to the explicit empty state — never
    /// a fabricated placeholder. A response whose request id no longer matches the in-flight fetch (a
    /// superseded selection) is dropped so a late reply cannot overwrite a newer image.
    fn drain_ckc_media_bytes_backend(&self) {
        loop {
            let delivered = self
                .ckc_media_bytes_cell
                .lock()
                .ok()
                .and_then(|mut slot| slot.pop_front());
            let Some((request_id, result)) = delivered else {
                return;
            };
            let mut set_bytes: Option<Vec<u8>> = None;
            let mut clear_asset = false;
            let mut asset_id: Option<String> = None;
            if let Ok(mut state) = self.state.lock() {
                if state.ckc_active_media_fetch != Some(request_id) {
                    // Stale response for a selection that has since changed; keep draining so a
                    // current response queued behind it can be applied in this frame.
                    continue;
                }
                state.ckc_active_media_fetch = None;
                asset_id = state.ckc_media_fetch_asset.take();
                match result {
                    Ok(bytes) => {
                        state.ckc_media_image_status = format!(
                            "Loaded real linked image bytes from ArtifactStore ({} bytes).",
                            bytes.len()
                        );
                        set_bytes = Some(bytes);
                    }
                    Err(err) => {
                        state.ckc_media_image_status = format!(
                            "Linked image fetch failed (kept explicit empty state, no placeholder): {err}"
                        );
                        clear_asset = true;
                    }
                }
            }
            // Apply to the decode cache OUTSIDE the state lock (set_ckc_media_image_bytes locks the
            // separate ckc_media_image mutex; keeping the two locks non-overlapping avoids any ordering
            // coupling).
            let Some(asset_id) = asset_id else {
                return;
            };
            if let Some(bytes) = set_bytes {
                self.set_ckc_media_image_bytes(&asset_id, Some(bytes));
            } else if clear_asset {
                self.set_ckc_media_image_bytes(&asset_id, None);
            }
            return;
        }
    }

    /// Supply (or clear) the raw bytes of one CKC linked-media asset in the per-asset decode cache. Real
    /// image bytes (PNG/JPEG/WebP/GIF) are decoded and uploaded lazily on the next paint. Passing `None`
    /// removes the asset's cached entry so its viewer returns to the explicit empty state. This is the
    /// seam the backend byte fetch (and tests) use to feed real bytes in.
    pub fn set_ckc_media_image_bytes(&self, asset_id: &str, bytes: Option<Vec<u8>>) {
        if let Ok(mut cache) = self.ckc_media_image.lock() {
            match bytes {
                Some(bytes) => {
                    let last_access = cache.next_access();
                    let entry = cache.entries.entry(asset_id.to_owned()).or_default();
                    if entry.bytes.as_deref() != Some(bytes.as_slice()) {
                        entry.bytes = Some(bytes);
                        // Force a re-decode/upload on the next paint.
                        entry.hash = None;
                        entry.texture = None;
                        entry.dims = None;
                        entry.error = None;
                    }
                    entry.last_access = last_access;
                    cache.evict_to_limit(Some(asset_id));
                }
                None => {
                    cache.entries.remove(asset_id);
                }
            }
        }
    }

    /// Whether the per-asset decode cache already holds real bytes for `asset_id` (so a re-selection can
    /// repaint from cache instead of firing another backend byte fetch).
    fn ckc_media_asset_is_cached(&self, asset_id: &str) -> bool {
        self.ckc_media_image
            .lock()
            .ok()
            .map(|mut cache| {
                let last_access = cache.next_access();
                cache
                    .entries
                    .get_mut(asset_id)
                    .map(|entry| {
                        let has_bytes = entry.bytes.is_some();
                        if has_bytes {
                            entry.last_access = last_access;
                        }
                        has_bytes
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    pub fn image_cache_debug_snapshot(&self) -> AtelierImageCacheDebugSnapshot {
        let (ckc_entry_count, ckc_asset_ids) = self
            .ckc_media_image
            .lock()
            .ok()
            .map(|cache| {
                (
                    cache.entries.len(),
                    cache.entries.keys().cloned().collect::<Vec<_>>(),
                )
            })
            .unwrap_or_default();
        let (pose_source_has_bytes, pose_source_has_texture) = self
            .pose_source_image
            .lock()
            .ok()
            .map(|cache| (cache.bytes.is_some(), cache.texture.is_some()))
            .unwrap_or((false, false));
        AtelierImageCacheDebugSnapshot {
            ckc_entry_limit: CKC_MEDIA_IMAGE_CACHE_ENTRY_LIMIT,
            ckc_entry_count,
            ckc_asset_ids,
            pose_source_slot_limit: 1,
            pose_source_has_bytes,
            pose_source_has_texture,
        }
    }

    /// When the selected CKC linked-media image changes, resolve it and (if it is a native ArtifactStore
    /// media asset and a backend client is present) dispatch a one-shot off-thread byte fetch. Fires at
    /// most once per distinct selection (guarded by `ckc_media_loaded_ref`) so the immediate-mode render
    /// loop does not spam the backend; a cached asset repaints without a refetch. An empty or
    /// non-resolvable selection clears the viewer to its explicit empty state instead of fetching. Never
    /// blocks the render thread and never fabricates bytes.
    ///
    /// INVARIANT (mirrors the MT-043 regression fix): the panel NEVER auto-fetches the initial/default
    /// selection. A backend byte fetch fires ONLY for a selection the user actively changed to AFTER the
    /// CKC tab first rendered. So opening the panel in its default/demo state — where the seeded first
    /// linked image is a resolvable `atelier://media/<uuid>` ref — emits ZERO backend byte-fetch requests
    /// (no spurious startup GET, and it can never consume a single-shot test mock reserved for another
    /// request). A real user-driven selection change still fetches + loads on the next sync.
    fn sync_ckc_media_fetch(&self, state: &mut AtelierPanelState, selected_index: usize) {
        let selected_media_key = state.ckc_selected_media_key.clone();
        let selected_media_ref = state
            .ckc_characters
            .get(selected_index)
            .and_then(|character| {
                character
                    .selected_or_first_media_location(selected_media_key.as_deref())
                    .map(|(album_idx, member_idx)| {
                        character.media_albums[album_idx].members[member_idx]
                            .media_ref
                            .trim()
                            .to_owned()
                    })
            });
        let current_ref = selected_media_ref.unwrap_or_default();
        if state.ckc_media_loaded_ref.as_deref() == Some(current_ref.as_str()) {
            if current_ref.is_empty() {
                return;
            }
            let Some(asset_id) = parse_media_asset_id_from_ref(&current_ref) else {
                return;
            };
            if self.ckc_media_asset_is_cached(&asset_id) {
                return;
            }
            let previously_loaded = state
                .ckc_media_image_status
                .contains("Loaded real linked image")
                || state
                    .ckc_media_image_status
                    .contains("Loaded linked image from cache");
            if !previously_loaded {
                return;
            }
        }
        // First observation of the selection (panel just initialized / CKC first shown): record it as the
        // baseline and DO NOT fetch. The default/demo selection never triggers a backend request; only a
        // subsequent user change does.
        let first_observation = state.ckc_media_loaded_ref.is_none();
        state.ckc_media_loaded_ref = Some(current_ref.clone());
        state.ckc_active_media_fetch = None;
        state.ckc_media_fetch_asset = None;
        if first_observation {
            return;
        }

        if current_ref.is_empty() {
            state.ckc_media_image_status = "No linked image selected.".to_owned();
            return;
        }
        let Some(asset_id) = parse_media_asset_id_from_ref(&current_ref) else {
            state.ckc_media_image_status = format!(
                "Linked media ref is not a resolvable ArtifactStore asset (expected atelier://media/<uuid>): {current_ref}"
            );
            return;
        };
        // Per-asset cache hit: repaint from the already-fetched bytes, no refetch.
        if self.ckc_media_asset_is_cached(&asset_id) {
            state.ckc_media_image_status =
                format!("Loaded linked image from cache for asset {asset_id}.");
            return;
        }
        let Some(client) = self.ckc_client.as_ref() else {
            state.ckc_media_image_status =
                "Backend offline; cannot fetch linked image bytes (explicit empty state)."
                    .to_owned();
            return;
        };
        state.ckc_media_fetch_seq = state.ckc_media_fetch_seq.saturating_add(1);
        let request_id = state.ckc_media_fetch_seq;
        state.ckc_active_media_fetch = Some(request_id);
        state.ckc_media_fetch_asset = Some(asset_id.clone());
        state.ckc_media_image_status = format!("Fetching linked image bytes for {current_ref} ...");
        client.fetch_media_asset_bytes(&asset_id, request_id, self.ckc_media_bytes_cell.clone());
    }

    /// Decode + upload the cached bytes for one CKC linked-media asset into a GPU texture (once per byte
    /// change) and report what the media viewer should paint. Real bytes → `Loaded`; a decode failure →
    /// an explicit `DecodeError`; no asset / no bytes → `Empty`. Never returns a fabricated placeholder.
    fn prepare_ckc_media_render(
        &self,
        ctx: &egui::Context,
        asset_id: Option<&str>,
    ) -> PoseSourceRender {
        let Some(asset_id) = asset_id else {
            return PoseSourceRender::Empty;
        };
        let Ok(mut cache) = self.ckc_media_image.lock() else {
            return PoseSourceRender::Empty;
        };
        let last_access = cache.next_access();
        let Some(entry) = cache.entries.get_mut(asset_id) else {
            return PoseSourceRender::Empty;
        };
        entry.last_access = last_access;
        let Some(bytes) = entry.bytes.clone() else {
            return PoseSourceRender::Empty;
        };
        let hash = stable_bytes_hash(&bytes);
        let needs_decode =
            entry.hash != Some(hash) || (entry.texture.is_none() && entry.error.is_none());
        if needs_decode {
            match image::load_from_memory(&bytes) {
                Ok(dynamic) => {
                    let rgba = dynamic.to_rgba8();
                    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
                    let handle = ctx.load_texture(
                        format!("atelier-ckc-media-image-{asset_id}"),
                        color,
                        egui::TextureOptions::LINEAR,
                    );
                    entry.texture = Some(handle);
                    entry.dims = Some((w, h));
                    entry.error = None;
                    entry.hash = Some(hash);
                }
                Err(err) => {
                    entry.texture = None;
                    entry.dims = None;
                    entry.error = Some(err.to_string());
                    entry.hash = Some(hash);
                }
            }
        }
        if let Some(err) = entry.error.clone() {
            return PoseSourceRender::DecodeError(err);
        }
        match (entry.texture.clone(), entry.dims) {
            (Some(texture), Some((width, height))) => PoseSourceRender::Loaded {
                texture,
                width,
                height,
                byte_len: bytes.len(),
            },
            _ => PoseSourceRender::Empty,
        }
    }

    fn drain_contact_sheet_export_backend(&self) {
        let export_result = self
            .ingest_contact_export_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = export_result {
            if let Ok(mut state) = self.state.lock() {
                if state.ingest_active_contact_export_request != Some(request_id) {
                    state.ingest_contact_sheet_receipt = format!(
                        "ignored_stale_contact_sheet=true request_seq={request_id} expected_request_seq={:?}",
                        state.ingest_active_contact_export_request
                    );
                    return;
                }
                state.ingest_contact_export_pending = false;
                state.ingest_active_contact_export_request = None;
                match result {
                    Ok(row) => {
                        let snapshot = contact_sheet_snapshot_from_backend(row);
                        state.ingest_contact_sheet_receipt =
                            contact_sheet_export_preview(&snapshot);
                        state.ingest_status = format!(
                            "Exported backend contact sheet: {}x{}@{}dpi rendered_item_count={} omitted_item_count={} svg_artifact_ref={} receipt_ref={}",
                            snapshot.rows,
                            snapshot.columns,
                            snapshot.dpi,
                            snapshot.rendered_item_count,
                            snapshot.omitted_item_count,
                            snapshot.svg_artifact_ref,
                            snapshot.receipt_ref
                        );
                        state.ingest_last_contact_sheet = Some(snapshot);
                    }
                    Err(err) => {
                        state.ingest_contact_sheet_receipt =
                            format!("contact_sheet_export_error={err}");
                        state.ingest_status = format!("Contact sheet export failed: {err}");
                    }
                }
            }
        }
    }

    fn drain_facial_analysis_backend(&self) {
        let analysis_result = self
            .ingest_facial_analysis_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = analysis_result {
            if let Ok(mut state) = self.state.lock() {
                if state.ingest_active_facial_analysis_request != Some(request_id) {
                    state.ingest_facial_analysis_receipt = format!(
                        "ignored_stale_facial_analysis=true request_seq={request_id} expected_request_seq={:?}",
                        state.ingest_active_facial_analysis_request
                    );
                    return;
                }
                state.ingest_facial_analysis_pending = false;
                state.ingest_active_facial_analysis_request = None;
                match result {
                    Ok(row) => {
                        let snapshot = facial_analysis_snapshot_from_backend(row);
                        state.ingest_facial_analysis_receipt =
                            facial_analysis_receipt_preview(Some(&snapshot));
                        state.ingest_status = format!(
                            "Exported backend Facial Ingest analysis: batch_id={} profile={} item_count={} decoded_count={} duplicate_group_count={} analysis_artifact_ref={} receipt_ref={}",
                            snapshot.batch_id,
                            snapshot.profile,
                            snapshot.item_count,
                            snapshot.decoded_count,
                            snapshot.duplicate_group_count,
                            snapshot.analysis_artifact_ref,
                            snapshot.receipt_ref
                        );
                        state.ingest_last_facial_analysis = Some(snapshot);
                    }
                    Err(err) => {
                        state.ingest_last_facial_analysis = None;
                        state.ingest_facial_analysis_receipt =
                            format!("facial_analysis_error={err}");
                        state.ingest_status = format!("Facial Ingest analysis failed: {err}");
                    }
                }
            }
        }
    }

    /// MT-030: drain the `GET /atelier/facial/features` capability registry result into the readout.
    fn drain_facial_features_backend(&self) {
        let features_result = self
            .ingest_facial_features_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = features_result {
            if let Ok(mut state) = self.state.lock() {
                if state.facial_features_active_request != Some(request_id) {
                    return;
                }
                state.facial_features_pending = false;
                state.facial_features_active_request = None;
                match result {
                    Ok(row) => {
                        state.ingest_status = format!(
                            "Loaded Facial capability registry: feature_count={} command_routes={}",
                            row.feature_count,
                            row.command_routes.len()
                        );
                        state.facial_features_readout = facial_features_preview(&row);
                    }
                    Err(err) => {
                        state.facial_features_readout = format!("facial_features_error={err}");
                        state.ingest_status =
                            format!("Facial capability registry load failed: {err}");
                    }
                }
            }
        }
    }

    /// MT-030/MT-031: drain ANY review command response (session/claim/decision/status/montage/export).
    /// Succeeded command envelopes update the command-specific lineage refs needed to chain the next
    /// command; degraded envelopes retain their result artifact for readout/recovery without pretending
    /// every command can advance lineage. Blocked/error Ok(row) outcome envelopes carry no result artifact,
    /// so lineage capture is a no-op and the primary status line is rendered honestly from `row.status` +
    /// error code + recovery hint. The `Err` path surfaces the backend transport/parse error honestly.
    /// The primary status line never fabricates success.
    fn drain_facial_command_backend(&self) {
        let command_result = self
            .ingest_facial_command_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = command_result {
            if let Ok(mut state) = self.state.lock() {
                if state.facial_review_active_request != Some(request_id) {
                    state.facial_command_receipt = format!(
                        "ignored_stale_facial_command=true request_seq={request_id} expected_request_seq={:?}",
                        state.facial_review_active_request
                    );
                    return;
                }
                state.facial_review_pending = false;
                state.facial_review_active_request = None;
                match result {
                    Ok(row) => {
                        state.facial_command_receipt = append_facial_parse_notes(
                            facial_command_receipt_preview(&row),
                            &state.facial_review_parse_notes,
                        );
                        let result_artifact_ref = row
                            .result_artifact
                            .as_ref()
                            .map(|artifact| artifact.artifact_ref.clone());
                        let command = row.command.as_str();
                        if command.contains(".session") {
                            if row.status == "succeeded" {
                                if let Some(reference) = result_artifact_ref {
                                    state.facial_review_session_ref = reference;
                                }
                            }
                            if row.status == "succeeded" {
                                if let Some(session_id) =
                                    facial_review_session_id_from_command_result(&row.result)
                                {
                                    state.facial_review_session_id = session_id;
                                }
                            }
                        } else if command.contains(".claim") {
                            if let Some(reference) = result_artifact_ref {
                                state.facial_review_claim_ref = reference.clone();
                                if !state.facial_review_claim_refs.contains(&reference) {
                                    state.facial_review_claim_refs.push(reference);
                                }
                            }
                        } else if command.contains(".decision") {
                            if let Some(reference) = result_artifact_ref {
                                if !state.facial_review_decision_refs.contains(&reference) {
                                    state.facial_review_decision_refs.push(reference);
                                }
                            }
                        } else if command.contains(".status") {
                            state.facial_review_status_readout = facial_review_status_preview(&row);
                        } else if command.contains(".montage") {
                            state.facial_review_montage_readout =
                                facial_review_montage_content_preview(&row);
                        } else if command.contains(".export") {
                            state.facial_review_export_readout =
                                facial_review_export_content_preview(&row);
                        }
                        let degraded = if row.degraded_reasons.is_empty() {
                            "none".to_owned()
                        } else {
                            row.degraded_reasons.join("|")
                        };
                        let receipt_ref = row.receipt_ref.as_deref().unwrap_or("<none>");
                        let recovery_hint = row.recovery_hint.as_deref().unwrap_or("<none>");
                        let error = row.error.as_deref().unwrap_or("<none>");
                        // MT-031: the parser now returns Ok(row) for succeeded/degraded/blocked/error
                        // outcome envelopes (blocked/degraded/error used to arrive as Err). Branch on
                        // row.status so the primary status line is HONEST — it must never say
                        // "succeeded" for a blocked/degraded/error outcome. The accurate
                        // facial_command_receipt readout above is unchanged.
                        let status_line = match row.status.as_str() {
                            "succeeded" => format!(
                                "Facial review command succeeded: command={} status={} receipt_ref={} degraded_reasons={}",
                                row.command, row.status, receipt_ref, degraded
                            ),
                            "degraded" => format!(
                                "Facial review command DEGRADED: command={} status=degraded receipt_ref={} degraded_reasons={} recovery_hint={}",
                                row.command, receipt_ref, degraded, recovery_hint
                            ),
                            // blocked | error (and, defensively, any non-success status): an honest
                            // FAILURE line carrying the stable status + error code + recovery hint.
                            _ => format!(
                                "Facial review command {}: command={} status={} error={} recovery_hint={} receipt_ref={}",
                                row.status,
                                row.command,
                                row.status,
                                error,
                                recovery_hint,
                                receipt_ref
                            ),
                        };
                        state.ingest_status = append_facial_parse_notes(
                            status_line,
                            &state.facial_review_parse_notes,
                        );
                    }
                    Err(err) => {
                        state.facial_command_receipt = append_facial_parse_notes(
                            format!("facial_command_error={err}"),
                            &state.facial_review_parse_notes,
                        );
                        state.ingest_status = append_facial_parse_notes(
                            format!("Facial review command failed: {err}"),
                            &state.facial_review_parse_notes,
                        );
                    }
                }
            }
        }
    }

    fn drain_prompt_feedback_backend(&self) {
        let import_result = self
            .prompt_feedback_import_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = import_result {
            if let Ok(mut state) = self.state.lock() {
                if state.prompt_feedback_active_import_request != Some(request_id) {
                    state.prompt_feedback_status = format!(
                        "ignored_stale_prompt_feedback_import=true request_seq={request_id} expected_request_seq={:?}",
                        state.prompt_feedback_active_import_request
                    );
                    return;
                }
                let current_fingerprint = prompt_feedback_import_fingerprint(
                    &state.prompt_feedback_project,
                    &state.prompt_feedback_adapter,
                    &state.prompt_feedback_rows,
                );
                if state.prompt_feedback_active_import_fingerprint.as_deref()
                    != Some(current_fingerprint.as_str())
                {
                    let expected_fingerprint = state
                        .prompt_feedback_active_import_fingerprint
                        .clone()
                        .unwrap_or_else(|| "<none>".to_owned());
                    state.prompt_feedback_import_pending = false;
                    state.prompt_feedback_active_import_request = None;
                    state.prompt_feedback_active_import_fingerprint = None;
                    state.prompt_feedback_status = format!(
                        "ignored_stale_prompt_feedback_import=true reason=input_fingerprint_changed request_seq={request_id} expected_fingerprint={expected_fingerprint} current_fingerprint={current_fingerprint}"
                    );
                    return;
                }
                state.prompt_feedback_import_pending = false;
                state.prompt_feedback_active_import_request = None;
                state.prompt_feedback_active_import_fingerprint = None;
                match result {
                    Ok(row) => {
                        let imported_count = row.imported_count;
                        let case_count = row.cases.len();
                        let first_case = row.cases.first().cloned();
                        state.prompt_feedback_rule_pack = row.seed_rule_pack.rule_pack_id.clone();
                        state.prompt_feedback_cases = row
                            .cases
                            .into_iter()
                            .map(prompt_feedback_case_row_from_backend)
                            .collect();
                        if state
                            .prompt_feedback_selected_case
                            .as_deref()
                            .is_some_and(|selected| {
                                !state
                                    .prompt_feedback_cases
                                    .iter()
                                    .any(|case| case.case_id == selected)
                            })
                        {
                            state.prompt_feedback_selected_case = None;
                        }
                        let first_case_id = first_case
                            .as_ref()
                            .map(|case| case.case_id.as_str())
                            .unwrap_or("<none>");
                        let first_source_case_id = first_case
                            .as_ref()
                            .map(|case| case.source_case_id.as_str())
                            .unwrap_or("<none>");
                        let requested_by = first_case
                            .as_ref()
                            .map(|case| case.imported_by.as_str())
                            .unwrap_or("<unknown>");
                        state.prompt_feedback_status = format!(
                            "backend_persisted source=backend_persisted imported_count={imported_count} case_count={case_count} seed_rule_pack_id={} seed_rule_pack_version={} seed_rule_pack_hash={} first_case_id={first_case_id} first_source_case_id={first_source_case_id} requested_by={requested_by}",
                            state.prompt_feedback_rule_pack,
                            row.seed_rule_pack.version,
                            row.seed_rule_pack.content_hash
                        );
                    }
                    Err(err) => {
                        state.prompt_feedback_status =
                            format!("backend_failed prompt_feedback_import_error={err}");
                    }
                }
            }
        }

        let rewrite_result = self
            .prompt_feedback_rewrite_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = rewrite_result {
            if let Ok(mut state) = self.state.lock() {
                if state.prompt_feedback_active_rewrite_request != Some(request_id) {
                    state.prompt_feedback_trace = format!(
                        "ignored_stale_prompt_feedback_rewrite=true request_seq={request_id} expected_request_seq={:?}",
                        state.prompt_feedback_active_rewrite_request
                    );
                    return;
                }
                let current_fingerprint = prompt_feedback_rewrite_fingerprint(
                    &state.prompt_feedback_rule_pack,
                    state.prompt_feedback_selected_case.as_deref(),
                    &state.prompt_feedback_selected_tags,
                );
                if state.prompt_feedback_active_rewrite_fingerprint.as_deref()
                    != Some(current_fingerprint.as_str())
                {
                    let expected_fingerprint = state
                        .prompt_feedback_active_rewrite_fingerprint
                        .clone()
                        .unwrap_or_else(|| "<none>".to_owned());
                    state.prompt_feedback_rewrite_pending = false;
                    state.prompt_feedback_active_rewrite_request = None;
                    state.prompt_feedback_active_rewrite_fingerprint = None;
                    state.prompt_feedback_trace = format!(
                        "ignored_stale_prompt_feedback_rewrite=true reason=input_fingerprint_changed request_seq={request_id} expected_fingerprint={expected_fingerprint} current_fingerprint={current_fingerprint}"
                    );
                    return;
                }
                state.prompt_feedback_rewrite_pending = false;
                state.prompt_feedback_active_rewrite_request = None;
                state.prompt_feedback_active_rewrite_fingerprint = None;
                match result {
                    Ok(row) => {
                        let changed_fields = if row.changed_fields.is_empty() {
                            "<none>".to_owned()
                        } else {
                            row.changed_fields.join("|")
                        };
                        state.prompt_feedback_rewrite_preview = format!(
                            "backend_persisted rewrite_id={} case_id={} source_case_id={} rule_pack_id={} rule_pack_version={} changed_fields={} output_hash={} rewritten_positive_prompt={} rewritten_negative_prompt={}",
                            row.rewrite_id,
                            row.case_id,
                            row.source_case_id,
                            row.rule_pack_id,
                            row.rule_pack_version,
                            changed_fields,
                            row.output_hash,
                            row.rewritten_positive_prompt,
                            row.rewritten_negative_prompt
                        );
                        state.prompt_feedback_trace = format!(
                            "backend_persisted rewrite_id={} input_hash={} output_hash={} planned_by={} outcome={}",
                            row.rewrite_id,
                            row.input_hash,
                            row.output_hash,
                            row.planned_by,
                            row.outcome
                        );
                    }
                    Err(err) => {
                        state.prompt_feedback_rewrite_preview =
                            format!("backend_failed prompt_feedback_rewrite_error={err}");
                        state.prompt_feedback_trace =
                            "backend_failed rule_trace_unavailable=true".to_owned();
                    }
                }
            }
        }

        let export_result = self
            .prompt_feedback_export_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = export_result {
            if let Ok(mut state) = self.state.lock() {
                if state.prompt_feedback_active_export_request != Some(request_id) {
                    state.prompt_feedback_export_status = format!(
                        "ignored_stale_prompt_feedback_export=true request_seq={request_id} expected_request_seq={:?}",
                        state.prompt_feedback_active_export_request
                    );
                    return;
                }
                let current_case_ids = prompt_feedback_export_case_ids(
                    &state.prompt_feedback_cases,
                    state.prompt_feedback_selected_case.as_deref(),
                );
                let current_fingerprint = prompt_feedback_export_fingerprint(
                    &state.prompt_feedback_rule_pack,
                    &current_case_ids,
                );
                if state.prompt_feedback_active_export_fingerprint.as_deref()
                    != Some(current_fingerprint.as_str())
                {
                    let expected_fingerprint = state
                        .prompt_feedback_active_export_fingerprint
                        .clone()
                        .unwrap_or_else(|| "<none>".to_owned());
                    state.prompt_feedback_export_pending = false;
                    state.prompt_feedback_active_export_request = None;
                    state.prompt_feedback_active_export_fingerprint = None;
                    state.prompt_feedback_export_status = format!(
                        "ignored_stale_prompt_feedback_export=true reason=input_fingerprint_changed request_seq={request_id} expected_fingerprint={expected_fingerprint} current_fingerprint={current_fingerprint}"
                    );
                    return;
                }
                state.prompt_feedback_export_pending = false;
                state.prompt_feedback_active_export_request = None;
                state.prompt_feedback_active_export_fingerprint = None;
                match result {
                    Ok(row) => {
                        state.prompt_feedback_export_status = format!(
                            "backend_persisted export_id={} rule_pack_id={} rule_pack_version={} artifact_ref={} manifest_ref={} content_hash={} byte_len={} row_count={} source_case_ids={} rewrite_ids={} exported_by={}",
                            row.export_id,
                            row.rule_pack_id,
                            row.rule_pack_version,
                            row.artifact_ref,
                            row.manifest_ref.unwrap_or_else(|| "<none>".to_owned()),
                            row.content_hash,
                            row.byte_len,
                            row.row_count,
                            row.source_case_ids.join("|"),
                            row.rewrite_ids.join("|"),
                            row.exported_by
                        );
                    }
                    Err(err) => {
                        state.prompt_feedback_export_status =
                            format!("backend_failed prompt_feedback_export_error={err}");
                    }
                }
            }
        }
    }

    fn drain_ingest_classification_backend(&self) {
        let outcome = self
            .ingest_classification_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(outcome) = outcome {
            if let Ok(mut state) = self.state.lock() {
                if state.ingest_apply_request_id.as_deref() != Some(outcome.request_id.as_str())
                    || state.ingest_apply_batch_id.as_deref() != outcome.batch_id.as_deref()
                {
                    let expected_request_id = state
                        .ingest_apply_request_id
                        .as_deref()
                        .unwrap_or("<none>")
                        .to_owned();
                    let expected_batch_id = state
                        .ingest_apply_batch_id
                        .as_deref()
                        .unwrap_or("<none>")
                        .to_owned();
                    let request_id = outcome.request_id.clone();
                    let batch_id = outcome
                        .batch_id
                        .clone()
                        .unwrap_or_else(|| "<none>".to_owned());
                    let applied_count = outcome.applied_count;
                    let applied_preview_count = outcome.applied.len();
                    let total_item_count = outcome.total_item_count.unwrap_or(applied_count);
                    let applied_ids =
                        ingest_receipt_applied_item_ids(&outcome.applied, applied_count);
                    let failed_detail = outcome
                        .failed
                        .as_ref()
                        .map(|failed| {
                            format!(
                                "failed_item_id={} failed_row={} failed_error={}",
                                failed.item_id,
                                failed.index + 1,
                                failed.error
                            )
                        })
                        .unwrap_or_else(|| "failed_item_id=<none>".to_owned());
                    state.ingest_last_apply_receipt = format!(
                        "ignored_stale=true request_id={request_id} expected_request_id={expected_request_id} batch_id={batch_id} expected_batch_id={expected_batch_id} requested_by={} applied_count={applied_count} applied_preview_count={applied_preview_count} total_item_count={total_item_count} applied_item_ids={applied_ids} {failed_detail}",
                        outcome.requested_by
                    );
                    state.ingest_status = format!(
                        "Ignored stale ingest classification response request_id={} batch_id={:?}.",
                        outcome.request_id, outcome.batch_id
                    );
                    return;
                }

                state.ingest_apply_pending = false;
                let expected_actor_id = state.ingest_apply_actor_id.clone();
                if let Some(expected_actor_id) = expected_actor_id.as_deref() {
                    if expected_actor_id != outcome.requested_by {
                        let request_id = outcome.request_id.clone();
                        let batch_id = outcome
                            .batch_id
                            .clone()
                            .unwrap_or_else(|| "<none>".to_owned());
                        state.ingest_apply_request_id = None;
                        state.ingest_apply_batch_id = None;
                        state.ingest_apply_actor_id = None;
                        state.ingest_last_apply_receipt = format!(
                            "ignored_actor_mismatch=true request_id={request_id} batch_id={batch_id} expected_requested_by={expected_actor_id} requested_by={}",
                            outcome.requested_by
                        );
                        state.ingest_status = format!(
                            "Ignored ingest classification response with actor mismatch: expected {expected_actor_id}, got {}.",
                            outcome.requested_by
                        );
                        return;
                    }
                }
                state.ingest_apply_request_id = None;
                state.ingest_apply_batch_id = None;
                state.ingest_apply_actor_id = None;

                let request_id = outcome.request_id.clone();
                let batch_id = outcome
                    .batch_id
                    .clone()
                    .unwrap_or_else(|| "<none>".to_owned());
                let applied_count = outcome.applied_count;
                let applied_preview_count = outcome.applied.len();
                let total_item_count = outcome.total_item_count.unwrap_or(applied_count);
                let applied_ids = ingest_receipt_applied_item_ids(&outcome.applied, applied_count);
                for row in &outcome.applied {
                    let decision = IngestDecision::from_lane(&row.item.lane);
                    state
                        .ingest_item_decisions
                        .insert(row.item.item_id.clone(), decision);
                    state
                        .ingest_persisted_item_ids
                        .insert(row.item.item_id.clone());
                }

                if let Some(failed) = outcome.failed {
                    state.ingest_last_apply_receipt = format!(
                        "request_id={request_id} batch_id={batch_id} requested_by={} applied_count={applied_count} applied_preview_count={applied_preview_count} total_item_count={total_item_count} applied_item_ids={applied_ids} failed_item_id={} failed_row={} failed_error={}",
                        outcome.requested_by,
                        failed.item_id,
                        failed.index + 1,
                        failed.error
                    );
                    state.ingest_status = format!(
                        "Persisted {applied_count}/{total_item_count} canonical intake item classification(s); failed item {} at row {}: {}",
                        failed.item_id,
                        failed.index + 1,
                        failed.error
                    );
                } else {
                    let actor_label = outcome.requested_by.clone();
                    state.ingest_last_apply_receipt = format!(
                        "request_id={request_id} batch_id={batch_id} requested_by={actor_label} applied_count={applied_count} applied_preview_count={applied_preview_count} total_item_count={total_item_count} applied_item_ids={applied_ids} failed_item_id=<none>"
                    );
                    state.ingest_status = format!(
                        "Persisted {applied_count}/{total_item_count} canonical intake item classification(s) through backend actor {}.",
                        actor_label
                    );
                }
            }
        }
    }

    fn show_ckc(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        self.ensure_ckc_load_requested();
        self.drain_ckc_backend();
        // MT-010: deliver any fetched linked-media image bytes to the decode cache BEFORE preparing the
        // render so freshly-arrived bytes paint the same frame.
        self.drain_ckc_media_bytes_backend();
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if self.ckc_client.is_none() && state.ckc_characters.is_empty() {
            state.ckc_characters = seeded_ckc_characters();
            state.ckc_selected_index = 0;
        }
        self.show_ckc_mode_controls(ui, palette, &mut state.ckc_book_mode);
        let book_mode = state.ckc_book_mode;
        let selected_index = state
            .ckc_selected_index
            .min(state.ckc_characters.len().saturating_sub(1));
        state.ckc_selected_index = selected_index;
        // MT-010: when the selected linked-media image changed, (re)dispatch the byte fetch or clear to
        // the explicit empty state. First observation of the default/demo selection never fetches.
        self.sync_ckc_media_fetch(&mut state, selected_index);
        let available_width = ui.available_width();
        let has_middle = book_mode.has_middle_panel();
        let page_widths = ckc_book_page_widths(available_width, has_middle);
        let narrow_stacked_book = has_middle && available_width < 700.0;
        let narrow_left_page_max_height =
            narrow_stacked_book.then(|| (ui.available_height() * 0.16).clamp(72.0, 96.0));
        let narrow_middle_page_max_height =
            narrow_stacked_book.then(|| (ui.available_height() * 0.42).clamp(190.0, 250.0));
        let narrow_right_page_max_height =
            narrow_stacked_book.then(|| (ui.available_height() * 0.24).clamp(120.0, 160.0));
        let book_response = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(egui::Id::new(ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID)),
                |ui| {
                    let book_layout = if narrow_stacked_book {
                        egui::Layout::top_down(egui::Align::Min)
                    } else {
                        egui::Layout::left_to_right(egui::Align::Min)
                    };
                    ui.with_layout(book_layout, |ui| {
                        let left_response = ui
                            .scope_builder(
                                egui::UiBuilder::new().id_salt(egui::Id::new(
                                    ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
                                )),
                                |ui| {
                                    constrain_ckc_book_page(ui, page_widths.left);
                                    let mut scroll = egui::ScrollArea::vertical()
                                        .id_salt("atelier-ckc-book-left-media-scroll")
                                        .auto_shrink([false, false]);
                                    if let Some(max_height) = narrow_left_page_max_height {
                                        scroll = scroll.max_height(max_height);
                                    }
                                    scroll.show(ui, |ui| {
                                            constrain_ckc_book_page(ui, page_widths.left);
                                            ui.heading(
                                                egui::RichText::new("Character images")
                                                    .color(palette.text),
                                            );
                if state.ckc_loading {
                    ui.label(egui::RichText::new("Loading CKC database...").color(palette.text_subtle));
                }
                if let Some(error) = &state.ckc_error {
                    // MT-037: surface the CKC backend-error / local-fallback status as an Argus-readable
                    // node. It was previously a human-only red label, so a parallel model could not read
                    // local-only, non-persistent create/save results. The character-create local branch
                    // below sets this to an explicit non-persistence status.
                    let error_label = format!("CKC backend: {error}");
                    let error_response =
                        ui.label(egui::RichText::new(&error_label).color(palette.error_text));
                    emit_node(
                        ui.ctx(),
                        error_response.id,
                        accesskit::Role::Label,
                        ATELIER_CKC_ERROR_AUTHOR_ID,
                        &error_label,
                        false,
                    );
                }
                self.show_ckc_search(ui, palette, &mut state);
                ui.separator();
                let list_response = ui
                    .vertical(|ui| {
                        let mut pending_selection = None;
                        for (idx, character) in state.ckc_characters.iter().enumerate() {
                            let selected = state.ckc_selected_index == idx;
                            let row_label = if character.sheet_seq > 0 {
                                format!("{}  v{}", character.display_name, character.sheet_seq)
                            } else {
                                format!("{}  no sheet", character.display_name)
                            };
                            let row = ui
                                .push_id(
                                    ("ckc-character-row", character.character_internal_id.as_str()),
                                    |ui| ui.add(egui::Button::new(row_label).selected(selected)),
                                )
                                .inner;
                            emit_node(
                                ui.ctx(),
                                row.id,
                                accesskit::Role::Button,
                                &ckc_character_row_author_id(&character.character_internal_id),
                                &format!(
                                    "{} sheet version {}",
                                    character.display_name, character.sheet_seq
                                ),
                                selected,
                            );
                            if row.clicked() {
                                pending_selection = Some(idx);
                            }
                        }
                        if let Some(idx) = pending_selection {
                            if state.ckc_selected_index != idx {
                                state.ckc_selected_index = idx;
                                state.ckc_last_export = None;
                                state.ckc_selected_media_key = None;
                                state.ckc_selected_album_collection_id = None;
                                state.ckc_selected_sheet_artifact_link_id = None;
                                state.ckc_sheet_artifact_reuse_ref.clear();
                            }
                        }
                    })
                    .response;
                emit_node(
                    ui.ctx(),
                    list_response.id,
                    accesskit::Role::List,
                    ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
                    "CKC character database",
                    false,
                );
                ui.separator();
                ui.horizontal(|ui| {
                    let create_name = ui.text_edit_singleline(&mut state.ckc_new_display_name);
                    emit_node(
                        ui.ctx(),
                        create_name.id,
                        accesskit::Role::TextInput,
                        ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
                        "New character display name",
                        false,
                    );
                    let create = ui.button("Create");
                    emit_node(
                        ui.ctx(),
                        create.id,
                        accesskit::Role::Button,
                        ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID,
                        "Create CKC character",
                        state.ckc_create_pending,
                    );
                    if create.clicked() {
                        let display_name = state.ckc_new_display_name.trim().to_owned();
                        if !display_name.is_empty() {
                            let next = state.ckc_characters.len() + 1;
                            let public_id = slugify_public_id(&display_name, next);
                            if let Some(client) = self.ckc_client.as_ref() {
                                if !state.ckc_create_pending {
                                    let expected_thread_id =
                                        ckc_character_create_model_ops_thread_id(&public_id);
                                    match model_ops_mutation_context_from_state(
                                        &state,
                                        client.actor_id(),
                                        &expected_thread_id,
                                    ) {
                                        Ok(model_ops) => {
                                            state.ckc_create_pending = true;
                                            state.ckc_error = None;
                                            client.create_ckc_character(
                                                &public_id,
                                                &display_name,
                                                &model_ops.actor_id,
                                                model_ops.context,
                                                self.ckc_create_cell.clone(),
                                            );
                                        }
                                        Err(err) => {
                                            state.ckc_error =
                                                Some(format!("CKC create blocked: {err}"));
                                        }
                                    }
                                }
                            } else {
                                let character_internal_id = Uuid::new_v4().to_string();
                                state.ckc_characters.push(CkcCharacterRecord {
                                    public_id: public_id.clone(),
                                    display_name: display_name.clone(),
                                    character_internal_id: character_internal_id.clone(),
                                    character_ref: format!("atelier://character/{character_internal_id}"),
                                    sheet_version_id: None,
                                    parent_sheet_version_id: None,
                                    sheet_seq: 0,
                                    sheet_editor_text: format!(
                                        "CHAR-ID-001 \u{2014} Character_ID: {public_id}\nCHAR-ID-002 \u{2014} Name: {display_name}\nCHAR-ID-006 \u{2014} Primary_Role: reusable character/avatar\nPIPELINES\npipelines: ComfyUI, Unreal, Blender\nnotes: "
                                    ),
                                    sheet_version_ref: None,
                                    sheet_artifact_links: Vec::new(),
                                    media_album_offset: 0,
                                    media_album_limit: 0,
                                    media_album_member_limit: 0,
                                    media_album_count: 0,
                                    media_albums_next_offset: None,
                                    media_albums: Vec::new(),
                                    story_documents: Vec::new(),
                                    moodboard_documents: Vec::new(),
                                });
                                state.ckc_selected_index = state.ckc_characters.len() - 1;
                                state.ckc_last_export = None;
                                state.ckc_new_display_name = "New character".to_owned();
                                // MT-037: the local (no-backend) create path was previously SILENT — it
                                // set no status. Surface a non-persistence status through the now
                                // Argus-readable ckc_error node so a model does not mistake a local-only
                                // character for a persisted backend write.
                                state.ckc_error = Some(format!(
                                    "Created local CKC character {display_name}. Backend is unavailable; this is not persisted."
                                ));
                            }
                        }
                    }
                });
                ui.separator();
                let selected_index = state
                    .ckc_selected_index
                    .min(state.ckc_characters.len().saturating_sub(1));
                let media_save_pending = state.ckc_media_save_pending;
                let selected_media_key = state.ckc_selected_media_key.clone();
                let selected_album_collection_id = state.ckc_selected_album_collection_id.clone();
                let album_create_pending = state.ckc_album_create_pending;
                let album_link_pending = state.ckc_album_link_pending;
                let album_page_pending = state.ckc_album_page_pending;
                let album_page_pending_collection_id =
                    state.ckc_album_page_pending_collection_id.clone();
                let album_list_page_pending = state.ckc_album_list_page_pending;
                let album_list_page_pending_character_id =
                    state.ckc_album_list_page_pending_character_id.clone();
                let album_status = state.ckc_album_status.clone();
                let media_image_status = state.ckc_media_image_status.clone();
                let mut album_create_name = std::mem::take(&mut state.ckc_album_create_name);
                let mut album_create_notes = std::mem::take(&mut state.ckc_album_create_notes);
                let mut album_create_tags = std::mem::take(&mut state.ckc_album_create_tags);
                let mut album_link_asset_ids =
                    std::mem::take(&mut state.ckc_album_link_asset_ids);
                let mut album_link_source_path_ref =
                    std::mem::take(&mut state.ckc_album_link_source_path_ref);
                let mut album_link_source_url_ref =
                    std::mem::take(&mut state.ckc_album_link_source_url_ref);
                let mut album_link_edit_source_path_ref =
                    std::mem::take(&mut state.ckc_album_link_edit_source_path_ref);
                let mut album_link_edit_source_url_ref =
                    std::mem::take(&mut state.ckc_album_link_edit_source_url_ref);
                let mut album_link_edit_clear_source_path =
                    state.ckc_album_link_edit_clear_source_path;
                let mut album_link_edit_clear_source_url =
                    state.ckc_album_link_edit_clear_source_url;
                let mut media_actor_id = std::mem::take(&mut state.ckc_media_actor_id);
                let mut pending_media_save = None;
                let mut pending_media_selection = None;
                let mut pending_album_selection = None;
                let mut pending_album_create = None;
                let mut pending_album_link = None;
                let mut pending_album_unlink = None;
                let mut pending_album_link_edit = None;
                let mut pending_album_reorder = None;
                let mut pending_album_page = None;
                let mut pending_album_list_page = None;
                let mut pending_album_status_update = None;
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    let (
                        save,
                        media_selection,
                        album_selection,
                        album_create,
                        album_link,
                        album_unlink,
                        album_link_edit,
                        album_reorder,
                        album_page,
                        album_list_page,
                        album_status_update,
                    ) = self
                        .show_ckc_linked_media(
                        ui,
                        palette,
                        character,
                        media_save_pending,
                        selected_media_key.as_deref(),
                        selected_album_collection_id.as_deref(),
                        album_create_pending,
                        album_link_pending,
                        album_page_pending,
                        album_page_pending_collection_id.as_deref(),
                        album_list_page_pending,
                        album_list_page_pending_character_id.as_deref(),
                        &album_status,
                        &media_image_status,
                        &mut album_create_name,
                        &mut album_create_notes,
                        &mut album_create_tags,
                        &mut album_link_asset_ids,
                        &mut album_link_source_path_ref,
                        &mut album_link_source_url_ref,
                        &mut album_link_edit_source_path_ref,
                        &mut album_link_edit_source_url_ref,
                        &mut album_link_edit_clear_source_path,
                        &mut album_link_edit_clear_source_url,
                        &mut media_actor_id,
                    );
                    pending_media_save = save;
                    pending_media_selection = media_selection;
                    pending_album_selection = album_selection;
                    pending_album_create = album_create;
                    pending_album_link = album_link;
                    pending_album_unlink = album_unlink;
                    pending_album_link_edit = album_link_edit;
                    pending_album_reorder = album_reorder;
                    pending_album_page = album_page;
                    pending_album_list_page = album_list_page;
                    pending_album_status_update = album_status_update;
                }
                state.ckc_album_create_name = album_create_name;
                state.ckc_album_create_notes = album_create_notes;
                state.ckc_album_create_tags = album_create_tags;
                state.ckc_album_link_asset_ids = album_link_asset_ids;
                state.ckc_album_link_source_path_ref = album_link_source_path_ref;
                state.ckc_album_link_source_url_ref = album_link_source_url_ref;
                state.ckc_album_link_edit_source_path_ref = album_link_edit_source_path_ref;
                state.ckc_album_link_edit_source_url_ref = album_link_edit_source_url_ref;
                state.ckc_album_link_edit_clear_source_path = album_link_edit_clear_source_path;
                state.ckc_album_link_edit_clear_source_url = album_link_edit_clear_source_url;
                state.ckc_media_actor_id = media_actor_id;
                if let Some(status) = pending_album_status_update {
                    state.ckc_album_status = status;
                }
                if let Some(media_key) = pending_media_selection {
                    state.ckc_selected_media_key = Some(media_key);
                }
                if let Some(collection_id) = pending_album_selection {
                    state.ckc_selected_album_collection_id = Some(collection_id);
                }
                if let Some(request) = pending_album_create {
                    if let Some(client) = self.ckc_client.as_ref() {
                        let expected_thread_id =
                            ckc_character_model_ops_thread_id(&request.character_internal_id);
                        match model_ops_mutation_context_from_state(
                            &state,
                            &request.actor_id,
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => {
                                state.ckc_album_create_pending = true;
                                state.ckc_album_status =
                                    format!("Creating CKC album {}", request.name);
                                state.ckc_error = None;
                                client.create_ckc_media_album(
                                    &request.character_internal_id,
                                    &request.name,
                                    request.notes.as_deref(),
                                    request.sheet_version_id.as_deref(),
                                    &request.tags,
                                    &model_ops.actor_id,
                                    model_ops.context,
                                    self.ckc_media_album_create_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.ckc_album_status =
                                    format!("CKC album create blocked: {err}");
                                state.ckc_error = Some(state.ckc_album_status.clone());
                            }
                        }
                    }
                }
                if let Some(request) = pending_album_link {
                    if let Some(client) = self.ckc_client.as_ref() {
                        let expected_thread_id =
                            ckc_media_album_model_ops_thread_id(&request.collection_id);
                        match model_ops_mutation_context_from_state(
                            &state,
                            &request.actor_id,
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => {
                                state.ckc_album_link_pending = true;
                                state.ckc_album_status = format!(
                                    "Linking {} media asset(s) into CKC album {}",
                                    request.asset_ids.len(),
                                    request.collection_id
                                );
                                state.ckc_error = None;
                                client.add_ckc_media_album_items(
                                    &request.collection_id,
                                    &request.asset_ids,
                                    request.source_path_ref.as_deref(),
                                    request.source_url_ref.as_deref(),
                                    &model_ops.actor_id,
                                    model_ops.context,
                                    self.ckc_media_album_items_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.ckc_album_status = format!("CKC album link blocked: {err}");
                                state.ckc_error = Some(state.ckc_album_status.clone());
                            }
                        }
                    }
                }
                if let Some(request) = pending_album_unlink {
                    if let Some(client) = self.ckc_client.as_ref() {
                        let expected_thread_id =
                            ckc_media_album_model_ops_thread_id(&request.collection_id);
                        match model_ops_mutation_context_from_state(
                            &state,
                            &request.actor_id,
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => {
                                state.ckc_album_link_pending = true;
                                state.ckc_album_status = format!(
                                    "Unlinking media {} from album {}; non-destructive album membership removal only; actor={}.",
                                    request.asset_id, request.collection_id, model_ops.actor_id
                                );
                                state.ckc_error = None;
                                client.unlink_ckc_media_album_item(
                                    &request.collection_id,
                                    &request.asset_id,
                                    &model_ops.actor_id,
                                    model_ops.context,
                                    self.ckc_media_album_items_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.ckc_album_status =
                                    format!("CKC album unlink blocked: {err}");
                                state.ckc_error = Some(state.ckc_album_status.clone());
                            }
                        }
                    }
                }
                if let Some(request) = pending_album_link_edit {
                    if let Some(client) = self.ckc_client.as_ref() {
                        let expected_thread_id =
                            ckc_media_album_model_ops_thread_id(&request.collection_id);
                        match model_ops_mutation_context_from_state(
                            &state,
                            &request.actor_id,
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => {
                                state.ckc_album_link_pending = true;
                                state.ckc_album_status = format!(
                                    "Updating album link provenance for media {} in album {}; asset-level provenance will remain unchanged; actor={}.",
                                    request.asset_id, request.collection_id, model_ops.actor_id
                                );
                                state.ckc_error = None;
                                client.update_ckc_media_album_item_link(
                                    &request.collection_id,
                                    &request.asset_id,
                                    request.source_path_ref.as_deref(),
                                    request.source_url_ref.as_deref(),
                                    request.clear_source_path_ref,
                                    request.clear_source_url_ref,
                                    &model_ops.actor_id,
                                    model_ops.context,
                                    self.ckc_media_album_items_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.ckc_album_status =
                                    format!("CKC album link provenance edit blocked: {err}");
                                state.ckc_error = Some(state.ckc_album_status.clone());
                            }
                        }
                    }
                }
                if let Some(request) = pending_album_reorder {
                    if let Some(client) = self.ckc_client.as_ref() {
                        let expected_thread_id =
                            ckc_media_album_model_ops_thread_id(&request.collection_id);
                        match model_ops_mutation_context_from_state(
                            &state,
                            &request.actor_id,
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => {
                                state.ckc_album_link_pending = true;
                                state.ckc_album_status = format!(
                                    "Reordering {} media item(s) in CKC album {}; actor={}.",
                                    request.items.len(),
                                    request.collection_id,
                                    model_ops.actor_id
                                );
                                state.ckc_error = None;
                                client.reorder_ckc_media_album_items(
                                    &request.collection_id,
                                    &request.items,
                                    &model_ops.actor_id,
                                    model_ops.context,
                                    self.ckc_media_album_items_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.ckc_album_status =
                                    format!("CKC album reorder blocked: {err}");
                                state.ckc_error = Some(state.ckc_album_status.clone());
                            }
                        }
                    }
                }
                if let Some(request) = pending_album_page {
                    if let Some(client) = self.ckc_client.as_ref() {
                        state.ckc_album_page_pending = true;
                        state.ckc_album_page_pending_collection_id =
                            Some(request.collection_id.clone());
                        state.ckc_album_status = format!(
                            "Loading CKC media album {} from offset {}",
                            request.collection_id, request.offset
                        );
                        state.ckc_error = None;
                        client.fetch_ckc_media_album_items(
                            &request.collection_id,
                            request.offset,
                            200,
                            self.ckc_media_album_page_cell.clone(),
                        );
                    }
                }
                if let Some(request) = pending_album_list_page {
                    if let Some(client) = self.ckc_client.as_ref() {
                        state.ckc_album_list_page_pending = true;
                        state.ckc_album_list_page_pending_character_id =
                            Some(request.character_internal_id.clone());
                        state.ckc_album_status = format!(
                            "Loading CKC album list for character {} from offset {}",
                            request.character_internal_id, request.offset
                        );
                        state.ckc_error = None;
                        client.fetch_ckc_media_album_list_page(
                            &request.character_internal_id,
                            request.offset,
                            self.ckc_media_album_list_page_cell.clone(),
                        );
                    }
                }
                if let Some(request) = pending_media_save {
                    if let Some(client) = self.ckc_client.as_ref() {
                        let expected_thread_id =
                            format!("atelier.media.asset.{}.notes-tags", request.asset_id);
                        match model_ops_mutation_context_from_state(
                            &state,
                            &request.actor_id,
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => {
                                state.ckc_media_save_pending = true;
                                state.ckc_error = None;
                                client.save_ckc_media_notes_tags(
                                    &request.asset_id,
                                    Some(&request.notes),
                                    Some(&request.tags),
                                    request.review_status.as_deref(),
                                    &model_ops.actor_id,
                                    model_ops.context,
                                    self.ckc_media_notes_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.ckc_media_image_status =
                                    format!("CKC media notes blocked: {err}");
                                state.ckc_error = Some(state.ckc_media_image_status.clone());
                            }
                        }
                    }
                }
                ui.separator();
                if let Ok(mut side_panel) = self.side_panel.lock() {
                    side_panel.show(ui, palette);
                }
                                        });
                                },
                            )
                            .response;
                        emit_node(
                            ui.ctx(),
                            left_response.id,
                            accesskit::Role::Group,
                            ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
                            "CKC left page: character images albums media notes and source refs",
                            false,
                        );
                        ui.separator();
                        let selected_index = state
                            .ckc_selected_index
                            .min(state.ckc_characters.len().saturating_sub(1));
                        state.ckc_selected_index = selected_index;
                        if has_middle {
                            let middle_response = ui
                                .scope_builder(
                                    egui::UiBuilder::new()
                                        .id_salt(egui::Id::new(ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID)),
                                    |ui| {
                                        constrain_ckc_book_page(ui, page_widths.middle);
                                        let mut scroll = egui::ScrollArea::vertical()
                                            .id_salt("atelier-ckc-book-middle-scroll")
                                            .auto_shrink([false, false]);
                                        if let Some(max_height) = narrow_middle_page_max_height {
                                            scroll = scroll.max_height(max_height);
                                        }
                                        scroll.show(ui, |ui| {
                                                constrain_ckc_book_page(ui, page_widths.middle);
                                                self.show_ckc_middle_work_panel(
                                                    ui,
                                                    palette,
                                                    &mut state,
                                                    selected_index,
                                                );
                                            });
                                    },
                                )
                                .response;
                            emit_node(
                                ui.ctx(),
                                middle_response.id,
                                accesskit::Role::Group,
                                ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
                                book_mode.middle_label(),
                                false,
                            );
                            ui.separator();
                        }
                        let right_response = ui
                            .scope_builder(
                                egui::UiBuilder::new().id_salt(egui::Id::new(
                                    ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
                                )),
                                |ui| {
                                    constrain_ckc_book_page(ui, page_widths.right);
                                    let mut scroll = egui::ScrollArea::vertical()
                                        .id_salt("atelier-ckc-book-right-sheet-scroll")
                                        .auto_shrink([false, false]);
                                    if let Some(max_height) = narrow_right_page_max_height {
                                        scroll = scroll.max_height(max_height);
                                    }
                                    scroll.show(ui, |ui| {
                                            constrain_ckc_book_page(ui, page_widths.right);
                                            self.show_ckc_character_sheet_panel(
                                                ui,
                                                palette,
                                                &mut state,
                                                selected_index,
                                            );
                                            ui.separator();
                                            self.show_ckc_sheet_tools(
                                                ui,
                                                palette,
                                                &mut state,
                                                selected_index,
                                            );
                                            ui.separator();
                                            self.show_ckc_sheet_artifact_panel(
                                                ui,
                                                palette,
                                                &mut state,
                                                selected_index,
                                            );
                                        });
                                },
                            )
                            .response;
                        emit_node(
                            ui.ctx(),
                            right_response.id,
                            accesskit::Role::Group,
                            ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
                            "CKC right page: editable character sheet and sheet tools",
                            false,
                        );
                    });
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            book_response.id,
            accesskit::Role::Group,
            ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
            if book_mode.has_middle_panel() && available_width < 700.0 {
                "CKC book layout with stacked readable three-page pages left media middle work surface and right sheet"
            } else if book_mode.has_middle_panel() {
                "CKC book layout with left media middle work surface and right sheet"
            } else {
                "CKC book layout with left media and right sheet"
            },
            false,
        );
    }

    fn show_ckc_mode_controls(
        &self,
        ui: &mut egui::Ui,
        _palette: &HsPalette,
        mode: &mut CkcBookMode,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label("CKC book mode:");
            for next_mode in CkcBookMode::ALL {
                let selected = *mode == next_mode;
                let response = ui.add(egui::Button::selectable(selected, next_mode.label()));
                emit_node(
                    ui.ctx(),
                    response.id,
                    accesskit::Role::Button,
                    next_mode.author_id(),
                    next_mode.middle_label(),
                    selected,
                );
                if response.clicked() {
                    *mode = next_mode;
                }
            }
        });
    }

    fn show_ckc_character_sheet_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        let append_pending = state.ckc_append_pending;
        let mut pending_append_request: Option<(String, String, Option<String>)> = None;
        let mut clear_last_export = false;
        // MT-037 (F1): read the current sheet status once (disjoint from the character borrow below); the
        // local save branch collects a new status into pending_sheet_status and it is applied after.
        let sheet_status = state.ckc_sheet_status.clone();
        let mut pending_sheet_status: Option<String> = None;
        if let Some(character) = state.ckc_characters.get_mut(selected_index) {
            let selected_response = ui
                .vertical(|ui| {
                    ui.heading(egui::RichText::new(&character.display_name).color(palette.text));
                    ui.label(format!("public_id: {}", character.public_id));
                    if character.sheet_seq > 0 {
                        ui.label(format!("sheet seq: {}", character.sheet_seq));
                    } else {
                        ui.label("sheet seq: no sheet version yet");
                    }
                    if let Some(parent) = &character.parent_sheet_version_id {
                        ui.label(format!("parent_version_id: {parent}"));
                    }
                })
                .response;
            emit_node(
                ui.ctx(),
                selected_response.id,
                accesskit::Role::Group,
                ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
                &format!(
                    "{} current sheet version {}",
                    character.display_name, character.sheet_seq
                ),
                true,
            );
            ui.add_space(4.0);
            let character_ref = character.character_ref();
            let character_ref_response = ui.label(format!("character_ref: {character_ref}"));
            emit_node(
                ui.ctx(),
                character_ref_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
                &character_ref,
                false,
            );
            let sheet_ref = character.sheet_atelier_ref();
            if let Some(sheet_ref) = &sheet_ref {
                debug_assert_eq!(sheet_ref.item_kind, AtelierItemKind::CharacterSheet);
            }
            let ref_kind = sheet_ref
                .as_ref()
                .map(|sheet_ref| sheet_ref.ref_kind())
                .unwrap_or("character_sheet");
            let ref_kind_response = ui.label(format!("hsLink refKind: {ref_kind}"));
            emit_node(
                ui.ctx(),
                ref_kind_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID,
                ref_kind,
                false,
            );
            let sheet_version_ref = character
                .sheet_version_ref()
                .unwrap_or_else(|| "pending-first-sheet-version".to_owned());
            let sheet_ref_response = ui.label(format!("sheet_version_ref: {sheet_version_ref}"));
            emit_node(
                ui.ctx(),
                sheet_ref_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
                &sheet_version_ref,
                false,
            );
            // MT-037 (F1): sheet-version persistence status + backend-mode receipt. Without a backend the
            // local "Append sheet version" mints a sheet_version_ref that otherwise looks identical to a
            // persisted backend version — surface that it is a local-preview, not-persisted write.
            let sheet_status_response =
                ui.label(egui::RichText::new(sheet_status.as_str()).color(palette.text_subtle));
            emit_node(
                ui.ctx(),
                sheet_status_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_SHEET_STATUS_AUTHOR_ID,
                sheet_status.as_str(),
                false,
            );
            let sheet_backend_mode = if self.ckc_client.is_some() {
                "backend:persistent"
            } else {
                "local-preview:not-persisted"
            };
            let sheet_backend_mode_response = ui.label(
                egui::RichText::new(format!("sheet mode: {sheet_backend_mode}"))
                    .color(palette.text_subtle),
            );
            emit_value_node(
                ui.ctx(),
                sheet_backend_mode_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_SHEET_BACKEND_MODE_AUTHOR_ID,
                "CKC sheet persistence mode",
                sheet_backend_mode,
            );
            ui.add_space(8.0);
            let editor = ui.add(
                egui::TextEdit::multiline(&mut character.sheet_editor_text)
                    .desired_rows(15)
                    .lock_focus(true),
            );
            emit_node(
                ui.ctx(),
                editor.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
                "CKC character sheet editor",
                false,
            );
            let save = ui.button("Append sheet version");
            emit_node(
                ui.ctx(),
                save.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
                "Append CKC sheet version",
                append_pending,
            );
            if save.clicked() {
                if self.ckc_client.is_some() {
                    if !append_pending {
                        pending_append_request = Some((
                            character.character_internal_id.clone(),
                            character.sheet_editor_text.clone(),
                            character.sheet_version_id.clone(),
                        ));
                    }
                } else {
                    character.parent_sheet_version_id = character.sheet_version_id.clone();
                    let next_sheet_version_id = Uuid::new_v4().to_string();
                    character.sheet_version_id = Some(next_sheet_version_id.clone());
                    character.sheet_seq += 1;
                    character.sheet_version_ref = Some(format!(
                        "atelier://sheet/{}/{}",
                        character.character_internal_id, next_sheet_version_id
                    ));
                    clear_last_export = true;
                    // MT-037 (F1): the local sheet-version save is NOT a backend write; surface it so the
                    // minted sheet_version_ref is not mistaken for a persisted backend version.
                    pending_sheet_status = Some(format!(
                        "Saved local CKC sheet version seq {} ({}). Backend is unavailable; this is not persisted.",
                        character.sheet_seq,
                        character
                            .sheet_version_ref
                            .as_deref()
                            .unwrap_or("pending-first-sheet-version")
                    ));
                }
            }
        } else {
            ui.label(egui::RichText::new("No CKC characters yet").color(palette.text_subtle));
        }
        if clear_last_export {
            state.ckc_last_export = None;
        }
        // MT-037 (F1): apply the local sheet-version save status after the character borrow ends.
        if let Some(status) = pending_sheet_status {
            state.ckc_sheet_status = status;
        }
        if let Some((character_internal_id, raw_text, expected_parent_version_id)) =
            pending_append_request
        {
            if let Some(client) = self.ckc_client.as_ref() {
                let expected_thread_id = ckc_character_model_ops_thread_id(&character_internal_id);
                match model_ops_mutation_context_from_state(
                    state,
                    client.actor_id(),
                    &expected_thread_id,
                ) {
                    Ok(model_ops) => {
                        state.ckc_append_pending = true;
                        state.ckc_error = None;
                        client.append_ckc_sheet_version(
                            &character_internal_id,
                            &raw_text,
                            expected_parent_version_id.as_deref(),
                            Some("handshake-native-atelier"),
                            &model_ops.actor_id,
                            model_ops.context,
                            self.ckc_append_cell.clone(),
                        );
                    }
                    Err(err) => {
                        state.ckc_sheet_status = format!("CKC sheet append blocked: {err}");
                        state.ckc_error = Some(state.ckc_sheet_status.clone());
                    }
                }
            }
        }
    }

    fn show_ckc_character_notes_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        notes_buffer: &mut String,
        notes_source_key: &mut Option<String>,
        notes_status: &mut String,
    ) {
        let source_key = ckc_sheet_notes_source_key(character);
        if notes_source_key.as_deref() != Some(source_key.as_str()) {
            *notes_buffer = extract_ckc_sheet_notes(&character.sheet_editor_text);
            *notes_source_key = Some(source_key);
        }
        ui.heading(egui::RichText::new("Character sheet notes").color(palette.text));
        ui.label(egui::RichText::new(notes_status.as_str()).color(palette.text_subtle));
        ui.label(
            egui::RichText::new(
                "Image notes stay in the left media panel; this edits sheet notes.",
            )
            .color(palette.text_subtle),
        );
        let notes = ui.add(
            egui::TextEdit::multiline(notes_buffer)
                .desired_rows(8)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            notes.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
            "CKC character sheet notes editor",
            false,
        );
        let apply = ui.button("Apply notes to sheet");
        emit_node(
            ui.ctx(),
            apply.id,
            accesskit::Role::Button,
            ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID,
            "Apply CKC character notes to the selected sheet text",
            false,
        );
        if apply.clicked() {
            upsert_ckc_sheet_notes(&mut character.sheet_editor_text, notes_buffer);
            *notes_status = format!(
                "Applied character sheet notes to {}. Append the sheet version to persist.",
                character
                    .sheet_version_ref()
                    .unwrap_or_else(|| "pending sheet".to_owned())
            );
        }
    }

    fn show_ckc_moodboard_canvas(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        ui.separator();
        ui.heading(egui::RichText::new("Moodboard").color(palette.text));
        ui.add_space(4.0);
        let canvas_response = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt(egui::Id::new(ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID)),
                |ui| {
                    // WP-CKC MT-012 (FIX 2): render the moodboard's OWN isolated board. Its interaction
                    // events are intentionally DROPPED here rather than pushed into the shared
                    // `canvas_events` queue that `app.rs::route_canvas_events` PATCHes onto the workspace
                    // `default-canvas` block. `LoomCanvasBoard::show` mutates placement geometry IN PLACE
                    // (drag/resize update `card.x/y/w/h` directly), so dropping the events loses no visual
                    // state; the ONLY persistence path for the moodboard is the snapshot reverse-projection
                    // performed by "Save moodboard" ([`board_to_ckc_moodboard_json`]). `drain_knowledge_events`
                    // is still called to clear the board's per-frame swarm-dispatch queue.
                    if let Ok(mut board) = self.moodboard_board.lock() {
                        let _ = board.show(ui, palette);
                        let _ = board.drain_knowledge_events();
                    }
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            canvas_response.id,
            accesskit::Role::Group,
            ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
            "Native CKC moodboard canvas surface",
            false,
        );
    }

    fn show_ckc_middle_work_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        match state.ckc_book_mode {
            CkcBookMode::Sheet => {}
            CkcBookMode::Notes => {
                let mut notes_buffer = std::mem::take(&mut state.ckc_character_notes_buffer);
                let mut notes_source_key =
                    std::mem::take(&mut state.ckc_character_notes_source_key);
                let mut notes_status = std::mem::take(&mut state.ckc_character_notes_status);
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    self.show_ckc_character_notes_panel(
                        ui,
                        palette,
                        character,
                        &mut notes_buffer,
                        &mut notes_source_key,
                        &mut notes_status,
                    );
                } else {
                    ui.label(
                        egui::RichText::new("No CKC character selected for notes.")
                            .color(palette.text_subtle),
                    );
                }
                state.ckc_character_notes_buffer = notes_buffer;
                state.ckc_character_notes_source_key = notes_source_key;
                state.ckc_character_notes_status = notes_status;
            }
            CkcBookMode::Story | CkcBookMode::Moodboard => {
                let mode = state.ckc_book_mode;
                let mut story_card_title = std::mem::take(&mut state.ckc_story_card_title);
                let mut story_card_body = std::mem::take(&mut state.ckc_story_card_body);
                let mut story_beat_text = std::mem::take(&mut state.ckc_story_beat_text);
                let mut story_status = std::mem::take(&mut state.ckc_story_status);
                let mut moodboard_status = std::mem::take(&mut state.ckc_moodboard_status);
                let mut active_story_document_id =
                    std::mem::take(&mut state.ckc_active_story_document_id);
                let mut active_moodboard_document_id =
                    std::mem::take(&mut state.ckc_active_moodboard_document_id);
                let model_ops_fields = ModelOpsUiFields::from_state(state);
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    self.show_ckc_story_and_moodboard(
                        ui,
                        palette,
                        character,
                        &mut story_card_title,
                        &mut story_card_body,
                        &mut story_beat_text,
                        &mut story_status,
                        &mut moodboard_status,
                        &mut active_story_document_id,
                        &mut active_moodboard_document_id,
                        &model_ops_fields,
                        mode,
                    );
                    if mode == CkcBookMode::Moodboard {
                        self.show_ckc_moodboard_canvas(ui, palette);
                    }
                } else {
                    ui.label(
                        egui::RichText::new("No CKC character selected for this work surface.")
                            .color(palette.text_subtle),
                    );
                }
                state.ckc_story_card_title = story_card_title;
                state.ckc_story_card_body = story_card_body;
                state.ckc_story_beat_text = story_beat_text;
                state.ckc_story_status = story_status;
                state.ckc_moodboard_status = moodboard_status;
                state.ckc_active_story_document_id = active_story_document_id;
                state.ckc_active_moodboard_document_id = active_moodboard_document_id;
            }
        }
    }

    fn show_ckc_story_and_moodboard(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        story_card_title: &mut String,
        story_card_body: &mut String,
        story_beat_text: &mut String,
        story_status: &mut String,
        moodboard_status: &mut String,
        active_story_document_id: &mut Option<String>,
        active_moodboard_document_id: &mut Option<String>,
        model_ops_fields: &ModelOpsUiFields,
        mode: CkcBookMode,
    ) {
        if character.story_documents.is_empty() {
            character.story_documents.push(pending_ckc_story_document(
                &character.character_internal_id,
                &character.display_name,
            ));
        }
        if character.moodboard_documents.is_empty() {
            character
                .moodboard_documents
                .push(pending_ckc_moodboard_document(
                    &character.character_internal_id,
                    &character.display_name,
                ));
        }
        let mut active_story_idx = character
            .story_documents
            .iter()
            .position(|story| {
                active_story_document_id.as_deref() == Some(story.document_id.as_str())
            })
            .unwrap_or(0);
        *active_story_document_id = character
            .story_documents
            .get(active_story_idx)
            .map(|story| story.document_id.clone());
        let mut active_moodboard_idx = character
            .moodboard_documents
            .iter()
            .position(|moodboard| {
                active_moodboard_document_id.as_deref() == Some(moodboard.document_id.as_str())
            })
            .unwrap_or(0);
        *active_moodboard_document_id = character
            .moodboard_documents
            .get(active_moodboard_idx)
            .map(|moodboard| moodboard.document_id.clone());
        let character_internal_id = character.character_internal_id.clone();
        let display_name = character.display_name.clone();

        if mode == CkcBookMode::Story {
            ui.heading(egui::RichText::new("Story").color(palette.text));
            // MT-037: emit the story status as an Argus-readable node (was a plain label with no author_id)
            // so a model can read local-only, non-persistent story results.
            let story_status_response =
                ui.label(egui::RichText::new(story_status.as_str()).color(palette.text_subtle));
            emit_node(
                ui.ctx(),
                story_status_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_STORY_STATUS_AUTHOR_ID,
                story_status.as_str(),
                false,
            );
            // MT-037: per-surface persistence receipt mirroring the media backend_mode value node
            // (:atelier-ckc-media-backend-mode). Reads backend:persistent vs local-preview:not-persisted.
            let story_backend_mode = if self.ckc_client.is_some() {
                "backend:persistent"
            } else {
                "local-preview:not-persisted"
            };
            let story_backend_mode_response = ui.label(
                egui::RichText::new(format!("story mode: {story_backend_mode}"))
                    .color(palette.text_subtle),
            );
            emit_value_node(
                ui.ctx(),
                story_backend_mode_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_STORY_BACKEND_MODE_AUTHOR_ID,
                "CKC story persistence mode",
                story_backend_mode,
            );

            {
                let story = character
                    .story_documents
                    .get_mut(active_story_idx)
                    .expect("story placeholder inserted before rendering");
                let story_ref_label = format!("story_document_ref: {}", story.document_ref);
                let story_ref_response = ui.label(&story_ref_label);
                emit_node(
                    ui.ctx(),
                    story_ref_response.id,
                    accesskit::Role::Label,
                    ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
                    &story_ref_label,
                    false,
                );
                let story_editor = ui.add(
                    egui::TextEdit::multiline(&mut story.body_raw_text)
                        .desired_rows(2)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    story_editor.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
                    "CKC story document editor",
                    false,
                );
                let story_save = ui.button("Save story note");
                emit_node(
                    ui.ctx(),
                    story_save.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
                    "Save CKC story document draft",
                    false,
                );
                if story_save.clicked() {
                    let tags = story.tags.clone();
                    if let Some(client) = self.ckc_client.as_ref() {
                        if is_pending_ckc_document_id(&story.document_id) {
                            let expected_thread_id =
                                ckc_character_model_ops_thread_id(&character_internal_id);
                            match model_ops_fields
                                .mutation_context(client.actor_id(), &expected_thread_id)
                            {
                                Ok(model_ops) => {
                                    *story_status =
                                        format!("Creating CKC story document for {display_name}");
                                    client.create_ckc_character_document(
                                        &character_internal_id,
                                        "story",
                                        &story.title,
                                        &story.body_raw_text,
                                        &tags,
                                        &model_ops.actor_id,
                                        model_ops.context,
                                        self.ckc_character_document_cell.clone(),
                                    );
                                }
                                Err(err) => {
                                    *story_status =
                                        format!("CKC story document create blocked: {err}");
                                }
                            }
                        } else {
                            let expected_parent_version_id = story.current_version_id.as_deref();
                            let expected_thread_id =
                                ckc_document_model_ops_thread_id(&story.document_id);
                            match model_ops_fields
                                .mutation_context(client.actor_id(), &expected_thread_id)
                            {
                                Ok(model_ops) => {
                                    *story_status = format!(
                                        "Appending CKC story document {}",
                                        story.document_ref
                                    );
                                    client.append_ckc_character_document_version(
                                        &story.document_id,
                                        &story.title,
                                        &story.body_raw_text,
                                        &tags,
                                        expected_parent_version_id,
                                        &model_ops.actor_id,
                                        model_ops.context,
                                        self.ckc_character_document_cell.clone(),
                                    );
                                }
                                Err(err) => {
                                    *story_status =
                                        format!("CKC story document append blocked: {err}");
                                }
                            }
                        }
                    } else {
                        if is_pending_ckc_document_id(&story.document_id) {
                            let next_document_id = Uuid::new_v4().to_string();
                            story.document_id = next_document_id.clone();
                            story.document_ref = format!("atelier://document/{next_document_id}");
                            story.current_version_id = Some(Uuid::new_v4().to_string());
                            story.current_version_seq = 1;
                        }
                        *story_status = format!(
                            "Saved local CKC story document draft {} separate from sheet/image/tag notes. Backend is unavailable; this is not persisted.",
                            story.document_ref
                        );
                    }
                }
            }

            let _story_doc_list = ui
                .vertical(|ui| {
                    for (idx, story) in character.story_documents.iter().enumerate() {
                        let selected = idx == active_story_idx;
                        let marker = if selected { "active" } else { "linked" };
                        let row_label = format!(
                            "{marker} story document: {} [{}]",
                            story.title, story.document_ref
                        );
                        let row = ui.selectable_label(selected, &row_label);
                        if row.clicked() {
                            active_story_idx = idx;
                            *active_story_document_id = Some(story.document_id.clone());
                            *story_status =
                                format!("Selected CKC story document {}", story.document_ref);
                        }
                        let author_id = ckc_story_document_row_author_id(&story.document_id);
                        emit_node(
                            ui.ctx(),
                            row.id,
                            accesskit::Role::ListItem,
                            &author_id,
                            &row_label,
                            selected,
                        );
                    }
                })
                .response;

            {
                let story = character
                    .story_documents
                    .get_mut(active_story_idx)
                    .expect("story placeholder inserted before rendering");
                if !story.tags.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("story tags: {}", story.tags.join(", ")))
                            .color(palette.text_subtle),
                    );
                }

                let story_card_list = ui
                    .vertical(|ui| {
                        if story.cards.is_empty() {
                            ui.label(
                                egui::RichText::new("No story cards yet.")
                                    .color(palette.text_subtle),
                            );
                        }
                        for card in &story.cards {
                            let card_label = format!(
                                "{} [{}] document:{} ref:{}",
                                card.title,
                                card.card_ref,
                                card.story_document_id,
                                card.story_document_ref
                            );
                            let card_row = ui.label(&card_label);
                            let card_author_id =
                                ckc_story_card_row_author_id(&story.document_id, &card.card_id);
                            emit_node(
                                ui.ctx(),
                                card_row.id,
                                accesskit::Role::ListItem,
                                &card_author_id,
                                &format!(
                                    "atelier-ref story_card:{} story_document:{}",
                                    card.card_ref, card.story_document_ref
                                ),
                                false,
                            );
                            if !card.body_raw_text.is_empty() {
                                ui.label(
                                    egui::RichText::new(card.body_raw_text.clone())
                                        .color(palette.text_subtle),
                                );
                            }
                        }
                        for beat in &story.beats {
                            let card = beat.card_id.as_deref().unwrap_or("unscoped");
                            let beat_label = format!(
                                "beat {} {} document:{} ref:{} [{}]: {}",
                                beat.beat_id,
                                beat.beat_ref,
                                beat.story_document_id,
                                beat.story_document_ref,
                                card,
                                beat.beat_text
                            );
                            let beat_row = ui
                                .label(egui::RichText::new(&beat_label).color(palette.text_subtle));
                            let beat_author_id =
                                ckc_story_beat_row_author_id(&story.document_id, &beat.beat_id);
                            emit_node(
                                ui.ctx(),
                                beat_row.id,
                                accesskit::Role::ListItem,
                                &beat_author_id,
                                &format!(
                                    "atelier-ref story_beat:{} story_document:{}",
                                    beat.beat_ref, beat.story_document_ref
                                ),
                                false,
                            );
                        }
                    })
                    .response;
                emit_node(
                    ui.ctx(),
                    story_card_list.id,
                    accesskit::Role::List,
                    ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID,
                    "CKC reusable story cards and beats",
                    false,
                );

                ui.horizontal_wrapped(|ui| {
                    let title = ui.text_edit_singleline(story_card_title);
                    emit_node(
                        ui.ctx(),
                        title.id,
                        accesskit::Role::TextInput,
                        ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID,
                        "CKC story card title",
                        false,
                    );
                    let save_card = ui.button("Add card");
                    emit_node(
                        ui.ctx(),
                        save_card.id,
                        accesskit::Role::Button,
                        ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID,
                        "Add CKC story card",
                        false,
                    );
                    if save_card.clicked() {
                        let title = story_card_title.trim().to_owned();
                        if !title.is_empty() {
                            let body = story_card_body.trim().to_owned();
                            let tags = vec!["story".to_owned()];
                            if let Some(client) = self.ckc_client.as_ref() {
                                if is_pending_ckc_document_id(&story.document_id) {
                                    *story_status =
                                        "Save the CKC story document before adding cards."
                                            .to_owned();
                                } else {
                                    let expected_thread_id =
                                        ckc_document_model_ops_thread_id(&story.document_id);
                                    match model_ops_fields
                                        .mutation_context(client.actor_id(), &expected_thread_id)
                                    {
                                        Ok(model_ops) => {
                                            *story_status = format!(
                                                "Adding CKC story card {title} under {}",
                                                story.document_ref
                                            );
                                            client.add_ckc_story_card(
                                                &story.document_id,
                                                &title,
                                                &body,
                                                &tags,
                                                &model_ops.actor_id,
                                                model_ops.context,
                                                self.ckc_story_card_cell.clone(),
                                            );
                                        }
                                        Err(err) => {
                                            *story_status =
                                                format!("CKC story card blocked: {err}");
                                        }
                                    }
                                }
                            } else {
                                if is_pending_ckc_document_id(&story.document_id) {
                                    let next_document_id = Uuid::new_v4().to_string();
                                    story.document_id = next_document_id.clone();
                                    story.document_ref =
                                        format!("atelier://document/{next_document_id}");
                                }
                                let card_id = Uuid::new_v4().to_string();
                                story.cards.push(CkcStoryCardRecord {
                                    card_id: card_id.clone(),
                                    card_ref: format!("atelier://story-card/{card_id}"),
                                    story_document_id: story.document_id.clone(),
                                    story_document_ref: story.document_ref.clone(),
                                    title: title.clone(),
                                    body_raw_text: body,
                                    tags,
                                });
                                *story_status = format!(
                                    "Added local CKC story card {title} under {}. Backend is unavailable; this is not persisted.",
                                    story.document_ref
                                );
                            }
                        }
                    }
                });
                let card_body = ui.add(
                    egui::TextEdit::multiline(story_card_body)
                        .desired_rows(3)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    card_body.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID,
                    "CKC story card body",
                    false,
                );
                let beat_editor = ui.add(
                    egui::TextEdit::multiline(story_beat_text)
                        .desired_rows(2)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    beat_editor.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID,
                    "CKC story beat editor",
                    false,
                );
                let save_beat = ui.button("Add beat");
                emit_node(
                    ui.ctx(),
                    save_beat.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID,
                    "Add CKC story beat",
                    false,
                );
                if save_beat.clicked() {
                    let beat_text = story_beat_text.trim().to_owned();
                    if !beat_text.is_empty() {
                        let card_id = story.cards.first().map(|card| card.card_id.clone());
                        if let Some(client) = self.ckc_client.as_ref() {
                            if is_pending_ckc_document_id(&story.document_id) {
                                *story_status =
                                    "Save the CKC story document before adding beats.".to_owned();
                            } else {
                                let expected_thread_id =
                                    ckc_document_model_ops_thread_id(&story.document_id);
                                match model_ops_fields
                                    .mutation_context(client.actor_id(), &expected_thread_id)
                                {
                                    Ok(model_ops) => {
                                        *story_status = format!(
                                            "Adding CKC story beat under {}",
                                            story.document_ref
                                        );
                                        client.add_ckc_story_beat(
                                            &story.document_id,
                                            card_id.as_deref(),
                                            &beat_text,
                                            &model_ops.actor_id,
                                            model_ops.context,
                                            self.ckc_story_beat_cell.clone(),
                                        );
                                    }
                                    Err(err) => {
                                        *story_status = format!("CKC story beat blocked: {err}");
                                    }
                                }
                            }
                        } else {
                            if is_pending_ckc_document_id(&story.document_id) {
                                let next_document_id = Uuid::new_v4().to_string();
                                story.document_id = next_document_id.clone();
                                story.document_ref =
                                    format!("atelier://document/{next_document_id}");
                            }
                            let beat_id = Uuid::new_v4().to_string();
                            story.beats.push(CkcStoryBeatRecord {
                                beat_id: beat_id.clone(),
                                beat_ref: format!("atelier://story-beat/{beat_id}"),
                                story_document_id: story.document_id.clone(),
                                story_document_ref: story.document_ref.clone(),
                                card_id,
                                beat_text,
                            });
                            *story_status = format!(
                                "Added local CKC story beat under {}. Backend is unavailable; this is not persisted.",
                                story.document_ref
                            );
                        }
                    }
                }
            }
        }

        if mode == CkcBookMode::Moodboard {
            ui.separator();
            ui.heading(egui::RichText::new("Moodboard links").color(palette.text));
            // MT-037: emit the moodboard status as an Argus-readable node (was a plain label with no
            // author_id) so a model can read local-only, non-persistent moodboard results.
            let moodboard_status_response =
                ui.label(egui::RichText::new(moodboard_status.as_str()).color(palette.text_subtle));
            emit_node(
                ui.ctx(),
                moodboard_status_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_MOODBOARD_STATUS_AUTHOR_ID,
                moodboard_status.as_str(),
                false,
            );
            // MT-037: per-surface persistence receipt mirroring the media backend_mode value node.
            let moodboard_backend_mode = if self.ckc_client.is_some() {
                "backend:persistent"
            } else {
                "local-preview:not-persisted"
            };
            let moodboard_backend_mode_response = ui.label(
                egui::RichText::new(format!("moodboard mode: {moodboard_backend_mode}"))
                    .color(palette.text_subtle),
            );
            emit_value_node(
                ui.ctx(),
                moodboard_backend_mode_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_MOODBOARD_BACKEND_MODE_AUTHOR_ID,
                "CKC moodboard persistence mode",
                moodboard_backend_mode,
            );

            let _moodboard_doc_list = ui
                .vertical(|ui| {
                    for (idx, moodboard) in character.moodboard_documents.iter().enumerate() {
                        let selected = idx == active_moodboard_idx;
                        let marker = if selected { "active" } else { "linked" };
                        let row_label = format!(
                            "{marker} moodboard document: {} [{}]",
                            moodboard.title, moodboard.document_ref
                        );
                        let row = ui.selectable_label(selected, &row_label);
                        if row.clicked() {
                            active_moodboard_idx = idx;
                            *active_moodboard_document_id = Some(moodboard.document_id.clone());
                            *moodboard_status = format!(
                                "Selected CKC moodboard document {}",
                                moodboard.document_ref
                            );
                        }
                        let author_id =
                            ckc_moodboard_document_row_author_id(&moodboard.document_id);
                        emit_node(
                            ui.ctx(),
                            row.id,
                            accesskit::Role::ListItem,
                            &author_id,
                            &row_label,
                            selected,
                        );
                        if let Some(snapshot_id) = &moodboard.latest_snapshot_id {
                            let latest_ref = moodboard
                                .latest_snapshot_ref
                                .clone()
                                .unwrap_or_else(|| format!("atelier://moodboard/{snapshot_id}"));
                            let snapshot_author_id = ckc_moodboard_snapshot_row_author_id(
                                &moodboard.document_id,
                                snapshot_id,
                            );
                            let snapshot_label = format!(
                                "{marker} moodboard snapshot: {} [{}] {}",
                                moodboard.moodboard_name, snapshot_id, latest_ref
                            );
                            let snapshot_row = ui.label(
                                egui::RichText::new(&snapshot_label).color(palette.text_subtle),
                            );
                            emit_node(
                                ui.ctx(),
                                snapshot_row.id,
                                accesskit::Role::ListItem,
                                &snapshot_author_id,
                                &format!(
                                    "atelier-ref moodboard:{latest_ref} document:{}",
                                    moodboard.document_ref
                                ),
                                selected,
                            );
                        }
                    }
                })
                .response;

            {
                let moodboard = character
                    .moodboard_documents
                    .get_mut(active_moodboard_idx)
                    .expect("moodboard placeholder inserted before rendering");
                let doc_ref_label = format!("moodboard_document_ref: {}", moodboard.document_ref);
                let doc_ref_response = ui.label(&doc_ref_label);
                emit_node(
                    ui.ctx(),
                    doc_ref_response.id,
                    accesskit::Role::Label,
                    ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
                    &doc_ref_label,
                    false,
                );
                let latest_ref = moodboard
                    .latest_snapshot_ref
                    .clone()
                    .unwrap_or_else(|| "pending-native-moodboard-snapshot".to_owned());
                let latest_ref_label = format!("latest_moodboard_ref: {latest_ref}");
                let latest_ref_response = ui.label(&latest_ref_label);
                emit_node(
                    ui.ctx(),
                    latest_ref_response.id,
                    accesskit::Role::Label,
                    ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
                    &latest_ref_label,
                    false,
                );
                if !moodboard.tags.is_empty() {
                    ui.label(
                        egui::RichText::new(format!(
                            "moodboard tags: {}",
                            moodboard.tags.join(", ")
                        ))
                        .color(palette.text_subtle),
                    );
                }
                let moodboard_editor = ui.add(
                    egui::TextEdit::multiline(&mut moodboard.body_raw_text)
                        .desired_rows(5)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    moodboard_editor.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
                    "CKC moodboard native snapshot JSON editor",
                    false,
                );
                let save_moodboard = ui.button("Save moodboard");
                emit_node(
                    ui.ctx(),
                    save_moodboard.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
                    "Save CKC native moodboard snapshot",
                    false,
                );
                if save_moodboard.clicked() {
                    if moodboard.body_raw_text.trim().is_empty() {
                        moodboard.body_raw_text = local_ckc_moodboard_snapshot_json(
                            &moodboard.document_id,
                            &moodboard.moodboard_name,
                            &format!(
                                "Native CKC moodboard for {} linked to {}",
                                display_name, moodboard.document_ref
                            ),
                        );
                    }
                    // WP-CKC MT-012 (FIX 1): the LIVE moodboard canvas is the source of truth on save.
                    // Fold the isolated board's per-element geometry back onto the current snapshot JSON
                    // (preserving every non-visual field + image ArtifactStore refs) BEFORE persisting, so
                    // visual layout edits survive reload instead of being lost to the stale editor text. On
                    // a no-op (nothing moved) this re-serializes byte-identically, so the backend
                    // content_sha256 dedup suppresses a redundant snapshot. Invalid JSON is left untouched
                    // for the validation gate immediately below to report.
                    if let Ok(prev) =
                        serde_json::from_str::<serde_json::Value>(&moodboard.body_raw_text)
                    {
                        if let Ok(board) = self.moodboard_board.lock() {
                            moodboard.body_raw_text = board_to_ckc_moodboard_json(&board, &prev);
                        }
                    }
                    if let Err(err) =
                        ckc_moodboard_snapshot_to_canvas_projection(&moodboard.body_raw_text)
                    {
                        *moodboard_status = format!("CKC moodboard save blocked: {err}");
                    } else if let Some(client) = self.ckc_client.as_ref() {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            let expected_thread_id =
                                ckc_character_model_ops_thread_id(&character_internal_id);
                            match model_ops_fields
                                .mutation_context(client.actor_id(), &expected_thread_id)
                            {
                                Ok(model_ops) => {
                                    *moodboard_status = format!(
                                        "Creating CKC moodboard document and snapshot for {display_name}"
                                    );
                                    client.create_ckc_moodboard_document_snapshot(
                                        &character_internal_id,
                                        &moodboard.title,
                                        &moodboard.body_raw_text,
                                        &moodboard.tags,
                                        &model_ops.actor_id,
                                        model_ops.context,
                                        self.ckc_character_document_cell.clone(),
                                        self.ckc_moodboard_latest_cell.clone(),
                                    );
                                }
                                Err(err) => {
                                    *moodboard_status =
                                        format!("CKC moodboard create blocked: {err}");
                                }
                            }
                        } else {
                            let expected_parent_version_id =
                                moodboard.current_version_id.as_deref();
                            let expected_thread_id =
                                ckc_document_model_ops_thread_id(&moodboard.document_id);
                            match model_ops_fields
                                .mutation_context(client.actor_id(), &expected_thread_id)
                            {
                                Ok(model_ops) => {
                                    *moodboard_status = format!(
                                        "Saving CKC moodboard document and snapshot for {}",
                                        moodboard.document_ref
                                    );
                                    client.save_ckc_moodboard_document_snapshot(
                                        &moodboard.document_id,
                                        &moodboard.title,
                                        &moodboard.body_raw_text,
                                        &moodboard.tags,
                                        expected_parent_version_id,
                                        &model_ops.actor_id,
                                        model_ops.context,
                                        self.ckc_character_document_cell.clone(),
                                        self.ckc_moodboard_latest_cell.clone(),
                                    );
                                }
                                Err(err) => {
                                    *moodboard_status =
                                        format!("CKC moodboard save blocked: {err}");
                                }
                            }
                        }
                    } else {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            let next_document_id = Uuid::new_v4().to_string();
                            moodboard.document_id = next_document_id.clone();
                            moodboard.document_ref =
                                format!("atelier://document/{next_document_id}");
                        }
                        moodboard.current_version_id = Some(Uuid::new_v4().to_string());
                        moodboard.current_version_seq = (moodboard.current_version_seq + 1).max(1);
                        let snapshot_id = Uuid::new_v4().to_string();
                        moodboard.latest_snapshot_id = Some(snapshot_id.clone());
                        moodboard.latest_snapshot_ref =
                            Some(format!("atelier://moodboard/{snapshot_id}"));
                        let latest_ref = moodboard
                            .latest_snapshot_ref
                            .clone()
                            .unwrap_or_else(|| "pending-native-moodboard-snapshot".to_owned());
                        match apply_ckc_moodboard_snapshot_to_board(
                            &self.moodboard_board,
                            &moodboard.body_raw_text,
                            &latest_ref,
                        ) {
                            Ok(()) => {
                                *moodboard_status = format!(
                                    "Saved local CKC moodboard {latest_ref}. Backend is unavailable; this is not persisted."
                                );
                            }
                            Err(err) => {
                                *moodboard_status =
                                    format!("CKC local moodboard save projection failed: {err}");
                            }
                        }
                    }
                }
                let open = ui.button("Open moodboard");
                emit_node(
                    ui.ctx(),
                    open.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
                    "Open CKC native moodboard snapshot",
                    false,
                );
                if open.clicked() {
                    if let Some(client) = self.ckc_client.as_ref() {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            let expected_thread_id =
                                ckc_character_model_ops_thread_id(&character_internal_id);
                            match model_ops_fields
                                .mutation_context(client.actor_id(), &expected_thread_id)
                            {
                                Ok(model_ops) => {
                                    *moodboard_status = format!(
                                        "Creating CKC moodboard document and snapshot for {display_name}"
                                    );
                                    client.create_ckc_moodboard_document_snapshot(
                                        &character_internal_id,
                                        &moodboard.title,
                                        &moodboard.body_raw_text,
                                        &moodboard.tags,
                                        &model_ops.actor_id,
                                        model_ops.context,
                                        self.ckc_character_document_cell.clone(),
                                        self.ckc_moodboard_latest_cell.clone(),
                                    );
                                }
                                Err(err) => {
                                    *moodboard_status =
                                        format!("CKC moodboard open-create blocked: {err}");
                                }
                            }
                        } else {
                            *moodboard_status = format!(
                                "Opening CKC moodboard snapshot for {}",
                                moodboard.document_ref
                            );
                            client.fetch_ckc_latest_moodboard_snapshot(
                                &moodboard.document_id,
                                self.ckc_moodboard_latest_cell.clone(),
                            );
                        }
                    } else {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            let next_document_id = Uuid::new_v4().to_string();
                            moodboard.document_id = next_document_id.clone();
                            moodboard.document_ref =
                                format!("atelier://document/{next_document_id}");
                            moodboard.current_version_id = Some(Uuid::new_v4().to_string());
                            moodboard.current_version_seq = 1;
                        }
                        if moodboard.latest_snapshot_id.is_none() {
                            let snapshot_id = Uuid::new_v4().to_string();
                            moodboard.latest_snapshot_id = Some(snapshot_id.clone());
                            moodboard.latest_snapshot_ref =
                                Some(format!("atelier://moodboard/{snapshot_id}"));
                        }
                        if ckc_moodboard_snapshot_to_canvas_projection(&moodboard.body_raw_text)
                            .is_err()
                        {
                            moodboard.body_raw_text = local_ckc_moodboard_snapshot_json(
                                &moodboard.document_id,
                                &moodboard.moodboard_name,
                                &format!(
                                    "Native CKC moodboard for {} linked to {}",
                                    display_name, moodboard.document_ref
                                ),
                            );
                        }
                        let latest_ref = moodboard
                            .latest_snapshot_ref
                            .clone()
                            .unwrap_or_else(|| "pending-native-moodboard-snapshot".to_owned());
                        match apply_ckc_moodboard_snapshot_to_board(
                            &self.moodboard_board,
                            &moodboard.body_raw_text,
                            &latest_ref,
                        ) {
                            Ok(()) => {
                                *moodboard_status =
                                    format!("Opened local CKC moodboard {latest_ref}");
                            }
                            Err(err) => {
                                *moodboard_status =
                                    format!("CKC local moodboard projection failed: {err}");
                            }
                        }
                    }
                }
            }
        }
    }

    fn show_ckc_sheet_tools(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        ui.heading(egui::RichText::new("Sheet tools").color(palette.text));
        let template_status = ui.label(&state.ckc_template_status);
        emit_node(
            ui.ctx(),
            template_status.id,
            accesskit::Role::Label,
            ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID,
            &state.ckc_template_status,
            state.ckc_template_pending || state.ckc_safe_subset_pending,
        );
        ui.horizontal_wrapped(|ui| {
            let load_template = ui.add_enabled(
                !state.ckc_template_pending,
                egui::Button::new("Load template"),
            );
            emit_node(
                ui.ctx(),
                load_template.id,
                accesskit::Role::Button,
                ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID,
                "Load bundled CKC CHARACTER_SHEET__v2.00.txt metadata",
                state.ckc_template_pending,
            );
            if load_template.clicked() {
                if let Some(client) = self.ckc_client.as_ref() {
                    state.ckc_template_pending = true;
                    state.ckc_template_status = "Loading CHARACTER_SHEET__v2.00.txt".to_owned();
                    client.fetch_ckc_template(self.ckc_template_cell.clone());
                } else {
                    state.ckc_template_status =
                        "CHARACTER_SHEET__v2.00.txt is bundled locally; live backend not connected."
                            .to_owned();
                }
            }

            let load_safe = ui.add_enabled(
                !state.ckc_safe_subset_pending,
                egui::Button::new("Safe subset"),
            );
            emit_node(
                ui.ctx(),
                load_safe.id,
                accesskit::Role::Button,
                ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID,
                "Load CKC LLM_SAFE_SUBSET__v2.00.json short/SFW-safe Field ID subset",
                state.ckc_safe_subset_pending,
            );
            if load_safe.clicked() {
                if let Some(client) = self.ckc_client.as_ref() {
                    state.ckc_safe_subset_pending = true;
                    state.ckc_template_status = "Loading LLM_SAFE_SUBSET__v2.00.json".to_owned();
                    client.fetch_ckc_safe_subset(self.ckc_safe_subset_cell.clone());
                } else {
                    state.ckc_template_status =
                        "LLM_SAFE_SUBSET__v2.00.json is a bundled Field ID whitelist for short/SFW-safe use."
                            .to_owned();
                }
            }
        });

        ui.label(egui::RichText::new("Import raw sheet text").color(palette.text_subtle));
        let import_editor = ui.add(
            egui::TextEdit::multiline(&mut state.ckc_import_text)
                .desired_rows(3)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            import_editor.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID,
            "CKC raw character sheet import text",
            false,
        );

        let selected_sheet = state.ckc_characters.get(selected_index).map(|character| {
            (
                character.character_internal_id.clone(),
                character.sheet_version_id.clone(),
            )
        });
        ui.horizontal_wrapped(|ui| {
            let import_enabled = !state.ckc_import_pending
                && selected_sheet.is_some()
                && !state.ckc_import_text.trim().is_empty();
            let import = ui.add_enabled(import_enabled, egui::Button::new("Import sheet"));
            emit_node(
                ui.ctx(),
                import.id,
                accesskit::Role::Button,
                ATELIER_CKC_IMPORT_AUTHOR_ID,
                "Import CKC raw sheet text as a guarded append-only version",
                state.ckc_import_pending || !import_enabled,
            );
            if import.clicked() {
                if let Some((character_internal_id, expected_parent_version_id)) =
                    selected_sheet.clone()
                {
                    if let Some(client) = self.ckc_client.as_ref() {
                        let expected_thread_id =
                            ckc_character_model_ops_thread_id(&character_internal_id);
                        match model_ops_mutation_context_from_state(
                            &state,
                            client.actor_id(),
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => {
                                state.ckc_import_pending = true;
                                state.ckc_export_status =
                                    "Importing CKC sheet as append-only version".to_owned();
                                client.import_ckc_sheet_version(
                                    &character_internal_id,
                                    &state.ckc_import_text,
                                    expected_parent_version_id.as_deref(),
                                    Some("handshake-native-atelier-import"),
                                    &model_ops.actor_id,
                                    model_ops.context,
                                    self.ckc_import_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.ckc_export_status =
                                    format!("CKC sheet import blocked: {err}");
                                state.ckc_error = Some(state.ckc_export_status.clone());
                            }
                        }
                    } else {
                        local_import_ckc_sheet(state, selected_index);
                    }
                }
            }

            let selected_version_id = selected_sheet
                .as_ref()
                .and_then(|(_, version_id)| version_id.clone());
            let export_enabled = !state.ckc_export_pending && selected_version_id.is_some();
            let export_txt = ui.add_enabled(export_enabled, egui::Button::new("Export txt"));
            emit_node(
                ui.ctx(),
                export_txt.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_TXT_AUTHOR_ID,
                "Export CKC sheet version as deterministic txt content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_txt.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "txt");
            }
            let export_json = ui.add_enabled(export_enabled, egui::Button::new("Export json"));
            emit_node(
                ui.ctx(),
                export_json.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_JSON_AUTHOR_ID,
                "Export CKC sheet version as deterministic json content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_json.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "json");
            }
            let export_safe_txt =
                ui.add_enabled(export_enabled, egui::Button::new("Export safe txt"));
            emit_node(
                ui.ctx(),
                export_safe_txt.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID,
                "Export CKC sheet version as short/SFW-safe txt content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_safe_txt.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "safe-txt");
            }
            let export_safe_json =
                ui.add_enabled(export_enabled, egui::Button::new("Export safe json"));
            emit_node(
                ui.ctx(),
                export_safe_json.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID,
                "Export CKC sheet version as short/SFW-safe json content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_safe_json.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "safe-json");
            }
        });
        let export_status = ui.label(&state.ckc_export_status);
        emit_node(
            ui.ctx(),
            export_status.id,
            accesskit::Role::Label,
            ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID,
            &state.ckc_export_status,
            state.ckc_export_pending || state.ckc_import_pending,
        );
        if let Some(export) = state.ckc_last_export.as_ref() {
            let export_ref_label = format!(
                "{} {} {} {} {}",
                export.file_name,
                export.version_id,
                short_hash(&export.content_hash),
                export.character_ref,
                export.sheet_version_ref
            );
            let export_ref = ui.label(&export_ref_label);
            emit_node(
                ui.ctx(),
                export_ref.id,
                accesskit::Role::Label,
                ATELIER_CKC_EXPORT_REF_AUTHOR_ID,
                &export_ref_label,
                false,
            );
            let mut preview = export.content.clone();
            let preview_response = ui.add(
                egui::TextEdit::multiline(&mut preview)
                    .desired_rows(4)
                    .interactive(false),
            );
            emit_value_node(
                ui.ctx(),
                preview_response.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID,
                "CKC deterministic sheet export preview",
                &export.content,
            );
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Field ID");
            let field = ui.text_edit_singleline(&mut state.ckc_field_suggestion_id);
            emit_node(
                ui.ctx(),
                field.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID,
                "CKC Field ID for prior-value suggestions",
                false,
            );
            let load = ui.add_enabled(
                !state.ckc_field_suggestion_pending
                    && !state.ckc_field_suggestion_id.trim().is_empty(),
                egui::Button::new("Load suggestions"),
            );
            emit_node(
                ui.ctx(),
                load.id,
                accesskit::Role::Button,
                ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID,
                "Load CKC prior values for the exact Field ID",
                state.ckc_field_suggestion_pending,
            );
            if load.clicked() {
                let field_id = state.ckc_field_suggestion_id.trim().to_owned();
                if let Some(client) = self.ckc_client.as_ref() {
                    state.ckc_field_suggestion_pending = true;
                    state.ckc_field_suggestion_status =
                        format!("Loading prior CKC values for {field_id}");
                    client.fetch_ckc_field_suggestions(
                        &field_id,
                        8,
                        self.ckc_field_suggestions_cell.clone(),
                    );
                } else {
                    state.ckc_field_suggestions =
                        local_field_suggestions(&state.ckc_characters, &field_id);
                    state.ckc_field_suggestion_status = format!(
                        "Loaded {} local prior value(s) for {field_id}",
                        state.ckc_field_suggestions.len()
                    );
                }
            }
        });
        let suggestion_response = ui
            .vertical(|ui| {
                ui.label(&state.ckc_field_suggestion_status);
                for suggestion in &state.ckc_field_suggestions {
                    let label = format!(
                        "{} = {} ({})",
                        suggestion.field_id, suggestion.value, suggestion.occurrences
                    );
                    let row = ui.label(&label);
                    let author_id =
                        ckc_field_suggestion_row_author_id(&suggestion.field_id, &suggestion.value);
                    emit_node(
                        ui.ctx(),
                        row.id,
                        accesskit::Role::ListItem,
                        &author_id,
                        &label,
                        false,
                    );
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            suggestion_response.id,
            accesskit::Role::List,
            ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID,
            &state.ckc_field_suggestion_status,
            state.ckc_field_suggestion_pending,
        );
    }

    fn request_ckc_sheet_export(
        &self,
        state: &mut AtelierPanelState,
        selected_index: usize,
        format: &'static str,
    ) {
        let Some(character) = state.ckc_characters.get(selected_index) else {
            state.ckc_export_status = "No CKC character selected for export.".to_owned();
            return;
        };
        let Some(version_id) = character.sheet_version_id.clone() else {
            state.ckc_export_status =
                "Selected CKC character has no sheet version to export.".to_owned();
            return;
        };
        if let Some(client) = self.ckc_client.as_ref() {
            state.ckc_export_pending = true;
            state.ckc_export_status = format!("Exporting CKC sheet as {format}");
            client.export_ckc_sheet_version(&version_id, format, self.ckc_export_cell.clone());
        } else {
            let export = local_export_ckc_sheet(character, format);
            state.ckc_export_status = format!(
                "Local export {} as {} ({} bytes, hash {})",
                export.file_name,
                export.format,
                export.content.len(),
                short_hash(&export.content_hash)
            );
            state.ckc_last_export = Some(export);
        }
    }

    fn show_ckc_search(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
    ) {
        ui.heading(egui::RichText::new("Search").color(palette.text));
        let query = ui.text_edit_singleline(&mut state.ckc_search_query);
        emit_node(
            ui.ctx(),
            query.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
            "CKC fuzzy vector combined search query",
            false,
        );
        let tags = ui.text_edit_singleline(&mut state.ckc_search_tags);
        emit_node(
            ui.ctx(),
            tags.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
            "CKC rich tag filter",
            false,
        );
        ui.horizontal_wrapped(|ui| {
            let character =
                ui.checkbox(&mut state.ckc_search_filter_selected_character, "Character");
            emit_node(
                ui.ctx(),
                character.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
                "Filter CKC search to the selected character",
                state.ckc_search_filter_selected_character,
            );
            let collection = ui.checkbox(&mut state.ckc_search_filter_selected_collection, "Album");
            emit_node(
                ui.ctx(),
                collection.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
                "Filter CKC search to the selected album",
                state.ckc_search_filter_selected_collection,
            );
            let media = ui.checkbox(&mut state.ckc_search_filter_selected_media, "Media");
            emit_node(
                ui.ctx(),
                media.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
                "Filter CKC search to the selected media asset",
                state.ckc_search_filter_selected_media,
            );
            let similarity = ui.checkbox(
                &mut state.ckc_search_use_selected_media_similarity,
                "Similarity",
            );
            emit_node(
                ui.ctx(),
                similarity.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
                "Use selected media as CKC image-similarity source",
                state.ckc_search_use_selected_media_similarity,
            );
        });
        ui.horizontal(|ui| {
            for mode in CkcSearchMode::ALL {
                let selected = state.ckc_search_mode == mode;
                let button = ui.add(egui::Button::selectable(selected, mode.label()));
                emit_node(
                    ui.ctx(),
                    button.id,
                    accesskit::Role::Button,
                    mode.author_id(),
                    mode.label(),
                    selected,
                );
                if button.clicked() {
                    state.ckc_search_mode = mode;
                }
            }
        });
        let run = ui.add_enabled(!state.ckc_search_pending, egui::Button::new("Search CKC"));
        emit_node(
            ui.ctx(),
            run.id,
            accesskit::Role::Button,
            ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
            "Run CKC search",
            state.ckc_search_pending,
        );
        if run.clicked() {
            let tags = ckc_tags_from_buffer(&state.ckc_search_tags);
            let mode = state.ckc_search_mode;
            let filters = selected_ckc_search_filter_refs(state);
            let character_internal_id = if state.ckc_search_filter_selected_character {
                filters.character_internal_id.as_deref()
            } else {
                None
            };
            let collection_id = if state.ckc_search_filter_selected_collection {
                filters.collection_id.as_deref()
            } else {
                None
            };
            let media_asset_id = if state.ckc_search_filter_selected_media {
                filters.media_asset_id.as_deref()
            } else {
                None
            };
            let similar_to_asset_id = if state.ckc_search_use_selected_media_similarity {
                filters.media_asset_id.as_deref()
            } else {
                None
            };
            if let Some(client) = self.ckc_client.as_ref() {
                state.ckc_search_pending = true;
                state.ckc_search_status = format!("Searching CKC with {} mode", mode.label());
                let modes = vec![mode.backend_value().to_owned()];
                client.search_ckc(
                    &state.ckc_search_query,
                    &modes,
                    &tags,
                    character_internal_id,
                    collection_id,
                    media_asset_id,
                    similar_to_asset_id,
                    None,
                    12,
                    self.ckc_search_cell.clone(),
                );
            } else {
                let mut results =
                    local_ckc_search(&state.ckc_characters, &state.ckc_search_query, mode, &tags);
                results.retain(|result| {
                    ckc_search_result_matches_filters(
                        result,
                        &filters,
                        state.ckc_search_filter_selected_character,
                        state.ckc_search_filter_selected_collection,
                        state.ckc_search_filter_selected_media
                            || state.ckc_search_use_selected_media_similarity,
                    )
                });
                state.ckc_search_results = results;
                state.ckc_search_status = format!(
                    "Local CKC {} search returned {} result(s)",
                    mode.label(),
                    state.ckc_search_results.len()
                );
            }
        }
        let status =
            ui.label(egui::RichText::new(&state.ckc_search_status).color(palette.text_subtle));
        emit_node(
            ui.ctx(),
            status.id,
            accesskit::Role::Label,
            ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID,
            &state.ckc_search_status,
            state.ckc_search_pending,
        );

        let results_response = ui
            .vertical(|ui| {
                if state.ckc_search_results.is_empty() {
                    ui.label(
                        egui::RichText::new("No CKC search results").color(palette.text_subtle),
                    );
                }
                for result in &state.ckc_search_results {
                    let row = ui.label(result.summary_label());
                    emit_node(
                        ui.ctx(),
                        row.id,
                        accesskit::Role::ListItem,
                        &ckc_search_result_row_author_id(&result.target_ref),
                        &result.summary_label(),
                        false,
                    );
                    if !result.snippet.is_empty() {
                        ui.label(
                            egui::RichText::new(result.snippet.clone()).color(palette.text_subtle),
                        );
                    }
                    let mut refs = Vec::new();
                    for value in [
                        result.character_ref.as_deref(),
                        result.sheet_version_ref.as_deref(),
                        result.collection_ref.as_deref(),
                        result.media_ref.as_deref(),
                        result.tag_ref.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        refs.push(value.to_owned());
                    }
                    if !refs.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("refs: {}", refs.join(" | ")))
                                .color(palette.text_subtle),
                        );
                    }
                    if !result.tags.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("tags: {}", result.tags.join(", ")))
                                .color(palette.text_subtle),
                        );
                    }
                    for note in &result.tag_notes {
                        let scope = note.scope_ref.as_deref().unwrap_or("global");
                        ui.label(
                            egui::RichText::new(format!(
                                "tag note {} [{}]: {}",
                                note.tag_text, scope, note.note
                            ))
                            .color(palette.text_subtle),
                        );
                    }
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            results_response.id,
            accesskit::Role::List,
            ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
            "CKC search results",
            false,
        );

        ui.add_space(4.0);
        ui.label(egui::RichText::new("Tag note").color(palette.text));
        ui.horizontal(|ui| {
            let tag = ui.text_edit_singleline(&mut state.ckc_tag_note_tag);
            emit_node(
                ui.ctx(),
                tag.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
                "CKC tag note tag",
                false,
            );
            let scope = ui.text_edit_singleline(&mut state.ckc_tag_note_scope_ref);
            emit_node(
                ui.ctx(),
                scope.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID,
                "CKC tag note scope ref",
                false,
            );
        });
        let note = ui.add(
            egui::TextEdit::multiline(&mut state.ckc_tag_note_editor)
                .desired_rows(2)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            note.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
            "CKC rich tag note editor",
            false,
        );
        let save = ui.add_enabled(
            !state.ckc_tag_note_pending,
            egui::Button::new("Save tag note"),
        );
        emit_node(
            ui.ctx(),
            save.id,
            accesskit::Role::Button,
            ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
            "Save CKC rich tag note",
            state.ckc_tag_note_pending,
        );
        if save.clicked() {
            let request = CkcTagNoteSaveRequest {
                tag_text: state.ckc_tag_note_tag.trim().to_ascii_lowercase(),
                scope_ref: if state.ckc_tag_note_scope_ref.trim().is_empty() {
                    None
                } else {
                    Some(state.ckc_tag_note_scope_ref.trim().to_owned())
                },
                note: state.ckc_tag_note_editor.clone(),
            };
            if !request.tag_text.is_empty() {
                if let Some(client) = self.ckc_client.as_ref() {
                    let expected_thread_id = ckc_tag_note_model_ops_thread_id(
                        &request.tag_text,
                        request.scope_ref.as_deref(),
                    );
                    match model_ops_mutation_context_from_state(
                        &state,
                        client.actor_id(),
                        &expected_thread_id,
                    ) {
                        Ok(model_ops) => {
                            state.ckc_tag_note_pending = true;
                            client.save_ckc_tag_note(
                                &request.tag_text,
                                request.scope_ref.as_deref(),
                                &request.note,
                                &model_ops.actor_id,
                                model_ops.context,
                                self.ckc_tag_note_cell.clone(),
                            );
                        }
                        Err(err) => {
                            state.ckc_search_status = format!("CKC tag note blocked: {err}");
                        }
                    }
                } else {
                    let note = CkcTagNoteRecord {
                        tag_ref: format!("atelier://tag/local-{}", request.tag_text),
                        tag_text: request.tag_text.clone(),
                        scope_ref: request.scope_ref,
                        note: request.note,
                    };
                    attach_tag_note_to_visible_results(&mut state.ckc_search_results, note);
                    state.ckc_search_status = format!(
                        "Saved local CKC tag note for {}. Backend is unavailable; this is not persisted.",
                        request.tag_text
                    );
                }
            }
        }
    }

    fn show_ckc_sheet_artifact_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        if let Some(selected_link_id) = state.ckc_selected_sheet_artifact_link_id.as_deref() {
            let selected_link_still_belongs_to_character = state
                .ckc_characters
                .get(selected_index)
                .is_some_and(|character| {
                    character
                        .sheet_artifact_links
                        .iter()
                        .any(|link| link.link_id == selected_link_id)
                });
            if !selected_link_still_belongs_to_character {
                state.ckc_selected_sheet_artifact_link_id = None;
                state.ckc_sheet_artifact_reuse_ref.clear();
            }
        }
        let pending = state.ckc_sheet_artifact_pending;
        let status = state.ckc_sheet_artifact_status.clone();
        let selected_link_id = state.ckc_selected_sheet_artifact_link_id.clone();
        let reuse_ref = state.ckc_sheet_artifact_reuse_ref.clone();
        let latest_pose_export = state.pose_last_export.clone();
        let backend_available = self.ckc_client.is_some();
        let mut artifact_kind = std::mem::take(&mut state.ckc_sheet_artifact_kind);
        let mut artifact_ref = std::mem::take(&mut state.ckc_sheet_artifact_ref);
        let mut manifest_ref = std::mem::take(&mut state.ckc_sheet_artifact_manifest_ref);
        let mut label = std::mem::take(&mut state.ckc_sheet_artifact_label);
        let mut reuse_role = std::mem::take(&mut state.ckc_sheet_artifact_reuse_role);
        if state.ckc_sheet_artifact_actor_id.trim().is_empty() {
            state.ckc_sheet_artifact_actor_id = self
                .ckc_client
                .as_ref()
                .map(|client| client.actor_id().to_owned())
                .unwrap_or_else(|| "local-atelier-panel".to_owned());
        }
        let mut actor_id = std::mem::take(&mut state.ckc_sheet_artifact_actor_id);
        let mut pending_attach = None;
        let mut pending_detach = None;
        let mut next_selection = None;
        let mut next_reuse_ref = None;
        let mut next_status = None;

        if let Some(character) = state.ckc_characters.get_mut(selected_index) {
            let (attach, detach, selection, reuse, status) = self.show_ckc_sheet_artifacts(
                ui,
                palette,
                character,
                pending,
                &status,
                &mut artifact_kind,
                &mut artifact_ref,
                &mut manifest_ref,
                &mut label,
                &mut reuse_role,
                &mut actor_id,
                selected_link_id.as_deref(),
                &reuse_ref,
                latest_pose_export.as_ref(),
                backend_available,
            );
            pending_attach = attach;
            pending_detach = detach;
            next_selection = selection;
            next_reuse_ref = reuse;
            next_status = status;
        }

        state.ckc_sheet_artifact_kind = artifact_kind;
        state.ckc_sheet_artifact_ref = artifact_ref;
        state.ckc_sheet_artifact_manifest_ref = manifest_ref;
        state.ckc_sheet_artifact_label = label;
        state.ckc_sheet_artifact_reuse_role = reuse_role;
        state.ckc_sheet_artifact_actor_id = actor_id;

        if let Some(selection) = next_selection {
            state.ckc_selected_sheet_artifact_link_id = Some(selection);
        }
        if let Some(reuse_ref) = next_reuse_ref {
            state.ckc_sheet_artifact_reuse_ref = reuse_ref;
        }
        if let Some(status) = next_status {
            state.ckc_sheet_artifact_status = status;
            state.ckc_error = None;
        }

        if let Some(request) = pending_attach {
            if let Some(client) = self.ckc_client.as_ref() {
                let CkcSheetArtifactAttachRequest {
                    sheet_version_id,
                    artifact_kind,
                    artifact_ref,
                    manifest_ref,
                    source_ref,
                    label,
                    reuse_role,
                    metadata,
                    actor_id,
                } = request;
                let expected_thread_id = ckc_sheet_artifacts_model_ops_thread_id(&sheet_version_id);
                match model_ops_mutation_context_from_state(&state, &actor_id, &expected_thread_id)
                {
                    Ok(model_ops) => {
                        state.ckc_sheet_artifact_pending = true;
                        state.ckc_sheet_artifact_status = format!(
                            "Attaching {artifact_kind} reusable artifact to sheet {sheet_version_id}"
                        );
                        state.ckc_error = None;
                        client.attach_ckc_sheet_artifact_link(
                            &sheet_version_id,
                            &artifact_kind,
                            &artifact_ref,
                            manifest_ref.as_deref(),
                            source_ref.as_deref(),
                            label.as_deref(),
                            reuse_role.as_deref(),
                            metadata,
                            &model_ops.actor_id,
                            model_ops.context,
                            self.ckc_sheet_artifact_links_cell.clone(),
                        );
                    }
                    Err(err) => {
                        state.ckc_sheet_artifact_status =
                            format!("Sheet artifact attach blocked: {err}");
                        state.ckc_error = Some(state.ckc_sheet_artifact_status.clone());
                    }
                }
            }
        }

        if let Some(request) = pending_detach {
            if let Some(client) = self.ckc_client.as_ref() {
                let expected_thread_id =
                    ckc_sheet_artifacts_model_ops_thread_id(&request.sheet_version_id);
                match model_ops_mutation_context_from_state(
                    &state,
                    &request.actor_id,
                    &expected_thread_id,
                ) {
                    Ok(model_ops) => {
                        state.ckc_sheet_artifact_pending = true;
                        state.ckc_sheet_artifact_status = format!(
                            "Detaching reusable artifact {} from sheet {}",
                            request.link_id, request.sheet_version_id
                        );
                        state.ckc_error = None;
                        client.detach_ckc_sheet_artifact_link(
                            &request.sheet_version_id,
                            &request.link_id,
                            &model_ops.actor_id,
                            model_ops.context,
                            self.ckc_sheet_artifact_links_cell.clone(),
                        );
                    }
                    Err(err) => {
                        state.ckc_sheet_artifact_status =
                            format!("Sheet artifact detach blocked: {err}");
                        state.ckc_error = Some(state.ckc_sheet_artifact_status.clone());
                    }
                }
            }
        }
    }

    fn show_ckc_sheet_artifacts(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        pending: bool,
        status: &str,
        artifact_kind: &mut String,
        artifact_ref: &mut String,
        manifest_ref: &mut String,
        label: &mut String,
        reuse_role: &mut String,
        actor_id: &mut String,
        selected_link_id: Option<&str>,
        reuse_ref: &str,
        latest_pose_export: Option<&PosekitExportSnapshot>,
        backend_available: bool,
    ) -> (
        Option<CkcSheetArtifactAttachRequest>,
        Option<CkcSheetArtifactDetachRequest>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        ui.heading(egui::RichText::new("Reusable sheet artifacts").color(palette.text));
        let status_response = ui.label(egui::RichText::new(status).color(palette.text_subtle));
        emit_node(
            ui.ctx(),
            status_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_SHEET_ARTIFACT_STATUS_AUTHOR_ID,
            status,
            pending,
        );

        let mut pending_attach = None;
        let mut pending_detach = None;
        let selected_link_id = selected_link_id
            .filter(|link_id| {
                character
                    .sheet_artifact_links
                    .iter()
                    .any(|link| &link.link_id == link_id)
            })
            .map(ToOwned::to_owned)
            .or_else(|| {
                character
                    .sheet_artifact_links
                    .first()
                    .map(|link| link.link_id.clone())
            });
        let mut pending_selection = selected_link_id.clone();
        let mut pending_reuse_ref = None;
        let mut pending_status = None;
        let selected_link_id = selected_link_id.as_deref();

        let list_response = ui
            .vertical(|ui| {
                if character.sheet_artifact_links.is_empty() {
                    ui.label(
                        egui::RichText::new("No reusable sheet artifacts linked yet.")
                            .color(palette.text_subtle),
                    );
                }
                for link in &character.sheet_artifact_links {
                    let selected = selected_link_id == Some(link.link_id.as_str());
                    let row_author_id = ckc_sheet_artifact_row_author_id(&link.link_id);
                    let row = ui.selectable_label(selected, link.summary());
                    emit_node(
                        ui.ctx(),
                        row.id,
                        accesskit::Role::ListItem,
                        &row_author_id,
                        &format!(
                            "{} {} {} {}",
                            link.sheet_version_ref, link.typed_ref, link.artifact_kind, link.artifact_ref
                        ),
                        selected,
                    );
                    if row.clicked() {
                        pending_selection = Some(link.link_id.clone());
                        pending_reuse_ref = Some(link.typed_ref.clone());
                    }
                    if selected {
                        let detail_label = format!(
                            "sheet={} sheet_version_id={} character={} character_internal_id={} manifest={} source={} label={} linked_by={} metadata={}",
                            link.sheet_version_ref,
                            link.sheet_version_id,
                            link.character_ref,
                            link.character_internal_id,
                            link.manifest_ref.as_deref().unwrap_or("<none>"),
                            link.source_ref.as_deref().unwrap_or("<none>"),
                            link.label.as_deref().unwrap_or("<none>"),
                            link.linked_by,
                            link.metadata
                        );
                        let detail_response = ui.label(
                            egui::RichText::new(&detail_label).color(palette.text_subtle),
                        );
                        let detail_author_id = ckc_sheet_artifact_detail_author_id(&link.link_id);
                        emit_value_node(
                            ui.ctx(),
                            detail_response.id,
                            accesskit::Role::Label,
                            &detail_author_id,
                            &detail_label,
                            &detail_label,
                        );
                    }
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            list_response.id,
            accesskit::Role::List,
            ATELIER_CKC_SHEET_ARTIFACT_LIST_AUTHOR_ID,
            "CKC reusable sheet artifact links",
            pending,
        );

        let selected_reuse_ref = pending_reuse_ref
            .as_deref()
            .or_else(|| {
                selected_link_id.and_then(|link_id| {
                    character
                        .sheet_artifact_links
                        .iter()
                        .find(|link| link.link_id == link_id)
                        .map(|link| link.typed_ref.as_str())
                })
            })
            .unwrap_or(reuse_ref);
        let reuse_response = ui.label(format!("reuse typed_ref: {selected_reuse_ref}"));
        emit_node(
            ui.ctx(),
            reuse_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_SHEET_ARTIFACT_REUSE_REF_AUTHOR_ID,
            selected_reuse_ref,
            false,
        );

        ui.horizontal_wrapped(|ui| {
            let kind = ui.text_edit_singleline(artifact_kind);
            emit_node(
                ui.ctx(),
                kind.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_KIND_AUTHOR_ID,
                "Sheet artifact kind",
                false,
            );
            let artifact = ui.text_edit_singleline(artifact_ref);
            emit_node(
                ui.ctx(),
                artifact.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_REF_AUTHOR_ID,
                "Reusable artifact_ref to attach to the current CKC sheet version",
                false,
            );
        });
        ui.horizontal_wrapped(|ui| {
            let manifest = ui.text_edit_singleline(manifest_ref);
            emit_node(
                ui.ctx(),
                manifest.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_MANIFEST_AUTHOR_ID,
                "Optional manifest_ref or Comfy receipt_ref",
                false,
            );
            let label_response = ui.text_edit_singleline(label);
            emit_node(
                ui.ctx(),
                label_response.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_LABEL_AUTHOR_ID,
                "Human label for this reusable sheet artifact",
                false,
            );
            let role = ui.text_edit_singleline(reuse_role);
            emit_node(
                ui.ctx(),
                role.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_ROLE_AUTHOR_ID,
                "Reuse role for downstream tools",
                false,
            );
            let actor = ui.text_edit_singleline(actor_id);
            emit_value_node(
                ui.ctx(),
                actor.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_ACTOR_AUTHOR_ID,
                "Parallel agent actor_id for sheet artifact attach/detach writes",
                actor_id,
            );
        });

        ui.horizontal_wrapped(|ui| {
            let attach = ui.add_enabled(!pending, egui::Button::new("Attach artifact"));
            emit_node(
                ui.ctx(),
                attach.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_ARTIFACT_ATTACH_AUTHOR_ID,
                "Attach reusable artifact ref to current CKC sheet version",
                pending,
            );
            if attach.clicked() {
                if let Some(sheet_version_id) = character.sheet_version_id.clone() {
                    let request = CkcSheetArtifactAttachRequest {
                        sheet_version_id,
                        artifact_kind: artifact_kind.trim().to_owned(),
                        artifact_ref: artifact_ref.trim().to_owned(),
                        manifest_ref: non_empty_trimmed(manifest_ref),
                        source_ref: None,
                        label: non_empty_trimmed(label),
                        reuse_role: non_empty_trimmed(reuse_role),
                        metadata: serde_json::json!({
                            "attached_from": "atelier_ckc_manual",
                        }),
                        actor_id: actor_id_or_default(actor_id),
                    };
                    if !request.artifact_kind.is_empty() && !request.artifact_ref.is_empty() {
                        if backend_available {
                            pending_attach = Some(request);
                        } else if let Some(link) = CkcSheetArtifactLinkRecord::local(
                            character,
                            request.artifact_kind,
                            request.artifact_ref,
                            request.manifest_ref,
                            request.source_ref,
                            request.label,
                            request.reuse_role,
                            request.metadata,
                            request.actor_id,
                        ) {
                            pending_selection = Some(link.link_id.clone());
                            pending_reuse_ref = Some(link.typed_ref.clone());
                            character.sheet_artifact_links.push(link);
                            pending_status = Some(
                                "Attached local reusable sheet artifact. Backend is unavailable; this is not persisted."
                                    .to_owned(),
                            );
                        }
                    }
                }
            }

            let attach_pose = ui.add_enabled(
                !pending && latest_pose_export.is_some(),
                egui::Button::new("Attach Posekit export"),
            );
            emit_node(
                ui.ctx(),
                attach_pose.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_ARTIFACT_ATTACH_POSE_AUTHOR_ID,
                "Attach latest Posekit OpenPose PNG export to current CKC sheet version",
                pending || latest_pose_export.is_none(),
            );
            if attach_pose.clicked() {
                if let (Some(sheet_version_id), Some(snapshot)) =
                    (character.sheet_version_id.clone(), latest_pose_export)
                {
                    let source_ref = snapshot
                        .rig_id
                        .as_ref()
                        .map(|rig_id| format!("posekit://rig/{rig_id}"))
                        .unwrap_or_else(|| snapshot.source_ref.clone());
                    let request = CkcSheetArtifactAttachRequest {
                        sheet_version_id,
                        artifact_kind: "openpose_png".to_owned(),
                        artifact_ref: snapshot.png_artifact_ref.clone(),
                        manifest_ref: Some(snapshot.png_manifest_ref.clone()),
                        source_ref: Some(source_ref),
                        label: Some(format!("Posekit yaw {:.0} OpenPose", snapshot.yaw_deg)),
                        reuse_role: Some("cui_openpose_conditioning".to_owned()),
                        metadata: serde_json::json!({
                            "schema": "hsk.atelier.posekit.openpose_export@1",
                            "yaw_deg": snapshot.yaw_deg,
                            "pitch_deg": snapshot.pitch_deg,
                            "zoom": snapshot.zoom,
                            "receipt_ref": snapshot.receipt_ref,
                            "json_artifact_ref": snapshot.json_artifact_ref,
                            "json_manifest_ref": snapshot.json_manifest_ref,
                            "content_hash": snapshot.content_hash,
                        }),
                        actor_id: actor_id_or_default(actor_id),
                    };
                    if backend_available {
                        pending_attach = Some(request);
                    } else if let Some(link) = CkcSheetArtifactLinkRecord::local(
                        character,
                        request.artifact_kind,
                        request.artifact_ref,
                        request.manifest_ref,
                        request.source_ref,
                        request.label,
                        request.reuse_role,
                        request.metadata,
                        request.actor_id,
                    ) {
                        pending_selection = Some(link.link_id.clone());
                        pending_reuse_ref = Some(link.typed_ref.clone());
                        character.sheet_artifact_links.push(link);
                        pending_status = Some(
                            "Attached local Posekit OpenPose export. Backend is unavailable; this is not persisted."
                                .to_owned(),
                        );
                    }
                }
            }

            let detach = ui.add_enabled(
                selected_link_id.is_some() && !pending,
                egui::Button::new("Detach"),
            );
            emit_node(
                ui.ctx(),
                detach.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_ARTIFACT_DETACH_AUTHOR_ID,
                "Soft-detach selected reusable sheet artifact",
                pending || selected_link_id.is_none(),
            );
            if detach.clicked() {
                if let (Some(sheet_version_id), Some(link_id)) =
                    (character.sheet_version_id.clone(), selected_link_id)
                {
                    if backend_available {
                        pending_detach = Some(CkcSheetArtifactDetachRequest {
                            sheet_version_id,
                            link_id: link_id.to_owned(),
                            actor_id: actor_id_or_default(actor_id),
                        });
                    } else {
                        character
                            .sheet_artifact_links
                            .retain(|link| link.link_id != link_id);
                        pending_selection = character
                            .sheet_artifact_links
                            .first()
                            .map(|link| link.link_id.clone());
                        pending_reuse_ref = character
                            .sheet_artifact_links
                            .first()
                            .map(|link| link.typed_ref.clone());
                        pending_status = Some(
                            "Detached local reusable sheet artifact. Backend is unavailable; this is not persisted."
                                .to_owned(),
                        );
                    }
                }
            }
        });

        (
            pending_attach,
            pending_detach,
            pending_selection,
            pending_reuse_ref,
            pending_status,
        )
    }

    fn show_ckc_linked_media(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        media_save_pending: bool,
        selected_media_key: Option<&str>,
        selected_album_collection_id: Option<&str>,
        album_create_pending: bool,
        album_link_pending: bool,
        album_page_pending: bool,
        album_page_pending_collection_id: Option<&str>,
        album_list_page_pending: bool,
        album_list_page_pending_character_id: Option<&str>,
        album_status: &str,
        media_image_status: &str,
        album_create_name: &mut String,
        album_create_notes: &mut String,
        album_create_tags: &mut String,
        album_link_asset_ids: &mut String,
        album_link_source_path_ref: &mut String,
        album_link_source_url_ref: &mut String,
        album_link_edit_source_path_ref: &mut String,
        album_link_edit_source_url_ref: &mut String,
        album_link_edit_clear_source_path: &mut bool,
        album_link_edit_clear_source_url: &mut bool,
        media_actor_id: &mut String,
    ) -> (
        Option<CkcMediaSaveRequest>,
        Option<String>,
        Option<String>,
        Option<CkcAlbumCreateRequest>,
        Option<CkcAlbumLinkAssetsRequest>,
        Option<CkcAlbumUnlinkRequest>,
        Option<CkcAlbumLinkEditRequest>,
        Option<CkcAlbumReorderRequest>,
        Option<CkcAlbumPageRequest>,
        Option<CkcAlbumListPageRequest>,
        Option<String>,
    ) {
        ui.heading(egui::RichText::new("Linked media").color(palette.text));
        let mut pending_album_create = None;
        let mut pending_album_link = None;
        let mut pending_album_unlink = None;
        let mut pending_album_link_edit = None;
        let mut pending_album_reorder = None;
        let mut pending_album_page = None;
        let mut pending_album_list_page = None;
        let mut pending_album_status = None;
        let mut pending_album_selection = None;
        let mut selected_member_invalidated = false;
        let album_status_response = ui.label(album_status);
        emit_node(
            ui.ctx(),
            album_status_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID,
            album_status,
            album_create_pending
                || album_link_pending
                || album_page_pending
                || album_list_page_pending,
        );
        ui.horizontal_wrapped(|ui| {
            let backend_mode = if self.ckc_client.is_some() {
                "backend:persistent"
            } else {
                "local-preview:not-persisted"
            };
            let mode = ui.label(
                egui::RichText::new(format!("media mode: {backend_mode}"))
                    .color(palette.text_subtle),
            );
            emit_value_node(
                ui.ctx(),
                mode.id,
                accesskit::Role::Label,
                ATELIER_CKC_MEDIA_BACKEND_MODE_AUTHOR_ID,
                "CKC media persistence mode",
                backend_mode,
            );
            ui.label(egui::RichText::new("actor").color(palette.text_subtle));
            let actor = ui.text_edit_singleline(media_actor_id);
            emit_node(
                ui.ctx(),
                actor.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_MEDIA_ACTOR_AUTHOR_ID,
                "Parallel agent actor_id for CKC album and media writes",
                false,
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("album name").color(palette.text_subtle));
            let name = ui.text_edit_singleline(album_create_name);
            emit_node(
                ui.ctx(),
                name.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
                "CKC album name",
                false,
            );
            ui.label(egui::RichText::new("album tags").color(palette.text_subtle));
            let tags = ui.text_edit_singleline(album_create_tags);
            emit_node(
                ui.ctx(),
                tags.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
                "CKC album tags",
                false,
            );
            let create = ui.add_enabled(!album_create_pending, egui::Button::new("Create album"));
            emit_node(
                ui.ctx(),
                create.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID,
                "Create CKC media album for selected character",
                album_create_pending,
            );
            if create.clicked() {
                let name = album_create_name.trim().to_owned();
                if !name.is_empty() {
                    let notes = non_empty_trimmed(album_create_notes);
                    let tags = ckc_tags_from_buffer(album_create_tags);
                    if self.ckc_client.is_some() {
                        pending_album_create = Some(CkcAlbumCreateRequest {
                            character_internal_id: character.character_internal_id.clone(),
                            name,
                            notes,
                            sheet_version_id: character.sheet_version_id.clone(),
                            tags,
                            actor_id: actor_id_or_default(media_actor_id),
                        });
                    } else {
                        let actor_id = actor_id_or_default(media_actor_id);
                        let collection_id = Uuid::new_v4().to_string();
                        let collection_ref = format!("atelier://collection/{collection_id}");
                        character.media_albums.push(CkcMediaAlbumRecord {
                            collection_id: collection_id.clone(),
                            collection_ref,
                            name,
                            description: notes.unwrap_or_default(),
                            tags,
                            member_count: 0,
                            members_next_offset: None,
                            created_by: actor_id.clone(),
                            updated_by: actor_id.clone(),
                            members: Vec::new(),
                        });
                        character.media_album_count =
                            character.media_album_count.max(character.media_albums.len());
                        pending_album_selection = Some(collection_id);
                        pending_album_status = Some(format!(
                            "Created local CKC album for actor {}. Backend is unavailable; this is not persisted.",
                            actor_id
                        ));
                    }
                }
            }
        });
        ui.label(egui::RichText::new("album notes").color(palette.text_subtle));
        let notes = ui.add(
            egui::TextEdit::multiline(album_create_notes)
                .desired_rows(2)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            notes.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID,
            "CKC album notes",
            false,
        );

        let album_total = character
            .media_album_count
            .max(character.media_albums.len());
        let album_list_next_offset_label = character
            .media_albums_next_offset
            .map(|offset| offset.to_string())
            .unwrap_or_else(|| "none".to_owned());
        let album_list_pending_for_character = album_list_page_pending
            && album_list_page_pending_character_id
                == Some(character.character_internal_id.as_str());
        let album_list_page_state = format!(
            "loaded={} total={} offset={} limit={} member_limit={} next_offset={} pending={} final_page={}",
            character.media_albums.len(),
            album_total,
            character.media_album_offset,
            character.media_album_limit,
            character.media_album_member_limit,
            album_list_next_offset_label,
            album_list_pending_for_character,
            character.media_albums_next_offset.is_none()
        );
        let album_list_page_response = ui.label(
            egui::RichText::new(format!("album list page: {album_list_page_state}"))
                .color(palette.text_subtle),
        );
        emit_value_node(
            ui.ctx(),
            album_list_page_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_ALBUM_LIST_PAGE_STATE_AUTHOR_ID,
            "CKC album-list loaded/total/next-offset page state",
            &album_list_page_state,
        );
        if let Some(next_offset) = character.media_albums_next_offset {
            let load_more_albums = ui.add_enabled(
                !album_list_page_pending,
                egui::Button::new("Load more albums"),
            );
            emit_node(
                ui.ctx(),
                load_more_albums.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_LIST_LOAD_MORE_AUTHOR_ID,
                "Load the next CKC character media-album page",
                album_list_page_pending,
            );
            if load_more_albums.clicked() {
                pending_album_list_page = Some(CkcAlbumListPageRequest {
                    character_internal_id: character.character_internal_id.clone(),
                    offset: next_offset,
                });
            }
        }

        let selected_album_target = selected_album_collection_id
            .filter(|collection_id| {
                character
                    .media_albums
                    .iter()
                    .any(|album| album.collection_id == *collection_id)
            })
            .map(ToOwned::to_owned)
            .or_else(|| pending_album_selection.clone());
        let selected_album_target_label = selected_album_target
            .as_ref()
            .and_then(|collection_id| {
                character
                    .media_albums
                    .iter()
                    .find(|album| &album.collection_id == collection_id)
            })
            .map(|album| format!("target album: {} ({})", album.name, album.collection_ref))
            .unwrap_or_else(|| "target album: none selected".to_owned());
        let target = ui.label(
            egui::RichText::new(selected_album_target_label.as_str()).color(palette.text_subtle),
        );
        emit_value_node(
            ui.ctx(),
            target.id,
            accesskit::Role::Label,
            ATELIER_CKC_ALBUM_LINK_TARGET_AUTHOR_ID,
            "CKC album link target",
            &selected_album_target_label,
        );

        let resolved_selection = character
            .selected_or_first_media_location(selected_media_key)
            .map(|(album_idx, member_idx)| {
                let album = &character.media_albums[album_idx];
                let member = &album.members[member_idx];
                ckc_media_occurrence_key(&album.collection_id, &member.asset_id)
            });
        // MT-010 de-scaffold: resolve the selected linked-media ref to a media-asset UUID and prepare the
        // REAL decoded texture from the per-asset cache BEFORE painting the viewer, so the same frame
        // shows the actual image (or an explicit empty/error state) instead of the old placeholder tile.
        let selected_media_render_ref = character
            .selected_or_first_media_location(resolved_selection.as_deref())
            .map(|(album_idx, member_idx)| {
                character.media_albums[album_idx].members[member_idx]
                    .media_ref
                    .clone()
            });
        let selected_media_asset_id = selected_media_render_ref
            .as_deref()
            .and_then(parse_media_asset_id_from_ref);
        let media_render =
            self.prepare_ckc_media_render(ui.ctx(), selected_media_asset_id.as_deref());
        let viewer_inner = ui.vertical(|ui| {
            ui.label(egui::RichText::new("Selected image preview").color(palette.text));
            if let Some((album_idx, member_idx)) =
                character.selected_or_first_media_location(resolved_selection.as_deref())
            {
                let album = &character.media_albums[album_idx];
                let member = &album.members[member_idx];
                // Paint the REAL decoded linked image (or an explicit empty/error state) — never a
                // fabricated placeholder tile.
                let (state_token, detail) =
                    draw_ckc_media_image(ui, palette, member.media_ref.as_str(), &media_render);
                ui.label(
                    egui::RichText::new(format!(
                        "{}\n{}\n{}",
                        member.display_label,
                        member
                            .source_path_ref
                            .as_deref()
                            .unwrap_or("no source_path_ref"),
                        member
                            .source_url_ref
                            .as_deref()
                            .unwrap_or("no source_url_ref")
                    ))
                    .color(palette.text_subtle),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "album: {} | status: {}",
                        album.name,
                        member.review_status.as_deref().unwrap_or("unreviewed")
                    ))
                    .color(palette.text_subtle),
                );
                (state_token, detail)
            } else {
                ui.label(
                    egui::RichText::new("No linked image selected for this character.")
                        .color(palette.text_subtle),
                );
                ("empty", "dimensions=none".to_owned())
            }
        });
        let viewer_response = viewer_inner.response;
        let (media_state_token, media_detail) = viewer_inner.inner;
        // Keep ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID stable but enrich its value so Argus can read whether
        // the viewer painted a real image (media_image=loaded dimensions=WxH), the empty state, or a
        // decode error — the viewer node alone previously carried no load state.
        let viewer_node_label = format!(
            "CKC selected image preview and source refs media_image={media_state_token} {media_detail}"
        );
        emit_value_node(
            ui.ctx(),
            viewer_response.id,
            accesskit::Role::Group,
            ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
            &viewer_node_label,
            &viewer_node_label,
        );
        // MT-010: explicit linked-image byte-fetch status node (mirrors atelier-pose-source-status) —
        // distinguishes an unset selection, an unresolvable ref, backend offline, a pending fetch, a
        // load, a cache hit, and a fetch failure, which the viewer node alone cannot.
        let media_status_response =
            ui.label(egui::RichText::new(media_image_status).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            media_status_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_MEDIA_IMAGE_STATUS_AUTHOR_ID,
            "CKC linked image byte-fetch status",
            media_image_status,
        );
        let (
            selected_media_occurrence,
            selected_album_ref,
            selected_media_ref,
            selected_folder_ref,
            selected_source_url_ref,
            selected_folder_ref_kind,
            selected_source_url_ref_kind,
            selected_source_path_origin,
            selected_source_url_origin,
            selected_album_attribution,
            selected_media_attribution,
            selected_media_status,
        ) = character
            .selected_or_first_media_location(resolved_selection.as_deref())
            .map(|(album_idx, member_idx)| {
                let album = &character.media_albums[album_idx];
                let member = &album.members[member_idx];
                let member_updated_at = member.member_updated_at_utc.as_deref().unwrap_or("none");
                let notes_updated_at = member.notes_updated_at_utc.as_deref().unwrap_or("none");
                (
                    format!(
                        "{}::{} order={}",
                        album.collection_id, member.asset_id, member.sort_order
                    ),
                    album.collection_ref.clone(),
                    member.media_ref.clone(),
                    member.source_path_ref.clone().unwrap_or_default(),
                    member.source_url_ref.clone().unwrap_or_default(),
                    if member.source_path_ref.is_some() {
                        "folder".to_owned()
                    } else {
                        "none".to_owned()
                    },
                    if member.source_url_ref.is_some() {
                        "source_url".to_owned()
                    } else {
                        "none".to_owned()
                    },
                    member.source_path_ref_origin.clone(),
                    member.source_url_ref_origin.clone(),
                    format!(
                        "created_by={} updated_by={}",
                        album.created_by, album.updated_by
                    ),
                    format!(
                        "linked_by={} member_updated_by={} member_updated_at_utc={} notes_updated_by={} notes_updated_at_utc={}",
                        member.linked_by,
                        member.member_updated_by,
                        member_updated_at,
                        member.notes_updated_by,
                        notes_updated_at
                    ),
                    member
                        .review_status
                        .clone()
                        .unwrap_or_else(|| "unreviewed".to_owned()),
                )
            })
            .unwrap_or_else(|| {
                (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    "none".to_owned(),
                    "none".to_owned(),
                    "none".to_owned(),
                    "none".to_owned(),
                    "created_by=none updated_by=none".to_owned(),
                    "linked_by=none member_updated_by=none member_updated_at_utc=none notes_updated_by=none notes_updated_at_utc=none".to_owned(),
                    "no linked media selected".to_owned(),
                )
            });
        ui.vertical(|ui| {
            for (author_id, label, value) in [
                (
                    ATELIER_CKC_SELECTED_MEDIA_OCCURRENCE_AUTHOR_ID,
                    "selected media occurrence",
                    selected_media_occurrence.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_ALBUM_REF_AUTHOR_ID,
                    "selected album ref",
                    selected_album_ref.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_MEDIA_REF_AUTHOR_ID,
                    "selected media ref",
                    selected_media_ref.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_FOLDER_REF_AUTHOR_ID,
                    "selected folder ref",
                    selected_folder_ref.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_SOURCE_URL_REF_AUTHOR_ID,
                    "selected source URL ref",
                    selected_source_url_ref.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_FOLDER_REF_KIND_AUTHOR_ID,
                    "selected folder refKind",
                    selected_folder_ref_kind.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_SOURCE_URL_REF_KIND_AUTHOR_ID,
                    "selected source URL refKind",
                    selected_source_url_ref_kind.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_FOLDER_REF_ORIGIN_AUTHOR_ID,
                    "selected folder ref origin",
                    selected_source_path_origin.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_SOURCE_URL_REF_ORIGIN_AUTHOR_ID,
                    "selected source URL ref origin",
                    selected_source_url_origin.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_ALBUM_ATTRIBUTION_AUTHOR_ID,
                    "selected album attribution",
                    selected_album_attribution.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_MEDIA_ATTRIBUTION_AUTHOR_ID,
                    "selected media attribution",
                    selected_media_attribution.as_str(),
                ),
                (
                    ATELIER_CKC_SELECTED_MEDIA_STATUS_AUTHOR_ID,
                    "selected media status",
                    selected_media_status.as_str(),
                ),
            ] {
                let response = ui.label(
                    egui::RichText::new(format!("{label}: {value}")).color(palette.text_subtle),
                );
                emit_value_node(
                    ui.ctx(),
                    response.id,
                    accesskit::Role::Label,
                    author_id,
                    label,
                    value,
                );
            }
        });
        let mut pending_selection = None;
        let list_response = ui
            .vertical(|ui| {
                if character.media_albums.is_empty() {
                    ui.label(egui::RichText::new("No linked albums").color(palette.text_subtle));
                }
                for album in &character.media_albums {
                    let album_ref = AtelierRef::media_album(&album.collection_ref, &album.name);
                    debug_assert_eq!(album_ref.item_kind, AtelierItemKind::MediaAlbum);
                    let album_selected =
                        selected_album_collection_id == Some(album.collection_id.as_str());
                    let album_author_id = ckc_media_album_row_author_id(&album.collection_id);
                    let album_payload = DragPayload::AtelierRef(album_ref.clone());
                    let album_drag = ui
                        .dnd_drag_source(egui::Id::new(&album_author_id), album_payload, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} Album: {} ({} items created_by={} updated_by={})",
                                    if album_selected { "*" } else { "" },
                                    album.name,
                                    album.member_count,
                                    album.created_by,
                                    album.updated_by
                                ))
                                .color(palette.text),
                            );
                        })
                        .response;
                    let album_row = ui.interact(
                        album_drag.rect,
                        egui::Id::new(&album_author_id),
                        egui::Sense::click_and_drag(),
                    );
                    emit_draggable_list_item_node(
                        ui.ctx(),
                        album_row.id,
                        &album_author_id,
                        &format!("{} {}", album_ref.ref_kind(), album.collection_ref),
                        &format!(
                            "draggable; atelier-ref {}:{} created_by={} updated_by={}",
                            album_ref.ref_kind(),
                            album_ref.item_id,
                            album.created_by,
                            album.updated_by
                        ),
                        album_selected,
                    );
                    if album_row.clicked() {
                        pending_album_selection = Some(album.collection_id.clone());
                    }
                    if !album.description.is_empty() {
                        ui.label(
                            egui::RichText::new(&album.description).color(palette.text_subtle),
                        );
                    }
                    if !album.tags.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("album tags: {}", album.tags.join(", ")))
                                .color(palette.text_subtle),
                        );
                    }
                    let page_state_author_id =
                        ckc_media_album_page_state_author_id(&album.collection_id);
                    let next_offset_label = album
                        .members_next_offset
                        .map(|offset| offset.to_string())
                        .unwrap_or_else(|| "none".to_owned());
                    let page_state = format!(
                        "loaded={} total={} next_offset={} pending={} final_page={}",
                        album.members.len(),
                        album.member_count,
                        next_offset_label,
                        album_page_pending
                            && album_page_pending_collection_id
                                == Some(album.collection_id.as_str()),
                        album.members_next_offset.is_none()
                    );
                    let page_state_response = ui.label(
                        egui::RichText::new(format!("album page: {page_state}"))
                            .color(palette.text_subtle),
                    );
                    emit_value_node(
                        ui.ctx(),
                        page_state_response.id,
                        accesskit::Role::Label,
                        &page_state_author_id,
                        "CKC album loaded/total/next-offset page state",
                        &page_state,
                    );
                    if let Some(next_offset) = album.members_next_offset {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "showing {} of {}; next offset {}",
                                    album.members.len(),
                                    album.member_count,
                                    next_offset
                                ))
                                .color(palette.text_subtle),
                            );
                            let load_more_author_id =
                                ckc_media_album_load_more_author_id(&album.collection_id);
                            let load_more =
                                ui.add_enabled(!album_page_pending, egui::Button::new("Load more"));
                            emit_node(
                                ui.ctx(),
                                load_more.id,
                                accesskit::Role::Button,
                                &load_more_author_id,
                                "Load the next CKC album media page",
                                album_page_pending,
                            );
                            if load_more.clicked() {
                                pending_album_page = Some(CkcAlbumPageRequest {
                                    collection_id: album.collection_id.clone(),
                                    offset: next_offset,
                                });
                            }
                        });
                    }
                    for member in &album.members {
                        let media_key =
                            ckc_media_occurrence_key(&album.collection_id, &member.asset_id);
                        let selected = resolved_selection.as_deref() == Some(media_key.as_str());
                        let media_author_id =
                            ckc_media_row_author_id(&album.collection_id, &member.asset_id);
                        let media_ref = AtelierRef::new(
                            member.media_ref.clone(),
                            AtelierItemKind::Media,
                            member.display_label.clone(),
                        );
                        let media_payload = DragPayload::AtelierRef(media_ref.clone());
                        let media_drag = ui
                            .dnd_drag_source(egui::Id::new(&media_author_id), media_payload, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}{} [order={} status={} path_origin={} url_origin={} link_path_status={} link_url_status={} asset_path_status={} asset_url_status={} linked_by={} member_updated_by={} notes_updated_by={}]",
                                        if selected { "*" } else { "" },
                                        member.display_label,
                                        member.sort_order,
                                        member.review_status.as_deref().unwrap_or("unreviewed"),
                                        member.source_path_ref_origin,
                                        member.source_url_ref_origin,
                                        member.link_source_path_ref_status,
                                        member.link_source_url_ref_status,
                                        member.asset_source_path_ref_status,
                                        member.asset_source_url_ref_status,
                                        member.linked_by,
                                        member.member_updated_by,
                                        member.notes_updated_by
                                    ))
                                    .color(palette.text),
                                );
                            })
                            .response;
                        let media_row = ui.interact(
                            media_drag.rect,
                            egui::Id::new(&media_author_id),
                            egui::Sense::click_and_drag(),
                        );
                        emit_draggable_list_item_node(
                            ui.ctx(),
                            media_row.id,
                            &media_author_id,
                            &format!(
                                "{} order={} status={} path_origin={} url_origin={} link_path_status={} link_url_status={} asset_path_status={} asset_url_status={} linked_by={} member_updated_by={} member_updated_at_utc={} notes_updated_by={} notes_updated_at_utc={}",
                                member.media_ref,
                                member.sort_order,
                                member.review_status.as_deref().unwrap_or("unreviewed"),
                                member.source_path_ref_origin,
                                member.source_url_ref_origin,
                                member.link_source_path_ref_status,
                                member.link_source_url_ref_status,
                                member.asset_source_path_ref_status,
                                member.asset_source_url_ref_status,
                                member.linked_by,
                                member.member_updated_by,
                                member.member_updated_at_utc.as_deref().unwrap_or("none"),
                                member.notes_updated_by,
                                member.notes_updated_at_utc.as_deref().unwrap_or("none")
                            ),
                            &format!(
                                "draggable; atelier-ref {}:{}",
                                media_ref.ref_kind(),
                                media_ref.item_id
                            ),
                            selected,
                        );
                        if media_row.clicked() {
                            pending_selection = Some(media_key);
                            pending_album_selection = Some(album.collection_id.clone());
                        }
                        if let Some(folder_ref) = &member.source_path_ref {
                            let folder_author_id = ckc_folder_row_author_id(
                                &album.collection_id,
                                &member.asset_id,
                                folder_ref,
                            );
                            let folder_ref_value = AtelierRef::new(
                                folder_ref.clone(),
                                AtelierItemKind::Folder,
                                folder_ref.clone(),
                            );
                            let folder_payload = DragPayload::AtelierRef(folder_ref_value.clone());
                            let folder_drag = ui
                                .dnd_drag_source(
                                    egui::Id::new(&folder_author_id),
                                    folder_payload,
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "folder_ref: {folder_ref}"
                                            ))
                                            .color(palette.text_subtle),
                                        );
                                    },
                                )
                                .response;
                            let folder_structured_value = format!(
                                "ref_kind=folder ref={} collection_id={} asset_id={} origin={} link_status={} asset_status={} linked_by={} member_updated_by={} notes_updated_by={}",
                                folder_ref,
                                album.collection_id,
                                member.asset_id,
                                member.source_path_ref_origin,
                                member.link_source_path_ref_status,
                                member.asset_source_path_ref_status,
                                member.linked_by,
                                member.member_updated_by,
                                member.notes_updated_by
                            );
                            emit_draggable_list_item_node(
                                ui.ctx(),
                                folder_drag.id,
                                &folder_author_id,
                                &folder_structured_value,
                                &format!(
                                    "draggable; atelier-ref {}:{}; {}",
                                    folder_ref_value.ref_kind(),
                                    folder_ref_value.item_id,
                                    folder_structured_value
                                ),
                                false,
                            );
                        }
                        if let Some(source_url_ref) = &member.source_url_ref {
                            let source_url_author_id = ckc_source_url_row_author_id(
                                &album.collection_id,
                                &member.asset_id,
                                source_url_ref,
                            );
                            let source_url_ref_value = AtelierRef::new(
                                source_url_ref.clone(),
                                AtelierItemKind::SourceUrl,
                                source_url_ref.clone(),
                            );
                            let source_url_payload =
                                DragPayload::AtelierRef(source_url_ref_value.clone());
                            let source_url_drag = ui
                                .dnd_drag_source(
                                    egui::Id::new(&source_url_author_id),
                                    source_url_payload,
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "source_url_ref: {source_url_ref}"
                                            ))
                                            .color(palette.text_subtle),
                                        );
                                    },
                                )
                                .response;
                            let source_url_structured_value = format!(
                                "ref_kind=source_url ref={} collection_id={} asset_id={} origin={} link_status={} asset_status={} linked_by={} member_updated_by={} notes_updated_by={}",
                                source_url_ref,
                                album.collection_id,
                                member.asset_id,
                                member.source_url_ref_origin,
                                member.link_source_url_ref_status,
                                member.asset_source_url_ref_status,
                                member.linked_by,
                                member.member_updated_by,
                                member.notes_updated_by
                            );
                            emit_draggable_list_item_node(
                                ui.ctx(),
                                source_url_drag.id,
                                &source_url_author_id,
                                &source_url_structured_value,
                                &format!(
                                    "draggable; atelier-ref {}:{}; {}",
                                    source_url_ref_value.ref_kind(),
                                    source_url_ref_value.item_id,
                                    source_url_structured_value
                                ),
                                false,
                            );
                        }
                    }
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            list_response.id,
            accesskit::Role::List,
            ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
            "CKC linked images folders and albums",
            false,
        );

        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("media IDs").color(palette.text_subtle));
            let asset_ids = ui.text_edit_singleline(album_link_asset_ids);
            emit_node(
                ui.ctx(),
                asset_ids.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID,
                "Existing media asset IDs to link into the selected CKC album",
                false,
            );
            ui.label(egui::RichText::new("source path").color(palette.text_subtle));
            let source_path = ui.text_edit_singleline(album_link_source_path_ref);
            emit_node(
                ui.ctx(),
                source_path.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID,
                "Optional link-scoped CKC source path ref",
                false,
            );
            ui.label(egui::RichText::new("source URL").color(palette.text_subtle));
            let source_url = ui.text_edit_singleline(album_link_source_url_ref);
            emit_node(
                ui.ctx(),
                source_url.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
                "Optional link-scoped CKC source URL ref",
                false,
            );
            let link_enabled = !album_link_pending && selected_album_target.is_some();
            let link = ui.add_enabled(link_enabled, egui::Button::new("Link media IDs"));
            emit_node(
                ui.ctx(),
                link.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
                "Link existing media asset IDs into selected CKC album",
                !link_enabled,
            );
            if link.clicked() {
                let asset_ids = ckc_asset_ids_from_buffer(album_link_asset_ids);
                let selected_collection_id = selected_album_target.clone();
                match ckc_album_source_refs_from_inputs(
                    album_link_source_path_ref,
                    album_link_source_url_ref,
                ) {
                    Ok((source_path_ref, source_url_ref)) => {
                        if let (Some(collection_id), false) =
                            (selected_collection_id, asset_ids.is_empty())
                        {
                            if self.ckc_client.is_some() {
                                pending_album_link = Some(CkcAlbumLinkAssetsRequest {
                                    collection_id,
                                    asset_ids,
                                    source_path_ref,
                                    source_url_ref,
                                    actor_id: actor_id_or_default(media_actor_id),
                                });
                            } else if let Some(album) = character
                                .media_albums
                                .iter_mut()
                                .find(|album| album.collection_id == collection_id)
                            {
                                for asset_id in asset_ids {
                                    if album
                                        .members
                                        .iter()
                                        .any(|member| member.asset_id == asset_id)
                                    {
                                        continue;
                                    }
                                    album.members.push(local_ckc_media_member(
                                        &asset_id,
                                        album.members.len() as i64,
                                        source_path_ref.clone(),
                                        source_url_ref.clone(),
                                        media_actor_id,
                                    ));
                                }
                                album.member_count = album.members.len();
                                album.updated_by = actor_id_or_default(media_actor_id);
                                pending_album_selection = Some(album.collection_id.clone());
                                pending_album_status = Some(format!(
                                    "Linked local media into CKC album {} for actor {}. Backend is unavailable; this is not persisted.",
                                    album.collection_id,
                                    actor_id_or_default(media_actor_id)
                                ));
                            }
                        } else if asset_ids.is_empty() {
                            pending_album_status = Some(
                                "No CKC media IDs entered; paste existing atelier://media/{asset_id} refs before linking."
                                    .to_owned(),
                            );
                        } else {
                            pending_album_status = Some(
                                "No CKC album selected or available for media linking.".to_owned(),
                            );
                        }
                    }
                    Err(message) => {
                        pending_album_status = Some(format!(
                            "CKC album link source refs rejected: {message}"
                        ));
                    }
                }
            }
        });

        let selected_media_key = pending_selection
            .as_deref()
            .or(resolved_selection.as_deref());
        let Some((album_idx, member_idx)) =
            character.selected_or_first_media_location(selected_media_key)
        else {
            return (
                None,
                pending_selection,
                pending_album_selection,
                pending_album_create,
                pending_album_link,
                pending_album_unlink,
                pending_album_link_edit,
                pending_album_reorder,
                pending_album_page,
                pending_album_list_page,
                pending_album_status,
            );
        };
        let selected_album_id = character.media_albums[album_idx].collection_id.clone();
        let selected_asset_id = character.media_albums[album_idx].members[member_idx]
            .asset_id
            .clone();
        let selected_occurrence_label = format!(
            "{}::{} order={}",
            selected_album_id,
            selected_asset_id,
            character.media_albums[album_idx].members[member_idx].sort_order
        );
        ui.label(
            egui::RichText::new(format!("selected occurrence: {selected_occurrence_label}"))
                .color(palette.text_subtle),
        );

        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("edit link source path").color(palette.text_subtle));
            let path = ui.text_edit_singleline(album_link_edit_source_path_ref);
            emit_node(
                ui.ctx(),
                path.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_EDIT_SOURCE_PATH_AUTHOR_ID,
                "Edit selected album link source_path_ref only",
                false,
            );
            ui.label(egui::RichText::new("source URL").color(palette.text_subtle));
            let url = ui.text_edit_singleline(album_link_edit_source_url_ref);
            emit_node(
                ui.ctx(),
                url.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_EDIT_SOURCE_URL_AUTHOR_ID,
                "Edit selected album link source_url_ref only",
                false,
            );
            let clear_path = ui.checkbox(album_link_edit_clear_source_path, "clear path");
            emit_node(
                ui.ctx(),
                clear_path.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_ALBUM_LINK_EDIT_CLEAR_SOURCE_PATH_AUTHOR_ID,
                "Clear selected album link source_path_ref",
                false,
            );
            let clear_url = ui.checkbox(album_link_edit_clear_source_url, "clear URL");
            emit_node(
                ui.ctx(),
                clear_url.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_ALBUM_LINK_EDIT_CLEAR_SOURCE_URL_AUTHOR_ID,
                "Clear selected album link source_url_ref",
                false,
            );
            let save_enabled = !album_link_pending;
            let save = ui.add_enabled(save_enabled, egui::Button::new("Update link refs"));
            emit_node(
                ui.ctx(),
                save.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_LINK_EDIT_SAVE_AUTHOR_ID,
                "Update selected CKC album link provenance without changing asset provenance",
                !save_enabled,
            );
            if save.clicked() {
                let source_path_result = if *album_link_edit_clear_source_path {
                    Ok(None)
                } else {
                    ckc_album_source_path_ref_from_input(album_link_edit_source_path_ref)
                };
                let source_url_result = if *album_link_edit_clear_source_url {
                    Ok(None)
                } else {
                    ckc_album_source_url_ref_from_input(album_link_edit_source_url_ref)
                };
                match (source_path_result, source_url_result) {
                    (Ok(source_path_ref), Ok(source_url_ref)) => {
                        if source_path_ref.is_none()
                            && source_url_ref.is_none()
                            && !*album_link_edit_clear_source_path
                            && !*album_link_edit_clear_source_url
                        {
                            pending_album_status = Some(
                                "No selected CKC album link provenance change requested."
                                    .to_owned(),
                            );
                        } else if self.ckc_client.is_some() {
                            pending_album_link_edit = Some(CkcAlbumLinkEditRequest {
                                collection_id: selected_album_id.clone(),
                                asset_id: selected_asset_id.clone(),
                                source_path_ref,
                                source_url_ref,
                                clear_source_path_ref: *album_link_edit_clear_source_path,
                                clear_source_url_ref: *album_link_edit_clear_source_url,
                                actor_id: actor_id_or_default(media_actor_id),
                            });
                        } else {
                            let actor_id = actor_id_or_default(media_actor_id);
                            let member =
                                &mut character.media_albums[album_idx].members[member_idx];
                            if *album_link_edit_clear_source_path {
                                member.link_source_path_ref = None;
                                member.source_path_ref = None;
                                member.link_source_path_ref_status = "none".to_owned();
                                member.asset_source_path_ref_status = "none".to_owned();
                                member.source_path_ref_origin = "none".to_owned();
                            } else if source_path_ref.is_some() {
                                member.link_source_path_ref = source_path_ref.clone();
                                member.source_path_ref = source_path_ref;
                                member.link_source_path_ref_status = "present".to_owned();
                                member.source_path_ref_origin = "link".to_owned();
                            }
                            if *album_link_edit_clear_source_url {
                                member.link_source_url_ref = None;
                                member.source_url_ref = None;
                                member.link_source_url_ref_status = "none".to_owned();
                                member.asset_source_url_ref_status = "none".to_owned();
                                member.source_url_ref_origin = "none".to_owned();
                            } else if source_url_ref.is_some() {
                                member.link_source_url_ref = source_url_ref.clone();
                                member.source_url_ref = source_url_ref;
                                member.link_source_url_ref_status = "present".to_owned();
                                member.source_url_ref_origin = "link".to_owned();
                            }
                            member.member_updated_by = actor_id.clone();
                            character.media_albums[album_idx].updated_by = actor_id.clone();
                            pending_album_status = Some(format!(
                                "Updated local CKC album link provenance for media {} in album {} for actor {}. Backend is unavailable; asset-level provenance is unchanged and this is not persisted.",
                                selected_asset_id,
                                selected_album_id,
                                actor_id
                            ));
                        }
                    }
                    (Err(message), _) | (_, Err(message)) => {
                        pending_album_status = Some(format!(
                            "CKC album link source refs rejected: {message}"
                        ));
                    }
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            let move_up_items =
                ckc_album_reorder_items_for_move(&character.media_albums[album_idx], member_idx, -1);
            let move_down_items =
                ckc_album_reorder_items_for_move(&character.media_albums[album_idx], member_idx, 1);
            let reorder_has_full_member_set = character.media_albums[album_idx]
                .members_next_offset
                .is_none()
                && character.media_albums[album_idx].members.len()
                    == character.media_albums[album_idx].member_count;
            let move_disabled_description = if reorder_has_full_member_set {
                "Move selected CKC album media by sending a full dense album order"
            } else {
                "Load all CKC album media before reordering; full dense album order requires the complete member set"
            };
            let move_up =
                ui.add_enabled(!album_link_pending && move_up_items.is_some(), egui::Button::new("Move up"));
            emit_node(
                ui.ctx(),
                move_up.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_MOVE_UP_AUTHOR_ID,
                move_disabled_description,
                album_link_pending || move_up_items.is_none(),
            );
            if move_up.clicked() {
                if let Some(items) = move_up_items {
                    if self.ckc_client.is_some() {
                        pending_album_reorder = Some(CkcAlbumReorderRequest {
                            collection_id: selected_album_id.clone(),
                            items,
                            actor_id: actor_id_or_default(media_actor_id),
                        });
                    } else {
                        let actor_id = actor_id_or_default(media_actor_id);
                        apply_ckc_album_dense_order(&mut character.media_albums[album_idx], &items);
                        character.media_albums[album_idx].updated_by = actor_id.clone();
                        for member in &mut character.media_albums[album_idx].members {
                            member.member_updated_by = actor_id.clone();
                        }
                        pending_selection = Some(ckc_media_occurrence_key(
                            &selected_album_id,
                            &selected_asset_id,
                        ));
                        pending_album_status = Some(format!(
                            "Reordered local CKC album {selected_album_id}; backend is unavailable and this is not persisted."
                        ));
                        selected_member_invalidated = true;
                    }
                }
            }
            let move_down = ui.add_enabled(
                !album_link_pending && move_down_items.is_some(),
                egui::Button::new("Move down"),
            );
            emit_node(
                ui.ctx(),
                move_down.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_MOVE_DOWN_AUTHOR_ID,
                move_disabled_description,
                album_link_pending || move_down_items.is_none(),
            );
            if move_down.clicked() {
                if let Some(items) = move_down_items {
                    if self.ckc_client.is_some() {
                        pending_album_reorder = Some(CkcAlbumReorderRequest {
                            collection_id: selected_album_id.clone(),
                            items,
                            actor_id: actor_id_or_default(media_actor_id),
                        });
                    } else {
                        let actor_id = actor_id_or_default(media_actor_id);
                        apply_ckc_album_dense_order(&mut character.media_albums[album_idx], &items);
                        character.media_albums[album_idx].updated_by = actor_id.clone();
                        for member in &mut character.media_albums[album_idx].members {
                            member.member_updated_by = actor_id.clone();
                        }
                        pending_selection = Some(ckc_media_occurrence_key(
                            &selected_album_id,
                            &selected_asset_id,
                        ));
                        pending_album_status = Some(format!(
                            "Reordered local CKC album {selected_album_id}; backend is unavailable and this is not persisted."
                        ));
                        selected_member_invalidated = true;
                    }
                }
            }
            let unlink =
                ui.add_enabled(!album_link_pending, egui::Button::new("Unlink from album"));
            emit_node(
                ui.ctx(),
                unlink.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_UNLINK_AUTHOR_ID,
                "Unlink selected media from this CKC album only; media asset, bytes, notes, and tags are preserved",
                album_link_pending,
            );
            if unlink.clicked() {
                if self.ckc_client.is_some() {
                    pending_album_unlink = Some(CkcAlbumUnlinkRequest {
                        collection_id: selected_album_id.clone(),
                        asset_id: selected_asset_id.clone(),
                        actor_id: actor_id_or_default(media_actor_id),
                    });
                } else {
                    let actor_id = actor_id_or_default(media_actor_id);
                    let album = &mut character.media_albums[album_idx];
                    album.members.retain(|member| member.asset_id != selected_asset_id);
                    album.member_count = album.members.len();
                    album.updated_by = actor_id;
                    pending_selection = None;
                    pending_album_status = Some(format!(
                        "Unlinked local media {selected_asset_id} from album {selected_album_id}; media asset, bytes, image notes, and tags are preserved. Backend is unavailable; this is not persisted."
                    ));
                    selected_member_invalidated = true;
                }
            }
        });
        if selected_member_invalidated {
            return (
                None,
                pending_selection,
                pending_album_selection,
                pending_album_create,
                pending_album_link,
                pending_album_unlink,
                pending_album_link_edit,
                pending_album_reorder,
                pending_album_page,
                pending_album_list_page,
                pending_album_status,
            );
        }

        let member = &mut character.media_albums[album_idx].members[member_idx];
        ui.add_space(4.0);
        ui.label(egui::RichText::new("image notes").color(palette.text_subtle));
        let notes = ui.add(
            egui::TextEdit::multiline(&mut member.notes)
                .desired_rows(3)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            notes.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
            "CKC image notes editor",
            false,
        );
        ui.label(egui::RichText::new("image tags").color(palette.text_subtle));
        let tags = ui.text_edit_singleline(&mut member.tags_buffer);
        emit_node(
            ui.ctx(),
            tags.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
            "CKC image tags editor",
            false,
        );
        let save = ui.add_enabled(!media_save_pending, egui::Button::new("Save media notes"));
        emit_node(
            ui.ctx(),
            save.id,
            accesskit::Role::Button,
            ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
            "Save CKC image notes and tags",
            media_save_pending,
        );
        if save.clicked() {
            if self.ckc_client.is_some() {
                (
                    Some(CkcMediaSaveRequest {
                        asset_id: member.asset_id.clone(),
                        notes: member.notes.clone(),
                        tags: ckc_tags_from_buffer(&member.tags_buffer),
                        review_status: member.review_status.clone(),
                        actor_id: actor_id_or_default(media_actor_id),
                    }),
                    pending_selection,
                    pending_album_selection,
                    pending_album_create,
                    pending_album_link,
                    pending_album_unlink,
                    pending_album_link_edit,
                    pending_album_reorder,
                    pending_album_page,
                    pending_album_list_page,
                    pending_album_status,
                )
            } else {
                let actor_id = actor_id_or_default(media_actor_id);
                member.notes_updated_by = actor_id.clone();
                member.notes_updated_at_utc = None;
                pending_album_status = Some(format!(
                    "Edited local CKC media notes/tags for actor {}. Backend is unavailable; this is not persisted.",
                    actor_id
                ));
                (
                    None,
                    pending_selection,
                    pending_album_selection,
                    pending_album_create,
                    pending_album_link,
                    pending_album_unlink,
                    pending_album_link_edit,
                    pending_album_reorder,
                    pending_album_page,
                    pending_album_list_page,
                    pending_album_status,
                )
            }
        } else {
            (
                None,
                pending_selection,
                pending_album_selection,
                pending_album_create,
                pending_album_link,
                pending_album_unlink,
                pending_album_link_edit,
                pending_album_reorder,
                pending_album_page,
                pending_album_list_page,
                pending_album_status,
            )
        }
    }

    /// Supply (or clear) the raw bytes of the Posekit source image shown in the left viewport. Real
    /// image bytes (PNG/JPEG/WebP/GIF) are decoded and uploaded lazily on the next render. Passing
    /// `None` clears the image and returns the viewport to its explicit empty state. This is the seam a
    /// future DAM/`ckc_client` source-asset fetch (or a test) uses to feed real bytes in.
    pub fn set_pose_source_image_bytes(&self, bytes: Option<Vec<u8>>) {
        Self::set_pose_image_cache_bytes(&self.pose_source_image, bytes);
    }

    fn set_pose_openpose_image_bytes(&self, bytes: Option<Vec<u8>>) {
        Self::set_pose_image_cache_bytes(&self.pose_openpose_image, bytes);
    }

    fn set_pose_image_cache_bytes(cache: &Mutex<PoseSourceImageCache>, bytes: Option<Vec<u8>>) {
        if let Ok(mut cache) = cache.lock() {
            if cache.bytes != bytes {
                cache.bytes = bytes;
                // Force a re-decode/upload next frame.
                cache.hash = None;
                cache.texture = None;
                cache.dims = None;
                cache.error = None;
            }
        }
    }

    /// Decode + upload the cached source-image bytes into a GPU texture (once per byte change) and
    /// report what the left viewport should paint. Real bytes → `Loaded`; a decode failure → an
    /// explicit `DecodeError`; no bytes → `Empty`. Never returns a fabricated placeholder.
    fn prepare_pose_source_render(&self, ctx: &egui::Context) -> PoseSourceRender {
        Self::prepare_pose_image_render(&self.pose_source_image, ctx, "atelier-pose-source-image")
    }

    fn prepare_pose_openpose_png_render(&self, ctx: &egui::Context) -> PoseSourceRender {
        Self::prepare_pose_image_render(&self.pose_openpose_image, ctx, "atelier-pose-openpose-png")
    }

    fn prepare_pose_image_render(
        cache: &Mutex<PoseSourceImageCache>,
        ctx: &egui::Context,
        texture_name: &str,
    ) -> PoseSourceRender {
        let Ok(mut cache) = cache.lock() else {
            return PoseSourceRender::Empty;
        };
        let Some(bytes) = cache.bytes.clone() else {
            cache.hash = None;
            cache.texture = None;
            cache.dims = None;
            cache.error = None;
            return PoseSourceRender::Empty;
        };
        let hash = stable_bytes_hash(&bytes);
        let needs_decode =
            cache.hash != Some(hash) || (cache.texture.is_none() && cache.error.is_none());
        if needs_decode {
            match image::load_from_memory(&bytes) {
                Ok(dynamic) => {
                    let rgba = dynamic.to_rgba8();
                    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
                    let handle =
                        ctx.load_texture(texture_name, color, egui::TextureOptions::LINEAR);
                    cache.texture = Some(handle);
                    cache.dims = Some((w, h));
                    cache.error = None;
                    cache.hash = Some(hash);
                }
                Err(err) => {
                    cache.texture = None;
                    cache.dims = None;
                    cache.error = Some(err.to_string());
                    cache.hash = Some(hash);
                }
            }
        }
        if let Some(err) = cache.error.clone() {
            return PoseSourceRender::DecodeError(err);
        }
        match (cache.texture.clone(), cache.dims) {
            (Some(texture), Some((width, height))) => PoseSourceRender::Loaded {
                texture,
                width,
                height,
                byte_len: bytes.len(),
            },
            _ => PoseSourceRender::Empty,
        }
    }

    fn show_posekit(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        self.drain_posekit_export_backend();
        // MT-043: deliver any fetched source-image bytes to the decode cache BEFORE preparing the render
        // so freshly-arrived bytes paint the same frame.
        self.drain_pose_source_bytes_backend();
        // MT-050: same delivery rule for the exported OpenPose PNG artifact bytes that drive the right
        // viewport once a backend export returns an ArtifactStore `png_artifact_ref`.
        self.drain_pose_export_png_bytes_backend();
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        // MT-043: when the source ref changed, (re)dispatch the byte fetch or clear to empty state.
        self.sync_pose_source_fetch(&mut state);
        self.sync_pose_export_png_fetch(&mut state);
        drop(state);
        let source_render = self.prepare_pose_source_render(ui.ctx());
        let openpose_png_render = self.prepare_pose_openpose_png_render(ui.ctx());
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Source image").color(palette.text));
            let source = ui.text_edit_singleline(&mut state.pose_source_ref);
            emit_value_node(
                ui.ctx(),
                source.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_SOURCE_REF_AUTHOR_ID,
                "Posekit source image ref",
                &state.pose_source_ref,
            );
            ui.label(egui::RichText::new("Rig id").color(palette.text));
            let rig = ui.text_edit_singleline(&mut state.pose_rig_id);
            emit_value_node(
                ui.ctx(),
                rig.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_RIG_ID_AUTHOR_ID,
                "Posekit stored rig id",
                &posekit_optional_rig_id(&state.pose_rig_id).unwrap_or_else(|| "<none>".to_owned()),
            );
        });
        let readout = posekit_state_readout(&state);
        let readout_response = ui.label(egui::RichText::new(&readout).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            readout_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_STATE_READOUT_AUTHOR_ID,
            "Posekit current pose state",
            &readout,
        );
        // MT-043: source-image byte-fetch status (distinguishes an unset ref, an unresolvable ref, a
        // pending fetch, a load, and a fetch failure — the source viewport node alone cannot).
        let source_status = state.pose_source_status.clone();
        let source_status_response =
            ui.label(egui::RichText::new(&source_status).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            source_status_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_SOURCE_STATUS_AUTHOR_ID,
            "Posekit source image byte-fetch status",
            &source_status,
        );
        let openpose_png_status = state.pose_export_png_status.clone();
        let openpose_png_status_response =
            ui.label(egui::RichText::new(&openpose_png_status).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            openpose_png_status_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_OPENPOSE_PNG_STATUS_AUTHOR_ID,
            "Posekit exported OpenPose PNG byte-fetch status",
            &openpose_png_status,
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let yaw_minus = ui.button("Yaw -15");
            emit_node(
                ui.ctx(),
                yaw_minus.id,
                accesskit::Role::Button,
                ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
                "Yaw -15",
                false,
            );
            if yaw_minus.clicked() {
                state.pose_yaw = (state.pose_yaw - 15.0).max(-180.0);
            }
            let yaw_plus = ui.button("Yaw +15");
            emit_node(
                ui.ctx(),
                yaw_plus.id,
                accesskit::Role::Button,
                ATELIER_POSE_YAW_PLUS_AUTHOR_ID,
                "Yaw +15",
                false,
            );
            if yaw_plus.clicked() {
                state.pose_yaw = (state.pose_yaw + 15.0).min(180.0);
            }
            let reset = ui.button("Reset");
            emit_node(
                ui.ctx(),
                reset.id,
                accesskit::Role::Button,
                ATELIER_POSE_RESET_AUTHOR_ID,
                "Reset pose",
                false,
            );
            if reset.clicked() {
                state.pose_yaw = 0.0;
                state.pose_pitch = 0.0;
                state.pose_zoom = 1.0;
                state.pose_face = true;
                state.pose_body = true;
                state.pose_hands = false;
                state.pose_export_pending = false;
                state.pose_active_export_request = None;
                state.pose_last_export = None;
                state.pose_marker_family = "face".to_owned();
                state.pose_marker_index = 12;
                state.pose_marker_x = 321.0;
                state.pose_marker_y = 222.0;
                state.pose_marker_confidence = 0.87;
                state.pose_marker_edits.clear();
                state.pose_marker_status =
                    "Pose reset; marker edits cleared and ready for the next export.".to_owned();
                state.pose_framing_preset = "standard".to_owned();
                state.pose_framing_lens_mm = 50;
                state.pose_framing_padding_top_px = 0;
                state.pose_framing_padding_right_px = 0;
                state.pose_framing_padding_bottom_px = 0;
                state.pose_framing_padding_left_px = 0;
                state.pose_export_status =
                    "Pose reset; export again to refresh OpenPose artifact metadata.".to_owned();
            }
            ui.separator();
            let face = ui.checkbox(&mut state.pose_face, "Face");
            emit_node(
                ui.ctx(),
                face.id,
                accesskit::Role::CheckBox,
                ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
                "Face markers",
                state.pose_face,
            );
            let body = ui.checkbox(&mut state.pose_body, "Body");
            emit_node(
                ui.ctx(),
                body.id,
                accesskit::Role::CheckBox,
                ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID,
                "Body markers",
                state.pose_body,
            );
            let hands = ui.checkbox(&mut state.pose_hands, "Hands");
            emit_node(
                ui.ctx(),
                hands.id,
                accesskit::Role::CheckBox,
                ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID,
                "Hand markers",
                state.pose_hands,
            );
            if face.changed() || body.changed() || hands.changed() {
                posekit_warn_for_disabled_staged_marker_edits(&mut state);
            }
        });
        // Real egui sliders (MT-014 de-scaffold: these were text inputs wearing `-slider` author_ids).
        // Each stays Argus-steerable: `emit_pose_slider_node` publishes a `Role::Slider` node with the
        // stable author_id plus a numeric value/min/max/step and the `SetValue` action, and egui's
        // built-in slider consumes the numeric `Action::SetValue` an out-of-process model dispatches
        // via `argus.set_value` (see `mcp::action::UiAction::SetSliderValue`).
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Yaw").color(palette.text));
            let mut yaw = state.pose_yaw;
            let yaw_resp = ui.add(
                egui::Slider::new(&mut yaw, -180.0..=180.0)
                    .step_by(1.0)
                    .fixed_decimals(0),
            );
            if yaw_resp.changed() {
                state.pose_yaw = yaw.clamp(-180.0, 180.0);
            }
            emit_pose_slider_node(
                ui.ctx(),
                yaw_resp.id,
                ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
                "Posekit yaw degrees",
                state.pose_yaw as f64,
                -180.0,
                180.0,
                1.0,
            );
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Pitch").color(palette.text));
            let mut pitch = state.pose_pitch;
            let pitch_resp = ui.add(
                egui::Slider::new(&mut pitch, -45.0..=45.0)
                    .step_by(1.0)
                    .fixed_decimals(0),
            );
            if pitch_resp.changed() {
                state.pose_pitch = pitch.clamp(-45.0, 45.0);
            }
            emit_pose_slider_node(
                ui.ctx(),
                pitch_resp.id,
                ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
                "Posekit pitch degrees",
                state.pose_pitch as f64,
                -45.0,
                45.0,
                1.0,
            );
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Zoom").color(palette.text));
            let mut zoom = state.pose_zoom;
            let zoom_resp = ui.add(
                egui::Slider::new(&mut zoom, 0.4..=2.2)
                    .step_by(0.01)
                    .fixed_decimals(2),
            );
            if zoom_resp.changed() {
                state.pose_zoom = zoom.clamp(0.4, 2.2);
            }
            emit_pose_slider_node(
                ui.ctx(),
                zoom_resp.id,
                ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
                "Posekit zoom",
                state.pose_zoom as f64,
                0.4,
                2.2,
                0.01,
            );
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Marker").color(palette.text));
            let family = ui.text_edit_singleline(&mut state.pose_marker_family);
            emit_value_node(
                ui.ctx(),
                family.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_FAMILY_AUTHOR_ID,
                "Posekit marker family",
                &state.pose_marker_family,
            );

            ui.label(egui::RichText::new("Index").color(palette.text));
            let mut index_text = state.pose_marker_index.to_string();
            let index = ui.text_edit_singleline(&mut index_text);
            if index.changed() {
                if let Ok(value) = index_text.trim().parse::<i32>() {
                    state.pose_marker_index = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                index.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_INDEX_AUTHOR_ID,
                "Posekit marker index",
                &state.pose_marker_index.to_string(),
            );

            ui.label(egui::RichText::new("X").color(palette.text));
            let mut x_text = format!("{:.1}", state.pose_marker_x);
            let x = ui.text_edit_singleline(&mut x_text);
            if x.changed() {
                if let Ok(value) = x_text.trim().parse::<f32>() {
                    state.pose_marker_x = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                x.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_X_AUTHOR_ID,
                "Posekit marker x coordinate",
                &format!("{:.1}", state.pose_marker_x),
            );

            ui.label(egui::RichText::new("Y").color(palette.text));
            let mut y_text = format!("{:.1}", state.pose_marker_y);
            let y = ui.text_edit_singleline(&mut y_text);
            if y.changed() {
                if let Ok(value) = y_text.trim().parse::<f32>() {
                    state.pose_marker_y = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                y.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_Y_AUTHOR_ID,
                "Posekit marker y coordinate",
                &format!("{:.1}", state.pose_marker_y),
            );

            ui.label(egui::RichText::new("Conf").color(palette.text));
            let mut confidence_text = format!("{:.2}", state.pose_marker_confidence);
            let confidence = ui.text_edit_singleline(&mut confidence_text);
            if confidence.changed() {
                if let Ok(value) = confidence_text.trim().parse::<f32>() {
                    state.pose_marker_confidence = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                confidence.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_CONFIDENCE_AUTHOR_ID,
                "Posekit marker confidence",
                &format!("{:.2}", state.pose_marker_confidence),
            );
        });
        ui.horizontal_wrapped(|ui| {
            let nudge_left = ui.button("<");
            emit_node(
                ui.ctx(),
                nudge_left.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_LEFT_AUTHOR_ID,
                "Nudge marker left",
                false,
            );
            if nudge_left.clicked() {
                posekit_nudge_marker(&mut state, -1.0, 0.0);
            }
            let nudge_right = ui.button(">");
            emit_node(
                ui.ctx(),
                nudge_right.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_RIGHT_AUTHOR_ID,
                "Nudge marker right",
                false,
            );
            if nudge_right.clicked() {
                posekit_nudge_marker(&mut state, 1.0, 0.0);
            }
            let nudge_up = ui.button("^");
            emit_node(
                ui.ctx(),
                nudge_up.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_UP_AUTHOR_ID,
                "Nudge marker up",
                false,
            );
            if nudge_up.clicked() {
                posekit_nudge_marker(&mut state, 0.0, -1.0);
            }
            let nudge_down = ui.button("v");
            emit_node(
                ui.ctx(),
                nudge_down.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_DOWN_AUTHOR_ID,
                "Nudge marker down",
                false,
            );
            if nudge_down.clicked() {
                posekit_nudge_marker(&mut state, 0.0, 1.0);
            }

            let apply = ui.button("Apply");
            emit_node(
                ui.ctx(),
                apply.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_APPLY_AUTHOR_ID,
                "Apply marker edit",
                false,
            );
            if apply.clicked() {
                posekit_stage_marker_edit(&mut state, "set");
            }
            let add = ui.button("Add");
            emit_node(
                ui.ctx(),
                add.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_ADD_AUTHOR_ID,
                "Add marker into empty slot",
                false,
            );
            if add.clicked() {
                posekit_stage_marker_edit(&mut state, "add");
            }
            let remove = ui.button("Remove");
            emit_node(
                ui.ctx(),
                remove.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_REMOVE_AUTHOR_ID,
                "Remove marker",
                false,
            );
            if remove.clicked() {
                posekit_stage_marker_edit(&mut state, "remove");
            }
            let reset_marker = ui.button("Clear edits");
            emit_node(
                ui.ctx(),
                reset_marker.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_RESET_AUTHOR_ID,
                "Clear staged marker edits",
                false,
            );
            if reset_marker.clicked() {
                state.pose_marker_edits.clear();
                state.pose_marker_status =
                    "Posekit marker edits cleared; last export remains visible until refreshed."
                        .to_owned();
            }
        });
        let marker_status = state.pose_marker_status.clone();
        let marker_status_response =
            ui.label(egui::RichText::new(&marker_status).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            marker_status_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_MARKER_STATUS_AUTHOR_ID,
            "Posekit marker edit status",
            &marker_status,
        );

        egui::Grid::new("atelier-pose-framing-grid")
            .num_columns(4)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Framing").color(palette.text));
                let preset = ui.add(
                    egui::TextEdit::singleline(&mut state.pose_framing_preset).desired_width(110.0),
                );
                emit_value_node(
                    ui.ctx(),
                    preset.id,
                    accesskit::Role::TextInput,
                    ATELIER_POSE_FRAMING_PRESET_AUTHOR_ID,
                    "Posekit framing preset",
                    &posekit_framing_preset(&state.pose_framing_preset),
                );

                ui.label(egui::RichText::new("Lens mm").color(palette.text));
                let mut lens_text = state.pose_framing_lens_mm.to_string();
                let lens = ui.add(egui::TextEdit::singleline(&mut lens_text).desired_width(48.0));
                if lens.changed() {
                    if let Ok(value) = lens_text.trim().parse::<i32>() {
                        state.pose_framing_lens_mm = value.clamp(18, 120);
                    }
                }
                emit_value_node(
                    ui.ctx(),
                    lens.id,
                    accesskit::Role::TextInput,
                    ATELIER_POSE_FRAMING_LENS_AUTHOR_ID,
                    "Posekit framing lens millimeters",
                    &state.pose_framing_lens_mm.clamp(18, 120).to_string(),
                );
                ui.end_row();

                ui.label(egui::RichText::new("Top").color(palette.text));
                let mut top_text = state.pose_framing_padding_top_px.to_string();
                let top = ui.add(egui::TextEdit::singleline(&mut top_text).desired_width(48.0));
                if top.changed() {
                    if let Ok(value) = top_text.trim().parse::<i32>() {
                        state.pose_framing_padding_top_px = value.clamp(0, 256);
                    }
                }
                emit_value_node(
                    ui.ctx(),
                    top.id,
                    accesskit::Role::TextInput,
                    ATELIER_POSE_FRAMING_PADDING_TOP_AUTHOR_ID,
                    "Posekit framing top padding pixels",
                    &state.pose_framing_padding_top_px.clamp(0, 256).to_string(),
                );

                ui.label(egui::RichText::new("Right").color(palette.text));
                let mut right_text = state.pose_framing_padding_right_px.to_string();
                let right = ui.add(egui::TextEdit::singleline(&mut right_text).desired_width(48.0));
                if right.changed() {
                    if let Ok(value) = right_text.trim().parse::<i32>() {
                        state.pose_framing_padding_right_px = value.clamp(0, 256);
                    }
                }
                emit_value_node(
                    ui.ctx(),
                    right.id,
                    accesskit::Role::TextInput,
                    ATELIER_POSE_FRAMING_PADDING_RIGHT_AUTHOR_ID,
                    "Posekit framing right padding pixels",
                    &state
                        .pose_framing_padding_right_px
                        .clamp(0, 256)
                        .to_string(),
                );
                ui.end_row();

                ui.label(egui::RichText::new("Bottom").color(palette.text));
                let mut bottom_text = state.pose_framing_padding_bottom_px.to_string();
                let bottom =
                    ui.add(egui::TextEdit::singleline(&mut bottom_text).desired_width(48.0));
                if bottom.changed() {
                    if let Ok(value) = bottom_text.trim().parse::<i32>() {
                        state.pose_framing_padding_bottom_px = value.clamp(0, 256);
                    }
                }
                emit_value_node(
                    ui.ctx(),
                    bottom.id,
                    accesskit::Role::TextInput,
                    ATELIER_POSE_FRAMING_PADDING_BOTTOM_AUTHOR_ID,
                    "Posekit framing bottom padding pixels",
                    &state
                        .pose_framing_padding_bottom_px
                        .clamp(0, 256)
                        .to_string(),
                );

                ui.label(egui::RichText::new("Left").color(palette.text));
                let mut left_text = state.pose_framing_padding_left_px.to_string();
                let left = ui.add(egui::TextEdit::singleline(&mut left_text).desired_width(48.0));
                if left.changed() {
                    if let Ok(value) = left_text.trim().parse::<i32>() {
                        state.pose_framing_padding_left_px = value.clamp(0, 256);
                    }
                }
                emit_value_node(
                    ui.ctx(),
                    left.id,
                    accesskit::Role::TextInput,
                    ATELIER_POSE_FRAMING_PADDING_LEFT_AUTHOR_ID,
                    "Posekit framing left padding pixels",
                    &state.pose_framing_padding_left_px.clamp(0, 256).to_string(),
                );
                ui.end_row();
            });
        let framing_readout = posekit_framing_readout(&state);
        let framing_response =
            ui.label(egui::RichText::new(&framing_readout).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            framing_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_FRAMING_READOUT_AUTHOR_ID,
            "Posekit export framing readout",
            &framing_readout,
        );
        ui.separator();
        // Bind the right viewport to the REAL generated OpenPose keypoints (single source of truth):
        // prefer the backend export's `openpose_json` when one exists; otherwise render the live
        // offline-preview keypoints produced by the SAME `posekit_openpose_json` the local export path
        // uses. Either way the viewport paints the genuine keypoints that would be exported, not a
        // hand-authored schematic.
        let (openpose_render_json, openpose_bound_source) =
            if let Some(snapshot) = state.pose_last_export.as_ref() {
                (snapshot.openpose_json.clone(), "backend_export")
            } else {
                let marker_edits = posekit_marker_edits_json(&state.pose_marker_edits);
                let framing = posekit_framing_json_from_state(&state);
                let json = posekit_openpose_json(
                    &state.pose_source_ref,
                    posekit_optional_rig_id(&state.pose_rig_id).as_deref(),
                    state.pose_yaw,
                    state.pose_pitch,
                    state.pose_zoom,
                    state.pose_face,
                    state.pose_body,
                    state.pose_hands,
                    &marker_edits,
                    &framing,
                );
                (json, "live_preview")
            };
        let openpose_png_artifact_ref = state
            .pose_last_export
            .as_ref()
            .map(|snapshot| snapshot.png_artifact_ref.as_str());
        let split = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID),
                |ui| {
                    ui.columns(2, |cols| {
                        draw_pose_source_view(
                            &mut cols[0],
                            palette,
                            ATELIER_POSE_3D_VIEWPORT_AUTHOR_ID,
                            &state.pose_source_ref,
                            posekit_optional_rig_id(&state.pose_rig_id).as_deref(),
                            &source_render,
                        );
                        draw_pose_openpose_view(
                            &mut cols[1],
                            palette,
                            ATELIER_POSE_OPENPOSE_VIEWPORT_AUTHOR_ID,
                            &openpose_render_json,
                            openpose_bound_source,
                            &state.pose_source_ref,
                            posekit_optional_rig_id(&state.pose_rig_id).as_deref(),
                            state.pose_yaw,
                            state.pose_pitch,
                            state.pose_zoom,
                            state.pose_face,
                            state.pose_body,
                            state.pose_hands,
                            &openpose_png_render,
                            openpose_png_artifact_ref,
                            &openpose_png_status,
                        );
                    });
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            split.id,
            accesskit::Role::Group,
            ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID,
            "Posekit rig/OpenPose split view",
            false,
        );
        ui.separator();
        let export = ui.add_enabled(
            !state.pose_export_pending,
            egui::Button::new("Export OpenPose"),
        );
        emit_node(
            ui.ctx(),
            export.id,
            accesskit::Role::Button,
            ATELIER_POSE_EXPORT_AUTHOR_ID,
            "Export ComfyUI-ready OpenPose",
            state.pose_export_pending,
        );
        if export.clicked() {
            if !(state.pose_face || state.pose_body || state.pose_hands) {
                state.pose_export_status =
                    "Posekit OpenPose export failed: enable at least one marker layer.".to_owned();
            } else if state.pose_source_ref.trim().is_empty()
                || state.pose_source_ref.trim() != state.pose_source_ref
            {
                state.pose_export_status =
                    "Posekit OpenPose export failed: source_ref must be non-empty and unpadded."
                        .to_owned();
            } else if let Some(client) = self.ckc_client.as_ref() {
                match posekit_validate_staged_marker_edits_for_export(&state, true) {
                    Ok(()) => {
                        let rig_id = posekit_optional_rig_id(&state.pose_rig_id);
                        let expected_thread_id = posekit_openpose_model_ops_thread_id(
                            &state.pose_source_ref,
                            rig_id.as_deref(),
                        );
                        let model_ops = match model_ops_mutation_context_from_state(
                            &state,
                            client.actor_id(),
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => model_ops,
                            Err(err) => {
                                state.pose_export_status =
                                    format!("Posekit OpenPose export blocked: {err}");
                                return;
                            }
                        };
                        state.pose_export_request_seq =
                            state.pose_export_request_seq.saturating_add(1);
                        let request_id = state.pose_export_request_seq;
                        state.pose_active_export_request = Some(request_id);
                        state.pose_export_pending = true;
                        state.pose_export_status =
                            "Posekit backend OpenPose export pending; waiting for ArtifactStore refs."
                                .to_owned();
                        let marker_edits = posekit_marker_edits_json(&state.pose_marker_edits);
                        let framing = posekit_framing_json_from_state(&state);
                        client.export_posekit_openpose(
                            &state.pose_source_ref,
                            state.pose_yaw,
                            state.pose_pitch,
                            state.pose_zoom,
                            state.pose_face,
                            state.pose_body,
                            state.pose_hands,
                            rig_id.as_deref(),
                            marker_edits,
                            framing,
                            &model_ops.actor_id,
                            model_ops.context,
                            request_id,
                            self.pose_export_cell.clone(),
                        );
                    }
                    Err(err) => {
                        state.pose_export_status = format!("Posekit OpenPose export failed: {err}");
                    }
                }
            } else {
                match posekit_export_snapshot(&state) {
                    Ok(snapshot) => {
                        state.pose_export_status = format!(
                            "Local Argus preview only (offline preview, NOT a real artifact export; refs use the preview:// scheme only — connect a backend to produce real ArtifactStore refs): yaw_deg={:.0} png_artifact_ref={} json_artifact_ref={} receipt_ref={}",
                            snapshot.yaw_deg,
                            snapshot.png_artifact_ref,
                            snapshot.json_artifact_ref,
                            snapshot.receipt_ref
                        );
                        state.pose_last_export = Some(snapshot);
                    }
                    Err(err) => {
                        state.pose_export_status =
                            format!("Local Posekit OpenPose export failed: {err}");
                    }
                }
            }
        }
        let export_status = state.pose_export_status.clone();
        let status = ui.label(egui::RichText::new(&export_status).color(palette.text));
        emit_value_node(
            ui.ctx(),
            status.id,
            accesskit::Role::Label,
            ATELIER_POSE_EXPORT_STATUS_AUTHOR_ID,
            "Posekit export status",
            &export_status,
        );
        if let Some(snapshot) = state.pose_last_export.as_ref() {
            let export_ref_label = format!(
                "{} {} {} {} {} {}",
                snapshot.png_artifact_ref,
                snapshot.json_artifact_ref,
                snapshot.receipt_ref,
                snapshot.content_hash,
                snapshot.png_manifest_ref,
                snapshot.json_manifest_ref
            );
            let export_ref = ui.label(&export_ref_label);
            emit_value_node(
                ui.ctx(),
                export_ref.id,
                accesskit::Role::Label,
                ATELIER_POSE_EXPORT_REF_AUTHOR_ID,
                "Posekit OpenPose artifact and receipt refs",
                &export_ref_label,
            );
            let mut preview = posekit_export_preview(snapshot);
            let preview_response = ui.add(
                egui::TextEdit::multiline(&mut preview)
                    .desired_rows(8)
                    .interactive(false),
            );
            emit_value_node(
                ui.ctx(),
                preview_response.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_EXPORT_PREVIEW_AUTHOR_ID,
                "Posekit ComfyUI-ready OpenPose export preview",
                &preview,
            );
        }
    }

    /// MT-020: prompt-feedback sub-mode (nested under INGEST). A minimal WIP
    /// surface for the deterministic prompt-feedback kernel: import panel, case
    /// list grouped by segment/cell/render-stack, failure-tag picker, deterministic
    /// rewrite preview + trace, and export button. All controls carry stable
    /// `atelier-promptfeedback-*` Argus author_ids. The determinism itself lives in
    /// the backend engine (`/atelier/prompt-feedback/*`); this panel surfaces the
    /// workflow and the protected-eval invariant.
    fn show_prompt_feedback(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        let header = ui.label(
            egui::RichText::new("Prompt feedback kernel (deterministic rewrite)")
                .color(palette.text)
                .strong(),
        );
        emit_value_node(
            ui.ctx(),
            header.id,
            accesskit::Role::Label,
            ATELIER_PROMPTFEEDBACK_HEADER_AUTHOR_ID,
            "Prompt feedback kernel header",
            "Prompt feedback kernel (deterministic rewrite)",
        );

        // Import panel.
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Project").color(palette.text));
            let project = ui.text_edit_singleline(&mut state.prompt_feedback_project);
            emit_value_node(
                ui.ctx(),
                project.id,
                accesskit::Role::TextInput,
                ATELIER_PROMPTFEEDBACK_PROJECT_AUTHOR_ID,
                "Prompt feedback project id",
                &state.prompt_feedback_project,
            );
            ui.label(egui::RichText::new("Adapter").color(palette.text));
            let adapter = ui.text_edit_singleline(&mut state.prompt_feedback_adapter);
            emit_value_node(
                ui.ctx(),
                adapter.id,
                accesskit::Role::TextInput,
                ATELIER_PROMPTFEEDBACK_ADAPTER_AUTHOR_ID,
                "Prompt feedback adapter id",
                &state.prompt_feedback_adapter,
            );
        });
        let rows = ui.text_edit_multiline(&mut state.prompt_feedback_rows);
        emit_value_node(
            ui.ctx(),
            rows.id,
            accesskit::Role::TextInput,
            ATELIER_PROMPTFEEDBACK_ROWS_AUTHOR_ID,
            "Prompt feedback CUIPP import rows (JSONL)",
            &state.prompt_feedback_rows,
        );
        ui.horizontal(|ui| {
            let import = ui.add_enabled(
                !state.prompt_feedback_import_pending,
                egui::Button::new("Import CUIPP rows"),
            );
            emit_node(
                ui.ctx(),
                import.id,
                accesskit::Role::Button,
                ATELIER_PROMPTFEEDBACK_IMPORT_AUTHOR_ID,
                "Import CUIPP rows",
                state.prompt_feedback_import_pending,
            );
            if import.clicked() {
                if let Some(client) = self.ckc_client.as_ref() {
                    let project_id = state.prompt_feedback_project.trim().to_owned();
                    let adapter_id = state.prompt_feedback_adapter.trim().to_owned();
                    if project_id.is_empty() || adapter_id.is_empty() {
                        state.prompt_feedback_status =
                            "backend_blocked prompt_feedback_import_error=project_or_adapter_empty"
                                .to_owned();
                    } else {
                        match parse_prompt_feedback_rows_input(&state.prompt_feedback_rows) {
                            Ok(rows) => {
                                let expected_thread_id =
                                    prompt_feedback_project_model_ops_thread_id(&project_id);
                                let model_ops = match model_ops_mutation_context_from_state(
                                    &state,
                                    client.actor_id(),
                                    &expected_thread_id,
                                ) {
                                    Ok(model_ops) => model_ops,
                                    Err(err) => {
                                        state.prompt_feedback_status = format!(
                                            "backend_blocked prompt_feedback_import_error={err}"
                                        );
                                        return;
                                    }
                                };
                                let source_system =
                                    prompt_feedback_source_system(&project_id, &adapter_id);
                                state.prompt_feedback_request_seq =
                                    state.prompt_feedback_request_seq.saturating_add(1);
                                let request_id = state.prompt_feedback_request_seq;
                                state.prompt_feedback_import_pending = true;
                                state.prompt_feedback_active_import_request = Some(request_id);
                                state.prompt_feedback_active_import_fingerprint =
                                    Some(prompt_feedback_import_fingerprint(
                                        &project_id,
                                        &adapter_id,
                                        &state.prompt_feedback_rows,
                                    ));
                                state.prompt_feedback_status = format!(
                                    "backend_pending prompt_feedback_import_pending=true request_seq={request_id} project_id={project_id} adapter_id={adapter_id} expected_thread_id={expected_thread_id}"
                                );
                                let actor_id = model_ops.actor_id.clone();
                                let model_ops_context = model_ops.context.clone();
                                client.import_prompt_feedback(
                                    &project_id,
                                    &source_system,
                                    &adapter_id,
                                    Some("ui-jsonl"),
                                    rows,
                                    &actor_id,
                                    model_ops_context,
                                    request_id,
                                    self.prompt_feedback_import_cell.clone(),
                                );
                            }
                            Err(err) => {
                                state.prompt_feedback_status =
                                    format!("backend_blocked prompt_feedback_import_error={err}");
                            }
                        }
                    }
                } else {
                    state.prompt_feedback_status = "Local import preview only: no backend mutation dispatched. Use POST /atelier/prompt-feedback/import with Model Ops headers for persisted PromptCases.".to_owned();
                }
            }
            ui.label(egui::RichText::new("Rule pack").color(palette.text));
            let pack = ui.text_edit_singleline(&mut state.prompt_feedback_rule_pack);
            emit_value_node(
                ui.ctx(),
                pack.id,
                accesskit::Role::TextInput,
                ATELIER_PROMPTFEEDBACK_RULEPACK_AUTHOR_ID,
                "Prompt feedback rule pack id",
                &state.prompt_feedback_rule_pack,
            );
        });

        ui.separator();

        // Case list grouped by segment / cell / render stack.
        let list_header = ui.label(egui::RichText::new("Cases").color(palette.text));
        emit_value_node(
            ui.ctx(),
            list_header.id,
            accesskit::Role::Label,
            ATELIER_PROMPTFEEDBACK_CASE_LIST_AUTHOR_ID,
            "Prompt feedback case list",
            "Cases grouped by segment / cell / render stack",
        );
        let cases = state.prompt_feedback_cases.clone();
        for case in &cases {
            let selected =
                state.prompt_feedback_selected_case.as_deref() == Some(case.case_id.as_str());
            let label = format!(
                "{} / {} / {} : source_case_id={} case_id={} source={} imported_by={}{}",
                case.segment,
                case.cell,
                case.render_stack,
                case.source_case_id,
                case.case_id,
                case.persistence_source,
                case.imported_by,
                if case.identity_judgement_allowed {
                    ""
                } else {
                    "  [prompt-quality only]"
                }
            );
            let row = ui.add(egui::Button::selectable(selected, label.clone()));
            emit_value_node(
                ui.ctx(),
                row.id,
                accesskit::Role::ListItem,
                &prompt_feedback_case_row_author_id(&case.case_id),
                &label,
                &label,
            );
            if row.clicked() {
                state.prompt_feedback_selected_case = Some(case.case_id.clone());
            }
        }

        ui.separator();

        // Failure-tag picker.
        ui.label(egui::RichText::new("Failure tags").color(palette.text));
        ui.horizontal_wrapped(|ui| {
            for tag in PROMPT_FEEDBACK_FAILURE_TAGS {
                let selected = state.prompt_feedback_selected_tags.contains(*tag);
                let button = ui.add(egui::Button::selectable(selected, *tag));
                emit_node(
                    ui.ctx(),
                    button.id,
                    accesskit::Role::Button,
                    &prompt_feedback_failure_tag_author_id(tag),
                    tag,
                    selected,
                );
                if button.clicked() {
                    if selected {
                        state.prompt_feedback_selected_tags.remove(*tag);
                    } else {
                        state
                            .prompt_feedback_selected_tags
                            .insert((*tag).to_owned());
                    }
                }
            }
        });

        ui.separator();

        // Deterministic rewrite preview + trace.
        ui.horizontal(|ui| {
            let rewrite = ui.add_enabled(
                !state.prompt_feedback_rewrite_pending,
                egui::Button::new("Preview deterministic rewrite"),
            );
            emit_node(
                ui.ctx(),
                rewrite.id,
                accesskit::Role::Button,
                ATELIER_PROMPTFEEDBACK_REWRITE_AUTHOR_ID,
                "Preview deterministic rewrite",
                state.prompt_feedback_rewrite_pending,
            );
            if rewrite.clicked() {
                let selected_case = state.prompt_feedback_selected_case.clone();
                let pack = state.prompt_feedback_rule_pack.clone();
                let tags: Vec<String> = state.prompt_feedback_selected_tags.iter().cloned().collect();
                match selected_case
                    .as_ref()
                    .and_then(|id| cases.iter().find(|case| &case.case_id == id))
                {
                    Some(case) => {
                        if let Some(client) = self.ckc_client.as_ref() {
                            if case.persistence_source != "backend_persisted" {
                                state.prompt_feedback_rewrite_preview = format!(
                                    "backend_blocked prompt_feedback_rewrite_error=selected_case_not_persisted source_case_id={} source={}",
                                    case.source_case_id,
                                    case.persistence_source
                                );
                                state.prompt_feedback_trace =
                                    "backend_blocked rule_trace_unavailable=true".to_owned();
                            } else {
                                let expected_thread_id =
                                    prompt_feedback_case_model_ops_thread_id(&case.case_id);
                                let model_ops = match model_ops_mutation_context_from_state(
                                    &state,
                                    client.actor_id(),
                                    &expected_thread_id,
                                ) {
                                    Ok(model_ops) => model_ops,
                                    Err(err) => {
                                        state.prompt_feedback_rewrite_preview = format!(
                                            "backend_blocked prompt_feedback_rewrite_error={err}"
                                        );
                                        state.prompt_feedback_trace =
                                            "backend_blocked rule_trace_unavailable=true"
                                                .to_owned();
                                        return;
                                    }
                                };
                                state.prompt_feedback_request_seq =
                                    state.prompt_feedback_request_seq.saturating_add(1);
                                let request_id = state.prompt_feedback_request_seq;
                                state.prompt_feedback_rewrite_pending = true;
                                state.prompt_feedback_active_rewrite_request = Some(request_id);
                                state.prompt_feedback_active_rewrite_fingerprint =
                                    Some(prompt_feedback_rewrite_fingerprint(
                                        &pack,
                                        Some(&case.case_id),
                                        &state.prompt_feedback_selected_tags,
                                    ));
                                state.prompt_feedback_rewrite_preview = format!(
                                    "backend_pending prompt_feedback_rewrite_pending=true request_seq={request_id} case_id={} source_case_id={} rule_pack_id={} expected_thread_id={expected_thread_id}",
                                    case.case_id,
                                    case.source_case_id,
                                    pack
                                );
                                state.prompt_feedback_trace =
                                    "backend_pending rule_trace_unavailable_until_response=true"
                                        .to_owned();
                                let actor_id = model_ops.actor_id.clone();
                                let model_ops_context = model_ops.context.clone();
                                client.rewrite_prompt_feedback_case(
                                    &case.case_id,
                                    &pack,
                                    Some(1),
                                    &actor_id,
                                    model_ops_context,
                                    request_id,
                                    self.prompt_feedback_rewrite_cell.clone(),
                                );
                            }
                        } else {
                            let protected = case.segment == "standard";
                            state.prompt_feedback_rewrite_preview = format!(
                                "Local deterministic rewrite preview for {} via rule pack {}; no backend mutation dispatched.{}",
                                case.source_case_id,
                                pack,
                                if protected {
                                    " Protected standard row: a prompt-stress tail is hard-rejected (protected_eval_prompt_mutation)."
                                } else {
                                    " Prompt-quality only: this is never identity-success evidence."
                                }
                            );
                            state.prompt_feedback_trace = format!(
                                "local_rule_trace_preview: rule_pack_id={}, tags=[{}]. Backend POST /atelier/prompt-feedback/rewrite returns durable rule_trace when dispatched headlessly with Model Ops headers.",
                                pack,
                                tags.join(", ")
                            );
                        }
                    }
                    None => {
                        state.prompt_feedback_rewrite_preview =
                            "Select a case to preview a deterministic rewrite.".to_owned();
                        state.prompt_feedback_trace = String::new();
                    }
                }
            }
        });
        let preview_text = state.prompt_feedback_rewrite_preview.clone();
        let preview = ui.label(egui::RichText::new(&preview_text).color(palette.text));
        emit_value_node(
            ui.ctx(),
            preview.id,
            accesskit::Role::Label,
            ATELIER_PROMPTFEEDBACK_REWRITE_PREVIEW_AUTHOR_ID,
            "Prompt feedback rewrite preview",
            &preview_text,
        );
        let trace_text = state.prompt_feedback_trace.clone();
        let trace = ui.label(egui::RichText::new(&trace_text).color(palette.text));
        emit_value_node(
            ui.ctx(),
            trace.id,
            accesskit::Role::Label,
            ATELIER_PROMPTFEEDBACK_TRACE_AUTHOR_ID,
            "Prompt feedback rewrite trace",
            &trace_text,
        );

        ui.separator();

        // Export.
        ui.horizontal(|ui| {
            let export = ui.add_enabled(
                !state.prompt_feedback_export_pending,
                egui::Button::new("Export corrected JSONL"),
            );
            emit_node(
                ui.ctx(),
                export.id,
                accesskit::Role::Button,
                ATELIER_PROMPTFEEDBACK_EXPORT_AUTHOR_ID,
                "Export corrected JSONL",
                state.prompt_feedback_export_pending,
            );
            if export.clicked() {
                if let Some(client) = self.ckc_client.as_ref() {
                    let pack = state.prompt_feedback_rule_pack.trim().to_owned();
                    let case_ids = prompt_feedback_export_case_ids(
                        &state.prompt_feedback_cases,
                        state.prompt_feedback_selected_case.as_deref(),
                    );
                    if pack.is_empty() || case_ids.is_empty() {
                        state.prompt_feedback_export_status = format!(
                            "backend_blocked prompt_feedback_export_error=missing_rule_pack_or_backend_cases rule_pack_id={} backend_case_count={}",
                            pack,
                            case_ids.len()
                        );
                    } else {
                        let expected_thread_id =
                            prompt_feedback_export_model_ops_thread_id(&pack, 1, &case_ids);
                        let model_ops = match model_ops_mutation_context_from_state(
                            &state,
                            client.actor_id(),
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => model_ops,
                            Err(err) => {
                                state.prompt_feedback_export_status = format!(
                                    "backend_blocked prompt_feedback_export_error={err}"
                                );
                                return;
                            }
                        };
                        state.prompt_feedback_request_seq =
                            state.prompt_feedback_request_seq.saturating_add(1);
                        let request_id = state.prompt_feedback_request_seq;
                        state.prompt_feedback_export_pending = true;
                        state.prompt_feedback_active_export_request = Some(request_id);
                        state.prompt_feedback_active_export_fingerprint =
                            Some(prompt_feedback_export_fingerprint(&pack, &case_ids));
                        state.prompt_feedback_export_status = format!(
                            "backend_pending prompt_feedback_export_pending=true request_seq={request_id} rule_pack_id={pack} rule_pack_version=1 case_ids={} expected_thread_id={expected_thread_id}",
                            case_ids.join("|")
                        );
                        let actor_id = model_ops.actor_id.clone();
                        let model_ops_context = model_ops.context.clone();
                        client.export_prompt_feedback(
                            &pack,
                            Some(1),
                            &case_ids,
                            &actor_id,
                            model_ops_context,
                            request_id,
                            self.prompt_feedback_export_cell.clone(),
                        );
                    }
                } else {
                    state.prompt_feedback_export_status = format!(
                        "Local export preview only: no ArtifactStore write dispatched. Use POST /atelier/prompt-feedback/export with Model Ops headers to materialize hashed JSONL for rule pack {}.",
                        state.prompt_feedback_rule_pack
                    );
                }
            }
        });
        let export_status = state.prompt_feedback_export_status.clone();
        let export_readout = ui.label(egui::RichText::new(&export_status).color(palette.text));
        emit_value_node(
            ui.ctx(),
            export_readout.id,
            accesskit::Role::Label,
            ATELIER_PROMPTFEEDBACK_EXPORT_STATUS_AUTHOR_ID,
            "Prompt feedback export status",
            &export_status,
        );

        let status = state.prompt_feedback_status.clone();
        let status_readout = ui.label(egui::RichText::new(&status).color(palette.text));
        emit_value_node(
            ui.ctx(),
            status_readout.id,
            accesskit::Role::Label,
            ATELIER_PROMPTFEEDBACK_STATUS_AUTHOR_ID,
            "Prompt feedback status",
            &status,
        );
    }

    fn show_ingest(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        self.drain_contact_sheet_export_backend();
        self.drain_facial_analysis_backend();
        self.drain_facial_features_backend();
        self.drain_facial_command_backend();
        self.drain_prompt_feedback_backend();
        self.drain_ingest_classification_backend();
        // MT-020 + MT-030: INGEST sub-mode selector (Dataset intake / Prompt feedback / Facial review).
        let ingest_mode = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            ui.horizontal(|ui| {
                for mode in IngestMode::ALL {
                    let selected = state.ingest_mode == mode;
                    let button = ui.add(egui::Button::selectable(selected, mode.label()));
                    emit_node(
                        ui.ctx(),
                        button.id,
                        accesskit::Role::Button,
                        mode.author_id(),
                        mode.label(),
                        selected,
                    );
                    if button.clicked() {
                        state.ingest_mode = mode;
                    }
                }
            });
            state.ingest_mode
        };
        ui.separator();
        match ingest_mode {
            IngestMode::PromptFeedback => {
                self.show_prompt_feedback(ui, palette);
                return;
            }
            IngestMode::FacialReview => {
                self.show_facial_review(ui, palette);
                return;
            }
            IngestMode::DatasetIntake => {}
        }
        ui.label(egui::RichText::new("Intake batch source").color(palette.text));
        if let Ok(mut side_panel) = self.side_panel.lock() {
            side_panel.show(ui, palette);
        }
        ui.separator();
        let (expanded_items, expanded_batch_summary) = self
            .side_panel
            .lock()
            .ok()
            .map(|panel| {
                (
                    panel
                        .expanded()
                        .map(|(batch_id, items)| (batch_id.to_owned(), items.to_vec())),
                    panel.expanded_batch_summary(),
                )
            })
            .unwrap_or((None, None));
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.ingest_actor.trim().is_empty() {
            state.ingest_actor = self
                .ckc_client
                .as_ref()
                .map(|client| client.actor_id().to_owned())
                .unwrap_or_else(|| "atelier-ingest".to_owned());
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Dataset").color(palette.text));
            let dataset = ui.text_edit_singleline(&mut state.ingest_dataset_ref);
            emit_value_node(
                ui.ctx(),
                dataset.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_DATASET_REF_AUTHOR_ID,
                "Ingest dataset or source folder ref",
                &state.ingest_dataset_ref,
            );
            ui.label(egui::RichText::new("Character").color(palette.text));
            let character = ui.text_edit_singleline(&mut state.ingest_character_ref);
            emit_value_node(
                ui.ctx(),
                character.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CHARACTER_REF_AUTHOR_ID,
                "CKC character ref for passed image links",
                &state.ingest_character_ref,
            );
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Actor").color(palette.text));
            let actor = ui.text_edit_singleline(&mut state.ingest_actor);
            emit_value_node(
                ui.ctx(),
                actor.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_ACTOR_AUTHOR_ID,
                "Ingest backend actor id for batch apply",
                &state.ingest_actor,
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for decision in [
                IngestDecision::Pass,
                IngestDecision::Reject,
                IngestDecision::Unsure,
            ] {
                let selected = state.ingest_decision == decision;
                let button = ui.add(egui::Button::selectable(selected, decision.label()));
                let author_id = match decision {
                    IngestDecision::Pass => ATELIER_INGEST_PASS_AUTHOR_ID,
                    IngestDecision::Reject => ATELIER_INGEST_REJECT_AUTHOR_ID,
                    IngestDecision::Unsure => ATELIER_INGEST_UNSURE_AUTHOR_ID,
                };
                emit_node(
                    ui.ctx(),
                    button.id,
                    accesskit::Role::Button,
                    author_id,
                    decision.label(),
                    selected,
                );
                if button.clicked() {
                    state.ingest_decision = decision;
                    let loaded_count = expanded_items.as_ref().map_or(0, |(_, items)| {
                        for item in items {
                            state
                                .ingest_item_decisions
                                .insert(item.item_id.clone(), decision);
                            state.ingest_persisted_item_ids.remove(&item.item_id);
                        }
                        items.len()
                    });
                    state.ingest_status = if loaded_count > 0 {
                        format!(
                            "Ingest {} staged for {} loaded rows",
                            decision.machine_label(),
                            loaded_count
                        )
                    } else {
                        format!(
                            "Ingest default decision staged: {}",
                            decision.machine_label()
                        )
                    };
                }
            }
            let link = ui.checkbox(&mut state.ingest_link_passed, "CKC link intent");
            emit_node(
                ui.ctx(),
                link.id,
                accesskit::Role::CheckBox,
                ATELIER_INGEST_LINK_PASSED_AUTHOR_ID,
                "Persist CKC link intent metadata for passed images",
                state.ingest_link_passed,
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Batch tags").color(palette.text));
            let tags = ui.text_edit_singleline(&mut state.ingest_tag_buffer);
            emit_value_node(
                ui.ctx(),
                tags.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
                "Batch tags",
                &state.ingest_tag_buffer,
            );
            ui.label(egui::RichText::new("Facial").color(palette.text));
            let facial = ui.text_edit_singleline(&mut state.ingest_facial_profile);
            emit_value_node(
                ui.ctx(),
                facial.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID,
                "Facial quality, dedupe, identity, and review analysis profile",
                &state.ingest_facial_profile,
            );
            let analyze = ui.add_enabled(
                !state.ingest_facial_analysis_pending,
                egui::Button::new("Analyze Facial"),
            );
            emit_node(
                ui.ctx(),
                analyze.id,
                accesskit::Role::Button,
                ATELIER_INGEST_FACIAL_ANALYZE_AUTHOR_ID,
                "Run native Facial-derived Ingest analysis for the canonical expanded batch",
                state.ingest_facial_analysis_pending,
            );
            if analyze.clicked() {
                let profile = state.ingest_facial_profile.trim().to_owned();
                if profile.is_empty() {
                    state.ingest_last_facial_analysis = None;
                    state.ingest_status =
                        "Facial Ingest analysis blocked: profile is empty".to_owned();
                    state.ingest_facial_analysis_receipt =
                        "facial_analysis_blocked=empty_profile".to_owned();
                } else if let (Some(client), Some((batch_id, items))) =
                    (self.ckc_client.as_ref(), expanded_items.as_ref())
                {
                    let actor_id = state.ingest_actor.trim().to_owned();
                    let actor_id = if actor_id.is_empty() {
                        "atelier-ingest".to_owned()
                    } else {
                        actor_id
                    };
                    let expected_thread_id = intake_batch_model_ops_thread_id(batch_id);
                    let model_ops = match model_ops_mutation_context_from_state(
                        &state,
                        &actor_id,
                        &expected_thread_id,
                    ) {
                        Ok(model_ops) => model_ops,
                        Err(err) => {
                            state.ingest_last_facial_analysis = None;
                            state.ingest_status =
                                format!("Facial Ingest analysis blocked: {err}");
                            state.ingest_facial_analysis_receipt =
                                "facial_analysis_blocked=model_ops_context".to_owned();
                            return;
                        }
                    };
                    state.ingest_facial_analysis_request_seq =
                        state.ingest_facial_analysis_request_seq.saturating_add(1);
                    let request_seq = state.ingest_facial_analysis_request_seq;
                    state.ingest_facial_analysis_pending = true;
                    state.ingest_active_facial_analysis_request = Some(request_seq);
                    state.ingest_last_facial_analysis = None;
                    state.ingest_facial_analysis_receipt =
                        "facial_analysis_pending=true".to_owned();
                    state.ingest_status = format!(
                        "Dispatching Facial Ingest analysis to backend actor {actor_id}: batch_id={batch_id} profile={profile} visible_item_count={} canonical_source=backend",
                        items.len()
                    );
                    client.analyze_ingest_facial(
                        batch_id,
                        &profile,
                        &model_ops.actor_id,
                        model_ops.context,
                        request_seq,
                        self.ingest_facial_analysis_cell.clone(),
                    );
                } else if expanded_items.is_none() {
                    state.ingest_last_facial_analysis = None;
                    state.ingest_status =
                        "Facial Ingest analysis needs an expanded backend intake batch."
                            .to_owned();
                    state.ingest_facial_analysis_receipt =
                        "facial_analysis_blocked=missing_expanded_batch".to_owned();
                } else {
                    state.ingest_last_facial_analysis = None;
                    state.ingest_status =
                        "Facial Ingest analysis needs a backend client; no local fake analysis emitted."
                            .to_owned();
                    state.ingest_facial_analysis_receipt =
                        "facial_analysis_blocked=backend_required".to_owned();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Note").color(palette.text));
            let note = ui.text_edit_singleline(&mut state.ingest_batch_note);
            emit_value_node(
                ui.ctx(),
                note.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_BATCH_NOTE_AUTHOR_ID,
                "Batch note applied to reviewed images",
                &state.ingest_batch_note,
            );
        });
        egui::Grid::new("atelier-ingest-metadata-grid")
            .num_columns(4)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Event").color(palette.text));
                let event = ui
                    .add(egui::TextEdit::singleline(&mut state.ingest_event).desired_width(140.0));
                emit_value_node(
                    ui.ctx(),
                    event.id,
                    accesskit::Role::TextInput,
                    ATELIER_INGEST_EVENT_AUTHOR_ID,
                    "Batch event metadata",
                    &state.ingest_event,
                );
                ui.label(egui::RichText::new("Date").color(palette.text));
                let date =
                    ui.add(egui::TextEdit::singleline(&mut state.ingest_date).desired_width(100.0));
                emit_value_node(
                    ui.ctx(),
                    date.id,
                    accesskit::Role::TextInput,
                    ATELIER_INGEST_DATE_AUTHOR_ID,
                    "Batch date metadata",
                    &state.ingest_date,
                );
                ui.end_row();
                ui.label(egui::RichText::new("Location").color(palette.text));
                let location = ui.add(
                    egui::TextEdit::singleline(&mut state.ingest_location).desired_width(220.0),
                );
                emit_value_node(
                    ui.ctx(),
                    location.id,
                    accesskit::Role::TextInput,
                    ATELIER_INGEST_LOCATION_AUTHOR_ID,
                    "Batch location metadata",
                    &state.ingest_location,
                );
                ui.end_row();
            });
        ui.horizontal(|ui| {
            let apply = ui.add_enabled(
                !state.ingest_apply_pending,
                egui::Button::new("Apply full batch"),
            );
            emit_node(
                ui.ctx(),
                apply.id,
                accesskit::Role::Button,
                ATELIER_INGEST_APPLY_BATCH_AUTHOR_ID,
                "Apply the full persisted intake batch with visible-row overrides and structured ingest metadata",
                state.ingest_apply_pending,
            );
            if apply.clicked() {
                let readout = ingest_queue_readout(&state);
                if let (Some(client), Some((batch_id, items))) =
                    (self.ckc_client.as_ref(), expanded_items.as_ref())
                {
                    let request_id = format!("atelier-ingest-{}", Uuid::new_v4());
                    let default_decision = state.ingest_decision;
                    let metadata = ingest_metadata_payload(
                        &state,
                        &request_id,
                        Some(batch_id.as_str()),
                        items.len(),
                    );
                    let reason = format!(
                        "dataset_ref={} character_ref={} link_passed={} tags={} note={} event={} date={} location={} facial_profile={}",
                        state.ingest_dataset_ref.trim(),
                        state.ingest_character_ref.trim(),
                        state.ingest_link_passed,
                        state.ingest_tag_buffer.trim(),
                        state.ingest_batch_note.trim(),
                        state.ingest_event.trim(),
                        state.ingest_date.trim(),
                        state.ingest_location.trim(),
                        state.ingest_facial_profile.trim()
                    );
                    let overrides: Vec<AtelierIntakeClassificationDecision> = items
                        .iter()
                        .filter(|item| state.ingest_item_decisions.contains_key(&item.item_id))
                        .map(|item| {
                            let decision = ingest_item_decision(&state, item);
                            AtelierIntakeClassificationDecision {
                                item_id: item.item_id.clone(),
                                lane: decision.backend_lane().to_owned(),
                                reason: Some(reason.clone()),
                                metadata: metadata.clone(),
                            }
                        })
                        .collect();
                    let override_count = overrides.len();
                    let actor_id = state.ingest_actor.trim().to_owned();
                    let actor_id = if actor_id.is_empty() {
                        "atelier-ingest".to_owned()
                    } else {
                        actor_id
                    };
                    let expected_thread_id = intake_batch_model_ops_thread_id(batch_id);
                    let model_ops = match model_ops_mutation_context_from_state(
                        &state,
                        &actor_id,
                        &expected_thread_id,
                    ) {
                        Ok(model_ops) => model_ops,
                        Err(err) => {
                            state.ingest_status =
                                format!("Canonical batch classification blocked: {err}");
                            state.ingest_last_apply_receipt =
                                "classification_blocked=model_ops_context".to_owned();
                            return;
                        }
                    };
                    state.ingest_apply_pending = true;
                    state.ingest_apply_request_id = Some(request_id.clone());
                    state.ingest_apply_batch_id = Some(batch_id.clone());
                    state.ingest_apply_actor_id = Some(model_ops.actor_id.clone());
                    state.ingest_status = format!(
                        "Dispatching canonical batch classification to backend actor {} with {override_count} visible-row override(s), default_decision={}, request_id={request_id}: {readout}",
                        model_ops.actor_id,
                        default_decision.machine_label(),
                    );
                    client.apply_intake_batch_classifications(
                        request_id,
                        batch_id.clone(),
                        default_decision.backend_lane().to_owned(),
                        Some(reason),
                        metadata,
                        overrides,
                        &model_ops.actor_id,
                        model_ops.context,
                        self.ingest_classification_cell.clone(),
                    );
                } else {
                    state.ingest_status = format!(
                        "Full-batch apply needs an expanded backend intake batch; metadata staged locally: {readout}"
                    );
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Contact sheet").color(palette.text));
            let rows = ui.add(
                egui::TextEdit::singleline(&mut state.ingest_contact_rows).desired_width(42.0),
            );
            emit_value_node(
                ui.ctx(),
                rows.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_ROWS_AUTHOR_ID,
                "Contact sheet rows",
                &state.ingest_contact_rows,
            );
            ui.label("x");
            let columns = ui.add(
                egui::TextEdit::singleline(&mut state.ingest_contact_columns).desired_width(42.0),
            );
            emit_value_node(
                ui.ctx(),
                columns.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_COLUMNS_AUTHOR_ID,
                "Contact sheet columns",
                &state.ingest_contact_columns,
            );
            ui.label("@");
            let dpi = ui
                .add(egui::TextEdit::singleline(&mut state.ingest_contact_dpi).desired_width(58.0));
            emit_value_node(
                ui.ctx(),
                dpi.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_DPI_AUTHOR_ID,
                "Contact sheet DPI",
                &state.ingest_contact_dpi,
            );
            ui.label("dpi");
            let labels = ui.checkbox(&mut state.ingest_contact_include_labels, "Labels");
            emit_node(
                ui.ctx(),
                labels.id,
                accesskit::Role::CheckBox,
                ATELIER_INGEST_CONTACT_LABELS_AUTHOR_ID,
                "Include labels in contact sheet export",
                state.ingest_contact_include_labels,
            );
            ui.label(egui::RichText::new("Fit").color(palette.text));
            let fit = ui.add(
                egui::TextEdit::singleline(&mut state.ingest_contact_thumbnail_fit)
                    .desired_width(72.0),
            );
            emit_value_node(
                ui.ctx(),
                fit.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_FIT_AUTHOR_ID,
                "Contact sheet thumbnail fit: contain, cover, or stretch",
                &state.ingest_contact_thumbnail_fit,
            );
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Output").color(palette.text));
            let output = ui.text_edit_singleline(&mut state.ingest_contact_output_path);
            emit_value_node(
                ui.ctx(),
                output.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_OUTPUT_AUTHOR_ID,
                "Contact sheet output path or artifact ref",
                &state.ingest_contact_output_path,
            );
            let export = ui.add_enabled(
                !state.ingest_contact_export_pending,
                egui::Button::new("Export contact sheet"),
            );
            emit_node(
                ui.ctx(),
                export.id,
                accesskit::Role::Button,
                ATELIER_INGEST_CONTACT_EXPORT_AUTHOR_ID,
                "Export a native Atelier contact sheet SVG and receipt for the expanded intake batch",
                state.ingest_contact_export_pending,
            );
            if export.clicked() {
                let (rows, columns, dpi, cells) = ingest_contact_sheet_shape(&state);
                if let Some((batch_id, items)) = expanded_items.as_ref() {
                    let actor_id = state.ingest_actor.trim().to_owned();
                    let actor_id = if actor_id.is_empty() {
                        "atelier-ingest".to_owned()
                    } else {
                        actor_id
                    };
                    if let Some(client) = self.ckc_client.as_ref() {
                        let thumbnail_fit =
                            contact_sheet_thumbnail_fit_value(&state.ingest_contact_thumbnail_fit);
                        let output_path = ingest_optional_string(&state.ingest_contact_output_path);
                        let expected_thread_id = contact_sheet_model_ops_thread_id(
                            "ingest_batch",
                            batch_id,
                        );
                        let model_ops = match model_ops_mutation_context_from_state(
                            &state,
                            &actor_id,
                            &expected_thread_id,
                        ) {
                            Ok(model_ops) => model_ops,
                            Err(err) => {
                                state.ingest_status =
                                    format!("Contact sheet export blocked: {err}");
                                state.ingest_contact_sheet_receipt =
                                    "contact_sheet_export_blocked=model_ops_context".to_owned();
                                return;
                            }
                        };
                        state.ingest_contact_export_request_seq =
                            state.ingest_contact_export_request_seq.saturating_add(1);
                        let request_seq = state.ingest_contact_export_request_seq;
                        state.ingest_contact_export_pending = true;
                        state.ingest_active_contact_export_request = Some(request_seq);
                        state.ingest_status = format!(
                            "Dispatching contact sheet export to backend actor {actor_id}: batch_id={batch_id} rows={rows} columns={columns} dpi={dpi} cells={cells} labels={} thumbnail_fit={} output_path={} item_count={}",
                            state.ingest_contact_include_labels,
                            thumbnail_fit,
                            output_path.as_deref().unwrap_or("<none>"),
                            items.len()
                        );
                        let contact_items = contact_sheet_items_from_ingest(items);
                        client.export_contact_sheet(
                            "ingest_batch",
                            batch_id,
                            rows,
                            columns,
                            dpi,
                            state.ingest_contact_include_labels,
                            &thumbnail_fit,
                            output_path.as_deref(),
                            contact_items,
                            &model_ops.actor_id,
                            model_ops.context,
                            request_seq,
                            self.ingest_contact_export_cell.clone(),
                        );
                    } else {
                        let thumbnail_fit =
                            contact_sheet_thumbnail_fit_value(&state.ingest_contact_thumbnail_fit);
                        let snapshot = contact_sheet_local_snapshot(
                            batch_id,
                            items,
                            rows,
                            columns,
                            dpi,
                            state.ingest_contact_include_labels,
                            &thumbnail_fit,
                            ingest_optional_string(&state.ingest_contact_output_path),
                        );
                        state.ingest_contact_sheet_receipt =
                            contact_sheet_export_preview(&snapshot);
                        state.ingest_status = format!(
                            "Local contact sheet preview: {rows}x{columns}@{dpi}dpi labels={} thumbnail_fit={} rendered_item_count={} omitted_item_count={} svg_artifact_ref={} receipt_ref={}",
                            snapshot.include_labels,
                            snapshot.thumbnail_fit,
                            snapshot.rendered_item_count,
                            snapshot.omitted_item_count,
                            snapshot.svg_artifact_ref,
                            snapshot.receipt_ref
                        );
                        state.ingest_last_contact_sheet = Some(snapshot);
                    }
                } else {
                    state.ingest_status = format!(
                        "Contact sheet export needs an expanded intake batch; settings staged locally: {rows}x{columns}@{dpi}dpi cells={cells}"
                    );
                    state.ingest_contact_sheet_receipt =
                        "contact_sheet_export_blocked=missing_expanded_batch".to_owned();
                }
            }
        });
        ui.add_space(6.0);
        let queue_readout = ingest_queue_readout(&state);
        let queue_visible = compact_visual_cell(&queue_readout, 96);
        let queue = ui
            .label(egui::RichText::new(&queue_visible).color(palette.text_subtle))
            .on_hover_text(&queue_readout);
        emit_value_node(
            ui.ctx(),
            queue.id,
            accesskit::Role::Label,
            ATELIER_INGEST_QUEUE_READOUT_AUTHOR_ID,
            "Ingest queue readout",
            &queue_readout,
        );
        let batch_summary = expanded_batch_summary
            .as_ref()
            .map(|summary| summary.value.clone())
            .unwrap_or_else(|| "No expanded intake batch summary.".to_owned());
        let summary_visible = compact_visual_cell(&batch_summary, 96);
        let summary = ui
            .label(egui::RichText::new(&summary_visible).color(palette.text_subtle))
            .on_hover_text(&batch_summary);
        emit_value_node(
            ui.ctx(),
            summary.id,
            accesskit::Role::Label,
            ATELIER_INGEST_BATCH_SUMMARY_AUTHOR_ID,
            "Ingest canonical batch lane counts",
            &batch_summary,
        );
        let ingest_status = state.ingest_status.clone();
        let status_visible = compact_visual_cell(&ingest_status, 96);
        let status = ui
            .label(egui::RichText::new(&status_visible).color(palette.text))
            .on_hover_text(&ingest_status);
        emit_value_node(
            ui.ctx(),
            status.id,
            accesskit::Role::Label,
            ATELIER_INGEST_STATUS_AUTHOR_ID,
            "Ingest status",
            &ingest_status,
        );
        let contact_preview =
            contact_sheet_visual_preview(state.ingest_last_contact_sheet.as_ref());
        let contact_preview_visible = compact_visual_cell(&contact_preview, 96);
        let preview = ui
            .label(egui::RichText::new(&contact_preview_visible).color(palette.text_subtle))
            .on_hover_text(&contact_preview);
        emit_value_node(
            ui.ctx(),
            preview.id,
            accesskit::Role::Label,
            ATELIER_INGEST_CONTACT_PREVIEW_AUTHOR_ID,
            "Ingest contact sheet visual grid preview",
            &contact_preview,
        );
        let facial_summary =
            facial_analysis_summary_preview(state.ingest_last_facial_analysis.as_ref());
        let facial = ui.add(
            egui::Label::new(egui::RichText::new(&facial_summary).color(palette.text_subtle))
                .wrap(),
        );
        emit_value_node(
            ui.ctx(),
            facial.id,
            accesskit::Role::Label,
            ATELIER_INGEST_FACIAL_SUMMARY_AUTHOR_ID,
            "Ingest Facial analysis summary",
            &facial_summary,
        );
        let last_apply_receipt = state.ingest_last_apply_receipt.clone();
        let last_apply_receipt_visible = compact_visual_cell(&last_apply_receipt, 96);
        let receipt = ui
            .label(egui::RichText::new(&last_apply_receipt_visible).color(palette.text_subtle))
            .on_hover_text(&last_apply_receipt);
        emit_value_node(
            ui.ctx(),
            receipt.id,
            accesskit::Role::Label,
            ATELIER_INGEST_LAST_RECEIPT_AUTHOR_ID,
            "Last Ingest backend apply receipt",
            &last_apply_receipt,
        );
        let contact_sheet_receipt = state.ingest_contact_sheet_receipt.clone();
        let contact_sheet_receipt_visible = compact_visual_cell(&contact_sheet_receipt, 96);
        let contact_receipt = ui
            .label(egui::RichText::new(&contact_sheet_receipt_visible).color(palette.text_subtle))
            .on_hover_text(&contact_sheet_receipt);
        emit_value_node(
            ui.ctx(),
            contact_receipt.id,
            accesskit::Role::Label,
            ATELIER_INGEST_CONTACT_RECEIPT_AUTHOR_ID,
            "Last Ingest contact sheet export receipt",
            &contact_sheet_receipt,
        );
        let facial_receipt = ui.add(
            egui::Label::new(
                egui::RichText::new(&state.ingest_facial_analysis_receipt)
                    .color(palette.text_subtle),
            )
            .wrap(),
        );
        emit_value_node(
            ui.ctx(),
            facial_receipt.id,
            accesskit::Role::Label,
            ATELIER_INGEST_FACIAL_RECEIPT_AUTHOR_ID,
            "Last Ingest Facial analysis receipt",
            &state.ingest_facial_analysis_receipt,
        );
        ui.separator();
        egui::Grid::new("atelier-ingest-grid")
            .striped(true)
            .min_col_width(72.0)
            .show(ui, |ui| {
                ui.strong("Asset");
                ui.strong("Source");
                ui.strong("Decision");
                ui.strong("Apply state");
                ui.strong("Set");
                ui.strong("Tags");
                ui.strong("CKC link");
                ui.end_row();
                let Some((batch_id, items)) = expanded_items.as_ref() else {
                    ui.label("No loaded intake batch.");
                    ui.label("-");
                    ui.label(state.ingest_decision.label());
                    ui.label("-");
                    ui.label("-");
                    ui.label(&state.ingest_tag_buffer);
                    ui.label("-");
                    ui.end_row();
                    return;
                };
                if items.is_empty() {
                    ui.label(format!("Batch {batch_id} has no loaded items."));
                    ui.label("-");
                    ui.label(state.ingest_decision.label());
                    ui.label("-");
                    ui.label("-");
                    ui.label(&state.ingest_tag_buffer);
                    ui.label("-");
                    ui.end_row();
                    return;
                }
                for item in items {
                    let item_decision = ingest_item_decision(&state, item);
                    let persisted_state = if state.ingest_persisted_item_ids.contains(&item.item_id)
                    {
                        "persisted"
                    } else {
                        "staged"
                    };
                    let item_ref = format!(
                        "item_id={} file_name={} source_path={} source_lane={} staged_decision={} apply_state={}",
                        item.item_id,
                        item.file_name,
                        item.source_path,
                        item.lane,
                        item_decision.machine_label(),
                        persisted_state
                    );
                    let item_label = ui.label(&item.file_name);
                    let row_author_id = ingest_item_row_author_id(&item.item_id);
                    emit_value_node(
                        ui.ctx(),
                        item_label.id,
                        accesskit::Role::ListItem,
                        &row_author_id,
                        &format!("Ingest item {}", item.file_name),
                        &item_ref,
                    );
                    let visible_source = compact_visual_cell(&item.source_path, 48);
                    ui.label(visible_source).on_hover_text(&item.source_path);
                    ui.label(item_decision.label());
                    ui.label(persisted_state);
                    ui.horizontal(|ui| {
                        for decision in [
                            IngestDecision::Pass,
                            IngestDecision::Reject,
                            IngestDecision::Unsure,
                        ] {
                            let selected = item_decision == decision;
                            let button = ui
                                .add(egui::Button::selectable(selected, decision.machine_label()));
                            let author_id = match decision {
                                IngestDecision::Pass => ingest_item_pass_author_id(&item.item_id),
                                IngestDecision::Reject => {
                                    ingest_item_reject_author_id(&item.item_id)
                                }
                                IngestDecision::Unsure => {
                                    ingest_item_unsure_author_id(&item.item_id)
                                }
                            };
                            emit_node(
                                ui.ctx(),
                                button.id,
                                accesskit::Role::Button,
                                &author_id,
                                &format!("Set {} to {}", item.file_name, decision.machine_label()),
                                selected,
                            );
                            if button.clicked() {
                                state
                                    .ingest_item_decisions
                                    .insert(item.item_id.clone(), decision);
                                state.ingest_persisted_item_ids.remove(&item.item_id);
                                state.ingest_status = format!(
                                    "Ingest item staged: {} -> {}",
                                    item.file_name,
                                    decision.machine_label()
                                );
                            }
                        }
                    });
                    ui.label(&state.ingest_tag_buffer);
                    ui.label(
                        if state.ingest_link_passed && item_decision == IngestDecision::Pass {
                            state.ingest_character_ref.as_str()
                        } else {
                            "-"
                        },
                    );
                    ui.end_row();
                }
            });
    }

    /// MT-030 Facial Review sub-mode (nested under INGEST). Surfaces the native Facial review-queue
    /// command family (backend MT-028/MT-029: session / claim / decision / status / montage / export)
    /// that had backend + backend-manual coverage but NO frontend surface, plus the capability registry
    /// (`GET /atelier/facial/features`) and an identity real-vs-unavailable provenance readout. Every
    /// control a model must SET is a `Role::TextInput` (argus.set_value is TextInput-only per
    /// mcp/action.rs); actions are buttons/checkboxes (argus.click). All backend calls route through the
    /// actor-attributed `facial_review_*_actor_request` builders and the off-thread
    /// `dispatch_facial_review_command` loop — visible pending/degraded status, never a silent no-op.
    fn show_facial_review(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        // The canonical expanded intake batch id (session-create needs it). Read without holding the
        // side_panel lock across the state lock.
        let expanded_batch_id = self.side_panel.lock().ok().and_then(|panel| {
            panel
                .expanded()
                .map(|(batch_id, _items)| batch_id.to_owned())
        });
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let actor_id = self
            .ckc_client
            .as_ref()
            .map(|client| client.actor_id().to_owned())
            .unwrap_or_else(|| "atelier-ingest".to_owned());
        let client = self.ckc_client.as_ref();
        let command_cell = self.ingest_facial_command_cell.clone();
        let features_cell = self.ingest_facial_features_cell.clone();

        egui::ScrollArea::vertical()
            .id_salt("atelier-ingest-facial-review-scroll")
            .show(ui, |ui| {
                let text_input =
                    |ui: &mut egui::Ui, label: &str, author_id: &str, value: &mut String| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(label).color(palette.text));
                            let resp = ui.text_edit_singleline(value);
                            emit_value_node(
                                ui.ctx(),
                                resp.id,
                                accesskit::Role::TextInput,
                                author_id,
                                label,
                                value,
                            );
                        });
                    };
                let readout = |ui: &mut egui::Ui, author_id: &str, label: &str, value: &str| {
                    let resp = ui.add(
                        egui::Label::new(egui::RichText::new(value).color(palette.text_subtle))
                            .wrap(),
                    );
                    emit_value_node(
                        ui.ctx(),
                        resp.id,
                        accesskit::Role::Label,
                        author_id,
                        label,
                        value,
                    );
                };

                let pending = state.facial_review_pending;
                // What to dispatch this frame (built inside a valid, backed handler); one dispatch site.
                let mut pending_command: Option<(
                    crate::backend_client::ActorRequestSpec,
                    String,
                    String,
                )> = None;
                let mut pending_features = false;

                ui.label(
                    egui::RichText::new(
                        "Facial review queue — session / claim / decision / status / montage / export",
                    )
                    .strong()
                    .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(
                        "Non-destructive review of the canonical intake batch. Run Analyze Facial in Dataset intake first to seed provenance; then start a session and chain claim -> decision -> status/montage/export.",
                    )
                    .color(palette.text_subtle),
                );
                ui.separator();

                // ── Capability registry + identity/quality/dedupe provenance (real vs unavailable) ──
                ui.horizontal(|ui| {
                    let load = ui.add_enabled(
                        !state.facial_features_pending,
                        egui::Button::new("Load Facial capabilities"),
                    );
                    emit_node(
                        ui.ctx(),
                        load.id,
                        accesskit::Role::Button,
                        ATELIER_INGEST_FACIAL_FEATURES_LOAD_AUTHOR_ID,
                        "Load the native Facial capability/command registry (GET /atelier/facial/features)",
                        state.facial_features_pending,
                    );
                    if load.clicked() {
                        if client.is_some() {
                            pending_features = true;
                        } else {
                            state.facial_features_readout =
                                "facial_features_blocked=backend_required".to_owned();
                            state.ingest_status =
                                "Facial capability registry needs a backend client.".to_owned();
                        }
                    }
                });
                let features_readout = state.facial_features_readout.clone();
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_FEATURES_READOUT_AUTHOR_ID,
                    "Facial capability registry readout",
                    &features_readout,
                );
                let provenance =
                    facial_provenance_preview(state.ingest_last_facial_analysis.as_ref());
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_PROVENANCE_AUTHOR_ID,
                    "Facial identity/quality/dedupe provenance (real vs unavailable)",
                    &provenance,
                );
                ui.separator();

                // ── Session create ────────────────────────────────────────────────────────────────
                ui.label(egui::RichText::new("Review session").strong().color(palette.text));
                text_input(
                    ui,
                    "Profile",
                    ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID,
                    &mut state.ingest_facial_profile,
                );
                text_input(
                    ui,
                    "Shard count",
                    ATELIER_INGEST_FACIAL_SHARD_COUNT_AUTHOR_ID,
                    &mut state.facial_review_shard_count,
                );
                text_input(
                    ui,
                    "Claim TTL seconds",
                    ATELIER_INGEST_FACIAL_TTL_AUTHOR_ID,
                    &mut state.facial_review_claim_ttl,
                );
                let session = ui.add_enabled(!pending, egui::Button::new("Start review session"));
                emit_node(
                    ui.ctx(),
                    session.id,
                    accesskit::Role::Button,
                    ATELIER_INGEST_FACIAL_SESSION_START_AUTHOR_ID,
                    "Create a Facial review session over the canonical intake batch",
                    pending,
                );
                if session.clicked() {
                    match (client, expanded_batch_id.as_deref()) {
                        (Some(client), Some(batch_id)) => {
                            let profile = state.ingest_facial_profile.trim().to_owned();
                            let profile_opt =
                                if profile.is_empty() { None } else { Some(profile.as_str()) };
                            let mut parse_notes = Vec::new();
                            let shard_count = parse_facial_optional_usize_with_note(
                                &state.facial_review_shard_count,
                                "session.shard_count",
                                &mut parse_notes,
                            );
                            let ttl = parse_facial_optional_u64_with_note(
                                &state.facial_review_claim_ttl,
                                "session.claim_ttl_seconds",
                                &mut parse_notes,
                            );
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&parse_notes);
                            let spec = client.facial_review_session_actor_request(
                                batch_id,
                                profile_opt,
                                shard_count,
                                ttl,
                                &actor_id,
                            );
                            pending_command = Some((
                                spec,
                                "atelier.facial.review.session.create".to_owned(),
                                intake_batch_model_ops_thread_id(batch_id),
                            ));
                        }
                        (None, _) => {
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            state.facial_command_receipt =
                                "facial_command_blocked=backend_required".to_owned();
                            state.ingest_status =
                                "Facial review session needs a backend client; no local fake session emitted."
                                    .to_owned();
                        }
                        (_, None) => {
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            state.facial_command_receipt =
                                "facial_command_blocked=missing_expanded_batch".to_owned();
                            state.ingest_status =
                                "Facial review session needs an expanded backend intake batch."
                                    .to_owned();
                        }
                    }
                }
                text_input(
                    ui,
                    "Session artifact ref",
                    ATELIER_INGEST_FACIAL_SESSION_REF_AUTHOR_ID,
                    &mut state.facial_review_session_ref,
                );
                text_input(
                    ui,
                    "Session id",
                    ATELIER_INGEST_FACIAL_SESSION_ID_AUTHOR_ID,
                    &mut state.facial_review_session_id,
                );
                ui.separator();

                // ── Claim a shard ─────────────────────────────────────────────────────────────────
                ui.label(egui::RichText::new("Claim shard").strong().color(palette.text));
                text_input(
                    ui,
                    "Claim shard index",
                    ATELIER_INGEST_FACIAL_CLAIM_SHARD_AUTHOR_ID,
                    &mut state.facial_review_claim_shard,
                );
                let steal = ui.checkbox(
                    &mut state.facial_review_steal_expired,
                    "Steal expired claims",
                );
                emit_node(
                    ui.ctx(),
                    steal.id,
                    accesskit::Role::CheckBox,
                    ATELIER_INGEST_FACIAL_CLAIM_STEAL_AUTHOR_ID,
                    "Steal expired review claims when claiming a shard",
                    state.facial_review_steal_expired,
                );
                let claim = ui.add_enabled(!pending, egui::Button::new("Claim shard"));
                emit_node(
                    ui.ctx(),
                    claim.id,
                    accesskit::Role::Button,
                    ATELIER_INGEST_FACIAL_CLAIM_START_AUTHOR_ID,
                    "Claim a Facial review shard from the active session",
                    pending,
                );
                if claim.clicked() {
                    if let Some(client) = client {
                        let session_ref = state.facial_review_session_ref.trim().to_owned();
                        let session_id = state.facial_review_session_id.trim().to_owned();
                        if session_ref.is_empty() || session_id.is_empty() {
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            state.facial_command_receipt =
                                "facial_command_blocked=missing_session_ref_or_id".to_owned();
                        } else {
                            let mut parse_notes = Vec::new();
                            let shard = parse_facial_optional_usize_with_note(
                                &state.facial_review_claim_shard,
                                "claim.shard",
                                &mut parse_notes,
                            );
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&parse_notes);
                            let steal = state.facial_review_steal_expired;
                            let claim_refs = state.facial_review_claim_refs.clone();
                            let decision_refs = state.facial_review_decision_refs.clone();
                            let spec = client.facial_review_claim_actor_request(
                                &session_ref,
                                &claim_refs,
                                &decision_refs,
                                shard,
                                steal,
                                &actor_id,
                            );
                            match facial_review_model_ops_thread_from_state(&state, &session_id) {
                                Ok(expected_thread_id) => {
                                    pending_command = Some((
                                        spec,
                                        "atelier.facial.review.claim".to_owned(),
                                        expected_thread_id,
                                    ));
                                }
                                Err(err) => {
                                    state.facial_command_receipt =
                                        "facial_command_blocked=model_ops_thread".to_owned();
                                    state.ingest_status = err;
                                }
                            }
                        }
                    } else {
                        state.facial_review_parse_notes = facial_review_numeric_parse_notes(&[]);
                        state.facial_command_receipt =
                            "facial_command_blocked=backend_required".to_owned();
                    }
                }
                text_input(
                    ui,
                    "Claim artifact ref",
                    ATELIER_INGEST_FACIAL_CLAIM_REF_AUTHOR_ID,
                    &mut state.facial_review_claim_ref,
                );
                ui.separator();

                // ── Record a decision ─────────────────────────────────────────────────────────────
                ui.label(egui::RichText::new("Record decision").strong().color(palette.text));
                text_input(
                    ui,
                    "Decision item id",
                    ATELIER_INGEST_FACIAL_DECISION_ITEM_AUTHOR_ID,
                    &mut state.facial_review_decision_item,
                );
                text_input(
                    ui,
                    "Decision verdict (accept/reject/hold synonyms)",
                    ATELIER_INGEST_FACIAL_DECISION_VERDICT_AUTHOR_ID,
                    &mut state.facial_review_decision_verdict,
                );
                text_input(
                    ui,
                    "Decision reason",
                    ATELIER_INGEST_FACIAL_DECISION_REASON_AUTHOR_ID,
                    &mut state.facial_review_decision_reason,
                );
                text_input(
                    ui,
                    "Decision tags (comma-separated)",
                    ATELIER_INGEST_FACIAL_DECISION_TAGS_AUTHOR_ID,
                    &mut state.facial_review_decision_tags,
                );
                text_input(
                    ui,
                    "Decision notes",
                    ATELIER_INGEST_FACIAL_DECISION_NOTES_AUTHOR_ID,
                    &mut state.facial_review_decision_notes,
                );
                let record = ui.add_enabled(!pending, egui::Button::new("Record decision"));
                emit_node(
                    ui.ctx(),
                    record.id,
                    accesskit::Role::Button,
                    ATELIER_INGEST_FACIAL_DECISION_RECORD_AUTHOR_ID,
                    "Record one Facial review decision into the append-only decision ledger",
                    pending,
                );
                if record.clicked() {
                    if let Some(client) = client {
                        let session_ref = state.facial_review_session_ref.trim().to_owned();
                        let session_id = state.facial_review_session_id.trim().to_owned();
                        let claim_ref = state.facial_review_claim_ref.trim().to_owned();
                        let item_id = state.facial_review_decision_item.trim().to_owned();
                        if session_ref.is_empty()
                            || session_id.is_empty()
                            || claim_ref.is_empty()
                            || item_id.is_empty()
                        {
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            state.facial_command_receipt =
                                "facial_command_blocked=missing_session_ref_or_id_or_claim_or_item".to_owned();
                        } else {
                            let verdict =
                                state.facial_review_decision_verdict.trim().to_owned();
                            let reason = state.facial_review_decision_reason.trim().to_owned();
                            let tags: Vec<String> = state
                                .facial_review_decision_tags
                                .split(',')
                                .map(|tag| tag.trim().to_owned())
                                .filter(|tag| !tag.is_empty())
                                .collect();
                            let notes_raw =
                                state.facial_review_decision_notes.trim().to_owned();
                            let notes =
                                if notes_raw.is_empty() { None } else { Some(notes_raw.as_str()) };
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            let spec = client.facial_review_decision_actor_request(
                                &session_ref,
                                &claim_ref,
                                &item_id,
                                &verdict,
                                &reason,
                                &tags,
                                notes,
                                &actor_id,
                            );
                            match facial_review_model_ops_thread_from_state(&state, &session_id) {
                                Ok(expected_thread_id) => {
                                    pending_command = Some((
                                        spec,
                                        "atelier.facial.review.decision".to_owned(),
                                        expected_thread_id,
                                    ));
                                }
                                Err(err) => {
                                    state.facial_command_receipt =
                                        "facial_command_blocked=model_ops_thread".to_owned();
                                    state.ingest_status = err;
                                }
                            }
                        }
                    } else {
                        state.facial_review_parse_notes = facial_review_numeric_parse_notes(&[]);
                        state.facial_command_receipt =
                            "facial_command_blocked=backend_required".to_owned();
                    }
                }
                ui.separator();

                // ── Status / montage / export ─────────────────────────────────────────────────────
                ui.label(
                    egui::RichText::new("Status / montage / export").strong().color(palette.text),
                );
                let replay = ui.add_enabled(!pending, egui::Button::new("Replay review status"));
                emit_node(
                    ui.ctx(),
                    replay.id,
                    accesskit::Role::Button,
                    ATELIER_INGEST_FACIAL_STATUS_REPLAY_AUTHOR_ID,
                    "Replay Facial review queue status from persisted session/claim/decision refs",
                    pending,
                );
                if replay.clicked() {
                    if let Some(client) = client {
                        let session_ref = state.facial_review_session_ref.trim().to_owned();
                        let session_id = state.facial_review_session_id.trim().to_owned();
                        if session_ref.is_empty() || session_id.is_empty() {
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            state.facial_command_receipt =
                                "facial_command_blocked=missing_session_ref_or_id".to_owned();
                        } else {
                            let claim_refs = state.facial_review_claim_refs.clone();
                            let decision_refs = state.facial_review_decision_refs.clone();
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            let spec = client.facial_review_status_actor_request(
                                &session_ref,
                                &claim_refs,
                                &decision_refs,
                                &actor_id,
                            );
                            match facial_review_model_ops_thread_from_state(&state, &session_id) {
                                Ok(expected_thread_id) => {
                                    pending_command = Some((
                                        spec,
                                        "atelier.facial.review.status".to_owned(),
                                        expected_thread_id,
                                    ));
                                }
                                Err(err) => {
                                    state.facial_command_receipt =
                                        "facial_command_blocked=model_ops_thread".to_owned();
                                    state.ingest_status = err;
                                }
                            }
                        }
                    } else {
                        state.facial_review_parse_notes = facial_review_numeric_parse_notes(&[]);
                        state.facial_command_receipt =
                            "facial_command_blocked=backend_required".to_owned();
                    }
                }
                let status_readout = state.facial_review_status_readout.clone();
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_REVIEW_STATUS_READOUT_AUTHOR_ID,
                    "Facial review queue status readout",
                    &status_readout,
                );

                text_input(
                    ui,
                    "Montage page",
                    ATELIER_INGEST_FACIAL_MONTAGE_PAGE_AUTHOR_ID,
                    &mut state.facial_review_montage_page,
                );
                text_input(
                    ui,
                    "Montage columns",
                    ATELIER_INGEST_FACIAL_MONTAGE_COLUMNS_AUTHOR_ID,
                    &mut state.facial_review_montage_columns,
                );
                text_input(
                    ui,
                    "Montage rows",
                    ATELIER_INGEST_FACIAL_MONTAGE_ROWS_AUTHOR_ID,
                    &mut state.facial_review_montage_rows,
                );
                text_input(
                    ui,
                    "Montage decision filter",
                    ATELIER_INGEST_FACIAL_MONTAGE_FILTER_AUTHOR_ID,
                    &mut state.facial_review_montage_filter,
                );
                let montage = ui.add_enabled(!pending, egui::Button::new("Build montage"));
                emit_node(
                    ui.ctx(),
                    montage.id,
                    accesskit::Role::Button,
                    ATELIER_INGEST_FACIAL_MONTAGE_BUILD_AUTHOR_ID,
                    "Build an Argus-addressable Facial review montage tile map",
                    pending,
                );
                if montage.clicked() {
                    if let Some(client) = client {
                        let session_ref = state.facial_review_session_ref.trim().to_owned();
                        let session_id = state.facial_review_session_id.trim().to_owned();
                        if session_ref.is_empty() || session_id.is_empty() {
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            state.facial_command_receipt =
                                "facial_command_blocked=missing_session_ref_or_id".to_owned();
                        } else {
                            let decision_refs = state.facial_review_decision_refs.clone();
                            let mut parse_notes = Vec::new();
                            let page = parse_facial_usize_with_note(
                                &state.facial_review_montage_page,
                                "montage.page",
                                0,
                                &mut parse_notes,
                            );
                            let columns = parse_facial_usize_with_note(
                                &state.facial_review_montage_columns,
                                "montage.columns",
                                5,
                                &mut parse_notes,
                            );
                            let rows = parse_facial_usize_with_note(
                                &state.facial_review_montage_rows,
                                "montage.rows",
                                4,
                                &mut parse_notes,
                            );
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&parse_notes);
                            let filter_raw =
                                state.facial_review_montage_filter.trim().to_owned();
                            let filter = if filter_raw.is_empty() {
                                None
                            } else {
                                Some(filter_raw.as_str())
                            };
                            let spec = client.facial_review_montage_actor_request(
                                &session_ref,
                                &decision_refs,
                                page,
                                columns,
                                rows,
                                filter,
                                &actor_id,
                            );
                            match facial_review_model_ops_thread_from_state(&state, &session_id) {
                                Ok(expected_thread_id) => {
                                    pending_command = Some((
                                        spec,
                                        "atelier.facial.review.montage".to_owned(),
                                        expected_thread_id,
                                    ));
                                }
                                Err(err) => {
                                    state.facial_command_receipt =
                                        "facial_command_blocked=model_ops_thread".to_owned();
                                    state.ingest_status = err;
                                }
                            }
                        }
                    } else {
                        state.facial_review_parse_notes = facial_review_numeric_parse_notes(&[]);
                        state.facial_command_receipt =
                            "facial_command_blocked=backend_required".to_owned();
                    }
                }

                text_input(
                    ui,
                    "Export dataset name",
                    ATELIER_INGEST_FACIAL_EXPORT_DATASET_AUTHOR_ID,
                    &mut state.facial_review_export_dataset,
                );
                text_input(
                    ui,
                    "Export repeats",
                    ATELIER_INGEST_FACIAL_EXPORT_REPEATS_AUTHOR_ID,
                    &mut state.facial_review_export_repeats,
                );
                text_input(
                    ui,
                    "Export output root ref",
                    ATELIER_INGEST_FACIAL_EXPORT_OUTPUT_AUTHOR_ID,
                    &mut state.facial_review_export_output,
                );
                let allow_partial = ui.checkbox(
                    &mut state.facial_review_export_allow_partial,
                    "Allow partial export (skip undecided)",
                );
                emit_node(
                    ui.ctx(),
                    allow_partial.id,
                    accesskit::Role::CheckBox,
                    ATELIER_INGEST_FACIAL_EXPORT_ALLOW_PARTIAL_AUTHOR_ID,
                    "Allow a partial Facial review export when undecided items remain",
                    state.facial_review_export_allow_partial,
                );
                let export = ui.add_enabled(!pending, egui::Button::new("Export dataset manifest"));
                emit_node(
                    ui.ctx(),
                    export.id,
                    accesskit::Role::Button,
                    ATELIER_INGEST_FACIAL_EXPORT_BUILD_AUTHOR_ID,
                    "Build a non-destructive Facial review export lineage manifest (no source mutation)",
                    pending,
                );
                if export.clicked() {
                    if let Some(client) = client {
                        let session_ref = state.facial_review_session_ref.trim().to_owned();
                        let session_id = state.facial_review_session_id.trim().to_owned();
                        let dataset = state.facial_review_export_dataset.trim().to_owned();
                        let output = state.facial_review_export_output.trim().to_owned();
                        if session_ref.is_empty()
                            || session_id.is_empty()
                            || dataset.is_empty()
                            || output.is_empty()
                        {
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&[]);
                            state.facial_command_receipt =
                                "facial_command_blocked=missing_session_ref_or_id_or_dataset_or_output"
                                    .to_owned();
                        } else {
                            let decision_refs = state.facial_review_decision_refs.clone();
                            let mut parse_notes = Vec::new();
                            let repeats = parse_facial_u32_with_note(
                                &state.facial_review_export_repeats,
                                "export.repeats",
                                10,
                                &mut parse_notes,
                            );
                            state.facial_review_parse_notes =
                                facial_review_numeric_parse_notes(&parse_notes);
                            let allow_partial = state.facial_review_export_allow_partial;
                            let spec = client.facial_review_export_actor_request(
                                &session_ref,
                                &decision_refs,
                                &dataset,
                                repeats,
                                allow_partial,
                                &output,
                                &actor_id,
                            );
                            match facial_review_model_ops_thread_from_state(&state, &session_id) {
                                Ok(expected_thread_id) => {
                                    pending_command = Some((
                                        spec,
                                        "atelier.facial.review.export".to_owned(),
                                        expected_thread_id,
                                    ));
                                }
                                Err(err) => {
                                    state.facial_command_receipt =
                                        "facial_command_blocked=model_ops_thread".to_owned();
                                    state.ingest_status = err;
                                }
                            }
                        }
                    } else {
                        state.facial_review_parse_notes = facial_review_numeric_parse_notes(&[]);
                        state.facial_command_receipt =
                            "facial_command_blocked=backend_required".to_owned();
                    }
                }
                ui.separator();

                // ── Single dispatch site (BEFORE the readouts so a click's pending status is visible
                //    in the same frame; no per-button repetition). ────────────────────────────────
                if pending_features && !state.facial_features_pending {
                    if let Some(client) = client {
                        state.facial_features_request_seq =
                            state.facial_features_request_seq.saturating_add(1);
                        let seq = state.facial_features_request_seq;
                        state.facial_features_pending = true;
                        state.facial_features_active_request = Some(seq);
                        state.ingest_status =
                            "Loading Facial capability registry from GET /atelier/facial/features"
                                .to_owned();
                        client.load_facial_features(seq, features_cell.clone());
                    }
                }
                if let Some((spec, command_name, expected_thread_id)) = pending_command {
                    if !state.facial_review_pending {
                        if let Some(client) = client {
                            let model_ops_fields = ModelOpsUiFields::from_state(&state);
                            match model_ops_fields.mutation_context(&actor_id, &expected_thread_id)
                            {
                                Ok(model_ops) => {
                                    state.facial_review_request_seq =
                                        state.facial_review_request_seq.saturating_add(1);
                                    let seq = state.facial_review_request_seq;
                                    state.facial_review_pending = true;
                                    state.facial_review_active_request = Some(seq);
                                    let parse_notes = state.facial_review_parse_notes.clone();
                                    state.facial_command_receipt = append_facial_parse_notes(
                                        format!("facial_command_pending={command_name}"),
                                        &parse_notes,
                                    );
                                    state.ingest_status = append_facial_parse_notes(format!(
                                        "Dispatching Facial review command {command_name} to backend actor {}",
                                        model_ops.actor_id
                                    ), &parse_notes);
                                    client.dispatch_facial_review_command(
                                        actor_spec_with_model_ops(spec, model_ops),
                                        seq,
                                        command_cell.clone(),
                                    );
                                }
                                Err(err) => {
                                    state.facial_command_receipt =
                                        "facial_command_blocked=model_ops_context".to_owned();
                                    state.ingest_status = format!(
                                        "Facial review command {command_name} blocked: {err}"
                                    );
                                }
                            }
                        }
                    }
                }

                // ── Lineage refs + last command receipt / degraded readout ───────────────────────
                let refs = facial_review_refs_preview(&state);
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_REVIEW_REFS_AUTHOR_ID,
                    "Accumulated Facial review lineage refs",
                    &refs,
                );
                let montage_readout = state.facial_review_montage_readout.clone();
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_MONTAGE_READOUT_AUTHOR_ID,
                    "Facial montage content readout",
                    &montage_readout,
                );
                let export_readout = state.facial_review_export_readout.clone();
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_EXPORT_READOUT_AUTHOR_ID,
                    "Facial export content readout",
                    &export_readout,
                );
                let parse_notes = state.facial_review_parse_notes.clone();
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_PARSE_NOTES_AUTHOR_ID,
                    "Facial numeric parse notes",
                    &parse_notes,
                );
                let command_receipt = state.facial_command_receipt.clone();
                readout(
                    ui,
                    ATELIER_INGEST_FACIAL_COMMAND_RECEIPT_AUTHOR_ID,
                    "Last Facial review command receipt / degraded readout",
                    &command_receipt,
                );
                let ingest_status = state.ingest_status.clone();
                readout(
                    ui,
                    ATELIER_INGEST_STATUS_AUTHOR_ID,
                    "Ingest status",
                    &ingest_status,
                );
            });
    }
}

pub struct AtelierPanelPaneMount {
    panel: Arc<AtelierPanel>,
    palette: SharedPalette,
}

impl AtelierPanelPaneMount {
    pub fn new(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        palette: SharedPalette,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
    ) -> Self {
        Self::with_optional_client(side_panel, canvas_board, palette, canvas_events, None)
    }

    pub fn with_client(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        palette: SharedPalette,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
        ckc_client: AtelierClient,
    ) -> Self {
        Self::with_optional_client(
            side_panel,
            canvas_board,
            palette,
            canvas_events,
            Some(ckc_client),
        )
    }

    fn with_optional_client(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        palette: SharedPalette,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
        ckc_client: Option<AtelierClient>,
    ) -> Self {
        Self {
            panel: Arc::new(AtelierPanel::with_client(
                side_panel,
                canvas_board,
                canvas_events,
                ckc_client,
            )),
            palette,
        }
    }

    /// WP-CKC MT-006: a shared handle to the singleton [`AtelierPanel`] this mount renders. The shell
    /// keeps a clone so `set_module` can drive the panel's internal `[Castkit Codex | Posekit | Ingest]`
    /// tab (module-ckc -> Castkit Codex, module-ingest -> Ingest) directly through `set_active_tab`,
    /// which mutates the panel's interior tab state. Both the mount and the shell address the SAME
    /// `AtelierPanel`, so the module-entry tab selection and the rendered panel never diverge.
    pub fn panel_handle(&self) -> Arc<AtelierPanel> {
        Arc::clone(&self.panel)
    }
}

impl PaneFactory for AtelierPanelPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::AtelierEditor
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        self.panel.show(ui, &palette);
    }
}

fn palette_of(cell: &SharedPalette) -> HsPalette {
    cell.lock()
        .map(|p| p.clone())
        .unwrap_or_else(|p| p.into_inner().clone())
}

/// Left viewport: the REAL Posekit source image, or an explicit empty / decode-error state.
///
/// MT-014 de-scaffold: the previous implementation painted a hash-derived colour tile plus a
/// procedural 3D skeleton, neither bound to the actual source asset. This now displays the decoded
/// MT-043: resolve a Posekit source ref to a media-asset UUID string for the byte-fetch route. Only a
/// canonical single-segment `atelier://media/<uuid>` ref resolves; the demo placeholder, padded refs,
/// multi-segment paths, and any non-UUID tail return `None` so an unresolvable source keeps the explicit
/// empty state instead of firing a byte fetch. Mirrors the backend `parse_media_asset_ref` shape.
fn parse_media_asset_id_from_ref(source_ref: &str) -> Option<String> {
    let id = source_ref.strip_prefix("atelier://media/")?;
    if id.is_empty() || id.contains('/') {
        return None;
    }
    Uuid::parse_str(id).ok().map(|uuid| uuid.to_string())
}

/// source-image texture when bytes are available and otherwise an honest empty state — it never
/// fabricates a stand-in. Source bytes arrive via [`AtelierPanel::set_pose_source_image_bytes`].
fn draw_pose_source_view(
    ui: &mut egui::Ui,
    palette: &HsPalette,
    author_id: &str,
    source_ref: &str,
    rig_id: Option<&str>,
    render: &PoseSourceRender,
) {
    ui.label(
        egui::RichText::new("Source image preview")
            .strong()
            .color(palette.text),
    );
    let height = 260.0;
    let width = ui.available_width().max(180.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, palette.surface);
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );

    let (state_token, detail) = match render {
        PoseSourceRender::Loaded {
            texture,
            width: w,
            height: h,
            ..
        } => {
            let image_rect = fit_rect_preserving_aspect(rect.shrink(8.0), *w as f32, *h as f32);
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            painter.image(texture.id(), image_rect, uv, egui::Color32::WHITE);
            ("loaded", format!("dimensions={w}x{h}"))
        }
        PoseSourceRender::DecodeError(err) => {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Source image could not be decoded",
                egui::FontId::proportional(13.0),
                palette.text_subtle,
            );
            ("decode_error", format!("error={err}"))
        }
        PoseSourceRender::Empty => {
            let message = if source_ref.trim().is_empty() {
                "No source image loaded"
            } else {
                "Awaiting source bytes"
            };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                message,
                egui::FontId::proportional(13.0),
                palette.text_subtle,
            );
            ("empty", "dimensions=none".to_owned())
        }
    };

    let node_label = format!(
        "Source image preview source_ref={} rig_id={} source_image={} {}",
        source_ref,
        rig_id.unwrap_or("<none>"),
        state_token,
        detail
    );
    emit_value_node(
        ui.ctx(),
        response.id,
        accesskit::Role::Group,
        author_id,
        &node_label,
        &node_label,
    );
}

/// MT-010 de-scaffold: paint the CKC linked-media viewer for the selected image. Mirrors
/// [`draw_pose_source_view`] — it shows the REAL decoded image texture when bytes are available and
/// otherwise an honest empty state, or an explicit decode-error state; it never fabricates a stand-in
/// tile. The media bytes arrive via [`AtelierPanel::set_ckc_media_image_bytes`]. Returns
/// `(state_token, detail)` (loaded/dimensions, decode_error/error, or empty) so the caller can carry the
/// load state on the stable `atelier-ckc-media-viewer` node value for Argus.
fn draw_ckc_media_image(
    ui: &mut egui::Ui,
    palette: &HsPalette,
    media_ref: &str,
    render: &PoseSourceRender,
) -> (&'static str, String) {
    let height = 200.0;
    let width = ui.available_width().max(160.0);
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, palette.surface);
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    match render {
        PoseSourceRender::Loaded {
            texture,
            width: w,
            height: h,
            ..
        } => {
            let image_rect = fit_rect_preserving_aspect(rect.shrink(8.0), *w as f32, *h as f32);
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            painter.image(texture.id(), image_rect, uv, egui::Color32::WHITE);
            ("loaded", format!("dimensions={w}x{h}"))
        }
        PoseSourceRender::DecodeError(err) => {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Linked image could not be decoded",
                egui::FontId::proportional(13.0),
                palette.text_subtle,
            );
            ("decode_error", format!("error={err}"))
        }
        PoseSourceRender::Empty => {
            let message = if media_ref.trim().is_empty() {
                "No linked image selected"
            } else {
                "No decoded image for this ref yet (awaiting linked-media bytes)"
            };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                message,
                egui::FontId::proportional(13.0),
                palette.text_subtle,
            );
            ("empty", "dimensions=none".to_owned())
        }
    }
}

/// Right viewport: paints the REAL OpenPose keypoints carried in `openpose_json` — the SAME structure
/// the export produces (the backend export result when one exists, else the live offline-preview
/// keypoints from [`posekit_openpose_json`]). Mirrors the backend `render_posekit_openpose_png`
/// geometry (COCO-18 bones + amber body/face joints + green hands), so the preview is the genuine
/// OpenPose that would be exported rather than a hand-authored schematic.
#[allow(clippy::too_many_arguments)]
fn draw_pose_openpose_view(
    ui: &mut egui::Ui,
    palette: &HsPalette,
    author_id: &str,
    openpose_json: &serde_json::Value,
    bound_source: &str,
    source_ref: &str,
    rig_id: Option<&str>,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    face: bool,
    body: bool,
    hands: bool,
    artifact_render: &PoseSourceRender,
    artifact_ref: Option<&str>,
    artifact_status: &str,
) {
    ui.label(
        egui::RichText::new("OpenPose preview")
            .strong()
            .color(palette.text),
    );
    let height = 260.0;
    let width = ui.available_width().max(180.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::BLACK);

    let artifact_ref = artifact_ref.unwrap_or("<none>");
    match artifact_render {
        PoseSourceRender::Loaded {
            texture,
            width: w,
            height: h,
            byte_len,
        } => {
            let image_rect = fit_rect_preserving_aspect(rect.shrink(6.0), *w as f32, *h as f32);
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            painter.image(texture.id(), image_rect, uv, egui::Color32::WHITE);
            let node_label = format!(
                "OpenPose preview openpose_png=loaded dimensions={}x{} bytes={} artifact_ref={} artifact_status={} keypoint_source=artifact_png_bytes source_ref={} rig_id={} yaw_deg={:.0} pitch_deg={:.0} zoom={:.2} markers={}",
                w,
                h,
                byte_len,
                artifact_ref,
                artifact_status,
                source_ref,
                rig_id.unwrap_or("<none>"),
                yaw,
                pitch,
                zoom,
                marker_layer_summary(face, body, hands)
            );
            emit_value_node(
                ui.ctx(),
                response.id,
                accesskit::Role::Group,
                author_id,
                &node_label,
                &node_label,
            );
            return;
        }
        PoseSourceRender::DecodeError(err) => {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Exported OpenPose PNG could not be decoded",
                egui::FontId::proportional(13.0),
                palette.text_subtle,
            );
            let node_label = format!(
                "OpenPose preview openpose_png=decode_error error={} artifact_ref={} artifact_status={} keypoint_source={} source_ref={} rig_id={} yaw_deg={:.0} pitch_deg={:.0} zoom={:.2} markers={}",
                err,
                artifact_ref,
                artifact_status,
                bound_source,
                source_ref,
                rig_id.unwrap_or("<none>"),
                yaw,
                pitch,
                zoom,
                marker_layer_summary(face, body, hands)
            );
            emit_value_node(
                ui.ctx(),
                response.id,
                accesskit::Role::Group,
                author_id,
                &node_label,
                &node_label,
            );
            return;
        }
        PoseSourceRender::Empty => {}
    }

    let body_pts = posekit_viewport_points(openpose_json, "pose_keypoints_2d");
    let face_pts = posekit_viewport_points(openpose_json, "face_keypoints_2d");
    let left_hand = posekit_viewport_points(openpose_json, "hand_left_keypoints_2d");
    let right_hand = posekit_viewport_points(openpose_json, "hand_right_keypoints_2d");

    let (canvas_w, canvas_h) = posekit_viewport_canvas(openpose_json);
    let content = rect.shrink(6.0);
    let scale = (content.width() / canvas_w).min(content.height() / canvas_h);
    let origin = content.center() - egui::vec2(canvas_w * scale * 0.5, canvas_h * scale * 0.5);
    let map = |p: &(f32, f32, f32)| origin + egui::vec2(p.0 * scale, p.1 * scale);

    let cyan = egui::Color32::from_rgb(70, 220, 255);
    let amber = egui::Color32::from_rgb(255, 190, 80);
    let green = egui::Color32::from_rgb(120, 255, 150);

    // Body bones (COCO-18 connectivity — identical pairs to the backend PNG renderer).
    if body_pts.len() >= POSEKIT_BODY_KEYPOINT_COUNT {
        for (a, b) in POSEKIT_VIEWPORT_BODY_BONES {
            let (from, to) = (body_pts[a], body_pts[b]);
            if posekit_point_visible(from) && posekit_point_visible(to) {
                painter.line_segment([map(&from), map(&to)], egui::Stroke::new(2.0, cyan));
            }
        }
        for point in &body_pts {
            if posekit_point_visible(*point) {
                painter.circle_filled(map(point), 3.5, amber);
            }
        }
    }
    // Face joints.
    for point in &face_pts {
        if posekit_point_visible(*point) {
            painter.circle_filled(map(point), 2.4, amber);
        }
    }
    // Hands (finger bones + joints), mirroring the backend hand connectivity.
    for hand in [&left_hand, &right_hand] {
        draw_pose_viewport_hand(&painter, hand, &map, green);
    }

    let body_visible = body_pts
        .iter()
        .filter(|p| posekit_point_visible(**p))
        .count();
    let face_visible = face_pts
        .iter()
        .filter(|p| posekit_point_visible(**p))
        .count();
    let left_visible = left_hand
        .iter()
        .filter(|p| posekit_point_visible(**p))
        .count();
    let right_visible = right_hand
        .iter()
        .filter(|p| posekit_point_visible(**p))
        .count();
    let rendered = body_visible + face_visible + left_visible + right_visible;
    // Signature over the ACTUAL rendered body coordinates: it changes whenever the real keypoints move
    // (e.g. a yaw rotation), which is what proves the viewport is bound to the generated keypoints and
    // not to a fixed schematic.
    let signature_source = body_pts
        .iter()
        .map(|(x, y, c)| format!("{x:.1},{y:.1},{c:.2}"))
        .collect::<Vec<_>>()
        .join(";");
    let signature = stable_posekit_hash(&signature_source);

    let openpose_png_state = if artifact_status.contains("fetch failed") {
        "fetch_failed"
    } else if artifact_ref == "<none>" {
        "empty"
    } else if artifact_status.contains("Fetching") {
        "fetching"
    } else {
        "empty"
    };
    let node_label = format!(
        "OpenPose preview bound_openpose=real_keypoints openpose_png={} artifact_ref={} artifact_status={} keypoint_source={} keypoints_body={} keypoints_face={} keypoints_hand_left={} keypoints_hand_right={} rendered_points={} keypoint_signature={} source_ref={} rig_id={} yaw_deg={:.0} pitch_deg={:.0} zoom={:.2} markers={}",
        openpose_png_state,
        artifact_ref,
        artifact_status,
        bound_source,
        body_visible,
        face_visible,
        left_visible,
        right_visible,
        rendered,
        &signature[..12],
        source_ref,
        rig_id.unwrap_or("<none>"),
        yaw,
        pitch,
        zoom,
        marker_layer_summary(face, body, hands)
    );
    emit_value_node(
        ui.ctx(),
        response.id,
        accesskit::Role::Group,
        author_id,
        &node_label,
        &node_label,
    );
}

/// COCO-18 body bone connectivity, identical to the backend `render_posekit_openpose_png` pairs.
const POSEKIT_VIEWPORT_BODY_BONES: [(usize, usize); 17] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 4),
    (1, 5),
    (5, 6),
    (6, 7),
    (1, 8),
    (8, 9),
    (9, 10),
    (1, 11),
    (11, 12),
    (12, 13),
    (0, 14),
    (14, 16),
    (0, 15),
    (15, 17),
];

/// Read one keypoint array (`people[0][field]`) from an OpenPose JSON as (x, y, confidence) triples.
/// Tolerant: a missing/short array yields an empty vec (the viewport simply draws nothing for it).
fn posekit_viewport_points(openpose_json: &serde_json::Value, field: &str) -> Vec<(f32, f32, f32)> {
    let Some(values) = openpose_json
        .get("people")
        .and_then(|people| people.as_array())
        .and_then(|people| people.first())
        .and_then(|person| person.get(field))
        .and_then(|value| value.as_array())
    else {
        return Vec::new();
    };
    values
        .chunks_exact(3)
        .map(|chunk| {
            (
                chunk[0].as_f64().unwrap_or(0.0) as f32,
                chunk[1].as_f64().unwrap_or(0.0) as f32,
                chunk[2].as_f64().unwrap_or(0.0) as f32,
            )
        })
        .collect()
}

/// Canvas size from the OpenPose JSON, defaulting to the 768×768 export canvas.
fn posekit_viewport_canvas(openpose_json: &serde_json::Value) -> (f32, f32) {
    let canvas = openpose_json.get("canvas");
    let width = canvas
        .and_then(|c| c.get("width"))
        .and_then(|v| v.as_f64())
        .unwrap_or(POSEKIT_EXPORT_WIDTH as f64) as f32;
    let height = canvas
        .and_then(|c| c.get("height"))
        .and_then(|v| v.as_f64())
        .unwrap_or(POSEKIT_EXPORT_HEIGHT as f64) as f32;
    (width.max(1.0), height.max(1.0))
}

/// An OpenPose point is drawn only when confident and positive (matches the backend visibility rule).
fn posekit_point_visible(point: (f32, f32, f32)) -> bool {
    point.2 > 0.0 && point.0 > 0.0 && point.1 > 0.0
}

/// Draw one hand (21 keypoints) with the backend finger connectivity plus joint discs.
fn draw_pose_viewport_hand(
    painter: &egui::Painter,
    points: &[(f32, f32, f32)],
    map: &impl Fn(&(f32, f32, f32)) -> egui::Pos2,
    color: egui::Color32,
) {
    if points.len() >= POSEKIT_HAND_KEYPOINT_COUNT {
        for finger in 0..5 {
            let base = 1 + finger * 4;
            let segments = [
                (0, base),
                (base, base + 1),
                (base + 1, base + 2),
                (base + 2, base + 3),
            ];
            for (a, b) in segments {
                let (from, to) = (points[a], points[b]);
                if posekit_point_visible(from) && posekit_point_visible(to) {
                    painter.line_segment([map(&from), map(&to)], egui::Stroke::new(1.5, color));
                }
            }
        }
    }
    for point in points {
        if posekit_point_visible(*point) {
            painter.circle_filled(map(point), 2.0, color);
        }
    }
}

/// Fit a `w×h` image into `bounds` preserving aspect ratio, centered.
fn fit_rect_preserving_aspect(bounds: egui::Rect, w: f32, h: f32) -> egui::Rect {
    if w <= 0.0 || h <= 0.0 {
        return bounds;
    }
    let scale = (bounds.width() / w).min(bounds.height() / h);
    let size = egui::vec2(w * scale, h * scale);
    egui::Rect::from_center_size(bounds.center(), size)
}

/// Stable non-cryptographic hash of raw bytes, used to detect source-image byte changes for the
/// texture cache.
fn stable_bytes_hash(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn emit_node(
    ctx: &egui::Context,
    id: egui::Id,
    role: accesskit::Role,
    author_id: &str,
    label: &str,
    selected: bool,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(role);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        if selected {
            node.set_selected(true);
        }
        if matches!(
            role,
            accesskit::Role::Tab
                | accesskit::Role::Button
                | accesskit::Role::CheckBox
                | accesskit::Role::ListItem
        ) {
            node.add_action(accesskit::Action::Click);
        }
        if matches!(role, accesskit::Role::TextInput | accesskit::Role::Slider) {
            node.add_action(accesskit::Action::Focus);
        }
    });
}

fn emit_draggable_list_item_node(
    ctx: &egui::Context,
    id: egui::Id,
    author_id: &str,
    label: &str,
    description: &str,
    selected: bool,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let description = description.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_description(description.clone());
        if selected {
            node.set_selected(true);
        }
        node.add_action(accesskit::Action::Click);
    });
}

fn emit_value_node(
    ctx: &egui::Context,
    id: egui::Id,
    role: accesskit::Role,
    author_id: &str,
    label: &str,
    value: &str,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let value = value.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(role);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(value.clone());
        if matches!(role, accesskit::Role::TextInput | accesskit::Role::Slider) {
            node.add_action(accesskit::Action::Focus);
        }
    });
}

/// Publish a `Role::Slider` AccessKit node for a real `egui::Slider`, carrying the stable `author_id`,
/// a numeric value/min/max/step, and the `SetValue` + `Focus` actions. The explicit `SetValue` action
/// is what lets [`crate::mcp::action::resolve_target`] accept `argus.set_value` on the slider and what
/// egui's built-in slider consumes to apply the numeric value out-of-process. Mirrors the
/// splitter/scrollbar-rail numeric-node pattern (`split_layout.rs`, `rails.rs`).
fn emit_pose_slider_node(
    ctx: &egui::Context,
    id: egui::Id,
    author_id: &str,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Slider);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_numeric_value(value);
        node.set_min_numeric_value(min);
        node.set_max_numeric_value(max);
        node.set_numeric_value_step(step);
        // A string mirror of the numeric value so plain value-readers (and the existing text-oriented
        // snapshot assertions) still see a stable readout.
        node.set_value(format!("{value:.2}"));
        node.add_action(accesskit::Action::SetValue);
        node.add_action(accesskit::Action::Focus);
    });
}
