# Unified Merge Matrix + Exhaustive Outline for the Multi-Agent Architecture Research Paper

## Purpose

This file is the working merge blueprint for combining the three uploaded research drafts into one publishable paper.

### Source corpus
1. `Multi-agent architecture research by GPT 5.4 extended thinking pro.md`
2. `Deep Research Report on Multi‑Agent Architectures in LLM Systems.md`
3. `compass_artifact_wf-212edd11-6f93-411b-a7c1-1c02d2e455b5_text_markdown.md`

### Merge objective
Produce **one paper with one thesis**, while preserving **all named examples** from the source drafts.

The merged paper should read as:
- a coherent argument, not three stitched surveys
- a systems paper, not a repo directory
- a builder-facing technical analysis, not a hype piece

### Working thesis
Multi-agent systems stop being toy chat choreography when they are treated as a runtime problem:
- durable state
- orchestration
- isolated execution
- verification
- observability
- operator control

The paper should keep that thesis front and center, then use the examples as evidence.

## Prompt normalization

The original prompt repeats and overlaps several asks. For the merged paper, normalize it into this map:

1. **Anchor case studies** — Cursor and Steve Yegge / Gas Town / Beads  
2. **Representative GitHub systems** — mainstream frameworks and practical architectures  
3. **Novel / contrarian systems** — unusual coordination models or opposite design bets  
4. **Comparative architecture analysis** — trends, heuristics, use-case fit, anti-patterns  
5. **Gaps + risk engineering** — good ideas without depth, current mitigations, unresolved problems  
6. **Copy / combine / shrink** — reusable patterns, strong hybrids, minimum viable orchestrator  
7. **Local models + communication** — local serving, provider abstraction, inter-agent exchange patterns  
8. **UI / UX / GUI** — what a serious operator surface needs  
9. **Synthesis** — insights, advice, concerns, convergence / divergence  
10. **3–5 year target architecture** — what the harness should look like and what current tech already supports

### Editorial rule for UI duplication
The prompt asks for UI/UX/GUI repeatedly. Consolidate that in two layers:
- a **short UI/UX note inside each system entry**
- a **single dedicated UI/UX/GUI chapter** for the build-your-own guidance

## Editorial decisions

### Role assignment by source file

| File | Primary role in the merge |
|---|---|
| `Multi-agent architecture research by GPT 5.4 extended thinking pro.md` | narrative spine, architecture families, copyable patterns, hybrid combinations, smallest viable footprint, 3–5 year harness |
| `Deep Research Report on Multi‑Agent Architectures in LLM Systems.md` | evidence layer for Cursor / Gas Town, sandboxes, worktrees, hooks, ticket / PR workflows, control-plane UX |
| `compass_artifact_wf-212edd11-6f93-411b-a7c1-1c02d2e455b5_text_markdown.md` | breadth layer for frontier paradigms, protocol stack, quantitative framing, local serving detail, communication taxonomies |

### Hard merge rules

1. **Do not delete named examples.**  
   Keep every named example from the three drafts somewhere in the merged paper, even if some move into sidebars or appendices.

2. **Merge at the claim level, not the paragraph level.**  
   Prefer one strong paragraph per claim, then support it with additional examples.

3. **Keep one description per project in the main body.**  
   If the same system appears multiple times across drafts, give it one canonical slot and reference it elsewhere only when needed.

4. **Separate core evidence from frontier signals.**  
   Repo-grounded systems belong in the main body. More speculative paradigms belong in a dedicated frontier chapter or appendix.

5. **Keep the paper builder-facing.**  
   The paper should answer:
   - what actually works
   - what breaks
   - what can be copied
   - what a next-generation harness should look like

### Recommended output shape
- **main paper**: case studies + grouped systems + synthesis
- **appendix**: exhaustive registry of additional examples and frontier signals

## Merge matrix

