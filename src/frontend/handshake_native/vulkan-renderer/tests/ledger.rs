//! The claim ledger without a GPU: admission, accounting and settlement of one client's claims.

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AuthorizeRefusal, ByteQuantity, ChargeClass, ChargeRefusal, ClaimDisposition,
    ClaimLedger, NullDiagnostics, PolicyRevision, QuotaPolicy, SettleOutcome, SettleRefusal,
    Settlement, SubmissionObservation, SubmissionRefusal, SubmissionState, WorkObserved,
};
use std::sync::{Arc, Barrier};
use support::{attach, bench, bounded_policy, policy, request, KEY};

const COMPLETED: SubmissionState = SubmissionState::Finished(SubmissionObservation::Completed);

/// Canceled, so that it says nothing about what ran.
fn clean() -> Settlement {
    Settlement::clean(SettleOutcome::Canceled)
}

fn retaining(retained: ByteQuantity) -> Settlement {
    Settlement {
        retained,
        ..clean()
    }
}

#[test]
fn without_a_quota_every_admission_is_denied() {
    let bench = bench(QuotaPolicy::Unavailable);
    for bytes in [0, 1, u64::MAX] {
        assert_eq!(
            bench.client.admit(&request(bytes)).expect_err("no quota"),
            AdmissionDenial::HostQuotaUnavailable
        );
    }
    let usage = bench.ledger.usage();
    assert_eq!((usage.charged_bytes, usage.open_claims), (0, 0));
    let counters = bench.ledger.counters();
    assert_eq!((counters.admitted, counters.denied), (0, 3));
}

#[test]
fn admission_needs_a_healthy_bound_device_and_an_open_ledger() {
    let ledger = ClaimLedger::new(policy(100), Arc::new(NullDiagnostics));
    let client = attach(&ledger, 1, 1);
    let ask = request(10);
    assert_eq!(
        client.admit(&ask).expect_err("no device"),
        AdmissionDenial::DeviceUnavailable
    );
    let generation = ledger.bind_device(KEY);
    assert!(ledger.mark_device_lost(generation));
    assert_eq!(
        client.admit(&ask).expect_err("lost"),
        AdmissionDenial::DeviceLost { generation }
    );
    ledger.bind_device(KEY);
    ledger.close_admission();
    assert_eq!(
        client.admit(&ask).expect_err("closed"),
        AdmissionDenial::AdmissionClosed
    );
    assert_eq!(ledger.usage().charged_bytes, 0);
}

#[test]
fn a_reservation_must_fit_what_the_quota_has_left() {
    let bench = bench(policy(100));
    let first = bench.claim(60);
    assert_eq!(first.info().policy_revision, PolicyRevision(7));
    assert!(bench.client.has_room(&request(40)));
    assert!(!bench.client.has_room(&request(41)));
    assert_eq!(
        bench.client.admit(&request(41)).expect_err("too big"),
        AdmissionDenial::BudgetExceeded {
            requested_bytes: 41
        }
    );
    let _second = bench.claim(40);
    assert_eq!(
        bench.client.admit(&request(1)).expect_err("full"),
        AdmissionDenial::BudgetExceeded { requested_bytes: 1 }
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 100);
}

#[test]
fn the_largest_byte_numbers_do_not_wrap() {
    let bench = bench(policy(u64::MAX));
    let all = bench.claim(u64::MAX);
    assert_eq!(
        bench.client.admit(&request(1)).expect_err("full"),
        AdmissionDenial::BudgetExceeded { requested_bytes: 1 }
    );
    let client = &bench.client;
    assert_eq!(
        client.charge(&all, ChargeClass::Committed, u64::MAX),
        Ok(())
    );
    assert_eq!(
        client.charge(&all, ChargeClass::Staging, u64::MAX),
        Err(ChargeRefusal::ReservationExceeded {
            reserved_bytes: u64::MAX,
            charged_bytes: u64::MAX,
            requested_bytes: u64::MAX,
        })
    );
    assert_eq!(bench.ledger.usage().charged_bytes, u64::MAX);
    assert_eq!(
        client.discharge(&all, ChargeClass::Committed, u64::MAX),
        Ok(())
    );
    let receipt = client.settle(&all, clean()).expect("settles");
    assert_eq!(receipt.released_bytes, u64::MAX);
    assert_eq!(bench.ledger.usage().charged_bytes, 0);
}

