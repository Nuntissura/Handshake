---
file_id: LANE-SMB-FOLIO-CHANNEL-RECORD
file_kind: lane_brief
updated_at: 2026-10-31T14:38:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-FOLIO-CHANNEL-RECORD
lane_file: builders/lanes/hsk-studio-folio-channel-record.json
crates: [hsk-studio-folio (channel record), hsk-studio-pigment (selection <-> channel), hsk-studio-interop-psd (PSD alpha channels), hsk-studio-package (tests only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="STU-RAS-022/031: persistent alpha and spot channels as one Folio record, save/load selection through it, channel list operations, PSD alpha-channel interchange">

## Target

Queued ledger lane SMB-FOLIO-CHANNEL-RECORD (no commits). No channel store exists at e9c46302: folio has no channel record (grep `StudioChannel|alpha_channel|selection_set` over folio and pigment src finds no channel type: `StudioChannelBlend`, folio raster.rs:426, and unrelated settings names); pigment selections are compute only (`selection.rs:1-5`); harness-view keeps the selection as an interim session typed entry (ledger note). Engine-only under D6; the harness migration to the record is a view change and is carried forward.

**Spec** (`spec-modules/14-studio-creative-suite.md`): STU-RAS-022 :820 (save a selection to a persistent alpha/spare channel and reload it; round-trippable interchange; save/load selection, alpha channels and the Channels surface are ONE mechanism); STU-RAS-031 :1412 (colour, alpha and spot channels; visibility, reorder, rename; convert to/from selections and masks by the same mechanism; Duplicate/Split/Merge; Apply-Image; Calculations).

**MTs** (index status PENDING, `gui_host` none, `gpu` via_render_provider_port):
- closed: MT-038 (AC-001 persists and reloads identically, AC-002 shares the Channels mechanism, AC-003 interchange round trip), MT-30095 (STU-RAS-022);
- advanced: MT-050 (AC-005 shared store; the record half of AC-001: visibility, reorder, rename, convert; Duplicate of AC-002). Split/Merge, Apply-Image, Calculations and the surface stay open.

**Base at e9c46302:**
- folio: `StudioRasterRecords` `src/raster.rs:961` (beside the document so struct literals stay valid, :1-6; `new` :975; empty members not encoded); `TileRef` `src/lib.rs:282`; `Resolver::tile` :383.
- package: `src/references.rs:7-9` collects every object whose `schema_id` is the Folio tile token from the document and record entries, so channel tiles that are `TileRef`s travel with no package source change.
- pigment: `CoveragePlane` `src/selection.rs:128`, `CombineMode` :91, `invert` :290; `ActorContext` use in `src/caller.rs:4,360`.
- interop-psd: `MergedImage` extra planes `src/document.rs:95-140` (alpha is the first extra plane only with merged transparency); resources kept byte-exact (`src/resources.rs:1-2`, `find` :87); `TileBlob` / `MaskTileBlob` `src/proposal.rs:201,217`; `ImportProposal` :342; `to_proposal` :2032; `export_psd` `src/export.rs:279`.

### Slices, one local commit each, red tests first

1. **Folio channel record** (MT-038 AC-002, MT-050 AC-005):
   - `StudioRasterRecords.channels: Vec<StudioChannelRecord>` with serde default and empty not encoded, so every existing document and digest is unchanged (a test re-serialises a fixture byte-identically);
   - record: stable `channel_id`, bounded `name`, `kind` in {`masked_area_alpha`, `selected_area_alpha`, `spot_colour`} (component channels are derived, never stored), `visible`, display colour and opacity or spot solidity percent, depth 8 / 16 / 32, `tiles: Vec<TileRef>`;
   - typed validation: duplicate id, bad depth, tiles that do not cover the document raster;
   - new package test `tests/channel_tiles.rs`: a document with a channel round-trips its channel tile bytes byte-identically through the package writers and reader (rule 4b; classify a new field in `tests/externals.rs` only if that test flags it).
2. **Save and load selection** (pigment `src/selection_channel_roundtrip.rs`, the path MT-038 names; MT-038 AC-001, MT-30095):
   - `CoveragePlane` -> channel samples at the record's depth plus tile payloads for the host to store;
   - channel samples -> `CoveragePlane`, with `CombineMode` into an existing selection;
   - the polarity rule per kind is declared in the module docs (Studio reading; Photoshop behaviour UNVERIFIED, D5). Converting between the two alpha kinds inverts samples and never merges kinds (MT-050 step 1). Save then load gives the identical plane at 8, 16 and 32 bits (exact for 16/32; 8-bit quantisation declared).
   - Every op returns a new record set and takes an `ActorContext` from its caller (rule 4); no `default_actor()`.
3. **Channel list operations** (MT-050 AC-001 record half, Duplicate): rename, reorder, visibility, duplicate, delete and kind conversion as typed record transforms, same `ActorContext` rule; refusals typed (unknown id, name too long, duplicate id).
4. **PSD alpha channels** (interop-psd; MT-038 AC-003):
   - import: extra planes after the merged-transparency plane become channel records and tile blobs in the proposal, named from image resources 1006 (Pascal) and 1045 (Unicode), identified by 1053, with kind, colour and opacity from DisplayInfo 1077;
   - export: written back; an unmodified file re-emits identical channel planes and resources;
   - the resource ids and layouts come from the public PSD file format specification; confirm each against it and the psd-tools corpus, else UNVERIFIED;
   - hostile input (V4-01/02): channel count, name lengths and plane sizes checked before allocation; a `dims_u32_max` and an `aggregate_over_budget` test.
- Terminal outcomes through observe with codes only (rule 5). No command is registered: the Channels surface and its command ids come with the GUI (D7).

### Not in this lane
- Harness-view migration from the interim session entry to the record (view code; slot 5 owns harness-view this wave); Split/Merge, Apply-Image, Calculations; TIFF extra-sample channels.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "via_render_provider_port", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-RAS-022", "STU-RAS-031", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045", "GUI-DESKTOP-025", "GUI-DESKTOP-028", "GUI-DESKTOP-032", "GPU-QUOTA-010", "STU-ARC-002", "STU-ARC-018"], "refactor_exposure": "low", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"attribution": "every channel save, load and list operation takes an ActorContext from its caller and returns a new record set; no command id is registered until the Channels surface exists (D7)", "gpu": "CPU sample conversion only; no device or GPU path", "diagnostics": "codes and opaque ids only; channel names never reach observe"}, "recorded_at": "2026-10-31T14:38:14Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-folio-channel-record`, branch `lane/smb-folio-channel-record`, base e9c46302)

