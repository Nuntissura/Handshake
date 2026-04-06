# Agent Swarm and Parallel Orchestration Landscape Research

- RESEARCH_ID: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION
- DATE: 2026-04-06
- AUTHOR: Orchestrator (Claude Opus 4.6)
- SCOPE: Agent swarm frameworks, parallel agentic coding, mechanical communication patterns, and their applicability to Handshake repo governance and product governance
- RELATED_GOVERNANCE: RGF-88 through RGF-97, CX-503A through CX-503H

---

## 1. Executive Summary

This research surveys the 2025-2026 agent swarm landscape across production frameworks, academic research, and open-source implementations. The goal is to identify patterns, architectures, and techniques that can improve both Handshake's repo governance (the current orchestrator-managed ACP workflow) and the Handshake product's governance engine (the future LLM swarm harness for cloud + local model coordination).

Key finding: the industry has converged on a shared set of architectural principles that Handshake's repo governance independently discovered through iteration. The gap is not in the patterns — it is in the infrastructure that makes those patterns mechanical instead of prompt-dependent.

---

## 2. Frameworks Surveyed

### 2.1 Production Frameworks

#### Claude Code Agent Teams (Anthropic, 2026)
- SOURCE: https://code.claude.com/docs/en/agent-teams
- ARCHITECTURE: Lead session + teammate sessions, each in its own context window. Shared task list with self-coordination. Direct teammate-to-teammate messaging via mailbox.
- KEY INNOVATION: Teammates self-claim tasks from a shared list. No orchestrator assignment needed. Task dependencies auto-resolve when upstream tasks complete. Hooks (`TeammateIdle`, `TaskCreated`, `TaskCompleted`) provide mechanical quality gates.
- COMMUNICATION: Direct messaging between agents via a mailbox system. Broadcast to all teammates supported. No central relay bottleneck.
- VALIDATION: Hook-based quality gates. Exit code 2 from a hook sends feedback and blocks the action.
- LIMITATIONS: Experimental. No session resumption with in-process teammates. Task status can lag. One team per session. No nested teams.

#### Overstory (jayminwest, 2026)
- SOURCE: https://github.com/jayminwest/overstory
- ARCHITECTURE: Multi-runtime adapters (Claude Code, Pi, Codex). Each agent runs in an isolated git worktree via tmux. Instruction overlays (dynamic CLAUDE.md per task) define scope; tool-call guards enforce access control.
- KEY INNOVATION: SQLite mail system in WAL mode with ~1-5ms latency. 8 typed message protocols (worker_done, merge_ready, dispatch, escalation, etc.). FIFO merge queue with 4-tier conflict resolution.
- COMMUNICATION: SQLite-backed, not file-based. Broadcast messaging to groups (@all, @builders). Typed message protocols prevent ambiguous communication.
- VALIDATION: 3-tier watchdog system. Tier 0: mechanical daemon (tmux/PID liveness). Tier 1: AI-assisted failure triage (transcript analysis). Tier 2: continuous fleet patrol agent.
- RELEVANCE TO HANDSHAKE: Overstory's SQLite mail system solves the JSONL file-lock contention problem. Its tool-call guards solve the "model doesn't follow instructions" problem mechanically. Its multi-runtime adapter pattern is directly applicable to Handshake's GPT + Claude + Codex Spark + Ollama requirement.

#### Metaswarm (dsifry, 2026)
- SOURCE: https://github.com/dsifry/metaswarm
- ARCHITECTURE: 18 specialized personas, 13 orchestration skills. Recursive orchestration with Swarm Coordinators spawning Issue Orchestrators.
- KEY INNOVATION: BEADS (git-native issue tracking) as coordination backbone instead of separate communication layer. 4-phase execution loop: IMPLEMENT -> VALIDATE -> ADVERSARIAL REVIEW -> COMMIT. "Never trust subagent self-reports."
- COMMUNICATION: Git-native via BEADS. No separate messaging system — coordination happens through git issues and commit metadata.
- VALIDATION: TDD mandatory with coverage thresholds as blocking gates. Adversarial reviewers check spec compliance with file:line evidence. Human escalation after 3 failed iterations.
- RELEVANCE TO HANDSHAKE: Metaswarm's adversarial review pattern validates our coder/validator split but goes further — the reviewer actively CHALLENGES the implementer. The 4-phase loop matches our MT-commit -> validator-review -> fix -> next-MT pattern exactly.

