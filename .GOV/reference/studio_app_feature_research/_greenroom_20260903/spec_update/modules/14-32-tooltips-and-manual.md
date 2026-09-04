---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-32"
section_id: "14.32"
title: "14.32 Tooltips, the Help System, and the Generated UserManual Contract"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
supersedes_section: "NONE — new sub-section. Extends 14.22 (Studio UserManual) of .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md; [STU-MAN-001] through [STU-MAN-004] remain in force unchanged."
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-32-tooltips-and-manual.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.32 Tooltips, the Help System, and the Generated UserManual Contract

This sub-section specifies the tooltip and help system for every Studio surface, the ONE record that backs a tooltip, an accessible description, a menu leaf, a palette row and a manual anchor simultaneously, the build-time generation path from the captured corpora into that record, and the honest coverage ceiling with the gate that keeps it honest.

The requirement it satisfies is "tooltips wherever possible", and the requirement's own generation principle is normative: **tooltip text MUST be generated from the captured corpora rather than hand-written, so it cannot drift from the implementation.** The captures already hold vendor-written descriptions plus real ranges, units, defaults and decimal places. This sub-section is the contract that turns that into a shipped property of the product rather than an intention.

The clauses [STU-MAN-001] through [STU-MAN-004] of v02.205 remain in force UNCHANGED. This sub-section adds the mechanism that makes them satisfiable at Studio's scale — 362 tools and roughly 14,700 parameters — where hand-seeding is not viable.

---

### 1. The tooltip system

**[STU-SHL-230] Five disclosure levels (normative, closed).** Studio has exactly five tooltip levels. Each has a distinct trigger and a distinct content contract. A surface MUST NOT invent a sixth.

| Level | Name | Trigger | Content |
|---|---|---|---|
| L0 | Name chip | ~400ms hover dwell | `display_name` and `shortcut_display`. ONE LINE. Never prose. |
| L1 | Extended tip | ~900ms dwell, or immediately if an L0 tip is already warm from a sibling | see [STU-SHL-232] |
| L2 | What does this do | the `?` glyph in the L1 corner, `F1` on the focused control, or `Shift`+hover | opens the UserManual anchor for that EXACT element in an in-Studio side sheet, never a browser ([STU-SHL-233]) |
| L3 | Rich tip | opt-in preference, OFF by default | a 2–4 second looping demo of the tool's gesture, for the ~120 core-set tools only, never all 362. Existence is an open question; 120 demo clips is a content-production task, not a code task |
| L-status | Hintline | ALWAYS, while a tool is active | the current gesture's modifier map, in the status bar |

**[STU-SHL-231] L0 contract.** L0 renders exactly ONE LINE and never prose: the element's `display_name`, then its `shortcut_display` when the active shortcut set binds one. The chord is RESOLVED at render time from `shortcut_id` and is never baked into stored text ([STU-SHL-242]), so a migration set changes every L0 tip without regenerating the descriptor set. L0 MUST NOT render a summary sentence, a range, a unit, an availability reason or a remedy; an element whose only recovered text is its name still gets a complete L0 tip, which is why L0 coverage is ~100% while summary coverage is not ([STU-MAN-109]). L0 fires on a dwell of roughly 400ms and is suppressed entirely under the anti-annoyance rules of [STU-SHL-235].

**[STU-SHL-232] L1 contract.** L1 renders, in this order, omitting any part that is absent:

1. `display_name` and `shortcut_display`;
2. `summary` — ONE sentence. When `summary` is null, L1 renders WITHOUT the sentence rather than blocking the tip or inventing one ([STU-MAN-105]);
3. for a numeric control: the SOFT range with the hard range in parentheses when they differ, the default, the unit, and the modifier hints;
4. for a dimmed element: the availability reason and the ONE-CLICK REMEDY ([STU-SHL-019], [STU-SHL-020]);
5. for a keyframable property: its `TemporalState` and its keyframe-at-playhead status ([STU-SHL-201]).

