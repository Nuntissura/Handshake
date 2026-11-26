//! The shell's frame without a window: rendered into an owner image of every format the owner offers, read back and
//! compared texel by texel with what the frame says, on the machine's real device and in a process of its own. This
//! is the proof path of Argus and visual tests; it needs no presentation and no desktop.

mod support;

use std::time::{Duration, Instant};

use handshake_shell_surface::{render_offscreen, Background, ClearFrame, ReadbackError};
use handshake_vulkan_renderer::{
    AttachmentFormat, AuthorizationProvenance, ClaimRequest, ClientId, Extent, ModuleId,
    OperationId, OperationKind, ResourceRefusal, SettleOutcome, SettleRefusal, Settlement,
};

use support::gpu_env::{in_own_process, ran_in_its_own_process};

const SIZES: [Extent; 3] = [
    Extent {
        width: 1,
        height: 1,
    },
    Extent {
        width: 64,
        height: 48,
    },
    // Odd, and wider than tall: a row or column mix-up is found out.
    Extent {
        width: 333,
        height: 17,
    },
];

#[test]
fn the_frame_reads_back_texel_for_texel_in_every_format() {
    in_own_process!();
    let Some((owner, _log)) = support::open_owner(false) else {
        return;
    };
    let client = owner
        .attach_client(ModuleId(214), ClientId(7))
        .expect("a client");
    let request = ClaimRequest {
        operation: OperationKind::Readback,
        operation_id: OperationId(1),
        reserve_bytes: 64 << 20,
        provenance: AuthorizationProvenance([2; 32]),
    };
    let claim = client.admit(&request).expect("a readback claim fits");
    let frame = ClearFrame::new(Background::DARK);
    let other = ClearFrame::new(Background::LIGHT);
    let mut checked = Vec::new();
    for format in AttachmentFormat::ALL {
        for extent in SIZES {
            let pending = match render_offscreen(&client, &claim, &frame, format, extent) {
                Ok(pending) => pending,
                // The 8-bit UNORM formats are mandatory for these uses; the others are the device's choice.
                Err(ReadbackError::Resource(ResourceRefusal::FormatUnsupported))
                    if !matches!(
                        format,
                        AttachmentFormat::Rgba8Unorm | AttachmentFormat::Bgra8Unorm
                    ) =>
                {
                    println!("HSK_SHELL_SKIP format={}", format.key());
                    break;
                }
                Err(error) => panic!("{} {extent:?}: {}", format.key(), error.code()),
            };
            let readback = pending
                .wait(&client, Duration::from_secs(10))
                .unwrap_or_else(|error| panic!("{} {extent:?}: {}", format.key(), error.code()));
            assert_eq!((readback.format, readback.extent), (format, extent));
            let comparison = readback.compare(&frame).expect("compared");
            assert!(comparison.matches(), "{} {extent:?}: {comparison:?}", format.key());
            // The control: the same readback against another background differs in every texel.
            let against_other = readback.compare(&other).expect("compared");
            assert_eq!(against_other.mismatched, extent.texels(), "{}", format.key());
            checked.push((format, extent));
        }
    }
    for format in [AttachmentFormat::Rgba8Unorm, AttachmentFormat::Bgra8Unorm] {
        let sizes = checked.iter().filter(|(seen, _)| *seen == format).count();
        assert_eq!(sizes, SIZES.len(), "{} at every size", format.key());
    }
    // The images and buffers went with their readbacks; the owner destroys them after their work, then the claim
    // settles with what it observed.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let _ = client.poll();
        match client.settle(&claim, Settlement::clean(SettleOutcome::Completed)) {
            Ok(receipt) => {
                assert!(receipt.observed.completed >= checked.len() as u64);
                break;
            }
            Err(SettleRefusal::InFlight { .. } | SettleRefusal::ResourcesOutstanding { .. })
                if Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(refusal) => panic!("the readback claim did not settle: {}", refusal.key()),
        }
    }
    println!(
        "HSK_SHELL_HEADLESS device={} checked={}",
        owner.identity().name,
        checked.len()
    );
}
