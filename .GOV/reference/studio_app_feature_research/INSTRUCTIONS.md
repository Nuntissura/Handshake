# How to Use and Edit This Research Package

## [INSTR.what] What This Is

This is a source-backed feature inventory for rebuilding Photoshop, Affinity Photo/Designer/Publisher, InDesign, Illustrator, and Figma-class behavior as native local-first Handshake Studio tools.

It is reference material only. It does not authorize product code, spec edits, task status changes, or validator gates by itself.

## [INSTR.layout] Layout

- `index.yaml` is the machine-readable map.
- `00-preamble.md` explains scope, schema, risks, and recommended build approach.
- `01-photoshop-feature-map.md` records Photoshop and Camera Raw feature families.
- `02-affinity-suite-feature-map.md` records Affinity Photo, Designer, and Publisher feature families.
- `03-indesign-feature-map.md` records InDesign feature families.
- `04-affinity-leaf-index.md` records machine-expanded Affinity help table-of-contents leaves.
- `05-studio-primitive-map.md` maps vendor feature domains to Handshake-native Studio primitives and Rust engine modules.
- `06-photoshop-leaf-index.md` records machine-expanded Photoshop help table-of-contents leaves.
- `07-indesign-leaf-index.md` records machine-expanded InDesign help table-of-contents leaves.
- `08-gap-resolution-notes.md` records resolved, mitigated, and residual research gaps.
- `09-affinity-desktop-delta.md` records Affinity desktop help delta rows.
- `10-studio-command-contracts.md` records the command-contract seed.
- `11-provider-posture-map.md` classifies provider/cloud/compatibility posture for the first app set.
- `12-cross-app-parity-matrix.md` maps Photoshop, Affinity, and InDesign parity lanes to Studio primitives.
- `13-layer-graph-vertical-slice.md` records the first non-destructive layer graph vertical slice.
- `14-feature-use-card-schema.md` defines the Feature Use Card planning shape.
- `15-photoshop-feature-use-cards.md` records generated Photoshop Feature Use Cards.
- `16-affinity-feature-use-cards.md` records generated Affinity Feature Use Cards.
- `17-indesign-feature-use-cards.md` records generated InDesign Feature Use Cards.
- `18-feature-use-card-manual-handoff-index.md` groups all generated cards into future Studio UserManual surfaces.
- `19-studio-local-first-rust-posture.md` records the local-first, no-cloud-required, Rust-forward Studio posture.
- `20-illustrator-feature-map.md` records Illustrator feature families.
- `21-figma-feature-map.md` records Figma feature families across Design, Draw, FigJam, Motion, Slides, Sites, Buzz, Make, Dev Mode, API, AI, and collaboration.
- `22-illustrator-leaf-index.md` records machine-expanded Illustrator help leaves.
- `23-figma-leaf-index.md` records machine-expanded Figma help leaves and verified category evidence.
- `24-illustrator-feature-use-cards.md` records generated Illustrator Feature Use Cards.
- `25-figma-feature-use-cards.md` records generated Figma Feature Use Cards.
- `26-illustrator-figma-provider-posture-map.md` classifies Illustrator/Figma provider, cloud, collaboration, and compatibility posture.
- `27-illustrator-figma-parity-matrix.md` maps Illustrator/Figma parity lanes to Studio primitives.
- `28-adobe-count-methodology.md` defines count semantics and proof rules for online-source Adobe app distillation.
- `29-photoshop-expanded-count-ledger.md` records Photoshop/Camera Raw expanded online-source counts and installed-enrichment hooks.
- `30-indesign-expanded-count-ledger.md` records InDesign expanded online-source counts and installed-enrichment hooks.
- `31-illustrator-expanded-count-ledger.md` records Illustrator expanded online-source counts and installed-enrichment hooks.
- `32-adobe-installed-ui-export-playbook.md` records optional installed-app enrichment for IDs, shortcuts, panels, context states, and file-dialog detail.
- `33-online-source-distilled-feature-ledger.md` records the unified source-distilled feature/tool ledger for Photoshop, InDesign, Illustrator, Affinity, and Figma.
- `34-photoshop-source-distilled-domain-ledger.md` records online-source-distilled Photoshop and Camera Raw feature/tool domains.
- `35-indesign-source-distilled-domain-ledger.md` records online-source-distilled InDesign feature/tool domains.
- `36-illustrator-source-distilled-domain-ledger.md` records online-source-distilled Illustrator feature/tool domains.
- `37-affinity-source-distilled-domain-ledger.md` records online-source-distilled Affinity Photo, Designer, and Publisher feature/tool domains.
- `38-figma-source-distilled-domain-ledger.md` records online-source-distilled Figma product-family feature/tool domains.
- `39-photoshop-source-distilled-feature-rows.md` records generated source-distilled Photoshop feature rows.
- `40-indesign-source-distilled-feature-rows.md` records generated source-distilled InDesign feature rows.
- `41-illustrator-source-distilled-feature-rows.md` records generated source-distilled Illustrator feature rows.
- `42-affinity-source-distilled-feature-rows.md` records generated source-distilled Affinity feature rows.
- `43-figma-source-distilled-feature-rows.md` records generated source-distilled Figma feature rows.
- `44-cross-app-overlap-and-affinity-dedupe-map.md` records the generated cross-app overlap policy and Affinity dedupe map.
- `45-source-distilled-tool-registry.md` records generated source-distilled tool, panel, command, output, workflow, API, and workspace-mode registry rows from the app domain ledgers.
- `46-file-format-compatibility-registry.md` records generated native, import, export, round-trip, and fixture-required format compatibility targets.
- `47-studio-rust-implementation-backlog.md` records generated implementation-facing Studio primitive backlog lanes for future local-first Rust work.
- `48-provider-offline-parity-registry.md` records generated provider/offline parity rows that keep cloud, AI, collaboration, runtime, automation, and compatibility-adjacent source behavior local-first in Studio.
- `49-source-coverage-verification-matrix.md` records generated per-feature coverage verification for required planning fields, source reference strength, provider/offline linkage, format linkage, tool linkage, and implementation obligations.
- `50-proprietary-format-fixture-plan.md` records generated fixture, round-trip, unsupported-feature receipt, and Rust lane requirements for native/proprietary/local-copy format compatibility targets.
- `51-photoshop-deep-feature-delta.md` records the sub-TOC Photoshop/Camera Raw deep delta: tools, menus, filters, adjustments/blending, channels, Camera Raw, type engine, panels, automation/scripting, dialogs, smart objects, preferences.
- `52-illustrator-deep-feature-delta.md` records the sub-TOC Illustrator deep delta: tools, menus, full Effect catalog, appearance/styles, color systems, typography, artboards, import/export, scripting, panels, preferences, cloud/AI posture.
- `53-indesign-deep-feature-delta.md` records the sub-TOC InDesign deep delta: tools, menus, text/typography option catalogs, styles, pages/layout, tables, frames, color/output, interactive/EPUB, long-document, prepress, scripting, panels, preferences.
- `54-affinity-deep-feature-delta.md` records the desktop-grounded Affinity deep delta: personas/StudioLink, per-persona toolsets, adjustments/live filters/blend modes, selections, color, typography, Publisher production depth, formats, automation, panels, 2.x version deltas.
- `55-figma-deep-feature-delta.md` records the Figma-family deep delta: design core/auto layout, vector networks/Draw, typography, components/variables, prototyping, Dev Mode/Code Connect/MCP, FigJam, Slides/Sites/Buzz/Make, collaboration, Plugin API, REST API, AI, org posture.
- `56-studio-handshake-integration-architecture.md` records the Studio-Handshake integration architecture: pillar wiring, model visibility, visual steerability, parallel workflows, propose-work, per-file history/undo, visual inspection duty, headless/quiet law, operator unification, dual-audience UserManual strategy.
- `57-deep-delta-cross-app-overlap-map.md` records the generated cross-app overlap map over the deep-delta files (no-double-features grouping under one Studio primitive per shared capability).
- `_source_snapshots/` stores fetched official source snapshots used by generated leaf indexes.
- `_tools/generate-source-distilled-feature-rows.py` regenerates `39` through `43` from the Feature Use Cards and domain ledgers.
- `_tools/generate-cross-app-dedupe-map.py` regenerates `44` from `34` through `43`.
- `_tools/generate-source-distilled-tool-registry.py` regenerates `45` from `34` through `38` and `44`.
- `_tools/generate-file-format-compatibility-registry.py` regenerates `46` from `34` through `43`.
- `_tools/generate-studio-rust-implementation-backlog.py` regenerates `47` from `05`, `10`, `18`, and `39` through `46`.
- `_tools/generate-provider-offline-parity-registry.py` regenerates `48` from `39` through `43`.
- `_tools/generate-source-coverage-verification-matrix.py` regenerates `49` from `39` through `48`.
- `_tools/generate-proprietary-format-fixture-plan.py` regenerates `50` from `46`, `47`, and `49`.
- `_tools/generate-deep-delta-overlap-map.py` regenerates `57` from `51` through `55`.

