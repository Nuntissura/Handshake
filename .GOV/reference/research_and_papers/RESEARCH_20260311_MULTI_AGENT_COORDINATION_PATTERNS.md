# Research: Multi-Agent Coordination Patterns & Governance

## METADATA
- DATE_UTC: 2026-03-11
- RESEARCHER: EXTERNAL (Claude Opus 4.6, research-only)
- PURPOSE: Survey industry patterns for parallel agentic work, governance, task handoff, and status tracking. Identify what the Handshake governance system can learn from or already aligns with.

---

## 1. OPENCLAW ECOSYSTEM

### 1.1 Core Governance Files
OpenClaw workspaces use three identity/governance files read on every agent wake:
- **SOUL.md** — Defines who the agent *is*: personality, values, behavioral constraints. Functions as an alignment layer. Some implementations add drift checks and heartbeat monitoring.
- **AGENTS.md** — Operational instructions: tools, file conventions, lessons learned. Updated by agents as they discover patterns.
- **USER.md** — User preferences and context.

**Relevance to Handshake:** Your `AGENTS.md` + role protocols (`CODER_PROTOCOL.md`, etc.) cover similar ground. You don't have a SOUL.md equivalent, but your role protocols encode behavioral constraints per-role rather than per-agent.

### 1.2 Lobster Workflow Engine (Deterministic Pipelines)
Lobster is OpenClaw's built-in workflow engine. Key patterns:

- **YAML-defined state machines** control flow, not LLMs. "LLMs do what LLMs are good at: writing code, analyzing code. Lobster does what code is good at: sequencing, counting, routing, retrying."
- **Session targeting** — Agents send messages to specific session keys (e.g., `pipeline:project-a:programmer`) via `sessions_send()`, enabling peer-to-peer communication.
- **Conditional gating** — Steps execute only when conditions evaluate true (e.g., `$code-review-loop.json.approved == true`).
- **Loop limits** — Sub-workflows enforce maximum iterations (e.g., code-review loops max 3).
- **Schema validation** — `llm-task` tool validates LLM outputs against JSON schemas before proceeding.
- **Approval gates** — Steps can pause until explicit approval.

**Relevance to Handshake:** Your system is file-based and relay-friendly rather than session-based. Your `RUNTIME_STATUS.json` serves a similar purpose to Lobster's conditional gating (tracking phase, readiness, next-actor). Consider: could you add loop limits or max-iteration guards to prevent runaway validation cycles?

### 1.3 OpenClaw Mission Control (Enterprise Governance)
Centralized operations platform providing:
- Unified visibility across all agents and sessions
- Approval-driven governance (human-in-the-loop gates)
- Gateway-aware orchestration (Gateway as emergency stop)
- Per-agent tool allow/deny lists (deny wins over allow)
- Prometheus metrics: per-agent runs, success/failure counts, token usage

**Relevance to Handshake:** Your `gov-check.mjs` validation is analogous to their approval gates. You don't have centralized metrics yet, but `RUNTIME_STATUS.json` could evolve toward structured observability if needed.

### 1.4 OpenClaw Skills: Self-Governance
The `agent-self-governance` skill implements:
- **WAL (Write-Ahead Log)** — Log intent before executing
- **VBR (Verify Before Reporting)** — Validate results before marking complete
- **ADL (Action Decision Log)** — Record why decisions were made

The `agent-team-orchestration` skill implements:
- Defined roles with task lifecycles
- Handoff protocols
- Review workflows

**Relevance to Handshake:** Your RECEIPTS.md maps to WAL/ADL concepts. Your packet lifecycle (stub -> official -> validated) maps to VBR. Consider making these mappings explicit so agents can reason about them.

---

## 2. TINYCLAW

### 2.1 Architecture
Lightweight multi-agent collaboration framework by TinyAGI:
- **Pub/sub event bus** for inter-agent communication with wildcard subscriptions and bounded history
- **Persistent team chat rooms** using `[#team_id: message]` tags with broadcast to all teammates
- **SQLite queue** with atomic transactions, retry logic, and dead-letter management
- Parallel message processing with persistent session context

### 2.2 Key Patterns
- Chained execution or fan-out collaboration patterns
- Agents process messages concurrently
- Cross-platform context sharing (Discord, WhatsApp, Telegram)
- Dead-letter queue for failed message delivery

