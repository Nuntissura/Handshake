//! Shutdown without a GPU: it says truthfully what remained, and what remained stays settleable.

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AuthorizeRefusal, ByteQuantity, ClientId, DeviceFate, LedgerClient, ModuleId,
    OwnerCore, OwnerEvent, OwnerPhase, SettleOutcome, SettleRefusal, Settlement, ShutdownReport,
    SubmissionObservation, SubmissionState,
};
use std::sync::Arc;
use support::{claim_subject, policy, request, EventLog, KEY};

const DEVICE_LOST: SubmissionState = SubmissionState::Finished(SubmissionObservation::DeviceLost);

fn lost() -> Settlement {
    Settlement::clean(SettleOutcome::Lost)
}

/// An owner with a bound device and one attached client.
fn opened() -> (OwnerCore, LedgerClient, Arc<EventLog>) {
    let log = Arc::new(EventLog::default());
    let core = OwnerCore::new(policy(100), log.clone());
    let client = core.attach(ModuleId(1), ClientId(1)).expect("free");
    core.device_bound(KEY);
    (core, client, log)
}

#[test]
fn shutdown_reports_what_was_still_held_and_keeps_it_settleable() {
    let (core, client, log) = opened();
    let held = client.admit(&request(20)).expect("fits");
    let running = client.begin_submission(&held).expect("authorized");
    core.begin_shutdown();
    assert_eq!(
        client.admit(&request(1)).expect_err("closing"),
        AdmissionDenial::AdmissionClosed
    );
    assert!(core.attach(ModuleId(2), ClientId(2)).is_err());
    let report = core.finish_shutdown(DeviceFate::Destroyed, 0);
    assert_eq!((report.open_claims, report.charged_bytes), (1, 20));
    assert_eq!(
        (report.in_flight, report.device),
        (0, DeviceFate::Destroyed)
    );
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
    assert_eq!(running.state(), DEVICE_LOST);
    assert_eq!(
        client.authorize(&held),
        Err(AuthorizeRefusal::DeviceUnavailable)
    );
    assert!(client.settle(&held, lost()).is_ok());
    assert_eq!(core.ledger().usage().charged_bytes, 0);
}

#[test]
fn shutdown_with_a_device_that_stays_alive_ends_no_submission() {
    for (fate, holders) in [
        (DeviceFate::Held { holders: 2 }, 2),
        (DeviceFate::Unconfirmed, 0),
    ] {
        let (core, client, log) = opened();
        let held = client.admit(&request(20)).expect("fits");
        let running = client.begin_submission(&held).expect("authorized");
        core.begin_shutdown();
        let report = core.finish_shutdown(fate, 0);
        assert_eq!((report.device, report.in_flight), (fate, 1));
        assert!(!report.is_settled());
        assert_eq!(running.state(), SubmissionState::Pending);
        assert_eq!(
            log.count(OwnerEvent::DeviceRetainedAtClose {
                generation: 1,
                holders
            }),
            1
        );
        // Neither the end nor the loss of a device that is kept.
        for never in [
            OwnerEvent::GenerationRetired {
                generation: 1,
                resolved: 0,
            },
            OwnerEvent::DeviceLeaked { generation: 1 },
        ] {
            assert_eq!(log.count_like(never), 0);
        }
        assert_eq!(
            client.settle(&held, lost()),
            Err(SettleRefusal::InFlight { in_flight: 1 })
        );
    }
}

/// What is kept can still end either way, and is told once.
#[test]
fn a_device_kept_at_a_close_can_complete_its_work_or_be_lost() {
    let (core, client, log) = opened();
    let held = client.admit(&request(20)).expect("fits");
    let first = client.begin_submission(&held).expect("authorized");
    let second = client.begin_submission(&held).expect("authorized");
    let memory = client.admit(&request(5)).expect("fits");
    drop(client.hold(&memory).expect("authorized"));
    core.begin_shutdown();
    core.finish_shutdown(DeviceFate::Held { holders: 1 }, 0);
    assert_eq!(core.ledger().resolve_through(1, 1), 1);
    assert!(core.device_lost(1));
    assert!(!core.device_lost(1));
    assert_eq!(log.count(OwnerEvent::VulkanDeviceLost { generation: 1 }), 1);
    // The owner is closed and stays so.
    assert_eq!(
        log.count(OwnerEvent::Phase {
            phase: OwnerPhase::DeviceLost,
            generation: 1
        }),
        0
    );
    assert_eq!(core.ledger().resolve_lost(1), 1);
    assert_eq!(
        [first.state(), second.state()],
        [
            SubmissionState::Finished(SubmissionObservation::Completed),
            DEVICE_LOST
        ]
    );
    // Memory work on a device that is lost did not complete.
    assert!(matches!(
        client.settle(&memory, Settlement::clean(SettleOutcome::Completed)),
        Err(SettleRefusal::OutcomeContradicted { .. })
    ));
    assert!(client.settle(&memory, lost()).is_ok());
    assert!(client.settle(&held, lost()).is_ok());
}