#### Agent Swarm / desplega-ai (2026)
- SOURCE: https://github.com/desplega-ai/agent-swarm
- ARCHITECTURE: Lead agent + worker agents in Docker containers. MCP API server as central coordination hub with SQLite persistence.
- KEY INNOVATION: Workers in Docker isolation with full dev environments. 6 execution hooks (PreToolUse, PostToolUse, Stop). Agents learn from failures via searchable memory indexed by embeddings.
- COMMUNICATION: MCP API server with SQLite. Workers expose HTTP services for inter-worker communication. Real-time dashboard for monitoring.
- VALIDATION: Hook-based lifecycle with tool loop detection. Session summarization on stop.
- RELEVANCE TO HANDSHAKE: The failure memory pattern (storing fix patterns indexed by embeddings for future retrieval) is directly applicable. If a coder hits the same compile error pattern that a previous coder fixed, it should find that fix automatically.

### 2.2 Research Papers

#### Multi-Agent LLM Orchestration for Deterministic Decision Support (2025)
- SOURCE: https://arxiv.org/abs/2511.15755
- KEY FINDING: Multi-agent orchestration achieves 100% actionable recommendation rates compared to 1.7% for single-agent, with ZERO quality variance across all trials. This enables production SLA commitments impossible with inconsistent single-agent outputs.
- RELEVANCE: Proves that the orchestration layer itself is what creates determinism, not the individual models. Our gov-check gates, validator rubrics, and mechanical hooks are the orchestration layer.

#### Multi-Agent System Orchestration: Architectures, Protocols, and Enterprise Adoption (2026)
- SOURCE: https://arxiv.org/html/2601.13671v1
- KEY FINDINGS:
  - Separate tool access (MCP) from peer collaboration (A2A) protocols
  - Layered orchestration: Planning unit -> Control unit -> Policy unit -> State management
  - "No implicit handoffs — every transition is logged, validated, and supervised"
  - Reliability comes from the orchestration layer, not from intelligent agents
  - Fragility comes from: treating orchestration as optional, assuming self-coordination, decentralizing validation, ignoring coordination cost
- RELEVANCE: This is the theoretical foundation for what we've been building empirically. Our packet truth, session registry, receipt/notification system, and clause closure matrices are implementations of this paper's recommendations.

#### ParaCodex: Profiling-Guided Autonomous Parallel Code Generation (2026)
- SOURCE: https://arxiv.org/abs/2601.04327
- KEY INNOVATION: "Correctness gating" — validate at every stage on actual hardware, not just syntax. Profiling-guided refinement means the agent measures actual performance, identifies bottlenecks, and optimizes iteratively.
- RELEVANCE: Our per-MT validation loop is a form of correctness gating. But we only validate correctness (compile + test), not performance. For Handshake product work, profiling-guided optimization could improve quality.

#### PARL: Parallel-Agent Reinforcement Learning (Swarm Corporation, 2026)
- SOURCE: https://github.com/The-Swarm-Corporation/PARL
- KEY INNOVATION: The orchestrator model is TRAINED to decompose tasks, not just prompted. Up to 100 sub-agents across 1,500+ coordinated steps. Sub-agents are frozen (no training); only the orchestrator learns.
- RELEVANCE: Currently we prompt the orchestrator to decompose WPs into MTs. If the orchestrator model were fine-tuned on successful decomposition patterns, the quality of MT plans would improve. This is relevant to Handshake's distillation pipeline (WP-1-Session-Spawn-Conversation-Distillation-v1).

### 2.3 Industry Trends (2026)

- Anthropic's 2026 Agentic Coding Trends Report: 95% of professional developers use AI coding tools weekly; 75% rely on AI for at least half their engineering work.
- Every major AI coding platform shipped multi-agent capabilities in February 2026 within the same two-week window.
- Kimi K2.5: visual debugging loop (generate -> render -> compare against design -> fix -> repeat until quality threshold).
- Gartner: 1,445% surge in multi-agent system inquiries from Q1 2024 to Q2 2025.

---

## 3. Architectural Pattern Analysis

### 3.1 Communication Patterns

