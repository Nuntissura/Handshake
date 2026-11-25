//! The claim ledger without a GPU: admission, accounting, settlement and revocation.

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AuthorizeRefusal, ByteQuantity, ChargeClass, ChargeRefusal, ClaimDisposition,
    ClaimLedger, ClientId, ModuleId, PolicyRevision, QuotaPolicy, SettleOutcome, SettleRefusal,
    Settlement, SubmissionObservation, SubmissionRefusal, SubmissionState,
};
use std::sync::Arc;
use support::{bounded_policy, claim, ledger, policy, request, KEY};

fn retaining(retained: ByteQuantity) -> Settlement {
    Settlement {
        retained,
        ..Settlement::clean(SettleOutcome::Completed)
    }
}

#[test]
fn without_a_quota_every_admission_is_denied() {
    let ledger = ledger(QuotaPolicy::Unavailable);
    for bytes in [0, 1, u64::MAX] {
        assert_eq!(
            ledger.admit(&request(1, 1, bytes)).expect_err("no quota"),
            AdmissionDenial::HostQuotaUnavailable
        );
    }
    let usage = ledger.usage();
    assert_eq!((usage.charged_bytes, usage.open_claims), (0, 0));
    assert_eq!(
        (ledger.counters().admitted, ledger.counters().denied),
        (0, 3)
    );
}

#[test]
fn admission_needs_a_healthy_bound_device_and_an_open_ledger() {
    let ledger = ClaimLedger::new(policy(100));
    let ask = request(1, 1, 10);
    assert_eq!(
        ledger.admit(&ask).expect_err("no device"),
        AdmissionDenial::DeviceUnavailable
    );
    let generation = ledger.bind_device(KEY);
    assert!(ledger.mark_device_lost(generation));
    assert_eq!(
        ledger.admit(&ask).expect_err("lost"),
        AdmissionDenial::DeviceLost { generation }
    );
    ledger.bind_device(KEY);
    ledger.close_admission();
    assert_eq!(
        ledger.admit(&ask).expect_err("closed"),
        AdmissionDenial::AdmissionClosed
    );
    assert_eq!(ledger.usage().charged_bytes, 0);
}

#[test]
fn a_reservation_must_fit_what_the_quota_has_left() {
    let ledger = ledger(policy(100));
    let first = claim(&ledger, 60);
    assert_eq!(first.info().policy_revision, PolicyRevision(7));
    assert_eq!(
        ledger.admit(&request(1, 2, 41)).expect_err("too big"),
        AdmissionDenial::BudgetExceeded {
            requested_bytes: 41,
            remaining_bytes: 40
        }
    );
    let _second = claim(&ledger, 40);
    assert_eq!(
        ledger.admit(&request(1, 3, 1)).expect_err("full"),
        AdmissionDenial::BudgetExceeded {
            requested_bytes: 1,
            remaining_bytes: 0
        }
    );
    assert_eq!(ledger.usage().charged_bytes, 100);
}

#[test]
fn the_largest_byte_numbers_do_not_wrap() {
    let ledger = ledger(policy(u64::MAX));
    let all = claim(&ledger, u64::MAX);
    assert_eq!(
        ledger.admit(&request(1, 2, 1)).expect_err("full"),
        AdmissionDenial::BudgetExceeded {
            requested_bytes: 1,
            remaining_bytes: 0
        }
    );
    assert_eq!(
        ledger.charge(&all, ChargeClass::Committed, u64::MAX),
        Ok(())
    );
    assert_eq!(
        ledger.charge(&all, ChargeClass::Staging, u64::MAX),
        Err(ChargeRefusal::ReservationExceeded {
            reserved_bytes: u64::MAX,
            charged_bytes: u64::MAX,
            requested_bytes: u64::MAX,
        })
    );
    assert_eq!(ledger.usage().charged_bytes, u64::MAX);
    assert_eq!(
        ledger.discharge(&all, ChargeClass::Committed, u64::MAX),
        Ok(())
    );
    let receipt = ledger
        .settle(&all, Settlement::clean(SettleOutcome::Completed))
        .expect("settles");
    assert_eq!(receipt.released_bytes, u64::MAX);
    assert_eq!(ledger.usage().charged_bytes, 0);
}

