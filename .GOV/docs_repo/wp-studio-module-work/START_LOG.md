---
file_id: WP-STUDIO-MODULE-WORK-START-LOG
file_kind: session_start_log_and_snapshot
updated_at: 2026-10-11T00:40:00Z
wp: WP-KERNEL-STUDIO
assignment: STUDIO-MODULE-BUILD
lead_role: KERNEL_BUILDER (lead/coordinator only; writes no product code)
---

# WP Studio Module Work — Start Log and Snapshot

Shared workspace for the Studio module team: research, diagnostics, workflow review, builder briefs, logs.
Prose surface (context, not authority). Authority stays in Codex, KB protocol, packet/MT JSON and STUDIO_MODULES.json.

<topic id="operator-instructions" status="active" summary="Operator instructions 2026-10-11, preserved">

## Operator instructions (2026-10-11, live chat, preserved)

Message 1:
- Work on WP-KERNEL-STUDIO. MT-35536 changes approved (changed approach + additional fix round).
- Goal: create the crates and module asset that is Studio for Handshake. Do NOT wire it into Handshake yet.
- Studio is a standalone module asset: update/expand/patch it outside Handshake, then merge back into Handshake / update; use Studio in other apps; use sub-modules and features of Studio in other apps.
- Studio module = standalone, multiple crates; develop without interrupting Handshake; ensure a modular system.
- Reuse cargo, warm cargo; a cache tool may have been installed (see `cargo-tooling` topic: sccache NOT found). Bundle checks and tests.
- Do not create massive evals, tests and checks. Focus on minimal implementation; focus on what is blocking or failing at its root causal lane.
- Team allowed: cheap reader / online researcher agents; a team of Opus 5.5 diagnostics; a workflow agent that is adversarial about how tasks are performed.
- Create a sub folder in gov kernel docs where all agents work, log, research, document: this folder (`wp studio module work` -> `wp-studio-module-work`, spaces replaced per naming policy).
- Studio is a module asset; push product code to https://github.com/Nuntissura/assets in a sub folder (existing: `handshake-studio-assets/` on branch `codex/handshake-studio-assets`).
- Adobe suite is installed on this computer; "Craft" Rust rewrites of Adobe apps should also be present. Craft is a case study, far from perfect. Prefer rewriting concepts and behaviour from Adobe in Rust over relying on Craft.
- Use cheap agents (Sonnet 5.5; Haiku 5.5 for very low level/risk tasks).
- Observed: people testing Craft apps — promising, missing many features, and PSD images display differently than in Adobe.
- File formats: Handshake native file format likely has more capability. Open questions: compression, color space, codecs, RAW file handling. As much as possible in Rust, especially core/engine level.
- If unclear: agents research online; do not guess. Do not treat the WIP WP-KERNEL-012 codebase or its version of the master spec as a basis (it is being refactored on another device); it may be explored only as reference to resolve problems.
- Create a snapshot and start log with: repo rules, WP Studio state of repo and work, operator additional instructions, cargo rules, current workflow (which may and should change if the workflow agent sees repeated failure).
- Tick: lead checks team and progress every 15 min; workflow agent has an offset tick.
- Resolve problems by online research, deliberation, code exploration of Adobe suite, Craft products, or WP-KERNEL-012 (reference only).
- Allowed: expand scope, create extra MTs, download tools that help lint/check code/test runs/diagnose.
- Work autonomously; rely on the team; do not guess; refresh agents when useful to keep context fresh (lead context drifts first; hence workflow agent and diagnostic team).
- If something cannot be deleted, move it to the recycle bin.
- Time is the most valuable resource. Strategise before expensive cargo builds. Do not get stuck on multiple/massive tests, reviews, checks; aggressive progression. Heuristic philosophy (not mandate): build it, break it, research and diagnose, fix it.

Message 2:
- The lead does NOT implement code. Create a team that can work in parallel for code work.

</topic>

<topic id="snapshot" status="active" summary="Repo and work state at session start">

## Snapshot (2026-10-11T00:37Z)

