---
file_id: model-manual
file_kind: ModelManual
updated_at: "2026-05-18T00:00:00Z"
manual_version: "1.0.6"
---

# ModelManual

This on-demand projection is a build artifact. The Rust ModelManual manifest remains canonical.

<topic id="feature-hbr-process-diagnostics" status="current" version="1.0.6" summary="HBR, Process Ledger, And Diagnostics">

## Feature Group: HBR, Process Ledger, And Diagnostics

ID: `hbr_process_diagnostics`

Build-rule enforcement, typed violation receipts, process lifecycle evidence, and no-context diagnostics entrypoints.

Commands:
- `model_manual_get`
- `model_manual_list_commands`
- `model_manual_search`
- `hbr_violation_emit`
- `hbr_matrix_check`
- `hbr_validator_scan`
- `process_ledger_writer`
- `process_ledger_overflow_event`
- `process_ledger_reclaim`
- `process_ledger_staleness_reclaim`
- `diagnostics_capture_headless`
- `visual_debug_dom_snapshot`
- `visual_debug_console_stream_start`
- `visual_debug_console_stream_stop`
- `backend_inspector_read`
- `kernel_inspector_port`
- `kernel_inspector_list_sessions`
- `kernel_inspector_session_state`
- `kernel_inspector_event_ledger_tail`
- `kernel_inspector_process_ledger_active`
- `kernel_inspector_trace_projection`
- `kernel_inspector_loaded_models`
- `inspector_replay_drive`
- `swarm_harness_run`
- `handshake_swarm_cli`
- `kernel_swarm_run`
- `swarm_scenario_n8_perf`
- `swarm_scenario_session_cancel`
- `swarm_scenario_lease_contention`

</topic>

<topic id="feature-sandbox" status="current" version="1.0.6" summary="Sandbox">

## Feature Group: Sandbox

ID: `sandbox`

KERNEL-004 sandbox adapter surfaces for boxed model-written code and validation runners.

Commands:
- `sandbox_adapter_run`
- `sandbox_adapter_health`

</topic>

<topic id="feature-model-runtime" status="current" version="1.0.6" summary="Model Runtime">

## Feature Group: Model Runtime

ID: `model_runtime`

Local model registration, generation, LoRA hot-swap, KV cache controls, and inference-lab technique switches.

Commands:
- `model_runtime_generate`
- `model_runtime_register_model`
- `inference_lab_apply_technique`

</topic>

<topic id="feature-memory-self-improvement" status="current" version="1.0.6" summary="Memory And Self Improvement">

## Feature Group: Memory And Self Improvement

ID: `memory_self_improvement`

Memory capsule retrieval, distillation candidates, and bounded self-improvement loop controls.

Commands:
- `memory_capsule_build`
- `memory_calibration_snapshot`
- `self_improvement_run_iteration`
- `distillation_candidate_review`

</topic>

<topic id="command-model-manual-get" status="wired" version="1.0.6" summary="model_manual_get">

## Command: model_manual_get

ID: `model_manual_get`

Status: `wired`

IPC channel: `kernel.model_manual.get`

Tauri command: `model_manual_get`

Description: Return the full typed ModelManual manifest as JSON for an in-app or no-context model reader.

Expected input: No input.

Expected output: Manual JSON containing version, feature groups, command references, safety constraints, and workflows.

Schema fields:
- version
- feature_groups
- command_reference
- safety_constraints
- workflows

Common errors:
- IPC bridge not registered
- manual projection consumer expects stale field names

Recovery steps:
- Check MANUAL_VERSION and the MT-013 IPC bridge registration
- Use model_manual_list_commands to inspect the current command index


</topic>

<topic id="command-model-manual-list-commands" status="wired" version="1.0.6" summary="model_manual_list_commands">

## Command: model_manual_list_commands

ID: `model_manual_list_commands`

Status: `wired`

IPC channel: `kernel.model_manual.list_commands`

Tauri command: `model_manual_list_commands`

Description: Return the command reference index without the long workflow and recovery text.

Expected input: No input.

Expected output: CommandReference summary rows keyed by stable command id.

Schema fields:
- id
- name
- status
- ipc_channel
- tauri_command

Common errors:
- Command id not found in a feature group

Recovery steps:
- Run model_manual_tests to verify feature group references resolve


</topic>

<topic id="command-model-manual-search" status="wired" version="1.0.6" summary="model_manual_search">

## Command: model_manual_search

ID: `model_manual_search`

Status: `wired`

IPC channel: `kernel.model_manual.search`

Tauri command: `model_manual_search`

Description: Search command, workflow, safety, and feature group text by substring.

Expected input: A non-empty query string.

Expected output: Search results referencing command ids, workflow ids, and safety constraint ids.

Schema fields:
- query
- results
- command_id
- workflow_id

Common errors:
- Empty query
- manual data not loaded through the IPC bridge

Recovery steps:
- Call model_manual_get for the full manifest
- Retry with a concrete command, workflow, or HBR id


