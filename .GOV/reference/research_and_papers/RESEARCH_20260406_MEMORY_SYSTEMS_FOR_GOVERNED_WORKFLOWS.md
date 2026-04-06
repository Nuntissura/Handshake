# Memory Systems for Governed Multi-Session Workflows

- RESEARCH_ID: RESEARCH-20260406-MEMORY-SYSTEMS-GOVERNED-WORKFLOWS
- DATE: 2026-04-06
- AUTHOR: Orchestrator (Claude Opus 4.6)
- SCOPE: Provider-agnostic memory architectures for repo governance — indexing, RAG, compaction, multi-layered memory, cross-session knowledge persistence
- RELATED_GOVERNANCE: RGF-101 (SQLite backbone), RGF-103 (failure memory), CX-503P (self-hosting convergence)

---

## 1. Executive Summary

This research surveys the 2025-2026 memory system landscape for LLM agents, with a focus on what can be applied to Handshake's repo governance workflow. The goal: any model (GPT, Claude, Codex, future providers) that enters a governed session should have access to structured, retrievable memory from all previous sessions — across months, across roles, across WPs.

Key finding: the industry has converged on a multi-layered architecture (working + episodic + semantic + procedural memory) backed by hybrid retrieval (vector embeddings + BM25 keyword search + knowledge graph). The infrastructure gap in repo governance is not the concept — it's that governance artifacts (receipts, thread entries, smoketest reviews, packets) exist as files but are not indexed, not queryable, and not automatically loaded into session context.

The recommended path: SQLite as the unified backbone (already started with RGF-101), FTS5 for keyword search, local embeddings via nomic-embed-text for semantic search, cron-based compaction for hygiene, and a pointer-index pattern for session startup context.

---

## 2. Current State: What Exists, What's Missing

### What we have

| Artifact | Format | Queryable? | Loaded at startup? |
|---|---|---|---|
| Task packets | Markdown | No (file read only) | Yes (active packet) |
| Refinements | Markdown | No | Yes (active refinement) |
| RECEIPTS.jsonl | JSONL per WP | No (linear scan) | No |
| THREAD.md | Markdown per WP | No | No |
| RUNTIME_STATUS.json | JSON per WP | Yes (parsed) | Yes (active WP) |
| Smoketest reviews | Markdown | No | No |
| Governance changelog | Markdown | No | No |
| RGF task board | Markdown | No | No |
| Failure memory (RGF-103) | Planned | Planned | Planned |
| SQLite comms (RGF-101) | Planned | Planned | Planned |

### What's missing

1. **No cross-session retrieval**: coder session 2 cannot find what coder session 1 learned
2. **No semantic search**: "how did we fix the DateTime import issue?" requires manual file scanning
3. **No compaction**: RECEIPTS.jsonl grows unbounded; old WP artifacts accumulate
4. **No structured handoff memory**: session-to-session context transfer is prompt-based, not indexed
5. **No pattern extraction**: repeated fix patterns are not captured as reusable knowledge
6. **No governance artifact indexing**: 100+ governance files exist but aren't searchable
7. **No decay or hygiene**: stale knowledge from months-old sessions has equal weight to recent knowledge

---

## 3. GitHub Repository Survey

### 3.1 Dedicated Memory Frameworks

