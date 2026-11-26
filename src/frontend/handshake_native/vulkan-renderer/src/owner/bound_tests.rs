//! Every wait ends at its own deadline, and notices what it waits for before it. In unit tests a wait inside Vulkan takes
//! 25 seconds and proves nothing (`vulkan/fault.rs`), whatever the machine's driver does: one that is made exceeds every
//! bound checked here, and it is counted.

use super::fault_tests::support::gpu_env::{in_own_process, open, ran_in_its_own_process};
use super::fault_tests::support::{policy, request};
use super::fault_tests::{attach, rig, Rig, COMPLETED, DEVICE_LOST, LATE, LOST, QUOTA, SHORT};
use super::MAX_WAIT;
use crate::vulkan::fault::{waits, AfterLoss, Call};
use crate::{
    DeviceFate, DeviceFault, DeviceHealth, DeviceRebindRefusal, OwnerEvent, ReleaseRefusal,
    SubmissionRefusal, SubmissionState,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn timed<T>(call: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let result = call();
    (result, start.elapsed())
}

/// Not before its deadline and not long after it.
fn ended_at(deadline: Duration, took: Duration) {
    assert!(
        took >= deadline && took < deadline + LATE,
        "{took:?} for {deadline:?}"
    );
}

/// Well before the deadline of a wait of `MAX_WAIT`.
fn noticed(took: Duration) {
    assert!(took < MAX_WAIT * 3 / 5, "{took:?}");
}

#[test]
fn a_wait_for_running_work_ends_at_its_deadline() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let worker = rig.worker();
    rig.device(|device| device.close_gate());
    let submission = rig.submit(&worker.claim, &worker, 1);
    for timeout in [Duration::ZERO, SHORT, SHORT * 8] {
        let (state, took) = timed(|| rig.client.wait(&submission, timeout));
        assert_eq!(state, Ok(SubmissionState::Pending));
        ended_at(timeout, took);
        let (health, took) = timed(|| rig.owner.probe(timeout));
        assert_eq!(health, Ok(DeviceHealth::Busy));
        ended_at(timeout, took);
        let (release, took) = timed(|| worker.lease.wait_release(timeout));
        assert_eq!(release, Err(ReleaseRefusal::InFlight { in_flight: 1 }));
        ended_at(timeout, took);
    }
    // However long the caller would wait.
    let (state, took) = timed(|| rig.client.wait(&submission, Duration::MAX));
    assert_eq!(state, Ok(SubmissionState::Pending));
    ended_at(MAX_WAIT, took);
    rig.device(|device| device.open_gate());
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    worker.settle(&rig.client);
    assert!(rig.owner.close().is_settled());
    assert_eq!(waits(), 0, "a wait inside Vulkan");
}

#[test]
fn the_end_of_the_work_and_a_loss_are_noticed_while_waiting() {
    in_own_process!();
    type Cause = fn(&Rig);
    let causes: [(Cause, SubmissionState, Result<DeviceHealth, DeviceFault>); 2] = [
        (
            |rig| rig.device(|device| device.open_gate()),
            COMPLETED,
            Ok(DeviceHealth::Idle),
        ),
        (
            |rig| rig.device(|device| device.faults().lose(AfterLoss::LOST)),
            DEVICE_LOST,
            Err(LOST),
        ),
    ];
    for (cause, state, health) in causes {
        let Some(rig) = rig() else { return };
        let worker = rig.worker();
        rig.device(|device| device.close_gate());
        let submission = rig.submit(&worker.claim, &worker, 2);
        // Nothing panics before the cause: the waits would go on.
        let (waited, probed, released, early) = std::thread::scope(|scope| {
            let wait = scope.spawn(|| timed(|| rig.client.wait(&submission, MAX_WAIT)));
            let probe = scope.spawn(|| timed(|| rig.owner.probe(MAX_WAIT)));
            let release = scope.spawn(|| timed(|| worker.lease.wait_release(MAX_WAIT)));
            std::thread::sleep(SHORT);
            let early = wait.is_finished() || probe.is_finished() || release.is_finished();
            cause(&rig);
            (wait.join(), probe.join(), release.join(), early)
        });
        assert!(!early, "ended before its cause");
        let (waited, took) = waited.expect("no panic");
        assert_eq!(waited, Ok(state));
        noticed(took);
        let (probed, took) = probed.expect("no panic");
        assert_eq!(probed, health);
        noticed(took);
        let (released, took) = released.expect("no panic");
        assert_eq!(released, Ok(()));
        noticed(took);
        // An injected loss ended nothing on the real device.
        rig.device(|device| device.open_gate());
        rig.device(|device| device.wait_really(1));
        drop(worker.let_go(&rig.client));
    }
    assert_eq!(waits(), 0, "a wait inside Vulkan");
}

