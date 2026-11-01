---
file_id: LANE-SMB-FOLIO-ANGLE-UNBOUNDED-1
file_kind: lane_brief
updated_at: 2026-11-01T05:38:43Z
wp: WP-KERNEL-STUDIO
lane: SMB-FOLIO-ANGLE-UNBOUNDED-1
lane_file: builders/lanes/hsk-studio-folio-angle-unbounded-1.json
crates: [hsk-studio-folio (recovered ANGLE effect parameters unbounded per SD-26), hsk-studio-composite (only where it pins a changed folio bound)]
model: sonnet
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Store every recovered STU-FX-131a ANGLE parameter unbounded with full signed degrees (v02.228 SD-26)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-FX-131a (note at :13825 and the v02.228 SD-26 note beside it; the ANGLE rows of the recovered tables now carry `--` in all four bound cells, defaults and precision provisional UNVERIFIED). Research basis: `research/sd26-30-research-20261101.json` item SD-26 (the capture decoded ANGLE records at slider offsets, so every numeric ANGLE bound was junk). Code at base: folio `src/composition/effects.rs` (`StudioEffectParamSpec`, `StudioBoundState`; e.g. Glow `color_phase` hard 0.003906..0.496094 near :661, Directional Blur Direction soft 0..7936 near :593), and the composite / folio callers that validate against those specs.

1. **Every recovered ANGLE row unbounded.** Find every folio parameter record that comes from a STU-FX-131a ANGLE row (kind angle or unit degrees, recovered origin). Store its four bounds as unknown / unbounded-in-source exactly as the record type represents "source declares nothing" (do not invent a range, do not mirror). Values are full signed degrees including revolutions (e.g. 725 degrees is two turns plus 5 degrees, stored as given); never wrap or clamp on store. Defaults and precision stay as recorded but are labelled provisional / UNVERIFIED in the record's provenance where the type can carry it, otherwise in the lane notes.
2. **Callers.** Wherever an engine evaluates an angle, it reduces modulo 360 only at evaluation if the effect's maths needs a principal angle, never in the stored value. Fix every validation that rejected a value because of the old junk bounds (Glow Color Phase above about 0.5 degrees is the known case).
3. **Tests.** Change the folio tests that assert the old pairs (list each old -> new with the row); add tests: a multi-turn and a negative angle store and round-trip through the package unchanged for at least Glow Color Phase, Directional Blur Direction and one Wave World or Foam angle row; no ANGLE row from a recovered table carries numeric bounds (a sweep over the folio catalogue).
4. **MTs.** Read the MT JSONs regenerated for SD-26 (the 33 STU-FX-131a angle-table MTs, MT-31934..MT-32022 range; list them by grepping for unit degrees) and claim only rows this lane meets; most of those MTs cover whole effects, so expect partial claims on the angle rows only.

Existing composite behaviour changes only where a stored bound changes; determinism stays. Vendor parity UNVERIFIED (D5).

Out of scope: the capture decoder fix (governance tool `ae_common.py`, blocked on absent captures); SD-27 engine scales (a SPEC GAP); no other effect parameter; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: no mutating command added", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-folio-angle-unbounded-1`, branch `lane/smb-folio-angle-unbounded-1`, base a7be22a1)

- folio: `src/composition/effects.rs` and the effect-catalogue modules holding recovered ANGLE rows, their tests.
- composite: only call sites and tests that pin a changed folio angle bound.
- Another lane (SMB-ID-TOC-PERSIST-1) edits folio `src/longdoc.rs` and `src/layout.rs` in parallel: do not touch those files.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of folio's dependents (RED COMMITS).
- One bundled run on D: (`lane-check.sh --owner smb-folio-angle-unbounded-1`; C: holds validation round STUDIO-R51): step 0; clippy over folio and its direct dependents (wide; expect a long run); nextest WHOLE suites of folio, package (4b) and composite (default and `motion` units); doctests if added.
- Push only after STUDIO-R51 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
