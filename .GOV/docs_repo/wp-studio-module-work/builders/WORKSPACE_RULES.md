---
file_id: WP-STUDIO-MODULE-WORK-WORKSPACE-RULES
file_kind: builder_workspace_rules
updated_at: 2026-10-11T01:55:00Z
owner: workspace-steward
---

<topic id="cargo-form" status="active" summary="Only allowed cargo invocation">

- Always: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo <cmd> -p <your-crate> ...`
- `<cmd>`: `check --tests`, `clippy --tests`, focused `test`. Never `--workspace` tests; no `timeout` wrapper; > 2 min -> background + poll. Nothing on C: (env puts target + TMP on D:).
- `Blocking waiting for file lock` = another agent building; wait, do not kill.

</topic>

<topic id="files" status="active" summary="What builders may edit and stage">

- Edit and stage only `crates/<your-crate>/**`. Never edit or stage root `Cargo.toml`, `Cargo.lock`, `tools/**`, `rust-toolchain.toml`, other crates. Steward commits lock sync + registry/probe updates before each validator round.
- Crate `Cargo.toml` is parsed by `tools/workspace-probe.rs` (bounded TOML subset): single-line inline tables; no dotted keys (`x.workspace = true` -> `x = { workspace = true }`); no `[dependencies.foo]` subtables; no git/registry/branch/rev; no `workspace = true` deps.
- Commit with explicit paths; on `index.lock` wait 5 s and retry; never force push.

</topic>

<topic id="path-deps" status="active" summary="Allowed intra-Studio edges (root workspace.metadata.studio.dependencies)">

- Add allowed edges yourself: `hsk-studio-<dep> = { path = "../hsk-studio-<dep>", version = "=0.1.0", default-features = false }`. Any other edge rejects (`dependency_outside_declared_dag`); no cycles.
- Every crate may use `accord` + `observe`, except: accord -> none; observe -> accord; folio -> accord only. Extra edges:
- chronicle: folio | pigment: prism, folio | prism, nib, type, score, reel, controls: none
- layout: folio, type, nib | develop: pigment, prism | components: folio, layout | pulse: folio
- motion: folio, nib, type | composite: folio, prism, pigment, nib, type
- render-cpu, render-gpu: prism, pigment, nib, type | package, interop-media, interop-lrcat, press, web: folio
- interop-psd: folio, pigment, prism, type | interop-vector: folio, nib, type, prism
- interop-fig: folio, components, type | interop-layout: folio, layout, type, prism
- session: folio, chronicle, package | harness: folio, session, controls | kernel-bindings: folio, session
- score-device, score-plugin: score | reel-native: reel | motion-js: motion

</topic>

<topic id="crates-io-deps" status="active" summary="Adding a crates.io dependency">

1. Add it directly in your crate Cargo.toml so you can compile now: `<dep> = { version = "=X.Y.Z", default-features = false, features = [...] }` (exact `=` pin, explicit no-defaults, `features` always present even if `[]`). Renamed: add `package = "<real>"`.
2. AND append ONE line to `builders/dep-requests.jsonl`: `{"crate":"hsk-studio-<x>","dep":"<real-name>","version":"X.Y.Z","features":[...],"reason":"<why + research ref>","pure_rust":true}`. Unrequested deps block the steward's sync.
3. Steward runs `tools/registry_sync.py` (work folder): derives `direct` + full `locked` closure from Cargo.lock into root metadata and the probe `APPROVED` table, runs the probe, commits.
- Prefer versions already pinned (serde 1.0.229, serde_json 1.0.151, schemars 1.2.2, sha2 0.10.9/0.11.0, kurbo 0.13.1, moxcms 0.9.1, bitflags 2.13.2, bytemuck 1.25.2).
- Native policy (probe): `-sys`/FFI only in render-gpu, score-device, score-plugin, reel-native, motion-js (`pure_rust:false`); wgpu only render-gpu; egui only controls; ffmpeg-sys-next only reel-native. Always rejected: handshake_core, handshake_native, surrealdb, rocksdb, eframe, image, libsqlite3-sys. Accord/folio: no egui/eframe/wgpu/image/codec/db deps at all.

</topic>
