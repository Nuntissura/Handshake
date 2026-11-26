//! Owner resources on the machine's real device under conditions a healthy machine does not hold still on its own: work
//! kept pending by the fault seam's gate, a recording kept open by the test, an injected device loss, an injected
//! budget refusal, and a frame that a plain submission must not use.

use super::fault_tests::support::gpu_env::{
    in_own_process, is_required, ran_in_its_own_process, REQUIRE_GPU,
};
use super::fault_tests::support::{claim_subject, policy, EventLog};
use super::fault_tests::{rig, Rig, COMPLETED, QUOTA};
use super::{GpuOwner, MAX_WAIT};
use crate::present::{PresentCounters, SurfaceOrigin};
use crate::vulkan::Retirement;
use crate::{
    AccessRefusal, AttachmentFormat, BufferMemory, BufferRequest, BufferUsage, ByteQuantity, Claim,
    Extent, Frame, ImageLayout, ImageRequest, ImageUsage, OwnerBuffer, OwnerEvent, OwnerImage,
    OwnerOptions, RecordAborted, RecordRefusal, RenderAttachment, ResourceRefusal, SettleOutcome,
    SettleRefusal, Settlement, Submission,
};
use std::sync::{Arc, Barrier};

fn readback(rig: &Rig, claim: &Claim, size: u64) -> OwnerBuffer {
    let request = BufferRequest {
        size,
        usage: BufferUsage::READBACK,
        memory: BufferMemory::Readback,
    };
    rig.client.create_buffer(claim, &request).expect("made")
}

/// Bytes of the owner's own memory objects, over all heaps.
fn owner_bytes(rig: &Rig) -> u64 {
    rig.owner
        .memory_use()
        .expect("bound")
        .heaps
        .iter()
        .map(|heap| heap.bytes.measured().expect("measured"))
        .sum()
}

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

#[test]
fn a_recording_refuses_an_image_or_buffer_of_another_generation() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let old_image = image(&rig, &claim);
    let old_buffer = readback(&rig, &claim, 32 * 32 * 4);
    rig.lose();
    assert_eq!(rig.owner.rebind_device(), Ok(2));
    let current = rig.claim(16 << 20);
    let new_image = image(&rig, &current);
    let new_buffer = readback(&rig, &current, 32 * 32 * 4);
    let submission = rig
        .client
        .submit(&current, |recording| {
            let (from, to) = (ImageLayout::Undefined, ImageLayout::TransferSrc);
            assert_eq!(
                recording.clear(&old_image, from, [0.0; 4], to),
                Err(RecordRefusal::OtherGeneration)
            );
            assert_eq!(
                recording.transition(&old_image, from, to),
                Err(RecordRefusal::OtherGeneration)
            );
            assert_eq!(
                recording.read_back(&new_image, from, &old_buffer, to),
                Err(RecordRefusal::OtherGeneration)
            );
            // The same calls on this generation's own are recorded.
            recording
                .read_back(&new_image, from, &new_buffer, to)
                .map_err(|_| RecordAborted)
        })
        .expect("submits");
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    drop((old_image, old_buffer, new_image, new_buffer));
    assert_eq!(rig.owner.poll(), Ok(()));
    rig.settle(&current, SettleOutcome::Completed)
        .expect("settles");
}

#[test]
fn a_memory_object_over_the_drivers_budget_is_refused_and_placed_nowhere_else() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let upload = BufferRequest {
        size: 4096,
        usage: BufferUsage {
            transfer_src: true,
            ..BufferUsage::default()
        },
        memory: BufferMemory::Upload,
    };
    assert_eq!(owner_bytes(&rig), 0);
    // The driver's budget says no to the next memory object, whatever memory type it is of.
    rig.device(|device| device.faults().refuse_budget_once());
    let refused = rig.client.create_buffer(&claim, &upload);
    let heap = match &refused {
        Err(ResourceRefusal::OverBudget { heap }) => *heap,
        other => panic!("{other:?}"),
    };
    // Not moved to another memory type: nothing was allocated, nothing is charged, and the refusal is told.
    assert_eq!(owner_bytes(&rig), 0, "placed in another memory type");
    assert_eq!(rig.owner.usage().owned_resources, 0);
    let subject = claim_subject(claim.info());
    let reason = ResourceRefusal::OverBudget { heap }.number();
    assert_eq!(
        rig.log
            .count(OwnerEvent::ResourceRefused { subject, reason }),
        1
    );
    // The budget answered once: the same request is placed now.
    let placed = rig.client.create_buffer(&claim, &upload).expect("placed");
    assert!(owner_bytes(&rig) >= placed.bytes());
    drop(placed);
    assert_eq!(rig.owner.poll(), Ok(()));
    assert_eq!(owner_bytes(&rig), 0);
    rig.settle(&claim, SettleOutcome::NoEffect)
        .expect("settles");
}

