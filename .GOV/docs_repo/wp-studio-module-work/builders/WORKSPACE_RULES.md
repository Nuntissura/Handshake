---
file_id: WP-STUDIO-MODULE-WORK-WORKSPACE-RULES
file_kind: builder_workspace_rules
updated_at: 2026-10-11T02:40:00Z
owner: workspace-steward
---

<topic id="cargo-form" status="active" summary="Only allowed cargo invocation">

- Always: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo <cmd> -p <your-crate> ...`
- `<cmd>`: `check --tests`, `clippy --tests`, focused `test`. Never `--workspace` tests; no `timeout` wrapper; > 2 min -> background + poll. Nothing on C:. `Blocking waiting for file lock` = wait, never kill.

</topic>

<topic id="git" status="active" summary="Staging and committing in the shared worktree">

- Edit only `crates/<your-crate>/**`. Never edit, stage or commit root `Cargo.toml`, `Cargo.lock`, `tools/**`, `.config/**`, `rust-toolchain.toml`, other crates.
- Commit ONLY with a pathspec: `git commit -m "..." -- crates/hsk-studio-<own>` (new files: `git add -- <paths>` first). Never a bare `git commit`: after steward CAS commits the shared index can hold stale Cargo.lock/Cargo.toml/probe entries that a bare commit would revert.
- Push immediately after each commit (steward sync needs local HEAD == origin). On `index.lock` wait 5 s and retry; never force push.
- Crate `Cargo.toml` is parsed by `tools/workspace-probe.rs` (bounded TOML): single-line inline tables; no dotted keys (`x = { workspace = true }`); no `[dependencies.foo]` subtables; no git/registry/branch/rev; no `workspace = true` deps.

</topic>

<topic id="path-deps" status="active" summary="Allowed intra-Studio edges (root workspace.metadata.studio.dependencies)">

- Add allowed edges yourself: `hsk-studio-<dep> = { path = "../hsk-studio-<dep>", version = "=0.1.0", default-features = false }`. Other edges reject (`dependency_outside_declared_dag`); a new edge needs a lead decision, then the steward adds it (`steward-sync.sh --add-edge crate=dep`).
- Every crate may use `accord` + `observe`, except: accord -> none; observe -> accord; folio -> accord only. Extra edges:
- chronicle: folio | pigment: prism, folio | prism, nib, type, score, reel, controls: none
- layout: folio, type, nib | develop: pigment, prism | components: folio, layout | pulse: folio
- motion: folio, nib, type | composite: folio, prism, pigment, nib, type | interop-media: folio, pulse (D9)
- render-cpu, render-gpu: prism, pigment, nib, type | package, interop-lrcat, press, web: folio
- interop-psd: folio, pigment, prism, type | interop-vector: folio, nib, type, prism
- interop-fig: folio, components, type | interop-layout: folio, layout, type, prism
- session: folio, chronicle, package | harness: folio, session, controls | kernel-bindings: folio, session
- score-device, score-plugin: score | reel-native: reel | motion-js: motion

</topic>

<topic id="crates-io-deps" status="active" summary="Adding a crates.io dependency">

1. Add it in your crate Cargo.toml: `<dep> = { version = "=X.Y.Z", default-features = false, features = [...] }` (`features` always present, even `[]`; renamed: `package = "<real>"`).
2. Append ONE line to `builders/dep-requests.jsonl`: `{"crate":"hsk-studio-<x>","dep":"<real-name>","version":"X.Y.Z","features":[...],"reason":"<why + research ref>","pure_rust":true}`.
3. Steward sync pins it before the next round. Native policy: `-sys`/FFI only render-gpu, score-device, score-plugin, reel-native, motion-js; wgpu only render-gpu; egui only controls; ffmpeg-sys-next only reel-native; never handshake_core/handshake_native/surrealdb/rocksdb/eframe/image/libsqlite3-sys; copyleft goes to the lead.

</topic>

<topic id="steward-sync" status="active" summary="Steward sync commit method (round candidate)">

- Tool: `bash .GOV/docs_repo/wp-studio-module-work/tools/steward-sync.sh [--add-edge crate=dep]` (helpers: export-refresh.sh, edges.py, license_gate.py, probe_provenance.py; assets `tools/registry-sync.py`).
- Steps: require HEAD == origin; refresh `Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/sync-export/src` in place (dedicated index, changed files only); derive Cargo.lock with `cargo metadata --offline` (minimal, pins kept, never `cargo update`); `registry-sync.py --write`; license gate; verify `cargo metadata --locked --offline`, registry-sync check, probe per crate.
- Commit: temp `GIT_INDEX_FILE`, `git read-tree HEAD`, `update-index --cacheinfo` for changed Cargo.lock / root Cargo.toml / tools/workspace-probe.rs only, `write-tree`, `commit-tree -p HEAD`, `git update-ref refs/heads/codex/handshake-studio-assets <new> <HEAD>` (CAS; fails if a builder moved HEAD -> rerun), push. Shared index and working files untouched. SYNC_SHA = round candidate; the sync export gets its ROUND_IDENTITY.
- Round entrypoint `tools/run-round.sh` (export mode) rewrites nothing: step 0 `cargo metadata --locked --offline` + `registry-sync.py` check exit 75 on drift -> request a steward sync.

</topic>
