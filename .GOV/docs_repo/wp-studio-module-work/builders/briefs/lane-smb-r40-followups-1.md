---
file_id: LANE-SMB-R40-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-30T03:40:00Z
---

<topic id="brief" status="active" summary="Fix the three R40 code findings F-R40-2, F-R40-5, F-R40-6">

## SMB-R40-FOLLOWUPS-1

Source: `validation/round-R40.json` `findings_not_failures` (read the three entries in full first).

1. F-R40-2 (MEDIUM, hsk-studio-motion-js): `tests/os_caps.rs:203` wall guard now accepts 300 ms..9 s, so it cannot tell the host CPU poll kill (measured 408 ms) from the kernel per-process backstop (3.77 s), yet `src/descriptor.rs` `os_caps.windows` still claims host-poll enforcement measured at 408 ms and cites this test (V4-07 claim without proof). Preferred repair: count host-poll kills apart from kernel-limit kills (WorkerStats or the existing stats surface) and assert the host-poll path in the test, keeping a wide wall bound for machine load. If that is not possible without new edges, narrow the DESCRIPTOR claim to what the test proves. Do not tighten wall-clock bounds (R39/R40 ran under full machine load).
2. F-R40-5 (INFO, hsk-studio-harness-view): `src/view/design/instance.rs:538` rolls back with `let _ = self.run.undo();` and says "(the redraw was undone)" even when that undo failed. Check the result and name the undo failure in the error.
3. F-R40-6 (LOW, hsk-studio-folio): `src/layout_rules.rs:312-323` linked story / page-item fingerprints are FNV-1a 64 of `serde_json::to_vec(..).unwrap_or_default()`. Hash with sha256 (folio already depends on it; check Cargo.toml, no new dependency) and propagate the serialization error as a typed error. If the fingerprint is persisted in Folio records, keep reading old values: decide by the record contract and state the decision in the lane notes (a stored FNV value must not silently compare equal or unequal by accident; a typed "stale fingerprint, recompute" path is fine).

Owned paths: those three crates only. Other lanes may be pushing hsk-studio-folio (SMB-ID-CJK-KINSOKU-1, SMB-AI-AREA-TYPE-1, other files): rebase before push and keep your diff to your own hunks.

Proof: one bundled lane-check on C: (`--disk ssd`) covering the WHOLE suites of motion-js (with the feature sets its round units use: check `.config/round-features.toml`), harness-view, folio, plus hsk-studio-package (folio change) and folio's direct dependents if a folio public signature changes. Red-first: each fix has a test that fails on origin ce5df74c (run on a scratch copy or state the red evidence from the test's logic if a red run is not practical, labelled).

Model tier: opus/medium.

</topic>