| Project | Stars | Architecture | Storage | Indexing | Compaction | Provider Agnostic? |
|---|---|---|---|---|---|---|
| **Mem0** | ~52k | Two-phase (extract + update), dual-storage | Qdrant + SQLite/Postgres | Embedding + keyword hybrid | LLM semantic dedup + merge | Yes |
| **Letta/MemGPT** | ~22k | OS-inspired 3-tier (core/recall/archival) | Postgres (42 tables) + pgvector | Vector embeddings | Agent self-edits memory | Yes |
| **Graphiti** (Zep) | ~25k | Temporal knowledge graph | Neo4j/FalkorDB/Kuzu | Bi-temporal graph + vector | Temporal invalidation | Yes |
| **Cognee** | ~15k | 6-stage pipeline with knowledge graph | Kuzu + LanceDB + SQLite | Vector + graph hybrid | Self-improving (prune/reweight) | Yes |
| **Supermemory** | ~21k | Fact extraction + dual timestamps | Proprietary | Semantic fact search | Auto-contradiction/forgetting | Yes |
| **MemOS** | ~8k | "Memory OS" with skill evolution | Neo4j + Qdrant + SQLite + Redis | FTS5 + vector hybrid | Task summarization | Yes |
| **Hermes Agent** | ~28k | ReAct loop + skill documents | FTS5 (SQLite) | FTS5 full-text | Agent-curated + cron | Yes (200+ models) |
| **memU** | ~13k | 3-layer hierarchical (resource/item/category) | Postgres + pgvector | Vector embeddings | Auto-flush before compaction | Partial |
| **OpenMemory** (Cavira) | ~4k | 5-sector cognitive (episodic/semantic/procedural/emotional/reflective) | SQLite or Postgres | Multi-sector + temporal graph | Adaptive decay | Yes |
| **A-MEM** | ~1k | Zettelkasten-inspired linked notes | ChromaDB | Dynamic indexing + linking | Network self-organizes | Yes |
| **LangMem** | ~1k | Semantic + episodic + procedural | Pluggable (LangGraph stores) | Vector search | Consolidation + removal | Yes |
| **SimpleMem** | ~3k | Triple-index (semantic + lexical + symbolic) | Triple-layer indices | Dense + BM25 + metadata | Semantic lossless compression | Yes (MCP) |
| **ReMe** | ~3k | File-based markdown + vector | Filesystem (MEMORY.md + JSONL) | File retrieval + optional embeddings | Differential compression by age | Yes |
| **Google Always-On** | N/A | Multi-agent with scheduled consolidation | SQLite (2 tables) | Structured metadata | 30-minute consolidation cron | Partial |
| **ByteRover** | N/A | Hierarchical context tree (markdown-on-disk) | Filesystem (markdown) | 5-tier progressive retrieval | Adaptive knowledge lifecycle | Yes |
| **TeleMem** | ~457 | Mem0 drop-in with character profiles | FAISS + JSON | Vector + character isolation | LLM semantic clustering | Partial |
| **memclawz** | N/A | Causality graph + working memory scratchpad | Qdrant + Neo4j | Composite scoring | Sleep-time reflection | Yes (local) |

### 3.2 Coding-Agent Memory Systems

| System | Architecture | Storage | Cross-Session? | Key Innovation |
|---|---|---|---|---|
| **Claude Code** | CLAUDE.md + auto-memory pointer index | Markdown files in `.claude/` | Yes (persistent files) | Pointer-index: MEMORY.md as lightweight index, topic files on-demand |
| **Cursor** | Codebase vector index + `.cursorrules` | Turbopuffer (vector) | Index yes, memory limited | Comment/docstring-weighted embeddings, two-stage retrieval |
| **Windsurf** | 6-step context assembly + Memories | Proprietary | Yes (workspace-scoped) | Flow awareness (auto-tracks IDE actions), rules vs memories split |
| **Aider** | Graph-ranked repo map (tree-sitter + PageRank) | diskcache + tree-sitter tags | No (stateless) | PageRank personalization by mentioned identifiers |
| **Continue.dev** | 4-backend indexing (vector + symbol + FTS5 + chunks) | LanceDB + SQLite FTS5 | Index yes, memory no | Multi-backend fusion, tree-sitter AST chunking |
| **Overstory** | SQLite WAL mail + checkpoints + agent CVs | SQLite | Yes (checkpoints, CVs) | Typed message protocol, instruction overlays |
| **Metaswarm/BEADS** | Git-native SQLite + JSONL dual-layer | SQLite + JSONL | Yes (git-versioned) | `bd prime` file-scope knowledge priming, `bd compact` memory decay |
| **Cline** | Structured `.memory-bank/` directory | Markdown files | Yes (file-based) | Typed memory files (projectbrief, activeContext, systemPatterns) |
| **VS Code Copilot** | 3-scope memory (user/repo/session) | Files + GitHub hosted | Yes (28-day expiry) | Auto-validation against current codebase before applying |
| **AgentMemory** | Triple-stream (BM25 + vector + knowledge graph) | Pluggable | Yes (MCP server) | 12 lifecycle hooks, P2P mesh sync, 103 REST endpoints |

### 3.3 Vector Databases (Infrastructure Layer)

| System | Stars | Best For | SQLite Compatible? |
|---|---|---|---|
| **ChromaDB** | ~27k | Simplicity (4-function API) | SQLite-backed persistent mode |
| **Qdrant** | ~29k | Performance (Rust, GPU-accelerated) | No |
| **Milvus** | ~40k | Scale (distributed) | No |
| **Weaviate** | ~16k | Object + vector co-storage | No |
| **sqlite-vec** | ~4k | Zero-dependency vector search in SQLite | YES (single file) |
| **LanceDB** | ~12k | File-based, no server | No (but file-local) |

