---
schema: handshake.indexed_spec.module@1
spec_version: "v02.205"
bundle_id: "master-spec-v02.205"
module_id: "14"
section_id: "14"
title: "14. Studio -- Unified Creative Suite"
source_baseline_version: "v02.199"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "81e296ec522d1058614cf2480b05bfdc0359fa2da85258a2fffc43c3ec756a15"
body_sha256: "5b280ffe9a2d533867e0eee0ee476faa62f88d3d298ce9d22d7a64f71c7dac0d"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 14. Studio -- Unified Creative Suite [STU-SECTION-001]

[ADD v02.199] Section 14 (Studio) is added in Master Spec v02.199 as the normative unified-creative-suite creative module, merging the full deduped Studio feature research corpus into the Master Spec as sole authority (no sidecars). The matching `FEAT-STUDIO-*`, `PRIM-Studio*`, and `IMX-STUDIO-*` appendix rows in Section 12 carry the same `[ADD v02.199]` marker; this Main Body section leads that appendix growth per the spec-growth-discipline rule.

[ADD v02.200] v02.200 hardens Section 14 with the universal command contract [STU-CON-007] (every Studio tool/command/primitive in 14.4-14.15 MUST be model-invokable + parallel-safe + deterministic + visually-verifiable, so parallel model agents can deterministically edit photos/vector-masks/layouts across multiple files at once in conjunction with the visual tools), the no-operator-only-by-default rule [STU-MDL-006], and two scope clarifications ([STU-RAS-051] focus-stack/seamless-blend composite op; [STU-OVR-015] video-footage clip-editing out of scope).

Studio is the Handshake-native creative module that unifies raster imaging, vector illustration, page layout/publishing, typography, color, effects, design systems, prototyping/motion, and whiteboarding into ONE operator-facing application and ONE model-steerable command surface, built as native Rust tools/primitives under the Studio pillar (product-reference pillar #18). It targets full feature parity with Adobe Photoshop (+ Camera Raw), Adobe Illustrator, Adobe InDesign, the Affinity V2/V3 suite (Photo/Designer/Publisher and the v3 unified app), and the Figma family (Design, Draw, FigJam, Slides, Sites, Buzz, Make, Motion, Dev Mode) — rebuilt as Handshake-native primitives, offline, with no external creative-app dependency and no cloud requirement.

This section is product LAW. It attaches to the kernel exactly like the Tailor creative module (Section 13): a `handshake_core::studio` domain module bound to Handshake-managed SurrealDB/EventLedger authority, CRDT collaboration, the sandbox -> validation -> `PromotionGate` lifecycle, and model lanes, plus standalone UI-agnostic Rust engine crates reached only through typed traits. The canonical authority for every Studio type, field, unit, event, schema-id, table, migration, validation check, and promotion-equivalence rule is sub-section 14.23 (Canonical Studio Authority Contracts) as overridden by the v02.204 authority contract below; where any other sub-section conflicts with that override, the override wins.

## 14.0 SurrealDB Authority Override (v02.204) [STU-SDB-001]

The clauses in this sub-section are the canonical Studio storage and data-schema contract. They
override every incompatible physical-storage phrase, code sample, DDL fragment, test instruction,
or migration filename elsewhere in Section 14, including text that previously called 14.23
canonical. Older PostgreSQL, `sqlx`, `PgPool`, `JSONB`, RLS, `pgvector`, `LISTEN`/`NOTIFY`, and
SQL-migration wording is retained only as legacy design provenance and as a field-by-field inventory
that must be translated. It does not authorize a PostgreSQL runtime, fallback, dual-write mode,
proof path, dependency, or second authority.

**[STU-SDB-002] Runtime authority.** Studio durable authority MUST use the Handshake-managed
SurrealDB service through the official SurrealDB Rust SDK and the kernel's typed authority-store
abstraction. `AppState` supplies that shared SurrealDB authority client; Studio and its standalone
engine crates MUST NOT create a private authority database, use `PgPool`, issue `sqlx` queries,
accept a PostgreSQL connection URL, or attempt PostgreSQL connectivity for migration or
verification. PostgreSQL drivers, runtime lifecycle code, environment variables, configuration
keys, startup modes, provisioning scripts, health checks, and proof fixtures MUST be retired from
the active Studio dependency and acceptance surface. SQLite remains prohibited for authority,
caches, fixtures offered as runtime proof, and tests that claim production-storage equivalence.
There is no PostgreSQL fallback, dual authority, import lane, or transitional runtime.

**[STU-SDB-003] Schema translation.** Every canonical `studio_*` relation named in this module
remains the same semantic entity and uses the same stable schema IDs, prefixed string identifiers,
field names, units, status domains, relationship meaning, timestamps, retention rules, and
EventLedger links, but MUST be implemented as a SurrealDB `SCHEMAFULL` table. A legacy SQL column
maps to a typed SurrealDB field; `JSONB` maps to typed object or array fields validated against the
named Handshake schema rather than an opaque JSON string; SQL foreign keys map to typed record
references plus explicit application and schema assertions; SQL `CHECK`, `UNIQUE`, and ordinary
indexes map to equivalent SurrealDB field assertions and indexes. Record IDs MAY use SurrealDB
record identifiers internally, but the specified domain ID value and prefix remain stable at every
API, EventLedger, import/export, and receipt boundary.

**[STU-SDB-004] EventLedger atomicity and concurrency.** The EventLedger remains the semantic
mutation authority and replay source, and its event records, idempotency records, stream heads,
and replay metadata MUST be persisted in SurrealDB. Every accepted Studio mutation MUST execute
its document or component changes, SurrealDB-persisted EventLedger append, idempotency-key claim,
revision/precondition check, and any projection-head update in one SurrealDB transaction. Duplicate
idempotency keys return the prior receipt without a second mutation or event. Concurrent writes
MUST use an expected-revision or equivalent compare-and-set precondition and fail closed on
conflict. Replay from an empty SurrealDB namespace/database MUST reconstruct the same authoritative
document state, ordering, stable IDs, privacy scope, history, and terminal receipt outcomes
required by the existing clauses.

**[STU-SDB-005] Privacy and resource scope.** Every authority table defaults to `PERMISSIONS NONE`
and grants only explicit authenticated record-user `SELECT`, `CREATE`, `UPDATE`, and `DELETE`
expressions for the owning account, project, role, and resource scope. Sensitive fields additionally
use field-level permissions where visibility is narrower than the record. The kernel
`ResourceBroker` remains the required application-layer authorization gate for artifacts, placed
assets, derived previews, model contexts, exports, logs, and live-query subscriptions. Root, system,
namespace, or database-owner credentials bypass record permissions and therefore MUST NOT be used
as privacy, tenant-isolation, or least-privilege proof; those proofs MUST execute with authenticated
record-user sessions and adversarial cross-account/project cases.

**[STU-SDB-006] Queries, subscriptions, and vectors.** Query and repository code MUST use typed
SurrealQL through the official SDK. SurrealDB live queries replace any PostgreSQL
`LISTEN`/`NOTIFY` design while preserving reconnect, resume, ordering, scope, and deduplication
requirements. Where an existing Studio behavior requires semantic vector retrieval, the embedding
remains a typed vector field with a SurrealDB vector index and explicit filtered search; vector
candidates MUST still pass record-user permissions and `ResourceBroker` authorization. Non-vector
indexes retain their specified lookup, uniqueness, ordering, and performance intent.

**[STU-SDB-007] Fresh schema initialization and activation.** Studio MUST initialize its
SurrealDB `SCHEMAFULL` tables, typed fields, assertions, indexes, permissions, EventLedger records,
and projection heads from a clean SurrealDB namespace/database. The initialization ships as a
SurrealKit rollout with explicit `start`, initialize, verify, cold-authority activation,
`complete`, and `rollback` stages. Activation MUST occur only after the empty authority passes the
schema, transaction, EventLedger, privacy, index, and restart checks in [STU-SDB-008]. The dated
`.sql`/`.down.sql` names and fenced PostgreSQL DDL retained below are non-executable semantic
inventories and historical provenance only; they MUST NOT be executed, connected to, parsed as
migration input, or used as a data-import source. No PostgreSQL data migration, reconciliation,
dual read, dual write, compatibility bridge, or authority cutover exists. Rollback returns to an
unactivated or prior SurrealDB schema state and MUST NOT select or contact PostgreSQL.

**[STU-SDB-008] Test and acceptance proof.** Studio storage acceptance MUST provision an isolated
SurrealDB namespace/database, apply the SurrealKit rollout, execute real official-SDK calls, and
prove create/read/update/delete, transaction rollback, idempotent retry, optimistic conflict,
EventLedger append/replay, restart recovery, live-query reconnect, index behavior, retention, and
authenticated record-user privacy. Mock repositories, translated SQL text, root-authenticated
queries, PostgreSQL fixtures, or inspection of a retired PostgreSQL schema cannot substitute for
this runtime proof. Acceptance MUST also prove that active Studio startup, configuration, and tests
contain no PostgreSQL connection attempt or runtime dependency.

**[STU-SDB-009] Legacy-name preservation.** Historical work-packet names, migration identifiers,
source paths, and quoted implementation examples containing PostgreSQL-family terms remain valid
only as provenance locators. When such a locator is cited by new work, the implementation MUST
apply the mappings in [STU-SDB-002] through [STU-SDB-008] and MUST record the SurrealDB replacement
artifact; it MUST NOT recreate the superseded backend.

[STU-SECTION-002] SOLE-AUTHORITY / NO-SIDECARS. This Master Spec section is the single source of truth for Studio. The full deduped feature surface, its normative requirements, and its canonical contracts live here in the spec, not in an external research corpus. The research package `.GOV/reference/studio_app_feature_research/` (5,034+ source-verified feature rows across the five source suites, the 160-group cross-app overlap map, and the integration architecture) is non-normative historical provenance only; it MUST NOT be treated as co-authority, and Studio implementation, validation, and no-context onboarding MUST resolve intent and contracts from this section, not from the corpus. Where the corpus and this section disagree, this section wins and the corpus is stale provenance.

[STU-SECTION-003] DEDUP / NO-DOUBLE-FEATURES. Shared capability across the source suites maps to exactly ONE Studio primitive and ONE Studio command family. The per-suite variants recorded in the research provenance collapse into a single normative Studio feature; a source suite's product name (Photoshop, Illustrator, InDesign, Affinity, Photo, Designer, Publisher, Figma, FigJam, and so on) is never a Studio tool, command, panel, or manual name. Studio ships Handshake-native names. This section's per-domain feature catalogs (14.4-14.15) are the deduped normative Studio feature set; the interaction/force-multiplier edges live in Appendix 12.6 (`IMX-STUDIO-*`).

---

## 14.1 Overview, Scope, and the Unified-Suite Differentiator

---

### 1. What Studio Is

Studio is the Handshake-native unified creative suite. It is a kernel-attached creative module in the same architectural position as the atelier module and the Tailor module: a domain subdirectory under `handshake_core` that receives `AppState` by reference, emits `KernelEventType` variants to the EventLedger, operates through the kernel sandbox and `PromotionGate`, persists authority records to Handshake-managed SurrealDB through the official SurrealDB Rust SDK and the kernel's typed authority-store abstraction, and exposes both a native operator UI (egui/wgpu/AccessKit per the canonical native-Rust shell) and a typed model-steerable command/MCP surface.

Studio consists of the following compile units, shared by every creative domain:

1. **`handshake_core::studio`** (`src/studio/`) — the kernel-bound creative module: authority storage, EventLedger emission, CRDT collaborative editing, sandbox dispatch, promotion-gate integration, model-lane binding, and REST/MCP API surface (`src/api/studio.rs`). Per-domain domain logic lives under `src/studio/<domain>/` (e.g. `src/studio/raster/`, `src/studio/vector/`, `src/studio/layout/`).

2. **`studio-engine`** — a standalone Rust workspace crate (no `handshake_core` dependency) that owns the compute-heavy, UI-agnostic engines: the raster compositor and pixel pipeline, the vector/path geometry and tessellation engine, the layout/reflow engine, the text-shaping/typography engine, the color-management engine, and the GPU render path (wgpu/WGSL). `handshake_core::studio` calls into `studio-engine` only through typed traits (`RasterEngine`, `VectorEngine`, `LayoutEngine`, `TextEngine`, `ColorEngine`, `RenderEngine`). Studio MUST NOT put `wgpu`/WGSL/GPU dependencies into `handshake_core`'s `Cargo.toml`; all GPU code is isolated in `studio-engine`, exactly as Tailor isolates GPU code in `tailor-solver` ([TAI-OVR-009]).

[STU-OVR-001] Studio MUST be a Cargo workspace member set in the Handshake monorepo, not a separately versioned external crate, so the engines and the kernel binding are always tested together (mirrors [TAI-OVR-010]).

[STU-OVR-002] Studio MUST NOT require Adobe Photoshop, Illustrator, InDesign, the Affinity suite, Figma, or any other subscription-gated, account-gated, or platform-locked creative application at runtime. Interoperability with those tools' file formats (PSD, AI, IDML, the Affinity `.afphoto`/`.afdesign`/`.afpub`/`.af` family, `.fig`, SVG, PDF, and the raster/vector interchange set) is in scope as import/export/round-trip (14.13); runtime dependency on those apps is out of scope. Note the field precedent: Affinity's own October-2025 v3 relaunch unified its three apps into one application with Vector/Pixel/Layout studios over a single `.af` document — the same unification Studio targets — but gated that app behind a required online Canva account; Studio's unification is fully local-first with no account and no sign-in.

[STU-OVR-015] [ADD v02.200] Editing imported VIDEO FOOTAGE as a clip-timeline (trimming/sequencing/encoding video clips as raster-video layers, i.e. a non-linear video editor) is OUT of Studio scope. Studio's motion surface (14.11) is keyframe animation + motion export (video/GIF/animated-SVG) and placing/rendering media; footage-clip non-linear editing is a separate Handshake concern (the `engine.director` mechanical engine, Spec §11.8). Studio MAY place and render video media (14.6/14.11) but does not own footage clip-editing. This is recorded as an explicit, intentional scope edge, not an omission.

[STU-OVR-003] Studio MUST NOT introduce a SQLite dependency anywhere (not in `studio-engine`, not in tests, not as a development cache). The `no_sqlite_tripwire` and SurrealDB-only startup tripwire in [STU-SDB-002] apply to Studio without exception. Durable Studio authority is SurrealDB/EventLedger only; live collaborative document state is CRDT; neither is SQLite-backed.

### 2. Scope

Studio's normative feature scope is the deduped union of the five source suites, organized into the domain catalogs below. Each catalog sub-section is the normative Studio feature set for that domain; the research provenance rows are collapsed into these normative features per [STU-SECTION-003].

| Domain catalog | Sub-section | Source-suite parity basis (provenance) |
|---|---|---|
| Raster imaging & photo editing | 14.4 | Photoshop, Affinity Photo, Figma raster fills |
| Vector graphics & illustration | 14.5 | Illustrator, Affinity Designer, Figma/Draw vector networks |
| Page layout & publishing | 14.6 | InDesign, Affinity Publisher |
| Typography engine | 14.7 | all five suites' text engines |
| Color management & pipeline | 14.8 | all five suites' color systems |
| Effects, filters & adjustments | 14.9 | Photoshop filters/adjustments, Illustrator effects, Affinity live filters, Figma effects |
| Design systems, components & variables | 14.10 | Figma components/variants/variables, Illustrator symbols, InDesign styles |
| Prototyping, motion & interaction | 14.11 | Figma prototyping + Motion, InDesign interactive/EPUB |
| Camera Raw / Develop pipeline | 14.12 | Photoshop Camera Raw, Affinity Develop persona |
| Import/export & file-format compatibility | 14.13 | all five suites' format matrices |
| Automation, scripting & plugin/API surface | 14.14 | Photoshop UXP/actions, Illustrator/InDesign scripting DOM, Figma Plugin/REST API |
| Whiteboard & diagramming | 14.15 | FigJam |

The cross-cutting normative requirements — model visibility/steerability (14.16), parallel workflows (14.17), the propose-work system (14.18), per-file history/undo (14.19), the headless/quiet law (14.20), the operator unification surface (14.21), the dual-audience UserManual (14.22), the canonical contracts (14.23), and validation/promotion/HBR (14.24) — apply to every domain catalog.

Out of scope: replicating any source suite's exact menu tree, pixel-perfect UI clone, vendor cloud services, vendor product naming, or a new invented interchange format that replaces the existing creative formats (14.13 defines the compatibility posture instead).

### 3. The Unified-Suite Differentiator

Studio's defining differentiators over the five source suites are (a) **one unified document + primitive model** across raster/vector/layout/design-system domains rather than separate applications, and (b) **model-steerability and swarm-parallel operator/model co-work as a first-class design constraint**, not a plugin. No source suite offers both: the Adobe suite is multi-application with app-specific state; Affinity v3 unifies the app but gates it on an account and has no model-steerable command surface; Figma is model-adjacent (Plugin/REST/Dev-Mode MCP) and multiplayer but cloud-hosted, single-domain-per-file, and not a full raster+layout+publishing suite. Studio owns the unified local-first pipeline natively inside kernel model-lane infrastructure.

---

## 14.2 Architecture: Kernel Binding and Engine Crates

Studio binds to the kernel exactly as Tailor does (§13.11). This sub-section states the normative consequences.

[STU-ARC-001] `handshake_core::studio` MUST follow the atelier/Tailor module pattern: a `src/studio/` directory with `mod.rs` defining `StudioEngineError` and `event_family` constants; per-domain subdirectories (`raster/`, `vector/`, `layout/`, `typography/`, `color/`, `effects/`, `design_system/`, `prototype/`, `raw/`, `interop/`, `automation/`, `whiteboard/`); storage glue in `src/studio/storage_glue.rs`; and Axum routes in `src/api/studio.rs` registered in `api/mod.rs`. All module files receive `AppState` (and thus the shared SurrealDB authority client, `LlmClient`, `SandboxRunner`, `PromotionGate`, and CRDT infrastructure) by reference; no separate initialization is permitted.

[STU-ARC-002] `handshake_core`'s `Cargo.toml` MUST NOT gain `wgpu`, WGSL, or GPU-compute dependencies for Studio. All GPU/compute code is isolated in the `studio-engine` workspace crate and reached through the `RasterEngine`/`VectorEngine`/`LayoutEngine`/`TextEngine`/`ColorEngine`/`RenderEngine: Send + Sync` trait boundaries defined in `studio-engine/src/lib.rs` (mirrors [TAI-OVR-009]).

[STU-ARC-003] EventLedger event variants for Studio MUST follow the canonical addition list in 14.23. The Studio module adds variants to `KernelEventType` in `kernel/mod.rs` (wire format `STUDIO_*` SCREAMING_SNAKE_CASE via `as_str()`) with dot-namespaced lowercase `event_family` constants (`studio.document`, `studio.raster`, `studio.vector`, `studio.layout`, `studio.typography`, `studio.color`, `studio.effect`, `studio.export`, `studio.history`, `studio.proposal`, `studio.design_system`, `studio.prototype`), and registers every variant in `required_first_slice_events()`. Every `WriteContext` carries a `KernelActor` variant so model-authored Studio rows are distinguishable from operator rows in the audit log.

[STU-ARC-004] SurrealDB `SCHEMAFULL` tables for Studio MUST preserve prefixed string domain IDs (e.g. `document_id = "SDOC-{uuid_v7}"`, `layer_id = "SLYR-{uuid_v7}"`, `artboard_id = "SART-{uuid_v7}"`) at every API and receipt boundary. Every Studio authority record MUST carry a required typed `event_ledger_event_id` reference to `kernel_event_ledger`, and every create or mutating update MUST enter the SurrealDB-only authority guard and the atomic transaction required by [STU-SDB-004]. Schema evolution uses SurrealKit under [STU-SDB-007]; legacy dated SQL names are provenance only. The canonical table set is defined in 14.23 as translated by [STU-SDB-003].

[STU-ARC-005] A model-authored Studio mutation (any command batch that changes document authority) MUST NOT be written directly to a SurrealDB authority record. It MUST enter the kernel sandbox (`SandboxAdapter`, process-tier by default), be validated by the `StudioValidationDescriptor` catalog (14.24), and pass the `PromotionGate` (`PromotionDecisionV1: Accepted`) before authority records change. This lifecycle is not optional and MUST NOT be bypassed regardless of model confidence (mirrors [TAI-OVR-004]). See 14.18 for the propose-work system that carries model edits through this lifecycle.

[STU-ARC-006] Studio MUST NOT be activated as a build-target work packet until the Handshake kernel governance baseline (the sandbox, `PromotionGate`, CRDT surfaces, and the native egui/wgpu/AccessKit shell from WP-KERNEL-011/012) is stable enough that the surfaces Studio depends on are not simultaneously under active structural change. Individual `studio-engine` crates (raster compositor, vector geometry, text shaping, color) MAY be prototyped in isolation before the kernel binding is authored, because they have no `handshake_core` dependency.

---

## 14.3 Unified Document Model and Studio Primitive Set

[STU-DOC-001] `StudioDocument` (schema id `hsk.studio.document@1`) is the single unified document type spanning all Studio domains. One document holds a tree of `StudioLayer` nodes over one or more `StudioArtboard`/`StudioPageSpread` containers; a layer's `kind` selects its domain payload (`raster`, `vector`, `text`, `group`, `adjustment`, `live_filter`, `mask`, `fill`, `component_instance`, `frame`, `placed_asset`). There is no per-source-app document silo: a raster edit, a vector edit, and a layout edit operate on layers in the same `StudioDocument` through the same selection, history, color, and export surfaces. `StudioDocument` MUST derive `schemars::JsonSchema` so its MCP `inputSchema` is auto-generated, and it is the single type shared between the model's output surface, the engines' input surface, and the typed SurrealDB object field (`studio_documents.doc_json`).

[STU-DOC-002] The canonical Studio primitive set is normative and deduped (one primitive per shared capability): `StudioDocument`, `StudioArtboard`, `StudioPageSpread`, `StudioLayer`, `StudioLayerGraph`, `StudioSelectionSet`, `StudioMask`, `StudioRasterTile`, `StudioVectorPath`, `StudioVectorNetwork`, `StudioTextStory`, `StudioTypeStyle`, `StudioColorProfile`, `StudioSwatch`, `StudioGradient`, `StudioPattern`, `StudioEffectStack`, `StudioAdjustment`, `StudioLiveFilter`, `StudioBlendMode`, `StudioComponent`, `StudioComponentInstance`, `StudioVariable`, `StudioVariableCollection`, `StudioStyleRegistry`, `StudioLayoutGrid`, `StudioConstraint`, `StudioAutoLayout`, `StudioPrototypeFlow`, `StudioMotionTimeline`, `StudioExportRecipe`, `StudioImportProfile`, `StudioHistoryEntry`, `StudioEditProposal`, `StudioModelAdapter`, `StudioValidationDescriptor`, `StudioRenderHarness`. The full field-level canonical definitions live in 14.23; the per-domain catalogs (14.4-14.15) reference these primitives rather than redefining them.

[STU-DOC-003] Unit law: every model-facing and operator-facing Studio surface (measurements, positions, sizes, type sizes) uses explicit typed units — points for typography, pixels for raster document coordinates, and a document-declared unit (mm/in/px/pt) for layout geometry with the unit carried on every length-bearing field. Color values carry an explicit `StudioColorProfile` reference; there is no implicit device color. The conversion boundary MUST be the API decode step. Mixed-unit fields are forbidden.

[STU-DOC-004] Studio is a shared-primitives product: a capability exposed in one domain (e.g. a mask, a gradient, a text-on-path, an export slice) is the SAME primitive when used from another domain, exposed through the same typed command contract. The operator UI and the model API are two projections of that single primitive (see 14.16, 14.21); there is no separate model shim and no separate per-domain reimplementation of a shared capability.


## 14.4 Raster Imaging & Photo Editing

Studio's raster imaging surface is the deduped union of the Photoshop (and Camera Raw's pixel side), Affinity Photo, and Figma raster-fill feature sets, rebuilt as Handshake-native pixel primitives. It is the pixel-editing domain of the unified `StudioDocument`: raster content lives as `StudioLayer` nodes whose `kind` selects a pixel payload, sharing one selection surface, one masking surface, one color pipeline, one history, and one export surface with every other Studio domain (14.3, [STU-DOC-004]). This sub-section is the normative Studio raster feature set; the per-suite provenance rows in `.GOV/reference/studio_app_feature_research/` collapse into the single features defined here per [STU-SECTION-003], and no source-suite product name is a Studio tool, panel, adjustment, or command name. Every raster capability MUST be exposed as a non-destructive primitive wherever the source suites offer a non-destructive form: destructive-in-place editing is a mode of a primitive, never the only path. Canonical field-level types, schema ids, event variants, tables, and validation checks for every primitive named here (`StudioLayer`, `StudioRasterTile`, `StudioSelectionSet`, `StudioMask`, `StudioAdjustment`, `StudioLiveFilter`, `StudioBlendMode`, `StudioEffectStack`, `StudioGradient`, `StudioPattern`, `StudioColorProfile`) are defined in 14.23; where this sub-section and 14.23 conflict, 14.23 wins.

---

### 1. Cross-Cutting Raster Obligations

[STU-RAS-001] Every raster feature in this sub-section that creates, changes, or removes an operator-visible surface (a layer, a selection, a mask, an adjustment, a filter layer, a stroke, a transform, a channel, or a document color/bit-depth state) MUST expose (a) a native operator GUI control per the Studio shell and model-visibility contract (14.16), (b) a typed, deterministic, model-steerable command with a stable identifier and a `schemars`-generated `inputSchema` (14.16, 14.14), (c) Argus observability — structured state, receipts, and a visual/pixel snapshot path for a no-context model (14.16, 14.20), and (d) a dual-audience UserManual entry (14.22). This obligation is stated once here and is normative for every feature row and clause in 14.4; it MUST NOT be re-stated per feature and MUST NOT be omitted per feature.

[STU-RAS-002] Raster operations MUST obey the headless/quiet law (14.20): brush strokes, filter previews, ML-backed selections, transforms, and batch pixel work MUST run without stealing focus, popping foreground windows, or hijacking input, and MUST be observable through logs, receipts, and snapshots rather than through a visible application window.

[STU-RAS-003] A model-authored raster mutation (any command batch that changes pixel authority — pixel writes, layer-graph edits, mask edits, adjustment/filter parameter changes, channel operations, mode/bit-depth conversions) MUST NOT write a SurrealDB authority record directly. It MUST enter the kernel sandbox, be validated by the `StudioValidationDescriptor` catalog (14.24), and pass the `PromotionGate` (`PromotionDecisionV1: Accepted`) before authority changes, exactly as [STU-ARC-005] requires. This lifecycle is not optional and MUST NOT be bypassed on model confidence.

[STU-RAS-004] Raster pixel data MUST be stored and composited as `StudioRasterTile` tiles (14.23), not as monolithic full-frame buffers, so that large documents, partial edits, undo/redo (14.19), and CRDT collaborative editing operate on bounded tile deltas. All compute-heavy pixel work (compositing, filtering, transform resampling, ML selection/inpaint inference) MUST execute in the `studio-engine` crate through the `RasterEngine`/`RenderEngine` traits and MUST NOT introduce `wgpu`/WGSL/GPU dependencies into `handshake_core` ([STU-ARC-002]).

[STU-RAS-005] Every raster operation MUST be reversible through the unified per-document history/undo surface (14.19); an operation that cannot be represented as a `StudioHistoryEntry` delta MUST NOT be shipped as a raster command.

---

### 2. Raster Document and StudioLayer Kinds

[STU-RAS-006] A raster document is a `StudioDocument` whose layer tree contains one or more raster-domain `StudioLayer` nodes over one or more `StudioArtboard` containers. Multiple named artboards MUST be supported inside one document, each with its own pixel dimensions, background, guides, layer auto-nesting, and export path, without forking the document (this dedupes the Photoshop "Artboards" and Affinity artboard surfaces into one `StudioArtboard` primitive; see 14.6 for layout geometry).

[STU-RAS-007] Studio MUST provide the following normative raster `StudioLayer` kinds. Each row is one deduped primitive; the `StudioLayer.kind` discriminant and its payload contract are canonical in 14.23.

| Layer kind (`StudioLayer.kind`) | Normative behavior |
|---|---|
| `raster` (pixel layer) | Editable pixel layer holding `StudioRasterTile` data at the document bit depth; target of all painting, retouching, and destructive filters. |
| `placed_asset` (non-destructive placed container) | Encapsulates source content (raster or vector) unrasterized at native resolution; hosts non-destructive transforms, filters, and effects and supports nested child-document editing. This ONE primitive dedupes Photoshop smart objects and Affinity image/placed layers; §3 defines its instance/link semantics. |
| `group` | Nests child layers with shared opacity, blend mode, effects, mask, and clip scope; default group blend is Pass Through ([STU-RAS-040]). |
| `adjustment` | Hosts one `StudioAdjustment` applied non-destructively to layers below (or clipped to its parent) with a built-in mask (§6). |
| `live_filter` | Hosts one `StudioLiveFilter` (a re-editable, maskable filter effect) applied non-destructively to layers below or clipped to its parent (§5, [STU-RAS-034]). |
| `fill` | Whole-scope re-editable fill: solid color (`StudioSwatch`), gradient (`StudioGradient`), or pattern (`StudioPattern`), including a live tiling-pattern mode where painting one tile updates the repeat (§4). |
| `mask` | Grayscale alpha mask node hiding/revealing its parent; paintable, fillable, selection-derived, or parametric (§3 masking). |

[STU-RAS-008] Adjustment, live-filter, fill, and mask layers MUST be non-destructive and re-editable at any time: their parameters MUST persist as structured `StudioLayer` payload and MUST NOT be baked into pixels until an explicit rasterize/merge command. A destructive equivalent (apply-in-place) MUST be available as an explicit operator/model command, and MUST emit a distinct history entry.

[STU-RAS-009] Studio MUST support layer organization metadata as canonical `StudioLayer` fields: freeform tags (including export-semantic and accessibility tags), color labels, and named layer states (saved visibility/config sets, including query-based states) that recall document variations without duplicating the document. A layer/object find surface MUST query these fields.

[STU-RAS-010] Fill-opacity and layer-opacity MUST be independent, canonical `StudioLayer` fields: fill-opacity fades pixel/fill content while leaving `StudioEffectStack` effects at full strength; layer-opacity fades the whole layer including effects. Both MUST be exposed and MUST NOT be collapsed into a single opacity field.

[STU-RAS-011] Studio MUST support pixel-layer rasterization, merge-down, merge-visible, flatten, and stamp-visible (merge-visible-to-new-layer) operations; each MUST be an explicit command emitting a `StudioHistoryEntry`, and MUST honor matting/defringe/remove-halo cleanup of edge fringe left by a prior selection or extraction.

---

### 3. Placed-Asset (Non-Destructive Container) and Masking Semantics

[STU-RAS-012] The `placed_asset` layer MUST support both embedded mode (source content stored inside the document) and linked mode (source referenced from an external file), with explicit conversion in both directions, and MUST surface link health (up-to-date / modified / missing) as inspectable state with an update-all command. This dedupes Photoshop embedded/linked smart objects and Affinity image layers into one primitive.

[STU-RAS-013] Duplicating a `placed_asset` MUST create a shared-source instance (edits to the source propagate to all instances), while an explicit "new independent copy" command MUST create a detached source. Replacing a `placed_asset` source MUST preserve all applied transforms, live filters, adjustments, and effects across every instance.

[STU-RAS-014] A `placed_asset` MUST support: non-destructive accumulated transforms with a reset-transforms command; unpacking back into its component layers in place (convert-to-layers); exporting its embedded source back to a standalone file in its original format (export-contents touchpoint to 14.13); and statistical stack rendering (mean, median, maximum, range, and the other stack modes) over a multi-layer container for noise reduction and analysis. A "collect linked assets into one portable folder" (package) command MUST be provided as a document-portability touchpoint (see 14.13; honor [GLOBAL-PORTABILITY] — relocatable, not machine-locked).

[STU-RAS-051] [ADD v02.200] Studio MUST provide a focus-stack / seamless-blend composite operation (auto-blend layers): given a multi-layer stack, automatically align (reusing the auto-align primitive), then per-region select and blend the sharpest/best-exposed content into one seamless result (extended depth of field, and seamless panorama/exposure blending), producing a non-destructive masked composite. This composite reuses the existing auto-align, statistical-stack ([STU-RAS-014]), masking ([STU-RAS-018]), and HDR/exposure ([STU-RAS-033]) primitives; it is a named model-steerable command subject to [STU-CON-007].

[STU-RAS-015] `StudioMask` is the single canonical masking primitive across all Studio domains (14.3). Studio MUST support these deduped mask forms, all attachable to any maskable layer or group:

| Mask form | Normative behavior |
|---|---|
| Grayscale (pixel) mask | Paintable/fillable 8/16-bit alpha mask hiding/revealing its parent; created blank, from selection, or from a channel. |
| Vector mask | Path-defined (`StudioVectorPath`) mask with resolution-independent edges; convertible to/from a pixel mask and combinable with one. |
| Clipping mask | Uses a base layer's content/alpha to clip the layers clipped to it. |
| Compound mask | Combines multiple mask nodes non-destructively via boolean operators (add / subtract / intersect / xor). |
| Parametric (live) mask | Non-destructive mask generated live from image properties and auto-updating with the image, in the normative types Hue-Range, Luminosity-Range, and Band-pass; stays re-editable. |

[STU-RAS-016] Masks MUST support density/opacity, feather, and enable/disable, and MUST be linkable or unlinkable from parent-layer position. Any selection (§4) MUST be convertible to a mask, and any mask MUST be loadable as a selection, through one shared conversion path.

---

### 4. Selection and Masking Tools

[STU-RAS-017] `StudioSelectionSet` (14.3) is the single canonical selection primitive. All selection tools produce, refine, or consume a `StudioSelectionSet`; there is no per-tool bespoke selection representation. Every selection-producing tool MUST expose the shared combine modes New / Add / Subtract / Intersect (via both explicit control and modifier keys), a document-wide default anti-alias toggle, and per-tool feather.

[STU-RAS-018] Studio MUST provide the following deduped selection tools/commands. Each row is one primitive collapsing the per-suite variants noted in the provenance overlap map (57).

| Selection primitive | Normative behavior |
|---|---|
| Geometric marquee | Rectangular, elliptical, single-row (1px), and single-column (1px) pixel selections with fixed-ratio/fixed-size constraint, draw-from-center, and proportional modifiers. |
| Freehand lasso | Freehand-drawn selection boundary. |
| Polygonal lasso | Straight-segment click-to-build selection. |
| Magnetic lasso | Edge-snapping selection with width, contrast, and frequency controls. |
| Quick/selection brush | Painted selection that grows to matching regions and snaps to edges, with add/subtract by stroke and an on-canvas selection overlay. |
| Color-range / magic wand (flood select) | Selects similar color/tone from a sampled point by tolerance, with contiguous and sample-all-layers options. |
| Object select | Auto-selects a detected object under a hover, rectangle, or lasso region using an on-device model, with multi-part component selection and optional matting. |
| Select Subject | One-command on-device selection of the dominant subject(s), recordable into a macro/batch. |
| Select Sky | One-command on-device selection of sky regions (dedup target for the sky-select surface). |
| Tonal-range select | Selects Shadows, Midtones, or Highlights tonal bands. |
| Luminosity / alpha select | Builds a selection from a layer's luminosity or content/alpha for luminosity-masking workflows. |
| Color-range select | Eyedropper-sampled fuzziness-masked selection by color similarity with a live preview. |

[STU-RAS-019] Object-select, Select-Subject, and Select-Sky MUST run on-device by default with no cloud dependency; any cloud-accelerated variant is an optional adapter lane per §10 and MUST NOT be a core dependency. On-device ML selection is a native Studio primitive, not a provider feature.

[STU-RAS-020] Studio MUST provide selection-refinement operations as commands on `StudioSelectionSet`: Grow / Shrink (radius, with circular option), Feather (radius), Smooth (radius), Border, and Expand/Contract. A unified edge-refinement surface (refine-edge / select-and-mask) MUST provide matte/edge refinement for hair and fine detail with border width, smooth, feather, and ramp controls, a refinement adjustment brush (matte / foreground / background / feather modes), preview modes (overlay / black matte / white matte / black-and-white / transparent), and output routing to Selection, Mask, New Layer, or New Layer With Mask. This one surface dedupes the source suites' refine-edge and refine-selection dialogs.

[STU-RAS-021] Quick-mask mode MUST let an operator or model convert the active `StudioSelectionSet` into a paintable grayscale/rubylith overlay channel, edit it with any painting tool, and convert it back to a selection, with alternative display modes.

[STU-RAS-022] Studio MUST support saving a selection to a persistent alpha/spare channel and reloading it as a selection, preserved in round-trippable interchange formats (14.13). Save/load selection, alpha channels, and the Channels surface (§7) are one shared mechanism, not three.

---

### 5. Painting, Brush Engine, and Retouching

[STU-RAS-023] Studio MUST implement one native brush engine driving every painting and retouching tool. The brush engine MUST expose, as shared canonical parameters, brush tip/preset, size, hardness, blend mode (`StudioBlendMode`), opacity, flow, spacing, angle/roundness, pressure and tilt dynamics, and a stroke-smoothing option set (0–100 with pulled-string, stroke-catch-up, catch-up-on-end, and zoom-adjust modes). Brush presets MUST be saved and reused as named presets, and tool presets MUST capture a tool plus its full option configuration.

[STU-RAS-024] Studio MUST provide the following deduped painting tools:

| Painting primitive | Normative behavior |
|---|---|
| Brush | Soft/antialiased strokes in the foreground color with full brush dynamics. |
| Pencil / pixel | Hard-edged, aliased, pixel-aligned strokes; supports auto-erase to background over foreground-colored pixels. |
| Mixer brush | Wet-paint mixing with canvas colors using wetness, load, mix, and flow. |
| Color-replacement brush | Paints a replacement color over sampled colors while preserving underlying texture/luminosity. |
| Pattern stamp | Paints with a `StudioPattern`, optionally aligned and impressionist-styled. |
| History / snapshot brush | Paints pixels from a chosen history state or snapshot back into the image (selective, brushed re-state). |
| Art-history brush | Paints stylized strokes derived from a history state with style, fidelity, and area controls. |

[STU-RAS-025] Studio MUST provide fill and gradient primitives: a bucket/flood fill filling similar contiguous areas with a color or `StudioPattern` by tolerance and anti-alias; a gradient tool drawing and editing `StudioGradient` fills interactively on layers, fill layers, and masks with on-canvas stop handles; and gradient geometry modes Linear, Radial, Angle, Reflected, and Diamond. Foreground/background fill and stroke-selection commands MUST be provided. Gradients and patterns authored here are the same `StudioGradient`/`StudioPattern` primitives used by fill layers and effects.

[STU-RAS-026] Studio MUST provide the following deduped retouching / clone-heal-inpaint family as one primitive group operating on pixel layers (and, where the source suites allow, directly on placed-asset/image layers):

| Retouch primitive | Normative behavior |
|---|---|
| Clone stamp | Paints exact pixel copies from a sampled source point with aligned sampling, sample-layer scope, cross-document sources, and clone-source overlay. |
| Healing brush | Paints from a sampled source or pattern while matching texture, lighting, and shading of the destination. |
| Spot/blemish heal | Removes small blemishes by painting or single click, auto-sampling repair texture from the surroundings with no source point. |
| Patch | Repairs a drawn/selected region by dragging it over source pixels, in normal or content-aware mode with structure and color adaptation. |
| Content-aware move | Moves or extends a selected object and content-aware fills the vacated area, with structure/color adaptation and transform-on-drop. |
| Inpaint / object remove | Brushes over an unwanted region and synthesizes a fill from surrounding data using an on-device model (native, local-first). |
| Red-eye | Removes red flash reflections with pupil-size and darken controls while preserving eye detail. |

[STU-RAS-027] Studio MUST provide the local tonal/detail retouch brushes as brush-engine tools: Dodge (lighten) and Burn (darken) with tonal-range targeting (shadows/midtones/highlights), exposure, and protect-tones; Sponge (saturate/desaturate) with vibrance protection; and the local-effect brushes Blur (soften), Sharpen (edge contrast with protect-detail), Median (edge-preserving noise reduction), and Smudge (smear pixels in the drag direction, with finger-painting).

[STU-RAS-028] Studio MUST provide the following deduped eraser family: a general eraser (erase to transparency or background color, in brush/pencil/block modes, and erase-to-history-state); a background eraser (erase sampled background color to transparency while protecting a foreground color, with sampling modes and tolerance); a magic/flood eraser (erase all similar-colored pixels to transparency in one action, by tolerance/contiguity); and an undo brush (paint areas back to an earlier history/snapshot state). Where a source suite routes erasing on a non-destructive placed-asset layer through masking rather than pixel destruction, Studio MUST prefer the masking route by default.

---

### 6. Transforms and Content-Aware Operations

[STU-RAS-029] Studio MUST provide the following deduped transform and reshaping primitives on layers, selections, and placed-asset containers. Numeric and handle-based input MUST both be supported, and every transform MUST carry explicit typed units per [STU-DOC-003].

| Transform primitive | Normative behavior |
|---|---|
| Move | Moves selection or layer content, with auto-select and layer-bounds hover options. |
| Crop | Crops/expands canvas with ratio and absolute presets, overlay guides (including rule-of-thirds and golden-ratio), straighten, delete-vs-hide cropped pixels, crop-to-selection, and content-aware fill of newly exposed areas on commit. |
| Perspective crop | Crops while correcting keystoned perspective to a straight-on rectangle via corner handles. |
| Free transform | Scale, rotate, skew, distort, and flip in one interactive operation with numeric entry and reference-point control. |
| Warp | Grid/handle mesh warp with warp presets and custom control-point deformation, including bezier mesh warp. |
| Puppet warp | Pin-and-deform mesh with density and expansion controls for organic reshaping. |
| Perspective warp | Multi-plane perspective reshaping/correction (single- and dual-plane). |
| Content-aware scale | Scales while protecting flagged content (e.g. subjects) from distortion using a protection mask/channel. |
| Content-aware fill | Fills a selection by synthesizing plausible pixels from sampled source regions, with sampling-area, color-adaptation, and output-target controls (native, on-device). |
| Liquify | Localized mesh push-forward, push-left, twirl, pinch, punch, turbulence, mesh-clone, and reconstruct brushes, with freeze/thaw masking to protect regions from warp. |

[STU-RAS-030] Content-aware fill, content-aware scale, content-aware crop-expand, and inpaint/remove ([STU-RAS-026]) MUST have a native on-device implementation in `studio-engine`. Any generative/provider-model variant is an optional adapter lane (§10) and MUST NOT be the only implementation of these features.

---

### 7. Channels, Color Modes, and Bit Depth

[STU-RAS-031] Studio MUST provide a Channels surface exposing per-document color channels, alpha channels, and spot-color channels with visibility, editability, reorder, rename, and thumbnail controls, and MUST support converting channel content to/from selections, spare channels, and masks (this is the same mechanism as [STU-RAS-022]). Studio MUST provide Duplicate / Split / Merge channel operations and the two channel-math operations: Apply-Image (blend a source layer/channel onto a target with blend mode, opacity, invert, mask, and preserve-transparency) and Calculations (combine two source channels with a blend operation, outputting a new channel, document, or selection).

[STU-RAS-032] Studio MUST support the following document color modes as canonical `StudioColorProfile`-bound states, with explicit convert commands and mode-appropriate feature availability: RGB, CMYK, Lab, Grayscale, Bitmap (1-bit, with 50%-threshold / pattern-dither / diffusion-dither / halftone-screen / custom-pattern conversion), Indexed Color (palette type, color count, forced colors, transparency, matte, dither, editable color table), Duotone (mono/duo/tri/quadtone ink with per-ink transfer curves and overprint), and Multichannel. No implicit device color is permitted; every value carries a `StudioColorProfile` reference ([STU-DOC-003]).

[STU-RAS-033] Studio MUST support 8-, 16-, and 32-bit-per-channel documents. 32-bit floating point MUST store HDR luminance beyond display range. Studio MUST provide: a merge-bracketed-exposures-to-HDR operation (with ghost removal and tone-mapping); HDR Toning and Shadows/Highlights tone-mapping between 32/16/8-bit; a 32-bit HDR editing workflow with a preview-exposure control; and the documented reduced tool/filter/blend-mode availability at 32-bit MUST be surfaced as inspectable capability state rather than silent failure. Bit-depth and mode conversions are model-steerable commands subject to [STU-RAS-003].

[STU-RAS-034] Studio color management MUST provide: working-space profile settings (RGB/CMYK/Gray/Spot) and mismatch policies as saved presets; Assign-Profile (retag without changing values) and Convert-to-Profile (convert values to a destination profile with rendering-intent choice); embed-profile-on-save; soft-proof (proof setup / proof colors) previewing an output condition on screen without converting; gamut warning; a color picker supporting HSB/RGB/Lab/CMYK/hex with out-of-gamut warning; and spot-color library selection from installed color books. Soft-proofing MUST also be available as an in-stack `StudioAdjustment` ([STU-RAS-036] row) and an OpenColorIO (OCIO) source-to-destination color-space transform MUST be available as an adjustment. Full color-pipeline authority is 14.8; the raster surface consumes it and MUST NOT fork it.

---

### 8. Adjustments, Live Filters, Blend Modes, and Effects

[STU-RAS-035] `StudioAdjustment` is the single canonical adjustment primitive; every adjustment below MUST be usable both as a non-destructive `adjustment`-kind layer (with built-in mask, clip-to-parent option, and re-editable parameters) and as an explicit destructive apply-in-place command. Presets and one-click adjustment creation MUST be supported. The adjustment set is normative and deduped (one `StudioAdjustment` kind per row):

| `StudioAdjustment` kind | Normative behavior |
|---|---|
| Brightness/Contrast | Lightness and tonal-difference sliders (legacy-mode option). |
| Levels | Black/white/gamma input-output remap per composite or channel, with eyedroppers and auto options. |
| Curves | Point-editable tonal curve per composite or channel, with on-image targeting, eyedroppers, and auto algorithms. |
| Exposure | Exposure, offset, and gamma for linear/HDR correction. |
| Vibrance | Saturation weighted to protect already-saturated pixels and skin tones, plus a saturation slider. |
| Hue/Saturation (HSL) | Hue/saturation/lightness for master or per-hue-range, with colorize and on-image targeting. |
| White Balance | Temperature and tint cast removal. |
| Color Balance | Cyan-red / magenta-green / yellow-blue per shadows/midtones/highlights, with luminosity preservation. |
| Photo/Lens Filter | Warming/cooling or custom-color tint with density and preserve-luminosity. |
| Black & White | Per-hue luminance-weight monochrome conversion with optional tint. |
| Channel Mixer | Weighted source-to-output channel mix with constant offset and monochrome mode. |
| Selective Color | CMYK ink-percentage shift within specific color ranges, relative or absolute. |
| Color Lookup / LUT | Applies 3D LUT / abstract / device-link lookup tables for graded looks. |
| OCIO | OpenColorIO source-to-destination color-space transform. |
| Gradient Map | Maps tonal values onto a `StudioGradient` with dither and reverse. |
| Recolor | Monochrome tint by a specified hue/saturation/lightness. |
| Split Toning | Independent highlight and shadow tint with balance. |
| Invert | Reverses channel values to a negative. |
| Posterize | Reduces to a set number of flat tonal levels per channel. |
| Threshold | High-contrast black/white around a threshold level. |
| Shadows/Highlights | Per-range recovery with amount, tone width, radius, color correction, and midtone contrast. |
| HDR Toning | Tone-maps 32-bit HDR (or simulates on 8/16-bit) with exposure/gamma, highlight compression, local-adaptation curve, and detail. |
| Desaturate | Removes color to grayscale values in place. |
| Equalize | Redistributes brightness values evenly across the tonal range. |
| Match Color | Matches color statistics between images/layers/selections with luminance, intensity, fade, and neutralize, savable as settings. |
| Replace Color | Fuzziness-masked color selection with hue/saturation/lightness shift in one operation. |
| Clarity / Dehaze / Grain | Midtone-contrast clarity, atmospheric dehaze, and film-grain texture controls. |
| Soft Proof | In-stack output-condition preview for a target color space/device. |
| Normals | Adjusts and corrects normal maps for 3D/game-art pipelines. |

[STU-RAS-036] Where a single named control exists in more than one source suite (e.g. curves, levels, black-and-white, channel-mixer, selective-color, gradient-map, threshold, posterize, invert, exposure, shadows/highlights across Photoshop and Affinity), it MUST map to exactly one `StudioAdjustment` kind per [STU-SECTION-003]; suite-specific extra parameters are merged into that one kind's parameter set.

[STU-RAS-037] `StudioLiveFilter` is the single canonical live-filter primitive: a re-editable, maskable filter hosted on a `live_filter`-kind layer (or applied to a placed-asset container as a non-destructive smart filter). The concrete live-filter catalog — blur family (gaussian, box, median, bilateral, motion, radial, lens, depth-of-field, field, zoom, average), sharpen (unsharp mask, high-pass, clarity), distort, noise, stylize, lighting, and the rest — is enumerated once in 14.9 (Effects, filters & adjustments); 14.4 owns only the non-destructive live-filter-layer mechanism, the smart-filter attachment to placed-asset containers, per-filter masking, and the fact that any 14.9 filter that can run non-destructively MUST be available as a `StudioLiveFilter`. A destructive apply-to-pixels form MUST also exist for each.

[STU-RAS-038] `StudioBlendMode` is the single canonical blend-mode enum, shared by layers, groups, brush tools, fills, and effects across all Studio domains. Studio MUST implement the following normative blend modes:

| `StudioBlendMode` | Group | Normative behavior |
|---|---|---|
| Normal | Normal | Replaces base with blend color. |
| Dissolve | Normal | Randomly replaces pixels with base/blend proportional to opacity. |
| Behind | Normal (tool-only) | Paints only transparent areas of a layer. |
| Clear | Normal (tool-only) | Paints pixels to transparency. |
| Darken | Darken | Keeps the darker of base/blend per channel. |
| Multiply | Darken | Multiplies base by blend, always darkening. |
| Color Burn | Darken | Darkens by increasing contrast toward blend. |
| Linear Burn | Darken | Darkens by decreasing brightness toward blend. |
| Darker Color | Darken | Keeps the lower total-channel-value color. |
| Lighten | Lighten | Keeps the lighter of base/blend per channel. |
| Screen | Lighten | Multiplies inverses, always lightening. |
| Color Dodge | Lighten | Brightens by decreasing contrast toward blend. |
| Linear Dodge (Add) | Lighten | Brightens additively toward blend. |
| Lighter Color | Lighten | Keeps the higher total-channel-value color. |
| Overlay | Contrast | Multiplies or screens by base color, preserving highlights/shadows. |
| Soft Light | Contrast | Diffuse darken/lighten by blend, never pure black/white. |
| Hard Light | Contrast | Multiplies or screens by blend, like a harsh spotlight. |
| Vivid Light | Contrast | Burns/dodges by adjusting contrast per blend. |
| Linear Light | Contrast | Burns/dodges by adjusting brightness per blend. |
| Pin Light | Contrast | Conditional pixel replacement around 50% gray. |
| Hard Mix | Contrast | Sums channels and clamps to primaries/white/black. |
| Difference | Comparative | Subtracts darker from brighter per channel. |
| Exclusion | Comparative | Lower-contrast Difference. |
| Subtract | Comparative | Subtracts blend from base per channel. |
| Divide | Comparative | Divides base by blend per channel. |
| Hue | Component | Base luminance+saturation with blend hue. |
| Saturation | Component | Base luminance+hue with blend saturation. |
| Color | Component | Base luminance with blend hue+saturation (tinting). |
| Luminosity | Component | Base hue+saturation with blend luminance. |
| Pass Through | Group-only | Default group mode; lets inner adjustments/blending affect layers below the group. |

[STU-RAS-039] The additional comparative/intensity modes present in one source suite but not another (e.g. Average, Negation, Reflect, Glow, Erase) MUST be included as `StudioBlendMode` values so no source blend behavior is lost; they are additive to the table in [STU-RAS-038] and canonical in 14.23. `StudioBlendMode` MUST record per-bit-depth availability (32-bit restricts the mode set) and tool-vs-layer applicability (Behind/Clear are tool-only) as inspectable capability metadata rather than silent no-ops.

[STU-RAS-040] `StudioEffectStack` is the single canonical layer-effects/styles primitive. Studio MUST implement the following non-destructive, re-editable, per-layer effects, each re-orderable and independently maskable: Bevel & Emboss (style, technique, depth, direction, size, soften, angle/altitude, gloss contour, contour and texture sub-effects), Stroke (color/gradient/pattern at outside/inside/center), Inner Shadow, Inner Glow, Satin, Color Overlay, Gradient Overlay, Pattern Overlay, Outer Glow, and Drop Shadow (angle, distance, spread/choke, size, contour, noise, knockout). A document-wide Global Light angle MUST be shareable across shadow/bevel effects; a contour/gloss-contour editor MUST be provided. Effect combinations MUST be saveable as reusable styles (a `StudioStyleRegistry` entry), and effects MUST be copyable/pasteable between layers, scalable by percentage, hideable, removable, and convertible into standalone pixel layers.

[STU-RAS-041] Studio MUST provide advanced/conditional blending on every layer as canonical `StudioLayer` fields: Blend-If sliders dropping or revealing pixels by gray or per-channel tonal ranges of this layer and the underlying composite, with split-slider feathering; knockout (shallow/deep) punching through group content; per-channel blend enablement; and interior-effects / clipped-effects / transparency-shapes-layer blending toggles. Fill-vs-opacity ([STU-RAS-010]) is part of this advanced-blending surface.

---

### 9. Raster Export and Placed-Asset Linkage Touchpoints

[STU-RAS-042] Raster layers, artboards, selections, and slices MUST be exportable through the single unified `StudioExportRecipe` surface; the full export/format matrix (PSD/PSB, TIFF, PNG, JPEG, WebP, EXR/HDR, and the round-trip interchange set), slice/region export, and asset-generation rules are normative in 14.13. 14.4 owns only the raster-side touchpoints: a web-slice tool producing independently exportable canvas regions with per-slice name/URL/type, artboard-scoped export paths, per-layer/per-group export markers via layer tags ([STU-RAS-009]), and export of a placed-asset container's embedded source back to a standalone file.

[STU-RAS-043] Studio MUST preserve alpha channels, spot channels, layer groups, masks, adjustment/live-filter/fill layers, placed-asset links, and layer effects across import and export wherever the target format supports them (14.13), and MUST report, as inspectable state, any capability lost on flatten/export rather than dropping it silently.

---

### 10. Provider / Cloud / Generative AI Posture

[STU-RAS-044] Studio's raster domain is local-first: every raster primitive named in 14.4 — including the ML/AI-backed ones (object select, select-subject, select-sky, inpaint/remove, content-aware fill/scale, HDR merge, noise/stack reduction) — MUST have a native, on-device implementation in `studio-engine` that runs offline with no account, sign-in, or cloud call, per [STU-OVR-002]. On-device inference is a core Studio capability, not a provider dependency.

[STU-RAS-045] Cloud-, account-, or vendor-generative features from the source suites are NOT core Studio features. They MUST be recorded as normative rows here and MUST be either an optional `StudioModelAdapter` lane (14.14) or intentionally omitted; none may become a runtime dependency of any core raster primitive:

| Source-suite feature (provenance) | Studio posture |
|---|---|
| Generative fill / generative expand (text-prompted synthesis into a selection) | Optional adapter lane over a pluggable local or remote generative backend; the native content-aware fill/inpaint primitive ([STU-RAS-026], [STU-RAS-029]) is the non-optional baseline. |
| Cloud "neural"/generative filter items (background/scene synthesis, style transfer, super-resolution, colorize) requiring a vendor cloud | Adapter lane per filter where a local model exists; otherwise intentionally omitted. On-device filters that happen to be branded "neural" upstream ship as native `StudioLiveFilter`/adjustments. |
| Vendor generative image service (e.g. Firefly-class) | Adapter lane only; never a dependency; never a Studio brand or panel name. |
| Cloud-backed distraction/object removal modes | Adapter acceleration only; the on-device inpaint/remove primitive is the baseline. |
| Vendor cloud asset libraries (CC-Libraries-class linked assets) | Replaced by native local `placed_asset` links (§3) and the Studio asset library; vendor-cloud sync is an optional adapter, not a dependency. |
| Vendor cloud documents + cloud version history | Replaced by native local `StudioDocument` history/undo (14.19); vendor cloud storage is omitted. Collaboration is native CRDT (14.16/14.17), not vendor cloud. |
| Vendor share-for-review / cloud project spaces | Replaced by native Studio collaboration/review surfaces; vendor cloud is omitted. |

[STU-RAS-046] Any adapter lane under [STU-RAS-045] MUST be opt-in, MUST route through the sandbox->validation->PromotionGate lifecycle ([STU-RAS-003]) like any other model-authored mutation, MUST be attributable in the audit log via `KernelActor`, and MUST degrade to the native on-device primitive (or a clear "adapter unavailable" state) when the provider is absent — it MUST NOT block or break any core raster workflow.

---

### 11. Domain Authority and Dedup Notes

[STU-RAS-047] Where a raster capability is also reachable from another Studio domain (a mask from vector, a gradient/pattern from layout, an export slice, a color profile), it is the SAME primitive exposed through the same typed command ([STU-DOC-004]); 14.4 MUST NOT reimplement or rename it. The full filter catalog is 14.9, the color pipeline is 14.8, Camera-Raw/Develop is 14.12, export/interop is 14.13, and canonical contracts are 14.23; 14.4 references these and MUST NOT fork them.

[STU-RAS-048] Every feature enumerated in 14.4 is a deduped Studio primitive: the per-suite variants recorded in the provenance corpus (files 51, 54, 57) collapse into the single clauses and table rows here. Any raster capability discovered in the provenance corpus that is not represented by a clause or table row in 14.4 is a spec gap and MUST be added here as a new sequential `[STU-RAS-NNN]` clause or table row, never resolved from the corpus at implementation time ([STU-SECTION-002]).

[STU-RAS-049] Studio MUST provide the raster tonal-diagnostics feedback surface that the source photo-editing loops depend on when driving adjustments, levels, curves, exposure, and mode/bit-depth work: a live histogram (per-composite and per-channel, with clip warnings), a persistent multi-point color-sampler (up to at least four sample points reading live values), an eyedropper sampling into the active swatch with sample-size and sample-layer scopes, and an info/readout of pixel values, position, and dimensions under the pointer or over a selection. These are the measurement surface for the adjustment and channel work in §6–§8 and MUST be inspectable state for a no-context model per [STU-RAS-001]; the full color-metering/diagnostics authority is shared with 14.8 and the diagnostics surfaces of 14.16.

[STU-RAS-050] Navigation, view, measurement, and annotation tools that appear on the source raster toolbars but are not pixel-editing capabilities — pan/hand, zoom, rotate-view, navigator, screen/full-screen modes, ruler/measure, count, and canvas notes/annotations — are shared shell, diagnostics, and collaboration primitives owned by the Studio shell and cross-cutting surfaces (14.16, 14.17), not by 14.4. They MUST be deduped to exactly one Studio primitive each in their owning surface and MUST NOT be reimplemented, renamed, or re-catalogued as raster tools here; 14.4 records them only so their source rows are not lost to the raster domain during dedup ([STU-SECTION-003]).


## 14.5 Vector Graphics & Illustration

Vector Graphics & Illustration is the Studio domain that owns editable resolution-independent geometry: paths, vector networks, parametric shapes, boolean/geometry composition, fills and strokes, the multi-attribute appearance model, brushes, transforms and distortions, and the procedural constructs (repeat, blend, live paint, gradient mesh, image trace, intertwine, global edit). It is the deduped normative union of Illustrator's illustration surface, Affinity Designer's vector persona, and Figma/Figma Draw's vector-network drawing surface, collapsed into one Studio primitive set per [STU-SECTION-003].

Every capability in this sub-section operates on `StudioLayer` nodes whose `kind` is `vector` inside the unified `StudioDocument` (14.3), sharing the same selection, history, color, effect, mask, and export surfaces as every other Studio domain. Vector geometry is owned by two canonical primitives — `StudioVectorPath` (single ordered path) and `StudioVectorNetwork` (multi-edge topology) — and paint/appearance is owned by the shared `StudioGradient`, `StudioPattern`, `StudioSwatch`, `StudioStyleRegistry`, and `StudioEffectStack` primitives (14.23). This sub-section references those canonical contracts and MUST NOT redefine their fields; where any statement here conflicts with 14.23, 14.23 wins.

The compute-heavy geometry, tessellation, boolean, stroking, and brush-instancing work is owned by the `VectorEngine` trait in the `studio-engine` crate (14.2); `handshake_core::studio::vector` reaches it only through that typed boundary and never embeds GPU/tessellation dependencies in `handshake_core`.

The numbered groups below are the normative Studio vector feature set. Groups 1-4 define the geometry model, tools, parametric shapes, and geometry operations; groups 5-8 define paint (fills, strokes, appearance, brushes); groups 9-11 define artboards/transforms/procedural constructs; group 12 is the typography touchpoint; group 13 is the optional generative adapter lane; group 14 states the cross-cutting obligations once for the whole sub-section. Feature tables enumerate the deduped sets (tools, shapes, geometry operations, stroke attributes, brushes, provider capabilities) with one row per Studio feature and the collapsed source-suite variants recorded for provenance only. A normative clause governs behavior; a table row names a feature and MUST be read together with the clauses in its group.

---

### 1. Vector Geometry Model: Paths and Vector Networks

[STU-VEC-001] Studio MUST provide exactly two canonical vector geometry primitives, and every vector tool, shape, boolean result, brush spine, and import target MUST resolve to one of them: `StudioVectorPath` (schema id `hsk.studio.vector_path@1`), an ordered sequence of anchors forming one open or closed contour; and `StudioVectorNetwork` (schema id `hsk.studio.vector_network@1`), a graph of anchors joined by first-class selectable edges where any anchor MAY join three or more edges and enclosed regions MAY exist without a single closed contour. A `StudioVectorPath` MUST be losslessly promotable to a `StudioVectorNetwork`; the reverse conversion is lossy and MUST be an explicit flatten/simplify operation, never implicit.

[STU-VEC-002] An anchor MUST carry a position (document units per [STU-DOC-003]), an incoming tangent handle, an outgoing tangent handle, and a handle-mirroring mode. The mirroring mode MUST be one of:

- `independent` — no mirroring; the two tangents move separately (corner behavior).
- `mirror-angle` — the tangents share a direction but keep independent lengths.
- `mirror-angle-length` — the tangents are fully symmetric in direction and length (smooth behavior).

Corner anchors are anchors whose handles are `independent` or zero-length; smooth anchors mirror. Toggling an anchor between corner and smooth MUST be a non-destructive edit that preserves position and, where possible, handle geometry.

[STU-VEC-003] A segment (edge) between two anchors MUST be a cubic Bézier; a straight segment is the degenerate case where both governing handles are zero-length. Segments MUST be independently selectable and directly reshapeable (drag-to-bend), and dragging a straight segment MUST convert it to a curve by synthesizing handles on its endpoints without destroying adjacent geometry.

[STU-VEC-004] Every closed contour and every enclosed network region MUST carry an explicit fill rule, selectable between `non-zero-winding` and `even-odd` (alternate). A `StudioVectorNetwork` MUST support per-region fill state so a single vector layer MAY contain both filled and unfilled regions independently, and MUST support a paint-bucket region operation that fills or clears one enclosed region without altering the network topology.

[STU-VEC-005] Studio MUST provide live (non-destructive) corner treatment on individual anchors via a `StudioCornerSpec` on the anchor, carrying:

- corner kind — `none`, `round`, `rounded-inverted`, `chamfer`, `notch`, and equivalents;
- a per-anchor radius;
- a corner-smoothing percentage that blends a rounded corner toward a continuous-curvature squircle.

Parametric shapes ([STU-VEC-010]) MUST expose the same corner spec per vertex (uniform or per-corner), and corner edits MUST remain re-editable until the layer is explicitly expanded/flattened.

[STU-VEC-006] Studio MUST provide the non-destructive path-topology operations below as first-class commands, each available to both the operator UI and the model command surface as the identical typed contract per [STU-DOC-004]:

- Offset path — numeric inset/outset copy with join-style handling of corners.
- Simplify — anchor-count reduction at an adjustable strength while approximating the source curve.
- Outline stroke — convert a stroked path to filled geometry matching weight, align, caps, joins, and dashes.
- Join / average — join selected open endpoints and average anchor positions on one or both axes.

[STU-VEC-043] Every contour MUST carry an explicit path direction, and Studio MUST provide a reverse-direction command; direction MUST be preserved through import/export and MUST govern even-odd/winding hole resolution ([STU-VEC-004]) and the start/end semantics of arrowheads ([STU-VEC-020]) and text-on-path ([STU-VEC-038]). Reversing direction MUST NOT alter anchor positions or handle geometry.

[STU-VEC-044] Curvature continuity at an anchor MUST be classifiable and preservable: an anchor is C0 (position only, corner), C1 (tangent-continuous, `mirror-angle`), or G2-approximate (curvature-smoothed via corner smoothing [STU-VEC-005]). Tools that convert or reshape geometry MUST NOT silently downgrade a smooth anchor to a corner; any continuity change MUST be an explicit, history-tracked edit.

[STU-VEC-045] Geometry precision MUST be carried at a resolution independent of the current zoom or artboard scale, and coordinate decode/encode MUST occur only at the API boundary per [STU-DOC-003]. Studio MUST NOT round anchor or handle coordinates to device pixels except when the operator or model explicitly invokes pixel snapping ([STU-VEC-029]).

---

### 2. Drawing and Editing Tools

[STU-VEC-007] Studio MUST provide the deduped vector drawing/editing tool set in the table below. Each row is ONE Studio tool that subsumes all listed source-suite variants per [STU-SECTION-003]; a source product's tool name is never the Studio tool name. Every tool MUST emit its edits as `studio.vector` events through the sandbox->validation->promotion lifecycle ([STU-ARC-005]) when model-authored.

| Studio tool | Function (normative) | Deduped source variants |
|---|---|---|
| Pen | Place anchors and straight/curved segments; connect to any existing network anchor (not only endpoints); drag to create mirrored tangent handles | IL Pen + add/delete/anchor-point tools; Affinity Pen (Pen/Smart/Polygon/Line modes); Figma Pen |
| Curvature | Draw and edit smooth curves by placed points with rubber-band preview; click toggles corner/smooth | IL Curvature tool; Figma bend behavior |
| Bend / Anchor convert | Toggle anchor corner<->smooth, break/join tangents, drag straight edge to curve | IL Anchor Point tool; Figma Bend tool |
| Pencil (freehand) | Draw freehand strokes auto-smoothed to a path; options for smoothing, auto-close, and drawing with the current stroke style | IL Pencil + Smooth; Affinity Pencil; Figma Pencil |
| Node / Direct-Select | Select and edit individual anchors, handles, segments, and regions; multi-node selection and alignment | IL Direct/Group Selection; Affinity Node tool; Figma vector edit |
| Reshape | Adjust a path region while preserving overall curve continuity | IL Reshape tool |
| Width | Add/move/remove width points along a stroke to author a variable-width profile on-canvas; reset profile | IL Width tool; Affinity Stroke Width tool |
| Scissors | Split a path at a clicked point into open endpoints | IL Scissors; InDesign Scissors |
| Knife | Cut paths/shapes along a freehand or straight cut line into separate closed objects | IL Knife; Affinity Knife |
| Join | Join selected open endpoints; average anchor positions on axes; combined corner/smooth join | IL Join tool + Join/Average commands |
| Shape Builder | Drag across overlapping regions to merge, click to extract/delete regions, composing custom geometry without manual boolean stacking | IL Shape Builder; Affinity Shape Builder; Figma Shape Builder |
| Blob Brush | Paint filled unified vector shapes that merge with same-attribute geometry | IL Blob Brush |
| Vector Eraser | Erase along a dragged path, splitting/trimming vector geometry | IL Eraser (vector); Path Eraser |
| Corner | Apply live corner treatment ([STU-VEC-005]) to selected anchors | Affinity Corner tool; IL live corner widgets |
| Vector Crop | Non-destructively crop vector/placed objects to a region without discarding content | Affinity Vector Crop |
| Point Transform | Transform an object around a movable, node-keyed origin | Affinity Point Transform |

[STU-VEC-008] The Pen, Curvature, Pencil, Blob Brush, and brush tools ([Group 8]) MUST author either a `StudioVectorPath` or, where the drawn geometry joins existing edges or creates enclosed regions, contribute to a `StudioVectorNetwork`; the tool MUST NOT silently discard network topology by collapsing to a single contour.

[STU-VEC-009] The Node/Direct-Select tool MUST expose, for the current selection, the anchor mirroring mode ([STU-VEC-002]), the per-anchor corner spec ([STU-VEC-005]), and the region fill state ([STU-VEC-004]) as directly editable typed values, so a no-context model can read and set every geometry attribute without pixel-picking.

[STU-VEC-046] Studio MUST provide drawing modes that govern where new vector art is placed relative to the selection: `draw-normal` (above the active layer), `draw-behind` (below the current selection), and `draw-inside` (clipped into the selected object as an automatic clip). The active drawing mode MUST be a persisted, model-readable tool state and MUST apply uniformly to the pen, pencil, shape, brush, and blob tools.

[STU-VEC-047] Tool options (pen rubber-band preview, pencil fidelity/smoothing, shape corner defaults, brush parameters, width-tool step) MUST be persisted per tool and exposed as typed, model-settable values. A model MUST be able to configure a tool and then invoke it deterministically; interactive-only tool state that a model cannot read or set is forbidden where a structured path is practical ([STU-DOC-004]).

[STU-VEC-048] Scissors and knife operations MUST define deterministic results: scissors splits a path at a parametric point on a segment into two open endpoints sharing coincident position; knife cuts across one or more objects along a freehand or straight cut line, producing separately closed regions where the cut crosses filled area and open endpoints where it crosses strokes only. Both MUST preserve the source appearance stack on the resulting fragments.

[STU-VEC-071] Vector selection MUST use the shared `StudioSelectionSet` primitive (14.3) and MUST support object selection, direct anchor/segment/region selection, marquee and lasso selection, and selection-by-attribute (a magic-wand-style query gathering objects/regions that match a paint or geometry attribute within a tolerance). Selection scope MUST be model-addressable (a model MUST be able to state "all objects with stroke color X" as a typed query), not only mouse-driven.

[STU-VEC-072] Vector object management (group/ungroup, arrange/z-order, lock, hide, rename) is owned by the shared `StudioLayer`/`StudioLayerGraph` surface (14.3); the vector domain MUST consume it and MUST NOT reimplement grouping or stacking. Group/ungroup MUST preserve child identity and appearance ([STU-VEC-070]), and z-order MUST be the ordering input consumed by order-dependent geometry operations ([STU-VEC-049]).

---

### 3. Parametric Shape Catalog

[STU-VEC-010] Studio MUST provide a parametric shape catalog. Each shape is a live `StudioVectorPath`/`StudioVectorNetwork` whose defining parameters remain editable until the operator or model explicitly expands it to raw geometry. Expansion MUST be an explicit, history-tracked command. The normative catalog is the deduped union below; a shape present in any source suite MUST be representable, and the per-shape parameters MUST be preserved (no parameter dropped in dedup).

| Studio shape | Editable parameters (normative) | Deduped source variants |
|---|---|---|
| Rectangle | width, height, per-corner radius (uniform or 4-independent), corner kind, corner smoothing | IL/Affinity/Figma/InDesign rectangle; rounded rectangle |
| Ellipse / Arc | width, height, start angle, sweep angle, inner ratio (ring/donut/pie/segment) | IL Ellipse + Arc; Figma ellipse arc; Affinity ellipse/pie/segment/donut |
| Polygon | side count, corner radius, curvature | IL/Affinity/Figma/InDesign/PS polygon |
| Star | point count, inner radius, outer radius, corner radius | IL/Affinity/PS star; Figma star |
| Line / Segment | length, angle; snap-to-perpendicular/tangent | IL Line Segment; InDesign/PS line |
| Spiral | type (linear, decaying, semicircular, Fibonacci, plotted), turns, decay | IL Spiral; Affinity Spiral |
| Grid (rectangular / polar) | rows, columns / concentric dividers, radial dividers, skew | IL Rectangular + Polar Grid |
| Triangle / Diamond / Trapezoid | apex/midpoint/edge-offset parameters | Affinity parametric primitives |
| Arrow | head/tail style, shaft thickness, length | Affinity Arrow |
| Extended primitives | shape-specific parameters (cog: teeth/hole; crescent; heart; tear; cloud; callout: tail position/size; double/square star) | Affinity extended parametric shapes |
| QR / data glyph | encoded payload, error-correction level | Affinity QR Code |

[STU-VEC-011] The shape catalog is extensible: adding a Studio-native parametric shape MUST be a matter of registering a new parameter schema against the same shape-primitive contract, and MUST NOT require a new layer kind or a parallel geometry model. Novelty/vendor-branded source shapes with no production value MAY be omitted, but every parametric behavior that carries geometry meaning MUST be preserved under a Studio-native name.

[STU-VEC-065] Parametric shapes MUST support both handle-based on-canvas parameter editing and numeric entry of every parameter, and the two MUST be equivalent. A shape's parameters MUST remain individually editable after transforms ([Group 10]); a uniform scale MUST NOT silently expand a live shape to raw geometry unless the operator or model requests expansion ([STU-VEC-010]).

[STU-VEC-066] Rectangles, frames, and vector anchors MUST support per-corner independent radius (a uniform value or four/N independent values) plus the corner smoothing of [STU-VEC-005]; a corner radius MUST clamp to the available edge length rather than produce invalid geometry, and the clamp behavior MUST be deterministic.

---

### 4. Boolean, Compound, and Geometry Operations

[STU-VEC-012] Studio MUST provide ONE unified geometry-operation set that subsumes Illustrator's Pathfinder, Affinity's geometry/boolean commands, and Figma's boolean operations per [STU-SECTION-003]. The normative operations are:

| Studio operation | Result (normative) | Deduped source variants |
|---|---|---|
| Union (Add) | Merge selected regions into one combined outline | IL Pathfinder Unite; Affinity Add; Figma Union |
| Subtract (Minus Front) | Remove upper region(s) from the lowest | IL Minus Front/Back; Affinity Subtract; Figma Subtract |
| Intersect | Keep only the overlapping region | IL Intersect; Affinity Intersect; Figma Intersect |
| Exclude (XOR) | Keep non-overlapping regions (even-odd result) | IL Exclude; Affinity Combine; Figma Exclude |
| Divide | Split all overlaps into separate closed regions | IL Divide; Affinity Divide |
| Trim / Merge | Remove hidden overlaps; merge same-color adjacent regions | IL Trim + Merge |
| Crop | Clip artwork to the topmost region | IL Crop |
| Outline | Convert region borders to stroked outline geometry | IL Outline |
| Minus Back | Remove lower region(s) from the topmost | IL Minus Back |
| Offset / Expand stroke | Non-destructive outline offset; convert stroke (incl. variable width) to filled shape | Affinity Contour/Expand Stroke; IL Offset Path; Figma Outline Stroke |

[STU-VEC-013] Every boolean operation MUST support a live (non-destructive) result mode and a flattened (destructive) result mode. In live mode the result is a `StudioCompoundShape` (compound/boolean group) whose child geometry and per-child operator remain individually editable and movable while the composite outline updates; flatten MUST be an explicit command that bakes the composite into a single `StudioVectorNetwork`. Live boolean groups MUST participate in the appearance model ([Group 7]) as a single styleable object.

[STU-VEC-070] Flatten MUST be defined as the general destructive-merge command over any selection (boolean groups, live constructs, and text outlines included), producing one `StudioVectorNetwork` that bakes the composite geometry; it MUST be distinct from group/ungroup (which preserve child identity) and from expand-appearance ([STU-VEC-054], which bakes paint/effects). A model invoking flatten MUST be able to predict that child identity and live parameters are lost.

[STU-VEC-014] Compound paths (a single object with holes formed by multiple contours under a shared fill rule) MUST be a distinct, supported construct from compound/boolean groups, and Studio MUST provide make/release commands for them. Releasing a compound or boolean construct MUST restore the independent child geometry.

[STU-VEC-049] Geometry operations MUST be deterministic and repeatable: the same operation over the same input geometry and z-order MUST produce byte-identical output geometry, and the operation's dependence on z-order (subtract, minus-front/back, crop, trim) MUST be documented in its typed contract so a model can predict the result from selection order alone.

[STU-VEC-050] The Shape Builder tool ([STU-VEC-007]) MUST be defined as an interactive front-end over the same `VectorEngine` boolean core used by the explicit geometry operations ([STU-VEC-012]); merge gestures MUST resolve to union and extract gestures to subtract/divide, so gestural and command-driven geometry share one deterministic implementation and one result contract.

---

### 5. Fills

[STU-VEC-015] A vector object's paint MUST be expressed as an ordered stack of fill entries (see appearance model, [Group 7]); each fill entry MUST carry a fill kind, per-fill opacity, per-fill blend mode, and a visibility toggle. The normative fill kinds are: `none`, `solid` (a `StudioSwatch`/color under an explicit `StudioColorProfile`), `gradient` (`StudioGradient`), `pattern` (`StudioPattern`), and `image` (a placed raster source). Fills MUST be independently reorderable and removable.

[STU-VEC-016] `StudioGradient` MUST support the deduped gradient geometries:

- `linear` — a directional ramp along an axis.
- `radial` — a ramp from a center outward, with editable aspect ratio.
- `angular` (conic) — a ramp swept around a center.
- `diamond` — a ramp along diamond isolines.
- `freeform` — free-placed color points and color lines inside a shape, each with per-point spread and opacity.
- `mesh` — a grid of mesh points interpolating color and per-point opacity (see [STU-VEC-034]).

Every gradient MUST carry a multi-stop ramp with per-stop color and opacity, an editable midpoint/skew between stops, on-canvas handle editing of position/rotation/extent, and per-gradient interpolation control (`perceptual` or `linear`) plus a dithering toggle to control banding. Gradients MUST be applicable to fills and to strokes; on strokes the application mode MUST be selectable between `within`, `along`, and `across` the stroke.

[STU-VEC-017] `StudioPattern` MUST support tiled fills with an editable tile-layout type (`grid`, `brick-by-row`, `brick-by-column`, `hex-by-row`, `hex-by-column`), brick/hex offset, tile size (with move-with-art), spacing, overlap order, and side/corner/start/end tiling for path application. A pattern MUST be transformable independently of the object it fills (move/scale/rotate the fill without moving the object). Studio MUST also support a live-source pattern that tiles another in-document object (layer/group/frame) as a repeating fill or stroke with spacing/alignment controls.

[STU-VEC-067] Studio MUST provide a pattern editing mode that isolates a pattern's tile artwork for direct editing with live preview of the tiled result and dimmed neighbor copies, exiting back to the document without materializing the tiles. Edits to a pattern definition MUST update every object that references it ([STU-VEC-051]).

[STU-VEC-018] Image fills MUST support the scaling modes below, with rotation in 90-degree steps:

- `fill`/`cover` — scale to cover the bounds, cropping overflow.
- `fit`/`contain` — scale to fit within the bounds.
- `crop` — non-destructive in-bounds reposition/scale/rotate of the source, keeping the full source recoverable.
- `tile` — repeat the source at a set scale.

Image fills MUST also expose non-destructive render-time image adjustments (exposure, contrast, saturation, temperature, tint, highlights, shadows) consistent with the raster domain (14.4). Video/animated fills are an optional playback concern and MUST NOT be a required vector feature.

[STU-VEC-019] The color entry surface for any fill or stroke MUST accept HEX, RGB, HSL/HSB, CMYK, and Lab input under an explicit `StudioColorProfile` ([STU-DOC-003]), provide an eyedropper that samples anywhere on the canvas (including rendered images and gradients), support out-of-gamut warnings, and support global swatches (edit-updates-all-uses) and spot swatches with tint and Lab definitions. Named palette/harmony generation and recolor are specified in 14.8; the vector domain consumes those primitives and MUST NOT fork a parallel color model.

[STU-VEC-051] A gradient or pattern fill MUST be storable as a typed swatch in the `StudioSwatch`/`StudioStyleRegistry` surface and reusable across objects and documents; editing a shared gradient/pattern swatch MUST update every object referencing it, and an object MUST be able to break the link to hold a local copy. Freeform and mesh gradients MUST retain per-point editability when stored and re-applied.

[STU-VEC-052] Selection-wide color editing MUST be supported: a mixed selection MUST enumerate every distinct color/gradient/pattern/style in use and allow swapping each across all uses in one edit, and a "select same" query ([STU-VEC-037]) MUST be able to gather objects by any single paint attribute. These operations MUST route through the standard command lifecycle so bulk recolor edits are auditable and undoable as one history entry.

---

### 6. Strokes

[STU-VEC-020] A vector object's stroke MUST be expressed as an ordered stack of stroke entries paralleling fills ([STU-VEC-015]); each stroke entry MUST accept multiple stacked paint fills (solid/gradient/image/pattern) with per-fill opacity and blend behavior. Each stroke entry MUST carry the attributes in the table below; the attribute set is the deduped union across the source suites and no listed attribute MUST be dropped.

| Stroke attribute | Values (normative) | Deduped source variants |
|---|---|---|
| Weight | numeric width; whole-object or per-side (all/top/bottom/left/right/custom); weight excluded from layer bounds where applicable | IL/Affinity weight; Figma per-side weight |
| Align | `center`, `inside`, `outside` relative to the path | IL/Figma stroke align |
| Caps | `butt`/`none`, `round`, `projecting`/`square` | IL/Figma caps |
| Joins | `miter`, `round`, `bevel`, with miter-angle/limit threshold | IL/Figma joins |
| Dashes | up to N dash/gap pairs; dash cap (`none`/`round`/`square`); align-dashes-to-corners toggle; dotted preset | IL dashes; Figma dash patterns |
| Arrowheads | start/end markers (none, arrow, triangle, reverse triangle, diamond, custom) with independent scale, swap, and tip/end alignment | IL arrowheads; Figma endpoint markers |
| Variable width | a width profile (preset or user-authored via the Width tool [Group 2]) reshaping thickness along the path; flip along/across; taper | IL width profiles; Affinity pressure profile; Figma width taper |
| Brush stroke | switch stroke rendering to a `StudioVectorBrush` ([Group 8]) with direction control | IL brush strokes; Figma brush stroke type; Affinity vector brush |
| Dynamic stroke | non-destructive hand-drawn wobble with frequency, wiggle, and smoothen parameters | Figma Draw dynamic stroke |

[STU-VEC-021] Variable-width profiles MUST be storable as reusable named profiles in the `StudioStyleRegistry`, and outlining a variable-width or brush stroke ([STU-VEC-006], [STU-VEC-012]) MUST produce filled geometry that matches the rendered stroke including its width variation.

[STU-VEC-062] Caps/joins/arrowhead configuration MUST be defined for both simple open-path endpoints and for closed or branching (network) endpoints; where a network anchor terminates three or more edges, Studio MUST provide advanced endpoint controls resolving cap/join rendering per incident edge, and MUST NOT leave branching-endpoint rendering undefined.

[STU-VEC-063] On-canvas stroke and gradient editing MUST be exposed through a live annotator (draggable weight, dash, arrowhead, gradient-stop, and gradient-geometry handles) whose every manipulation maps to a typed value edit, so the same change is reproducible via the model command surface ([STU-VEC-047]). The annotator MUST be toggleable/hideable without losing the underlying values.

[STU-VEC-064] Studio MUST provide a directional-transparency (opacity-gradient) authoring surface that applies a gradient in the alpha channel of an object independently of its color fill, editable on-canvas like a color gradient ([STU-VEC-016]); this MUST reduce to a gradient on the object's opacity within the shared appearance model, not a separate object type.

---

### 7. Appearance Model, Graphic Styles, and Effects

[STU-VEC-022] Studio MUST provide a per-object appearance stack: an object, group, or layer MUST carry an ordered list of appearance rows composed of fills ([Group 5]), strokes ([Group 6]), opacity/blend settings, and effect entries, where each row is independently toggleable, duplicable, reorderable (reorder changes render order), and deletable. A single object MUST support multiple fills and multiple strokes, each with its own color/paint, opacity, blend mode, and effects. This is the same appearance surface used across Studio domains and MUST NOT be a vector-only reimplementation.

[STU-VEC-023] Live effects MUST be attachable at any of these scopes, and MUST be carried in the shared `StudioEffectStack` primitive (14.9, 14.23):

- the whole object;
- a group/layer target ring, affecting all children collectively;
- a single fill or stroke row within the appearance stack ([STU-VEC-022]).

Effects MUST remain non-destructive and re-editable until an explicit expand-appearance command ([STU-VEC-054]) bakes them into concrete geometry/raster.

[STU-VEC-024] Graphic styles MUST be named, reusable appearance presets stored in the `StudioStyleRegistry`. Studio MUST support:

- apply a style to a selection;
- additive-merge of a style onto an object's existing appearance;
- break-link to hold a local copy;
- redefine-from-selection, updating all linked users;
- shared style libraries across documents;
- a "new art inherits current appearance vs. basic appearance" toggle;
- clear-appearance and reduce-to-basic-appearance commands.

Effect stacks MUST also be publishable as named effect styles reusable like color/type styles.

[STU-VEC-025] Opacity and blend mode MUST be settable at object, group, layer, and individual appearance-row level, and Studio MUST support opacity (luminance) masks with clip, invert, link, and enable/disable controls, plus isolate-blending and knockout-group options. Blend modes MUST draw from the single canonical `StudioBlendMode` set (14.8/14.23); groups/frames MUST default to pass-through so children blend with content below.

[STU-VEC-053] Studio MUST support sibling/clip masking in addition to opacity masks: a vector object marked use-as-mask MUST clip the objects above it within its container to its region non-destructively and reversibly, with a selectable mask mode:

- `alpha` — the mask's transparency drives the clip.
- `vector` — the mask's hard outline drives the clip.
- `luminance` — the mask's brightness drives the clip.

Masking MUST be a shared Studio capability ([STU-DOC-004]) using the canonical `StudioMask` primitive, not a vector-only reimplementation.

[STU-VEC-054] Expand-appearance MUST be defined precisely: it MUST bake the current appearance stack (multiple fills/strokes, per-row effects, brushes, live corners, and live constructs) into concrete geometry and/or raster layers that reproduce the rendered result, and MUST be the single explicit boundary between non-destructive editing and destructive materialization. Expansion MUST be one history entry and MUST NOT occur implicitly during ordinary edits, save, or export.

---

### 8. Brushes

[STU-VEC-026] Studio MUST provide vector brushes as a single canonical `StudioVectorBrush` primitive with a discriminated `kind` field; a brush is applied either as a path stroke ([STU-VEC-020]) or via the brush/blob tools ([Group 2]). The normative brush kinds are the deduped union of Illustrator's brush families and Figma Draw's brush additions:

| Brush kind | Behavior (normative) | Deduped source variants |
|---|---|---|
| Calligraphic | Angled-nib stroke with angle, roundness, and size, each fixed / random / pressure-driven | IL Calligraphic |
| Scatter | Distribute copies of source art along the path with size, spacing, scatter, rotation variation | IL Scatter; Figma Draw scatter |
| Art | Stretch source artwork along the path length with direction, scale, and non-stretch guide segments | IL Art |
| Bristle | Simulate natural bristle painting with shape, bristle length/density/thickness, paint opacity, stiffness | IL Bristle |
| Pattern | Tile side/corner/start/end pattern tiles along a path with fit, flip, and auto-generated corners | IL Pattern |
| Image / textured | Paint raster-textured organic strokes along an editable vector spine | Affinity Vector Brush; Figma Draw brush |
| Custom (from art) | Capture any single vector layer (shape/path/flattened text) as a reusable brush applied along strokes | Figma Draw custom brush |

[STU-VEC-027] Each brush kind MUST support a colorization method remapping brush art to the current stroke color:

- `none` — keep the brush art's own colors;
- `tints` — remap to tints of the stroke color;
- `tints-and-shades` — remap to tints and shades of the stroke color;
- `hue-shift` — shift the brush art's hues toward the stroke color.

Each brush kind MUST also support per-path stroke-option overrides that alter brush parameters on one applied stroke without editing the shared brush definition. Brush definitions MUST be storable in the `StudioStyleRegistry` and shareable across documents.

---

### 9. Artboards, Frames, and Layout Touchpoints

[STU-VEC-028] Vector artwork MUST be placeable on one or more `StudioArtboard` containers within a single `StudioDocument` ([STU-DOC-001]). Per-artboard attributes MUST include name, preset/custom size, position, orientation, constrain-proportions, and display aids (center mark, cross hairs, safe areas). Studio MUST support many artboards per document and per-artboard ruler-origin selection (document-global origin or per-artboard reset). Frame containers, constraints, and auto-layout are owned by the layout domain (14.6) and design-system domain (14.10); the vector domain MUST consume those primitives (`StudioArtboard`, `StudioConstraint`, `StudioAutoLayout`) rather than fork them.

[STU-VEC-029] Studio MUST provide the vector-relevant alignment and measurement surfaces below:

- Align / distribute to selection, key object, or artboard, with numeric spacing values.
- Ruler guides, object-to-guide conversion, and release-guide-back-to-object.
- Live distance guides showing spacing between the selection and its neighbors or the artboard.
- Snap-to-perpendicular and snap-to-tangent while drawing.
- Pixel-snapping options for raster-targeted output.
- Isolation mode — isolate a group/subpath/symbol for editing while dimming and locking the rest, with a breadcrumb to exit levels.
- Distribute and orient objects along an arbitrary path spine.

[STU-VEC-057] Studio MUST provide vector measurement/inspection surfaces usable by operator and model: measure distance/angle between points, report enclosed region area in document units, and enumerate document vector inventory (object counts, fonts-as-outlines, linked/embedded placed images, spot colors, and pattern/gradient usage) for audit. Measurement readouts MUST be typed values, not screen-only overlays.

[STU-VEC-058] Reusable vector symbol instances are owned by the design-system domain (14.10) via `StudioComponent`/`StudioComponentInstance`; the vector domain MUST expose its geometry as valid symbol source and MUST honor per-instance override/sync behavior defined there, but MUST NOT fork a parallel symbol model. Isolation-mode editing of a symbol master MUST propagate to synced instances through that domain's contract.

---

### 10. Transforms and Distortions

[STU-VEC-030] Studio MUST provide the deduped transform and distortion set below. Each is a typed operation available identically to operator and model. Live/envelope distortions MUST be non-destructive constructs whose contents remain editable until explicitly expanded.

| Studio transform | Function (normative) | Deduped source variants |
|---|---|---|
| Move / Rotate / Scale / Reflect / Shear | Affine transforms about a settable reference point; scale-strokes-and-effects, corners, and pattern-tile toggles; numeric transform dialog; repeat-last-transform; per-object randomized transform-each | IL Rotate/Reflect/Scale/Shear + Transform Each; Affinity/InDesign transforms |
| Free Transform | Combined on-canvas move/scale/rotate/shear with constrain and perspective/distort modifiers | IL/InDesign Free Transform |
| Free Distort | Reshape by dragging four corner points of a distortion frame | IL Free Distort effect |
| Perspective distort | Map artwork onto a perspective plane / mockup surface | IL perspective grid + Mockup |
| Envelope Distort | Wrap artwork in an editable envelope from a warp preset, a custom mesh grid, or a top object; options control fidelity and gradient/pattern distortion; edit-contents and expand | IL Envelope Distort |
| Warp group | Live perspective/quad/mesh warp wrapping children that stay editable | Affinity warp groups |
| Puppet Warp | Pin-and-drag mesh deformation of a single object | IL Puppet Warp |
| Liquify-family warps | Warp, twirl, pucker, bloat, scallop, crystallize, wrinkle brush distortions on vector geometry | IL liquify tool group |

[STU-VEC-031] Studio MUST provide the procedural repeat construct with `radial`, `grid`, and `mirror` modes, kept live so instance counts, spacing, and symmetry stay editable, with make/release/options and expand. Repeat instances MUST render from a single source definition and MUST NOT be materialized as independent copies until expanded.

[STU-VEC-055] All distortion constructs that wrap editable content (envelope, warp group, puppet warp, perspective) MUST provide an edit-contents mode that lets the operator or model edit the underlying source geometry while the distortion continues to apply live, and an explicit expand that bakes the distorted result. Distorting a gradient/pattern fill MUST honor an option controlling whether the fill distorts with the geometry or remains undistorted.

[STU-VEC-056] Symmetry/mirror drawing MUST be supported for vector authoring (single- or multi-axis), reflecting live strokes across the configured axes; the mirrored result MUST be materializable to ordinary editable geometry on demand and MUST NOT depend on an interactive-only session state that a model cannot query.

---

### 11. Procedural Constructs: Blend, Live Paint, Gradient Mesh, Image Trace, Intertwine, Global Edit

[STU-VEC-032] Blend: Studio MUST support blending between two or more objects to generate live intermediate steps, with:

- spacing modes — `smooth-color` (auto step count for a smooth transition), `specified-steps` (fixed count), and `specified-distance` (fixed gap);
- orientation control (align to page or to the spine);
- an editable, replaceable, and reversible spine, plus reverse-front-to-back.

Blends MUST be live (re-editable and re-flowable along the spine) until explicitly expanded.

[STU-VEC-033] Live Paint: Studio MUST support a live-paint construct that treats overlapping paths as a surface of fillable faces and paintable edges, filling/stroking regions by click, with gap-detection options that close paint leaks by gap size, plus make/merge/release/expand. Live paint faces MUST update automatically as the underlying paths are edited.

[STU-VEC-034] Gradient Mesh: Studio MUST support gradient-mesh objects (`StudioGradient` kind `mesh`) — a grid of mesh points and lines interpolating color and per-point opacity across a shape — with add/remove mesh point/line editing and conversion from an existing gradient or shape. Mesh geometry MUST be editable with the node tooling ([Group 2]).

[STU-VEC-035] Image Trace: Studio MUST provide a deterministic native raster-to-vector trace primitive converting a placed raster into editable `StudioVectorPath`/`StudioVectorNetwork` geometry, with the full option set below, plus make / make-and-expand / release / expand:

- Mode — `color`, `grayscale`, or `black-and-white`.
- Palette and count — palette selection with a color count, gray count, or black/white threshold.
- Paths — trace fidelity to the source pixels.
- Corners — corner-detection aggressiveness.
- Noise — minimum traced-area size (ignore small speckles).
- Method — `abutting` (non-overlapping regions) or `overlapping` (stacked regions).
- Fills / Strokes — produce filled regions, stroked paths, or both.
- Snap-curves-to-lines — straighten near-linear curves.
- Ignore-white — drop white/background regions.

This native trace MUST NOT require any provider or network; a generative/AI-assisted vectorize is a separate optional lane ([Group 13]).

[STU-VEC-036] Intertwine: Studio MUST support an intertwine construct that makes selected overlapping objects appear woven (one object's region passing over/under another) non-destructively, with make/release/edit. The over/under assignment at each crossing MUST be a stored, editable, model-readable value.

[STU-VEC-059] Every procedural construct in this group (blend, live paint, gradient mesh, image trace, intertwine, repeat) MUST persist its live parameters and its source references as vector authority ([STU-VEC-042]) so the construct survives save/load and round-trips without being silently flattened, and MUST expose those parameters to the model command surface. A construct that can only be authored interactively and cannot be inspected or re-parameterized by a model is non-conformant.

[STU-VEC-037] Global Edit: Studio MUST support a global-edit mode that edits all similar objects together (scoped by matching shape/size/appearance across artboards) and a select-same query set that gathers objects by any of:

- fill paint or stroke color;
- stroke weight;
- opacity;
- blend mode;
- graphic style;
- shape kind;
- symbol/component instance.

Global edits MUST propagate through the standard command/validation lifecycle so model-authored global edits are auditable ([STU-ARC-005]).

---

### 12. Text-on-Path Touchpoint

[STU-VEC-038] A `StudioVectorPath`/`StudioVectorNetwork` MUST be usable as a typographic baseline (text on a path) and as a text-frame boundary (area type). The vector domain owns only the geometry; the text run, shaping, path-text options (alignment, flip, spacing, start/end insets), and area-type behavior are owned by the Typography engine (14.7) via the `StudioTextStory`/`StudioTypeStyle` primitives. The vector domain MUST expose the path as a stable typographic reference and MUST NOT reimplement text layout. Converting text to outlines (create-outlines) produces standard vector geometry under this sub-section.

[STU-VEC-068] Editing the geometry of a path bound to text (moving anchors, reshaping segments, reversing direction [STU-VEC-043]) MUST reflow the bound text along the updated baseline without detaching the text run, and deleting the path MUST follow the typography domain's detach/relink contract (14.7) rather than silently discarding the text. The vector domain MUST NOT bake text to outlines as a side effect of geometry editing.

[STU-VEC-069] Vector geometry authored under this sub-section MUST round-trip through the interchange formats owned by 14.13 (SVG as the primary open vector interchange, plus PDF and the source-suite vector formats) preserving anchors, handles, fill rules, corner specs, appearance stacks where the target format allows, and degrading predictably where it does not. Import/export fidelity, format matrices, and lossy-mapping rules are specified in 14.13; the vector domain MUST expose its primitives in a form that interop can serialize without a private shadow model.

---

### 13. Provider / AI Lane (Adapter-Backed, Optional)

[STU-VEC-039] Studio's default vector pipeline is fully local and deterministic; generative capabilities are an OPTIONAL adapter lane consistent with the local-first posture ([STU-OVR-002]). The deterministic recolor and raster-to-vector trace primitives are NATIVE ([STU-VEC-035], and the deterministic recolor primitive in 14.8); the generative variants below MUST be routed through `StudioModelAdapter` (14.23) via existing Handshake model routing, MUST be clearly marked as adapter-backed/optional in the UI and command surface, and MUST degrade cleanly to the native primitive (or an explicit unavailable state) when no adapter is configured. No generative feature is a required build gate for the vector domain.

| Provider/AI capability | Native fallback (normative) | Lane |
|---|---|---|
| Text-to-Vector (scenes/subjects/icons) | none (adapter-only); document remains ordinary editable vector output | adapter-backed / optional |
| Text-to-Pattern (generative pattern) | native `StudioPattern` authoring ([STU-VEC-017]) | adapter-backed / optional |
| Generative Shape Fill | native fills, gradients, patterns, live paint | adapter-backed / optional |
| Generative Recolor | native deterministic recolor + harmony palettes (14.8) | adapter-backed / optional |
| Generative Vectorize (raster->vector) | native Image Trace ([STU-VEC-035]) | adapter-backed / optional |
| Generative Expand (vector/image outpaint) | none (adapter-only) | adapter-backed / optional |

[STU-VEC-040] Every adapter-backed generative result MUST land as ordinary local `StudioDocument` content (editable vector geometry, patterns, or recolored artwork) subject to the same authority, history, and export surfaces as hand-authored geometry, and MUST carry a `KernelActor` attribution marking it model/adapter-authored ([STU-ARC-003]). A generative result MUST pass the sandbox->validation->`PromotionGate` lifecycle before it changes document authority ([STU-ARC-005]); provider availability, credentials, and offline-parity classification follow the provider registry referenced in 14.14, not this sub-section.

[STU-VEC-060] The native deterministic primitives and the generative adapter lane MUST be separable: the vector domain MUST build, validate, and ship with the adapter lane entirely absent, and the operator/model command surface MUST clearly distinguish a deterministic native command (e.g. Image Trace, deterministic recolor) from its optional generative counterpart so intent is never ambiguous. A generative command MUST NOT shadow, replace, or silently reroute a native deterministic command.

[STU-VEC-061] Prompt text, style-reference selections, and any source content sent to a generative adapter MUST be treated as model/adapter input governed by Handshake model routing and the provider registry (14.14); this sub-section does not authorize any implicit network egress. When adapter-backed vectorize/recolor is unavailable, Studio MUST fall back to the native primitive named in the [STU-VEC-039] table or surface an explicit unavailable state, never a silent no-op.

---

### 14. Cross-Cutting Obligations

[STU-VEC-041] Every vector tool, shape, geometry operation, fill/stroke attribute, appearance row, brush, transform, and procedural construct in this sub-section MUST satisfy the Studio cross-cutting obligations, stated once here rather than per feature:

- Model visibility and steerability — a stable command identifier and a typed contract for every capability (14.16).
- Quiet/headless operation — no focus-stealing or foreground popups during model or background work (14.20).
- Dual-audience UserManual — an in-product manual entry enabling a no-context model to operate the capability (14.22).
- GUI test hooks and visual capture — a stable `author_id` test hook on every operator control and Argus visual-capture coverage (14.16/14.22).

A vector capability is not complete until its typed command contract, its GUI surface with stable test hooks, and its UserManual entry all exist and its geometry round-trips through the `VectorEngine` boundary.

[STU-VEC-073] Vector history/undo MUST use the shared `StudioHistoryEntry` surface (14.19): each discrete vector command — a geometry edit, a boolean/geometry operation, an expand, a flatten, an appearance change, a bulk recolor, or a generative result — MUST record exactly one history entry that is individually undoable and redoable, and destructive commands (expand, flatten, knife) MUST be undoable to restore the pre-command live construct. The vector domain MUST NOT batch unrelated edits into one entry in a way that prevents targeted undo.

[STU-VEC-042] All durable vector authority (geometry, appearance, styles, brushes, patterns, gradients) MUST persist through the canonical Studio SurrealDB tables and `studio.vector` EventLedger events defined in 14.23 under the SurrealDB-only authority guard with the `no_sqlite_tripwire` in force ([STU-ARC-003], [STU-ARC-004]); live collaborative vector editing is CRDT-backed. Where this sub-section and 14.23 disagree on any type, field, event, or schema id, 14.23 is canonical and this sub-section MUST be corrected to match.


## 14.6 Page Layout & Publishing

Page Layout & Publishing is the Studio domain catalog for multi-page, print-and-publish document construction: the page/spread and parent-page model, threaded text stories, placed-graphic frames and their linked resources, tables, the layout-facing style system, long-document assembly (books, tables of contents, indexes, notes, cross-references), grids and guides, and the prepress/output pipeline (preflight, packaging, print, PDF/X, separations, data merge). It is the deduped normative union of the two page-layout source suites in the provenance corpus (InDesign, Affinity Publisher); per [STU-SECTION-003] each shared capability collapses to exactly ONE Studio primitive and ONE command family, and no source-suite product, panel, format, or menu name is a Studio name.

This catalog operates entirely on the shared Studio primitive set of 14.3 and MUST NOT introduce a parallel layout document model. A layout document is a `StudioDocument` (14.3 [STU-DOC-001]) whose containers are `StudioPageSpread` nodes; page furniture, frames, tables, and placed assets are `StudioLayer` nodes; flowed copy is `StudioTextStory`; every named format is a record in `StudioStyleRegistry`; every ruler/baseline/column construct is a `StudioLayoutGrid`; every render-to-output configuration is a `StudioExportRecipe`. Field-level definitions for every type, enum, event, table, and validation check named here are owned by 14.23 (Canonical Studio Authority Contracts); where this catalog and 14.23 conflict, 14.23 wins. This catalog states the normative layout behavior and enumerates the feature surface; it references primitives rather than redefining their fields.

Domain boundaries this catalog holds:
- The **typography engine** (glyph shaping, composers, OpenType, kerning/tracking, hyphenation/justification internals, `StudioTypeStyle` attribute semantics) is owned by 14.7. This catalog owns the *application* of type styles inside layout (paragraph/character style records in `StudioStyleRegistry`, style-driven layout flow, paragraph-level flow attributes, style-to-export-tag mapping) and references 14.7 for the attribute payloads.
- **Vector geometry and path editing** (frame outlines as paths, stroke geometry, text-on-path curve math, boolean shape building) are owned by 14.5. This catalog owns frames *as layout containers* and references 14.5 geometry.
- **Raster/placed-image pixel editing** is owned by 14.4; this catalog owns placement, linking, and fitting of raster assets.
- **Color pipeline and profiles** (`StudioColorProfile`, swatches, gradients, ink/overprint math, separations rendering) are owned by 14.8; this catalog owns the prepress *surfaces* that drive that pipeline to output.
- **Object effects/transparency** (drop shadow, glows, feathers, blend modes) share the `StudioEffectStack` primitive with 14.9; this catalog states their layout-frame targeting.
- **Interactive, multi-state, media, and EPUB export** touchpoints are owned by 14.11; this catalog defines the layout-side authoring of those objects and hands off export.
- **Per-file history and undo** are owned by 14.19; this catalog references it for document states and recovery.

---

### 1. Page and Spread Model

[STU-LAY-001] A layout `StudioDocument` MUST hold an ordered set of `StudioPageSpread` containers. A `StudioPageSpread` holds one or more pages; a facing-pages document pairs pages across a binding spine, and a non-facing document holds single-page spreads. The spread is the unit of parent-page application, spanning-object placement, and print imposition.

[STU-LAY-002] Multi-page (island) spreads are normative: a `StudioPageSpread` MUST support holding more than two pages (up to a document-configured maximum of at least ten) to model gatefold, trifold, accordion, and other fold formats. A per-document and per-spread "allow shuffle" flag MUST control whether repagination may reflow pages into or out of a spread; disabling shuffle preserves an island spread during page insertion/deletion.

[STU-LAY-003] Pages MUST support mixed sizes and orientations within one document. A page carries its own trim size, orientation, margins, bleed, slug, and liquid-layout rule independent of sibling pages (per-page geometry via the page-resize operation). Page and spread operations — insert, move, duplicate, delete, reorder by drag, hide/unhide from view and output, apply color labels, and a move/copy-pages dialog for precise placement — MUST be exposed as typed commands and MUST emit `studio.layout` EventLedger events.

[STU-LAY-004] View-only spread rotation (90°/180°/270°) MUST be available for editing rotated content without transforming the underlying objects; the rotation is a view state on the `StudioPageSpread` and MUST NOT change stored object geometry.

[STU-LAY-005] A layout document MUST carry a **document-wide layer model**: named layers span every page/spread with per-layer color, visibility, lock, print/export suppression, guide visibility, a wrap-when-hidden policy, and expandable per-object sublists. Document layers (`StudioLayer` bands at document scope) are distinct from a single page's object stack and MUST reorder every page's objects together when reordered.

#### 1.1 Parent (Master) Pages

[STU-LAY-006] Parent pages (the deduped Studio name for master pages) are reusable page templates applied to document pages. The parent-page model MUST support: multiple parents applied to one page simultaneously (each applied parent surfaces as its own `StudioLayer` band in the layer graph); parent-based-on-parent inheritance (a parent may be based on another parent, cascading changes); nested parents (one parent applied onto another); application to page ranges; and loading parents from another document.

[STU-LAY-007] Parent item override semantics MUST be preserved exactly: override a single parent item on a document page, override-all parent items on a page, detach an overridden item from its parent (severing the link), remove overrides (restoring inheritance), and a per-item "allow overrides" flag that can be disabled so an item cannot be locally altered.

[STU-LAY-008] A **primary text frame** MUST be supportable on a parent page: a designated text frame that new pages auto-adopt, that re-threads automatically when the applied parent changes, and that resizes to new page geometry without manual override. Primary text frames are the anchor for smart text reflow ([STU-LAY-020]).

#### 1.2 Liquid Layout and Adjust Layout

[STU-LAY-009] Studio MUST implement responsive layout adaptation when page size, orientation, margins, or bleed change. Two mechanisms coexist: per-page **liquid rules** (applied continuously as geometry changes) and the on-demand **Adjust Layout** operation. The liquid rule set is enumerable and normative:

| Liquid rule | Behavior |
|---|---|
| Off | No liquid adaptation; objects keep absolute geometry. |
| Scale | All page content scales proportionally, preserving relative positions. |
| Re-center | Content keeps original size and re-centers on the resized page. |
| Object-based | Per-object pins to page edges plus resize constraints give mixed fixed/relative behavior. |
| Guide-based | Liquid guides slice the page; objects a guide crosses stretch while text reflows and images resize without distortion. |
| Controlled by Parent | The page inherits whatever liquid rule its parent page defines. |

[STU-LAY-010] **Adjust Layout** MUST recompute object positions and sizes when page size, margins, or bleed change, with options to adjust font size (with a min/max limit), include locked content, and move ruler guides. Adjust Layout is a discrete, undoable command distinct from the continuous liquid rules.

[STU-LAY-011] **Alternate layouts** MUST be supported: multiple named page-size/orientation variants coexist inside one `StudioDocument`, displayed side by side in the page-navigation surface, with stories linked ([STU-LAY-019]) back to the source layout so edits can propagate. A **flex/container layout** mode (container-based responsive layout with direction, wrap, alignment, and spacing properties) MUST be available with conflict reporting against fixed positioning.

[STU-LAY-012] Alignment and distribution MUST align/distribute selected objects to selection, a key object, margins, page, or spread, including distribute-by-spacing with explicit gap values. Gridified drawing (splitting a single drag of any frame tool into an equal grid of frames via modifier keys) MUST be supported.

[STU-LAY-013] The document MUST provide crash-safe **automatic recovery** (recovering unsaved changes on next launch from a configurable recovery location) and a **document-states** surface that lists session edit states and can jump the document to any recorded state beyond linear undo. Both bind to the kernel per-file history/undo model (14.19) and CRDT authority; Studio MUST NOT implement a private undo store.

---

### 2. Sections, Page Numbering, and Running Headers/Footers

[STU-LAY-014] **Sections** MUST partition a document for numbering: a named section restarts page numbering at a chosen start value, selects a number style (arabic, roman upper/lower, alphabetic, and locale-specific styles), optionally carries a section prefix and marker, and carries a per-section include-on-export flag. Section state is a record on the owning `StudioPageSpread` range, field-defined in 14.23.

[STU-LAY-015] Automatic page-number markers MUST resolve to the current page's section number wherever placed (parent page, running header, TOC). A "last page number" marker (section or document scope) MUST resolve for "page X of Y" constructs. Numbering markers are text-variable records ([STU-LAY-046]) resolved at composition time.

[STU-LAY-016] **Running headers/footers** MUST be content-derived: a header/footer field pulls the first or last on-page text carrying a chosen paragraph or character style (dictionary-style headers), with delete-trailing-punctuation and change-case options. Running headers resolve per page against the composed `StudioTextStory` and MUST update as content reflows.

---

### 3. Text Frames and Story Threading

[STU-LAY-017] Flowed copy is a `StudioTextStory` rendered through one or more threaded text frames (`StudioLayer` nodes of text kind). Text frame options MUST include: column count and gutter (fixed number or fixed width, with balance-columns), inset spacing per side, vertical justification (top/center/bottom/justify with paragraph-spacing limit), first-baseline offset control (and minimum), ignore-text-wrap, optional-frame auto-size behavior (off/height/width/height-and-width, with alignment and constraints), and vertical column rules (stroke, inset, offset, balance) between columns. A frame MAY carry a named-grid (CJK frame grid) format.

[STU-LAY-018] **Story threading** MUST let a single `StudioTextStory` flow through an ordered chain of frames across pages and spreads; cutting or reordering the thread re-flows the story. In/out ports on each frame MUST expose the thread chain, and a threads-view overlay MUST visualize the flow order.

[STU-LAY-019] **Linked stories** (place-and-link a child copy of a story) MUST show update state and support auto-update or warn-on-parent-change, so the same copy can appear in multiple layouts/alternate layouts. Threading state and link state are recorded on the `StudioTextStory` and surfaced in the linked-resource surface ([STU-LAY-028]).

[STU-LAY-020] Autoflow placement modes MUST be supported when placing a loaded text cursor, and are enumerable:

| Flow mode | Behavior |
|---|---|
| Manual | Places one frame's worth; cursor reloads with the remainder. |
| Semi-autoflow | Places a frame and reloads the cursor to continue (no page add). |
| Autoflow | Adds frames and pages until the story ends. |
| Fixed-page autoflow | Flows into existing pages only, without adding pages. |

**Smart text reflow** MUST automatically add or remove pages as a threaded story (scoped to primary text frames or to all frames) grows or shrinks. An overset-text condition MUST be detectable and is a preflight rule ([STU-LAY-057]).

[STU-LAY-021] **Span and split columns** MUST be a paragraph-level attribute: a paragraph may span all or N columns of its frame, or split into sub-columns, with before/after spacing and inside/outside gutter controls. This is layout flow behavior applied through the paragraph style/override system ([STU-LAY-035]).

[STU-LAY-022] Paragraph-level **layout-flow attributes** MUST be applied through the layout style system even though their glyph-level rendering is owned by 14.7: keep options (keep-with-previous, keep-with-next N lines, keep-lines-together all-or-start/end counts, start-paragraph anywhere/next-column/frame/page/odd/even), paragraph rules above/below (weight, type, color, tint, width, indents, offset), paragraph shading and paragraph borders (per-side widths, corner shapes, offsets, merge-consecutive, clip-to-frame, do-not-print), drop caps (line/character count, character style, scale-for-descenders), align-to-baseline-grid (all lines or first line only), and balance-ragged-lines. These attributes are stored on paragraph style records and paragraph overrides; their composition is executed by the typography engine (14.7).

[STU-LAY-023] **Text on a path** MUST bind a `StudioTextStory` to a `StudioVectorPath` (geometry owned by 14.5) with path-text options (alignment to path, spacing at curves, flip, and effect/rainbow modes); this is the single Studio text-on-path capability shared with the vector and typography domains.

[STU-LAY-024] **Text wrap** MUST be a per-object property with full options, enumerable by wrap shape:

| Wrap mode | Behavior |
|---|---|
| None | Object does not displace text. |
| Bounding box (Square) | Text wraps the rectangular bounds. |
| Object shape (Tight/Contour) | Text follows a contour source: object edges, alpha channel, embedded clipping path, or frame. |
| Jump object | Text jumps below the object within the column. |
| Jump to next column | Text jumps to the next column/frame. |

Wrap MUST support per-side offsets (top/bottom/left/right), invert, wrap-to side selection (both, left, right, largest area, toward/away from spine), include-inside-edges, and an independently editable wrap outline distinct from the object's geometry.

[STU-LAY-025] **Anchored/pinned objects** MUST attach a frame to a position in text so it travels with reflow. Positioning modes are enumerable: inline (on the baseline with a Y offset), above-line (with alignment left/center/right/toward-spine/away-from-spine/text-alignment and space before/after), and **custom** (positioned by reference point relative to the anchor marker, column, text frame, page margin, or page edge, with relative-to-spine mirroring across facing pages and keep-within-top/bottom boundaries). A pinning surface MUST manage inline and floating anchored objects.

[STU-LAY-026] A text-only editing surface (story editor) MUST present a `StudioTextStory` as linear text with a style column, depth indicator, overset marker, and inline display of notes, tracked changes, tables, and structure tags, editing the same story authority as the layout view. Placed-text import MUST honor saved option sets: word-processor/RTF import maps incoming styles ([STU-LAY-040]) or preserves them and carries footnotes, endnotes, and tables; plain-text import controls encoding, target dictionary, and carriage-return cleanup; spreadsheet import selects sheet, range, and formatting mode ([STU-LAY-035]).

[STU-LAY-027] A **find/change** surface MUST operate across modes over layout content: literal text (with metacharacter tokens and case/whole-word toggles), pattern/regex (capture groups, lookarounds, location tokens, formatting application), glyph (glyph ID / Unicode per font), object (search frames by object formatting and apply replacement attributes or object styles), and color (find a color usage and replace it). It MUST support saved/shared queries, predefined queries, and a search scope (all documents / document / story / to-end-of-story / selection) with include toggles for locked layers, hidden layers, parent pages, and notes.

---

### 4. Frames and Placed Graphics

[STU-LAY-028] Placed graphics live in graphic frames (`StudioLayer` of placed-asset kind). Picture frames of arbitrary shape (rectangle, ellipse, or any `StudioVectorPath`) MUST clip and fit placed content. **Place options** MUST be format-aware and are enumerable:

| Placed format class | Place options |
|---|---|
| Layered raster (e.g. PSD-class) | Preserve layers, layer comps, transparency, channels; per-place layer-visibility selection; color-mode support (RGB/CMYK/Lab/Grayscale). |
| Vector/PDF-class | Page selection; crop-to (bounding box, art, crop, trim, bleed, media); transparent background; multi-page load onto the cursor. |
| Flat raster (TIFF/JPEG/PNG-class) | Apply embedded clipping path; alpha-channel choice; per-image color profile and rendering intent. |
| Scalable vector (SVG-class) | Placed as scalable vector geometry. |
| Nested layout document | Page selection and layer-visibility overrides, tracked as a link. |
| Movie/sound (interactive) | Poster frame, controller skin, play-on-load, loop, navigation points (authoring here; export via 14.11). |

Multi-file placement and gridified placement MUST load multiple assets onto the cursor and place them in sequence or as a grid.

[STU-LAY-029] **Fitting** MUST support fit-content-to-frame, fit-frame-to-content, fit-content-proportionally, fill-frame-proportionally, center-content, and clear-fitting, plus a stored frame content-fit rule (scale-to-max-fit, scale-to-min-fit, stretch-to-fit, none) and an anchor point for placed content. Fitting MUST be expressible as an object-style property ([STU-LAY-038]).

[STU-LAY-030] **Linked resource management** MUST maintain, for every placed external asset, a link record carrying status (current/modified/missing), format, color space, actual and effective resolution (PPI), scale, layer, and file path. The linked-resource surface MUST support: relink, relink to folder, relink across file extensions, update link(s), edit-original / edit-with (round-trip to a source application and auto-update on save), go-to-link, reveal-in-file-manager, embed/unembed, and copy-links-to. Missing and modified links are preflight rules ([STU-LAY-057]). A collected-item placement surface (content conveyor) MUST hold items and item-sets for repeated placement with place-once / place-all / keep modes and a create-link toggle.

[STU-LAY-031] **Clipping and masking** of placed content MUST support applying an embedded clipping path, detecting edges, using an alpha channel, or a frame-as-mask, feeding both display clipping and the text-wrap contour source ([STU-LAY-024]).

[STU-LAY-032] **Object effects and transparency** apply through the shared `StudioEffectStack` (14.9). In layout, opacity, blend mode, and each effect MUST be independently targetable to Object, Fill, Stroke, or Text of a single frame, presented as an effects tree. The layout effect set is enumerable: drop shadow, inner shadow, outer glow, inner glow, bevel/emboss, satin, basic feather, directional feather, and gradient feather. Blend modes MUST include the standard sixteen (Normal, Multiply, Screen, Overlay, Soft Light, Hard Light, Color Dodge, Color Burn, Darken, Lighten, Difference, Exclusion, Hue, Saturation, Color, Luminosity) plus isolate-blending and knockout-group flags. Effect and blend math is owned by 14.9/14.8; this clause fixes the layout targeting model.

[STU-LAY-033] **Object/multi-state objects (MSO)** MUST be authorable in layout: convert a selection to a multi-state object, add/reorder/delete states, add objects to the visible state, paste into a state, reset all MSOs, and support hidden-until-triggered. Runtime state switching and interactive triggers are owned by 14.11; this clause owns the layout authoring of the MSO structure.

[STU-LAY-034] **QR codes** MUST be generatable as editable vector objects of type Web Hyperlink, Plain Text, Text Message, Email, or Contact Card, with color choice and post-generation editing; per-record QR generation is a data-merge capability ([STU-LAY-063]). **Reusable object libraries and snippets** MUST store, search, sort, and re-place objects (with per-item type/description metadata) and drag-out/export selections that re-place at original or cursor position. **Stroke options** for frames (weight, cap, join, miter, align inside/center/outside, dash/dotted/stripe stroke types, custom savable stroke styles, arrowheads with scale, gap color/tint) apply through the shared vector stroke model (14.5).

---

### 5. Tables

[STU-LAY-035] A table is a structured object flowed inside a `StudioTextStory` (its cell content is text, graphics, or nested tables). The table capability MUST include:

| Table area | Normative capability |
|---|---|
| Creation | Insert-table with body/header/footer row counts, column count, optional table style; draw-table tool; convert text↔table with selectable row/column separators. |
| Import | Spreadsheet import selecting sheet, view, and cell range as formatted/unformatted table or tabbed text; word-processor tables retaining or stripping formatting, optionally kept linked. |
| Table setup | Table border stroke, spacing before/after, stroke draw order, header/footer counts. |
| Alternating patterns | Alternating row strokes, column strokes, and fills (every-other, every-N, custom counts) with skip-first/skip-last. |
| Header/footer rows | Repeat per column, frame, or page with skip-first/last; convert body rows to/from header/footer. |
| Cell — text | Insets, vertical justification, first baseline, clip-to-cell, text rotation (0/90/180/270). |
| Cell — graphic | Convert cell to graphic cell holding a placed image with cell inset and fitting. |
| Cell — strokes/fills | Per-side stroke proxy (weight, type, color, tint, gap color, overprint) and cell fill. |
| Cell — rows/columns | Row height (at-least/exactly), column width, keep-with-next-row, start-row-on-next-frame/page. |
| Cell — diagonal lines | Diagonal/crossed lines with stroke settings and draw-in-front/behind. |
| Structure | Insert/delete/select rows and columns, merge/unmerge, split horizontally/vertically, distribute evenly, paste before/after, drag-duplicate, sort. |
| Flow | Break across threaded frames and pages with repeating headers/footers; go-to-row including header/footer sections. |
| Nesting & alignment | Nested tables (a table inside a cell); table alignment left/center/right within the frame column. |

Tables are formatted through table and cell styles ([STU-LAY-039]); the field-level table/cell model is owned by 14.23.

---

### 6. Layout Style System

[STU-LAY-036] Every named format is a record in the shared `StudioStyleRegistry`. The layout style types are enumerable and normative:

| Style type | Scope |
|---|---|
| Paragraph style | Full paragraph-level text formatting bundle (attribute payload owned by 14.7; layout-flow attributes per [STU-LAY-022]). |
| Character style | Partial-attribute run-level formatting; applies only explicitly set attributes over the paragraph style. |
| Object style | Frame/object formatting with per-category include toggles (stroke, fill, effects, text-frame options, text wrap, anchoring, fitting, export options, size/position); default text-frame and graphic-frame style slots. |
| Table style | Table-level formatting referencing up to five cell styles (header, footer, body, left column, right column) plus border and alternating patterns. |
| Cell style | Cell-level formatting (insets, strokes, fills, diagonal lines, optional paragraph style). |
| TOC style | Stored table-of-contents definitions ([STU-LAY-043]), reusable across documents and books. |

[STU-LAY-037] Style application mechanics MUST be preserved across all style types: **based-on** inheritance (a child stores only deltas and updates when its parent changes); **next-style** chaining for paragraph styles (with apply-style-then-next-style formatting a whole selection sequentially); **override** handling (override indicator, override highlighter, clear-overrides with character-vs-paragraph scoping, clear-on-apply toggle); **redefine style** from the current selection; **break link to style** (freeze current formatting as local values); and **style groups** (folders that organize styles and participate in load/import). Optional bundled style sets (coordinated style packs) and role-detecting auto-style application MAY be provided over this mechanism.

[STU-LAY-038] **Pattern (regex) styles** MUST apply a character style automatically to every regular-expression match inside paragraphs carrying a paragraph style. **Nested styles** MUST apply character styles through or up-to N occurrences of a delimiter (character, word, tab, or end-nested-style marker) inside a paragraph, plus per-line nested line styles. Both are paragraph-style-embedded rules resolved at composition time.

[STU-LAY-039] Object styles are the layout counterpart of text styles and MUST carry the full frame-formatting surface named in [STU-LAY-036]; applying an object style sets fitting ([STU-LAY-029]), text wrap ([STU-LAY-024]), anchoring ([STU-LAY-025]), effects targeting ([STU-LAY-032]), and export tagging in one operation. Table and cell styles MUST format tables and cells declaratively; a table style references its component cell styles, and applying it cascades through the table.

[STU-LAY-040] Cross-document style transfer MUST support loading selected styles from another document with per-style conflict resolution (use-incoming vs auto-rename) and MUST support incoming word-processor style mapping on import with saved presets.

[STU-LAY-041] **Export tag mapping** MUST let each text and object style declare an export tag and class (for reflowable/HTML export) plus a tagged-PDF role, editable singly or in bulk. This drives accessible/tagged output ([STU-LAY-061]) and reflowable export via 14.11; the mapping is stored on the style record in `StudioStyleRegistry`.

---

### 7. Long-Document System

[STU-LAY-042] **Books** MUST bind multiple chapter `StudioDocument`s into one publication unit that shares numbering and output. The book surface MUST support: add/remove/reorder documents, designate a style-source document, show per-document status, and open documents from the book list.

[STU-LAY-043] **Book synchronization** MUST propagate selected categories from the style-source document across book documents: styles (with smart style-group matching), swatches, text variables, numbered lists, cross-reference formats, conditional-text conditions, parent pages, and trap presets. **Book numbering** MUST continue page and chapter numbering across documents (continue, continue-on-next-odd/even with inserted blanks), update on demand, and support disabling automatic numbering. **Book-wide output** MUST print, export (PDF/reflowable), preflight, and package the whole book or selected documents. Numbered lists MUST be continuable across stories and across book documents for figure/table numbering.

[STU-LAY-044] **Tables of contents** MUST be style-driven: included paragraph styles map to TOC entry styles and levels, with page-number placement (before/after/none) and number character style, between-entry separators, alphabetical sort, run-in vs nested format, include-book-documents, include-hidden-layer text, numbered-paragraph handling, and generate-bookmarks. **Multiple TOCs per document** MUST be supported (contents, figures, tables, and secondary per-section TOCs), each with independent settings, and an update-all-TOCs command MUST refresh every TOC. TOC entries MUST be able to become live hyperlinks and bookmarks for interactive output (handoff to 14.11).

[STU-LAY-045] **Indexing** MUST build a topic/subtopic hierarchy of at least four levels through inserted index marks, with sort-by overrides, per-reference page-range scoping (to next style change, to next use of style, to end of story/document/section, or a custom paragraph count), and cross-reference forms (See, See also, See herein, See also herein, and custom). A generate-index operation MUST produce the formatted index story with title style, nested vs run-in format, section headings, include-book-documents, and include-hidden-entries. Capture shortcuts MUST index the selected word and proper names (last-name-first).

[STU-LAY-046] **Notes** MUST include footnotes, endnotes, and sidenotes. Footnote options MUST cover numbering (style, start, restart per page/spread/section, prefix/suffix, character/paragraph styles, separator) and layout (spacing, rule above, placement, span across columns). Endnote options MUST cover title, numbering style and restart scope, story-vs-document scope, endnote-frame placement, and styles. Sidenotes MUST anchor to the last text line or a frame edge, inside or outside the margin, with per-scope restart.

[STU-LAY-047] **Cross-references** MUST insert references to paragraphs or named text anchors using editable formats assembled from building blocks (page number, paragraph text/number, chapter number, file name) with a character style, and MUST flag stale/broken references in preflight ([STU-LAY-057]). **Named text anchors** MUST provide hyperlink and cross-reference destinations at exact text positions across documents.

[STU-LAY-048] **Text variables** MUST be resolvable placeholders whose single edit updates every instance. The variable set is enumerable and normative:

| Variable | Resolves to |
|---|---|
| Chapter Number | Document chapter number, with before/after text and numbering style. |
| Creation Date | Date/time the document was first saved. |
| Modification Date | Date/time last saved to disk. |
| Output Date | Date/time of the current print/export/package operation. |
| Custom Text | A reusable text placeholder. |
| File Name | Document file name, with include-path and include-extension options. |
| Image Name (Metadata Caption) | Metadata pulled from a nearby placed image, driving live captions. |
| Last Page Number | Section or document last page number for page-x-of-y. |
| Running Header | First or last on-page text carrying a chosen paragraph/character style ([STU-LAY-016]). |
| Document Fields | Author and other document metadata fields, plus user-defined custom variables. |

[STU-LAY-049] **Conditional text** MUST let named conditions (with indicator styling) hide or show tagged text ranges, and condition sets MUST capture reusable visibility combinations. Conditional visibility resolves before composition and interacts with smart text reflow ([STU-LAY-020]).

---

### 8. Grids, Guides, and Measurement

[STU-LAY-050] Grid and guide constructs are `StudioLayoutGrid` records. The document MUST support: a **baseline grid** (start position, relative-to, increment, view threshold) that paragraphs can snap to (all lines or first line only, per [STU-LAY-022]); a **document grid** (subdivisions and snap behavior); **column and margin guides** per page/spread; and **ruler guides** with per-guide color, view threshold, lock, copy/paste across pages, select-all, and delete-all-on-spread.

[STU-LAY-051] **Smart guides** MUST give dynamic alignment feedback against object edges and centers, plus smart-dimension and smart-spacing hints while dragging. Snapping MUST be toggleable per construct (grid, guides, objects).

[STU-LAY-052] **Layout grids and named grids** (character-count-based page grids and reusable frame-grid formats importable across documents) MUST be supported for grid-based composition. Named grids apply to text frames ([STU-LAY-017]).

[STU-LAY-053] **Measurement systems** MUST be selectable per axis and MUST include at least points, picas, inches, decimal inches, millimeters, centimeters, ciceros, agates, pixels, and a custom unit, cycled from the ruler or a units setting. Every length-bearing layout field MUST carry an explicit unit per the Studio unit law ([STU-DOC-003]); the document declares a default layout unit and mixed-unit fields are forbidden.

---

### 9. Prepress and Output

[STU-LAY-054] All render-to-output operations are `StudioExportRecipe` executions dispatched through the quiet/headless output path. Output MUST run without stealing focus or popping foreground windows and MUST be observable as a background task with progress and cancel, per the headless/quiet law ([STU-LAY-066], 14.20).

#### 9.1 Preflight

[STU-LAY-055] Studio MUST run a **live preflight** engine that continuously validates the active document against a selected profile, reporting an error count and per-error fix information, limitable to a page range, with a status indicator (clear/warning/error). Preflight MUST also be runnable on export.

[STU-LAY-056] **Preflight profiles** are editable rule sets carrying severity thresholds. Profiles MUST be creatable, exportable/importable as portable profile files, and embeddable in a document so recipients preflight against the same rules.

[STU-LAY-057] The preflight rule categories are enumerable and normative:

| Category | Representative rules |
|---|---|
| Links / resources | Missing links, modified links, low placed-image DPI, outdated linked resources, passthrough compatibility. |
| Color | Blend space, allowed plates, allowed color spaces, overprint, rich-black violations, CMY-in-gray, mismatched RGB spaces. |
| Ink | Ink density over threshold for fills/strokes/text. |
| Images & objects | Resolution, transparency/rasterization-forcing effects, minimum stroke weight, non-proportional scaling, hidden objects, bleed-zone hazards. |
| Text | Missing fonts, missing characters, overset text, spelling, text patterns (double spaces, straight quotes, double hyphens). |
| Document | Page size, page count, blank pages, bleed/slug. |
| Accessibility / data | Missing alt text, stale data-merge sources, out-of-date TOC, unnamed anchors, invalid hyperlinks, stale cross-references. |

Preflight rules MUST be surfaced to the model-steerable command surface as structured findings, not only to the operator UI.

#### 9.2 Package and Print

[STU-LAY-058] **Package** MUST collect the document, its linked resources, and its fonts (subject to font-licensing) into a portable folder with a report, for handoff to a printer or archive. Package MUST operate over a book as well as a single document.

[STU-LAY-059] The **print** pipeline MUST expose its full option surface, grouped and enumerable:

| Print panel | Options |
|---|---|
| General | Pages/spreads, copies, collation, layers-to-print. |
| Setup | Paper size/orientation, scale (with constrain), fit-to-page, position on media, thumbnails, tiling. |
| Marks & Bleed | Crop/bleed/registration marks, color bars, page-information marks, offset and weight, bleed values, include-slug. |
| Output | Composite (leave-unchanged/gray/RGB/CMYK) vs separations vs in-RIP separations, text-as-black, trapping mode, flip/negative, per-ink screening frequency/angle, ink-manager access. |
| Graphics | Image data sent (all/optimized/proxy/none), font-download policy, PostScript level. |
| Color Management | Printer profile and rendering intent. |
| Advanced | Flattener preset, OPI omission. |
| Summary | Full settings listing. |

**Print presets** MUST save complete print states as named, exportable/importable presets. Print-as-bitmap (rasterize page content at a chosen resolution) MUST be available for non-PostScript devices, and device-independent/device-specific PostScript and per-page EPS creation MUST be supported.

[STU-LAY-060] **Booklet / imposition** MUST arrange pages for folding: N-up and booklet imposition (2-up saddle-stitch, 2-up perfect-bound, and consecutive) with creep, bleed-between-pages, and signature controls, producing an imposed `StudioExportRecipe` output.

#### 9.3 PDF and Ink/Separation Output

[STU-LAY-061] **PDF (print) export** MUST expose its full panel surface, enumerable:

| PDF panel | Options |
|---|---|
| General | Preset and standard (PDF/X) choice, compatibility version, pages/spreads, view/layout open settings, layer export, include (bookmarks, hyperlinks, non-printing objects, visible guides/grids). |
| Compression | Per image class (color/grayscale/monochrome) downsampling method and threshold, codec (JPEG/ZIP/JPEG2000/CCITT), quality tier, crop-to-frames. |
| Marks & Bleeds | Printer marks, bleed/slug inclusion. |
| Output | Color conversion (none/convert-preserve-numbers), destination profile, PDF/X output intent, ink-manager access. |
| Advanced | Font-subsetting threshold, OPI omission, flattener preset for legacy compatibility. |
| Security | Open and permissions passwords, print/copy restrictions. |
| Summary | Full settings listing. |

**Tagged/accessible PDF** MUST be produced from style-to-tag mapping ([STU-LAY-041]), per-object alt text and roles, article/reading-order driven order, tab order, and document-title metadata.

[STU-LAY-062] Prepress control surfaces that drive the color pipeline (14.8) MUST be provided and are enumerable:

| Surface | Capability |
|---|---|
| PDF/X output intents | At least PDF/X-1a, PDF/X-3, PDF/X-4 with embedded output intents through built-in presets. |
| Ink manager | Per-ink spot-to-process conversion, all-spots-to-process, ink aliasing, use-standard-Lab-values-for-spots, per-ink trapping type (normal/transparent/opaque/opaque-ignore) with neutral density and sequence. |
| Overprint | Per-object overprint of fill, stroke, and gap, plus a black-overprint policy. |
| Trap presets | Trap width/black width, join/end styles, appearance thresholds, image-trap placement, assignable to page ranges. |
| Separations preview | Per-plate on/off preview, ink-limit view with a configurable total-ink threshold, per-ink coverage readouts. |
| Transparency flattener | Flattener presets plus a flattener preview highlighting rasterized/outlined regions. |

Ink/overprint/separation math and profiles are owned by 14.8; this clause fixes the prepress control surfaces that drive it.

[STU-LAY-063] **Raster export** (JPEG/PNG-class) MUST export selection, ranges, or all pages/spreads with quality, resolution, color space, anti-alias, bleed, and overlap. **Data merge** MUST bind a data source (delimited CSV/TSV, JSON array-of-objects, or spreadsheet, including image fields by path) to placeholder fields in the layout and generate merged output; it MUST support preview records, multiple records per page with arrangement (rows-first/columns-first), margins and spacing, a repeating grid-layout mode replicating a first-cell design to all cells, content-placement options (fitting, center, link images) and blank-line removal, record-range filtering, per-record QR-code fields, and direct-to-PDF merge without an intermediate document. Data merge is a `StudioExportRecipe` variant; the source is embeddable in the document.

#### 9.4 Structured Interchange

[STU-LAY-064] Round-trip interchange formats (layout markup export/open for cross-version exchange, tagged-text plain-format round-trip, and structured-XML tagging with a structure tree, tag mapping to/from styles, schema validation, and image-copying export) MUST be supported. The concrete interchange file formats and matrices are owned by 14.13; this clause fixes that layout content is representable in structured exchange form.

---

### 10. Interactive/EPUB Touchpoint and Collaboration Posture

[STU-LAY-065] Layout objects with interactive behavior — hyperlinks, bookmarks, buttons, multi-state objects ([STU-LAY-033]), placed media ([STU-LAY-028]), interactive TOCs ([STU-LAY-044]), and reflowable/fixed-layout EPUB export — are AUTHORED in this catalog but their runtime interaction model and reflowable/EPUB export pipeline are owned by 14.11. A layout object's interactive payload MUST be carried on its `StudioLayer` and handed to 14.11 for export; this catalog MUST NOT define a second interactive runtime.

[STU-LAY-066] Review and collaboration MUST be local-first. The native review surface — comments, annotations, and markup anchored to layout positions, and a document-states/history surface beyond linear undo (14.19) — is a first-class native Studio capability backed by kernel CRDT collaboration, requiring no external account or cloud service. Hosted/cloud collaboration capabilities MUST be treated as optional, adapter-backed rows, never as the primary path:

| Capability | Studio posture |
|---|---|
| Share-for-review hosted link (pin/highlight/strike/insert/reply comments) | Adapter-backed / optional; native local review surface is the primary path. |
| Cloud documents / autosave / cross-device version history | Adapter-backed / optional; local document + kernel history is the primary path. |
| Invite-to-edit / shared projects / asset-library sync | Adapter-backed / optional; kernel CRDT co-edit is the primary path. |
| Hosted-font activation and hosted custom-font rendering in reviews | Adapter-backed / optional; local font install is the primary path. |
| File-based assignment/check-in-check-out copy-editing workflow | Native (operates on shared local/network storage). |
| Generative-AI assists (text variation, text-to-image, expand-image) | Adapter-backed / optional; provider-dependent via the model-lane surface. |

Import of external review comments back into the layout, anchored to positions, MUST be supported through the interchange path (14.13).

---

### 11. Cross-Cutting Obligations

[STU-LAY-067] Every capability in this catalog MUST be exposed through BOTH the operator UI and the typed model-steerable command/MCP surface as two projections of one primitive per [STU-DOC-004], MUST satisfy the model-visibility/steerability and parallel-workflow requirements (14.16, 14.17), MUST obey the headless/quiet output law so preflight, package, print, PDF, and merge never steal focus or block on a foreground window (14.20), and MUST be represented in the dual-audience UserManual and the GUI/diagnostic and accessibility surfaces (14.22, 14.16). Model-authored layout mutations MUST pass the sandbox → validation → `PromotionGate` lifecycle ([STU-ARC-005]) via the propose-work system (14.18) before layout authority rows change; this obligation is stated once here and is not restated per clause.


## 14.7 Typography Engine

Typography is a shared Studio capability, not a per-domain feature: the raster document, the vector illustration document, and the page-layout document all place, shape, and format text through ONE text primitive (`StudioTextStory`), ONE shaping engine (`TextEngine`), and ONE type-style binding (`StudioTypeStyle`). This sub-section is the deduped normative Studio type/glyph engine; it collapses the five source suites' text engines (Photoshop type-engine, Illustrator Character/Paragraph/OpenType panels, InDesign text-and-typography, the Affinity Typography/Text-Styles surface, and Figma typography) into a single Studio surface per [STU-SECTION-003]. Where a source suite shipped a per-app text tool (point type, artistic text, frame text, story editor, and so on), Studio exposes exactly one primitive with a `text_kind` selector; the source product name is never a Studio tool, panel, or command name.

14.7 owns the **type/glyph engine**: the text model, the native shaping/line-breaking engine, character and paragraph formatting, OpenType and variable-font exposure, the glyph/special-character catalogs, and font management. It does NOT own the reusable named-style application lifecycle — that is `StudioTypeStyle`/`StudioStyleRegistry`, governed for layout by 14.6 and for design-system tokens by 14.10; 14.7 binds to those styles (group 8) but does not redefine their storage. Warp/envelope-on-text is a `StudioEffectStack` touchpoint owned by 14.5/14.9; 14.7 states the type-side contract only (group 10). All field-level type contracts, event variants, tables, and validation descriptors are canonical in 14.23; where this sub-section and 14.23 disagree, 14.23 wins.

[STU-TYP-OBLIG-001] GUI / Argus / UserManual obligation (stated once for 14.7). Every operator-facing typography surface enumerated in this sub-section MUST be reachable and drivable through the native operator UI and through the typed model-steerable command surface as two projections of the same primitive (14.16); MUST be observable and safely steerable headlessly through the Argus visual-debug path with stable `author_id` targeting under the quiet/headless law (14.20); and MUST be documented in the dual-audience UserManual so a no-context operator or model can locate and operate it (14.22). Every model-authored typography mutation follows the sandbox -> validation -> `PromotionGate` lifecycle of [STU-ARC-005]; no confidence level bypasses it.

---

### 1. Text Model — `StudioTextStory`

[STU-TYP-001] `StudioTextStory` (schema id `hsk.studio.text_story@1`) is the single Studio text primitive, carried by a `StudioLayer` of kind `text`. It stores the character stream, per-range character attributes, per-paragraph attributes, style bindings, and a `text_kind` discriminator. Studio does NOT ship separate primitives for the source suites' point/artistic/paragraph/frame text tools; they are `text_kind` values on one story.

[STU-TYP-002] `text_kind` MUST support at minimum:
- `point` — free-flowing text from an insertion point, no bounding box, grows with content (provenance: point text, artistic text).
- `area` — text reflowed inside a closed container (rectangular frame or arbitrary closed `StudioVectorPath`), with overflow/overset state (provenance: paragraph text, frame text, area type).
- `path` — text set along an open or closed `StudioVectorPath` (text-on-path); see [STU-TYP-004].
Conversion between `point` and `area` MUST be a non-destructive operation preserving all character/paragraph attributes and style bindings.

[STU-TYP-003] Auto-size behavior for `area`/`point` stories MUST support `auto_width` (grow horizontally), `auto_height` (wrap and grow vertically), and `fixed` (clip/overset), switching mode on manual resize, plus optional truncation with an ellipsis after a configurable maximum line count (provenance: Figma auto-resize + truncation; Photoshop dynamic text). Repositionable start/end points on the bounding box are preserved across resize.

[STU-TYP-004] Text-on-path: a `StudioTextStory` with `text_kind = path` binds to a `StudioVectorPath` and flows glyphs along it, with editable flow-extent handles (start/end), per-section baseline-distance offset, flow-direction control, side-flip, and reverse-path-direction (provenance: Affinity path-text green/orange handles + baseline control + Reverse Text Path; Photoshop type-on-path/inside-shape; Figma TextPath; Illustrator/InDesign type-on-a-path tools). Text remains editable while following path geometry. The path is the SAME `StudioVectorPath` primitive used by the vector domain (14.5); there is no text-only path type.

[STU-TYP-005] Threaded / linked text: multiple `area` stories MAY be threaded so a single logical story flows across containers, pages, and spreads with an overset indicator. Placement flow modes MUST cover manual flow, semi-autoflow (one container, cursor reloaded), autoflow (add containers/pages until story ends), and fixed-page autoflow (fill existing pages only) (provenance: InDesign thread-text + autoflow modes). Smart-reflow MAY add or remove `StudioPageSpread` pages as a threaded story grows or shrinks, scoped to primary containers or all containers (provenance: InDesign Smart Text Reflow). Linked stories (a child story that mirrors a parent story with update/warn-on-change status) are supported as a story-link relation (provenance: InDesign linked stories).

[STU-TYP-006] Story-editor view: Studio MUST provide a text-only editing projection of a `StudioTextStory` (linear character stream with style column, depth ruler, overset indicator, and inline display of notes/tracked-changes/table/tag markers) as an alternative to on-canvas editing (provenance: InDesign Story Editor). This is a view over the same primitive, not a separate document.

[STU-TYP-007] Placeholder + import: Studio MUST support inserting placeholder filler text into a story (provenance: Paste Lorem Ipsum) and placing external text (TXT/RTF/DOC/DOCX) into `area` stories with encoding, style-map/preserve, and cleanup options; full import-profile detail is governed by 14.13 and referenced here only as the text-ingest touchpoint.

[STU-TYP-050] History binding: every editing operation on a `StudioTextStory` (character/paragraph attribute change, style bind, glyph insert, reflow, thread edit) MUST emit a `StudioHistoryEntry` so type edits participate in the per-file undo/history surface (14.19) identically to raster and vector edits; text edits are not a separate undo silo (provenance: all suites treat type edits as first-class undoable operations).

[STU-TYP-051] Type measurement law: type sizes, leading, baseline shift, and tab positions are carried in points per [STU-DOC-003]; Figma density-independent-pixel inputs and CSS-style percentages are accepted at the API decode step and converted to the typed unit there, never stored as a mixed-unit field.

---

### 2. Native Shaping & Line-Breaking Engine (`TextEngine`)

[STU-TYP-008] Native-Rust shaping mandate. The Studio text-shaping engine MUST be a native Rust text-shaping/layout stack of the cosmic-text / rustybuzz / swash class (Unicode segmentation, complex-script shaping, bidi, font fallback, glyph rasterization), owned by the `studio-engine` crate behind the `TextEngine: Send + Sync` trait ([STU-ARC-002]). Studio MUST NOT depend on a platform text engine (DirectWrite, Core Text, Pango) or any external/subscription-gated shaping service at runtime ([STU-OVR-002]). Shaping, line-breaking, justification, and glyph selection are deterministic given identical inputs so that model-authored and operator-authored layout produce byte-identical results across hosts (a promotion-equivalence requirement of 14.24).

[STU-TYP-009] Unified complex-script shaping. ONE engine MUST handle Latin, Cyrillic, Greek, Arabic, Hebrew, Indic, Southeast-Asian, and CJK scripts with script-aware shaping, right-to-left and bidirectional runs, Kashida justification and diacritic positioning for Arabic/Hebrew, and CJK line-breaking (kinsoku) and inter-character spacing (mojikumi) rules (provenance: Photoshop unified text engine + international scripts; Illustrator RTL/Kashida + Asian composers; InDesign World-Ready + Japanese composers). There is no separate "world-ready" build; complex-script support is always present.

[STU-TYP-010] Composer (line-break engine) selection is per-paragraph. Studio MUST expose, under Studio-native names, at minimum:

| Studio composer (`composer`) | Behavior | Provenance |
|---|---|---|
| `paragraph` (default) | Evaluates all lines of the paragraph together, weighting break points by letter-spacing, word-spacing, and hyphenation penalties for even color | Adobe Paragraph / Every-line Composer |
| `single_line` | Composes one line at a time for traditional, predictable manual break control | Adobe Single-line Composer |
| `world_ready` | Complex-script (Arabic/Hebrew/Indic) shaping-aware paragraph/single-line composition | Adobe World-Ready Composers |
| `cjk` | CJK line-breaking with kinsoku and mojikumi | Japanese Paragraph / Single-line Composers |

[STU-TYP-011] Balance-ragged-lines: for non-justified (left/center/right) paragraphs, an optional balance mode MUST redistribute break points to even out ragged line lengths (headings, pull-quotes, centered paragraphs); it requires the `paragraph` composer (provenance: InDesign Balance Ragged Lines).

---

### 3. Character Controls

[STU-TYP-012] Character attributes are stored per contiguous character range on the `StudioTextStory`; mixed values across a selection are representable and reported as mixed. The normative character control set is:

| Control | Contract / units | Provenance (deduped) |
|---|---|---|
| Font family + style | Family and named style/instance; resolved via [STU-TYP-030] font management | all five suites |
| Size | Points (typographic unit per [STU-DOC-003]); Figma density-independent px accepted at API decode and converted | all five |
| Leading (line height) | Absolute (pt/px) or auto-leading percentage of size | PS/AI/ID/AF/Figma |
| Tracking | Range letter-spacing in 1/1000 em (numeric or %; may be negative) | all five |
| Kerning | `metrics` (font pair table), `optical` (computed from glyph shapes), or `manual` per-pair value | PS/AI/ID/AF |
| Horizontal / vertical scale | Per-glyph scale percentages | PS/AI/ID |
| Baseline shift | Raise/lower from baseline (manual super/subscript) | PS/AI/ID |
| Character rotation | Per-character rotation angle | AI/ID |
| Case | All-caps, small-caps (true OpenType or synthesized), title case, lowercase; non-destructive (stored characters unchanged) | PS/AI/ID/AF/Figma |
| Position | Superscript / subscript (synthesized) | PS/AI/ID/AF |
| Decoration | Underline (solid/dotted/wavy with thickness, offset, skip-ink) and strikethrough, each with custom options | PS/AI/ID/AF/Figma |
| Faux style | Faux bold, faux italic (synthesized when the font lacks the real cut) | PS/AI/ID |
| Language | Per-range language tag driving hyphenation, spelling, and shaping | AI/ID/AF |
| No-break | Prevents chosen characters/words from breaking across lines | PS/ID |
| Fractional widths | Sub-pixel fractional vs whole-pixel character advance | PS |
| Vertical trim | Leading-trim that removes space above cap height / below baseline so boxes hug glyphs | Figma |
| Anti-alias method | Per-story glyph edge rendering (none/sharp/crisp/strong/smooth) for raster output | PS/AI/AF |
| Fill / stroke color | Per-range color as a `StudioColorProfile`-tagged value or `StudioSwatch` reference (see 14.8); partial-selection coloring | PS/AI/ID/AF/Figma |
| Hyperlink | Per-range link to a URL or an in-document target (`StudioArtboard`/`StudioPageSpread`) | Figma; InDesign |

[STU-TYP-013] Change-across-selection: any character attribute (notably font family/style) MUST be applicable to every selected `StudioTextStory` in one operation (provenance: Photoshop change-font-on-multiple-layers).

[STU-TYP-014] Text-color law: text fill/stroke color is not an ad-hoc RGB value; it references the color pipeline of 14.8 and carries an explicit `StudioColorProfile` per [STU-DOC-003]. There is no implicit device color for type.

[STU-TYP-046] Kerning modes are per-range and MUST include `metrics` (from the font's pair-kerning table), `optical` (computed by the shaping engine from glyph shapes so unrelated or mixed fonts kern evenly), and `manual` (an explicit per-pair value in 1/1000 em). Metrics and optical are automatic; manual overrides at the caret pair (provenance: Illustrator/Photoshop/InDesign kerning modes, deduped).

[STU-TYP-047] Case and synthesized styles are non-destructive render transforms that MUST NOT alter the stored characters: all-caps, small-caps (true OpenType `smcp` when available, synthesized otherwise), title case, and lowercase; faux bold and faux italic synthesize a missing cut; superscript/subscript synthesize position when the font lacks true figures. True OpenType equivalents ([STU-TYP-017]) take precedence when the font declares them (provenance: PS/AI/ID/AF/Figma case + faux styles, deduped).

[STU-TYP-048] Text decoration is per-range: underline (solid / dotted / wavy) with configurable thickness, offset, and skip-ink, and strikethrough as an independent decoration, each with custom color/weight options (provenance: Figma underline styles + Photoshop/InDesign underline/strikethrough, deduped).

---

### 4. Paragraph Controls

[STU-TYP-015] Paragraph attributes are stored per paragraph. The normative paragraph control set is:

- **Alignment** — left / center / right horizontal alignment plus, for vertical/CJK text, top / center / bottom; vertical-in-box alignment (top/middle/bottom/justified) is a container property (provenance: PS/AI/ID/AF/Figma; Affinity text-frame vertical justification).
- **Justification (full H&J)** — justify-last-left / -center / -right and justify-all, with minimum / desired / maximum ranges for word spacing, letter spacing, and glyph scaling, plus auto-leading percentage and single-word-justification policy ([STU-TYP-016]).
- **Indents** — left, right, and first-line indent; last-line indent where applicable (provenance: PS/AI/ID/Figma).
- **Space** — space-before and space-after paragraph; "space-between-same-style" where supported (provenance: PS/AI/ID).
- **Drop caps** — drop-cap line count and character count, optional bound character style, align-left-edge, and scale-for-descenders (provenance: InDesign drop caps).
- **Hyphenation** — automatic hyphenation with minimum word length, letters-after-first, letters-before-last, consecutive-hyphen limit, hyphenation zone, better-spacing/fewer-hyphens bias, and toggles for capitalized words, last word, and across-column/frame hyphenation, resolved against the per-range language dictionary ([STU-TYP-031]) (provenance: PS/AI/ID/AF, deduped).
- **Composer** — per-paragraph composer selection per [STU-TYP-010].
- **Balance ragged lines** — per [STU-TYP-011].
- **Keep options** — keep-with-previous, keep-with-next N lines, keep-lines-together (all, or start/end line counts), widow/orphan control (prevent orphaned first / widowed last lines), and paragraph-start position (anywhere / next column / next frame / next page / next odd / next even) (provenance: InDesign Keep Options + Affinity Flow options, deduped).
- **Optical margin alignment** — hang punctuation (quotes, commas, periods, hyphens, dashes) and glyph edges outside the margin for optically aligned edges; also hanging list bullets and hanging opening quotes (provenance: Photoshop/InDesign Roman Hanging Punctuation; Figma hanging lists/quotes, deduped into one optical-margin control).
- **Paragraph shading** — fill the paragraph area with a `StudioColorProfile`/`StudioSwatch` color with per-side offsets, corner radii, clip-to-container, and do-not-print/export toggle (provenance: InDesign paragraph shading).
- **Paragraph border** — stroked border around the paragraph with per-side widths, corner shapes, offsets, and merge-consecutive-borders behavior (provenance: InDesign paragraph border).
- **Paragraph rules** — rule-above and rule-below with weight, stroke type, color/tint, overprint, width (column vs text), indents, and offset (provenance: InDesign paragraph rules).
- **Baseline-grid alignment** — snap all lines (or first line only) to a document or per-container baseline grid (provenance: InDesign/Affinity baseline grids).
- **Span / split columns** — a paragraph spans all or N container columns, or splits into sub-columns, with before/after spacing and gutter (provenance: InDesign Span/Split Columns).
- **Tabs** — left/center/right/decimal tab stops with leader characters and align-on character (provenance: Illustrator/InDesign Tabs).
- **Lists** — bulleted and numbered/ordered lists with nesting levels, list-spacing, restart/continue numbering, and markdown-style creation shortcuts (provenance: Photoshop/InDesign/Figma lists, deduped).
- **Nested / GREP / line styles** — apply a character style through or up to N delimiter occurrences (nested style), per-line nested-line styles, and regex-matched character styling (GREP style) inside paragraphs carrying a paragraph style (provenance: InDesign nested/GREP/line styles). These bind character styles defined per group 8.

[STU-TYP-016] Justification and single-word-justification values are stored on the paragraph and consumed deterministically by the selected composer ([STU-TYP-010]); glyph-scaling range MAY be omitted by a font/script that does not permit it (provenance: Affinity Publisher exposes word/letter spacing without glyph scaling).

The following clauses deepen individual paragraph controls from [STU-TYP-015] into independently citeable normative requirements; the table above is the overview and these are the binding detail.

[STU-TYP-034] Alignment MUST cover left/center/right for horizontal text, top/center/bottom for vertical/CJK text, and container-level vertical justification (top/middle/bottom/justified) as a property of the `area` container rather than the paragraph, so vertical justification survives paragraph edits (provenance: Affinity text-frame vertical justification).

[STU-TYP-035] Full H&J: the justification record MUST store minimum / desired / maximum percentages for word spacing, letter spacing, and glyph scaling; an auto-leading percentage; and a single-word-justification policy (full-justify / align-left / align-center / align-right for a lone word on a line). The composer consumes these ranges to place break points; identical inputs MUST yield identical breaks across hosts ([STU-TYP-008]).

[STU-TYP-036] Hyphenation is a per-paragraph record resolved against the per-range language dictionary ([STU-TYP-031]) with: minimum word length, letters-after-first, letters-before-last, consecutive-hyphen limit, hyphenation zone, a better-spacing/fewer-hyphens bias, and independent toggles for hyphenating capitalized words, the last word of a paragraph, and across a column/frame boundary (provenance: Photoshop/Illustrator/InDesign hyphenation dialogs, deduped into one record).

[STU-TYP-037] Keep / flow options MUST cover keep-with-previous, keep-with-next N lines, keep-lines-together (all lines, or start/end line counts), widow control (prevent widowed last line), orphan control (prevent orphaned first line), and paragraph-start position (anywhere / next column / next frame / next page / next odd page / next even page). Keep/flow interacts with threaded-story flow ([STU-TYP-005]) and inserted break characters ([STU-TYP-022]) (provenance: InDesign Keep Options + Affinity Flow options, deduped).

[STU-TYP-038] Optical margin alignment is ONE control: it MAY hang punctuation (quotes, commas, periods, hyphens, dashes) and glyph edges outside the margin, and MAY hang list bullets and opening quotation marks outside the text box, for optically aligned paragraph edges (provenance: Photoshop/InDesign Roman Hanging Punctuation + Figma hanging lists/quotes, collapsed into one primitive so the two source behaviors are not two Studio features).

[STU-TYP-039] Drop caps MUST store drop line count, drop character count, an optional bound character style ([STU-TYP-025]), align-left-edge, and scale-for-descenders (provenance: InDesign drop caps).

[STU-TYP-040] Paragraph decoration MUST provide three independent, composable records — paragraph shading (area fill with per-side offsets, corner radii, clip-to-container, do-not-print/export), paragraph border (stroked box with per-side widths, corner shapes, offsets, merge-consecutive-borders), and paragraph rules above/below (weight, stroke type, color/tint, overprint, column-vs-text width, indents, offset) — each carrying `StudioColorProfile`/`StudioSwatch` color references per [STU-TYP-014] (provenance: InDesign paragraph shading/border/rules).

[STU-TYP-041] Baseline-grid alignment MUST snap a paragraph's lines (all lines, or first line only) to a document-scoped or container-scoped baseline grid; the grid is a shared layout construct owned by 14.6 and referenced here (provenance: InDesign/Affinity baseline grids).

[STU-TYP-042] Span / split columns MUST let a paragraph span all or N columns of its `area` container, or split into sub-columns, with before/after spacing and inside/outside gutter (provenance: InDesign Span/Split Columns).

[STU-TYP-043] Tabs MUST provide left / center / right / decimal (align-on-character) tab stops with leader characters, stored per paragraph (provenance: Illustrator/InDesign Tabs, deduped).

[STU-TYP-044] Lists MUST provide bulleted and numbered/ordered lists with nesting levels, per-level bullet/number format, list spacing, restart/continue numbering, and creation shortcuts (provenance: Photoshop/InDesign/Figma lists, deduped into one list model).

[STU-TYP-045] Auto-styling inside paragraphs MUST support nested styles (apply a character style through or up to N delimiter occurrences), nested line styles (per-line character styling), and GREP styles (apply a character style to every regex match), all binding `StudioTypeStyle` character styles from group 8 (provenance: InDesign nested/GREP/line styles). The delimiter set includes characters, words, tab, and an explicit end-nested-style marker.

---

### 5. OpenType Feature Exposure

[STU-TYP-017] Studio MUST expose the font-declared OpenType feature set per character range; a feature is offered only when the active font declares it. Features map to standard OpenType feature tags and are stored on the character range, not baked into the character stream. The normative feature surface (deduped across all five suites) is:

| Studio feature group | Members / behavior | OpenType tags (illustrative) |
|---|---|---|
| Ligatures | Standard, discretionary, and contextual ligatures | `liga`, `dlig`, `clig`, `rlig` |
| Alternates | Stylistic alternates, contextual alternates, titling alternates, swash, ornaments | `salt`, `calt`, `titl`, `swsh`, `ornm` |
| Stylistic sets | Named/numbered stylistic set toggles | `ss01`–`ss20` |
| Figure style | Lining vs oldstyle numerals | `lnum`, `onum` |
| Figure width | Proportional vs tabular numerals | `pnum`, `tnum` |
| Fractions | Diagonal fractions and stacked fractions | `frac`, `afrc` |
| Ordinals | Ordinal indicators | `ordn` |
| Superscript / subscript | True OpenType figure/letter positions | `sups`, `subs` |
| Numerator / denominator | Fraction building blocks | `numr`, `dnom` |
| Slashed zero | Disambiguated zero | `zero` |
| Caps forms | Small caps, all-small-caps, petite caps, capital spacing/forms | `smcp`, `c2sc`, `pcap`, `cpsp`, `case` |
| Positional forms | Initial/medial/final/isolated forms for connected scripts | `init`, `medi`, `fina`, `isol` |
| Kerning / marks | OpenType kerning, mark positioning | `kern`, `mark`, `mkmk` |

[STU-TYP-049] Feature-application semantics: OpenType features are stored on the character range as feature-tag toggles/values and applied by the shaping engine at layout time; they never rewrite the underlying character stream (so text stays searchable and re-editable). A feature is inert when the active font does not declare it, and the UI/model surface MUST report which features the font actually provides so a no-context model does not request unavailable features (provenance: all suites gate feature exposure on font declaration).

[STU-TYP-018] Color-glyph fonts: Studio MUST render multi-color / gradient OpenType-SVG and `COLR`/`CPAL` color fonts, including composable emoji sequences, through the native shaping/raster path (provenance: Photoshop/Illustrator OpenType-SVG and emoji fonts). No platform emoji service is required.

---

### 6. Variable Fonts

[STU-TYP-019] Studio MUST support OpenType variable fonts: expose each font's registered and custom design axes (weight `wght`, width `wdth`, optical size `opsz`, slant `slnt`, italic `ital`, and arbitrary custom axes) as continuous per-range controls producing arbitrary instances, and expose the font's predefined named instances for one-click selection (provenance: Photoshop/Illustrator/Affinity/Figma variable-font axis sliders + named instances, deduped). Axis values are stored on the character range and MAY be bound to a `StudioVariable` (number) for token-driven typography (see group 8 and 14.10).

---

### 7. Glyphs, Special Characters, and White-space Catalogs

[STU-TYP-020] Glyph browser / insert-glyph: Studio MUST provide a glyph panel that browses every glyph in the active font, filters to alternates-for-selection and recently-used, inserts any glyph (including alternates unreachable by keyboard), and offers on-canvas alternate suggestions where the font provides them (provenance: Photoshop Glyphs panel + on-canvas alternates; InDesign/Illustrator Glyphs).

[STU-TYP-021] User glyph sets: the operator/model MAY define named glyph sets that store chosen glyphs (optionally remembering their font) for reuse across documents (provenance: InDesign glyph sets).

[STU-TYP-022] Special-character and break/white-space catalogs: Studio MUST expose an insertable, deduped catalog of special characters, break characters, and white-space characters (provenance: InDesign/Illustrator special-character and break menus). Break characters interact with the paragraph keep/flow model ([STU-TYP-037]) and threaded flow ([STU-TYP-005]). The normative catalog is:

| Catalog | Members (non-exhaustive, deduped) |
|---|---|
| Symbols / special | Em dash, en dash, ellipsis, bullet, copyright, registered, trademark, section, paragraph mark, single/double typographer quotes, degree, discretionary hyphen, non-breaking hyphen |
| Markers | Current page number, next/previous page number, section marker, footnote/endnote reference marker, anchored-object marker |
| Break characters | Forced line break, column break, frame break, page break, odd-page break, even-page break, paragraph return |
| White-space characters | Em space, en space, third/quarter/sixth space, thin space, hair space, figure space, punctuation space, flush space, non-breaking space (fixed and flexible width) |

[STU-TYP-023] Text variables / auto-inserted fields: Studio MUST support live text variables whose value is computed rather than typed, each with before/after text and type-specific options; editing the variable updates every inserted instance (provenance: InDesign text variables + running headers, deduped). The normative variable type set is:

| Variable type | Value | Options |
|---|---|---|
| Chapter number | Document chapter number | Numbering style |
| Creation date | Date/time first saved | Shared date-format tokens |
| Modification date | Date/time last saved | Date-format tokens |
| Output date | Date/time of print/export/package | Date-format tokens |
| Custom text | Reusable literal placeholder | Single edit updates all instances |
| File name | Document file name | Include path / include extension |
| Image name (metadata caption) | Metadata from a nearby placed image | Metadata field, live caption |
| Last page number | Section/document last page (page-x-of-y) | Numbering style, scope |
| Running header/footer | First/last on-page text carrying a chosen paragraph or character style | Delete-end-punctuation, change-case |

Cross-references, footnotes/endnotes, and conditional text are layout/long-document concerns bound here at the character level but governed in detail by 14.6.

[STU-TYP-024] Convert-to-outline: a `StudioTextStory` MAY be converted to editable vector geometry — a `StudioVectorNetwork`/`StudioVectorPath` of glyph outlines — for logo/path work; this is destructive to text editability and is the SAME vector primitive used by 14.5 (provenance: Photoshop convert-to-shape/work-path; Illustrator/InDesign create-outlines; Figma text-to-outlines, deduped). Filling glyph shapes with an image is expressed as a `StudioMask`/clipping relation, not a bespoke type feature (provenance: Photoshop fill-text-with-image).

---

### 8. Type-Style Binding (`StudioTypeStyle` / `StudioStyleRegistry`)

[STU-TYP-025] `StudioTypeStyle` (character-scope and paragraph-scope) is the canonical reusable named-text-style primitive. A `StudioTextStory` binds character ranges and paragraphs to `StudioTypeStyle` entries; the story stores the binding plus any local overrides, while the style definitions live in the `StudioStyleRegistry`. 14.7 owns the ENGINE that resolves a bound style into concrete character/paragraph attributes (groups 3–6); it does NOT own the style-registry storage, application UI, override/redefine lifecycle, based-on inheritance, or next-style cascade — those are governed by 14.6 (layout styles) and 14.10 (design-system tokens), which this group references rather than duplicates.

[STU-TYP-026] Style features that Studio MUST support through that binding (definitions owned upstream, engine support owned here): paragraph and character styles; based-on inheritance and next-style/following-paragraph cascade; override indicators, redefine-from-selection, and clear-overrides; nested styles, nested line styles, and GREP styles ([STU-TYP-015]); and object/frame styles for `area` containers (provenance: all five suites' style systems, deduped into `StudioTypeStyle` + `StudioStyleRegistry`).

[STU-TYP-027] Token-driven typography: number `StudioVariable`s MAY bind to size, leading, tracking, and paragraph spacing, and string `StudioVariable`s MAY bind to text content and font family, so a `StudioVariableCollection` can drive typography as design tokens (provenance: Figma typography variables). The variable system itself is governed by 14.10.

---

### 9. Font Management

[STU-TYP-028] Font sources: Studio resolves fonts from OS-installed fonts and from a Studio-managed local font library; there is no required cloud font service ([STU-OVR-002]). An org-shared / project-embedded font set MAY be supported as a local library, not a subscription dependency (provenance: Figma font sources, de-clouded).

[STU-TYP-029] Font picker: the picker MUST preview each family in its own glyphs, search by name, filter by source/classification, and support favorites and similar-font filtering (provenance: Photoshop/Figma font pickers, deduped).

[STU-TYP-030] Missing-font handling: on open, Studio MUST flag layers/stories using unavailable fonts and offer a bulk replace-font mapping from missing families/styles to available ones (provenance: Photoshop/Figma missing-font detection + replacement, deduped). Find/replace-font across the document (list all used fonts, replace one with another) is the same operation surfaced document-wide (provenance: Illustrator/InDesign Find/Replace Fonts).

[STU-TYP-031] Language + proofing: per-range language tags drive hyphenation and spelling dictionaries; Studio MUST support a Hunspell-class dictionary stack with per-language user dictionaries (added/removed word lists, import/export) and inline spell-check with suggestions (provenance: InDesign dictionary stack; Illustrator per-run dictionaries; Figma spell-check; Affinity per-language hyphenation, deduped). Match-font (analyze type in a raster region and suggest matching installed faces) is an optional deterministic assist; any generative font-suggestion path is an optional `StudioModelAdapter`, never a hard dependency (provenance: Photoshop Match Font).

[STU-TYP-032] Find/change over text: Studio MUST support text, regex (GREP), and glyph find/change modes with format-attribute criteria, scope selection (all documents / document / story / to-end-of-story / selection), inclusion toggles (locked/hidden layers, parent pages, footnotes), and saved reusable queries (provenance: InDesign Find/Change text/GREP/glyph modes + saved queries; Affinity regex find/replace, deduped). Object-mode and color-mode find/change are governed by 14.5/14.6/14.8 respectively.

---

### 10. Warp / Envelope Text Touchpoint

[STU-TYP-033] Warping or enveloping a `StudioTextStory` (arc, arch, bulge, wave, flag, fish, rise, and free-form envelope distortions, with orientation, bend, and horizontal/vertical distortion) is a `StudioEffectStack` operation owned by 14.5 (envelope/warp mesh) and 14.9 (live effects), applied non-destructively over a still-editable story (provenance: Photoshop Warp Text; Illustrator envelope distort). 14.7's contract is only that the story remains a live, editable, re-shapeable `StudioTextStory` under the warp and that the shaping engine re-runs on edit; 14.7 does NOT define warp styles or the distortion mesh.

---

## 14.8 Color Management & Pipeline

Color is a shared Studio pipeline, not a per-domain feature: every fill, stroke, swatch, gradient, text color, and adjustment across raster, vector, and layout resolves through ONE color model (`StudioColorProfile`), ONE swatch primitive (`StudioSwatch`), ONE gradient primitive (`StudioGradient`), and ONE pattern primitive (`StudioPattern`). This sub-section is the deduped normative Studio color surface; it collapses the five source suites' color systems (Photoshop channels-and-color/color-settings, Illustrator color-systems, InDesign color-and-output, the Affinity color-and-formats surface, and Figma fills/color) into one Studio pipeline per [STU-SECTION-003]. A source suite's color panel or command name is never a Studio name. Every color-bearing value in any Studio primitive carries an explicit `StudioColorProfile` reference per [STU-DOC-003]; there is no implicit device color anywhere in Studio. All field-level color contracts, event variants, tables, and validation descriptors are canonical in 14.23; where this sub-section and 14.23 disagree, 14.23 wins.

[STU-COL-OBLIG-001] GUI / Argus / UserManual obligation (stated once for 14.8). Every operator-facing color surface enumerated in this sub-section MUST be reachable and drivable through the native operator UI and the typed model-steerable command surface as two projections of the same primitive (14.16); MUST be observable and safely steerable headlessly through the Argus visual-debug path with stable `author_id` targeting under the quiet/headless law, including soft-proof/gamut/separation previews as inspectable state (14.20); and MUST be documented in the dual-audience UserManual (14.22). Every model-authored color mutation follows the sandbox -> validation -> `PromotionGate` lifecycle of [STU-ARC-005].

---

### 1. Color Model, Bit Depth, and Profiles — `StudioColorProfile`

[STU-COL-001] `StudioColorProfile` (schema id `hsk.studio.color_profile@1`) is the canonical color-space/profile primitive. It declares a color model, a bit depth, and an ICC (or OCIO) profile binding, and is referenced by every color-bearing Studio value. A `StudioDocument` carries a document color model, bit depth, and working-space profiles; individual placed assets MAY carry their own embedded profile.

[STU-COL-002] Color models MUST include, deduped across suites:

| Studio color model | Channels / role | Bit depths | Provenance |
|---|---|---|---|
| RGB | Additive three-channel working/display space | 8 / 16 / 32-bit | PS/AI/ID/AF/Figma |
| CMYK | Four-plate subtractive prepress space, device-dependent profile | 8 / 16-bit | PS/AI/ID/AF |
| Lab | Device-independent lightness + a/b opponent channels | 8 / 16-bit | PS/AI/ID/AF |
| Gray | Single-channel tonal | 8 / 16 / 32-bit | PS/AI/ID/AF |
| Bitmap | 1-bit black/white with conversion methods (threshold, pattern/diffusion dither, halftone screen, custom pattern) | 1-bit | PS |
| Indexed | Palette-limited with palette type, color count, forced colors, transparency, matte, dither, editable color table | 8-bit | PS |
| Duotone | Mono/duo/tri/quadtone inks with per-ink transfer curves and overprint colors | tonal | PS |
| Multichannel | Channel-per-plate special mode | per-plate | PS |

[STU-COL-003] Bit-depth law: documents MUST support 8-bit and 16-bit integer precision, and 32-bit floating-point (linear, unbounded HDR luminance) precision where the model allows (RGB/Gray), with tool/filter availability legitimately reduced at 32-bit (provenance: Photoshop 8/16/32-bit + HDR; Affinity 8/16/32-bit float, deduped).

[STU-COL-004] ICC color management: a `StudioColorProfile` binds an ICC profile. Studio MUST support document working spaces per model (RGB/CMYK/Gray/spot), color-management policies for profile mismatches on open/paste, embedding the profile on export, assign-profile (retag without changing values), and convert-to-profile (change values to a destination profile with a chosen rendering intent) (provenance: Photoshop Color Settings + Assign/Convert Profile + embed-on-save; Affinity ICC assign/convert; InDesign color-management pipeline, deduped). Wide-gamut working spaces are supported as ordinary ICC profiles.

[STU-COL-005] OCIO for 32-bit / scene-linear: for 32-bit float documents, Studio MUST support an OpenColorIO (OCIO v2-class) configuration path driving color-space transforms and a display transform (choice of ICC display transform, unmanaged linear light, or OCIO display transform), plus a non-destructive 32-bit preview control (exposure/gamma, EDR/HDR display) that changes screen presentation only and leaves document values unchanged (provenance: Affinity OCIO v2 pipeline + 32-bit Preview panel, deduped). Exporting an adjustment-stack grade as a 3D LUT is an optional color-pipeline export (provenance: Affinity export-3D-LUT).

[STU-COL-006] Native color engine: the color transform/CMM, ICC and OCIO handling, gamut mapping, and separation math are owned by the `studio-engine` crate behind the `ColorEngine: Send + Sync` trait ([STU-ARC-002]); Studio MUST NOT depend on a platform CMM or subscription color service ([STU-OVR-002]). Transforms are deterministic given identical profiles and intents (a promotion-equivalence requirement of 14.24).

[STU-COL-031] Mode conversion MUST be an explicit, reversible-where-lossless operation between the models of [STU-COL-002], carrying model-specific conversion options: Bitmap conversion exposes method (50% threshold, pattern dither, diffusion dither, halftone screen with frequency/angle/shape, custom pattern) and output resolution; Indexed conversion exposes palette type, color count, forced colors, transparency, matte, and dither with an editable color table; Duotone conversion exposes mono/duo/tri/quadtone ink slots with per-ink transfer curves and overprint colors; Grayscale conversion exposes the tonal mapping (provenance: Photoshop mode-conversion option dialogs, deduped). Lossy conversions warn before discarding channels.

[STU-COL-032] Every color-bearing value across all Studio primitives (`StudioSwatch`, `StudioGradient` stops, `StudioPattern` colors, text fill/stroke, paragraph decoration, adjustments) MUST carry a `StudioColorProfile` reference; the conversion boundary is the API decode step per [STU-DOC-003], and a value MUST NOT be stored as an untagged device triple.

---

### 2. Swatches — `StudioSwatch`

[STU-COL-007] `StudioSwatch` (schema id `hsk.studio.swatch@1`) is the canonical named-color primitive. A swatch declares a `swatch_kind`, a `StudioColorProfile`-tagged value, and membership in an optional swatch group. Swatch kinds MUST include, deduped:

| `swatch_kind` | Behavior | Provenance |
|---|---|---|
| `process` | Ordinary process color in the document model | all five suites |
| `global` | Process color that live-updates every object using it when edited | Illustrator global / Affinity global colors |
| `spot` | Named separation ink; first-class native primitive; optional Lab definition for accurate screen/proof simulation independent of CMYK; tintable | PS spot channels / AI spot / ID spot / AF |
| `mixed_ink` | One spot ink combined with process inks in one swatch | InDesign mixed ink |
| `mixed_ink_group` | Stepped series generated from base inks with editable steps | InDesign mixed ink groups |
| `tint` | Percentage tint of a base swatch that follows base edits | InDesign tints |
| reserved | Built-in None, Paper/Background, Registration, and Black swatches | AI/ID |

[STU-COL-008] Swatch groups + palette scope: swatches organize into named groups/color groups, and palettes are scoped to the document, the application, or the OS system palette (provenance: Illustrator color groups; Affinity document/application/system palettes, deduped). Swatch-panel operations MUST include create/duplicate/edit/delete with replace-on-delete, merge swatches, add unnamed/used colors, select-all-unused, and colorize-grayscale-image application (provenance: InDesign swatch operations + Illustrator, deduped).

[STU-COL-033] Global-swatch propagation: editing a `global` or `spot` swatch MUST live-update every object, text range, gradient stop, and pattern color referencing it, in one deterministic operation, without touching per-object overrides (provenance: Illustrator global swatches; Affinity global colors, deduped).

[STU-COL-034] Spot-swatch definition: a `spot` swatch MUST support an optional Lab definition used for accurate screen and soft-proof simulation independent of any CMYK alternate, plus a per-use tint percentage; the spot separates as its own plate regardless of the display alternate (provenance: Illustrator spot-Lab values; InDesign spot swatches, deduped).

[STU-COL-035] Tint and mixed-ink swatches MUST follow their base ink(s): a `tint` swatch is a percentage of a base swatch that tracks base edits; a `mixed_ink` swatch combines a spot with process inks; a `mixed_ink_group` generates a stepped, editable series from its base inks (provenance: InDesign tints + mixed-ink swatches/groups).

[STU-COL-009] Swatch interchange: Studio MUST import/export swatches and palettes as a portable exchange format (Adobe Swatch Exchange `.ase`-class) and a Studio-native palette file, without requiring an external app (provenance: Illustrator ASE; Affinity `.afpalette`/ASE, deduped). Import of swatches from another Studio document is supported.

---

### 3. Gradients and Patterns — `StudioGradient` / `StudioPattern`

[STU-COL-010] `StudioGradient` (schema id `hsk.studio.gradient@1`) is the canonical gradient primitive: a multi-stop model with per-stop color (`StudioColorProfile`/`StudioSwatch`) and opacity, editable midpoint/skew, and geometry. Geometries MUST include, deduped:

| Gradient geometry | Behavior | Provenance |
|---|---|---|
| `linear` | Axis with angle | all suites |
| `radial` | Center + radius, with aspect ratio | AI/ID/AF |
| `elliptical` / `conical` | Elliptical radial and angular sweep | Affinity |
| `freeform` | Free-placed color points or color lines inside a shape, per-point spread/opacity (`points` and `lines` modes) | Illustrator freeform |
| `mesh` | Editable mesh grid interpolating color between mesh points with per-point transparency; convertible from a gradient | Illustrator gradient mesh |
| `bitmap` | Image-based fill with editable placement (fill kind alongside gradients) | Affinity bitmap fill |

[STU-COL-011] Gradient controls MUST include reverse, per-gradient dither toggle, and perceptual-vs-linear interpolation to control banding and hue transitions, plus application of a gradient to a stroke (within / along / across the stroke) (provenance: Illustrator gradient dither/interpolation + gradient-on-stroke, deduped). A gradient MAY be stored as a named `StudioSwatch` (gradient swatch).

[STU-COL-036] Mesh and freeform gradients carry extra structure: a `mesh` gradient stores an editable grid of mesh points each with color and transparency, interpolating across the patch, and is convertible from a `linear`/`radial` gradient; a `freeform` gradient stores free-placed color points (`points` mode) or color lines (`lines` mode) each with spread and opacity, not bound to an axis (provenance: Illustrator gradient mesh + freeform gradients). These remain one `StudioGradient` primitive discriminated by geometry, not separate primitives.

[STU-COL-012] `StudioPattern` (schema id `hsk.studio.pattern@1`) is the canonical repeating-fill primitive: a tile with tile type (grid, brick-by-row, brick-by-column, hex-by-row, hex-by-column), brick/hex offset, tile size (with move-with-art), spacing, and overlap order; a pattern fill MAY be transformed (move/scale/rotate) independently of the object it fills (provenance: Illustrator Pattern Options + transform-pattern-independently). Patterns are stored as named swatches where the source suites did so.

---

### 4. Color-Management Operations (proof / separation / prepress)

[STU-COL-013] Studio MUST provide the following color-management and prepress operations as one deduped surface (member operations collapse per-suite variants into one Studio command each):

| Operation | Behavior | Provenance (deduped) |
|---|---|---|
| Soft-proof (proof setup / proof colors) | Simulate an output condition on screen using a device profile and rendering intent WITHOUT converting the document; includes color-blindness simulation profiles | PS/AI/ID proof setup + colors |
| Rendering intents | Perceptual, relative colorimetric, saturation, absolute colorimetric, with black-point compensation, selectable on convert and on proof | PS/ID color management |
| Gamut warning | Overlay a warning color on pixels/objects outside the target output gamut, with out-of-gamut picking | PS gamut warning + picker warnings |
| Separations preview | Per-plate on/off preview, ink-limit view with configurable total-ink threshold, and per-ink coverage readouts | InDesign Separations Preview |
| Overprint | Per-object overprint fill / stroke / gap (knock-out vs overprint) with overprint-preview, plus black-overprint policy | AI/ID/AF overprint |
| Ink manager | Per-ink spot-to-process conversion, all-spots-to-process, ink aliasing, use-standard-Lab-for-spots, and trap ink type (normal/transparent/opaque/opaque-ignore) with neutral density and sequence | InDesign Ink Manager |
| Trapping | Named trap presets (trap/black width, join/end styles, appearance thresholds, image placement) assigned to page ranges | InDesign trap presets; Illustrator/Photoshop trap |
| Transparency flattening preview | Highlight areas affected by flattening (rasterized regions, outlined text/strokes) per flattener preset | InDesign Flattener Preview |
| Appearance of black | Control whether 100% K displays/prints as rich vs accurate black on RGB devices, per screen and export/print | InDesign Appearance of Black |
| Colorize grayscale | Apply a swatch color to grayscale/bitmap image content in place | InDesign colorize grayscale |

The following clauses deepen individual operations from the [STU-COL-013] table into independently citeable normative requirements; the table is the overview and these are the binding detail.

[STU-COL-022] Soft-proof: Studio MUST simulate a chosen output condition on screen using a device `StudioColorProfile` and a rendering intent WITHOUT converting document values, toggleable on/off, and MUST include color-blindness simulation profiles as proof conditions. The proof state is inspectable via Argus per [STU-COL-OBLIG-001] (provenance: Photoshop/Illustrator/InDesign proof setup + proof colors, deduped).

[STU-COL-023] Rendering intents MUST include perceptual, relative colorimetric, saturation, and absolute colorimetric, with an independent black-point-compensation toggle, selectable both when converting to a profile ([STU-COL-004]) and when soft-proofing ([STU-COL-022]) (provenance: Photoshop/InDesign color management).

[STU-COL-024] Gamut warning MUST overlay a configurable warning color on pixels/objects outside the target output gamut and MUST offer a one-click in-gamut substitute in the picker ([STU-COL-016]) (provenance: Photoshop gamut warning + CMYK-equivalent picking).

[STU-COL-025] Separations preview MUST provide per-plate on/off preview, an ink-limit view with a configurable total-ink-coverage threshold, and per-ink percentage-coverage readouts at the cursor (provenance: InDesign Separations Preview).

[STU-COL-026] Overprint MUST be settable per object on fill, stroke, and gap (overprint vs knock-out), previewable in an overprint-preview mode, with a document black-overprint policy; overprint attributes feed the separations/ink pipeline (provenance: Illustrator/InDesign/Affinity overprint, deduped).

[STU-COL-027] Ink manager MUST provide per-ink spot-to-process conversion, all-spots-to-process, ink aliasing (map one ink to another), use-standard-Lab-values-for-spots, and per-ink trap type (normal / transparent / opaque / opaque-ignore) with neutral density and trapping sequence (provenance: InDesign Ink Manager). It operates on native `StudioSwatch(spot)` inks ([STU-COL-020]).

[STU-COL-028] Trapping MUST support named trap presets (trap width, black width, join and end styles, trap-appearance thresholds, image-trap placement) assignable to page ranges (provenance: InDesign trap presets; Illustrator/Photoshop trap, deduped).

[STU-COL-029] Transparency-flattening preview MUST highlight page areas affected by flattening (rasterized regions, outlined text/strokes) per a flattener preset, as inspectable state (provenance: InDesign Flattener Preview).

[STU-COL-030] Appearance-of-black MUST let the document control whether 100% K displays and prints as rich vs accurate black on RGB devices, independently for screen and for export/print (provenance: InDesign Appearance of Black).

[STU-COL-014] Channel operations (channels panel; alpha/spot channels; duplicate/split/merge channels; apply-image; calculations) are a raster/selection-mask concern deduped into the `StudioLayerGraph`/`StudioMask`/channel surface of 14.4; 14.8 owns only the spot-channel-as-`StudioSwatch(spot)` relation and the color-mode conversions of [STU-COL-002]. This cross-reference prevents a double feature between 14.4 and 14.8.

[STU-COL-015] Find/replace color: Studio MUST support finding usages of a color and replacing them with another across objects and text (provenance: InDesign Find/Change color mode), operating on `StudioSwatch`/`StudioColorProfile` references.

---

### 5. Color Picker / Chooser

[STU-COL-016] Studio MUST provide one color picker/chooser primitive that selects and enters color across models, deduped across suites:

| Picker facet | Contract | Provenance |
|---|---|---|
| Value models | HSB/HSL, RGB, RGB hex, CMYK, Lab, Gray entry | PS/AI/ID/AF |
| Value modes | 8-bit, 16-bit, or percentage value entry | Affinity |
| Wheel / sliders / spectrum | HSL color wheel, per-model sliders, spectrum bar | AI/AF |
| Warnings | Out-of-gamut and out-of-web warnings with one-click in-gamut/web substitute | PS/AI |
| Library access | Reach spot-color libraries and swatch groups from the picker | PS/AI/ID |
| Add-to-swatches | Promote a picked color to a `StudioSwatch` | ID/AI |

---

### 6. Color Harmony and Recolor

[STU-COL-017] Color harmony / guide: Studio MUST provide a deterministic harmony primitive that generates variation palettes from a base color using harmony rules (complementary, analogous, monochromatic, triad, tetrad, compound, shades, warm/cool, vivid/muted), with the option to constrain generated harmonies to a chosen swatch library (e.g. a spot book) (provenance: Illustrator Color Guide harmony rules + limit-to-library; Affinity color chords, deduped). Output is a set of `StudioSwatch`es.

[STU-COL-018] Recolor-artwork: Studio MUST provide a deterministic recolor primitive that remaps all colors in a selection — an assign mode (map current-to-new color rows with merge/exclude and prominence-weighted extraction) and an edit mode (move linked color handles on a wheel) — plus color reduction to N colors or to a library with preserve options for white/black/grays (provenance: Illustrator Recolor Artwork assign/edit + color-reduction, deduped). This primitive is fully deterministic and local.

[STU-COL-019] Generative recolor: any AI/generative recolor or palette-suggestion capability is an OPTIONAL `StudioModelAdapter` layered over the deterministic recolor/harmony primitives of [STU-COL-017]/[STU-COL-018]; it MUST NOT be a hard dependency and MUST NOT replace the deterministic primitives, which are first-class and always available offline ([STU-OVR-002]).

---

### 7. Spot-Color and Named-Library Posture (Pantone)

[STU-COL-020] Native spot color is first-class: the `StudioSwatch(spot)` primitive ([STU-COL-007]) — a named separation ink with optional Lab definition, tint control, and separation/preview behavior — is a native Studio primitive that ships and functions with no external library and no license ([STU-OVR-002]). Spot inks separate, proof, overprint, and appear in the Ink Manager/Separations Preview regardless of any branded library.

[STU-COL-021] Branded spot libraries (Pantone and other color-book libraries — DIC, HKS, Focoltone, Trumatch, ANPA, Toyo, and equivalents) are LICENSE-GATED EXTERNAL DATA and MUST be modeled as an OPTIONAL adapter (a `StudioModelAdapter`/import path that populates native `StudioSwatch(spot)` entries), never as a required bundled dependency or a Studio-owned name. Provenance note: current Photoshop no longer bundles Pantone books and routes them through a licensed plug-in, while Affinity bundles Pantone libraries directly — Studio's posture is the license-safe middle: native spot primitive always present, branded books loaded only through an optional adapter the operator supplies. Loading a branded book maps its inks onto native spot swatches with their library-defined Lab/CMYK values; absence of the adapter degrades only the branded-name lookup, never spot-color functionality.


## 14.9 Effects, Filters & Adjustments

Sub-section 14.9 is the normative Studio **effect/filter engine**: the single non-destructive model under which every filter, live effect, layer fx, and vector live-effect from the five source suites is applied, masked, re-edited, reordered, and rendered. It deduplicates four separately-branded systems — the smart-filter stack, the live-filter layer system, the vector live-effect (appearance) stack, and the layer-effects panel — into ONE Studio effect model built on the canonical primitives `StudioEffectStack`, `StudioLiveFilter`, `StudioAdjustment`, `StudioBlendMode`, and `StudioRenderHarness` (14.3, field-level authority in 14.23). Every deterministic filter primitive listed here is a native `studio-engine` compute pass run through the `RenderEngine` (wgpu/WGSL) on GPU where a row is marked GPU-required; generative/ML filters are an optional adapter lane per the local-first posture. This sub-section owns the filter/effect engine and any effect usable across raster AND vector layers; raster adjustment layers (Curves, Levels, HSL, and the rest of the tonal/color adjustment set) are owned by 14.4 and are referenced, not redefined, here.

The catalog tables below are the deduped normative Studio effect set. Each row is one Studio effect; where several source suites shipped the same capability under different product names, the row is the single collapsed Studio effect and the per-suite variants are stale provenance per [STU-SECTION-003]. A source suite's product or panel name (Filter Gallery, Blur Gallery, Neural Filters, Smart Filters, Vanishing Point brand strings, and so on) is never a Studio name; Studio ships the Handshake-native names in the tables. No feature in the source material is dropped: every distinct filter, effect, workspace, and layer-fx behavior is a row.

---

### 1. The Non-Destructive Effect Model

[STU-FX-001] Studio has exactly ONE non-destructive effect model. A filter or effect applied to any `StudioLayer` is recorded as a `StudioLiveFilter` node inside that layer's `StudioEffectStack` (schema `hsk.studio.effect_stack@1`), never as a destructive pixel/geometry mutation of the layer's source data. The layer's source raster tiles, vector network, or text story are preserved unchanged; the effect result is composed at render time by the `RenderEngine`. This unifies the smart-filter stack (Photoshop), the live-filter layer system (Affinity), the appearance/live-effect stack (Illustrator), and the layer-effect panel (Figma/InDesign) into a single model with no per-suite variant.

[STU-FX-002] A `StudioEffectStack` is an ordered list of `StudioLiveFilter` entries. Each entry carries: `filter_kind` (catalog key from the tables in this sub-section), typed parameter payload, an optional `StudioMask` reference (the entry's own effect mask), a `StudioBlendMode` + opacity for how the entry composes onto the pre-effect result, an `enabled` flag, and an `render_scope` (`layer` | `below` | `group`) selecting whether the effect reads only the layer's own render or the backdrop beneath it (backdrop-reading effects are marked in the catalog). Field-level definitions are canonical in 14.23.

[STU-FX-003] Every stack entry is re-editable, reorderable, maskable, and individually enable/disable-able after application with no quality loss, because the effect is re-evaluated from the preserved source on every change. Reordering two entries re-composes the stack in the new order deterministically. Deleting an entry restores the exact prior result. There is no "flatten to apply" requirement and no destructive bake unless the operator or a model explicitly requests an `EFFECT_BAKE` (rasterize/expand) command, which is itself a reversible history step (14.19).

[STU-FX-004] A `StudioLiveFilter` entry MAY host its own `StudioMask` (raster mask, vector clip, or gradient mask). The effect is applied only where the mask is opaque, feathered by the mask's own falloff. This is the single deduped realization of smart-filter masks, live-filter-layer masks, and effect-region masks; there is no separate masking mechanism per effect family.

[STU-FX-005] The same `StudioLiveFilter` kind is applicable from any domain in which it is defined as valid: a blur, a shadow, a distort, or a stylize effect is the SAME primitive whether invoked on a raster layer, a vector layer, a text layer, or a group. The catalog marks each row's valid layer targets (`R` raster, `V` vector, `T` text, `G` group). Where an effect is defined for vector targets, Studio evaluates it on the live geometry (a vector live effect) rather than pre-rasterizing, unless the row is raster-only.

[STU-FX-006] An effect stack applied to a `group` layer or a `StudioComponent` is inherited by the group's rendered result as a single composite, matching the "effect on a frame/group" semantics of the source suites. Effect stacks are also savable as reusable `StudioStyleRegistry` entries (effect styles / graphic styles) per group 19.

[STU-FX-006a] Every deterministic filter/effect MUST evaluate correctly across the Studio raster bit depths (8/16/32-bit-per-channel per 14.4/14.12) and MUST evaluate in the layer's declared `StudioColorProfile` working space, not an implicit device space ([STU-DOC-003]); an effect that clips or is undefined outside 8-bit MUST declare its supported depths in its descriptor rather than silently degrading. Color-valued and luminance-driven filters read and write through the color-managed pipeline (14.8).

[STU-FX-006b] Where a source suite capped the instance count of a stackable effect (for example up to eight drop shadows or two noise effects per layer), Studio treats that cap as a non-normative source detail: the `StudioEffectStack` imposes no fixed per-kind instance limit beyond a document-level performance budget surfaced through the render harness. Import/round-trip (14.13) preserves source caps for fidelity but Studio authoring is not bound by them.

---

### 2. Effect Application, Masking, Blending, and Ordering

[STU-FX-007] Effect application is a model-steerable typed command (`STUDIO_EFFECT_APPLY`, `STUDIO_EFFECT_UPDATE`, `STUDIO_EFFECT_REORDER`, `STUDIO_EFFECT_TOGGLE`, `STUDIO_EFFECT_REMOVE`, `STUDIO_EFFECT_MASK_EDIT`, `STUDIO_EFFECT_BAKE`) on `event_family = studio.effect`. Every model-authored effect mutation traverses the sandbox -> `StudioValidationDescriptor` -> `PromotionGate` lifecycle ([STU-ARC-005]); an effect change is never written directly to authority.

[STU-FX-008] Each stack entry composes onto the accumulated result using its `StudioBlendMode` + opacity, exactly as a layer blends, so a filter can be blended (e.g. a sharpen pass at reduced opacity, an overlay-blended high-pass) without a separate "fade" command. This deduplicates the post-filter fade/blend-options control into the standard blend surface (14.8 owns the `StudioBlendMode` set; 14.9 consumes it).

[STU-FX-009] A "repeat last effect" affordance re-applies the most recent `StudioLiveFilter` kind + parameters to the current selection as a new stack entry; a "repeat with new settings" variant opens the parameter surface first. This is one deduped command, not two menu items.

[STU-FX-010] Effect parameters are typed and unit-bearing per [STU-DOC-003]: radii and offsets in document pixels (or the document unit for vector geometry), angles in degrees, and color-valued parameters carry an explicit `StudioColorProfile`. Mixed-unit or implicit-device-color effect parameters are forbidden.

[STU-FX-011] Effect evaluation is deterministic: given the same source data, parameters, mask, seed, and `StudioColorProfile`, the `RenderEngine` MUST produce byte-stable output for a given backend and promotion-equivalent output across backends (group 18, 14.24). Filters with a random component (noise, clouds, spatter, and the like) carry an explicit integer `seed` parameter so their output is reproducible.

[STU-FX-011a] Effect application on a `placed_asset`/smart-object layer applies to the layer's rendered proxy non-destructively; editing the placed source re-flows the same effect stack over the updated content. There is no separate "smart filter" concept — a filter on any layer is already the non-destructive stack of [STU-FX-001]; the "convert for filters" step of the source suites collapses to a no-op because Studio layers are filter-hosting by default.

[STU-FX-011b] Time-varying effect parameters (animatable blur amount, distortion, glow, and the like) are keyable by the `StudioMotionTimeline` (14.11): an effect parameter exposed to motion is the same typed field, sampled per frame, and the effect re-evaluates deterministically per timeline sample. 14.9 defines the effect and its parameters; 14.11 owns the timeline that animates them.

[STU-FX-012] A `StudioAdjustment` (14.4-owned tonal/color adjustment) and a `StudioLiveFilter` (14.9-owned filter/effect) coexist in the same layer ordering surface: an adjustment layer and a live-filter entry are both non-destructive nodes composed in stack order. 14.9 does not redefine adjustments; it guarantees they interleave with filters in one order (group 17).

[STU-FX-012a] An effect MAY be scoped to the active `StudioSelectionSet` at apply time: with a live selection, the applied `StudioLiveFilter` records the selection as its effect region (equivalent to a captured mask) so the effect touches only the selected pixels/geometry, re-editable afterward as the entry's `StudioMask`. With no selection, the effect applies to the whole layer render. This is one deduped selection-scoping rule; there is no separate "filter selection" versus "filter layer" mode.

[STU-FX-012b] Effect evaluation MUST support tiled, progressive, region-of-interest re-rendering: editing one stack entry's parameters re-evaluates only the affected tiles and only the entries at or above the changed entry, so large-document interactive preview stays bounded. The interactive preview is the same `RenderEngine` pass as the committed result at reduced resolution/quality, never a separate approximate preview path that could diverge from the final output.

[STU-FX-012c] Every effect mutation ([STU-FX-007]) is a reversible `StudioHistoryEntry` (14.19): apply, update, reorder, toggle, mask-edit, remove, and bake each produce one undoable step, and undo/redo restores the exact prior stack state because the source data is preserved ([STU-FX-001]). A destructive `EFFECT_BAKE` is itself undoable and records the pre-bake stack for restoration.

---

### 3. Blur Family

[STU-FX-013] The blur family is a set of `StudioLiveFilter` kinds sharing a common blur-core in `studio-engine`. Field/iris/tilt-shift/path/spin blurs expose on-canvas pin/handle controls and are GPU-accelerated; all blur kinds are re-editable stack entries. Bokeh/motion/noise styling controls (light bokeh, bokeh color, light range, strobe, restored grain) are shared parameters on the pin-based blurs, recorded as one styling row.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Average (fill-with-mean) | R,G | No | Replaces the selection with its single average color; used as a color-sampling and flat-fill primitive. |
| Basic Blur / Blur More | R | No | Fixed-strength smoothing of color transitions; the "more" step is ~3-4x stronger. Parameterless quick softening. |
| Box Blur | R | Opt | Neighborhood-average blur with adjustable radius. |
| Gaussian Blur | R,V,T,G | Opt | Weighted bell-curve blur by adjustable radius; the canonical uniform layer blur. Dedup of the uniform layer-blur effect and the standalone gaussian filter. |
| Median Blur | R | Opt | Replaces each pixel with the median of its radius neighborhood; edge-preserving speckle removal. |
| Bilateral Blur | R | Opt | Edge-aware blur that smooths within a range threshold while preserving edges. |
| Lens Blur | R | Req | Depth-of-field simulation using an optional depth map, iris shape (blades/rotation/curvature), and specular-highlight controls. |
| Depth-of-Field Blur | R | Req | Interactive shallow-DoF with an in-focus region (radial/linear) and falloff, distinct from map-driven lens blur. |
| Motion Blur | R | Opt | Directional blur, angle -360..+360 deg, intensity 1-999, simulating movement during exposure. |
| Radial Blur (spin/zoom) | R | Opt | Spin or zoom blur around a draggable center with draft/good/best quality. |
| Spin Blur | R | Req | Rotational blur in degrees around one or more movable center points with size/shape handles (compositable). |
| Shape Blur | R | Opt | Blurs using a selectable custom-shape kernel with radius. |
| Smart/Selective Blur | R | Opt | Precision edge-respecting blur with radius, threshold, quality, and normal/edge-only/overlay-edge modes. |
| Surface Blur | R | Opt | Edge-preserving blur with radius + threshold, used for noise/grain reduction while keeping edges. |
| Field Blur | R | Req | Blur gradient built from multiple on-image pins, each with its own blur amount. |
| Iris Blur | R | Req | Elliptical sharp-focus region with feathered blur falloff outside, via on-image ellipse handles. |
| Tilt-Shift Blur | R | Req | Band of sharpness fading to blur at both edges with distortion control (miniature-photography look). |
| Path Blur | R | Req | Motion blur along editable multi-point paths with speed, taper, and end-point velocity; multiple paths composite. |
| Background Blur (backdrop) | R,V,G | Req | Blurs content BEHIND the layer (`render_scope = below`); the layer fill must be semi-transparent for the effect to read. |
| Progressive Blur | R,V,G | Req | Layer or backdrop blur with a gradient variant: controllable size, direction, and start/end intensity forming a blur ramp. |
| Diffuse Glow | R | Opt | See-through white-noise glow/bloom fading from highlights/center like a diffusion filter. |
| Bokeh / Motion / Noise styling | R | Req | Shared pin-blur styling: light bokeh, bokeh color, light range, strobe motion effects, and restored-grain matching. |

---

### 4. Distort Family

[STU-FX-014] Distort effects warp the layer geometry non-destructively. Raster distorts resample source tiles; where the same distort is defined for vector targets it warps the live path. Displacement-map and mesh distorts reference an auxiliary map/mesh stored with the stack entry.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Displace | R | Opt | Shifts pixels by the luminance of a separate displacement map, with stretch/tile fitting and edge handling (wrap/repeat/background). |
| Pinch / Punch | R | Opt | Squeezes a region inward (pinch) or outward (punch), -100%..+100%. |
| Polar Coordinates | R | Opt | Converts between rectangular and polar coordinate mappings of the region. |
| Ripple | R | Opt | Undulating pond-ripple pattern with ripple count and size. |
| Ocean Ripple | R | Opt | Randomly-spaced underwater-look surface ripples. |
| Shear | R | Opt | Distorts along a user-drawn curve with wrap or repeat-edge handling. |
| Spherize / Spherical | R | Opt | Wraps the region around a spherical curve for a 3D bulge/dent. |
| Twirl | R | Opt | Rotates the region more sharply at the center by a specified angle. |
| Wave | R | Opt | Parametric wave distortion: generator count, wavelength, amplitude, sine/triangle/square type, randomize. |
| ZigZag | R | Opt | Radial distortion with ridges and pond-ripple / out-from-center / around-center modes. |
| Glass | R | Opt | Refracts through selectable or custom glass surfaces with scaling, distortion, and smoothness. |
| Lens Distortion | R | Opt | Live barrel/pincushion distortion create or correct as a re-editable filter (distinct from the correction workspace in group 12). |
| Perspective Warp | R | Opt | Live perspective-plane distortion of the layer render. |
| Mesh Warp | R,V | Opt | Grid-mesh warp of the render/geometry, re-editable as a filter entry. |
| Texture (surface displace) | R,V,G | Req | Distorts the render with x/y size, radius (spread beyond bounds), and clip-to-shape toggle (grain/organic surface texture). |
| Glass (refractive material) | R,V,G | Req | Simulates refractive material with light angle, light intensity, refraction, depth, dispersion, frost, and splay parameters (glassmorphism). |

---

### 5. Noise Family

[STU-FX-015] Noise effects add or remove stochastic detail. Additive-noise kinds carry a `seed` for reproducibility ([STU-FX-011]) and a color-mode (mono/duo/multi). Noise reduction and artifact-removal kinds are detail-preserving.

[STU-FX-015a] The deterministic denoise kind (Reduce Noise / Denoise) is the native-first alternative to the adapter-backed ML denoise ([STU-FX-034]); Studio prefers the native pass when parity is achievable and reaches for the adapter lane only when the operator/model explicitly selects the ML variant.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Add Noise | R,V,G | Opt | Adds uniform or gaussian random pixels; mono/duo/multi color mode, x/y noise size, density; monochromatic option. Dedup of the additive-noise filter and the layer noise effect. |
| Despeckle | R | Opt | Blurs everything except detected edges to remove noise while keeping detail. |
| Dust & Scratches | R | Opt | Removes small defects by replacing dissimilar pixels within radius/threshold bounds. |
| Reduce Noise / Denoise | R | Opt | Overall and per-channel luminance/color noise reduction with edge preservation and JPEG-artifact-removal option. |
| Defringe | R | Opt | Removes chromatic fringing along high-contrast edges. |
| Diffuse (pixel scatter) | R | Opt | Shuffles/scatters pixels with normal, darken-only, lighten-only, or anisotropic modes. |

---

### 6. Pixelate Family

[STU-FX-016] Pixelate effects cluster pixels into cells/dots/blocks. Halftone/screen kinds carry per-channel angle and dot parameters. Cell-mosaic kinds (mosaic, crystallize, Voronoi) share a cell-generation core.

[STU-FX-016a] Cell-mosaic effects reuse the `studio-engine` tessellation core shared with the vector engine (14.5) rather than a private per-filter implementation, so cell generation is consistent and testable in one place; the deduped Voronoi and crystallize kinds differ only in cell-fill rule, not in cell topology.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Color Halftone | R | Opt | Simulates enlarged per-channel halftone screens; dot radius 4-127, per-channel screen angles. |
| Halftone Screen | R,V | Opt | Dot / line / circular halftone screening with continuous-tone option. Dedup of the halftone pattern filter and the halftone live filter. |
| Crystallize | R | Opt | Clumps pixels into solid-color polygonal cells. |
| Facet | R | Opt | Clumps similar-color pixels into blocks for a hand-painted look. |
| Fragment | R | Opt | Averages four offset copies of the region's pixels. |
| Mezzotint | R | Opt | Random black/white or saturated dot/line/stroke patterns from a type menu. |
| Mosaic | R | Opt | Clumps pixels into uniform square color blocks. |
| Pointillize | R | Opt | Random pointillist dots with the background color as canvas between dots. |
| Voronoi | R | Opt | Voronoi cell-mosaic tessellation. |

---

### 7. Render & Generative-Pattern Family (GPU)

[STU-FX-017] Render/generate effects synthesize new pixel data (clouds, fibers, flares, lights, scripted patterns). They generate procedurally from parameters + `seed` and may replace or blend with existing layer data (marked). Lighting, flame, picture-frame, tree, and oil-paint style renders REQUIRE the GPU `RenderEngine` and degrade gracefully to an operator-visible "GPU required" state when no supported backend is present ([STU-FX-036]).

[STU-FX-018] GPU-required render effects MUST declare their GPU dependency in the `StudioLiveFilter` descriptor so a headless/quiet run (14.20) and a no-GPU environment surface a determinate `EFFECT_GPU_UNAVAILABLE` result rather than silently producing empty output.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Clouds | R | Opt | Random cloud pattern between two colors, replacing layer data (seeded). |
| Difference Clouds | R | Opt | Blends generated clouds with existing pixels in Difference mode; marble-like on repeat (seeded). |
| Fibers | R | Opt | Woven-fiber texture from two colors with variance, strength, randomize. |
| Lens Flare | R | Opt | Simulated lens-refraction flare with placeable center and lens-type presets. |
| Lighting Effects | R | Req | GPU lighting workspace: point/spot/infinite lights, per-light properties, and grayscale texture channels as bump maps. Dedup of the lighting render filter and the 3D-style lighting live filter. |
| Flame | R | Req | Parameterized realistic flames rendered along a selected path onto a pixel layer; requires GPU + an active work path. |
| Picture Frame | R | Req | Scripted decorative frame designs drawn around the canvas; requires GPU + a pixel layer. |
| Tree | R | Req | Parameterized tree species with leaf/branch/light settings; requires GPU + a pixel layer. |
| Procedural Texture (formula) | R | Opt | User-authored mathematical pixel-math formulas generate/transform pixels (scriptable generator surface, deterministic per inputs). |

---

### 8. Sharpen & Detail Family

[STU-FX-019] Sharpen/detail effects increase local/edge contrast. High-pass is one deduped kind used both as a sharpen row here and as the overlay-sharpening detail extractor; it is not duplicated between families.

[STU-FX-019a] The deterministic super-resolution / detail-synthesis capability is split by posture: bicubic/Lanczos and edge-preserving upscale are native resampling owned by 14.4/14.13, while ML detail synthesis is the adapter-backed super-resolution row ([STU-FX-034]); 14.9 sharpen kinds do not add pixels, only redistribute existing contrast.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Sharpen / Sharpen More | R | No | Fixed-strength clarity boost by increasing adjacent-pixel contrast; the "more" step is stronger. |
| Sharpen Edges | R | No | Sharpens only detected edges, preserving smooth areas; parameterless. |
| Unsharp Mask | R | Opt | Edge-contrast sharpening with amount, radius, threshold producing light/dark edge lines. |
| Smart Sharpen | R | Opt | Adaptive sharpening with algorithm choice, shadow/highlight fade, noise-reduction slider, per-channel and CMYK support. |
| Clarity | R | Opt | Local-contrast (midtone) enhancement. |
| High Pass | R,V | Opt | Keeps edge detail within a radius and suppresses low-frequency content (inverse of gaussian); the overlay-sharpen detail extractor. |

---

### 9. Stylize Family

[STU-FX-020] Stylize effects re-render the image with edge, relief, extrusion, or exposure transforms. Oil-paint is GPU-required. Glowing-edges is a gallery-stackable stylize.

[STU-FX-020a] The raster Stylize kinds here (edge/relief/exposure) are distinct from the vector Stylize layer-fx of group 13 (shadow/glow/round-corners/scribble) despite the shared source-menu label; they are separate `filter_kind` families and MUST NOT be conflated. Emboss and Extrude produce raster relief; the vector 3D-and-materials effects of group 14 produce true parametric 3D geometry, and the two are not substitutes.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Emboss | R | Opt | Raised/stamped gray relief with angle, height, and color-amount. |
| Extrude | R | Opt | 3D block or pyramid extrusion with size, depth, random/level-based depth, solid faces, mask-incomplete option. |
| Find Edges | R | No | Outlines significant transitions with dark lines on white; parameterless. |
| Glowing Edges | R | Opt | Neon-like glow on detected color edges; cumulatively stackable in the effect gallery. |
| Oil Paint | R | Req | GPU painterly effect: stylization, cleanliness, scale, bristle detail, lighting controls. |
| Solarize | R | No | Blends negative and positive image like brief exposure during development. |
| Tiles | R | Opt | Offsets the image into tiles with a fill choice for the gaps. |
| Trace Contour | R | Opt | Outlines brightness-level transitions per channel at a chosen tonal level with upper/lower edge option. |
| Wind | R | Opt | Horizontal windblown streaks with Wind, Blast, and Stagger methods. |

---

### 10. Artistic / Effect-Gallery Family

[STU-FX-021] The artistic families (Artistic, Brush Strokes, Sketch, Texture) are stackable painterly/media-simulation filters. They apply through the **Effect Gallery** browser (the deduped native realization of the filter-gallery / effect-gallery browser): a single surface that applies one or more artistic effects cumulatively as ordered gallery layers with per-effect previews, reorder, and show/hide, each becoming a `StudioLiveFilter` entry on commit. Several draw with the active foreground/background colors and expose texture/relief/light-direction sub-controls, preserved as typed parameters. Available to raster targets and to vector targets via the live-raster-effect path ([STU-FX-005]) governed by the document raster-effect resolution setting (group 14).

**Effect Gallery browser**

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Effect Gallery workspace | R,V | Opt | Applies/stacks the artistic gallery effects cumulatively with per-effect layers, reorder, preview, and show/hide inside one surface. |

**Artistic**

| Studio effect | Behavior (normative) |
|---|---|
| Colored Pencil | Redraws as colored-pencil crosshatch strokes over a solid background color. |
| Cutout | Renders as layered cut colored-paper shapes with posterized regions. |
| Dry Brush | Simplifies color range and paints edges between oil and watercolor (dry-brush). |
| Film Grain | Grain on shadows/midtones with smoother saturated grain in highlights to unify sources and reduce banding. |
| Fresco | Short rounded coarse daubs in a fresco style. |
| Neon Glow | Adds configurable colored glows to objects while softening the image. |
| Paint Daubs | Painterly effect; brush sizes 1-50, types Simple/Light Rough/Dark Rough/Wide Sharp/Wide Blurry/Sparkle. |
| Palette Knife | Reduces detail to a thinly-painted canvas look revealing underlying texture. |
| Plastic Wrap | Coats the image in shiny plastic accentuating surface detail. |
| Poster Edges | Posterizes colors and draws black edge lines along detected edges. |
| Rough Pastels | Pastel chalk strokes over a selectable texture with scaling, relief, light-direction. |
| Smudge Stick | Softens with short diagonal smears of darker areas. |
| Sponge | Sponge-painting simulation with textured contrasting color areas. |
| Underpainting | Paints onto a texture and overlays the final image with texturizing options. |
| Watercolor | Simplifies detail in a watercolor style, saturating color at tonal-change edges. |

**Brush Strokes**

| Studio effect | Behavior (normative) |
|---|---|
| Accented Edges | Accentuates edges as chalk-like or ink-like accents by edge brightness. |
| Angled Strokes | Repaints with diagonal strokes, opposite directions for light and dark areas. |
| Crosshatch | Simulated pencil hatching over preserved detail, 1-3 strength passes. |
| Dark Strokes | Short tight dark strokes for dark areas, long white strokes for light areas. |
| Ink Outlines | Fine narrow ink-style lines over original detail. |
| Spatter | Spatter-airbrush effect with simplification controls. |
| Sprayed Strokes | Angled sprayed strokes in the image's dominant colors. |
| Sumi-e | Japanese wet-brush style with soft blurred edges and rich inky blacks. |

**Sketch**

| Studio effect | Behavior (normative) |
|---|---|
| Bas Relief | Carved low-relief look using foreground for dark, background for light. |
| Chalk & Charcoal | Chalk (background color) highlights/midtones, diagonal charcoal (foreground) shadows. |
| Charcoal | Posterized smudged charcoal drawing with bold edges in foreground color. |
| Chrome | Renders as a polished reflective chrome surface. |
| Conte Crayon | Conte-crayon texture using foreground/background with texturizing options. |
| Graphic Pen | Fine linear ink strokes; foreground as ink, background as paper. |
| Halftone Pattern | Simulates halftone screening while keeping continuous tones. |
| Note Paper | Handmade-paper embossing + grain; dark areas as holes revealing background color. |
| Photocopy | Photocopy simulation: edge-only dark areas, hard black/white midtone falloff. |
| Plaster | Molds as 3D plaster with raised dark and recessed light areas colored by fg/bg. |
| Reticulation | Film-emulsion shrinkage: clumped shadows, lightly grained highlights. |
| Stamp | Simplifies to a rubber/wood stamp look. |
| Torn Edges | Reconstructs as ragged torn-paper pieces colorized with fg/bg. |
| Water Paper | Blotchy daubs on fibrous damp paper with flowing blended colors. |

**Texture**

| Studio effect | Behavior (normative) |
|---|---|
| Craquelure | Paints onto high-relief plaster producing a crack network following contours. |
| Grain | Adds grain in ten types (Regular, Soft, Sprinkles, Clumped, Contrasty, Enlarged, Stippled, Horizontal, Vertical, Speckle). |
| Mosaic Tiles | Small tile chips with grout gaps between tiles. |
| Patchwork | Squares filled with dominant area color at randomized depths simulating highlights/shadows. |
| Stained Glass | Single-color adjacent cells outlined with the foreground color. |
| Texturizer | Applies a selected or loaded texture with scale, relief, invert, and light-direction. |

---

### 11. Utility, Channel & Convolution Filters

[STU-FX-022] Utility filters are low-level pixel operators used in masking, compositing, and legacy-format workflows. Maximum/Minimum are morphological dilate/erode used to choke/spread masks. The watermarking filter is a retired third-party surface, recorded for provenance only and NOT shipped as a native Studio effect.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Custom Convolution | R | Opt | User-defined 5x5 convolution kernel with scale and offset; savable/loadable. |
| Maximum (dilate) | R | Opt | Morphological dilate within a radius with squareness/roundness preservation (spread masks). |
| Minimum (erode) | R | Opt | Morphological erode within a radius with squareness/roundness preservation (choke masks). |
| Offset | R | Opt | Shifts the region horizontally/vertically with wrap-around, repeat-edge, or background fill of vacated areas. |
| De-Interlace | R | No | Removes odd/even interlace lines by duplication or interpolation (video-source cleanup). |
| Broadcast-Safe Colors | R | No | Restricts gamut to television-safe colors to prevent scan-line bleed. |
| Watermark (retired) | R | No | Legacy third-party copyright-watermark embed; retired posture, NOT a native Studio effect. Provenance-only row. |

---

### 12. Interactive Filter Workspaces

[STU-FX-023] Four filter capabilities are interactive on-canvas workspaces rather than parameter-only effects; each is still recorded as a re-editable `StudioLiveFilter` (or a smart-object-hosted stack entry) so the workspace edit is non-destructive. Liquify, mesh warp, and the perspective/wide-angle workspaces are GPU-accelerated.

[STU-FX-024] The mesh-push (Liquify) workspace hosts sub-tools as one effect kind: forward-warp (push pixels in drag direction), pucker (contract toward center), bloat (expand outward), twirl (rotate cw/ccw), plus freeze-mask (protect regions), thaw-mask (release), and reconstruct/smooth (revert or relax distortion). A face-aware mode detects faces and exposes parametric eye/nose/mouth/face-shape sliders. Brush size/pressure/density and a preview toggle are shared workspace parameters. Its distortion is stored as a re-editable mesh on the stack entry (dedup of the destructive workspace and the live-filter mesh warp).

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Mesh-Push (Liquify) workspace | R | Req | Interactive push/pucker/bloat/twirl warping with freeze/thaw masking, reconstruct/smooth, face-aware sliders, and re-editable mesh; brush size/pressure/density + preview. |
| Perspective-Plane workspace | R | Req | Perspective-plane workspace where clone/paint/paste/transform operations conform to user-defined perspective planes. |
| Lens Correction workspace | R | Opt | Fixes barrel/pincushion distortion, chromatic aberration, and vignetting via lens-profile auto-correction or a manual custom tab. |
| Adaptive Wide-Angle workspace | R | Req | Corrects panoramic/wide-angle/fisheye distortion by drawing constraint lines that straighten curved geometry. |
| Develop-as-Filter | R | Opt | Applies the full Camera Raw / Develop edit stack (14.12) as a non-destructive filter entry on any layer. Engine owned by 14.12; hosted here as a `StudioLiveFilter`. |

---

### 13. Layer FX (Cross-Domain Effect Kinds)

[STU-FX-025] Layer effects (shadows, glows, bevels, overlays, strokes) are `StudioLiveFilter` kinds applicable to raster AND vector AND text AND group layers, deduplicated from the layer-style panel, the vector Stylize effects, and the frame-effect panel into one cross-domain effect set. Shadow/glow effects read the layer's alpha and compose behind or inside it; they are stackable (multiple instances) where the source suites allowed multiple instances, and each instance carries its own blend mode, color, and mask.

[STU-FX-026] Because these are ordinary stack entries, a layer fx interleaves with filters and adjustments in one order (a blur below a drop shadow differs from a blur above it), which no single source suite offered uniformly across raster and vector. Overlays consume the same `StudioSwatch`/`StudioGradient`/`StudioPattern` primitives as fills (14.8).

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Drop Shadow (stackable, up to 8) | R,V,T,G | Opt | Offset shadow behind the layer alpha with x/y offset, blur, spread, color+opacity, blend mode, and show-behind-transparent toggle. Dedup of drop-shadow across all suites. |
| Inner Shadow (stackable, up to 8) | R,V,T,G | Opt | Shadow rendered inside the layer bounds with x/y offset, blur, spread, color+opacity. |
| Outer Glow | R,V,T,G | Opt | Glow emanating outward from the layer edges with blend mode, opacity, blur, color/gradient. |
| Inner Glow | R,V,T,G | Opt | Glow emanating inward from edges or center with blend mode, opacity, blur. |
| Bevel & Emboss | R,V,T,G | Opt | Raised/recessed 3D edge with style, depth, size, soften, light angle/altitude, highlight/shadow modes, and contour/texture sub-effects. |
| Satin | R,V,T,G | Opt | Interior shading that reacts to the layer shape for a satin/cloth sheen. |
| Color Overlay | R,V,T,G | No | Fills the layer region with a solid `StudioSwatch` at a blend mode/opacity. |
| Gradient Overlay | R,V,T,G | Opt | Overlays a `StudioGradient` (linear/radial/angle/reflected/diamond) with scale/angle/align-with-layer. |
| Pattern Overlay | R,V,T,G | No | Tiles a `StudioPattern` over the layer with scale and link-with-layer. |
| Stroke / Outline fx | R,V,T,G | Opt | Live outline of the layer alpha (inside/center/outside) filled with a swatch/gradient/pattern. |
| Feather (edge) | R,V,T,G | Opt | Fades the layer/object edges to transparent over a feather radius. |
| Round Corners | V,T | No | Live-rounds corner points of a vector object by radius (vector-native corner effect). |
| Scribble | V,T | No | Renders fills/strokes as hand-drawn scribble hatching with angle, path overlap, stroke width, curviness, spacing variance. |

---

### 14. Vector Live Effects

[STU-FX-027] Vector live effects are `StudioLiveFilter` kinds that operate on the live `StudioVectorPath`/`StudioVectorNetwork` geometry inside the effect stack, re-evaluated whenever the geometry or parameters change, and expandable to real geometry on explicit `EFFECT_BAKE`. They deduplicate the appearance/live-effect stack into the same Studio effect model as raster filters. Raster-group vector effects (artistic/blur/etc. applied to vector art) render through the document raster-effect resolution setting ([STU-FX-030]) and reuse the group-10 catalog rather than redefining those filters.

[STU-FX-028] 3D-and-materials effects extrude/revolve/inflate/rotate 2D vector art into 3D with cap/bevel/rotation controls, and expose a materials + lighting model with an optional ray-traced GPU render pass. Parametric materials and community material assets are an optional provider-asset dependency (group 16); the geometric extrude/revolve/inflate primitives are native.

**3D & Materials**

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Extrude & Bevel | V,T,G | Req | Extrudes 2D art into 3D with depth, cap, bevel shape/width, rotation. |
| Revolve | V,T,G | Req | Revolves a profile path around a vertical axis with angle, offset, cap options. |
| Inflate | V,T,G | Req | Inflates flat art into a puffed 3D volume with depth/inflation controls. |
| 3D Rotate | V,T,G | Req | Rotates flat art in 3D space (plane effect) without adding depth. |
| Materials | V,T,G | Req | Applies parametric materials + custom graphics onto 3D surfaces (community material assets are provider-optional). |
| 3D Lighting | V,T,G | Req | Multiple lights with intensity, rotation, height, softness, ambient, and shadow options. |
| Ray-Traced Render | V,T,G | Req | GPU ray-traced render pass for 3D objects with quality settings before export/expand. |
| 3D (Classic) | V,T,G | Opt | Legacy 3D engine (classic extrude/revolve/rotate) with surface shading and map-art options. |

**Distort & Transform (vector)**

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Free Distort | V,T,G | No | Reshapes vector art by dragging four corner points of a distortion frame. |
| Pucker & Bloat | V,T,G | No | Pulls anchor points outward (pucker) or inward (bloat), curving segments opposite. |
| Roughen | V,T,G | No | Turns segments into jagged peaks/valleys with size, detail, smooth/corner points. |
| Transform (step-repeat) | V,T,G | No | Live move/scale/rotate/reflect with a copies count for procedural step-and-repeat stacks. |
| Tweak | V,T,G | No | Randomly curves/distorts segments inward/outward by relative or absolute amounts. |
| Twist | V,T,G | No | Rotates geometry more sharply at the center than the edges by a twist angle. |
| Zig Zag (vector) | V,T,G | No | Transforms segments into uniform jagged or wavy arrays with size, ridges, point type. |

**Pathfinder Effects (live)**

[STU-FX-029] Pathfinder effects are live boolean/print-composite operations applied as appearance effects on a group/layer, non-destructive until expanded. They dedup the pathfinder-as-effect surface; the destructive pathfinder command set is owned by 14.5 and reuses the same boolean core.

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Add (union) | V,G | No | Live union of group/layer contents. |
| Intersect | V,G | No | Keeps only overlapping areas. |
| Exclude | V,G | No | Removes overlapping areas. |
| Subtract | V,G | No | Subtracts front objects from the backmost object. |
| Minus Back | V,G | No | Subtracts back objects from the frontmost object. |
| Divide | V,G | No | Divides into independent faces at every intersection. |
| Trim | V,G | No | Removes hidden parts of filled objects, discarding strokes. |
| Merge | V,G | No | Trim plus merge of adjacent same-color faces. |
| Crop | V,G | No | Crops artwork to the frontmost object's outline. |
| Outline (edges) | V,G | No | Divides artwork into unfilled edge segments. |
| Hard Mix | V,G | No | Color mix taking the highest CMYK component of overlaps. |
| Soft Mix | V,G | No | Color mix making overlapping colors visible through a mixing-rate percentage. |
| Trap | V,G | No | Print traps spreading lighter colors into darker overlap zones (thickness, tint reduction). |

**Convert-to-Shape, Path & Warp Effects**

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| Convert to Rectangle | V,T,G | No | Live-converts object silhouette to a rectangle by absolute/relative size. |
| Convert to Rounded Rectangle | V,T,G | No | Live-converts silhouette to a rounded rectangle with corner radius. |
| Convert to Ellipse | V,T,G | No | Live-converts silhouette to an ellipse by absolute/relative size. |
| Crop Marks | V,G | No | Adds live crop marks around the object bounds as an appearance effect. |
| Offset Path | V,T,G | No | Live non-destructive offset of the path outline within the stack. |
| Outline Object | V,T,G | No | Live-outlines text/objects so later effects operate on outlines. |
| Outline Stroke | V,T,G | No | Live-converts the stroke to an outline within the stack. |
| Warp: Arc | V,T,G | No | Bends art into an arc (shared Bend + H/V distortion sliders). |
| Warp: Arc Lower | V,T,G | No | Bends only the lower edge. |
| Warp: Arc Upper | V,T,G | No | Bends only the upper edge. |
| Warp: Arch | V,T,G | No | Arches the whole object. |
| Warp: Bulge | V,T,G | No | Bulges edges outward. |
| Warp: Shell Lower | V,T,G | No | Curves into a lower shell shape. |
| Warp: Shell Upper | V,T,G | No | Curves into an upper shell shape. |
| Warp: Flag | V,T,G | No | Waves from the center like a flag. |
| Warp: Wave | V,T,G | No | Applies a full sinusoidal wave. |
| Warp: Fish | V,T,G | No | Tapers into a fish-like silhouette. |
| Warp: Rise | V,T,G | No | Rises the wave from a flat baseline. |
| Warp: Fisheye | V,T,G | No | Bulges the center like a fisheye lens. |
| Warp: Inflate | V,T,G | No | Inflates edges outward in both axes. |
| Warp: Squeeze | V,T,G | No | Pinches the middle inward. |
| Warp: Twist | V,T,G | No | Rotates art progressively around the center. |

**SVG, Rasterize & Raster-Effect Coordination**

[STU-FX-030] The document raster-effect resolution/color-model/background/anti-aliasing settings are a document-level `StudioEffectStack` policy applied by every raster effect used on vector art. SVG-filter effects are an XML-defined interchange path (import/apply/edit) preserved for `.svg` round-trip (14.13) and are adapter-neutral (native evaluation where the filter primitives map to native passes).

| Studio effect | Targets | GPU | Behavior (normative) |
|---|---|---|---|
| SVG Filter (apply/import) | V,T,G | Opt | Applies XML-defined SVG filter effects and imports filters from SVG files; editable as XML for round-trip. |
| Rasterize (live) | V,T,G | Opt | Live raster appearance (resolution, background, anti-aliasing) without destroying the vector source. |
| Document Raster-Effect Settings | doc | No | Global resolution, color model, background, anti-aliasing applied by every raster effect in the document. |
| Apply Last Effect | R,V,T,G | n/a | Re-applies the most recent effect with same or new settings ([STU-FX-009]). |

---

### 15. Figma-Origin Composite Effects

[STU-FX-031] The layer-effect rows originating in the Figma effect model (drop/inner shadow, layer blur, progressive blur, background/backdrop blur, noise, texture, glass, effect styles) are NOT a separate effect system: they dedup into the blur family (group 3), the noise family (group 5), the distort family (group 4, texture + refractive glass), the layer-fx set (group 13, shadows), and the effect-styles surface (group 19). This row exists to record the dedup so no Figma effect is lost or double-counted; the normative homes are those groups.

| Figma-origin effect | Deduped Studio home |
|---|---|
| Drop shadow (up to 8) | Layer FX -> Drop Shadow (group 13). |
| Inner shadow (up to 8) | Layer FX -> Inner Shadow (group 13). |
| Layer blur (uniform) | Blur -> Gaussian Blur (group 3). |
| Progressive blur | Blur -> Progressive Blur (group 3). |
| Background blur | Blur -> Background Blur / backdrop (group 3). |
| Noise (mono/duo/multi) | Noise -> Add Noise (group 5). |
| Texture effect | Distort -> Texture / surface displace (group 4). |
| Glass effect | Distort -> Glass / refractive material (group 4). |
| Effect styles | Effect Styles & presets (group 19). |

---

### 16. Provider / AI Generative Filter Lane (Adapter-Backed, Optional)

[STU-FX-032] Generative and ML-model filters are an OPTIONAL adapter lane, not native deterministic effects, per the local-first posture ([STU-OVR-002]). They are exposed as `StudioLiveFilter` entries whose `filter_kind` binds to a `StudioModelAdapter` (14.23); when no adapter is configured the effect surfaces a determinate `EFFECT_ADAPTER_UNAVAILABLE` result and is never a silent no-op. A locally-downloadable model (run through a local adapter) and a cloud provider (run through a remote adapter) are the SAME effect kind with different adapter bindings. No generative filter is required for Studio parity; the deterministic filter primitives in groups 3-14 are the native baseline.

[STU-FX-033] Adapter-backed filter output MUST land as a non-destructive stack entry or a new masked layer selectable by the operator/model (current layer / new layer / masked layer / new document), never overwriting source data. Provider calls are subject to the headless/quiet law (14.20): no foreground popup, bounded, observable, attributable.

[STU-FX-034] The ML/neural filter models below and the generative surfaces are recorded as normative Studio effect ROWS with `provider = adapter-backed/optional`. Native-first alternatives (e.g. deterministic denoise vs. ML denoise, deterministic upscale vs. ML super-resolution) exist in groups 3-9 and are preferred when parity is achievable natively.

| Studio effect | Lane | Behavior (normative) |
|---|---|---|
| ML Filter lane (workspace) | adapter | Machine-learning filter gallery: downloadable models, categories, output to current/new/masked layer, smart-filter, or new document. Local adapter after download; some models remote. |
| ML: Skin Smoothing | adapter (local) | One-click smoothing of blemishes/spots/acne in portraits (small local model). |
| ML: Smart Portrait | adapter | Portrait edits for happiness, surprise, anger, facial age, gaze, hair thickness, head direction, light direction (some ops remote). |
| ML: Super-Resolution (Super Zoom) | adapter | Zoom-crop and synthesize added detail to recover resolution. |
| ML: JPEG Artifact Removal | adapter (local) | Removes compression artifacts from JPEG-compressed images. |
| ML: Colorize | adapter (local) | Adds color to black-and-white photos with placeable focal color points and attribute controls. |
| ML: Style Transfer | adapter (local) | Applies artistic styles (larger local model download). |
| ML: Makeup Transfer | adapter | Transfers eye/mouth makeup style from one open image to another. |
| ML: Photo Restoration | adapter | Fixes scratches, contrast, and damage in old photos. |
| ML: Harmonization | adapter | Matches color/tone of one layer to another for composites with intensity refinement. |
| ML: Landscape Mixer | adapter | Blends input/reference landscape images or presets to restyle scenery. |
| ML: Depth Blur | adapter | Depth-based environmental blur with haze/temperature (some ops remote). |
| ML: Color Transfer | adapter | Applies a reference image's color palette with brightness/saturation/luminescence sliders. |
| Generative Fill / Expand | adapter (cloud) | Text-prompted generative fill and canvas expansion; provider + credits; adapter-backed or omitted, not local parity. |
| Generative Background Replace | adapter (cloud) | Prompted background replacement/generation. |
| Generative Upscale / Harmonize | adapter (cloud) | Generative upscaling and composite harmonization. |
| ML Background Removal | adapter (local pref) | One-step subject/background segmentation to a mask; local model preferred, remote optional. |

---

### 17. Adjustments Coordination with 14.4

[STU-FX-035] 14.9 owns the filter/effect ENGINE and every effect usable across raster+vector; 14.4 owns the raster tonal/color **adjustment layer** set (Brightness/Contrast, Levels, Curves, Exposure, Vibrance, Hue/Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup/LUT, Invert, Posterize, Threshold, Gradient Map, Selective Color, Shadows/Highlights, and the rest). 14.9 does NOT redefine those; it guarantees they are `StudioAdjustment` nodes that interleave with `StudioLiveFilter` nodes in ONE non-destructive stack order ([STU-FX-012]) and share the same masking, blend, opacity, enable/disable, and re-edit surfaces. An adjustment applied "as a filter" (a tonal/color adjustment hosted inside an effect stack rather than as a standalone layer) is the same primitive reached through the effect surface. Where a source suite shipped an adjustment as both an adjustment layer and a filter (e.g. shadow/highlight, lens-filter tint), it is ONE `StudioAdjustment` primitive with two entry points, owned by 14.4 and hosted by 14.9.

---

### 18. GPU Render Requirement & Cross-Backend Equivalence

[STU-FX-036] Every filter/effect evaluates through the `studio-engine` `RenderEngine` (wgpu/WGSL), driven by the `StudioRenderHarness` primitive. Rows marked GPU `Req` MUST run on a supported GPU backend; when none is available the effect returns a determinate `EFFECT_GPU_UNAVAILABLE` state ([STU-FX-018]) rather than a silent empty result, and the harness records the shortfall. Rows marked `Opt` have a CPU fallback path in `studio-engine` that produces promotion-equivalent output. GPU dependencies stay isolated in `studio-engine` and never enter `handshake_core`'s `Cargo.toml` ([STU-ARC-002]).

[STU-FX-037] Filter output MUST satisfy the deterministic cross-backend promotion-equivalence contract of 14.24: for a given effect kind, parameters, seed, and `StudioColorProfile`, output is byte-stable on a fixed backend and within the declared promotion-equivalence tolerance across backends (CPU fallback, Vulkan, Metal, DX12, WebGPU). An effect whose backends disagree beyond tolerance fails validation and cannot be promoted. The per-effect tolerances and the golden-image validation set are authored in 14.24; 14.9 requires that every catalog row is covered by that set.

[STU-FX-038] The effect engine runs headless and quiet (14.20): batch/model-driven effect application MUST NOT pop a foreground window, steal focus, or require an interactive workspace to complete; the interactive workspaces of group 12 expose an equivalent headless parameter path (constraint lines, planes, mesh points as typed data) so a model can apply them without the on-canvas UI.

---

### 19. Effect Styles, Presets & Reuse

[STU-FX-039] A `StudioEffectStack` (or any sub-range of it) is savable as a named, reusable **effect style** in the `StudioStyleRegistry` (schema authority 14.23, 14.10), publishable across documents like color/type/grid styles and deduplicating effect-styles, graphic-styles, and saved-filter-preset surfaces into one primitive. An effect style is a live reference: updating the style updates every layer that applies it. Individual effect parameter sets are also savable as presets (per-filter presets, gallery presets, lighting presets, warp presets) reached through the same registry. Applying an effect style is a typed command carried through the propose-work lifecycle (14.18) for model authorship.

---

### 20. Model Steerability, GUI, Diagnostics & Manual Obligation

[STU-FX-039a] Effect styles and per-filter presets ship as portable data (part of the `StudioStyleRegistry`) so a moved or relocated project (portability posture) carries its effect library with no absolute-path or machine-local dependency; preset assets (textures, displacement maps, custom kernels, material graphs) are referenced through document-relative or configured asset roots, never hardcoded paths.

[STU-FX-040] Every effect/filter capability in 14.9 is a single primitive with two projections — the operator effect UI and the typed model/MCP command surface ([STU-DOC-004]) — and MUST satisfy the cross-cutting obligations once, without re-statement per row: full model visibility and steerability of the effect stack, its entries, parameters, masks, order, and results per 14.16; parallel-safe, attributable, recoverable effect authorship per 14.17-14.19; the headless/quiet law per 14.20 ([STU-FX-038]); Argus visual-diagnostic coverage of effect rendering, GPU-fallback state, and cross-backend equivalence per 14.16/14.24; and dual-audience UserManual documentation of the effect model, every catalog family, GPU/adapter requirements, and failure/recovery states per 14.22. No effect row is complete until its model command, its GUI surface, its Argus diagnostic hook, and its UserManual entry exist.


## 14.10 Design Systems, Components & Variables

This sub-section is the normative Studio feature set for design systems: reusable components and instances, variants and component properties, the override model, design-token variables and their collections/modes, the styles registry, and the responsive-layout contract (auto layout, constraints, layout grids). It is the deduped union of the source suites' design-system surfaces — the Figma component/variant/variable/style/auto-layout system, Illustrator symbols and the appearance/graphic-styles model, and InDesign's style/library posture — collapsed to one Studio primitive and one command family per shared capability ([STU-SECTION-003]). Every capability here operates on `StudioLayer` nodes inside the single unified `StudioDocument` ([STU-DOC-001]); the canonical field-level definitions for `StudioComponent`, `StudioComponentInstance`, `StudioVariable`, `StudioVariableCollection`, `StudioStyleRegistry`, `StudioAutoLayout`, `StudioConstraint`, and `StudioLayoutGrid` live in 14.23, and where this sub-section and 14.23 disagree, 14.23 wins. Provider-hosted behaviors in the provenance (team-library distribution, cloud analytics, mode-count plan gates) are re-expressed here as local-first primitives with optional cloud adapters; Studio requires no account and no cloud ([STU-OVR-002]).

---

### 1. Components and Instances

[STU-DS-001] `StudioComponent` (schema id `hsk.studio.component@1`) is the single canonical reusable-definition primitive. It subsumes both the Figma-class component and the Illustrator-class symbol: a source suite's "symbol" and "component" collapse to one `StudioComponent`, never two features, and no source product name ("component", "symbol", "master") is a Studio type or command name.

[STU-DS-002] A `StudioComponent` is created from a `StudioSelectionSet` of one or more `StudioLayer` nodes. Studio MUST support both single-component creation (the whole selection becomes one main component) and bulk creation (one component per selected top-level node in a single command).

[STU-DS-002a] The Illustrator-class symbol-set placement and manipulation tooling (spray, shift, scrunch, size, spin, stain, screen, style a set of instances) dedups onto `StudioComponentInstance` placement and per-instance transform/style operations; a symbol set is a group of `StudioComponentInstance` nodes, not a distinct primitive. No symbol-tool product name is a Studio command name.

[STU-DS-003] `StudioComponentInstance` (schema id `hsk.studio.component_instance@1`) is a live reference to a `StudioComponent`. An instance renders the current component definition plus its own local overrides ([STU-DS-016]); it is not a copy.

[STU-DS-003a] Instances MUST remain valid and renderable when their main `StudioComponent` is deleted, and Studio MUST provide a restore command that regenerates the main `StudioComponent` from any surviving instance.

[STU-DS-004] Every `StudioComponent` and `StudioComponentInstance` MUST carry an optional description and documentation-link field, surfaced in the operator asset browser, the instance inspector, and the model/UserManual surfaces. Components, styles ([STU-DS-024]), and variables ([STU-DS-013]) share this description contract.

[STU-DS-005] Studio MUST provide an asset-browser surface that lists local and enabled-library components with text search, section grouping by library and by document page, and drag-to-insert (operator) / typed-insert (model) instantiation.

[STU-DS-005a] Component and style organization uses slash-path names (e.g. `Button/Primary/Large`) that render as nested picker folders without a separate folder entity; the slash-path is a naming convention on the component/style name, not a distinct primitive.

### 2. Variants and Variant Properties

Variants let one component carry many named states (size, style, state) without duplicating definitions, and are the backing for runtime variant switching and interactive components in 14.11.

[STU-DS-006] A `StudioComponent` MAY be a variant set: a group of related component definitions addressed by named `property=value` pairs (e.g. `state=hover`, `size=large`). A variant set is one `StudioComponent` with a variant-property schema, not N separate components; each variant is a member definition selected by its property assignment.

[STU-DS-006a] Studio MUST support combining separate components into a variant set and extracting a member back to a standalone component, with variant properties added, renamed, reordered, and value-edited on the set.

[STU-DS-007] Studio MUST detect variant conflicts: two member definitions with identical variant-property assignments are a validation error flagged on the component until the assignments are made unique. Conflict detection is a `StudioValidationDescriptor` check (14.24), not merely a UI hint.

[STU-DS-008] An instance of a variant-set component exposes its variant properties as switchable fields; setting a variant property re-selects the member definition in place while preserving compatible overrides ([STU-DS-017]). Runtime variant switching from a prototype interaction is the `change to` action defined in 14.11 ([STU-PRO-014c]).

### 3. Component Properties

Component properties are the alternative to variant explosion: instead of a distinct variant for every combination, an author exposes typed knobs on the instance. Each property type binds to a specific internal surface of the component and is edited from the instance inspector.

[STU-DS-009] A `StudioComponent` MAY declare named component properties that surface on every instance's inspector as typed fields, decoupling instance-level configuration from variant explosion. The property-type set is normative and enumerated below; each type binds to a specific internal surface of the component.

| Property type | Anchor | Binds to | Instance-level effect |
|---|---|---|---|
| Boolean | [STU-DS-010] | visibility of one or more nested layers | toggles sub-elements on/off without new variants |
| Text | [STU-DS-011] | the content of one or more text layers | edits copy per instance from a named field |
| Instance-swap | [STU-DS-012] | a nested `StudioComponentInstance` slot | swaps the nested component, constrained to an author-curated preferred-values list |
| Exposed nested | [STU-DS-013] | properties of a nested instance marked "exposed" | surfaces the inner instance's own properties on the outer instance inspector |
| Slot | [STU-DS-014] | a structural placeholder region | lets a consumer insert or replace arbitrary child content per instance |

[STU-DS-015] Instance-swap properties MUST support an author-curated preferred-values list that restricts the swap picker to an allowed component set while still permitting a full search-all fallback. Slots ([STU-DS-014]) complement instance-swap for open-ended composition where the inserted content is not a single swappable instance.

### 4. The Override Model

The override model is what makes instances useful: a consumer customizes an instance locally while still tracking upstream component changes. Its correctness (what inherits, what persists, what resets) is a normative contract, not a UI convenience.

[STU-DS-016] An instance inherits every change to its main `StudioComponent` except properties the consumer has locally overridden. Overridable surfaces are: text content, fills/strokes/paint, effects, visibility, nested instance swaps, and exposed-nested property values.

[STU-DS-017] Overrides MUST persist across variant switches and component swaps when the target structure matches by layer name and hierarchy, and MUST be resettable both per-property and wholesale (reset-all-overrides). Override matching uses the same name/hierarchy pairing the smart-animate engine uses in 14.11 ([STU-PRO-018]).

[STU-DS-018] Instance swap MUST offer a searchable picker that replaces an instance with any other component and carries compatible overrides onto the new component; incompatible overrides are dropped, not silently reassigned. Swap and override state are authority fields on `StudioComponentInstance` in 14.23.

### 5. Libraries, Publishing, and Swap

Libraries are how a design system is shared and versioned across documents: authors publish components/styles/variables, consumers subscribe and accept updates. Studio's model is local-first — the whole publish/consume loop works offline — with distribution as an optional adapter.

[STU-DS-019] A library is a publish/subscribe surface over `StudioComponent`, `StudioStyleRegistry` entries, and `StudioVariableCollection`s. The publish/subscribe model is a local-first Studio primitive; cross-machine or cross-tenant distribution is an optional adapter, never a runtime requirement ([STU-OVR-002]).

[STU-DS-019a] Publishing MUST push selected items with a per-item change list and publish notes.

[STU-DS-019b] Consuming documents MUST receive update-review prompts, preview pending updates, and apply them selectively while keeping instance overrides intact ([STU-DS-016]).

[STU-DS-020] Each `StudioDocument` MUST be able to enable or disable individual published libraries, scoping the visible design-system surface (asset browser, style pickers, variable pickers) per document.

[STU-DS-021] Library publish, update-review, and swap operations authored by a model MUST pass through the sandbox -> validation -> `PromotionGate` lifecycle ([STU-ARC-005]) before authority rows change; a library update that would break instances is a validation failure, not an accepted mutation.

### 6. Variables and Collections

Variables are Studio's design-token layer: single, mode-aware, aliasable values bound to properties across the document, distinct from the multi-property styles in group 7 and shared with the prototyping runtime in 14.11.

[STU-DS-022] `StudioVariable` (schema id `hsk.studio.variable@1`) is the single design-token primitive. Its type set is normative and enumerated below. Every variable resolves to exactly one value per active mode of its owning collection ([STU-DS-023]).

| Variable type | Anchor | Value domain | Primary bind surfaces |
|---|---|---|---|
| Color | [STU-DS-022a] | a color value carrying an explicit `StudioColorProfile` ref ([STU-DOC-003]) | fills, strokes, paint slots |
| Number | [STU-DS-022b] | a typed numeric with unit | dimensions, gaps, padding, corner radius, min/max, typographic numerics, prototype arithmetic |
| String | [STU-DS-022c] | a text value | text content, font-family names, variant-property values, prototype concatenation |
| Boolean | [STU-DS-022d] | true/false | layer visibility, boolean component properties, prototype conditionals |

[STU-DS-023] `StudioVariableCollection` (schema id `hsk.studio.variable_collection@1`) groups variables and defines the collection's modes (e.g. `light`/`dark`, `compact`/`comfortable`). Every variable in a collection stores one value per mode. Studio MUST NOT impose a provider-style plan-gated mode-count cap; mode count is bounded only by document and performance limits, documented in 14.23.

[STU-DS-024] A mode is set explicitly on any `StudioLayer` container (frame/section/page) or inherited (auto) from ancestors; setting or changing a mode re-resolves every bound variable in that subtree. Runtime mode switching from a prototype is the `set_variable_mode` action in 14.11 ([STU-PRO-016]).

[STU-DS-024a] Binding a `StudioVariable` to a property replaces a raw literal with a live reference; changing the variable's per-mode value, or changing the active mode, re-resolves every bound property. A property MUST be able to hold either a literal or a variable binding, never an ambiguous mix on the same field.

[STU-DS-025] Each `StudioVariable` carries a scope that restricts which property pickers offer it (e.g. a color variable scoped to fills only), plus a hide-from-publishing flag. Scoping is enforced in both the operator picker and the model command surface so a scoped variable is not offered on an ineligible property.

[STU-DS-026] Variables MUST support aliasing: a variable may reference another variable (semantic-token -> primitive-token chains), resolved through the active mode at bind time. Alias chains MUST be cycle-checked by a `StudioValidationDescriptor` (14.24); a cyclic alias is a validation error.

[STU-DS-027] A `StudioVariableCollection` MAY extend another collection, inheriting its variables while overriding selected values, to support multi-brand token systems. Each variable MAY additionally store optional per-platform code-syntax names (e.g. Web/Android/iOS token names) consumed by the export/codegen surface (14.14) instead of the raw variable name.

[STU-DS-028] Number, string, and boolean variables participate in prototype runtime expressions and conditionals defined in 14.11 ([STU-PRO-025], [STU-PRO-026]); the variable primitive is shared between the design-system surface here and the prototyping surface there — there is no separate prototype-only variable type.

### 7. Styles versus Variables (StudioStyleRegistry)

Styles capture reusable multi-property definitions (a full paint stack, a complete type spec, an effect stack, a grid) where variables capture single mode-aware values. The two are complementary: a style may consume variables, and both publish through libraries.

[STU-DS-029] `StudioStyleRegistry` (schema id `hsk.studio.style_registry@1`) owns named multi-property styles, distinct from single-value variables. The registry holds four style kinds, each a normative Studio primitive that subsumes the corresponding source-suite style and the Illustrator-class graphic style / appearance preset:

| Style kind | Anchor | Captures | Applies to |
|---|---|---|---|
| Color/paint style | [STU-DS-030] | a full paint stack (multiple paints incl. gradients/images) | fills and strokes |
| Text style | [STU-DS-031] | a complete typographic definition (`StudioTypeStyle`, 14.7) | text stories/ranges |
| Effect style | [STU-DS-032] | a `StudioEffectStack` (14.9) — shadows, blurs, glows | any effect-bearing layer |
| Grid style | [STU-DS-033] | a `StudioLayoutGrid` configuration ([STU-DS-047]) | frames |

[STU-DS-034] Styles-vs-variables division of labor is normative: styles bundle multi-property definitions, variables are single mode-aware values, and a variable MAY be consumed inside a style (e.g. a color variable inside a paint style). Both publish through libraries ([STU-DS-019]).

[STU-DS-034a] Every `StudioStyleRegistry` entry MUST support the standard style operations: apply, merge (additive), break-link (detach), and redefine-from-selection (update all users). These are the registry's uniform verbs across all four style kinds and subsume the Illustrator-class graphic-style operations.

[STU-DS-034b] The Illustrator-class appearance stack — multiple stacked fills/strokes per object, each with its own color/opacity/blend and per-row live effects, reorderable and independently toggleable — is represented on the layer's paint model and `StudioEffectStack` (14.5, 14.9), not by a separate design-system primitive; a saved appearance preset is a `StudioStyleRegistry` color/effect entry.

### 8. Auto Layout (StudioAutoLayout)

Auto layout is the responsive engine behind components, lists, and adaptive UI: it turns a static frame into a self-arranging container. The full contract below is a single primitive so a no-context model can produce responsive structures without a separate CSS/flex/grid model.

[STU-DS-035] `StudioAutoLayout` (schema id `hsk.studio.auto_layout@1`) is the responsive-layout contract on a frame `StudioLayer`. A frame with auto layout repositions its children automatically as children are added, removed, resized, or reordered. The full contract below is normative and MUST be expressible on one `StudioAutoLayout` value.

[STU-DS-036] Flow direction MUST support vertical, horizontal, and grid (two-dimensional). Grid flow arranges children into resizable rows and columns with per-child row/column span controls, giving a CSS-grid-style 2D auto layout in the same primitive as one-dimensional stacks.

[STU-DS-037] Horizontal flow MUST support wrap, flowing overflowing children onto the next line to produce responsive multi-row layouts from a single container.

[STU-DS-038] Gap between children MUST accept a numeric value, an "auto" value (space-between distribution across the container), and negative values (overlapping stacks such as avatar rows) while preserving flow order.

[STU-DS-039] Auto-layout frames MUST expose a canvas-stacking setting controlling whether overlapping children render first-on-top or last-on-top.

[STU-DS-040] Padding MUST be settable uniformly, per axis (horizontal/vertical), or per individual side (top/right/bottom/left).

[STU-DS-041] Child alignment MUST support a nine-position alignment box plus a separate text-baseline alignment toggle for aligning mixed-height text rows.

[STU-DS-041a] Alignment MUST distinguish the primary (flow) axis from the counter axis: the primary axis governs packing/distribution (including the auto/space-between gap of [STU-DS-038]) and the counter axis governs cross-alignment; grid flow ([STU-DS-036]) resolves alignment per cell.

[STU-DS-042] Every auto-layout participant MUST have a per-axis resizing mode: hug-contents (the frame sizes to its children), fill-container (the child stretches to its parent), or fixed. This is the core responsive sizing contract.

[STU-DS-043] Auto-layout objects MUST accept minimum and maximum width/height clamps that bound hug and fill resizing so responsive components neither collapse nor overgrow.

[STU-DS-044] Individual children MUST be excludable from the flow (ignore-auto-layout / absolute position), remaining inside the frame under constraint-based positioning ([STU-DS-046]) for overlays such as notification badges.

[STU-DS-045] `StudioAutoLayout` MUST additionally support: an advanced setting choosing whether child stroke weights count in spacing calculations, and arbitrarily deep nesting of auto-layout frames combining per-level direction/gap/padding/resizing to express complete responsive component trees. A convert-to-auto-layout command (optionally assisted) MAY restructure a plain frame's contents into an equivalent auto-layout structure; the result is ordinary local `StudioDocument` state.

### 9. Constraints (StudioConstraint)

Constraints handle responsive positioning for children that are not in an auto-layout flow: they pin or scale a child as its parent frame resizes. Constraints and auto layout are mutually exclusive per axis for a given child ([STU-DS-048]).

[STU-DS-046] `StudioConstraint` (schema id `hsk.studio.constraint@1`) governs how a non-auto-layout child responds when its parent frame resizes.

[STU-DS-046a] Each child MUST carry exactly one horizontal constraint — left, right, left-and-right (stretch), center, or scale — controlling its x position and width on parent resize.

[STU-DS-046b] Each child MUST carry exactly one vertical constraint — top, bottom, top-and-bottom (stretch), center, or scale — controlling its y position and height on parent resize. The default constraint is top-left.

[STU-DS-046c] The scale constraint MUST store size and position as percentages of the parent so the child grows and shrinks proportionally on both axes.

[STU-DS-046d] Studio MUST support temporarily suspending child constraints during a parent resize (modifier-held drag) so the frame resizes without repositioning children.

### 10. Layout Grids (StudioLayoutGrid)

[STU-DS-047] `StudioLayoutGrid` (schema id `hsk.studio.layout_grid@1`) is the alignment-guide overlay on a frame `StudioLayer`. It is a rendering/alignment aid distinct from `StudioAutoLayout` (which repositions children) and `StudioConstraint` (which resizes children); a frame MAY carry all three independently.

[STU-DS-047a] `StudioLayoutGrid` MUST support a uniform square grid with configurable cell size, color, and opacity, for icon and pixel-alignment work.

[STU-DS-047b] `StudioLayoutGrid` MUST support column and row guides with a count and one of two sizing modes: fixed size with left/center/right (columns) or top/center/bottom (rows) anchoring plus offset, or stretch mode with margin and gutter. Each guide set carries its own color and opacity.

[STU-DS-047c] A single frame MUST support multiple stacked `StudioLayoutGrid` overlays (e.g. columns + rows + uniform) that render simultaneously and toggle individually.

[STU-DS-047d] A `StudioLayoutGrid` configuration MAY be saved as a named grid style in `StudioStyleRegistry` ([STU-DS-033]) and reapplied across frames and documents like any other shared style.

[STU-DS-047e] Studio MUST additionally support canvas- and frame-scoped ruler guides that snap objects during moves and clear per scope, live alt-hover distance measurement between objects and frame edges, object snapping with transient alignment lines and equal-spacing badges, a pixel grid visible at high zoom with a snap-to-pixel preference, and a tidy-up/smart-selection command that evenly distributes a rough selection into a row/column/grid with draggable spacing handles. These alignment aids are part of the layout surface but are not `StudioLayoutGrid` overlays.

[STU-DS-048] Resizing behavior across the layout system is unified: a child's response to a resizing parent is governed by `StudioAutoLayout` participation ([STU-DS-042]) when the parent is an auto-layout frame, and by `StudioConstraint` ([STU-DS-046]) otherwise. The two MUST NOT both drive the same child on the same axis simultaneously; absolute-positioned auto-layout children ([STU-DS-044]) fall back to `StudioConstraint`.

### 11. Design-System Analytics and Library-Management Posture

This group fixes the operational posture of the whole design system: it is local-first authority, works offline, and treats any cloud aggregation as an optional adapter rather than a dependency.

[STU-DS-049] Studio MUST be local-first for the entire design system: components, variants, variables, collections, modes, styles, and library enablement are authority records in the local `StudioDocument`/SurrealDB layer and function fully offline with no account ([STU-OVR-002], [STU-ARC-003]). No design-system feature in this sub-section may hard-depend on a cloud service.

[STU-DS-050] Design-system analytics (component/variable/style usage counts, adoption, detachment rates, orphaned-instance reports) MUST be available as a locally computed report over the authority rows. Any cloud/team aggregation of analytics is an optional adapter layered on the local report, never a prerequisite for using or publishing the design system.

[STU-DS-051] GUI / Argus / UserManual obligation. Every design-system panel, picker, inspector field, and visible state in this sub-section (component and variant editors, the component-property inspector, the variable/collection/mode editor, the style registry, and the auto-layout/constraint/grid controls) MUST be model-visible and typed-steerable through the Studio command surface ([STU-SECTION 14.16]); MUST be headlessly inspectable, steerable, and screenshot-capturable through Argus with no foreground focus steal ([STU-SECTION 14.20], HBR-VIS/HBR-QUIET); and MUST ship dual-audience UserManual entries — operator layer (task-oriented) plus model layer (command ids, typed I/O, receipts, undo semantics, Argus targets, failure/recovery) — kept same-change current ([STU-SECTION 14.22]).

---

## 14.11 Prototyping, Motion & Interaction

This sub-section is the normative Studio feature set for interaction and motion: prototype flows with triggers/actions/animations/overlays/scroll behaviors, the keyframe motion timeline, interactive-document objects (buttons, forms, multi-state objects, page transitions, media), and the binding of runtime variables into all of them. It is the deduped union of the Figma prototyping and Figma Motion surfaces and the InDesign interactive/EPUB surface, collapsed to one Studio primitive per shared capability ([STU-SECTION-003]); a source product's feature name ("Smart Animate", "Motion", "Multi-State Object", "DPS") is never a Studio command or panel name. Prototyping and motion operate on `StudioLayer` nodes in the unified `StudioDocument` and bind to the same `StudioVariable`/`StudioVariableCollection` primitives defined in 14.10. The canonical field-level definitions for `StudioPrototypeFlow` and `StudioMotionTimeline` live in 14.23, which wins on any conflict. Provider-hosted behaviors in the provenance (cloud share links, hosted mobile viewers, plan-gated exports, AI keyframe agents) are re-expressed as local-first primitives with optional adapters; nothing here requires an account or network ([STU-OVR-002]). Interactive-document export formats (interactive PDF, EPUB, HTML) are authored here and produced through the export pipeline in 14.13.

---

### 1. Prototype Flows and Connections

[STU-PRO-001] `StudioPrototypeFlow` (schema id `hsk.studio.prototype_flow@1`) is the single prototyping primitive. It owns a document's interaction graph: hotspot -> trigger -> action bindings between `StudioLayer` nodes, named flow starting points, and per-flow settings. It replaces every source-suite prototyping surface with one primitive and one command family.

[STU-PRO-002] A flow MUST support multiple named starting points per page, each defining an entry path, listed in the flow sidebar and independently launchable. A single page may host several flows.

[STU-PRO-002a] Interaction connections MUST render as editable on-canvas arrows; selecting objects reveals their interactions for retargeting or deletion, and a per-page view MUST list all connections for bulk edit.

[STU-PRO-003] Prototype settings MUST let interactive-component state and variable values either reset or persist when navigating between frames (state preservation), and MUST let a navigation optionally preserve the destination's scroll offset from the origin ([STU-PRO-023]).

[STU-PRO-003a] A running prototype MUST maintain navigation history so the back action ([STU-PRO-014b]) is well-defined; overlays layer above the current frame without pushing history unless configured otherwise.

### 2. Triggers

Triggers and actions are the two halves of every interaction: a trigger is the input event, an action ([STU-PRO-014]) is the response. Studio ships the full trigger vocabulary so pointer, touch, keyboard/gamepad, timer, and media-driven prototypes are all expressible.

[STU-PRO-004] A trigger is the event that fires an interaction on a hotspot `StudioLayer` or frame. The trigger set is normative and enumerated below; Studio MUST support the full set. Touch equivalents (tap, touch-down/up) map to the same trigger primitives as their pointer forms.

| Trigger | Anchor | Fires when |
|---|---|---|
| On click / tap | [STU-PRO-005] | the hotspot is clicked (pointer) or tapped (touch) |
| On drag | [STU-PRO-006] | the object is dragged, with continuous movement mapping (swipeable UI) |
| While hovering | [STU-PRO-007] | the cursor is over the hotspot; auto-reverts on exit |
| While pressing | [STU-PRO-008] | during click-and-hold / tap-and-hold; reverts on release |
| Key / gamepad | [STU-PRO-009] | a key, key combination, or controller button is input |
| Mouse enter / leave | [STU-PRO-010] | the cursor enters / exits the hotspot area, without auto-revert |
| Mouse down / up | [STU-PRO-011] | press start / release, without auto-revert (press state machines) |
| After delay | [STU-PRO-012] | a frame-level dwell timer elapses (splash screens, autoplay, timed states) |
| Media timestamp / end | [STU-PRO-013] | a media-bearing layer reaches a set timestamp or ends playback |

### 3. Actions

Actions are the response half of an interaction. Studio's action set spans navigation, overlays, variant switching, scrolling, external links, variable writes, conditionals, and media control, and actions compose (multiple per trigger, nestable under conditionals) so a single trigger can run a small runtime program.

[STU-PRO-014] An action is what a fired trigger does. The action set is normative and enumerated below; Studio MUST support the full set, MUST allow multiple actions chained on a single trigger, and MUST allow actions nested inside conditional branches ([STU-PRO-026]).

| Action | Anchor | Behavior |
|---|---|---|
| Navigate to frame | [STU-PRO-014a] | replaces the top-level frame with a destination, pushing prototype history |
| Back | [STU-PRO-014b] | pops navigation history to the previous frame |
| Change to (variant switch) | [STU-PRO-014c] | switches an instance (incl. nested) to another variant in place ([STU-DS-008]) |
| Open / swap / close overlay | [STU-PRO-014d] | layers a frame above; swaps the active overlay without history; dismisses it |
| Scroll to object | [STU-PRO-014e] | scrolls the viewport or a nested scroll container to a target, instant or eased |
| Open link | [STU-PRO-014f] | opens an external URL (local viewer defers to OS handler) |
| Set variable | [STU-PRO-015] | writes a literal, alias, or expression result into a `StudioVariable` at runtime |
| Set variable mode | [STU-PRO-016] | switches the active mode of a `StudioVariableCollection` for a target scope |
| Conditional (if/else) | [STU-PRO-026] | branches into different action lists on a boolean expression over variables |
| Media playback control | [STU-PRO-017] | play/pause/toggle, mute/unmute/toggle, seek, and jump forward/back by seconds |

### 4. Animations, Smart Animate, and Easing

Animation governs how a transition or keyframe moves between states. Studio provides fixed transition types, a layer-matching smart-animate tween, and one easing catalog shared by prototyping and the motion timeline so a motion system stays consistent across both.

[STU-PRO-018] Transition animations MUST include: instant (no transition), dissolve (cross-fade with duration + easing), and directional move-in/out, push, and slide-in/out (left/right/up/down) with duration + easing. Smart-animate MUST pair layers across origin and destination frames by name and hierarchy and tween position, size, rotation, opacity, and fill differences, fading unmatched layers; it uses the same name/hierarchy pairing as instance override matching ([STU-DS-017]).

[STU-PRO-019] The easing catalog is normative and shared between prototype transitions and the motion timeline ([STU-PRO-030]). It comprises bezier presets, a custom cubic-bezier editor, spring presets, and a custom spring, enumerated below. Easing/timing values MAY be stored as `StudioVariable`s with modes so a mode switch updates every referencing animation ([STU-DS-028]).

| Easing preset | Anchor | Kind |
|---|---|---|
| Linear | [STU-PRO-019a] | bezier |
| Ease in / Ease out / Ease in-and-out | [STU-PRO-019b] | bezier |
| Ease in back / out back / in-and-out back | [STU-PRO-019c] | bezier (overshoot) |
| Hold (jump to final value) | [STU-PRO-019d] | step (timeline segments) |
| Custom cubic-bezier | [STU-PRO-019e] | four-value / two-control-point editor; out-of-[0,1] y produces overshoot |
| Spring: Gentle / Quick / Bouncy / Slow | [STU-PRO-019f] | physics preset |
| Custom spring | [STU-PRO-019g] | parameterized by stiffness, damping, mass |

### 5. Overlays

[STU-PRO-020] An overlay is a frame layered above the current frame, opened/swapped/closed by the overlay actions ([STU-PRO-014d]).

[STU-PRO-020a] Overlays MUST support manual positioning or a set of default position presets.

[STU-PRO-020b] Overlays MUST support an optional background color and opacity behind the overlay and a close-on-click-outside behavior.

[STU-PRO-020c] Swap-overlay replaces the active overlay without adding navigation history; close-overlay dismisses it; open-overlay layers a new one.

### 6. Scroll and Overflow Behaviors

Scroll behavior reproduces real app scrolling inside a running prototype: overflow containers, fixed and sticky elements, and scroll continuity across navigation.

[STU-PRO-021] A frame MUST declare overflow scrolling behavior — none, horizontal, vertical, or both — creating nested scroll containers inside a running prototype.

[STU-PRO-022] Layers MUST support fixed positioning (stay in the viewport while content scrolls) and sticky positioning (stick to the top when reached), reproducing app header/nav behavior.

[STU-PRO-023] Navigation actions MUST optionally preserve the origin frame's scroll offset onto the destination frame for continuity between screens.

### 7. Device Frames, Presentation, and Preview

Presentation and preview are how a prototype is played and reviewed: device chrome, an in-editor live preview, and a full presentation view with zoom, hotspot hints, and keyboard navigation — all local, with shared and mobile viewers as optional adapters.

[STU-PRO-024] Presentation settings MUST support wrapping the running prototype in a device frame from a catalog (phones, tablets, desktops, watches, custom) with a configurable background.

[STU-PRO-024a] Studio MUST provide an in-editor inline preview panel that plays the prototype without leaving the editor and updates live with edits.

[STU-PRO-024b] Studio MUST provide a full presentation view offering fit / fill / actual-size zoom, optional hotspot hinting on misclick, a flow sidebar, restart-flow control, and keyboard navigation across flow frames.

[STU-PRO-024c] Presentation and preview are local; any shared-link or mobile-device viewer is an optional adapter over the local prototype, never required ([STU-OVR-002]). Touch-gesture triggers ([STU-PRO-005], [STU-PRO-006]) MUST map natively when a prototype runs on a touch surface.

### 8. Interactive Components and State

Interactive components move interaction into the component itself: hover/press/toggle states authored once on a variant set run automatically in every instance, so screens do not re-wire common states.

[STU-PRO-027] Variant-to-variant interactions authored on a variant-set `StudioComponent` ([STU-DS-006]) MUST run automatically inside every instance, so hover/press/toggle states work without per-screen wiring (interactive components). Whether these states and their bound variables reset or persist across navigation is governed by the flow's state-preservation setting ([STU-PRO-003]).

### 9. Variables in Prototyping

Runtime variables give prototypes state and logic: set-variable writes, conditionals branch, and expressions compute over the same `StudioVariable` primitives defined in 14.10 — there is no separate prototype-only variable type.

[STU-PRO-025] Set-variable ([STU-PRO-015]) and conditional ([STU-PRO-026]) actions evaluate runtime expressions over `StudioVariable`s: arithmetic on number variables, string concatenation, comparisons, and boolean logic, including reading mode-specific values. The variable, collection, and mode primitives are exactly those in 14.10 ([STU-DS-022] through [STU-DS-028]); prototyping does not define a separate variable type.

[STU-PRO-026] Conditional actions evaluate a boolean expression over variables and branch into distinct action lists; conditionals and multiple chained actions compose so one trigger can run a small runtime program. Expression evaluation MUST be deterministic and side-effect-scoped to variables and navigation, and expression/alias cycles are validation errors ([STU-DS-026]).

### 10. Motion Timeline (StudioMotionTimeline)

The motion timeline is Studio's frame-by-frame animation authoring surface, complementing state-to-state prototype transitions with explicit keyframes, tracks, motion paths, and playback. It is one engine with the interactive-document animation surface ([STU-PRO-040d]), sharing the easing catalog and keyframe model.

[STU-PRO-028] `StudioMotionTimeline` (schema id `hsk.studio.motion_timeline@1`) is the keyframe-animation primitive. It docks a timeline over the document with one track per animated `StudioLayer`. It is a mode of the unified editor, not a separate app or file type; motion data persists inside the `StudioDocument` authority ([STU-DOC-001]) and follows the document's history/undo surface (14.19).

[STU-PRO-028a] The timeline MUST resize/collapse, zoom (slider, pinch, modifier+wheel), and collapse/expand all tracks. Per-layer clips MUST drag to reposition in time and stretch/shrink via side handles, enabling stacked (simultaneous) or sequenced motion.

[STU-PRO-029] Keyframable properties MUST include position, scale, rotation, and opacity per layer, plus animatable effect/material properties exposed as numeric fields; keyframe-eligible properties show an explicit per-property keyframe control.

[STU-PRO-029a] Keyframes MUST support add-at-playhead, single / shift / marquee / per-layer selection, drag-move with snap to the playhead and to other keyframes, jump-to (double-click), and per-keyframe or per-property deletion.

[STU-PRO-029b] An auto-keyframe recording mode MUST record design changes as keyframes at the current playhead position while active, with a clear active-state indicator.

[STU-PRO-029c] Rotation and scale MUST pivot around a per-layer anchor point that defaults to object center and is repositionable on canvas.

[STU-PRO-030] Timeline segment easing uses the shared easing catalog ([STU-PRO-019]), including Hold and custom cubic-bezier with overshoot, and the four spring presets plus custom spring, applied per segment between two keyframes.

[STU-PRO-030a] Playback MUST support Loop, Once, and Ping-pong modes, a scrubbable playhead, jump-to-time, and a seconds/milliseconds time ruler. New animations MUST default to an adjustable duration.

[STU-PRO-031] Position keyframes MUST render an on-canvas motion path shown as easing-spaced dots with a per-keyframe box; boxes drag to reposition and a modifier+handle bends the path into a bezier curve.

[STU-PRO-031a] Preset animation styles (e.g. fade, move, scale) and composite styles (multiple presets applied in one action) MUST be applicable to a selection, land on the timeline at the playhead, and remain editable for type, settings, duration, and easing afterward.

[STU-PRO-031b] An animation MAY be converted into a reusable animated `StudioComponent` ([STU-DS-001]) whose motion travels with the component across documents, supporting a shared motion system.

[STU-PRO-032] Any AI-assisted keyframe generation is an optional model-lane capability that emits real editable `StudioMotionTimeline` keyframes grounded in the document's components and variables; it MUST route through the sandbox -> validation -> `PromotionGate` lifecycle ([STU-ARC-005]) and is never required to author motion by hand.

### 11. Interactive Documents

Interactive documents are the publishing-side counterpart to prototype flows: instead of a design prototype, they produce shipped interactive PDF, EPUB, and HTML with buttons, forms, multi-state objects, media, animations, and page transitions. They reuse the unified document, motion, and export primitives rather than a parallel engine.

[STU-PRO-033] Studio MUST support InDesign-class interactive-document objects on `StudioLayer` nodes for interactive PDF, fixed/reflowable EPUB, and HTML output. These are authored with the same selection, override, and history surfaces as all other Studio primitives; buttons, form fields, and multi-state objects are interactive roles on layers, not a separate document silo.

[STU-PRO-034] Buttons MUST support the event set ([STU-PRO-034a] through [STU-PRO-034c]) and the action set ([STU-PRO-035] through [STU-PRO-037]) enumerated below.

[STU-PRO-034d] Buttons MUST support Normal, Rollover, and Click appearance states, each holding distinct artwork, plus hidden-until-triggered behavior for popup patterns.

[STU-PRO-034e] A per-page tab order MUST sequence keyboard focus across buttons and form fields. A sample library of preconfigured buttons and form elements (e.g. navigation arrows preset to next/previous page) SHOULD be available for insertion.

| Button event | Anchor |
|---|---|
| On release/tap, On click | [STU-PRO-034a] |
| On roll over, On roll off | [STU-PRO-034b] |
| On focus, On blur | [STU-PRO-034c] |

| Button action group | Anchor | Actions |
|---|---|---|
| Navigation/PDF | [STU-PRO-035] | go to destination; first/last/next/previous page; go to URL; go to next/previous view; open file; view zoom |
| Show/hide & forms | [STU-PRO-036] | show/hide buttons and forms; clear form; print form; submit form |
| Media & state (EPUB/interactive) | [STU-PRO-037] | animation; go to page; go to state / next state / previous state; sound; video |

[STU-PRO-038] PDF form fields MUST include check box, combo box, list box, radio button, signature field, and text field.

[STU-PRO-038a] Form fields MUST support the option set: description, required, printable, multiline, password, read-only, sort-items, and export values, applied per field type as relevant.

[STU-PRO-039] Multi-state objects (MSOs) MUST convert a selection into an object with ordered states that add/reorder/delete, add objects to the visible state, paste-into-state, reset-all, and support hidden-until-triggered; MSO state changes are driven by the button state actions ([STU-PRO-037]).

[STU-PRO-040] The interactive-document animation surface MUST support motion presets applied to objects with event triggers (on page load, on page click, on click self, on roll over self, on button event), and per-animation duration, play count/loop, speed easing, and animate-from/to properties (opacity, rotation, scale, visibility).

[STU-PRO-040a] Interactive-document animations MUST support editable vector motion paths, with any drawn path convertible to a motion path for a selected object.

[STU-PRO-040b] A timing surface MUST sequence animations per trigger event with delays, reordering, and linked play-together groups with per-group play counts.

[STU-PRO-040c] Custom motion presets MUST be saveable, duplicable, deletable, and importable/exportable as preset files.

[STU-PRO-040d] The interactive-document animation surface and `StudioMotionTimeline` ([STU-PRO-028]) MUST share the easing catalog ([STU-PRO-019]) and the keyframe/motion-path model; they are two authoring entry points onto one motion engine, not two independent engines ([STU-DOC-004]).

[STU-PRO-041] Media placement MUST support video (with poster frame, controller skin, play-on-load, loop, and navigation points) and audio, targetable by button media actions ([STU-PRO-037]) and media triggers ([STU-PRO-013]).

[STU-PRO-041a] Hyperlinks MUST support URL, file, email, page-with-zoom, text-anchor, and shared-destination targets with configurable appearance and highlight, plus auto URL detection. Nested, sortable PDF bookmarks and generated QR codes (web/text/SMS/email/business-card, editable afterward) MUST be authorable as interactive objects.

[STU-PRO-042] Page transitions MUST support per-spread or all-spread presets (e.g. blinds, comb, dissolve, fade, push, wipe, zoom, page-turn) with direction and speed, honored in full-screen interactive PDF output.

[STU-PRO-042a] An in-app interactivity-preview surface MUST preview animations, MSOs, buttons, and media for the current spread or the whole document before export.

### 12. Interactive and Animated Export Touchpoint

This group is the boundary between authoring (here) and format production (14.13): it enumerates the interactive and animated targets and their option surfaces without duplicating the export engine.

[STU-PRO-043] All interactive and motion output is produced through the export pipeline in 14.13 via `StudioExportRecipe`, not by a separate exporter here. This sub-section authors the interactive/motion content; 14.13 owns the format writers.

[STU-PRO-043a] Interactive PDF export MUST expose general options (pages/spreads, full-screen, page transitions, forms and media inclusion), compression, advanced options (accessibility, tagged PDF), and security.

[STU-PRO-043b] Reflowable and fixed-layout EPUB export MUST carry buttons, MSOs, animations, and media, and MUST expose the EPUB option set: version, cover source, navigation TOC, content order / article threads, split-by-style, image conversion, additional CSS and JavaScript, and metadata.

[STU-PRO-043c] HTML export MUST support content order, image conversion, and CSS options; content-order/article threads MUST be authorable and shared across EPUB, HTML, and tagged-PDF reading order.

[STU-PRO-044] Animated frames MUST export from `StudioMotionTimeline` and prototype transitions as MP4, WebM, GIF, and animated SVG, with size, frame-rate, quality (MP4/WebM), and loop (GIF) settings.

[STU-PRO-044a] Lottie is an optional future export target and is NOT a required format; its absence MUST NOT block motion authoring or the other export formats.

[STU-PRO-044b] A read-only motion-inspection surface MAY expose timing values, easing curves, and keyframes as copyable CSS/JSON/framework code through the automation/dev surface (14.14) as an optional adapter; it is not required to author or export motion.

[STU-PRO-045] GUI / Argus / UserManual obligation. Every prototyping and motion panel, control, and visible state in this sub-section (the flow/connection editor, trigger/action inspectors, the overlay/scroll/device/presentation controls, the motion timeline and keyframe/motion-path canvas, and every interactive-document object editor) MUST be model-visible and typed-steerable through the Studio command surface ([STU-SECTION 14.16]); MUST be headlessly inspectable, steerable, and screenshot-capturable through Argus with no foreground focus steal ([STU-SECTION 14.20], HBR-VIS/HBR-QUIET); and MUST ship dual-audience UserManual entries — operator layer (task-oriented) plus model layer (command ids, typed I/O, receipts, undo semantics, Argus targets, failure/recovery) — kept same-change current ([STU-SECTION 14.22]).


## 14.12 Camera Raw / Develop Pipeline

Studio ships one native, non-destructive raw develop pipeline that replaces the two source-suite raw modalities (the Photoshop/Camera-Raw plug-in host and the Affinity Develop persona) with a single deduped Studio surface. The pipeline is a `StudioRawDevelop` graph over a decoded raw sensor input: the operator (or a model lane) applies an ordered stack of parametric develop adjustments and local masks that never alter the original raw bytes, and the developed result becomes a layer in a `StudioDocument` (14.3, 14.4). Every raw control group, every mask type, the Enhance operations, and the profile/preset/workflow surfaces recorded across the source suites collapse into this one pipeline per [STU-SECTION-003]; the source product names (Camera Raw, ACR, Develop persona) are never Studio tool, panel, or command names.

Canonical primitive addition: `StudioRawDevelop` (schema id `hsk.studio.raw_develop@1`) joins the canonical Studio primitive set of [STU-DOC-002] alongside `StudioLayer`, `StudioMask`, `StudioSelectionSet`, `StudioAdjustment`, and `StudioColorProfile`; its field-level definition is owned by 14.23 and referenced (not redefined) here. All raw develop state is durable Studio authority under the SurrealDB/EventLedger contract of [STU-ARC-004] — there is no XMP sidecar file, no private raw-develop database, and no SQLite cache; the "non-destructive settings storage" behavior of the source suites is satisfied by the EventLedger + authority-record model, not by an external metadata file.

---

### 1. StudioRawDevelop pipeline and non-destructive contract

[STU-RAW-001] `StudioRawDevelop` MUST be a non-destructive, re-editable parametric graph over an immutable decoded raw input. The original raw sensor data (the demosaic source) MUST NOT be mutated by any develop operation; every Basic/Curve/Detail/Color/Optics/Geometry/Effects/Calibration adjustment, every local mask, and every Enhance result is stored as parameters and derived buffers, and the pipeline is fully reversible to the as-decoded state at any time. The develop stack is ordered and deterministic: given the same raw input, the same `StudioRawDevelop` parameter set, and the same process version ([STU-RAW-012]), the rendered output MUST be bit-reproducible on the same engine build.

[STU-RAW-002] Develop authority MUST persist as Studio authority rows bound to the EventLedger per [STU-ARC-003]/[STU-ARC-004] (event family `studio.raster` or a dedicated `studio.raw` family per 14.23), never as an XMP sidecar, a proprietary raw-settings database, or a DNG-embedded settings block. Import of develop settings that arrive embedded in a source container (e.g. a DNG carrying prior develop metadata, a raw with an adjacent sidecar) is a `StudioImportProfile` decode step (14.13) that translates the incoming parameters into `StudioRawDevelop` parameters and records an unsupported-parameter receipt for anything Studio cannot represent; export back to such a container is a `StudioExportRecipe` step.

### 2. Raw input scope and sensor decode

[STU-RAW-003] The pipeline MUST accept mosaic raw sensor inputs (Bayer and X-Trans families) and demosaic them through a native deterministic engine in `studio-engine` (the `RasterEngine`/`RenderEngine` trait boundary of [STU-ARC-002]); it MUST also accept the documented DNG raw container and non-raw high-bit sources (TIFF, JPEG) routed through the same develop surface so that develop adjustments are available on non-raw layers as a re-editable filter (the deduped equivalent of the source "Camera Raw as a filter" and the Affinity Develop-persona-on-pixel-layer behaviors). Large inputs MUST be supported up to the document engine's raster ceiling; the decode boundary is the API decode step per [STU-DOC-003], and color values carry an explicit `StudioColorProfile` from decode onward with no implicit device color.

[STU-RAW-004] The pipeline MUST expose the following as read/observe surfaces, each a projection of the same `StudioRawDevelop` state to the operator UI and the model command surface per [STU-DOC-004] (never a separate model shim):

- A live histogram with toggleable shadow/highlight clipping indication.
- Zoom, pan, and hand navigation with zoom-level presets and a full-screen presentation/review view.
- Before/after preview cycling and per-panel adjustment-visibility toggling (temporarily hiding one group's contribution for comparison).
- A multi-image filmstrip with sort and filter (capture date, name, rating, color label) and per-image star ratings, color labels, and mark-for-deletion state.
- A configurable preview/settings cache with a maximum size, purge, and relocation controls, held under the no-SQLite authority rule ([STU-OVR-003]).

### 3. Panel group — Basic tone and color

[STU-RAW-005] The Basic group MUST provide, as typed parameters on `StudioRawDevelop` each with an explicit unit/range and an active/reset toggle:

- White balance: Temperature and Tint sliders, an eyedropper picker, and an Auto white-balance analysis.
- Tone: Exposure, Contrast, Highlights, Shadows, Whites, and Blacks.
- Presence: Texture, Clarity, and Dehaze-class local-contrast; Vibrance; and Saturation.
- An Auto tone pass that analyzes the image and proposes a full Basic parameter set.

This single Studio group deduplicates the Photoshop Basic panel and the Affinity Develop Basic panel's Exposure / Enhance (contrast, clarity, saturation, vibrance) / White Balance / Shadows-&-Highlights subgroups into one control set.

### 4. Panel group — Tone Curve

[STU-RAW-006] The Tone Curve group MUST provide:

- A parametric curve with region sliders (highlights, lights, darks, shadows) and adjustable split points.
- A point curve with arbitrary control points on the composite channel.
- Independent Red, Green, and Blue channel point curves.

This group also absorbs the Affinity Develop Tones panel's curve / black-and-white / split-toning tonal operations as curve-and-grade parameters on the same primitive; the split-toning behavior is expressed through the Color Grading group ([STU-RAW-008]) rather than as a second control family.

### 5. Panel group — Detail (sharpening and noise)

[STU-RAW-007] The Detail group MUST provide, on the deterministic native engine:

- Capture sharpening: Amount, Radius, Detail, and Masking.
- Luminance noise reduction: Luminance, Luminance Detail, and Luminance Contrast.
- Color noise reduction: Color, Color Detail, and Color Smoothness.
- A noise-addition (grain-preserving) control, matching the Affinity Develop Details panel's Detail Refinement / Noise Reduction / Noise Addition operations under one deduped control set.

Detail operations are deterministic; the AI-model-backed Enhance denoise path is a distinct optional adapter defined in [STU-RAW-014] and MUST NOT be conflated with the deterministic Detail sliders.

### 6. Panel group — Color Mixer / HSL

[STU-RAW-008a] The Color Mixer group MUST provide per-hue-band adjustment of Hue, Saturation, and Luminance (the HSL model) with a Color sub-mode, across the standard color ranges of the image (red, orange, yellow, green, aqua, blue, purple, magenta), plus a targeted on-image adjustment mode that maps a drag to the underlying hue band. This is the same `StudioColorProfile`-aware color surface used elsewhere in Studio (14.8); it is not a raw-only reimplementation.

### 7. Panel group — Color Grading

[STU-RAW-008] The Color Grading group MUST provide:

- Independent Shadows, Midtones, and Highlights color wheels (hue + saturation per range, with a per-range luminance slider).
- A global color wheel.
- Blending and Balance controls governing how the ranges overlap and weight.

It MUST subsume the split-toning capability noted in [STU-RAW-006] as its two-range degenerate case rather than a separate primitive.

### 8. Panel group — Optics / lens correction / defringe

[STU-RAW-009] The Optics group MUST provide:

- Lens-profile correction removing geometric distortion, lens vignetting, and chromatic aberration, with automatic profile match plus manual override and profile selection.
- A manual Defringe with purple and green fringe amount sliders and per-fringe hue-range selection, and a fringe-color sampler.

This deduplicates the Photoshop Optics panel and the Affinity Develop Lens panel (Lens Correction, Chromatic Aberration Reduction, Defringe, Remove Lens Vignette). Lens profiles are Studio-native assets or imported profile data; no vendor lens-profile service is a runtime dependency.

### 9. Panel group — Geometry / transform

[STU-RAW-010] The Geometry group MUST provide:

- Automatic perspective/level correction (an upright-class analysis) with off, level (auto-horizon), vertical, full, and guided modes.
- Guided-mode reference lines the operator/model draws to define the correction.
- Manual transform sliders: Vertical, Horizontal, Rotate, Aspect, Scale, and X/Y Offset.
- A Constrain-Crop option that trims exposed borders produced by the correction.

Geometry corrections are parametric and reversible like every other develop group.

### 10. Panel group — Effects (grain and vignette)

[STU-RAW-011] The Effects group MUST provide:

- Film-grain synthesis: Amount, Size, and Roughness.
- A post-crop vignette: Amount, Midpoint, Roundness, Feather, and Highlight Priority, with style selection.

These are develop-time creative effects distinct from the Optics lens-vignette *removal* of [STU-RAW-009]; both MUST coexist on the same primitive without ambiguity.

### 11. Panel group — Calibration and process version

[STU-RAW-012] The Calibration group MUST expose:

- The develop process version (the versioned rendering model that governs demosaic and adjustment math) as a first-class field.
- Shadow tint calibration.
- Red, Green, and Blue primary hue and saturation calibration.

Changing the process version is an explicit, receipted operation because it alters reproducibility ([STU-RAW-001]); imported documents MUST record their originating process version and MUST NOT be silently re-rendered under a newer version without an operator/model-visible migration receipt.

### 12. Camera profiles, creative profiles, and preset browser

[STU-RAW-013] The pipeline MUST provide:

- A profile stage applied before slider edits: camera-matching and neutral/standard render profiles (`camera profiles`) and look/creative profiles (`creative profiles`), selectable from a profile registry.
- A preset browser with hover-preview and a preset-amount/intensity slider, plus user-saved presets.
- Snapshots: named develop-state versions that can be reapplied later.
- Default-settings management: per-camera-model, per-serial-number, and per-ISO default develop settings, and a develop output-profile (ICC) selection for the developed result.
- Per-adjustment and per-panel add / delete / reset-to-default preset management with active toggles.

Profiles and presets are Studio assets reusing the `StudioSwatch`/`StudioColorProfile`/style-registry machinery of 14.8/14.10; a source suite's bundled profile pack is provenance, not a Studio-shipped asset name.

### 13. Masking system (all mask types, intersect/subtract)

[STU-RAW-014a] Local adjustment MUST be expressed through `StudioMask` (the same masking primitive used by 14.4/14.9), not a raw-only mask type. The develop masking system MUST support the full mask-source set:

- Manual/geometric sources: Linear gradient (optionally bidirectional with flip orientation); Radial gradient (oval, inside/outside, with feather); and Brush (Size, Feather, Flow, Density, and Auto-Mask color confinement).
- Range sources: Color range (up to a multi-sample set with a Refine/breadth slider, usable as an intersect refinement); Luminance range (with a Select-Luminance slider and a luminance-map view); and Depth range (using embedded depth data with a Select-Depth slider and a depth-map view).
- AI-assisted sources (optional model adapter per [STU-RAW-014], degrading to the manual sources when no adapter is present): Subject, Sky, Background, Landscape components, Objects (from a rough brush stroke or drawn rectangle), and People (per-person, with per-person component sub-masks such as skin, hair, and clothing).

[STU-RAW-014b] Masks MUST compose and manage:

- Combine: add, subtract, and intersect between any mask sources; invert (including duplicate-and-invert).
- Manage: duplicate, rename, hide (eye toggle), and delete.
- Review: a customizable mask overlay display (color overlay, color-on-black/white, image-on-black/white, white-on-black, and equivalent review modes).
- Local edit: each mask carries its own full develop slider stack (a mask-local `StudioRawDevelop` sub-stack), and mask-local slider stacks MUST be saveable as local-adjustment presets re-applied with an amount control.

Intersect/subtract semantics and the mask-component model are the canonical `StudioMask` semantics of 14.23; the develop pipeline does not fork them.

### 14. Enhance — denoise, raw details/demosaic, super-resolution

[STU-RAW-014] Enhance operations MUST be split into a native deterministic tier and an optional ML-adapter tier:

- (a) Native deterministic primitives: the demosaic / raw-details refinement (edge rendition, color rendering, and artifact improvement at native resolution for Bayer/X-Trans sources) and integer/linear super-resolution upscale MUST be available on the deterministic `studio-engine` path and MUST produce reproducible output under a fixed process version.
- (b) ML-model-backed Enhance — AI denoise, AI raw-details, AI super-resolution, and the AI mask sources of [STU-RAW-014a] — MUST be implemented only as an optional `StudioModelAdapter` (14.23) with a local model preferred and no required cloud/account dependency. When no adapter is installed, the pipeline MUST fall back to the deterministic tier and surface a capability receipt rather than failing.
- (c) Re-editability and lifecycle: an Enhance result MUST remain re-editable. An update/flatten lifecycle MUST let AI results be refreshed when models change and flattened to a baked buffer with a documented reset path back to the editable state, and every Enhance run MUST emit a receipt naming the adapter, the model identity, and whether it ran locally.

### 15. Local repair tools within develop

[STU-RAW-015] The pipeline MUST provide non-destructive crop and straighten (aspect, angle, rotate, flip), a heal/clone spot-removal tool with source-point control, and a red-eye/pet-eye correction (pupil size, darken). These are develop-scoped operations on `StudioRawDevelop` and reuse the raster retouch primitives of 14.4 where the capability is shared per [STU-DOC-004]; they are not a parallel retouch implementation.

### 16. Workflow options and re-editable linkage to the raster document

[STU-RAW-016] Workflow options MUST configure how a developed raw is handed to the `StudioDocument`:

- Output color space (`StudioColorProfile`), bit depth, and output pixel dimensions/resolution.
- Open behavior, including whether the developed raw opens flat or as a re-editable placed object.

The pipeline MUST support raw-as-re-editable-object: a developed raw MAY be placed into the raster document (14.4) as a re-editable placed object (`StudioLayer` of the placed/smart kind) whose `StudioRawDevelop` settings remain editable in place — reopening the object returns to the full develop surface with all parameters, masks, and process version intact. This is the deduped equivalent of "open raw as smart object" and the Affinity develop-then-embed flow, expressed through the one linked-object primitive of 14.4; there is no separate raw-embed format.

[STU-RAW-017] Save/output of a developed raw MUST route through `StudioExportRecipe` (14.13) for derived deliverables (DNG/TIFF/PNG/JPEG and layered document formats) with format-specific options, and MUST NOT invent a raw-develop-only export path that bypasses the 14.13 export contract. Multi-image apply (copy develop settings from one raw to a selection of others, paste-settings, previous, and preset apply across a filmstrip selection) MUST operate on the canonical selection set, not on the visible/loaded subset.

### 17. Model steerability, headless operation, and validation obligation

[STU-RAW-018] Every `StudioRawDevelop` control group, mask source, Enhance operation, profile/preset action, and workflow option MUST be exposed as a typed, model-steerable command with a stable identifier and MUST be observable through the Studio visual-debug/inspection surface (Argus) per 14.16; all raw develop operations MUST run headless and quiet under the headless/quiet law of 14.20 (no foreground window, no focus steal, bounded and observable), and each MUST carry a dual-audience UserManual entry per 14.22 covering purpose, inputs/outputs, and failure/recovery. A model-authored develop edit MUST pass the sandbox -> `StudioValidationDescriptor` -> `PromotionGate` lifecycle of [STU-ARC-005] before it changes authority rows; model confidence never bypasses the gate.

---

## 14.13 Import/Export & File-Format Compatibility

Studio's interoperability posture is deduped into two canonical primitives — `StudioImportProfile` (schema id `hsk.studio.import_profile@1`) and `StudioExportRecipe` (schema id `hsk.studio.export_recipe@1`), both members of the [STU-DOC-002] primitive set and field-owned by 14.23 — plus one normative format compatibility matrix. Every per-suite import path, export dialog, save surface, and format matrix recorded in the research provenance (the 410-record / 38-family compatibility registry, the 15-target proprietary-format fixture plan, and the Photoshop/Affinity import-export modalities) collapses into these two primitives and this one matrix per [STU-SECTION-003]. A source suite's dialog name (Export As, Save for Web, Export Persona, Save a Copy) is never a Studio command name; Studio ships Handshake-native import/export commands that are two projections of the same primitives for the operator UI and the model command surface per [STU-DOC-004].

[STU-IO-001] NATIVE-FORMAT DECLARATION. Studio MUST NOT invent a new interchange document format to replace the existing creative formats. The native Studio document is the unified `StudioDocument` (14.3) persisted through Handshake document/project storage under SurrealDB/EventLedger authority ([STU-ARC-003]/[STU-ARC-004]) with CRDT live state; it is not a new file-on-disk interchange format competing with PSD/AI/IDML/`.af`/`.fig`. This restates and is bound by [STU-OVR-002]: interoperability with the source formats is in scope as import / edit-preserve / export / round-trip; a replacement interchange format and any runtime dependency on the source applications are out of scope. Where a deliverable file must leave Studio, it leaves through a `StudioExportRecipe` into one of the matrix formats below, never through a bespoke Studio-only container.

---

### 1. StudioImportProfile / StudioExportRecipe primitives and the round-trip contract

[STU-IO-002] `StudioImportProfile` MUST define a typed, deterministic decode from a source format into `StudioDocument`/`StudioLayer`/`StudioMask`/`StudioTextStory`/`StudioColorProfile` primitives, and `StudioExportRecipe` MUST define a typed encode from those primitives into a target format. Together they MUST satisfy a four-part support contract stated per format in the matrix ([STU-IO-006]):

- **import** — open or place the source into Studio primitives.
- **edit-preserve** — mutate the document and re-persist without discarding still-representable source structure.
- **export** — encode Studio primitives into the target format.
- **round-trip** — import then export with a documented, bounded fidelity envelope.

Each direction a format claims MUST be backed by fixtures ([STU-IO-011]) and MUST NOT be asserted from capability memory.

[STU-IO-003] Every import and export MUST be `StudioColorProfile`-explicit and unit-explicit per [STU-DOC-003]: the decode step is the conversion boundary; there is no implicit device color and no mixed-unit geometry crossing the boundary. Import and export are authority-affecting operations and MUST emit EventLedger events (`studio.document` / `studio.export`) with a `KernelActor` distinguishing model-authored from operator-authored interop per [STU-ARC-003].

### 2. Compatibility scope, fixtures, receipts, and recovery — the promotion gate

[STU-IO-004] NORMATIVE GATE. A format import or export feature MUST NOT be promoted (marked parity-complete or shippable) unless it declares ALL of the following five artifacts:

- (a) **compatibility scope** — an explicit statement of which structures are preserved, transformed, rasterized, or unsupported.
- (b) **fixtures** — representative fixtures for every supported app-and-direction it claims.
- (c) **round-trip expectations** — the documented, bounded fidelity envelope after import->export.
- (d) **unsupported-feature diagnostics/receipts** — a machine-readable receipt enumerating every source feature that could not be represented, was degraded, or was baked, emitted at decode/encode time and surfaced to the operator and model.
- (e) **recovery behavior** — deterministic, non-destructive handling of malformed, partial, truncated, password-protected, or version-newer inputs (fail closed with a receipt, never a silent partial write to authority).

A format that cannot meet all five is `unsupported` or `optional adapter`, not promoted.

[STU-IO-005] Unsupported-feature handling MUST be lossless-of-intent where the format is a native round-trip target:

- Vendor-private structures that Studio cannot semantically model MUST be preserved as an opaque preservation blob carried with the document.
- The preservation blob MUST be re-emitted on export where the target is the same family, so a round-trip does not silently drop unmodeled structure.
- A receipt MUST record what was preserved-opaque versus what was semantically imported into editable Studio primitives.
- Preservation blobs are authority data under the no-SQLite rule ([STU-OVR-003]) and MUST NOT be stored in any SQLite cache.

### 3. Format compatibility matrix (normative rows)

[STU-IO-006] The following matrix is NORMATIVE. Each row is one format family with its Studio support direction and a fidelity note. Direction tokens: **I** = import, **EP** = edit-preserve, **X** = export, **RT** = round-trip. Fidelity tokens: **preserved** (structure kept as editable Studio primitives), **transformed** (converted to the nearest Studio primitive with a documented mapping), **rasterized** (flattened to pixels), **preservation-blob** (opaque vendor-private data carried and re-emitted), **unsupported** (declared out, receipt emitted). Posture: **NRT** = native/local round-trip target; **SOX** = source-observable import/export target; **ADPT** = optional provider/cloud adapter or omitted. Every claimed direction is fixture-gated per [STU-IO-004].

| # | Format (Studio row) | Ext / family | Category | Direction | Fidelity note | Posture |
|---|---|---|---|---|---|---|
| 1 | Layered raster document | `.psd` | Raster | I·EP·X·RT | Layers, groups, masks, adjustment/live-filter layers, linked/embedded objects, blend modes preserved; unrepresentable features baked with receipt | NRT |
| 2 | Large layered raster document | `.psb` | Raster | I·EP·X·RT | As row 1 beyond standard raster/size ceilings; large-document container preserved | NRT |
| 3 | Tagged image raster | `.tif`/`.tiff` | Raster | I·EP·X·RT | Multi-layer (where present), alpha, high bit-depth, embedded ICC preserved; per-format compression options exposed | SOX |
| 4 | Portable network graphics | `.png` | Raster | I·X | Flat raster + alpha; 8/16-bit; no layers (rasterized on export) | SOX |
| 5 | JPEG | `.jpg`/`.jpeg` | Raster | I·X | Flat lossy raster; metadata/ICC preserved; quality options exposed | SOX |
| 6 | Graphics interchange | `.gif` | Raster | I·X | Indexed color + animation frames; color-table editable | SOX |
| 7 | WebP | `.webp` | Raster | I·X | Raster + alpha + animation; lossy/lossless options | SOX |
| 8 | HEIC / HEIF | `.heic`/`.heif` | Raster | I·X | Raster + alpha (codec-gated); depth/aux channels transformed; codec availability surfaced as capability receipt | SOX |
| 9 | OpenEXR / Radiance HDR | `.exr`/`.hdr` | Raster (HDR) | I·EP·X·RT | High-dynamic-range float; multi-channel/multi-part (EXR) preserved; scene-linear color preserved | SOX |
| 10 | JPEG-XL | `.jxl` | Raster | I·X | Raster + alpha, HDR, lossless option; extends registry raster coverage | SOX |
| 11 | FITS (scientific) | `.fits`/`.fit` | Raster (sci) | I·X | Float scientific raster preserved as raster layer; astronomy/WCS metadata transformed to metadata; extends registry | SOX |
| 12 | Additional raster interchange | `.bmp`/`.tga`/`.dcm`/`.cin`/`.pbm` + `format.unspecified` long-tail | Raster | I·X | Flat raster in/out; long-tail interchange handled by the same [STU-IO-004] gate and receipts | SOX |
| 13 | Camera raw (mosaic) | proprietary raw family (`.cr2`/`.cr3`/`.nef`/`.arw`/`.raf`/…), `format.raw` | Camera raw | I·EP | Decoded + developed via `StudioRawDevelop` (14.12); develop settings live in EventLedger, not a sidecar; export via derived deliverables | SOX |
| 14 | Digital negative | `.dng` | Camera raw | I·EP·X | Documented raw container; embedded develop metadata translated to `StudioRawDevelop`; Enhance-DNG output supported | SOX |
| 15 | Vector illustration document | `.ai` | Vector | I·EP·(X via PDF-compatible) | Paths, artboards (as layers/pages), text, gradients transformed to Studio vector primitives; PDF-compatible stream read; unrepresentable art preservation-blobbed | NRT |
| 16 | Vector template | `.ait` | Vector | I·X·RT | Illustration template round-trip target | NRT |
| 17 | Scalable vector graphics | `.svg`/`.svgz` | Vector | I·EP·X·RT | Paths, text, gradients, symbols preserved; filters/effects transformed; SVGZ = gzip variant of same row | SOX |
| 18 | Encapsulated PostScript | `.eps` | Vector | I·X | Legacy vector interchange transformed; rasterized where structure is opaque | SOX |
| 19 | PostScript | `.ps` | Vector | I·X | PostScript stream transformed/rasterized on decode | SOX |
| 20 | Portable document format | `.pdf` | Vector/doc | I·EP·X | Multi-page import (pages as layers/spreads); vector+text+raster; text preserved on export where not rasterized | SOX |
| 21 | CAD drawing | `.dwg` | Vector (CAD) | I·X | CAD vector with drawing-scale support; transformed to Studio vector; non-vector CAD entities receipted | SOX |
| 22 | CAD interchange | `.dxf` | Vector (CAD) | I·X | CAD interchange transformed with drawing scale; unsupported entities receipted | SOX |
| 23 | Legacy vector import | Freehand 10/MX family | Vector | I | Legacy vector transformed; multi-page concatenated; text import unsupported (receipt) | SOX |
| 24 | Layout interchange | `.idml` | Layout | I·EP·X·RT | Pages, spreads, frames, text stories, styles transformed to Studio layout primitives; round-trip target; binary `.indd` out (row 25) | NRT |
| 25 | Native layout document (binary) | `.indd` | Layout | I·EP | Binary layout imported where structure is readable; unreadable structure preservation-blobbed with receipt; export goes via IDML/PDF | NRT |
| 26 | Prepress PDF | PDF/X (`.pdf`) | Layout (prepress) | X | Standards-constrained prepress PDF; color conversion, marks, bleed preserved to the chosen PDF/X standard | SOX |
| 27 | Tagged / accessible PDF | tagged `.pdf` | Layout (a11y) | X | Structure tags, reading order, alt text preserved | SOX |
| 28 | Reflowable/fixed publication | `.epub` | Layout | X | Reflowable or fixed-layout export; interactive/EPUB features transformed; unsupported receipted | SOX |
| 29 | Packaged/collect output | package folder | Layout | X | Document + linked assets + fonts + profiles collected into one portable folder | SOX |
| 30 | Affinity photo-native | `.afphoto` | Affinity native | I·EP·X·RT | Raster layers, adjustments, live filters, masks transformed; vendor-private structure preservation-blobbed | NRT |
| 31 | Affinity designer-native | `.afdesign` | Affinity native | I·EP·X·RT | Vector network, layers, color, typography transformed; vendor-private preservation-blob | NRT |
| 32 | Affinity publisher-native | `.afpub` | Affinity native | I·EP·X·RT | Layout, frames, styles, linked assets transformed; vendor-private preservation-blob | NRT |
| 33 | Affinity unified-native | `.af` | Affinity native | I·EP·X·RT | Unified vector/pixel/layout document (single-doc successor to the trio) transformed into one `StudioDocument`; preservation-blob for unrepresentable structure | NRT |
| 34 | Affinity template | `.aftemplate` | Affinity native | I·X | Reusable template round-trip target across domains | NRT |
| 35 | Figma design document (local copy) | `.fig` | Figma family | I·EP·X·RT | Frames, components, variants, variables, auto-layout, prototype flows transformed; **local-copy** round-trip; cloud sync = ADPT | NRT |
| 36 | Whiteboard document (local copy) | `.jam` | Figma family | I·EP·X·RT | Whiteboard/diagram objects (14.15) transformed; local-copy round-trip | NRT |
| 37 | Slides/deck document (local copy) | `.deck` | Figma family | I·EP·X·RT | Slide layouts/decks transformed; local-copy round-trip | NRT |
| 38 | Buzz document (local copy) | `.buzz` | Figma family | I·EP·X·RT | Marketing/asset document transformed; local-copy round-trip | NRT |
| 39 | Sites document (local copy) | `.site` | Figma family | I·EP·X·RT | Site layout/design transformed; local-copy round-trip; publishing = ADPT | NRT |
| 40 | Make document (local copy) | `.make` | Figma family | I·EP·X·RT | Make/app document transformed; local-copy round-trip | NRT |
| 41 | Sketch document | `.sketch` | Figma-adjacent | I | Frames, symbols, styles transformed on import; export not claimed | SOX |
| 42 | Fonts | `.otf`/`.ttf`/`.ttc`/variable/OpenType-SVG | Fonts | I (embed/reference) | Glyphs, OpenType features, variable axes, SVG/emoji glyphs preserved; missing-font detection + replacement diagnostics | SOX |
| 43 | Color profiles | ICC/ICM (`.icc`/`.icm`) | Profiles | I·(embed)·X | ICC input/output profiles preserved and embeddable on export; explicit `StudioColorProfile` binding | SOX |
| 44 | Linked / placed assets | any placeable source | Links | I (link)·X (package) | External sources referenced as linked objects with modified/missing status + update-all; collected on package | SOX |
| 45 | Data-merge sources | `.csv`/`.xls`/`.xlsx`/`.xml` | Data | I | Tabular/structured data imported as merge/variable sources; not a rendered document | SOX |
| 46 | Web / markup output | `.html`/`.css` (+ slices) | Web | I·X | Sliced/continuous web output and style tokens transformed; HTML export from slices; CSS style import transformed | SOX |
| 47 | Office document interchange | `.pptx` | Office | I·X | Slides/shapes transformed to Studio layout primitives; unsupported office features receipted | SOX |

[STU-IO-007] Matrix row-law: the matrix in [STU-IO-006] is the complete deduped Studio format surface. A format present in the source registries (38 families across 410 records) that is not listed above MUST be treated as covered by the nearest row of its category plus the `format.unspecified` long-tail row (row 12) and is governed by the same [STU-IO-004] gate; adding a new format is adding a matrix row with its five gate artifacts, not a new import/export subsystem.

### 4. Native round-trip families and the preservation-blob law

[STU-IO-008] The **NRT** rows are the highest-fidelity interop targets and MUST implement the preservation-blob behavior of [STU-IO-005] — everything Studio can semantically model becomes editable Studio primitives; everything it cannot is carried opaque and re-emitted on same-family export, with a receipt separating the two. The NRT families are:

- Layered raster: `.psd` / `.psb`.
- Vector illustration: `.ai` / `.ait`.
- Layout: `.idml` / `.indd`.
- Affinity native: `.af` (unified) / `.afphoto` / `.afdesign` / `.afpub` / `.aftemplate`.
- Figma local-copy: `.fig` / `.jam` / `.deck` / `.buzz` / `.site` / `.make`.

The Figma family is explicitly a **local-copy** round-trip: Studio imports/exports a local document copy with no cloud/account requirement; live cloud collaboration, provider sync, and provider publishing are optional adapters ([STU-IO-013]), never a runtime dependency, consistent with [STU-OVR-002].

### 5. Fonts, color profiles, and linked assets

[STU-IO-009] Fonts, profiles, and links MUST each be handled as first-class interop inputs:

- Fonts: import MUST preserve OpenType features, variable-font axes, and OpenType-SVG/emoji glyphs (14.7) and MUST emit missing-font diagnostics with a replacement path on document open; fonts are embedded or referenced per document policy, never silently substituted without a receipt.
- Color profiles: ICC/ICM profiles MUST be importable, embeddable on export, and bound explicitly via `StudioColorProfile` (14.8), with no implicit device color per [STU-DOC-003].
- Linked/placed assets: MUST track modified/missing status with an update-all command and MUST be collectable into a package (rows 29 and 44).

### 6. Unified export surfaces (StudioExportRecipe)

[STU-IO-010] All export surfaces recorded across the source suites MUST unify into `StudioExportRecipe` — one primitive, many recipes — with NO separate per-surface export subsystem. The unified recipe MUST cover:

- Single-target export: per-document, per-layer, and per-artboard export — the deduped "Export As" / "Save a Copy" derived-format matrix, plus a one-click "Quick Export" default with a preferred format and location.
- Export-for-screens / slices: independent named export regions drawn from areas, layers, or artboards, each with its own format and scale set; multi-scale variants (1x/2x/3x…) with scale-suffix filename tokens; and continuous/automatic re-export when slice content changes.
- Legacy web optimizer: 2-up/4-up preview, GIF/JPEG/PNG-8/PNG-24 settings, dither, transparency matte, color-table editing, image-size, and slice-aware HTML/image output.
- PDF export: option groups for preset/standard (including PDF/X), general/preserve-editing, compression/downsampling, output color conversion, and security, with named reusable PDF presets.
- Batch and packaging: export of each layer or artboard to individual files, and artboards-to-PDF packaging.
- Video/animation render: timeline to encoder presets or image sequences with range, size, and frame-rate options.
- Print output: printer setup; Studio- vs printer-managed color with rendering intent and soft-proof; position and scaled print size; printing marks; and bleed/background/border functions.

Each is a `StudioExportRecipe` configuration; export area MUST resolve against the canonical document/selection/slice set, not the currently rendered/visible subset.

### 7. Fixtures and round-trip expectations

[STU-IO-011] Fixture law: every supported app-and-direction in the matrix MUST have representative fixtures and receipts; a format is not parity-complete until its fixtures pass and its unsupported features are documented in receipts and the Studio UserManual (14.22). The proprietary/native round-trip families MUST be covered by the native fixture plan of 15 native targets:

- By family: Affinity (5), Figma family (6), Illustrator (3), InDesign (2), Photoshop (2).
- Each target declares all four required support directions — import, edit-preserve, export, round-trip.
- Each target declares its schema-public status, which governs how much is semantically imported versus preservation-blobbed: documented interchange XML with source-behavior fixtures; partly-documented native/large-native document; vendor-private native document; vendor-private-or-PDF-compatible native document; vendor-private local-copy document; and vendor-private template document.

Round-trip expectations MUST be stated as a bounded fidelity envelope, not "identical," and MUST be verified by fixture comparison rather than asserted from capability memory.

### 8. Recovery and diagnostics

[STU-IO-012] Import/export MUST fail closed and never write a silent partial or corrupt result to authority:

- Malformed, truncated, version-newer, or password/DRM-protected inputs MUST produce a deterministic diagnostic receipt (what failed, at which decode stage, and the safe fallback) and MUST leave existing authority unchanged.
- Every lossy or transforming operation — rasterization, baking, preservation-blob, text-rasterized-on-export, and unsupported-feature drop — MUST emit an operator- and model-visible receipt at the time it occurs.
- Receipts are the audit surface for interop fidelity and are EventLedger-bound per [STU-ARC-003].

### 9. Provider / cloud formats and account-gated surfaces

[STU-IO-013] Any format or export target that requires a provider account, cloud service, or subscription MUST be marked **optional adapter (ADPT) or omitted** and MUST NOT be a runtime dependency of any core matrix row per [STU-OVR-002]. Account-gated surfaces include:

- Vendor cloud-document containers and their version history.
- Cloud asset libraries and library-linked assets.
- Provider share-for-review and provider publishing.
- Cloud media-encoder services for video render.
- Provider site/app publishing targets.

The local-first equivalent (local document history 14.19, local fixtures, local export) is primary; the cloud path is an installable `StudioModelAdapter`/provider adapter that degrades cleanly to local behavior with a capability receipt when absent.

### 10. Model steerability, headless operation, and validation obligation

[STU-IO-014] Every `StudioImportProfile` and `StudioExportRecipe` action MUST be a typed, model-steerable command with a stable identifier, observable through the Studio visual-debug/inspection surface (Argus) per 14.16, and MUST run headless and quiet under 14.20 (no foreground window, no focus steal, bounded and observable) so batch import/export is safe for parallel operator/model co-work. Each format row and export surface MUST carry a dual-audience UserManual entry per 14.22 stating its compatibility scope, fixtures, round-trip envelope, unsupported-feature receipts, and recovery behavior. A model-authored import/export that changes document authority MUST pass the sandbox -> `StudioValidationDescriptor` -> `PromotionGate` lifecycle of [STU-ARC-005]; the fixture/receipt gate of [STU-IO-004] is part of that validation and is not optional regardless of model confidence.


## 14.14 Automation, Scripting & Plugin/API Surface

This sub-section is the normative operator- and plugin-facing automation surface for Studio: how operators, third-party extensions, and model plugins script, batch, and extend Studio without hand-editing documents. It is distinct from — and built on top of — the model-steerable command surface (14.16) and the propose-work system (14.18), which are Handshake-native and describe how the *kernel's own model lanes* drive Studio. 14.14 exposes that same machinery outward to operator scripts and installable plugins.

The defining dedup for this domain is a single collapse. Each source suite ships its own scripting object model: one suite's UXP/JavaScript DOM (app/document/layer/action/`batchPlay` descriptor interface), two suites' ExtendScript/UXP DOM class trees (Application, Document, Artboard/Spread, Layer, PageItem, PathItem, TextFrame/Story, Table, Style collections, ExportOptions), and one suite's Plugin API scene-graph (DocumentNode, PageNode, FrameNode, VectorNode, TextNode, InstanceNode, factory methods, events, manifest). The cross-app overlap map records these as one deduped primitive (`overlap_key: "application"`, `one_studio_primitive_multiple_source_variants`, automation domain). Studio therefore does **not** reimplement five scripting DOMs. It exposes **one typed Studio command contract** over the canonical primitive set (`StudioDocument`, `StudioLayer`, `StudioSelectionSet`, `StudioVectorPath`, `StudioTextStory`, and the rest of the 14.3 set) — the *same* command contract the model lanes call per 14.16 — and layers an actions/macro/batch surface, a data-driven-graphics surface, a find/change surface, and a sandboxed plugin surface on top of it. A source suite's scripting-language name, DOM class name, plugin-runtime name, or marketplace name is never a Studio surface name.

Affinity's deliberate no-user-scripting posture (macros + batch only, no scripting API) is recorded here strictly as a source data point of what Studio does **not** do: Studio HAS a first-class native automation and scripting API. Its macro/batch layer is one projection of that API, not a substitute for it.

---

### 1. The Unified Studio Command API (single automation surface)

[STU-AUT-001] Studio MUST expose exactly ONE typed command contract as its automation surface. Every automatable Studio operation — document, layer, selection, path, text, table, color, effect, layout, component, export, and history operation — is a `StudioCommand`: a typed, schema-described, named operation over the canonical primitive set of 14.3. There is no second, parallel automation API for operators or plugins; the operator script surface, the plugin surface, and the model-lane surface (14.16) are three callers of the one contract, not three contracts.

[STU-AUT-002] Every `StudioCommand` MUST be (a) **typed** — its input and output are `schemars::JsonSchema`-deriving types so the MCP `inputSchema` and the operator/plugin type stubs are generated from one source; (b) **dry-runnable** — invocable in a preview mode that computes and returns the resulting `StudioEditProposal` diff without mutating authority; (c) **receipted** — every non-dry-run invocation emits an EventLedger receipt under a `studio.*` `event_family` carrying the `KernelActor` variant (operator, model adapter, or named plugin) so authorship is attributable; and (d) **undoable** — it produces a `StudioHistoryEntry` and participates in per-file history/undo/revert-of-undo per 14.19. A command that cannot satisfy all four properties is not admissible to the command contract.

[STU-AUT-003] A command invoked by any non-operator caller that mutates document authority (a plugin, an external API client, or a model lane) MUST NOT write authority rows directly. It MUST route through the kernel sandbox → `StudioValidationDescriptor` catalog (14.24) → `PromotionGate` lifecycle exactly as a model edit does ([STU-ARC-005]); dry-run and read-only commands are exempt. Operator-initiated commands from the native UI follow the same promotion-equivalence lifecycle as any operator edit. Automation never earns a bypass by virtue of being scripted.

[STU-AUT-004] Commands MUST be composable into an ordered **command sequence** (`StudioCommandBatch`) that executes as one transactional unit with one aggregate `StudioEditProposal`, one promotion decision, and one coalesced `StudioHistoryEntry` group, so that a recorded macro, a batch job, or a plugin's multi-step edit is a single reviewable, single-undoable operation rather than N unrelated authority writes.

[STU-AUT-005] The command contract is versioned and capability-namespaced. Each command declares the capability it requires (e.g. `studio.document.write`, `studio.export`, `studio.filesystem.read`); the caller's granted capability set (operator, or a plugin's manifest-declared and consent-granted set per Section 7 below) gates which commands resolve. An unknown or ungranted command MUST fail closed with a typed error, never silently no-op.

The command contract is organized into stable command families. This table is the normative family set; it is the deduped union of the five source scripting DOMs' object domains, mapped onto canonical primitives.

| Command family | Canonical primitive(s) operated on | Deduped source-DOM provenance |
|---|---|---|
| `document` | `StudioDocument`, `StudioArtboard`, `StudioPageSpread` | app/document/documents root, open/save/export/print, spreads/pages |
| `layer` | `StudioLayer`, `StudioLayerGraph`, `StudioMask` | Layer/Layers, PageItem/GroupItem, FrameNode/GroupNode/SectionNode |
| `selection` | `StudioSelectionSet` | selection APIs, currentPage selection, bulk edit |
| `vector` | `StudioVectorPath`, `StudioVectorNetwork` | PathItem/PathPoint/CompoundPath, VectorNode (network + paths), boolean-operation nodes |
| `text` | `StudioTextStory`, `StudioTypeStyle` | TextFrame/TextRange/Story/Characters/Paragraphs, TextNode range styling, Story/Text hierarchy |
| `table` | `StudioLayer` (table kind) | Table/Row/Column/Cell scripting domain |
| `color` | `StudioSwatch`, `StudioGradient`, `StudioPattern`, `StudioColorProfile` | Swatch/SwatchGroup/Gradient/Spot/Pattern color-asset model |
| `effect` | `StudioEffectStack`, `StudioAdjustment`, `StudioLiveFilter`, `StudioBlendMode` | brush/graphic-style application, effect/adjustment domains |
| `design_system` | `StudioComponent`, `StudioComponentInstance`, `StudioVariable`, `StudioVariableCollection`, `StudioStyleRegistry` | Symbol/SymbolItem, ComponentNode/InstanceNode, variables API, style collections |
| `layout` | `StudioLayoutGrid`, `StudioConstraint`, `StudioAutoLayout` | layout grids, auto-layout, constraints |
| `export` | `StudioExportRecipe`, `StudioImportProfile` | ExportOptions/SaveOptions classes, `exportAsync`, per-layer export settings, slice regions |
| `history` | `StudioHistoryEntry` | undo/redo, versions surface |
| `app` | application/preferences scope | Application root, preferences, doScript, editorType/mode, viewport |

### 2. Scripting object model exposure

[STU-AUT-006] Studio MUST expose the canonical primitive set as a typed read/write object model over `StudioDocument` — the "scripting DOM" equivalent — without inventing a parallel object hierarchy. Reads project canonical primitive fields; writes are expressed as commands from Section 1 and inherit the dry-run/receipt/undo/promotion properties of [STU-AUT-002]/[STU-AUT-003]. There is no second in-memory document model exposed to scripts; the object model IS a typed view over the single `StudioDocument` that the engines and the typed SurrealDB authority object share ([STU-DOC-001]).

[STU-AUT-007] The object model MUST provide a low-level typed descriptor path — the deduped equivalent of a descriptor/`batchPlay`-style interface — for operations not yet surfaced as a named high-level command, so no capability is unreachable from automation. The descriptor path is still typed (a `StudioCommand` variant), still receipted, still undoable, and still promotion-gated; it is a lower-level entry to the same contract, never an unaudited escape hatch.

[STU-AUT-008] Scripts, plugins, and model lanes MUST address document objects by the stable prefixed IDs of the canonical primitives ([STU-ARC-004], e.g. `SLYR-*`), and MAY additionally attach operator-assigned string labels to objects so automation can locate specific objects by intent (the deduped equivalent of a script-label facility). Object addressing MUST NOT depend on layer position, name text, or z-order alone, so a macro or plugin re-run resolves the same object deterministically.

[STU-AUT-009] Any command that mutates document authority MUST execute under the transactional/modal-gated boundary of the sandbox → validation → `PromotionGate` lifecycle ([STU-AUT-003]); this is the deduped equivalent of a source runtime's "modal execution" gate around every mutation. Reads and dry-runs need no gate; a mutation attempted outside the gate is rejected, never applied optimistically.

The object-model domains below are the normative deduped exposure surface. Each row collapses the equivalent per-suite DOM domains into one Studio domain over canonical primitives.

| Object-model domain | Exposes (read/write) | Canonical primitive backing |
|---|---|---|
| Application/session | open documents, active document, preferences, run mode, editor mode, viewport center/zoom | app scope + `StudioDocument` set |
| Document | dimensions, resolution, unit, color profile, page/artboard tree, crop/resize/rotate/flatten | `StudioDocument`, `StudioArtboard`, `StudioPageSpread` |
| Layer / node graph | kind, name, opacity, blend mode, z-order, parent/child traversal, lock/visibility, scale/transform | `StudioLayer`, `StudioLayerGraph`, `StudioMask` |
| Path / vector network | anchors, handles, segments, regions, winding, boolean membership, compound paths | `StudioVectorPath`, `StudioVectorNetwork` |
| Text story | characters, ranges, per-range font/size/fill/decoration/OpenType, threaded stories, styles | `StudioTextStory`, `StudioTypeStyle` |
| Table | rows, columns, cells, merges, cell fills/strokes, cell content | `StudioLayer` (table kind) |
| Color assets | swatches, swatch groups, gradients, gradient stops, spot inks, patterns | `StudioSwatch`, `StudioGradient`, `StudioPattern` |
| Design system | components, instances, overrides, swap/detach, variables, collections, modes, styles | `StudioComponent`, `StudioComponentInstance`, `StudioVariable`, `StudioVariableCollection`, `StudioStyleRegistry` |
| Placed / raster | linked and embedded placed assets, relink/embed, raster tiles | `StudioLayer` (placed_asset/raster kinds), `StudioRasterTile` |
| Export / slice / print | export recipes, per-layer/per-slice settings, render-to-bytes, typed print-job options (paper, marks, color management) | `StudioExportRecipe` |
| Long-document | books, XML elements/tags, hyperlinks, bookmarks, cross-references, index topics, sections | `StudioDocument` (long-document fields, 14.6) |
| Node data | per-node private and shared key-value metadata namespaced to a plugin, travelling with the document | `StudioLayer` metadata + plugin namespace (Section 7) |

### 3. Actions and macros (record / playback / conditional)

[STU-AUT-010] Studio MUST provide a native **action/macro** system: an operator records a live sequence of Studio commands into a named, reorderable macro (`StudioMacro`), plays it back over the current document, and re-records or edits individual steps. Because every UI edit is already a `StudioCommand` ([STU-AUT-001]), recording is capture of the emitted command stream, not a separate scripting translation layer. A recorded macro is a persisted `StudioCommandBatch` and replays under the transactional/undo semantics of [STU-AUT-004].

[STU-AUT-011] The macro system MUST support the deduped control surface below and MUST NOT require the operator to write code to use it. Macro steps are individually parameter-adjustable after recording, so a captured operation can be re-parameterized (the deduped equivalent of adjustable macro step parameters and change-settings-on-playback).

| Macro control | Behavior | Deduped source provenance |
|---|---|---|
| Record / stop / play | Capture the live command stream into a named macro; replay over the active document | actions panel record/play; macro record/replay |
| Per-step modal pause | Mark a step to pause playback and surface its parameter dialog for operator input | action modal controls; macro step parameters |
| Step enable / exclude / reorder | Toggle, drop, or reorder recorded steps without re-recording | per-step toggles, command exclusion, reordering |
| Insert message stop | Pause playback with an operator message and optional continue | Insert Stop |
| Insert command step | Embed an otherwise non-interactively-captured command by explicit selection | Insert Menu Item (non-recordable commands) |
| Insert path step | Record exact vector-path geometry into the macro so playback recreates it | Insert Path / record a path |
| Record tool strokes | Optionally capture brush/painting tool strokes for replay | record painting tools in actions |
| Conditional step | Branch to another macro based on a document condition (orientation, color mode, layer state) | conditional actions |
| Conditional mode change | Convert color mode only when the document matches a source-mode set, for batch safety | conditional mode change |
| Event-bound trigger | Bind a macro to a document/application event (open, new, save, export) | Script Events Manager; startup scripts; plugin events |
| Macro library | Store macros in named categories with import/export for reuse | macro library panel; action sets |

### 4. Batch processing (droplet / image-processor / batch-job equivalents)

[STU-AUT-012] Studio MUST provide a native **batch** surface that applies a command sequence or macro over a set of input files and writes results, without the operator writing code. The batch runner executes on the kernel Job Runtime as a headless, bounded, quiet job per the headless/quiet law (14.20): it MUST NOT pop foreground windows, steal focus, or hijack input while running, and MUST expose progress and per-file results through structured job state, not modal UI. Every processed file emits a receipt and each mutating output passes the sandbox → validation → `PromotionGate` lifecycle.

[STU-AUT-013] The batch surface MUST cover the deduped capabilities below as ONE runner with options, not as separate tools. A source suite's separate batch/droplet/image-processor/statistics/stack utilities collapse into one batch job type parameterized by source set, applied command sequence, output format set, and post-processing options.

| Batch capability | Behavior | Deduped source provenance |
|---|---|---|
| Batch-apply over a folder/set | Run a macro or command sequence across an input file set with override-open, suppress-dialogs, destination, filename template, and error log | Batch dialog; batch jobs |
| Portable batch action ("droplet" equivalent) | Export a batch job as a portable, re-runnable job artifact that processes a dropped/passed file set | droplets |
| Format-conversion batch ("image-processor" equivalent) | Batch-convert a folder to target formats with resize and profile conversion, with or without a prior recorded macro | Image Processor |
| Multi-format output | Emit one or more output formats per source in a single pass | batch-job multi-format output |
| Load set into one document | Load multiple files as layers of one document with optional auto-align and stacking | Load Files into Stack |
| Stack statistics render | Render a layer stack through statistical modes (mean, median, max, min, range, standard deviation, summation, and related) | Statistics stack modes |
| Utility batches | Contact-sheet, multi-page presentation, fit-to-size, split-scanned-images, and panorama-merge utilities as batch operations | Contact Sheet / PDF Presentation / Fit Image / Crop-and-Straighten / Photomerge |
| Watched-folder asset export | Continuously export assets from layers whose names carry export directives on change | Generate Image Assets |

### 5. Variables, data sets, and data-driven graphics (data merge)

[STU-AUT-014] Studio MUST provide a native **data-driven graphics** surface: an operator defines typed `StudioVariable` bindings on document fields — layer visibility, pixel/asset replacement, and text-string replacement — then binds a dataset (rows entered manually or imported from delimited text/CSV/XML) and previews or applies each row to produce a rendered variant. This deduplicates the three source suites' variables/data-sets and data-merge features into one binding-plus-dataset model over the canonical `StudioVariable`/`StudioVariableCollection` primitives; it MUST reuse those design-system variable primitives (14.10) rather than introduce a separate automation-only variable type.

[STU-AUT-015] The data-driven surface MUST support the deduped capabilities below. Batch generation of one output document (or exported asset) per dataset row MUST run on the batch runner of Section 4 and inherit its receipting, quiet-law, and promotion semantics.

| Data-driven capability | Behavior | Deduped source provenance |
|---|---|---|
| Variable definition | Bind visibility, pixel/placed-asset, and text-content variables to named document fields | variables and data sets; DOM Variable/Dataset; data-merge fields |
| Dataset binding | Attach a dataset of rows entered manually or imported from delimited text / CSV / XML | data-set import; XML dataset binding; data-merge source |
| Row preview | Preview any single dataset row applied to the document before committing | data-set preview |
| Batch expansion | Generate one document or exported asset per dataset row via the batch runner | export data sets as files; data-merge output |

### 6. Find / change and GREP as a batch operation

[STU-AUT-016] Studio MUST provide a native **find/change** command family that operates over the whole document (or a selection scope) as a single batch-style operation, deduplicating the source suites' multi-mode find/change into one command with typed modes. Find/change is a `StudioCommand` and therefore dry-runnable (preview all matches and the resulting diff), receipted, and undoable as one operation. Saved queries are persisted and reusable, and pattern-based text find/change powers pattern-driven text styles (14.7).

| Find/change mode | Matches / changes | Deduped source provenance |
|---|---|---|
| Text | Literal and metacharacter text search with format-attribute criteria and case/whole-word scope | Find/Change Text mode |
| Pattern (GREP) | Regular-expression text patterns with attribute find/change, driving pattern-based styles | Find/Change GREP mode; GREP styles |
| Glyph | Specific glyphs / OpenType variants by font and ID | Find/Change Glyph mode |
| Object | Object attributes (size, stroke, fill, effects) across page items | Find/Change Object mode |
| Color | Swatch / color usages across the document for global recolor | Find/Change Color mode |

### 7. Plugin / extension surface

[STU-AUT-017] Studio MUST provide a Handshake-native **plugin/extension model** so third parties and models can extend Studio, deduplicating the source Plugin API, Widget API, and Dev-Mode plugin concepts into one Studio plugin contract. A plugin declares a **manifest**: identity, the editor/document modes it targets, the command capabilities it requests, any network domains it needs (with stated reasoning), and its extension points (panels/UI, codegen, find/change/text-review participation, board widgets, and relaunch/quick-run entry points). Plugins operate on scene nodes only through the typed command contract of Section 1 — they receive the same typed object-model view ([STU-AUT-006]) and the same descriptor path ([STU-AUT-007]) that operators and model lanes use; there is no privileged private plugin document model.

[STU-AUT-018] Plugins MUST run under the kernel capability/consent gate system and MUST NEVER run unsandboxed. A plugin's manifest-declared capabilities are inert until consent-granted; the granted set gates every command the plugin can resolve ([STU-AUT-005]). Plugin execution is sandboxed (process-tier by default, as with model edits per [STU-ARC-005]); a plugin has no ambient filesystem, network, or authority access beyond its consent-granted capabilities. Document mutations authored by a plugin carry a `KernelActor` plugin identity in their receipts and pass the sandbox → validation → `PromotionGate` lifecycle. Plugin UI runs in an isolated surface with no direct access to the native shell's internals; it communicates with the sandboxed plugin logic through a typed message channel.

[STU-AUT-019] The plugin surface MUST support the deduped extension capabilities below. A plugin capability that the manifest does not declare and consent does not grant is unavailable at runtime and fails closed.

| Plugin capability | Behavior | Deduped source provenance |
|---|---|---|
| Manifest & mode targeting | Declare identity, targeted document/editor modes, and required capabilities | manifest editorType targeting |
| Capability & network gating | Request typed command capabilities and whitelisted network domains with reasoning; consent-gated | manifest networkAccess/permissions; capabilities |
| Scene-node access | Read/write scene nodes through the typed object model and descriptor path | node/document/page/frame/vector/text node APIs |
| Node factories | Create new primitives (paths, text, frames, shapes, images, board objects) as commands | `create*` node factory methods |
| Event subscription | Subscribe to run, selection-change, document-change, view-change, text-review, style-change, and drop events | plugin event subscription |
| Parameter quick-run | Declare typed input parameters gathered from the command surface for headless quick runs | parameter-driven quick-run |
| Relaunch entry points | Pin contextual re-run entry points onto specific nodes | relaunch buttons; setRelaunchData |
| Per-node plugin data | Store private and cross-plugin shared key-value data on nodes, travelling with the document | plugin data (private + shared) |
| Local client storage | Persist plugin-local key-value data separate from document data | clientStorage persistence |
| Codegen extension | Register languages and emit code for the current selection | codegen API / codegen plugins |
| Text-review extension | Participate in a text-review pipeline over document text | text-review capability |
| Board widget | Ship an interactive board widget with synced multi-user state (whiteboard mode, 14.15) | Widget API model; widgets on boards |
| Dev tooling | Typed stubs, hot reload, and a developer console for plugin development | plugin dev tooling (typings/hot reload/console) |
| Local registry distribution | Discover, install, update, and remove plugins from a local/native registry | plugins marketplace posture; private plugin distribution |

[STU-AUT-020] Studio's plugin distribution MUST be a local/native tool-and-extension registry, not a runtime dependency on any vendor's hosted marketplace or account. Importing a source suite's plugin ecosystem is out of scope; the manifest/capability/consent contract is the Studio-native model. This preserves the local-first, no-account posture of [STU-OVR-002].

### 8. External API, inspect / codegen, and MCP posture

[STU-AUT-021] Studio's canonical automation entry is **local-first**: the typed command contract exposed as MCP tools (auto-generated from the command `inputSchema`s per [STU-AUT-002]) is the primary programmatic surface, and it is the *same* surface the model lanes drive per 14.16 — Studio exposes its command contract over MCP natively rather than shipping a separate integration server. A hosted REST/webhook surface is an OPTIONAL adapter over the command contract for external, cross-process, or cloud callers; it is never required for automation, never the source of authority, and never a second command model. When present, the REST adapter's authentication maps to the kernel capability/consent gates ([STU-AUT-018]); its scopes are capability names, and its event/webhook taxonomy maps to `studio.*` EventLedger triggers.

[STU-AUT-022] Studio MUST provide a **developer inspect and codegen** surface — the deduped equivalent of a design-to-code inspect mode and its native MCP server — as a local, read-oriented projection of the command contract. It exposes: structured design context and generated code for the current selection or a node reference; the variables/tokens and styles used within a selection; a rendered screenshot of a selection for visual verification; and a component-to-code mapping concept that substitutes a team's real component code for auto-generated snippets. The prior art here is a design tool's Dev-Mode MCP server exposed to coding agents; Studio's equivalent is native (no hosted dependency) and is a projection of the one command contract, not a separate product mode. This inspect/codegen surface is read-oriented; any writeback it offers is a normal promotion-gated command.

| External/inspect capability | Behavior | Local-first posture |
|---|---|---|
| MCP command tools | The full typed command contract as MCP tools | Native, primary surface (same as 14.16); no hosted dependency |
| Document JSON tree read | Serialize a document (or a node subtree) as a typed JSON node tree | Native command; source-suite serialized-tree schema is a compatibility reference only |
| Server-side render | Render a node/selection to PNG/JPG/SVG/PDF bytes at a requested scale | Native `export` command / `StudioExportRecipe` |
| Inspect context / codegen | Structured design context + framework-shaped code for a selection | Native inspect projection; framework set is codegen-plugin-extensible |
| Token/variable defs | Variables, tokens, and styles used within a selection | Native read over `StudioVariable`/`StudioStyleRegistry` |
| Selection screenshot | Rendered screenshot of a selection for agent visual verification | Native render; ties to the visual inspection duty (14.16) |
| Component-to-code map | Map a component to real code implementations, overriding generated snippets | Native mapping concept over `StudioComponent` |
| Optional REST/webhooks | Files/nodes/images/comments/versions/components/variables endpoints and an event webhook catalog | OPTIONAL adapter over the command contract; capability-gated; maps to `studio.*` events |

[STU-AUT-023] Studio's local automation MUST NOT depend on any vendor account, hosted marketplace, hosted MCP endpoint, or subscription service at runtime ([STU-OVR-002]). External/hosted adapters are opt-in conveniences layered over the local-first command contract, and their absence MUST NOT disable any operator-, plugin-, or model-facing automation capability.

[STU-AUT-024] The inspect surface MUST additionally support the deduped **design-handoff aids** below as read-oriented projections; they reuse the diagnostics and collaboration substrates (14.16/14.17) rather than an automation-private mechanism, and none is a runtime dependency: persistent annotations combining free text with auto-updating property values on a node; persistent measurements between nodes that stay in sync as geometry changes; external dev-resource links attached to nodes (tickets/repos/docs); a ready-for-implementation status markable on sections/frames/components with a filtered review view; a focus view isolating one design with full inspect tooling; a version compare view diffing a node against an earlier version; a component playground for exercising a component's variants/properties without editing the file; an instance-vs-main comparison that flags drift and detached components; and full-resolution asset download of a selection without author-set export settings.

### 9. Asset browser, OS, and clipboard handoff

[STU-AUT-025] Studio MUST provide a local, native **asset browser** as the deduped equivalent of a companion asset-management application handoff (browse, previews, batch rename, bulk metadata/keyword editing, and hand-off to batch processing). It MUST be a native surface, not a runtime dependency on a separate vendor application; batch operations it launches run on the batch runner of Section 4 with the same receipting, quiet-law, and promotion semantics.

[STU-AUT-026] Studio MUST support OS and clipboard handoff as commands: copy-as-CSS/SVG/PNG of a selection to the clipboard for cross-tool paste; open-in / edit-in round trips that stay within the ONE unified `StudioDocument` rather than exporting between separate applications (dedup of cross-app edit-in round trips — Studio needs none because raster/vector/layout are one document per [STU-DOC-001]); and loading third-party raster filter plugins under the sandboxed plugin/consent contract of Section 7, never as ambient native code. Platform-exclusive OS integrations (OS photo-app editor extensions, pen/dial/tablet input) are supported where the native shell provides the surface and are otherwise out of scope, never a hosted or platform-locked runtime dependency ([STU-OVR-002]).

### 10. GUI, diagnostics, and manual obligations

[STU-AUT-027] Every automation surface in this sub-section — the command contract, macros, batch runner, data-driven graphics, find/change, the plugin surface, the inspect/codegen surface, and the asset/OS/clipboard handoff — MUST be operable both from the native operator UI and from the typed model/plugin surface (14.16), MUST honor the headless/quiet law (14.20) for all background and batch execution (bounded, non-focus-stealing, non-input-hijacking, observable through structured job state), MUST expose stable AccessKit/`author_id` targets and structured state to the Argus visual-inspection/steering surface so a no-context model can drive and re-observe every automation control, and MUST be documented in the dual-audience Studio UserManual (14.22) with operator recording/batching/plugin-install workflows and the model/plugin command reference derived from the same command `inputSchema`s.

---

## 14.15 Whiteboard & Diagramming

This sub-section is the normative Studio whiteboard and diagramming surface (the deduped FigJam-class parity target). It is intentionally proportionate: whiteboarding is a lower-priority parity domain than the raster/vector/layout/design-system catalogs, so this section is complete but compact. Whiteboarding reuses the unified document model and primitive set of 14.3 rather than introducing a separate application: a whiteboard is a `StudioDocument` operating in **whiteboard mode** (a `StudioBoard` container of freeform board objects), sharing the same selection, history/undo (14.19), color, collaboration (14.17), command (14.16 / 14.14), export (14.13), and model-steerability surfaces as every other Studio document. Board object types are `StudioLayer` kinds, board widgets are Studio plugins (Section 14.14.7), and facilitation is a projection of the collaboration substrate (14.17). A source suite's whiteboard product name is never a Studio surface name.

---

### 1. Whiteboard document mode

[STU-WB-001] Studio MUST support a **whiteboard mode** of `StudioDocument`: an unbounded freeform board surface (`StudioBoard`) holding board objects as `StudioLayer` nodes, distinct from the fixed-canvas/artboard and page-spread modes but sharing the same document, selection, history, color, collaboration, command, and export machinery. A whiteboard document MUST NOT be a separate document type with its own siloed model; it is a mode of the one `StudioDocument` ([STU-DOC-001]), so board content can be selected, styled, versioned, exported, and model-steered through the identical primitives and commands as any other Studio content.

[STU-WB-002] A whiteboard document MUST support multiple pages/boards within one document, switchable from a navigator, so a single file can hold multiple exercises or board views (the deduped equivalent of multi-page boards), reusing the artboard/page navigation surface rather than a whiteboard-specific one.

[STU-WB-003] Board objects are collaborative-editing-first: whiteboard mode MUST operate over the CRDT collaboration substrate (14.17) so multiple operators and model lanes edit one board concurrently with attributable, receipted, undoable changes under the same parallel-workflow guarantees as the rest of Studio.

### 2. Whiteboard object types

[STU-WB-004] Studio MUST provide the deduped whiteboard object set below as first-class `StudioLayer` kinds addressable by the typed command contract (14.14) and, where noted, reusing existing Studio primitives rather than reinventing them. Connectors are "smart": they attach to object anchor magnets and auto-reroute as endpoints move.

| Board object | Behavior | Reused / backing primitive |
|---|---|---|
| Sticky note | Auto-sizing text card with color options and optional author-name attribution | `StudioLayer` (sticky kind) + `StudioTextStory` |
| Shape with text | Diagram shape (rectangle, ellipse, diamond, and related) carrying inline editable text, resizing around it, as a flowchart node | `StudioVectorPath` + `StudioTextStory` |
| Smart connector | Line attaching to object anchor magnets, auto-rerouting on move, with straight/elbow styles, arrowheads, and inline labels | `StudioVectorPath` (connector kind) |
| Stamp / emote | Droppable stamp objects (dots, stars, hearts, plus-ones with attribution) and transient emote animations | `StudioLayer` (stamp kind) + collaboration transient (14.17) |
| Table | Cell grid with add/remove rows and columns, cell text, and cell fills for lightweight matrices | `StudioLayer` (table kind) |
| Mind map | Node tree that branches via keyboard/handle with automatic layout and sibling rebalancing | `StudioLayer` (mindmap kind) + connectors |
| Section | Named container grouping board content, collapsible/expandable, usable as a navigation and voting target | `StudioLayer` (section kind) |
| Media embed | Images, video/GIF media, external iframe embeds, and unfurled link-preview cards | `StudioLayer` (placed_asset / embed kinds) |
| Code block | Syntax-highlighted code snippet with language selection | `StudioLayer` (code kind) + `StudioTextStory` |
| Washi tape | Patterned decorative tape strips for attaching or decorating content | `StudioLayer` (decoration kind) + `StudioPattern` |
| Drawing (marker/highlighter) | Freehand marker and highlighter ink strokes tuned for whiteboarding rather than precision vector illustration | `StudioVectorPath` (ink kind) |

### 3. Templates and widgets

[STU-WB-005] Studio MUST provide a board **template** surface: boards start from a built-in template gallery, teams save and reuse custom templates, and quick-create shortcuts stamp common structures. Template distribution is local/native (a local template library), consistent with the no-hosted-marketplace posture of [STU-AUT-020].

[STU-WB-006] Studio MUST support interactive board **widgets** — polls, trackers, voting aids, and similar interactive objects with synced multi-user state — implemented as Studio plugins on the whiteboard-widget extension point ([STU-AUT-019], Widget capability), running under the kernel capability/consent gates. A widget is not a special-cased built-in; it is a plugin-authored board object with synced state over the CRDT substrate, so third parties and models can ship widgets through the one plugin contract.

### 4. Facilitation

[STU-WB-007] Studio MUST provide session **facilitation** as a projection of the collaboration substrate (14.17), not as whiteboard-private machinery. The facilitation set below reuses cursors, presence, and comments from 14.17.

| Facilitation feature | Behavior | Backing |
|---|---|---|
| Voting session | Facilitator opens a session; participants place a limited number of (optionally anonymous) votes on objects, with reveal and tally at end | 14.17 collaboration + `StudioLayer` vote targets |
| Timer | Shared countdown timer visible to all participants, with optional audio, for timed exercises | 14.17 session state |
| Cursor chat | Transient chat bubble attached to a live cursor for lightweight in-canvas messaging | 14.17 live cursors |
| Spotlight / follow | A presenter pulls all participants' viewports along with their navigation until spotlight ends | 14.17 presence/viewport |

[STU-WB-008] Facilitation features that in the source suite depend on hosted identity/session services (anonymous open sessions for external visitors, hosted presence) are supported to the extent the local-first collaboration substrate (14.17) provides them; hosted-only variants are out of scope and MUST degrade to the local capability rather than introduce a hosted runtime dependency ([STU-OVR-002]).

### 5. Diagram generation

[STU-WB-009] Studio MUST support **diagram generation** into whiteboard mode: board and diagram structures (flowcharts, mind maps, matrices, stickied breakdowns) generated from a prompt or from existing canvas content via the model-steerable command surface (14.16). Generation is expressed as a `StudioCommandBatch` (14.14) that mints board objects through node-factory commands, so a generated diagram is one reviewable, promotion-gated, undoable proposal (14.18/14.19) authored under a `KernelActor` model identity — never an unaudited direct board mutation. Model-driven board operations (summarize stickies, cluster/sort, ideate) are ordinary commands over board `StudioLayer` nodes and inherit the receipting and quiet-law guarantees of the rest of Studio; Studio's generation is native to the kernel model lanes, not a dependency on a hosted board-AI service.

### 6. Import / export posture

[STU-WB-010] Whiteboard import/export MUST route through the unified import/export surface (14.13), not a whiteboard-private format layer. Boards export to raster (PNG), document (PDF), and tabular (CSV, e.g. stickies/table rows) forms via `StudioExportRecipe`; delimited data (CSV/spreadsheet rows) imports as stickies or table objects. Importers for other whiteboard tools' content are compatibility targets under 14.13 with unsupported-feature receipts; the resulting objects are local Studio board objects, and no external importer service is a runtime dependency.

### 7. GUI, diagnostics, and manual obligations

[STU-WB-011] The whiteboard surface — board mode, every board object type, templates, widgets, facilitation, diagram generation, and board import/export — MUST be operable both from the native operator UI and from the typed model command surface (14.16), MUST honor the headless/quiet law (14.20) for background board operations and model-driven generation (bounded, non-focus-stealing, non-input-hijacking, observable through structured state), MUST expose stable AccessKit/`author_id` targets and structured state to the Argus visual-inspection/steering surface so a no-context model can place, connect, and re-observe board objects, and MUST be documented in the dual-audience Studio UserManual (14.22) with operator facilitation/board workflows and the model command reference for board objects and diagram generation.


---

## 14.16 Model Visibility and Steerability (Backend and Frontend)

Studio tools MUST be visible to models in BOTH the backend and the frontend, and MUST be visually steerable. This sub-section is normative.

[STU-MDL-001] BACKEND VISIBILITY. Every Studio capability MUST be reachable through a typed command contract on the unified Studio command API (14.14). The API MUST expose: (a) a structured read surface over `StudioDocument` — the scene/layer-graph tree, selectors by id/kind/name, semantic diffs between document states, and structured render receipts; and (b) a typed write surface where every command declares typed inputs/outputs, `dry_run` support, a receipt shape, undo semantics, and deterministic replay. A model MUST be able to inspect the full Studio document and tool surface from the API alone, without screen-reading and without source-app knowledge.

[STU-MDL-002] FRONTEND VISIBILITY. Every operator-visible Studio surface (pane, panel, tab, tool, control, canvas) MUST expose a stable `author_id` target through the native AccessKit/egui accessibility tree so Argus can identify, inspect, and steer it. The frontend MUST expose machine-readable UI-state snapshots (which tool is active, current selection, panel/layout state, document/artboard focus) queryable without pixel screen-reading.

[STU-MDL-003] COMMAND-CONTRACT COMPLETENESS. Every shipped Studio command MUST carry a typed contract with: stable `command_id` (`studio.<domain>.<command>.v<N>`), `inputSchema`/`outputSchema` (auto-generated via `schemars::JsonSchema`), `dry_run` availability, a `SimulationReceipt`-class receipt with stable finding codes and a JSON-Pointer `suggested_fix` (deterministic model self-correction loop, mirroring [TAI-OVR-005]), undo semantics (14.19), and a declared Argus target for any operator-visible surface it affects. Commands without a typed contract MUST NOT ship.

[STU-MDL-004] STEERABILITY IS COMMAND-API-FIRST. A model steers Studio through the canonical command API first; it MUST NOT use OS-level input injection. For GUI proof and steering, Argus drives the AccessKit-visible controls by `author_id` (`argus.inspect`/`argus.click`/`argus.set_value`/`argus.screenshot`) per `.GOV/roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md`. Both paths (command API for authority mutation; Argus for visual verification/steering) resolve to the same `StudioDocument` authority through the promotion lifecycle (14.18).

[STU-MDL-005] The model API and the operator UI are two projections of the same command contract and the same primitives (14.3); there is no separate model shim and no capability reachable from one projection but not the other, except where a control is intentionally operator-only or intentionally headless and records a typed reason.

[STU-MDL-006] [ADD v02.200] NO OPERATOR-ONLY CAPABILITY BY DEFAULT. Every Studio tool, command, and primitive MUST be model-invokable through the typed command API. Absence of a model command path for any editing capability is a spec-conformance defect, not a design choice. A capability may be operator-only or intentionally headless ONLY if it records a typed `operator_only_reason` / `headless_reason` per [STU-MDL-005]; unreasoned absence of a model path is forbidden. This makes the full Studio tool/primitive surface — photo/raster tools, vector and mask tools, layout, typography, color, effects, design-system, prototyping, raw, and interop — drivable by parallel model agents, not only by the operator.

---

## 14.17 Parallel Workflows (Multi-File, Multi-Model, Operator Concurrency)

Studio MUST support parallel model work across files and parallel models on a single or multiple files, concurrently with operator work, without interference. This sub-section is normative and depends on the kernel scheduler, leases, CRDT, and process-ownership systems.

[STU-PAR-001] MULTI-FILE PARALLELISM. Each open Studio file is one CRDT document and one job/lease scope. Models MAY work on multiple files concurrently; each file's mutations are lease-scoped and promoted independently through its own promotion lifecycle. Renders, exports, filters, and batch operations run as kernel scheduler jobs with leases, backpressure, cooperative cancellation, and stale-session recovery (mirrors the Tailor jobs binding).

[STU-PAR-002] MULTI-MODEL ON ONE FILE. Multiple model sessions MAY work on the same file concurrently. Concurrency MUST be mediated by the per-file CRDT document with per-actor identity/attribution (`KernelActor` variants; Argus `agent_label`), conflict surfacing, and deterministic promotion ordering through EventLedger idempotency. Two models MUST NOT silently overwrite each other; conflicting proposals surface as conflict state (HBR-SWARM), and promotion is ordered and idempotent.

[STU-PAR-003] MULTI-MODEL ACROSS FILES. The scheduler MUST distribute parallel model sessions across files with leases/backpressure so no file is mutated by two uncoordinated writers and no session starves. Runtime state (active/next operation, lease holder, waiting state, worktree/project scope) MUST be observable, attributable, and restartable.

[STU-PAR-004] OPERATOR CONCURRENCY ISOLATION. Operator sessions and agent sessions bind to explicit project/module scopes. An agent working in one project/module MUST NOT affect, interrupt, focus-steal from, or conflict with the operator working in a different project/module. Concurrent operator+agent work in the SAME file uses CRDT presence (14.21) with agent activity visible but non-intrusive (14.20).

[STU-PAR-005] PROCESS OWNERSHIP. Every Studio-spawned process (render workers, export jobs, headless harness instances) MUST register in the kernel process-lifecycle ledger with `owner_session`/`owner_wp`/`owner_role`/`started_at`, and MUST be reclaimed on session close, failure, staleness, or operator cancel. No orphan processes after a run (HBR-QUIET-003).

---

## 14.18 Propose-Work System

Model-authored Studio edits MUST flow through a typed propose-work system, never directly into authority. This sub-section is normative.

[STU-PW-001] `StudioEditProposal` (schema id `hsk.studio.edit_proposal@1`) is the typed unit of proposed model work: a command batch over a target `StudioDocument`, plus preview artifacts (rendered before/after captures and structured document-diff), plus a receipt with validation findings. A model authors a `StudioEditProposal` into sandbox/CRDT draft space; it MUST NOT touch authority rows until promoted. This reuses the kernel's existing model-edit-proposal path and the DCC Approval Inbox; Studio MUST NOT build a parallel approval system.

[STU-PW-002] VALIDATION. Every `StudioEditProposal` MUST be validated by deterministic checks (document-integrity checks and visual-diff checks in the `StudioValidationDescriptor` catalog, 14.24) in the kernel sandbox before it is eligible for promotion. Validation findings carry stable codes, a `severity` (`blocking`/`advisory`/`info`), and an optional `suggested_fix`.

[STU-PW-003] APPROVAL + PROMOTION. Accepted proposals convert into authority EventLedger events through `PromotionGate::evaluate()` with an idempotency key (`STUDIO-PROM-{proposal_id}-{validation_run_id}`, mirroring the Tailor `CPROM-` pattern). Operator approval is surfaced in the DCC Approval Inbox; automated self-approval by the authoring model is architecturally blocked. Rejected proposals persist as replayable evidence (they are not discarded), so a model can inspect why a proposal failed and self-correct.

[STU-PW-004] REPLAYABILITY. A promoted or rejected proposal, and the resulting authority state, MUST be fully reconstructable from EventLedger receipts alone, without chat history, session context, or agent-local memory (mirrors [TAI-OVR-007]).

---

## 14.19 Per-File History, Undo, and Revert-of-Undo

Studio MUST provide per-file history/undo with revert-of-undo. This sub-section is normative. The operator requirement ("history/undo/revert undo 1 level deep per file") is recorded with both readings and a recommended contract; the recommended reading (A) is normative unless the operator directs otherwise.

[STU-HIS-001] PER-FILE HISTORY STACK. Each Studio file maintains its own history stack; there is one unified history/undo surface across all domains in that file (no per-domain or per-mode undo stacks — 14.21). Every promoted command batch appends a `StudioHistoryEntry` backed by the EventLedger events it produced. History is model-visible via a history-query command (14.16).

[STU-HIS-002] UNDO. Undo reverts the most recent promoted `StudioHistoryEntry` by applying its inverse command or a snapshot revert, and is itself a ledger-recorded, receipted, replayable operation (`STUDIO_HISTORY_UNDONE`) — never a hidden in-memory state pop.

[STU-HIS-003] REVERT-OF-UNDO (redo). After an undo, Studio MUST support reverting the undo (redo) at least one level deep per file — Reading B, the minimum floor. Reading A (recommended, normative default): Studio maintains a per-file undo stack with at least one-level redo, and SHOULD support deep undo with a redo depth of >= 1; redo is a ledger-recorded, receipted operation (`STUDIO_HISTORY_REDONE`). The exact redo depth (>1) is an open contract point (14.24 open questions); the floor is: undo is deep per file, and at least one level of revert-of-undo is available per file.

[STU-HIS-004] SNAPSHOT SEMANTICS. For raster-destructive or large-payload operations where inverse-command replay is impractical, history entries reference content-addressed snapshots so undo/redo remain correct and replayable. Snapshots live in product-managed artifact storage; history/authority truth resolves through SurrealDB/EventLedger + CRDT, never SQLite.

---

## 14.20 Headless / Quiet Operation Law

Single-file and multi-file work by single or parallel agents MUST run headless and MUST NOT take keyboard input, pop up windows/apps, or conflict with the operator using the app on other projects/modules. This sub-section is normative and binds to HBR-QUIET and Argus.

[STU-QUIET-001] HEADLESS BY LAW. Agent-driven Studio work MUST run headless (no visible window brought to foreground). It MUST NOT steal keyboard focus, hijack keyboard input, move or click the OS mouse/cursor, capture focus, or use attention-stealing desktop APIs (HBR-QUIET-001).

[STU-QUIET-002] NO OS-INPUT AUTOMATION. Every Studio automation surface MUST be reachable through the command API and the Argus/AccessKit path without OS-level input injection; a negative-test harness MUST confirm no automation surface responds to simulated global keyboard/mouse input (HBR-QUIET-002).

[STU-QUIET-003] NON-INTRUSIVE CO-WORK. While the operator uses Studio (or any other Handshake module) on a project/module, agent work on a different project/module MUST NOT bring Studio to the foreground, pop panes, move the operator's viewport, or change the operator's active tool/selection. Agent activity in the operator's current file is surfaced through presence/pending-proposal indicators (14.21), never through intrusive UI takeover.

[STU-QUIET-004] PROCESS/TERMINAL HYGIENE. Headless render/export/harness processes are hidden, owner-tagged, and reclaimed on completion/failure/staleness (14.17; HBR-QUIET-003). No blank or stale windows persist on the operator desktop.

[STU-QUIET-005] HEADLESS VISUAL PROOF. Agents visually inspect output through the headless `StudioRenderHarness` (render-to-buffer capture) and Argus on the AccessKit tree. Where headless-GPU pixel readback is unreliable (documented `Harness::render` readback crash class `0xc0000005` on headless-GPU hosts), the harness MUST fall back to AccessKit-tree/state assertions and run pixel screenshots on a real-GPU host, never blocking on foreground automation.

---

## 14.21 Operator Unification Surface

Studio MUST also work as ONE unified operator-facing creative application that replaces the five source suites for a human operator. This sub-section is normative.

[STU-UNI-001] ONE DOCUMENT, ONE TOOL SURFACE. Studio presents one unified `StudioDocument` model and one unified tool surface across raster/vector/layout/design-system/whiteboard domains. There are no per-source-app silos: a shared capability exists once as a Studio primitive (14.3) and every operator-facing tool/panel/command is a projection of that single primitive — the same typed contract models call (14.16).

[STU-UNI-002] TASK MODES OVER ONE DOCUMENT. Operator workflows are organized as workspace/persona-style TASK MODES over the SAME document and primitives — a photo/pixel mode, a vector/design mode, a page-layout mode, a design-system/prototyping mode, and a whiteboard mode — never separate applications or separate document states. Switching mode changes tool prominence and panel layout only; document state, selection, color, and history are untouched. (Field precedent: Affinity v3's Vector/Pixel/Layout studios over one `.af` document; Studio adopts the shared-primitive architecture, not an app-switching shell, and stays fully local-first.)

[STU-UNI-003] SHARED OPERATOR UX INVARIANTS. Across all task modes: ONE selection model (`StudioSelectionSet` regardless of raster/vector/layout context), ONE undo/history surface (14.19), ONE color pipeline (14.8), ONE asset/library surface (Loom is the home for brushes, styles, palettes, components, export recipes, and placed assets — no Studio-private asset silo), and ONE export surface (`StudioExportRecipe`, 14.13).

[STU-UNI-004] OPERATOR+MODEL CO-WORK. Operators and models co-work in the same unified surface: the per-file CRDT document carries presence for operator and model sessions with per-actor identity/attribution. Agent activity is visible in the operator UI (presence indicators, pending-proposal badges, conflict state) but never intrusive (14.20). The operator keeps editing while agents work adjacent state.

[STU-UNI-005] The operator surface is a control room, not a chat window: it surfaces document/layer state, selection, active tool, pending proposals, validation state, history, and export status as structured state first. It MUST NOT steal keyboard focus or open uncontrolled windows (14.20).

---

## 14.22 Studio UserManual (Dual-Audience)

Handshake has ONE in-product internal UserManual for no-context models AND operators (CX-982-001). Studio manual coverage MUST be extensive and dual-audience. This sub-section is normative and binds to HBR-MAN.

[STU-MAN-001] DUAL-AUDIENCE ENTRY CONTRACT. Every Studio tool, command, panel, and workflow MUST have ONE UserManual entry with two layers of the same entry: (a) an operator layer — user-friendly, task-oriented, minimal technicality ("how do I crop", "how do I mask", "how do I set type on a path"), navigation path, expected result; and (b) a model layer — technically complete: `command_id`, typed inputs/outputs, `dry_run` availability, receipt shape, undo semantics, Argus `author_id` targets, proof/evidence path, failure modes, recovery. Both layers satisfy CX-982-003 and CX-982-004 (Flight Recorder/EventLedger linkage + HBR-INT-009 three-tier posture per entry).

[STU-MAN-002] FULL-TOOL-SURFACE COVERAGE. ALL Studio tools available and how to use them MUST be documented: a no-context model MUST be able to discover the complete Studio tool surface from the manual alone — no chat history, no source-app prior knowledge, no repo reading (CX-982-003; HBR-MAN-002 no-context operation test). Coverage completeness is checkable: every shipped command contract MUST have a matching manual entry, and manual entries for wired surfaces MUST be code-truthful with self-consistency tests (HBR-MAN-003); drift is a build-rule failure.

[STU-MAN-003] SAME-CHANGE CURRENCY. Every implementation MT that adds, changes, wires, exposes, deprecates, or removes a Studio behavior MUST update the UserManual in the SAME change (HBR-MAN-001, CX-982-002), with self-consistency verification (HBR-MAN-003), the no-context operation harness (HBR-MAN-002), and the HBR-INT-009 diagnostic-posture linkage per entry (HBR-MAN-004; internal_diagnostics/Palmistry absence recorded DEFERRED-with-reason, never silently skipped).

[STU-MAN-004] SEARCHABILITY. The Studio manual MUST be queryable along at least four axes: tool name, task intent (e.g. "remove background", "set type on a path"), `command_id` (exact `studio.<domain>.<command>.v<N>` lookup for models), and Argus `author_id` target (reverse lookup from a UI target to the manual entry that documents it). Search MUST work for both audiences without chat history.

---

## 14.23 Canonical Studio Authority Contracts

This sub-section is the single canonical semantic authority for every Studio type, field, unit,
event variant, schema id, table/entity, and promotion rule, subject to the physical-storage,
schema-rollout, privacy, transaction, and proof override in [STU-SDB-001] through [STU-SDB-009].
Where another Studio sub-section conflicts with 14.23 on semantic product behavior, 14.23 wins;
where 14.23 uses a legacy database-specific form, the v02.204 SurrealDB override wins.

[STU-CON-001] CANONICAL PRIMITIVES. The canonical Studio primitive set is defined in 14.3 [STU-DOC-002]. Each primitive's field-level struct is authored here as implementation lands; the per-domain catalogs (14.4-14.15) reference these primitives and MUST NOT fork them. Newly discovered primitives MUST be added here (not invented locally in a domain catalog) via governed spec enrichment.

[STU-CON-002] SCHEMA IDS. Canonical Studio schema ids (each a `pub const SCHEMA_STUDIO_* = "hsk.studio.*@N"`): `hsk.studio.document@1`, `hsk.studio.layer@1`, `hsk.studio.artboard@1`, `hsk.studio.page_spread@1`, `hsk.studio.selection_set@1`, `hsk.studio.mask@1`, `hsk.studio.vector_path@1`, `hsk.studio.vector_network@1`, `hsk.studio.text_story@1`, `hsk.studio.type_style@1`, `hsk.studio.color_profile@1`, `hsk.studio.swatch@1`, `hsk.studio.gradient@1`, `hsk.studio.pattern@1`, `hsk.studio.effect_stack@1`, `hsk.studio.adjustment@1`, `hsk.studio.live_filter@1`, `hsk.studio.component@1`, `hsk.studio.component_instance@1`, `hsk.studio.variable@1`, `hsk.studio.variable_collection@1`, `hsk.studio.style_registry@1`, `hsk.studio.auto_layout@1`, `hsk.studio.constraint@1`, `hsk.studio.layout_grid@1`, `hsk.studio.prototype_flow@1`, `hsk.studio.motion_timeline@1`, `hsk.studio.raw_develop@1`, `hsk.studio.export_recipe@1`, `hsk.studio.import_profile@1`, `hsk.studio.history_entry@1`, `hsk.studio.edit_proposal@1`, `hsk.studio.simulation_receipt@1`.

[STU-CON-003] EVENT VARIANTS. Canonical `KernelEventType` additions (wire `STUDIO_*` SCREAMING_SNAKE_CASE): `STUDIO_DOCUMENT_CREATED`, `STUDIO_DOCUMENT_PROMOTED`, `STUDIO_LAYER_CREATED`, `STUDIO_LAYER_MUTATED`, `STUDIO_SELECTION_CHANGED`, `STUDIO_MASK_APPLIED`, `STUDIO_ADJUSTMENT_APPLIED`, `STUDIO_EFFECT_APPLIED`, `STUDIO_VECTOR_PATH_EDITED`, `STUDIO_TEXT_EDITED`, `STUDIO_STYLE_APPLIED`, `STUDIO_COMPONENT_PUBLISHED`, `STUDIO_VARIABLE_SET`, `STUDIO_PROTOTYPE_EDITED`, `STUDIO_RAW_DEVELOPED`, `STUDIO_EXPORT_RENDERED`, `STUDIO_IMPORT_COMPLETED`, `STUDIO_EDIT_PROPOSAL_RECORDED`, `STUDIO_EDIT_PROMOTED`, `STUDIO_EDIT_REJECTED`, `STUDIO_HISTORY_UNDONE`, `STUDIO_HISTORY_REDONE`. The Flight Recorder business-event family is `FR-EVT-STUDIO-*` (14.24). Every variant registers in `required_first_slice_events()`.

[STU-CON-004] TABLES. Canonical `studio_*` SurrealDB `SCHEMAFULL` tables (all preserving the specified prefixed domain ids, all with a required typed `event_ledger_event_id` record reference, all protected by authenticated record-user and field permissions plus `ResourceBroker`, and all evolved through SurrealKit): `studio_documents` (SDOC-), `studio_layers` (SLYR-), `studio_artboards` (SART-), `studio_page_spreads` (SPGS-), `studio_masks` (SMSK-), `studio_vector_paths` (SVPT-), `studio_text_stories` (STXT-), `studio_type_styles` (STYS-), `studio_color_profiles` (SCPF-), `studio_swatches` (SSWT-), `studio_effect_stacks` (SEFX-), `studio_components` (SCMP-), `studio_component_instances` (SCIN-), `studio_variables` (SVAR-), `studio_variable_collections` (SVCL-), `studio_style_registries` (SSTY-), `studio_prototype_flows` (SPTF-), `studio_motion_timelines` (SMTL-), `studio_export_recipes` (SXPR-), `studio_import_profiles` (SIMP-), `studio_history_entries` (SHIS-), `studio_edit_proposals` (SEPR-). Typed field definitions, assertions, indexes, permissions, and rollout manifests are authored here as each table's implementing MT lands.

[STU-CON-005] UNITS & DETERMINISM. Unit law per [STU-DOC-003]. Promotion equivalence for renders validated across different GPU backends/drivers MUST use a pixel/vertex tolerance comparator (max per-channel/position deviation <= a declared epsilon), NOT SHA-256 content-hash equality; content hashes are reserved for same-machine same-run idempotency and EventLedger receipt fingerprinting (mirrors [TAI-OVR-006]).

[STU-CON-006] OPEN CONTRACT POINTS (resolve via governed enrichment/refinement before the implementing MT hardens): (1) redo depth beyond one level (14.19); (2) STUDIO_* event granularity/coalescing for high-frequency raster/vector edits; (3) auto-accept policy class for low-risk model proposals (14.18); (4) raster authority storage shape (tile table vs artifact-ref) and its CRDT representation; (5) FR-EVT-STUDIO-* family registration; (6) the plugin capability/permission schema (14.14). These are tracked as spec-debt until resolved.

[STU-CON-007] [ADD v02.200] UNIVERSAL COMMAND CONTRACT (HARD). Every Studio command, tool, and primitive defined in sub-sections 14.4-14.15 — without exception and regardless of domain — MUST satisfy ALL FOUR of the following properties. A per-domain obligation clause MUST enumerate all four; omission of any one for a shipped command is a spec-conformance defect, not domain discretion; and each domain's acceptance surface MUST carry a conformance check proving all four hold for that domain's commands.

- (a) MODEL-INVOKABLE (14.16): reachable through the typed model command API with `command_id`, auto-generated input/output schema, `dry_run`, a `SimulationReceipt`-class receipt, and undo semantics. Operator-only/headless is allowed only with a typed reason ([STU-MDL-006]).
- (b) PARALLEL-SAFE (14.17, 14.18): every document-state mutation routes through the per-file CRDT document + file-scoped lease + EventLedger-idempotent promotion path, so multiple model agents editing multiple files, AND multiple model agents editing one file, cannot corrupt, race, or silently overwrite each other, and never interfere with concurrent operator work on other projects/modules (14.20). No editing command may mutate authority outside this path.
- (c) DETERMINISTIC ([STU-CON-005]): the same command with the same inputs produces byte-stable output on a given backend and output within the declared tolerance across GPU backends/drivers, so a parallel agent's edit is reproducible and fully replayable from EventLedger receipts alone. Non-deterministic behavior (e.g. an unseeded stochastic filter) MUST expose an explicit seed making it deterministic.
- (d) VISUALLY-VERIFIABLE (14.20, 14.24 [STU-VAL-004]): any command producing visual output MUST be inspectable through the headless `StudioRenderHarness` render capture + Argus, so a model can deterministically edit a photo, a raster or vector mask, a layout, or any visual surface, and then verify the rendered result — capturing before/after evidence — without foreground UI or focus steal.

This clause makes the entire Studio tool/primitive surface usable by parallel model agents on multiple files simultaneously, editing deterministically in conjunction with the visual tools, as a HARD product invariant rather than a per-tool option.

---

## 14.24 Validation, Promotion Equivalence, and HBR

[STU-VAL-001] `StudioValidationDescriptor` is the Studio validation-check catalog (document-integrity checks, visual-diff checks, format round-trip checks, forbidden-pattern/anti-scaffold checks). Every model-authored proposal (14.18) and every implementing MT MUST pass the applicable checks before promotion/handoff. Checks carry stable codes and severities.

[STU-VAL-002] THREE-TIER DIAGNOSTICS (HBR-INT-009). Every observable Studio runtime behavior MUST be mapped across the three diagnostic tiers with a recorded per-tier outcome (WIRED | NOT_APPLICABLE-with-reason | DEFERRED-with-reason): Tier 1 Flight Recorder (`FR-EVT-STUDIO-*` business events — WIRED at build); Tier 2 internal_diagnostics (frame-time on canvas/viewport renders, panic hooks in `studio-engine`, UI-thread heartbeat for Studio panes, GPU/CPU/RSS counters during render/filter jobs, diagnostic-event API for render-cache/GPU-device-lost/readback failures — DEFERRED-until-shipped per WP-KERNEL-012/016); Tier 3 Palmistry (external watcher over freeze/crash during heavy render — DEFERRED-until-shipped). Deferral is typed, never silent.

[STU-VAL-003] ARGUS VISUAL PROOF (HBR-VIS). Every GUI/operator-surface/diagnostic-surface/frontend Studio change MUST be inspected and (where steered) verified through Argus with stable `author_id` targets, before/after observation, and recorded evidence; missing Argus visibility/steering is HBR-VIS debt with allowed same-MT/WP remediation.

[STU-VAL-004] VISUAL INSPECTION DUTY. A model MUST inspect BOTH structured output (document-model diffs, receipts, exported-file integrity) AND rendered visual output (canvas/screenshot capture via `StudioRenderHarness` + Argus) before claiming a Studio edit done. Proposal receipts (14.18) carry before/after evidence references.

[STU-VAL-005] NATIVE-RUNTIME + SURREALDB/CRDT DUTY. Studio durable authority is Handshake-managed SurrealDB/EventLedger; live collaborative state is CRDT with persistence/reconnect/replay/conflict-visibility and promotion into authority. No PostgreSQL fallback or dual authority, no SQLite, no SQL-portability shim, no mock-only resource as default proof, and no Docker/third-party-daemon/outside-app as a core-operation dependency or acceptance shortcut. Test-only/fixture-only proof cannot satisfy a runtime/storage/EventLedger/UI/replay MUST; storage and privacy proof MUST meet [STU-SDB-008].

[STU-VAL-006] HBR MATRIX. Every Studio WP/MT MUST carry the applicable HBR rows (INT/SWARM/VIS/QUIET/MAN/STOP) in its acceptance matrix, resolved to PROVED / NOT_APPLICABLE-with-reason / BLOCKED-with-cause; PASS closure is illegal while any required HBR row is PENDING/STEER/BLOCKED.
