use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use handshake_member_enrollment::{
    command_plan::{encode_enrollment, EnrollmentDescriptor, Timestamp},
    invitation::validate_expiry,
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;
use zeroize::Zeroizing;
use crate::model_runtime::cloud::secrets_vault::SecretsVault;

// NEW host encoding choice, not preexisting Source030 law.
const SECRET_DOMAIN: &[u8] = b"handshake.identity-enrollment.invitation-secret.v1\0";
const PURPOSE: &str = "identity_enrollment";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SecretRecoveryError { InvalidOriginal, Conflict, Unknown }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InvalidInvitation;
impl std::fmt::Display for InvalidInvitation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invitation denied")
    }
}
impl std::error::Error for InvalidInvitation {}

// Projection of the ACTUAL canonical original row. This struct is data, not
// authorization. The protected Account owner must supply the original read.
pub(crate) struct OriginalInvitationRow<'a> {
    pub operation_id: Uuid,
    pub descriptor_hex: &'a str,
    pub descriptor_sha256: &'a str,
    pub account_id: &'a str,
    pub principal_id: &'a str,
    pub access_space_id: &'a str,
    pub account_name: &'a str,
    pub token_digest_hex: &'a str,
    pub purpose: &'a str,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    pub status: &'a str,
}

// No public fields/Clone/Debug. A successful crypto observation is not an
// enrollment permission, recipient identity proof, or one-use CAS receipt.
pub(crate) struct BoundOriginalInvitation<'a> {
    descriptor: &'a EnrollmentDescriptor,
    canonical_descriptor: Vec<u8>,
    descriptor_digest: [u8; 32],
    secret_reference: String,
}
pub(crate) struct RecoveredInvitationSecret<'a> {
    binding: &'a BoundOriginalInvitation<'a>,
    secret: Zeroizing<String>,
}
pub(crate) struct RecipientSecretObservation<'a> {
    binding: &'a BoundOriginalInvitation<'a>,
}

fn decode_digest(text: &str) -> Result<[u8; 32], ()> {
    if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(());
    }
    let mut digest = [0; 32];
    hex::decode_to_slice(text, &mut digest).map_err(|_| ())?;
    Ok(digest)
}

fn canonical_secret(text: &str) -> Result<Zeroizing<Vec<u8>>, ()> {
    if text.len() != 43 { return Err(()); }
    let bytes = Zeroizing::new(URL_SAFE_NO_PAD.decode(text).map_err(|_| ())?);
    if bytes.len() != 32 { return Err(()); }
    let canonical = Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes.as_slice()));
    if canonical.as_bytes().ct_eq(text.as_bytes()).unwrap_u8() != 1 {
        return Err(());
    }
    Ok(bytes)
}

fn purpose_digest(operation_id: Uuid, purpose: &str, secret: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(SECRET_DOMAIN);
    hash.update((purpose.len() as u64).to_be_bytes());
    hash.update(purpose.as_bytes());
    hash.update(operation_id.as_bytes());
    hash.update((secret.len() as u64).to_be_bytes());
    hash.update(secret);
    hash.finalize().into()
}

fn currently_valid(descriptor: &EnrollmentDescriptor) -> Result<(), InvalidInvitation> {
    validate_expiry(descriptor).map_err(|_| InvalidInvitation)?;
    let now = chrono::Utc::now();
    let now = Timestamp::new(now.timestamp(), now.timestamp_subsec_nanos())
        .map_err(|_| InvalidInvitation)?;
    if now < descriptor.issued_at() || now >= descriptor.expires_at() {
        return Err(InvalidInvitation);
    }
    Ok(())
}

// NEW deterministic lane convention within the caller's configured OS vault.
// The actual original row has no secret-reference field yet. A durable owner
// must commit this exact reference with the original immutable intent.
fn original_secret_reference(operation_id: Uuid) -> String {
    format!("invitation/{PURPOSE}/{}", operation_id.hyphenated())
}

pub(crate) fn bind_original<'a>(
    descriptor: &'a EnrollmentDescriptor,
    row: &OriginalInvitationRow<'_>,
    prospective_secret_reference: &str,
) -> Result<BoundOriginalInvitation<'a>, InvalidInvitation> {
    currently_valid(descriptor)?;
    let canonical = encode_enrollment(descriptor).map_err(|_| InvalidInvitation)?;
    let descriptor_digest: [u8; 32] = Sha256::digest(&canonical).into();
    let row_digest = decode_digest(row.descriptor_sha256).map_err(|_| InvalidInvitation)?;
    let row_token_digest = decode_digest(row.token_digest_hex).map_err(|_| InvalidInvitation)?;
    let row_descriptor = hex::decode(row.descriptor_hex).map_err(|_| InvalidInvitation)?;
    let staged = descriptor.staged();
    if descriptor.operation_id().is_nil()
        || row.operation_id != descriptor.operation_id()
        || row.account_id != staged.account_id()
        || row.principal_id != staged.principal_id()
        || row.access_space_id != staged.access_space_id()
        || row.account_name != descriptor.account_name()
        || row.purpose != PURPOSE || descriptor.invitation_purpose() != PURPOSE
        || row.issued_at != descriptor.issued_at()
        || row.expires_at != descriptor.expires_at()
        || row.status != "active"
        || row_descriptor != canonical
        || descriptor_digest.ct_eq(&row_digest).unwrap_u8() != 1
        || descriptor.token_digest().ct_eq(&row_token_digest).unwrap_u8() != 1
        || prospective_secret_reference != original_secret_reference(row.operation_id)
    {
        return Err(InvalidInvitation);
    }
    Ok(BoundOriginalInvitation {
        descriptor, canonical_descriptor: canonical, descriptor_digest,
        secret_reference: prospective_secret_reference.to_owned(),
    })
}