| Item | Value |
|---|---|
| Kernel | `wt-gov-kernel` on `gov_kernel`, HEAD `6e9e19f4` == `origin/gov_kernel` |
| Kernel dirty (not ours, untouched) | `.GOV/docs_repo/tmp/studio-modular-authoring-gameplan.yaml` (M), `.GOV/docs_repo/tmp/github-sync-gameplan.yaml` (??) |
| Product worktree | `../wt-studio-assets` (repo `Nuntissura/assets`), branch `codex/handshake-studio-assets`, HEAD `55c1096` == origin, 47 commits |
| Workspace | `wt-studio-assets/handshake-studio-assets/` (Cargo workspace, edition 2024, toolchain 1.97.1, resolver 3) |
| Crates present (8/35) | accord 1.4k LOC, observe 3.1k, folio 2.0k, chronicle 2.6k, prism 2.5k, pigment 3.4k, nib 4.0k, type 61.8k (vendored rustybuzz subset) |
| Crates declared, absent (27) | layout, develop, components, pulse, score, reel, motion, composite, render-cpu, render-gpu, package, interop-psd, interop-vector, interop-fig, interop-layout, interop-media, controls, session, harness, kernel-bindings, press, web, score-device, score-plugin, reel-native, interop-lrcat, motion-js |
| Module map | `.GOV/task_packets/WP-KERNEL-STUDIO/STUDIO_MODULES.json` (35 modules + host modules, allowed dependency edges) |
| Packet | `.GOV/task_packets/WP-KERNEL-STUDIO/packet.json` lifecycle `STUB`, execution_authority `NON_EXECUTION_STUB`; Operator 2026-10-11 authorizes implementation (this log) |
| MT corpus | 6222 MT JSON: 6195 PENDING (blocked_on SPEC207_PUBLICATION...), 8 PASS (V1..V7), 1 BLOCKED (MT-35536) |
| MT-35536 (Type) | FAIL_V3, BLOCKED fix budget exhausted; operator_decision recorded 2026-10-11: changed approach + one fix round APPROVED |
| Spec | `SPEC_CURRENT` -> v02.215 (`.GOV/spec/master-spec-v02.215/`) |
| Artifact root | `Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/` (verified exists) |
| Host | 32 logical CPUs, 128 GiB RAM; D: 3.1 TiB free; C: 638 GiB free |
| Adobe | `C:/Program Files/Adobe/`: Photoshop, Illustrator, InDesign, After Effects, Premiere Pro, Media Encoder, Character Animator, Lightroom CC + Classic, Acrobat DC, Dreamweaver 2021 |
| Craft apps | NOT found under Program Files / LocalAppData Programs (research agent to locate) |

</topic>

<topic id="repo-rules" status="active" summary="Binding rules digest for every team member">

## Repo rules digest (authority: Codex v1.5 file `.GOV/codex/Handshake_Codex_v1.4.md`, `.GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md`)