#[test]
fn a_device_given_up_reports_the_claims_nobody_holds_as_leaked() {
    let (core, client, log) = opened();
    let kept = client.admit(&request(1)).expect("fits");
    let dropped = client.admit(&request(20)).expect("fits");
    let leaked = OwnerEvent::ClaimLeaked {
        subject: claim_subject(dropped.info()),
        leaked_bytes: 20,
    };
    drop(client.begin_submission(&dropped).expect("authorized"));
    drop(dropped);
    core.begin_shutdown();
    core.finish_shutdown(DeviceFate::Unconfirmed, 0);
    assert_eq!(log.count(leaked), 0, "somebody may still look");
    core.retained_device_ended(1, false);
    assert_eq!((log.count(leaked), log.count_like(leaked)), (1, 1));
    assert_eq!(log.count(OwnerEvent::DeviceLeaked { generation: 1 }), 1);
    assert!(client
        .settle(&kept, Settlement::clean(SettleOutcome::NoEffect))
        .is_ok());
}

#[test]
fn a_close_during_the_creation_of_a_device_says_so() {
    let (core, _client, log) = opened();
    core.device_lost(1);
    core.begin_shutdown();
    let report = core.finish_shutdown(DeviceFate::Creating, 0);
    assert_eq!(report.device, DeviceFate::Creating);
    assert!(!report.is_settled());
    let creating = OwnerEvent::DeviceCreatingAtClose;
    assert_eq!(log.count(creating), 1);
    assert!(creating.is_degraded());
    assert_eq!(creating.subject(), None);
}

/// Each number alone keeps a shutdown from being settled, and so does each device that is not gone.
#[test]
fn every_term_alone_keeps_a_shutdown_unsettled() {
    let settled = ShutdownReport {
        open_claims: 0,
        retained_claims: 0,
        charged_bytes: 0,
        in_flight: 0,
        leases: 0,
        device: DeviceFate::Destroyed,
        lost_devices_held: 0,
    };
    assert!(settled.is_settled());
    let none = ShutdownReport {
        device: DeviceFate::NoDevice,
        ..settled
    };
    assert!(none.is_settled());
    type Change = fn(&mut ShutdownReport);
    let changes: [(&str, Change); 9] = [
        ("open_claims", |report| report.open_claims = 1),
        ("retained_claims", |report| report.retained_claims = 1),
        ("charged_bytes", |report| report.charged_bytes = 1),
        ("in_flight", |report| report.in_flight = 1),
        ("leases", |report| report.leases = 1),
        ("lost_devices_held", |report| report.lost_devices_held = 1),
        ("held", |report| {
            report.device = DeviceFate::Held { holders: 1 };
        }),
        ("unconfirmed", |report| {
            report.device = DeviceFate::Unconfirmed;
        }),
        ("creating", |report| report.device = DeviceFate::Creating),
    ];
    for (term, change) in changes {
        let mut report = settled;
        change(&mut report);
        assert!(!report.is_settled(), "{term}");
    }
}

/// And each of them is what the owner counted, not a default.
#[test]
fn a_shutdown_reports_each_thing_that_was_left() {
    let (core, client, _log) = opened();
    let open = client.admit(&request(20)).expect("fits");
    let hold = client.hold(&open).expect("authorized");
    let _running = client.begin_submission(&open).expect("authorized");
    let retained = client.admit(&request(10)).expect("fits");
    let keeps = Settlement {
        retained: ByteQuantity::Measured(4),
        ..Settlement::clean(SettleOutcome::Canceled)
    };
    client.settle(&retained, keeps).expect("retains");
    core.begin_shutdown();
    let fate = DeviceFate::Held { holders: 1 };
    assert_eq!(
        core.finish_shutdown(fate, 3),
        ShutdownReport {
            open_claims: 1,
            retained_claims: 1,
            charged_bytes: 24,
            in_flight: 1,
            leases: 1,
            device: fate,
            lost_devices_held: 3,
        }
    );
    drop(hold);
}
