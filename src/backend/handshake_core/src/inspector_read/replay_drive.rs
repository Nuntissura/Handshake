#![cfg(feature = "inspector")]

//! MT-030 research basis:
//! - Axum 0.7 routes POST handlers with `routing::post`, matching the existing
//!   MT-029 server style.
//! - Axum body-consuming extractors must be the last handler argument; this
//!   handler accepts the raw body so syntactically valid but forbidden shapes can
//!   return the required 403 instead of the default JSON extractor status.
//! - Serde `deny_unknown_fields` is used for strict envelope deserialization;
//!   top-level shape is checked before deserialization so extra fields are a
//!   fail-closed parallel-mutation attempt.
//!
//! Per-run shared-secret authentication (MT-029 spec §6.5.5 + MT-030 CRIT-4
//! remediation): every inspector launch generates a fresh CSPRNG secret
//! (256 bits from the OS RNG; previously a UUIDv7, which leaked the launch
//! timestamp) that callers MUST present (a) as the
//! `X-Handshake-Inspector-Secret` HTTP header enforced by the server on
//! EVERY route — reads, the event-stream upgrade, and the replay-drive
//! write — via a single router-wide middleware (MT-029) and (b) as the
//! HMAC-SHA256 key over the envelope contents (MT-030). The keyless `sha256(canonical_json({schema_id,
//! signer, write_box}))` signature it replaces let any local process forge a
//! passing envelope under any signer name; HMAC-SHA256 binds the signature to
//! knowledge of the per-launch secret, so forgery requires reading operator-
//! visible launch state rather than just inspecting bytes on the wire.

use std::sync::Arc;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::kernel::{
    action_catalog::{kernel002_action_catalog, KernelActionCatalogV1, KernelCatalogActionV1},
    context_bundle::canonical_json_bytes,
    write_boxes::{validate_write_box_common, WriteBoxCommon},
    KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
};

pub const REPLAY_DRIVE_ROUTE: &str = "/inspector/v1/replay-drive";
pub const WRITE_BOX_V1_ENVELOPE_SCHEMA_ID: &str = "hsk.write_box_v1_envelope@1";
pub const REPLAY_DRIVE_RESPONSE_SCHEMA_ID: &str = "hsk.inspector.replay_drive.response@1";

/// Header name the inspector server requires on every mutating request
/// (currently `replay_drive`); value must match the launch-time per-run
/// secret in hex form. MT-029 spec §6.5.5.
pub const PER_RUN_SECRET_HEADER: &str = "x-handshake-inspector-secret";

/// Number of CSPRNG bytes [`PerRunSecret::generate`] draws per inspector
/// launch. 32 bytes = 256 bits of entropy, comfortably above the 128-bit
/// minimum the MT-029 secret must carry and matching the HMAC-SHA256 block
/// the secret keys.
pub const PER_RUN_SECRET_LEN: usize = 32;

/// Per-launch shared secret used both as the inspector HTTP header value
/// (MT-029) and as the HMAC-SHA256 key over the envelope canonical bytes
/// (MT-030). The bytes are drawn from the operating-system CSPRNG
/// (`getrandom`/`OsRng`) so they carry no launch-time structure and cannot
/// be derived from the process start instant; they rotate per process
/// start. The operator sees the hex form printed at launch and can include
/// it when calling the inspector plane.
///
/// History: this previously used `Uuid::now_v7()`, whose 128-bit value
/// embeds a millisecond launch timestamp and a small random tail (~48
/// unpredictable bits in the worst case), letting an attacker who knows
/// roughly when the inspector launched brute-force the remaining bits. The
/// CSPRNG draw closes that predictability gap.
#[derive(Clone, PartialEq, Eq)]
pub struct PerRunSecret {
    bytes: Vec<u8>,
}

impl PerRunSecret {
    /// Generate a fresh secret from the OS cryptographically-secure RNG.
    /// Call once per inspector launch. Draws [`PER_RUN_SECRET_LEN`] bytes
    /// (256 bits) of OS entropy; panics only if the OS RNG is unavailable,
    /// which is a fail-closed posture — the inspector must not bind with a
    /// weak or empty secret.
    pub fn generate() -> Self {
        let mut bytes = vec![0u8; PER_RUN_SECRET_LEN];
        getrandom::getrandom(&mut bytes)
            .expect("OS CSPRNG must be available to mint the inspector per-run secret");
        Self { bytes }
    }

    /// Construct from raw bytes — used for tests that need determinism
    /// and for the inspector launch path that reads an operator-supplied
    /// secret. Production callers should prefer [`PerRunSecret::generate`].
    /// Accepts any byte length so 16-byte legacy fixtures and 32-byte
    /// CSPRNG secrets both round-trip.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }

