---
file_id: LANE-SMB-SETTINGS-SCHEMA-1
file_kind: lane_brief
updated_at: 2026-10-31T11:29:59Z
wp: WP-KERNEL-STUDIO
lane: SMB-SETTINGS-SCHEMA-1
lane_file: builders/lanes/hsk-studio-settings-schema-1.json
crates: [hsk-studio-accord (additive settings schema), hsk-studio-pigment (selection entries), hsk-studio-develop (export-stage entries), hsk-studio-harness-view (brush entries and Studio catalog, egui-free files only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="RF-09 phase 0: typed settings schema with stable ids, PreferenceSchemaEntry-compatible; asset side of MT-36249">

## Target

RF-09 phase 0 of `research/integration-refactor-inventory-20261030.json`, asset side of MT-36249 (AC-HOST-001/002 asset-local evidence only; AC-HOST-003..005 are host phase 1). Spec: GUI-DESKTOP-014..019 (`spec-modules/15-native-desktop-gui-vulkan-gpu-ownership.md:52-62`), SET-REC-001..004 and SET-SCOPE-001 (`spec-modules/10-product-surfaces.md:8294-8325`). Engine lane under D6: harness-view edits only in egui-free files.

**Owner: accord.** Every Studio crate may depend on accord with no new edge (root `Cargo.toml` `[workspace.metadata.studio.dependencies]`); controls is reachable only from harness-core/-view/harness, not from pigment or develop; a new crate is a root input change. Accord has no dependencies (its `Cargo.toml`): no serde; typed values plus a hand-written deterministic JSON projection, as accord's own `to_json` (`src/lib.rs:552`).

**Handshake mapping** (read-only, `wtc-native-editors-v1` @292d7a1c, `src/backend/handshake_core/src/preferences/mod.rs`):
- `PreferenceSchemaEntry` :252-268: `preference_id` `<namespace>.<key>` :253-254, `namespace` :256, `scope_kind` :258, `label` :260, `constraint` :262, `redaction_class` :264, `default_value` :267.
- `PreferenceConstraint` :198-211 (Bool :200, IntRange{min,max} :202, FloatRange{min,max} :204, Enum :206; SyntaxColorMap :208 and ChordMap :210 are editor-only, no Studio entry needs them); value types :33-59, :215-225; scopes global/workspace/surface :79-96; redaction public/non-public :170-184.
- `validate` :278-393 codes: `not_bool` :288, `not_int` :296, `out_of_range` :302/:319, `not_float` :310, `not_finite` :315, `not_string` :327, `unknown_enum_member` :333; error `{preference_id, code, message}` :231-238.
- Registry invariants: unique ids :740-754; every default validates its own constraint :757-763; static registry only (`editor_preference_registry` :607, lookup :721).
- API `src/api/preferences.rs`: unknown id 404 `unknown_preference` :128-138; validation before commit, 400 `preference_validation_failed` :214-223; projection `hsk.preference_projection@1` :172-177. No module registration API exists (Handshake prerequisite).
- **New gap (for the lead, extends SD-07):** SET-SCOPE-001 (`10-product-surfaces.md:8294-8298`) admits only `data-roots`, `view-defaults`, `retention`, `feature-toggles`; Studio tool defaults fit none. Entries carry namespace `studio` marked `namespace_status: spec_delta_pending`; never claim a SET-SCOPE namespace. UNVERIFIED until the spec owner decides.

### Slices, one local commit each, red tests first

1. **accord `src/settings.rs` (new, additive):** `SettingId` (grammar `studio.<module>.<feature>.<key>`, lowercase ASCII and `-` segments, accord id bounds); `SettingConstraint {Bool, IntRange{min,max}, FloatRange{min,max} (finite), Enum(&'static [&'static str])}`; `SettingValue`; `SettingScope {Global, Workspace, Surface}`; `Redaction {Public, NonPublic}`; `SettingEntry {id, module, feature, label, unit: Option, constraint, default, scope, redaction, bound_source: Spec | StudioDeclared}`; `SettingsSchema::new(&[&[SettingEntry]])` refuses typed: duplicate id, min > max, empty or duplicate enum member, default outside its constraint, entry count over a declared max; `validate(id, &SettingValue)` returns `{id, code}` with the Handshake codes plus `unknown_setting`; `to_preference_json()` deterministic with the field names above. Do NOT add to `CANONICAL_SCHEMAS` (pinned at 38, `tests/foundation_contract.rs:264`); add a `"settings"` key to `DESCRIPTOR` (`src/lib.rs:1036`).
2. **pigment `src/settings.rs`:** `WandOptions` (`src/select_tools/wand.rs:27`, Default :35: tolerance, contiguous, sample_merged, sample_size, connectivity) and `MagneticOptions` (`src/select_tools/magnetic.rs:27`, Default :38: width, contrast, frequency, sample_merged, margin_px). The engine admits any finite tolerance >= 0 (wand.rs:24-25): the entry bound lies inside the engine domain; a test feeds every bound value to the tool and it is accepted.
3. **develop `src/settings.rs`:** `OutputSizing` (`src/export_stage.rs:158`, Default :178), `OutputSharpening` (:441, Default :449), scalar fields of `WatermarkSettings` (:350, Default :365; the graphic/text source gets no entry). Enum members reuse the existing `spec_member` strings (:98, :131, :148, :402, :420).
4. **harness-view:** brush entries beside `BrushSettings` (`src/view/edit.rs:206`, Default :214; no egui in the file) and new egui-free `src/view/settings_catalog.rs`: `studio_settings_catalog()` = one `SettingsSchema` over brush, pigment and develop entries, one feature subtab per module (brush, selection, export), and a tree walk that finds every entry by id (AC-HOST-001 asset side). The catalog audit reports through observe with codes and counters only (accord has no observe edge, so accord returns typed codes).
- Every slice: drift test, entry default == the struct's `Default` field for field.

### Out of scope (listed)
- harness-view surface files (GUI paused): `FillSettings` `src/view/fill_tools.rs:115` (egui :685-739); `PsSettings` `src/view/select_liquify.rs:48` non-engine fields (quick_diameter_px, liquify_*, refine_*; its wand and magnetic members are slice 2); `Prefer` `src/view/render.rs:46` (`egui::TextureId` in public items :59, :76); audio `ImportOptions` `src/view/audio_import.rs:77`.
- `DeviceOptions` (render-gpu `src/device.rs:93`): budgets belong to the GPU owner (field doc :99-101) and render-gpu is SMB-R47-FOLLOWUPS-1's crate: phase 1 (MT-203, MT-36248).
- `ImportSettings` (harness-core `src/import.rs:313`): harness-core is SMB-R47-FOLLOWUPS-1's crate this wave; next settings lane.
- Audio device: score-device `StreamConfig` (`src/config.rs:40`) has no `Default`; device selection is outside score-device (`DESCRIPTOR` not_in_scope, `src/lib.rs:52`): phase 1. CLI-flag mapping (harness `src/cli.rs`, AC-HOST-002): later. Controls Registry ids for settings (rule 3): phase 1 with the host tab.
- **Engine Profiles: phase 1.** A provider descriptor needs a host/asset API: `handshake_dexterity` (assets `hskernel/handshake_dexterity/src/profile.rs`) knows only `Engine {GenieXQairt, GenieXLlamaCpp}` :8 and `Device {Cpu, Npu}` :12 (`EngineProfile` :46), no render provider; it is outside the Studio workspace (no edge, no git deps) and host adoption MT-196 is BLOCKED (inventory `settings_engine_profiles`).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "module_tab_feature_subtab", "spec_refs": ["GUI-DESKTOP-014", "GUI-DESKTOP-015", "GUI-DESKTOP-016", "GUI-DESKTOP-017", "GUI-DESKTOP-018", "GUI-DESKTOP-019", "SET-REC-001", "SET-REC-002", "SET-REC-003", "SET-REC-004", "SET-SCOPE-001"], "refactor_exposure": "low", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"settings": "phase 0 typed schema and entries only; the host tab, persistence through PreferenceRecord and Handpick search are MT-36249 phase 1", "attribution": "no mutating command in phase 0; setting a value is a host PreferenceRecord write", "diagnostics": "MT-36249's flight_recorder and palmistry rows are host rows"}, "recorded_at": "2026-10-31T11:29:59Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths and do-not-touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-settings-schema-1`, branch `lane/smb-settings-schema-1`, base 4f69b03b)

- accord: new `src/settings.rs`, `src/lib.rs` (mod, pub use, `DESCRIPTOR` key), new `tests/settings_schema.rs`.
- pigment and develop: new `src/settings.rs`, one `src/lib.rs` mod line each, new `tests/settings_entries.rs` each.
- harness-view: `src/view/edit.rs` (entries beside `BrushSettings` only), new `src/view/settings_catalog.rs`, one mod line in `src/view/mod.rs`, new `tests/settings_catalog.rs` (`#![cfg(feature = "viewer")]`, round unit `["viewer"]`, `.config/round-features.toml:44`).
- No `Cargo.toml` change, no root input, no egui item, no harness-view surface file, no render-gpu, harness-core or harness edit.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per slice; 5 or fewer focused tests per slice; vendor behaviour UNVERIFIED (D5).
- One bundled run (lane-check semantics, one lock hold): step 0; clippy over accord, pigment, develop, harness-view `[viewer]` and their direct dependents (48 crate manifests name accord at 4f69b03b, so effectively the workspace); nextest WHOLE suites (rule 4a) of accord, pigment, develop, harness-view `["viewer"]` and `["desktop", "audio-batch"]`; `cargo test --doc -p hsk-studio-accord` if doc examples are added (4d). Name the log per RUN LOGS.
- **Push LAST of this wave**, after SMB-AE-PARTICLES-2, SMB-PDF-PRESETS-1, SMB-ID-BOOKLET-1 and SMB-R47-FOLLOWUPS-1: accord and pigment sit in all their closures and harness-view's closure holds all of their crates, so running last costs one run per lane. Do NOT start the bundled run until the lead names the disk and the base.
- Dispatch after the R48 verdicts: a FAIL on SMB-ATTRIBUTION-1 or SMB-BUSINESS-EVENTS-1 (accord) or SMB-JOB-ADMISSION-1 (harness-view) reopens remediation in these crates. SMB-FIG-GRID-3 (PAUSED, local harness-view commits) rebases over this lane.
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
