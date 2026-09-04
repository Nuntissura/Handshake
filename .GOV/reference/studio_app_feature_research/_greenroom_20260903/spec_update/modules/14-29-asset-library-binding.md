---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-29"
section_id: "14.29"
title: "14.29 Asset Library Binding (CKC)"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
new_domain: true
new_domain_note: "Section 14 v02.205 names 'the Studio asset library' exactly once, at line 471, with no owner and no anchor, while [STU-AUT-025] separately requires Studio to grow its own native asset browser. That is a double-feature under [STU-SECTION-003]. The operator decided on 2026-09-03 that CKC is the asset library and file manager for Studio, Tailor and the future cui-studio. This module writes that decision into the spec and closes the violation."
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-29-asset-library-binding.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.29 Asset Library Binding (CKC)

This sub-section binds Studio to ONE asset library and file manager, names it, states what Studio consumes from it, states what Studio publishes back into it, and forbids Studio from growing a parallel library. It is a dedup clause block under [STU-SECTION-003], not a new feature.

---

### 1. The binding

**[STU-ASSET-001] CKC is the asset library and file manager.** CKC is the single asset library, media catalog and file manager for Studio, for Tailor (Section 13), and for the future cui-studio surface. Studio does not have an asset library of its own and MUST NOT acquire one. Where Section 14 previously said "the Studio asset library" ([STU-RAS-045], v02.205 line 471), it means CKC reached through the binding defined here.

**[STU-ASSET-002] What CKC is, in kernel terms.** CKC is a Handshake-native kernel-attached module, not an external application and not a vendor product. It was delivered by work packet WP-KERNEL-005 as 207 microtasks, reached `PASSED_INTEGRATION_VALIDATION_V3`, and is contained in the main line. Its domain module is declared in that packet's contract at `src/backend/handshake_core/src/atelier`, covering core catalog records, sheet and tabular records, media records, intake, collections, search and exports, plus a kernel event bridge. CKC is already a first-class module in the shell's module rail, alongside the Studio entry. Studio therefore binds to a shipped, validated kernel module; this clause block introduces no new subsystem.

The path above is read from the WP-KERNEL-005 packet contract and has NOT been re-verified against the active development worktree. Before any microtask derived from this module resolves a code surface, the path MUST be reconciled against that tree per the reference-tree rule, and a mismatch MUST be corrected here rather than worked around.

**[STU-ASSET-003] Primitive reuse, not redefinition.** The binding reuses existing canonical primitives and MUST NOT redefine them:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*
| Primitive | Role in the binding |
|---|---|
| `PRIM-Asset` | the catalog identity of an asset; the thing a Studio placed-asset link points at |
| `PRIM-ArtifactService` | the content-addressed artifact store that holds the actual bytes |
| `PRIM-ArtifactManifest` | the manifest that makes a stored byte stream addressable and receipted |
| `PRIM-AiJob` | the governed job model any derived-asset generation runs under |
| `PRIM-CapabilityProfile` | the capability gate every cross-module asset read and write passes |

**[STU-ASSET-004] Prohibition — no parallel library.** Studio MUST NOT implement, and MUST NOT ship, any of the following: a second asset catalog; a second media or file browser rooted outside CKC; a second thumbnail or preview cache keyed independently of CKC's; a second metadata, keyword, rating, label or flag store; a second collection or smart-collection mechanism over assets; a second duplicate-detection index; a second folder-synchronisation watcher; or a second content-addressed blob tier. Each of these already exists in CKC, and duplicating one is a `[STU-SECTION-003]` violation, not an optimisation.

---

### 2. Consumption: how Studio reads assets

**[STU-ASSET-005] Placed-asset link (normative record).** Studio references an external asset ONLY as a placed-asset link. The link record is:

| Field | Type | Required | Semantics |
|---|---|---|---|
| `link_id` | prefixed string `SPAL-{uuid_v7}` | yes | identity of the LINK, distinct from the identity of the asset |
| `asset_id` | `PRIM-Asset` handle | yes | the catalog identity in CKC |
| `artifact_manifest_id` | `PRIM-ArtifactManifest` handle | yes | the exact byte stream this link resolved to when it was last resolved |
| `placement_kind` | `linked` \| `embedded` | yes | see [STU-ASSET-006] |
| `resolved_content_hash` | string | yes | the content hash observed at last resolution; drives the modified/missing state of [STU-ASSET-007] |
| `resolved_at` | timestamp | yes | when the link was last resolved |
| `intrinsic_metrics` | object or null | no | width, height, duration, page count, colour profile id — read from the CKC record, never by opening the file a second time |
| `transform` | object | yes | the placement transform inside the `StudioDocument`; belongs to Studio, not to CKC |
| `crop` / `fit_mode` / `frame_selection` | domain fields | no | Studio-owned placement fields |

`transform`, `crop`, `fit_mode` and `frame_selection` are STUDIO state. `asset_id`, `artifact_manifest_id`, `resolved_content_hash` and `intrinsic_metrics` are CKC state PROJECTED onto the link and MUST NOT be edited by Studio.

**[STU-ASSET-006] Linked versus embedded.** `placement_kind = linked` means the document holds only the link and resolves bytes from CKC at render, export and publish time. `placement_kind = embedded` means the bytes were copied into the document's own artifact stream. Embedding is permitted, because some deliverables must be self-contained, but an embedded placement MUST STILL carry `asset_id` and `artifact_manifest_id` so provenance survives, and Studio MUST offer `asset.relink_to_catalog` to convert an embedded placement back to a linked one. An embedded placement whose `asset_id` is null is legal only for bytes that never came from CKC (for example a pasted screenshot); such a placement MUST be reported by [STU-ASSET-016] so the operator can catalog it.

**[STU-ASSET-007] Link-state enumeration (normative, closed, five members).** A placed-asset link resolves to exactly one state:

| State | Meaning | Studio behaviour |
|---|---|---|
| `resolved` | asset exists, content hash matches `resolved_content_hash` | render normally |
| `modified` | asset exists, content hash differs | render the current bytes, mark the link modified, offer update-all |
| `missing` | `asset_id` resolves to no catalog record | render a placeholder at the recorded intrinsic metrics, never a zero-size box; emit a diagnostic |
| `unauthorized` | the record exists but the `ResourceBroker` denied this actor | render a denied placeholder; the diagnostic MUST NOT leak the asset's name or path |
| `unresolved` | resolution has not been attempted in this session | render from the last cached preview if one exists, otherwise the placeholder |

`unauthorized` is a distinct state from `missing` and MUST NOT be collapsed into it: telling an operator an asset is missing when it is actually denied is a privacy leak in one direction and a debugging trap in the other.

**[STU-ASSET-008] Bulk binary lives in the artifact tier, not in the document.** Raster tiles, video media, brush-tip bitmaps, LUTs, camera and lens profiles, fonts and framework bundles are content-addressed artifacts. SurrealDB holds the records and the references; the artifact tier holds the bytes. This is not a preference: the scale is such that these do not belong in a document database.

**[STU-ASSET-009] Resolution at export and publish.** At export ([STU-IO-100] and 14.13) and at publish ([STU-WEB-127]), every `linked` placement MUST be resolved from CKC to actual bytes through `PRIM-ArtifactService`, and the export or publish receipt MUST record, per placement, the `asset_id`, the `artifact_manifest_id` actually used, and the link state at resolution time. An export containing a `missing` or `unauthorized` placement MUST fail closed with that receipt, never write a partial deliverable, and never silently substitute a placeholder into shipped output.