Item 4 is load-bearing for the whole design: the reason line on a dimmed element is what REPLACES the persona toggle. An element the operator cannot use right now EXPLAINS ITSELF instead of vanishing. A dimmed element with no reason line is a conformance defect, not a cosmetic gap.

**[STU-SHL-233] L2 contract.** L2 resolves the element's `manual_anchor` and opens it in an in-Studio side sheet. It MUST NOT open a browser, MUST NOT require network access, and MUST resolve to an address the out-of-process inspector can click and assert — the shell's manual pane already renders every topic at a stable per-topic `author_id`, and L2 targets that address. `HELP > Studio Manual > Help for This Tool / This Panel / This Command` are the menu-reachable equivalents of the same resolution, per [STU-SHL-013].

**[STU-SHL-236] L3 contract.** L3 renders a 2-4 second looping demo of a tool's gesture, is OPT-IN through a preference and is OFF by default. Its scope is the roughly 120 core-set tools of [STU-SHL-148] and it MUST NOT be required for the remaining tools, because 120 demo clips is a content-production task rather than a code task and gating any tool's discoverability on a clip that does not exist would make the tool undiscoverable. A tool with no clip falls back to L1 with no visible defect. Whether L3 ships at all is an open item: it is recorded here so its absence is a recorded decision rather than a silent omission, and a shipped clip MUST carry the same `extraction_method` provenance as any other generated asset ([STU-SHL-241]).

**[STU-SHL-234] L-status contract.** The hintline is a fourth disclosure channel costing no hover and no dwell, and it is the right home for transient modifier guidance ("hold the subtract modifier to subtract from the selection"). It is ALWAYS present while a tool is active and it is generated from the tool's declared gesture map, never authored per tool.

**[STU-SHL-235] Anti-annoyance rules (normative).** "Tooltips wherever possible" is a COVERAGE requirement, not a permission to interrupt.

1. Suppress L0 entirely while a drag or scrub is in progress and for 500ms after it ends ([STU-SHL-104] already forbids a tooltip firing during an owned gesture).
2. Suppress L0 on a control the operator has just operated.
3. NEVER place a tip over the control it describes, and never over the canvas region under the cursor.
4. One global tips-off preference plus a per-level cap.
5. A tooltip MUST NOT steal focus, MUST NOT capture the pointer, and MUST NOT appear in a headless or agent-driven session ([STU-QUIET-001]).

---

### 2. The UiDescriptor data contract

**[STU-SHL-240] One record per addressable element (normative).** Prose is NEVER authored inside a UI source file. Every addressable element carries EXACTLY ONE `UiDescriptor`, looked up by `author_id` from a BUILD-TIME-GENERATED set. The renderer, the tooltip, the AccessKit node, the palette row, the menu item and the manual anchor all read THE SAME RECORD, so they cannot disagree. Required fields:

| Field | Semantics |
|---|---|
| `author_id` | the stable address, per the grammar of [STU-MDL-100] |
| `kind` | `tool` \| `tool_group` \| `command` \| `menu_item` \| `panel` \| `control` \| `param` \| `preset` \| `task_scope` \| `layout_preset` |
| `display_name` | the operator-facing name. NEVER a vendor product name ([STU-SHL-008]) |
| `summary` | one sentence, NULLABLE. Null means NO VENDOR TEXT WAS RECOVERED, reported and never faked |
| `shortcut_id` | resolves to the CURRENT binding at render time; a chord is never baked into prose |
| `manual_anchor` | a manual-anchor row: `{anchor_id, page_id, anchor_kind, anchor_value}` where `anchor_value` is the element's own `author_id` |
| `provenance` | `{source_app, source_file, source_key, extraction_method, confidence}` |
| `availability` | `{requires, unavailable_reason_template, remedy_command_id}` ([STU-SHL-049]) |
| `a11y` | `{role, accessible_name, accessible_description}` — see [STU-SHL-244] |
| `menu_path`, `menu_order`, `domain`, `capability`, `menu_only` | the menu projection fields of [STU-SHL-015] |
| `aliases[]` | previous `author_id`s the inspector still accepts ([STU-MDL-103]) |
| ParamSpec block | present when `kind == param` ([STU-SHL-171]) |