| Pattern | Used By | Mechanism | Latency | Reliability |
|---|---|---|---|---|
| Direct messaging (mailbox) | Claude Agent Teams | In-memory per session | <1ms | HIGH (in-process) |
| SQLite mail | Overstory | WAL-mode SQLite with typed protocols | ~1-5ms | HIGH (ACID, no lock contention) |
| Git-native (BEADS/commits) | Metaswarm, Handshake | Git issues or commit hooks | ~100ms | MEDIUM (filesystem-dependent) |
| JSONL files + file locks | Handshake (current) | Append-only JSONL with advisory locks | ~10-50ms | LOW (lock contention, no atomic queries) |
| MCP API server | Agent Swarm (desplega-ai) | HTTP + SQLite persistence | ~5-10ms | MEDIUM (network hop) |
| A2A Protocol | Research paper | Structured metadata payloads | Variable | MEDIUM (depends on transport) |

**Assessment for Handshake:**
- Current JSONL approach works but has the worst reliability and query characteristics.
- SQLite (Overstory pattern) is the clear upgrade path: same filesystem locality, ACID guarantees, typed queries, no lock contention.
- For the Handshake PRODUCT, the MCP API server pattern is more appropriate (multiple machines, network communication).
- For repo governance (single machine), SQLite is optimal.

### 3.2 Coordination Patterns

| Pattern | Used By | Orchestrator Role | MT/Task Granularity |
|---|---|---|---|
| Orchestrator assigns, workers execute | Handshake (current), Agent Swarm | Central relay + monitor | Orchestrator decomposes + assigns |
| Shared task list with self-claim | Claude Agent Teams | Creates tasks, monitors | Agents claim from shared list |
| Recursive decomposition | Metaswarm | Top-level only; sub-orchestrators handle details | Hierarchical decomposition |
| Fully decentralized (pheromone) | Academic swarm research | None | Emergent from local rules |

**Assessment for Handshake:**
- Current "orchestrator assigns" pattern is the bottleneck. Every MT assignment requires an orchestrator turn.
- Shared task list with self-claim (Claude Agent Teams pattern) removes the orchestrator from the assignment loop.
- For Handshake product, recursive decomposition (Metaswarm) scales better for complex multi-WP work.

### 3.3 Validation Patterns

| Pattern | Used By | When | Mechanical? |
|---|---|---|---|
| Hook-based gates (exit code 2) | Claude Agent Teams, Agent Swarm | On task create/complete/idle | YES |
| Post-commit hooks | Handshake (new), Metaswarm | On git commit | YES |
| 4-phase loop (implement -> validate -> adversarial review -> commit) | Metaswarm | Every work unit | PARTIALLY (review is LLM-dependent) |
| Per-MT validator review | Handshake (current) | After each MT commit | PARTIALLY (auto-relay depends on model calling wp-review-response) |
| Correctness gating (compile + test at each stage) | ParaCodex | After each code generation step | YES |
| Visual debugging loop (generate -> render -> compare -> fix) | Kimi K2.5 | Each visual output iteration | YES |
| 3-tier watchdog (PID -> transcript -> fleet) | Overstory | Continuous | YES for tier 0, PARTIAL for 1-2 |