- folio: `src/raster.rs` (channel record and validation; a new `src/channel.rs` is allowed), `DESCRIPTOR` entries in `src/lib.rs`, new `tests/channel_records.rs`.
- pigment: new `src/selection_channel_roundtrip.rs`, module and pub use lines in `src/lib.rs`, new `tests/selection_channels.rs`.
- interop-psd: `src/document.rs`, `src/resources.rs`, `src/proposal.rs`, `src/export.rs`, `src/writer.rs` as needed (additive public API only), new `tests/psd_alpha_channels.rs`.
- package: new `tests/channel_tiles.rs`; `tests/externals.rs` only if it flags a new field. No package `src` edit.
- No `Cargo.toml` change, no root input, no new dependency.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over folio, pigment, interop-psd and their direct dependents (chronicle, components, composite, develop, harness-core, harness-view, harness, interop-fig-base, interop-fig-native, interop-fig, interop-layout, interop-media, interop-vector-core, interop-vector-svg, interop-vector, kernel-bindings, layout, package, pulse, render-cpu, render-gpu, session by manifest), plus `-p hsk-studio-harness --features import-psd`, because harness reaches interop-psd only behind that feature (4e).
- nextest WHOLE suites of folio, pigment, interop-psd and package (4a, 4b); the harness `import-psd` unit's `import_psd` target if the proposal output changes for any fixture it pins; doctests if added (4d). This is the wave's widest clippy set; expect the longest run. Name the log per RUN LOGS.
- Push order: folio and pigment sit in the closures of SMB-AE-WATER-SIMS-1 (composite) and SMB-ID-PDF-EXPORT-PREFS-1 (layout), so this lane pushes AFTER both; press is unaffected. If slot 5 changes accord or observe (every lane's closure), the lead sequences it. Do NOT start the bundled run until the lead names the disk and confirms R49 judged SMB-SETTINGS-SCHEMA-1 (it changed pigment; a FAIL there is remediated first).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