**Relevance to Handshake:** Your THREAD.md is analogous to TinyClaw's team chat rooms. Your file-based approach lacks the pub/sub immediacy but gains durability and relay-friendliness. Dead-letter management is worth considering — what happens when a role fails to process a thread entry?

---

## 3. ANTHROPIC: BUILDING EFFECTIVE AGENTS

### 3.1 Orchestrator-Worker Pattern
From Anthropic's research system and "Building Effective Agents" guide:
- Lead agent analyzes queries, develops strategy, spawns task-specific subagents in parallel
- Each subagent receives: objective, output format, tool/source guidance, clear task boundaries
- Without detailed specs, agents "duplicate work, leave gaps, or fail to find necessary information"
- Synchronous execution simplifies coordination — orchestrator waits for subagents before proceeding

### 3.2 State Persistence
- Lead agent saves plan to Memory to survive context truncation at 200K tokens
- Fresh subagents spawned with clean contexts; continuity maintained through careful handoffs
- **Artifact systems** where agents store outputs that persist independently (not just in conversation history)

### 3.3 Effort Scaling
Embedded in prompts:
- Simple fact-finding: 1 agent, 3-10 tool calls
- Direct comparisons: 2-4 subagents, 10-15 calls each
- Deep research: many subagents, extensive tool use

### 3.4 Safety Patterns
- Deterministic safeguards: retry logic, regular checkpoints
- Rainbow deployments for updates (old + new running simultaneously)
- Human evaluation remains critical for catching hallucinations and subtle biases

**Relevance to Handshake:** Your packet system is essentially an artifact system — work products persist independently of conversation. Your RUNTIME_STATUS.json tracks liveness state similar to Anthropic's checkpointing. The effort-scaling pattern is interesting — your packets could declare expected complexity to guide agent resource allocation.

---

## 4. ACADEMIC RESEARCH

### 4.1 Multi-Agent Collaboration Mechanisms Survey (arXiv 2501.06322, Jan 2025)
Characterizes collaboration across dimensions:
- **Actors**: Which agents are involved
- **Types**: Cooperation, competition, or coopetition
- **Structures**: Peer-to-peer, centralized, or distributed
- **Strategies**: Role-based or model-based
- **Coordination protocols**: How agents synchronize

**Relevance:** Your system is centralized (orchestrator coordinates), role-based (coder/validator/orchestrator), cooperative. This is the most common and well-studied pattern.

### 4.2 MultiAgentBench (ACL 2025)
Benchmark for evaluating multi-agent systems:
- Measures task completion AND quality of collaboration
- Uses milestone-based KPIs (not just final output)
- Tests both collaboration and competition scenarios

**Relevance:** Your validation gates are milestone-based. Consider whether your RECEIPTS could track milestone completion for debugging failed workflows.

### 4.3 Evolving Orchestration (NeurIPS 2025)
"Puppeteer-style" paradigm:
- Centralized orchestrator dynamically directs agents in response to evolving task states
- Orchestrator trained via RL to adaptively sequence and prioritize agents
- Agents are "puppets" — they execute but don't decide sequencing

**Relevance:** Your orchestrator role already follows this pattern. The key insight is that sequencing decisions should be explicit (in packets/status) rather than emergent from agent conversation.

### 4.4 Agent-to-Agent (A2A) Protocol
Emerging standard for agent communication:
- Task receipt acknowledgment
- Status states: submitted -> working -> input-required -> completed/failed/canceled
- Parallel result delivery paths

**Relevance:** Your RUNTIME_STATUS.json `current_phase` field maps to A2A status states. Consider aligning your status vocabulary with A2A for interoperability.

### 4.5 Stigmergy (File-Based Coordination)
Indirect coordination where agents observe effects of each other's actions on shared environment:
- Agent A processes files in input directory, moves to output directory
- Agent B monitors output directory, begins work when new files appear
- No direct communication needed — the file system IS the coordination layer

**Relevance:** Your WP_COMMUNICATIONS system is a form of structured stigmergy. THREAD.md, RUNTIME_STATUS.json, and RECEIPTS.md are the shared environment. This is a well-established pattern in distributed systems.

---

## 5. INDUSTRY FRAMEWORKS

### 5.1 CrewAI (Role-Based)
- Each agent has clearly defined responsibility
- "Crews" = autonomous teams; "Flows" = event-driven pipelines
- Role assignment feels like structured team environment
- Best for: teams with clear role separation

