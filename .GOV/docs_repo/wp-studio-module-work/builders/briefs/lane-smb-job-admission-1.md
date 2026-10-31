---
file_id: LANE-SMB-JOB-ADMISSION-1
file_kind: lane_brief
updated_at: 2026-10-31T08:05:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-JOB-ADMISSION-1
lane_file: builders/lanes/hsk-studio-job-admission-1.json
crates: [hsk-studio-session (additive typed job lifecycle), hsk-studio-harness-view (egui-free queue modules plus call sites with no toolkit code)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="RF-08 phase 0: one typed admission path through the session JobPort for the four local queues; typed overload, cancellation, stale-revision, release and superseded outcomes">

## Target

This is RF-08 of `research/integration-refactor-inventory-20261030.json`, the asset side of MT-36228. Spec basis: STU-ARC-008, -012, -013 and -020, and GUI-DESKTOP-033..035.

Engine lane under D6. Harness-view edits in a file with egui code add no toolkit code: no egui or eframe type, call, import or widget.

### What the code holds at 2b42a0fe (planner, read-only)

- **The session job contract.**
  - `JobPort { admit(&JobScope, Cost) -> Result<JobTicket, JobRefused { retry_after_ms }>; release(JobTicket) }` (`session src/ports.rs:117-142`). It has one typed outcome only, overload, and no cancellation, stale-revision or settlement type.
  - Session `Admission` (`src/admission.rs`, slots plus bytes, `Permit` releases on drop) is session-internal (`session.rs:866 admit_job`).
  - `JobPort` is implemented by kernel-bindings `BridgeJobs` (`src/adapter.rs:322`) and by test doubles. Leave the trait unchanged.
- **The four queues.**
  1. **reel `ExportQueue`** (`reel src/export.rs:351`).
     - reel depends on accord and observe only: no session edge.
     - The only product constructor is harness-view `src/view/timeline_export.rs:589-602` (`export_queue()`).
     - Cancellation is per handle; `run` executes on the caller's thread.
  2. **AE render queue** (`harness-view src/view/timeline_export/render_queue.rs`).
     - It builds reel `ExportJob`s; it is not a queue itself.
     - No product source runs those jobs at 2b42a0fe; only harness tests do (`render_queue_records.rs`, `render_queue_settings.rs`, `composition_export.rs`, `motion_records_session.rs`).
  3. **Export Media worker** (`harness-view src/view/timeline/export_media.rs:481-486` and `:666-671`): a `std::thread` named `hsk-export-media` that runs a reel queue; cancel is at `:687-690`.
  4. **`AudioBatchQueue`** (`harness-view src/view/audio_batch.rs:281`, which copies reel's queue shape).
     - The submit site is `src/view/hosted.rs:1447-1462`, in a file with egui code.
     - `AudioBatchQueue::new(actor)` takes its actor from SMB-ATTRIBUTION-1.
- **egui at 2b42a0fe** (checked with `\begui::|use egui`; the `egui|eframe` pattern also matched "wireframe"):
  - egui-free: `timeline_export.rs`, `timeline_export/render_queue.rs`, `timeline_export/composition.rs`, `timeline/export_media.rs`, `audio_batch.rs`, `audio_out.rs`;
  - egui code: `hosted.rs` (the AudioBatchQueue call site) and `mod.rs` (`render_if_dirty` :733, the preview path).
- **Real-time audio.**
  - `AudioOut::pump` (`src/view/audio_out.rs:257`) is a host-loop method that fills score-device's ring.
  - score-device `src/callback.rs` and `src/ring.rs` name no observe item.
  - `pump` has no product caller at 2b42a0fe; only the harness tests `audio_import.rs:362`, `audio_session.rs:260` and `viewer_audio.rs:151` call it. Live playback is not wired in the viewer.
  - So `pump()` is off-callback by construction. Record this grep evidence; do not edit score-device.

### Slices, one local commit each, red tests first

1. **Session: typed job lifecycle** (new `src/jobs.rs`, additive; `JobPort` and `Admission` unchanged).
   - **`AdmittedJob`** is a guard that holds:
     - the ticket and the port;
     - the `JobScope` and `Cost` (actor and resource attribution; STU-ARC-008 bounded admission by actor, document and resource);
     - the bound document revision.
   - `settle(self, JobOutcome) -> JobSettlement` releases the ticket exactly once. Drop without settle settles `Abandoned` once and counts it. A double or late release is impossible by move, and the port's release is called exactly once (GUI-DESKTOP-034).
   - **`JobAdmissionRefusal`:** `Overloaded { retry_after_ms }` (from `JobRefused`), `Canceled` (token set before admission), `StaleRevision { expected, head }`.
   - **`JobOutcome`:**
     - `Completed`;
     - `CompletedStale { bound, head }`: a committed export finished from its bound snapshot after the head moved. It is never dropped silently (STU-ARC-013);
     - `Canceled` and `Failed(code)`;
     - `Superseded { revision }`: replaceable previews only.
   - **Release timing.** Cancellation never releases before the job's own terminal receipt (GUI-DESKTOP-033). `settle` is called with the receipt's outcome, not at `cancel()`.
   - **`AdmissionJobPort`:** an asset-local `JobPort` over session `Admission`, so harness-view can admit without a host. Declare it asset-local in the `DESCRIPTOR`.
   - Every terminal outcome is emitted through observe with codes and counters only (rule 5).
2. **Harness-view: one admitted path for the reel queue, the Export Media worker and the render-queue jobs** (egui-free files only).
   - `export_queue()` (`timeline_export.rs:589`) stays the single product constructor and returns the queue with an admission handle.
   - A new egui-free module (proposed `src/view/job_admission.rs`) admits each submitted item through `JobPort` before `submit` and settles it from that item's `ExportReceipt`:
     - in `run` order for the caller-thread path;
     - on the worker's join for `export_media.rs:481-486` / `:666-671`;
     - after the receipt arrives for the cancel path `:687-690`.
   - The render-queue job path (`render_queue.rs`) goes through the same module wherever jobs are run. Today that is test-only; say so in the notes.
   - Overload refuses typed before any submit, thread or encoder start.
   - **reel itself is not changed.** No reel → session edge; harness-view adapts. Tests that build a reel `ExportQueue` directly stay valid as asset-local engine tests. The lane notes say that product code builds queues only through `export_queue()` (grep proof).
3. **Harness-view: AudioBatchQueue and previews.**
   - `audio_batch.rs` admits each item through the same session lifecycle and settles from its `AudioBatchReceipt`.
   - The `hosted.rs:1447-1462` call site passes the `JobPort`. That is the only edit in that file, with no toolkit code.
   - **Previews.** `render_if_dirty` (`mod.rs:733`) has no stale-frame revision gate.
     - Put a gate in a new egui-free module (proposed `src/view/preview_gate.rs`): a frame whose revision is older than the snapshot's at display time is recorded as typed `Superseded { revision }` (counter plus status line), never shown as current.
     - `render_if_dirty` calls it in one line, with no toolkit code.
     - Previews are not admitted through `JobPort` (they are synchronous); say so in the notes.

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
  "jobs": "bounded_admission_via_job_port",
  "settings": "none",
  "spec_refs": ["STU-ARC-008", "STU-ARC-012", "STU-ARC-013", "STU-ARC-020", "GUI-DESKTOP-033", "GUI-DESKTOP-034", "GUI-DESKTOP-035"],
  "refactor_exposure": "low",
  "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief",
  "notes": {
    "jobs": "the four local queues admit through the session JobPort with typed overload, cancellation, stale-revision, release and superseded outcomes; host admission is MT-36228 (the kernel-bindings BridgeJobs implementation already exists)",
    "realtime": "audio callbacks never call observe: score-device callback.rs and ring.rs name no observe item, and AudioOut::pump is host-loop only (grep evidence)",
    "gui_host": "the hosted.rs and mod.rs edits are one-line call sites with no toolkit code (D6 lead reading)"
  },
  "recorded_at": "2026-10-31T08:05:14Z"
}
```

</topic>

<topic id="paths" status="active" summary="Owned paths and do-not-touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-job-admission-1`, branch `lane/smb-job-admission-1`, base = the origin carrying SMB-ATTRIBUTION-1)