</topic>

<topic id="command-hbr-violation-emit" status="wired" version="1.0.6" summary="hbr_violation_emit">

## Command: hbr_violation_emit

ID: `hbr_violation_emit`

Status: `wired`

CLI flag: `--self-test`

Description: Emit or validate canonical HBR_VIOLATION JSONL receipts for build-time and handoff-time routing.

Expected input: An HBR_VIOLATION object or self-test flag.

Expected output: Sorted-key UTF-8 JSONL receipt with a UUID v7 receipt_uuid.

Schema fields:
- receipt_kind
- schema_version
- receipt_uuid
- hbr_id
- wp_id
- violation_class

Common errors:
- Unknown violation_class
- receipt_uuid is not UUID v7
- freeform notes used as routing data

Recovery steps:
- Validate against hbr-violation.schema.json
- Rebuild the receipt from typed HBR fields instead of notes text


</topic>

<topic id="command-hbr-matrix-check" status="wired" version="1.0.6" summary="hbr_matrix_check">

## Command: hbr_matrix_check

ID: `hbr_matrix_check`

Status: `wired`

CLI flag: `--all-packets`

Description: Verify PACKET_ACCEPTANCE_MATRIX HBR rows block PASS closure when any row remains PENDING, STEER, or BLOCKED.

Expected input: A packet path or all-packets mode.

Expected output: Exit 0 on pass; JSONL failures on matrix violations.

Schema fields:
- acceptance_matrix
- hbr
- hbr_not_applicable
- evidence_pointer
- validator_verdict

Common errors:
- PROVED row has no evidence_pointer
- NOT_APPLICABLE row lacks a ledger reason

Recovery steps:
- Hydrate the matrix from HANDSHAKE_BUILD_RULES
- Attach runnable evidence or not-applicable reasons


</topic>

<topic id="command-hbr-validator-scan" status="wired" version="1.0.6" summary="hbr_validator_scan">

## Command: hbr_validator_scan

ID: `hbr_validator_scan`

Status: `wired`

CLI flag: `--packet`

Description: Resolve PROVED HBR evidence pointers to tests, receipts, artifacts, or EventLedger rows.

Expected input: A work-packet JSON file with HBR matrix rows.

Expected output: Exit 0 when PROVED evidence resolves; downgraded STEER rows written atomically otherwise.

Schema fields:
- evidence_pointer
- hbr_evidence_results
- hbr_event_ledger

Common errors:
- Unknown evidence URI scheme
- test evidence has no recorded PASS result

Recovery steps:
- Use test://, receipt://, artifact://, or event:// pointers
- Record the passing proof result before claiming PROVED


</topic>

<topic id="command-process-ledger-writer" status="wired" version="1.0.6" summary="process_ledger_writer">

## Command: process_ledger_writer

ID: `process_ledger_writer`

Status: `wired`

Description: Append START and STOP process lifecycle events through the bounded ProcessOwnershipLedger writer.

Expected input: Typed ProcessStart and ProcessStop records from model engines, sandbox containers, workers, plugins, or helper subprocesses.

Expected output: Postgres kernel_process_lifecycle rows with UUID v7 process_uuid values and matched lifecycle timestamps.

Schema fields:
- process_uuid
- engine_kind
- started_at
- stopped_at
- exit_code
- owner_role
- owner_wp

Common errors:
- Writer channel saturated
- STOP arrives after START overflow
- Postgres lifecycle migration missing

Recovery steps:
- Inspect FR_EVT_LEDGER_OVERFLOW events
- Use the STOP upsert path to preserve termination evidence
- Apply migration 0021_kernel_process_lifecycle.sql


</topic>

<topic id="command-process-ledger-overflow-event" status="wired" version="1.0.6" summary="process_ledger_overflow_event">

## Command: process_ledger_overflow_event

ID: `process_ledger_overflow_event`

Status: `wired`

Description: Emit typed FR_EVT_LEDGER_OVERFLOW EventLedger rows when the process lifecycle writer channel saturates.

Expected input: A dropped LedgerEvent sampled from a full bounded channel.

Expected output: FR_EVT_LEDGER_OVERFLOW NewKernelEvent payload with overflow count, configured capacity, dropped event kind, and sampled lifecycle payload.

Schema fields:
- event_type
- overflow_count
- capacity
- dropped_event_kind
- sampled_event_payload

Common errors:
- Overflow sink unavailable
- sampled payload lacks process_uuid
- degraded flag not cleared after drain

Recovery steps:
- Verify the overflow sink records the event before returning Ok
- Drain queued lifecycle events
- Confirm process_ledger::is_degraded() returns false after a successful drain


</topic>

<topic id="command-process-ledger-reclaim" status="wired" version="1.0.6" summary="process_ledger_reclaim">

## Command: process_ledger_reclaim

ID: `process_ledger_reclaim`

Status: `wired`

Description: Reclaim open process lifecycle rows on session close, failure, staleness, or operator cancel.

