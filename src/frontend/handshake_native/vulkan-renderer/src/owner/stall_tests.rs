//! MT-203 E3-1 and E3-2 on the machine's real device: a submission held inside the driver call keeps nothing but other
//! submissions waiting, and a diagnostics sink that calls back into the owner never meets a lock of the owner.

use super::fault_tests::support::gpu_env::{
    in_own_process, is_required, ran_in_its_own_process, REQUIRE_GPU,
};
use super::fault_tests::support::{policy, request};
use super::fault_tests::{attach, rig, LATE, QUOTA};
use super::{GpuClient, GpuOwner, MAX_WAIT};
use crate::vulkan::VulkanGeneration;
use crate::{
    AuthorizeRefusal, ClientId, DeviceFate, ModuleId, OwnerDiagnostics, OwnerEvent, SettleOutcome,
    Settlement, SubmissionState, SubmitRefusal, Unavailable,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Weak};
use std::time::Instant;

#[test]
fn a_submit_held_in_the_driver_keeps_poll_observe_leases_and_close_free() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(1 << 20);
    let done = rig.submit_empty(&claim);
    rig.device(|device| device.wait_really(1));
    let other = attach(&rig.owner, 2);
    let other_claim = other.admit(&request(1 << 20)).expect("fits");
    // Held outside the slot lock: the submission takes that lock before it reaches the driver call.
    let device = rig.device(Arc::clone);
    device.faults().stall_submits();
    std::thread::scope(|scope| {
        let held = scope.spawn(|| other.submit(&other_claim, |_| Ok(())));
        assert!(
            device.faults().is_stalled(),
            "the submission reached the driver call"
        );
        let start = Instant::now();
        assert_eq!(rig.owner.poll(), Ok(()));
        assert!(matches!(
            rig.client.observe(&done),
            Ok(SubmissionState::Finished(_))
        ));
        let lease = rig.client.lease(&claim).expect("authorized");
        drop(lease);
        let quick = start.elapsed();
        assert!(quick < LATE, "{quick:?}");
        // The close drains for at most MAX_WAIT: the held submission is counted, so the device is not destroyed under it.
        let report = rig.owner.close();
        let closing = start.elapsed();
        assert!(closing < MAX_WAIT + LATE, "{closing:?}");
        assert!(
            matches!(
                report.device,
                DeviceFate::Held { .. } | DeviceFate::Unconfirmed
            ),
            "{report:?}"
        );
        device.faults().release_submits();
        // The reservation came before the seal, so the queue takes the work and it ends as any other.
        let submission = held.join().expect("ends").expect("the queue took it");
        device.wait_really(submission.sequence());
    });
    drop(device);
    assert_eq!(rig.owner.poll().err(), Some(crate::DeviceFault::NoDevice));
    // Nothing holds the device or runs on it any more: the next look destroys it, once.
    drop(other);
    let _ = rig.owner.poll();
    assert_eq!(
        rig.log.count_like(OwnerEvent::RetainedDeviceDestroyed {
            generation: 1,
            resolved: 0,
        }),
        1
    );
}

/// Calls the owner back from inside every event, as a host's diagnostics adapter may.
#[derive(Default)]
struct CallsBack {
    client: OnceLock<Weak<GpuClient>>,
    calls: AtomicU64,
    events: AtomicU64,
}