**[STU-SHL-241] `extraction_method` (normative, closed).** Four members: `parsed` (read directly from a capture), `resolved` (joined across two capture artefacts), `derived` (computed from captured values, e.g. `keyframable` from the inverse of a flag), `handshake_authored` (written by Handshake because no capture supplies it). The member is REQUIRED on every descriptor, so authored versus derived stays COUNTABLE at any time.

**[STU-SHL-242] Shortcuts are resolved, never baked.** `shortcut_id` resolves to the current binding at render time. A descriptor MUST NOT contain a chord as literal text in `display_name` or `summary`, because chords change with the active shortcut set ([STU-SHL-043]) and a baked chord becomes a lie the moment a migration set is loaded.

**[STU-SHL-243] Ids are never reused.** An `author_id`, once shipped, MUST NOT be reused for a different element and MUST NOT be renamed in place. A rename ADDS a new `author_id` and records the previous one in `aliases[]`; the out-of-process inspector, the manual anchor index and any stored dock layout MUST all continue to resolve through the alias ([STU-MDL-103]). Reusing a retired id for a different element is the one failure this rule exists to prevent, because every stored layout, every test assertion and every manual anchor pointing at the old id would then silently resolve to the wrong element rather than failing loudly. The generator MUST refuse to emit a descriptor set in which a live `author_id` also appears in another descriptor's `aliases[]`.

**[STU-SHL-244] Synchronisation is not achieved, it is UNREPRESENTABLE.** The accessible name and description are not COPIES that must be kept in step; they are the same fields read twice.

1. `accessible_name` **IS** `display_name` — the same field, read twice, never copied.
2. `accessible_description` **IS** `summary` — the same field, read twice, never copied.
3. The tooltip renderer and the AccessKit node builder BOTH take a `&UiDescriptor`; neither accepts a string literal, because the API has no such signature.
4. Localisation swaps the DESCRIPTOR SET, not the call sites, so a translated build changes both surfaces together or neither.

**[STU-SHL-245] The helper signature makes hand-written prose a TYPE ERROR.** There MUST be exactly ONE descriptor-driven tooltip helper, and it MUST take the GENERATED ID ENUM, never a `&str`. It emits BOTH the egui tooltip and the AccessKit description from one record and registers the element in the accessibility registry. There MUST be no signature anywhere in the Studio surface that accepts a string literal for tooltip text or for an accessible description. This is not a lint and not a review convention: it is the type system, because the failure it prevents is the path of least resistance under deadline. A missing generated id is a COMPILE error, not a runtime blank.

**[STU-SHL-246] Localisation.** A localised build MUST swap the DESCRIPTOR SET and MUST leave every call site unchanged, because a call site names a generated id and never a string ([STU-SHL-245]). Locale text MUST NOT appear in an `author_id` ([STU-MDL-103]), in a `tool_id`, in a `command_id`, in a manual `anchor_value`, or in a captured option token ([STU-MDL-114]): those are machine addresses and a translated address breaks every stored layout, every test and every manual anchor at once. `display_name`, `summary` and manual page bodies are the only localisable fields, and localising them MUST change the tooltip and the accessible description together, since both read the same field ([STU-SHL-244]).

**[STU-SHL-247] The existing product defect this contract repairs.** The shell today carries 142 direct hover-text calls across 24 files, with no shared helper, and NOTHING sets an AccessKit description anywhere in the crate. Tooltip text is therefore currently INVISIBLE to the out-of-process inspector, and the divergence this contract exists to prevent is ALREADY PRESENT in the codebase: the tooltip string and the accessible description are two unrelated values today, and only one of them is inspectable. Adding Studio tooltips the same way would multiply an existing product-wide defect by the size of the largest module in the product. The helper of [STU-SHL-245] is the prerequisite (SHL-P-07); whether the 142-site migration is executed inside the Studio work packet is an open operator decision ([STU-SHL-136] OD-7), and leaving it means accessible descriptions remain absent outside Studio. The existing seed to build on is the shell's left-rail icon-button helper, which already passes tooltip text alongside an `author_id` and is one step from taking a descriptor id instead of two strings.