| Unified section | Prompt coverage | Function in merged paper | Lead source | Supporting source(s) | Notes |
|---|---|---|---|---|---|
| 1. Executive summary | All topics | State the thesis once: multi-agent is a runtime/state/control problem, not a chat-window count problem. | GPT54 | DR, Compass | Use no long catalogs here. Only name Cursor, Gas Town/Beads, LangGraph, Open SWE/OpenHands, and protocols as representative anchors. |
| 2. Scope, method, and selection criteria | All topics | Explain why the paper mixes inspectable repos, product case studies, and frontier paradigms. | GPT54 | Compass | Define 'representative', 'novel', 'frontier', and 'protocol' buckets. |
| 3. Anchor case studies: Cursor vs Steve Yegge / Gas Town / Beads | Topic 1 | Contrast IDE-native control-plane design with colony-manager / durable coordination-substrate design. | DR | GPT54, Compass | Also slot Claude Code, Codex, Cline, Roo Code, OpenHands, Open SWE, Cairn as adjacent SWE systems. |
| 4. Mainstream architecture families and representative GitHub systems | Topic 2 | Group the conventional examples by architecture family instead of forcing an arbitrary top-10. | GPT54 | DR, Compass | Include every mainstream example from all drafts; use a normalized entry template. |
| 5. Novel, minimal, opposite, and contrarian systems | Topic 3 | Collect the repo-grounded unconventional systems and explain what design variable they push on. | GPT54 + DR | Compass | Keep repo-backed systems in the main body; push research-only paradigms to the next section. |
| 6. Frontier paradigms beyond current mainstream practice | Topic 3 tail / Topic 9 / Topic 10 | Cover academic or ecosystem-level ideas that point beyond today’s frameworks. | Compass | GPT54 | This is where AIOS, CoMLRL, DALA, AgentSociety, AGNTCY, etc. belong. |
| 7. Comparative architecture analysis | Topic 4 | Synthesize the families, heuristics, best-fit builds, and anti-patterns. | GPT54 | DR, Compass | This replaces repeated trend discussion scattered across the three drafts. |
| 8. Good ideas without depth, gaps, and risk engineering | Topic 5 | Merge gaps, mitigations, and unaddressed concerns into one operational chapter. | GPT54 + DR | Compass | Keep concrete technical mitigation patterns: sandboxes, hooks, middleware, checkpoints, typed outputs, audit trails. |
| 9. What to copy, what to combine, and how to shrink it | Topic 6 | Turn the research into direct build recommendations. | GPT54 | DR, Compass | Keep the three hybrid combinations and smallest viable orchestrator discussion. |
| 10. Local model implementation patterns | Topic 6 / Topic 7 overlap | Explain how local inference is actually wired into these systems. | Compass | GPT54, DR | Provider abstraction, serving stacks, model routing, reliability limits, resource constraints. |
| 11. Communication between agents/models | Topic 7 | Taxonomy of communication substrates plus breakpoints and failure modes. | DR | Compass, GPT54 | Shared transcripts, message hubs, blackboards, artifacts, protocol RPC. |
| 12. UI/UX/GUI for a serious multi-agent product | Topic 8 | Define the control-plane UI requirements. | DR | GPT54, Compass | Mission control, artifact review, trace/replay, permission surfaces, environment clarity. |
| 13. Insights, advice, concerns, convergence/divergence | Topic 9 | Close the analysis layer before the future architecture chapter. | GPT54 | DR, Compass | Separate high-confidence conclusions from speculative bets. |
| 14. The 3–5 year reference architecture / harness | Topic 10 | Specify the layered target architecture and near-term implementation path. | GPT54 | DR, Compass | Interface layer, control plane, state plane, execution plane, interop/tool plane, role set, build path. |
| Appendix A. Exhaustive example registry | All example asks | Guarantee that no named example is lost. | Merged | Merged | List every system, protocol, and infrastructure component with its placement. |
| Appendix B. Citation cleanup and terminology normalization | Editorial | Resolve inconsistent naming and citation style across drafts. | Merged | Merged | Convert internal citation tokens, normalize project names, verify quantitative claims. |