Expected input: A session id and ReclaimTrigger value: close, failure, stale, or operator_cancel.

Expected output: ReclaimReport with every reclaimed process, kill result, and queued STOP evidence using exit_code -1.

Schema fields:
- session_id
- trigger
- processes_reclaimed
- kill_result
- total_duration_ms

Common errors:
- SandboxKill binding missing
- kill returns an error
- STOP writer channel saturated

Recovery steps:
- Treat kill errors as report data, not as permission to skip STOP
- Verify the STOP writer accepted ProcessStop rows
- Inspect kernel_process_lifecycle open rows with the reclaim FOR UPDATE query


</topic>

<topic id="command-process-ledger-staleness-reclaim" status="wired" version="1.0.6" summary="process_ledger_staleness_reclaim">

## Command: process_ledger_staleness_reclaim

ID: `process_ledger_staleness_reclaim`

Status: `wired`

Description: Run the background staleness reclaim loop with a configurable heartbeat TTL and scan interval.

Expected input: StalenessReclaimConfig plus a stale-session source.

Expected output: Background task that calls process_ledger_reclaim with trigger stale for each stale session.

Schema fields:
- ttl
- scan_interval
- stale_sessions
- trigger

Common errors:
- TTL configured as zero
- scan interval configured as zero
- stale source fails

Recovery steps:
- Use the 5 minute default TTL when unset
- Use the 30 second default scan interval when unset
- Keep stale-source failures isolated to the current scan tick


</topic>

<topic id="command-diagnostics-capture-headless" status="planned" version="1.0.6" summary="diagnostics_capture_headless">

## Command: diagnostics_capture_headless

ID: `diagnostics_capture_headless`

Status: `planned`

IPC channel: `kernel.diagnostics.capture`

Tauri command: `diagnostics_capture_headless`

Description: Capture GUI state, screenshots, console events, and network events without stealing operator focus.

Expected input: A target route, viewport, and optional selector.

Expected output: Diagnostic capture artifact references and replay metadata.

Schema fields:
- viewport
- selector
- screenshot_ref
- console_events
- network_events

Common errors:
- Target route not mounted
- foreground window requested during model-driven testing

Recovery steps:
- Use hidden-window/headless mode
- Open the diagnostics panel replay for the failed step


</topic>

<topic id="command-visual-debug-dom-snapshot" status="wired" version="1.0.6" summary="kernel_visual_debug_dom_snapshot">

## Command: kernel_visual_debug_dom_snapshot

ID: `visual_debug_dom_snapshot`

Status: `wired`

IPC channel: `kernel.visual_debug.dom_snapshot`

Tauri command: `kernel_visual_debug_dom_snapshot`

Description: Return a CDP DOM.getDocument or selector-scoped DOM.describeNode snapshot for diagnostics without foreground interaction.

Expected input: DomScope JSON: { kind: 'full' } or { kind: 'selector', selector: '<css selector>' }.

Expected output: DomTree JSON with owned nodes, attributes, children, and stable_element_id populated only from data-testid.

Schema fields:
- kind
- selector
- root
- node_id
- node_name
- attributes
- children
- stable_element_id

Common errors:
- WebView2 CDP port unavailable
- selector did not match any element
- CDP DOM response omitted node data

Recovery steps:
- Check kernel_visual_debug_launch_config for the active CDP port
- Retry with a data-testid selector from the current route
- Use the screenshot command to confirm the renderer loaded before snapshotting


</topic>

<topic id="command-visual-debug-console-stream-start" status="wired" version="1.0.6" summary="kernel_visual_debug_console_stream_start">

## Command: kernel_visual_debug_console_stream_start

ID: `visual_debug_console_stream_start`

Status: `wired`

IPC channel: `kernel.visual_debug.console_stream.start`

Tauri command: `kernel_visual_debug_console_stream_start`

Description: Start a WebView2 CDP Runtime console/exception stream and emit console_event payloads to the diagnostics panel.

Expected input: ConsoleScope JSON: { kind: 'all' }.

Expected output: Object containing a UUID v7 stream_id; subsequent console_event payloads contain stream_id, kind, message, timestamp, and optional stack.

Schema fields:
- stream_id
- kind
- level
- message
- timestamp
- stack

Common errors:
- WebView2 CDP port unavailable
- CDP Runtime.enable failed
- diagnostics event listener not registered

Recovery steps:
- Check kernel_visual_debug_launch_config for the active CDP port
- Restart the stream and subscribe to console_event before injecting diagnostics actions


</topic>

<topic id="command-visual-debug-console-stream-stop" status="wired" version="1.0.6" summary="kernel_visual_debug_console_stream_stop">

## Command: kernel_visual_debug_console_stream_stop

ID: `visual_debug_console_stream_stop`

Status: `wired`

IPC channel: `kernel.visual_debug.console_stream.stop`

Tauri command: `kernel_visual_debug_console_stream_stop`

