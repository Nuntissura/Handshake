//! The owner on the machine's real Vulkan device: open, lease, submit, observe, settle and close.
//!
//! No window, surface or file. A machine without an acceptable device skips with the typed reason on one line
//! (`HSK_GPU_SKIP`), which is not a pass of these cases. Each test opens the process's one owner, so the tests take
//! turns, and each closes it again. Nothing here provokes a device loss: that needs hardware faults.

mod support;

use handshake_vulkan_renderer::{
    AdmissionDenial, AuthorizeRefusal, DeviceClass, DeviceRebindRefusal, GpuOwner, OwnerEvent,
    QuotaPolicy, RecordAborted, SettleOutcome, SettleRefusal, Settlement, SubmissionObservation,
    SubmissionRefusal, SubmissionState, SubmitRefusal, Unavailable, MAX_WAIT,
};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use support::{policy, request, EventLog};

static TURN: Mutex<()> = Mutex::new(());
const BOUND: Duration = MAX_WAIT;
const COMPLETED: SubmissionState = SubmissionState::Finished(SubmissionObservation::Completed);

fn turn() -> MutexGuard<'static, ()> {
    TURN.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The owner, or `None` after printing why this machine has no acceptable Vulkan device.
fn open(policy: QuotaPolicy) -> Option<(GpuOwner, Arc<EventLog>)> {
    let log = Arc::new(EventLog::default());
    match GpuOwner::open(policy, log.clone()) {
        Ok(owner) => Some((owner, log)),
        Err(
            reason @ (Unavailable::LoaderMissing
            | Unavailable::InstanceRefused
            | Unavailable::ApiBelowBaseline
            | Unavailable::NoPhysicalDevice
            | Unavailable::SoftwareDeviceRefused
            | Unavailable::LayeredDriverRefused
            | Unavailable::NoSuitableDevice
            | Unavailable::FeatureMissing),
        ) => {
            println!(
                "HSK_GPU_SKIP reason={} number={}",
                reason.key(),
                reason.number()
            );
            None
        }
        Err(reason) => panic!("a device exists but did not open: {}", reason.key()),
    }
}

#[test]
fn the_process_has_one_owner_on_a_hardware_device() {
    let _turn = turn();
    let Some((owner, log)) = open(policy(1 << 20)) else {
        return;
    };
    let identity = owner.identity();
    assert!(!matches!(
        identity.class,
        DeviceClass::Cpu | DeviceClass::Other
    ));
    assert_eq!(identity.api_version[0], 1);
    assert!(identity.api_version[1] >= 3);
    let device = owner.device().expect("bound");
    assert_eq!(
        (device.key, device.generation, device.lost),
        (identity.key, 1, false)
    );
    let refused = Arc::new(EventLog::default());
    assert_eq!(
        GpuOwner::open(policy(1), refused.clone()).err(),
        Some(Unavailable::OwnerAlreadyOpen)
    );
    assert_eq!(refused.events().len(), 1);
    assert!(!owner.memory_snapshot().expect("bound").heaps.is_empty());
    assert_eq!(owner.probe(), Ok(()));
    assert_eq!(owner.rebind_device(), Err(DeviceRebindRefusal::NotLost));
    let report = owner.close();
    assert!(report.is_settled(), "{report:?}");
    assert_eq!(owner.close(), report);
    assert_eq!(owner.rebind_device(), Err(DeviceRebindRefusal::Closed));
    assert!(owner.memory_snapshot().is_err());
    assert_eq!(
        log.count(OwnerEvent::GenerationRetired {
            generation: 1,
            resolved: 0
        }),
        1
    );
    // Closed is enough: the slot is free although the value still exists.
    let Some((again, _)) = open(policy(1)) else {
        panic!("the same device opens again")
    };
    assert!(again.close().is_settled());
}

#[test]
fn a_device_without_a_quota_admits_nothing() {
    let _turn = turn();
    let Some((owner, _)) = open(QuotaPolicy::Unavailable) else {
        return;
    };
    assert_eq!(
        owner.admit(&request(1, 1, 0)).expect_err("no quota"),
        AdmissionDenial::HostQuotaUnavailable
    );
    assert!(owner.close().is_settled());
}

#[test]
fn work_is_submitted_observed_and_settled_once() {
    let _turn = turn();
    let Some((owner, log)) = open(policy(1 << 20)) else {
        return;
    };
    let claim = owner.admit(&request(1, 1, 4096)).expect("fits");
    let events = log.events().len();
    let mut submissions = Vec::new();
    for expected in 1..=3 {
        let submission = owner
            .submit(&claim, |recording| {
                assert_ne!(recording.command_buffer(), 0);
                assert_eq!(recording.generation(), 1);
                Ok(())
            })
            .expect("submits");
        assert_eq!(
            (submission.generation(), submission.sequence()),
            (1, expected)
        );
        assert_eq!(submission.claim_info(), claim.info());
        submissions.push(submission);
    }
    let last = submissions.last().expect("three");
    assert_eq!(owner.wait(last, BOUND), Ok(COMPLETED));
    for submission in &submissions {
        assert_eq!(owner.observe(submission), Ok(COMPLETED));
    }
    assert_eq!(owner.usage().in_flight, 0);
    assert_eq!(log.events().len(), events, "ordinary work emits no event");
    let clean = Settlement::clean(SettleOutcome::Completed);
    assert_eq!(
        owner.settle(&claim, clean).expect("settles").released_bytes,
        4096
    );
    assert_eq!(
        owner.settle(&claim, clean),
        Err(SettleRefusal::DoubleRelease)
    );
    assert!(matches!(
        owner.submit(&claim, |_| Ok(())),
        Err(SubmitRefusal::NotAuthorized(AuthorizeRefusal::Settled))
    ));
    assert!(owner.close().is_settled());
    // What was observed stays readable after the device is gone.
    assert_eq!(owner.observe(last), Ok(COMPLETED));
}

