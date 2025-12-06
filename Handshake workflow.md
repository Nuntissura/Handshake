Below is a concrete “ideal workflow” for building Handshake with:

* Master Spec v02.12 as LAW_2
* Codex v0.6 as LAW_1 (v0.5 + a few things promoted back to MUST: strong UNKNOWN_SPEC, NO_SCOPE_SWAP, strict patch semantics for specs, etc.)
* v0.4 used as an explicit “strict overlay” when needed.

I’ll break it into:

1. Project-level phases
2. Day-to-day feature/work-packet workflow
3. Spec/governance workflow
4. When to flip into strict mode

---

## 1. Project-level: drive everything from spec + roadmap

1. Pick the current phase from §7.6 of the Master Spec (Phase 0, Phase 1, …).

2. For that phase, identify:

   * The **vertical slice** that proves the phase: e.g.
     “Open doc → edit → ask AI → see changes + logs in Flight Recorder.”
   * The **MUST-have infra** for that slice (per §2 architecture + §3/4 infra).
   * The **acceptance criteria** in §7.6 for “phase done”.

3. Create a small L1 design doc for the active slice in `/docs_local/`:

   * Name it `L1_<phase>_<slice>.md`.
   * At the top, explicitly reference the Master Spec sections it depends on:

     * e.g. `Ref: §2.3 Editor Architecture, §4.1 LLM Infrastructure, §5.x Observability`.
   * List:

     * “Why this slice exists” (risk it reduces).
     * “What must be visible in UI/logs when done.”
     * Out-of-scope items for this slice.

4. Use that L1 doc + the Master Spec as the binding LAW for all work packets in this phase.

   * Codex v0.6 PRECEDENCE_PRODUCT / PRECEDENCE_IMPL decides how to resolve conflicts:

     * Product/behaviour: Master Spec wins.
     * Layout/assistant behaviour: Codex wins.

---

## 2. Day-to-day feature / work-packet workflow

Think in **small work packets**, each aimed at moving the phase vertical slice closer to “Done”.

### 2.1 Intake & scoping (per task)

For each task you start:

1. In your bootloader / diary entry, define a clear “work packet”:

   * Title: `WP-<phase>-<short-name>`
   * Scope: 3–7 bullet points of what this packet will change.
   * Links: which spec sections + which L1 doc(s) govern it.

2. When you start an AI session for that packet, codex v0.6 SHOULD enforce:

   * Restate task + scope in its own words.
   * Declare:

     * Which LAW docs are in play (`Codex v0.6`, `Master Spec v02.12` + specific sections, relevant L1 doc).
     * Any UNKNOWN_SPEC areas (strong UNKNOWN_SPEC MUST behaviour).
   * Propose a short plan of 3–5 steps, each small and testable:

     * e.g. “Step 1: define backend module stub; Step 2: wire to IPC; Step 3: add tests; Step 4: basic logging/observability surface.”

3. NO_SCOPE_SWAP is treated as MUST in v0.6:

   * If new work appears, it MUST either:

     * Be rejected as out of scope, or
     * Be added explicitly as a new work packet.

### 2.2 Design pass (quick, but explicit)

Before writing code:

1. Map the change into the architecture:

   * Which **AI Jobs** are involved (per AI Job Model)?
   * Which **workflow(s)** (per Workflow & Automation Engine)?
   * Which **data surfaces** (Raw / Derived / Display)?
   * Which **observability surfaces** (logs, Flight Recorder, debug UI) must reflect this change?

2. Check Codex hard invariants:

   * HARD_RDD: you don’t collapse Raw + Display “just because it’s easy”.
   * HARD_LLM_CLIENT: all LLM calls go through `/src/backend/llm/LLMClient`.
   * HARD_STORAGE_LAYER: only `/src/backend/storage/…` talks to DB/filesystem.
   * HARD_LOGGING: use the shared logging utilities, no random `print()` in production paths.
   * HARD_NO_TOPDIR: no new root dirs unless explicitly decided.

3. Update the L1 doc if the design decision is “architectural”, not just an implementation detail:

   * New Job type? Add a short section.
   * New workflow step? Sketch in the L1 doc, not only in code.

### 2.3 Implementation loop (small slices, patch-first)

For each design step:

1. **Find the governing file(s)**:

   * Code: under `/src/backend`, `/src/frontend`, `/src/shared`, etc.
   * Docs: `/docs_local` or Master Spec (only if you explicitly asked for LAW edits).

2. Apply codex v0.6 patch discipline:

   * Default: PATCH-style edits (show BEFORE/AFTER hunks, comment clearly what changed and why).
   * Full-file rewrite is allowed ONLY when:

     * You explicitly request it (“rewrite this full file”), and
     * The assistant restates that intention.

3. Always integrate testing + observability:

   * For each new Job / endpoint / feature:

     * Add tests under `/tests/` (unit or small integration).
     * Add logging at key points, ensuring events hit the **Flight Recorder** layer defined in the Master Spec.
   * Verify that the logs/events carry:

     * Job ID, workflow ID where applicable.
     * Inputs/outputs or at least hashes/refs.
     * Status transitions (queued → running → success/failure).

