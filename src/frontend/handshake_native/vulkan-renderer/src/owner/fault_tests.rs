//! The owner on the machine's real Vulkan device under faults a healthy machine does not produce.
//!
//! The device, its queue and its work are real. Two things are the test's: single answers of the driver, replaced at
//! the seam in `vulkan/fault.rs` (an injected loss is lost for every call from there on), and a gate that keeps
//! submitted work really pending until the test opens it. Each case runs in a process of its own, and without an
//! acceptable device it skips or fails as in `tests/gpu_owner.rs`.

#[path = "../../tests/support/mod.rs"]
pub(super) mod support;

use super::{GpuClient, GpuOwner, MAX_HELD_LOST_DEVICES, MAX_WAIT};
use crate::vulkan::fault::{refuse_open, Call};
use crate::vulkan::VulkanGeneration;
use crate::{
    AdmissionDenial, AuthorizeRefusal, Claim, ClientId, DeviceFate, DeviceFault,
    DeviceRebindRefusal, ModuleId, OwnerEvent, QuotaPolicy, RebindRefusal, SettleOutcome,
    SettleRefusal, Settlement, Submission, SubmissionObservation, SubmissionState, SubmitRefusal,
    Unavailable,
};
use ash::vk;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use support::gpu_env::{in_own_process, open, ran_in_its_own_process};
use support::gpu_work::Worker;
use support::{claim_subject, policy, request, EventLog};

pub(super) const COMPLETED: SubmissionState =
    SubmissionState::Finished(SubmissionObservation::Completed);
const DEVICE_LOST: SubmissionState = SubmissionState::Finished(SubmissionObservation::DeviceLost);
const LOST: DeviceFault = DeviceFault::Lost { generation: 1 };
pub(super) const QUOTA: u64 = 1 << 30;
pub(super) const SHORT: Duration = Duration::from_millis(50);

pub(super) struct Rig {
    pub(super) client: GpuClient,
    pub(super) owner: GpuOwner,
    pub(super) log: Arc<EventLog>,
}

impl Drop for Rig {
    /// A failing case may leave its gate closed, and a process that ends with held work may hang.
    fn drop(&mut self) {
        if let Some(live) = self.owner.shared.lock_slot().live.as_ref() {
            live.vulkan.faults().stall(false);
            live.vulkan.open_gate();
        }
    }
}

pub(super) fn rig_with(policy: QuotaPolicy) -> Option<Rig> {
    let (owner, log) = open(policy)?;
    let client = attach(&owner, 1);
    Some(Rig { client, owner, log })
}

pub(super) fn rig() -> Option<Rig> {
    rig_with(policy(QUOTA))
}

fn attach(owner: &GpuOwner, module: u32) -> GpuClient {
    owner
        .attach_client(ModuleId(module), ClientId(1))
        .expect("free")
}

impl Rig {
    pub(super) fn claim(&self, bytes: u64) -> Claim {
        self.client.admit(&request(bytes)).expect("fits")
    }

    pub(super) fn worker(&self) -> Worker {
        Worker::new(&self.client, self.claim(Worker::RESERVE))
    }

    /// The bound device, with the slot locked.
    pub(super) fn device<T>(&self, use_it: impl FnOnce(&Arc<VulkanGeneration>) -> T) -> T {
        let slot = self.owner.shared.lock_slot();
        use_it(&slot.live.as_ref().expect("bound").vulkan)
    }

    pub(super) fn submit(&self, claim: &Claim, worker: &Worker, value: u32) -> Submission {
        self.client
            .submit(claim, |recording| {
                worker.work.record(recording, value);
                Ok(())
            })
            .expect("submits")
    }

    /// The bound device is lost and the owner has seen it.
    fn lose(&self) {
        let generation = self.owner.device().expect("bound").generation;
        self.device(|device| device.faults().lose_at(Call::Counter));
        assert_eq!(self.owner.poll(), Err(DeviceFault::Lost { generation }));
    }

    pub(super) fn settle(
        &self,
        claim: &Claim,
        outcome: SettleOutcome,
    ) -> Result<u64, SettleRefusal> {
        self.client
            .settle(claim, Settlement::clean(outcome))
            .map(|receipt| receipt.released_bytes)
    }
}