---

### 3. Generation

**[STU-MAN-100] The descriptor generator (normative).** A BUILD-TIME generator reads the captured corpora and emits, as one artefact set:

1. the `UiDescriptor` set, including every `ParamSpec`;
2. a generated Rust id ENUM covering every `author_id`;
3. `UserManualToolEntry`, `UserManualAnchor` and `UserManualPage` SEED ROWS;
4. `tooltip-gaps.json`, the gap ledger of [STU-MAN-110].

UI code names an id from the enum. It NEVER types a display string, a range, a unit or a tooltip.

**[STU-MAN-101] The generator targets the CANONICAL manual store, and one specific wrong target is named.** Seed rows MUST be written to the canonical user-manual store backed by the SurrealDB `user_manual_*` tables at the current canonical manual version. They MUST NOT be written to the declared-deprecated model-manual shim. The shim is named explicitly because it is a plausible wrong target: it still compiles, is still path-included by the desktop application, still exports its own manual version constant, and has a command-reference type that LOOKS like the right destination. Entries written there would NOT appear in the manual search endpoint, the manual tools endpoint or the freshness comparator — so L2 "what does this do" would resolve to NOTHING while every governance check passed. Storage is SurrealDB with the EventLedger only ([STU-SHL-007]); no SQLite, libSQL, Turso or PostgreSQL is introduced by the manual store or by the generator's cache.

**[STU-MAN-102] The descriptor row and the manual row are the SAME GENERATED RECORD.** This is the mechanism, not a convenience. [STU-MAN-003] requires the UserManual to change in the SAME implementation change as the behaviour, and a build guard fails a wired-surface diff without a manual-version bump. With 362 tools and roughly 14,700 parameters, hand-seeding cannot satisfy that gate. Making the descriptor and the manual row one generated record means "the manual is updated in the same change as the behaviour" holds BY CONSTRUCTION rather than by discipline.

**[STU-MAN-103] Generation is idempotent and diffable.** Re-running the generator over unchanged captures produces a byte-identical artefact set. A capture REFRESH produces a REVIEWABLE DIFF, never a silent behaviour change. The generator MUST be runnable offline and MUST NOT require network access.

**[STU-MAN-104] Captured text is used verbatim.** Where a capture supplies text it is used VERBATIM with spelling normalised only, tagged `extraction_method = parsed` or `resolved`, with `source_app`, `source_file` and `source_key` recorded so any sentence can be traced back to the byte it came from.

**[STU-MAN-105] Where no capture supplies text, NOTHING IS INVENTED.** `summary` is set to NULL, the `author_id` is written into `tooltip-gaps.json`, and L1 renders without the sentence ([STU-SHL-232]). Inventing a plausible sentence is FORBIDDEN: an invented sentence is indistinguishable from a captured one at read time, it cannot be traced, and it silently converts a known gap into an unknown error. Handshake-only elements — surfaces with no vendor equivalent at all, such as the model-lane and command-API menus — get AUTHORED text in the SAME descriptor file with `extraction_method = handshake_authored`, so authored and derived stay countable and separable.

**[STU-MAN-106] Field sources for `summary` and `manual_anchor`.**

| Field | Read from |
|---|---|
| `summary` | tool-summary and tool-description strings from the one vendor that ships per-tool prose (212 sentences) and its command descriptions (1,532); type-library help strings (663); toolbar tooltips (73); video effect descriptions (246); compositing plain-English effect descriptions (62) |
| `manual_anchor` | per-effect support URLs on 359 effects resolving to **225 DISTINCT manual pages**. Every other anchor is authored once alongside its descriptor. |

