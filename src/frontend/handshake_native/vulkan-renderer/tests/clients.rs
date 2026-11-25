//! Client handles without a GPU: identity fixed at issue, isolation, revocation, and what becomes of a claim whose
//! handle was dropped.

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AttachRefusal, AuthorizeRefusal, ByteQuantity, ChargeClass, ChargeRefusal,
    ClientId, ModuleId, OwnerEvent, RebindRefusal, SettleOutcome, SettleRefusal, Settlement,
    Subject, SubmissionRefusal, SubmissionState,
};
use support::{attach, bench, bounded_policy, claim_subject, client_subject, policy, request, KEY};

fn clean() -> Settlement {
    Settlement::clean(SettleOutcome::Canceled)
}

#[test]
fn a_claim_carries_the_identity_the_owner_issued() {
    let bench = bench(policy(100));
    let client = attach(&bench.ledger, 4, 9);
    assert_eq!(
        (client.module(), client.client()),
        (ModuleId(4), ClientId(9))
    );
    let claim = client.admit(&request(10)).expect("fits");
    assert_eq!(
        (claim.info().module, claim.info().client),
        (ModuleId(4), ClientId(9))
    );
    let by_module: Vec<_> = bench
        .ledger
        .usage_by_module()
        .iter()
        .map(|usage| (usage.module.0, usage.charged_bytes))
        .collect();
    assert_eq!(by_module, vec![(4, 10)]);
}

#[test]
fn one_client_has_one_handle_at_a_time() {
    let bench = bench(policy(100));
    let attached = |module, client| {
        bench
            .ledger
            .attach(ModuleId(module), ClientId(client))
            .map(drop)
    };
    assert_eq!(attached(1, 1), Err(AttachRefusal::AlreadyAttached));
    // The same client number in another module is another client.
    let other = attach(&bench.ledger, 2, 1);
    assert_eq!(bench.ledger.usage().attached_clients, 2);
    drop(other);
    assert_eq!(bench.ledger.usage().attached_clients, 1);
    assert_eq!(attached(2, 1), Ok(()));
    bench.ledger.close_admission();
    assert_eq!(attached(3, 1), Err(AttachRefusal::Closed));
    assert_eq!(
        bench.log.count(OwnerEvent::AttachRefused {
            subject: client_subject(1, 1),
            reason: AttachRefusal::AlreadyAttached.number()
        }),
        1
    );
}

/// Two clients of one module, and one of another: none of them does anything with a claim that is not its own.
#[test]
fn a_client_cannot_act_on_another_clients_claim() {
    let bench = bench(policy(100));
    let claim = bench.claim(10);
    let submission = bench.client.begin_submission(&claim).expect("authorized");
    for (module, client) in [(1, 2), (2, 1)] {
        let other = attach(&bench.ledger, module, client);
        // With a claim of its own, so that the ledger knows it.
        let _own = other.admit(&request(1)).expect("fits");
        assert_eq!(other.authorize(&claim), Err(AuthorizeRefusal::Unknown));
        assert!(other.hold(&claim).is_err());
        assert!(other.begin_submission(&claim).is_err());
        assert_eq!(
            other.charge(&claim, ChargeClass::Committed, 1),
            Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Unknown))
        );
        assert_eq!(
            other.discharge(&claim, ChargeClass::Committed, 0),
            Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Unknown))
        );
        assert_eq!(
            other.settle(&claim, clean()),
            Err(SettleRefusal::StaleClaim)
        );
        assert_eq!(
            other.release_retained(&claim),
            Err(SettleRefusal::StaleClaim)
        );
        assert_eq!(
            other.rebind(&claim).expect_err("not its own"),
            RebindRefusal::Unknown
        );
        assert!(!other.owns(&submission));
        // The reports of both refusals name who tried, not whom the claim belongs to.
        assert_eq!(
            bench.log.count(OwnerEvent::SettleRefused {
                subject: Subject {
                    module: ModuleId(module),
                    client: ClientId(client),
                    ..claim_subject(claim.info())
                },
                reason: SettleRefusal::StaleClaim.number()
            }),
            2
        );
    }
    assert!(bench.client.owns(&submission));
    assert_eq!(bench.client.authorize(&claim), Ok(()));
    assert_eq!(bench.ledger.usage().in_flight, 1);
}

