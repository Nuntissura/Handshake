# Task Packet: WP-1-Flight-Recorder-UI-v2

## Metadata
- TASK_ID: WP-1-Flight-Recorder-UI-v2
- WP_ID: WP-1-Flight-Recorder-UI-v2
- DATE: 2025-12-26T23:55:00Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: Done

## User Context
We are upgrading the "Flight Recorder" view, which is the system's "Black Box" recorder. It shows every decision and action the AI takes. This update makes it easier to read security alerts and navigate between a task, the commands it ran, and the documents it changed.

## Scope
- **What**: Enhance the Flight Recorder UI to comply with Â§11.5 and Â§10.5.
- **Why**: Strategic Audit alignment for Phase 1. The current UI is a placeholder and fails to show Character-accurate evidence mapping for security violations.
- **IN_SCOPE_PATHS**:
  * app/src/components/FlightRecorderView.tsx (Upgrade)
  * app/src/App.css (Styling for new event types)
  * app/src/lib/api.ts (Type definitions)
- **OUT_OF_SCOPE**:
  * Backend API changes (current `/api/flight_recorder` is sufficient).
  * Real-time streaming (polling is sufficient).

## Quality Gate
- **RISK_TIER**: MEDIUM
  - Justification: UI update for observability; no backend logic changes.
- **TEST_PLAN**:
  ```bash
  # 1. Frontend Lint & Type Check
  pnpm -C app run lint
  pnpm -C app run build  # Verify TS types in production build
  
  # 2. Visual Verification:
  # - Trigger a security violation (e.g. use 'force_prompt_injection' in job input)
  # - Verify the 'SecurityViolation' event is red/highlighted in Flight Recorder
  # - Verify offset and context snippet are displayed outside the JSON blob
  
  # 3. Navigation Verification:
  # - Clicking a Trace ID should filter the view to that specific trace.
  
  # 4. Final hygiene
  just post-work WP-1-Flight-Recorder-UI-v2
  ```
- **DONE_MEANS**:
  * âœ… UI renders `trace_id` and `actor_id` for all events per Â§11.5.
  * âœ… Security violations (`FR-EVT-SEC-VIOLATION`) are visually highlighted (e.g., red background or border).
  * âœ… Security violations display Character Offsets and Context Snippets directly in the list/expanded view (not hidden in JSON).
  * âœ… UI supports "Linkability": clicking a Trace ID filters for that trace.
  * âœ… All traceability fields (`job_id`, `workflow_id`) are rendered clearly.
  * âœ… Forensic evidence (payload hashes) is visible in the detailed payload view.

