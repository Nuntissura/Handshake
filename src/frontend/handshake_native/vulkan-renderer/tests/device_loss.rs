//! Device loss, rebind and shutdown without a GPU: nothing is refunded, nothing stale submits, every loss is
//! reported once and shutdown says truthfully what remained.

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AuthorizeRefusal, ChargeClass, DeviceFault, DeviceRebindRefusal, OwnerCore,
    OwnerEvent, OwnerPhase, QuotaPolicy, RebindRefusal, SettleOutcome, SettleRefusal, Settlement,
    SubmissionObservation, SubmissionState, SubmitRefusal, Unavailable, DESCRIPTOR,
};
use std::sync::Arc;
use support::{claim, ledger, policy, request, EventLog, KEY, OTHER_KEY};

#[test]
fn a_loss_is_counted_once_and_refunds_nothing() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 60);
    ledger
        .charge(&held, ChargeClass::Committed, 50)
        .expect("fits");
    let running = ledger.begin_submission(&held).expect("authorized");

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
    assert_eq!(
        ledger.authorize(&held),
        Err(AuthorizeRefusal::Lost { generation: 1 })
    );
    assert!(ledger.begin_submission(&held).is_err());
    assert!(ledger.hold(&held).is_err());
    assert!(ledger.charge(&held, ChargeClass::Committed, 1).is_err());
    assert_eq!(ledger.discharge(&held, ChargeClass::Committed, 10), Ok(()));
    assert_eq!(
        ledger.admit(&request(1, 2, 1)).expect_err("lost"),
        AdmissionDenial::DeviceLost { generation: 1 }
    );
    assert_eq!(
        running.state(),
        SubmissionState::Pending,
        "a loss alone ends no submission"
    );
    assert_eq!(
        ledger.settle(&held, Settlement::clean(SettleOutcome::Lost)),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
}

#[test]
fn a_healthy_generation_is_neither_lost_nor_retired() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 10);
    ledger
        .charge(&held, ChargeClass::Committed, 10)
        .expect("fits");
    let running = ledger.begin_submission(&held).expect("authorized");
    assert_eq!(ledger.resolve_lost(1), 0);
    assert_eq!(ledger.generation_retired(1), 0);
    assert_eq!(running.state(), SubmissionState::Pending);
    assert_eq!(ledger.usage().committed_bytes, 10);
    assert_eq!(
        ledger.rebind(&held).expect_err("healthy"),
        RebindRefusal::NotLost
    );
}

#[test]
fn a_quiescent_loss_ends_the_submissions_of_that_generation_only() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 10);
    let running = ledger.begin_submission(&held).expect("authorized");
    ledger.mark_device_lost(1);
    assert_eq!(ledger.resolve_lost(2), 0);
    assert_eq!(ledger.resolve_lost(1), 1);
    assert_eq!(ledger.resolve_lost(1), 0);
    assert_eq!(
        running.state(),
        SubmissionState::Finished(SubmissionObservation::DeviceLost)
    );
    assert!(ledger
        .settle(&held, Settlement::clean(SettleOutcome::Lost))
        .is_ok());
}