- CX-212C/212F: governance lives in kernel (`gov_kernel`); product code only in `wt-studio-assets/handshake-studio-assets/**`. Never commit `.GOV` into the assets repo.
- CX-211: product code never reads/writes `.GOV`.
- CX-GIT-001: no agent/test/tool creates a worktree or branch.
- CX-GIT-002: every commit names explicit paths; no force push; push only to existing `codex/handshake-studio-assets`.
- CX-GIT-003: tests clean everything they create outside the artifact root.
- CX-107/108/SAFE-001: no destructive ops on unowned targets; never stop processes this session did not start.
- CX-SAFE-002: background, non-interactive; no installers with prompts, no foreground windows.
- CX-503R: SurrealDB/EventLedger exclusive DB authority; no SQLite/Postgres in Studio (lrcat = bounded byte decoder only).
- CX-503S: no Docker/outside apps as prerequisites.
- CX-008-VIS: native Rust; no Electron/webview (webview only in-app browser).
- CX-CAP-001: one active owner per capability/mechanic; consume the lowest owner.
- CX-UNIT-001 + STUDIO_MODULES `allowed` edges: crate dependency direction is law; no forbidden edges.
- CX-MODULE-LIFECYCLE-001/002: module lands standalone first in assets repo; host adoption is a separate later step (not now).
- CX-PROOF-002/005: builders submit READY_FOR_VALIDATION; never self-certify PASS; independent validator issues verdicts.
- CX-EXEC-003/013/015: two failed attempts on same blocker -> changed approach on evidence or escalate; no identical reruns.
- CX-EXEC-007: commit + push as soon as changed code compiles.
- CX-EXEC-009: anything > 2 min runs in background; poll <= 60 s.
- CX-984-*: all build/test output under the artifact root; owner subfolders; no repo-local `target/`.
- CX-109/109A: relocatable paths; names use hyphens/underscores, no spaces.
- HSK naming: use only names from STUDIO_MODULES/spec/code; no invented product names.
- No speculation: unverified claims labelled UNVERIFIED; research online instead of guessing.
- Privacy (2026-10-11 incident: one crates.io API request by the Type lane carried the Operator's email in its User-Agent): never put the Operator's email, name or any personal data into requests, headers, URLs or payloads. For APIs that require a User-Agent use `hsk-studio-research/0.1 (no-contact)`; prefer WebFetch/docs pages over raw API calls.

</topic>

<topic id="cargo-tooling" status="active" summary="Cargo rules for the team">

## Cargo rules (team standard)

- OPERATOR 2026-10-11 (disk rule): C: is NOT used for any Studio cargo build, test or check (C: compile is reserved for WP-KERNEL-012 and is being withdrawn). Tools MAY be installed on C: (e.g. `~/.cargo/bin`), but their compilation also runs on D:. All cargo builds/tests/checks run under `Handshake_Artifacts/WP-KERNEL-STUDIO/<owner or MT>/`, with TMP/TEMP there too.
- Env: `cd "<dir>" && source "<kernel>/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo ...` sets CARGO_TARGET_DIR, TMP/TEMP/TMPDIR (D:, owner `STUDIO_OWNER`, default `studio-team-dev`) and line-tables-only debug.
- Workspace root: `wt-studio-assets/handshake-studio-assets` (toolchain pinned 1.97.1 by `rust-toolchain.toml`).
- Shared warm target (one owner dir, Cargo's own build-dir lock serializes concurrent builders = one build at a time on D:):
  `CARGO_TARGET_DIR=<Handshake Worktrees>/Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/target`
- `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `CARGO_PROFILE_TEST_DEBUG=line-tables-only` (disk cap; KB-CARGO-IO-001).
- Builders: `cargo check -p <own-crate> --tests`, `cargo clippy -p <own-crate> --tests`, focused `cargo test -p <own-crate>` (operator: build it, break it, fix it). Never `--workspace` tests from builders; never link unrelated binaries.
- Gameplans: dev loop `.GOV/task_packets/WP-KERNEL-STUDIO/gameplans/STUDIO-MODULE-BUILD.yaml` (machine checks only: disk >= 50 GiB, toolchain pin); validator rounds `gameplans/STUDIO-MODULE-ROUND.yaml` (build-round preset minus clean-tree, plus export-identity and per-test-timeout confirms).
- Pointers: `wt-studio-assets/.gameplan-active` and `Handshake_Artifacts/WP-KERNEL-STUDIO/.gameplan-active` -> BUILD; `Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/.gameplan-active` -> ROUND.
- The hook ignores inline `GAMEPLAN_ROLE=` prefixes (it reads its own process env), so role scoping is not used. Always prefix cargo with `cd "<dir>" &&` so the hook finds a Studio pointer. From the kernel root the hook resolves WP-KERNEL-012's gameplan, whose confirm binding is broken (other device; do not touch).
- Gate patterns match anywhere in a command's text, including quoted strings and heredocs (e.g. the per-test runner's name). Use the Edit/Write tools, not shell, to write text containing those words.
- No outer `timeout` wrappers (hook blocks; CX-EXEC-005).
- Validator union rounds: one build + one test invocation over all ready crates, on an export of the pushed SHA refreshed in place at `Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/` (GLOBAL-VALIDATION-023..025).
- sccache and cargo-nextest: NOT installed at session start (not on PATH, not in `~/.cargo/bin`). A first install attempt built in cargo's temp dir on C: (operator correction: not allowed); it was stopped and its two C: temp dirs (only `.rustc_info.json`) removed. Reinstall from source on D: was stopped (sccache pulls aws-lc-sys C build; blocked the critical path). INSTALLED 2026-10-11T01:15Z from official GitHub release binaries (no build): sccache 0.18.0 (mozilla/sccache, zip sha256 8965c74d…363d verified) and the per-test runner 0.9.148 (nextest-rs, zip sha256 7a931b4a…c782 verified), copied to `~/.cargo/bin` (tools on C: allowed). sccache use: validator rounds only (`RUSTC_WRAPPER=sccache`, `SCCACHE_DIR` under `Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/sccache`, `CARGO_INCREMENTAL=0`); builders keep incremental without sccache. Already installed: cargo-deny, clippy, rustfmt, cargo-tarpaulin, cargo-miri, rust-analyzer, just (retired for governance). Tool installs allowed by operator if they cut time (record here).
- Only the workspace steward edits root `Cargo.toml` (members, workspace metadata/registry). Builders edit only their crate dir; dependency needs go to `builders/dep-requests.jsonl`.

</topic>

<topic id="workflow-v1" status="active" version="1" summary="Current workflow; workflow agent may revise">

## Workflow v1

Roles:
- Lead (kernel builder): coordinates, ticks every 15 min, owns governance records, writes no product code.
- Workspace steward (Opus): baseline check of existing workspace, scaffold all declared crates as members, owns root `Cargo.toml`/registry metadata, merges dep requests.
- Type lane (Opus): MT-35536 approved changed approach + one fix round.
- Research team (Sonnet/Haiku): formats/PSD/native, color/codecs/RAW, Craft case study, Adobe behaviour, spec-to-brief readers, existing API digest.
- Builders (Sonnet, parallel, one crate each, disjoint files): implement minimal vertical slice from brief; commit+push per compile; mark lane READY_FOR_VALIDATION in ledger.
- Diagnostics (Opus): on any builder failure twice, or red round: root cause, fix direction.
- Validator (Opus, independent, fresh context): union test rounds at batch boundaries; verdicts in ledger.
- Workflow agent (Opus, adversarial): offset tick; challenges process; proposes workflow vN+1 in `workflow/`.

Loop: brief -> build minimal slice -> check/clippy/focused test -> commit+push -> ledger READY -> validator round -> PASS or FAIL+diagnosis -> fix.

State: `ledger.json` (lane status per CX-STATUS-001 values), `journal/tick-log.jsonl` (lead ticks), `team.json` (roster).

</topic>

<topic id="prep-corpus" status="active" summary="Existing Studio prep research: read before any new research">

## Existing prep corpus (Operator 2026-10-11: most prep work is already done — reuse, research only gaps)

- `.GOV/reference/studio_app_feature_research/`: feature maps (01 Photoshop, 02 Affinity, 03 InDesign, 20 Illustrator, 21 Figma), leaf indexes (04, 06, 07, 22, 23), feature-use cards (15-17, 24, 25), 05 studio primitive map, 08 gap resolution notes, 10 studio command contracts, 11 + 26 provider posture maps, 12 + 27 parity matrices, 13 layer-graph vertical slice, 19 local-first Rust posture, `_installed_export_scripts/` (ExtendScript exporters for Photoshop/Illustrator/InDesign = oracle starting point), `_source_snapshots/`, `repass-2026-10-09/` (engineering, photocraft/filmcraft/light-effect/vector-design root drills = Craft case studies, argus-diagnostics, handshake root drills).
- `.GOV/docs_repo/studio-collaboration-2026-10-09/`: adobe-original-team (photo/design/media original behaviour, cross-suite control semantics, parity red team), modular-authoring team preparations (visual, visual-components, visual-companion, media, web, shared, embedding, partition, whiteboard gap, autonomous research), backend red team (plugin platform, expression provider, PDF/audio drills, backpressure), spec-mt team (asset-first MT refactor plan, spec/MT source evidence).
- `Handshake_Artifacts/WP-KERNEL-STUDIO/autonomous-research-team/` (concurrency, interchange, presentation), `studio-repass-*/source`.
- Files are large (up to 30 MB): grep or json selectors with bounded output only.

</topic>

<topic id="decisions" status="active" summary="Lead decisions with reasons">

## Lead decisions

NOTE (correction 2026-10-11T01:50:42Z): decision timestamps 02:15Z-04:15Z below were lead estimates ahead of wall-clock; the actual times are ~00:55Z-01:50Z. Git commit times are authoritative.

- 2026-10-11T01:00Z — Operator asked whether to decompile the Adobe suite (at least the engines) to rewrite in Rust. Lead decision: NO decompilation/disassembly of Adobe binaries. Reasons: Adobe terms prohibit reverse engineering; EU Software Directive 2009/24/EC art. 6 allows decompilation only for interoperability of an independent program and not to develop a substantially similar program (lead reading of public text, not legal advice); derived code would taint the reusable GitHub asset; decompiled optimized C++ is slower to use than public specs. Instead: public specs/SDK docs (PSD/PSB, ISO 32000 blend modes, DNG, XMP, IDML, Photoshop/AE/UXP/ExtendScript object models) + black-box oracle testing as a licensed user (art. 5(3): observe, study, test) — controlled inputs rendered/exported by installed Adobe apps, compared to Studio output. Adobe apps open foreground windows -> oracle export sessions need Operator approval (scripted batch or one-time manual export). Research agents draft the oracle plan.
- 2026-10-11T00:55Z — Lead session restart stopped all agents after ~8 min; resumed all 10 from saved transcripts with the D:-only cargo env.
- 2026-10-11T01:40Z — `tools/workspace-probe.rs` (validator dependency gate; Cargo ignores it) bans wgpu/egui/`-sys` for every owner, contradicting the approved module map (render-gpu = wgpu, controls = egui, reel-native/score-device/score-plugin/motion-js = isolated native providers). Decision: steward changes the probe to allow these only for their designated owner crates (forbidden elsewhere), with honest pin kinds (`native-ffi`). Builders add deps directly in their crate manifest so they compile immediately and log a dep request; steward mechanically batches registry pins + probe approvals before each validator round.
- 2026-10-11T01:53:20Z — Round entrypoint = `tools/run-round.sh` (the hook gates scripts with that name) + `.config/nextest.toml`; export mode refreshes source mtimes because a shared target dir otherwise reuses artifacts built from other trees (observed: 10 crates 'fresh' on a never-built export). OPERATOR ITEM D8: empty folder `C:\x` was created by the steward's refusal test (violates CX-GIT-003 and the no-C: rule); the safety check blocked removal; the lead does not route around that block — Operator please delete it by hand.
- 2026-10-11T04:15Z — motion-js engine: follow prep decision `studio-collaboration-2026-10-09/backend-redteam/expression-provider-adoption.json` (rquickjs 0.14.0 / QuickJS-NG 0.16.2, conditional, behind feature `quickjs`, required controls EXPR-PROVIDER-*); Boa only with proven equivalent controls; worker isolation/RSS later lane. Folio appearance lane SMB-FOLIO-ENG-2 (opacity, fill, blend discriminant, clipped, isolation, mask ref). Importer-local PSD kinds stay namespaced until spec defines canonical kinds.
- 2026-10-11T03:55Z — Render architecture (diagnostics/render-plan-ir.md, pigment 4c3ac91): pigment owns the render-plan IR + RenderBackend port + the single per-pixel blend math (StudioBlendMode 35, Exactness classes); composite lowers to it, render-cpu/render-gpu implement the port, interop-psd maps keys to it. Numerics: f32, out-of-range refused (OutOfDomain); HDR/f64 later only inside pigment::blend_math. Spec gaps recorded: STU-CMP-020c enum owner wording, STU-CMP-020 38 members; motion cannot reach pigment.
- 2026-10-11T02:55Z — From spec-reader-a: (a) render-plan IR has no shared home (no composite<->render edge) -> Opus architect lane SMB-PIGMENT-ENG-1 decides owner (candidate pigment) and specifies IR + backend port; (b) controls/render-gpu pin host-lock versions egui =0.33.3, wgpu =27.0.1 (embedding compatibility); (c) no PSD/IDML/INDD/AI/FIG fixtures on the machine: Adobe/Figma parity NOT_PROVEN until fixtures (Operator) or public corpora; (d) web ships a dependency-free span-preserving HTML subset parser; full parser (html5ever/lol_html) = later decision; (e) spec gaps logged in briefs: input_ordinal direction, STU-DS-117 override location, controls fine_step vs precision, Folio frame/thread type and web mode field.
- 2026-10-11T02:50Z — Timeline ownership: Folio owns sequence/track/clip document types (STU-VID-010; one StudioDocument graph) with time fields as `hsk_studio_accord::Ticks` (accord already owns the tick unit; folio may depend only on accord). Pulse reuses accord::Ticks and owns rates/timecode/conversions/sequence-time operations (no own tick type, no document types). interop-media maps into Folio types. Lane SMB-FOLIO-ENG-1.
- 2026-10-11T03:30:19Z — AV1 (reel feature `av1`, rav1d 1.1.0 via C ABI, unsafe confined to src/av1/ffi.rs): accepted as OFF-by-default and TRUSTED-INPUT-ONLY; 9.7% of single-byte corruptions abort the process (uncatchable unwrap in rav1d). Untrusted AV1 requires worker isolation in hsk-studio-reel-native (lane SMB-REEL-NATIVE-AV1-WORKER) before use. License policy extended: CC0-1.0 (public-domain dedication, e.g. to_method) counts as permissive; Operator may override. Rejected: rav1d-safe (AGPL-3.0-only/commercial), dav1d crate (needs libdav1d C).
- 2026-10-11T02:15Z — From `research/formats-psd-native.md`:
  - interop-psd: OWN parser written from the public Adobe PSD/PSB spec (Operator: rely on rewriting Adobe concepts, not on Craft; PhotoCraft clean-room claim disputed). photocraft-psd/ag-psd-rs may be read as behaviour references only; no code copied. Rejected deps: `psd` 0.3.5, zune-psd, rawpsd.
  - package: in-house STORE-only ZIP64 writer + bounded reader (crc32fast) over `zip` 9.0.1 (C zstd via defaults, no stated limits); ruzstd 0.9.0 decode (encoder beyond Fastest UNVERIFIED), lz4_flex optional, C zstd only behind a feature; SHA-256 identity (existing pin). Atomic save = temp + sync_all + rename with Windows PermissionDenied backoff, own implementation; real interrupted-save test on Windows required.
  - PSD fidelity priorities: exact composite semantics (pass-through, clipping, masks, fill vs opacity, Blend-If, knockout); Photoshop blend functions in gamma-encoded document space, straight alpha; cached pixels for text/smart objects as display truth + compositor-rendered effects; ICC/CMYK/Lab display path + `hasRealMergedData` (res 1057) check; byte-stable unknown blocks with typed loss receipts; never default an unknown blend mode to Normal.
  - Oracle tiers: Tier 0 = merged composite of Photoshop-saved PSDs (public corpora: psd-tools fixtures MIT; photocraft-corpus license to verify) downloaded as test DATA under `Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/` (Operator download permission for check/test aids); Tier 1 = ~50 Photoshop-authored gap files + TIFF references -> Operator-approved foreground session (D5). Comparer lives in a steward-owned tool crate (interop-psd may not depend on composite).
- 2026-10-11T01:50Z — From `research/color-codecs-raw.md`:
  - BPC: Prism owns black-point compensation (moxcms 0.9.1 exposes no BPC option; STU-COL-136 needs an independent flag) -> extra lane SMB-PRISM-ENG-1 (implement published BPC on top of moxcms transforms).
  - License policy (lead default, Operator may override): default build closures permissive only (MIT/Apache-2.0/BSD/Zlib/ISC); copyleft (LGPL/GPL/AGPL: rawler, rawloader, imagepipe, dng, jpegxl-rs, lensfun, heic) only inside optional isolated providers and only with Operator approval. Reason: Studio is a reusable asset for other apps.
  - develop: start with an own DNG/TIFF-EP reader (public DNG spec) + kamadak-exif; proprietary RAW via the LightCraft raw leaf is deferred (provenance of Craft's "clean-room" claim disputed in `research/craft-case-study.md`).
  - Still-image codecs (png/jpeg/webp/tiff/exr/jxl/avif): no owner in STUDIO_MODULES. Interim: hsk-studio-reel adapters (closest responsibility "decode/encode media"); final owner = Operator decision D6 (module-map change).
  - OCIO (vendor `ocio` 0.1.0 vs own subset): deferred; not needed for first slices.
  - ICC oracle (LittleCMS golden generator outside the workspace vs STU-COL-140): question routed to engine-color researcher.
- 2026-10-11T01:40Z — Craft (ArtCraft "Crafting Apps", github org storytold; PhotoCraft etc.) is NOT installed locally; source snapshots exist at `gov_runtime/tools/studio-craft-source-cache/`. PSD display gap verified in PhotoCraft source: re-composites layered PSDs with fitted/approximate math and ignores the stored merged composite unless flattened; macOS-only display profile reader. Lessons in `research/craft-case-study.md`.

</topic>

<topic id="workflow-v2-adoption" status="active" version="2" summary="Lead adoption of workflow-review-001">

## Workflow v2 adoption (2026-10-11T01:30Z; source `workflow/workflow-review-001.md`, `workflow/workflow.json`)

Adopted:
- Builders run their brief's focused tests (`--no-fail-fast`) before READY_FOR_VALIDATION; a resubmission after FAIL must first pass the validator's exact failing command. Basis: Operator instruction "build it, break it, research and diagnose, fix it" + "bundle checks and tests" (CX-021 Operator authority over CX-EXEC-014 implementer-only-compile procedure). Independent validator verdicts unchanged (CX-PROOF-002).
- Scaffold gate: no builder starts before the steward's 27-crate scaffold is pushed and `cargo metadata` succeeds. Waves by dependency: wave 1 = crates needing only the existing 8; later waves when upstream lanes have a pushed compiling commit. Every lane ends each turn compiling.
- Shared files steward-only: builders stage only `crates/hsk-studio-<own>/**`; never `Cargo.lock`/root `Cargo.toml`; steward commits a lock sync before each round; lock-only failure = infrastructure.
- Per-lane state file `builders/lanes/<crate>.json` written at every commit and before long jobs (with PID); `ledger.json` = lead index; resume + lead-restart prompts in DISPATCH_TEMPLATES.
- Continuous rounds with one reusable round script; re-check a PASS lane only when its relevant inputs changed (CX-VAL-002); every engine brief carries >= 1 oracle value the builder cannot edit (e.g. PSD Maximize-Compatibility merged composite as a no-Adobe-launch oracle).
- Paperwork cap: at most one kernel commit per lead tick; no new gameplan step without an observed failure.
- One-crate-one-agent rule (incident 2026-10-11T02:34:44Z: two agents in crates/hsk-studio-folio; a pathspec commit swept the other lane's uncommitted files and broke origin briefly): never assign any path inside a crate to a second agent while another agent owns it; cross-crate migrations wait for the owner or go through the owner.
- Agent refresh rule (Operator: refresh agents to keep context fresh): when a builder has used >~400k tokens and its next lane is in a different crate, end it and spawn a fresh agent from DISPATCH_TEMPLATES + lane file + brief; continuing slices in the same crate may reuse the agent. Session cap: 20 concurrent subagents; queue lanes in ledger.dispatch_queue when full.
- Gameplan retry-guard interpretation (2026-10-11T02:44:53Z): a PRODUCT test failure fixed by a code commit (HEAD change clears the guard) needs no new gameplan step; gameplan steps are added only for preparation/infrastructure failures or wasted runs (GLOBAL-GAMEPLAN-003 applied to preparation scope per CX-GP-001 'gameplan holds preparation checks only').
- Retry-guard discipline (2026-10-11T03:42:40Z): two agents cleared retry-guard blocks by changing command flags/filters (interop-vector-migrate --no-fail-fast; render-chain narrower per-target commands). That is a bypass in spirit (GLOBAL-GAMEPLAN-004). Rule: product failure -> fixing commit; infrastructure cause -> one cause-naming gameplan step; never argument tweaks. Added to DISPATCH_TEMPLATES and broadcast to active agents.
- Gameplan/git discipline (2026-10-11T03:12:07Z): retired shared dev-gameplan steps GP-020..GP-025 (product code-pattern checks for session, nib, interop-psd fixtures) because product compile/test mistakes belong in lane notes + fixing commits, not the shared preparation gameplan (brittle file-pattern gates block all lanes; old 66-step anti-pattern). Incident: a `git commit --amend` in the shared worktree amended another lane's local commit (restored to pushed d575678; nothing force-pushed). Rule: no --amend/rebase/bare commit; pathspec commits only. Broadcast to all active code agents.
- Shared gameplan hygiene (2026-10-11T02:52:29Z): a lane-specific gameplan step (e.g. GP-018 develop digest placeholder) gates EVERY builder's test runs; keep such steps self-clearing machine checks and retire them as soon as the lane fixes the cause; prefer lane-local guards in the lane's own tests.
- Shared-tree rule (observed 03:45Z: pigment mid-edit E0583 blocked dependents): every agent keeps its crate compiling at every save; new module files first, `mod` wiring last in one edit.
- Workflow tick stays +7 min (11,26,41,56); after two quiet ticks may drop to 30 min (lead decides per tick).

Adapted:
- Model choice: builders stay Sonnet (Operator: cheap agents for code); an engine lane escalates to an Opus builder at its first NEEDS_NEW_APPROACH or second FAIL; Opus diagnostics on any repeated failure.
- One build per disk (CX-984-002) KEPT without waiver: validator rounds use the SAME shared target dir as builders (`studio-team-dev/target`), so Cargo's build-dir lock serializes them (different source path = separate fingerprints; disk cost accepted).

Workflow v3 amendments (2026-10-11T02:12:42Z, source workflow/workflow-review-ticks.md tick 1; adopted):
- Rounds freeze ONLY on steward sync commits (steward derives Cargo.lock + registry pins + probe list from an export of HEAD's committed manifests, commits via temp index with compare-and-swap `update-ref`), lead names that SHA in the round dispatch; at most one round per 20 min while lanes wait. Round-time lock derivation rejected (CX-VAL-007/009).
- Round entrypoint step 0: fail-fast locked metadata + registry sync check (~1 s) before any build (CX-EXEC-015(4)).
- Lane-file fixed key set + `date -u` timestamps only (12/17 lane files had future stamps); pathspec-only commits for builders.
- Adversarial review findings are routed to builders immediately (do not wait for the round to open).

- Workflow v4 (): ten rules V4-01..10 adopted from workflow/workflow-review-ticks.md tick-2 (R5 failure classes: hostile sizes, silent green, frozen duplicates, fault hooks, same-class re-FAIL, doc claims, hostile text, inline free text, rate-limit resilience); text lives in builders/DISPATCH_TEMPLATES.md topic v4-rules; mechanical parts (NOT_PROVEN accounting, machine-local path grep, duplicate fingerprints) go to the round entrypoint step 0 (steward).

- Lead acknowledgements 2026-10-11T05:47:08Z (prism ENG-6): (1) validated ENG-3 test changed to follow ICC.1:2022 8.10.2 intent fallback (spec-backed; ENG-3 is re-judged next round under CX-VAL-009); (2) Lab-indexed output CLUT interpolation is now trilinear to match lcms2 (ICC leaves CLUT interpolation implementation-defined) - PROVISIONAL, added to the D5 Photoshop oracle list; (3) re-pinned wide determinism digests and new WIDE_ENGINE_* receipt names accepted (prism moved to deterministic f64 evaluation; moxcms parse-only). D13 resolved in code: AdmittedWideEngine (e7cf81f) charges the caller ledger (STU-CON-044); develop adoption routed. Perf follow-up stub SMB-PRISM-ENG-7 (fast path with proven error bound).

Open Operator decisions (not adopted; recorded for the Operator):
- D1: explicit confirmation that builder focused tests override CX-EXEC-014 for this WP (lead adopted on the basis of Operator instructions; Operator may reverse).
- D2: waive CX-984-002 one-build-per-disk for D: (would let validator rounds and builders build in parallel on separate targets).
- D3: change the global gameplan hook to match gated words only in command position, not inside quoted text/heredocs (global tooling; Operator-only).
- D4: prep tools `ae_render.py`, `ae_scripting.py`, `adobe-install-harvest.py` (prep corpus `_tools/`) extract embedded JSON/string tables from Adobe DLLs (string extraction, not disassembly). Lead: NOT run this session pending Operator policy.
- D9 (provisional lead decision; BLOCKED: the permission system denied the Cargo.toml edit; awaiting Operator) 2026-10-11T02:14:33Z): add dependency edge hsk-studio-interop-media -> hsk-studio-pulse (no cycle) so timeline interchange uses Pulse's timecode instead of a duplicate (CX-CAP-001). Root Cargo `allowed` updated by steward; STUDIO_MODULES.json NOT edited (drift recorded, like pigment->folio). Operator may reverse (then interop-media keeps raw fields only).
- D10 (download approval for TEST DATA, not executed): three CC0 DNG samples from raw.pixls.us (license verified via its JSON listing: publicdomain/zero/1.0) into `Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/dng/`: Canon EOS 5D Mark III id 2204 1.47 MB lossless JPEG sha256 1d77dcc6...b603e; Samsung SM-G973U id 3839 1.26 MB uncompressed + OpcodeList2 sha256 6492b7e0...84ab1; Nikon D80 id 1988 7.90 MB tiled LJPEG + linearization sha256 5095ca26...26bb. Fetch script `crates/hsk-studio-develop/tests/real_dng/fetch-corpus.sh --go` verifies hashes. Not downloaded: the Operator granted downloads of tools, not explicitly test data.
- D7 evidence (2026-10-11T02:28:57Z, diagnostics/type-lane.md topic harfrust-spike): harfrust =0.14 behind feature `harfrust`: 11/11 exact parity with the pinned reference (made by upstream rustybuzz 0.20.1); shapes Devanagari + Hebrew that the vendored subset refuses (unverified correctness, no reference); metering only at call boundary (pre-charge, cancel before/after, internal allocations unseen); needs own font copy (CJK peak 10.4 -> 18.7 MB). Decision for Operator: replace vendored subset with harfrust (coarser metering) or keep both.
- D11: desktop GUI host for the standalone harness: eframe (and image) are forbidden for every crate by the workspace probe; a desktop window needs eframe or winit+egui-wgpu. Options: allow eframe only for hsk-studio-harness, or winit + egui_wgpu via render-gpu's wgpu. Harness slice 1 is headless until decided.
- D9 disclosure (2026-10-11T02:41:51Z): the steward added the interop-media->pulse root metadata edge in sync 9e867f5 before the lead's 'do not add' notice arrived; reverted in sync 404ff3a; interop-media never used pulse. D9 remains with the Operator.
- D12: Lightroom catalog oracle for hsk-studio-interop-lrcat: a Lightroom Classic-written .lrcat (a small throwaway catalog the Operator creates, or approval to create one in an Operator-approved Lightroom session) to verify table/column names, schema versions and process-version allow-list; until then all real catalogs report schema_version_unrecognised.
- D13 (spec enrichment, CX-105 Operator-only): STU-CON-044 (v02.215 l.22588) prescribes admitted Prism transform in <=64-pixel chunks with accounted lifetime; develop now uses Prism WideEngine (large chunks, plan cache, not charged to the caller admission ledger). Either enrich the spec or add a Prism admission-ledger hook (lane SMB-PRISM-ENG-6 can carry the hook).
- D14 (packaging): motion-js's `motion-js-worker` executable (quickjs feature) must ship next to the host executable or be passed via `with_worker_program`; decide distribution layout for the standalone module + Handshake embedding. Same question will apply to any worker-isolated native decoder (e.g. reel AV1 if moved to a worker).
- D15: three prism vs lcms2 divergences need a Photoshop (ACE) T3 reading to choose Studio's normative behaviour: v2 display wtpt treated as D50 in absolute colorimetric; forced BPC for v4 perceptual/saturation; CMYK perceptual BPC black. Folded into the D5 oracle session.
- D16 (2026-10-11T05:13:39Z, CX-CAP-001 consolidation): three bounded ZIP readers exist (interop-layout for IDML, interop-fig over miniz_oxide, package's strict STORE-only canonical container). Options: (a) package exposes a non-canonical bounded ZIP reader and interop-layout/interop-fig get a package edge (module-map change), (b) new leaf crate owning bounded ZIP read (module-map change), (c) keep per-format readers and record a waiver. Lead recommends (a): package already owns ZIP64 + bounded parsing and its tests. Scope also covers the duplicated CRC-32 table (interop-vector gzip.rs vs interop-layout). Not executed: module-map edges are Operator-owned (see D9).
- D17 (2026-10-11T05:13:39Z): Figma-authored `.fig` fixtures (any small files exported by the Operator from Figma desktop, local copies only) to confirm interop-fig UNVERIFIED items (chunk numbering, u32 length prefix, Kiwi field names Message/nodeChanges/guid/type, int64 skip rule) and record real version numbers for the admit list; until then interop-fig admits no version.
- D18 (2026-10-11T05:31:12Z, CX-CAP-001 + packaging; R5 review group B): worker-process supervision (spawn CREATE_NO_WINDOW, env_clear, deadline/kill, heartbeat, crash classification) is duplicated between motion-js and reel-native, and score-plugin needs the same. Options: (a) new leaf crate `hsk-studio-worker` owning supervision, consumed by motion-js, reel-native, score-plugin (module-map change), (b) fold into an existing foundation crate (changes its responsibility), (c) waiver. Lead recommends (a); decide together with D14 whether worker executables get their own packages. Until then no third copy (score-plugin keeps supervision in one extractable module).
- Lead decisions 2026-10-11T05:31:12Z: render-gpu non-separable modes tolerance 2e-6 (pigment's own f32 reference deviates 1.6e-6 from f64 at clip edges; random/hand cases < 5e-7); D13 resolved toward option (b): Prism admission-ledger hook (queued on prism ENG-6) instead of spec enrichment; SMB-MOTION-JS-WORKER undelivered scope (AST validator, OS RSS/CPU cap) split into stub lane SMB-MOTION-JS-SANDBOX-2.
- D5: black-box oracle sessions (Photoshop/Illustrator/InDesign/AE render-reference export scripts per `research/adobe-behavior.md`) open foreground windows and the Photoshop gamma-blend check flips a global Color Settings value (restore after) -> needs an Operator-approved session.

</topic>

<topic id="ticks" status="active" summary="Tick schedule">

## Ticks

- Lead: cron `4,19,34,49 * * * *` (every 15 min): read ledger/logs/agent status, steer, refresh stale agents, append `journal/tick-log.jsonl`.
- Workflow agent: offset cron `11,26,41,56 * * * *` (+7 min): adversarial review of last interval; may request a different offset.
- Session crons expire after 7 days.

</topic>