#[test]
fn the_number_of_claims_is_bounded_and_retained_claims_count() {
    let ledger = ledger(bounded_policy(100, 2, 4));
    let first = claim(&ledger, 10);
    let second = claim(&ledger, 0);
    let denied = AdmissionDenial::ClaimLimit { max_claims: 2 };
    assert_eq!(
        ledger.admit(&request(1, 3, 0)).expect_err("two held"),
        denied
    );
    ledger
        .settle(&first, retaining(ByteQuantity::Measured(5)))
        .expect("retains");
    assert_eq!(
        ledger.admit(&request(1, 3, 0)).expect_err("one retains"),
        denied
    );
    ledger
        .settle(&second, Settlement::clean(SettleOutcome::NoEffect))
        .expect("closes");
    assert!(ledger.admit(&request(1, 3, 0)).is_ok());
}

#[test]
fn a_policy_change_leaves_existing_claims_alone() {
    let ledger = ledger(bounded_policy(100, 8, 1));
    let old = claim(&ledger, 80);
    ledger.set_policy(QuotaPolicy::Configured {
        revision: PolicyRevision(8),
        limit_bytes: 50,
        max_claims: 8,
        max_in_flight_per_claim: 9,
    });
    assert_eq!(
        ledger
            .admit(&request(1, 2, 1))
            .expect_err("over the new limit"),
        AdmissionDenial::BudgetExceeded {
            requested_bytes: 1,
            remaining_bytes: 0
        }
    );
    ledger.set_policy(QuotaPolicy::Unavailable);
    assert_eq!(old.info().policy_revision, PolicyRevision(7));
    assert_eq!(ledger.authorize(&old), Ok(()));
    let _running = ledger.begin_submission(&old).expect("first");
    assert_eq!(
        ledger
            .begin_submission(&old)
            .expect_err("bound of its own policy"),
        AuthorizeRefusal::InFlightLimit { max_in_flight: 1 }
    );
    assert_eq!(ledger.usage().charged_bytes, 80);
}

#[test]
fn concurrent_admissions_never_spend_the_same_quota_twice() {
    let ledger = Arc::new(ledger(bounded_policy(100, 1000, 1)));
    let workers: Vec<_> = (0..8)
        .map(|worker| {
            let ledger = Arc::clone(&ledger);
            std::thread::spawn(move || {
                (0..20)
                    .filter_map(|_| ledger.admit(&request(1, worker, 10)).ok())
                    .count()
            })
        })
        .collect();
    let admitted: usize = workers
        .into_iter()
        .map(|worker| worker.join().expect("no panic"))
        .sum();
    assert_eq!(admitted, 10);
    assert_eq!(ledger.usage().charged_bytes, 100);
}

#[test]
fn charges_stay_inside_their_reservation_and_class() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 100);
    assert_eq!(ledger.charge(&held, ChargeClass::Committed, 70), Ok(()));
    assert_eq!(ledger.charge(&held, ChargeClass::Staging, 30), Ok(()));
    let full = ChargeRefusal::ReservationExceeded {
        reserved_bytes: 100,
        charged_bytes: 100,
        requested_bytes: 1,
    };
    assert_eq!(ledger.charge(&held, ChargeClass::Staging, 1), Err(full));
    assert_eq!(
        ledger.discharge(&held, ChargeClass::Staging, 31),
        Err(ChargeRefusal::NotCharged {
            charged_bytes: 30,
            requested_bytes: 31
        })
    );
    assert_eq!(ledger.discharge(&held, ChargeClass::Staging, 30), Ok(()));
    let usage = ledger.usage();
    assert_eq!(
        (
            usage.committed_bytes,
            usage.staging_bytes,
            usage.charged_bytes
        ),
        (70, 0, 100)
    );
}

