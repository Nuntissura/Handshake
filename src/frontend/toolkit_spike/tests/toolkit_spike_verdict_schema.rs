// Contract proof_target: `cargo test --test toolkit_spike_verdict_schema`.
// Loads the verdict JSON written by the spike runner and validates every required key + type
// (including the make-or-break custom_viewport field). Run the spike first so the file exists.

use std::path::PathBuf;

fn verdict_path() -> PathBuf {
    // CARGO_MANIFEST_DIR = <worktree>/src/frontend/toolkit_spike
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let worktree = manifest
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("worktree root");
    worktree.join("tests").join("native_gui").join("toolkit_spike_verdict.json")
}

#[test]
fn verdict_schema_valid() {
    let path = verdict_path();
    let data = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "verdict json not found at {} ({e}); run `cargo run --release --bin toolkit_spike` first",
            path.display()
        )
    });
    let v: serde_json::Value = serde_json::from_str(&data).expect("verdict must be valid JSON");

    assert!(v["spike_date"].is_string(), "spike_date must be a string");
    assert!(v["selected_toolkit"].is_string(), "selected_toolkit must be a string");
    assert!(v["selection_rationale"].is_string(), "selection_rationale must be a string");

    let sel = v["selected_toolkit"].as_str().unwrap();
    assert!(
        sel == "egui" || sel == "gpui" || sel == "undecided",
        "selected_toolkit must be egui|gpui|undecided, got {sel}"
    );

    for cand in ["egui", "gpui"] {
        let c = &v["candidates"][cand];
        for k in [
            "accesskit_out_of_process",
            "docking_save_restore",
            "editor_surface",
            "single_binary",
            "custom_viewport",
        ] {
            assert!(c[k].is_boolean(), "candidates.{cand}.{k} must be a boolean");
        }
        assert!(c["notes"].is_string(), "candidates.{cand}.notes must be a string");
    }
}