---

## 4. Technique Deep Dives

### 4.1 Retrieval Techniques

**Hybrid search is the 2026 consensus.** Dense (vector) + sparse (BM25/FTS5) with Reciprocal Rank Fusion (RRF):
```
Score_total = (w_vec / (k + rank_vec)) + (w_fts / (k + rank_fts))
```
Where k=60 is the industry standard. 15-30% better recall than either approach alone.

**For SQLite specifically**: FTS5 provides BM25 natively. sqlite-vec adds vector search. A single `.sqlite` file contains both indices. This is the lightweight path — no Qdrant, no Postgres, no external services.

**Reranking**: After broad retrieval, cross-encoder reranking (BGE-reranker for self-hosted) narrows to the top 3-5 most relevant results. Adds 50-100ms latency but significantly improves precision.

**Contextual compression**: After retrieval, extract only the parts relevant to the query. EmbeddingsFilter (cheap, fast) or LLMChainExtractor (accurate, expensive). Critical for keeping session startup context lean.

### 4.2 Embedding Models (Provider-Agnostic)

| Model | MTEB Score | Dims | License | Self-Host | Best For |
|---|---|---|---|---|---|
| nomic-embed-text-v1.5 | ~62 | 768 (flex) | Apache 2.0 | Ollama | Best openness (weights + code + data), strong quality |
| all-MiniLM-L6-v2 | 56.3 | 384 | Apache 2.0 | CPU | Fastest (14.7ms/1K tokens), smallest |
| BGE-M3 | 63.0 | 1024 | MIT | Yes | Dense + sparse + multi-vector from one model |
| Qwen3-Embedding-8B | 70.58 | 7168 | Apache 2.0 | GPU | Highest quality open-source |

**Recommendation for repo governance**: nomic-embed-text-v1.5. Runs on Ollama, fully open, strong quality, Matryoshka dimensions (can reduce to 256 for storage savings). Falls back to all-MiniLM-L6-v2 on CPU-only machines.

### 4.3 Multi-Layered Memory Architecture

The converged model across research and production systems:

**Working Memory** (current session context)
- The LLM context window itself
- Managed by token budgeting and context assembly
- Letta pattern: "core memory" blocks pinned in context like RAM

**Episodic Memory** (what happened in past sessions)
- Records of specific events with temporal context
- "Coder session on WP-1-Storage-Trait-Purity-v1 hit a CRLF issue in sqlite.rs on 2026-03-15"
- Stored with timestamps, participants, outcomes, file scope
- Retrieved by temporal queries or situation similarity

**Semantic Memory** (facts, patterns, knowledge)
- Generalized knowledge distilled from experience
- "This codebase uses the Database trait boundary for all storage access"
- Formed through consolidation from episodic memory
- Stored as structured facts or entity-attribute-value triples

**Procedural Memory** (how to do things)
- Step-by-step knowledge of how to perform tasks
- "To fix a DateTime import issue in sqlite.rs: add `use chrono::{DateTime, Utc};`"
- Extracted from successful fix patterns (RGF-103 failure memory is this layer)

**Layer interaction**:
1. New session loads pointer-index (semantic + procedural summaries)
2. MT assignment triggers scoped retrieval (episodic + semantic for relevant files)
3. During execution, new episodic memories are written
4. On session completion, episodic memories consolidate into semantic facts
5. Successful fix patterns become procedural memory
6. Cron job periodically compacts old episodic memories and prunes stale semantic facts

### 4.4 Compaction and Hygiene

**Summarization-based compaction**: Periodically compress old episodic memories into semantic facts. ReMe's differential approach: recent memories get light compression, old memories get aggressive summarization.

**Importance scoring**: Mnemosyne's hybrid: connectivity (how linked) + frequency (how often accessed) + recency (time since last access) + entropy (information content). Memories below threshold are archived.

**Temporal decay (Ebbinghaus)**: `strength = importance * e^(-lambda * days) * (1 + recall_count * 0.2)`. Memories decay unless reinforced by access. MemoryBank implements this with spaced-repetition-style refreshes.

**Cron-based maintenance** (production pattern):
- Every commit: index new/changed governance artifacts
- Every session completion: extract episodic memories from receipts + thread entries
- Daily: deduplication sweep (merge near-duplicate memories)
- Weekly: compaction run (summarize old episodic into semantic)
- Monthly: integrity check + re-embedding if model changed