**[STU-ASSET-010] Read commands.** Studio MUST reach CKC through typed commands, never through direct filesystem access or a private index: `asset.search` (query the catalog), `asset.get` (fetch one catalog record), `asset.get_preview` (fetch a rendered preview at a requested size), `asset.resolve_bytes` (fetch the artifact stream), `asset.place` (create a placed-asset link in the current document), `asset.relink` (repoint a link at a different `asset_id`), `asset.relink_to_catalog` (embedded to linked), `asset.update_all_modified` (re-resolve every `modified` link in the document in one transaction), and `asset.list_placements` (every placement in a document, with state). Every one of them is a `StudioCommand` under [STU-AUT-001] and inherits its typed, dry-runnable, receipted and undoable contract.

**[STU-ASSET-011] Design-system library versus asset catalog — the boundary.** These are two different mechanisms and MUST NOT be merged:

| | Design-system library (14.10) | CKC asset catalog |
|---|---|---|
| Holds | component definitions, styles, variables and collections | assets: images, video, audio, documents, fonts, raw files |
| Identity | `publish_key` on a publishable entity ([STU-DS-141]) | `PRIM-Asset` handle |
| Distribution | publish and subscribe between documents ([STU-DS-019]) | catalog membership and collections |
| Bytes | none of its own; any byte stream inside a definition is a placed-asset link | owns the bytes through the artifact tier |
| Search surface | the design-system asset browser ([STU-DS-005], narrowed by [STU-DS-101]) | the CKC catalog surface |

A component definition that contains an image contains a LINK to a CKC asset, not the image. Publishing that component to a design-system library publishes the definition and the link; the consumer resolves the asset through CKC exactly as the author did.

---

### 3. Publication: how Studio writes back

**[STU-ASSET-012] Exports publish back into CKC.** Every deliverable Studio produces — a rendered export, a packaged folder, a published web output set, a rendered video, a batch-processed file set, a contact sheet — MUST be materialised through `PRIM-ArtifactService` with a `PRIM-ArtifactManifest` and MUST be registered into the CKC catalog as an asset. Studio MUST NOT write product output to an arbitrary path with no catalog record and no manifest.

**[STU-ASSET-013] Export-back record (normative).** Registering an export back into CKC MUST record: the producing `StudioDocument` id and its revision, the `StudioExportRecipe` id and its resolved parameter set, the artifact manifest id, the list of source `asset_id`s consumed ([STU-ASSET-009]), the `KernelActor` that authored the export, and the EventLedger event id. This is the provenance chain that lets an operator or a model ask, of any delivered file, which document, which recipe and which source assets produced it.

**[STU-ASSET-014] Derived-asset relationship.** An export registered under [STU-ASSET-012] is a DERIVED asset. CKC MUST record the derivation edge from each source asset to the derived asset so that a source asset's catalog view can list what was made from it, and a derived asset can be traced back. The edge is a catalog relationship, not a Studio-private table.

**[STU-ASSET-015] Operator-directed destinations.** An operator may direct an export to a specific filesystem location. That does not exempt the export from [STU-ASSET-012]: the file is written where the operator asked AND registered in the catalog with a manifest. The catalog record is the durable authority; the filesystem copy is a materialisation of it.

**[STU-ASSET-016] Uncataloged-content report.** Studio MUST be able to report, per document, every byte stream that is not backed by a CKC asset: embedded placements with a null `asset_id` ([STU-ASSET-006]), pasted image data, and any legacy embedded content from an imported source file. The report is the operator's path to bringing that content under catalog management, and it is readable through the command surface.

---

### 4. Closures: what Studio builds instead

**[STU-ASSET-017] Supersession of [STU-AUT-025].** v02.205 [STU-AUT-025] requires Studio to provide "a local, native asset browser" with browse, previews, batch rename, bulk metadata and keyword editing, and hand-off to batch processing. That clause is SUPERSEDED in its ownership and PRESERVED in its capability. Restated: those capabilities are CKC capabilities. Studio MUST surface them through a CKC-backed projection inside the Studio viewport — the operator does not have to leave Studio to browse, preview, rename, retag or hand off — but the projection reads and writes CKC authority records through the commands of [STU-ASSET-010] and the CKC command surface, and holds no catalog state of its own. A Studio-local asset index, cache or metadata table is forbidden by [STU-ASSET-004].

