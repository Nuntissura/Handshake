---
file_id: WP-STUDIO-MODULE-WORK-RESEARCH-CRAFT-CASE-STUDY
file_kind: research_note
updated_at: 2026-10-11T01:20:00Z
wp: WP-KERNEL-STUDIO
lane: research-craft
note_type: gap_delta
---

# Craft case study: delta over prep (prose surface, not authority)

Scope per coordinator: prep already studied Craft; this note lists only what prep lacks. Claims cite a file or URL; anything else is labelled UNVERIFIED. Source-cache root `<HW>` = `Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/`; `PC` = `<HW>photocraft-175603a644f10d2e8c61f82a63f7c2659c71dc47/`.

<topic id="prep-baseline" status="active" summary="What prep already found; do not redo">

## Already in prep (read these, do not duplicate)

- What Craft is + where source lives: `.GOV/docs_repo/studio-collaboration-2026-10-09/source-cache-manifest.json` pins 5 repos (photocraft 175603a6, vectorcraft 825d4fdb, designcraft 66c7ce7e, filmcraft 7b6c1344, lightcraft 88ea1a80) as extracted archives; the cache dir also holds effectcraft, pdfcraft, soundcraft (not in the manifest `projects` excerpt read here).
- PhotoCraft findings PHOTO-F-001..013, reuse boundaries PHOTO-REUSE-001..007, reject PHOTO-REJECT-001, unrun proof lanes PHOTO-PROOF-001..006: `.GOV/reference/studio_app_feature_research/repass-2026-10-09/photocraft-root-drill.json`. Verdict there: runtime/build/tests NOT_RUN, Photoshop GUI NOT_INSPECTED.
- Already covered, so not repeated: cached smart/type pixels inflate corpus passes (F-002), PSD import loss semantics (F-004), early-return GPU tests (F-003), RAW CR3/RAF stub (F-007), 8-bit display downgrade (F-008), ZIP 65,535 entry cap (F-001), `psd` crate as reusable leaf (REUSE-001), cms/color as ICC candidate (REUSE-002).
- Additional Craft references: `.GOV/docs_repo/studio-collaboration-2026-10-09/adobe-original-team/{photo,design,media}-original.json`, `independent-original-parity-redteam.json` (LightCraft WAL, DesignCraft IDML, PdfCraft redact).

</topic>

<topic id="local-install-findings" status="active" summary="No Craft install on this machine; source-only">

## Local install findings (read-only inspection, 2026-10-11)