#[test]
fn a_rebind_moves_the_charge_once_and_closes_the_old_claim() {
    let ledger = ledger(policy(100));
    let old = claim(&ledger, 60);
    ledger
        .charge(&old, ChargeClass::Committed, 60)
        .expect("fits");
    let running = ledger.begin_submission(&old).expect("authorized");
    ledger.mark_device_lost(1);
    assert_eq!(
        ledger.rebind(&old).expect_err("work unresolved"),
        RebindRefusal::InFlight { in_flight: 1 }
    );
    ledger.resolve_lost(1);
    assert_eq!(
        ledger.rebind(&old).expect_err("no successor device"),
        RebindRefusal::Lost { generation: 1 }
    );
    ledger.unbind_device();
    assert_eq!(
        ledger.rebind(&old).expect_err("no device"),
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
    assert_eq!(
        ledger.authorize(&old),
        Err(AuthorizeRefusal::Lost { generation: 1 })
    );
    let successor = ledger.rebind(&old).expect("moves");
    assert_eq!(
        (successor.generation(), successor.reserved_bytes()),
        (2, 60)
    );
    assert_eq!(successor.info().module, old.info().module);
    assert_eq!(successor.info().operation_id, old.info().operation_id);
    assert_eq!(ledger.usage().charged_bytes, 60);
    assert_eq!(ledger.usage().open_claims, 1);

    let clean = Settlement::clean(SettleOutcome::Lost);
    assert_eq!(ledger.settle(&old, clean), Err(SettleRefusal::LateRelease));
    assert_eq!(
        ledger.release_retained(&old),
        Err(SettleRefusal::LateRelease)
    );
    assert_eq!(
        ledger.rebind(&old).expect_err("superseded"),
        RebindRefusal::NotOpen
    );
    assert_eq!(
        ledger.authorize(&old),
        Err(AuthorizeRefusal::Lost { generation: 1 })
    );
    assert_eq!(
        ledger.rebind(&successor).expect_err("current"),
        RebindRefusal::NotLost
    );

    // The successor's fence starts over; the old generation's values say nothing about it.
    let next = ledger.begin_submission(&successor).expect("authorized");
    assert_eq!((next.generation(), next.sequence()), (2, 1));
    assert_eq!(ledger.resolve_through(1, u64::MAX), 0);
    assert_eq!(next.state(), SubmissionState::Pending);
    assert_eq!(
        running.state(),
        SubmissionState::Finished(SubmissionObservation::DeviceLost)
    );
    ledger.resolve_through(2, 1);
    assert_eq!(
        ledger
            .settle(&successor, Settlement::clean(SettleOutcome::Completed))
            .expect("settles")
            .released_bytes,
        60
    );
    assert_eq!(ledger.usage().charged_bytes, 0);
}

#[test]
fn a_claim_is_not_rebound_under_a_hold_or_onto_another_adapter() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 10);
    let hold = ledger.hold(&held).expect("authorized");
    ledger.mark_device_lost(1);
    ledger.unbind_device();
    ledger.bind_device(OTHER_KEY);
    assert_eq!(
        ledger.rebind(&held).expect_err("held"),
        RebindRefusal::LeaseOutstanding { leases: 1 }
    );
    drop(hold);
    assert_eq!(
        ledger.rebind(&held).expect_err("other adapter"),
        RebindRefusal::DeviceChanged
    );
    assert_eq!(ledger.usage().charged_bytes, 10);
    ledger.revoke_client(held.info().module, held.info().client);
    assert_eq!(
        ledger.rebind(&held).expect_err("revoked"),
        RebindRefusal::NotOpen
    );
}

#[test]
fn a_destroyed_device_ends_what_was_in_flight_on_it() {
    let ledger = ledger(policy(100));
    let held = claim(&ledger, 30);
    ledger
        .charge(&held, ChargeClass::Staging, 30)
        .expect("fits");
    let running = ledger.begin_submission(&held).expect("authorized");
    ledger.mark_device_lost(1);
    assert_eq!(ledger.generation_retired(1), 1);
    assert_eq!(
        running.state(),
        SubmissionState::Finished(SubmissionObservation::DeviceLost)
    );
    let counters = ledger.counters();
    assert_eq!(
        (counters.retired_in_flight, counters.completed_lost),
        (1, 1)
    );
    assert_eq!(ledger.usage().charged_bytes, 30);
    assert!(ledger
        .settle(&held, Settlement::clean(SettleOutcome::Lost))
        .is_ok());
}

#[test]
fn the_owner_reports_a_loss_once_and_every_refusal() {
    let log = Arc::new(EventLog::default());
    let core = OwnerCore::new(QuotaPolicy::Unavailable, log.clone());
    assert!(core.admit(&request(1, 1, 1)).is_err());
    core.device_unavailable(Unavailable::Driver(-13));
    let generation = core.device_bound(KEY);
    assert!(core.admit(&request(1, 1, 1)).is_err());
    core.set_policy(policy(100));
    let held = core.admit(&request(1, 1, 20)).expect("fits");
    assert!(core.device_lost(generation));
    assert!(!core.device_lost(generation));
    assert!(core.begin_submission(&held).is_err());
    core.submit_refused(SubmitRefusal::RecordAborted);
    core.device_fault(generation, DeviceFault::Driver(-13));
    core.device_rebind_refused(DeviceRebindRefusal::DeviceHeld { holders: 1 });
    assert_eq!(core.generation_retired(generation), 0);
    let no_device = AdmissionDenial::DeviceUnavailable.number();
    let no_quota = AdmissionDenial::HostQuotaUnavailable.number();
    let lost = AuthorizeRefusal::Lost { generation }.number();
    let held_back = DeviceRebindRefusal::DeviceHeld { holders: 1 }.number();
    assert_eq!(
        log.events(),
        vec![
            OwnerEvent::AdmissionDenied { reason: no_device },
            OwnerEvent::Unavailable {
                reason: 13,
                vk_result: -13
            },
            OwnerEvent::Phase {
                phase: OwnerPhase::Unavailable,
                generation: 0
            },
            OwnerEvent::Phase {
                phase: OwnerPhase::Open,
                generation
            },
            OwnerEvent::AdmissionDenied { reason: no_quota },
            OwnerEvent::Admitted {
                generation,
                reserved_bytes: 20
            },
            OwnerEvent::VulkanDeviceLost { generation },
            OwnerEvent::Phase {
                phase: OwnerPhase::DeviceLost,
                generation
            },
            OwnerEvent::AuthorizeRefused { reason: lost },
            OwnerEvent::SubmitRefused {
                reason: SubmitRefusal::RecordAborted.number()
            },
            OwnerEvent::DeviceFault {
                generation,
                fault: 4,
                vk_result: -13
            },
            OwnerEvent::DeviceRebindRefused { reason: held_back },
            OwnerEvent::GenerationRetired {
                generation,
                resolved: 0
            },
        ]
    );
}