## Unified outline draft

## 1. Executive Summary
1.1 Core thesis  
1.2 Main conclusions for builders  
1.3 What the paper covers and excludes  

## 2. Scope, Method, and Selection Criteria
2.1 Why Cursor and Steve Yegge / Gas Town / Beads are the anchor cases  
2.2 Why repo-inspectable systems matter  
2.3 How “representative”, “novel”, and “frontier” examples are separated  
2.4 Evidence quality and citation policy  

## 3. Cursor vs Steve Yegge / Gas Town / Beads
3.1 Cursor’s evolution
- shadow workspace
- worktrees / remote isolation
- background agents
- model routing / subagents
- hooks, sandboxes, approvals
- control-plane UI

3.2 Gas Town / Beads
- mayor / worker colony model
- durable work graph
- mailboxes, hooks, watchdogs
- operator console orientation
- colony-manager mental model

3.3 Direct comparison
- control plane vs coordination substrate
- IDE-native vs CLI colony manager
- artifact review vs durable work ledger
- mainstream supervision vs frontier operator workflow

3.4 Adjacent SWE systems in the same orbit
- Claude Code
- Codex
- Cline
- Roo Code
- OpenHands
- Open SWE
- Cairn

## 4. Mainstream architecture families and representative GitHub systems
Use one consistent entry template per system:
**Overview → Technical approach → Other incorporated systems/features → UI/UX/GUI note**

### 4.1 Graph / state-machine runtimes
- Microsoft Agent Framework
- AutoGen
- LangGraph
- Semantic Kernel
- PydanticAI
- Mastra

### 4.2 Role / team / organization abstractions
- CrewAI
- MetaGPT
- CAMEL
- Agency Swarm
- AutoGPT
- ChatDev

### 4.3 Coding-agent harnesses and workflow platforms
- OpenHands
- Open SWE
- TaskWeaver
- AG2
- AgentScope

### 4.4 What this mainstream set implies
- why durable state keeps recurring
- why orchestration and observability keep moving together
- where mainstream systems still stay vague

## 5. Novel, minimal, opposite, and contrarian repo-grounded systems
### 5.1 Durable coordination and memory-first systems
- Gas Town
- Beads
- Letta
- Letta Code
- GNAP

### 5.2 Minimal and scaffold-light systems
- OpenAI Swarm
- SWE-agent / mini-swe-agent
- smolagents
- PocketFlow

### 5.3 Protocol-first and substrate-first systems
- A2A Protocol
- MCP specification / MCP servers
- BeeAI Framework
- Langroid

### 5.4 Alternate runtime / workflow bets
- Cairn
- BabyAGI 2o
- OWL
- SuperAGI
- Mastra
- ChatDev (cross-reference if already handled above)

### 5.5 What these contrarian systems teach
- when less scaffold wins
- when memory beats chat
- when protocolization matters
- when colony management becomes the real product

## 6. Frontier paradigms beyond mainstream frameworks
### 6.1 Decentralized / protocol / internet-of-agents visions
- Fetch.ai uAgents
- AGNTCY

### 6.2 Emergent social / debate / market coordination
- AgentSociety
- MAD
- DALA
- Market Making

### 6.3 Learned or formalized coordination
- CoMLRL
- Declarative pipeline DSL

### 6.4 OS-kernel and macro-governance metaphors
- AIOS
- Artificial Leviathan

### 6.5 Why these matter even if they are not near-term defaults

## 7. Comparative architecture analysis
7.1 Architecture families
- graph/state runtimes
- role/team abstractions
- coding harnesses
- persistent memory / task-graph systems
- protocol / interoperability layers
- frontier coordination paradigms

7.2 Heuristic families
- planner → worker → judge
- manager / worker star topologies
- handoff networks
- artifact-first pipelines
- typed-output / validation-first designs
- model routing
- market / debate / learned coordination

