---
file_id: LANE-SMB-BUSINESS-EVENTS-1
file_kind: lane_brief
updated_at: 2026-10-31T08:05:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-BUSINESS-EVENTS-1
lane_file: builders/lanes/hsk-studio-business-events-1.json
crates: [hsk-studio-kernel-bindings (business-event catalog, envelope, port), hsk-studio-harness-core (Run emission at commit time)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="RF-07 phase 0: kernel-bindings emits the 22 FR-EVT-STUDIO-* business events as typed envelopes, wired from harness-core's Run at commit time">

## Target

This is RF-07 of `research/integration-refactor-inventory-20261030.json`, the asset side of MT-36230. It starts after SMB-ATTRIBUTION-1 has pushed: it uses that lane's `CommandContext` (actor, client session, correlation, causation) and its `RunContext`. The host adapter, durable delivery and reconciliation through Handshake's Flight Recorder and EventLedger stay MT-36230. No leaf crate emits a business event (STU-CON-014: "The binding, not a leaf engine, emits after the canonical event").

### The 22 business events (spec v02.225, STU-CON-014 at 14:22638, wire variants per STU-CON-003 at 14:22620)

| Wire variant | event_family | Business ID |
|---|---|---|
| `STUDIO_DOCUMENT_CREATED` | `studio.document` | `FR-EVT-STUDIO-DOCUMENT-CREATED` |
| `STUDIO_DOCUMENT_PROMOTED` | `studio.document` | `FR-EVT-STUDIO-DOCUMENT-PROMOTED` |
| `STUDIO_LAYER_CREATED` | `studio.document` | `FR-EVT-STUDIO-LAYER-CREATED` |
| `STUDIO_LAYER_MUTATED` | `studio.document` | `FR-EVT-STUDIO-LAYER-MUTATED` |
| `STUDIO_SELECTION_CHANGED` | `studio.document` | `FR-EVT-STUDIO-SELECTION-CHANGED` |
| `STUDIO_MASK_APPLIED` | `studio.raster` | `FR-EVT-STUDIO-MASK-APPLIED` |
| `STUDIO_ADJUSTMENT_APPLIED` | `studio.color` | `FR-EVT-STUDIO-ADJUSTMENT-APPLIED` |
| `STUDIO_EFFECT_APPLIED` | `studio.effect` | `FR-EVT-STUDIO-EFFECT-APPLIED` |
| `STUDIO_VECTOR_PATH_EDITED` | `studio.vector` | `FR-EVT-STUDIO-VECTOR-PATH-EDITED` |
| `STUDIO_TEXT_EDITED` | `studio.typography` | `FR-EVT-STUDIO-TEXT-EDITED` |
| `STUDIO_STYLE_APPLIED` | `studio.layout` | `FR-EVT-STUDIO-STYLE-APPLIED` |
| `STUDIO_COMPONENT_PUBLISHED` | `studio.design_system` | `FR-EVT-STUDIO-COMPONENT-PUBLISHED` |
| `STUDIO_VARIABLE_SET` | `studio.design_system` | `FR-EVT-STUDIO-VARIABLE-SET` |
| `STUDIO_PROTOTYPE_EDITED` | `studio.prototype` | `FR-EVT-STUDIO-PROTOTYPE-EDITED` |
| `STUDIO_RAW_DEVELOPED` | `studio.color` | `FR-EVT-STUDIO-RAW-DEVELOPED` |
| `STUDIO_EXPORT_RENDERED` | `studio.export` | `FR-EVT-STUDIO-EXPORT-RENDERED` |
| `STUDIO_IMPORT_COMPLETED` | `studio.document` | `FR-EVT-STUDIO-IMPORT-COMPLETED` |
| `STUDIO_EDIT_PROPOSAL_RECORDED` | `studio.proposal` | `FR-EVT-STUDIO-EDIT-PROPOSAL-RECORDED` |
| `STUDIO_EDIT_PROMOTED` | `studio.proposal` | `FR-EVT-STUDIO-EDIT-PROMOTED` |
| `STUDIO_EDIT_REJECTED` | `studio.proposal` | `FR-EVT-STUDIO-EDIT-REJECTED` |
| `STUDIO_HISTORY_UNDONE` | `studio.history` | `FR-EVT-STUDIO-HISTORY-UNDONE` |
| `STUDIO_HISTORY_REDONE` | `studio.history` | `FR-EVT-STUDIO-HISTORY-REDONE` |

No Studio source names any of them at 2b42a0fe. accord declares `SCHEMA_STUDIO_BUSINESS_EVENT = "hsk.studio.business_event@1"` (`src/lib.rs:39`), and nothing uses it.

### Slices, one local commit each, red tests first

1. **Catalog and envelope** (kernel-bindings, new `src/business_event.rs`).
   - **`StudioEventVariant`.** A closed enum of the 22 rows, with `as_str()` (the wire variant), `family()` and `business_id()`.
     - Pin the table verbatim in a test against this brief's table.
     - No wildcard match, so a new variant fails to compile.
   - **`StudioBusinessEvent`.** A typed, `deny_unknown_fields` JSON envelope with these fields:
     - `schema_id` = `SCHEMA_STUDIO_BUSINESS_EVENT`, the variant, family and business id;
     - `document`, an `SDOC` `DomainId` validated with `SCHEMA_STUDIO_DOCUMENT`;
     - the actor (`ActorWire`), client session, `event_id`, `sequence`, correlation and causation, from SMB-ATTRIBUTION-1's context;
     - `revision`, the exact successor revision;
     - `command_id`;
     - optional input and output digests and typed artifact refs.
   - **Construction rules:**
     - `event_id` is deterministic from document and sequence, and is refused typed on an envelope collision (STU-CON-013 immutable-envelope collision refusal).
     - Payload bounds are checked before allocation.
     - No document text, label, path or font name: ids and digests only.
   - **Accord and the envelope.** Accord maps no identity prefix for the business-event schema (`identity_prefix` → `UnsupportedIdentityMapping`, `src/lib.rs:201-226`), so accord's `ValidatedEnvelope` cannot carry it as a resource. The envelope's resource is the document. Do not edit accord.
   - **Semantics.** Selection, proposal and rejection variants never claim a document promotion (STU-CON-014). Replaceable previews are not business events (STU-CON-013).
2. **Port and asset-local sink.**
   - `BusinessEventPort: Send + Sync { fn try_emit(&self, &StudioBusinessEvent) -> Result<(), EventDeliveryError> }`.
     - `EventDeliveryError { Saturated, Rejected, Unavailable, Indeterminate }`, closed, with observe's semantics: Indeterminate needs reconciliation.
   - An asset-local `TransientBusinessEvents`: a bounded in-memory list with typed overflow, memory only, declared asset-local in the `DESCRIPTOR` (storage NONE, as `TransientHost`).
3. **Run emission at commit time** (harness-core `src/harness.rs`, after SMB-ATTRIBUTION-1 is on origin).
   - **Wiring.**
     - `RunContext` gains an optional `Arc<dyn BusinessEventPort>`, additively.
     - Run emits only after the session accepted the commit (receipt in hand), never on a refusal.
     - A delivery failure never undoes the commit. It goes into the op record (`business_event`: id plus delivery result). Indeterminate sets a Run-level reconciliation flag, like observe.
   - **Mapping to verify in code** (record the final table in the lane notes):
     - `rename`, `set_visible`, `set_opacity`, `set_blend_mode`, `panel_set`, `set_tiles` and `set_tiles_and_entry` → `LAYER_MUTATED`;
     - `set_vector_paths` and `set_vector_paths_and_entry` → `VECTOR_PATH_EDITED`;
     - `restructure` and `restructure_and_entry` → `LAYER_CREATED` for a placement that adds a layer, else `LAYER_MUTATED`;
     - `open_import` and `open_import_detailed` → `IMPORT_COMPLETED`, plus `DOCUMENT_CREATED` only where the import mints a new document identity;
     - `undo` / `redo` → `HISTORY_UNDONE` / `HISTORY_REDONE`, with causation = the step's correlation;
     - `play_action`: each inner commit emits its own event, with causation = the action's correlation;
     - `set_document_entry` and the `*_and_entry` entry part: map an entry token only where the variant is unambiguous (grep the tokens: develop → `RAW_DEVELOPED`, a text story → `TEXT_EDITED`, …). An unmapped token emits nothing and is listed.
   - **No asset-local producer.** Typed and tested in the catalog, but not emitted; list each as NOT_PROVEN with its reason:
     - `DOCUMENT_PROMOTED`, `EDIT_PROPOSAL_RECORDED`, `EDIT_PROMOTED` and `EDIT_REJECTED`: no promotion or proposal flow in the asset-local Run;
     - `SELECTION_CHANGED`: view state, not a Run commit;
     - `COMPONENT_PUBLISHED`, `VARIABLE_SET` and `PROTOTYPE_EDITED`, unless an entry token maps;
     - `EXPORT_RENDERED`: exports run in harness-view queues, not in Run. A follow-up wires it at job settlement, after SMB-JOB-ADMISSION-1.
   - **Timeline and audio commits** (`submit_sequence*`, `submit_sequence_edits*`) have no catalogue family, so they emit nothing. Add a spec-delta request in the lane notes.
   - `save_to` and `checkpoint` are not promotion, so they emit nothing.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

## host_integration

```json
{
  "schema_id": "hsk.studio_mt_host_integration@1",
  "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030",
  "spec_version": "v02.225",
  "gui_host": "none",
  "gpu": "none",
  "diagnostics": {
    "internal_diagnostics": "producer_through_observe",
    "flight_recorder": "business_event_ids",
    "palmistry": "none"
  },
  "control_descriptors": "none",
  "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing",
  "jobs": "none",
  "settings": "none",
  "spec_refs": ["STU-CON-003", "STU-CON-013", "STU-CON-014", "STU-ARC-003", "STU-VAL-002"],
  "refactor_exposure": "low",
  "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief",
  "notes": {
    "flight_recorder": "typed emission at the binding only; registration in Handshake's closed FrEventId / KernelEventType enums, durable delivery and Palmistry-to-FR ingestion are MT-36230 and WP-KERNEL-016",
    "proof": "asset-local sink results are evidence, never host acceptance"
  },
  "recorded_at": "2026-10-31T08:05:14Z"
}
```

</topic>

<topic id="paths" status="active" summary="Owned paths and do-not-touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-business-events-1`, branch `lane/smb-business-events-1`, base = the origin carrying SMB-ATTRIBUTION-1)

- kernel-bindings:
  - new `src/business_event.rs`;
  - `src/host.rs` (the transient sink, additive);
  - additive `src/lib.rs` lines and the `DESCRIPTOR`;
  - new `tests/business_events.rs`.
- harness-core:
  - `src/harness.rs` (the commit-time emission and the op-record field);
  - the `RunContext` field (additive);
  - new `tests/business_events_run.rs`.
- Do not touch:
  - accord (the schema constant exists and stays);
  - kernel-bindings `src/context.rs` and `src/wire.rs`, and the attribution paths SMB-ATTRIBUTION-1 owns, except to consume them;
  - harness-view and harness;
  - session;
  - leaf crates (folio, chronicle, pigment and the other engines);
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run order">

## Proof (5 or fewer focused tests per slice)

- **Catalog:**
  - the 22 rows equal the table;
  - envelope round-trip;
  - unknown field, wrong schema, non-`SDOC` document, oversize and duplicate `event_id` are each refused typed;
  - a decoy document text never appears in the encoded event.
- **Port:** overflow is the typed `Saturated` and Indeterminate is reported.
- **Run:**
  - a property edit, a vector-path edit, an import, an undo and a redo each emit exactly one event, after the commit, with the Run's actor, client session, correlation and revision;
  - a refused commit emits nothing;
  - a sink failure leaves the commit and history intact and is in the op record;
  - a grep proof in the notes: only kernel-bindings defines the events and only harness-core's Run emits them.

## Closure and run timing

- **Closure:**
  - kernel-bindings is upstream of interop-lrcat, harness-core, harness-view and harness (with the edge SMB-ATTRIBUTION-1 adds);
  - harness-core is upstream of harness-view and harness.
- **The bundled run (rules 4a and 4b):**
  - step 0 first;
  - clippy over kernel-bindings, interop-lrcat, harness-core, harness-view and harness;
  - the WHOLE suites of kernel-bindings (round units `["test-faults"]`, `["ckc-test-double"]`) and harness-core;
  - the harness-view and harness suites only if an op-record or `RunContext` change is pinned by their tests (check by grep and say which).
- **Disk:** run on D: (`--disk d`) unless the lead names C:. R47 holds C:.
- **Wait for the lead:** do NOT start the bundled run until the lead messages you with the disk and the base.
- **Logs:** name the log per RUN LOGS.
- **Push order:** SECOND, after SMB-ATTRIBUTION-1 and before SMB-JOB-ADMISSION-1. Rebase onto SMB-ATTRIBUTION-1 before slice 3, and re-run if the rebase touched your crates or their dependencies.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
