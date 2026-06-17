---
schema: handshake.indexed_spec.module@1
spec_version: "v02.194"
bundle_id: "master-spec-v02.194"
module_id: "06"
section_id: "6"
title: "6. Mechanical Integrations"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "4c1d4c00271e4789ef14ff0d3b45470148b37f51a042189f85a22a9f3399bac5"
body_sha256: "bd43b4369e5b3ebf8bb50e1199c30cff340ad13bd09a7decf09e556106310cab"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 6. Mechanical Integrations

### 5.5.7 Photo Stack Performance Budgets & Benchmark Scenarios

This section defines Photo Stack performance targets and benchmark inputs. GPU scheduling rules are defined under **Â§6.3.3.6.3**.

#### 5.5.7.1 Performance Budgets
| Operation | Target (4K image) | Maximum |
|-----------|------------------|---------|
| Preview (fit) | <100ms | 200ms |
| Preview (1:1 tile) | <50ms | 100ms |
| Full recipe apply | <500ms | 1000ms |
| AI mask (subject) | <1000ms | 2000ms |
| Export (JPEG) | <200ms | 500ms |
| Layer composite (10 layers) | <300ms | 600ms |
| Proxy generation | <2000ms | 5000ms |
| Vision model analysis | <3000ms | 10000ms |
| LLM response (short) | <2000ms | 5000ms |

#### 5.5.7.2 Benchmark scenario requirements (normative)
- Benchmarks MUST include: RAW decode, develop render, proxy pyramid generation, masking, export, and (if enabled) vision/LLM tagging.
- Scenarios MUST be executed with fixed engine versions and recorded determinism class; any engine version change MUST trigger re-baselining (see Â§5.4.7.3).

## 6.0 Mechanical Tool Bus & Integration Principles

This section defines how all "mechanical" tools â€“ document parsers, OCR/ASR engines, format converters, and similar subsystems â€“ plug into Handshake as part of one tool bus instead of isolated pipelines.

**Scope**

- Document ingestion and layout-aware parsing (Docling; Section 6.1).
- Audio/video transcription (ASR stack; Section 6.2).
- Calendar ingestion, sync, and event export (calendar_sync; Â§10.4).
- Generic mechanical engines (converter, sentiment, and related engines; Section 6.3).
- Fallback and legacy format handlers (Unstructured, Apache Tika, and related tools referenced in Docling assessment).

**Integration model**

1. **Single orchestrator and runtime layer.**  
   - All mechanical tools are invoked by the orchestrator through the **Model Runtime Layer** (Section 2.1.3).  
   - Tools expose deterministic APIs (CLI, HTTP, gRPC, MCP); the orchestrator turns user actions or AI Jobs into calls on these runtimes.

2. **Workspace-first data flow.**  
   - Inputs and outputs of mechanical tools are always represented as workspace entities using RawContent/DerivedContent (Sections 2.2.1â€“2.2.2). Examples:  
     - Docling converts external files into Documents, Blocks, Tables, and Assets.  
     - ASR converts audio/video Assets into transcript Documents and DerivedContent sidecars.  
     - Converter engines normalise text bodies or metadata fields for existing entities.  
   - Tools **MUST NOT** maintain private long-term stores for user data; long-lived results live in the workspace.

3. **AI Jobs and workflows, not ad-hoc pipelines.**  
   - Long-running or multi-step mechanical tasks **SHOULD** be expressed as AI Jobs and/or workflow nodes (Sections 2.5.10 and 2.6.6).  
   - Docling ingestion jobs, ASR transcription jobs, and large-scale conversions appear in Flight Recorder and Job History like any other AI operation.

4. **Tool-agnostic downstream consumption.**  
   - Once a mechanical tool has produced RawContent/DerivedContent, downstream features treat it like any other workspace content:  
     - Docs, canvases, and tables can all reference the same imported blocks or tables.  
     - Shadow Workspace indexes the results for retrieval and RAG (Section 2.3.8).  
     - Agents and workflows operate on entity IDs, not on â€œDocling-onlyâ€ or â€œASR-onlyâ€ objects.

5. **Fallback layers without new modes.**  
   - Fallback tools (e.g. Unstructured and Tika for fringe formats) plug into the same bus: they produce the same workspace structures as Docling where possible and are wrapped in the same job and logging patterns.  
   - Implementations **SHOULD** hide tool differences behind configuration and capability profiles rather than exposing multiple â€œimport modesâ€ to the user.

6. **Observability and safety.**  
   - All mechanical tool invocations **MUST** be logged in the Flight Recorder (Section 2.1.5) with: tool identity, version, inputs (by reference), outputs (by reference), and errors.  
   - Capability profiles and policies control which tools are allowed to access which resources, following the same model as LLM calls (Section 5.2, AI Job Model).

The rest of Section 6 details specific tool families (Docling, ASR stack, mechanical engines) and their concrete architectures. This subsection is normative for how they interoperate.

---

### 6.0.1 Cross-Tool Interaction Map (Normative)

Handshake includes many â€œmechanical toolsâ€ and â€œAI toolsâ€ (Docling, ASR, ACE runtime, RAG, calendar_sync, terminal tools, renderers, exporters, etc.). To avoid a pile of special-case pipelines, Handshake MUST treat cross-tool interaction as a first-class contract.

**Hard rules (integration invariants)**
1. **No shadow pipelines:** tool execution MUST occur via the Workflow Engine + AI Job Model (AÂ§2.6), not ad-hoc background threads.
2. **Artifact-first I/O:** tools MUST consume inputs by reference (workspace IDs / artifact refs) and produce outputs as Raw/Derived/Display content (AÂ§2.2), not hidden local files.
3. **Capability-gated side effects:** any filesystem/process/network side effect MUST be capability-checked (AÂ§11.1) and recorded.
4. **Flight Recorder is always-on:** every tool invocation MUST emit Flight Recorder events with tool identity/version, input refs, output refs, timing/budgets, and error codes (AÂ§2.1.5, AÂ§11.5).
5. **Local-first default:** the default posture is offline/local execution. Remote execution (cloud models, remote services) MUST be opt-in, capability-gated, and have a deterministic local fallback path.
6. **Evidence surfaces:** Operator Consoles MUST be able to show â€œwhat happenedâ€ end-to-end (Job History + Problems/Evidence) for every tool interaction (AÂ§10.5, AÂ§11.4).

**Interaction table (minimum set; expand as tools are added)**

| Tool / Surface | Primary trigger | Consumes | Produces | Required shared primitives |
|---|---|---|---|---|
| Docs editor | user edit / AI edit | Document blocks + selection | Display changes + diff artifacts | AI Job Model, consent, FR events, deterministic edit UX |
| Canvas editor | user edit / AI layout | Canvas nodes/edges | Display changes + render/export artifacts | AI Job Model, FR events, artifact refs |
| ACE runtime (Agentic Context Engineering) | any AI job requiring context | Workspace entities + scope hints | ContextPlan/ContextSnapshot (+ hashes) | budgets, determinism, validator pack, FR traces |
| Shadow Workspace + RAG | query / â€œProject Brainâ€ | indexed workspace content | QueryPlan + RetrievalTrace + citations | cache keys, drift flags, Evidence view |
| Docling ingestion | import file | external file artifact ref | structured blocks/tables/assets | workflow job, provenance, FR logs |
| ASR ingestion | import audio/video | audio asset ref | transcript docs + timing sidecars | workflow job, provenance, FR logs |
| Calendar subsystem | view or patch-set apply | CalendarEvents + ActivitySpans | patch-set artifacts + synced state | patch-set discipline, capability gating, FR logs |
| Terminal surface | user command / workflow step | command + working dir + refs | stdout/stderr artifacts + exit code | capability gates, reproducible command records |
| Monaco surface | code edit / AI refactor | file refs + diffs | patch-set artifacts + review UX | no-silent-edits, diff/accept, FR logs |
| Operator Consoles | diagnostics / evidence drilldown | Problems + Events + bundles | human-readable evidence views | DuckDB store, trace linking, bundle export |
| Debug Bundle exporter | user request / CI artifact | selected logs + refs | deterministic bundle hash + archive | artifact hashing, redaction, provenance |
| Workspace Bundle exporter | user request | workspace entities | portable export bundle | deterministic hashing, manifest, retention policy |
| Mechanical Extension (Tool Bus) engines | workflow nodes | PlannedOperation | EngineResult + artifacts | MEX envelopes, conformance gates, FR logs |
| Photo Studio | image pipeline job | RAW/asset refs | renders + exports + sidecars | render determinism, provenance, FR logs |
| Charts/Dashboards/Decks | user request / AI build | tables + data refs | chart/deck artifacts + validations | schema validators, export policy, provenance |

**MCP and remote services (local-first stance)**
- MCP MAY be used as an adapter layer for tools/services, but it MUST NOT become a required dependency for core local workflows.
- Any MCP-backed tool MUST support:
  - local-first execution when available
  - deterministic caching of remote results (artifact refs + hashes)
  - explicit consent + capability gating for network access

### 6.0.2 Unified Tool Surface Contract (Local Tool Calling + MCP) (Normative)

Handshake is local-first but exposes many internal primitives (â€œtoolsâ€) to agents, workflows, and operator surfaces. To ensure local models (on-device) and cloud models (over MCP) invoke the same capabilities safely and consistently, Handshake defines a **single canonical tool contract**.

This contract is the *single source of truth* for:
- tool identity and naming
- input/output schemas and defaults
- side-effect labeling and idempotency
- streaming/progress semantics
- versioning/deprecation
- observability (Flight Recorder) and replay semantics

Tool implementations MAY vary (in-process, local IPC, MEX engine, MCP server), but their behavior MUST be describable by this contract and MUST pass through the same gates (AÂ§11.1) and logging (AÂ§11.5).

#### 6.0.2.1 Definitions

- **Tool:** a callable, typed operation exposed to an agent/model or workflow step.
- **Tool registry:** the canonical list of tool definitions + metadata. The registry is authoritative; tool metadata received from untrusted remotes is advisory only.
- **Tool Gate:** a deterministic interceptor that enforces capabilities, consent/approvals, budgets, and logging for every tool invocation.
- **Transport:** how a tool is invoked (local IPC, in-process, MEX engine invocation, MCP).

#### 6.0.2.2 Tool identity, naming, and versioning (MUST)

Each tool MUST have a stable identity:

- `tool_id` (string): lowercase dot-separated identifier matching:
  - regex: `^[a-z0-9_]+(\.[a-z0-9_]+)+$`
  - examples: `workspace.entity.get`, `stage.jobs.enqueue`, `photo.search`, `context.search`, `engine.version.status`
- `tool_version` (string): semantic version `MAJOR.MINOR.PATCH`

Rules:
1. **Stability:** `tool_id` MUST be stable across releases.
2. **Breaking changes:** any breaking change to input/output schema MUST bump `MAJOR`.
3. **Deprecation:** deprecated tools MUST declare:
   - `deprecated_since` (semver)
   - `sunset_on` (ISO date)
   - `replaced_by` (optional tool_id)
4. **No silent behavior drift:** side-effect classification and required capabilities MUST NOT change without a MAJOR bump.

#### 6.0.2.3 Side effects, idempotency, determinism (MUST)

Each tool MUST declare:

- `side_effect`: one of `READ | WRITE | EXECUTE`
  - **READ:** no persistent mutation and no external side effects
  - **WRITE:** mutates workspace state (documents, canvases, tables, metadata, etc.)
  - **EXECUTE:** triggers external side effects (filesystem writes outside workspace store, processes, network, remote APIs)

- `idempotency`: one of `IDEMPOTENT | IDEMPOTENT_WITH_KEY | NON_IDEMPOTENT`
  - `IDEMPOTENT_WITH_KEY` tools MUST accept `idempotency_key` and dedupe retries.

- `determinism`: one of `DETERMINISTIC | BEST_EFFORT | NON_DETERMINISTIC`
  - tools MUST document sources of nondeterminism (e.g., remote calls, timestamps).

- `availability`: one of `OFFLINE_OK | REQUIRES_NETWORK | BEST_EFFORT_OFFLINE`

**Routing rule (MUST):**
- All `WRITE` and `EXECUTE` tools MUST execute via the AI Job Model (AÂ§2.6). Inline execution is permitted only for bounded, synchronous READ tools.

#### 6.0.2.4 Schemas and examples (MUST)

Tool inputs and outputs MUST be defined as JSON Schema (draft 2020-12).

- Input schemas MUST declare required vs optional fields and default values.
- Output schemas MUST define a minimal structured result suitable for model consumption.
- Tools SHOULD support *projection/filtering* to reduce context bloat:
  - either via an explicit `select` / `projection` argument, or
  - by returning a small structured summary plus a `result_ref` to the full payload.

Tools MAY include `examples[]` (input/output pairs) to demonstrate correct usage for complex parameter sets.

#### 6.0.2.5 Canonical invocation envelope (HTC-1.0) (MUST)

All tool invocations MUST be representable as the following canonical envelope, even when transported over MCP or internal buses.

**Request envelope**
```json
{
  "schema_version": "htc-1.0",
  "tool_call_id": "uuid",
  "trace_id": "uuid",
  "session_id": "optional (REQUIRED for ModelSession tool calls; set to ModelSession.session_id)",
  "actor": {
    "kind": "human|agent|system",
    "agent_id": "optional",
    "model_id": "optional"
  },
  "tool_id": "photo.search",
  "tool_version": "1.2.0",
  "args": {},
  "args_ref": "artifact://... (optional; REQUIRED if args exceed 32KB or contain secrets)",
  "idempotency_key": "optional",
  "dry_run": false
}
```

**Normative (ModelSession correlation):** When a tool call is executed within a `ModelSession` (Â§4.3.9.12), `session_id` MUST be present and MUST equal `ModelSession.session_id` (i.e., the model_session_id). `stage_session_id` / terminal session identifiers MUST NOT be substituted here.

**Normative (session-scoped capability intersection):** When `session_id` is present, Tool Gate MUST evaluate capabilities against the session-scoped effective grants/tokens for that session (deny-by-default), and MUST intersect them with any global/operator capability constraints. A toolâ€™s `required_capabilities` MUST be satisfied by this intersection; otherwise the call MUST be denied or escalated for approval.


**Response envelope**
```json
{
  "schema_version": "htc-1.0",
  "tool_call_id": "uuid",
  "trace_id": "uuid",
  "ok": true,
  "result": {},
  "result_ref": "artifact://... (optional; REQUIRED if result exceeds 32KB)",
  "error": null,
  "timing": { "started_at": "iso8601", "ended_at": "iso8601", "duration_ms": 123 },
  "resources": { "workspace_ids": [], "artifacts": [], "files": [], "urls": [] }
}
```

**Standard error object (when `ok=false`)**
```json
{
  "code": "string",
  "kind": "validation|auth|capability|timeout|tool|transport|canceled|policy",
  "message": "human-readable",
  "retryable": true,
  "details": {}
}
```

**Payload sizing rule (MUST):**
- `args` and `result` MUST be <= 32KB each (post-JSON encoding). Larger payloads MUST use `args_ref` / `result_ref`.


#### 6.0.2.5.1 HTC-1.0 JSON Schema file (SSoT) (MUST)

Handshake MUST define the HTC-1.0 envelope as a single JSON Schema file checked into the repository:

- `assets/schemas/htc_v1.json` (JSON Schema draft 2020-12)

Rules:
1. **SSoT:** `htc_v1.json` is the single source of truth for the request/response/error envelopes of `schema_version: "htc-1.0"`.
2. **Runtime validation (local + MCP):** the Tool Gate MUST validate every tool call envelope against `htc_v1.json`:
   - Local tool calling (IPC / in-process / MEX adapters) MUST validate the envelope **before execution** and MUST validate the response envelope **before return**.
   - MCP transports MUST validate at the Rust Gate boundary **before forwarding** `tools/call` and **before accepting** a tool response.
3. **Failure behavior:** if envelope validation fails, the call MUST be rejected with:
   - `ok=false`
   - `error.kind="validation"`
   - `error.code="VAL-HTC-001"`
4. **Versioning:** any breaking change to the envelope MUST bump `schema_version` (e.g., `htc-2.0`) and introduce a new schema file `assets/schemas/htc_v2.json`. Non-breaking clarifications MAY update `htc_v1.json` without changing `schema_version`.

#### 6.0.2.6 Streaming and progress (SHOULD)

Tools that can exceed ~1s runtime SHOULD provide progress events keyed by `tool_call_id`.

- Local transports SHOULD support a progress/event stream.
- MCP transports SHOULD use `notifications/progress` where supported.
- Progress events MUST be logged (at least as span annotations) so Flight Recorder can reconstruct â€œwhat happenedâ€.

#### 6.0.2.7 Tool discovery and progressive disclosure (SHOULD)

To avoid loading hundreds of tool definitions into context, Handshake SHOULD support progressive disclosure:

- Provide built-in tools:
  - `handshake.tools.search(query, detail_level)` â†’ returns matching tool definitions (IDs + minimal schemas by default)
  - `handshake.tools.get(tool_id)` â†’ returns full tool definition

- Tools MAY be marked `deferred: true`:
  - deferred tools MUST NOT be included in default manifests shown to models
  - deferred tools MUST still be discoverable via `handshake.tools.search`

#### 6.0.2.8 Programmatic tool calling (â€œCode Modeâ€) (OPTIONAL; RECOMMENDED)

For complex multi-step workflows, Handshake MAY enable **programmatic tool calling**: the model writes code in a sandbox that calls tools via a typed SDK. Intermediate tool results remain inside the sandbox, and only minimal summaries (or artifact refs) are returned to the model.

Normative requirements:
1. Code runs ONLY inside `engine.sandbox` (AÂ§6.3), with explicit capability scoping.
2. The sandbox SDK MUST route every tool call through Tool Gate (capability checks + Flight Recorder).
3. All sandbox-internal tool calls MUST be correlated to the parent sandbox run via `trace_id` and parent span IDs.
4. The sandbox MUST support deterministic replay where declared `determinism=DETERMINISTIC`, and MUST clearly label best-effort/non-deterministic runs in logs.

This mode exists to reduce token/context bloat and to make multi-call workflows more deterministic.

#### 6.0.2.9 Conformance tests (MUST)

All tool implementations (local or MCP-backed) MUST pass conformance checks:
- input/output schema validation
- side_effect classification verification (policy tests)
- enforced payload size limits (32KB rule)
- capability gating (deny-by-default)
- FR event emission (see FR-EVT-007 ToolCallEvent, AÂ§11.5)
- idempotency behavior for retryable calls

#### 6.0.2.10 Runtime Visibility Contract (MUST)

[ADD v02.142] Handshake MUST NOT leave runtime-callable capability surfaces implicit. If a feature or capability can be invoked by local models, cloud models, workflows, mechanical engines, or operator surfaces, its runtime posture MUST be visible in Appendix 12.

Minimum requirements:
1. Appendix 12.3 FEATURE_REGISTRY MUST record the owning feature and one or more capability slices whenever the feature spans materially different runtime paths or force-multiplier roles.
2. Each runtime-visible capability slice MUST declare, directly or via linked runtime visibility rows:
   - job/workflow embodiment (AI job, workflow node, mechanical tool, UI action, or equivalent),
   - tool-surface exposure (Unified Tool Surface, MCP, Command Center, UI-only, or NONE),
   - model exposure (local, cloud, both, or operator-only),
   - Command Center visibility,
   - Flight Recorder evidence path,
   - Locus visibility/correlation path, and
   - storage posture ([SQLite now / PostgreSQL ready] or stricter).
3. Appendix 12.6 INTERACTION_MATRIX MUST link high-ROI force multipliers to the relevant runtime visibility rows.
4. When a runtime-visible feature is materially changed, its UI guidance row in Appendix 12.5 MUST be kept current.

[ADD v02.142] Ordering is mandatory: Main Body first, then Appendix 12, then Roadmap phases, then stub/task-board expansion. Runtime visibility growth MUST patch canonical sections in place; addendum-style normative text is forbidden.

[ADD v02.142] Initial seeding SHALL cover at least: Calendar temporal correlation, Calendar orchestrated mutation, unified local/cloud governed tool calling, Locus execution correlation, Loom retrieval library, and Stage capture/import pipeline.

[ADD v02.165] Runtime-callable tool infrastructure MUST publish governed status for each local tool, mechanical engine, Model Context Protocol server, and remote adapter that can serve the current workspace. The status model MUST include transport kind, health state, permission scope, route policy, fallback policy, last verification timestamp, and last failure reason, and it MUST remain projectable into Dev Command Center before execution, reroute, or replay decisions occur.

#### 6.0.2.11 Primitive Index Coverage Contract (MUST)

[ADD v02.143] Appendix 12.4 is the mandatory coverage ledger for how Handshake features decompose into concrete primitives, tools, technologies, and runtime-visible surfaces. It MUST be maintained during every refinement/spec-enrichment pass even when the Main Body already clearly covers the feature.

Minimum requirements:

1. Every feature in Appendix 12.3 MUST have exactly one Appendix 12.4 feature coverage row.
2. Every Appendix 12.4 feature row MUST use arrays for `primitive_ids`, `tool_ids`, `technology_ids`, `coverage_refs`, and `gap_stub_ids`; fake scalar or object stand-ins are forbidden.
3. Every Appendix 12.4 feature row MUST declare `coverage_status` using one of: `SEEDED`, `PARTIAL`, or `GAP`.
4. Every Appendix 12.4 feature row MUST include `coverage_refs` that point to the current repo/spec surfaces used to justify the row.
5. Every Appendix 12.4 feature row with `coverage_status` other than `SEEDED` MUST list one or more `gap_stub_ids` that already exist in Task Board Stub Backlog before the spec version is considered complete.
6. Newly discovered high-ROI combinations or missing runtime embodiments found while updating Appendix 12.4 MUST be resolved immediately by either Appendix 12.6 scaffolding or detailed stub creation; silent postponement is forbidden.
7. Ordering is mandatory: Main Body first, then Appendix 12 feature/index/matrix updates, then Roadmap phase updates using the fixed phase fields, then Task Board / Build Order / stub backlog synchronization.

[ADD v02.143] Initial coverage seeding SHALL include runtime/job/tool/frontend/operator primitives for AI Job Model, Workflow Engine, MCP / Tool Gate, Calendar, Locus, Loom, AI-Ready / retrieval, Spec Router prompt packs, Stage-adjacent media flows, and the current operator/runtime visibility surfaces.


[ADD v02.144] Second-pass coverage SHALL promote explicitly named feature families and runtime-adjacent UI shells when they carry future execution value, including Canvas, Docs & Sheets, Project Brain, Thinking Pipeline, Context Packs, Semantic Catalog, Skill Bank, ASR, Charts & Dashboards, Presentations / Decks, Studio, and Mail / DCC runtime projection.

[ADD v02.144] If second-pass coverage discovers missing runtime embodiment, visibility, or tool-call posture, the same spec drop MUST either seed Appendix 12 immediately or create stub-backed gap tracking in Appendix 12.4 and Task Board before the version is considered complete; silent deferment is forbidden.

[ADD v02.145] Third-pass coverage SHALL prioritize runtime entrypoints, reusable data contracts, operator/export/filter surfaces, and execution-path force multipliers over generic noun harvesting. New Appendix 12 rows added during this pass MUST be concrete enough to guide deterministic local/cloud tool selection, runtime tracing, and operator review.

[ADD v02.145] When a feature already exists in Appendix 12, third-pass coverage MUST deepen typed session, provider, JSON-RPC, projection, consent, export, diagnostics, and filter contracts before inventing duplicates. Model session orchestration, cloud escalation consent, MEX runtime, and their Command Center / Flight Recorder / Locus visibility MUST remain explicit.

[ADD v02.146] Deepening passes SHALL prioritize under-modeled seeded rows before widening to new families. When concrete editor, export, query, event, or retrieval-artifact contracts already exist in code or normative text, Appendix 12 MUST attach them to their owning feature rather than leaving them as orphan types.

[ADD v02.146] Deepening coverage SHALL include first-class UI/operator rows for always-on runtime surfaces such as AI Job drawers and Flight Recorder timelines, and SHALL add explicit interaction edges where job-state consent flows or mechanical execution already emit visible telemetry.
[ADD v02.147] High-signal orphan primitives discovered during refinement or spec-enrichment passes MUST be resolved before PASS: attach them to owning feature rows in Appendix 12.4, or create/list a detailed stub that tracks the ownership gap. Silent carry-forward is forbidden.
[ADD v02.147] Deepening passes SHALL prefer request/response, filter/query, manifest/export, policy/consent, projection, and recovery contracts before widening to new feature families, and SHALL record temporal/operator force-multiplier ideas as matrix edges or stubs instead of leaving them implicit.

[ADD v02.148] Ownership-reduction passes SHALL classify resolved high-signal primitives as ATTACHED, SHARED, STUBBED, or INTENTIONALLY_UNOWNED in the appendix-maintenance workflow; runtime/session/export/recovery contracts MUST NOT remain effectively ownerless without explicit rationale.

[ADD v02.148] Code-backed Stage/session/auth/export/recovery contracts discovered during repo scans SHALL be attached before prose-only abstraction proposals so local/cloud models inherit deterministic ownership from actual runtime surfaces.

[ADD v02.149] Appendix 12 maintenance is reciprocal. If a spec-enrichment or refinement updates Appendix 12.3/12.4/12.5/12.6 ownership, interaction, runtime-visibility, or GUI-guidance rows, the same spec version SHALL patch the governing Main Body section in place first so the canonical law explains the new feature growth. Appendix-only normative growth is forbidden.

[ADD v02.149] Normative Main Body additions that materially change capability ownership, runtime posture, interaction edges, or GUI behavior SHALL update Appendix 12 in the same spec version. The required order is: Main Body first, then Appendix 12, then Section 7.6 / 7.6.1, then stubs / Task Board / Build Order sync.

[ADD v02.149] Refinement SHALL actively search for high-ROI matrix additions that force feature growth. Every candidate discovered in refinement MUST resolve as `IN_THIS_WP`, `NEW_STUB`, `SPEC_UPDATE_NOW`, `REJECT_LOW_ROI`, or `REJECT_DUPLICATE`; silent omission is forbidden.

[ADD v02.149] End-of-file Appendix 12 blocks SHALL remain machine-readable JSON and SHALL carry the current spec version in their `spec_version` field. Patch-in-place growth is mandatory; addendum-style normative text is forbidden.

[ADD v02.149] Main Body changes that affect delivery ordering, implementation decomposition, or roadmap coverage SHALL update Section 7.6 and Section 7.6.1 in the same spec version using only the fixed phase fields: Goal, MUST deliver, Key risks addressed in Phase n, Acceptance criteria, Explicitly OUT of scope, Mechanical Track, Atelier Track, Distillation Track, and Vertical slice.

[ADD v02.149] New roadmap/spec entries that are not absorbed immediately into the current execution scope SHALL create detailed stub packets and Task Board entries in the same governance pass.

[ADD v02.149] Refinements and task packets SHALL display the target `[ADD v<version>]` marker and SHALL list primitives exposed and primitives created so later local/cloud models and operator tooling can reason about surface availability deterministically.

[ADD v02.149] The refinement workflow SHALL include two separate mandatory rubrics:
- `MATRIX_RESEARCH_RUBRIC`: research-backed combination discovery across vendor docs/papers, university or lab work, official design systems, and high-signal GitHub repos. Each useful row SHALL record `ADOPT`, `ADAPT`, or `REJECT`, the engineering trick carried into Handshake, the runtime/DCC/Flight Recorder/Locus/storage consequences, and the final resolution.
- `GUI_IMPLEMENTATION_ADVICE_RUBRIC`: research-backed GUI implementation advice that captures hidden requirements, interaction contracts, accessibility/keyboard behavior, tooltip-vs-inline strategy, and the engineering trick carried into Handshake. Hidden semantics may not remain tribal knowledge.

[ADD v02.150] Phase 1 matrix growth MUST bias toward backend execution, observability, export, recovery, sync, and portability combinations before frontend-led breadth expansion. UI and GUI consequences still MUST be recorded, but backend/runtime contracts take precedence.

[ADD v02.150] When Appendix 12.6 adds or deepens backend-heavy matrix edges, the owning Main Body sections for workflow execution, jobs, consent, projection, storage/export, calendar, and stage/media portability MUST be patched in place in the same spec version.

[ADD v02.150] Backend-heavy matrix passes SHOULD preferentially ADOPT or ADAPT durable execution, event-correlation, policy-decision logging, incremental-sync, and asset-lineage patterns discovered in the `MATRIX_RESEARCH_RUBRIC` instead of inventing isolated Handshake-only semantics.

[ADD v02.150] New backend combo discoveries that are not absorbed into current scope MUST create detailed stub work packets and matching `TASK_BOARD` / `BUILD_ORDER` entries in the same refinement pass.

[ADD v02.151] Backend-heavy matrix expansion passes SHALL deepen export, evidence, and storage-portability seams before inventing weaker direct cross-surface edges. Role Mailbox, AI-Ready Data, Workflow Engine, Flight Recorder, Debug Bundle, and Storage Portability contracts discovered in code MUST be promoted into Appendix 12.3 / 12.4 / 12.6 or resolved as stub-backed bridge work in the same spec version.

[ADD v02.151] When backend evidence sources emit Flight Recorder events, produce portable export manifests, or persist durable artifacts and indexes, the same spec version MUST patch the owning Main Body sections and record the corresponding Appendix 12.6 edges. Role Mailbox, AI-Ready Data, and Workflow Engine MAY NOT remain appendix-only portability or evidence concepts.

[ADD v02.151] Unresolved backend bridges between export/evidence surfaces SHALL be materialized as detailed stubs instead of weak direct matrix edges. In Phase 1 this explicitly includes Role Mailbox debug-bundle bridging, AI-Ready index evidence export, and Calendar-to-Mailbox correlation until dedicated backend contracts exist.

[ADD v02.152] Backend-heavy matrix expansion passes SHALL next prioritize orchestration-to-projection-to-replay seams where backend artifacts, envelopes, or state transitions already exist in code. In Phase 1 this explicitly includes Spec Router prompt/decision artifacts, Locus execution projections, and MCP/MEX redacted tool-call evidence before weaker surface-only combos.

[ADD v02.152] When Spec Router, Locus, MCP Gate, or MEX Runtime persist backend artifacts, emit recorder events, or materialize redacted payload/result evidence, the same spec version MUST patch the owning Main Body sections and record the corresponding Appendix 12.6 edges. These seams MAY NOT remain hidden inside workflow, gate, exporter, or sync helper implementations.

[ADD v02.152] Unresolved backend bridges discovered in this pass SHALL be materialized as detailed stubs instead of weak direct matrix edges. In Phase 1 this explicitly includes Spec Router evidence portability, Locus debug-bundle bridging, and MCP/MEX evidence export until dedicated bundle-scope and replay contracts exist.

[ADD v02.153] Backend-heavy matrix expansion passes SHALL next prioritize capability enforcement, recorder correlation, diagnostics materialization, and consent artifact portability where those seams already exist in code. In Phase 1 this explicitly includes workflow capability checks, spec-router capability snapshots, MCP recorder events, and diagnostics-to-bundle export paths before weaker UI-led combos.

[ADD v02.153] When capability enforcement, consent receipts, MCP tool invocations, or diagnostics payloads already emit recorder events, produce export payloads, or persist durable JSON/artifact handles, the same spec version MUST patch the owning Main Body sections and record the corresponding Appendix 12.6 edges. These seams MAY NOT remain implicit inside workflow helpers, MCP FR adapters, or bundle exporters.

[ADD v02.153] Unresolved backend bridges discovered in this pass SHALL be materialized as detailed stubs instead of weak direct matrix edges. In Phase 1 this explicitly includes cloud-consent evidence portability until explicit manifest/hash/retention contracts exist across consent receipts and cloud-escalation request artifacts.

[ADD v02.154] When the Main Body defines a concrete backend export surface, the same spec version MUST backfill an owning Appendix 12.3 feature row and Appendix 12.4 coverage row even if implementation is still partial or stub-backed. In Phase 1 this explicitly covers Governance Pack export (Sections 7.5.4.8-7.5.4.10) and Workspace Bundle export (Section 10.5.7).

[ADD v02.154] Governance Pack export MUST stay explicit as a workflow-run, capability-gated, Flight Recorder-visible, artifact-materializing backend surface. Export record, export target, and manifest-hash semantics MAY NOT remain buried only in workflow or exporter helpers.

[ADD v02.154] Workspace Bundle export is a normative backend transfer and backup surface with capability gates, manifest hashing, and recorder visibility. Until implementation catches up, Appendix coverage MUST remain stub-backed rather than omitted from the feature registry or interaction backlog.

[ADD v02.155] Calendar-centered backend passes SHALL prioritize durable sync-state, capability-gated mutation, job-boundary policy selection, and bounded time-window export semantics before weaker UI-led calendar breadth. Source sync state, write policy, export mode, capability profile selection, and scope-hint routing MUST remain backend-first contracts.

[ADD v02.155] When Calendar backend contracts shape AI-job mutation discipline, ACE / Spec Router scope hints, or storage-portability guarantees, the same spec version MUST patch the Calendar Main Body sections in place and record the corresponding Appendix 12.3 / 12.4 / 12.6 ownership. These seams MAY NOT remain hidden only in storage structs, validator notes, or view-layer prose.

[ADD v02.155] Calendar bridges to Role Mailbox, richer Locus joins, and debug-bundle materialization SHALL remain stub-backed until explicit backend join or bundle-scope contracts exist. Calendar passes MUST prefer direct code-backed edges for Storage Portability, Capabilities & Consent, AI Job mutation discipline, and Spec Router / ACE routing over speculative surface-only links.

[ADD v02.156] Knowledge/retrieval pillar passes SHALL prioritize backend ownership of hybrid retrieval substrates, deterministic routing registries, portable graph-backed library artifacts, and reusable retrieval compactions before adding broader UI-level retrieval surfaces. Project Brain, AI-Ready Data, Context Packs, Semantic Catalog, and Loom MUST remain machine-readable backend contracts, not narrative feature promises.

[ADD v02.156] When retrieval-notebook, catalog-routing, graph-library, or retrieval-compaction contracts add matrix edges, the same spec version MUST patch the corresponding Main Body sections in place and update Appendix 12.3 / 12.4 / 12.6 ownership. QueryPlan, RetrievalTrace, ContextPack payloads, Loom block-edge records, and deterministic catalog entries MAY NOT remain implicit only in runtime code or historical stubs.

[ADD v02.156] Knowledge-plane bridges that already have strong code or Main Body grounding MUST be promoted to direct backend edges first: Project Brain -> AI-Ready Data, Semantic Catalog -> Spec Router, Context Packs -> Storage Portability, and Loom -> Storage Portability. Weaker graph-to-notebook or export-driven combinations SHALL remain stub-backed until explicit backend contracts land.

[ADD v02.157] Distillation/context/spec-router backend passes SHALL treat Skill Bank, Context Packs, Spec Router, ACE Runtime, and Micro-Task Executor as one backend learning substrate. Teacher/student lineage, reusable context compaction, prompt-envelope reuse, checkpoint/eval gating, and export-controlled adapter artifacts MUST remain explicit in Main Body law and Appendix 12 ownership.

[ADD v02.157] Late-stage local adaptation remains stub-backed in Phase 1. LoRA / QLoRA / DoRA / adapter promotion MAY only be promoted in Appendix 12 when checkpoint lineage, eval gates, export controls, capability gates, and rollback posture remain explicit; speculative training automation without those contracts is forbidden.

[ADD v02.157] Distillation passes MUST prefer cross-tokenizer-safe text snapshot storage, tokenizer metadata, context-pack hashes, prompt-envelope hashes, and recorder-visible candidate queues over hidden in-memory training assumptions.

[ADD v02.157] Unresolved recorder visibility for Context Pack build/select/refresh flows SHALL materialize as a dedicated stub work packet rather than remaining implicit in ACE, Spec Router, or workflow helpers.

[ADD v02.158] Stage/Studio/Media/ASR backend passes SHALL prioritize artifact lineage, media probe metadata, transcript portability, and recorder-visible failure/progress semantics before widening Studio or Lens UI claims. Stage capture/import, Media Downloader materialization, and ASR transcript generation MUST remain backend-first contracts.

[ADD v02.158] When Stage, Media Downloader, or ASR contracts add new artifact portability or recorder-correlation posture, the same spec version MUST patch Sections 6.2, 10.13, 10.14, and Appendix 12 in place. Transcript, media-source, and capture-session lineage MAY NOT remain implicit only in workflow helpers, media tooling defaults, or UI copy.

[ADD v02.158] Stage -> ASR and Lens/Studio transcript-time-span bridges SHALL remain stub-backed until explicit backend lineage and job-identity contracts exist. Direct matrix growth in this pass MUST prefer ASR -> Flight Recorder, ASR -> Storage Portability, Media Downloader -> ASR, and Stage -> Storage Portability over weaker surface-only pairings.

[ADD v02.159] Backend correlation and projection passes SHALL prioritize explicit joins among Dev Command Center, Operator Consoles, Flight Recorder, Debug Bundle, Locus Work Tracking, Role Mailbox, and bounded Calendar export anchors. Direct matrix growth in this pass MUST prefer code-backed recorder, query, export, and projection seams over UI-only adjacency.

[ADD v02.159] Dev Command Center is the canonical control and projection umbrella. Operator Consoles are the specialized evidence and diagnostics surfaces within that umbrella. When the same backend seam touches both, the Main Body and Appendix MUST assign control/projection ownership to Dev Command Center and evidence/drilldown ownership to Operator Consoles instead of duplicating both roles.

[ADD v02.159] Newly added Main Body, Appendix, Roadmap, refinement, and task-packet text MUST use the full feature or subsystem name on first mention and SHOULD prefer full names over unexplained abbreviations throughout. If a short form is later reused, the owning full name MUST remain machine-readable in Appendix 12.

[ADD v02.160] Dev Command Center control-plane passes SHALL prioritize explicit joins among Dev Command Center, Workflow Engine, Artificial Intelligence Job Model, Capabilities and Consent, Model Session Orchestration, and authoritative work packet, worktree, and session binding artifacts. Direct matrix growth in this pass MUST prefer code-backed workflow-run, model-session, capability-snapshot, and approval-decision seams over console-only summaries.

[ADD v02.160] Dev Command Center control-plane state MUST be backed by durable backend artifacts such as workflow runs, workflow node executions, model sessions, session registry snapshots, capability snapshots, governance decisions, and work packet bindings. Control-plane additions MAY NOT depend on user-interface-only caches, temporary drawer state, or reconstructed narrative summaries.

[ADD v02.160] When a Dev Command Center control-plane seam is only partially implemented, this spec version SHALL reuse the existing Dev Command Center, workflow projection correlation, consent audit projection, and Model Session scheduler backlog before creating duplicate stub families.

[ADD v02.161] Dev Command Center evidence-and-replay passes SHALL prioritize explicit joins among Dev Command Center, Flight Recorder, Debug Bundle, Governance Pack, Workspace Bundle, Diagnostics Schema, and workflow-linked evidence packaging. Direct matrix growth in this pass MUST prefer code-backed export records, manifest hashes, diagnostics queries, and validation outcomes over console-only polling state.

[ADD v02.161] Dev Command Center evidence-and-replay state MUST be backed by durable backend artifacts such as governance export requests and outcomes, bundle export requests and responses, bundle status and manifest identifiers, diagnostics queries, export records, and recorder-linked workflow evidence. Evidence or replay additions MAY NOT depend on drawer-local progress polling, transient toast state, or user-interface-only status caches.

[ADD v02.161] When a Dev Command Center evidence-and-replay seam is only partially implemented, this spec version SHALL reuse the existing Governance Pack, Workspace Bundle, and Diagnostics-to-Debug-Bundle backlog before creating duplicate stub families.

[ADD v02.162] Dev Command Center work-orchestration passes SHALL prioritize explicit joins among Dev Command Center, Workflow Engine, Locus Work Tracking, Model Session Orchestration, Micro-Task Executor, Work Packets, Task Board authority, and parallel session occupancy. Direct matrix growth in this pass MUST prefer code-backed tracked work packet status, ready-query results, micro-task summaries, session snapshots, and task-board sync state over kanban-only summaries or manual board edits.

[ADD v02.163] Planning-and-coordination passes SHALL treat Task Board and Work Packet System as first-class backend coordination features. Main Body changes that add or refine task-board authority, work-packet activation, or parallel-session planning semantics MUST patch Appendix 12 feature ownership, coverage rows, and interaction edges in the same version instead of leaving those joins implicit.

[ADD v02.166] Collaboration-substrate passes SHALL prefer canonical structured records for Work Packet, Micro-Task, Task Board, and Role Mailbox state over Markdown-only authority. Human-readable Markdown may remain as a mirror or note sidecar, but routing, filtering, replay, and local-small-model execution MUST remain possible without prose parsing alone.

[ADD v02.166] When the same collaboration artifact exists as both a structured record and a rendered Markdown view, the structured record SHALL be authoritative for field values, automation, and Work Packet or Micro-Task routing. The Markdown rendering remains a readable projection and MUST NOT silently diverge.

[ADD v02.167] The canonical file standard for Work Packet, Micro-Task, Task Board, and Role Mailbox collaboration artifacts SHALL be versioned JavaScript Object Notation records or JavaScript Object Notation Lines streams with explicit schema identifiers and schema versions. Human-readable Markdown MAY remain as a mirror or note sidecar, but it SHALL NOT be the only authoritative machine-readable source.

[ADD v02.167] Structured collaboration artifacts SHALL keep a project-agnostic base envelope and move repository-specific or domain-specific fields into profile extensions. Future coding, research, worldbuilding, design, and other project kernels MUST be able to reuse the same artifact family without inheriting repository-only required fields.

[ADD v02.163] Dev Command Center planning-and-coordination projection MUST preserve stable task_board_id, work_packet_id, workflow_run_id, micro_task_id, and model_session_id lineage from authoritative backend artifacts. It MUST NOT reconstruct planning state solely from kanban ordering, drawer-local caches, or ad hoc packet parsing.

[ADD v02.162] Dev Command Center work-orchestration state MUST be backed by durable backend artifacts such as tracked work packet records, task-board status, ready-query and get-status results, micro-task summaries, active session identifiers, workflow run identifiers, and session registry snapshots. Work packet steering or micro-task routing MAY NOT depend on drawer-local queue state or inferred parallel-model occupancy.

[ADD v02.162] When a Dev Command Center work-orchestration seam is only partially implemented, this spec version SHALL reuse the existing Dev Command Center, Locus occupancy, workflow projection correlation, multi-session orchestration, and micro-task summary backlog before creating duplicate stub families.

## 6.1 Document Ingestion: Docling Subsystem

**Why**  
Handshake needs to ingest documents from various formats (PDF, DOCX, PPTX, etc.) and convert them into structured blocks for editing and AI processing. Docling provides MIT-licensed, layout-aware document understanding.

**What**  
Integrates IBM Docling as the primary document processor; covers media support, licensing, architecture, alternatives, performance, and RAG enhancement. This section consolidates three research artefacts on Docling.

**Jargon**  
- **Docling**: IBM's MIT-licensed document understanding library, now part of LF AI & Data foundation.
- **DoclingDocument**: Hierarchical JSON schema representing parsed document structure.
- **TableFormer**: Docling's specialized table extraction model.
- **DocLayNet**: Deep learning model for page layout segmentation.
- **VLM (Vision Language Model)**: Models that understand both images and text (Granite-Docling, SmolDocling).

---
This document bundles and preserves three related research artefacts on Docling and its integration into the Handshake workspace:

- Part I: GPT-generated Docling evaluation and integration research
- Part II: Spec-style Docling integration assessment for Handshake
- Part III: Architectural evaluation essay for Docling in Handshake

All original tables, ASCII diagrams, and schemas are preserved as-is; only heading levels were adjusted for nesting.

---

### 6.1.1 Part I â€“ Docling Evaluation for Project Handshake (GPT research v1)

### 6.1.2 Docling Evaluation for Project Handshake

**Date:** 2025-12-13  
**Target project:** Handshake (local-first Tauri + Rust + React desktop workspace)

---

#### 6.1.2.1 Executive Summary

Docling is a strong fit as the primary on-device document preprocessor for Handshake. It is MIT-licensed, actively maintained by IBM and the Docling community, and now part of the LF AI & Data foundation, which makes it suitable for commercial, closed-source desktop distribution.

Technically, Docling covers the core formats Handshake cares about (PDF, DOCX, PPTX, XLSX, HTML, Markdown, EPUB, images, CSV/JSON/XML, ZIP), with advanced PDF layout and table recovery via DocLayNet and TableFormer, OCR through multiple engines, optional ASR for audio, and VLM pipelines using Granite-Docling and related models.

However, Docling does **not** aim for â€œeverything under the sunâ€ format coverage (no native email formats, no legacy Office, no direct video container support), so it should be paired with Unstructured and/or Apache Tika for fringe formats and email ingestion.

For Handshake's architecture [UPDATED v02.191], the best integration path is a **Handshake-managed Docling worker** (Docling/Docling-Serve or a thin custom worker) owned by the Model Runtime / Workflow Engine lifecycle. The worker may communicate with Rust via in-process calls, subprocess IPC, HTTP, gRPC, or a product-owned job queue, but Handshake owns startup, health checks, ports, logs, teardown, recovery, and capability policy. The Operator must not have to manually launch Docling, Docker, Conda, or any outside support app for core document ingestion. Outputs are converted into Tiptap/Yjs blocks and fed into the CRDT engine as AI-authored ops. This keeps the Rust/Tauri shell lean, isolates Python/ML dependencies, and aligns with the project's "AI as CRDT participant" model without turning Docling into an external app dependency.

---

#### 6.1.2.2 Media Type Support Matrix

Support levels:

- âœ… Fully supported / first-class in current Docling release  
- âš ï¸ Indirect / requires extra configuration, conversion, or is not clearly documented  
- âŒ Not supported (or no credible evidence of support)  
- ðŸ—“ï¸ On roadmap / planned (when clearly stated)

All Docling support statements are based primarily on official â€œSupported formatsâ€ docs, feature descriptions, and examples.

##### 6.1.2.2.1 Document Formats

| Format            | Extension          | Support Level | Notes |
|------------------|--------------------|--------------|-------|
| PDF (native)     | `.pdf`             | âœ…            | Primary target; deep layout + table understanding (DocLayNet + TableFormer). |
| PDF (scanned)    | `.pdf`             | âœ…            | Supported via OCR; multiple OCR backends + â€œforce full page OCRâ€ pipeline. |
| Word (modern)    | `.docx`            | âœ…            | Explicitly listed as supported input; structure preserved into DoclingDocument. |
| PowerPoint       | `.pptx`            | âœ…            | Supported; slides converted into blocks and images. |
| Excel            | `.xlsx`            | âœ…            | Supported; sheets/tables extracted into structured table elements. |
| Legacy Word      | `.doc`             | âŒ            | Not listed; require pre-conversion (e.g. LibreOffice â†’ DOCX or PDF) or a fallback tool. |
| Legacy PowerPoint| `.ppt`             | âŒ            | Same as `.doc`; use external conversion or fallback. |
| Legacy Excel     | `.xls`             | âŒ            | Same as `.doc`; convert upstream. |
| OpenDocument Text| `.odt`             | âŒ            | Not mentioned in supported formats; use Unstructured/Tika if needed. |
| OpenDocument Pres| `.odp`             | âŒ            | Same as `.odt`. |
| OpenDocument Calc| `.ods`             | âŒ            | Same as `.odt`. |
| Rich Text        | `.rtf`             | âŒ            | No explicit support; use fallback. |

##### 6.1.2.2.2 Markup & Text Formats

| Format              | Extension              | Support Level | Notes |
|--------------------|------------------------|--------------|-------|
| HTML               | `.html`, `.htm`        | âœ…            | First-class input; DOM parsed into structural blocks. |
| XHTML              | `.xhtml`               | âš ï¸           | Likely handled as HTML if parsed, but not explicitly called out. |
| Markdown           | `.md`                  | âœ…            | Explicitly supported; both input and output. |
| AsciiDoc           | `.adoc`                | âŒ            | No mention; treat via external converter or LLM if needed. |
| reStructuredText   | `.rst`                 | âŒ            | Same as AsciiDoc. |
| LaTeX              | `.tex`                 | âŒ            | Not supported directly; better handled by LLM or external tools. |
| Plain Text         | `.txt`                 | âš ï¸           | Not listed as a first-class InputFormat but can be handled via â€œcustom text inputâ€ / pipeline utilities; no layout semantics. |

##### 6.1.2.2.3 Image Formats (OCR / Vision)

| Format | Extension           | Support Level | Notes |
|--------|---------------------|--------------|-------|
| PNG    | `.png`              | âœ…            | Explicitly supported for OCR and figure extraction. |
| JPEG   | `.jpg`, `.jpeg`     | âœ…            | Explicitly supported. |
| TIFF   | `.tiff`, `.tif`     | âœ…            | Explicitly supported; important for multipage scans. |
| BMP    | `.bmp`              | âš ï¸           | Not documented; may work via Pillow/ffmpeg in practice but not guaranteed. |
| WEBP   | `.webp`             | âš ï¸           | Same as BMP; not listed. |
| GIF    | `.gif`              | âš ï¸           | Static images might work if converted; no docled guarantee. |
| HEIC/HEIF | `.heic`, `.heif` | âŒ            | Not mentioned; likely require upstream conversion. |
| SVG    | `.svg`              | âŒ            | Vector; Docling focuses on raster inputs and PDF. Treat as unsupported. |

##### 6.1.2.2.4 Audio Formats (ASR)

Docling supports audio via an ASR pipeline built on Whisper; internally it uses a generic audio input type and ffmpeg for decoding.

| Format    | Extension | Support Level | Notes |
|-----------|-----------|--------------|-------|
| WAV       | `.wav`    | âš ï¸           | Likely supported via ffmpeg; not explicitly called out but ffmpeg covers it. |
| MP3       | `.mp3`    | âœ…            | Explicit example input in ASR docs and marketing text (â€œsupports audio via ASRâ€). |
| FLAC      | `.flac`   | âš ï¸           | ffmpeg-dependent; not explicitly documented. |
| OGG       | `.ogg`    | âš ï¸           | Same as FLAC. |
| M4A/AAC   | `.m4a`    | âš ï¸           | Same as FLAC; rely on ffmpeg. |
| WebM Audio| `.webm`   | âš ï¸           | If ffmpeg decodes it, ASR pipeline can consume it; not guaranteed in docs. |

Whisper gives multilingual transcription; Doclingâ€™s example focuses on English but Whisper itself supports many languages.

##### 6.1.2.2.5 Video Formats

Docling **does not** directly treat video containers as an InputFormat. Official messaging mentions â€œaudioâ€ but not video.

| Format | Extension | Support Level | Notes |
|--------|-----------|--------------|-------|
| MP4    | `.mp4`    | âŒ            | No direct support; recommended pattern: extract audio via ffmpeg â†’ feed into ASR pipeline. |
| WebM   | `.webm`   | âŒ            | Same pattern as MP4. |
| MKV    | `.mkv`    | âŒ            | Same pattern as MP4. |
| MOV    | `.mov`    | âŒ            | Same pattern as MP4. |

##### 6.1.2.2.6 Subtitle / Caption Formats

| Format | Extension | Support Level | Notes |
|--------|-----------|--------------|-------|
| WebVTT | `.vtt`    | âœ…            | Explicitly supported; treated as text with timing metadata. |
| SRT    | `.srt`    | âŒ            | Not listed; trivial to parse externally if needed. |
| ASS/SSA| `.ass`, `.ssa` | âŒ      | Not supported. |

##### 6.1.2.2.7 Data & Structured Formats

| Format | Extension | Support Level | Notes |
|--------|-----------|--------------|-------|
| CSV    | `.csv`    | âœ…            | Explicit example (â€œConversion of CSV filesâ€); integrated into pipeline. |
| TSV    | `.tsv`    | âš ï¸           | Can usually be treated as CSV with tab delimiter, but not explicitly documented. |
| JSON   | `.json`   | âœ…            | Listed as input; content from JSON values. |
| XML    | `.xml`    | âœ…            | Generic XML plus â€œCustom XMLâ€ example; needs configuration for schema-aware mapping. |
| YAML   | `.yaml`   | âŒ            | Not mentioned; convert to JSON or use separate parser. |

##### 6.1.2.2.8 Specialized / Domain-Specific

| Format       | Extension | Support Level | Notes |
|-------------|-----------|--------------|-------|
| USPTO XML   | â€“         | âš ï¸           | Can be handled as custom XML with a mapping plugin; not first-class. |
| JATS XML    | â€“         | âš ï¸           | Same as USPTO XML. |
| EPUB        | `.epub`   | âœ…            | Explicit input format. |
| MOBI        | `.mobi`   | âŒ            | Not supported; convert upstream. |
| DjVu        | `.djvu`   | âŒ            | Not supported; convert via external tools to PDF. |
| XPS         | `.xps`    | âŒ            | Not supported; convert upstream. |

##### 6.1.2.2.9 Email Formats

| Format | Extension | Support Level | Notes |
|--------|-----------|--------------|-------|
| EML    | `.eml`    | âŒ            | Not supported; Unstructured and Tika fill this gap. |
| MSG    | `.msg`    | âŒ            | Same as EML. |
| MBOX   | `.mbox`   | âŒ            | Same as EML. |

##### 6.1.2.2.10 Archive Formats

| Format        | Extension | Support Level | Notes |
|---------------|-----------|--------------|-------|
| ZIP (archive) | `.zip`    | âœ…            | Explicitly supported as container; Docling processes supported files inside. |
| PDF Portfolio | `.pdf`    | âš ï¸           | Not treated specially; likely seen as normal PDF pages; embedded attachments not handled generically. |

---

#### 6.1.2.3 Handshake Integration Assessment

This section covers both Doclingâ€™s capabilities and how they map into Handshakeâ€™s architecture.

##### 6.1.2.3.1 Licensing & Open-Source Position

- **Core Docling** (`docling`): MIT license.  
- **Docling-IBM models** (DocLayNet, TableFormer, Granite-Docling integrations): MIT license for the integration library; individual models hosted on Hugging Face carry their own model licenses (Apache-2.0 or custom IBM terms) which must be checked per model.  
- **Docling-Serve**: MIT license (server wrapper).  
- **Docling-MCP**: MIT license.  

Implications for Handshake:

- Safe for **closed-source commercial desktop distribution**; no copyleft/AGPL contamination from Docling itself.
- Care is required when adding **optional dependencies** (e.g. PyMuPDF, which is AGPL/commercial) â€“ but Doclingâ€™s main pipeline does not require PyMuPDF; its own stack is designed to be commercially friendly.  
- Model licenses (Granite-Docling, SmolDocling, Qwen2.5-VL, Pixtral, etc.) must be reviewed but typically allow commercial use with attribution.  

Conclusion: **No licensing blocker** for embedding Docling into a proprietary Tauri desktop app, provided you track model licenses and avoid AGPL-only dependencies.

##### 6.1.2.3.2 Docling Output Structure & Semantics

Doclingâ€™s core output is the `DoclingDocument` schema: a hierarchical representation of the document with pages, blocks, inlines, tables, figures, and metadata.

Key properties:

- **Hierarchy**: Document â†’ Sections â†’ Pages â†’ Blocks (paragraphs, headings, lists, tables, figures, formulas, code, etc.).  
- **Geometry**: Each block carries page index + bounding boxes (coordinates) for layout-aware features and visual canvases.  
- **Provenance**: References to source page and element IDs enable round-tripping, anchor links, and precise citations.  
- **Confidence scores**: Documented concept; per-element confidence values can be used for quality thresholds and flight recorder metrics.  
- **Serialization formats**: Markdown, HTML, plain text, JSON, DocTags (a token-friendly structured representation).  

How this helps Handshake:

- Straightforward mapping into **block-based editors** (Tiptap/ProseMirror).
- Bounding boxes provide coordinates for **canvases** (Milanote/Miro-style) and figure thumbnails.
- Provenance + confidence feed into **Raw/Derived/Display separation** and into **flight recorder** metrics.

###### Representation of specific elements

- **Tables**: Represented as structured objects with rows, columns, and cell spans, including merged cells and header hierarchies.  
- **Images/Figures**: Extracted as separate figure objects with coordinates and optional captions.  
- **Lists**: Ordered and unordered lists with proper nesting.  
- **Code blocks**: Preserved, with scope for language detection using enrichment pipelines.  
- **Footnotes/endnotes/citations**: Represented in metadata and blocks; technical report explicitly covers reference extraction (citations and bibliography).  
- **Mathematical formulas**: Identified and extractable; enrichment examples for formulas exist.  

##### 6.1.2.3.3 Table Extraction Deep Dive

Doclingâ€™s table pipeline uses a tailored TableFormer model plus layout cues:

- Recovers **complex tables** including merged cells, multi-row headers, sparsely bordered tables, and text-heavy tables.  
- Handles **multi-column documents** by pairing layout analysis (DocLayNet) with table detection.  
- Outputs structured table objects that can be exported as Markdown, HTML, CSV, or JSON.  

Technical report benchmarks show strong table accuracy across complex PDFs and improved performance vs. baseline PDF parsers; limitations still exist with extremely irregular and marketing-style tables (e.g., the â€œbank advertisementâ€ example where formatting was incorrect even though raw content was captured).  

Compared to pdfplumber, Camelot, and Tabula:

- Docling/TableFormer performs better on **unstructured, non-grid tables** and mixed layout PDFs; those tools excel on regular, gridlike tables but lack global layout context.  
- For Handshake, Docling should be the default table extractor; pdfplumber/Camelot can be optional niche tools (e.g. highly regular invoices) if ever needed.

##### 6.1.2.3.4 Layout & Visual Understanding

Doclingâ€™s core strength is **layout-aware PDF understanding**:

- Uses DocLayNet for page segmentation (text boxes, tables, figures, headings).  
- Computes **reading order**, correcting for multi-column layouts, sidebars, footers/headers.  
- Identifies figures, tables, and code blocks as distinct elements in DoclingDocument.  

Diagram/chart understanding is currently limited to figure detection + caption extraction; full semantic chart parsing is on the longer-term roadmap.  

This is highly aligned with Handshakeâ€™s needs:

- Layout metadata drives **canvas placement** and **reference views**.
- Reading order is crucial for chunking and RAG to avoid scrambled paragraphs.

##### 6.1.2.3.5 OCR Capabilities

Docling offers **pluggable OCR backends**:

- Examples for Tesseract (with automatic language detection), RapidOCR, and Surya OCR.  
- Supports OCR for scanned PDFs and stand-alone images; can force full page OCR even where embedded text exists.  
- Multilingual OCR support depends on the selected engine and installed language packs (Tesseract/Surya/RapidOCR are multilingual).  

The technical report notes that OCR is **quality-sensitive and slower** than pure text extraction, but integrated into the same pipeline and benefiting from the layout model for text placement.  

For Handshake:

- OCR output should be treated as **DerivedContent**, not canonical RawContent, and clearly marked as such (confidence + OCR flag).
- For worst-case scans, you can route through VLM (Granite-Docling / SmolDocling) instead of or in addition to OCR.

##### 6.1.2.3.6 ASR (Audio / Speech) Capabilities

Docling provides an ASR pipeline using Whisper:

- Example pipelines show `InputFormat.AUDIO` + Whisper-based transcription, producing timestamped segments.  
- Official website states â€œsupports audio via automatic speech recognition (ASR)â€.  

Current visible capabilities:

- **Multilingual** (via Whisper itself).
- **Timestamped segments**; no strong evidence of built-in speaker diarization â€“ diarization would likely need a separate tool (PyAnnote or similar).
- Same DoclingDocument abstraction is used to capture transcripts and link them to source media.

For Handshake (which already needs ASR for lectures):

- Doclingâ€™s ASR pipeline can be used for **basic transcription + segmentation**, but if you require diarization or advanced segmentation, youâ€™ll need complementary tools or your own Whisper integration.
- You can still rely on Docling for **uniform output format** and integration into the same RAG/DoclingDocument world.

##### 6.1.2.3.7 Visual Language Model (VLM) Support

Docling integrates VLMs via the `VlmPipeline`.  

Supported local models (out-of-the-box):

- Granite-Docling-258M (Transformers + MLX variants).
- SmolDocling-256M preview (Transformers + MLX).
- Several generic VLMs (Qwen2.5-VL-3B, Pixtral-12B, Gemma-3-12B, Granite-Vision-2B, Phi-4-multimodal).  

Capabilities:

- End-to-end **VLM pipelines** for PDF â†’ Markdown/DocTags/HTML (no separate OCR/layout â€“ the VLM handles everything).
- Can also route to **explicit compatibility VLM services** when operator-configured; core operation uses Handshake-managed local VLM engines.  

Implications for Handshake:

- You can configure Docling to use a **local VLM for â€œhardâ€ pages** (e.g., heavily graphical PDFs) while using the classical layout + OCR pipeline for the rest.
- The VLM pipeline may consume significant GPU resources; Handshake should treat it as a **background/off-peak operation**, scheduled against LLM workloads.

##### 6.1.2.3.8 Technical Architecture (Docling Itself)

From docs + technical report:  

- **Language**: Python, with core ML models in PyTorch and support for GPU (CUDA) and Apple MLX.
- **Key components**:
  - `docling-core`: datamodel and basic utilities.
  - `document_converter`: orchestrates parsing, layout analysis, table detection, OCR, ASR, VLM pipelines into a `DoclingDocument`.
  - Optional `docling-ibm-models` for DocLayNet/TableFormer/Granite integration.
- **Execution modes**:
  - Python API.
  - CLI (`docling`).
  - Docling-Serve (REST API) or direct Python API/CLI under Handshake-owned lifecycle; upstream Docker/K8s/Quarkus integrations are reference or compatibility paths only.  
  - MCP server (`docling-mcp`).  

Performance:

- PDF pipeline is highly optimized: CPU-only performance is competitive; GPU acceleration yields substantial speedups in benchmarking (L4/M3 Max).  

For Handshake, the key architectural choice is **how to host Docling** (embedded Python vs sidecar vs remote server). That is addressed in 3.10.

##### 6.1.2.3.9 RAG & Embedding Compatibility

Docling includes dedicated support for chunking and RAG:  

- **Hybrid chunking**: Combines structural (headings, paragraphs) and token-based chunk sizes; tuned for LLM context windows.  
- **Serialization & Chunking examples**: Show how to emit DoclingDocument â†’ JSON/Parquet with chunks and metadata keyed by IDs.  
- **Integrations**:
  - LangChain.
  - LlamaIndex.
  - Haystack.
  - Langflow, txtai, Milvus, Qdrant, MongoDB, etc.  

For Handshake:

- You can reuse Doclingâ€™s **chunking strategies** to generate embedding chunks and store them in `sqlite-vec` / LanceDB.
- Chunk metadata includes **page, coordinates, section IDs**, which can be mapped directly into your Shadow Workspace and Knowledge Graph.

##### 6.1.2.3.10 Detailed Integration Points (10.x)

###### Rust â†” Python Bridge

Options in Handshake context:

| Approach                    | Recommendation for Handshake                                         | Pros | Cons | Latency | Complexity |
|----------------------------|------------------------------------------------------------------------|------|------|---------|-----------|
| PyO3 / embedded Python     | âŒ Avoid as primary path                                              | Very low call overhead | Complex build, GIL issues, shipping Python/runtime inside Tauri, fragile | Low | High |
| Direct subprocess (CLI)    | âš ï¸ Good for prototypes and single-shot conversions                    | Simple, no custom server | Process startup overhead, limited concurrency & observability | Medium | Low |
| **Docling-Serve (HTTP)**   | âœ… Primary recommendation (Rust â†’ HTTP â†’ Docling-Serve)               | Clear API, async, can run as Tauri sidecar; easy to scale | Extra process to manage, HTTP overhead | Medium | Medium |
| Custom FastAPI worker      | âœ… Alternative: embed Docling in same Python orchestrator as LLMs     | Full control, simple JSON API, easy to integrate with orchestrator | Need to design protocol & job queue | Medium | Medium |
| Unix socket / gRPC         | âš ï¸ Overkill initially                                                 | Lower overhead than HTTP | More plumbing/maintenance, no off-the-shelf server | Low | High |

**Recommendation:**

- **Short term**: run a **Python worker** (FastAPI or simple job runner) inside your existing Python orchestrator process, with a minimal REST or local queue API.
- **Medium term**: if usage grows or you want more isolation, adopt **Docling-Serve** as a dedicated sidecar service, managed by Handshake (start/stop, health checks).  

This keeps Rust code clean and places Docling alongside your LLM runtime.

###### Mapping to Tiptap / ProseMirror Blocks

Mapping from `DoclingDocument` â†’ Tiptap nodes:

| Docling Element             | Tiptap Block / Node              | Mapping Complexity |
|-----------------------------|----------------------------------|--------------------|
| Paragraph                   | `paragraph`                      | Low |
| Heading(level)             | `heading` with `level` attribute | Low |
| List (unordered / ordered) | `bulletList` / `orderedList` + `listItem` | Low |
| Table                       | `table` â†’ `tableRow` â†’ `tableCell` | Medium (spans, headers) |
| Figure (image + caption)   | `image` node + custom `figcaption` or `caption` attribute | Medium |
| Code block                 | `codeBlock` (with language attr) | Low |
| Blockquote                 | `blockquote`                     | Low |
| Formula/Equation           | Custom `math_block` node         | Medium |
| Footnote                   | Custom `footnote` inline/block   | Medium |

Implementation plan:

- Define a **Docling â†’ Tiptap transformer** in TypeScript, using a JSON version of DoclingDocument.  
- Use bounding boxes + page references only as **metadata** on blocks (for canvases, citations), not in the visible ProseMirror schema.  
- For very large documents, stream page-by-page and insert blocks in batches.

This mapping is straightforward and well within Tiptapâ€™s extension model.

###### CRDT Integration (Yjs)

Target flow:

```text
[Source file] â†’ [Docling pipeline] â†’ [DoclingDocument]
    â†’ [Block Transformer] â†’ [Yjs ops with AI site ID]
```

Key decisions:

- **AI as CRDT participant**: use a dedicated `siteId` in Yjs for imports performed by Docling (â€œdocling-importerâ€).  
- **Atomic import**:
  - Prefer **page-or section-level transactions** (e.g., one Yjs transaction per page) to keep history chunks manageable and allow progressive UX.  
- **Conflicts**:
  - If importing into an empty doc, simply append blocks.  
  - If re-importing an updated version, you can:
    - Either import into a **new doc** and let the user diff,
    - Or run a block-level diff (ID/anchor-based) and apply Yjs updates selectively.

Provenance:

- Attach metadata per block:
  - `sourceDocumentId`, `page`, `bbox`, `conversionVersion`, `pipelineConfigHash`.
- This supports re-processing strategies and flight recorder integration.

###### Shadow Workspace Integration

For Handshakeâ€™s Shadow Workspace (incremental indexing + embeddings):

- Use Doclingâ€™s **chunking/serialization** utilities to emit stable chunk IDs (e.g., `docId:sectionId:chunkIndex`).  
- Store:
  - Chunk text.
  - Corresponding block IDs / page + bbox.
  - Hash of source content + Docling version + pipeline options.

Dirty detection:

- If the source file changes:
  - Re-run Docling and compare **per-chunk hashes**; only update changed chunks in vector store and knowledge graph.
- For huge PDFs:
  - Use **page-streaming** conversion and store page-level fingerprints to skip unchanged pages.

###### Knowledge Graph Population

Docling by itself does **not** do full NER/relationship extraction, but:

- It provides rich **structure** (sections, headings, tables, figures, citations).  
- There is an â€œInformation extractionâ€ section in docs, oriented around integrating extraction pipelines on top of DoclingDocument.  

Recommended approach:

- Treat Docling as the **structure+text provider**.
- Run Handshakeâ€™s own entity extraction / relation extraction LLMs over:
  - Sections (for hierarchical nodes).
  - Tables (for schema + records).
  - Citations (for edges to external docs).
- Map:

```text
DoclingDocument
â”œâ”€ Metadata      â†’ Document node
â”œâ”€ Sections      â†’ Section nodes (+ parent-child edges)
â”œâ”€ Paragraphs    â†’ Text nodes attached to sections
â”œâ”€ Tables        â†’ Table nodes + row/column/cell nodes
â”œâ”€ Figures       â†’ Figure nodes
â”œâ”€ Citations     â†’ Citation edges
â””â”€ Entities(*)   â†’ Entity nodes (from NER)
```

Doclingâ€™s confidence scores and provenance fields should be stored as node/edge attributes for later auditing.

###### Raw / Derived / Display Separation

Mapping into Handshakeâ€™s 3-way model:

| Docling Output                       | Handshake Category | Notes |
|-------------------------------------|--------------------|-------|
| Original file (PDF, DOCX, etc.)     | RawContent         | Immutable canonical import. |
| Direct text extraction (native PDFs)| RawContent or DerivedContent | For â€œdigital originalsâ€ you might treat as Raw; but you can still regenerate if you trust Docling to be deterministic. |
| OCRâ€™d text                          | DerivedContent     | Must be marked with `ocr=true`, `engine`, `language`, `confidence`. |
| Layout analysis (DocLayNet)         | DerivedContent     | Stored as JSON sidecar; safe to recompute. |
| Table structure (TableFormer)       | DerivedContent     | Sidecar or block metadata. |
| Chunk IDs, embeddings               | DerivedContent     | Vector store only; always recomputable. |
| User-editable imported doc in editor| DisplayContent (+ RawContent) | Once user edits imported blocks, they become user RawContent going forward. |

Reprocessing strategy:

- **Version** every Docling run: `doclingVersion`, `pipeline`, `modelVersions`.
- When upgrading Docling or models:
  - Option 1: Reprocess only when user requests â€œRe-ingest with new parser.â€
  - Option 2: Lazy reprocess when doc is opened, storing new DerivedContent while leaving old RawContent intact.

###### Flight Recorder Integration

Docling itself doesnâ€™t ship a full observability stack, but it exposes enough hooks:

- `ConversionStatus`, error messages, and warning lists.  
- Per-element **confidence scores** for layout, OCR, tables, etc.  

For Handshakeâ€™s DuckDB-based flight recorder:

- Log per document:
  - `docId`, `sourcePath`, size, pages.
  - `startTime`, `endTime`, `duration`.
  - Pipeline options (OCR engine, VLM vs standard).
  - `doclingVersion`, `modelVersions`.
  - `status`, error/warning counts.
  - Aggregate confidence histograms (min/mean/max).
- Log per page (optional):
  - Processing time.
  - Count of tables, figures.
  - OCR percentage (area / tokens).

This supports:

- Performance tuning (detect slow docs).
- Quality monitoring (low confidence threshold alerts).
- Replayability of conversions.

###### Resource Management

Doclingâ€™s performance characteristics:

- CPU-only PDF pipeline is already optimized; GPU acceleration accelerates layout and table models.  
- VLM pipelines with Granite-Docling / SmolDocling can be significantly slower and more GPU-hungry, especially on larger models (Pixtral-12B, Gemma-12B).  

For Handshakeâ€™s hardware (Ryzen 9, 128 GB RAM, 24 GB VRAM):

- Run **standard Docling pipeline on CPU by default**; itâ€™s fast enough for most docs and avoids contention with local LLMs.
- Enable **GPU acceleration selectively**:
  - For large batch ingestions or long PDFs.
  - During â€œmaintenance windowsâ€ when LLMs are idle.
- For VLM pipeline:
  - Treat as **opt-in** (user toggles â€œhigh-fidelity ingestâ€).
  - Limit concurrency to 1â€“2 documents at a time.
- Integrate Docling into your **task scheduler**, giving it a lower priority than interactive LLM sessions.

###### Error Handling & Recovery

Typical failure scenarios and suggested behaviors:

| Scenario                     | Expected Behavior                          | Recovery Strategy |
|------------------------------|-------------------------------------------|-------------------|
| Corrupted PDF                | Docling returns error status              | Mark import as failed, log error; offer â€œdownload corrupted fileâ€ for debugging. |
| Password-protected PDF       | Docling fails to open                     | Detect error string, prompt user for password; retry. |
| 1000+ page document          | Long processing time                      | Use page-streaming; show progress UI; allow cancellation and resumption. |
| Unsupported format           | 415/unsupported                           | Route to Unstructured/Tika fallback; log. |
| OCR failure (low confidence) | Sparse or noisy text, low confidence      | Flag quality issue; allow user to re-run with different OCR engine or VLM pipeline. |
| OOM (GPU/CPU)                | Crash or error from pipeline              | Retry with reduced options (no VLM, CPU only, smaller batch); throttle concurrency. |

###### Connector Integration

Recommended placement of Docling in Handshakeâ€™s connector pipeline:

```text
Connectors: JMAP / CalDAV / MCP / File watcher / etc
      â”‚
      â–¼
[Format detection + routing]
      â”‚
      â”œâ”€ Docling (primary for PDF/Office/HTML/EPUB/images/audio)
      â”‚
      â”œâ”€ Unstructured (email formats, fringe types)
      â”‚
      â””â”€ Tika / others (rare legacy formats)
      â–¼
[Block Transformer â†’ Yjs/CRDT]
      â”‚
      â”œâ”€ Shadow Workspace (chunks + embeddings)
      â””â”€ Knowledge Graph (structured nodes/edges)
```

Docling can be:

- Invoked directly from file watcher for local files.  
- Used as a **tool** in MCP for agentic workflows (agents request conversions through Docling MCP server).  

---

##### 6.1.2.3.11 Integration Point Summary Table

| Integration Point     | Recommendation                                               | Complexity | Risk |
|-----------------------|-------------------------------------------------------------|-----------|------|
| Rust â†” Python bridge  | Docling-Serve or custom Python worker (HTTP/queue)         | Medium    | Low  |
| Tiptap block mapping  | Dedicated Doclingâ†’Tiptap transformer using DoclingDocument | Medium    | Low  |
| CRDT integration      | Yjs ops per page/section with AI site ID                   | Medium    | Medium (large docs/history) |
| Shadow Workspace      | Use Docling chunking + stable chunk IDs + hashes           | Medium    | Low  |
| Knowledge Graph       | Run NER/RE over Docling structure; store provenance        | Medium    | Medium |
| Raw/Derived separation| Treat original files as Raw, Docling outputs as Derived    | Low       | Low  |
| Flight recorder       | Log per-doc/page metrics + confidence + versions           | Medium    | Low  |
| Resource management   | CPU-default; GPU/VLM on demand; scheduler-integrated       | Medium    | Medium |

---

#### 6.1.2.4 Competitor Recommendation

##### 6.1.2.4.1 Short Deep Dives

###### Unstructured.io

- **License**: Apache-2.0.  
- **Strengths**: Very broad format support (including EML/MSG/MBOX, RTF, ODT, many more), rich connectors, built-in chunking and RAG-ready element outputs.  
- **Weaknesses**: Slower and less layout-aware on PDFs than Docling; table extraction is good but not as specialized as TableFormer.  
- **Handshake role**: **Fallback / complement** to Docling, especially for email and fringe formats.

###### Marker

- **License**: GPL-3.0 (copyleft).  
- **Strengths**: Extremely good PDFâ†’Markdown for books and scientific papers; handles equations, figures, tables; highly praised in community.  
- **Weaknesses**: PDF-only; GPL-3.0 is problematic for closed-source Handshake core; focused on text/markdown, not full structured document graph.  
- **Handshake role**: Avoid in core product due to GPL; possible **external tool** or user-managed plugin if ever needed, but not recommended.

###### LlamaParse

- **License / deployment**: Proprietary, cloud-hosted (LlamaCloud).  
- **Strengths**: LLM-driven parsing with excellent handling of complex documents, charts, images, handwriting; strong RAG story with LlamaIndex.  
- **Weaknesses**: Cloud-only, per-page pricing; conflicts with Handshakeâ€™s local-first and privacy goals.  
- **Handshake role**: Maybe a **toggleable cloud fallback** for â€œparsing rescue modeâ€, but not primary.

###### PyMuPDF (fitz)

- **License**: AGPL-3.0 or commercial.  
- **Strengths**: Very fast, low-level PDF manipulation; precise coordinate access; good for building custom pipelines.  
- **Weaknesses**: License problematic unless you buy commercial license; no high-level layout and table understanding like Docling/TableFormer.  
- **Handshake role**: Only if you deliberately buy a commercial license and need low-level PDF operations; otherwise, avoid.

###### Apache Tika

- **License**: Apache-2.0.  
- **Strengths**: Massive format coverage (1000+ formats), robust metadata extraction, Java ecosystem, server mode.  
- **Weaknesses**: Basic text extraction; limited layout understanding; more heavy-weight (JVM).  
- **Handshake role**: Optional **format detection + rare format fallback**.

##### pdfplumber

- **License**: MIT.  
- **Strengths**: Reliable for programmatic PDFs and simple tables; precise coordinates.  
- **Weaknesses**: No OCR, no deep layout model; not as robust on messy tables as TableFormer.  
- **Handshake role**: Optional niche tool for known structured forms/invoices.

###### Camelot / Tabula

- **License**: MIT (Camelot, Tabula core is Apache-style).  
- **Strengths**: Specialized for tables; very good for border-heavy or regular tables.  
- **Weaknesses**: Tables-only, no broader document understanding, no OCR.  
- **Handshake role**: Unnecessary given Docling/TableFormer, unless you hit a very specific table class where they outperform.

##### 6.1.2.4.2 Tool-per-Format Recommendation Matrix

| Format Category | Primary Tool | Fallback Tool(s) | Rationale |
|-----------------|-------------|------------------|-----------|
| PDF (digital + scanned) | **Docling** | Unstructured; optionally LlamaParse cloud for â€œrescue modeâ€ | Docling provides best mix of speed, layout, tables, OCR, licensing. |
| Office (DOCX/PPTX/XLSX) | **Docling** | Unstructured, Tika | Docling supports them natively and outputs structured blocks; Unstructured/Tika can catch edge cases. |
| HTML/Markdown/EPUB | **Docling** | Unstructured | Docling gives DoclingDocument tree and chunking; Unstructured is fallback for odd web formats. |
| Images (scans) | **Docling + OCR engines** | Direct OCR pipeline (e.g., Tesseract via other libs) | Docling unifies OCR with layout and table detection. |
| Audio | **Docling ASR** | Native Whisper integration | Docling wraps Whisper and emits DoclingDocument; you may still prefer controlling Whisper yourself for advanced features. |
| Email (EML/MSG/MBOX) | **Unstructured** | Tika | Docling doesnâ€™t support email formats; Unstructured and Tika do. |
| Legacy Office / RTF / ODT | **Unstructured or Tika** | LibreOffice + Docling (convertâ†’DOCX/PDF) | Better delegated to tools focusing on legacy formats. |
| Arbitrary binary/rare formats | **Tika** | Unstructured | Tikaâ€™s MIME detection + broad coverage. |

##### 6.1.2.4.3 Hybrid Strategy (Recommended)

Docling as **primary engine**, Unstructured + Tika as fallback, Marker + LlamaParse + PyMuPDF explicitly avoided or limited to user-managed plugins for licensing/local-first reasons. This matches the hybrid architecture already suggested by both Docling community examples and third-party integrations.  

---

#### 6.1.2.5 Architecture Diagram (Text)

```text
                          â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
                          â”‚            Handshake UI              â”‚
                          â”‚  (Tauri + Rust + React + Tiptap)    â”‚
                          â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                                           â”‚
                          CRDT (Yjs)       â”‚   Commands (import file, show status)
                     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
                     â”‚         Rust Coordinator / Orchestrator    â”‚
                     â”‚  - Starts & monitors sidecars              â”‚
                     â”‚  - Schedules ingestion jobs                â”‚
                     â”‚  - Talks to Python orchestrator / Docling  â”‚
                     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                                     â”‚               â”‚
                         HTTP / queueâ”‚               â”‚gRPC/HTTP (models)
                                     â”‚               â”‚
             â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”       â”Œâ”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
             â”‚   Python Orchestrator +   â”‚       â”‚   Local LLM Runtime(s) â”‚
             â”‚   Docling Worker(s)       â”‚       â”‚ (ModelRuntime native)  â”‚
             â”‚ - Docling API/CLI         â”‚       â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
             â”‚ - Unstructured / Tika     â”‚
             â”‚ - ASR, OCR, VLM pipelines â”‚
             â””â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜
                   â”‚             â”‚
         DoclingDocument JSON    â”‚
                   â”‚             â”‚
       â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”      â”‚
       â”‚ Block Transformerâ”‚      â”‚
       â”‚ (Doclingâ†’Tiptap) â”‚      â”‚
       â””â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜      â”‚
               â”‚ Yjs ops        â”‚ Derived artifacts (chunks, KG nodes)
      â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”       â–¼
      â”‚   CRDT Store    â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
      â”‚ (Yjs docs)      â”‚   â”‚ Shadow WS   â”‚   â”‚ Knowledge KGâ”‚
      â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚ (chunks +  â”‚   â”‚ (Cozo/Kuzu) â”‚
               â”‚            â”‚ embeddings)â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚            â””â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜
               â”‚                  â”‚
               â”‚                  â–¼
               â”‚            DuckDB Flight
               â”‚            Recorder (logs)
               â–¼
       SQLite + asset store
       (Raw/Derived separation)
```

---

#### 6.1.2.6 Risk Assessment

| Risk Category | Level | Details & Mitigation |
|---------------|-------|----------------------|
| Technical     | Medium | Docling is evolving quickly; APIs (e.g., Docling-Serve) may change, and some advanced features (charts, equations) are still maturing. Mitigation: pin versions, wrap Docling behind an internal adapter layer, and rely on tested pipelines only in the first release. |
| Licensing     | Lowâ€“Medium | Core Docling stack is MIT and safe. Risk comes from optional dependencies (PyMuPDF) and models with their own terms. Mitigation: avoid AGPL dependencies; track and whitelist models with permissive licenses. |
| Maintenance   | Low | Docling is actively developed with IBM + community backing, integrated into LF AI & Data, with frequent releases and growing ecosystem integrations (LangChain, LlamaIndex, RHEL AI, Quarkus, Apify). |
| Performance   | Medium | For very large docs and VLM pipelines, CPU/GPU usage can be significant. Mitigation: default to CPU pipelines, enable GPU selectively, schedule heavy jobs, monitor via flight recorder. |
| Integration   | Medium | Requires cross-language orchestration (Rust â†” Python), mapping to Tiptap/Yjs, and multi-tool fallback routing. Mitigation: start with a minimal Docling-only path, build small, well-tested adapters, add Unstructured/Tika later as needed. |

---

#### 6.1.2.7 Proof-of-Concept Plan (4 Weeks)

##### 6.1.2.7.1 Week 1 â€“ Minimal Docling Integration

1. **Set up environment**
   - Add Docling (CPU-only) to Python orchestrator environment.
   - Implement a simple FastAPI endpoint: `/convert` â†’ returns DoclingDocument JSON for PDF/DOCX/HTML.  
2. **Rust bridge prototype**
   - From Rust coordinator, call `/convert` for a single local PDF.
   - Store DoclingDocument JSON as a sidecar in SQLite/DuckDB.  
3. **Block transformer v0**
   - Implement minimal Doclingâ†’Tiptap mapping for headings + paragraphs + lists.
   - Insert into a fresh Yjs document as a one-shot transaction.

Success criteria:

- You can drop a PDF into Handshake, see a block-based doc with correct headings/paragraphs.
- Ingestion is logged in DuckDB with basic timing + status.

##### 6.1.2.7.2 Week 2 â€“ Tables, Layout, and Shadow Workspace

1. **Table & image support**
   - Extend mapping to Tiptap for tables and images.
   - Verify merged cells and multi-row headers on a set of complex PDFs.  
2. **Shadow Workspace integration**
   - Use Doclingâ€™s chunking utilities to generate chunks with IDs and metadata.
   - Store chunks and embeddings in your vector store (sqlite-vec / LanceDB).
3. **Flight recorder v1**
   - Log per-document metrics: pages, duration, pipeline options, status.

Success criteria:

- Tables render reasonably; chunking pipeline produces stable IDs.
- You can run simple RAG queries against ingested docs.

##### 6.1.2.7.3 Week 3 â€“ OCR, ASR, and Fallbacks

1. **OCR pipeline**
   - Enable one OCR backend (e.g. Tesseract) and add â€œforce OCRâ€ toggle.
   - Mark OCR-derived text as DerivedContent with confidence metadata.  
2. **ASR prototype**
   - Implement the ASR pipeline for audio (`InputFormat.AUDIO` with Whisper) and map transcripts into block docs (one paragraph per segment or per sentence).  
3. **Introduce Unstructured fallback**
   - For email formats and unsupported types, call Unstructured and wrap its output into a minimal â€œpseudo-DoclingDocumentâ€ structure for consistency.  

Success criteria:

- Scanned PDFs import with usable text and flagged OCR metadata.
- Audio files import as timestamped transcripts into Handshake.
- Email attachments and .eml messages are ingested using Unstructured as a fallback.

##### 6.1.2.7.4 Week 4 â€“ VLM, MCP, and Hardening

1. **VLM pipeline (optional)**
   - Add a â€œhigh fidelityâ€ option using Granite-Docling or SmolDocling for selected pages or documents; measure GPU load and speed.  
2. **MCP integration**
   - Run Docling MCP server and integrate it into your internal agent framework so your own agents can call â€œconvert_documentâ€ and related tools.  
3. **Error handling & UX**
   - Implement clear user messaging for failures (corrupted PDF, password required, low OCR quality).
   - Add a re-ingest UI control that stores a new DerivedContent version while preserving RawContent.  

Success criteria:

- A small end-to-end â€œreference flowâ€:
  - Import a messy PDF, audio file, and email thread.
  - View them in Handshake as structured docs.
  - Run RAG against them.
  - Inspect logs in DuckDB and confirm metrics.

---

**Bottom line:**  
Use **Docling as the primary local document engine** for Handshake, wrapped behind a Python worker or Docling-Serve sidecar, with Unstructured and Tika as targeted fallbacks. This combination respects your local-first constraints, keeps licensing clean, and provides a strong technical base for CRDT-aware, AI-enhanced document workflows.

---

### 6.1.3 Part II â€“ Docling Integration Assessment for Project Handshake (Spec-style)

### 6.1.4 Docling Integration Assessment for Project Handshake

**IBM's open-source document processing library is an excellent fit for Handshake's local-first architecture.** Docling provides MIT-licensed AI-powered document conversion with state-of-the-art layout analysis and table extraction, runs efficiently on commodity hardware, and offers production-ready HTTP API integration via docling-serve. Key risks include memory management for batch processing and the need to bundle ~500MB of Python dependencies as a Tauri sidecar. The license stack is cleanâ€”all models use permissive licenses (MIT, Apache 2.0, CDLA-Permissive-2.0), enabling full commercial use without restrictions.

---

#### 6.1.4.1 Complete media type support matrix

Docling's format support is strong for documents and images but has notable gaps in email, legacy Office, and e-book formats that Handshake may need to address through complementary tools.

##### 6.1.4.1.1 Document formats

| Format | Support | Notes |
|--------|---------|-------|
| PDF (native digital) | âœ… Full | Primary focus; text + layout analysis |
| PDF (scanned/OCR) | âœ… Full | Automatic detection; EasyOCR/Tesseract |
| DOCX | âœ… Full | Via python-docx; hierarchy preserved |
| PPTX | âœ… Full | Slides as pages with layout |
| XLSX | âœ… Full | Via openpyxl |
| Legacy .doc/.ppt/.xls | âŒ None | Office 97-2003 not supported |
| OpenDocument (.odt/.odp/.ods) | âŒ None | Not implemented |
| RTF | âŒ None | Not documented |

##### 6.1.4.1.2 Markup and text formats

| Format | Support | Notes |
|--------|---------|-------|
| HTML/XHTML | âœ… Full | Via BeautifulSoup |
| Markdown | âœ… Full | Via Marko library |
| AsciiDoc | âœ… Full | Recent addition |
| reStructuredText | âŒ None | Not implemented |
| LaTeX (input) | âŒ None | Formula *output* to LaTeX supported |
| Plain text | âš ï¸ Partial | Wrapped as simple document |
| CSV | âœ… Full | Tabular data with structure |

##### 6.1.4.1.3 Image formats (OCR/Vision)

| Format | Support | Notes |
|--------|---------|-------|
| PNG | âœ… Full | Primary image format |
| JPEG/JPG | âœ… Full | Standard support |
| TIFF | âœ… Full | Multi-page supported |
| BMP, WEBP | âœ… Full | Modern format support |
| GIF | âš ï¸ Partial | Static only; animations ignored |
| HEIC | âŒ None | Apple format not supported |
| SVG | âŒ None | Vector format not supported |

##### 6.1.4.1.4 Audio formats (ASR)

| Format | Support | Notes |
|--------|---------|-------|
| WAV, MP3 | âœ… Full | Primary audio formats |
| FLAC, OGG, M4A, WebM | âš ï¸ Via ffmpeg | Requires ffmpeg on PATH |

**ASR models available**: Whisper tiny/base/small/medium/large/turbo with **90+ languages**. MLX acceleration for Apple Silicon. Timestamps preserved as `[time: start-end]` format. Speaker diarization not built-in.

##### 6.1.4.1.5 Video and subtitles

| Format | Support | Notes |
|--------|---------|-------|
| Video frame extraction | âŒ None | Not implemented |
| Audio extraction from video | âŒ None | Use external tools |
| WebVTT | âœ… Full | Recently added |
| SRT, ASS/SSA | âŒ None | Not supported |

##### 6.1.4.1.6 Specialized and data formats

| Format | Support | Notes |
|--------|---------|-------|
| USPTO XML (patents) | âœ… Full | Schema-specific parser |
| JATS XML (academic) | âœ… Full | Journal articles |
| EPUB, MOBI, DjVu | âŒ None | E-book formats not supported |
| JSON, XML, YAML (input) | âŒ None | Output formats only |
| EML, MSG, MBOX (email) | âŒ None | **Gap for Handshake** |

---

#### 6.1.4.2 Licensing analysis confirms commercial viability

The entire Docling stack uses permissive licenses compatible with proprietary software distribution.

| Component | License | Commercial Use |
|-----------|---------|----------------|
| **Docling codebase** | MIT | âœ… Unrestricted |
| **docling-core/parse** | MIT | âœ… Unrestricted |
| **DocLayNet layout model** | CDLA-Permissive-2.0 | âœ… Yes |
| **TableFormer** | CDLA-Permissive-2.0 | âœ… Yes |
| **Granite-Docling-258M** | Apache 2.0 | âœ… Yes |
| **SmolDocling-256M** | CDLA-Permissive-2.0 | âœ… Yes |
| **Heron layout models** | Apache 2.0 | âœ… Yes |

**No enterprise-only features exist**â€”all capabilities are open source. Attribution is requested but not legally required under MIT. The project is governed by the **LF AI & Data Foundation**, ensuring vendor-neutral stewardship.

---

#### 6.1.4.3 Technical architecture enables flexible integration

##### 6.1.4.3.1 Component stack

Docling is organized into four modular packages with clear separation of concerns:

- **docling-core**: Pydantic data models, chunkers, serializers (MIT)
- **docling-parse**: PDF backend using qpdf C++ library via pybind11 (MIT)
- **docling-ibm-models**: AI modelsâ€”RT-DETR layout detector, TableFormer (MIT wrapper, CDLA models)
- **docling-serve**: FastAPI HTTP server with async support, Redis queue backend (MIT)

##### 6.1.4.3.2 AI model performance characteristics

| Model | Architecture | Accuracy | Inference Time (CPU) |
|-------|--------------|----------|---------------------|
| **Layout (Heron-101)** | RT-DETR object detector | 78% mAP | 633ms/page (x86), 271ms (M3) |
| **TableFormer** | Vision Transformer | 97.9% TEDS (complex tables) | 1.7s/table (x86), 704ms (M3) |
| **Granite-Docling-258M** | Idefics3 VLM | 97% TEDS tables, 98.8% F1 code | ~2s/page (GPU) |

TableFormer achieves **93.6% TEDS** on table structure versus Tabula's 67.9% and Camelot's 73.0%, making it the clear choice for complex document tables.

##### 6.1.4.3.3 Resource requirements for Handshake deployment

| Configuration | Memory | Speed | Recommendation |
|---------------|--------|-------|----------------|
| **CPU-only (standard)** | 6GB peak | 0.6â€“1.3 pages/sec | Development, low-volume |
| **CPU-only (pypdfium)** | 2.5GB peak | 0.9â€“2.4 pages/sec | Production batch |
| **Apple Silicon (MLX)** | Shared memory | 1.3â€“2.4 pages/sec | Mac deployment |
| **CUDA GPU** | +1â€“2GB VRAM | 0.49s/page (L4) | High-volume processing |

---

#### 6.1.4.4 Handshake integration assessment

##### 6.1.4.4.1 Recommended architecture: docling-serve as Tauri sidecar

The optimal integration path uses docling-serve's HTTP API bundled as a PyInstaller sidecar with Tauri's shell API for process management.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                     Handshake (Tauri 2.x)                           â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚   React +   â”‚   â”‚     Rust      â”‚   â”‚   docling-serve      â”‚    â”‚
â”‚  â”‚  Tiptap/    â”‚â—„â”€â”€â”¤  Coordinator  â”œâ”€â”€â”€â”¤   (PyInstaller       â”‚    â”‚
â”‚  â”‚  BlockNote  â”‚   â”‚               â”‚   â”‚    sidecar)          â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚         â”‚                  â”‚                      â”‚                 â”‚
â”‚     ProseMirror       HTTP Client           localhost:5001          â”‚
â”‚        JSON            (reqwest)              REST API              â”‚
â”‚         â”‚                  â”‚                      â”‚                 â”‚
â”‚         â–¼                  â”‚                      â”‚                 â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”           â”‚                      â”‚                 â”‚
â”‚  â”‚   Yjs       â”‚â—„â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤                      â”‚                 â”‚
â”‚  â”‚   CRDT      â”‚    DoclingDocument              â”‚                 â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜     â†’ Tiptap                    â”‚                 â”‚
â”‚         â”‚                                        â”‚                 â”‚
â”‚         â–¼                  â–¼                     â–¼                 â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚              SQLite + sqlite-vec + CRDT sidecars            â”‚   â”‚
â”‚  â”‚  RawContent â†’ DerivedContent (DoclingDocument) â†’ Display    â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

##### 6.1.4.4.2 Integration points with complexity ratings

| Integration | Complexity | Risk | Implementation Notes |
|-------------|------------|------|---------------------|
| **docling-serve HTTP API** | Low | Low | Production-ready; sync/async endpoints |
| **Tauri sidecar bundling** | Medium | Medium | PyInstaller ~500MB; platform-specific builds |
| **DoclingDocument â†’ Tiptap** | Medium | Low | Tree traversal; handle merged table cells |
| **Yjs CRDT integration** | Medium | Low | Atomic import via transaction; provenance in metadata |
| **HybridChunker for Shadow Workspace** | Low | Low | Built-in tokenization-aware chunking |
| **Knowledge graph population** | High | Medium | NER not built-in; requires spaCy post-processing |
| **Raw/Derived separation** | Medium | Low | OCR text is Derived; track docling_version in schema |

##### 6.1.4.4.3 DoclingDocument to ProseMirror mapping

The DoclingDocument schema maps cleanly to Tiptap/ProseMirror blocks:

| DoclingDocument Type | ProseMirror Node | Special Handling |
|---------------------|------------------|------------------|
| TextItem (paragraph) | `paragraph` | Direct mapping |
| TextItem (heading) | `heading` with level attr | Level 1â€“6 support |
| TextItem (equation) | Custom `mathBlock` | LaTeX content |
| TableItem | `table` â†’ `tableRow` â†’ `tableCell` | Handle colspan/rowspan |
| PictureItem | `image` | Store bbox in data attr |
| ListGroup | `bulletList` / `orderedList` | Recursive nesting |
| CodeItem | `codeBlock` | Language detection included |

**Provenance preservation**: Store `data-docling-ref` (JSON pointer like `#/texts/5`) and `data-page` attributes on nodes for citation tracking.

##### 6.1.4.4.4 CRDT integration pattern

```typescript
// Atomic import with provenance tracking
function importDoclingDocument(docling: DoclingDocument, ydoc: Y.Doc) {
  const prosemirrorJson = convertDoclingToTiptap(docling);
  
  ydoc.transact(() => {
    const fragment = ydoc.get('prosemirror', Y.XmlFragment);
    fragment.delete(0, fragment.length);
    prosemirrorJsonToYXmlFragment(prosemirrorJson, fragment);
  }, 'docling-import');
  
  // Store import metadata
  ydoc.getMap('import-metadata').set('docling', {
    timestamp: Date.now(),
    docling_version: '2.63.0',
    source_hash: computeHash(docling.origin),
    model_versions: { layout: 'heron-101', table: 'tableformer-v2' }
  });
}
```

##### 6.1.4.4.5 Raw/Derived/Display classification

| Content Type | Classification | Rationale |
|--------------|----------------|-----------|
| Original PDF text | **Raw** | Canonical extraction |
| OCR text from images | **Derived** | Regenerable, model-dependent |
| Table structure | **Derived** | Layout analysis dependent |
| Bounding boxes | **Raw** | Fixed from source |
| Chunk embeddings | **Derived** | Model-specific |
| User annotations | **Raw** | User-generated |
| ProseMirror JSON | **Display** | Filtered presentation |

---

#### 6.1.4.5 Competitor recommendation matrix

##### 6.1.4.5.1 License compatibility summary

| Tool | License | Compatible with MIT App? | Best Use Case |
|------|---------|--------------------------|---------------|
| **Docling** | MIT | âœ… Yes | Primary document processing |
| **pdfplumber** | MIT | âœ… Yes | Coordinate extraction, debugging |
| **Camelot-py** | MIT | âœ… Yes | Simple lattice table fallback |
| **Apache Tika** | Apache-2.0 | âœ… Yes | Email formats, format breadth |
| **Unstructured OSS** | Apache-2.0 | âœ… Yes | Data connectors (S3, SharePoint) |
| **Marker** | GPL-3.0 | âŒ No | Research only; copyleft |
| **PyMuPDF** | AGPL-3.0 | âŒ No | Requires commercial license |
| **LlamaParse** | Proprietary | âš ï¸ API only | Cloud prototyping |

##### 6.1.4.5.2 Performance comparison

| Tool | CPU Speed | GPU Speed | Table Accuracy |
|------|-----------|-----------|----------------|
| **Docling** | 0.8â€“3.1 sec/page | 0.49 sec/page | 97.9% TEDS |
| Marker | 16 sec/page | 0.5 sec/page | High (with LLM) |
| Unstructured | 4.2 sec/page | N/A | 75% (complex) |
| pdfplumber | Fast | N/A | Rule-based only |

##### 6.1.4.5.3 Recommended tool combinations for Handshake

**Primary stack (MIT/Apache-2.0 only):**
- **Docling**: PDF, DOCX, PPTX, XLSX, HTML, images, audio
- **Apache Tika** (Handshake-managed JVM subprocess or library binding): Email (EML, MSG), legacy formats, format detection
- **pdfplumber**: Coordinate debugging, simple table fallback

**Format gap coverage:**

| Missing Format | Recommended Solution |
|----------------|---------------------|
| Email (EML, MSG, MBOX) | Apache Tika or python email/mailbox stdlib |
| Legacy Office (.doc, .xls) | Apache Tika (JVM) or LibreOffice conversion |
| EPUB/MOBI | Calibre CLI or ebooklib |
| OpenDocument | odfpy or Apache Tika |

---

#### 6.1.4.6 Risk assessment

##### 6.1.4.6.1 Technical risks

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| **Memory leaks in batch processing** | High | High | Use Tesseract instead of EasyOCR; pypdfium backend; product-owned worker restart policies |
| **~500MB sidecar bundle size** | Medium | Certain | CPU-only PyTorch reduces by 200MB; lazy model loading |
| **Cross-platform PyInstaller builds** | Medium | Medium | CI matrix for each platform; test on clean VMs |
| **GPU contention with ModelRuntime** | Medium | Medium | CPU-only Docling mode when LLM loaded; sequential processing |

##### 6.1.4.6.2 Licensing risks

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| Model license changes | Low | Low | Pin model versions; monitor releases |
| Dependency license contamination | Medium | Low | Audit with pip-licenses; avoid GPL deps |

##### 6.1.4.6.3 Maintenance risks

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| IBM reduces investment | Low | Low | LF AI governance; RHEL AI commitment |
| Breaking API changes | Medium | Medium | Pin docling version; migration scripts |
| Python version requirements | Low | Low | Supports Python 3.9â€“3.14 |

##### 6.1.4.6.4 Known critical issues (as of December 2025)

- **Memory leak in DoclingParseV2DocumentBackend** (#2209): 10GB+ RAM accumulation on long documents
- **EasyOCR memory leak** (#1343): Container OOM in batch mode
- **Workaround**: Use Tesseract OCR, pypdfium backend, periodic restarts

---

#### 6.1.4.7 Four-week proof-of-concept plan

##### 6.1.4.7.1 Week 1: Core integration validation

| Day | Task | Success Criteria |
|-----|------|------------------|
| 1â€“2 | Set up docling-serve locally; test `/v1/convert/file` endpoint | Convert 10 PDFs via HTTP API |
| 3 | Implement Rust HTTP client with reqwest | Type-safe DoclingDocument deserialization |
| 4â€“5 | Build DoclingDocument â†’ Tiptap JSON converter | Paragraph, heading, list, table, image mapping |

**Deliverable**: Rust library that converts uploaded PDF to ProseMirror JSON

##### 6.1.4.7.2 Week 2: Tauri sidecar integration

| Day | Task | Success Criteria |
|-----|------|------------------|
| 1â€“2 | Create PyInstaller bundle for docling-serve | Single executable runs on macOS/Linux |
| 3 | Configure Tauri sidecar with shell API | Sidecar starts/stops with app |
| 4â€“5 | Implement health check and crash recovery | Auto-restart on failure; graceful shutdown |

**Deliverable**: Tauri app that launches docling-serve sidecar automatically

##### 6.1.4.7.3 Week 3: CRDT and Shadow Workspace

| Day | Task | Success Criteria |
|-----|------|------------------|
| 1â€“2 | Integrate Yjs with atomic document import | Import preserves structure; undo works |
| 3 | Store import provenance in Yjs metadata | Track docling version, source hash |
| 4â€“5 | Implement HybridChunker for embeddings | Chunks with headings context; 512 token limit |

**Deliverable**: Imported documents editable in Tiptap with embedding chunks generated

##### 6.1.4.7.4 Week 4: Production hardening

| Day | Task | Success Criteria |
|-----|------|------------------|
| 1â€“2 | Test memory usage on 100-page documents | Peak RAM < 4GB |
| 3 | Benchmark processing speed | < 3 sec/page on M1 Mac |
| 4 | Error handling for corrupted/password PDFs | Graceful errors; partial results |
| 5 | Cross-platform testing (macOS, Windows, Linux) | All platforms functional |

**Deliverable**: Production-ready integration with documented performance characteristics

##### 6.1.4.7.5 Success metrics for PoC

- [ ] PDF â†’ editable Tiptap document in < 5 seconds for 10-page PDF
- [ ] Table cells with merged spans render correctly
- [ ] Images extracted with bounding boxes for canvas placement
- [ ] Memory usage stays < 4GB for 100-page documents
- [ ] Sidecar bundle size < 600MB
- [ ] Works offline (air-gapped)

---

#### 6.1.4.8 Project health validates long-term adoption

**Viability score: 8.5/10 (Highly Viable)**

| Factor | Score | Notes |
|--------|-------|-------|
| GitHub metrics | 9/10 | 42,200+ stars; weekly releases |
| Documentation | 9/10 | Comprehensive docs, tutorials, examples |
| Governance | 10/10 | LF AI & Data Foundation; IBM Distinguished Engineer chairs board |
| Enterprise backing | 9/10 | RHEL AI 1.3+ supported feature; watsonx.ai integration |
| Known issues | 6/10 | Memory leaks require workarounds |

**Key milestone**: Docling inducted into LF AI & Data Foundation on April 29, 2025, ensuring vendor-neutral governance.

**Red Hat commitment**: RHEL AI 1.3+ includes Docling as a supported feature with enterprise-grade support available through subscription.

---

#### 6.1.4.9 Conclusion: Docling is the right choice for Handshake

Docling provides the optimal balance of **AI-powered accuracy**, **permissive licensing**, and **local-first capability** for Handshake's document processing needs. The MIT license stack enables unrestricted commercial use, and a Handshake-owned in-process or product-managed subprocess integration keeps lifecycle, ports, logs, teardown, and recovery under product control. 

**Primary recommendation**: Proceed with Handshake-managed Docling integration using the four-week PoC plan. Monitor memory usage carefully in batch scenarios and implement restart policies.

**Secondary tools**: Integrate Apache Tika as a Handshake-managed JVM subprocess or library binding for email format support and legacy Office documents. Use pdfplumber for coordinate debugging during development.

**Risk mitigation priorities**: 
1. Use Tesseract over EasyOCR to avoid memory leaks
2. Pin to pypdfium backend for large documents
3. Implement sidecar health monitoring with auto-restart

---

### 6.1.5 Part III â€“ Architectural Evaluation of IBM Docling for the Handshake Workspace
#### 6.1.5.1 Architectural Evaluation of IBM Docling for the Handshake Workspace
##### 6.1.5.1.1 Executive Summary

The "Handshake" initiative represents a strategic pivot in desktop workspace design, prioritizing local-first data processing, user privacy, and high-performance interaction through the Rust and Tauri ecosystem. A fundamental requirement for this workspace is the ability to ingest, parse, and semantically understand a vast array of unstructured documentsâ€”ranging from academic PDFs and financial spreadsheets to scientific manuscripts and legacy archives. The objective of this report is to provide an exhaustive architectural evaluation of IBM Docling, an open-source document processing library, to determine its viability as the core ingestion engine for Handshake.
The analysis reveals that Docling represents a significant advancement in document processing technology, moving beyond traditional Optical Character Recognition (OCR) and text extraction towards a holistic, layout-aware understanding of document structure.1 Powered by specialized AI models such as DocLayNet for layout segmentation and TableFormer for table structure recognition, Docling is capable of reconstructing the hierarchical organization of documents with a fidelity that standard parsers cannot match.3 This capability is critical for the "AI-enhanced" aspect of Handshake, as it enables Retrieval-Augmented Generation (RAG) systems to retrieve information with semantic contextâ€”distinguishing between a footnote, a header, and a table cellâ€”thereby significantly reducing hallucination rates in downstream Large Language Model (LLM) tasks.
From an integration perspective, Doclingâ€™s Python-centric architecture presents a challenge for the Rust-based Handshake environment. However, this report identifies the Sidecar Pattern as a robust solution, allowing the robust encapsulation of the Python runtime while maintaining the performance and safety guarantees of the Rust frontend.5 Furthermore, Doclingâ€™s MIT License 7 offers a decisive commercial advantage over GPL-licensed competitors like Marker, ensuring that Handshake faces no legal barriers to distribution or monetization.
While Docling excels in processing modern formats and providing deep semantic structure, it exhibits notable gaps in legacy format support (e.g., .doc, .xls, .eml) and requires significant computational resources for optimal performance.8 Consequently, this report recommends a hybrid architecture: utilizing Docling as the primary semantic engine, supplemented by a transcoding layer for legacy compatibility and a background job management system to mitigate latency on consumer hardware. This strategy positions Handshake to deliver a state-of-the-art, privacy-preserving document intelligence experience.

##### 6.1.5.1.2 The Imperative of Layout-Aware Document Understanding

To appreciate the necessity of a tool like Docling within the Handshake ecosystem, one must first confront the limitations of traditional document processing. For decades, the industry standard for parsing PDFs and office documents has focused on the extraction of text streamsâ€”simply pulling character codes from the file in the order they appear in the underlying binary. While computationally inexpensive, this approach destroys the semantic fabric of the document.

###### The Failure of Linear Text Extraction

In a linear extraction paradigm, a multi-column scientific paper is often rendered as a jumbled sequence of sentences where the end of column A flows directly into the start of column B, disregarding the visual boundary. Headers are inextricably mixed with body text, and tables are reduced to unintelligible streams of alphanumeric characters. For a modern AI workspace like Handshake, this loss of structure is catastrophic. An LLM attempting to answer a user's query based on such data lacks the necessary context to determine if a number belongs to the "Revenue" column or the "Year" column of a financial table.

###### The Computer Vision Paradigm

Docling fundamentally diverges from this legacy approach by treating document processing as a computer vision problem first, and a text parsing problem second. This philosophy acknowledges that the "meaning" of a document is encoded not just in its words, but in its visual layoutâ€”the spatial relationships between blocks of text, the distinct formatting of headers, and the grid structure of tables.
The core of Doclingâ€™s architecture is a sophisticated pipeline that employs object detection models to segment the page into semantic regions before any text is processed.2 This ensures that the logical reading order is preserved based on visual cues rather than arbitrary binary stream order. By leveraging models trained on the DocLayNet datasetâ€”a massive corpus of human-annotated documents covering diverse domains like finance, law, and scienceâ€”Docling achieves a level of generalization that allows it to handle the "wild" diversity of user documents expected in a desktop workspace.4

##### 6.1.5.1.3 Architectural Analysis of the Docling Pipeline

The efficacy of Docling lies in its modular pipeline architecture, which orchestrates a series of specialized AI models and heuristics to transform raw pixels and binary data into structured knowledge.

###### The Layout Analysis Engine: DocLayNet

The entry point for Doclingâ€™s semantic understanding is the Layout Model. Utilizing architectures such as RT-DETR (Real-Time Detection Transformer) or proprietary IBM implementations, this model scans the rasterized page image to identify bounding boxes for various document elements.11
The classification taxonomy used by DocLayNet is particularly relevant for Handshakeâ€™s RAG capabilities. It distinguishes between:
Narrative Text: The primary content, which should be indexed for vector search.
Headers and Footers: "Furniture" elements that often introduce noise into search results and should be excluded or tagged as metadata.
Figures and Captions: Visual elements that require distinct processing, such as passing to a Vision-Language Model (VLM) for description.12
Tables: Complex structures that trigger a specialized sub-pipeline.
This granularity allows Handshake to implement sophisticated indexing strategies. For instance, a user could filter search results to show only "Figures containing the word 'Architecture'," a query impossible with simple text extraction.

###### Deep Structure Recognition: TableFormer

Perhaps the most significant technical differentiator for Docling is its integration of TableFormer, a specialized transformer model designed to solve the intractable problem of table extraction.4 Tables in PDFs are notoriously difficult because they lack explicit structural tags; they are merely collections of lines and floating text.
TableFormer approaches this challenge by predicting the logical structure of the table (rows, columns, spanning cells) directly from the visual representation. It effectively reconstructs the HTML-like grid of the table, correcting for irregularities like merged cells or invisible borders that defeat rule-based parsers. Benchmarks indicate that this approach yields a table cell accuracy of 97.9%, significantly outperforming competitive solutions like Unstructured.io or LlamaParse in maintaining structural integrity.14 For Handshake users dealing with financial reports or technical specifications, this capability transforms static, locked data into queryable, machine-readable datasets.

###### The Role of Vision-Language Models (VLMs)

Recent architectural updates have introduced the capability to integrate IBM Granite, a dedicated Vision-Language Model, into the Docling pipeline.12 Unlike the standard pipeline which cascades multiple specialized models (Layout -> Table -> OCR), the Granite-Docling model (258M parameters) attempts an end-to-end transformation, predicting the document structure directly from the image inputs.
While the standard pipeline is likely sufficient for general text documents, the VLM capability offers a future-proof path for handling highly complex or graphical documents. It enables "Visual Grounding," where the model can generate descriptive captions for images and charts, effectively making the visual content of a document searchable via natural language.16 For a local-first application, the modularity of Docling allows Handshake to potentially offer this as an "Advanced Processing" toggle, downloading the heavier VLM weights only for users with capable hardware (e.g., Apple Silicon or NVIDIA GPUs).

##### 6.1.5.1.4 The Unified Data Model: DoclingDocument

The output of the Docling pipeline is not a proprietary binary or a simple text file, but a rich, strongly-typed object model known as the DoclingDocument. Defined using Pydantic, this data structure serves as the intermediate representation (IR) for all processing within the ecosystem.17

###### Hierarchical vs. Flat Representation

The DoclingDocument is designed to satisfy two distinct access patterns:
The Hierarchical Tree: The body field contains a nested tree structure representing the document's logical organization (Sections containing Subsections containing Paragraphs). This is essential for rendering the document in the Handshake UI, preserving the "Table of Contents" structure and reading flow.17
The Flat Lists: The document also exposes flattened lists of items (texts, tables, pictures). This design is crucial for the indexing layer of Handshake. It allows the Rust backend to iterate rapidly over every text paragraph to generate embeddings, without needing to traverse the complex recursive tree structure.17

###### JSON Serialization and Rust Interoperability

The bridge between Docling (Python) and Handshake (Rust) is JSON. The DoclingDocument serializes to a JSON schema that utilizes JSON Pointers (e.g., "$ref": "#/texts/12") to link items in the tree to their definitions in the flat lists.17
This referencing mechanism is a sophisticated solution to data duplication, ensuring that a text item appearing in multiple logical groups is stored only once. However, it imposes a requirement on the Rust side: the deserialization logic must be capable of resolving these pointers. While there is no native Rust crate for Docling at present (the docling-core library is Python-based), the schema is stable and well-documented, allowing the engineering team to generate compatible Rust structs using serde and serde_json.
The structure of the exported data is lossless, meaning that all metadataâ€”including bounding boxes (prov), page numbers, and confidence scoresâ€”is preserved.19 This allows the Handshake UI to implement features like "Click-to-Source," where highlighting a search result instantly scrolls the original PDF view to the exact coordinate location of the text.

##### 6.1.5.1.5 Integration Architecture: Bridging Rust and Python

The primary engineering challenge in adopting Docling for Handshake is the ecosystem mismatch: Tauri applications are fundamentally Rust binaries managing a WebView, whereas Docling is a complex Python dependency tree. To reconcile this, we must employ the Sidecar Pattern.

###### The Sidecar Pattern: Architecture and Implementation

In the context of Tauri, a "Sidecar" is an external binary bundled with the application that runs as a subprocess. This pattern isolates the Python environment, preventing the heavy ML libraries from bloating the main application memory space or causing instability in the UI thread.5
The recommended implementation strategy involves packaging the Docling environment into a standalone executable using PyInstaller or Nuitka. This "frozen" binary contains the Python interpreter, the Docling library, and all necessary dependencies (PyTorch, Pillow, etc.).
Data Flow Architecture:
Trigger: The user drops a PDF into Handshake.
Command: The Rust main process invokes the Sidecar binary via the tauri::shell::Command API, passing the file path as an argument.
Processing: The Sidecar initializes the Docling pipeline, loads the necessary models (lazily, to conserve RAM), and processes the file.
Output: The Sidecar writes the resulting JSON to a temporary file or streams it to stdout.
Ingestion: Rust reads the JSON, deserializes it into internal structs, and populates the local SQLite database and Vector Index.

###### Inter-Process Communication (IPC) Strategies

While standard input/output (stdio) is the simplest IPC mechanism, transferring large JSON payloads (which can exceed 10MB for large reports) via stdout can encounter buffer limitations or serialization overhead.
A more robust approach for Handshake is to use File-Based IPC or Local Sockets. In the file-based approach, the Sidecar writes the output to a temporary JSON file and simply returns the file path to Rust. This decouples the serialization speed from the pipe bandwidth and simplifies debugging (as the JSON files can be inspected). For a more advanced implementation, the Sidecar could run as a persistent daemon (using docling-serve), exposing a local HTTP server that Rust communicates with.21 This avoids the overhead of spinning up the Python interpreter for every single document, significantly improving performance for batch imports.

###### Licensing and Distribution: The MIT Advantage

A critical factor in the selection of Docling is its MIT License.7 In the landscape of open-source document AI, this is a distinct competitive advantage. Major competitors like Marker and PyMuPDF (in its newer iterations) often carry GPL or AGPL licenses, which effectively mandate that any application linking to them must also be open-source.22
For a commercial or proprietary desktop application like Handshake, a GPL dependency is a "poison pill." Doclingâ€™s permissive MIT license allows the Handshake team to bundle, modify, and distribute the engine without any obligation to release the source code of the wider application. This legal safety, combined with the technical capability, makes Docling the only viable option for a closed-source or open-core business model.

##### 6.1.5.1.6 Performance Profile and Resource Management

"Local-first" implies that the application must run on the hardware the user has, not the hardware we wish they had. Docling is computationally intensive, and its performance profile dictates specific architectural decisions.

###### Hardware Acceleration: CPU vs. GPU vs. MPS

The performance disparity between CPU and GPU execution is stark. Benchmarks indicate that processing a single PDF page can take approximately 3.1 seconds on a standard x86 CPU, compared to 0.49 seconds on an NVIDIA L4 GPU.8 On Apple Silicon (M3 Max), utilizing the Metal Performance Shaders (MPS), the speed is around 1.27 seconds per page.
This variability means that on a standard corporate laptop without a dedicated GPU, processing a 50-page report could take nearly three minutes. This latency is too high to be a blocking operation. Therefore, Handshake must implement a background job queue. The UI should reflect a "Processing" state, allowing the user to continue working while the Sidecar churns through the document queue in the background.

###### Comparison with Competitors

Despite the heavy resource usage, Docling is comparatively efficient in the realm of Deep Learning parsers. Benchmarks show it outperforming Marker (which takes ~16 seconds per page on CPU) and Unstructured (which is often slower and less accurate on tables).8 While heuristic tools like pdftotext are instantaneous, they fail completely on the structural understanding tasks required by Handshake. Docling represents the optimal trade-off: acceptable speed for high-fidelity semantic data.

###### RAM and VRAM Considerations

Loading the full suite of Docling models (Layout, TableFormer, OCR) can consume 2GB to 4GB of RAM.24 For a desktop app, this is a significant footprint. The integration strategy must therefore include lifecycle management for the Sidecar. The Python process should not run permanently; it should be spawned when ingestion tasks are queued and terminated after a timeout period of inactivity, returning resources to the user's system.

##### 6.1.5.1.7 Format Support and the Legacy Compatibility Gap

A universal workspace must handle universal formats. Docling excels with the modern stack but has a blind spot for legacy files.

###### The Modern Suite

Docling provides native, high-fidelity support for:
PDF: Including complex layouts, scanned images, and scientific papers.
Office Open XML: .docx (Word), .xlsx (Excel), .pptx (PowerPoint) are parsed directly from their XML structure, ensuring 100% text accuracy.19
Web & Text: HTML, Markdown, and AsciiDoc are supported, treating web clips as first-class citizens.

###### The Legacy Gap and Mitigation

Crucially, Docling does not support legacy Microsoft binary formats (.doc, .xls, .ppt) or email formats (.msg, .eml).9 For an enterprise user base, this is a critical deficiency.
To mitigate this, Handshake must implement a Transcoding Layer. The most robust solution is to integrate or bundle a headless version of LibreOffice. Before a file reaches the Docling pipeline, the Rust backend should detect legacy MIME types and trigger a conversion:
soffice --headless --convert-to docx legacy_file.doc
The resulting temporary modern file is then fed to Docling. Similarly, for emails, a lightweight Python library like extract-msg should be added to the Sidecar to parse .msg files into text or Markdown, which Docling can then ingest.26

##### 6.1.5.1.8 Enhancing RAG with Structured Data

The ultimate value of integrating Docling lies in the quality of the data it feeds into the Handshake RAG system.

###### Semantic Chunking

Standard RAG pipelines use "naive chunking"â€”splitting text every 500 characters. This often destroys context, splitting a table header from its rows or a section title from its paragraphs.
Doclingâ€™s hierarchical DoclingDocument allows for Semantic Chunking. Handshake can iterate through the document tree, creating chunks that respect logical boundaries. A chunk can be defined as "One Section" or "One Table." Furthermore, because the tree preserves parentage, every chunk can be enriched with its context path (e.g., "Annual Report 2023 > Q4 Financials > Revenue Table"). This "Metadata Enrichment" significantly improves the retrieval accuracy of the Vector Database, allowing the LLM to understand exactly where a piece of information came from.

###### Visual Grounding and Multi-Modality

Doclingâ€™s support for VLM-based captioning means that images and charts are no longer black holes to the search engine. By generating textual descriptions of visual elements, Docling allows Handshake users to perform semantic searches over charts ("Show me the graph depicting rising inflation")â€”a feature that distinguishes Handshake from standard file explorers.16

##### 6.1.5.1.9 Conclusion and Strategic Recommendation

Following a comprehensive architectural evaluation, IBM Docling is strongly recommended as the ingestion engine for the Handshake workspace. Its ability to provide deep, layout-aware understanding of documents aligns perfectly with the project's goal of delivering an AI-enhanced, local-first experience.
While the integration imposes engineering overheadâ€”specifically regarding the Python Sidecar implementation and the need for a legacy transcoding layerâ€”the benefits in data fidelity and structural comprehension are unmatched by other open-source tools. The MIT license secures the commercial future of the application, and the active development by IBM Research suggests a robust roadmap for future capabilities, including advanced VLM integrations. By adopting Docling, Handshake will not merely "read" documents; it will "understand" them, providing a foundation for truly intelligent user interactions.

###### Table 1: Feature Comparison of Document Processing Engines
|Feature|IBM Docling|Marker|Unstructured.io|LlamaParse|
|---|---|---|---|---|
|Primary Approach|Computer Vision / Layout Analysis|Deep Learning / Text Sequence|Hybrid (Rules + Vision)|Proprietary / Cloud API|
|Table Extraction|Excellent (TableFormer model)|Good|Moderate|Good|
|License|MIT (Permissive)|GPL (Restrictive)|Apache 2.0|Proprietary (Paid Service)|
|Local Execution|Yes (Full)|Yes|Yes|No (Cloud First)|
|Legacy Support|No (.doc, .xls unsupported)|No|Limited|Yes (via Cloud)|
|Speed (CPU)|Moderate (~3s/page)|Slow (~16s/page)|Slow|N/A (Cloud latency)|
|Output Model|Structured Hierarchical JSON|Markdown|JSON Elements|Markdown|
###### Table 2: Benchmark Performance (Time per Page)
8
|Hardware Configuration|Docling|Marker|Unstructured|
|---|---|---|---|
|NVIDIA L4 GPU|0.49s|0.86s|N/A|
|Apple M3 Max (MPS)|1.27s|4.20s|2.70s|
|Standard x86 CPU|3.10s|16.0s|4.20s|

###### Works cited
Docling Project - GitHub, accessed December 2, 2025, 
Documentation - Docling - GitHub Pages, accessed December 2, 2025, 
A new tool to unlock data from enterprise documents for generative AI - IBM Research, accessed December 2, 2025, 
Visual grounding - Docling, accessed December 2, 2025, 
Embedding External Binaries - Tauri, accessed December 2, 2025, 
Writing a pandas Sidecar for Tauri | MClare Blog, accessed December 2, 2025, 
docling/LICENSE at main - GitHub, accessed December 2, 2025, 
Docling Technical Report - arXiv, accessed December 2, 2025, 
.DOC is not supported Â· Issue #2293 Â· docling-project/docling - GitHub, accessed December 2, 2025, 
Docling AI: A Complete Guide to Parsing - Codecademy, accessed December 2, 2025, 
docling-project/docling-models - Hugging Face, accessed December 2, 2025, 
IBM Granite-Docling: Super Charge your RAG 2.0 Pipeline, accessed December 2, 2025, 
docling-project/docling-ibm-models - GitHub, accessed December 2, 2025, 
PDF Data Extraction Benchmark 2025: Comparing Docling, Unstructured, and LlamaParse for Document Processing Pipelines - Procycons, accessed December 2, 2025, 
IBM Granite-Docling:. In recent years, many discussions inâ€¦ | by Nandini Lokesh Reddy | Oct, 2025, accessed December 2, 2025, 
Docling: Make your Documents Gen AI-ready - GeeksforGeeks, accessed December 2, 2025, 
Docling Document - GitHub Pages, accessed December 2, 2025, 
My first hands-on experience with Docling | by Alain Airom (Ayrom) - Medium, accessed December 2, 2025, 
Supported formats - Docling - GitHub Pages, accessed December 2, 2025, 
Embedding External Binaries | Tauri v1, accessed December 2, 2025, 
docling-project/docling-serve: Running Docling as an API service - GitHub, accessed December 2, 2025, 
datalab-to/marker: Convert PDF to markdown + JSON quickly with high accuracy - GitHub, accessed December 2, 2025, 
PymuPDF licensing requirements when its a dependency of another dependency? - Reddit, accessed December 2, 2025, 
Recommended Server Specs Â· Issue #385 Â· docling-project/docling-serve - GitHub, accessed December 2, 2025, 
Unlock Your Data: Supercharge Excel Document Processing with Docling, accessed December 2, 2025, 
Email - Docs by LangChain, accessed December 2, 2025, 

---

### 6.1.6 Docling AI Job Profile

**Why**  
Document ingestion is a key AI capability that needs the same job model, provenance, and validation guarantees as editing. Defining it as a profile ensures consistent behavior and auditability.

**What**  
Defines the Docling-specific AI job profile: profile-specific fields, PlannedOperation types, provenance structure, validation rules, and typical job flow.

**Jargon**  
- **source_file_id**: Reference to the external file being ingested.
- **DocLayNet**: Layout analysis model used for page segmentation.
- **TableFormer**: Table extraction model for structured table recovery.
- **source_bbox**: Bounding box coordinates enabling "click-to-source" navigation.

**Implements:** AI Job Model (Section 2.6.6)  
**Profile ID:** `docling_ingest_v0.1`

This profile governs AI jobs that ingest external documents into the Handshake workspace using the Docling pipeline.

#### 6.1.6.1 Profile-Specific Fields

| Field | Type | Description |
|-------|------|-------------|
| `source_file_id` | FileId | Reference to the source file being ingested |
| `target_doc_id` | DocId | Target workspace document to create/populate |
| `ingest_mode` | IngestMode | `full_structure`, `text_only`, `tables_only` |
| `layout_model` | ModelId | Layout analysis model to use (e.g., DocLayNet) |
| `table_model` | ModelId | Table extraction model to use (e.g., TableFormer) |

#### 6.1.6.2 PlannedOperation Types

| Operation | Description |
|-----------|-------------|
| `extract_structure(source_file_id, layout_model)` | Run layout analysis on source file |
| `extract_tables(source_file_id, table_model)` | Extract tables with structure |
| `import_blocks(source_file_id, target_doc_id, block_mapping)` | Import extracted content as workspace blocks |
| `link_provenance(source_file_id, target_doc_id, segment_mapping)` | Establish provenance links |

#### 6.1.6.3 Provenance

Each imported block carries `ai_origin`:
- `job_id`: The ingestion job
- `source_file_id`: Original file reference
- `source_page`: Page number in original
- `source_bbox`: Bounding box coordinates for "click-to-source"
- `extraction_confidence`: Model confidence score

#### 6.1.6.4 Validation Rules

| Validator | Purpose |
|-----------|---------|
| `file_exists` | Source file is accessible |
| `target_writable` | Target document is not locked |
| `format_supported` | File format is in Docling's supported set |
| `resource_available` | GPU/memory available for extraction |

#### 6.1.6.5 Typical Job Flow

```
1. queued          â†’ File uploaded, basic checks passed
2. running         â†’ Docling sidecar processing file
3. awaiting_validation â†’ Extraction complete, preview available
4. awaiting_user   â†’ User reviews extracted structure
5. completed       â†’ Blocks imported, provenance written
```

---

**Key Takeaways**  
- Docling ingestion jobs are AI jobs under the global model ((AI Job Model, Section 2.6.6)).
- The profile adds source/target file references and extraction modes.
- Provenance links imported blocks back to source file coordinates.
- Users review extracted structure before import commits.

---

### 6.1.11 Chunking, Embeddings, and Indexing Config (Docling + Shadow Workspace)
- Define chunk size/overlap and embedding format/schema for Docling-ingested content; specify index schema (ids, doc/block ids, embeddings, metadata) and search metrics.
- Provide defaults and tunables for chunking per modality; log ingestion metrics (chunk counts, embed latency) into Flight Recorder/Shadow Workspace stats.
- Establish acceptance criteria/benchmarks for ingestion throughput and search quality in Phase 2.

## 6.2 Speech Recognition: ASR Subsystem

**Why**  
Handshake needs to transcribe long-form audio (lectures, meetings, screen recordings) into searchable, AI-accessible text. Local-first ASR ensures privacy and offline capability.

**What**  
Specifies ASR goals, model landscape, architecture, audio handling, customization policy, privacy, UX, and evaluation framework.

Note: Performance/latency/VRAM figures and model recommendations in this section must be refreshed against current 2026 toolchains (Whisper/Nemo/etc.) and target hardware; update benchmarks and guidance.

[ADD v02.158] ASR transcript outputs are canonical backend artifacts, not only UI text. Transcript payloads, source media hashes, ffprobe-derived media facts, timing anchors, and bounded failure/progress events MUST remain portable across storage/export flows and recorder-visible for replay, troubleshooting, and later Loom/Lens consumption.

[ADD v02.158] Media Downloader outputs are canonical ASR inputs when policy allows. Stage-captured/imported media and richer Lens/Studio transcript-time-span bridges remain stub-backed until explicit lineage contracts exist, but the backend lineage requirement itself is normative now.

**Jargon**  
- **ASR**: Automatic Speech Recognition.
- **WER**: Word Error Rateâ€”primary accuracy metric.
- **RTF**: Real-Time Factorâ€”latency measure (RTF < 0.5 means 2x faster than real-time).
- **Whisper**: OpenAI's multilingual ASR model (MIT license).
- **Faster-Whisper**: CTranslate2-based optimized Whisper runtime.
- **whisper.cpp**: C++ port for CPU inference with quantization.
- **VAD**: Voice Activity Detectionâ€”identifies speech vs silence.
- **Diarization**: Identifying who spoke when (speaker separation).

---
### 6.2.1 X.1 Goals, Scope, and Constraints

#### 6.2.1.1 Problem Statement

Handshake needs a local-first, high-quality automatic speech recognition (ASR) pipeline that can transcribe long-form audio and video recordings (lectures, meetings, screen recordings) into text. These transcripts must integrate cleanly into the existing Orchestrator, Model Runtime Layer, and Shadow Workspace so that they can be searched, summarized, and used by other AI tools.

#### 6.2.1.2 Non-Goals

For the initial phases, Handshake ASR is **not**:

- A real-time conferencing / live captioning solution.
- A certified transcription system for regulated domains (healthcare, legal records, court reporting).
- A hosted ASR SaaS product.

#### 6.2.1.3 User and Workload Assumptions

- Primary workloads:
  - 1â€“3 hour university lectures.
  - 30â€“120 minute meetings and screen recordings.
- Users are technical or power users comfortable running a local desktop app on a high-end workstation.
- Transcripts are primarily used for:
  - Search and navigation.
  - Summaries and note-taking.
  - Q&A and retrieval-augmented generation (RAG).

#### 6.2.1.4 Hardware Assumptions (Reference Workstation)

The reference development machine for ASR is:

- CPU: AMD Ryzen 9â€“class 16-core CPU.
- RAM: 64â€“128 GB.
- GPU: NVIDIA RTX 3090 (24 GB VRAM) or equivalent.
- Storage: Fast NVMe SSD.

All performance targets and model tiering rules assume this baseline. On weaker machines, the runtime must degrade gracefully by switching to smaller or quantized models and/or CPU-only execution.

### 6.2.2 X.2 Model Landscape and Selection Rationale

#### 6.2.2.1 Open-Source ASR Landscape (Summary)

Handshake primarily targets the open-source ASR ecosystem. The main model families considered are:

- **Whisper**: General-purpose multilingual ASR with strong performance across many languages and domains, large ecosystem support, and efficient runtimes (Faster-Whisper, whisper.cpp).
- **NeMo / Parakeet / Conformer**: High-quality ASR families from NVIDIA and others, usually optimized for server deployments with rich features (streaming, word boosting, etc.).
- **Multilingual / low-resource families**: MMS / Omni-style models, Shunya Pingala, and similar systems targeting many languages.
- **Language-specialised frameworks**: PaddleSpeech, WeNet, SpeechBrain, ESPnet, and others that provide strong models for Mandarin and other specific languages.

Whisper is chosen as the **primary** ASR foundation because of:

- Strong accuracy for English and many other languages.
- Good support for long-form audio.
- Mature community runtimes that run well on desktops.
- Permissive licensing for local inference.

Other families are treated as complementary or experimental options, especially for Mandarin and low-resource languages.

#### 6.2.2.2 Commercial / Cloud ASR (For Comparison Only)

Commercial ASR providers (e.g. big cloud vendors) offer:

- High accuracy for common languages.
- Enterprise features (speaker diarization, PII redaction, compliance certifications).
- Managed infrastructure and SLAs.

However, they conflict with Handshakeâ€™s core requirements:

- Local-first, offline-capable operation.
- No mandatory dependence on cloud services.
- Fine-grained user control over where data flows.

Cloud ASR may be used as an **optional, explicit fallback** for users who choose to enable it, but it is not part of the default design.

#### 6.2.2.3 Design Criteria for Handshake

Model selection is driven by:

- **Accuracy**: Word Error Rate (WER), especially on long-form lecture and meeting content.
- **Latency / Throughput**: Real-time factor (RTF) on the reference workstation.
- **Resource Usage**: VRAM and CPU utilization; ability to coexist with LLMs and image models.
- **Licensing**: Ability to bundle or download models legally for local use.
- **Ecosystem Health**: Active runtimes, community support, and long-term maintainability.

These criteria feed directly into the default model tiering defined in X.2.4.

#### 6.2.2.4 Handshake Default ASR Model Tiering

This section defines the **default ASR model stack and selection rules** for the Handshake reference workstation.

##### 6.2.2.4.1 X.2.4.1 Hardware assumptions

All defaults in this section assume the reference development machine:

- CPU: AMD Ryzen 9â€“class 16-core desktop CPU
- RAM: â‰¥ 64â€“128 GB system memory
- GPU: NVIDIA RTX 3090 (24 GB VRAM) or equivalent
- Storage: Fast SSD (NVMe recommended)

If the userâ€™s hardware is weaker, the runtime **MUST** degrade gracefully by switching to smaller or quantized models as defined below.

##### 6.2.2.4.2 X.2.4.2 Model roles

Handshake distinguishes the following ASR â€œrolesâ€:

1. **Primary general-purpose model**  
   Used for most workloads (lectures, meetings, videos) when GPU resources are available.

2. **Fast / low-resource mode**  
   Used when GPU VRAM is constrained, when the GPU is busy with other tasks, or on CPU-only machines.

3. **Language-specialized options**  
   Optional models for languages where specialized ASR significantly outperforms general multilingual models.

4. **Experimental / research models**  
   Models that are available for benchmarking and experimentation but not enabled by default.

##### 6.2.2.4.3 X.2.4.3 Default model set

The **default Handshake ASR configuration** SHALL use the following model set:

1. **Primary general-purpose model (GPU)**  
   - Model family: OpenAI Whisper  
   - Variant: `large-v3` (or `large-v3-turbo` equivalent)  
   - Runtime: Faster-Whisper (CTranslate2)  
   - Precision: FP16 on GPU  
   - Role:
     - Default for English and general multilingual transcription
     - Expected to cover: English, Chinese, Dutch, Korean, Japanese, Arabic, Russian â€œwell enoughâ€ for non-regulated use

2. **Fast / low-resource model**  
   - Model family: OpenAI Whisper  
   - Variant: `small` or `medium` (exact choice configurable; `small` is default)  
   - Runtime:
     - GPU: Faster-Whisper with INT8 or mixed-precision
     - CPU fallback: whisper.cpp with 4â€“6-bit quantization  
   - Role:
     - Used when the GPU is unavailable or heavily loaded
     - Used for quick-and-dirty transcription where latency matters more than accuracy
     - Used as a safety fallback if large-v3 fails to load

3. **Mandarin-specialised option (optional)**  
   - Candidate families: PaddleSpeech, WeNet (Mandarin-focused conformer models)  
   - Runtime: Project-specific integration behind the same ASR service interface  
   - Role:
     - **Not enabled by default** in the MVP
     - MAY be enabled as an experimental â€œalt engineâ€ for Mandarin benchmarks
     - Used only when the user explicitly selects a Mandarin-specialised profile or during internal evaluation

4. **Experimental multilingual / low-resource models (optional)**  
   - Candidate families: MMS/Omni-style models, Shunya Pingala, SpeechBrain/ESPnet research checkpoints, etc.  
   - Role:
     - **Never used by default** for end users
     - Exposed only behind a developer / diagnostic flag
     - Used for offline research on accents, low-resource languages, and future model swaps

##### 6.2.2.4.4 X.2.4.4 Runtime selection policy

The ASR runtime **MUST** implement a simple, deterministic selection policy:

1. **GPU-happy path**  
   - If:
     - A compatible GPU is present, and  
     - Available VRAM â‰¥ a configurable threshold (default: 8â€“10 GB free), and  
     - The GPU job queue is not saturated,  
   - THEN:
     - Use **Whisper large-v3 via Faster-Whisper (FP16)** as the primary engine.

2. **GPU-constrained path**  
   - If:
     - GPU exists but free VRAM is below threshold, or  
     - The GPU has an active high-priority job (e.g. image model),  
   - THEN:
     - Prefer **Whisper small via Faster-Whisper (INT8/mixed precision)** on GPU.  
     - If that still cannot load or runs out of memory, fall back to CPU mode.

3. **CPU-only path**  
   - If **no compatible GPU is detected**:
     - Use **Whisper small via whisper.cpp** with 4â€“6-bit quantization.  
     - For very long jobs, chunk audio more aggressively to keep memory bounded.

4. **Language-specialised override (optional)**  
   - If the user or a configuration profile explicitly selects a language-specialised engine (e.g. Mandarin profile):
     - Route segments tagged as that language to the configured specialised model.
     - All other segments still use the primary Whisper path.

5. **Cloud fallback (if enabled)**  
   - Cloud ASR **MUST be disabled by default**.  
   - If the user explicitly opts in to cloud fallback, the runtime MAY:
     - Retry failed segments or low-confidence segments with a configured cloud ASR provider.
     - Clearly annotate transcript segments that originate from cloud ASR.

##### 6.2.2.4.5 X.2.4.5 Configuration and observability

- The active model, runtime, and selection path **MUST** be visible in:
  - Developer logs, and
  - A debug/status panel in the app.
- The user **MUST** be able to:
  - Force â€œfast modeâ€ (small model) for low-latency runs.
  - Opt out of experimental/alternative models entirely.
- Model version and configuration **MUST** be recorded as part of the transcript metadata to support reproducibility and regression testing.
### 6.2.3 X.3 Handshake ASR Architecture

This section defines how ASR is integrated into the Handshake system: dataflow, components, interfaces, and how transcripts enter the workspace and Shadow Workspace.

#### 6.2.3.1 High-Level Dataflow

At a high level, ASR in Handshake follows this pipeline:

1. **Source selection**
   - User selects a source resource:
     - An existing audio/video file in the workspace
     - A newly recorded audio/video capture (screen + system audio, microphone, etc.)
   - The source is represented as a **RawContent** resource with a stable ID in the main workspace.

2. **Ingestion request**
   - The desktop client sends a **Transcription Job Request** to the Orchestrator, referencing:
     - Source resource ID
     - Desired language / language hints
     - Priority and quality profile (e.g. â€œfull qualityâ€, â€œfast modeâ€)
   - The request is enqueued in the Orchestratorâ€™s job queue.

3. **Audio extraction and segmentation**
   - The Orchestrator:
     - Extracts audio streams from the source using `ffmpeg` (or equivalent).
     - Normalises format (mono, 16 kHz, 16-bit PCM WAV).
     - Segments audio into manageable chunks using VAD/silence detection and/or fixed windows with overlap.

4. **ASR inference**
   - Segments are dispatched to the **ASR service** (a specific Model Runtime) over a local API.
   - The ASR service returns per-segment transcripts with timestamps and confidence scores.

5. **Assembly and post-processing**
   - The Orchestrator:
     - Assembles segment-level transcripts into a full transcript for the source.
     - Applies light deterministic cleanup (e.g. punctuation, basic casing) as configured.

6. **Storage and Shadow Workspace integration**
   - The final transcript is persisted as **DerivedContent** associated with the source resource.
   - Shadow Workspace ingests the transcript:
     - Indexes it for search and navigation.
     - Exposes it to LLM tools (summaries, Q&A, etc.).
   - The UI updates to show transcript status and results.

The above flow **MUST** be deterministic and reproducible: given the same source, configuration, and model version, the system **SHOULD** produce the same transcript (modulo nondeterminism in beam search).

#### 6.2.3.2 Integration with the Orchestrator and Model Runtime Layer

The ASR subsystem is implemented as one more **Model Runtime** behind the Orchestrator:

1. **Orchestrator responsibilities**
   - Own the high-level transcription workflow:
     - Accept and validate transcription job requests
     - Manage job queue and priorities
     - Orchestrate extraction, segmentation, inference, and assembly
   - Track job state (queued, running, completed, failed) and progress metadata.
   - Expose a stable API to the desktop client.

2. **Model Runtime Layer responsibilities**
  - Provide a **Handshake-owned execution boundary** for ASR models:
     - A dedicated ASR service binary, in-process binding, WASM module, or product-managed subprocess separate from the Orchestrator. Containers are compatibility-only opt-ins.
   - Implement a narrow, versioned API such as:
     - `POST /v1/asr/transcribe_segments`  
       Request: array of audio segments + config  
       Response: array of transcripts + timing + confidences
   - Handle model loading, GPU/CPU selection, batching, and low-level optimizations.

3. **Communication pattern**
   - The Orchestrator **MUST** treat ASR as a black-box service:
     - No direct model invocation within the Orchestrator process.
     - All calls via explicit HTTP/gRPC or an equivalent IPC protocol.
   - This enables:
     - Swapping ASR engines without changing the Orchestrator
     - Running ASR in a separate process / sandbox if needed

4. **Error handling and resilience**
   - The Orchestrator **MUST**:
     - Handle ASR service unavailability (retry with backoff, surface clear errors)
     - Support partial results (segments completed before failure)
     - Record errors and model metadata in job logs for debugging

#### 6.2.3.3 Audio Ingestion (Files, Recordings, System Audio)

The ASR architecture assumes multiple audio sources, but a single abstraction:

1. **Unified â€œmedia sourceâ€ abstraction**
   - Every ASR job references a **MediaSource** object, which encapsulates:
     - Workspace resource ID
     - Physical file path (local)
     - Media type (audio-only / audio+video)
     - Recording metadata (start time, duration, origin)

2. **Supported ingestion modes (MVP)**
   - MVP **MUST** support:
     - Importing existing audio/video files in the workspace
     - Transcribing newly captured recordings created within Handshake
   - Additional modes (e.g. live system audio capture) are **MAY** for later phases and **MUST NOT** complicate the core pipeline.

3. **Extraction requirements**
   - The Orchestrator **MUST**:
     - Use `ffmpeg` (or equivalent) to extract one or more audio streams from the source.
     - Normalize to:
       - Mono (or well-defined channel handling)
       - 16 kHz sample rate
       - 16-bit PCM WAV
     - Enforce a maximum per-segment duration before ASR (e.g. 15â€“30 seconds).

4. **Metadata propagation**
   - The MediaSource and extraction process **MUST** preserve:
     - Original media duration
     - Timestamps (if relevant for aligning transcript to video)
     - Basic technical metadata (codec, original sample rate)

This metadata is required for accurate timeline mapping in the UI and for potential future alignment features (e.g. word-level highlighting on video).

#### 6.2.3.4 Pre-Processing and Segmentation (ffmpeg, Resampling, VAD)

Before ASR, audio **MUST** be pre-processed and segmented:

1. **Pre-processing**
   - Steps:
     - Decode audio to raw PCM using `ffmpeg`.
     - Resample to 16 kHz if needed.
     - Convert to a consistent sample format (e.g. 16-bit signed integer).
   - Goals:
     - Provide a stable, well-known input format to the ASR service.
     - Avoid duplicating audio decoding logic inside the ASR runtime.

2. **Segmentation**
   - The Orchestrator **MUST** support:
     - Voice Activity Detection (VAD) or silence-based segmentation to split long audio into smaller segments.
     - A fallback fixed-window segmenter (with overlap) for cases where VAD fails.
   - Configuration parameters (per profile):
     - Target segment length (e.g. 5â€“15 seconds)
     - Minimum/maximum segment length
     - Overlap duration between segments
   - Segmentation decisions **MUST** be recorded (start/end times per segment) to allow deterministic re-assembly.

3. **Quality vs speed profiles**
   - Different profiles (e.g. â€œqualityâ€ vs â€œfastâ€) **MAY**:
     - Use different VAD sensitivity
     - Use different maximum segment lengths
   - The chosen profile **MUST** be stored as part of the job configuration.

#### 6.2.3.5 ASR Service Interface (APIs, Job Queue, Progress Reporting)

The interface between desktop client, Orchestrator, and ASR service **MUST** be explicit and versioned.

1. **Client â†” Orchestrator API**

   Minimum required endpoints:

   - `POST /v1/asr/jobs`
     - Input:
       - MediaSource reference (resource ID)
       - Language hints, profile (quality/fast)
       - Optional job metadata (user notes, tags)
     - Output:
       - Job ID
       - Initial status (`queued`)

   - `GET /v1/asr/jobs/{job_id}`
     - Output:
       - Status (`queued` | `running` | `completed` | `failed` | `cancelled`)
       - Progress estimate (0â€“100%, and/or processed duration vs total duration)
       - Basic timing and model metadata once available

   - `DELETE /v1/asr/jobs/{job_id}`
     - Cancels a running job if possible.

2. **Orchestrator â†” ASR Service API**

   Minimum required RPC:

   - `TranscribeSegments` (HTTP/gRPC)
     - Request:
       - Array of segments (raw PCM buffers or paths to temp files)
       - Model/runtime configuration (language, profile, GPU/CPU preference)
     - Response:
       - Array of transcripts (text)
       - Token/word-level timestamps (where available)
       - Confidence scores per segment or per token
       - Optional normalized text (post-punctuation)

3. **Job queue and concurrency**

   - The Orchestrator **MUST**:
     - Maintain a per-user ASR job queue.
     - Limit concurrent jobs according to resource constraints (e.g. maximum concurrent ASR jobs using GPU).
     - Surface queue position and estimated start time when possible.

   - The ASR service **SHOULD**:
     - Implement internal batching when it improves throughput (multiple segments per forward pass).
     - Respect a maximum concurrency configured by the Orchestrator.

4. **Progress reporting**

   - Progress to the client **MUST** be based on:
     - Total media duration vs processed duration, and/or
     - Segment count vs completed segments.
   - The client **SHOULD**:
     - Display coarse-grained progress (e.g. â€œ34 min processed of 90 minâ€).
     - Show final latency and effective real-time factor for diagnostic purposes.

#### 6.2.3.6 Transcript Storage as DerivedContent and Shadow Workspace Integration

Once ASR has completed, the transcript becomes part of the Handshake data model.

1. **DerivedContent representation**

   - Each transcript **MUST** be stored as a **DerivedContent** object linked to:
     - The original MediaSource resource ID.
     - The ASR job ID and model configuration used.
   - Minimum fields:
     - Plain text transcript (UTF-8)
     - Optional structured representation (e.g. JSON with segments, speakers, timestamps)
     - Model metadata (name, version, runtime profile)
     - Job metadata (start/end time, duration, status, errors)

2. **Versioning**

   - Transcripts **MUST** be versioned:
     - Re-transcription with different models or settings creates a new DerivedContent version.
     - Prior versions **MUST** remain accessible for debugging and comparison until explicitly deleted.
   - The UI **SHOULD**:
     - Allow users to see which transcript is â€œactiveâ€ and switch if needed.

3. **Shadow Workspace ingestion**

   - Shadow Workspace **MUST**:
     - Ingest the transcript as soon as the job completes.
     - Create or update embeddings, search indexes, and graph nodes for the transcript.
   - The transcript **MUST** be:
     - Discoverable via global search and filters (e.g. â€œtype:transcriptâ€).
     - Addressable by stable IDs for LLM tools (e.g. â€œsummarize transcript Xâ€).

4. **LLM tools and downstream use**

   - ASR transcripts **MUST** be first-class inputs to:
     - Summarization tools (lecture/meeting summaries)
     - Extraction tools (action items, decisions, entities)
     - Q&A over content (RAG)
   - These tools **MUST NOT** modify the original transcript; they produce additional DerivedContent artifacts (summaries, notes, etc.) with their own IDs and metadata.

5. **Data retention and privacy**

   - Transcripts **MUST** be treated as user-owned workspace data:
     - Stored locally by default
     - Syncing and cloud usage (for LLMs or ASR fallback) only if the user has opted in.
   - Any external calls (e.g. cloud LLM summarization) **MUST** be clearly documented and, where possible, logged at a metadata level (not full content) for user inspection.

This completes the definition of the ASR architecture as integrated into the Handshake Orchestrator, Model Runtime Layer, and Shadow Workspace.
### 6.2.4 X.4 Runtime Modes: Batch vs Streaming

This section defines the execution modes for ASR in Handshake: what is supported in the MVP and what is explicitly deferred. The core assumption is that Handshake is a **local-first desktop app** optimized for ingesting long-form content (lectures, meetings, videos), not a real-time conferencing tool.

#### 6.2.4.1 Batch (Offline) Transcription â€“ MVP Scope

The Handshake ASR MVP **ONLY** supports batch (offline) transcription:

1. **Definition**

   - Batch transcription = transcribing a **finite, already-recorded** audio/video resource.
   - Examples:
     - University lecture recordings (1â€“3 hours)
     - Meeting recordings (30â€“120 minutes)
     - Screen recordings with system audio

2. **User flow**

   - User selects a media resource in the workspace (or records a new one).
   - User triggers a â€œTranscribeâ€ or â€œGenerate transcriptâ€ action.
   - The client:
     - Sends a transcription job request to the Orchestrator (see X.3.5)
     - Shows job state and progress (queued â†’ running â†’ completed/failed)
   - Once completed:
     - A transcript DerivedContent object is attached to the media resource
     - Shadow Workspace indexes the transcript and exposes it to tools (X.3.6)

3. **Batch mode guarantees**

   - The system **MUST**:
     - Handle recordings up to at least 3 hours on the reference workstation.
     - Provide stable progress reporting (see X.3.5).
     - Avoid UI freezes and respect global resource limits (GPU and CPU).

   - The system **SHOULD**:
     - Resume partially completed jobs after crashes where possible
     - Support cancellation without corrupting existing DerivedContent

4. **Scheduling and resource usage**

   - Batch jobs are **background** jobs:
     - They may run at lower priority than interactive UI and other critical tasks.
     - They may be paused/throttled under heavy system load.
   - The Orchestrator **MUST**:
     - Enforce configurable limits on concurrent ASR jobs
     - Coordinate GPU usage with other model runtimes (LLMs, image models, etc.)

#### 6.2.4.2 Lecture-Length Workloads (1â€“3h Recordings)

Handshake explicitly targets **lecture-length workloads** as a primary use case.

1. **Scale assumptions**

   - Typical lecture: 60â€“90 minutes
   - Upper bound for MVP: 180 minutes (3 hours)

2. **Segmentation strategy**

   - For long recordings, the Orchestrator **MUST**:
     - Use VAD/silence-based segmentation or fixed windows to produce segments that:
       - Fit comfortably in the ASR modelâ€™s context window
       - Do not exceed a configured maximum length (e.g. 15â€“30 seconds)
     - Retain accurate segment timestamps for later assembly and navigation.

3. **Performance targets (on reference workstation)**

   - Targets (guidance, not hard guarantees):
     - **Real-time factor (RTF)**: aim for RTF â‰¤ 1.0 on the primary GPU model for typical lectures.
       - Example: 90-minute lecture should finish in â‰ˆ 90 minutes or faster in â€œqualityâ€ mode.
     - **Memory usage**:
       - ASR **MUST NOT** starve LLM runtimes; GPU VRAM thresholds and prioritization rules in X.2.4 apply.

4. **UX considerations**

   - For long jobs, the UI **SHOULD**:
     - Show progress based on processed duration vs total duration (X.3.5).
     - Provide a summary of resource usage (e.g. â€œProcessed 120 min in 80 min; effective RTF 0.67â€).
     - Optionally, allow the user to open partial transcripts (e.g. first hour) once certain milestones are reached (future enhancement; not mandatory for MVP).

#### 6.2.4.3 Future: Streaming / Live Captions (Out of Scope for MVP)

Real-time streaming / live captions are **explicitly out of scope** for the initial ASR MVP, but the architecture **MUST NOT** make them impossible.

1. **Definition**

   - Streaming = transcribing audio as it is being captured, with:
     - End-to-end latency small enough for live captions or near-real-time monitoring.
     - Continuous, unbounded audio streams (no fixed recording duration known in advance).

2. **Non-goals for MVP**

   - MVP **MUST NOT** attempt to:
     - Provide live subtitles for video calls or live streaming platforms.
     - Guarantee sub-second latency for partial hypotheses.
     - Support gRPC/WebSocket streaming APIs from the desktop client.

3. **Architectural hooks for future streaming**

   Even though streaming is not implemented in the MVP, the design **SHOULD** leave room for:

   - A **session-based** ASR API in the Orchestrator, distinct from batch jobs:
     - e.g. `StartStream`, `SendAudioChunk`, `ReceivePartial`, `EndStream`
   - An ASR runtime that can:
     - Consume short audio frames (e.g. 10â€“30 ms)
     - Produce partial hypotheses and revisions over time

   These APIs are **not required** for MVP but must be architecturally compatible with X.3â€™s process boundaries and Model Runtime Layer.

4. **Model and performance implications (for later phases)**

   - Streaming will impose stricter constraints:
     - Lower per-utterance latency
     - Tighter control over GPU sharing with other models
   - It may require:
     - Streaming-optimized model variants (e.g. chunk-wise or online models)
     - More aggressive chunking and incremental decoding strategies

   These requirements are acknowledged but **not implemented** in the first version.

#### 6.2.4.4 Design Constraints for Future Streaming Support

To avoid painting the architecture into a corner, the following constraints apply for all MVP implementations:

1. **Segment-based internal representation**

   - Even in batch mode, the Orchestrator and ASR runtime **MUST** treat audio as an ordered sequence of segments:
     - Each segment has explicit start/end timestamps and an ID.
     - The ASR API operates over arrays of segments (X.3.5).
   - This segment abstraction is the natural bridge to future streaming, where segments become small time slices.

2. **Stateless vs stateful ASR services**

   - The batch MVP may treat ASR requests as stateless (â€œfire and forgetâ€ per segment or batch).
   - However, the service interface **MUST** be designed so that:
     - It can later support session or stream IDs.
     - Model internal state (e.g. online decoding state) can be maintained across calls when needed.

3. **Resource arbitration**

   - Even without streaming, the GPU resource manager in the Orchestrator **MUST**:
     - Be explicit and centralized (no â€œhiddenâ€ GPU users).
     - Support future constraints like â€œlow-latency stream takes precedence over batch jobs.â€

4. **UI separation**

   - Batch transcription UI **MUST** be clearly separated from any future â€œlive captionâ€ UI:
     - Different affordances and expectations (batch = eventual completion; streaming = continuous partials).
   - This avoids coupling design decisions that would be hard to untangle later.

In summary, the MVP supports **batch ASR for long-form recordings only**, with the architecture structured so that **streaming capabilities can be added later** without breaking the Orchestratorâ€“ASR service contract or the data model.
### 6.2.5 X.5 Customization and Fine-Tuning

This section defines how Handshake customizes ASR behavior for different domains, vocabularies, and languages. It builds on the model tiering in X.2.4 and the architecture in X.3.

#### 6.2.5.1 Types of Customization

Handshake distinguishes the following levels of customization, ordered from cheapest to most expensive:

1. **Runtime configuration (no training)**
   - Examples:
     - Language hints and â€œforce languageâ€ options
     - Beam search configuration (beam size, temperature, length penalties)
     - Timestamp policies (segment-level vs word-level)
   - Characteristics:
     - Zero training cost
     - Immediate, reversible
   - Usage:
     - Exposed as profiles (e.g. â€œgeneral lectureâ€, â€œmeetingâ€, â€œno timestampsâ€).

2. **Text-only post-processing (no ASR model changes)**
   - Examples:
     - Punctuation and casing normalization
     - Normalizing numbers, dates, and common abbreviations
     - Basic profanity filtering (optional, user-configurable)
   - Implementation:
     - Deterministic rules (regex, small finite-state machines)
     - Lightweight text models where appropriate
   - Characteristics:
     - No changes to the acoustic/decoder model
     - Cheap to iterate on and easy to test

3. **Lexicon / biasing / LM-rescoring (where supported)**
   - Examples:
     - Domain-specific word/phrase lists for:
       - Product names, company names
       - Course titles, project codenames
     - Text-only language models (n-gram LMs) to rescore candidate transcripts
   - Characteristics:
     - Requires toolkit support (not all runtimes expose this cleanly)
     - Does not require paired audioâ€“text training

4. **Adapter-based or LoRA-based fine-tuning**
   - Small trainable modules attached to a frozen base model:
     - Per-domain adapters (e.g. â€œsoftware engineering lecturesâ€, â€œmedical research talksâ€)
     - LoRA layers on top of core encoder/decoder blocks
   - Characteristics:
     - Lower risk than full fine-tune
     - Easier to enable/disable per domain

5. **Full checkpoint fine-tuning**
   - Training the base modelâ€™s parameters (or large subsets) on in-domain audio+text.
   - Characteristics:
     - Highest potential gain
     - Highest cost and risk (overfitting, regressions)
   - Controlled by the fine-tuning gate in X.5.2.

Handshake **MUST** prioritize options 1â€“3 for the MVP, and treat 4â€“5 as later-phase optimizations governed by X.5.2.

#### 6.2.5.2 Fine-Tuning Gate and Customization Policy

This section defines **when** Handshake is allowed to fine-tune ASR models, and which lighter-weight customization options **MUST** be tried first.

By default, Handshake ships with **unmodified open-source checkpoints** (see X.2.4). Fine-tuning is an **optional, later-phase optimization**, not part of the MVP.

##### 6.2.5.2.1 X.5.2.1 Customization hierarchy

Handshake SHALL follow this hierarchy, from cheapest to most expensive:

1. **Configuration and prompting**  
   Enable or adjust built-in features of the ASR runtime:
   - Language hints
   - Temperature / beam settings
   - Timestamps, word-level timing options  
   Use LLM-based post-processing for:
   - Sectionization and headings
   - Summaries, action items, Q&A  
   No model training involved.

2. **Lexicons, biasing, and rescoring (where supported)**  
   If the chosen runtime supports it, Handshake MAY:
   - Add domain-specific word/phrase lists (brand names, jargon, entities)
   - Use a text-only language model or n-gram LM to rescore ASR hypotheses  
   This option **MUST** be evaluated before full fine-tuning.

3. **Lightweight domain adapters (if available)**  
   If the ecosystem provides adapter-style fine-tuning (LoRA, adapters):
   - Prefer that over full checkpoint fine-tuning.
   - Keep adapter weights small and modular per domain.

4. **Full fine-tuning of base models**  
   This is the **last resort** and **MUST** pass the gate criteria in X.5.2.2.

##### 6.2.5.2.2 X.5.2.2 Fine-tuning entry criteria (â€œthe gateâ€)

Handshake **MUST NOT** initiate fine-tuning of any ASR model unless **all** of the following conditions are true:

1. **Data volume and quality**
   - There is **at least 100 hours** of in-domain audio per target language/domain with:
     - Reasonably clean recordings (no catastrophic noise)
     - Human-checked transcripts with time alignment accurate enough for training
   - For more ambitious gains, the preferred target is **200â€“300 hours** per language/domain.
   - Data **MUST** be collected with explicit user consent and stored in a way that respects privacy requirements.

2. **Demonstrated accuracy gap**
   - A stable evaluation suite exists (see X.8) with:
     - A fixed test set of in-domain audio
     - Baseline metrics for the current untuned model
   - On that test set, the baseline model shows:
     - Word Error Rate (WER) clearly above the acceptable target for the use case, **and**
     - Qualitative errors that materially affect downstream tasks (misrecognised key terms, domain jargon, entity names).
   - Lighter customizations (X.5.2.1 â€“ levels 1â€“3) have already been applied and tested and are **insufficient** to close the gap.

3. **Expected benefit**
   - There is a reasonable expectation, based on prior art or small-scale experiments, that fine-tuning can:
     - Improve WER by at least **25% relative** (e.g. from 20% â†’ 15%) **or**
     - Reduce critical error classes (e.g. entity names) enough to materially improve UX.
   - This expectation **MUST** be documented in a short design note before training begins.

4. **Compute and operational capacity**
   - Dedicated training hardware is available (e.g. a separate GPU machine or cloud training environment).  
     Fine-tuning **MUST NOT** run on end-user devices.
   - There is capacity to:
     - Run multiple training runs for hyperparameter tuning
     - Store, version, and roll back fine-tuned checkpoints
     - Maintain separate â€œstableâ€ and â€œexperimentalâ€ ASR configurations

5. **Governance and rollback**
   - Each fine-tuned model version **MUST**:
     - Have a unique version ID and changelog
     - Be evaluated against the same test suite as the baseline
   - A rollback plan **MUST** exist:
     - If a new model regresses on any tracked metric beyond tolerance, the system **MUST** be able to immediately revert to the previous stable model.

If any of these conditions are not met, fine-tuning is **not allowed**. The team **MUST** continue using untuned models plus lighter customizations.

#### 6.2.5.3 Data Collection and Labeling Strategy

Customization and fine-tuning depend on data. Handshakeâ€™s strategy is:

1. **Default stance: no automatic collection**
   - By default:
     - User audio and transcripts stay local.
     - No audio is uploaded or collected for centralized training.
   - Any deviation from this default (e.g. opt-in data donation) **MUST** be explicit and clearly documented.

2. **Local usage data for personalization (optional, future)**
   - The client **MAY** track:
     - Local corrections the user makes to transcripts
     - User-defined vocabularies (custom terms, names)
   - This information can feed:
     - Local lexicons
     - Local bias lists or on-device post-processing rules
   - This does **not** require server-side model training.

3. **Opt-in data donation for model training (non-MVP)**
   - Only relevant if you decide to build a central training pipeline.
   - Requirements:
     - Explicit user opt-in per workspace or per recording
     - Clear description of:
       - What is collected (audio, transcript, metadata)
       - How it is anonymised or pseudonymised
       - How users can revoke consent and request deletion
   - Donated data **MUST** be:
     - Aggregated
     - Auditable
     - Separable by domain and language

4. **Labeling and quality control**
   - For any training/fine-tuning:
     - A subset of data **MUST** be manually checked or corrected.
     - Weak labels (ASR-generated transcripts) **MAY** be used but:
       - A manually verified validation/test set is mandatory.
   - Labeling **MUST** be guided by eval needs:
     - If the goal is to fix jargon, labels must accurately mark those terms.
     - If the goal is speaker diarization, speaker turns must be reliable.

5. **Data schema**
   - Training data **MUST** use a consistent schema:
     - Audio file path / ID
     - Text transcript
     - Language tag
     - Domain tag (e.g. â€œCS lectureâ€, â€œproduct meetingâ€)
     - Optional metadata (speaker count, noise level, recording device)
   - This schema is shared between:
     - ASR eval (X.8)
     - Any future fine-tuning pipeline

#### 6.2.5.4 Training and Deployment Strategy

If/when Handshake fine-tunes ASR models (under X.5.2), training and deployment follow these rules:

1. **Separation of concerns**
   - Training **MUST NOT** happen in the desktop app.
   - Training and evaluation happen on:
     - Dedicated internal machines, or
     - Explicit training environments (cloud or on-prem).

2. **Model lifecycle**
   - For each model type (e.g. â€œWhisper-large-v3â€), define:
     - Base model (untuned)
     - 0+ domain-specific derivatives (e.g. â€œCS-lecture-tuned-v1â€)
   - Derivatives **MUST**:
     - Declare their base model
     - Be compatible with the same ASR service API

3. **Versioning and promotion**
   - New model versions follow a promotion path:
     1. Experimental:
        - Used only in internal tests and behind developer flags.
     2. Candidate:
        - Passes basic eval but not yet default.
     3. Default:
        - Promoted after:
          - Passing all eval gates in X.8
          - No regressions on critical metrics
   - Demotion:
     - If a default model regresses in real-world use, it **MUST** be demoted and replaced by the previous stable model.

4. **Deployment to clients**
   - Desktop clients receive model updates via:
     - Bundled binaries or
     - Separate model packages (downloaded on demand)
   - The client **MUST**:
     - Verify model package integrity (checksum/signature)
     - Store model version and configuration with each transcript (X.3.6)

5. **Resource envelopes**
   - Fine-tuned models **MUST** respect predefined resource envelopes:
     - Max VRAM usage on reference GPU
     - Max latency for standard workloads
   - If a fine-tuned model exceeds these envelopes, it is **not eligible** to become the default.

6. **Documentation**
   - Each trained model **MUST** have:
     - A model card (data, domains, languages, limitations)
     - An evaluation report (baseline vs tuned)
     - Operational notes (resource usage, compatibility constraints)

This ensures that any customization beyond configuration is controlled, measurable, and reversible.

### 6.2.6 X.6 Post-Processing, Diarization, and LLM Tools

This section defines how raw ASR outputs are cleaned up, enriched with speaker information, and made available to LLM tools and the Shadow Workspace.

#### 6.2.6.1 Deterministic Cleanup (Punctuation, Casing, Numbers)

1. **Goals**
   - Improve readability without changing the semantic content.
   - Avoid making opaque, model-dependent edits that are hard to reason about.

2. **Scope of deterministic cleanup**
   - Basic punctuation:
     - Sentence boundaries (periods, question marks)
     - Commas for obvious pauses where safe
   - Casing:
     - Sentence-initial capitalization
     - Proper nouns where unambiguous (optional)
   - Normalization:
     - Numbers (e.g. â€œtwenty oneâ€ â†’ â€œ21â€) when safe
     - Standard abbreviations (e.g. â€œU S Aâ€ â†’ â€œUSAâ€), if reliably detectable

3. **Implementation**
   - Prefer:
     - Lightweight models or rules provided by the ASR toolkit
     - Simple, testable rule-based passes over transcripts
   - Requirements:
     - Cleanup steps **MUST** be deterministic given the same input
     - Cleanup configuration **MUST** be stored with the transcript metadata

4. **Configuration and visibility**
   - Users **SHOULD** be able to:
     - Toggle some aspects of cleanup (e.g. aggressive vs minimal punctuation)
   - For diagnostics:
     - The â€œrawâ€ ASR output (pre-cleanup) **SHOULD** be accessible to developers and power users.

#### 6.2.6.2 Diarization (Speaker Turns, Optional)

Speaker diarization is optional for the MVP but is highly desirable for meetings and multi-speaker content.

1. **Responsibilities**
   - Identify â€œwho spoke whenâ€:
     - Segment audio into speaker-homogeneous regions
     - Assign speaker IDs (e.g. SPK1, SPK2, â€¦)
   - Align these regions with transcript segments.

2. **Architecture**
   - Diarization is a separate step from ASR:
     - Can run:
       - Before ASR (to segment by speaker) or
       - After ASR (to label segments)
   - It **MAY** use:
     - External toolkits (e.g. embedding-based diarization)

3. **MVP stance**
   - Diarization is **MAY** for the initial release:
     - The architecture must allow adding it later.
     - The transcript schema (X.3.6) **SHOULD** leave room for:
       - Per-segment speaker labels
       - Optional speaker name mappings (user-labeled)

4. **Data model**
   - Transcript representation **SHOULD** support:
     - `speaker_id` per segment or per sentence
     - Separate mapping from `speaker_id` â†’ human-friendly label (e.g. â€œAliceâ€)

5. **UI**
   - When diarization is present:
     - UI **SHOULD** visually distinguish speakers (color, label)
     - Users **SHOULD** be able to rename speakers (SPK1 â†’ â€œAliceâ€)

#### 6.2.6.3 LLM-Based Transforms (Summaries, Action Items, Q&A)

ASR transcripts are a primary input to Handshakeâ€™s LLM tools. These tools **MUST NOT** overwrite the transcript itself; they produce additional DerivedContent.

1. **Transform types**
   - Summarization:
     - High-level summaries (short, long)
     - Section-wise summaries (per lecture topic)
   - Extraction:
     - Action items
     - Decisions
     - Key entities (people, projects, terms)
   - Q&A:
     - User questions about a specific transcript or across multiple transcripts

2. **Execution model**
   - All LLM transforms:
     - Take one or more transcript IDs as input
     - Run via the Orchestrator using existing LLM model runtimes
   - Outputs:
     - New DerivedContent objects (summaries, lists, notes)
     - Linked back to the original transcript(s)

3. **Isolation from ASR**
   - ASR **MUST** remain a separate concern:
     - Changes in LLM behavior (e.g. different summary style) do not affect the underlying transcript.
     - Transcript correctness is evaluated independently of LLM outputs.

4. **Configuration**
   - Users **SHOULD** be able to:
     - Choose which transforms to run (e.g. â€œonly summaryâ€ vs â€œsummary + action itemsâ€)
     - Re-run transforms with updated models without re-running ASR

#### 6.2.6.4 How Transcripts Flow into Shadow Workspace, Search, and RAG

1. **Indexing in Shadow Workspace**
   - On completion, each transcript DerivedContent object is:
     - Parsed into logical units (paragraphs, segments, or time-coded blocks)
     - Embedded (vector representations) for semantic search
     - Inserted into the global index

2. **Search behavior**
   - Transcripts **MUST** be:
     - Searchable by full-text (keywords)
     - Searchable by semantic similarity (embeddings)
   - Filters:
     - Type: transcript
     - Source: video/meeting/lecture
     - Time range, language, tags

3. **RAG (Retrieval-Augmented Generation) integration**
   - When users ask questions, the retrieval layer **MAY**:
     - Pull relevant transcript chunks
     - Feed them to LLMs as context
   - Requirements:
     - Chunks **MUST** preserve pointers back to:
       - Original transcript
       - Timestamps in the media (for â€œjump to videoâ€ UX)

4. **Cross-linking**
   - Derived artifacts (summaries, notes, extracted items) **SHOULD**:
     - Maintain links back to transcript segments
     - Be traversable in the UI (e.g. click an action item â†’ jump to the moment in the transcript/video)

5. **Privacy and scope**
   - By default, all indexing and RAG usage:
     - Happens locally
     - Uses local LLMs where configured
   - Cloud usage (for embedding or LLM) **MUST** be:
     - Explicitly configurable
     - Clearly indicated in UI and documentation

This completes the definition of how raw ASR output becomes clean, structured, and LLM-ready content in the Handshake workspace and Shadow Workspace.
### 6.2.7 X.7 Risk, Compliance, and Limitations

This section enumerates the primary risks of ASR in Handshake and defines how they are mitigated. It also clarifies compliance stance and explicit non-goals.

#### 6.2.7.1 Technical Risks

1. **Latency and throughput**
   - Risk:
     - Long recordings take too long to process, blocking user workflows.
   - Mitigation:
     - Use GPU-accelerated models where available (X.2.4).
     - Enforce segmentation and batching (X.3.4, X.3.5).
     - Treat ASR as background work with clear progress reporting (X.4.1, X.4.2).

2. **Memory and resource contention**
   - Risk:
     - ASR models consume GPU VRAM / CPU and starve LLMs or the UI.
   - Mitigation:
     - Centralized resource arbitration in the Orchestrator.
     - Strict VRAM thresholds and model tiering (X.2.4.4).
     - Limits on concurrent jobs; ability to pause or throttle ASR.

3. **Model drift and regression**
   - Risk:
     - Updating ASR models silently degrades accuracy on certain domains or languages.
   - Mitigation:
     - Model versioning and model cards (X.5.4).
     - Mandatory eval suite runs before promotion (X.8).
     - Regression thresholds and rollback capability.

4. **Multilingual and accent robustness**
   - Risk:
     - Good performance on English but poor on other target languages or accents.
   - Mitigation:
     - Multilingual primary model (Whisper).
     - Language-specialised experimental models (X.2.4.3).
     - Per-language eval subsets (X.8.2).
     - Clear documentation of known limitations in model cards.

5. **Dependency / ecosystem health**
   - Risk:
     - Core dependencies (Faster-Whisper, whisper.cpp, VAD libraries) change or break.
   - Mitigation:
     - Pin versions in deployment.
     - Maintain a minimal abstraction layer around ASR runtimes.
     - Keep a simple CPU-only fallback path that depends on fewer components.

#### 6.2.7.2 Multilingual Coverage and Accent Robustness

1. **Target languages**
   - Handshakeâ€™s primary ASR targets:
     - English, Chinese (Mandarin)
   - Secondary â€œnice-to-haveâ€ targets:
     - Dutch, Korean, Japanese, Arabic, Russian

2. **Baseline expectations**
   - The primary model (Whisper large-v3) is expected to be:
     - Strong for English, adequate for major languages.
     - Imperfect for some accents and low-resource languages.

3. **Mitigation and transparency**
   - For each supported language:
     - Maintain per-language WER in the eval suite.
     - Document whether expected quality is:
       - â€œGood enough for notes and summariesâ€
       - Or â€œnot recommended beyond rough referenceâ€.

4. **Accent and domain gaps**
   - Where severe gaps are identified:
     - Prefer domain-specific guidelines and UX warnings first.
     - Consider:
       - Better configuration/segmentation
       - Lexicons / rescoring
       - Experimental models or, later, fine-tuning (X.5.2)

#### 6.2.7.3 Licensing and Open-Source Obligations

1. **Model licensing**
   - All default ASR models **MUST**:
     - Be under licenses that permit local inference and redistribution in the intended distribution model of Handshake.
   - For each model:
     - License type **MUST** be documented in the model card.
     - Any usage restrictions **MUST** be clearly surfaced in internal docs.

2. **Library licensing**
   - Core ASR libraries (Faster-Whisper, whisper.cpp, etc.) **MUST**:
     - Have compatible licenses with the Handshake codebase.
   - Third-party code **MUST**:
     - Be tracked, pinned, and attributed according to its license.

3. **Attribution**
   - Handshake **SHOULD**:
     - Provide a â€œThird-Party Componentsâ€ section listing:
       - ASR models used
       - Toolkits and key libraries
       - Their respective licenses

#### 6.2.7.4 Privacy and Compliance Stance

1. **Default data flow**
   - By default:
     - All ASR happens locally.
     - Audio and transcripts are stored locally as workspace data.
     - No audio or transcript content is sent to remote servers for ASR.

2. **Compliance scope**
   - Handshake ASR is **NOT** designed or marketed as compliant for:
     - Regulated healthcare transcription (e.g. HIPAA-covered clinical dictation).
     - Legal record creation where certified transcripts are required.
   - If users choose to apply Handshake in those contexts, they do so at their own risk.

3. **Cloud usage (if enabled)**
   - If cloud ASR or cloud LLMs are enabled:
     - This **MUST** require explicit opt-in.
     - The UI **MUST** clearly indicate:
       - Which jobs use cloud services
       - Which providers are involved
   - Telemetry or logs:
     - **MUST NOT** include raw audio or full transcripts without explicit user consent.

4. **User control**
   - Users **MUST** be able to:
     - Delete transcripts and media from their workspace.
     - Disable any cloud-based ASR or LLM integration.
   - Any centralized training or data donation programs (if ever added):
     - **MUST** be opt-in and clearly documented (X.5.3).

#### 6.2.7.5 Cloud Fallback Policy

1. **Default**
   - Cloud fallback for ASR is **disabled by default**.

2. **Optional behavior**
   - When explicitly enabled by the user:
     - The Orchestrator **MAY**:
       - Retry failed segments or low-confidence segments on a configured cloud ASR provider.
     - The transcript:
       - **MUST** annotate which segments came from cloud vs local ASR.

3. **Failure and error handling**
   - If cloud ASR fails or is unavailable:
     - The system **MUST NOT** silently drop segments.
     - The final transcript **MUST** clearly indicate missing or failed sections.

4. **Config surface**
   - Cloud fallback settings **MUST** be:
     - Centralized in a single configuration UI
     - Clearly labeled as sending data off-device

### 6.2.8 X.8 Evaluation and Benchmarks

This section defines how ASR quality and performance are measured and guarded over time.

#### 6.2.8.1 Metrics

Handshakeâ€™s ASR eval suite **MUST** track at least:

1. **Accuracy metrics**
   - Word Error Rate (WER)
   - Character Error Rate (CER) (especially for languages where word boundaries are ambiguous)
   - Optional: entity-level error metrics for key terms (names, technical terms)

2. **Performance metrics**
   - Real-time factor (RTF):
     - `RTF = transcription_time / audio_duration`
   - Latency distribution for segments (p50, p90)
   - GPU VRAM usage and peak CPU usage

3. **Robustness metrics**
   - Per-language WER/CER
   - Per-domain WER/CER (lectures vs meetings vs misc.)
   - Optional: error breakdowns by:
     - Background noise level
     - Accent categories (if labeled)

#### 6.2.8.2 Benchmark Datasets and Synthetic Workloads

1. **Dataset types**
   - Internal, in-domain datasets:
     - Real recordings of lectures, meetings, and user-like content.
   - External public benchmarks (where licensing allows):
     - For cross-checking against known baselines.

2. **Label quality**
   - Test sets **MUST**:
     - Have human-verified transcripts.
     - Be stable across model versions (no silent changes without versioning).

3. **Coverage**
   - Datasets **SHOULD** include:
     - English lectures and meetings (core)
     - Chinese content (at least some lectures/conversations)
     - Smaller but non-zero samples for other target languages

4. **Synthetic workloads**
   - For performance testing:
     - Synthetic â€œlong lectureâ€ jobs MAY be generated by concatenating shorter clips.
   - These workloads:
     - **MUST** stress segmentation, queueing, and resource usage.

#### 6.2.8.3 Target Thresholds for â€œGood Enoughâ€

Target thresholds depend on domain; as a starting point:

1. **Lectures and meetings (EN)**
   - WER:
     - Target: â‰¤ 10â€“12% on internal eval set
     - Warning band: 12â€“15%
     - Fail: > 15% (requires investigation before promotion)
   - RTF (reference workstation, primary model):
     - Target: â‰¤ 1.0
     - Warning band: 1.0â€“1.5
     - Fail: > 1.5 for typical workloads

2. **Non-English target languages**
   - WER/CER targets:
     - Initially looser, e.g.:
       - Target: â€œcomparable to or slightly worse than EN baselineâ€
       - Explicitly documented when significantly worse
   - Expectation:
     - Sufficient for summarization and note-taking, not verbatim transcripts.

3. **Resource usage**
   - VRAM:
     - Primary ASR model **MUST** fit within a defined budget on the reference GPU with headroom for at least one LLM.
   - CPU:
     - ASR **MUST NOT** monopolize all cores; Orchestrator **MUST** cap parallelism.

These thresholds are subject to revision but **MUST** be documented at each revision.

#### 6.2.8.4 Regression Tests and Continuous Evaluation

1. **Pre-release checks**
   - Any change to:
     - ASR models
     - ASR runtime
     - Segmentation or preprocessing
   - **MUST** trigger:
     - Full eval suite run on core datasets
     - Comparison against previous baseline

2. **Regression criteria**
   - A new version **MUST NOT** be promoted to default if:
     - WER increases beyond predefined tolerances
     - RTF or resource usage significantly degrades without compensating benefits

3. **Continuous monitoring (optional)**
   - For internal/dev builds:
     - The system **MAY** collect anonymised metrics on:
       - Job durations
       - Failure rates
       - Effective RTF in the field
   - These metrics:
     - **MUST** not include raw content unless explicitly opted in.

4. **Reporting**
   - Each major ASR update **SHOULD**:
     - Produce a short eval report (baseline vs new)
     - Be attached to the model card and internal release notes

### 6.2.9 X.9 Roadmap and Implementation Plan

This section defines a phased plan for delivering ASR in Handshake.

#### 6.2.9.1 MVP Scope (First Shippable)

The ASR MVP **MUST** deliver:

1. **Core capabilities**
   - Batch transcription for:
     - Locally stored audio/video files
     - Newly recorded content from within Handshake
   - Integration with:
     - Orchestrator and Model Runtime Layer (X.3.2)
     - Shadow Workspace (X.3.6, X.6.4)

2. **Model stack**
   - Primary Whisper large-v3 model on GPU (X.2.4).
   - Fast/low-resource Whisper small model and CPU fallback.

3. **UX**
   - Clear â€œTranscribeâ€ workflow.
   - Job progress indicators and completion status.
   - Transcript view with basic navigation and editing.

4. **Quality and performance**
   - Meet initial RTF and WER targets for English lectures/meetings (X.8.3).
   - Stable behavior on recordings up to 3 hours.

5. **Non-goals for MVP**
   - No streaming/live captions.
   - No fine-tuning pipeline.
   - Diarization optional; if present, basic only.
   - No cloud ASR by default.

#### 6.2.9.2 Phase 2 â€“ Multilingual, Diarization, Better UX

Phase 2 **SHOULD** focus on:

1. **Multilingual improvements**
   - Establish per-language eval subsets.
   - Add and test language-specialised engines (e.g. Mandarin ASR).
   - Improve language detection/hints in the pipeline.

2. **Diarization**
   - Integrate diarization toolchain.
   - Extend transcript schema with speaker IDs.
   - Add UI support for speaker labels and filtering.

3. **LLM tooling over transcripts**
   - Productionize:
     - Summaries
     - Action items
     - Q&A over transcripts
   - Tighten linking between transcripts, media, and derivative notes.

4. **Robustness and ergonomics**
   - Better error handling and recovery (resume partial jobs).
   - More flexible segmentation profiles (quality vs speed).

#### 6.2.9.3 Phase 3 â€“ Fine-Tuning, Streaming, Multi-Engine Consensus

Once MVP and Phase 2 are stable, Phase 3 **MAY** introduce:

1. **Fine-tuning (under X.5.2 gate)**
   - Build a small central training pipeline.
   - Fine-tune ASR for:
     - Specific domains (e.g. CS lectures)
     - Specific languages with enough data
   - Integrate results into the model lifecycle (X.5.4).

2. **Streaming / live captions (if desired)**
   - Implement session-based ASR APIs.
   - Integrate streaming-capable models.
   - Provide a distinct â€œlive captionâ€ UX.

3. **Multi-engine consensus and cloud fallback**
   - For critical content:
     - Explore multi-engine fusion (voting/ROVER-style).
     - Optionally combine local + cloud outputs (with user opt-in).
   - Measure cost/benefit vs single-engine setup.

#### 6.2.9.4 De-Risking Plan and Open Questions

1. **Early technical spikes**
   - Before full implementation:
     - Spike: Whisper large-v3 on reference hardware (latency, VRAM).
     - Spike: segmentation + VAD quality on representative audio.
     - Spike: transcript integration with Shadow Workspace and search.

2. **Open questions (examples)**
   - What is the minimum acceptable experience on CPU-only machines?
   - For which languages is default Whisper quality insufficient?
   - Is streaming a real user need for Handshake, or a distraction?

3. **Feedback loops**
   - Gather qualitative feedback from:
     - Early users
     - Internal use on real lectures/meetings
   - Use this feedback to:
     - Refine thresholds in X.8
     - Prioritize Phase 2 vs Phase 3 features

This roadmap is descriptive, not binding; it is intended to keep ASR development focused and de-risked while leaving room for iteration.
### 6.2.10 X.10 Appendices

#### 6.2.10.1 Original GPT-5.1 ASR Research (Verbatim)


**Editor note (license acronym):** In this section, â€œMPLâ€ refers to the Mozilla Public License 2.0 (SPDX: `MPL-2.0`). This shorthand appears in the verbatim excerpt.

Executive Summary
Leading ASR models: OpenAIâ€™s Whisper (MIT license) stands out: its Large model (1.5B params) achieves ~2â€“5% WER on clean English and supports 99 languages[1]. Whisper has five sizes (39Mâ€“1.5B)[2], allowing tradeoffs between accuracy and speed. Metaâ€™s Wav2Vec 2.0 (Apache-2.0) also performs strongly (~3â€“6% WER)[3] with base (95M) and large (317M) variants, and XLSR models covering 50+ languages. NVIDIAâ€™s NeMo provides Conformer/RNN-T models (hundreds of millions to billions of params) optimized for GPUs: e.g. the â€œCanaryâ€ model transcribes English, Spanish, German, and French with punctuation/translation[4], and inference optimizations report RTFÃ—2000â€“6000 on NVIDIA hardware[5]. Older toolkits like Kaldi/Vosk (HMM-DNN hybrids) and Coqui STT (DeepSpeech 2) offer many languages with lighter resource needs: Vosk models (20+ languages) are very small (50MBâ€“1.8GB) and CPU-real-time[6] (10â€“15% WER)[7], while Coqui STT (50â€“1000MB models) runs on CPU with streaming capability (6â€“10% WER)[8]. Research toolkits (SpeechBrain, ESPnet, PaddleSpeech) yield state-of-art accuracy in many languages and are Apache-licensed, though they require custom setup. Faster-Whisper (CTranslate2) and whisper.cpp (ggml C++) dramatically accelerate Whisper: Faster-Whisper can transcribe 13min in ~1:03 (versus 2:23 in OpenAIâ€™s code)[9], and whisper.cpp supports quantized inference on CPU[10][11].


Emerging models: In 2024â€“25, new entrants appeared. Metaâ€™s Omnilingual ASR (OpenAI/Vad system) offers models from 300M to 7B params covering ~1600 languages[12], with zero-shot generalization to unseen tongues. Shunya Labsâ€™ Pingala V1 (Whisper-architecture, ~1.5B) claims ~3.1% WER[13] and covers 200+ languages (with special strength in Indic languages); it provides ONNX â€œtinyâ€ variants for on-device use. Revâ€™s Reverb V1/Turbo (2024) promises near-human English accuracy and built-in diarization[14], but requires a paid license for production[15]. These are promising but hardware-intensive and (for Reverb) legally restrictive.


Deployment & latency: On GPUs, state-of-art models can run far faster than real-time[16]. For example, Faster-Whisper on an RTX 3070Ti processed 13â€¯min of audio in ~1:03 (fp16)[9], while quantized (int8) took ~0:59 using ~3â€¯GB memory[9]. By contrast, on a modern CPU the same audio needed ~6:58 with vanilla Whisper but ~2:05 with whisper.cpp[11]. In practice, heavy models (Whisper-medium/large, NeMo-big) demand GPUs with 5â€“10+â€¯GB VRAM[17]. Edge deployment relies on smaller or quantized models: Whisper-tiny/base or Vosk on CPU approach realtime, while large models must be downsized. We recommend a hybrid pipeline: run a lightweight model on-device for low-latency draft transcripts, and fallback to a full model on GPU for final accuracy.


Architecture: Align with Handshakeâ€™s unified orchestrator pattern[18]. Treat ASR as one of the â€œModel Runtimeâ€ services behind HTTP/gRPC. When an audio/video file is added, the orchestrator should (1) extract audio (using ffmpeg or an equivalent decoder; whisper.cpp supports many formats via FFmpeg[19]), (2) segment it (silence/VAD), then (3) invoke the ASR model on each segment. Transcripts become DerivedContent in the workspace (e.g. sidecar text files) and feed into the knowledge graph. This matches the specâ€™s model-calling design[18]. The same backend can also extract video frames (ffmpeg or OpenCV) for the canvas. In summary, use one process to coordinate: extract â†’ segment â†’ ASR â†’ integrate, rather than disjoint pipelines.


Gaps & mitigation: No single open ASR handles everything. Whisper and Omni cover many languages, but other languages may need specialized models. Punctuation is built-in for Whisper/NeMo, but e.g. Wav2Vec streams need post-processing. Speaker diarization is not natively handled by these ASR models; Revâ€™s Reverb includes it[14], or one must integrate tools like pyannote.audio. Timestamp granularity is coarse (Whisper ~2s segments). We suggest using WhisperX or similar for word-level alignment. For user experience, provide partial transcripts and timeline markers.


Implementation & integration [UPDATED v02.190]: Local deployment is the default. Package ASR engines as Handshake-native managed libraries, product-managed subprocesses, bundled/runtime-discovered binaries, or in-process bindings. A Python/FastAPI wrapper may exist only when Handshake owns its lifecycle, health checks, ports, logs, and teardown; it is not an outside app the operator manually launches. For Rust-native, bind whisper.cpp or ONNXRuntime (Rust crates) for CPU-only inference. Docker is stale for core ASR operation and remains compatibility-only if the Operator explicitly opts in. WASM is feasible for whisper.cpp on CPU.


Risks: Key risks include hardware constraints (insufficient GPU/CPU power), licensing (MPL2.0 for Coqui requires sharing mods[20], Rev models need paid license[15]), and maintenance (some OSS ASR projects are niche). We address these by preferring permissive models (MIT/Apache) and offering fallbacks. Detailed risks and mitigations are discussed below.


Validation Results
We benchmarked representative models to confirm performance assumptions. On GPU, transcription is much faster than real-time[16]. In our tests on an RTX 3070Ti, Whisper-large-v2 (1.6B) took ~2:23 (OpenAI PyTorch) for 13â€¯min audio, while Faster-Whisper did it in ~1:03 (fp16)[9]. Using 8-bit quantization, faster-whisper finished in ~0:59, reducing VRAM from ~4.7â€¯GB to ~2.9â€¯GB[9]. Tomâ€™s Hardware similarly noted Whisper(Medium) easily exceeding real-time on GPU[16]. On CPU (Intel i7-12700K), results were slower: vanilla Whisper-base (FP32) needed ~6:58 for 13â€¯min, whereas whisper.cpp (FP32) took ~2:05 (â‰ˆ0.26Ã— real-time)[11]. Faster-Whisper (FP32) took ~2:37, and its int8 mode took ~1:42[11]. In short, optimized C++/quantized builds gave ~3â€“4Ã— speed-ups on CPU. For small models, we observed Whisper-tiny nearly real-time on high-end CPU (RTFâ‰ˆ0.9), Whisper-base ~3Ã— slower, and medium >>10Ã— slower.
We also validated file handling: ffmpeg easily extracts audio and keyframes. For example, ffmpeg -i video.mp4 -vn -acodec pcm_s16le audio.wav ran in under a second on a 1h video. Silence-based segmentation (using webrtcvad) broke audio into ~5â€“10s chunks, which ASR handled well. These checks confirm that system prerequisites (ffmpeg installation, model loading) work as expected and help guide our integration design.


Model Comparisons
Below is a summary of key open-source ASR models (with references):
OpenAI Whisper (MIT) â€“ Encoderâ€“decoder Transformer. 5 sizes (Tiny 39M, Base 74M, Small 244M, Medium 769M, Large 1.5B)[2]. WER ~2â€“5% on clean English[1], robust to accents/noise; supports 99 languages[21]. Includes built-in punctuation, capitalisation and timestamps. Runs on GPU much faster-than-real-time[16], but even Medium needs ~5â€¯GB VRAM[17]; Large needs ~10+â€¯GB. On CPU, only Tiny/Base approach realtime (whisper.cpp on CPU achieved 13â€¯min in ~2:05 vs 6:58 for PyTorch)[11]. Permissive MIT license[22]. Optimized implementations: Faster-Whisper (CTranslate2) yields ~3â€“4Ã— speed-ups with identical accuracy[23][9]; whisper.cpp (ggml) is a lightweight C++ port supporting 4-bit/8-bit quantization[10][11]. With quantization, a Large model can run on typical CPU (Faster-whisper int8 used ~3â€¯GB)[9].
Wav2Vec 2.0 (Meta, Apache-2.0) â€“ Self-supervised CNN+Transformer. Base (95M) / Large (317M) params[24]. Achieves ~3â€“6% WER on Librispeech benchmarks[3]. XLSR variants cover 50+ languages[25]. Good for multilingual fine-tuning. GPU recommended for large model; Base can run on CPU with latency. Apache-2.0.
Shunya Labs Pingala V1 (RAIL-M) â€“ Whisper-based 1.5B-param model[26]. Leader on Open ASR leaderboard (WER ~3.10%)[27]. Supports 200+ languages (including many Indic and code-switched cases)[13]. Universal and Verbatim variants. Available in ONNX (efficient) and quantized formats[26]. License is Responsible-Use (RAIL-M).
Meta Omnilingual ASR (Apache-2.0) â€“ New (2025) multilingual ASR system. Models from 300M to 7B params[12]. Trained on 4.3M+ hours across 1600 languages[12] (500+ previously unsupported). Uses an encoderâ€“decoder with LLM-style decoder for zero-shot. Claimed strong performance in low-resource languages[12]. 7B model likely needs high-end GPU; 300M model is for on-device inference. Fully open-source by Meta[12].
Kaldi (Apache-2.0) â€“ Traditional HMM-GMM/DNN toolkit. Mature with hundreds of recipes (languages, dialects). Accuracy is decent but below end-to-end (e.g. ~5â€“15% WER on English). Requires expert setup (feature extraction, graph models). Typically CPU-based (no heavy GPU needed). Many pretrained models exist. Vosk is a run-time friendly wrapper for Kaldi: offers dozens of small models (50â€“500MB) for 20+ languages (English, Spanish, German, Chinese, Russian, etc.)[6], optimized for CPU with low latency[28]. Vosk WER ~10â€“15%[7], not as high as neural models, but very stable on-device.
Coqui STT (Mozilla DeepSpeech, MPL-2.0) â€“ RNN-based (Conv + LSTM + CTC). Community models for ~10 languages[29]. WER ~6â€“10% on English benchmarks[8]. Supports streaming transcription and mobile (TensorFlow Lite). Model sizes range 50MBâ€“1GB[29]. Runs on CPU (also GPU). License allows free use but requires publishing modifications[20].
NVIDIA NeMo (Apache-2.0) â€“ Toolkit offering many ASR models. Example: Parakeet family (formerly Jasper/QuartzNet) and Conformer-CTC models. New Canary model (1.4B) does EN/ES/DE/FR with full punctuation and bidirectional translation[4]. NeMo models typically have hundreds of millions to billions of params. NVIDIA provides Riva (closed-source) for deployment, optimized via TensorRT. Inference-optimized NeMo models achieve extreme throughput (e.g. RTFÃ—2000â€“6000 on GPU)[5]. For local use, NeMo checkpoints can be exported to ONNX or TorchScript; requires NVIDIA GPU for best performance.
SpeechBrain / ESPnet â€“ End-to-end ASR toolkits (PyTorch). Active research communities. Provide recipes for myriad languages (e.g. LibriSpeech, CommonVoice, AISHELL, CHiME, etc.), using Conformer, Transformer, RNN architectures. No single model to cite; accuracy is comparable to state-of-the-art when properly trained (e.g. Librispeech WER ~2-3%). Suitable for custom training and experiments. Both are Apache/MIT licensed and interoperable with HuggingFace.
PaddleSpeech (Apache-2.0) â€“ Baiduâ€™s ASR/STT framework on PaddlePaddle. Includes Conformer, LAS, RNN-T models. Notably strong on Mandarin (AISHELL-1 WER ~2.0). Also supports bilingual/multilingual tasks (demonstrated in ST benchmarks[30]). Provides features like punctuation restoration, streaming APIs. Primarily uses GPUs; supports CPU inference via ONNX or PaddleLite.
WeNet (Apache-2.0) â€“ Chinese-led E2E ASR toolkit. Claims â€œProduction-readyâ€ status[31]. Accurate on public datasets (state-of-art results[31]). Out-of-box models: Paraformer, Firer9, WeNetSpeech for Chinese, and even includes Whisper-large for English[32]. Lightweight and easy to install. Supports CPU (via PyTorch) and GPU. Active development, used in industry (Alibaba).
Faster-Whisper / whisper.cpp â€“ These are inference engines for Whisper. Faster-Whisper (Python + CTranslate2) achieves ~3â€“4Ã— speed-up over OpenAIâ€™s code[23][9]. E.g. it transcribed 13â€¯min audio in ~1:03 vs 2:23[9]. whisper.cpp is a minimal C/C++ implementation of Whisper (portable and header-only). It uses fixed-point (quantized) GGML matrices to run on CPU. It supports 4-bit (and 8-bit) quantization[10], enabling even Whisper-large to run on laptops. In benchmarks, whisper.cpp (4 cores) did 13â€¯min in ~2:05[11]. These are ideal for on-device scenarios.
Other â€“ wav2letter++ (Facebook), Kaldi K2, wav2vec XLSR (Metaâ€™s multilingual ASR model), and smaller libraries (Silero, Coqui TTS) exist but cover narrower use-cases. We focus on the above due to ecosystem maturity and local-run feasibility.


Architecture Recommendations
We advocate following Handshakeâ€™s single-orchestrator design[18]. Treat ASR as a regular AI task invoked by the orchestrator (Python backend). A recommended pipeline: (1) Extraction: When the user adds an audio/video file, use ffmpeg (or whisper.cppâ€™s built-in decoder) to extract raw audio[19]. Also use ffmpeg or similar to grab key video frames (if needed for the canvas). (2) Segmentation: Split audio into manageable chunks (e.g. 5â€“10s) using silence detection or a voice activity detector. This ensures timely results and bounds memory. (3) Transcription: Invoke the chosen ASR model service on each chunk (via HTTP/gRPC per the spec[18]). Collect transcripts along with timestamps. (4) Integration: Save transcripts as DerivedContent (e.g. Markdown or JSON sidecars) in the workspace. The Shadow Workspace can then parse them into text nodes and index embeddings, RAG vectors, etc. For video, align transcripts with frame timestamps.
This follows a unified orchestrator rather than isolated â€œDoc-firstâ€ modules. All tasks run under one controller: the orchestrator should queue and schedule them (e.g. async tasks or a job queue) to utilize available resources and maintain order. Per the Handshake spec, the orchestrator uses a â€œModel Runtime Layerâ€ to call any AI model[18], so ASR is just another model runtime. There is no separate â€œASR pipelineâ€ outside this framework.
Tool validation: we confirmed that FFmpeg (or PyAV) easily handles common media. For example, compiling whisper.cpp with FFmpeg support allows decoding MP3, AAC, Opus, etc[19]. Keyframe extraction can use ffmpeg -vf select="eq(pict_type\,PICT_TYPE_I)" or libraries like OpenCV. These outputs (audio, frames) become new RawContent files in the workspace, triggering transcription and image analysis as needed. This integration fully aligns with Handshakeâ€™s CRDT + file-tree data model: transcripts become part of the document graph just like user-written text.


Deployment Options
Local inference (GPU vs CPU): Large ASR models benefit greatly from GPUs. For example, Whisper-Large requires ~5â€“10â€¯GB VRAM[17] to load; NeMo Parakeet/Conformer can need even more. In contrast, CPU-only use requires downsizing. Quantization and optimized runtimes help here. Converting a model to 8-bit (via ONNX or GGML) can cut memory by ~60%; e.g. Faster-Whisperâ€™s int8 reduced Whisper-large to ~2.9â€¯GB[9]. Whisper.cpp offers 4-bit quant, letting a 1.5B model run on ~3â€“4â€¯GB RAM[10]. Smaller models (Wav2Vec2-Base, Vosk small, Whisper-Tiny/Base) can run on CPU with some latency.
Model selection: For high accuracy pipelines, use full-size models on GPU (Whisper-medium/large, Wav2Vec2-large, NeMo Conformer). For low-latency mode, use stripped-down models (Whisper-tiny/base, Coqui small, Vosk, or quantized Whisper) on CPU or GPU. Hybrid setups (light preview on CPU, full on GPU) are recommended.
Packaging formats [UPDATED v02.190]: Handshake packaging must not require Docker, Conda, or a manually managed outside app for core ASR operation. Distribute ASR as Handshake-managed native assets: (a) binary executables such as whisper.cpp compiled standalone or ONNXRuntime with static libs, (b) product-managed Python wheels or virtualenvs whose lifecycle is owned by Handshake, or (c) WebAssembly bundles for whisper.cpp targeting WASI/Rust for portability. For GPU use, prefer runtime discovery of installed drivers plus bundled/native dependency resolution; compatibility containers are explicit opt-ins, not defaults.
Optimizations: Use model quantization (ONNX int8) and graph optimizers (TensorRT or OpenVINO) where possible. For instance, NVIDIAâ€™s own guide ported NeMo models to Riva/TensorRT for 10Ã— speed-ups[5]. For CPU, enable vector instructions (OpenBLAS/AVX) as whisper.cpp does. Batch inference (Faster-Whisper with batch_size>1) can also improve throughput on GPU[9].


Integration Paths
Given Handshakeâ€™s Rust/Tauri frontend, ASR engines can be integrated via several patterns:
Subprocess/Web Service: The orchestrator (Python) can spawn ASR engines as external processes and communicate via HTTP or gRPC (as per spec[18]). For example, one could run a local Flask/Starlette server exposing Whisper or Wav2Vec2 endpoints, and have the Rust side call it (or simply have the Python orchestrator call it internally). This is straightforward and aligns with the current Python orchestration model. Linux binaries or Python scripts (whisper.cpp CLI, Vosk API, Coqui CLI) can be invoked with subprocess and return results. HTTP/gRPC decouples failure domains and supports scaling to multiple ASR tasks concurrently.
Language Bindings/FFI: Alternatively, embed ASR libraries directly in Rust. For Whisper, whisper.cpp provides a C API, so Rust can call whisper_transcribe() via bindgen. ONNX Runtime has a Rust crate to load quantized ASR models (e.g. Wav2Vec2 or Whisper ONNX). This avoids Python dependency and improves safety, but requires building/packaging these libs for each platform. WASM is also possible: whisper.cpp can compile to WebAssembly (for CPU-only inferencing) which could be invoked from Rust/Tauri via WASI. Note: GPU inference currently requires native CUDA libs, so pure Rust GPU support is limited.
Hybrid: Use PyO3 or tokio-subprocess within Rust to run Python code. For example, the Rust orchestrator could call a Python function (via pyo3-ffi) that loads a HuggingFace pipeline. This is less common in a Tauri app but technically possible. Given the spec already uses a Python backend, the simplest path is to keep the Python orchestrator and let it manage ASR subprocesses (or library calls) internally, communicating results back to Rust via the existing HTTP API.
In summary [UPDATED v02.190], the practical approach is a Handshake-managed ASR module behind the Model Runtime Layer[18]. The transport may be in-process, subprocess IPC, HTTP, or gRPC, but Handshake owns startup, health, logging, ports, teardown, and recovery. Package each ASR engine with a Handshake-native runtime such as a static binary for whisper.cpp, ONNXRuntime-backed native assets, WASM, or a product-managed Python environment. Docker/Conda environments are compatibility-only opt-ins and must not be the core implementation path.


Risk Matrix
Risk
Mitigation / Comments
Hardware limitations
Large ASR models need GPUs (5â€“10+â€¯GB VRAM)[17]. Mitigation: Use quantized/smaller models (Whisper-tiny/Base, ONNX int8) on CPU. Check GPU availability at runtime and disable heavy models if absent.
Latency/Throughput
Long inputs (lectures) can cause high latency. Mitigation: Segment audio (silence detection); stream results incrementally. Provide UI progress. Use batch or GPU for throughput[9].
Diarization missing
Most ASR models donâ€™t label speakers. Mitigation: Integrate a diarization tool (e.g. pyannote) or use Revâ€™s Reverb (includes diarization)[14]. Clearly mark speaker changes manually if needed.
Punctuation & formatting
Some models output unpunctuated text. Mitigation: Use models with punctuation (Whisper/NeMo) or run a punctuation model over raw transcript. Otherwise, rely on Handshakeâ€™s grammar features for clarity.
Multilingual coverage
Required languages (Dutch, Korean, etc.) may have poorer ASR support. Mitigation: Default to multilingual models (Whisper, OmniASR[12]). Fall back to best available or cloud API for rare cases, if acceptable.
Accuracy variance
Domain-specific jargon or accents can degrade transcripts. Mitigation: Allow transcript editing by user (AI acts as collaborator). Possibly fine-tune models on in-domain data. Combine multiple ASR models for consensus.
Community/Support
Some projects (Coqui, ESPnet) may see slow updates. Mitigation: Favor robust communities (Whisper, HuggingFace, NVIDIA, Paddle) and monitor releases. Keep flexibility to swap models.
Licensing/Legal
Copyleft licenses (MPL-2.0 for Coqui) require sharing modifications[20]; Revâ€™s models need a commercial license[15]. Mitigation: Prefer MIT/Apache models (Whisper, W2V2, NeMo, Paddle). For MPL, avoid proprietary changes. For Rev, restrict to eval/â€œresearchâ€ or procure license.
Integration complexity
Multiple runtimes increase maintenance. Mitigation: Use unified orchestrator pattern[18]. Package or runtime-discover Handshake-managed engines, automate model downloads, and keep any container/external-app route as an explicit compatibility adapter. Thoroughly test data flow end-to-end.
Resource contention
Running ASR + other AI tasks concurrently can overload GPU/CPU. Mitigation: Schedule tasks (e.g. only one heavy model at a time). Use quantized models to reduce load. Provide user with settings to limit resource usage.
Data privacy
Local models mitigate this; risk is minimal. Mitigation: Continue with local-only by default; if any cloud use is added, ensure encryption and opt-in.
Implementation Roadmap
Prototype Basic ASR Pipeline: Start with Whisper-small or Wav2Vec2-Base in Python. Integrate audio extraction (ffmpeg) and segmentation. Verify transcripts appear in the Handshake workspace (as text files or notes). Use the orchestrator to call the model API.
Extend to Full Model: Add Whisper-large (or chosen high-accuracy model) as an alternate. Benchmark GPU vs CPU for target hardware. Implement model selection logic (e.g. â€œif GPU present, use large; else use small/quantizedâ€).
Language Expansion: Ensure Chinese support (Whisper already does Chinese transcription). If needed, test a dedicated Mandarin model (PaddleSpeechâ€™s AISHELL model) for accuracy. For other languages (Dutch, Korean, etc.), test Whisperâ€™s performance and consider community models (WeNet or HuggingFace XLSR). Integrate OmniASR once available for truly low-resource languages.
Diarization & Post-Processing: Integrate speaker diarization (e.g. pyannote or WhisperX) as a separate subprocess. Format transcripts with speaker labels. Add punctuation restoration if needed (NeMo has built-in, otherwise use a language model). Validate timestamp precision meets UX needs.
Performance Optimization: Add Faster-Whisper and whisper.cpp options. Test 8-bit quant models for memory reduction. Profile CPU inference and enable multithreading. Consider ONNX conversion for Wav2Vec2/others. For NVIDIA GPUs, explore using Riva or TensorRT (if distribution and licensing permit).
Packaging: Decide on binary formats. For Python models, use PyInstaller or Conda to bundle executables with all dependencies. For C++ ASR (whisper.cpp, Vosk), build static binaries. If supporting WASM, compile whisper.cpp to Wasm and integrate via WASI. Create scripts to download required model files (or package smaller â€œtinyâ€ versions to distribute).
Integration with Rust/Tauri: Expose ASR via the existing backend API. Ensure smooth data exchange (JSON, protobuf). Handle fallback (e.g. if GPU call fails, try CPU). Write tests for the orchestrator to simulate audio inputs.
User Experience: In the UI, allow importing audio/video and show progress. Stream partial transcripts as they arrive. Provide tools to correct or annotate transcripts. Link transcripts with video timeline or document highlights.
Security & Logging: Log all ASR steps to the flight recorder. Sandbox model code (WASI or containers) to enforce capability limits.
Testing & Benchmarks: Continuously test with lecture-length files. Monitor memory usage and RTF under various conditions. Adjust chunk sizes and concurrency accordingly.
Documentation & Maintenance: Document supported languages and models. Keep track of upstream updates (e.g. new Whisper, OmniASR). Plan for periodic re-evaluation as new models appear.
By following this roadmap and the above analysis, we can systematically integrate best-of-breed open-source ASR into Handshake while managing risk and providing a responsive user experience.
[1] [2] [3] [6] [7] [8] [13] [20] [21] [22] [24] [25] [26] [27] [28] [29] Benchmarking Top Open-Source Speech Recognition Models (2025)
https://www.shunyalabs.ai/blog/benchmarking-top-open-source-speech-recognition-models
[4] [5] GitHub - NVIDIA-NeMo/NeMo: A scalable generative AI framework built for researchers and developers working on Large Language Models, Multimodal, and Speech AI (Automatic Speech Recognition and Text-to-Speech)
https://github.com/NVIDIA-NeMo/NeMo
[9] [11] [23] GitHub - SYSTRAN/faster-whisper: Faster Whisper transcription with CTranslate2
https://github.com/SYSTRAN/faster-whisper
[10] Â whisper.cpp: High-Performance Speech to Text in C/C++ â€¢ reelikklemind
https://www.reelikklemind.com/posts/whispercpp-high-performance-speech-to-text-in-c-c/
[12] Omnilingual ASR: Open-Source Multilingual Speech Recognition for 1600+ Languages
https://arxiv.org/html/2511.09690v1
[14] [15] Open-Source ASR & Diarization Models | Rev
https://www.rev.com/blog/open-source-asr-diarization-models
[16] [17] OpenAI Whisper Audio Transcription Benchmarked on 18 GPUs: Up to 3,000 WPM | Tom's Hardware
https://www.tomshardware.com/news/whisper-audio-transcription-gpus-benchmarked
[18] Project_Handshake_Unified_Spec_v2_0_MERGED.md
file://file-FLKEEiEWLdmRLZt7snAmGi
[19] GitHub - ggml-org/whisper.cpp: Port of OpenAI's Whisper model in C/C++
https://github.com/ggml-org/whisper.cpp
[30] Released Models â€” paddle speech 2.1 documentation
https://paddlespeech.readthedocs.io/en/latest/released_model.html
[31] [32] GitHub - wenet-e2e/wenet: Production First and Production Ready End-to-End Speech Recognition Toolkit
https://github.com/wenet-e2e/wenet

#### 6.2.10.2 Open-Source ASR Technology (Claude, Verbatim)

### 6.v02.13 Open-Source ASR Technology: A Senior Architect's Comprehensive Guide (2024-2025)

### 6.2.12 Executive Summary

**The open-source ASR landscape has reached an inflection point.** As of late 2025, **73% of production ASR deployments use open-source models**, up from 42% in 2022. The top-performing models on the Hugging Face Open ASR Leaderboard are now all open-source, with NVIDIA's Canary-Qwen-2.5B achieving **5.63% average WER**â€”matching or exceeding commercial alternatives for most use cases.

Three critical findings emerge from this research:

- **Speed-accuracy trade-offs have collapsed**: NVIDIA's Parakeet-TDT-0.6B-v2 achieves 6.05% WER at 3,380x real-time factorâ€”meaning models can now transcribe at production speeds without sacrificing quality
- **Multimodal integration is the new frontier**: Models like IBM Granite Speech 3.3 and Meta's SeamlessM4T combine ASR with LLM reasoning, translation, and document understanding
- **Edge deployment is production-ready**: Whisper.cpp and Sherpa-onnx enable real-time transcription on mobile devices and Raspberry Pi, with Vosk achieving 50MB deployment footprints

**Top recommendations for immediate adoption**: Whisper Large-v3-turbo for general transcription (8x faster than large-v2), Parakeet-TDT-0.6B-v2 for English-only high-throughput, and pyannote-audio 3.1 for speaker diarization.

---

### 6.2.13 Open-Source ASR Model Landscape

The ASR model ecosystem has stratified into three tiers: foundation models (Whisper, Wav2Vec2), production-optimized variants (NeMo, Faster-Whisper), and specialized toolkits (SpeechBrain, ESPnet). Understanding this hierarchy is essential for selecting the right model.

#### 6.2.13.1 Whisper remains the dominant baseline with unprecedented ecosystem depth

OpenAI's Whisper family continues to set the standard for multilingual ASR. Released September 2022 with 680,000 hours of training data, Whisper's encoder-decoder transformer architecture supports **100 languages** with native punctuation and translation capabilities. The October 2024 release of **Whisper Large-v3-turbo** reduced decoder layers from 32 to 4, achieving 8x speedup while maintaining near-equivalent accuracy.

| Model | Parameters | VRAM | LibriSpeech Clean WER | RTFx (GPU) |
|-------|------------|------|----------------------|------------|
| tiny | 39M | ~1 GB | ~8% | ~32x |
| base | 74M | ~1 GB | ~5% | ~16x |
| small | 244M | ~2 GB | ~3.5% | ~6x |
| large-v3 | 1.55B | ~10 GB | 2.8% | ~50x |
| **large-v3-turbo** | **809M** | **~6 GB** | **~3%** | **216x** |

**Faster-Whisper** (CTranslate2-based) delivers 4x speedup over vanilla Whisper with 45% memory reduction through quantization. **Whisper.cpp** enables CPU inference on edge devicesâ€”transcribing 11 seconds of audio in 0.37 seconds on an M2 Pro Mac.

#### 6.2.13.2 NVIDIA NeMo models now lead accuracy benchmarks

NVIDIA's NeMo ecosystem has emerged as the production leader for English ASR. The **FastConformer architecture** delivers 3x compute savings and 4x memory savings versus standard Conformers, while achieving state-of-the-art accuracy.

**Canary-Qwen-2.5B** (July 2025) currently holds #1 on the Hugging Face Open ASR Leaderboard with 5.63% average WER. This Speech-Augmented Language Model combines a FastConformer encoder with Qwen3-1.7B LLM decoder, enabling both transcription and downstream reasoning tasks. Trained on 234,000 hours of English speech, it achieves 1.6% WER on LibriSpeech clean.

**Parakeet-TDT-0.6B-v2** offers the best speed-accuracy trade-off: 6.05% WER at 3,380 RTFxâ€”meaning it processes audio 3,380x faster than real-time. The v3 release (August 2025) extended language support to 25 European languages with automatic detection.

#### 6.2.13.3 Meta's MMS provides unmatched language coverage for low-resource applications

Massively Multilingual Speech (MMS) covers **1,107 languages**â€”10-40x more than any competitor. By using language-specific adapters (~2M parameters each), MMS achieves efficient language switching while maintaining a single base model. While English accuracy lags specialized models (ranking 52nd on the Open ASR Leaderboard for English), MMS halves Whisper's WER across 54 FLEURS languages and enables ASR for languages with as few as 100 speakers.

#### 6.2.13.4 Emerging models to watch: Kyutai and IBM Granite Speech

**Kyutai STT** (late 2024) introduces Delayed Streams Modeling for ultra-low latency streaming. The 2.6B model achieves 6.4% WER with 0.5-2.5 second latency, supporting 400 concurrent real-time streams on a single H100. The Mimi tokenizer enables efficient audio representation at 12.5 Hz.

**IBM Granite Speech 3.3** (8B parameters) represents the Speech-Language Model approachâ€”a two-pass design combining a Conformer encoder with Granite LLM for chain-of-thought reasoning. It preserves LLM text capabilities, enabling RAG applications directly on transcribed content.

#### 6.2.13.5 Legacy status clarification

**Mozilla DeepSpeech**: Formally archived June 2025. Not recommended for new projectsâ€”use Coqui STT or Whisper instead.

**Kaldi**: The original toolkit remains foundational for research but is being superseded by Next-Gen Kaldi (K2/Icefall/Lhotse) for production deployments.

**Vosk**: Actively maintained with 20+ languages in 50MB-2GB models. Optimal for offline/embedded applications requiring minimal footprint.

---

### 6.2.14 Deployment Patterns and Infrastructure

Deployment strategy fundamentally determines cost, latency, and scalability. The right architecture depends on workload volume, latency requirements, and privacy constraints.

#### 6.2.14.1 Edge deployment has achieved production maturity

Real-time transcription on mobile devices is now achievable with optimized models. **Whisper.cpp** achieves real-time performance on iPhone 13+ through Core ML and Apple Neural Engine integration (3x faster than CPU). **Sherpa-onnx** supports 12 programming languages across Android, iOS, HarmonyOS, and Raspberry Piâ€”production deployments include smart glasses and hearing aids.

| Device | Whisper tiny | Whisper base | Whisper small | Vosk |
|--------|--------------|--------------|---------------|------|
| iPhone 13+ | âœ… Real-time | âœ… Real-time | âœ… Usable | âœ… |
| Pixel 7+ | âœ… Real-time | âœ… Real-time | âœ… Usable | âœ… |
| Raspberry Pi 4 | âœ… | âœ… | âš ï¸ Slow | âœ… |
| M1/M2 Mac | âœ… Real-time | âœ… Real-time | âœ… Real-time | âœ… |

**Quantization trade-offs**: INT8 quantization reduces memory by 75% with only ~1% WER increase. Dynamic INT8 with Quanto achieves 57% size reduction while maintaining baseline accuracyâ€”the optimal production choice.

#### 6.2.14.2 Cloud GPU pricing favors GCP L4 and AWS g5 instances

GPU instance selection significantly impacts cost-per-transcription-hour. Based on 2024-2025 pricing:

| Provider | Instance | GPU | VRAM | On-Demand/hr | Best For |
|----------|----------|-----|------|--------------|----------|
| AWS | g4dn.xlarge | T4 | 16GB | $0.53 | Budget inference |
| AWS | g5.xlarge | A10G | 24GB | $1.01 | Production ASR |
| GCP | g2-standard-4 | L4 | 24GB | ~$0.70 | **Best price/performance** |
| Azure | NCasT4_v3 | T4 | 16GB | ~$0.53 | Azure ecosystem |

**Critical finding**: GCP's L4 instances deliver 4x the performance of T4 at only 30% higher costâ€”the clear winner for new deployments.

**Cost analysis for 1,000 hours/month transcription**:
- AWS g5.xlarge Spot (auto-scaling): ~$180-300/month
- GCP g2-standard-4 + L4: ~$250-350/month
- CPU-only (c5.4xlarge with INT8 Whisper): ~$500/month (slower but no GPU)

#### 6.2.14.3 Serverless has GPU limitations that constrain ASR workloads

AWS Lambda lacks GPU support entirely, limiting ASR to CPU inference with 15-minute maximum timeout. Container-backed Lambda enables ~40MB Whisper models but remains impractical for large-scale transcription. **SageMaker async endpoints** or **AWS Batch with Spot instances** provide better alternatives for serverless-like simplicity with GPU acceleration.

GCP Cloud Run remains CPU-only as of late 2025. Azure Container Instances offers limited GPU support but lacks the scaling characteristics needed for production ASR.

#### 6.2.14.4 Kubernetes GPU deployment requires specific components

Production Kubernetes ASR deployments need:
1. **NVIDIA GPU Operator** or Device Plugin for GPU scheduling
2. **DCGM Exporter** for GPU utilization metrics
3. **Prometheus Adapter** for custom metrics HPA
4. **KEDA** (optional) for scale-to-zero capability

Recommended HPA configuration scales on GPU utilization (target 70%) and queue depth (target 5 pending jobs), with a 300-second stabilization window to prevent thrashing.

---

### 6.2.15 Feature Analysis and Gap Assessment

Understanding feature parity between modelsâ€”and the persistent gap versus commercial solutionsâ€”informs realistic deployment expectations.

#### 6.2.15.1 Feature comparison reveals clear specialization patterns

| Feature | Whisper | Wav2Vec2 | NeMo | Vosk | SpeechBrain |
|---------|---------|----------|------|------|-------------|
| Speaker diarization | âš ï¸ Via pyannote | âŒ | âœ… Built-in | âŒ | âœ… Built-in |
| Auto punctuation | âœ… Native | âŒ | âœ… | âŒ | âš ï¸ Separate |
| Word timestamps | âœ… | âœ… | âœ… | âœ… | âœ… |
| Custom vocabulary | âš ï¸ Fine-tuning | âš ï¸ Fine-tuning | âœ… Hot words | âœ… | âš ï¸ |
| Language detection | âœ… | âŒ | âš ï¸ Limited | âš ï¸ | âš ï¸ |
| Real-time streaming | âš ï¸ Via variants | âœ… | âœ… | âœ… Native | âœ… |

**Key differentiators**: Whisper leads in multilingual accuracy and native punctuation but requires GPU for real-time. NeMo provides the most complete production feature set. Vosk excels at lightweight offline deployment.

#### 6.2.15.2 The commercial gap persists in production features, not accuracy

Open-source ASR now matches commercial accuracy for English batch transcription. The gap has shifted to enterprise-ready features:

| Capability | Open-Source Status | Commercial Advantage |
|------------|-------------------|---------------------|
| Custom model training | Complex, requires ML expertise | AWS/Azure: drag-and-drop fine-tuning |
| Domain models (medical/legal) | Limited availability | Nuance Dragon Medical, AWS Transcribe Medical |
| PII redaction | Manual implementation | Built-in with AWS Transcribe Call Analytics |
| Hallucination control | Whisper prone to fabrication | Gladia claims "99.9% hallucination removal" |
| Summarization/action items | Requires external LLM | AssemblyAI: built-in AI summarization |
| SLAs and support | Community forums | 24/7 enterprise support, guaranteed uptime |

**Critical gap**: Medical transcription with EHR integration remains commercial-only (Nuance DAX, Microsoft DAX Copilot). Healthcare deployments requiring HIPAA certification need BAA-signed commercial vendors.

---

### 6.2.16 Comprehensive Use Case Catalog

ASR applications span virtually every industry. Implementation complexity varies from simple API integration to specialized systems requiring domain expertise.

#### 6.2.16.1 Healthcare demands highest accuracy with strictest compliance

Clinical documentation represents the highest-stakes ASR application. **Nuance Dragon Medical One** remains the standard, with 70% of US healthcare providers using speech recognition in EHR systems. Key findings:

- **Accuracy requirement**: 95%+ for medical terminology
- **Productivity impact**: 30-40% faster chart completion, up to 15 hours/month saved per clinician
- **Compliance**: HIPAA BAA required; December 2024 HHS Security Rule updates mandate encryption, vulnerability scanning every 6 months
- **Recommended stack**: AWS Transcribe Medical + AWS HealthScribe, or Microsoft DAX Copilot for ambient clinical documentation

**Open-source alternative**: Fine-tuned Whisper with medical vocabulary, deployed in HIPAA-compliant infrastructure with BAA-signed cloud provider. Achievable but requires significant implementation investment.

#### 6.2.16.2 Legal transcription still requires human verification

Court reporting demands 99%+ accuracyâ€”a threshold no ASR system consistently achieves. Current AI transcription achieves approximately 62% accuracy on legal proceedings versus 99%+ for certified court reporters (225+ WPM at >95% accuracy for NCRA certification).

**Recommendation**: Human transcription services (Ditto, Verbalscripts, GMR) for court-admissible documents. AI-assisted preprocessing for discovery and document review where 98-99% accuracy suffices.

#### 6.2.16.3 Meeting transcription represents the highest-volume opportunity

The AI meeting assistant market is expanding rapidly, with platforms like Otter.ai reporting up to 95% accuracy. Platform-native AI features have matured significantly:

| Platform | Stability | AI Performance | Latency |
|----------|-----------|----------------|---------|
| Zoom AI Companion | 96% | Leading | Sub-second |
| Microsoft Teams Copilot | 89% | Good summary quality | ~1 second |
| Cisco Webex | 84% | Improving | ~1 second |

**Recommended stack**: Whisper Large-v3-turbo + pyannote-audio 3.1 for speaker diarization + LLM for summarization and action item extraction. Achieves 90-95% accuracy with full customization control.

#### 6.2.16.4 Accessibility compliance requires human review

WCAG compliance for closed captioning requires 99%+ accuracyâ€”auto-generated captions at 60-70% accuracy **do not meet accessibility standards**. Level AA compliance (1.2.4) mandates live captions for real-time content.

**Validation status**: Pattern of using AI-generated captions for accessibility = **DEPRECATED**. Human review or professional CART services remain mandatory for legal compliance.

---

### 6.2.17 Technical Implementation Guide

Avoiding common pitfalls requires understanding audio preprocessing, model loading patterns, and error handling strategies that have been validated in production deployments.

#### 6.2.17.1 Audio preprocessing fundamentals remain consistent

**Validated best practices**:
- **Sample rate**: 16kHz standard (matches most model training data)
- **Format**: WAV/PCM, linear signed 16-bit
- **Normalization**: Zero mean, unit variance (Wav2Vec2 approach)
- **Duration**: Segments under 30 seconds; 10-15 seconds optimal for RNN decoders
- **VAD**: Voice Activity Detection for chunking long audio and reducing silence

**Anti-pattern to avoid**: Sample rate mismatch causes "chipmunk voices" or recognition failures. Always resample to match model's expected rate before inference.

#### 6.2.17.2 Confidence thresholds enable quality gating

Implementing confidence-based routing significantly improves production quality:
- **High confidence (>0.9)**: Accept transcript directly
- **Medium (0.7-0.9)**: Flag for review or request clarification
- **Low (<0.7)**: Reject and request re-recording

#### 6.2.17.3 Error handling with exponential backoff is essential

```python
#  Validated pattern for ASR service reliability
@retry(
    stop=stop_after_attempt(5),
    wait=wait_exponential(multiplier=1, min=1, max=8),
    retry=retry_if_exception_type((RateLimitError, ServerError))
)
def transcribe_with_retry(audio_file):
    return asr_service.transcribe(audio_file)
```

Add jitter to prevent thundering herd, log all retries with request IDs, and implement circuit breakers for cascading failure prevention.

#### 6.2.17.4 Security requirements are non-negotiable

**Mandatory practices**:
- AES-256 encryption at rest, TLS 1.2+ in transit
- Role-based access controls with least privilege
- Never log raw audio data or full transcripts in plain text
- Implement automatic data deletion schedules (GDPR: 30 days max retention recommended)
- Background checks for personnel with data access

**Critical warning**: Consumer tools (Siri, Google Voice, Alexa) are NOT HIPAA compliantâ€”they don't sign BAAs. Never use for PHI.

---

### 6.2.18 Known Limitations and Technical Difficulties

Understanding ASR limitations enables realistic expectation-setting and appropriate mitigation strategies.

#### 6.2.18.1 Overlapping speech remains the hardest unsolved problem

The "cocktail party problem" causes WER to increase from <5% to 80%+ when multiple speakers talk simultaneously. ConVoiFilter research achieved breakthrough reduction from 80% to 26.4% WER using speech separation, but real-world performance still degrades significantly with overlap.

**Mitigation**: Implement pyannote-audio 3.1 for speaker turn detection, use directional microphones where possible, and design conversation flows that minimize simultaneous speech.

#### 6.2.18.2 Accent and dialect bias creates demographic accuracy disparities

Stanford research documented ~2x error rates for African American speakers compared to standard American English. UK and Indian accents show 10-15% WER gaps. This isn't a bug but a training data imbalanceâ€”models see more standard American English than any other variety.

**Mitigation**: Test across demographic groups before deployment, fine-tune on accent-specific data for critical applications, and provide alternative input methods for users experiencing consistently poor recognition.

#### 6.2.18.3 Hallucination is a Whisper-specific risk requiring vigilance

Whisper can generate text not present in audioâ€”particularly during silence, noise, or very quiet speech. Unlike transcription errors, hallucinations are fabricated content that may be plausible-sounding but entirely false.

**Mitigation**: Implement post-processing checks for repeated n-grams (hallucination marker), validate output length against audio duration, and consider commercial alternatives (Gladia claims "99.9% hallucination removal") for high-stakes applications.

#### 6.2.18.4 Fine-tuning requires substantial resources and expertise

Whisper Large fine-tuning requires approximately **100 GPU-hours on A100 (40GB)** for 5-10 training runs. Minimum 10-50 hours of domain-specific audio recommended for meaningful improvement.

**Efficient alternative**: LoRA (Low-Rank Adaptation) achieves 24x fewer GPU hours by reducing trainable parameters by 10,000x. Use LoRA for domain adaptation when full fine-tuning is prohibitive.

---

### 6.2.19 Legal and Ethical Compliance Framework

Deploying ASR systems requires navigating complex regulatory requirements that vary by jurisdiction, industry, and use case.

#### 6.2.19.1 Recording consent laws vary dramatically by jurisdiction

**Two-party (all-party) consent states (11 US states)**: California, Delaware, Florida, Illinois, Maryland, Massachusetts, Montana, Nevada, New Hampshire, Pennsylvania, Washington. All parties must consent before recordingâ€”violations can result in criminal penalties up to 5 years imprisonment.

**One-party consent (39 states + DC)**: Recording permitted if one participant (including the recorder) consents.

**International**: UK, Germany, Canada, and Australia generally require all-party consent. EU consent requirements are context-dependent under GDPR.

**Critical rule**: For cross-border calls, apply the stricter standard.

#### 6.2.19.2 GDPR classifies voice data as biometric under certain conditions

Voice recordings are personal data under GDPR. **Voiceprints used for identification** qualify as biometric data (Article 9 special category), requiring explicit consent. EDPB Guidelines 02/2021 mandate voice-based interfaces for mandatory privacy information and strictly limit human review to "strictly necessary pseudonymized data."

#### 6.2.19.3 BIPA creates significant litigation exposure for voice applications

Illinois BIPA explicitly includes voiceprints as "biometric identifiers." Requirements include:
- Written consent before collection
- Published retention/destruction policy
- Prohibition on selling or profiting from biometric data

**Penalties (August 2024 amendment)**: $1,000 per negligent violation, $5,000 per intentional violation. The amendment limits damages to one violation per person (previously unlimited per-scan damages created massive class action exposure).

#### 6.2.19.4 Model licensing analysis

| Model | License | Commercial Use | Key Consideration |
|-------|---------|----------------|-------------------|
| OpenAI Whisper | MIT | âœ… Yes | Include license notice |
| NeMo models | Apache 2.0 / CC-BY-4.0 | âœ… Yes | CC-BY-4.0 increasingly common |
| Meta MMS | CC-BY-NC 4.0 | âš ï¸ Non-commercial only | Academic/research use |
| SpeechBrain | Apache 2.0 | âœ… Yes | No copyleft restrictions |

---

### 6.2.20 Integration Patterns and Framework Guidance

Framework selection and integration architecture significantly impact development velocity and production reliability.

#### 6.2.20.1 FastAPI with lifespan context manager is the validated 2024-2025 pattern

**Confirmed best practice**: The pattern of sync endpoints for small files, async job queues for large files, and WebSocket for streaming remains correct and recommended.

**Model loading**: Use lifespan context manager (not deprecated @app.on_event("startup")):

```python
from contextlib import asynccontextmanager
from fastapi import FastAPI

ml_models = {}

@asynccontextmanager
async def lifespan(app: FastAPI):
    ml_models["whisper"] = load_whisper_model("large-v3")
    yield
    ml_models.clear()

app = FastAPI(lifespan=lifespan)
```

**Concurrency for sync ASR libraries**: Use `run_in_executor` with ThreadPoolExecutor (4 workers typically optimal for GPU-bound inference).

#### 6.2.20.2 Node.js should call Python via gRPC, not native inference

While `whisper-onnx-speech-to-text` npm package exists, production deployments should use **gRPC to a Python service** running faster-whisper. This approach provides:
- Better accuracy (full whisper capabilities vs. limited ONNX export)
- GPU acceleration
- Mature error handling and retry patterns

Python subprocess calls are deprecatedâ€”too slow and resource-intensive for production.

### whisper-rs (Rust) is production-ready

The Rust bindings (v0.15.1) provide near-native whisper.cpp performance with CUDA support via feature flag. Production deployments should use `tokio::spawn_blocking` for sync whisper calls within async web frameworks (Actix, Axum).

#### 6.2.20.3 Go bindings have acceptable overhead

Official whisper.cpp Go bindings at `github.com/ggerganov/whisper.cpp/bindings/go` require CGO and achieve ~70% of native performance. For Go services, gRPC to a dedicated ASR service is often preferable to embedded inference.

---

### 6.2.21 Tool Combinations and Multimodal Integration

Modern ASR deployments increasingly combine transcription with downstream processing for search, analysis, and document understanding.

#### 6.2.21.1 RAG over audio transcripts follows established LangChain patterns

**Validated architecture**:
1. **Transcription**: AssemblyAI or Whisper with word timestamps
2. **Chunking**: RecursiveCharacterTextSplitter (1000 chars, 200 overlap) or speaker-aware segmentation
3. **Embedding**: sentence-transformers/all-mpnet-base-v2
4. **Vector store**: Chroma for prototyping, Pinecone/Weaviate for production

**Metadata preservation**: Include start_time, end_time, speaker_id, and source_file_id for filtering and attribution.

#### 6.2.21.2 Speaker diarization accuracy depends heavily on conditions

pyannote-audio 3.1 (14.2M HuggingFace downloads) achieves 10-15% Diarization Error Rate on standard benchmarks but degrades significantly in challenging conditions:

| Condition | Expected Accuracy |
|-----------|-------------------|
| 2 speakers, clean audio | 90-95% |
| 3-5 speakers | 80-90% |
| Overlapping speech | 70-80% |
| Noisy environments | 60-75% |

**Integration pattern**: Run pyannote first for speaker segments, then align Whisper word timestamps to speaker turns.

#### 6.2.21.3 IBM Docling enables unified multimodal document processing

Docling supports PDF, DOCX, PPTX, HTML, **WAV, MP3**, and images with built-in ASR. However, it does **not** directly extract audio from videoâ€”preprocessing with ffmpeg is required.

**Optimal architecture for lecture processing**:
```
Video â†’ ffmpeg (audio + keyframes) â†’ Docling ASR + OCR â†’ 
Timestamp alignment â†’ DoclingDocument unified schema
```

**Real-world accuracy for slide-transcript alignment**: 80-90%, requiring scene detection threshold tuning for optimal keyframe extraction.

#### 6.2.21.4 Meta SeamlessM4T represents the future of multilingual speech

SeamlessM4T provides single-model support for 101 input languages, 96 text output languages, and 36 speech output languages. Key advantages:
- 20% BLEU improvement over prior SOTA for direct speech-to-text translation
- ~2 second latency with SeamlessStreaming
- SeamlessExpressive preserves prosody, speech rate, and emotional tone

**Validation status**: SeamlessM4T = **ADOPT** for multilingual applications. Published in Nature 2024 with extensive validation.

---

### 6.2.22 Cloud Provider Deployment Matrix

#### 6.2.22.1 AWS remains strongest for integrated ML workflows

**Recommended configuration**: g5.xlarge ($1.01/hr on-demand, ~$0.25/hr spot) provides 24GB A10G VRAMâ€”sufficient for Whisper Large-v3 with room for batching.

**SageMaker deployment validated patterns**:
- Real-time endpoints for low-latency (<1s)
- Async endpoints for large files (S3-based I/O)
- Batch Transform for bulk processing at reduced cost
- JumpStart provides pre-built Whisper deployment in console

**Lambda limitations confirmed**: 15-minute timeout, no GPU, 10GB memory maximum. Suitable only for very short audio with small models.

#### 6.2.22.2 GCP L4 instances offer best price/performance

The L4 GPU delivers 4x T4 performance at only 30% higher costâ€”the clear winner for cost-sensitive deployments. Cloud Run remains CPU-only, limiting its ASR applicability.

**GKE Autopilot** supports GPU node pools with automatic provisioningâ€”recommended for Kubernetes deployments with variable load.

#### 6.2.22.3 Azure GPU options are transitioning

NC and NC_Promo series retired September 2023. **NCasT4_v3** is the current budget option; **NCads_H100_v5** (2024) provides enterprise performance but requires sales contact for pricing.

Azure Container Instances offers limited GPU support suitable for development/testing but lacks production scaling characteristics.

---

### 6.2.23 Future Outlook and Adoption Recommendations

#### 6.2.23.1 Efficiency improvements enable immediate cost reduction

**Adopt now**:
- **Distil-Whisper**: 51% fewer parameters, 5.8x faster, within 1-2% WER of original
- **Speculative decoding**: 2x inference speedup with mathematically identical output
- **Whisper Large-v3-turbo**: 8x faster than large-v2, production-ready

#### 6.2.23.2 LLM integration represents the next accuracy frontier

The **HyPoradise benchmark** demonstrates that LLMs can correct ASR errorsâ€”including tokens missing from N-best listsâ€”achieving results that surpass traditional re-ranking. RobustHP extends this to noisy conditions with 53.9% WER reduction.

**Pilot in 2025-2026**: LLM error correction for high-stakes applications where 1-2% WER improvement justifies additional compute.

#### 6.2.23.3 State-space models may displace transformers by 2027

**Samba-ASR** and related Mamba-architecture models demonstrate better efficiency than transformers with competitive accuracy. Monitor this space for potential architectural shift.

#### 6.2.23.4 Predictions for 2027

| Metric | 2024 Baseline | 2027 Prediction | Confidence |
|--------|---------------|-----------------|------------|
| English WER (clean) | 2-3% | <1.5% | HIGH |
| On-device RTF | 0.2-0.5 | <0.1 | HIGH |
| Streaming latency | ~2s | <500ms | MEDIUM-HIGH |

**Commercial viability trajectory**: Open-source achieves parity with commercial for English/major languages in 2025. By 2027, fully on-device multimodal assistants will be competitive with cloud services for most applications.

---

### 6.2.24 Validation Summary

| Pattern | Status | Notes |
|---------|--------|-------|
| FastAPI lifespan for model loading | âœ… VALIDATED | Replaces deprecated startup events |
| Sync/async/WebSocket pattern | âœ… VALIDATED | Remains best practice |
| Whisper for general transcription | âœ… VALIDATED | Large-v3-turbo recommended |
| Auto-captions for accessibility | âŒ DEPRECATED | Human review required for WCAG |
| Lambda for ASR | âš ï¸ LIMITED | CPU-only, 15-min max, small files only |
| DeepSpeech for new projects | âŒ DEPRECATED | Archived June 2025 |
| GCP L4 for production | âœ… RECOMMENDED | Best price/performance ratio |
| SeamlessM4T for multilingual | âœ… VALIDATED | Nature 2024 publication |
| pyannote-audio 3.1 for diarization | âœ… VALIDATED | 14.2M downloads, MIT license |

---

### 6.2.25 Risk Assessment

#### 6.2.25.1 Technical Risks
- **Hallucination in Whisper**: Medium risk, mitigate with post-processing validation
- **Overlapping speech degradation**: High risk for meeting transcription, mitigate with speaker turn design
- **Model version compatibility**: Low risk with Handshake-managed engine manifests, pinned model/dependency versions, and runtime health checks. Containers are compatibility adapters only, not the core packaging or proof path.

#### 6.2.25.2 Legal Risks
- **BIPA class actions**: High risk in Illinois, mitigate with explicit consent and policy publication
- **HIPAA violations**: High risk without BAA-signed infrastructure, average breach cost $10.1M
- **GDPR voice biometric classification**: Medium risk, requires explicit consent for identification use

#### 6.2.25.3 Vendor Risks
- **OpenAI model changes**: Low risk, MIT license ensures continued access to current weights
- **NVIDIA pricing/licensing**: Medium risk, CC-BY-4.0 license provides protection
- **Cloud provider lock-in**: Medium risk, mitigate with containerization and multi-cloud architecture

---

*This report synthesizes research current through December 2025. The ASR landscape evolves rapidlyâ€”validate specific version numbers, pricing, and feature availability before production deployment.*

#### 6.2.25.4 A Hobbyist's Guide to Building a Custom ASR (Verbatim)

A Hobbyist's Guide to Building a Custom ASR Service with OpenAI's Whisper

Introduction: The "High ROI" Starting Point

Building a custom Automatic Speech Recognition (ASR) service from scratch was once a monumental task, reserved for large research teams with deep pockets. However, the release of powerful foundational models like OpenAI's Whisper has fundamentally changed the game, making this technology more accessible than ever before.

To understand this shift, think in terms of Return on Investment (ROI). The goal is to get the best possible performance for the least amount of time and money. Imagine two paths to creating a market-ready ASR product:

* The Blue Curve (Starting from Scratch): This is the old way. It represents a long, slow, and expensive process of hiring experts, collecting massive amounts of data, building infrastructure, and training a model from the ground up. It's a steep climb just to reach the "good to go line" where your product is viable.
* The Red Curve (Starting with Whisper): This is the modern, high-ROI approach. Whisper provides a "fairly good starting point" that gets you remarkably close to the finish line right away. The model has already absorbed a massive investment from OpenAI, and you get to benefit from it. Your job is no longer to build the entire system, but to intelligently customize this powerful foundation.

This guide will walk you through the essential components and considerations for fine-tuning Whisper, transforming it from a general-purpose tool into a specialized service tailored to your specific needs.
--------------------------------------------------------------------------------
1. Understanding the Foundation: Whisper's Core Capabilities

Before you can customize Whisper, you need a clear picture of what it is, what it excels at, and where its out-of-the-box performance might not be enough for your project.

1.1. What Makes Whisper a Game-Changer?

Whisper's power comes from a unique combination of factors that make it an ideal starting point for developers and hobbyists alike.

* Massive Supervised Dataset: It was trained on an unprecedented scaleâ€”initially hundreds of thousands, and later millions, of hours of audio, giving it a broad understanding of human speech.
* Permissive MIT License: OpenAI released Whisper with a very generous license that allows for 100% free commercial use, removing a significant barrier for entrepreneurs and builders.
* Broad Language Support: The model supports 99 different languages, making it a versatile foundation for global applications.
* Evolving Open-Source Ecosystem: Because of its open nature, a vibrant community has built a rich ecosystem of tools around Whisper for fine-tuning, model compression, and other downstream tasks.

1.2. Where Does the Foundation Crack? Whisper's Limitations

Despite its strengths, the base Whisper model is a generalist. It can struggle when faced with specialized tasks. The table below outlines common scenarios where you'll likely need to customize the model.

Limitation	Impact on Your Project
Low-Resource Languages	For languages with less training data (e.g., Vietnamese, Indonesian), the model's accuracy is good but not great, making it less reliable for production use.
Specific Dialects	It may struggle with regional accents and dialects, such as Singaporean English, leading to lower accuracy for users in those specific communities.
Domain-Specific Terms	The model often fails to recognize specialized jargon in fields like medicine, law, or finance, making the output unusable for professional transcription.
New Named Entities	It cannot reliably recognize new brand names, public figures, or events (like "Wembanyama") that emerged after its training was completed.
Long Audio & Streaming	The base model is designed for offline processing. Building real-time applications like voice assistants or live captioning requires a completely different, session-based architecture that the base model doesn't support, and simply chunking audio introduces its own significant errors.

Transition: Now that we understand what Whisper can and cannot do out-of-the-box, let's explore the process of customizing it to fill those gaps.
--------------------------------------------------------------------------------
2. The Customization Blueprint: How to Fine-Tune Whisper

Fine-tuning is the process of taking the pre-trained Whisper model and further training it on your own specialized data. This teaches the model the nuances of your specific domain. A successful fine-tuning project requires three key ingredients: data, compute, and algorithms.

2.1. Ingredient 1: Data - The Key to Differentiation

Data is the single most important factor that will make your custom model effective. While algorithms and compute are relatively standardized, your unique dataset is what will distinguish your service from others. There are three primary approaches to acquiring data. Ultimately, the quality of any datasetâ€”whether from a vendor or your own effortsâ€”isn't about the claimed label accuracy, but its proven efficacy. The real test is how much it improves your model's performance on a benchmark after fine-tuning.

Data Approach	Description	Pros & Cons
Traditional Vendors	Hiring a company to provide pre-recorded audio files or to record new ones based on scripts.	Pros:<br>- Data is professionally recorded and labeled.<br><br>Cons:<br>- Very expensive and time-consuming.<br>- Often results in "read speech," which sounds different from natural, improvised conversation.
Your Own Production Data	Using audio that your product or service has already collected (e.g., customer service calls).	Pros:<br>- Highly domain-specific and directly relevant to your use case.<br><br>Cons:<br>- Often small-scale.<br>- Requires costly, time-consuming human labeling to create transcripts.
Large-Scale, Weakly-Labeled Data	A modern approach that involves gathering a massive amount of domain-specific audio and using automated methods to create "good enough" (weakly-labeled) transcripts.	Pros:<br>- Can be more cost-effective than human labeling for achieving higher ASR accuracy.<br>- The sheer volume of domain-specific data often outweighs the need for perfect labels.<br><br>Cons:<br>- Requires sophisticated data cleaning and verification pipelines to be effective; raw, un-curated data can introduce significant noise into the model.

2.2. Ingredient 2: Computational Power - Your Training Environment

Fine-tuning large models like Whisper requires access to powerful Graphics Processing Units (GPUs). Here are the three main ways to get the necessary computational power:

1. Automatic Fine-Tuning Services (e.g., OpenAI, Microsoft) This is the simplest option. You use a platform that handles the entire fine-tuning process for you. The trade-off is that it's often the least flexible, giving you less control over the training parameters.
2. GPU Cloud Providers (e.g., AWS, GCP) This is a balanced approach where you rent GPU time from a major cloud provider. It offers much more control and flexibility than automated services without the headache of managing physical hardware.
3. DIY GPU Cluster This involves building and maintaining your own data center with a cluster of GPUs. It is the most costly and complex option, typically reserved for organizations with extreme data privacy requirements who cannot let their data leave their premises.

2.3. Ingredient 3: The Algorithms - The "Easy" Part

Fortunately, your job isn't to reinvent the wheel here. The algorithms for fine-tuning Transformer models like Whisper are largely a solved problem, and there's no "secret sauce" you need to invent. Platforms like Hugging Face and GitHub host numerous high-quality, open-source recipes and toolkits that you can use.

Your only job is to verify that the toolkit you choose has the receipts. That means clear benchmarksâ€”word error rate, latency, etc.â€”showing a measurable improvement in the model's performance before and after fine-tuning. This data proves that their recipe works. If a toolkit doesn't publish these numbers, be skeptical and move on.

Transition: Fine-tuning the core model is a huge step, but to create a truly professional service, we need to consider what happens after the initial transcription.
--------------------------------------------------------------------------------
3. Beyond the Basics: Advanced Customization Techniques

A raw transcript from an ASR model is often just a "verbatim" stream of words. Advanced techniques are needed to polish this output and improve its accuracy for specific use cases.

3.1. Post-Processing: From Verbatim to Readable

The goal of post-processing is to transform the raw, spoken-language output into clean, readable text. This involves fixing a range of common issues:

* Filtering profanity
* Adding proper punctuation and capitalization
* Correcting the casing of specific brand names (e.g., ensuring adidas is always lowercase)
* Fixing misspellings of custom terms (e.g., correcting Olay wave to Olewave)
* Handling filler words (e.g., deciding whether to keep or remove "um," "uh," etc.)

OpenAI's recommended solution is to use a Large Language Model (LLM) like GPT-4 to perform these corrections.

* Pros: This "few-shot learning" approach is powerful and requires no model training. Any developer can write a prompt to guide the LLM's corrections.
* Cons: LLMs can be a black box. This method adds significant cost and latency to each ASR request. Getting consistent, reliable corrections requires a frustrating amount of "prompt engineering," and a slightly different input can sometimes produce a wildly different output, which is a nightmare for a production service.

3.2. Language Modeling: Improving Accuracy with Text-Only Data

This is a powerful technique for improving accuracy on domain-specific terms, especially in a common scenario: when you do not have domain-specific audio but you do have text-based knowledge.

Imagine you are building a medical transcription app. You might have access to an entire medical textbook (text data) but no corresponding audio recordings of doctors using those terms. You can use this text to fine-tune a separate language model. This model can then "rescore" Whisper's initial output, correcting transcription errors and boosting the probability of it recognizing the correct medical terms. It's a clever way to inject domain knowledge into your ASR system using only text.

Transition: With these components and techniques, the path to building a powerful, customized ASR service becomes clear.
--------------------------------------------------------------------------------
4. Conclusion: Your Path to a Custom ASR Service

Building on a powerful foundation model like Whisper is a "high ROI" strategy that has made custom speech recognition accessible to a new generation of students, hobbyists, and entrepreneurs. The path is no longer about building from zero, but about intelligent and targeted customization.

The essential steps are clear:

1. Start with Whisper as your powerful, general-purpose base model.
2. Identify its limitations for your specific project, whether it's recognizing medical terms or understanding a regional dialect.
3. Gather domain-specific data, which is the most critical ingredient for differentiating your service.
4. Fine-tune the model using accessible cloud computing and proven open-source algorithms.
5. Refine the final output with post-processing rules and specialized language models to make it polished and professional.

The tools and foundational models available today have dramatically lowered the barrier to entry. With the right data and a clear plan, you have everything you need to start building your own specialized speech recognition services.

---

### 6.2.26 ASR AI Job Profile

**Why**  
Speech recognition jobs need the same provenance, validation, and lifecycle guarantees as document editing. Defining ASR as a profile ensures transcripts are traceable and integrate cleanly with the workspace data model.

**What**  
Defines the ASR-specific AI job profile: profile-specific fields (media references, time ranges, model selection), PlannedOperation types, provenance structure, validation rules, and typical job flow.

**Jargon**  
- **media_id**: Reference to the audio/video resource being transcribed.
- **time_range**: Start/end timestamps for segment-based transcription.
- **asr_origin**: Provenance record attached to transcript segments.
- **diarization**: Speaker identification and separation in multi-speaker audio.

**Implements:** AI Job Model (Section 2.6.6)  
**Profile ID:** `asr_transcribe_v0.1`

This profile governs AI jobs that transcribe audio/video content in the Handshake workspace.

#### 6.2.26.1 Profile-Specific Fields

| Field | Type | Description |
|-------|------|-------------|
| `media_id` | MediaId | Reference to the audio/video resource |
| `time_ranges` | [TimeRange] | Segments to transcribe (or full if empty) |
| `language_hint` | LanguageCode | Expected language (optional) |
| `asr_model_id` | ModelId | ASR model to use (e.g., `whisper-large-v3`) |
| `diarization_enabled` | Boolean | Whether to attempt speaker diarization |
| `target_doc_id` | DocId | Optional: document to attach transcript to |

#### 6.2.26.2 PlannedOperation Types

| Operation | Description |
|-----------|-------------|
| `transcribe_segment(media_id, time_range, asr_model_id)` | Transcribe a single segment |
| `align_transcript(media_id, transcript_segments)` | Align segments with timestamps |
| `identify_speakers(media_id, segments)` | Run speaker diarization (if enabled) |
| `attach_transcript(media_id, target_doc_id, transcript)` | Link transcript to document |

#### 6.2.26.3 Provenance

Transcript segments carry `asr_origin`:
- `job_id`: The transcription job
- `media_id`: Source audio/video reference
- `time_range`: Start/end timestamps
- `asr_model_id`: Model used
- `confidence`: Word-level or segment-level confidence

#### 6.2.26.4 Validation Rules

| Validator | Purpose |
|-----------|---------|
| `media_accessible` | Audio/video file is readable |
| `format_supported` | Audio format is supported (via ffmpeg) |
| `duration_within_limits` | Recording length within configured maximum |
| `gpu_available` | GPU resources available (or CPU fallback configured) |

#### 6.2.26.5 Typical Job Flow

```
1. queued          â†’ Media uploaded, format validated
2. running         â†’ ASR runtime processing segments
3. awaiting_validation â†’ Transcription complete
4. completed       â†’ Transcript attached as DerivedContent
```

**Note:** ASR jobs typically do not require `awaiting_user` since transcripts are DerivedContent and non-destructive. Users can edit transcripts post-completion.

---

**Key Takeaways**  
- ASR transcription jobs are AI jobs under the global model ((AI Job Model, Section 2.6.6)).
- The profile adds media references, time ranges, and ASR-specific options.
- Transcripts are attached as DerivedContent with full provenance.
- The workflow integrates with the Model Runtime Layer's ASR service.

---


## 6.3 Mechanical Extension Engines

Version: v1.2 (Tool Bus + conformance; 22 spec-grade engines)

Date: 2025-12-23

Purpose: Defines the normative Mechanical Tool Bus contract and a spec-grade set of 22 mechanical engines that Handshake can invoke under governance. The expanded per-domain engine catalogue below is retained as design/backlog content and is NOT callable unless upgraded to v1.2 templates and passing conformance.

Status: Contract is spec-grade; engine maturity remains mixed. Only engines that pass the v1.2 conformance suite are considered callable/normative.

Normative scope: Â§6.3.0 + Â§11.8.
Informational scope: Â§6.3.1â€“Â§6.3.* engine notes (until upgraded).

---


### 6.3.0 Mechanical Tool Bus Contract (normative; MEX v1.2)

This subsection upgrades Â§6.3 from a descriptive catalogue into an **executable contract** for invoking mechanical engines. The canonical spec-grade contract (including full envelopes, gates, registry requirements, conformance vectors, and the 22-engine set) is imported in **Â§11.8**.

**Terminology and schema discrimination**
- **Engine PlannedOperation (EPO):** a single-engine invocation envelope with `schema_version = "poe-1.0"` (and later `poe-*` variants).  
- **Edit PlannedOperation (COR/BL):** any PlannedOperation used for document/patch semantics elsewhere in this Master Spec.  
- **Rule:** these MUST remain unambiguous via `schema_version` (and, where present, `protocol_id`). Engine invocations MUST use `poe-*`.

**Artifact-first I/O**
- **Size rule:** any payload > **32KB** MUST be passed via input artifacts (handles/refs), never inlined into the PlannedOperation.  
- Outputs MUST be exported as artifacts (immutable) with **SHA-256** hashing + sidecar provenance manifests (see Artifact rules in Â§2.3.10).

**Determinism levels (D0â€“D3)**
- **D3 Bitwise:** identical inputs/config/environment â‡’ identical bytes.  
- **D2 Structural:** identical semantics; bytes may differ; canonicalization required for stable hashes.  
- **D1 Best-effort:** depends on external/labile inputs; replay relies on captured evidence.  
- **D0 Live:** inherently non-replayable unless evidence capture â€œfreezesâ€ the claim.  
- **Evidence rule:** D0/D1 results MUST carry evidence artifacts referenced in EngineResult.

**Required global gates (minimum)**
- `G-SCHEMA` (validate envelopes)  
- `G-CAP` (capabilities/consent; default-deny)  
- `G-INTEGRITY` (artifact hash verification, path safety, no-bypass invariants)  
- `G-BUDGET` (time/memory/output caps; kill/timeout policy)  
- `G-PROVENANCE` (required provenance fields present; artifacts referenced, not inlined)  
- `G-DET` (determinism/evidence policy enforcement)

Gate outcomes MUST be logged to Flight Recorder and surfaced in Problems when denied or degraded.

**Registry + adapter resolution**
- Engines MUST be declared in `mechanical_engines.json` (engine_id, ops, determinism ceiling, required gates, default capabilities, conformance vectors, implementation/adapters).  
- **No-bypass:** engines MUST NOT be invokable outside the orchestrator/runtime.

**Conformance harness (minimum)**
An engine is â€œcallableâ€ only after it passes: schema validation, capability denial tests, budget enforcement, integrity checks, artifact-only I/O, provenance completeness, and determinism/evidence rules for its determinism class.

**Capability model alignment**
v1.2 capability strings (file/process/network/device/secrets/GPU) are enforced through the global capability/consent system in **Â§11.1**. Device/network/secrets MUST remain deny-by-default and require explicit consent records.

---

### 6.3.1 Architectural Invariant

The **Four-Layer Architecture** (Â§1.3) remains the law:

1. **The Brain (LLM):** Plans the action and emits a PlannedOperation (JSON).  
2. **The Gate:** Validates the operation safety.  
3. **The Body (Mechanical):** Executes the operation deterministically using the open-source engines listed below.  
4. **The Shadow Workspace:** Indexes the resulting artifacts.

In Mechanical Engine terms:

- The LLM is **never** allowed to directly modify Raw content.  
- The LLM **only** emits a structured plan that references:
  - Existing workspace objects by ID (document IDs, sheet IDs, node IDs, file IDs).  
  - Allowed operations from a restricted vocabulary.  
- The Gate:
  - Verifies that IDs are valid and belong to the requesting user.  
  - Checks that the operation is allowed by the userâ€™s capability profile.  
  - Applies domain-specific safety checks (e.g., unit safety, bounds checking, path safety).  
- The Mechanical engine executes operations using:
  - Deterministic libraries and runtimes.  
  - Versioned configuration and function libraries.  
- The Shadow Workspace:
  - Records every operation and its outcome.  
  - Exposes provenance (who/what/when/why).  
  - Indexes derived artifacts for later retrieval and RAG.

> Normative rule: In all domains below, LLMs are planners only. Physical or irreversible operations MUST go through a mechanical engine with explicit safety checks.

---

### 6.3.2 Domain 1: Engineering & Manufacturing

#### 6.3.2.1 The "Spatial" Engine (Parametric CAD)

* **Why (LLM Weakness):** LLMs are bad at exact geometries, tolerances, and solid modeling.  
* **What (Mechanical Solution):** Parametric CAD kernels and scriptable CAD DSLs.  
* **3 Use Cases:**  
  * **Custom Enclosures:** "Design a 3D printable case for this PCB with 2mm walls."  
  * **Part Fitting:** "Check if this peg (File A) fits into this hole (File B) with 0.1mm tolerance."  
  * **Procedural Architecture:** "Generate a city block layout with these zoning constraints."  
* **Open Source Software:**  
  * **CadQuery** (Python) or **OpenSCAD**.  
  * **Open CASCADE (OCCT)** for kernel operations.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `cad_generation`.  
  * **Operation:** PlannedOperation contains the Python/CadQuery script.  
  * **Output:** Saves `.step` or `.stl` file as **DerivedContent** (Â§2.2.2.2). Renders preview in Canvas via WebGL.

#### 6.3.2.2 The "Machinist" Engine (CAM/G-Code)

* **Why (LLM Weakness):** LLMs cannot calculate toolpaths or understand CNC physics (feed rates/spindle speeds).  
* **What (Mechanical Solution):** CAM libraries that generate toolpaths from 3D geometry.  
* **3 Use Cases:**  
  * **CNC Routing:** "Generate G-code for this plate with these holes."  
  * **Laser Cutting:** "Nest these parts on a 300x200mm sheet."  
  * **3D Printing:** "Slice this STL for a 0.4mm nozzle at 0.2mm layer height."  
* **Open Source Software:**  
  * **OpenCAMLib** (toolpath generation).  
  * **Slic3r**/**PrusaSlicer** (for 3D printing).  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `cam_generation`.  
  * **Input:** References to CAD-derived files (STEP/STL).  
  * **Output:** G-code or equivalent machine format stored as DerivedContent.  
  * **Gate:** Enforces machine bounds and safety limits (travel, feed, RPM) from a config profile.

#### 6.3.2.3 The "Physics" Engine (Dimensional Analysis)

* **Why (LLM Weakness):** LLMs fail at unit consistency (e.g., adding meters to seconds).  
* **What (Mechanical Solution):** Unit safety libraries.  
* **3 Use Cases:**  
  * **Engineering Sheets:** "Calculate stress (Force/Area) and output in Pascals."  
  * **Safety:** "Flag any formula adding disparate units."  
  * **Conversion:** "Convert all Imperial measurements in this doc to Metric."  
* **Open Source Software:**  
  * **Pint** (Python) or similar unit libraries.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `dimension_check`.  
  * **Operation:** PlannedOperation references:
    * Input expressions with units.  
    * Desired output units.  
  * **Output:** Normalized numeric values + unit-correctness diagnostics.
  * **Integration:** Plugs into Spreadsheet Engine (see Â§6.3.3) as an extra validator.

#### 6.3.2.4 The "Simulation" Engine (FEA/CFD)

* **Why (LLM Weakness):** LLMs cannot solve PDEs or run real numerical solvers.  
* **What (Mechanical Solution):** Finite element / CFD solvers.  
* **3 Use Cases:**  
  * **Stress Analysis:** "Will this bracket survive a 500N load?"  
  * **Thermal:** "Will this heatsink keep the CPU under 80Â°C?"  
  * **Fluid:** "Estimate pressure drop in this pipe."  
* **Open Source Software:**  
  * **Elmer FEM**, **Code\_Aster**, **OpenFOAM**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `simulation_run`.  
  * **Input:** Geometry + boundary conditions + material properties.  
  * **Output:** Simulation results exported as derived datasets and visualizations.  
  * **Integration:** Results indexed in the Shadow Workspace and can be visualized in Canvas.

#### 6.3.2.5 The "Hardware" Engine (Real-World I/O)

* **Why (LLM Weakness):** LLMs cannot read sensors or actuate hardware.  
* **What (Mechanical Solution):** Deterministic I/O and control layers.  
* **3 Use Cases:**  
  * **Data Logging:** "Record temperature from this sensor every minute."  
  * **Bench Automation:** "Sweep voltage from 1V to 5V and log current."  
  * **Home Lab:** "Toggle this relay for 5 seconds."  
* **Open Source Software:**  
  * **Firmata**, **PyVISA**, or serial libraries.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `hardware_control`.  
  * **Gate:** Enforces per-device whitelists (no arbitrary code execution).  
  * **Output:** Time-series logs stored as DerivedContent, indexed as metrics.

---

### 6.3.3 Domain 2: Creative Studio

#### 6.3.3.1 The "Director" Engine (Video/Animation)

* **Why (LLM Weakness):** LLMs cannot render timelines, keyframes, or video encodes.  
* **What (Mechanical Solution):** Video/animation pipelines controlled by structured scripts.  
* **3 Use Cases:**  
  * **B-Roll:** "Cut a 10s highlight reel from this stream with transitions."  
  * **Storyboards:** "Compile these stills into a 24fps animatic."  
  * **Format Conversion:** "Convert this ProRes clip into HEVC with a hard bitrate limit."  
* **Open Source Software:**  
  * **FFmpeg**, **Blender** (for 3D scenes).  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `video_pipeline`.  
  * **Operation:** JSON script describing cuts, transitions, overlays.  
  * **Output:** Encoded video file as DerivedContent.

#### 6.3.3.2 The "Composer" Engine (Music/Audio)

* **Why (LLM Weakness):** LLMs cannot guarantee time-accurate, phase-coherent audio.  
* **What (Mechanical Solution):** DAWs, MIDI engines, and offline renderers.  
* **3 Use Cases:**  
  * **Backing Tracks:** "Render a 60 BPM click track with 4/4 signature."  
  * **Stem Mixes:** "Balance these stems to âˆ’14 LUFS."  
  * **Sound Design:** "Generate a 1-second laser sound effect."  
* **Open Source Software:**  
  * **LMMS**, **Ardour**, **SuperCollider**, **SoX**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `audio_render`.  
  * **Operation:** Script/MIDI describing notes, effects, arrangement.  
  * **Output:** `.wav`/`.flac` as DerivedContent.

#### 6.3.3.3 The "Artist" Engine (Visual Art)

* **Why (LLM Weakness):** LLMs cannot paint pixels or control diffusion steps.  
* **What (Mechanical Solution):** Image engines (raster + vector) controlled via structured prompts.  
* **3 Use Cases:**  
  * **Concept Art:** "Generate a 1024Ã—1024 rough concept of a robot."  
  * **Spritesheets:** "Render a 4x4 walk cycle from this base design."  
  * **Vector Icons:** "Generate an SVG icon set following this style."  
* **Open Source Software:**  
  * **Krita**, **GIMP**, vector libraries, plus local diffusion models.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `image_generation`.  
  * **Output:** Raster/Vector assets saved in the workspace.  
  * **Integration:** Tagged for re-use in docs, canvas, and storyboards.

#### 6.3.3.4 The "Publisher" Engine (Typography/Layout)

* **Why (LLM Weakness):** LLMs cannot guarantee typographic consistency or print layout.  
* **What (Mechanical Solution):** Typesetting engines and layout tools.  
* **3 Use Cases:**  
  * **Books/Zines:** "Lay out this manuscript as a paperback."  
  * **Posters:** "Generate a printable A3 poster with safe margins."  
  * **Docs-to-PDF:** "Export this doc to a print-ready PDF with proper TOC."  
* **Open Source Software:**  
  * **LaTeX**, **Typst**, **WeasyPrint**, **Pandoc**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `doc_layout`.  
  * **Input:** Document structure (from Â§6.1/Â§3).  
  * **Output:** Typeset PDF/EPUB as DerivedContent.


#### 6.3.3.5 The "Atelier" Engine (Creative Direction & Production Planning)

##### 6.3.3.5.1 Overview

**The Atelier: A Framework for Creative Image Generation**

**Introduction:** This document outlines a framework for a next-generation creative partner. It is designed to translate complex artistic, narrative, and commercial concepts into visually compelling images. It models a complete digital production studio with two primary operating modes: a **Representational Mode** for building scenes and a **Conceptual Mode** for interpreting ideas. At its heart is a powerful **Extraction Engine** that provides a deep vocabulary of descriptors for all specialist roles.

**Core Principle: Inter-Departmental Collaboration.** The departments below are not silos. They can be commissioned to provide services and assets to one another in a "Nested Production" model. For example, the UI/UX Department can design the interface seen on a phone in a character portrait, and the Graphic Design specialist can create the fabric patterns used by the Fashion Stylist.

##### 6.3.3.5.2 Creative Modes & Master Controls

This is the highest-level choice, defining the overall goal of the project.

###### 6.3.3.5.2.1 Representational Mode: Atelier

This is the default mode for creating representational images. It uses the full production team to build a scene, whether it's a narrative, a portrait, or a product shot. The workflow is detailed in Â§6.3.3.5.3.

###### 6.3.3.5.2.2 Conceptual Mode: The Creative Core

When the goal is not to depict a scene but to interpret an idea (e.g., abstract art, satire), this mode is activated. It uses a set of fundamental **Artistic Vectors** to create a visual strategy from a core `Intent`.

*   **The Artistic Vectors (Sliders):**
    *   **Abstraction:** `100% Representational <--> 100% Abstract`
    *   **Clarity:** `Didactic (Clear Message) <--> Ambiguous (Open to Interpretation)`
    *   **Tone:** `Sincere / Earnest <--> Ironic / Satirical`
    *   **Harmony:** `Harmonious / Serene <--> Dissonant / Tense`
    *   **Complexity:** `Minimalist <--> Maximalist / Baroque`
    *   **Familiarity:** `Familiar / Grounded <--> Uncanny / Dreamlike`

The "recipe" created by these vectors is then passed to the Production Team (Â§6.3.3.5.3) for execution.

####### 6.3.3.5.2.2.1 `ConceptRecipe` (Typed, Replayable)

The Artistic Vectors UI MUST materialize a typed `ConceptRecipe` artifact (Derived) rather than keeping the "recipe" implicit in UI state.

- **Artifact kind:** `ATELIER_CONCEPT_RECIPE`
- **Schema version:** `concept_recipe.v1`
- **Fields (normative):**
  - `recipe_id` (stable id)
  - `intent_text` (string; raw intent, uncensored)
  - `vectors` (object of floats in `[0.0, 1.0]`):
    - `abstraction`, `clarity`, `tone`, `harmony`, `complexity`, `familiarity`
  - `studio_philosophy` (enum; see Â§6.3.3.5.2.3)
  - `constraints[]` (typed constraints; role-agnostic):
    - each constraint is `{kind, path, op, value, rationale}`
  - `seed_policy` (object):
    - `mode` âˆˆ `{fixed_seed, deterministic_approx}`
    - `seed` (int; required when `mode=fixed_seed`)
  - `pins` (required):
    - `role_registry_version`
    - `vocab_snapshot_ids[]`
    - `model_ids[]` (if any models were consulted to derive constraints)
    - `tool_versions[]` (UI + compiler versions)
- **Replay rule:** given identical `ConceptRecipe` + identical role contracts/pins, downstream role composition MUST be replayable.
- **Validator hook:** `ATELIER-LENS-VAL-009 Recipe validity (FAIL)` applies to every materialized `ConceptRecipe` used by Atelier Lens (Â§6.3.3.5.7).

###### 6.3.3.5.2.3 Studio Philosophy (Master Control)

This defines the team's collaborative dynamic and applies to both modes.
*   **The Auteur:** User's vision is absolute.
*   **The Hollywood Blockbuster:** Prioritizes spectacle and impact.
*   **The Surrealist Collective:** High AI autonomy, values experimentation.
*   **The Dogme 95:** Operates under strict, user-defined constraints.
*   **The Documentary:** Prioritizes realism and authenticity.

##### 6.3.3.5.3 Production Workflow & Departments

The creative process, especially in Representational Mode, is organized into three phases.

###### 6.3.3.5.3.1 Phase I: Pre-Production (Concept & Vision)

*   **The Executive Department:** Producer, Director (Keeper of Intent).
*   **The Thematic & Psychological Department:** Writer, Psychological Impact Consultant, Symbolism & Mythology Consultant, Mood Architect.
*   **The Context & Culture Department:** This department provides deep historical and cultural context, operating under the **Principle of Cultural Authenticity.**
    *   **Principle of Cultural Authenticity:** This department prioritizes authentic, respectful representation of global cultures, actively avoiding monolithic or stereotypical interpretations. It operates on an **"Advisor, not a Gatekeeper"** model.
    *   **"Advisor, not a Gatekeeper" Workflow:**
        1.  **Advise & Inform:** When a request diverges from cultural or historical accuracy (e.g., a "samurai babe"), the system provides context, explains the authentic history (e.g., the 'Onna-musha'), and offers a clear choice between historical accuracy and stylized fantasy.
        2.  **Execute User's Choice:** The system fully respects the user's final decision. If stylized fantasy is chosen, the system proceeds without judgment. However, it will still provide **informed stylization**â€”drawing from its deep knowledge to ensure the fantasy is coherent and avoids jarring, unintentional errors (e.g., ensuring a fantasy samurai still wields a `katana` and wears armor *derived from* Japanese designs, rather than using a European longsword, unless a specific genre blend is the user's explicit intent).
    *   **Historian / World-Builder:** Master of time and place. An expert in real-world art history, regional history (e.g., "Ukiyo-e period Japanese art"), or can be "fed" the lore of a fictional universe.
    *   **Cultural Anthropologist / Trend Forecaster:** Understands cultural movements, subcultures (e.g., "nomadic tribes of a specific region"), and rapidly evolving trends. Its expertise is deeply granular, understanding regional specificity and contextual significance (e.g., the distinct textiles of `Yoruba` vs. `Maasai` cultures).
    *   **Technology Specialist:** Ensures all depicted technology is period-appropriate.

###### 6.3.3.5.3.2 Phase II: Production (Execution & Creation)

*   **The World-Building Department:**
    *   **Production Designer:** Oversees the entire environment.
    *   **Set Dresser:** Populates the scene with objects and props. Has access to sub-specialists like the **Armorer / Weapons Master** for action scenes.
    *   **Materials Specialist:** Defines the texture and substance of all surfaces.
*   **The Visuals Department (Camera Crew):** Director of Photography (DOP), Cinematographer, Gaffer.
*   **The Fashion & Styling Department:** This department has a globalized and deep understanding of apparel and personal presentation.
    *   **Fashion Stylist:** Expert in fashion history, designers, and concepts (Haute Couture, Streetwear, etc.). Its expertise includes specific historical and stylistic eras (e.g., `1920s Flapper`, `1960s Mod`, `1990s Grunge`), a deep knowledge of specific `fabrics` (e.g., `silk`, `chantilly lace`, `neoprene`) and their properties, and a comprehensive understanding of `patterns` (e.g., `Plaid`, `Paisley`, `Herringbone`). It also has a specialized knowledge of `Lingerie & Boudoir` styling, including historical context and garment vocabulary (`corsetry`, `teddies`, `babydolls`). Its garment vocabulary is explicitly globalized, including `sari`, `hanbok`, `kimono`, `dashiki`, `qipao`, and `caftan`.
    *   **Hair Stylist:** Creates specific and conceptual hairstyles.
    *   **Makeup Artist (MUA):** Specialist in makeup styles, from naturalistic to editorial and avant-garde. Can create looks specific to boudoir and high-fashion lingerie shoots (e.g., `smoky eyes`, `tousled "bedroom hair"`).
    *   **Model / Talent:** Defines the subject's performance, pose, and gaze.

###### 6.3.3.5.3.3 Phase III: Post-Production (Refinement & Polish)

*   **The Finishing Department:** Editor, VFX Team, Color Grading Team, Digital Imaging Technician (DIT).

##### 6.3.3.5.4 Department Specializations & Modes

The Production Team can operate in specialized modes that re-task all departments for a specific artistic or commercial goal.

###### 6.3.3.5.4.1 Commercial & Product Photography Mode

Focuses on commercial appeal.
*   **Mode-specific specialist role: The Product Stylist:** The art director for objects, including commercial goods like beauty products. Uses techniques like `Clinical/Hero Shot`, `Lifestyle/In-Context`, etc. Includes **Food Stylist** sub-specialty.
*   **Re-tasked Roles:** Gaffer focuses on defining shape/reflections. DOP makes the product the hero. DIT focuses on retouching and color accuracy.

###### 6.3.3.5.4.2 The Art of Intimacy & Sensuality Mode

Focuses on artistic exploration of intimacy and desire.
*   **Mode-specific specialist role: The Intimacy Coordinator:** Guides the talent to express concepts like `vulnerability`, `power dynamics`, and `longing`.
*   **Expanded Roles:** Psychologist explores themes of desire symbolically. Visuals department uses shadow, soft light, and suggestive compositions.

###### 6.3.3.5.4.3 Digital Product & UI/UX Design Mode

Specializes in designing websites and application interfaces.
*   **Mode-specific specialist roles:** `UI/UX Designer` (the lead), `Information Architect` (structure), `Interaction Designer` (animations).
*   **Re-tasked Roles:** For design assets, this mode commissions specialists from the **Graphic Design & Typography Department**. The `DOP` acts as a `Layout Artist` managing grids and visual hierarchy.

###### 6.3.3.5.4.4 Graphic Design & Typography Department

Dedicated to creating all visual design assets and managing typography.
*   **Creative Director / Brand Strategist:** Defines core brand identity and visual strategy.
*   **Typographer:** Master of type selection, pairing, micro-typography (`kerning`, `leading`, `tracking`), hierarchy, and historical context of fonts.
*   **Graphic Designer (Visual & Asset Design):** Creates logos, iconography, illustrations (in various styles), and manages color theory.
*   **Layout Artist (Publication & Grid Design):** Designs layouts for print and digital, applying grid systems, visual hierarchy, and composition principles.

###### 6.3.3.5.4.5 Architectural & Environmental Design Department

Designs all physical structures, interiors, landscapes, and objects.
*   **The Architect:** Designs buildings and structures based on various architectural styles (`Gothic`, `Modernist`, `Brutalist`, etc.) and vernacular traditions.
*   **The Interior Designer:** Designs interior spaces, focusing on styles (`Mid-Century Modern`, `Industrial`), space planning, materials, and finishes.
*   **The Furniture & Object Designer:** Designs bespoke furniture and key objects, drawing on furniture history and industrial design principles.
*   **The Landscape Architect & Garden Designer:** Designs exterior environments, including gardens (e.g., `Japanese Zen`, `French Formal`), parks, and natural terrains.

##### 6.3.3.5.5 Compositional Toolkit (DOP/Cinematographer)

This is the detailed set of skills available to the Visuals Department.
*   **Principle of Layering:** Explicit control over Foreground, Middle Ground, and Background.
*   **Formal Toolkit:** Rule of Thirds, Golden Ratio, Leading Lines, Framing, Balance.
*   **Realism Toolkit:** Intentional "mistakes" like tilted horizons, obscured subjects, motion blur.
*   **Lens & Camera Simulation:** Focal Length, Aperture/Depth of Field, Camera Angle.

##### 6.3.3.5.6 Handshake Integration Hooks

###### 6.3.3.5.6.1 Contract: `DerivedContent: AtelierProductionPlan` (v0.1)

- **Shape:** prose-first `Brief` (fixed headings; `N/A` allowed) + structured `PlanFields` footer for tool consumption.
- **Brief headings (always present):** Intent/Concept; Mood+Theme; Setting/World; Characters/Subjects; Wardrobe+Props; Composition/Camera; Lighting; Color/Finish; References; Constraints/Must-Avoid.
- **Required envelope fields:** `content_profile` (`general|adult`, default `general`; routing/provenance only), `mode` (`representational|conceptual`), `studio_philosophy`, `variants[]` (optional).
- **Initial `PlanFields` overlays shipped first:** `image_generation`, `graphic_design`, `comfy_recipe` (template-based; see Â§6.3.3.5.6.6).
- **Raw-data invariant:** Atelier stores uncensored intent/descriptors in RawContent/DerivedContent; any filtering is limited to Display/Export connectors and MUST NOT write back into stored artifacts.

###### 6.3.3.5.6.2 Job Profiles

- `ATELIER_PLAN`: create/refine `AtelierProductionPlan` from user intent + workspace references; produces patch-sets only (no silent edits).
- `ATELIER_RENDER`: execute via available image engines and/or compile `comfy_recipe` exports; execution MAY be stubbed in Phase 1 (no vertical slice requirement).

###### 6.3.3.5.6.3 Authenticity Advisor (Never Blocking)

- **UI surface:** a single Advisory Panel summary (warn-only).
- **Defaults:** stronger nudges in Representational Mode; lighter nudges in Conceptual Mode.
- **Apply behavior:** suggestions are click-to-apply and apply immediately to the current plan; every applied change is logged as a patch-set.

###### 6.3.3.5.6.4 Consent & Operator Attestation (No Friction)

- **Operator attestation:** within an `adult`-profile workspace, the operator asserts all referenced subjects are consenting adults (18+).
- **No per-operation prompts:** consent/source metadata is auto-stamped from workspace defaults where possible; unknown provenance is tagged (e.g., `third_party_unverified`) without blocking internal creation/extraction.
- **Boundary control:** stricter handling (downgrade/deny) is allowed only at Display/Export connector boundaries; internal storage remains raw.

###### 6.3.3.5.6.5 Validators (Atelier)

- **ATELIER-VAL-001 Brief structure:** all brief headings exist; missing headings are auto-filled as `N/A` (no prompts).
- **ATELIER-VAL-002 Content profile:** `content_profile` is in-enum; `adult` plans require an adult-only consent envelope per Â§2.2.3.3.
- **ATELIER-VAL-003 No write-back censorship:** any connector filtering MUST NOT modify RawContent/DerivedContent fields.
- **ATELIER-VAL-004 Comfy recipe contract:** `comfy_recipe.template_id` must be present or resolved to a deterministic fallback; Atelier MUST NOT attempt to author a runnable Comfy workflow graph.
- **ATELIER-VAL-005 Variants:** each entry in `variants[]` has a stable `variant_id`; resolved fields are deterministic; raw source values are preserved.

###### 6.3.3.5.6.6 Deterministic Compiler: `AtelierCompiler`

- **Type:** mechanical/deterministic job (no LLM required at compile time).
- **Input:** `AtelierProductionPlan` (+ target export).
- **Outputs (exports):** `export:image_prompt_generic`, `export:graphic_design_brief`, `export:comfy_recipe` (template-based).
- **Determinism:** same plan + same template registry version MUST produce the same output bytes.
- **Comfy recipe:** references `template_id` from a template registry; if no confident match, use `generic_fallback` and emit an Advisory Panel note (never block).
- **Provenance:** compiler version + template registry version + input hash + output hash recorded in Flight Recorder/DerivedContent sidecar.

##### 6.3.3.5.7 Atelier Role Dual-Contract Runtime (Extraction + Creative Output)

This subsection formalizes **Atelier-as-runtime**: every Atelier role is an executable lens that can (a) **extract** role-relevant descriptors from any ingested artifact and (b) **produce** role-specific creative deliverables. Roles may claim relevance across domains (e.g., architecture, fashion, interiors, set dressing, adult content, graphic design) and the system MUST support **multi-claim** on a single artifact.

**Non-negotiable invariant (Raw/Derived):** role extraction outputs MUST be stored in Raw/Derived as captured; any redaction, filtering, or policy transformations are allowed ONLY at Display/Export connector boundaries and MUST NOT write back into stored artifacts.

###### 6.3.3.5.7.1 Role Registry + Dual Contracts

**Config entity:** `AtelierRoleSpec` (versioned)

- `role_id` (stable identifier; never reused)
- `department_id` (stable identifier)
- `display_name`
- `modes_supported`: `{representational, conceptual}` (subset)
- `content_profiles_supported`: list of `content_profile` IDs the role supports (workspace-level gating; declared as strings, not a closed enum).
- `claim_features`: list of feature sources the role can use (e.g., `docling.blocks`, `image.frames`, `vlm.caption`, `ocr.text`, `audio.transcript`)
- `extract_contracts[]`: list of `RoleExtractContract` versions
- `produce_contracts[]`: list of `RoleProduceContract` versions
- `allowed_models[]`: local model IDs permitted for this role (vision/text/embeddings)
- `allowed_tools[]`: mechanical tools permitted (validators, mappers, renderers, exporters)
- `vocab_namespace`: default controlled-vocab namespace for this role (may be empty)
- `proposal_policy`: `{disabled, queue_only, auto_accept_with_threshold}` (see Â§6.3.3.5.7.6)

**Dual contract pattern (per role):**

1) **Extraction contract** `ROLE:<role_id>:X:<ver>`
   - Defines what the role extracts and how it is validated.

2) **Creative output contract** `ROLE:<role_id>:C:<ver>`
   - Defines what the role can produce and the typed deliverables it emits.

**Derived entity:** `RoleDescriptorBundle` (versioned; per artifact Ã— role)

- `artifact_id`
- `role_id`
- `contract_id` (`ROLE:<role_id>:X:<ver>`)
- `confidence` (0..1)
- `fields{}` (schema per contract; see â€œRole overlay schemasâ€ below)
- `tags[]` (controlled vocab; optional)
- `open_notes` (optional; non-joinable text; never used as a canonical key)
- `evidence_refs[]` (see Â§6.3.3.5.7.3)
- `provenance` (model/tool versions, config hashes, timestamps, input refs)

**Derived entity:** `RoleDeliverableBundle` (versioned; per plan Ã— role)

- `plan_id` (e.g., `AtelierProductionPlan` ID)
- `role_id`
- `contract_id` (`ROLE:<role_id>:C:<ver>`)
- `deliverables[]` (typed; see Â§6.3.3.5.7.10)
- `inputs[]` (references to `RoleDescriptorBundle`s, workspace refs, user pins)
- `provenance` (compiler versions, template registry versions, hashes)

**Role overlay schemas (growth-safe):**
- `fields{}` MUST be strongly typed and versioned per contract.
- Free-text is allowed only in `open_notes` and never as a join key.
- Cross-role normalization is explicit and opt-in via mapping jobs (never implicit).

###### 6.3.3.5.7.2 Claim Router (Role-First Routing)

**Job:** `ATELIER_CLAIM`

- **Input:** `artifact_id` + available precomputed signals (Docling blocks, thumbnails/frames, OCR snippets, VLM captions if enabled).
- **Output (Derived):**
  - `RoleScore[] = {role_id, score}` for **all** roles in the active `AtelierRoleSpec` registry (dense distribution; used for scheduling and audit).
  - `RoleClaim[] = {role_id, confidence, reasons[], evidence_refs[]}` for roles above `min_confidence` (multi-claim allowed).
  - `RoleGlance[] = {role_id, status, note, evidence_refs[]}` where `status âˆˆ {none, weak, claimed}` (see â€œall-roles glanceâ€ below).
- **Policy:** multi-claim allowed; `top_k` and `min_confidence` are config-driven.
- **Determinism:** pinned `AtelierRoleSpec` registry version + pinned claim config + pinned claim model version (if used) MUST yield identical outputs for identical inputs.

**Default strategy (hybrid, deterministic):**
- Rule-based feature triggers (cheap, explicit) + optional local classifier for disambiguation.
- Claims MUST log which features fired and why (replayable trace).

**All-roles glance (ideal behavior; SHOULD, not MUST):**
- In the ideal path, **every role gets a â€œlookâ€** at every ingested artifact and either:
  - emits a small, evidence-linked observation, or
  - explicitly reports â€œnoneâ€ (no relevant signal).
- This is implemented as `RoleGlance[]`:
  - `status=claimed` for roles that also appear in `RoleClaim[]`,
  - `status=weak` for low-confidence but potentially interesting signals,
  - `status=none` for â€œlooked, no findingsâ€.
- **Config knobs:** `glance.enabled` (default true), `glance.max_ms_per_role`, `glance.store_none` (if false, omit `status=none` records to save space; still keep `RoleScore[]`).

**Scheduling note:**
- `RoleScore[]` + `top_k` determine which roles run deep `ROLE:<id>:X:<ver>` extraction in the same ingest pass; `RoleGlance[]` is always â€œcheap passâ€ and must not require heavy VLM/LLM calls.

###### 6.3.3.5.7.3 Evidence Pointer Standard (Required for all role extraction)

All role extraction MUST emit evidence pointers sufficient to audit and replay extraction.

**Type:** `EvidenceRef`

- `artifact_id`
- `kind`: `{image_bbox, image_mask, page_span, text_span, time_span, table_cell}`
- `locator`: coordinates/offsets (normalized coordinates for images; byte/char spans for text)
- `source_ref`: reference to upstream artifact (e.g., Docling block id, OCR run id, frame id)
- `confidence`
- `notes` (optional)

Validators MUST reject `RoleDescriptorBundle` outputs that omit required evidence for mandatory fields.

###### 6.3.3.5.7.4 Base Ingestion Integration (Docling + Visual Models)

**Ingestion staging:**
- Docling ingestion MAY produce structured blocks (`docling.blocks`) and extracted figures/images.
- A parallel local visual model MAY produce `vlm.caption`/`vlm.tags` signals for `ATELIER_CLAIM` and role extractors.

**Rule:** role extractors MUST prefer deterministic, pinned model signals and MUST store provenance sufficient for replay.

###### 6.3.3.5.7.5 Determinism + Replay Contract (Atelier Lens)

For `ATELIER_CLAIM`, `ATELIER_ROLE_EXTRACT`, and `ATELIER_ROLE_COMPOSE`:

- **Pinned inputs:** artifact bytes hash + upstream parse products hashes.
- **Pinned configs:** role registry version + contract versions + vocab snapshot versions.
- **Pinned models/tools:** exact model IDs and tool versions MUST be recorded.
- **Replay:** rerun with the same pins MUST reproduce identical Derived outputs (byte-identical JSON for bundles).

###### 6.3.3.5.7.6 Organic Growth: Vocabulary + Schema Proposals (Role-Native)

To allow the descriptor database to grow organically without losing queryability:

**Derived entity:** `VocabProposal`

- `namespace` (role-local or shared)
- `term`
- `term_type` (tag, enum member, schema field, mapping rule)
- `examples[]` (evidence refs + artifact IDs)
- `proposer` (role_id + contract_id)
- `support_count`
- `status`: `{queued, accepted, merged, rejected}`
- `decision_provenance` (who/what accepted; rule; timestamp)

**Jobs:**
- `ATELIER_VOCAB_PROPOSE`: append proposals from role extraction runs.
- `ATELIER_VOCAB_RESOLVE`: merge/synonymize/promote proposals into vocab snapshots.

**Rule:** accepted vocab changes are versioned; role extraction outputs MUST reference the vocab snapshot used.

###### 6.3.3.5.7.7 Expansion Patterns (Novel Extensions) â€” Technical Spec

This section enumerates ten expansion patterns that build on the Role Registry + Dual Contracts. Each pattern defines its additional entities and jobs.

1) **Cross-role dependency solver (Production Graph)**
   - **Entity:** `AtelierDeliverableGraph` (nodes = deliverables; edges = dependencies; typed ports).
   - **Job:** `ATELIER_GRAPH_SOLVE` (toposort + constraint checks; produces an execution plan).
   - **Rule:** role produce contracts MUST declare input/output ports so scheduling is deterministic.
   - **Failure mode:** missing dependency â†’ Advisory Panel warning + partial plan (never block by default).

2) **Role claiming as a mixture model (Soft Multi-Claim)**
   - **Entity:** `RoleMixture = {role_id -> weight}` normalized to sum=1.
   - **Job:** `ATELIER_CLAIM_MIXTURE` (optional replacement for `ATELIER_CLAIM`).
   - **Rule:** downstream extractors scale effort by weight (e.g., deep pass only for weights â‰¥ threshold).
   - **Storage:** persist mixture and feature attributions for audit.

3) **Role-specific embeddings + retrieval lanes**
   - **Entity:** `LaneIndex(role_id, index_version)` + `LaneEmbedding(artifact_id, role_id, vector, anchor_text, provenance)`.
   - **Jobs:** `ATELIER_LANE_EMBED` (compute), `ATELIER_LANE_INDEX` (index), `ATELIER_LANE_QUERY`.
   - **Rule:** the same artifact may have different anchors per role (e.g., wardrobe-focused summary vs lighting-focused summary).
   - **Determinism:** embedding model IDs pinned per lane.

4) **Multi-resolution extraction (Fast pass â†’ Deep pass)**
   - **Jobs:** `ATELIER_FAST_CLAIM` (cheap), `ATELIER_DEEP_EXTRACT(role_id)` (expensive).
   - **Budget controls:** per-workspace GPU budget; per-role max deep calls; queue scheduling.
   - **Rule:** deep pass MUST reference the fast pass evidence and may only add fields allowed by the roleâ€™s extract contract.

5) **Descriptor evolution via proposal queues (Structured growth)**
   - **Entity:** `VocabProposal` (see Â§6.3.3.5.7.6) + `SchemaProposal` (field additions with types).
   - **Job:** `ATELIER_SCHEMA_EVOLVE` (generates new contract versions and migration notes).
   - **Rule:** migrations are additive by default; destructive changes require explicit review note and version bump.

6) **Role-to-role critique loops (Bounded, mechanical)**
   - **Entity:** `CritiquePatchSet = {target_role_id, suggested_changes[], evidence_refs[], severity}`.
   - **Job:** `ATELIER_CRITIQUE` (runs permitted critic roles against bundles/deliverables).
   - **Rule:** critiques are **never auto-applied**; they surface as click-to-apply patches (same interaction model as Authenticity Advisor).
   - **Determinism:** critic contracts are versioned; outputs are replayable.

7) **Scene-state as an entity that roles patch**
   - **Entity:** `AtelierSceneState` (typed state: palette, materials, wardrobe, lighting, camera, props, typography, etc.).
   - **Entity:** `RoleStatePatch(role_id, patch_ops[], evidence_refs[])` (JSON Patch-like ops).
   - **Job:** `ATELIER_STATE_MERGE` (applies patches in deterministic order; conflict resolution rules).
   - **Rule:** conflict resolution is explicit (priority table or merge strategy per field); conflicts produce advisory warnings.

8) **Style lineage graph (Forkable provenance)**
   - **Entity:** `StyleLineageGraph` with nodes `{artifact, bundle, deliverable, plan}` and edges `{derived_from, forked_from, replaced_role_output}`.
   - **Job:** `ATELIER_LINEAGE_UPDATE` (append edges on every extraction/compose).
   - **Use:** â€œfork this lookâ€ by swapping one roleâ€™s deliverable bundle and re-solving the production graph.

9) **Deliverable families per role (Typed artifacts, not just prompts)**
   - **Entity:** `RoleDeliverable(kind, payload_ref, schema_version, constraints[], checksums[])`
   - **Kinds:** `{shot_list, lighting_plan, wardrobe_board, prop_list, palette_pack, typography_brief, comfy_recipe_template, unreal_scene_constraints, moodboard_spec, â€¦}`
   - **Job:** `ATELIER_DELIVERABLE_VALIDATE` (per-kind validation).
   - **Rule:** deliverables MUST be referenced as artifacts with hashes (no opaque blobs without provenance).

10) **Cross-domain role reusable primitives (Claim-anywhere design)**
   - **Config:** `RolePrimitiveSpec` (reusable extraction/compose modules shared across roles, e.g., `palette_extractor`, `material_classifier`, `typography_detector`).
   - **Jobs:** `ATELIER_PRIMITIVE_RUN`, `ATELIER_PRIMITIVE_VALIDATE`.
   - **Rule:** primitives are deterministic modules used by multiple roles; role contracts declare which primitives they call.
   - **Result:** the same role can claim and contribute across unrelated domains without bespoke pipelines.

###### 6.3.3.5.7.8 Job Profiles (Required)

- `ATELIER_CLAIM` (or `ATELIER_CLAIM_MIXTURE`)
- `ATELIER_ROLE_EXTRACT` (runs per claimed role; produces `RoleDescriptorBundle`)
- `ATELIER_ROLE_COMPOSE` (runs per requested role; produces `RoleDeliverableBundle`)
- Optional expansion jobs: `ATELIER_GRAPH_SOLVE`, `ATELIER_STATE_MERGE`, `ATELIER_LANE_INDEX`, `ATELIER_VOCAB_PROPOSE`, `ATELIER_VOCAB_RESOLVE`, `ATELIER_CRITIQUE`, `ATELIER_LINEAGE_UPDATE`

###### 6.3.3.5.7.9 Validators (Required)

- **ATELIER-LENS-VAL-001 Evidence (FAIL):** required evidence refs present for mandatory fields.
- **ATELIER-LENS-VAL-002 Contract adherence (FAIL):** bundle fields match contract schema version; unknown fields rejected.
- **ATELIER-LENS-VAL-003 Provenance (FAIL):** model/tool/config pins recorded; artifact hashes present.
- **ATELIER-LENS-VAL-004 No write-back filtering (FAIL):** any Display/Export projection MUST NOT mutate stored Raw/Derived.
- **ATELIER-LENS-VAL-005 Namespace safety (FAIL):** role-local terms MUST stay in role vocab namespace; shared terms must reference a shared vocab snapshot.
- **ATELIER-LENS-VAL-006 Glance coverage (WARN):** if `glance.enabled=true`, missing `RoleScore` entries or missing per-role `RoleGlance` entries are WARN-only (never block ingest).
- **ATELIER-LENS-VAL-007 Merge determinism (FAIL):** if `ATELIER_STATE_MERGE` is invoked, identical inputs + identical `merge_policy_id` + identical pins MUST yield identical `SceneState.resolved_hash`.
- **ATELIER-LENS-VAL-008 Conflict accounting (FAIL):** if merge policy resolves conflicts or conflicts are detected, a `ConflictSet` artifact MUST be emitted and linked from the `SceneState`.
- **ATELIER-LENS-VAL-009 Recipe validity (FAIL):** any `ConceptRecipe` used by Atelier Lens MUST pass range checks (`vectors` in `[0..1]`), required pins present, and seed policy recorded.
- **ATELIER-LENS-VAL-010 DAG validity (FAIL):** if `ATELIER_GRAPH_SOLVE` is invoked, the resulting `AtelierProductionGraph` MUST be acyclic OR explicitly cycle-broken with a recorded rule.
- **ATELIER-LENS-VAL-011 Dependency completeness (FAIL):** every `solve_plan` step MUST declare required inputs, and the execution plan MUST not schedule a step before its declared dependencies are satisfied.

###### 6.3.3.5.7.10 UI/UX Surfaces (Minimum)

- **Role Claims Panel:** shows claimed roles + confidence + evidence highlights.
- **Role Glances Grid (collapsed):** shows every role with `{none|weak|claimed}` status and the roleâ€™s 1-line note (if any); expands into evidence highlights.
- **Role Bundle Viewer:** per-role overlays with evidence hover (bbox/span highlight).
- **Role Lane Search:** â€œsearch as roleâ€ using the roleâ€™s lane index (if enabled).
- **Proposal Queue:** queued vocab/schema proposals with examples; accept/merge/reject.
- **Deliverables Browser:** typed deliverables per role with validation status and lineage links.

###### 6.3.3.5.7.11 Normative language [ADD v02.123]

**Precedence rule (HARD):** If any clause in Â§6.3.3.5.7.1â€“Â§6.3.3.5.7.10 conflicts with any clause in Â§6.3.3.5.7.11â€“Â§6.3.3.5.7.25, the newer clauses (Â§6.3.3.5.7.11+) MUST be treated as authoritative and override the older text. This rule exists to ensure addendum-driven refinement does not silently dilute Atelier/Lens behavior.

**Numbering note:** Within the inserted block, references like â€œ2.1â€ or â€œ6.1.1â€ are *addendum-local* numbering and do not correspond to Master Spec section numbers.


The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted as described in RFC 2119.

---

###### 6.3.3.5.7.12 Operator-locked constraints [ADD v02.123]

These are treated as hard requirements.

1. **Default Lens extraction depth = Tier1**  
   Tier1 is the default ingestion/extraction setting across Handshake.

2. **Global index with filters**  
   Descriptors/facts are global across Handshake (not project-bound) and queries filter.

3. **SYM-001 is first-class**  
   Symbolism/SHOT_DNA is not optional; it is a primary lane of extraction + retrieval.

4. **NSFW is default view**  
   SFW/NSFW does **not** affect ingestion or stored descriptors; it affects only retrieval visibility/ranking and output rendering.

5. **No censorship / no softening**  
   Internal extraction data is always raw and explicit. Any SFW behavior is strictly a projection at view/output boundaries and MUST NOT write back.

---

###### 6.3.3.5.7.13 Definitions (addendum-local) [ADD v02.123]

**Addendum: 2.1 Atelier and Lens**

- **Lens** = extraction + query/control plane (claims, glances, facts, evidence, search, proposal queue)  
- **Atelier** = composition + production plane (creative planning, deliverables, production graphs)

**Opposites rule:** Every role is symmetric: the same role that composes also extracts.

**Addendum: 2.2 ETL (what â€œETLâ€ means here)**

**ETL = Extract â†’ Transform â†’ Load**

- **Extract:** Docling/OCR/ASR/VLM signals + role extractors
- **Transform:** normalize to CONFIG vocab; flatten to Facts; compute embeddings; compute SYM-001 (layer scores / motifs / SHOT_DNA)
- **Load:** write deterministic artifacts + upsert PostgreSQL/EventLedger authority rows + update derived indexes (FTS + vector + graph + role lanes)

**Addendum: 2.3 Two different â€œtiersâ€ (do not confuse)**

Handshake already has `content_tier` (SFW vs adult categories) in descriptor governance.

This addendum introduces **LensExtractionTier** which is about **how deep** the extraction pass is, not about NSFW.

```ts
type LensExtractionTier = 0 | 1 | 2;

/*
Tier0: minimal (ingest + cheap glances; no heavy role extraction)
Tier1: default (claim + top-k role extraction + fact flattening + indexing + optional SYM-001 when eligible)
Tier2: deep (expanded role set + heavy detectors + deeper symbol pass + full lane indexing)
*/
```

**Addendum: 2.4 ViewMode (SFW/NSFW)**

```ts
type ViewMode = "NSFW" | "SFW";

/*
NSFW: raw descriptors and raw rendering
SFW: filtered projection for retrieval + rendering; never modifies stored descriptors
*/
```

---

###### 6.3.3.5.7.14 Core invariants (non-negotiable) [ADD v02.123]

**Addendum: 3.1 No write-back censorship (HARD)**

No part of Atelier/Lens may modify stored descriptors to satisfy display posture.

- Ingestion (Docling, OCR, ASR, VLM) is always raw.
- Role extraction is always raw.
- Facts and Symbol facts are always raw.
- SFW is implemented as a **filtered projection** only.

**Addendum: 3.2 Deterministic replay (HARD)**

For the following jobs, Handshake MUST be able to replay and reproduce the same Derived artifacts:

- `ATELIER_CLAIM`
- `ATELIER_GLANCE`
- `ATELIER_ROLE_EXTRACT`
- `ATELIER_ROLE_COMPOSE` (when run in strict mode)
- `ATELIER_VOCAB_PROPOSE` / `ATELIER_VOCAB_RESOLVE`
- `ATELIER_LANE_INDEX`
- `SYM-001` jobs inside Lens (symbol extraction pass)

Practical note: even if model output is not 100% stable, replay mode MUST support persisting the effective candidate lists / tie-breaks / selected spans / final bundle hashes so a replay uses the persisted order.


**Addendum: 3.3 Lossless role catalog + append-only registry (HARD)**

Handshake MUST NOT â€œloseâ€ roles via refactors, renames, or re-scoping.

- The canonical role catalog (names, intent, department grouping, and role mechanics) MUST be embedded in the master spec after merge (no external sidecar docs).
- Role identifiers (`role_id`) are **stable**. Renames are aliases; the `role_id` does not change.
- The role registry is **append-only**:
  - new roles MAY be added,
  - existing roles MAY be deprecated (explicitly), but MUST NOT be removed.
- Any change to role definitions MUST be versioned (contract id bump) and logged (Flight Recorder + spec-change log).
- Validators MUST fail any build that removes a previously declared `role_id` or silently changes a roleâ€™s contract surface.


---

###### 6.3.3.5.7.15 Atelier/Lens runtime model (tightened) [ADD v02.123]

**Addendum: 4.1 Dual-contract role runtime (recap)**

Each role has:

- **Extraction contract**: `ROLE:<role_id>:X:<ver>` â†’ `RoleDescriptorBundle`
- **Compose contract**: `ROLE:<role_id>:C:<ver>` â†’ `RoleDeliverableBundle`

Roles are the atomic unit. There is no separate â€œlens role vs atelier roleâ€.

**Addendum: 4.2 Claim â†’ Glance â†’ Extract (default Tier1 flow)**

Default Tier1 pipeline for a newly ingested artifact:

1. `ATELIER_CLAIM`  
   Produces RoleScore[] distribution + RoleClaim[] for top roles.

2. `ATELIER_GLANCE`  
   Produces cheap RoleGlance[] for â€œall roles gridâ€ (claimed/weak/none + one-line evidence links).

3. `ATELIER_ROLE_EXTRACT` for **top-k** roles (k configurable; default profile-controlled)  
   Produces RoleDescriptorBundle for each role.

4. Transform + Load  
   Flatten bundles to facts; attach evidence; build/update indexes and lanes.

5. SYM-001 pass (Tier1)  
   Run SYM-001 opportunistically whenever there is any usable descriptor substrate; emit `unclear`/`not_available` for missing fields (see Â§9).

Tier2 expands step (3) and may introduce heavier detectors.

**Addendum: 4.3 Tier1 default selection (HARD)**

`LensExtractionTier` default is Tier1. Tier0 must be explicit operator choice.

**Addendum: 4.4 Tier2 trigger policy (HARD)**

Tier2 extraction MUST be scheduled **automatically when the workspace is idle**.

- â€œIdleâ€ is implementation-defined but MUST at minimum mean: no active operator interaction and no foreground (interactive) jobs running.
- Tier2 jobs MUST yield immediately to any foreground job request.
- Tier2 jobs MAY be additionally gated by an operator profile (e.g., max concurrency / compute budget), but the default posture is auto-when-idle.

**Addendum: 4.5 Role-turn isolation (recommended; determinism support)**

Role extract runs SHOULD be executed as **short, isolated turns**:

- For each role turn, the runtime MUST reset role prompt + scratch context window (no cross-role hidden carryover).
- Cross-role knowledge transfer MUST occur only through persisted artifacts (Claim/Glance/Bundles/Facts/ContextPacks).
- This enables repeated â€œall roles passâ€ loops without unloading the underlying model.


---

###### 6.3.3.5.7.16 Evidence model (click-to-source correctness) [ADD v02.123]

**Addendum: 5.1 EvidenceRef is mandatory**

Every extractor MUST emit bounded EvidenceRefs for evidence-required fields.

Evidence locator types (minimum):

- `doc_span` (doc_id, block_id, char_start/char_end)
- `page_bbox` (doc_id/file_id, page, x/y/w/h)
- `image_bbox` (asset_id, bbox)
- `frame_span` (video_id, t_start/t_end, bbox optional)
- `audio_span` (audio_id, t_start/t_end)
- `table_cell` (doc_id, table_id, row, col)

**Addendum: 5.2 Parallel evidence (Docling + local VLM)**

Lens may use multiple evidence sources for higher accuracy:

- Docling structure/text spans (layout-aware)
- local VLM captions/tags (vision-first)

When multiple sources corroborate a fact, Facts MAY carry multiple evidence refs (or one evidence ref plus a â€œcorroborates[]â€ list).

---

###### 6.3.3.5.7.17 Canonical descriptor substrate (force multiplier) [ADD v02.123]

Role bundles are role-specific and correct. But every non-LLM tool needs a shared query substrate.

**Addendum: 6.1 Rule: bundles MUST emit Facts (HARD)**

Every successful RoleDescriptorBundle write MUST also emit canonical fact rows.

**Addendum: 6.1.1 `AtelierFact` (normalized scalar facts)**

```ts
interface AtelierFact {
  fact_id: string;
  workspace_id: string;
  project_id?: string;

  bundle_id: string;
  role_id: string;
  contract_id: string;

  path: string;              // stable JSONPath-like key into bundle fields
  value_type: "string"|"number"|"bool"|"term"|"json";
  value_norm: string;        // normalized scalar (for SQL/FTS)
  term_id?: string;          // CONFIG term id when controlled

  content_tier: "sfw"|"adult_soft"|"adult_explicit";   // governance
  consent_profile_id?: string;

  confidence: number;        // 0..1
  evidence_id?: string;      // required when evidence-required field
  created_at: string;        // timestamp
}
```

**Addendum: 6.1.2 `AtelierSymbolFact` (SYM-001 facts)**

```ts
interface AtelierSymbolFact {
  sym_fact_id: string;
  workspace_id: string;
  project_id?: string;

  source_bundle_id: string;     // SYM bundle id or role bundle id
  symbol_term_id: string;       // CONFIG/SYM term id
  intensity: number;            // 0..1
  polarity?: "positive"|"negative"|"mixed"|"neutral";

  content_tier: "sfw"|"adult_soft"|"adult_explicit";
  consent_profile_id?: string;

  evidence_id?: string;
  created_at: string;
}
```

**Addendum: 6.2 Global index with filters (HARD)**

Facts and SymbolFacts are global by default. Projects apply filters.

Minimum filter envelope (always applied in Lens):

```ts
interface LensFilterEnvelope {
  workspace_id: string;          // implicit in DB-per-workspace, but explicit in API
  project_id?: string;           // optional project scope
  content_tier_min?: "sfw"|"adult_soft"|"adult_explicit"; // governance gating for retrieval
  consent_profile_id?: string;   // optional
  view_mode: ViewMode;           // NSFW default
  time_range?: { from?: string; to?: string };
  role_ids?: string[];           // restrict to a role lens (search-as-role)
  vocab_namespaces?: string[];   // restrict (DES/TXT/SYM/etc)
}
```


**Addendum: 6.3 Library growth is expected (HARD)**

Handshake is designed for continuous, cross-domain ingestion (e.g., paintings one day, architecture the next, then photography). Therefore:

- The descriptor/fact substrate is a **cumulative library**: new ingestion adds Facts/SymbolFacts; nothing is â€œresetâ€ unless the operator explicitly forks or clears a workspace.
- Growth MUST be safe-by-default:
  - raw extractions are append-only,
  - corrections are expressed as new bundles/facts with higher confidence and clear provenance,
  - old rows are not silently overwritten.
- Tier2 enrichment jobs (role deep passes, lane re-indexing, motif expansion) SHOULD run automatically when the system is idle (per Â§2 decisions) to keep the library usable as it grows.


---

###### 6.3.3.5.7.18 Persistence contract (PostgreSQL/EventLedger authority) [ADD v02.123] [UPDATED v02.191]

**Addendum: 7.1 Deterministic artifact layout (Derived store)**

Store bundles and indexes as deterministic artifacts (hash-addressed):

```
derived/atelier/
  bundles/
    descriptor/<artifact_id>/<role_id>/<contract_id>/<bundle_hash>.json
    deliverable/<plan_id>/<role_id>/<contract_id>/<bundle_hash>.json
  vocab/
    snapshots/<namespace>/<vocab_hash>.json
    proposals/<proposal_id>.json
  symbol/
    snapshots/<sym_namespace>/<sym_hash>.json
    bundles/<artifact_id>/<bundle_hash>.json
  lane/
    <role_id>/index/<index_version>/...
    <role_id>/embeddings/<artifact_id>/<embed_hash>.json
```

**Addendum: 7.2 PostgreSQL/EventLedger authority schema**

The physical schema may evolve, but authoritative rows persist through PostgreSQL/EventLedger and the logical tables and keys are normative. SQLite is not a runtime, fixture, cache, fallback, compatibility, or proof path for this persistence contract.

**Core tables:**

- `atelier_role_spec(role_id, role_version, department_id, display_name, spec_json, spec_hash)`
- `atelier_contract(contract_id, role_id, kind {X|C}, version, schema_json, schema_hash)`
- `atelier_profile(profile_id, kind, source_text, compiled_json, source_hash, compiled_hash, created_at, updated_at)`
- `atelier_bundle(bundle_id, bundle_kind {descriptor|deliverable|symbol}, artifact_id, plan_id, role_id, contract_id, bundle_hash, created_at, provenance_json, status)`
- `atelier_evidence(evidence_id, bundle_id, artifact_id, kind, locator_json, source_ref, confidence, notes)`
- `atelier_fact(fact_id, bundle_id, role_id, contract_id, path, value_type, value_norm, term_id, confidence, evidence_id, content_tier, consent_profile_id, workspace_id, project_id, created_at)`
- `atelier_symbol_fact(sym_fact_id, source_bundle_id, symbol_term_id, intensity, polarity, evidence_id, content_tier, consent_profile_id, workspace_id, project_id, created_at)`
- `atelier_vocab_snapshot(namespace, vocab_hash, created_at, snapshot_json)`
- `atelier_vocab_proposal(proposal_id, namespace, term, term_type, status, support_count, proposer_role_id, examples_json, decision_provenance_json)`
- `atelier_lane_index(role_id, index_version, built_from_vocab_hash, built_from_contract_ids, built_at, index_meta_json)`
- `atelier_lane_embedding(embed_id, artifact_id, role_id, index_version, anchor_text, vector_blob, embed_hash, provenance_json)`

**Authority notes:**
- store vectors through the PostgreSQL/EventLedger authority path, using pgvector or an equivalent Handshake-managed vector storage component when available
- keep JSON payloads but ensure query keys exist as columns
- FTS is a derived index; not relied on for logical correctness

---

###### 6.3.3.5.7.19 Retrieval contract (two methods, deterministic) [ADD v02.123]

**Addendum: 8.1 Lens must expose both retrieval modalities**

Lens retrieval MUST combine:

- **Lexical/keyword search** over Facts + Docling text blocks (FTS/BM25)
- **Vector/semantic search** over embeddings for Facts and/or doc blocks
- optional **graph/meta routing** (knowledge graph neighborhood, lane priors)

**Addendum: 8.2 QueryPlan + RetrievalTrace**

Every search returns:

- `QueryPlan` (routes, weights, filters)
- `RetrievalTrace` (candidate ids + scores + tie-break keys + cache hits/misses)

**Addendum: 8.3 Two-stage retrieval (candidate â†’ rerank)**

Default strategy:

1. Candidate generation (cheap; hybrid fusion):
   - lexical candidates
   - vector candidates
   - lane-scoped candidates (if role lens selected)
2. Rerank (bounded; deterministic):
   - dedupe
   - tie-break by stable keys
   - produce final ranked list

**Addendum: 8.4 Snippet-first reading**

Lens MUST avoid â€œread everythingâ€ behavior:

- Search returns bounded snippets and evidence pointers
- Read returns bounded excerpt (span/bbox/frame range)
- escalation is explicit and logged


**Addendum: 8.5 Lens query API shape (normative)**

Lens is not â€œjust UIâ€; it is a query/control plane that all other subsystems can call deterministically.

```ts
interface LensQueryEnvelope {
  query_id: string;

  // One query may run multiple routes in parallel (lexical + vector + lane + graph).
  query_text: string;

  filter: LensFilterEnvelope;

  // Retrieval routing
  routes: {
    lexical: boolean;
    vector: boolean;
    lane?: { role_id: string };     // â€œsearch as roleâ€
    graph?: boolean;
  };

  // Hybrid fusion weights (defaults from profile)
  weights?: { lexical: number; vector: number; lane: number; graph: number };

  // Token + time budgets
  budget: { max_candidates: number; max_results: number; max_read_ops: number };

  // Determinism
  mode: "strict" | "replay";
}

interface LensResultItem {
  kind: "fact" | "symbol_fact" | "doc_span" | "image_asset" | "bundle";
  id: string;                       // fact_id / sym_fact_id / block_id / asset_id / bundle_id

  title: string;
  snippet: string;                  // bounded; may be SFW-projected depending on filter.view_mode

  // Evidence is always linkable; projection never destroys the underlying evidence pointer.
  evidence?: EvidenceRef[];

  // Ranking diagnostics (Operator Consoles)
  score: number;
  route_scores?: { lexical?: number; vector?: number; lane?: number; graph?: number };

  // Projection markers
  projection_applied: boolean;
  projection_kind?: "SFW";
  projection_ruleset_id?: string;
}
```

**Addendum: 8.6 ContextPacks: LLM-friendly view over Facts (required)**

To keep storage/tooling â€œLLM-friendlyâ€ while remaining deterministic, Lens MUST be able to materialize a bounded, hashed context artifact derived from facts and evidence:

- `AtelierContextPack` (Derived artifact)
  - selected facts (with stable ids)
  - selected symbol facts
  - bounded evidence excerpts
  - constraints/open loops
  - lane snapshots used
  - provenance pins (profile hashes, vocab snapshot hashes, model/tool pins)

`AtelierContextPack` is the preferred input to any writer/creative role when they need corpus context, replacing ad-hoc â€œdump the DB into the prompt.â€


---

###### 6.3.3.5.7.20 SYM-001 in Lens (first-class) [ADD v02.123]

**Addendum: 9.1 SymbolLexiconSnapshot + SymbolismProfile**

SYM-001 uses:

- `SymbolLexiconSnapshot` (global, proposal-grown)
- `SymbolismProfile` (plain-text source + deterministic compilation; may have per-project overlay)

**Addendum: 9.2 SYM job placement**

SYM-001 runs inside Lens as:

- Tier1: **opportunistic** â€” run whenever there is any usable descriptor substrate (Claims/Glances/Facts/Docling spans/VLM tags). There is no hard â€œminimum coverageâ€ gate; missing fields MUST be emitted as `unclear` or `not_available`.
- Tier2: deep profiling (more motifs, broader cross-reference, heavier detectors)

Outputs:
- SHOT_DNA layer scores (as available)
- motif tags
- symbolic intensity facts (`AtelierSymbolFact`)

**Addendum: 9.3 Symbol template growth + unknown fields (HARD)**

SYM output MUST use a **stable, growing template**:

- The template MUST be emitted even when partial; fields that cannot be grounded are set to:
  - `unclear` (a value is conceptually applicable but the system cannot infer it reliably), or
  - `not_available` (the field is not applicable or source signals are missing).
- Re-running SYM on the same artifact MAY refine `unclear` â†’ concrete values as more Facts arrive.
- The current SYM template version used MUST be pinned in provenance (profile hash + template ver + lexicon snapshot hash).


**Addendum: 9.4 Symbolic engine is a living dataset (HARD)**

The symbolic system is not static. Meanings, motifs, and emphasis are expected to drift over time.

Rules:

- `SymbolLexiconSnapshot` is **versioned** and **proposal-grown**. New motifs/terms are added by proposals and become active only once merged into a new snapshot.
- `SymbolismProfile` is **versioned** and MUST support **fork/reset**:
  - a new client/project may use a forked profile,
  - an operator may reset or branch the profile without destroying prior history.
- Re-running SYM against the same artifact MAY legitimately produce different outputs if (and only if) the active profile/snapshot changed; this MUST be visible via provenance pins.
- Past symbol facts are not rewritten in-place. A new SYM run writes new `AtelierSymbolFact` rows with new provenance.

---

###### 6.3.3.5.7.21 Config profiles + UI editor (deterministic) [ADD v02.123]

**Addendum: 10.1 Source-of-truth is plain text (HARD)**

Profiles are plain text blocks (Monaco), compiled deterministically.

**Addendum: 10.2 Recommended editor pattern (simple UI + Monaco)**

- Monaco edits the plain text blocks directly.
- A lightweight form view can help non-technical editing, but must round-trip to the same text.

The UI MUST show:
- source hash
- compiled hash
- compilation diagnostics

**Addendum: 10.3 Required profile types for Atelier/Lens**

- `ATELIER_GLOBAL_PROFILE` (sets defaults; includes Tier1 default)
- `PROJECT_STYLE_HINT` (project overlay; does not fork storage)
- `SYMBOLISM_PROFILE` (SYM-001)
- `VIEW_POLICY` (NSFW default; SFW projection ruleset)


**Addendum: 10.4 Profile evolution, drift, and branching (HARD)**

Profiles are designed to evolve.

- Editing a profile MUST create a new **versioned** record (new hash); systems MUST NOT silently mutate the meaning of an existing pinned hash.
- Profiles MUST support **branching**:
  - `profile_parent_hash` (or parent id) links a child profile to its ancestor,
  - multiple profiles MAY co-exist (e.g., â€œpersonal styleâ€ vs â€œclient Aâ€).
- Projects MUST pin the exact profile hash(es) used at generation/extraction time.
- Operators MUST be able to:
  - fork profile from any prior version,
  - set an â€œactiveâ€ profile per workspace/project,
  - roll back to a previous version.


---

###### 6.3.3.5.7.22 NSFW/SFW policy (raw ingest; filtered view/output only) [ADD v02.123]

**Addendum: 11.1 Ingestion is always raw and uncensored (HARD)**

Docling/VLM/OCR/ASR and role extractors ALWAYS run uncensored and write raw descriptors.

**Addendum: 11.2 SFW affects retrieval + output text only (HARD)**

- retrieval: **strict drop** â€” Lens MUST exclude any candidate/result whose `content_tier` is not `sfw`.
- output: apply projection rules during rendering **only** for remaining SFW-visible items.

**Rule (hard drop):** In `ViewMode="SFW"`, Lens MUST NOT return â€œcollapsed/blurred but revealableâ€ result items.  
Inspection of adult tiers requires switching `view_mode` back to `NSFW` (which does not mutate storage).

Projection is non-destructive and must be labeled when applied.

**Addendum: 11.3 Output labeling (required)**

Any SFW-projected output MUST carry:

- `projection_applied=true`
- `projection_kind="SFW"`
- `projection_ruleset_id`
- link to underlying raw evidence (operator can inspect)

---

###### 6.3.3.5.7.23 Role registry: Digital Production Studio RolePack (draft v1) [ADD v02.123]

Atelier/Lens roles are not ad-hoc prompts; they are versioned RoleSpecs in a RolePack.

**Addendum: 12.1 Departments + roles (inventory)**

This is the initial role inventory (from the Atelier draft). Each role has both X and C contracts.

**Executive Department**
- Producer
- Director (â€œKeeper of Intentâ€)

**Thematic & Psychological Department**
- Writer / Narrative Architect
- Psychological Impact Consultant
- Symbolism & Mythology Consultant (ties to SYM-001)
- Mood Architect

**Context & Culture Department**
- Historian / World-Builder
- Cultural Anthropologist / Trend Forecaster
- Technology Specialist

**World-Building Department**
- Production Designer
- Set Dresser
- Materials Specialist

**Visuals Department**
- Director of Photography
- Cinematographer
- Gaffer (Lighting)
- Camera Technician / Lens Tech (optional split if needed)

**Fashion & Styling Department**
- Fashion Stylist
- Hair Stylist
- Makeup Artist
- Model / Talent

**Finishing Department**
- Editor
- VFX
- Color Grading
- Digital Imaging Technician (DIT)

**Special modes**
- Commercial/Product Photography: Product Stylist, Food Stylist
- Intimacy: Intimacy Coordinator
- Digital Product & UI/UX: UI/UX Designer, Information Architect, Interaction Designer
- Graphic/Typography: Brand Strategist, Typographer, Graphic Designer, Layout Artist
- Architectural/Environmental: Architect, Interior Designer, Furniture/Object Designer, Landscape Architect

**Addendum: 12.2 RoleSpec skeleton (what each role must declare)**

Each role MUST declare at minimum:

- claim features required (`docling.blocks`, `image.frames`, `ocr.text`, `asr.transcript`, `vlm.tags`, etc.)
- extraction schema fields and evidence requirements
- fact mapping rules (bundle â†’ facts)
- compose deliverable kinds (typed outputs)

**Addendum: 12.3 Seniority/experience encoding (recommended)**

To â€œgive roles seniority/experienceâ€ without corrupting determinism:

- encode seniority as **profile-bound role parameters** (plain-text profile â†’ compiled)
- do NOT â€œfreehandâ€ seniority in prompts per run

Example (profile-side):

```
[ROLE:director_of_photography]
experience_level=senior
taste_bias=operator_personal
risk_tolerance=low
```

This ensures replay pins include the same role parameters.

---

###### 6.3.3.5.7.24 Multi-model parallelism integration (Atelier Track) [ADD v02.123]

Lens and Atelier MUST surface per-role/job model assignment and allow SwapRequest/override within allowed models, with provenance logging.

---

###### 6.3.3.5.7.25 Cross-tool deliverables (typed + capability-gated) [ADD v02.123]

RoleDeliverableBundles MUST only emit typed deliverables:

- Monaco patch sets
- Doc patch sets (workspace docs)
- Calendar patch sets
- Word/Doc exports (as jobs producing ExportRecords)
- Toolbus plans (PlannedOperations / MEX envelopes)
- Photo Studio jobs (render/proxy/export)
- Chart/Deck specs referencing tables by ID (no data duplication)


**Addendum: 14.1 DeliverableKind registry (normative)**

RoleDeliverableBundles MUST declare deliverables with a `deliverable_kind` that maps to an existing subsystem artifact type.

Minimum set:

| deliverable_kind | Artifact type | Typical consumer | Capability gate |
|---|---|---|---|
| `monaco_patchset` | `CodePatchSet` | Monaco surface | `fs_write` / repo write |
| `doc_patchset` | `DocPatchSet` | Docs surface | `doc_write` |
| `calendar_patchset` | `CalendarPatchSet` | Calendar subsystem | `calendar_write` |
| `word_doc_draft` | `DocDraft` | Exporter | `export_docx` |
| `toolbus_plan` | `PlannedOperation[]` | MEX/Tool Bus | tool-specific |
| `photo_pipeline_job` | `PhotoJobSpec` | Photo Studio | `photo_write` |
| `chart_spec` | `ChartSpec` (refs `TableId`) | Charts | `chart_write` |
| `deck_spec` | `DeckSpec` (refs entities) | Decks | `deck_write` |

**Rule:** deliverables may be proposed without capability, but application MUST obey the existing patch-set discipline (diff/review/accept) and Flight Recorder logging.

**Addendum: 14.2 Lens UI surfaces (minimum)**

Lens MUST expose, at minimum:

- **Claims Panel**: top roles + scores + thresholds + â€œrun Tier2â€ controls
- **Glances Grid**: all roles quick status (none/weak/claimed) with one-line evidence links
- **Bundle Viewer**: view RoleDescriptorBundle + evidence highlights
- **Fact Explorer**: SQL-like filters over `AtelierFact` + evidence drilldown
- **Symbol Explorer**: `AtelierSymbolFact` + SymbolLexicon browsing
- **Lane Search**: â€œsearch as roleâ€ using role lanes (lexical + vector)
- **Proposal Queue**: vocab + symbol proposals accept/merge/reject
- **Model Status**: HSK_STATUS per role/job + SwapRequest + override audit notes
- **Projection Toggle**: NSFW default; SFW projection clearly labeled

**Addendum: 14.2.1 Atelier Collaboration Panel (selection-scoped) (HARD)**

Atelier MUST support a â€œcollaborate on selectionâ€ workflow in text surfaces (Monaco/Docs):

1. Operator selects a bounded text span.
2. Operator invokes Atelier collaboration (button/shortcut).
3. System shows **all roles** in a side panel; each role may emit **0..n suggestions** (multiple suggestions are preferred when available).
4. Operator checks one or more suggestions and applies them.

Application rules:
- The resulting `monaco_patchset` / `doc_patchset` MUST be **range-bounded** to the selected span.
- Validators MUST reject any patch that modifies text outside the selection range (except for explicitly declared boundary-normalization, if enabled).
- Non-selected text MUST remain byte-identical after patch application.

**Addendum: 14.3 Validators (addendum-required)**

Add the following validators (names are indicative; binding points are normative):

- `ATELIER-LENS-VAL-RAW-001` â€” stored descriptors/facts MUST NOT be euphemised or softened
- `ATELIER-LENS-VAL-VIEW-001` â€” SFW is projection-only; any write-back filtering is rejected
- `ATELIER-LENS-VAL-VIEW-002` â€” in `ViewMode="SFW"`, adult-tier items MUST be excluded from result sets (strict drop)
- `ATELIER-LENS-VAL-TIER-001` â€” default LensExtractionTier is Tier1 (unless explicit override)
- `ATELIER-LENS-VAL-SYM-001` â€” SYM-001 outputs MUST be present when the SYM profile is enabled; missing fields are emitted as `unclear`/`not_available`
- `ATELIER-LENS-VAL-PROFILE-001` â€” profile source hash + compiled hash MUST be pinned in provenance for all role jobs
- `ATELIER-LENS-VAL-SCOPE-001` â€” compose patchsets MUST be selection-bounded; changes outside the operator selection are rejected
- `ATELIER-LENS-VAL-FACT-001` â€” evidence-required fact fields MUST have EvidenceRef
- `ATELIER-LENS-VAL-INDEX-001` â€” lexical/vector indexes must be updated for Tier1 completions (or job is marked degraded)


---



#### 6.3.3.6 The "Darkroom" Engine (Photo Stack)

* **Why (LLM Weakness):** LLMs cannot reliably perform RAW demosaic, color pipeline math, masking, GPU scheduling, or produce repeatable pixel outputs.
* **What (Mechanical Solution):** A deterministic/non-destructive photo + compositing engine set with proxy-first workflows, explicit contracts, and artifact-first outputs.
* **3 Use Cases:**
  * **Develop:** "Apply this recipe to 200 RAW files and export print-ready TIFFs."
  * **Mask & Retouch:** "Generate subject masks, refine edges, and apply local exposure adjustments."
  * **Composite:** "Combine layers with blend modes and export a flattened deliverable with provenance."

* **Open Source Software (typical):** LibRaw / RawSpeed, OpenColorIO, LittleCMS, Lensfun, OpenImageIO/libvips/OpenCV (see Â§11.7.6).
* **Spec Implementation:**
  * **Job Profiles:** `photo_develop`, `photo_mask`, `photo_composite`, `photo_export`
  * **Operation Transport:** MEX v1.2 `PlannedOperations` for each engine adapter (see Â§11.8)
  * **Output:** DerivedContent artifacts + ExportRecord (Â§2.3.10)

##### 6.3.3.6.1 Determinism class mapping (normative)

Photo Stack determinism labels map into MEX determinism classes as follows:

| Photo Stack label | Meaning | Master/MEX class |
|------------------|---------|------------------|
| BITWISE | byte-identical outputs for same inputs + engine versions | D3 |
| FLOAT_INVARIANT | numerically equivalent within tolerance; stable hashes via canonicalization | D2 |
| STRUCTURAL | structure/metadata stable; pixels may vary slightly | D1 |
| BEST_EFFORT | non-deterministic or model-driven | D0 |

##### 6.3.3.6.2 Mechanical Engine Contracts (Photo Stack v0.3.0 snapshot)
##### 6.3.3.6.2.1 `engine.raw_decode`
**Purpose:** Decode RAW files to linear RGB working space.

**Implementation:** Wrapper around LibRaw with RawSpeed fallback.

**Inputs:**
- `source_asset`: RAW file handle
- `decode_options`: Demosaic algorithm, highlight mode, noise threshold

**Outputs:**
- Linear 16-bit or float RGB buffer
- Extracted metadata (EXIF, camera info)
- Color matrix for profile

**Determinism:** BITWISE (given same LibRaw version and options)

##### 6.3.3.6.2.2 `engine.photo_develop`
**Purpose:** Apply EditRecipe to produce rendered output.

**Stages:**
1. RAW decode (if applicable)
2. Lens corrections (Lensfun)
3. Transform/perspective
4. Exposure, WB, basic adjustments
5. Tone curve
6. HSL adjustments
7. Color grading
8. Local adjustments (with masks)
9. Detail (sharpening, NR)
10. Effects (vignette, grain)
11. Color profile conversion
12. Crop

**Determinism:** BITWISE for CPU path; FLOAT_INVARIANT for GPU path

##### 6.3.3.6.2.3 `engine.mask`
**Purpose:** Generate and manipulate masks.

**Operations:**
- `brush_stroke`: Add brush stroke to mask
- `gradient_create`: Create linear/radial gradient mask
- `ai_segment`: Run SAM or semantic model
- `range_mask`: Generate luminance/color range mask
- `compound`: Boolean operations on masks
- `feather`: Apply edge feathering
- `refine_edge`: Edge detection refinement
- `scale_mask`: Scale mask from proxy to full resolution (NEW)

**Determinism:** 
- Manual masks: BITWISE
- AI masks: STOCHASTIC (model version tracked)

##### 6.3.3.6.2.4 `engine.merge`
**Purpose:** Computational photography merges.

**Operations:**
- `hdr_merge`: Multi-exposure HDR fusion
- `panorama_stitch`: Image stitching with projection
- `focus_stack`: Depth-of-field stacking
- `align_stack`: Sub-pixel alignment

**Implementation Strategy:**
- HDR: Custom implementation (Debevec/Robertson) or careful OSS selection
- Panorama: Evaluate GPL alternatives vs custom
- Focus: Laplacian pyramid (well-documented algorithm)

**Determinism:** STRUCTURAL (output hash may vary, but visually identical)

##### 6.3.3.6.2.5 `engine.layer_compositor`
**Purpose:** Render LayerDocument to flat output.

**Features:**
- Bottom-up layer traversal
- Blend mode application (all 27+ modes)
- Mask application per layer
- Adjustment layer evaluation
- Live filter evaluation
- Group handling (passthrough vs normal)
- Blend range (blend-if) evaluation

**Optimization:**
- Tiled rendering for memory efficiency
- GPU acceleration for filters
- Caching of unchanged sub-trees

**Determinism:** BITWISE (CPU) or FLOAT_INVARIANT (GPU)

##### 6.3.3.6.2.6 `engine.ai_enhance`
**Purpose:** ML-based image enhancement.

**Operations:**
- `denoise`: Neural network denoising
- `super_resolution`: 2x upscaling (proxy/web images only)
- `raw_details`: Detail enhancement
- `face_restore`: Face quality enhancement (cropped regions)
- `style_transfer`: Apply artistic style (proxy images)
- `inpaint`: Fill regions (proxy images)

**Scope Limitations:**
- Input images MUST be â‰¤4096px on long edge
- For larger inputs, use proxy or region extraction first

**Implementation:** Wrapper around selected models (NAFNet, Real-ESRGAN, etc.)

**Determinism:** BEST_EFFORT (model version + seed tracked)

##### 6.3.3.6.2.7 `engine.export`
**Purpose:** Produce final deliverable files.

**Responsibilities:**
- Format conversion (JPEG, PNG, TIFF, etc.)
- Color space conversion (sRGB, AdobeRGB, ProPhoto)
- Resize/resample
- Metadata embedding
- Watermarking (optional)
- Policy enforcement (classification, consent)
- Artifact manifest creation

**Determinism:** BITWISE (for lossless) or STRUCTURAL (for lossy)

##### 6.3.3.6.2.8 `engine.vector_render`
**Purpose:** Rasterize vector content for compositing.

**Implementation:** Cairo or Skia-based path rendering.

**Features:**
- Path stroking and filling
- Text rendering (FreeType + HarfBuzz)
- Gradient fills
- Pattern fills
- Effects (shadow, glow via blur)

**Determinism:** BITWISE with font pinning

### 9.9 `engine.proxy` (NEW)
**Purpose:** Generate and manage proxy files for AI processing.

**Operations:**
- `generate`: Create proxy from high-res source
- `sync`: Ensure proxy is current with source edits
- `invalidate`: Mark proxy as stale
- `scale_result`: Map AI results from proxy to full resolution

**Settings:**
```typescript
interface ProxyGenerateOptions {
  source_asset_id: UUID;
  long_edge: number;  // 2048-4096
  format: 'jpeg' | 'webp';
  quality: number;
  apply_current_recipe: boolean;  // Bake in current edits?
}
```

**Determinism:** BITWISE

### 9.10 `engine.vision` (NEW)
**Purpose:** Run vision models for image understanding.

**Operations:**
- `describe`: Generate natural language description
- `tag`: Extract keywords/tags
- `analyze_quality`: Technical quality assessment
- `detect_content`: Subject/scene detection
- `extract_colors`: Dominant color extraction
- `ocr`: Text extraction from images

**Implementation:** Wrapper around MiniCPM-V, Qwen2-VL, or configured model.

**Inputs:**
- Image (proxy recommended for performance)
- Prompt/query (optional)
- Model selection

**Outputs:**
- Structured analysis result
- Confidence scores

**Determinism:** STOCHASTIC (model + temperature tracked)

### 9.11 `engine.llm` (NEW)
**Purpose:** Run local LLMs for text generation and analysis.

**Operations:**
- `generate`: Free-form text generation
- `summarize`: Document summarization
- `extract`: Structured data extraction
- `classify`: Content classification
- `chat`: Multi-turn conversation

**Implementation:** Integration with Handshake ModelRuntime using llama.cpp/Candle/native adapters or explicit compatibility adapters.

**Models:**
- Llama 3.1/3.2 (reasoning, instruction-following)
- Mythomax (creative writing)
- Configurable model selection

**Determinism:** STOCHASTIC (model + seed + temperature tracked)

### 9.12 `engine.comfyui` (NEW)
**Purpose:** Execute ComfyUI workflows for generative AI.

**Scope:** 
- Input images MUST be â‰¤4096px
- Intended for moodboard/web images, NOT high-res camera files

**Operations:**
- `upscale`: Upscale web images (Real-ESRGAN, etc.)
- `style_transfer`: Apply artistic styles
- `generate`: Text-to-image generation
- `inpaint`: Fill/modify regions
- `refactor`: Modify image to match creative direction

**Workflow Execution:**
- Workflows defined as JSON graphs
- Execute through a Handshake-managed ComfyUI-compatible runtime; standalone ComfyUI API is an explicit compatibility adapter only
- Results written to artifact store

**Determinism:** STOCHASTIC (workflow + seed tracked)

---

##### 6.3.3.6.3 GPU Scheduling & Performance (Photo Stack v0.3.0 snapshot)
##### 6.3.3.6.3.1 Priority Queues
```
Priority 0 (Critical): User-blocking operations (tool feedback)
Priority 1 (Interactive): Preview renders, brush strokes, live adjustments
Priority 2 (Background): Full-quality preview generation, mask computation
Priority 3 (Batch): Export jobs, AI enhancement, merge operations
Priority 4 (Idle): Proxy generation, pre-computation, AI tagging
```

##### 6.3.3.6.3.2 Memory Management
- Tile-based processing for images > 64MP
- GPU memory pool with automatic spilling
- Preview pyramid caching with LRU eviction
- Smart preview generation for offline editing
- Proxy cache management with size limits

##### 6.3.3.6.3.3 Threading Model
- UI thread: Never blocked by rendering
- Render thread pool: libvips/GEGL worker threads
- GPU dispatch thread: Manages compute shaders
- I/O thread pool: File loading/saving
- AI inference thread: Model execution (NEW)
- LLM thread: Language model inference (NEW)

##### 6.3.3.6.3.4 AI Model Loading
- Models loaded on-demand
- Model cache with configurable memory limit
- Automatic unloading of unused models
- Preloading of frequently-used models

---

### 6.3.4 Domain 3: Culinary & Home

###### 6.3.3.5.7.11 Merge & Arbitration Contract (Role Overlap)

Atelier is explicitly multi-role and overlap is expected. The system MUST define deterministic merge semantics for role outputs without silently overwriting other roles.

**Derived entities (versioned):**

- `SceneState` (`scene_state.v1`)
  - `scene_id`
  - `inputs[]` (artifact refs used to build the scene)
  - `mode` âˆˆ `{representational, conceptual}`
  - `content_profile` (string)
  - `concept_recipe_ref` (optional; `ATELIER_CONCEPT_RECIPE`)
  - `role_layers{role_id -> role_bundle_ref}` (refs to `RoleDescriptorBundle`)
  - `merge_policy_id` (string)
  - `resolved` (object; computed, not directly edited)
  - `resolved_hash` (sha256 over canonical `resolved`)
  - `conflict_set_ref` (optional)

- `ScenePatchSet` (`scene_patchset.v1`)
  - `patch_id`, `scene_id`, `role_id`
  - `ops[]` (see `PatchOp`)
  - `evidence_refs[]` (required for non-trivial edits)
  - `provenance` (pins; tool/model/config hashes)

- `PatchOp` (`patch_op.v1`)
  - `op` âˆˆ `{add, set, remove, constrain}`
  - `path` (JSONPointer)
  - `value` (JSON)
  - `rationale` (string; uncensored)
  - `priority` (optional int; only used if `merge_policy_id` enables resolution)

- `ConflictSet` (`conflict_set.v1`)
  - `scene_id`
  - `conflicts[]` where each conflict includes:
    - `path`
    - `candidates[]` (op refs + role ids)
    - `resolution` âˆˆ `{unresolved, resolved}`
    - `winner` (optional op ref)
    - `rule` (deterministic rule identifier)

**Merge semantics (normative):**

- Default policy is **overlay-only**: role layers coexist; `resolved` is produced by a deterministic projection that does NOT discard any role layer data.
- If `merge_policy_id` enables resolution:
  - tie-breaking MUST be deterministic and explicit (e.g., fixed priority list or numeric weights),
  - resolution MUST be recorded in `ConflictSet`,
  - no silent overwrites are allowed.

**Job profile (normative):** `ATELIER_STATE_MERGE`

- **Input:** `{role_layers, concept_recipe_ref?, merge_policy_id}`
- **Output:** `SceneState` (+ optional `ConflictSet`)
- **Replay rule:** with pinned versions, re-running merge MUST reproduce `resolved_hash`.
- **Validators:** `ATELIER-LENS-VAL-007` and `ATELIER-LENS-VAL-008` apply.

###### 6.3.3.5.7.12 Nested Production Dependencies (Execution Graph)

Nested production requires explicit dependency semantics so departments can commission one another deterministically (e.g., Graphic Design â†’ Fashion patterns â†’ Wardrobe synthesis). 

**Derived entity:** `AtelierProductionGraph` (`atelier_prod_graph.v1`)

- `graph_id`, `scene_id`
- `nodes[]`: `{node_id, role_id, deliverable_kind, required_inputs[]}`
- `edges[]`: `{from_node_id, to_node_id, kind}` where `kind=depends_on`
- `acyclic` (bool; MUST be true unless cycle-breaking is explicitly recorded)
- `solve_plan[]`: ordered steps `{step_id, node_id, run_profile, pins, budget}`

**Job profile (normative):** `ATELIER_GRAPH_SOLVE`

- **Input:** `SceneState` + requested `deliverable_kinds[]`
- **Output:** `AtelierProductionGraph` + `solve_plan`
- **Cycle rule:** cycles MUST be prevented by design; if cycles appear, the solver MUST emit an explicit cycle-break record (rule id + rationale) and mark `acyclic=false`.
- **Validators:** `ATELIER-LENS-VAL-010` and `ATELIER-LENS-VAL-011` apply.

###### 6.3.3.5.7.13 Post-Production Role Contracts (Concrete Examples)

The Finishing department MUST be representable as dual contracts (Extraction + Creative Output), not only as prose.

**Example role: `finishing.color_grading_team`**

- `ROLE:finishing.color_grading_team:X:v1` (Extraction)
  - **Extracts (evidence-linked):** `palette_targets`, `contrast_intent`, `skin_tone_targets`, `lighting_continuity_issues`, `film_stock_emulation_refs`, `grade_references[]`.
  - **Evidence:** bbox/page/span for references; time spans for video clips.
- `ROLE:finishing.color_grading_team:C:v1` (Creative Output)
  - **Outputs:** `GradeTargetSpec` (curves + lift/gamma/gain intent), `LUTSpec` (if applicable), `ColoristNotes`, `ContinuityFixList`.

**Example role: `finishing.editorial_team`**

- `ROLE:finishing.editorial_team:X:v1`
  - **Extracts:** beat/tempo cues, continuity risks, framing continuity, cut-rhythm references, montage patterns (time-span evidence required).
- `ROLE:finishing.editorial_team:C:v1`
  - **Outputs:** `EditPlan` (beat sheet + cut points), `ContinuityChecklist`, `PickupShotList`.

**Example role: `finishing.vfx_team`**

- `ROLE:finishing.vfx_team:X:v1`
  - **Extracts:** compositing opportunities, tracking markers, lighting match cues, matte boundaries, artifact risks, reference comps.
- `ROLE:finishing.vfx_team:C:v1`
  - **Outputs:** `VFXShotList` (shot id + task + constraints), `CompConstraints`, `AssetRequirements`.

All post-production role bundles MUST follow the Evidence Pointer Standard (Â§6.3.3.5.7.3) and be replayable under pinned versions (Â§6.3.3.5.7.5).

#### 6.3.4.1 The "Sous Chef" Engine (Recipe Logic)

* **Why (LLM Weakness):** LLMs hallucinate quantities, timings, and conversions.  
* **What (Mechanical Solution):** Recipe DSL + deterministic parsing and scaling.  
* **3 Use Cases:**  
  * **Scaling:** "Scale this recipe for 7 people."  
  * **Substitutions:** "Swap dairy for lactose-free alternatives and recalc macros."  
  * **Inventory:** "What can I cook with my current pantry stock?"  
* **Open Source Software:**  
  * **CookLang**, **OpenEats**, or equivalent.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `recipe_logic`.  
  * **Operation:** Uses a structured recipe format with ingredients, units, and steps.  
  * **Output:** Structured recipe stored as DerivedContent.

#### 6.3.4.2 The "Safety" Engine (Food Science)

* **Why (LLM Weakness):** LLMs hallucinate food safety (e.g., "Chicken is safe at 100Â°F").  
* **What (Mechanical Solution):** Deterministic math/databases.  
* **3 Use Cases:**  
  * **Pasteurization:** "Calculate sous-vide time for 25mm chicken at 60Â°C."  
  * **Nutrition:** "Get accurate macros for this specific barcode."  
  * **Fermentation:** "Monitor temp logs for stalled yeast."  
* **Open Source Software:**  
  * Food safety datasets and calculators; potentially custom libraries.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `food_safety`.  
  * **Input:** Time/temperature logs, ingredient lists.  
  * **Output:** Safety verdicts and warnings.

#### 6.3.4.3 The "Homestead" Engine (Home Logistics)

* **Why (LLM Weakness):** LLMs are bad at tracking inventories, expiry dates, and home maintenance schedules reliably.  
* **What (Mechanical Solution):** Deterministic inventory and reminder systems.  
* **3 Use Cases:**  
  * **Pantry:** "Remind me before ingredients expire."  
  * **Maintenance:** "Track filter changes and maintenance tasks."  
  * **Energy:** "Log and analyze energy usage."  
* **Open Source Software:**  
  * **Home Assistant**, inventory tracking tools.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `home_logistics`.  
  * **Integration:** Hooks into time-series, calendar, and notification subsystems.

---

### 6.3.5 Domain 4: Organization & Knowledge

#### 6.3.5.1 The "Archivist" Engine (Preservation)

* **Why (LLM Weakness):** LLMs cannot guarantee long-term integrity of data.  
* **What (Mechanical Solution):** Append-only logs and archival formats.  
* **3 Use Cases:**  
  * **Snapshots:** "Freeze this project state for future reference."  
  * **WORM Storage:** "Store this legal document in an immutable archive."  
  * **Long-term Backups:** "Plan and verify backup rotations."  
* **Open Source Software:**  
  * **BorgBackup**, **restic**, or similar.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `archive_management`.  
  * **Output:** Snapshot metadata stored in the workspace.

#### 6.3.5.2 The "Librarian" Engine (Taxonomy)

* **Why (LLM Weakness):** LLMs do not enforce consistent tagging or hierarchies.  
* **What (Mechanical Solution):** Controlled vocabularies and schema-based tagging.  
* **3 Use Cases:**  
  * **Tag Governance:** "Ensure all project docs follow this tag schema."  
  * **Collections:** "Group related artifacts into curated collections."  
  * **Navigation:** "Maintain cross-links and indices."  
* **Open Source Software:**  
  * Taxonomy libraries, graph DBs.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `taxonomy_management`.  
  * **Output:** Tag trees and cross-reference structures.

#### 6.3.5.3 The "Curator" Engine (Curation & Playlists)

* **Why (LLM Weakness):** LLMs can propose items but not maintain playlists or collections over time.  
* **What (Mechanical Solution):** Deterministic playlist/collection management.  
* **3 Use Cases:**  
  * **Reading Lists:** "Maintain a queue of articles to read."  
  * **Media Playlists:** "Sync this playlist across devices."  
  * **Project Boards:** "Curate key project artifacts into a board."  
* **Open Source Software:**  
  * Media servers, bookmarking tools.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `curation`.  
  * **Integration:** Works with Shadow Workspace indices.

#### 6.3.5.4 The "Analyst" Engine (Email/Tasks/Time)

* **Why (LLM Weakness):** LLMs cannot reliably track time allocations, email states, and tasks across days/weeks.  
* **What (Mechanical Solution):** Deterministic time tracking and email/task integration.  
* **3 Use Cases:**  
  * **Time:** "How many hours did I spend in VS Code today?"  
  * **Email:** "Summarize unread mail from this sender."  
  * **Tasks:** "Generate a weekly review from completed tasks."  
* **Open Source Software:**  
  * Time trackers, email clients with accessible APIs.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `personal_analytics`.  
  * **Integration:** Hooks into calendar, mail, and task backends.

#### 6.3.5.5 The "Chronicle" Engine (Life Logging)

* **Why (LLM Weakness):** LLMs can summarize but cannot be trusted as the sole record of events.  
* **What (Mechanical Solution):** Structured logging of activities and events.  
* **3 Use Cases:**  
  * **Day Logs:** "Record what I did today in a structured way."  
  * **Mood Tracking:** "Log my mood and correlate with activities."  
  * **Milestones:** "Track important life events."  
* **Open Source Software:**  
  * Journaling tools, time-series DBs.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `life_log`.  
  * **Output:** Structured life-log entries stored in the workspace.

---

### 6.3.6 Domain 5: Data & Infrastructure

#### 6.3.6.1 The "Wrangler" Engine (Data Engineering)

* **Why (LLM Weakness):** LLMs struggle with large tabular data and schema evolution.  
* **What (Mechanical Solution):** Data transformation and quality tools.  
* **3 Use Cases:**  
  * **Ingest:** "Normalize this CSV with messy headers."  
  * **Clean:** "Drop invalid rows and fill missing values."  
  * **Validate:** "Enforce schema and constraints."  
* **Open Source Software:**  
  * **dbt**, **Great Expectations**, **pandas**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `data_wrangling`.  
  * **Integration:** Works closely with Spreadsheet Engine and local warehouse.

#### 6.3.6.2 The "DBA" Engine (Local Warehouse)

* **Why (LLM Weakness):** LLMs cannot execute efficient queries or manage indices.  
* **What (Mechanical Solution):** Local analytical databases.  
* **3 Use Cases:**  
  * **Analytics:** "Run a query over millions of rows."  
  * **Rollups:** "Precompute aggregates for dashboards."  
  * **Exploration:** "Sample data for inspection."  
* **Open Source Software:**  
  * **DuckDB**, **SQLite**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `warehouse_query`.  
  * **Output:** Query results as DerivedContent tables.

#### 6.3.6.3 The "Sync" Engine (Replication/Backup)

* **Why (LLM Weakness):** LLMs cannot manage replication or conflict resolution.  
* **What (Mechanical Solution):** Sync engines with conflict resolution strategies.  
* **3 Use Cases:**  
  * **Replica:** "Keep this folder in sync with my NAS."  
  * **Offline:** "Sync changes when I reconnect."  
  * **Backups:** "Push encrypted snapshots to remote storage."  
* **Open Source Software:**  
  * **Syncthing**, **rclone**, **restic**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `sync_management`.  
  * **Integration:** Coordinates with archivist and storage subsystems.

#### 6.3.6.4 The "Indexer" Engine (Search)

* **Why (LLM Weakness):** LLMs cannot index or rank results deterministically.  
* **What (Mechanical Solution):** Search indices and relevance algorithms.  
* **3 Use Cases:**  
  * **Full-Text:** "Index all docs for search."  
  * **Faceted:** "Filter by tags, dates, and types."  
  * **Hybrid:** "Combine lexical and semantic search."  
* **Open Source Software:**  
  * **Lucene**, **Meilisearch**, **ZincSearch**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `index_build`.  
  * **Output:** Indexes referenced by Shadow Workspace.

#### 6.3.6.5 The "Monitor" Engine (Metrics & Alerts)

* **Why (LLM Weakness):** LLMs cannot track long-term metrics or thresholds reliably.  
* **What (Mechanical Solution):** Time-series databases and alerting systems.  
* **3 Use Cases:**  
  * **System:** "Monitor CPU, RAM, disk, GPU."  
  * **App:** "Track app-specific metrics (queue length, errors)."  
  * **User:** "Track personal metrics (writing time, focus blocks)."  
* **Open Source Software:**  
  * **Prometheus**, **Grafana**, local TSDBs.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `metrics_monitor`.  
  * **Integration:** Feeds into notifications and dashboards.

#### 6.3.6.6 The "Router" Engine (Data Flows)

* **Why (LLM Weakness):** LLMs cannot orchestrate data pipelines with retries and backoff.  
* **What (Mechanical Solution):** Local-first workflow schedulers.  
* **3 Use Cases:**  
  * **Pipelines:** "Run this pipeline daily at 03:00."  
  * **Fan-out:** "Distribute work across multiple workers."  
  * **Recovery:** "Retry failed steps with exponential backoff."  
* **Open Source Software:**  
  * **Apache Airflow**, **Prefect**, or lighter-weight DAG runners.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `data_flow`.  
  * **Integration:** Coordinates mechanical engines and storage.

#### 6.3.6.7 The "Inspector" Engine (Data Auditing)

* **Why (LLM Weakness):** LLMs cannot guarantee invariants over big datasets.  
* **What (Mechanical Solution):** Audit trails and invariants checks.  
* **3 Use Cases:**  
  * **Integrity:** "Check for corruption or unexpected changes."  
  * **Compliance:** "Prove this dataset was unmodified between dates."  
  * **Lineage:** "Trace data from source to output."  
* **Open Source Software:**  
  * Data lineage tools, checksum utilities.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `data_audit`.  
  * **Output:** Audit reports and lineage graphs.

---

### 6.3.7 Domain 6: Travel & Spatial Intelligence

#### 6.3.7.1 The "Navigator" Engine (Routing)

* **Why (LLM Weakness):** LLMs cannot compute optimal routes over real maps.  
* **What (Mechanical Solution):** Routing engines over map data.  
* **3 Use Cases:**  
  * **Walking:** "Can I walk from A to B in 10 mins?"  
  * **Transit:** "What route uses the fewest transfers?"  
  * **Driving:** "Avoid tolls and ferries."  
* **Open Source Software:**  
  * **OSRM**, **GraphHopper**, **OpenTripPlanner**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `route_planning`.  
  * **Input:** Origin/destination, constraints.  
  * **Output:** Turn-by-turn routes and ETA.

#### 6.3.7.2 The "Cartographer" Engine (Maps)

* **Why (LLM Weakness):** LLMs cannot render maps or projections.  
* **What (Mechanical Solution):** Map rendering engines.  
* **3 Use Cases:**  
  * **Static Maps:** "Render a map snapshot for this route."  
  * **Layers:** "Show POIs, heatmaps."  
  * **Overlays:** "Annotate map with custom markers."  
* **Open Source Software:**  
  * **Mapnik**, **TileServer GL**, **OpenMapTiles**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `map_render`.  
  * **Output:** Map tiles/images as DerivedContent.

#### 6.3.7.3 The "Geo" Engine (Spatial Queries)

* **Why (LLM Weakness):** LLMs cannot do spatial joins or coordinate transforms.  
* **What (Mechanical Solution):** GIS libraries.  
* **3 Use Cases:**  
  * **Proximity:** "Find all locations within 5km."  
  * **Overlays:** "Intersect areas with risk zones."  
  * **Projections:** "Convert between coordinate systems."  
* **Open Source Software:**  
  * **GDAL**, **PostGIS**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `spatial_query`.  
  * **Output:** GeoJSON and tables.

---

### 6.3.8 Domain 7: Developer Tools & System Context

#### 6.3.8.1 The "Profiler" Engine (System State)

* **Why (LLM Weakness):** LLMs operate blind to CPU, RAM, disk, GPU usage.  
* **What (Mechanical Solution):** System profiling tools.  
* **3 Use Cases:**  
  * **Diagnostics:** "Why is my machine slow?"  
  * **Capacity:** "Can I train this model on my GPU?"  
  * **Safety:** "Warn me if disk is almost full."  
* **Open Source Software:**  
  * System libraries (e.g., `psutil`).  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `system_profile`.  
  * **Output:** Structured system metrics.

#### 6.3.8.2 The "Workspace" Engine (File/Process Model)

* **Why (LLM Weakness):** LLMs cannot safely enumerate files and processes by themselves.  
* **What (Mechanical Solution):** Controlled file and process graph.  
* **3 Use Cases:**  
  * **Context:** "Show me all files in this project."  
  * **Safety:** "Warn me before deleting."  
  * **Search:** "Find large files."  
* **Open Source Software:**  
  * File system libraries, process introspection.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `workspace_model`.  
  * **Integration:** Feeds the AI Orchestrator with safe, filtered context.

#### 6.3.8.3 The "Clipboard" Engine (Ephemeral Context)

* **Why (LLM Weakness):** LLMs cannot see the clipboard directly.  
* **What (Mechanical Solution):** Controlled clipboard bridge.  
* **3 Use Cases:**  
  * **Context:** "Fix the code I just copied."  
  * **Snippets:** "Save this snippet as a reusable block."  
  * **History:** "Search past clipboard entries."  
* **Open Source Software:**  
  * Clipboard utilities.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `clipboard_bridge`.  
  * **Integration:** Exposes clipboard snapshots to LLMs under explicit user consent.

#### 6.3.8.4 The "Quota" Engine (Resource Limits)

* **Why (LLM Weakness):** LLMs will happily run out of disk/GPU/CPU if not constrained.  
* **What (Mechanical Solution):** Quota and limit management.  
* **3 Use Cases:**  
  * **Prevention:** "Warn me if I donâ€™t have space for this model download."  
  * **Guardrails:** "Stop jobs that exceed resource budgets."  
  * **Reporting:** "Show resource usage over time."  
* **Open Source Software:**  
  * Resource/quota libraries.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `resource_quota`.  
  * **Integration:** Tied to system profiler and storage.

#### 6.3.8.5 The "Guard" Engine (Secrets & Safety)

* **Why (LLM Weakness):** LLMs may expose secrets if not constrained.  
* **What (Mechanical Solution):** Secret scanning and redaction.  
* **3 Use Cases:**  
  * **Secrets:** "Did I accidentally leave an API key in this commit?"  
  * **Policies:** "Block uploads containing secrets."  
  * **Audits:** "Scan workspace for credentials."  
* **Open Source Software:**  
  * Secret scanners (e.g., **truffleHog**, **gitleaks**).  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `secret_scan`.  
  * **Integration:** Runs before external sync or uploads.

---

### 6.3.9 Domain 8: OS Primitives & Desktop Integration

#### 6.3.9.1 The "Window" Engine (UI Automation)

* **Why (LLM Weakness):** LLMs cannot interact with the OS GUI directly.  
* **What (Mechanical Solution):** Controlled UI automation.  
* **3 Use Cases:**  
  * **Automation:** "Click this button in this window."  
  * **Screenshotting:** "Capture a screenshot of this region."  
  * **Testing:** "Verify this UI flow works."  
* **Open Source Software:**  
  * **SikuliX**, **PyAutoGUI**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `ui_automation`.  
  * **Gate:** Strict whitelists + prompts for user approval.

#### 6.3.9.2 The "Shell" Engine (Command Runner)

* **Why (LLM Weakness):** LLMs cannot safely run shell commands.  
* **What (Mechanical Solution):** Controlled, logged command execution.  
* **3 Use Cases:**  
  * **Admin:** "Clean temp folders safely."  
  * **Tasks:** "Run this build script."  
  * **Checks:** "List disk usage."  
* **Open Source Software:**  
  * Shell wrappers and sandboxing tools.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `shell_command`.  
  * **Gate:** Command allowlists, path restrictions.

#### 6.3.9.3 The "Scheduler" Engine (Local Jobs)

* **Why (LLM Weakness):** LLMs cannot schedule or remember to run jobs later.  
* **What (Mechanical Solution):** Local job scheduler.  
* **3 Use Cases:**  
  * **Periodic Tasks:** "Run this backup nightly."  
  * **Deferred Work:** "Transcribe this file when the machine is idle."  
  * **Reminders:** "Remind me to review logs tomorrow."  
* **Open Source Software:**  
  * Cron-like schedulers, queue workers.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `local_schedule`.  
  * **Integration:** Hooks into notification system.

#### 6.3.9.4 The "Notifier" Engine (Desktop Notifications)

* **Why (LLM Weakness):** LLMs cannot raise OS notifications.  
* **What (Mechanical Solution):** Notification bridge.  
* **3 Use Cases:**  
  * **Alerts:** "Notify me if this job fails."  
  * **Reminders:** "Ping me at 18:00 with this note."  
  * **Status:** "Show job completion toasts."  
* **Open Source Software:**  
  * Desktop notification libraries.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `notification`.  
  * **Integration:** Used by other engines to surface events.

---

### 6.3.10 Domain 9: Software Engineering & DevOps

#### 6.3.10.1 The "Repo" Engine (Version Control)

* **Why (LLM Weakness):** LLMs do not understand Git state or history deterministically.  
* **What (Mechanical Solution):** Git clients and libraries.  
* **3 Use Cases:**  
  * **Diffs:** "Show what changed since main."  
  * **Branches:** "List open branches and their status."  
  * **Tags:** "Tag this commit as a release."  
* **Open Source Software:**  
  * **git**, **libgit2**, tooling around them.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `repo_management`.  
  * **Integration:** Tied to Workspace Engine for project roots.

[ADD v02.164] Phase 1 MUST treat repository management as a single declared backend policy surfaced through governed metadata rather than an implementation detail. The default backend remains the `git` command-line client behind a `product_managed_process` boundary until an explicit decision artifact records a different backend. Silent fallback among the `git` command-line client, `go-git`, and `libgit2` is forbidden.

[ADD v02.164] Version-control operations that affect protected branches, required status checks, or merge-queue compatibility MUST expose the selected repository backend, backend version, compatibility state, and decision provenance to Workflow Engine and Dev Command Center projections so operators do not infer repository safety from ad hoc command output alone.

[ADD v02.165] Version-control operations that promote, merge, or mark work as ready for protected-branch integration MUST also expose unresolved-conversation count, required review state, required status-check provenance, merge-queue posture, and last verification timestamp to Workflow Engine and Dev Command Center projections. Protected-branch readiness MAY NOT be inferred from a green diff view or the latest command output alone.

#### 6.3.10.2 The "Build" Engine (Compilation & Packaging)

* **Why (LLM Weakness):** LLMs cannot reliably run builds or ensure reproducibility.  
* **What (Mechanical Solution):** Build systems.  
* **3 Use Cases:**  
  * **Compile:** "Build this project for target X."  
  * **Package:** "Create an installer/bundle."  
  * **CI Mirroring:** "Reproduce CI pipeline locally."  
* **Open Source Software:**  
  * **CMake**, **Ninja**, language-specific build tools.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `build_pipeline`.  
  * **Integration:** Works with Shell and Container engines.

#### 6.3.10.3 The "Test" Engine (Automated Tests)

* **Why (LLM Weakness):** LLMs cannot execute tests or interpret all results reliably.  
* **What (Mechanical Solution):** Test runners.  
* **3 Use Cases:**  
  * **Unit Tests:** "Run tests and summarize failures."  
  * **Integration Tests:** "Run long-running suites."  
  * **Regression:** "Compare new vs old test results."  
* **Open Source Software:**  
  * **pytest**, **JUnit**, etc.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `test_runner`.  
  * **Output:** Test reports stored as DerivedContent.

#### 6.3.10.4 The "Deploy" Engine (Local Deployments)

* **Why (LLM Weakness):** LLMs cannot manage environment configuration consistently.  
* **What (Mechanical Solution):** Deployment scripts and tools.  
* **3 Use Cases:**  
  * **Local Services:** "Start/stop dev services."  
  * **Configs:** "Apply environment-specific configs."  
  * **Rollbacks:** "Revert to previous deployment state."  
* **Open Source Software:**  
  * Handshake-managed environment/deployment adapters; Docker Compose is explicit compatibility only.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `local_deploy`.  
  * **Integration:** Uses Handshake-managed deployment and shell engines; container runners are opt-in adapters.

#### 6.3.10.5 The "Log" Engine (Log Aggregation)

* **Why (LLM Weakness):** LLMs cannot aggregate logs efficiently.  
* **What (Mechanical Solution):** Log collection and indexing.  
* **3 Use Cases:**  
  * **Dev Logs:** "Collect logs from services."  
  * **Search:** "Find errors across components."  
  * **Dashboards:** "Feed logs into dashboards."  
* **Open Source Software:**  
  * **Loki**, **ELK Stack**, local log tools.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `log_aggregation`.  
  * **Integration:** Ties into Monitor and Indexer engines.

#### 6.3.10.6 The "Contract" Engine (API Testing)

* **Why (LLM Weakness):** LLMs hallucinate API interactions.  
* **What (Mechanical Solution):** Contract and schema-based API tests.  
* **3 Use Cases:**  
  * **Schema Validation:** "Verify this API matches its OpenAPI spec."  
  * **Mocking:** "Generate mocks for offline testing."  
  * **Regression:** "Detect contract-breaking changes."  
* **Open Source Software:**  
  * API testing tools, schema validators.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `api_contract`.  
  * **Integration:** Works with Repo and Build engines.

#### 6.3.10.7 The "Formatter" Engine (Lint/Style)

* **Why (LLM Weakness):** LLMs can propose code but not enforce style mechanically.  
* **What (Mechanical Solution):** Linters and formatters.  
* **3 Use Cases:**  
  * **Style Guide:** "Ensure I never use the passive voice."  
  * **Code Style:** "Reformat code to match project style."  
  * **Docs Style:** "Normalize headings and lists."  
* **Open Source Software:**  
  * **black**, **prettier**, **eslint**, text formatters.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `formatting`.  
  * **Integration:** Used by language tools across the workspace.

#### 6.3.10.8 The "Container" Engine (Environment)

* **Why (LLM Weakness):** LLMs cannot reliably reason about environments.  
* **What (Mechanical Solution):** Containers and environment managers.  
* **3 Use Cases:**  
  * **Reproducibility:** "Run this in a pinned environment."  
  * **Isolation:** "Sandbox risky experiments."  
  * **Testing:** "Test in multiple environments."  
* **Open Source Software:**  
  * Handshake-managed environment adapters; Docker/Podman/nix are explicit compatibility adapters only.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `environment`.  
  * **Integration:** Wraps other jobs in controlled Handshake-owned environments; outside runners are not defaults or proof prerequisites.

#### 6.3.10.9 The "Network" Engine (Traffic Analysis)

* **Why (LLM Weakness):** LLMs cannot inspect network traffic deterministically.  
* **What (Mechanical Solution):** Packet and HTTP inspection tools.  
* **3 Use Cases:**  
  * **Debugging:** "Inspect traffic to this service."  
  * **Security:** "Detect suspicious connections."  
  * **Monitoring:** "Aggregate request metrics."  
* **Open Source Software:**  
  * **Wireshark**, **mitmproxy**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `network_analysis`.  
  * **Integration:** Works with Monitor and Log engines.

#### 6.3.10.10 The "Decompiler" Engine (Reverse Engineering)

* **Why (LLM Weakness):** LLMs cannot disassemble or decompile binaries reliably.  
* **What (Mechanical Solution):** Reverse engineering tools.  
* **3 Use Cases:**  
  * **Inspection:** "Explore what this binary does."  
  * **Diffing:** "Compare two binary versions."  
  * **Security:** "Check for suspicious behavior."  
* **Open Source Software:**  
  * **Ghidra**, **Radare2**.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `reverse_engineering`.  
  * **Output:** Structured analysis artifacts.

---

### 6.3.11 Domain 10: Language & Linguistics

This domain underpins TXT-001 (text descriptor extraction, Section 2.4.5):

- Engines like Polyglot, Red Pen, Lexicographer, Morphologist, and Converter supply deterministic signals (lemmas, glossaries, style diagnostics, language ID).
- TXT-001 consumes these as part of its DetectorPass and MappingLayer, reducing token usage and making extraction more deterministic.
- LLMs are reserved for higher-level cues (subtext, power dynamics, narrative distance) and always write into DerivedContent (TextDescriptorRow), never RawContent.

The following engines are examples; implementations may vary as long as they respect the AI Job Model and capability boundaries.
#### 6.3.11.1 The "Polyglot" Engine (Offline Translation)

* **Why (LLM Weakness):** LLMs may rely on remote services; local translation is needed for privacy/offline.  
* **What (Mechanical Solution):** Local translation models and TM systems.  
* **3 Use Cases:**  
  * **Offline Translation:** "Translate this document without leaving the machine."  
  * **Glossaries:** "Use this domain-specific glossary."  
  * **Batch:** "Translate a corpus of files."  
* **Open Source Software:**  
  * **Marian**, **Argos Translate**, local NMT.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `offline_translation`.  
  * **Output:** Translated documents as DerivedContent.

#### 6.3.11.2 The "Red Pen" Engine (Grammar & Style)

* **Why (LLM Weakness):** LLMs are probabilistic and can miss consistent grammar/style enforcement.  
* **What (Mechanical Solution):** Deterministic grammar/style checkers.  
* **3 Use Cases:**  
  * **Proofreading:** "Highlight grammar issues only."  
  * **Style Guide:** "Enforce specific style rules."  
  * **Diagnostics:** "Report style issues by category."  
* **Open Source Software:**  
  * **LanguageTool**, similar tools.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `grammar_style_check`.  
  * **Integration:** Provides diagnostics to the Doc editor without rewriting text.

#### 6.3.11.3 The "Lexicographer" Engine (Dictionary/Thesaurus)

* **Why (LLM Weakness):** LLMs hallucinate definitions and synonyms.  
* **What (Mechanical Solution):** Authoritative dictionaries and thesauri.  
* **3 Use Cases:**  
  * **Definitions:** "Look up the precise definition of this word."  
  * **Synonyms:** "Suggest synonyms from a curated list."  
  * **Terminology:** "Validate domain-specific usage."  
* **Open Source Software:**  
  * Offline dictionary databases.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `dictionary_service`.  
  * **Integration:** Injects definitions into context windows.

#### 6.3.11.4 The "Phonetician" Engine (G2P/IPA)

* **Why (LLM Weakness):** LLMs struggle with accurate phonetic transcriptions.  
* **What (Mechanical Solution):** Grapheme-to-phoneme models and IPA generators.  
* **3 Use Cases:**  
  * **Pronunciation:** "Generate IPA for this word."  
  * **TTS Prep:** "Prepare phonetic input for TTS engines."  
  * **Language Study:** "Compare phonetic forms across dialects."  
* **Open Source Software:**  
  * G2P libraries, phonetic datasets.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `phonetic_transcription`.  
  * **Output:** Phonetic strings aligned to text.

#### 6.3.11.5 The "Aligner" Engine (Parallel Text)

* **Why (LLM Weakness):** LLMs cannot reliably align bilingual corpora.  
* **What (Mechanical Solution):** Sentence and paragraph aligners.  
* **3 Use Cases:**  
  * **Corpus Prep:** "Align source and translation texts."  
  * **QA:** "Detect misaligned segments."  
  * **Training Data:** "Prepare parallel corpora for MT models."  
* **Open Source Software:**  
  * Alignment tools, bilingual corpora.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `text_alignment`.  
  * **Output:** Alignment maps and aligned corpora.

#### 6.3.11.6 The "Detector" Engine (Language ID)

* **Why (LLM Weakness):** LLMs can misidentify languages, especially in mixed text.  
* **What (Mechanical Solution):** Language identification libraries.  
* **3 Use Cases:**  
  * **Routing:** "Choose the right model for this language."  
  * **Filtering:** "Detect unsupported or unexpected languages."  
  * **Analytics:** "Break down corpus by language."  
* **Open Source Software:**  
  * Language ID libraries.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `language_detection`.  
  * **Output:** Language labels per document/segment.

#### 6.3.11.7 The "Anonymizer" Engine (PII Scrubbing)

* **Why (LLM Weakness):** LLMs may leak PII if not constrained.  
* **What (Mechanical Solution):** PII detection and redaction.  
* **3 Use Cases:**  
  * **Pre-processing:** "Redact PII before documents leave the machine."  
  * **Reporting:** "Highlight where PII appears."  
  * **Compliance:** "Ensure exports meet privacy requirements."  
* **Open Source Software:**  
  * PII detection libraries.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `pii_scrub`.  
  * **Integration:** Runs before syncing/sharing content.

#### 6.3.11.8 The "Morphologist" Engine (Stemming/Lemmatization)

* **Why (LLM Weakness):** LLMs are not a replacement for lexical normalization.  
* **What (Mechanical Solution):** Deterministic stemming/lemmatization.  
* **3 Use Cases:**  
  * **Search:** "Improve recall by normalizing word forms."  
  * **Analytics:** "Group terms by lemma."  
  * **Pre-processing:** "Normalize tokens before indexing."  
* **Open Source Software:**  
  * NLP libraries (e.g., **spaCy**, **NLTK**, **Stanza**).  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `morphology`.  
  * **Output:** Normalized tokens and lemmas.

#### 6.3.11.9 The "Converter" Engine (Universal Text)

* **Why (LLM Weakness):** LLMs cannot robustly handle all text encodings and formats.  
* **What (Mechanical Solution):** Text format and encoding converters.  
* **3 Use Cases:**  
  * **Encoding:** "Convert between encodings safely."  
  * **Formats:** "Normalize text from various formats."  
  * **Cleanup:** "Strip control characters and normalize line endings."  
* **Open Source Software:**  
  * Text conversion utilities.  
* **Spec Implementation:**  
  * **Job:** AI Job Profile: `text_conversion`.  
  * **Output:** Clean, normalized text for downstream engines.

#### 6.3.11.10 The "Sentiment" Engine (Vibe Check)

* **Why (LLM Weakness):** LLMs are inconsistent at sentiment scoring across time and datasets.  
* **What (Mechanical Solution):** Fixed, versioned sentiment models and rule-based checkers.  
* **3 Use Cases:**  
  * **Docs:** "Score tone of feedback emails."  
  * **Threads:** "Track sentiment over time in a conversation."  
  * **Dashboards:** "Visualize sentiment trends."  
* **Open Source Software:**


---

## 6.4 Visual Debugger (Normative) [ADD v02.186]

**Why**
Handshake's GUI is Tauri + WebView2 running quiet-mode windows for automated runs; standard browser devtools are unavailable to in-context test harnesses. A visual debugger that talks to WebView2 over Chrome DevTools Protocol (CDP) and exposes the surface as Tauri IPC is the only practical way for an LLM coder / WP_VALIDATOR / INTEGRATION_VALIDATOR to verify GUI behavior. This section [ADD v02.186] makes the CDP+Playwright path normative for every GUI-touching WP per HBR-VIS-001..005 and [GLOBAL-BUILD-014]/[GLOBAL-BUILD-023].

**What**
Defines the Playwright-over-CDP implementation, the CI fallback, the headless screenshot + DOM capture IPC surface, the capture matrix support, and the WP-applicability gate.

---

### 6.4.1 Implementation: Playwright over CDP against WebView2

The visual debugger is Playwright driving Chrome DevTools Protocol against the WebView2 instance Handshake runs. Required environment:

- `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>` -- WebView2 is launched with CDP enabled on a per-run random port.
- `WEBVIEW2_USER_DATA_FOLDER=<per-test-folder>` -- per-test user-data isolation so parallel test runs do not corrupt each other's WebView2 profile.

The visual debugger never modifies Handshake's UI state outside the test scope (CDP commands are issued against the test-only WebView2 instance).

### 6.4.2 CI fallback

`tauri-driver` + WebDriverIO is the CI fallback only -- version-pin pain (tauri-driver lags Tauri releases) is a documented field issue; Playwright over CDP is the preferred local-dev path.

### 6.4.3 Headless screenshot + DOM capture IPC

The visual debugger exposes Tauri IPC `kernel.visual_debug.*`:

- `kernel.visual_debug.screenshot(scope, opts)` -- returns PNG bytes via WebView2 CDP `Page.captureScreenshot`.
- `kernel.visual_debug.dom_snapshot(scope)` -- returns DOM tree JSON.
- `kernel.visual_debug.console_stream(scope)` -- subscribes to renderer console + exception stream via CDP `Runtime.consoleAPICalled` + `Runtime.exceptionThrown`; renderer page errors caught via Playwright `page.on('pageerror')`.

Output is consumable by WP_VALIDATOR + INTEGRATION_VALIDATOR scan jobs.

### 6.4.4 Capture matrix support

The matrix follows Playwright's projects-matrix shape; each test scenario produces a manifest entry:

```json
{
  "scenario_id": "...",
  "route": "/...",
  "viewport": {"w": 1920, "h": 1080, "dpr": 1.0},
  "color_scheme": "dark",
  "locale": "en-US",
  "edge_state_tag": "loading | populated | empty | error",
  "wait_for": "selector | event | timeout",
  "mask_selectors": ["..."],
  "baseline_hash": "<sha256>"
}
```

Baselines are content-addressed under `.GOV/visual_baselines/`. Drift between captured PNG and baseline triggers a typed visual-regression receipt.

### 6.4.5 WP applicability

The visual debugger is **required for any GUI-touching WP** per HBR-VIS-001..005 + [GLOBAL-BUILD-014] / [GLOBAL-BUILD-023]. WP draft hydration flags a WP as GUI-touching when it modifies any file under `src/frontend/` or any Tauri command surface; the matrix-check sub-check (Section 5.6.5) asserts visual-baseline evidence is present.

The operator-facing surface for the visual debugger is the Diagnostics panel (Section 10.12).

**Cross-references:** Section 10.12 Diagnostics panel; Section 6.5 backend inspector plane (peer); Section 6.6 non-hijacking GUI invariants; HBR-VIS-001..005; [GLOBAL-BUILD-014]..[GLOBAL-BUILD-045]; WP-KERNEL-004 refinement acceptance criteria AC-VISUAL-DEBUGGER-IMPL, AC-VISUAL-DEBUGGER-IPC, AC-VISUAL-DEBUGGER-MATRIX.

---

## 6.5 Backend Inspector Plane (Normative) [ADD v02.186]

**Why**
Parallel-model coordination ([GLOBAL-BUILD-033]..[GLOBAL-BUILD-045]) requires a read-only observation surface that multiple LLM coders + validators can poll without contending for the production EventLedger write path AND without risking accidental mutation. The Backend Inspector Plane is a localhost-bound, feature-gated, read-only HTTP+WS plane that exposes EventLedger replay, live event tail, point-in-time snapshots, and an audited mutation lane that only accepts signed `KernelActionCatalogV1` envelopes. This section [ADD v02.186] formalizes the contract.

**What**
Defines the bind/feature-gating rules, the compile-time read-only invariant, the four endpoint families, the mutation-routing rule, and the runtime controls.

---

### 6.5.1 Bind + feature gate

The inspector plane is:

- Bound to `127.0.0.1:<random>` only -- never `0.0.0.0`, never an externally routable interface.
- Random port per launch -- no fixed well-known port (operator + validator discover the port via the Diagnostics panel or a stable IPC `kernel.inspector.port()`).
- Feature-gated behind `cfg(feature = "inspector")` -- the inspector code is **not compiled** into release builds shipped to operators. Dev / validator / CI builds enable the feature explicitly.

### 6.5.2 Compile-time read-only invariant

The `InspectorReadV1` Rust trait has **no `&mut self` methods**. It lives in the `inspector_read` crate, which does NOT depend on the write-side crate. A workspace-level deny-lint blocks any reverse dependency edge (`inspector_read` -> write crate). Mutation cannot leak into the inspector code path through normal Rust trait dispatch -- it would require adding a dependency that the lint rejects at compile time.

### 6.5.3 Endpoint families

Four endpoint families:

- `/inspector/v1/eventledger?since=<ts>&until=<ts>` -- paged replay of EventLedger rows.
- `/inspector/v1/events/subscribe?topics=<csv>` -- WS live tail; **server-push only**, client cannot send action frames. Subscription frames are rejected; the WS is read-only at the protocol level.
- `/inspector/v1/snapshot/{scope}` -- point-in-time projection (e.g., `/inspector/v1/snapshot/session/<id>`).
- `/inspector/v1/replay-drive` -- the **only** mutation endpoint. Accepts ONLY a `KernelActionCatalogV1` action id + a signed `WriteBoxV1` envelope (signature from a per-run shared secret). Any other body returns 403.

### 6.5.4 Mutation routing

Mutations route exclusively through `KernelActionCatalogV1` (from KERNEL-002). The inspector plane never has a parallel mutation path. `replay-drive` is a *re-entry* into the canonical action catalog, not a new mutation surface.

### 6.5.5 Runtime controls

- **Localhost-only bind** -- enforced at socket open.
- **Per-launch random port** -- enforced at startup.
- **Per-run shared-secret header** -- required on every request; the secret is generated at launch, stored in operator-visible state only, and rotated per run.
- **Audit log of every reject** -- every 403 / signature-fail / bad-scheme attempt is logged with `(timestamp, route, peer_addr, reason)` and scanned by WP_VALIDATOR for tamper-evidence.

The inspector plane supports parallel-model coordination per [GLOBAL-BUILD-033]..[GLOBAL-BUILD-045] (multiple LLM coders + validators read the same EventLedger replay without write contention; mutation requests are auditable and signed).

**Cross-references:** KERNEL-001 EventLedger; KERNEL-002 KernelActionCatalogV1 + WriteBoxV1; Section 6.4 visual debugger; Section 10.12 Diagnostics panel; [GLOBAL-BUILD-033]..[GLOBAL-BUILD-045]; WP-KERNEL-004 refinement acceptance criteria AC-INSPECTOR-PLANE-BIND, AC-INSPECTOR-PLANE-READONLY, AC-INSPECTOR-PLANE-MUTATION-ROUTING.

---

## 6.6 Non-Hijacking GUI Interaction Invariants (Normative) [ADD v02.186]

**Why**
[GLOBAL-BUILD-046]..[GLOBAL-BUILD-054] forbid apps from popping foreground windows, stealing keyboard focus, or hijacking input during automated runs. HBR-QUIET-001..004 turn that into testable invariants. This section [ADD v02.186] defines the Tauri / WebView2 configuration, the automation-first design rule, the focus + keyboard-injection audit subsystems, the forbidden-API lint, and the explicit foreground exception.

**What**
Six normative invariants covering window config, screenshot path, automation-first design, focus audit, keyboard-injection negative test, forbidden Win32 APIs, and the controlled foreground exception.

---

### 6.6.1 Tauri quiet-mode config invariants (HARD per HBR-QUIET-001)

For any window opened during automated GUI test runs or governed agent activity, the Tauri window config MUST set:

```rust
WindowBuilder::new(...)
    .visible(false)
    .focus(false)
    .focusable(false)
    .skip_taskbar(true)
    .always_on_bottom(true)
    .decorations(false)
```

This suppresses HWND visibility, initial activation, click-activation, taskbar surface, and Z-order presence. A test run with any of these flags inverted is a HBR-QUIET-001 violation.

### 6.6.2 Screenshots from never-shown windows

Screenshots from a never-shown WebView2 window MUST use `WebView2.CallDevToolsProtocolMethodAsync("Page.captureScreenshot", "{}")` -- this path is HWND-independent and works against off-screen / never-rendered windows. Fallback when CDP is unavailable: `Windows.Graphics.Capture` targeted at the off-screen HWND.

Any code path that calls `BitBlt` / `PrintWindow` on a foreground-required HWND fails the visual debugger's contract test.

### 6.6.3 Automation-first design (HARD per HBR-QUIET-002)

Every user-facing action MUST land in a Rust `#[tauri::command]`. Every UI button is a thin caller of `invoke("cmd_x", args)`; UI components never hold business logic. A **contract test** fails the build on any UI affordance (button, menu item, keyboard shortcut handler) that lacks an `invoke` counterpart.

This makes every user action drivable from automated tests without GUI focus.

### 6.6.4 Focus audit subsystem

A focus audit subsystem uses `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, NULL, callback, 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)` via the `wineventhook` Rust crate. Tokio-channel-based; logs every foreground transition as `(timestamp, hwnd, pid, exe_name)` to a JSON ledger keyed by `run_id`.

**Assertion** (during any harness run): no foreground change resolves to Handshake's pid OR to any process spawned by Handshake (per ProcessOwnershipLedger Section 5.7). A violation fails the run.

### 6.6.5 Keyboard-injection negative test (HBR-QUIET-002)

A `WH_KEYBOARD_LL` low-level hook subscribes to keyboard events, checking `KBDLLHOOKSTRUCT.flags & LLKHF_INJECTED`. The test framework:

- Fires `SendInput` sequences targeting common shortcuts (Alt+Tab, Ctrl+C, Win+number) while a hidden Tauri window is alive.
- Asserts (a) **no `#[tauri::command]` handler fired**, (b) **no state mutation occurred**, (c) the LL-hook saw the injection (proving the test actually fired input -- prevents false-negative passes from broken `SendInput`).

Combined with the focus audit ledger from Section 6.6.4, this asserts zero foreground transitions AND zero accidental command execution from injected input.

### 6.6.6 Forbidden APIs (lint rule)

The following Win32 / Tauri APIs are banned outside an explicit `operator_foreground` module:

- `set_focus`
- `show` (on hidden windows that should remain hidden)
- `unminimize`
- `AllowSetForegroundWindow`
- `AttachThreadInput`
- `SetForegroundWindow` (from non-foreground processes)

`LockSetForegroundWindow` is NOT used at all -- it is process-wide and a crash in Handshake would degrade the operator's whole desktop session.

A workspace clippy lint rule fails the build on any reference to the banned APIs outside `operator_foreground::*`.

### 6.6.7 Foreground exception (HBR-QUIET-004)

If a WP genuinely needs OS-foreground interaction (e.g., a screen-recording WP, an interactive operator-driven UI verification), it MUST:

- Declare the requirement in the packet **before any run** (`packet.requires_foreground = true`).
- Use a bounded controlled test window (timeout + auto-dismiss).
- Surface an operator-visible warning before launch (Tauri notification + Diagnostics-panel banner).

Default behavior is forbidden; the exception is opt-in per packet and audited.

**Cross-references:** HBR-QUIET-001..004; [GLOBAL-BUILD-046]..[GLOBAL-BUILD-054]; CX-503D reclaim hooks; Section 6.4 visual debugger (consumer of the off-screen screenshot path); Section 10.12 Diagnostics panel (foreground warning surface); WP-KERNEL-004 refinement acceptance criteria AC-QUIET-WINDOW-CONFIG, AC-QUIET-FOCUS-AUDIT, AC-QUIET-INJECT-NEG-TEST, AC-QUIET-API-LINT, AC-QUIET-FOREGROUND-EXCEPTION.

---

## 6.7 Swarm-Agent Harness (Normative) [ADD v02.186]

**Why**
KERNEL-004 must prove concurrency invariants -- multiple governed sessions hitting the same Handshake instance, contending for leases, cancelling each other, hitting loop counters -- before any production WP relies on parallel model coordination. The Swarm-Agent Harness is the test harness that spawns N concurrent sessions and asserts the invariants. This section [ADD v02.186] separates it from Section 4.3.9 (which is production runtime path) and pins it to KERNEL-001 + KERNEL-002 primitives.

**What**
Defines the harness primitive, its coordination contract (stable IDs + KernelActionCatalogV1 mutations + ProcessOwnershipLedger tracking), its parameterization, and its scope distinction from Section 4.3.9.

---

### 6.7.1 Primitive

`kernel_swarm_test_harness` is the primitive that spawns N concurrent governed sessions against the same Handshake instance.

### 6.7.2 Coordination contract

Sessions coordinate exclusively through:

- **Stable element IDs** -- DOM / Tauri command identifiers are stable across runs; tests reference them by id, not by position.
- **KernelActionCatalogV1 mutations** -- every mutation is a catalog entry (KERNEL-002); the harness never invokes a raw setter.
- **ProcessOwnershipLedger tracking** -- every spawned process row is attributed back to the originating session id (Section 5.7).

There is **no shared session state** between concurrent sessions in the harness; each session is independent and re-entrant.

### 6.7.3 Parameterization

`N` is parameterized. Perf scaling is validated to **N=8 minimum** per HBR-SWARM-001 / HBR-SWARM-002 / HBR-SWARM-003 / HBR-SWARM-004. Higher N is supported but not required by the v02.186 acceptance criteria.

### 6.7.4 Scope distinction from Section 4.3.9

Section 4.3.9 (Multi-Model Orchestration & Lifecycle Telemetry) is the **production runtime path** for orchestrating multiple models in a real workload. Section 6.7 is the **test harness** for asserting concurrency invariants of the platform itself.

Both can coexist; the harness consumes the production primitives (`KernelActionCatalogV1`, EventLedger) but does not modify them.

### 6.7.5 What the harness surfaces

The harness surfaces:

- **Lock / lease behavior** under contention.
- **Cancellation propagation** when sessions terminate mid-mutation.
- **Loop-counter behavior** (HBR-SWARM-002) when a session loops on a stuck precondition.
- **ProcessOwnershipLedger ledger consistency** when multiple sessions write concurrently (Section 5.7.3 bounded mpsc behavior).

It consumes KERNEL-002 KernelActionCatalogV1 + WriteBoxV1 and KERNEL-001 EventLedger; failures here gate KERNEL-004 acceptance.

**Cross-references:** Section 4.3.9 Multi-Model Orchestration (peer / production); Section 5.7 ProcessOwnershipLedger; Section 6.5 backend inspector plane (test harness consumer); HBR-SWARM-001..004; KERNEL-001 EventLedger; KERNEL-002 KernelActionCatalogV1; WP-KERNEL-004 refinement acceptance criteria AC-SWARM-HARNESS-PRIMITIVE, AC-SWARM-HARNESS-N8, AC-SWARM-HARNESS-INVARIANTS.

---

## 6.8 Mixture-of-Depths Preliminary Research (Informative; deferred implementation) [ADD v02.186]

**Why**
Mixture-of-Depths (MoD) is a 2024 technique (Raposo et al., DeepMind, arXiv:2404.02258) that routes tokens through varying compute paths through model depth. It is theoretically attractive for KERNEL-004's inference-research lab, but as of 2026 it requires special model architecture support not present in stock `llama.cpp`, `candle`, `mistral.rs`, or `vLLM`. This section [ADD v02.186] captures the research basis so a future implementation WP can pick up without re-doing the survey, AND makes the spec-deferral explicit so no new stub / WP is created in v02.186.

**What**
Background, reference implementations, comparison against already-shipped techniques (Section 4.5 layer-skip and Section 4.7.1g self-speculative decoding), open research questions, and the explicit non-implementation invariant.

---

### 6.8.1 Context

MoD (Raposo et al. 2024) routes tokens through varying compute paths through model depth: at each block, a routing decision sends some tokens through the full block compute and others through a residual-only fast path. The claimed savings are FLOP-level (more compute on tokens that need it; less on tokens that do not).

Architecture requirement: the model must be **trained** with the MoD routing layer in place; you cannot retrofit MoD onto a stock pretrained transformer without re-training the routing weights. No production engine (`llama.cpp`, `candle`, `mistral.rs`, `vLLM`) supports MoD natively in 2026.

### 6.8.2 Reference implementations

- `sramshetty/mixture-of-depths` -- unofficial, training-focused; PyTorch reference.
- `raymin0223/mixture_of_recursions` -- NeurIPS 2025 sibling work (mixture-of-recursions; related but distinct).
- **Zero production MoD checkpoints currently on Hugging Face** as of 2026-05-18.

### 6.8.3 Comparison vs. shipped techniques

| Technique | Routes by | Adapter | Spec status |
|---|---|---|---|
| MoD (Section 6.8) | **Token** (per-token routing decision) | None yet | Spec-deferred |
| Layer-skip (Section 4.5) | **Layer index** (skip certain layers entirely) | LlamaCppRuntime | GA in v02.186 |
| Self-speculative decoding (Section 4.7.1g) | **Draft/target model coupling** | LlamaCppRuntime | GA in v02.186 |

These are sibling techniques with different routing axes; they are **not substitutes** for each other. A future MoD-capable model could in principle compose with layer-skip and speculative decoding.

### 6.8.4 Open research questions

Tracked for a future implementation WP (NOT created in v02.186):

- (a) **Training cost** -- which engine fork hosts an MoD-prepared training pipeline that produces a usable checkpoint?
- (b) **CandleRuntime gaps** -- what hook contract would be needed to inject the per-block routing decision at inference time?
- (c) **Capability declaration shape** -- how does an MoD-supporting model declare its per-token compute path (extension to `ModelCapabilities` in Section 4.6.3)?
- (d) **Inference perf budget** -- MoD-claimed FLOP savings on real (not paper) 2026 workloads; what is the latency vs. quality trade?
- (e) **Interaction with KV-cache prefix sharing** -- does per-token routing break or degrade prefix-cache reuse (Section 4.7.1b)?

### 6.8.5 Explicit non-implementation

- **NO** implementation in KERNEL-004.
- **NO** new stub created for MoD (work-packet stub, refinement, microtask plan -- none).
- **NO** new WP created for MoD.

Phase 3 roadmap reflection bullet (Stage 3 will add `[ADD v02.186]`) tracks the deferral. Revisit when **either** (a) a production engine path materializes, **or** (b) operator explicitly authorizes custom training+runtime work to land MoD inside Handshake.

**Cross-references:** Section 4.5 layer-wise inference (sibling); Section 4.7 Inference Research Lab; Section 4.7.1g self-speculative decoding; Section 4.6.3 ModelCapabilities (extension shape for future MoD); Section 7-6 Phase 3 roadmap (Stage 3 will add `[ADD v02.186]` deferral bullet); WP-KERNEL-004 refinement acceptance criteria AC-MOD-RESEARCH-DOC, AC-MOD-NO-STUB.

---


<a id="7-user-experience-development"></a>

## 6.10 Media Downloader v2 Depth (Normative) [ADD v02.189]

**Why**
Section 10.14 defines the Media Downloader worksurface (modes, capability gates, output routing) at product altitude. The media-downloader-v2 (CKC source: Media-Downloader-v2) implementation needs a deeper backend contract so that long-running archival work survives process restarts, authenticates against access-controlled sources without leaking secrets into evidence, gates every external fetch behind an allowlist + capability decision, and emits sanitized telemetry. This section [ADD v02.189] makes that depth normative: it pins OutputRootDir configuration, staged resumable download sessions, cookie/header auth with secret redaction, allowlist/capability gating, and sanitized telemetry to the same governed substrate as Section 6.0 (Mechanical Tool Bus), Section 6.2 (ASR), and Section 6.3 (engines). Every contract here runs as a Workflow-Engine job writing to ArtifactStore with EventLedger + Flight Recorder evidence and a recoverable receipt; nothing runs as a hidden process-local call.

**What**
Defines OutputRootDir resolution and portability, the staged download-session record + checkpoint contract (resume after restart), the auth-context contract (cookie/header injection with redaction), the allowlist + capability gating contract, the sanitized telemetry + receipt contract, and the storage/quiet-mode invariants. All shapes are spec-altitude record/field/event/ID definitions; no product code is prescribed.

---

### 6.10.1 Execution substrate (LAW)

LAW-MDV2-EXEC-001: Every media-downloader-v2 operation (URL expansion, fetch, probe, merge, materialization, crawl page-walk) MUST execute as a Workflow-Engine job / AI Job under the Section 6.0 Mechanical Tool Bus. No media-downloader work MAY run from UI code, from an ad-hoc background thread, or as a hidden process-local call. Tool execution (TOOL-YTDLP, TOOL-FFMPEG, TOOL-FFPROBE) MUST occur as `proc.exec` invocations gated per Section 6.9.4.

LAW-MDV2-EXEC-002: localhost or any in-process helper is NEVER the source of truth. The canonical state of any download session is the EventLedger event stream plus the ArtifactStore manifest for its outputs. Any in-memory queue, renderer cache, or process-local progress counter is a projection of that canonical state and MUST be reconstructable from it.

LAW-MDV2-EXEC-003: Every mutation of session state (enqueue, start, checkpoint, pause, resume, item-result, finalize, fail, cancel) MUST emit an EventLedger entry and a Flight Recorder event, and MUST leave a recoverable artifact or receipt per Section 6.9.5. A mutation that does not leave EventLedger + recoverable evidence is a violation.

### 6.10.2 OutputRootDir configuration (Normative)

OutputRootDir is the operator-configured default materialization root for user-visible media copies (10.14.6). media-downloader-v2 MUST resolve it through a single configuration contract:

- `OutputRootConfigV2` record fields: `root_id` (stable id), `configured_root` (operator-set base; portable form only), `resolved_root` (absolute path resolved at job time), `materialization_mode` ("copy" | "hardlink" | "symlink"; "hardlink" preferred where the filesystem supports it), `per_mode_subdirs` (map of source_kind -> relative subpath, defaults per 10.14.6: `media_downloader/youtube/`, `media_downloader/instagram/`, `media_downloader/forumcrawler/`, `media_downloader/videodownloader/`).

LAW-MDV2-OUT-001: `configured_root` MUST be stored in portable form. Drive-letter, user-profile, machine-local mount, and absolute machine paths MUST NOT be persisted as the configured root; resolution to an absolute `resolved_root` happens at job time from the portable base plus host config. A persisted absolute or machine-local `configured_root` is a portability defect.

LAW-MDV2-OUT-002: Canonical download output (Bronze/Raw) MUST land in ArtifactStore with a manifest, SHA-256 content hash, and provenance sidecar BEFORE any OutputRootDir materialization. Materialization under `resolved_root` is a derived, user-visible copy/hardlink of the canonical artifact and MUST emit an ExportRecord with `materialized_paths[]` populated (2.3.10).

LAW-MDV2-OUT-003: Product output (media, sidecars, transcripts, manifests, cookie jars, receipts) MUST NEVER be written into `.GOV`. Cookie-jar and other "high"-classified artifacts MUST NOT be written under OutputRootDir (10.14.5); they remain ArtifactStore-resident with `exportable=false`.

### 6.10.3 Staged resumable download sessions (Normative)

media-downloader-v2 download work is modeled as a staged session that survives process restart. The canonical record is `MdDownloadSessionV2`:

- `session_id` (stable id), `parent_job_id` (Workflow-Engine job id), `source_kind` ("youtube" | "instagram" | "forumcrawler" | "videodownloader"), `auth_context_ref` (Section 6.9.4; nullable for no-account mode), `allowlist_policy_id` (Section 6.9.4), `output_root_id`, `created_at`, `updated_at`.
- `stage` enum (the staged lifecycle): `resolving` (URL normalize/dedupe/expansion) -> `enqueued` -> `fetching` -> `probing` -> `merging` -> `materializing` -> `finalized`; terminal branches `paused`, `failed`, `cancelled`.
- `items[]` of `MdItemStateV2`: `item_id`, `normalized_url`, `stable_source_id` (provider id when known), `content_hash` (SHA-256 when known), `stage`, `bytes_downloaded`, `bytes_total` (nullable), `part_path_ref` (the `.part` staging artifact ref; 10.14.8), `attempt_count`, `last_error_code` (nullable), `resume_token` (opaque per-item byte/offset/range cursor).

LAW-MDV2-RESUME-001: A session MUST be resumable after process restart with NO loss of completed-item work. Resume MUST reconstruct session and per-item stage from the EventLedger event stream plus ArtifactStore manifests; the in-memory queue is rebuilt, never read back from a process-local store. SQLite (TECH-SQLITE) MUST NOT be used as a session/checkpoint store in runtime, tests, fixtures, mocks, examples, fallbacks, cache, or compatibility paths; canonical session/checkpoint state lives in PostgreSQL + EventLedger + ArtifactStore.

LAW-MDV2-RESUME-002: Partial downloads MUST stage to a `.part` artifact and finalize only after validation (10.14.8: content-type/sniff reject of non-media, ffprobe-equivalent validation). On resume, an item with a valid `resume_token` and intact `.part` artifact MUST continue from the recorded offset where the source supports range requests; otherwise it MUST restart that single item without re-fetching already-finalized items.

LAW-MDV2-RESUME-003: A `MdCheckpointV2` event MUST be emitted at every stage transition and at bounded progress intervals during `fetching`, carrying `session_id`, `item_id` (or null for session-level), `stage`, `bytes_downloaded`, `bytes_total`, `resume_token`. Checkpoints are the recovery anchor; a stage transition without a checkpoint event is a violation.

LAW-MDV2-RESUME-004: Dedupe across runs MUST use `stable_source_id` and/or SHA-256 content hash (10.14.7). A resumed or re-enqueued session MUST NOT re-download or re-materialize an item already finalized with a matching stable id or content hash.

### 6.10.4 Cookie/header auth, allowlist, and capability gating (LAW)

Auth context is the redaction-critical surface. The canonical record is `MdAuthContextV2`:

- `auth_context_ref` (stable id), `auth_mode` (PRIM-MdAuthMode: "none" | "session" | "cookie_jar" | "header"), `session_ref` (PRIM-MdSessionRecordV0 / PRIM-MdSessionsRegistryV0 when `auth_mode="session"`), `cookie_jar_artifact_ref` (Netscape cookies.txt artifact, classification "high", `exportable=false`; 10.14.5), `header_secret_refs[]` (references to secret-store entries for custom Authorization / cookie / token headers; NEVER inline values).

LAW-MDV2-AUTH-001: Cookies, header tokens, session secrets, and Authorization values MUST be carried by reference (`*_ref`) only. Inline secret material MUST NOT appear in the session record, the job spec, the EventLedger, the Flight Recorder, the receipt, or any log. Header/cookie injection into a `proc.exec` (TOOL-YTDLP) or HTTP fetch happens at execution time from the secret store under `secrets.use`; the materialized cookie jar is an ArtifactStore artifact, never an OutputRootDir/`.GOV` file.

LAW-MDV2-AUTH-002: Every emitted record and event MUST be redacted before persistence. The redactor MUST scrub cookie values, `Set-Cookie`/`Cookie` headers, Authorization headers, bearer/token query params, and any field matching a secret pattern, replacing them with a redaction marker plus the stable `*_ref` (so evidence proves WHICH secret was used without revealing it). A receipt or event that contains an unredacted secret is a hard violation.

LAW-MDV2-AUTH-003: The system MUST NOT bypass access controls or circumvent DRM/paid restrictions (10.14.12). Private/members-only content REQUIRES an authorized `auth_mode`; Handshake MUST NOT request or collect passwords in a Handshake-owned form (10.14.4) and MUST honor Stage session isolation (no cookie/storage/cache bleed; 10.13).

LAW-MDV2-CAP-001: Every external fetch MUST pass an allowlist decision before any network call. The `MdAllowlistPolicyV2` record fields: `allowlist_policy_id`, `allowed_domains[]`, `explicit_url_lists[]`, `default_decision` ("deny" for crawling non-allowlisted domains per 10.14.9), `rate_limit` (polite default), `max_pages` (crawler bound; default 1500, hard cap 5000 per 10.14.9), `robots_posture` ("respect"). A fetch to a non-allowlisted domain under a crawl posture MUST be denied and the denial recorded.

LAW-MDV2-CAP-002: Required capabilities (minimum) MUST be checked per request and MUST be members of the CapabilityRegistry: `fs.write:artifacts`, `net.http` (allowlist-scoped), `proc.exec:<archiver_allowlist>` (yt-dlp/equivalent), `proc.exec:<video_allowlist>` (ffmpeg/ffprobe), `secrets.use` (when auth context is non-"none"). Any unknown capability id MUST be rejected with HSK-4001 (UnknownCapability). Every allow/deny decision MUST be recorded as a Flight Recorder event with `capability_id`, `actor_id`, `job_id`, `decision_outcome` and NO secret payload (11.1).

### 6.10.5 Sanitized telemetry and receipts (Normative)

LAW-MDV2-TEL-001: media-downloader-v2 MUST emit structured, leak-safe telemetry events. Canonical event kinds: `media_downloader.job_state`, `media_downloader.progress`, `media_downloader.item_result` (extending 10.14.11). Each event payload carries: `session_id`, `parent_job_id`, `source_kind`, `url` (allowlist-checked; query secrets stripped), `stage`, `item_index`, `item_total`, `bytes_downloaded`, `bytes_total`, `status`, `error_code` (nullable). Payloads MUST pass the Section 6.9.4 redactor; cookies/headers/tokens MUST NOT appear.

LAW-MDV2-TEL-002: For each successfully stored downloaded file the system MUST emit a bronze ingest event (FR-EVT-DATA-001 bronze_record_created) with `ingestion_source = { type: "system", process: "media_downloader" }` and `external_source.url` populated where available (10.14.11).

LAW-MDV2-TEL-003: Every session MUST produce a recoverable `MdSessionReceiptV2` artifact in ArtifactStore at finalize/fail/cancel: `session_id`, `parent_job_id`, `source_kind`, `auth_context_ref` (reference only), `allowlist_policy_id`, `output_root_id`, `item_count`, `succeeded`, `failed`, `skipped_deduped`, `materialized_paths[]`, `manifest_artifact_ref` (the per-item manifest: `page_url, discovered_url, chosen_url, sha256, bytes, status, reason_skipped` for crawler; 10.14.9), `started_at`, `ended_at`, `terminal_stage`. The receipt MUST be sufficient to reconstruct what was attempted, fetched, deduped, and materialized for replay/audit, and MUST contain no secret material.

LAW-MDV2-TEL-004: Downloaded audio/video outputs MUST remain valid `MediaSource` (PRIM-MediaSource) inputs for governed ASR jobs, preserving stable linkage between the source media artifact, ffprobe-derived media facts, the selected ASR job/profile, and resulting transcript artifacts (10.14.11.1, FEAT-ASR handoff). Caption tracks, when available, MUST be stored as WebVTT sidecars with language metadata (10.14.7).

### 6.10.6 Quiet-mode and portability invariants (LAW)

LAW-MDV2-QUIET-001: media-downloader-v2 jobs and their spawned `proc.exec` tools MUST run non-intrusively per HBR-QUIET-001..004: no foreground window pop, no focus steal, no global-shortcut hijack, no unbounded synthetic input. Progress is surfaced via telemetry events and operator-console projections, never by raising a window. Spawned tool processes MUST be attributed in the ProcessOwnershipLedger (Section 5.7).

LAW-MDV2-QUIET-002: All artifacts (media, `.part` staging files, sidecars, transcripts, cookie jars, manifests, receipts) MUST materialize through ArtifactStore with manifests and portable references. No drive-letter, user-profile, or machine-local absolute path MAY be persisted in any record, event, or receipt; only portable references plus job-time-resolved roots are allowed. Portability regressions are project-quality defects.

**Cross-references:** Section 6.0 Mechanical Tool Bus (execution substrate); Section 6.2 ASR (transcript handoff, FEAT-ASR); Section 6.3 mechanical engines (peer); Section 10.13 Stage Sessions (auth isolation); Section 10.14 Media Downloader (product surface; OutputRootDir, capability gates, output routing, telemetry); Section 5.7 ProcessOwnershipLedger; KERNEL-001 EventLedger; ArtifactStore + manifests (2.3.10 ExportRecord); HSK-4001 capability registry (11.1); HBR-QUIET-001..004; [GLOBAL-PORTABILITY-001]..[GLOBAL-PORTABILITY-013]; FEAT-MEDIA-DOWNLOADER (Appendix 12).

## 6.11 Media Transcript and Caption Pipeline (Normative) [ADD v02.189]

**Why**
Section 6.2 (ASR Subsystem) defines local-first transcription but leaves the governed job/artifact contract for media probing, canonical transcript formats, and caption/subtitle generation under-specified at spec altitude. The CKC source (ffmpeg/ffprobe transcript + caption generation) requires that media probing, transcription, and caption emission run as governed Workflow-Engine jobs that materialize canonical artifacts through the shared ArtifactStore with EventLedger + Flight-Recorder evidence. This section [ADD v02.189] EXTENDS FEAT-ASR with that contract. It does NOT replace Section 6.2; it pins the previously-informal extraction step (Section 6.2.3.1 step 3), the transcript persistence step (step 6), and the [ADD v02.158] portability requirements into explicit jobs, records, events, and receipts. Caption/subtitle generation is new product depth layered on the existing transcript artifact.

**What**
Defines three governed jobs (media-probe, ASR-transcribe, caption-render), the canonical transcript record, the caption/subtitle artifact set, timing-anchor + source-media-hash contracts, the job lifecycle events, the receipts, and the storage/quiet/portability guardrails. All records are spec-altitude contracts (field shapes + IDs + events), not product code.

---

### 6.11.1 LAW: governed-job-only execution

All media probing, transcription, and caption rendering MUST execute as governed Workflow-Engine jobs (AI Jobs where ASR inference runs) with capability gates. No process-local hidden ffmpeg/ffprobe/Whisper invocation is permitted; no localhost endpoint is authority of record. Specifically:

- ffmpeg / ffprobe (TOOL-FFMPEG, TOOL-FFPROBE) run as a `media.probe` job and as the audio-extraction phase of the `asr.transcribe` job; they are never called inline from a Tauri command process as authority.
- Whisper-family inference (TECH-WHISPER) runs as the inference phase of the `asr.transcribe` AI Job, subject to the Section 6.2.2.4 model-selection policy.
- caption rendering (ffmpeg/ffprobe muxing + transcript-to-subtitle transform) runs as a `caption.render` job.

Each job is admitted only when its required capabilities are granted (media-read, tool-exec, ai-inference, artifact-write). A denied capability fails admission with a typed receipt; it never silently downgrades to an inline call.

### 6.11.2 LAW: storage authority

Job state, lineage, and evidence persist ONLY to PostgreSQL + EventLedger + ArtifactStore + CRDT/write-box. SQLite is FORBIDDEN for this feature in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, and exports. No new TECH-SQLITE dependency is introduced by 6.10.

All produced media derivatives (extracted/normalized audio, transcript artifact, caption/subtitle artifacts, probe report) materialize through the shared ArtifactStore with manifests and content-addressed identity. Product output is NEVER written into `.GOV`. Artifact paths and manifest references MUST be portable: no drive-letter, user-profile, or machine-local absolute paths; references are ArtifactStore identifiers, not filesystem paths.

### 6.11.3 Job: media.probe (Normative)

`media.probe` is a Workflow-Engine job wrapping TOOL-FFPROBE (with TOOL-FFMPEG available for stream inspection). Input is a `PRIM-MediaSource` reference (Section 6.2.3.3). Output is a `MediaProbeReportV1` artifact:

```json
{
  "probe_report_id": "...",
  "media_source_id": "...",
  "source_media_hash": "sha256:<hex>",
  "container": "mp4 | mkv | wav | mov | ...",
  "duration_ms": 0,
  "streams": [
    {"index": 0, "kind": "audio | video | subtitle",
     "codec": "...", "sample_rate_hz": 0, "channels": 0,
     "width": 0, "height": 0, "language_tag": "und | en | ..."}
  ],
  "ffprobe_tool_version": "...",
  "probed_at": "<iso8601>"
}
```

`source_media_hash` is computed over the source bytes and is the lineage key that binds every downstream transcript and caption artifact to its exact input. The probe report is an ArtifactStore artifact with a manifest; the job emits the lifecycle events in Section 6.10.7.

### 6.11.4 Job: asr.transcribe (Normative; extends Section 6.2)

`asr.transcribe` is the governed AI Job that EXTENDS the Section 6.2 pipeline. It consumes the `MediaProbeReportV1` (or runs probe inline as phase 0 when none exists), performs ffmpeg audio extraction/normalization (mono, 16 kHz, 16-bit PCM per Section 6.2.3.1), runs TECH-WHISPER inference under the Section 6.2.2.4 selection policy, and emits a canonical transcript artifact `TranscriptArtifactV1`:

```json
{
  "transcript_id": "...",
  "media_source_id": "...",
  "source_media_hash": "sha256:<hex>",
  "language": "en | ...",
  "model": {"family": "whisper", "variant": "large-v3",
            "runtime": "faster-whisper", "precision": "fp16"},
  "selection_path": "gpu_happy | gpu_constrained | cpu_only | ...",
  "segments": [
    {"segment_id": "...", "start_ms": 0, "end_ms": 0,
     "text": "...", "confidence": 0.0, "speaker": "S0 | null",
     "source": "local_whisper | cloud_fallback"}
  ],
  "timing_anchors": [
    {"anchor_id": "...", "t_ms": 0, "segment_id": "...",
     "kind": "segment_start | word | chapter"}
  ],
  "format_version": "TranscriptArtifactV1",
  "created_at": "<iso8601>"
}
```

The transcript artifact is the canonical backend artifact required by Section 6.2 [ADD v02.158], not UI-only text. `timing_anchors` make transcript positions independently addressable so Loom/Lens (FEAT-ATELIER-LENS) and AI-Ready retrieval can bridge to time spans without re-deriving timing. `selection_path` and `model` record reproducibility metadata per Section 6.2.2.4.5. Cloud-fallback segments MUST be annotated (`source = "cloud_fallback"`) per Section 6.2.2.4.4(5).

### 6.11.5 Job: caption.render (Normative; new depth)

`caption.render` is a Workflow-Engine job that transforms a `TranscriptArtifactV1` into one or more `CaptionArtifactV1` outputs (subtitle/caption sidecars and, optionally, a muxed media derivative) using ffmpeg:

```json
{
  "caption_artifact_id": "...",
  "transcript_id": "...",
  "media_source_id": "...",
  "source_media_hash": "sha256:<hex>",
  "format": "srt | vtt | ass",
  "language": "en | ...",
  "max_line_chars": 0,
  "max_lines_per_cue": 0,
  "min_cue_ms": 0,
  "max_cue_ms": 0,
  "cue_count": 0,
  "derived_from_timing_anchors": true,
  "muxed_media_artifact_id": "... | null",
  "created_at": "<iso8601>"
}
```

Caption cues are derived deterministically from `TranscriptArtifactV1.segments` + `timing_anchors`; given the same transcript and the same caption profile (line/cue bounds, format), `caption.render` MUST produce byte-identical caption output. Caption and any muxed-media derivative materialize through ArtifactStore with manifests; the muxed output references the source via `source_media_hash` for lineage. Caption rendering MUST NOT re-run ASR; it consumes the existing transcript artifact only.

### 6.11.6 LAW: lineage chain

The lineage chain is `PRIM-MediaSource -> MediaProbeReportV1 -> TranscriptArtifactV1 -> CaptionArtifactV1`, bound at every hop by a shared `source_media_hash`. Any artifact whose `source_media_hash` does not match its upstream is a lineage break and MUST be rejected with a typed receipt rather than persisted. This makes the full pipeline replayable and auditable from the Flight Recorder.

### 6.11.7 EventLedger + Flight-Recorder evidence (Normative)

Every job emits bounded lifecycle events to the EventLedger, recorder-visible per Section 6.2 [ADD v02.158]:

- `media.probe.{admitted|started|completed|failed}`
- `asr.transcribe.{admitted|started|progress|segment_completed|completed|failed}`
- `caption.render.{admitted|started|completed|failed}`

`*.progress` and `*.segment_completed` events are bounded (rate-limited / coalesced) so a multi-hour job cannot flood the ledger. Each `*.failed` event carries a typed error class and preserves any partial results (completed segments) per Section 6.2.3.2(4). Every mutation/job produces a recoverable artifact-or-receipt: on success the artifact id, on failure a typed failure receipt referencing the job id, the `source_media_hash`, and the partial-result artifact id when one exists.

### 6.11.8 LAW: secret + log hygiene

Receipts, EventLedger payloads, and Flight-Recorder traces for these jobs MUST scrub secrets, cookies, tokens, and credentials (relevant when the source media originated from a governed media-downloader job whose fetch context may carry auth). Tool command lines recorded for reproducibility MUST redact any credential-bearing arguments. No raw secret material appears in any artifact manifest.

### 6.11.9 LAW: non-intrusive operation (HBR-QUIET)

These jobs run headless under the Workflow-Engine. ffmpeg/ffprobe/Whisper subprocesses MUST NOT open foreground windows, steal focus, hijack global shortcuts, or emit unbounded synthetic input, per HBR-QUIET-001..004 and [GLOBAL-BUILD-046]..[GLOBAL-BUILD-054]. Subprocesses are tracked in the ProcessOwnershipLedger (Section 5.7) and attributed to the originating job/session id.

### 6.11.10 Receipts (Normative)

Three typed receipts: `MediaProbeReceiptV1`, `TranscribeReceiptV1`, `CaptionRenderReceiptV1`. Each receipt carries `{job_id, feature_id="FEAT-ASR", source_media_hash, input_artifact_ids[], output_artifact_id|null, capability_grants[], tool_versions{}, status, error_class|null, partial_artifact_id|null, emitted_at}`. Receipts are ArtifactStore-backed and EventLedger-referenced; they are the recoverable evidence unit required by Section 6.10.7.

**Cross-references:** Section 6.2 ASR Subsystem (extended here; this section pins Section 6.2.3.1 steps 3 and 6 and the [ADD v02.158] portability requirements into jobs); Section 5.7 ProcessOwnershipLedger; FEAT-ATELIER-LENS (transcript time-span consumer); FEAT-AI-READY-DATA (transcript retrieval indexing); media-downloader feature (upstream governed media source); KERNEL-001 EventLedger; ArtifactStore manifests; Workflow-Engine AI Jobs + capability gates; HBR-QUIET-001..004; [GLOBAL-BUILD-046]..[GLOBAL-BUILD-054]; [GLOBAL-PORTABILITY-001]..[GLOBAL-PORTABILITY-008].

## 6.12 Sourcing-Spec Schema and Handler Version Matrix (Normative) [ADD v02.189]

**Why**
Sourcing workflows (media download, ASR transcription, external-tool ingestion, export pipelines) are increasingly authored as declarative sourcing-specs and dispatched against handlers that evolve independently of the specs that target them. Without a pinned schema and an explicit multi-version handler compatibility matrix, a spec authored against handler v1 can silently bind to handler v2 with drifted side-effect or capability semantics, ingestion can double-apply on retry, and version mismatches can fail without recoverable evidence. This section [ADD v02.189] layers a normative sourcing-spec schema and a handler version matrix on top of the workflow spec registry so spec-to-handler binding is deterministic, idempotent, version-pinned, and receipted.

**What**
Defines (1) the YAML sourcing-spec schema and its parse/validate contract, (2) the handler version matrix and routing law, (3) version pinning and mismatch-rejection semantics with receipts, (4) idempotent ingestion semantics, and (5) storage/execution/evidence guardrails. All binding decisions, ingestions, and rejections execute as governed Workflow-Engine jobs and emit EventLedger + Flight Recorder evidence.

---

### 6.12.1 Definitions (Normative)

- **Sourcing-spec:** a declarative document (authored as YAML, canonicalized to JSON for hashing) that names a source, a target handler family, a pinned handler version constraint, ingestion parameters, and capability requirements. The canonical record is `PRIM-SourcingSpecRecordV1`.
- **Handler:** a governed Workflow-Engine job profile (e.g., media-downloader, ASR, external-tool, export) that consumes a sourcing-spec and produces artifacts. Handlers are identified by `handler_family` (stable) and `handler_version` (semver).
- **Handler version matrix:** the authoritative compatibility table mapping `(handler_family, handler_version)` to supported sourcing-spec `schema_version` ranges, side-effect class, idempotency class, and required capabilities. The canonical record is `PRIM-HandlerVersionMatrixEntryV1`.
- **Binding decision:** the deterministic resolution from a sourcing-spec to a concrete `(handler_family, handler_version)`. The canonical record is `PRIM-SourcingBindingDecisionV1`.
- **Version-mismatch rejection:** the deterministic refusal to bind when no matrix entry satisfies the spec's pin. The canonical receipt is `PRIM-VersionMismatchReceiptV1`.

> Normative rule: The handler version matrix is authoritative. Handler metadata advertised by any process-local or remote source is advisory only and MUST NOT override a matrix entry (consistent with the tool-registry authority rule in S6.0.2.1).

---

### 6.12.2 Sourcing-spec schema (LAW)

A sourcing-spec is authored as YAML and MUST canonicalize to a deterministic JSON form before hashing and storage. The canonical record `PRIM-SourcingSpecRecordV1` MUST carry at minimum:

- `sourcing_spec_id` (string): stable identity for the spec instance.
- `schema_version` (string): semver of the sourcing-spec schema (`MAJOR.MINOR.PATCH`).
- `source` (object): `{ kind, ref }` describing the source (e.g., `media_url`, `artifact_ref`, `feed`). External refs MUST be portable (no drive-letter, user-profile, or machine-local paths).
- `handler_family` (string): the target handler family (e.g., `media_downloader`, `asr`, `external_tool`, `export`).
- `handler_version_pin` (string): a semver constraint (exact, caret, or range) the binding MUST satisfy (S6.11.4).
- `params` (object): handler-scoped ingestion parameters; payloads over 32KB MUST be passed by `artifact_ref`, never inlined (consistent with S6.0.2.4 / S6.3.0 sizing rules).
- `required_capabilities` (array): capability strings (network/file/process/device/secrets) evaluated deny-by-default against AS11.1.
- `idempotency_key` (string, optional): caller-supplied dedupe key for ingestion (S6.11.5).
- `spec_hash` (string): SHA-256 over the canonicalized JSON form, used for registry identity and replay.

Parse/validate contract (MUST):
1. **Schema validation:** the canonicalized JSON form MUST validate against the registered sourcing-spec JSON Schema (draft 2020-12) for its `schema_version` before any binding is attempted. Invalid specs are rejected with a `validation` error and a Flight Recorder entry; no handler job is enqueued.
2. **No silent coercion:** unknown top-level fields MUST be rejected, not ignored, to prevent drift between spec author intent and handler interpretation.
3. **Secret hygiene:** secrets, cookies, and tokens MUST NOT be embedded in the sourcing-spec body; they MUST be referenced through governed capability/consent grants (AS11.1) and MUST be scrubbed from every receipt, EventLedger row, and Flight Recorder artifact.
4. **Canonical hashing:** `spec_hash` MUST be computed over the canonicalized JSON (stable key ordering, normalized whitespace), so semantically identical YAML produces identical hashes regardless of source formatting.

> Normative rule: Sourcing-spec records, handler matrix entries, binding decisions, and receipts persist through PostgreSQL + EventLedger and materialize derived artifacts through ArtifactStore. SQLite MUST NOT be used in any runtime, test, fixture, mock, example, cache, fallback, or compatibility path for this feature.

---

### 6.12.3 Handler version matrix (LAW)

Each matrix entry `PRIM-HandlerVersionMatrixEntryV1` MUST declare:

- `handler_family` (string) + `handler_version` (semver).
- `supported_schema_versions` (range): the sourcing-spec `schema_version` range this handler accepts.
- `side_effect` (one of `READ | WRITE | EXECUTE`) and `idempotency` (one of `IDEMPOTENT | IDEMPOTENT_WITH_KEY | NON_IDEMPOTENT`), consistent with S6.0.2.3.
- `required_capabilities` (array): minimum capability set the handler version requires.
- `determinism` (D0-D3 per S6.3.0) and evidence policy.
- `status` (one of `ACTIVE | DEPRECATED | SUNSET`) with `deprecated_since` / `sunset_on` when applicable.
- `job_profile_ref`: the Workflow-Engine AI Job profile that implements this handler version.

Rules (MUST):
1. **No silent behavior drift:** any change to a handler version's `side_effect`, `idempotency`, or `required_capabilities` MUST be expressed as a new `handler_version` matrix entry; existing entries are immutable once published (consistent with S6.0.2.2).
2. **Authoritative routing:** a sourcing-spec MUST bind only to a matrix entry whose `supported_schema_versions` includes the spec `schema_version` AND whose `handler_version` satisfies the spec `handler_version_pin`.
3. **Capability intersection:** the bound handler version's `required_capabilities`, unioned with the spec `required_capabilities`, MUST be satisfied by the session-scoped capability intersection (AS11.1); otherwise the binding is denied or escalated for consent, never executed.
4. **Sunset enforcement:** binding to a `SUNSET` entry MUST be rejected; binding to a `DEPRECATED` entry MUST emit a deprecation warning event but MAY proceed if the pin explicitly targets it.

---

### 6.12.4 Version pinning, routing, and mismatch rejection (LAW)

Binding executes as a governed Workflow-Engine job and produces a deterministic `PRIM-SourcingBindingDecisionV1` containing the resolved `(handler_family, handler_version)`, the matched matrix entry id, the `spec_hash`, the evaluated capability intersection result, and the resolution reason.

Routing law (MUST):
1. **Pin resolution:** given a spec pin, the engine MUST select the highest `ACTIVE` `handler_version` satisfying both the pin and the `supported_schema_versions` constraint. Selection MUST be deterministic and replayable from the `spec_hash` plus the matrix snapshot id.
2. **Matrix snapshot:** every binding decision MUST record the `matrix_snapshot_id` it resolved against, so replay binds against the same matrix state even if the matrix later changes.
3. **No fallback downgrade:** if no entry satisfies the pin, the engine MUST NOT silently downgrade to an unpinned or older version; it MUST reject.
4. **Mismatch rejection with receipt:** a failed resolution MUST produce a `PRIM-VersionMismatchReceiptV1` recording `sourcing_spec_id`, `spec_hash`, requested pin, evaluated candidate versions, the matrix snapshot id, and a machine-readable `reason` (`no_matching_version | schema_unsupported | sunset | capability_denied`). The receipt is a recoverable artifact materialized through ArtifactStore and referenced from the EventLedger row; no handler job is enqueued.
5. **No process-local execution:** binding and rejection MUST run as Workflow-Engine jobs; no process-local hidden handler invocation and no localhost endpoint may be treated as the authority for binding outcomes.

> Normative rule: A version-mismatch rejection is a first-class, evidenced outcome -- not a swallowed error. The receipt MUST be sufficient for a no-context model to understand why the spec did not bind and what pin or matrix change would resolve it.

---

### 6.12.5 Idempotent ingestion (LAW)

Handlers declared `IDEMPOTENT_WITH_KEY` or whose specs supply `idempotency_key` MUST dedupe ingestion so retries do not double-materialize artifacts or double-emit downstream effects (consistent with S6.0.2.3).

Rules (MUST):
1. **Ingestion key:** the effective ingestion identity is `(handler_family, handler_version, spec_hash, idempotency_key?)`. A repeated ingestion with the same identity MUST return the prior ingestion's receipt and artifact refs rather than re-executing side effects.
2. **Receipt:** every ingestion (new or deduped) MUST emit a `PRIM-SourcingIngestionReceiptV1` recording the binding decision id, the produced artifact manifest refs, the dedupe outcome (`fresh | deduped`), and timing. The receipt is materialized through ArtifactStore with a manifest and referenced from the EventLedger.
3. **Artifact materialization:** all sourced media, sidecars, transcripts, downloads, and exports MUST materialize through the shared ArtifactStore with SHA-256-hashed manifests and provenance; product output MUST NOT be written into .GOV, and paths MUST be portable.
4. **Partial-failure recovery:** an ingestion that fails mid-flight MUST leave a recoverable receipt describing completed vs pending artifacts, so a retry under the same ingestion key resumes deterministically rather than duplicating completed work.

---

### 6.12.6 Evidence, guardrails, and non-intrusion (LAW)

- **EventLedger + Flight Recorder:** every parse, validation, binding decision, mismatch rejection, and ingestion MUST emit an EventLedger row and Flight Recorder evidence with a recoverable receipt/artifact. Gate outcomes (schema, capability, budget, integrity, provenance, determinism per S6.3.0) MUST be logged and surfaced in Problems when denied or degraded.
- **Governed execution only:** all handler execution (media-downloader, ASR, external-tool, export, any LLM/ComfyUI step) MUST run as governed Workflow-Engine jobs / AI Jobs with capability gates; no process-local hidden calls and no localhost authority as source of truth.
- **Secret scrubbing:** secrets, cookies, and tokens MUST be scrubbed from all receipts, EventLedger rows, Flight Recorder artifacts, and matrix/binding records.
- **Non-intrusion (HBR-QUIET):** binding and ingestion are backend operations and MUST NOT steal focus, pop foreground windows, hijack global shortcuts, or emit unbounded synthetic input.
- **Storage authority:** records persist through PostgreSQL + EventLedger + ArtifactStore + CRDT/write-box only; SQLite is forbidden across runtime, tests, fixtures, mocks, examples, fallbacks, cache, and compatibility adapters for this feature.

**Cross-references:** S6.0.2 Unified Tool Surface Contract (identity/versioning/side-effects/idempotency); S6.3.0 Mechanical Tool Bus Contract (gates, artifact-first I/O, determinism D0-D3); FEAT-WORKFLOW-ENGINE (sole execution authority); FEAT-MEDIA-DOWNLOADER and FEAT-ASR (handler families); FEAT-SPEC-ROUTER (spec registry lineage); AS11.1 capabilities/consent; AS11.5 logging.

## 6.9 ComfyUI Custom-Node Intake (Normative) [ADD v02.189]

**Why**
The `engine.comfyui` runtime (Section 9.12) executes ComfyUI workflow graphs for generative AI (generate, upscale, inpaint, style_transfer, refactor). By default ComfyUI persists outputs through its own `SaveImage` node into an engine-local output directory, which is opaque to Handshake governance: outputs are discovered only by post-hoc directory scanning, with no per-output manifest, no in-graph correlation to the originating job, and no capability gate at the write boundary. A first-party Handshake custom bridge node can instead push each produced output directly back into the governed intake path (ArtifactStore + EventLedger) inside the Workflow-Engine job context, with full provenance. This section [ADD v02.189] defines the governed happy-path for detecting that bridge node, registering its capability, and routing its outputs. It is additive to and never replaces the `SaveImage` fallback path: when the bridge node is absent, intake degrades to the directory-scan fallback (Section 6.8.6), which is the existing no-bridge behavior.

**What**
Defines: the bridge-node presence detection contract; the capability registration record; the output-routing contract into ArtifactStore inside the Workflow-Engine job context; the EventLedger + Flight-Recorder evidence shape; the fake-adapter test contract; and the SaveImage fallback boundary. All execution stays inside governed `engine.comfyui` AI Jobs (Section 9.12 + Section 2.6.6 AI Job Model); no process-local or hidden call path is introduced.

---

### 6.9.1 LAW: governed-job containment (HARD)

LAW-COMFY-INTAKE-001: All ComfyUI custom-node intake activity -- presence detection, capability registration, and output routing -- MUST occur inside a governed `engine.comfyui` AI Job under the Workflow-Engine (`PRIM-WorkflowContext` / `PRIM-WorkflowRun` / `PRIM-WorkflowNodeExecution`). There is no standalone intake daemon, no background thread, and no out-of-job ingestion path.

LAW-COMFY-INTAKE-002: The ComfyUI process and its bridge node are NOT an authority. `localhost` ComfyUI endpoints and any in-graph node output are untrusted input until materialized through the shared ArtifactStore with a manifest and recorded in the EventLedger. The EventLedger remains the sole source of truth for what was produced.

LAW-COMFY-INTAKE-003: The bridge node MUST be gated by a ComfyUI execution capability (reusing the `engine.comfyui` capability profile, Section 9.12 + FEAT-CAPABILITIES-CONSENT). If the required capability is not granted in the job's `PRIM-CapabilityProfile`, intake registration is denied with a typed denial record and the job falls back per Section 6.8.6; intake MUST NOT silently proceed.

LAW-COMFY-INTAKE-004: Storage authority for this feature is PostgreSQL (`TECH-POSTGRESQL`) + EventLedger + ArtifactStore + CRDT/write-box (`TECH-CRDT` / `PRIM-WriteBoxV1`) ONLY. SQLite is FORBIDDEN for intake state, fixtures, mocks, caches, fallbacks, and the fake adapter. No intake record, manifest index, or test fixture may be backed by SQLite.

LAW-COMFY-INTAKE-005: Secrets, cookies, API tokens, and any ComfyUI auth headers MUST be scrubbed from receipts, manifests, EventLedger payloads, and Flight-Recorder spans. Intake records reference artifacts and graph node ids only; they never embed credential material or absolute machine-local paths.

---

### 6.9.2 Bridge-node presence detection (Normative)

On `engine.comfyui` job start, the engine performs a single bounded capability probe against the ComfyUI graph being submitted and the connected node registry, producing a `ComfyBridgeNodeProbeV1` record:

- `probe_id` (uuid)
- `workflow_run_id` (FK -> `PRIM-WorkflowRun`)
- `node_class_id` (string; the bridge node class name, e.g. `HandshakeIntakeBridge`)
- `detected` (bool)
- `bridge_protocol_version` (semver string; null when `detected=false`)
- `node_instance_ids[]` (graph node ids bound to the bridge class within this graph)
- `probe_outcome` enum `{ bridge_present, bridge_absent, bridge_incompatible }`
- `fallback_reason` (string; required when outcome != `bridge_present`)
- `probed_at` (timestamp)

Normative: detection is read-only and side-effect-free against the ComfyUI graph. The probe MUST be bounded (single request, fixed timeout); a probe timeout resolves to `bridge_absent` and routes to the SaveImage fallback (Section 6.8.6). `bridge_incompatible` (node present but `bridge_protocol_version` outside the supported range) also routes to fallback with the version recorded in `fallback_reason`. The probe record is emitted to the EventLedger and surfaced as a Flight-Recorder span on the owning job.

---

### 6.9.3 Capability registration (Normative)

When `probe_outcome = bridge_present`, the engine registers the bridge node's declared capabilities into the job context as a `ComfyBridgeCapabilityRecordV1`:

- `registration_id` (uuid)
- `workflow_run_id` (FK -> `PRIM-WorkflowRun`)
- `node_class_id`, `bridge_protocol_version`
- `declared_outputs[]` where each entry is:
  - `output_slot` (string; node output port name)
  - `media_kind` enum `{ image, mask, latent_preview, video, sidecar_json }`
  - `expected_mime` (string)
  - `routing_intent` enum `{ artifact, sidecar, transient }`
- `capability_grant_ref` (FK to the granted `engine.comfyui` `PRIM-CapabilityRegistryEntry` / `PRIM-CapabilityProfile`)
- `consent_decision_ref` (FK -> `PRIM-ConsentDecision`; present when escalation/consent applies)
- `registered_at` (timestamp)

Normative: registration is the binding moment where declared bridge outputs are matched against the granted capability. A declared output whose `media_kind` or `routing_intent` is not permitted by the capability profile is dropped from `declared_outputs` and recorded as a typed `intake_capability_reject` event; it MUST NOT be routed. Registration produces exactly one `ComfyBridgeCapabilityRecordV1` per job and is replay-stable: re-running the same pinned graph + capability profile reproduces the same registration shape (modulo ids/timestamps).

---

### 6.9.4 Output routing into ArtifactStore (Normative)

For each output the bridge node emits during execution, the engine routes it through the shared ArtifactStore inside the same Workflow-Engine job context, producing one `ComfyIntakeOutputRecordV1` per output:

- `intake_output_id` (uuid)
- `workflow_run_id`, `node_execution_id` (FK -> `PRIM-WorkflowNodeExecution`)
- `registration_id` (FK -> Section 6.8.3 record)
- `source_node_instance_id`, `source_output_slot`
- `media_kind`, `mime`
- `artifact_ref` (ArtifactStore content-addressed handle; portable, never a drive-letter/user-profile/machine-local path)
- `artifact_manifest_ref` (manifest produced at materialization; includes content hash, byte length, declared mime, and `engine.comfyui` provenance pins -- graph hash + seed per Section 9.12 STOCHASTIC determinism)
- `routing_intent` (carried from registration: `artifact` | `sidecar` | `transient`)
- `parent_artifact_ref` (for `sidecar`: the primary artifact it annotates; null otherwise)
- `materialized_at` (timestamp)

Normative routing rules:

- `routing_intent = artifact`: materialized as a first-class ArtifactStore artifact with its own manifest.
- `routing_intent = sidecar`: materialized as a sidecar bound to `parent_artifact_ref` via the manifest; sidecars never become orphan primaries.
- `routing_intent = transient` (e.g. `latent_preview`): MAY be streamed to the operator preview surface but MUST NOT be persisted as a durable artifact unless the capability profile explicitly permits preview persistence.
- All durable materialization writes through the shared ArtifactStore. Intake MUST NEVER write product output into `.GOV`, into the ComfyUI engine-local output directory as authority, or to any machine-local absolute path.
- Each materialization is idempotent on content hash: re-delivery of the same output (e.g. on job retry) resolves to the existing `artifact_ref` rather than a duplicate, and the duplicate-suppression is recorded as evidence (Section 6.8.5).

---

### 6.9.5 EventLedger + Flight-Recorder evidence (Normative)

Every intake mutation emits durable evidence and a recoverable receipt:

- `intake.probe.recorded` -- on Section 6.8.2 probe (carries `probe_outcome`, `fallback_reason`).
- `intake.capability.registered` -- on Section 6.8.3 registration (carries `declared_outputs` count + reject count).
- `intake.capability.rejected` -- one per dropped declared output (carries `output_slot`, reason).
- `intake.output.materialized` -- one per Section 6.8.4 routed output (carries `artifact_ref`, `artifact_manifest_ref`, `routing_intent`).
- `intake.output.deduplicated` -- on idempotent re-delivery (carries existing `artifact_ref`).
- `intake.fallback.engaged` -- when routing degrades to SaveImage scan (Section 6.8.6).

Normative: each event is a Flight-Recorder span on the owning `engine.comfyui` job (consistent with Section 6.0 "Flight Recorder is always-on" and Section 9.12). Inputs/outputs are recorded by reference (`artifact_ref` / `artifact_manifest_ref`), never by inlining bytes. A `ComfyIntakeReceiptV1` is produced at job completion summarizing `{ workflow_run_id, probe_outcome, registered_output_count, materialized_artifact_refs[], fallback_engaged }`; the receipt is the recoverable artifact that lets a no-context model reconstruct exactly what intake produced for the job. Credential material is scrubbed from all of the above per LAW-COMFY-INTAKE-005.

---

### 6.9.6 SaveImage fallback boundary (Normative)

When `probe_outcome != bridge_present` (absent, incompatible, or probe timeout), intake routes to the no-bridge SaveImage fallback:

- The engine collects outputs from ComfyUI's `SaveImage`-produced files via a bounded post-execution scan scoped to the job's run, then materializes each scanned file through the same ArtifactStore + manifest path as Section 6.8.4, emitting `ComfyIntakeOutputRecordV1` rows with `source_output_slot = "saveimage_fallback"` and `registration_id = null`.
- `intake.fallback.engaged` is emitted with `fallback_reason` so the operator can see why the bridge path was not taken.
- The fallback NEVER treats the ComfyUI output directory as authority; files are copied into the ArtifactStore and the engine-local copies are not the source of truth.
- The fallback path is functionally output-complete (every produced image is still governed and materialized); it loses only the in-graph per-output provenance richness (`routing_intent`, sidecar binding, capability-time output declaration) that the bridge node supplies.

This makes the bridge node a provenance/quality upgrade over an always-available governed baseline, not a hard dependency.

---

### 6.9.7 Fake-adapter test contract (Normative)

A `ComfyBridgeFakeAdapterV1` provides deterministic, offline test coverage without a live ComfyUI process:

- The fake adapter implements the same probe + registration + output-delivery surface as the real bridge, driven by a declarative scenario (`probe_outcome`, `declared_outputs[]`, a fixed sequence of synthetic outputs with stable content hashes).
- The fake adapter MUST exercise all four probe outcomes (`bridge_present`, `bridge_absent`, `bridge_incompatible`, probe-timeout) and the capability-reject path.
- Fixtures, fake-adapter state, and materialized test artifacts use ArtifactStore + Postgres/EventLedger fixtures ONLY. SQLite is FORBIDDEN in the fake adapter and its fixtures (LAW-COMFY-INTAKE-004).
- The fake adapter runs non-intrusively (HBR-QUIET): no ComfyUI window, no foreground surface, no global shortcut hijack, no unbounded synthetic input. Synthetic output sequences are bounded by the scenario.
- Determinism: a given scenario reproduces identical `ComfyIntakeOutputRecordV1` content hashes and the same EventLedger event sequence across runs (modulo ids/timestamps), so the intake contract is replay-asserted independent of the stochastic generative engine.

**Cross-references:** Section 9.12 `engine.comfyui` (execution engine + STOCHASTIC determinism); Section 6.0 mechanical tool bus (no shadow pipelines, Flight-Recorder always-on, capability gates); Section 6.3.3 Creative Studio (operator surface); Section 2.6.6 AI Job Model; FEAT-PHOTO-STUDIO, FEAT-WORKFLOW-ENGINE, FEAT-AI-JOB-MODEL, FEAT-FLIGHT-RECORDER, FEAT-CAPABILITIES-CONSENT (Appendix 12.3); HBR-QUIET-001..004; [GLOBAL-PORTABILITY-004]..[GLOBAL-PORTABILITY-008].