## ROLLBACK_HINT
```bash
git checkout app/src/components/FlightRecorderView.tsx app/src/App.css app/src/lib/api.ts
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * app/src/components/FlightRecorderView.tsx
  * .GOV/roles_shared/SPEC_CURRENT.md (v02.93 Â§11.5)
  * src/backend/handshake_core/src/flight_recorder/mod.rs (for normative event schemas)
  * app/src/lib/api.ts (for FlightEvent type)
- **SEARCH_TERMS**:
  * "FR-EVT-SEC-VIOLATION"
  * "trace_id"
  * "actor_id"
  * "PromptInjectionDetected"
- **RUN_COMMANDS**:
  ```bash
  pnpm -C app dev
  ```
- **RISK_MAP**:
  * "JSON Parsing Error" -> Payload rendering (Fix: use safe JSON.stringify)
  * "Z-Index Clashes" -> Detailed payload view overlapping navigation
  * "Performance Lag" -> Large log lists (Fix: limit initial fetch to 100 events)

## Authority
- **SPEC_CURRENT**: .GOV/roles_shared/SPEC_CURRENT.md (Master Spec v02.93)
- **SPEC_ANCHOR**: Â§11.5, Â§10.5
- **Strategic Audit Reference**: [WP-1-ACE-Validators-v3] (Provides the data this UI must render)

---

**Last Updated:** 2025-12-27
**User Signature Locked:** <pending>

---

## VALIDATION REPORT â€” WP-1-Flight-Recorder-UI-v2
Verdict: PASS

### Evidence Mapping (Spec â†’ Code)
- **Character-Accurate Evidence [Â§10.5]**: SATISFIED. `FlightRecorderView.tsx:151-175` extracts `offset` and `context` from the security violation payload and renders them outside the JSON blob.
- **Trace ID Linkability [Â§11.5]**: SATISFIED. `FlightRecorderView.tsx:68-77` implements `toggleTraceFilter`, which correctly updates the filter state and triggers a refresh.
- **Visual Hardening**: SATISFIED. `App.css:1-10` adds `flight-recorder__row--violation` with mandatory red signaling.
- **Type Safety**: SATISFIED. `app/src/lib/api.ts:151-160` implements `SecurityViolationPayload`.

### Tests Executed
- `pnpm run lint`: PASS
- `pnpm run build`: PASS
- Visual verification of red rows and context rendering: VERIFIED

### REASON FOR PASS
UI successfully surfaces character-accurate evidence and provides mandatory trace-level navigation per Master Spec Â§11.5. Forensic data is correctly modeled and displayed.

**STATUS:** VALIDATED

## VALIDATION REPORT â€” 2025-12-27 (Revalidation)
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Flight-Recorder-UI-v2.md (STATUS: Done)
- Spec: Handshake_Master_Spec_v02.93 (A11.5, A10.5) via .GOV/roles_shared/SPEC_CURRENT.md
- Codex: Handshake Codex v1.4.md

Files Checked:
- app/src/components/FlightRecorderView.tsx:60-191 (trace filter, security violation highlighting, context/offset rendering)
- app/src/lib/api.ts:136-161 (FlightEvent and SecurityViolationPayload typing)
- app/src/App.css:1-12 (violation row styling)

Findings:
- Trace ID filter toggles linkability and refreshes data; security violations render trigger/offset/context outside the JSON blob with red highlight.
- Traceability fields (job_id, workflow_id, trace_id) are surfaced per DONE_MEANS.
- Forbidden Pattern Audit [CX-573E]: PASS for in-scope files (no TODO/panic/unwrap/console.log).
- Zero Placeholder Policy [CX-573D]: PASS; UI logic and typings are fully implemented without stubs.

Tests:
- `pnpm -C app run lint` (PASS)
- `pnpm -C app run build` (PASS)

REASON FOR PASS: UI meets A11.5/A10.5 requirements for evidence visibility and trace navigation; lint/build succeed.

---

## VALIDATION REPORT - 2025-12-30 (Revalidation, Batch 5)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Flight-Recorder-UI-v2.md
- Spec (SPEC_CURRENT): .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md
- Protocol: .GOV/roles/validator/VALIDATOR_PROTOCOL.md

Commands Run:
- just validator-spec-regression: PASS
- just cargo-clean: PASS (Removed 0 files)
- just gate-check WP-1-Flight-Recorder-UI-v2: FAIL (Implementation detected without SKELETON APPROVED marker)
- node .GOV/scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-UI-v2: FAIL (non-ASCII + missing COR-701 validation manifest fields/gates)
- just validator-packet-complete WP-1-Flight-Recorder-UI-v2: FAIL (STATUS missing/invalid; requires canonical **Status:** Ready for Dev / In Progress / Done)
- just post-work WP-1-Flight-Recorder-UI-v2: FAIL (blocked at gate-check)

Blocking Findings:
- Phase gate violation [CX-GATE-001]: gate-check fails because implementation is present without a prior "SKELETON APPROVED" marker in this packet.
- Deterministic manifest gate (COR-701): post-work-check reports missing required manifest fields (target_file, start, end, pre_sha1, post_sha1, line_delta) and missing/unchecked gates (C701-G01, C701-G02, C701-G04, C701-G05, C701-G06, C701-G08).
- ASCII-only requirement: post-work-check reports non-ASCII characters in the task packet.
  - NON_ASCII_COUNT=18 (sample: Line 16 Col 59 U+00A7, Line 47 Col 5 U+2705, Line 91 Col 22 U+2014, Line 94 Col 28 U+2192)
- Spec mismatch: this packet asserts Master Spec v02.93, but .GOV/roles_shared/SPEC_CURRENT.md points to v02.98. Prior PASS claims are not valid against the current spec.

Manual Spot-Checks (evidence only; does not override the failures above):
- Trace filter clickability: app/src/components/FlightRecorderView.tsx:60-69 and app/src/components/FlightRecorderView.tsx:159-165.
- Security violation highlighting + offset/context rendering: app/src/components/FlightRecorderView.tsx:147-178 and app/src/App.css:280-298.
- Event typing includes security_violation and payload fields offset/context: app/src/lib/api.ts:136-165.

REASON FOR FAIL:
- Blocking process gates (phase gate + COR-701 manifest + ASCII-only + STATUS marker) fail; spec alignment to v02.98 is not demonstrated.

Required Fixes:
1) Bring this packet back into protocol: include proper BOOTSTRAP/SKELETON/IMPLEMENTATION/HYGIENE/VALIDATION sections and obtain explicit "SKELETON APPROVED" before implementation evidence.
2) Make the task packet ASCII-only (remove/replace non-ASCII characters; rerun post-work-check until clean).
3) Add a COR-701 validation manifest (target_file/start/end/pre_sha1/post_sha1/line_delta + gates checklist) and ensure `just post-work WP-1-Flight-Recorder-UI-v2` passes.
4) Re-anchor DONE_MEANS + evidence mapping to Handshake_Master_Spec_v02.98.md and revalidate against v02.98 requirements.

**Status:** Ready for Dev

Addendum (2025-12-30):
- The canonical **Status:** line above addresses the earlier status-marker failure, but packet completeness still fails because the required user signature field is missing/pending.



