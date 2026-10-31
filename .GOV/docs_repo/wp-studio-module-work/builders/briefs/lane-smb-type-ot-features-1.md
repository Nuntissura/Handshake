---
file_id: LANE-SMB-TYPE-OT-FEATURES-1
file_kind: lane_brief
updated_at: 2026-10-31T18:38:25Z
wp: WP-KERNEL-STUDIO
lane: SMB-TYPE-OT-FEATURES-1
lane_file: builders/lanes/hsk-studio-type-ot-features-1.json
crates: [hsk-studio-type (OpenType feature-tag registry and map, font feature availability query, legacy boolean / figure style / stylistic-set projections, special-character catalogs)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="STU-TYP-170..175 and STU-TYP-017: the OpenType feature surface behind the InDesign/Illustrator OpenType panels; STU-TYP-207 insertable character catalogs">

## Target

Engine-only under D6. At 63ecf5a6 Type applies caller features per run (`src/contract.rs:60` `Feature {tag, value: u32, range, declared}`; `src/engine.rs:347-385` builds the shape plan, asks `plan.feature_available` and records `Applied` or `UnavailableInert`, `src/result.rs:13-21`; a declared but missing feature is `UnsupportedFeature`, `:373`). There is no registry, no group projection, no per-font availability query, no legacy projection and no character catalog (`git grep -i "feature_registry|check_opentype_feature|figure_style|em.?space"` over type/src finds nothing).

**Spec** (`spec-modules/14-studio-creative-suite.md`, v02.225): STU-TYP-017 :9527; STU-TYP-170 :8516 (ordered tag -> bool or alternate index; registry of at least the 229 observed tags; any well-formed tag accepted; no closed list); **recovered record STU-TYP-171 :8523** (20 groups with their tags, table :8528-8549; presentation only); STU-TYP-172 :8551 (applied at shaping, never rewrites text, unavailable = inert, stored, reported); STU-TYP-173 :8556 (`TextEngine` query: declared tags + per-tag alternate counts; `check_opentype_feature(tag)`); STU-TYP-174 :8562 (14 legacy booleans stored as tag entries, documented mapping, no second storage); STU-TYP-175 :8569 (`figure_style` five values -> lnum/onum x pnum/tnum; `stylistic_sets` bitfield -> ss01..ss20 at the import boundary); STU-TYP-207 :8936 (four catalogs; flush space, fixed and flexible non-breaking spaces are distinct).

**MTs** (write `crates/hsk-studio-type/**`): closes MT-30957 (semantics), MT-30959 (legacy booleans), MT-31140 and MT-31141 (STU-TYP-017, gui_host none). Advances MT-30955 (the 229-tag observed union is NOT in the spec: AC-001 stays open as a spec gap), MT-30960 (its Package round trip, step 2, is outside type), MT-30956 and MT-30958 (vulkan_shell_client: engine half only), MT-31005..31009 (catalog half; composition behaviour lives in layout, an R50 crate).

### Slices, one local commit each, red tests first

1. **Registry and tag map** (new `src/features.rs`): registry = every tag of :8528-8549 verbatim plus ss01..ss20 and cv01..cv99, grouped as the table says (a tag in two groups, e.g. `abvm`, `blwm`, keeps both memberships); a typed ordered map tag -> `Binary(bool) | Alternate(u32)` converting to and from `Feature`; malformed tags refused with the existing well-formedness rule (`src/validate.rs:212`), unregistered well-formed tags accepted and applied. A test pins the registry count and records the 229 gap (the observed union is not recovered). STU-TYP-172: a stored feature the font lacks round-trips as `UnavailableInert` and the shaped text bytes are unchanged.
2. **Availability query** (`TextEngine`, `src/engine.rs:7`): for one font resource, the sorted, deduplicated GSUB and GPOS feature tags over all scripts and language systems, and per tag the alternate count (largest `AlternateSet`, vendored `src/provider/ttf/tables/gsub.rs:108`, over the feature's type-3 lookups; 0 otherwise); `check_opentype_feature(tag)`. Fonts only through `fonts::load` (`src/fonts.rs:9`: resolver, revision and hash checks; no file reads). Hostile fonts (V4-01/02): lookup and feature counts bounded before allocation; a truncated FeatureList and an out-of-range lookup index give typed refusals; a `dims_u32_max` and an `aggregate_over_budget` test. Expected tags for `tests/fixtures/fonts/Inter-Regular.ttf` and `NotoSansArabic-Regular.ttf` come from an independent reader (fontTools `ttx -t GSUB -t GPOS` if available; else say UNVERIFIED in the lane notes); never from this code's own output.
3. **Projections** (STU-TYP-174/175): the 14 booleans read and write the tag map (no second field). Mappings whose tag the spec or a public source names (e.g. `ligatures` -> `liga`, `discretionary_ligature` -> `dlig`, `fraction` -> `frac`, `slashed_zero` -> `zero`) are Studio readings with the source in the module docs; a boolean with no named tag (candidates `overlap_swash`, `stretched_alternate`, `hv_kana`) refuses typed `legacy_projection_unmapped`, never a guess. `figure_style` five values <-> the two tag pairs; the stylistic-set bitfield <-> ss01..ss20 (bit order is a Studio reading, UNVERIFIED; bits beyond ss20 are a typed loss).
4. **Character catalogs** (new `src/catalog.rs`, STU-TYP-207): four catalogs with stable member ids; real characters carry their Unicode scalar (Unicode Standard values; the vendor mapping UNVERIFIED) and typed composition flags (`stretches`, `break_opportunity`, `absorbs_final_line_slack`) so layout's breakers can consume them later; markers and page, column and frame breaks have no Unicode character: their stream sentinel is NOT assigned here (`stream_form: pending`, a spec-owner item), so no second marker convention appears beside Folio's `StudioRunningPart` (`hsk-studio-folio/src/longdoc.rs:1020`).
- Do not change the HarfRust path (`src/engine.rs:24-33`, a separate Operator decision). Terminal outcomes: codes only through observe (the `report` pattern of `src/finalization.rs:384`). `descriptor()` gains a `features` entry.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-TYP-017", "STU-TYP-170", "STU-TYP-171", "STU-TYP-172", "STU-TYP-173", "STU-TYP-174", "STU-TYP-175", "STU-TYP-207", "STU-SECTION-003", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045"], "refactor_exposure": "none", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"gui_host": "MT-30956 and MT-30958 are vulkan_shell_client with shared_typed_source descriptors; this lane builds their engine half and registers no command, so the OpenType panel and its descriptors stay open (D7)", "attribution": "queries and projections are pure; storing a tag map on a text range is a Folio/host commit outside this lane", "diagnostics": "codes and opaque ids only; tags, text and font names never reach observe"}, "recorded_at": "2026-10-31T18:38:25Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-type-ot-features-1`, branch `lane/smb-type-ot-features-1`, base 63ecf5a6)