pub(crate) fn recover_existing<'a>(
    vault: &dyn SecretsVault,
    binding: &'a BoundOriginalInvitation<'a>,
) -> Result<RecoveredInvitationSecret<'a>, SecretRecoveryError> {
    currently_valid(binding.descriptor).map_err(|_| SecretRecoveryError::InvalidOriginal)?;
    // No put/delete/mint fallback. Missing and backend failure remain Unknown.
    let secret = vault.get(&binding.secret_reference).map_err(|_| SecretRecoveryError::Unknown)?;
    let bytes = canonical_secret(secret.as_str()).map_err(|_| SecretRecoveryError::Conflict)?;
    let digest = purpose_digest(binding.descriptor.operation_id(), PURPOSE, bytes.as_slice());
    if digest.ct_eq(binding.descriptor.token_digest()).unwrap_u8() != 1 {
        return Err(SecretRecoveryError::Conflict);
    }
    currently_valid(binding.descriptor).map_err(|_| SecretRecoveryError::InvalidOriginal)?;
    Ok(RecoveredInvitationSecret { binding, secret })
}

pub(crate) fn verify_presented<'a>(
    binding: &'a BoundOriginalInvitation<'a>,
    presented: &str,
    purpose: &str,
    operation_id: Uuid,
) -> Result<RecipientSecretObservation<'a>, InvalidInvitation> {
    currently_valid(binding.descriptor)?;
    let bytes = canonical_secret(presented).map_err(|_| InvalidInvitation)?;
    let digest = purpose_digest(operation_id, purpose, bytes.as_slice());
    let digest_matches = digest.ct_eq(binding.descriptor.token_digest()).unwrap_u8() == 1;
    if operation_id != binding.descriptor.operation_id() || purpose != PURPOSE || !digest_matches {
        return Err(InvalidInvitation);
    }
    currently_valid(binding.descriptor)?;
    Ok(RecipientSecretObservation { binding })
}

impl RecoveredInvitationSecret<'_> {
    // Deliberate recipient delivery only. No implicit Debug/Display/Serialize.
    pub(crate) fn recipient_token(&self) -> &str { self.secret.as_str() }
    pub(crate) fn operation_id(&self) -> Uuid { self.binding.descriptor.operation_id() }
}
impl RecipientSecretObservation<'_> {
    pub(crate) fn operation_id(&self) -> Uuid { self.binding.descriptor.operation_id() }
    pub(crate) fn descriptor_digest(&self) -> &[u8; 32] { &self.binding.descriptor_digest }
    pub(crate) fn descriptor_bytes(&self) -> &[u8] { &self.binding.canonical_descriptor }
}

// First issuance is deliberately NOT implemented with SecretsVault::put.
// Required future original-owner protocol:
// 1. Atomically compare-create full immutable issue intent under the ORIGINAL
//    operation, original tuple/issued_at/expires_at and exact vault reference.
//    Hold exclusive serialized write custody across every process using it.
// 2. Lookup vault FIRST. Existing canonical secret must match retained digest;
//    return the same secret. Never overwrite a mismatch or an unknown get.
// 3. Only a durable NeverWriteStarted record may prepare CSPRNG 32-byte data.
//    Durably retain its exact purpose_digest and WriteStarted transition BEFORE
//    invoking put, and keep the Zeroizing candidate in the original owner.
// 4. put is overwrite-capable, NOT create-if-absent. Use it only with actual
//    exclusive custody and exact original intent. Cache whole result/error;
//    reread and cryptographically compare exact bytes before publication.
// 5. Missing secret after WriteStarted is Unknown, including after restart.
//    Never remint, reset expiry, generate a replacement operation or infer abort.
// 6. Commit only digest/ref/descriptor/audit through the actual protected owner.
//    Missing durable row/ref binding or custody supplier blocks first issuance.

pub(crate) struct OwnedRecipientSecretObservation {
    descriptor: EnrollmentDescriptor,
    canonical_descriptor: Vec<u8>,
    descriptor_digest: [u8; 32],
}
impl RecipientSecretObservation<'_> {
    // Only an actual successful verify_presented observation can enter here.
    // Own the verified immutable input; no borrowed/self-referential escape.
    pub(crate) fn into_owned(self) -> OwnedRecipientSecretObservation {
        OwnedRecipientSecretObservation {
            descriptor: self.binding.descriptor.clone(),
            canonical_descriptor: self.binding.canonical_descriptor.clone(),
            descriptor_digest: self.binding.descriptor_digest,
        }
    }
}
impl OwnedRecipientSecretObservation {
    pub(crate) fn require_same_descriptor(
        &self, descriptor: &EnrollmentDescriptor,
    ) -> Result<(), InvalidInvitation> {
        let encoded = encode_enrollment(descriptor).map_err(|_| InvalidInvitation)?;
        let digest: [u8; 32] = Sha256::digest(&encoded).into();
        if descriptor != &self.descriptor || encoded != self.canonical_descriptor
            || digest.ct_eq(&self.descriptor_digest).unwrap_u8() != 1 {
            return Err(InvalidInvitation);
        }
        Ok(())
    }
    pub(crate) fn operation_id(&self) -> Uuid { self.descriptor.operation_id() }
    pub(crate) fn descriptor_bytes(&self) -> &[u8] { &self.canonical_descriptor }
    pub(crate) fn descriptor_digest(&self) -> &[u8; 32] { &self.descriptor_digest }
}
// This retained observation proves the actual earlier secret comparison only.
// Current canonical/source/expiry checks, private admission installation, real
// authenticated physical transaction and one-use engine CAS remain mandatory.
