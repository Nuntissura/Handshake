## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-MEX-v1.2-Runtime-v3
- CREATED_AT: 2026-01-01T22:14:20.1511391+01:00
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- SPEC_TARGET_SHA1: 76e8e6e8259b64a6dc4aed5cf2afb754ff1f3aed
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010120262219

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Gate outcomes MUST be logged to Flight Recorder and surfaced in Problems when denied or degraded (see 6.3.0 contract).
- FR-EVT-003 (DiagnosticEvent) is the Flight Recorder linkage mechanism for Problems/Diagnostics visibility.
- Capability checks MUST be audited (HSK-4001 audit requirement).

### RED_TEAM_ADVISORY (security failure modes)
- Default-allow capability execution: if requested caps are treated as granted, engine execution becomes an unbounded RCE surface.
- Bypass execution path: if engines can be invoked outside runtime, gates can be skipped.
- Artifact discipline violation: inline payloads >32KB pollute context and bypass provenance/evidence.
- Non-deterministic outputs without evidence_policy/evidence artifacts: undermines reproducibility and auditability.

### PRIMITIVES (traits/structs/enums)
- PlannedOperation (engine invocation envelope; schema_version=poe-1.0).
- EngineResult (result envelope with provenance/evidence/log refs).
- Gate pipeline labels: G-SCHEMA, G-CAP, G-INTEGRITY, G-BUDGET, G-PROVENANCE, G-DET.
- DiagnosticEvent (FR-EVT-003) for linking denials to Problems/Diagnostics.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.100 defines the normative MEX v1.2 contract (6.3.0 + 11.8) including envelopes and the required gate pipeline, and defines FR-EVT-003 for Problems linkage; no enrichment needed.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The spec already contains a spec-grade (verbatim) 11.8 contract plus the 6.3.0 normative gates/logging rules and FR-EVT-003 schema for diagnostics visibility.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 6.3.0 Mechanical Tool Bus Contract (required global gates; artifact-first; no-bypass)
- CONTEXT_START_LINE: 16770
- CONTEXT_END_LINE: 16808
- CONTEXT_TOKEN: G-PROVENANCE
- EXCERPT_ASCII_ESCAPED:
  ```text
Normative scope: \u00C2\u00A76.3.0 + \u00C2\u00A711.8.
Informational scope: \u00C2\u00A76.3.1\u00E2\u20AC\u201C\u00C2\u00A76.3.* engine notes (until upgraded).

---


### 6.3.0 Mechanical Tool Bus Contract (normative; MEX v1.2)

This subsection upgrades \u00C2\u00A76.3 from a descriptive catalogue into an **executable contract** for invoking mechanical engines. The canonical spec-grade contract (including full envelopes, gates, registry requirements, conformance vectors, and the 22-engine set) is imported in **\u00C2\u00A711.8**.

**Terminology and schema discrimination**
- **Engine PlannedOperation (EPO):** a single-engine invocation envelope with `schema_version = "poe-1.0"` (and later `poe-*` variants).  
- **Edit PlannedOperation (COR/BL):** any PlannedOperation used for document/patch semantics elsewhere in this Master Spec.  
- **Rule:** these MUST remain unambiguous via `schema_version` (and, where present, `protocol_id`). Engine invocations MUST use `poe-*`.

**Artifact-first I/O**
- **Size rule:** any payload > **32KB** MUST be passed via input artifacts (handles/refs), never inlined into the PlannedOperation.  
- Outputs MUST be exported as artifacts (immutable) with **SHA-256** hashing + sidecar provenance manifests (see Artifact rules in \u00C2\u00A72.3.10).

**Determinism levels (D0\u00E2\u20AC\u201CD3)**
- **D3 Bitwise:** identical inputs/config/environment \u00E2\u2021\u2019 identical bytes.  
- **D2 Structural:** identical semantics; bytes may differ; canonicalization required for stable hashes.  
- **D1 Best-effort:** depends on external/labile inputs; replay relies on captured evidence.  
- **D0 Live:** inherently non-replayable unless evidence capture \u00E2\u20AC\u0153freezes\u00E2\u20AC\u009D the claim.  
- **Evidence rule:** D0/D1 results MUST carry evidence artifacts referenced in EngineResult.

**Required global gates (minimum)**
- `G-SCHEMA` (validate envelopes)  
- `G-CAP` (capabilities/consent; default-deny)  
- `G-INTEGRITY` (artifact hash verification, path safety, no-bypass invariants)  
- `G-BUDGET` (time/memory/output caps; kill/timeout policy)  
- `G-PROVENANCE` (required provenance fields present; artifacts referenced, not inlined)  
- `G-DET` (determinism/evidence policy enforcement)

Gate outcomes MUST be logged to Flight Recorder and surfaced in Problems when denied or degraded.

**Registry + adapter resolution**
- Engines MUST be declared in `mechanical_engines.json` (engine_id, ops, determinism ceiling, required gates, default capabilities, conformance vectors, implementation/adapters).  
- **No-bypass:** engines MUST NOT be invokable outside the orchestrator/runtime.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.8 Mechanical Extension Specification v1.2 (Verbatim) 4.1 PlannedOperation envelope + 4.2 EngineResult envelope
- CONTEXT_START_LINE: 31645
- CONTEXT_END_LINE: 31685
- CONTEXT_TOKEN: 4.1 PlannedOperation envelope
- EXCERPT_ASCII_ESCAPED:
  ```text
**No-bypass:** engines MUST NOT be invokable outside the orchestrator/runtime.

#### 4. Mechanical Tool Bus Contract (normative, minimum)

##### 4.1 PlannedOperation envelope (minimum fields)