#[test]
fn the_number_of_claims_is_bounded_and_retained_claims_count() {
    let bench = bench(bounded_policy(100, 2, 4));
    let first = bench.claim(10);
    let second = bench.claim(0);
    let denied = AdmissionDenial::ClaimLimit { max_claims: 2 };
    assert!(!bench.client.has_room(&request(0)));
    assert_eq!(
        bench.client.admit(&request(0)).expect_err("two held"),
        denied
    );
    bench
        .client
        .settle(&first, retaining(ByteQuantity::Measured(5)))
        .expect("retains");
    assert_eq!(
        bench.client.admit(&request(0)).expect_err("one retains"),
        denied
    );
    bench.client.settle(&second, clean()).expect("closes");
    assert!(bench.client.admit(&request(0)).is_ok());
}

#[test]
fn a_policy_change_leaves_existing_claims_alone() {
    let bench = bench(bounded_policy(100, 8, 1));
    let old = bench.claim(80);
    bench.ledger.set_policy(QuotaPolicy::Configured {
        revision: PolicyRevision(8),
        limit_bytes: 50,
        max_claims: 8,
        max_in_flight_per_claim: 9,
    });
    assert_eq!(
        bench
            .client
            .admit(&request(1))
            .expect_err("over the new limit"),
        AdmissionDenial::BudgetExceeded { requested_bytes: 1 }
    );
    bench.ledger.set_policy(QuotaPolicy::Unavailable);
    assert_eq!(old.info().policy_revision, PolicyRevision(7));
    assert_eq!(bench.client.authorize(&old), Ok(()));
    assert!(!bench.client.is_in_flight_full(&old));
    let _running = bench.client.begin_submission(&old).expect("first");
    let full = AuthorizeRefusal::InFlightLimit { max_in_flight: 1 };
    assert!(bench.client.is_in_flight_full(&old));
    assert_eq!(bench.client.authorize_submission(&old), Err(full));
    assert_eq!(
        bench
            .client
            .begin_submission(&old)
            .expect_err("bound of its own policy"),
        full
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 80);
}

/// Every round has room for exactly one claim and lets all threads ask at the same moment. A check and a reservation
/// that are not one step let two in, in some of the rounds.
#[test]
fn concurrent_admissions_never_spend_the_same_quota_twice() {
    const ROUNDS: usize = 400;
    const THREADS: u64 = 8;
    let bench = bench(bounded_policy(10, 1000, 1));
    let clients: Vec<_> = (10..10 + THREADS)
        .map(|client| attach(&bench.ledger, 1, client))
        .collect();
    let barrier = Barrier::new(clients.len());
    // Nothing panics between two waits: the other threads would wait for ever.
    let (mut admitted, mut spent_twice, mut unsettled) = (0, 0, 0);
    std::thread::scope(|scope| {
        let workers: Vec<_> = clients
            .iter()
            .map(|client| {
                let (barrier, ledger) = (&barrier, &bench.ledger);
                scope.spawn(move || {
                    let mut seen = (0, 0, 0);
                    for _ in 0..ROUNDS {
                        barrier.wait();
                        let claim = client.admit(&request(10)).ok();
                        barrier.wait();
                        seen.1 += usize::from(ledger.usage().charged_bytes != 10);
                        barrier.wait();
                        if let Some(claim) = claim {
                            seen.0 += 1;
                            seen.2 += usize::from(client.settle(&claim, clean()).is_err());
                        }
                    }
                    seen
                })
            })
            .collect();
        for worker in workers {
            let seen = worker.join().expect("no panic");
            admitted += seen.0;
            spent_twice += seen.1;
            unsettled += seen.2;
        }
    });
    assert_eq!((admitted, spent_twice, unsettled), (ROUNDS, 0, 0));
    let counters = bench.ledger.counters();
    let asked = ROUNDS as u64 * THREADS;
    assert_eq!(counters.admitted + counters.denied, asked);
    assert_eq!(bench.ledger.usage().charged_bytes, 0);
}

