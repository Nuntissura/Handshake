---
file_id: stage-rw-022-high-volume-tab-cpu-memory-spike
file_kind: reference-research-note
updated_at: "2026-07-20"
research_workstream: STAGE-RW-022
verification_status: harness-built-and-run-core-scaling-claim-empirically-validated-storm-gpu-and-ui-cost-open
---

<topic id="stage-rw-022-high-volume-tab-cpu-memory-spike" status="hardened" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-20">

# High-volume tab CPU/memory core spike (3000+ tabs without taxing CPU)

## Question

Stage's defining requirement (`STAGE-DEC-009`, `STAGE-DEC-020`): can a window hold 3000+ tabs where CPU and memory scale with the small live working set, not the saved-tab count — and stay light enough to run alongside Handshake's heavy creative tools (ComfyUI, Photoshop/Illustrator/Marvelous-Designer-class editors)?

## What was built and run

A Rust harness (`tab_scale` binary in the same spike project, `C:\Handshake_Stage_Spike`) builds N durable tab records (metadata-only structs: id, url, title, folder, favicon host) plus K live WebView2 renderers in ONE host process sharing ONE WebView2 environment — the real Stage model. It navigates the K renderers, then idles while an external PowerShell sampler measures the host process plus all its `msedgewebview2.exe` children for RAM (working set) and idle CPU over a 5s window.

## Live baseline: the operator's current Firefox (measured 2026-07-20)

The operator's Firefox was open with ~3000 real tabs on a 128 GB / 32-core machine:

- Firefox: 16 processes, **3.88 GB RAM**, ~0.1% idle CPU at sample time (Firefox already lazy-unloads tabs, so its idle CPU is low; its cost is dominated by memory).

## Results — light page (example.com), sweep

| records | live renderers | webview2 procs | RAM | idle CPU |
|--------:|---------------:|---------------:|----:|---------:|
| 3000 | 0 | 0 | 22 MB | 0.01% |
| 3000 | 3 | 8 | 461 MB | 0.01% |
| 3000 | 10 | 15 | 846 MB | 0.01% |
| 10 | 3 | 8 | 444 MB | 0.03% |
| 10000 | 3 | 8 | 451 MB | 0.04% |

## Results — real YouTube pages (representative of the operator's tabs)

| records | live renderers | webview2 procs | RAM | idle CPU |
|--------:|---------------:|---------------:|----:|---------:|
| 3000 | 3 | 11 | 1264 MB | 0.22% |
| 3000 | 5 | 12 | 1729 MB | 0.38% |

## Findings

1. **Saved tabs are nearly free.** 3000 records with 0 live renderers = 22 MB and ~0% CPU. 10000 records added only a few MB over 3000. The durable-record cost does not meaningfully scale.
2. **Memory tracks the live renderer count K, not the record count N.** At a fixed 3 live renderers, RAM is essentially flat (~450 MB light / ~1.26 GB YouTube) whether 10, 3000, or 10000 tabs are held. Each additional live YouTube renderer costs ~230 MB.
3. **The footprint is operator-capped.** Because cost tracks K, a live-renderer ceiling is a hard cap on Stage's browser footprint regardless of how many tabs are saved. This is the mechanism that leaves headroom for the heavy creative tools.
4. **Idle CPU is negligible** in every configuration (0.01–0.38%), even with live YouTube renderers (they throttle when unfocused).
5. **vs Firefox:** for the same 3000 tabs, the Stage model with 3 live YouTube renderers uses **1.26 GB vs Firefox's 3.88 GB** (~3× less; ~2.6 GB more free for ComfyUI/editors), and with tabs unloaded to records the overhead is ~22 MB.

## Verdict

The core "3000+ tabs without taxing CPU" architecture is **empirically validated** at the resource level: durable records are ~free, live cost is bounded by and tracks the renderer ceiling K, idle CPU is ~0, and the steady-state footprint is a fraction of Firefox's while remaining operator-controllable. This is the load-headroom property Stage needs to coexist with Handshake's heavy tools.

## Honest caveats / not yet measured (increment 2)

- **Load-storm CPU:** this spike measures steady-state idle after load. It does NOT measure the CPU/IO spike of restoring or waking many tabs at once. STAGE-RW-011's staggered-restore contract addresses this; it needs its own measurement.
- **GPU:** not measured. The operator's "heavy load" likely includes GPU compositing; a real Stage must budget GPU against ComfyUI/editor GPU use.
- **Virtualized-sidebar UI cost:** the 3000 records here are in-memory structs, not a rendered egui list. The 22 MB proves the DATA cost is trivial; the per-frame cost of drawing 3000–10000 rows in a virtualized egui sidebar (`ScrollArea::show_rows`) is a separate proof still to run.
- **Per-renderer numbers** are for the anonymous youtube.com homepage; a playing-video or logged-in watch renderer is heavier, but still bounded by K.

## Recommended next step

Increment 2: a headless egui benchmark timing per-frame layout/update cost for a virtualized tab sidebar at 100 / 1000 / 3000 / 10000 rows (proving UI cost stays flat), plus a load-storm measurement (stagger vs simultaneous restore of K renderers). Together with this note, that closes the core-architecture proof.

## Artifacts

- Harness: `C:\Handshake_Stage_Spike\webview2-youtube-auth-spike\src\bin\tab_scale.rs`
- Sweep data: `C:\Handshake_Stage_Spike\out\tab-scale-sweep.json`
- Samplers: `C:\Handshake_Stage_Spike\sweep.ps1`, `yt-sweep.ps1`

</topic>
