---
file_id: LANE-SMB-COL-SWATCH-GROUPS-1
file_kind: lane_brief
updated_at: 2026-11-01T10:46:05Z
wp: WP-KERNEL-STUDIO
lane: SMB-COL-SWATCH-GROUPS-1
lane_file: builders/lanes/hsk-studio-col-swatch-groups-1.json
crates: [hsk-studio-folio (swatch groups, palette scope, colour-book resource records), the crate that already parses ICC profiles (named-colour binding, only if it parses ICC tags)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Swatch groups (STU-COL-177), palette scope (STU-COL-178), colour-book resource and licensing posture (STU-COL-185..188)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-COL-177 at :10605, STU-COL-178, STU-COL-185..188 (:10668 onwards); read STU-COL-170..184 around them for the swatch model they extend. Code at base: folio `src/vector_tools.rs` `StudioSwatch` (already has `group_ref`), `StudioSwatchKind`, `StudioSpotInk`; no group record, no palette scope, no colour-book record exists (`git grep` at base: none).

1. **Swatch groups (STU-COL-177).** A typed group record set with exactly one ROOT group, named groups, membership through `StudioSwatch.group_ref`, and the clause's rules (what may nest, what deleting a group does to its swatches, ordering) with typed refusals for a second root, a dangling `group_ref`, and cycles if nesting is allowed. Package round trip (rule 4b externals for any reference-looking field).
2. **Palette scope (STU-COL-178).** `document`, `application`, `system` as a typed enumeration on the palette / swatch-library record with the clause's precedence and visibility rules; a document stores only document-scope swatches, and references to other scopes resolve as the clause says (typed refusal or fallback, exactly as stated).
3. **Colour-book resource (STU-COL-185..188).** A colour book is a data adapter that populates native spot swatches: a typed book record (book id, display name, provenance, per-book licensing posture per STU-COL-186 as a typed field, never a blanket flag), with the rule that a book whose licensing posture forbids bundling is never shipped or embedded and only referenced; STU-COL-187 coverage recorded as data for planning (no book content invented or copied); STU-COL-188 named-colour profile binding: if a crate in the workspace already parses ICC profiles and their tags, add parsing of a named-colour profile there and bind it to the book adapter; if none parses ICC tags, store the binding as a typed reference and record the parser as a follow-up. Never commit any branded book data (licensed content); tests use a synthetic book with invented names that are obviously not real.
4. **MTs.** Claim only what is met: MT-31239 (STU-COL-177), MT-31240 (STU-COL-178), MT-31241 (STU-COL-179 panel operations, only the operations on groups you implement), MT-31248..MT-31251 (STU-COL-185..188).

Hostile input (V4-01/02): book files and group lists are budgeted with typed refusals. Vendor parity UNVERIFIED (D5).

Out of scope: swatch exchange formats (STU-COL-180), find/replace colour, mixed inks, any GUI panel; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: creating, moving and deleting groups and loading a book are mutating document edits and take an ActorContext from the caller; never default_actor()", "jobs": "none: book loading stays a bounded call", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-col-swatch-groups-1`, branch `lane/smb-col-swatch-groups-1`, base 99836457)

- folio: `src/vector_tools.rs` and a new swatch-library module, `lib.rs` exports and DESCRIPTOR, their tests; the package externals test only for new fields (4b).
- The ICC-parsing crate (find it first): a new named-colour module and its tests only.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of folio's dependents (RED COMMITS).
- One bundled run on D: (`lane-check.sh --owner smb-col-swatch-groups-1`; C: holds validation round STUDIO-R52): step 0; clippy over the changed crates and their direct dependents (wide for folio); nextest WHOLE suites of folio, package (4b) and the ICC crate if changed; doctests if added.
- Push only after STUDIO-R52 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