#[test]
fn charges_stay_inside_their_reservation_and_class() {
    let bench = bench(policy(100));
    let client = &bench.client;
    let held = bench.claim(100);
    assert_eq!(client.charge(&held, ChargeClass::Committed, 70), Ok(()));
    assert_eq!(client.charge(&held, ChargeClass::Staging, 30), Ok(()));
    let full = ChargeRefusal::ReservationExceeded {
        reserved_bytes: 100,
        charged_bytes: 100,
        requested_bytes: 1,
    };
    assert_eq!(client.charge(&held, ChargeClass::Staging, 1), Err(full));
    assert_eq!(
        client.discharge(&held, ChargeClass::Staging, 31),
        Err(ChargeRefusal::NotCharged {
            charged_bytes: 30,
            requested_bytes: 31
        })
    );
    assert_eq!(client.discharge(&held, ChargeClass::Staging, 30), Ok(()));
    let usage = bench.ledger.usage();
    assert_eq!(
        (
            usage.committed_bytes,
            usage.staging_bytes,
            usage.charged_bytes
        ),
        (70, 0, 100)
    );
    assert_eq!(bench.ledger.counters().charge_refused, 2);
    client
        .discharge(&held, ChargeClass::Committed, 70)
        .expect("charged");
}

#[test]
fn a_claim_settles_once_and_is_released_once() {
    let bench = bench(policy(100));
    let held = bench.claim(40);
    let receipt = bench.client.settle(&held, clean()).expect("settles");
    assert_eq!(
        (receipt.released_bytes, receipt.disposition),
        (40, ClaimDisposition::Closed)
    );
    assert_eq!(receipt.claim, held.info());
    for _ in 0..2 {
        assert_eq!(
            bench.client.settle(&held, clean()),
            Err(SettleRefusal::DoubleRelease)
        );
        assert_eq!(
            bench.client.release_retained(&held),
            Err(SettleRefusal::DoubleRelease)
        );
    }
    assert_eq!(
        bench.client.authorize(&held),
        Err(AuthorizeRefusal::Settled)
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 0);
    let counters = bench.ledger.counters();
    assert_eq!((counters.settled, counters.double_release), (1, 4));
}