4. Keep work packets small:

   * Aim for work packets that can be coded + tested in one focused session:

     * e.g. “Implement base `LLMClient` + one concrete runtime adapter.”
   * When a packet grows, split it:

     * `WP-0-llm-client-base`
     * `WP-0-llm-client-openai-adapter`
     * `WP-0-llm-client-local-runtime-adapter`
       etc.

### 2.4 Verification & DCR (light but mandatory for risk)

At the end of a work packet:

1. Run tests (even if manually at first):

   * Backend test suite relevant to the change.
   * Minimal smoke run of the phase’s vertical slice, if possible.

2. DCR loop with v0.6:

   * Draft: what was implemented.
   * Critique: self-check along these axes:

     * Spec alignment: “Did I obey the Master Spec + L1 doc?”
     * Invariants: “Did I break RDD, shared LLM client, or storage rules?”
     * Observability: “Can I see this behaviour in logs/Flight Recorder?”
     * Risk: “Where is this most likely to fail in real use?”
   * Refine: apply only necessary follow-up changes; anything larger becomes a new work packet.

3. Update the work packet log (micro-logger / diary):

   * Mark `WP-…` as `DONE` or `BLOCKED`.
   * If blocked, record why and which LAW or infra constraint caused the block.

---

## 3. Spec / governance workflow

You want to avoid the failures you already experienced (lost content, drift, random spec edits). Use a strict pattern for spec work.

### 3.1 Layered specs (L0 / L1 / L2)

1. L0: frozen Master Spec file (`Handshake_Master_Spec_v02.12.md`).

   * Only changed when you explicitly decide to bump the canonical version.

2. L1: working spec copies per topic.

   * E.g. `L1_Section-4_LLM-Infrastructure_v0.x.md` in `/docs_local/`.
   * Contains refinements, clarifications, but no content deletion without trace.

3. L2: per-task editing copies.

   * When doing surgery on a specific section: copy L1 to L2, edit there.
   * Once vetted, promote L2 → L1, and later fold into the Master Spec.

Codex v0.6 MUST treat L0 as read-mostly unless you say “we’re doing LAW surgery now”.

### 3.2 Spec edit workflow (when you do need to change LAW)

When you explicitly request spec edits:

1. Switch to **strict overlay** (see section 4 below).
2. Define exactly which section(s) of the Master Spec are in scope.
3. Use PATCH-only editing:

   * No full-file rewrites.
   * Every deletion/addition is in context.
4. After edits:

   * Re-scan the TOC and numbering.
   * Check consistency: job definitions ↔ workflow engine ↔ observability ↔ AI roles.
5. Bump the version number and record a short changelog in the header.

---

## 4. When and how to use strict mode (v0.4 overlay)

Use Codex v0.6 as the default, but explicitly opt into v0.4-style strictness when risk is high.

### 4.1 Switch to strict mode for:

* Editing:

  * Master Spec (L0) or core L1 LAW docs.
  * AI Job Model canonical definitions.
  * Workflow & Automation Engine spec.
  * Governance / logging / audit rules.
* Any task where you say “do not drop content” or “no restructuring errors allowed”.

### 4.2 Behaviour changes in strict mode

When you say “strict mode” or similar:

1. UNKNOWN_SPEC becomes BLOCKER:

   * If a needed spec slice is missing, work stops until you supply it.

2. Spec usage ritual becomes mandatory:

   * The assistant MUST:

     * Identify relevant sections by number.
     * Quote or summarise them before proposing changes.
     * Mark assumptions explicitly.

3. Patch semantics are hard enforced:

   * No full-file rewrites.
   * No implicit deletions.
   * No file state hallucination: the assistant works only from content you’ve actually provided.

4. DCR + self-check is non-optional:

   * For every substantial change, you get a micro-post-mortem.

---

## 5. Minimal operational checklist

You can treat this as a short, repeatable loop:

1. Pick phase + vertical slice from §7.6.
2. Create/update the L1 doc for that slice in `/docs_local/`.
3. Define a small work packet with clear scope and acceptance criteria.
4. Start an AI session:

   * Declare LAW stack (Codex v0.6 + relevant specs).
   * Enforce NO_SCOPE_SWAP, strong UNKNOWN_SPEC.
   * Get a 3–5 step plan.
5. Implement in small steps:

   * Patch-style edits.
   * Tests + logging for every new job/feature.
   * Keep RDD, LLM client, and storage invariants intact.
6. Run tests + light DCR at the end of the packet.
7. Log outcome in micro-logger/diary; create next packet or mark slice closer to “Done”.
8. For spec changes, explicitly switch to strict mode and follow the spec surgery protocol.

If you follow this, Codex v0.6 becomes the “behavioural skeleton” that keeps Handshake development deterministic and recoverable, and the Master Spec stays the product/architecture truth you’re always aligning back to.
