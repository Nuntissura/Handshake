# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Layerwise-Inference-Foundations-v1

## STUB_METADATA
- WP_ID: WP-1-Layerwise-Inference-Foundations-v1
- BASE_WP_ID: WP-1-Layerwise-Inference-Foundations
- CREATED_AT: 2026-01-29T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.122.md 7.6.3 (Phase 1) -> [ADD v02.122 - Layer-wise Inference Foundations (Hooks + Governance + Observability)]
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.122.md 2.5.2.1 Model Runtime Layer Contract (Normative) [ADD v02.122]
  - Handshake_Master_Spec_v02.122.md 4.3.7.5 Work Profile Schema Extensions (Multi-Model + Dynamic Compute) (Normative) [ADD v02.122]
  - Handshake_Master_Spec_v02.122.md 11.5.11 FR-EVT-LLM-EXEC-001 (llm_exec_policy) + hsk.layerwise_trace@0.1 (Normative) [ADD v02.122]
  - Handshake_Master_Spec_v02.122.md 4.5 Layer-wise Inference & Dynamic Compute (Exploratory) [ADD v02.122] (foundation rules; approximate requires waiver; requested vs effective policy logged)

## INTENT (DRAFT)
- What: Ship Phase 1 dynamic-compute foundations: accept `settings.exec_policy` in the runtime contract, govern per-role compute presets (standard / fast_exact / fast_approx) with waiver-required approximation, and emit Flight Recorder telemetry for requested vs effective policy.
- Why: Enable future experimentation without silent quality regressions or audit gaps; keep default posture "standard exact".

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Schema hooks: runtime `settings` accepts `exec_policy`; Work Profiles accept per-role compute presets + approximate control (waiver_ref required when enabled).
  - Deterministic downgrade semantics when requested policy is unsupported or waiver missing (requested vs effective recorded).
  - Flight Recorder event `llm_exec_policy` (FR-EVT-LLM-EXEC-001) emission rules + trace artifact reference handling (`hsk.layerwise_trace@0.1`).
  - Operator UX/config surface for per-role `speed_preset` and waiver-bound "approximate" toggle (minimum viable).
- OUT_OF_SCOPE:
  - Implementing true layer-wise inference algorithms (early exit, block skipping, etc).
  - Multi-device/sharded inference.

## ACCEPTANCE_CRITERIA (DRAFT)
- If `approximate.allowed=false` or waiver is missing/expired, approximate execution cannot occur; system downgrades to exact and logs the downgrade.
- When `settings.exec_policy` is present, Flight Recorder contains `llm_exec_policy` capturing requested vs effective policy (hashes + summary fields).
- If approximate execution occurs, the event includes `waiver_ref` and a trace artifact reference (or explicit null with reason).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: LLM core runtime contract wiring (local runtime) and Work Profiles baseline routing surfaces.
- Likely overlaps with: WP-1-Work-Profiles-v1 (schema + operator UX), coordinate scope to avoid file-lock conflicts.

## RISKS / UNKNOWNs (DRAFT)
- Risk: scope creep into "real" layer-wise inference features in Phase 1 (explicitly out of scope).
- Risk: privacy leakage via high-volume traces; enforce "no raw prompts; no token IDs by default".

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Layerwise-Inference-Foundations-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Layerwise-Inference-Foundations-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