7.3 Best-fit builds by use case
- solo IDE acceleration
- background SWE automation
- enterprise workflow systems
- research / evaluation systems
- high-parallelism agent colonies
- protocol-federated cross-org systems

7.4 Anti-patterns
- premature multi-agent decomposition
- chat as state
- approval spam
- tool sprawl
- context explosion
- overbuilt orchestration for simple tasks

## 8. Good ideas without depth, technology gaps, and risk engineering
8.1 Good ideas that are underspecified
8.2 Technology gaps
- durable task/artifact graphs
- memory lifecycle management
- team-level evaluation
- failure attribution
- policy / trust layers
- cross-framework interoperability maturity

8.3 How current systems technically mitigate risk
- sandboxes, worktrees, containers, VMs
- hooks and deterministic middleware
- human approvals and scoped gating
- audit trails and observability
- typed outputs and validators
- tests / linters / PRs as judges

8.4 Under-addressed risks
- prompt injection at the agent boundary
- memory poisoning
- cost runaway and model DoS
- supply chain risk in tools / plugins / MCP servers
- branch ownership and merge ambiguity
- collusion / emergent bad behavior
- weak whole-system evaluation

## 9. What to copy, combine, and shrink
9.1 Patterns worth copying now
9.2 Three strong hybrid combinations
9.3 Smallest viable orchestrator
9.4 When not to use multi-agent at all

## 10. Local models in multi-agent systems
10.1 Provider abstraction as the dominant pattern
10.2 Serving stacks and what each contributes
- Ollama
- vLLM
- llama.cpp
- LocalAI
- LM Studio
- LiteLLM as routing middleware

10.3 Hybrid local/cloud routing by role
10.4 Tool-calling reliability limits
10.5 Resource constraints and context management
10.6 Why local inference and local execution isolation are different problems

## 11. How agents communicate
11.1 Shared transcript / handoff history
11.2 Message hubs and actor-style routing
11.3 Shared memory / blackboard / graph state
11.4 Artifact exchange
11.5 Protocol RPC and streaming
11.6 Breakpoints and failure modes
11.7 Novel communication concepts worth tracking

## 12. UI/UX/GUI for building your own system
12.1 Chat is one pane, not the operating system
12.2 Mission-control view of concurrency
12.3 Artifact-first review
12.4 Trace, replay, and time-travel
12.5 Permission and policy surfaces
12.6 Environment clarity
12.7 Cost visibility and escalation
12.8 Multi-repo / multi-workspace / local-cloud handoff ergonomics

## 13. Insights, advice, and concerns
13.1 Main insights after the research
13.2 Practical advice for builders
13.3 Main concerns
13.4 Where the field is converging
13.5 Where it is diverging

## 14. The 3–5 year reference architecture / harness
14.1 Layered reference architecture
- interface layer
- control plane
- state plane
- execution plane
- tooling / interoperability plane

14.2 Standard role set
- planner
- decomposer
- executor
- verifier / judge
- researcher
- reviewer
- supervisor / operator bridge

14.3 What current technology already enables
14.4 What still needs maturity
14.5 Suggested build path
- v1 narrow orchestrator
- v2 isolated worker pool + operator console
- v3 persistent memory + protocol federation

## Appendix A. Exhaustive example registry
Map every named example to its place in the merged paper.

## Appendix B. Citation cleanup and terminology normalization

## Exhaustive example registry

This registry exists so that **all named examples from the three source drafts remain accounted for**.

### Anchor case studies and adjacent SWE products

| Example | Recommended placement | Why it stays |
|---|---|---|
| Cursor | Section 3 | Primary anchor case study; control-plane, worktrees, sandboxes, hooks, approvals. |
| Steve Yegge / Gas Town | Section 3 | Primary anchor case study; colony manager and high-parallelism operator workflow. |
| Beads | Section 3 + Section 5 | Durable task graph / coordination substrate. |
| Claude Code | Section 3.4 | Adjacent coding-agent benchmark for tools, hooks, checkpoints, autonomy. |
| Codex | Section 3.4 | Adjacent command-center / worktree / subagent reference. |
| Cline | Section 3.4 | Human-in-the-loop coding agent with strong permission surfaces. |
| Roo Code | Section 3.4 | Mode-based alternative to full concurrent swarms. |
| OpenHands | Section 3.4 + Section 4 | Open-source coding-agent platform and sandboxed execution reference. |
| Open SWE | Section 3.4 + Section 4 | Issue/PR-first async coding-agent pattern. |
| Cairn | Section 3.4 + Section 5 | Background coding-agent system with web interface and task persistence. |

