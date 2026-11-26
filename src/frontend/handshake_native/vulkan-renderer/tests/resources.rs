//! Owner images and buffers on the machine's real Vulkan device, through the public surface: charged inside their claim,
//! cleared and read back to the host texel by texel, refused when they do not fit, and released once after their work.
//! Presentation is enabled only when the host asks for it.
//!
//! No window. A machine without an acceptable device fails; see `support/gpu_env.rs`. Cases that need work held pending
//! are in the crate's unit tests (`src/owner/resource_tests.rs`).

mod support;

use handshake_vulkan_renderer::{
    AccessRefusal, AttachmentFormat, AuthorizeRefusal, BufferMemory, BufferRequest, BufferUsage,
    ChargeRefusal, ClientId, Extent, GpuClient, GpuOwner, ImageLayout, ImageRequest, ImageUsage,
    ModuleId, NativeWindow, OwnerEvent, OwnerOptions, RecordAborted, RecordRefusal,
    ResourceRefusal, SettleOutcome, SettleRefusal, Settlement, SubmissionObservation,
    SubmissionState, SurfaceTarget, Unavailable, DESCRIPTOR, MAX_WAIT,
};
use std::num::NonZeroIsize;
use std::sync::Arc;
use support::gpu_env::{in_own_process, is_required, open, ran_in_its_own_process, REQUIRE_GPU};
use support::{claim_subject, policy, request, EventLog};

const QUOTA: u64 = 1 << 30;
const COMPLETED: SubmissionState = SubmissionState::Finished(SubmissionObservation::Completed);
const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;

fn attach(owner: &GpuOwner, module: u32) -> GpuClient {
    owner
        .attach_client(ModuleId(module), ClientId(1))
        .expect("free")
}

fn target(format: AttachmentFormat) -> ImageRequest {
    ImageRequest {
        format,
        extent: Extent {
            width: WIDTH,
            height: HEIGHT,
        },
        usage: ImageUsage::OFFSCREEN,
    }
}

fn readback(size: u64) -> BufferRequest {
    BufferRequest {
        size,
        usage: BufferUsage::READBACK,
        memory: BufferMemory::Readback,
    }
}

/// Bytes of the owner's own memory objects, over all heaps.
fn owner_bytes(owner: &GpuOwner) -> u64 {
    owner
        .memory_use()
        .expect("bound")
        .heaps
        .iter()
        .map(|heap| heap.bytes.measured().expect("measured"))
        .sum()
}

