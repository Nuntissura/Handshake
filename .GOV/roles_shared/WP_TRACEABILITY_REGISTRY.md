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

- **Superseded Packet**: A prior packet revision that is no longer authoritative. Superseded packets are immutable history; do not ???catch them up???.

---

## Workflow (Deterministic)

1. **Roadmap points to Base WP IDs** (not packet revisions).  
2. **Task Board tracks WPs** (Base IDs and/or packet revisions). This registry resolves the Base WP ??? Active Packet mapping when `-vN` revisions exist.
3. **Task packets live in** `.GOV/task_packets/`. **Stubs live in** `.GOV/task_packets/stubs/`.
4. If a packet must change due to audit/spec drift:
   - Create a **new packet revision** `...-v{N}` (do not edit locked history).
   - Mark the older packet as **Superseded** on `.GOV/roles_shared/TASK_BOARD.md`.
   - Update this registry to point the Base WP to the new Active Packet.

**Registry update is mandatory whenever more than one packet exists for the same Base WP.** If mapping is missing or ambiguous, the WP is governance-blocked until resolved.

### How to use with `just` / validation scripts (frictionless rule)

- When running `just pre-work`, `just post-work`, `just gate-check`, validator scripts, etc., use the **Active Packet WP_ID** (the filename stem), not the Base WP ID.
  - Example: if Active Packet is `.GOV/task_packets/WP-1-Workflow-Engine-v4.md`, run `just pre-work WP-1-Workflow-Engine-v4`.
- If the Active Packet is a stub under `.GOV/task_packets/stubs/`, it is **not executable**: activate it first (Technical Refinement Block ??? USER_SIGNATURE ??? create official task packet).

---

## Registry (Phase 1)

Format:
- **Base WP ID**: stable
- **Active Packet**: authoritative file path
- **Task Board**: where to find the status entry
- **Notes**: supersedes history / special cases