**[STU-ASSET-018] Amendment of the v02.205 line-471 row.** The [STU-RAS-045] matrix row reading "Replaced by native local `placed_asset` links (§3) and the Studio asset library" is amended to read: "Replaced by native local placed-asset links ([STU-ASSET-005]) resolved against CKC ([STU-ASSET-001]); vendor-cloud sync remains an optional adapter, not a dependency." The row's posture is unchanged; only the owner is named.

**[STU-ASSET-019] Catalog capability set inherited by Studio.** Studio's asset projection inherits, and MUST NOT re-implement, the catalog capabilities defined for the Library/DAM surface at Spec 10.10.4.5: import by copy, import by reference in place, import by move, content-hash duplicate detection, hierarchical keyword tagging, 0-to-5 star rating, colour labels, pick/reject/unflagged flags, manual collections, rule-based smart collections, filesystem folder synchronisation, EXIF/IPTC/XMP metadata read and write, face-region tagging and identity clustering, geotagging and map view, image stacking, and virtual copies. Where a Studio workflow needs one of these, it calls it; it does not rebuild it.

**[STU-ASSET-020] Tailor and cui-studio share the same binding.** Tailor (Section 13) and the future cui-studio surface bind to CKC through the same placed-asset link record, the same link states, the same read commands and the same export-back contract. A Studio export is visible to Tailor as a catalog asset without a transfer step, because there is one catalog. Any module-specific extension to the link record MUST be namespaced on the link, never on the CKC asset record.

**[STU-ASSET-021] Cross-module authorization.** Every cross-module asset read and write passes the kernel `ResourceBroker` and the record-level SurrealDB permissions of [STU-SDB-005]. Studio holding a link does not grant Studio's actor access to the asset; the broker decides per actor per read. A denied read produces the `unauthorized` state of [STU-ASSET-007], and the denial itself MUST be receipted.

**[STU-ASSET-022] Model-lane parity.** A model lane operating in Studio has exactly the same asset access as an operator in Studio: the same commands, the same broker gate, the same receipts, and the same `KernelActor` attribution distinguishing model-authored placements from operator-authored ones. There is no model-only asset path and no operator-only asset path.

**[STU-ASSET-023] Parallel safety.** Two Studio documents, or two model lanes in one document, may place the same asset concurrently; placement creates independent link records and MUST NOT contend. Mutating the CKC asset record itself (retag, rate, move, delete) uses the expected-revision precondition of [STU-SDB-004] at asset granularity. Deleting a catalog asset that has live placements MUST be refused with a typed error listing the referring documents, unless the operator explicitly forces it, in which case every referring link transitions to `missing` and each transition is receipted.

**[STU-ASSET-024] Determinism.** Asset resolution MUST be deterministic: resolving the same `artifact_manifest_id` MUST yield byte-identical content, and an export run twice against the same document revision and the same resolved manifest ids MUST produce byte-identical output. This is what makes promotion equivalence hold across hosts for documents containing placed assets.

---

### 5. Declared gaps

**[STU-ASSET-025] DECLARED GAP — the artifact tier binding is named but not verified in the target tree.** This module binds Studio's bulk binary to `PRIM-ArtifactService` / `PRIM-ArtifactManifest`, which are canonical primitives in the appendix and are referenced normatively by the Media Annotation Overlays surface (Spec 10.16) and the Settings surface (Spec 10.17). The specific artifact/blob tier implementation present in the WP-KERNEL-012 worktree has NOT been read as part of authoring this module, and this module therefore does NOT state its API, its addressing scheme, its size limits, its eviction policy, or its transactional relationship to a SurrealDB write. Before any microtask derived from this module is activated, the 012 tree MUST be read and one of two things done: (a) this module amended with the real tier's contract, or (b) an explicit BLOCKED record raised if no such tier exists. Inventing a tier is forbidden. An implementer reading this module today knows exactly which primitive to bind to and knows that its concrete surface is unstated.

