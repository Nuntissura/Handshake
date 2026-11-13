---
file_id: LANE-SMB-MOTION-IMPLIED-VALUES-1
file_kind: lane_brief
updated_at: 2026-11-13T17:08:44Z
wp: WP-KERNEL-STUDIO
lane: SMB-MOTION-IMPLIED-VALUES-1
lane_file: builders/lanes/hsk-studio-motion-implied-values-1.json
crates: harness-core, composite
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Enable animation keys the evaluated value of an absent anchor or position, and no key-creating path seeds an absent stream from the caller's key (STU-MOT-038, STU-MOT-020b)">

## Target

Absent motion streams take their implied value, never the caller's key: enable animation and toggle-on of an absent anchor or position key the evaluated value, and point_of_interest, text animator params and track apply seed nothing from a key (harness-core motion_commands.rs; composite track_apply.rs with approval)

Why this lane: ledger PRIORITY candidate 2026-11-13T15:35:24Z, verbatim: "PRIORITY: enable_animation and toggle-on of an absent anchor or position: evaluate the implied value through the composite evaluator and key it, so new layers can enable anchor and position animation (STU-MOT-038; review aa4a35e3 F2, accepted limit in SMB-FIX-ANIMATION-VALUES-1)". Its three siblings from the same lane's V4-06 sweep (builders/lanes/hsk-studio-fix-animation-values-1.json notes.v4_06 and next_lane_candidates), verbatim: "camera and light point_of_interest: keyframe.set or paste on an absent point_of_interest creates it from the caller's value, and disable leaves a two-node camera or light (STU-MOT-020b; sibling of SMB-FIX-ANIMATION-VALUES-1)"; "text animator params: keyframe.set or paste on a param absent from the animator creates the stream from the caller's value (STU-MOT-020b; sibling of SMB-FIX-ANIMATION-VALUES-1)"; "composite track_apply put(): a missing position, rotation or scale stream gets the first key's value as its static value, so disabling afterwards keeps it (STU-MOT-020b; sibling of SMB-FIX-ANIMATION-VALUES-1)".

Facts are READ by the wave-next-2 planner at origin 89f27330 (read-only, not run; `git diff a77c4ee4 89f27330` touches only harness-view src/view/menu_tree.rs). Treat them as claims and check them.
- Spec: STU-MOT-038 (module 14 :27695, enable animation captures the current evaluated value) and STU-MOT-020b (:27569, disabling every keyframe returns exactly the prior static value).
- `harness-core/src/motion_commands.rs:60-66` module docs: enable_animation, add_at_current and toggle_at_current refuse `property_default_unknown` for an absent anchor or position. `:2141` current_slot calls slot(e, id, prop, None); `:2090-2107` implied_property, implied_stream, implied_return.
- `:1771-1774` member(v, default, hint, prop): no default and no hint is property_default_unknown, a hint seeds the member; camera `:1881` and light `:1892` point_of_interest call member(.., None, hint, ..).
- `:1999-2020` text_slot(e, id, rest, hint, prop): an absent animator param is created from the hint, else property_default_unknown (`:2020`).
- composite `src/track_apply.rs:198-206` put(): a missing stream gets `keys.first()` as static_value (static_implied false).
- harness-core has no hsk-studio-composite dependency (its Cargo.toml path deps: accord, folio, session, controls, observe, web, package, interop-psd, develop, interop-tiff, press, interop-raster, interop-affinity, prism); harness and harness-view take composite only under their feature `viewer` (harness Cargo.toml `viewer = [.., "dep:hsk-studio-composite", ..]`; harness-view likewise), and the motion tests of the previous lane and the R60 probe run hsk-studio-harness with default features (no composite). Evaluating through composite needs a seam the lead approves: a port the host fills under [viewer] or [media], or a steward-owned harness-core -> composite edge.
- Tests of the previous lane: harness/tests/fix_animation_values.rs, motion_commands.rs, motion_commands_2.rs, motion_records_session.rs; its accepted limit (notes.cut) is this lane's target.

Red test (must fail first):
`harness/tests/motion_implied_values.rs` (new): (a) property.enable_animation on a new layer's absent anchor and position keys the composite-evaluated value at the current time (today property_default_unknown), disable returns the implied static, and toggle on, off, on at one time is one key each with undo restoring; (b) keyframe.set on an absent camera and light point_of_interest, then disable: the node returns to its prior state, not a two-node camera or light with the caller's value as static; (c) keyframe.set and paste on a text animator param absent from the animator, then disable: the static is the param's implied value, not the caller's key; (d) only if the lead approves the composite edit: apply_track on a layer with no position, rotation or scale stream, then disable: the static is the layer's prior value, not the first track key. They fail at the base.

Fix contract:
- FIRST: send the lead the seam shape before coding (where the implied value is evaluated, which of (a) to (d) it covers, the asks: new edge or port, composite file approval). Report it as a blocker message and wait.
- Each command stays one undo step named by its catalog id (common ruling 10) with the caller's ActorContext; static_implied semantics of SMB-FIX-ANIMATION-VALUES-1 are kept.
- No folio record change without asking the lead; no new crate edge without the steward and lead.
- 2 to 5 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-pr-sequence-session-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-harness-core` (lib and its own tests) and `cargo nextest run -p hsk-studio-harness --test motion_implied_values --test fix_animation_values --test motion_commands --test motion_commands_2 --test motion_records_session`; `-p hsk-studio-composite --features motion` only if composite is touched. Bundled: the harness-core suite, composite if touched, and those harness binaries in one lane-check.
MENU COVERAGE: if a catalog row changes (action/commands/motion.rs), run native-host [document] gui_menu MENU-INV-1 at landing or add menu_tree entries, and say which; otherwise say none changed.

Out of scope: F-R60-42 (set_parent compensation), F-R60-24 (composition.remove_layer leaves the adjustment's document layer), expressions, keyframe selection UI, harness-core harness.rs (SMB-FIX-LAYER-RECORD-PRUNE-1 may add a Run hook there), harness-view view/mod.rs and view/commands.rs (that lane's), any Cargo.lock change without asking.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PROTOTYPE-PLAY-CANCEL-1, 2 SMB-PR-TIMELINE-FIXES-1, 3 SMB-MOTION-IMPLIED-VALUES-1, 4 SMB-GUI-GESTURE-GATE-1; the lead may interleave SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 3 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): STUDIO-R60 attempt 4 runs on the D: round slot and holds the D: lock (class round, FIFO priority) for many hours. Your lock-needing steps queue behind it: that is expected. Start the lock-needing step in the background and wait to be notified. Never work around the lock, never poll in a loop, and never run cargo outside a hold. Read code and edit while you wait. ONE HOLD PER PHASE (wave-next-2, 2026-11-13): the four lanes share the D: lock with each other and with the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Lanes land one at a time on the lead's release.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: each motion command's existing terminal frame, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-sequence-session-1`, branch `lane/smb-motion-implied-values-1`, base 89f27330)

hsk-studio-harness-core/src/motion_commands.rs; hsk-studio-harness-core/src/action/commands/motion.rs (rows only, lead approval); hsk-studio-composite/src/track_apply.rs (lead approval); the seam's host side in hsk-studio-harness/src (lead approval); hsk-studio-harness/tests/motion_implied_values.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-sequence-session-1 --disk d`): step 0; harness-core suite; composite suite [motion] if touched; harness --test motion_implied_values, fix_animation_values, motion_commands, motion_commands_2, motion_records_session; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-sequence-session-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