#[test]
fn a_loss_met_by_submit_stays_in_flight_until_the_device_says_its_work_ended() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let claim = rig.claim(64);
    rig.device(|device| {
        device.faults().stall(true);
        device.faults().lose_at(Call::Submit);
    });
    assert_eq!(
        rig.client.submit(&claim, |_| Ok(())).err(),
        Some(SubmitRefusal::Device(LOST))
    );
    assert!(rig.owner.device().expect("bound").lost);
    // The work may have reached the queue, and the device does not say that it ended.
    assert_eq!(rig.owner.usage().in_flight, 1);
    assert_eq!(
        rig.settle(&claim, SettleOutcome::Lost),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    assert_eq!(rig.owner.poll(), Err(LOST));
    assert_eq!(rig.owner.probe(SHORT), Err(LOST));
    assert_eq!(
        rig.owner.rebind_device(),
        Err(DeviceRebindRefusal::NotQuiescent)
    );
    assert_eq!(rig.owner.device().expect("still bound").generation, 1);
    assert_eq!(rig.owner.usage().in_flight, 1);

    rig.device(|device| device.faults().stall(false));
    assert_eq!(rig.owner.poll(), Err(LOST));
    assert_eq!(rig.owner.usage().in_flight, 0);
    for outcome in [SettleOutcome::Completed, SettleOutcome::NoEffect] {
        assert!(matches!(
            rig.settle(&claim, outcome),
            Err(SettleRefusal::OutcomeContradicted { .. })
        ));
    }
    assert_eq!(rig.settle(&claim, SettleOutcome::Lost), Ok(64));
    assert_eq!(
        rig.log
            .count(OwnerEvent::VulkanDeviceLost { generation: 1 }),
        1
    );
    assert_eq!(rig.owner.rebind_device(), Ok(2));
}

#[test]
fn a_loss_met_by_observe_wait_probe_or_poll_is_reported_once_and_ends_the_work() {
    in_own_process!();
    type Meet = fn(&Rig, &Submission);
    let ways: [(Call, Meet); 4] = [
        (Call::Counter, |rig, submission| {
            assert_eq!(rig.client.observe(submission), Ok(DEVICE_LOST));
        }),
        (Call::Wait, |rig, submission| {
            assert_eq!(rig.client.wait(submission, SHORT), Ok(DEVICE_LOST));
        }),
        (Call::Wait, |rig, _| {
            assert_eq!(rig.owner.probe(SHORT), Err(LOST));
        }),
        (Call::Counter, |rig, _| {
            assert_eq!(rig.client.poll(), Err(LOST));
        }),
    ];
    for (call, meet) in ways {
        let Some(rig) = rig() else { return };
        let worker = rig.worker();
        rig.device(|device| device.close_gate());
        let submission = rig.submit(&worker.claim, &worker, 1);
        rig.device(|device| device.faults().lose_at(call));
        meet(&rig, &submission);
        assert_eq!(submission.state(), DEVICE_LOST);
        assert_eq!(rig.owner.usage().in_flight, 0);
        assert_eq!(rig.owner.poll(), Err(LOST));
        assert_eq!(
            rig.log
                .count(OwnerEvent::VulkanDeviceLost { generation: 1 }),
            1
        );
        // The injected loss ended nothing on the real device: let it run before its buffers go.
        rig.device(|device| device.open_gate());
        rig.device(|device| device.wait_really(1));
        drop(worker.let_go(&rig.client));
    }
}

#[test]
fn a_lost_device_is_replaced_and_the_new_one_does_real_work() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let key = rig.owner.identity().key;
    let first = rig.worker();
    let done = rig.submit(&first.claim, &first, 3);
    assert_eq!(rig.client.wait(&done, MAX_WAIT), Ok(COMPLETED));
    rig.lose();
    assert_eq!(
        rig.client.admit(&request(1)).expect_err("lost"),
        AdmissionDenial::DeviceLost { generation: 1 }
    );
    let claim = first.let_go(&rig.client);
    assert_eq!(rig.owner.rebind_device(), Ok(2));
    assert_eq!(
        rig.log.count(OwnerEvent::GenerationRetired {
            generation: 1,
            resolved: 0
        }),
        1
    );
    let bound = rig.owner.device().expect("bound");
    assert_eq!((bound.key, bound.generation, bound.lost), (key, 2, false));
    assert_eq!(
        rig.client.authorize(&claim),
        Err(AuthorizeRefusal::Lost { generation: 1 })
    );
    let successor = rig.client.rebind_claim(&claim).expect("moves");
    assert_eq!(rig.owner.usage().charged_bytes, Worker::RESERVE);
    assert_eq!(
        rig.settle(&claim, SettleOutcome::Lost),
        Err(SettleRefusal::LateRelease)
    );
    let second = Worker::new(&rig.client, successor);
    let again = rig.submit(&second.claim, &second, 9);
    assert_eq!((again.generation(), again.sequence()), (2, 1));
    assert_eq!(rig.client.wait(&again, MAX_WAIT), Ok(COMPLETED));
    assert!(second.work.holds(9));
    second.settle(&rig.client);
    assert!(rig.owner.close().is_settled());
}

