//! What happens while a rebind creates its device. The creation is the driver's and takes as long as it takes: here it
//! waits until the case lets it go on.

use super::fault_tests::support::gpu_env::{in_own_process, open, ran_in_its_own_process};
use super::fault_tests::support::{policy, request};
use super::fault_tests::{rig, Rig, LATE};
use super::GpuOwner;
use crate::vulkan::fault::{hold_open, refuse_open};
use crate::{
    AdmissionDenial, DeviceFate, DeviceFault, DeviceRebindRefusal, NullDiagnostics, OwnerEvent,
    OwnerPhase, Unavailable,
};
use std::sync::Arc;
use std::time::Instant;

/// Rebind on one thread and, while its device is being created, do `meanwhile` on this one. Nothing in `meanwhile`
/// panics: the creation has to go on.
fn during_the_creation<T>(
    rig: &Rig,
    meanwhile: impl FnOnce() -> T,
) -> (Result<u64, DeviceRebindRefusal>, T) {
    let (rebound, reached, done) = std::thread::scope(|scope| {
        let held = hold_open();
        let rebind = scope.spawn(|| rig.owner.rebind_device());
        let reached = held.is_reached();
        let done = meanwhile();
        drop(held);
        (rebind.join(), reached, done)
    });
    assert!(reached, "no device was being created");
    (rebound.expect("no panic"), done)
}

fn slot_is_taken() -> bool {
    matches!(
        GpuOwner::open(policy(1), Arc::new(NullDiagnostics)),
        Err(Unavailable::OwnerAlreadyOpen)
    )
}

/// The close does not wait for the driver, does not call a device gone that is about to exist, and that device is never
/// bound.
#[test]
fn a_close_during_the_creation_says_so_and_the_device_never_appears() {
    in_own_process!();
    for refuse in [false, true] {
        let Some(rig) = rig() else { return };
        let claim = rig.claim(5);
        rig.lose();
        let (rebound, (report, took, taken)) = during_the_creation(&rig, || {
            let start = Instant::now();
            let report = rig.owner.close();
            let took = start.elapsed();
            let taken = slot_is_taken();
            refuse_open(refuse);
            (report, took, taken)
        });
        refuse_open(false);
        assert_eq!(rebound, Err(DeviceRebindRefusal::Closed));
        assert_eq!(report.device, DeviceFate::Creating);
        assert_eq!((report.open_claims, report.charged_bytes), (1, 5));
        assert!(!report.is_settled());
        assert!(took < LATE, "{took:?}");
        assert!(taken, "a second owner beside a device being created");

        assert_eq!(rig.owner.close(), report);
        assert_eq!(rig.owner.device(), None);
        assert_eq!(rig.owner.poll(), Err(DeviceFault::NoDevice));
        assert_eq!(rig.owner.rebind_device(), Err(DeviceRebindRefusal::Closed));
        assert_eq!(rig.log.count(OwnerEvent::DeviceCreatingAtClose), 1);
        let events = rig.log.events();
        let position = |event| events.iter().position(|seen| *seen == event);
        let closed = OwnerEvent::DeviceRebindRefused {
            reason: DeviceRebindRefusal::Closed.number(),
        };
        // The end of it is told after the close.
        assert!(position(closed) > position(OwnerEvent::DeviceCreatingAtClose));
        for never in [
            OwnerEvent::Phase {
                phase: OwnerPhase::Open,
                generation: 2,
            },
            OwnerEvent::Phase {
                phase: OwnerPhase::Unavailable,
                generation: 0,
            },
        ] {
            assert_eq!(rig.log.count(never), 0, "{never:?}");
        }
        // It is gone, whether it came to exist or not.
        let Some((again, _)) = open(policy(1)) else {
            panic!("the same device opens again")
        };
        assert!(again.close().is_settled());
        drop(claim);
    }
}

#[test]
fn a_second_rebind_is_refused_while_one_creates_its_device() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    rig.lose();
    let (rebound, second) = during_the_creation(&rig, || rig.owner.rebind_device());
    assert_eq!(second, Err(DeviceRebindRefusal::InProgress));
    assert_eq!(rebound, Ok(2));
    assert_eq!(
        rig.log.count(OwnerEvent::DeviceRebindRefused {
            reason: DeviceRebindRefusal::InProgress.number()
        }),
        1
    );
    // The first one is over, and the next one is a first one again.
    assert_eq!(rig.owner.rebind_device(), Err(DeviceRebindRefusal::NotLost));
    rig.lose();
    assert_eq!(rig.owner.rebind_device(), Ok(3));
    assert!(rig.owner.close().is_settled());
}

#[test]
fn clients_are_answered_while_a_device_is_created() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let claim = rig.claim(5);
    rig.lose();
    let (rebound, (polled, probed, denied, leased, took)) = during_the_creation(&rig, || {
        let start = Instant::now();
        let polled = rig.client.poll();
        let probed = rig.owner.probe(super::MAX_WAIT);
        let denied = rig.client.admit(&request(1)).err();
        let leased = rig.client.lease(&claim).is_ok();
        (polled, probed, denied, leased, start.elapsed())
    });
    assert_eq!(rebound, Ok(2));
    assert_eq!(polled, Err(DeviceFault::NoDevice));
    assert_eq!(probed, Err(DeviceFault::NoDevice));
    assert_eq!(denied, Some(AdmissionDenial::DeviceLost { generation: 1 }));
    assert!(!leased);
    assert!(took < LATE, "{took:?}");
    assert!(rig.client.rebind_claim(&claim).is_ok());
}