**[STU-ASSET-026] DECLARED GAP — CKC's own command ids are not enumerated here.** [STU-ASSET-010] names the Studio-side commands. The CKC-side command ids they call are part of the single canonical command corpus (Spec 10.19 `KernelActionCatalogV1`) and MUST be cited by their real `action_id`s when the microtasks are authored. This module does not invent them.

**[STU-ASSET-027] DECLARED GAP — catalog scale is unverified.** [STU-ASSET-019] inherits the catalog capability list from Spec 10.10.4.5, which is a capability list, not a performance contract. The catalog size at which search, preview generation, duplicate detection and smart-collection evaluation must remain interactive is NOT stated anywhere in the spec today. Studio's asset projection MUST declare the tested ceiling in its UserManual, and the ceiling MUST be established by measurement, not asserted.

**[STU-ASSET-028] GUI / Argus / UserManual obligation.** The Studio asset projection, the placed-asset inspector, the link-state indicators, the update-all-modified surface, the relink picker and the uncataloged-content report MUST be model-visible and typed-steerable through the Studio command surface (14.16); MUST be headlessly inspectable, steerable and screenshot-capturable through Argus with no foreground focus steal (14.20); and MUST ship dual-audience UserManual entries covering, at minimum, the five link states of [STU-ASSET-007], what each looks like, what causes it, and how to clear it (14.22).

---

### 6. Microtask Derivation