#[test]
fn one_unreturned_lease_does_not_block_the_recovery_of_the_others() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let idle = attach(&rig.owner, 2);
    let idle_claim = idle.admit(&request(16)).expect("fits");
    let idle_lease = idle.lease(&idle_claim).expect("authorized");
    assert!(idle_lease.handles().is_ok());
    let claim = rig.claim(Worker::RESERVE);
    rig.lose();
    let told = Some(AuthorizeRefusal::Lost { generation: 1 });
    assert_eq!(idle_lease.handles().err(), told);

    assert_eq!(rig.owner.rebind_device(), Ok(2));
    let retired = OwnerEvent::GenerationRetired {
        generation: 1,
        resolved: 0,
    };
    assert_eq!(rig.log.count_like(retired), 0);
    assert_eq!(
        rig.log.count(OwnerEvent::LostDeviceSetAside {
            generation: 1,
            holders: 1
        }),
        1
    );
    let successor = rig.client.rebind_claim(&claim).expect("moves");
    let worker = Worker::new(&rig.client, successor);
    let submission = rig.submit(&worker.claim, &worker, 4);
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert!(worker.work.holds(4));
    worker.settle(&rig.client);
    // However often the device is lost, that one lease uses up no more than its own device.
    for generation in 3..=3 + MAX_HELD_LOST_DEVICES as u64 {
        rig.lose();
        assert_eq!(rig.owner.rebind_device(), Ok(generation));
    }

    // The holder is still told, keeps its own charge, and can clean up.
    assert_eq!(idle_lease.handles().err(), told);
    assert_eq!(
        idle.rebind_claim(&idle_claim).expect_err("holds a lease"),
        RebindRefusal::LeaseOutstanding { leases: 1 }
    );
    assert_eq!(rig.owner.usage().charged_bytes, 16);
    assert_eq!(idle_lease.handles_to_release().generation, 1);
    let ended = OwnerEvent::RetainedDeviceDestroyed {
        generation: 1,
        resolved: 0,
    };
    assert_eq!(rig.log.count(ended), 0);
    drop(idle_lease);
    assert_eq!(rig.log.count(ended), 1);
    let moved = idle.rebind_claim(&idle_claim).expect("moves now");
    assert!(idle
        .settle(&moved, Settlement::clean(SettleOutcome::Lost))
        .is_ok());
    assert!(rig.owner.close().is_settled());
}

#[test]
fn a_held_device_refuses_the_rebind_only_beyond_the_bound_on_set_aside_devices() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let mut held = Vec::new();
    for generation in 1..=MAX_HELD_LOST_DEVICES as u64 {
        let claim = rig.claim(1);
        let lease = rig.client.lease(&claim).expect("authorized");
        assert_eq!(lease.generation(), generation);
        held.push((lease, claim));
        rig.lose();
        assert_eq!(rig.owner.rebind_device(), Ok(generation + 1));
    }
    let claim = rig.claim(1);
    let last = rig.client.lease(&claim).expect("authorized");
    rig.lose();
    let refused = DeviceRebindRefusal::DeviceHeld { holders: 1 };
    assert_eq!(rig.owner.rebind_device(), Err(refused));
    assert!(rig.owner.device().expect("still bound").lost);
    assert_eq!(
        rig.log.count(OwnerEvent::DeviceRebindRefused {
            reason: refused.number()
        }),
        1
    );
    // One of them comes back, and the bound has room again.
    drop(held.remove(0));
    let next = MAX_HELD_LOST_DEVICES as u64 + 2;
    assert_eq!(rig.owner.rebind_device(), Ok(next));
    // A close with lost devices still held says so.
    let report = rig.owner.close();
    assert_eq!(report.lost_devices_held, MAX_HELD_LOST_DEVICES as u64);
    assert_eq!(report.device, DeviceFate::Destroyed);
    assert!(!report.is_settled());
    drop(last);
    drop(held);
}

