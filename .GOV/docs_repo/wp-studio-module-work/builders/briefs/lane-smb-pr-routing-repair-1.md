---
file_id: LANE-SMB-PR-ROUTING-REPAIR-1
file_kind: lane_brief
updated_at: 2026-11-07T05:20:57Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-ROUTING-REPAIR-1
lane_file: builders/lanes/hsk-studio-pr-routing-repair-1.json
crates: folio, session, pulse, harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A removed bus no longer makes every sequence's routing unreadable, and a new sequence no longer silently takes a removed sequence's routing">

## Target

STUDIO-R57 findings F-R57-2 and F-R57-3 (MEDIUM). Read them in full in `S/validation/round-R57.json` key `findings_not_failures`, with `direction.after_r57_20261106.lead_rulings` in `S/ledger.json`, SD-64 and SD-67 in `S/spec-enrichment/STUDIO-ENRICHMENT-20261030-HOST-INTEGRATION.json`, and your predecessor SMB-PR-ENTRY-REFERENCES-1's lane file. The lead's rulings below are overrulable readings, and two of the lead's rulings have failed this week: treat them as claims to break, and tell the lead if the code says otherwise.

1. **Bus, red first.** A bus removed while a live route plays into it (output or send) commits with no refusal through a timeline, a multicam and a records step. Then `audio_routing_records` (`session/src/sequence_records.rs:267`) and the listing answer `document_rejected` for every sequence, other sequences included. Lead ruling: an output or send target naming an absent bus is inert like the rest of the rule, the set stays readable, the route plays to the master, a missing send is skipped, and the listing names them. Do not refuse at commit: harness-core commits the sequence before its entry (`harness-core/src/harness.rs:1777-1783`), so a refusal blocks a step that removes a bus and repoints the routing together. Prove it against the rendered mix, not only the listing.
2. **`angle_order`: send options first.** It stays Unreadable after a timeline or records step (pinned in the entry references table). It is derived and read by index (`folio/src/multicam.rs:25`, `:35`, `:81`, `:140`), so a wrong repair moves an angle to another track. There is no ruling. Read it, then send the lead the options with what each does to a stored multicam, before any edit of this cell.
3. **Takeover, red first.** Remove seq-a, insert a brand-new sequence with the same id: the commit is clean, and the removed routing's master volume applies (R57 measured rms ratio 0.100 against a default routing). Master effects and master automation are the same field. Lead ruling: the commit reports a typed takeover when a step that is not an undo gives a new sequence an id with a stored routing. An undo of the removal stays live again by id, which is right. Viewer ids are first-unused counters, not uuids, so id minting must skip ids an inert entry names.
4. **Failure-class sweep (V4-06).** Every Viewer id mint that can land on an inert entry: at least `view/timeline.rs:1034`, `view/source_monitor.rs:306`, `view/timeline/nest.rs:55`, `view/layers.rs:51`, `view/mixer.rs:164`. List each site, fixed or not applicable with a reason. Every other entry whose stale id can make a whole set unreadable: list it.
5. **MTs.** Claim a row only if you read it and this proves it. Do not edit MT files.

Hostile input (V4-01/02): a set with many stale routes answers in bounded time; no new unbounded allocation.

Out of scope: F-R57-4, any drawing or panel, no crate `Cargo.toml` change, no root input.

PUSH HOLD: STUDIO-R59 is judging c4d06841 on C:. Build on D: only. Push nothing until the lead tells you R59 is recorded; when you are ready to push, message the lead with the local tip and your test counts. One lane pushes at a time.

Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005). The folder name and `--owner` are the old lane's; your branch, log names and records are this lane's.

No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D6, D7, D8), unless this brief names it. No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal session frame per operation, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-entry-references-1`, branch `lane/smb-pr-routing-repair-1`, base c4d06841)

`crates/hsk-studio-folio/src/multicam.rs` and routing records, `crates/hsk-studio-session/src/sequence_records.rs` and entry code, `crates/hsk-studio-pulse` where the mix reads a route, `crates/hsk-studio-harness-view/src/view` id mint functions only, their tests. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-entry-references-1 --disk d`): step 0; folio, session, pulse and harness-view suites; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-entry-references-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
