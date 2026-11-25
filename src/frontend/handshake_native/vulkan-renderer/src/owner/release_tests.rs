//! How a holder learns that it may let go, and that leaving early costs nobody the device or the owner slot.

use super::fault_tests::support::gpu_env::{in_own_process, open, ran_in_its_own_process};
use super::fault_tests::support::gpu_work::Worker;
use super::fault_tests::support::{policy, request};
use super::fault_tests::{attach, rig, Rig, COMPLETED, SHORT};
use super::{GpuOwner, MAX_WAIT};
use crate::vulkan::fault::AfterLoss;
use crate::{
    AuthorizeRefusal, ChargeClass, DeviceFate, DeviceFault, OwnerEvent, ReleaseRefusal,
    SettleOutcome, Settlement, SubmissionState, Unavailable,
};
use std::time::{Duration, Instant};

const ENDED: OwnerEvent = OwnerEvent::RetainedDeviceDestroyed {
    generation: 1,
    resolved: 0,
};
const LEAKED: OwnerEvent = OwnerEvent::DeviceLeaked { generation: 1 };
const RUNNING: Result<(), ReleaseRefusal> = Err(ReleaseRefusal::InFlight { in_flight: 1 });

/// Whether the process's owner slot is taken. Frees it again when it was free.
fn slot_is_taken() -> bool {
    match GpuOwner::open(policy(1), std::sync::Arc::new(crate::NullDiagnostics)) {
        Err(Unavailable::OwnerAlreadyOpen) => true,
        Err(reason) => panic!("{}", reason.key()),
        Ok(owner) => {
            assert!(owner.close().is_settled());
            false
        }
    }
}

/// Close over the work of `rig`, which is held by its gate, with one lease out.
fn close_over_held_work(rig: &Rig) {
    let report = rig.owner.close();
    assert_eq!(
        (report.device, report.in_flight, report.leases),
        (DeviceFate::Held { holders: 1 }, 1, 1)
    );
}

#[test]
fn a_holder_learns_after_a_close_when_it_may_let_go() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let worker = rig.worker();
    assert_eq!(worker.lease.may_release(), Ok(()), "nothing runs yet");
    rig.device(|device| device.close_gate());
    let submission = rig.submit(&worker.claim, &worker, 17);
    assert_eq!(worker.lease.may_release(), RUNNING);
    close_over_held_work(&rig);

    assert_eq!(
        worker.lease.handles().err(),
        Some(AuthorizeRefusal::DeviceUnavailable)
    );
    assert_eq!(worker.lease.may_release(), RUNNING);
    assert_eq!(worker.lease.wait_release(SHORT), RUNNING);
    let pending = Ok(SubmissionState::Pending);
    assert_eq!(rig.client.observe(&submission), pending);
    assert_eq!(rig.client.wait(&submission, SHORT), pending);
    assert!(!worker.work.holds(17));

    rig.generation(1, |device| device.open_gate());
    assert_eq!(worker.lease.wait_release(MAX_WAIT), Ok(()));
    assert_eq!(submission.state(), COMPLETED);
    assert!(worker.work.holds(17), "allowed before the GPU was done");
    assert_eq!(worker.lease.may_release(), Ok(()));
    assert!(slot_is_taken());
    assert_eq!(rig.log.count(ENDED), 0);
    let claim = worker.let_go(&rig.client);
    assert_eq!((rig.log.count(ENDED), rig.log.count(LEAKED)), (1, 0));
    assert!(rig.settle(&claim, SettleOutcome::Completed).is_ok());
    assert!(!slot_is_taken());
}

/// Every look after the work has ended will do: a client's, the host's or another lease that is returned.
#[test]
fn a_lease_returned_too_early_leaves_its_device_to_the_next_look() {
    in_own_process!();
    type Look = fn(&Rig);
    let looks: [Look; 2] = [
        |rig| assert_eq!(rig.client.poll(), Err(DeviceFault::NoDevice)),
        |rig| assert_eq!(rig.owner.poll(), Err(DeviceFault::NoDevice)),
    ];
    for look in looks {
        let Some(rig) = rig() else { return };
        let claim = rig.claim(16);
        let lease = rig.client.lease(&claim).expect("authorized");
        rig.device(|device| device.close_gate());
        let submission = rig.submit_empty(&claim);
        close_over_held_work(&rig);
        assert_eq!(lease.may_release(), RUNNING);
        drop(lease);
        look(&rig);
        // Its work runs, so it is kept; and it is not given up.
        assert_eq!((rig.log.count(ENDED), rig.log.count(LEAKED)), (0, 0));
        assert!(slot_is_taken());

        rig.generation(1, |device| {
            device.open_gate();
            device.wait_really(1);
        });
        assert_eq!(rig.log.count(ENDED), 0, "nobody looked");
        look(&rig);
        assert_eq!((rig.log.count(ENDED), rig.log.count(LEAKED)), (1, 0));
        assert_eq!(submission.state(), COMPLETED);
        assert_eq!(rig.settle(&claim, SettleOutcome::Completed), Ok(16));
        assert!(!slot_is_taken());
    }
}