#[test]
fn ordinary_work_emits_no_event_per_submission() {
    let log = Arc::new(EventLog::default());
    let core = OwnerCore::new(policy(100), log.clone());
    let generation = core.device_bound(KEY);
    let held = core.admit(&request(1, 1, 20)).expect("fits");
    let before = log.events().len();
    for value in 1..=3 {
        core.begin_submission(&held).expect("authorized");
        assert_eq!(core.resolve_through(generation, value), 1);
    }
    assert_eq!(log.events().len(), before);
}

#[test]
fn shutdown_reports_what_was_still_held_and_keeps_it_settleable() {
    let log = Arc::new(EventLog::default());
    let core = OwnerCore::new(policy(100), log.clone());
    core.device_bound(KEY);
    let held = core.admit(&request(1, 1, 20)).expect("fits");
    let _running = core.begin_submission(&held).expect("authorized");
    core.begin_shutdown();
    assert_eq!(
        core.admit(&request(1, 1, 1)).expect_err("closing"),
        AdmissionDenial::AdmissionClosed
    );
    let report = core.finish_shutdown(None);
    assert_eq!((report.open_claims, report.charged_bytes), (1, 20));
    assert_eq!((report.in_flight, report.device_retained), (0, false));
    assert!(!report.is_settled());
    assert_eq!(
        log.count(OwnerEvent::ShutdownOutstanding {
            open_claims: 1,
            retained_claims: 0,
            charged_bytes: 20
        }),
        1
    );
    assert_eq!(
        log.count(OwnerEvent::GenerationRetired {
            generation: 1,
            resolved: 1
        }),
        1
    );
    assert_eq!(
        core.authorize(&held),
        Err(AuthorizeRefusal::DeviceUnavailable)
    );
    assert!(core
        .settle(&held, Settlement::clean(SettleOutcome::Lost))
        .is_ok());
    assert_eq!(core.usage().charged_bytes, 0);
}

#[test]
fn shutdown_with_a_device_that_stays_alive_ends_no_submission() {
    let log = Arc::new(EventLog::default());
    let core = OwnerCore::new(policy(100), log.clone());
    core.device_bound(KEY);
    let held = core.admit(&request(1, 1, 20)).expect("fits");
    let running = core.begin_submission(&held).expect("authorized");
    core.begin_shutdown();
    let report = core.finish_shutdown(Some(2));
    assert!(report.device_retained && !report.is_settled());
    assert_eq!(report.in_flight, 1);
    assert_eq!(running.state(), SubmissionState::Pending);
    assert_eq!(
        log.count(OwnerEvent::DeviceRetainedAtClose {
            generation: 1,
            holders: 2
        }),
        1
    );
    assert_eq!(
        core.settle(&held, Settlement::clean(SettleOutcome::Lost)),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
}

#[test]
fn an_idle_shutdown_is_settled() {
    let core = OwnerCore::new(policy(100), Arc::new(EventLog::default()));
    core.device_bound(KEY);
    core.begin_shutdown();
    assert!(core.finish_shutdown(None).is_settled());
}

#[test]
fn the_descriptor_names_what_is_not_here() {
    for needle in [
        "\"slice\":1",
        "closure excludes wgpu, egui, eframe, winit",
        "GpuClientPort",
    ] {
        assert!(DESCRIPTOR.contains(needle), "{needle}");
    }
}