**[STU-ASSET-029] Derivation rule (NORMATIVE).** The asset-binding microtask set is derived from this module mechanically, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. Each numbered clause that states the **binding itself or a prohibition on breaking it** ([STU-ASSET-001], [STU-ASSET-002], [STU-ASSET-003], [STU-ASSET-004]), a **placed-asset link record field set** ([STU-ASSET-005], [STU-ASSET-006], [STU-ASSET-008]), a **link state** ([STU-ASSET-007] — the five-member closed state set, each state's render behaviour and its diagnostic rule), a **required read command** ([STU-ASSET-010]), a **resolution rule at a boundary** ([STU-ASSET-009]), a **domain-boundary rule** ([STU-ASSET-011]), a **publication and provenance-chain contract** ([STU-ASSET-012], [STU-ASSET-013], [STU-ASSET-014], [STU-ASSET-015], [STU-ASSET-016]), a **supersession closure that changes what gets built** ([STU-ASSET-017], [STU-ASSET-019], [STU-ASSET-020]), or an **execution guarantee** ([STU-ASSET-021], [STU-ASSET-022], [STU-ASSET-023], [STU-ASSET-024]), where that clause can be implemented and proven independently of its siblings.
2. Each **declared gap** — in this module exactly three, [STU-ASSET-025], [STU-ASSET-026] and [STU-ASSET-027]. Each yields a microtask under [STU-ASSET-030], not nothing.

This module names no validation-descriptor set of its own: its failure modes are enforced by the descriptor sets of the consuming domains ([STU-DS-165], [STU-PRO-143], [STU-IO-165], [STU-AUT-171], [STU-WB-135], [STU-WEB-137]), each of which already carries an asset-resolution or credential check. There is therefore no descriptor row in the yields index below, and adding one would double-count checks that already have an owner.

No other unit yields a microtask. Exactly 5 clauses in this module yield nothing, and they are:

- **This derivation sub-section itself** — its five clauses yield nothing.

Every other clause yields at least one unit. This list is the module's declared non-yielding set and is the authority a derivation tool reconciles against.

**[STU-ASSET-030] Open items and blocked dependencies.** This module declares three, and each YIELDS a microtask whose FIRST acceptance criterion is resolving the named dependency. The first of the three is a genuine BLOCKED dependency and gates the rest of the binding:

| Declared gap | Clause | First acceptance criterion of its microtask |
|---|---|---|
| The artifact/blob tier is bound by primitive name but its concrete surface was never read in the target worktree | [STU-ASSET-025] | Read the artifact tier in the WP-KERNEL-012 worktree and amend [STU-ASSET-008] with its real API, addressing scheme, size limits, eviction policy and transactional relationship to a SurrealDB write — or, if no such tier exists there, raise a BLOCKED record naming that exact absence. Inventing a tier is forbidden. No microtask derived from [STU-ASSET-005], [STU-ASSET-008], [STU-ASSET-009] or [STU-ASSET-012] may be activated until this criterion passes, because all four resolve bytes through that tier. |
| The CKC-side command identifiers this module calls are not enumerated | [STU-ASSET-026] | Enumerate the real `action_id` values from the canonical command corpus and amend [STU-ASSET-010] so each Studio-side command names the CKC-side entry it calls. Inventing identifiers is forbidden. |
| The catalog scale at which search, preview generation, duplicate detection and smart-collection evaluation must stay interactive is stated nowhere | [STU-ASSET-027] | Establish the ceiling by measurement against a populated catalog and record it in the UserManual. Asserting a ceiling without measurement does not satisfy the criterion. |

A declared gap MUST NOT be dropped from the yields index, because a gap that yields nothing disappears silently and is rediscovered at implementation time. The same rule governs any open item a later amendment introduces.

**[STU-ASSET-031] Microtask content obligation.** A microtask derived under [STU-ASSET-029] MUST carry into its own body: the clause anchor; the COMPLETE field set of the placed-asset link record of [STU-ASSET-005], with the Studio-owned fields distinguished from the CKC-projected read-only ones; all FIVE link states of [STU-ASSET-007] with the rule that `unauthorized` is never collapsed into `missing`; the full provenance chain of [STU-ASSET-013] where it touches export; the eight-item prohibition list of [STU-ASSET-004] where it touches anything catalog-shaped; and the blocked-dependency gate of [STU-ASSET-030] where it resolves bytes. A microtask that says "place an asset" without the five link states and the linked-versus-embedded rule of [STU-ASSET-006] does not satisfy this clause.

**[STU-ASSET-032] Yields index (NORMATIVE).** The counts below are the derivation surface of this module under [STU-ASSET-029]. They are not estimates: they are the measured output of applying that rule to this module's text, and every row states which unit kinds it contributes.

| Unit group | Clauses | Units by kind | Yields |
|---|---|---|---|
| The binding | [STU-ASSET-001]-[STU-ASSET-004] | 4 clause | 4 |
| Consumption: how Studio reads assets | [STU-ASSET-005]-[STU-ASSET-011] | 7 clause, 1 enumeration | 8 |
| Publication: how Studio writes back | [STU-ASSET-012]-[STU-ASSET-016] | 5 clause | 5 |
| Closures: what Studio builds instead | [STU-ASSET-017]-[STU-ASSET-024] | 8 clause | 8 |
| Declared gaps | [STU-ASSET-025]-[STU-ASSET-028] | 4 clause | 4 |
| Clauses yielding nothing | 5 clauses, listed in [STU-ASSET-029] | — | 0 |
| **Module total** | | **33 clauses** | **29** |

Of this module's 33 clauses, 5 yield nothing and 28 yield at least one unit; tables inside yielding clauses contribute the remainder. The module total is **29**. The last numeric column is the yields count.

**[STU-ASSET-033] Anchor binding.** A microtask derived from this module cites its clause anchor directly. Because 14.29 is a NEW sub-section, no staged microtask predates it: every asset-binding microtask carries a real anchor from the outset, never `spec_anchor_status = "PROVISIONAL"`. A microtask that cannot cite an anchor in [STU-ASSET-001]-[STU-ASSET-028] is out of scope for the asset binding and MUST be re-derived or retired, not activated. A microtask in ANOTHER Studio domain that touches assets does NOT derive from this module; it derives from its own domain clause and carries this module's obligations as acceptance criteria per [STU-ASSET-031]. That is what prevents the binding being re-implemented once per domain, which is the failure [STU-ASSET-004] exists to forbid.