- Verdict: NO installed Craft app (no PhotoCraft/VectorCraft/FilmCraft/LightCraft/PdfCraft/PrintCraft/EffectCraft/DesignCraft binary, shortcut, registry entry, Appx or package) found. Craft exists locally only as source snapshots (no `target/`, `.exe`, `.msi` in cache; checked `find -maxdepth 3`): `<HW>` (8 repos + .tar.gz) and copies under `Handshake_Artifacts/WP-KERNEL-STUDIO/studio-repass-engineering/source/{photocraft,vectorcraft,designcraft}` and `.../studio-repass-media/source/{effectcraft,filmcraft,lightcraft}-<sha>`.
- Checked: both Start Menu trees (depth 4; only `StarCraft II` matches "craft"), `%LOCALAPPDATA%` top + `\Programs`, `%APPDATA%` top, `C:\Users\Ilja Smets` top, Desktop, Downloads (names), `D:\` top, `D:\Projects` top, `~/.cargo/bin` (no Craft; has cargo-clippy/deny/miri/tarpaulin/sqlx, just, rust-analyzer), HKLM/HKCU Uninstall (only `RoadCraft`, a Steam game), `winget list` (no Craft), `Get-AppxPackage`, chocolatey lib, `C:\Program Files` + `(x86)` top-level.
- A recursive `*craft*` name search (depth 3 over AppData, `D:\Projects`, Documents, Downloads, `.cargo`, `~/bin`) completed (exit 0); its only hits were unrelated: a minecraft-modpack skill dir, Temp ComfyUI `recraft_*` images, `off_target_crafts_or_kids_activity`, and `chaewon-qa-craft.jpg`. No Craft app. Not searched: other drives and deeper than depth 3.
- Not Craft but near-miss, for the record: `Canva.Affinity` 3.2.3.4646 Appx (winget shows 3.3.1.4940 available) + `Downloads/Affinity x64.msix`; `C:\Program Files\CKC` is an Electron app ("Character sheet management system" shortcut description), unrelated.
- Consequence: nothing on this machine can be used to eyeball Craft's PSD display. Verification of PSD behaviour must come from source (below) or a future operator-approved build/download (not done: no installs/GUI allowed).
- Naming: press calls the Acrobat-class app `PrintCraft` (GIGAZINE); the repo/cache dir is `pdfcraft`. Use the repo name per [GLOBAL-NAMING-003].

</topic>

<topic id="what-craft-is" status="active" summary="New facts beyond prep: authors, scale, maturity, claims">

## What Craft is (only facts not in prep)

- Brand: "ArtCraft Crafting Apps" by the ArtCraft team, GitHub org `storytold`; PhotoCraft created 2026-09-30, MIT OR Apache-2.0, "early alpha", self-described not a daily professional replacement (https://github.com/storytold/photocraft; Phoronix https://www.phoronix.com/news/Photocraft-Filmcraft-AI; GIGAZINE https://gigazine.net/gsc_news/en/20261005-artcraft-crafting-apps/). Written largely with AI agents (Phoronix headline; one press summary names Claude Opus 5.5, UNVERIFIED); README says all commands are exposed via CLI, JSON control channel and MCP.
- Own stats (vendor claims, UNVERIFIED independently): 24 crates, 500+ commands, 55 tools (repo page) vs "34 tools" in one secondary summary, 1,700+ tests; star count differs by source (42.2k on repo page when fetched).
- "Clean-room" is contested (Phoronix; E. S. Raymond speculation is unverified). Not our concern for lessons, but it bears on any copying: take ideas, not code. PC `crates/psd/src/lib.rs` header itself says some undocumented details follow MIT psd-tools / ag-psd.
- Maintainer promised "100% parity within a month", later "99% parity will take a while" (secondary report of HN comments; HN thread itself not opened: UNVERIFIED).

</topic>

<topic id="feature-gaps" status="active" summary="Self-reported and externally reported gaps beyond prep">

## Feature gaps (new)

- Self-scored parity, PC `docs/parity-estimate.md:15-17`: menu breadth 625/625 (100%), PSD oracle ~111/170 = ~65%, overall functional ~86%; header admits menu coverage "measures wiring, not behaviour" (repo page). Enforced corpus floors, PC `docs/scorecard.md:86-88`: 146/170, 236/309, 133/256 oracle passes, i.e. 14%, 24% and 48% of files may fail pixel match yet CI stays green.
- `docs/scorecard.md` ~161: "Settings that do nothing: 55 of 141" preferences are stored but never read (unconsumed list at line 163).
- Large-file/perf: 20 MP layered PSD first pixels 1753-2794 ms vs 1000 ms budget (scorecard P18); no 2 GB PSD, 150-layer or 20 MP fixtures, nothing in CI (FILE-216-2); "exports re-open in a third-party oracle editor: missing" (FILE-216-4), so PSD export is only proven against its own importer.
- Known partial: Advanced Blending (knockout, blend-as-group; `knockout-isolated-groups`, `advanced-blending`, `blend-and-clipping` still differ; GPU falls back to CPU) FILE-215-16; type-on-path missing (TYPE-218-1); Optical/manual kerning lost on PSD import/export (BUG-206-2/3); Windows has no display-profile reader (below).
- External reports: promising but "no RAW support, missing features, incomplete UI" (secondary HN summary, UNVERIFIED); open GitHub issues #594, #583, #1347, #1433, #148 (see next topic).

</topic>

<topic id="psd-display-issue" status="active" summary="Why PSDs look different from Adobe: verified causes ranked">

## PSD display discrepancy: root-cause map

Operator observation: PSDs display differently than in Adobe. No local Craft install, so this is from source + public issues. Verified = read in source; reported = GitHub issue; hypothesis = UNVERIFIED.

1. VERIFIED, mechanism: for layered RGB/Gray/CMYK/Lab PSDs PhotoCraft does NOT display Photoshop's stored merged image. `PC crates/io/src/psd_import.rs:631-648`: merged composite "decoded only when something consumes it" (flattened/multichannel/extra-alpha files); normal layered saves skip it ("nobody reads"). It re-composites layers itself (`crates/compose`, `Buffer{px: Vec<[f32;4]>}`). Except smart/type/shape/fill layers, where Photoshop's cached layer pixels are reused (psd_import.rs:348,368; prep F-002). So every divergence in Craft's own blend/effect/adjustment math is visible; Adobe shows its own render.
2. VERIFIED, fidelity class: compose math is empirical. `crates/compose/src/adjust.rs:3-4` "documented approximations of Photoshop behaviour"; `effects.rs:730` bevel height map is a blur "fitted on the corpus" (size 5 within 0.7% mean, size 41 within 2.6%); `psblend.rs:191` text-blend gamma fitted on two PSD files; adjust.rs:16 Dot Gain 20% gray ≈ gamma 1.73 "fitted"; `docs/parity-estimate.md:68,91` lists open failures: bevel/satin shapes, Lab blending, noise-gradient RNG, toe curves ("proprietary RNGs and unpublished curve models"). Matches reported issue #594 (Bevel & Emboss: highlights less golden, shadows darker/olive, heavier bevel; open) https://github.com/storytold/photocraft/issues/594.
3. VERIFIED, color management on Windows: `apps/photocraft/src/monitor_profile.rs:113-116` non-macOS `detect()` returns "this platform doesn't report display profiles"; `display_color.rs:158` "No platform reader (web, Linux, Windows, tests)". With Monitor Profile `auto` the canvas assumes sRGB. A Windows wide-gamut monitor with an installed profile is therefore shown differently from Photoshop (which uses the OS monitor profile). The oracle corpus was authored on macOS Photoshop 2026 (corpus README, https://github.com/storytold/photocraft-corpus).
4. VERIFIED, profiles are substitutes: `crates/cms/src/builtin.rs:37,100-120` ships "Adobe RGB (1998)-compatible", "ProPhoto-compatible" and a synthetic coated CMYK; `crates/cms/src/synth.rs:1-25` says Adobe's SWOP/FOGRA39 are proprietary and its CMYK is "plausible, not a measured characterisation". Default CMYK is this synthetic profile, so CMYK PSDs without an embedded profile cannot match Photoshop. Matches issue #583 (8-bit CMYK PSD more saturated than Photoshop, macOS 0.2.0; reporter suspects color settings; open) https://github.com/storytold/photocraft/issues/583.
5. VERIFIED, test blind spot: the Photoshop oracle corpus (256 generated PSDs) embeds NO ICC profiles; CMYK/Lab made from RGB with default settings; oracle compares to the merged composite within 2/255 premultiplied (`crates/engine/tests/photoshop_oracles.rs:1-35`, corpus README). Real-world ICC-tagged files (Adobe RGB, ProPhoto, Display P3, SWOP) are untested against Photoshop. Merged composite only exists with Maximize Compatibility set; corpus forces "Always".
6. REPORTED, adjustment import: Photo Filter PSD colors differed from the stored Photoshop preview (issue #442, closed by PR #850 on 2026-10-08) https://github.com/storytold/photocraft/issues/442; Color Lookup tetrahedral->trilinear on PSD save changes composite silently (#1347); type-layer bounds clipped (#148); brush edges look pixelated (#1433).
7. VERIFIED, absent feature: no support found for Photoshop's "Blend RGB Colors Using Gamma" document option (grep of compose/doc/io found only a text-gamma setting; UNVERIFIED that it is wholly unsupported). 16-bit/32-bit: documents open natively per README, but display texture can drop to 8-bit (prep F-008).
8. Hypothesis, UNVERIFIED: the Operator's specific files may hit any of 1-5; confirming needs one Operator PSD rendered by a built PhotoCraft versus Photoshop's merged image.

</topic>

<topic id="lessons-per-crate" status="active" summary="New lessons per Studio crate; each adds to prep, none restates it">

## Lessons per Studio crate (new vs prep)

- hsk-studio-interop-psd: (a) Treat Photoshop's merged composite as an asset, not waste: decode it lazily in the background as a self-check oracle and offer a "Photoshop preview" toggle; Craft discards it (psd_import.rs:631) and loses the cheapest fidelity check. (b) Every import records the display tier shown (`merged` | `layer-cache` | `re-composite`) and a delta score (premultiplied max diff + % pixels over tolerance, as `max_diff` in photoshop_oracles.rs:43-58) into the loss report. (c) Detect "no merged composite" (Maximize Compatibility off) and flag that no oracle exists. (d) Flat-import quirk: Photoshop mattes the merged image alpha against white; undo it (psd_import.rs:684-687). (e) Never claim export proof from our own importer (FILE-216-4); require an external-editor re-open.
- hsk-studio-prism: (a) Provenance field on every profile: `adobe-licensed | icc-file | synthetic-approx`; never label a synthetic profile SWOP/Adobe RGB, and stamp renders that used one. (b) Display profile source enum `os-reported | manual | fallback-sRGB` surfaced to Argus snapshots; implement a Windows reader (candidate: Windows color system profile query; API choice UNVERIFIED) because Craft's reader is macOS-only. (c) Keep per-(doc profile, mode, monitor profile) cache with engine version (display_color.rs:17) which already matches the module responsibility. (d) CMYK default must be an explicit user/doc choice, not silent.
- hsk-studio-pigment: Make premultiplied-vs-straight and sample type part of the tile contract; Craft's whole oracle metric (premultiplied, visible pixels only, sizes differ => 1.0) depends on it. Keep f32 working sample path for fitted-curve parity; avoid 8-bit intermediate between adjustments.
- hsk-studio-composite: Tag every operator with a fidelity class `exact | fitted | approximate | unsupported` and carry the lowest class into the render receipt; Craft's "documented approximation" comments are invisible to users and CI. Treat knockout/blend-as-group/clipping interaction as a first-class test row (Craft still fails these). Put the blend-space setting (gamma vs linear, text gamma) in the document model, not in a global (Craft's text gamma is a process-global atomic, psblend.rs:text_gamma).
- hsk-studio-render-cpu: Keep it the only reference oracle (Craft's CPU/GPU split does this; GPU tolerance is 1/255+4 ulp vs CPU, 2/255 vs Photoshop). A missing external corpus must FAIL, not skip (photoshop_oracles.rs:3-5 does this; contrast GPU early-returns in prep F-003).
- hsk-studio-render-gpu: Serialize wgpu instance/device creation in tests (Craft `GPU_LOCK`: concurrent wgpu instances segfault on some drivers, gpu/tests/parity.rs); matters for our parallel builders. Expose per-feature capability so composite routes unsupported ops (advanced blending) to CPU explicitly, not by silent fallback.
- hsk-studio-controls: Registry entries carry `status: wired | behaviour-verified | stub` and a build-time audit that fails when a registered setting has no consumer (Craft: 55/141 prefs do nothing; 625/625 menu "wired"). Reject duplicate ids (prep F-012) and also report capabilities that return `available:true` with no device.
- Avoid: copying PhotoCraft's engine/UI facades (prep REJECT-001); using a pass-rate headline as acceptance; treating a macOS-authored oracle as valid for Windows display.

</topic>

<topic id="sources" status="active" summary="Citations; fetched 2026-10-11">

## Sources

- Local: `<HW>photocraft-175603a6.../` files cited above; prep: `.GOV/reference/studio_app_feature_research/repass-2026-10-09/photocraft-root-drill.json`, `.GOV/docs_repo/studio-collaboration-2026-10-09/source-cache-manifest.json`; module map `.GOV/task_packets/WP-KERNEL-STUDIO/STUDIO_MODULES.json`.
- Web: https://github.com/storytold/photocraft ; https://github.com/storytold/photocraft-corpus ; issues /583 /442 /594 (fetched), /1347 /148 /1433 (search snippets only) ; https://www.phoronix.com/news/Photocraft-Filmcraft-AI (403 on fetch; search snippet only) ; https://gigazine.net/gsc_news/en/20261005-artcraft-crafting-apps/ ; https://www.notebookcheck.net/Photoshop-Premiere-more-Experienced-Rust-developer-recreates-Adobe-suite-with-AI-assistance-as-open-source.1419230.0.html (search snippet only).
- Not done: HN/Reddit/X threads not opened; no build, run, or GUI launch; Adobe Photoshop 2026 (27.11.0.26) is installed locally but was not launched, so no side-by-side was made.

</topic>
