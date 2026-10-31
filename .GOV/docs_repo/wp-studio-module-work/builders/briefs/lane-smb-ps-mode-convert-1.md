---
file_id: LANE-SMB-PS-MODE-CONVERT-1
file_kind: lane_brief
updated_at: 2026-10-31T18:38:25Z
wp: WP-KERNEL-STUDIO
lane: SMB-PS-MODE-CONVERT-1
lane_file: builders/lanes/hsk-studio-ps-mode-convert-1.json
crates: [hsk-studio-prism (document mode records and depth availability, grayscale -> bitmap, RGB -> indexed, indexed colour table and colour-table file codec)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Photoshop Image > Mode: STU-COL-118..121 mode and depth contract, Bitmap conversion (five methods), Indexed conversion (palettes, forced colours, matte, dither) and the indexed colour table with its file codec">

## Target

Engine-only under D6. No mode-conversion engine exists at 63ecf5a6: `git grep -i "dither|halftone|to_bitmap|to_indexed|document_mode"` over the crates finds only interop-psd's PSD header colour-mode enum (`hsk-studio-interop-psd/src/header.rs:13`, read side). Prism already owns colour maths that this lane reuses (`src/recolor.rs`: `luminance` :182, `srgb_to_lab` :187, `delta_e` :198, `reduce_colours` :430, bounded by `MAX_REDUCE_COLOURS` :50) and the hostile-text precedent `parse_cube` (`src/lut.rs`, re-exported `src/lib.rs:32`).

**Spec** (`spec-modules/14-studio-creative-suite.md`, v02.225): STU-COL-106 :9774 (seven-field parameter record); STU-COL-118 :9904 (eight modes, five creation modes, seven change operations, no `to_duotone`); **recovered records STU-COL-119 :9913**: discard warning + one undo (:9914-9915), Bitmap six fields with method default `diffusion_dither` and six halftone shapes (:9917-9921), its table (`bitmap.resolution` default 72 ppi, the other bounds UNKNOWN; :9924-9929), Indexed eight fields with twelve palettes (default `exact`), forced, transparency, dither, matte (:9932-9938), its table (`indexed.colors` hard_max 256, `indexed.dither_amount` 1..100; :9943-9946), read-only colour counts (:9948-9950), the indexed colour table resource and its four shipped tables (:9952-9955); STU-COL-120 :9974 (1-bit only for bitmap; 32-bit unbounded linear); **STU-COL-121 :9979** (per-model depth matrix, a Studio choice, :9983-9992); STU-SECTION-003 :118.

**MTs** (gui_host none, write `crates/hsk-studio-prism/**`): closes MT-31176 (bitmap parameter rows), MT-31177 (indexed parameter rows), MT-31179 (depth matrix). Advances MT-31174 (enumerations, legality and the discard receipt; the document record and the one-undo commit are Folio/host), MT-31175 (bitmap and indexed records; duotone, multichannel and grayscale tonal mapping open), MT-31178 (depth rules and per-operation availability; HDR pixel processing is not this lane).

### Slices, one local commit each, red tests first

1. **Mode contract** (new `src/mode.rs`): the three enumerations; `BitmapOptions` and `IndexedOptions` with every field, the recovered defaults and seven-field parameter rows (UNKNOWN stays `None`, never a soft clamp, STU-COL-106); method-specific fields kept inert when unused and round-tripped; the depth matrix as a pure query; `plan_mode_change(from, to, depth, channel_set)` refuses illegal pairs typed (`to_duotone`, CMYK or Lab at 32, 1-bit outside bitmap, indexed outside 8) and returns a discard report (channels, alpha and spot planes lost) that the host must confirm before executing. Read-only `indexed.colors` for the derived palettes is refused on write.
2. **Grayscale -> Bitmap** (new `src/mode/bitmap.rs`): 8- or 16-bit grayscale plane in, packed 1-bit plane out; `half_threshold` (`bitmap.threshold` percent), `pattern_dither` (ordered matrix), `diffusion_dither` (error diffusion, raster order, single-threaded so the result is deterministic), `halftone_screen` (frequency, angle, the six spot shapes), `custom_pattern` (caller tile; missing tile is typed `pattern_missing`). A resolution different from the source is refused typed (`bitmap_resample_not_in_prism`; resampling stays with pigment). Matrices, kernels and spot functions are Studio readings named in the module docs; Photoshop parity UNVERIFIED (D5).
3. **RGB -> Indexed** (new `src/mode/indexed.rs`): 8-bit RGB(A) in, index plane plus table (at most 256) out; `exact` (more than 256 distinct colours refused typed), `uniform`, `web` (216), `local_perceptual` / `local_selective` / `local_adaptive` (Studio readings over a bounded histogram; reuse `srgb_to_lab`/`delta_e`), `master_*` refused (needs several images), `mac_os`, `windows`, `previous` refused `palette_table_not_captured` (their entries are not in the spec); forced colours, `preserve_exact_colors`, transparency with the seven mattes, dither none / diffusion (`dither_amount`) / pattern / noise (explicit seed). Same input and options give the same bytes.
4. **Indexed colour table + file codec** (new `src/mode/color_table.rs`): ordered entries plus optional `transparency_index`; shipped `black_white` (2) and `grayscale` (256); `mac_os` and `windows` absent and reported as a spec gap; a hand-written colour-table file codec (768-byte RGB block, optional count and transparency trailer) taken from Adobe's public Photoshop file-format specification (confirm the layout there, else UNVERIFIED); hostile input (V4-01/02): exact sizes only, count at most 256, index below count, a `dims_u32_max` and an `aggregate_over_budget` test; encode then decode is identity.
- **Every slice:** typed refusals with stable `code()` before allocation; `deny(clippy::arithmetic_side_effects)`; no thread pool in a conversion; the source buffer is borrowed and never written (a failed conversion leaves it intact, MT-31175 step 3); terminal outcomes as codes through observe (prism `src/lib.rs:3` imports), never pixels or names. `DESCRIPTOR` (`src/lib.rs:652`) gains a `mode_conversion` key.

### Carried forward
- Duotone, multichannel and grayscale tonal mapping (MT-31175), the Folio document-mode record and the host's one-undo commit, bitmap resampling, master palettes, the `mac_os`/`windows` tables (spec gap), the Image > Mode menu (D7).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "via_render_provider_port", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-COL-106", "STU-COL-118", "STU-COL-119", "STU-COL-120", "STU-COL-121", "STU-SECTION-003", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045", "GUI-DESKTOP-025", "GUI-DESKTOP-028", "GUI-DESKTOP-032", "GPU-QUOTA-010", "STU-ARC-002", "STU-ARC-018"], "refactor_exposure": "low", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"gpu": "CPU conversions only; no device; via_render_provider_port is the MT default for Prism", "attribution": "conversions are pure functions returning new buffers; the mode change that mutates a document is the host's one undoable commit and carries the ActorContext there", "jobs": "a conversion is one bounded call; no queue or worker is added"}, "recorded_at": "2026-10-31T18:38:25Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ps-mode-convert-1`, branch `lane/smb-ps-mode-convert-1`, base 63ecf5a6)

- prism: new `src/mode.rs` and `src/mode/**`, `mod`/`pub use` lines and the `DESCRIPTOR` key in `src/lib.rs`, new `tests/mode_conversion.rs` (test fn names carry the MT recipe names `mt_31174`..`mt_31179`; record the mapping).
- No edit to existing prism modules beyond `pub(crate)` visibility of a helper, no `Cargo.toml` change, no root input, no new dependency.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over prism and its direct dependents by manifest (composite, develop, interop-fig, interop-layout, interop-psd, pigment, pulse, render-cpu, render-gpu; harness-core, harness-view and harness reach prism only through import features: include `-p hsk-studio-harness-core --features import-psd`, `-p hsk-studio-harness-view --features viewer`, `-p hsk-studio-harness --features import-psd`, 4e); nextest WHOLE prism suite (default unit; prism has no round feature unit); doctests if added (4d). The widest clippy set of this wave: expect the longest run. Name the log per RUN LOGS. Then `lane-check.sh --owner smb-ps-mode-convert-1 --per-sha origin/codex/handshake-studio-assets`; push only on `PER_SHA_RESULT OK`.
- Push order: prism's closure (accord, observe, moxcms, sha2) holds no other lane of this wave; any order with the other three. Prism sits in the closures of R50's pigment and interop-psd (SMB-FOLIO-CHANNEL-RECORD) and composite (SMB-AE-WATER-SIMS-1): push only when no R50 remediation run holding prism is in flight. Do NOT start the bundled run until the lead names the disk and confirms R50 judged SMB-R49-FOLLOWUPS-1 (accord, observe).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