**Assessment for Handshake:**
- Our git post-commit hook is the right mechanical trigger. Most frameworks converge on hook-based gates.
- Missing: per-MT compile gate (block next MT if current doesn't compile). This would catch 80% of coder errors mechanically.
- Missing: adversarial review (dedicated challenger who tries to break the code, not just confirm it works).
- Missing: visual validation (relevant for GUI-bearing WPs; the screenshot tool stub WP-1-Product-Screenshot-Visual-Validation-v1 addresses this).

### 3.4 Failure Recovery Patterns

| Pattern | Used By | Mechanism |
|---|---|---|
| Agent restarts on crash | All frameworks | New session from checkpoint |
| Searchable failure memory | Agent Swarm (desplega-ai) | Embedding-indexed fix patterns |
| 3-retry then human escalation | Metaswarm | Counter + escalation flag |
| Healing agents | Research paper | Dedicated agent resets workflow state |
| Self-settle detection | Handshake (current) | Broker detects orphaned run |
| Stall timeout + escalation | Handshake (current) | heartbeat/stale_after timers |

**Assessment for Handshake:**
- Stall detection exists but is passive (timeout-based). Active detection (transcript analysis for stuck patterns) would be faster.
- Failure memory is the biggest missing piece. Coder sessions start fresh with no knowledge of past failures. If a previous coder session fixed a CRLF issue in `storage/tests.rs`, future sessions should know about it.
- 3-retry escalation is a good heuristic. We don't track retry counts per MT.

---

## 4. Gap Analysis: Handshake Repo Governance vs State of the Art

### 4.1 What We Have That Others Don't

| Capability | Status | Why It Matters |
|---|---|---|
| Exhaustive spec-anchored refinement (HYDRATED_RESEARCH_V1) | IMPLEMENTED | No other framework forces cross-pillar, cross-engine, primitive, and UI discovery before coding starts |
| Feature discovery checkpoint (RGF-94) | IMPLEMENTED | Forces refinement to produce new stubs, primitives, and matrix edges — exponential feature growth |
| Clause closure matrix | IMPLEMENTED | Per-clause proof tracking through implementation and validation |
| Multi-provider governed runtime | IMPLEMENTED | GPT, Claude, Codex Spark in one governed pipeline with packet-declared profiles |
| Smoketest live document model | IMPLEMENTED | Roles append findings during execution, not post-hoc narration |
| Deterministic computed policy gate | IMPLEMENTED | 50+ mechanical checks that every WP must pass at every lifecycle stage |

### 4.2 What Others Have That We Need

| Capability | Who Has It | Priority | Effort |
|---|---|---|---|
| SQLite communication backbone | Overstory | HIGH | MEDIUM — replace JSONL notification/receipt files with SQLite |
| Shared task list with self-claim | Claude Agent Teams | HIGH | MEDIUM — create MT task board, agents claim from it |
| Per-MT completion hooks (compile gate) | Claude Agent Teams, ParaCodex | HIGH | LOW — git post-commit hook runs cargo check before review request |
| Adversarial review | Metaswarm | MEDIUM | LOW — prompt change; validator actively challenges, not just confirms |
| Failure memory (embedding-indexed) | Agent Swarm (desplega-ai) | MEDIUM | MEDIUM — store fix patterns from smoketest reviews, index by error type |
| Multi-tier watchdog | Overstory | MEDIUM | MEDIUM — add transcript analysis for stuck patterns |
| Tool-call guards (read-only enforcement) | Overstory | LOW | LOW — we have role boundary enforcement but it's prompt-based, not mechanical |
| Visual debugging loop | Kimi K2.5 | LOW | MEDIUM — WP-1-Product-Screenshot-Visual-Validation-v1 addresses this |
| Trained orchestrator (not just prompted) | PARL | LOW | HIGH — requires fine-tuning infrastructure |

### 4.3 What Nobody Has (Handshake Product Opportunity)

| Capability | Why It Matters | Handshake Advantage |
|---|---|---|
| Governed mixed cloud+local model swarm | Local models for cheap coding, cloud models for reasoning/validation | Handshake is designed for this (Ollama + cloud providers) |
| Cross-domain governance (not just software delivery) | Software, research, creative, worldbuilding — same governance kernel | Project profile extensions already support multi-domain |
| LLM-friendly artifact storage | All artifacts structured for model consumption, not human presentation | LLM-first data pillar already mandated in spec |
| Distillation from swarm execution | Spawn tree conversations become training data for model specialization | WP-1-Session-Spawn-Conversation-Distillation-v1 stub exists |
| In-product governance visualization | DCC shows governance state, not just code diffs | DCC + session spawn tree stubs exist |

---

## 5. Recommendations for Repo Governance (Next Steps)

### 5.1 Immediate (next 1-2 WPs)

1. **Per-MT compile gate in post-commit hook** — before firing wp-review-request, run `cargo check` in the hook. If it fails, log the error and don't fire the review request. The coder sees the compile error in the hook output and fixes it before the validator is even notified. This is ParaCodex's "correctness gating" at the git-commit level.

2. **Adversarial review prompt** — update the validator's startup prompt to include: "After confirming the code compiles and tests pass, actively try to break it. Look for edge cases, race conditions, missing error handling, and spec requirements the coder missed. Your job is not to confirm the code works — it's to find where it doesn't."

3. **MT retry counter** — track how many times each MT has been through the fix cycle. After 3 fix cycles on the same MT, escalate to orchestrator with a summary of what keeps failing. This prevents infinite fix loops.

### 5.2 Short-term (next month)

4. **SQLite communication backbone** — replace JSONL notification/receipt files with a single SQLite database per WP. This eliminates file-lock contention, enables typed queries, and supports broadcast. The schema would be:
   - `messages` table: sender_role, target_role, message_type, content, timestamp, acknowledged
   - `tasks` table: mt_id, status (pending/claimed/completed/failed), claimed_by, evidence
   - `receipts` table: receipt_kind, actor_role, summary, timestamp

5. **Shared MT task board with self-claim** — instead of the orchestrator sending each MT prompt, populate a task board at packet creation time. The coder session reads the board, claims the next unclaimed MT, implements it, marks it complete. The validator automatically reviews completed MTs. The orchestrator only intervenes on failure.

6. **Failure memory** — after each smoketest review, extract error patterns and fix patterns into a searchable index:
   - Error: "DateTime import missing in sqlite.rs" → Fix: "add `use chrono::{DateTime, Utc};`"
   - Error: "parseSectionField doesn't match `- ` bullet prefix" → Fix: "use `(?:-\s*)?` regex prefix"
   Future coder sessions query this index at startup.

### 5.3 Medium-term (next quarter)

7. **Transcript-based stall detection** — instead of only checking receipt timestamps, periodically scan the coder's session output JSONL for stuck patterns:
   - Same error message repeated 3+ times → stall
   - No file writes in last 5 minutes → stall
   - Model saying "I'll try again" 3+ times → stall

8. **Per-MT compile hook** — extend the post-commit hook to run `cargo check` before firing wp-review-request. If compile fails, the hook logs the error to WP_COMMUNICATIONS and does NOT fire the review request. The coder sees the error in the git output.

9. **Tool-call guards** — for validator sessions, mechanically enforce read-only file access. The validator can read any file but cannot write to files under `src/`, `app/`, or `tests/`. This prevents the validator from "helping" by editing code (a role boundary violation).

### 5.4 Long-term (Handshake product)

10. **In-product session manager** — replace the ACP broker + system terminals with an in-app session panel. Sessions display live model interactions, command outputs, and governance state. Operators inspect work without opening OS terminals.

11. **Distillation pipeline from governed work** — successful coder-validator review cycles become training pairs. The coder's implementation + validator's feedback + final fix = a supervised learning example for model specialization.

12. **Local model integration** — Ollama-hosted models handle simple MTs (struct definitions, import fixes, test scaffolding). Cloud models handle complex MTs (architecture decisions, cross-module integration, security review). The MT task board indicates complexity tier per MT.

13. **Cross-project governance portability** — the governance kernel's patterns (task board, MT loop, auto-relay, validation gates) should work for non-software projects (research, design, worldbuilding) via project profile extensions.

---

## 6. What We Should Explore Further

### 6.1 For Repo Governance

| Topic | Why | How to Research |
|---|---|---|
| Event-driven architecture for agent communication | JSONL polling is our biggest reliability gap | Study Overstory's SQLite WAL implementation; prototype a SQLite-backed notification system |
| Trained orchestrator models (PARL approach) | Prompted orchestrators are expensive and inconsistent | Study PARL training methodology; evaluate whether spawn tree conversations can train a task decomposition model |
| Adversarial validation techniques | Our validators confirm; they should challenge | Study Metaswarm's adversarial review patterns; study red team automation from security research |
| Mechanical tool-call enforcement | We rely on prompts for role boundaries | Study Overstory's hook-based tool guards; study Claude Code's permission hooks |
| Cost-optimal model routing per task | We use the same model for all MTs regardless of complexity | Study K2.5's frozen sub-agent model; evaluate per-MT complexity scoring to route simple MTs to cheaper models |

### 6.2 For Handshake Product Governance

| Topic | Why | How to Research |
|---|---|---|
| MCP + A2A protocol dual-layer communication | Handshake needs both tool access (MCP) and peer coordination (A2A) | Study the orchestration protocols paper (arxiv 2601.13671); design Handshake's native communication protocol |
| Visual debugging loop for GUI-bearing WPs | Current validation is code-diff only; GUI work needs visual comparison | Study Kimi K2.5's render-compare-fix loop; design the screenshot tool WP |
| Multi-domain governance portability | Software delivery is one profile; Handshake supports research, creative, worldbuilding | Study how existing governance patterns (MT loop, validation gates) translate to non-code domains |
| Checkpoint-based session recovery at product scale | Current checkpointing is per-session; product needs cross-session recovery | Study ParaCodex's correctness gating; evaluate checkpoint granularity (per-tool-call vs per-MT vs per-session) |
| Decentralized validation for local model swarms | Cloud validators are expensive; local models should validate local model work | Study academic swarm consensus mechanisms; evaluate lightweight validation probes that local models can run |
| Governance artifact versioning and migration | As governance evolves, imported governance artifacts in the product need migration paths | Study the governance pack export/import pattern; design version-aware artifact migration |

### 6.3 Open Questions

1. **Can the orchestrator role be eliminated entirely?** Claude Agent Teams' self-claim pattern suggests yes for simple swarms. But for governed work with spec alignment and clause proof, someone needs to enforce governance. Could the governance enforcement be purely mechanical (hooks + gates) with no LLM orchestrator?

2. **Is adversarial review worth the token cost?** Metaswarm's "never trust self-reports" is sound, but a dedicated adversarial reviewer doubles the validation cost. Is the quality improvement measurable? Could the adversarial review be done by a cheaper model?

3. **What's the right stall timeout for different task complexities?** Our default is 20 minutes. Simple MTs might stall in 5 minutes; complex MTs might legitimately take 30. Should the timeout be per-MT based on estimated complexity?

4. **How do you prevent coordination overhead from exceeding the value of parallel execution?** The orchestration protocols paper warns that coordination cost grows faster than the value of adding agents. At what team size does adding another agent hurt more than it helps? Claude Agent Teams recommends 3-5 teammates.

5. **Can failure memory generalize across projects?** A fix pattern from Handshake repo governance might apply to a completely different Rust project. But the context (crate structure, dependency versions, coding conventions) differs. How much generalization is safe?

---

## 7. Sources

### Frameworks
- [Swarms Framework](https://github.com/kyegomez/swarms) — enterprise-grade multi-agent orchestration
- [Agency Swarm](https://github.com/VRSEN/agency-swarm) — OpenAI Agents SDK extension
- [OpenAI Swarm](https://github.com/openai/swarm) — educational multi-agent framework (replaced by Agents SDK)
- [PARL](https://github.com/The-Swarm-Corporation/PARL) — parallel-agent reinforcement learning
- [Agent Swarm (desplega-ai)](https://github.com/desplega-ai/agent-swarm) — Docker-isolated coding agents
- [Overstory](https://github.com/jayminwest/overstory) — multi-runtime orchestration with SQLite mail
- [Metaswarm](https://github.com/dsifry/metaswarm) — 18-agent recursive orchestration with TDD
- [Ruflo](https://github.com/ruvnet/ruflo) — Claude orchestration platform

### Documentation
- [Claude Code Agent Teams](https://code.claude.com/docs/en/agent-teams) — official Anthropic multi-agent documentation
- [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/) — production agent framework
- [Anthropic 2026 Agentic Coding Trends Report](https://resources.anthropic.com/hubfs/2026%20Agentic%20Coding%20Trends%20Report.pdf)

### Research Papers
- [Multi-Agent LLM Orchestration Achieves Deterministic Decision Support](https://arxiv.org/abs/2511.15755)
- [The Orchestration of Multi-Agent Systems: Architectures, Protocols, and Enterprise Adoption](https://arxiv.org/html/2601.13671v1)
- [ParaCodex: Profiling-Guided Autonomous Coding Agent for Reliable Parallel Code Generation](https://arxiv.org/abs/2601.04327)

### Industry Analysis
- [Kimi K2.5 Agent Swarm](https://www.morphllm.com/kimi-k2-5-agent-swarm) — 100 parallel sub-agents
- [Agentic Coding 2026 Guide](https://www.verdent.ai/guides/ai-coding-agent-2026)
- [Agent Orchestration Patterns: Swarm vs Mesh vs Hierarchical](https://gurusup.com/blog/agent-orchestration-patterns)
- [Multi-Agent Frameworks for Enterprise AI](https://www.adopt.ai/blog/multi-agent-frameworks)
