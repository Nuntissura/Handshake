---
file_id: BUILDER-BRIEF-LANE-SMB-PS-SOFT-PROOF-2
file_kind: builder_brief
updated_at: 2026-10-24T03:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-PS-SOFT-PROOF-2
lane_file: builders/lanes/hsk-studio-ps-soft-proof-2.json
crates: [hsk-studio-folio, hsk-studio-session (source.rs token only), hsk-studio-package (tests/externals.rs only), hsk-studio-harness-view, hsk-studio-harness (RUN-UI.md only), hsk-studio-prism (src/proof/gamut.rs doc lines only)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip 7a16f66d); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
depends_on: [SMB-PS-SOFT-PROOF-1 READY at 7a16f66d (prism proof API)]
---

<topic id="target" status="active" summary="App, rows, expected gain, model">

## Target

- App: Photoshop (proof provenance also Illustrator/InDesign, rows NOT_INSPECTED). Row PS-28 "Soft proofing and gamut warning". SMB-PS-SOFT-PROOF-1 built the engine (prism `src/proof/**`); its lane file lists the two gaps this lane closes: "document-scoped proof state (STU-COL-240): Folio record + rule 4b package suite" and "on-screen toggle, proof preset menu and gamut overlay in the window (harness-view)".
- Expected gain: PS-28 engine closes its last named part (document-scoped state), so it is BUILT_NOT_VALIDATED without the "engine-only" caveat (+0.7 pp Photoshop engine if SOFT-PROOF-1 is judged PARTIAL; 0 if already BUILT); strict UI row PS-28 becomes doable in the window (+1 of 36 = +2.8 pp Photoshop GUI strict). No pooled GUI workflow is proof-specific (audit gui-surfaces.json lists none), so the pooled GUI figure does not move.
- Model / effort: opus / high (new Folio record + rule 4b, a session token, GUI across shell/mod/canvas, and the Folio DESCRIPTOR text for two record sets).

</topic>

<topic id="paths" status="active" summary="Owned paths, shared hunks, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-ps-soft-proof-2`)

- Folio: NEW `crates/hsk-studio-folio/src/proof_state.rs` (record set `StudioProofRecords`, schema `hsk.studio.proof_records@1`, `validate_proof_records`, `parse_proof_records`, `proof_records_schema`), NEW `tests/proof_records.rs`; `src/lib.rs`: `mod proof_state;` / `pub use proof_state::*;` AND the DESCRIPTOR text, which also takes the multicam text SMB-PR-MULTICAM-1 left for the lead (lane file `notes.folio_descriptor_text_for_lead`, copy it verbatim). Lead decision recorded here: this lane owns every Folio hunk tonight; SMB-PR-MULTICAM-2 stays out of Folio.
- Session: `crates/hsk-studio-session/src/source.rs:150` add `"proof"` to `DOCUMENT_ENTRY_TOKENS` (+ its doc line). No other session file (SMB-PR-MULTICAM-2 owns session `lib.rs`, `session.rs`, `timeline.rs`, `multicam.rs` tonight). Read and write the entry from harness-view through the public `Session::document_entry` / `set_document_entry` (`session.rs:2173-2189`), validating with Folio on every set and read (pattern of `session/src/motion.rs:37-76`, implemented in harness-view).
- Package: `crates/hsk-studio-package/tests/externals.rs` one schema line `("proof_records", serde_json::to_value(folio::proof_records_schema()).unwrap()),` in `every_folio_reference_property_is_classified` plus the classifications it then demands (field names containing `profile`, `digest`, `source` ... match the VOCAB). If a proof profile reference needs a NEW external kind in `package/src/externals.rs`, stop and report (out of scope).
- Harness-view: NEW `src/view/proof.rs` (+ `src/view/proof/**`), NEW `tests/viewer_proof.rs`; additive hunks in `src/view/shell.rs` (`ShellCommand::Proof(ProofCommand)`, a VIEW > Proof submenu and a COLOR > Proof submenu built with `MenuNode::submenu`, availability "no open document"), `src/view/mod.rs` (`pub mod proof;`, one Viewer field, the canvas-texture hook for the proof preview and overlay, `state_json` key `proof`), and `src/view/render.rs` only if the composite RGBA read-back is not reachable from mod.rs (one additive accessor; record it).
- Harness: `crates/hsk-studio-harness/RUN-UI.md` one usage section (operator + model layer).
- Prism: `crates/hsk-studio-prism/src/proof/gamut.rs:17-18` insert the blank `//!` line before "The default warning colour is mid grey" (clippy `doc_lazy_continuation`, SOFT-PROOF-1 `notes.clippy_nits`). No other prism edit.

## Do NOT touch