/// A device that has told its loss and is silent from then on: whatever asks it gives up at its own deadline.
#[test]
fn a_rebind_and_a_close_of_a_silent_lost_device_are_bounded() {
    in_own_process!();
    let Some(mut rig) = rig() else { return };
    let claim = rig.claim(8);
    let submission = rig.submit_empty(&claim);
    rig.device(|device| {
        // Really over, so that nothing runs when this process ends.
        device.wait_really(1);
        device.faults().after_loss(AfterLoss::SILENT);
        device.faults().lose_at(Call::Sentinel);
    });
    assert_eq!(rig.owner.poll(), Err(LOST));

    let (refused, took) = timed(|| rig.owner.rebind_device());
    assert_eq!(refused, Err(DeviceRebindRefusal::NotQuiescent));
    ended_at(MAX_WAIT, took);
    let (state, took) = timed(|| rig.client.wait(&submission, SHORT));
    assert_eq!(state, Err(SubmissionRefusal::Device(LOST)));
    ended_at(SHORT, took);
    let (health, took) = timed(|| rig.owner.probe(SHORT));
    assert_eq!(health, Err(LOST));
    ended_at(SHORT, took);
    let (report, took) = timed(|| rig.owner.close());
    assert_eq!(report.device, DeviceFate::Unconfirmed);
    ended_at(MAX_WAIT, took);
    assert_eq!(submission.state(), SubmissionState::Pending);

    // What drops the last handle looks once and waits for nothing.
    let log = rig.log.clone();
    rig.rescue = false;
    let ((), took) = timed(|| drop(rig));
    ended_at(Duration::ZERO, took);
    assert_eq!(log.count(OwnerEvent::DeviceLeaked { generation: 1 }), 1);
    assert_eq!(waits(), 0, "a wait inside Vulkan");
}

/// MT-203 S2-F3: a close holds no lock across its draining and the destruction that follows. While the first close
/// drains work that does not end, a second close and a rebind return at once; the first one's report is what every
/// later close returns, and its device, whose work did not end in time, is kept.
#[test]
fn a_close_that_drains_keeps_no_other_call_waiting() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let claim = rig.claim(8);
    rig.device(|device| device.close_gate());
    let submission = rig.submit_empty(&claim);
    let (first, second, rebind, early) = std::thread::scope(|scope| {
        let first = scope.spawn(|| timed(|| rig.owner.close()));
        // Admission closes as the close begins; the usage is read with no lock of the close.
        let begun = Instant::now();
        while rig.owner.usage().admission_open {
            assert!(begun.elapsed() < LATE, "the close began");
            std::thread::sleep(Duration::from_millis(1));
        }
        let second = timed(|| rig.owner.close());
        let rebind = timed(|| rig.owner.rebind_device());
        let early = first.is_finished();
        (first.join().expect("no panic"), second, rebind, early)
    });
    assert!(!early, "the first close was still draining");
    let (second, took) = second;
    assert_eq!(second.device, DeviceFate::Closing);
    assert!(!second.is_settled());
    noticed(took);
    let (rebind, took) = rebind;
    assert_eq!(rebind, Err(DeviceRebindRefusal::Closed));
    noticed(took);
    let (first, took) = first;
    assert_eq!(first.device, DeviceFate::Unconfirmed, "kept: its work did not end");
    ended_at(MAX_WAIT, took);
    assert_eq!(rig.owner.close(), first, "single-shot");
    rig.generation(1, |device| device.open_gate());
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert_eq!(waits(), 0, "a wait inside Vulkan");
}

/// The host that drops the owner gets on, whatever the device still does.
#[test]
fn a_dropped_owner_over_running_work_is_bounded() {
    in_own_process!();
    let Some((owner, log)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1);
    let claim = client.admit(&request(8)).expect("fits");
    let shared = Arc::clone(&owner.shared);
    let gate = |open: bool| {
        let slot = shared.lock_slot();
        let live = slot.generation(1).expect("exists");
        if open {
            live.vulkan.open_gate();
        } else {
            live.vulkan.close_gate();
        }
    };
    gate(false);
    let submission = client.submit(&claim, |_| Ok(())).expect("submits");
    let ((), took) = timed(|| drop(owner));
    let state = client.observe(&submission);
    let before = log.events();
    gate(true);
    ended_at(MAX_WAIT, took);
    assert_eq!(state, Ok(SubmissionState::Pending));
    let ended = OwnerEvent::RetainedDeviceDestroyed {
        generation: 1,
        resolved: 0,
    };
    assert!(!before.contains(&ended));
    // The client that is left ends what the owner could not.
    assert_eq!(client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert_eq!(log.count(ended), 1);
    assert_eq!(waits(), 0, "a wait inside Vulkan");
    let Some((again, _)) = open(policy(1)) else {
        panic!("the same device opens again")
    };
    assert!(again.close().is_settled());
}
