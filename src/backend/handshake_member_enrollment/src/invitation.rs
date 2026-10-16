//! Pure invitation comparisons and transition data. None of these results grants authority.
//! Hosts own authentication, purpose-bound hashing, atomic persistence and settlement.

use crate::command_plan::{
    encode_enrollment, EnrollmentDescriptor, RevocationRequest, StagedIdentity, Timestamp,
};
use std::fmt;
use uuid::Uuid;

/// A missing canonical observation remains unknown, never evidence of no commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorComparison {
    Exact,
    Different,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeadlineError {
    Overflow,
    NoValidity,
    InvalidExpiry,
}

/// Recompute every captured bound. The sixty seconds is a maximum, never a floor.
pub fn maximum_deadline(descriptor: &EnrollmentDescriptor) -> Result<Timestamp, DeadlineError> {
    let issued = descriptor.issued_at();
    let mut limit = issued
        .checked_add_seconds(60)
        .map_err(|_| DeadlineError::Overflow)?;
    limit = limit.min(descriptor.session_expires_at());
    for source in [descriptor.enroll_source(), descriptor.assignment_source()] {
        if let Some(expiry) = source.grant_expires_at() {
            limit = limit.min(expiry);
        }
    }
    if let Some(expiry) = descriptor.policy_expires_at() {
        limit = limit.min(expiry);
    }
    if limit <= issued {
        return Err(DeadlineError::NoValidity);
    }
    Ok(limit)
}

/// Shorter expiries are valid; a descriptor cannot extend any captured bound.
pub fn validate_expiry(descriptor: &EnrollmentDescriptor) -> Result<(), DeadlineError> {
    let limit = maximum_deadline(descriptor)?;
    if descriptor.expires_at() <= descriptor.issued_at() || descriptor.expires_at() > limit {
        return Err(DeadlineError::InvalidExpiry);
    }
    Ok(())
}

