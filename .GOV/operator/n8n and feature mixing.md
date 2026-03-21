### Non-negotiable constraints (so “force multipliers” don’t break Handshake)

These apply to **every feature/primitive/mechanical tool** and must stay true even when you add more n8n-like behavior:

* **Single execution authority:** every AI action is a **workflow definition + workflow run**; there is **no separate executor**. 
* **Commit discipline:** workspace changes only commit on `completed` / `completed_with_issues`; failed/cancelled/poisoned never commit. 
* **Typed node graphs + no arbitrary code:** workflows are typed node graphs; AI emits JSON plans over a known catalog. 
* **Durable execution + validation pipeline:** SQLite state, checkpoint/replay, and 5-stage validation before runs/edits. 
* **Artifact/handle discipline + leakage controls:** e.g., ArtifactHandleOnlyGuard, CloudLeakageGuard, PromptInjectionGuard. 
* **Mechanical tool bus law:** tools output Raw/Derived workspace entities, downstream is tool-agnostic, and everything is logged to Flight Recorder with inputs/outputs by reference. 

That’s the “frame” for every force multiplier below.

---

## What you already copied from n8n (Handshake-native) and how to exploit it harder

Handshake already has the core n8n pattern: **graph workflows of typed nodes (triggers, connectors, control flow, utilities)** with durable executions and strong validation. 

To exploit this across *all* primitives/tools, treat **every primitive as a node I/O type** and make “mixing” a first-class workflow activity:

1. **Every primitive gets 3 node families**

* `read_*` nodes (ID → bounded data refs)
* `write_*` nodes (patch-set, validated, previewable)
* `derive_*` nodes (produce DerivedContent overlays, never mutate RawContent silently)

This matches your “views over the same data” and “no parallel data stores” principles across docs/canvas/tables/charts/decks. 

2. **Every mechanical tool is wrapped the same way**

* A deterministic “mechanical runner” invocation that outputs artifacts/DerivedContent and logs provenance. 

3. **Every cross-surface conversion is just another workflow**
   Docling→Docs blocks→Tables→Charts→Decks is already explicitly envisioned as a single vertical slice under jobs + exports + provenance.  

---

## The extra n8n features I suggested adding (and how they multiply *everything*)

These are the “missing multipliers” because they let you package, reuse, and debug cross-primitive pipelines without inventing new subsystems.

### A) Subworkflows (workflow calls workflow)

**Force multiplier:** turns every good pipeline into a reusable “lego brick.”

* Create a `call_workflow` node with:

  * schema-checked inputs/outputs,
  * capability inheritance (most restrictive wins, consistent with your capability resolution model), 
  * lineage links parent job ↔ child job in Flight Recorder.

Use it everywhere:

* “Ingest asset” subworkflow (Docling/Tika/Unstructured fallback)
* “Index + cache assimilation” subworkflow
* “Chart + deck export” subworkflow
* “Mail thread → doc + tasks” subworkflow
* “Calendar compile day” subworkflow

### B) Error workflows (first-class failure routing)

**Force multiplier:** makes failure productive and consistent across *every* engine.

You already have job states and “poisoned = don’t auto-retry.” 
Add:

* an `on_failure` handler workflow that runs when:

  * node fails,
  * validator blocks,
  * job becomes `awaiting_user`,
  * job becomes `poisoned`.

Handler workflow outputs:

* a Diagnostic/Problem + Evidence links,
* a deterministic Debug Bundle export,
* optionally a “repair plan” workflow draft (still needs approval).

This aligns with Operator Consoles needing Problems/Evidence deep links. 

### C) Templates (import/export, parameterized workflow packs)

**Force multiplier:** makes “good practice” spread instantly across domains.

Because workflows are typed JSON plans , you can ship:

* template packs per domain (mail/calendar/creative/devops/etc),
* deterministic ID remapping,
* credential placeholders (no secret leakage),
* schema-version migration hooks.

### D) Execution inspector + safe retry modes

**Force multiplier:** everything becomes debuggable without reading code.

You already require node event records and durable state. 
Add UI semantics:

* retry “from last good node” vs “full replay,”
* rerun with “same workflow version” vs “current saved version,”
* block retries automatically when `poisoned`. 

### E) Pinned data / fixtures (deterministic dev)

**Force multiplier:** converts any pipeline into a testable artifact factory.

Implement “pin output” at any node:

* pin = store output artifact handles + hashes,
* allow downstream nodes to replay from pinned handles,
* use pinned runs as fixtures for CI determinism.

This directly supports your determinism/validator culture (e.g., strict vs replay requirements). 

### F) Safe parameter mapping (no JS expressions)

**Force multiplier:** makes “mixing” primitives easy without violating “no arbitrary code.”

Implement mapping via:

* bounded selectors + schema transforms,
* explicit “transform nodes” (versioned),
* reject dynamic scripting (keeps the invariant). 

---

# How to “exploit everything together” (systematic composition plan)

Instead of enumerating bespoke pairings, use a small set of universal “composition macros” that apply to **every primitive/tool**.

## 1) Ingest → Normalize → Index → Enrich → Present

Applies to: files, mail, calendar, web cache, assets, papers.