### 4.5 Corrective and Self-RAG

**CRAG** (Corrective RAG): external evaluator scores retrieval quality. Above threshold: use directly. Between thresholds: supplement with broader search. Below threshold: discard and search differently. Plug-and-play improvement for any RAG pipeline.

**Self-RAG**: the model itself decides whether to retrieve, whether results are relevant, and whether the response is supported. Requires model fine-tuning but produces self-aware retrieval.

---

## 5. Translation Matrix: Each Approach to Our Governance

| Pattern | Source | Governance Translation | Effort |
|---|---|---|---|
| **Pointer-index at startup** | Claude Code MEMORY.md | SQLite `memory_index` table loaded at session start; full content fetched on-demand | LOW |
| **FTS5 + sqlite-vec hybrid search** | Continue.dev, OpenClaw | Single SQLite file with FTS5 for keyword + sqlite-vec for semantic search over all governance artifacts | MEDIUM |
| **Typed message protocol** | Overstory | Already in RGF-101 (SQLite comms). Extend to memory operations (add/retrieve/update/compact) | LOW |
| **File-scope knowledge priming** | BEADS `bd prime` | At MT start, load only memories relevant to the MT's file scope and task type | MEDIUM |
| **Dual-layer storage (SQLite + JSONL)** | BEADS | SQLite for speed, JSONL for git auditability. Already the direction with RGF-101 receipts | LOW |
| **Structured handoff objects** | Azure patterns, ADK | MT handoffs as typed JSON with relevant memory context, not free text | MEDIUM |
| **Memory decay / compaction** | BEADS `bd compact`, ReMe | Cron job summarizes old episodic memories; importance scoring prunes stale facts | MEDIUM |
| **Failure memory extraction** | RGF-103, desplega-ai | Extract error-fix pairs from smoketest reviews + validator findings into procedural memory | LOW (RGF-103 exists) |
| **Adversarial cross-model review** | Metaswarm | Already in RGF-99 (adversarial validator). Memory confirms the pattern works | NONE |
| **Self-improving memory** | Cognee | Stale node pruning, edge reweighting based on access patterns. Apply to governance fact graph | HIGH |
| **Scheduled consolidation** | Google Always-On | Cron job every N hours: consolidate episodic → semantic, prune below threshold | MEDIUM |
| **Token budget per context source** | Aider, Windsurf | Binary search for maximum relevant memory within a token budget per session startup | LOW |
| **Progressive retrieval (avoid LLM calls)** | ByteRover | 5-tier: exact match → FTS5 → vector → graph → agentic. Most queries resolve without LLM | MEDIUM |

---

## 6. Recommended Architecture for Repo Governance

### 6.1 Storage: Single SQLite Database per Project

```
gov_runtime/roles_shared/GOVERNANCE_MEMORY.db

Tables:
  memory_index     — pointer-index loaded at every session start
  episodic         — timestamped session events (who did what, when, outcome)
  semantic         — distilled facts (codebase patterns, decisions, preferences)
  procedural       — fix patterns, workflows, recipes (RGF-103 failure memory)
  embeddings       — vector embeddings for semantic search (via sqlite-vec)
  consolidation_log — tracks compaction/hygiene runs
```

FTS5 virtual tables shadow `episodic`, `semantic`, and `procedural` for keyword search. sqlite-vec column in `embeddings` for vector search. Single file, no external services, ACID-compliant, portable.

### 6.2 Indexing: Hybrid (FTS5 + Embeddings)

- **Keyword search**: SQLite FTS5 with BM25 ranking. Finds exact WP IDs, MT numbers, error messages, file paths.
- **Semantic search**: nomic-embed-text-v1.5 embeddings stored via sqlite-vec. Finds conceptually similar memories ("how did we handle storage trait purity?" matches memories about Database trait boundary work).
- **Hybrid fusion**: RRF combines both result sets. Weighted: `0.6 * vector_score + 0.4 * fts5_score` (tunable).

### 6.3 Memory Operations (Tool-Based, Provider-Agnostic)

Expose memory as governance helper commands that any model can call:

```
just memory-add <role> <wp_id> <type> "<content>"       # add episodic/semantic/procedural
just memory-search "<query>" [--scope <wp_id>] [--type <type>] [--limit N]
just memory-prime <wp_id> <mt_id>                        # load relevant context for MT
just memory-compact [--older-than 30d]                   # cron-driven compaction
just memory-stats                                        # index health, counts, staleness
```

