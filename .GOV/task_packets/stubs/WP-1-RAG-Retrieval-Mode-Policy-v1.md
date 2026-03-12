# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
- For any stub covering current-spec Phase 1 roadmap additions (`[ADD v<current>]`), `ROADMAP_ADD_COVERAGE` MUST enumerate the exact spec line numbers so governance checks can verify no additions were missed.

---

# Work Packet Stub: WP-1-RAG-Retrieval-Mode-Policy-v1

## STUB_METADATA
- WP_ID: WP-1-RAG-Retrieval-Mode-Policy-v1
- BASE_WP_ID: WP-1-RAG-Retrieval-Mode-Policy
- CREATED_AT: 2026-03-11T12:30:00.000Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-AI-Ready-Data-Architecture, WP-1-ACE-Runtime, WP-1-Project-Brain-Runtime-Backfill, WP-1-Spec-Router-SpecPromptCompiler, WP-1-Micro-Task-Executor
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.178.md 7.6.3 (Phase 1) -> [ADD v02.178] Governed RAG retrieval modes and no-RAG policy
- ROADMAP_ADD_COVERAGE: SPEC=v02.178; PHASE=7.6.3; LINES=47314,47826,47842
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.178.md AI-Ready Data retrieval-mode ladder [ADD v02.178]
  - Handshake_Master_Spec_v02.178.md 2.5.8 Project Brain (RAG Interface) [ADD v02.178]
  - Handshake_Master_Spec_v02.178.md 2.6.6.7.14 ACE-RAG-001 [ADD v02.178]
  - Handshake_Master_Spec_v02.178.md 2.6.8.5 Prompt-to-Spec Router [ADD v02.178]
  - Handshake_Master_Spec_v02.178.md 2.6.6.8 Micro-Task Executor [ADD v02.178]

## INTENT (DRAFT)
- What: Define and later implement one governed retrieval-mode policy for Handshake so `QueryPlan`, `RetrievalTrace`, Project Brain, Prompt-to-Spec Router, Loom-aware graph expansion, Work Packet loads, and Micro-Task context assembly choose the cheapest authoritative retrieval path instead of defaulting to hybrid retrieval.
- Why: Handshake now has rich retrieval substrates and graph signals, but it still needs explicit no-RAG and lower-cost retrieval rules so exact identifiers, authoritative packet state, bounded local-small-model loops, and freshness-sensitive operations do not get blurred into one generic semantic-search path.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Retrieval-mode selection contract shared by `none`, `direct_load`, `exact_lookup`, `graph_traversal`, and `hybrid_rag`.
  - `non_hybrid_reason` policy and telemetry coverage for exact-id, authority, freshness, and bounded-executor cases.
  - Prompt-to-Spec authoritative preload rules and Project Brain discovery-versus-lookup posture.
  - Loom graph-bias rules that shape retrieval without replacing direct block or asset loads.
  - Work Packet and Micro-Task direct-load-first policy for authoritative and bounded local-model execution.
  - Dev Command Center, operator surfaces, and debug or replay visibility for retrieval mode decisions.
- OUT_OF_SCOPE:
  - Replacing AI-Ready Data substrate implementation or index storage choices.
  - Broad user-interface redesign of Project Brain or Loom.
  - General export or bundle portability beyond retrieval-mode evidence and traceability needed for Phase 1.

## UI_UX_SKETCH (DRAFT)
- Principle: prefer exposing retrieval posture and reason before adding new search complexity.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center retrieval trace inspector
  - Project Brain answer provenance drawer
  - Prompt-to-Spec routing provenance panel
  - Work Packet follow-up related-context section
  - Micro-Task compact context inspector
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Retrieval mode badge | Type: status chip | Tooltip: Shows whether the current answer or context used no retrieval, direct load, exact lookup, graph traversal, or hybrid retrieval. | Notes: opens trace details
  - Control: Non-hybrid reason badge | Type: status chip | Tooltip: Explains why hybrid retrieval was skipped. | Notes: hidden when hybrid retrieval is used
  - Control: Related-context expansion | Type: side panel | Tooltip: Shows graph-biased or hybrid related context without replacing the authoritative object view. | Notes: advisory only
  - Control: Compact context preview | Type: side panel | Tooltip: Shows what was compacted for a local-small-model Micro-Task. | Notes: bounded snippet only
- UI_STATES (empty/loading/error):
  - Exact authority loaded
  - Graph traversal used
  - Hybrid retrieval used
  - Hybrid retrieval skipped by policy
  - Freshness uncertain
  - Retrieval trace unavailable

## RESEARCH_SCOUTING (DRAFT)
- RESEARCH_CURRENCY_REQUIRED: YES
- TARGET_BUCKETS:
  - BIG_TECH
  - UNIVERSITY|PAPER
  - GITHUB|OSS_DOC
- SEARCH_SEEDS:
  - contextual retrieval direct load exact id policy
  - graph rag direct lookup authoritative state
  - bounded rag for local models compact context
  - filtered vector search exact metadata retrieval
  - hybrid retrieval chunking strategy authoritative ids