#[test]
fn a_claim_settles_once_and_is_released_once() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 40);
    let receipt = ledger
        .settle(&held, Settlement::clean(SettleOutcome::Completed))
        .expect("settles");
    assert_eq!(
        (receipt.released_bytes, receipt.disposition),
        (40, ClaimDisposition::Closed)
    );
    assert_eq!(receipt.claim, held.info());
    for _ in 0..2 {
        assert_eq!(
            ledger.settle(&held, Settlement::clean(SettleOutcome::Completed)),
            Err(SettleRefusal::DoubleRelease)
        );
        assert_eq!(
            ledger.release_retained(&held),
            Err(SettleRefusal::DoubleRelease)
        );
    }
    assert_eq!(ledger.authorize(&held), Err(AuthorizeRefusal::Settled));
    assert_eq!(ledger.usage().charged_bytes, 0);
    assert_eq!(
        (ledger.counters().settled, ledger.counters().double_release),
        (1, 4)
    );
}

#[test]
fn work_in_flight_keeps_a_claim_unsettled_whatever_the_client_reports() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 40);
    let running = ledger.begin_submission(&held).expect("authorized");
    let clean = Settlement::clean(SettleOutcome::Canceled);
    assert_eq!(
        ledger.settle(&held, clean),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    let more = Settlement {
        in_flight: 3,
        ..clean
    };
    assert_eq!(
        ledger.settle(&held, more),
        Err(SettleRefusal::InFlight { in_flight: 3 })
    );
    // A dropped submission stays counted.
    drop(running);
    assert_eq!(
        ledger.settle(&held, clean),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    assert_eq!(ledger.usage().charged_bytes, 40);
    assert_eq!(ledger.resolve_through(1, 1), 1);
    let reported = Settlement {
        in_flight: 1,
        ..clean
    };
    assert_eq!(
        ledger.settle(&held, reported),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    assert!(ledger.settle(&held, clean).is_ok());
}

#[test]
fn retained_memory_stays_charged_until_its_own_release() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 100);
    assert_eq!(
        ledger.release_retained(&held),
        Err(SettleRefusal::NotSettled)
    );
    let receipt = ledger
        .settle(&held, retaining(ByteQuantity::Measured(30)))
        .expect("settles");
    assert_eq!(
        (receipt.released_bytes, receipt.disposition),
        (70, ClaimDisposition::Retained { charged_bytes: 30 })
    );
    assert_eq!(ledger.authorize(&held), Err(AuthorizeRefusal::Settled));
    assert_eq!(
        ledger.settle(&held, retaining(ByteQuantity::Measured(0))),
        Err(SettleRefusal::AlreadySettled)
    );
    let usage = ledger.usage();
    assert_eq!(
        (
            usage.charged_bytes,
            usage.retained_bytes,
            usage.retained_claims
        ),
        (30, 30, 1)
    );
    assert_eq!(
        ledger
            .release_retained(&held)
            .expect("releases")
            .released_bytes,
        30
    );
    assert_eq!(
        ledger.release_retained(&held),
        Err(SettleRefusal::DoubleRelease)
    );
    assert_eq!(ledger.usage().charged_bytes, 0);
}

#[test]
fn an_unmeasured_remainder_never_becomes_zero() {
    for unmeasured in [
        ByteQuantity::Estimated(0),
        ByteQuantity::Estimated(5),
        ByteQuantity::Unknown,
    ] {
        let ledger = ledger(policy(100));
        let held = claim(&ledger, 60);
        let receipt = ledger
            .settle(&held, retaining(unmeasured))
            .expect("settles");
        assert_eq!(
            (receipt.released_bytes, receipt.disposition),
            (0, ClaimDisposition::Retained { charged_bytes: 60 })
        );
        assert_eq!(ledger.usage().charged_bytes, 60);
    }
}