The 225 figure is corrected from an earlier reading of 24; the difference is an order of magnitude and it matters, because 225 distinct vendor manual anchors is a serious seed for `manual_anchor` where 24 would have been near-useless.

**[STU-MAN-107] Anchor kinds (normative, closed).** `studio_tool`, `studio_param`, `studio_command`, `studio_panel`, `studio_region`, `studio_layout_preset`, `studio_task_scope`. `anchor_value` is ALWAYS the element's own `author_id`, which is what makes reverse lookup from a UI target to its manual entry ([STU-MAN-114]) a direct index read rather than a search.

**[STU-MAN-108] Manual entry shape for generated rows.** A generated manual entry satisfies [STU-MAN-001]'s two layers as follows: the OPERATOR layer takes `display_name`, `summary`, the navigation path derived from `menu_path`, and the expected result; the MODEL layer takes `command_id`, the typed input and output schema, dry-run availability, the receipt shape, undo semantics, the `author_id` targets, the proof path, the failure modes drawn from the `reason_code` set, and the recovery drawn from `remedy_command_id`. Both layers are generated from ONE record; neither is written by hand.

---

### 4. The honest ceiling and the gate that keeps it honest

**[STU-MAN-109] Measured coverage (normative, and it MUST NOT be softened).**

| Level | Coverage |
|---|---|
| `display_name` | ~100% |
| `summary` at CONTROL level | 78–86% |
| `summary` at TOOL level | **at most 58.0%** — 210 recoverable sentences against 362 tools — and **currently 0% mechanically bound** |

Only ONE captured application ships per-tool prose at all: 212 explanatory sentences across its tool-summary and tool-description string contexts. Every other captured application ships tool names and shortcuts with no adjacent description — one carries name plus a four-character type code plus a keystroke and nothing else; another carries a command id and a keystroke; a third's 39 tool rows all have a null label and a null menu location; a fourth's names were recovered by pattern from panel strings with no adjacent description; a fifth has no tool registry at all; two more carry names and shortcuts but no prose.

**57 tools are PROVEN to have no descriptive sentence at all**: their strings appear under BOTH the description context and the summary context with the tool's own NAME as the value, because the vendor fell back to the name where no summary was written. These are not missing data; they are confirmed absences and MUST be recorded as such rather than re-queried.

The 0%-bound figure is a JOIN problem, not an absence problem: that vendor's resource keys are the neutral English source text plus a bracketed context, and NOTHING in the resource file joins a tool's name to its summary. The closing action is SHL-P-05, one further extraction pass over the assembly IL recording which resource keys are loaded by the same type or method — the class that loads a tool's description is the class that loads its summary, and that join is IN the IL even though it is absent from the string table. Expected yield: tool-level mechanically bound prose from 0% to roughly 58%, and it resolves the 143 unbound tool identifiers of [STU-SHL-132] SD-7 at the same time.

The compositing fold-in does NOT dissolve this gap: it adds 62 plain-English effect descriptions and 225 manual pages, but effects are not tools, and the tool-level ceiling is unchanged.

**[STU-MAN-110] The gap ledger.** `tooltip-gaps.json` is a generated artefact, one row per element with a null `summary`, carrying `author_id`, `kind`, `family_id` where applicable, `provenance.source_app`, and a `proven_absent` boolean set true for the 57 confirmed absences of [STU-MAN-109]. It is a TRACKED BACKLOG, not an error log: a gap is a countable item of work, not a build failure.

**[STU-MAN-111] A missing tooltip is a TRACKED GAP, never an invented sentence.** The design MUST ship for a 40–60% prose cold start at the tool level. The rules that make that survivable:

1. a missing `summary` renders L1 WITHOUT the sentence and never blocks the tip ([STU-SHL-232]);
2. every gap is a row in the gap ledger;
3. an authored replacement carries `extraction_method = handshake_authored` and is therefore countable against derived text at any moment;
4. hand-writing prose inside a UI source file is a TYPE ERROR ([STU-SHL-245]), not a review finding.