Description: Stop a running WebView2 CDP Runtime console/exception stream and unregister its in-memory task handle.

Expected input: The stream_id returned by kernel_visual_debug_console_stream_start.

Expected output: Object with stopped flag indicating whether a matching stream was found and aborted.

Schema fields:
- stream_id
- stopped

Common errors:
- Unknown stream_id
- stream already stopped
- console stream registry unavailable

Recovery steps:
- Treat false stopped as idempotent cleanup
- Start a fresh stream before the next visual diagnostics capture


</topic>

<topic id="command-backend-inspector-read" status="planned" version="1.0.6" summary="backend_inspector_read">

## Command: backend_inspector_read

ID: `backend_inspector_read`

Status: `planned`

IPC channel: `kernel.inspector.read`

Tauri command: `backend_inspector_read`

Description: Read structured backend state for model navigation without screen scraping.

Expected input: An inspector surface id and optional selector.

Expected output: Structured state snapshot with stable ids and trace references.

Schema fields:
- surface
- selector
- state_snapshot
- trace_id

Common errors:
- Unknown inspector surface
- state snapshot exceeds configured budget

Recovery steps:
- List available inspector surfaces
- Narrow the selector before retrying


</topic>

<topic id="command-kernel-inspector-port" status="wired" version="1.0.6" summary="kernel_inspector_port">

## Command: kernel_inspector_port

ID: `kernel_inspector_port`

Status: `wired`

IPC channel: `kernel.inspector.port`

Tauri command: `kernel_inspector_port`

Description: Return the optional local inspector HTTP port exposed to diagnostics tooling.

Expected input: No input.

Expected output: InspectorReadV1-adjacent port response: the current random local inspector port, or null when the HTTP inspector is not running.

Schema fields:
- port

Common errors:
- Inspector port state is not registered
- inspector port state mutex poisoned

Recovery steps:
- Verify Tauri Builder manages InspectorPortState
- Fall back to in-process kernel.inspector.* IPC commands when no port is running


</topic>

<topic id="command-kernel-inspector-list-sessions" status="wired" version="1.0.6" summary="kernel_inspector_list_sessions">

## Command: kernel_inspector_list_sessions

ID: `kernel_inspector_list_sessions`

Status: `wired`

IPC channel: `kernel.inspector.list_sessions`

Tauri command: `kernel_inspector_list_sessions`

Description: Read all visible inspector sessions through the in-process InspectorReadV1 trait.

Expected input: No input.

Expected output: InspectorReadV1 SessionSummary rows serialized for Diagnostics-panel navigation.

Schema fields:
- id
- state
- model_id
- active_process_count

Common errors:
- InspectorReadV1 state is not registered
- session list is empty before a governed run starts

Recovery steps:
- Verify Tauri Builder manages Arc<dyn InspectorReadV1>
- Start or resume a governed session before expecting active rows


</topic>

<topic id="command-kernel-inspector-session-state" status="wired" version="1.0.6" summary="kernel_inspector_session_state">

## Command: kernel_inspector_session_state

ID: `kernel_inspector_session_state`

Status: `wired`

IPC channel: `kernel.inspector.session_state`

Tauri command: `kernel_inspector_session_state`

Description: Read a single session state by SessionId through the in-process InspectorReadV1 trait.

Expected input: A SessionId for the governed model session.

Expected output: InspectorReadV1 SessionStateRead JSON or null when the session is unknown.

Schema fields:
- session_id
- id
- state
- latest_event_id
- active_process_count

Common errors:
- Unknown session_id
- InspectorReadV1 state is not registered

Recovery steps:
- Call kernel_inspector_list_sessions to discover valid ids
- Confirm the diagnostics panel is attached to the current app process


</topic>

<topic id="command-kernel-inspector-event-ledger-tail" status="wired" version="1.0.6" summary="kernel_inspector_event_ledger_tail">

## Command: kernel_inspector_event_ledger_tail

ID: `kernel_inspector_event_ledger_tail`

Status: `wired`

IPC channel: `kernel.inspector.event_ledger_tail`

Tauri command: `kernel_inspector_event_ledger_tail`

Description: Read the newest EventLedger rows through the in-process InspectorReadV1 trait without using the HTTP server.

Expected input: A non-negative tail limit n.

Expected output: InspectorReadV1 EventLedgerRow JSON ordered by the reader implementation's tail semantics.

Schema fields:
- n
- event_id
- event_type
- event_sequence
- created_at_utc
- payload_hash
- payload

Common errors:
- Tail limit is too small to include the needed event
- InspectorReadV1 state is not registered

Recovery steps:
- Increase n for the diagnostic query
- Use kernel_inspector_trace_projection for session-scoped reconstruction


</topic>

<topic id="command-kernel-inspector-process-ledger-active" status="wired" version="1.0.6" summary="kernel_inspector_process_ledger_active">

## Command: kernel_inspector_process_ledger_active

ID: `kernel_inspector_process_ledger_active`

