---
file_id: LANE-SMB-TYPE-SENTINELS-REGISTRY-1
file_kind: lane_brief
updated_at: 2026-11-01T05:38:43Z
wp: WP-KERNEL-STUDIO
lane: SMB-TYPE-SENTINELS-REGISTRY-1
lane_file: builders/lanes/hsk-studio-type-sentinels-registry-1.json
crates: [hsk-studio-type (stream sentinels per SD-29, 245-tag feature registry per SD-30)]
model: sonnet
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Assign the v02.228 stream sentinels and grow the OpenType feature registry to the 245-tag table">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` (current). STU-TYP-170 at :8544 with the v02.228 (SD-30) 245-tag registry table under it; STU-TYP-207 at :9106 with the v02.228 (SD-29) stream-sentinel assignment. Research basis: `research/sd26-30-research-20261101.json` items SD-29 and SD-30 (field evidence, Unicode facts). The type crate at base carries SMB-TYPE-OT-FEATURES-1 (`src/catalog.rs` `StreamForm::Pending` for markers, breaks and flush space; the feature registry with 209 tags and an `#[ignore]` NOT_PROVEN 229-tag test).

1. **Stream sentinels (SD-29).** Replace `StreamForm::Pending` with the assigned forms exactly as the spec states: the fourteen members without a Unicode character on U+FDD0..U+FDDD in the spec's order, the anchored-object marker on U+FFFC, U+FDDE..U+FDEF reserved (inserting or importing a reserved code point is a typed refusal or replacement exactly as the spec says). Break sentinels terminate their paragraph wherever this crate partitions paragraphs. Tests: one per member (insert yields the assigned code point), paragraph termination for each break, NFC/NFD stability of every sentinel (noncharacters and U+FFFC survive normalisation; flush space is not U+2001), reserved-range refusal, and foreign noncharacters on import handled as the spec says.
2. **Feature registry (SD-30).** Grow the registry to the 245 rows of the table (the 243 OpenType 1.9.1 registered tags with cv01..cv99 and ss01..ss20 counted individually, plus the two observed-only tags), with the table's status per tag (registered / deprecated / observed_unregistered) and its in-observed-union flag. Remove the `#[ignore]` from the 229-tag test and make it prove the 229-tag observed union against the table; add a test that the registered set is exactly 243 and the deprecated tags carry their status.
3. **MTs.** Read each MT JSON under `.GOV/task_packets/WP-KERNEL-STUDIO/` and claim only what is met, one assertion per concrete acceptance row: MT-31005 (now carries the sentinel rows), MT-31007, MT-31008, MT-31009, MT-30955 (now carries the 245-tag rows), MT-30959.

Existing tests change only where SD-29/30 change a pinned value (for example a test asserting `Pending` or 209 tags); list each with old -> new and the clause. Vendor parity UNVERIFIED (D5).

Out of scope: layout paragraph partition and composition of markers, IDML/OOXML/ODF import-export mapping (a later layout / interop-layout lane); no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: insertion keeps its existing command context; no new mutating command", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-type-sentinels-registry-1`, branch `lane/smb-type-sentinels-registry-1`, base a7be22a1)

- type: `src/catalog.rs`, the feature-registry module(s) the OT-features lane added, `lib.rs` exports and DESCRIPTOR text, their tests and new test files.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of type and its direct dependents (RED COMMITS).
- One bundled run on D: (`lane-check.sh --owner smb-type-sentinels-registry-1`; C: holds validation round STUDIO-R51): step 0; clippy over type and its direct dependents; nextest WHOLE suite of type in every round feature unit (4a, 4e: the harfrust unit too); doctests if added (4d).
- Push only after STUDIO-R51 is recorded (SMB-TYPE-OT-FEATURES-1 is being judged on this crate) and the lead releases you: rebase, `lane-check.sh --owner smb-type-sentinels-registry-1 --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