#[test]
fn work_in_flight_keeps_a_claim_unsettled_whatever_the_client_reports() {
    let bench = bench(policy(100));
    let client = &bench.client;
    let held = bench.claim(40);
    let running = client.begin_submission(&held).expect("authorized");
    assert_eq!(
        client.settle(&held, clean()),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    let more = Settlement {
        in_flight: 3,
        ..clean()
    };
    assert_eq!(
        client.settle(&held, more),
        Err(SettleRefusal::InFlight { in_flight: 3 })
    );
    // A dropped submission stays counted.
    drop(running);
    assert_eq!(
        client.settle(&held, clean()),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 40);
    assert_eq!(bench.ledger.resolve_through(1, 1), 1);
    let reported = Settlement {
        in_flight: 1,
        ..clean()
    };
    assert_eq!(
        client.settle(&held, reported),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    assert!(client.settle(&held, clean()).is_ok());
}

/// Every outcome against every kind of work the owner can have observed on a healthy device.
#[test]
fn an_outcome_that_contradicts_what_was_observed_is_refused() {
    use SettleOutcome::{Abandoned, Canceled, Completed, Failed, Lost, NoEffect};
    #[derive(Clone, Copy, Debug)]
    enum Touch {
        Nothing,
        Lease,
        /// Charged and discharged again.
        Memory(u64),
    }
    let completed = WorkObserved {
        completed: 1,
        ..WorkObserved::default()
    };
    let refused = WorkObserved {
        no_effect: 1,
        ..WorkObserved::default()
    };
    let nothing = WorkObserved::default();
    let ran = &[Completed, Canceled, Failed, Abandoned][..];
    let ran_not = &[NoEffect, Canceled, Failed, Abandoned][..];
    let either = &[Completed, NoEffect, Canceled, Failed, Abandoned][..];
    for (observed, touch, accepted) in [
        (completed, Touch::Nothing, ran),
        (refused, Touch::Nothing, ran_not),
        (nothing, Touch::Nothing, ran_not),
        (nothing, Touch::Lease, either),
        (nothing, Touch::Memory(1), either),
        // No memory is no memory work.
        (nothing, Touch::Memory(0), ran_not),
    ] {
        for outcome in [Completed, Canceled, Failed, Lost, NoEffect, Abandoned] {
            let bench = bench(policy(100));
            let held = bench.claim(10);
            match touch {
                Touch::Nothing => {}
                Touch::Lease => drop(bench.client.hold(&held).expect("authorized")),
                Touch::Memory(bytes) => {
                    let class = ChargeClass::Committed;
                    assert_eq!(bench.client.charge(&held, class, bytes), Ok(()));
                    assert_eq!(bench.client.discharge(&held, class, bytes), Ok(()));
                }
            }
            if observed != nothing {
                let submission = bench.client.begin_submission(&held).expect("authorized");
                if observed == completed {
                    assert_eq!(bench.ledger.resolve_through(1, 1), 1);
                } else {
                    assert_eq!(bench.ledger.submission_no_effect(&submission), Ok(()));
                }
            }
            let result = bench.client.settle(&held, Settlement::clean(outcome));
            if accepted.contains(&outcome) {
                assert_eq!(
                    result.map(|receipt| (receipt.observed, receipt.released_bytes)),
                    Ok((observed, 10)),
                    "{outcome:?} after {touch:?}"
                );
            } else {
                assert_eq!(
                    result,
                    Err(SettleRefusal::OutcomeContradicted { observed }),
                    "{outcome:?} after {touch:?} and {observed:?}"
                );
                assert_eq!(bench.ledger.usage().charged_bytes, 10);
            }
        }
    }
}

#[test]
fn retained_memory_stays_charged_until_its_own_release() {
    let bench = bench(policy(100));
    let client = &bench.client;
    let held = bench.claim(100);
    assert_eq!(
        client.release_retained(&held),
        Err(SettleRefusal::NotSettled)
    );
    let receipt = client
        .settle(&held, retaining(ByteQuantity::Measured(30)))
        .expect("settles");
    assert_eq!(
        (receipt.released_bytes, receipt.disposition),
        (70, ClaimDisposition::Retained { charged_bytes: 30 })
    );
    assert_eq!(client.authorize(&held), Err(AuthorizeRefusal::Settled));
    assert_eq!(
        client.settle(&held, retaining(ByteQuantity::Measured(0))),
        Err(SettleRefusal::AlreadySettled)
    );
    let usage = bench.ledger.usage();
    assert_eq!(
        (
            usage.charged_bytes,
            usage.retained_bytes,
            usage.retained_claims
        ),
        (30, 30, 1)
    );
    assert_eq!(
        client
            .release_retained(&held)
            .expect("releases")
            .released_bytes,
        30
    );
    assert_eq!(
        client.release_retained(&held),
        Err(SettleRefusal::DoubleRelease)
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 0);
}

#[test]
fn an_unmeasured_remainder_never_becomes_zero() {
    for unmeasured in [
        ByteQuantity::Estimated(0),
        ByteQuantity::Estimated(5),
        ByteQuantity::Unknown,
    ] {
        let bench = bench(policy(100));
        let held = bench.claim(60);
        let receipt = bench
            .client
            .settle(&held, retaining(unmeasured))
            .expect("settles");
        assert_eq!(
            (receipt.released_bytes, receipt.disposition),
            (0, ClaimDisposition::Retained { charged_bytes: 60 })
        );
        assert_eq!(bench.ledger.usage().charged_bytes, 60);
    }
}

#[test]
fn a_settlement_that_contradicts_the_accounting_is_refused() {
    let bench = bench(policy(100));
    let client = &bench.client;
    let held = bench.claim(50);
    assert_eq!(
        client.settle(&held, retaining(ByteQuantity::Measured(51))),
        Err(SettleRefusal::RetainedExceedsReservation {
            reserved_bytes: 50,
            retained_bytes: 51
        })
    );
    client
        .charge(&held, ChargeClass::Committed, 20)
        .expect("fits");
    client.charge(&held, ChargeClass::Staging, 5).expect("fits");
    assert_eq!(
        client.settle(&held, retaining(ByteQuantity::Measured(0))),
        Err(SettleRefusal::RetainedBelowCharged {
            charged_bytes: 25,
            retained_bytes: 0
        })
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 50);
    client
        .discharge(&held, ChargeClass::Staging, 5)
        .expect("charged");
    let receipt = client
        .settle(&held, retaining(ByteQuantity::Measured(20)))
        .expect("agrees");
    assert_eq!(receipt.released_bytes, 30);
}

#[test]
fn a_held_device_keeps_a_claim_unsettled() {
    let bench = bench(policy(100));
    let client = &bench.client;
    let held = bench.claim(10);
    let first = client.hold(&held).expect("authorized");
    let second = client.hold(&held).expect("authorized");
    assert_eq!(bench.ledger.usage().leases, 2);
    assert_eq!(
        client.settle(&held, clean()),
        Err(SettleRefusal::LeaseOutstanding { leases: 2 })
    );
    drop(first);
    assert_eq!(
        client.settle(&held, clean()),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    drop(second);
    assert!(client.settle(&held, clean()).is_ok());
    assert!(client.hold(&held).is_err());
}

#[test]
fn retained_memory_keeps_its_lease_until_it_is_released() {
    let bench = bench(policy(100));
    let client = &bench.client;
    let held = bench.claim(10);
    let hold = client.hold(&held).expect("authorized");
    client
        .settle(&held, retaining(ByteQuantity::Measured(4)))
        .expect("retains under its lease");
    assert_eq!(bench.ledger.usage().leases, 1);
    assert_eq!(hold.standing(), Err(AuthorizeRefusal::Settled));
    assert_eq!(
        client.release_retained(&held),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 4);
    drop(hold);
    assert_eq!(
        client
            .release_retained(&held)
            .expect("releases")
            .released_bytes,
        4
    );
    let usage = bench.ledger.usage();
    assert_eq!((usage.charged_bytes, usage.leases), (0, 0));
}

#[test]
fn a_fence_value_ends_exactly_the_submissions_up_to_it() {
    let bench = bench(policy(100));
    let other = attach(&bench.ledger, 2, 2);
    let one = bench.claim(10);
    let two = other.admit(&request(10)).expect("fits");
    let first = bench.client.begin_submission(&one).expect("authorized");
    let second = other.begin_submission(&two).expect("authorized");
    let third = bench.client.begin_submission(&one).expect("authorized");
    assert_eq!(
        [first.sequence(), second.sequence(), third.sequence()],
        [1, 2, 3]
    );
    assert_eq!((second.generation(), second.claim_info()), (1, two.info()));

    let ledger = &bench.ledger;
    assert_eq!(ledger.resolve_through(1, 0), 0);
    assert_eq!(ledger.resolve_through(9, 3), 0);
    assert_eq!(ledger.resolve_through(1, 2), 2);
    assert_eq!(
        [first.state(), second.state(), third.state()],
        [COMPLETED, COMPLETED, SubmissionState::Pending]
    );
    let done = Settlement::clean(SettleOutcome::Completed);
    assert!(other.settle(&two, done).is_ok());
    assert_eq!(
        bench.client.settle(&one, done),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    // Observing the same value again ends nothing twice.
    assert_eq!(ledger.resolve_through(1, 2), 0);
    assert_eq!(ledger.resolve_through(1, u64::MAX), 1);
    let receipt = bench.client.settle(&one, done).expect("settles");
    assert_eq!(receipt.observed.completed, 2);
    let counters = ledger.counters();
    assert_eq!((counters.submitted, counters.completed), (3, 3));
}

#[test]
fn a_submission_that_never_reached_the_queue_ends_once_as_no_effect() {
    let bench = bench(bounded_policy(100, 8, 1));
    let held = bench.claim(10);
    let refused = bench.client.begin_submission(&held).expect("authorized");
    assert_eq!(bench.ledger.submission_no_effect(&refused), Ok(()));
    assert_eq!(
        refused.state(),
        SubmissionState::Finished(SubmissionObservation::NoEffect)
    );
    assert_eq!(
        bench.ledger.submission_no_effect(&refused),
        Err(SubmissionRefusal::AlreadyResolved)
    );
    // Its place under the in-flight bound is free again and its fence value is not reused.
    assert_eq!(
        bench
            .client
            .begin_submission(&held)
            .expect("room again")
            .sequence(),
        2
    );
    assert_eq!(bench.ledger.usage().in_flight, 1);
}
