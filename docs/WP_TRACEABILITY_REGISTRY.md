# Work Packet Traceability Registry (SSoT)

**Purpose**  
Handshake uses Work Packets (WPs) as execution units, but the Master Spec Main Body must remain stable and WP-free. This registry is the **single source of truth** for mapping:

- **Base WP IDs** (stable planning identifiers used by Roadmap/Task Board), to
- **Active Task Packet files** (the concrete packet to implement/validate), including any `-vN` revisions.

This avoids retroactively embedding WP IDs into the Master Spec and prevents drift when packets are revised (v2/v3/v4) after audits.

---

## Definitions

- **Base WP ID**: The stable identifier for a scope of work, formatted `WP-{phase}-{name}` (e.g., `WP-1-Workflow-Engine`).  
  - Base IDs **do not** include packet revision suffixes.
  - Base IDs are the preferred identifiers for Roadmap pointers and Task Board tracking.

- **Packet Revision**: A revised task packet for the same Base WP, named `WP-{phase}-{name}-v{N}` (e.g., `WP-1-Workflow-Engine-v4`).  
  - Naming rule is governed by Handshake Codex v1.4 `[CX-580C]` (no date/time stamps; use `-vN`).
  - **Legacy exception:** historical packets may contain date stamps (e.g., `-20251228`). Do not create new date-stamped packet IDs; convert future revisions to `-vN`.

- **Active Packet**: The single packet file that is currently authoritative for implementation/validation of a Base WP.

- **Superseded Packet**: A prior packet revision that is no longer authoritative. Superseded packets are immutable history; do not “catch them up”.

---

## Workflow (Deterministic)

1. **Roadmap points to Base WP IDs** (not packet revisions).  
2. **Task Board tracks WPs** (Base IDs and/or packet revisions). This registry resolves the Base WP → Active Packet mapping when `-vN` revisions exist.
3. **Task packets live in** `docs/task_packets/`. **Stubs live in** `docs/task_packets/stubs/`.
4. If a packet must change due to audit/spec drift:
   - Create a **new packet revision** `...-v{N}` (do not edit locked history).
   - Mark the older packet as **Superseded** on `docs/TASK_BOARD.md`.
   - Update this registry to point the Base WP to the new Active Packet.

**Registry update is mandatory whenever more than one packet exists for the same Base WP.** If mapping is missing or ambiguous, the WP is governance-blocked until resolved.

### How to use with `just` / validation scripts (frictionless rule)

- When running `just pre-work`, `just post-work`, `just gate-check`, validator scripts, etc., use the **Active Packet WP_ID** (the filename stem), not the Base WP ID.
  - Example: if Active Packet is `docs/task_packets/WP-1-Workflow-Engine-v4.md`, run `just pre-work WP-1-Workflow-Engine-v4`.
- If the Active Packet is a stub under `docs/task_packets/stubs/`, it is **not executable**: activate it first (Technical Refinement Block → USER_SIGNATURE → create official task packet).

---

## Registry (Phase 1)

Format:
- **Base WP ID**: stable
- **Active Packet**: authoritative file path
- **Task Board**: where to find the status entry
- **Notes**: supersedes history / special cases

