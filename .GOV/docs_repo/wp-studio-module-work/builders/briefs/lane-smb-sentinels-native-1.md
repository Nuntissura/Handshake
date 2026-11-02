---
file_id: LANE-SMB-SENTINELS-NATIVE-1
file_kind: lane_brief
updated_at: 2026-11-02T02:34:03Z
wp: WP-KERNEL-STUDIO
lane: SMB-SENTINELS-NATIVE-1
lane_file: builders/lanes/hsk-studio-sentinels-native-1.json
crates: [hsk-studio-type (public paragraph-terminator helper), hsk-studio-layout (use it, drop the duplicate), hsk-studio-interop-layout (host composition through native Type; IDML sentinel mappings)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R53 F-R53-3 and map the stream sentinels in IDML">

## Target

Read `validation/round-R53.json` F-R53-3 and the lane files of SMB-TYPE-SENTINELS-REGISTRY-1 and SMB-LAYOUT-SENTINELS-1 first. Spec: Master Spec v02.229 STU-TYP-207 with the v02.228 SD-29 sentinel assignment and its import / export mappings (:9106-9160); research basis `research/sd26-30-research-20261101.json` item SD-29. interop-layout already depends on layout, type and observe.

1. **One paragraph-terminator rule.** type has `validate::ends_paragraph` (`pub(crate)`, :290) and layout has its own `sentinels::ends_paragraph` (:61). Expose type's rule as a public `hsk_studio_type::ends_paragraph` (additive; no behaviour change in type) and make layout call it, removing layout's duplicate. Tests: both crates agree for every code point the rule names.
2. **Composition through native Type (F-R53-3).** `compose_with_sentinels` is proven only with a closure shaper. Add a host test in interop-layout that composes a multi-page, multi-section story through the real hsk-studio-type engine (use type's test fixture fonts through the corpus config, never machine paths, V4-03) with every marker member, every break member and flush space, and asserts: Type accepts layout's paragraph partition (no InvalidBidi), markers resolve per page, breaks reach their targets, the stored story bytes are unchanged.
3. **IDML mappings of sentinels.** Map, as the spec states: markers to and from IDML (ACE processing instructions with PageNumberType, as the spec / research record says), breaks to and from `Br` plus `ParagraphBreakType`, flush space, and foreign noncharacters on import (replacement per the spec, SD-37 reading U+FFFD if still a gap). Round-trip tests on synthetic IDML snippets; typed losses where IDML cannot carry a member. If the spec does not state a mapping for a member, leave it typed unavailable and record it (no invented mapping).
4. **MTs.** Claim only what is met: MT-31005, MT-31007, MT-31008, MT-31009 rows this proves.

Vendor parity UNVERIFIED (D5).

Out of scope: OOXML / ODF mappings (later), GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: composition and import / export keep their existing context", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-sentinels-native-1`, branch `lane/smb-sentinels-native-1`, base 92ae2232)

- type: `src/lib.rs` (the public helper export), `src/validate.rs` (visibility only), tests.
- layout: `src/sentinels.rs` (use the type helper), tests.
- interop-layout: the IDML story import / export modules for sentinels, a new host-composition test file.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of the changed crates and their direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-sentinels-native-1`; C: is reserved for STUDIO-R53): step 0; clippy over type, layout, interop-layout and their direct dependents; nextest WHOLE suites of type (both round units), layout and interop-layout; doctests if added.
- Push only after STUDIO-R53 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
