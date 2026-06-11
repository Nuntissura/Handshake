//! WP-KERNEL-009 MT-065 CRDTAndConcurrencyCore-065-ActorSiteIdModel.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11
//! "AI edit proposals, graph mutation proposals, relationship extraction,
//! auto-linking, auto-tagging, and manual edits MUST leave actor, source
//! span, state-vector, validation, denial, or promotion receipts." A receipt
//! can only be attributed when every CRDT update and EventLedger event
//! carries a stable, typed actor id and a stable CRDT site id.
//!
//! Actor-kind vocabulary is pinned to the MT-041 swarm lease/checkpoint seed
//! (`operator|local_model|cloud_model|validator|system`) so AgentLaneLease
//! rows (MT-076), SwarmCheckpoints (MT-079), graph mutation proposals
//! (MT-068), and AI edit proposals (MT-074) all attribute work with the same
//! tokens that CRDT update records persist.
//!
//! Storage authority: actor ids are persisted inside PostgreSQL rows
//! (`kernel_crdt_updates.actor_id/actor_kind`, `kernel_event_ledger.actor_*`,
//! and the WP-009 `knowledge_crdt_*` tables added by this MT group). There is
//! no sidecar or file-based actor registry.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::kernel::KernelActor;

use super::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};

pub const KNOWLEDGE_ACTOR_ID_SCHEMA_ID: &str = "hsk.kernel.knowledge_actor_id@1";
pub const KNOWLEDGE_SITE_ID_DERIVATION_V1: &str = "hsk-knowledge-site-v1";

/// Typed actor kinds allowed to touch ProjectKnowledgeIndex CRDT state.
///
/// Tokens match the MT-041 `swarm_lease_checkpoint_contract_seed` actor_kind
/// vocabulary exactly; they are also the values persisted in
/// `kernel_crdt_updates.actor_kind` for WP-009 documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeActorKind {
    /// The human operator (UI-driven edits, approvals, manual merges).
    Operator,
    /// A Handshake-managed local model session (GGUF/llama/candle lanes).
    LocalModel,
    /// A cloud model session routed through a provider adapter.
    CloudModel,
    /// A validation lane actor (WP validators, integration validators).
    Validator,
    /// Handshake itself (migrations, sweeps, recovery, projections).
    System,
}

impl KnowledgeActorKind {
    pub const ALL: [KnowledgeActorKind; 5] = [
        KnowledgeActorKind::Operator,
        KnowledgeActorKind::LocalModel,
        KnowledgeActorKind::CloudModel,
        KnowledgeActorKind::Validator,
        KnowledgeActorKind::System,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::LocalModel => "local_model",
            Self::CloudModel => "cloud_model",
            Self::Validator => "validator",
            Self::System => "system",
        }
    }

    pub fn parse(value: &str) -> Result<Self, KnowledgeActorIdError> {
        match value {
            "operator" => Ok(Self::Operator),
            "local_model" => Ok(Self::LocalModel),
            "cloud_model" => Ok(Self::CloudModel),
            "validator" => Ok(Self::Validator),
            "system" => Ok(Self::System),
            _ => Err(KnowledgeActorIdError::UnknownActorKind {
                found: value.to_string(),
            }),
        }
    }

    /// Model-driven actors require AI-proposal receipts (spec 2.3.13.11);
    /// operator and validator actors review/decide instead.
    pub fn is_model(&self) -> bool {
        matches!(self, Self::LocalModel | Self::CloudModel)
    }
}

/// Typed parse/format errors for [`KnowledgeActorIdV1`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeActorIdError {
    MissingSeparator { found: String },
    UnknownActorKind { found: String },
    EmptyIdent,
    IdentTooLong { len: usize, max: usize },
    IdentBadChar { found: char },
}

impl std::fmt::Display for KnowledgeActorIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSeparator { found } => {
                write!(f, "actor id '{found}' is missing the '<kind>:<ident>' separator")
            }
            Self::UnknownActorKind { found } => write!(
                f,
                "unknown actor kind '{found}' (expected operator|local_model|cloud_model|validator|system)"
            ),
            Self::EmptyIdent => write!(f, "actor ident must not be empty"),
            Self::IdentTooLong { len, max } => {
                write!(f, "actor ident length {len} exceeds maximum {max}")
            }
            Self::IdentBadChar { found } => write!(
                f,
                "actor ident contains forbidden character '{found}' (allowed: A-Z a-z 0-9 . _ -)"
            ),
        }
    }
}

impl std::error::Error for KnowledgeActorIdError {}

pub const KNOWLEDGE_ACTOR_IDENT_MAX_LEN: usize = 128;

/// Stable typed actor id: `<kind>:<ident>`, e.g. `local_model:qwen3-coder-a3`,
/// `operator:ilja`, `validator:wp-validator-1`, `system:handshake-recovery`.
///
/// Serialized as the canonical string form so PostgreSQL TEXT columns,
/// EventLedger payloads, and JSON API envelopes all carry the identical
/// representation; parse/format round-trip is proven in tests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KnowledgeActorIdV1 {
    kind: KnowledgeActorKind,
    ident: String,
}

impl KnowledgeActorIdV1 {
    pub fn new(
        kind: KnowledgeActorKind,
        ident: impl Into<String>,
    ) -> Result<Self, KnowledgeActorIdError> {
        let ident = ident.into();
        validate_actor_ident(&ident)?;
        Ok(Self { kind, ident })
    }

    pub fn kind(&self) -> KnowledgeActorKind {
        self.kind
    }

    pub fn ident(&self) -> &str {
        &self.ident
    }