- SMB-TIMELINE-TRIM-1 files: `src/view/timeline.rs`, `src/view/timeline_ui.rs`, `src/view/timeline/trim*.rs`, `tests/viewer_timeline_trim.rs` (and `tests/viewer_timeline.rs`).
- SMB-PR-MULTICAM-2 files: `crates/hsk-studio-pulse/**`, session `lib.rs`, `session.rs`, `timeline.rs`, `multicam.rs`, session tests other than ones you add for the token (prefer none).
- `src/view/hosted.rs`, `src/view/file_dialog.rs`, `src/view/timeline_export/**`, `hsk-studio-controls/**`, any other Folio file (`multicam.rs`, `timeline.rs`, `composition.rs` ...), root files.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-COL-240]`: "Soft-proofing simulates an output condition on screen WITHOUT converting document values. It is toggleable, its state is document-scoped and inspectable, and it never alters authority values ([STU-COL-164])."
- `[STU-COL-241]`: "A proof condition carries SIX fields: `proof_profile` ..., `proof_intent` ..., `display_intent` ..., `black_point_compensation` (bool), `simulate_paper_color` (bool) and `simulate_black_ink` (bool)."
- `[STU-COL-242]`: "A proof condition is nameable, savable and selectable. Studio ships at least: current CMYK working space; current RGB working space; legacy uncompensated RGB; and the accessibility conditions of [STU-COL-243]."
- `[STU-COL-244]`: "with a configurable warning colour and warning opacity ... MUST be queryable as structured state, at minimum as an out-of-gamut area fraction and a bounding region list"; table row "| `gamut_warning.opacity` | 0 | 100 | 0 | 100 | 100 | percent | 0 |".
- `[STU-COL-164]`: "Gamut warning, out-of-gamut markers, separation previews, ink limit views and proof previews are OVERLAYS computed from the proof branch. They MUST NOT be composited into the document, MUST NOT be present in an export unless the export explicitly requests a proof rendering, and MUST be individually queryable as inspection state so a headless model can read \"12.4 percent of pixels out of gamut\" rather than looking at a picture."
- `[STU-SHL-025]` VIEW (grep "`Proof` | submenu | `studio.color.proof`"): "- `Proof` | submenu | `studio.color.proof` → Proof Setup…, Proof Colours, Gamut Warning, Colour-Vision Proof…".
- `[STU-SHL-030]` COLOR (grep "Custom Proof Condition"): "- `Proof` | submenu → Proof Setup…, Custom Proof Condition…, Proof Colours, Gamut Warning, Colour-Vision Proof…, Save Proof Preset…".
- Command ids and bindings: the spec declares the submenu id `studio.color.proof` and the leaf labels only; `grep -n "cmd\.[a-z0-9._]*\(proof\|gamut\)"` returns 0 hits. Use leaf ids derived from the submenu id (`studio.color.proof.setup`, `.colours`, `.gamut_warning`, `.colour_vision`, `.custom_condition`, `.save_preset`; declared Studio reading) and NO default bindings (Photoshop's Ctrl+Y / Ctrl+Shift+Y are vendor bindings, UNVERIFIED, not declared by the spec).
- `[STU-COL-250]`: "Every operator-facing colour surface enumerated in this module MUST be reachable and drivable through the native operator UI and the typed model-steerable command surface as two projections of the same primitive".
- `[STU-SHL-008]` naming law (leaf labels are the spec's British spellings: "Proof Colours", "Colour-Vision Proof…").

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-ps-soft-proof-1.json`: capabilities (`ProofCondition`, `ProofConditionRecord::to_json`, `select_proof_condition`, `shipped_proof_conditions`, `prepare_soft_proof` / `SoftProof::run`, `SoftProof::gamut_warning` with `area_fraction`, `bounds`, regions capped at 256, `GamutWarningStyle`, `apply_overlay`, `in_gamut_substitute`), `notes.spec_deltas` (declared readings to keep: legacy RGB refused for non-RGB documents, CIE76 > 5 for table profiles, default warning colour (0.5, 0.5, 0.5), toggle-off = document -> display under display_intent with BPC on), `notes.gaps`.
- `builders/lanes/hsk-studio-pr-multicam-1.json` `notes.folio_descriptor_text_for_lead` and `notes.shared_files` (the externals.rs rows it added).
- Code: `crates/hsk-studio-prism/src/proof/**`, `src/lib.rs` (`pub use proof::...`); harness-view `src/view/shell.rs:511 enum ShellCommand`, `:579-596` entries, `:835-860` MenuNode titles/submenus; `src/view/render.rs:270` (canvas renders in `WORKING_PROFILE_LINEAR_SRGB`: the proof source profile, declared reading), `src/view/display.rs` (GPU shared-device path draws straight to a texture: the proof preview uses the CPU read-back path while Proof Colours is on; declare it); Folio `src/clip_finishing.rs` / `src/multicam.rs` (standalone record-set pattern), `src/vector_tools.rs` `StudioIccProfile` (SCPF id, `content_digest`) for an ICC proof profile reference; session `src/source.rs:150`, `src/motion.rs`; harness `tests/audio_waveform.rs:142` (document-entry edits appear as undo labels).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; push rules in risks)