- session:
  - new `src/jobs.rs`;
  - additive `src/lib.rs` lines and the `DESCRIPTOR`;
  - new `tests/job_lifecycle.rs`.
- harness-view:
  - `src/view/timeline_export.rs` (the `export_queue` factory);
  - `src/view/timeline/export_media.rs` (admit and settle around the worker);
  - `src/view/timeline_export/render_queue.rs` and `composition.rs` (the job-run path only);
  - `src/view/audio_batch.rs`;
  - new `src/view/job_admission.rs` and `src/view/preview_gate.rs`;
  - one call-site line each in `src/view/hosted.rs` (:1447-1462) and `src/view/mod.rs` (`render_if_dirty`);
  - new tests.
- Do not touch:
  - reel and score-device;
  - the `JobPort` trait and `Admission` signatures;
  - kernel-bindings;
  - harness-core (SMB-ATTRIBUTION-1 and SMB-BUSINESS-EVENTS-1);
  - harness-core `src/batch.rs` (its `BatchPorts.jobs` already takes a `JobPort`; not one of the four queues);
  - egui items;
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run order">

## Proof (5 or fewer focused tests per slice)

- **Session:**
  - settle releases the ticket exactly once on every exit (success, failure, cancel, drop);
  - a canceled token refuses before admission;
  - `JobRefused` maps to `Overloaded` with its retry;
  - `StaleRevision` and `CompletedStale` carry both revisions;
  - `AdmissionJobPort` bounds slots and bytes.