### Mainstream frameworks and representative systems

| Example | Recommended placement | Why it stays |
|---|---|---|
| Microsoft Agent Framework | Section 4.1 | Enterprise graph orchestration, checkpointing, time-travel, telemetry. |
| AutoGen | Section 4.1 | Classic layered multi-agent conversation framework; historical importance. |
| AG2 | Section 4.3 | AutoGen lineage reframed as AgentOS / conversable-agent orchestration. |
| LangGraph | Section 4.1 | Durable graph orchestration and observability anchor. |
| CrewAI | Section 4.2 | Role-based collaboration plus deterministic flows. |
| Semantic Kernel | Section 4.1 | Plugin-centric enterprise SDK with local deployment story. |
| MetaGPT | Section 4.2 | SOP-based software-company-as-agents pattern. |
| ChatDev | Section 4.2 or 5.4 | Virtual software company plus learnable orchestrator / zero-code turn. |
| CAMEL | Section 4.2 | Role-playing / scaling-law / research-oriented framework. |
| AgentScope | Section 4.3 | Message hub, MCP/A2A integration, runtime and observability. |
| TaskWeaver | Section 4.3 | Code-first analytics agent with shared memory and execution history. |
| AutoGPT | Section 4.2 | Low-code platform / agent builder / monitoring surface. |
| Agency Swarm | Section 4.2 | Organizational communication graph with explicit developer control. |
| PydanticAI | Section 4.1 | Typed output, validation-first, durable execution, HITL approvals. |
| Mastra | Section 4.1 or 5.4 | Full-stack TypeScript agent runtime with graphs, memory, and UI integration. |

### Contrarian / minimal / protocol-first repo-grounded systems

| Example | Recommended placement | Why it stays |
|---|---|---|
| OpenAI Swarm | Section 5.2 | Minimal handoff runtime; lower-bound orchestrator. |
| Letta | Section 5.1 | Memory-first long-lived agents. |
| Letta Code | Section 5.1 | Coding harness on top of long-lived memory. |
| smolagents | Section 5.2 | Tiny scaffold, code-first actions, strong sandbox support. |
| PocketFlow | Section 5.2 | Minimal graph runtime with shared-state communication. |
| BeeAI Framework | Section 5.3 | Policy surface plus A2A/MCP serving direction. |
| A2A Protocol | Section 5.3 | Agent-to-agent interoperability substrate. |
| MCP specification | Section 5.3 | Tool/resource interoperability substrate. |
| MCP servers | Section 5.3 | Operational ecosystem around protocol-first tool exposure. |
| GNAP | Section 5.1 | Git-native four-entity coordination substrate. |
| SWE-agent / mini-swe-agent | Section 5.2 | Minimal coding-agent scaffold and trajectory-first debugging. |
| BabyAGI 2o | Section 5.4 | Self-building tool-creation experiment. |
| OWL | Section 5.4 | CAMEL-derived multi-agent workforce with browser and MCP support. |
| Langroid | Section 5.3 | Actor-like multi-agent programming model. |
| SuperAGI | Section 5.4 | GUI-heavy autonomous-agent platform with permission console. |

### Frontier paradigms and out-of-distribution coordination models