    /// Construct from a hex string (even number of hex chars). Returns
    /// `None` on odd length, empty input, or non-hex input.
    pub fn from_hex(hex_str: &str) -> Option<Self> {
        let bytes = hex::decode(hex_str).ok()?;
        if bytes.is_empty() {
            return None;
        }
        Some(Self { bytes })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Length of the secret in bytes. Used by tests/diagnostics to assert
    /// the secret carries at least 128 bits of entropy.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }
}

impl std::fmt::Debug for PerRunSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the secret in default Debug output. Use `to_hex`
        // explicitly at the operator-visible launch log site.
        f.debug_struct("PerRunSecret").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayDriveRequest {
    pub action_id: String,
    pub envelope: WriteBoxV1Envelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WriteBoxV1Envelope {
    pub schema_id: String,
    pub signer: String,
    pub signature: String,
    pub write_box: WriteBoxCommon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedWriteBoxV1Envelope {
    pub signer: String,
    pub write_box: WriteBoxCommon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayDriveResponse {
    pub schema_id: String,
    pub action_id: String,
    pub status: String,
    pub result_schema_id: String,
    pub write_box_id: String,
    pub event: ReplayDriveEventReceipt,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayDriveEventReceipt {
    pub event_id: String,
    pub event_type: String,
    pub idempotency_key: String,
    pub envelope_signer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayDriveError {
    MalformedJson,
    ForbiddenShape,
    InvalidSignature,
    UnknownAction { action_id: String },
    Dispatch { message: String },
    EventLedger { message: String },
}

pub trait WriteBoxEnvelopeVerifier: Send + Sync {
    fn verify(
        &self,
        envelope: WriteBoxV1Envelope,
    ) -> Result<VerifiedWriteBoxV1Envelope, ReplayDriveError>;
}

/// HMAC-SHA256 verifier keyed to the per-launch shared secret. Constructed
/// at inspector launch and injected into [`ReplayDriveService`]. Replaces
/// the prior keyless verifier that accepted any envelope whose recomputed
/// SHA-256 hash matched its own bytes.
#[derive(Debug)]
pub struct Kernel002WriteBoxEnvelopeVerifier {
    secret: Arc<PerRunSecret>,
}

impl Kernel002WriteBoxEnvelopeVerifier {
    pub fn new(secret: Arc<PerRunSecret>) -> Self {
        Self { secret }
    }
}

impl WriteBoxEnvelopeVerifier for Kernel002WriteBoxEnvelopeVerifier {
    fn verify(
        &self,
        envelope: WriteBoxV1Envelope,
    ) -> Result<VerifiedWriteBoxV1Envelope, ReplayDriveError> {
        if envelope.schema_id != WRITE_BOX_V1_ENVELOPE_SCHEMA_ID {
            return Err(ReplayDriveError::InvalidSignature);
        }
        if envelope.signer.trim().is_empty() {
            return Err(ReplayDriveError::InvalidSignature);
        }
        validate_write_box_common(&envelope.write_box)
            .map_err(|_| ReplayDriveError::InvalidSignature)?;

        let expected =
            expected_write_box_v1_signature(&self.secret, &envelope.signer, &envelope.write_box);
        if !constant_time_eq(envelope.signature.as_bytes(), expected.as_bytes()) {
            return Err(ReplayDriveError::InvalidSignature);
        }

        Ok(VerifiedWriteBoxV1Envelope {
            signer: envelope.signer,
            write_box: envelope.write_box,
        })
    }
}

pub trait ReplayDriveActionDispatcher: Send + Sync {
    fn dispatch(
        &self,
        action: &KernelCatalogActionV1,
        envelope: &VerifiedWriteBoxV1Envelope,
    ) -> Result<Value, ReplayDriveError>;
}

#[derive(Debug, Default)]
pub struct CatalogMetadataReplayDriveDispatcher;

impl ReplayDriveActionDispatcher for CatalogMetadataReplayDriveDispatcher {
    fn dispatch(
        &self,
        action: &KernelCatalogActionV1,
        envelope: &VerifiedWriteBoxV1Envelope,
    ) -> Result<Value, ReplayDriveError> {
        Ok(json!({
            "dispatched_through": "KernelActionCatalogV1",
            "action_id": action.action_id,
            "title": action.title,
            "input_schema_id": action.input_schema_id,
            "result_schema_id": action.result_schema_id,
            "write_box_id": envelope.write_box.write_box_id,
        }))
    }
}

pub trait ReplayDriveEventLedger: Send + Sync {
    fn append(&self, event: NewKernelEvent) -> Result<KernelEvent, ReplayDriveError>;
}

#[derive(Debug, Default)]
pub struct ProjectingReplayDriveEventLedger;

impl ReplayDriveEventLedger for ProjectingReplayDriveEventLedger {
    fn append(&self, event: NewKernelEvent) -> Result<KernelEvent, ReplayDriveError> {
        Ok(KernelEvent::from_new(event))
    }
}

#[derive(Clone)]
pub struct ReplayDriveService {
    catalog: KernelActionCatalogV1,
    verifier: Arc<dyn WriteBoxEnvelopeVerifier>,
    dispatcher: Arc<dyn ReplayDriveActionDispatcher>,
    event_ledger: Arc<dyn ReplayDriveEventLedger>,
}

impl ReplayDriveService {
    pub fn new(
        catalog: KernelActionCatalogV1,
        verifier: Arc<dyn WriteBoxEnvelopeVerifier>,
        dispatcher: Arc<dyn ReplayDriveActionDispatcher>,
        event_ledger: Arc<dyn ReplayDriveEventLedger>,
    ) -> Self {
        Self {
            catalog,
            verifier,
            dispatcher,
            event_ledger,
        }
    }

    /// Convenience constructor that wires the default catalog, dispatcher,
    /// and projection event-ledger with a [`Kernel002WriteBoxEnvelopeVerifier`]
    /// keyed to the supplied per-run secret.
    pub fn with_per_run_secret(secret: Arc<PerRunSecret>) -> Self {
        Self::new(
            kernel002_action_catalog(),
            Arc::new(Kernel002WriteBoxEnvelopeVerifier::new(secret)),
            Arc::new(CatalogMetadataReplayDriveDispatcher),
            Arc::new(ProjectingReplayDriveEventLedger),
        )
    }

    pub fn handle_body(&self, body: &str) -> Result<ReplayDriveResponse, ReplayDriveError> {
        let value: Value =
            serde_json::from_str(body).map_err(|_| ReplayDriveError::MalformedJson)?;
        if !has_exact_replay_drive_shape(&value) {
            return Err(ReplayDriveError::ForbiddenShape);
        }
        let request: ReplayDriveRequest =
            serde_json::from_value(value).map_err(|_| ReplayDriveError::ForbiddenShape)?;
        self.dispatch(request)
    }

    pub fn dispatch(
        &self,
        request: ReplayDriveRequest,
    ) -> Result<ReplayDriveResponse, ReplayDriveError> {
        let action = self.catalog.action(&request.action_id).ok_or_else(|| {
            ReplayDriveError::UnknownAction {
                action_id: request.action_id.clone(),
            }
        })?;
        let verified = self.verifier.verify(request.envelope)?;
        let result = self.dispatcher.dispatch(action, &verified)?;
        let event = self.emit_replay_drive_event(action, &verified)?;

        Ok(ReplayDriveResponse {
            schema_id: REPLAY_DRIVE_RESPONSE_SCHEMA_ID.to_string(),
            action_id: action.action_id.to_string(),
            status: "dispatched".to_string(),
            result_schema_id: action.result_schema_id.clone(),
            write_box_id: verified.write_box.write_box_id.clone(),
            event: ReplayDriveEventReceipt {
                event_id: event.event_id,
                event_type: KernelEventType::InspectorReplayDrive.as_str().to_string(),
                idempotency_key: event.idempotency_key,
                envelope_signer: verified.signer,
            },
            result,
        })
    }

    fn emit_replay_drive_event(
        &self,
        action: &KernelCatalogActionV1,
        envelope: &VerifiedWriteBoxV1Envelope,
    ) -> Result<KernelEvent, ReplayDriveError> {
        let write_box = &envelope.write_box;
        let idempotency_key = format!(
            "INSPECTOR_REPLAY_DRIVE:{}:{}",
            action.action_id, write_box.replay_metadata.idempotency_key
        );
        let event = NewKernelEvent::builder(
            write_box.replay_metadata.replay_plan_ref.clone(),
            write_box.workspace_id.clone(),
            KernelEventType::InspectorReplayDrive,
            KernelActor::System("inspector_replay_drive".to_string()),
        )
        .aggregate("write_box", write_box.write_box_id.clone())
        .idempotency_key(idempotency_key)
        .correlation_id(write_box.replay_metadata.idempotency_key.clone())
        .source_component("inspector_replay_drive")
        .payload(json!({
            "event_type": KernelEventType::InspectorReplayDrive.as_str(),
            "action_id": action.action_id,
            "envelope_signer": envelope.signer,
            "write_box_id": write_box.write_box_id,
            "result": "dispatched",
        }))
        .build()
        .map_err(|error| ReplayDriveError::EventLedger {
            message: error.to_string(),
        })?;
        self.event_ledger.append(event)
    }
}

/// HMAC-SHA256 of the canonical envelope payload under the per-launch
/// secret. Output is prefixed with `hmac-sha256:` so callers can tell at a
/// glance that this is keyed authentication, not the legacy keyless
/// `sha256:` shape.
pub fn expected_write_box_v1_signature(
    secret: &PerRunSecret,
    signer: &str,
    write_box: &WriteBoxCommon,
) -> String {
    let payload = json!({
        "schema_id": WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
        "signer": signer,
        "write_box": write_box,
    });
    let canonical = canonical_json_bytes(&payload);
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(&canonical);
    let tag = mac.finalize().into_bytes();
    format!("hmac-sha256:{}", hex::encode(tag))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    if left.len() != right.len() {
        return false;
    }
    left.ct_eq(right).into()
}

fn has_exact_replay_drive_shape(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == 2 && object.contains_key("action_id") && object.contains_key("envelope")
}