| Base WP ID | Active Packet | Task Board | Notes |
|-----------:|---------------|------------|-------|
| WP-1-ACE-Auditability | .GOV/task_packets/stubs/WP-1-ACE-Auditability-v2.md | Stub Backlog (Not Activated): WP-1-ACE-Auditability-v2 | stub (remediation); supersedes: WP-1-ACE-Auditability |
| WP-1-ACE-Runtime | .GOV/task_packets/WP-1-ACE-Runtime-v2.md | Ready for Dev: WP-1-ACE-Runtime-v2 | active=WP-1-ACE-Runtime-v2; activated from stub .GOV/task_packets/stubs/WP-1-ACE-Runtime-v2.md; supersedes: WP-1-ACE-Runtime |
| WP-1-ACE-Validators | .GOV/task_packets/WP-1-ACE-Validators-v4.md | Done: WP-1-ACE-Validators-v4 | active=WP-1-ACE-Validators-v4; supersedes: WP-1-ACE-Validators, WP-1-ACE-Validators-v2, WP-1-ACE-Validators-v3 |
| WP-1-AI-Job-Model | .GOV/task_packets/WP-1-AI-Job-Model-v4.md | Ready for Dev: WP-1-AI-Job-Model-v4 | active=WP-1-AI-Job-Model-v4; activated from stub .GOV/task_packets/stubs/WP-1-AI-Job-Model-v4.md; supersedes: WP-1-AI-Job-Model-v3, WP-1-AI-Job-Model-v2 |
| WP-1-AI-Ready-Data-Architecture | .GOV/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md | Ready for Dev: WP-1-AI-Ready-Data-Architecture-v1 | active=WP-1-AI-Ready-Data-Architecture-v1; activated from stub .GOV/task_packets/stubs/WP-1-AI-Ready-Data-Architecture-v1.md |
| WP-1-Locus-Work-Tracking-System-Phase1 | .GOV/task_packets/stubs/WP-1-Locus-Work-Tracking-System-Phase1-v1.md | Stub Backlog (Not Activated): WP-1-Locus-Work-Tracking-System-Phase1-v1 | stub (spec v02.116); new roadmap item |
| WP-1-AI-UX-Actions | .GOV/task_packets/WP-1-AI-UX-Actions-v2.md | Ready for Dev: WP-1-AI-UX-Actions-v2 | active=WP-1-AI-UX-Actions-v2; activated from stub .GOV/task_packets/stubs/WP-1-AI-UX-Actions-v2.md; supersedes: WP-1-AI-UX-Actions |
| WP-1-AI-UX-Rewrite | .GOV/task_packets/stubs/WP-1-AI-UX-Rewrite-v2.md | Stub Backlog (Not Activated): WP-1-AI-UX-Rewrite-v2 | stub (remediation); supersedes: WP-1-AI-UX-Rewrite |
| WP-1-AI-UX-Summarize-Display | .GOV/task_packets/WP-1-AI-UX-Summarize-Display-v2.md | Ready for Dev: WP-1-AI-UX-Summarize-Display-v2 | active=WP-1-AI-UX-Summarize-Display-v2; activated from stub .GOV/task_packets/stubs/WP-1-AI-UX-Summarize-Display-v2.md; supersedes: WP-1-AI-UX-Summarize-Display |
| WP-1-AppState-Refactoring | .GOV/task_packets/WP-1-AppState-Refactoring-v3.md | Done: WP-1-AppState-Refactoring-v3 | active=WP-1-AppState-Refactoring-v3; supersedes: WP-1-AppState-Refactoring, WP-1-AppState-Refactoring-v2 |
| WP-1-Atelier-Lens | .GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.md | Stub Backlog (Not Activated): WP-1-Atelier-Lens-v2 | stub (remediation); supersedes: WP-1-Atelier-Lens |
| WP-1-Calendar-Lens | .GOV/task_packets/stubs/WP-1-Calendar-Lens-v2.md | Stub Backlog (Not Activated): WP-1-Calendar-Lens-v2 | stub (remediation); supersedes: WP-1-Calendar-Lens |
| WP-1-Canvas-Typography | .GOV/task_packets/WP-1-Canvas-Typography-v2.md | Done: WP-1-Canvas-Typography-v2 | active=WP-1-Canvas-Typography-v2; activated from stub .GOV/task_packets/stubs/WP-1-Canvas-Typography-v2.md; supersedes: WP-1-Canvas-Typography |
| WP-1-Capability-SSoT | .GOV/task_packets/WP-1-Capability-SSoT-v2.md | Ready for Dev: WP-1-Capability-SSoT-v2 | active=WP-1-Capability-SSoT-v2; activated from stub .GOV/task_packets/stubs/WP-1-Capability-SSoT-v2.md; supersedes: WP-1-Capability-SSoT |
| WP-1-Cross-Tool-Interaction-Conformance | .GOV/task_packets/WP-1-Cross-Tool-Interaction-Conformance-v1.md | Ready for Dev: WP-1-Cross-Tool-Interaction-Conformance-v1 | active=WP-1-Cross-Tool-Interaction-Conformance-v1; activated from stub .GOV/task_packets/stubs/WP-1-Cross-Tool-Interaction-Conformance-v1.md |
| WP-1-Debug-Bundle | .GOV/task_packets/WP-1-Debug-Bundle-v3.md | Done: WP-1-Debug-Bundle-v3 | active=WP-1-Debug-Bundle-v3; supersedes: WP-1-Debug-Bundle, WP-1-Debug-Bundle-v2 |
| WP-1-Dev-Experience-ADRs | .GOV/task_packets/WP-1-Dev-Experience-ADRs-v1.md | Ready for Dev: WP-1-Dev-Experience-ADRs-v1 | active=WP-1-Dev-Experience-ADRs-v1; activated from stub .GOV/task_packets/stubs/WP-1-Dev-Experience-ADRs.md |
| WP-1-Distillation | .GOV/task_packets/stubs/WP-1-Distillation-v2.md | Stub Backlog (Not Activated): WP-1-Distillation-v2 | stub (remediation); supersedes: WP-1-Distillation |
| WP-1-Dual-Backend-Tests | .GOV/task_packets/WP-1-Dual-Backend-Tests-v2.md | Done: WP-1-Dual-Backend-Tests-v2 | active=WP-1-Dual-Backend-Tests-v2; supersedes: WP-1-Dual-Backend-Tests |
| WP-1-Editor-Hardening | .GOV/task_packets/WP-1-Editor-Hardening-v2.md | Done: WP-1-Editor-Hardening-v2 | active=WP-1-Editor-Hardening-v2; activated from stub .GOV/task_packets/stubs/WP-1-Editor-Hardening-v2.md; supersedes: WP-1-Editor-Hardening |
| WP-1-Flight-Recorder | .GOV/task_packets/WP-1-Flight-Recorder-v3.md | Done: WP-1-Flight-Recorder-v3 | active=WP-1-Flight-Recorder-v3; supersedes: WP-1-Flight-Recorder, WP-1-Flight-Recorder-v2 |
| WP-1-Flight-Recorder-UI | .GOV/task_packets/WP-1-Flight-Recorder-UI-v3.md | Ready for Dev: WP-1-Flight-Recorder-UI-v3 | activated from stub .GOV/task_packets/stubs/WP-1-Flight-Recorder-UI-v3.md; supersedes: WP-1-Flight-Recorder-UI-v2, WP-1-Flight-Recorder-UI |
| WP-1-Gate-Check-Tool | .GOV/task_packets/WP-1-Gate-Check-Tool-v2.md | Done: WP-1-Gate-Check-Tool-v2 | active=WP-1-Gate-Check-Tool-v2; supersedes: WP-1-Gate-Check-Tool |
| WP-1-Global-Silent-Edit-Guard | .GOV/task_packets/WP-1-Global-Silent-Edit-Guard.md | Done: WP-1-Global-Silent-Edit-Guard | active=WP-1-Global-Silent-Edit-Guard (no -vN revision); activated from stub .GOV/task_packets/stubs/WP-1-Global-Silent-Edit-Guard.md |
| WP-1-Governance-Hooks | .GOV/task_packets/stubs/WP-1-Governance-Hooks-v2.md | Stub Backlog (Not Activated): WP-1-Governance-Hooks-v2 | stub (remediation); supersedes: WP-1-Governance-Hooks |
| WP-1-Governance-Kernel-Conformance | .GOV/task_packets/WP-1-Governance-Kernel-Conformance-v1.md | Done: WP-1-Governance-Kernel-Conformance-v1 | active=WP-1-Governance-Kernel-Conformance-v1; activated from stub .GOV/task_packets/stubs/WP-1-Governance-Kernel-Conformance-v1.md |
| WP-1-Governance-Template-Volume | .GOV/task_packets/WP-1-Governance-Template-Volume-v1.md | Done: WP-1-Governance-Template-Volume-v1 | active=WP-1-Governance-Template-Volume-v1; activated from stub .GOV/task_packets/stubs/WP-1-Governance-Template-Volume-v1.md |
| WP-1-Governance-Workflow-Mirror | .GOV/task_packets/stubs/WP-1-Governance-Workflow-Mirror-v1.md | Stub Backlog (Not Activated): WP-1-Governance-Workflow-Mirror-v1 | stub (new); mirror repo governance workflow in Handshake runtime per v02.113 |
| WP-1-LLM-Core | .GOV/task_packets/WP-1-LLM-Core-v3.md | Done: WP-1-LLM-Core-v3 | active=WP-1-LLM-Core-v3; supersedes: WP-1-LLM-Core |
| WP-1-MCP-End-to-End | .GOV/task_packets/stubs/WP-1-MCP-End-to-End-v2.md | Stub Backlog (Not Activated): WP-1-MCP-End-to-End-v2 | stub (remediation); supersedes: WP-1-MCP-End-to-End |
| WP-1-MCP-Skeleton-Gate | .GOV/task_packets/stubs/WP-1-MCP-Skeleton-Gate-v2.md | Stub Backlog (Not Activated): WP-1-MCP-Skeleton-Gate-v2 | stub (remediation); supersedes: WP-1-MCP-Skeleton-Gate |
| WP-1-Metrics-Mock-Tokens | .GOV/task_packets/WP-1-Metrics-Mock-Tokens.md | Done: WP-1-Metrics-Mock-Tokens | active=WP-1-Metrics-Mock-Tokens (no -vN revision); new packet |
| WP-1-Metrics-OTel | .GOV/task_packets/stubs/WP-1-Metrics-OTel-v2.md | Stub Backlog (Not Activated): WP-1-Metrics-OTel-v2 | stub (remediation); supersedes: WP-1-Metrics-OTel |
| WP-1-Metrics-Traces | .GOV/task_packets/stubs/WP-1-Metrics-Traces-v2.md | Stub Backlog (Not Activated): WP-1-Metrics-Traces-v2 | stub (remediation); supersedes: WP-1-Metrics-Traces |
| WP-1-MEX-Observability | .GOV/task_packets/stubs/WP-1-MEX-Observability-v2.md | Stub Backlog (Not Activated): WP-1-MEX-Observability-v2 | stub (remediation); supersedes: WP-1-MEX-Observability |
| WP-1-MEX-Safety-Gates | .GOV/task_packets/stubs/WP-1-MEX-Safety-Gates-v2.md | Stub Backlog (Not Activated): WP-1-MEX-Safety-Gates-v2 | stub (remediation); supersedes: WP-1-MEX-Safety-Gates |
| WP-1-MEX-UX-Bridges | .GOV/task_packets/stubs/WP-1-MEX-UX-Bridges-v2.md | Stub Backlog (Not Activated): WP-1-MEX-UX-Bridges-v2 | stub (remediation); supersedes: WP-1-MEX-UX-Bridges |
| WP-1-MEX-v1.2-Runtime | .GOV/task_packets/WP-1-MEX-v1.2-Runtime-v3.md | Done: WP-1-MEX-v1.2-Runtime-v3 | active=WP-1-MEX-v1.2-Runtime-v3; supersedes: WP-1-MEX-v1.2-Runtime, WP-1-MEX-v1.2-Runtime-v2 |
| WP-1-Micro-Task-Executor | .GOV/task_packets/WP-1-Micro-Task-Executor-v1.md | Ready for Dev: WP-1-Micro-Task-Executor-v1 | active=WP-1-Micro-Task-Executor-v1; activated from stub .GOV/task_packets/stubs/WP-1-Micro-Task-Executor-v1.md; spec origin v02.114 |
| WP-1-Migration-Framework | .GOV/task_packets/WP-1-Migration-Framework-v2.md | Done: WP-1-Migration-Framework-v2 | active=WP-1-Migration-Framework-v2 (remediation); supersedes: WP-1-Migration-Framework |
| WP-1-Model-Profiles | .GOV/task_packets/stubs/WP-1-Model-Profiles-v2.md | Stub Backlog (Not Activated): WP-1-Model-Profiles-v2 | stub (remediation); supersedes: WP-1-Model-Profiles |
| WP-1-Mutation-Traceability | .GOV/task_packets/WP-1-Mutation-Traceability-v2.md | Ready for Dev: WP-1-Mutation-Traceability-v2 | active=WP-1-Mutation-Traceability-v2; activated from stub .GOV/task_packets/stubs/WP-1-Mutation-Traceability-v2.md; supersedes: WP-1-Mutation-Traceability |
| WP-1-Operator-Consoles | .GOV/task_packets/WP-1-Operator-Consoles-v3.md | Done: WP-1-Operator-Consoles-v3 | active=WP-1-Operator-Consoles-v3; supersedes: WP-1-Operator-Consoles, WP-1-Operator-Consoles-v1, WP-1-Operator-Consoles-v2 |
| WP-1-OSS-Governance | .GOV/task_packets/WP-1-OSS-Governance-v2.md | Ready for Dev: WP-1-OSS-Governance-v2 | active=WP-1-OSS-Governance-v2; activated from stub .GOV/task_packets/stubs/WP-1-OSS-Governance-v2.md; supersedes: WP-1-OSS-Governance |
| WP-1-OSS-Register-Enforcement | .GOV/task_packets/WP-1-OSS-Register-Enforcement-v1.md | Done: WP-1-OSS-Register-Enforcement-v1 | active=WP-1-OSS-Register-Enforcement-v1 |
| WP-1-PDF-Pipeline | .GOV/task_packets/stubs/WP-1-PDF-Pipeline-v2.md | Stub Backlog (Not Activated): WP-1-PDF-Pipeline-v2 | stub (remediation); supersedes: WP-1-PDF-Pipeline |
| WP-1-Photo-Studio | .GOV/task_packets/stubs/WP-1-Photo-Studio-v2.md | Stub Backlog (Not Activated): WP-1-Photo-Studio-v2 | stub (remediation); supersedes: WP-1-Photo-Studio |
| WP-1-RAG-Iterative | .GOV/task_packets/stubs/WP-1-RAG-Iterative-v2.md | Stub Backlog (Not Activated): WP-1-RAG-Iterative-v2 | stub (remediation); supersedes: WP-1-RAG-Iterative |
| WP-1-Role-Mailbox | .GOV/task_packets/WP-1-Role-Mailbox-v1.md | Ready for Dev: WP-1-Role-Mailbox-v1 | active=WP-1-Role-Mailbox-v1; activated from stub .GOV/task_packets/stubs/WP-1-Role-Mailbox-v1.md |
| WP-1-Response-Behavior-ANS-001 | .GOV/task_packets/WP-1-Response-Behavior-ANS-001.md | Ready for Dev: WP-1-Response-Behavior-ANS-001 | active=WP-1-Response-Behavior-ANS-001; activated from stub .GOV/task_packets/stubs/WP-1-Response-Behavior-ANS-001.md |
| WP-1-Security-Gates | .GOV/task_packets/WP-1-Security-Gates-v3.md | Done: WP-1-Security-Gates-v3 | active=WP-1-Security-Gates-v3; supersedes: WP-1-Security-Gates, WP-1-Security-Gates-v2 |
| WP-1-Semantic-Catalog | .GOV/task_packets/stubs/WP-1-Semantic-Catalog-v2.md | Stub Backlog (Not Activated): WP-1-Semantic-Catalog-v2 | stub (remediation); supersedes: WP-1-Semantic-Catalog |
| WP-1-Spec-Enrichment-LLM-Core | .GOV/task_packets/WP-1-Spec-Enrichment-LLM-Core-v1.md | Done: WP-1-Spec-Enrichment-LLM-Core-v1 | active=WP-1-Spec-Enrichment-LLM-Core-v1 |
| WP-1-Spec-Router-Session-Log | .GOV/task_packets/stubs/WP-1-Spec-Router-Session-Log.md | Stub Backlog (Not Activated): WP-1-Spec-Router-Session-Log | stub |
| WP-1-Storage-Abstraction-Layer | .GOV/task_packets/WP-1-Storage-Abstraction-Layer-v3.md | Done: WP-1-Storage-Abstraction-Layer-v3 | active=WP-1-Storage-Abstraction-Layer-v3; supersedes: WP-1-Storage-Abstraction-Layer, WP-1-Storage-Abstraction-Layer-v2 |
| WP-1-Storage-Foundation | .GOV/task_packets/WP-1-Storage-Foundation-v3.md | Done: WP-1-Storage-Foundation-v3 | active=WP-1-Storage-Foundation-v3; supersedes: WP-1-Storage-Foundation-20251228 |
| WP-1-Supply-Chain-MEX | .GOV/task_packets/WP-1-Supply-Chain-MEX-v2.md | Ready for Dev: WP-1-Supply-Chain-MEX-v2 | activated from stub .GOV/task_packets/stubs/WP-1-Supply-Chain-MEX-v2.md; supersedes: WP-1-Supply-Chain-MEX |
| WP-1-Terminal-LAW | .GOV/task_packets/WP-1-Terminal-LAW-v3.md | Done: WP-1-Terminal-LAW-v3 | active=WP-1-Terminal-LAW-v3; supersedes: WP-1-Terminal-LAW, WP-1-Terminal-LAW-v2 |
| WP-1-Tokenization-Service | .GOV/task_packets/WP-1-Tokenization-Service-v3.md | Done: WP-1-Tokenization-Service-v3 | active=WP-1-Tokenization-Service-v3; supersedes: WP-1-Tokenization-Service-20251228 |
| WP-1-Validator-Error-Codes | .GOV/task_packets/WP-1-Validator-Error-Codes-v1.md | Done: WP-1-Validator-Error-Codes-v1 | active=WP-1-Validator-Error-Codes-v1 |
| WP-1-Workflow-Engine | .GOV/task_packets/WP-1-Workflow-Engine-v4.md | Done: WP-1-Workflow-Engine-v4 | active=WP-1-Workflow-Engine-v4; supersedes: WP-1-Workflow-Engine-v2, WP-1-Workflow-Engine-v3 |
| WP-1-Workspace-Bundle | .GOV/task_packets/stubs/WP-1-Workspace-Bundle-v2.md | Stub Backlog (Not Activated): WP-1-Workspace-Bundle-v2 | stub (remediation); supersedes: WP-1-Workspace-Bundle |
| WP-1-Model-Swap-Protocol | .GOV/task_packets/WP-1-Model-Swap-Protocol-v1.md | Ready for Dev: WP-1-Model-Swap-Protocol-v1 | active=WP-1-Model-Swap-Protocol-v1; activated from stub .GOV/task_packets/stubs/WP-1-Model-Swap-Protocol-v1.md; new roadmap item |
| WP-1-Work-Profiles | .GOV/task_packets/stubs/WP-1-Work-Profiles-v1.md | Stub Backlog (Not Activated): WP-1-Work-Profiles-v1 | stub (spec v02.120); new roadmap item |
| WP-1-Cloud-Escalation-Consent | .GOV/task_packets/stubs/WP-1-Cloud-Escalation-Consent-v1.md | Stub Backlog (Not Activated): WP-1-Cloud-Escalation-Consent-v1 | stub (spec v02.120); new roadmap item |
| WP-1-Inbox-Role-Mailbox-Alignment | .GOV/task_packets/stubs/WP-1-Inbox-Role-Mailbox-Alignment-v1.md | Stub Backlog (Not Activated): WP-1-Inbox-Role-Mailbox-Alignment-v1 | stub (spec v02.120); new roadmap item |
| WP-1-Autonomous-Governance-Protocol | .GOV/task_packets/stubs/WP-1-Autonomous-Governance-Protocol-v1.md | Stub Backlog (Not Activated): WP-1-Autonomous-Governance-Protocol-v1 | stub (spec v02.120); new roadmap item |
| WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry | .GOV/task_packets/stubs/WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1.md | Stub Backlog (Not Activated): WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1 | stub (spec v02.122); new Phase 1 roadmap addition |
| WP-1-Layerwise-Inference-Foundations | .GOV/task_packets/stubs/WP-1-Layerwise-Inference-Foundations-v1.md | Stub Backlog (Not Activated): WP-1-Layerwise-Inference-Foundations-v1 | stub (spec v02.122); new Phase 1 roadmap addition |
| WP-1-Atelier-Collaboration-Panel | .GOV/task_packets/WP-1-Atelier-Collaboration-Panel-v1.md | Ready for Dev: WP-1-Atelier-Collaboration-Panel-v1 | activated from stub .GOV/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.md; active=WP-1-Atelier-Collaboration-Panel-v1 |
| WP-1-Lens-Extraction-Tier | .GOV/task_packets/stubs/WP-1-Lens-Extraction-Tier-v1.md | Stub Backlog (Not Activated): WP-1-Lens-Extraction-Tier-v1 | stub (spec v02.123); new Phase 1 roadmap addition |
| WP-1-Lens-ViewMode | .GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.md | Stub Backlog (Not Activated): WP-1-Lens-ViewMode-v1 | stub (spec v02.123); new Phase 1 roadmap addition |
| WP-1-Role-Registry-AppendOnly | .GOV/task_packets/WP-1-Role-Registry-AppendOnly-v1.md | Ready for Dev: WP-1-Role-Registry-AppendOnly-v1 | activated from stub .GOV/task_packets/stubs/WP-1-Role-Registry-AppendOnly-v1.md; active=WP-1-Role-Registry-AppendOnly-v1 |
| WP-1-Role-Turn-Isolation | .GOV/task_packets/stubs/WP-1-Role-Turn-Isolation-v1.md | Stub Backlog (Not Activated): WP-1-Role-Turn-Isolation-v1 | stub (spec v02.123); new Phase 1 roadmap addition |
| WP-1-Artifact-System-Foundations | .GOV/task_packets/WP-1-Artifact-System-Foundations-v1.md | Ready for Dev: WP-1-Artifact-System-Foundations-v1 | active=WP-1-Artifact-System-Foundations-v1; activated from stub .GOV/task_packets/stubs/WP-1-Artifact-System-Foundations-v1.md; Phase 1 roadmap coverage (artifact system) |
| WP-1-Git-Engine-Decision-Gate | .GOV/task_packets/stubs/WP-1-Git-Engine-Decision-Gate-v1.md | Stub Backlog (Not Activated): WP-1-Git-Engine-Decision-Gate-v1 | stub (spec v02.123); Phase 1 roadmap coverage (repo engine decision gate) |
| WP-1-Retrieval-Trace-Bundle-Export | .GOV/task_packets/stubs/WP-1-Retrieval-Trace-Bundle-Export-v1.md | Stub Backlog (Not Activated): WP-1-Retrieval-Trace-Bundle-Export-v1 | stub (spec v02.123); Phase 1 roadmap coverage (retrieval evidence bundle export) |