| Base WP ID | Active Packet | Task Board | Notes |
|-----------:|---------------|------------|-------|
| WP-1-ACE-Auditability | docs/task_packets/WP-1-ACE-Auditability.md | Ready for Dev: WP-1-ACE-Auditability |  |
| WP-1-ACE-Runtime | docs/task_packets/WP-1-ACE-Runtime.md | Ready for Dev: WP-1-ACE-Runtime |  |
| WP-1-ACE-Validators | docs/task_packets/WP-1-ACE-Validators-v4.md | Done: WP-1-ACE-Validators-v4 | active=WP-1-ACE-Validators-v4; supersedes: WP-1-ACE-Validators, WP-1-ACE-Validators-v2, WP-1-ACE-Validators-v3 |
| WP-1-AI-Job-Model | docs/task_packets/WP-1-AI-Job-Model-v3.md | Ready for Dev: WP-1-AI-Job-Model-v3 | active=WP-1-AI-Job-Model-v3; supersedes: WP-1-AI-Job-Model-v2 |
| WP-1-AI-UX-Actions | docs/task_packets/WP-1-AI-UX-Actions.md | Ready for Dev: WP-1-AI-UX-Actions |  |
| WP-1-AI-UX-Rewrite | docs/task_packets/WP-1-AI-UX-Rewrite.md | Ready for Dev: WP-1-AI-UX-Rewrite |  |
| WP-1-AI-UX-Summarize-Display | docs/task_packets/WP-1-AI-UX-Summarize-Display.md | Ready for Dev: WP-1-AI-UX-Summarize-Display |  |
| WP-1-AppState-Refactoring | docs/task_packets/WP-1-AppState-Refactoring-v2.md | Ready for Dev: WP-1-AppState-Refactoring-v2 | active=WP-1-AppState-Refactoring-v2 |
| WP-1-Atelier-Lens | docs/task_packets/WP-1-Atelier-Lens.md | Ready for Dev: WP-1-Atelier-Lens |  |
| WP-1-Calendar-Lens | docs/task_packets/WP-1-Calendar-Lens.md | Ready for Dev: WP-1-Calendar-Lens |  |
| WP-1-Canvas-Typography | docs/task_packets/WP-1-Canvas-Typography.md | Ready for Dev: WP-1-Canvas-Typography |  |
| WP-1-Capability-SSoT | docs/task_packets/WP-1-Capability-SSoT.md | Ready for Dev: WP-1-Capability-SSoT |  |
| WP-1-Debug-Bundle | docs/task_packets/WP-1-Debug-Bundle-v3.md | Done: WP-1-Debug-Bundle-v3 | active=WP-1-Debug-Bundle-v3; supersedes: WP-1-Debug-Bundle, WP-1-Debug-Bundle-v2 |
| WP-1-Dev-Experience-ADRs | docs/task_packets/stubs/WP-1-Dev-Experience-ADRs.md | Stub Backlog (Not Activated): WP-1-Dev-Experience-ADRs | stub |
| WP-1-Distillation | docs/task_packets/WP-1-Distillation.md | Ready for Dev: WP-1-Distillation |  |
| WP-1-Dual-Backend-Tests | docs/task_packets/WP-1-Dual-Backend-Tests-v2.md | Done: WP-1-Dual-Backend-Tests-v2 | active=WP-1-Dual-Backend-Tests-v2; supersedes: WP-1-Dual-Backend-Tests |
| WP-1-Editor-Hardening | docs/task_packets/WP-1-Editor-Hardening.md | Ready for Dev: WP-1-Editor-Hardening |  |
| WP-1-Flight-Recorder | docs/task_packets/WP-1-Flight-Recorder-v3.md | Done: WP-1-Flight-Recorder-v3 | active=WP-1-Flight-Recorder-v3; supersedes: WP-1-Flight-Recorder, WP-1-Flight-Recorder-v2 |
| WP-1-Flight-Recorder-UI | docs/task_packets/WP-1-Flight-Recorder-UI-v2.md | Ready for Dev: WP-1-Flight-Recorder-UI-v2 | active=WP-1-Flight-Recorder-UI-v2; supersedes: WP-1-Flight-Recorder-UI |
| WP-1-Gate-Check-Tool | docs/task_packets/WP-1-Gate-Check-Tool-v2.md | Done: WP-1-Gate-Check-Tool-v2 | active=WP-1-Gate-Check-Tool-v2; supersedes: WP-1-Gate-Check-Tool |
| WP-1-Global-Silent-Edit-Guard | docs/task_packets/stubs/WP-1-Global-Silent-Edit-Guard.md | Stub Backlog (Not Activated): WP-1-Global-Silent-Edit-Guard | stub |
| WP-1-Governance-Hooks | docs/task_packets/WP-1-Governance-Hooks.md | Ready for Dev: WP-1-Governance-Hooks |  |
| WP-1-LLM-Core | docs/task_packets/WP-1-LLM-Core-v3.md | Done: WP-1-LLM-Core-v3 | active=WP-1-LLM-Core-v3; supersedes: WP-1-LLM-Core |
| WP-1-MCP-End-to-End | docs/task_packets/WP-1-MCP-End-to-End.md | Ready for Dev: WP-1-MCP-End-to-End |  |
| WP-1-MCP-Skeleton-Gate | docs/task_packets/WP-1-MCP-Skeleton-Gate.md | Ready for Dev: WP-1-MCP-Skeleton-Gate |  |
| WP-1-Metrics-OTel | docs/task_packets/WP-1-Metrics-OTel.md | Ready for Dev: WP-1-Metrics-OTel |  |
| WP-1-Metrics-Traces | docs/task_packets/WP-1-Metrics-Traces.md | Ready for Dev: WP-1-Metrics-Traces |  |
| WP-1-MEX-Observability | docs/task_packets/WP-1-MEX-Observability.md | Ready for Dev: WP-1-MEX-Observability |  |
| WP-1-MEX-Safety-Gates | docs/task_packets/WP-1-MEX-Safety-Gates.md | Ready for Dev: WP-1-MEX-Safety-Gates |  |
| WP-1-MEX-UX-Bridges | docs/task_packets/WP-1-MEX-UX-Bridges.md | Ready for Dev: WP-1-MEX-UX-Bridges |  |
| WP-1-MEX-v1.2-Runtime | docs/task_packets/WP-1-MEX-v1.2-Runtime-v3.md | Done: WP-1-MEX-v1.2-Runtime-v3 | active=WP-1-MEX-v1.2-Runtime-v3; supersedes: WP-1-MEX-v1.2-Runtime, WP-1-MEX-v1.2-Runtime-v2 |
| WP-1-Migration-Framework | docs/task_packets/WP-1-Migration-Framework.md | Ready for Dev: WP-1-Migration-Framework |  |
| WP-1-Model-Profiles | docs/task_packets/WP-1-Model-Profiles.md | Ready for Dev: WP-1-Model-Profiles |  |
| WP-1-Mutation-Traceability | docs/task_packets/WP-1-Mutation-Traceability.md | Ready for Dev: WP-1-Mutation-Traceability |  |
| WP-1-Operator-Consoles | docs/task_packets/WP-1-Operator-Consoles-v3.md | Done: WP-1-Operator-Consoles-v3 | active=WP-1-Operator-Consoles-v3; supersedes: WP-1-Operator-Consoles, WP-1-Operator-Consoles-v1, WP-1-Operator-Consoles-v2 |
| WP-1-OSS-Governance | docs/task_packets/WP-1-OSS-Governance.md | Ready for Dev: WP-1-OSS-Governance |  |
| WP-1-OSS-Register-Enforcement | docs/task_packets/WP-1-OSS-Register-Enforcement-v1.md | Done: WP-1-OSS-Register-Enforcement-v1 | active=WP-1-OSS-Register-Enforcement-v1 |
| WP-1-PDF-Pipeline | docs/task_packets/WP-1-PDF-Pipeline.md | Ready for Dev: WP-1-PDF-Pipeline |  |
| WP-1-Photo-Studio | docs/task_packets/WP-1-Photo-Studio.md | Ready for Dev: WP-1-Photo-Studio |  |
| WP-1-RAG-Iterative | docs/task_packets/WP-1-RAG-Iterative.md | Ready for Dev: WP-1-RAG-Iterative |  |
| WP-1-Response-Behavior-ANS-001 | docs/task_packets/stubs/WP-1-Response-Behavior-ANS-001.md | Stub Backlog (Not Activated): WP-1-Response-Behavior-ANS-001 | stub |
| WP-1-Security-Gates | docs/task_packets/WP-1-Security-Gates-v3.md | Done: WP-1-Security-Gates-v3 | active=WP-1-Security-Gates-v3; supersedes: WP-1-Security-Gates, WP-1-Security-Gates-v2 |
| WP-1-Semantic-Catalog | docs/task_packets/WP-1-Semantic-Catalog.md | Ready for Dev: WP-1-Semantic-Catalog |  |
| WP-1-Spec-Router-Session-Log | docs/task_packets/stubs/WP-1-Spec-Router-Session-Log.md | Stub Backlog (Not Activated): WP-1-Spec-Router-Session-Log | stub |
| WP-1-Storage-Abstraction-Layer | docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md | Done: WP-1-Storage-Abstraction-Layer-v3 | active=WP-1-Storage-Abstraction-Layer-v3; supersedes: WP-1-Storage-Abstraction-Layer, WP-1-Storage-Abstraction-Layer-v2 |
| WP-1-Storage-Foundation | docs/task_packets/WP-1-Storage-Foundation-v3.md | Done: WP-1-Storage-Foundation-v3 | active=WP-1-Storage-Foundation-v3; supersedes: WP-1-Storage-Foundation-20251228 |
| WP-1-Supply-Chain-MEX | docs/task_packets/WP-1-Supply-Chain-MEX.md | Ready for Dev: WP-1-Supply-Chain-MEX |  |
| WP-1-Terminal-LAW | docs/task_packets/WP-1-Terminal-LAW-v3.md | Done: WP-1-Terminal-LAW-v3 | active=WP-1-Terminal-LAW-v3; supersedes: WP-1-Terminal-LAW, WP-1-Terminal-LAW-v2 |
| WP-1-Tokenization-Service | docs/task_packets/WP-1-Tokenization-Service-v3.md | Done: WP-1-Tokenization-Service-v3 | active=WP-1-Tokenization-Service-v3; supersedes: WP-1-Tokenization-Service-20251228 |
| WP-1-Validator-Error-Codes | docs/task_packets/WP-1-Validator-Error-Codes-v1.md | Done: WP-1-Validator-Error-Codes-v1 | active=WP-1-Validator-Error-Codes-v1 |
| WP-1-Workflow-Engine | docs/task_packets/WP-1-Workflow-Engine-v4.md | Done: WP-1-Workflow-Engine-v4 | active=WP-1-Workflow-Engine-v4; supersedes: WP-1-Workflow-Engine-v2, WP-1-Workflow-Engine-v3 |
| WP-1-Workspace-Bundle | docs/task_packets/WP-1-Workspace-Bundle.md | Ready for Dev: WP-1-Workspace-Bundle |  |
