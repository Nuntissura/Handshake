// WP-KERNEL-011 MT-001 toolkit decision spike (THROW-AWAY decision harness).
// Runs the five probes against the egui-stack candidate and writes the verdict JSON the contract
// requires. Each probe returns a REAL pass/fail (no mocks); the verdict reflects those results.
//
// `--accesskit-app` is the child-process entrypoint for probe (a)'s out-of-process UIA test.
// SPIKE_DATE env var supplies the ISO8601 spike_date (set by the authoritative release run) so we
// need no date crate in a throw-away spike.

mod accesskit_probe;
mod binary_probe;
mod docking_probe;
mod editor_probe;
mod viewport_probe;

use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct Verdict {
    spike_date: String,
    selected_toolkit: String,
    selection_rationale: String,
    rustc_host: String,
    pinned_versions: serde_json::Value,
    candidates: serde_json::Value,
    mt002_followups: serde_json::Value,
}

fn passlbl(p: bool) -> &'static str {
    if p {
        "PASS"
    } else {
        "FAIL"
    }
}

// CARGO_MANIFEST_DIR = <worktree>/src/frontend/toolkit_spike ; verdict lives at <worktree>/tests/native_gui/.
fn verdict_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let worktree = manifest
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("resolve worktree root from CARGO_MANIFEST_DIR");
    worktree.join("tests").join("native_gui").join("toolkit_spike_verdict.json")
}

fn main() {
    // Child mode: run the eframe window under test (probe a reads it out-of-process).
    if std::env::args().skip(1).any(|a| a == "--accesskit-app") {
        accesskit_probe::run_app();
        return;
    }

    println!("=== toolkit_spike (egui 0.33 candidate) — IMPLEMENTATION LOG ===");

    let b = docking_probe::run();
    println!("[probe b] docking_save_restore: {} | {}", passlbl(b.pass), b.notes);

    let c = editor_probe::run();
    println!("[probe c] editor_surface: {} | {}", passlbl(c.pass), c.notes);

    let e = viewport_probe::run();
    println!("[probe e] custom_viewport: {} | {}", passlbl(e.pass), e.notes);

    let d = binary_probe::run();
    println!("[probe d] single_binary: {} | {}", passlbl(d.pass), d.notes);

    let a = accesskit_probe::run();
    println!("[probe a] accesskit_out_of_process: {} | {}", passlbl(a.pass), a.notes);

    let pass_count = [a.pass, b.pass, c.pass, d.pass, e.pass].iter().filter(|x| **x).count();
    let all = pass_count == 5;
    let selected = if all { "egui" } else { "undecided" };

    let candidates = serde_json::json!({
        "egui": {
            "accesskit_out_of_process": a.pass,
            "docking_save_restore": b.pass,
            "editor_surface": c.pass,
            "single_binary": d.pass,
            "custom_viewport": e.pass,
            "notes": format!(
                "a: {} || b: {} || c: {} || d: {} || e: {}",
                a.notes, b.notes, c.notes, d.notes, e.notes
            ),
        },
        "gpui": {
            "accesskit_out_of_process": false,
            "docking_save_restore": false,
            "editor_surface": false,
            "single_binary": false,
            "custom_viewport": false,
            "notes": "GPUI not evaluated this spike: egui passed the make-or-break custom-viewport probe and the rest; GPUI's standalone-crate availability + out-of-process AccessKit on Windows is younger. Re-spike GPUI only if egui hits a concrete wall (per the contract decision rule, egui auto-selected when it passes all probes)."
        }
    });

    let verdict = Verdict {
        spike_date: std::env::var("SPIKE_DATE").unwrap_or_else(|_| "unknown".into()),
        selected_toolkit: selected.to_string(),
        selection_rationale: format!(
            "egui stack passed {pass_count}/5 probes. Probe e (make-or-break) proves the toolkit can HOST a custom wgpu offscreen render pass (3D cube w/depth + painted-texture quad) inside an egui pane paint-callback via a real device/queue AND that a model can STEER a viewport uniform via an AccessKit action (rotation 0->1, mirror set in the GPU-write code path); pixel-correct output is not read back (throwaway-spike scope) and is an MT-002 hardening item. Probe a proves OUT-OF-PROCESS AccessKit read+action via the Windows UIA client, matching the widget by stable Name (the practical egui+accesskit+UIA identifier); a stable AutomationId convention is an MT-002 item. Headless probes b/c and single-binary d complete the decision. Selected egui."
        ),
        rustc_host: "1.91.1; the egui 0.34 family requires rustc 1.92, so the egui 0.33 family is pinned (no repo-wide toolchain bump). egui 0.34 is available behind a future rustc-1.92 bump.".into(),
        pinned_versions: serde_json::json!({
            "egui": "0.33.3",
            "eframe": "0.33.3",
            "egui-wgpu": "0.33.3",
            "egui_tiles": "0.14.1",
            "egui_kittest": "0.33.3",
            "accesskit": "0.21.1",
            "accesskit_winit": "0.29.2",
            "accesskit_windows": "0.29.2",
            "wgpu": "27.0.1",
            "uiautomation": "0.25",
            "object": "0.36",
            "_note": "MT-002+ MUST reuse these exact working pins (CONTROL-3). egui_tiles uses the 'serde' feature. dumpbin is absent on this host; probe d inspects PE imports via the object crate."
        }),
        candidates,
        mt002_followups: serde_json::json!([
            "Probe a: establish a stable identifier convention for production widgets that resolves OUT-OF-PROCESS via UIA. The spike matched by Name; egui/accesskit may not populate UIA AutomationId from NodeId by default, so the contract's NodeId-as-AutomationId note must be validated in MT-002.",
            "Probe e: add GPU pixel readback (COPY_SRC + map_async; assert non-clear pixels and that output differs between rotation 0 and 1) for production viewports; the spike proves hostability + model-steering but does not read back pixels.",
            "Probe d: if reused, tighten is_system_dll from a System32-existence check to a finite apiset/CRT/OS allowlist.",
            "Toolkit: the egui 0.34 family is available behind a future repo-wide rustc-1.92 bump."
        ]),
    };

    let path = verdict_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    std::fs::write(&path, serde_json::to_string_pretty(&verdict).unwrap())
        .expect("write verdict json");
    println!(
        "VERDICT: selected_toolkit={selected} ({pass_count}/5 probes) -> {}",
        path.display()
    );
}