Status: `wired`

IPC channel: `kernel.inspector.process_ledger_active`

Tauri command: `kernel_inspector_process_ledger_active`

Description: Read active process ownership rows through the in-process InspectorReadV1 trait.

Expected input: No input.

Expected output: InspectorReadV1 ProcessRow JSON for currently running owned processes.

Schema fields:
- process_uuid
- session_id
- engine_kind
- status

Common errors:
- No active processes
- process ledger writer has not emitted START evidence

Recovery steps:
- Inspect session state and EventLedger tail
- Use process reclaim surfaces if stale process rows remain


</topic>

<topic id="command-kernel-inspector-trace-projection" status="wired" version="1.0.6" summary="kernel_inspector_trace_projection">

## Command: kernel_inspector_trace_projection

ID: `kernel_inspector_trace_projection`

Status: `wired`

IPC channel: `kernel.inspector.trace_projection`

Tauri command: `kernel_inspector_trace_projection`

Description: Read the HBR-VIS-004 session TraceProjection through the in-process InspectorReadV1 trait.

Expected input: A SessionId for the governed model session.

Expected output: InspectorReadV1 TraceProjection JSON or null when the session has no durable trace events.

Schema fields:
- session_id
- opened_at
- closed_at
- what_task
- what_context
- what_returns
- what_tool_calls
- what_artifacts
- what_validation
- what_promotion

Common errors:
- Unknown session_id
- session has no durable EventLedger rows

Recovery steps:
- Call kernel_inspector_event_ledger_tail to confirm events exist
- Use the HTTP /inspector/v1/trace/{session_id} route only when in-process IPC is unavailable


</topic>

<topic id="command-kernel-inspector-loaded-models" status="wired" version="1.0.6" summary="kernel_inspector_loaded_models">

## Command: kernel_inspector_loaded_models

ID: `kernel_inspector_loaded_models`

Status: `wired`

IPC channel: `kernel.inspector.loaded_models`

Tauri command: `kernel_inspector_loaded_models`

Description: Read loaded model rows through the in-process InspectorReadV1 trait.

Expected input: No input.

Expected output: InspectorReadV1 ModelLoadedRow JSON for local or registered model adapters.

Schema fields:
- model_id
- adapter_id
- process_uuid
- loaded_at_utc

Common errors:
- No model is loaded
- model runtime has not registered an adapter row

Recovery steps:
- Register or load a model before expecting rows
- Inspect process ledger rows for the backing model process


</topic>

<topic id="command-inspector-replay-drive" status="wired" version="1.0.6" summary="inspector_replay_drive">

## Command: inspector_replay_drive

ID: `inspector_replay_drive`

Status: `wired`

IPC channel: `/inspector/v1/replay-drive`

Description: POST the only inspector-plane mutation path through KernelActionCatalogV1 plus a verified WriteBoxV1 envelope.

Expected input: JSON body with exactly action_id and envelope. Any extra or missing top-level field is a forbidden parallel-mutation attempt.

Expected output: ReplayDriveResponse with dispatched status, write_box_id, result_schema_id, and INSPECTOR_REPLAY_DRIVE EventLedger receipt metadata.

Schema fields:
- action_id
- envelope
- schema_id
- signer
- signature
- write_box
- event_type

Common errors:
- Malformed JSON
- forbidden body shape
- invalid WriteBoxV1 envelope signature
- unknown KernelActionCatalogV1 action_id

Recovery steps:
- Send only { action_id, envelope }
- Regenerate the envelope signature from the WriteBoxV1 payload
- Check the action catalog before retrying


</topic>

<topic id="command-swarm-harness-run" status="wired" version="1.0.6" summary="swarm_harness_run">

## Command: swarm_harness_run

ID: `swarm_harness_run`

Status: `wired`

Description: Run a bounded multi-agent harness scenario through the shared ScenarioRegistry and return observable session evidence.

Expected input: A registered scenario_id and positive session count n.

Expected output: SwarmReport with per-session results, ProcessLedger START/STOP rows, contention events, and ledger overflow count.

Schema fields:
- scenario_id
- n
- sessions
- contention_events
- ledger_overflow_count

Common errors:
- Unknown scenario_id
- n is zero
- ProcessLedger writer channel saturated

Recovery steps:
- Call the registry-backed n8-perf, session-cancel, or lease-contention scenario id
- Reduce n if host capacity is limited
- Inspect ledger_overflow_count before retrying


</topic>

<topic id="command-handshake-swarm-cli" status="wired" version="1.0.6" summary="handshake-swarm">

## Command: handshake-swarm

ID: `handshake_swarm_cli`

Status: `wired`

CLI flag: `--scenario`

Description: Command-line entrypoint for the KERNEL-004 swarm harness; runs registered scenarios without app-runtime.

Expected input: `handshake-swarm --scenario <n8-perf|session-cancel|lease-contention> --n <count> [--report <path>]`.

Expected output: SwarmReport JSON on stdout, or a JSON receipt containing report_path when --report is provided.

