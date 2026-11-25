//! A lost device completes nothing, whatever it says of the work it cut: Vulkan lets it say anything. The work is really
//! pending behind its gate, so what is called completed here was not.

use super::fault_tests::support::gpu_env::{in_own_process, ran_in_its_own_process};
use super::fault_tests::support::gpu_work::Worker;
use super::fault_tests::{rig, Rig, DEVICE_LOST, LOST, SHORT};
use super::MAX_WAIT;
use crate::vulkan::fault::{AfterLoss, Call};
use crate::{
    DeviceFate, DeviceFault, DeviceRebindRefusal, OwnerEvent, SettleOutcome, SettleRefusal,
    Submission, SubmissionRefusal, SubmissionState,
};
use ash::vk;

type Look = fn(&Rig, &Submission);

/// Every way in which the owner asks the bound device. What each of them answers is not the subject here.
const LOOKS: [(&str, Look); 7] = [
    ("poll", |rig, _| {
        let _ = rig.owner.poll();
    }),
    ("client poll", |rig, _| {
        let _ = rig.client.poll();
    }),
    ("observe", |rig, submission| {
        let _ = rig.client.observe(submission);
    }),
    ("wait", |rig, submission| {
        let _ = rig.client.wait(submission, SHORT);
    }),
    ("probe", |rig, _| {
        let _ = rig.owner.probe(SHORT);
    }),
    ("rebind", |rig, _| {
        // Only a loss that is known is recovered from.
        let _ = rig.owner.poll();
        let _ = rig.owner.rebind_device();
    }),
    ("close", |rig, _| {
        let _ = rig.owner.close();
    }),
];

/// A worker whose work is really pending.
fn held_work(rig: &Rig) -> (Worker, Submission) {
    let worker = rig.worker();
    rig.device(|device| device.close_gate());
    let submission = rig.submit(&worker.claim, &worker, 5);
    (worker, submission)
}

/// The work was cut, and nobody says that it completed or had no effect.
fn assert_lost(rig: &Rig, submission: &Submission, worker: Worker, name: &str) {
    assert_eq!(submission.state(), DEVICE_LOST, "{name}");
    let counters = rig.owner.counters();
    assert_eq!(
        (counters.completed, counters.completed_lost),
        (0, 1),
        "{name}"
    );
    assert_eq!(
        rig.log
            .count(OwnerEvent::VulkanDeviceLost { generation: 1 }),
        1,
        "{name}"
    );
    // The injected loss ended nothing on the real device: let it run before its buffers go.
    rig.generation(1, |device| device.open_gate());
    rig.generation(1, |device| device.wait_really(1));
    let claim = worker.let_go(&rig.client);
    for outcome in [SettleOutcome::Completed, SettleOutcome::NoEffect] {
        assert!(
            matches!(
                rig.settle(&claim, outcome),
                Err(SettleRefusal::OutcomeContradicted { .. })
            ),
            "{name}"
        );
    }
    assert!(rig.settle(&claim, SettleOutcome::Lost).is_ok(), "{name}");
}

/// The fences are all signalled, as a driver may leave them when it loses the device. Only the fence that nothing was
/// submitted with shows why.
#[test]
fn a_loss_that_signals_every_fence_completes_nothing() {
    in_own_process!();
    for sentinel in [vk::Result::ERROR_DEVICE_LOST, vk::Result::SUCCESS] {
        for (name, look) in LOOKS {
            let Some(rig) = rig() else { return };
            let (worker, submission) = held_work(&rig);
            rig.device(|device| {
                device.faults().lose(AfterLoss {
                    fences: vk::Result::SUCCESS,
                    sentinel,
                })
            });
            look(&rig, &submission);
            assert_lost(&rig, &submission, worker, name);
        }
    }
}

/// A device that has told its loss and then answers as if nothing had happened: first that the work runs, which ends
/// nothing, and then that it has ended, which completes nothing.
#[test]
fn a_lost_device_that_later_calls_its_work_finished_completes_nothing() {
    in_own_process!();
    for (name, look) in LOOKS {
        let Some(rig) = rig() else { return };
        let (worker, submission) = held_work(&rig);
        rig.device(|device| {
            device.faults().after_loss(AfterLoss::SILENT);
            device.faults().lose_at(Call::Sentinel);
        });
        assert_eq!(rig.owner.poll(), Err(LOST), "{name}");
        let refused = SubmissionRefusal::Device(LOST);
        assert_eq!(rig.client.observe(&submission), Err(refused), "{name}");
        assert_eq!(rig.client.wait(&submission, SHORT), Err(refused), "{name}");
        assert_eq!(rig.owner.probe(SHORT), Err(LOST), "{name}");
        assert_eq!(submission.state(), SubmissionState::Pending, "{name}");
        assert_eq!(rig.owner.usage().in_flight, 1, "{name}");

        rig.device(|device| device.faults().after_loss(AfterLoss::FINISHED));
        look(&rig, &submission);
        assert_lost(&rig, &submission, worker, name);
    }
}

/// As above, for a device that its owner has let go of before: set aside by a rebind, or kept by a close.
#[test]
fn a_kept_device_that_is_lost_completes_nothing() {
    in_own_process!();
    type Ask = fn(&Rig, &Submission);
    let asks: [(&str, Ask); 3] = [
        ("poll", |rig, _| {
            assert_eq!(rig.owner.poll(), Err(DeviceFault::NoDevice));
        }),
        ("observe", |rig, submission| {
            let _ = rig.client.observe(submission);
        }),
        ("wait", |rig, submission| {
            let _ = rig.client.wait(submission, SHORT);
        }),
    ];
    for (name, ask) in asks {
        let Some(rig) = rig() else { return };
        let (worker, submission) = held_work(&rig);
        // Closed over running work; the loss comes after that, and its fences are all signalled.
        assert_eq!(
            rig.owner.close().device,
            DeviceFate::Held { holders: 1 },
            "{name}"
        );
        rig.generation(1, |device| {
            device.faults().lose(AfterLoss {
                fences: vk::Result::SUCCESS,
                ..AfterLoss::LOST
            })
        });
        ask(&rig, &submission);
        assert_lost(&rig, &submission, worker, name);
        assert_eq!(
            rig.log.count(OwnerEvent::RetainedDeviceDestroyed {
                generation: 1,
                resolved: 0
            }),
            1,
            "{name}"
        );
    }
}

/// What a lost device says of its work cannot be completed, and what it does not say is not assumed either.
#[test]
fn a_silent_lost_device_keeps_its_work_in_flight() {
    in_own_process!();
    let Some(rig) = rig() else { return };
    let (worker, submission) = held_work(&rig);
    rig.device(|device| {
        device.faults().after_loss(AfterLoss::SILENT);
        device.faults().lose_at(Call::Fence);
    });
    assert_eq!(rig.owner.poll(), Err(LOST));
    assert_eq!(
        rig.owner.rebind_device(),
        Err(DeviceRebindRefusal::NotQuiescent)
    );
    assert_eq!(rig.client.poll(), Err(LOST));
    for outcome in [SettleOutcome::Lost, SettleOutcome::Completed] {
        assert_eq!(
            rig.settle(&worker.claim, outcome),
            Err(SettleRefusal::InFlight { in_flight: 1 })
        );
    }
    assert!(worker.lease.may_release().is_err());
    assert_eq!(submission.state(), SubmissionState::Pending);
    // It says so in the end, and that is all that was missing.
    rig.device(|device| device.faults().after_loss(AfterLoss::LOST));
    assert_eq!(worker.lease.wait_release(MAX_WAIT), Ok(()));
    assert_lost(&rig, &submission, worker, "in the end");
}
