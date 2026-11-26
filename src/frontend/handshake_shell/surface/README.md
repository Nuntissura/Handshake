# handshake_shell_surface

Shell windows presenting real Vulkan frames (WP-KERNEL-012 MT-214 slice 2, the surface crate of AC214-12). It composes
`handshake_shell_window` (windows, the event loop) with the central GPU owner `handshake-vulkan-renderer` (MT-203);
neither of them depends on the other. No window library, Vulkan binding or Win32 type is in this crate's code or
interface, and no egui, eframe, wgpu or glow in its closure. Windows x64 first; on other systems only the frame, the
readback and the watch compile.

The contract is in the crate documentation (`src/lib.rs`): how a host uses it, frames, swapchains, present progress,
headless frames, failure modes.

## Files

| Path | Holds |
| --- | --- |
| `src/target.rs` | `ShellWindowTarget`: the window crate's `NativeWindowToken` as the owner's `SurfaceTarget`, the one place the two meet |
| `src/clear.rs` | the frame (theme background, clear values per format), its expected texels, the texel-by-texel comparison |
| `src/readback.rs` | the frame rendered into an owner image and read back; captures of presented swapchain images |
| `src/decide.rs` | decisions without a window or a device: events to steps, frame results to what follows, the timer schedule |
| `src/slot.rs` | one window's client, claim, surface and swapchain; the GPU calls |
| `src/host.rs` | `PresentHost`, the window crate's host: when the calls are made, its one wake lane |
| `src/host/ending.rs` | closing a window, settling the claims of windows that are gone, shutdown |
| `src/shell.rs` | `PresentConfig`, `ShellApp`, `Shell`, `PresentReport`, `WindowPresentation` |
| `src/watch.rs` | `PresentWatch`: present progress for a watcher on any thread (MT-215 HB-4) |
| `src/error.rs` | `PresentError`, closed codes |
| `examples/first_frame.rs` | one quiet window on the Operator's desktop, by hand only |

## What draws a frame

Damage-driven (AC214-4). Nothing is drawn unless the window crate hands out a frame, and it does only when asked.

| Cause | Draws | Swapchain |
| --- | --- | --- |
| The operating system exposes the window (first show, uncovered) | yes | as it is |
| `Shell::damage`, `Shell::capture_next` | yes | as it is |
| `WindowEvent::Resized` (not reported while minimised) | yes | configured again before the frame, old one as `oldSwapchain` |
| `DisplayChanged` to minimised | no | suspended at once: no acquire, the old one kept for the next configuration |
| `DisplayChanged` back to normal or maximised | yes | configured again (restore recreates) |
| `ScaleChanged` | yes | as it is; the size change that follows is a resize |
| Present suboptimal | yes, at once, twice in a row at most | configured again |
| Present or acquire out of date, frame recording refused | yes, at once (8 failures in a row stop it) | configured again |
| Surface lost | yes, at once | a new surface on a new token |
| Acquire not ready (timeout 0) | on the presenter's timer, 4 ms later (250 in a row stop it) | as it is |
| A presented swapchain was replaced, without `VK_EXT_swapchain_maintenance1` | up to 2 x images + 2 frames, 16 ms apart, until the owner ended the old one | as it is |
| Input, focus, moves, monitor changes, a close request | no | as it is |
| A window that presented is closed, without `VK_EXT_swapchain_maintenance1` | one frame, before the hide | configured again once, then its images acquired back until the owner proved its presents ended |

An idle window submits no GPU work. The presenter's timer also lets the owner look (`GpuOwner::poll`, without waiting)
while old swapchains, captures or closing windows wait for it: 8 ms doubling to 512 ms, 24 looks in a row at most.

## Recreation and retirement, as implemented

- Configuration only through `OwnerSurface::configure`, with the window's client size; on Win32 the surface extent is
  the window's, and (0, 0) when minimised, which suspends (Vulkan specification, WSI, Win32 Platform).
- The owner destroys a replaced swapchain once (VUID-vkDestroySwapchainKHR-swapchain-01282), after its work and
  presents are proven ended by its own surface. The presenter never destroys or waits for anything itself.