Schema fields:
- scenario_id
- n
- report_path

Common errors:
- Missing --scenario
- Missing --n
- Unknown scenario id
- Report path parent cannot be created

Recovery steps:
- Run with --help to inspect syntax
- Use one of the documented scenario ids
- Write reports under the configured artifact root


</topic>

<topic id="command-kernel-swarm-run" status="wired" version="1.0.6" summary="kernel_swarm_run">

## Command: kernel_swarm_run

ID: `kernel_swarm_run`

Status: `wired`

IPC channel: `kernel.swarm.run`

Tauri command: `kernel_swarm_run`

Description: Debug-build Tauri IPC bridge that launches the swarm harness from Diagnostics-panel tooling through kernel.swarm.run.

Expected input: JSON arguments with scenario_id and positive n; available only in debug builds compiled with feature swarm_ipc.

Expected output: SwarmReport JSON matching the handshake-swarm CLI output shape.

Schema fields:
- scenario_id
- n
- sessions
- ledger_overflow_count

Common errors:
- App not compiled with swarm_ipc
- Release build omitted the command
- Unknown scenario_id
- n is zero

Recovery steps:
- Rebuild the Tauri app in debug mode with --features swarm_ipc
- Use handshake-swarm CLI when IPC is unavailable
- Retry with n8-perf, session-cancel, or lease-contention


</topic>

<topic id="command-swarm-scenario-n8-perf" status="wired" version="1.0.6" summary="n8-perf">

## Command: n8-perf

ID: `swarm_scenario_n8_perf`

Status: `wired`

CLI flag: `--scenario`

Description: n8-perf scenario: run high-volume catalog-backed mutation steps for each requested swarm session.

Expected input: scenario_id n8-perf and positive n; canonical proof uses n=8 with 100 mutations per session.

Expected output: SwarmReport with 100 completed catalog mutation steps per session and zero ledger overflow for healthy runs.

Schema fields:
- scenario_id
- n
- sessions

Common errors:
- Host too slow for high-volume proof
- Unknown action catalog id

Recovery steps:
- Start with --n 2 for smoke testing
- Use the hbr-swarm-n8 proof for the full HBR-SWARM evidence floor


</topic>

<topic id="command-swarm-scenario-session-cancel" status="wired" version="1.0.6" summary="session-cancel">

## Command: session-cancel

ID: `swarm_scenario_session_cancel`

Status: `wired`

CLI flag: `--scenario`

Description: session-cancel scenario: exercise cancellation propagation through the ProcessLedger reclaim seam.

Expected input: scenario_id session-cancel and positive n.

Expected output: SwarmReport with each session completing an OperatorCancel reclaim step and closing with START/STOP process rows.

Schema fields:
- scenario_id
- n
- sessions

Common errors:
- Reclaim trigger missing from scenario steps
- ProcessLedger STOP evidence absent

Recovery steps:
- Inspect per-session reclaim_triggers
- Run process_ledger_reclaim_tests if STOP evidence is missing


</topic>

<topic id="command-swarm-scenario-lease-contention" status="wired" version="1.0.6" summary="lease-contention">

## Command: lease-contention

ID: `swarm_scenario_lease_contention`

Status: `wired`

CLI flag: `--scenario`

Description: lease-contention scenario: make every requested session target the shared lease workspace through the catalog lease action.

Expected input: scenario_id lease-contention and positive n.

Expected output: SwarmReport with shared-resource mutation attempts, inspector reads, and closed sessions for contention debugging.

Schema fields:
- scenario_id
- n
- sessions

Common errors:
- Lease action catalog entry missing
- Shared workspace evidence is ambiguous

Recovery steps:
- Verify kernel.role_mailbox_claim_lease.project exists in KernelActionCatalogV1
- Use per-session envelope refs to trace the contested resource


</topic>

<topic id="command-sandbox-adapter-run" status="planned" version="1.0.6" summary="sandbox_adapter_run">

## Command: sandbox_adapter_run

ID: `sandbox_adapter_run`

Status: `planned`

IPC channel: `kernel.sandbox.run`

Tauri command: `sandbox_adapter_run`

Description: Run model-written code or validation work inside the selected SandboxAdapter.

Expected input: Adapter id, workspace ref, command descriptor, and ownership context.

Expected output: Sandbox run result with process ledger references and denial evidence when blocked.

Schema fields:
- adapter_id
- workspace_ref
- command_descriptor
- process_uuid

Common errors:
- Adapter unavailable
- command descriptor requests undeclared filesystem or network access

Recovery steps:
- Check sandbox_adapter_health
- Repair the command descriptor allowlist and retry


</topic>

<topic id="command-sandbox-adapter-health" status="planned" version="1.0.6" summary="sandbox_adapter_health">

## Command: sandbox_adapter_health

ID: `sandbox_adapter_health`

Status: `planned`

IPC channel: `kernel.sandbox.health`

