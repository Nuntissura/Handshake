//! Immutable comparison data and a single canonical codec. No value here grants authority.
use std::{error::Error, fmt};
use uuid::Uuid;

pub const MAX_STRING_BYTES: usize = 1_048_576;
pub const MAX_VECTOR_ITEMS: usize = 1_024;
pub const MAX_ENCODED_BYTES: usize = 16_777_216;
pub const CODEC_PREFIX: &[u8] = b"hsk.member_enrollment.command_plan@1\0";
pub const MEMBER_CAPABILITIES: [&str; 3] = ["model_lane.navigation.read", "fs.write", "diagnostics.read"];
const FIXED_ENROLLMENT_TAGS: &[u8] = &[1, 2, 1, 2, 3, 1, 2, 3];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    Empty(&'static str),
    Limit(&'static str),
    NilOperation,
    OperationAlias,
    InvalidTimestamp,
    Truncated,
    InvalidUtf8,
    InvalidTag,
    TrailingBytes,
    AuditMismatch,
}
impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl Error for PlanError {}

fn text(value: &str, field: &'static str) -> Result<(), PlanError> {
    if value.is_empty() { return Err(PlanError::Empty(field)); }
    if value.len() > MAX_STRING_BYTES { return Err(PlanError::Limit(field)); }
    Ok(())
}
fn operation(value: Uuid) -> Result<(), PlanError> {
    if value.is_nil() { Err(PlanError::NilOperation) } else { Ok(()) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp { unix_seconds: i64, nanoseconds: u32 }
impl Timestamp {
    pub fn new(unix_seconds: i64, nanoseconds: u32) -> Result<Self, PlanError> {
        if nanoseconds >= 1_000_000_000 { return Err(PlanError::InvalidTimestamp); }
        Ok(Self { unix_seconds, nanoseconds })
    }
    pub fn unix_seconds(self) -> i64 { self.unix_seconds }
    pub fn nanoseconds(self) -> u32 { self.nanoseconds }
    pub fn checked_add_seconds(self, seconds: i64) -> Result<Self, PlanError> {
        Self::new(self.unix_seconds.checked_add(seconds).ok_or(PlanError::InvalidTimestamp)?, self.nanoseconds)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityScope {
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
    access_space_id: String, workspace_id: String,
}
impl IdentityScope {
    pub fn new(owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
        access_space_id: String, workspace_id: String) -> Result<Self, PlanError>
    {
        for value in [&owner_account_id, &actor_principal_id, &authenticated_session_id, &access_space_id, &workspace_id] {
            text(value, "identity")?;
        }
        Ok(Self { owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id })
    }
    pub fn owner_account_id(&self) -> &str { &self.owner_account_id }
    pub fn actor_principal_id(&self) -> &str { &self.actor_principal_id }
    pub fn authenticated_session_id(&self) -> &str { &self.authenticated_session_id }
    pub fn access_space_id(&self) -> &str { &self.access_space_id }
    pub fn workspace_id(&self) -> &str { &self.workspace_id }
    fn ordered(&self) -> [&str; 5] {
        [&self.owner_account_id, &self.actor_principal_id, &self.authenticated_session_id, &self.access_space_id, &self.workspace_id]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedIdentity { account_id: String, principal_id: String, access_space_id: String }
impl StagedIdentity {
    pub fn new(account_id: String, principal_id: String, access_space_id: String) -> Result<Self, PlanError> {
        for value in [&account_id, &principal_id, &access_space_id] { text(value, "staged identity")?; }
        Ok(Self { account_id, principal_id, access_space_id })
    }
    pub fn account_id(&self) -> &str { &self.account_id }
    pub fn principal_id(&self) -> &str { &self.principal_id }
    pub fn access_space_id(&self) -> &str { &self.access_space_id }
    fn ordered(&self) -> [&str; 3] { [&self.account_id, &self.principal_id, &self.access_space_id] }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRef {
    resource_id: String, grant_id: String, grant_version: i64,
    policy_versions: Vec<i64>, revocation_epochs: Vec<i64>, grant_expires_at: Option<Timestamp>,
}
impl SourceRef {
    pub fn new(resource_id: String, grant_id: String, grant_version: i64,
        policy_versions: Vec<i64>, revocation_epochs: Vec<i64>, grant_expires_at: Option<Timestamp>)
        -> Result<Self, PlanError>
    {
        text(&resource_id, "resource")?; text(&grant_id, "grant")?;
        if policy_versions.len() > MAX_VECTOR_ITEMS || revocation_epochs.len() > MAX_VECTOR_ITEMS {
            return Err(PlanError::Limit("source vector"));
        }
        Ok(Self { resource_id, grant_id, grant_version, policy_versions, revocation_epochs, grant_expires_at })
    }
    pub fn resource_id(&self) -> &str { &self.resource_id }
    pub fn grant_id(&self) -> &str { &self.grant_id }
    pub fn grant_version(&self) -> i64 { self.grant_version }
    pub fn policy_versions(&self) -> &[i64] { &self.policy_versions }
    pub fn revocation_epochs(&self) -> &[i64] { &self.revocation_epochs }
    pub fn grant_expires_at(&self) -> Option<Timestamp> { self.grant_expires_at }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnrollmentDescriptor {
    operation_id: Uuid, original_enroller: IdentityScope, staged: StagedIdentity,
    account_name: String, administrator_resource: String,
    enroll_source: SourceRef, assignment_source: SourceRef,
    issued_at: Timestamp, expires_at: Timestamp, session_expires_at: Timestamp,
    policy_expires_at: Option<Timestamp>, token_digest: [u8; 32],
}
impl EnrollmentDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(operation_id: Uuid, original_enroller: IdentityScope, staged: StagedIdentity,
        account_name: String, administrator_resource: String, enroll_source: SourceRef, assignment_source: SourceRef,
        issued_at: Timestamp, expires_at: Timestamp, session_expires_at: Timestamp,
        policy_expires_at: Option<Timestamp>, token_digest: [u8; 32]) -> Result<Self, PlanError>
    {
        operation(operation_id)?; text(&account_name, "account name")?; text(&administrator_resource, "administrator resource")?;
        Ok(Self { operation_id, original_enroller, staged, account_name, administrator_resource,
            enroll_source, assignment_source, issued_at, expires_at, session_expires_at, policy_expires_at, token_digest })
    }
    pub fn format_version(&self) -> u8 { 1 }
    pub fn operation_id(&self) -> Uuid { self.operation_id }
    pub fn original_enroller(&self) -> &IdentityScope { &self.original_enroller }
    pub fn staged(&self) -> &StagedIdentity { &self.staged }
    pub fn account_name(&self) -> &str { &self.account_name }
    pub fn administrator_resource(&self) -> &str { &self.administrator_resource }
    pub fn enroll_source(&self) -> &SourceRef { &self.enroll_source }
    pub fn assignment_source(&self) -> &SourceRef { &self.assignment_source }
    pub fn issued_at(&self) -> Timestamp { self.issued_at }
    pub fn expires_at(&self) -> Timestamp { self.expires_at }
    pub fn session_expires_at(&self) -> Timestamp { self.session_expires_at }
    pub fn policy_expires_at(&self) -> Option<Timestamp> { self.policy_expires_at }
    pub fn token_digest(&self) -> &[u8; 32] { &self.token_digest }
    pub fn profile_id(&self) -> &'static str { "MemberEnrollment" }
    pub fn capabilities(&self) -> &'static [&'static str; 3] { &MEMBER_CAPABILITIES }
    pub fn invitation_purpose(&self) -> &'static str { "identity_enrollment" }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationRequest {
    request_id: Uuid, enrollment_operation_id: Uuid, staged: StagedIdentity,
    administrator_resource: String, original_descriptor_digest: [u8; 32],
    current_revoker: IdentityScope, source: SourceRef, requested_at: Timestamp,
}
impl RevocationRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(request_id: Uuid, enrollment_operation_id: Uuid, staged: StagedIdentity,
        administrator_resource: String, original_descriptor_digest: [u8; 32],
        current_revoker: IdentityScope, source: SourceRef, requested_at: Timestamp) -> Result<Self, PlanError>
    {
        operation(request_id)?; operation(enrollment_operation_id)?;
        if request_id == enrollment_operation_id { return Err(PlanError::OperationAlias); }
        text(&administrator_resource, "administrator resource")?;
        Ok(Self { request_id, enrollment_operation_id, staged, administrator_resource,
            original_descriptor_digest, current_revoker, source, requested_at })
    }
    pub fn request_id(&self) -> Uuid { self.request_id }
    pub fn enrollment_operation_id(&self) -> Uuid { self.enrollment_operation_id }
    pub fn staged(&self) -> &StagedIdentity { &self.staged }
    pub fn administrator_resource(&self) -> &str { &self.administrator_resource }
    pub fn original_descriptor_digest(&self) -> &[u8; 32] { &self.original_descriptor_digest }
    pub fn current_revoker(&self) -> &IdentityScope { &self.current_revoker }
    pub fn source(&self) -> &SourceRef { &self.source }
    pub fn requested_at(&self) -> Timestamp { self.requested_at }
}

struct Encoder(Vec<u8>);
impl Encoder {
    fn new(kind: u8) -> Result<Self, PlanError> {
        let mut result = Self(Vec::new()); result.bytes(CODEC_PREFIX)?; result.bytes(&[kind])?; Ok(result)
    }
    fn bytes(&mut self, bytes: &[u8]) -> Result<(), PlanError> {
        let end = self.0.len().checked_add(bytes.len()).ok_or(PlanError::Limit("total"))?;
        if end > MAX_ENCODED_BYTES { return Err(PlanError::Limit("total")); }
        self.0.extend_from_slice(bytes); Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), PlanError> {
        text(value, "string")?;
        let length = u32::try_from(value.len()).map_err(|_| PlanError::Limit("string"))?;
        self.bytes(&length.to_le_bytes())?; self.bytes(value.as_bytes())
    }
    fn timestamp(&mut self, value: Timestamp) -> Result<(), PlanError> {
        self.bytes(&value.unix_seconds.to_le_bytes())?; self.bytes(&value.nanoseconds.to_le_bytes())
    }
    fn optional_timestamp(&mut self, value: Option<Timestamp>) -> Result<(), PlanError> {
        match value { None => self.bytes(&[0]), Some(value) => { self.bytes(&[1])?; self.timestamp(value) } }
    }
    fn vector(&mut self, values: &[i64]) -> Result<(), PlanError> {
        if values.len() > MAX_VECTOR_ITEMS { return Err(PlanError::Limit("vector")); }
        self.bytes(&(values.len() as u32).to_le_bytes())?;
        for value in values { self.bytes(&value.to_le_bytes())?; }
        Ok(())
    }
    fn scope(&mut self, scope: &IdentityScope) -> Result<(), PlanError> {
        for value in scope.ordered() { self.string(value)?; } Ok(())
    }
    fn staged(&mut self, staged: &StagedIdentity) -> Result<(), PlanError> {
        for value in staged.ordered() { self.string(value)?; } Ok(())
    }
    fn source(&mut self, source: &SourceRef) -> Result<(), PlanError> {
        self.string(&source.resource_id)?; self.string(&source.grant_id)?;
        self.bytes(&source.grant_version.to_le_bytes())?;
        self.vector(&source.policy_versions)?; self.vector(&source.revocation_epochs)?;
        self.optional_timestamp(source.grant_expires_at)
    }
}

struct Decoder<'a> { input: &'a [u8], position: usize }
impl<'a> Decoder<'a> {
    fn new(input: &'a [u8], kind: u8) -> Result<Self, PlanError> {
        if input.len() > MAX_ENCODED_BYTES { return Err(PlanError::Limit("total")); }
        let mut result = Self { input, position: 0 };
        result.expect(CODEC_PREFIX)?; result.expect(&[kind])?; Ok(result)
    }
    fn bytes(&mut self, length: usize) -> Result<&'a [u8], PlanError> {
        let end = self.position.checked_add(length).ok_or(PlanError::Truncated)?;
        let bytes = self.input.get(self.position..end).ok_or(PlanError::Truncated)?;
        self.position = end; Ok(bytes)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], PlanError> {
        self.bytes(N)?.try_into().map_err(|_| PlanError::Truncated)
    }
    fn expect(&mut self, expected: &[u8]) -> Result<(), PlanError> {
        if self.bytes(expected.len())? != expected { return Err(PlanError::InvalidTag); } Ok(())
    }
    fn string(&mut self) -> Result<String, PlanError> {
        let length = u32::from_le_bytes(self.array()?) as usize;
        if length > MAX_STRING_BYTES { return Err(PlanError::Limit("string")); }
        let value = std::str::from_utf8(self.bytes(length)?).map_err(|_| PlanError::InvalidUtf8)?.to_owned();
        text(&value, "string")?; Ok(value)
    }
    fn timestamp(&mut self) -> Result<Timestamp, PlanError> {
        Timestamp::new(i64::from_le_bytes(self.array()?), u32::from_le_bytes(self.array()?))
    }
    fn optional_timestamp(&mut self) -> Result<Option<Timestamp>, PlanError> {
        match self.array::<1>()?[0] {
            0 => Ok(None), 1 => Ok(Some(self.timestamp()?)), _ => Err(PlanError::InvalidTag),
        }
    }
    fn vector(&mut self) -> Result<Vec<i64>, PlanError> {
        let count = u32::from_le_bytes(self.array()?) as usize;
        if count > MAX_VECTOR_ITEMS { return Err(PlanError::Limit("vector")); }
        let bytes = count.checked_mul(8).ok_or(PlanError::Limit("vector"))?;
        if bytes > self.input.len().saturating_sub(self.position) { return Err(PlanError::Truncated); }
        (0..count).map(|_| Ok(i64::from_le_bytes(self.array()?))).collect()
    }
    fn scope(&mut self) -> Result<IdentityScope, PlanError> {
        IdentityScope::new(self.string()?, self.string()?, self.string()?, self.string()?, self.string()?)
    }
    fn staged(&mut self) -> Result<StagedIdentity, PlanError> {
        StagedIdentity::new(self.string()?, self.string()?, self.string()?)
    }
    fn source(&mut self) -> Result<SourceRef, PlanError> {
        SourceRef::new(self.string()?, self.string()?, i64::from_le_bytes(self.array()?),
            self.vector()?, self.vector()?, self.optional_timestamp()?)
    }
    fn finish(self) -> Result<(), PlanError> {
        if self.position == self.input.len() { Ok(()) } else { Err(PlanError::TrailingBytes) }
    }
}

pub fn encode_enrollment(value: &EnrollmentDescriptor) -> Result<Vec<u8>, PlanError> {
    let mut output = Encoder::new(1)?;
    output.bytes(value.operation_id.as_bytes())?; output.scope(&value.original_enroller)?;
    output.staged(&value.staged)?; output.string(&value.account_name)?; output.string(&value.administrator_resource)?;
    output.source(&value.enroll_source)?; output.source(&value.assignment_source)?;
    output.timestamp(value.issued_at)?; output.timestamp(value.expires_at)?;
    output.timestamp(value.session_expires_at)?; output.optional_timestamp(value.policy_expires_at)?;
    output.bytes(&value.token_digest)?; output.bytes(FIXED_ENROLLMENT_TAGS)?;
    Ok(output.0)
}
pub fn decode_enrollment(input: &[u8]) -> Result<EnrollmentDescriptor, PlanError> {
    let mut source = Decoder::new(input, 1)?;
    let value = EnrollmentDescriptor::new(
        Uuid::from_bytes(source.array()?), source.scope()?, source.staged()?, source.string()?, source.string()?,
        source.source()?, source.source()?, source.timestamp()?, source.timestamp()?, source.timestamp()?,
        source.optional_timestamp()?, source.array()?,
    )?;
    source.expect(FIXED_ENROLLMENT_TAGS)?; source.finish()?; Ok(value)
}
pub fn encode_revocation(value: &RevocationRequest) -> Result<Vec<u8>, PlanError> {
    let mut output = Encoder::new(2)?;
    output.bytes(value.request_id.as_bytes())?; output.bytes(value.enrollment_operation_id.as_bytes())?;
    output.staged(&value.staged)?; output.string(&value.administrator_resource)?;
    output.bytes(&value.original_descriptor_digest)?; output.scope(&value.current_revoker)?;
    output.source(&value.source)?; output.timestamp(value.requested_at)?; output.bytes(&[3])?; Ok(output.0)
}
pub fn decode_revocation(input: &[u8]) -> Result<RevocationRequest, PlanError> {
    let mut source = Decoder::new(input, 2)?;
    let value = RevocationRequest::new(
        Uuid::from_bytes(source.array()?), Uuid::from_bytes(source.array()?), source.staged()?, source.string()?,
        source.array()?, source.scope()?, source.source()?, source.timestamp()?,
    )?;
    source.expect(&[3])?; source.finish()?; Ok(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdminAction { Create, Update }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind { Enroll, AssignFixedMemberProfile, RevokeEnrollment }
impl CommandKind {
    pub fn resource_kind(self) -> &'static str { "AccountAdministration" }
    pub fn capability_id(self) -> &'static str {
        match self {
            Self::Enroll => "account.identity.enroll",
            Self::AssignFixedMemberProfile => "account.identity.member_profile.assign",
            Self::RevokeEnrollment => "account.identity.enrollment.revoke",
        }
    }
    pub fn action(self) -> AdminAction {
        match self { Self::Enroll => AdminAction::Create, _ => AdminAction::Update }
    }
    pub fn purpose(self) -> &'static str {
        match self {
            Self::Enroll => "identity_enrollment",
            Self::AssignFixedMemberProfile => "member_profile_assignment",
            Self::RevokeEnrollment => "identity_enrollment_revocation",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdminIntent {
    kind: CommandKind, operation_id: Uuid, enrollment_operation_id: Uuid,
    authorizer: IdentityScope, staged: StagedIdentity, administrator_resource: String, source: SourceRef,
}
impl AdminIntent {
    pub fn kind(&self) -> CommandKind { self.kind }
    pub fn operation_id(&self) -> Uuid { self.operation_id }
    pub fn enrollment_operation_id(&self) -> Uuid { self.enrollment_operation_id }
    pub fn authorizer(&self) -> &IdentityScope { &self.authorizer }
    pub fn staged(&self) -> &StagedIdentity { &self.staged }
    pub fn administrator_resource(&self) -> &str { &self.administrator_resource }
    pub fn source(&self) -> &SourceRef { &self.source }
    pub fn profile_id(&self) -> &'static str { "MemberEnrollment" }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnrollmentPlan { descriptor: EnrollmentDescriptor, intents: [AdminIntent; 2] }
impl EnrollmentPlan {
    pub fn descriptor(&self) -> &EnrollmentDescriptor { &self.descriptor }
    pub fn intents(&self) -> &[AdminIntent; 2] { &self.intents }
}
pub fn prepare_enrollment_plan(descriptor: &EnrollmentDescriptor) -> EnrollmentPlan {
    let intent = |kind, source: &SourceRef| AdminIntent {
        kind, operation_id: descriptor.operation_id, enrollment_operation_id: descriptor.operation_id,
        authorizer: descriptor.original_enroller.clone(), staged: descriptor.staged.clone(),
        administrator_resource: descriptor.administrator_resource.clone(), source: source.clone(),
    };
    EnrollmentPlan {
        descriptor: descriptor.clone(),
        intents: [
            intent(CommandKind::Enroll, &descriptor.enroll_source),
            intent(CommandKind::AssignFixedMemberProfile, &descriptor.assignment_source),
        ],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationPlan { request: RevocationRequest, intent: AdminIntent }
impl RevocationPlan {
    pub fn request(&self) -> &RevocationRequest { &self.request }
    pub fn intent(&self) -> &AdminIntent { &self.intent }
}
pub fn prepare_revocation_plan(request: &RevocationRequest) -> RevocationPlan {
    RevocationPlan { request: request.clone(), intent: AdminIntent {
        kind: CommandKind::RevokeEnrollment, operation_id: request.request_id,
        enrollment_operation_id: request.enrollment_operation_id,
        authorizer: request.current_revoker.clone(), staged: request.staged.clone(),
        administrator_resource: request.administrator_resource.clone(), source: request.source.clone(),
    }}
}

/// Checks raw linkage only; the external digest must be computed and authenticated by the host.
pub fn validate_revocation_link(request: &RevocationRequest, original: &EnrollmentDescriptor,
    expected_descriptor_digest: &[u8; 32]) -> Result<(), PlanError>
{
    if request.enrollment_operation_id != original.operation_id || request.staged != original.staged
        || request.administrator_resource != original.administrator_resource
        || &request.original_descriptor_digest != expected_descriptor_digest
    { return Err(PlanError::AuditMismatch); }
    Ok(())
}

/// Raw observed comparison data, never a proof of a decision or an EventLedger append.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedAdminAudit {
    operation_id: Uuid, enrollment_operation_id: Uuid, authorizer: IdentityScope,
    staged: StagedIdentity, administrator_resource: String, resource_kind: String,
    capability_id: String, action: AdminAction, purpose: String, profile_id: String,
}
impl ObservedAdminAudit {
    #[allow(clippy::too_many_arguments)]
    pub fn new(operation_id: Uuid, enrollment_operation_id: Uuid, authorizer: IdentityScope,
        staged: StagedIdentity, administrator_resource: String, resource_kind: String,
        capability_id: String, action: AdminAction, purpose: String, profile_id: String) -> Result<Self, PlanError>
    {
        operation(operation_id)?; operation(enrollment_operation_id)?;
        for value in [&administrator_resource, &resource_kind, &capability_id, &purpose, &profile_id] {
            text(value, "audit field")?;
        }
        Ok(Self { operation_id, enrollment_operation_id, authorizer, staged, administrator_resource,
            resource_kind, capability_id, action, purpose, profile_id })
    }
    pub fn operation_id(&self) -> Uuid { self.operation_id }
    pub fn enrollment_operation_id(&self) -> Uuid { self.enrollment_operation_id }
    pub fn authorizer(&self) -> &IdentityScope { &self.authorizer }
    pub fn staged(&self) -> &StagedIdentity { &self.staged }
    pub fn administrator_resource(&self) -> &str { &self.administrator_resource }
    pub fn resource_kind(&self) -> &str { &self.resource_kind }
    pub fn capability_id(&self) -> &str { &self.capability_id }
    pub fn action(&self) -> AdminAction { self.action }
    pub fn purpose(&self) -> &str { &self.purpose }
    pub fn profile_id(&self) -> &str { &self.profile_id }
}
pub fn validate_created_audit(plan: &EnrollmentPlan, observed: &[ObservedAdminAudit]) -> Result<(), PlanError> {
    if observed.len() != 2 { return Err(PlanError::AuditMismatch); }
    for (intent, audit) in plan.intents.iter().zip(observed) {
        if audit.operation_id != intent.operation_id || audit.enrollment_operation_id != intent.enrollment_operation_id
            || audit.authorizer != intent.authorizer || audit.staged != intent.staged
            || audit.administrator_resource != intent.administrator_resource
            || audit.resource_kind != intent.kind.resource_kind()
            || audit.capability_id != intent.kind.capability_id() || audit.action != intent.kind.action()
            || audit.purpose != intent.kind.purpose() || audit.profile_id != intent.profile_id()
        { return Err(PlanError::AuditMismatch); }
    }
    Ok(())
}

/// Eligibility is a set; only the codec's fixed representation has a fixed byte order.
pub fn is_member_capability_set(values: &[&str]) -> bool {
    values.len() == 3 && MEMBER_CAPABILITIES.iter().all(|expected| values.iter().filter(|value| *value == expected).count() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(seconds: i64, nanos: u32) -> Timestamp { Timestamp::new(seconds, nanos).unwrap() }
    fn scope() -> IdentityScope {
        IdentityScope::new("o".into(), "p".into(), "s".into(), "a".into(), "w".into()).unwrap()
    }
    fn staged() -> StagedIdentity { StagedIdentity::new("n".into(), "q".into(), "z".into()).unwrap() }
    fn source() -> SourceRef { SourceRef::new("r".into(), "g".into(), 0, vec![], vec![], None).unwrap() }
    fn descriptor() -> EnrollmentDescriptor {
        EnrollmentDescriptor::new(Uuid::from_bytes([1; 16]), scope(), staged(), "m".into(), "r".into(),
            source(), source(), time(0, 0), time(0, 0), time(0, 0), None, [0; 32]).unwrap()
    }
    fn revocation(original: &EnrollmentDescriptor) -> RevocationRequest {
        RevocationRequest::new(Uuid::from_bytes([2; 16]), original.operation_id(), original.staged().clone(),
            original.administrator_resource().into(), [4; 32], scope(), source(), time(0, 0)).unwrap()
    }
    fn unhex(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        value.as_bytes().chunks_exact(2).map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
        }).collect()
    }
    fn observed(intent: &AdminIntent) -> ObservedAdminAudit {
        ObservedAdminAudit::new(intent.operation_id, intent.enrollment_operation_id,
            intent.authorizer.clone(), intent.staged.clone(), intent.administrator_resource.clone(),
            intent.kind.resource_kind().into(), intent.kind.capability_id().into(), intent.kind.action(),
            intent.kind.purpose().into(), intent.profile_id().into()).unwrap()
    }

    #[test]
    fn enrollment_v1_golden_and_repeatable_roundtrip() {
        let mut expected = b"hsk.member_enrollment.command_plan@1\0".to_vec();
        expected.extend(unhex(concat!(
            "01", "01010101010101010101010101010101",
            "010000006f", "0100000070", "0100000073", "0100000061", "0100000077",
            "010000006e", "0100000071", "010000007a", "010000006d", "0100000072",
            "010000007201000000670000000000000000000000000000000000",
            "010000007201000000670000000000000000000000000000000000",
            "000000000000000000000000", "000000000000000000000000", "000000000000000000000000", "00",
            "0000000000000000000000000000000000000000000000000000000000000000", "0102010203010203"
        )));
        let value = descriptor();
        assert_eq!(encode_enrollment(&value).unwrap(), expected);
        assert_eq!(encode_enrollment(&value).unwrap(), expected);
        assert_eq!(decode_enrollment(&expected).unwrap(), value);
        assert_eq!(encode_enrollment(&decode_enrollment(&expected).unwrap()).unwrap(), expected);
    }

    #[test]
    fn revocation_v1_golden_preserves_distinct_operation_roles() {
        let original = descriptor();
        let value = revocation(&original);
        let mut expected = b"hsk.member_enrollment.command_plan@1\0".to_vec();
        expected.extend(unhex(concat!(
            "02", "02020202020202020202020202020202", "01010101010101010101010101010101",
            "010000006e", "0100000071", "010000007a", "0100000072",
            "0404040404040404040404040404040404040404040404040404040404040404",
            "010000006f", "0100000070", "0100000073", "0100000061", "0100000077",
            "010000007201000000670000000000000000000000000000000000",
            "000000000000000000000000", "03"
        )));
        assert_eq!(encode_revocation(&value).unwrap(), expected);
        assert_eq!(decode_revocation(&expected).unwrap(), value);
        assert_eq!(original.original_enroller(), &scope());
        assert_eq!(value.enrollment_operation_id(), original.operation_id());
        assert_ne!(value.request_id(), value.enrollment_operation_id());
        assert!(decode_enrollment(&expected).is_err());
        assert!(decode_revocation(&encode_enrollment(&original).unwrap()).is_err());
    }

    #[test]
    fn opaque_identity_utf8_and_lossless_signed_timestamps_are_not_normalized() {
        let mut value = descriptor();
        value.original_enroller.workspace_id = "workspace:text/非UUID ".into();
        value.original_enroller.owner_account_id = "canonical-opaque-owner".into();
        value.staged.account_id = "staged-account-ref".into();
        value.account_name = "Á / A\u{301} ".into();
        value.enroll_source.policy_versions = vec![-1, 0, i64::MAX];
        value.enroll_source.revocation_epochs = vec![i64::MIN, 2];
        value.enroll_source.grant_expires_at = Some(time(-1, 999_999_999));
        value.assignment_source.grant_expires_at = Some(time(11, 1));
        value.issued_at = time(-3, 7);
        value.expires_at = time(-2, 8);
        value.session_expires_at = time(22, 9);
        value.policy_expires_at = Some(time(33, 10));
        assert_eq!(decode_enrollment(&encode_enrollment(&value).unwrap()).unwrap(), value);
        assert_eq!(value.issued_at().checked_add_seconds(60).unwrap(), time(57, 7));
        assert!(Timestamp::new(0, 1_000_000_000).is_err());
        assert!(time(i64::MAX, 0).checked_add_seconds(60).is_err());
    }

    #[test]
    fn every_effect_field_changes_canonical_enrollment_bytes() {
        let base = descriptor();
        let bytes = encode_enrollment(&base).unwrap();
        let mut changes = Vec::new();
        macro_rules! changed {
            ($($field:ident).+, $value:expr) => {{
                let mut next = base.clone(); next.$($field).+ = $value; changes.push(next);
            }};
        }
        changed!(operation_id, Uuid::from_bytes([3; 16]));
        changed!(original_enroller.owner_account_id, "owner2".into());
        changed!(original_enroller.actor_principal_id, "principal2".into());
        changed!(original_enroller.authenticated_session_id, "session2".into());
        changed!(original_enroller.access_space_id, "space2".into());
        changed!(original_enroller.workspace_id, "workspace2".into());
        changed!(staged.account_id, "account2".into());
        changed!(staged.principal_id, "principal3".into());
        changed!(staged.access_space_id, "space3".into());
        changed!(account_name, "member2".into());
        changed!(administrator_resource, "admin2".into());
        changed!(enroll_source.resource_id, "resource2".into());
        changed!(enroll_source.grant_id, "grant2".into());
        changed!(enroll_source.grant_version, 7);
        changed!(enroll_source.policy_versions, vec![1]);
        changed!(enroll_source.revocation_epochs, vec![2]);
        changed!(enroll_source.grant_expires_at, Some(time(3, 4)));
        changed!(assignment_source.resource_id, "resource3".into());
        changed!(assignment_source.grant_id, "grant3".into());
        changed!(assignment_source.grant_version, 8);
        changed!(assignment_source.policy_versions, vec![3]);
        changed!(assignment_source.revocation_epochs, vec![4]);
        changed!(assignment_source.grant_expires_at, Some(time(5, 6)));
        changed!(issued_at, time(1, 2));
        changed!(expires_at, time(2, 3));
        changed!(session_expires_at, time(3, 4));
        changed!(policy_expires_at, Some(time(4, 5)));
        changed!(token_digest, [1; 32]);
        for value in changes {
            let changed = encode_enrollment(&value).unwrap();
            assert_ne!(changed, bytes);
            assert_eq!(decode_enrollment(&changed).unwrap(), value);
        }
    }

    #[test]
    fn decoder_rejects_all_truncations_unknown_tags_trailing_and_nil_operations() {
        let bytes = encode_enrollment(&descriptor()).unwrap();
        for end in 0..bytes.len() { assert!(decode_enrollment(&bytes[..end]).is_err(), "cut={end}"); }
        let revoked = encode_revocation(&revocation(&descriptor())).unwrap();
        for end in 0..revoked.len() { assert!(decode_revocation(&revoked[..end]).is_err(), "revoke cut={end}"); }
        let mut malformed = bytes.clone(); malformed.push(0);
        assert_eq!(decode_enrollment(&malformed), Err(PlanError::TrailingBytes));
        let mut malformed = revoked.clone(); malformed.push(0);
        assert_eq!(decode_revocation(&malformed), Err(PlanError::TrailingBytes));
        let mut malformed = bytes.clone(); malformed[CODEC_PREFIX.len()] = 3;
        assert_eq!(decode_enrollment(&malformed), Err(PlanError::InvalidTag));
        let mut malformed = bytes.clone(); malformed[CODEC_PREFIX.len() - 2] = b'2';
        assert_eq!(decode_enrollment(&malformed), Err(PlanError::InvalidTag));
        let mut malformed = bytes.clone();
        malformed[CODEC_PREFIX.len() + 1..CODEC_PREFIX.len() + 17].fill(0);
        assert_eq!(decode_enrollment(&malformed), Err(PlanError::NilOperation));
        for offset in bytes.len() - FIXED_ENROLLMENT_TAGS.len()..bytes.len() {
            let mut malformed = bytes.clone(); malformed[offset] = 255;
            assert_eq!(decode_enrollment(&malformed), Err(PlanError::InvalidTag));
        }
        let mut alias = revoked;
        alias[CODEC_PREFIX.len() + 1..CODEC_PREFIX.len() + 17].fill(1);
        assert_eq!(decode_revocation(&alias), Err(PlanError::OperationAlias));
    }

    #[test]
    fn decoder_enforces_lengths_utf8_optional_flags_nanoseconds_and_bounds() {
        let bytes = encode_enrollment(&descriptor()).unwrap();
        let body = CODEC_PREFIX.len() + 1 + 16;
        let mut invalid = bytes.clone(); invalid[body..body + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(decode_enrollment(&invalid), Err(PlanError::Limit("string")));
        let mut invalid = bytes.clone(); invalid[body + 4] = 255;
        assert_eq!(decode_enrollment(&invalid), Err(PlanError::InvalidUtf8));
        // Ten one-byte strings precede the two 27-byte source references in the golden body.
        let first_source = body + 50;
        let mut invalid = bytes.clone(); invalid[first_source + 18..first_source + 22].copy_from_slice(&1025u32.to_le_bytes());
        assert_eq!(decode_enrollment(&invalid), Err(PlanError::Limit("vector")));
        let mut invalid = bytes.clone(); invalid[first_source + 26] = 2;
        assert_eq!(decode_enrollment(&invalid), Err(PlanError::InvalidTag));
        let issued = first_source + 54;
        let mut invalid = bytes; invalid[issued + 8..issued + 12].copy_from_slice(&1_000_000_000u32.to_le_bytes());
        assert_eq!(decode_enrollment(&invalid), Err(PlanError::InvalidTimestamp));
        assert!(SourceRef::new("r".into(), "g".into(), 0, vec![0; MAX_VECTOR_ITEMS + 1], vec![], None).is_err());
        assert!(IdentityScope::new("x".repeat(MAX_STRING_BYTES + 1), "p".into(), "s".into(), "a".into(), "w".into()).is_err());
        let boundary = IdentityScope::new("x".repeat(MAX_STRING_BYTES), "p".into(), "s".into(), "a".into(), "w".into()).unwrap();
        let mut value = descriptor(); value.original_enroller = boundary;
        value.enroll_source.policy_versions = vec![0; MAX_VECTOR_ITEMS];
        assert_eq!(decode_enrollment(&encode_enrollment(&value).unwrap()).unwrap(), value);
        assert_eq!(decode_enrollment(&vec![0; MAX_ENCODED_BYTES + 1]), Err(PlanError::Limit("total")));
        let mut output = Encoder(Vec::new());
        output.bytes(&vec![0; MAX_ENCODED_BYTES]).unwrap();
        assert_eq!(output.bytes(&[0]), Err(PlanError::Limit("total")));
    }

    #[test]
    fn plans_require_separate_ordered_capabilities_and_raw_created_audit_pair() {
        let original = descriptor();
        let plan = prepare_enrollment_plan(&original);
        assert_eq!(plan.intents()[0].kind(), CommandKind::Enroll);
        assert_eq!(plan.intents()[1].kind(), CommandKind::AssignFixedMemberProfile);
        assert_eq!(plan.intents()[0].kind().action(), AdminAction::Create);
        assert_eq!(plan.intents()[1].kind().action(), AdminAction::Update);
        assert_ne!(plan.intents()[0].kind().capability_id(), plan.intents()[1].kind().capability_id());
        let good = plan.intents().iter().map(observed).collect::<Vec<_>>();
        validate_created_audit(&plan, &good).unwrap();
        assert!(validate_created_audit(&plan, &good[..1]).is_err());
        assert!(validate_created_audit(&plan, &[]).is_err());
        let mut extra = good.clone(); extra.push(good[1].clone());
        assert!(validate_created_audit(&plan, &extra).is_err());
        let mut swapped = good.clone(); swapped.swap(0, 1);
        assert!(validate_created_audit(&plan, &swapped).is_err());
        let mut duplicated = good.clone(); duplicated[1] = good[0].clone();
        assert!(validate_created_audit(&plan, &duplicated).is_err());
        macro_rules! reject {
            ($($field:ident).+, $value:expr) => {{
                let mut bad = good.clone(); bad[1].$($field).+ = $value;
                assert!(validate_created_audit(&plan, &bad).is_err());
            }};
        }
        reject!(operation_id, Uuid::from_bytes([9; 16]));
        reject!(enrollment_operation_id, Uuid::from_bytes([9; 16]));
        reject!(authorizer.owner_account_id, "other".into());
        reject!(authorizer.actor_principal_id, "other".into());
        reject!(authorizer.authenticated_session_id, "other".into());
        reject!(authorizer.access_space_id, "other".into());
        reject!(authorizer.workspace_id, "other".into());
        reject!(staged.account_id, "other".into());
        reject!(staged.principal_id, "other".into());
        reject!(staged.access_space_id, "other".into());
        reject!(administrator_resource, "other".into());
        reject!(resource_kind, "Workspace".into());
        reject!(capability_id, "account.identity.enroll".into());
        reject!(action, AdminAction::Create);
        reject!(purpose, "identity_enrollment".into());
        reject!(profile_id, "Operator".into());
        assert!(is_member_capability_set(&["fs.write", "diagnostics.read", "model_lane.navigation.read"]));
        assert!(!is_member_capability_set(&["fs.write", "fs.write", "model_lane.navigation.read"]));
        assert!(!is_member_capability_set(&["fs.write", "model_lane.navigation.read"]));
        assert!(!is_member_capability_set(&["fs.write", "diagnostics.read", "model_lane.navigation.read", "proc.exec"]));
    }

    #[test]
    fn revocation_checks_link_without_restamping_original_or_reusing_create() {
        let original = descriptor();
        let mut request = revocation(&original);
        request.current_revoker.actor_principal_id = "another-current-principal".into();
        validate_revocation_link(&request, &original, &[4; 32]).unwrap();
        let plan = prepare_revocation_plan(&request);
        assert_eq!(plan.intent().kind(), CommandKind::RevokeEnrollment);
        assert_eq!(plan.intent().kind().action(), AdminAction::Update);
        assert_eq!(plan.intent().kind().capability_id(), "account.identity.enrollment.revoke");
        assert_eq!(plan.intent().authorizer(), request.current_revoker());
        assert_ne!(plan.intent().authorizer(), original.original_enroller());
        assert_eq!(original.original_enroller(), &scope());
        assert!(validate_revocation_link(&request, &original, &[5; 32]).is_err());
        let mut bad = request.clone(); bad.enrollment_operation_id = Uuid::from_bytes([7; 16]);
        assert!(validate_revocation_link(&bad, &original, &[4; 32]).is_err());
        let mut bad = request.clone(); bad.staged.account_id = "other".into();
        assert!(validate_revocation_link(&bad, &original, &[4; 32]).is_err());
        let mut bad = request; bad.administrator_resource = "other".into();
        assert!(validate_revocation_link(&bad, &original, &[4; 32]).is_err());
    }

    #[test]
    fn every_revocation_field_is_bound_and_cannot_alias_enrollment() {
        let base = revocation(&descriptor());
        let bytes = encode_revocation(&base).unwrap();
        let mut changes = Vec::new();
        macro_rules! changed {
            ($($field:ident).+, $value:expr) => {{
                let mut next = base.clone(); next.$($field).+ = $value; changes.push(next);
            }};
        }
        changed!(request_id, Uuid::from_bytes([3; 16]));
        changed!(enrollment_operation_id, Uuid::from_bytes([4; 16]));
        changed!(staged.account_id, "account2".into());
        changed!(staged.principal_id, "principal2".into());
        changed!(staged.access_space_id, "space2".into());
        changed!(administrator_resource, "admin2".into());
        changed!(original_descriptor_digest, [5; 32]);
        changed!(current_revoker.owner_account_id, "owner2".into());
        changed!(current_revoker.actor_principal_id, "principal3".into());
        changed!(current_revoker.authenticated_session_id, "session2".into());
        changed!(current_revoker.access_space_id, "space3".into());
        changed!(current_revoker.workspace_id, "workspace2".into());
        changed!(source.resource_id, "resource2".into());
        changed!(source.grant_id, "grant2".into());
        changed!(source.grant_version, -7);
        changed!(source.policy_versions, vec![1, 2]);
        changed!(source.revocation_epochs, vec![3, 4]);
        changed!(source.grant_expires_at, Some(time(9, 1)));
        changed!(requested_at, time(8, 7));
        for value in changes {
            let changed = encode_revocation(&value).unwrap();
            assert_ne!(changed, bytes);
            assert_eq!(decode_revocation(&changed).unwrap(), value);
        }
        assert_eq!(RevocationRequest::new(base.enrollment_operation_id, base.enrollment_operation_id,
            base.staged.clone(), base.administrator_resource.clone(), base.original_descriptor_digest,
            base.current_revoker.clone(), base.source.clone(), base.requested_at), Err(PlanError::OperationAlias));
        assert!(EnrollmentDescriptor::new(Uuid::nil(), scope(), staged(), "m".into(), "r".into(), source(), source(),
            time(0, 0), time(1, 0), time(2, 0), None, [0; 32]).is_err());
    }
}
