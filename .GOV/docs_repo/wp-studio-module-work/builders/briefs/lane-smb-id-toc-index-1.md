---
file_id: LANE-SMB-ID-TOC-INDEX-1
file_kind: lane_brief
updated_at: 2026-11-01T02:58:58Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-TOC-INDEX-1
lane_file: builders/lanes/hsk-studio-id-toc-index-1.json
crates: [hsk-studio-layout (tables of contents, index, trap presets), hsk-studio-folio (additive record fields only, if a style record must persist in the package)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Tables of contents (STU-LAY-044/148), index (STU-LAY-045/149) and trap presets (STU-LAY-152) in the layout engine">

## Target

Spec: Master Spec `14-studio-creative-suite.md` (v02.226 lines; v02.227 changes only wgpu/egui wording). STU-LAY-044 + STU-LAY-148 at :6628-6670, STU-LAY-045 + STU-LAY-149 at :6672-6719, STU-LAY-152 at :7394-7411. Read the clauses whole, plus the clauses they cite (STU-LAY-036/037 style registry and application, STU-LAY-042/147 books, STU-LAY-100/103 bound rules, STU-LAY-107 page object, STU-LAY-166 constraint and error vocabulary). The product code has no table-of-contents or index model yet (`git grep` at base: none); layout `src/print.rs` carries the print-preset `trapping` enumeration but no trap-preset record.

1. **Tables of contents (STU-LAY-044, STU-LAY-148).** A typed TOC style record with every field of the TOC style table and a TOC entry record with every field of the entry table (enumerations as typed members; style fields as typed style references in the layout style model). The generate command takes the TOC style, a replace-existing flag, an optional book, a place point and the include-overset flag (exposed, default as the spec says or unknown when it says nothing); it collects source paragraphs by mapped paragraph style, honours `level`, `page_number_position`, `separator`, `sort_alphabet`, `run_in`, `include_hidden`, `remove_forced_line_break` and `make_anchor` (a live anchor in the source paragraph). Multiple TOCs per document with independent settings, and an update-all command that refreshes every one. The three typed errors: a style with no entry (a default-styled TOC is the only exception), the same paragraph style mapped twice in one TOC style, and `make_anchor` on a locked story reported as a typed outcome, never silently skipped. Where the book model cannot supply book documents, the flag is stored and a generate that asks for them returns a typed unavailable outcome (no invented book behaviour).
2. **Index (STU-LAY-045, STU-LAY-149).** Inserted index marks building a topic / subtopic hierarchy; Studio supports exactly the four captured levels and declares its own maximum as a named constant (it MAY exceed four). Sort-by overrides, per-reference page-range scoping and the cross-reference forms. The index-options record with every field of the table: the SIX separators are six independent strings (never collapsed), `index_format` `{run_in, nested}` governing level 2 and lower, section headings and empty sections (valid only when headings are on). The eight-member cross-reference-type enumeration and the four-member capitalisation enumeration with its constraint (the first two members refused when capitalising a whole index). A pluggable section-header-set model (name, language id, group list) with the captured header-type enumeration: use the member names the spec states; if the spec does not list all twenty-seven, implement the ones it names, keep the model open, and record the missing names as a spec gap in the lane notes (no invented members). The captured typed errors: invalid page entry, invalid topic entry, invalid referenced topic entry, index not updated because a story is locked, document missing from the book.
3. **Trap presets (STU-LAY-152).** A trap-preset record with the seven rows of the parameter table as seven-field records (reuse the bound-record type layout already uses for print-preset parameters in `src/print.rs`; read it first): hard and soft bounds separate, `unknown` never mirrored from the opposite range, `black_density` hard maximum 12.0 (the wider capture, per STU-LAY-103) with precision 3, `pt` canonical for the two widths with the declared per-unit ranges as conversions. `trap_end_type` and `trap_image_placement` as typed enumerations. A preset is assignable to a page range through the page's `applied_trap_preset` (STU-LAY-107): add the field additively if layout's page object exists; if it does not, record that in the lane notes and stop at the record.
4. **MTs.** Read each MT JSON under `.GOV/task_packets/WP-KERNEL-STUDIO/` and claim only what is met, one assertion per acceptance row where the row is concrete: MT-173, MT-174, MT-30776, MT-30777, MT-30778, MT-30779, MT-30846, MT-30847.

Hostile input: TOC sources, index marks and header sets come from documents; every list is budgeted with a typed refusal (V4-01/02), arithmetic checked. Vendor parity is UNVERIFIED (D5); never claim it.

Out of scope:
- Rendering the generated TOC or index into composed frames beyond inserting the generated story content through layout's existing story/flow API; no new export path; no GUI; no command-registry rows (control_descriptors none, as for every engine lane).
- Books beyond the typed unavailable outcome; notes, cross-references and text variables (STU-LAY-046/047/048).
- No egui, eframe, winit or wgpu line (D1, D7); no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.226", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: generate, update and update-all take an ActorContext from the caller (actor/session, command id, document revision, correlation); never default_actor(); correlation grants nothing", "jobs": "none: generation stays a bounded call under its budgets", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-toc-index-1`, branch `lane/smb-id-toc-index-1`, base 31454a07)

- layout: new modules for the TOC, the index and the trap preset (your names), their `lib.rs` exports and DESCRIPTOR text, `src/print.rs` (trap-preset record beside the print-preset parameters only), the page object (additive `applied_trap_preset` only), and new test files.
- folio: additive record fields only, if a TOC style, index options or trap preset must persist in the package; then rule 4b applies (package suite).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of its changed crates and direct dependents (RED COMMITS).
- One bundled run on C: (`lane-check.sh --owner smb-id-toc-index-1`, one lock hold): step 0; clippy over the changed crates and their direct dependents; nextest WHOLE suite of layout (4a), and of folio plus package if folio changed (4b); doctests if added (4d). Name the log per RUN LOGS.
- Push order: SMB-R50-FOLLOWUPS-1 (harness-core / harness, whose closure includes layout) pushes first; push after it is on origin (`git log origin/codex/handshake-studio-assets`) or when the lead releases you: rebase, `lane-check.sh --owner smb-id-toc-index-1 --per-sha origin/codex/handshake-studio-assets`, push only on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