Tauri command: `sandbox_adapter_health`

Description: Report adapter availability and host capability gaps before a sandbox run starts.

Expected input: Optional adapter id filter.

Expected output: Health rows for WSL2 Podman, Windows native jail, Docker compatibility, and future adapters.

Schema fields:
- adapter_id
- availability
- reason
- host_capabilities

Common errors:
- Host capability probe failed
- adapter is intentionally compatibility-only

Recovery steps:
- Select an available adapter
- Use Docker only when explicitly opted in


</topic>

<topic id="command-model-runtime-generate" status="planned" version="1.0.6" summary="model_runtime_generate">

## Command: model_runtime_generate

ID: `model_runtime_generate`

Status: `planned`

IPC channel: `kernel.model_runtime.generate`

Tauri command: `model_runtime_generate`

Description: Generate text through the bound local ModelRuntime adapter.

Expected input: Model id, prompt, and generation options.

Expected output: Generated text plus runtime metrics, process ledger refs, and cache metadata.

Schema fields:
- model_id
- prompt
- runtime_adapter
- generation_options
- process_uuid

Common errors:
- Model not loaded
- runtime adapter does not support requested technique

Recovery steps:
- Register or load the model
- Disable unsupported inference-lab toggles


</topic>

<topic id="command-model-runtime-register-model" status="planned" version="1.0.6" summary="model_runtime_register_model">

## Command: model_runtime_register_model

ID: `model_runtime_register_model`

Status: `planned`

IPC channel: `kernel.model_runtime.register_model`

Description: Catalog-only governed action for registering a local model artifact with immutable adapter binding and capability metadata.

Expected input: Artifact path, sha256, adapter kind, and capability declaration.

Expected output: Model registry entry available to runtime and inference-lab surfaces after the governed action is wired.

Schema fields:
- model_id
- artifact_path
- sha256
- adapter_kind
- capabilities

Common errors:
- Artifact hash mismatch
- adapter kind unsupported for artifact format

Recovery steps:
- Recompute sha256
- Choose LlamaCppRuntime for GGUF or CandleRuntime for safetensors where supported
- Use catalog action kernel.model_runtime.register_model rather than a Tauri command until the IPC surface is implemented


</topic>

<topic id="command-inference-lab-apply-technique" status="planned" version="1.0.6" summary="inference_lab_apply_technique">

## Command: inference_lab_apply_technique

ID: `inference_lab_apply_technique`

Status: `planned`

IPC channel: `kernel.inference_lab.apply_technique`

Tauri command: `inference_lab_apply_technique`

Description: Apply a supported inference technique such as LoRA, KV cache controls, steering vectors, or speculative decoding.

Expected input: Technique id, model id, enabled flag, and technique parameters.

Expected output: Updated runtime configuration or a typed unsupported-technique failure.

Schema fields:
- technique_id
- enabled
- parameters
- model_id

Common errors:
- Technique unsupported by selected adapter
- parameter range invalid

Recovery steps:
- Inspect model capabilities
- Reset the technique to its default parameters


</topic>

<topic id="command-memory-capsule-build" status="planned" version="1.0.6" summary="memory_capsule_build">

## Command: memory_capsule_build

ID: `memory_capsule_build`

Status: `planned`

IPC channel: `kernel.memory_capsule.build`

Tauri command: `memory_capsule_build`

Description: Build a bounded memory capsule for a task from approved retrieval sources.

Expected input: Task kind, token budget, and retrieval policy parameters.

Expected output: Memory capsule artifact and audit reference.

Schema fields:
- task_kind
- budget_tokens
- capsule_refs
- audit_ref

Common errors:
- Capsule exceeds token budget
- retrieval policy returns no eligible sources

Recovery steps:
- Lower top-k or capsule size
- Inspect retrieval audit refs


</topic>

<topic id="command-memory-calibration-snapshot" status="wired" version="1.0.6" summary="memory_calibration_snapshot">

## Command: memory_calibration_snapshot

ID: `memory_calibration_snapshot`

Status: `wired`

IPC channel: `kernel.memory_calibration.snapshot`

Tauri command: `kernel_memory_calibration_snapshot`

Description: Return the read-only FEMS calibration snapshot for operator and model diagnostics.

Expected input: No input.

Expected output: CalibrationSnapshot with six typed signal values, thresholds, timestamps, details, and per-signal source errors.

Schema fields:
- signals.bloat
- signals.stale_dominance
- signals.trust_drift
- signals.embedding_gap
- signals.degradation_rate
- signals.hygiene_lag
- signal_errors

Common errors:
- Postgres memory calibration state unavailable
- invalid calibration threshold
- calibration source IO failed

Recovery steps:
- Check DATABASE_URL and Postgres availability
- Inspect signal_errors to identify which collector failed
- Run calibration_tests for threshold and snapshot-shape regression coverage


</topic>

<topic id="command-self-improvement-run-iteration" status="planned" version="1.0.6" summary="self_improvement_run_iteration">