/// Equality uses B's single encoding, with no second serialization or normalization.
pub fn compare_descriptor(
    expected: &EnrollmentDescriptor,
    observed_bytes: Option<&[u8]>,
) -> Result<DescriptorComparison, crate::command_plan::PlanError> {
    let Some(observed) = observed_bytes else {
        return Ok(DescriptorComparison::Unknown);
    };
    let expected_bytes = encode_enrollment(expected)?;
    Ok(if expected_bytes == observed {
        DescriptorComparison::Exact
    } else {
        DescriptorComparison::Different
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvitationStatus {
    Active,
    Consumed,
    Revoked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityStatus {
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceStatus {
    Disabled,
    Active,
}

/// Raw supplied state, not a canonical-read or authorization witness.
pub struct InvitationSnapshot<'a> {
    pub descriptor: &'a EnrollmentDescriptor,
    pub observed_descriptor_bytes: Option<&'a [u8]>,
    pub status: InvitationStatus,
    pub account_status: IdentityStatus,
    pub principal_status: IdentityStatus,
    pub space_status: SpaceStatus,
}

/// Supplied comparison data only. Digest equality is not cryptographic verification.
/// Recipient/password/current-authorizer checks remain separate host responsibilities.
pub struct RedemptionAttempt<'a> {
    pub enrollment_operation_id: Uuid,
    pub staged: &'a StagedIdentity,
    pub purpose: &'a str,
    pub token_digest: &'a [u8; 32],
    pub descriptor_digest: &'a [u8; 32],
}

/// All invalid/expired/replayed/disabled cases have one public failure classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvitationDenied;

impl fmt::Display for InvitationDenied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invitation denied")
    }
}

impl std::error::Error for InvitationDenied {}

/// Proposed state data, not proof that these effects occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedemptionTransition {
    pub invitation_status: InvitationStatus,
    pub account_status: IdentityStatus,
    pub principal_status: IdentityStatus,
    pub space_status: SpaceStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RevocationTransition {
    pub invitation_status: InvitationStatus,
}

fn require_active_staged(
    snapshot: &InvitationSnapshot<'_>,
    now: Timestamp,
) -> Result<(), InvitationDenied> {
    let descriptor = snapshot.descriptor;
    validate_expiry(descriptor).map_err(|_| InvitationDenied)?;
    if now < descriptor.issued_at()
        || now >= descriptor.expires_at()
        || snapshot.status != InvitationStatus::Active
        || snapshot.account_status != IdentityStatus::Disabled
        || snapshot.principal_status != IdentityStatus::Disabled
        || snapshot.space_status != SpaceStatus::Disabled
        || compare_descriptor(descriptor, snapshot.observed_descriptor_bytes)
            .map_err(|_| InvitationDenied)?
            != DescriptorComparison::Exact
    {
        return Err(InvitationDenied);
    }
    Ok(())
}

pub fn evaluate_redemption(
    snapshot: &InvitationSnapshot<'_>,
    attempt: &RedemptionAttempt<'_>,
    expected_descriptor_digest: &[u8; 32],
    now: Timestamp,
) -> Result<RedemptionTransition, InvitationDenied> {
    require_active_staged(snapshot, now)?;
    let descriptor = snapshot.descriptor;
    if attempt.enrollment_operation_id != descriptor.operation_id()
        || attempt.staged != descriptor.staged()
        || attempt.purpose != "identity_enrollment"
        || attempt.token_digest != descriptor.token_digest()
        || attempt.descriptor_digest != expected_descriptor_digest
    {
        return Err(InvitationDenied);
    }
    Ok(RedemptionTransition {
        invitation_status: InvitationStatus::Consumed,
        account_status: IdentityStatus::Enabled,
        principal_status: IdentityStatus::Enabled,
        space_status: SpaceStatus::Active,
    })
}

/// A distinct revocation request links to the immutable original enrollment.
/// Its current revoker need not be the original enroller; host authorization is mandatory.
pub fn evaluate_revocation(
    snapshot: &InvitationSnapshot<'_>,
    request: &RevocationRequest,
    expected_descriptor_digest: &[u8; 32],
    now: Timestamp,
) -> Result<RevocationTransition, InvitationDenied> {
    require_active_staged(snapshot, now)?;
    let descriptor = snapshot.descriptor;
    if request.request_id().is_nil()
        || request.request_id() == descriptor.operation_id()
        || request.enrollment_operation_id() != descriptor.operation_id()
        || request.staged() != descriptor.staged()
        || request.administrator_resource() != descriptor.administrator_resource()
        || request.original_descriptor_digest() != expected_descriptor_digest
    {
        return Err(InvitationDenied);
    }
    Ok(RevocationTransition {
        invitation_status: InvitationStatus::Revoked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_plan::{IdentityScope, SourceRef};

    fn time(seconds: i64, nanos: u32) -> Timestamp {
        Timestamp::new(seconds, nanos).unwrap()
    }

    fn scope(last: &str) -> IdentityScope {
        IdentityScope::new(
            "owner".into(),
            "principal".into(),
            "session".into(),
            "space".into(),
            last.into(),
        )
        .unwrap()
    }

    fn staged() -> StagedIdentity {
        StagedIdentity::new("member".into(), "human".into(), "private".into()).unwrap()
    }

    fn source(id: &str, expiry: Option<Timestamp>) -> SourceRef {
        SourceRef::new(
            "admin-resource".into(),
            id.into(),
            2,
            vec![3, 4],
            vec![0, 1],
            expiry,
        )
        .unwrap()
    }

    fn descriptor(
        issued: Timestamp,
        expiry: Timestamp,
        session: Timestamp,
        enroll: Option<Timestamp>,
        assignment: Option<Timestamp>,
        policy: Option<Timestamp>,
    ) -> EnrollmentDescriptor {
        EnrollmentDescriptor::new(
            Uuid::from_bytes([1; 16]),
            scope("workspace opaque/α"),
            staged(),
            "Member".into(),
            "admin-resource".into(),
            source("enroll-grant", enroll),
            source("assignment-grant", assignment),
            issued,
            expiry,
            session,
            policy,
            [7; 32],
        )
        .unwrap()
    }

    fn normal() -> EnrollmentDescriptor {
        descriptor(time(100, 123), time(150, 123), time(200, 0), None, None, None)
    }

    fn snapshot<'a>(
        descriptor: &'a EnrollmentDescriptor,
        bytes: Option<&'a [u8]>,
    ) -> InvitationSnapshot<'a> {
        InvitationSnapshot {
            descriptor,
            observed_descriptor_bytes: bytes,
            status: InvitationStatus::Active,
            account_status: IdentityStatus::Disabled,
            principal_status: IdentityStatus::Disabled,
            space_status: SpaceStatus::Disabled,
        }
    }

    fn attempt(descriptor: &EnrollmentDescriptor) -> RedemptionAttempt<'_> {
        RedemptionAttempt {
            enrollment_operation_id: descriptor.operation_id(),
            staged: descriptor.staged(),
            purpose: "identity_enrollment",
            token_digest: descriptor.token_digest(),
            descriptor_digest: &[8; 32],
        }
    }

    fn revoke(descriptor: &EnrollmentDescriptor) -> RevocationRequest {
        RevocationRequest::new(
            Uuid::from_bytes([2; 16]),
            descriptor.operation_id(),
            descriptor.staged().clone(),
            descriptor.administrator_resource().into(),
            [8; 32],
            scope("different current revoker workspace"),
            source("revoke-grant", None),
            time(120, 0),
        )
        .unwrap()
    }

    #[test]
    fn each_bound_can_be_the_earliest_deadline() {
        let issued = time(100, 123);
        for (session, enroll, assignment, policy, expected) in [
            (time(200, 0), None, None, None, time(160, 123)),
            (time(140, 1), None, None, None, time(140, 1)),
            (time(200, 0), Some(time(130, 2)), None, None, time(130, 2)),
            (time(200, 0), None, Some(time(120, 3)), None, time(120, 3)),
            (time(200, 0), None, None, Some(time(110, 4)), time(110, 4)),
        ] {
            let d = descriptor(issued, expected, session, enroll, assignment, policy);
            assert_eq!(maximum_deadline(&d), Ok(expected));
            assert_eq!(validate_expiry(&d), Ok(()));
        }
    }

    #[test]
    fn all_bounds_are_recomputed_and_shorter_expiry_is_valid() {
        let d = descriptor(
            time(100, 0),
            time(101, 0),
            time(150, 0),
            Some(time(140, 0)),
            Some(time(130, 0)),
            Some(time(120, 0)),
        );
        assert_eq!(maximum_deadline(&d), Ok(time(120, 0)));
        assert_eq!(validate_expiry(&d), Ok(()));
        let extended = descriptor(
            time(100, 0),
            time(130, 1),
            time(150, 0),
            Some(time(140, 0)),
            Some(time(130, 0)),
            None,
        );
        assert_eq!(validate_expiry(&extended), Err(DeadlineError::InvalidExpiry));
    }

    #[test]
    fn cap_equality_and_nanoseconds_are_lossless() {
        let d = descriptor(time(100, 999_999_999), time(160, 999_999_999), time(200, 0), None, None, None);
        assert_eq!(validate_expiry(&d), Ok(()));
        let bytes = encode_enrollment(&d).unwrap();
        let s = snapshot(&d, Some(&bytes));
        assert!(evaluate_redemption(&s, &attempt(&d), &[8; 32], time(160, 999_999_998)).is_ok());
        assert_eq!(evaluate_redemption(&s, &attempt(&d), &[8; 32], time(160, 999_999_999)), Err(InvitationDenied));
    }

    #[test]
    fn overflow_and_nonpositive_validity_fail_closed() {
        let overflow = descriptor(time(i64::MAX - 1, 0), time(i64::MAX, 0), time(i64::MAX, 0), None, None, None);
        assert_eq!(maximum_deadline(&overflow), Err(DeadlineError::Overflow));
        for bound in [time(99, 999_999_999), time(100, 0)] {
            let d = descriptor(time(100, 0), time(101, 0), bound, None, None, None);
            assert_eq!(maximum_deadline(&d), Err(DeadlineError::NoValidity));
        }
        let zero = descriptor(time(100, 0), time(100, 0), time(200, 0), None, None, None);
        assert_eq!(validate_expiry(&zero), Err(DeadlineError::InvalidExpiry));
    }

    #[test]
    fn successful_redemption_is_only_closed_transition_data() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        assert_eq!(
            evaluate_redemption(&snapshot(&d, Some(&bytes)), &attempt(&d), &[8; 32], time(120, 0)),
            Ok(RedemptionTransition {
                invitation_status: InvitationStatus::Consumed,
                account_status: IdentityStatus::Enabled,
                principal_status: IdentityStatus::Enabled,
                space_status: SpaceStatus::Active,
            })
        );
    }

    #[test]
    fn wrong_purpose_token_digest_descriptor_digest_and_operation_deny_identically() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        let s = snapshot(&d, Some(&bytes));
        for field in 0..4 {
            let mut a = attempt(&d);
            match field {
                0 => a.purpose = "session_exchange",
                1 => a.token_digest = &[0; 32],
                2 => a.descriptor_digest = &[0; 32],
                _ => a.enrollment_operation_id = Uuid::from_bytes([3; 16]),
            }
            assert_eq!(evaluate_redemption(&s, &a, &[8; 32], time(120, 0)), Err(InvitationDenied));
        }
        assert_eq!(InvitationDenied.to_string(), "invitation denied");
    }

    #[test]
    fn each_staged_identity_mismatch_denies() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        for values in [
            ("other", "human", "private"),
            ("member", "other", "private"),
            ("member", "human", "other"),
        ] {
            let other = StagedIdentity::new(values.0.into(), values.1.into(), values.2.into()).unwrap();
            let mut a = attempt(&d);
            a.staged = &other;
            assert_eq!(evaluate_redemption(&snapshot(&d, Some(&bytes)), &a, &[8; 32], time(120, 0)), Err(InvitationDenied));
        }
    }

    #[test]
    fn each_original_identity_axis_is_bound_by_the_one_codec() {
        let d = normal();
        for axis in 0..5 {
            let mut values = ["owner", "principal", "session", "space", "workspace opaque/α"];
            values[axis] = "other";
            let other = EnrollmentDescriptor::new(
                d.operation_id(),
                IdentityScope::new(values[0].into(), values[1].into(), values[2].into(), values[3].into(), values[4].into()).unwrap(),
                d.staged().clone(),
                d.account_name().into(),
                d.administrator_resource().into(),
                d.enroll_source().clone(),
                d.assignment_source().clone(),
                d.issued_at(), d.expires_at(), d.session_expires_at(),
                d.policy_expires_at(), *d.token_digest(),
            ).unwrap();
            let bytes = encode_enrollment(&other).unwrap();
            assert_eq!(compare_descriptor(&d, Some(&bytes)).unwrap(), DescriptorComparison::Different);
            assert_eq!(evaluate_redemption(&snapshot(&d, Some(&bytes)), &attempt(&d), &[8; 32], time(120, 0)), Err(InvitationDenied));
        }
    }

    #[test]
    fn missing_changed_or_noncanonical_descriptor_is_never_equivalence() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        assert_eq!(compare_descriptor(&d, Some(&bytes)).unwrap(), DescriptorComparison::Exact);
        assert_eq!(compare_descriptor(&d, None).unwrap(), DescriptorComparison::Unknown);
        let mut extra = bytes.clone();
        extra.push(0);
        assert_eq!(compare_descriptor(&d, Some(&extra)).unwrap(), DescriptorComparison::Different);
        assert_eq!(evaluate_redemption(&snapshot(&d, None), &attempt(&d), &[8; 32], time(120, 0)), Err(InvitationDenied));
    }

    #[test]
    fn terminal_and_already_enabled_states_cannot_redeem_or_revoke() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        for case in 0..5 {
            let mut s = snapshot(&d, Some(&bytes));
            match case {
                0 => s.status = InvitationStatus::Consumed,
                1 => s.status = InvitationStatus::Revoked,
                2 => s.account_status = IdentityStatus::Enabled,
                3 => s.principal_status = IdentityStatus::Enabled,
                _ => s.space_status = SpaceStatus::Active,
            }
            assert_eq!(evaluate_redemption(&s, &attempt(&d), &[8; 32], time(120, 0)), Err(InvitationDenied));
            assert_eq!(evaluate_revocation(&s, &revoke(&d), &[8; 32], time(120, 0)), Err(InvitationDenied));
        }
    }

    #[test]
    fn future_or_expired_invitation_never_transitions() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        for now in [time(100, 122), d.expires_at(), time(151, 0)] {
            let s = snapshot(&d, Some(&bytes));
            assert_eq!(evaluate_redemption(&s, &attempt(&d), &[8; 32], now), Err(InvitationDenied));
            assert_eq!(evaluate_revocation(&s, &revoke(&d), &[8; 32], now), Err(InvitationDenied));
        }
    }

    #[test]
    fn revocation_links_original_and_allows_distinct_current_revoker_data() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        assert_eq!(
            evaluate_revocation(&snapshot(&d, Some(&bytes)), &revoke(&d), &[8; 32], time(120, 0)),
            Ok(RevocationTransition { invitation_status: InvitationStatus::Revoked })
        );
        let request = RevocationRequest::new(
            Uuid::from_bytes([2; 16]), Uuid::from_bytes([4; 16]),
            d.staged().clone(), d.administrator_resource().into(), [8; 32],
            scope("current"), source("revoke", None), time(120, 0),
        ).unwrap();
        assert_eq!(evaluate_revocation(&snapshot(&d, Some(&bytes)), &request, &[8; 32], time(120, 0)), Err(InvitationDenied));
    }

    #[test]
    fn revocation_rejects_each_target_admin_resource_and_original_digest_mismatch() {
        let d = normal();
        let bytes = encode_enrollment(&d).unwrap();
        for case in 0..5 {
            let target = match case {
                0 => StagedIdentity::new("other".into(), "human".into(), "private".into()).unwrap(),
                1 => StagedIdentity::new("member".into(), "other".into(), "private".into()).unwrap(),
                2 => StagedIdentity::new("member".into(), "human".into(), "other".into()).unwrap(),
                _ => d.staged().clone(),
            };
            let request = RevocationRequest::new(
                Uuid::from_bytes([2; 16]), d.operation_id(), target,
                if case == 3 { "other" } else { d.administrator_resource() }.into(),
                if case == 4 { [0; 32] } else { [8; 32] },
                scope("current"), source("revoke", None), time(120, 0),
            ).unwrap();
            assert_eq!(evaluate_revocation(&snapshot(&d, Some(&bytes)), &request, &[8; 32], time(120, 0)), Err(InvitationDenied));
        }
    }
}