**[STU-MAN-112] CI gates (normative).** All ten MUST run in continuous integration and all ten MUST fail the build when violated, except the last, which fails without a recorded waiver.

| Gate |
|---|
| 1. Every widget call site's `author_id` exists in the descriptor set — enforced by the generated enum, so this is a compile error |
| 2. Every descriptor has a non-empty `display_name` |
| 3. Every descriptor's `manual_anchor` resolves to a real UserManual section |
| 4. Every UserManual section is referenced by at least one descriptor — NO ORPHAN DOCUMENTATION |
| 5. No literal string may reach the tooltip or the AccessKit API: the helper signatures accept the id enum, not `&str` ([STU-SHL-245]) |
| 6. For every `kind == param` descriptor: hard bounds present OR `bounds_unknown` explicitly true |
| 7. For every `kind == param` descriptor: `format(accessible_value) == rendered_text` ([STU-MDL-113]) |
| 8. For every `kind == param` descriptor: `soft_min` and `soft_max` are PRESENT AS FIELDS even where they equal the hard bounds ([STU-SHL-172]) |
| 9. For every descriptor: `extraction_method` is present and drawn from the closed set ([STU-SHL-241]) |
| 10. **The `tooltip-gaps.json` count does not INCREASE between builds without an explicit recorded waiver** |

Gate 10 is the gate that keeps the coverage honest. It permits a 42% cold start and forbids the gap from silently growing, which is the realistic failure mode: a new surface ships with no prose, the count creeps, and the ceiling stops being a known number.

**[STU-MAN-113] Coverage is REPORTED, not asserted.** The build MUST emit a coverage report carrying: total descriptors by `kind`; `summary` coverage by `kind`; the split of `extraction_method` across the whole set; the gap count with its `proven_absent` subset broken out; and the delta against the previous build. A coverage claim in a work packet MUST cite that report, never an estimate.

---

### 5. Binding to the existing manual obligations

**[STU-MAN-114] Searchability extension.** [STU-MAN-004] requires the Studio manual to be queryable along four axes: tool name, task intent, `command_id`, and `author_id` reverse lookup. Because `anchor_value` IS the element's `author_id` ([STU-MAN-107]), the fourth axis is a direct index read. The generator MUST emit that index. Search MUST work for both audiences with no chat history and no network.

**[STU-MAN-115] Same-change currency for generated rows.** [STU-MAN-003] holds for generated rows exactly as for authored ones. Because the descriptor and the manual row are one record ([STU-MAN-102]), a behaviour change that does not regenerate its descriptor is DETECTED as a wired-surface diff rather than merely being against policy.

**[STU-MAN-116] Full-tool-surface coverage.** [STU-MAN-002] requires that a no-context model can discover the COMPLETE Studio tool surface from the manual alone. The mechanism is the exhaustive menu index ([STU-SHL-013]) projected through the descriptor set: every one of the 362 tools has a menu leaf, a command id, an `author_id` and a manual anchor, and `HELP > Full Command Index` enumerates them. Coverage completeness is checkable by CI gates 3 and 4 of [STU-MAN-112].

**[STU-MAN-117] Dual-audience layers at scale.** Every generated entry carries both layers of [STU-MAN-001] ([STU-MAN-108]). A generated entry whose operator layer has a null `summary` is still a valid entry: it carries the name, the navigation path, the shortcut, the availability contract and the complete model layer. A null summary degrades the operator layer; it never removes the entry.

**[STU-MAN-118] Diagnostic-posture linkage.** [STU-MAN-003] requires the three-tier diagnostic posture per entry ([STU-VAL-002]). For generated entries the posture is generated too: Tier 1 (Flight Recorder business events) is WIRED for every command; Tiers 2 and 3 carry `DEFERRED-with-reason` inherited from the module-level posture until those surfaces ship. Deferral is typed and per entry, never silently skipped.

---

### 6. Declared spec debt for this sub-section