* Ingest (Docling/ASR/Tika/Unstructured) produces Raw/Derived entities. 
* Normalize via Converter/Language engines (Derived overlays only).
* Index via Shadow Workspace.
* Enrich via AI jobs (summaries, entity extraction, descriptors).
* Present in Docs/Canvas/Sheets/Charts/Decks (all are views over same entities). 

**n8n multipliers used:** subworkflow templates (“IngestPack”), error workflow (fallback + diagnostics), pinned fixtures (golden ingestions), inspector.

## 2) Extract → Compute → Visualize → Export

Applies to: PDFs, spreadsheets, logs, mail analytics, calendar analytics.

You already specify Chart+Decks as ID-referencing entities with export as mechanical job and strict validators. 

**n8n multipliers used:** template packs (“Finance PDF → Deck”), subworkflows (“ExportPack”), pinned chart specs (render determinism), inspector (replay exports).

## 3) Detect → Suggest → Gate → Apply

Applies to: calendar writes, mail sending, repo changes, OS actions, hardware/CNC.

Handshake already mandates explicit confirmation for sensitive actions. 
And calendar mutation discipline is non-negotiable. 

**n8n multipliers used:** error workflows (validator blocks become actionable tasks), templates (“SafeSendEmail”, “CalendarPatchProposal”), inspector (diff previews + retries).

## 4) Observe → Diagnose → Bundle → Repair

Applies to: every domain because everything logs to Flight Recorder and Operator Consoles are required. 

**n8n multipliers used:** error trigger workflows, pinned fixtures for repro, subworkflow “DebugBundlePack”.

---

## Mechanical tools & engines: how to multiply them with the above

### Mechanical Tool Bus families (directly in spec)

* Docling ingestion (+ MCP sidecar option) 
* ASR stack (ffmpeg validation, Whisper, optional diarization) 
* Fallback handlers (Unstructured, Apache Tika) 
* “Generic engines” like Converter/Sentiment appear under 6.3.11 

**Force multiplier pattern:** every one of these becomes a reusable subworkflow with:

* standard inputs (EntityRefs),
* standard outputs (artifact handles + DerivedContent refs),
* standard failure routing (Diagnostics + Debug Bundle),
* pinned fixtures per tool/version.

### Mechanical Extension Engines (§6.3) — turn them into “domain packs”

§6.3 explicitly defines engines as deterministic bodies under the Four-Layer Architecture (Brain plans, Gate validates, Mechanical executes, Shadow indexes). 

Engines listed under §6.3 headings (grouped by domain):

* Engineering & Manufacturing: Spatial, Machinist, Physics, Simulation, Hardware
* Creative Studio: Director, Composer, Artist, Publisher, Atelier
* Culinary & Home: Sous Chef, Safety, Homestead
* Organization & Knowledge: Archivist, Librarian, Curator, Analyst, Chronicle
* Data & Infrastructure: Wrangler, DBA, Sync, Indexer, Monitor, Router, Inspector
* Travel & Spatial Intelligence: Navigator, Cartographer, Geo
* Developer Tools & System Context: Profiler, Workspace, Clipboard, Quota, Guard
* OS Primitives & Desktop Integration: Window, Shell, Scheduler, Notifier
* Software Engineering & DevOps: Repo, Build, Test, Deploy, Log, Contract, Formatter, Container, Network, Decompiler
* Language & Linguistics: Polyglot, Red Pen, Lexicographer, Phonetician, Aligner, Detector, Anonymizer, Morphologist, Converter, Sentiment

**How to exploit them together (without bespoke glue):**

* Make each domain a **template pack** of subworkflows:

  * `engine.<name>.run` (single engine node wrapper)
  * `domain.<domain>.pipeline` (multi-engine recipes)
  * `domain.<domain>.on_failure` (repair + diagnostics)
* Cross-domain compositions are then trivial:

  * “Mail attachment → Docling → Table → Chart → Deck → Export” (IngestPack + VisualizePack). 
  * “ASR transcript → transcript_to_deck → export” is already specified. 
  * “Calendar compile day” consumes tasks/docs/mail signals and outputs suggestions; keep writes gated.  

---

## Two global risks when you “multiply everything,” and how to avoid them

1. **Accidentally creating a second orchestrator**

* If n8n (external) or plugins start coordinating runs independently, you break “single execution authority.” 
  Mitigation: any external automation must only trigger **Handshake workflows**, never bypass them.

2. **Context/artifact leakage through automation**

* Automation increases the chance of shoving big blobs into prompts or exporting sensitive content.
  Mitigation: enforce the validator pack (ArtifactHandleOnlyGuard, CloudLeakageGuard, PromptInjectionGuard) on every template/subworkflow. 

---

## The shortest “force multiplier backlog” that unlocks the rest

1. Add **call_workflow** (subworkflow) node + capability inheritance.
2. Add **on_failure routing** (error workflows) that emits Problems/Evidence + Debug Bundle. 
3. Add **template packs** + deterministic import/export.
4. Add **pin outputs** (fixtures) + replay-from-pin.
5. Add **execution inspector** semantics (retry modes + workflow-version choice) bounded by `poisoned` rules. 

If you want, I can turn this into a deterministic spec patch plan: exact section insertions under **§2.6 Workflow Engine**, plus a “template pack manifest” format that maps to your capability model and Flight Recorder requirements.