/// Clear `image` to `color` and copy it into `buffer` in one submission, wait for it, and read every byte.
fn clear_and_read(
    client: &GpuClient,
    claim: &handshake_vulkan_renderer::Claim,
    image: &handshake_vulkan_renderer::OwnerImage,
    buffer: &handshake_vulkan_renderer::OwnerBuffer,
    from: ImageLayout,
    color: [f32; 4],
) -> Vec<u8> {
    let submission = client
        .submit(claim, |recording| {
            recording
                .clear(image, from, color, ImageLayout::TransferSrc)
                .map_err(|_| RecordAborted)?;
            recording
                .read_back(
                    image,
                    ImageLayout::TransferSrc,
                    buffer,
                    ImageLayout::ShaderRead,
                )
                .map_err(|_| RecordAborted)
        })
        .expect("submits");
    assert_eq!(client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    let mut texels = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    buffer.read(0, &mut texels).expect("nothing runs any more");
    texels
}

#[test]
fn an_offscreen_image_reads_back_the_colour_it_was_cleared_to() {
    in_own_process!();
    let Some((owner, _log)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1);
    // Byte order in memory: R, G, B, A for RGBA, and B, G, R, A for BGRA. The components are multiples of 1/255.
    let cases = [
        (
            AttachmentFormat::Rgba8Unorm,
            [255u8, 0, 51, 153],
            [0u8, 255, 0, 255],
        ),
        (
            AttachmentFormat::Bgra8Unorm,
            [51u8, 0, 255, 153],
            [0u8, 255, 0, 255],
        ),
    ];
    for (format, first, second) in cases {
        let claim = client.admit(&request(64 << 20)).expect("fits");
        let image = client.create_image(&claim, &target(format)).expect("made");
        let bytes = u64::from(WIDTH * HEIGHT * format.texel_bytes());
        let buffer = client
            .create_buffer(&claim, &readback(bytes))
            .expect("made");
        let texels = clear_and_read(
            &client,
            &claim,
            &image,
            &buffer,
            ImageLayout::Undefined,
            [1.0, 0.0, 0.2, 0.6],
        );
        assert_eq!(texels.len(), (WIDTH * HEIGHT * 4) as usize);
        for (index, texel) in texels.chunks_exact(4).enumerate() {
            assert_eq!(texel, first, "{} texel {index}", format.key());
        }
        // A second clear shows that the host reads what the device wrote now, not what it wrote before.
        let texels = clear_and_read(
            &client,
            &claim,
            &image,
            &buffer,
            ImageLayout::ShaderRead,
            [0.0, 1.0, 0.0, 1.0],
        );
        assert!(
            texels.chunks_exact(4).all(|texel| texel == second),
            "{}",
            format.key()
        );
        let mut two = [0u8; 2];
        assert_eq!(
            buffer.read(bytes - 1, &mut two),
            Err(AccessRefusal::OutOfRange)
        );
        drop(image);
        drop(buffer);
        assert_eq!(client.poll(), Ok(()));
        assert_eq!(owner.usage().owned_resources, 0);
        let receipt = client
            .settle(&claim, Settlement::clean(SettleOutcome::Completed))
            .expect("settles");
        assert_eq!(receipt.released_bytes, 64 << 20);
    }
    assert_eq!(
        owner_bytes(&owner),
        0,
        "every memory object went back to the driver"
    );
    assert!(owner.close().is_settled());
}

#[test]
fn an_image_is_charged_inside_its_claim_and_freed_once_after_its_release() {
    in_own_process!();
    let Some((owner, log)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1);
    assert_eq!(owner_bytes(&owner), 0);
    let claim = client.admit(&request(64 << 20)).expect("fits");
    let request = ImageRequest {
        extent: Extent {
            width: 256,
            height: 256,
        },
        ..target(AttachmentFormat::Rgba8Unorm)
    };
    let image = client.create_image(&claim, &request).expect("made");
    let bytes = image.bytes();
    assert!(bytes >= 256 * 256 * 4, "{bytes}");
    let usage = owner.usage();
    assert_eq!((usage.owned_resources, usage.owned_bytes), (1, bytes));
    // The reservation is charged from admission on; the image lives inside it and changes nothing of the total.
    assert_eq!(usage.charged_bytes, 64 << 20);
    assert!(owner_bytes(&owner) >= bytes);
    let subject = claim_subject(claim.info());
    assert_eq!(log.count(OwnerEvent::ResourceMade { subject, bytes }), 1);
    let attachment = *image.attachment().expect("may use its device");
    assert_ne!(attachment.view, 0);
    assert_eq!(
        (attachment.extent, attachment.format),
        (request.extent, request.format)
    );
    assert_eq!(
        client.settle(&claim, Settlement::clean(SettleOutcome::Completed)),
        Err(SettleRefusal::ResourcesOutstanding { resources: 1 })
    );
    drop(image);
    assert_eq!(client.poll(), Ok(()));
    assert_eq!(owner.usage().owned_resources, 0);
    assert_eq!(owner_bytes(&owner), 0);
    assert_eq!(log.count(OwnerEvent::ResourceEnded { subject, bytes }), 1);
    assert_eq!(owner.counters().resources_ended, 1);
    // Looking again releases nothing twice.
    assert_eq!(owner.poll(), Ok(()));
    assert_eq!(
        log.count_like(OwnerEvent::ResourceEnded { subject, bytes }),
        1
    );
    let receipt = client
        .settle(&claim, Settlement::clean(SettleOutcome::Completed))
        .expect("settles");
    assert_eq!(receipt.released_bytes, 64 << 20);
    assert!(owner.close().is_settled());
}

#[test]
fn a_resource_that_is_not_allowed_or_does_not_fit_leaves_nothing_behind() {
    in_own_process!();
    let Some((owner, log)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1);
    let small = client.admit(&request(4096)).expect("fits");
    let refused = client.create_image(&small, &target(AttachmentFormat::Rgba8Unorm));
    assert!(
        matches!(
            refused.as_ref().err(),
            Some(ResourceRefusal::Charge(
                ChargeRefusal::ReservationExceeded {
                    reserved_bytes: 4096,
                    ..
                }
            ))
        ),
        "{refused:?}"
    );
    assert_eq!(
        owner_bytes(&owner),
        0,
        "the memory placed for it went back at once"
    );
    assert_eq!(owner.usage().owned_resources, 0);
    let zero = ImageRequest {
        extent: Extent {
            width: 0,
            height: 4,
        },
        ..target(AttachmentFormat::Rgba8Unorm)
    };
    assert_eq!(
        client.create_image(&small, &zero).err(),
        Some(ResourceRefusal::InvalidRequest)
    );
    let unused = ImageRequest {
        usage: ImageUsage::default(),
        ..target(AttachmentFormat::Rgba8Unorm)
    };
    assert_eq!(
        client.create_image(&small, &unused).err(),
        Some(ResourceRefusal::InvalidRequest)
    );
    assert_eq!(
        client.create_buffer(&small, &readback(0)).err(),
        Some(ResourceRefusal::InvalidRequest)
    );
    assert_eq!(
        log.count_like(OwnerEvent::ResourceRefused {
            subject: claim_subject(small.info()),
            reason: 0
        }),
        4
    );
    client
        .settle(&small, Settlement::clean(SettleOutcome::NoEffect))
        .expect("settles");
    assert_eq!(
        client.create_buffer(&small, &readback(16)).err(),
        Some(ResourceRefusal::NotAuthorized(AuthorizeRefusal::Settled))
    );
    assert!(owner.close().is_settled());
}

#[test]
fn buffers_round_trip_through_the_host_and_stay_with_their_client() {
    in_own_process!();
    let Some((owner, _log)) = open(policy(QUOTA)) else {
        return;
    };
    let client = attach(&owner, 1);
    let other = attach(&owner, 2);
    let claim = client.admit(&request(16 << 20)).expect("fits");
    let upload = BufferRequest {
        size: 4096,
        usage: BufferUsage {
            transfer_src: true,
            ..BufferUsage::default()
        },
        memory: BufferMemory::Upload,
    };
    let staging = client.create_buffer(&claim, &upload).expect("made");
    let written: Vec<u8> = (0..4096u32).map(|index| (index % 251) as u8).collect();
    staging.write(0, &written).expect("nothing runs");
    let mut read = vec![0u8; 4096];
    staging.read(0, &mut read).expect("nothing runs");
    assert_eq!(read, written);
    let device = BufferRequest {
        memory: BufferMemory::Device,
        ..upload
    };
    let local = client.create_buffer(&claim, &device).expect("made");
    // Device memory may or may not be visible to the host (it is on unified-memory devices); never silently both.
    assert!(matches!(
        local.read(0, &mut read[..16]),
        Ok(()) | Err(AccessRefusal::NotHostVisible)
    ));
    // Another client's image is not this client's to record on.
    let other_claim = other.admit(&request(16 << 20)).expect("fits");
    let foreign = other
        .create_image(&other_claim, &target(AttachmentFormat::Rgba8Unorm))
        .expect("made");
    let submission = client
        .submit(&claim, |recording| {
            assert_eq!(
                recording.clear(
                    &foreign,
                    ImageLayout::Undefined,
                    [0.0; 4],
                    ImageLayout::General
                ),
                Err(RecordRefusal::Foreign)
            );
            Ok(())
        })
        .expect("submits");
    assert_eq!(client.wait(&submission, MAX_WAIT), Ok(COMPLETED));
    drop((staging, local, foreign));
    assert_eq!(client.poll(), Ok(()));
    assert_eq!(owner.usage().owned_resources, 0);
    client
        .settle(&claim, Settlement::clean(SettleOutcome::Completed))
        .expect("settles");
    other
        .settle(&other_claim, Settlement::clean(SettleOutcome::Completed))
        .expect("settles");
    assert!(owner.close().is_settled());
}

/// A window that is never used: the owner refuses before it would ask for one.
struct NoWindow;

impl SurfaceTarget for NoWindow {
    fn native_window(&self) -> NativeWindow {
        let one = NonZeroIsize::new(1).expect("not zero");
        NativeWindow::Win32 {
            hwnd: one,
            hinstance: one,
        }
    }
}

#[test]
fn presentation_is_enabled_only_when_the_host_asks_for_it() {
    in_own_process!();
    let Some((owner, _log)) = open(policy(QUOTA)) else {
        return;
    };
    assert!(!owner.identity().extensions.contains(&"VK_KHR_swapchain"));
    let client = attach(&owner, 1);
    let claim = client.admit(&request(1 << 20)).expect("fits");
    assert_eq!(
        client.create_surface(&claim, Box::new(NoWindow)).err(),
        Some(ResourceRefusal::PresentationNotEnabled)
    );
    client
        .settle(&claim, Settlement::clean(SettleOutcome::NoEffect))
        .expect("settles");
    assert!(owner.close().is_settled());
    let options = OwnerOptions { presentation: true };
    let log = Arc::new(EventLog::default());
    match GpuOwner::open_with(policy(QUOTA), options, log) {
        Ok(presenting) => {
            let extensions = presenting.identity().extensions;
            assert!(extensions.contains(&"VK_KHR_swapchain"), "{extensions:?}");
            for extension in &extensions {
                assert!(
                    DESCRIPTOR.contains(&format!("\"{extension}\"")),
                    "{extension}"
                );
            }
            assert!(presenting.close().is_settled());
        }
        Err(Unavailable::PresentationUnsupported) => {
            assert!(
                !is_required(std::env::var_os(REQUIRE_GPU).as_deref()),
                "the device cannot present to windows of this platform"
            );
            println!("HSK_GPU_SKIP reason=presentation_unsupported");
        }
        Err(reason) => panic!("{}", reason.key()),
    }
}
