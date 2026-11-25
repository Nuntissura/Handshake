//! Device loss and rebind without a GPU: nothing is refunded, nothing stale submits and every loss is reported once.

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AuthorizeRefusal, ChargeClass, ClientId, DeviceFault, DeviceRebindRefusal,
    LedgerClient, ModuleId, OwnerCore, OwnerEvent, OwnerPhase, QuotaPolicy, RebindRefusal,
    SettleOutcome, SettleRefusal, Settlement, SubmissionObservation, SubmissionRefusal,
    SubmissionState, SubmitRefusal, Unavailable, WorkObserved,
};
use std::sync::Arc;
use support::{bench, claim_subject, client_subject, policy, request, EventLog, KEY, OTHER_KEY};

const DEVICE_LOST: SubmissionState = SubmissionState::Finished(SubmissionObservation::DeviceLost);

fn lost() -> Settlement {
    Settlement::clean(SettleOutcome::Lost)
}

fn attach(core: &OwnerCore) -> LedgerClient {
    core.attach(ModuleId(1), ClientId(1)).expect("free")
}

#[test]
fn a_loss_is_counted_once_and_refunds_nothing() {
    let bench = bench(policy(100));
    let (ledger, client) = (&bench.ledger, &bench.client);
    let held = bench.claim(60);
    client
        .charge(&held, ChargeClass::Committed, 50)
        .expect("fits");
    let hold = client.hold(&held).expect("authorized");
    let running = client.begin_submission(&held).expect("authorized");

    assert!(
        !ledger.mark_device_lost(2),
        "a generation that is not bound"
    );
    assert!(ledger.mark_device_lost(1));
    assert!(!ledger.mark_device_lost(1), "reported already");
    assert_eq!(ledger.counters().device_losses, 1);

    let usage = ledger.usage();
    assert_eq!(
        (usage.charged_bytes, usage.committed_bytes, usage.in_flight),
        (60, 50, 1)
    );
    let refused = AuthorizeRefusal::Lost { generation: 1 };
    assert_eq!(client.authorize(&held), Err(refused));
    // The holder of an idle lease learns of it as well.
    assert_eq!(hold.standing(), Err(refused));
    assert!(client.begin_submission(&held).is_err());
    assert!(client.hold(&held).is_err());
    assert!(client.charge(&held, ChargeClass::Committed, 1).is_err());
    assert_eq!(client.discharge(&held, ChargeClass::Committed, 10), Ok(()));
    assert_eq!(
        client.admit(&request(1)).expect_err("lost"),
        AdmissionDenial::DeviceLost { generation: 1 }
    );
    assert_eq!(
        running.state(),
        SubmissionState::Pending,
        "a loss alone ends no submission"
    );
    assert_eq!(
        client.settle(&held, lost()),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
}

#[test]
fn a_healthy_generation_is_neither_lost_nor_retired() {
    let bench = bench(policy(100));
    let (ledger, client) = (&bench.ledger, &bench.client);
    let held = bench.claim(10);
    client
        .charge(&held, ChargeClass::Committed, 10)
        .expect("fits");
    let running = client.begin_submission(&held).expect("authorized");
    let hold = client.hold(&held).expect("authorized");
    assert_eq!(ledger.resolve_lost(1), 0);
    assert_eq!(ledger.generation_retired(1), 0);
    assert_eq!(running.state(), SubmissionState::Pending);
    assert_eq!(ledger.usage().committed_bytes, 10);
    // Not asked to wait for a loss that never came, nor to return its lease for it.
    assert_eq!(
        client.rebind(&held).expect_err("healthy"),
        RebindRefusal::NotLost
    );
    ledger.resolve_through(1, 1);
    assert_eq!(
        client.rebind(&held).expect_err("healthy"),
        RebindRefusal::NotLost
    );
    assert_eq!(hold.standing(), Ok(()));
}

#[test]
fn a_quiescent_loss_ends_the_submissions_of_that_generation_only() {
    let bench = bench(policy(100));
    let held = bench.claim(10);
    let running = bench.client.begin_submission(&held).expect("authorized");
    bench.ledger.mark_device_lost(1);
    assert_eq!(bench.ledger.resolve_lost(2), 0);
    assert_eq!(running.state(), SubmissionState::Pending);
    assert_eq!(bench.ledger.resolve_lost(1), 1);
    assert_eq!(bench.ledger.resolve_lost(1), 0);
    assert_eq!(running.state(), DEVICE_LOST);
    assert!(bench.client.settle(&held, lost()).is_ok());
}

#[test]
fn a_lost_generation_completes_nothing_any_more() {
    let bench = bench(policy(100));
    let held = bench.claim(10);
    let first = bench.client.begin_submission(&held).expect("authorized");
    let second = bench.client.begin_submission(&held).expect("authorized");
    assert_eq!(bench.ledger.resolve_through(1, 1), 1);
    bench.ledger.mark_device_lost(1);
    for value in [0, 1, 2, u64::MAX] {
        assert_eq!(bench.ledger.resolve_through(1, value), 0);
    }
    assert_eq!(second.state(), SubmissionState::Pending);
    assert_eq!(bench.ledger.usage().in_flight, 1);
    // Nor once its successor is bound; and that one completes its own.
    bench.ledger.bind_device(KEY);
    assert_eq!(bench.ledger.resolve_through(1, u64::MAX), 0);
    let next = bench.claim(10);
    let running = bench.client.begin_submission(&next).expect("authorized");
    assert_eq!(bench.ledger.resolve_through(2, 1), 1);
    assert_eq!(bench.ledger.resolve_lost(1), 1);
    let completed = SubmissionState::Finished(SubmissionObservation::Completed);
    assert_eq!(
        [first.state(), second.state(), running.state()],
        [completed, DEVICE_LOST, completed]
    );
}

#[test]
fn lost_work_settles_neither_as_completed_nor_as_no_effect() {
    let bench = bench(policy(100));
    let (ledger, client) = (&bench.ledger, &bench.client);
    let held = bench.claim(10);
    let _first = client.begin_submission(&held).expect("authorized");
    let _second = client.begin_submission(&held).expect("authorized");
    assert_eq!(ledger.resolve_through(1, 1), 1);
    ledger.mark_device_lost(1);
    ledger.resolve_lost(1);
    let observed = WorkObserved {
        completed: 1,
        lost: 1,
        no_effect: 0,
    };
    for outcome in [SettleOutcome::Completed, SettleOutcome::NoEffect] {
        assert_eq!(
            client.settle(&held, Settlement::clean(outcome)),
            Err(SettleRefusal::OutcomeContradicted { observed })
        );
    }
    assert_eq!(ledger.usage().charged_bytes, 10);
    let receipt = client.settle(&held, lost()).expect("settles");
    assert_eq!(receipt.observed, observed);
}

#[test]
fn work_that_completed_before_the_loss_stays_completed() {
    let bench = bench(policy(100));
    let held = bench.claim(10);
    let idle = bench.claim(10);
    drop(bench.client.hold(&idle).expect("authorized"));
    let _done = bench.client.begin_submission(&held).expect("authorized");
    bench.ledger.resolve_through(1, 1);
    bench.ledger.mark_device_lost(1);
    bench.ledger.resolve_lost(1);
    let completed = Settlement::clean(SettleOutcome::Completed);
    assert!(bench.client.settle(&held, completed).is_ok());
    // Memory work on a device that is lost did not complete.
    assert!(matches!(
        bench.client.settle(&idle, completed),
        Err(SettleRefusal::OutcomeContradicted { .. })
    ));
    assert!(bench.client.settle(&idle, lost()).is_ok());
}

#[test]
fn a_rebind_moves_the_charge_once_and_closes_the_old_claim() {
    let bench = bench(policy(100));
    let (ledger, client) = (&bench.ledger, &bench.client);
    let old = bench.claim(60);
    client
        .charge(&old, ChargeClass::Committed, 60)
        .expect("fits");
    let running = client.begin_submission(&old).expect("authorized");
    ledger.mark_device_lost(1);
    assert_eq!(
        client.rebind(&old).expect_err("work unresolved"),
        RebindRefusal::InFlight { in_flight: 1 }
    );
    ledger.resolve_lost(1);
    assert_eq!(
        client.rebind(&old).expect_err("no successor device"),
        RebindRefusal::Lost { generation: 1 }
    );
    ledger.unbind_device();
    assert_eq!(
        client.rebind(&old).expect_err("no device"),
        RebindRefusal::DeviceUnavailable
    );
    assert_eq!(
        ledger.generation_retired(1),
        0,
        "nothing was left in flight"
    );
    assert_eq!(
        ledger.usage().committed_bytes,
        0,
        "its memory is gone with the device"
    );
    assert_eq!(
        ledger.usage().charged_bytes,
        60,
        "its reservation is not refunded"
    );

    assert_eq!(ledger.bind_device(KEY), 2);
    let stale = AuthorizeRefusal::Lost { generation: 1 };
    assert_eq!(client.authorize(&old), Err(stale));
    let successor = client.rebind(&old).expect("moves");
    assert_eq!(
        (successor.generation(), successor.reserved_bytes()),
        (2, 60)
    );
    assert_eq!(successor.info().module, old.info().module);
    assert_eq!(successor.info().operation_id, old.info().operation_id);
    assert_eq!(
        (ledger.usage().charged_bytes, ledger.usage().open_claims),
        (60, 1)
    );
    assert_eq!(
        bench.log.count(OwnerEvent::ClaimRebound {
            subject: claim_subject(successor.info()),
            superseded: old.id()
        }),
        1
    );

    assert_eq!(client.settle(&old, lost()), Err(SettleRefusal::LateRelease));
    assert_eq!(
        client.release_retained(&old),
        Err(SettleRefusal::LateRelease)
    );
    assert_eq!(
        client.rebind(&old).expect_err("superseded"),
        RebindRefusal::NotOpen
    );
    assert_eq!(client.authorize(&old), Err(stale));
    assert_eq!(
        client.rebind(&successor).expect_err("current"),
        RebindRefusal::NotLost
    );

    // The successor's fence starts over; the old generation's values say nothing about it.
    let next = client.begin_submission(&successor).expect("authorized");
    assert_eq!((next.generation(), next.sequence()), (2, 1));
    assert_eq!(ledger.resolve_through(1, u64::MAX), 0);
    assert_eq!(next.state(), SubmissionState::Pending);
    assert_eq!(running.state(), DEVICE_LOST);
    ledger.resolve_through(2, 1);
    let receipt = client
        .settle(&successor, Settlement::clean(SettleOutcome::Completed))
        .expect("its work was done again and completed");
    assert_eq!(receipt.released_bytes, 60);
    assert_eq!((receipt.observed.completed, receipt.observed.lost), (1, 1));
    assert_eq!(ledger.usage().charged_bytes, 0);
}

#[test]
fn a_successor_does_not_hide_the_work_its_predecessor_lost() {
    let bench = bench(policy(100));
    let (ledger, client) = (&bench.ledger, &bench.client);
    let old = bench.claim(10);
    let _running = client.begin_submission(&old).expect("authorized");
    ledger.mark_device_lost(1);
    ledger.resolve_lost(1);
    ledger.bind_device(KEY);
    let successor = client.rebind(&old).expect("moves");
    for outcome in [SettleOutcome::Completed, SettleOutcome::NoEffect] {
        assert!(matches!(
            client.settle(&successor, Settlement::clean(outcome)),
            Err(SettleRefusal::OutcomeContradicted { .. })
        ));
    }
    assert!(client.settle(&successor, lost()).is_ok());
}

#[test]
fn a_claim_is_not_rebound_under_a_hold_or_onto_another_adapter() {
    let bench = bench(policy(100));
    let held = bench.claim(10);
    let hold = bench.client.hold(&held).expect("authorized");
    bench.ledger.mark_device_lost(1);
    bench.ledger.bind_device(OTHER_KEY);
    assert_eq!(
        bench.client.rebind(&held).expect_err("held"),
        RebindRefusal::LeaseOutstanding { leases: 1 }
    );
    drop(hold);
    assert_eq!(
        bench.client.rebind(&held).expect_err("other adapter"),
        RebindRefusal::DeviceChanged
    );
    assert_eq!(bench.ledger.usage().charged_bytes, 10);
    bench
        .ledger
        .revoke_client(held.info().module, held.info().client);
    assert_eq!(
        bench.client.rebind(&held).expect_err("revoked"),
        RebindRefusal::NotOpen
    );
}

/// The lost device may live on for another client: memory on it is not gone by the loss.
#[test]
fn a_claim_is_not_rebound_while_it_reports_memory_on_the_lost_device() {
    let bench = bench(policy(100));
    let (ledger, client) = (&bench.ledger, &bench.client);
    let frees = bench.claim(10);
    let waits = bench.claim(10);
    for claim in [&frees, &waits] {
        client
            .charge(claim, ChargeClass::Committed, 4)
            .expect("fits");
    }
    ledger.mark_device_lost(1);
    ledger.bind_device(KEY);
    for claim in [&frees, &waits] {
        assert_eq!(
            client.rebind(claim).expect_err("holds memory"),
            RebindRefusal::MemoryHeld { bytes: 4 }
        );
    }
    client
        .discharge(&frees, ChargeClass::Committed, 4)
        .expect("charged");
    let moved = client.rebind(&frees).expect("freed it");
    ledger.generation_retired(1);
    let moved_too = client.rebind(&waits).expect("gone with the device");
    assert_eq!((moved.generation(), moved_too.generation()), (2, 2));
    assert_eq!(ledger.usage().charged_bytes, 20);
}

#[test]
fn a_destroyed_device_ends_what_was_in_flight_on_it() {
    let bench = bench(policy(100));
    let (ledger, client) = (&bench.ledger, &bench.client);
    let held = bench.claim(30);
    client
        .charge(&held, ChargeClass::Staging, 30)
        .expect("fits");
    let running = client.begin_submission(&held).expect("authorized");
    ledger.mark_device_lost(1);
    assert_eq!(ledger.generation_retired(1), 1);
    assert_eq!(running.state(), DEVICE_LOST);
    let counters = ledger.counters();
    assert_eq!(
        (counters.retired_in_flight, counters.completed_lost),
        (1, 1)
    );
    assert_eq!(ledger.usage().charged_bytes, 30);
    assert!(client.settle(&held, lost()).is_ok());
}

#[test]
fn the_owner_reports_a_loss_once_and_every_refusal_with_whom_it_concerns() {
    let log = Arc::new(EventLog::default());
    let core = OwnerCore::new(QuotaPolicy::Unavailable, log.clone());
    let client = attach(&core);
    assert!(client.admit(&request(1)).is_err());
    core.device_unavailable(Unavailable::Driver(-13));
    let generation = core.device_bound(KEY);
    assert!(client.admit(&request(1)).is_err());
    core.ledger().set_policy(policy(100));
    let held = client.admit(&request(20)).expect("fits");
    let submission = client.begin_submission(&held).expect("authorized");
    assert!(core.device_lost(generation));
    assert!(!core.device_lost(generation));
    assert!(client.begin_submission(&held).is_err());
    client.submit_refused(&held, SubmitRefusal::RecordAborted);
    client.submission_refused(&submission, SubmissionRefusal::Unknown);
    core.device_fault(generation, DeviceFault::Driver(-13));
    core.device_rebind_refused(DeviceRebindRefusal::DeviceHeld { holders: 1 });
    core.lost_device_set_aside(generation, 1);
    assert!(client.charge(&held, ChargeClass::Staging, 1).is_err());
    assert!(client.settle(&held, lost()).is_err());
    assert!(client.rebind(&held).is_err());
    core.retained_device_ended(generation, true);
    core.retained_device_ended(generation, false);
    let of_client = client_subject(1, 1);
    let asked = handshake_vulkan_renderer::Subject {
        operation_id: Some(request(1).operation_id),
        ..of_client
    };
    let subject = claim_subject(held.info());
    let phase = |phase, generation| OwnerEvent::Phase { phase, generation };
    let lost = AuthorizeRefusal::Lost { generation };
    assert_eq!(
        log.events(),
        vec![
            OwnerEvent::ClientAttached { subject: of_client },
            OwnerEvent::AdmissionDenied {
                subject: asked,
                reason: AdmissionDenial::DeviceUnavailable.number()
            },
            OwnerEvent::Unavailable {
                reason: 13,
                vk_result: -13
            },
            phase(OwnerPhase::Unavailable, 0),
            phase(OwnerPhase::Open, generation),
            OwnerEvent::AdmissionDenied {
                subject: asked,
                reason: AdmissionDenial::HostQuotaUnavailable.number()
            },
            OwnerEvent::Admitted {
                subject,
                reserved_bytes: 20
            },
            OwnerEvent::VulkanDeviceLost { generation },
            phase(OwnerPhase::DeviceLost, generation),
            OwnerEvent::AuthorizeRefused {
                subject,
                reason: lost.number()
            },
            OwnerEvent::SubmitRefused {
                subject,
                reason: SubmitRefusal::RecordAborted.number()
            },
            OwnerEvent::SubmissionRefused {
                subject,
                reason: SubmissionRefusal::Unknown.number()
            },
            OwnerEvent::DeviceFault {
                generation,
                fault: 4,
                vk_result: -13
            },
            OwnerEvent::DeviceRebindRefused {
                reason: DeviceRebindRefusal::DeviceHeld { holders: 1 }.number()
            },
            OwnerEvent::LostDeviceSetAside {
                generation,
                holders: 1
            },
            OwnerEvent::ChargeRefused { subject, reason: 1 },
            OwnerEvent::SettleRefused {
                subject,
                reason: SettleRefusal::InFlight { in_flight: 1 }.number()
            },
            OwnerEvent::ClaimRebindRefused {
                subject,
                reason: RebindRefusal::InFlight { in_flight: 1 }.number()
            },
            OwnerEvent::RetainedDeviceDestroyed {
                generation,
                resolved: 1
            },
            OwnerEvent::DeviceLeaked { generation },
        ]
    );
}

#[test]
fn ordinary_work_emits_no_event_per_submission() {
    let log = Arc::new(EventLog::default());
    let core = OwnerCore::new(policy(100), log.clone());
    let client = attach(&core);
    let generation = core.device_bound(KEY);
    let held = client.admit(&request(20)).expect("fits");
    let before = log.events().len();
    for value in 1..=3 {
        client.begin_submission(&held).expect("authorized");
        assert_eq!(core.ledger().resolve_through(generation, value), 1);
    }
    assert_eq!(log.events().len(), before);
}
