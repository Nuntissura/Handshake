//! Presentation cases that need real windows. This crate creates no window, so it only compiles them (with every test
//! target, as part of `support`); the window host runs them with windows of its own: MT-214's window crate, or the
//! eval's harness. A host includes this file with `#[path = "<this crate>/tests/support/present_cases.rs"] mod present_cases;`,
//! implements [`HostWindow`], opens the owner with `OwnerOptions { presentation: true }` and calls each case. Every case
//! uses only the public surface and panics with what it found.
//!
//! What each case proves, and the rule it can fail on:
//! - [`frames_are_presented_and_counted`]: acquire, record, present; the probe counts what the queue took.
//! - [`a_resize_ends_the_old_swapchain_once`]: configure on resize retires the old swapchain, which ends by its own
//!   surface's proof (S2-F2).
//! - [`minimise_suspends_and_restore_resumes`]: no swapchain of no size; acquire refused while suspended.
//! - [`a_dropped_frame_needs_a_new_configuration`]: the image stays acquired, the swapchain is out of date.
//! - [`a_surface_proven_before_release_lets_its_window_go`]: one present on a new swapchain whose image comes back
//!   ends the old one and the new one, and the window is let go before the close (MT-214 SF-2).
//! - [`a_released_surface_lets_its_window_go`]: the target is dropped once the surface ended, which without
//!   `VK_EXT_swapchain_maintenance1` is at the close for a surface that presented and was not proven.
//! - [`another_windows_presents_never_end_this_ones_swapchain`]: the multiwindow retirement rule (S2-F2).
//! - [`a_blocked_present_does_not_stall_another_windows_acquire`]: one lock per surface (S2-F5).
#![allow(dead_code)]