- CANDIDATE_SOURCES:
  - Source: Anthropic Contextual Retrieval | Kind: BIG_TECH | Date: 2024-11-13 | Retrieved: 2026-03-11T12:10:00Z | URL: https://www.anthropic.com/news/contextual-retrieval | Why: clarifies when contextual enrichment improves retrieval and why naive retrieval can fail.
  - Source: Microsoft GraphRAG | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T12:10:00Z | URL: https://github.com/microsoft/graphrag | Why: graph expansion patterns and retrieval over entity relationships.
  - Source: LightRAG | Kind: UNIVERSITY|PAPER | Date: unknown | Retrieved: 2026-03-11T12:10:00Z | URL: https://github.com/HKUDS/LightRAG | Why: lightweight graph-aware retrieval and hybrid indexing ideas.
  - Source: Qdrant filtering | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T12:10:00Z | URL: https://qdrant.tech/documentation/concepts/filtering/ | Why: exact metadata filtering before broad semantic recall.
  - Source: Cohere advanced chunking strategies | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T12:10:00Z | URL: https://docs.cohere.com/docs/advanced-chunking-strategies | Why: chunk selection and compaction design tradeoffs.
  - Source: OpenAI Cookbook pgvector semantic search | Kind: BIG_TECH | Date: unknown | Retrieved: 2026-03-11T12:10:00Z | URL: https://cookbook.openai.com/examples/vector_databases/pgvector/semantic_search | Why: practical filtered retrieval stack with vector search.
  - Source: Hugging Face smolagents RAG example | Kind: GITHUB|OSS_DOC | Date: unknown | Retrieved: 2026-03-11T12:10:00Z | URL: https://huggingface.co/docs/smolagents/examples/rag | Why: bounded retrieval patterns for agent loops and tool-backed retrieval.

## RESEARCH_DECISIONS (DRAFT)
- ADOPT:
  - Source: Qdrant filtering | Pattern: exact metadata filtering before semantic recall | Why: known ids and authoritative selectors should avoid broad hybrid retrieval.
  - Source: Anthropic Contextual Retrieval | Pattern: retrieval helps when the task is ambiguous or discovery-oriented | Why: supports Hybrid RAG as a discovery mode, not a universal default.
  - Source: Hugging Face smolagents RAG | Pattern: bounded retrieval for agent loops | Why: Micro-Tasks need compact context, not broad corpus dumps.
- ADAPT:
  - Source: Microsoft GraphRAG | Pattern: graph expansion and relationship-aware retrieval | Why: Loom should bias retrieval through graph signals, but Handshake must preserve direct-load-first authority.
  - Source: LightRAG | Pattern: efficient graph-aware hybrid retrieval | Why: good fit for Project Brain and Loom-assisted exploration, but only after exact lookups fail.
  - Source: Cohere chunking strategies | Pattern: context compaction and chunk quality controls | Why: retrieved context for local models must be compacted before execution.
- REJECT:
  - Source: vector-search-everything posture | Pattern: always default to semantic retrieval | Why: conflicts with authority, freshness, replay, and bounded local-model context rules.

## GITHUB_PROJECT_SCOUTING (DRAFT)
- SEARCH_QUERIES:
  - exact lookup before rag github
  - graph rag prompt router packet state github
- MATCHED_PROJECTS:
  - Repo: microsoft/graphrag | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: strong graph-expansion and entity-neighborhood retrieval design.
  - Repo: HKUDS/LightRAG | Intent: ARCH_PATTERN | Decision hint: ADAPT | Impact hint: SPEC_UPDATE | Notes: lightweight graph-aware hybrid retrieval posture.
  - Repo: openai/openai-cookbook | Intent: SPEC_PATTERN | Decision hint: ADOPT | Impact hint: SPEC_UPDATE | Notes: practical filtered vector retrieval and metadata-first selection examples.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: RAG | STATUS: TOUCHED | NOTES: retrieval modes and no-RAG policy are the direct subject of this stub | Stub follow-up: THIS_STUB
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: retrieval substrate, filtering, chunking, and reranking live here | Stub follow-up: THIS_STUB
  - PILLAR: Prompt-to-Spec | STATUS: TOUCHED | NOTES: router must preload authoritative state before hybrid retrieval | Stub follow-up: THIS_STUB
  - PILLAR: Loom | STATUS: TOUCHED | NOTES: graph signals bias retrieval but do not replace direct object loads | Stub follow-up: THIS_STUB
  - PILLAR: ACE | STATUS: TOUCHED | NOTES: retrieval mode selection and traceability are compiler/runtime duties | Stub follow-up: THIS_STUB
  - PILLAR: Work packets (product, not repo) | STATUS: TOUCHED | NOTES: authoritative direct-load-first execution contracts | Stub follow-up: THIS_STUB
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: bounded no-RAG or low-RAG defaults for local-small-model execution | Stub follow-up: THIS_STUB
