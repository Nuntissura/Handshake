---
file_id: WP-STUDIO-MODULE-WORK-HANDOFF
file_kind: handoff
updated_at: 2026-11-07T05:14:28Z
wp: WP-KERNEL-STUDIO
authority: Handshake Codex, KERNEL_BUILDER_PROTOCOL, START_LOG.md, ledger.json, validation/*, builders/briefs/wave-parity-2-common.md, builders/studio-env.sh (this file is a projection; they win on conflict)
previous_version: handoff-archive/HANDOFF-20261101.md
---

<topic id="takeover-prompt" status="active" summary="Prompt for the new master dev leader (paste into a new session opened in wt-gov-kernel)">

## Takeover prompt

Paste this into a new session opened in `wt-gov-kernel`.

```text
You are the new MASTER DEV LEADER (KERNEL_BUILDER role, coordinator) for WP-KERNEL-STUDIO. Studio Kit is a Rust rewrite of the Adobe suite, Figma and Affinity, in repo Nuntissura/assets, branch codex/handshake-studio-assets, subfolder handshake-studio-assets. It is standalone and is not integrated into Handshake.

You coordinate. You write no product code. You work through fresh, bounded sub-agents: builders, validators, stewards and read-only auditors. Use the Agent tool with an explicit model and effort.

READ FIRST, in this order. The repo's authority wins over your preferences and over this prompt:
1. Kernel root CLAUDE.md / AGENTS.md.
2. .GOV/codex/Handshake_Codex_v1.4.md (CX-VAL-001..011, CX-VERIFY, CX-MODULE-LIFECYCLE).
3. .GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md.
4. .GOV/docs_repo/wp-studio-module-work/HANDOFF.md, every topic. It holds the truth check of 2026-11-05, the Vulkan GUI pivot, current state and work order.
5. In the same folder: ledger.json key direction (workflow_20261107 first), builders/briefs/wave-parity-2-common.md, builders/DISPATCH_TEMPLATES.md.
Verify every claim against files, git and command output. Label what you cannot verify UNVERIFIED. Do not write agent-local memory.

STATE: read the state topic and ledger.json; this prompt holds no fixed state.

FIRST STEPS:
1. Confirm which cargo, rustc or nextest processes are running and who started them; product origin; lane statuses in ledger.json.
2. Report to the Operator in plain words where things stand, then put open decisions to them one at a time (workflow topic).
3. Follow the workflow topic, then the next-work topic.

DO:
- Follow the Operator's instructions directly and fully. Do not hand instructions back or ask them to re-confirm a decision. Operator decides scope, pace and stopping.
- Keep lane state in ledger.json, one line per event in START_LOG.md, and update HANDOFF.md before any pause.
- Guard every push: SHA=$(git rev-parse HEAD) && [ -n "$SHA" ] && git push origin "$SHA:refs/heads/<branch>". Commit with git -c core.fscache=false, path-limited. End commit messages with the Co-Authored-By line your session specifies.
- Report your own and your agents' mistakes plainly. Check what agents report before you repeat it: they state mistakes as confidently as facts.
- Follow the workflow topic: width, depth and a working Vulkan GUI first, then fix what is broken; many lanes at once; review before expensive builds; minimal testing; reuse before building; close what you open; no stray cargo.

DO NOT:
- No new egui, eframe, winit, wgpu, naga or WGSL line unless `direction.blocker_decisions_20261107` in ledger.json says otherwise (D1, D7, D8). Studio GUI work restarted on the Operator's word of 2026-11-07, in pure Vulkan.
- Do not delete anything (files, branches, worktrees, build targets) without explicit Operator approval in the same turn; deletions only through a guarded script. The one standing approval: a completed lane's worktree and local lane branch, through tools/lead/lane_close.sh (Codex CX-GIT-001A). Never delete or force-push a remote branch. No git reset --hard or git clean without approval.
- Do not ask the Operator to close Chrome; they use it.
- Do not stop processes your session did not start.
- Do not create, confirm or edit gameplan steps for Studio, reinstall the gameplan hook, or edit the Operator's global config.
- Do not touch WP-KERNEL-012 files. Do not integrate Studio into Handshake.
- No database other than SurrealDB. Permissive licences only; never read GPL, LGPL or MPL source. Never commit the proprietary Arial host font. Never treat the cargo registry cache as removable.
- Do not invent workflows, rules or governance documents while the repo's authority covers the need.
- Never touch refund-claim-20261021.md.
```

</topic>

<topic id="what-studio-is" status="active" summary="What Handshake Studio is, from the spec and the WP contract">

## What Studio is

**Source:** Master Spec v02.229 `spec-modules/14-studio-creative-suite.md` (33,657 lines; the section is product law), `task_packets/WP-KERNEL-STUDIO/packet.json`.

Studio is the Handshake creative module that unifies raster imaging, vector illustration, page layout and publishing, typography, colour, effects, design systems, prototyping and motion, and whiteboarding into one operator application and one model-steerable command surface, in native Rust. The WP scope also preserves RAW and catalog, clips and video timeline, composite, web authoring, audio, and import and export.

**Parity target named by the spec and packet:** Adobe Photoshop with Camera Raw, Illustrator, InDesign, the Affinity suite, the Figma family (Design, Draw, FigJam, Slides, Sites, Buzz, Make, Motion, Dev Mode); the module also targets Lightroom Classic, Premiere, After Effects and Audition. Vendor AI and generative features are out of scope. No vendor runtime or cloud is required.

**Governing ideas:**
- One `StudioDocument`, one canonical owner per shared mechanic, not one app per vendor.
- STU-CON-007: every tool or command must be model-invokable, parallel-safe, deterministic and visually verifiable. STU-MDL-006: nothing is operator-only by default. Parallel model agents are first-class users.
- Asset first (STU-OVR-001): Studio is built and proven as an independently buildable modular asset, then separately embedded in Handshake.
- Authority at integration: Handshake's SurrealDB and EventLedger. No SQLite or PostgreSQL. The one exception is a bounded read-only `.lrcat` byte decoder.
- Host direction (Operator 2026-10-30): native Vulkan GUI, connections to internal_diagnostics, Flight Recorder and the out-of-process watcher Palmistry, Handshake as memory and GPU manager of every module.

**What exists physically:** 56 crates, about 931,000 lines of Rust, 4,722 `#[test]` functions (mechanical count at 1940292c, vendored code included). Families: contracts (`accord`, `observe`), document (`folio`, `chronicle`, `session`, `package`), engines (`pigment` raster, `nib` vector, `type`, `layout`, `prism` colour, `develop` RAW, `components`, `pulse` timeline, `score` audio, `reel` media, `motion`, `composite`), renderers (`render-cpu`, `render-gpu`), interop (`interop-*`, `press-*`, `ps-*`, `web-*`), test host and bindings (`controls`, `harness`, `harness-core`, `harness-view`, `kernel-bindings`, `worker`).

</topic>

<topic id="this-wp" status="active" summary="What WP-KERNEL-STUDIO is doing and how the work runs">

## What this WP is doing

**Contract:** "All original and approved added acceptance independently proven on exact final unchanged candidate" (`packet.json` `outcome.completion_statement`). Scope: `handshake-studio-assets/**`. 6,220 microtasks derived from the spec, 21,426 acceptance rows.

**Phase now:** engine lanes under the HOST INTEGRATION rules. GUI lanes are paused for the Vulkan pivot. Engine work has moved through video timeline, layout, type and session in the last rounds.

**How work runs:**
1. The lead scopes a lane: brief, lane file, ledger row, and a reused or new worktree in `wt-studio-lanes/` (workflow topic).
2. A builder writes red tests first, then the fix, runs one bundled check, and pushes after a per-commit compile check.
3. A separate validator round judges every READY claim at a frozen sha, adversarially. Rounds are full or scoped (CX-VAL-011 decides; R54 was full, R55 and R56 scoped, so R57 must be full).
4. Findings become lanes. Decisions go to the Operator or the spec owner as SD items.

**Latest rounds:** R54 @96ace3ca (2026-11-02, FULL), R55 @ce644577 (one FAIL, remediated), R56 @091c4d1b (2026-11-05, scoped, 326 PASS, 0 FAIL). R56's union took 2 h 23 min on C:.

</topic>

<topic id="truth-check" status="active" summary="Health and progress truth check, 2026-11-05, at product origin 1940292c">

## Truth check 2026-11-05

**Method:** eight read-only sub-agents at 1940292c; no cargo was run. The lead re-checked the main claims against git and code. Full evidence, with file:line, is in `research/truth-check-20261105-*.json`: `hbr-usage`, `hbr-host`, `vulkan-pivot`, `code-health`, `mt-raster-effects-motion`, `mt-vector-type-layout-interop`, `mt-video-audio-design-web`, `mt-shell-manual-foundation`, `mt-prefilter`.

**Bottom line:** a large, honest, well-gated engine codebase, far from the parity its size suggests. It proves mostly that Studio agrees with Studio. The product has no real GUI under the current decisions, no process-level way for a model to operate it, and no wired diagnostics. There is no defensible single percentage: samples are judgmental and domains overlap. Do not quote one.

### Health

| Area | Truth | Evidence |
|---|---|---|
| Tests | Last FULL round R54: 4,545 distinct tests passed, 0 failed. R56 scoped: 5,578 rows passed, 0 failed. 0 failed rows and 0 timeouts in every union R46..R56 | `round-R54.json`, `round-R56.json` |
| No verdict | 31 commits, +6,600 lines since R56, including 4 READY lanes | `git diff 091c4d1b 1940292c` (lead checked) |
| Not on the product branch | 23 commits, +9,753 lines on 5 lane branches, including the only fix for HIGH F-R56-1. On GitHub backup branches since 2026-11-05, after this check; no tests run on them since | `git ls-remote origin 'refs/heads/lane/*'` (lead checked) |
| Proof surface | Debug profile, one Windows machine, one RTX 3090. No CI (lead checked), no clippy gate, no fuzzing of parsers | `code-health` |
| Vendor parity | 27 of 308 audited rows (8.8%) touch real vendor output, 7 (2.3%) match fully; row audit dated 2026-10-21, corpus unchanged since. One oracle session, 2026-10-16. Premiere, Audition, Affinity and Figma have no oracle. 302 code lines say UNVERIFIED (D5) | `code-health` q1 |
| Crash risk | F-R56-1 (stack overflow from document data) is still on origin. Same-class candidate `interop-fig-native export.rs:384`, UNVERIFIED. 0 `todo!`, 0 `unimplemented!`, 7 `panic!` (test, const or worker only) | `code-health` q2 |
| Silent wrong output | `frame_blending` and `optical_flow` show a floored frame (F-R56-2); EDL drops a hold (F-R56-10); removing a sequence invalidates all routing (F-R56-3). Fixes for F-R56-2 and F-R56-3 are not on the product branch (backup branches only) | `round-R56.json` |
| Licences and graph | Clean: 549 crates, no GPL, LGPL or MPL obligation, no database linked, no forbidden edge broken | `code-health` q4, q5 |
| Spec fidelity | 22 items SD-31..52 PROPOSED, and D7A-01..17; shipped behaviour rests on those lead readings | enrichment files |
| Records lag code | `STUDIO_MODULES.json`: all 35 modules read PLANNED_NOT_IMPLEMENTED, 21 of 56 crates not in `modules`, and it still calls render-gpu a "wgpu backend" and controls an "egui" mechanic (lead checked). `_MT_INDEX.json` says 2257 shell-client MTs; the MT files say 2266 (lead recounted) | direct read |
| Publication | Repo default branch is `codex/handpick`, not Studio's branch (lead checked); they diverge by 504 and 516 commits. Its catalog points to a stale Studio copy (45 crates) with 3 commits of its own; 0 tags | `hbr-host` |

**Strengths to keep:** rounds exit non-zero rather than hide NOT_PROVEN; validators probe adversarially; every HIGH and MEDIUM closure opened in the audit held; the D1 and D7 quarantines held.

### Against the build rules

Build rules: `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json`, 61 rules. Plus the Operator's global requirements for a built-in model manual, debug suite, parallel paths and quiet operation.

| Rule set | Rows | MET | PARTIAL | MISSING | Not yet | Not inspected |
|---|---|---:|---:|---:|---:|---:|
| INT, VIS, MAN, QUIET, SWARM + global | 60 | 4 | 37 | 13 | 6 | 0 |
| PRIV, COMPART, COMPONENT, HOST, MODULE-LIFECYCLE, STOP | 33 | 2 | 18 | 10 | 1 | 2 |

`round-R56.json` never mentions HBR-INT, VIS, MAN, QUIET, SWARM or Argus, so rounds do not gate these.

**Top gaps, in order:**
1. **No process-level way to drive or read the product.** Every window action is a Rust `Viewer` method (`harness/RUN-UI.md:375`); `state_json` has no CLI (lead checked); no IPC. CLI and binary use one fixed actor (`harness/src/cli.rs:1029`). This breaks the parallel-model requirement at its root.
2. **Diagnostics not wired.** Shipped paths report `no_sink` (`harness-core/src/harness.rs:532-534`, lead checked). `Observe::emit*` is not wired to the ring port (`observe/src/ring.rs:6-7`, lead checked). No traces, health check or panic hook.
3. **Manual incomplete and drifted.** No startup, safety, recovery or navigation section (`harness-core/src/manual.rs:56-70`); 16 parsed CLI flags undocumented; not generated from canonical descriptors, as contracts require; 0 of 362 tool entries.
4. **No single control and command source.** Five id vocabularies, 380 string id constants, two ids for save (`view/shell.rs:559`, `action.rs:868`); no separate operator and model views.
5. **Privacy has no persisted scope.** With no grant port a Session allows everything (`session/src/session.rs:147`; declared standalone behaviour, lead checked). No document or package stores an owner or AccessSpace; derived objects carry no scope.
6. **Multi-process write safety.** Save has no on-disk revision compare (`session.rs:2543`), the temp name is fixed (`view/export.rs:142`), no lease expiry.
7. **Foreground trap.** The manual says `studio-harness help`, but the desktop binary treats `help` as a file and opens a window (`harness/src/bin/studio-harness.rs:53-78`, lead checked). No-window is NOT_PROVEN.
8. **Integration refactor:** of 12 items, 0 done, 7 partial (RF-03, 04, 06, 07, 08, 09, 10), 5 not started (RF-01, 02, 05, 11, 12).

### Against the microtasks

**What status means:** `lifecycle.status` is PASS_Vn only when a round names that exact MT as met. Rounds judge lanes. So 6,146 PENDING, 62 PASS (lead recounted) says little about code: code is far ahead of status, and status cannot measure progress. Nor does PASS guarantee reach: 7 PASS_V1 vector effects cannot reach a document (`nib::live_effect` has no caller outside nib, lead checked), and PASS_V1 type features cannot be stored on a range. "398 PASS lanes" is a claim count, not acceptance.

**Proof commands:** about 5,400 to 5,500 of 6,220 MTs have a proof that cannot run: a test target that does not exist, or a NOT_IMPLEMENTED status (audit 5,519, lead's recount 5,410), and 39 have no checks. 53 of the 62 PASS MTs are among them; their PASS came from lane verdicts.

**Sampled engine state.** ESTIMATES from samples, and "built" means a test exists and reads as proving the clause. Treat built as an upper bound: 6 of 9 spot-checked built clusters fell to partial. The prefilter was wrong on 105 of the sampled rows in one audit, so never use anchor counts as progress.

| Domain | MTs | PASS | Audit's estimate | What is true |
|---|---:|---:|---|---|
| raster | 362 | 0 | ~69% engine present, ~20% missing | 35 blend modes, 20 adjustments, most of develop. Brush is parametric only; no merge or flatten commands |
| colour | 374 | 7 | ~50%, ~34% missing | ICC, soft proof, separations strong. No scopes, OCIO, display characterisation |
| motion | 226 | 1 | ~52%, ~42% missing | Keyframes and graph data headless; `wiggle`, `ease`, `random` unavailable; no scripting rows |
| compositing | 151 | 0 | ~45%, ~40% missing | No row built with an operator path; time effects and channel arithmetic almost absent |
| effects | 1,310 | 2 | ~23.5%, ~69% missing | 20 of 427 catalogue rows are effect kinds; 22 of 208 parameter tables. Keying works in pigment with no document path. Many rows are not effects (dialogs, plugin codes) |
| vector | 276 | 7 | ~35% built, ~29% missing | Paths, shapes, 9 of 13 geometry operations, brushes. No blend, live paint, trace, repeat, puppet |
| typography | 268 | 5 | ~27%, ~43% missing | Shaping, composers, H&J, kinsoku, features. `StoryRange` stores only style and language: no tabs, lists, indents, drop caps, ruby, vertical |
| layout | 217 | 18 | ~39%, ~25% missing | Strongest. Threads, books, TOC, index, notes, liquid, booklet. No autoflow, text-wrap record, table styles, output package |
| interop | 91 | 11 | ~16%, ~48% missing | See format list below |
| video_timeline | 163 | 1 | ~33 built, ~70 partial, ~50 missing | Edit engine and settings real. 0 of 453 export parameter ids, 60 of 674 command ids; writers MP4 and MOV only |
| audio (Score) | 199 | 0 | ~5, ~54, ~128 | Of 140 catalogue rows 97 are dialog or test sheets; of 37 real rows sampled, none fully built |
| design_systems | 166 | 0 | ~50, ~47, ~56 | Resolution and auto layout over imported `.fig`; no authoring commands, libraries, expressions |
| prototyping | 165 | 0 | ~31, ~28, ~87 | Player, transitions, overlays. No conditional or media actions |
| automation | 133 | 0 | ~25, ~45, ~63 | Actions and batch over only 8 commands; no object model or generic plugins |
| asset_library | 34 | 0 | ~5, ~21, ~8 | CKC is ports plus a test double |
| web_authoring | 180 | 0 | ~0, ~18, ~157 | A tested library no other crate uses |
| whiteboard | 65 | 0 | Does not exist | 0 of 55 sampled (lead checked: menu and descriptor text only) |
| operator_shell | 689 | 0 | ~15, ~141, ~313; ~200 GUI-only | No 362-tool registry; 25 panels and 33 tools in the egui test host |
| model_facing, manual | 54 | 0 | ~0, ~11, ~40 | The model surface is mostly missing |
| null domain | 1,219 | 10 | ~222, ~521, ~435 | Legacy, foundation, companion rows. Host adoption absent apart from the GPU client port |

**Formats (read / write):** PSD, PSB, TIFF, IDML, SVG read and write with round trip (vendor oracle only for IDML, and PSD engine output). PDF read and write, no tagged structure or encryption. PNG, JPEG read and write. AI: PDF stream only, no writer. Affinity: container only. FIG: read, `editable NOT_PROVEN`, no writer. DXF read only; EPS needs a worker for vector content. INDD, PRPROJ, AEP, SESX, DWG detected and refused. WebP refused; GIF, HEIC, JXL, BMP, TGA, FITS, PPTX, Sketch have no code.

**Blocked by the pivot:** 2,266 MTs are marked `vulkan_shell_client`. Registry, predicate, parameter-spec and generator logic can still be built headless.

**Do not repeat as truth:** the 2026-10-23 parity rollup (`research/parity-20261023-rollup.json`, "engine 55.8%"), anchor counts, MT status as progress, lane PASS counts as acceptance.

</topic>

<topic id="parity" status="active" updated_at="2026-11-11T15:30:00Z" summary="Feature parity per app vs Adobe, Figma and Affinity, 2026-11-11 (research/parity-20261111-rollup.json)">

## Parity on 2026-11-11

Method as on 2026-10-23: 308 feature rows over 9 apps, 108 GUI workflows (`research/feature-completeness-20261021.json`). Engine counts a validated row 1, built-not-validated 0.5, partial 0.25. Tiers: VALIDATED = PASS in R59 @ c4d06841, the latest finished round; UNVALIDATED = on origin 87f331f9 (197 commits since R59, R60 never opened); LOCAL = lane worktrees only.

| App | Rows | Engine % (V/U/L) | GUI pooled % | GUI strict % | Engine % on 10-23 |
|---|---|---|---|---|---|
| Photoshop | 36 | 57.6 | 61.1 | 30.6 | 47.2 |
| Lightroom Classic | 35 | 59.3 | 69.2 | 31.4 | 57.9 |
| Illustrator | 32 | 71.9 / 73.4 / 73.4 | 88.2 | 34.4 | 64.8 |
| InDesign | 31 | 58.1 | 70.0 | 25.8 | 54.0 |
| Premiere Pro | 25 | 63.0 | 70.0 | 48.0 | 56.0 |
| After Effects | 24 | 75.0 | 0 | 0 | 62.5 |
| Audition | 22 | 86.4 | 70.0 | 31.8 | 86.4 |
| Figma | 23 | 60.9 | 42.9 | 8.7 | 60.9 |
| Affinity (Photo, Designer, Publisher) | 80 | 46.3 / 46.6 / 46.6 | 73.3 | 27.5 | 43.8 |
| **All 9 apps** | 308 | **60.5 / 60.7 / 60.7** | **64.8** (70 of 108) | **27.3** (84 of 308) | 55.8 |

Without Affinity (8 apps, 228 rows): engine 65.5 / 65.7 / 65.7 (60.0 on 10-23).

Adobe products outside the row pool (not scored): Acrobat Pro (PDF engine, redaction, no app surface), Bridge (pieces only), Camera Raw (via the Lightroom develop engine), InCopy (story editor inside the layout editor), Dreamweaver (web host and site commands, SMB-WEB-*), Adobe Color (swatch groups), Media Encoder (export presets, partly), Character Animator, Animate, Premiere Rush, Adobe Podcast (nothing).

Biggest gains since 10-23: Illustrator window workflows 8 to 15 of 17 (13 hosted panels, area and path type, Create Outlines); Photoshop soft proofing, Info/histogram, actions and batch, gradient and paint bucket; InDesign liquid and alternate layouts; Premiere timeline with five trim modes, Nest, Rate Stretch and the first Export Media form; After Effects time remap, frame blending, roto and a 20-effect layer stack; Audition multitrack and automation lanes; Figma .fig opens editable; Affinity macros, batch and HDR merge.

Biggest gaps: After Effects has no window surface; new work since 2026-10-30 lands as typed commands without a window route, and the Vulkan host has no app workflow yet; Photoshop window lacks masks, adjustments, filters, retouch and crop; Lightroom has no real catalog and the AI features are absent; Premiere program monitor shows no media frames, multicam plays no angle, transitions all dissolve; Figma has no collaboration and no component, variable or constraint authoring in the window; Affinity native files only give a lossy preview; InDesign cannot open a PDF as a layout or set vertical text.

Readings: GUI numbers measure the frozen egui test host. 30 crates changed since R59, so their PASS verdicts need R60 attempt 3. Details and evidence: `research/parity-20261111-adobe-print.json`, `parity-20261111-video-audio.json`, `parity-20261111-figma-affinity-shell.json`.

## Spec coverage on 2026-11-11 (all 2,303 spec requirements)

Method: eight auditors judged every spec requirement ID against origin code and the latest finished round, with NOT_PRODUCT rows left out of the denominator and score = (VALIDATED + 0.5 x BUILT_NOT_VALIDATED + 0.25 x PARTIAL) / product rows, the parity weights.

| Status | Rows |
|---|---|
| VALIDATED | 579 |
| BUILT_NOT_VALIDATED | 52 |
| PARTIAL | 781 |
| MISSING | 640 |
| LOCAL | 9 |
| NOT_PRODUCT | 242 |
| Total (product rows 2061) | 2303 |

Overall: validated 28.1%, score 38.8%, missing 31.1%. Rows tagged with several apps count once per app.

| App | Product requirements | Validated % | Score % | Missing % |
|---|---|---|---|---|
| photoshop | 265 | 37.4 | 48.3 | 20.4 |
| lightroom_classic | 93 | 47.3 | 58.3 | 8.6 |
| illustrator | 264 | 34.5 | 44.1 | 28.4 |
| indesign | 340 | 31.2 | 41.5 | 27.6 |
| premiere_pro | 111 | 42.3 | 55.2 | 17.1 |
| after_effects | 179 | 40.2 | 52.2 | 14.5 |
| audition | 16 | 37.5 | 48.4 | 18.8 |
| figma | 382 | 28.3 | 36.8 | 38.2 |
| affinity | 163 | 39.3 | 50.0 | 18.4 |
| acrobat | 28 | 57.1 | 66.1 | 7.1 |
| dreamweaver | 161 | 1.2 | 7.1 | 74.5 |
| media_encoder | 37 | 18.9 | 47.3 | 8.1 |
| bridge | 31 | 22.6 | 35.5 | 25.8 |
| cross_app | 614 | 18.7 | 31.7 | 31.9 |

Top missing areas (MISSING rows covered):

- Web authoring (Dreamweaver): code intelligence, CSS and responsive authoring, templates, library items, snippets, behaviours, views and remote transports (119)
- Shell, tools and generated manual: dock engine, 362-tool registry, tooltips, value-field time, generated descriptors, shortcuts, storage and parallel-work host surfaces (95)
- Typography depth: paragraph controls, CJK and complex scripts, font management and proofing, character attribute model (63)
- Whiteboard mode: board node kinds, connectors, facilitation, templates, diagram generation (55)
- Interactive documents, prototyping, After Effects authoring and Premiere pipeline: buttons, forms, prototype logic, Figma Slides, expression and timeline editors, preview rendering (50)
- Automation and plugins: scripting object model, command contracts, plugin platform for design and graphics (48)
- Vector procedural constructs: repeat, blend, live paint, image trace, intertwine, global edit, symmetry (45)
- Colour management and measurement: video scopes, display characterisation, CIEDE2000 accuracy, OCIO and document colour policy, ASE swatches (41)
- Raster tools, brush engine and effects: brush depth, dodge/burn and other pixel tools, filter gallery, effect registration contract (39)
- Design-system libraries, variables, handoff and CKC write-back: publish/subscribe, variable commands, layout grids, inspect/codegen, asset provenance (36)
- Interop, history and model path: preset and library import, validation descriptors, StudioEditProposal, event-ledger undo/redo, kernel integration (26)
- InDesign layout production: document-wide layers, spreads, autoflow, bullets and numbering, multi-state objects, QR codes, snippets, real package (23)

Rollup: `research/spec-coverage-20261111-rollup.json`; group files: `research/spec-coverage-20261111-g1-*.json` to `-g8-*.json`.

</topic>

<topic id="vulkan-gui-pivot" status="active" summary="The Vulkan GUI pivot: decisions, code state, blockers, what can be done now">

## The Vulkan GUI pivot

**Do not forget this.** Studio's only GUI today is an egui test host on a retired path. All GUI lanes are paused. Full evidence: `research/truth-check-20261105-vulkan-pivot.json`.

### Decisions of 2026-11-07

The lead decided O1 to O10 on the Operator's word, after an agent team and a red team: `ledger.json` `direction.blocker_decisions_20261107`, which wins over the open questions below. GUI work is running: asset-local Vulkan host first, winit in one leaf crate, ash, offline SPIR-V, `kumihimo-*` crates, designer's spec in `gui-design/`.

### What was decided

| When | Decision | Words |
|---|---|---|
| 2026-10-30 | Direction | GUI must be Vulkan; Handshake is memory and GPU manager of all modules |
| 2026-10-30 | D1 | "Drop egui". Existing egui code is asset-local test host only; no new egui |
| 2026-10-30 | D6 | "Yes, engine lanes". Engine lanes resume; GUI lanes wait |
| 2026-10-31 | D7 | Pure Vulkan, no wgpu. render-gpu wgpu code quarantined; refactor RF-12; binding crate left open for the GPU owner |
| 2026-10-31 | D8 | Kumihimo stays shared-controls owner; non-egui pure-Vulkan renderer beside `kumihimo-egui` (Kumihimo asset work) |
| 2026-10-31 | STU-SHL-130 | Layer-based compositing |

These replaced D11 (eframe plus wgpu window) as a product path. Spec: v02.226..229, section 15 `15-native-desktop-gui-vulkan-gpu-ownership.md`, STU-ARC-023..033 in section 14.

### State of the code

- **Quarantines held, by rule only.** Since D7 (63ecf5a6, 157 commits): 0 lines added for egui, eframe, winit, wgpu, naga or WGSL; render-gpu changed 0 lines. No mechanical gate exists: `workspace-probe` still allows these crates.
- **`render-gpu` is not Vulkan yet.** It is wgpu 27.0.1 (`Cargo.toml:22`) with no `ash`, `vulkano` or raw Vulkan anywhere (lead checked). When rounds say "vulkan discrete RTX 3090", that is wgpu's Vulkan backend: asset-local proof, never host acceptance.
- **Shaders:** 9 WGSL files, 2,942 lines (lead checked), plus runtime-generated bodies, must become SPIR-V. Language and tool are undecided.
- **GPU client port:** `GpuClientPort` exists, but a host grant has no lease, so the backend returns `no_device`. It cannot yet receive a host device.
- **egui reach:** `controls/Cargo.toml:13` has a non-optional egui dependency (lead checked), which pulls egui into `harness-core`. The Viewer cannot build without egui. Public egui surface: 18 signatures by one count, 32 items by another. About 87% of harness-view is non-draw logic (heuristic).
- **To re-host:** harness-view 44k source lines, 10.5k and 20k lines of tests, 25 panels, 33 tools, `controls` `egui_adapter` and `menu_bar_ui`. 43 of 74 Viewer test files name egui.

### Blockers, in order

1. **WP-KERNEL-012's shell conflicts with D7, and nobody has raised it.** Its remote tip `cbce875e` (lead checked the ref) uses eframe, egui-wgpu and `kumihimo-egui`; MT-198 records that path as verified; its records never mention D7. Owner: Operator with the WP-012 owner.
2. MT-197 and MT-203 (Handshake): binding crate, shader path, device hand-off. NOT_STARTED.
3. Non-egui native Vulkan shell UI layer: window, input, focus, IME, DPI, AccessKit, text, docking, surface recovery. Owner Handshake. Nothing exists.
4. Kumihimo non-egui renderer, and where the control mechanic lives. Not started.
5. RF-01 and RF-05 (Studio), can start.
6. Host pane mount and host adapters (Handshake).
7. A non-egui test host for GUI lanes to prove against (Operator O6).
8. Re-brief paused GUI lanes: SMB-FIG-GRID-3, SMB-AE-KEYFRAME-TIMELINE-1, the 25 panels (RF-02).
9. RF-12. Canvas may show render-cpu read-back first, so this need not gate the first GUI lane (UNVERIFIED).

### Open Operator decisions

- **O1** Does WP-012 leave eframe, egui-wgpu and kumihimo-egui for pure Vulkan, and when? Everything above assumes yes.
- **O2** Name the Vulkan binding crate, or delegate to the MT-203 owner.
- **O3** Does D7 reach naga and WGSL as a source language, or only wgpu? Decides compile or rewrite for 2,942 lines.
- **O4** Boundary between the shell's UI layer and Kumihimo's renderer: who owns text, IME, AccessKit, widget drawing.
- **O5** Where the control mechanic lives: STU-ARC-023 says Kumihimo, the code says `hsk-studio-controls`, Kumihimo core has none.
- **O6** Until the layer exists: wait entirely, allow window-free Viewer logic lanes only (today's practice), or build a non-egui asset-local Vulkan test host.
- **O7** May render-gpu be refactored behind its port before the owner exists, or is that "new wgpu code"?
- **O8** Platform scope after D7: macOS without Metal; Windows without a usable Vulkan driver.
- **O9** D3 was never answered; the lead reading stands (Studio defines the client port, Handshake the owner).
- **O10** Spec-owner items D7A-01..17 and SD-31..52.

### What Studio can do now, without waiting

- RF-01 phase 0: move egui out of public signatures and Viewer fields into a gated leaf. It moves toolkit code and adds none. Needs a lead ruling: `direction.dispatch` pauses harness-view surfaces, while the inventory lists RF-01 under phase 0.
- Make egui optional in `controls`; gate `egui_adapter` and `menu_bar_ui`.
- Move gesture semantics out of `egui_adapter.rs` into renderer-free code; convert tests that do not assert drawing to the headless path.
- A mechanical quarantine gate in `workspace-probe`, so D1 and D7 stop depending on reading.
- Send the port gap to the MT-203 owner; build a GPU parity table so RF-12 knows its backlog.

**Risks:** every render-gpu PASS row gets re-proved at RF-12; engine lanes keep adding CPU behaviour while GPU kernels are frozen (composite +1,074, prism +2,877 since D7; GPU body need NOT_INSPECTED), so list CPU-only operators as typed refusals; a pure-Vulkan client inside an eframe shell means two device owners, violating GUI-DESKTOP-025.

**Stale records that contradict this:** `STUDIO_MODULES.json` responsibilities; `integration-refactor-inventory-20261030.json` "DX12-only" (true only at 292d7a1c).

</topic>

<topic id="workflow" status="active" updated_at="2026-11-07T05:14:28Z" summary="How the work runs from 2026-11-07, on the Operator's instruction">

## Workflow from 2026-11-07

**Source:** the Operator, 2026-11-07, on the lead's suggested list: "record the new suggest workflow so future sessions and handoffs use it", "worktrees are allowed only if you merge en remove them after completion", "chrome stays open because i am using it", "also manage cargo better. reuse when posible, remove only after inspection of the proposed cargo removal/ or partially removal". **Authority:** Codex CX-GIT-001A, root HSK-ART-005, `ledger.json` `direction.workflow_20261107`, which also holds the account of why this changed.

1. **Keep several lanes running.** One builder per lane, disjoint files, one lane pushing at a time. A round takes hours, so it should judge many fixes, not one. If the team shrinks to one builder, tell the Operator why.
2. **New lane: reuse first.** Run `tools/lead/target_report.py`. Take a completed lane's clean worktree and its target together, choosing the one whose built crates overlap most: confirm its tip is on origin, then `git -C "$W/<old lane>" switch -c lane/<new lane> <base>`. Give `lanegen` `tree=<old lane>`; `--owner` stays the old lane's dir, because the target is bound to that tree. Add the branch to `direction.workflow_20261107.lane_branches`. Never point two trees at one target (`diagnostics/stale-artifacts.md` M2). Only if nothing fits, create a new worktree. sccache is not the answer: measured at 0% hits across lane targets on 2026-10-20 (`START_LOG.md`), so do not retry it unless that cause changed.
3. **Close what you open.** A worktree stays only until its lane is complete: PASS verdict and no next lane reusing it. Then `bash tools/lead/lane_close.sh <dir> <LANE-ID>` (dry run) and again with `--apply`. It checks the verdict, a clean tree, that the tip is on origin, and that no process names the tree. It removes with native Git only and writes a read-only record. If it stops, stop: no manual cleanup. The roughly 100 older worktrees are not covered: list them for the Operator.
4. **Cargo removal: inspect, propose, then approve.** Before proposing any removal, inspect with `tools/lead/target_report.py` and show what each part holds and what would rebuild. Prefer the partial removal that keeps compile reuse: test binaries take most of the space (47 of 51 GiB in `smb-pr-retime-output-1`) and cost only a relink, while third-party libraries are about 1.7 GiB and cost a compile. The Operator still approves each exact list (root HSK-ART-005, CX-107). On 2026-11-07 they also said "manage asigned diskpace/folders", but the lead's edit relaxing HSK-ART-005 was refused by the harness's permission check, so the root rule stands until the Operator changes it. Plan R is approved. Unchanged: incremental stays off, the registry cache is never removable, C: tree cap 360 GiB. Measured 2026-11-07: one test had 10 fingerprints across 4 feature sets, and `.pdb` files were 36 of 47 GiB of test binaries; open follow-ups are in `research/cargo-cleanup-plan-20261107-R.json` `findings`.
5. **Decisions go to the Operator one at a time, in the next-step box.** A question buried in a long message can go unseen: CX-GIT-001's went unanswered for about 16 hours while the team ran one builder. Keep the rest in the `operator-items` topic.
6. **Chrome stays open.** Do not ask to close it. Measure commit free before a round and record it. If it is under 40 GB (the lead's judgement, not a measurement), say so and let the Operator decide; builders then use `CARGO_BUILD_JOBS=2`. The page file is system-managed, so the commit limit moves (199.5 GB, later 177.9 GB). Rounds take no runtime override (CX-VAL-007).
7. **Do not let one edge case hold the team.** The fix budget (CX-EXEC-015) still ends a loop at 3. Keep other lanes moving during it. Before ruling on behaviour, read the code and ask what kind of clip or row it is; give a validator the lead's checks as claims to break.
8. **Watch test-run time.** Compare each union's duration when recording a round. On file: R52 92 units 2 h 03; R54 160 units 2 h 03; R56 94 units 2 h 23; R57 160 units 3 h 48; R58 91 units 3 h 48. The slowdown's cause is UNVERIFIED; 44 of the lead's leftover watchers and a Chrome memory shortage were both present during R57 and R58. Look into a scoped union over 3 hours.
9. **Priority (Operator, 2026-11-07).** Width and depth across every Adobe, Figma and Affinity tool and feature, and a working, coherent GUI, come first. Then fix what is broken. Finish fixes already underway quickly and open no new edge-case lanes while feature work waits. The lead may expand scope and create MTs when that helps: do not spend time on MT paperwork that moves nothing.
10. **Review before expensive builds.** A builder's first runs are `cargo check` and its focused tests. Before its bundled run, read-only code-check and red-team agents inspect the diff (`builders/DISPATCH_TEMPLATES.md` is not changed by this; the prompts are in `direction.operator_direction_20261107.review_prompts`). They report only what would break or be wrong, never polish.
11. **Minimal testing, no busy work.** Red test and focused tests per item, one bundled run. `--per-sha` runs on the tip only (`--per-sha HEAD~1`), not on every commit. Landing has no push slot and no round hold (`direction.blocker_decisions_20261107.landing`); a READY lane with its tip on origin hands over its tree and target. Rounds run in batches when a real wave has landed, not after each fix, while CX-VAL-011's full-round rule still applies to the rounds that run. READY still means unjudged, and only a validator's verdict says PASS.
12. **No stray cargo.** Builders run cargo only through `lane-check.sh` or after sourcing `builders/studio-env.sh` with their owner. `tools/lead/stray_cargo.py` reports repo-local targets, stray artifact folders and processes writing outside the assigned roots; `lane_watch.sh` prints it.
13. **Blockers are the lead's to resolve.** The Operator: "resolve all blockers on your own, deliberate with a fitting agent team". Use a position agent per concern, then a red team, then record the decision in `direction.blocker_decisions_20261107`. Do not hold work on a question to the Operator when a recorded decision can be reversed later.
14. **GUI designer and workflow agents.** A designer owns the Vulkan GUI's look, layout and coherence (`gui-design/`). Workflow agents write professional projects end to end as scenarios (`workflows/scenario-*.json`): tool calls, GUI expectations and a measured gap list. The gap lists choose the next feature lanes, and later the same scenarios test the tools and the GUI.
15. **Leave no processes behind.** `lane_watch.sh` now ends itself after `WATCH_MAX_S`. After stopping a watcher, count what is left. Before any pause, confirm none remain.
16. **Lanes land one at a time (amends item 11, 2026-11-07).** With many lanes on one disk lock, free-for-all landing livelocked: SMB-DOC-NEW-1 rebased five times in about 3 hours, because each landing touched a file it edits and each re-check waited 30 to 60 minutes in the D: queue. Builders now hold their push until the lead releases them, and the lead releases one lane at a time. A lane whose push needs no run (a clean, disjoint rebase) may go ahead of one still waiting for a check. The rest of the landing rule stands: rebase; cargo check only if landed commits touch a file the lane edits; `--per-sha HEAD~1`; guarded push.
17. **Trunk check after about every 3 landings.** A builder whose tree sits at origin runs `cargo check --workspace --tests`, plus the harness [viewer] and native-host [document] units, on D:. All green on 2026-11-07: 5341e561, 672a1804, 9ab80edd.

</topic>

<topic id="state" status="active" updated_at="2026-11-08T05:40:01Z" summary="RUNNING again from 2026-11-08 05:30Z on the Operator's word; R60 FULL on C:, six lanes on D:; older sections kept for history">

## PAUSED 2026-11-11 ~15:20Z for the Operator's cargo cleanup

**RESUMED 2026-11-12T08:22:07Z** on the Operator's "Resume lanes": all 8 lane agents restarted from their recorded resume steps (INDENT and SCORE first on the D: lock). DEVELOP-COMMANDS-1 lands now because it was RELEASED before the pause; INDENT and SCORE keep priority among the rest. Record: ledger `direction.pause_20261111.resumed`.

Operator: "lets pause all agents so we can do the cargo clean up faster ... we will resume work after the cargo clean up". Every Studio build process tree was stopped; each lane's tree, branch, head and resume step are in `ledger.json` direction.pause_20261111. All trees are clean except SMB-DS-COMPONENTS-AUTHOR-1 (8 uncommitted files, left as is). Origin is 87f331f9. Landed today: PARSER-HOSTILE-1 27365b81, LAYOUT-PLACE-1 87f331f9. SMB-DEVELOP-COMMANDS-1 was RELEASED and rebased (4506a6cf) but not pushed. To resume: SendMessage each lane agent (ids in the pause record) or dispatch takeover builders with the rulings file; INDENT and SCORE land first, then plan T T-C01/T-C02 run, then R60 attempt 3.

Disk state: C: Studio tree about 344 GiB after plan T T-C04 (19.47) and plan U (25.97); plan T T-C01/T-C02 removed 111.27 GiB more on 2026-11-12 12:53Z (C: free 334G to 445G, `research/cargo-cleanup-result-20261112-T-c01-c02.json`). D: Studio used 1,912 GiB before plan V (WizTree 2026-11-11 14:55Z): steward-export 540, active lane targets 66 to 205 each. Plan V EXECUTED 2026-11-11 17:45Z to 18:01Z on the Operator's "can you execute it?" (`research/cargo-cleanup-plan-20261111-V.json`): older copies removed from 11 Studio D: caches, 1,395.2 GiB, 27,841 files, all 11 REMOVED, keeping the newest copy of every binary and all libraries (`research/cargo-cleanup-result-20261111-V.json`); the lead's post-check found no duplicate test or binary copies left in any candidate's deps folder. D: free 3.3 TiB after the run. The Operator's separate delete of every non-Studio Handshake_Artifacts entry ("delete it all") ended 2026-11-11 19:29Z: all 32 entries GONE, none left, only WP-KERNEL-STUDIO remains in Handshake_Artifacts; D: free 4,264 GiB (801 GiB before that delete and plan V).

## RUNNING (2026-11-08, from about 05:30Z; updated 2026-11-08T08:55:13Z)

**Operator:** "ok lets continue, i will go to bed, i expect you to resolve blockers and deliberate with your team on questions so you dont have to ask me." Vox Vulgi may be closed only if it blocks work; Chrome stays open. Record: `direction.resume_20261108` (with r60 attempts, steward sync, trunk break, review rulings, landing queue).

- **R60:** attempt 1 at cede5958 did not open (F-R60-1, fixed by steward 5d020939 with two lead-approved edges). **Attempt 2 FULL with canary at 35c50d3b** (120 commits, 21 claims) runs on C: from about 07:22Z. Its candidate carries the harness_drive_typed pin break fixed later in 89e9a2ac (validator told).
- **Landed today, READY, unjudged:** SMB-PR-EXPORT-RECIPE-1 8712fa23; VK-UI-PASS-1 compose hook 42e9e5a4 (no claim); steward 5d020939; SMB-ARTBOARD-EXPORT-1 35c50d3b plus fixture fix 89e9a2ac; SMB-DRIVE-TYPED-2 08b372af; SMB-PLACE-ASSET-1 0a733db8; SMB-GUI-FRAME-1 eb17fad6 (designer MATCHES_DESIGN; D-F7, N1, N2 to the next GUI lane).
- **C: lanes held for R60 (HOLD C: 07:20Z), each reviewed and fixed:** SMB-AI-LIVE-EFFECTS-1 (bundled green f962ff49, item 5 unbuilt), SMB-TYPE-INDENT-SPACING-1, SMB-SCORE-EFFECTS-6, SMB-PARSER-HOSTILE-1. On R60's union end: send C: RELEASED; they check, prove red, run bundled, land one at a time (PARSER yields).
- **D: lanes:** SMB-VK-UI-PASS-1 (validation on, 0 errors; fixing review F1-F6, then rebase on GUI-FRAME-1 and fill StatusFacts.chrome); new width wave on base eb17fad6 from `research/width-plan-20261108-*.json`: SMB-LAYOUT-PLACE-1 (smb-id-toc-persist-1), SMB-WEB-SITE-1 (smb-sentinels-native-1), SMB-DS-COMPONENTS-AUTHOR-1 (smb-ai-distort-effects-1), SMB-DEVELOP-COMMANDS-1 (smb-folio-angle-unbounded-1), SMB-MOTION-COMMANDS-2 (smb-pr-sequence-session-1), SMB-DELIVERY-QUEUE-1 (smb-r53-followups-1). Each stops at REVIEW READY for code-check and red-team, then BUNDLED GREEN, then lands on RELEASED.
- **Next width candidates (planned, gated):** SMB-DS-VARIABLES-AUTHOR-1 (after DS-COMPONENTS), SMB-PS-FILTER-ADJUST-COMMANDS-1, SMB-TYPE-CHARACTER-ATTRIBUTES-1 (after the indent lane), SMB-BOARD-1 (after AI-LIVE-EFFECTS), SMB-PR-MULTICAM-PLAY-1 (after SCORE-EFFECTS-6), SMB-PR-CONFORM-PEAKS-1. The next GUI lane starts with D-F7, N1, N2.
- **Process lessons today:** lane-check runs neither registry-sync nor the probe (F-R60-2): the lead runs the registry check before each freeze. A lane that changes a Viewer refusal order runs every harness process test driving that command. Shared files between parallel lanes are ruled per hunk with a landing order.
- **2026-11-08 about 10:10Z: every agent stopped on the account's weekly usage limit** (reset 2026-11-10 12:00 Europe/Brussels). Work was on disk; no process or lock was left. Resumed 2026-11-11 from 06:35Z with fresh takeover builders (cheaper than resuming 400k-800k-token contexts), reading `research/lead-rulings-20261108-by-lane.json`, where every ruling given only by message is now written down.
- **R60 attempt 2** stopped on the C: cap. **Plan T approved by the Operator on 2026-11-11** (older copies in the four C: lane targets, executed per candidate after each lane lands, after a fresh inspection; a candidate past the tool's 5% check is presented again). The C: tree measured 309 GiB on 2026-11-11. **Plan T EXECUTED:** T-C03 and T-C04 on 2026-11-11; T-C01 (59.16 GiB) and T-C02 (52.1 GiB) on 2026-11-12 12:53Z after INDENT and SCORE landed, 111.27 GiB of older copies off C: (`research/cargo-cleanup-result-20261112-T-c01-c02.json`). R60 attempt 3 follows, with no C: lane build during it.
- **Process change:** lane-check.sh step 0 now runs registry-sync.py in check mode (exit 70 on drift). SMB-R60-FOLLOWUPS-1 fixes the attempt-2 probe failures; web, photo.info and motion probes went to SMB-WEB-SITE-1, SMB-DEVELOP-COMMANDS-1 and SMB-MOTION-COMMANDS-2.
- **Vox Vulgi** (desktop.exe 141544) reads its data in cache-served bursts; not blocking; left running.

## PAUSED (2026-11-07, about 18:20Z)

**Operator:** "let the agents finish their work then lets find a spot to pause work". Every agent reached its stop point and reported; no builder, reviewer, scope agent or watcher is running. No round is running. Nothing is pushed beyond what is listed.

**Product origin:** cede5958. **READY_FOR_VALIDATION, unjudged (19):** routing-repair, write-safety, harness-drive, diag-sink, controls-egui-gate, gui-steward, motion-commands, command-surface, pigment-filter-kernels, ui-tree, audio-out-mix, doc-new, vk-host, doc-export, layout-pdf-host, project-bin, web-host, layout-persist, gui-spine. Trunk checks green at 5341e561, 672a1804 and 9ab80edd.

**C: has room again.** Plan S executed on the Operator's approval (`direction.c_cap_20261107.plan_s_executed`): 169.15 GiB of older test-binary copies removed; C: free 338G to 504G; the C: Studio tree measured 208 GiB (du, 18:03 to 18:27Z) against the 360 GiB cap. The round target kept its newest copy of every binary, so R60 relinks only what changed.

**First action on resume: R60 FULL with canary.** Freeze the origin tip, then `python tools/lead/dispatch_r60.py <tip8> <commits since c4d06841> --write` writes `validation/dispatch-prompt-R60.txt` from the ledger's READY lanes. Read it, then dispatch one opus validator. Builders stay on D: during the round. A lane may land during it (CX-VAL-001), one at a time.

**Paused lanes (code committed locally, not pushed; each lane file's `next` is exact):**

| Lane | Tree (owner, disk d) | Branch | Tip | Base | Left |
|---|---|---|---|---|---|
| SMB-PR-EXPORT-RECIPE-1 | smb-r53-followups-1 | lane/smb-pr-export-recipe-1 | c79c858d | aa68164c | Review done (`research/review-smb-pr-export-recipe-1-c79c858d.json`, FIX_FIRST): F1 an output named *.vtt with a captions sidecar is overwritten by the sidecar while the receipt says Complete (refuse in CaptionPlan::new); F2 receipt.captions None when the media is published and the sidecar rename fails; F3 use std::fs::hard_link for no-clobber publish, rename fallback on FAT/exFAT. Then bundled, land. |
| SMB-ARTBOARD-EXPORT-1 | smb-ai-distort-effects-1 | lane/smb-artboard-export-1 | 9db105fc | aa68164c | Pre-build review, bundled, land. Adds session token `export_settings`. |
| SMB-PLACE-ASSET-1 | smb-pr-sequence-session-1 | lane/smb-place-asset-1 | 9d83222c | a3215161 | Pre-build review, bundled, land. Adds session token `raster_records`: whichever of these two lands second merges DOCUMENT_ENTRY_TOKENS and the count in session tests/entry_references.rs. Named, not fixed: layers.rs duplicate_layer copies a placed layer's pixels without its record. |
| SMB-DRIVE-TYPED-2 | smb-pr-edit-footprint-1 | lane/smb-drive-typed-2 | 2bbe2207 | aa68164c | Pre-build review, bundled, land. About 80 typed rows; ALL_ROWS is a static; doc-new's rows .opt() and document arm done. |

**Scoped, not dispatched (GUI line, one at a time before B9; these two touch different crates and may run together):**
- SMB-GUI-FRAME-1 (new name): `research/scope-20261107-gui-frame-1.json`, tree smb-pr-entry-references-1. The complete static raster frame (B5) on the CPU painter. Decided 2026-11-08 (`direction.lead_rulings_20261108.gui_frame_1`): rename the Viewport id to `studio.viewer.canvas` (STU-SHL-066 wins over the spine's `studio.viewer.raster`); unbuilt controls follow B6 (disabled with a reason); icons in this lane, cut first if it passes 10 commits. The lane also fixes the design data it found: two fit_table rows 1 px short; `studio.toolgroup.smart_select` fails the STU-MDL-103 id charset.
- SMB-VK-UI-PASS-1 (red team's proposed name): `research/scope-20261107-vk-ui-pass-1.json`, tree smb-pr-retime-hold-1. The GPU draws kumihimo-ui's display list with offline naga SPIR-V and glyph and icon atlases, pixel-compared against the CPU painter. Approved 2026-11-08 (`direction.lead_rulings_20261108.vk_ui_pass_1`): the nested spv-build Cargo.lock (34 packages, all already in the root lock and cache) and one exact workspace-probe WGSL allow entry under O3. It runs its GPU tests with the validation layer and goes READY only validation-clean (O7).

**Waiting width lanes:** SMB-AI-LIVE-EFFECTS-1, SMB-TYPE-INDENT-SPACING-1, SMB-SCORE-EFFECTS-6 (re-scope first), SMB-PARSER-HOSTILE-1, and a filter-commands slice for the 19 new kernels (not scoped).

**Open with the Operator (2026-11-08):** nothing. **The Vulkan validation layer is here.** The Vulkan SDK 1.4.363.1 core package is installed in `C:\Program Files\VulkanSDK\1.4.363.1` (04:41Z, on the Operator's retry of the admin prompt). The loader lists VK_LAYER_KHRONOS_validation 1.4.363 machine-wide (HKLM ExplicitLayers) on the RTX 3090, and spirv-val v2026.4 runs (`direction.lead_rulings_20261108.vulkan_sdk.proof`). Lanes turn the layer on with `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation`, and receipts say validation: on, not absent. O7 and SMB-VK-UI-PASS-1 go READY only validation-clean. The P3 reading, the three vendor tooltip sentences (ship as the spec has them; STU-IO-106 forbids inventing labels) and the GUI scope leanings are now lead rulings in `direction.lead_rulings_20261108`, not Operator questions.

**Still true:** stash a06b4ea6 stays in the shared stash list (a builder's; left as instructed). The C: trees of the five lanes moved to D: are idle and untouched. Lane worktrees stay until their lanes PASS, then `tools/lead/lane_close.sh`.

## State at about 17:40Z (2026-11-07, before the pause)

**Authority:** `ledger.json` lanes and `direction.workflow_20261107`, `direction.blocker_decisions_20261107`, `direction.c_cap_20261107` (with `p3_note` and its amendment).

- **Product origin:** a3215161. R59 PASSED (SMB-PR-MEDIA-MARKERS-1 PASS_V3). Since then 18 lanes landed READY_FOR_VALIDATION, unjudged: routing-repair, write-safety, harness-drive, diag-sink, controls-egui-gate, gui-steward, motion-commands, command-surface, pigment-filter-kernels, ui-tree, audio-out-mix, doc-new, vk-host, doc-export, layout-pdf-host, project-bin, web-host and layout-persist. Every one had a pre-build review, and every review found real defects, fixed before its bundled run.
- **R60 is blocked on C: room.** It must be FULL with a canary (the steward changed root inputs). The C: Studio tree measured 377 GiB at 12:25Z against the 360 GiB cap, so the guard refuses C: builds. Proposal S (`research/cargo-cleanup-plan-20261107-S.json`, 169.2 GiB of older test-binary copies, mostly in the idle round target) waits for the Operator; nothing is deleted without approval. The dispatch prompt is generated at freeze by `python tools/lead/dispatch_r60.py <candidate8> <commits since c4d06841> --write` from the ledger's READY lanes (FULL plus canary, the new Vulkan crates, the GPU lock).
- **All building is on D:.** Five C: lanes moved to idle D: trees at 12:35Z; their C: trees and branches are untouched (`direction.c_cap_20261107.moves`). D: holds more than 6 non-PASS lane targets (P3 note): no lane creates a new D: target; new lanes reuse READY lanes' trees.
- **In flight:** SMB-GUI-SPINE-1 (first real panel: a layered document beside a kumihimo-ui Layers panel in the Vulkan window, capture `Handshake_Artifacts/WP-KERNEL-STUDIO/smb-gui-spine-1/gui-final-offscreen-5e882c1d.png`), in pre-build review. A width wave on base aa68164c: SMB-ARTBOARD-EXPORT-1, SMB-PLACE-ASSET-1, SMB-PR-EXPORT-RECIPE-1 and SMB-DRIVE-TYPED-2 (with doc-new's .opt() and document-arm fixes).
- **Waiting width lanes (scoped):** SMB-AI-LIVE-EFFECTS-1, SMB-TYPE-INDENT-SPACING-1, SMB-SCORE-EFFECTS-6 (re-scope first), SMB-PARSER-HOSTILE-1 (safety track), and a filter-commands slice for the 19 new kernels (not yet scoped).
- **Open with the Operator:** plan S; the Vulkan SDK download (`vulkansdk-windows-X64-1.4.363.1.exe`, 303,066,832 bytes, sdk.lunarg.com) for the validation layer; the P3 reading (disk space or concurrency).
- **Notes for the report:** stash a06b4ea6 stays in the shared stash list (a builder's, left as instructed); 16-bit PSD layer-info placement against Photoshop is UNVERIFIED; two engine defects pinned by tests (export_svg on a group gives DuplicateIdentity; replace_text with a newline keeps one paragraph).

## Earlier state (2026-11-07 morning, new lead running)

**Authority:** `ledger.json` `direction.workflow_20261107`, `direction.resume_20261106` and `direction.after_r57_20261106`, `validation/round-R57.json`. They supersede the pause text below, which is kept for history.

- All five paused lanes landed; product origin 4ffe1b06 at STUDIO-R57 FULL: 406 PASS, 1 FAIL. HIGH F-R56-1 closed on the crash.
- STUDIO-R58 at 89bbf702: 325 PASS, 1 FAIL. SMB-PR-MEDIA-MARKERS-1 is FAIL_V2 on reversed audio (F-R58-1); fix budget 2 of 3, so one more FAIL escalates to the Operator. Repaired on origin at c4d06841, READY, SAFE_TO_JUDGE by the third V4-13 read (`research/pre-review-r59-media-markers.json`). Approach changed under CX-EXEC-003 after three attempts: alignment reads the continuous mapping on every clip, appearing reads keep the frame law on Video (`direction.after_r58_20261106.approach_change`).
- DIRECTION: the Operator handed development to the lead on 2026-11-07 (`direction.operator_direction_20261107`): width, depth and a working Vulkan GUI first; workflow topic items 9 to 14.
- NEXT ACTION: read STUDIO-R59 when it is recorded (`validation/dispatch-prompt-R59.txt`). The memory gate opened on 2026-11-07: commit free 120.5 GB with Chrome open at 14.1 GB. A full round is due by R60 (CX-VAL-011).
- Lead error on record: the new lead approved a merge_sync pin by a hand check that missed the row's clip was audio. Withdrawn. Lesson: ask what kind of clip a row holds before reasoning about frames, and give a validator the lead's checks as claims to break.
- New lanes are allowed again under Codex CX-GIT-001A, on the Operator's condition: merge, then remove (workflow topic).
- Open with the Operator: O1, O3, O6 (lead's picks in the session report: O1 yes through a separate new WP while WP-012 keeps its pin; O3 wgpu only, WGSL compiled by naga; O6 window-free Viewer logic only), cleanup list, waivers (F-R57-14).
- Lead reading on SD-67 withdrawn and SD-64 revised on F-R57-3 and F-R57-2.
- C: physical free fell 415 to about 240 GB outside Studio's tree (cause UNVERIFIED). R57's default build took 28 m against 13 m from outside load.

## State at the pause (2026-11-05)

**Authority:** `ledger.json` `direction.pause_20261105`. Nothing is running; the lead confirmed no cargo, rustc or nextest process.

- **Product origin:** 1940292c on `codex/handshake-studio-assets`. **Spec:** v02.229.
- **Disks at the pause:** C: 415 GB free (Studio tree cap 360 GiB, alert 330, stop 355), D: 1,385 GB free.

**READY for STUDIO-R57 (claims on origin):**

| Lane | Claim | Notes |
|---|---|---|
| SMB-PR-TRACK-DISPLAY-1 | e8647bfa..c143625f | MT-33739 partly met; SD-49, SD-50 |
| SMB-PR-EDIT-FOOTPRINT-1 | 56c3a5f3..41d0e0da | Exhaustive clip diff; F-R56-10's `plan_move` part |
| SMB-LAYOUT-BLANK-LINE-LOOKUPS-1 | c9648315..66c0e7c9 | F-R56-5, F-R56-9; SD-51 |
| SMB-TYPE-AUTHORED-TOKEN-1 | 1894d7f7..1940292c | F-R56-4, F-R56-6; MT-30898 partly met; SD-52. Whole type, folio, session, package and harness-core suites last ran at 62f81467 |

**Paused, not on the product branch, status null. Each tip is also on a GitHub backup branch, `lane/<lane name in lower case>` (`direction.backup_20261105`). Exact steps are in each lane file's `next`:**

| Order | Lane | Local tip | Disk | Last result | Left |
|---|---|---|---|---|---|
| 1 | SMB-PR-RETIME-OUTPUT-1 | cff47aef | C: | Bundled 326 passed; per-sha OK on old base | Rebase, harness-view suite once, per-sha, new tests and wall-clock watch, push |
| 2 | SMB-PR-NUDGE-1 | 31de741d | C: | Bundled 317 passed; per-sha OK on old base | Same shape |
| 3 | SMB-PR-ENTRY-REFERENCES-1 | 4529259c | D: | Bundled 932 passed | Rebase (conflicts expected in session), classify `composition_token` in `tests/entry_references.rs`, one bundled run, per-sha, push |
| 4 | SMB-LAYOUT-COMPOSER-DEPTH-1 | 286d9241 | C: | Focused green; 1549 rows byte-identical | First bundled run (no clippy or whole suite on final code), timings, rebase, per-sha, push. Closes HIGH F-R56-1 |
| 5 | SMB-PR-MEDIA-MARKERS-1 | 3cfbbeed | C: | Folio 11, session 57 focused green | Furthest: the pulse read has never compiled. `git apply -R` of `resume/pulse-green.patch`, pulse red, then its `next` |

Backup branches stay at these tips. Rebased or amended tips go to the product branch only, never to a backup branch: that would need a force push, which is forbidden. Deleting them needs Operator approval once each lane has landed.

Media markers also has 3 uncommitted files in its worktree. GitHub branch `lane/smb-pr-media-markers-1-uncommitted` holds them as one restore-only commit, `b1a299ed`, on top of `3cfbbeed`: never merge it, because its pulse read has never compiled. The files stay uncommitted in the worktree, so the lane's `next` runs as written. On one disk only: the `resume/` helpers that `next` uses (`pulse-green.patch`, `nx.sh`, `package_green.py`).

The composer fix has been waiting longest for a HIGH finding; if the C: queue allows, it may run beside the first two.

**Lead decisions due when these lanes finish:**
- Composer: the Knuth-Plass 2^25-node cap refuses some very large paragraphs the base could answer (about 1,500 long hyphenated words failing the first pass, or about 45,000 CJK glyphs with mojikumi). It is typed, but it changes an answer. Read `notes.measurements` before accepting.
- Composer: 8 justified rows (80, 334, 609, 807, 842, 1201, 1269, 1440) keep the base answer, where the last line takes the rest of the paragraph with the excess in `hang_pt`, not marked overfull. Fixing changes output; it needs its own lane (candidate recorded).
- Entry references: confirm three changed pins the lead did not name: `multicam_commit.rs:372`, `audio_routing_records.rs:121`, `:189`. Record the inert rule as an SD item on READY.

**Overrulable lead readings now in code or lanes:** composition token @3 instead of a spec exception; display entries inert (SD-49); no stored token reads as @1 with a typed refusal for U+2028 in text saved since 2fba5f61 (one constant; SD-52); `audio_routing` inert when its sequence, clip or track is gone (the lead's earlier refusal reading was withdrawn when the builder showed Run commits the sequence before the entry; sequence settings and multicam keep their refusal).

**Tree for read-only audits:** `wt-studio-lanes/smb-type-authored-token-1` is clean at 1940292c.

</topic>

<topic id="next-work" status="active" summary="Recommended work order after R57, with the reason for each">

## Work order

Lead's recommendation from the truth check. The Operator's instructions override it. "Lead reading" marks inference.

1. **Paused lanes, then R57 FULL.** This turns 54 unjudged and local commits into verdicts and closes the HIGH.
2. **Vulkan decisions O1, O3, O6, O5.** The largest strategic blocker costs a conversation, not a build.
3. **Superseded in priority by the workflow topic item 9 on 2026-11-07; order below is still the reasoning.** **Studio-side pivot prep:** egui optional in `controls`, the mechanical quarantine gate, RF-01 on a lead ruling. Every Viewer lane otherwise adds rebase surface.
4. **One typed command and control contract, with a process-level drive path.** Four audits independently rank this first: `hbr-usage` gap 1, `hbr-host` gap 3, `mt-video-audio-design-web` missing 1, `mt-shell-manual-foundation` missing 1 and 2. Crates: `session`, `harness-core`, `kernel-bindings`, `controls`; MT-33547..33557, MT-33500..33507, MT-33634..33639, MT-35107..35133. It is what makes STU-CON-007 true. The schema part does not depend on O5 (lead reading, UNVERIFIED).
5. **Durable diagnostics and a real manual:** default JSONL sink, wire `Observe::emit*` to the ring port, stop dropped frames, health check, panic hook; generate the manual from descriptors; fix the `help` window trap.
6. **Multi-process write safety:** on-disk revision compare, unique temp names.
7. **Engine gaps with the highest reach, each already scoped in the audit files:**
   - Character and paragraph attribute model (`folio`, `layout`, `type`): blocks tabs, lists, indents, drop caps, ruby, and stored OpenType features.
   - Vector live effects wiring (`folio`, `composite`, `nib`), then blend, live paint, trace, repeat.
   - Composition effect kinds, starting with maths already in pigment; keying document path, MT-34092..34108 (`folio`, `composite`).
   - Filter kernels: 5 exist; effect gallery 0 of 43 (`pigment`). Brush tips, merge and flatten.
   - Timeline delivery and export writers (`pulse`, `reel`, `session`): MT-33742, 33741, 33765, 33734, 33732, 33766..33797.
   - Audio processors (`score`): MT-32661, 32662, 32667, 32706..32709.
   - Design authoring (`folio`, `components`): MT-33062, 33068, 33140, 33147.
   - Whiteboard and web authoring are nearly untouched domains; the Operator should say when they start.
8. **Vendor oracle session.** The weakest truth in the product. Needs the Operator at the apps. Premiere, Audition, Affinity and Figma first; carried items are in `operator-items`.
9. **Hardening:** fuzz the parsers (PSD, PDF, TIFF, RAW, `.fig`, IDML, SVG), check `interop-fig-native export.rs:384`, CI and a clippy gate, a release-profile run, gate the HBR pillars in rounds.
10. **Record repair, kept small:** `STUDIO_MODULES.json` to 56 crates with true status and no wgpu or egui wording; `_MT_INDEX.json` counts; MT proof commands repointed (F-R55-3, F-R56-13); a mechanical MT-to-lane map so status can measure progress; publication: settle one canonical location, reconcile the 3 handpick-side commits, update the catalog.

Remaining candidates: `ledger.json` `direction.next_lane_candidates`.

</topic>

<topic id="how-to-lead" status="active" summary="How the previous lead worked: sub-agents, lanes, rounds, records">

## How to lead this

**Layout.** Kernel `wt-gov-kernel` on `gov_kernel` holds all governance. S = `.GOV/docs_repo/wp-studio-module-work/`: `ledger.json` (lane status authority, key `lane_id`), `START_LOG.md`, `builders/lanes/*.json`, `builders/briefs/*.md`, `validation/round-R*.json`, `spec-enrichment/`, `research/`, `tools/`. Lane worktrees `wt-studio-lanes/<lane>`, product tree inside at `handshake-studio-assets/`. MTs in `.GOV/task_packets/WP-KERNEL-STUDIO/`.

**Agents.** You scope, decide and verify; agents do the rest.

| Job | Model / effort | Notes |
|---|---|---|
| Validation round | opus / xhigh | One, independent, fresh |
| Engine builder | opus / high | One lane, one agent, background |
| Read-only auditor | opus / high | Narrow scope, one output JSON, no cargo. Eight ran in parallel without trouble |
| Record keeping, log summaries | sonnet / low or medium | Never give these a verdict, deletion, licence call or spec reading |

Dispatch in the background and wait for the notification; do not poll. A builder's questions arrive as messages: answer with a decision, a reason and conditions. Stay close to the evidence: read the code before ruling, because several of the previous lead's brief errors came from reasoning without reading.

**Check what agents report.** Before you repeat a claim, verify the ones that matter. On 2026-11-05 this caught nothing false, but it corrected a count and showed a lead grep too narrow. A round's verdict is the validator's; a READY is only the builder's claim. Confirm pushes with `git ls-remote`.

**New lane.**
1. Read the code and write facts into the brief: exact clauses to grep, file:line, what is out of scope.
2. Reuse a completed lane's tree and target (workflow topic, item 2). Only if nothing fits: `git -C "$W/<existing lane>" worktree add -q -b lane/<lane> "$W/<lane>" <base>`, which must be removed after completion (CX-GIT-001A).
3. Write brief, lane file and ledger row (`tools/lead/lanegen.py`), commit and push, then dispatch.

Every brief needs: V4-06 failure-class sweep, red first, hostile input, host_integration block, MT rows to read, what not to touch. Keep builders' commit counts modest: a `--per-sha` hold grows with commits and dependents (12 commits on folio and session held D: for about 55 minutes).

**On READY.** `tools/lead/record_ready.py <spec.json>` verifies the lane file and origin, updates ledger and log, adds SD items, commits and pushes. Lane files are written by builders and committed by the lead.

**Rounds.** Freeze the origin tip, no builders on C: during a round (they may build on D:). `lane-check.sh --disk ssd` is C: and the default; `--disk d` is D:. `--disk c` is not valid. The union runs on `C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-round/target`. Canary: decide under CX-VAL-010 from whether root inputs changed.

**Short path after a move of origin:** if the new commits touch none of a lane's crates: rebase, `--per-sha`, new tests only. If they touch one: that crate's suite once. The next full round proves the union. Do not repeat whole runs for a move.

**Disk.** Two lock queues, C: and D:. At most 6 live lane targets per disk (P3); a PASS lane's target is a reuse candidate first, and is proposed for whole or partial removal only after inspection (HSK-ART-005, workflow topic item 4). The lead reads "live" as not yet PASS. When no round is running, moving queued builders to C: cuts their lock wait.

**Red commits.** A pushed red commit must compile and fail only its own new tests. Do not tell builders that no pushed commit may fail its tests.

**Wall-clock watch:** two harness audio tests with fixed 400 ms deadlines (`audio_import.rs:360`, `viewer_audio.rs:225`) failed once under load; a latent V4-03 risk.

**Windows shell notes.** Long inline heredocs fail in this harness: write scripts and messages with the Write tool. Use `MSYS_NO_PATHCONV=1` for `git show "$O:$A/..."`. Do not run the Grep tool across `.GOV` (6,000+ JSON files time out); use `git grep` in a product tree and python for JSON. A `TaskStop` can leave an agent's child processes alive; agents must stop their own by PID.

**Helpers** (optional, read before use): `tools/lead/lanegen.py`, `tools/lead/record_ready.py`, `tools/lead/mt_prefilter.py`, `tools/lead/target_report.py`, `tools/lead/lane_close.sh`, `tools/lead/lane_watch.sh`.

</topic>

<topic id="dispatch-prompts" status="active" summary="Prompts to reuse: resume a paused lane, new builder, validation round">

## Prompts

**Validation round:** full text of the R56 prompt in `validation/dispatch-prompt-R56.txt`. For R57 change: FULL scope; candidate; claims and probes from `state`; findings from R56; decide the canary under CX-VAL-010. Agent: general-purpose, opus, xhigh, background.

**Resume a paused lane** (general-purpose, opus, high, background):

```text
ROLE: Builder on lane <LANE> (WP-KERNEL-STUDIO, Studio Kit: a Rust rewrite of the Adobe suite plus Figma and Affinity). You take over a lane paused on 2026-11-05. You write product code only in your lane worktree. You never claim PASS; a separate validator judges.

Kernel root: D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel (re-derive it if the project moved). Records root S = .GOV/docs_repo/wp-studio-module-work/ under it.

READ FIRST: S/builders/lanes/<lane file>.json, every note, and `next` exactly; S/builders/briefs/lane-<lane id lowercase>.md; S/builders/briefs/wave-parity-2-common.md and the v4-rules topic of S/builders/DISPATCH_TEMPLATES.md. Gameplans are retired.

WORK in wt-studio-lanes/<lane id lowercase>/handshake-studio-assets, branch lane/<lane id lowercase>. STUDIO_OWNER <lane id lowercase>. Follow `next` in order. Red commits stay. Run anything over 2 minutes in the background. Two failed bundled runs on the same failure stop the lane; report it. Stay in owned paths; ask by message for any other, naming file, line and why. No new egui, eframe, winit or wgpu line; no Cargo.toml change; no root input; no Master Spec edit; do not edit MT files. Do not delete anything, do not touch other lanes' worktrees, do not stop processes you did not start.

PUSH: rebase on origin/codex/handshake-studio-assets, lane-check.sh --owner <lane id lowercase> [--disk d] --per-sha origin/codex/handshake-studio-assets, push on PER_SHA_RESULT OK with SHA=$(git rev-parse HEAD) && [ -n "$SHA" ] && git push origin "$SHA:refs/heads/codex/handshake-studio-assets". Never force. I release one lane at a time.

LANE FILE: keep its keys, indent 2, UTC timestamps. Set READY_FOR_VALIDATION on a pushed commit. Do not commit it in the kernel; I do. Never touch refund-claim-20261021.md.

FINAL MESSAGE, 12 lines or fewer: shas, exact test counts and exit codes, what is not proven or not run, MT state, any path beyond the brief.
```

**New lane:** same shape, with READ FIRST pointing at the brief, the predecessor lanes' notes, the MT files and the exact spec clauses with line numbers, and a final message that names what the lane must report.

**Read-only auditor:** state the question, the tree, the method (`git grep`, python for JSON), "no cargo, no writes except one named output file", what counts as evidence, labels NOT_INSPECTED and UNVERIFIED, output schema, final message length.

</topic>

<topic id="lessons" status="active" summary="What to do and not do; cargo, agents, reasoning tiers, parallelism (lead 2026-10-23, from the records)">

## Lessons (lead 2026-10-23, each from a recorded event)

### Cargo and disks
- The build disk decides the pace. D: is a spinning disk: a round's canary ran 12 h without reaching the union (R38) and died with the machine; builders queued 2 to 4 h behind it and gave up at the 7200 s lock default (SHELL-2 exit 75 after 13666 s). Builds belong on an SSD. If the Operator reopens C:, a round target plus a few lane targets without incremental state is roughly 300 to 400 GiB; the current 150 GB grant is too small for that.
- Incremental state was the single largest regrowing class of Studio cargo output (288 GiB purged 2026-10-21, regrown daily). CARGO_INCREMENTAL=0 is now set for builders (studio-env.sh) and rounds (run-round.sh 23b1bb7). Keep it.
- Every lane, probe and export owner gets a complete target bound to its own worktree and nothing retired it: about 95 owner dirs on D: for 6 active lanes, 1.1 TiB on D: on 2026-10-21. Rule P2: at lane PASS (commits on origin) or abandonment the lead lists the lane's <owner>/target and tmp in the next approved deletion batch. Rule P3: at most 6 live lane targets per disk.
- Deletions: only through the guarded script pattern (research/cargo-cleanup-proposal-20261022.json + cleanup2/3/4.ps1 in the lead scratchpad): allowed roots, forbidden names (steward-export, oracle-corpus, downloads, tools, app-inspect, engine-video-oracle, studio-round, studio-team-dev on D:, every active lane, anything outside WP-KERNEL-STUDIO), allowed shapes, skip paths a running process references, rate limit (150 files/s, 40 MiB/s), background priority, log + result JSON. Never scan disks with parallel walkers (the inventory agent hammered both disks); size from existing inventories and directory listings.
- A validation round and a cleanup on the same disk hammer it; a cleanup and builds on the SSD do too. One disk-heavy job per disk.
- Steward syncs need the disk (probe compile, canary): never dispatch one while a round holds D:; it waits its timeout and dies (two attempts lost).
- A canary costs hours here: ~134 units (crate x round feature set), no artifact sharing across feature sets, an export with fresh mtimes (full rebuild), the spinning disk. Run one only after a sync that changes the lock, scoped to the changed closure; reuse a same-lock canary (C53 reused for 5ab33c9 after the lead wasted 5 h on a redundant full one). Review the round-features matrix for combinations that prove nothing extra.
- The D: lock protocol: builders 900 s max hold, rounds 5400 s; over-hold only logs; stale locks break only when holder and child are provably dead. After a crash the `held` dir of the dead round is cleared by the next waiter.
- Windows Search indexing and Defender real-time scanning cover the build roots (no exclusions); uTorrent and the Codex app add disk ops. These are Operator settings; the lead only reports them.

### Agents (what cost time and tokens)
- Validators: ~280k tokens per round; a full-scope round costs hours of disk. R37 ran full scope for nothing because round-select counted step-0 config files as round inputs: fixed (kernel tools/round-select.py, self-test 25/25). Always scope rounds (CX-VAL-011) and reuse a same-sha canary.
- Three round FAILs had one cause class (package externals classifier on new Folio record fields) before it became brief rule 4b. When a validator FAILs twice on the same class, write the rule into the brief the same day.
- Loop caps work: the timeline lane burned 5 runs on a test defect (pointer aimed at the first egui frame); after the cap and a fresh diagnostics agent it shipped in one run. Give every builder "two failed runs on the same failure, then stop and report" and dispatch a fresh diagnostics agent instead of resuming the builder.
- Agents that wait on locks waste nothing but wall-clock; agents that scan or build concurrently waste disk. Dispatch builders when the disk can serve them, not when you have ideas.
- Lane worktrees + per-lane targets isolate lanes but multiply disk; three lanes in harness-view at once (timeline, shell-2, panels-1) also guarantee rebase conflicts in shell.rs/mod.rs. Prefer two GUI lanes at most at once, and sequence lanes that share a crate.
- Briefs must carry: the exact spec clauses to grep (never "read the module"), the predecessor lane file, the test shape, rules 4a/4b, PUSH SAFETY, the loop cap, the disk, and "report blockers verbatim". Every omission came back as a question or a wasted run.
- Agents cannot be stopped from inside their own session when the permission classifier denies it (steward attempt 2 ran unmonitored). Design agent commands so a stray process is harmless (bounded timeouts, own owner dir) rather than relying on stopping it.
- The lead's own errors this period: registry cache deleted on a misclassification (never again: forbidden in every cleanup script); a push chained after `;` and a $TMPDIR commit-message file git could not read; an over-scoped disk inventory agent (whole disks, three parallel walkers); a steward sync dispatched while a round held D:.

### Reasoning tiers (which agent for what)
- Operator 2026-10-24: "manage agent reasoning strength to save tokens, don't skimp when higher reasoning is useful and can prevent mistakes, identify gaps, risks, conflicts". The lead sets model AND effort on every dispatch; when unsure whether judgment is needed, pick the higher tier.
- High reasoning (Opus-class): validators (adversarial reading, verdict attribution), stewards (lock/licence/edge decisions), builders on engine lanes with spec parameter tables (roto, effect stack, DNG), diagnostics agents on a failing test, parity auditors. Their cost is justified only with a tight brief and a loop cap.
- Effort per role (model / effort): validator round opus / xhigh; diagnostics on a failing test opus / high; steward sync and canary opus / high; cargo manager planning and candidate classification opus / high, its guarded deletion run opus / medium; parity audit, spec reading, licence call, shared-file merge conflict opus / high; engine builder with a spec parameter table or a cross-crate contract opus / high; GUI or routine engine slice opus / medium; simple, fully specified single-crate slice sonnet / medium; rebase of a finished lane plus one green bundled run sonnet / medium (a conflict in a shared file goes to opus / high); record keeping, dependency-request filing, log summaries, size reports from existing inventories, copying round results sonnet / low.
- Low reasoning (Sonnet/Haiku-class): record keeping (ledger/lane-file updates from a report), lane-file drift fixes, dependency-request filing, reading a lane's log and summarising, directory listings and size reports from existing inventories, copying round results into records, rebasing a finished lane and re-running one green bundled run, writing START_LOG lines from a dictated outcome. None of these need judgment; all were done by the lead or an Opus agent this period and should not be.
- Never give a low-reasoning agent a verdict, a deletion, a licence call, a spec reading or a merge-conflict resolution in a shared file.

### Parallelism and speed
- The real limit is one build per disk. Parallel agents only help while they write code or wait; the moment two need the disk, they serialise. Plan the disk queue explicitly: round, then steward canary, then lanes by size.
- Cheapest throughput gains, in order: (1) SSD for builds (Operator decision); (2) scoped rounds and canary reuse (done); (3) no incremental state (done); (4) batch lane close-outs: one guarded push per lane after one green bundled run, then one scoped round for several lanes, instead of a round per lane; (5) sequence shared-crate lanes; (6) keep the steward's work (edges, pins, licence) ahead of the lanes that need it so no lane blocks on a sync.
- Do not hold the Operator's open decisions silently: the C: guard sat on the list for two days while builders queued on D:. Put every blocking decision in chat with the numbers the first time.
- A machine restart kills everything in flight: every agent must leave durable state (lane files, local commits, logs) after every slice, and every round must write its running record so a restart leaves a known state. That held; R38's 12 h did not.

### Claims and push order (lead 2026-10-30)
- A READY claim must cite a run log the lane itself wrote: SMB-AI-AREA-TYPE-3 claimed 285/0/20 at c719b1dc from another lane's log (both used the scratch name run2.log); its own run had been killed and its own test failed (150 of 150 pixels differ). Two other lanes' whole-suite runs caught it within hours. Logs carry the lane id and the built sha in the path and start with the sha.
- Release held pushes downstream-first (harness-view -> harness-core -> folio/composite -> pigment -> nib): a lane rebasing over a change outside its dependency closure needs no re-run. Releasing an upstream lane (nib) while a harness-view lane was mid-check cost that lane a third run. Two lanes that change each other's closure (folio vs pigment) cannot both avoid a re-run: the one that still needs a run anyway pushes last.
- While a round runs on C:, builders start no C: lane-check: one builder hold (max_hold 900 s) against the round's 300 s slices stalled R41 (3062 s of waits) and R42 (12+ min behind smb-view-polish-1). Builders code during a round and run after its verdicts, or build on D:. Measure the C: tree before every C: round and C: lane dispatch (R42 started at 364.9 GiB > 360 cap on a stale 261 GiB figure).
- Before releasing any push, list every in-flight lane run whose dependency closure holds the pushed crates: SMB-R42-FOLLOWUPS-1 (harness-core) pushed while SMB-AU-MULTITRACK-3 and SMB-AI-AREA-TYPE-4 were mid-run, voiding one run and needing a disclosed waiver for the other. Run logs go to Handshake_Artifacts/WP-KERNEL-STUDIO/<lane>/, never only the session scratchpad (HSK-ART-001).
- Never edit a tool script (tools/lane-check.sh, studio-env.sh) while any builder or round may be running it: bash reads scripts incrementally, and the 17:35Z lane-check header edit broke SMB-FIG-GRID-2's in-flight run with a syntax error (one infrastructure re-run). Edit tools only when no lane run or round is in flight, or ship the change as a new file and switch over.
- Before ruling on a numeric divergence between backends, read both implementations: the R46 threshold-tie disposition assumed binary64 on the CPU from memory; pigment is binary32 end to end, so the exemption hid a real Vulkan divergence (F-R46-1). A test-only dev-dependency that changes a crate's feature set rebuilds every dependent test binary under new artifact identities and leaves the old ones on disk (F-R46-8: 146 GiB in one round); route such features through a dedicated test crate or retire the stale artifacts in the next batch.
- When a new contract is cloned from an existing one, diff every field against the source before committing: MT-36248..36251 kept MT-36233's execution block under new closure units for a day, and only a planner reading MT-36249 for settings work caught it.

### Lead 2026-11-05 (each from a recorded event)
- Status is not progress. 62 of 6,220 MTs read PASS while most engines exist, and some PASS rows cannot reach a document. Measure progress by reading code against contracts, not from status, lane PASS counts or anchor counts. Rounds never gated the HBR pillars; a green round says nothing about manual, diagnostics or the model surface.
- Read the code before you put a claim in a brief or a ruling. This period's lead errors, all caught by builders: U+2028 "composes in both providers" (the harfrust spike is v1 only); `--disk c` (not valid); `ClipChange` "flows through session" (it does not); a blanket removal refusal for view state, and then for `audio_routing` (Run commits the sequence before the entry, so a refusal would have blocked the Viewer's own undo). When a builder shows your reading is wrong, withdraw it on the record.
- Tell builders plainly that red commits stay. "No commit you push fails its own tests" contradicted the red rule and cost a question.
- Per-sha holds are long: 12 commits on folio and session held the D: lock for about 55 minutes. A HIGH-finding lane sat over an hour queued behind them. Keep commit counts modest, split lanes across C: and D: when no round runs, and have builders take red and focused runs in one hold, because each cargo call queues separately.
- Pausing: "finish the job that is compiling, stop what is only queued, no new holds, write `next`" gave five clean stops. Do not kill a job holding a build lock. `TaskStop` can leave an agent's child processes alive, so the agent must stop its own by PID.
- A lane paused on a rebased base wastes its per-sha: nudge's check ran against a base origin had already passed. Check the base before you let a long hold start.
- Keep dispatch prompts and helpers in the repo, not only in a session scratchpad or transcript. A new session cannot resume another session's agents: the next lead dispatches fresh builders from the lane file's `next`.
- Lead script error: a commit message path was relative to the wrong cwd, so records were written but not committed. Verify the commit, not only the script's exit.

</topic>

<topic id="operator-items" status="active" summary="Decisions and approvals only the Operator can give">

## Waiting on the Operator

**Operator 2026-11-11:** "i have no idea what you open questions are about, just deliberate with specialised agents and use that result as decision for it." So the lead does not put open questions to the Operator: a specialised agent deliberates and its result is the decision (`direction.operator_direction_20261111`). What remains below is held by a hard rule (deletions of worktrees, branches and new removal lists; legal; Windows settings; work at the vendor apps), not asked.

**Vulkan pivot:** O1..O10 were decided by the lead on 2026-11-07 after an agent team and a red team (`direction.blocker_decisions_20261107`); nothing open.

**Waivers (GLOBAL-SCOPE-005):** decided by a scope-audit agent on 2026-11-11 (`research/waivers-20261111-global-scope-005.json`); the list below is kept for history.

**Durability:** done 2026-11-05 on the Operator's approval: the 23 commits are on 5 GitHub backup branches, and media markers' 3 uncommitted files are on a sixth (`state` topic).

**Readings the lead took, overrulable:** listed in the `state` topic.

**Waiver of paths outside briefs (GLOBAL-SCOPE-005), decided 2026-11-11: all WAIVE (research/waivers-20261111-global-scope-005.json):**
- SMB-PR-SEQUENCE-PRESETS-1: folio `composition.rs`.
- SMB-PR-RETIME-HOLD-1: pulse `src/transition.rs`, `src/multicam.rs`, `src/timeline_edit.rs`, `tests/multicam_flatten.rs`; session `tests/retime_commit.rs`; `rate.rs`.
- SMB-PR-MERGE-SYNC-1: pulse `src/timeline_edit.rs`, `src/annotate.rs`.
- SMB-R54-FOLLOWUPS-1: harness-view `src/view/vector.rs`; layout `src/threaded_reflow.rs`, `tests/cjk_kinsoku.rs`, `tests/marker_lookups.rs`.
- SMB-LAYOUT-ENTRY-TEXT-1: interop-layout `tests/longdoc_pdf.rs`; layout `src/index_contract.rs`, `src/sentinels.rs`.
- SMB-PR-EDIT-FOOTPRINT-1: harness-view `plan_move`, its helper and new test file.
- SMB-LAYOUT-BLANK-LINE-LOOKUPS-1: layout `src/toc_contract.rs`, `src/endnotes.rs`, the `TocOmitted` export in `lib.rs`.
- SMB-TYPE-AUTHORED-TOKEN-1: harness-core `harness.rs`, `import.rs`; harness-view `fig_import.rs`, `vector/interop.rs`, `vector.rs`, `vector/path_text.rs`; harness tests `import_psd.rs`, `viewer_mask_package.rs`; type `descriptor.rs`; session `sequence_records.rs`.
- Not on the product branch yet: SMB-PR-ENTRY-REFERENCES-1: harness-view `tests/viewer_inert_routing.rs`, pulse `tests/inert_routing_mix.rs`, a doc line in session `src/source.rs`. SMB-PR-NUDGE-1: a comment in pulse `src/timeline_edit.rs`.

**Plan U: DONE 2026-11-11 13:08Z** on the Operator's "Approve plan U": 5 old C: build caches of validated lanes removed whole, 25.97 GiB, 83,650 files (`research/cargo-cleanup-result-20261111-U.json`). Next for the Operator: a Studio D: removal list (D: Studio 1,912 GiB), prepared after the WizTree file export.

**Plan V: DONE 2026-11-11 18:01Z** on the Operator's "can you execute it?": older copies of test and binary outputs removed from 11 Studio D: build caches, 1,395.2 GiB, 27,841 files, newest copy of every binary and all libraries kept (`research/cargo-cleanup-result-20261111-V.json`).

**C: cap 2026-11-11:** the C: tree reached 389 GiB, over the 400 GB grant, while three C: lanes rebuilt. T-C04 (plan T) freed 19.47 GiB, leaving 369.9 GiB. A cargo-specialist deliberation (`research/decision-20261111-c-cap-placement.json`) moved SMB-TYPE-INDENT-SPACING-1 and SMB-SCORE-EFFECTS-6 onto READY D: trees with warm targets. Both landed (153a410d, 733789d6); T-C01 and T-C02 executed 2026-11-12 12:53Z, 59.16 + 52.1 = 111.27 GiB of older copies off C:, C: free 334G to 445G (`research/cargo-cleanup-result-20261112-T-c01-c02.json`). Plan T is EXECUTED.

**Cleanup list (held by the hard rule on deleting worktrees and branches; not asked; nothing has been deleted):** registered scratch worktree `Handshake_Artifacts/WP-KERNEL-STUDIO/smb-type-sentinels-registry-1/rw`; probe trees `validator-r53-probe`, `validator-r54-probe`, `validator-r55-probe`, `validator-r56-probe`; C: target `lanes/smb-pr-sequence-presets-1-red` and owner folder; worktrees and targets of PASS lanes including SMB-R54-FOLLOWUPS-1, SMB-PR-SEQUENCE-SESSION-1, SMB-PR-RETIME-HOLD-1, SMB-PR-MERGE-SYNC-1, SMB-LAYOUT-ENTRY-TEXT-1; local ref `backup/smb-pr-track-display-1-pre-fold`. Later, once each paused lane has landed on the product branch: its GitHub backup branch, plus `lane/smb-pr-media-markers-1-uncommitted` for media markers (`direction.backup_20261105`).

**Spec owner:** SD-31..SD-52 and D7A-01..17 PROPOSED. MT proof commands name targets that do not exist (F-R55-3, F-R56-13).

**Oracle work needing the Operator at the apps:** PS-C28 mask variants; Photoshop colour setting HSK-gamma-1.0; Affinity test files; Figma `.fig` fixtures; Acrobat PDF/X oracle; held-out InDesign oracle set; AE-02 tangents rerun; Premiere and Audition have no oracle.

**Carried from earlier handoffs:** os-encode legal check before commercial release; Windows Search and Defender exclusions for the build roots (report only); a Chrome GPU process slows GPU tests in rounds 3-30x (report only); spec residue folders `master-spec-v02.218` and `master-spec-v02.220`; older lane branches and worktrees; optional deletion of GitHub branch `lane/smb-layout-ui-2-backup`. Earlier resolved decisions: `handoff-archive/HANDOFF-20261101.md`.

</topic>