impl OwnerDiagnostics for CallsBack {
    fn record(&self, _event: OwnerEvent) {
        self.events.fetch_add(1, Ordering::SeqCst);
        if let Some(client) = self.client.get().and_then(Weak::upgrade) {
            // Each takes the slot lock.
            let _ = client.poll();
            let _ = client.device();
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// Looks at the bound generation's lane whenever an authorization refusal is delivered.
#[derive(Default)]
struct LaneWatch {
    device: OnceLock<Weak<VulkanGeneration>>,
    refusals: AtomicU64,
    under_the_lane: AtomicU64,
}

impl OwnerDiagnostics for LaneWatch {
    fn record(&self, event: OwnerEvent) {
        if !matches!(event, OwnerEvent::AuthorizeRefused { .. }) {
            return;
        }
        self.refusals.fetch_add(1, Ordering::SeqCst);
        let device = self.device.get().and_then(Weak::upgrade);
        if device.is_some_and(|device| !device.lane_is_free()) {
            self.under_the_lane.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn open_owner(sink: Arc<dyn OwnerDiagnostics>) -> Option<GpuOwner> {
    match GpuOwner::open(policy(QUOTA), sink) {
        Ok(owner) => Some(owner),
        Err(reason @ (Unavailable::OwnerAlreadyOpen | Unavailable::DeviceCreationRefused)) => {
            panic!("{}", reason.key())
        }
        Err(reason) => {
            assert!(
                !is_required(std::env::var_os(REQUIRE_GPU).as_deref()),
                "no acceptable device ({})",
                reason.key()
            );
            println!("HSK_GPU_SKIP reason={}", reason.key());
            None
        }
    }
}

/// MT-203 E3-2 for the one section that holds the lane: a refusal met while the lane is held (the claim is authorized
/// again under it) reaches the sink only after the lane is let go.
#[test]
fn an_event_met_while_the_lane_is_held_is_delivered_after_it() {
    in_own_process!();
    let sink = Arc::new(LaneWatch::default());
    let Some(owner) = open_owner(sink.clone()) else {
        return;
    };
    let device = Arc::downgrade(
        &owner
            .shared
            .lock_slot()
            .live
            .as_ref()
            .expect("bound")
            .vulkan,
    );
    assert!(sink.device.set(device).is_ok());
    let client = attach(&owner, 1);
    let claim = client.admit(&request(1 << 20)).expect("fits");
    // The host revokes the client while it records: the authorization under the lane refuses the submission.
    let refused = client.submit(&claim, |_| {
        assert_eq!(owner.revoke_client(ModuleId(1), ClientId(1)), 1);
        Ok(())
    });
    assert!(
        matches!(refused, Err(SubmitRefusal::NotAuthorized(_))),
        "{refused:?}"
    );
    assert_eq!(
        sink.refusals.load(Ordering::SeqCst),
        1,
        "the refusal was told"
    );
    assert_eq!(
        sink.under_the_lane.load(Ordering::SeqCst),
        0,
        "told while the lane was held"
    );
    client
        .settle(&claim, Settlement::clean(SettleOutcome::NoEffect))
        .expect("settles");
    assert!(owner.close().is_settled());
}

#[test]
fn a_sink_that_calls_back_into_the_owner_never_meets_its_locks() {
    in_own_process!();
    let sink = Arc::new(CallsBack::default());
    let owner = match GpuOwner::open(policy(QUOTA), sink.clone()) {
        Ok(owner) => owner,
        Err(reason @ (Unavailable::OwnerAlreadyOpen | Unavailable::DeviceCreationRefused)) => {
            panic!("{}", reason.key())
        }
        Err(reason) => {
            assert!(
                !is_required(std::env::var_os(REQUIRE_GPU).as_deref()),
                "no acceptable device ({})",
                reason.key()
            );
            println!("HSK_GPU_SKIP reason={}", reason.key());
            return;
        }
    };
    let client = Arc::new(attach(&owner, 1));
    assert!(sink.client.set(Arc::downgrade(&client)).is_ok());
    let claim = client.admit(&request(1 << 20)).expect("fits");
    let settled = Settlement::clean(SettleOutcome::NoEffect);
    client.settle(&claim, settled).expect("settles");
    let calls = sink.calls.load(Ordering::SeqCst);
    // Refused under the slot lock: the event waits for its release, and the sink's calls then take that lock.
    assert_eq!(client.lease(&claim).err(), Some(AuthorizeRefusal::Settled));
    assert!(
        sink.calls.load(Ordering::SeqCst) > calls,
        "the sink was called"
    );
    // A submit refused under the slot lock, and a close that reports under the close lock.
    assert!(client.submit(&claim, |_| Ok(())).is_err());
    let report = owner.close();
    assert!(report.is_settled(), "{report:?}");
    assert!(sink.events.load(Ordering::SeqCst) >= sink.calls.load(Ordering::SeqCst));
    drop(client);
}