**[STU-MAN-120] SD-9 — the manual entry shape for canvas tools is undecided.** The existing `UserManualToolEntry` carries transport fields — an IPC channel, a desktop command, a CLI flag, an HTTP route and an HTTP method — ALL of which are null for a canvas tool. Whether canvas tools need a new page kind or anchor kind, or reuse the tool-entry shape with null transport fields, is NOT decided. **This blocks SHL-P-06**, the generator itself, because the generator cannot emit rows whose shape is undecided. Owner: operator.

**[STU-MAN-121] SD-10 — the generator's manual-version behaviour is undecided.** A build guard fails a wired-surface diff without a manual-version bump. Whether a GENERATED BATCH bumps the manual version ONCE for the batch or once per element is not decided, and the answer changes the generator's contract. A per-element bump on a 14,700-parameter batch is clearly wrong; a per-batch bump must be shown not to defeat the freshness comparator. **This blocks SHL-P-06.** Owner: operator.

**[STU-MAN-122] SD-11 — the model-invokability floor is undecided.** [STU-CON-007] requires every Studio tool, command and primitive to be model-invokable, parallel-safe, deterministic and visually verifiable. Whether that means every one of the 362 tools needs its own inspector action AND its own manual tool entry with a real invocation path, or whether model-invokability is satisfied at the command-registry level with manual entries generated per COMMAND rather than per tool, sets the floor for how much of this design is MANDATORY rather than desirable. Recorded as [STU-SHL-136] OD-13. Owner: operator.

**[STU-MAN-123] Carried capture gaps affecting prose.** [STU-SHL-132] SD-6 (three of nine applications have no captured menu hierarchy) and SD-7 (143 tool identifiers and 53 panel identifiers unbound to names) both constrain what prose can be attributed to what element. Where an identifier is unbound, a descriptor MUST NOT guess a name; the element takes a Handshake name with `extraction_method = handshake_authored` and the unbound identifier is recorded in its provenance.

---

### 7. Obligations

**[STU-SHL-250] Universal command contract.** `HELP > Studio Manual`, the manual search surface, the L2 side sheet, the gap ledger export and the coverage report are all commands and MUST satisfy [STU-CON-007]: model-invokable, parallel-safe, deterministic and visually verifiable, and MUST NOT steal focus or open an uncontrolled window ([STU-QUIET-001]).

**[STU-SHL-251] Validation descriptors.** This sub-section contributes at minimum these `StudioValidationDescriptor` checks (14.24): `descriptor_missing_for_author_id`, `descriptor_display_name_empty`, `descriptor_summary_invented_without_provenance`, `descriptor_extraction_method_missing`, `manual_anchor_unresolved`, `manual_section_orphaned`, `tooltip_literal_string_at_call_site`, `accessible_description_diverges_from_summary`, `shortcut_baked_into_prose`, `tooltip_gap_count_increased_without_waiver`, `manual_row_written_to_deprecated_store`, `vendor_product_name_in_display_name`, `tooltip_fired_during_active_gesture`, `l1_dimmed_element_without_reason_line`.

**[STU-SHL-252] The naming law applies to every generated string.** [STU-SECTION-003] and [STU-SHL-008] apply to `display_name`, to `summary`, to every manual page title and to every anchor id. A vendor product name may appear ONLY inside `provenance.source_app`. A generated `display_name` containing a vendor product name is a build failure ([STU-SHL-251]).


---

### 8. Microtask Derivation