- Close: the window is hidden (`WindowCommand::Hide`), the surface released, the claim settled with what the owner
  observed, and the window destroyed once its native token count is 0, which is when the owner dropped the target. The
  target counts its own drop: `PresentReport::targets_released`. Without the extension, a surface that presented first
  proves its presents ended (MT-214 SF-2): a new swapchain and one present on it before the hide, then the presenter's
  looks call `OwnerSurface::prove_presents` until the image of that present came back, which ends the old swapchain by
  its successor's present and the new one by its own image; then the surface is released. No wait for the device; a
  proof that does not come within `PROVE_WINDOW` (2 s) releases the surface all the same, which then keeps its last
  swapchain, target and window until the owner closes, counted in `PresentReport::closes_unproven`.
- Shutdown (`Shell::shutdown`): after the application heard `ShutdownBegan`, every surface is released, claims settled,
  the owner closed (bounded by its own deadline), claims still open let go, and `Control::finish` tried until no target
  is held.
- Nothing on the frame path waits: acquire with timeout 0, present without waits. `vkDeviceWaitIdle` is not called
  anywhere in the owner or here.

## Readback

`render_offscreen` clears an owner image of the swapchain's format to the same frame, copies it to a readback buffer in
the same submission, and `PendingReadback::poll` hands it over once the owner observed the submission completed.
`Shell::capture_next` reads back the next presented swapchain image in its own present submission (clear to
`TRANSFER_SRC_OPTIMAL`, copy, then `PRESENT_SRC_KHR`). `FrameReadback::compare` checks every texel within one step per
channel, the rounding the Vulkan specification allows for a float to normalized conversion.

## Failure codes

`PresentError::code` (`surface.*`), `ReadbackError::code` (`readback.*`); the owner's own keys inside through
`PresentError::owner_key`. None carries a title, a path or a driver string.

## Tests

`cargo test --locked -p handshake_shell_surface` from `src/frontend/handshake_shell` (validator only). Every GPU test runs
in a process of its own (the owner's `gpu_env`, with its deadline, its device rule and its cleanup of the Direct3D shader
cache); a machine without a Vulkan device fails them unless `HANDSHAKE_REQUIRE_VULKAN_GPU=0`. Every window is quiet and
lives on a private desktop that receives no input, under the window crate's own watches (`window/tests/support`); no
test touches the Operator's desktop, focus or clipboard, and none runs the example.

| Target | Rule |
| --- | --- |
| unit tests | expected texels per format and channel order, sRGB and half-float conversion, the comparison and its control; events to steps (idle events draw nothing); frame results to what follows; bounded retirement burst and looks; the schedule; the watch |
| `headless` | the frame in every owner format at three sizes, read back and compared texel by texel; another background differs in every texel; the claim settles |
| `present_cases` | the owner's eight presentation cases with real windows: present loop and counts, a resize ending the old swapchain once, minimise and restore, a dropped frame, a surface proven before release letting its window go before the close (SF-2), a released surface letting its window go once, multiwindow retirement, a FIFO-blocked present not stalling another window's acquire |
| `frame_loop` | an idle window submits nothing for 500 ms, nor for 500 ms more while input, focus and move messages reach it on the real loop (SF-1), and one damage is one frame and one submission; the captured presented frame matches texel by texel; resize, minimise, restore and a scale change each recreate, nothing is drawn while minimised, every replaced swapchain ends; closing one of two windows releases its target once and destroys it before shutdown, the other keeps presenting; six windows opened and closed one after another leave no swapchain or target behind (SF-2) |
| `source_scan` | no window library, binding or `unsafe` in the code; no focus, cursor or input call anywhere; every loop on a private desktop; the example gated by its flag and never named by a test; no banned crate locked |

Not provable here: presentation on a desktop the Operator sees (a private desktop is never composed on screen), real
monitors and DPI, occlusion, device loss and rebind (AC214-9, a later slice), the unavailable-device screen, frame
pacing under load. Those need native x64 and a real desktop.

## The example

```text
cargo run --manifest-path src/frontend/handshake_shell/Cargo.toml -p handshake_shell_surface --example first_frame -- --show-on-my-desktop
```

Only by hand. Without the flag it opens nothing. The window is quiet: no foreground, no focus, no taskbar button, at the
bottom of the Z order. It closes after 30 s (`--seconds N`) or with its close button, and prints `HSK_SHELL_*` receipt
lines: device, swapchain (format, present mode, images, extent), the readback comparison of the presented frame, the
counters. It is not the product: no `device.gpu.shell`, no kernel issuer (MT-214 S5).