These are shell commands wrapping Node.js scripts — any model that can call shell commands can use the memory system.

### 6.4 Session Startup Context Assembly

Following the Windsurf 6-step pattern, adapted for governance:

1. **Load governance rules** — Codex, role protocol (already done)
2. **Load memory pointer-index** — top-level summary of available memories (NEW)
3. **Load active packet + refinement** — current WP context (already done)
4. **Retrieve scoped memories** — `just memory-prime WP-{ID} MT-{N}` returns relevant episodic + semantic + procedural memories for the current MT's file scope (NEW)
5. **Load active communications** — notifications, receipts, thread (already done)
6. **Token budget trim** — binary search for maximum memory content within token budget (NEW)

### 6.5 Lifecycle: Write → Index → Retrieve → Compact

**On every governed receipt write** (already happens via `wp-receipt-append`):
- Extract memory-worthy content from the receipt
- Write to `episodic` table with timestamp, role, WP, MT, file scope, summary
- Compute and store embedding

**On session completion**:
- Extract key decisions, patterns, and outcomes
- If fix pattern: write to `procedural` table
- If new fact about codebase: write to `semantic` table

**On smoketest review completion**:
- Extract all findings into `semantic` and `procedural` memories
- These are high-value: they represent human-validated observations

**Cron: daily**:
- Re-index any governance artifacts that changed since last run
- Deduplication sweep on `semantic` table

**Cron: weekly**:
- Compact `episodic` entries older than 30 days into `semantic` summaries
- Importance-score all `semantic` entries; archive those below threshold
- Log compaction run in `consolidation_log`

### 6.6 Provider Agnosticism

The memory system is fully provider-agnostic because:
- **Storage**: SQLite (universal, zero config)
- **Indexing**: FTS5 (built into SQLite) + sqlite-vec (C extension, no dependencies)
- **Embeddings**: nomic-embed-text-v1.5 via Ollama (local) or any provider's embedding API
- **Interface**: Shell commands (`just memory-*`) — any model that can call commands can use it
- **No model-specific features**: no Claude memory API, no GPT memory API — purely external storage

If the embedding model changes, re-embed incrementally (batch job). Raw text is always stored alongside embeddings for re-embedding.

---

## 7. Implementation as RGF Items

### Phase 1: Foundation (immediate)

| ID | Item | Depends On | Effort |
|---|---|---|---|
| RGF-115 | Governance Memory SQLite Schema and CLI | RGF-101 | MEDIUM |
| RGF-116 | FTS5 Indexing of Governance Artifacts | RGF-115 | LOW |
| RGF-117 | Memory Pointer-Index for Session Startup | RGF-115 | LOW |

### Phase 2: Semantic Search (short-term)

| ID | Item | Depends On | Effort |
|---|---|---|---|
| RGF-118 | Local Embedding Pipeline (nomic-embed-text) | RGF-115 | MEDIUM |
| RGF-119 | Hybrid Search (FTS5 + sqlite-vec + RRF) | RGF-116, RGF-118 | MEDIUM |
| RGF-120 | MT-Scoped Memory Priming (`just memory-prime`) | RGF-119 | MEDIUM |

### Phase 3: Lifecycle and Hygiene (medium-term)

| ID | Item | Depends On | Effort |
|---|---|---|---|
| RGF-121 | Episodic Memory Extraction from Receipts | RGF-115 | LOW |
| RGF-122 | Procedural Memory from Smoketest Findings | RGF-115, RGF-103 | LOW |
| RGF-123 | Cron-Based Compaction and Decay | RGF-115 | MEDIUM |
| RGF-124 | Token-Budgeted Context Assembly | RGF-120 | LOW |

---

## 8. Sources