use handshake_vulkan_renderer::{
    AttachmentFormat, AuthorizationProvenance, Claim, ClaimRequest, Extent, Frame, GpuClient,
    GpuOwner, ImageLayout, OperationId, OperationKind, OwnerSurface, PresentMode, PresentRefusal,
    PresentsProof, RecordRefusal, Recording, SurfaceTarget, SwapchainRequest, SwapchainStatus,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A window of the host's, driven by the case.
pub trait HostWindow: Sync {
    /// A target that keeps the window alive for as long as it exists.
    fn target(&self) -> Box<dyn SurfaceTarget>;
    /// The window's client size now.
    fn size(&self) -> Extent;
    /// Change the window, and run its messages until the change is what Vulkan reports for its surface.
    fn resize(&self, to: Extent);
    fn minimise(&self);
    fn restore(&self);
    /// Run the window's messages once. Called from another thread than the window's, it does nothing.
    fn pump(&self);
}

/// How often a case asks again for what it waits for, and how long it waits between.
const TRIES: u32 = 600;
const PAUSE: Duration = Duration::from_millis(5);

fn display(client: &GpuClient) -> Claim {
    let request = ClaimRequest {
        operation: OperationKind::Display,
        operation_id: OperationId(1),
        reserve_bytes: 256 << 20,
        provenance: AuthorizationProvenance([1; 32]),
    };
    client.admit(&request).expect("a display claim fits")
}

fn ready(surface: &OwnerSurface, window: &dyn HostWindow) -> u64 {
    let request = SwapchainRequest {
        extent: window.size(),
        format: AttachmentFormat::Bgra8Unorm,
        mode: PresentMode::Fifo,
    };
    match surface.configure(&request).expect("configured") {
        SwapchainStatus::Ready(info) => info.swapchain,
        SwapchainStatus::Suspended => panic!("a window with a size suspended its swapchain"),
    }
}

/// The frame the next acquire hands out, asking again while none is ready.
fn acquire(surface: &OwnerSurface, window: &dyn HostWindow) -> Frame {
    for _ in 0..TRIES {
        match surface.acquire() {
            Ok(frame) => return frame,
            Err(PresentRefusal::NotReady) => {
                window.pump();
                std::thread::sleep(PAUSE);
            }
            Err(refusal) => panic!("acquire: {}", refusal.key()),
        }
    }
    panic!("no image became ready")
}

/// Fill the frame and leave it for the present.
fn draw(recording: &Recording<'_>, frame: &Frame) -> Result<(), RecordRefusal> {
    if frame.attachment().usage.transfer_dst {
        recording.clear(
            frame,
            ImageLayout::Undefined,
            [0.1, 0.4, 0.8, 1.0],
            ImageLayout::Present,
        )
    } else {
        recording.transition(frame, ImageLayout::Undefined, ImageLayout::Present)
    }
}

/// Acquire, draw and present one frame.
fn present(client: &GpuClient, claim: &Claim, surface: &OwnerSurface, window: &dyn HostWindow) {
    let frame = acquire(surface, window);
    client
        .present(claim, frame, |recording, frame| {
            draw(recording, frame).map_err(|_| handshake_vulkan_renderer::RecordAborted)
        })
        .expect("presented");
    window.pump();
}

/// Swapchains of the bound generation that exist now.
fn swapchains(owner: &GpuOwner) -> u64 {
    owner.memory_use().expect("bound").swapchains
}

/// Do `step` until `expected` swapchains exist: the owner destroyed the retired ones the case waits for. Counts are
/// taken relative to the start of each case, since a case before may have left swapchains that end only at the close.
fn present_until(owner: &GpuOwner, expected: u64, mut step: impl FnMut()) {
    for _ in 0..TRIES {
        let _ = owner.poll();
        if swapchains(owner) == expected {
            return;
        }
        step();
    }
    panic!(
        "{} swapchains still exist, {expected} expected",
        swapchains(owner)
    );
}

fn has_present_fences(owner: &GpuOwner) -> bool {
    owner
        .identity()
        .extensions
        .contains(&"VK_EXT_swapchain_maintenance1")
}

pub fn frames_are_presented_and_counted(client: &GpuClient, window: &dyn HostWindow) {
    let claim = display(client);
    let surface = client
        .create_surface(&claim, window.target())
        .expect("surface");
    ready(&surface, window);
    for _ in 0..30 {
        present(client, &claim, &surface, window);
    }
    let progress = surface.probe().read();
    assert_eq!(
        (progress.presented, progress.failed, progress.abandoned),
        (30, 0, 0)
    );
    assert!(progress.acquired >= 30);
    assert!(progress.since_last_present.is_some());
}

pub fn a_resize_ends_the_old_swapchain_once(
    owner: &GpuOwner,
    client: &GpuClient,
    window: &dyn HostWindow,
) {
    let base = swapchains(owner);
    let claim = display(client);
    let surface = client
        .create_surface(&claim, window.target())
        .expect("surface");
    let first = ready(&surface, window);
    present(client, &claim, &surface, window);
    let size = window.size();
    window.resize(Extent {
        width: size.width / 2 + 1,
        height: size.height / 2 + 1,
    });
    let second = ready(&surface, window);
    assert!(second > first, "a new swapchain");
    // The old one waits for its own proof: the new one's presents, or its own present fences.
    present_until(owner, base + 1, || {
        present(client, &claim, &surface, window)
    });
    assert_eq!(surface.swapchain().map(|info| info.swapchain), Some(second));
}

pub fn minimise_suspends_and_restore_resumes(client: &GpuClient, window: &dyn HostWindow) {
    let claim = display(client);
    let surface = client
        .create_surface(&claim, window.target())
        .expect("surface");
    ready(&surface, window);
    present(client, &claim, &surface, window);
    window.minimise();
    let request = SwapchainRequest {
        extent: window.size(),
        format: AttachmentFormat::Bgra8Unorm,
        mode: PresentMode::Fifo,
    };
    assert_eq!(surface.configure(&request), Ok(SwapchainStatus::Suspended));
    assert_eq!(surface.acquire().err(), Some(PresentRefusal::Suspended));
    assert_eq!(surface.swapchain(), None);
    window.restore();
    ready(&surface, window);
    present(client, &claim, &surface, window);
}

pub fn a_dropped_frame_needs_a_new_configuration(client: &GpuClient, window: &dyn HostWindow) {
    let claim = display(client);
    let surface = client
        .create_surface(&claim, window.target())
        .expect("surface");
    ready(&surface, window);
    drop(acquire(&surface, window));
    assert_eq!(surface.acquire().err(), Some(PresentRefusal::OutOfDate));
    ready(&surface, window);
    present(client, &claim, &surface, window);
    assert_eq!(surface.probe().read().abandoned, 1);
}

/// Counts the drops of the targets it wraps.
struct Counted {
    inner: Box<dyn SurfaceTarget>,
    dropped: Arc<AtomicU64>,
}

impl SurfaceTarget for Counted {
    fn native_window(&self) -> handshake_vulkan_renderer::NativeWindow {
        self.inner.native_window()
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

/// MT-214 SF-2: a surface that presented, proven before it is released, lets its window go then and not with the
/// device. One present on a new swapchain, whose image comes back to `prove_presents`, ends the old swapchain (its
/// successor's present is done) and the new one (its only present is). With `VK_EXT_swapchain_maintenance1` the present
/// fences prove the same.
pub fn a_surface_proven_before_release_lets_its_window_go(
    owner: &GpuOwner,
    client: &GpuClient,
    window: &dyn HostWindow,
) {
    let base = swapchains(owner);
    let claim = display(client);
    let dropped = Arc::new(AtomicU64::new(0));
    let target = Counted {
        inner: window.target(),
        dropped: Arc::clone(&dropped),
    };
    let surface = client
        .create_surface(&claim, Box::new(target))
        .expect("surface");
    ready(&surface, window);
    for _ in 0..3 {
        present(client, &claim, &surface, window);
    }
    ready(&surface, window);
    if !has_present_fences(owner) {
        assert_eq!(
            surface.prove_presents(),
            Ok(PresentsProof::NeedsPresent),
            "the old swapchain waits for a present of the new one"
        );
    }
    present(client, &claim, &surface, window);
    let mut proven = false;
    for _ in 0..TRIES {
        match surface.prove_presents().expect("proving") {
            PresentsProof::Proven => {
                proven = true;
                break;
            }
            PresentsProof::Waiting => {
                window.pump();
                std::thread::sleep(PAUSE);
            }
            PresentsProof::NeedsPresent => panic!("the new swapchain was presented"),
        }
    }
    assert!(proven, "the image of the last present came back");
    // The old swapchain ends once its work has, too.
    present_until(owner, base + 1, || std::thread::sleep(PAUSE));
    if !has_present_fences(owner) {
        assert_eq!(
            surface.acquire().err(),
            Some(PresentRefusal::OutOfDate),
            "images acquired back are no frames"
        );
    }
    drop(surface);
    present_until(owner, base, || std::thread::sleep(PAUSE));
    assert_eq!(
        dropped.load(Ordering::SeqCst),
        1,
        "the window is let go once its surface ended, before the close"
    );
}

/// Closes `owner`: run it last.
pub fn a_released_surface_lets_its_window_go(
    owner: &GpuOwner,
    client: &GpuClient,
    window: &dyn HostWindow,
) {
    let base = swapchains(owner);
    let claim = display(client);
    let dropped = Arc::new(AtomicU64::new(0));
    let target = Counted {
        inner: window.target(),
        dropped: Arc::clone(&dropped),
    };
    let surface = client
        .create_surface(&claim, Box::new(target))
        .expect("surface");
    ready(&surface, window);
    present(client, &claim, &surface, window);
    drop(surface);
    if has_present_fences(owner) {
        present_until(owner, base, || std::thread::sleep(PAUSE));
        assert_eq!(
            dropped.load(Ordering::SeqCst),
            1,
            "the window is let go once its surface ended"
        );
    } else {
        // Nothing proves the presents of its last swapchain: it stays until the device is destroyed.
        for _ in 0..20 {
            let _ = owner.poll();
            std::thread::sleep(PAUSE);
        }
        assert_eq!(swapchains(owner), base + 1);
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        let _ = owner.close();
        assert_eq!(
            dropped.load(Ordering::SeqCst),
            1,
            "the window is let go with the device"
        );
    }
}

pub fn another_windows_presents_never_end_this_ones_swapchain(
    owner: &GpuOwner,
    client: &GpuClient,
    a: &dyn HostWindow,
    b: &dyn HostWindow,
) {
    let base = swapchains(owner);
    let claim = display(client);
    let first = client.create_surface(&claim, a.target()).expect("surface");
    let other = client.create_surface(&claim, b.target()).expect("surface");
    ready(&first, a);
    ready(&other, b);
    present(client, &claim, &first, a);
    present(client, &claim, &other, b);
    let size = a.size();
    a.resize(Extent {
        width: size.width / 2 + 1,
        height: size.height / 2 + 1,
    });
    ready(&first, a);
    if !has_present_fences(owner) {
        // A's old swapchain is presented and its successor is not: B's presents, however many, prove nothing of A.
        for _ in 0..60 {
            present(client, &claim, &other, b);
            let _ = owner.poll();
        }
        assert_eq!(
            swapchains(owner),
            base + 3,
            "another window's presents ended this one's swapchain"
        );
    }
    present_until(owner, base + 2, || present(client, &claim, &first, a));
}

/// Presents on `a` from another thread, FIFO, as fast as the queue takes them, while this thread acquires on `b`: no
/// acquire on `b` waits for `a`'s presents. A FIFO present may block until the next vertical blank.
pub fn a_blocked_present_does_not_stall_another_windows_acquire(
    client: &GpuClient,
    a: &dyn HostWindow,
    b: &dyn HostWindow,
) {
    let claim = display(client);
    let first = client.create_surface(&claim, a.target()).expect("surface");
    let other = client.create_surface(&claim, b.target()).expect("surface");
    ready(&first, a);
    ready(&other, b);
    let slowest = std::thread::scope(|scope| {
        let presenting = scope.spawn(|| {
            for _ in 0..120 {
                present(client, &claim, &first, a);
            }
        });
        let mut slowest = Duration::ZERO;
        while !presenting.is_finished() {
            let start = Instant::now();
            match other.acquire() {
                Ok(frame) => {
                    slowest = slowest.max(start.elapsed());
                    client
                        .present(&claim, frame, |recording, frame| {
                            draw(recording, frame)
                                .map_err(|_| handshake_vulkan_renderer::RecordAborted)
                        })
                        .expect("presented");
                }
                Err(PresentRefusal::NotReady) => slowest = slowest.max(start.elapsed()),
                Err(refusal) => panic!("acquire: {}", refusal.key()),
            }
            b.pump();
        }
        presenting.join().expect("presents");
        slowest
    });
    // A vertical blank is at most about 33 ms at 30 Hz; an acquire that waited for one would show it.
    assert!(
        slowest < Duration::from_millis(10),
        "an acquire waited {slowest:?}"
    );
}