#[test]
fn a_read_of_memory_the_host_does_not_see_coherently_invalidates_it_first() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let image = image(&rig, &claim);
    let buffer = readback(&rig, &claim, 32 * 32 * 4);
    let submission = rig
        .client
        .submit(&claim, |recording| {
            let color = [1.0, 0.0, 0.2, 0.6];
            recording
                .clear(
                    &image,
                    ImageLayout::Undefined,
                    color,
                    ImageLayout::TransferSrc,
                )
                .map_err(|_| RecordAborted)?;
            recording
                .read_back(
                    &image,
                    ImageLayout::TransferSrc,
                    &buffer,
                    ImageLayout::ShaderRead,
                )
                .map_err(|_| RecordAborted)
        })
        .expect("submits");
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    let key = buffer.origin.key;
    let needs = rig.device(|device| device.needs_invalidate(key));
    let before = rig.device(|device| device.faults().invalidations());
    let mut texels = vec![0u8; 32 * 32 * 4];
    buffer.read(0, &mut texels).expect("nothing runs");
    let invalidated = rig.device(|device| device.faults().invalidations()) - before;
    assert!(texels
        .chunks_exact(4)
        .all(|texel| texel == [255, 0, 51, 153]));
    if needs {
        assert!(
            invalidated > 0,
            "read without an invalidate of memory that is not coherent"
        );
    } else {
        assert_eq!(invalidated, 0, "coherent memory needs no invalidate");
        println!("HSK_GPU_NOTE readback_memory=host_coherent invalidate_path=not_exercised");
    }
    drop((image, buffer));
    assert_eq!(rig.owner.poll(), Ok(()));
    rig.settle(&claim, SettleOutcome::Completed)
        .expect("settles");
}

/// Which proof ends retired swapchains follows what the device was opened with: its own present fences only when
/// `VK_EXT_swapchain_maintenance1` is enabled, and swapchain memory is reported, as an estimate, beside the measured.
#[test]
fn a_presenting_owner_ends_retired_swapchains_by_what_its_device_enabled() {
    in_own_process!();
    let log = Arc::new(EventLog::default());
    let options = OwnerOptions { presentation: true };
    let owner = match GpuOwner::open_with(policy(QUOTA), options, log) {
        Ok(owner) => owner,
        Err(reason) => {
            assert!(
                !is_required(std::env::var_os(REQUIRE_GPU).as_deref()),
                "no device that presents ({})",
                reason.key()
            );
            println!("HSK_GPU_SKIP reason={}", reason.key());
            return;
        }
    };
    let (rule, fences) = {
        let slot = owner.shared.lock_slot();
        let device = &slot.live.as_ref().expect("bound").vulkan;
        let fences = device
            .identity()
            .extensions
            .contains(&"VK_EXT_swapchain_maintenance1");
        (device.retirement(), fences)
    };
    let expected = if fences {
        Retirement::PresentFences
    } else {
        Retirement::SuccessorPresent
    };
    assert_eq!(rule, Some(expected));
    println!("HSK_GPU_NOTE retirement={expected:?}");
    let usage = owner.memory_use().expect("bound");
    assert_eq!(
        (usage.swapchains, usage.swapchain_image_bytes),
        (0, ByteQuantity::Estimated(0))
    );
    assert!(owner.close().is_settled());
    drop(owner);
    // Without presentation there is nothing to retire.
    let Some(rig) = rig() else {
        panic!("the same device opens again");
    };
    assert_eq!(rig.device(|device| device.retirement()), None);
}

/// A frame as an acquire hands it out, around an owner image of the same client: the owner refuses it in a plain
/// submission before it records anything, so no swapchain is needed to show that.
fn frame_over(rig: &Rig, claim: &Claim, image: &OwnerImage) -> Frame {
    Frame {
        owner: Arc::clone(&rig.owner.shared),
        surface: SurfaceOrigin {
            ledger: image.origin.ledger,
            seat: image.origin.seat,
            claim: claim.info(),
            key: 0,
        },
        swapchain: 1,
        index: 0,
        semaphore: 0,
        attachment: RenderAttachment {
            usage: ImageUsage::OFFSCREEN,
            ..image.attachment
        },
        suboptimal: false,
        counters: Arc::new(PresentCounters::new()),
        // Not the owner's to abandon: it was never acquired.
        pending: false,
    }
}

#[test]
fn a_frame_is_refused_in_any_recording_but_its_own_present() {
    in_own_process!();
    let Some(rig) = rig() else {
        return;
    };
    let claim = rig.claim(16 << 20);
    let image = image(&rig, &claim);
    let frame = frame_over(&rig, &claim, &image);
    let submission = rig
        .client
        .submit(&claim, |recording| {
            let refused = Err(RecordRefusal::FrameNotPresented);
            let (from, to) = (ImageLayout::Undefined, ImageLayout::Present);
            assert_eq!(recording.clear(&frame, from, [0.0; 4], to), refused);
            assert_eq!(recording.transition(&frame, from, to), refused);
            // The image it wraps is the client's own and is recorded on as an image.
            recording
                .transition(&image, from, ImageLayout::General)
                .map_err(|_| RecordAborted)
        })
        .expect("submits");
    assert_eq!(rig.client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    drop((frame, image));
    assert_eq!(rig.owner.poll(), Ok(()));
    rig.settle(&claim, SettleOutcome::Completed)
        .expect("settles");
}