1. Folio record set + session token: `StudioProofRecords { document_id, conditions: [StudioProofConditionRecord { name, proof_profile: shipped key | icc { profile id + content digest, declared reading } | colour_vision kind | operator matrix [9], proof_intent, display_intent, black_point_compensation, simulate_paper_color, simulate_black_ink }], active_condition: Option<name>, proof_colours: bool, gamut_warning: { enabled, colour [3], opacity 0..=100 default 100 } }`; names unique, bounds from the table, operator-matrix bounds equal to prism's (finite, |c| <= 4, row sums 0..=2), typed refusals, `Budget` nodes, absent record = byte-identical documents; `"proof"` token; externals schema line + classifications. Tests: `proof_records_validate_six_fields_and_bounds`, `gamut_opacity_follows_the_parameter_table`, `proof_records_absent_keep_documents_byte_identical`, package `every_folio_reference_property_is_classified` green.
2. Window commands and menus: VIEW > Proof (Proof Setup…, Proof Colours, Gamut Warning, Colour-Vision Proof…) and COLOR > Proof (+ Custom Proof Condition…, Save Proof Preset…), one implementation behind both; `Viewer::proof_command(ProofCommand)` typed surface; Proof Setup selects a shipped or saved condition (preset menu = `shipped_proof_conditions` + saved records); every change writes the `proof` document entry (validated) as one undo step; state survives save and reopen. Tests: `proof_menus_list_the_spec_leaves`, `accesskit_click_toggles_proof_colours`, `selecting_a_preset_writes_the_document_entry_and_undo_restores`, `custom_condition_and_save_preset_round_trip_through_save_and_reopen`, `proof_menu_is_unavailable_without_a_document`.
3. Canvas preview and overlay (STU-COL-164): with Proof Colours on, the displayed canvas is `SoftProof::run` over the composite read-back; Gamut Warning paints `apply_overlay` on top; neither touches document values or exports (Export PNG/JPEG stays unproofed). Tests: `proof_colours_change_the_displayed_texture_not_the_document`, `export_stays_unproofed_while_proof_is_on`, `gamut_overlay_marks_out_of_gamut_pixels_with_the_configured_colour`.
4. Inspection (STU-COL-240/244, Argus): `state_json()["proof"]` = enabled, active condition (six fields), gamut warning on/off, colour, opacity, `area_fraction`, region count, `regions_truncated`, bounds, receipt ids/hashes. Tests: `state_json_reports_area_fraction_and_regions`, `hostile_proof_entries_read_as_folio_diagnostics`.
5. Prism doc fix (gamut.rs:17-18), in the same bundled run.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Headless egui + AccessKit as in `tests/viewer_timeline.rs` (settle the layout before any pointer event, F-TL-2); assertions through `state_json()`, the session's `proof` entry and the rendered texture bytes compared with prism's own `SoftProof::run` output on the same buffer.
- NOT_PROVEN (`NOT_PROVEN:` lines): the GPU shared-device display path with proof on (desktop window); equality with Photoshop Proof Colors / Gamut Warning output and defaults (no D5 proof case).

## Bundled lane-check (rule 4a whole suites of every changed crate; rule 4b package)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-ps-soft-proof-2/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-ps-soft-proof-2 --disk d -- cargo nextest run -p hsk-studio-folio -p hsk-studio-session -p hsk-studio-package -p hsk-studio-prism -p hsk-studio-harness-view -p hsk-studio-harness --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: folio, session (token list), package, prism (doc lines), harness-view, harness (RUN-UI.md; its shell suites walk the menus). Rule 4b: new Folio record set -> package suite included. Clippy spans folio's and prism's many direct dependents: a long hold on D:. Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, conflicts, deadline">

## Edges and dependencies

- None: harness-view already depends on prism, folio and session; no crates.io dependency.

## UNVERIFIED (D5)

- Photoshop proof menu bindings, default proof condition, gamut-warning colour/opacity defaults and proof-preset file semantics.

## Risks and conflicts

- V4-14: SMB-PS-SOFT-PROOF-1 (prism) and SMB-PR-MULTICAM-1 (folio, session, package) are READY and unjudged. A push to prism, folio, session or package before the round that judges them freezes its candidate makes their claims stale. Keep slice commits local until the lead writes the frozen candidate sha into this lane file (or lists your commits in their claims); then rebase, run, push.
- Shared files with READY lanes, additive hunks only: Folio `lib.rs` (SMB-PS-FILL-LAYERS, SMB-PR-MULTICAM-1), package `tests/externals.rs` (SMB-AE-ROTO-WIRING, SMB-PS-FILL-LAYERS, SMB-PR-MULTICAM-1), `shell.rs`/`mod.rs` (SMB-HARNESS-SHELL-2, SMB-HARNESS-PANELS-1, SMB-TIMELINE-UI-1); a remediation in those files lands first, then rebase and re-run once.
- SMB-PR-MULTICAM-2 runs in parallel in session: you own only `source.rs`; if its session suite pins the token list, the second lane to push rebases and re-runs.
- The GPU display path bypasses the CPU buffer: proof preview on the read-back path is a declared limitation, not a silent fallback (status line names it).

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, subject to the V4-14 hold above and to a green bundled run per push; while either blocks, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
