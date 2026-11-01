---
file_id: LANE-SMB-ID-TOC-PERSIST-1
file_kind: lane_brief
updated_at: 2026-11-01T05:38:43Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-TOC-PERSIST-1
lane_file: builders/lanes/hsk-studio-id-toc-persist-1.json
crates: [hsk-studio-folio (TOC / index records gain the contract fields; page gains applied_trap_preset), hsk-studio-layout (lift between folio records and the contracts), hsk-studio-interop-layout (struct literals only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Persist the STU-LAY-148/149 contract fields and the STU-LAY-107 applied trap preset in the package">

## Target

Follow-up of SMB-ID-TOC-INDEX-1 (in STUDIO-R51). That lane built the TOC (STU-LAY-148), index (STU-LAY-149) and trap-preset (STU-LAY-152) contracts in layout `src/toc_contract.rs`, `src/index_contract.rs` and `src/print.rs`, beside the older Folio records, and could not persist the new fields: folio `src/longdoc.rs` `StudioTocStyle` / `StudioTocEntry` / `StudioIndexOptions` / `StudioIndexMark` / `StudioIndexXref` lack them, and adding fields breaks interop-layout struct literals; and the page's `applied_trap_preset` (STU-LAY-107) had no page object in layout, while folio `src/layout.rs` has `StudioPage`. Read that lane's file and notes first.

1. **Folio records.** Add the missing STU-LAY-148/149 fields to the folio TOC and index records with the contract's kinds (typed enumerations, the six independent index separators, the four level styles, include flags, numbered-paragraph and story-direction enumerations, index format, capitalisation, header-set reference). Additive and backward compatible: an older package without the fields loads with the spec's defaults or unknown (never invented values), and the package externals classification covers every new reference-looking field (rule 4b).
2. **Trap preset on the page.** Add `applied_trap_preset` to folio `StudioPage` as a typed reference to a stored trap preset (STU-LAY-107), with the folio record for stored trap presets if the package must hold them (reuse layout's record shape by value; folio must not depend on layout). A page range assignment is the same reference on each page.
3. **Layout lift.** Lossless conversion both ways between the folio records and layout's contract types; replace any default-filled lift the TOC-index lane wrote; the generators read the persisted fields.
4. **interop-layout.** Update only the struct literals the new fields break (with explicit defaults or unknown); no new IDML mapping in this lane (record which IDML attributes would map, as notes for a later lane).
5. **Tests.** Package round trip of every new field; an old-shape package loads; the lift is lossless both ways; a page with an applied trap preset round-trips; existing tests change only where a literal changes (list them).
6. **MTs.** Claim only what is met: MT-30777, MT-30779, MT-30846 (page assignment), MT-173/MT-174 persistence rows if any.

Spec gaps SD-31..SD-36 stay as recorded (no invented members or defaults). Vendor parity UNVERIFIED (D5).

Out of scope: IDML/ICML import-export of the new fields, hyperlink/bookmark export, rendering; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: setting a page's applied trap preset is a mutating document edit and takes an ActorContext from the caller; never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-toc-persist-1`, branch `lane/smb-id-toc-persist-1`, base a7be22a1)

- folio: `src/longdoc.rs`, `src/layout.rs` (`StudioPage` and a trap-preset record), their tests. Another lane (SMB-FOLIO-ANGLE-UNBOUNDED-1) edits folio `src/composition/effects.rs` in parallel: do not touch it.
- package: the externals classification test, only for the new fields (4b).
- layout: `src/toc_contract.rs`, `src/index_contract.rs`, `src/print.rs` (lift and the trap-preset reference only), `src/toc.rs` / `src/index.rs` (read the persisted fields), their tests.
- interop-layout: struct literals only.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of the changed crates and their direct dependents (RED COMMITS).
- One bundled run on D: (`lane-check.sh --owner smb-id-toc-persist-1`; C: holds validation round STUDIO-R51): step 0; clippy over folio, layout, interop-layout and their direct dependents (wide); nextest WHOLE suites of folio, package (4b), layout and interop-layout; doctests if added.
- Push only after STUDIO-R51 is recorded and the lead releases you (SMB-ID-TOC-INDEX-1 is being judged on layout): rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form. If SMB-FOLIO-ANGLE-UNBOUNDED-1 pushes first, rebase over it.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
