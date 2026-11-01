---
file_id: LANE-SMB-TIFF-CHANNELS-1
file_kind: lane_brief
updated_at: 2026-11-01T13:46:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-TIFF-CHANNELS-1
lane_file: builders/lanes/hsk-studio-tiff-channels-1.json
crates: [hsk-studio-interop-tiff (Folio-free multi extra-sample planes), hsk-studio-harness-core (map TIFF extra planes to and from Folio channel records)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="F-R52-2 TIFF part: channel records survive TIFF interchange, with interop-tiff kept Folio-free">

## Target

Follow-up of SMB-R52-FOLLOWUPS-1 (read its lane file: item 2 was blocked because interop-tiff may depend only on accord and observe). LEAD DECISION T1 (2026-11-01T13:46:33Z): no new interop-tiff -> folio edge. interop-tiff stays format-only and Folio-free; the mapping between TIFF extra planes and Folio channel records lives in harness-core, which already depends on folio and already stores PSD channel tiles as package assets (`src/import.rs`, SMB-R50-FOLLOWUPS-1). Spec: v02.229 STU-RAS-022 (:844), STU-RAS-043 (:1885), STU-IO-006 row 3 (TIFF, alpha preserved).

1. **interop-tiff, Folio-free.** Decode and encode ANY number of extra samples (today it decodes one and encodes one ExtraSamples value) with their ExtraSamples values (0 unspecified, 1 associated alpha, 2 unassociated alpha), as plain typed planes in interop-tiff's own model, at the bit depths interop-tiff supports, with budgets (V4-01/02) and typed refusals for counts or depths it cannot carry.
2. **harness-core mapping.** On TIFF import, each extra plane beyond the image's own alpha becomes a Folio channel record with its tile stored as a package asset (as PSD channels are); on TIFF export, each channel record becomes an extra plane. Fields TIFF cannot carry (names, display colour, solidity, channel kind beyond alpha) are typed losses in the export / import result, named and listed in the lane notes. Find where TIFF export is invoked (harness-core or harness) and map there; if TIFF export has no host path that sees the document's channel records, record that and do import plus a host-callable export function.
3. **Tests.** Red first: a document with an alpha channel and a channel saved from a selection exports to TIFF and re-imports with both channel records and identical pixels (at each depth TIFF carries), the typed losses are reported, and interop-tiff's own tests cover 0, 1 and several extra samples.
4. **MTs.** Claim only what is met: MT-30095 (TIFF round-trip rows), MT-038 rows this touches.

Out of scope: EXR (spec-owner item SD-41); other formats; GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (no new edge: if harness-core does not depend on interop-tiff, stop and report); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: import and export keep their existing context", "jobs": "none: import and export stay bounded calls", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-tiff-channels-1`, branch `lane/smb-tiff-channels-1`, base e3afbb62)

- interop-tiff: `src/scan.rs`, `src/encode.rs`, `src/model.rs`, `lib.rs`, tests.
- harness-core: `src/import.rs` (TIFF channel mapping only) and the TIFF export call site, tests; harness `tests/import_tiff.rs`.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first, proven red by running it; a pushed red commit must compile the round units of the changed crates and their direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-tiff-channels-1`): step 0; clippy over interop-tiff, harness-core and their direct dependents; nextest WHOLE suites of interop-tiff and harness-core, plus harness's import units that read TIFF (4e).
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
