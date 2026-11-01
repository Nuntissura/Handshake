---
file_id: LANE-SMB-PRESS-INK-MANAGER-1
file_kind: lane_brief
updated_at: 2026-11-01T10:46:05Z
wp: WP-KERNEL-STUDIO
lane: SMB-PRESS-INK-MANAGER-1
lane_file: builders/lanes/hsk-studio-press-ink-manager-1.json
crates: [hsk-studio-press-write (ink manager contract STU-LAY-160, PDF/X standards enumeration), hsk-studio-layout (transparency flattener preset record STU-LAY-161)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Ink manager and PDF/X contract (STU-LAY-160) and the transparency flattener preset (STU-LAY-161)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-LAY-160 at :7206 and STU-LAY-161 at :7417; read STU-LAY-100/103 (bounds), STU-LAY-105 (`overprint_black_on_save`), STU-LAY-135 (spread flattener level). Code at base: press-write `src/inks.rs` (`InkManager`, ink types used by the ink-coverage lane), `src/export.rs` / `src/export/pdfx.rs` (PDF/X-4 identification only), `src/separations.rs` and `src/coverage.rs` (SMB-PRESS-INK-COVERAGE-1, PASS_V1 in R51); layout `src/print.rs` (print presets carry `flattener_preset_name`) and `src/pdf_export_prefs.rs` (`applied_flattener_preset` is a bounded name with no record behind it). No ink angle / frequency / neutral density / solidity fields, no PDF/X standards enumeration and no flattener preset record exist at base.

1. **Ink manager (STU-LAY-160).** Add `ink_angle`, `ink_frequency`, `neutral_density`, `solidity` per ink as seven-field records exactly as the table (neutral density precision 3; unknown never mirrored); `ink_type` as the four-member enumeration (reuse the type the coverage lane uses if it is the same enumeration; never a second one); per-ink spot-to-process conversion, all-spots-to-process, ink aliasing and use-standard-Lab-values-for-spots; the typed errors: a process ink cannot be aliased; a mixed ink holds at most 27 spot inks and at most 31 inks in total; a mixed-ink group holds at most 1,000 swatches (enforce where mixed inks are represented in press-write; if they live elsewhere, record where and enforce only in press-write's own records).
2. **PDF/X standards (STU-LAY-160).** The six-member enumeration (`pdfx.none`, `pdfx.x1a_2001`, `pdfx.x3_2002`, `pdfx.x1a_2003`, `pdfx.x3_2003`, `pdfx.x4_2010`) as a typed value on the export settings; export writes the identification only for the standards this crate actually implements (X-4 at base) and refuses the others with a typed not-implemented outcome, never a false claim of conformance.
3. **Separations readouts.** Per-plate on/off preview state, the ink-limit view threshold and per-ink coverage readouts readable as typed values (reuse the coverage lane's records; add only what is missing).
4. **Flattener preset (STU-LAY-161).** A typed flattener-preset record in layout beside the print-preset parameters: `line_art_and_text_resolution` (1..9600 dpi) and `gradient_and_mesh_resolution` (0..1200 dpi) as seven-field records, the raster-vector balance with its bounds undeclared (SPEC GAP: store unknown, never invent), the four booleans, and the five-member spread flattener level of STU-LAY-135 (reuse it if it exists). `applied_flattener_preset` and `flattener_preset_name` resolve against stored presets (typed refusal on a dangling name). The flattener PREVIEW and actual flattening are out of scope (record as follow-up; the coverage lane's `transparency_flattening` stage stays unperformed).
5. **MTs.** Claim only what is met: MT-30844, MT-30845 (STU-LAY-160), MT-30848, MT-30849 (STU-LAY-161), MT-35695 (its ink / separations / flattener-preset rows only).

Existing tests change only where a contract changes a pinned value; list each with old -> new. Hostile sizes (V4-01/02). Vendor parity UNVERIFIED (D5).

Out of scope: trapping, transparency flattening itself, the flattener preview, overprint simulation changes; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (the press-write -> observe edge is a pending steward item; keep `internal_diagnostics` as the coverage lane recorded it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none: press-write has no observe edge (steward item, F-R51-7); layout producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: editing inks, aliases and stored flattener presets are mutating document edits and take an ActorContext from the caller; never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-press-ink-manager-1`, branch `lane/smb-press-ink-manager-1`, base 99836457)

- press-write: `src/inks.rs`, `src/export.rs`, `src/export/pdfx.rs`, `src/separations.rs` / `src/coverage.rs` (readouts only), `lib.rs` exports, their tests; the press face `crates/hsk-studio-press/src/lib.rs` re-exports and DESCRIPTOR only.
- layout: `src/print.rs` (flattener preset record and name resolution), `src/pdf_export_prefs.rs` (name resolution only), their tests.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of the changed crates and their direct dependents (RED COMMITS).
- One bundled run on D: (`lane-check.sh --owner smb-press-ink-manager-1`; C: holds validation round STUDIO-R52): step 0; clippy over press-write, press, layout and their direct dependents; nextest WHOLE suites of press-write, press and layout; doctests if added.
- Push only after STUDIO-R52 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
