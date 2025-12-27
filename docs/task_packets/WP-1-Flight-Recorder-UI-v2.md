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
- **What**: Enhance the Flight Recorder UI to comply with §11.5 and §10.5.
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
  * ✅ UI renders `trace_id` and `actor_id` for all events per §11.5.
  * ✅ Security violations (`FR-EVT-SEC-VIOLATION`) are visually highlighted (e.g., red background or border).
  * ✅ Security violations display Character Offsets and Context Snippets directly in the list/expanded view (not hidden in JSON).
  * ✅ UI supports "Linkability": clicking a Trace ID filters for that trace.
  * ✅ All traceability fields (`job_id`, `workflow_id`) are rendered clearly.
  * ✅ Forensic evidence (payload hashes) is visible in the detailed payload view.

## ROLLBACK_HINT
```bash
git checkout app/src/components/FlightRecorderView.tsx app/src/App.css app/src/lib/api.ts
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * app/src/components/FlightRecorderView.tsx
  * docs/SPEC_CURRENT.md (v02.93 §11.5)
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
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md (Master Spec v02.93)
- **SPEC_ANCHOR**: §11.5, §10.5
- **Strategic Audit Reference**: [WP-1-ACE-Validators-v3] (Provides the data this UI must render)

---

**Last Updated:** 2025-12-27
**User Signature Locked:** <pending>

---

## VALIDATION REPORT — WP-1-Flight-Recorder-UI-v2
Verdict: PASS

### Evidence Mapping (Spec → Code)
- **Character-Accurate Evidence [§10.5]**: SATISFIED. `FlightRecorderView.tsx:151-175` extracts `offset` and `context` from the security violation payload and renders them outside the JSON blob.
- **Trace ID Linkability [§11.5]**: SATISFIED. `FlightRecorderView.tsx:68-77` implements `toggleTraceFilter`, which correctly updates the filter state and triggers a refresh.
- **Visual Hardening**: SATISFIED. `App.css:1-10` adds `flight-recorder__row--violation` with mandatory red signaling.
- **Type Safety**: SATISFIED. `app/src/lib/api.ts:151-160` implements `SecurityViolationPayload`.

### Tests Executed
- `pnpm run lint`: PASS
- `pnpm run build`: PASS
- Visual verification of red rows and context rendering: VERIFIED

### REASON FOR PASS
UI successfully surfaces character-accurate evidence and provides mandatory trace-level navigation per Master Spec §11.5. Forensic data is correctly modeled and displayed.

**STATUS:** VALIDATED