### 5.2 LangGraph (Graph-Based)
- Agent interactions as nodes in directed graph
- Conditional logic, branching workflows, dynamic adaptation
- Durable execution with human-in-the-loop
- Reached v1.0 in late 2025
- Best for: complex stateful workflows

### 5.3 Microsoft Agent Framework (AutoGen + Semantic Kernel)
- Merged AutoGen's multi-agent patterns with Semantic Kernel's enterprise features
- Supports A2A, MCP, and AG-UI protocols out of the box
- Release Candidate Feb 2026
- Best for: enterprise with existing Microsoft stack

### 5.4 GitHub Agentic Workflows (Tech Preview Feb 2026)
- Agents run in GitHub Actions (sandboxed)
- Markdown-defined workflows (not YAML) in `.github/workflows/`
- Git worktrees for parallel agent isolation
- Agents autonomously fix CI failures, address review comments, open PRs
- "Secure output" feature to protect agentic workflows from misuse

**Relevance:** GitHub's use of git worktrees for agent isolation is directly aligned with your worktree-per-WP pattern. Their markdown-defined workflows are conceptually similar to your packet templates.

---

## 6. PATTERNS YOUR SYSTEM ALREADY IMPLEMENTS WELL

| Pattern | Industry Term | Your Implementation |
|---------|---------------|---------------------|
| Task specification as artifact | "Task Packet" / "Artifact System" | TASK_PACKET_TEMPLATE.md |
| Role-based coordination | "Role-based multi-agent" | Coder/Validator/Orchestrator protocols |
| Authority hierarchy | "Governance layer" | "Packet wins" rule |
| Append-only discussion | "Team chat room" / "Stigmergy" | THREAD.md |
| Machine-readable status | "Liveness tracking" / "A2A status" | RUNTIME_STATUS.json |
| Audit trail | "WAL" / "ADL" / "Receipts" | RECEIPTS.md |
| Validation gates | "Approval gates" / "Schema validation" | gov-check.mjs pipeline |
| Worktree isolation | "Sandboxed execution" | Per-WP worktrees |
| Backward compatibility | "Graceful opt-in" | Validator only enforces for packets that declare comm fields |

## 7. GAPS AND OPPORTUNITIES

### 7.1 High Priority (Industry consensus)
1. **Schema validation for status/receipts** — Every mature framework validates structured outputs against schemas (Lobster, A2A, LangGraph). Your RUNTIME_STATUS.json and RECEIPTS.md lack defined schemas.
2. **Explicit status vocabulary** — A2A defines: submitted -> working -> input-required -> completed/failed/canceled. Standardize your status states.
3. **Loop/iteration limits** — Lobster enforces max iterations on review loops. Without this, validation cycles could run indefinitely.

### 7.2 Medium Priority (Common in advanced systems)
4. **Effort scaling hints** — Anthropic embeds complexity estimates in task descriptions. Packets could declare expected scope to help agents budget tool calls.
5. **Write-Ahead Logging** — OpenClaw's WAL pattern (log intent before executing) would make your RECEIPTS more useful for debugging.
6. **Dead-letter handling** — TinyClaw's dead-letter queue for failed deliveries. What happens when a role fails to act on a packet?
7. **Milestone-based tracking** — MultiAgentBench measures milestone completion, not just final output. Your RECEIPTS could track intermediate milestones.

### 7.3 Lower Priority (Emerging patterns)
8. **Metrics/observability** — OpenClaw Mission Control surfaces per-agent token usage, latency, success rates. Useful for optimization.
9. **Emergency stop** — OpenClaw Gateway shutdown kills all agents. Your system lacks an explicit halt mechanism.
10. **Rainbow deployments** — Anthropic's pattern for updating agents mid-workflow without disruption.

---

## 8. SUMMARY

Your WP_COMMUNICATIONS governance addition is well-aligned with industry patterns. The core architecture (role-based, packet-authoritative, file-based coordination, worktree isolation) matches what the most mature systems use. The main gaps are in **structured validation** (schemas, status vocabularies, iteration limits) rather than architectural design. The file-based approach is a deliberate and defensible choice for a relay-friendly system that needs to work with both manual and autonomous operation.

---

*Research complete. This document is informational only — no changes were made to the repository.*