| Example | Recommended placement | Why it stays |
|---|---|---|
| Fetch.ai uAgents | Section 6.1 | Blockchain identity, discovery, payments, decentralized coordination. |
| AGNTCY | Section 6.1 | Internet-of-agents stack, directory/discovery/security substrate. |
| AgentSociety | Section 6.2 | Large-scale emergent social simulation. |
| MAD | Section 6.2 | Adversarial debate as a reasoning pattern. |
| DALA | Section 6.2 | Auction-based communication / strategic silence. |
| Market Making | Section 6.2 | Prediction-market-style truth-seeking coordination. |
| CoMLRL | Section 6.3 | MARL-trained cooperation replacing hand-built heuristics. |
| Declarative pipeline DSL | Section 6.3 | Language-agnostic workflow IR and formal pipeline definition. |
| AIOS | Section 6.4 | LLM operating system / kernel metaphor. |
| Artificial Leviathan | Section 6.4 | Social-contract / sovereign emergence paradigm. |

### Local-model and serving infrastructure examples

| Example | Recommended placement | Why it stays |
|---|---|---|
| Ollama | Section 10 | Local model runner and common adapter target. |
| vLLM | Section 10 | PagedAttention and serving throughput anchor. |
| llama.cpp | Section 10 | Low-level local inference and constrained grammars. |
| LocalAI | Section 10 | OpenAI-compatible local serving shim. |
| LM Studio | Section 10 | Desktop / local API bridge. |
| LiteLLM | Section 10 | Provider routing / compatibility middleware. |

### UI / protocol / observability ecosystem examples

| Example | Recommended placement | Why it stays |
|---|---|---|
| AG-UI | Section 12 | Agent-to-frontend event protocol; include in UI chapter or appendix. |
| A2UI | Section 12 | Generative UI protocol; include in UI chapter or appendix. |
| LangSmith | Section 12 | Observability / traces / debugging reference. |
| AutoGen Studio | Section 12 | Visual multi-agent composition reference. |
| Dify | Appendix or Section 12 sidebar | Production workflow/debugging UI example. |
| LangFlow | Appendix or Section 12 sidebar | Visual DAG editor / IDE reference. |
| n8n | Appendix or Section 12 sidebar | General workflow automation UI reference. |
| Flowise | Appendix or Section 12 sidebar | Lightweight flow builder reference. |
| Rivet | Appendix or Section 12 sidebar | Reasoning / process visibility example. |
| CopilotKit | Appendix or Section 12 sidebar | Frontend integration surface. |


## Suggested assembly order

1. **Write Sections 1–3 first.**  
   Lock the thesis and the Cursor vs Gas Town framing before expanding the catalog sections.

2. **Build the project entry library next.**  
   Write short normalized entries for every system in Sections 4–6 using the same entry template.

3. **Write the synthesis chapters after the examples are normalized.**  
   Only then write Sections 7–14 so the analysis emerges from the examples rather than being imposed too early.

4. **Append the registry and citation cleanup last.**  
   The registry is a completeness check, not the main narrative.

## Recommended entry template for every system

Use this exact four-part structure for each project entry:

```text
Overview
Technical approach
Other systems / incorporated features
UI/UX/GUI note
```

This keeps the paper consistent even when the systems are very different.

## Acceptance checklist

A merged draft is ready when all of the following are true:

- The paper has **one central thesis** and does not read like three surveys glued together.
- Cursor and Gas Town / Beads are treated as the two anchor case studies.
- All named examples from the three drafts appear somewhere in the paper or appendix.
- The “mainstream”, “contrarian repo-grounded”, and “frontier paradigm” buckets are clearly separated.
- UI/UX/GUI guidance lives mostly in one dedicated chapter, with only short notes inside system entries.
- Risk discussion is concrete and technical, not generic.
- The final chapter specifies a layered 3–5 year reference harness.
- Citation style is normalized across the merged document.

## Short editorial recommendation

Do **not** force a fixed “10 + 10” structure in the merged paper.

The source material now contains too many valid examples for that frame to remain useful. A stronger merged paper will:
- keep the prompt spirit
- preserve all examples
- organize them by architecture family and evidentiary value
- move overflow examples into appendix or sidebar treatment instead of dropping them