## Command: self_improvement_run_iteration

ID: `self_improvement_run_iteration`

Status: `planned`

IPC channel: `kernel.self_improvement.run_iteration`

Tauri command: `self_improvement_run_iteration`

Description: Run one bounded self-improvement iteration against the fixed HBR corpus.

Expected input: Corpus id, editable surface parameters, and promotion thresholds.

Expected output: Iteration report with dev and holdout metrics plus promotion decision.

Schema fields:
- corpus_id
- dev_score
- holdout_score
- promotion_decision

Common errors:
- Goodhart sentinel paused the loop
- holdout score below promotion floor

Recovery steps:
- Review promotion gate evidence
- Adjust only approved retrieval or manual parameters


</topic>

<topic id="command-distillation-candidate-review" status="planned" version="1.0.6" summary="distillation_candidate_review">

## Command: distillation_candidate_review

ID: `distillation_candidate_review`

Status: `planned`

IPC channel: `kernel.distillation.review_candidate`

Tauri command: `distillation_candidate_review`

Description: Review a distillation candidate before promotion into a model runtime surface.

Expected input: Candidate id and review decision payload.

Expected output: Promotion, rejection, or remediation receipt with review evidence refs.

Schema fields:
- candidate_id
- license_tags
- pii_scan
- operator_decision

Common errors:
- PII scan failed
- license tag missing
- operator decision absent

Recovery steps:
- Run content review again
- Attach required provenance and operator decision refs


</topic>

<topic id="safety-manual-same-commit-currency" status="current" version="1.0.6" summary="manual_same_commit_currency">

## Safety Constraint: manual_same_commit_currency

Constraint: HBR-MAN-001 requires every wired surface diff to update ModelManual content and bump MANUAL_VERSION in the same commit.

Enforcement point: hbr-man-001 paired-diff check

</topic>

<topic id="safety-manual-no-context-operation" status="current" version="1.0.6" summary="manual_no_context_operation">

## Safety Constraint: manual_no_context_operation

Constraint: HBR-MAN-002 requires a no-context model to operate core workflows from ModelManual alone without chat history.

Enforcement point: manual no-context harness

</topic>

<topic id="safety-quiet-model-operation" status="current" version="1.0.6" summary="quiet_model_operation">

## Safety Constraint: quiet_model_operation

Constraint: HBR-QUIET rules require diagnostics, GUI testing, and model-driven interaction to stay non-intrusive and observable.

Enforcement point: visual debugging and process-ledger checks

</topic>

<topic id="safety-typed-hbr-receipts" status="current" version="1.0.6" summary="typed_hbr_receipts">

## Safety Constraint: typed_hbr_receipts

Constraint: HBR violations route through typed HBR_VIOLATION receipts; notes may add context but never carry routing authority.

Enforcement point: hbr-violation.schema.json

</topic>

<topic id="workflow-startup" status="current" version="1.0.6" summary="Startup">

## Workflow: Startup

ID: `startup`

Prerequisites:
- Open the active worktree
- Read local AGENTS.md and packet authority
- Confirm the active microtask claim

Steps:
- Read the ModelManual manifest through kernel.model_manual.get when the IPC bridge is available.
- List command references and identify which feature group owns the task area.
- Run the required proof commands from the active microtask before claiming completion.

Expected outcome: The model knows the command surface, safety constraints, and verification path without prior chat context.

Failure modes:
- Manual IPC bridge not registered
- Command reference is planned and not yet callable


</topic>

<topic id="workflow-governed-session-run" status="current" version="1.0.6" summary="Basic Governed Session Run">

## Workflow: Basic Governed Session Run

ID: `governed_session_run`

Prerequisites:
- Active work packet
- Claimed microtask
- Known proof commands

Steps:
- Use the feature group to find command references relevant to the microtask.
- Execute only wired commands unless a planned entry is explicitly being implemented.
- Emit typed receipts or events for HBR failures and rerun verification after repair.

Expected outcome: The governed session completes with traceable evidence and no stale manual assumptions.

Failure modes:
- HBR matrix row remains PENDING
- Evidence pointer cannot be resolved
- MANUAL_VERSION not bumped after a wired-surface diff


</topic>

<topic id="workflow-diagnostics-panel-triage" status="current" version="1.0.6" summary="Diagnostics Panel Triage">

## Workflow: Diagnostics Panel Triage

ID: `diagnostics_panel_triage`

Prerequisites:
- Diagnostics capture surface available
- Target app route or backend state selector

Steps:
- Capture a headless visual or backend snapshot without foreground focus.
- Inspect stable element ids, logs, traces, and process ledger refs.
- Attach screenshot, event, artifact, or receipt evidence to the relevant HBR row.

Expected outcome: A no-context model can isolate UI or runtime failures without screen-scraping guesses.

Failure modes:
- Capture attempts to open a foreground window
- Snapshot lacks stable identifiers
- Process ledger START/STOP evidence is missing


</topic>