**[STU-SHL-253] Derivation rule (NORMATIVE).** The tooltip, help and manual-generation microtask set is derived from this sub-section MECHANICALLY, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. **Each numbered clause of this sub-section**, except the bookkeeping clauses named in [STU-SHL-254]. A clause states a contract, a rule, a structure or an enumeration that can be implemented and PROVEN independently, and it yields one microtask whether or not the sentence carrying it happens to use MUST: a stored contract may be stated in the indicative mood.
2. **Each ROW of a catalogue table** — a table whose FIRST COLUMN names a separate implementable subject rather than a facet of one subject. Each such row is its own microtask, because one microtask reading "362 tools" or "90 panels" is not implementable and would let the work disappear behind a number. The remaining columns of the row are that microtask's acceptance criteria.
3. **Each enumeration table, taken WHOLE** — its members are acceptance criteria of one microtask, not separate microtasks.
4. **Each command, shortcut, binding, preset or template table, taken WHOLE.** Binding a key is not a unit of implementation work and MUST NOT be one microtask per row.
5. **Each parameter table, taken WHOLE**, where the row's seven bound fields are its acceptance criteria.

This sub-section contains NO catalogue table, and that is correct rather than an omission: it specifies ONE generator, ONE descriptor record and ONE tooltip system, and the subjects those enumerate — 362 tools and 90 panels — are catalogued where they live, in [STU-SHL-155] and [STU-SHL-113]. The generator does not get 362 microtasks for producing 362 descriptors; the tools get 362 microtasks and the generator gets the ones that make its output correct for all of them.

**[STU-SHL-254] Clauses that yield NO microtask.** Exactly one class: the four clauses of this derivation sub-section itself — [STU-SHL-253] through [STU-SHL-256]. **Every other clause in this sub-section yields.**

**[STU-SHL-255] An open item still yields a microtask.** As [STU-SHL-118]. [STU-MAN-120] (the manual entry shape for canvas tools) and [STU-MAN-121] (the generator's manual-version behaviour) each yield a microtask whose FIRST acceptance criterion is obtaining the operator decision, because both BLOCK the generator step SHL-P-06 and the generator cannot be written around them. [STU-MAN-122] yields a microtask that sets the model-invokability floor. The 42% residual prose gap of [STU-MAN-109] is NOT a blocked microtask: it is a standing backlog measured by the gap ledger and gated by CI gate 10, and each gap row becomes an authored-prose microtask only when the operator commissions it. The ten CI gates of [STU-MAN-112] are acceptance criteria of one gate-suite microtask rather than ten microtasks, because they are ten assertions over one generated artefact set; the same holds for the validation descriptors of [STU-SHL-251].

**[STU-SHL-256] Yields index.** Applying [STU-SHL-253] through [STU-SHL-255] to this sub-section yields the counts below. Every count is enumerated from the module text, not estimated.

| Unit group | Clauses | Yields |
|---|---|---|
| The five disclosure levels and the anti-annoyance rules | [STU-SHL-230]-[STU-SHL-236] | 7 |
| The UiDescriptor data contract and the type-level prose prohibition | [STU-SHL-240]-[STU-SHL-247] | 8 |
| Obligations, validation descriptors and the naming law | [STU-SHL-250]-[STU-SHL-252] | 3 |
| The generator, its target store, its artefacts and its field sources | [STU-MAN-100]-[STU-MAN-108] | 9 |
| Coverage measurement, the gap ledger and the CI gates | [STU-MAN-109]-[STU-MAN-113] | 5 |
| Binding to the existing manual obligations | [STU-MAN-114]-[STU-MAN-118] | 5 |
| Declared spec debt | [STU-MAN-120]-[STU-MAN-123] | 4 |
| **Module total** | 45 clauses | **41** |

Every row here is one-per-clause. An earlier revision of this ledger claimed 75 by counting the five tooltip levels, the four synchronisation rules, the ten CI gates, the two manual layers and the fourteen validation descriptors as separate microtasks. They are not: each is an assertion over one artefact — one tooltip helper, one descriptor record, one generated set — and splitting them produces microtasks that cannot be finished independently. They are acceptance criteria of their clause, and the ledger is corrected to say so. The one genuine omission that correction exposed was L3, the rich tip, which had no clause of its own while the other four disclosure levels did; it now has [STU-SHL-236] and is derived.

**[STU-SHL-256A] Anchor binding.** As [STU-SHL-119A]. A microtask derived here cites its clause anchor directly and clears `spec_anchor_status` on binding.