#[test]
fn a_settlement_that_contradicts_the_accounting_is_refused() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 50);
    assert_eq!(
        ledger.settle(&held, retaining(ByteQuantity::Measured(51))),
        Err(SettleRefusal::RetainedExceedsReservation {
            reserved_bytes: 50,
            retained_bytes: 51
        })
    );
    ledger
        .charge(&held, ChargeClass::Committed, 20)
        .expect("fits");
    ledger.charge(&held, ChargeClass::Staging, 5).expect("fits");
    assert_eq!(
        ledger.settle(&held, retaining(ByteQuantity::Measured(0))),
        Err(SettleRefusal::RetainedBelowCharged {
            charged_bytes: 25,
            retained_bytes: 0
        })
    );
    assert_eq!(ledger.usage().charged_bytes, 50);
    ledger
        .discharge(&held, ChargeClass::Staging, 5)
        .expect("charged");
    let receipt = ledger
        .settle(&held, retaining(ByteQuantity::Measured(20)))
        .expect("agrees");
    assert_eq!(receipt.released_bytes, 30);
}

#[test]
fn a_held_device_keeps_a_claim_unsettled() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 10);
    let first = ledger.hold(&held).expect("authorized");
    let second = ledger.hold(&held).expect("authorized");
    assert_eq!(ledger.usage().leases, 2);
    let clean = Settlement::clean(SettleOutcome::Completed);
    assert_eq!(
        ledger.settle(&held, clean),
        Err(SettleRefusal::LeaseOutstanding { leases: 2 })
    );
    drop(first);
    assert_eq!(
        ledger.settle(&held, clean),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    drop(second);
    assert!(ledger.settle(&held, clean).is_ok());
    assert!(ledger.hold(&held).is_err());
}

#[test]
fn retained_memory_keeps_its_lease_until_it_is_released() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 10);
    let hold = ledger.hold(&held).expect("authorized");
    ledger
        .settle(&held, retaining(ByteQuantity::Measured(4)))
        .expect("retains under its lease");
    assert_eq!(ledger.usage().leases, 1);
    assert_eq!(
        ledger.release_retained(&held),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    assert_eq!(ledger.usage().charged_bytes, 4);
    drop(hold);
    assert_eq!(
        ledger
            .release_retained(&held)
            .expect("releases")
            .released_bytes,
        4
    );
    assert_eq!(
        (ledger.usage().charged_bytes, ledger.usage().leases),
        (0, 0)
    );
}