    /// Canonical `<kind>:<ident>` form persisted in actor_id columns.
    pub fn canonical(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.ident)
    }

    pub fn parse(value: &str) -> Result<Self, KnowledgeActorIdError> {
        let (kind_token, ident) =
            value
                .split_once(':')
                .ok_or_else(|| KnowledgeActorIdError::MissingSeparator {
                    found: value.to_string(),
                })?;
        let kind = KnowledgeActorKind::parse(kind_token)?;
        validate_actor_ident(ident)?;
        Ok(Self {
            kind,
            ident: ident.to_string(),
        })
    }

    /// Map onto the EventLedger actor taxonomy so every CRDT-side event
    /// carries a `KernelActor` consistent with the rest of the kernel.
    pub fn to_kernel_actor(&self) -> KernelActor {
        match self.kind {
            KnowledgeActorKind::Operator => KernelActor::Operator(self.canonical()),
            KnowledgeActorKind::LocalModel | KnowledgeActorKind::CloudModel => {
                KernelActor::ModelAdapter(self.canonical())
            }
            KnowledgeActorKind::Validator => KernelActor::ValidationRunner(self.canonical()),
            KnowledgeActorKind::System => KernelActor::System(self.canonical()),
        }
    }
}

impl std::fmt::Display for KnowledgeActorIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonical())
    }
}

impl TryFrom<String> for KnowledgeActorIdV1 {
    type Error = KnowledgeActorIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<KnowledgeActorIdV1> for String {
    fn from(value: KnowledgeActorIdV1) -> Self {
        value.canonical()
    }
}

fn validate_actor_ident(ident: &str) -> Result<(), KnowledgeActorIdError> {
    if ident.is_empty() {
        return Err(KnowledgeActorIdError::EmptyIdent);
    }
    if ident.len() > KNOWLEDGE_ACTOR_IDENT_MAX_LEN {
        return Err(KnowledgeActorIdError::IdentTooLong {
            len: ident.len(),
            max: KNOWLEDGE_ACTOR_IDENT_MAX_LEN,
        });
    }
    if let Some(found) = ident
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
    {
        return Err(KnowledgeActorIdError::IdentBadChar { found });
    }
    Ok(())
}

/// Stable per-(workspace, crdt document, actor) CRDT site identity.
///
/// `site_id` keys [`super::state_vector::KnowledgeStateVectorV1`] clocks;
/// `yjs_client_id` is the deterministic u32 the frontend Yjs doc must adopt
/// (`new Y.Doc({ clientID })`) so browser updates and backend records agree
/// on attribution without a registration round-trip.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KnowledgeSiteIdV1 {
    pub site_id: String,
    pub yjs_client_id: u32,
    pub derivation: String,
}

/// Deterministically derive the CRDT site identity for an actor on a
/// document. Same inputs always produce the same site id, so a restarted
/// no-context session recovers its site without any local state.
pub fn derive_knowledge_site_id(
    workspace_id: &str,
    crdt_document_id: &str,
    actor: &KnowledgeActorIdV1,
) -> KnowledgeSiteIdV1 {
    let mut hasher = Sha256::new();
    hasher.update(KNOWLEDGE_SITE_ID_DERIVATION_V1.as_bytes());
    hasher.update(b"|");
    hasher.update(workspace_id.as_bytes());
    hasher.update(b"|");
    hasher.update(crdt_document_id.as_bytes());
    hasher.update(b"|");
    hasher.update(actor.canonical().as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    let yjs_client_id = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    KnowledgeSiteIdV1 {
        site_id: format!("site-{}", &hex[..16]),
        yjs_client_id,
        derivation: KNOWLEDGE_SITE_ID_DERIVATION_V1.to_string(),
    }
}

/// Deterministic authority links for a ProjectKnowledgeIndex rich-document
/// draft. Every field is non-empty (CrdtWorkspaceIdentityV1 validation) and
/// derived purely from the document identity, so any session can rebuild the
/// identical links from the MT contract plus document ids alone.
pub fn knowledge_document_authority_links(
    document_id: &str,
    crdt_document_id: &str,
    action_trace_id: &str,
) -> CrdtAuthorityLinksV1 {
    CrdtAuthorityLinksV1 {
        work_item_id: format!("knowledge-doc:{document_id}"),
        action_trace_id: action_trace_id.to_string(),
        artifact_proposal_id: format!("knowledge-draft:{crdt_document_id}"),
        role_mailbox_thread_id: format!("knowledge-doc-thread:{document_id}"),
        dcc_projection_id: format!("knowledge-doc-dcc:{document_id}"),
        event_ledger_stream_id: format!("knowledge-crdt:{crdt_document_id}"),
    }
}

/// Build a fully-populated [`CrdtWorkspaceIdentityV1`] for a typed actor on
/// a knowledge rich-document draft. This is the single integration point
/// between the MT-065 actor/site model and the existing kernel CRDT layer:
/// actor_id is the canonical typed form, actor_kind the typed token, and
/// crdt_site_id/crdt_client_id the deterministic derivation.
pub fn knowledge_crdt_identity(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    document_schema_id: &str,
    actor: &KnowledgeActorIdV1,
    action_trace_id: &str,
) -> CrdtWorkspaceIdentityV1 {
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, actor);
    CrdtWorkspaceIdentityV1 {
        schema_id: "hsk.kernel.crdt_workspace_identity@1".to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        actor_id: actor.canonical(),
        actor_kind: actor.kind().as_str().to_string(),
        crdt_site_id: site.site_id,
        crdt_client_id: site.yjs_client_id.to_string(),
        document_schema_id: document_schema_id.to_string(),
        authority_links: knowledge_document_authority_links(
            document_id,
            crdt_document_id,
            action_trace_id,
        ),
    }
}
