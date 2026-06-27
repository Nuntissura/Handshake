---
schema: handshake.indexed_spec.module@1
spec_version: "v02.196"
bundle_id: "master-spec-v02.196"
module_id: "10"
section_id: "10"
title: "10. Product Surfaces"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "1573148f4d958735d3ca2ac7a2d36c918763f123822dadf6576ee0f628d69a59"
body_sha256: "cb18fbf8eebfe5f1e661f7823095870ceae39ea1550432289a84621ecd2f6116"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 10. Product Surfaces

**Purpose**  
Dedicated home for user-facing application surfaces (developer tools and productivity apps) so requirements stay discoverable and versioned without renumbering earlier sections.
Note: the staging file `terminal_monaco_v02.18_staged.md` is non-canonical; this section is the source of truth.

## 10.1 Terminal Experience

Status: Exploratory (aligned with v02.18 + Mechanical Extension Engines v1.1); not yet implemented.

### 10.1.1 Security, Capabilities, and API (LAW)

#### 10.1.1.1 Security model: policy vs hard isolation
- **TERM-SEC-001 (Policy vs Hard Isolation)**
  - Handshake MUST distinguish between:
    - **Policy-scoped shells** (default): PTY spawned as the user, with workspace-scoped *policies* but no OS-level isolation.
    - **Sandboxed shells** (optional, â€œsecure modeâ€): PTY or shell running inside a container/VM/sandbox with OS-level isolation.
  - The spec MUST state that **only sandboxed shells provide strong containment** against filesystem and network access; policy-scoped shells are best-effort.

- **TERM-SEC-002 (Workspace policy scoping)**
  - For policy-scoped shells, Handshake MUST:
    - Enforce workspace-relative default `cwd`.
    - Apply policy checks **before** spawning commands (`run_command`) and **before** binding AI jobs to a session:
      - Check allowed directories,
      - Check allowed/denied command patterns.
    - Surface violations as explicit capability failures, not silent no-ops.

- **TERM-SEC-003 (Secure mode)**
  - Handshake SHOULD provide a â€œsecure shell modeâ€ where:
    - Shells run in a container/VM with:
      - Restricted filesystem (bind-mounted workspace only or ephemeral FS),
      - Optional network restrictions.
    - AI job terminals default to secure mode when available.

#### 10.1.1.2 Consent / capability UX and caching
- **TERM-CAP-001 (Consent granularity)**
  - Shell capabilities MUST be defined along these axes:
    - Model (which LLM),
    - Workspace,
    - Command class (e.g., â€œbuild/testâ€, â€œgitâ€, â€œarbitraryâ€),
    - Time scope (single job vs session vs persistent).

- **TERM-CAP-002 (Approval types)**
  - Handshake MUST support at least:
    - **Per-job approval**: user approves a set of commands for one AI job run.
    - **Per-model-per-workspace approval**: user approves a command class for a model in a workspace, cached until revoked or expired.
  - Approvals MUST be:
    - Visible in a â€œCapabilitiesâ€ UI.
    - Revocable without restarting the app.

- **TERM-CAP-003 (Escalations)**
  - When a model attempts a command outside its approved capability:
    - The run MUST be blocked.
    - The orchestrator MUST surface an escalation request to the user:
      - Show model, workspace, proposed command, reason.
      - Allow â€œapprove onceâ€, â€œapprove for jobâ€, â€œapprove for workspaceâ€, or â€œdenyâ€.
    - The decision MUST be logged in Flight Recorder for that job.

- **TERM-CAP-004 (Expiry)**
  - Long-lived approvals MUST have:
    - Either a time-to-live, or
    - An explicit â€œunlimited until revokedâ€ flag visible in the UI.

#### 10.1.1.3 `run_command` API contract
- **TERM-API-001 (Signature)**
  - The internal `run_command` tool MUST expose at least:
    - `command: string` (full command line or argv-list),
    - `cwd?: string` (workspace-relative path),
    - `mode: 'non_interactive' | 'interactive_session'`,
    - `timeout_ms?: number`,
    - `env_overrides?: Record<string, string | null>`,
    - `max_output_bytes?: number`,
    - `capture_stdout?: boolean`,
    - `capture_stderr?: boolean`,
    - `stdin_chunks?: string[]` (for scripted interactive runs),
    - `idempotency_key?: string` (optional for at-least-once semantics).

- **TERM-API-002 (Timeout & cancellation)**
  - If `timeout_ms` is omitted, a reasonable default MUST be used (recommended: 180_000 ms).
  - When timeout is reached:
    - The backend MUST send a termination signal to the process,
    - Then a kill signal if it doesnâ€™t exit within a grace period (recommended: 10_000 ms).
    - The result MUST include `timed_out: true`.
  - The orchestrator MUST be able to cancel an in-flight command:
    - Cancellation MUST propagate to the PTY,
    - Result MUST include `cancelled: true`.

- **TERM-API-003 (Output handling)**
  - For `non_interactive` mode:
    - Output MAY be streamed to the caller, but MUST be **bounded** by `max_output_bytes` (recommended default: 1â€“2 MB):
      - If truncated, the result MUST indicate truncation and how many bytes were emitted.
  - For `interactive_session`:
    - Output MUST be streamed, but the logging policy (below) still applies.
  - The API MUST separate:
    - Raw unredacted stream (for UI),
    - Redacted/logged stream (for Flight Recorder).

- **TERM-API-004 (Environment rules)**
  - Default environment MUST be inherited from the appâ€™s process (subject to secrets policy).
  - `env_overrides`:
    - `value` overrides or injects variables,
    - `null` explicitly unsets a variable.
  - Certain variables (e.g., secrets) MAY be suppressed from being passed into AI-run shells if policy dictates.

- **TERM-API-005 (Deterministic logging)**
  - Every `run_command` call MUST emit a log event containing:
    - `job_id` (if any),
    - `model_id` (if any),
    - `session_id` (if bound to a TerminalSession),
    - `command`, `cwd`, `exit_code`, `duration_ms`,
    - `timed_out`, `cancelled`, `truncated_bytes`.
  - This event MUST be stable enough for replay and auditing.

### 10.1.2 Logging, Matchers, UX, Platform (LAW)

#### 10.1.2.1 Output logging & redaction
- **TERM-LOG-001 (Logging levels)**
  - Terminal logging MUST have at least:
    - `NONE`: no output, commands only.
    - `COMMANDS_ONLY`: command line + metadata.
    - `COMMANDS_PLUS_REDACTED_OUTPUT`: output stored with redaction.
  - Default MUST be `COMMANDS_ONLY` for AI job terminals.
  - Recommended retention:
    - Commands-only: retained per Flight Recorder retention window.
    - Redacted output: 7 days by default; configurable.
    - Full-output logging is disabled for AI terminals in v1; human terminals may enable per-session with explicit warning.

- **TERM-LOG-002 (Redaction engine)**
  - For levels that log output, Handshake MUST run output through a redaction engine that:
    - Applies pattern-based redactions for:
      - Common token formats (e.g., cloud keys),
      - `=...` env-looking secrets (`API_KEY=`, `TOKEN=`, etc.),
      - Typical `.env`-style assignments.
    - Replaces matches with a placeholder (`***REDACTED***`).

- **TERM-LOG-003 (User warnings)**
  - When a user enables full output logging (even with redaction), Handshake MUST show:
    - A clear warning that some secrets may still leak into logs.
    - A pointer to the projectâ€™s log retention and export policy.

#### 10.1.2.2 Problem matchers
- **TERM-DIAG-001 (Matcher schema)**
  - A problem matcher MUST have at least:
    - `id: string`,
    - `pattern: regex` (or multiple patterns for multi-line),
    - `fileGroup`, `lineGroup`, `columnGroup`, `severityGroup?`, `codeGroup?`, `messageGroup`,
    - `appliesTo: 'stdout' | 'stderr' | 'both'`,
    - `source: 'builtin' | 'plugin:<id>' | 'workspace'`.

- **TERM-DIAG-002 (Storage)**
  - Built-in matchers: shipped with Handshake core.
  - Workspace matchers: stored in a workspace config file (e.g., `.handshake/diagnostics.json`).
  - Plugin matchers: declared in plugin manifests and merged at runtime.

- **TERM-DIAG-003 (Conflict resolution)**
  - If multiple matchers could apply:
    - Built-in matchers have lowest precedence,
    - Workspace matchers override built-ins,
    - Plugin matchers override both within their declared scope.

- **TERM-DIAG-004 (Diagnostics pipeline)**
  - Parsed output MUST be normalized into the common Diagnostics schema (see 11.4),
  - Then fed into:
    - Monaco,
    - Problems panel,
    - Flight Recorder (as `DiagnosticEvent`).

#### 10.1.2.3 Human vs AI terminal invariants
- **TERM-UX-001 (Session types)**
  - Every terminal session MUST be labeled as:
    - `HUMAN_DEV` (created by user),
    - `AI_JOB` (created by an AI job),
    - `PLUGIN_TOOL` (optional).
  - This type MUST be visible in the UI (badge, color, or icon).

- **TERM-UX-002 (AI attachment rule)**
  - AI models MUST NOT:
    - Attach to,
    - Type into,
    - Or read from `HUMAN_DEV` sessions by default.
  - Any override MUST be:
    - Explicitly requested,
    - Capability-guarded,
    - Confirmed by the user.

- **TERM-UX-003 (Trace linkage)**
  - Every `AI_JOB` session MUST be linked to:
    - A `job_id`,
    - One or more WSIDs,
    - Capability set,
  - And MUST be visible from Flight Recorder with a â€œjump to terminalâ€ link.

#### 10.1.2.4 Platform matrix
- **TERM-PLAT-001 (Baseline support)**
  - v1 MUST support policy-scoped terminals on:
    - Windows (ConPTY-based shells: PowerShell, CMD, WSL),
    - Linux (PTY-based shells: Bash, Zsh),
    - macOS (PTY-based shells: Bash, Zsh).

- **TERM-PLAT-002 (Guaranteed baseline features)**
  - Across all three platforms v1 MUST support:
    - Multiple sessions,
    - Tabs,
    - Workspace-root cwd,
    - Command logging (commands only),
    - File path linkification,
    - Non-interactive `run_command` with timeouts.

- **TERM-PLAT-003 (Advanced features MAY vary)**
  - Shell integration (decorations, cwd markers), splits, persistent sessions, and sandboxed shells MAY initially be limited to subsets of platforms; the spec MUST:
    - Document per-platform availability,
    - Keep the API behavior consistent (even if no-op).
  - Recommended defaults:
    - Policy mode is baseline everywhere.
    - Secure/sandboxed shells: supported on Linux/macOS (container/VM); on Windows use WSL/container when present, else policy-only.
    - AI job terminals SHOULD prefer secure mode when available; fallback to policy MUST show an explicit banner.

### 10.1.3 v1 Scope & Invariants (LAW)

- **TERM-V1-SCOPE**
  - MUST for v1:
    - Integrated terminal panel,
    - Multiple sessions with tabs,
    - `run_command` API with timeout, logging, and capabilities,
    - AI job terminals separate from human terminals,
    - File path linkification,
    - Policy-scoped security model plus clear secure-mode definition.
  - MAY for v1 or later:
    - Splits, persistent sessions, sandboxed terminals.

- **TERM-INVARIANTS**
  - AI command execution MUST be capability-checked and trace-linked.
  - AI MUST NOT type into human terminals by default.
  - Every AI-run command MUST appear in Flight Recorder.

### 10.1.4 Design Notes & Feature Map (Non-normative)

#### 10.1.4.1 What we want and why
- Integrated terminal in main UI (bottom panel), multiple terminals/tabs/splits; user shells (PowerShell, CMD, WSL, Bash, Zsh, etc.).
- Tight wiring to worksurfaces, Flight Recorder/tracing, capability system.
- Structured hooks for parsing output into diagnostics and attaching to Monaco/Problems.
- Human and AI share primitives with safety and observability; avoid context switching; controlled AI command execution; inherit project environment.
- VS Code-like mental model with stronger AI visibility/safety and richer linking to specs/logs.

#### 10.1.4.2 What not to do
- No silent arbitrary AI shell commands without explicit capability and per-job context.
- No opaque terminal blob disconnected from worksurfaces/jobs/models.
- No logging of all keystrokes/output by default; avoid secrets leakage and noise.
- Do not conflate human vs job terminals; avoid hidden terminal parsing for core semantics (prefer structured APIs where possible).

#### 10.1.4.3 Terminal feature set (exhaustive list)
- Core behaviour: embedded panel (tabs/splits), session persistence (optional), default cwd at workspace root, shell selection, environment inheritance, command palette entry.
- UX: path linkification, copy/paste, find, scrollback, color profiles, themes, keyboard shortcuts, status decorations, notifications on exit/fail/long-run, tab renaming.
- Sessions: multiple terminals, tabs, optional splits, rename, duplicate, persistent sessions (optional), restart/clear, session types (human vs AI vs plugin).
- AI job sessions: auto-created per job or pooled; bound to job_id/model_id/wsid; apply capabilities; read-only output vs interactive modes with approvals; cancel/stop; trace links.
- Capabilities: allowed directories, allowed/denied commands; logging policy; approval scope; escalation flows; expiry; revocation UI; per-model/workspace caching.
- Execution: `run_command` tool, timeouts, cancellation, max output bytes/truncation, stdin scripting, env overrides/unset, idempotency key; policy checks before spawn.
- Logging: command metadata always; output logging gated; redaction engine; logging levels; warnings on full output logging; retention policy (to be defined).
- Diagnostics: problem matchers (builtin/workspace/plugin), multi-line patterns, schema fields (file/line/col/code/severity/message/source/stdout|stderr), precedence rules; normalization to Diagnostics schema; routing to Monaco/Problems/Flight Recorder.
- Security: policy vs sandboxed shells; sandbox is only strong containment; workspace-scoped default cwd; platform matrix; secure mode for AI terminals when available.
- Integrations: Flight Recorder events for commands; link to jobs and worksurfaces; capability decisions logged; Problems panel; Monaco jump-to-location; plugin registration for matchers.
- Platform: Windows (ConPTY shells), macOS/Linux PTY; advanced features may vary; consistent API even when feature is no-op.

#### 10.1.4.4 Risks
- Weak isolation in policy mode; mitigate with clear secure-mode definition and capability gates.
- Secrets leakage via output logging; mitigate with redaction defaults and warnings.
- Platform variance (ConPTY vs PTY quirks); document per-OS behaviour and ensure consistent API semantics.
- Capability model ambiguity; define scopes/TTL/defaults and escalation UX.

#### 10.1.4.5 Technical fit in Handshake architecture
- Frontend: xterm.js (or equivalent) terminal component in Tauri/React; session types surfaced in UI.
- Backend: TerminalService with PTY abstraction; policy checks before spawn; `run_command` tool exposed to orchestrator; logging to Flight Recorder.
- Integration: capability system for AI jobs; diagnostics normalized to shared schema and routed to Monaco/Problems; Flight Recorder linkages; platform matrix for PTY/ConPTY/sandbox.

### 10.1.5 Review & Hardening Map (Non-normative)
- Blocking gaps resolved by TERM-SEC/CAP/API/LOG/DIAG/UX/PLAT items above:
  - Terminal security model conceptual â†’ clarified policy vs sandbox and only sandbox is strong isolation.
  - Consent/capability UX/persistence â†’ approval scopes, caching, revocation, escalation flows, expiry.
  - `run_command` contract â†’ signature, timeouts, cancellation, stdin, streaming vs bounded output, env rules, deterministic logging.
- Major improvements:
  - Output logging redaction policy (TERM-LOG).
  - Problem matcher format/ownership/conflict resolution (TERM-DIAG).
  - Human vs AI terminal interaction rules (TERM-UX).
  - Platform matrix baseline/variance (TERM-PLAT).
- Cross-cutting integration:
  - Diagnostics schema + Flight Recorder events (see 11.4/11.5) as concrete anchors.

## 10.2 Monaco Editor Experience

Status: Exploratory (aligned with v02.18); normative LAW sections drafted but not implemented.

### 10.2.1 IDs, AST, Bundling (LAW)

#### 10.2.1.1 ID lines + DocumentAST stability
- **EDIT-ID-001 (ID format)**
  - Block IDs MUST be:
    - Opaque strings, stable across sessions (e.g., `B-<base36>`),
    - Unique per worksurface,
    - Not semantically meaningful.

- **EDIT-ID-002 (Block vs line IDs)**
  - For document worksurfaces:
    - Primary ID is **block-level** (paragraph / heading / list item).
    - Line IDs MAY be derived for dense change tracking but are derived, not canonical.
  - AI patch APIs MUST address content by **block ID**, not by visual line numbers.

- **EDIT-ID-003 (Generation)**
  - IDs MUST be assigned:
    - On initial import (DOCX â†’ AST),
    - When creating new blocks,
    - When splitting a block:
      - New block gets a new ID,
      - Original retains its ID.

- **EDIT-ID-004 (Merges)**
  - When merging blocks:
    - The resulting block MUST keep one of the original block IDs (e.g., left-most),
    - Other IDs MUST be retired (but MAY be kept in change history).

- **EDIT-ID-005 (Stability under edits)**
  - Edits to text within a block MUST NOT change its ID.
  - Reordering blocks MUST NOT change their IDs.

- **EDIT-ID-006 (AI patch contract)**
  - AI editing APIs MUST:
    - Accept block IDs and new text content as primary operands,
    - Reject operations referencing unknown IDs,
    - Emit clear errors when an ID was retired so the orchestrator can refresh context.

#### 10.2.1.2 AST canonical invariant
- **EDIT-AST-001 (Canonical source of truth)**
  - For **document** worksurfaces:
    - The `DocumentAST` MUST be the canonical representation.
    - No view (Monaco or RichView) may write directly to disk; they MUST update AST via well-defined operations.
  - For **code** worksurfaces:
    - The file text (as held by the file model) is canonical; Monaco is a view and editor over that text.

- **EDIT-AST-002 (View projections)**
  - `RichView` MUST render from AST.
  - `Monaco StructuredTextView` MUST render a textual projection of AST (with or without ID column).
  - AST changes MUST propagate to all open views for that worksurface.

- **EDIT-AST-003 (Persistence)**
  - On save:
    - Document worksurfaces MUST serialize AST to the on-disk representation.
    - DOCX export/import MUST go via AST, never via ad-hoc parsing of Monaco text.

#### 10.2.1.3 Monaco bundling
- **EDIT-BND-001 (Initial language set)**
  - v1 MUST pre-bundle at least:
    - TypeScript/JavaScript,
    - JSON,
    - HTML/CSS,
    - Rust,
    - Markdown,
    - Handshake-specific language (via Monarch grammar).

- **EDIT-BND-002 (Lazy loading)**
  - Monaco core and languages SHOULD be lazy-loaded:
    - Only when opening an editor that needs Monaco.
    - Using dynamic imports or Viteâ€™s code-splitting.

- **EDIT-BND-003 (Workers)**
  - The Tauri/Vite config MUST:
    - Correctly route Monaco web workers,
    - Package fonts/TTFs,
    - Avoid network fetches at runtime for Monaco assets.

### 10.2.2 Diagnostics & Flight Recorder (LAW)

#### 10.2.2.1 Diagnostics schema (canonical in 11.4)
- **DIAG-SCHEMA-001 (Diagnostic shape)**  
  (See 11.4 for canonical schema; referenced here for Monaco consumption.)
- **DIAG-SCHEMA-002 (Routing)**  
  Diagnostics in this shape MUST be:
  - Pushed into Monaco,
  - Listed in Problems view,
  - Optionally stored as `DiagnosticEvent` in Flight Recorder.

#### 10.2.2.2 Flight Recorder event contracts (canonical in 11.5)
- **FR-EVT-001 (TerminalCommandEvent)**  
  (See 11.5; referenced for Monaco jump links.)
- **FR-EVT-002 (EditorEditEvent)**  
  Used for Monaco/Rich edits; see 11.5 for full shape.
- **FR-EVT-003 (DiagnosticEvent)**  
  Used for Problems/diagnostics; see 11.5 for full shape.
- **FR-EVT-004 (Retention & linkability)**  
  Retention window and navigation guarantees captured in 11.5.

### 10.2.3 v1 Scope & Invariants (LAW)

- **EDIT-V1-SCOPE**
  - MUST for v1:
    - Monaco as primary editor for code and at least one structured-text type,
    - Diagnostics pipeline wired into Monaco,
    - Basic `DocumentAST` + block ID scheme for one document type,
    - AST canonical invariant for that document type.
  - MAY for v1:
    - Full DOCX import/export,
    - Rich/Monaco dual view for all document types,
    - Advanced refactorings via LSP.

- **EDIT-INVARIANTS**
  - For docs: AST is canonical; Monaco and Rich are projections.
  - AI edits go through ID-based or range-based structured APIs, not raw keystrokes.
  - All AI edits are traceable via EditorEditEvents.

### 10.2.4 Design Notes & Feature Map (Non-normative)

#### 10.2.4.1 What we want and why
- Monaco as primary code and structured-text editor; supports code (Rust, TS/TSX, etc.), specs/logs/config, and structured document projections with ID lines.
- Coexists with rich-text â€œWord-likeâ€ view; both bound to same content.
- Deep hooks: diagnostics, navigation, AI edits (block/range with IDs), Flight Recorder integration.
- Rationale: Monaco is VS Codeâ€™s engine; extensible (languages, Monarch grammars, LSP), gives â€œVS Code inside Handshakeâ€ feel with deterministic AI editing via IDs/AST.

#### 10.2.4.2 What not to do
- Donâ€™t treat Monaco as raw `<textarea>`; leverage diagnostics/navigation/LSP.
- Donâ€™t let AI free-edit massive blobs without IDs/ranges; keep determinism/replay.
- Donâ€™t couple Monaco models directly to disk; AST/file model owns canonical data.
- Donâ€™t overload Monaco with heavy DOM/HTML rendering.

#### 10.2.4.3 Monaco feature set (exhaustive list)
- Core editing: insert/delete, undo/redo, clipboard; syntax highlighting, folding, indentation guides, bracket matching, word wrap.
- Cursors/selection: multi-cursor, column/block selection.
- Search/replace: regex, replace-all.
- Language support: built-in TS/JS/JSON/CSS/HTML; custom Monarch grammars (Handshake spec/log formats); LSP for Rust/TS/etc via bridge.
- Diagnostics/Problems: squiggles, gutter icons, hovers; central Problems view; sources from LSP, terminal matchers, validators.
- Navigation/refactoring: go to def/type/ref; go to symbol; rename; basic code actions; document outline.
- Theming/layout: Monaco theme aligned with Handshake; optional VS Code theme JSON support; tabbed editors; diff view; font/ligatures; minimap; gutter with line numbers and ID column.
- Structured docs & ID lines: Monaco projection of `DocumentAST`; toggleable ID column; structural validation/markers.
- Rich text interplay: dual view (RichView + Monaco StructuredTextView); both bound to AST; DOCX import/export via AST.
- AI integration: structured edit API (`apply_patch`, `replace_block`); Monaco updated through AST/patch; AI change highlights; diff preview; revert AI edits.
- Observability/versioning: local history snapshots; Flight Recorder logging of edits (before/after hashes, ranges, model/job); diff viewer for job changes.
- UX conveniences: snippets; completions for spec IDs; global search opening in Monaco.

#### 10.2.4.4 Risks
- Bundle size/performance; mitigate with lazy-loading and focused language set.
- Bundling complexity (workers/fonts/CSS) in Tauri+Vite; use known patterns.
- Custom language maintenance; keep grammars small, test highlighting/structure.
- AST sync bugs between Monaco/Rich/AST; enforce single source of truth and explicit change paths.
- Concurrent edits (multi-model/human+AI); use queued writes or per-job branches, preview before apply.
- Security: treat text as text; avoid evaluating code within editor.

#### 10.2.4.5 Technical fit in Handshake architecture
- Frontend: `monaco-editor` with React wrapper; component props include `wsid`, viewType, language, value, onChange; attach diagnostics/decorations/ID gutter.
- Worksurfaces/AST: `Worksurface` model with `wsid`, type, `DocumentAST` or text; docs use AST as canonical; code uses file text as canonical.
- Diagnostics pipeline: inputs from LSP, terminal matchers, validators; normalized shape (11.4); routed to Monaco/Problems/Flight Recorder.
- Navigation/cross-linking: LSP bridge for definitions/refs/rename; Handshake-specific language service for RID/LAW/WSID; terminal â†” Monaco via file/line parsing; Flight Recorder â†” Monaco via jump links.
- AI integration/capabilities: structured edit operations via orchestrator backed by Monaco/file model; per-model rights (read/write WSIDs, edit bounds); logging for every AI edit with diffs/hashes/job/model.
- Extension fit: plugins register diagnostics providers, code actions, snippets, custom languages via API; isolation via editor service API (no direct Monaco manipulation).

### 10.2.5 Review & Hardening Map (Non-normative)
- Monaco-specific notes resolved by EDIT-ID/AST/BND + EDIT-INVARIANTS:
  - ID stability rules for block vs line IDs; generation/split/merge/reorder invariants.
  - Dual-view sync invariant: AST is canonical; all views are projections; no direct disk writes.
  - Bundling plan: worker routing, lazy-load, initial language set.
- Cross-cutting integration (with 11.4/11.5):
  - Normalized diagnostics schema and event contracts as concrete anchors for Problems/Flight Recorder.

### 10.2.6 Document AI Actions (LAW)

**Status:** v0.1 block-level implementation delivered (Phase 1).

- **DOC-AI-001 (The Rewrite Loop)**
  - **Invariant:** AI MUST NOT silently mutate RawContent.
  - **v0.1 Implementation Detail:** Edits currently operate at the **Block Level** (replacing a whole block identified by ID) rather than arbitrary text spans.
  - Workflow:
    1. **Trigger**: User selects a block + invokes `DOC_REWRITE`.
    2. **Gate**: Request flows through MCP Gate; checked against `doc.write` capability.
    3. **Generation**: AI Job produces a `ChangeProposal` containing original and proposed text.
    4. **Review**: UI displays "Diff" (Old vs New).
    5. **Decision**: User clicks "Accept" (applies block-level patch) or "Reject" (logs `rejected_idea`).
  - **Logging (Flight Recorder Events)**:
    - `rewrite_proposal`: Emitted during Generation. Includes: `job_id`, `block_id`, `proposal_hash` (SHA-256).
    - `rewrite_decision`: Emitted during Decision. Includes: `job_id`, `decision` (`accept` | `reject`), `block_id`.

- **DOC-AI-002 (Variations)**
  - The `DOC_REWRITE` job SHOULD support producing multiple `ChangeProposal` variants (e.g., "Concise", "Professional", "Creative") in a single pass if requested.
  - UI MUST allow cycling through variants before Accepting.

- **DOC-AI-003 (Rejected Ideas)**
  - Rejected proposals MUST be logged to Flight Recorder (tagged `rejected_idea`) to preserve "lost" work for potential future retrieval.

## 10.3 Mail Client

Status: v0.5 research/design; not implemented. Behaviours require capability/consent gates and workflow enforcement before shipping.

 Mail as a First-Class, Classified Domain in the Shadow Workspace

Version: 0.5 (research / design)  
Status: Exploratory, aligned with Master Spec v02.18 + Mechanical Extension Engines v1.1  

---

### 10.3.1 Motivation and Goals

Email is one of the highest-signal, highest-friction streams in a knowledge workerâ€™s life:

- It contains requirements, commitments, contracts, decisions.  
- It carries attachments that are often the *real* payload (contracts, invoices, reports, exports, recordings).  
- It is a rich signal source for personal analytics (time, relationships, obligations, sentiment).

In most systems, mail lives in a siloed client (Gmail/Outlook/Thunderbird); AI is bolted on as â€œsmart replyâ€ or â€œsummarize this threadâ€.

In Handshake, the goals are:

1. Treat mail as a first-class **RawContent** domain, equal to docs, code, and transcripts.  
2. Run mail through the **Mechanical Extension Engines** so email + attachments become structured, queryable, auditable artifacts.  
3. Let **local LLMs and cloud LLMs** act on mail via the **AI Job Model** and **Workflow Engine**, never by scraping the GUI.  
4. Use a **classification-based governance model** (PUBLIC â†’ HIGHLY_RESTRICTED) to protect trade secrets, financials, legal mail, and personal data.  
5. Isolate **mail communication descriptors** from **art / creative / NSFW descriptor domains**, so learning signals donâ€™t contaminate each other.  
6. Expose everything through a **single cohesive GUI** (Handshake), even while external mail clients temporarily coexist.  

---

### 10.3.2 Architectural Context

We stay within the existing four-layer architecture (Mechanical Engines Â§6.3.1):

1. **Brain (LLM)**  
   - Plans tasks, emits `PlannedOperation` JSON, consumes structured inputs (blocks, tables, descriptors).  

2. **Gate**  
   - Validates planned operations for safety and policy compliance (capabilities, classification routing, resource limits).  

3. **Engines (Mechanical Tools)**  
   - Deterministic, inspectable tools (Docling, Unstructured, DuckDB, ffmpeg, Whisper, TXT-001, Wrangler, DBA, Indexer, Archivist, etc.).  

4. **Logs (Flight Recorder)**  
   - Every call and decision is persisted with provenance: inputs, outputs, engines, classification decisions, errors.

Other key layers:

- **Content stack:** `RawContent â†’ DerivedContent â†’ DisplayContent`  
- **Shadow Workspace:** CRDT/Yjs + index layer hosting blocks from all sources  
- **Knowledge Graph:** entities (persons, projects, docs, mail threads) and edges  
- **Workflow Engine:** triggers + nodes (AI, connectors, mechanical engines)  
- **Mechanical Extension Engines:** a catalogue of engines across media, data, language, analytics, infra  

Mail must plug into these layers rather than bypass them.

---

### 10.3.3 Mail as First-Class Content

#### 10.3.3.1 Content types

Mail is modeled as canonical external content.

**MailMessage (RawContent)**

- Identity:
  - `internal_message_id` â€“ Handshakeâ€™s stable primary key for this message  
  - `rfc822_message_id` â€“ value from the `Message-ID` header (if present)  
  - `provider_message_id` â€“ provider-specific ID (e.g. Gmail `id`)  
  - `account_id` â€“ referencing the configured mail account  
  - `imap_uid` (optional) â€“ UID within an IMAP folder  
  - `imap_modseq` (optional) â€“ last known MODSEQ for concurrency  

- Headers:
  - `from` (address, display name)  
  - `to[]`, `cc[]`, `bcc[]`  
  - `subject`  
  - `date` (as reported by provider, plus normalised timestamp)  
  - `in_reply_to`  
  - `references[]`  

- Bodies:
  - `body_text` (canonical text rendering)  
  - `body_html` (canonical HTML rendering, if any)  

- Organisation:
  - `folder_ids[]` / `label_ids[]` (normalised across providers)  
  - `flags` (seen, answered, flagged, draft, deleted, etc.)  

- Attachments:
  - `attachment_refs[]` (paths / content IDs into the attachment store)  

- Classification / tags (see Â§7):
  - `classification_level` (PUBLIC â†’ HIGHLY_RESTRICTED)  
  - `classification_tags[]` (PERSONAL_DATA, LEGAL, FINANCIAL, SECURITY, etc.)

**MailThread**

- `thread_id` (Handshake-level identifier; see Â§10.3.3.4)  
- `account_id`  
- `message_ids[]` (ordered)  
- `thread_key` (stable key derived from provider conversation id or RFC headers)  
- Participants (derived from headers)  
- Last activity timestamp  
- Thread-level `classification_level` and `classification_tags[]` (rollup; see Â§7.4)  
- Derived tags:
  - `project_ids[]`  
  - `client_ids[]`  
  - `topic_cluster_ids[]`  

#### 10.3.3.2 Connectors and capability contracts

Connectors:

- IMAP / JMAP / Gmail (and later others) feed `MailMessage` + attachments into the mail store.  
- External GUI clients (Gmail web, Thunderbird, mobile apps) can continue to operate; Handshake is an additional client.

Capabilities:

- `READ_EMAIL(account_id, folder_scope, duration)`  
  - `folder_scope` maps Handshakeâ€™s logical folders/labels to provider structures.  
  - `duration` (e.g. 8h) bounds how long agents may access mail without renewed consent.  

- `SEND_EMAIL(account_id, from_identity, max_rate, require_confirmation)`  
  - `from_identity` restricts which From/alias values are allowed.  
  - `max_rate` is a rate limit (e.g. N emails per hour) for automated workflows.  
  - `require_confirmation = true` is a global invariant for user-facing assistants:
    - No AI workflow may send email without explicit human confirmation of a specific draft.

---

#### 10.3.3.3 Identifier taxonomy and idempotency

To avoid ambiguity and duplication:

- `internal_message_id` is the primary key inside Handshake.  
- `rfc822_message_id` is used for de-duplication across imports and providers.  
- `provider_message_id` is used to reconcile with provider APIs (Gmail/Graph).  
- `imap_uid` / `imap_modseq` are used for folder-level sync and conflict detection.  

All ingest and sync operations must be idempotent:

- For any raw message blob (EML / provider payload), Handshake computes:
  - `raw_hash` (cryptographic hash of the canonical raw payload).  
- If a message with the same `(account_id, raw_hash)` and `rfc822_message_id` already exists, ingestion becomes an update rather than a new insert.

---

#### 10.3.3.4 Threading model and sync semantics

##### 10.3.3.4.1 Thread construction

Threading precedence:

1. If the provider exposes a conversation/thread ID (e.g. Gmail thread id), that becomes the primary `thread_key`.  
2. Otherwise, Handshake uses RFC-5322 headers:
   - `References[]` + `In-Reply-To` to construct a conversation tree.  
3. As a last fallback, subject + heuristics (subject â€œstrippingâ€ rules) are used, but **never** without marking such threads as heuristic-constructed.

Thread merge rules:

- If multiple messages with different provider thread IDs share an RFC conversation chain, Handshake:
  - Treats provider thread IDs as hints;  
  - Builds a single logical thread keyed by the RFC chain;  
  - Persists provider thread IDs as attributes for debugging.

##### 10.3.3.4.2 Thread classification rollup

Thread-level classification is computed as:

- `thread.classification_level = max(classification_level(message_i))` across all messages in the thread.  
- `thread.classification_tags = union(classification_tags(message_i))`.

Re-computation:

- When a messageâ€™s classification changes, the threadâ€™s classification is recomputed deterministically.  
- Downgrading a threadâ€™s classification is only allowed:
  - Via explicit user action,  
  - With justification recorded in the Flight Recorder.

##### 10.3.3.4.3 Sync semantics and state reconciliation

For each `(account_id, folder_id/label_id)`:

- Handshake tracks:
  - Last known `highest_uid`, `highest_modseq`.  
  - Mapping of messages to folders/labels.

Sync rules:

- New messages:
  - Discover via IMAP/JMAP confidence mechanisms and provider listing.  
- Deletions:
  - If provider marks a message as deleted, Handshake marks the corresponding `MailMessage` as deleted (soft-delete) and removes it from active folder membership.  
- Moves / relabeling:
  - Provider folder/label changes are reflected in `folder_ids[]/label_ids[]`.  
- Flags:
  - `seen`, `answered`, `flagged`, etc. are reconciled as:
    - Provider is source of truth; local changes are pushed back when possible.  
- Multi-client:
  - Handshake does not attempt conflict resolution beyond mirroring provider state for canonical flags and folder membership.  
  - Local-only metadata (classification, descriptors, graph links) stays in Handshake and never travels back to the provider.

All sync operations must be idempotent and logged, with clear handling of partial failures and retry.

---

### 10.3.4 Mechanical Ingestion Path for Mail

Mail flows through the same mechanical pipeline as PDFs and docs:

```text
Mail Connector (IMAP / JMAP / Gmail)
    â”‚  (EML / MSG / MBOX + attachments)
    â–¼
[Format Detection + Routing]
    â”‚
    â”œâ”€ Unstructured / Tika â†’ email body + headers
    â”œâ”€ Docling â†’ attachment content (PDF / Office / HTML / CSV / etc.)
    â”œâ”€ OCR â†’ scanned PDFs / images
    â””â”€ ASR â†’ audio / video attachments
    â–¼
Block Transformer
    â–¼
Yjs / CRDT (Shadow Workspace) + Knowledge Graph
    â–¼
Indexers (lexical, vector, faceted, hybrid)
```

#### 10.3.4.1 Body parsing: Unstructured / Tika

Email containers (.eml, .msg, mbox) are parsed by **Unstructured** and/or **Tika**:

- Extract:
  - Canonical `body_text`  
  - Canonical `body_html`  
  - Inline quotes and reply separators  
  - Signature blocks where detectable  
  - Multipart/alternative semantics:
    - One primary body is chosen (e.g. text/html as canonical, text/plain retained for reference).

#### 10.3.4.2 Attachment processing: Docling, OCR, ASR

Attachments are routed by MIME type:

- **Docling**
  - PDFs, DOCX, PPTX, XLSX, HTML, Markdown, EPUB, CSV/JSON/XML, ZIP containers  
  - Produces logical blocks: headings, paragraphs, tables, figures  

- **OCR** (e.g. Tesseract / EasyOCR)
  - Scanned PDFs and images (contracts, scanned forms, screenshots with text)  

- **ASR** (Whisper / equivalent)
  - Voicemails, voice notes, meeting recordings sent via mail  

Each attachment becomes:

- A DerivedContent doc/table/transcript linked back to its `MailMessage`  
- A block set injected into the Shadow Workspace and indexed

#### 10.3.4.3 Determinism, versioning, and re-ingestion

To support reproducibility and audit:

- Every ingestion step records:
  - Engine name (Docling/Unstructured/Tika/OCR/ASR)  
  - Engine version (pinned)  
  - Engine configuration (relevant flags)  

- For each raw artifact (message body or attachment), Handshake records:
  - `raw_hash` (hash of canonical raw payload)  
  - `derived_hash` (hash of canonical block representation)  

Re-ingestion rules:

- Re-running an ingestion engine with the same version and config over the same `raw_hash` must be idempotent (same `derived_hash`).  
- When upgrading an engine version:
  - Re-ingestion may produce different blocks, but the change must be logged as a migration in the Flight Recorder.

---

### 10.3.5 Descriptor Domains and TXT-001 for Mail

TXT-001 is the generic text descriptor engine. It must enrich mail without polluting descriptor corpora used for creative / art / NSFW domains.

#### 10.3.5.1 Orthogonal axes: descriptor domain vs classification

Two completely separate axes:

- `descriptor_domain` (semantic domain of the text, for corpora separation), e.g.:
  - `MAIL_COMMUNICATION`  
  - `ART_VISUAL`  
  - `NSFW_SCENE` (optional)  
  - `GENERIC_TEXT`  

- `classification_level` / `classification_tags[]` (confidentiality and risk), e.g.:
  - PUBLIC â†’ HIGHLY_RESTRICTED, plus tags like PERSONAL_DATA, LEGAL, FINANCIAL, SECURITY.

**Routing and data exposure are governed solely by classification.**  
**Corpus/training separation is governed by descriptor domains.**  

The spec must never rely on descriptor domains (e.g. NSFW_SCENE vs MAIL_COMMUNICATION) to decide what may be sent to which destination.

#### 10.3.5.2 Mail-specific TXT-001 profile

`txt001_mail_profile` fields:

- Participants:
  - `sender_role` (client, internal, vendor, unknown)  
  - `recipient_roles[]`  
  - `relationship_state` (prospect, active_client, ex_client, partner, internal_only)  

- Communication properties:
  - `tone` (formal, informal, neutral, hostile, enthusiastic, etc.)  
  - `emotion` (calm, stressed, annoyed, hopeful, etc.)  
  - `urgency` (low, medium, high, immediate)  
  - `certainty` (speculative, tentative, committed)  

- Conversation structure:
  - `topic_cluster_ids[]` (link to graph topics)  
  - `decision_points[]`  
  - `commitments[]`  
  - `open_questions[]`  

- Action extraction:
  - `action_items[]` with:
    - `assignee` (person/entity)  
    - `due_date` (explicit/inferred)  
    - `confidence`  

TXT-001 runs in mail mode:

- Always emits `descriptor_domain = MAIL_COMMUNICATION`.  
- Never emits art/visual/scene-specific fields or domains.

#### 10.3.5.3 Classification and descriptor propagation

Propagation rules:

- Derived artifacts (attachment-derived docs, descriptors, summaries) inherit:
  - `classification_level = max(parent.classification_level)` across all parents.  
  - `classification_tags = union(parent.classification_tags)`.

- A human may manually **downgrade** classification only if:
  - They explicitly confirm the downgrade.  
  - The action is recorded in Flight Recorder, including justification.

Descriptor rows must carry both:

- `descriptor_domain`  
- `classification_level` / `classification_tags[]`  

This enables routing decisions for LLMs and connectors downstream, even when the descriptor is the only intermediate artifact being processed.

#### 10.3.5.4 Librarian and Curator enforcement

- **Librarian (`taxonomy_management`)**
  - Enforces that each descriptor corpus declares allowed `descriptor_domain` values.  
  - Rejects mixed-domain corpora when they claim to be â€œmail-onlyâ€ or â€œart-onlyâ€.

- **Curator (`curation`)**
  - Maintains distinct corpora:
    - â€œMail style corpusâ€  
    - â€œArt taste corpusâ€  
    - â€œScene corpusâ€ (if used)  
  - Prevents accidental merging unless explicitly requested.

---

### 10.3.6 Mechanical Engines Exploited for Mail

#### 10.3.6.1 Analyst and Chronicle (Email/Tasks/Time, Life Logging)

- **Analyst (`personal_analytics`)**
  - Analytics over:
    - Time use (time-of-day/week email patterns)  
    - Mail state (unread, overdue replies)  
    - Task completion derived from mail descriptors  
  - Typical queries:
    - â€œSummarize unread mail from sender X.â€  
    - â€œAverage response time per client in last 30 days.â€  
    - â€œMail volume by topic/tag.â€

- **Chronicle (`life_log`)**
  - Writes key events to a life log based on mail + attachments:
    - â€œSigned contract with client X.â€  
    - â€œLaunched campaign Y.â€  
    - â€œResolved dispute Z.â€

Outputs feed weekly reviews, OKR tracking, and retrospective docs.

#### 10.3.6.2 Librarian and Archivist (Taxonomy and Archival)

- **Librarian (`taxonomy_management`)**
  - Enforces tag schemas on mail threads:
    - `project_id`, `client_id`, `content_type`, `legal_critical`, etc.  
  - Detects â€œunclassifiedâ€ threads for manual triage.

- **Archivist (`archive_management`)**
  - Freezes critical threads:
    - Final contracts  
    - Approvals  
    - Risk decisions  
  - Mirrors them into immutable/WORM archives when configured.

Mail gains long-term governance beyond ad-hoc labels.

#### 10.3.6.3 Wrangler, DBA, Indexer, Inspector, Sync (Data & Infra)

- **Wrangler (`data_wrangling`)**
  - Cleans CSV/XLSX attachments:
    - Invoices, exports, financial reports.  
  - Produces typed, normalised tables.

- **DBA (`warehouse_query`)**
  - Runs queries across:
    - Attachment-derived tables  
    - Mail metadata tables  

- **Indexer (`index_build`)**
  - Builds lexical, faceted, and hybrid search indexes over:
    - Mail bodies  
    - Attachment texts  
    - Transcripts  

- **Inspector (`data_audit`)**
  - Verifies ingestion completeness:
    - IMAP UID ranges covered  
    - No unexplained gaps in a mailbox export  

- **Sync (`sync_management`)**
  - Manages replication/backups of the mail store.

#### 10.3.6.4 Language Engines (Polyglot, Red Pen, Detector, Anonymizer, Sentiment, Morphologist, Converter)

- **Detector (`language_detection`)**
  - Identifies language per mail segment; drives translation and model routing.

- **Polyglot (`offline_translation`)**
  - Local translation for incoming/outgoing mail.  
  - Options for storing dual-language views.

- **Red Pen (`grammar_style_check`)**
  - Deterministic grammar/style checks on drafts.  
  - Enforces per-client style rules.

- **Anonymizer (`pii_scrub`)**
  - Strips PII before:
    - Exporting mail content outside Handshake.  
    - Feeding corpora into generic/cloud models.

- **Sentiment (`sentiment`)**
  - Versioned sentiment scoring:
    - Track conversation tone over time.  
    - Support alerts (â€œclient sentiment droppingâ€).

- **Morphologist + Converter**
  - Normalise tokens/encodings for multi-language mail.  
  - Improve search, descriptors, analytics.

#### 10.3.6.5 Creative Studio: Publisher, Artist, Director, Composer

- **Publisher (`doc_layout`)**
  - Turn key threads into formatted reports:
    - â€œClient X negotiation history Q1â€ as a shareable PDF.

- **Artist / Director / Composer**
  - Generate visual/audio assets for campaigns:
    - Reusable logos, infographics, short promo video/audio snippets.  
  - Assets remain DerivedContent with known provenance.

---

### 10.3.7 Classification-Based Governance (Corporate Secrets, PII, etc.)

We replace any â€œNSFW vs cleanâ€ concept with a classification axis for confidentiality and risk.

#### 10.3.7.1 Classification levels and tags

`classification_level` (example ladder):

- `PUBLIC` â€“ safe for external sharing/publishing.  
- `INTERNAL` â€“ routine internal communication.  
- `CONFIDENTIAL` â€“ contracts, sensitive internal plans.  
- `SECRET` â€“ trade secrets, product roadmaps, M&A, privileged legal.  
- `HIGHLY_RESTRICTED` â€“ data that should never leave device (raw user dumps, security keys, extremely sensitive legal).

`classification_tags[]` (orthogonal flags):

- `PERSONAL_DATA` â€“ PII present.  
- `LEGAL` â€“ legal agreements or litigation.  
- `FINANCIAL` â€“ sensitive financials.  
- `SECURITY` â€“ keys, passwords, security reports.  
- others as needed.

#### 10.3.7.2 Classification inference and safe bootstrapping

Classification must not leak sensitive mail to cloud before classification is known.

Rules:

1. First-pass classification is performed by:
   - Deterministic heuristics (e.g., path, folder, sender).  
   - Mechanical / **local** models only (no cloud).  

2. Only once an artifact is classified as `PUBLIC` or `INTERNAL` (and passes policy) may **cloud-assisted classification** or tagging run on it.

This ensures no unclassified mail is ever sent to cloud models.

#### 10.3.7.3 Policy and routing engine

Policies are defined per workspace and per destination (model runtime / connector):

- Maximum allowed `classification_level`.  
- Allowed / disallowed `classification_tags[]`.  
- Required redaction strategies for certain tags.

Examples:

- Cloud LLM A:
  - Allowed: `PUBLIC`, `INTERNAL` with no `SECURITY` tags.  
  - Disallowed: `CONFIDENTIAL+` and any `SECURITY` content.

- Cloud LLM B (enterprise tenant):
  - Allowed: up to `CONFIDENTIAL` without `SECURITY` tags.  

- Local LLM:
  - Allowed: all levels; classification still logged but not used to block.

- External connector (3rd-party analytics):
  - Allowed: only `PUBLIC`, no `PERSONAL_DATA` or `SECURITY`.

Routing behaviour for each call:

1. Gather all input artifacts (mails, attachments, descriptors, transcripts).  
2. Inspect `classification_level` + `classification_tags[]`.  
3. Evaluate policy for the destination.  
4. If allowed:
   - Optionally construct a redacted `DisplayContent` view:
     - e.g. mask PII when `PERSONAL_DATA` is present.  
   - Call the destination with this view.  
5. If not allowed:
   - Block and record a denial in Flight Recorder (artifact IDs, policy, reason).

#### 10.3.7.4 Propagation rules

- Derived artifacts inherit:
  - `classification_level = max(parent.level)`  
  - `classification_tags = union(parent.tags)`  

- Manual downgrades require:
  - Explicit user action.  
  - Logging in Flight Recorder with justification.  

This ensures that summaries, descriptors, and exports are never â€œless classifiedâ€ than their sources without an intentional override.

---

### 10.3.8 Mail Enriches the Rest of Handshake

#### 10.3.8.1 Continuous, high-value input

Mail becomes a primary structured feed:

- Requirements and decisions  
- Negotiations and approvals  
- Attachments: contracts, invoices, reports, recordings  

These drive:

- Project worksurfaces (with linked threads).  
- Finance and analytics tables (via Wrangler/DBA).  
- Life logging and retrospectives (Chronicle).

#### 10.3.8.2 Social / collaborator graph

From headers and threads:

- Person entities and roles  
- Edges:
  - who talks to whom  
  - about which topics/projects  
  - with what sentiment trends  

This boosts:

- Relevance of AI suggestions  
- Triage (VIPs, stakeholders, etc.)  
- Taste modelling per contact or account

#### 10.3.8.3 Calendar and tasks

Using TXT-001 mail descriptors + workflow nodes:

- Extract events:
  - dates, times, locations â†’ **draft** calendar events.  
- Extract tasks:
  - action items with assignees and due dates â†’ **draft** tasks in local tables.

External sync:

- Draft events/tasks become authoritative calendar objects or tasks in external systems **only** after explicit user confirmation.

#### 10.3.8.4 Stronger RAG and research

Mail + attachments live in the same Shadow Workspace:

- Questions like:
  - â€œWhat did the client say about requirement Y?â€  
  - â€œSummarise all negotiations around contract Z.â€  
- Pull from:
  - Mail threads  
  - Parsed attachments  
  - Meeting notes and transcripts  

Retrieval is not constrained to â€œmail searchâ€; all content sits in one substrate.

---

### 10.3.9 AI Jobs, Workflow DSL, and Mail

#### 10.3.9.1 Core mail AI job profiles

Representative jobs:

- `mail_summarize_thread_v0.1`
  - Input: `thread_id`, options (scope, language, tone).  
  - Output: summary blocks, `decision_points[]`, `action_items[]`.  

- `mail_triage_inbox_v0.1`
  - Input: list of `thread_id`s (latest N, or filtered).  
  - Output: labels (important/later/newsletter/suspect) + suggested actions.

- `mail_draft_reply_v0.1`
  - Input: `thread_id`, instructions.  
  - Output: `DraftReply` (subject/body) as DerivedContent; never auto-sent.

- `mail_thread_to_doc_v0.1`
  - Input: `thread_id`.  
  - Output: doc summarising history, decisions, and tasks.

- `mail_analytics_brief_v0.1`
  - Input: timeframe, labels, client filters.  
  - Output: narrative summary based on Analyst metrics.

State-mutating behaviour:

- Only mechanical engines (e.g. DBA, email_send, calendar_sync) may write to authoritative tables or external systems.  
- LLM jobs output structured proposals that must pass through deterministic transformers/validators before applying side effects.

#### 10.3.9.2 Workflow trigger DSL (sketch)

Triggers use a simple, explicit DSL (to be formalised in the main Workflow spec), e.g.:

```text
on email_received
where sender.domain in ["vendor.com", "billing.partner.com"]
  and subject ~= /invoice/i
  and auth.trust_score >= 0.8
```

DSL requirements:

- Deterministic evaluation order.  
- Explicit case/locale rules.  
- Safe regex constraints (bounded complexity).  
- Access to authentication metadata:
  - SPF/DKIM/DMARC results  
  - `auth.trust_score` derived from provider metadata

#### 10.3.9.3 Example workflows

**Invoice pipeline**

1. Trigger (DSL): invoice-like mail from trusted domains.  
2. Nodes:
   - Mail connector: fetch message + attachments.  
   - Docling + Wrangler: parse/clean invoice tables.  
   - DBA: insert/update finance DB using idempotent keys.  
   - AI: summarise invoice and link to project.  
   - Optional: `mail_draft_reply_v0.1` for confirmation text.  
3. Output:
   - Updated finance tables.  
   - Linked graph entities.  
   - Draft reply for user confirmation.

**Dev-log linking**

1. Trigger: notifications from code-hosting provider.  
2. Nodes:
   - TXT-001 mail profile: extract repo/issue IDs and event type.  
   - Graph engine: attach events to dev-log entities.  
   - Doc node: append to dev-log doc.

**Daily / weekly mail review**

1. Scheduled by Scheduler (local).  
2. Nodes:
   - Analyst: compute metrics (backlog, response times, sentiment).  
   - AI: `mail_analytics_brief_v0.1`.  
   - Chronicle: append to life log.  
3. Output:
   - Brief with explicit â€œyou owe replies toâ€¦â€ and â€œnotable sentiment changesâ€¦â€.

---

### 10.3.10 UX: One Cohesive GUI, No Extra Mail App

Principles:

- Threads are first-class entities, rendered as stacked message blocks in a doc-like view.  
- Attachments are integrated:
  - Contracts open as Handshake docs.  
  - Tables show as local tables.  
  - Transcripts inline for audio/video.

- Project/contact views include:
  - â€œRelated mailâ€ panels  
  - Quick actions (summarise thread, draft reply, extract tasks)

The same command palette and context menu surface actions for mail and non-mail content.

External mail clients remain optional during transition but are not required long-term.

---

### 10.3.11 Security, Edge Cases, and Safety

#### 10.3.11.1 Provider-specific differences (threading, labels)

- Thread model reconciles Gmail-style conversations, RFC references, and IMAP folders/labels via explicit precedence (Â§10.3.3.4.1).  
- Provider-specific IDs are stored alongside Handshakeâ€™s own IDs for debugging and export.

#### 10.3.11.2 Multipart and inline content

- Ingestion defines canonical rules:
  - For multipart/alternative, select a primary body.  
  - Inline images and CID references are treated as attachments with special display metadata.  
- Duplicate and phantom attachments are prevented by normalising CID and content hashes.

#### 10.3.11.3 Calendar invites and structured mail

- ICS attachments and structured meeting invites are parsed by dedicated mechanical paths:
  - Extract events and updates as structured objects.  
  - Present them as **draft** calendar entries first.  
  - External CalDAV/ICS sync only after confirmation.
- Canonical invite â†’ draft event â†’ patch-set/export behavior is defined in Â§10.4 (Calendar).

#### 10.3.11.4 Encrypted and signed mail (S/MIME/PGP)

- Encrypted messages:
  - Stored as RawContent; Handshake attempts local decryption only if keys are present.  
  - If decryption fails, content remains opaque; no LLM or mechanical engine sees plaintext.  

- Signed messages:
  - Signature status recorded as metadata for audit.  
  - Verified signatures can boost `auth.trust_score` in triggers.

#### 10.3.11.5 Large or hostile attachments

- Attachment safety policy:
  - Maximum size per MIME class.  
  - Archive recursion limits (ZIP bombs).  
  - Optional malware scan hooks prior to Docling/Tika.  
  - Sandboxing requirements (OS-level or container) for risky formats.

Unprocessable attachments are quarantined with clear error markers; ingestion does not crash the pipeline.

#### 10.3.11.6 Phishing, spoofing, and financial workflows

- Authentication metadata:
  - SPF/DKIM/DMARC results captured from provider.  
  - Derived `auth.trust_score` accessible to triggers and workflows.

- Finance/contract workflows:
  - Must require a minimum `auth.trust_score` threshold.  
  - May refuse to run on suspicious messages or require manual override.

#### 10.3.11.7 Concurrency, duplicates, and job locking

- Idempotency keys:
  - `(account_id, raw_hash, rfc822_message_id)` for ingestion.  
  - Thread-level keys for per-thread workflows.

- Job locking:
  - Ingestion and workflow engines must ensure that a given message/thread is not processed concurrently in conflicting ways.  
  - Retries on failure are tracked and bounded.

---

### 10.3.12 Comparison with Conventional AI Mail Stacks

Conventional pattern (Gmail/Outlook + AI):

- Centralised mail store in provider infra.  
- AI features:
  - Smart replies, smart compose.  
  - Thread summaries.  
  - Priority inbox heuristics.  
- Limited control over:
  - Model choice.  
  - Data routing.  
  - Logging/audit.

Handshake pattern:

- Local-first mirror of mail and attachments.  
- Mechanical engines convert mail into structured docs/tables/transcripts.  
- AI jobs:
  - Use a unified AI Job Model and model-runtime abstraction.  
  - Are governed by explicit classification-based routing policies.  

- Mail, docs, code, transcripts share:
  - The same Shadow Workspace.  
  - The same Knowledge Graph.  

- Flight Recorder logs:
  - Every significant operation, including classification and routing decisions.

Result: deeper, more inspectable integration than â€œAI in the inboxâ€.

---

### 10.3.13 Incremental Implementation Plan (High-Level)

 Phase 1 â€” Read-only ingestion

- Implement mail store + IMAP/JMAP sync with `READ_EMAIL`.  
- Parse bodies via Unstructured/Tika.  
- Run attachments through Docling + OCR + ASR.  
- Inject mail blocks into Shadow Workspace and indexes.  
- Implement TXT-001 mail profile with `MAIL_COMMUNICATION` domain.  
- Minimal UI: thread list + read-only view.  
- Record engine versions/configs in Flight Recorder.

 Phase 2 â€” Local AI jobs (no sending)

- Add `mail_summarize_thread_v0.1`, `mail_triage_inbox_v0.1`, `mail_thread_to_doc_v0.1` using local models only.  
- Introduce basic classification inference (local-only) and policy-based routing.  
- Add Analyst-based daily/weekly mail briefs.  
- Replies still sent via external clients.

 Phase 3 â€” Drafting and controlled sending

- Implement `mail_draft_reply_v0.1`.  
- Add mechanical `email_send` engine with:
  - Identity management (`from_identity`).  
  - Pre-send checks (Red Pen, Anonymizer, classification validation).  
  - Explicit before/after diff and provenance.

- Enforce SEND_EMAIL contracts and â€œrequire_confirmation = trueâ€ for all AI flows.

 Phase 4 â€” Advanced analytics, classification, and taste

- Introduce full classification ladder + tags and policy engine.  
- Tighten routing rules for cloud models and external connectors.  
- Wire in Polyglot/Red Pen/Sentiment/Anonymizer deeper into flows.  
- Enable Chronicle + Analyst dashboards for mail.  
- Train domain-specific taste models for mail reply style per client/classification.  

---

### 10.3.14 Conclusion

In this design, mail is:

- A first-class content domain within Handshakeâ€™s content and graph model.  
- Fully processed by Mechanical Extension Engines (Docling, OCR, ASR, TXT-001, Wrangler, DBA, Indexer, Librarian, Archivist, Analyst, etc.).  
- Governed by a classification-based routing system that protects corporate secrets, financials, legal mail, and personal data.  
- Integrated into the same Shadow Workspace and Knowledge Graph as all other content.  
- Acted on by both local and (optionally) cloud LLMs through a uniform, auditable AI Job Model.

Descriptor domains keep mail-style signals separate from creative / NSFW / other domains, while classification levels drive routing and consent. The result is an inbox that is no longer a silo or a blind spot, but a mechanically grounded surface deeply interwoven with the rest of the Handshake ecosystem.
## 10.4 Calendar

Status: Calendar Law v0.4 (verbatim import) is normative for semantics/sync; ACE integration v0.3 principles added in Â§10.4.2. Implementation in progress; enforce capability/consent + Workflow Engine gates before enabling writes.

### 10.4.0 Scope and positioning

This section specifies the Calendar surface as a first-class Handshake domain. It is governed by the same capability gating, AI Job Model, Workflow Engine, and Flight Recorder invariants as other surfaces.

**Authority / non-drift note**
- The â€œCalendar Lawâ€ inside Â§10.4.1 is normative for Calendar behavior.
- Â§10.4.1 is a zero-loss import of `Handshake_Calendar_Research_v0.4.md`; only Markdown heading levels were adjusted for nesting, and the original document title line was replaced by the Â§10.4.1 heading below.
- [ADD v02.155] In Phase 1, Calendar is also a backend force multiplier: `CalendarSourceSyncState`, `CalendarSource.write_policy`, `CalendarEvent.export_mode`, `capability_profile_id`, and `CalendarScopeHint` are canonical backend contracts for sync recovery, consent posture, AI-job mutation discipline, and scope-hint routing. These contracts MUST remain portable across SQLite-now / PostgreSQL-ready storage and explicit in Appendix 12 ownership/matrix rows.

### 10.4.1 Handshake Calendar Research v0.4 (verbatim import; headings adjusted)

#### -1. Revision notes

- Hardened temporal invariants (timezone/DST/floating time) and recurrence semantics (RRULE exceptions, per-instance edits).
- Added deterministic sync conflict policy + idempotency rules for bidirectional mirrors.
- Expanded capability/redaction matrix for calendar context exposure.
- Added UI state-machine invariants (draft/committed/read-only/multi-source overlays).
- Specified join semantics for CalendarEvent â†” ActivitySpan attribution.
- Added a normative â€œCalendar Lawâ€ section to prevent logic drift across UI/sync/agents.
- Expanded sync design into an explicit per-source state machine with deterministic transitions.
- Added a correctness-first test plan (golden fixtures, property tests, sync simulations) to lock down edge cases.


#### 0. Design stance

Goal: a calendar that is not â€œanother appâ€, but a time-structured view over the same workspace graph that docs, mail, tasks, descriptors, and mechanical engines already use.

Core stance:

- Events are first-class workspace entities (RawContent) with Derived analytics and Display views.
- React Big Calendar (RBC) is the calendar **view layer only**. No hidden â€œbusiness logicâ€ lives in the component.
- All manipulation (create, move, delete, autobook, summarise) runs through:
  - AI Job Model
  - Workflow Engine
  - Capability + Gate system
  - Shadow Workspace (indexing + analytics)
  - Flight Recorder (full traces of jobs, tools, and model runs)
- Google Calendar is treated as an external, lower-resolution mirror: integrated but not canonical.

The calendar must benefit from the â€œeverything can speak to anythingâ€ fabric of Handshake; it is a force multiplier for time, not just a date grid. It also becomes the main lens onto your Flight Recorder and activity history.


#### 1. Why React Big Calendar

##### 1.1 Feature fit

React Big Calendar (RBC):

- Pure React component, designed to look and behave like Google/Outlook calendar.
- Provides multiple views: month, week, work week, day, agenda.
- Supports drag-and-drop and resizing via the DnD addon.
- Uses `date-fns`/Luxon/moment for time math; you own the data and timezone logic.
- MIT licensed, no premium/closed features.

In practice, RBC is used to build scheduling dashboards, booking interfaces, and team calendars. It is opinionated visually but â€œdumbâ€ with respect to data, which is exactly what Handshake needs.

##### 1.2 Why not FullCalendar / tui.calendar / others

Alternatives like FullCalendar or tui.calendar are excellent but:

- They are more like mini-platforms with their own plugin systems.
- FullCalendar mixes MIT core with commercial â€œPremiumâ€ features, increasing licensing risk.
- They carry more assumptions about where logic lives, which competes with Handshakeâ€™s capability and workflow layers.

Conclusion:

- Use **React Big Calendar as the single calendar UI dependency**.
- Handshake owns the calendar semantics; RBC renders rectangles and captures user gestures.


#### 2. Core calendar model in Handshake

The calendar model follows the same Raw / Derived / Display pattern as the rest of the workspace.

##### 2.0 Calendar Law (authoritative invariants)

This section is **normative**. If any UI behavior, sync adapter, mechanical engine, derived analytics, or agent output conflicts with this law, the law wins.

###### 2.0.1 Calendar primitives (glossary)

- `instant_utc`: a unique instant on the UTC timeline (storage + comparison primitive).
- `tzid`: IANA timezone identifier (e.g., `Europe/Brussels`) used to interpret wall time and DST rules.
- `wall_time`: a local clock representation in a timezone (e.g., â€œ09:00 on 2025-12-13â€ in `Europe/Brussels`).
- `date_only`: a calendar date without time-of-day (all-day semantics).
- `floating`: a wall time with no timezone binding at creation time (rare; must be normalized deterministically).
- `duration`: positive length of time; for events stored as `(start, end)` and never inferred from title/body.

###### 2.0.2 Canonical storage rules

1. **Store instants, preserve intent:** persist `start_instant_utc` and `end_instant_utc` *and* the original `tzid` and `wall_time` fields used to create the event.
2. **All-day events are date-only:** persist `start_date` and `end_date_exclusive` (half-open) and compute instants only for display/export. Do not treat all-day as â€œmidnight timestampsâ€.
3. **Floating time normalization:** if an external source provides floating times, normalize using `CalendarSource.default_tzid` (or workspace tz) and preserve the original payload verbatim.
4. **Half-open intervals:** all time ranges are `[start, end)` (inclusive start, exclusive end). This applies to events, recurrence instances, free/busy blocks, and ActivitySpans.
5. **No silent coercion:** if conversion would change the user-visible meaning (DST gap/overlap), record an explicit `CalendarNormalizationNote` and require a user-facing badge.

###### 2.0.4 Mutation governance (Hard Invariant) [ilja251220250127]

- **[HSK-CAL-WRITE-GATE]:** Direct database writes to `calendar_events` are **PROHIBITED** from the API layer or UI components. 
- All mutations MUST be submitted as `CalendarMutation` patches via a `WorkflowRun` targeting the `calendar_sync` mechanical engine.
- Every successful mutation MUST emit a `Flight Recorder` span of type `calendar_mutation` with a back-link to the `job_id`.

###### 2.0.3 Recurrence and instance identity

- **RRULE is source-of-truth:** store RRULE + `DTSTART` semantics + exceptions (`EXDATE`, `RDATE`) without lossy â€œflatteningâ€.
- **Stable instance identity:** every concrete occurrence MUST have an `instance_key` stable under re-sync and UI refresh, derived as:

  `instance_key = hash(source_id + external_id + series_master_id + occurrence_start_instant_utc + tzid + recurrence_generation)`

  where `recurrence_generation` increments only when the recurrence definition itself changes (not when display fields change).

- **Per-instance overrides:** edits to a single instance create an override record keyed by `instance_key` (or provider-native instance id) and never mutate unrelated instances.
- **â€œThis and followingâ€ split:** split recurrence by creating a new series master with a new `series_master_id`, and persist a `recurrence_split_from_instance_key` backlink for provenance.

###### 2.0.4 Mutation and governance rules

- **No direct UI writes:** UI gestures emit jobs; only the host applies patches after validation and gates.
- **Patch-sets are the only write primitive:** all calendar writes (local or external) are expressed as validated patch-sets with:
  - preconditions (`expected_etag`, `expected_local_rev`)
  - effect (`set`, `unset`, `append`, `remove`)
  - provenance (`job_id`, `client_op_id`, `idempotency_key`)
- **External writes are explicitly gated:** any write that leaves the device requires capability + user confirmation unless the source is configured as `auto_export=true`.

###### 2.0.5 Never-lose-data rule

- Preserve the original provider payload in `source_payload` (encrypted-at-rest if needed).
- If parsing fails, store the raw record with `parse_status="failed"` and surface it as â€œunparsed eventâ€, never drop it.


##### 2.1 Raw entities

```text
CalendarEvent (RawContent)
- id (RID)
- workspace_id
- source_id (CalendarSource.id, e.g. "local", "google:...", "ics:...")
- external_id (nullable; provider-specific event id)
- external_etag (nullable; for conflict detection)
- title
- description
- start_ts (timestamp + timezone)
- end_ts (timestamp + timezone)
- all_day (bool)
- recurrence_rule (RRULE string, optional)
- location (free text)
- status (confirmed | tentative | cancelled)
- visibility (public | private | busy_only)
- export_mode (local_only | busy_only | full_export)
- attendees[] (ParticipantRef)
- links[] (EntityLinkRef -> doc, canvas, task, mail_thread, etc.)
- created_by (User/Agent RID)
- created_at
- updated_at
```

```text
CalendarSource (RawContent)
- id: "local:<id>" | "google:<account_id>:<calendar_id>" | "ics:<url>" | ...
- type: "local" | "google" | "ics" | "caldav" | "other"
- label: "Local", "Google / Personal", "Google / Work", ...
- connection_config_ref (credential/secret reference)
- google_calendar_id (for Google sources; e.g. "primary" or explicit id)
- sync_state:
    - sync_token (nullable, provider-specific)
    - last_synced_at
    - last_full_sync_at
    - last_error
- capability_profile_id: which jobs/agents may touch this source
```

Export modes control how much of a local event is mirrored to external calendars:

- `local_only`   â†’ event exists only in Handshake.
- `busy_only`    â†’ external calendar sees a generic â€œbusyâ€ block.
- `full_export`  â†’ external calendar sees full details (subject, time, maybe description).



###### 2.1.1 Temporal invariants (timezone, DST, â€œfloatingâ€ events)

Handshake must treat time as a **deterministic, lossless** domain. The following invariants are required:

- **Canonical storage:** store `start_ts_utc` and `end_ts_utc` as UTC instants, and also store the originating `tzid` (IANA timezone string) and the original â€œwall-clockâ€ local time used to create the event (`start_local`, `end_local`) when available.
- **Display rule:** UI renders from `(start_ts_utc/end_ts_utc + tzid)`; never from a raw local timestamp without timezone context.
- **All-day events:** store as date boundaries in the eventâ€™s `tzid` (local-midnight anchored), and derive UTC instants for query/index only.
- **Floating events:** events created without a timezone (or imported as â€œfloatingâ€) must be normalized by assigning a `tzid` explicitly at ingest time (default: the owning `CalendarSource.default_tzid`), and the fact that it was floating must be preserved (`was_floating=true`) for audit.
- **DST correctness:** recurrence expansion and display must use tzdb rules for `tzid` at the occurrence date, not a fixed offset.
- **Query semantics:** all time-window queries are performed on UTC instants. If the user selects â€œweek viewâ€ in a timezone, the selected range is converted to `[from_utc, to_utc)` before querying.

Required fields (additions to `CalendarEvent`):

- `tzid: string`
- `start_ts_utc: timestamp`
- `end_ts_utc: timestamp`
- `start_local: string?` (RFC3339 without offset, or structured local datetime)
- `end_local: string?`
- `was_floating: bool`

###### 2.1.2 Recurrence invariants (RRULE, exceptions, per-instance edits)

Recurrence is treated as a **series + instances** problem, not a rendering trick.

- **Series definition:** store `rrule` (and optional `rdate[]`, `exdate[]`) on the series event.
- **Instance identity:** every expanded occurrence must have a stable `instance_key`:
  - `instance_key = hash(event_id + original_start_local + tzid)` (or equivalent stable tuple).
- **Exceptions:** per-instance edits are stored as explicit override records keyed by `instance_key`:
  - move/resize a single instance â†’ create `CalendarEventOverride` record rather than mutating the whole series.
- **Split semantics:** â€œthis and followingâ€ creates a new series with a new `event_id` and an `rrule` starting at the split boundary; the original series gains an `UNTIL` or an `EXDATE` set covering the split.
- **Expansion window:** expansion is performed server-side for the active display window plus a safety margin (e.g., +30 days) and is cached.

Required fields / structures:

- `rrule: string?`
- `rdate: string[]?`
- `exdate: string[]?`
- `series_id: string?` (self for series roots; set for instances/overrides)
- `instance_key: string?` (set for instances/overrides)
- `is_override: bool`

Minimal override structure:

```text
CalendarEventOverride
- id
- series_id
- instance_key
- patch_set (start/end/title/attendees/etc)
- created_by (human | job_id)
- created_at
```

###### 2.1.3 Identity and idempotency invariants (dedupe, stable linkage)

- **Canonical identity tuple:** `(source_id, external_id)` identifies an imported event uniquely.
- **Content versioning:** store `external_etag` (or equivalent) and `source_last_seen_at`. Never â€œblind overwriteâ€ local edits if the external version is unchanged.
- **Stable internal id:** `CalendarEvent.id` must never change after creation, even if the event is mirrored to other sources.
- **Deduplication rule:** on ingest, if `(source_id, external_id)` exists, update in-place; never create a new `CalendarEvent` row.

##### 2.2 Derived and Display entities

```text
CalendarAnalytics (DerivedContent)
- id
- time_range (day | week | month)
- metrics:
    - total_hours_meetings
    - deep_work_blocks
    - context_switch_count
    - by_tag[project] -> minutes
- source_event_ids[]
```

```text
CalendarSuggestion (DerivedContent)
- id
- suggestion_type (block_focus_time | auto_schedule | reschedule | cluster_meetings | chores | meals | etc.)
- target_event_ids[]
- proposed_changes (patch set over CalendarEvent)
- confidence
- produced_by_job_id
```

Display entities:

- `DisplayTimelineView` â€“ saved view configuration (filters, colors, grouping, visible sources).
- `DisplayAgendaExport` â€“ static agenda renders for export/share.


##### 2.3 Storage and indexing

- Relational table `calendar_events` with indices on `(workspace_id, start_ts, end_ts)` and full-text on `title`, `description`, `location`.

**Persistence Layer (SQLite) [ilja251220250127]:**

```sql
-- [CX-340] Centralized Storage
CREATE TABLE calendar_sources (
    id TEXT PRIMARY KEY NOT NULL, -- "local:uuid" | "google:account:id" | "caldav:url"
    workspace_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    provider_type TEXT NOT NULL, -- "local", "google", "caldav"
    write_policy TEXT NOT NULL, -- "read_only_import", "two_way_mirror", "publish_from_handshake"
    default_tzid TEXT NOT NULL DEFAULT 'UTC',
    auto_export BOOLEAN NOT NULL DEFAULT 0,
    credentials_ref TEXT, -- handle to encrypted store (Tauri side)
    last_sync_ts DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE calendar_events (
    id TEXT PRIMARY KEY NOT NULL, -- stable internal UUID
    workspace_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    external_id TEXT, -- provider-specific ID (e.g. iCal UID)
    external_etag TEXT,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    start_ts_utc DATETIME NOT NULL,
    end_ts_utc DATETIME NOT NULL,
    tzid TEXT NOT NULL DEFAULT 'UTC',
    all_day BOOLEAN NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'confirmed', -- 'confirmed', 'tentative', 'cancelled'
    visibility TEXT NOT NULL DEFAULT 'private', -- 'private', 'public', 'busy_only'
    export_mode TEXT NOT NULL DEFAULT 'full_export',
    rrule TEXT, -- iCal RRULE string
    is_recurring BOOLEAN NOT NULL DEFAULT 0,
    instance_key TEXT, -- hash for recurrence instances
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_id) REFERENCES calendar_sources(id)
);
```

- Graph edges:
  - `event -> doc` (meeting notes)
  - `event -> mail_thread` (invites)
  - `event -> task` (follow-up tasks)
- Shadow Workspace indices:
  - Time index (events by time range)
  - Participant index (who you meet)
  - Topic index (based on descriptors/labels)
  - Link index (entities associated with each event)


#### 3. Rust calendar infrastructure (ICS / CalDAV / JSCalendar)

Handshake should not re-implement calendar standards; it should adapter-wrap existing Rust crates.

Recommended crates:

- `calcard` â€“ modern crate for iCalendar + vCard + JSCalendar + JSContact, including recurrence expansion and timezone handling.
- `icalendar` â€“ builder + parser for RFC 5545 iCalendar, convenient for ICS generation.
- `ical` â€“ low-level iCalendar/vCard parser, as a fallback.

Mechanical engine: `calendar_sync`.

```text
Engine: calendar_sync
- Input:
  - CalendarSource.id
  - direction: pull | push | bidirectional
  - time_window: [from, to]
- Behaviour (high level):
  - Pull:
    - Use appropriate adapter (Google API, CalDAV, plain ICS, etc.).
    - Parse into an internal event representation via calcard/icalendar.
    - Upsert CalendarEvent rows, maintaining external_id and external_etag.
  - Push:
    - Compute deltas for events that should be mirrored.
    - Convert CalendarEvent â†’ external format and call provider APIs.
  - Always record sync activity in Flight Recorder.
- Output:
  - calendar_sync_result (DerivedContent)
  - Updated CalendarEvent rows
```

All external writes are capability-gated (e.g. `EXTERNAL_CALENDAR_WRITE`) and must go through the Workflow Engine, not ad-hoc helpers.

**`calendar_sync` Mechanical Engine Contract [ilja251220250127]:**

```rust
pub enum CalendarMutation {
    Create(CalendarEvent),
    Update { id: Uuid, patch: CalendarPatch },
    Delete(Uuid),
}

pub struct CalendarSyncInput {
    pub source_id: String,
    pub mutation: Option<CalendarMutation>, // If None, it is a periodic "pull" sync
}
```

##### 3.1 Deterministic parsing / emitting (ICS, CalDAV, JSCalendar)

Calendar standards are treated as **wire formats**, not authoritative semantics.

- Parse inbound objects into the canonical `CalendarEvent` model (including the temporal/recurrence invariants in Â§2.1.1â€“Â§2.1.3).
- Emit outbound updates from canonical state only.
- Preserve â€œunknownâ€ vendor fields in an `external_blob` map so round-trips do not silently drop data.
- Normalize:
  - time ranges to `[start_ts_utc, end_ts_utc)`
  - tzid to IANA names
  - recurrence to `rrule + rdate/exdate` (even if the source uses alternate encodings)

##### 3.2 Sync conflict resolution and idempotency (bidirectional mirrors)

When Handshake syncs with an external calendar, every change must be reproducible and safe under retries.

**Idempotency rules**

- Every sync run has a `sync_run_id`.
- Every external mutation request carries an `idempotency_key`:
  - `idempotency_key = hash(source_id + operation + target_external_id + desired_version + sync_run_id)`
- The host persists an `outbox` table with `(idempotency_key, request_payload, status, last_error)` so retries do not duplicate events.

**Conflict policy (deterministic)**

Define `write_policy` per `CalendarSource`:

- `read_only_import` (default for â€œWorkâ€ calendars): external always wins; local edits are prohibited.
- `two_way_mirror` (personal calendars): conflicts are captured and require resolution.
- `publish_from_handshake` (local-first): Handshake wins; external is a projection.

For `two_way_mirror`, define versions:

- `last_external_etag`
- `last_local_edit_rev`
- `last_sync_applied_rev`

Conflict detection:

- If local changed since `last_sync_applied_rev` **and** external `etag` changed since `last_external_etag` â†’ conflict.

Conflict handling:

- Create a `CalendarConflict` record with both â€œcandidate statesâ€ plus a machine-generated diff.
- UI shows the conflict badge; no silent auto-merge.
- Resolution is a gated job (`CALENDAR_RESOLVE_CONFLICT`) producing an explicit patch-set to apply to the chosen side(s).

Minimal conflict record:

```text
CalendarConflict
- id
- source_id
- event_id
- external_id
- local_rev
- external_etag
- local_snapshot
- external_snapshot
- created_at
- status (open | resolved)
- resolved_by (human | job_id)
```

**Locking**

- Per-source sync uses an advisory lock: only one sync run per `CalendarSource` at a time.
- Per-event external writes are serialized by `(source_id, external_id)` to avoid interleaved updates.


###### 3.2.1 Per-source sync state (persisted, queryable)

Each `CalendarSource` persists a sync state record. This is the *single* source of truth for incremental sync and recovery.

```text
CalendarSourceSyncState
- source_id
- state: IDLE | PULLING | APPLYING | PUSHING | CONFLICTED | ERROR_BACKOFF | DISABLED
- cursor/sync_token (provider-specific; nullable)
- last_ok_at
- last_pull_at
- last_push_at
- last_error_at
- last_error_code (provider-specific)
- backoff_until (nullable)
- consecutive_failures
- last_remote_watermark (etag/updatedMin/sequence; provider-specific)
- last_local_applied_rev (monotonic local rev applied to this source)
```

All state transitions MUST be logged to the Flight Recorder as `SYNC_STATE_CHANGE` with `(source_id, from_state, to_state, sync_run_id, reason)`.

###### 3.2.2 Outbox and mutation journal (local â†’ external)

External writes never occur â€œinlineâ€ from UI. They are queued and replayed.

```text
CalendarOutbox
- id (RID)
- source_id
- idempotency_key
- client_op_id (nullable; from UI gesture/job)
- op: CREATE | PATCH | DELETE
- target_external_id (nullable)
- expected_etag (nullable; If-Match semantics when supported)
- patch_set / payload
- status: PENDING | IN_FLIGHT | APPLIED | FAILED | CONFLICT
- retry_count
- last_error
- created_at
- updated_at
```

Rules:

- `idempotency_key` is the dedupe key. If an identical key exists with `APPLIED`, treat as success.
- For `PATCH`/`DELETE`, set `expected_etag` when the provider supports conditional writes.
- Outbox application order is stable: `(created_at, id)`.

###### 3.2.3 Sync state machine (deterministic transitions)

Per source, sync is a strict state machine:

```text
IDLE
  -> PULLING (manual trigger | schedule | outbox non-empty)
PULLING
  -> APPLYING (pull ok)
  -> ERROR_BACKOFF (pull error)
APPLYING
  -> PUSHING (local apply ok and outbox allowed)
  -> CONFLICTED (conflict detected)
  -> ERROR_BACKOFF (apply error)
PUSHING
  -> IDLE (push ok and outbox empty)
  -> PUSHING (push ok and outbox still has items)
  -> CONFLICTED (etag precondition failed / remote changed)
  -> ERROR_BACKOFF (push error)
CONFLICTED
  -> PUSHING (after a resolved conflict emits new outbox items)
  -> IDLE (if configured read-only or user discards local candidates)
ERROR_BACKOFF
  -> IDLE (when backoff_until elapsed; next trigger restarts at PULLING)
DISABLED
  -> (no transitions unless user re-enables the source)
```

Determinism requirements:

- One sync run at a time per `source_id`.
- Within a run, perform phases in fixed order: `PULL â†’ APPLY â†’ PUSH`.
- Never interleave pull/apply/push across sources in a way that breaks per-source ordering guarantees.

###### 3.2.4 Rate limits, backoff, and recovery

- Backoff schedule: exponential with jitter *bounded* by `max_backoff_minutes` (configurable per source).
- Provider â€œreset requiredâ€ (e.g., Google sync token invalidation / 410 GONE): clear `cursor/sync_token`, mark `state=ERROR_BACKOFF`, set `backoff_until=now+short`, and restart with a full-window pull.
- Hard auth failure: `state=DISABLED` and surface a re-auth CTA (no silent retries).
- Partial success is allowed only if explicitly recorded (e.g., some outbox items applied, some failed); never report â€œsync okâ€ unless all phases that were attempted completed without unresolved failures.

###### 3.2.5 Deterministic replay guarantee

Given:

- the same initial local DB state,
- the same external change stream (pulled in the same order),
- and the same queued outbox entries,

the resulting local calendar tables MUST be identical. This is required for trustworthy auditing and for reproducing failures from the Flight Recorder.



##### 3.3 Correctness-first test plan (lock down the hard parts)

Calendar correctness fails in the edges (DST, recurrence exceptions, per-instance edits, interleaved sync). Handshake should treat these as *hard requirements* and ship with a regression suite from day one.

###### 3.3.1 Golden ICS fixtures (hand-curated)

Maintain a fixture corpus (versioned in-repo) that covers:

- DST forward gap (non-existent local times) and DST backward overlap (ambiguous local times), at minimum:
  - `Europe/Brussels`
  - `America/New_York`
- All-day events:
  - single-day, multi-day, crossing DST boundaries
  - with and without timezones in provider payload
- Floating events (no tz) normalized via `CalendarSource.default_tzid`
- Recurrence:
  - RRULE with COUNT / UNTIL
  - BYDAY / BYMONTHDAY / BYSETPOS
  - EXDATE and RDATE combinations
  - per-instance override (edit one occurrence)
  - â€œthis and followingâ€ split creating a second series
- Provider oddities:
  - missing/invalid `TZID`
  - `DTEND` omitted (duration only)
  - recurring series where instances carry different `etag`s

Fixture layout (example):

```text
tests/fixtures/calendar/ics/
  dst_gap_brussels.ics
  dst_overlap_brussels.ics
  rrule_exdate_rdate.ics
  split_this_and_following.ics
  floating_times_normalize.ics
```

Each fixture comes with an expected â€œgolden expansionâ€ for a fixed window (e.g., 90 days) and expected normalized RawContent fields.

###### 3.3.2 Property tests (Rust) for invariants

Use property-based testing to ensure invariants hold under random-but-valid inputs:

- Expansion is deterministic: same input â†’ identical ordered instance list.
- No duplicates: `instance_key` unique within `(source_id, external_id, window)`.
- Monotonic ordering: instances strictly ordered by `(start_instant_utc, end_instant_utc, instance_key)`.
- Half-open interval correctness: adjacent instances do not overlap unless explicitly allowed (e.g., all-day + timed event).
- Idempotency: applying the same patch-set/outbox item twice yields identical DB state.
- Round-trip safety (bounded): parse â†’ normalize â†’ emit ICS should preserve semantic meaning for supported fields.

###### 3.3.3 Sync simulation tests (two-writer interleavings)

Build a deterministic simulator that replays interleavings:

- Local edits (outbox) and remote edits (pull stream) targeting:
  - same event fields
  - different fields
  - recurrence series vs instance overrides
- Expected outcomes:
  - conflicts raised where rules require
  - no duplicated events
  - no lost updates under retry
  - stable conflict records with reproducible diffs

Minimum scenarios:

1. Local moves an event while remote edits description â†’ should merge or conflict based on policy.
2. Local edits instance override while remote edits master RRULE â†’ conflict.
3. Remote deletes event while local patches â†’ conflict or discard based on `write_policy`.

###### 3.3.4 CI gating and regression discipline

- Any change to calendar parsing, normalization, recurrence, or sync must run the full suite.
- Failures must be reproducible from a single `sync_run_id` + fixture id.
- When a bug is found, add a fixture or a property-test seed before patching (â€œtest-first bug captureâ€).

Success metric: â€œcalendar bugsâ€ become additions to the fixture corpus, not recurring incidents.



#### 4. UI integration with React Big Calendar

##### 4.1 Event projection

RBC expects events in the shape:

```ts
type RBCEvent = {
  id: string;
  title: string;
  start: Date;
  end: Date;
  allDay?: boolean;
  resource?: any;
};
```

Mapping from `CalendarEvent`:

```text
rbcEvent.id       = CalendarEvent.id
rbcEvent.title    = CalendarEvent.title
rbcEvent.start    = CalendarEvent.start_ts (converted to JS Date, respecting timezone)
rbcEvent.end      = CalendarEvent.end_ts
rbcEvent.allDay   = CalendarEvent.all_day
rbcEvent.resource = {
  sourceId: CalendarEvent.source_id,
  status: CalendarEvent.status,
  visibility: CalendarEvent.visibility,
  exportMode: CalendarEvent.export_mode,
  links: CalendarEvent.links,
  suggestion: CalendarSuggestion | null,
}
```

Recurrence:

- Recurring rules (RRULE) are expanded by backend logic (calcard) for the visible time window.
- RBC only sees concrete instances; recurrence editing happens via Handshake logic, not inside RBC.


##### 4.2 Gestures â†’ AI Jobs

RBC exposes handlers such as:

- `onSelectSlot` (user selects empty time range)
- `onSelectEvent`
- `onEventDrop` / `onEventResize` (DnD addon)

Handshake must never mutate events directly from the component. Instead, each gesture creates a typed AI Job:

Example: move existing event by drag-and-drop.

```text
Job: CALENDAR_MOVE_EVENT
- profile: calendar_mutation
- inputs:
  - event_id
  - proposed_start_ts
  - proposed_end_ts
- constraints:
  - respect_busy_for: [self, critical_contacts]
  - cannot_overlap_tags: ["deep_work"]
- tools:
  - Shadow Workspace (conflict check)
  - Analyst (impact on deep work / meeting load)
  - optional LLM explainer
- outputs:
  - CalendarEvent patch set
  - optional CalendarSuggestion describing alternatives
```

The Workflow Engine:

- Runs deterministic checks first (conflicts, invariants).
- Optionally calls an LLM to justify or propose alternatives.
- Applies patches only when gates approve, or after explicit user confirmation.



##### 4.3 UI invariants and state machine (draft vs committed, read-only, multi-source)

To prevent â€œlogic driftâ€ into the UI and avoid confusing cross-source edits, the calendar UI follows an explicit state model.

**Event states**

- `draft_local`: created/edited locally but not yet gated/applied (pending approval).
- `committed_local`: stored canonical event in Handshake DB.
- `mirrored_external`: linked to an external source (`external_id`) and in-sync.
- `read_only_external`: imported from a `read_only_import` source (cannot be edited).
- `conflict`: has an open `CalendarConflict` record.

**UI rules**

- Drag/resize on `read_only_external` is disabled (tooltips explain why).
- Drag/resize on `mirrored_external` creates a job that may update both local canonical state and external projection, depending on `write_policy`.
- A draft edit always shows a â€œpendingâ€ badge and never mutates canonical state until the Workflow Engine applies patches.
- Multi-source overlay is always explicit:
  - filter chips per `CalendarSource`
  - event badges show source + export mode (`local_only` / `busy_only` / `full_export`)
- â€œConvert external event into local rich eventâ€ is a copy/link action:
  - creates a new `committed_local` event and an edge linking to the imported external event (no implicit takeover).

**Conflict visualization**

- Conflicts render with a persistent badge.
- Clicking opens a deterministic diff view and a single resolution action path (`CALENDAR_RESOLVE_CONFLICT`).


#### 5. Local and cloud models using the calendar

Models use the calendar in two ways: as a context source and as an actuator surface.

##### 5.1 Context provider

Calendar data is exposed to models through a `CalendarContext` provider in the orchestrator:

```text
CalendarContext query
- filters:
  - time_range
  - tags
  - participants
  - project_id
  - sources (local, google, etc.)
- projection:
  - minimal (time + title)
  - full (including description + links)
  - analytics (aggregated metrics)
- redaction rules:
  - respect visibility (private/busy-only)
  - respect classification and export_mode
```

Local model:

- Can see richer, more private slices of calendar + linked docs/tasks because it runs on-device.

Cloud model:

- Receives summarised or redacted slices (e.g. â€œ9â€“11 busy with Project Xâ€, not full text) based on capabilities and export policies.

Typical queries:

- â€œSummarise my weekâ€ â†’ events + linked notes.
- â€œWhen did I last work on PROJECT_X?â€ â†’ events tagged PROJECT_X and associated docs.
- â€œWhere is my uninterrupted 2-hour deep-work window?â€ â†’ Derived analytics + raw free/busy grid.



###### 5.1.1 Capability and redaction matrix (what models can see / change)

Calendar is high-signal personal data. Exposure is controlled by **capabilities** and by `CalendarSource.write_policy` + `CalendarEvent.export_mode`.

**Suggested capabilities**

- `CALENDAR_READ_BASIC` â€” may read time bounds + busy/free, no titles/descriptions.
- `CALENDAR_READ_DETAILS` â€” may read titles, participants, links, descriptions (subject to classification).
- `CALENDAR_READ_ANALYTICS` â€” may read Derived analytics only (aggregates).
- `CALENDAR_WRITE_LOCAL` â€” may create/update local canonical events.
- `CALENDAR_WRITE_EXTERNAL` â€” may push changes to external sources (requires per-source allow-list).
- `CALENDAR_DELETE_LOCAL`
- `CALENDAR_DELETE_EXTERNAL`
- `CALENDAR_RESOLVE_CONFLICT` â€” may choose resolution outcomes (human gate recommended).

**Redaction rules (projection-level)**

- `projection=minimal`:
  - always return: `start/end`, `all_day`, `busy/free`, `source_id`, `export_mode`
  - never return: title/description/attendees
- `projection=full`:
  - return title/description/attendees only if:
    - capability includes `CALENDAR_READ_DETAILS`, and
    - event classification allows it, and
    - export_mode is not `busy_only` for the consumer context
- `projection=analytics_only`:
  - return only aggregates (counts, durations, tokens), no per-event strings

**Local vs cloud default**

- Local models default to `projection=full` for `local_only` sources and `projection=minimal` for external imports unless explicitly requested.
- Cloud models default to `projection=analytics_only` or `projection=minimal` unless a job has explicit user approval.


##### 5.2 Actuator via AI Job profiles

Suggested job profiles:

1. `calendar_suggest_slots`
   - Input: duration, participants, constraints, time window.
   - Behavior: build busy map (local + external), propose candidate slots with confidence.
   - Output: list of candidate slots plus explanation.

2. `calendar_autobook`
   - Input: accepted suggestion or natural language (â€œbook 1h with Bob next weekâ€).
   - Behavior: create CalendarEvent; optionally call external sync engine and mail engine for invites.
   - Output: created event(s), optional mail drafts.

3. `calendar_summarize_day`
   - Input: list of events for a given date + linked docs/mails.
   - Behavior: produce human-readable narrative; emit Chronicle entry and Derived analytics.

4. `calendar_refactor_week`
   - Input: weekâ€™s events + constraints (no-meeting blocks, energy/chronotype pattern).
   - Behavior: propose a new arrangement that reduces context switching, groups meetings, protects deep work.
   - Output: CalendarSuggestion (patches) requiring explicit approval.

5. `calendar_from_mail`
   - Input: MailMessage / MailThread (â€œletâ€™s meet Wednesday at 3pm CETâ€¦â€).
   - Behavior: parse date/time/timezone, participants, topic; propose tentative event + reply draft.

In all cases, the Workflow Engine enforces capabilities (which calendars an agent can touch), logs every mutation, and ensures human-in-the-loop where needed.


#### 6. Interactions with other Handshake tools

The calendar should be tightly wired into existing engines and views.

##### 6.1 Docs & Canvas

- Each `CalendarEvent` can have a primary `doc_id` for notes:
  - Event detail view opens or splits to that doc.
- Derived `MeetingOutcome` entities can capture decisions, actions, owners.
- Shadow Workspace indices: `event_id -> outcome_ids` for later analytics.

Use cases:

- Embedded mini-calendar inside a project doc showing related events.
- Canvas-based planning boards that drop â€œWork Sessionâ€ cards into a calendar lane, creating linked CalendarEvents.


##### 6.2 Tasks and rich to-do list

Model tasks separately but link them:

```text
Task (RawContent)
- id
- title
- description
- status (todo | doing | done)
- due_ts (optional)
- estimated_duration
- project_id
- linked_event_id (optional)
```

Interactions:

- `calendar_block_task` job:
  - Input: Task.id + time preferences.
  - Behavior: propose time slots and create â€œWork on <task>â€ events.

- `calendar_autofill_from_tasks` job:
  - Input: free time windows + tasks nearing due dates.
  - Behavior: fill free slots with blocks assigned to tasks, under constraints (max hours/day, no meetings after X, etc.).

Calendar remains a time-allocation engine; tasks remain the unit of work. Links bind them.


##### 6.3 Mail

- Use mail ingestion to detect invites and â€œsoft invitesâ€ (natural language scheduling).
- Mechanical engine `mail_to_event`:
  - Input: mail thread.
  - Behavior: parse candidate times, locations, people; propose CalendarEvent(s).
  - Output: tentative event + card in the mail UI (â€œCreate event from this thread?â€).

Links:

- `CalendarEvent.links` includes `{"type": "invitation_mail", "target": MailThread.id}`.

AI use:

- â€œDecline all meetings without agendaâ€:
  - Find events without linked notes doc or meeting outcome.
  - Generate polite decline replies via mail engine.


##### 6.4 Sous Chef, Homestead, Safety

Temporalisation of daily-life engines:

- Sous Chef:
  - Convert meal plans into prep/cook/eat events.
  - Avoid conflict with deep work and fixed commitments.

- Homestead:
  - Schedule recurring maintenance events (filters, inspections, cleaning rotations).
  - Use descriptors/taste to avoid stacking hated chores in a single block.

- Safety:
  - Time-based alerts for food spoilage windows, medication timings, or cooking safety windows.

Pattern:

- Each engine produces `CalendarSuggestion` rows tagged by source.
- User accepts â†’ suggestion is promoted into CalendarEvent with back-links.


##### 6.5 Archivist, Chronicle, Analyst, Wrangler, DBA

- Archivist:
  - Periodic snapshots of calendar events to append-only archives (for compliance or long-term â€œwhat was my schedule?â€ queries).

- Chronicle:
  - Nightly `calendar_summarize_day` job writes narrative daily logs referencing events and notes.

- Analyst:
  - Metrics like time in meetings, deep work ratio, collaboration graph, by-project time allocation.

- Wrangler / DBA:
  - Mirror `calendar_events` into DuckDB/SQLite for heavy queries.


#### 7. Big-tech landscape and gaps

Mainstream calendars (Google, Outlook, Apple) are strong on:

- Collaboration and sharing.
- Tight integration with their own ecosystem (Gmail/Meet, Outlook/Teams).
- Mobile clients and notifications.

Typical gaps:

- Data locality and privacy: everything is cloud-first; no full local brain that can see your entire life context.
- Shallow analytics: coarse â€œtime insightsâ€, no serious per-project or per-entity metrics, no integration with your docs/notes/tasks beyond simple attachments.
- Weak programmability at the user level: scripting exists (Apps Script, Graph API) but is not accessible to most users and lacks AI-native patterns (jobs, capabilities, validators).
- Poor integration with arbitrary tools: integrations are siloed or vendor-gated.

Handshake calendar explicitly targets these gaps with:

- Local-first architecture.
- Deep graph integration (docs, tasks, mail, descriptors, engines).
- Analytics via Shadow Workspace + Wrangler/DBA.
- AI-native workflows and capabilities as first-class concepts.


#### 8. MCP and calendar

MCP (Model Context Protocol) is a standard to connect models to tools and data in a uniform way. The calendar fits into MCP in two symmetric roles.

##### 8.1 Handshake as MCP client for external calendars

- Implement MCP tools that wrap Google Calendar, Outlook/Exchange, and generic CalDAV.
- Use these tools **inside** the `calendar_sync` engine instead of hardcoding clients.
- This lets your orchestrator call â€œlist eventsâ€ / â€œcreate eventâ€ / â€œupdate eventâ€ uniformly, regardless of provider.

Conceptual MCP tool examples:

```json
{
  "tool": "external_calendar_list_events",
  "input_schema": {
    "type": "object",
    "properties": {
      "calendar_id": {"type": "string"},
      "time_min": {"type": "string", "format": "date-time"},
      "time_max": {"type": "string", "format": "date-time"}
    },
    "required": ["calendar_id", "time_min", "time_max"]
  }
}
```

##### 8.2 Handshake as MCP server exposing its own calendar

- Expose clean MCP tools like `handshake_calendar_list`, `handshake_calendar_create`, `handshake_calendar_update`, `handshake_calendar_delete` backed by CalendarEvent entities.
- External AI agents (including future tools) can interact with the Handshake calendar under the same gates/validators.

Benefits:

- A single, consistent story of â€œcalendar as a toolâ€ for both internal and external actors.
- Capability and validation systems remain in control, regardless of where calls originate.


#### 9. Novel use cases leveraging â€œeverything connectsâ€

##### 9.1 Workspace-aware day compiler

Job: `calendar_compile_day`

- Input:
  - Date D
  - User constraints (wake/sleep, no-meeting windows, deep-work hour targets)
- Behaviour:
  - Gather tasks near due dates.
  - Scan docs with â€œtodayâ€ markers or high-priority tags.
  - Pull existing events from local + external sources.
  - Ask Taste Engine / descriptors how user prefers to work (morning vs evening, heavy vs light tasks).
  - Produce a day plan: a sequence of CalendarEvents + recommended tasks per block.
- Output:
  - `CalendarSuggestion` set (events + ordering).
  - â€œDay planâ€ doc linked to each event.

This goes beyond big-tech calendars by merging tasks, docs, and personal taste into a single daily script.


##### 9.2 Attention debt collector

Job: `calendar_collect_attention_debt`

- Input: time window (e.g. last 2 weeks).
- Behaviour:
  - Find events with open TODOs in notes or unresolved MeetingOutcome entities.
  - Cluster them by project/topic.
  - Propose new â€œattention debtâ€ blocks to clean up.
- Output:
  - Suggested events to clear debt.
  - Summary doc listing unresolved items by theme.

Under the hood:

- Uses Shadow Workspace to query MeetingOutcome and tasks.
- Uses Analyst/Wrangler to aggregate by project and impact.


##### 9.3 Context seal for deep work

Job: `calendar_enforce_context_seal`

- Input: all events tagged as deep work blocks.
- Behaviour:
  - Detect overlapping or adjacent meetings/emails/interruptions.
  - Propose rescheduling or declining micro-meetings that break focus.
  - Optionally send instructions to an OS/Hardware engine to flip Do Not Disturb during these blocks.
- Output:
  - CalendarSuggestion patches.
  - Optional OS-level state changes (when that engine exists).


##### 9.4 Daily living engines interlock

Use the same pattern for Sous Chef, Homestead, Safety:

- Convert domain knowledge (meals, chores, safety windows) into time blocks with realistic constraints.
- Avoid stacking chores at energy lows; exploit patterns discovered by Taste Engine.

Each engine:

- Emits `CalendarSuggestion` rows.
- On acceptance, these become events with backlinks, allowing analytics like â€œhow much time did I spend cooking vs cleaning this month?â€.


#### 10. Google Calendar integration design

You personally use Google Calendar. Handshake should treat it as â€œexternal, low-res mirrorâ€ while the local Handshake calendar remains canonical.

##### 10.1 Integration stance

- Handshake is the **high-resolution brain**; Google is a **mirror** used at work and on mobile.
- Every event has an `export_mode`:
  - `local_only`   â†’ never leaves Handshake.
  - `busy_only`    â†’ mirrored as anonymous busy slots.
  - `full_export`  â†’ mirrored with full details.
- Sync operates per `CalendarSource` with explicit user control.


##### 10.2 Authentication and configuration

- Use OAuth 2.0 for installed apps to obtain refresh tokens for Google Calendar API.
- Store tokens encrypted on disk in the Tauri host, referenced by `CalendarSource`.
- Internal RPC endpoints:
  - `/calendar/google/auth/start`
  - `/calendar/google/auth/complete`
- For each Google calendar you enable:
  - Create a `CalendarSource` with `type="google"` and the providerâ€™s `calendar_id`.


##### 10.3 Pull: Google â†’ Handshake

Use Googleâ€™s `events.list` with incremental sync:

- Initial sync:
  - Call `events.list` with `timeMin`, `timeMax`, `singleEvents=true`.
  - Expand recurring events into instances.
  - Upsert corresponding `CalendarEvent` rows with `source_id` and `external_id` set.
  - Store `sync_token` returned by Google.

- Incremental sync:
  - Call `events.list` with `syncToken=...`.
  - Process new, updated, and cancelled events.
  - Update or delete local `CalendarEvent` entries accordingly.

- If sync token is invalid (410 GONE):
  - Clear external mapping for that source and re-run initial sync.

All of this runs inside `calendar_sync_google`, a specialization of `calendar_sync` with provider â€œgoogleâ€. It is capability-gated and fully logged.


##### 10.4 Push: Handshake â†’ Google

When a `CalendarEvent` with a Google `source_id` and non-`local_only` export mode changes:

- If `external_id` is null:
  - Call `events.insert` to create a new Google event.
- Else:
  - Call `events.update` or `events.patch` with the known `external_id` and `external_etag` for conflict detection.

Export mode logic:

- `local_only`:
  - Never call Google; event lives only in Handshake.
- `busy_only`:
  - On Google:
    - `summary = "Busy"`
    - `description = ""` (or minimal)
    - Only time and busy/free status are meaningful.
- `full_export`:
  - On Google:
    - `summary = CalendarEvent.title`
    - `description = subset of CalendarEvent.description` (redacted if needed)

Deletions:

- If event is removed in Handshake and has `external_id`, call `events.delete`.


##### 10.5 Read-only imports

Some sources (e.g. work calendar) may be used in **read-only** mode:

- `CalendarSource` has a flag `write_back=false`.
- `calendar_sync_google` for that source only pulls; it never calls insert/update/delete.
- Local events representing these are labeled clearly as â€œexternal / read-onlyâ€ in the UI.


##### 10.6 UI behaviours (RBC)

- Source filters:
  - Checkboxes/toggles for each `CalendarSource` (â€œHandshake / Localâ€, â€œGoogle / Personalâ€, â€œGoogle / Workâ€).
- Export badges:
  - `local_only`   â†’ lock icon.
  - `busy_only`    â†’ hollow/busy icon.
  - `full_export`  â†’ normal event styling.
- Conflict indicators:
  - Events imported from Google can be shown with distinct color or border.
  - Context menu actions:
    - â€œAttach notes docâ€
    - â€œLink tasksâ€
    - â€œMirror as rich local eventâ€ (create a local twin with more context).


##### 10.7 AI job examples with Google + Handshake

1. `calendar_autobook_with_google`
   - Inputs:
     - Participants (emails)
     - Duration
     - Constraints
     - Target `CalendarSource` (e.g. `google:personal`, `google:work`)
   - Behaviour:
     - Sync Google source.
     - Build merged busy map (Handshake + Google).
     - Use `calendar_suggest_slots` to propose options.
     - On user confirmation:
       - Create local CalendarEvent with full details.
       - Mirror to Google via `events.insert` using chosen export mode.
       - Optionally create/attach notes doc and draft invite mail.

2. `calendar_import_google_readonly`
   - Inputs: `CalendarSource.id` for work calendar.
   - Behaviour:
     - Run `calendar_sync_google` in read-only mode.
     - Tag events as read-only external; allow linking notes and tasks locally.

Net effect:

- At home: use the Handshake calendar as the canonical, context-rich planner.
- At work/on phone: see a lighter representation via Google, with good-enough detail.



##### 10.8 Conflict resolution and idempotency (Google mirror)

Google Calendar introduces two practical constraints: incremental sync tokens and ETag-based concurrency.

**Pull (Google â†’ Handshake)**

- Treat every pulled event as an upsert keyed by `(source_id, external_id)`.
- Store `external_etag` and `external_updated_at`.
- If the event is from a `read_only_import` source, never generate local patches that attempt to modify it.

**Push (Handshake â†’ Google)**

- Use `If-Match: external_etag` when updating/deleting events to avoid lost updates.
- All push operations are written to the persisted outbox with an `idempotency_key` (see Â§3.2).

**Conflict detection**

- If Google returns a precondition failure (etag mismatch) or the pulled `external_etag` changed while a local edit exists â†’ open a `CalendarConflict`.
- No automatic merge of title/description/attendees; resolution is gated and explicit.

**Busy-only export safety**

- When `export_mode=busy_only`, outbound payload must not include:
  - titles, descriptions, attendees, or linked entity references.
- If a user changes export_mode from `busy_only` to `full_export`, this is treated as a privacy-sensitive action and should be explicitly gated.


#### 11. Calendar + Flight Recorder and Activity Spans

You plan to log everything: local models, tool calls, AI jobs, sessions, worksurfaces. The calendar should become the front-door onto this Flight Recorder.

##### 11.1 ActivitySpan and SessionSpan

Define generic temporal entities that represent recorded activity:

```text
ActivitySpan (DerivedContent)
- id
- kind (session | job | tool_call | model_run | meeting | etc.)
- start_ts
- end_ts
- duration
- workspace_id / project_id
- source_rid (Job/Tool/Doc/etc. RID)
- model_id / engine_id (for AI/model/tool spans)
- tags[] (phase, topic, domain, job_profile, etc.)
- metrics:
    - tokens_in
    - tokens_out
    - latency_ms
    - success (bool)
    - error_code (if any)
```

```text
SessionSpan (DerivedContent)
- id
- start_ts
- end_ts
- label ("Phase 0.5 backend validation", "Calendar spec research", ...)
- workspace_context (active worksurfaces, repo path, branch)
- dominant_job_profiles[]
- notes_ref (optional doc)
```

Every job, tool call, and model run writes an `ActivitySpan`. Periods of coherent work (you at the keyboard) are grouped into `SessionSpan`s.


##### 11.2 Calendar events as containers for activity



###### 11.2.1 Join semantics and attribution rules (deterministic)

A calendar block is a **time window**; activity is a set of spans. Attribution must be consistent.

**Overlap definition**

- Represent all spans as half-open intervals: `[start_ts, end_ts)`.
- A span â€œbelongsâ€ to an event if:
  - `span.start_ts < event.end_ts` AND `span.end_ts > event.start_ts` (any overlap), and
  - optional filters match (workspace_id/project_id/tag/source).

**Attribution modes**

- `overlap_seconds` (default): assign overlap duration to the event (supports partial overlap).
- `dominant_event`: assign the entire span to the event with the largest overlap.
- `manual_pin`: allow the user (or a gated job) to explicitly pin a span to an event; pinned wins over derived overlap.

**Multi-event overlap**

- If a span overlaps multiple events, `overlap_seconds` may allocate across events.
- Prevent double-counting at report time by enforcing a single attribution mode per report.

**Deterministic queries**

- All â€œshow activity for eventâ€ queries must specify:
  - attribution mode
  - overlap threshold (e.g., >= 60 seconds)
  - whether to include pinned spans only / derived only / both

Each `CalendarEvent` can act as a container/anchor for spans that occur in its time window:

```text
CalendarEvent
- ...
- activity_span_ids[] (spans that overlap this event)
- session_span_ids[]  (sessions that overlap this event)
```

UI behaviour:

- Clicking an event opens tabs:
  - â€œDetailsâ€ (time, title, links)
  - â€œNotesâ€ (primary doc)
  - â€œActivityâ€:
    - Nested timeline grouped by kind (sessions, jobs, tool calls, models).

You do not need perfect pre-linking: the orchestrator can always query `ActivitySpan WHERE [start,end] overlaps [event.start,end]`. Persisting ids is an optimisation for important blocks.


##### 11.3 Calendar range as a query surface

Any selection on the calendar (day, week, specific block) can be treated as a query into the Flight Recorder:

```text
Input: time_range + filters (kind, project, job_profile, model_id)
Query: ActivitySpan WHERE start_ts < to AND end_ts > from AND filters...  (interval overlap)
```

This enables:

- â€œShow me everything I did during this deep-work block.â€
- â€œWhat did orchestrator + codex do during this Phase 0.5 week?â€

Important ranges can then be â€œpromotedâ€ to named `CalendarEvent`s (â€œLogger v3 design sprintâ€) linked to the corresponding `SessionSpan`s.


##### 11.4 AI usage analytics per event and per day

Define usage metrics that aggregate ActivitySpans under a time range:

```text
AIUsageMetrics (DerivedContent)
- time_range (day | week | event)
- total_jobs
- total_tool_calls
- local_model_tokens
- cloud_model_tokens
- failures_by_engine{engine_id -> count}
- jobs_by_profile{profile -> count}
- time_spent_per_engine{engine_id -> minutes}
```

These can hang off:

- specific CalendarEvents (â€œthis deep-work block burned 20k tokens on ASRâ€)
- daily/weekly ranges (â€œthis week, 80% of AI calls were mechanical vs 20% LLMâ€)

Calendar becomes:

- visual layer for time,
- plus per-block and per-day AI usage dashboards.


##### 11.5 Dataset slicing for personal model training

To train smaller personal models, you want to slice your logs by time and tags.

Pipeline:

```text
calendar_range -> events -> activity_spans -> filtered_traces -> training dataset
```

Concrete flow:

- Select a month + filter calendar events by tag `mechanical` or `spec_work`.
- Collect ActivitySpans under those events with `job_profile` in some set and `success=true` (thumbs-up or no error).
- From spans, gather:
  - prompts
  - tool call traces
  - outputs
  - your explicit feedback (thumbs up/down, corrections)
- Emit a dataset artifact for distillation/RLHF.

The calendar is the UI for selecting which parts of your life become training data.


###
### 11.5.13 Front End Memory System events (FR-EVT-MEM-*) (Normative) [ADD v02.138]

FEMS MUST emit dedicated events for proposal, review, commit, and pack-build so memory influence is debuggable and replayable.

**Privacy rule (HARD)**  
Memory events MUST NOT inline raw memory content. They MUST log **IDs/hashes** and artifact handles.

```ts
type MemoryEventCode =
  | "FR-EVT-MEM-001" // memory_write_proposed
  | "FR-EVT-MEM-002" // memory_write_reviewed (approved/rejected)
  | "FR-EVT-MEM-003" // memory_write_committed
  | "FR-EVT-MEM-004" // memory_pack_built
  | "FR-EVT-MEM-005" // memory_item_status_changed
  ;

type MemoryWriteProposedEvent = FlightRecorderEventBase & {
  event_code: "FR-EVT-MEM-001";
  proposal_id: string;
  proposal_hash: string;           // sha256(canonical proposal json)
  artifact_ref: ArtifactHandle;     // MemoryWriteProposal artifact
  scope_refs: EntityRef[];
  op_count: number;
  requires_review_count: number;
};

type MemoryWriteReviewedEvent = FlightRecorderEventBase & {
  event_code: "FR-EVT-MEM-002";
  proposal_id: string;
  decision: "approved" | "rejected" | "partial";
  reviewer_kind: "user" | "policy";
  commit_report_ref?: ArtifactHandle;
};

type MemoryWriteCommittedEvent = FlightRecorderEventBase & {
  event_code: "FR-EVT-MEM-003";
  commit_id: string;
  proposal_id: string;
  commit_report_hash: string;
  artifact_ref: ArtifactHandle;     // MemoryCommitReport artifact
  changed_memory_ids_hash: string;  // sha256(sorted memory ids)
};

type MemoryPackBuiltEvent = FlightRecorderEventBase & {
  event_code: "FR-EVT-MEM-004";
  pack_id: string;
  memory_pack_hash: string;
  artifact_ref: ArtifactHandle;     // MemoryPack artifact (may be local-only)
  memory_policy: "EPHEMERAL" | "SESSION_SCOPED" | "WORKSPACE_SCOPED";
  scope_refs: EntityRef[];
  item_count: number;
  token_estimate: number;
  truncation_occurred: boolean;
};

type MemoryItemStatusChangedEvent = FlightRecorderEventBase & {
  event_code: "FR-EVT-MEM-005";
  memory_id: string;
  previous_status: string;
  new_status: string;
  reason: "pin" | "unpin" | "invalidate" | "tombstone" | "supersede" | "merge";
  actor: "user" | "job" | "policy";
};
```

## 11.6 Debugging and regression analysis

When you change a model, config, or orchestrator behaviour, you can compare â€œbeforeâ€ and â€œafterâ€ windows.

Job: `CALENDAR_COMPARE_ACTIVITY_WINDOWS`

- Inputs:
  - `window_a` (start/end)
  - `window_b` (start/end)
  - filters (job profiles, project, model id)
- Behaviour:
  - Fetch ActivitySpans in both ranges.
  - Compute metrics (success rates, latency, token usage, error codes).
  - Produce diff + narrative summary.
- Output:
  - Derived comparison report attached to both windows.

UI:

- Select two date ranges in the calendar.
- Run comparison job.
- Inspect â€œBefore vs Afterâ€ changes for specific engines/models.


##### 11.7 Calendar as meta-orchestrator (policy profiles)

You can use the calendar as a policy surface that shapes how the orchestrator behaves in different time blocks.

Extend `CalendarEvent` with a policy profile:

```text
CalendarEvent
- ...
- policy_profile_id (e.g. FOCUS_MODE | EXPERIMENT_MODE | NORMAL)
```

Examples:

- `FOCUS_MODE`:
  - Prefer local models.
  - Suppress non-essential tool calls.
  - Forbid long-latency cloud calls unless explicitly requested.
  - Tighten logging of failures, but reduce UI noise.

- `EXPERIMENT_MODE`:
  - Allow experimental models/tools.
  - Run additional evaluations/ablation when possible.
  - Log more detail for later analysis.

The orchestrator, when scheduling jobs, looks up the active CalendarEvent (if any) and applies the corresponding policy profile to routing and capability checks.


##### 11.8 Tool evaluation windows

Define dedicated evaluation blocks as CalendarEvents tagged for a specific engine/tool:

```text
CalendarEvent
- title: "ASR evaluation window"
- tags: ["EVAL_WINDOW", "ASR"]
```

Job: `EVAL_RUN_FOR_WINDOW`

- Inputs:
  - engine/tool id
  - time window (eventâ€™s start/end)
- Behaviour:
  - Filter ActivitySpans in that window for the targeted engine.
  - Compute error rates, coverage, and user feedback stats.
  - Emit a Derived evaluation report with recommendations.

This lets you schedule evaluation sessions in time, then automatically summarise how that engine behaved in that block.


##### 11.9 Chronicle and life-log integration

Chronicle becomes the narrative layer over Calendar + ActivitySpans.

Nightly job: `CHRONICLE_WRITE_DAY_SUMMARY`

- Inputs:
  - date D
- Behaviour:
  - Gather CalendarEvents for D.
  - Gather SessionSpans and ActivitySpans within D.
  - Generate:
    - narrative summary (what you did)
    - metrics (time coding, time in research, time wrangling tools, tokens burned)
    - highlight events (big wins/failures)
  - Write a `ChronicleDay` entry:

```text
ChronicleDay
- date
- summary_text
- highlight_event_ids[]
- metrics:
    - hours_coding
    - hours_research
    - hours_debugging
    - tokens_cloud
    - tokens_local
- links[] (CalendarEvents, docs, tasks)
```

You can then query:

- â€œShow me all high-productivity days last month.â€
- â€œFind days with many ASR failures.â€
- â€œFind days where I spent > 4h in meetings and < 1h coding.â€


##### 11.10 Safety, privacy, and boundaries

Because this is powerful, you need explicit control over what is visible and what is used for training or cloud calls.

Concepts:

- Classification and redaction:
  - Some ActivitySpans are marked â€œsensitiveâ€; only local models can see them.
  - Cloud models see only aggregates or redacted views by default.

- Training opt-out:
  - Spans, SessionSpans, and Chronicle entries can carry a flag `exclude_from_training=true`.
  - Dataset-building jobs must honour this flag.

- Aggregation levels:
  - Calendar-driven analytics jobs default to aggregate metrics when cloud is involved (counts, ratios, durations) rather than raw prompts or logs.

This keeps the calendar + Flight Recorder integration powerful but under your explicit control.


##### 11.11 Minimal implementation steps

To make this real in early Handshake versions:

1. Add `ActivitySpan` and `SessionSpan` entity definitions.
2. Instrument the orchestrator so every job, tool call, and model run writes an ActivitySpan.
3. Implement basic calendar â†’ ActivitySpan query:
   - select time range in UI â†’ show spans grouped by kind.
4. Add a first analytics job:
   - `CALENDAR_ACTIVITY_SUMMARY` for a day/week (counts, tokens, time).
5. Later, add policy profiles and evaluation windows on top.


#### 12. Summary

- React Big Calendar is a strong fit as the calendar view: pure React, MIT, and visually close to mainstream calendars while leaving semantics to Handshake.
- Handshake defines its own `CalendarEvent`, `CalendarSource`, and derived entities so that calendar data is just another part of the workspace graph.
- All mutations and AI behaviours are expressed as AI Jobs running through the Workflow Engine, with capabilities, gates, and Flight Recorder logging.
- The calendar connects deeply to docs, tasks, mail, Sous Chef, Homestead, Archivist, Chronicle, Analyst, Wrangler, and Taste Engine to become a force multiplier for time allocation, not just a schedule.
- Google Calendar integration is explicit: Handshake remains canonical; Google is a sync target or read-only source, controlled per event and per calendar via `export_mode` and capability profiles.
- MCP can be used both to consume external calendars and to expose Handshakeâ€™s calendar to other agents under the same rules.
- By wiring ActivitySpans and SessionSpans into the calendar, the calendar turns into the primary lens onto the Flight Recorder: you can inspect, debug, and compare everything your models and tools did in any time window.
- Calendar-driven analytics enable per-block and per-day AI usage metrics, regression analysis, evaluation windows, and policy profiles that change orchestrator behaviour based on time and intent.
- Chronicle ties it together into a human-readable life log, giving you narrative plus metrics for how you actually spent your time and how your AI stack behaved.

### 10.4.2 Calendar â†” ACE Integration (v0.1)

**Purpose**  
This section defines how the Calendar surface participates in the ACE runtime without breaking ACE invariants (compiled context, caching discipline, artifact-first, sub-agent isolation) and without duplicating the normative Calendar Law in Â§10.4.1.

**Scope note**
- ACE runtime is mandatory and authoritative. Calendar-driven ACE is optional and acts only as an integration layer (scope hint, policy selector at job boundaries, audit lens); it must not replace ACE runtime primitives.

**Authority / non-drift note**
- Â§10.4.1 (Calendar Law) remains normative for calendar semantics, sync, redaction, and external write rules.
- Â§2.6.6.7 (ACE Runtime) remains normative for context compilation, PromptEnvelope, ContextSnapshot, compaction, and validators.
- If this section conflicts with Â§10.4.1 or Â§2.6.6.7, the higher-precedence LAW section wins.

#### 10.4.2.0 Design principles (Calendar-driven ACE v0.3)
- **P1 â€” Weak signal, never hard filter:** Calendar boosts retrieval; it never pins context.
- **P2 â€” Projection + redaction by default:** Cloud defaults to `minimal`/`analytics_only`; elevation is explicit and logged.
- **P3 â€” Deterministic compiled context:** Same inputs â‡’ same scope_hint, retrieval order, and PromptEnvelope hashes.
- **P4 â€” Mixed-mode stability:** Routing (local/cloud/mixed) cannot flip mid-job; policy changes happen at job boundaries.
- **P5 â€” Untrusted calendar content is sanitized:** Titles/descriptions/attendees treated as untrusted; injected instructions are blocked.

**Projection/redaction defaults**

Table A â€” Event field exposure by projection (model-visible):
```
minimal:        id, time_range, tags, policy_profile_id?, linked_entity_refs, trust_level
analytics_only: minimal + aggregates (counts/durations), no titles/descriptions/attendees
full:           minimal + title/description/attendees/location (local only by default)
```

Table B â€” Cloud hardening defaults:
```
Cloud default: projection = minimal or analytics_only
Elevation to full: explicit user elevation + capability + consent; logged in ContextSnapshot
```

Table C â€” Required redaction transforms:
```
Strip/obfuscate: emails, phone numbers, URLs in calendar titles/descriptions/attendees for cloud runs
Replace with typed placeholders; keep SourceRefs for reversibility
```

#### 10.4.2.1 CalendarScopeHint (the only calendar-derived object the Context Compiler may consume)

A CalendarScopeHint is a small, typed hint passed to the Context Compiler and logged in ContextSnapshot metadata. It is a *boost signal* and *policy selector* â€” never a hard retrieval gate.

```text
CalendarScopeHint (DerivedContent, ephemeral)
- time_range: [start_ts, end_ts)
- active_event_id?: CalendarEvent.id
- source: (active_event | manual_override | none)
- policy_profile_id?: string              // see Â§10.4.1 â€œpolicy profilesâ€ and Â§10.4.2.3 precedence
- projection: (minimal | full | analytics_only)
- sensitivity_class: (low | medium | high)
- linked_entity_refs[]: EntityRef         // docs/tasks/projects explicitly linked to event
- scope_boost_terms[]: string             // tags/project keys only; never raw title/description
- trust_level: (local_authoritative | external_import | unknown)
- confidence: float                       // 0..1; deterministic (see Â§10.4.2.4)
```

**Projection and redaction**
- The CalendarContext provider, capability matrix, and projection/redaction rules are defined in Â§10.4.1 (Local vs cloud models using the calendar).
- CalendarScopeHint MUST obey those rules; for cloud runs default to `minimal` or `analytics_only` unless explicitly elevated by a gated job.

#### 10.4.2.2 ACE compatibility invariants (calendar must not break core runtime)

These invariants prevent calendar integration from breaking caching, compaction, and isolation:

**I1 â€” Prefix Stability (caching-safe)**
- CalendarScopeHint MUST only appear in the PromptEnvelope *variable suffix* as a `scope_hint` ContextBlock.
- StablePrefix MUST NOT include raw event title/description/attendees/location/links.

**I2 â€” Retrieval Beats Pinning**
- CalendarScopeHint may only boost candidate scoring; it MUST NOT pin facts into context.
- Only global criticals (safety rules, active blockers, hard constraints) may be pinned, independent of calendar.

**I3 â€” Compaction and Reversibility**
- Calendar text MUST NOT be promoted into LongTermMemory as unqualified â€œfacts.â€
- Any compaction referencing calendar MUST do so via `event_id` / `time_range` / linked artifact handles and retain SourceRefs.

**I4 â€” Sub-agent Isolation**
- Sub-agents MUST NOT share a transcript or â€œevent blob.â€
- Each sub-agent receives the same minimal CalendarScopeHint plus its own scoped EntityRefs.
- Communication is artifacts + typed summaries only; each agent emits ContextSnapshot.

**I5 â€” Strategy/Playbook Evolution**
- Calendar may select which policy/playbook applies (by `policy_profile_id`), but MUST NOT auto-mutate playbooks.
- Playbook updates require explicit proposal + eval + promotion gate (ACE runtime).

**I6 â€” Tool Surface Minimization**
- Calendar-related tool I/O must be normalized: heavy payloads are artifacts, model sees bounded summaries + handles.
- Raw invite bodies and long descriptions are artifacts, not prompt inline.

**I7 â€” Job-Boundary Routing**
- Calendar policy changes MUST NOT change model tier/projection mid-job.
- Policy changes take effect only at the next job boundary (ACE runtime JobBoundaryRoutingGuard).

**I8 â€” Cloud Leakage**
- Default cloud posture is `minimal` or `analytics_only`.
- Any elevation to full details must be explicit, gated, and logged.

#### 10.4.2.2a Security and abuse resistance (v0.3 alignment)
- **Prompt injection hardening:** Calendar titles/descriptions/attendees treated as untrusted; PromptInjectionGuard must scan/redact before PromptEnvelope assembly.
- **Privacy leakage to cloud:** Projection defaults enforced per Table A-C; non-exportable artifacts must block cloud routing unless explicitly elevated.
- **Over-coupling scope to time:** CalendarScopeHint boosts only; cannot gate retrieval or hide non-calendar-critical evidence.
- **Conflicts/overlaps:** Deterministic active-event selection (see Â§10.4.2.4) plus tie-breakers; overlaps do not merge scope.
- **False attribution:** Compactions and LTM promotions must cite `event_id`/`time_range` SourceRefs; no unqualified fact promotion.

#### 10.4.2.3 PolicyProfile resolution and precedence (deterministic)

Calendar policies influence routing/tooling/logging, but must resolve deterministically per job boundary.

**Precedence order (highest wins)**
1. Explicit scope override policy (session override)
2. AI Job profile policy (job_kind/profile binding)
3. CalendarEvent.policy_profile_id (active event)
4. Workspace default policy

**Conflict rule**
- Higher-precedence policies may restrict capabilities/projection; lower-precedence policies cannot widen them.

#### 10.4.2.4 Deterministic confidence function (active event selection)

This function is used only when multiple events overlap â€œnowâ€ and the orchestrator must pick a default active event. It MUST be deterministic.

Inputs:
- `pin` (bool): user pinned / manual override
- `overlap_seconds`: overlap between resolver window and event interval
- `resolver_window_seconds`: constant (default 900)
- `conflict` (bool): event overlap conflict state
- `trust_level`: local_authoritative | external_import | unknown
- `busy_only` (bool): non-actionable placeholder
- `age_seconds`: staleness since last successful sync/update

Constants:
- trust weights: local_authoritative=1.0, external_import=0.85, unknown=0.70
- conflict penalty: 0.20
- busy_only penalty: 0.15
- staleness penalty: min(0.25, (age_seconds / 86400) * 0.10)

Computation:
1) If `pin=true` â†’ confidence = 1.0  
2) Else:
- overlap_frac = clamp(overlap_seconds / resolver_window_seconds, 0..1)
- base = overlap_frac * trust_weight(trust_level)
- confidence = clamp(base - conflict_penalty - busy_only_penalty - staleness_penalty, 0..1)

Tie-breaker:
- earliest start_ts, then lexicographic event_id.

#### 10.4.2.5 Logging requirements (Calendar â†” ContextSnapshot)

For any job step using CalendarScopeHint, the ContextSnapshot MUST record:
- sanitized CalendarScopeHint (IDs/tags only; projection-compliant)
- resolved policy_profile_id and precedence source
- routing outcome (local/cloud/mixed) and projection mode used
- active_event_id and confidence score used by CalendarScopeResolver (including resolver constants)
- whether a ScopeOverride was applied and by whom (user/session/system)
- redaction/projection table version + transforms applied (e.g., placeholder substitutions)
- any blocked fields/attachments (prompt-injection/redaction guard) and reason
- candidate source hashes if CalendarScopeHint influenced retrieval scoring

This enables auditing â€œwhat the model sawâ€ without exporting raw calendar details.

#### 10.4.2.6 Acceptance tests (minimum)

- CalendarScopeHint never appears in StablePrefix (prefix stability test).
- Cloud runs never receive raw event title/description by default (projection test).
- Active event selection is deterministic with identical inputs (confidence test).
- Policy precedence cannot be bypassed by calendar metadata (precedence test).
- Job-boundary routing: model tier/projection cannot change mid-job (guard test).
- Redaction transforms applied before cloud routing (Table C compliance test).
- Prompt-injection guard blocks untrusted calendar instructions (calendar payload test).

#### 10.4.2.7 Minimal implementation steps (v0.3 parity)
- Implement CalendarScopeResolver with deterministic confidence function and logging.
- Enforce projection/redaction defaults for cloud vs local; ship Table A-C in code.
- Wire PolicyProfile precedence (override â†’ job profile â†’ event policy â†’ workspace default) and log source.
- Add acceptance tests above to CI, plus CalendarScopeHint + ContextSnapshot schema validation.


## 10.5 Operator Consoles: Debug & Diagnostics

### 10.5.0 Purpose

Handshake is a local-first AI workspace where a significant share of implementation work is performed by LLM â€œcodersâ€.
This section defines a rigid, operator-friendly Debug & Diagnostics system that lets a non-coder reliably:

- Detect failures and regressions
- Capture deterministic evidence
- Correlate evidence to jobs/tools/connectors/indexes
- Export a redaction-safe â€œDebug Bundleâ€ that an LLM coder can act on

This section is UI-agnostic. Monaco and other UIs are **adapters**. The underlying contracts (diagnostics, Flight Recorder linkability, bundle export) MUST remain stable even if UI components are replaced.

[ADD v02.159] Operator Consoles are the specialized evidence and diagnostics surfaces within the broader Dev Command Center projection umbrella. Problems, Jobs, Timeline, and Evidence views own drilldown, evidence selection, and export-launch behavior; they do not replace the control and orchestration role of Dev Command Center.

[ADD v02.161] Operator Consoles continue to own drilldown, evidence selection, and export-launch behavior, but Dev Command Center MUST project long-lived Governance Pack export status, Workspace Bundle export status, diagnostics query state, and workflow-linked evidence packaging by stable backend identifiers rather than drawer-local polling state.

[ADD v02.184] Kernel V1 Dev Command Center, diagnostics, Timeline, Problems, Evidence Drawer, Debug Bundle, and trace inspector surfaces are projections over product authority. When they display Kernel V1 session, tool, artifact, validation, or promotion state, they MUST link to durable EventLedger event IDs plus `KernelTaskRun` and `SessionRun` IDs. These surfaces MUST NOT become scheduling, replay, validation, or promotion authority for Kernel V1.

### 10.5.1 Scope

In scope:

- Operator consoles: Problems, Jobs, Timeline (Flight Recorder), Evidence Drawer, Policy Inspector, Connector Health, Index Doctor, Debug Bundle export.
- A deterministic â€œtriage loopâ€ that produces a shareable, reproducible bug packet.
- Normative surface requirements (MUST/SHOULD/MUST NOT) and acceptance criteria.

Out of scope:

- Any specific frontend framework choice.
- A specific editor implementation (Monaco may be used, but is not required here).
- A full security model; this section references the capability/policy system defined elsewhere.
- Long-term archival formats beyond the Debug Bundle contract.

### 10.5.2 Normative language

The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, **MAY** are to be interpreted as in RFC 2119.

### 10.5.3 Non-negotiable principles

P-01. **One shape, many sources.** Diagnostics from LSP/validators/engines/connectors MUST be normalized into a single schema (canonical in Â§11.4).

P-02. **Correlate, donâ€™t guess.** The console MUST show link confidence (direct/inferred/ambiguous/unlinked) and MUST never present ambiguous correlations as certain.

P-03. **Evidence first.** Every console action that changes state (ack/mute/re-run/export/resync/rebuild) MUST emit a Flight Recorder event (canonical in Â§11.5).

P-04. **Redaction-safe by default.** Debug Bundle export MUST default to a redaction mode that cannot leak secrets/PII in typical usage.

P-05. **Deterministic fingerprints.** Problems grouping MUST be driven by a deterministic fingerprinting function (canonical in Â§11.4).

P-06. **Kernel authority linkability.** Kernel V1 diagnostic rows MUST expose EventLedger/run identifiers when they describe kernel execution. UI state, pinned timeline slices, and debug bundles are useful projections, but the backend EventLedger remains the replay and promotion authority.

### 10.5.4 The operator triage loop (fixed)

A compliant UI MUST support the following loop without requiring the operator to interpret logs:

1. Open **Problems** and filter `severity âˆˆ {error,fatal}`.
2. Select the top issue; open **Evidence Drawer**.
3. From Evidence, open the **Related Job** (if any) and its **Timeline slice**.
4. Review **Policy** (allowed/blocked capabilities) relevant to the failure.
5. Export **Debug Bundle** for the selected issue/time window/job.
6. Provide the generated **LLM coder prompt** + bundle to the coding agent.

### 10.5.5 Console surfaces

All surfaces below MUST deep-link to each other via `job_id`, `diagnostic_id`, `wsid`, and Flight Recorder event ids (see Â§11.4 and Â§11.5).

#### 10.5.5.1 Problems

MUST:
- Render a table of normalized diagnostics (canonical schema: Â§11.4).
- Provide filters for: `severity`, `source`, `surface`, `wsid`, `job_id`, `time_range`.
- Group by deterministic `fingerprint` (see Â§11.4), while retaining access to raw instances.
- Support: open/ack/mute/resolved statuses (local-only metadata is permitted).
- Open Evidence Drawer on selection.

SHOULD:
- Show `count`, `first_seen`, `last_seen`.
- Show correlation quality (`link_confidence`) and provide â€œwhy linked?â€ explanation.

MUST NOT:
- Hide or drop raw diagnostic instances when recomputing the Problems index.

#### 10.5.5.2 Jobs

MUST:
- List jobs with filters: status, kind, workspace (`wsid`), time range.
- Provide a Job Inspector with tabs: Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, Policy.
- Allow exporting a Debug Bundle scoped to a job.

SHOULD:
- Provide â€œclone + rerunâ€ in sandbox mode (subject to capability policy).

MUST NOT:
- Allow running privileged actions without an explicit policy decision being visible in the Policy tab.

#### 10.5.5.3 Timeline (Flight Recorder)

MUST:
- Render a time-window view over Flight Recorder events (canonical: Â§11.5).
- Provide filters: job_id, wsid, actor, surface, event types.
- Allow opening Evidence Drawer for any event.
- Support â€œpin this sliceâ€ (stable query) for bundle export.

SHOULD:
- Provide â€œexpand contextâ€ affordances (e.g., show preceding N seconds/events).

#### 10.5.5.4 Evidence Drawer (shared detail view)

MUST:
- Show a single â€œevidence cardâ€ for a selected diagnostic or event:
  - raw JSON (redacted view default),
  - linked entities (job, wsid, spans),
  - relevant policy/capability decisions,
  - related artifacts by hash,
  - link_confidence and correlation explanation.
- Provide â€œExport Debug Bundleâ€ entrypoint.

SHOULD:
- Provide a â€œcopy as coder promptâ€ action (see 10.5.6.4).

#### 10.5.5.5 Capability & Policy Inspector + Simulator

MUST:
- Show declared vs granted vs active capabilities for: plugins, engines, connectors, models.
- Provide a simulator: â€œIf job X attempted tool Y with capability Z, would it be allowed?â€
- Record simulator runs as Flight Recorder events.

SHOULD:
- Provide a policy diff view (what changed since last run).

#### 10.5.5.6 Connector & Sidecar Health

MUST:
- Show connector/sidecar status, last sync, error counts, queue depth (where applicable).
- Provide â€œresync / restart / rebuildâ€ actions gated by policy.
- Emit Flight Recorder events for operational actions.

SHOULD:
- Provide a minimal â€œhealth summaryâ€ suitable for Debug Bundle inclusion.

#### 10.5.5.7 Index Doctor / Consistency Auditor

MUST:
- Show index backlog, stale derived artifacts, broken refs/orphans (where applicable).
- Provide â€œrebuild / backfill / reindexâ€ actions gated by policy.
- Emit Problems diagnostics for detected inconsistencies.
- Show retrieval evidence traces (QueryPlan + RetrievalTrace) linked to job steps, including: route_taken, cache hit/miss markers, selected IDs, spans, and truncation flags.
- Surface drift signals explicitly:
  - embedding drift (`source_hash` mismatch),
  - KG provenance missing/invalid,
  - LocalWebCacheIndex staleness/TTL vs pinning.
- Show per-source caps enforcement (max snippets per source, max snippets total) and diversity method metadata when used (e.g., MMR lambda).
SHOULD:
- Provide a â€œdry-runâ€ mode that reports what would change.

#### 10.5.5.8 Debug Bundle Export

MUST:
- Export a deterministic bundle containing:
  - job metadata (if scoped),
  - Flight Recorder slice,
  - [ADD v02.120] Role Mailbox ("Inbox") thread exports + mailbox thread summaries (if available),
  - [ADD v02.120] Work Profile selection history (FR-EVT-PROFILE-*),
  - normalized diagnostics involved,
  - environment summary (versions/config without secrets),
  - redaction report and retention summary,
  - generated LLM coder prompt.
- Default to redaction-safe mode.
- Emit a Flight Recorder `debug_bundle_export` event (Â§11.5).

SHOULD:
- Support bundle scopes: `problem`, `job`, `time-window`, `workspace`.

#### 10.5.5.9 Spec Session Log

MUST:
- Render a dedicated view of Spec Session Log entries (Task Board + Work Packet events) keyed by `spec_id`.
- Provide filters: spec_id, task_board_id, work_packet_id, actor, time range, governance_mode.
- Deep-link to SpecIntent, SpecRouterDecision, WorkPacketBinding, and related Flight Recorder events.
- Show the current status for linked Task Board items and Work Packets.

SHOULD:
- Provide a timeline mode that overlays Spec Session Log entries with the Flight Recorder timeline.
- Show mode transitions (GOV_LIGHT/GOV_STANDARD/GOV_STRICT) and safety commit status (git workflows only).

MUST NOT:
- Allow manual edits to Task Board or Work Packet data outside the governed workflow/job runtime.

Layout (suggested):
- Left rail: filters (spec_id, work_packet_id, governance_mode, time range).
- Main table: timestamp, event_type, actor, summary, task_board_id, work_packet_id, governance_mode.
- Detail drawer: linked SpecIntent, SpecRouterDecision, WorkPacketBinding, and artifact handles.

Wireframe (ASCII):
```
+----------------------+--------------------------------------------------------------+
| Filters              | Spec Session Log                                              |
| - spec_id            | +----------------------+---------------------------------------+ |
| - work_packet_id     | | timestamp            | summary                               | |
| - governance_mode    | | event_type           | actor                                 | |
| - time range         | | task_board_id        | work_packet_id                         | |
| - actor              | | governance_mode      | linked_artifacts (count)               | |
|                      | +----------------------+---------------------------------------+ |
+----------------------+--------------------------------------------------------------+
| Detail Drawer (linked SpecIntent, SpecRouterDecision, bindings, artifacts)   |
+-------------------------------------------------------------------------------+
```

Data contracts (normative):
```
SpecSessionLogQuery
- spec_id?: string
- task_board_id?: string
- work_packet_id?: string
- governance_mode?: GovernanceMode
- actor?: string
- time_range?: {start: Timestamp, end: Timestamp}

SpecSessionLogViewRow
- entry_id: string
- timestamp: Timestamp
- event_type: string
- actor: string
- summary: string
- spec_id: string
- task_board_id: string
- work_packet_id?: string
- governance_mode: GovernanceMode
- linked_artifacts?: ArtifactHandle[]
- link_refs?: {spec_intent_id?: string, spec_router_decision_id?: string, work_packet_id?: string}
```

### 10.5.6 Debug Bundle (export artifact)

#### 10.5.6.1 Goals

- Deterministic, shareable evidence packet that enables an LLM coder to act without asking follow-ups.
- Safe-by-default redaction, with explicit opt-in to less-redacted modes.

#### 10.5.6.2 Minimum structure

A bundle is a folder or zip with at least:

- `bundle_manifest.json`
- `env.json`
- `jobs.json` (or `job.json` if scoped to one job)
- `workflow_node_executions.jsonl` when `scope.kind = "workflow_run"` or `scope.kind = "workflow_node_execution"`
- `trace.jsonl` (Flight Recorder slice)
- `diagnostics.jsonl` (normalized diagnostics)
- `retention_report.json`
- `redaction_report.json`
- `repro.md`
- `coder_prompt.md`

#### 10.5.6.3 Redaction modes

- **SAFE_DEFAULT**: must remove secrets/PII patterns and replace raw payloads with hashes + minimal previews.
- **WORKSPACE**: may include more local context but MUST still redact secrets/PII.
- **FULL_LOCAL**: includes full payloads; MUST NOT be exportable unless policy explicitly allows.

#### 10.5.6.4 Generated LLM coder prompt (required)

The bundle MUST contain a prompt that includes:

- What failed (title + message)
- Exact version/build identifiers
- Time range and workspace/job ids
- Steps to reproduce (if known; otherwise â€œunknownâ€)
- Expected vs actual
- Direct links/ids: `diagnostic_id`, `job_id`, relevant `event_id`s
- Policy notes (allowed/blocked)
- What evidence is missing due to retention/redaction

#### 10.5.6.5 Bundle File Schemas (Normative)

All bundle files MUST conform to the schemas defined below. Schema violations MUST cause VAL-BUNDLE-001 to fail.

**v02.98 note:** Added normative file schemas, Rust trait, API endpoints, job profile, and frontend UI spec for Debug Bundle export [ilja291220250100].

##### 10.5.6.5.1 `bundle_manifest.json`

```typescript
interface BundleManifest {
  // Identity
  schema_version: "1.0";
  bundle_id: string;                    // uuid v4
  bundle_kind: "debug_bundle";
  created_at: string;                   // RFC3339 UTC

  // Scope
  scope: {
    kind: "problem" | "job" | "workflow_run" | "workflow_node_execution" | "time_window" | "workspace";
    problem_id?: string;                // when kind=problem
    job_id?: string;                    // when kind=job
    workflow_run_id?: string;           // when kind=workflow_run | workflow_node_execution
    workflow_node_execution_id?: string;// when kind=workflow_node_execution
    time_range?: {                      // when kind=time_window
      start: string;                    // RFC3339
      end: string;                      // RFC3339
    };
    wsid?: string;                      // always present if scoped to workspace
  };

  // Redaction
  redaction_mode: "SAFE_DEFAULT" | "WORKSPACE" | "FULL_LOCAL";

  // Provenance
  workflow_run_id: string;
  job_id: string;                       // export job itself
  exporter_version: string;             // semver
  platform: {
    os: string;
    arch: string;
    app_version: string;
    build_hash: string;
  };

  // Content inventory
  files: Array<{
    path: string;                       // relative path in bundle
    sha256: string;                     // hex-encoded
    size_bytes: number;
    redacted: boolean;                  // true if content was redacted
  }>;

  // Completeness
  included: {
    job_count: number;
    diagnostic_count: number;
    event_count: number;
    workflow_node_execution_count?: number;
  };
  missing_evidence: Array<{
    kind: "job" | "diagnostic" | "event" | "artifact";
    id: string;
    reason: "retention_expired" | "redacted" | "access_denied" | "not_found";
  }>;

  // Validation
  bundle_hash: string;                  // sha256 of normalized ZIP
}
```

[ADD v02.179] Workflow-correlated debug bundles MUST stay bounded by stable workflow lineage ids.

- `scope.workflow_run_id` and `scope.workflow_node_execution_id` identify the targeted workflow lineage for the exported evidence set.
- The top-level provenance `workflow_run_id` continues to identify the export job's own runtime instance.
- When `scope.kind = "workflow_run"`, `scope.workflow_run_id` MUST be present.
- When `scope.kind = "workflow_node_execution"`, both `scope.workflow_run_id` and `scope.workflow_node_execution_id` MUST be present.

##### 10.5.6.5.2 `env.json`

Environment context with mandatory redaction.

```typescript
interface BundleEnv {
  // Safe to include
  app_version: string;
  build_hash: string;
  platform: { os: string; arch: string; };
  rust_version: string;
  node_version?: string;

  // Workspace context (IDs only, no paths)
  wsid?: string;
  workspace_name?: string;              // redacted if contains PII

  // Runtime config (safe subset)
  config: {
    model_runtime: string;              // e.g., "ollama"
    default_model?: string;             // e.g., "llama3"
    flight_recorder_retention_days: number;
    // NO secrets, NO paths, NO env vars
  };

  // Feature flags (names only)
  feature_flags: string[];

  // Redaction note
  redaction_note: string;               // e.g., "Paths, env vars, and secrets removed per SAFE_DEFAULT policy"
}
```

**Redaction rules for `env.json`:**
- MUST NOT include: file paths, environment variables, API keys, tokens, database URLs
- MUST redact workspace paths to `[WORKSPACE_PATH]`
- MUST redact user home paths to `[HOME]`

##### 10.5.6.5.3 `jobs.json` / `job.json`

```typescript
// job.json when scope.kind = "job"
// jobs.json when scope.kind = "time_window" | "workspace" | "problem"

interface BundleJob {
  job_id: string;
  job_kind: string;
  protocol_id: string;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";

  // Timestamps
  created_at: string;
  started_at?: string;
  ended_at?: string;

  // Profile
  profile_id: string;
  capability_profile_id: string;

  // Context (IDs only in SAFE_DEFAULT)
  wsid?: string;
  doc_id?: string;

  // Inputs/Outputs as hashes (SAFE_DEFAULT) or previews (WORKSPACE)
  inputs_hash: string;
  outputs_hash?: string;
  inputs_preview?: string;              // first 200 chars, redacted
  outputs_preview?: string;             // first 200 chars, redacted

  // Error (if failed)
  error?: {
    code: string;
    message: string;                    // redacted for secrets
    diagnostic_id?: string;
  };

  // Metrics
  metrics?: {
    duration_ms?: number;
    tokens_in?: number;
    tokens_out?: number;
    model_name?: string;
  };

  // Links
  workflow_run_id?: string;
  parent_job_id?: string;
  diagnostic_ids: string[];
  event_ids: string[];                  // FR event IDs for this job
}

type BundleJobs = BundleJob[];
```

##### 10.5.6.5.4 `diagnostics.jsonl`

One JSON object per line, conforming to DIAG-SCHEMA-001 with redaction applied.

```typescript
interface BundleDiagnostic {
  // Core identity (from DIAG-SCHEMA-001)
  id: string;
  fingerprint: string;
  severity: "error" | "warning" | "info" | "hint";
  source: string;
  surface: string;
  code: string;
  title: string;
  message: string;                      // redacted for secrets/PII

  // Timestamps
  created_at: string;

  // Correlation
  wsid?: string;
  job_id?: string;
  workflow_run_id?: string;

  // Location (paths redacted to relative)
  file_path?: string;                   // relative to workspace root or [EXTERNAL]
  line_start?: number;
  line_end?: number;

  // Evidence
  link_confidence: "direct" | "inferred" | "ambiguous" | "unlinked";
  evidence_refs: string[];              // artifact hashes or IDs

  // Grouping context
  occurrence_count?: number;            // if grouped
  first_seen?: string;
  last_seen?: string;
}
```

##### 10.5.6.5.5 `trace.jsonl`

Flight Recorder event slice. One JSON object per line.

```typescript
// Each line is a FlightRecorderEvent (see FR-EVT-* schemas)
// with the following redaction applied:

// SAFE_DEFAULT:
// - payload fields replaced with hashes
// - paths replaced with [PATH]
// - env vars replaced with [ENV]

// WORKSPACE:
// - paths kept relative to workspace
// - payload previews (first 500 chars)

// FULL_LOCAL:
// - full payloads (requires explicit policy)
```

**Slice extraction rules:**
- If scope.kind = "job": all events with matching `job_id`
- If scope.kind = "problem": all events linked to the diagnostic + its job (if any)
- If scope.kind = "time_window": all events in range, up to 10,000 events (paginate if more)
- If scope.kind = "workspace": all events for wsid in last 24h, up to 10,000 events

##### 10.5.6.5.6 `retention_report.json`

```typescript
interface RetentionReport {
  report_generated_at: string;          // RFC3339

  retention_policy: {
    flight_recorder_days: number;       // default 7
    diagnostics_days: number;           // default 30
    job_metadata_days: number;          // default 30
  };

  // What was available
  available: {
    jobs: number;
    diagnostics: number;
    events: number;
  };

  // What was lost to retention
  expired: {
    jobs: Array<{ job_id: string; expired_at: string }>;
    diagnostics: Array<{ diagnostic_id: string; expired_at: string }>;
    event_ranges: Array<{ start: string; end: string; count: number }>;
  };

  // Gaps in evidence
  evidence_gaps: Array<{
    kind: string;
    description: string;
    impact: "high" | "medium" | "low";
  }>;
}
```

##### 10.5.6.5.7 `redaction_report.json`

```typescript
interface RedactionReport {
  redaction_mode: "SAFE_DEFAULT" | "WORKSPACE" | "FULL_LOCAL";
  report_generated_at: string;

  // Detectors used
  detectors: Array<{
    id: string;                         // e.g., "secret_api_key"
    version: string;
    patterns_count: number;
  }>;

  // Redaction summary
  summary: {
    files_scanned: number;
    files_redacted: number;
    total_redactions: number;
    by_category: Record<string, number>; // e.g., { "api_key": 3, "path": 12 }
  };

  // Redaction log (locations, not content)
  redactions: Array<{
    file: string;                       // bundle-relative path
    location: string;                   // e.g., "$.jobs[0].inputs_preview"
    category: string;                   // e.g., "api_key", "path", "pii"
    detector_id: string;
    replacement: string;                // e.g., "[REDACTED:api_key]"
  }>;

  // Policy decisions
  policy_decisions: Array<{
    item_kind: string;
    item_id: string;
    decision: "include" | "exclude" | "redact";
    reason: string;
  }>;
}
```

##### 10.5.6.5.8 `repro.md`

```markdown
# Reproduction Steps

## Environment
- App Version: {{app_version}}
- Build: {{build_hash}}
- Platform: {{os}} / {{arch}}
- Workspace: {{wsid}}

## Timeline
- First observed: {{first_seen}}
- Last observed: {{last_seen}}
- Occurrence count: {{count}}

## Steps to Reproduce
{{#if steps_known}}
1. {{step_1}}
2. {{step_2}}
...
{{else}}
Steps to reproduce are unknown. The following context may help:
- User action that triggered: {{trigger_action}}
- Active document/surface: {{active_context}}
{{/if}}

## Expected Behavior
{{expected}}

## Actual Behavior
{{actual}}

## Related Artifacts
- Job ID: {{job_id}}
- Diagnostic ID: {{diagnostic_id}}
- See `trace.jsonl` for full event sequence
```

##### 10.5.6.5.9 `coder_prompt.md`

```markdown
# Debug Bundle for LLM Coder

## Issue Summary
**Title:** {{diagnostic.title}}
**Severity:** {{diagnostic.severity}}
**Code:** {{diagnostic.code}}

## Message
{{diagnostic.message}}

## Context
- **Workspace ID:** {{wsid}}
- **Job ID:** {{job_id}}
- **Diagnostic ID:** {{diagnostic_id}}
- **Time Range:** {{time_range.start}} to {{time_range.end}}

## Version Information
- App: {{env.app_version}} ({{env.build_hash}})
- Platform: {{env.platform.os}} / {{env.platform.arch}}
- Model Runtime: {{env.config.model_runtime}}

## What Failed
{{#if job}}
Job `{{job.job_kind}}` ({{job.job_id}}) ended with status `{{job.status}}`.
{{#if job.error}}
Error: {{job.error.code}} - {{job.error.message}}
{{/if}}
{{/if}}

## Steps to Reproduce
See `repro.md` for detailed reproduction steps.

## Expected vs Actual
- **Expected:** {{expected_behavior}}
- **Actual:** {{actual_behavior}}

## Evidence Files
| File | Description |
|------|-------------|
| `jobs.json` | Job metadata and status |
| `diagnostics.jsonl` | Normalized diagnostics ({{diagnostic_count}} entries) |
| `trace.jsonl` | Flight Recorder events ({{event_count}} entries) |
| `workflow_node_executions.jsonl` | Workflow node lineage inventory for workflow-correlated exports |
| `env.json` | Environment context (redacted) |
| `retention_report.json` | Evidence availability |
| `redaction_report.json` | What was redacted |

## Key IDs for Investigation
- Diagnostic ID: `{{diagnostic_id}}`
- Job ID: `{{job_id}}`
- Workflow Run ID: `{{workflow_run_id}}`
- Workflow Node Execution ID: `{{workflow_node_execution_id}}`
- Event IDs: {{#each event_ids}}`{{this}}`{{#unless @last}}, {{/unless}}{{/each}}

## Policy Notes
{{#if policy_notes}}
{{#each policy_notes}}
- {{this}}
{{/each}}
{{else}}
No policy restrictions applied.
{{/if}}

## Missing Evidence
{{#if missing_evidence.length}}
The following evidence is unavailable:
{{#each missing_evidence}}
- **{{kind}}** `{{id}}`: {{reason}}
{{/each}}
{{else}}
All requested evidence is included.
{{/if}}

## Instructions for Coder
1. Start by reading this prompt and understanding the issue
2. Examine `jobs.json` for the failing job's context
3. Search `diagnostics.jsonl` for related errors
4. Trace the event sequence in `trace.jsonl`
5. Check `retention_report.json` for any evidence gaps
6. Propose a fix based on the evidence
```

##### 10.5.6.5.10 `workflow_node_executions.jsonl`

Required when `scope.kind = "workflow_run"` or `scope.kind = "workflow_node_execution"`.

Each line MUST be one JSON object:

```typescript
interface WorkflowNodeExecutionBundleRecord {
  workflow_node_execution_id: string;
  workflow_run_id: string;
  node_id: string;
  status: string;
  started_at: string;
  finished_at?: string;
  job_id?: string;
  input_sha256?: string;
  output_sha256?: string;
}
```

#### 10.5.6.6 DebugBundleExporter Trait (Normative Rust)

```rust
/// HSK-TRAIT-005: Debug Bundle Exporter
///
/// Normative contract for debug bundle export operations.
/// Implementations MUST:
/// - Apply redaction per the requested mode
/// - Emit FR-EVT-005 on export completion
/// - Run as a capability-gated job
/// - Produce deterministic output for identical inputs
/// - Accept `workflow_run` and `workflow_node_execution` as first-class bounded scopes
#[async_trait]
pub trait DebugBundleExporter: Send + Sync {
    /// Export a debug bundle for the given scope.
    ///
    /// # Arguments
    /// * `request` - Export parameters including scope and redaction mode
    ///
    /// # Returns
    /// * `Ok(manifest)` - Bundle manifest on success
    /// * `Err(BundleExportError)` - On failure (partial exports recorded in error)
    async fn export(
        &self,
        request: DebugBundleRequest,
    ) -> Result<DebugBundleManifest, BundleExportError>;

    /// Validate an existing bundle for VAL-BUNDLE-001 compliance.
    ///
    /// # Arguments
    /// * `bundle_path` - Path to bundle ZIP or directory
    ///
    /// # Returns
    /// * `Ok(report)` - Validation results with pass/fail and findings
    async fn validate(
        &self,
        bundle_path: &Path,
    ) -> Result<BundleValidationReport, BundleExportError>;

    /// List items available for export in the given scope.
    /// Used to populate export UI.
    /// Workflow-correlated inventory SHOULD expose workflow runs and workflow node executions
    /// when enough lineage exists to materialize a bounded bundle deterministically.
    async fn list_exportable(
        &self,
        filter: ExportableFilter,
    ) -> Result<ExportableInventory, BundleExportError>;
}

#[derive(Debug, Clone)]
pub struct DebugBundleRequest {
    pub scope: BundleScope,
    pub redaction_mode: RedactionMode,
    pub output_path: Option<PathBuf>,   // None = temp dir
    pub include_artifacts: bool,         // Include referenced artifact hashes
}

#[derive(Debug, Clone)]
pub enum BundleScope {
    Problem { diagnostic_id: String },
    Job { job_id: String },
    WorkflowRun { workflow_run_id: String },
    WorkflowNodeExecution {
        workflow_run_id: String,
        workflow_node_execution_id: String,
    },
    TimeWindow { start: DateTime<Utc>, end: DateTime<Utc>, wsid: Option<String> },
    Workspace { wsid: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionMode {
    SafeDefault,    // Maximum redaction, safe to share
    Workspace,      // Local context included, secrets still redacted
    FullLocal,      // Full payloads, requires explicit policy
}

#[derive(Debug, ThisError)]
pub enum BundleExportError {
    #[error("HSK-400-INVALID-SCOPE: Invalid export scope: {0}")]
    InvalidScope(String),

    #[error("HSK-403-CAPABILITY: Missing capability: {0}")]
    CapabilityDenied(String),

    #[error("HSK-404-NOT-FOUND: {kind} not found: {id}")]
    NotFound { kind: String, id: String },

    #[error("HSK-409-POLICY: Export blocked by policy: {0}")]
    PolicyDenied(String),

    #[error("HSK-500-EXPORT: Export failed: {0}")]
    ExportFailed(String),

    #[error("HSK-500-IO: IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct BundleValidationReport {
    pub valid: bool,
    pub schema_version: String,
    pub findings: Vec<ValidationFinding>,
}

#[derive(Debug, Clone)]
pub struct ValidationFinding {
    pub severity: FindingSeverity,
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub path: Option<String>,   // JSON path within file
}

#[derive(Debug, Clone, Copy)]
pub enum FindingSeverity {
    Error,      // Fails validation
    Warning,    // Passes but notable
    Info,       // Informational
}
```

#### 10.5.6.7 API Endpoints

##### POST `/api/bundles/debug/export`

Initiate a debug bundle export. Runs as a job.

**Request:**
```typescript
interface ExportRequest {
  scope: {
    kind: "problem" | "job" | "time_window" | "workspace";
    problem_id?: string;
    job_id?: string;
    time_range?: { start: string; end: string };
    wsid?: string;
  };
  redaction_mode: "SAFE_DEFAULT" | "WORKSPACE" | "FULL_LOCAL";
}
```

**Response (202 Accepted):**
```typescript
interface ExportResponse {
  export_job_id: string;        // Job ID for tracking
  status: "queued" | "running";
  estimated_size_bytes?: number;
}
```

##### GET `/api/bundles/debug/:bundle_id`

Get bundle manifest and status.

**Response (200 OK):**
```typescript
interface BundleStatus {
  bundle_id: string;
  status: "pending" | "ready" | "expired" | "failed";
  manifest?: BundleManifest;    // Present when status=ready
  error?: string;               // Present when status=failed
  expires_at?: string;          // RFC3339, when bundle will be deleted
}
```

##### GET `/api/bundles/debug/:bundle_id/download`

Download bundle as ZIP.

**Response:** `application/zip` stream with `Content-Disposition: attachment`

##### GET `/api/bundles/debug/exportable`

List items available for export.

**Query params:** `wsid`, `start`, `end`, `limit`

**Response:**
```typescript
interface ExportableInventory {
  jobs: Array<{ job_id: string; job_kind: string; status: string; created_at: string }>;
  diagnostics: Array<{ diagnostic_id: string; severity: string; title: string }>;
  workflow_runs: Array<{ workflow_run_id: string; status: string; started_at: string }>;
  workflow_node_executions: Array<{
    workflow_node_execution_id: string;
    workflow_run_id: string;
    node_id: string;
    status: string;
    started_at: string;
    finished_at?: string;
  }>;
  time_range: { earliest: string; latest: string };
}
```

##### POST `/api/bundles/debug/:bundle_id/validate`

Run VAL-BUNDLE-001 on a bundle.

**Response:**
```typescript
interface ValidationResponse {
  valid: boolean;
  findings: ValidationFinding[];
}
```

#### 10.5.6.8 Job Profile `debug_bundle_export_v0`

```typescript
const DEBUG_BUNDLE_EXPORT_PROFILE = {
  profile_id: "debug_bundle_export_v0",
  job_kind: "debug_bundle_export",
  protocol_id: "hsk.bundle.export.v0",

  // Required capabilities
  capabilities_required: [
    "export.debug_bundle",      // Core export capability
    "fr.read",                  // Read Flight Recorder
    "diagnostics.read",         // Read diagnostics
    "jobs.read",                // Read job metadata
  ],

  // Optional capabilities (for enhanced modes)
  capabilities_optional: [
    "export.include_payloads",  // For WORKSPACE/FULL_LOCAL modes
    "fs.write",                 // For custom output path
  ],

  // Constraints
  constraints: {
    max_bundle_size_mb: 100,
    max_events: 10000,
    max_jobs: 1000,
    max_diagnostics: 1000,
    timeout_seconds: 300,
  },

  // Status transitions
  status_transitions: [
    "queued -> running",
    "running -> completed",
    "running -> failed",
    "queued -> cancelled",
    "running -> cancelled",
  ],
};
```

#### 10.5.6.9 Secret Redactor Integration

##### Pattern Registry

The Secret Redactor MUST check for the following pattern categories:

| Category | Pattern ID | Examples |
|----------|------------|----------|
| API Keys | `secret_api_key` | `sk-...`, `api_...`, Bearer tokens |
| AWS | `secret_aws` | `AKIA...`, AWS secret keys |
| Database URLs | `secret_db_url` | `postgres://`, `mysql://`, connection strings |
| Private Keys | `secret_private_key` | `-----BEGIN RSA PRIVATE KEY-----` |
| Passwords | `secret_password` | `password=`, `passwd:`, credential patterns |
| Tokens | `secret_token` | JWT tokens, OAuth tokens, session tokens |
| PII Email | `pii_email` | Email address patterns |
| PII Phone | `pii_phone` | Phone number patterns |
| File Paths | `path_absolute` | Absolute paths (`C:\`, `/Users/`, `/home/`) |
| Env Vars | `env_var` | `$VAR`, `${VAR}`, `%VAR%` |

##### Redaction Output Format

Redacted content MUST be replaced with: `[REDACTED:<category>:<detector_id>]`

Examples:
- `[REDACTED:api_key:secret_api_key]`
- `[REDACTED:path:path_absolute]`
- `[REDACTED:pii:pii_email]`

##### Integration with Guard Engine

The Secret Redactor SHOULD delegate pattern detection to the Guard engine (`engine.guard.secret_scan`) when available, falling back to built-in patterns.

#### 10.5.6.10 Determinism & Hashing

##### ZIP Normalization

To ensure deterministic bundle hashes, ZIP creation MUST:
1. Sort entries alphabetically by path
2. Use fixed modification timestamps (Unix epoch 0 or bundle creation time)
3. Use DEFLATE compression level 6 (consistent)
4. Exclude OS-specific metadata (uid/gid, extended attributes)

##### Hash Algorithm

All hashes in bundles MUST use SHA-256, hex-encoded lowercase.

##### Bundle Hash Computation

`bundle_hash` in manifest = SHA-256 of:
1. Serialize manifest WITHOUT the `bundle_hash` field
2. Concatenate `\n` + SHA-256 of each file in sorted order
3. Hash the result

```
bundle_hash = sha256(
  json(manifest_without_hash) + "\n" +
  files.sorted_by(path).map(f => f.sha256).join("\n")
)
```

#### 10.5.6.11 Frontend UI Specification

##### Export Triggers

Debug Bundle export MUST be accessible from:
1. **Evidence Drawer** - "Export Debug Bundle" button
2. **Jobs View** - Context menu on job row -> "Export Debug Bundle"
3. **Problems View** - Context menu on problem row -> "Export Debug Bundle"
4. **Timeline View** - "Export time range" action after selecting a range
5. **Workflow Run History** - Context menu on workflow run row -> "Export Debug Bundle"
6. **Workflow Node Detail** - node execution action -> "Export Debug Bundle"

When stable workflow lineage ids are available, the scope selector MUST offer `workflow_run` and `workflow_node_execution` options instead of forcing operators to reconstruct scope from a broad time window.

##### Export Modal Flow

```
+---------------------------------------------+
| Export Debug Bundle                      [X]|
+---------------------------------------------+
|                                             |
| Scope: [Job: abc-123 v]                     |
|                                             |
| Redaction Mode:                             |
| (*) Safe Default (recommended)              |
|     Removes all secrets, PII, and paths     |
| ( ) Workspace                               |
|     Includes local context, redacts secrets |
| ( ) Full Local (requires policy)            |
|     Full payloads - do not share externally |
|                                             |
| Estimated size: ~2.4 MB                     |
| Events: 847 | Jobs: 1 | Diagnostics: 3      |
|                                             |
+---------------------------------------------+
|                    [Cancel]  [Export]       |
+---------------------------------------------+
```

##### Progress Display

```
+---------------------------------------------+
| Exporting Debug Bundle...                   |
+---------------------------------------------+
| [============--------] 60%                  |
|                                             |
| [x] Collecting job metadata                 |
| [x] Collecting diagnostics                  |
| [>] Extracting Flight Recorder events...    |
| [ ] Applying redaction                      |
| [ ] Generating coder prompt                 |
| [ ] Creating ZIP                            |
+---------------------------------------------+
|                              [Cancel]       |
+---------------------------------------------+
```

##### Completion

```
+---------------------------------------------+
| [x] Debug Bundle Ready                      |
+---------------------------------------------+
|                                             |
| Bundle ID: dbg-20251229-abc123              |
| Size: 2.4 MB                                |
| Files: 9                                    |
| Redactions applied: 47                      |
|                                             |
| The bundle will expire in 24 hours.         |
|                                             |
+---------------------------------------------+
|       [Copy Path]  [Open Folder]  [Done]    |
+---------------------------------------------+
```

#### 10.5.6.12 VAL-BUNDLE-001: Debug Bundle Completeness (Expanded)

The validator MUST check:

1. **Required files present:**
   - `bundle_manifest.json`
   - `env.json`
   - `jobs.json` OR `job.json`
   - `workflow_node_executions.jsonl` when `scope.kind = "workflow_run"` or `scope.kind = "workflow_node_execution"`
   - `trace.jsonl`
   - `diagnostics.jsonl`
   - `retention_report.json`
   - `redaction_report.json`
   - `repro.md`
   - `coder_prompt.md`

2. **Schema compliance:**
   - Each file parses without error
   - Required fields present per section 10.5.6.5.x schemas
   - Enum values are valid

3. **Internal consistency:**
   - All `job_id` references in `coder_prompt.md` exist in `jobs.json`
   - All `diagnostic_id` references exist in `diagnostics.jsonl`
   - All `event_id` references exist in `trace.jsonl`
   - `files` array in manifest matches actual bundle contents
   - All file hashes match actual content
   - `scope.workflow_run_id` is present when `scope.kind = "workflow_run"`
   - `scope.workflow_run_id` and `scope.workflow_node_execution_id` are present when `scope.kind = "workflow_node_execution"`
   - `included.workflow_node_execution_count` matches the number of lines in `workflow_node_executions.jsonl` when that file is present
   - A `workflow_node_execution` scoped bundle contains exactly one targeted node execution record and all listed node executions share the scoped `workflow_run_id`

4. **Redaction compliance:**
   - If `redaction_mode = SAFE_DEFAULT`:
     - No absolute paths in any file
     - No matches for secret patterns
     - No PII patterns
   - `redaction_report.json` lists all redactions applied

5. **Missing evidence accounting:**
   - Every item in `missing_evidence` has a valid `reason`
   - `retention_report.json` aligns with `missing_evidence`

### 10.5.7 Workspace Bundle Export (v0)

#### 10.5.7.1 Purpose
Provide a deterministic â€œbackup/transfer/fixtureâ€ export path that:
- preserves original imported bytes when present
- exports Handshake canonical workspace state (docs/canvases/tables)
- exports Display-derived renders for portability and review
- remains policy-gated and exportability-safe

#### 10.5.7.2 Non-negotiable invariants
- Export artifacts are derived from **DisplayContent**; filtering/redaction applies at **Display/Export only** and must not mutate Raw/Derived stores.
- **CloudLeakageGuard** must deny inclusion of artifacts marked `exportable=false` unless an explicit policy override exists.
- Export runs as a **mechanical workflow job** (capability-gated, logged, hashed).

#### 10.5.7.3 Terms
- **BundleKind**: `debug_bundle` (existing), `workspace_bundle` (new)
- **ExportProfile**:
  - `SAFE_DEFAULT`: redacts secrets/PII, uses hashes/minimal previews
  - `WORKSPACE`: includes more local context but still redacts secrets/PII
  - `FULL_LOCAL`: full payloads; must not be exportable unless policy explicitly allows

#### 10.5.7.4 Bundle format
- Default output is a **zip** (or folder) containing:
  - `bundle_manifest.json` (required; schema versioned)
  - `workspace/`
    - `docs/<doc_id>.json`
    - `canvases/<canvas_id>.json`
    - `tables/<table_id>.json`
    - `tables/<table_id>.csv`
  - `assets/raw/` (byte-identical originals, if imported assets exist)
  - `assets/rendered/` (Display-derived render outputs: PDF/PNG/SVG/CSV as applicable)
  - `export_report.json` (included/excluded counts + reasons)

#### 10.5.7.5 Manifest requirements (`bundle_manifest.json`)
Must include:
- `bundle_kind`, `schema_version`, `created_at`
- workspace identifier (wsid) and exported entity IDs
- `job_id`, `workflow_run_id`
- `export_profile_id`
- tool/renderer versions used
- input hashes (raw/canonical) and output hash list

#### 10.5.7.6 Policy + capabilities
- Capability gating (minimum):
  - `export.bundle` (initiate export)
  - `fs.write` (destination-scoped)
  - optional `export.include_nonexportable` (explicit and default-deny)
- Policy context for export must be captured and visible (same treatment as other operational actions).
- When `exportable=false` artifacts are encountered:
  - default action: **exclude** and record reason in `export_report.json`
  - surface denial in Problems + Flight Recorder logs

#### 10.5.7.7 Determinism
- Same inputs + same ExportProfile + same renderer/template versions should produce stable hashes for deterministic outputs (platform constraints noted in manifest if needed).

#### 10.5.7.8 Observability
- Emit a distinct Flight Recorder event for Workspace Bundle export (parallel to the debug bundle export event).
- Store/export logs must include: selected IDs, profile, outputs, hashes, denials.
- [ADD v02.154] Workspace Bundle export MUST also remain explicit in Appendix 12 as a feature/coverage entry even while implementation stays stub-backed; it MAY NOT exist only as Main Body prose.
- [ADD v02.161] Dev Command Center MUST project Workspace Bundle export state, manifest identifiers, export records, and bounded replay or backup status from authoritative backend artifacts rather than reconstructing them from transient console state.

#### 10.5.7.9 Explicitly out of scope (v0)
- Round-trip writers for proprietary formats (DOCX/PPTX/XLSX)
- cloud upload/sharing
- any export path that mutates Raw/Derived stores

### 10.5.8 Acceptance criteria and validators

- Acceptance criteria (AC) MUST be written such that they can be checked by deterministic validators (VAL).
- Validator definitions live in **Â§11.4.1** (and reference Â§11.5 where needed).

#### 10.5.7.1 Acceptance criteria (AC) â†’ Validators (VAL)

| AC ID | Requirement | Validators |
|------:|-------------|------------|
| AC-OPS-001 | Problems view consumes canonical `Diagnostic` objects and renders them with filters for severity/source/surface/wsid/job/time. | VAL-DIAG-001 |
| AC-OPS-002 | Problems grouping uses deterministic `fingerprint` and preserves access to raw instances. | VAL-FP-001 |
| AC-OPS-003 | Evidence Drawer displays `link_confidence` and candidate links; ambiguous links are never shown as certain. | VAL-CORR-001 |
| AC-OPS-004 | Every operator action that changes state emits a Flight Recorder event (with actor=`human`) and is navigable from Timeline. | VAL-CONSOLE-001, VAL-NAV-001 |
| AC-OPS-005 | Debug Bundle contains the required minimum files and references are internally consistent. | VAL-BUNDLE-001 |
| AC-OPS-006 | SAFE_DEFAULT export contains no secrets/PII matches per configured detectors; redaction report is present. | VAL-REDACT-001 |
| AC-OPS-007 | Operational actions (resync/rebuild/export/rerun) record policy context and expose it in the UI. | VAL-POLICY-001 |


## 10.6 Canvas: Typography & Font Packs

Status: Font Packs + Canvas Typography Support Spec v0.1 (verbatim import) is normative for font packaging, import, runtime loading, and Canvas text rendering. Not yet implemented.

### 10.6.0 Scope and positioning

This section specifies a curated open-font pack (bundled + user-imported) and a deterministic, secure loading pipeline for Canvas typography. The same font registry and loader MAY be reused by other Handshake surfaces (e.g., previews, editors) but Canvas is the first target.

**Authority / non-drift note**
- Â§10.6.1 is normative for the font pack contents, licensing requirements, import/scan/manifest pipeline, and runtime registration/loading behavior.
- Â§10.6.1 is a zero-loss import of `Handshake_Font_Packs_and_Canvas_Typography_Spec_v0.1.md`; the original document title line is replaced by the Â§10.6.1 heading below.

**Integration hooks (additive, non-normative)**
- Capabilities/consent: operations such as import/remove/rebuild MUST be gated via capability/consent policy before any filesystem writes.
- AI Job Model: font operations SHOULD be routable as explicit jobs when invoked by the agent layer (no implicit background mutation).
- Security: UI surfaces MUST NOT crawl the filesystem directly; font discovery is backend-owned, with a narrow CSP surface for font asset delivery.

### 10.6.1 Handshake â€” Font Packs + Canvas Typography Support Spec v0.1 (verbatim import; headings adjusted)

**Status:** Draft (implementation-ready)  
**Owner:** Handshake  
**Audience:** Handshake core (Tauri/Rust), UI (React/WebView), Canvas sketch tool  
**Primary goal:** Ship a curated open-font library (40+), usable in **Canvas sketches** and other Handshake surfaces, with a deterministic, secure import + loading pipeline.

---

#### 1. Why this exists

Handshakeâ€™s Canvas sketch surface is meant for quick graphic/layout ideation. Default system fonts make everything look â€œgeneric.â€ A curated open-font set and fast font tooling enables â€œdesign/architecture-magazineâ€ style comps without requiring the user to know typography.

---

#### 2. Scope

##### In scope
- Bundle a **curated open-font pack** (40 fonts) + optional â€œextrasâ€
- Self-host fonts (no CDN dependency)
- User-managed font import (drop-in or import dialog)
- Deterministic font scanning + manifest
- Safe runtime registration (WebView) and reliable Canvas rendering
- Font picker UI primitives + pairing presets for fast composition

##### Out of scope (explicit)
- Licensing automation beyond bundling license files + notices
- Paid fonts / Adobe Fonts / custom foundry licensing workflows
- Full typesetting features (optical margins, OpenType feature UI, hyphenation engines)
- Font editing, subsetting pipelines, or variable-axis UI beyond â€œweight/italicâ€

---

#### 3. Licensing & compliance model (non-negotiable)

##### 3.1 Baseline principle
Even â€œfreeâ€ fonts are licensed; shipping a font means shipping licensed software. Google Fonts explicitly states the most common license is **SIL OFL**, with some fonts under **Apache** or **Ubuntu Font License**, and that you can use and redistribute fonts under the license conditions.  
Sources: Google Fonts FAQ.

##### 3.2 Packaging requirements
When Handshake redistributes fonts (bundled pack), Handshake MUST:
- Keep a **per-font license file** (e.g., `OFL.txt`, `LICENSE`, etc.)
- Maintain a project-level **THIRD_PARTY_NOTICES** file listing:
  - font family name
  - license type (SPDX identifier if known, e.g. `OFL-1.1`)
  - source (Google Fonts page / upstream repo)
  - checksum (sha256) of shipped binary
- If any font uses OFL Reserved Font Names (RFNs): do **not** modify the font files (format conversion, subsetting, rebuilding) unless the RFN rules are satisfied (rename) or explicit permission exists.
  - RFNs are a formal OFL concept and are declared in the OFL text / metadata.
  - If you do modify and redistribute, include original copyright statements, RFN declarations, and license text.

Notes:
- This spec assumes **no modification** of shipped fonts beyond â€œcopy as-is.â€
- Prefer shipping vendor-provided `.woff2` if available, otherwise ship `.ttf/.otf` unmodified.

---

#### 4. Font pack inventory

##### 4.1 Design Pack 40 (default full pack)

These are selected to match â€œmodern design studio + editorial + architectural annotationâ€ patterns. Many are directly present in Typewolfâ€™s curated â€œbest free Google Fontsâ€ list (used as a popularity proxy for design usage).

###### Sans / UI / grotesk (20)
1. Inter
2. DM Sans
3. Manrope
4. Space Grotesk
5. Work Sans
6. IBM Plex Sans
7. Plus Jakarta Sans
8. Outfit
9. Urbanist
10. Montserrat
11. Poppins
12. Open Sans
13. Source Sans 3
14. Libre Franklin
15. Fira Sans
16. Karla
17. Lato
18. PT Sans
19. Chivo
20. Rubik

###### Serif / editorial (12)
21. Playfair Display
22. Lora
23. Source Serif 4
24. Spectral
25. Cormorant
26. Alegreya
27. Libre Baskerville
28. Eczar
29. Fraunces
30. Inknut Antiqua
31. Merriweather
32. BioRhyme

###### Mono / annotation (4)
33. JetBrains Mono (OFL; upstream confirms OFL 1.1)
34. Space Mono
35. Inconsolata
36. Archivo Narrow (use as condensed label font; not mono but often used similarly in diagrams)

###### â€œArchitectural handwriting / sketchâ€ accents (4)
37. Architects Daughter (architectural-note vibe)
38. Syne (display)
39. Proza Libre
40. Alegreya Sans

##### 4.2 Optional extras (not installed by default; can be toggled)
Rationale: these are high value but may add size dramatically.

- Noto Sans (wide language coverage; huge)
- Noto Serif (wide language coverage; huge)
- Noto Sans Mono (wide language coverage; huge)
- Crimson Text (classic editorial)
- EB Garamond (classic editorial)
- Raleway (thin display)
- Oswald (condensed display)
- Barlow / Barlow Condensed (signage vibe)
- Sora (modern product)
- Recursive (variable mono/sans hybrid)

---

#### 5. UX primitives (Canvas-first)

##### 5.1 Typography presets (one-click)
Provide pairing presets to reduce â€œtypography skill requirement.â€

Preset examples:
- **Clean Editorial:** Inter (body) + Playfair Display (headline) + JetBrains Mono (labels)
- **Modern Studio:** Space Grotesk (headline/body) + Fraunces (accent) + Space Mono (labels)
- **Architecture Deck:** IBM Plex Sans (body) + IBM Plex Serif (headline) + JetBrains Mono (dimensions)

##### 5.2 Text objects (not raster stamps)
Canvas must treat text as structured objects so it stays editable.

**TextNode fields**
- `id: string`
- `x,y,w,h: number` (canvas coordinates)
- `text: string`
- `family: string`
- `sizePx: number`
- `weight: number | [min,max]`
- `style: "normal"|"italic"`
- `align: "left"|"center"|"right"`
- `lineHeight: number`
- `tracking: number` (optional)
- `color: RGBA`
- `rotation: number` (optional)

Editing mechanism:
- Double-click opens an **HTML overlay textarea** positioned over canvas for real caret/selection/IME.
- Commit -> update TextNode -> redraw.

---

#### 6. Technical architecture

##### 6.1 Principle: backend owns the filesystem
Frontend must not crawl arbitrary filesystem paths. Font import + scanning happens in Rust.

##### 6.2 Path model
- Repo fonts: `app/src-tauri/resources/fonts/`
- Runtime fonts live in the per-user app data directory.

Runtime layout:
- `{APP_DATA}/fonts/`
  - `bundled/` (copied from resources on first run or pack version bump)
  - `user/` (user imported fonts)
  - `cache/`
    - `manifest.json`
    - `manifest.lock.json` (hashes/mtimes; optional)
    - `fonts.css` (optional; generated stylesheet)

---

#### 7. Backend (Rust/Tauri) specification

##### 7.1 Commands (required)

1) `fonts_bootstrap_pack(pack_id?: string)`
- Ensures runtime folders exist
- Copies packaged fonts from `resources/fonts/<pack_id>` into `{APP_DATA}/fonts/bundled/`
- Writes/updates `fonts_pack_version.json` to detect future upgrades

2) `fonts_rebuild_manifest() -> Manifest`
- Scans `{APP_DATA}/fonts/bundled/` and `{APP_DATA}/fonts/user/`
- For each font file:
  - validate extension allowlist: `.ttf`, `.otf`, `.woff2` (optional `.woff`)
  - compute sha256
  - extract metadata: family, style, weight range, postscript name (best effort)
- Writes `{APP_DATA}/fonts/cache/manifest.json`

3) `fonts_list() -> Manifest`
- Returns cached manifest (rebuild only if missing or invalid)

4) `fonts_import(paths: string[]) -> ImportResult`
- Copies user-selected files into `{APP_DATA}/fonts/user/`
- Enforces size limit (configurable)
- Deduplicates by sha256
- Rebuilds manifest

5) `fonts_remove(font_id: string)`
- Removes from `user/` only
- Rebuilds manifest

##### 7.2 Manifest schema

```json
{
  "schemaVersion": 1,
  "generatedAt": "ISO-8601",
  "packVersion": "design-pack-40@1",
  "fonts": [
    {
      "id": "sha256:...",
      "family": "Inter",
      "style": "normal",
      "weight": { "type": "variable", "min": 100, "max": 900 },
      "source": "bundled|user",
      "format": "woff2|ttf|otf",
      "path": "absolute-device-path",
      "license": {
        "spdx": "OFL-1.1|Apache-2.0|UFL-1.0|UNKNOWN",
        "licenseFile": "absolute-device-path-or-null"
      }
    }
  ]
}
```

Validation rules:
- `family` must be sanitized for CSS usage (see 8.3)
- `path` must be under `{APP_DATA}/fonts/**` (reject otherwise)
- `id` uniqueness enforced

##### 7.3 Security notes (backend)
- If using `@tauri-apps/plugin-fs`, enforce strict scope globs and keep font IO in Rust commands anyway.
- Do not expose the â€œopenâ€ endpoint to untrusted input without validation regex.

---

#### 8. Frontend (React/WebView) specification

##### 8.1 Font URL conversion (required)
Use `convertFileSrc(font.path)` to convert a device path into a WebView-loadable URL.

Important: Tauri requires `asset:` and `http://asset.localhost` to be included in CSP when using `convertFileSrc()`.

##### 8.2 Deterministic loading (FontFace API)
When entering Canvas (or any surface needing fonts):
1) `manifest = await invoke("fonts_list")`
2) For each font:
   - `url = convertFileSrc(font.path)`
   - `face = new FontFace(family, `url(${url})`, { style, weight })`
   - `await face.load(); document.fonts.add(face)`
3) `await document.fonts.ready`
4) Then render canvas text to avoid fallback/layout shift.

##### 8.3 Safe CSS + name handling
To prevent CSS injection via font family names:
- Allow only `[A-Za-z0-9 _-]` for display names
- Strip/replace quotes, semicolons, newlines
- Maintain an internal `cssFamily` name if the fontâ€™s true family contains unsafe characters.

##### 8.4 CSS generation options
Option A (preferred):
- Backend generates `{APP_DATA}/fonts/cache/fonts.css`
- Frontend `<link rel="stylesheet" href={convertFileSrc(cssPath)} />`

Option B:
- Frontend injects a `<style>` tag at runtime (only if CSP permits)

This spec recommends **FontFace API** as primary; CSS file generation is optional.

---

#### 9. Tauri configuration requirements

##### 9.1 CSP
Handshake must configure CSP so that:
- `asset:` and `http://asset.localhost` are allowed where needed (fonts, styles if used)
- `font-src` includes `asset:` and `http://asset.localhost`

Tauri CSP is intentionally restrictive; do not weaken it broadlyâ€”add only what is required for fonts.

##### 9.2 Asset protocol
Enable `assetProtocol` and scope it narrowly to the fonts directory under app data, not `**/*`.

---

#### 10. Performance strategy

Problems:
- Bundling 40+ fonts increases app size.
- Loading all fonts at runtime wastes memory and delays first draw.

Required mitigations:
- **Lazy register:** only load a font when it is selected in UI, plus a small default set.
- Keep a small â€œCanvas default setâ€ preloaded (e.g., Inter, Space Grotesk, JetBrains Mono).
- Cache â€œloaded familiesâ€ in-memory per session.

Optional:
- A â€œpack toggleâ€ UI to install extras on demand (download or copy from resources).

---

#### 11. Test plan

##### 11.1 Backend tests
- Import a valid font -> appears in manifest, sha256 stable
- Import duplicate -> dedup works
- Import invalid file -> rejected
- Path traversal attempt -> rejected
- Manifest rebuild on missing cache works

##### 11.2 Frontend tests
- Font selection changes canvas rendering
- First render after loading uses correct font (no fallback flash)
- Editing overlay preserves cursor/selection/IME
- Export (PNG/SVG) uses the selected font and matches on-screen rendering (within tolerance)

---

#### 12. Acceptance criteria

1) Handshake ships with Design Pack 40 available offline.
2) Canvas can render and edit text objects using those fonts.
3) Users can import additional fonts without granting the UI arbitrary filesystem access.
4) Font loading is deterministic (no â€œrandom fallbackâ€ on first draw).
5) Licensing artifacts are present (per-font license files + THIRD_PARTY_NOTICES).
6) CSP and asset protocol scopes remain narrow and security-conscious.

---

#### 13. Sources used for this spec (for verification)

(Keep these as reference during implementation; do not require runtime network access.)

- Google Fonts FAQ (licensing + commercial/redistribution guidance)
- Google Fonts Knowledge: â€œLicensingâ€ glossary
- Typewolf: curated â€œ40 best free Google Fontsâ€ list (popularity proxy)
- JetBrains Mono official page / repo (OFL confirmation)
- Tauri v2 `convertFileSrc()` docs (asset protocol + CSP requirement)
- Tauri v2 CSP docs
- Tauri filesystem plugin security docs (path traversal + scoping)
- MDN CSS Font Loading API (FontFace / document.fonts)
- OFL resources (RFN handling and modification guidance)


## 10.7 Charts & Dashboards

Status: Draft (not yet implemented). Defines the first finance-friendly visualization surface built on top of Tables without creating a parallel datastore.

### 10.7.0 Scope and positioning

- Charts are first-class entities (see Â§2.2.1) that reference Tables by ID and optional range/query.
- Dashboards are a DisplayContent/layout mode that composes Charts + Tables + KPIs using existing workspace entities (typically via Canvas or Document layouts).
- Charts/Dashboards MUST obey the cross-view integration rules: no new persistent stores, ID-based refs, AI Jobs for non-trivial operations (Â§7.1.0; Â§2.6.6).

### 10.7.1 Data model (normative minimum)

**Chart (RawContent)**
- `chart_id: UUID`
- `title: string`
- `source_table_id: UUID`
- `source_range: RangeSpec?` (optional)
- `chart_spec: JSON` (versioned; validates via `chart_spec_schema`)
- `theme_ref: string?` (optional; style only)

**Chart (DerivedContent)**
- `thumbnail_ref: AssetId?`
- `computed_series_cache: JSON?` (optional; regenerable)
- `last_render_hash: string`

**Chart (DisplayContent)**
- Interactive render in UI; may expose â€œopen source tableâ€ deep-link.

### 10.7.2 Determinism + provenance requirements

- Chart rendering MUST be reproducible given:
  - `chart_spec` + referenced Table version/hash + render engine version.
- Any AI-generated chart creation/update MUST be executed as an AI Job under Â§2.5.11 with preview and validators.

---

## 10.8 Presentations (Decks)

Status: Draft (not yet implemented). Defines an in-app deck surface plus deterministic export.

### 10.8.0 Scope and positioning

- A Deck is an ordered set of slides that reference existing entities (blocks, canvas frames, charts, assets) rather than copying their RawContent.
- Deck export (PPTX/PDF/HTML) is a mechanical operation that produces artifacts with provenance and MUST be logged like other mechanical tools (see Â§6.0 and Â§11.5).

### 10.8.1 Data model (normative minimum)

**Deck (RawContent)**
- `deck_id: UUID`
- `title: string`
- `slides: Slide[]` (ordered)

**Slide (RawContent)**
- `slide_id: UUID`
- `layout: enum` (e.g., `title`, `two_column`, `full_bleed`, `custom`)
- `elements: SlideElement[]`

**SlideElement (RawContent)**
- `kind: enum` (`text_ref`, `chart_ref`, `asset_ref`, `canvas_frame_ref`, `doc_block_ref`)
- `ref: EntityRef`
- `frame: {x,y,w,h}` (relative slide coords)
- `style: JSON?` (optional; no content duplication)

**Deck (DerivedContent)**
- `preview_thumbnails: AssetId[]`
- `export_history: [{format, ts, artifact_ref, policy}]`

### 10.8.2 Export discipline (normative minimum)

- Export MUST be invoked as `export_deck(...)` via the Charts & Decks AI Job Profile (Â§2.5.11) or an equivalent workflow node.
- Export outputs MUST be artifact references/handles (no large inline payloads).
- Export MUST record:
  - deck_id + slide_ids
  - referenced entity IDs + hashes
  - export engine version
  - export policy (`SAFE_DEFAULT` vs `FULL_FIDELITY`)
  - redaction/projection decisions (if any)

## 10.9 Future Surfaces

Reserved for future user-facing surfaces; add scoped subsections here.

---

<a id="11-shared-dev-platform-oss-foundations"></a>
## 10.10 Photo Studio

Photo Studio is a Lightroom/Affinity/Illustrator-class local-first photo + compositing surface. It is implemented by Photo Stack entities (Â§2.2.3) and executed by Darkroom engines (Â§6.3.3.6), producing artifacts under the unified export contract (Â§2.3.10).

### 10.10.1 Purpose & Scope Evolution

This specification defines a **technically rigorous, testable, deterministic** "Photo Stack" inside Handshake delivering:

- **Lightroom-class**: Ingest + catalog/DAM, non-destructive Develop pipeline, AI masking, merges, export presets, batch throughput
- **Affinity Photo-class**: Layered compositing (layers/masks/blend modes/live filters), personas, macros/batch, export slices
- **Illustrator/Affinity Designer-class (subset)**: Vector primitives, text/typography, shape tools, pathfinder operations
- **Integrated AI Stack**: Local LLMs, vision models, and generative AI for intelligent automation
- **Cross-Tool Integration**: Unified workflows spanning documents, spreadsheets, calendars, email, and creative tools

This document survives engineering scrutiny by defining:
- Complete tool inventories mapped to reference software
- Open-source components available for reuse (with licenses)
- Gaps requiring custom implementation
- Mechanical engine contracts with determinism guarantees
- AI/ML integration architecture with local-first processing
- Proxy workflow architecture for high-resolution camera support
- Validation and testing requirements

---

### 10.10.2 Normative Language

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, **MAY** are normative per RFC 2119.

If a requirement conflicts with the future Handshake Master Spec, this spec is the source of truth for the Photo Stack until merged.

---

### 10.10.3 Design Principles

#### 10.10.3.1 Local-first + Reproducible
1. The Photo Stack MUST function offline for all operations once inputs exist locally.
2. Every derived output MUST be reproducible from recorded inputs + engine versions + parameters, subject to determinism class.
3. All AI/ML inference SHOULD run locally by default; cloud services are opt-in only.

#### 10.10.3.2 Raw / Derived / Display Separation
1. **Raw** (original files) MUST NOT be mutated by any edit operation.
2. **Derived** holds recipes, masks, previews, layer graphs, caches, descriptors.
3. **Display** is a projection (UI render and export pipeline).

#### 10.10.3.3 Single Execution Authority
All edits MUST execute via the Handshake workflow/job runtime (no ad-hoc bypass). UI actions produce **job requests**, not direct mutations.

#### 10.10.3.4 Artifact-first Outputs
Large outputs MUST be written as artifacts (files in the artifact store) and referenced by handles. Prompts or logs MUST carry references, not binaries.

#### 10.10.3.5 Open-Source Preference
Where production-quality open-source libraries exist, implementations SHOULD leverage them to reduce custom code, improve maintainability, and benefit from community improvements.

**License Preference Order:**
1. MIT / Apache 2.0 / BSD (most permissive, preferred for future monetization flexibility)
2. LGPL (dynamic linking acceptable)
3. MPL 2.0 (file-level copyleft manageable)
4. GPL (avoid unless isolated subprocess)

#### 10.10.3.6 Proxy-Based High-Resolution Workflow
1. High-resolution camera files (>20MP) SHOULD be processed via proxy workflow for AI/ML operations.
2. Full-resolution processing MUST be available for traditional (non-AI) operations.
3. AI-derived adjustments SHOULD be expressible as parameters applicable to full-resolution files.

#### 10.10.3.7 Cross-Tool Context Sharing
1. Tools within Handshake SHOULD share context (selections, color palettes, metadata).
2. Clipboard operations SHOULD preserve semantic information across tool boundaries.
3. MCP (Model Context Protocol) SHOULD be used for tool interoperability where applicable.

---

### 10.10.4 Complete Tool Inventory

This section provides exhaustive tool/feature lists from reference software, organized by functional domain.

#### 10.10.4.1 RAW Development & Global Adjustments (Lightroom-class)

##### 10.10.4.1.1 Basic Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| White Balance | Color temperature + tint correction | temp (2000-50000K), tint (-150 to +150) | P0 |
| Auto WB Presets | As Shot, Daylight, Cloudy, Shade, Tungsten, Fluorescent, Flash, Custom | preset enum | P0 |
| Exposure | Overall brightness adjustment | EV (-5.0 to +5.0) | P0 |
| Contrast | Midtone contrast | -100 to +100 | P0 |
| Highlights | Bright area recovery | -100 to +100 | P0 |
| Shadows | Dark area lift | -100 to +100 | P0 |
| Whites | White point clipping | -100 to +100 | P0 |
| Blacks | Black point clipping | -100 to +100 | P0 |
| Texture | Mid-frequency detail | -100 to +100 | P0 |
| Clarity | Local contrast / edge enhancement | -100 to +100 | P0 |
| Dehaze | Atmospheric haze removal | -100 to +100 | P0 |
| Vibrance | Saturation (muted colors prioritized) | -100 to +100 | P0 |
| Saturation | Global saturation | -100 to +100 | P0 |

##### 10.10.4.1.2 Tone Curve Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Parametric Curve | Region-based adjustment | highlights, lights, darks, shadows ranges + amounts | P0 |
| Point Curve | Arbitrary spline control | Array of (x,y) control points | P0 |
| RGB Curves | Per-channel curves | Red, Green, Blue separate point arrays | P0 |
| Curve Presets | Linear, Medium Contrast, Strong Contrast, Custom | preset enum | P1 |

##### 10.10.4.1.3 HSL/Color Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Hue Adjustment | Per-color hue shift | 8 color channels Ã— hue offset (-100 to +100) | P0 |
| Saturation Adjustment | Per-color saturation | 8 color channels Ã— saturation (-100 to +100) | P0 |
| Luminance Adjustment | Per-color brightness | 8 color channels Ã— luminance (-100 to +100) | P0 |
| Target Adjustment Tool | Click-drag in image to adjust | coordinate + drag delta + channel mode | P1 |
| Color Channels | Red, Orange, Yellow, Green, Aqua, Blue, Purple, Magenta | enum | P0 |

##### 10.10.4.1.4 Color Grading Panel (3-way + Global)
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Shadows Wheel | Color tint for shadows | hue (0-360), saturation (0-100), luminance (-100 to +100) | P0 |
| Midtones Wheel | Color tint for midtones | hue, saturation, luminance | P0 |
| Highlights Wheel | Color tint for highlights | hue, saturation, luminance | P0 |
| Global Wheel | Overall color tint | hue, saturation, luminance | P0 |
| Blending | Blend mode between wheels | balance slider (-100 to +100) | P1 |
| Balance | Shadow/highlight range definition | balance (-100 to +100) | P1 |

##### 10.10.4.1.5 Detail Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Sharpening Amount | Overall sharpening strength | 0 to 150 | P0 |
| Sharpening Radius | Edge detection radius | 0.5 to 3.0 px | P0 |
| Sharpening Detail | Fine detail preservation | 0 to 100 | P0 |
| Sharpening Masking | Edge-only sharpening | 0 to 100 | P0 |
| Noise Reduction Luminance | Luminance noise reduction | 0 to 100 | P0 |
| NR Luminance Detail | Detail preservation | 0 to 100 | P0 |
| NR Luminance Contrast | Contrast preservation | 0 to 100 | P0 |
| Noise Reduction Color | Chroma noise reduction | 0 to 100 | P0 |
| NR Color Detail | Color detail preservation | 0 to 100 | P0 |
| NR Color Smoothness | Color transition smoothing | 0 to 100 | P1 |

##### 10.10.4.1.6 Lens Corrections Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Profile Corrections | Auto lens profile application | enable/disable, profile selection | P0 |
| Distortion Correction | Barrel/pincushion correction | -100 to +100 (manual), auto from profile | P0 |
| Vignette Correction | Light falloff correction | 0 to 200 (amount), 0 to 100 (midpoint) | P0 |
| Chromatic Aberration | Lateral CA removal | enable/disable, purple/green amount + hue | P0 |
| Defringe | Color fringe removal | purple hue, purple amount, green hue, green amount | P1 |

##### 10.10.4.1.7 Transform Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Upright Auto | Automatic perspective correction | Off, Auto, Level, Vertical, Full, Guided | P0 |
| Vertical Perspective | Manual vertical correction | -100 to +100 | P0 |
| Horizontal Perspective | Manual horizontal correction | -100 to +100 | P0 |
| Rotate | Image rotation | -180 to +180 degrees | P0 |
| Aspect | Aspect ratio adjustment | -100 to +100 | P1 |
| Scale | Image scale | 50 to 150% | P0 |
| X Offset | Horizontal position | pixels | P1 |
| Y Offset | Vertical position | pixels | P1 |
| Guided Upright | Manual guide lines | Array of line segments | P1 |

##### 10.10.4.1.8 Effects Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Post-Crop Vignette Amount | Vignette strength | -100 to +100 | P0 |
| Vignette Midpoint | Vignette size | 0 to 100 | P0 |
| Vignette Roundness | Vignette shape | -100 to +100 | P1 |
| Vignette Feather | Edge softness | 0 to 100 | P0 |
| Vignette Highlights | Highlight preservation | 0 to 100 | P1 |
| Vignette Style | Highlight Priority, Color Priority, Paint Overlay | enum | P1 |
| Grain Amount | Film grain strength | 0 to 100 | P1 |
| Grain Size | Grain particle size | 0 to 100 | P1 |
| Grain Roughness | Grain uniformity | 0 to 100 | P1 |

##### 10.10.4.1.9 Calibration Panel
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Process Version | Raw processing algorithm version | enum | P0 |
| Shadow Tint | Shadow color bias | -100 to +100 | P1 |
| Red Primary Hue | Red channel hue shift | -100 to +100 | P1 |
| Red Primary Saturation | Red channel saturation | -100 to +100 | P1 |
| Green Primary Hue | Green channel hue shift | -100 to +100 | P1 |
| Green Primary Saturation | Green channel saturation | -100 to +100 | P1 |
| Blue Primary Hue | Blue channel hue shift | -100 to +100 | P1 |
| Blue Primary Saturation | Blue channel saturation | -100 to +100 | P1 |

#### 10.10.4.2 Local Adjustments & Masking

##### 10.10.4.2.1 Manual Mask Tools
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Brush | Paint-on mask | size, feather, flow, density, auto-mask | P0 |
| Linear Gradient | Graduated mask | start point, end point, feather | P0 |
| Radial Gradient | Elliptical mask | center, radii, feather, invert | P0 |
| Eraser | Remove from mask | size, feather, flow | P0 |

##### 10.10.4.2.2 AI/Automated Masks
| Tool | Description | Output | Priority |
|------|-------------|--------|----------|
| Select Subject | Semantic subject detection | Binary mask | P0 |
| Select Sky | Sky region detection | Binary mask | P0 |
| Select Background | Inverse of subject | Binary mask | P0 |
| Select People | Human detection | Binary mask + sub-regions | P0 |
| People: Skin | Skin region from person mask | Binary mask | P1 |
| People: Body Skin | Body skin excluding face | Binary mask | P2 |
| People: Facial Skin | Face skin only | Binary mask | P2 |
| People: Eyebrows | Eyebrow regions | Binary mask | P2 |
| People: Sclera | Eye whites | Binary mask | P2 |
| People: Iris & Pupil | Colored eye portion | Binary mask | P2 |
| People: Lips | Lip regions | Binary mask | P2 |
| People: Teeth | Teeth regions | Binary mask | P2 |
| People: Hair | Hair regions | Binary mask | P1 |
| People: Clothes | Clothing regions | Binary mask | P2 |
| Select Objects | Promptable object selection | Binary mask | P1 |
| Landscape Masking | Terrain features (new 2025) | Multiple categorical masks | P2 |

##### 10.10.4.2.3 Range Masks
| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Luminance Range | Brightness-based masking | range (0-100 min/max), smoothness | P0 |
| Color Range | Color-based masking | sample colors, range, smoothness | P0 |
| Depth Range | Depth-map masking (phone portraits) | range, feather | P2 |

##### 10.10.4.2.4 Mask Operations
| Operation | Description | Priority |
|-----------|-------------|----------|
| Add | Union of masks | P0 |
| Subtract | Difference of masks | P0 |
| Intersect | Intersection of masks | P0 |
| Invert | Complement of mask | P0 |
| Duplicate | Copy mask | P0 |
| Rename | Label mask | P0 |
| Feather | Edge softness adjustment | P0 |
| Density | Overall mask opacity | P0 |

##### 10.10.4.2.5 Local Adjustment Parameters
All global adjustment parameters (section 3.1) available per-mask, plus:
| Parameter | Description | Priority |
|-----------|-------------|----------|
| MoirÃ© | MoirÃ© pattern reduction | P2 |
| Defringe | Local fringe removal | P2 |
| Hue | Local hue shift | P1 |

#### 10.10.4.3 Retouching Tools

| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Spot Removal (Heal) | Content-aware healing | size, feather, opacity, source selection | P0 |
| Spot Removal (Clone) | Direct clone stamp | size, feather, opacity, source selection | P0 |
| Red Eye Removal | Pet/human red-eye fix | pupil size, darken amount | P1 |
| Content-Aware Remove | AI object removal | brush strokes defining area | P1 |
| Generative Remove | AI-powered removal with inpainting | selection area, prompt | P2 |

#### 10.10.4.4 Merge/Computational Photography

| Tool | Description | Inputs | Output | Priority |
|------|-------------|--------|--------|----------|
| HDR Merge | High dynamic range fusion | 2+ bracketed exposures | 32-bit DNG or rendered output | P0 |
| Panorama Merge | Image stitching | 2+ overlapping images | Large stitched image | P0 |
| HDR Panorama | Combined HDR + stitch | Multiple bracketed pano sets | 32-bit stitched | P1 |
| Focus Stacking | Depth-of-field extension | 2+ focus-varied images | All-in-focus composite | P1 |
| AI Denoise | ML-based noise reduction | Single image | Denoised output | P0 |
| Super Resolution | AI upscaling (2x) | Single image (proxy/web only) | 2x resolution output | P1 |
| AI Raw Details | ML detail enhancement | Single raw | Enhanced DNG | P2 |

#### 10.10.4.5 Library/DAM Functions

| Function | Description | Priority |
|----------|-------------|----------|
| Import (Copy) | Copy files to managed location | P0 |
| Import (Add) | Reference files in place | P0 |
| Import (Move) | Move files to managed location | P1 |
| Duplicate Detection | Content-hash deduplication | P0 |
| Keyword Tagging | Hierarchical keywords | P0 |
| Star Ratings | 0-5 star rating | P0 |
| Color Labels | Red, Yellow, Green, Blue, Purple | P0 |
| Pick/Reject Flags | Flagged, Unflagged, Rejected | P0 |
| Collections | Manual groupings | P0 |
| Smart Collections | Rule-based dynamic groupings | P1 |
| Folder Sync | Sync with filesystem | P0 |
| Metadata Read/Write | EXIF/IPTC/XMP handling | P0 |
| Face Detection | Face region tagging | P2 |
| Face Recognition | Identity clustering | P2 |
| GPS/Map Integration | Geotagging and map view | P2 |
| Stacking | Group related images | P1 |
| Virtual Copies | Multiple edit versions | P0 |
| AI Auto-Tagging | Vision model keyword generation | P1 |
| AI Captioning | Vision model description generation | P1 |

#### 10.10.4.6 Layer Compositor (Affinity Photo-class)

##### 10.10.4.6.1 Layer Types
| Layer Type | Description | Priority |
|------------|-------------|----------|
| Pixel/Raster Layer | Standard bitmap layer | P0 |
| Adjustment Layer | Non-destructive adjustment | P0 |
| Live Filter Layer | Non-destructive filter | P0 |
| Group | Layer container | P0 |
| Mask Layer | Grayscale mask | P0 |
| Text Layer | Editable typography | P1 |
| Shape Layer | Vector shapes | P1 |
| Curve Layer | Vector paths | P1 |
| Artboard | Multi-canvas container | P2 |
| Linked Layer | External file reference | P2 |
| Embedded Document | Nested document | P2 |

##### 10.10.4.6.2 Blend Modes (Complete Set)
| Category | Modes | Priority |
|----------|-------|----------|
| Normal | Normal, Dissolve | P0 |
| Darken | Darken, Multiply, Color Burn, Linear Burn, Darker Color | P0 |
| Lighten | Lighten, Screen, Color Dodge, Linear Dodge (Add), Lighter Color | P0 |
| Contrast | Overlay, Soft Light, Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix | P0 |
| Inversion | Difference, Exclusion, Subtract, Divide | P0 |
| Component | Hue, Saturation, Color, Luminosity | P0 |
| Special | Passthrough (groups), Average, Negation, Reflect, Glow | P1 |

##### 10.10.4.6.3 Layer Properties
| Property | Description | Priority |
|----------|-------------|----------|
| Opacity | Layer transparency (0-100%) | P0 |
| Fill | Fill opacity (affects layer, not effects) | P1 |
| Visibility | Show/hide layer | P0 |
| Lock Position | Prevent moving | P0 |
| Lock Alpha | Preserve transparency | P0 |
| Lock All | Prevent all edits | P0 |
| Color Tag | Visual organization | P1 |
| Blend Ranges | Source/destination blend-if sliders | P1 |

##### 10.10.4.6.4 Adjustment Layers
| Adjustment | Parameters | Priority |
|------------|------------|----------|
| Brightness/Contrast | brightness, contrast, use legacy | P0 |
| Levels | input levels, output levels, per-channel | P0 |
| Curves | point array, per-channel | P0 |
| Exposure | exposure, offset, gamma | P0 |
| Hue/Saturation | hue, saturation, lightness, colorize | P0 |
| Color Balance | shadows/mids/highlights, preserve luminosity | P0 |
| Black & White | channel mixer percentages, tint | P0 |
| Photo Filter | filter color, density, preserve luminosity | P1 |
| Channel Mixer | RGB matrix, monochrome mode | P1 |
| Gradient Map | gradient definition, reverse, dither | P1 |
| Selective Color | per-color CMYK adjustments | P1 |
| Invert | negate colors | P0 |
| Posterize | levels | P1 |
| Threshold | threshold level | P0 |
| Vibrance | vibrance, saturation | P0 |
| HSL | per-channel H/S/L | P0 |
| Color Lookup (LUT) | 3D LUT file reference | P1 |
| Shadows/Highlights | amount, tone, radius | P0 |
| Lens Filter | filter color, optical density | P2 |
| Split Toning | shadow/highlight colors, balance | P1 |
| White Balance | temperature, tint | P0 |
| OCIO | OpenColorIO transform | P1 |
| Soft Proof | ICC profile simulation | P2 |

##### 10.10.4.6.5 Live Filter Layers
| Filter | Parameters | Priority |
|--------|------------|----------|
| Gaussian Blur | radius | P0 |
| Motion Blur | angle, distance | P1 |
| Radial Blur | amount, center, type | P1 |
| Zoom Blur | amount, center | P1 |
| Lens Blur (Depth of Field) | radius, shape, bokeh | P1 |
| Surface Blur | radius, threshold | P1 |
| Bilateral Blur | radius, tolerance | P2 |
| Unsharp Mask | amount, radius, threshold | P0 |
| High Pass | radius | P0 |
| Clarity | amount | P0 |
| Add Noise | amount, type, monochromatic | P1 |
| Denoise | luminance, color | P0 |
| Shadows/Highlights | shadow amount, highlight amount | P0 |
| Vignette | exposure, hardness, shape | P1 |
| Lighting Effects | light type, position, intensity, color | P2 |
| Lens Distortion | barrel/pincushion, chromatic aberration | P1 |
| Perspective | 4-point warp | P1 |
| Live Mesh Warp | grid deformation | P1 |
| Liquify (Live) | warp operations | P2 |
| Procedural Texture | noise type, scale, octaves | P2 |
| Diffuse Glow | amount, radius | P2 |
| Halftone | pattern, angle, size | P2 |

##### 10.10.4.6.6 Mask Types
| Mask Type | Description | Priority |
|-----------|-------------|----------|
| Layer Mask | Grayscale transparency mask | P0 |
| Vector Mask | Shape-based mask | P1 |
| Clipping Mask | Clip to layer below | P0 |
| Luminosity Mask | Auto-generated from luminance | P1 |
| Hue Range Mask | Color-based live mask | P1 |
| Band Pass Mask | Frequency-based edge mask | P2 |
| Compound Mask | Combined mask operations | P1 |

#### 10.10.4.7 Selection Tools

| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Rectangular Marquee | Rectangular selection | fixed ratio, fixed size, feather | P0 |
| Elliptical Marquee | Elliptical selection | fixed ratio, fixed size, feather | P0 |
| Row/Column Marquee | Single pixel line | row or column | P2 |
| Lasso | Freehand selection | feather | P0 |
| Polygonal Lasso | Point-to-point selection | feather | P0 |
| Magnetic Lasso | Edge-snapping selection | width, contrast, frequency | P1 |
| Magic Wand | Contiguous color selection | tolerance, contiguous, anti-alias | P0 |
| Quick Selection | Brush-based smart selection | size, hardness, auto-enhance | P0 |
| Selection Brush | Paint selection | size, hardness, mode | P0 |
| Flood Select | All matching pixels | tolerance, contiguous | P0 |
| Select by Color Range | Color-based selection | fuzziness, range, localized | P1 |
| Select Subject (AI) | One-click subject selection | | P0 |
| Select and Mask | Refine edge workspace | edge detection, global refinements | P1 |
| Grow Selection | Expand by tolerance | | P1 |
| Similar Selection | Select similar across image | | P1 |

#### 10.10.4.8 Paint & Retouch Tools

| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Brush | General painting | size, hardness, opacity, flow, blend mode, dynamics | P0 |
| Pencil | Aliased painting | size, opacity | P1 |
| Eraser | Remove pixels | size, hardness, opacity, mode | P0 |
| Background Eraser | Edge-aware erasure | tolerance, sampling, limits | P1 |
| Magic Eraser | One-click area erasure | tolerance, contiguous | P1 |
| Clone Stamp | Direct clone painting | size, hardness, opacity, flow, aligned | P0 |
| Pattern Stamp | Pattern painting | size, pattern, aligned | P2 |
| Healing Brush | Texture-aware healing | size, hardness, source mode | P0 |
| Spot Healing | Auto-source healing | size, type | P0 |
| Patch Tool | Region-based healing | patch mode, diffusion | P1 |
| Content-Aware Fill | AI-powered fill | structure, color adaptation | P1 |
| Inpainting Brush | Intelligent fill | size, iterations | P0 |
| Red Eye Tool | Red eye correction | pupil size, darken | P1 |
| Dodge | Lighten areas | size, range, exposure | P1 |
| Burn | Darken areas | size, range, exposure | P1 |
| Sponge | Saturate/desaturate | size, flow, mode | P1 |
| Smudge | Pixel smearing | size, strength | P1 |
| Blur Tool | Local blur painting | size, strength | P1 |
| Sharpen Tool | Local sharpen painting | size, strength | P1 |
| Gradient Tool | Gradient fill | type, mode, opacity, dither, reverse | P0 |
| Paint Bucket | Area fill | tolerance, contiguous, all layers | P0 |

#### 10.10.4.9 Vector & Shape Tools (Designer/Illustrator subset)

| Tool | Description | Priority |
|------|-------------|----------|
| Pen | Bezier path creation | P0 |
| Node/Direct Selection | Edit path nodes | P0 |
| Rectangle | Rectangle/rounded rectangle | P0 |
| Ellipse | Circle/ellipse | P0 |
| Polygon | Regular polygon | P1 |
| Star | Star shape | P1 |
| Line | Line segment | P0 |
| Arrow | Arrow shape | P1 |
| Custom Shape | Shape library | P1 |
| Text (Point) | Single-line text | P0 |
| Text (Frame) | Paragraph text | P0 |
| Text on Path | Path-following text | P2 |

#### 10.10.4.10 Transform Tools

| Tool | Description | Parameters | Priority |
|------|-------------|------------|----------|
| Move | Reposition content | x, y offset | P0 |
| Free Transform | Scale, rotate, skew | bounds, rotation, skew | P0 |
| Scale | Resize | width, height, constrain proportions | P0 |
| Rotate | Rotation | angle, center point | P0 |
| Skew | Shear transform | horizontal, vertical | P0 |
| Distort | 4-corner distortion | corner positions | P0 |
| Perspective | Perspective transform | vanishing point | P0 |
| Warp | Grid-based warp | warp style or custom mesh | P1 |
| Puppet Warp | Pin-based deformation | pin positions | P2 |
| Content-Aware Scale | Seam-carving resize | protected areas | P2 |
| Flip Horizontal | Mirror horizontally | | P0 |
| Flip Vertical | Mirror vertically | | P0 |

#### 10.10.4.11 Export Functions

| Format | Options | Priority |
|--------|---------|----------|
| JPEG | Quality, progressive, subsampling, embed profile | P0 |
| PNG | Bit depth (8/16), interlaced, transparency | P0 |
| TIFF | Compression (None/LZW/ZIP), bit depth, layers | P0 |
| WebP | Quality, lossless option | P0 |
| AVIF | Quality, speed, bit depth | P1 |
| HEIC/HEIF | Quality | P1 |
| PSD | Layers, compatibility mode | P1 |
| PDF | Quality, color space, compatibility | P1 |
| DNG | Lossy/lossless, embed original | P0 |
| OpenEXR | Compression, half/full float | P2 |
| JPEG XL | Quality, effort, lossless | P2 |
| ORA (OpenRaster) | Layer preservation | P0 |

#### 10.10.4.12 Batch & Automation

| Feature | Description | Priority |
|---------|-------------|----------|
| Preset Application | Apply develop preset to selection | P0 |
| Sync Settings | Copy settings across images | P0 |
| Auto Sync | Live sync while editing | P1 |
| Export Presets | Saved export configurations | P0 |
| Batch Export | Export multiple with presets | P0 |
| Macros (Recording) | Record action sequence | P1 |
| Macros (Playback) | Execute recorded actions | P1 |
| Batch Macro | Apply macro to multiple files | P1 |
| Watch Folders | Auto-import from folders | P2 |
| Slices | Multi-region export | P2 |

---

### 10.10.5 Integrated Tool Ecosystem

#### 10.10.5.1 Core Handshake Tools (Built-in)

| Tool | Purpose | Integration Points |
|------|---------|-------------------|
| **Canvas** | Visual editing surface | Photo editing, compositing, moodboards |
| **Word** | Document creation | Reports, client briefs, photo books |
| **Monaco** | Code/text editor | Scripts, automation, config |
| **Calendar** | Scheduling | Shoot dates from EXIF, client deadlines |
| **Mail** | Communication | Client delivery, proof notifications |
| **Excel** | Data/spreadsheets | Metadata batch editing, analytics |
| **Terminal** | Command line | Batch operations, scripting |
| **ComfyUI** | Generative AI workflows | See section 4.2 |

#### 10.10.5.2 ComfyUI Integration Scope

ComfyUI provides generative AI capabilities with specific scope limitations based on file size constraints.

##### 10.10.5.2.1 Supported Use Cases (Proxy/Web Images)

| Use Case | Input Size | Description | Priority |
|----------|-----------|-------------|----------|
| **Moodboard Enhancement** | â‰¤4K px | Upscale/style-match web-sourced reference images | P0 |
| **Image Refactoring** | â‰¤4K px | Modify found images to fit creative direction | P0 |
| **Style Transfer** | â‰¤2K px | Apply artistic styles to proxy images | P1 |
| **Object Generation** | N/A | Generate synthetic elements for compositing | P0 |
| **Background Generation** | â‰¤4K px | Create or extend backgrounds | P1 |
| **Texture Generation** | N/A | Procedural textures for compositing | P1 |
| **Concept Visualization** | N/A | Generate mood/concept imagery from prompts | P0 |
| **Inpainting (Small)** | â‰¤4K px | Fill regions in proxy images | P1 |
| **Upscaling (Web Images)** | â‰¤2K px input | Upscale web-sourced images 2-4x | P0 |
| **Face Restoration** | Cropped region | Fix faces in portrait crops | P1 |

##### 10.10.5.2.2 NOT Supported (High-Resolution Camera Files)

The following operations are **explicitly out of scope** for ComfyUI due to memory/compute constraints:

| Camera | Resolution | Why Not Supported |
|--------|-----------|-------------------|
| Canon R5 | 45MP (8192Ã—5464) | ~130MB uncompressed, exceeds VRAM |
| Sony A7RV | 61MP (9504Ã—6336) | ~175MB uncompressed, exceeds VRAM |
| Fuji GFX100S | 102MP (11648Ã—8736) | ~290MB uncompressed, far exceeds VRAM |
| Any >20MP | >5472Ã—3648 | Typical diffusion models operate at 1024-2048px |

**For high-resolution camera files, use:**
- Traditional processing pipeline (section 3.1-3.5)
- Proxy workflow (section 5) for AI-assisted adjustments
- Region-based processing (crop, process, composite back)

##### 10.10.5.2.3 ComfyUI Workflow Patterns

```
Pattern 1: Web Image Enhancement
  Web image (â‰¤2K) â†’ ComfyUI upscale â†’ Moodboard

Pattern 2: Style Reference Matching  
  Found image â†’ ComfyUI style transfer â†’ Match to project aesthetic

Pattern 3: Asset Generation
  Text prompt â†’ ComfyUI generate â†’ Composite into high-res file

Pattern 4: Proxy-Based Enhancement
  High-res â†’ Export 2K proxy â†’ ComfyUI enhance â†’ Apply learnings to full-res (where applicable)

Pattern 5: Region Processing
  High-res crop (face/detail) â†’ ComfyUI process â†’ Composite back at full resolution
```

#### 10.10.5.3 Additional Recommended Tools (Permissive Licenses)

##### 10.10.5.3.1 Vector Graphics & Design

| Tool | License | Purpose | Integration |
|------|---------|---------|-------------|
| **Excalidraw** | MIT | Hand-drawn diagrams, wireframes | Moodboards, client presentations |
| **Fabric.js** | MIT | Canvas manipulation, SVG import/export | Extended canvas capabilities |
| **Paper.js** | MIT | Vector graphics scripting, bezier editing | Vector layer implementation |
| **Snap.svg** | Apache 2.0 | SVG manipulation and animation | Vector export, web graphics |
| **SVG-edit** | MIT | Web-based SVG editor | Simple vector editing |
| **SVG.js** | MIT | Lightweight SVG manipulation | Programmatic vector generation |

##### 10.10.5.3.2 Audio/Video

| Tool | License | Purpose | Integration |
|------|---------|---------|-------------|
| **ffmpeg.wasm** | MIT (wrapper) | Video transcoding, editing in browser | Video timeline, format conversion |
| **Howler.js** | MIT | Audio playback, sprites, spatial audio | Slideshow audio, sound effects |
| **Tone.js** | MIT | DAW-like synthesis, sequencing | Audio branding, sound design |
| **WaveSurfer.js** | BSD-3 | Waveform visualization | Audio editing UI |

##### 10.10.5.3.3 Document Processing

| Tool | License | Purpose | Integration |
|------|---------|---------|-------------|
| **Docling** | MIT | Document parsing (PDF, DOCX, etc.) | Client brief extraction, invoice processing |
| **pdf-lib** | MIT | PDF creation/modification | Photo book export, proof sheets |
| **jsPDF** | MIT | PDF generation | Reports, contact sheets |
| **PDF.js** | Apache 2.0 | PDF viewing/rendering | Document preview |

##### 10.10.5.3.4 Metadata & Color

| Tool | License | Purpose | Integration |
|------|---------|---------|-------------|
| **exifr** | MIT | Fast EXIF/IPTC/XMP/ICC parsing | Metadata panel, auto-organization |
| **Chroma.js** | MIT | Color manipulation, palette generation | Color tools, palette extraction |
| **Color Thief** | MIT | Dominant color extraction | Moodboard color analysis |
| **culori** | MIT | Color space conversions (LAB, LCH, OKLab) | Color science operations |

##### 10.10.5.3.5 Diagramming & Visualization

| Tool | License | Purpose | Integration |
|------|---------|---------|-------------|
| **Mermaid** | MIT | Diagrams from text | Workflow documentation |
| **Rough.js** | MIT | Hand-drawn style rendering | Sketch overlays |
| **Three.js** | MIT | 3D rendering | 3D asset preview, product photography |

#### 10.10.5.4 AI/ML Model Stack

##### 10.10.5.4.1 Local LLMs

| Model | Size | Strength | Use Cases | Priority |
|-------|------|----------|-----------|----------|
| **Llama 3.1/3.2** | 8B-70B | Reasoning, instruction-following | Document analysis, code generation, complex queries | P0 |
| **Mythomax** | 7B/13B | Creative writing | Client emails, social captions, blog posts, storytelling | P0 |
| **Qwen2.5** | 7B-72B | Multilingual, coding | International clients, automation scripts | P1 |

##### 10.10.5.4.2 Vision Models

| Model | VRAM | Strength | Use Cases | Priority |
|-------|------|----------|-----------|----------|
| **MiniCPM-V 2.6** | ~8GB | High-res native (1.8M pixels) | Image understanding without aggressive downscale | P0 |
| **Qwen2-VL** | ~8GB | Strong OCR, Chinese support | Document scanning, text extraction | P0 |
| **Molmo 7B** | ~8GB | General vision, Apache 2.0 | Image description, tagging | P1 |
| **LLaVA 1.6** | ~20GB | Best quality | Complex scene analysis | P2 |
| **InternVL2** | Various | Detailed understanding | Technical image analysis | P2 |

##### 10.10.5.4.3 AI Integration Patterns

```
Pattern: Auto-Tagging Pipeline
  Image â†’ Vision Model â†’ Keywords + Description â†’ IPTC/XMP metadata

Pattern: Document â†’ Calendar Integration  
  PDF (client brief) â†’ Docling â†’ LLM extract dates â†’ Calendar events

Pattern: Intelligent Culling Assistant
  Batch images â†’ Vision Model â†’ Quality scores + descriptions â†’ Smart collection

Pattern: Client Communication
  Shoot metadata â†’ LLM â†’ "Your photos are ready" email draft

Pattern: Moodboard Analysis
  Reference images â†’ Vision Model â†’ Style description â†’ ComfyUI prompt guidance
```

#### 10.10.5.5 MCP (Model Context Protocol) Integration

MCP enables standardized communication between tools and AI models.

##### 10.10.5.5.1 MCP Server Capabilities

| Server | Purpose | Operations |
|--------|---------|------------|
| **Filesystem** | File access | Read, write, list, watch |
| **Database** | Catalog queries | SQL queries against catalog |
| **Image** | Image operations | Metadata read, thumbnail generation |
| **Calendar** | Scheduling | Create, read, update events |
| **Mail** | Communication | Draft, send, search emails |

##### 10.10.5.5.2 MCP Tool Exposure

The Photo Stack exposes these tools via MCP for AI agents:

```typescript
interface PhotoStackMCPTools {
  // Catalog operations
  'photo.search': (query: SearchQuery) => PhotoResult[];
  'photo.getMetadata': (photoId: UUID) => PhotoMetadata;
  'photo.setMetadata': (photoId: UUID, metadata: Partial<PhotoMetadata>) => void;
  'photo.addToCollection': (photoId: UUID, collectionId: UUID) => void;
  
  // Edit operations
  'photo.applyPreset': (photoId: UUID, presetId: UUID) => void;
  'photo.adjustBasic': (photoId: UUID, adjustments: BasicAdjustments) => void;
  'photo.export': (photoId: UUID, exportSettings: ExportSettings) => ArtifactHandle;
  
  // AI operations
  'photo.analyzeWithVision': (photoId: UUID) => VisionAnalysis;
  'photo.generateTags': (photoId: UUID) => string[];
  'photo.generateCaption': (photoId: UUID) => string;
  
  // Batch operations
  'photo.batchApplyPreset': (photoIds: UUID[], presetId: UUID) => void;
  'photo.batchExport': (photoIds: UUID[], exportSettings: ExportSettings) => ArtifactHandle[];
}
```

---

### 10.10.6 Proxy Workflow Architecture

#### 10.10.6.1 Overview

High-resolution camera files (Canon R5, Sony A7RV, Fuji GFX100S, etc.) require a proxy-based workflow for AI/ML operations due to memory constraints.

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                           PROXY WORKFLOW                                     â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚   INGEST     â”‚     â”‚   PROCESSING     â”‚     â”‚      OUTPUT          â”‚    â”‚
â”‚  â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤     â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤     â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤    â”‚
â”‚  â”‚              â”‚     â”‚                  â”‚     â”‚                      â”‚    â”‚
â”‚  â”‚ RAW File     â”‚â”€â”€â”€â”€â–¶â”‚ Full Resolution  â”‚â”€â”€â”€â”€â–¶â”‚ Print/Archive        â”‚    â”‚
â”‚  â”‚ (45-102MP)   â”‚     â”‚ Traditional      â”‚     â”‚ (Full quality)       â”‚    â”‚
â”‚  â”‚              â”‚     â”‚ Pipeline         â”‚     â”‚                      â”‚    â”‚
â”‚  â”‚   LibRaw     â”‚     â”‚ (Section 3.1-3.5)â”‚     â”‚                      â”‚    â”‚
â”‚  â”‚   Decode     â”‚     â”‚                  â”‚     â”‚                      â”‚    â”‚
â”‚  â”‚              â”‚     â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤     â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤    â”‚
â”‚  â”‚      â”‚       â”‚     â”‚                  â”‚     â”‚                      â”‚    â”‚
â”‚  â”‚      â–¼       â”‚     â”‚ Proxy (2-4K)     â”‚â”€â”€â”€â”€â–¶â”‚ Web/Social           â”‚    â”‚
â”‚  â”‚ Generate     â”‚â”€â”€â”€â”€â–¶â”‚ AI Processing    â”‚     â”‚ (Optimized)          â”‚    â”‚
â”‚  â”‚ Proxy        â”‚     â”‚ - Vision Model   â”‚     â”‚                      â”‚    â”‚
â”‚  â”‚ (2048px)     â”‚     â”‚ - ComfyUI        â”‚     â”‚                      â”‚    â”‚
â”‚  â”‚              â”‚     â”‚ - Analysis       â”‚     â”‚                      â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                â”‚                                            â”‚
â”‚                                â–¼                                            â”‚
â”‚                       â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                 â”‚
â”‚                       â”‚  AI OUTPUTS      â”‚                                 â”‚
â”‚                       â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤                                 â”‚
â”‚                       â”‚ â€¢ Keywords/Tags  â”‚â”€â”€â–¶ Apply to full-res metadata  â”‚
â”‚                       â”‚ â€¢ Descriptions   â”‚â”€â”€â–¶ IPTC captions               â”‚
â”‚                       â”‚ â€¢ Quality Scores â”‚â”€â”€â–¶ Smart collections           â”‚
â”‚                       â”‚ â€¢ Masks (scaled) â”‚â”€â”€â–¶ Upscale & apply to full-res â”‚
â”‚                       â”‚ â€¢ Adjustments    â”‚â”€â”€â–¶ Recipe parameters           â”‚
â”‚                       â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                                 â”‚
â”‚                                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

#### 10.10.6.2 Proxy Generation

```typescript
interface ProxySettings {
  // Size settings
  long_edge: 2048 | 3072 | 4096;  // pixels
  format: 'jpeg' | 'webp' | 'avif';
  quality: number;  // 80-95 recommended
  
  // Color settings  
  color_space: 'sRGB';  // Always sRGB for AI compatibility
  embed_profile: boolean;
  
  // Generation trigger
  generate_on: 'import' | 'first_view' | 'manual';
  
  // Storage
  location: 'alongside' | 'cache_directory';
}

const defaultProxySettings: ProxySettings = {
  long_edge: 2048,
  format: 'jpeg',
  quality: 85,
  color_space: 'sRGB',
  embed_profile: true,
  generate_on: 'import',
  location: 'cache_directory'
};
```

#### 10.10.6.3 AI Output Application

When AI processes a proxy, outputs must be mapped back to full resolution:

| AI Output | Application Method | Fidelity |
|-----------|-------------------|----------|
| Keywords/Tags | Direct metadata write | 100% |
| Descriptions | Direct metadata write | 100% |
| Quality Scores | Catalog field update | 100% |
| Adjustment Suggestions | Recipe parameters (exposure, WB, etc.) | 100% |
| Masks | Scale up with interpolation + edge refinement | 95-98% |
| Crop Suggestions | Ratio-based, recalculate for full-res | 100% |
| Face Regions | Proportional coordinate mapping | 98% |

#### 10.10.6.4 Region-Based Processing

For AI operations on specific regions of high-res files:

```typescript
interface RegionProcessingRequest {
  source_photo_id: UUID;
  region: {
    type: 'rectangle' | 'mask';
    // For rectangle:
    x: number;  // Percentage 0-100
    y: number;
    width: number;
    height: number;
    // For mask:
    mask_id?: UUID;
  };
  max_dimension: number;  // Max px for extracted region
  operation: 'face_restore' | 'detail_enhance' | 'inpaint' | 'style_transfer';
  operation_params: Record<string, unknown>;
}

// Example: Face restoration on GFX100S portrait
const request: RegionProcessingRequest = {
  source_photo_id: 'uuid-here',
  region: {
    type: 'rectangle',
    x: 30,   // Face is 30% from left
    y: 10,   // 10% from top  
    width: 40,   // Face spans 40% of width
    height: 60   // 60% of height
  },
  max_dimension: 2048,  // Will extract and resize to max 2048px
  operation: 'face_restore',
  operation_params: {
    model: 'codeformer',
    fidelity: 0.7
  }
};
```

---

### 10.10.7 File Format Support Matrix

#### 10.10.7.1 Import Formats

| Format | Extensions | Library | Notes |
|--------|------------|---------|-------|
| JPEG | .jpg, .jpeg | libjpeg-turbo | Full EXIF preservation |
| PNG | .png | libpng | 8/16-bit, alpha |
| TIFF | .tif, .tiff | libtiff | All compressions, layers |
| WebP | .webp | libwebp | Lossy and lossless |
| HEIF/HEIC | .heif, .heic | libheif | HDR support |
| AVIF | .avif | libavif | HDR, wide gamut |
| PSD | .psd, .psb | custom parser | Layers, adjustments (subset) |
| Camera RAW | 100+ formats | LibRaw | See LibRaw supported cameras |
| DNG | .dng | LibRaw + DNG SDK | Linear and mosaic |
| OpenRaster | .ora | ZIP + XML | Full layer support |
| JPEG XL | .jxl | libjxl | Progressive, HDR |
| OpenEXR | .exr | OpenEXR | HDR, deep compositing |
| XMP Sidecar | .xmp | exifr/custom | Adobe Camera Raw import |
| LCP | .lcp | custom parser | Adobe Lens Profiles |
| PDF | .pdf | Docling/Poppler | Document import (NEW) |
| DOCX | .docx | Docling | Document import (NEW) |
| Video | .mp4, .mov, .webm | ffmpeg.wasm | Frame extraction (NEW) |

#### 10.10.7.2 Export Formats

| Format | Extensions | Options |
|--------|------------|---------|
| JPEG | .jpg | Quality 1-100, subsampling, progressive |
| PNG | .png | 8/16-bit, compression level, interlace |
| TIFF | .tif | None/LZW/ZIP/JPEG compression, 8/16/32-bit |
| WebP | .webp | Quality, lossless |
| AVIF | .avif | Quality, speed, 8/10/12-bit |
| HEIF | .heif | Quality |
| DNG | .dng | Lossy/lossless, embed original option |
| PSD | .psd | Layers, compatibility mode |
| PDF | .pdf | Quality, color space |
| ORA | .ora | Full layer preservation |
| JPEG XL | .jxl | Quality, effort, lossless |
| Video | .mp4, .webm | Via ffmpeg.wasm (NEW) |

#### 10.10.7.3 Internal Formats

| Purpose | Format | Schema |
|---------|--------|--------|
| Edit Recipe | `.hs.recipe.json` | JSON Schema v3 |
| Layer Document | `.hs.layers.json` | JSON Schema v2 |
| Mask (raster) | `.hs.mask.png` | 16-bit grayscale PNG |
| Mask (vector) | `.hs.mask.svg` | SVG subset |
| Preview Pyramid | `.hs.preview/` | Directory with tiles + manifest |
| Smart Preview | `.hs.smart.dng` | Reduced-resolution DNG |
| Proxy | `.hs.proxy.jpg` | JPEG/WebP at configured size (NEW) |
| Export Record | `.hs.export.json` | Provenance manifest |
| Catalog | `.hs.catalog.db` | SQLite + JSON |
| Moodboard | `.hs.moodboard.json` | JSON Schema v1 (NEW) |
| AI Metadata | `.hs.ai.json` | Vision/LLM outputs (NEW) |

---

### 10.10.8 Cross-Tool Integration Patterns

#### 10.10.8.1 Photo â†’ Document Workflows

```
Pattern: Contact Sheet Generation
  Selected photos â†’ Vision Model captions â†’ Word template â†’ PDF export

Pattern: Photo Book
  Collection â†’ Layout engine â†’ Word/PDF with embedded images

Pattern: Client Proof Sheet
  Photos + watermarks â†’ Metadata extraction â†’ Excel manifest â†’ PDF

Pattern: Technical Report
  Photo + EXIF â†’ Docling template â†’ Word document
```

#### 10.10.8.2 Document â†’ Photo Workflows

```
Pattern: Brief Extraction
  Client PDF â†’ Docling parse â†’ LLM extract requirements â†’ Task list

Pattern: Shot List Generation
  Brief document â†’ LLM analysis â†’ Calendar events + checklist

Pattern: Reference Gathering
  Brief mentions "golden hour beach" â†’ Web search â†’ Moodboard auto-population
```

#### 10.10.8.3 Moodboard Workflows

```
Pattern: Style Analysis
  Moodboard images â†’ Vision Model â†’ Style description â†’ ComfyUI prompt guidance

Pattern: Color Extraction
  Moodboard â†’ Color Thief â†’ Palette â†’ Apply to presets/grading

Pattern: Reference Matching
  Shot photo â†’ Compare to moodboard â†’ Suggest adjustments
```

#### 10.10.8.4 Calendar/Mail Integration

```
Pattern: Shoot Scheduling
  Photo EXIF dates â†’ Calendar events â†’ Auto-organize by shoot

Pattern: Delivery Notification
  Export complete â†’ LLM draft email â†’ Send to client

Pattern: Follow-up Automation
  Shoot date + 7 days â†’ Calendar reminder â†’ Draft follow-up email
```

---

### 10.10.9 Security, Consent, & Classification

#### 10.10.9.1 Content Sensitivity
```typescript
interface ContentSensitivity {
  level: 'public' | 'internal' | 'confidential' | 'restricted';
  contains_faces: boolean;
  face_consent: ConsentStatus[];
  contains_location: boolean;
  location_consent: 'allowed' | 'strip' | 'blur';
  custom_restrictions: string[];
}
```

#### 10.10.9.2 Export Policies
- `exportable: false` assets MUST NOT be exported without override
- Face regions MAY require consent verification before export
- GPS data MAY be stripped based on policy
- Watermarking MAY be enforced for certain sensitivity levels

#### 10.10.9.3 External Service Controls
- All cloud/ML services MUST be opt-in
- Local-first processing MUST be default
- Data sent externally MUST be logged with consent

#### 10.10.9.4 AI Data Handling
- AI models run locally by default
- No image data sent to external services without explicit consent
- AI-generated metadata clearly marked as such
- Option to disable AI features entirely

---

### 10.10.10 Implementation Phases
#### 10.10.10.1 Phase 1: Foundation (Months 1-6)
- [ ] Core data model implementation
- [ ] RAW decode integration (LibRaw)
- [ ] Basic develop pipeline (exposure, WB, contrast)
- [ ] Preview pyramid generation
- [ ] Simple catalog/DAM
- [ ] JPEG/PNG/TIFF export
- [ ] Proxy generation system
- [ ] Basic metadata handling (exifr)

#### 10.10.10.2 Phase 2: Develop Parity (Months 7-12)
- [ ] Complete global adjustments
- [ ] Tone curve, HSL, color grading
- [ ] Lens corrections (Lensfun integration)
- [ ] Detail panel (sharpening, NR)
- [ ] Transform/perspective
- [ ] Manual mask tools (brush, gradients)
- [ ] Preset system
- [ ] Vision model integration (basic tagging)

#### 10.10.10.3 Phase 3: AI & Merge (Months 13-18)
- [ ] AI mask integration (SAM)
- [ ] AI denoise
- [ ] HDR merge
- [ ] Panorama stitching
- [ ] Focus stacking
- [ ] Smart previews
- [ ] Local LLM integration
- [ ] ComfyUI integration (proxy images)
- [ ] Moodboard system

#### 10.10.10.4 Phase 4: Compositor (Months 19-24)
- [ ] Layer document model
- [ ] All blend modes
- [ ] Adjustment layers
- [ ] Live filters
- [ ] Layer masks
- [ ] Basic vector/text layers
- [ ] Docling integration
- [ ] Cross-tool workflows

#### 10.10.10.5 Phase 5: Polish & Parity (Months 25-30)
- [ ] Remaining Lightroom tools
- [ ] Remaining Affinity features
- [ ] Performance optimization
- [ ] Extended format support
- [ ] Batch/automation
- [ ] Advanced vector tools
- [ ] MCP server implementation
- [ ] Full AI pipeline orchestration

---

### 10.10.11 Appendix A: Blend Mode Formulas

```
// All formulas assume RGB values normalized to [0, 1]
// a = base (bottom layer), b = blend (top layer)

Normal:      result = b
Multiply:    result = a * b
Screen:      result = 1 - (1 - a) * (1 - b)
Overlay:     result = a < 0.5 ? 2 * a * b : 1 - 2 * (1 - a) * (1 - b)
Soft Light:  result = b < 0.5 ? a - (1 - 2*b) * a * (1 - a) : a + (2*b - 1) * (D(a) - a)
             where D(x) = x <= 0.25 ? ((16*x - 12)*x + 4)*x : sqrt(x)
Hard Light:  result = b < 0.5 ? 2 * a * b : 1 - 2 * (1 - a) * (1 - b)
Color Dodge: result = b == 1 ? 1 : min(1, a / (1 - b))
Color Burn:  result = b == 0 ? 0 : 1 - min(1, (1 - a) / b)
Darken:      result = min(a, b)
Lighten:     result = max(a, b)
Difference:  result = abs(a - b)
Exclusion:   result = a + b - 2 * a * b
Hue:         result = SetLum(SetSat(b, Sat(a)), Lum(a))
Saturation:  result = SetLum(SetSat(a, Sat(b)), Lum(a))
Color:       result = SetLum(b, Lum(a))
Luminosity:  result = SetLum(a, Lum(b))
```

---

### 10.10.12 Appendix B: Open-Source License Summary

| License | Commercial Use | Modification | Distribution | Patent Grant | Copyleft |
|---------|---------------|--------------|--------------|--------------|----------|
| MIT | âœ“ | âœ“ | âœ“ | âœ— | âœ— |
| BSD 2/3-Clause | âœ“ | âœ“ | âœ“ | âœ— | âœ— |
| Apache 2.0 | âœ“ | âœ“ | âœ“ | âœ“ | âœ— |
| LGPL 2.1/3.0 | âœ“ | âœ“ | âœ“ | âœ— | Weak (dynamic linking OK) |
| MPL 2.0 | âœ“ | âœ“ | âœ“ | âœ“ | File-level |
| GPL 2.0/3.0 | âœ“ | âœ“ | âœ“ | âœ— | Strong (derivatives must be GPL) |

**Handshake Preference Order:**
1. Apache 2.0 / MIT / BSD (most permissive, best for future monetization)
2. LGPL (dynamic linking acceptable)
3. MPL 2.0 (file-level copyleft manageable)
4. GPL (avoid unless isolated subprocess)

---

### 10.10.13 Appendix C: Reference Implementation Notes

#### 10.10.13.1 darktable Module Mapping

| darktable Module | Handshake Equivalent | Notes |
|------------------|---------------------|-------|
| `exposure` | BasicAdjustments.exposure | Direct mapping |
| `colorbalancergb` | ColorGradingSettings | Similar 3-way + global |
| `filmic rgb` | Custom tone curve variant | May inspire implementation |
| `lens correction` | LensCorrectionSettings | Both use Lensfun |
| `denoise (profiled)` | engine.ai_enhance.denoise | Different approach (ML vs profiled) |
| `retouch` | SpotRemoval[] | Similar heal/clone |
| `liquify` | (Phase 5) | Mesh warp tool |

#### 10.10.13.2 Affinity Photo Feature Mapping

| Affinity Feature | Handshake Equivalent | Notes |
|------------------|---------------------|-------|
| Develop Persona | engine.photo_develop | Similar RAW workflow |
| Photo Persona | Layer compositor | Main editing |
| Liquify Persona | (Phase 5) | Warp tools |
| Tone Mapping Persona | HDR processing | Subset |
| Export Persona | engine.export | Slice support Phase 5 |
| Live Filters | LiveFilterLayer | Non-destructive filters |
| Macros | Batch/automation system | Recording + playback |

#### 10.10.13.3 Camera Support Matrix (High-Resolution)

| Camera | Resolution | Proxy Long Edge | Notes |
|--------|-----------|-----------------|-------|
| Canon R5 | 45MP | 2048px | Full RAW support via LibRaw |
| Canon R5 II | 45MP | 2048px | Full RAW support via LibRaw |
| Sony A7RV | 61MP | 2048px | Full RAW support via LibRaw |
| Sony A1 | 50MP | 2048px | Full RAW support via LibRaw |
| Fuji GFX100S | 102MP | 3072px | Full RAW support via LibRaw |
| Fuji GFX100 II | 102MP | 3072px | Full RAW support via LibRaw |
| Hasselblad X2D | 100MP | 3072px | Full RAW support via LibRaw |
| Phase One IQ4 | 150MP | 4096px | Full RAW support via LibRaw |

---

### 10.10.14 Appendix D: AI Model Configuration

#### 10.10.14.1 Vision Model Defaults

```typescript
const visionModelConfig = {
  default_model: 'minicpm-v-2.6',
  fallback_model: 'qwen2-vl-7b',
  
  models: {
    'minicpm-v-2.6': {
      max_image_pixels: 1800000,  // ~1344Ã—1344 or 1680Ã—1120
      vram_required: '8GB',
      strengths: ['high-res', 'general'],
    },
    'qwen2-vl-7b': {
      max_image_pixels: 1048576,  // 1024Ã—1024
      vram_required: '8GB',
      strengths: ['ocr', 'multilingual'],
    },
    'molmo-7b': {
      max_image_pixels: 1048576,
      vram_required: '8GB',
      strengths: ['general', 'permissive-license'],
    },
  },
  
  tasks: {
    'auto_tag': { prompt_template: '...', max_tokens: 100 },
    'caption': { prompt_template: '...', max_tokens: 200 },
    'quality_assess': { prompt_template: '...', max_tokens: 50 },
  }
};
```

#### 10.10.14.2 LLM Defaults

```typescript
const llmConfig = {
  default_model: 'llama-3.1-8b',
  creative_model: 'mythomax-13b',
  
  models: {
    'llama-3.1-8b': {
      context_length: 8192,
      vram_required: '8GB',
      strengths: ['reasoning', 'instruction-following'],
    },
    'llama-3.1-70b': {
      context_length: 8192,
      vram_required: '40GB',
      strengths: ['complex-reasoning', 'accuracy'],
    },
    'mythomax-13b': {
      context_length: 4096,
      vram_required: '12GB',
      strengths: ['creative-writing', 'storytelling'],
    },
  },
  
  tasks: {
    'email_draft': { model: 'mythomax-13b', temperature: 0.7 },
    'metadata_extract': { model: 'llama-3.1-8b', temperature: 0.1 },
    'brief_analysis': { model: 'llama-3.1-8b', temperature: 0.3 },
  }
};
```

---

### 10.10.15 Appendix E: ComfyUI Workflow Examples

#### 10.10.15.1 Moodboard Image Upscale

```json
{
  "workflow_id": "moodboard_upscale_2x",
  "description": "Upscale web image 2x for moodboard use",
  "max_input_size": 2048,
  "nodes": {
    "load": { "type": "LoadImage" },
    "upscale": { 
      "type": "ImageUpscaleWithModel",
      "model": "RealESRGAN_x2plus"
    },
    "save": { "type": "SaveImage" }
  }
}
```

#### 10.10.15.2 Style Transfer for Reference Matching

```json
{
  "workflow_id": "style_match",
  "description": "Match web image style to project aesthetic",
  "max_input_size": 1024,
  "nodes": {
    "load_content": { "type": "LoadImage" },
    "load_style": { "type": "LoadImage" },
    "transfer": {
      "type": "StyleTransfer",
      "strength": 0.7
    },
    "save": { "type": "SaveImage" }
  }
}
```

---

### 10.10.16 Document History
| Version | Date | Author | Changes |
|---------|------|--------|---------|
| v0.1.0 | 2025-12-24 | Initial | Base specification |
| v0.2.0 | 2025-12-24 | Expanded | Complete tool inventory, OSS matrix, custom requirements |
| v0.3.0 | 2025-12-24 | AI Integration | Added: Docling, vision models, local LLMs, MCP, ComfyUI scope clarification, proxy workflow, moodboard system, cross-tool integration patterns, expanded OSS matrix |

---

*End of Photo Studio specification*

## 10.11 Dev Command Center (Sidecar Integration)

[ADD v02.127] This section inlines and normalizes the content of `Handshake_Sidecar_Tech_Integration_Spec_v0.3.md` into the Master Spec as a Product Surface. It is **technology + contracts** (UI-agnostic), not a Sidecar TUI/keybinding port.

### 10.11.0 Purpose

The Dev Command Center is the canonical developer/operator surface that binds:

**work (Locus WP/MT)** â†” **workspaces (git worktrees)** â†” **execution sessions (agent/model runs)** â†” **approvals/logs/diffs**

This surface exists to make Handshakeâ€™s governed mechanical toolchain *usable*: approvals become a first-class inbox, VCS becomes a governed panel, and context/search/run queues become predictable, observable jobs.

**Doc lineage (non-authoritative):**
- Doc ID: HSK-SIDECAR-INTEGRATION
- Source: `Handshake_Sidecar_Tech_Integration_Spec_v0.3.md` (2026-02-17)
- Merge: Handshake Master Spec v02.127

**Non-negotiable constraints (normative):**
- The Dev Command Center MUST NOT bypass Workflow Engine + gates + Flight Recorder (no direct tool execution from UI).
- Local-first is default; MCP is supported but MUST NOT be required for core dev workflow.
- Git operations that rewrite/hide the working tree MUST require explicit same-turn approval (Codex **[CX-108] HARD_GIT_WORKTREE_REWRITE_CONSENT**).
- The Dev Command Center MUST NOT replace Locus; authority remains in Locus + runtime governance artifacts; Objective Anchors are non-authoritative and advisory only.

[ADD v02.159] Within this umbrella, Operator Consoles are the specialized evidence and diagnostics cluster. Dev Command Center owns control, projection, orchestration, approval, and worktree/session binding state; Operator Consoles own Problems, Jobs, Timeline, and Evidence drilldown surfaces.

[ADD v02.160] Dev Command Center control-plane state MUST project workflow runs, artificial intelligence job state, model-session scheduler snapshots, effective capability state, approval decisions, and work packet or worktree or session bindings from authoritative backend artifacts. It MAY steer or approve only through governed backend operations and MUST NOT mutate long-lived orchestration state solely through user-interface-local caches.

[ADD v02.161] Dev Command Center evidence-and-replay state MUST project Governance Pack export lifecycle, Workspace Bundle export lifecycle, diagnostics query state, and workflow-linked evidence packaging from authoritative backend artifacts. It MAY launch or poll those flows only through governed backend operations and MUST preserve stable export identifiers, manifest identifiers, and validation outcomes across projection surfaces.

[ADD v02.162] Dev Command Center work-orchestration state MUST project tracked Work Packet status, Task Board freshness, ready-query results, Micro-Task summaries, hard-gate state, workflow-linked work packet activation, and parallel model session occupancy from authoritative backend artifacts. It MUST route or steer work only through governed backend operations and MUST preserve stable work-packet, micro-task, workflow-run, and model-session identifiers across every projection surface.

[ADD v02.163] Dev Command Center planning-and-coordination state MUST also project Task Board entries, Work Packet bindings, Spec Session Log continuity, workflow-linked activation, ready-work selection, and parallel-session occupancy from authoritative backend artifacts. It MUST NOT infer work authority from kanban-only ordering, mailbox-only coordination, or packet-local prose summaries.

[ADD v02.164] Dev Command Center resilience-and-decision state MUST also project session checkpoints, session heartbeat freshness, provider capability readiness, span-linked cost and recovery posture, repository-engine backend policy, and anti-pattern alerts from authoritative backend artifacts. It MUST NOT treat user-interface badges, console-local filters, or packet prose as the authority for recovery readiness, provider safety, or version-control policy.

[ADD v02.165] Dev Command Center operating state MUST also project replay-safe run history, tool infrastructure health, workspace runtime readiness, and promotion-gate snapshots from authoritative backend artifacts. It MUST NOT infer execution chronology, tool availability, workspace safety, or protected-branch readiness from drawer-local timers, latest visible chat messages, or one-off repository command output.

[ADD v02.166] Dev Command Center collaboration state MUST also project structured Work Packet records, structured Micro-Task definitions, Task Board projection rows, append-only note timelines, and Role Mailbox triage state from authoritative backend artifacts. It MUST render typed fields before raw Markdown or raw JSON blobs and MUST NOT make prose-only summaries the only operator surface for routing, replay, or handoff decisions.

[ADD v02.167] Dev Command Center board and queue state MUST be derived from the same canonical structured artifacts that back Work Packet, Micro-Task, Task Board, and Role Mailbox records. Kanban, Jira-like, list, queue, roadmap, or timeline layouts MAY vary by view configuration, but they MUST NOT create a competing source of truth for status, scope, or routing.

[ADD v02.168] Dev Command Center typed viewers and derived layouts MUST understand the shared structured-collaboration base envelope, project-profile extension metadata, compact summaries, and mirror-state semantics. Operators MUST be able to distinguish base-envelope fields from profile-specific extensions without opening raw files or reconstructing state from prose.

[ADD v02.169] Dev Command Center mirror-governance state MUST also project mirror authority mode, reconciliation action, drift summary, last reconciliation timestamp, and manual-edit-zone posture from authoritative backend artifacts. It MUST NOT guess that a readable Markdown view is safe to trust just because it is the most recent surface an operator opened.

[ADD v02.170] Dev Command Center board, queue, list, roadmap, inbox-triage, and execution-queue operating layouts MUST be driven by explicit view presets, lane definitions, and governed action bindings. Dragging a card, reordering a queue, or firing a quick action MUST either remain a pure view preference or disclose the exact canonical field or workflow mutation before execution.

[ADD v02.171] Dev Command Center MUST also project `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from authoritative backend records. Operators and local-small-model routing flows MUST understand why work is waiting, blocked, under review, or ready from that explicit vocabulary rather than from lane position, mailbox thread order, or prose-only notes.

---

### 10.11.1 Executive summary

Sidecar (marcus/sidecar) demonstrates a practical pattern for supervising parallel, agent-assisted development work:
- workspaces backed by **git worktrees**
- task discipline that survives context resets
- unified conversation/log browsing
- explicit approvals and separation of implementation vs review

Handshake already has the macroâ†’micro primitives (Locus Work Packets + Micro-Tasks + Task Board + Flight Recorder + governed mechanical tools). The missing piece is a **control surface**: a GUI that binds:

**work** (WP/MT) â†” **workspaces** (worktrees) â†” **execution sessions** (agent/model runs) â†” **approvals/logs/diffs**

This spec defines that control surface as a Handshake module/surface (â€œDev Command Centerâ€) and adds one supporting primitive:

- **Objective Anchor Store**: a durable, queryable, local-first store for objective anchors + handoff records that remain stable across context resets and link directly to Locus WPs/MTs (authority remains in Locus).

---

### 10.11.2 Scope

#### 10.11.2.1 In scope

- **Workspaces** as first-class, **worktree-backed** entities: create/switch/archive/cleanup.
- **Execution sessions**: identify which agent/model is running what, where, and under which permissions.
- **Objective Anchor Store**
  - stable objective anchors (index + handoff hub)
  - dependencies / blocked reasons (surface Locus dependencies; optionally add pre-WP edges)
  - mandatory handoff artifacts at key boundaries
- **Git review & commit flows** (diff, revert, merge, commit)
  - â€œstaging is UI selection stateâ€ (no explicit staging op required)
- **Unified conversation timeline** (adapter-driven ingestion + search + analytics)
- **Fast codebase search** via `context.search`
- **Approval Inbox**: a surface for capability-gated â€œpending approvalsâ€ (HITL)

#### 10.11.2.2 Out of scope

- Copying Sidecarâ€™s TUI or keybindings.
- Replacing Locus WP/MT with another task engine.
- Multi-user CRDT syncing (beyond referencing master-spec direction).
- Full GitHub/CI platform integration in v0.x (optional later).
- Shipping a general-purpose remote sandbox service (local policy shells first; secure mode later).

---

### 10.11.3 Reference model: Sidecar primitives (condensed)

This section is **descriptive**, not prescriptive. It exists to anchor what is valuable from Sidecar without importing TUI constraints.

#### 10.11.3.1 Workspaces (parallel work)

- Sidecar uses workspaces, commonly implemented as **git worktrees**, enabling parallel branches/agents.
- Handshake already expects worktree-based concurrency where multiple WPs are active.

#### 10.11.3.2 Task discipline + durable state

- Sidecar â€œTDâ€ is a persistent task discipline layer that survives context resets.
- Handshake should express this as **Objective Anchor Store** (non-authoritative vs Locus).

#### 10.11.3.3 Unified conversation/log browser

- Sidecarâ€™s conversations plugin unifies agent chats/logs.
- Handshake should implement **adapter-driven log ingestion** into a normalized timeline + search, with explicit consent gates.

---

### 10.11.4 Design goals for Handshake

#### 10.11.4.1 Goals

- Make governed mechanical tools **operable**: search/diff/commit/merge/run-tests should be easy *without bypassing gates*.
- Make parallel work (worktrees) manageable: visibility, linkage to WP/MT, cleanup.
- Make context resets cheap: mandatory handoffs + durable anchor state.
- Keep tool surface lean: local-first defaults; lazy MCP/connector enablement.

#### 10.11.4.2 Non-negotiable constraints (normative)

- **No bypass**: all mechanical operations run as Workflow Engine jobs and are recorded by Flight Recorder.
- **Artifact-first**: large text inputs (commit messages, scripts, long previews) are passed by ArtifactHandle, not inline blobs.
- **Explicit approvals**: all destructive or rewrite/hide working tree operations require explicit same-turn approval.
- **Review separation**: worker/coder sessions cannot self-approve finalization into protected branches.

#### 10.11.4.3 Local-first tool routing (MCP as fallback)

- Default route:
  - `engine.context` for search/open excerpts
  - `engine.version` for VCS operations
  - `engine.sandbox` for allowlisted build/test/run
- MCP/tool plugins:
  - off by default
  - started lazily when user opens the relevant panel / enables connector
  - must publish expected tool footprint (commands, network, scopes)

---

### 10.11.5 Handshake feature set (Sidecar-derived + enriched)

#### 10.11.5.1 Project registry + state restoration

- Maintain a local registry of projects (root path + default base branch).
- Restore last opened workspace + UI state (panel selection, filters).

#### 10.11.5.2 Workspaces = git worktrees + WP linkage

- Workspaces are first-class entities. For git repos, workspace = worktree.
- Each workspace tracks:
  - repo root, worktree path, branch, base branch
  - linked `wp_id` (optional but preferred)
  - linked execution sessions (0..n)

**Workspace metadata file (durable linkage, local-first)**  
Each worktree SHOULD contain `.handshake/workspace.json`:
- restores workspaceâ†”WP/session linkage even if the central DB is lost
- enables opening the worktree in external tools without losing Handshake context
- MUST be gitignored by default
- MUST NOT contain secrets

Exact schema + migrations: see **Appendix A**.

#### 10.11.5.3 Objective Anchor Store (rename of Sidecar â€œTDâ€ concept)

Objective Anchor Store is a durable, queryable local store for:
- objective anchors (stable â€œwhat weâ€™re doingâ€ objects)
- handoff records (done/remaining/decisions/uncertainties/blockers + evidence links)
- optional â€œpre-WPâ€ dependencies (later reconciled into Locus dependencies)

Rules:
- Locus remains authoritative for WP/MT state.
- Anchors MUST link to a WP once the WP exists.
- Notes are append-only (no silent rewriting).

#### 10.11.5.4 Execution Session Manager (the â€œmodel managerâ€ control surface)

Execution Session Manager binds:
- model/agent runtime â†” workspace â†” WP/MT â†” capability grants â†” approvals â†” logs

Requirements:
- sessions have a visible status (running/paused/waiting approval/closed)
- capability grants are explicit and time-bounded
- session close requires a handoff (unless explicitly waived and logged)

#### 10.11.5.5 Approval Inbox (capability gating made usable)

- A dedicated UI surface listing pending approvals with:
  - operation summary + bounded preview (paths, diff stats, command preview)
  - requested capabilities
  - one-click approve/reject + reason
- Decisions MUST be logged to Flight Recorder.

#### 10.11.5.6 Git monitoring + review + â€œstaging is UI selection stateâ€

- UI shows status + diffs; user selects file paths to include in commit.
- Commit operation is `version.commit(message_ref, paths[])`.
- â€œStagingâ€ is not a persistent git index state; itâ€™s the UI selection.
- [ADD v02.165] Review and promotion readiness MUST surface unresolved conversations, required review state, required status-check provenance, merge-queue posture, and last verification time before any protected-branch commit, merge, or promote action is offered.

#### 10.11.5.7 Unified conversation timeline (multi-agent adapters)

- Ingest logs from supported runtimes via adapters.
- Normalize to a common schema and attach to workspace/session/WP where possible.
- Support search, filters, token/cost analytics.


#### 10.11.X Multi-Session Steering Panel [ADD v02.137]

**Addresses:** GAP D (missing operator steering surface for parallel runs).

The DCC MUST include a **Sessions panel** that provides a unified control plane for parallel model sessions:

##### 10.11.X.1 Session List View

- List all active, paused, blocked, queued, and recently completed sessions.
- Per-session display: session_id (short), role, model_id, backend, state badge, bound WP/MT, spawn depth indicator, token count, cost estimate, elapsed time.
- Grouping: by Work Packet (default), by role, by model, flat list.
- Filter: by state, by role, by backend (local/cloud).

##### 10.11.X.2 Session Actions

- **Pause/Resume:** transition session to `PAUSED` / `ACTIVE` (with checkpoint).
- **Cancel:** cancel session + cascade cancel children (with confirmation).
- **Retry:** restart from last checkpoint (creates new model_run job).
- **Route task:** send a new task (work packet or micro-task) to a specific session.
- **Inspect:** deep-link to session's message thread, Flight Recorder timeline, and bound artifacts.
- **Swap model:** trigger a ModelSwapRequest for the session (with governance approval if cloud).

##### 10.11.X.3 Spawn Tree View

- Visualize parent-child session hierarchy as an indented tree.
- Show spawn depth, active children count, and cascade-cancel scope.
- Highlight sessions approaching spawn limits.

##### 10.11.X.4 Cost Dashboard

- Real-time aggregated cost across all active sessions.
- Per-session, per-WP, per-provider cost breakdowns.
- Budget threshold indicators with configurable alerts.

##### 10.11.X.5 Approval Inbox Integration

- Cloud consent receipts pending approval surface in the Approval Inbox (existing DCC panel) with fan-out disclosure and one-click approve/deny.
- Session-scoped approvals: operator can approve all pending calls for a session in one action.


#### 10.11.5.8 Fast codebase search (`context.search`)

- Provide ripgrep-class search with snippet references, not full dumps.
- Open excerpts with `context.open_ref`.

#### 10.11.5.9 Local build/test/run queue (work packet gates as force multiplier)

- Allowlisted recipes for:
  - build
  - unit tests
  - lint/format
- Jobs run in sandbox, emit artifacts, and can be linked as evidence to handoffs/WPs.

#### 10.11.5.10 GitHub / PR integration (optional, later)

- Local-first; enable connectors only when panel is opened.
- Remote operations must require net capability + policy gate + explicit approval.

#### 10.11.5.11 Diagnostics + update UX (optional)

- Health checks for engines (context/version/sandbox).
- DCC DB integrity checks (repair/rebuild path).

#### 10.11.5.12 GUI information architecture (kanban-first, non-binding)

Non-binding surfaces:
- Workspaces list
- WP/MT inspector
- VCS status + diff + commit flow
- Search panel
- Approval Inbox
- Unified timeline (sessions/logs/tool runs)

#### 10.11.5.13 Tool Call Ledger (live feed + per-call detail) (Normative)

The Dev Command Center (DCC) is the canonical UI surface to observe, debug, and (when required) approve tool execution.

- DCC MUST provide a **live tool-call feed** backed by Flight Recorder events (AÂ§11.5).
  - Each entry MUST show: `started_at`, `tool_id@tool_version`, `side_effect`, `status`, `actor` (agent_id/model_id), `workspace_id`, and `trace_id`.
- DCC MUST provide a **per-call detail view**:
  - arguments (redacted) and/or `args_ref`
  - outputs/errors and/or `result_ref`
  - resources touched (files/URLs/workspace entities/artifacts)
  - required capabilities + approval/hold status
  - timing metrics (duration, retries, timeouts)
- DCC MUST support **approvals/holds** for `WRITE`/`EXECUTE` tool calls (AÂ§11.1), including per-agent capability settings.
- Scaling with parallel models:
  - the feed MUST be filterable by `agent_id`, `model_id`, `workspace_id`, `work_packet_id`, and `trace_id`
  - ordering MUST be stable (by `started_at`, then `tool_call_id`)
  - nested tool calls MUST be groupable by `parent_span_id` (e.g., â€œCode Modeâ€ sandbox runs)

---


#### 10.11.5.14 Front End Memory Panel (FEMS) [ADD v02.138]

The DCC MUST provide a **Front End Memory** panel to make memory observable, auditable, and operator-controlled.

**Required views**
- **Memory Browser**
  - Filter by `scope_ref` (workspace / project / WP / session / contact), `memory_class`, `type`, `trust_level`, `status`, and `classification`.
  - Show provenance: `source_refs`, `created_by_job_id`, `created_at`, `last_verified_at`, `version`, `supersedes/superseded_by`.
- **Memory Write Review (Approval Inbox integration)**
  - Display `MemoryWriteProposal` ops with evidence links.
  - Allow approve/reject per op; procedural and CRM ops MUST require explicit approval.
  - Approval emits a `MemoryCommitReport` and FR-EVT-MEM events.
- **MemoryPack Preview (per session / per model call)**
  - Show the exact `MemoryPack` (or a redacted preview) injected into a call, including token estimate and pack hash.
  - Provide a one-click â€œdisable memory for this sessionâ€ action (switch `memory_policy` to EPHEMERAL for subsequent calls).
- **Conflict / consolidation queue**
  - Surface dedupe suggestions, conflict sets, and supersedence chains.
  - Merges MUST be applied via governed jobs, never via silent in-place edits.

**Hard rules**
- UI edits MUST NOT directly mutate `MemoryItem`s. All changes MUST go through `MemoryWriteProposal â†’ commit` with logged provenance.
- The panel MUST respect classification and consent: high-sensitivity memory is not previewed/exported without explicit policy allowance.

---

#### 10.11.5.15 Recovery, Provider Readiness, and Repository Decision Surface [ADD v02.164]

The Dev Command Center MUST provide a governed recovery and decision surface that makes session recovery, provider readiness, and repository-engine policy visible before an operator resumes, reroutes, or finalizes work.

**Required views**
- **Recovery queue**
  - List sessions that are active-without-heartbeat, paused-for-recovery, blocked on pending tool side effects, or explicitly marked for operator inspection.
  - Each entry MUST surface `session_id`, `work_packet_id`, `micro_task_id`, `workflow_run_id`, last checkpoint age, pending tool call count, latest heartbeat age, and the last recorded recovery reason.
  - Resume and cancel actions MUST route through governed backend operations and emit Flight Recorder evidence.
- **Provider readiness**
  - Show the effective provider capability posture for each active or resumable session, including streaming, tool calling, structured output, multi-turn support, context-window limits, and any provider-specific sanitization requirements.
  - Provider readiness MUST be queryable by stable `session_id`, `provider_id`, and `workflow_run_id` values rather than inferred from one-off adapter logs.
- **Repository-engine policy**
  - Show the declared version-control backend, backend version, decision provenance, silent-fallback posture, required status checks, and merge-queue compatibility for the current workspace and protected-branch target.
  - The panel MUST make it obvious whether the workspace is operating under the default `git` command-line `product_managed_process` posture or an explicitly approved alternate backend.
- **Anti-pattern alerts**
  - Surface session and repository safety warnings before an operator approves reroute, recovery, or version-control actions.
  - Alerts MUST remain tied to explicit evidence and never rely on free-text summaries alone.

**Normative projection objects**

```yaml
# ADD v02.164
RepositoryEngineDecisionSurface:
  workspace_id: string
  repository_root: string
  selected_backend: enum [GIT_CLI_EXTERNAL_PROCESS, GO_GIT_EMBEDDED, LIBGIT2_EMBEDDED]
  backend_version: string | null
  decision_source_ref: string | null
  silent_fallback_allowed: false
  required_status_checks: string[]
  merge_queue_compatibility: enum [COMPATIBLE, INCOMPATIBLE, UNKNOWN]
  last_verified_at: string

AntiPatternAlert:
  alert_id: string
  anti_pattern_id: string
  severity: enum [INFO, WARNING, BLOCKING]
  session_id: string | null
  work_packet_id: string | null
  workflow_run_id: string | null
  evidence_ref: string
  summary: string
```

**Hard rules**
- Recovery queue rows MUST be backed by `SessionCheckpoint`, `ModelSessionSpanBinding`, `ActivitySpanBinding`, session-registry state, and workflow evidence. They MUST NOT be reconstructed only from chat transcripts or drawer-local state.
- Provider readiness MUST be backed by authoritative `ProviderCapabilities` evaluation for the session's resolved provider. Static documentation or stale onboarding notes are insufficient.
- Repository-engine policy MUST be backed by governed tool metadata and the recorded implementation decision. The Dev Command Center MUST NOT guess the backend from whichever library or command happens to answer first.
- Anti-pattern alerts MUST resolve to explicit evidence and the applicable anti-pattern identifier before they can block or warn. Generic "something looks wrong" banners are forbidden.
- When recovery, provider readiness, or repository-engine policy is incomplete, this spec version SHALL reuse the existing Dev Command Center, Provider Feature Coverage, Session Crash Recovery, Session Observability, Session Anti-Pattern Registry, Workflow Projection Correlation, and Git Engine Decision Gate backlog instead of creating duplicate stub families.

---

#### 10.11.5.16 Run History, Tool Infrastructure, Workspace Runtime, and Promotion Gates [ADD v02.165]

The Dev Command Center MUST provide a governed operating surface that makes durable execution history, tool infrastructure health, workspace runtime readiness, and promotion blockers visible before operators replay, reroute, or promote work.

**Required views**
- **Run history and replay**
  - List workflow runs, workflow node executions, tool calls, checkpoints, retries, reroutes, and operator interventions for the current workspace, work packet, and session scope.
  - Each row MUST surface `workflow_run_id`, `workflow_node_execution_id`, `session_id`, `tool_call_id`, `checkpoint_id`, queue state, attempt index, transition type, replay eligibility, and event time.
  - Replay, retry, and reroute actions MUST route through governed backend operations and preserve lineage to prior attempts.
- **Tool infrastructure registry**
  - Show each local tool, mechanical engine, Model Context Protocol server, and remote adapter that can serve the current workspace.
  - Each entry MUST surface transport kind, health state, permission scope, route policy, fallback policy, last verification timestamp, and last failure reason.
  - Operators MUST be able to distinguish contract mismatch, health failure, capability denial, and transport unavailability without reading backend logs first.
- **Workspace runtime**
  - Show workspace path, workspace class, linked work packet, isolation posture, startup readiness, startup-log reference, idle-shutdown posture, and last synchronization time.
  - External or non-managed workspaces MUST be marked explicitly rather than presented as equivalent to governed worktree-backed workspaces.
- **Promotion gate snapshot**
  - Show the target branch/ruleset source, unresolved conversations, required review state, required status checks, status-check provenance, merge-queue posture, stale-review posture, and blocker references for the current protected-branch target.
  - Protected-branch actions MUST remain disabled when the snapshot is stale, incomplete, or blocking.

**Normative projection objects**

```yaml
# ADD v02.165
DevCommandRunHistoryEntry:
  history_entry_id: string
  workspace_id: string
  work_packet_id: string | null
  workflow_run_id: string
  workflow_node_execution_id: string | null
  session_id: string | null
  tool_call_id: string | null
  checkpoint_id: string | null
  queue_state: string
  transition_type: string
  attempt_index: int
  replay_eligible: bool
  event_time: string
  evidence_ref: string

ToolInfrastructureStatus:
  status_id: string
  tool_or_server_id: string
  display_name: string
  transport_kind: enum [LOCAL_PROCESS, MECHANICAL_ENGINE, MODEL_CONTEXT_PROTOCOL, HTTP_API]
  health_state: enum [READY, DEGRADED, UNAVAILABLE, UNKNOWN]
  permission_scope: string[]
  route_policy_ref: string | null
  fallback_policy_ref: string | null
  last_verified_at: string | null
  last_failure_ref: string | null

WorkspaceRuntimeStatus:
  workspace_id: string
  workspace_path: string
  workspace_class: enum [PRIMARY, SECONDARY, EXTERNAL]
  linked_work_packet_id: string | null
  isolation_posture: enum [WORKTREE_ISOLATED, FILE_SCOPE_LOCKED, EXTERNAL_UNMANAGED]
  startup_state: enum [READY, PREPARING, DEGRADED, FAILED]
  startup_log_ref: string | null
  idle_shutdown_at: string | null
  last_sync_at: string | null

PromotionGateSnapshot:
  workspace_id: string
  target_branch: string
  protection_source: enum [BRANCH_PROTECTION, RULESET, NONE]
  unresolved_conversation_count: int
  required_review_state: enum [SATISFIED, MISSING, STALE, BLOCKED]
  required_check_ids: string[]
  required_check_source_ref: string | null
  merge_queue_state: enum [NOT_REQUIRED, READY, QUEUED, BLOCKED, UNKNOWN]
  verified_at: string
  blocker_refs: string[]
```

**Hard rules**
- Run history MUST be backed by `WorkflowRun`, `WorkflowNodeExecution`, tool-call events, checkpoints, and governance decisions. It MUST NOT be reconstructed only from visible message order.
- Tool infrastructure registry entries MUST be backed by Tool Registry, Tool Gate, and health-check evidence. A listed tool or server is not automatically a ready tool or server.
- Workspace runtime state MUST be backed by workspace metadata, session-registry state, startup or prebuild checks, and workspace-safety policy. It MUST clearly distinguish governed worktrees from external workspaces.
- Promotion gate snapshots MUST be backed by governed repository metadata plus review/check evidence. Protected-branch actions MUST require a fresh enough snapshot rather than relying on cached optimism.
- When run history, tool infrastructure, workspace runtime, or promotion gates are incomplete, this spec version SHALL reuse the existing Dev Command Center, Workflow Projection Correlation, Unified Tool Surface Contract, Workspace Safety Parallel Sessions, and Git Engine Decision Gate backlog instead of creating duplicate stub families.

#### 10.11.5.17 Structured Work Records, Notes, and Collaboration Inbox [ADD v02.166]

The Dev Command Center MUST provide a structured viewer over canonical Work Packet, Micro-Task, Task Board, and Role Mailbox records so operators can inspect fields without parsing raw Markdown or raw JSON files directly.

**Required views**
- **Work Packet record viewer**
  - Show identity, status, dependencies, routing posture, linked sessions, linked Micro-Tasks, evidence references, and append-only note timeline from the canonical structured record.
  - The viewer SHOULD render long-form Markdown notes, but field values come from the structured record first.
  - [ADD v02.168] The viewer SHOULD separate base-envelope fields, compact summary fields, and project-profile extension fields so software-delivery-specific data does not look universal.
  - [ADD v02.170] The viewer SHOULD also surface the active view preset, layout badges, and governed next-action bindings so operators can tell whether a move, review request, or promotion action changes authoritative state or only changes the current view.
  - [ADD v02.171] The viewer SHOULD also surface `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` so packet routing, review, approval, validation, and completion posture stay explicit even when project-profile labels differ.
- **Micro-Task record viewer**
  - Show file scope, allowed tools, retry budget, preferred execution tier, escalation target, done criteria, current iteration state, and validation evidence.
  - Local-small-model routing decisions SHOULD be understandable from this view without opening packet prose first.
  - [ADD v02.168] If a compact summary exists, Dev Command Center SHOULD load the summary first and progressively hydrate the canonical detail record only when deeper inspection is needed.
  - [ADD v02.170] The viewer SHOULD expose readiness buckets such as `ready_for_local_small_model`, `blocked_on_human`, `escalation_required`, `validation_required`, and `awaiting_mailbox_response` from explicit fields or queue presets rather than ad hoc lane heuristics.
  - [ADD v02.171] The viewer SHOULD show which base workflow-state family and queue reason produced the current readiness bucket, plus which governed actions are currently allowed.
- **Task Board projection viewer**
  - Show structured board rows keyed by stable `task_board_id` and `work_packet_id`, plus freshness, manual-edit detection, and sync status.
  - Any Markdown board is read-only by default from this view unless a governed sync or status-update workflow is being invoked.
  - [ADD v02.168] Board rows SHOULD expose the base-envelope status, next action, blockers, and project-profile kind before board-specific grouping metadata.
  - [ADD v02.170] Board, list, queue, and roadmap layouts SHOULD read from the same row set and declare which lane definitions, grouping keys, and action bindings are active for the current preset.
  - [ADD v02.171] Board rows SHOULD expose `workflow_state_family` and `queue_reason_code` separately from any project-specific display label so queue semantics remain portable across project kernels.
- **Role Mailbox triage**
  - Show message type, expected response, expiry, evidence references, linked Work Packet or Micro-Task identifiers, and handoff completeness.
  - Role Mailbox remains non-authoritative, but Dev Command Center MUST make collaboration state queryable without reading transcript blobs line by line.
  - [ADD v02.168] Thread and message views SHOULD expose the shared base-envelope fields and any mailbox-specific profile extensions separately.
  - [ADD v02.170] Inbox-triage presets SHOULD group by expected response, expiry, linked work identifier, or escalation posture, and MUST keep any reply or escalation mutation visibly separate from non-authoritative message text.
  - [ADD v02.171] Mailbox rows SHOULD show when expected-response or escalation posture contributes to a linked record's `queue_reason_code`, without turning the mailbox thread into the authority for the linked record's `workflow_state_family`.
  - [ADD v02.173] Mailbox rows SHOULD also surface `thread_lifecycle_state`, `message_delivery_state`, `allowed_responses`, due posture, snooze posture, and dead-letter posture so operators can distinguish triage state from governed state mutation.
- **Local-small-model execution queue**
  - Show the compact-summary-first queue for ready Micro-Tasks, human-blocked Micro-Tasks, escalation-required Micro-Tasks, validation-required Micro-Tasks, and mailbox-response-dependent work.
  - The queue SHOULD explain why an item is present, what action bindings are available, and whether the preferred executor is local or cloud-backed.
  - [ADD v02.171] Queue membership SHOULD be explainable directly from `workflow_state_family`, `queue_reason_code`, preferred executor, and allowed action ids before any note body or Markdown mirror is opened.
- **Notes drawer**
  - Render append-only Markdown or JSON note bodies attached to Work Packet or Role Mailbox records.
  - Notes MUST stay linkable to the structured records that own them.
  - [ADD v02.169] Generated Markdown mirrors, advisory human edits, and append-only note sidecars SHOULD be visually separated so operators can see what is derived, what is canonical, and what still needs normalization.

**Hard rules**
- When both a structured record and a Markdown mirror exist, the structured record is authoritative for field values, workflow routing, and readiness state.
- Markdown editors or raw JSON editors MAY exist as advanced tools, but the operator-default surface SHOULD render typed fields and derived summaries.
- [ADD v02.168] Base-envelope fields MUST remain visible even when project-profile extensions are collapsed or unavailable, so generic viewers and local-small-model ingestion continue to function across project kernels.
- [ADD v02.169] Mirror contract metadata SHOULD surface current authority mode, reconciliation action, and last reconciliation time before any operator regenerates or edits a Markdown mirror.
- [ADD v02.170] Every board, queue, roadmap, or inbox surface MUST expose its active `view_id`, layout kind, and any action-binding identifiers before the operator acts on a card or row.
- [ADD v02.170] A lane move, queue action, or bulk action MAY mutate state only through an explicit action binding that previews target record ids plus the target field paths or workflow identifiers.
- [ADD v02.171] Typed viewers and queues MUST show the base workflow-state family and queue reason alongside any project-profile label override so project-specific wording never hides the portable routing contract.
- [ADD v02.173] Mailbox quick actions MUST distinguish mailbox-local responses such as acknowledge or snooze from governed actions that would mutate linked authoritative records, and the preview MUST surface that distinction before execution.
- Local small models SHOULD consume bounded structured Work Packet and Micro-Task fields before any long-form note bodies or Markdown mirrors.
- Note writes MUST remain append-only and create new note or message artifacts; silent in-place rewrite of prior handoff text is forbidden.
- When structured work-record viewing is incomplete, this spec version SHALL reuse the existing Dev Command Center, Locus Work Tracking, Role Mailbox, Micro-Task Executor, and Workflow Projection Correlation backlog instead of creating duplicate collaboration-surface stub families.

#### 10.11.5.18 Projected Boards and Queue Layouts over Structured Records [ADD v02.167]

The Dev Command Center MAY offer kanban, queue, list, roadmap, or future Jira-like layouts, but every such layout SHALL be a projection over canonical structured collaboration artifacts rather than a second work-tracking authority.

**Required view behavior**
- Lane, swimlane, group, and sort configuration SHOULD read from structured Work Packet, Micro-Task, Task Board, and Role Mailbox fields rather than infer state from Markdown ordering.
- Layout presets SHOULD be versioned `DevCommandCenterViewPresetV1` contracts or their equivalent so board, queue, list, roadmap, inbox-triage, and execution-queue views can be compared, replayed, and migrated without hidden user-interface state.
- Lane-based views SHOULD expose stable `TaskBoardLaneDefinitionV1` records that declare source fields, filter semantics, sort keys, work-in-progress constraints, and action-binding ids.
- A board move, queue reorder, or view regrouping MUST be treated as one of:
  - a pure view preference change, or
  - an explicit governed edit to a structured record field with stable evidence and replay semantics
- The operator MUST be able to see which structured fields power the current board or queue layout.
- [ADD v02.168] Layout viewers SHOULD show whether a lane, filter, or grouping key comes from the shared base envelope or a project-profile extension.
- Future Jira-like views MAY be added without redefining core artifact schemas, provided they consume the same project-agnostic base envelope plus profile extensions.
- [ADD v02.169] Layouts SHOULD also expose whether a card or queue row is reading synchronized Markdown, stale Markdown, or canonical-only state before the operator acts on the row.
- [ADD v02.170] Queue rows, board cards, and roadmap items SHOULD disclose the governed action binding that would fire for drag, quick action, or bulk action before any mutation occurs.
- [ADD v02.171] Board lanes and queue groupings SHOULD derive first from `workflow_state_family` and `queue_reason_code`, then optionally layer project-profile display labels or custom grouping hints on top.

**Hard rules**
- View configuration MUST NOT silently mutate authoritative Work Packet or Task Board state.
- Board projections MUST surface drift when a Markdown mirror, imported board, or ad hoc manual reorder does not match the canonical structured records.
- Dragging a card into a lane with no action binding MUST remain view-only and MUST NOT imply a hidden status change.
- Bulk actions MUST preview the affected record ids, target field paths or workflow ids, approval requirements, and evidence posture before execution.
- [ADD v02.171] View-specific labels such as `in progress`, `awaiting design review`, or `ready for operator` MUST map back to one base workflow-state family and one queue reason code before they may drive routing, queue priority, or local-small-model execution.
- Local small models SHOULD consume compact summaries and bounded structured fields that back the current board or queue, not full rendered board Markdown.
- When projected board and queue layouts are incomplete, this spec version SHALL reuse the existing Dev Command Center, Locus Work Tracking, Task Board, and Workflow Projection Correlation backlog and MAY add dedicated structured-artifact or mirror-sync stubs only where the implementation gap is genuinely new.

#### 10.11.5.19 Canonical Records, Markdown Mirrors, and Drift Reconciliation [ADD v02.169]

The Dev Command Center MUST provide a mirror-reconciliation surface so operators can see when readable Markdown mirrors lag behind canonical structured records and what the safe next action is.

**Required views**
- **Mirror reconciliation queue**
  - List canonical-only, stale, advisory-edit, and normalization-required collaboration artifacts.
  - Each row MUST surface `record_id`, `record_kind`, `mirror_state`, `authority_mode`, `reconciliation_action`, `last_reconciled_at`, and the linked canonical or mirror artifact references.
- **Canonical versus mirror diff summary**
  - Show a bounded explanation of drift such as canonical field changes, advisory human edits, missing mirror generation, or template-version mismatch.
  - The default explanation SHOULD be field-oriented and SHOULD NOT require raw Markdown diff reading before the operator knows what happened.
- **Normalization actions**
  - `regenerate_mirror` MUST rebuild the readable Markdown from canonical state and record the new mirror hash.
  - `promote_advisory_note` MUST create or update a canonical note or sidecar artifact rather than silently patching structured fields with prose.
  - `manual_resolution_required` MUST block one-click regeneration until the operator resolves the conflict explicitly.

**Hard rules**
- Regenerating a mirror MUST preserve append-only note sidecars and MUST NOT silently delete operator-authored narrative that has not been normalized yet.
- Advisory Markdown edits MUST never backdoor canonical status, blockers, routing, or readiness fields.
- When a mirror is stale or advisory, Dev Command Center SHOULD default to compact summary plus canonical detail rather than treating the readable mirror as safe.
- Mirror reconciliation decisions MUST emit durable evidence and remain replayable.

#### 10.11.5.20 Typed Viewer Presets, Lane Definitions, and Governed Actions [ADD v02.170]

The Dev Command Center MUST ship typed operating layouts that translate canonical structured records into board, queue, list, roadmap, inbox-triage, and execution-queue views without hiding field provenance or mutation semantics.

**Required operating layouts**
- **Work Packet operating layouts**
  - At minimum provide one board or list preset and one queue or roadmap preset over the same Work Packet or Task Board row set.
  - Cards or rows MUST show stable identifiers, status, blockers, next action, mirror posture, and the governed actions available from the current preset.
- **Micro-Task execution queue**
  - Provide explicit readiness buckets for local-small-model execution, human blockers, escalation-required items, validation-required items, and mailbox-response dependencies.
  - Queue membership SHOULD come from canonical fields and compact summaries rather than heuristic lane ordering.
- **Role Mailbox inbox triage**
  - Provide grouped queues for response-due, blocker, review-request, decision-request, and handoff items.
  - Each row MUST show linked Work Packet or Micro-Task identifiers plus the governed reply or escalation actions currently available.
- **Roadmap and milestone layouts**
  - Roadmap views MAY group by cycle, milestone, due date, or project-profile-specific planning keys, but they MUST remain projections over the same canonical records already visible in board or queue layouts.
- **Raw-record drilldown**
  - Raw JavaScript Object Notation or JavaScript Object Notation Lines inspection MAY exist, but it is a secondary diagnostic surface after typed fields, compact summaries, mirror badges, and action previews.

**Hard rules**
- Every non-view action exposed from a card or row MUST resolve through `ProjectionActionBindingV1`.
- The operator MUST be able to inspect the target record ids, field paths or workflow ids, approval posture, and evidence posture before executing any action binding.
- View presets MUST degrade gracefully when a project-profile extension is unknown by falling back to base-envelope fields and compact summaries.
- Local-small-model execution queues MUST stay derivable from compact summaries plus Role Mailbox response state; they MUST NOT require full Markdown packet reads as the default readiness path.
- When typed-viewer preset and layout-registry work is incomplete, this spec version SHALL reuse the existing Dev Command Center structured-viewer and mirror-sync backlog and MAY materialize a dedicated layout-projection registry stub where the implementation gap is genuinely new.

#### 10.11.5.21 Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]

The Dev Command Center and every structured collaboration artifact family member MUST share one portable workflow-state and action vocabulary so review, approval, validation, waiting, and ready semantics survive across coding, research, design, worldbuilding, and later Handshake kernels.

**Required state contract**
- Canonical records SHOULD expose:
  - `workflow_state_family`
  - `queue_reason_code`
  - `allowed_action_ids`
  - optional project-profile display labels
- `workflow_state_family` MUST remain portable across record kinds.
- `queue_reason_code` MUST explain why the record is currently grouped, queued, or blocked.
- `allowed_action_ids` MUST reference registered `GovernedActionDescriptorV1` records rather than ad hoc user-interface verbs.

**Required queue and routing behavior**
- Local-small-model queues SHOULD prefer records where:
  - `workflow_state_family=ready`
  - `queue_reason_code=ready_for_local_small_model`
- Cloud-model routing SHOULD prefer records where:
  - `workflow_state_family=ready`
  - `queue_reason_code=ready_for_cloud_model`
    or
  - `workflow_state_family=waiting`
  - `queue_reason_code=escalation_wait`
- Review and approval queues MUST distinguish:
  - `workflow_state_family=review`
  - `queue_reason_code=human_review_wait`
  - `workflow_state_family=approval`
  - `queue_reason_code=approval_wait`
- Validation queues MUST use `workflow_state_family=validation` plus explicit validation reasons rather than generic blocked state.
- Mailbox-linked waits MUST remain visible as `queue_reason_code=mailbox_response_wait`, but the mailbox thread itself MUST NOT become the authority for the linked record's state family.

**Required action behavior**
- `GovernedActionDescriptorV1` SHOULD be the reusable contract for verbs such as:
  - `start`
  - `delegate`
  - `request_review`
  - `request_decision`
  - `escalate`
  - `reply`
  - `retry`
  - `reroute`
  - `validate`
  - `approve`
  - `reject`
  - `complete`
  - `cancel`
  - `archive`
- Each governed action MUST declare the base families it may start from and the family it produces.
- A view MAY rename an action for presentation, but the underlying action id and result-family semantics MUST stay stable.

**Hard rules**
- Project-profile extensions MAY relabel families for display, but they MUST NOT change the base semantic meaning of `ready`, `waiting`, `review`, `approval`, `validation`, `blocked`, `done`, `canceled`, or `archived`.
- A record MUST NOT be routed only from freeform note text, lane position, or mailbox thread order when `workflow_state_family` and `queue_reason_code` are available.
- Unknown project-profile workflow extensions MUST degrade to the base workflow-state families, reason codes, and governed action ids rather than hiding the record.
- When workflow-state registry work is incomplete, this spec version SHALL reuse the existing structured-collaboration schema, project-profile extension, and Dev Command Center layout-registry backlog and MAY materialize one dedicated workflow-state registry stub where the implementation gap is genuinely new.

#### 10.11.5.22 Workflow Transition Matrix, Queue Automation, and Executor Eligibility [ADD v02.172]

The Dev Command Center and every structured collaboration artifact family member MUST now share one portable transition matrix so the system can explain not only current state, but who may move the state, why the move is legal, and whether the move is automatic, approval-gated, or blocked.

**Required transition contract**
- Canonical records SHOULD expose or reference:
  - `transition_rule_ids`
  - `queue_automation_rule_ids`
  - `executor_eligibility_policy_ids`
- `WorkflowTransitionRuleV1` MUST remain portable across Work Packets, Micro-Tasks, Task Board rows, and Role Mailbox-linked waits.
- A transition rule MUST identify:
  - the source family
  - the result family
  - the required action id
  - the actor kinds allowed to invoke it
  - any prerequisite or blocked reason codes
  - whether approval or evidence is required

**Required automation behavior**
- `QueueAutomationRuleV1` SHOULD be the reusable contract for triggers such as:
  - dependency cleared
  - mailbox response received
  - validation passed
  - validation failed
  - approval granted
  - approval denied
  - retry timer elapsed
  - policy block cleared
  - executor unavailable
  - operator pause lifted
- Automation MAY recommend or prepare a transition, but it MUST NOT silently cross an approval boundary.
- If an automation changes queue posture, Dev Command Center MUST show:
  - which trigger fired
  - which rule fired
  - which governed action id was invoked or recommended
  - which actor kind remains eligible to continue the work

**Required executor eligibility behavior**
- `ExecutorEligibilityPolicyV1` SHOULD be the reusable contract for executor kinds such as:
  - `operator`
  - `local_small_model`
  - `cloud_model`
  - `workflow_engine`
  - `reviewer`
  - `governance`
- Local-small-model eligibility MUST require:
  - a compact summary
  - a ready-family state
  - a compatible ready reason
  - no blocker reason that requires human judgement or approval
- Cloud-model eligibility MAY include escalation and synthesis-heavy states, but MUST still obey the same approval, evidence, and transition rules.
- Reviewer and governance eligibility MUST remain explicit for review, approval, and validation families rather than inferred from board labels.

**Hard rules**
- A board move, queue regroup, inbox quick action, or automation event MUST NOT change canonical workflow state unless a valid transition rule and an eligible actor or automation rule allow it.
- If a record has no eligible executor for its current family and reason, Dev Command Center MUST surface that as a blocked or waiting posture rather than guessing.
- Project-profile display labels MAY rename transition and queue language for presentation, but MUST NOT change the underlying transition legality, automatic-trigger posture, or executor eligibility semantics.
- When workflow-transition implementation work is incomplete, this spec version SHALL reuse the existing workflow-state registry, project-profile extension, and Dev Command Center layout-registry backlog and MAY materialize one dedicated transition-and-automation registry stub where the implementation gap is genuinely new.

#### 10.11.5.23 Role Mailbox Thread Lifecycle and Action Requests [ADD v02.173]

The Dev Command Center MUST now treat Role Mailbox as a typed asynchronous collaboration fabric rather than a transcript list. Triage views, execution queues, and handoff views MUST preserve the distinction between mailbox-local state and linked authoritative state.

**Required triage behavior**
- Mailbox triage SHOULD expose:
  - `thread_lifecycle_state`
  - `message_delivery_state`
  - `allowed_responses`
  - due and expiry posture
  - snooze posture
  - dead-letter posture
  - linked record identifiers
  - any governed action candidates attached to the action request
- Operators MUST be able to tell whether a quick action is:
  - mailbox-local only
  - a governed action request on a linked record
  - an explicit transcription request
  - an escalation that still leaves the linked work waiting

**Required Micro-Task loop behavior**
- Role Mailbox SHOULD support bounded Micro-Task loop control through message families such as:
  - `MicroTaskRequest`
  - `MicroTaskFeedback`
  - `MicroTaskVerificationNeeded`
  - `MicroTaskEscalation`
  - `MicroTaskCompletionReport`
- Micro-Task loop messages MUST remain joinable to stable Work Packet, Micro-Task, verification, and evidence identifiers so retry, verifier feedback, announce-back, and escalation do not require transcript replay to understand the current state.
- Loop checkpoints and verifier outcomes MUST remain visible beside the mailbox thread so retry, escalate, complete, and transcribe actions can be previewed from bounded state instead of full transcript replay.

**Hard rules**
- Dead-lettered or expired mailbox messages MUST surface as mailbox state first. They MAY trigger queue automation only through explicit automation and transition rules on linked records.
- A mailbox row MAY recommend a governed next action, but the recommended action MUST remain visibly separate from the mailbox transcript and MUST NOT execute implicitly just because the row was replied to.
- Unified inbox, per-role inbox, and execution-queue views MAY differ in grouping, but they MUST share the same underlying thread lifecycle, message delivery, and allowed-response contract.

### 10.11.5.24 Role Mailbox Micro-Task Loop Control [ADD v02.174]

The Dev Command Center MUST treat mailbox-driven Micro-Task loops as a bounded operational state machine, not a freeform conversation.

**Required loop projections**
- linked `thread_id`, `work_packet_id`, and `micro_task_id`
- current iteration number
- remaining retry budget
- latest structured verifier outcome
- escalation target and current executor kind
- latest loop checkpoint reference
- whether the next action is mailbox-local, governed retry, governed escalation, governed completion, or transcription-only
- whether linked Work Packet notes and Task Board waits have already been updated from the latest completion report or escalation

**Hard rules**
- Dev Command Center MUST show the latest loop checkpoint and structured verifier outcome before offering retry, escalate, verify, or complete actions.
- Operators MUST be able to tell when a mailbox reply only acknowledges the thread versus when it will request a governed Micro-Task or Work Packet mutation.
- Waiting, retrying, escalated, verification-needed, and completed loop posture MUST be explainable from compact structured state without replaying the full mailbox transcript.

### 10.11.5.25 Role Mailbox Triage Queues and Remediation Controls [ADD v02.175]

The Dev Command Center MUST treat mailbox triage as a durable queue-management surface, not just a thread list.

**Required triage projections**
- linked `thread_id`, latest `message_id`, and linked record identifiers
- triage queue state and queue age
- reminder schedule, `snooze_until`, and `expires_at`
- latest delivery failure or dead-letter cause
- chosen dead-letter disposition and required operator next step
- whether the next control is mailbox-local, automation-triggering, or a governed linked-record mutation
- whether linked Task Board, Work Packet, and Micro-Task views already reflect the latest mailbox queue posture

**Hard rules**
- Dev Command Center MUST separate mailbox queue state from linked work state. A thread may be snoozed, expired, or dead-lettered while linked work remains waiting, blocked, or review-bound.
- Reminder, unsnooze, retry-delivery, reroute, and archive controls MUST show the exact timestamps, automation or action identifiers, and linked records they affect before execution.
- Expired or dead-lettered rows MUST remain visible in a remediation queue until resolved, archived, or explicitly linked to a governed follow-up.
- Queue sorting by urgency SHOULD default to durable timestamps plus explicit triage queue state rather than unread flags or latest-message order.
- Bulk mailbox actions MUST preview which rows are mailbox-local only and which will request governed transitions on linked work.

### 10.11.5.26 Role Mailbox Executor Routing, Claim-Lease, and Response Authority [ADD v02.176]

The Dev Command Center MUST treat mailbox ownership as temporary, inspectable routing state rather than implicit assignment.

**Required routing projections**
- current claimant identity and `executor_kind`
- `claim_mode`, `claim_state`, `claimed_at`, `lease_expires_at`, and lease age
- response-authority scope for reply, clarify, reroute, escalate, resolve, and transcription actions
- takeover policy, takeover legality, and last handback or takeover reason
- whether a claim action is mailbox-local only, automation-triggering, or linked to a governed work mutation
- whether linked Task Board, Work Packet, and Micro-Task views already reflect claimant or stale-lease posture

**Hard rules**
- Dev Command Center and Role Mailbox views MUST surface actor-ineligible actions explicitly instead of hiding the underlying thread or silently allowing an unauthorized reply.
- Claim, release, renew, reroute, and takeover controls MUST preview whether they only update mailbox claim metadata or also request a governed change on linked work.
- Lease expiry MUST return the thread to visible triage with claimant-history visibility; it MUST NOT imply completion, cancellation, or resolved authority on linked work.
- Local-small-model execution queues SHOULD filter mailbox threads by claim availability and response-authority scope so a smaller model does not pull work it may read but may not answer.

### 10.11.5.27 Role Mailbox Handoff Bundle and Announce-Back Provenance [ADD v02.177]

The Dev Command Center MUST treat mailbox handoff summaries as replay-safe operator context, not as freeform notes that can silently outrank linked work authority.

**Required handoff projections**
- current handoff bundle id and source thread or message ids
- remaining work summary, unresolved blockers, changed-scope summary, recommended next actor, risk summary, and confidence posture
- announce-back provenance kind and whether it is advisory, completion, escalation, or transcription-confirmed
- linked transcription targets, transcription status, and last authoritative record update
- whether Work Packet, Micro-Task, Task Board, and Locus views already reflect the latest handoff bundle
- compact summary first, with full thread replay only as drilldown

**Hard rules**
- Dev Command Center MUST distinguish advisory announce-back from authoritative transcription-confirmed completion.
- Handoff and announce-back views MUST show provenance gaps explicitly; missing transcription links, stale handoff bundles, or unresolved scope changes MUST block optimistic done or handoff-complete badges.
- Operator handoff review MUST prefer the latest accepted structured handoff bundle over thread chronology, but MUST still offer drilldown into source thread lines and linked authoritative notes.
- Bulk handoff or announce-back actions MUST preview whether they only normalize mailbox summaries or also request governed note transcription or linked work mutation.

### 10.11.5.28 Kernel Action Catalog and Write Box Projections [ADD v02.185]

The Dev Command Center MUST expose Kernel V1 action-catalog and write-box state as typed product projections, not as raw transcript or repo-governance mirrors.

**Required projections**
- Action catalog viewer: list KernelActionCatalogV1 entries by stable action id, target authority class, input schema version, actor eligibility, approval or capability requirements, preview behavior, and allowed output receipt types.
- Write box queue: show draft write boxes by write_box_id, actor, CRDT site id, target refs, validation state, stale-state-vector posture, denial receipt, promotion receipt, and linked EventLedger events when promoted.
- Direct-edit denial view: show attempted actor, target, action, denial reason, recovery instruction, and whether the blocked edit can be normalized into an advisory write box.
- Promotion preview: before promotion, show affected target refs, current state vector, validation checks, idempotency key, expected EventLedger event types, and stale or duplicate risk.
- Projection freshness badges: distinguish live CRDT draft state, compacted snapshot state, pending promotion, accepted promotion, rejected promotion, and stale projection.

**Hard rules**
- Dev Command Center controls MAY request catalog-backed write-box actions or promotion, but they MUST NOT directly mutate EventLedger authority or silently apply CRDT updates as authority.
- Every write-capable control MUST reveal the action id and target authority class before execution.
- Denied edits MUST remain visible long enough for recovery, normalization, or audit; a denial toast alone is insufficient.
- Visual debugging and acceptance proof MUST include stable element identifiers for action catalog rows, write-box rows, denial receipts, promotion previews, and stale projection badges.

### 10.11.6 System architecture in Handshake terms

#### 10.11.6.1 Subsystems involved

- **Locus**: authoritative WP/MT + dependency graph + Task Board sync
- **Workflow Engine**: durable execution + job tracking
- **Gates**: capability checks, policy checks, determinism/budget, provenance
- **Mechanical engines**:
  - `engine.context` (`context.search`, `context.open_ref`, `context.find_todos`)
  - `engine.version` (`version.status`, `version.diff`, `version.commit`, `version.revert`, `version.merge`)
  - `engine.sandbox` (`sandbox.run_shell`)
- **Flight Recorder**: audit trail + spans
- **Artifact store**: immutable payloads referenced by handles

#### 10.11.6.2 Control path (normative)

```text
DCC UI Action
  -> Coordinator builds JobRequest
     -> Workflow Engine queues job
        -> Gate pipeline runs (global + op-specific)
           -> Engine executes (context/version/sandbox)
              -> Outputs exported as artifacts (SHA-256)
                 -> Flight Recorder logs (inputs/outputs/gates/caps)
                    -> UI updates via job events + state refresh
```

#### 10.11.6.3 Dev Command Center wiring (normative)

DCC is implemented as a module exposing a small command set that translates UI intent into **jobs** that call Handshake engines.

Recommended minimal command surface:

- Projects/workspaces:
  - `projects.list/open`
  - `workspaces.create_worktree/switch/archive/remove`
- Search:
  - `search.query/open_ref/find_todos` â†’ `engine.context`
- VCS:
  - `vcs.status/diff/commit/revert/merge` â†’ `engine.version`
- Run:
  - `run.allowlisted/custom` â†’ `engine.sandbox` (custom always routes through approval)
- Work tracking:
  - `locus.bind_workspace_to_wp`
- Continuity:
  - `anchors.create/update/link/query`
  - `handoff.write`
- Governance:
  - `approvals.list/decide`
- Session:
  - `sessions.spawn/pause/resume/close/request_capability`

Concrete engine PlannedOperation envelope templates and sample params are in **Appendix C**.

---

### 10.11.7 Data model (expanded + storage)

The goal is to avoid inventing a parallel authority to Locus while still adding the missing â€œcontrol surfaceâ€ entities.

#### 10.11.7.1 Entities (logical model)

(Types here are â€œstorage typesâ€; artifact payloads are referenced by handle strings.)

##### Project
- `project_id` (uuid)
- `name`
- `root_path`
- `repo_kind` (`git`)
- `default_base_branch`
- `last_open_workspace_id?`
- `ui_state_json?` (schema-versioned)
- `policy_json?` (MCP/log ingestion enable flags, etc.)
- `created_at`, `updated_at`

##### Workspace (worktree-backed)
- `workspace_id` (uuid)
- `project_id`
- `kind` (`root` | `worktree`)
- `role` (`human` | `coder` | `validator`)
- `branch`, `base_branch`
- `worktree_path`, `repo_root_path`
- `status` (`active` | `paused` | `archived`)
- `linked_wp_id?`, `linked_mt_id?`
- `head_rev?` (cached)
- `created_at`, `updated_at`, `last_opened_at?`

##### ExecutionSession
- `execution_session_id` (uuid)
- `workspace_id`
- `agent_type` (`worker` | `validator` | `human`)
- `agent_name`
- `model_role?`, `model_id?`
- `bound_wp_id?`, `bound_mt_id?`
- `status` (`running` | `waiting_approval` | `paused` | `interrupted` | `closed`)
- `capability_grants[]` (refs; or join table)
- `conversation_ids[]` (refs; or join table)
- `handoff_required` (bool)
- `last_handoff_id?`
- `started_at`, `ended_at?`

##### ObjectiveAnchor (Objective Anchor Store)
- `anchor_id` (uuid)
- `project_id`
- `title`
- `description_ref?` (ArtifactHandle)
- `status` (`stub` | `ready` | `in_progress` | `blocked` | `in_review` | `done` | `cancelled`)
- `acceptance_criteria_json` (array)
- `linked_wp_id?`
- `linked_mt_ids[]`
- `dependencies[]` (anchor/wp refs)
- `links[]` (workspace/session/commit/artifact refs)
- `notes_append_only[]` (note_ref artifacts + ts + author)
- `created_at`, `updated_at`

##### HandoffRecord
- `handoff_id` (uuid)
- `project_id`
- `workspace_id`
- `execution_session_id`
- `anchor_id?`
- `wp_id?`, `mt_id?`
- `summary_done_ref` (ArtifactHandle)
- `summary_remaining_ref` (ArtifactHandle)
- `decisions_json`, `uncertainties_json`, `blockers_json`
- `evidence_links[]` (diff/commit/artifact refs)
- `created_at`

##### PendingApproval
- `approval_id` (uuid)
- `created_at`
- `workspace_id`
- `execution_session_id`
- `requested_capabilities_json` (array)
- `operation_preview_json` (bounded)
- `status` (`pending` | `approved` | `rejected` | `expired`)
- `decided_by?` (`human` | `session`)
- `decided_at?`
- `decision_reason?`
- `token_id?` (if approved)

##### ApprovalToken
- `token_id` (uuid)
- `created_at`, `expires_at?`
- `scope_json`
- `granted_capabilities_json`
- `consent_receipt_ref` (ArtifactHandle or FR ref)

##### ConversationSession (normalized)
- `conversation_id` (uuid)
- `source` (adapter id)
- `source_locator` (pointer to raw logs/db; redacted as needed)
- `project_id?` / `workspace_id?` / `execution_session_id?`
- `started_at`, `ended_at?`
- `token_usage?` / `cost?`
- `transcript_ref` (ArtifactHandle)
- `messages_index_ref?` (ArtifactHandle)

#### 10.11.7.2 Adapter contract (conversation/log ingestion)

Each adapter MUST define:

- **Discovery**: enumerate sessions incrementally (not full rescan every time).
- **Parse**: map raw data â†’ normalized schema.
- **Safety**:
  - secret redaction policy
  - refusal/skip on corrupted formats
  - explicit consent gate (first enable)
- **Provenance**:
  - adapter version/hash
  - source file hashes/paths (redacted as needed)
- **Retention**:
  - default conservative retention
  - explicit â€œdelete/forgetâ€ actions

#### 10.11.7.3 Storage model (normative)

DCC uses a hybrid local-first storage model:

1) **Per-worktree**: `.handshake/workspace.json`  
- minimal durable linkage (workspace_id/project_id/branch/WP linkage)
- survives loss of central DB
- schema + migrations: Appendix A

2) **Per-project**: `.handshake/devcc.db` (SQLite)  
- indexes and queryable state for:
  - projects/workspaces
  - sessions/capability grants
  - anchors/handoffs/approvals
  - conversations index
- schema DDL (v1): Appendix B

3) **Artifact store**: `.handshake/artifacts/...`  
- all large payloads (diff patches, transcripts, summaries, scripts, receipts)
- referenced by ArtifactHandle strings in DB and workspace.json

#### 10.11.7.4 Migrations (normative)

- `.handshake/workspace.json` uses `schema_version` and supports in-place upgrades (Appendix A).
- `devcc.db` uses SQL migrations:
  - `0001_devcc_init.up.sql` / `0001_devcc_init.down.sql` (Appendix B provides both)
  - migration runner MUST be deterministic and logged to Flight Recorder.
- If DB corruption is detected:
  - workspace.json + git introspection + artifact store are sufficient to rebuild the workspace index
  - anchors/handoffs are recovered from DB; therefore DB backups should be enabled by default (local-only)

---

### 10.11.8 Governance, capabilities, and safety

#### 10.11.8.1 Capability categories (suggested)

These are additive to existing Locus capabilities (e.g., `locus.read`, `locus.write`, `locus.gate`).

- `dev.read_repo` (browse/search/read files)
- `dev.version.read` (status/diff)
- `dev.version.write` (commit/revert/merge)
- `dev.worktree.manage` (create/delete/archive)
- `dev.shell.run` (run local commands)
- `dev.logs.ingest` (read external agent logs)
- `dev.connectors.github` (PR/API)
- `dev.mcp.enable` (start MCP servers; default off)

#### 10.11.8.2 HITL defaults (safe baseline)

- Worktree creation/deletion: **confirm**
- Merge: **confirm** + preflight checks
- Commit: **confirm** unless user has explicitly granted a scoped approval token
- Push/PR creation: **confirm**
- Running arbitrary commands: **confirm** unless allowlisted
- Log ingestion: **opt-in per adapter/source**
- Enabling MCP server(s): **confirm** + show expected tool footprint

#### 10.11.8.3 Role separation enforcement (normative)

Protected branches:
- default: `project.default_base_branch` (usually `main`)
- optionally extended by repo policy (GovPack)

Rules:
- Workspaces with `role = coder`:
  - MAY commit on feature branches/worktrees.
  - MUST NOT merge/rebase into protected branches.
- Workspaces with `role = validator` or `role = human`:
  - MAY merge into protected branches (still HITL-confirmed and logged).
- Waivers:
  - require explicit approval + reason + TTL
  - are logged to Flight Recorder
  - SHOULD be off by default

#### 10.11.8.4 Git rewrite/hide consent (normative)

Operations that rewrite/hide working tree state require explicit same-turn approval:
- `version.merge` (always)
- any future â€œswitch/checkout/rebase/reset/clean/stashâ€ equivalents
- any sandbox script containing these commands

Approval preview MUST include:
- branch/ref targets
- diff stats (bounded)
- â€œwhat could be lostâ€ warning when untracked files may be affected

#### 10.11.8.5 Secrets and privacy

- No plaintext tokens in JSON config files.
- Prefer OS keychain / secret store.
- Secret redaction MUST run before Flight Recorder persistence for any process/env metadata.
- Conversation retention must be explicit; default conservative.
- Avoid keystroke logging; prefer structured job outputs.

---

### 10.11.9 Risk register (updated)

| Risk | Type | Why it matters | Required mitigation in Handshake |
|---|---|---|---|
| Upstream agent ToS conflict (log extraction) | Legal/Compliance | Ingesting local DB/log formats may be interpreted as unauthorized extraction | Adapter allow-list + explicit user consent + â€œdisable per sourceâ€ |
| Plaintext secrets in config | Security | Local compromise â†’ token compromise | Keychain storage + encryption at rest + redaction |
| Destructive git actions | Security/Operational | Agents can delete/overwrite | Capabilities + HITL on write ops + rewrite/hide consent enforcement |
| Worktree sprawl | Operational/Compliance | Worktrees evade backup/scanners | Workspace index + quotas + cleanup workflows + archive state |
| Supply chain risk (external tools) | Security | Compromised scripts | Pin versions + checksums + controlled build path |
| Duplicate authority (Anchor Store vs Locus) | Governance | Two task systems diverge | Anchor Store is non-authoritative; always link to WP when created; reconcile view derived from Locus |
| Workspace metadata accidentally committed | Operational/Security | Internal IDs leak; noisy diffs | Default `.gitignore` for `.handshake/`; never store secrets; detect tracked file and warn |
| Prompt/tool-surface bloat (MCP/plugins) | Cost/Performance | Startup prompts get huge; small models degrade | Lazy-load MCP, minimal tool manifests, tool loadouts per session, â€œtoken overhead meterâ€, ability to disable connectors |
| Approval bypass / â€œskip permsâ€ patterns | Security | Drift into unsafe mode | No bypass mode; waivers require explicit artifact + FR log + TTL |
| DB corruption / mismatch | Operational | UI cannot restore state | workspace.json + git introspection fallback + repair tool |

---

### 10.11.10 Phased implementation plan (UI-agnostic)

#### 10.11.10 Phase A â€” Read-only value (lowest risk)
- Repo search via `context.search` + file preview + Monaco links
- Git status + diff (read-only)
- Flight Recorder wiring + inspector panel basics
- Workspace index display (read-only)

#### 10.11.10 Phase B â€” Controlled write ops + storage foundation
- Implement `.handshake/workspace.json` writer + migrator
- Implement `devcc.db` (SQLite) + migrations (Appendix B)
- Commit flow using UI selection staging (`version.commit(paths[])`)
- Revert flow with confirmations
- Approval Inbox (pending approvals list + decisions logged)

#### 10.11.10 Phase C â€” Continuity + parallelism
- Objective Anchor Store (anchors + handoffs) + linking to WPs
- Worktree-backed workspace create/switch/archive/delete (job wrappers; sandbox scripts until dedicated ops)
- Execution Session Manager (spawn/attach/pause/resume/close)
- Review separation enforcement + waiver logging

#### 10.11.10 Phase D â€” Integrations + platform glue
- Build/test/run queue integrated with gates + Problems panel
- Optional PR integration (local-first; MCP fallback)
- Additional ingestion adapters (more agents)
- Retention + delete/forget UX

---

### 10.11.11 Definition of done (conformance)

A DCC feature is not considered implemented until:

1. **No-bypass**: UI cannot execute raw git/rg/shell outside Workflow Engine jobs.
2. **Capability denial**: write ops fail safely without explicit grants.
3. **Approval enforcement**: merge/revert/worktree ops require explicit approval with preview.
4. **Role enforcement**: coder role cannot merge to protected branches.
5. **Artifact-first**: commit messages/scripts/transcripts are passed by ArtifactHandle when large.
6. **Flight Recorder completeness**: every job is correlated to workspace/session/WP and stores outputs by artifact reference.
7. **Workspace recovery**: deleting `devcc.db` still allows restoring workspace linkage from workspace.json + git.
8. **No secrets**: workspace.json and devcc.db contain no plaintext secrets.

---

### 10.11.12 References (pointers)

- Sidecar upstream: `https://github.com/marcus/sidecar`  
- Sidecar docs: `https://sidecar.haplab.com/docs/intro`  
- Sidecar TD (task discipline): `https://sidecar.haplab.com/docs/td`  
- Sidecar conversations: `https://sidecar.haplab.com/docs/conversations-plugin`  
- OpenHands workspace abstraction: `https://docs.openhands.dev/sdk/arch/workspace`  
- Hook systems inspiration (sessionStart / preTool / postTool): `https://github.com/OpenHands/OpenHands/issues/11943`  
- Token overhead risk example (tool/plugin bloat): `https://github.com/anomalyco/opencode/issues/9858`  
- Handshake master: `Handshake_Master_Spec_v02.127.md`

---

### 10.11.13 High-ROI force multipliers (top 10)

These are the highest impact items to merge because they amplify existing Handshake primitives instead of inventing new ones.

| # | ROI item | Why itâ€™s high ROI | Example (concrete) |
|---:|---|---|---|
| 1 | **Execution Session Manager** | Solves the â€œmodel managerâ€ gap: binds model/agent â†” workspace â†” WP/MT â†” approvals/logs | â€œSession S-42 is running WP-17 in workspace `auth-fix` on model `worker_small`; itâ€™s waiting on `dev.shell.run` approval.â€ |
| 2 | **Mandatory handoff artifacts** | Context resets + model swaps become cheap; small local models can resume reliably | After each MT iteration, write a `handoff.md` with done/remaining/decisions/uncertainties and link it to WP notes + Anchor Store. |
| 3 | **Review separation enforcement** | Removes self-approval foot-guns and aligns with governance gates | Worker session cannot move WP to DONE; validator (or user) must pass post-work gate; waivers are explicit and logged. |
| 4 | **Approval Inbox** | Makes capability gating usable (not hidden in logs); reduces friction safely | Global inbox shows pending â€œcommit selected pathsâ€ + â€œrun testsâ€ approvals with previews and one-click decisions. |
| 5 | **Staging-as-selection commit** | Simplifies git UX and reduces â€œindex stateâ€ confusion | Select `src/auth.ts` + `tests/auth.test.ts` in UI â†’ commit job calls `version.commit(message_ref, paths=[...])`. |
| 6 | **Dependency graph + critical path view** | Locus already tracks dependencies; surfacing it unlocks real scheduling | WP-UI blocked by WP-Auth; board shows critical path and suggests parallelizable WPs. |
| 7 | **Workspace/worktree lifecycle + cleanup** | Prevents repo sprawl while enabling parallel work | â€œArchiveâ€ auto-prunes stale worktrees older than N days after DONE; shows disk usage per workspace. |
| 8 | **Local-first tool router (MCP optional)** | Avoids context/tool bloat and improves latency; still supports integrations | Git ops/search run locally; GitHub tools load only when PR panel opened; MCP starts only then. |
| 9 | **Conversation + telemetry analytics** | Flight Recorder + normalized conversations become debuggable and cost-visible | Filter timeline to tool calls; show token/cost by model; link each file edit to a diff and WP/MT. |
| 10 | **Hooks at lifecycle points** | Clean extension points for policy + automation without spaghetti | `sessionStart` auto-links workspaceâ†”WP; `preToolUse` forces approvals; `postToolUse` writes FR events + updates handoff draft. |

---

### 10.11.A â€” `.handshake/workspace.json` schema (v1.0) + migration rules

See also: `hsk_devcc_workspace.schema.v1.json` (generated artifact alongside this spec).

#### 10.11.A.1 Intent

`.handshake/workspace.json` is a per-worktree, gitignored, non-secret metadata cache used to:
- restore workspace â†” project â†” WP linkage even if `devcc.db` is lost
- allow opening a worktree in external tools without losing Handshake context

It is not authoritative; Locus + DB remain authoritative.

#### 10.11.A.2 Required rules

- MUST be gitignored by default (entire `.handshake/` dir recommended).
- MUST NOT contain secrets (tokens, cookies, private keys).
- SHOULD be small (prefer references to artifacts).
- MUST include `schema_version`.
- MUST be treated as cache; safe to delete and regenerate.

#### 10.11.A.3 Schema (Draft 2020-12 JSON Schema)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "hsk.devcc.workspace@1.0",
  "title": "Handshake Dev Command Center Workspace Metadata",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "schema_version",
    "workspace_id",
    "project_id",
    "role",
    "git",
    "created_at",
    "updated_at"
  ],
  "properties": {
    "schema_version": {
      "type": "string",
      "const": "hsk.devcc.workspace@1.0"
    },
    "workspace_id": {
      "type": "string",
      "format": "uuid"
    },
    "project_id": {
      "type": "string",
      "format": "uuid"
    },
    "role": {
      "type": "string",
      "enum": ["human", "coder", "validator"]
    },
    "git": {
      "type": "object",
      "additionalProperties": false,
      "required": ["branch", "base_branch"],
      "properties": {
        "branch": { "type": "string", "minLength": 1 },
        "base_branch": { "type": "string", "minLength": 1 },
        "head_rev_hint": { "type": "string", "minLength": 7 },
        "remote_origin_url_hint": { "type": "string" }
      }
    },
    "linkage": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "linked_wp_id": { "type": "string" },
        "linked_mt_id": { "type": "string" },
        "anchor_ids": {
          "type": "array",
          "items": { "type": "string", "format": "uuid" }
        },
        "last_execution_session_id": { "type": "string", "format": "uuid" }
      }
    },
    "paths": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "worktree_path_hint": { "type": "string" },
        "repo_root_path_hint": { "type": "string" },
        "git_common_dir_hint": { "type": "string" }
      }
    },
    "created_at": { "type": "string", "format": "date-time" },
    "updated_at": { "type": "string", "format": "date-time" },
    "last_opened_at": { "type": "string", "format": "date-time" }
  }
}
```

#### 10.11.A.4 Example

```json
{
  "schema_version": "hsk.devcc.workspace@1.0",
  "workspace_id": "2c7f0b1a-7a2f-4a8f-8a1b-0f4e5a1b3c2d",
  "project_id": "6c3f51a1-2e9e-4a1b-8a2f-6d1e3b1b9a10",
  "role": "coder",
  "git": {
    "branch": "feat/WP-123-auth-fix",
    "base_branch": "main",
    "head_rev_hint": "a1b2c3d4",
    "remote_origin_url_hint": "git@github.com:org/repo.git"
  },
  "linkage": {
    "linked_wp_id": "WP-123",
    "linked_mt_id": "MT-004",
    "anchor_ids": ["8b0c1d2e-3f4a-5b6c-7d8e-9f0a1b2c3d4e"],
    "last_execution_session_id": "8e2a9f90-9f6c-4a4c-8a30-9a0b8fbc4d11"
  },
  "paths": {
    "worktree_path_hint": "/abs/path/to/worktrees/WP-123",
    "repo_root_path_hint": "/abs/path/to/repo",
    "git_common_dir_hint": "/abs/path/to/repo/.git"
  },
  "created_at": "2026-02-17T10:30:00Z",
  "updated_at": "2026-02-17T11:05:00Z",
  "last_opened_at": "2026-02-17T11:05:00Z"
}
```

#### 10.11.A.5 Migration rules

DCC MUST support these upgrades:

- **No file present**:
  - create a new file at `.handshake/workspace.json` with:
    - new `workspace_id` (uuid)
    - existing `project_id` if known; otherwise create/select project in DCC and backfill
    - git branch/base branch from `engine.version.status` or `git rev-parse`
- **Legacy v0 file (no `schema_version`)**:
  - treat as `hsk.devcc.workspace@0`
  - expected keys: `workspace_id`, `project_id`, `branch`, `base_branch`, `linked_wp_id?`
  - upgrade mapping:
    - add `schema_version = hsk.devcc.workspace@1.0`
    - move `branch/base_branch` into `git`
    - move `linked_wp_id` into `linkage.linked_wp_id`
    - set `role = human` if unknown
    - set timestamps (`created_at` from file mtime if available; `updated_at` = now)
- **Unknown future schema_version**:
  - DCC MUST NOT overwrite.
  - show a warning and open file read-only; offer â€œexport and re-initâ€ workflow.

DCC SHOULD detect accidental git tracking:
- if `.handshake/workspace.json` is tracked, show warning and propose:
  - add to `.gitignore`
  - remove from index (`git rm --cached`) via a governed job (HITL-confirmed)

---

### 10.11.B â€” `devcc.db` SQLite schema (v1) DDL

See also: `devcc_db_schema_v1.sql` (generated artifact alongside this spec).

#### 10.11.B.1 Notes

- This is local-first state. It MUST NOT store secrets.
- Use WAL mode + foreign keys.
- Large payloads are stored in artifacts; DB stores references.

#### 10.11.B.2 Migration file: `0001_devcc_init.up.sql`

```sql
-- 0001_devcc_init.up.sql
PRAGMA foreign_keys = ON;

-- Projects
CREATE TABLE IF NOT EXISTS projects (
  project_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  root_path TEXT NOT NULL UNIQUE,
  repo_kind TEXT NOT NULL CHECK (repo_kind IN ('git')),
  default_base_branch TEXT NOT NULL,
  last_open_workspace_id TEXT NULL,
  ui_state_json TEXT NULL,
  policy_json TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- Workspaces (worktree-backed)
CREATE TABLE IF NOT EXISTS workspaces (
  workspace_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('root','worktree')),
  role TEXT NOT NULL CHECK (role IN ('human','coder','validator')),
  status TEXT NOT NULL CHECK (status IN ('active','paused','archived')),
  worktree_path TEXT NOT NULL,
  repo_root_path TEXT NOT NULL,
  branch TEXT NOT NULL,
  base_branch TEXT NOT NULL,
  head_rev TEXT NULL,
  linked_wp_id TEXT NULL,
  linked_mt_id TEXT NULL,
  workspace_json_relpath TEXT NOT NULL DEFAULT '.handshake/workspace.json',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_opened_at TEXT NULL,
  UNIQUE(project_id, worktree_path)
);

CREATE INDEX IF NOT EXISTS idx_workspaces_project ON workspaces(project_id);
CREATE INDEX IF NOT EXISTS idx_workspaces_status ON workspaces(status);
CREATE INDEX IF NOT EXISTS idx_workspaces_linked_wp ON workspaces(linked_wp_id);
CREATE INDEX IF NOT EXISTS idx_workspaces_last_opened ON workspaces(last_opened_at);

-- Approval tokens (capability grants)
CREATE TABLE IF NOT EXISTS approval_tokens (
  token_id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  expires_at TEXT NULL,
  scope_json TEXT NOT NULL,
  granted_capabilities_json TEXT NOT NULL,
  consent_receipt_ref TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tokens_expires ON approval_tokens(expires_at);

-- Execution sessions
CREATE TABLE IF NOT EXISTS execution_sessions (
  execution_session_id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  agent_type TEXT NOT NULL CHECK (agent_type IN ('worker','validator','human')),
  agent_name TEXT NOT NULL,
  model_role TEXT NULL,
  model_id TEXT NULL,
  bound_wp_id TEXT NULL,
  bound_mt_id TEXT NULL,
  status TEXT NOT NULL CHECK (status IN ('running','waiting_approval','paused','interrupted','closed')),
  started_at TEXT NOT NULL,
  ended_at TEXT NULL,
  handoff_required INTEGER NOT NULL DEFAULT 0 CHECK (handoff_required IN (0,1)),
  last_handoff_id TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_workspace ON execution_sessions(workspace_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON execution_sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON execution_sessions(started_at);

-- Session capability grants (refs to tokens/receipts)
CREATE TABLE IF NOT EXISTS session_capability_grants (
  execution_session_id TEXT NOT NULL REFERENCES execution_sessions(execution_session_id) ON DELETE CASCADE,
  capability_id TEXT NOT NULL,
  token_id TEXT NULL REFERENCES approval_tokens(token_id) ON DELETE SET NULL,
  grant_ref TEXT NOT NULL,
  expires_at TEXT NULL,
  PRIMARY KEY (execution_session_id, capability_id, grant_ref)
);

CREATE INDEX IF NOT EXISTS idx_session_caps_cap ON session_capability_grants(capability_id);

-- Objective Anchors
CREATE TABLE IF NOT EXISTS objective_anchors (
  anchor_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('stub','ready','in_progress','blocked','in_review','done','cancelled')),
  description_ref TEXT NULL,
  acceptance_criteria_json TEXT NOT NULL DEFAULT '[]',
  linked_wp_id TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_anchors_project ON objective_anchors(project_id);
CREATE INDEX IF NOT EXISTS idx_anchors_status ON objective_anchors(status);
CREATE INDEX IF NOT EXISTS idx_anchors_linked_wp ON objective_anchors(linked_wp_id);

-- Anchor â†” MT links
CREATE TABLE IF NOT EXISTS anchor_mt_links (
  anchor_id TEXT NOT NULL REFERENCES objective_anchors(anchor_id) ON DELETE CASCADE,
  mt_id TEXT NOT NULL,
  PRIMARY KEY (anchor_id, mt_id)
);

-- Anchor dependencies (anchor/wp)
CREATE TABLE IF NOT EXISTS anchor_dependencies (
  anchor_id TEXT NOT NULL REFERENCES objective_anchors(anchor_id) ON DELETE CASCADE,
  dep_kind TEXT NOT NULL CHECK (dep_kind IN ('anchor','wp')),
  dep_id TEXT NOT NULL,
  PRIMARY KEY (anchor_id, dep_kind, dep_id)
);

CREATE INDEX IF NOT EXISTS idx_anchor_deps_dep ON anchor_dependencies(dep_kind, dep_id);

-- Anchor generic links (workspace/session/commit/artifact)
CREATE TABLE IF NOT EXISTS anchor_links (
  anchor_id TEXT NOT NULL REFERENCES objective_anchors(anchor_id) ON DELETE CASCADE,
  link_kind TEXT NOT NULL CHECK (link_kind IN ('workspace','session','commit','artifact')),
  link_id TEXT NOT NULL,
  PRIMARY KEY (anchor_id, link_kind, link_id)
);

CREATE INDEX IF NOT EXISTS idx_anchor_links_target ON anchor_links(link_kind, link_id);

-- Anchor notes (append-only)
CREATE TABLE IF NOT EXISTS anchor_notes (
  note_id TEXT PRIMARY KEY,
  anchor_id TEXT NOT NULL REFERENCES objective_anchors(anchor_id) ON DELETE CASCADE,
  ts TEXT NOT NULL,
  author TEXT NOT NULL CHECK (author IN ('human','agent')),
  note_ref TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_anchor_notes_anchor_ts ON anchor_notes(anchor_id, ts);

-- Handoffs
CREATE TABLE IF NOT EXISTS handoffs (
  handoff_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
  workspace_id TEXT NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  execution_session_id TEXT NOT NULL REFERENCES execution_sessions(execution_session_id) ON DELETE CASCADE,
  anchor_id TEXT NULL REFERENCES objective_anchors(anchor_id) ON DELETE SET NULL,
  wp_id TEXT NULL,
  mt_id TEXT NULL,
  summary_done_ref TEXT NOT NULL,
  summary_remaining_ref TEXT NOT NULL,
  decisions_json TEXT NOT NULL DEFAULT '[]',
  uncertainties_json TEXT NOT NULL DEFAULT '[]',
  blockers_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_handoffs_workspace_ts ON handoffs(workspace_id, created_at);
CREATE INDEX IF NOT EXISTS idx_handoffs_session_ts ON handoffs(execution_session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_handoffs_anchor_ts ON handoffs(anchor_id, created_at);

-- Handoff evidence links
CREATE TABLE IF NOT EXISTS handoff_evidence (
  handoff_id TEXT NOT NULL REFERENCES handoffs(handoff_id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('diff','commit','artifact')),
  ref TEXT NOT NULL,
  PRIMARY KEY (handoff_id, kind, ref)
);

-- Pending approvals (Approval Inbox)
CREATE TABLE IF NOT EXISTS pending_approvals (
  approval_id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  workspace_id TEXT NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  execution_session_id TEXT NOT NULL REFERENCES execution_sessions(execution_session_id) ON DELETE CASCADE,
  requested_capabilities_json TEXT NOT NULL,
  operation_preview_json TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending','approved','rejected','expired')),
  decided_by TEXT NULL CHECK (decided_by IN ('human','session')),
  decided_at TEXT NULL,
  decision_reason TEXT NULL,
  token_id TEXT NULL REFERENCES approval_tokens(token_id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_approvals_workspace_status ON pending_approvals(workspace_id, status);
CREATE INDEX IF NOT EXISTS idx_approvals_session_status ON pending_approvals(execution_session_id, status);
CREATE INDEX IF NOT EXISTS idx_approvals_created ON pending_approvals(created_at);

-- Conversations (normalized)
CREATE TABLE IF NOT EXISTS conversation_sessions (
  conversation_id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  source_locator TEXT NOT NULL,
  project_id TEXT NULL REFERENCES projects(project_id) ON DELETE SET NULL,
  workspace_id TEXT NULL REFERENCES workspaces(workspace_id) ON DELETE SET NULL,
  execution_session_id TEXT NULL REFERENCES execution_sessions(execution_session_id) ON DELETE SET NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT NULL,
  tokens_in INTEGER NULL,
  tokens_out INTEGER NULL,
  cost_currency TEXT NULL,
  cost_amount REAL NULL,
  transcript_ref TEXT NOT NULL,
  messages_index_ref TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_convos_source ON conversation_sessions(source);
CREATE INDEX IF NOT EXISTS idx_convos_project_ts ON conversation_sessions(project_id, started_at);
CREATE INDEX IF NOT EXISTS idx_convos_workspace_ts ON conversation_sessions(workspace_id, started_at);
CREATE INDEX IF NOT EXISTS idx_convos_session_ts ON conversation_sessions(execution_session_id, started_at);

-- Session â†” Conversation mapping (many-to-many)
CREATE TABLE IF NOT EXISTS session_conversations (
  execution_session_id TEXT NOT NULL REFERENCES execution_sessions(execution_session_id) ON DELETE CASCADE,
  conversation_id TEXT NOT NULL REFERENCES conversation_sessions(conversation_id) ON DELETE CASCADE,
  PRIMARY KEY (execution_session_id, conversation_id)
);
```

#### 10.11.B.3 Migration file: `0001_devcc_init.down.sql`

```sql
-- 0001_devcc_init.down.sql
PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS session_conversations;
DROP TABLE IF EXISTS conversation_sessions;

DROP TABLE IF EXISTS pending_approvals;

DROP TABLE IF EXISTS handoff_evidence;
DROP TABLE IF EXISTS handoffs;

DROP TABLE IF EXISTS anchor_notes;
DROP TABLE IF EXISTS anchor_links;
DROP TABLE IF EXISTS anchor_dependencies;
DROP TABLE IF EXISTS anchor_mt_links;
DROP TABLE IF EXISTS objective_anchors;

DROP TABLE IF EXISTS session_capability_grants;
DROP TABLE IF EXISTS execution_sessions;

DROP TABLE IF EXISTS approval_tokens;

DROP TABLE IF EXISTS workspaces;
DROP TABLE IF EXISTS projects;
```

---

### 10.11.C â€” Concrete engine operation templates (EPO)

This appendix provides reference PlannedOperation templates (schema `poe-1.0`) aligned to the engine operations defined in `Handshake_Master_Spec_v02.127.md`.

**Important:** these are templates. Exact capability IDs and allowlists depend on the project Capability Registry.

#### 10.11.C.1 `engine.context` operations

##### `context.search`

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.context",
  "engine_version_req": ">=1.0",
  "operation": "context.search",
  "inputs": [],
  "params": {
    "query": "PlannedOperation",
    "scopes": ["/abs/path/to/worktree/src"],
    "mode": "literal",
    "k": 20,
    "output": "json"
  },
  "capabilities_requested": ["fs.read:scopes", "proc.exec:rg_allowlist"],
  "budget": { "max_duration_ms": 8000, "max_output_kb": 2048 },
  "determinism": "D3",
  "output_spec": { "kind": "artifact.dataset" }
}
```

##### `context.open_ref`

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.context",
  "engine_version_req": ">=1.0",
  "operation": "context.open_ref",
  "inputs": [],
  "params": {
    "file": "/abs/path/to/worktree/src/foo.rs",
    "range": { "start_line": 120, "end_line": 180 },
    "output": "text"
  },
  "capabilities_requested": ["fs.read:scopes"],
  "budget": { "max_duration_ms": 2000, "max_output_kb": 256 },
  "determinism": "D3",
  "output_spec": { "kind": "artifact.document" }
}
```

#### 10.11.C.2 `engine.version` operations

##### `version.status`

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.version",
  "engine_version_req": ">=1.0",
  "operation": "version.status",
  "inputs": [],
  "params": {
    "repo_ref": { "kind": "path", "path": "/abs/path/to/worktree" },
    "output": "json"
  },
  "capabilities_requested": ["fs.read:repo", "proc.exec:vcs_allowlist"],
  "budget": { "max_duration_ms": 5000, "max_output_kb": 512 },
  "determinism": "D3",
  "output_spec": { "kind": "artifact.dataset" }
}
```

##### `version.diff`

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.version",
  "engine_version_req": ">=1.0",
  "operation": "version.diff",
  "inputs": [],
  "params": {
    "repo_ref": { "kind": "path", "path": "/abs/path/to/worktree" },
    "from": "HEAD",
    "to": "WORKTREE",
    "paths": ["src/foo.rs"],
    "output": "patch"
  },
  "capabilities_requested": ["fs.read:repo", "proc.exec:vcs_allowlist"],
  "budget": { "max_duration_ms": 8000, "max_output_kb": 2048 },
  "determinism": "D3",
  "output_spec": { "kind": "artifact.document" }
}
```

##### `version.commit` (staging = UI selection state)

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.version",
  "engine_version_req": ">=1.0",
  "operation": "version.commit",
  "inputs": [
    { "kind": "artifact", "artifact_ref": "artifact://sha256/<commit_message_text>" }
  ],
  "params": {
    "repo_ref": { "kind": "path", "path": "/abs/path/to/worktree" },
    "message_ref": "artifact://sha256/<commit_message_text>",
    "paths": ["src/foo.rs", "README.md"],
    "output": "json"
  },
  "capabilities_requested": ["fs.read:repo", "fs.write:repo", "proc.exec:vcs_allowlist"],
  "budget": { "max_duration_ms": 15000, "max_output_kb": 512 },
  "determinism": "D3",
  "output_spec": { "kind": "artifact.dataset" }
}
```

##### `version.revert`

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.version",
  "engine_version_req": ">=1.0",
  "operation": "version.revert",
  "inputs": [],
  "params": {
    "repo_ref": { "kind": "path", "path": "/abs/path/to/worktree" },
    "target": "2026-02-17T12:00:00Z",
    "output": "json"
  },
  "capabilities_requested": ["fs.write:repo", "proc.exec:vcs_allowlist"],
  "budget": { "max_duration_ms": 15000, "max_output_kb": 512 },
  "determinism": "D3",
  "output_spec": { "kind": "artifact.dataset" }
}
```

##### `version.merge` (rewrite/hide consent + role restriction)

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.version",
  "engine_version_req": ">=1.0",
  "operation": "version.merge",
  "inputs": [],
  "params": {
    "repo_ref": { "kind": "path", "path": "/abs/path/to/worktree" },
    "source": "feat/WP-123",
    "strategy": "merge",
    "output": "json"
  },
  "capabilities_requested": ["fs.write:repo", "proc.exec:vcs_allowlist"],
  "budget": { "max_duration_ms": 60000, "max_output_kb": 2048 },
  "determinism": "D3",
  "output_spec": { "kind": "artifact.dataset" }
}
```

#### 10.11.C.3 `engine.sandbox` operation

##### `sandbox.run_shell`

```jsonc
{
  "schema_version": "poe-1.0",
  "op_id": "UUID",
  "engine_id": "engine.sandbox",
  "engine_version_req": ">=1.0",
  "operation": "sandbox.run_shell",
  "inputs": [
    { "kind": "artifact", "artifact_ref": "artifact://sha256/<script_text>" },
    { "kind": "artifact", "artifact_ref": "artifact://sha256/<command_allowlist_json>" },
    { "kind": "artifact", "artifact_ref": "artifact://sha256/<output_spec_json>" }
  ],
  "params": {
    "script_ref": "artifact://sha256/<script_text>",
    "shell": "bash",
    "command_allowlist_ref": "artifact://sha256/<command_allowlist_json>",
    "inputs": [],
    "output_spec_ref": "artifact://sha256/<output_spec_json>"
  },
  "capabilities_requested": ["proc.exec:<allowlist>", "fs.read:repo", "fs.write:artifacts"],
  "budget": { "max_duration_ms": 120000, "max_output_kb": 8192 },
  "determinism": "D2",
  "output_spec": { "kind": "artifact.bundle" }
}
```

---

### 10.11.D â€” Worktree lifecycle (job wrapper guidance)

Until `engine.version` grows explicit worktree ops, implement worktree creation/removal as a governed job that calls allowlisted commands in `sandbox.run_shell`.

Rules:
- Always HITL-confirm worktree add/remove.
- Enforce concurrency: one active WP â†’ one worktree.
- Always write `.handshake/workspace.json` in the new worktree after creation.
- Always log the operation to Flight Recorder with affected paths.

---

## 10.12 Diagnostics Panel (Visual Debugger Operator Surface) [ADD v02.186]

**Why**
The visual debugger (Section 6.4) and backend inspector plane (Section 6.5) produce a lot of useful operator-facing state: live WebView2 screenshots, DOM tree views, Playwright command history, swarm-session selectors, console/network event streams, focus-audit ledgers. Operators need a single surface to consume them. Section 10.5 (Operator Consoles: Debug & Diagnostics) already covers triage-loop semantics; Section 10.12 [ADD v02.186] **extends** it with the visual-debugger-specific surfaces, without duplicating Section 10.5's authority.

**What**
Defines the panel's extension relationship to Section 10.5, the surfaces it adds, the upstream contracts it consumes, and the operator interactions it supports.

---

### 10.12.1 Extension relationship

Section 10.12 extends Section 10.5 Operator Consoles: Debug & Diagnostics. It does **not** duplicate Section 10.5's content. Section 10.5 remains the authority for the operator triage loop, Problems / Jobs / Timeline / Evidence Drawer, and Debug Bundle export. Section 10.12 adds visual-debugger-specific surfaces on top.

### 10.12.2 Surfaces added

- **Live WebView2 screenshot stream** -- driven by Section 6.4.3 `kernel.visual_debug.screenshot` IPC; updates at a configurable cadence (default 1 Hz; operator can pin to 'on-demand' for headless runs).
- **DOM tree** -- driven by Section 6.4.3 `kernel.visual_debug.dom_snapshot` IPC; collapsible, with stable-element-id highlighting.
- **Playwright command history** -- chronological list of CDP commands issued in the current run; clickable to jump to the corresponding screenshot.
- **Per-step screenshots** -- captured at each Playwright step boundary; baseline-comparison overlay when a baseline exists.
- **Swarm-N session selector** -- when the swarm-agent harness (Section 6.7) is running with N>1, operator picks which session's view is currently displayed.
- **Console / network event stream** -- driven by Section 6.4.3 `kernel.visual_debug.console_stream` IPC.
- **Focus-audit ledger viewer** -- displays the Section 6.6.4 SetWinEventHook ledger; flags any foreground transition to Handshake's pid in red.

### 10.12.3 Upstream contracts

The panel is a pure consumer of:

- Section 6.4 visual debugger (screenshot + DOM + console IPC).
- Section 6.5 backend inspector plane (EventLedger replay + live tail).

The panel does not maintain its own state authority; it is a projection over Section 6.4 and Section 6.5 contracts.

### 10.12.4 Operator interaction

- **Pause / resume / step** through automation runs (driven via the Section 6.5 inspector plane `replay-drive` endpoint with operator-authorized signed envelope).
- **Replay from event log** -- pick an EventLedger row, replay the screenshot sequence up to that point.

### 10.12.5 Three-tier diagnostic model (Flight Recorder + internal_diagnostics + Palmistry) [ADD v02.196]

Handshake's diagnostic responsibility is decoupled into three tiers (HBR-INT-009 v1.5.0; Codex CX-981):

- **Flight Recorder** (Section 5.3) -- the backend BUSINESS-EVENT ledger for replay and audit (KEPT AS-IS).
- **internal_diagnostics** (Section 5.8) -- the Handshake-native, in-process self-diagnostics tool inside the native egui app (heartbeat, frame-time, resource counters, panic record, open diagnostic-event API).
- **Palmistry** (Section 6.13) -- the external, out-of-process watcher that survives freezes and crashes and captures the moment of death.

The Diagnostics Panel surfaces **internal_diagnostics events** (live heartbeat, frame-time stats, resource counters, last-N diagnostic events) and **Palmistry freeze / crash / debris records** (stale-heartbeat freezes, minidumps, recovered survivor-store evidence) in one operator surface, alongside the Section 10.5 triage loop.

**Cross-references:** Section 6.4 visual debugger; Section 6.5 backend inspector plane; Section 6.6 non-hijacking GUI (focus-audit ledger source); Section 6.7 swarm harness (session selector); Section 10.5 Operator Consoles (extended); HBR-VIS-001..005; WP-KERNEL-004 refinement acceptance criteria AC-DIAG-PANEL-EXTENSION, AC-DIAG-PANEL-SURFACES, AC-DIAG-PANEL-INTERACTION.

---

## 10.13 ModelRuntime Control Panel [ADD v02.186]

**Why**
The ModelRuntime primitive (Section 4.6) and its adapters (LlamaCppRuntime + CandleRuntime) need an operator-facing control surface: which models are loaded, which adapter hosts each, KV-cache occupancy, active LoRA stack, active steering vectors, ProcessOwnershipLedger row per model. Without a control panel, operators have to query IPC by hand. Section 10.13 [ADD v02.186] makes the control panel a first-class product surface.

**What**
Defines the displayed state, the operator actions, and the IPC contract the panel reads.

---

### 10.13.1 Displayed state

For each currently loaded model row:

- Model id + on-disk artifact path + SHA256.
- Active adapter (`LlamaCppRuntime` | `CandleRuntime`).
- KV-cache occupancy (bytes used / cap; prefix-cache hit rate; quant level).
- LoRA stack (ordered list of active LoRA ids + per-LoRA strength).
- Steering vectors active (id, layer index, intensity).
- ProcessOwnershipLedger row link (Section 5.7) -- click-through to the lifecycle row.
- Live perf stats (tokens/sec, VRAM resident, time-since-last-call).

### 10.13.2 Operator actions

- **Unload model** -- calls `ModelRuntime.unload` (Section 4.6.1); writes a `STOP` row to the ledger.
- **Swap adapter for a given model** -- unload + re-register with the alternate adapter (constrained by per-model adapter compatibility; e.g., a Mamba2 model cannot be moved to LlamaCppRuntime).
- **Inspect engine internals** -- adapter-specific drilldown (LlamaCppRuntime: per-layer KV-cache state; CandleRuntime: hook registry + active steering hooks).
- **View per-model perf stats** without leaving the panel (no need to open Section 10.12).

### 10.13.3 IPC contract

The panel reads via Tauri IPC:

- `kernel.model_runtime.list_loaded()` -- returns the row set.
- `kernel.model_runtime.capabilities(model_id)` -- returns Section 4.6.3 `ModelCapabilities`.
- `kernel.model_runtime.perf_stats(model_id)` -- returns live perf snapshot.

Capability values drive which controls in the panel are visible (e.g., the LoRA stack viewer is hidden for a model that does not declare `supports_lora=true`).

**Cross-references:** Section 4.6 ModelRuntime + LocalModelAdapter; Section 5.7 ProcessOwnershipLedger; Section 10.14 Inference Lab UI (peer panel for toggles); HBR-INT-002 (model invocation wiring); WP-KERNEL-004 refinement acceptance criteria AC-MODEL-RUNTIME-CONTROL-PANEL.

---

## 10.14 Inference Lab UI [ADD v02.186]

**Why**
The eight production inference techniques in Section 4.7.1 are each gated behind a `settings.exec_policy` knob. Without a UI, operators cannot reach the knobs without hand-editing config files. The Inference Lab UI is the per-model toggle interface, with before/after comparison so operators can see the effect of a knob change before committing it to a Work Profile. Section 10.14 [ADD v02.186] formalizes the surface.

**What**
Defines the toggle interface, per-technique controls, the experiment/save-to-Work-Profile workflow, and the MoD documentation-not-toggle boundary.

---

### 10.14.1 Toggle interface

The Inference Lab UI is a toggle interface for the eight production inference techniques (Section 4.7.1). Each toggle writes through to `settings.exec_policy` for the selected model + Work Profile combination. Toggles for techniques that the model's adapter does not support (per Section 4.6.3 `ModelCapabilities`) are **hidden, not greyed** -- the operator never sees a path they cannot use.

### 10.14.2 Per-technique controls

Each technique exposes the controls relevant to it:

- **(a) LoRA hot-swap** -- LoRA stack composer (drag-and-drop ordering; per-LoRA strength slider).
- **(b) KV caching** -- KV quant level (q4 / q8 / q4_q8_mix); prefix-cache TTL slider.
- **(c-e) Steering / Refusal / CAA** -- per-vector intensity sliders; layer-index picker; vector source (file path | derived from CAA pairs).
- **(g) Self-Speculative Decoding** -- draft-model picker; mode (ngram | draft | eagle3 when supported).
- **(h) Subquadratic** -- state-vector persistence on/off; cross-session restore on/off.
- **Before/after comparison view** -- side-by-side generation output with the current knob value vs. the proposed knob value; operator can A/B before committing.

### 10.14.3 Experiment + save to Work Profile

Operator workflow:

1. Pick a model + a base Work Profile.
2. Toggle knobs; observe before/after comparison.
3. Save the resulting `exec_policy` state as a **new Work Profile** (Section 4.3.7) OR overwrite the base profile.

This makes the lab a discovery surface, not a destructive editor.

### 10.14.4 MoD documented, not toggleable

Section 6.8 Mixture-of-Depths is documented in the lab as a future technique but is **not toggleable** in v02.186 (per Section 4.7.2 spec deferral). The lab UI shows a 'planned / deferred' badge with a link to Section 6.8.

**Cross-references:** Section 4.6 ModelRuntime + LocalModelAdapter; Section 4.7 Inference Research Lab (technique authority); Section 4.5 layer-wise inference (related compute-policy controls; shares `settings.exec_policy` schema root); Section 4.3.7 Work Profile system (save target); Section 6.8 MoD (documented-not-toggleable); WP-KERNEL-004 refinement acceptance criteria AC-INFER-LAB-UI-TOGGLES, AC-INFER-LAB-UI-AB-COMPARE, AC-INFER-LAB-UI-MOD-BADGE.

---

## 10.15 ModelManual Surface [ADD v02.186]

**Why**
[GLOBAL-BUILD-002]..[GLOBAL-BUILD-011] require every app to ship a built-in manual that a no-context model can read to operate the product. Handshake's manual is the **ModelManual** -- typed Rust structs that hold purpose / workflows / startup / commands / expected I/O / navigation / safety / failure modes / recovery steps for every wired surface. Section 10.15 [ADD v02.186] formalizes the authority location, the IPC surface, the on-demand md projection, the in-app browsing surface, the self-consistency check, and the CI gate.

**What**
Defines the seven sub-elements: authority location, IPC, on-demand projection, browsing surface, self-consistency check, GLOBAL-BUILD coverage, same-commit CI hook.

---

### 10.15.1 Authority location

The ModelManual authority lives at `src/backend/handshake_core/src/model_manual/mod.rs` (CX-200 BACKEND root). Typed Rust structs (e.g., `ManualCommand`, `ManualWorkflow`, `ManualSafetyConstraint`) hold the canonical text. There is no separate `MODEL_MANUAL.md` file that competes for authority.

### 10.15.2 IPC surface

Frontend reads the manual via Tauri IPC:

- `kernel.model_manual.get()` -- returns the full manifest as JSON.
- `kernel.model_manual.list_commands()` -- returns the command index.
- `kernel.model_manual.search(query)` -- substring + tag search.

### 10.15.3 On-demand md projection

The Markdown projection (`MODEL_MANUAL.md`) is rendered **on demand** by `just generate-model-manual-md`. This matches CX-908 (deterministic-atomic-governance-files) and `feedback_no_default_md_files` (do not auto-emit .md files). The projection is a build artifact, not an authority surface; deleting it never affects the manual content.

### 10.15.4 In-app browsing surface

Operators (and no-context models) browse the manual via the Diagnostics panel (Section 10.12) which embeds a dedicated 'Manual' tab. The tab calls the IPC contracts in Section 10.15.2; no separate browsing app is shipped.

### 10.15.5 Self-consistency check (HBR-MAN-003)

A self-consistency check (`hbr-man-003-scan.mjs`) grep-scans the Rust source for every command name, IPC channel id, schema field, config key, and CLI flag the manual names. A name in the manual that does not resolve to a Rust source symbol is a HBR-MAN-003 violation. This prevents the manual from drifting away from the wired surface.

### 10.15.6 GLOBAL-BUILD coverage

The manual content satisfies [GLOBAL-BUILD-002]..[GLOBAL-BUILD-011]:

- (002) purpose
- (003) core workflows
- (004) startup
- (005) run commands
- (006) expected inputs and outputs
- (007) navigation
- (008) safety constraints
- (009) common failure modes
- (010) recovery steps
- (011) no-context model operability

A WP that adds a new wired surface without populating the corresponding ModelManual fields fails the coverage gate.

### 10.15.7 Same-commit CI hook (HBR-MAN-001)

A CI hook (`hbr-man-001-paired-diff.mjs`) enforces:

- Any wired surface change (Rust `#[tauri::command]`, IPC channel, schema field, config key, CLI flag) MUST be accompanied by a corresponding ModelManual diff in the same commit.
- `MANUAL_VERSION` constant in `model_manual/mod.rs` MUST bump.

A commit that adds a Tauri command without updating the manual fails `just gov-check`.

**Cross-references:** HBR-MAN-001 / HBR-MAN-002 / HBR-MAN-003; [GLOBAL-BUILD-002]..[GLOBAL-BUILD-011]; CX-200 BACKEND root; CX-908 deterministic-atomic-governance-files; `feedback_no_default_md_files`; Section 10.12 Diagnostics panel (embeds the Manual tab); WP-KERNEL-004 refinement acceptance criteria AC-MODEL-MANUAL-AUTHORITY, AC-MODEL-MANUAL-IPC, AC-MODEL-MANUAL-MD-ON-DEMAND, AC-MODEL-MANUAL-SELF-CONSISTENCY, AC-MODEL-MANUAL-CI-HOOK.

### 10.15.8 UserManual migration bridge [ADD v02.192]

WP-KERNEL-009 renames the no-context manual product surface to UserManual. The existing `model_manual` Rust modules, IPC names, tests, and projections are legacy compatibility paths until the owning implementation migrates them. During the bridge, a legacy ModelManual path MAY remain in code only if it maps deterministically to a UserManualRecord authority entry and emits a compatibility receipt when used.

UserManual is the canonical product concept for operator and model operation guidance. A UserManualRecord MUST explain purpose, workflows, startup, run commands, expected inputs and outputs, navigation, safety constraints, common failure modes, recovery steps, and visual-debug/backend-navigation hooks. The in-app manual view, diagnostics manual tab, command-corpus parity, and HBR-MAN checks MUST resolve against UserManual authority even if the temporary code symbol still contains `model_manual`.

New WP-KERNEL-009 surfaces MUST add or update UserManual coverage in the same implementation unit as the wired surface. A rich editor command, ProjectKnowledgeIndex route, Loom navigation command, retrieval trace viewer, backend navigation API, or visual-debug action without UserManual coverage is a build-rule defect.

---

## 10.16 Media Annotation Overlays (Normative) [ADD v02.189]

Media Annotation Overlays are DAM-level, per-asset overlays that layer typed regions, boxes, points, and notes over a media asset (image or video frame) as durable data plus a read-only display projection. They are decoupled from Photo Studio develop recipes (Section 10.10), from PoseKit pose keypoints, and from moodboard / Canvas nodes. They reuse the Asset (PRIM-Asset) and ArtifactStore (PRIM-ArtifactService, PRIM-ArtifactManifest) contracts and the AI Job model (PRIM-AiJob). The CKC source is the MediaPane annotation layers concept. This surface composes with Loom (Section 10.12) and Photo Studio Library/DAM functions (Section 10.10.4.5) without owning catalog identity.

### 10.16.1 Purpose and Scope

1. The Media Annotation Overlay surface MUST let an operator or model attach typed annotations to an existing media asset without mutating the asset's Raw bytes (consistent with Raw/Derived/Display separation, Section 10.10.3.2).
2. Annotations are Derived data: they reference an asset by handle and never copy or re-encode the asset.
3. In scope: rectangular and point regions, freeform/polygon regions, and text notes anchored to a region or to the asset as a whole; per-asset layered grouping; export of overlays as sidecar artifacts and as an optional burned-in render.
4. Out of scope and explicitly distinct:
   - PoseKit pose keypoints (skeletal/landmark data) MUST NOT be modeled as annotation overlays; an overlay MAY reference a pose artifact by handle but MUST NOT redefine keypoint semantics.
   - Photo Studio masks (PRIM-MaskDefinition) and develop adjustments are not overlays; an overlay region MAY be promoted into a mask request via the interaction edges in Section 10.11.9 but the two record types remain separate.
   - Moodboard / Canvas nodes (PRIM-CanvasView, PRIM-ExcalidrawCanvas) are scene-graph nodes on a board, not per-asset overlays bound to a single Asset identity.

### 10.16.2 Normative Language

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, MAY are normative per RFC 2119. Where this section conflicts with a general Product Surfaces rule, the storage, execution, artifact, evidence, and quiet-operation guardrails in Sections 10.11.3 through 10.11.8 are controlling.

### 10.16.3 Storage Authority (LAW)

1. AnnotationLayer and AnnotationRegion records MUST be persisted only through the canonical authority stack: PostgreSQL for committed authority, EventLedger for the mutation history, ArtifactStore for materialized sidecars and renders, and the CRDT/write-box path for pre-promotion draft edits.
2. SQLite MUST NOT be used for any part of this surface in runtime, tests, fixtures, mocks, examples, fallbacks, caches, compatibility adapters, temporary harnesses, imports, or exports. No annotation data path may declare or depend on TECH-SQLITE.
3. Draft annotation edits (in-progress drawing, uncommitted note text) MUST route through the CRDT/write-box path; direct authoritative writes that bypass the write-box MUST be denied with durable evidence, mirroring the Kernel V1 write-box contract.
4. Promotion of a draft annotation set into PostgreSQL EventLedger authority MUST require validation, idempotency, and stale-state checks, and MUST emit a replayable receipt.
5. The read-only display projection (the rendered overlay shown in the MediaPane) is a Display-layer projection rebuilt from authority; it MUST NOT be treated as a source of truth and MUST be reconstructable from PostgreSQL + EventLedger + ArtifactStore alone.

### 10.16.4 Data Contract (Normative)

This subsection defines record and field shapes at spec altitude. It does not prescribe product code.

1. AnnotationLayer (committed authority record)
   - `layer_id` - stable ULID identity for the layer; primary correlation key across EventLedger, projection, and exports.
   - `asset_id` - handle into the Asset/Library catalog (PRIM-Asset); the overlay binds to this asset and MUST NOT embed asset bytes.
   - `asset_revision` - the asset content hash (SHA-256) the layer was authored against, so a projection can flag overlays authored against a superseded revision.
   - `frame_ref` (optional) - for video assets, a frame index or timecode the layer applies to; absent for whole-image overlays.
   - `name`, `color_tag`, `order_index` - presentation-only grouping metadata for layered display.
   - `visibility` - default render visibility (does not affect persistence).
   - `created_by_actor`, `created_at`, `updated_at`, `layer_revision` - provenance and optimistic-concurrency fields.
   - `region_ids[]` - ordered references to AnnotationRegion records in this layer.
2. AnnotationRegion (committed authority record)
   - `region_id` - stable ULID identity.
   - `layer_id` - owning layer.
   - `kind` - enum: `box` (axis-aligned rectangle), `point`, `polygon` (ordered vertices), `freeform` (open or closed path), `whole_asset` (note anchored to the asset, no geometry).
   - `geometry` - normalized coordinates in the asset's intrinsic pixel space expressed as fractions in [0.0, 1.0] of asset width/height, so geometry survives proxy/derived resolution changes (consistent with Section 10.10.3.6). For `point`: a single (x, y). For `box`: (x, y, w, h). For `polygon`/`freeform`: an ordered vertex array plus a `closed` flag.
   - `label` - short typed label string (taxonomy is operator-defined; not a fixed enum).
   - `note_text` (optional) - free text note bound to this region; rich text is out of scope for v0 (plain UTF-8).
   - `confidence` (optional) - present only when the region was proposed by an AI Job (Section 10.11.5); absent for human-authored regions.
   - `source` - enum: `human` or `ai_job`; when `ai_job`, `source_job_id` MUST reference the originating AiJob.
   - `created_at`, `updated_at`, `region_revision` - provenance and concurrency fields.
3. Identity and stability
   - `layer_id` and `region_id` MUST be stable across edits, exports, replays, and storage swaps; tools, EventLedger entries, receipts, and exports MUST cite these IDs rather than file position or display order.
   - Coordinates are storage-resolution-independent (normalized); the projection layer maps them to whatever proxy or full-resolution surface is being rendered.

### 10.16.5 Execution and AI-Assisted Annotation (LAW)

1. Any AI-assisted annotation (auto-detect regions, vision-model labeling, caption-to-region, suggested boxes) MUST execute as a governed Workflow-Engine job / AI Job (PRIM-AiJob) behind capability gates. No process-local hidden inference, no GUI scraping, and no localhost endpoint may serve as the authority source of truth.
2. Vision/LLM/ComfyUI/ASR or other external-tool calls used to propose regions MUST run as Workflow-Engine nodes with explicit capability checks; a denied capability MUST surface as an explicit failure, not a silent no-op.
3. AI-proposed regions MUST be written with `source = ai_job`, a populated `source_job_id`, and an optional `confidence`; they remain proposals until an operator or governed flow commits them. A proposal MUST NOT silently overwrite a human-authored region.
4. Secrets, cookies, tokens, and credentials used by any annotation job MUST be scrubbed from receipts, EventLedger entries, Flight Recorder evidence, and exported artifacts.

### 10.16.6 Events and Evidence (Normative)

1. Every annotation mutation MUST emit an EventLedger entry and Flight Recorder evidence (PRIM-FlightRecorder, PRIM-FlightRecorderEventBase) carrying at minimum `layer_id`, the affected `region_id` (when applicable), `asset_id`, actor, and a content hash of the post-mutation record.
2. Minimum event set:
   - `annotation_layer_created` - includes `layer_id`, `asset_id`, `asset_revision`.
   - `annotation_layer_updated` - includes `layer_id`, changed fields.
   - `annotation_region_added` - includes `region_id`, `layer_id`, `kind`, `source`.
   - `annotation_region_updated` - includes `region_id`, changed fields.
   - `annotation_region_removed` - includes `region_id`, `layer_id`.
   - `annotation_ai_proposed` - includes `source_job_id`, proposed `region_id`(s), `confidence`.
   - `annotation_export` - includes `layer_id`(s), output artifact manifest handle, export mode.
3. Each mutation MUST produce a recoverable artifact or receipt: the post-state is reconstructable from EventLedger replay, and any promotion produces a replayable receipt per Section 10.11.3.4.
4. Rejected AI proposals SHOULD be retained as Flight Recorder evidence (tagged for lost-work retrieval) consistent with the rejected-idea pattern in Section 10.2.6.

### 10.16.7 Artifacts, Export, and Portability (LAW)

1. Exported overlays (sidecars, burned-in renders, transcript-linked notes) MUST materialize through the shared ArtifactStore (PRIM-ArtifactService) with a manifest (PRIM-ArtifactManifest); the export MUST NOT write product output into .GOV.
2. Export modes:
   - Sidecar mode - overlay data serialized as a portable structured sidecar (TECH-JSON) referencing the asset by handle and content hash; this is the canonical round-trippable export and MUST re-import without semantic drift.
   - Burned-in mode (optional) - a derived render composited over a proxy or full-resolution copy of the asset, executed via a Workflow-Engine job; the source asset Raw bytes MUST NOT be mutated.
3. All paths in manifests and sidecars MUST be portable: no drive letters, user-profile paths, or machine-local mount points; assets are referenced by ArtifactStore handle, not absolute filesystem path.
4. Overlays MUST survive export and re-import, storage swaps, and EventLedger replay with their `layer_id`/`region_id` identity and normalized geometry intact.

### 10.16.8 Non-Intrusive Operation (LAW)

1. Annotation drawing, AI proposal review, and export MUST honor HBR-QUIET: no focus stealing, no foreground window popping, no global shortcut hijack, and no unbounded synthetic input while a model or background job operates this surface.
2. Background annotation jobs MUST be bounded and observable through Operator Consoles and Flight Recorder without interrupting the operator.

### 10.16.9 Interaction Edges (Normative)

1. The overlay surface reads asset identity from the Library/DAM catalog (Section 10.10.4.5) and Loom (Section 10.12); it MUST resolve an exact asset by direct handle load before any semantic search.
2. A box or polygon region MAY be promoted into a Photo Studio mask request (PRIM-MaskDefinition / PRIM-RegionProcessingRequest); promotion is an explicit, separately gated action and copies geometry, it does not merge the two record types.
3. Notes anchored to regions MAY be surfaced to Project Brain and Loom as retrieval-shaping signals, but the canonical annotation authority remains PostgreSQL + EventLedger.
4. AI proposal flows compose with the AI Job Model and Workflow Engine; transcript-linked notes (for video frames) MAY reference ASR artifacts (FEAT-ASR) by handle without owning transcript data.

### 10.16.10 UI Guidance

The MediaPane MUST present overlays as a toggleable, ordered layer list above the asset with each region selectable and editable in place; AI-proposed regions MUST be visually distinguished from human-authored regions (e.g., by source badge and confidence) and require explicit commit before they become authority. The overlay view is read-only projection until the operator enters an edit/draw mode, and all destructive edits (region removal, layer deletion) MUST be recoverable via EventLedger evidence.

## 10.17 Settings and Preferences Domain (Normative) [ADD v02.189]

**Why**
Handshake accumulates operator-tunable behavior across many surfaces: where data roots resolve, default views and densities, how long evidence and artifacts are retained, and which product capabilities are toggled on. Today these knobs are implicit, scattered, or hidden in per-surface code. Section 10.12 makes the general settings/preferences domain a first-class, governed product surface with typed records, validation, defaults, change events, and a redacted projection. This domain is deliberately NARROW: it governs general product preferences only. It is DISTINCT from LLM/provider configuration (Section 10.13 ModelRuntime, FEAT-AI-JOB-MODEL, FEAT-MODEL-SESSION-ORCHESTRATION) and DISTINCT from kernel runtime config (the kernel write box / FEAT-KERNEL-WORKSPACE-WRITE-BOX). It MUST NOT absorb, mirror, or re-author either.

**What**
Defines the preference record contract, the namespaces it covers, validation and defaulting rules, the change-event and receipt contract, the redacted projection, and the operator interaction surface. All shapes are defined at spec altitude; this section authors no product code.

> NOTE (numbering): this section is authored against requested anchor 10.12. Module 10 already binds 10.12-10.15 to the [ADD v02.186] operator surfaces; on integration the orchestrator MUST renumber this section to the next free top-level anchor in Module 10 (e.g. 10.16) and update FEATURE_REGISTRY `spec_anchor` accordingly. The normative content below is anchor-stable and does not depend on the exact integer.

### 10.17.0 Normative language

The keywords MUST, MUST NOT, SHOULD, SHOULD NOT, MAY are to be interpreted as in RFC 2119.

### 10.17.1 Scope and boundaries (LAW)

- SET-SCOPE-001 (In scope). The Settings and Preferences domain MUST cover only general product preferences in these namespaces:
  - `data-roots`: declared logical roots for product data (artifacts, downloads, exports, scratch) expressed as portable identifiers, never as resolved absolute paths.
  - `view-defaults`: per-surface default view state (density, default filters, default sort, default time window, theme).
  - `retention`: default retention policy selections that map to PRIM-RetentionPolicy for governed evidence/artifact classes.
  - `feature-toggles`: enable/disable flags for product capabilities that are gated and safe to toggle at the product layer.
- SET-SCOPE-002 (Out of scope). This domain MUST NOT store, mirror, or override:
  - LLM/provider/model configuration, API keys, endpoints, routing, or inference parameters (owned by the LLM-config surfaces in Section 10.13 and FEAT-AI-JOB-MODEL / FEAT-MODEL-SESSION-ORCHESTRATION).
  - Kernel runtime configuration, write-box action catalogs, or kernel execution policy (owned by FEAT-KERNEL-WORKSPACE-WRITE-BOX and the kernel control plane).
  - Capability/consent grants (owned by FEAT-CAPABILITIES-CONSENT); feature-toggles MAY reference a PRIM-CapabilityKind but MUST NOT grant, expand, or bypass a capability.
- SET-SCOPE-003 (No secret authority). Preference records MUST NOT be a store for secrets, cookies, tokens, or credentials. If a preference references a credential it MUST do so by opaque reference id only; the secret value lives in the kernel secret store and never in a preference record, event, receipt, or projection.

### 10.17.2 Storage authority (LAW)

- SET-STORE-001 (Canonical storage). The authoritative preference store MUST live in PostgreSQL (TECH-POSTGRESQL). Every mutation MUST emit an EventLedger entry; durable change evidence MUST be Flight-Recorder-backed (PRIM-FlightRecorder / PRIM-FlightEvent). Any externalized snapshot/export MUST materialize through the ArtifactStore (PRIM-ArtifactService) with a PRIM-ArtifactManifest.
- SET-STORE-002 (No SQLite). No part of the Settings and Preferences domain MAY use SQLite in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, or exports. The preference store, its projections, and its export artifacts MUST be backend-portable per FEAT-STORAGE-PORTABILITY without introducing an embedded-file authority.
- SET-STORE-003 (No product output in governance). Preference snapshots, exports, and redacted projections are product artifacts and MUST materialize through the shared ArtifactStore. They MUST NOT be written into `.GOV`. All path-like preference values (e.g. `data-roots`) MUST be portable logical identifiers; drive letters, user-profile paths, and machine-local absolute paths MUST be rejected at validation (see PRIM-NamingPolicy and FEAT-STORAGE-PORTABILITY).

### 10.17.3 PreferenceRecord contract (LAW)

- SET-REC-001 (Record shape). The domain MUST define a typed PreferenceRecord (PRIM-PreferenceRecord) with at least:
  - `preference_id`: stable id, namespaced as `<namespace>.<key>` (e.g. `view-defaults.problems.density`), kebab/dotted, portable, never positional.
  - `namespace`: one of the SET-SCOPE-001 namespaces.
  - `value_type`: declared type drawn from PRIM-PreferenceValueType (`bool`, `int`, `float`, `string`, `enum`, `duration`, `portable-path-ref`, `capability-ref`, `json-object`).
  - `value`: the typed current value, schema-valid for `value_type` and the record's PRIM-PreferenceSchemaEntry.
  - `scope`: PRIM-PreferenceScope, one of `global`, `workspace` (carries `wsid`), `surface` (carries surface id). Resolution order is surface over workspace over global over default.
  - `default_value`: the spec/registry default; present for every defined preference.
  - `source`: provenance of the current value (`default`, `operator`, `import`, `migration`).
  - `updated_at`, `updated_by` (PRIM-WriteActor), `revision`: monotonically increasing per `preference_id`+`scope`.
  - `redaction_class`: PRIM-RedactionMode selector controlling whether the value appears in the redacted projection (SET-PROJ-001).
- SET-REC-002 (Typed and validated). Every PreferenceRecord MUST be validated against a registry-defined PRIM-PreferenceSchemaEntry before commit. Validation MUST enforce `value_type`, enum domains, numeric bounds, duration bounds, portable-path-ref portability (SET-STORE-003), and capability-ref existence. Validation failures MUST be rejected as explicit, structured errors, never silently coerced or dropped.
- SET-REC-003 (Defaults are first-class). Every defined `preference_id` MUST have a registry default. A read for an unset preference MUST resolve deterministically to the default via the SET-REC-001 resolution order; the domain MUST NOT return null/undefined for a defined preference.
- SET-REC-004 (Stable ids). `preference_id`, namespace ids, schema-entry ids, and enum domains MUST be stable ids citeable by tools and audits without relying on file position or prose labels.

### 10.17.4 Mutation, events, and receipts (LAW)

- SET-EVT-001 (Change event). Every set/reset/import/migration of a PreferenceRecord MUST emit a PreferenceChangedEvent (PRIM-PreferenceChangedEvent) to the EventLedger with at least: `preference_id`, `scope`, `wsid`/surface (when scoped), `old_value_ref`, `new_value_ref`, `value_type`, `source`, `actor` (PRIM-WriteActor), `revision`, `redaction_class`, `event_ts`. Values that are non-public per `redaction_class` MUST be referenced by hash/opaque id in the event, never inlined.
- SET-EVT-002 (Receipt and recoverability). Every mutation MUST produce a recoverable receipt (PRIM-PreferenceChangeReceipt) carrying the before/after revision and a pointer to the EventLedger entry and the Flight Recorder event, sufficient to replay or revert the change. Resets to default MUST be modeled as a mutation with `source=operator` and an explicit receipt, not as a deletion that loses provenance.
- SET-EVT-003 (Flight Recorder). Every state-changing preference action MUST emit a Flight Recorder event so operator triage (Section 10.5) can correlate behavior changes to preference changes by `preference_id` and `revision`.
- SET-EVT-004 (No hidden runtime calls). The Settings domain MUST NOT itself execute LLM, ComfyUI, media-downloader, ASR, or external-tool work. Any preference whose application requires such work (e.g. re-indexing after a retention change, regenerating proxies after a default change) MUST dispatch it as a governed Workflow-Engine job / AI Job (FEAT-WORKFLOW-ENGINE / FEAT-AI-JOB-MODEL) under the relevant capability gate; the preference change records the request, and the job carries its own EventLedger/receipt evidence. No process-local hidden calls and no localhost authority.

### 10.17.5 Retention and feature-toggle semantics (LAW)

- SET-RET-001 (Retention binding). `retention` namespace preferences MUST select among governed PRIM-RetentionPolicy options for named evidence/artifact classes; they MUST NOT define ad hoc deletion behavior. Applying a retention preference that prunes data MUST run as a governed Workflow-Engine job emitting a PRIM-RetentionReport, so prune evidence survives backend swaps per FEAT-STORAGE-PORTABILITY.
- SET-RET-002 (No silent data loss). A retention preference change that would shorten any retention window MUST surface the prune scope and require explicit operator confirmation before the prune job is dispatched. The confirmation, the resulting PRIM-RetentionReport, and the change receipt MUST be linkable from the same `revision`.
- SET-TOG-001 (Toggle safety). `feature-toggles` MUST only enable/disable product capabilities that are independently gated. A toggle MUST NOT grant a capability, bypass FEAT-CAPABILITIES-CONSENT, or enable a kernel/runtime path. Toggling a feature off MUST quiesce that feature's surfaces without deleting its data.
- SET-TOG-002 (Toggle observability). Each toggle's current value, default, scope, and last `revision` MUST be inspectable; flipping a toggle MUST emit SET-EVT-001 evidence so operators can attribute behavior changes.

### 10.17.6 Redacted projection (LAW)

- SET-PROJ-001 (Redacted projection). The domain MUST expose a redacted projection (PRIM-PreferenceProjection) suitable for debug bundles, support, and no-context operators. The projection MUST default to a redaction mode (PRIM-RedactionMode) that cannot leak secrets, credential reference ids, or values whose `redaction_class` marks them non-public; redactions MUST be recorded via PRIM-RedactionReport.
- SET-PROJ-002 (Faithful and bounded). The projection MUST be a deterministic read-only view over canonical PostgreSQL state showing `preference_id`, `namespace`, `scope`, effective value (redacted as required), `default_value`, `source`, and `revision`. It MUST NOT be a separate authority; on any conflict with canonical state, canonical state wins. When externalized, the projection MUST materialize through the ArtifactStore with a PRIM-ArtifactManifest and MUST NOT be written into `.GOV`.

### 10.17.7 Operator interaction (Normative)

- SET-UI-001. The Settings surface MUST present preferences grouped by namespace, showing effective value, default, scope, source, and last revision per record, with inline validation feedback before commit.
- SET-UI-002. The surface MUST provide reset-to-default per record and per namespace, each modeled as an SET-EVT-001 mutation with a receipt; it MUST NOT offer a destructive "wipe" that loses provenance.
- SET-UI-003. The surface MUST link each preference to its change history (EventLedger/Flight Recorder by `preference_id`) and, for retention preferences, to the most recent PRIM-RetentionReport.
- SET-UI-004 (HBR-QUIET). The Settings surface and any preference-triggered jobs MUST be non-intrusive: no focus stealing, no foreground window pops, no global shortcut hijack, and no unbounded synthetic input. Background application of a preference (e.g. a prune or re-index job) MUST be observable without interrupting the operator.

### 10.17.8 v1 scope and invariants (LAW)

- SET-INV-001. PostgreSQL + EventLedger + ArtifactStore are the only storage authorities for this domain; SQLite is forbidden everywhere per SET-STORE-002.
- SET-INV-002. Every defined preference has a typed schema entry and a default; reads of defined preferences never return null.
- SET-INV-003. Every mutation emits EventLedger + Flight Recorder evidence and a recoverable receipt; no mutation is silent.
- SET-INV-004. This domain stores no secrets and grants no capabilities; LLM-config and kernel-runtime-config remain owned by their respective features.
- SET-INV-005. All preference application that requires execution runs as governed Workflow-Engine/AI Jobs; the domain itself performs no model, tool, or network execution.

**Cross-references:** Section 10.5 Operator Consoles (triage correlation, redaction-safe export); Section 10.13 ModelRuntime / FEAT-AI-JOB-MODEL (LLM config boundary); FEAT-KERNEL-WORKSPACE-WRITE-BOX (kernel runtime config boundary); FEAT-CAPABILITIES-CONSENT (toggle/grant boundary); FEAT-STORAGE-PORTABILITY (PRIM-RetentionPolicy, PRIM-RetentionReport, PRIM-ArtifactManifest); FEAT-WORKFLOW-ENGINE / FEAT-FLIGHT-RECORDER (governed application + evidence); HBR-QUIET (non-intrusive operation).

## 10.18 Stealth Reference Window (Normative) [ADD v02.189]

**Why**
Operators and governed agents frequently need a persistent, glanceable reference surface -- a pinned spec anchor, a transcript excerpt, a job receipt, a screenshot of a never-shown debug window -- that is visible to diagnostics and automation WITHOUT ever competing with the operator's foreground or with another window's focus. [GLOBAL-BUILD-046]..[GLOBAL-BUILD-054] and HBR-QUIET-001..004 (Section 6.6) forbid focus stealing, foreground popping, taskbar presence, and synthetic-input hijack. A naive "always-on-top reference panel" violates every one of those invariants. Section 10.16 [ADD v02.189] defines the Stealth Reference Window: a non-intrusive, never-foreground projection surface whose state is a governed, read-only registry over content references, consumable through IPC and the off-screen capture path (Section 6.6.2) by diagnostics and automation, with zero OS input injection.

**What**
Defines (1) the window-registry state model, (2) the content-reference contract, (3) visibility / quiet flags, (4) the read-only projection + capture path, (5) the IPC contract, (6) EventLedger + Flight-Recorder + ArtifactStore evidence obligations, and (7) the storage / portability / quiet guardrails.

---

### 10.18.1 LAW: non-intrusive by construction (HARD per HBR-QUIET-001..004)

The Stealth Reference Window is a `tauri` window (TECH-TAURI) opened ONLY with the Section 6.6.1 quiet-mode config (`visible(false)`, `focus(false)`, `focusable(false)`, `skip_taskbar(true)`, `always_on_bottom(true)`, `decorations(false)`). The window MUST NEVER be shown as foreground, MUST NEVER call any forbidden API from Section 6.6.6 (`set_focus`, `show`, `unminimize`, `AllowSetForegroundWindow`, `AttachThreadInput`, `SetForegroundWindow`), and MUST NEVER inject synthetic input. The surface is consumed by reading state over IPC (Section 10.16.5) and by capturing pixels over the HWND-independent CDP path (`Page.captureScreenshot`, Section 6.6.2) -- not by bringing a window to the front. Any code path that surfaces the Stealth Reference Window to foreground is a HBR-QUIET-001 violation and fails the focus-audit assertion of Section 6.6.4. The only escape hatch is the per-packet foreground exception of Section 6.6.7 (`packet.requires_foreground = true`), which is opt-in and audited; it is NOT the default.

### 10.18.2 Window-registry state model (Normative)

The Stealth Reference Window state is a governed registry, not ad-hoc window handles. The registry is held in the kernel and persisted in PostgreSQL (TECH-POSTGRESQL); concurrent edits to a registry entry's annotation/layout fields go through the CRDT write-box (TECH-CRDT). SQLite is forbidden for this state in every path (runtime, tests, fixtures, mocks, examples, fallbacks, cache, adapters, harnesses, imports, exports).

`StealthReferenceWindow` (registry entry):

- `window_ref_id` -- stable UUID v7 (per HBR-INT-008) primary key; survives restart.
- `owner_actor` -- the operator or governed-agent actor that created the entry (links to PRIM-Actor; for agent-owned windows, links to the ProcessOwnershipLedger row, Section 5.7 / PRIM-ProcessOwnershipLedger).
- `title` -- ASCII display label (never rendered to foreground; used by diagnostics/automation only).
- `content_refs` -- ordered list of `ContentRef` (Section 10.16.3).
- `layout` -- logical layout descriptor (panel order, scroll offsets); NOT physical screen coordinates, to keep the surface disk- and display-agnostic.
- `visibility` -- `VisibilityFlag` (Section 10.16.4); default `OffScreenOnly`.
- `quiet` -- `QuietFlags` (Section 10.16.4); all quiet invariants default ON.
- `tauri_hwnd_handle` -- runtime-only, never persisted; rebound on each launch under the Section 6.6.1 config.
- `created_at` / `updated_at` -- ISO8601 (PRIM-ISO8601Timestamp).
- `revision` -- monotonically increasing; CRDT-merge-derived for concurrently edited fields.

The registry never holds product content inline; it holds REFERENCES only (Section 10.16.3). This keeps the registry small, portable, and free of media/secret payloads.

### 10.18.3 Content-reference contract (Normative)

`ContentRef` is a typed pointer to canonical state already materialized elsewhere. It MUST resolve through a governed source; it MUST NOT embed raw media, transcripts, or secrets.

`ContentRef` fields:

- `ref_id` -- UUID v7, unique within the window entry.
- `ref_kind` -- one of: `ARTIFACT` (an ArtifactStore manifest entry, PRIM-ArtifactManifest), `SPEC_ANCHOR` (a spec-router section id), `TRANSCRIPT` (an ASR transcript artifact under FEAT-ASR), `JOB_RECEIPT` (a Workflow-Engine / AI-Job receipt), `LEDGER_EVENT` (an EventLedger event id), `SCREENSHOT` (an off-screen capture artifact from Section 6.6.2), `DIAGNOSTIC` (a PRIM-DiagnosticSurface row).
- `resolver` -- the canonical locator: an ArtifactStore manifest id (PRIM-ArtifactService) for `ARTIFACT`/`TRANSCRIPT`/`SCREENSHOT`, a section id for `SPEC_ANCHOR`, an EventLedger id for `LEDGER_EVENT`/`JOB_RECEIPT`, a diagnostic id for `DIAGNOSTIC`.
- `content_sha256` -- SHA256 (TECH-SHA256) of the resolved payload at pin time; lets diagnostics detect drift between the pinned reference and current canonical state.
- `pinned_at` -- ISO8601 pin timestamp.
- `redaction_state` -- asserts that the resolved view is already scrubbed; secrets, cookies, and tokens MUST NOT appear in any `ContentRef` payload, receipt, or log (per the external-tool execution guardrail).

LAW: a `ContentRef` whose `resolver` does not resolve to a governed source (ArtifactStore / EventLedger / spec-router / DiagnosticSurface / ASR transcript artifact) is invalid and rejected at registry write time. There is no "localhost authority" or process-local hidden source for reference content.

### 10.18.4 Visibility and quiet flags (Normative)

`VisibilityFlag` (enum): `OffScreenOnly` (default -- never composited to a visible surface; only IPC + CDP capture), `DiagnosticEmbed` (rendered inside an already-visible diagnostics/operator surface, e.g. Section 10.5 / Section 10.12, which the operator opened deliberately), `ForegroundExceptionBound` (only reachable under the Section 6.6.7 per-packet foreground exception; bounded by timeout + auto-dismiss).

`QuietFlags` (record, all default `true`): `no_focus_steal`, `no_foreground`, `no_taskbar`, `no_global_shortcut`, `no_synthetic_input`. The kernel MUST refuse to open or re-show a Stealth Reference Window whose `QuietFlags` are not all `true`, except under an explicit, audited `ForegroundExceptionBound` packet. Inverting any quiet flag outside that exception is a HBR-QUIET violation surfaced to the focus-audit ledger (Section 6.6.4) and fails the run.

### 10.18.5 Read-only projection + capture path (Normative)

The Stealth Reference Window exposes a READ-ONLY projection of its registry state. Mutations (create entry, add/remove `ContentRef`, reorder layout, repin) are the only write operations and each is a governed mutation (Section 10.16.6); rendering and consumption are read-only.

Consumption paths:

- **IPC read** -- diagnostics and automation read the registry + resolved reference views via the Tauri commands in this section. No GUI focus is required (Section 6.6.3 automation-first design).
- **Off-screen capture** -- a pixel snapshot of the never-shown window is taken via `WebView2.CallDevToolsProtocolMethodAsync("Page.captureScreenshot", "{}")` (Section 6.6.2), HWND-independent, materialized as a `SCREENSHOT` artifact in ArtifactStore. `BitBlt` / `PrintWindow` on a foreground-required HWND is forbidden.

IPC contract (Tauri `#[tauri::command]`, per Section 6.6.3 every action is invoke-driven):

- `kernel.stealth_ref.list_windows()` -- returns the registry rows visible to the calling actor.
- `kernel.stealth_ref.get_window(window_ref_id)` -- returns one entry with resolved `ContentRef` view metadata (not raw payload).
- `kernel.stealth_ref.resolve_ref(window_ref_id, ref_id)` -- returns the governed, redacted resolved view for a single reference.
- `kernel.stealth_ref.capture(window_ref_id)` -- triggers an off-screen CDP capture; returns the ArtifactStore manifest id of the resulting screenshot artifact.
- `kernel.stealth_ref.upsert_window(...)` / `kernel.stealth_ref.add_ref(...)` / `kernel.stealth_ref.remove_ref(...)` / `kernel.stealth_ref.reorder(...)` -- the governed mutations.

### 10.18.6 Evidence obligations (Normative)

Every mutation and every capture emits a recoverable evidence trail:

- **EventLedger** -- each mutation (`STEALTH_REF_WINDOW_CREATED`, `STEALTH_REF_ADDED`, `STEALTH_REF_REMOVED`, `STEALTH_REF_REORDERED`, `STEALTH_REF_CAPTURED`, `STEALTH_REF_WINDOW_CLOSED`) appends an EventLedger event carrying `window_ref_id`, `owner_actor`, the affected `ref_id`(s), and the resulting `revision`.
- **Flight Recorder** -- each mutation/capture emits a Flight-Recorder entry (PRIM-FlightRecorder / PRIM-FlightRecorderEntry) so a no-context model can reconstruct what reference state existed at any run point.
- **ArtifactStore receipt** -- captures and any exportable projection materialize through ArtifactStore (PRIM-ArtifactService) with a manifest (PRIM-ArtifactManifest); the manifest id is the recoverable receipt. Capture jobs that invoke the WebView2/CDP path run as governed Workflow-Engine jobs with capability gates -- no process-local hidden calls.

LAW: a mutation or capture that does not produce an EventLedger event, a Flight-Recorder entry, and (for materialized output) an ArtifactStore manifest is incomplete and MUST be treated as failed/rolled back.

### 10.18.7 Storage, portability, and quiet guardrails (Normative)

- **Storage authority** -- registry state lives in PostgreSQL (TECH-POSTGRESQL); concurrent annotation/layout edits flow through the CRDT write-box (TECH-CRDT); event history in EventLedger; materialized captures/exports in ArtifactStore. SQLite (TECH-SQLITE) is FORBIDDEN in every path for this feature; no SQLite cache, fallback, fixture, mock, or compatibility adapter.
- **No product output in .GOV** -- screenshot artifacts, exports, and any materialized reference output write only through ArtifactStore; never into `.GOV`.
- **Portable paths** -- `resolver` locators are governed ids (manifest ids, section ids, ledger ids), never drive-letter / user-profile / machine-local filesystem paths; `layout` is logical, not physical screen coordinates, so the registry is disk- and display-agnostic ([GLOBAL-PORTABILITY-004]).
- **Secret hygiene** -- secrets, cookies, and tokens are scrubbed from every `ContentRef` view, EventLedger event, Flight-Recorder entry, and capture artifact; `redaction_state` asserts this at pin time.
- **Quiet enforcement** -- the focus-audit subsystem (Section 6.6.4) MUST observe zero foreground transitions resolving to a Stealth Reference Window; the keyboard-injection negative test (Section 6.6.5) MUST observe zero command execution and zero state mutation from injected input against a live Stealth Reference Window.

**Cross-references:** Section 6.6 Non-Hijacking GUI Interaction Invariants (HBR-QUIET-001..004; quiet-mode window config; off-screen CDP capture; automation-first; focus audit; foreground exception); Section 10.5 Operator Consoles + Section 10.12 Diagnostics Panel (`DiagnosticEmbed` host surfaces); Section 5.7 ProcessOwnershipLedger (agent-owned window attribution); FEAT-ASR (transcript reference kind); FEAT-FLIGHT-RECORDER + FEAT-DIAGNOSTICS-SCHEMA (evidence consumers); EventLedger / ArtifactStore / Workflow-Engine (FEAT-WORKFLOW-ENGINE) as governed authority; HBR-INT-008 UUID v7; [GLOBAL-BUILD-046]..[GLOBAL-BUILD-054]; [GLOBAL-PORTABILITY-004].

## 10.19 Command Corpus and Action Catalog Parity (Normative) [ADD v02.189]

**Why**
Handshake exposes a large operator/model command surface -- preload-exposed renderer commands plus backend IPC handlers (Tauri `#[tauri::command]` channels) -- numbering 100-plus distinct entries. Section 10.11.5.28 [ADD v02.185] already requires the Dev Command Center to project `KernelActionCatalogV1` rows, and Section 10.15 [ADD v02.186] requires every wired surface to carry ModelManual coverage. What is missing is a single normative parity contract that (a) enumerates the FULL command/handler corpus into one typed action-catalog schema, (b) cross-checks every catalog entry against the no-context ModelManual for coverage, and (c) makes a missing product anchor an explicit, durable BLOCKED marker rather than a silent gap. Section 10.14 [ADD v02.189] formalizes that contract at spec altitude. This is a backend-authority + projection contract; it defines records, IDs, events, and gates, not product code.

**What**
Defines the canonical command-corpus source, the per-entry action-catalog descriptor parity record, the ModelManual parity cross-check, the BLOCKED-anchor record, the receipt/event evidence shape, and the CI parity gate. The corpus source-of-truth is the preload command registry plus the backend IPC handler set; the catalog is the projection of that corpus into descriptor records.

> NOTE (numbering): the heading is fixed verbatim by the authoring contract as "10.14". Module 10 already carries a "10.14 Inference Lab UI [ADD v02.186]" and ends at 10.15. This section is authored under the mandated label and MUST be renumbered (recommended 10.16) by the editor merging the bundle so the module section index stays collision-free; the normative content is renumber-stable and carries no positional dependencies.

---

### 10.19.1 LAW: Command corpus is a single typed catalog (Normative)

LAW-CORPUS-PARITY-001. There MUST be exactly one canonical command-corpus catalog for the product. It is the `KernelActionCatalogV1` (Section 10.11.5.28, registry `PRIM-KernelActionCatalogV1`) extended to enumerate EVERY preload-exposed renderer command and EVERY backend IPC handler that an operator or model can invoke. There MUST NOT be a second competing command list in prose, in a sidecar `.md`, or in a UI-local table that claims authority.

LAW-CORPUS-PARITY-002. The corpus source-of-truth is mechanical: (a) the preload command registry (the names the renderer is allowed to call) and (b) the backend IPC handler set (the `#[tauri::command]` channels and kernel action handlers). A command name that exists in either source but not in the catalog is a parity defect (Section 10.14.6). A catalog entry that resolves to no source symbol is a parity defect.

LAW-CORPUS-PARITY-003. The catalog MUST be machine-readable backend authority. Operator-facing views (Dev Command Center action-catalog viewer, Diagnostics Manual tab) are projections over it per Section 10.5 [ADD v02.184] and Section 10.11.5.28; they MUST NOT become the parity authority.

LAW-CORPUS-PARITY-004. Every catalog entry that executes LLM, ComfyUI, media-download, ASR, or any external-tool work MUST declare that execution routes through a governed Workflow-Engine job / AI Job with capability gates (Sections referencing `PRIM-WorkflowRun`, `PRIM-CapabilityProfile`). No catalog entry MAY describe a process-local hidden call or treat a localhost endpoint as authority. A catalog entry whose handler performs ungoverned external execution is a parity defect and MUST be marked BLOCKED until governed.

---

### 10.19.2 Normative: Action-catalog parity descriptor

Each enumerated command projects into one parity descriptor, an extension of `PRIM-KernelActionDescriptorV1` (`PRIM-CommandCorpusEntryV1`). Each descriptor MUST carry the following fields at spec altitude:

- `action_id` -- stable kebab/dotted action id (e.g., `media.download`, `asr.transcribe`, `version.commit`); MUST resolve to a preload command name and/or an IPC handler symbol.
- `corpus_source` -- enum `{preload, ipc_handler, both}`; which mechanical source(s) the command was discovered in.
- `owner` -- owning subsystem/module id (e.g., `engine.context`, `engine.version`, `engine.sandbox`, `asr`, `media_downloader`, `model_runtime`); MUST be a real backend owner, never "ui".
- `actor_eligibility` -- which actor classes MAY invoke (operator, model/coder, mechanical orchestrator); drives projection visibility per Section 10.11.5.28.
- `params` -- typed input schema reference + `input_schema_version`; no free-form blob.
- `capabilities` -- required capability set (`PRIM-CapabilityKind` / `PRIM-CapabilityProfile`); empty set MUST be explicit, not absent.
- `execution_class` -- enum `{pure_projection, write_box, workflow_job, ai_job}`. `pure_projection` is read-only; `write_box` routes through `PRIM-WriteBoxV1` and promotion; `workflow_job`/`ai_job` route through Workflow-Engine.
- `receipt_shape` -- the typed receipt/output schema id the command emits on success (e.g., promotion receipt `PRIM-WriteBoxPromotionReceiptV1`, transcript link `PRIM-TranscriptionLink`, artifact manifest `PRIM-ArtifactManifest`).
- `errors` -- enumerated error variants + recovery instruction per variant; a denied or failed invocation MUST yield a typed error, never an untyped throw.
- `foreground_flag` -- boolean; `true` ONLY if the command unavoidably requires foreground interaction. Per HBR-QUIET, `foreground_flag=true` MUST be surfaced before execution and MUST NOT be set for routine model-driven invocation; unbounded synthetic input is forbidden.
- `manual_anchor` -- the ModelManual command id (Section 10.15) this entry maps to, or the sentinel `BLOCKED` (Section 10.14.4).
- `evidence_class` -- the EventLedger event type(s) and Flight Recorder span(s) the command emits (Section 10.14.5).

Descriptor fields MUST be storage-portable: no drive-letter, user-profile, or machine-local path may appear in any field default or example; artifact references use `PRIM-ArtifactManifest` handles, not raw filesystem paths.

---

### 10.19.3 Normative: ModelManual coverage cross-check

PARITY-MANUAL-001. For every catalog descriptor, exactly one of the following MUST hold: (a) `manual_anchor` resolves to a live ModelManual command entry (Section 10.15.1 `PRIM-ModelManual`) whose purpose / inputs / outputs / safety / failure / recovery fields are populated; or (b) `manual_anchor = BLOCKED` with a recorded BLOCKED record (Section 10.14.4).

PARITY-MANUAL-002. A ModelManual command entry that names an `action_id` absent from the catalog is a coverage defect: the manual MUST NOT advertise a command the corpus does not expose. This is the inverse of HBR-MAN-003 (Section 10.15.5) and is enforced alongside it.

PARITY-MANUAL-003. Coverage parity MUST satisfy the no-context-model bar of [GLOBAL-BUILD-002]..[GLOBAL-BUILD-011]: a model with no conversation history MUST be able to discover every invocable command, its inputs/outputs, its capability/foreground posture, and its failure/recovery path, from the manual projection alone. Any catalog entry not so describable fails coverage.

PARITY-MANUAL-004. The cross-check is deterministic and mechanical (a scan, not a judgement). It produces a parity report record (`PRIM-CommandCorpusParityReportV1`) listing: total corpus size, covered count, BLOCKED count, orphaned-manual count, and per-defect rows. The report is a build artifact materialized through the ArtifactStore (`PRIM-ArtifactManifest`); it is never written into `.GOV` and never treated as authority.

---

### 10.19.4 Normative: BLOCKED anchor records

BLOCKED-001. A command MAY exist in the corpus with no valid product anchor (e.g., a handler that currently performs ungoverned external execution, or a command with no ModelManual coverage and no governed execution path). Such a command MUST carry a durable BLOCKED record (`PRIM-CommandCorpusBlockedRecordV1`), not be silently omitted.

BLOCKED-002. A BLOCKED record MUST carry: `action_id`, `blocked_reason` (enum incl. `no_manual_anchor`, `ungoverned_execution`, `no_capability_gate`, `foreground_unbounded`, `no_typed_receipt`, `no_event_evidence`), `discovered_in` (corpus source), `recovery_instruction`, and `first_seen` / `last_seen` timestamps.

BLOCKED-003. A BLOCKED entry MUST remain visible in the Dev Command Center action-catalog projection (Section 10.11.5.28) and MUST NOT be cleared by a denial toast alone; it persists until the underlying anchor is supplied or the command is removed from the corpus. This mirrors the denial-visibility rule of Section 10.11.5.28.

BLOCKED-004. BLOCKED is a parity state, not an execution grant. A BLOCKED command MUST NOT be made invocable by operators or models until its block is cleared; attempting it yields the typed `errors` variant for its block reason plus the recovery instruction.

---

### 10.19.5 Normative: Evidence, receipts, and governed execution

EVIDENCE-001. Every catalog command whose `execution_class` is `write_box`, `workflow_job`, or `ai_job` MUST, on invocation, emit (a) an EventLedger event of the type declared in `evidence_class`, (b) a Flight Recorder span (`PRIM-FlightRecorder` / `PRIM-FlightRecorderEntry`) linkable by `action_id` and run id, and (c) a recoverable typed receipt of the declared `receipt_shape`. Postgres EventLedger remains the replay/promotion authority per Section 10.5 P-06.

EVIDENCE-002. All command output artifacts (media, sidecars, transcripts, downloads, exports) MUST materialize through the shared ArtifactStore with an `PRIM-ArtifactManifest`; no command MAY write product output into `.GOV`, and all artifact references MUST be portable handles. ASR transcripts route through FEAT-ASR contracts (`PRIM-TranscriptionLink`, `PRIM-MediaSource`).

EVIDENCE-003. Receipts, EventLedger payloads, and Flight Recorder spans MUST be redaction-safe by default per Section 10.5 P-04: secrets, cookies, tokens, and credentials MUST be scrubbed from every catalog command receipt and log. A command whose receipt can leak a secret in typical usage is a parity defect and MUST be marked BLOCKED with reason `no_typed_receipt` until the receipt is redaction-safe.

EVIDENCE-004. Storage authority for all of the above is PostgreSQL + EventLedger + ArtifactStore + CRDT/write-box only. No catalog descriptor, receipt, parity report, or BLOCKED record MAY be stored in, fall back to, or cache through any embedded local relational store; such a path is a hard parity defect.

---

### 10.19.6 Normative: Parity CI gate (HBR-CORPUS)

GATE-001. A CI parity gate (HBR-CORPUS-001) MUST run on `just gov-check`. It (a) enumerates the corpus from the preload registry + IPC handler set, (b) diffs the corpus against the catalog, (c) runs the ModelManual cross-check (Section 10.14.3), and (d) validates per-descriptor invariants (typed params, capabilities present-or-explicitly-empty, governed execution for external work, foreground discipline, typed receipt, event evidence).

GATE-002. The gate FAILS the build on any of: a corpus command missing from the catalog; a catalog entry resolving to no source symbol; a descriptor missing a required field; an external-execution command without a Workflow-Engine/AI-Job route or capability gate; a `foreground_flag` set for routine model invocation; a ModelManual command naming a non-existent `action_id`; or a command lacking both a valid `manual_anchor` and a BLOCKED record.

GATE-003. A WP that adds, renames, or removes a preload command or IPC handler MUST update the catalog, the ModelManual coverage, and (if applicable) the BLOCKED records in the same commit, consistent with the same-commit discipline of Section 10.15.7 (HBR-MAN-001). Corpus/catalog drift is a project-quality defect.

GATE-004. Visual debugging and acceptance proof MUST expose stable element identifiers for action-catalog rows, BLOCKED rows, parity-report rows, and coverage badges, consistent with Section 10.11.5.28; HBR-QUIET applies to all parity tooling (no focus stealing, no foreground windows, no global-shortcut hijack, bounded observable runs).

**Cross-references:** Section 10.5 Operator Consoles (P-04 redaction, P-06 kernel authority linkability); Section 10.11.5.28 Kernel Action Catalog and Write Box Projections [ADD v02.185]; Section 10.15 ModelManual Surface [ADD v02.186] (HBR-MAN-001/002/003, [GLOBAL-BUILD-002]..[GLOBAL-BUILD-011]); FEAT-ASR (transcript artifacts); FEAT-KERNEL-WORKSPACE-WRITE-BOX (`PRIM-KernelActionCatalogV1`, `PRIM-KernelActionDescriptorV1`, `PRIM-WriteBoxV1`, `PRIM-WriteBoxPromotionReceiptV1`); HBR-QUIET (non-intrusive operation); WP-KERNEL-004 / WP-KERNEL-009 governance lineage.

---

## 10.20 Project Knowledge Index and Rich Editor Surface [ADD v02.192]

**Why**
WP-KERNEL-009 needs one operator/model surface where project knowledge, Loom navigation, Obsidian-like backlinks, rich documents, embedded code, retrieval traces, claims, and UserManual recovery can be inspected without leaving Handshake. The surface must be a projection over product authority, not a second store.

**What**
The Project Knowledge Index surface exposes:

- a project/root selector backed by KnowledgeSource records;
- source/span/entity/edge/claim inspectors with stable ids and evidence links;
- Loom-style all/unlinked/sorted/pinned/backlink graph views over canonical records;
- rich document editing through the Section 7.1.1.8 editor stack;
- Monaco code-node editing with diagnostics and round-trip status;
- retrieval-mode and RetrievalTrace views showing direct load, exact lookup, graph traversal, hybrid RAG, and skip reasons;
- UserManual lookup and recovery panes for no-context model operation;
- visual-debug snapshots and backend navigation receipts for every model/operator action.

The surface MUST NOT persist its own authority state. UI filters, open panes, expanded graph groups, selection state, and screenshots are projections. The authority source is Section 2.3.13.11 plus EventLedger, CRDT/write-box records, ArtifactStore handles, and UserManualRecord entries.

Model-facing backend navigation MUST be available for all core actions: register source, open source, cite span, inspect entity, inspect edge, inspect claim, traverse graph, open Loom block, edit rich document draft, propose AI edit, inspect retrieval trace, rebuild projection, and open UserManual entry. Each action MUST return a typed receipt with actor/session/correlation ids so parallel models can coordinate without screen scraping.

Visual verification MUST cover desktop and constrained responsive layouts for: empty index, indexing in progress, conflict/error state, rich document with code node, media/PDF embed, graph traversal, retrieval trace, UserManual result, and denied promotion. Text overlap, hidden critical state, unreachable controls, or missing stable element identifiers are verification failures.
