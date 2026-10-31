---
file_id: LANE-SMB-ATTRIBUTION-1
file_kind: lane_brief
updated_at: 2026-10-31T08:05:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-ATTRIBUTION-1
lane_file: builders/lanes/hsk-studio-attribution-1.json
crates: [hsk-studio-kernel-bindings (additive command context), hsk-studio-harness-core, hsk-studio-harness-view (call sites only, no toolkit code), hsk-studio-harness (asset-local test host entry points and tests)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="RF-06 phase 0: no default or literal actor in product paths; Run and HistoryStep carry actor, command id, revision and a context correlation">

## Target

This is RF-06 of `research/integration-refactor-inventory-20261030.json`. Spec basis: GUI-DESKTOP-029, -041, -045 and -050, STU-ARC-014 and STU-ARC-015.

Engine lane under D6. Harness-view edits are allowed only where they add no toolkit code: no egui, eframe or winit type, call, import or widget. A call-site change in a file that already contains egui code is allowed under that rule.

### Verified sites at 2b42a0fe (planner, read-only)

`default_actor()` is defined at `harness-core src/harness.rs:224` and re-exported at `harness-core src/lib.rs:19` and `harness src/lib.rs:31`. Its 11 product call sites:

| Crate | Site | egui code in the file |
|---|---|---|
| harness-core | `src/batch.rs:772` (`actor.unwrap_or_else(default_actor)`) | no |
| harness-core | `src/import/colour.rs:51` | no |
| harness-view | `src/view/mod.rs:298` | yes |
| harness-view | `src/view/shell.rs:1392` | yes |
| harness-view | `src/view/edit.rs:519` | no (a doc comment only) |
| harness-view | `src/view/fill_tools.rs:462` | yes |
| harness-view | `src/view/hosted.rs:1447` | yes |
| harness-view | `src/view/waveform.rs:503` | yes |
| harness-view | `src/view/waveform_plugin.rs:89` | no |
| harness | `src/cli.rs:678` | no |
| harness | `src/cli.rs:1023` | no |

The 3 literal actors:
- `harness-view src/view/text.rs:489` (egui code in the file);
- `src/view/vector/separations.rs:68` (no egui);
- `src/view/vector/swatch.rs:77` (no egui).

The `ActorContext::new` calls in chronicle, pigment, kernel-bindings and accord parse wire actors; they are not literals.

### Slices, one local commit each, red tests first

1. **kernel-bindings: typed command context** (new `src/context.rs`, additive).
   - **New types:**
     - `ClientSessionId` (opaque, validated like the accord id bounds);
     - `CorrelationId(u64)` and `CausationId(u64)`, opaque. They are u64 so they fit observe's correlation field;
     - `CommandContext { actor: ActorContext, client_session: ClientSessionId, correlation: CorrelationId, causation: Option<CausationId> }`;
     - a `CorrelationSource: Send + Sync` trait (`fn next(&self) -> CorrelationId`);
     - an asset-local `TransientCorrelation` (monotonic, memory only, marked asset-local in the `DESCRIPTOR`, beside `TransientHost`).
   - **`Envelope` (`src/wire.rs:315`)** gains `command: Option<CommandContextWire>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, so envelopes without it keep their bytes. `BridgePorts` fills it when the caller supplies a context.
   - **Keep `Envelope.correlation_id` as it is.** It is a per-exchange reply-matching counter (`src/adapter.rs:54` mints it, `:79` matches the reply on it), so it cannot carry the command correlation. Document that.
   - **Envelope literals to update:** `src/adapter.rs:55` and `tests/bridge_contract.rs:468, 561, 737, 794, 832`.
   - Process birth and PID stay host-filled (observe `RingRecord` placeholder; MT-36216). They are not in this lane.
2. **harness-core: Run attribution.**
   - **New module edge `hsk-studio-harness-core → hsk-studio-kernel-bindings`.** No cycle: kernel-bindings depends on accord, observe and session only. Module edges are steward-owned, so report it to the lead before the first push.
   - **`RunContext`** = actor + client session + `Arc<dyn CorrelationSource>`. It is a public type in harness-core; re-export the kernel-bindings context types from harness-core, so harness-view needs no new edge.
   - **`Run`:**
     - mints one `CommandContext` per mutating command;
     - gains `pub fn actor(&self) -> &ActorContext` and `pub fn context(&self) -> &RunContext`.
   - **Op records** (`push` :368-381) gain `actor` (the existing `actor_json`), `command_id` and `correlation`:
     - `command_id` is the `action::CMD_*` / RUN_COMMANDS id where one exists, else `null` with the op name kept in `op`;
     - `revision` stays;
     - `n` stays a local ordinal, never a correlation.
   - **`HistoryStep`** (:306) gains `actor`, `command_id`, `base_revision`, `revision` and `correlation`. `Run::history()` (:1535) shows them.
   - **Replace every op-number correlation with the command context's:** :391-392 (`emit_own` `Observe::new` / `Observation`), :1215, :1311 and :1372 (`TimelineStep` / `MulticamStep` `correlation_id`; `command_id` stops being `format!("{op}-{n}")`), :1560, :1608, :1759 (undo/redo inverse: a fresh context, with causation = the undone step's correlation), :1952 and :1991 (`patch_json*`).
   - **`batch.rs`:** `run_batch` takes a required actor. `BatchPorts.actor: Option<ActorContext>` becomes `ActorContext` through `BatchPorts::new(actor)`, and the `Default` impl goes.
   - **`import/colour.rs:51`:** `converter` takes the actor from its caller (the Run's).
3. **harness-view: call sites, no toolkit code.**
   - `Viewer` stores its `RunContext`, with a new `Viewer::with_context(prefer, gpu_options, RunContext)`.
   - `view/mod.rs:298` and `shell.rs:1392` build the run from the stored context.
   - `edit.rs:519`, `fill_tools.rs:462` and `hosted.rs:1447` use `self.run.actor()`.
   - `waveform.rs:503`, `waveform_plugin.rs:89`, `text.rs:489`, `separations.rs:68` and `swatch.rs:77`: their private functions gain an `&ActorContext` parameter from the Viewer caller.
4. **Asset-local test host.**
   - `default_actor()` leaves harness-core's default API. It becomes `asset_local_test_context()` (actor + client session + `TransientCorrelation`) behind a non-default harness-core feature, proposed name `asset-local-test-host`.
   - `Viewer::new(prefer)` and `Viewer::with_gpu_options` stay behind the same-named non-default harness-view feature: 49 harness-view test sites and 113 harness test sites use them, and they keep compiling.
   - harness (the asset-local test host, SC-2) enables both features in its normal dependencies, and `cli.rs:678, 1023` take the context from that one constructor.
   - harness-core and harness-view enable them for their own tests (self dev-dependency or `required-features`; rule 4e: check `.config/round-features.toml` and name a missing unit for the steward).
   - Product harness-view source never calls the asset-local constructor. Prove it with a grep in the notes.

### Public signature changes and their callers

| Change | Callers to update |
|---|---|
| `Harness::run(ActorContext, &CancellationToken)` → `Harness::run(RunContext, &CancellationToken)` | harness-core `src/batch.rs` (1) and tests: action_batch 1, action_record_replay 6, run_and_entry_redo 5, run_restructure_and_entry 3; harness-view `src/view/mod.rs`, `src/view/shell.rs` and tests: viewer_fig_edit_persist, viewer_proof, viewer_timeline_nest_2, viewer_vector_text_entry, viewer_vector_text_tools (1 each); harness `src/cli.rs` (2) and tests: harness_headless 7, import_psd 3, import_tiff 2, import_pdf 1, import_raster 1, viewer_develop_2 1, viewer_headless 1 |
| `default_actor()` removed; `asset_local_test_context()` behind the test-host feature | the same test sites, plus the re-exports in harness-core and harness `lib.rs` |
| `BatchPorts` actor required (`BatchPorts::new`, no `Default`) | harness `src/cli.rs:445`; harness-core `tests/action_batch.rs` (9 sites) |
| `Envelope` gains `command` | kernel-bindings `src/adapter.rs:55`, `tests/bridge_contract.rs` (5 literals) |
| `Viewer::new` / `with_gpu_options` feature-gated; `Viewer::with_context` added | none break while the feature is on (harness and test builds) |
| additive: `Run::actor`, `Run::context` | none |

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
    "flight_recorder": "none",
    "palmistry": "none"
  },
  "control_descriptors": "none",
  "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing",
  "jobs": "none",
  "settings": "none",
  "spec_refs": ["GUI-DESKTOP-029", "GUI-DESKTOP-041", "GUI-DESKTOP-045", "GUI-DESKTOP-050", "STU-ARC-014", "STU-ARC-015"],
  "refactor_exposure": "medium",
  "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief",
  "notes": {
    "gui_host": "harness-view edits are call sites with no toolkit code (D6 lead reading); no surface code",
    "internal_diagnostics": "observe frames carry the command context's correlation instead of the op ordinal",
    "flight_recorder": "business events are SMB-BUSINESS-EVENTS-1 and build on this lane's CommandContext",
    "attribution": "the actor, client session and correlation come from the caller's RunContext; the asset-local test host is the only place a fixed context is built"
  },
  "recorded_at": "2026-10-31T08:05:14Z"
}
```

</topic>

<topic id="paths" status="active" summary="Owned paths and do-not-touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-attribution-1`, branch `lane/smb-attribution-1`, base 2b42a0fe)

- kernel-bindings:
  - new `src/context.rs`;
  - `src/wire.rs` (the `Envelope` field and its wire type);
  - `src/adapter.rs` (fill the field);
  - additive `src/lib.rs` lines and the `DESCRIPTOR`;
  - `tests/bridge_contract.rs` (the literals) and a new context test.
- harness-core:
  - `src/harness.rs` (Run, HistoryStep, op record, correlation sites, `default_actor`);
  - `src/batch.rs` (`BatchPorts`, `run_batch` actor);
  - `src/import/colour.rs`;
  - `src/lib.rs` (re-exports);
  - `Cargo.toml` (the kernel-bindings edge and the test-host feature);
  - tests that call `Harness::run` or `default_actor`.
- harness-view: only the 11 sites listed above, the `Viewer` constructor and context field in `src/view/mod.rs`, `Cargo.toml` (the feature), and test imports.
- harness: `src/cli.rs` (the two entry points and the `BatchPorts` call), `src/lib.rs` (re-export), `Cargo.toml` (features), and test imports.
- Do not touch:
  - session;
  - reel;
  - the four job queues (SMB-JOB-ADMISSION-1);
  - kernel-bindings `src/business_event.rs` (SMB-BUSINESS-EVENTS-1);
  - observe;
  - any egui item;
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run order">

## Proof (5 or fewer focused tests per slice)

- **kernel-bindings:**
  - an `Envelope` without `command` encodes to the same bytes as before;
  - with it, the context round-trips;
  - the reply match still uses the exchange id (stale-reply refusal unchanged);
  - hostile `ClientSessionId` input is refused typed.
- **harness-core:**
  - two Runs with different contexts attribute their ops, history steps and observe frames to their own actor and correlation;
  - no frame, patch or timeline step carries the op ordinal as correlation (a test with a correlation source starting at 1000);
  - undo/redo carry causation = the undone step's correlation;
  - `run_batch` without an actor does not compile (`compile_fail` doctest; rule 4d).
- **harness-view:**
  - an edit, a fill, a waveform effect and a batch made through a Viewer built with a test context attribute to that context's actor;
  - the grep proof that no product source names `asset_local_test_context`, `default_actor` or a literal `ActorContext::new(`.

## Closure and run timing

- **Closure:**
  - kernel-bindings is upstream of interop-lrcat, and after the new edge also of harness-core, harness-view and harness;
  - harness-core is upstream of harness-view and harness.
- **The bundled run (rules 4a, 4b, 4d and 4e):**
  - step 0 first;
  - clippy over kernel-bindings, interop-lrcat, harness-core, harness-view and harness;
  - the WHOLE suites of kernel-bindings (round units `["test-faults"]`, `["ckc-test-double"]`), interop-lrcat (it uses the kernel-bindings API), harness-core, harness-view (`["viewer"]`, `["desktop", "audio-batch"]`) and harness (every round unit whose tests call `Harness::run` or `Viewer::new`);
  - doctests.
- **Disk:** run on D: (`--disk d`) unless the lead names C:. R47 holds C:.
- **Wait for the lead:** do NOT start the bundled run until the lead messages you with the disk and the base.
- **Logs:** name the log per RUN LOGS.
- **Push order:** this lane pushes FIRST of the three, then SMB-BUSINESS-EVENTS-1 (it builds on `CommandContext` and edits harness-core `src/harness.rs` and kernel-bindings `src/lib.rs`), then SMB-JOB-ADMISSION-1 (it edits harness-view `src/view/mod.rs` and `src/view/hosted.rs` after this lane). Rebase first, and re-run if the rebase touched your crates or their dependencies.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
