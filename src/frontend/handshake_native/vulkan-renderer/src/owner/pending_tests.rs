//! Work that is really pending on the machine's device: a gate holds it until the case opens it, so whatever the
//! owner says of it before is not a matter of timing. See `fault_tests` for the seam.

use super::fault_tests::support::gpu_env::{in_own_process, ran_in_its_own_process};
use super::fault_tests::support::gpu_work::Worker;
use super::fault_tests::support::{bounded_policy, claim_subject, policy, request};
use super::fault_tests::{rig, rig_with, COMPLETED, QUOTA, SHORT};
use super::MAX_WAIT;
use crate::{
    AuthorizeRefusal, DeviceFate, DeviceHealth, OwnerEvent, SettleOutcome, SettleRefusal,
    SubmissionState, SubmitRefusal,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[test]
fn work_is_pending_until_the_device_says_otherwise() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let worker = rig.worker();
    rig.device(|device| device.close_gate());
    let submission = rig.submit(&worker.claim, &worker, 11);
    assert_eq!(
        rig.client.observe(&submission),
        Ok(SubmissionState::Pending)
    );
    let start = Instant::now();
    assert_eq!(
        rig.client.wait(&submission, SHORT),
        Ok(SubmissionState::Pending)
    );
    // Timers may be early by a tick.
    assert!(start.elapsed() >= SHORT / 2);
    assert_eq!(rig.owner.probe(SHORT), Ok(DeviceHealth::Busy));
    for outcome in [SettleOutcome::Canceled, SettleOutcome::Completed] {
        assert_eq!(
            rig.settle(&worker.claim, outcome),
            Err(SettleRefusal::InFlight { in_flight: 1 })
        );
    }
    assert!(!worker.work.holds(11));
    rig.device(|device| device.open_gate());
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert!(worker.work.holds(11));
    assert_eq!(rig.owner.probe(SHORT), Ok(DeviceHealth::Idle));
    worker.settle(&rig.client);
    assert!(rig.owner.close().is_settled());
}

#[test]
fn the_in_flight_bound_is_checked_before_the_client_records() {
    in_own_process!();
    let Some(rig) = rig_with(bounded_policy(QUOTA, 8, 1)) else {
        return;
    };
    let worker = rig.worker();
    rig.device(|device| device.close_gate());
    let first = rig.submit(&worker.claim, &worker, 1);
    let mut recorded = false;
    let refused = rig.client.submit(&worker.claim, |_| {
        recorded = true;
        Ok(())
    });
    assert_eq!(
        refused.err(),
        Some(SubmitRefusal::NotAuthorized(
            AuthorizeRefusal::InFlightLimit { max_in_flight: 1 }
        ))
    );
    assert!(!recorded);
    rig.device(|device| device.open_gate());
    assert_eq!(rig.client.wait(&first, MAX_WAIT), Ok(COMPLETED));
    worker.settle(&rig.client);
}

#[test]
fn a_probe_of_a_busy_device_blocks_nobody() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let worker = rig.worker();
    rig.device(|device| device.close_gate());
    let submission = rig.submit(&worker.claim, &worker, 2);
    let (probing, probed) = (AtomicBool::new(false), AtomicBool::new(false));
    std::thread::scope(|scope| {
        let probe = scope.spawn(|| {
            probing.store(true, Ordering::SeqCst);
            let health = rig.owner.probe(MAX_WAIT);
            probed.store(true, Ordering::SeqCst);
            health
        });
        while !probing.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        std::thread::sleep(SHORT);
        // Every one of these needs the slot, the queue or the ledger. Nothing here panics: the gate has to open.
        let observed = rig.client.observe(&submission);
        let polled = rig.client.poll();
        let other = rig.client.admit(&request(1));
        let submitted = other
            .as_ref()
            .ok()
            .map(|claim| rig.client.submit(claim, |_| Ok(())).is_ok());
        let was_probed = probed.load(Ordering::SeqCst);
        rig.device(|device| device.open_gate());
        let health = probe.join().expect("no panic");
        assert!(!was_probed, "waited for the probe");
        assert_eq!(observed, Ok(SubmissionState::Pending));
        assert_eq!((polled, submitted), (Ok(()), Some(true)));
        assert_eq!(health, Ok(DeviceHealth::Idle));
    });
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert_eq!(rig.owner.probe(MAX_WAIT), Ok(DeviceHealth::Idle));
    worker.settle(&rig.client);
}