- **Harness-view:**
  - an export with a saturated port is refused typed before any thread or encoder starts;
  - canceling a running Export Media job keeps the ticket held until the worker's receipt, then settles it `Canceled` once;
  - an export whose document head moved finishes `CompletedStale`, with the file from the bound snapshot;
  - an AudioBatchQueue item settles once per receipt;
  - a preview frame older than the snapshot is recorded `Superseded` and not displayed.
- **Real time:** grep evidence in the notes that score-device `callback.rs` / `ring.rs` name no observe item and that `pump` has no callback caller.
- **Rule 4e:** check that the harness-view tests you add compile under a round unit (`["viewer"]`, `["desktop", "audio-batch"]`; Export Media may need `media`). Name a missing unit for the steward.

## Closure and run timing

- **Closure:**
  - session is upstream of kernel-bindings (→ interop-lrcat), harness-core, harness-view and harness;
  - harness-view is upstream of harness.
- **The bundled run (rules 4a and 4b):**
  - step 0 first;
  - clippy over session, kernel-bindings, interop-lrcat, harness-core, harness-view and harness;
  - the WHOLE suites of session, harness-view (both units) and harness (the units that run export, render-queue and audio-batch tests: at least `["media"]`, `["audio-batch"]`, `["viewer"]`).
  - The kernel-bindings suite is not needed unless its `JobPort` tests pin something you changed (they should not: the trait is unchanged).
- **Disk:** run on D: (`--disk d`) unless the lead names C:. R47 holds C:.
- **Wait for the lead:** do NOT start the bundled run until the lead messages you with the disk and the base.
- **Logs:** name the log per RUN LOGS.
- **Push order:** THIRD, after SMB-ATTRIBUTION-1 (it edits `mod.rs` and `hosted.rs:1447` first) and SMB-BUSINESS-EVENTS-1 (harness-core is upstream of harness-view). Rebase first, and re-run if the rebase touched your crates or their dependencies.
- **Follow-up, not this lane:** `EXPORT_RENDERED` is emitted at job settlement through SMB-BUSINESS-EVENTS-1's port.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