#[test]
fn another_lease_that_is_returned_ends_a_device_whose_work_has_ended() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let claim = rig.claim(16);
    let first = rig.client.lease(&claim).expect("authorized");
    let last = rig.client.lease(&claim).expect("authorized");
    rig.device(|device| device.close_gate());
    let submission = rig.submit_empty(&claim);
    assert_eq!(
        rig.owner.close().device,
        DeviceFate::Held { holders: 2 },
        "both"
    );
    drop(first);
    rig.generation(1, |device| {
        device.open_gate();
        device.wait_really(1);
    });
    assert_eq!(rig.log.count(ENDED), 0, "one is still out");
    assert_eq!(last.may_release(), Ok(()));
    assert_eq!(rig.log.count(ENDED), 0, "one is still out");
    drop(last);
    assert_eq!((rig.log.count(ENDED), rig.log.count(LEAKED)), (1, 0));
    assert_eq!(submission.state(), COMPLETED);
    assert!(rig.settle(&claim, SettleOutcome::Completed).is_ok());
}

/// The whole life of a module that loses its handle first (GUI-DESKTOP-035): what it holds is ended by its holder.
#[test]
fn a_holder_without_its_clients_handle_can_end_everything() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let other = attach(&rig.owner, 2);
    let claim = other.admit(&request(Worker::RESERVE)).expect("fits");
    let worker = Worker::new(&other, claim);
    rig.device(|device| device.close_gate());
    let submission = other
        .submit(&worker.claim, |recording| {
            worker.work.record(recording, 23);
            Ok(())
        })
        .expect("submits");
    drop(other);
    assert_eq!(
        worker.lease.handles().err(),
        Some(AuthorizeRefusal::Revoked)
    );
    assert_eq!(worker.lease.may_release(), RUNNING);
    rig.device(|device| device.open_gate());
    assert_eq!(worker.lease.wait_release(MAX_WAIT), Ok(()));
    assert_eq!(submission.state(), COMPLETED);
    let Worker { work, lease, claim } = worker;
    assert!(work.holds(23));
    let bytes = work.allocated_bytes();
    drop(work);
    assert_eq!(claim.discharge(ChargeClass::Committed, bytes), Ok(()));
    let completed = Settlement::clean(SettleOutcome::Completed);
    assert!(claim.settle(completed).is_err(), "its lease is out");
    drop(lease);
    let receipt = claim.settle(completed).expect("settles");
    assert_eq!(receipt.released_bytes, Worker::RESERVE);
    let usage = rig.owner.usage();
    assert_eq!(
        (usage.charged_bytes, usage.open_claims, usage.leaked_claims),
        (0, 0, 0)
    );
    assert!(rig.owner.close().is_settled());
}

/// Only when the last handle of all goes before the device said that its work has ended is there nobody left to look.
#[test]
fn a_silent_device_is_given_up_once_with_the_last_handle_also_after_an_early_return() {
    in_own_process!();
    let Some(mut rig) = rig() else { return };
    let claim = rig.claim(7);
    let lease = rig.client.lease(&claim).expect("authorized");
    drop(rig.submit_empty(&claim));
    rig.device(|device| {
        // Really over, and the driver does not say so.
        device.wait_really(1);
        device.faults().lose(AfterLoss::SILENT);
    });
    assert_eq!(rig.owner.close().device, DeviceFate::Held { holders: 1 });
    drop(lease);
    let end = Instant::now() + SHORT;
    while Instant::now() < end {
        assert_eq!(rig.client.poll(), Err(DeviceFault::NoDevice));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!((rig.log.count(ENDED), rig.log.count(LEAKED)), (0, 0));
    let log = rig.log.clone();
    rig.rescue = false;
    drop(rig);
    assert_eq!((log.count(ENDED), log.count(LEAKED)), (0, 1));
    assert!(slot_is_taken());
    drop(claim);
    assert_eq!(log.count(LEAKED), 1);
}

#[test]
fn a_closed_owner_with_nothing_kept_answers_as_before() {
    in_own_process!();
    let Some((owner, _)) = open(policy(64)) else {
        return;
    };
    let client = attach(&owner, 1);
    let claim = client.admit(&request(8)).expect("fits");
    let submission = client.submit(&claim, |_| Ok(())).expect("submits");
    let report = owner.close();
    assert_eq!(
        (report.device, report.in_flight),
        (DeviceFate::Destroyed, 0)
    );
    assert_eq!(client.observe(&submission), Ok(COMPLETED));
    assert_eq!(client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert_eq!(client.poll(), Err(DeviceFault::NoDevice));
    assert_eq!(owner.probe(MAX_WAIT), Err(DeviceFault::NoDevice));
    assert_eq!(owner.close(), report);
    assert!(client
        .settle(&claim, Settlement::clean(SettleOutcome::Completed))
        .is_ok());
}