## [INSTR.headers] Header Convention

Each topic file uses:

1. YAML frontmatter.
2. One topic header: `## [<TOPIC_ID>] <Title>`.
3. Greppable subtopic headers: `### [<TOPIC_ID>.<suffix>] <Title>`.
4. Fenced YAML blocks for machine-ingestable feature records.
5. A final `### [<TOPIC_ID>.sources] Sources` block at EOF.

Generated ledgers `39` through `44`, `48`, and `49` additionally wrap sections in flat `<topic>` tags to preserve machine-parseable topic boundaries. Keep both the `##`/`###` headers and the flat topic wrappers when regenerating them.

Feature IDs are stable. Do not rename or renumber them unless a later reconciliation file records the supersession.

## [INSTR.edit] Editing Discipline

- Add new records inside fenced YAML blocks.
- Preserve `id`, `name`, `app_behavior`, `primitive_domain`, and `source_ids` on every feature.
- If a source-backed claim is uncertain, add `verification_status: UNVERIFIED`.
- Update `index.yaml` if feature counts, summaries, topics, known gaps, or source policy changes.
- Keep file and folder names hyphenated or underscored; do not create names with spaces.
- Do not stop at help-leaf card totals when documenting all tools/features.
- Treat online sources as sufficient to produce the source-distilled feature/tool rebuild inventory.
- Treat installed exports as optional enrichment for exact ids, shortcuts, locale/version context, hidden states, and file-dialog fixtures.
- Do not remove Affinity rows because similar Adobe rows exist. Use `44-cross-app-overlap-and-affinity-dedupe-map.md` to group shared behavior under one Studio primitive while preserving Affinity-specific source variants.
- Do not replace source creative file formats with a new interchange format. Use `46-file-format-compatibility-registry.md` to drive import/export/round-trip fixture planning and unsupported-feature diagnostics.
- Do not treat `47-studio-rust-implementation-backlog.md` as product authority. It is a bridge into future work packets/specs; every selected item still needs command-contract promotion, exact behavior inspection, fixtures, diagnostics, and a same-change Studio UserManual entry.
- Do not turn provider, cloud, AI, collaboration, community, hosted API, or compatibility behavior into mandatory online dependencies for Studio. Use `48-provider-offline-parity-registry.md` to define the local-first primitive, optional adapter, fallback, receipt, and verification posture for those rows.
- Use `49-source-coverage-verification-matrix.md` before claiming coverage completeness. If rows are marked `source_distilled_complete_without_local_snapshot_path`, they still have source URLs and required planning fields, but need local snapshot capture if the project requires fully reproducible offline source evidence.
- Use `50-proprietary-format-fixture-plan.md` before claiming compatibility with native/proprietary/local-copy formats. Compatibility claims require fixtures, round-trip receipts, unsupported-feature receipts, and same-change Studio UserManual entries.

## [INSTR.next] Recommended Next Passes

1. Explode each category-level row into leaf tool/command rows from official help indexes.
2. Add a `studio_primitive` field for Handshake-native primitives such as `LayerGraph`, `SelectionMask`, `ColorPipeline`, `LayoutFrame`, `PageSpread`, `ExportRecipe`, and `ModelToolContract`.
3. Add a parity matrix that maps vendor features to reusable Rust crates, internal engine modules, data schemas, diagnostics, and tests.
4. Add a red-team pass for impossible parity, licensing traps, cloud-only behavior, AI-provider lock-in, and performance risks.
