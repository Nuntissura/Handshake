---
file_id: LANE-SMB-PR-SEQUENCE-PRESETS-1
file_kind: lane_brief
updated_at: 2026-11-02T14:03:34Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-SEQUENCE-PRESETS-1
lane_file: builders/lanes/hsk-studio-pr-sequence-presets-1.json
crates: [hsk-studio-folio (sequence settings completeness, sequence presets, editing modes, immersive and preview-render settings on the sequence)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Sequence settings record, presets and editing modes (STU-VID-011, 016, 016a, 017, 017a, 018, 019)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-VID-010 .. STU-VID-019 (:22982-23600). Code at base: folio `src/timeline.rs` `StudioSequence` (:394), folio `src/composition.rs` `StudioImmersiveSettings` (STU-VID-018 shape, used for compositions), pulse has the time-display enumeration. No `StudioSequencePreset` or `StudioEditingMode` exists (`git grep` at base: none).

1. **STU-VID-011, settings completeness.** Compare `StudioSequence`'s settings with the normative record: every required field present and typed, values per STU-VID-012 / 012a / 013 / 013a (ticks; time-display enumeration reused from where it already lives if folio can reach it, otherwise the same tokens in folio with a note; never a second divergent enumeration). Add only what is missing; list each added field.
2. **STU-VID-016 / 016a, sequence presets.** `StudioSequencePreset` as a named, complete settings record; the preset library as data in the `StudioStyleRegistry` (find where the style registry lives; if folio holds it, add a preset section; if not, record where and stop that part), portable, with the commands of the 016a command table as typed operations taking the caller's context.
3. **STU-VID-017 / 017a, editing modes.** `StudioEditingMode` as a named constraint set over sequence settings; the constraint lists themselves are a declared gap: carry the structure with no invented constraints, and the 017a command table operations.
4. **STU-VID-018 / 019.** Immersive settings on the sequence record (reuse `StudioImmersiveSettings`), and the preview render settings of STU-VID-019 on the sequence record.
5. **Package round trip** of everything new (rule 4b externals), old-shape sequences load unchanged.
6. **MTs.** Claim only what is met: MT-33686, MT-33695, MT-33696, MT-33697, MT-33698, MT-33699, MT-33700, MT-33701, MT-33702.

Hostile input (V4-01/02): preset and mode lists are budgeted with typed refusals. Vendor parity UNVERIFIED (D5).

Out of scope: timeline editing, rendering, any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: folio may depend only on accord; operations return code-only reports", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: preset and editing-mode operations are mutating document edits and take an ActorContext from the caller; never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-sequence-presets-1`, branch `lane/smb-pr-sequence-presets-1`, base 96ace3ca)

- folio: `src/timeline.rs`, a new sequence-preset module, the style-registry module (preset section only), `lib.rs` exports and DESCRIPTOR, tests; package externals test only for new fields (4b).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of folio's dependents.
- One bundled run on C: (`lane-check.sh --owner smb-pr-sequence-presets-1`): step 0; clippy over folio and its direct dependents (wide); nextest WHOLE suites of folio and package (4b), and of any dependent that pins `StudioSequence` (find with grep; at least pulse and reel if they do); doctests if added.
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
