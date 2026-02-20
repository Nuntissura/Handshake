# Task Packet: WP-1-Cloud-Escalation-Consent-v2

## METADATA
- TASK_ID: WP-1-Cloud-Escalation-Consent-v2
- WP_ID: WP-1-Cloud-Escalation-Consent-v2
- BASE_WP_ID: WP-1-Cloud-Escalation-Consent (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-19T23:58:35.245Z
- MERGE_BASE_SHA: dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4
- REQUESTOR: Operator (ilja)
- AGENT_ID: codex-cli:gpt-5.2 (orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: codex-cli:gpt-5.2
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja200220260034
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: ALLOWED
- OPERATOR_APPROVAL_EVIDENCE: coder A can use agents.
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Cloud-Escalation-Consent-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the Master Spec v02.133 Cloud Escalation consent flow and enforcement: require ProjectionPlan + ConsentReceipt binding before any outbound cloud invocation; enforce WorkProfile.governance.allow_cloud_escalation and LOCKED fail-closed behavior; emit FR-EVT-CLOUD-001..004 and validate schemas at Flight Recorder ingestion; satisfy conformance tests T-CLOUD-001..005.
- Why: Prevent accidental/exfiltrative external transmission and make cloud escalation auditable and deterministic (tamper-evident consent + leak-safe telemetry).
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/llm/guard.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/openai_compat.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - Handshake_Master_Spec_v02.133.md
  - .GOV/scripts/validation/refinement-check.mjs
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - .GOV/refinements/WP-1-Cloud-Escalation-Consent-v1.md
  - app/src/components/operator/JobsView.tsx
  - app/src/lib/api.ts
- OUT_OF_SCOPE:
  - Any Master Spec changes (already resolved in v02.133)
  - Adding new cloud providers beyond wiring the consent/artifact flow
  - Emitting raw payloads (or secrets/PII) to Flight Recorder or logs

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Cloud-Escalation-Consent-v2

# Backend format/lint/test:
cd src/backend/handshake_core
cargo fmt
cargo clippy --all-targets --all-features
cargo test

# Frontend (only if app/ is touched):
# cd app
# pnpm test

just cargo-clean
just post-work WP-1-Cloud-Escalation-Consent-v2 --range dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD
```

### DONE_MEANS
- Any outbound cloud invocation is blocked unless a valid ProjectionPlan + ConsentReceipt pair is present and binds (projection_plan_id + payload_sha256) per 11.1.7 (T-CLOUD-001, T-CLOUD-002).
- If GovernanceMode/AutomationLevel is LOCKED, cloud escalation is blocked (fail-closed; no consent prompt) and a denial event is emitted (T-CLOUD-004).
- If WorkProfile.governance.allow_cloud_escalation=false, cloud escalation is blocked and a denial event is emitted.
- FR-EVT-CLOUD-001..004 are emitted at the correct lifecycle points (requested/approved/denied/executed) and remain leak-safe (no raw payloads) per 11.5.8 + 11.5.8.1.
- Conformance tests T-CLOUD-001..005 pass (either as new automated tests or as validated end-to-end evidence in the packet EVIDENCE section).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.133.md (recorded_at: 2026-02-19T23:58:35.245Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.133.md 11.1.7, CloudEscalationRequest Schema, 4.3.7, 11.5.8, 10.5
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.
- Prior stub packet: `.GOV/task_packets/stubs/WP-1-Cloud-Escalation-Consent-v1.md` (preserve intent: consent artifacts + enforcement + FR events).
- Prior refinement attempt: `.GOV/refinements/WP-1-Cloud-Escalation-Consent-v1.md` (blocked by spec ambiguity; superseded by v2 after Spec Enrichment v02.133 aligned FR-EVT-CLOUD catalog).

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.133.md
  - src/backend/handshake_core/src/llm/guard.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "CloudEscalationGuard"
  - "CloudEscalationDenied"
  - "HSK-403-CLOUD-CONSENT-REQUIRED"
  - "HSK-403-CLOUD-CONSENT-MISMATCH"
  - "GOV_GATE_TYPE_CLOUD_ESCALATION"
  - "FlightRecorder"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Cloud-Escalation-Consent-v2
  cd src/backend/handshake_core && cargo test
  ```
- RISK_MAP:
  - "consent spoofing" -> "external transmission without valid binding"
  - "digest mismatch (TOCTOU)" -> "user-approved hash differs from transmitted bytes"
  - "LOCKED bypass" -> "unsafe autonomous escalation with no human intervention"
  - "payload leakage in FR/logs" -> "secrets/PII disclosure"

## SKELETON

### PROPOSED_INTERFACES_TYPES_CONTRACTS
- Cloud consent artifacts (Spec 11.1.7, schema v0.4):
  - `ProjectionPlanV0_4` (`hsk.projection_plan@0.4`)
  - `ConsentReceiptV0_4` (`hsk.consent_receipt@0.4`)
  - Keep/relocate existing structs from `src/backend/handshake_core/src/llm/guard.rs` into a shared `llm::cloud_escalation` module so both guard + workflows can use them.
- Cloud escalation request (Spec CloudEscalationRequest schema, v0.4):
  - New `CloudEscalationRequestV0_4` (`hsk.cloud_escalation@0.4`) with required fields: `request_id`, `wp_id`, `mt_id`, `reason`, `local_attempts`, `last_error_summary`, `requested_model_id`, `projection_plan_id`, `consent_receipt_id`.
  - New `CloudEscalationBundleV0_4` (request + ProjectionPlan + ConsentReceipt) used at the trust boundary (server validates bindings + digest).
- Outbound payload hashing + binding (Spec 2.6.6.7.0 canonical serialization + hashing; Refinement red-team advisory):
  - New helper `canonical_json_bytes_nfc(Value) -> Vec<u8>` and `sha256_hex(bytes) -> String`.
  - Binding definition for cloud consent: `payload_sha256 = sha256(canonical_json_bytes_nfc(final_outbound_request_body_json))`.
  - Cloud adapter MUST transmit the same canonical bytes used for hashing so digest matches transmitted payload (T-CLOUD-002).
- LLM invocation plumbing (backend enforcement boundary):
  - Extend `llm::CompletionRequest` with `cloud_escalation: Option<CloudEscalationBundleV0_4>` (serde default + skip when None).
  - Update `CloudEscalationGuard` to read `CompletionRequest.cloud_escalation` (per-invocation) for artifacts (not env-only), and to compute/verify payload_sha256 from the final outbound bytes before calling the inner adapter.
  - Policy enforcement remains fail-closed: deny in LOCKED, deny when allow_cloud_escalation=false, deny on missing/invalid/mismatched artifacts.
- Flight Recorder (Spec 11.5.8 + 11.5.8.1 canonical event family):
  - Add `FlightRecorderEventType` variants for cloud escalation: Requested/Approved/Denied/Executed.
  - Add `validate_cloud_escalation_event_payload(payload, expected_type)` enforcing the Spec 11.5.8 CloudEscalationEvent shape + leak-safe bounds.
  - Emit FR-EVT-CLOUD-001..004 at lifecycle points from workflows/guard (see END_TO_END_CLOSURE_PLAN below).
- Workflows (cloud escalation is always human-gated; Spec 11.1.7.3):
  - When escalation chain reaches a cloud tier: create ProjectionPlan + CloudEscalationRequest and persist as workspace artifacts under the job dir; emit FR-EVT-CLOUD-001; pause for consent.
  - On resume: load ConsentReceipt artifact, validate bindings, emit FR-EVT-CLOUD-002, then invoke LLM with `CompletionRequest.cloud_escalation` populated; guard emits FR-EVT-CLOUD-004 immediately before outbound dispatch; emit FR-EVT-CLOUD-003 on denial paths.

### OPEN_QUESTIONS
- UI vs backend-only consent capture: do we implement ProjectionPlan display + consent capture in `app/` in this WP, or implement a backend artifact + pause flow (JobState::AwaitingUser) and leave UI wiring for a follow-up WP?
- Payload model for hashing (T-CLOUD-002): confirm `ProjectionPlan.payload_sha256` binds to the canonical JSON bytes of the actual OpenAI-compatible request body that is transmitted (model + messages + params), not just raw prompt bytes.
- Request identity: request_id as `trace_id` string vs deterministic UUID derived from (job_id, wp_id, mt_id, to_model/to_level). Proposed: deterministic request_id to avoid duplicates on retry, while still recording trace_id in the Flight Recorder envelope.
- user_id source for ConsentReceipt: expected from UI/session; if unavailable, require it via job input rather than minting a dummy.
- WorkProfile mapping: treat `ExecutionPolicy.cloud_escalation_allowed` as WorkProfile.governance.allow_cloud_escalation for micro_task_executor_v1 until full WorkProfile system is implemented.

### NOTES
- Current code already has env-based `CloudEscalationGuard` in `src/backend/handshake_core/src/llm/guard.rs`, but hashing is prompt-bytes and consent artifacts are read once at startup. This WP shifts to per-invocation artifacts and canonical JSON hashing so the digest can match transmitted bytes.
- `workflows.rs` already enforces LOCKED fail-closed (forces cloud_escalation_allowed=false) and has a pause/resume "human gate" mechanism; cloud escalation consent will reuse that so consent is always human-driven even when AutomationLevel is AUTONOMOUS.

### END_TO_END_CLOSURE_PLAN (SKELETON)
- Producer/output fields:
  - ProjectionPlan.payload_sha256: computed server-side from canonical outbound bytes.
  - ConsentReceipt: created by human/UI and validated server-side; must bind to (projection_plan_id, payload_sha256).
  - CloudEscalationRequest: created server-side when escalation is queued; persisted as an artifact; referenced by FR-EVT-CLOUD-* via request_id.
- Transport/schema changes:
  - `CompletionRequest.cloud_escalation` carries the (request + artifacts) bundle across the enforcement boundary to the cloud adapter call site.
- Trust boundary + verification:
  - Treat ProjectionPlan/ConsentReceipt as untrusted until server recomputes payload_sha256 and validates bindings + policy (LOCKED + allow_cloud_escalation).
  - Do not trust client-provided provenance for wp_id/mt_id/model_id without server correlation (prefer job state/inputs).
- Audit/event/log payload:
  - Emit only IDs/hashes (request_id, projection_plan_id, consent_receipt_id, payload_sha256, wp_id?, mt_id?, trace_id, requested_model_id); never include raw payload or prompt text.
  - Events are schema-validated at ingestion by `FlightRecorderEvent::validate` (DuckDB recorder gate).
- Error taxonomy:
  - Missing consent: HSK-403-CLOUD-CONSENT-REQUIRED.
  - Binding/digest mismatch: HSK-403-CLOUD-CONSENT-MISMATCH.
  - Policy denied (LOCKED / allow_cloud_escalation=false): HSK-403-GOVERNANCE-LOCKED or HSK-403-CLOUD-ESCALATION-DENIED.
- Determinism:
  - Canonical JSON serialization + NFC + SHA-256 for payload_sha256; cloud adapter transmits the same canonical bytes used for hashing.
  - Post-work range: dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD (per packet TEST_PLAN).

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: client(UI)->backend(enforcement)->cloud adapter
- SERVER_SOURCES_OF_TRUTH:
  - WorkProfile resolved + pinned profile_id for the job/session (do not trust client toggles)
  - ProjectionPlan + ConsentReceipt parsed and validated server-side (bindings enforced; hash computed from final bytes)
  - Governance LOCKED state (fail-closed)
- REQUIRED_PROVENANCE_FIELDS:
  - request_id, projection_plan_id, consent_receipt_id, payload_sha256
  - job_id, wp_id, mt_id, trace_id
  - requested_model_id
- VERIFICATION_PLAN:
  - Reject on missing consent, mismatched projection_plan_id, mismatched payload_sha256, or LOCKED/allow_cloud_escalation=false.
  - Emit FR-EVT-CLOUD-001..004 events using IDs/hashes only; no raw payloads.
- ERROR_TAXONOMY_PLAN:
  - Missing consent (HSK-403-CLOUD-CONSENT-REQUIRED)
  - Consent binding mismatch / digest mismatch (HSK-403-CLOUD-CONSENT-MISMATCH)
  - Policy denied (HSK-403-CLOUD-ESCALATION-DENIED)
- UI_GUARDRAILS:
  - Display ProjectionPlan summary + payload_sha256 before approval; disable approve when LOCKED or allow_cloud_escalation=false; make denial explicit.
- VALIDATOR_ASSERTIONS:
  - All cloud invocations are gated by consent artifacts + policy (11.1.7, 4.3.7) and emit canonical FR events (11.5.8); conformance tests in 10.5 are satisfied.

## IMPLEMENTATION
- Backend:
  - Enforce consent bindings (ProjectionPlan + ConsentReceipt + canonical request bytes digest) at the cloud escalation trust boundary.
  - Add pause/resume flow + consent recording endpoints for jobs (`/api/jobs/:id/cloud_escalation/consent`, `/api/jobs/:id/resume`).
  - Emit and validate Flight Recorder events for cloud escalation lifecycle (requested/approved/denied/executed).
- UI (`app/`):
  - Display ProjectionPlan + server-computed `payload_sha256`.
  - Capture a stable local `user_id` and submit approve/deny, then resume the job.

## HYGIENE
- See `## EVIDENCE` for command outputs and exit codes.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: for `just post-work ... --range <base>..HEAD`, `Pre-SHA1` is the SHA1 of the file bytes at `<base>` and `Post-SHA1` is the SHA1 of the file bytes at `HEAD`.

- **Target File**: `Handshake_Master_Spec_v02.133.md`
- **Start**: 1
- **End**: 68234
- **Line Delta**: 68234
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `9dac473bd1aa01b6d2900874169869c915fc355f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `app/src/components/operator/JobsView.tsx`
- **Start**: 10
- **End**: 421
- **Line Delta**: 120
- **Pre-SHA1**: `d522830660c52f807fc9418068b46d2f345c613a`
- **Post-SHA1**: `fdb79747dd93d8a10c532c00cc79573eb44b5686`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `app/src/lib/api.ts`
- **Start**: 660
- **End**: 683
- **Line Delta**: 24
- **Pre-SHA1**: `14a9485f1f6cff0203f8d592f2f3f855ffb80062`
- **Post-SHA1**: `9614f5eecb99ed45d2cd26b41850b446101ac3b0`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/api/jobs.rs`
- **Start**: 1
- **End**: 388
- **Line Delta**: 137
- **Pre-SHA1**: `49b8c6c3c6d7d83cb1a6b93245984b9eb4a3ea27`
- **Post-SHA1**: `f20fa2f70fff95b80656df7727f42ac43d777793`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 774
- **End**: 783
- **Line Delta**: 10
- **Pre-SHA1**: `9be8b53607d400a5a1366ce8c75c49166e5ddfda`
- **Post-SHA1**: `a2052209cb68b398c77d558eb6e075a854c299b7`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 104
- **End**: 3554
- **Line Delta**: 143
- **Pre-SHA1**: `5edf703771c18f4697901d0ead275d0f32b3386e`
- **Post-SHA1**: `9800c3816675c821e14572b4fb179d27c3828563`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/llm/guard.rs`
- **Start**: 1
- **End**: 467
- **Line Delta**: 17
- **Pre-SHA1**: `ddb026260596c43f1f142de23d8e2a00e2791dac`
- **Post-SHA1**: `7eb568e8a669b047cfc4be3c63695496b5b35d5a`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/llm/mod.rs`
- **Start**: 1
- **End**: 548
- **Line Delta**: 198
- **Pre-SHA1**: `58a3b75611ce7f7354bbe51d3c96443f7bde8cba`
- **Post-SHA1**: `7f6acfdef2a761522cdeefa55f54abb9bf2ff639`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/llm/openai_compat.rs`
- **Start**: 8
- **End**: 415
- **Line Delta**: -24
- **Pre-SHA1**: `9a3bf868e58119f737a532d0a592b2746cbe369c`
- **Post-SHA1**: `ae9b0dbcfe1953c9d58a87c328dd59f546a42944`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 26
- **End**: 9205
- **Line Delta**: 615
- **Pre-SHA1**: `13773956b14c10256cce253fc3c7e7bc3a88583c`
- **Post-SHA1**: `ab092c8008362a8fe31fb363cdb9bab0c19f47aa`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.133.md
- **Notes**: `Handshake_Master_Spec_v02.133.md` does not exist at `MERGE_BASE_SHA` (new file in range); `Pre-SHA1` is the empty SHA1 for deterministic completeness.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (implementation complete; ready for Validator review)
- What changed in this update:
  - Implementation: `964660f` (cloud escalation consent flow + UI + FR event validation/mapping).
  - Packet hygiene/evidence: `07a7809`, `6b0ec5b` (manifest + evidence mapping + command evidence).
- Next step / handoff hint:
  - Validator: review evidence mapping vs DONE_MEANS/SPEC_ANCHOR and re-run gates/tests as needed.
  - Operator (optional): authorize reverting local rustfmt-only unstaged diffs (see `git status -sb`) to leave the WP worktree clean.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "Any outbound cloud invocation is blocked unless a valid ProjectionPlan + ConsentReceipt pair is present and binds (projection_plan_id + payload_sha256) per 11.1.7 (T-CLOUD-001, T-CLOUD-002)."
- EVIDENCE: `src/backend/handshake_core/src/llm/guard.rs:166`
- EVIDENCE: `src/backend/handshake_core/src/llm/guard.rs:172`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:8415`
- REQUIREMENT: "If GovernanceMode/AutomationLevel is LOCKED, cloud escalation is blocked (fail-closed; no consent prompt) and a denial event is emitted (T-CLOUD-004)."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:6500`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:9188`
- REQUIREMENT: "If WorkProfile.governance.allow_cloud_escalation=false, cloud escalation is blocked and a denial event is emitted."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:7249`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:7271`
- REQUIREMENT: "FR-EVT-CLOUD-001..004 are emitted at the correct lifecycle points (requested/approved/denied/executed) and remain leak-safe (no raw payloads) per 11.5.8 + 11.5.8.1."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:8500`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2271`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:8578`
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:624`
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:3458`
- REQUIREMENT: "Conformance tests T-CLOUD-001..005 pass (either as new automated tests or as validated end-to-end evidence in the packet EVIDENCE section)."
- EVIDENCE: `src/backend/handshake_core/src/llm/guard.rs:401`
- EVIDENCE: `src/backend/handshake_core/src/llm/guard.rs:428`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Cloud-Escalation-Consent-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: `just post-work WP-1-Cloud-Escalation-Consent-v2 --range dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD`
- EXIT_CODE: 1
- PROOF_LINES: "Errors: 1. EVIDENCE_MAPPING has no file:line evidence ... 9. Manifest[1]: Target file does not exist: path\\to\\file (C701-G06)"
- COMMAND: `just post-work WP-1-Cloud-Escalation-Consent-v2 --range dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD`
- EXIT_CODE: 0
- PROOF_LINES: "Post-work validation PASSED (deterministic manifest gate; not tests) with warnings"
- COMMAND: `cd src/backend/handshake_core; cargo fmt`
- EXIT_CODE: 0
- COMMAND: `cd src/backend/handshake_core; cargo clippy --all-targets --all-features`
- EXIT_CODE: 0
- PROOF_LINES: "Finished `dev` profile [unoptimized + debuginfo]"
- COMMAND: `cd src/backend/handshake_core; cargo test -q`
- EXIT_CODE: 0
- PROOF_LINES: "test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
- COMMAND: `cd app; pnpm test`
- EXIT_CODE: 0
- PROOF_LINES: "Test Files 6 passed (6); Tests 13 passed (13)"
- COMMAND: `just validator-scan`
- EXIT_CODE: 0
- PROOF_LINES: "validator-scan: PASS - no forbidden patterns detected in backend/frontend sources."
- COMMAND: `just validator-error-codes`
- EXIT_CODE: 0
- PROOF_LINES: "validator-error-codes: PASS - no stringly errors or nondeterminism patterns detected."
- COMMAND: `cd src/backend/handshake_core; cargo test -q -j 1`
- EXIT_CODE: 0
- PROOF_LINES: "test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
- COMMAND: `just cargo-clean`
- EXIT_CODE: 0
- PROOF_LINES: "Removed 2346 files, 13.6GiB total"
- COMMAND: `just post-work WP-1-Cloud-Escalation-Consent-v2 --range dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD`
- EXIT_CODE: 0
- PROOF_LINES: "Git range: dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..b881593edda9cf473f3eac26c18be4451735e680"
- COMMAND: `just post-work WP-1-Cloud-Escalation-Consent-v2 --range dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD`
- EXIT_CODE: 0
- PROOF_LINES: "Git range: dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..1a8ec6898003068951ab660284cb0b02210712fe"
- COMMAND: `cd src/backend/handshake_core; cargo fmt`
- EXIT_CODE: 0
- COMMAND: `just validator-scan`
- EXIT_CODE: 0
- PROOF_LINES: "validator-scan: PASS - no forbidden patterns detected in backend/frontend sources."
- COMMAND: `just validator-error-codes`
- EXIT_CODE: 0
- PROOF_LINES: "validator-error-codes: PASS - no stringly errors or nondeterminism patterns detected."
- COMMAND: `cd src/backend/handshake_core; cargo clippy --all-targets --all-features`
- EXIT_CODE: 0
- PROOF_LINES: "Finished `dev` profile [unoptimized + debuginfo]"
- COMMAND: `cd src/backend/handshake_core; cargo test -q -j 1`
- EXIT_CODE: 0
- PROOF_LINES: "test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
- COMMAND: `just cargo-clean`
- EXIT_CODE: 0
- PROOF_LINES: "Removed 2306 files, 12.9GiB total"
- COMMAND: `just post-work WP-1-Cloud-Escalation-Consent-v2 --range dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..HEAD`
- EXIT_CODE: 0
- PROOF_LINES: "Git range: dfbf8d09a5753d15ea6c52916ee021bd36bcbbc4..4f053e00cdb643cbcd4835b61829881a4d797890"

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