#[test]
fn a_submission_the_queue_refuses_has_no_effect_and_is_no_loss() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let claim = rig.claim(8);
    rig.device(|device| {
        device
            .faults()
            .refuse_submit(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)
    });
    assert_eq!(
        rig.client.submit(&claim, |_| Ok(())).err(),
        Some(SubmitRefusal::Device(DeviceFault::OutOfMemory))
    );
    assert!(!rig.owner.device().expect("bound").lost);
    assert_eq!(rig.owner.usage().in_flight, 0);
    assert_eq!(
        rig.log.count(OwnerEvent::DeviceFault {
            generation: 1,
            fault: DeviceFault::OutOfMemory.number(),
            vk_result: 0
        }),
        1
    );
    assert!(matches!(
        rig.settle(&claim, SettleOutcome::Completed),
        Err(SettleRefusal::OutcomeContradicted { .. })
    ));
    assert_eq!(rig.settle(&claim, SettleOutcome::NoEffect), Ok(8));
    // The queue goes on.
    let worker = rig.worker();
    let submission = rig.submit(&worker.claim, &worker, 6);
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert!(worker.work.holds(6));
    worker.settle(&rig.client);
}

#[test]
fn a_rebind_without_an_adapter_leaves_no_device_and_may_be_repeated() {
    in_own_process!();
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            refuse_open(false);
        }
    }
    let Some(rig) = rig() else { return };
    let claim = rig.claim(5);
    rig.lose();
    let _reset = Reset;
    refuse_open(true);
    let missing = Unavailable::DeviceCreationRefused;
    for _ in 0..2 {
        assert_eq!(
            rig.owner.rebind_device(),
            Err(DeviceRebindRefusal::Unavailable(missing))
        );
        assert_eq!(rig.owner.device(), None);
        assert_eq!(rig.owner.poll(), Err(DeviceFault::NoDevice));
    }
    assert_eq!(
        rig.client.admit(&request(1)).expect_err("no device"),
        AdmissionDenial::DeviceUnavailable
    );
    assert_eq!(rig.owner.usage().charged_bytes, 5);
    refuse_open(false);
    assert_eq!(rig.owner.rebind_device(), Ok(2));
    let successor = rig.client.rebind_claim(&claim).expect("moves");
    assert_eq!(rig.settle(&successor, SettleOutcome::Lost), Ok(5));
    assert!(rig.owner.close().is_settled());
}

#[test]
fn a_device_that_does_not_say_its_work_ended_is_left_alive_and_reported() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let dropped = rig.claim(7);
    let leaked = OwnerEvent::ClaimLeaked {
        subject: claim_subject(dropped.info()),
        leaked_bytes: 7,
    };
    drop(rig.client.submit(&dropped, |_| Ok(())).expect("submits"));
    drop(dropped);
    rig.device(|device| {
        // Really over, and the driver does not say so.
        device.wait_really(1);
        device.faults().stall(true);
        device.faults().lose_at(Call::Wait);
    });
    let report = rig.owner.close();
    assert_eq!(report.device, DeviceFate::Unconfirmed);
    assert_eq!((report.in_flight, report.charged_bytes), (1, 7));
    assert_eq!(rig.log.count(OwnerEvent::DeviceLeaked { generation: 1 }), 1);
    assert_eq!(rig.log.count(leaked), 1);
    assert_eq!(
        rig.log.count_like(OwnerEvent::GenerationRetired {
            generation: 1,
            resolved: 0
        }),
        0
    );
    // Its device is still the process's one device.
    assert_eq!(
        GpuOwner::open(policy(1), rig.log.clone()).err(),
        Some(Unavailable::OwnerAlreadyOpen)
    );
}

#[test]
fn a_leased_device_that_never_says_so_is_reported_when_its_last_holder_returns_it() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let claim = rig.claim(7);
    let lease = rig.client.lease(&claim).expect("authorized");
    rig.device(|device| {
        device.faults().stall(true);
        device.faults().lose_at(Call::Wait);
    });
    assert_eq!(rig.owner.close().device, DeviceFate::Held { holders: 1 });
    let leaked = OwnerEvent::DeviceLeaked { generation: 1 };
    assert_eq!(rig.log.count(leaked), 0);
    drop(lease);
    assert_eq!(rig.log.count(leaked), 1);
    assert_eq!(
        rig.log.count_like(OwnerEvent::RetainedDeviceDestroyed {
            generation: 1,
            resolved: 0
        }),
        0
    );
    assert_eq!(rig.settle(&claim, SettleOutcome::Canceled), Ok(7));
}

#[test]
fn a_second_rebind_is_refused_while_one_runs() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    rig.lose();
    let running = &rig.owner.shared.rebinding;
    running.store(true, Ordering::SeqCst);
    for _ in 0..2 {
        assert_eq!(
            rig.owner.rebind_device(),
            Err(DeviceRebindRefusal::InProgress)
        );
    }
    running.store(false, Ordering::SeqCst);
    assert_eq!(rig.owner.rebind_device(), Ok(2));
    assert!(!running.load(Ordering::SeqCst));
}