#[test]
fn a_dropped_claim_stays_charged_while_its_work_is_really_pending() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let worker = rig.worker();
    let dropped = rig.claim(4096);
    let subject = claim_subject(dropped.info());
    rig.device(|device| device.close_gate());
    drop(rig.submit(&dropped, &worker, 7));
    drop(dropped);
    assert_eq!(
        rig.log.count(OwnerEvent::ClaimAbandoned {
            subject,
            in_flight: 1,
            charged_bytes: 4096
        }),
        1
    );
    let reclaimed = OwnerEvent::ClaimReclaimed {
        subject,
        released_bytes: 4096,
    };
    for _ in 0..3 {
        assert_eq!(rig.owner.poll(), Ok(()));
        assert_eq!(rig.owner.probe(SHORT), Ok(DeviceHealth::Busy));
        assert_eq!(rig.log.count(reclaimed), 0);
    }
    assert_eq!(rig.owner.usage().charged_bytes, Worker::RESERVE + 4096);
    rig.device(|device| device.open_gate());
    let end = Instant::now() + MAX_WAIT;
    while rig.log.count(reclaimed) == 0 {
        assert!(Instant::now() < end, "not reclaimed");
        assert_eq!(rig.owner.poll(), Ok(()));
    }
    assert!(worker.work.holds(7), "reclaimed before the GPU was done");
    assert_eq!(rig.owner.usage().charged_bytes, Worker::RESERVE);
    // Nothing ran under the worker's own claim.
    let claim = worker.let_go(&rig.client);
    assert_eq!(
        rig.settle(&claim, SettleOutcome::Canceled),
        Ok(Worker::RESERVE)
    );
}

#[test]
fn an_admission_that_finds_no_room_lets_the_owner_look_first() {
    in_own_process!();
    let Some(rig) = rig_with(policy(Worker::RESERVE + 64)) else {
        return;
    };
    let worker = rig.worker();
    let dropped = rig.claim(64);
    drop(rig.submit(&dropped, &worker, 8));
    drop(dropped);
    // The work ends without anybody asking the owner.
    rig.device(|device| device.wait_really(1));
    assert_eq!(rig.owner.usage().abandoned_claims, 1);
    let next = rig.claim(64);
    assert_eq!(rig.owner.counters().denied, 0);
    assert_eq!(rig.settle(&next, SettleOutcome::NoEffect), Ok(64));
    let claim = worker.let_go(&rig.client);
    assert!(rig.settle(&claim, SettleOutcome::Canceled).is_ok());
}

#[test]
fn a_close_over_running_work_is_bounded_and_says_what_it_left() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let worker = rig.worker();
    rig.device(|device| device.close_gate());
    let submission = rig.submit(&worker.claim, &worker, 13);
    let device = rig.device(Arc::downgrade);
    // Nothing panics before the gate is open.
    let (report, took, meanwhile, was_closing) = std::thread::scope(|scope| {
        let closing = scope.spawn(|| {
            let start = Instant::now();
            (rig.owner.close(), start.elapsed())
        });
        while rig.owner.shared.lock_slot().live.is_some() && !closing.is_finished() {
            std::thread::yield_now();
        }
        let meanwhile = rig.client.observe(&submission);
        let was_closing = !closing.is_finished();
        let (report, took) = closing.join().expect("no panic");
        (report, took, meanwhile, was_closing)
    });
    let state = submission.state();
    let before = worker.work.holds(13);
    let ended = OwnerEvent::RetainedDeviceDestroyed {
        generation: 1,
        resolved: 0,
    };
    let ended_before = rig.log.count_like(ended);
    if let Some(device) = device.upgrade() {
        device.open_gate();
        device.wait_really(1);
    }
    // A close that drains keeps no client waiting, and its work is still observed.
    assert_eq!(
        (meanwhile, was_closing),
        (Ok(SubmissionState::Pending), true)
    );
    assert!(took >= MAX_WAIT / 2 && took < 2 * MAX_WAIT, "{took:?}");
    assert_eq!(
        (report.device, report.in_flight),
        (DeviceFate::Held { holders: 1 }, 1)
    );
    assert_eq!(state, SubmissionState::Pending);
    assert!(!before);
    assert_eq!(ended_before, 0);
    // Its last holder returns it: what ran has completed by then, and that is what is said of it.
    assert!(worker.work.holds(13));
    let claim = worker.let_go(&rig.client);
    assert_eq!(rig.log.count(ended), 1);
    assert_eq!(submission.state(), COMPLETED);
    assert_eq!(
        rig.settle(&claim, SettleOutcome::Completed),
        Ok(Worker::RESERVE)
    );
}