### Memory Frameworks
- [Mem0](https://github.com/mem0ai/mem0) — 52k stars, dual-storage, LLM-powered dedup
- [Letta/MemGPT](https://github.com/letta-ai/letta) — 22k stars, OS-inspired 3-tier
- [Graphiti](https://github.com/getzep/graphiti) — 25k stars, temporal knowledge graph
- [Cognee](https://github.com/topoteretes/cognee) — 15k stars, self-improving knowledge engine
- [Supermemory](https://github.com/supermemoryai/supermemory) — 21k stars, fact-based extraction
- [MemOS](https://github.com/MemTensor/MemOS) — 8k stars, skill evolution
- [Hermes Agent](https://github.com/NousResearch/hermes-agent) — 28k stars, FTS5 + skill documents
- [memU](https://github.com/NevaMind-AI/memU) — 13k stars, hierarchical 3-layer
- [OpenMemory](https://github.com/CaviraOSS/OpenMemory) — 4k stars, 5-sector cognitive
- [A-MEM](https://github.com/agiresearch/A-mem) — 1k stars, Zettelkasten-inspired
- [LangMem](https://github.com/langchain-ai/langmem) — 1k stars, procedural memory
- [SimpleMem](https://github.com/aiming-lab/SimpleMem) — 3k stars, triple-index
- [ReMe](https://github.com/agentscope-ai/ReMe) — 3k stars, file-based markdown
- [TeleMem](https://github.com/TeleAI-UAGI/telemem) — 457 stars, Mem0 drop-in
- [memclawz](https://github.com/yoniassia/memclawz) — causality graph + sleep-time reflection
- [AgentMemory](https://github.com/rohitg00/agentmemory) — triple-stream, 12 hooks, P2P mesh

### Coding-Agent Systems
- [Overstory](https://github.com/jayminwest/overstory) — SQLite WAL mail, checkpoints, agent CVs
- [BEADS](https://github.com/steveyegge/beads) — git-native SQLite + JSONL, `bd prime`, `bd compact`
- [Metaswarm](https://github.com/dsifry/metaswarm) — 18 agents, knowledge priming by file scope
- [Claude Code Memory](https://code.claude.com/docs/en/memory) — pointer-index MEMORY.md + topic files
- [Cline Memory Bank](https://cline.bot/blog/memory-bank-how-to-make-cline-an-ai-agent-that-never-forgets) — structured `.memory-bank/` directory
- [Continue.dev](https://deepwiki.com/continuedev/continue/3.4-codebase-indexing) — 4-backend indexing

### Vector Databases
- [ChromaDB](https://github.com/chroma-core/chroma) — 27k stars, SQLite-backed
- [Qdrant](https://github.com/qdrant/qdrant) — 29k stars, Rust, GPU-accelerated
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — zero-dependency vector search in SQLite

### Research Papers
- [Memory in the Age of AI Agents](https://arxiv.org/abs/2512.13564) — 47-author survey, 3-dimensional taxonomy
- [Rethinking Memory Mechanisms](https://arxiv.org/html/2602.06052v3) — substrate/cognitive/subject dimensions
- [Anatomy of Agentic Memory](https://arxiv.org/html/2602.19320v1) — benchmark saturation, latency tax analysis
- [A-MEM](https://arxiv.org/abs/2502.12110) — Zettelkasten-inspired, NeurIPS 2025
- [AgeMem](https://arxiv.org/html/2601.01885v1) — 6-tool RL-trained memory management
- [Memory-R1](https://arxiv.org/abs/2508.19828) — RL-trained memory manager, 152 training pairs
- [Zep/Graphiti](https://arxiv.org/abs/2501.13956) — temporal knowledge graph, 94.8% DMR
- [SimpleMem](https://arxiv.org/abs/2601.02553) — semantic lossless compression, 30x token reduction
- [ByteRover](https://arxiv.org/abs/2604.01599) — hierarchical context tree, zero infrastructure
- [Multi-Layered Memory Architectures](https://arxiv.org/abs/2603.29194) — 46.85% success rate, 5.1% false memory
- [MemGovern](https://arxiv.org/abs/2601.06789) — governed human experiences for code agents
- [CRAG](https://arxiv.org/abs/2401.15884) — corrective RAG with quality evaluation
- [ICLR 2026 MemAgents Workshop](https://openreview.net/pdf?id=U51WxL382H)
- [MemoryAgentBench](https://github.com/HUST-AI-HYZ/MemoryAgentBench) — ICLR 2026, 4-competency benchmark

### Embedding Models
- [nomic-embed-text-v1.5](https://ollama.com/library/nomic-embed-text) — fully open, Ollama-native, Apache 2.0
- [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2) — fastest, CPU-friendly
- [BGE-M3](https://huggingface.co/BAAI/bge-m3) — MIT, dense+sparse+multi-vector
- [Qwen3-Embedding-8B](https://huggingface.co/Qwen/Qwen3-Embedding-8B) — MTEB leader, Apache 2.0