#[test]
fn a_recording_that_ends_without_work_submits_nothing() {
    let _turn = turn();
    let Some((owner, _)) = open(policy(1 << 20)) else {
        return;
    };
    let claim = owner.admit(&request(1, 1, 16)).expect("fits");
    assert_eq!(
        owner.submit(&claim, |_| Err(RecordAborted)).err(),
        Some(SubmitRefusal::RecordAborted)
    );
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        owner.submit(&claim, |_| -> Result<(), RecordAborted> {
            panic!("recording failed")
        })
    }));
    assert!(panicked.is_err());
    // Revoked while recording: the second authorization refuses it.
    let revoked = owner.submit(&claim, |_| {
        owner.revoke_client(claim.info().module, claim.info().client);
        Ok(())
    });
    assert_eq!(
        revoked.err(),
        Some(SubmitRefusal::NotAuthorized(AuthorizeRefusal::Revoked))
    );
    assert_eq!(
        (owner.usage().in_flight, owner.counters().submitted),
        (0, 0)
    );
    assert!(owner
        .settle(&claim, Settlement::clean(SettleOutcome::Canceled))
        .is_ok());
    // The owner is usable after all of it.
    let next = owner.admit(&request(1, 2, 16)).expect("fits");
    let submission = owner.submit(&next, |_| Ok(())).expect("submits");
    assert_eq!(owner.wait(&submission, BOUND), Ok(COMPLETED));
    assert!(owner
        .settle(&next, Settlement::clean(SettleOutcome::Completed))
        .is_ok());
    assert!(owner.close().is_settled());
}

#[test]
fn concurrent_clients_share_one_queue_in_fence_order() {
    let _turn = turn();
    let Some((owner, _)) = open(policy(1 << 20)) else {
        return;
    };
    let mut values = std::thread::scope(|scope| {
        let clients: Vec<_> = (1..=4u64)
            .map(|client| {
                let owner = &owner;
                scope.spawn(move || {
                    let claim = owner.admit(&request(1, client, 16)).expect("fits");
                    let mut values = Vec::new();
                    for _ in 0..8 {
                        let submission = owner.submit(&claim, |_| Ok(())).expect("submits");
                        values.push(submission.sequence());
                        assert_eq!(owner.wait(&submission, BOUND), Ok(COMPLETED));
                    }
                    owner
                        .settle(&claim, Settlement::clean(SettleOutcome::Completed))
                        .expect("settles");
                    values
                })
            })
            .collect();
        clients
            .into_iter()
            .flat_map(|client| client.join().expect("no panic"))
            .collect::<Vec<u64>>()
    });
    values.sort_unstable();
    assert_eq!(values, (1..=32).collect::<Vec<u64>>());
    assert!(owner.close().is_settled());
}

#[test]
fn a_leased_device_outlives_the_close_of_its_owner() {
    let _turn = turn();
    let Some((owner, log)) = open(policy(1 << 20)) else {
        return;
    };
    let claim = owner.admit(&request(1, 1, 16)).expect("fits");
    let lease = owner.lease(&claim).expect("authorized");
    assert_eq!((lease.generation(), lease.claim()), (1, claim.id()));
    let handles = lease.handles();
    assert!(handles.get_instance_proc_addr != 0 && handles.instance != 0);
    assert!(handles.physical_device != 0 && handles.device != 0);
    assert!(handles.features.contains(&"dynamicRendering"));
    let clean = Settlement::clean(SettleOutcome::Canceled);
    assert_eq!(
        owner.settle(&claim, clean),
        Err(SettleRefusal::LeaseOutstanding { leases: 1 })
    );
    let report = owner.close();
    assert!(report.device_retained && !report.is_settled());
    assert_eq!(report.leases, 1);
    assert_eq!(
        log.count(OwnerEvent::DeviceRetainedAtClose {
            generation: 1,
            holders: 1
        }),
        1
    );
    // The leased device is still the process's one device.
    assert_eq!(
        GpuOwner::open(policy(1), Arc::new(EventLog::default())).err(),
        Some(Unavailable::OwnerAlreadyOpen)
    );
    drop(lease);
    assert!(owner.settle(&claim, clean).is_ok());
    let Some((again, _)) = open(policy(1)) else {
        panic!("the same device opens again")
    };
    assert!(again.close().is_settled());
}

#[test]
fn another_owners_submission_is_unknown() {
    let _turn = turn();
    let Some((first, _)) = open(policy(1 << 20)) else {
        return;
    };
    let claim = first.admit(&request(1, 1, 16)).expect("fits");
    let submission = first.submit(&claim, |_| Ok(())).expect("submits");
    // Closing drains the queue, so the submission ends as completed and not as lost.
    let report = first.close();
    assert_eq!((report.in_flight, report.open_claims), (0, 1));
    assert_eq!(submission.state(), COMPLETED);
    assert!(first
        .settle(&claim, Settlement::clean(SettleOutcome::Completed))
        .is_ok());
    let Some((second, _)) = open(policy(1 << 20)) else {
        panic!("the same device opens again")
    };
    assert_eq!(second.observe(&submission), Err(SubmissionRefusal::Unknown));
    assert_eq!(second.lease(&claim).err(), Some(AuthorizeRefusal::Unknown));
    assert!(second.close().is_settled());
}
