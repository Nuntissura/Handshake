# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Ollama-Local-Model-MT-Routing-v1

## STUB_METADATA
- WP_ID: WP-1-Ollama-Local-Model-MT-Routing-v1
- BASE_WP_ID: WP-1-Ollama-Local-Model-MT-Routing
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Session-Spawn-Contract, WP-1-ModelSession-Core-Scheduler
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Mixed cloud+local model execution via Ollama
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 4.3.9.12 ModelSession (model_id, backend)
  - Handshake_Master_Spec_v02.179.md LLM provider registry sections
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-109, K2.5 + OpenClaw)

## INTENT (DRAFT)
- What: Route MTs to models based on complexity tier. Simple MTs (struct definitions, import fixes, test scaffolding) route to Ollama-hosted local models (qwen2.5-coder, mistral). Complex MTs (cross-module integration, security-sensitive code) route to cloud models (GPT, Claude). MT task board indicates complexity_tier per MT. Auto-escalate to cloud on local model failure.
- Why: Based on OpenClaw Gateway routing pattern and K2.5 frozen sub-agent architecture. Cloud model costs dominate the budget. Routing simple, well-defined MTs to local models reduces cloud spend while preserving quality for complex work. Auto-escalation ensures no MT is blocked by local model limitations.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - OLLAMA_LOCAL model profile in the model profile catalog.
  - Complexity tier field on MT task board (SIMPLE, MEDIUM, COMPLEX).
  - Routing logic in send-mt and session launcher.
  - Ollama provider adapter in the ACP broker.
  - Auto-escalation from local to cloud on failure.
  - Cost tracking per model tier.
- OUT_OF_SCOPE:
  - Ollama installation and model management.
  - Training or fine-tuning local models.
  - Multi-GPU scheduling.

## ACCEPTANCE_CRITERIA (DRAFT)
- OLLAMA_LOCAL model profile is registered in the model profile catalog.
- MTs with SIMPLE complexity tier route to Ollama-hosted local models by default.
- MTs with COMPLEX complexity tier route to cloud models.
- Auto-escalation triggers when a local model fails an MT, re-routing to cloud automatically.
- Cost tracking records model tier and provider for each MT execution.
- The Ollama provider adapter integrates with the existing ACP broker provider interface.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Session-Spawn-Contract for session lifecycle and MT routing hooks.
- Depends on WP-1-ModelSession-Core-Scheduler for model selection and scheduling infrastructure.
- Requires Ollama to be installed and running locally (out of scope for this WP).

## RISKS / UNKNOWNs (DRAFT)
- Risk: Local model quality for code generation may be insufficient even for SIMPLE MTs, leading to high escalation rates and negating cost savings.
- Risk: Ollama API stability and compatibility may vary across model versions.
- Risk: Complexity tier classification may be inaccurate, routing complex tasks to underpowered local models.
- Unknown: Optimal complexity tier thresholds and which MT characteristics best predict local model success.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Ollama-Local-Model-MT-Routing-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Ollama-Local-Model-MT-Routing-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