- type: new `src/features.rs`, `src/catalog.rs`, the query method in `src/engine.rs` (additive), `mod`/`pub use` lines in `src/lib.rs`, `src/descriptor.rs`, read-only use of `src/provider/ttf/**` (an accessor there only if none exists, additive), new `tests/opentype_features.rs` and `tests/character_catalogs.rs` (test fn names carry the MT recipe names `mt_30955`, `mt_30957`, `mt_30958`, `mt_30959`, `mt_30960`, `mt_31005`..`mt_31009`, `mt_31140`, `mt_31141`; record the mapping).
- No `Cargo.toml` change, no new fixture font (use the checked-in fonts), no root input, no new dependency.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over type and its direct dependents by manifest (composite, interop-fig-native, interop-fig, interop-layout, layout, render-cpu, render-gpu; harness-view and harness through `--features viewer`, 4e) plus `-p hsk-studio-type --features harfrust` (round unit, `.config/round-features.toml:16`); nextest WHOLE type suite in the default and `harfrust` units (4a, 4e); doctests if added (4d). Name the log per RUN LOGS. Then `lane-check.sh --owner smb-type-ot-features-1 --per-sha origin/codex/handshake-studio-assets`; push only on `PER_SHA_RESULT OK`.
- Push order: type's closure (accord, observe) holds no other lane of this wave; any order with the other three. Type sits in the closures of R50's composite, layout and interop-psd (SMB-AE-WATER-SIMS-1, SMB-ID-PDF-EXPORT-PREFS-1, SMB-FOLIO-CHANNEL-RECORD): push only when no R50 remediation run holding type is in flight. Do NOT start the bundled run until the lead names the disk and confirms R50 judged SMB-R49-FOLLOWUPS-1 (accord, observe).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
