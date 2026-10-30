---
file_id: LANE-SMB-PS-ACTIONS-1
file_kind: lane_brief
updated_at: 2026-10-30T11:00:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-PS-ACTIONS-1
lane_file: builders/lanes/hsk-studio-ps-actions-1.json
crates: [hsk-studio-harness-core, hsk-studio-harness (slice 4 only, src/cli.rs)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules
---

<topic id="target" status="active" summary="Engine-only actions: record the Run command stream, replay as one undo step, batch over documents">

## Target

Engine only, no window code. Base: origin c719b1dc. The typed run model is `hsk_studio_harness_core::Run` (`crates/hsk-studio-harness-core/src/harness.rs`). Every public Run method is one typed operation that writes one op record (`push` / `own_op`). Read that file first, especially `property_edit`, `restructure`, `HistoryStep`, `undo_stack`, `history_step` and `save_to`. Then read `tests/run_restructure_and_entry.rs` for the fixture and `Selection` pattern, `src/manual.rs`, and `hsk-studio-session/src/ports.rs` (`JobPort`, `JobScope`, `Cost`). Push after each green slice, but only after the lead says go (see paths).

1. **Action records, validators and portable artifact** (new `src/action.rs`).
   - Records (spec names go in rustdoc):
     - `ActionSet {action_set_id, name, actions[]}` (STU-AUT-102).
     - Action record with all ten STU-AUT-103 fields and their closed domains: `function_key_index` 0..=12, `color_index` 0..=7, `declared_step_count`.
     - Step record with all ten STU-AUT-104 fields.
     - `dialog_options`: four tokens with values 0..=3 (STU-AUT-105).
     - Recorded value types (STU-AUT-106). The heading says "seven members" but the table has eight rows (reference, class, enumerated, measured, number, boolean, string, list). Implement the eight rows, keep string and list distinct, and list the count mismatch in the notes as a spec-delta request.
     - Six unit tokens (STU-AUT-107).
   - Codec: encode and decode JSON through `serde_json::Value`. harness-core's only approved dependency is serde_json, so no serde derive. Decode strictly: unknown, duplicate or missing fields are typed refusals.
   - Validators. Use the clause codes, and make each diagnostic name what its clause says it must name:
     - `action_declared_step_count_mismatch` (STU-AUT-181): the load fails.
     - `action_shortcut_collision` (STU-AUT-182).
     - `action_step_command_unresolvable` (STU-AUT-183). On import the step is kept and marked unresolvable, never dropped. It stays opaque and round-trips (STU-RAS-169, STU-AUT-114).
     - `action_step_parameter_key_unknown` (STU-AUT-184): warning.
     - `action_measured_value_missing_unit` (STU-AUT-185).
   - Command corpus: the validators resolve against a const table `RUN_COMMANDS`. It lists Run's operations with `command_id`, `recordable`, `mutates_authority`, `dry_runnable`, declared parameter keys and value types (an STU-AUT-147 subset). There is no `schemars` input_schema because schemars is not approved for harness-core: record that as a gap.
   - Naming: Folio already has a public `StudioAction` (prototype reactions, `hsk-studio-folio/src/reactions.rs:177`). Do not add a second public `StudioAction`. Use `ActionSet`, `ActionRecord` and `ActionStep`. Whether the records later move into Folio as authority records (STU-AUT-114) is a lead decision and out of scope here.
   - `action_id` (`SACT-{uuid_v7}`) is supplied by the caller and validated as text. Accord has no SACT identity prefix: record that as a gap.
   - Tests in new `tests/action_records.rs`, red-first:
     - every validator fires with its code and its named fields;
     - export then import is byte-identical;
     - an unknown command id survives import and re-export unchanged and is reported;
     - domain edges are refused: function key 13, colour 8, dialog option 4, an unknown unit token.

2. **Recording and replay on Run** (`src/action.rs` plus Run hunks in `src/harness.rs`).
   - Recording:
     - API: `Run::start_recording(action_id, name)` and `Run::stop_recording()`, which returns the action plus a recording report.
     - While recording, each successful recordable operation appends a step. The step's `command_id` is the corpus id. Its `parameters` are the resolved typed inputs:
       - layer ids as `reference` by stable id (STU-AUT-008);
       - opacity as `measured` percent;
       - blend mode as `enumerated` `blend_mode.<member>`. Find an existing member-name source; if none exists, use `number` and say so in the notes;
       - names as `string`;
       - visibility as `boolean`.
     - A refused operation is not recorded. This is a Studio reading; Photoshop behaviour is UNVERIFIED (D5).
   - Recordable set in this lane: `rename`, `set_visible`, `set_opacity`, `set_blend_mode`, `panel_set`, and `restructure` for placement-only writes (check whether create payloads replay faithfully; if they do not, mark them not recordable).
   - Everything else is `recordable: false` in `RUN_COMMANDS`. When such an operation runs during a recording, add a typed `not_recordable` entry to the report. Silent omission is forbidden (STU-AUT-109). This covers:
     - opens, reopens, saves, checkpoints, verifies;
     - the reads `summary` and `panel`;
     - undo and redo;
     - the byte-payload operations `set_tiles`, `set_tiles_and_entry`, `set_vector_paths(_and_entry)`, `restructure_and_entry`, `set_document_entry`, `submit_sequence`. Their future route is the `path_literal` / `tool_stroke` step kinds of STU-AUT-111 (gap).
   - Replay: `Run::play_action(&ActionRecord, mode)` replays each enabled step through the same public Run method the original edit used. That is the STU-AUT-109 "identical command path"; there is no translation layer.
     - Disabled steps are skipped but kept in the record (STU-AUT-104).
     - Unresolvable steps and references that do not resolve fail with a typed code naming the step index and command id.
     - Replay is ONE transactional unit and ONE undo step (STU-AUT-004; the STU-AUT-100 correction of STU-AUT-010). Add a group history step that holds the replayed inverses and undoes them newest-first.
     - If a step fails, roll back the steps already applied in this replay. The document content and history are then as they were before the replay.
     - Headless mode: an enabled `always_show` step refuses before any step runs (STU-AUT-105, STU-AUT-136). Value 2 (`show_in_interactive_playback`) is suppressed in headless playback, per the clause.
   - Keep the history change additive: existing `HistoryStep` kinds and the undo/redo of non-group steps must behave exactly as before. If you have to change those code paths, also run the dependents' suites (see proof).
   - Tests in new `tests/action_record_replay.rs`, using fixture `../../hsk-studio-harness/tests/fixtures/demo.handshake`:
     - record rename, opacity and visible: three typed steps;
     - replay on a fresh open of the fixture: the same field values (checked with `verify_live` expectations) and exactly ONE new undo label;
     - one undo reverts all three;
     - a step with an unknown layer reference leaves content and history unchanged;
     - a disabled step is skipped;
     - a non-recordable operation during recording shows up as `not_recordable` in the report;
     - replayed op records carry the same `op` names as the originals.

3. **Batch runner over documents** (new `src/batch.rs`).
   - Job record: STU-AUT-126 shape.
   - Source kinds (STU-AUT-127):
     - implemented: `explicit_file_list`, and `folder` with a recurse flag and include/exclude patterns using `*` and `?` (no new dependency);
     - refused as `batch_source_kind_unsupported`, naming the kind: `open_documents`, `selection_within_document`, `catalog_query`.
   - Applied automation (STU-AUT-128): all three of `action`, `command_sequence` and `none`.
   - Output targets (STU-AUT-129), one or more per input:
     - format `handshake` writes a native package via `Run::save_to`;
     - other formats are refused per target with a typed code.
   - Filename template (STU-AUT-130):
     - tokens: original filename, original extension, sequence number (start and zero padding; hard_min 0, hard_max 2^31-1, soft 1..100000, default 1), output format, custom string;
     - the date/time and dimension tokens are refused with a typed code. Declare this as a gap: no clock port yet, and refusing keeps runs deterministic.
   - Overwrite policy (STU-AUT-131): `overwrite`, `rename`, `skip`. `ask` refuses at start in headless mode (`batch_ask_overwrite_in_headless`, STU-AUT-187).
   - Error policy (STU-AUT-131):
     - `stop_on_first_error`;
     - `skip_and_continue` (the default);
     - `retry_then_skip` with a declared retry count. Record the backoff value, but never sleep;
     - `quarantine`: refuse as `batch_error_policy_unsupported` unless you implement it as a copy into a declared folder. Never move or delete an input.
   - Start-time refusals: any enabled `always_show` step (`batch_always_show_dialog_in_headless`, STU-AUT-186, naming the step and its action), and `ask`.
   - Job Runtime:
     - admit one ticket per input through an optional `Arc<dyn hsk_studio_session::JobPort>` with `Cost{jobs: 1, bytes}`;
     - release the ticket on every exit path;
     - an admission refusal goes through the error policy.
   - Per-file receipts (STU-AUT-132) carry:
     - the input identity: path, byte length, and a content digest from an existing workspace function (no new dependency);
     - the applied automation and its resolved parameters;
     - the output path of each target;
     - the outcome;
     - elapsed time, declared non-deterministic and left out of equality checks.
   - Quiet law (STU-AUT-136): cancellation through the `CancellationToken` between inputs and between steps; structured job state (done/total, current input, cancelled) through a callback or polled state; no window and no prompt.
   - Concurrency is 1 (sequential). Other values are refused as `batch_concurrency_unsupported` (Studio reading). Export and import of the job is the portable batch artifact (STU-AUT-133).
   - Tests in new `tests/action_batch.rs`:
     - three fixture copies give three outputs with the action applied (reopen and verify);
     - a name collision gets a rename suffix; `skip` leaves the existing file alone;
     - a corrupt input under `skip_and_continue` gives two successes and one failure receipt;
     - `stop_on_first_error` stops at the failure;
     - `always_show` and `ask` refuse before any file is touched;
     - cancelling after the first input stops the job;
     - a `JobPort` double that refuses once triggers a retry under `retry_then_skip`;
     - job export then import is byte-identical.

4. **(Optional, lead go only) CLI and manual.**
   - Add subcommands `batch JOB.json` and `action check SET.json` (runs the validators) in harness `src/cli.rs`, plus their `COMMANDS` entries in harness-core `src/manual.rs` (built-in model manual).
   - Test in new `crates/hsk-studio-harness/tests/harness_batch_cli.rs`. This slice brings the harness crate into the proof run (see proof).

</topic>

<topic id="spec" status="active" summary="Spec basis (module 14 v02.225, grep by clause id)">

## spec_basis

- STU-RAS-169 (:1310): macro steps are ordinary Studio commands. The system MUST be built on the typed command surface with no parallel vocabulary. An unknown step is preserved opaque and receipted.
- STU-AUT-001/002/004/008 (:21849-21883): one typed command contract; command batch as one transactional unit with one coalesced history group; addressing by stable prefixed ids.
- STU-AUT-010/011/012 (:21906-21926): record, play and batch; macro control table.
- STU-AUT-100 (:21450): correction table — an action is a record with a step list, replayed INSIDE a command batch.
- STU-AUT-101..114 (:21463-21547): record shapes and closed enumerations.
- STU-AUT-126..136 (:21605-21628): batch.
- STU-AUT-147 (:21674): command corpus.
- STU-AUT-181..187 (:21805-21817): validator codes.
- Studio readings, UNVERIFIED (D5, no Photoshop oracle):
  - refused commands are not recorded;
  - headless handling of `show_if_configured`;
  - `*`/`?` folder patterns;
  - the refused template tokens;
  - sequential concurrency.
- Declared gaps, each a `NOT_PROVEN:` line in the tests:
  - GUI actions panel;
  - Folio persistence of action sets;
  - schemars input schemas;
  - the STU-AUT-111 step kinds;
  - event-bound actions (STU-AUT-112);
  - conditional predicates (STU-AUT-113);
  - the SACT accord prefix.

</topic>

<topic id="paths" status="active" summary="Owned paths, do-not-touch, coordination">

## Owned paths (lane worktree `wt-studio-lanes/smb-ps-actions-1`)

- harness-core:
  - NEW `src/action.rs` and `src/batch.rs`;
  - `src/lib.rs` (mod and `pub use` hunk);
  - `src/harness.rs` (recording hooks, group history step and `play_action` hunks only);
  - `src/manual.rs` (slice 4 entries only);
  - NEW `tests/action_records.rs`, `tests/action_record_replay.rs`, `tests/action_batch.rs`.
- harness (slice 4 only): `src/cli.rs` (subcommand hunk) and NEW `tests/harness_batch_cli.rs`.
- Do NOT touch:
  - harness-view: SMB-AU-MULTITRACK-1 owns `src/view/timeline*` and SMB-R41-FOLLOWUPS-1 owns `src/view/vector.rs` and two tests;
  - folio: record placement is a lead decision;
  - session: use `JobPort` as is. If a port API is missing, stop and report the exact signature;
  - accord;
  - the harness fixture: read only; new fixtures go in as new files;
  - `RUN-UI.md`;
  - root `Cargo.toml`, `Cargo.lock`, `tools/`, `.config/`.
- Coordination: harness-core is a dependency of harness-view. SMB-AU-MULTITRACK-1 is waiting on a confirming run on c719b1dc. Push only after the lead's go, so running lanes re-run once.

</topic>

<topic id="proof" status="active" summary="Proof and tier">

## Proof

- **Bundled run, per push.** One bundled lane-check on C: over the WHOLE harness-core suite (rule 4a): `--disk ssd --owner smb-ps-actions-1 --tests <slice tests> -- cargo nextest run -p hsk-studio-harness-core`.
  - The new pub API is additive: say so in the notes. lane-check's clippy covers the dependents harness and harness-view.
  - If an existing pub signature or the undo/redo behaviour of existing steps changes, also run `-p hsk-studio-harness -p hsk-studio-harness-view` with the round's viewer units (rules 4b and 4e).
- **Slice 4.** Add `-p hsk-studio-harness` with default features (the CLI tests). `cli.rs` also carries viewer-gated code, so run the harness `["viewer"]` round unit once before READY (`.config/round-features.toml`).
- **Out of scope.** No Folio record field changes in this lane, so rule 4b's package suite is not triggered. If you find you need one, stop.
- **Rules 4c-4e.**
  - 4c: run `--step0-only` first; tests write only under `std::env::temp_dir()` (studio-env.sh points TMP at the lane tmp); no path literals.
  - 4d: doc examples mean `cargo test --doc -p hsk-studio-harness-core`.
  - 4e: harness-core has no feature-gated tests; if you add one, name its unit.
- **Test style.** Red-first per slice. Print `NOT_PROVEN:` lines for the declared gaps.
- **Model tier.** opus/high: record tables, closed enumerations, transactional history.

</topic>

<topic id="gain" status="active" summary="Estimated parity gain">

## Estimated parity gain (engine formula of parity-20261023-rollup.json)

- PS-31, Photoshop (36 rows): MISSING -> at least PARTIAL (+0.69 pp).
  - BUILT_NOT_VALIDATED if the audit accepts record + replay + batch as the row (+1.39 pp).
  - VALIDATED after a round PASS (+2.78 pp).
- AF-12, Affinity (80 rows), which reuses PS-31. Its note reads "no macro recorder/player, no generic document batch". PARTIAL -> BUILT (+0.31 pp); VALIDATED +0.94 pp.
- AU-22 unchanged: favourites and effects-rack presets are not covered.
- Pooled 9-app engine (308 rows): +0.08 to +0.57 pp.
- GUI: none. A later GUI lane binds `Run::start_recording` and `play_action` to an actions panel.

</topic>