- `schema_version` (e.g., `poe-1.0`)
- `op_id` (UUID)
- `engine_id`
- `engine_version_req`
- `operation` (discriminator)
- `inputs` (ArtifactHandle[] / EntityRef[])
- `params` (engine-specific; MUST validate)
- `capabilities_requested`
- `budget`
- `determinism` (`D0|D1|D2|D3`)
- `evidence_policy` (required for D0/D1)
- `output_spec`

**Size rule:** any payload > 32KB MUST be passed as an input artifact.

##### 4.2 EngineResult envelope (minimum fields)

- `op_id`
- `status`
- `started_at`, `ended_at`
- `outputs` (ArtifactHandle[])
- `evidence` (ArtifactHandle[]; required for D0/D1)
- `provenance` (engine+impl+versions, inputs, outputs, config hash, capabilities granted, environment)
- `errors` (typed; may include `details_ref`)
- `logs_ref` (optional artifact)

#### 5. Capability and security model (normative, minimum)

Capabilities are explicit, least-privilege:

- `fs.read:<scope>`, `fs.write:artifacts`
- `net.http`
- `device.camera`, `device.mic`, `device.usb`, `device.serial`
- `proc.exec:<allowlist>`
- `gpu.compute`
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.1 Capabilities & Consent Model (HSK-4001 SSoT + audit requirement)
- CONTEXT_START_LINE: 29220
- CONTEXT_END_LINE: 29240
- CONTEXT_TOKEN: HSK-4001: UnknownCapability
- EXCERPT_ASCII_ESCAPED:
  ```text

- Scope/time-to-live defaults, approval caching, revocation UX, escalation paths, and capability axes for surfaces (terminal, editor, mail, calendar).
- Mapping to plugins and product surfaces:
  - Plugin manifest permissions MUST map to capability_profile_id entries; no ad-hoc plugin permissions.
  - Mail/Calendar surfaces MUST use the same capability/consent profiles (send_email, read_mail, export_calendar) used by plugin APIs and AI Job profiles.
  - Workflow/AI Job model MUST resolve effective capabilities from: plugin manifest (if tool), job profile, and surface-specific policy; the most restrictive wins.
- **Capability Registry & SSoT Enforcement ([HSK-4001]):**
  - The system MUST maintain a centralized `CapabilityRegistry` (SSoT) containing all valid Capability IDs (e.g. `fs.read`, `doc.summarize`, `terminal.exec`).
  - **Hard Invariant:** Any request for a Capability ID not defined in the Registry MUST be rejected with error `HSK-4001: UnknownCapability`. Ad-hoc or "magic string" capabilities are strictly forbidden.
  - **Audit Requirement:** Every capability check (Allow or Deny) MUST be recorded as a Flight Recorder event, capturing: `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
  - **Profile Schema:** `CapabilityProfile` objects (e.g. 'Analyst', 'Coder') MUST be defined solely as whitelists of IDs from the `CapabilityRegistry`.
- Redaction/safety propagation:
  - Content classification + redaction flags flow from data layer to plugin/tool calls and AI jobs; cloud routing MUST honor projection/redaction defaults per surface (mail/calendar/doc).
- Based on TERM-CAP: axes (model, workspace, command class/action type, time scope), approval types (per-job, per-model-per-workspace), visible \u00E2\u20AC\u0153Capabilities\u00E2\u20AC\u009D UI, revocation without restart, escalation flow with job/model/workspace/command context, decisions logged to Flight Recorder.
- Recommended defaults:
  - Per-job approvals as the default prompt; per-model-per-workspace approvals with 24h TTL by default.
  - \u00E2\u20AC\u0153Until revoked\u00E2\u20AC\u009D only as an explicit opt-in with warning.
  - Define approval classes for non-terminal surfaces (e.g., edit scopes for Monaco; send/read/sync scopes for mail/calendar).
- Canonical calendar capability identifiers (field projections/redaction rules: see \u00C2\u00A710.4):
  - `CALENDAR_READ_BASIC`
  - `CALENDAR_READ_DETAILS`
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 Flight Recorder Event Shapes - FR-EVT-003 (DiagnosticEvent)
- CONTEXT_START_LINE: 30875
- CONTEXT_END_LINE: 30920
- CONTEXT_TOKEN: interface DiagnosticEvent
- EXCERPT_ASCII_ESCAPED:
  ```text
  diff_hash?: string | null;

  // Edit details
  ops: EditorEditOp[];
}
```

- **FR-EVT-003 (DiagnosticEvent)**

A DiagnosticEvent links a Flight Recorder trace to a Diagnostic (`Diagnostic.id`) without duplicating the full Diagnostic payload.

```ts
interface DiagnosticEvent extends FlightRecorderEventBase {
  type: 'diagnostic';

  diagnostic_id: string;             // equals Diagnostic.id
  wsid?: string | null;
  severity?: 'fatal' | 'error' | 'warning' | 'info' | 'hint';
  source?: string;                   // optional echo for quick filtering
}
```

- **FR-EVT-004 (Retention & linkability)**

Flight Recorder MUST:
- Retain events for a configurable time window (default: 30 days),
- Allow navigation:
  - job trace \u00E2\u2020\u2019 terminal commands \u00E2\u20AC\u2019 opened files / edited documents,
  - diagnostics \u00E2\u2020\u2019 Problems \u00E2\u20AC\u2019 Monaco/other surface locations,
  - terminal events \u00E2\u2020\u2019 raw session output (subject to logging and redaction policy),
  - any operator action \u00E2\u2020\u2019 the initiating UI surface (see VAL-CONSOLE-001).

If evidence is missing due to retention, the UI MUST:
- show that evidence is missing,
- include the reason in Debug Bundle `retention_report.json`.

- **FR-EVT-005 (DebugBundleExportEvent)**
  ```