#[test]
fn another_ledgers_claim_is_unknown_here() {
    let issuing = bench(policy(100));
    let other = bench(policy(100));
    let foreign = issuing.claim(10);
    // The same numbers on both sides.
    let same = other.claim(10);
    assert_eq!(same.info(), foreign.info());
    assert_eq!(
        other.client.authorize(&foreign),
        Err(AuthorizeRefusal::Unknown)
    );
    assert_eq!(
        other.client.settle(&foreign, clean()),
        Err(SettleRefusal::StaleClaim)
    );
    let submission = issuing
        .client
        .begin_submission(&foreign)
        .expect("authorized");
    assert!(!other.client.owns(&submission));
    assert_eq!(
        other.ledger.submission_no_effect(&submission),
        Err(SubmissionRefusal::Unknown)
    );
    assert_eq!(submission.state(), SubmissionState::Pending);
}

#[test]
fn a_revoked_client_stops_submitting_and_still_settles_once() {
    let bench = bench(policy(100));
    let same_module = attach(&bench.ledger, 1, 2);
    let same_number = attach(&bench.ledger, 2, 1);
    let client = &bench.client;
    let revoked = bench.claim(30);
    let others = [
        (&same_module, same_module.admit(&request(10)).expect("fits")),
        (&same_number, same_number.admit(&request(10)).expect("fits")),
    ];
    client
        .charge(&revoked, ChargeClass::Committed, 30)
        .expect("fits");
    let hold = client.hold(&revoked).expect("authorized");
    let _running = client.begin_submission(&revoked).expect("authorized");
    let revoke = || bench.ledger.revoke_client(ModuleId(1), ClientId(1));
    assert_eq!(revoke(), 1);
    assert_eq!(revoke(), 0);

    assert_eq!(client.authorize(&revoked), Err(AuthorizeRefusal::Revoked));
    assert_eq!(hold.standing(), Err(AuthorizeRefusal::Revoked));
    assert_eq!(
        client.begin_submission(&revoked).expect_err("revoked"),
        AuthorizeRefusal::Revoked
    );
    assert!(client.hold(&revoked).is_err());
    assert!(client.charge(&revoked, ChargeClass::Committed, 0).is_err());
    assert_eq!(bench.ledger.usage().charged_bytes, 50);
    // Revocation is not a settlement: its work is still in flight and its lease out.
    assert_eq!(
        client.settle(&revoked, clean()),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    assert_eq!(bench.ledger.resolve_through(1, 1), 1);
    client
        .discharge(&revoked, ChargeClass::Committed, 30)
        .expect("freed memory is always reported");
    assert_eq!(
        client.settle(&revoked, clean()),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    drop(hold);
    assert!(client.settle(&revoked, clean()).is_ok());
    assert_eq!(
        client.settle(&revoked, clean()),
        Err(SettleRefusal::DoubleRelease)
    );
    for (other, claim) in &others {
        assert_eq!(other.authorize(claim), Ok(()));
    }
    let usage = bench.ledger.usage();
    assert_eq!((usage.charged_bytes, usage.attached_clients), (20, 2));
}

#[test]
fn a_revoked_client_is_admitted_again_only_through_a_new_handle() {
    let bench = bench(policy(100));
    let old_claim = bench.claim(10);
    bench.ledger.revoke_client(ModuleId(1), ClientId(1));
    assert!(!bench.client.is_attached());
    assert_eq!(
        bench.client.admit(&request(1)).expect_err("revoked"),
        AdmissionDenial::ClientDetached
    );
    let denied = OwnerEvent::AdmissionDenied {
        subject: Subject {
            operation_id: Some(request(1).operation_id),
            ..client_subject(1, 1)
        },
        reason: AdmissionDenial::ClientDetached.number(),
    };
    assert_eq!(bench.log.count(denied), 1);

    let again = attach(&bench.ledger, 1, 1);
    let new_claim = again.admit(&request(5)).expect("attached anew");
    // The old handle is not revived by that, and what it holds is not handed over.
    assert_eq!(
        bench.client.admit(&request(1)).expect_err("still revoked"),
        AdmissionDenial::ClientDetached
    );
    assert_eq!(again.authorize(&old_claim), Err(AuthorizeRefusal::Unknown));
    assert_eq!(
        bench.client.authorize(&old_claim),
        Err(AuthorizeRefusal::Revoked)
    );
    assert!(bench.client.settle(&old_claim, clean()).is_ok());
    // Dropping the old handle does not detach the new one.
    let support::Bench { ledger, client, .. } = bench;
    drop(client);
    assert_eq!(again.authorize(&new_claim), Ok(()));
    assert!(again.admit(&request(1)).is_ok());
    assert_eq!(ledger.usage().attached_clients, 1);
}

#[test]
fn a_dropped_client_handle_detaches_its_client() {
    let bench = bench(policy(100));
    let other = attach(&bench.ledger, 2, 5);
    let claim = other.admit(&request(10)).expect("fits");
    drop(other);
    assert_eq!(
        bench.log.count(OwnerEvent::ClientDropped {
            subject: client_subject(2, 5),
            claims: 1
        }),
        1
    );
    let usage = bench.ledger.usage();
    assert_eq!((usage.charged_bytes, usage.revoked_claims), (10, 1));
    // Nobody can settle it any more: dropping it lets the owner close it.
    drop(claim);
    assert_eq!(bench.ledger.usage().charged_bytes, 0);
}

#[test]
fn a_dropped_claim_without_work_is_reclaimed_at_once() {
    let bench = bench(bounded_policy(100, 1, 1));
    let claim = bench.claim(60);
    let subject = claim_subject(claim.info());
    drop(claim);
    let usage = bench.ledger.usage();
    assert_eq!(
        (
            usage.charged_bytes,
            usage.open_claims,
            usage.abandoned_claims
        ),
        (0, 0, 0)
    );
    let events = bench.log.events();
    assert_eq!(
        events[events.len() - 2..],
        [
            OwnerEvent::ClaimAbandoned {
                subject,
                in_flight: 0,
                charged_bytes: 60
            },
            OwnerEvent::ClaimReclaimed {
                subject,
                released_bytes: 60
            },
        ]
    );
    // Its slot is free as well.
    assert!(bench.client.admit(&request(100)).is_ok());
}

#[test]
fn a_dropped_claim_is_reclaimed_after_its_last_work_ended_and_not_earlier() {
    let bench = bench(policy(100));
    let claim = bench.claim(40);
    let subject = claim_subject(claim.info());
    let first = bench.client.begin_submission(&claim).expect("authorized");
    let last = bench.client.begin_submission(&claim).expect("authorized");
    drop((first, last, claim));
    let reclaimed = OwnerEvent::ClaimReclaimed {
        subject,
        released_bytes: 40,
    };
    assert_eq!(
        bench.log.count(OwnerEvent::ClaimAbandoned {
            subject,
            in_flight: 2,
            charged_bytes: 40
        }),
        1
    );
    assert_eq!(bench.ledger.resolve_through(1, 0), 0);
    assert_eq!(bench.ledger.resolve_through(1, 1), 1);
    assert_eq!(bench.ledger.usage().charged_bytes, 40, "one still runs");
    assert_eq!(bench.log.count(reclaimed), 0);
    assert_eq!(bench.ledger.resolve_through(1, 2), 1);
    assert_eq!(bench.ledger.usage().charged_bytes, 0);
    assert_eq!(bench.log.count(reclaimed), 1);
    // Once, whatever is observed later.
    bench.ledger.resolve_through(1, u64::MAX);
    bench.ledger.report_abandoned();
    assert_eq!(bench.log.count(reclaimed), 1);
    let counters = bench.ledger.counters();
    assert_eq!(
        (counters.abandoned, counters.reclaimed, counters.leaked),
        (1, 1, 0)
    );
}

#[test]
fn a_dropped_claim_waits_for_its_leases() {
    let bench = bench(policy(100));
    let claim = bench.claim(40);
    let first = bench.client.hold(&claim).expect("authorized");
    let last = bench.client.hold(&claim).expect("authorized");
    drop(claim);
    assert_eq!(bench.ledger.usage().charged_bytes, 40);
    // Who holds the device is still told whether it may use it.
    assert_eq!(last.standing(), Ok(()));
    drop(first);
    assert_eq!(bench.ledger.usage().charged_bytes, 40);
    drop(last);
    let usage = bench.ledger.usage();
    assert_eq!((usage.charged_bytes, usage.leases), (0, 0));
    assert_eq!(bench.ledger.counters().leaked, 0);
}

#[test]
fn a_dropped_claim_with_reported_memory_leaks_until_its_device_is_destroyed() {
    let bench = bench(policy(100));
    let claim = bench.claim(40);
    let subject = claim_subject(claim.info());
    bench
        .client
        .charge(&claim, ChargeClass::Committed, 1)
        .expect("fits");
    drop(bench.client.begin_submission(&claim).expect("authorized"));
    drop(claim);
    let leaked = OwnerEvent::ClaimLeaked {
        subject,
        leaked_bytes: 40,
    };
    assert_eq!(bench.log.count(leaked), 0, "its work may still run");
    bench.ledger.resolve_through(1, 1);
    assert_eq!(bench.log.count(leaked), 1);
    bench.ledger.resolve_through(1, u64::MAX);
    bench.ledger.report_abandoned();
    assert_eq!(bench.log.count(leaked), 1, "reported once");
    let usage = bench.ledger.usage();
    assert_eq!(
        (
            usage.charged_bytes,
            usage.leaked_claims,
            usage.abandoned_claims
        ),
        (40, 1, 1)
    );
    assert_eq!(bench.ledger.generation_retired(1), 0);
    assert_eq!(bench.ledger.usage().charged_bytes, 40, "a healthy device");
    assert!(bench.ledger.mark_device_lost(1));
    assert_eq!(bench.ledger.usage().charged_bytes, 40, "a loss alone");
    bench.ledger.generation_retired(1);
    assert_eq!(bench.ledger.usage().charged_bytes, 0);
    assert_eq!(
        bench.log.count(OwnerEvent::ClaimReclaimed {
            subject,
            released_bytes: 40
        }),
        1
    );
}

#[test]
fn a_dropped_claim_that_retains_memory_of_unknown_size_is_not_reclaimed() {
    let bench = bench(policy(100));
    let claim = bench.claim(0);
    let settlement = Settlement {
        retained: ByteQuantity::Unknown,
        ..clean()
    };
    bench.client.settle(&claim, settlement).expect("settles");
    drop(claim);
    let usage = bench.ledger.usage();
    assert_eq!((usage.retained_claims, usage.leaked_claims), (1, 1));
    bench.ledger.bind_device(KEY);
    bench.ledger.generation_retired(1);
    assert_eq!(bench.ledger.usage().retained_claims, 0);
}

#[test]
fn a_settled_or_superseded_claim_is_not_abandoned_by_being_dropped() {
    let bench = bench(policy(100));
    let settled = bench.claim(10);
    bench.client.settle(&settled, clean()).expect("settles");
    drop(settled);
    let old = bench.claim(20);
    bench.ledger.mark_device_lost(1);
    bench.ledger.bind_device(KEY);
    let successor = bench.client.rebind(&old).expect("moves");
    drop(old);
    assert_eq!(bench.ledger.counters().abandoned, 0);
    assert_eq!(bench.ledger.usage().charged_bytes, 20);
    assert_eq!(bench.client.authorize(&successor), Ok(()));
}
