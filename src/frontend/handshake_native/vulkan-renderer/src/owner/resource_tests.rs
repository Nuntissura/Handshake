//! Owner resources on the machine's real device under conditions a healthy machine does not hold still on its own: work
//! kept pending by the fault seam's gate, a recording kept open by the test, and an injected device loss.

use super::fault_tests::support::claim_subject;
use super::fault_tests::support::gpu_env::{in_own_process, ran_in_its_own_process};
use super::fault_tests::{rig, Rig, COMPLETED};
use super::MAX_WAIT;
use crate::{
    AccessRefusal, AttachmentFormat, BufferMemory, BufferRequest, BufferUsage, Claim, Extent,
    ImageLayout, ImageRequest, ImageUsage, OwnerEvent, OwnerImage, RecordAborted, SettleOutcome,
    SettleRefusal, Settlement, Submission,
};
use std::sync::Barrier;

fn image(rig: &Rig, claim: &Claim) -> OwnerImage {
    let request = ImageRequest {
        format: AttachmentFormat::Rgba8Unorm,
        extent: Extent {
            width: 32,
            height: 32,
        },
        usage: ImageUsage::OFFSCREEN,
    };
    rig.client.create_image(claim, &request).expect("made")
}

fn clear(rig: &Rig, claim: &Claim, image: &OwnerImage) -> Submission {
    rig.client
        .submit(claim, |recording| {
            recording
                .clear(
                    image,
                    ImageLayout::Undefined,
                    [0.5; 4],
                    ImageLayout::General,
                )
                .map_err(|_| RecordAborted)
        })
        .expect("submits")
}

/// Owner resources that exist on the bound generation, released ones included.
fn resources(rig: &Rig) -> u64 {
    rig.owner.memory_use().expect("bound").resources
}

#[test]
fn a_released_image_waits_for_the_submission_that_uses_it() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let image = image(&rig, &claim);
    let bytes = image.bytes();
    rig.device(|device| device.close_gate());
    let submission = clear(&rig, &claim, &image);
    drop(image);
    assert_eq!(rig.owner.poll(), Ok(()));
    assert_eq!(resources(&rig), 1, "destroyed under work that uses it");
    assert_eq!(rig.owner.usage().owned_resources, 1);
    assert_eq!(
        rig.client
            .settle(&claim, Settlement::clean(SettleOutcome::Completed)),
        Err(SettleRefusal::InFlight { in_flight: 1 })
    );
    rig.device(|device| device.open_gate());
    rig.device(|device| device.wait_really(submission.sequence()));
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert_eq!(resources(&rig), 0);
    assert_eq!(rig.owner.usage().owned_resources, 0);
    let subject = claim_subject(claim.info());
    assert_eq!(
        rig.log.count(OwnerEvent::ResourceEnded { subject, bytes }),
        1
    );
    rig.client
        .settle(&claim, Settlement::clean(SettleOutcome::Completed))
        .expect("settles");
}

#[test]
fn a_release_during_a_recording_waits_for_that_recording() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let other = rig.claim(1 << 20);
    let image = image(&rig, &claim);
    let (open, released) = (Barrier::new(2), Barrier::new(2));
    let submission = std::thread::scope(|scope| {
        let recording = scope.spawn(|| {
            rig.client
                .submit(&other, |_| {
                    open.wait();
                    // The image is released while this command buffer is open; it may refer to it.
                    released.wait();
                    Ok(())
                })
                .expect("submits")
        });
        open.wait();
        drop(image);
        assert_eq!(rig.owner.poll(), Ok(()));
        assert_eq!(
            resources(&rig),
            1,
            "destroyed while a recording may refer to it"
        );
        released.wait();
        recording.join().expect("ends")
    });
    rig.device(|device| device.wait_really(submission.sequence()));
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    assert_eq!(resources(&rig), 0);
}

#[test]
fn a_buffer_is_not_read_while_work_of_its_client_runs() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let request = BufferRequest {
        size: 256,
        usage: BufferUsage::READBACK,
        memory: BufferMemory::Readback,
    };
    let buffer = rig.client.create_buffer(&claim, &request).expect("made");
    rig.device(|device| device.close_gate());
    let pending = rig.client.submit(&claim, |_| Ok(())).expect("submits");
    let mut out = [0u8; 16];
    assert_eq!(
        buffer.read(0, &mut out),
        Err(AccessRefusal::InFlight { in_flight: 1 })
    );
    assert_eq!(
        buffer.write(0, &out),
        Err(AccessRefusal::InFlight { in_flight: 1 })
    );
    rig.device(|device| device.open_gate());
    rig.device(|device| device.wait_really(pending.sequence()));
    assert_eq!(rig.client.wait(&pending, MAX_WAIT), Ok(COMPLETED));
    assert_eq!(buffer.read(0, &mut out), Ok(()));
}

#[test]
fn resources_on_a_lost_device_end_with_it_once() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let first = image(&rig, &claim);
    let bytes = first.bytes();
    rig.lose();
    assert!(
        first.attachment().is_err(),
        "a lost device's image is not used"
    );
    // The image does not hold the lost device: it is destroyed with it, and the claim moves on.
    assert_eq!(rig.owner.rebind_device(), Ok(2));
    let successor = rig.client.rebind_claim(&claim).expect("moves");
    assert_eq!(rig.owner.usage().owned_resources, 0);
    drop(first);
    let _ = rig.owner.poll();
    let subject = claim_subject(claim.info());
    assert_eq!(
        rig.log.count(OwnerEvent::ResourceEnded { subject, bytes }),
        0
    );
    // The successor makes resources on the new device.
    let again = image(&rig, &successor);
    assert_eq!(again.generation(), 2);
    drop(again);
    assert_eq!(rig.owner.poll(), Ok(()));
    assert_eq!(rig.owner.usage().owned_resources, 0);
}