#[test]
fn a_revoked_client_stops_submitting_and_still_settles_once() {
    let ledger = ledger(policy(100));
    let revoked = ledger.admit(&request(1, 1, 30)).expect("fits");
    let other_client = ledger.admit(&request(1, 2, 10)).expect("fits");
    let other_module = ledger.admit(&request(2, 1, 10)).expect("fits");
    ledger
        .charge(&revoked, ChargeClass::Committed, 30)
        .expect("fits");
    let _running = ledger.begin_submission(&revoked).expect("authorized");
    assert_eq!(ledger.revoke_client(ModuleId(1), ClientId(1)), 1);
    assert_eq!(ledger.revoke_client(ModuleId(1), ClientId(1)), 0);

    assert_eq!(ledger.authorize(&revoked), Err(AuthorizeRefusal::Revoked));
    assert_eq!(
        ledger.begin_submission(&revoked).expect_err("revoked"),
        AuthorizeRefusal::Revoked
    );
    assert!(ledger.hold(&revoked).is_err());
    assert!(ledger.charge(&revoked, ChargeClass::Committed, 0).is_err());
    assert_eq!(ledger.usage().charged_bytes, 50);
    // Revocation is not a settlement: its work is still in flight.
    let clean = Settlement::clean(SettleOutcome::Canceled);
    assert_eq!(
        ledger.settle(&revoked, clean),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    assert_eq!(ledger.resolve_through(1, 1), 1);
    ledger
        .discharge(&revoked, ChargeClass::Committed, 30)
        .expect("freed memory is always reported");
    assert!(ledger.settle(&revoked, clean).is_ok());
    assert_eq!(
        ledger.settle(&revoked, clean),
        Err(SettleRefusal::DoubleRelease)
    );

    assert_eq!(ledger.authorize(&other_client), Ok(()));
    assert_eq!(ledger.authorize(&other_module), Ok(()));
    let usage = ledger.usage_by_module();
    assert_eq!(
        usage
            .iter()
            .map(|module| (module.module.0, module.charged_bytes))
            .collect::<Vec<_>>(),
        vec![(1, 10), (2, 10)]
    );
}

#[test]
fn another_ledgers_claim_is_unknown_here() {
    let issuing = ledger(policy(100));
    let other = ledger(policy(100));
    let foreign = claim(&issuing, 10);
    let _same_id = claim(&other, 10);
    let clean = Settlement::clean(SettleOutcome::Completed);
    assert_eq!(other.authorize(&foreign), Err(AuthorizeRefusal::Unknown));
    assert_eq!(
        other.settle(&foreign, clean),
        Err(SettleRefusal::StaleClaim)
    );
    assert_eq!(
        other.release_retained(&foreign),
        Err(SettleRefusal::StaleClaim)
    );
    assert!(other.charge(&foreign, ChargeClass::Committed, 1).is_err());
    let submission = issuing.begin_submission(&foreign).expect("authorized");
    assert!(!other.issued(&submission));
    assert_eq!(
        other.submission_no_effect(&submission),
        Err(SubmissionRefusal::Unknown)
    );
    assert_eq!(submission.state(), SubmissionState::Pending);
}

#[test]
fn a_fence_value_ends_exactly_the_submissions_up_to_it() {
    let ledger = ledger(policy(100));
    let one = ledger.admit(&request(1, 1, 10)).expect("fits");
    let two = ledger.admit(&request(2, 2, 10)).expect("fits");
    let first = ledger.begin_submission(&one).expect("authorized");
    let second = ledger.begin_submission(&two).expect("authorized");
    let third = ledger.begin_submission(&one).expect("authorized");
    assert_eq!(
        [first.sequence(), second.sequence(), third.sequence()],
        [1, 2, 3]
    );
    assert_eq!((second.generation(), second.claim_info()), (1, two.info()));

    assert_eq!(ledger.resolve_through(1, 0), 0);
    assert_eq!(ledger.resolve_through(9, 3), 0);
    assert_eq!(ledger.resolve_through(1, 2), 2);
    let completed = SubmissionState::Finished(SubmissionObservation::Completed);
    assert_eq!(
        [first.state(), second.state(), third.state()],
        [completed, completed, SubmissionState::Pending]
    );
    let clean = Settlement::clean(SettleOutcome::Completed);
    assert!(ledger.settle(&two, clean).is_ok());
    assert_eq!(
        ledger.settle(&one, clean),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    // Observing the same value again ends nothing twice.
    assert_eq!(ledger.resolve_through(1, 2), 0);
    assert_eq!(ledger.resolve_through(1, u64::MAX), 1);
    assert!(ledger.settle(&one, clean).is_ok());
    assert_eq!(
        (ledger.counters().submitted, ledger.counters().completed),
        (3, 3)
    );
}

#[test]
fn a_submission_that_never_reached_the_queue_ends_once_as_no_effect() {
    let ledger = ledger(bounded_policy(100, 8, 1));
    let held = claim(&ledger, 10);
    let refused = ledger.begin_submission(&held).expect("authorized");
    assert_eq!(ledger.submission_no_effect(&refused), Ok(()));
    assert_eq!(
        refused.state(),
        SubmissionState::Finished(SubmissionObservation::NoEffect)
    );
    assert_eq!(
        ledger.submission_no_effect(&refused),
        Err(SubmissionRefusal::AlreadyResolved)
    );
    // Its place under the in-flight bound is free again and its fence value is not reused.
    assert_eq!(
        ledger
            .begin_submission(&held)
            .expect("room again")
            .sequence(),
        2
    );
    assert_eq!(ledger.usage().in_flight, 1);
}
